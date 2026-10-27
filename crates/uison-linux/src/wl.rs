//! Wayland window on our own protocol code (`wire.rs`). Only `wl_compositor`,
//! `wl_shm` and `xdg_wm_base` are required; every compositor has them. The rest (fractional scale,
//! server-side frame, named cursors) is used if the compositor advertises it, otherwise a fallback.
//! Request and event opcodes follow the protocol XML (wayland.xml, wayland-protocols).

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::time::{Duration, Instant};

use uison_core::{App, Cursor, Event, MouseButton, Point, Rect, Size};

use crate::cursor::{CursorTheme, Shape, names};
use crate::decor::{Decor, Part};
use crate::gfx::Image;
use crate::kinetic::Kinetic;
use crate::shell::{Ask, ScriptStep, Shell, env_flag, xdg_open};
use crate::dbus::{Portal, Setting};
use crate::sys::{Map, memfd, pipe, wait_any};
use crate::wire::{Arg, Body, Conn, Msg};
use crate::xkb::Keyboard;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    Display,
    Registry,
    SyncCallback,
    FrameCallback,
    Compositor,
    Shm,
    Buffer(usize),
    Surface,
    CursorSurface,
    Seat,
    Pointer,
    Keyboard,
    Touch,
    Output(u32),
    WmBase,
    XdgSurface,
    Toplevel,
    Decoration,
    Fractional,
    DataDevice,
    DataSource,
    DataOffer,
    Other,
}

/// Clipboard text: MIME types in order of preference (as in GTK and Qt).
const TEXT_MIMES: [&str; 5] = ["text/plain;charset=utf-8", "UTF8_STRING", "text/plain", "TEXT", "STRING"];

/// Pixel rectangle of a buffer.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Px {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

impl Px {
    fn intersect(&self, o: &Px) -> Option<Px> {
        let (x0, y0) = (self.x.max(o.x), self.y.max(o.y));
        let (x1, y1) = ((self.x + self.w).min(o.x + o.w), (self.y + self.h).min(o.y + o.h));
        (x1 > x0 && y1 > y0).then_some(Px { x: x0, y: y0, w: x1 - x0, h: y1 - y0 })
    }
}

struct Buf {
    id: u32,
    _file: File,
    _map: Map,
    image: Image,
    w: i32,
    h: i32,
    /// With alpha (our frame's shadow) or opaque.
    argb: bool,
    busy: bool,
    /// What in this buffer is stale since it was last drawn (`None`: everything).
    stale: Option<Vec<Px>>,
}

struct Output {
    id: u32,
    name: u32,
    scale: i32,
}

/// Versions the code is written for (we take the smaller of this and the compositor's).
const V_COMPOSITOR: u32 = 6;
const V_SEAT: u32 = 8;
const V_OUTPUT: u32 = 4;
const V_WM_BASE: u32 = 6;

#[derive(Default)]
struct Globals {
    compositor: u32,
    compositor_v: u32,
    shm: u32,
    seat: u32,
    seat_v: u32,
    wm_base: u32,
    deco_mgr: u32,
    viewporter: u32,
    frac_mgr: u32,
    cursor_mgr: u32,
    data_mgr: u32,
}

/// Scrolling within one pointer frame (`wl_pointer.frame`).
#[derive(Default)]
struct Axis {
    source: Option<u32>,
    value: [f32; 2],
    v120: [i32; 2],
    stop: bool,
}

/// Where a finger touch went.
#[derive(Clone, Copy, PartialEq)]
enum TouchTarget {
    App,
    Button(Part),
    Window,
}

struct Wl {
    c: Conn,
    objs: HashMap<u32, Kind>,
    g: Globals,
    outputs: Vec<Output>,
    surface: u32,
    xdg_surface: u32,
    toplevel: u32,
    viewport: u32,
    fractional: u32,
    decoration: u32,
    pointer: u32,
    keyboard: u32,
    touch: u32,
    cursor_dev: u32,
    bufs: Vec<Option<Buf>>,
    /// Buffer of the last presentation (for screenshots).
    last_buf: Option<usize>,
    shell: Shell,
    /// App content size in logical points (without our own frame).
    size: (i32, i32),
    /// Scale x 120 (as in `fractional-scale-v1`; integer k is k x 120).
    scale120: u32,
    frac120: Option<u32>,
    pref_scale: Option<i32>,
    entered: Vec<u32>,
    /// Already told to the compositor (`set_buffer_scale`, viewport, window geometry).
    sent_buffer_scale: i32,
    sent_viewport: (i32, i32),
    sent_geometry: (i32, i32, i32, i32),
    configured: bool,
    /// The first frame waits for the portal's font and theme (no relayout, no flash of a light window), up to a deadline.
    portal_wait: Option<Instant>,
    pending_cfg: (i32, i32),
    decor: Decor,
    /// Redraw the frame (hover, window focus, state).
    decor_dirty: bool,
    /// Frame handed to the compositor but not shown yet: the next one waits for `frame`.
    frame_cb: u32,
    frame_at: Option<Instant>,
    /// Must draw right away (after configure) without waiting for `frame`.
    force_draw: bool,
    pointer_serial: u32,
    pointer_pos: Point,
    /// The press started in the content: motion and release go to the app, even over the frame.
    buttons_down: u32,
    app_hover: bool,
    shown_cursor: Option<Shape>,
    cursors: Option<CursorTheme>,
    /// Cursor theme and size from desktop settings (portal).
    cursor_pref: (Option<String>, Option<i32>),
    cursor_surface: u32,
    axis: Axis,
    /// Touchpad scroll inertia (on Wayland the app does it).
    kinetic: Kinetic,
    kinetic_pos: Point,
    kb: Option<Keyboard>,
    repeat: (i32, i32),
    repeat_key: Option<(u32, Instant)>,
    touch_id: Option<i32>,
    touch_pos: Point,
    touch_target: TouchTarget,
    /// Settings portal: dark theme and its changes.
    portal: Option<Portal>,
    /// Clipboard: the seat's data device and the serial of the last input (for `set_selection`).
    data_device: u32,
    input_serial: u32,
    /// Compositor offers (id -> MIME types): clipboard and drag over the window.
    offers: HashMap<u32, Vec<String>>,
    selection: u32,
    dnd_offer: u32,
    /// Our clipboard source and its text (`None`: another program owns the clipboard).
    clip: Option<(u32, String)>,
    /// Paste from another program: we read our end of the pipe to the end.
    paste: Option<(File, Vec<u8>)>,
    quit: bool,
    wheel_step: f32,
    /// UISON_WLDEBUG=1: all protocol events to stderr.
    debug: bool,
}

fn fail(msg: &str) -> ! {
    log!("uison: {msg}");
    std::process::exit(1)
}

pub fn run(app: impl App + 'static, title: &str) {
    let started = Instant::now();
    let c = Conn::connect().unwrap_or_else(|e| fail(&format!("Wayland: {e}")));
    let shell = Shell::new(app, started);
    let portal = Portal::connect();
    let fs = shell.rt.frame.size;
    let mut wl = Wl {
        c,
        objs: HashMap::from([(1, Kind::Display)]),
        g: Globals::default(),
        outputs: Vec::new(),
        surface: 0,
        xdg_surface: 0,
        toplevel: 0,
        viewport: 0,
        fractional: 0,
        decoration: 0,
        pointer: 0,
        keyboard: 0,
        touch: 0,
        cursor_dev: 0,
        bufs: Vec::new(),
        last_buf: None,
        shell,
        size: (fs.w.round() as i32, fs.h.round() as i32),
        scale120: 120,
        frac120: None,
        pref_scale: None,
        entered: Vec::new(),
        sent_buffer_scale: 1,
        sent_viewport: (0, 0),
        sent_geometry: (0, 0, 0, 0),
        configured: false,
        portal_wait: portal.is_some().then(|| Instant::now() + Duration::from_millis(150)),
        pending_cfg: (0, 0),
        decor: Decor::new(title),
        decor_dirty: false,
        frame_cb: 0,
        frame_at: None,
        force_draw: false,
        pointer_serial: 0,
        pointer_pos: Point::default(),
        buttons_down: 0,
        app_hover: false,
        shown_cursor: None,
        cursors: None,
        cursor_pref: (None, None),
        cursor_surface: 0,
        axis: Axis::default(),
        kinetic: Kinetic::default(),
        kinetic_pos: Point::default(),
        kb: None,
        repeat: (25, 600),
        repeat_key: None,
        touch_id: None,
        touch_pos: Point::default(),
        touch_target: TouchTarget::App,
        portal,
        data_device: 0,
        input_serial: 0,
        offers: HashMap::new(),
        selection: 0,
        dnd_offer: 0,
        clip: None,
        paste: None,
        quit: false,
        wheel_step: 48.0,
        debug: env_flag("UISON_WLDEBUG"),
    };
    wl.start(title);
    wl.run_loop();
}

impl Wl {
    fn new_obj(&mut self, kind: Kind) -> u32 {
        let id = self.c.new_id();
        self.objs.insert(id, kind);
        id
    }

    /// Wait for replies to everything sent (`wl_display.sync`).
    fn roundtrip(&mut self) {
        let cb = self.new_obj(Kind::SyncCallback);
        self.c.request(1, 0, &[Arg::New(cb)]); // wl_display.sync
        self.c.flush().unwrap_or_else(|e| fail(&format!("Wayland: {e}")));
        while self.objs.contains_key(&cb) {
            match self.c.read_blocking() {
                Ok(true) => {}
                Ok(false) => fail("Wayland: connection closed"),
                Err(e) => fail(&format!("Wayland: {e}")),
            }
            while let Some(m) = self.c.next() {
                self.handle(m);
            }
        }
    }

    fn start(&mut self, title: &str) {
        let reg = self.new_obj(Kind::Registry);
        self.c.request(1, 1, &[Arg::New(reg)]); // wl_display.get_registry
        self.roundtrip();
        if self.g.compositor == 0 || self.g.shm == 0 || self.g.wm_base == 0 {
            fail("compositor lacks wl_compositor / wl_shm / xdg_wm_base");
        }
        // Second round: input capabilities, monitor scales.
        self.roundtrip();
        if self.kb.is_none() && self.keyboard != 0 {
            log!("uison: no libxkbcommon — keyboard disabled");
        }
        if self.g.data_mgr != 0 && self.g.seat != 0 {
            self.data_device = self.new_obj(Kind::DataDevice);
            self.c.request(self.g.data_mgr, 1, &[Arg::New(self.data_device), Arg::Obj(self.g.seat)]); // get_data_device
        }

        self.surface = self.new_obj(Kind::Surface);
        self.c.request(self.g.compositor, 0, &[Arg::New(self.surface)]); // create_surface
        if self.g.frac_mgr != 0 && self.g.viewporter != 0 && std::env::var("UISON_FRACTIONAL").as_deref() != Ok("0") {
            self.fractional = self.new_obj(Kind::Fractional);
            self.c.request(self.g.frac_mgr, 1, &[Arg::New(self.fractional), Arg::Obj(self.surface)]);
            self.viewport = self.new_obj(Kind::Other);
            self.c.request(self.g.viewporter, 1, &[Arg::New(self.viewport), Arg::Obj(self.surface)]);
        }
        self.xdg_surface = self.new_obj(Kind::XdgSurface);
        self.c.request(self.g.wm_base, 2, &[Arg::New(self.xdg_surface), Arg::Obj(self.surface)]);
        self.toplevel = self.new_obj(Kind::Toplevel);
        self.c.request(self.xdg_surface, 1, &[Arg::New(self.toplevel)]); // get_toplevel
        self.c.request(self.toplevel, 2, &[Arg::Str(title)]); // set_title
        let exe = std::env::current_exe().ok().and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()));
        let app_id = format!("dev.uison.{}", exe.as_deref().unwrap_or("app"));
        self.c.request(self.toplevel, 3, &[Arg::Str(&app_id)]); // set_app_id
        if self.g.deco_mgr != 0 {
            self.decoration = self.new_obj(Kind::Decoration);
            self.c.request(self.g.deco_mgr, 1, &[Arg::New(self.decoration), Arg::Obj(self.toplevel)]);
            self.c.request(self.decoration, 1, &[Arg::Uint(2)]); // set_mode(server_side)
        } else {
            // The compositor doesn't draw a frame (GNOME, Weston): we draw our own.
            self.decor.on = true;
        }
        let min_h = 160 + self.decor.title_h() as i32;
        self.c.request(self.toplevel, 8, &[Arg::Int(240), Arg::Int(min_h)]); // set_min_size
        self.c.request(self.surface, 6, &[]); // commit without a buffer -> configure
        if self.shell.stats {
            let line = format!("window: {} ms, globals: {}", self.shell.started.elapsed().as_millis(), self.global_names());
            self.shell.stat(line);
        }
        self.roundtrip();
    }

    fn global_names(&self) -> String {
        let g = &self.g;
        let mut v = Vec::new();
        for (on, n) in [
            (g.deco_mgr, "xdg-decoration"),
            (g.frac_mgr, "fractional-scale"),
            (g.viewporter, "viewporter"),
            (g.cursor_mgr, "cursor-shape"),
            (g.data_mgr, "data-device"),
            (g.seat, "seat"),
        ] {
            if on != 0 {
                v.push(n);
            }
        }
        v.join(" ")
    }

    // ------------------------------------------------------------ loop

    fn run_loop(&mut self) {
        loop {
            self.asks();
            self.timers();
            self.update();
            if self.quit {
                break;
            }
            if let Err(e) = self.c.flush() {
                fail(&format!("Wayland: {e}"));
            }
            let now = Instant::now();
            let deadline = self.next_deadline();
            let timeout = deadline.map_or(-1, |d| d.saturating_duration_since(now).as_micros().div_ceil(1000) as i32);
            let mut fds = vec![self.c.fd()];
            let mut slot = |fd: Option<i32>| fd.map(|fd| {
                fds.push(fd);
                1u32 << (fds.len() - 1)
            });
            let portal_bit = slot(self.portal.as_ref().map(Portal::fd)).unwrap_or(0);
            let paste_bit = slot(self.paste.as_ref().map(|p| p.0.as_raw_fd())).unwrap_or(0);
            let wake_bit = slot(self.shell.wake.as_ref().map(|w| w.fd())).unwrap_or(0);
            let ready = wait_any(&fds, timeout).unwrap_or_else(|e| fail(&format!("poll: {e}")));
            if ready & paste_bit != 0 {
                self.read_paste();
            }
            if ready & wake_bit != 0 {
                self.shell.woke();
            }
            if ready & 1 != 0 {
                match self.c.read() {
                    Ok(true) => {}
                    Ok(false) => break,
                    Err(e) => fail(&format!("Wayland: {e}")),
                }
            }
            if ready & portal_bit != 0 {
                let settings = self.portal.as_mut().map(Portal::poll).unwrap_or_default();
                for s in settings {
                    self.setting(s);
                }
                if self.portal.as_ref().is_some_and(Portal::settled) {
                    self.portal_wait = None;
                }
            }
            while let Some(m) = self.c.next() {
                self.handle(m);
            }
        }
    }

    fn next_deadline(&self) -> Option<Instant> {
        let mut d = self.shell.next_deadline();
        let mut add = |t: Option<Instant>| {
            if let Some(t) = t {
                d = Some(d.map_or(t, |x: Instant| x.min(t)));
            }
        };
        add(self.repeat_key.map(|(_, t)| t));
        add(self.portal_wait);
        // Window hidden: the compositor sends no frames, so animation runs on a timer.
        if self.frame_cb != 0 && (self.shell.frame_tick || self.kinetic.active()) {
            add(self.frame_at.map(|t| t + Duration::from_millis(100)));
        }
        d
    }

    fn timers(&mut self) {
        let now = Instant::now();
        if self.shell.tick_at.is_some_and(|t| t <= now) {
            self.shell.tick();
        }
        if self.frame_cb != 0
            && (self.shell.frame_tick || self.kinetic.active())
            && self.frame_at.is_some_and(|t| now >= t + Duration::from_millis(100))
        {
            self.frame_at = Some(now);
            if self.shell.frame_tick {
                self.shell.tick();
            }
            self.kinetic_step();
        }
        if let Some((code, at)) = self.repeat_key
            && at <= now
        {
            let step = Duration::from_millis(1000 / self.repeat.0.max(1) as u64);
            self.repeat_key = Some((code, now + step));
            self.key_press(code);
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

    /// After a batch of events: draw the changes or request a frame for animation.
    fn update(&mut self) {
        self.update_cursor();
        if !self.configured {
            return;
        }
        if let Some(t) = self.portal_wait {
            if Instant::now() < t {
                return;
            }
            self.portal_wait = None;
        }
        let dirty = self.shell.has_damage() || self.decor_dirty;
        if dirty && (self.frame_cb == 0 || self.force_draw) {
            self.draw();
        } else if (self.shell.frame_tick || self.kinetic.active()) && self.frame_cb == 0 {
            // Animation with no changes in this tick: only a frame request.
            self.request_frame();
            self.c.request(self.surface, 6, &[]); // commit
        }
    }

    fn request_frame(&mut self) {
        self.frame_cb = self.new_obj(Kind::FrameCallback);
        self.frame_at = Some(Instant::now());
        self.c.request(self.surface, 3, &[Arg::New(self.frame_cb)]); // frame
    }

    fn frame_done(&mut self) {
        self.frame_cb = 0;
        if self.shell.frame_tick {
            self.shell.tick();
        }
        self.kinetic_step();
    }

    fn kinetic_step(&mut self) {
        if let Some((dx, dy)) = self.kinetic.step()
            && (dx != 0.0 || dy != 0.0)
        {
            self.shell.dispatch(Event::Scroll(self.kinetic_pos, dx, dy));
        }
    }

    fn capture(&mut self, name: Option<&str>) {
        if (self.shell.has_damage() || self.decor_dirty) && self.configured {
            self.force_draw = true;
            self.draw();
        }
        let Some(i) = self.last_buf else { return };
        if let Some(b) = &self.bufs[i] {
            // Via a pointer: `save_capture` takes `&mut shell`, the image is in the window's buffer.
            let image: *const Image = &b.image;
            self.shell.save_capture(unsafe { &*image }, name);
        } else if name.is_some() {
            log!("uison: capture {}", name.unwrap_or_default());
        }
    }

    // ------------------------------------------------------------ scale and drawing

    fn scale(&self) -> f32 {
        self.scale120 as f32 / 120.0
    }

    fn update_scale(&mut self) {
        let int = self.pref_scale.unwrap_or_else(|| {
            self.outputs.iter().filter(|o| self.entered.contains(&o.id)).map(|o| o.scale).max().unwrap_or(1)
        });
        let s120 = self.frac120.filter(|_| self.viewport != 0).unwrap_or(int.max(1) as u32 * 120);
        if s120 != self.scale120 {
            self.scale120 = s120;
            self.shell.invalidate();
            self.shown_cursor = None;
            if self.shell.stats {
                let line = format!("scale: {}/120", s120);
                self.shell.stat(line);
            }
        }
    }

    fn pixel_size(&self) -> (i32, i32) {
        let s = self.scale();
        let (w, h) = self.decor.surface(self.size);
        ((w as f32 * s).round().max(1.0) as i32, (h as f32 * s).round().max(1.0) as i32)
    }

    /// App content on the buffer (pixels) and frame strips around it, without overlaps.
    fn layout_px(&self, pw: i32, ph: i32) -> (Px, Vec<Px>) {
        let s = self.scale();
        let o = self.decor.offset();
        let (cx, cy) = ((o.x * s).round() as i32, (o.y * s).round() as i32);
        let cx1 = (((o.x + self.size.0 as f32) * s).round() as i32).min(pw);
        let cy1 = (((o.y + self.size.1 as f32) * s).round() as i32).min(ph);
        let content = Px { x: cx, y: cy, w: (cx1 - cx).max(0), h: (cy1 - cy).max(0) };
        let bands = [
            Px { x: 0, y: 0, w: pw, h: cy },
            Px { x: 0, y: cy, w: cx, h: content.h },
            Px { x: cx1, y: cy, w: pw - cx1, h: content.h },
            Px { x: 0, y: cy1, w: pw, h: ph - cy1 },
        ];
        (content, bands.into_iter().filter(|b| b.w > 0 && b.h > 0).collect())
    }

    fn create_buffer(&mut self, w: i32, h: i32, argb: bool) -> Option<usize> {
        let stride = w * 4;
        let len = (stride * h) as usize;
        let file = memfd(len).map_err(|e| log!("uison: memfd: {e}")).ok()?;
        let map = Map::shared(file.as_raw_fd(), len).map_err(|e| log!("uison: mmap: {e}")).ok()?;
        let pool = self.new_obj(Kind::Other);
        self.c.request(self.g.shm, 0, &[Arg::New(pool), Arg::Fd(file.as_raw_fd()), Arg::Int(len as i32)]); // create_pool
        let slot = self.bufs.iter().position(Option::is_none).unwrap_or_else(|| {
            self.bufs.push(None);
            self.bufs.len() - 1
        });
        let id = self.new_obj(Kind::Buffer(slot));
        // XRGB8888 is opaque: the compositor need not blend the window with the background;
        // ARGB8888 has our frame's shadow.
        let format = if argb { 0 } else { 1 };
        self.c.request(pool, 0, &[Arg::New(id), Arg::Int(0), Arg::Int(w), Arg::Int(h), Arg::Int(stride), Arg::Uint(format)]);
        self.c.request(pool, 1, &[]); // wl_shm_pool.destroy: the memory lives as long as the buffer
        // The descriptor is taken when sending: the file must stay open until then.
        let _ = self.c.flush();
        let image = unsafe { Image::over(map.ptr, w, h, stride) };
        self.bufs[slot] = Some(Buf { id, _file: file, _map: map, image, w, h, argb, busy: false, stale: None });
        Some(slot)
    }

    fn drop_buffer(&mut self, i: usize) {
        if let Some(b) = self.bufs[i].take() {
            self.c.request(b.id, 0, &[]); // wl_buffer.destroy
            if self.last_buf == Some(i) {
                self.last_buf = None;
            }
        }
    }

    fn draw(&mut self) {
        self.force_draw = false;
        let (pw, ph) = self.pixel_size();
        let argb = self.decor.margin() > 0.0;
        // Buffers of the old size or format are no longer needed.
        for i in 0..self.bufs.len() {
            if self.bufs[i].as_ref().is_some_and(|b| b.w != pw || b.h != ph || b.argb != argb) {
                self.drop_buffer(i);
            }
        }
        let free = self.bufs.iter().position(|b| b.as_ref().is_some_and(|b| !b.busy));
        let live = self.bufs.iter().filter(|b| b.is_some()).count();
        let i = match free {
            Some(i) => i,
            // Two buffers in turn: while the compositor holds one, we draw into the other.
            None if live < 2 => match self.create_buffer(pw, ph, argb) {
                Some(i) => i,
                None => return,
            },
            // Both busy: finish drawing when the compositor releases one (`release`).
            None => return,
        };
        let s = self.scale();
        let (content, bands) = self.layout_px(pw, ph);
        // New changes, in buffer pixels.
        let new: Option<Vec<Px>> = self.shell.take_damage().map(|rs| {
            rs.iter()
                .filter_map(|r| {
                    let x0 = content.x + (r.x * s).floor() as i32;
                    let y0 = content.y + (r.y * s).floor() as i32;
                    let x1 = content.x + (r.right() * s).ceil() as i32;
                    let y1 = content.y + (r.bottom() * s).ceil() as i32;
                    Px { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }.intersect(&content)
                })
                .collect()
        });
        let new = match (new, std::mem::take(&mut self.decor_dirty)) {
            (Some(mut n), true) => {
                n.extend_from_slice(&bands);
                Some(n)
            }
            (n, _) => n,
        };
        for b in self.bufs.iter_mut().flatten() {
            match (&mut b.stale, &new) {
                (Some(st), Some(n)) => st.extend_from_slice(n),
                (st, None) => *st = None,
                _ => {}
            }
        }
        let full = Px { x: 0, y: 0, w: pw, h: ph };
        let b = self.bufs[i].as_mut().unwrap();
        let rects = b.stale.take().unwrap_or_else(|| vec![full]);
        b.stale = Some(Vec::new());
        b.busy = true;
        let (id, cr) = (b.id, b.image.cr);

        // Frame: parts of the changed areas that fall on its strips.
        if self.decor.on {
            let pt = |p: &Px| Rect::new(p.x as f32 / s, p.y as f32 / s, p.w as f32 / s, p.h as f32 / s);
            for band in &bands {
                for r in &rects {
                    if let Some(c) = r.intersect(band) {
                        self.decor.paint(cr, s, pt(&c), self.size, self.shell.dark, &self.shell.text);
                    }
                }
            }
        }
        // App: parts inside the content, in frame points.
        let app: Vec<Rect> = rects
            .iter()
            .filter_map(|r| r.intersect(&content))
            .map(|p| Rect::new((p.x - content.x) as f32 / s, (p.y - content.y) as f32 / s, p.w as f32 / s, p.h as f32 / s))
            .collect();
        if !app.is_empty() {
            self.shell.paint(cr, s, self.decor.offset(), &app);
        }
        self.bufs[i].as_ref().unwrap().image.flush();

        // Buffer scale: fractional via viewport (buffer in pixels, surface in points),
        // integer via set_buffer_scale.
        let surf = self.decor.surface(self.size);
        if self.viewport != 0 {
            if self.sent_viewport != surf {
                self.sent_viewport = surf;
                self.c.request(self.viewport, 2, &[Arg::Int(surf.0), Arg::Int(surf.1)]); // set_destination
            }
        } else {
            let k = (self.scale120 / 120) as i32;
            if k != self.sent_buffer_scale && self.g.compositor_v >= 3 {
                self.sent_buffer_scale = k;
                self.c.request(self.surface, 8, &[Arg::Int(k)]); // set_buffer_scale
            }
        }
        // Window geometry without our frame's shadow (the compositor aligns and snaps to edges by it).
        let geo = self.decor.geometry(self.size);
        if geo != self.sent_geometry {
            self.sent_geometry = geo;
            self.c.request(self.xdg_surface, 3, &[Arg::Int(geo.0), Arg::Int(geo.1), Arg::Int(geo.2), Arg::Int(geo.3)]);
        }
        self.c.request(self.surface, 1, &[Arg::Obj(id), Arg::Int(0), Arg::Int(0)]); // attach
        for p in new.as_deref().unwrap_or(&[full]) {
            if self.g.compositor_v >= 4 {
                self.c.request(self.surface, 9, &[Arg::Int(p.x), Arg::Int(p.y), Arg::Int(p.w), Arg::Int(p.h)]); // damage_buffer
            } else {
                // Old compositors: damage in surface coordinates.
                let k = self.sent_buffer_scale.max(1);
                self.c.request(self.surface, 2, &[Arg::Int(p.x / k), Arg::Int(p.y / k), Arg::Int(p.w / k + 2), Arg::Int(p.h / k + 2)]);
            }
        }
        self.request_frame();
        self.c.request(self.surface, 6, &[]); // commit
        self.last_buf = Some(i);
        self.shell.presented();
    }

    fn setting(&mut self, s: Setting) {
        match s {
            Setting::OpenFailed(uri) => {
                xdg_open(&uri);
                return;
            }
            Setting::Dark(dark) => {
                if self.shell.set_dark(dark) {
                    self.decor_dirty = true;
                }
                return;
            }
            Setting::Font(f) => {
                if self.shell.set_font(&f) {
                    self.decor_dirty = true;
                }
                return;
            }
            Setting::CursorTheme(t) if self.cursor_pref.0.as_ref() != Some(&t) => self.cursor_pref.0 = Some(t),
            Setting::CursorSize(n) if self.cursor_pref.1 != Some(n) => self.cursor_pref.1 = Some(n),
            _ => return,
        }
        if self.shell.stats {
            let (t, n) = &self.cursor_pref;
            self.shell.stat(format!("cursors: {} {}, {} ms", t.as_deref().unwrap_or("-"), n.unwrap_or(0), self.shell.started.elapsed().as_millis()));
        }
        // Cursor theme changed: reload it and show again.
        self.cursors = None;
        self.shown_cursor = None;
    }

    fn cursor_shape(&self) -> Shape {
        match self.decor.hover {
            Some(Part::Edge(e)) => Shape::Edge(e),
            Some(_) => Shape::App(Cursor::Arrow),
            None => Shape::App(self.shell.cursor),
        }
    }

    fn update_cursor(&mut self) {
        if self.pointer == 0 || self.pointer_serial == 0 {
            return;
        }
        let shape = self.cursor_shape();
        if self.shown_cursor == Some(shape) {
            return;
        }
        self.shown_cursor = Some(shape);
        if self.cursor_dev != 0 {
            self.c.request(self.cursor_dev, 1, &[Arg::Uint(self.pointer_serial), Arg::Uint(shape.wp_shape())]); // set_shape
            return;
        }
        // Without cursor-shape: an image from the Xcursor theme on our own surface.
        let scale = self.scale120.div_ceil(120) as i32;
        let (name, size) = &self.cursor_pref;
        let theme = self.cursors.get_or_insert_with(|| CursorTheme::load(name.as_deref(), *size));
        let Some(img) = theme.image(names(shape), scale) else { return };
        let (w, h, hx, hy, k) = (img.w, img.h, img.hot_x, img.hot_y, img.scale);
        let pixels = img.pixels.clone();
        if self.cursor_surface == 0 {
            self.cursor_surface = self.new_obj(Kind::CursorSurface);
            self.c.request(self.g.compositor, 0, &[Arg::New(self.cursor_surface)]);
        }
        let stride = w * 4;
        let len = (stride * h) as usize;
        let Ok(file) = memfd(len) else { return };
        let Ok(map) = Map::shared(file.as_raw_fd(), len) else { return };
        unsafe { std::ptr::copy_nonoverlapping(pixels.as_ptr().cast::<u8>(), map.ptr, len) };
        let pool = self.new_obj(Kind::Other);
        self.c.request(self.g.shm, 0, &[Arg::New(pool), Arg::Fd(file.as_raw_fd()), Arg::Int(len as i32)]);
        let buf = self.new_obj(Kind::Other);
        self.c.request(pool, 0, &[Arg::New(buf), Arg::Int(0), Arg::Int(w), Arg::Int(h), Arg::Int(stride), Arg::Uint(0)]); // ARGB8888
        self.c.request(pool, 1, &[]);
        let _ = self.c.flush();
        let cs = self.cursor_surface;
        if self.g.compositor_v >= 3 {
            self.c.request(cs, 8, &[Arg::Int(k)]); // set_buffer_scale
        }
        self.c.request(cs, 1, &[Arg::Obj(buf), Arg::Int(0), Arg::Int(0)]);
        self.c.request(cs, 2, &[Arg::Int(0), Arg::Int(0), Arg::Int(w), Arg::Int(h)]);
        self.c.request(cs, 6, &[]);
        self.c.request(self.pointer, 0, &[Arg::Uint(self.pointer_serial), Arg::Obj(cs), Arg::Int(hx / k), Arg::Int(hy / k)]); // set_cursor
        // The compositor has already taken the buffer; destroy our reference (memory lives until release).
        self.c.request(buf, 0, &[]);
    }

    // ------------------------------------------------------------ events

    fn handle(&mut self, m: Msg) {
        let Some(&kind) = self.objs.get(&m.obj) else { return };
        if self.debug {
            log!("wl: {kind:?}#{} event {} ({} B)", m.obj, m.op, m.body.len());
        }
        let mut b = Body::new(&m.body);
        match (kind, m.op) {
            (Kind::Display, 0) => {
                let (obj, code, msg) = (b.u32(), b.u32(), b.str());
                fail(&format!("Wayland protocol error: object {obj} ({:?}), code {code}: {msg}", self.objs.get(&obj)));
            }
            (Kind::Display, 1) => {
                let id = b.u32();
                self.objs.remove(&id);
            }
            (Kind::Registry, 0) => {
                let (name, iface, version) = (b.u32(), b.str().to_string(), b.u32());
                self.global(m.obj, name, &iface, version);
            }
            (Kind::Registry, 1) => {
                let name = b.u32();
                self.outputs.retain(|o| o.name != name);
            }
            (Kind::SyncCallback, 0) => {
                self.objs.remove(&m.obj);
            }
            (Kind::FrameCallback, 0) => {
                self.objs.remove(&m.obj);
                if m.obj == self.frame_cb {
                    self.frame_done();
                }
            }
            (Kind::Buffer(i), 0) => {
                if let Some(b) = self.bufs.get_mut(i).and_then(Option::as_mut) {
                    b.busy = false;
                }
            }
            (Kind::Surface, 0 | 1) => {
                let out = b.u32();
                if m.op == 0 {
                    self.entered.push(out);
                } else {
                    self.entered.retain(|&o| o != out);
                }
                self.update_scale();
            }
            (Kind::Surface, 2) => {
                self.pref_scale = Some(b.i32());
                self.update_scale();
            }
            (Kind::Seat, 0) => self.seat_caps(b.u32()),
            (Kind::Pointer, _) => self.pointer_event(m.op, &mut b),
            (Kind::Keyboard, _) => self.keyboard_event(m.op, &mut b),
            (Kind::Touch, _) => self.touch_event(m.op, &mut b),
            (Kind::Output(id), 3) => {
                let s = b.i32();
                if let Some(o) = self.outputs.iter_mut().find(|o| o.id == id) {
                    o.scale = s;
                }
            }
            (Kind::Output(_), 2) => self.update_scale(),
            (Kind::WmBase, 0) => {
                let serial = b.u32();
                self.c.request(self.g.wm_base, 3, &[Arg::Uint(serial)]); // pong
            }
            (Kind::XdgSurface, 0) => self.configure(b.u32()),
            (Kind::Toplevel, 0) => {
                self.pending_cfg = (b.i32(), b.i32());
                let states: Vec<u32> = b.array().as_chunks::<4>().0.iter().map(|c| u32::from_ne_bytes(*c)).collect();
                self.set_states(&states);
            }
            (Kind::Toplevel, 1) => self.quit = true,
            (Kind::Decoration, 0) => {
                let csd = b.u32() == 1;
                if csd != self.decor.on {
                    self.decor.on = csd;
                    self.shell.invalidate();
                }
                if self.shell.stats {
                    let line = format!("decorations: {}", if csd { "client" } else { "server" });
                    self.shell.stat(line);
                }
            }
            (Kind::Fractional, 0) => {
                self.frac120 = Some(b.u32());
                self.update_scale();
            }
            (Kind::DataDevice, 0) => {
                // data_offer: a new object from the compositor; its MIME types (offer) follow.
                let id = b.u32();
                self.objs.insert(id, Kind::DataOffer);
                self.offers.insert(id, Vec::new());
            }
            (Kind::DataDevice, 1) => {
                // enter (drag over the window): serial, surface, x, y, offer.
                for _ in 0..4 {
                    b.u32();
                }
                let old = std::mem::replace(&mut self.dnd_offer, b.u32());
                self.drop_offer(old);
            }
            (Kind::DataDevice, 2) => {
                let old = std::mem::take(&mut self.dnd_offer);
                self.drop_offer(old);
            }
            (Kind::DataDevice, 5) => {
                // selection: new clipboard contents (0: empty).
                let id = b.u32();
                if id != self.selection {
                    let old = std::mem::replace(&mut self.selection, id);
                    self.drop_offer(old);
                }
            }
            (Kind::DataOffer, 0) => {
                let mime = b.str().to_string();
                if let Some(v) = self.offers.get_mut(&m.obj) {
                    v.push(mime);
                }
            }
            (Kind::DataSource, 1) => {
                // send: another program (or we ourselves) reads our clipboard; write the text into its pipe.
                let _mime = b.str();
                if let Some(fd) = self.c.take_fd()
                    && let Some((src, text)) = &self.clip
                    && *src == m.obj
                {
                    // Field text is almost always smaller than the pipe capacity (64 KB): the write doesn't wait for a reader.
                    if let Err(e) = File::from(fd).write_all(text.as_bytes()) {
                        log!("uison: clipboard: {e}");
                    }
                }
            }
            (Kind::DataSource, 2) => {
                // cancelled: another program took the clipboard.
                self.c.request(m.obj, 1, &[]); // destroy
                self.objs.remove(&m.obj);
                if self.clip.as_ref().is_some_and(|c| c.0 == m.obj) {
                    self.clip = None;
                }
            }
            _ => {}
        }
    }

    fn drop_offer(&mut self, id: u32) {
        if id != 0 && self.offers.remove(&id).is_some() {
            self.objs.remove(&id);
            self.c.request(id, 2, &[]); // wl_data_offer.destroy
        }
    }

    // ------------------------------------------------------------ clipboard, links

    /// App requests to the system.
    fn asks(&mut self) {
        for a in self.shell.take_asks() {
            match a {
                Ask::Copy(text) => self.copy(text),
                Ask::Paste => self.request_paste(),
                Ask::OpenUrl(u) => match self.portal.as_mut() {
                    Some(p) => p.open_uri(&u),
                    None => xdg_open(&u),
                },
            }
        }
    }

    fn copy(&mut self, text: String) {
        if self.data_device == 0 {
            return;
        }
        let src = self.new_obj(Kind::DataSource);
        self.c.request(self.g.data_mgr, 0, &[Arg::New(src)]); // create_data_source
        for mime in TEXT_MIMES {
            self.c.request(src, 0, &[Arg::Str(mime)]); // offer
        }
        // Serial of the last press: the compositor gives the clipboard only to the window being used.
        self.c.request(self.data_device, 1, &[Arg::Obj(src), Arg::Uint(self.input_serial)]); // set_selection
        self.clip = Some((src, text));
    }

    fn request_paste(&mut self) {
        // The clipboard is ours: no exchange through the compositor.
        if let Some((_, t)) = &self.clip {
            let t = t.clone();
            self.shell.dispatch(Event::Paste(t.into()));
            return;
        }
        let Some(mimes) = self.offers.get(&self.selection) else { return };
        let Some(mime) = TEXT_MIMES.into_iter().find(|m| mimes.iter().any(|x| x == m)) else { return };
        let (r, w) = match pipe() {
            Ok(p) => p,
            Err(e) => return log!("uison: pipe: {e}"),
        };
        self.c.request(self.selection, 1, &[Arg::Str(mime), Arg::Fd(w.as_raw_fd())]); // receive
        // The descriptor is taken when sending; close our write end, otherwise the data never ends.
        let _ = self.c.flush();
        drop(w);
        self.paste = Some((File::from(r), Vec::new()));
    }

    fn read_paste(&mut self) {
        let Some((f, buf)) = self.paste.as_mut() else { return };
        let mut chunk = [0u8; 16384];
        loop {
            match f.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => buf.extend_from_slice(&chunk[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return,
                Err(e) => {
                    log!("uison: paste: {e}");
                    self.paste = None;
                    return;
                }
            }
        }
        let (_, buf) = self.paste.take().unwrap();
        self.shell.dispatch(Event::Paste(String::from_utf8_lossy(&buf).into()));
    }

    fn global(&mut self, reg: u32, name: u32, iface: &str, version: u32) {
        let bind = |s: &mut Wl, kind: Kind, v: u32| -> u32 {
            let id = s.new_obj(kind);
            s.c.request(reg, 0, &[Arg::Uint(name), Arg::NewAny(iface, v.min(version), id)]);
            id
        };
        match iface {
            "wl_compositor" => {
                self.g.compositor = bind(self, Kind::Compositor, V_COMPOSITOR);
                self.g.compositor_v = V_COMPOSITOR.min(version);
            }
            "wl_shm" => self.g.shm = bind(self, Kind::Shm, 1),
            "wl_data_device_manager" => self.g.data_mgr = bind(self, Kind::Other, 3),
            "wl_seat" if self.g.seat == 0 => {
                self.g.seat = bind(self, Kind::Seat, V_SEAT);
                self.g.seat_v = V_SEAT.min(version);
            }
            "wl_output" => {
                let id = self.c.new_id();
                self.objs.insert(id, Kind::Output(id));
                self.c.request(reg, 0, &[Arg::Uint(name), Arg::NewAny(iface, V_OUTPUT.min(version), id)]);
                self.outputs.push(Output { id, name, scale: 1 });
            }
            "xdg_wm_base" => self.g.wm_base = bind(self, Kind::WmBase, V_WM_BASE),
            // UISON_CSD=1: our own frame even where the compositor draws one (for testing).
            "zxdg_decoration_manager_v1" if !env_flag("UISON_CSD") => self.g.deco_mgr = bind(self, Kind::Other, 1),
            "wp_viewporter" => self.g.viewporter = bind(self, Kind::Other, 1),
            "wp_fractional_scale_manager_v1" => self.g.frac_mgr = bind(self, Kind::Other, 1),
            "wp_cursor_shape_manager_v1" if std::env::var("UISON_CURSOR").as_deref() != Ok("xcursor") => {
                self.g.cursor_mgr = bind(self, Kind::Other, 1)
            }
            _ => {}
        }
    }

    fn seat_caps(&mut self, caps: u32) {
        if self.shell.stats {
            self.shell.stat(format!("seat: pointer {}, keyboard {}, touch {}", caps & 1, caps >> 1 & 1, caps >> 2 & 1));
        }
        // The device went away (mouse unplugged, virtual keyboard closed): the old
        // object gets nothing more; release it and create a new one when it reappears.
        let release = self.g.seat_v >= 3;
        if caps & 1 == 0 && self.pointer != 0 {
            if self.cursor_dev != 0 {
                self.c.request(self.cursor_dev, 0, &[]);
                self.cursor_dev = 0;
            }
            if release {
                self.c.request(self.pointer, 1, &[]);
            }
            self.pointer = 0;
            self.pointer_serial = 0;
        }
        if caps & 2 == 0 && self.keyboard != 0 {
            if release {
                self.c.request(self.keyboard, 0, &[]);
            }
            self.keyboard = 0;
            self.repeat_key = None;
        }
        if caps & 4 == 0 && self.touch != 0 {
            if release {
                self.c.request(self.touch, 0, &[]);
            }
            self.touch = 0;
            self.touch_id = None;
        }
        if caps & 1 != 0 && self.pointer == 0 {
            self.pointer = self.new_obj(Kind::Pointer);
            self.c.request(self.g.seat, 0, &[Arg::New(self.pointer)]);
            if self.g.cursor_mgr != 0 {
                self.cursor_dev = self.new_obj(Kind::Other);
                self.c.request(self.g.cursor_mgr, 1, &[Arg::New(self.cursor_dev), Arg::Obj(self.pointer)]);
            }
        }
        if caps & 2 != 0 && self.keyboard == 0 {
            self.keyboard = self.new_obj(Kind::Keyboard);
            self.c.request(self.g.seat, 1, &[Arg::New(self.keyboard)]);
            if self.kb.is_none() {
                self.kb = Keyboard::new().map_err(|e| log!("uison: {e}")).ok();
            }
        }
        if caps & 4 != 0 && self.touch == 0 {
            self.touch = self.new_obj(Kind::Touch);
            self.c.request(self.g.seat, 2, &[Arg::New(self.touch)]);
        }
    }

    /// Window states from `xdg_toplevel.configure`: maximized, fullscreen, tiled, focused.
    fn set_states(&mut self, states: &[u32]) {
        let d = &mut self.decor;
        let old = (d.maximized, d.fullscreen, d.tiled, d.active, d.margin());
        d.maximized = states.contains(&1);
        d.fullscreen = states.contains(&2);
        d.active = states.contains(&4);
        d.tiled = states.iter().any(|s| (5..=8).contains(s));
        if (d.maximized, d.fullscreen, d.tiled, d.active, d.margin()) != old {
            self.decor_dirty = true;
            if d.margin() != old.4 {
                // The shadow appeared or disappeared: the content moved.
                self.shell.invalidate();
            }
        }
    }

    fn configure(&mut self, serial: u32) {
        let (w, h) = self.pending_cfg;
        let first = !self.configured;
        // Size from the compositor is the window geometry, including our frame's title bar.
        let size = if w > 0 && h > 0 { (w, (h - self.decor.title_h() as i32).max(1)) } else { self.size };
        self.c.request(self.xdg_surface, 4, &[Arg::Uint(serial)]); // ack_configure
        self.configured = true;
        if size != self.size {
            self.size = size;
            self.shell.dispatch(Event::Resize(Size::new(size.0 as f32, size.1 as f32)));
            self.shell.invalidate();
        }
        if first {
            self.shell.invalidate();
            if self.shell.stats {
                let line = format!("configured: {} ms, {}x{} pt", self.shell.started.elapsed().as_millis(), size.0, size.1);
                self.shell.stat(line);
            }
        }
        // After configure the compositor expects a buffer of the new size.
        self.force_draw = true;
    }

    fn app_point(&self, p: Point) -> Point {
        let o = self.decor.offset();
        Point::new(p.x - o.x, p.y - o.y)
    }

    /// Pointer over the surface: over the frame it goes to the frame, over the content to the app.
    fn pointer_moved(&mut self, p: Point) {
        self.pointer_pos = p;
        let part = if self.buttons_down > 0 { None } else { self.decor.part_at(p, self.size) };
        if part != self.decor.hover {
            let buttons = |x: Option<Part>| matches!(x, Some(Part::Minimize | Part::Maximize | Part::Close));
            if buttons(part) || buttons(self.decor.hover) {
                self.decor_dirty = true;
            }
            self.decor.hover = part;
        }
        if part.is_some() {
            if std::mem::take(&mut self.app_hover) {
                self.shell.dispatch(Event::MouseLeave);
            }
        } else {
            self.app_hover = true;
            self.shell.dispatch(Event::MouseMove(self.app_point(p)));
        }
    }

    fn toggle_maximized(&mut self) {
        let op = if self.decor.maximized { 10 } else { 9 }; // unset_maximized / set_maximized
        self.c.request(self.toplevel, op, &[]);
    }

    /// A frame button was pressed and released over it.
    fn decor_button(&mut self, part: Part) {
        match part {
            Part::Close => self.quit = true,
            Part::Maximize => self.toggle_maximized(),
            Part::Minimize => self.c.request(self.toplevel, 13, &[]), // set_minimized
            _ => {}
        }
    }

    fn pointer_event(&mut self, op: u16, b: &mut Body) {
        match op {
            0 => {
                // enter
                self.pointer_serial = b.u32();
                b.u32();
                let p = Point::new(b.fixed(), b.fixed());
                self.shown_cursor = None;
                self.pointer_moved(p);
            }
            1 => {
                // leave
                if self.decor.hover.take().is_some() {
                    self.decor_dirty = true;
                }
                if std::mem::take(&mut self.app_hover) {
                    self.shell.dispatch(Event::MouseLeave);
                }
            }
            2 => {
                b.u32();
                let p = Point::new(b.fixed(), b.fixed());
                self.pointer_moved(p);
            }
            3 => {
                let serial = b.u32();
                b.u32();
                let (button, state) = (b.u32(), b.u32());
                if state == 1 {
                    self.input_serial = serial;
                }
                let btn = match button {
                    0x110 => MouseButton::Left,
                    0x111 => MouseButton::Right,
                    _ => MouseButton::Other,
                };
                let p = self.pointer_pos;
                if state == 1 {
                    // A press stops inertia (like a touch on a phone).
                    self.kinetic.stop();
                    let part = if self.buttons_down > 0 { None } else { self.decor.part_at(p, self.size) };
                    let seat = self.g.seat;
                    match (part, btn) {
                        (Some(Part::Title), MouseButton::Left) => {
                            if self.decor.double_click() {
                                self.toggle_maximized();
                            } else {
                                self.c.request(self.toplevel, 5, &[Arg::Obj(seat), Arg::Uint(serial)]); // move
                            }
                        }
                        (Some(Part::Title), MouseButton::Right) => {
                            let m = self.decor.margin();
                            let (x, y) = ((p.x - m) as i32, (p.y - m) as i32);
                            self.c.request(self.toplevel, 4, &[Arg::Obj(seat), Arg::Uint(serial), Arg::Int(x), Arg::Int(y)]); // show_window_menu
                        }
                        (Some(Part::Edge(e)), MouseButton::Left) => {
                            self.c.request(self.toplevel, 6, &[Arg::Obj(seat), Arg::Uint(serial), Arg::Uint(e)]); // resize
                        }
                        (Some(part @ (Part::Minimize | Part::Maximize | Part::Close)), MouseButton::Left) => {
                            self.decor.pressed = Some(part);
                            self.decor_dirty = true;
                        }
                        (Some(_), _) => {}
                        (None, _) => {
                            self.buttons_down += 1;
                            self.shell.dispatch(Event::MouseDown(self.app_point(p), btn));
                        }
                    }
                } else if let Some(part) = self.decor.pressed.take() {
                    self.decor_dirty = true;
                    if self.decor.hover == Some(part) {
                        self.decor_button(part);
                    }
                } else if self.buttons_down > 0 {
                    self.buttons_down -= 1;
                    self.shell.dispatch(Event::MouseUp(self.app_point(p), btn));
                    if self.buttons_down == 0 {
                        // Released over the frame: update hover.
                        self.pointer_moved(p);
                    }
                }
            }
            4 => {
                b.u32();
                let (axis, v) = (b.u32() as usize, b.fixed());
                if axis < 2 {
                    self.axis.value[axis] += v;
                }
                if self.g.seat_v < 5 {
                    self.flush_axis();
                }
            }
            5 => self.flush_axis(),
            6 => self.axis.source = Some(b.u32()),
            7 => self.axis.stop = true,
            9 => {
                let (axis, v) = (b.u32() as usize, b.i32());
                if axis < 2 {
                    self.axis.v120[axis] += v;
                }
            }
            _ => {}
        }
    }

    /// Scrolling within a pointer frame -> `Scroll`. Wheel in clicks (`value120`), touchpad precisely.
    fn flush_axis(&mut self) {
        let a = std::mem::take(&mut self.axis);
        let wheel = a.v120 != [0, 0];
        let d = |i: usize| -> f32 {
            // A positive Wayland value scrolls down (content moves up).
            if wheel { -(a.v120[i] as f32) / 120.0 * self.wheel_step } else { -a.value[i] }
        };
        let (dx, dy) = (d(1), d(0));
        let p = self.app_point(self.pointer_pos);
        if (dx != 0.0 || dy != 0.0) && self.app_hover {
            self.shell.dispatch(Event::Scroll(p, dx, dy));
        }
        // Touchpad: finger offsets accumulate for velocity, lifting the fingers starts inertia.
        if a.source == Some(1) && !wheel {
            if dx != 0.0 || dy != 0.0 {
                self.kinetic.sample(dx, dy);
            }
            if a.stop && self.app_hover && self.kinetic.release() {
                self.kinetic_pos = p;
            }
        } else if dx != 0.0 || dy != 0.0 {
            self.kinetic.stop();
        }
    }

    fn keyboard_event(&mut self, op: u16, b: &mut Body) {
        match op {
            0 => {
                // The descriptor is not in the body: it is in the SCM_RIGHTS queue.
                let (format, size) = (b.u32(), b.u32());
                let fd = self.c.take_fd();
                if self.debug {
                    log!("wl: keymap format {format}, {size} B, fd {:?}, xkb {}", fd.as_ref().map(|f| f.as_raw_fd()), self.kb.is_some());
                }
                if let (1, Some(fd), Some(kb)) = (format, fd, self.kb.as_mut()) {
                    match Map::read_only(fd.as_raw_fd(), size as usize) {
                        Ok(map) => kb.set_keymap(map.bytes()),
                        Err(e) => log!("uison: keymap: {e}"),
                    }
                }
            }
            2 => self.repeat_key = None,
            3 => {
                self.input_serial = b.u32();
                b.u32();
                let (key, state) = (b.u32(), b.u32());
                let code = key + 8;
                if self.debug {
                    log!("wl: key {key} state {state}");
                }
                if state == 1 {
                    self.key_press(code);
                    let repeats = self.kb.as_ref().is_some_and(|k| k.repeats(code)) && self.repeat.0 > 0;
                    self.repeat_key = repeats.then(|| (code, Instant::now() + Duration::from_millis(self.repeat.1 as u64)));
                } else if self.repeat_key.is_some_and(|(c, _)| c == code) {
                    self.repeat_key = None;
                }
            }
            4 => {
                b.u32();
                let (dep, lat, lock, group) = (b.u32(), b.u32(), b.u32(), b.u32());
                if let Some(kb) = self.kb.as_mut() {
                    kb.set_mods(dep, lat, lock, group);
                    self.shell.mods = kb.mods();
                }
            }
            5 => self.repeat = (b.i32(), b.i32()),
            _ => {}
        }
    }

    fn key_press(&mut self, code: u32) {
        if let Some(key) = self.kb.as_mut().and_then(|k| k.key(code)) {
            self.shell.dispatch(Event::Key(key));
        }
    }

    fn touch_event(&mut self, op: u16, b: &mut Body) {
        match op {
            0 => {
                let serial = b.u32();
                self.input_serial = serial;
                b.u32();
                b.u32();
                let id = b.i32();
                let p = Point::new(b.fixed(), b.fixed());
                if self.touch_id.is_some() {
                    return;
                }
                self.touch_id = Some(id);
                self.touch_pos = p;
                let seat = self.g.seat;
                self.touch_target = match self.decor.part_at(p, self.size) {
                    None => {
                        self.shell.dispatch(Event::MouseDown(self.app_point(p), MouseButton::Touch));
                        TouchTarget::App
                    }
                    Some(Part::Title) => {
                        self.c.request(self.toplevel, 5, &[Arg::Obj(seat), Arg::Uint(serial)]); // move
                        TouchTarget::Window
                    }
                    Some(Part::Edge(e)) => {
                        self.c.request(self.toplevel, 6, &[Arg::Obj(seat), Arg::Uint(serial), Arg::Uint(e)]); // resize
                        TouchTarget::Window
                    }
                    Some(part) => TouchTarget::Button(part),
                };
            }
            1 => {
                b.u32();
                b.u32();
                if self.touch_id == Some(b.i32()) {
                    self.touch_id = None;
                    match self.touch_target {
                        TouchTarget::App => self.shell.dispatch(Event::MouseUp(self.app_point(self.touch_pos), MouseButton::Touch)),
                        TouchTarget::Button(part) if self.decor.part_at(self.touch_pos, self.size) == Some(part) => {
                            self.decor_button(part);
                            true
                        }
                        _ => false,
                    };
                }
            }
            2 => {
                b.u32();
                let id = b.i32();
                let p = Point::new(b.fixed(), b.fixed());
                if self.touch_id == Some(id) {
                    self.touch_pos = p;
                    if self.touch_target == TouchTarget::App {
                        self.shell.dispatch(Event::MouseMove(self.app_point(p)));
                    }
                }
            }
            // The system took the touch: release outside, no click.
            4 if self.touch_id.take().is_some() && self.touch_target == TouchTarget::App => {
                self.shell.dispatch(Event::MouseUp(Point::new(-1.0, -1.0), MouseButton::Touch));
            }
            _ => {}
        }
    }
}
