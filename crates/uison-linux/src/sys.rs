//! libc by hand, only what std lacks: passing descriptors over a socket,
//! `mmap`, `memfd_create`, `poll`, `pipe2`.

use std::ffi::{c_char, c_int, c_uint, c_void};
use std::fs::File;
use std::io;
use std::os::fd::{FromRawFd, OwnedFd, RawFd};

#[repr(C)]
pub struct IoVec {
    pub base: *mut c_void,
    pub len: usize,
}

#[repr(C)]
pub struct MsgHdr {
    pub name: *mut c_void,
    pub namelen: u32,
    pub iov: *mut IoVec,
    pub iovlen: usize,
    pub control: *mut c_void,
    pub controllen: usize,
    pub flags: c_int,
}

#[repr(C)]
pub struct CmsgHdr {
    pub len: usize,
    pub level: c_int,
    pub kind: c_int,
}

#[repr(C)]
pub struct PollFd {
    pub fd: c_int,
    pub events: i16,
    pub revents: i16,
}

unsafe extern "C" {
    pub fn sendmsg(fd: c_int, msg: *const MsgHdr, flags: c_int) -> isize;
    pub fn recvmsg(fd: c_int, msg: *mut MsgHdr, flags: c_int) -> isize;
    pub fn poll(fds: *mut PollFd, n: u64, timeout: c_int) -> c_int;
    fn memfd_create(name: *const c_char, flags: c_uint) -> c_int;
    fn mmap(addr: *mut c_void, len: usize, prot: c_int, flags: c_int, fd: c_int, off: i64) -> *mut c_void;
    fn munmap(addr: *mut c_void, len: usize) -> c_int;
    fn pipe2(fds: *mut c_int, flags: c_int) -> c_int;
    fn posix_spawnp(
        pid: *mut c_int,
        file: *const c_char,
        actions: *const c_void,
        attr: *const c_void,
        argv: *const *const c_char,
        envp: *const *const c_char,
    ) -> c_int;
    static environ: *const *const c_char;
    fn eventfd(initval: c_uint, flags: c_int) -> c_int;
    fn write(fd: c_int, buf: *const c_void, n: usize) -> isize;
    fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
    fn fcntl(fd: c_int, cmd: c_int, arg: c_int) -> c_int;
}

const O_CLOEXEC: c_int = 0o2_000_000;
const O_NONBLOCK: c_int = 0o4000;
const F_SETFL: c_int = 4;

/// Start a program from PATH with arguments without waiting for it. `std::process::Command` weighs
/// ~30 KB; `posix_spawnp` is enough for `xdg-open`.
pub fn spawn(argv: &[&str]) -> io::Result<()> {
    let owned: Vec<Vec<u8>> = argv.iter().map(|a| [a.as_bytes(), b"\0"].concat()).collect();
    let mut ptrs: Vec<*const c_char> = owned.iter().map(|a| a.as_ptr().cast()).collect();
    ptrs.push(std::ptr::null());
    let mut pid = 0;
    let r = unsafe { posix_spawnp(&mut pid, ptrs[0], std::ptr::null(), std::ptr::null(), ptrs.as_ptr(), environ) };
    if r != 0 { Err(io::Error::from_raw_os_error(r)) } else { Ok(()) }
}

/// Event loop wake-up: `eventfd`, written from any thread, `poll` waits for it to be readable.
pub struct WakeFd(OwnedFd);

impl WakeFd {
    pub fn new() -> io::Result<WakeFd> {
        const EFD_CLOEXEC: c_int = 0o2_000_000;
        const EFD_NONBLOCK: c_int = 0o4000;
        let fd = unsafe { eventfd(0, EFD_CLOEXEC | EFD_NONBLOCK) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(WakeFd(unsafe { OwnedFd::from_raw_fd(fd) }))
    }

    pub fn fd(&self) -> RawFd {
        use std::os::fd::AsRawFd;
        self.0.as_raw_fd()
    }

    /// Wake up (from any thread): `fd` is `self.fd()`; the descriptor lives as long as the `WakeFd`.
    pub fn wake(fd: RawFd) {
        let one: u64 = 1;
        unsafe { write(fd, (&one as *const u64).cast(), 8) };
    }

    /// Reset the counter after waking up.
    pub fn drain(&self) {
        let mut v: u64 = 0;
        unsafe { read(self.fd(), (&mut v as *mut u64).cast(), 8) };
    }
}

/// Pipe (read, write): the read end is non-blocking (`poll` waits for it), the write end is normal:
/// it goes to another program (Wayland clipboard).
pub fn pipe() -> io::Result<(OwnedFd, OwnedFd)> {
    let mut fds = [0 as c_int; 2];
    if unsafe { pipe2(fds.as_mut_ptr(), O_CLOEXEC) } < 0 {
        return Err(io::Error::last_os_error());
    }
    let (r, w) = unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) };
    unsafe { fcntl(fds[0], F_SETFL, O_NONBLOCK) };
    Ok((r, w))
}

pub const SOL_SOCKET: c_int = 1;
pub const SCM_RIGHTS: c_int = 1;
pub const MSG_DONTWAIT: c_int = 0x40;
pub const MSG_NOSIGNAL: c_int = 0x4000;
pub const MSG_CMSG_CLOEXEC: c_int = 0x4000_0000;
pub const POLLIN: i16 = 1;
pub const POLLERR: i16 = 8;
pub const POLLHUP: i16 = 0x10;
const PROT_READ: c_int = 1;
const PROT_WRITE: c_int = 2;
const MAP_SHARED: c_int = 1;
const MAP_PRIVATE: c_int = 2;
const MFD_CLOEXEC: c_uint = 1;

/// A control message header is aligned to `usize`.
pub const fn cmsg_align(n: usize) -> usize {
    (n + size_of::<usize>() - 1) & !(size_of::<usize>() - 1)
}

/// Anonymous in-memory file of the given size (a buffer for the compositor).
pub fn memfd(size: usize) -> io::Result<File> {
    let fd = unsafe { memfd_create(c"uison".as_ptr(), MFD_CLOEXEC) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    let f = unsafe { File::from_raw_fd(fd) };
    f.set_len(size as u64)?;
    Ok(f)
}

/// File mapped into memory; unmapped in `Drop`.
pub struct Map {
    pub ptr: *mut u8,
    pub len: usize,
}

impl Map {
    pub fn shared(fd: RawFd, len: usize) -> io::Result<Map> {
        Self::new(fd, len, PROT_READ | PROT_WRITE, MAP_SHARED)
    }

    /// Read-only (keymap from the compositor: in Wayland 7+ it may only be mapped privately).
    pub fn read_only(fd: RawFd, len: usize) -> io::Result<Map> {
        Self::new(fd, len, PROT_READ, MAP_PRIVATE)
    }

    fn new(fd: RawFd, len: usize, prot: c_int, flags: c_int) -> io::Result<Map> {
        let p = unsafe { mmap(std::ptr::null_mut(), len, prot, flags, fd, 0) };
        if p as isize == -1 {
            return Err(io::Error::last_os_error());
        }
        Ok(Map { ptr: p.cast(), len })
    }

    pub fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl Drop for Map {
    fn drop(&mut self) {
        unsafe { munmap(self.ptr.cast(), self.len) };
    }
}

/// Wait until `fd` is readable, at most `timeout_ms` (-1: no limit).
/// `Ok(true)`: can read (or the connection is closed; the read will show it).
pub fn wait_readable(fd: RawFd, timeout_ms: i32) -> io::Result<bool> {
    Ok(wait_any(&[fd], timeout_ms)? & 1 != 0)
}

/// Wait until any of the descriptors is readable; result bits tell which are ready.
pub fn wait_any(fds: &[RawFd], timeout_ms: i32) -> io::Result<u32> {
    let mut p: Vec<PollFd> = fds.iter().map(|&fd| PollFd { fd, events: POLLIN, revents: 0 }).collect();
    loop {
        let r = unsafe { poll(p.as_mut_ptr(), p.len() as u64, timeout_ms) };
        if r < 0 {
            let e = io::Error::last_os_error();
            if e.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(e);
        }
        let mut bits = 0;
        for (i, x) in p.iter().enumerate() {
            if x.revents & (POLLIN | POLLERR | POLLHUP) != 0 {
                bits |= 1 << i;
            }
        }
        return Ok(bits);
    }
}
