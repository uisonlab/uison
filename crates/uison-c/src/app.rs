//! Low level: your own app — the `uison_app` function table, a frame of primitives as a JSON string.
//! The backend does the window, painting, diff and hit-test.

use std::ffi::{CStr, c_char, c_int, c_void};
use std::time::Duration;

use fw::{App, Event, Font, Host};

use crate::{User, guard, uison_drop, set_error, text};

/// Platform access for the app during a call: text metrics, clipboard, links.
pub struct uison_host {
    host: *const (dyn Host + 'static),
}

#[repr(C)]
pub struct uison_app {
    /// `sizeof(uison_app)` — so the table can grow without breaking the ABI.
    pub size: usize,
    pub user: *mut c_void,
    /// First frame (JSON of primitives). The string belongs to the app: it must live until the next call.
    pub frame: Option<unsafe extern "C" fn(user: *mut c_void, host: *mut uison_host) -> *const c_char>,
    /// Event (JSON) and the `id` of the hit area under the cursor (or NULL) → a new frame, or NULL (unchanged).
    pub on_event: Option<
        unsafe extern "C" fn(user: *mut c_void, event: *const c_char, hit: *const c_char, host: *mut uison_host) -> *const c_char,
    >,
    /// In how many microseconds to send a `tick` (animation, blinking); -1 — not needed. May be NULL.
    pub wants_tick_us: Option<unsafe extern "C" fn(user: *mut c_void) -> i64>,
    pub drop: uison_drop,
}

/// Event as JSON: `{"type":"down","x":10,"y":20,"button":"left"}` etc. (the form of `uison_frame::event`).
pub(crate) fn event_json(e: &Event) -> String {
    fw::event::to_json(e)
}

struct CApp {
    app: uison_app,
    _user: User,
}

impl CApp {
    fn call(&self, f: impl FnOnce(&mut uison_host) -> *const c_char, host: &dyn Host) -> Option<String> {
        // The lifetime of `host` is erased: the pointer lives only inside the call.
        let host: *const (dyn Host + '_) = host;
        let mut h = uison_host { host: unsafe { std::mem::transmute::<*const (dyn Host + '_), *const (dyn Host + 'static)>(host) } };
        let p = f(&mut h);
        (!p.is_null()).then(|| unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned())
    }
}

impl App for CApp {
    fn frame(&mut self, host: &dyn Host) -> String {
        let (f, u) = (self.app.frame, self.app.user);
        self.call(|h| f.map_or(std::ptr::null(), |f| unsafe { f(u, h) }), host)
            .unwrap_or_else(|| "{ size: [400, 300], items: [] }".into())
    }

    fn on_event(&mut self, event: &Event, hit: Option<&str>, host: &dyn Host) -> Option<String> {
        let f = self.app.on_event?;
        let ev = std::ffi::CString::new(event_json(event)).ok()?;
        let hit = hit.and_then(|h| std::ffi::CString::new(h).ok());
        let u = self.app.user;
        self.call(|h| unsafe { f(u, ev.as_ptr(), hit.as_ref().map_or(std::ptr::null(), |c| c.as_ptr()), h) }, host)
    }

    fn wants_tick(&self) -> Option<Duration> {
        let f = self.app.wants_tick_us?;
        let us = unsafe { f(self.app.user) };
        (us >= 0).then(|| Duration::from_micros(us as u64))
    }
}

/// Run your own app. `app` is copied (fields beyond `app->size` get defaults).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_run_app(app: *const uison_app, title: *const c_char) -> c_int {
    guard(|| {
        if app.is_null() {
            set_error("uison_run_app: app is NULL");
            return 1;
        }
        // A table from an older header is smaller — the missing fields are empty.
        let mut a = uison_app { size: 0, user: std::ptr::null_mut(), frame: None, on_event: None, wants_tick_us: None, drop: None };
        let n = unsafe { (*app).size }.min(size_of::<uison_app>());
        unsafe { std::ptr::copy_nonoverlapping(app.cast::<u8>(), (&mut a as *mut uison_app).cast::<u8>(), n) };
        let user = User { ptr: a.user, drop: a.drop };
        let title = unsafe { text(title) }.into_owned();
        fw::run(CApp { app: a, _user: user }, &title);
        0
    })
}

unsafe fn host<'a>(h: *mut uison_host) -> &'a dyn Host {
    assert!(!h.is_null(), "uison_host is NULL");
    unsafe { &*(*h).host }
}

/// Metrics of a string in font `family` (NULL — system) of size `size`: width, ascent, descent (points).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_host_measure(
    h: *mut uison_host,
    s: *const c_char,
    size: f64,
    bold: bool,
    family: *const c_char,
    width: *mut f64,
    ascent: *mut f64,
    descent: *mut f64,
) {
    guard(|| {
        let fam = (!family.is_null()).then(|| unsafe { text(family) }.into_owned().into());
        let font = Font { family: fam, size: size as f32, bold, italic: false };
        let m = unsafe { host(h) }.measure(&unsafe { text(s) }, &font);
        unsafe {
            if !width.is_null() {
                *width = m.width as f64;
            }
            if !ascent.is_null() {
                *ascent = m.ascent as f64;
            }
            if !descent.is_null() {
                *descent = m.descent as f64;
            }
        }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_host_copy(h: *mut uison_host, s: *const c_char) {
    guard(|| unsafe { host(h) }.set_clipboard(&unsafe { text(s) }))
}

/// Ask for the clipboard text: it arrives as a `{"type":"paste","text":…}` event.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_host_request_paste(h: *mut uison_host) {
    guard(|| unsafe { host(h) }.request_paste())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_host_open_url(h: *mut uison_host, url: *const c_char) -> bool {
    guard(|| unsafe { host(h) }.open_url(&unsafe { text(url) }))
}

#[cfg(test)]
mod tests {
    use super::event_json;
    use fw::{Event, Key, KeyCode, MouseButton, Point, Size};

    #[test]
    fn events_as_json() {
        assert_eq!(event_json(&Event::MouseDown(Point::new(10.0, 20.5), MouseButton::Left)), r#"{"type":"down","x":10,"y":20.5,"button":"left"}"#);
        // Cyrillic on purpose: non-ASCII text must pass through as UTF-8.
        let mut k = Key::char("ж");
        k.mods.shift = true;
        assert_eq!(
            event_json(&Event::Key(k)),
            r#"{"type":"key","code":"char","text":"ж","shift":true,"ctrl":false,"alt":false,"cmd":false}"#
        );
        assert!(event_json(&Event::Key(Key::new(KeyCode::Enter))).contains(r#""code":"enter","text":null"#));
        assert_eq!(event_json(&Event::Resize(Size::new(800.0, 600.0))), r#"{"type":"resize","w":800,"h":600}"#);
        assert_eq!(event_json(&Event::Paste("a\"b".into())), r#"{"type":"paste","text":"a\"b"}"#);
        assert_eq!(event_json(&Event::Tick), r#"{"type":"tick"}"#);
    }
}
