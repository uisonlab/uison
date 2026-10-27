//! TLS for `https://` comes from the system, with no dependencies: the OS handles certificate
//! verification, root certificates and their updates. Our HTTP/1.1 (net.rs) runs on top.
//!
//! - macOS, iOS: CoreFoundation TLS streams (`CFStreamCreatePairWithSocketToHost` + `NegotiatedSSL`).
//! - Linux: OpenSSL (`libssl.so.3`) via `dlopen` — like Cairo and Pango in uison-linux.
//! - Windows: the whole request is WinHTTP (see `winhttp`); here only the stream for the others.
//! - Android: the system TLS isn't available to apps from C — later via Java.

use std::io::{Read, Write};
#[cfg(any(target_vendor = "apple", target_os = "linux"))]
use std::io;

/// The connection HTTP runs over: plain TCP or TLS.
pub trait Conn: Read + Write + Send {}
impl<T: Read + Write + Send> Conn for T {}

pub fn connect(host: &str, port: u16) -> Result<Box<dyn Conn>, String> {
    imp::connect(host, port)
}

#[cfg(any(target_vendor = "apple", target_os = "linux"))]
fn err(e: impl std::fmt::Display) -> io::Error {
    io::Error::other(e.to_string())
}

#[cfg(target_vendor = "apple")]
mod imp {
    use std::ffi::{c_char, c_void};
    use std::io::{self, Read, Write};

    use super::{Conn, err};

    type Ref = *const c_void;

    // TLS stream properties live in CFNetwork.
    #[link(name = "CFNetwork", kind = "framework")]
    unsafe extern "C" {
        static kCFStreamPropertySocketSecurityLevel: Ref;
        static kCFStreamSocketSecurityLevelNegotiatedSSL: Ref;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFStringCreateWithBytes(alloc: Ref, bytes: *const u8, len: isize, encoding: u32, external: u8) -> Ref;
        fn CFStringGetCString(s: Ref, buf: *mut c_char, size: isize, encoding: u32) -> u8;
        fn CFStreamCreatePairWithSocketToHost(alloc: Ref, host: Ref, port: u32, read: *mut Ref, write: *mut Ref);
        fn CFReadStreamSetProperty(s: Ref, name: Ref, value: Ref) -> u8;
        fn CFReadStreamOpen(s: Ref) -> u8;
        fn CFWriteStreamOpen(s: Ref) -> u8;
        fn CFReadStreamRead(s: Ref, buf: *mut u8, len: isize) -> isize;
        fn CFWriteStreamWrite(s: Ref, buf: *const u8, len: isize) -> isize;
        fn CFReadStreamCopyError(s: Ref) -> Ref;
        fn CFWriteStreamCopyError(s: Ref) -> Ref;
        fn CFReadStreamClose(s: Ref);
        fn CFWriteStreamClose(s: Ref);
        fn CFErrorCopyDescription(e: Ref) -> Ref;
        fn CFErrorGetCode(e: Ref) -> isize;
        fn CFRelease(o: Ref);
    }

    const UTF8: u32 = 0x0800_0100;

    struct Stream {
        r: Ref,
        w: Ref,
    }

    // CoreFoundation streams may be used from any thread (one at a time).
    unsafe impl Send for Stream {}

    /// Text of a CFError (and release it).
    unsafe fn describe(e: Ref) -> String {
        if e.is_null() {
            return "TLS: unknown error".into();
        }
        unsafe {
            // Common Secure Transport codes as readable text.
            let known = match CFErrorGetCode(e) {
                -9807 => Some("certificate verification failed (expired, self-signed or issued for another name)"),
                -9812 | -9813 => Some("certificate issued by an unknown authority"),
                -9814 => Some("certificate expired"),
                -9843 => Some("certificate issued for another name"),
                -9806 | -9805 => Some("connection closed during the TLS handshake"),
                _ => None,
            };
            if let Some(k) = known {
                CFRelease(e);
                return k.into();
            }
            let d = CFErrorCopyDescription(e);
            let mut buf = [0 as c_char; 512];
            CFStringGetCString(d, buf.as_mut_ptr(), buf.len() as isize, UTF8);
            CFRelease(d);
            CFRelease(e);
            std::ffi::CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned()
        }
    }

    pub fn connect(host: &str, port: u16) -> Result<Box<dyn Conn>, String> {
        unsafe {
            let h = CFStringCreateWithBytes(std::ptr::null(), host.as_ptr(), host.len() as isize, UTF8, 0);
            let (mut r, mut w) = (std::ptr::null(), std::ptr::null());
            CFStreamCreatePairWithSocketToHost(std::ptr::null(), h, port as u32, &mut r, &mut w);
            CFRelease(h);
            if r.is_null() || w.is_null() {
                return Err(format!("{host}: couldn't create a connection"));
            }
            let s = Stream { r, w };
            // TLS with certificate verification against the host name (and SNI) — done by the system.
            CFReadStreamSetProperty(r, kCFStreamPropertySocketSecurityLevel, kCFStreamSocketSecurityLevelNegotiatedSSL);
            if CFReadStreamOpen(r) == 0 || CFWriteStreamOpen(w) == 0 {
                return Err(format!("{host}: {}", describe(CFReadStreamCopyError(r))));
            }
            Ok(Box::new(s))
        }
    }

    impl Read for Stream {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let n = unsafe { CFReadStreamRead(self.r, buf.as_mut_ptr(), buf.len() as isize) };
            if n < 0 { Err(err(unsafe { describe(CFReadStreamCopyError(self.r)) })) } else { Ok(n as usize) }
        }
    }

    impl Write for Stream {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let n = unsafe { CFWriteStreamWrite(self.w, buf.as_ptr(), buf.len() as isize) };
            if n < 0 { Err(err(unsafe { describe(CFWriteStreamCopyError(self.w)) })) } else { Ok(n as usize) }
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Drop for Stream {
        fn drop(&mut self) {
            unsafe {
                CFReadStreamClose(self.r);
                CFWriteStreamClose(self.w);
                CFRelease(self.r);
                CFRelease(self.w);
            }
        }
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use std::ffi::{CStr, CString, c_char, c_int, c_long, c_void};
    use std::io::{self, Read, Write};
    use std::net::TcpStream;
    use std::os::fd::AsRawFd;
    use std::sync::OnceLock;

    use super::{Conn, err};

    unsafe extern "C" {
        fn dlopen(name: *const c_char, flags: c_int) -> *mut c_void;
        fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
    }

    type P = *mut c_void;

    /// The OpenSSL 3 / 1.1 functions we need.
    struct Ssl {
        client_method: unsafe extern "C" fn() -> P,
        ctx_new: unsafe extern "C" fn(P) -> P,
        ctx_default_paths: unsafe extern "C" fn(P) -> c_int,
        ctx_set_verify: unsafe extern "C" fn(P, c_int, P),
        ctx_free: unsafe extern "C" fn(P),
        new: unsafe extern "C" fn(P) -> P,
        set_fd: unsafe extern "C" fn(P, c_int) -> c_int,
        ctrl: unsafe extern "C" fn(P, c_int, c_long, P) -> c_long,
        set1_host: unsafe extern "C" fn(P, *const c_char) -> c_int,
        connect: unsafe extern "C" fn(P) -> c_int,
        read: unsafe extern "C" fn(P, *mut c_void, c_int) -> c_int,
        write: unsafe extern "C" fn(P, *const c_void, c_int) -> c_int,
        get_error: unsafe extern "C" fn(P, c_int) -> c_int,
        verify_result: unsafe extern "C" fn(P) -> c_long,
        shutdown: unsafe extern "C" fn(P) -> c_int,
        free: unsafe extern "C" fn(P),
        verify_string: unsafe extern "C" fn(c_long) -> *const c_char,
    }

    // Pointers to the library's functions — shared by all threads.
    unsafe impl Send for Ssl {}
    unsafe impl Sync for Ssl {}

    fn lib() -> Result<&'static Ssl, String> {
        static LIB: OnceLock<Result<Ssl, String>> = OnceLock::new();
        LIB.get_or_init(load).as_ref().map_err(Clone::clone)
    }

    // The function type for `transmute` is inferred from the `Ssl` field — it can't be written in the macro.
    #[allow(clippy::missing_transmute_annotations)]
    fn load() -> Result<Ssl, String> {
        const RTLD_NOW: c_int = 2;
        let open = |names: &[&CStr]| names.iter().map(|n| unsafe { dlopen(n.as_ptr(), RTLD_NOW) }).find(|h| !h.is_null());
        let ssl = open(&[c"libssl.so.3", c"libssl.so.1.1", c"libssl.so"]).ok_or("no libssl (OpenSSL): apt install libssl3")?;
        let crypto = open(&[c"libcrypto.so.3", c"libcrypto.so.1.1", c"libcrypto.so"]).ok_or("no libcrypto (OpenSSL)")?;
        macro_rules! sym {
            ($h:expr, $name:literal) => {{
                let p = unsafe { dlsym($h, $name.as_ptr()) };
                if p.is_null() {
                    return Err(format!("OpenSSL: no {}", $name.to_string_lossy()));
                }
                unsafe { std::mem::transmute::<*mut c_void, _>(p) }
            }};
        }
        Ok(Ssl {
            client_method: sym!(ssl, c"TLS_client_method"),
            ctx_new: sym!(ssl, c"SSL_CTX_new"),
            ctx_default_paths: sym!(ssl, c"SSL_CTX_set_default_verify_paths"),
            ctx_set_verify: sym!(ssl, c"SSL_CTX_set_verify"),
            ctx_free: sym!(ssl, c"SSL_CTX_free"),
            new: sym!(ssl, c"SSL_new"),
            set_fd: sym!(ssl, c"SSL_set_fd"),
            ctrl: sym!(ssl, c"SSL_ctrl"),
            set1_host: sym!(ssl, c"SSL_set1_host"),
            connect: sym!(ssl, c"SSL_connect"),
            read: sym!(ssl, c"SSL_read"),
            write: sym!(ssl, c"SSL_write"),
            get_error: sym!(ssl, c"SSL_get_error"),
            verify_result: sym!(ssl, c"SSL_get_verify_result"),
            shutdown: sym!(ssl, c"SSL_shutdown"),
            free: sym!(ssl, c"SSL_free"),
            verify_string: sym!(crypto, c"X509_verify_cert_error_string"),
        })
    }

    struct Stream {
        lib: &'static Ssl,
        ctx: P,
        ssl: P,
        /// Keeps the socket open; read timeouts are set on it.
        _tcp: TcpStream,
    }

    // SSL* is used by one thread at a time.
    unsafe impl Send for Stream {}

    pub fn connect(host: &str, port: u16) -> Result<Box<dyn Conn>, String> {
        let lib = lib()?;
        let tcp = crate::net::tcp(host, port)?;
        let name = CString::new(host).map_err(|e| e.to_string())?;
        unsafe {
            let ctx = (lib.ctx_new)((lib.client_method)());
            (lib.ctx_default_paths)(ctx);
            (lib.ctx_set_verify)(ctx, 1, std::ptr::null_mut()); // SSL_VERIFY_PEER
            let ssl = (lib.new)(ctx);
            let s = Stream { lib, ctx, ssl, _tcp: tcp };
            (lib.set_fd)(ssl, s._tcp.as_raw_fd());
            // SNI (SSL_set_tlsext_host_name is a macro over SSL_ctrl) and the certificate name check.
            (lib.ctrl)(ssl, 55, 0, name.as_ptr() as P);
            (lib.set1_host)(ssl, name.as_ptr());
            if (lib.connect)(ssl) != 1 {
                let v = (lib.verify_result)(ssl);
                let why = if v != 0 { CStr::from_ptr((lib.verify_string)(v)).to_string_lossy().into_owned() } else { "TLS handshake failed".into() };
                return Err(format!("{host}: {why}"));
            }
            Ok(Box::new(s))
        }
    }

    impl Read for Stream {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let n = unsafe { (self.lib.read)(self.ssl, buf.as_mut_ptr().cast(), buf.len().min(i32::MAX as usize) as c_int) };
            if n > 0 {
                return Ok(n as usize);
            }
            match unsafe { (self.lib.get_error)(self.ssl, n) } {
                // ZERO_RETURN — closed properly; SYSCALL with 0 — the server just closed the socket.
                6 | 5 => Ok(0),
                e => Err(err(format!("TLS: read error {e}"))),
            }
        }
    }

    impl Write for Stream {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let n = unsafe { (self.lib.write)(self.ssl, buf.as_ptr().cast(), buf.len().min(i32::MAX as usize) as c_int) };
            if n > 0 { Ok(n as usize) } else { Err(err("TLS: write error")) }
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Drop for Stream {
        fn drop(&mut self) {
            unsafe {
                (self.lib.shutdown)(self.ssl);
                (self.lib.free)(self.ssl);
                (self.lib.ctx_free)(self.ctx);
            }
        }
    }
}

#[cfg(not(any(target_vendor = "apple", target_os = "linux")))]
mod imp {
    use super::Conn;

    pub fn connect(host: &str, _port: u16) -> Result<Box<dyn Conn>, String> {
        Err(format!("{host}: HTTPS isn't supported on this platform yet"))
    }
}
