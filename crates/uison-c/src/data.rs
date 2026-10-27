//! Data by path: `"form.email"`, `"todos.3.done"`.

use std::ffi::{c_char, c_int};

use fw::json::Value;
use fw::ui::{Data, Json};

use crate::{Arena, clear_error, guard, set_error, text};

/// Data of a `Ui` or a handler. Result strings live in this object's arena.
pub struct uison_data {
    pub(crate) d: *mut Data,
    pub(crate) arena: Arena,
}

impl uison_data {
    pub(crate) fn new(d: *mut Data) -> uison_data {
        uison_data { d, arena: Arena::default() }
    }
}

pub const UISON_NONE: c_int = -1;
pub const UISON_NULL: c_int = 0;
pub const UISON_BOOL: c_int = 1;
pub const UISON_NUMBER: c_int = 2;
pub const UISON_STRING: c_int = 3;
pub const UISON_ARRAY: c_int = 4;
pub const UISON_OBJECT: c_int = 5;

unsafe fn data<'a>(d: *mut uison_data) -> &'a mut uison_data {
    assert!(!d.is_null(), "uison_data is NULL");
    unsafe { &mut *d }
}

unsafe fn get<'a>(d: *mut uison_data, path: *const c_char) -> Option<&'a Json> {
    unsafe { (*data(d).d).get(&text(path)) }
}

fn parse(json: &str) -> Option<Json> {
    match fw::json::parse(json) {
        Ok(v) => Some(v.into_owned()),
        Err(e) => {
            set_error(format!("JSON: {e}"));
            None
        }
    }
}

/// Type of the value at a path: `UISON_NULL`…`UISON_OBJECT`, `UISON_NONE` (-1) — no such value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_data_type(d: *mut uison_data, path: *const c_char) -> c_int {
    guard(|| match unsafe { get(d, path) } {
        None => UISON_NONE,
        Some(Value::Null) => UISON_NULL,
        Some(Value::Bool(_)) => UISON_BOOL,
        Some(Value::Number(_)) => UISON_NUMBER,
        Some(Value::String(_)) => UISON_STRING,
        Some(Value::Array(_)) => UISON_ARRAY,
        Some(Value::Object(_)) => UISON_OBJECT,
    })
}

/// String at a path (not a string — empty; numbers and `bool` — their JSON form).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_data_str(d: *mut uison_data, path: *const c_char) -> *const c_char {
    guard(|| {
        let s = match unsafe { get(d, path) } {
            Some(Value::String(s)) => s.to_string(),
            Some(v @ (Value::Number(_) | Value::Bool(_))) => v.to_json(),
            _ => String::new(),
        };
        unsafe { data(d) }.arena.keep(s)
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_data_f64(d: *mut uison_data, path: *const c_char) -> f64 {
    guard(|| unsafe { get(d, path) }.and_then(Json::as_f64).unwrap_or(0.0))
}

/// Truthiness — as in `@path` bindings: `false`, `null`, `0`, `""`, `[]` are false.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_data_bool(d: *mut uison_data, path: *const c_char) -> bool {
    guard(|| unsafe { get(d, path) }.is_some_and(Json::truthy))
}

/// Array length or number of object fields (otherwise 0).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_data_len(d: *mut uison_data, path: *const c_char) -> usize {
    guard(|| match unsafe { get(d, path) } {
        Some(Value::Array(a)) => a.len(),
        Some(Value::Object(o)) => o.len(),
        _ => 0,
    })
}

/// Name of the `i`-th object field (in data order); none — `NULL`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_data_key(d: *mut uison_data, path: *const c_char, i: usize) -> *const c_char {
    guard(|| match unsafe { get(d, path) } {
        Some(Value::Object(o)) if i < o.len() => {
            let k = o[i].0.to_string();
            unsafe { data(d) }.arena.keep(k)
        }
        _ => std::ptr::null(),
    })
}

/// Subtree as JSON; none — `NULL`. Path `""` — all data.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_data_json(d: *mut uison_data, path: *const c_char) -> *const c_char {
    guard(|| match unsafe { get(d, path) }.map(Json::to_json) {
        Some(s) => unsafe { data(d) }.arena.keep(s),
        None => std::ptr::null(),
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_data_set_str(d: *mut uison_data, path: *const c_char, v: *const c_char) {
    guard(|| unsafe { (*data(d).d).set(&text(path), text(v).into_owned()) })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_data_set_f64(d: *mut uison_data, path: *const c_char, v: f64) {
    guard(|| unsafe { (*data(d).d).set(&text(path), v) })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_data_set_bool(d: *mut uison_data, path: *const c_char, v: bool) {
    guard(|| unsafe { (*data(d).d).set(&text(path), v) })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_data_set_null(d: *mut uison_data, path: *const c_char) {
    guard(|| unsafe { (*data(d).d).set(&text(path), Json::Null) })
}

/// Set a value from JSON (object, array, anything); `false` — parse error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_data_set_json(d: *mut uison_data, path: *const c_char, json: *const c_char) -> bool {
    guard(|| {
        clear_error();
        let Some(v) = parse(&unsafe { text(json) }) else { return false };
        unsafe { (*data(d).d).set(&text(path), v) };
        true
    })
}

/// Append to an array (no array — one is created).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_data_push_json(d: *mut uison_data, path: *const c_char, json: *const c_char) -> bool {
    guard(|| {
        clear_error();
        let Some(v) = parse(&unsafe { text(json) }) else { return false };
        unsafe { (*data(d).d).push(&text(path), v) };
        true
    })
}

/// Remove an array element (`"todos.3"`) or an object field (`"form.email"`). `false` — nothing to remove.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_data_remove(d: *mut uison_data, path: *const c_char) -> bool {
    guard(|| {
        let path = unsafe { text(path) };
        let dd = unsafe { &mut *data(d).d };
        let (parent, last) = path.rsplit_once('.').unwrap_or(("", &path));
        match dd.get(parent) {
            Some(Value::Array(_)) => last.parse::<usize>().ok().and_then(|i| dd.remove(parent, i)).is_some(),
            Some(Value::Object(_)) => dd.get_mut(parent).and_then(|o| o.remove(last)).is_some(),
            _ => false,
        }
    })
}
