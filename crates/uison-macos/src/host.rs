//! App requests to macOS: clipboard, links, modifiers for the mouse.

use std::cell::Cell;
use std::ffi::{CStr, c_char};

use uison_core::{Font, Host, Mods, TextMeasure, TextMetrics};

use uison_apple::ffi::*;
use uison_apple::{Text, class, sel};

/// The platform for the duration of one event. The backend performs paste after the event:
/// `Event::Paste` can't be given to the app while it is handling the previous one.
pub struct MacHost<'a> {
    pub text: &'a Text,
    pub paste: &'a Cell<bool>,
}

impl TextMeasure for MacHost<'_> {
    fn measure(&self, text: &str, font: &Font) -> TextMetrics {
        self.text.measure(text, font)
    }
}

impl Host for MacHost<'_> {
    fn mods(&self) -> Mods {
        mods(unsafe { msg(class!("NSEvent"), sel!("modifierFlags"), ()) })
    }

    fn set_clipboard(&self, text: &str) {
        unsafe {
            let pb: Id = msg(class!("NSPasteboard"), sel!("generalPasteboard"), ());
            msg::<isize, _>(pb, sel!("clearContents"), ());
            let (s, ty) = (nsstring(text), nsstring(PLAIN_TEXT));
            msg::<bool, _>(pb, sel!("setString:forType:"), (s, ty));
            release(s);
            release(ty);
        }
    }

    fn request_paste(&self) {
        self.paste.set(true);
    }

    fn open_url(&self, url: &str) -> bool {
        unsafe {
            let s = nsstring(url);
            let u: Id = msg(class!("NSURL"), sel!("URLWithString:"), (s,));
            release(s);
            if u.is_null() {
                return false;
            }
            let ws: Id = msg(class!("NSWorkspace"), sel!("sharedWorkspace"), ());
            msg(ws, sel!("openURL:"), (u,))
        }
    }
}

/// NSPasteboardTypeString.
const PLAIN_TEXT: &str = "public.utf8-plain-text";

/// Text from the system clipboard.
pub fn clipboard() -> Option<String> {
    unsafe {
        let pb: Id = msg(class!("NSPasteboard"), sel!("generalPasteboard"), ());
        let ty = nsstring(PLAIN_TEXT);
        let s: Id = msg(pb, sel!("stringForType:"), (ty,));
        release(ty);
        if s.is_null() {
            return None;
        }
        let c: *const c_char = msg(s, sel!("UTF8String"), ());
        (!c.is_null()).then(|| CStr::from_ptr(c).to_string_lossy().into_owned())
    }
}

/// Modifiers from `NSEventModifierFlags`.
pub fn mods(flags: usize) -> Mods {
    Mods { shift: flags & (1 << 17) != 0, ctrl: flags & (1 << 18) != 0, alt: flags & (1 << 19) != 0, cmd: flags & (1 << 20) != 0 }
}
