//! Minimal D-Bus client, only for the settings portal (XDG Desktop Portal): the desktop's dark
//! theme and its live changes (GNOME, KDE and any environment with the portal), GNOME cursor
//! theme and size (in a GNOME Wayland session `XCURSOR_THEME` is not set).
//! No libdbus: session bus socket, EXTERNAL authentication, messages with a few
//! types (s, o, g, u, v). The reply is awaited in the shared loop, so a slow portal does not delay startup.

use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::net::UnixStream;

unsafe extern "C" {
    fn getuid() -> u32;
}

const METHOD_CALL: u8 = 1;
const METHOD_RETURN: u8 = 2;
const ERROR: u8 = 3;
const SIGNAL: u8 = 4;

/// Message writer (little-endian), alignment from the start of the message.
struct W(Vec<u8>);

impl W {
    fn pad(&mut self, a: usize) {
        while !self.0.len().is_multiple_of(a) {
            self.0.push(0);
        }
    }

    fn u32(&mut self, v: u32) {
        self.pad(4);
        self.0.extend_from_slice(&v.to_le_bytes());
    }

    fn str(&mut self, s: &str) {
        self.u32(s.len() as u32);
        self.0.extend_from_slice(s.as_bytes());
        self.0.push(0);
    }

    fn sig(&mut self, s: &str) {
        self.0.push(s.len() as u8);
        self.0.extend_from_slice(s.as_bytes());
        self.0.push(0);
    }
}

fn method_call(serial: u32, dest: &str, path: &str, iface: &str, member: &str, args: &[&str]) -> Vec<u8> {
    let mut body = W(Vec::new());
    for a in args {
        body.str(a);
    }
    method_call_body(serial, dest, path, iface, member, &"s".repeat(args.len()), body)
}

/// Call with a ready body and its signature (the body starts on an 8-byte boundary, so alignment is the same).
fn method_call_body(serial: u32, dest: &str, path: &str, iface: &str, member: &str, sig: &str, body: W) -> Vec<u8> {
    let mut h = W(vec![b'l', METHOD_CALL, 0, 1]);
    h.u32(body.0.len() as u32);
    h.u32(serial);
    let at = h.0.len();
    h.u32(0);
    // Array length: from the first struct (aligned to 8) to the end.
    h.pad(8);
    let start = h.0.len();
    let field = |h: &mut W, code: u8, t: &str, v: &str| {
        h.pad(8);
        h.0.push(code);
        h.sig(t);
        if t == "g" { h.sig(v) } else { h.str(v) }
    };
    field(&mut h, 1, "o", path);
    field(&mut h, 6, "s", dest);
    field(&mut h, 2, "s", iface);
    field(&mut h, 3, "s", member);
    if !sig.is_empty() {
        field(&mut h, 8, "g", sig);
    }
    let len = (h.0.len() - start) as u32;
    h.0[at..at + 4].copy_from_slice(&len.to_le_bytes());
    h.pad(8);
    h.0.extend_from_slice(&body.0);
    h.0
}

/// Body/header reader; alignment from the start of the message.
struct R<'a> {
    b: &'a [u8],
    pos: usize,
}

impl R<'_> {
    fn pad(&mut self, a: usize) {
        self.pos = self.pos.div_ceil(a) * a;
    }

    fn u8(&mut self) -> Option<u8> {
        let v = *self.b.get(self.pos)?;
        self.pos += 1;
        Some(v)
    }

    fn u32(&mut self) -> Option<u32> {
        self.pad(4);
        let v = u32::from_le_bytes(self.b.get(self.pos..self.pos + 4)?.try_into().ok()?);
        self.pos += 4;
        Some(v)
    }

    fn str(&mut self) -> Option<String> {
        let n = self.u32()? as usize;
        let s = String::from_utf8_lossy(self.b.get(self.pos..self.pos + n)?).into_owned();
        self.pos += n + 1;
        Some(s)
    }

    fn sig(&mut self) -> Option<String> {
        let n = self.u8()? as usize;
        let s = String::from_utf8_lossy(self.b.get(self.pos..self.pos + n)?).into_owned();
        self.pos += n + 1;
        Some(s)
    }

    /// Variant value (possibly nested): a number or a string.
    fn variant(&mut self) -> Option<Val> {
        match self.sig()?.as_str() {
            "v" => self.variant(),
            "u" | "i" => self.u32().map(Val::Num),
            "s" => self.str().map(Val::Str),
            _ => None,
        }
    }
}

enum Val {
    Num(u32),
    Str(String),
}

/// Desktop setting from the portal.
pub enum Setting {
    /// The portal did not open the link (no `OpenURI`): open it another way.
    OpenFailed(String),
    Dark(bool),
    CursorTheme(String),
    CursorSize(i32),
    /// UI font as a Pango description (`Ubuntu Sans 11`).
    Font(String),
}

/// What we read: portal namespace and key. The portal answers the reads concurrently, in any order
/// (on a busy machine the font came seconds after the theme): the first frame waits for both, see [`Portal::settled`].
const KEYS: [(&str, &str); 4] = [
    ("org.gnome.desktop.interface", "font-name"),
    ("org.gnome.desktop.interface", "cursor-theme"),
    ("org.gnome.desktop.interface", "cursor-size"),
    ("org.freedesktop.appearance", "color-scheme"),
];
const SCHEME: usize = 3;
/// Reads the first frame waits for: the font (the layout) and the theme (no flash of a light window).
const FIRST_FRAME: [usize; 2] = [0, SCHEME];

fn setting(ns: &str, key: &str, v: Val) -> Option<Setting> {
    match (ns, key, v) {
        // 0: no preference, 1: dark, 2: light.
        ("org.freedesktop.appearance", "color-scheme", Val::Num(s)) => Some(Setting::Dark(s == 1)),
        ("org.gnome.desktop.interface", "cursor-theme", Val::Str(t)) if !t.is_empty() => Some(Setting::CursorTheme(t)),
        ("org.gnome.desktop.interface", "cursor-size", Val::Num(n)) if n > 0 => Some(Setting::CursorSize(n as i32)),
        ("org.gnome.desktop.interface", "font-name", Val::Str(f)) if !f.is_empty() => Some(Setting::Font(f)),
        _ => None,
    }
}

struct Msg {
    kind: u8,
    reply_to: u32,
    member: String,
    /// The body starts on an 8-byte boundary: alignment from its start equals that from the message start.
    body: Vec<u8>,
}

pub struct Portal {
    sock: UnixStream,
    buf: Vec<u8>,
    serial: u32,
    /// `Read` request serials, in `KEYS` order, and whether each has been answered (a value or an error).
    reads: Vec<u32>,
    answered: [bool; KEYS.len()],
    /// `OpenURI` requests awaiting a reply: serial and link.
    opens: Vec<(u32, String)>,
}

impl Portal {
    /// Connect to the session bus and ask for settings. `None` if there is no bus or `UISON_PORTAL=0`.
    /// Call before creating the window but after the heavy loading (Cairo, Pango, first frame):
    /// the portal answers within tens of milliseconds, so the theme is usually known by the first paint,
    /// and its processes don't take CPU away from our loading.
    pub fn connect() -> Option<Portal> {
        if std::env::var("UISON_PORTAL").as_deref() == Ok("0") {
            return None;
        }
        // No session bus (minimal systems, containers): just the light theme.
        let quiet = !crate::shell::env_flag("UISON_STATS");
        let sock = connect_session().map_err(|e| if !quiet { log!("uison: D-Bus: {e}") }).ok()?;
        let mut p = Portal { sock, buf: Vec::new(), serial: 0, reads: Vec::new(), answered: [false; KEYS.len()], opens: Vec::new() };
        p.auth().map_err(|e| if !quiet { log!("uison: D-Bus auth: {e}") }).ok()?;
        p.call("org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus", "Hello", &[]);
        let rule = "type='signal',interface='org.freedesktop.portal.Settings',member='SettingChanged'";
        p.call("org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus", "AddMatch", &[rule]);
        // Missing key (not GNOME): the portal replies with an error, which we skip.
        for (ns, key) in KEYS {
            let s = p.call("org.freedesktop.portal.Desktop", "/org/freedesktop/portal/desktop", "org.freedesktop.portal.Settings", "Read", &[ns, key]);
            p.reads.push(s);
        }
        p.sock.set_nonblocking(true).ok()?;
        Some(p)
    }

    pub fn fd(&self) -> RawFd {
        self.sock.as_raw_fd()
    }

    fn auth(&mut self) -> io::Result<()> {
        let uid: String = unsafe { getuid() }.to_string().bytes().map(|b| format!("{b:02x}")).collect();
        self.sock.write_all(format!("\0AUTH EXTERNAL {uid}\r\n").as_bytes())?;
        // Before BEGIN the bus sends only this line: read in chunks, not byte by byte.
        let mut line = Vec::new();
        let mut chunk = [0u8; 128];
        while !line.ends_with(b"\r\n") {
            let n = self.sock.read(&mut chunk)?;
            if n == 0 {
                return Err(io::ErrorKind::UnexpectedEof.into());
            }
            line.extend_from_slice(&chunk[..n]);
        }
        if !line.starts_with(b"OK") {
            return Err(io::Error::other(String::from_utf8_lossy(&line).trim().to_string()));
        }
        self.sock.write_all(b"BEGIN\r\n")
    }

    fn call(&mut self, dest: &str, path: &str, iface: &str, member: &str, args: &[&str]) -> u32 {
        self.serial += 1;
        let m = method_call(self.serial, dest, path, iface, member, args);
        let _ = self.sock.write_all(&m);
        self.serial
    }

    /// Open a link via the portal (`OpenURI(s parent, s uri, a{sv} options)`); an error arrives
    /// from `poll` as `Setting::OpenFailed`.
    pub fn open_uri(&mut self, uri: &str) {
        let mut body = W(Vec::new());
        body.str("");
        body.str(uri);
        body.u32(0); // empty a{sv}
        body.pad(8);
        self.serial += 1;
        let m = method_call_body(self.serial, "org.freedesktop.portal.Desktop", "/org/freedesktop/portal/desktop",
            "org.freedesktop.portal.OpenURI", "OpenURI", "ssa{sv}", body);
        // The socket is non-blocking: a short message goes out whole.
        let _ = self.sock.write_all(&m);
        self.opens.push((self.serial, uri.to_string()));
    }

    /// The font and the theme are known (or the portal has neither): the first frame can be drawn.
    pub fn settled(&self) -> bool {
        FIRST_FRAME.iter().all(|&i| self.answered[i])
    }

    /// Read what has arrived: settings that became known or changed.
    pub fn poll(&mut self) -> Vec<Setting> {
        let mut tmp = [0u8; 4096];
        loop {
            match self.sock.read(&mut tmp) {
                Ok(0) => break,
                Ok(n) => self.buf.extend_from_slice(&tmp[..n]),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        let mut out = Vec::new();
        let debug = std::env::var("UISON_WLDEBUG").is_ok_and(|v| v == "1");
        while let Some(m) = self.next() {
            if debug {
                log!("dbus: kind {} reply_to {} member '{}' body {:?}", m.kind, m.reply_to, m.member, &m.body[..m.body.len().min(48)]);
            }
            if let (ERROR | METHOD_RETURN, Some(i)) = (m.kind, self.reads.iter().position(|&s| s == m.reply_to)) {
                self.answered[i] = true;
            }
            let mut r = R { b: &m.body, pos: 0 };
            let got = match m.kind {
                ERROR | METHOD_RETURN if self.opens.iter().any(|o| o.0 == m.reply_to) => {
                    let i = self.opens.iter().position(|o| o.0 == m.reply_to).unwrap_or(0);
                    let (_, uri) = self.opens.remove(i);
                    (m.kind == ERROR).then_some(Setting::OpenFailed(uri))
                }
                METHOD_RETURN => match self.reads.iter().position(|&s| s == m.reply_to) {
                    Some(i) => r.variant().and_then(|v| setting(KEYS[i].0, KEYS[i].1, v)),
                    None => None,
                },
                // Portal without a theme: light; there is a reply, nothing for the first frame to wait for.
                ERROR if self.reads.get(SCHEME) == Some(&m.reply_to) => Some(Setting::Dark(false)),
                SIGNAL if m.member == "SettingChanged" => match (r.str(), r.str()) {
                    (Some(ns), Some(key)) => r.variant().and_then(|v| setting(&ns, &key, v)),
                    _ => None,
                },
                _ => None,
            };
            out.extend(got);
        }
        out
    }

    fn next(&mut self) -> Option<Msg> {
        let b = &self.buf;
        if b.len() < 16 {
            return None;
        }
        if b[0] != b'l' {
            // Big-endian bus: never happens in practice; not parsed.
            self.buf.clear();
            return None;
        }
        let body_len = u32::from_le_bytes(b[4..8].try_into().unwrap()) as usize;
        let fields_len = u32::from_le_bytes(b[12..16].try_into().unwrap()) as usize;
        let body_at = (16 + fields_len).div_ceil(8) * 8;
        let total = body_at + body_len;
        if b.len() < total {
            return None;
        }
        let mut r = R { b: &b[..16 + fields_len], pos: 16 };
        let (mut reply_to, mut member) = (0, String::new());
        while r.pos < 16 + fields_len {
            r.pad(8);
            let Some(code) = r.u8() else { break };
            let Some(t) = r.sig() else { break };
            match t.as_str() {
                "s" | "o" => {
                    let v = r.str().unwrap_or_default();
                    if code == 3 {
                        member = v;
                    }
                }
                "u" => {
                    let v = r.u32().unwrap_or(0);
                    if code == 5 {
                        reply_to = v;
                    }
                }
                "g" => {
                    r.sig();
                }
                _ => break,
            }
        }
        let m = Msg { kind: b[1], reply_to, member, body: b[body_at..total].to_vec() };
        self.buf.drain(..total);
        Some(m)
    }
}

/// `DBUS_SESSION_BUS_ADDRESS`: `unix:path=...` or `unix:abstract=...` (default `$XDG_RUNTIME_DIR/bus`).
fn connect_session() -> io::Result<UnixStream> {
    let addr = std::env::var("DBUS_SESSION_BUS_ADDRESS")
        .unwrap_or_else(|_| format!("unix:path={}/bus", std::env::var("XDG_RUNTIME_DIR").unwrap_or_default()));
    for a in addr.split(';') {
        let Some(rest) = a.strip_prefix("unix:") else { continue };
        for kv in rest.split(',') {
            if let Some(p) = kv.strip_prefix("path=") {
                return UnixStream::connect(p);
            }
            if let Some(name) = kv.strip_prefix("abstract=") {
                use std::os::linux::net::SocketAddrExt;
                let sa = std::os::unix::net::SocketAddr::from_abstract_name(name.as_bytes())?;
                return UnixStream::connect_addr(&sa);
            }
        }
    }
    Err(io::Error::other(format!("unsupported bus address {addr}")))
}
