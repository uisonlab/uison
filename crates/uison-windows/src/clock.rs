//! Display frames for animation: a thread waits for the next frame and sends `WM_FRAME` to the window.
//! Measurements (Task6.md, experiment 4): `IDXGIOutput::WaitForVBlank` — steady 60 fps;
//! `DwmFlush` — skips every 4th–5th frame; `SetTimer` — 30 fps (16 ms → two quanta
//! of 15.6), with `timeBeginPeriod(1)` — 35–40. `UISON_CLOCK=dwm` — for comparison.

use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::Thread;

use crate::com::{Com, GUID, HRESULT, com, create};
use crate::ffi::{DwmFlush, HMONITOR, HWND, MONITOR_DEFAULTTONEAREST, MonitorFromWindow, PostMessageW, RECT};
use crate::window::WM_FRAME;

const IID_IDXGIFACTORY: GUID = GUID(0x7b7166ec, 0x21c7, 0x44ae, [0xb2, 0x1a, 0xc9, 0xae, 0x32, 0x1a, 0xe3, 0x69]);

#[repr(C)]
struct OutputDesc {
    device_name: [u16; 32],
    desktop: RECT,
    attached: i32,
    rotation: u32,
    monitor: HMONITOR,
}

#[link(name = "dxgi")]
unsafe extern "system" {
    fn CreateDXGIFactory(iid: *const GUID, out: *mut *mut c_void) -> HRESULT;
}

/// The GPU output (`IDXGIOutput`) for a monitor; if none matches, the first one found.
fn output_for(monitor: usize) -> Option<Com> {
    let factory = create(|out| unsafe { CreateDXGIFactory(&IID_IDXGIFACTORY, out) }).ok()?;
    let mut first = None;
    for a in 0.. {
        // EnumAdapters / EnumOutputs: an error means the list has ended.
        let Ok(adapter) = create(|out| unsafe { com!(factory, 7, fn(u32, *mut *mut c_void) -> HRESULT, a, out) }) else { break };
        for o in 0.. {
            let Ok(output) = create(|out| unsafe { com!(adapter, 7, fn(u32, *mut *mut c_void) -> HRESULT, o, out) }) else { break };
            let mut desc: OutputDesc = unsafe { std::mem::zeroed() };
            unsafe { com!(output, 7, fn(*mut OutputDesc) -> HRESULT, &mut desc) }; // GetDesc
            if desc.monitor as usize == monitor {
                return Some(output);
            }
            first.get_or_insert(output);
        }
    }
    first
}

pub struct FrameClock {
    want: Arc<AtomicBool>,
    /// The window's monitor: if the window is dragged, wait for the other one's vblank.
    monitor: Arc<AtomicUsize>,
    hwnd: usize,
    thread: Thread,
}

impl FrameClock {
    pub fn start(hwnd: HWND) -> FrameClock {
        let want = Arc::new(AtomicBool::new(false));
        let monitor = Arc::new(AtomicUsize::new(unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) } as usize));
        let (w, m) = (want.clone(), monitor.clone());
        let hwnd = hwnd as usize;
        let dwm = std::env::var("UISON_CLOCK").as_deref() == Ok("dwm");
        let handle = std::thread::spawn(move || {
            let mut current: Option<(usize, Com)> = None;
            loop {
                if !w.load(Ordering::Acquire) {
                    std::thread::park();
                    continue;
                }
                let mon = m.load(Ordering::Relaxed);
                if !dwm && current.as_ref().is_none_or(|(cm, _)| *cm != mon) {
                    current = output_for(mon).map(|o| (mon, o));
                }
                match &current {
                    Some((_, o)) => unsafe {
                        com!(o, 10, fn() -> HRESULT); // WaitForVBlank
                    },
                    None => unsafe {
                        DwmFlush();
                    },
                }
                if w.swap(false, Ordering::AcqRel) {
                    unsafe { PostMessageW(hwnd as HWND, WM_FRAME, 0, 0) };
                }
            }
        });
        FrameClock { want, monitor, hwnd, thread: handle.thread().clone() }
    }

    /// Send one `WM_FRAME` after the next display frame.
    pub fn request(&self) {
        let mon = unsafe { MonitorFromWindow(self.hwnd as HWND, MONITOR_DEFAULTTONEAREST) } as usize;
        self.monitor.store(mon, Ordering::Relaxed);
        if !self.want.swap(true, Ordering::AcqRel) {
            self.thread.unpark();
        }
    }
}
