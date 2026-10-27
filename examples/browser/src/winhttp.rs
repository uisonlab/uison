//! HTTPS on Windows: the system WinHTTP client (TLS via SChannel, certificates from the Windows store).
//! It follows redirects itself; the final address is `WINHTTP_OPTION_URL`.

use std::ffi::c_void;

use crate::net::Response;
use crate::url;

type H = *mut c_void;

#[link(name = "winhttp")]
unsafe extern "system" {
    fn WinHttpOpen(agent: *const u16, access: u32, proxy: *const u16, bypass: *const u16, flags: u32) -> H;
    fn WinHttpConnect(session: H, server: *const u16, port: u16, reserved: u32) -> H;
    fn WinHttpOpenRequest(conn: H, verb: *const u16, path: *const u16, version: *const u16, referrer: *const u16, accept: *const *const u16, flags: u32) -> H;
    fn WinHttpSetTimeouts(h: H, resolve: i32, connect: i32, send: i32, receive: i32) -> i32;
    fn WinHttpSendRequest(req: H, headers: *const u16, headers_len: u32, data: *const c_void, data_len: u32, total: u32, ctx: usize) -> i32;
    fn WinHttpReceiveResponse(req: H, reserved: *mut c_void) -> i32;
    fn WinHttpQueryHeaders(req: H, level: u32, name: *const u16, buf: *mut c_void, len: *mut u32, index: *mut u32) -> i32;
    fn WinHttpQueryOption(h: H, option: u32, buf: *mut c_void, len: *mut u32) -> i32;
    fn WinHttpReadData(req: H, buf: *mut c_void, len: u32, read: *mut u32) -> i32;
    fn WinHttpCloseHandle(h: H) -> i32;
    fn WinHttpSetStatusCallback(h: H, cb: Option<StatusCallback>, flags: u32, reserved: usize) -> *mut c_void;
    fn WinHttpSetOption(h: H, option: u32, buf: *const c_void, len: u32) -> i32;
}

type StatusCallback = unsafe extern "system" fn(h: H, ctx: usize, status: u32, info: *mut c_void, len: u32);

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetLastError() -> u32;
}

const ACCESS_AUTOMATIC_PROXY: u32 = 4;
const FLAG_SECURE: u32 = 0x0080_0000;
const QUERY_STATUS_CODE: u32 = 19;
const QUERY_CONTENT_TYPE: u32 = 1;
const QUERY_FLAG_NUMBER: u32 = 0x2000_0000;
const OPTION_URL: u32 = 34;
const OPTION_ENABLE_FEATURE: u32 = 79;
const ENABLE_SSL_REVOCATION: u32 = 1;
/// WINHTTP_CALLBACK_FLAG_SECURE_FAILURE and WINHTTP_CALLBACK_STATUS_SECURE_FAILURE have the same value.
const SECURE_FAILURE: u32 = 0x0001_0000;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}

/// Close the handle when leaving the function.
struct Handle(H);

impl Drop for Handle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { WinHttpCloseHandle(self.0) };
        }
    }
}

/// Why TLS failed: WinHTTP reports it only to a status callback, as flags in a DWORD; `ctx`
/// (the `WinHttpSendRequest` context) points to our `u32`.
unsafe extern "system" fn on_status(_h: H, ctx: usize, status: u32, info: *mut c_void, _len: u32) {
    if status == SECURE_FAILURE && ctx != 0 && !info.is_null() {
        unsafe { *(ctx as *mut u32) |= *(info as *const u32) };
    }
}

/// Text for common WinHTTP errors; `cert` — the secure failure flags.
fn error_text(code: u32, cert: u32) -> String {
    let reasons = [
        (0x20, "the certificate has expired or is not yet valid"),
        (0x10, "the certificate is for another host"),
        (0x08, "the certificate is not from a trusted authority (self-signed or unknown root)"),
        (0x04, "the certificate is revoked"),
        (0x02, "the certificate is invalid"),
        (0x01, "the revocation check failed"),
        (0x8000_0000, "TLS error"),
    ];
    match code {
        12175 => {
            let why: Vec<&str> = reasons.iter().filter(|(f, _)| cert & f != 0).map(|(_, t)| *t).collect();
            if why.is_empty() { "insecure connection".into() } else { format!("insecure connection: {}", why.join("; ")) }
        }
        12007 => "server name not found".into(),
        12029 => "could not connect to the server".into(),
        12002 => "the server did not answer in time".into(),
        12030 | 12031 => "the connection was closed".into(),
        12152 => "invalid response from the server".into(),
        c => format!("WinHTTP error {c}"),
    }
}

fn check(h: H, what: &str) -> Result<Handle, String> {
    if h.is_null() { Err(format!("WinHTTP {what}: error {}", unsafe { GetLastError() })) } else { Ok(Handle(h)) }
}

pub fn get(target: &str) -> Result<Response, String> {
    let u = url::parse(target).ok_or_else(|| format!("Not an HTTP address: {target}"))?;
    unsafe {
        let session = check(WinHttpOpen(wide("uison-browser/0.1").as_ptr(), ACCESS_AUTOMATIC_PROXY, std::ptr::null(), std::ptr::null(), 0), "open")?;
        WinHttpSetTimeouts(session.0, 10_000, 10_000, 10_000, 10_000);
        let conn = check(WinHttpConnect(session.0, wide(u.host).as_ptr(), u.port, 0), "connect")?;
        let flags = if u.scheme == "https" { FLAG_SECURE } else { 0 };
        let req = check(
            WinHttpOpenRequest(conn.0, wide("GET").as_ptr(), wide(&url::encode_path(u.path)).as_ptr(), std::ptr::null(), std::ptr::null(), std::ptr::null(), flags),
            "request",
        )?;
        let headers = wide("Accept: text/html, */*\r\n");
        let mut cert = 0u32;
        // Off by default in WinHTTP: a revoked certificate opened the page. The cost is the first visit
        // (CRL or OCSP, then cached by the system): example.com 0.49 s instead of 0.22.
        let on = ENABLE_SSL_REVOCATION;
        WinHttpSetOption(req.0, OPTION_ENABLE_FEATURE, (&raw const on).cast(), 4);
        WinHttpSetStatusCallback(req.0, Some(on_status), SECURE_FAILURE, 0);
        let ctx = &raw mut cert as usize;
        if WinHttpSendRequest(req.0, headers.as_ptr(), u32::MAX, std::ptr::null(), 0, 0, ctx) == 0 || WinHttpReceiveResponse(req.0, std::ptr::null_mut()) == 0 {
            let code = GetLastError();
            return Err(format!("{}: {}", u.host, error_text(code, cert)));
        }
        let mut status = 0u32;
        let mut len = 4u32;
        WinHttpQueryHeaders(req.0, QUERY_STATUS_CODE | QUERY_FLAG_NUMBER, std::ptr::null(), (&mut status as *mut u32).cast(), &mut len, std::ptr::null_mut());
        if status >= 400 {
            return Err(format!("Server responded {status}"));
        }
        let text = |level_or_option: u32, header: bool| -> String {
            let mut buf = [0u16; 2048];
            let mut len = (buf.len() * 2) as u32;
            let ok = if header {
                WinHttpQueryHeaders(req.0, level_or_option, std::ptr::null(), buf.as_mut_ptr().cast(), &mut len, std::ptr::null_mut())
            } else {
                WinHttpQueryOption(req.0, level_or_option, buf.as_mut_ptr().cast(), &mut len)
            };
            if ok == 0 { String::new() } else { String::from_utf16_lossy(&buf[..(len as usize / 2).min(buf.len())]).trim_end_matches('\0').to_string() }
        };
        let content_type = text(QUERY_CONTENT_TYPE, true);
        let final_url = Some(text(OPTION_URL, false)).filter(|s| !s.is_empty()).unwrap_or_else(|| target.to_string());
        let mut body = Vec::new();
        let mut chunk = vec![0u8; 64 * 1024];
        loop {
            let mut read = 0u32;
            if WinHttpReadData(req.0, chunk.as_mut_ptr().cast(), chunk.len() as u32, &mut read) == 0 {
                return Err(format!("{}: {}", u.host, error_text(GetLastError(), 0)));
            }
            if read == 0 || body.len() > 8 << 20 {
                break;
            }
            body.extend_from_slice(&chunk[..read as usize]);
        }
        Ok(Response { url: final_url, content_type, body })
    }
}
