//! App requests to Android: clipboard, links, the file chooser — via `UisonView` methods.

use std::cell::Cell;

use uison_core::{Font, Host, TextMeasure, TextMetrics};

use crate::env;
use crate::java::j;
use crate::jni::*;
use crate::text::AndroidText;

/// The platform for the duration of one event; the backend performs the paste after the event.
pub struct AndroidHost<'a> {
    pub text: &'a AndroidText,
    /// Global reference to UisonView.
    pub view: jobject,
    pub paste: &'a Cell<bool>,
}

impl TextMeasure for AndroidHost<'_> {
    fn measure(&self, text: &str, font: &Font) -> TextMetrics {
        self.text.measure(text, font)
    }
}

impl Host for AndroidHost<'_> {
    fn set_clipboard(&self, text: &str) {
        let e = env();
        unsafe {
            let s = e.new_string(text);
            e.call_void(self.view, j().view_copy, &[l(s)]);
            e.delete_local(s);
        }
    }

    fn request_paste(&self) {
        self.paste.set(true);
    }

    fn open_url(&self, url: &str) -> bool {
        let e = env();
        unsafe {
            let s = e.new_string(url);
            let ok = e.call_bool(self.view, j().view_open_url, &[l(s)]);
            e.delete_local(s);
            ok
        }
    }

    fn pick_file(&self, action: &str) -> bool {
        let e = env();
        unsafe {
            let s = e.new_string(action);
            let ok = e.call_bool(self.view, j().view_pick_file, &[l(s)]);
            e.delete_local(s);
            ok
        }
    }
}

/// Text from the clipboard.
pub fn clipboard(view: jobject) -> Option<String> {
    let e = env();
    unsafe {
        let s = e.call_object(view, j().view_paste, &[]);
        if s.is_null() {
            return None;
        }
        let t = e.string(s);
        e.delete_local(s);
        Some(t)
    }
}
