//! Keyboard via libxkbcommon (present wherever Wayland is, and in X11 with XKB):
//! keymap from the compositor -> keysym and layout-aware text, dead keys via `xkb_compose`.

use std::ffi::{c_char, c_int, c_void};

use uison_core::{Key, KeyCode, Mods};

use crate::dl::dylib;

type Ctx = c_void;
type Keymap = c_void;
type State = c_void;
type ComposeTable = c_void;
type ComposeState = c_void;

dylib! {
    pub struct Xkb: ["libxkbcommon.so.0"] {
        fn xkb_context_new(flags: c_int) -> *mut Ctx;
        fn xkb_keymap_new_from_string(ctx: *mut Ctx, s: *const c_char, format: c_int, flags: c_int) -> *mut Keymap;
        fn xkb_keymap_unref(k: *mut Keymap);
        fn xkb_keymap_key_repeats(k: *mut Keymap, key: u32) -> c_int;
        fn xkb_keymap_key_get_syms_by_level(k: *mut Keymap, key: u32, layout: u32, level: u32, syms: *mut *const u32) -> c_int;
        fn xkb_state_new(k: *mut Keymap) -> *mut State;
        fn xkb_state_unref(s: *mut State);
        fn xkb_state_update_mask(s: *mut State, dep: u32, lat: u32, lock: u32, dep_l: u32, lat_l: u32, lock_l: u32) -> c_int;
        fn xkb_state_key_get_one_sym(s: *mut State, key: u32) -> u32;
        fn xkb_state_key_get_utf8(s: *mut State, key: u32, buf: *mut c_char, size: usize) -> c_int;
        fn xkb_state_mod_name_is_active(s: *mut State, name: *const c_char, kind: c_int) -> c_int;
        fn xkb_keysym_to_utf32(sym: u32) -> u32;
        fn xkb_compose_table_new_from_locale(ctx: *mut Ctx, locale: *const c_char, flags: c_int) -> *mut ComposeTable;
        fn xkb_compose_state_new(t: *mut ComposeTable, flags: c_int) -> *mut ComposeState;
        fn xkb_compose_state_feed(s: *mut ComposeState, sym: u32) -> c_int;
        fn xkb_compose_state_get_status(s: *mut ComposeState) -> c_int;
        fn xkb_compose_state_get_utf8(s: *mut ComposeState, buf: *mut c_char, size: usize) -> c_int;
        fn xkb_compose_state_reset(s: *mut ComposeState);
    }
}

dylib! {
    pub struct XkbX11: ["libxkbcommon-x11.so.0"] {
        fn xkb_x11_setup_xkb_extension(conn: *mut c_void, major: u16, minor: u16, flags: c_int,
            major_out: *mut u16, minor_out: *mut u16, event_out: *mut u8, error_out: *mut u8) -> c_int;
        fn xkb_x11_get_core_keyboard_device_id(conn: *mut c_void) -> i32;
        fn xkb_x11_keymap_new_from_device(ctx: *mut Ctx, conn: *mut c_void, device: i32, flags: c_int) -> *mut Keymap;
    }
}

dylib! {
    struct XcbXkb: ["libxcb-xkb.so.1"] {
        fn xcb_xkb_select_events(conn: *mut c_void, device: u16, affect_which: u16, clear: u16, select_all: u16,
            affect_map: u16, map: u16, details: *const c_void) -> u32;
    }
}

const STATE_MODS_EFFECTIVE: c_int = 8;
const COMPOSE_ACCEPTED: c_int = 1;
const COMPOSE_COMPOSING: c_int = 1;
const COMPOSE_COMPOSED: c_int = 2;
const COMPOSE_CANCELLED: c_int = 3;

pub struct Keyboard {
    lib: Xkb,
    ctx: *mut Ctx,
    keymap: *mut Keymap,
    state: *mut State,
    compose: *mut ComposeState,
}

impl Keyboard {
    pub fn new() -> Result<Keyboard, String> {
        let lib = Xkb::load()?;
        let ctx = unsafe { lib.xkb_context_new(0) };
        if ctx.is_null() {
            return Err("xkb_context_new failed".into());
        }
        // Dead keys and Compose follow the locale (like other apps).
        let locale = ["LC_ALL", "LC_CTYPE", "LANG"].iter().find_map(|v| std::env::var(v).ok().filter(|s| !s.is_empty())).unwrap_or_else(|| "C".into());
        let loc = format!("{locale}\0");
        let compose = unsafe {
            let table = lib.xkb_compose_table_new_from_locale(ctx, loc.as_ptr().cast(), 0);
            if table.is_null() { std::ptr::null_mut() } else { lib.xkb_compose_state_new(table, 0) }
        };
        Ok(Keyboard { lib, ctx, keymap: std::ptr::null_mut(), state: std::ptr::null_mut(), compose })
    }

    /// Keymap in XKB text format (from the compositor, in shared memory, NUL-terminated).
    pub fn set_keymap(&mut self, text: &[u8]) {
        let end = text.iter().position(|&b| b == 0).unwrap_or(text.len());
        let mut s = text[..end].to_vec();
        s.push(0);
        unsafe {
            let km = self.lib.xkb_keymap_new_from_string(self.ctx, s.as_ptr().cast(), 1, 0);
            if km.is_null() {
                log!("uison: bad keymap");
                return;
            }
            if !self.state.is_null() {
                self.lib.xkb_state_unref(self.state);
                self.lib.xkb_keymap_unref(self.keymap);
            }
            self.keymap = km;
            self.state = self.lib.xkb_state_new(km);
        }
    }

    /// X11: keymap from the server (XKB extension). The first call sets up the extension and
    /// subscribes to layout changes; a repeated one (on an XKB event) rereads the layout.
    /// Returns the first XKB event number (0 on failure).
    pub fn set_keymap_x11(&mut self, conn: *mut c_void) -> u8 {
        let Ok(x) = XkbX11::load() else { return 0 };
        unsafe {
            let (mut major, mut minor, mut ev, mut err) = (0u16, 0u16, 0u8, 0u8);
            if x.xkb_x11_setup_xkb_extension(conn, 1, 0, 0, &mut major, &mut minor, &mut ev, &mut err) == 0 {
                return 0;
            }
            if self.keymap.is_null()
                && let Ok(xx) = XcbXkb::load()
            {
                // With XKB enabled the server sends its own events instead of MappingNotify:
                // NewKeyboardNotify (1) and MapNotify (2). Device is the core keyboard (0x100).
                let which = 1 | 2;
                xx.xcb_xkb_select_events(conn, 0x100, which, 0, which, 0xff, 0xff, std::ptr::null());
            }
            let dev = x.xkb_x11_get_core_keyboard_device_id(conn);
            if dev < 0 {
                return 0;
            }
            let km = x.xkb_x11_keymap_new_from_device(self.ctx, conn, dev, 0);
            if km.is_null() {
                return 0;
            }
            if !self.state.is_null() {
                self.lib.xkb_state_unref(self.state);
                self.lib.xkb_keymap_unref(self.keymap);
            }
            self.keymap = km;
            self.state = self.lib.xkb_state_new(km);
            ev
        }
    }

    /// X11: modifiers and layout group come from the key event's `state` field
    /// (Shift...Mod5 bits match XKB modifier indices, the group is bits 13-14).
    pub fn set_core_state(&mut self, state: u16) {
        self.set_mods((state & 0xff) as u32, 0, 0, ((state >> 13) & 3) as u32);
    }

    pub fn ready(&self) -> bool {
        !self.state.is_null()
    }

    pub fn set_mods(&mut self, depressed: u32, latched: u32, locked: u32, group: u32) {
        if self.ready() {
            unsafe { self.lib.xkb_state_update_mask(self.state, depressed, latched, locked, 0, 0, group) };
        }
    }

    fn active(&self, name: &str) -> bool {
        let c = format!("{name}\0");
        unsafe { self.lib.xkb_state_mod_name_is_active(self.state, c.as_ptr().cast(), STATE_MODS_EFFECTIVE) > 0 }
    }

    pub fn mods(&self) -> Mods {
        if !self.ready() {
            return Mods::default();
        }
        // The main shortcut modifier is Ctrl (`cmd`), as on Windows.
        Mods { shift: self.active("Shift"), ctrl: false, alt: self.active("Mod1"), cmd: self.active("Control") }
    }

    /// Whether the key repeats when held (letters and arrows do, modifiers don't).
    /// `code` is an XKB code (evdev + 8).
    pub fn repeats(&self, code: u32) -> bool {
        self.ready() && unsafe { self.lib.xkb_keymap_key_repeats(self.keymap, code) } != 0
    }

    /// Key press -> app event (`None` for a modifier, a dead key, etc.).
    pub fn key(&mut self, code: u32) -> Option<Key> {
        if !self.ready() {
            return None;
        }
        let sym = unsafe { self.lib.xkb_state_key_get_one_sym(self.state, code) };
        let mods = self.mods();
        let special = match sym {
            0xff08 => Some(KeyCode::Backspace),
            0xff09 | 0xfe20 => Some(KeyCode::Tab),
            0xff0d | 0xff8d => Some(KeyCode::Enter),
            0xff1b => Some(KeyCode::Escape),
            0xffff | 0xff9f => Some(KeyCode::Delete),
            0xff50 | 0xff95 => Some(KeyCode::Home),
            0xff51 | 0xff96 => Some(KeyCode::Left),
            0xff52 | 0xff97 => Some(KeyCode::Up),
            0xff53 | 0xff98 => Some(KeyCode::Right),
            0xff54 | 0xff99 => Some(KeyCode::Down),
            0xff55 | 0xff9a => Some(KeyCode::PageUp),
            0xff56 | 0xff9b => Some(KeyCode::PageDown),
            0xff57 | 0xff9c => Some(KeyCode::End),
            0xff67 => Some(KeyCode::Menu),
            0xffc7 => Some(KeyCode::F10),
            _ => None,
        };
        if let Some(code) = special {
            self.reset_compose();
            return Some(Key { code, text: None, mods });
        }
        // Ctrl+symbol uses the first layout and no Shift (Ctrl+N works with a Cyrillic layout too).
        if mods.cmd && !mods.alt {
            let mut syms: *const u32 = std::ptr::null();
            let n = unsafe { self.lib.xkb_keymap_key_get_syms_by_level(self.keymap, code, 0, 0, &mut syms) };
            let ch = (n > 0).then(|| char::from_u32(unsafe { self.lib.xkb_keysym_to_utf32(*syms) })).flatten()?;
            if ch.is_control() || ch == '\0' {
                return None;
            }
            let mut buf = [0u8; 4];
            return Some(Key { code: KeyCode::Char, text: Some(ch.to_ascii_lowercase().encode_utf8(&mut buf).into()), mods });
        }
        let text = match self.compose_feed(sym) {
            Compose::Text(t) => t,
            Compose::Pending => return None,
            Compose::None => {
                let mut buf = [0u8; 64];
                let n = unsafe { self.lib.xkb_state_key_get_utf8(self.state, code, buf.as_mut_ptr().cast(), buf.len()) };
                String::from_utf8_lossy(&buf[..(n.max(0) as usize).min(buf.len() - 1)]).into_owned()
            }
        };
        let text: String = text.chars().filter(|c| !c.is_control()).collect();
        if text.is_empty() {
            return None;
        }
        Some(Key { code: KeyCode::Char, text: Some(text.into()), mods })
    }

    fn compose_feed(&mut self, sym: u32) -> Compose {
        if self.compose.is_null() {
            return Compose::None;
        }
        unsafe {
            if self.lib.xkb_compose_state_feed(self.compose, sym) != COMPOSE_ACCEPTED {
                return Compose::None;
            }
            match self.lib.xkb_compose_state_get_status(self.compose) {
                COMPOSE_COMPOSING => Compose::Pending,
                COMPOSE_COMPOSED => {
                    let mut buf = [0u8; 64];
                    let n = self.lib.xkb_compose_state_get_utf8(self.compose, buf.as_mut_ptr().cast(), buf.len());
                    self.lib.xkb_compose_state_reset(self.compose);
                    Compose::Text(String::from_utf8_lossy(&buf[..(n.max(0) as usize).min(buf.len() - 1)]).into_owned())
                }
                COMPOSE_CANCELLED => {
                    self.lib.xkb_compose_state_reset(self.compose);
                    Compose::Pending
                }
                _ => Compose::None,
            }
        }
    }

    fn reset_compose(&mut self) {
        if !self.compose.is_null() {
            unsafe { self.lib.xkb_compose_state_reset(self.compose) };
        }
    }
}

enum Compose {
    None,
    Pending,
    Text(String),
}
