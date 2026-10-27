//! HTTP/1.1 over `TcpStream` or the system TLS (tls.rs): GET, redirects,
//! `Content-Length`, `chunked`, read-until-close; page encodings.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use crate::tls::{self, Conn};
use crate::url;

/// Response: final address (after redirects), type and body.
pub struct Response {
    pub url: String,
    pub content_type: String,
    pub body: Vec<u8>,
}

const TIMEOUT: Duration = Duration::from_secs(10);
/// We don't read more than this: enough for a tiny browser.
const MAX_BODY: usize = 8 << 20;

pub fn get(start: &str) -> Result<Response, String> {
    let mut target = start.to_string();
    for _ in 0..6 {
        // Windows: WinHTTP does all of HTTPS (and follows further redirects itself).
        #[cfg(windows)]
        if target.starts_with("https://") {
            return crate::winhttp::get(&target);
        }
        let u = url::parse(&target).filter(|u| u.scheme == "http" || u.scheme == "https").ok_or_else(|| format!("Not an HTTP address: {target}"))?;
        let (status, headers, body) = request(&u)?;
        let header = |name: &str| headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str());
        if (300..400).contains(&status)
            && let Some(loc) = header("location")
        {
            target = url::resolve(&target, loc);
            continue;
        }
        if status >= 400 {
            return Err(format!("Server responded {status}"));
        }
        return Ok(Response { url: target, content_type: header("content-type").unwrap_or("").to_string(), body });
    }
    Err("Too many redirects".into())
}

/// Status, headers, body.
type Raw = (u16, Vec<(String, String)>, Vec<u8>);

/// TCP connection to the host: all addresses in turn (IPv6 may be unreachable), timeouts.
pub fn tcp(host: &str, port: u16) -> Result<TcpStream, String> {
    let mut err = format!("{host}: address not found");
    for addr in (host, port).to_socket_addrs().map_err(|e| format!("{host}: {e}"))? {
        match TcpStream::connect_timeout(&addr, TIMEOUT) {
            Ok(s) => {
                let _ = s.set_read_timeout(Some(TIMEOUT));
                return Ok(s);
            }
            Err(e) => err = format!("{host}: {e}"),
        }
    }
    Err(err)
}

fn request(u: &url::Url) -> Result<Raw, String> {
    let mut s: Box<dyn Conn> = if u.scheme == "https" { tls::connect(u.host, u.port)? } else { Box::new(tcp(u.host, u.port)?) };
    let default = if u.scheme == "https" { 443 } else { 80 };
    let host = if u.port == default { u.host.to_string() } else { format!("{}:{}", u.host, u.port) };
    let req = format!(
        "GET {} HTTP/1.1\r\nHost: {host}\r\nUser-Agent: uison-browser/0.1\r\nAccept: text/html, */*\r\nAccept-Encoding: identity\r\nConnection: close\r\n\r\n",
        url::encode_path(u.path)
    );
    s.write_all(req.as_bytes()).map_err(|e| e.to_string())?;
    let mut raw = Vec::new();
    s.take(MAX_BODY as u64).read_to_end(&mut raw).map_err(|e| e.to_string())?;
    parse_response(&raw)
}

/// Parse a response: status, headers, body (handling `chunked`).
fn parse_response(raw: &[u8]) -> Result<Raw, String> {
    let end = raw.windows(4).position(|w| w == b"\r\n\r\n").ok_or("Response without headers")?;
    let head = String::from_utf8_lossy(&raw[..end]);
    let mut lines = head.split("\r\n");
    let status: u16 = lines.next().and_then(|l| l.split_whitespace().nth(1)).and_then(|c| c.parse().ok()).ok_or("Malformed status line")?;
    let headers: Vec<(String, String)> = lines.filter_map(|l| l.split_once(':')).map(|(k, v)| (k.trim().to_string(), v.trim().to_string())).collect();
    let mut body = raw[end + 4..].to_vec();
    let chunked = headers.iter().any(|(k, v)| k.eq_ignore_ascii_case("transfer-encoding") && v.to_ascii_lowercase().contains("chunked"));
    if chunked {
        body = dechunk(&body);
    } else if let Some(n) = headers.iter().find(|(k, _)| k.eq_ignore_ascii_case("content-length")).and_then(|(_, v)| v.parse::<usize>().ok()) {
        body.truncate(n);
    }
    Ok((status, headers, body))
}

fn dechunk(mut b: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    while let Some(eol) = b.windows(2).position(|w| w == b"\r\n") {
        let size = std::str::from_utf8(&b[..eol]).ok().and_then(|s| usize::from_str_radix(s.split(';').next().unwrap_or("").trim(), 16).ok());
        let Some(size) = size.filter(|&n| n > 0) else { break };
        let start = eol + 2;
        let stop = (start + size).min(b.len());
        out.extend_from_slice(&b[start..stop]);
        b = b.get(stop + 2..).unwrap_or(&[]);
    }
    out
}

/// Page text: encoding from the header, else from `<meta charset>`, else UTF-8.
pub fn decode_html(body: &[u8], content_type: &str) -> String {
    let lower_ct = content_type.to_ascii_lowercase();
    let mut charset = lower_ct.split("charset=").nth(1).map(|c| c.trim_matches(['"', ' ', ';']).to_string());
    if charset.is_none() {
        // <meta charset="…"> or <meta http-equiv=… content="text/html; charset=…"> near the start.
        let head = String::from_utf8_lossy(&body[..body.len().min(2048)]).to_ascii_lowercase();
        if let Some(p) = head.find("charset=") {
            let rest = head[p + 8..].trim_start_matches(['"', '\'']);
            let end = rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_')).unwrap_or(rest.len());
            charset = Some(rest[..end].to_string());
        }
    }
    match charset.as_deref().unwrap_or("utf-8") {
        "windows-1251" | "cp1251" => body.iter().map(|&b| cp1251(b)).collect(),
        "koi8-r" => body.iter().map(|&b| koi8r(b)).collect(),
        "iso-8859-1" | "latin1" | "windows-1252" | "us-ascii" | "ascii" => body.iter().map(|&b| cp1252(b)).collect(),
        _ => String::from_utf8_lossy(body).into_owned(),
    }
}

/// Windows-1252: like Latin-1, but 0x80–0x9F are typographic characters.
fn cp1252(b: u8) -> char {
    const HI: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž', '\u{8f}', '\u{90}', '‘', '’', '“', '”', '•', '–',
        '—', '˜', '™', 'š', '›', 'œ', '\u{9d}', 'ž', 'Ÿ',
    ];
    if (0x80..0xA0).contains(&b) { HI[(b - 0x80) as usize] } else { b as char }
}

/// Windows-1251 (Cyrillic): the table below is the encoding itself.
fn cp1251(b: u8) -> char {
    const HI: &str = "ЂЃ‚ѓ„…†‡€‰Љ‹ЊЌЋЏђ‘’“”•–—\u{98}™љ›њќћџ\u{a0}ЎўЈ¤Ґ¦§Ё©Є«¬\u{ad}®Ї°±Ііґµ¶·ё№є»јЅѕї";
    match b {
        0..=0x7F => b as char,
        0x80..=0xBF => HI.chars().nth((b - 0x80) as usize).unwrap_or('?'),
        _ => char::from_u32(0x410 + (b - 0xC0) as u32).unwrap_or('?'),
    }
}

fn koi8r(b: u8) -> char {
    // KOI8-R (Cyrillic) 0xC0–0xFF: lowercase in KOI8 order, then uppercase; 0xA3/0xB3 — ё/Ё.
    const LOW: &str = "юабцдефгхийклмнопярстужвьызшэщчъ";
    const UP: &str = "ЮАБЦДЕФГХИЙКЛМНОПЯРСТУЖВЬЫЗШЭЩЧЪ";
    match b {
        0..=0x7F => b as char,
        0xA3 => 'ё',
        0xB3 => 'Ё',
        0xC0..=0xDF => LOW.chars().nth((b - 0xC0) as usize).unwrap_or('?'),
        0xE0..=0xFF => UP.chars().nth((b - 0xE0) as usize).unwrap_or('?'),
        _ => '?',
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_chunked_and_plain_responses() {
        let raw = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nContent-Type: text/html\r\n\r\n4\r\nWiki\r\n5;x=1\r\npedia\r\n0\r\n\r\n";
        let (status, headers, body) = parse_response(raw).unwrap();
        assert_eq!((status, body.as_slice()), (200, &b"Wikipedia"[..]));
        assert_eq!(headers[1], ("Content-Type".to_string(), "text/html".to_string()));
        let raw = b"HTTP/1.0 301 Moved\r\nLocation: /new\r\nContent-Length: 3\r\n\r\nabcdef";
        let (status, _, body) = parse_response(raw).unwrap();
        assert_eq!((status, body.as_slice()), (301, &b"abc"[..]));
    }

    #[test]
    fn decodes_charsets() {
        // Cyrillic on purpose: decoding of Cyrillic single-byte encodings.
        assert_eq!(decode_html(&[0xCF, 0xF0, 0xE8, 0xE2, 0xE5, 0xF2], "text/html; charset=windows-1251"), "Привет");
        assert_eq!(decode_html(&[0xF0, 0xD2, 0xC9, 0xD7, 0xC5, 0xD4], "text/html; charset=koi8-r"), "Привет");
        assert_eq!(decode_html(b"<meta charset=iso-8859-1>caf\xe9 \x93q\x94", ""), "<meta charset=iso-8859-1>café “q”");
        assert_eq!(decode_html("ok ✓".as_bytes(), "text/html"), "ok ✓");
    }
}
