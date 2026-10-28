//! X11 window via libxcb (`dlopen`), for sessions without Wayland (Xfce, MATE, old drivers,
//! remote desktop) and for testing under XWayland (`UISON_LINUX=x11`). The window manager draws
//! the frame; output via MIT-SHM (shared memory, `xcb_shm_attach_fd`), otherwise `xcb_put_image`.
//! Scale comes from `Xft.dpi` in `RESOURCE_MANAGER` (set by GNOME, KDE, Xfce): one for all monitors.

use std::ffi::{c_char, c_int, c_void};
use std::os::fd::{AsRawFd, IntoRawFd};
use std::time::{Duration, Instant};

use uison_core::{App, Cursor, Event, Mods, MouseButton, Point, Rect, Size};

use crate::dbus::{Portal, Setting};
use crate::dl::{self, dylib};
use crate::gfx::Image;
use crate::shell::{Ask, ScriptStep, Shell, xdg_open};
use crate::sys::{Map, memfd, wait_any};
use crate::xkb::Keyboard;

type Conn = c_void;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Cookie {
    seq: u32,
}

#[repr(C)]
struct Screen {
    root: u32,
    default_colormap: u32,
    white_pixel: u32,
    black_pixel: u32,
    current_input_masks: u32,
    width_in_pixels: u16,
    height_in_pixels: u16,
    width_in_millimeters: u16,
    height_in_millimeters: u16,
    min_installed_maps: u16,
    max_installed_maps: u16,
    root_visual: u32,
    backing_stores: u8,
    save_unders: u8,
    root_depth: u8,
    allowed_depths_len: u8,
}

#[repr(C)]
pub struct ScreenIter {
    data: *const Screen,
    rem: c_int,
    index: c_int,
}

#[repr(C)]
struct ExtReply {
    response_type: u8,
    pad: u8,
    sequence: u16,
    length: u32,
    present: u8,
    major_opcode: u8,
    first_event: u8,
    first_error: u8,
}

#[repr(C)]
struct ShmVersion {
    response_type: u8,
    shared_pixmaps: u8,
    sequence: u16,
    length: u32,
    major: u16,
    minor: u16,
}

dylib! {
    struct Xcb: ["libxcb.so.1"] {
        fn xcb_connect(name: *const c_char, screen: *mut c_int) -> *mut Conn;
        fn xcb_connection_has_error(c: *mut Conn) -> c_int;
        fn xcb_get_setup(c: *mut Conn) -> *const c_void;
        fn xcb_setup_roots_iterator(setup: *const c_void) -> ScreenIter;
        fn xcb_screen_next(it: *mut ScreenIter);
        fn xcb_generate_id(c: *mut Conn) -> u32;
        fn xcb_create_window(c: *mut Conn, depth: u8, wid: u32, parent: u32, x: i16, y: i16, w: u16, h: u16,
            border: u16, class: u16, visual: u32, mask: u32, values: *const u32) -> Cookie;
        fn xcb_map_window(c: *mut Conn, w: u32) -> Cookie;
        fn xcb_change_property(c: *mut Conn, mode: u8, w: u32, prop: u32, ty: u32, format: u8, len: u32, data: *const c_void) -> Cookie;
        fn xcb_change_window_attributes(c: *mut Conn, w: u32, mask: u32, values: *const u32) -> Cookie;
        fn xcb_intern_atom(c: *mut Conn, only_if_exists: u8, len: u16, name: *const c_char) -> Cookie;
        fn xcb_intern_atom_reply(c: *mut Conn, cookie: Cookie, e: *mut *mut c_void) -> *mut u8;
        fn xcb_get_property(c: *mut Conn, delete: u8, w: u32, prop: u32, ty: u32, off: u32, len: u32) -> Cookie;
        fn xcb_get_property_reply(c: *mut Conn, cookie: Cookie, e: *mut *mut c_void) -> *mut c_void;
        fn xcb_get_property_value(r: *const c_void) -> *const u8;
        fn xcb_get_property_value_length(r: *const c_void) -> c_int;
        fn xcb_create_gc(c: *mut Conn, gc: u32, drawable: u32, mask: u32, values: *const u32) -> Cookie;
        fn xcb_put_image(c: *mut Conn, format: u8, drawable: u32, gc: u32, w: u16, h: u16, x: i16, y: i16,
            left_pad: u8, depth: u8, len: u32, data: *const u8) -> Cookie;
        fn xcb_flush(c: *mut Conn) -> c_int;
        fn xcb_poll_for_event(c: *mut Conn) -> *mut u8;
        fn xcb_get_file_descriptor(c: *mut Conn) -> c_int;
        fn xcb_get_maximum_request_length(c: *mut Conn) -> u32;
        fn xcb_get_extension_data(c: *mut Conn, ext: *mut c_void) -> *const ExtReply;
        fn xcb_open_font(c: *mut Conn, fid: u32, len: u16, name: *const c_char) -> Cookie;
        fn xcb_create_glyph_cursor(c: *mut Conn, cid: u32, src: u32, mask: u32, src_char: u16, mask_char: u16,
            fr: u16, fg: u16, fb: u16, br: u16, bg: u16, bb: u16) -> Cookie;
        fn xcb_set_selection_owner(c: *mut Conn, owner: u32, selection: u32, time: u32) -> Cookie;
        fn xcb_convert_selection(c: *mut Conn, requestor: u32, selection: u32, target: u32, prop: u32, time: u32) -> Cookie;
        fn xcb_send_event(c: *mut Conn, propagate: u8, dest: u32, mask: u32, event: *const c_char) -> Cookie;
    }
}

dylib! {
    struct XcbShm: ["libxcb-shm.so.0"] {
        fn xcb_shm_query_version(c: *mut Conn) -> Cookie;
        fn xcb_shm_query_version_reply(c: *mut Conn, cookie: Cookie, e: *mut *mut c_void) -> *mut ShmVersion;
        fn xcb_shm_attach_fd(c: *mut Conn, seg: u32, fd: c_int, read_only: u8) -> Cookie;
        fn xcb_shm_detach(c: *mut Conn, seg: u32) -> Cookie;
        fn xcb_shm_put_image(c: *mut Conn, drawable: u32, gc: u32, total_w: u16, total_h: u16, src_x: u16, src_y: u16,
            src_w: u16, src_h: u16, dst_x: i16, dst_y: i16, depth: u8, format: u8, send_event: u8, seg: u32, offset: u32) -> Cookie;
    }
}

dylib! {
    struct XcbCursor: ["libxcb-cursor.so.0"] {
        fn xcb_cursor_context_new(c: *mut Conn, screen: *const Screen, ctx: *mut *mut c_void) -> c_int;
        fn xcb_cursor_load_cursor(ctx: *mut c_void, name: *const c_char) -> u32;
    }
}

unsafe extern "C" {
    fn free(p: *mut c_void);
}

// Predefined X11 atoms.
const ATOM_ATOM: u32 = 4;
const ATOM_CARDINAL: u32 = 6;
const ATOM_RESOURCE_MANAGER: u32 = 23;
const ATOM_STRING: u32 = 31;
const ATOM_WM_NAME: u32 = 39;
const ATOM_WM_NORMAL_HINTS: u32 = 40;
const ATOM_WM_SIZE_HINTS: u32 = 41;
const ATOM_WM_CLASS: u32 = 67;

const CW_BACK_PIXMAP: u32 = 1;
const CW_BIT_GRAVITY: u32 = 16;
const CW_EVENT_MASK: u32 = 2048;
const CW_CURSOR: u32 = 16384;
const PROPERTY_CHANGE: u32 = 0x400000;
// Keys, buttons, enter/leave, motion, Expose, StructureNotify, Focus; window properties for INCR.
const EVENT_MASK: u32 = 1 | 2 | 4 | 8 | 16 | 32 | 64 | 0x8000 | 0x20000 | 0x200000 | PROPERTY_CHANGE;
const Z_PIXMAP: u8 = 2;

// Event types.
const KEY_PRESS: u8 = 2;
const BUTTON_PRESS: u8 = 4;
const BUTTON_RELEASE: u8 = 5;
const MOTION_NOTIFY: u8 = 6;
const ENTER_NOTIFY: u8 = 7;
const LEAVE_NOTIFY: u8 = 8;
const EXPOSE: u8 = 12;
const CONFIGURE_NOTIFY: u8 = 22;
const PROPERTY_NOTIFY: u8 = 28;
const SELECTION_CLEAR: u8 = 29;
const SELECTION_REQUEST: u8 = 30;
const SELECTION_NOTIFY: u8 = 31;
const CLIENT_MESSAGE: u8 = 33;
const MAPPING_NOTIFY: u8 = 34;

/// Animation frame without a screen clock: X11 doesn't provide one without the Present extension.
const FRAME: Duration = Duration::from_micros(16_667);

fn fail(msg: &str) -> ! {
    log!("uison: {msg}");
    std::process::exit(1)
}

fn u16_at(e: &[u8], i: usize) -> u16 {
    u16::from_ne_bytes([e[i], e[i + 1]])
}

fn i16_at(e: &[u8], i: usize) -> i16 {
    u16_at(e, i) as i16
}

fn u32_at(e: &[u8], i: usize) -> u32 {
    u32::from_ne_bytes(e[i..i + 4].try_into().unwrap())
}

#[derive(Clone, Copy, Debug)]
struct Px {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

struct Buffer {
    map: Map,
    image: Image,
    w: i32,
    h: i32,
    /// MIT-SHM segment (0: output via `xcb_put_image`).
    seg: u32,
    /// The server is still reading the buffer (waiting for ShmCompletion).
    busy: bool,
}

struct X11 {
    x: Xcb,
    shm: Option<XcbShm>,
    shm_event: u8,
    c: *mut Conn,
    screen: *const Screen,
    win: u32,
    gc: u32,
    depth: u8,
    atoms: Atoms,
    buf: Option<Buffer>,
    shell: Shell,
    scale: f32,
    /// Window size in pixels.
    px: (i32, i32),
    exposed: Vec<Px>,
    mapped: bool,
    buttons_down: u32,
    kb: Option<Keyboard>,
    /// First event of the XKB extension (layout change).
    xkb_event: u8,
    /// The layout changed. One change arrives as a burst of events, and xdotool remaps a spare key for each character
    /// outside the layout and restores it a few ms later: rereading the layout (round trips) on every event made us
    /// late — the key was read with the restored layout. Now: reread at the first event of a batch (the key queued
    /// behind it is read with that layout), later changes in the batch — once, after it.
    keymap_read: bool,
    keymap_stale: bool,
    /// By `Cursor as usize`.
    cursors: [u32; 5],
    shown_cursor: Option<Cursor>,
    portal: Option<Portal>,
    last_tick: Instant,
    /// Server time of the last input: used to take clipboard ownership and to request it (ICCCM).
    last_time: u32,
    /// Our text in `CLIPBOARD` (`None`: another program owns the clipboard).
    clip: Option<String>,
    /// Waiting for a paste reply; `Some(data)`: it arrives in parts (INCR).
    paste: Option<Option<Vec<u8>>>,
    quit: bool,
    /// UISON_WLDEBUG=1: events to stderr.
    debug: bool,
}

struct Atoms {
    wm_protocols: u32,
    wm_delete: u32,
    net_wm_name: u32,
    utf8: u32,
    net_wm_pid: u32,
    net_wm_icon: u32,
    clipboard: u32,
    targets: u32,
    incr: u32,
    text: u32,
    text_plain: u32,
    /// Property of our window where the clipboard owner puts the reply.
    paste_prop: u32,
}

pub fn run(app: impl App + 'static, title: &str) {
    let started = Instant::now();
    let x = Xcb::load().unwrap_or_else(|e| fail(&format!("X11: {e}")));
    let mut screen_n = 0;
    let c = unsafe { x.xcb_connect(std::ptr::null(), &mut screen_n) };
    if c.is_null() || unsafe { x.xcb_connection_has_error(c) } != 0 {
        fail("X11: cannot connect to display (DISPLAY)");
    }
    let screen = unsafe {
        let mut it = x.xcb_setup_roots_iterator(x.xcb_get_setup(c));
        for _ in 0..screen_n {
            x.xcb_screen_next(&mut it);
        }
        it.data
    };
    let atom = |name: &str| -> u32 {
        unsafe {
            let ck = x.xcb_intern_atom(c, 0, name.len() as u16, name.as_ptr().cast());
            let r = x.xcb_intern_atom_reply(c, ck, std::ptr::null_mut());
            if r.is_null() {
                return 0;
            }
            let a = u32::from_ne_bytes(std::slice::from_raw_parts(r.add(8), 4).try_into().unwrap());
            free(r.cast());
            a
        }
    };
    let atoms = Atoms {
        wm_protocols: atom("WM_PROTOCOLS"),
        wm_delete: atom("WM_DELETE_WINDOW"),
        net_wm_name: atom("_NET_WM_NAME"),
        utf8: atom("UTF8_STRING"),
        net_wm_pid: atom("_NET_WM_PID"),
        net_wm_icon: atom("_NET_WM_ICON"),
        clipboard: atom("CLIPBOARD"),
        targets: atom("TARGETS"),
        incr: atom("INCR"),
        text: atom("TEXT"),
        text_plain: atom("text/plain;charset=utf-8"),
        paste_prop: atom("UISON_PASTE"),
    };
    let shell = Shell::new(app, started);
    let portal = Portal::connect();
    let mut w = X11 {
        shm: None,
        shm_event: 0,
        c,
        screen,
        win: 0,
        gc: 0,
        depth: unsafe { (*screen).root_depth },
        atoms,
        buf: None,
        shell,
        scale: 1.0,
        px: (1, 1),
        exposed: Vec::new(),
        mapped: false,
        buttons_down: 0,
        kb: None,
        xkb_event: 0,
        keymap_read: false,
        keymap_stale: false,
        cursors: [0; 5],
        shown_cursor: None,
        portal,
        last_tick: started,
        last_time: 0,
        clip: None,
        paste: None,
        quit: false,
        debug: crate::shell::env_flag("UISON_WLDEBUG"),
        x,
    };
    w.start(title);
    w.run_loop();
}

impl X11 {
    fn start(&mut self, title: &str) {
        let (x, c) = (&self.x, self.c);
        let root = unsafe { (*self.screen).root };
        self.scale = xft_scale(x, c, root);
        let fs = self.shell.rt.frame.size;
        self.px = ((fs.w * self.scale).round() as i32, (fs.h * self.scale).round() as i32);
        unsafe {
            self.win = x.xcb_generate_id(c);
            // No background: the server doesn't paint exposed areas (no flicker on resize).
            let values = [0u32, 1, EVENT_MASK];
            x.xcb_create_window(c, 0, self.win, root, 0, 0, self.px.0 as u16, self.px.1 as u16, 0, 1, (*self.screen).root_visual,
                CW_BACK_PIXMAP | CW_BIT_GRAVITY | CW_EVENT_MASK, values.as_ptr());
            // Xft.dpi change (scale changed in settings) is an event on the root window.
            x.xcb_change_window_attributes(c, root, CW_EVENT_MASK, [PROPERTY_CHANGE].as_ptr());
            let prop = |p: u32, ty: u32, fmt: u8, data: &[u8]| {
                x.xcb_change_property(c, 0, self.win, p, ty, fmt, (data.len() / (fmt as usize / 8)) as u32, data.as_ptr().cast());
            };
            prop(self.atoms.net_wm_name, self.atoms.utf8, 8, title.as_bytes());
            prop(ATOM_WM_NAME, ATOM_STRING, 8, title.as_bytes());
            let exe = std::env::current_exe().ok().and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned())).unwrap_or_default();
            prop(ATOM_WM_CLASS, ATOM_STRING, 8, format!("{exe}\0uison\0").as_bytes());
            prop(self.atoms.wm_protocols, ATOM_ATOM, 32, &self.atoms.wm_delete.to_ne_bytes());
            prop(self.atoms.net_wm_pid, ATOM_CARDINAL, 32, &std::process::id().to_ne_bytes());
            // Icon from `uison::set_icon`: the window manager shows it in the taskbar and the switcher.
            if let Some(icon) = uison_core::icon::icon().and_then(crate::image::icon_argb) {
                let bytes: Vec<u8> = icon.iter().flat_map(|v| v.to_ne_bytes()).collect();
                prop(self.atoms.net_wm_icon, ATOM_CARDINAL, 32, &bytes);
            }
            // WM_NORMAL_HINTS: minimum size (PMinSize flag).
            let mut hints = [0u32; 18];
            hints[0] = 16;
            hints[5] = (240.0 * self.scale) as u32;
            hints[6] = (160.0 * self.scale) as u32;
            let hb: Vec<u8> = hints.iter().flat_map(|v| v.to_ne_bytes()).collect();
            prop(ATOM_WM_NORMAL_HINTS, ATOM_WM_SIZE_HINTS, 32, &hb);
            self.gc = x.xcb_generate_id(c);
            x.xcb_create_gc(c, self.gc, self.win, 0, std::ptr::null());
        }
        self.init_shm();
        self.init_cursors();
        let mut kb = Keyboard::new().map_err(|e| log!("uison: {e}")).ok();
        if let Some(k) = kb.as_mut() {
            self.xkb_event = k.set_keymap_x11(self.c);
            if self.xkb_event == 0 {
                log!("uison: no XKB keymap (libxkbcommon-x11) — keyboard disabled");
                kb = None;
            }
        }
        self.kb = kb;
        unsafe {
            self.x.xcb_map_window(self.c, self.win);
            self.x.xcb_flush(self.c);
        }
        if self.shell.stats {
            let line = format!(
                "window: {} ms, X11, scale {}%, shm {}",
                self.shell.started.elapsed().as_millis(),
                (self.scale * 100.0).round() as u32,
                self.shm.is_some()
            );
            self.shell.stat(line);
        }
    }

    /// MIT-SHM with descriptor passing (version 1.2+, X.Org 1.18+); otherwise `xcb_put_image`.
    fn init_shm(&mut self) {
        if std::env::var("UISON_SHM").as_deref() == Ok("0") {
            return;
        }
        let Ok(shm) = XcbShm::load() else { return };
        let Ok(h) = dl::open(&["libxcb-shm.so.0"]) else { return };
        let Ok(id) = dl::sym(h, "xcb_shm_id\0") else { return };
        unsafe {
            let ext = self.x.xcb_get_extension_data(self.c, id);
            if ext.is_null() || (*ext).present == 0 {
                return;
            }
            let ck = shm.xcb_shm_query_version(self.c);
            let v = shm.xcb_shm_query_version_reply(self.c, ck, std::ptr::null_mut());
            if v.is_null() {
                return;
            }
            let ok = ((*v).major, (*v).minor) >= (1, 2);
            free(v.cast());
            if ok {
                self.shm_event = (*ext).first_event;
                self.shm = Some(shm);
            }
        }
    }

    /// Cursors from the theme (libxcb-cursor), otherwise from the X11 cursor font.
    fn init_cursors(&mut self) {
        let names: [&[&str]; 5] = [
            &["default", "left_ptr"],
            &["pointer", "hand2"],
            &["text", "xterm"],
            &["col-resize", "ew-resize", "sb_h_double_arrow"],
            &["row-resize", "ns-resize", "sb_v_double_arrow"],
        ];
        if let Ok(xc) = XcbCursor::load() {
            let mut ctx = std::ptr::null_mut();
            if unsafe { xc.xcb_cursor_context_new(self.c, self.screen, &mut ctx) } >= 0 {
                for (i, list) in names.iter().enumerate() {
                    for n in *list {
                        let s = format!("{n}\0");
                        let id = unsafe { xc.xcb_cursor_load_cursor(ctx, s.as_ptr().cast()) };
                        if id != 0 {
                            self.cursors[i] = id;
                            break;
                        }
                    }
                }
            }
        }
        if self.cursors.contains(&0) {
            let (x, c) = (&self.x, self.c);
            unsafe {
                let font = x.xcb_generate_id(c);
                x.xcb_open_font(c, font, 6, c"cursor".as_ptr());
                // XC_left_ptr, XC_hand2, XC_xterm, XC_sb_h_double_arrow, XC_sb_v_double_arrow.
                for (i, glyph) in [68u16, 60, 152, 108, 116].into_iter().enumerate() {
                    if self.cursors[i] == 0 {
                        let id = x.xcb_generate_id(c);
                        x.xcb_create_glyph_cursor(c, id, font, font, glyph, glyph + 1, 0, 0, 0, 0xffff, 0xffff, 0xffff);
                        self.cursors[i] = id;
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------ loop

    fn run_loop(&mut self) {
        let fd = unsafe { self.x.xcb_get_file_descriptor(self.c) };
        loop {
            self.drain();
            self.asks();
            self.timers();
            self.update();
            unsafe { self.x.xcb_flush(self.c) };
            if self.quit || unsafe { self.x.xcb_connection_has_error(self.c) } != 0 {
                break;
            }
            let now = Instant::now();
            let timeout = self.next_deadline().map_or(-1, |d| d.saturating_duration_since(now).as_micros().div_ceil(1000) as i32);
            let mut fds = vec![fd];
            let mut slot = |f: Option<i32>| f.map(|f| {
                fds.push(f);
                1u32 << (fds.len() - 1)
            });
            let portal_bit = slot(self.portal.as_ref().map(Portal::fd)).unwrap_or(0);
            let wake_bit = slot(self.shell.wake.as_ref().map(|w| w.fd())).unwrap_or(0);
            let ready = wait_any(&fds, timeout).unwrap_or_else(|e| fail(&format!("poll: {e}")));
            if ready & wake_bit != 0 {
                self.shell.woke();
            }
            // Cursors on X11: theme from server resources (`Xcursor.theme`, libxcb-cursor).
            if ready & portal_bit != 0 {
                for s in self.portal.as_mut().map(Portal::poll).unwrap_or_default() {
                    match s {
                        Setting::Dark(dark) => _ = self.shell.set_dark(dark),
                        Setting::Font(f) => _ = self.shell.set_font(&f),
                        Setting::OpenFailed(uri) => xdg_open(&uri),
                        _ => {}
                    }
                }
            }
        }
    }

    fn next_deadline(&self) -> Option<Instant> {
        let mut d = self.shell.next_deadline();
        if self.shell.frame_tick {
            let t = self.last_tick + FRAME;
            d = Some(d.map_or(t, |x| x.min(t)));
        }
        d
    }

    fn timers(&mut self) {
        let now = Instant::now();
        if self.shell.tick_at.is_some_and(|t| t <= now) {
            self.shell.tick();
        }
        if self.shell.frame_tick && now >= self.last_tick + FRAME {
            self.last_tick = now;
            self.shell.tick();
        }
        if self.shell.capture_at.is_some_and(|t| t <= now) {
            self.shell.capture_at = None;
            self.capture(None);
        }
        if self.shell.script_at.is_some_and(|t| t <= now) {
            match self.shell.step_script() {
                Some(ScriptStep::Capture(name)) => self.capture(Some(&name)),
                Some(ScriptStep::Done) => self.quit = true,
                None => {}
            }
        }
    }

    fn capture(&mut self, name: Option<&str>) {
        if self.shell.has_damage() && self.mapped {
            self.draw();
        }
        if let Some(b) = &self.buf {
            let image: *const Image = &b.image;
            self.shell.save_capture(unsafe { &*image }, name);
        }
    }

    fn update(&mut self) {
        self.update_cursor();
        if !self.mapped {
            return;
        }
        if self.shell.has_damage() {
            self.draw();
        } else if !self.exposed.is_empty() && self.buf.as_ref().is_some_and(|b| !b.busy) {
            // Part of the window was exposed: copy from the buffer, no redraw.
            let rects = std::mem::take(&mut self.exposed);
            self.put(&rects);
        }
        if !self.shell.frame_tick {
            self.last_tick = Instant::now();
        }
    }

    fn update_cursor(&mut self) {
        let c = self.shell.cursor;
        if self.shown_cursor == Some(c) {
            return;
        }
        self.shown_cursor = Some(c);
        let id = self.cursors[c as usize];
        unsafe { self.x.xcb_change_window_attributes(self.c, self.win, CW_CURSOR, [id].as_ptr()) };
    }

    // ------------------------------------------------------------ drawing

    fn ensure_buffer(&mut self) -> bool {
        let (w, h) = self.px;
        if self.buf.as_ref().is_some_and(|b| b.w == w && b.h == h) {
            return true;
        }
        if let Some(old) = self.buf.take()
            && old.seg != 0
            && let Some(shm) = &self.shm
        {
            unsafe { shm.xcb_shm_detach(self.c, old.seg) };
        }
        let stride = w * 4;
        let len = (stride * h) as usize;
        let Ok(file) = memfd(len) else { return false };
        let Ok(map) = Map::shared(file.as_raw_fd(), len) else { return false };
        let mut seg = 0;
        if let Some(shm) = &self.shm {
            seg = unsafe { self.x.xcb_generate_id(self.c) };
            // xcb takes the descriptor (closes it after sending).
            unsafe { shm.xcb_shm_attach_fd(self.c, seg, file.into_raw_fd(), 1) };
        }
        let image = unsafe { Image::over(map.ptr, w, h, stride) };
        self.buf = Some(Buffer { map, image, w, h, seg, busy: false });
        self.shell.invalidate();
        true
    }

    fn draw(&mut self) {
        if self.buf.as_ref().is_some_and(|b| b.busy) || !self.ensure_buffer() {
            return;
        }
        let (pw, ph) = self.px;
        let s = self.scale;
        let full = Px { x: 0, y: 0, w: pw, h: ph };
        let rects: Vec<Px> = match self.shell.take_damage() {
            None => vec![full],
            Some(rs) => rs
                .iter()
                .filter_map(|r| {
                    let x0 = ((r.x * s).floor() as i32).clamp(0, pw);
                    let y0 = ((r.y * s).floor() as i32).clamp(0, ph);
                    let x1 = ((r.right() * s).ceil() as i32).clamp(0, pw);
                    let y1 = ((r.bottom() * s).ceil() as i32).clamp(0, ph);
                    (x1 > x0 && y1 > y0).then_some(Px { x: x0, y: y0, w: x1 - x0, h: y1 - y0 })
                })
                .collect(),
        };
        let pts: Vec<Rect> = rects.iter().map(|p| Rect::new(p.x as f32 / s, p.y as f32 / s, p.w as f32 / s, p.h as f32 / s)).collect();
        let b = self.buf.as_ref().unwrap();
        let cr = b.image.cr;
        self.shell.paint(cr, s, Point::default(), &pts);
        self.buf.as_ref().unwrap().image.flush();
        let mut out = rects;
        out.append(&mut self.exposed);
        self.put(&out);
        self.shell.presented();
    }

    /// Copy buffer rectangles to the window.
    fn put(&mut self, rects: &[Px]) {
        let Some(b) = self.buf.as_mut() else { return };
        let (x, c) = (&self.x, self.c);
        if let Some(shm) = &self.shm {
            for (i, r) in rects.iter().enumerate() {
                let last = (i + 1 == rects.len()) as u8;
                unsafe {
                    shm.xcb_shm_put_image(c, self.win, self.gc, b.w as u16, b.h as u16, r.x as u16, r.y as u16, r.w as u16, r.h as u16,
                        r.x as i16, r.y as i16, self.depth, Z_PIXMAP, last, b.seg, 0);
                }
            }
            // Until the server has read the buffer (ShmCompletion), we must not draw into it.
            b.busy = !rects.is_empty();
            return;
        }
        // Without SHM: copy the rectangle row by row and send in parts no larger than a request.
        let max = (unsafe { x.xcb_get_maximum_request_length(c) } as usize * 4).saturating_sub(64).max(4096);
        let data = b.map.bytes();
        for r in rects {
            let row = r.w as usize * 4;
            let rows_per = (max / row).max(1);
            let mut y = r.y;
            while y < r.y + r.h {
                let n = (rows_per as i32).min(r.y + r.h - y);
                let mut chunk = Vec::with_capacity(row * n as usize);
                for yy in y..y + n {
                    let off = (yy * b.w + r.x) as usize * 4;
                    chunk.extend_from_slice(&data[off..off + row]);
                }
                unsafe {
                    x.xcb_put_image(c, Z_PIXMAP, self.win, self.gc, r.w as u16, n as u16, r.x as i16, y as i16, 0, self.depth,
                        chunk.len() as u32, chunk.as_ptr());
                }
                y += n;
            }
        }
    }

    // ------------------------------------------------------------ events

    fn drain(&mut self) {
        self.keymap_read = false;
        loop {
            let e = unsafe { self.x.xcb_poll_for_event(self.c) };
            if e.is_null() {
                break;
            }
            let ev: [u8; 32] = unsafe { std::slice::from_raw_parts(e, 32).try_into().unwrap() };
            unsafe { free(e.cast()) };
            self.event(&ev);
        }
        if std::mem::take(&mut self.keymap_stale) {
            self.read_keymap();
        }
    }

    fn keymap_changed(&mut self) {
        if self.keymap_read {
            self.keymap_stale = true;
        } else {
            self.read_keymap();
        }
    }

    fn read_keymap(&mut self) {
        self.keymap_read = true;
        if let Some(kb) = self.kb.as_mut() {
            kb.set_keymap_x11(self.c);
        }
    }

    fn point(&self, e: &[u8]) -> Point {
        // event_x, event_y are at offsets 24 and 26 in key, button and motion events.
        Point::new(i16_at(e, 24) as f32 / self.scale, i16_at(e, 26) as f32 / self.scale)
    }

    fn event(&mut self, e: &[u8; 32]) {
        let kind = e[0] & 0x7f;
        if self.debug {
            log!("x11: event {kind} detail {}", e[1]);
        }
        if self.shm.is_some() && kind == self.shm_event {
            // ShmCompletion: the server has finished reading the buffer.
            if let Some(b) = self.buf.as_mut() {
                b.busy = false;
            }
            return;
        }
        if self.xkb_event != 0 && kind == self.xkb_event {
            // xkbType: 0 NewKeyboardNotify, 1 MapNotify: reread the layout.
            if e[1] <= 1 {
                self.keymap_changed();
            }
            return;
        }
        match kind {
            KEY_PRESS => {
                let code = e[1] as u32;
                let state = u16_at(e, 28);
                self.last_time = u32_at(e, 4);
                if let Some(kb) = self.kb.as_mut() {
                    kb.set_core_state(state);
                    if let Some(key) = kb.key(code) {
                        self.shell.dispatch(Event::Key(key));
                    }
                }
            }
            BUTTON_PRESS | BUTTON_RELEASE => {
                let p = self.point(e);
                let down = kind == BUTTON_PRESS;
                self.last_time = u32_at(e, 4);
                // Modifiers from the event state: Shift, Control (main), Mod1 (Alt).
                let s = u16_at(e, 28);
                self.shell.mods = Mods { shift: s & 1 != 0, ctrl: false, alt: s & 8 != 0, cmd: s & 4 != 0 };
                let step = 48.0;
                match e[1] {
                    // Wheel: 4/5 up/down, 6/7 left/right (press only).
                    4..=7 => {
                        if down {
                            let (dx, dy) = match e[1] {
                                4 => (0.0, step),
                                5 => (0.0, -step),
                                6 => (step, 0.0),
                                _ => (-step, 0.0),
                            };
                            self.shell.dispatch(Event::Scroll(p, dx, dy));
                        }
                    }
                    b => {
                        let btn = match b {
                            1 => MouseButton::Left,
                            3 => MouseButton::Right,
                            _ => MouseButton::Other,
                        };
                        if down {
                            self.buttons_down += 1;
                            self.shell.dispatch(Event::MouseDown(p, btn));
                        } else {
                            self.buttons_down = self.buttons_down.saturating_sub(1);
                            self.shell.dispatch(Event::MouseUp(p, btn));
                        }
                    }
                }
            }
            MOTION_NOTIFY | ENTER_NOTIFY => {
                let p = self.point(e);
                self.shell.dispatch(Event::MouseMove(p));
            }
            LEAVE_NOTIFY => {
                // While a button is held the pointer is grabbed by the window: leaving doesn't count.
                if self.buttons_down == 0 {
                    self.shell.dispatch(Event::MouseLeave);
                }
            }
            EXPOSE => {
                let (x, y, w, h) = (u16_at(e, 8) as i32, u16_at(e, 10) as i32, u16_at(e, 12) as i32, u16_at(e, 14) as i32);
                if !self.mapped {
                    self.mapped = true;
                    if self.shell.stats {
                        let line = format!("mapped: {} ms", self.shell.started.elapsed().as_millis());
                        self.shell.stat(line);
                    }
                }
                if self.buf.is_some() {
                    self.exposed.push(Px { x, y, w, h });
                } else {
                    self.shell.invalidate();
                }
            }
            CONFIGURE_NOTIFY => {
                let (w, h) = (u16_at(e, 20) as i32, u16_at(e, 22) as i32);
                if (w, h) != self.px && w > 0 && h > 0 {
                    self.px = (w, h);
                    let size = Size::new(w as f32 / self.scale, h as f32 / self.scale);
                    self.shell.dispatch(Event::Resize(size));
                    self.shell.invalidate();
                }
            }
            // Next part of an INCR paste: our window's property got a new value (state 0).
            PROPERTY_NOTIFY if u32_at(e, 4) == self.win => {
                if u32_at(e, 8) == self.atoms.paste_prop && e[16] == 0 && matches!(self.paste, Some(Some(_))) {
                    self.read_paste();
                }
            }
            PROPERTY_NOTIFY => {
                if u32_at(e, 8) == ATOM_RESOURCE_MANAGER {
                    let root = unsafe { (*self.screen).root };
                    let s = xft_scale(&self.x, self.c, root);
                    if s != self.scale {
                        self.scale = s;
                        let size = Size::new(self.px.0 as f32 / s, self.px.1 as f32 / s);
                        self.shell.dispatch(Event::Resize(size));
                        self.shell.invalidate();
                    }
                }
            }
            CLIENT_MESSAGE => {
                if u32_at(e, 8) == self.atoms.wm_protocols && u32_at(e, 12) == self.atoms.wm_delete {
                    self.quit = true;
                }
            }
            MAPPING_NOTIFY => self.keymap_changed(),
            SELECTION_CLEAR if u32_at(e, 12) == self.atoms.clipboard => self.clip = None,
            SELECTION_REQUEST => self.selection_request(e),
            SELECTION_NOTIFY if self.paste.is_some() => {
                // Reply to ConvertSelection: property 0 means the owner didn't hand over the text.
                if u32_at(e, 20) == 0 {
                    self.paste = None;
                } else {
                    self.read_paste();
                }
            }
            _ => {}
        }
    }

    // ------------------------------------------------------------ clipboard, links

    fn asks(&mut self) {
        for a in self.shell.take_asks() {
            match a {
                Ask::Copy(text) => {
                    unsafe { self.x.xcb_set_selection_owner(self.c, self.win, self.atoms.clipboard, self.last_time) };
                    self.clip = Some(text);
                }
                Ask::Paste => match &self.clip {
                    // The clipboard is ours: no exchange through the server.
                    Some(t) => {
                        let t = t.clone();
                        self.shell.dispatch(Event::Paste(t.into()));
                    }
                    None => {
                        let a = &self.atoms;
                        unsafe { self.x.xcb_convert_selection(self.c, self.win, a.clipboard, a.utf8, a.paste_prop, self.last_time) };
                        self.paste = Some(None);
                    }
                },
                Ask::OpenUrl(u) => match self.portal.as_mut() {
                    Some(p) => p.open_uri(&u),
                    None => xdg_open(&u),
                },
            }
        }
    }

    /// Another program asks for our clipboard: put the text into its window's property and notify.
    fn selection_request(&mut self, e: &[u8; 32]) {
        let (time, requestor, selection, target) = (u32_at(e, 4), u32_at(e, 12), u32_at(e, 16), u32_at(e, 20));
        // Old clients don't specify a property: then it equals the target (ICCCM).
        let mut prop = match u32_at(e, 24) {
            0 => target,
            p => p,
        };
        let a = &self.atoms;
        let (x, c) = (&self.x, self.c);
        let max = unsafe { x.xcb_get_maximum_request_length(c) } as usize * 4 - 64;
        match &self.clip {
            Some(_) if selection == a.clipboard && target == a.targets => {
                let list = [a.targets, a.utf8, a.text_plain, a.text, ATOM_STRING];
                unsafe { x.xcb_change_property(c, 0, requestor, prop, ATOM_ATOM, 32, list.len() as u32, list.as_ptr().cast()) };
            }
            // We serve only one request (chunked transfer, INCR, isn't needed for field text).
            Some(t) if selection == a.clipboard && [a.utf8, a.text_plain, a.text, ATOM_STRING].contains(&target) && t.len() <= max => {
                let ty = if target == ATOM_STRING { ATOM_STRING } else { a.utf8 };
                unsafe { x.xcb_change_property(c, 0, requestor, prop, ty, 8, t.len() as u32, t.as_ptr().cast()) };
            }
            _ => prop = 0,
        }
        let mut ev = [0u8; 32];
        ev[0] = SELECTION_NOTIFY;
        for (i, v) in [(4, time), (8, requestor), (12, selection), (16, target), (20, prop)] {
            ev[i..i + 4].copy_from_slice(&v.to_ne_bytes());
        }
        unsafe { x.xcb_send_event(c, 0, requestor, 0, ev.as_ptr().cast()) };
    }

    /// Take the reply from our property (and delete it: that tells the owner to send more).
    fn read_paste(&mut self) {
        let (x, c) = (&self.x, self.c);
        let (ty, data) = unsafe {
            let ck = x.xcb_get_property(c, 1, self.win, self.atoms.paste_prop, 0, 0, u32::MAX / 4);
            let r = x.xcb_get_property_reply(c, ck, std::ptr::null_mut());
            if r.is_null() {
                self.paste = None;
                return;
            }
            // xcb_get_property_reply_t: type is at offset 8.
            let ty = u32::from_ne_bytes(std::slice::from_raw_parts(r.cast::<u8>().add(8), 4).try_into().unwrap());
            let n = x.xcb_get_property_value_length(r).max(0) as usize;
            let data = std::slice::from_raw_parts(x.xcb_get_property_value(r), n).to_vec();
            free(r);
            (ty, data)
        };
        match self.paste.take() {
            // Start of a chunked transfer: then a PropertyNotify per part, an empty one ends it.
            Some(None) if ty == self.atoms.incr => self.paste = Some(Some(Vec::new())),
            Some(None) => {
                self.shell.dispatch(Event::Paste(String::from_utf8_lossy(&data).into()));
            }
            Some(Some(mut acc)) if !data.is_empty() => {
                acc.extend_from_slice(&data);
                self.paste = Some(Some(acc));
            }
            Some(Some(acc)) => {
                self.shell.dispatch(Event::Paste(String::from_utf8_lossy(&acc).into()));
            }
            None => {}
        }
    }
}

/// Scale from `Xft.dpi` (96 = 100 %); `UISON_SCALE` sets it manually.
fn xft_scale(x: &Xcb, c: *mut Conn, root: u32) -> f32 {
    if let Some(s) = std::env::var("UISON_SCALE").ok().and_then(|v| v.parse::<f32>().ok()) {
        return s.max(0.5);
    }
    unsafe {
        let ck = x.xcb_get_property(c, 0, root, ATOM_RESOURCE_MANAGER, ATOM_STRING, 0, 1 << 16);
        let r = x.xcb_get_property_reply(c, ck, std::ptr::null_mut());
        if r.is_null() {
            return 1.0;
        }
        let n = x.xcb_get_property_value_length(r).max(0) as usize;
        let text = String::from_utf8_lossy(std::slice::from_raw_parts(x.xcb_get_property_value(r), n)).into_owned();
        free(r);
        let dpi = text.lines().find_map(|l| l.strip_prefix("Xft.dpi:").map(|v| v.trim().parse::<f32>().unwrap_or(96.0)));
        // Round to a quarter: 120 dpi -> 1.25, 144 -> 1.5.
        dpi.map_or(1.0, |d| ((d / 96.0) * 4.0).round().max(4.0) / 4.0)
    }
}
