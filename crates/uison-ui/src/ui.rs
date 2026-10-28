//! Runtime: screen (complex JSON) + data + handlers → frames.
//! Implements `uison_core::App`, so it works with `uison::run` and `Headless`.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use uison_core::time::now;

use uison_core::{App, Color, Event, Font, Host, Insets, KeyCode, MouseButton, Point, Rect, Size, TextMeasure};
use uison_frame::Writer;

use crate::data::{Data, IntoJson};
use crate::edit::{Field, next_char, next_word, prev_char, word_at, word_left, word_right};
use crate::el::{Behavior, El};
use crate::emit::{MIN_TOUCH, NodeInfo, Style, at_least, emit};
use crate::expand::{expand_root, node_kind};
use crate::gesture::{FLING_TICK, Fling, SLOP, Spring, Touch, rubber, unrubber};
use crate::layout::{Cons, Ctx, layout};
use crate::library::Library;
use crate::state::UiState;
use crate::text::{WrapMode, wrap};
use crate::values::{Env, Theme, to_text};
use crate::{Error, Json, Source};

/// Time of building the last frame, by step.
#[derive(Clone, Copy, Debug, Default)]
pub struct Stats {
    pub expand: Duration,
    pub layout: Duration,
    pub emit: Duration,
    /// Nodes in the expanded tree.
    pub nodes: usize,
    /// Primitives in the frame.
    pub prims: usize,
    /// Size of the frame text.
    pub bytes: usize,
}

type Handler = Box<dyn FnMut(&mut Cx)>;
type Drawer = Box<dyn FnMut(&mut Draw)>;

/// Custom drawing of a `Canvas { draw: "name" }` node (`Ui::draw`): primitives go to `w`, in window
/// coordinates; the node area is `rect`, the drawing is clipped to it. Called on every frame build.
pub struct Draw<'a> {
    pub w: &'a mut Writer,
    pub rect: Rect,
    /// Node `key`: one function can draw several nodes.
    pub key: &'a str,
    pub data: &'a Data,
    /// Text width — for labels.
    pub m: &'a dyn TextMeasure,
}

const CARET_BLINK: Duration = Duration::from_millis(530);
/// A timer with less than this left fires on the current tick: the display frame
/// arrives slightly before the `timer(16)` deadline (counted from the end of the previous tick's processing),
/// and without the tolerance animation would run every other frame (60 Hz, 120 Hz, compositor without vsync).
const TIMER_SLACK: Duration = Duration::from_millis(3);
/// Double and triple click: no longer between presses and no farther apart (pt).
const MULTI_CLICK: Duration = Duration::from_millis(500);
const MULTI_CLICK_DIST: f32 = 4.0;
/// Tooltip — after this many ms of motionless hover.
const TOOLTIP_DELAY: Duration = Duration::from_millis(500);
/// Margin of popups from the window edges.
const POPUP_MARGIN: f32 = 4.0;
type Derive = Box<dyn FnMut(&mut Data)>;

pub struct Ui {
    lib: Library,
    theme_name: String,
    /// Tokens on top of the selected theme (from the screen file or code).
    theme_over: Json,
    theme: Theme,
    root: Json,
    pub data: Data,
    st: UiState,
    handlers: HashMap<String, Handler>,
    drawers: HashMap<String, Drawer>,
    derives: Vec<Derive>,
    size: Size,
    infos: Vec<NodeInfo>,
    /// The laid-out tree of the last frame — kept only after the first `semantics` call.
    laid: Option<Vec<El>>,
    /// Text wrapping method (experiment).
    pub wrap: WrapMode,
    pub stats: Stats,
    warned: HashSet<String>,
    /// Deadline (platform time) and action.
    timers: Vec<(Duration, String)>,
    shortcuts: Vec<(Shortcut, String)>,
    caret_toggled: Duration,
    /// Grows when theme tokens change — the frame must be rebuilt.
    theme_version: u64,
    /// Finger on the screen (between MouseDown and MouseUp with `MouseButton::Touch`).
    touch: Option<Touch>,
    /// Scroll inertia after the finger is lifted.
    fling: Option<Fling>,
    /// Return of the overscroll past the scroll edge.
    spring: Option<Spring>,
    /// Window safe area (iOS) and whether to lay out the root inside it (the screen's `safeArea`).
    safe: Insets,
    safe_area: bool,
    /// Scroll the focused field into view on the next frame build
    /// (focus by finger, the keyboard shrank the screen).
    reveal: bool,
    /// System theme is dark (for the `system` theme).
    system_dark: bool,
    /// Requests to the platform — run after the event, when a `Host` is available.
    asks: Vec<Ask>,
    /// Previous press (time, point, key) and click number in the series — for double and triple clicks.
    last_press: Option<(Duration, Point, String, u32)>,
    /// Click number in the series for the last press (`cx.clicks`).
    clicks: u32,
    /// Cursor while a mouse event is handled (`cx.pos`).
    pointer: Option<Point>,
    /// Node with `on.hover` under the cursor: it gets a hover without a point when the cursor leaves.
    hovering: Option<String>,
    tip: Option<Tip>,
    menu: Option<Menu>,
    /// Queue from other threads (`Handle::post`) and the event loop wake-up.
    shared: Arc<Shared>,
    /// Live reload (debug only): files, when they were checked, the error of the last attempt.
    #[cfg(all(any(debug_assertions, feature = "reload"), not(target_arch = "wasm32")))]
    watched: Vec<crate::reload::Watched>,
    #[cfg(all(any(debug_assertions, feature = "reload"), not(target_arch = "wasm32")))]
    reload_at: Duration,
    #[cfg(all(any(debug_assertions, feature = "reload"), not(target_arch = "wasm32")))]
    reload_error: Option<String>,
}

/// A deferred call from another thread: runs on the UI thread with a context, like a handler.
type Posted = Box<dyn FnOnce(&mut Cx) + Send>;

#[derive(Default)]
struct Shared {
    queue: Mutex<Vec<Posted>>,
    wake: Mutex<Option<uison_core::Waker>>,
}

/// Reference to `Ui` for other threads: `Send + Sync`, cloneable.
/// `post` queues a function and wakes the event loop; on the UI thread it gets a `Cx`
/// (data, focus, timers…), after it the frame is rebuilt. Before the window starts — waits for the start.
#[derive(Clone)]
pub struct Handle(Arc<Shared>);

impl Handle {
    pub fn post(&self, f: impl FnOnce(&mut Cx) + Send + 'static) {
        self.0.queue.lock().unwrap().push(Box::new(f));
        let wake = self.0.wake.lock().unwrap().clone();
        if let Some(w) = wake {
            w();
        }
    }
}

impl Drop for Ui {
    /// The event loop is gone — remove the wake-up: `post` from live threads only enqueues.
    fn drop(&mut self) {
        *self.shared.wake.lock().unwrap() = None;
    }
}

/// Tooltip: node under the cursor, text, since when and where the cursor is motionless; whether it is shown.
struct Tip {
    key: String,
    text: String,
    since: Duration,
    at: Point,
    shown: bool,
}

/// Open context menu.
struct Menu {
    /// Node it was invoked on (`cx.key` of the item handler).
    key: String,
    at: Point,
    items: Vec<MenuItem>,
    /// Item selected with the keyboard.
    sel: Option<usize>,
    /// Input field menu: items are built-in actions on the field `key`.
    field: bool,
    /// Opened with the right button here: releasing it over an item (after moving) selects it.
    press: Option<Point>,
}

#[derive(Clone)]
struct MenuItem {
    label: String,
    action: String,
    shortcut: String,
    disabled: bool,
    sep: bool,
}

impl MenuItem {
    fn usable(&self) -> bool {
        !self.sep && !self.disabled
    }
}

/// A request to the platform from a handler or the runtime.
enum Ask {
    Copy(String),
    OpenUrl(String),
    Paste,
    PickFile(String),
}

/// Action handler context.
pub struct Cx<'a> {
    pub data: &'a mut Data,
    /// Screen tree — code can change node properties and children.
    pub tree: Tree<'a>,
    /// Action name.
    pub action: &'a str,
    /// Event: `click`, `change`, `submit`.
    pub event: &'a str,
    /// `key` of the node the event happened on.
    pub key: &'a str,
    /// Cursor relative to the top-left corner of the `key` node (mouse events); for `hover` when the
    /// cursor leaves the node — `None`.
    pub pos: Option<Point>,
    /// Click number in the series: 2 — double, 3 — triple.
    pub clicks: u32,
    req: &'a mut Requests,
}

/// What the handler asked of the runtime (run after the handler).
#[derive(Default)]
struct Requests {
    focus: Option<Option<String>>,
    theme: Option<String>,
    tokens: Vec<(String, Json)>,
    timers: Vec<(Duration, String)>,
    asks: Vec<Ask>,
}

/// Shortcut: modifiers + key.
#[derive(Clone, Debug, PartialEq)]
struct Shortcut {
    mods: uison_core::Mods,
    code: KeyCode,
    /// For `KeyCode::Char` — the character in lower case.
    ch: Option<String>,
}

impl Shortcut {
    /// `"cmd+n"`, `"cmd+shift+z"`, `"cmd+1"`, `"escape"`, `"cmd+enter"`.
    fn parse(s: &str) -> Option<Shortcut> {
        let mut sc = Shortcut { mods: uison_core::Mods::default(), code: KeyCode::Char, ch: None };
        let parts: Vec<String> = s.split('+').map(|p| p.trim().to_lowercase()).collect();
        let (key, mods) = parts.split_last()?;
        for m in mods {
            match m.as_str() {
                "cmd" => sc.mods.cmd = true,
                "ctrl" => sc.mods.ctrl = true,
                "alt" => sc.mods.alt = true,
                "shift" => sc.mods.shift = true,
                _ => return None,
            }
        }
        sc.code = match key.as_str() {
            "enter" => KeyCode::Enter,
            "escape" | "esc" => KeyCode::Escape,
            "tab" => KeyCode::Tab,
            "backspace" => KeyCode::Backspace,
            "delete" => KeyCode::Delete,
            "left" => KeyCode::Left,
            "right" => KeyCode::Right,
            "up" => KeyCode::Up,
            "down" => KeyCode::Down,
            k if k.chars().count() == 1 => {
                sc.ch = Some(k.to_string());
                KeyCode::Char
            }
            _ => return None,
        };
        Some(sc)
    }

    fn matches(&self, k: &uison_core::Key) -> bool {
        let m = k.mods;
        if (m.cmd, m.ctrl, m.alt, m.shift) != (self.mods.cmd, self.mods.ctrl, self.mods.alt, self.mods.shift) {
            return false;
        }
        match &self.ch {
            Some(c) => k.code == KeyCode::Char && k.text.as_deref().is_some_and(|t| t.to_lowercase() == *c),
            None => k.code == self.code,
        }
    }
}

impl Cx<'_> {
    /// Move focus to a node (input field) by `key`.
    pub fn focus(&mut self, key: &str) {
        self.req.focus = Some(Some(key.to_string()));
    }

    pub fn blur(&mut self) {
        self.req.focus = Some(None);
    }

    /// Switch the theme (`light`, `dark` or a custom one).
    pub fn set_theme(&mut self, name: &str) {
        self.req.theme = Some(name.to_string());
    }

    /// Replace a theme token on the fly (accent color, font size).
    pub fn set_token(&mut self, name: &str, value: impl IntoJson) {
        self.req.tokens.push((name.to_string(), value.into_json()));
    }

    /// Call an action after `ms` milliseconds (notifications, frame-by-frame animation).
    pub fn timer(&mut self, ms: u64, action: &str) {
        self.req.timers.push((Duration::from_millis(ms), action.to_string()));
    }

    /// Put text into the system clipboard.
    pub fn copy(&mut self, text: &str) {
        self.req.asks.push(Ask::Copy(text.to_string()));
    }

    /// Open a link in the browser (or in the program the system picks for the scheme).
    pub fn open_url(&mut self, url: &str) {
        self.req.asks.push(Ask::OpenUrl(url.to_string()));
    }

    /// The system's file chooser: the chosen file's own copy comes to `action` as `cx.key` (a path; empty —
    /// cancelled). Where there is no chooser (a computer), `action` runs at once with the event `unsupported`.
    pub fn pick_file(&mut self, action: &str) {
        self.req.asks.push(Ask::PickFile(action.to_string()));
    }
}

/// Mutable access to the screen tree.
pub struct Tree<'a>(&'a mut Json);

impl<'a> Tree<'a> {
    pub fn root(&mut self) -> &mut Json {
        self.0
    }

    /// Properties of the node with `key` (searched in the whole tree).
    pub fn find(&mut self, key: &str) -> Option<NodeMut<'_>> {
        find_mut(self.0, key).map(NodeMut)
    }
}

/// Node properties: `{ Text: { <these properties> } }`.
pub struct NodeMut<'a>(&'a mut Json);

impl NodeMut<'_> {
    pub fn get(&self, prop: &str) -> Option<&Json> {
        self.0.get(prop)
    }

    pub fn set(&mut self, prop: &str, v: impl IntoJson) {
        self.0.set(prop, v.into_json());
    }

    pub fn remove(&mut self, prop: &str) -> Option<Json> {
        self.0.remove(prop)
    }

    /// Children (`children`), created if needed.
    pub fn children(&mut self) -> &mut Vec<Json> {
        if self.0.get("children").and_then(Json::as_array).is_none() {
            self.0.set("children", Json::Array(Vec::new()));
        }
        self.0.get_mut("children").and_then(Json::as_array_mut).unwrap()
    }
}

fn find_mut<'a>(v: &'a mut Json, key: &str) -> Option<&'a mut Json> {
    if node_kind(v).is_some_and(|(_, p)| p.get("key").and_then(Json::as_str) == Some(key)) {
        return v.as_object_mut().map(|o| &mut o[0].1);
    }
    match v {
        Json::Array(a) => a.iter_mut().find_map(|x| find_mut(x, key)),
        Json::Object(o) => o.iter_mut().find_map(|(_, x)| find_mut(x, key)),
        _ => None,
    }
}

impl Ui {
    fn empty(root: Json) -> Ui {
        let mut ui = Ui {
            lib: Library::standard(),
            theme_name: "system".into(),
            theme_over: Json::object(),
            theme: Theme::new(),
            root,
            data: Data::default(),
            st: UiState { caret_on: true, ..UiState::default() },
            handlers: HashMap::new(),
            drawers: HashMap::new(),
            derives: Vec::new(),
            size: Size::new(800.0, 600.0),
            infos: Vec::new(),
            laid: None,
            wrap: WrapMode::WordSum,
            stats: Stats::default(),
            warned: HashSet::new(),
            timers: Vec::new(),
            shortcuts: Vec::new(),
            caret_toggled: now(),
            theme_version: 0,
            touch: None,
            fling: None,
            spring: None,
            safe: Insets::default(),
            safe_area: true,
            reveal: false,
            system_dark: false,
            asks: Vec::new(),
            last_press: None,
            clicks: 1,
            pointer: None,
            hovering: None,
            tip: None,
            menu: None,
            shared: Arc::default(),
            #[cfg(all(any(debug_assertions, feature = "reload"), not(target_arch = "wasm32")))]
            watched: Vec::new(),
            #[cfg(all(any(debug_assertions, feature = "reload"), not(target_arch = "wasm32")))]
            reload_at: Duration::ZERO,
            #[cfg(all(any(debug_assertions, feature = "reload"), not(target_arch = "wasm32")))]
            reload_error: None,
        };
        ui.merge_theme();
        ui.set_env();
        ui
    }

    /// Screen from complex JSON: `{ size, theme, components, data, root }`.
    pub fn new(src: &str) -> Result<Ui, Error> {
        let doc = uison_json::parse(src)?.into_owned();
        let fields = doc.as_object().ok_or_else(|| Error::new("screen must be an object"))?;
        let mut ui = Ui::empty(Json::Null);
        for (k, v) in fields {
            match &**k {
                "size" => {
                    let s = v.as_array().filter(|a| a.len() == 2).and_then(|a| Some(Size::new(a[0].as_f64()? as f32, a[1].as_f64()? as f32)));
                    ui.size = s.ok_or_else(|| Error::new("size must be [width, height]"))?;
                }
                "theme" => match v {
                    Json::String(name) => ui.set_theme(name),
                    Json::Object(_) => ui.theme_over = v.clone(),
                    _ => return Err(Error::new("theme must be a theme name or an object of tokens")),
                },
                "components" => ui.lib.add_components(v)?,
                "data" => ui.data = Data::new(v.clone()),
                "safeArea" => ui.safe_area = v.truthy(),
                "root" => ui.root = v.clone(),
                _ => return Err(Error::new(format!("unknown screen key '{k}' (expected size, theme, components, data, safeArea, root)"))),
            }
        }
        if ui.root.is_null() {
            return Err(Error::new("screen needs 'root'"));
        }
        ui.merge_theme();
        ui.set_env();
        Ok(ui)
    }

    /// Screen from a file (`source!("../app.json")`). With `UISON_RELOAD=folder` (debug build or the `reload` feature) the file
    /// is read from the folder, and when it is saved the screen is rebuilt without a restart.
    pub fn load(src: Source) -> Result<Ui, Error> {
        #[cfg(all(any(debug_assertions, feature = "reload"), not(target_arch = "wasm32")))]
        {
            let mut ui = Ui::new(&crate::reload::text(&src))?;
            ui.watch(src, crate::reload::Role::Screen);
            Ok(ui)
        }
        #[cfg(not(all(any(debug_assertions, feature = "reload"), not(target_arch = "wasm32"))))]
        Ui::new(src.text)
    }

    /// Components from a file (`source!("../components.json")`); reloaded like the screen.
    pub fn load_components(&mut self, src: Source) -> Result<&mut Ui, Error> {
        #[cfg(all(any(debug_assertions, feature = "reload"), not(target_arch = "wasm32")))]
        {
            self.add_components(&crate::reload::text(&src))?;
            self.watch(src, crate::reload::Role::Components);
            Ok(self)
        }
        #[cfg(not(all(any(debug_assertions, feature = "reload"), not(target_arch = "wasm32"))))]
        self.add_components(src.text)
    }

    #[cfg(all(any(debug_assertions, feature = "reload"), not(target_arch = "wasm32")))]
    fn watch(&mut self, src: Source, role: crate::reload::Role) {
        if let Some(path) = crate::reload::path(&src) {
            let modified = crate::reload::modified(&path);
            self.watched.push(crate::reload::Watched { role, path, modified });
        }
    }

    /// Files changed — rebuild the screen, keeping data and state; `true` — the frame is different.
    #[cfg(all(any(debug_assertions, feature = "reload"), not(target_arch = "wasm32")))]
    fn check_reload(&mut self, t: Duration) -> bool {
        use crate::reload::{POLL, Role, modified};
        if self.watched.is_empty() || t < self.reload_at + POLL {
            return false;
        }
        self.reload_at = t;
        let mut changed = false;
        for w in &mut self.watched {
            let m = modified(&w.path);
            changed |= m != w.modified;
            w.modified = m;
        }
        if !changed {
            return false;
        }
        let built = (|| -> Result<Ui, Error> {
            let read = |w: &crate::reload::Watched| {
                std::fs::read_to_string(&w.path).map_err(|e| Error::new(format!("{}: {e}", w.path.display())))
            };
            let screen = self.watched.iter().find(|w| w.role == Role::Screen);
            let mut fresh = match screen {
                Some(w) => Ui::new(&read(w)?).map_err(|e| Error::new(format!("{}: {e}", w.path.display())))?,
                None => Ui::from_node(self.root.clone()),
            };
            for w in self.watched.iter().filter(|w| w.role == Role::Components) {
                fresh.add_components(&read(w)?).map_err(|e| Error::new(format!("{}: {e}", w.path.display())))?;
            }
            Ok(fresh)
        })();
        match built {
            Ok(mut fresh) => {
                // New description, old data and state; new data keys come from the file.
                // (`Ui` has `Drop` — fields are swapped, not moved.)
                std::mem::swap(&mut self.lib, &mut fresh.lib);
                std::mem::swap(&mut self.root, &mut fresh.root);
                std::mem::swap(&mut self.theme_over, &mut fresh.theme_over);
                self.safe_area = fresh.safe_area;
                if let Some(fields) = fresh.data.root().as_object() {
                    for (k, v) in fields {
                        if self.data.get(k).is_none() {
                            self.data.set(k, v.clone());
                        }
                    }
                }
                self.merge_theme();
                self.theme_version += 1;
                self.run_derives();
                self.reload_error = None;
                eprintln!("uison-ui: reloaded in {} ms", now().saturating_sub(t).as_millis());
            }
            Err(e) => {
                eprintln!("uison-ui: reload failed: {e}");
                self.reload_error = Some(e.to_string());
            }
        }
        true
    }
    pub fn from_node(root: impl IntoJson) -> Ui {
        let mut ui = Ui::empty(root.into_json());
        ui.adopt_handlers();
        ui
    }

    /// Closures from the DSL (`.on_click(|cx| …)`) — into handlers under their names `~hN`.
    fn adopt_handlers(&mut self) {
        for (name, mut f) in crate::dsl::take_handlers() {
            self.handlers.insert(name, Box::new(move |cx: &mut Cx| f(cx)));
        }
    }

    pub fn with_size(mut self, w: f32, h: f32) -> Ui {
        self.size = Size::new(w, h);
        self.set_env();
        self
    }

    /// Environment for templates: `@env.width`, `@env.height`, `@env.platform`
    /// (`macos`, `ios`, `android`, `web`, `windows`, `linux`), `@env.touch` (the last press was by finger),
    /// `@env.safe.top|right|bottom|left`, `@env.dark` (system theme), `@env.mod` ("⌘" / "Ctrl+"),
    /// `@env.keyboard` (a finger focused a field — the on-screen keyboard is open).
    fn set_env(&mut self) {
        let platform = if cfg!(target_os = "ios") {
            "ios"
        } else if cfg!(target_os = "android") {
            "android"
        } else if cfg!(target_arch = "wasm32") {
            "web"
        } else if cfg!(windows) {
            "windows"
        } else if cfg!(target_os = "linux") {
            "linux"
        } else {
            "macos"
        };
        let touch = self.data.get("env.touch").map_or(cfg!(any(target_os = "ios", target_os = "android")), Json::truthy);
        // Main shortcut modifier for hints: "⌘1" on Apple, "Ctrl+1" on Windows,
        // Linux and Android. The browser doesn't know the OS in advance — "⌘" until the backend sets its own.
        let modkey = match self.data.get("env.mod").and_then(Json::as_str) {
            Some(m) => m.to_string(),
            None if cfg!(any(windows, target_os = "linux", target_os = "android")) => "Ctrl+".to_string(),
            None => "⌘".to_string(),
        };
        let s = self.safe;
        let d = &mut self.data;
        d.set("env.width", self.size.w as f64);
        d.set("env.height", self.size.h as f64);
        d.set("env.platform", platform);
        d.set("env.touch", touch);
        d.set("env.mod", modkey);
        d.set("env.dark", self.system_dark);
        for (k, v) in [("top", s.top), ("right", s.right), ("bottom", s.bottom), ("left", s.left)] {
            d.set(&format!("env.safe.{k}"), v as f64);
        }
    }

    pub fn size(&self) -> Size {
        self.size
    }

    /// Action handler (name from `on: { click: "name" }`).
    pub fn on(&mut self, action: &str, f: impl FnMut(&mut Cx) + 'static) -> &mut Ui {
        self.handlers.insert(action.to_string(), Box::new(f));
        self
    }

    /// Drawing function for `Canvas { draw: name }` nodes: charts, diagrams, custom elements.
    /// State — in the closure (`Rc<RefCell<…>>` together with the handlers) or in `d.data`.
    pub fn draw(&mut self, name: &str, f: impl FnMut(&mut Draw) + 'static) -> &mut Ui {
        self.drawers.insert(name.to_string(), Box::new(f));
        self
    }

    /// Recompute derived data after any model change (counters,
    /// filters). Called right away and after every change.
    pub fn derive(&mut self, mut f: impl FnMut(&mut Data) + 'static) -> &mut Ui {
        f(&mut self.data);
        self.derives.push(Box::new(f));
        self
    }

    /// Shortcut → action: `ui.shortcut("cmd+n", "newTask")`.
    pub fn shortcut(&mut self, combo: &str, action: &str) -> &mut Ui {
        let sc = Shortcut::parse(combo).unwrap_or_else(|| panic!("bad shortcut '{combo}'"));
        self.shortcuts.push((sc, action.to_string()));
        self
    }

    /// Add templates from a JSON object `{ Name: { params, interactive, body } }`.
    pub fn add_components(&mut self, src: &str) -> Result<&mut Ui, Error> {
        let defs = uison_json::parse(src)?.into_owned();
        self.lib.add_components(&defs)?;
        Ok(self)
    }

    /// Image from bytes (PNG, JPEG) for `Image { src: "mem:name" }` and `Avatar { src }`;
    /// returns that source. New bytes — under a new name (backends cache by source).
    pub fn image(&mut self, name: &str, bytes: impl Into<std::sync::Arc<[u8]>>) -> String {
        uison_core::image::put(name, bytes);
        format!("mem:{name}")
    }

    /// Change data from code outside a handler and recompute derived data (`derive`).
    pub fn update(&mut self, f: impl FnOnce(&mut Data)) -> &mut Ui {
        f(&mut self.data);
        self.run_derives();
        self
    }

    /// Reference for other threads: `ui.handle().post(|cx| cx.data.set("progress", 0.5))`.
    pub fn handle(&self) -> Handle {
        Handle(self.shared.clone())
    }

    /// Run what was deferred from other threads; `true` — something ran.
    fn run_posted(&mut self) -> bool {
        let posted = std::mem::take(&mut *self.shared.queue.lock().unwrap());
        let any = !posted.is_empty();
        for f in posted {
            self.with_cx("~post", "post", "", None, 1, f);
        }
        any
    }

    /// Call an action from code (as if an element had called it).
    pub fn run_action(&mut self, action: &str) {
        let version = self.data.version();
        self.call(action, "code", "");
        // As after an event: data changed — recompute derived data.
        if self.data.version() != version {
            self.run_derives();
        }
    }

    /// Add a template from JSON `{ params, interactive, body }`.
    pub fn component(&mut self, name: &str, def: &str) -> Result<&mut Ui, Error> {
        let def = uison_json::parse(def)?.into_owned();
        self.lib.add_component(name, &def)?;
        Ok(self)
    }

    pub fn set_theme(&mut self, name: &str) {
        self.theme_name = name.to_string();
        self.merge_theme();
    }

    pub fn theme_name(&self) -> &str {
        &self.theme_name
    }

    /// Mutable access to the screen tree.
    pub fn tree(&mut self) -> Tree<'_> {
        Tree(&mut self.root)
    }

    /// The `system` theme (default) — light or dark, as in the system ([`Event::Appearance`]).
    fn merge_theme(&mut self) {
        let name = match self.theme_name.as_str() {
            "system" if self.system_dark => "dark",
            "system" => "light",
            n => n,
        };
        let base = self.lib.themes.get(name).or_else(|| self.lib.themes.get("light"));
        let fields = base.and_then(Json::as_object).unwrap_or(&[]).iter().chain(self.theme_over.as_object().unwrap_or(&[]));
        self.theme = fields.map(|(k, v)| (k.to_string(), v.clone())).collect();
    }

    fn token_color(&self, name: &str, fallback: Color) -> Color {
        self.theme.get(name).and_then(Json::as_str).and_then(Color::from_hex).unwrap_or(fallback)
    }

    // ---------------------------------------------------------------- frame building

    /// Build a frame: expansion → layout → emission.
    pub fn render(&mut self, m: &dyn Host) -> String {
        // One more pass — if List learned a new area height (virtualization)
        // or scrolling moved to show the focused field.
        for pass in 0..3 {
            let t0 = now();
            let env = Env { lib: &self.lib, theme: &self.theme, data: &self.data, st: &self.st };
            let mut roots = match expand_root(&self.root, &env) {
                Ok(r) => r,
                Err(e) => return self.error_frame(&e, m),
            };
            let t1 = now();
            let cx = Ctx { m, wrap: self.wrap };
            let s = if self.safe_area { self.safe } else { Insets::default() };
            let inner = Size::new((self.size.w - s.left - s.right).max(0.0), (self.size.h - s.top - s.bottom).max(0.0));
            for r in &mut roots {
                layout(r, Cons::tight(inner), &cx);
            }
            let t2 = now();
            let style = Style {
                background: self.token_color("background", Color::WHITE),
                ink: self.token_color("text", Color::BLACK),
                scrollbar: self.token_color("scrollbar", Color::rgba(0, 0, 0, 80)),
                touch: self.data.bool("env.touch"),
                capture: self.st.dragging.clone(),
            };
            // Tooltip and menu — as separate roots on top of the frame, at the cursor, pressed against the edges.
            let safe = uison_core::Rect::new(s.left, s.top, inner.w, inner.h);
            for (node, at, above) in self.popups() {
                match expand_root(&node, &env) {
                    Ok(els) => {
                        for mut el in els {
                            // Width — by content (without a limit `fill` shrinks), then exactly to it:
                            // item rows across the whole menu width, shortcuts at the right edge.
                            let natural = layout(&mut el, Cons::loose(f32::INFINITY, f32::INFINITY), &cx);
                            let w = natural.w.min(inner.w - 2.0 * POPUP_MARGIN);
                            let c = Cons { min_w: w, max_w: w, min_h: 0.0, max_h: inner.h - 2.0 * POPUP_MARGIN };
                            let sz = layout(&mut el, c, &cx);
                            let p = place(at, above, sz, safe);
                            el.pos = Point::new(p.x - s.left, p.y - s.top);
                            roots.push(el);
                        }
                    }
                    Err(e) => eprintln!("uison-ui: {e}"),
                }
            }
            let (drawers, data, warned) = (&mut self.drawers, &self.data, &mut self.warned);
            let mut draw = |name: &str, key: &str, rect: Rect, w: &mut Writer| match drawers.get_mut(name) {
                Some(f) => f(&mut Draw { w, rect, key, data, m }),
                None if warned.insert(format!("draw:{name}")) => eprintln!("uison-ui: no draw function '{name}' (Canvas {key})"),
                None => {}
            };
            let em = emit(&roots, self.size, Point::new(s.left, s.top), &style, m, &mut draw);
            let t3 = now();
            self.stats = Stats {
                expand: t1 - t0,
                layout: t2 - t1,
                emit: t3 - t2,
                nodes: roots.iter().map(|r| r.count()).sum(),
                prims: em.prims,
                bytes: em.src.len(),
            };
            let mut viewport_changed = false;
            for info in &em.infos {
                if let Some(s) = info.scroll {
                    viewport_changed |= self.st.viewport.insert(info.key.clone(), s.viewport) != Some(s.viewport);
                    // Content shrank — don't leave the scroll past the end.
                    if let Some(off) = self.st.scroll.get_mut(&info.key) {
                        *off = off.min((s.content - s.viewport).max(0.0));
                    }
                }
            }
            self.infos = em.infos;
            if let Some(laid) = &mut self.laid {
                *laid = roots;
            }
            let focused = self.st.focus.as_deref().and_then(|f| self.infos.iter().find(|i| i.key == f));
            self.st.input_shift = focused.and_then(|i| Some((i.key.clone(), i.input.as_ref()?.shift)));
            self.warn_duplicate_keys();
            let revealed = std::mem::take(&mut self.reveal) && self.reveal_focused();
            if !(viewport_changed || revealed) || pass == 2 {
                return em.src;
            }
        }
        unreachable!()
    }

    /// A popup from the runtime: (node, point, top when there is no room below).
    fn popups(&self) -> Vec<(Json, Point, f32)> {
        let mut out = Vec::new();
        if let Some(m) = &self.menu {
            out.push((popup_node("ContextMenu"), m.at, m.at.y));
        }
        #[cfg(all(any(debug_assertions, feature = "reload"), not(target_arch = "wasm32")))]
        if let Some(e) = &self.reload_error {
            out.push((error_banner(e), Point::new(0.0, 0.0), 0.0));
        }
        if let Some(t) = self.tip.as_ref().filter(|t| t.shown) {
            // Below the cursor (arrow ~20 pt); if it doesn't fit — above it.
            out.push((popup_node("Tooltip"), Point::new(t.at.x, t.at.y + 20.0), t.at.y - 6.0));
        }
        out
    }

    /// Error in the description — a red screen with the text (as in Flutter) instead of a crash.
    fn error_frame(&mut self, e: &Error, m: &dyn TextMeasure) -> String {
        eprintln!("uison-ui: {e}");
        self.infos.clear();
        let mut w = Writer::new(self.size, Color::rgb(0x5A, 0x12, 0x12));
        let title = Font { family: None, size: 17.0, bold: true, italic: false };
        w.text("Error in the UI description", Point::new(24.0, 24.0)).size(17.0).bold().color(Color::WHITE);
        let body = Font { family: Some("monospace".into()), size: 12.0, bold: false, italic: false };
        let lh = m.measure("Ag", &body).height().ceil() + 2.0;
        let mut y = 24.0 + m.measure("Ag", &title).height() + 12.0;
        for line in wrap(&e.to_string(), &body, self.size.w - 48.0, usize::MAX, WrapMode::WordSum, m) {
            w.text(&line.text, Point::new(24.0, y)).size(12.0).font("Menlo").color(Color::rgb(0xFF, 0xD7, 0xD7));
            y += lh;
        }
        w.finish()
    }

    /// The same key on two interactive elements is an error in the description: press and
    /// hover highlight both, the handler can't tell them apart. Warn once per key.
    fn warn_duplicate_keys(&mut self) {
        let mut seen = HashSet::with_capacity(self.infos.len());
        let dups: Vec<String> = self.infos.iter().filter(|i| !seen.insert(i.key.as_str())).map(|i| i.key.clone()).collect();
        for key in dups {
            if self.warned.insert(key.clone()) {
                eprintln!(
                    "uison-ui: duplicate key '{key}' — interactive elements need unique keys \
                     (inside a component use $param, not @alias: \"item-{{$m.id}}\")"
                );
            }
        }
    }

    /// Move the nearest scroll so the focused field is fully visible
    /// (with a margin). `true` — scrolling changed.
    fn reveal_focused(&mut self) -> bool {
        const MARGIN: f32 = 16.0;
        let Some(i) = self.st.focus.as_deref().and_then(|f| self.index(f)) else { return false };
        if self.infos[i].input.is_none() {
            return false;
        }
        let r = self.infos[i].rect;
        let Some(s) = self.chain(Some(i)).into_iter().skip(1).find(|&p| self.infos[p].scroll.is_some()) else { return false };
        let view = self.infos[s].rect;
        let dy = if r.bottom() + MARGIN > view.bottom() {
            r.bottom() + MARGIN - view.bottom()
        } else if r.y - MARGIN < view.y {
            r.y - MARGIN - view.y
        } else {
            return false;
        };
        let key = self.infos[s].key.clone();
        self.scroll_by(&key, -dy)
    }

    // ---------------------------------------------------------------- events

    fn index(&self, key: &str) -> Option<usize> {
        self.infos.iter().position(|i| i.key == key)
    }

    /// A node and its interactive ancestors.
    fn chain(&self, idx: Option<usize>) -> Vec<usize> {
        let mut out = Vec::new();
        let mut cur = idx;
        while let Some(i) = cur {
            out.push(i);
            cur = self.infos[i].parent;
        }
        out
    }

    fn click(&mut self, idx: usize) {
        match self.infos[idx].act.behavior.clone() {
            Some(Behavior::Toggle(p)) => {
                let v = !self.data.bool(&p);
                self.data.set(&p, v);
                self.fire(idx, "change");
            }
            Some(Behavior::Select(sets)) => {
                for (p, v) in sets {
                    self.data.set(&p, v);
                }
                self.fire(idx, "change");
            }
            _ => {}
        }
        if let Some(href) = self.infos[idx].act.href.clone() {
            self.asks.push(Ask::OpenUrl(href));
        }
        self.fire(idx, "click");
    }

    /// The event bubbles up to the first ancestor with a handler in `on`.
    fn fire(&mut self, idx: usize, event: &str) {
        let key = self.infos[idx].key.clone();
        for i in self.chain(Some(idx)) {
            if let Some((_, action)) = self.infos[i].act.on.iter().find(|(e, _)| e == event) {
                let action = action.clone();
                self.call(&action, event, &key);
                return;
            }
        }
    }

    fn call(&mut self, action: &str, event: &str, key: &str) {
        self.call_at(action, event, key, None);
    }

    /// `at` — pointer position in the node and click count of a replayed action; otherwise the current ones.
    fn call_at(&mut self, action: &str, event: &str, key: &str, at: Option<(Option<Point>, u32)>) {
        if action == "~menu" {
            // Context menu item: key is `~menu/<number>`.
            if let Some(i) = key.rsplit('/').next().and_then(|n| n.parse().ok()) {
                self.pick(i);
            }
            return;
        }
        let (pos, clicks) = at.unwrap_or_else(|| {
            let rect = self.infos.iter().find(|i| i.key == key).map(|i| i.rect);
            let pos = self.pointer.zip(rect).map(|(p, r)| Point::new(p.x - r.x, p.y - r.y));
            (pos, if event == "click" { self.clicks } else { 1 })
        });
        let Some(mut h) = self.handlers.remove(action) else {
            if self.warned.insert(action.to_string()) {
                eprintln!("uison-ui: no handler for action '{action}' ({event} on {key})");
            }
            return;
        };
        self.with_cx(action, event, key, pos, clicks, |cx| h(cx));
        self.handlers.insert(action.to_string(), h);
        // The handler may have built nodes with closures (cx.find(…).set("child", node)).
        self.adopt_handlers();
    }

    /// Run `f` with a handler context and apply its requests (focus, theme, timers…).
    #[allow(clippy::too_many_arguments)]
    fn with_cx(&mut self, action: &str, event: &str, key: &str, pos: Option<Point>, clicks: u32, f: impl FnOnce(&mut Cx)) {
        let mut req = Requests::default();
        let mut cx = Cx { data: &mut self.data, tree: Tree(&mut self.root), action, event, key, pos, clicks, req: &mut req };
        f(&mut cx);
        if let Some(t) = req.theme {
            self.set_theme(&t);
        }
        if !req.tokens.is_empty() {
            for (k, v) in req.tokens {
                self.theme_over.set(&k, v);
            }
            self.merge_theme();
            self.theme_version += 1;
        }
        let t = now();
        for (d, a) in req.timers {
            self.timers.push((t + d, a));
        }
        self.asks.extend(req.asks);
        if let Some(f) = req.focus {
            self.st.focus = f;
            self.reset_caret();
        }
    }

    /// Caret to the end of the field, no selection (new focus).
    fn reset_caret(&mut self) {
        self.st.caret = usize::MAX;
        self.st.anchor = usize::MAX;
        self.st.caret_on = true;
    }

    fn drag_to(&mut self, idx: usize, p: Point) {
        let info = &self.infos[idx];
        if let Some(Behavior::Drag { path, min, max, step, inset, delta }) = &info.act.behavior {
            let r = info.rect;
            let mut v = if *delta != 0.0 {
                let (x0, v0) = self.st.drag_from;
                (v0 + delta * (p.x - x0)).clamp(*min, max.max(*min))
            } else {
                let t = ((p.x - r.x - inset) / (r.w - 2.0 * inset).max(1.0)).clamp(0.0, 1.0);
                min + t * (max - min)
            };
            if *step > 0.0 {
                v = min + ((v - min) / step).round() * step;
            }
            let path = path.clone();
            if self.data.num(&path) as f32 != v {
                self.data.set(&path, v as f64);
                self.fire(idx, "change");
            }
        }
    }

    /// Index of the character in the field closest to x on screen.
    fn caret_at(&self, idx: usize, x: f32, m: &dyn TextMeasure) -> usize {
        let Some(inp) = &self.infos[idx].input else { return 0 };
        let mut best = (f32::INFINITY, 0);
        for (i, _) in inp.value.char_indices().chain([(inp.value.len(), ' ')]) {
            let prefix = if inp.password { "•".repeat(inp.value[..i].chars().count()) } else { inp.value[..i].to_string() };
            let d = (inp.text_x + m.measure(&prefix, &inp.font).width - x).abs();
            if d < best.0 {
                best = (d, i);
            }
        }
        best.1
    }

    fn focus_next(&mut self, back: bool) {
        let order: Vec<usize> = (0..self.infos.len()).filter(|&i| self.infos[i].act.focusable && !self.infos[i].act.disabled).collect();
        if order.is_empty() {
            return;
        }
        let cur = self.st.focus.as_deref().and_then(|f| order.iter().position(|&i| self.infos[i].key == f));
        let n = order.len();
        let next = match (cur, back) {
            (None, false) => 0,
            (None, true) => n - 1,
            (Some(c), false) => (c + 1) % n,
            (Some(c), true) => (c + n - 1) % n,
        };
        self.st.focus = Some(self.infos[order[next]].key.clone());
        self.reset_caret();
    }

    /// Focused field: value, caret and selection.
    fn field(&self, idx: usize) -> Option<(crate::emit::InputInfo, Field)> {
        let inp = self.infos[idx].input.clone()?;
        let f = Field::new(self.data.str(&inp.path), self.st.caret, self.st.anchor);
        Some((inp, f))
    }

    /// A message from outside ([`Event::Message`]): `{"action":"save","event":"click","key":"ok",
    /// "pos":[x,y],"clicks":1}` runs the action as if the element had fired it, `{"data":{path: value}}`
    /// sets data. `false` — not a message `Ui` understands.
    fn message(&mut self, msg: &str) -> bool {
        let Ok(v) = uison_json::parse(msg) else { return false };
        if let Some(fields) = v.get("data").and_then(uison_json::Value::as_object) {
            for (path, value) in fields {
                self.data.set(path, value.clone().into_owned());
            }
        }
        let Some(action) = v.get("action").and_then(uison_json::Value::as_str) else { return v.get("data").is_some() };
        let s = |k: &str| v.get(k).and_then(uison_json::Value::as_str).unwrap_or("");
        let pos = v.get("pos").and_then(uison_json::Value::as_array).and_then(|p| {
            Some(Point::new(p.first()?.as_f64()? as f32, p.get(1)?.as_f64()? as f32))
        });
        let clicks = v.get("clicks").and_then(uison_json::Value::as_f64).map_or(1, |c| c as u32);
        let event = if s("event").is_empty() { "click" } else { s("event") };
        self.call_at(action, event, s("key"), Some((pos, clicks)));
        true
    }

    /// Apply a message as [`Event::Message`] does (tests, the remote side): an action or data change.
    pub fn apply(&mut self, msg: &str) -> bool {
        let version = self.data.version();
        let known = self.message(msg);
        if self.data.version() != version {
            self.run_derives();
        }
        known
    }

    /// Save the field after an edit: caret, selection and (if changed) the value.
    fn store_field(&mut self, idx: usize, path: &str, f: Field) {
        self.st.caret = f.caret;
        self.st.anchor = f.anchor;
        self.st.caret_on = true;
        self.caret_toggled = now();
        if self.data.str(path) != f.text {
            self.data.set(path, f.text);
            self.fire(idx, "change");
        }
    }

    /// Editing the focused field. Keys as in a system field: by words
    /// Alt on Apple and Ctrl (`cmd`) elsewhere, to the start and end of the line — ⌘ on Apple.
    fn edit(&mut self, idx: usize, key: &uison_core::Key) -> bool {
        let Some((inp, mut f)) = self.field(idx) else { return false };
        let m = key.mods;
        let apple = self.data.str("env.mod") == "⌘";
        let word = if apple { m.alt } else { m.cmd };
        let line = apple && m.cmd;
        let s = f.text.as_str();
        let c = f.caret;
        match key.code {
            KeyCode::Char if m.cmd => match key.text.as_deref().map(str::to_lowercase).as_deref() {
                Some("a") => f.select(0, f.text.len()),
                Some(k @ ("c" | "x")) => {
                    if f.selection().is_none() || inp.password {
                        return false;
                    }
                    self.asks.push(Ask::Copy(f.selected().to_string()));
                    if k == "x" {
                        f.insert("");
                    }
                }
                Some("v") => {
                    self.asks.push(Ask::Paste);
                    return false;
                }
                _ => return false,
            },
            // Ctrl+letter on Apple is not input (Emacs bindings are not supported).
            KeyCode::Char if m.ctrl => return false,
            KeyCode::Char => match &key.text {
                Some(t) => f.insert(t),
                None => return false,
            },
            KeyCode::Backspace => {
                let to = if line { 0 } else if word { word_left(s, c) } else { prev_char(s, c) };
                f.delete_to(to);
            }
            KeyCode::Delete => {
                let to = if line { s.len() } else if word && apple { word_right(s, c) } else if word { next_word(s, c) } else { next_char(s, c) };
                f.delete_to(to);
            }
            KeyCode::Left | KeyCode::Right => {
                let right = key.code == KeyCode::Right;
                let to = match f.selection() {
                    // An arrow without Shift collapses the selection to its edge.
                    Some((a, b)) if !m.shift && !word && !line => if right { b } else { a },
                    _ if line => if right { s.len() } else { 0 },
                    _ if word && right => if apple { word_right(s, c) } else { next_word(s, c) },
                    _ if word => word_left(s, c),
                    _ if right => next_char(s, c),
                    _ => prev_char(s, c),
                };
                f.move_to(to, m.shift);
            }
            KeyCode::Home | KeyCode::Up => f.move_to(0, m.shift),
            KeyCode::End | KeyCode::Down => f.move_to(s.len(), m.shift),
            KeyCode::Enter => {
                self.fire(idx, "submit");
                return true;
            }
            _ => return false,
        }
        self.store_field(idx, &inp.path, f);
        true
    }

    /// Paste from the clipboard into the focused field: the field is single-line — line breaks become spaces.
    fn paste(&mut self, text: &str) -> bool {
        let Some(i) = self.st.focus.as_deref().and_then(|k| self.index(k)) else { return false };
        let Some((inp, mut f)) = self.field(i) else { return false };
        let clean: String = text.replace("\r\n", " ").chars().map(|c| if c == '\n' || c == '\r' || c == '\t' { ' ' } else { c }).collect();
        f.insert(&clean);
        self.store_field(i, &inp.path, f);
        true
    }

    /// Handle an event; `true` — state or data changed.
    fn handle_event(&mut self, e: &Event, hit: Option<&str>, m: &dyn Host) -> bool {
        let theme = (self.theme_name.clone(), self.theme_version);
        self.pointer = e.pos();
        // A press, scroll, key or the mouse leaving hides the tooltip.
        let mut tip_hidden = false;
        if matches!(e, Event::MouseDown(..) | Event::Scroll(..) | Event::Key(_) | Event::MouseLeave | Event::Paste(_)) {
            tip_hidden = self.tip.take().is_some_and(|t| t.shown);
        }
        let changed = self.handle_inner(e, hit, m);
        self.sync_popup_data();
        changed || tip_hidden || theme != (self.theme_name.clone(), self.theme_version)
    }

    /// Data for the Tooltip and ContextMenu components (`@env.tooltip`, `@env.menu`).
    fn sync_popup_data(&mut self) {
        let tip = self.tip.as_ref().filter(|t| t.shown).map_or("", |t| t.text.as_str()).to_string();
        if self.data.str("env.tooltip") != tip {
            self.data.set("env.tooltip", tip);
        }
        let menu = match &self.menu {
            Some(m) => {
                let items: Vec<Json> = m
                    .items
                    .iter()
                    .enumerate()
                    .map(|(i, it)| {
                        let mut o = Json::object();
                        o.set("label", Json::from(it.label.clone()));
                        o.set("shortcut", Json::from(it.shortcut.clone()));
                        o.set("disabled", Json::from(it.disabled));
                        o.set("sep", Json::from(it.sep));
                        o.set("sel", Json::from(m.sel == Some(i)));
                        o
                    })
                    .collect();
                let mut o = Json::object();
                o.set("open", Json::from(true));
                o.set("items", Json::Array(items));
                o.set("itemH", Json::from(if self.data.bool("env.touch") { 40.0 } else { 26.0 }));
                o
            }
            None => Json::Null,
        };
        if self.data.get("env.menu") != Some(&menu) && !(menu.is_null() && self.data.get("env.menu").is_none()) {
            self.data.set("env.menu", menu);
        }
    }

    // ---------------------------------------------------------------- tooltips and menus

    /// Cursor over a node with a tooltip: counted from the last movement; a shown one doesn't blink
    /// while the cursor stays inside the same node. `true` — the visible changed.
    fn track_tooltip(&mut self, hit: Option<usize>, p: Point) -> bool {
        let target = if self.menu.is_some() || self.data.bool("env.touch") {
            None
        } else {
            self.chain(hit).into_iter().find_map(|i| Some((self.infos[i].key.clone(), self.infos[i].act.tooltip.clone()?)))
        };
        match (&mut self.tip, target) {
            (Some(t), Some((key, text))) if t.key == key => {
                if !t.shown {
                    (t.since, t.at, t.text) = (now(), p, text);
                }
                false
            }
            (tip, target) => {
                let was = tip.as_ref().is_some_and(|t| t.shown);
                *tip = target.map(|(key, text)| Tip { key, text, since: now(), at: p, shown: false });
                was
            }
        }
    }

    /// Items from `contextMenu`: `{ label, action, shortcut?, disabled? }` and `"-"`.
    fn parse_menu(items: &Json) -> Vec<MenuItem> {
        let mut out = Vec::new();
        for it in items.as_array().unwrap_or(&[]) {
            if it.as_str() == Some("-") {
                out.push(MenuItem { label: String::new(), action: String::new(), shortcut: String::new(), disabled: true, sep: true });
                continue;
            }
            let s = |k: &str| it.get(k).map(to_text).unwrap_or_default();
            out.push(MenuItem {
                label: s("label"),
                action: s("action"),
                shortcut: s("shortcut"),
                disabled: it.get("disabled").is_some_and(Json::truthy),
                sep: false,
            });
        }
        out
    }

    /// Open the menu of node `idx` (contextMenu property) or of an input field; `false` — the node has no menu.
    fn open_menu(&mut self, idx: usize, at: Point, press: Option<Point>) -> bool {
        self.tip = None;
        let info = &self.infos[idx];
        let key = info.key.clone();
        let (items, field) = if let Some(m) = &info.act.menu {
            (Self::parse_menu(m), false)
        } else if let Some(inp) = &info.input {
            let f = Field::new(self.data.str(&inp.path), self.st.caret, self.st.anchor);
            let sel = f.selection().is_some() && !inp.password;
            let sc = |k: &str| if self.data.bool("env.touch") { String::new() } else { format!("{}{k}", self.data.str("env.mod")) };
            let item = |label: &str, action: &str, k: &str, on: bool| MenuItem {
                label: label.into(),
                action: action.into(),
                shortcut: sc(k),
                disabled: !on,
                sep: false,
            };
            let items = vec![
                item("Cut", "cut", "X", sel),
                item("Copy", "copy", "C", sel),
                item("Paste", "paste", "V", true),
                MenuItem { label: String::new(), action: String::new(), shortcut: String::new(), disabled: true, sep: true },
                item("Select All", "selectAll", "A", !f.text.is_empty()),
            ];
            (items, true)
        } else {
            return false;
        };
        if items.is_empty() {
            return false;
        }
        self.menu = Some(Menu { key, at, items, sel: None, field, press });
        true
    }

    /// Choose a menu item: close the menu and run the action.
    fn pick(&mut self, i: usize) {
        let Some(menu) = self.menu.take() else { return };
        let Some(item) = menu.items.get(i).filter(|it| it.usable()).cloned() else {
            self.menu = Some(menu);
            return;
        };
        if !menu.field {
            self.call(&item.action, "menu", &menu.key);
            return;
        }
        let Some(idx) = self.index(&menu.key) else { return };
        let Some((inp, mut f)) = self.field(idx) else { return };
        match item.action.as_str() {
            "cut" | "copy" => {
                self.asks.push(Ask::Copy(f.selected().to_string()));
                if item.action == "cut" {
                    f.insert("");
                }
            }
            "paste" => self.asks.push(Ask::Paste),
            _ => f.select(0, f.text.len()),
        }
        self.st.focus = Some(menu.key.clone());
        self.store_field(idx, &inp.path, f);
    }

    /// Keys while a menu is open: arrows, Enter, Escape, the first letter of an item.
    fn menu_key(&mut self, k: &uison_core::Key) -> bool {
        let Some(m) = self.menu.as_mut() else { return false };
        let n = m.items.len();
        let usable: Vec<usize> = (0..n).filter(|&i| m.items[i].usable()).collect();
        let step = |back: bool| -> Option<usize> {
            let pos = m.sel.and_then(|s| usable.iter().position(|&u| u == s));
            let next = match (pos, back) {
                (None, false) => 0,
                (None, true) => usable.len().checked_sub(1)?,
                (Some(p), false) => (p + 1) % usable.len(),
                (Some(p), true) => (p + usable.len() - 1) % usable.len(),
            };
            usable.get(next).copied()
        };
        match k.code {
            KeyCode::Down => m.sel = step(false),
            KeyCode::Up => m.sel = step(true),
            KeyCode::Enter => {
                if let Some(s) = m.sel {
                    self.pick(s);
                }
            }
            KeyCode::Escape | KeyCode::Back => self.menu = None,
            KeyCode::Char => {
                let Some(c) = k.text.as_deref().and_then(|t| t.chars().next()).map(|c| c.to_lowercase().to_string()) else { return true };
                if let Some(&i) = usable.iter().find(|&&i| m.items[i].label.to_lowercase().starts_with(&c)) {
                    m.sel = Some(i);
                }
            }
            _ => {}
        }
        true
    }

    /// From the keyboard (Menu, Shift+F10): the focused node's menu — at its bottom-left corner.
    fn menu_for_focus(&mut self) -> bool {
        let Some(i) = self.st.focus.as_deref().and_then(|f| self.index(f)) else { return false };
        let r = self.infos[i].rect;
        let at = Point::new(r.x, r.bottom() + 2.0);
        let target = self.chain(Some(i)).into_iter().find(|&j| self.infos[j].act.menu.is_some() || self.infos[j].input.is_some());
        target.is_some_and(|j| self.open_menu(j, at, None))
    }

    fn handle_inner(&mut self, e: &Event, hit: Option<&str>, m: &dyn Host) -> bool {
        let hit = hit.and_then(|h| self.index(h));
        match e {
            Event::MouseMove(p) => {
                if let Some(i) = self.st.selecting.as_deref().and_then(|k| self.index(k)) {
                    let c = self.caret_at(i, p.x, m);
                    if c == self.st.caret {
                        return false;
                    }
                    self.st.caret = c;
                    self.st.caret_on = true;
                    return true;
                }
                if let Some(k) = self.st.dragging.clone() {
                    if let Some(i) = self.index(&k) {
                        self.drag_to(i, *p);
                    }
                    return false;
                }
                if self.touch.is_some() {
                    return self.touch_move(*p);
                }
                let tip = self.track_tooltip(hit, *p);
                let moved = self.fire_hover(hit);
                let hover: Vec<String> =
                    self.chain(hit).into_iter().filter(|&i| !self.infos[i].act.disabled).map(|i| self.infos[i].key.clone()).collect();
                if hover == self.st.hover {
                    return tip || moved;
                }
                self.st.hover = hover;
            }
            Event::MouseLeave => {
                let left = self.fire_hover(None);
                if self.st.hover.is_empty() {
                    return left;
                }
                self.st.hover.clear();
            }
            Event::MouseDown(p, MouseButton::Touch) => return self.touch_down(*p, hit, m),
            Event::MouseUp(p, MouseButton::Touch) => return self.touch_up(*p, hit, m),
            Event::MouseDown(p, MouseButton::Left) => {
                if self.data.bool("env.touch") {
                    self.data.set("env.touch", false);
                }
                if self.dismiss_outside(hit) {
                    return true;
                }
                self.press(*p, hit, m);
                // Dragging with the mouse selects text in the pressed field.
                self.st.selecting = self.st.pressed.clone().filter(|k| self.index(k).is_some_and(|i| self.infos[i].input.is_some()));
            }
            Event::MouseUp(p, MouseButton::Left) => return self.release(*p, hit),
            Event::MouseDown(p, MouseButton::Right) => {
                let was = self.menu.take().is_some();
                self.dismiss_outside(hit);
                let chain = self.chain(hit);
                let Some(i) = chain.iter().copied().find(|&i| self.infos[i].act.menu.is_some() || self.infos[i].input.is_some()) else {
                    return was;
                };
                if self.infos[i].input.is_some() {
                    // Right button in a field: focus; outside the selection — the caret goes there.
                    let key = self.infos[i].key.clone();
                    let c = self.caret_at(i, p.x, m);
                    let f = self.field(i).map(|(_, f)| f);
                    let inside = self.st.focus.as_deref() == Some(key.as_str())
                        && f.and_then(|f| f.selection()).is_some_and(|(a, b)| (a..=b).contains(&c));
                    self.st.focus = Some(key);
                    if !inside {
                        (self.st.caret, self.st.anchor) = (c, c);
                    }
                }
                return self.open_menu(i, *p, Some(*p)) || was;
            }
            Event::MouseUp(p, MouseButton::Right) => {
                // As in macOS: press the right button, move to an item, release — select it.
                let Some(m) = self.menu.as_mut() else { return false };
                let Some(p0) = m.press.take() else { return false };
                let moved = (p.x - p0.x).abs() > MULTI_CLICK_DIST || (p.y - p0.y).abs() > MULTI_CLICK_DIST;
                let item = hit.map(|h| self.infos[h].key.clone()).and_then(|k| k.strip_prefix("~menu/").and_then(|n| n.parse().ok()));
                if let (true, Some(i)) = (moved, item) {
                    self.pick(i);
                    return true;
                }
                return false;
            }
            Event::Paste(t) => return self.paste(t),
            Event::Scroll(_, _, dy) => {
                let Some(i) = self.chain(hit).into_iter().find(|&i| self.infos[i].scroll.is_some()) else { return false };
                let key = self.infos[i].key.clone();
                return self.scroll_by(&key, *dy);
            }
            Event::Key(k) => {
                if self.menu.is_some() {
                    return self.menu_key(k);
                }
                if k.code == KeyCode::Menu || (k.code == KeyCode::F10 && k.mods.shift) {
                    return self.menu_for_focus();
                }
                if let Some((_, action)) = self.shortcuts.iter().find(|(sc, _)| sc.matches(k)) {
                    let action = action.clone();
                    self.call(&action, "shortcut", "");
                    return true;
                }
                let focused = self.st.focus.as_deref().and_then(|f| self.index(f));
                match k.code {
                    // System "Back" (Android): close the popup, drop focus,
                    // otherwise — the app's `back` action (nothing changed — the system decides to exit).
                    KeyCode::Back if self.dismiss_all() => {}
                    KeyCode::Back if self.st.focus.is_some() => self.st.focus = None,
                    KeyCode::Back => {
                        if self.handlers.contains_key("back") {
                            self.call("back", "back", "");
                        }
                        return false;
                    }
                    KeyCode::Escape if self.dismiss_all() => {}
                    KeyCode::Tab => {
                        self.focus_next(k.mods.shift);
                        self.reveal = true;
                    }
                    KeyCode::Escape if self.st.focus.is_some() => self.st.focus = None,
                    _ => match focused {
                        Some(i) if self.infos[i].input.is_some() => return self.edit(i, k),
                        Some(i) if k.code == KeyCode::Enter || k.text.as_deref() == Some(" ") => self.click(i),
                        _ => return false,
                    },
                }
            }
            Event::Tick => {
                let now = now();
                // Fired timers — in deadline order.
                let mut due: Vec<(Duration, String)> = Vec::new();
                self.timers.retain(|(t, a)| {
                    let fire = *t <= now + TIMER_SLACK;
                    if fire {
                        due.push((*t, a.clone()));
                    }
                    !fire
                });
                due.sort_by_key(|(t, _)| *t);
                for (_, a) in &due {
                    self.call(a, "timer", "");
                }
                // The caret blinks by time, not by the number of ticks.
                let blink = self.focused_input() && now.saturating_sub(self.caret_toggled) >= CARET_BLINK;
                if blink {
                    self.st.caret_on = !self.st.caret_on;
                    self.caret_toggled = now;
                }
                let flung = self.fling_step(now);
                let held = self.hold_step(now, m);
                #[cfg(all(any(debug_assertions, feature = "reload"), not(target_arch = "wasm32")))]
                let held = held | self.check_reload(now);
                let tip = match self.tip.as_mut() {
                    Some(t) if !t.shown && now + TIMER_SLACK >= t.since + TOOLTIP_DELAY => {
                        t.shown = true;
                        true
                    }
                    _ => false,
                };
                return !due.is_empty() || blink || flung || held || tip;
            }
            Event::Resize(s) => {
                self.size = *s;
                self.set_env();
            }
            Event::Appearance(dark) => {
                if *dark == self.system_dark {
                    return false;
                }
                self.system_dark = *dark;
                self.merge_theme();
                self.theme_version += 1;
                self.set_env();
                // Let the app recompute its tokens (the `appearance` action, if any).
                if self.handlers.contains_key("appearance") {
                    self.call("appearance", "appearance", "");
                }
            }
            // An image finished loading (browser) — lay out again: its size may now be known.
            Event::ImageLoaded => {}
            Event::Wake => return self.run_posted(),
            Event::Message(msg) => return self.message(msg),
            Event::SafeArea(s) => {
                self.safe = *s;
                self.reveal = true;
                self.set_env();
            }
            _ => return false,
        }
        true
    }

    /// Press: the nearest pressable node, focus, caret, start of dragging.
    fn press(&mut self, p: Point, hit: Option<usize>, m: &dyn Host) {
        // The nearest pressable node becomes pressed, not the scroll around it.
        let target = self.chain(hit).into_iter().find(|&i| self.infos[i].act.pressable());
        self.st.pressed = target.map(|i| self.infos[i].key.clone());
        self.clicks = match &self.st.pressed {
            Some(key) => self.count_click(p, &key.clone()),
            None => 1,
        };
        let focus = target.filter(|&i| self.infos[i].act.focusable).map(|i| self.infos[i].key.clone());
        let same = focus.is_some() && focus == self.st.focus;
        if focus.is_some() || self.st.focus.is_some() {
            self.st.focus = focus;
        }
        if let Some(i) = target {
            if let Some(inp) = &self.infos[i].input {
                let value = inp.value.clone();
                let c = self.caret_at(i, p.x, m);
                let (a, b) = match self.clicks {
                    2 => word_at(&value, c),
                    3 => (0, value.len()),
                    // Shift+click in the focused field — extend the selection from the previous anchor.
                    _ if same && m.mods().shift => (Field::new(&value, self.st.caret, self.st.anchor).anchor, c),
                    _ => (c, c),
                };
                self.st.anchor = a;
                self.st.caret = b;
                self.st.caret_on = true;
                self.caret_toggled = now();
            }
            if let Some(Behavior::Drag { path, .. }) = &self.infos[i].act.behavior {
                self.st.dragging = Some(self.infos[i].key.clone());
                self.st.drag_from = (p.x, self.data.num(path) as f32);
                self.drag_to(i, p);
            }
        }
    }

    /// A `hover` event to the node with `on.hover` under the cursor (`cx.pos` — the point), and to the previous one
    /// the cursor left — without a point. `true` — the handler was called.
    fn fire_hover(&mut self, hit: Option<usize>) -> bool {
        let has = |i: &NodeInfo| i.act.on.iter().any(|(e, _)| e == "hover");
        let target = self.chain(hit).into_iter().find(|&i| has(&self.infos[i])).map(|i| self.infos[i].key.clone());
        let mut fired = false;
        if let Some(old) = self.hovering.take().filter(|k| Some(k) != target.as_ref())
            && let Some(i) = self.index(&old)
        {
            let p = self.pointer.take();
            self.fire(i, "hover");
            self.pointer = p;
            fired = true;
        }
        if let Some(i) = target.as_deref().and_then(|k| self.index(k)) {
            self.fire(i, "hover");
            fired = true;
        }
        self.hovering = target;
        fired
    }

    /// Click number in the series (1, 2, 3, then 1 again): fast and close to the previous one on the same node.
    fn count_click(&mut self, p: Point, key: &str) -> u32 {
        let t = now();
        let n = match &self.last_press {
            Some((t0, p0, k, n))
                if t.saturating_sub(*t0) <= MULTI_CLICK
                    && (p.x - p0.x).abs() <= MULTI_CLICK_DIST
                    && (p.y - p0.y).abs() <= MULTI_CLICK_DIST
                    && k == key =>
            {
                n % 3 + 1
            }
            _ => 1,
        };
        self.last_press = Some((t, p, key.to_string(), n));
        n
    }

    /// Release: a click if released over the pressed node.
    fn release(&mut self, p: Point, hit: Option<usize>) -> bool {
        // While dragging, the whole window is the dragged node's area (`emit`, `capture`): over the node
        // is where its own area is.
        let dragged = self.st.dragging.take().and_then(|k| self.index(&k));
        self.st.selecting = None;
        let Some(pressed) = self.st.pressed.take() else { return false };
        let over = match dragged {
            Some(i) => {
                let r = self.infos[i].rect;
                (if self.data.bool("env.touch") { at_least(r, MIN_TOUCH) } else { r }).contains(p).then_some(i)
            }
            None => self.chain(hit).into_iter().find(|&i| self.infos[i].key == pressed),
        };
        if let Some(i) = over {
            self.click(i);
        }
        true
    }

    /// Move scroll `key` (dy > 0 — content down); `false` — hit the edge.
    fn scroll_by(&mut self, key: &str, dy: f32) -> bool {
        let Some(s) = self.index(key).and_then(|i| self.infos[i].scroll) else { return false };
        let cur = self.st.scroll.get(key).copied().unwrap_or(0.0);
        let new = (cur - dy).clamp(0.0, (s.content - s.viewport).max(0.0));
        if new == cur {
            return false;
        }
        self.st.scroll.insert(key.to_string(), new);
        true
    }

    // ---------------------------------------------------------------- touches

    /// A finger touched: the press is visible right away, but focus and click — only on
    /// release without movement (the touch may turn out to be a scroll).
    fn touch_down(&mut self, p: Point, hit: Option<usize>, m: &dyn Host) -> bool {
        let t = now();
        if !self.data.bool("env.touch") {
            self.data.set("env.touch", true);
        }
        // A touch during inertia or bounce only stops them.
        let mut swallowed = self.fling.take().is_some() | self.spring.take().is_some();
        swallowed |= self.dismiss_outside(hit);
        let chain = self.chain(hit);
        let scroll = chain
            .iter()
            .find(|&&i| self.infos[i].scroll.is_some_and(|s| s.content > s.viewport))
            .map(|&i| self.infos[i].key.clone());
        // Long press — only where it makes sense (input field: select a word).
        let hold = chain
            .iter()
            .find(|&&i| self.infos[i].input.is_some() || self.infos[i].act.menu.is_some())
            .map(|&i| self.infos[i].key.clone());
        self.touch = Some(Touch::new(p, t, scroll, swallowed, hold.filter(|_| !swallowed)));
        self.st.hover.clear();
        if !swallowed {
            let target = chain.into_iter().find(|&i| self.infos[i].act.pressable());
            match target {
                // The slider is dragged right away.
                Some(i) if matches!(self.infos[i].act.behavior, Some(Behavior::Drag { .. })) => self.press(p, hit, m),
                _ => self.st.pressed = target.map(|i| self.infos[i].key.clone()),
            }
        }
        true
    }

    fn touch_move(&mut self, p: Point) -> bool {
        let t = now();
        let Some(touch) = self.touch.as_mut() else { return false };
        let mut dy = touch.moved(p, t);
        if !touch.scrolling {
            let total = p.y - touch.start.y;
            if touch.scroll.is_none() || total.abs() <= SLOP {
                return false;
            }
            // Became a scroll: the press is cancelled; content follows from the point where the threshold ended.
            touch.scrolling = true;
            dy = total - SLOP.copysign(total);
            self.st.pressed = None;
            // The finger may pick up content still pulled past the edge.
            let key = touch.scroll.clone().unwrap_or_default();
            let dim = self.scroll_info(&key).map_or(1.0, |s| s.viewport);
            let over = self.st.over.get(&key).copied().unwrap_or(0.0);
            let touch = self.touch.as_mut().unwrap();
            touch.raw = self.st.scroll.get(&key).copied().unwrap_or(0.0) + unrubber(over, dim);
        }
        let touch = self.touch.as_mut().unwrap();
        touch.raw -= dy;
        let (key, raw) = (touch.scroll.clone().unwrap_or_default(), touch.raw);
        self.set_raw_scroll(&key, raw)
    }

    fn scroll_info(&self, key: &str) -> Option<crate::emit::ScrollInfo> {
        self.index(key).and_then(|i| self.infos[i].scroll)
    }

    /// Offset "where the finger took it": within the content — scrolling, past the edge — overscroll with resistance.
    fn set_raw_scroll(&mut self, key: &str, raw: f32) -> bool {
        let Some(s) = self.scroll_info(key) else { return false };
        let off = raw.clamp(0.0, (s.content - s.viewport).max(0.0));
        let over = rubber(raw - off, s.viewport);
        let changed = self.st.scroll.get(key) != Some(&off) || self.st.over.get(key).copied().unwrap_or(0.0) != over;
        self.st.scroll.insert(key.to_string(), off);
        self.set_over(key, over);
        changed
    }

    fn set_over(&mut self, key: &str, over: f32) {
        if over == 0.0 {
            self.st.over.remove(key);
        } else {
            self.st.over.insert(key.to_string(), over);
        }
    }

    fn touch_up(&mut self, p: Point, hit: Option<usize>, m: &dyn Host) -> bool {
        let Some(touch) = self.touch.take() else { return false };
        // The long press already fired — the release presses nothing.
        if touch.held {
            self.st.pressed = None;
            return true;
        }
        if self.st.dragging.is_some() {
            return self.release(p, hit);
        }
        let t = now();
        let key = touch.scroll.clone().unwrap_or_default();
        // Released while pulled past the edge — spring back (also after a touch that stopped the bounce).
        if let Some(&over) = self.st.over.get(&key) {
            self.spring = Some(Spring::new(key, over, 0.0, t));
            self.st.pressed = None;
            return true;
        }
        if touch.scrolling {
            let v = touch.velocity(t);
            self.fling = Fling::new(key, v, t);
            return true;
        }
        if touch.swallowed {
            self.st.pressed = None;
            return true;
        }
        let Some(pressed) = self.st.pressed.clone() else {
            // A tap outside pressable elements drops focus (and hides the keyboard).
            let had = self.st.focus.is_some();
            self.press(p, hit, m);
            return had;
        };
        if self.chain(hit).iter().any(|&i| self.infos[i].key == pressed) {
            self.press(p, hit, m);
            self.reveal = true;
            self.release(p, hit);
        }
        self.st.pressed = None;
        true
    }

    /// The finger stays still longer than `HOLD` — a long press: in an input field — focus and the selected word.
    fn hold_step(&mut self, now: Duration, m: &dyn Host) -> bool {
        let Some(touch) = self.touch.as_mut() else { return false };
        if touch.hold_at().is_none_or(|t| now + TIMER_SLACK < t) {
            return false;
        }
        touch.held = true;
        let (p, key) = (touch.start, touch.hold.clone().unwrap_or_default());
        self.st.pressed = None;
        let Some(i) = self.index(&key) else { return true };
        if self.infos[i].input.is_none() {
            return self.open_menu(i, p, None);
        }
        let c = self.caret_at(i, p.x, m);
        // Inside the selection of the focused field: keep it (Select all, then Copy) — as system fields do.
        let (lo, hi) = (self.st.anchor.min(self.st.caret), self.st.anchor.max(self.st.caret));
        let inside = self.st.focus.as_deref() == Some(key.as_str()) && lo != hi && hi != usize::MAX && (lo..=hi).contains(&c);
        self.st.focus = Some(key);
        if !inside {
            let value = self.infos[i].input.as_ref().map_or(String::new(), |inp| inp.value.clone());
            let (a, b) = word_at(&value, c);
            self.st.anchor = a;
            self.st.caret = b;
        }
        self.st.caret_on = true;
        self.reveal = true;
        // Under the finger — above the word: the field menu above it.
        self.open_menu(i, Point::new(p.x, (p.y - 60.0).max(0.0)), None);
        true
    }

    /// A step of inertia or bounce; `true` — the scroll moved.
    fn fling_step(&mut self, now: Duration) -> bool {
        if let Some(s) = &self.spring {
            let (x, alive) = s.at(now);
            let key = s.key.clone();
            self.set_over(&key, x);
            if !alive {
                self.spring = None;
            }
            return true;
        }
        let Some(f) = self.fling.as_mut() else { return false };
        let (d, alive) = f.step(now);
        let (key, v) = (f.key.clone(), f.v);
        let moved = self.scroll_by(&key, d);
        if !alive || !moved {
            self.fling = None;
        }
        // Inertia hit the edge — the remaining velocity goes into the bounce.
        if alive && !moved {
            self.spring = Some(Spring::new(key, 0.0, -v, now));
            return self.fling_step(now + Duration::from_millis(1));
        }
        moved
    }

    /// Open overlays with `dismiss`, except those containing the click or whose
    /// toggle is pressed (it closes itself). Closes them; `true` — the click is consumed.
    fn dismiss_outside(&mut self, hit: Option<usize>) -> bool {
        let chain = self.chain(hit);
        let mut closed = false;
        for i in 0..self.infos.len() {
            let Some(path) = self.infos[i].act.dismiss.clone() else { continue };
            if !self.data.bool(&path) || chain.contains(&i) {
                continue;
            }
            let anchor = chain.iter().any(|&c| matches!(&self.infos[c].act.behavior, Some(Behavior::Toggle(p)) if *p == path));
            if !anchor {
                self.close(&path);
                closed = true;
            }
        }
        closed
    }

    fn dismiss_all(&mut self) -> bool {
        let paths: Vec<String> = self.infos.iter().filter_map(|i| i.act.dismiss.clone()).filter(|p| self.data.bool(p)).collect();
        for p in &paths {
            self.close(p);
        }
        !paths.is_empty()
    }

    /// Close the popup at the `dismiss` path (the context menu is runtime state).
    fn close(&mut self, path: &str) {
        self.data.set(path, false);
        if path == "env.menu.open" {
            self.menu = None;
        }
    }

    fn focused_input(&self) -> bool {
        self.st.focus.as_deref().and_then(|f| self.index(f)).is_some_and(|i| self.infos[i].input.is_some())
    }

    /// Run the requests to the platform.
    fn flush_asks(&mut self, host: &dyn Host) {
        let mut unsupported = Vec::new();
        for a in std::mem::take(&mut self.asks) {
            match a {
                Ask::Copy(t) => host.set_clipboard(&t),
                Ask::OpenUrl(u) => {
                    if !host.open_url(&u) {
                        eprintln!("uison-ui: can't open {u}");
                    }
                }
                Ask::Paste => host.request_paste(),
                Ask::PickFile(action) => {
                    if !host.pick_file(&action) {
                        unsupported.push(action);
                    }
                }
            }
        }
        for action in unsupported {
            self.call(&action, "unsupported", "");
        }
    }

    fn run_derives(&mut self) {
        for d in &mut self.derives {
            d(&mut self.data);
        }
    }

    /// The meaning of the screen (`sem.rs`): nodes with a role, a name, a state, a `key` and a rectangle
    /// on screen, as a JSON array — for agents and tests that read the screen without seeing it.
    /// From the last frame; the first call builds one and from then on frames keep their tree.
    pub fn semantics(&mut self, host: &dyn Host) -> Json {
        if self.laid.is_none() {
            self.laid = Some(Vec::new());
            self.render(host);
        }
        let s = if self.safe_area { self.safe } else { Insets::default() };
        let roots = self.laid.as_deref().unwrap_or(&[]);
        Json::Array(crate::sem::build(roots, Point::new(s.left, s.top), &self.data, self.st.focus.as_deref()))
    }

    /// Names of the actions with handlers (`ui.on`), sorted — what the program can do by name.
    pub fn actions(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self.handlers.keys().map(String::as_str).collect();
        v.sort_unstable();
        v
    }

    /// Current value at a path — handy in tests and debugging.
    pub fn value(&self, path: &str) -> String {
        to_text(&self.data.get(path).cloned().unwrap_or(Json::Null))
    }
}

impl App for Ui {
    fn frame(&mut self, host: &dyn Host) -> String {
        self.flush_asks(host);
        self.render(host)
    }

    fn on_event(&mut self, e: &Event, hit: Option<&str>, host: &dyn Host) -> Option<String> {
        let version = self.data.version();
        let changed = self.handle_event(e, hit, host);
        self.flush_asks(host);
        let keyboard = self.data.bool("env.touch") && self.focused_input();
        if self.data.bool("env.keyboard") != keyboard {
            self.data.set("env.keyboard", keyboard);
        }
        let data_changed = self.data.version() != version;
        if data_changed {
            self.run_derives();
        }
        (changed || data_changed).then(|| self.render(host))
    }

    fn set_waker(&mut self, wake: uison_core::Waker) {
        let pending = !self.shared.queue.lock().unwrap().is_empty();
        *self.shared.wake.lock().unwrap() = Some(wake.clone());
        // Queued before the window started — run right after it.
        if pending {
            wake();
        }
    }

    fn wants_keyboard(&self) -> Option<uison_core::Keyboard> {
        let i = self.st.focus.as_deref().and_then(|f| self.index(f))?;
        self.infos[i].input.as_ref().map(|inp| inp.keyboard)
    }

    fn wants_tick(&self) -> Option<Duration> {
        let now = now();
        let timer = self.timers.iter().map(|(t, _)| t.saturating_sub(now)).min();
        let caret = self.focused_input().then(|| (self.caret_toggled + CARET_BLINK).saturating_sub(now));
        let fling = (self.fling.is_some() || self.spring.is_some()).then_some(FLING_TICK);
        let hold = self.touch.as_ref().and_then(Touch::hold_at).map(|t| t.saturating_sub(now));
        let tip = self.tip.as_ref().filter(|t| !t.shown).map(|t| (t.since + TOOLTIP_DELAY).saturating_sub(now));
        #[cfg(all(any(debug_assertions, feature = "reload"), not(target_arch = "wasm32")))]
        let tip = tip.into_iter().chain((!self.watched.is_empty()).then(|| (self.reload_at + crate::reload::POLL).saturating_sub(now))).min();
        timer.into_iter().chain(caret).chain(fling).chain(hold).chain(tip).min()
    }
}

/// Node of a popup component (Tooltip, ContextMenu) without parameters.
fn popup_node(name: &str) -> Json {
    let mut o = Json::object();
    o.set(name, Json::object());
    o
}

/// Position of a popup of size `sz` at point `at`: if it doesn't fit below — top at `above - sz.h`;
/// pressed against the edges of the safe area.
fn place(at: Point, above: f32, sz: Size, safe: uison_core::Rect) -> Point {
    let m = POPUP_MARGIN;
    let x = at.x.min(safe.right() - m - sz.w).max(safe.x + m);
    let y = if at.y + sz.h <= safe.bottom() - m { at.y } else { (above - sz.h).max(safe.y + m) };
    Point::new(x, y.min(safe.bottom() - m - sz.h).max(safe.y + m))
}

#[cfg(all(any(debug_assertions, feature = "reload"), not(target_arch = "wasm32")))]
/// Reload error banner: the previous screen stays, the error is on top (at the top of the window).
fn error_banner(msg: &str) -> Json {
    let mut text = Json::object();
    text.set("text", Json::from(format!("Error in the UI description (previous screen kept): {msg}")));
    text.set("color", Json::from("#FFFFFF"));
    text.set("size", Json::from(12.0));
    let mut t = Json::object();
    t.set("Text", text);
    let mut b = Json::object();
    b.set("padding", Json::Array(vec![Json::from(8.0), Json::from(12.0)]));
    b.set("radius", Json::from(6.0));
    b.set("fill", Json::from("#B3261EF2"));
    b.set("child", t);
    let mut node = Json::object();
    node.set("Box", b);
    node
}

