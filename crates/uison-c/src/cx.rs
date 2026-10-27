//! Handler context (`Cx`): action, event, node key, data, requests to the runtime.
//! Lives only inside the handler — don't keep the pointer.

use std::ffi::{c_char, c_void};

use fw::ui::{Cx, Json};

use crate::data::uison_data;
use crate::{Arena, clear_error, guard, set_error, text};

pub struct uison_cx {
    /// `&mut Cx` for the duration of the handler call (lifetime erased — the pointer never escapes).
    cx: *mut Cx<'static>,
    data: uison_data,
    arena: Arena,
}

impl uison_cx {
    /// Call a C handler with this `cx`.
    pub(crate) fn call(cx: &mut Cx, f: unsafe extern "C" fn(*mut uison_cx, *mut c_void), user: *mut c_void) {
        let d: *mut fw::ui::Data = &mut *cx.data;
        let mut c = uison_cx { cx: (cx as *mut Cx).cast(), data: uison_data::new(d), arena: Arena::default() };
        unsafe { f(&mut c, user) };
    }
}

unsafe fn cx<'a>(c: *mut uison_cx) -> &'a mut uison_cx {
    assert!(!c.is_null(), "uison_cx is NULL");
    unsafe { &mut *c }
}

unsafe fn inner<'a>(c: *mut uison_cx) -> &'a mut Cx<'static> {
    unsafe { &mut *cx(c).cx }
}

/// Action name (`on: { click: "name" }`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_cx_action(c: *mut uison_cx) -> *const c_char {
    guard(|| {
        let s = unsafe { inner(c) }.action.to_string();
        unsafe { cx(c) }.arena.keep(s)
    })
}

/// Event: `click`, `change`, `submit`, `menu`, `shortcut`, `timer`, `code`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_cx_event(c: *mut uison_cx) -> *const c_char {
    guard(|| {
        let s = unsafe { inner(c) }.event.to_string();
        unsafe { cx(c) }.arena.keep(s)
    })
}

/// `key` of the node the event happened on (`"del-3"`, `"inboxList/12"`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_cx_key(c: *mut uison_cx) -> *const c_char {
    guard(|| {
        let s = unsafe { inner(c) }.key.to_string();
        unsafe { cx(c) }.arena.keep(s)
    })
}

/// App data (result strings live until the handler returns).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_cx_data(c: *mut uison_cx) -> *mut uison_data {
    unsafe { &mut cx(c).data }
}

/// Focus a node (text field) by `key`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_cx_focus(c: *mut uison_cx, key: *const c_char) {
    guard(|| unsafe { inner(c).focus(&text(key)) })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_cx_blur(c: *mut uison_cx) {
    guard(|| unsafe { inner(c).blur() })
}

/// Theme: `light`, `dark`, `system` or a custom one.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_cx_set_theme(c: *mut uison_cx, name: *const c_char) {
    guard(|| unsafe { inner(c).set_theme(&text(name)) })
}

/// Override a theme token (`"primary"`, `"\"#FF3B30\""`); the value is JSON.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_cx_set_token(c: *mut uison_cx, name: *const c_char, json: *const c_char) -> bool {
    guard(|| {
        clear_error();
        match fw::json::parse(&unsafe { text(json) }) {
            Ok(v) => {
                unsafe { inner(c).set_token(&text(name), v.into_owned()) };
                true
            }
            Err(e) => {
                set_error(format!("JSON: {e}"));
                false
            }
        }
    })
}

/// Call an action after `ms` milliseconds.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_cx_timer(c: *mut uison_cx, ms: u64, action: *const c_char) {
    guard(|| unsafe { inner(c).timer(ms, &text(action)) })
}

/// Put text on the system clipboard.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_cx_copy(c: *mut uison_cx, s: *const c_char) {
    guard(|| unsafe { inner(c).copy(&text(s)) })
}

/// Open a link in the browser.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_cx_open_url(c: *mut uison_cx, url: *const c_char) {
    guard(|| unsafe { inner(c).open_url(&text(url)) })
}

/// Property of a screen tree node by `key` as JSON; no node or property — `NULL`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_cx_node_get(c: *mut uison_cx, key: *const c_char, prop: *const c_char) -> *const c_char {
    guard(|| {
        let v = unsafe { inner(c) }.tree.find(&unsafe { text(key) }).and_then(|n| n.get(&unsafe { text(prop) }).map(Json::to_json));
        match v {
            Some(s) => unsafe { cx(c) }.arena.keep(s),
            None => std::ptr::null(),
        }
    })
}

/// Set a node property (`json` is the value); `false` — no node or a JSON error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_cx_node_set(c: *mut uison_cx, key: *const c_char, prop: *const c_char, json: *const c_char) -> bool {
    guard(|| {
        clear_error();
        let v = match fw::json::parse(&unsafe { text(json) }) {
            Ok(v) => v.into_owned(),
            Err(e) => {
                set_error(format!("JSON: {e}"));
                return false;
            }
        };
        let key = unsafe { text(key) };
        match unsafe { inner(c) }.tree.find(&key) {
            Some(mut n) => {
                n.set(&unsafe { text(prop) }, v);
                true
            }
            None => {
                set_error(format!("no node with key '{key}'"));
                false
            }
        }
    })
}
