//! `uison_ui`: screen (JSON) + handlers + data; `uison_run` — the window.

use std::ffi::{c_char, c_int, c_void};

use fw::ui::Ui;

use crate::cx::uison_cx;
use crate::data::uison_data;
use crate::{Arena, User, clear_error, guard, uison_derive, uison_drop, uison_handler, set_error, text};

pub struct uison_ui {
    ui: Ui,
    /// Data before launch (`uison_ui_data`): points at `ui.data` — `uison_ui` is on the heap, the address is stable.
    data: uison_data,
    arena: Arena,
}

/// Where the file is: an absolute path, or relative to the program's folder, then the current one.
fn find(path: &str) -> Option<std::path::PathBuf> {
    let p = std::path::Path::new(path);
    let mut tries = vec![p.to_path_buf()];
    if p.is_relative()
        && let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.to_path_buf()))
    {
        tries.insert(0, dir.join(p));
    }
    tries.into_iter().find(|t| t.is_file())
}

/// File text (see `find`); missing — an error.
/// File as a `Source`: by name, `UISON_RELOAD=folder` finds its copy and reloads it on save.
/// The strings live until the program exits (a screen is loaded once).
fn source(name: String, text: String) -> fw::ui::Source {
    fw::ui::Source { name: Box::leak(name.into_boxed_str()), text: Box::leak(text.into_boxed_str()) }
}

fn read(path: &str) -> Option<String> {
    let src = find(path).and_then(|p| std::fs::read_to_string(p).ok());
    if src.is_none() {
        set_error(format!("{path}: file not found (next to the program or in the current folder)"));
    }
    src
}

unsafe fn ui<'a>(u: *mut uison_ui) -> &'a mut uison_ui {
    assert!(!u.is_null(), "uison_ui is NULL");
    unsafe { &mut *u }
}

fn boxed(ui: Ui) -> *mut uison_ui {
    let mut b = Box::new(uison_ui { ui, data: uison_data::new(std::ptr::null_mut()), arena: Arena::default() });
    b.data.d = &mut b.ui.data;
    Box::into_raw(b)
}

/// Screen from JSON (`{ data, theme, components, root }`); `NULL` — error, message in `uison_last_error()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_ui_new(json: *const c_char) -> *mut uison_ui {
    guard(|| {
        clear_error();
        match Ui::new(&unsafe { text(json) }) {
            Ok(ui) => boxed(ui),
            Err(e) => {
                set_error(e);
                std::ptr::null_mut()
            }
        }
    })
}

/// Screen from a JSON file: an absolute path, or relative to the program's folder, then the current one.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_ui_load(path: *const c_char) -> *mut uison_ui {
    guard(|| {
        clear_error();
        let path = unsafe { text(path) }.into_owned();
        let Some(src) = read(&path) else { return std::ptr::null_mut() };
        match Ui::load(source(path.clone(), src)) {
            Ok(ui) => boxed(ui),
            Err(e) => {
                set_error(format!("{path}: {e}"));
                std::ptr::null_mut()
            }
        }
    })
}

/// Free a `Ui` without running it (handlers get their `drop` called). `NULL` — nothing.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_ui_free(u: *mut uison_ui) {
    guard(|| {
        if !u.is_null() {
            drop(unsafe { Box::from_raw(u) });
        }
    })
}

/// Add components: a JSON object `{ Name: { params, body } }`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_ui_add_components(u: *mut uison_ui, json: *const c_char) -> bool {
    guard(|| {
        clear_error();
        match unsafe { ui(u) }.ui.add_components(&unsafe { text(json) }) {
            Ok(_) => true,
            Err(e) => {
                set_error(e);
                false
            }
        }
    })
}

/// One component: `{ params, body }` under the name `name`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_ui_component(u: *mut uison_ui, name: *const c_char, json: *const c_char) -> bool {
    guard(|| {
        clear_error();
        match unsafe { ui(u) }.ui.component(&unsafe { text(name) }, &unsafe { text(json) }) {
            Ok(_) => true,
            Err(e) => {
                set_error(e);
                false
            }
        }
    })
}

/// Components from a file (path — as for `uison_ui_load`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_ui_load_components(u: *mut uison_ui, path: *const c_char) -> bool {
    guard(|| {
        clear_error();
        let path = unsafe { text(path) }.into_owned();
        let Some(src) = read(&path) else { return false };
        match unsafe { ui(u) }.ui.load_components(source(path.clone(), src)) {
            Ok(_) => true,
            Err(e) => {
                set_error(format!("{path}: {e}"));
                false
            }
        }
    })
}

/// Action handler. `user` is passed to every call; `drop(user)` — when the handler is
/// no longer needed (may be `NULL`). Registering the same action again replaces the previous one.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_ui_on(u: *mut uison_ui, action: *const c_char, f: uison_handler, user: *mut c_void, drop: uison_drop) {
    guard(|| {
        let user = User { ptr: user, drop };
        let Some(f) = f else { return };
        unsafe { ui(u) }.ui.on(&unsafe { text(action) }, move |cx| {
            // The whole `user` goes into the closure (not just a field): `drop` happens with the handler.
            let u = &user;
            uison_cx::call(cx, f, u.ptr)
        });
    })
}

/// Derived data: `f` is called right away and after every data change.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_ui_derive(u: *mut uison_ui, f: uison_derive, user: *mut c_void, drop: uison_drop) {
    guard(|| {
        let user = User { ptr: user, drop };
        let Some(f) = f else { return };
        unsafe { ui(u) }.ui.derive(move |d| {
            let u = &user;
            let mut c = uison_data::new(d);
            unsafe { f(&mut c, u.ptr) }
        });
    })
}

/// Keyboard shortcut → action: `"cmd+n"` (⌘ on Apple, Ctrl elsewhere), `"escape"`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_ui_shortcut(u: *mut uison_ui, combo: *const c_char, action: *const c_char) -> bool {
    guard(|| {
        clear_error();
        let combo = unsafe { text(combo) };
        // `Ui::shortcut` panics on an error — check beforehand with the same parser.
        let ok = combo.split('+').all(|p| !p.trim().is_empty());
        if !ok {
            set_error(format!("bad shortcut '{combo}'"));
            return false;
        }
        let u = unsafe { ui(u) };
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            u.ui.shortcut(&combo, &unsafe { text(action) });
        })) {
            Ok(()) => true,
            Err(_) => {
                set_error(format!("bad shortcut '{combo}'"));
                false
            }
        }
    })
}

/// Theme before launch: `light`, `dark`, `system` (default) or a custom one.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_ui_set_theme(u: *mut uison_ui, name: *const c_char) {
    guard(|| unsafe { ui(u) }.ui.set_theme(&unsafe { text(name) }))
}

/// Call an action from code as if an element had triggered it (handy in headless tests).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_ui_run_action(u: *mut uison_ui, action: *const c_char) {
    guard(|| unsafe { ui(u) }.ui.run_action(&unsafe { text(action) }))
}

/// A JSON message from outside, as `Event::Message`: `{"action":"save","event":"click","key":"ok","pos":[x,y],
/// "clicks":1}` runs the action as if the element had fired it, `{"data":{"path":value}}` sets data.
/// `false` — not a message `Ui` understands.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_ui_apply(u: *mut uison_ui, msg: *const c_char) -> bool {
    guard(|| unsafe { ui(u) }.ui.apply(&unsafe { text(msg) }))
}

/// Data before launch (initial values, checks in tests).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_ui_data(u: *mut uison_ui) -> *mut uison_data {
    unsafe { &mut ui(u).data }
}

/// Image from bytes (PNG, JPEG) — the bytes are copied. Returns the source for
/// `Image { src }` / `Avatar { src }`: `"mem:name"`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_ui_image(u: *mut uison_ui, name: *const c_char, bytes: *const u8, len: usize) -> *const c_char {
    guard(|| {
        let b: Vec<u8> = if bytes.is_null() { Vec::new() } else { unsafe { std::slice::from_raw_parts(bytes, len) }.to_vec() };
        let u = unsafe { ui(u) };
        let src = u.ui.image(&unsafe { text(name) }, b);
        u.arena.keep(src)
    })
}

/// Handle to the app for other threads (`uison_post`); thread-safe, free with `uison_handle_free`.
pub struct uison_handle(fw::ui::Handle);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_ui_handle(u: *mut uison_ui) -> *mut uison_handle {
    guard(|| Box::into_raw(Box::new(uison_handle(unsafe { ui(u) }.ui.handle()))))
}

/// `user` may be passed to another thread: the caller promises that (`uison_post`).
struct SendUser(User);
unsafe impl Send for SendUser {}

/// Run `f(cx, user)` on the UI thread (from any thread); then a new frame.
/// Before the window starts it runs right after launch. `drop(user)` — after it runs.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_post(h: *mut uison_handle, f: uison_handler, user: *mut c_void, drop: uison_drop) {
    guard(|| {
        assert!(!h.is_null(), "uison_handle is NULL");
        let user = SendUser(User { ptr: user, drop });
        let Some(f) = f else { return };
        unsafe { &*h }.0.post(move |cx| {
            let u = &user;
            uison_cx::call(cx, f, u.0.ptr)
        });
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_handle_free(h: *mut uison_handle) {
    guard(|| {
        if !h.is_null() {
            drop(unsafe { Box::from_raw(h) });
        }
    })
}

/// Image from a file (path — as for `uison_ui_load`: next to the program, then the current folder).
/// Returns the source `"mem:name"`, or `NULL` if there's no file.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_ui_image_file(u: *mut uison_ui, name: *const c_char, path: *const c_char) -> *const c_char {
    guard(|| {
        clear_error();
        let path = unsafe { text(path) }.into_owned();
        let Some(bytes) = find(&path).and_then(|p| std::fs::read(p).ok()) else {
            set_error(format!("{path}: file not found (next to the program or in the current folder)"));
            return std::ptr::null();
        };
        let u = unsafe { ui(u) };
        let src = u.ui.image(&unsafe { text(name) }, bytes);
        u.arena.keep(src)
    })
}

/// Open the window and enter the event loop. Takes ownership of `ui` (no need to free it); on desktop
/// systems returns after the window closes (on macOS it exits the process).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_run(u: *mut uison_ui, title: *const c_char) -> c_int {
    guard(|| {
        if u.is_null() {
            set_error("uison_run: ui is NULL");
            return 1;
        }
        let b = unsafe { Box::from_raw(u) };
        let title = unsafe { text(title) }.into_owned();
        fw::run(b.ui, &title);
        0
    })
}

/// Program icon (PNG, copied) for the running window; before `uison_run`. See `uison_core::icon`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn uison_set_icon(png: *const u8, len: usize) {
    if !png.is_null() {
        fw::set_icon(unsafe { std::slice::from_raw_parts(png, len) });
    }
}
