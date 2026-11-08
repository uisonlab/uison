//! App requests to Windows: clipboard (`CF_UNICODETEXT`), links (`ShellExecuteW`),
//! modifiers for the mouse (`GetKeyState`).

use std::cell::Cell;
use std::ptr::null;

use uison_core::{Font, Host, Mods, TextMeasure, TextMetrics};

use crate::ffi::*;
use crate::text::Text;

/// The platform for the duration of one event; the window performs the paste after the event.
pub struct WinHost<'a> {
    pub text: &'a Text,
    pub hwnd: HWND,
    pub paste: &'a Cell<bool>,
}

impl TextMeasure for WinHost<'_> {
    fn measure(&self, text: &str, font: &Font) -> TextMetrics {
        self.text.measure(text, font)
    }
}

impl Host for WinHost<'_> {
    fn mods(&self) -> Mods {
        let down = |k: i32| unsafe { GetKeyState(k) } < 0;
        // The main modifier is Ctrl (`cmd`), as for keys.
        Mods { shift: down(VK_SHIFT), ctrl: false, alt: down(VK_MENU), cmd: down(VK_CONTROL) }
    }

    fn set_clipboard(&self, text: &str) {
        let w: Vec<u16> = text.encode_utf16().chain([0]).collect();
        unsafe {
            if !open(self.hwnd) {
                return;
            }
            EmptyClipboard();
            let h = GlobalAlloc(GMEM_MOVEABLE, w.len() * 2);
            let p = GlobalLock(h).cast::<u16>();
            if !p.is_null() {
                std::ptr::copy_nonoverlapping(w.as_ptr(), p, w.len());
                GlobalUnlock(h);
                // The system takes ownership of the memory; if it refuses, free it ourselves.
                if SetClipboardData(CF_UNICODETEXT, h).is_null() {
                    GlobalFree(h);
                }
            }
            CloseClipboard();
        }
    }

    fn request_paste(&self) {
        self.paste.set(true);
    }

    fn open_url(&self, url: &str) -> bool {
        let (verb, url) = (wide("open"), wide(url));
        // Greater than 32 means success (ShellExecute's historical convention).
        unsafe { ShellExecuteW(self.hwnd, verb.as_ptr(), url.as_ptr(), null(), null(), SW_SHOWNORMAL) as usize > 32 }
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain([0]).collect()
}

/// Another program may hold the clipboard for a moment — retry a few times.
unsafe fn open(hwnd: HWND) -> bool {
    for _ in 0..5 {
        if unsafe { OpenClipboard(hwnd) } != 0 {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    false
}

/// Text from the clipboard.
pub fn clipboard(hwnd: HWND) -> Option<String> {
    unsafe {
        if !open(hwnd) {
            return None;
        }
        let h = GetClipboardData(CF_UNICODETEXT);
        let mut out = None;
        let p = if h.is_null() { std::ptr::null_mut() } else { GlobalLock(h).cast::<u16>() };
        if !p.is_null() {
            // Up to NUL, but not past the memory block.
            let max = GlobalSize(h) / 2;
            let n = (0..max).find(|&i| *p.add(i) == 0).unwrap_or(max);
            out = Some(String::from_utf16_lossy(std::slice::from_raw_parts(p, n)));
            GlobalUnlock(h);
        }
        CloseClipboard();
        out
    }
}
