//! Wayland protocol without libwayland: messages over a unix socket.
//! A message is `object id | (size << 16) | opcode`, arguments are 32-bit:
//! int, uint, fixed (24.8), string and array (length + bytes padded to 4); descriptors go
//! separately, via `SCM_RIGHTS`. Message opcodes follow the protocol XML (`wl.rs`).

use std::collections::VecDeque;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::net::UnixStream;

use crate::sys::*;

pub enum Arg<'a> {
    Int(i32),
    Uint(u32),
    /// Only `tools/lindrive.rs` needs it so far.
    #[allow(dead_code)]
    Fixed(f64),
    Str(&'a str),
    Obj(u32),
    New(u32),
    /// `new_id` without a type in the XML (`wl_registry.bind`): interface, version, id.
    NewAny(&'a str, u32, u32),
    Fd(RawFd),
}

/// How many descriptors can be received in one read.
const MAX_FDS: usize = 28;

pub struct Conn {
    sock: UnixStream,
    out: Vec<u8>,
    out_fds: Vec<RawFd>,
    inb: Vec<u8>,
    in_fds: VecDeque<OwnedFd>,
    next_id: u32,
}

/// Event: object, opcode, body.
pub struct Msg {
    pub obj: u32,
    pub op: u16,
    pub body: Vec<u8>,
}

impl Conn {
    /// `WAYLAND_DISPLAY` (a name in `XDG_RUNTIME_DIR` or an absolute path), `wayland-0` by default.
    pub fn connect() -> Result<Conn, String> {
        let name = std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "wayland-0".into());
        let path = if name.starts_with('/') {
            name
        } else {
            let dir = std::env::var("XDG_RUNTIME_DIR").map_err(|_| "XDG_RUNTIME_DIR is not set")?;
            format!("{dir}/{name}")
        };
        let sock = UnixStream::connect(&path).map_err(|e| format!("{path}: {e}"))?;
        // id 1 — wl_display.
        Ok(Conn { sock, out: Vec::with_capacity(4096), out_fds: Vec::new(), inb: Vec::new(), in_fds: VecDeque::new(), next_id: 2 })
    }

    pub fn fd(&self) -> RawFd {
        self.sock.as_raw_fd()
    }

    pub fn new_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn request(&mut self, obj: u32, op: u16, args: &[Arg]) {
        let start = self.out.len();
        self.out.extend_from_slice(&obj.to_ne_bytes());
        self.out.extend_from_slice(&0u32.to_ne_bytes());
        let put_str = |out: &mut Vec<u8>, s: &str| {
            out.extend_from_slice(&(s.len() as u32 + 1).to_ne_bytes());
            out.extend_from_slice(s.as_bytes());
            out.push(0);
            while !out.len().is_multiple_of(4) {
                out.push(0);
            }
        };
        for a in args {
            match *a {
                Arg::Int(v) => self.out.extend_from_slice(&v.to_ne_bytes()),
                Arg::Uint(v) | Arg::Obj(v) | Arg::New(v) => self.out.extend_from_slice(&v.to_ne_bytes()),
                Arg::Fixed(v) => self.out.extend_from_slice(&((v * 256.0).round() as i32).to_ne_bytes()),
                Arg::Str(s) => put_str(&mut self.out, s),
                Arg::NewAny(iface, version, id) => {
                    put_str(&mut self.out, iface);
                    self.out.extend_from_slice(&version.to_ne_bytes());
                    self.out.extend_from_slice(&id.to_ne_bytes());
                }
                Arg::Fd(fd) => self.out_fds.push(fd),
            }
        }
        let size = (self.out.len() - start) as u32;
        self.out[start + 4..start + 8].copy_from_slice(&((size << 16) | op as u32).to_ne_bytes());
        // Descriptors go out with the first byte of a batch: don't queue more than fit.
        if self.out_fds.len() >= MAX_FDS {
            let _ = self.flush();
        }
    }

    /// Send what has accumulated. The socket is blocking: the compositor reads fast.
    pub fn flush(&mut self) -> io::Result<()> {
        let mut sent = 0;
        while sent < self.out.len() {
            let mut iov = IoVec { base: self.out[sent..].as_mut_ptr().cast(), len: self.out.len() - sent };
            let mut cbuf = [0usize; (cmsg_align(size_of::<CmsgHdr>()) + MAX_FDS * 4).div_ceil(8)];
            let mut msg = MsgHdr { name: std::ptr::null_mut(), namelen: 0, iov: &mut iov, iovlen: 1, control: std::ptr::null_mut(), controllen: 0, flags: 0 };
            if !self.out_fds.is_empty() {
                let n = self.out_fds.len() * 4;
                let hdr = cbuf.as_mut_ptr() as *mut CmsgHdr;
                unsafe {
                    (*hdr).len = cmsg_align(size_of::<CmsgHdr>()) + n;
                    (*hdr).level = SOL_SOCKET;
                    (*hdr).kind = SCM_RIGHTS;
                    let data = (hdr as *mut u8).add(cmsg_align(size_of::<CmsgHdr>()));
                    std::ptr::copy_nonoverlapping(self.out_fds.as_ptr().cast::<u8>(), data, n);
                }
                msg.control = cbuf.as_mut_ptr().cast();
                msg.controllen = cmsg_align(cmsg_align(size_of::<CmsgHdr>()) + n);
            }
            let r = unsafe { sendmsg(self.fd(), &msg, MSG_NOSIGNAL) };
            if r < 0 {
                let e = io::Error::last_os_error();
                if e.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(e);
            }
            self.out_fds.clear();
            sent += r as usize;
        }
        self.out.clear();
        Ok(())
    }

    /// Read whatever has arrived (without blocking). `Ok(false)`: connection closed.
    pub fn read(&mut self) -> io::Result<bool> {
        let mut buf = [0u8; 8192];
        loop {
            let mut iov = IoVec { base: buf.as_mut_ptr().cast(), len: buf.len() };
            let mut cbuf = [0usize; (cmsg_align(size_of::<CmsgHdr>()) + MAX_FDS * 4).div_ceil(8)];
            let mut msg = MsgHdr {
                name: std::ptr::null_mut(),
                namelen: 0,
                iov: &mut iov,
                iovlen: 1,
                control: cbuf.as_mut_ptr().cast(),
                controllen: size_of_val(&cbuf),
                flags: 0,
            };
            let r = unsafe { recvmsg(self.fd(), &mut msg, MSG_DONTWAIT | MSG_CMSG_CLOEXEC) };
            if r < 0 {
                let e = io::Error::last_os_error();
                match e.kind() {
                    io::ErrorKind::Interrupted => continue,
                    io::ErrorKind::WouldBlock => return Ok(true),
                    _ => return Err(e),
                }
            }
            if r == 0 {
                return Ok(false);
            }
            if msg.controllen >= size_of::<CmsgHdr>() {
                let hdr = cbuf.as_ptr() as *const CmsgHdr;
                unsafe {
                    if (*hdr).level == SOL_SOCKET && (*hdr).kind == SCM_RIGHTS {
                        let n = ((*hdr).len - cmsg_align(size_of::<CmsgHdr>())) / 4;
                        let data = (hdr as *const u8).add(cmsg_align(size_of::<CmsgHdr>())) as *const RawFd;
                        for i in 0..n {
                            self.in_fds.push_back(OwnedFd::from_raw_fd(data.add(i).read_unaligned()));
                        }
                    }
                }
            }
            self.inb.extend_from_slice(&buf[..r as usize]);
            if (r as usize) < buf.len() {
                return Ok(true);
            }
        }
    }

    /// The next fully received event.
    pub fn next(&mut self) -> Option<Msg> {
        if self.inb.len() < 8 {
            return None;
        }
        let word = |i: usize| u32::from_ne_bytes(self.inb[i..i + 4].try_into().unwrap());
        let obj = word(0);
        let sz_op = word(4);
        let size = (sz_op >> 16) as usize;
        if size < 8 || self.inb.len() < size {
            return None;
        }
        let body = self.inb[8..size].to_vec();
        self.inb.drain(..size);
        Some(Msg { obj, op: (sz_op & 0xffff) as u16, body })
    }

    /// A descriptor from the queue: events with an `fd` take them in order.
    pub fn take_fd(&mut self) -> Option<OwnedFd> {
        self.in_fds.pop_front()
    }

    /// Wait for a reply synchronously (startup): read at least something.
    pub fn read_blocking(&mut self) -> io::Result<bool> {
        wait_readable(self.fd(), -1)?;
        self.read()
    }
}

/// Event body parser.
pub struct Body<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> Body<'a> {
    pub fn new(b: &'a [u8]) -> Body<'a> {
        Body { b, pos: 0 }
    }

    pub fn u32(&mut self) -> u32 {
        let v = self.b.get(self.pos..self.pos + 4).map_or(0, |s| u32::from_ne_bytes(s.try_into().unwrap()));
        self.pos += 4;
        v
    }

    pub fn i32(&mut self) -> i32 {
        self.u32() as i32
    }

    pub fn fixed(&mut self) -> f32 {
        self.i32() as f32 / 256.0
    }

    pub fn array(&mut self) -> &'a [u8] {
        let n = self.u32() as usize;
        let s = self.b.get(self.pos..self.pos + n).unwrap_or(&[]);
        self.pos += n.div_ceil(4) * 4;
        s
    }

    pub fn str(&mut self) -> &'a str {
        let a = self.array();
        std::str::from_utf8(a.strip_suffix(&[0]).unwrap_or(a)).unwrap_or("")
    }
}
