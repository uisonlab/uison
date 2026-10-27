//! App requests to iOS: clipboard, links.

use std::cell::Cell;
use std::ffi::{CStr, c_char};

use uison_core::{Font, Host, TextMeasure, TextMetrics};

use uison_apple::ffi::*;
use uison_apple::{Text, class, sel};

/// The platform for the duration of one event; the backend performs paste after the event.
pub struct IosHost<'a> {
    pub text: &'a Text,
    pub paste: &'a Cell<bool>,
}

impl TextMeasure for IosHost<'_> {
    fn measure(&self, text: &str, font: &Font) -> TextMetrics {
        self.text.measure(text, font)
    }
}

impl Host for IosHost<'_> {
    fn set_clipboard(&self, text: &str) {
        unsafe {
            let pb: Id = msg(class!("UIPasteboard"), sel!("generalPasteboard"), ());
            let s = nsstring(text);
            msg::<(), _>(pb, sel!("setString:"), (s,));
            release(s);
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
            let app: Id = msg(class!("UIApplication"), sel!("sharedApplication"), ());
            let opts: Id = msg(class!("NSDictionary"), sel!("dictionary"), ());
            msg::<(), _>(app, sel!("openURL:options:completionHandler:"), (u, opts, NIL));
            true
        }
    }
}

/// Text from the clipboard. Since iOS 16 the system asks for permission to paste.
pub fn clipboard() -> Option<String> {
    unsafe {
        let pb: Id = msg(class!("UIPasteboard"), sel!("generalPasteboard"), ());
        let s: Id = msg(pb, sel!("string"), ());
        if s.is_null() {
            return None;
        }
        let c: *const c_char = msg(s, sel!("UTF8String"), ());
        (!c.is_null()).then(|| CStr::from_ptr(c).to_string_lossy().into_owned())
    }
}
