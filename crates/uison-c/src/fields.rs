//! C structs ↔ data: a field table (`uison_field[]`, macros `UISON_INT`… in `uison_kit.h`)
//! describes the struct layout — JSON name, type, offset, size. The width of integers and
//! floats comes from the field size: `UISON_INT` works for `short`, `int`, `long long`,
//! `UISON_DOUBLE` — for `float` and `double`.

use std::ffi::{CStr, c_char, c_int};

use fw::json::Value;
use fw::ui::Json;

use crate::data::uison_data;
use crate::{Arena, clear_error, guard, set_error, text};

pub const UISON_FIELD_INT: c_int = 1;
pub const UISON_FIELD_DOUBLE: c_int = 2;
pub const UISON_FIELD_BOOL: c_int = 3;
pub const UISON_FIELD_CHARS: c_int = 4;
pub const UISON_FIELD_STR: c_int = 5;
pub const UISON_FIELD_STRUCT: c_int = 6;

#[repr(C)]
pub struct uison_field {
    name: *const c_char,
    kind: c_int,
    offset: usize,
    size: usize,
    sub: *const uison_field,
}

/// Table fields up to `UISON_END` (name `NULL`).
unsafe fn fields<'a>(f: *const uison_field) -> impl Iterator<Item = &'a uison_field> {
    let mut p = f;
    std::iter::from_fn(move || {
        if p.is_null() {
            return None;
        }
        let cur = unsafe { &*p };
        if cur.name.is_null() {
            return None;
        }
        p = unsafe { p.add(1) };
        Some(cur)
    })
}

fn name(f: &uison_field) -> String {
    unsafe { CStr::from_ptr(f.name) }.to_string_lossy().into_owned()
}

/// Data value → struct field. Whatever is missing in the data is left untouched.
unsafe fn read(v: &Json, base: *mut u8, table: *const uison_field, arena: &mut Arena) {
    for f in unsafe { fields(table) } {
        let Some(x) = v.get(&name(f)) else { continue };
        let p = unsafe { base.add(f.offset) };
        match f.kind {
            UISON_FIELD_INT => {
                let Some(n) = x.as_f64() else { continue };
                let n = n as i64;
                unsafe {
                    match f.size {
                        1 => p.cast::<i8>().write_unaligned(n as i8),
                        2 => p.cast::<i16>().write_unaligned(n as i16),
                        4 => p.cast::<i32>().write_unaligned(n as i32),
                        _ => p.cast::<i64>().write_unaligned(n),
                    }
                }
            }
            UISON_FIELD_DOUBLE => {
                let Some(n) = x.as_f64() else { continue };
                unsafe {
                    if f.size == 4 { p.cast::<f32>().write_unaligned(n as f32) } else { p.cast::<f64>().write_unaligned(n) }
                }
            }
            UISON_FIELD_BOOL => unsafe { p.write(x.truthy() as u8) },
            UISON_FIELD_CHARS | UISON_FIELD_STR => {
                let s = match x {
                    Value::String(s) => s.to_string(),
                    Value::Null => String::new(),
                    v => v.to_json(),
                };
                if f.kind == UISON_FIELD_STR {
                    unsafe { p.cast::<*const c_char>().write_unaligned(arena.keep(s)) };
                } else if f.size > 0 {
                    // char[N]: at most N-1 bytes, cut at a UTF-8 character boundary.
                    let mut n = s.len().min(f.size - 1);
                    while !s.is_char_boundary(n) {
                        n -= 1;
                    }
                    unsafe {
                        std::ptr::copy_nonoverlapping(s.as_ptr(), p, n);
                        p.add(n).write(0);
                    }
                }
            }
            UISON_FIELD_STRUCT => {
                if matches!(x, Value::Object(_)) {
                    unsafe { read(x, p, f.sub, arena) };
                }
            }
            _ => {}
        }
    }
}

/// Struct → JSON object with the table's fields.
unsafe fn write(base: *const u8, table: *const uison_field) -> Json {
    let mut o = Json::object();
    for f in unsafe { fields(table) } {
        let p = unsafe { base.add(f.offset) };
        let v = match f.kind {
            UISON_FIELD_INT => Value::Number(unsafe {
                match f.size {
                    1 => p.cast::<i8>().read_unaligned() as f64,
                    2 => p.cast::<i16>().read_unaligned() as f64,
                    4 => p.cast::<i32>().read_unaligned() as f64,
                    _ => p.cast::<i64>().read_unaligned() as f64,
                }
            }),
            UISON_FIELD_DOUBLE => Value::Number(unsafe {
                if f.size == 4 { p.cast::<f32>().read_unaligned() as f64 } else { p.cast::<f64>().read_unaligned() }
            }),
            UISON_FIELD_BOOL => Value::Bool(unsafe { p.read() } != 0),
            UISON_FIELD_CHARS => {
                let bytes = unsafe { std::slice::from_raw_parts(p, f.size) };
                let n = bytes.iter().position(|&b| b == 0).unwrap_or(f.size);
                Value::String(String::from_utf8_lossy(&bytes[..n]).into_owned().into())
            }
            UISON_FIELD_STR => {
                let s = unsafe { p.cast::<*const c_char>().read_unaligned() };
                if s.is_null() { Value::Null } else { Value::String(unsafe { text(s) }.into_owned().into()) }
            }
            UISON_FIELD_STRUCT => unsafe { write(p, f.sub) },
            _ => continue,
        };
        o.set(&name(f), v);
    }
    o
}

unsafe fn data<'a>(d: *mut uison_data) -> &'a mut uison_data {
    assert!(!d.is_null(), "uison_data is NULL");
    unsafe { &mut *d }
}

/// Read the object at a path into a struct: fields missing in the data stay as they were
/// (fill in defaults, then read). `false` — the path is not an object.
/// `UISON_STR` strings live like strings from `uison_data_str`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_data_get_struct(
    d: *mut uison_data,
    path: *const c_char,
    out: *mut u8,
    f: *const uison_field,
) -> bool {
    guard(|| {
        clear_error();
        let dd = unsafe { data(d) };
        let path = unsafe { text(path) };
        match unsafe { (*dd.d).get(&path) } {
            Some(v @ Value::Object(_)) => {
                unsafe { read(v, out, f, &mut dd.arena) };
                true
            }
            _ => {
                set_error(format!("uison_data_get_struct: '{path}' is not an object"));
                false
            }
        }
    })
}

/// Write a struct as an object at a path (the table's fields).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_data_set_struct(
    d: *mut uison_data,
    path: *const c_char,
    v: *const u8,
    f: *const uison_field,
) -> bool {
    guard(|| {
        let o = unsafe { write(v, f) };
        unsafe { (*data(d).d).set(&text(path), o) };
        true
    })
}

/// Append a struct to the array at a path (no array — one is created).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_data_push_struct(
    d: *mut uison_data,
    path: *const c_char,
    v: *const u8,
    f: *const uison_field,
) -> bool {
    guard(|| {
        let o = unsafe { write(v, f) };
        unsafe { (*data(d).d).push(&text(path), o) };
        true
    })
}

/// An array of structs (`n` of them, `stride` bytes apart) — as an array of objects at a path.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_data_set_array(
    d: *mut uison_data,
    path: *const c_char,
    items: *const u8,
    n: usize,
    stride: usize,
    f: *const uison_field,
) -> bool {
    guard(|| {
        let a: Vec<Json> = (0..n).map(|i| unsafe { write(items.add(i * stride), f) }).collect();
        unsafe { (*data(d).d).set(&text(path), Value::Array(a)) };
        true
    })
}

/// Number as locale-independent JSON (as `Writer` writes it): shortest form, `NaN`/infinity — `null`.
/// `out` must be at least 32 bytes; returns the length.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_json_number(v: f64, out: *mut c_char) -> usize {
    let s = Value::Number(v).to_json();
    let n = s.len().min(31);
    unsafe {
        std::ptr::copy_nonoverlapping(s.as_ptr(), out.cast::<u8>(), n);
        out.add(n).write(0);
    }
    n
}
