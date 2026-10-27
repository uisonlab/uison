//! Disk Usage on uison: where the disk space goes, as a sunburst chart.
//!
//! A port of the egui app (github.com/MarketKernel/disk-usage): the scanner, the tree and the chart
//! layout are the same; panels, list, menus and tooltips are a uison screen (`app.json`,
//! `components.json`), the chart is drawn by code in a `Canvas` node. State lives in `State` behind
//! `Rc<RefCell>`: handlers and drawing functions share it; only ready-made strings and colors
//! go into the screen data (`Data`).

mod colors;
mod format;
mod i18n;
mod platform;
mod queue;
pub mod scan;
mod sunburst;
mod tree;

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::Ordering::Relaxed;
use std::time::Duration;

use uison::prelude::*;
use uison::{Point, Size};

pub use crate::i18n::Lang;
use crate::queue::TrashQueue;
use crate::scan::{Outcome, Scan};
use crate::sunburst::{Highlight, Hit, Item, Layout, Transition};
use crate::tree::{NodeId, Tree};

/// Double click on a row: the first click already changed the list, the second lands on another row.
const ROW_DOUBLE: f64 = 0.4;

fn now() -> f64 {
    uison::time::now().as_secs_f64()
}

#[derive(Default)]
struct State {
    lang: Lang,
    tree: Option<Tree>,
    scan: Option<Scan>,
    /// Error count and the time of the last scan.
    stats: Option<(u64, Duration)>,
    /// Free and total space on the volume.
    disk: Option<(u64, u64)>,
    /// The node in the center of the chart.
    view: NodeId,
    selected: Option<NodeId>,
    layout: Option<Layout>,
    transition: Option<Transition>,
    /// What is under the cursor on the chart and where the cursor is (node coordinates).
    chart_hit: Option<Hit>,
    chart_pointer: Option<Point>,
    chart_size: Size,
    /// List or queue row under the cursor: highlights its sector.
    list_hover: Option<NodeId>,
    /// Whom the context menu (`@menu`) is for: what is under the cursor while the menu is closed.
    menu_target: Option<NodeId>,
    last_row_click: Option<(NodeId, f64)>,
    queue: TrashQueue,
    about: bool,
    message: Option<String>,
    quick: Vec<PathBuf>,
    /// Folders counted and when the number last changed, to notice that the scanner is waiting.
    scan_seen: (u64, f64),
    /// List rows: index by id, and the folder whose file is selected.
    rows: HashMap<NodeId, usize>,
    queue_rows: HashMap<NodeId, usize>,
    /// The folder dialog is open (Linux: it runs in the background, the window stays alive).
    picking: bool,
    marked: Option<NodeId>,
}

#[derive(Data, Default)]
struct Row {
    id: u32,
    name: String,
    size: String,
    pct: String,
    icon: String,
    color: String,
    tint: String,
    frac: String,
    fg: String,
    bg: Option<String>,
}

#[derive(Data, Default)]
struct QueueRow {
    id: u32,
    name: String,
    size: String,
    folder: String,
    icon: String,
    bg: Option<String>,
}

#[derive(Data, Default)]
struct Crumb {
    id: u32,
    name: String,
    last: bool,
    sep: bool,
}

#[derive(Data, Default)]
struct Quick {
    i: u32,
    path: String,
}

/// Number at the end of a key: "row-17" → 17.
fn key_id(key: &str) -> Option<u32> {
    key.rsplit('-').next()?.parse().ok()
}

impl State {
    // ------------------------------------------------------------ scanning

    fn start_scan(&mut self, cx: &mut Cx, path: PathBuf) {
        if let Some(scan) = self.scan.take() {
            scan.cancel();
        }
        let path = std::path::absolute(&path).unwrap_or(path);
        self.message = None;
        self.scan_seen = (0, now());
        self.scan = Some(Scan::start(path));
        cx.timer(16, "scanTick");
        self.sync(cx);
    }

    /// Scanning screen tick: counters, spinner, result.
    fn scan_tick(&mut self, cx: &mut Cx) {
        let Some(scan) = &self.scan else { return };
        let Some(done) = scan.poll() else {
            let p = &scan.progress;
            let dirs = p.dirs.load(Relaxed);
            let t = now();
            if dirs != self.scan_seen.0 {
                self.scan_seen = (dirs, t);
            }
            let d = &mut *cx.data;
            let lang = self.lang;
            d.set("scan.title", lang.scanning(&scan.root));
            let secs = scan.started.elapsed().as_secs();
            d.set("scan.counts", lang.scan_counts(p.files.load(Relaxed), dirs, p.bytes.load(Relaxed), secs));
            if let Ok(cur) = p.current.try_lock() {
                d.set("scan.current", cur.to_string_lossy().into_owned());
            }
            // A protected folder is waiting for an answer to the system prompt.
            d.set("scan.stall", if t - self.scan_seen.1 > 3.0 { lang.stall_hint() } else { "" });
            cx.timer(16, "scanTick");
            return;
        };
        let root = self.scan.take().map(|s| s.root).unwrap_or_default();
        match done.outcome {
            Outcome::Done(tree) => self.install(tree, done.errors, done.elapsed),
            Outcome::Cancelled => self.message = Some(self.lang.scan_cancelled().into()),
            Outcome::Unreadable(e) => self.message = Some(self.lang.cannot_read(&root, &e)),
            Outcome::Crashed => self.message = Some(self.lang.scanner_crashed().into()),
        }
        self.sync(cx);
    }

    fn install(&mut self, tree: Tree, errors: u64, elapsed: Duration) {
        self.disk = platform::disk_space(tree.root_path());
        self.stats = Some((errors, elapsed));
        if let Some(old) = self.tree.replace(tree) {
            // Millions of nodes aren't freed instantly, so not on the UI thread.
            std::thread::spawn(move || drop(old));
        }
        // Node ids of the old tree must not leak into the new one.
        self.layout = None;
        self.transition = None;
        self.view = Tree::ROOT;
        self.selected = None;
        self.chart_hit = None;
        self.list_hover = None;
        self.menu_target = None;
        self.last_row_click = None;
        self.queue.clear();
    }

    // ------------------------------------------------------------ actions

    fn select(&mut self, node: Option<NodeId>) {
        self.selected = node.filter(|&n| n != self.view);
    }

    fn zoom(&mut self, node: NodeId) {
        let Some(tree) = &self.tree else { return };
        if tree.node(node).is_dir() {
            // Going out: the folder we came from stays selected.
            let came_from = tree.is_within(self.view, node).then_some(self.view);
            self.view = node;
            self.selected = came_from.filter(|&n| n != node);
        } else if let Some(parent) = tree.parent(node) {
            self.view = parent;
            self.selected = Some(node);
        }
    }

    fn up(&mut self) {
        let Some(tree) = &self.tree else { return };
        if let Some(parent) = tree.parent(self.view) {
            self.selected = Some(self.view);
            self.view = parent;
        }
    }

    /// Select a node; if it isn't visible on the chart, go to its folder.
    fn locate(&mut self, node: NodeId) {
        let Some(tree) = &self.tree else { return };
        if self.layout.as_ref().is_none_or(|l| l.sector_of(node).is_none()) {
            self.view = tree.parent(node).unwrap_or(Tree::ROOT);
        }
        self.select(Some(node));
    }

    fn path(&self, node: NodeId) -> Option<PathBuf> {
        self.tree.as_ref().map(|t| t.path(node))
    }

    fn toggle_queue(&mut self, node: NodeId) {
        let Some(tree) = &self.tree else { return };
        if self.queue.contains(node) {
            self.queue.remove(node);
        } else {
            self.queue.add(tree, node);
        }
    }

    /// Everything in the queue goes to the Trash; whatever fails stays in the queue.
    fn trash_queued(&mut self) {
        let Some(tree) = &mut self.tree else { return };
        let (mut moved, mut freed, mut failed) = (Vec::new(), 0, Vec::new());
        for node in self.queue.items().to_vec() {
            let path = tree.path(node);
            match platform::move_to_trash(&path) {
                Ok(()) => {
                    freed += tree.node(node).size;
                    tree.remove(node);
                    self.queue.remove(node);
                    if tree.is_within(self.view, node) {
                        self.view = tree.parent(node).unwrap_or(Tree::ROOT);
                    }
                    if self.selected.is_some_and(|s| tree.is_within(s, node)) {
                        self.selected = None;
                    }
                    moved.push(path);
                }
                Err(e) => failed.push(self.lang.trash_failed(&path, &e)),
            }
        }
        if !moved.is_empty() {
            self.layout = None;
            self.disk = platform::disk_space(tree.root_path());
        }
        let mut parts = Vec::new();
        match moved.as_slice() {
            [] => {}
            [path] => parts.push(self.lang.trashed_one(path, freed)),
            _ => parts.push(self.lang.trashed_many(moved.len() as u64, freed)),
        }
        if let Some(first) = failed.first() {
            parts.push(match failed.len() {
                1 => first.clone(),
                n => self.lang.and_more(first, n as u64 - 1),
            });
        }
        self.message = Some(parts.join("  ·  "));
    }

    // ------------------------------------------------------------ screen data

    /// Layout for the current view; if the view changed, a zoom animation (`true`: ticks needed).
    fn ensure_layout(&mut self) -> bool {
        let Some(tree) = &self.tree else { return false };
        if self.layout.as_ref().is_some_and(|l| l.root == self.view) {
            return false;
        }
        let layout = Layout::compute(tree, self.view);
        self.transition = self.layout.take().and_then(|old| Transition::new(tree, old, &layout, now()));
        self.layout = Some(layout);
        self.chart_hit = None;
        self.transition.is_some()
    }

    fn color_of(&self, node: NodeId) -> uison::Color {
        let (Some(tree), Some(layout)) = (&self.tree, &self.layout) else { return colors::SMALL };
        let c = layout.sector_of(node).map_or(colors::SMALL, |s| s.color);
        if self.queue.covers(tree, node) { colors::queued(c) } else { c }
    }

    /// Everything the screen shows, after any action.
    fn sync(&mut self, cx: &mut Cx) {
        if self.ensure_layout() {
            cx.timer(16, "anim");
        }
        let d = &mut *cx.data;
        let mode = if self.scan.is_some() {
            "scan"
        } else if self.tree.is_some() {
            "view"
        } else {
            "start"
        };
        d.set("mode", mode);
        d.set("about", self.about);
        d.set("message", self.message.clone().unwrap_or_default());
        d.set("canRescan", self.tree.is_some() && self.scan.is_none());
        let Some(tree) = self.tree.as_ref().filter(|_| self.scan.is_none()) else {
            d.set("canUp", false);
            d.set("crumbs", Vec::<Crumb>::new());
            d.set("status", "");
            d.set("queue.count", 0);
            return;
        };
        d.set("canUp", self.view != Tree::ROOT);

        let chain = tree.ancestry(self.view);
        let crumbs: Vec<Crumb> = chain
            .iter()
            .enumerate()
            .map(|(i, &n)| Crumb { id: n, name: tree.display_name(n).into_owned(), last: i + 1 == chain.len(), sep: i > 0 })
            .collect();
        d.set("crumbs", crumbs);

        let lang = self.lang;
        let root = tree.node(Tree::ROOT);
        let mut parts = vec![lang.files(root.files.into()), format::bytes(root.size, lang)];
        if let Some((errors, elapsed)) = self.stats {
            if errors > 0 {
                parts.push(lang.unreadable(errors));
            }
            parts.push(lang.scanned_in(elapsed));
        }
        if let Some((free, total)) = self.disk {
            parts.push(lang.disk_free(free, total));
        }
        d.set("status", parts.join("  ·  "));

        // Panel title: the selection or the view itself.
        let target = self.selected.unwrap_or(self.view);
        let node = tree.node(target);
        let mut facts = Vec::new();
        if target != self.view {
            let share = format::percent(node.size, tree.node(self.view).size, lang);
            facts.push(lang.share_of(&share, &tree.short_name(self.view)));
        }
        if node.is_dir() {
            facts.push(lang.files(node.files.into()));
        }
        let covered = self.queue.covers(tree, target) && !self.queue.contains(target);
        d.set("sel.name", tree.short_name(target).into_owned());
        d.set("sel.color", colors::hex(self.color_of(target)));
        d.set("sel.size", format::bytes(node.size, lang));
        d.set("sel.facts", facts.join("  ·  "));
        d.set("sel.path", tree.path(target).to_string_lossy().into_owned());
        d.set("sel.canZoom", node.is_dir() && target != self.view);
        d.set("sel.canQueue", target != Tree::ROOT && !self.queue.contains(target));
        d.set("sel.covered", covered);
        d.set("sel.queued", self.queue.contains(target));
        d.set("sel.queueTip", lang.queue_tip(covered));

        // List: contents of the selected folder; for a file, its folder with the file highlighted.
        let (folder, marked) =
            if node.is_dir() { (target, None) } else { (tree.parent(target).unwrap_or(target), Some(target)) };
        self.marked = marked;
        let ids = tree.sorted_children(folder);
        let max = ids.first().map_or(1, |&c| tree.node(c).size.max(1));
        let total = tree.node(folder).size;
        self.rows = ids.iter().enumerate().map(|(i, &id)| (id, i)).collect();
        let rows: Vec<Row> = ids
            .iter()
            .map(|&id| {
                let n = tree.node(id);
                let color = self.color_of(id);
                let permille = n.size * 1000 / max;
                let (fg, bg) = self.row_look(id);
                Row {
                    id,
                    name: n.name_lossy().into_owned(),
                    size: format::bytes(n.size, lang),
                    pct: format::percent(n.size, total, lang),
                    icon: if n.is_dir() { "folder" } else { "file" }.into(),
                    color: colors::hex(color),
                    tint: colors::hex(colors::alpha(color, 0.18)),
                    frac: format!("{}.{}%", permille / 10, permille % 10),
                    fg,
                    bg,
                }
            })
            .collect();
        d.set("rows", rows);
        d.set("emptyText", if ids.is_empty() && node.is_dir() { lang.empty_folder() } else { "" });

        let items = self.queue.items();
        self.queue_rows = items.iter().enumerate().map(|(i, &id)| (id, i)).collect();
        let qrows: Vec<QueueRow> = items
            .iter()
            .map(|&id| {
                let n = tree.node(id);
                QueueRow {
                    id,
                    name: n.name_lossy().into_owned(),
                    size: format::bytes(n.size, lang),
                    folder: tree.parent(id).map(|p| tree.path(p).to_string_lossy().into_owned()).unwrap_or_default(),
                    icon: if n.is_dir() { "folder" } else { "file" }.into(),
                    bg: self.queue_look(id),
                }
            })
            .collect();
        d.set("queue.count", items.len());
        d.set("queue.size", format::bytes(self.queue.size(tree), lang));
        d.set("queue.label", lang.items(items.len() as u64));
        d.set("queue.items", qrows);
        let target = self.menu_target.filter(|&n| (n as usize) < tree.len()).unwrap_or(self.view);
        d.set("menu", self.menu_for(target));
    }

    /// What the sector under the cursor highlights in the lists.
    fn chart_node(&self) -> Option<NodeId> {
        match self.chart_hit {
            Some(Hit::Sector(item)) => Some(item.node()),
            _ => None,
        }
    }

    /// Row text color and background: the selected file and the one hovered on the chart are brighter.
    fn row_look(&self, id: NodeId) -> (String, Option<String>) {
        let active = self.marked == Some(id) || self.chart_node() == Some(id);
        let queued = self.tree.as_ref().is_some_and(|t| self.queue.covers(t, id));
        let fg = match (active, queued) {
            (true, _) => "#FFFFFF",
            (false, true) => "#9A9BA3",
            (false, false) => "#E8E8EC",
        };
        (fg.into(), active.then(|| colors::hex(colors::PANEL)))
    }

    fn queue_look(&self, id: NodeId) -> Option<String> {
        (self.chart_node() == Some(id)).then(|| colors::hex(colors::PANEL))
    }

    /// Hover on the chart changed: recolor only the affected rows.
    fn set_chart_hit(&mut self, d: &mut Data, hit: Option<Hit>) {
        let before = self.chart_node();
        self.chart_hit = hit;
        let after = self.chart_node();
        if before == after {
            return;
        }
        for id in [before, after].into_iter().flatten() {
            if let Some(&i) = self.rows.get(&id) {
                let (fg, bg) = self.row_look(id);
                d.set(&format!("rows.{i}.fg"), fg);
                d.set(&format!("rows.{i}.bg"), bg);
            }
            if let Some(&i) = self.queue_rows.get(&id) {
                d.set(&format!("queue.items.{i}.bg"), self.queue_look(id));
            }
        }
    }

    /// Menu for the node under the cursor; while the menu is open the target doesn't change.
    fn set_menu_target(&mut self, d: &mut Data, node: NodeId) {
        let open = d.get("env.menu").is_some_and(|m| !m.is_null());
        if open || self.menu_target == Some(node) {
            return;
        }
        self.menu_target = Some(node);
        d.set("menu", self.menu_for(node));
    }

    fn menu_for(&self, node: NodeId) -> Json {
        let Some(tree) = &self.tree else { return Json::Array(Vec::new()) };
        let item = |label: &str, action: &str| obj([("label", Json::from(label.to_string())), ("action", Json::from(action.to_string()))]);
        let off = |label: String| obj([("label", Json::from(label)), ("action", Json::from(String::new())), ("disabled", Json::from(true))]);
        let mut items = vec![off(format::ellipsize(&tree.display_name(node), 48)), Json::from("-".to_string())];
        let lang = self.lang;
        if tree.node(node).is_dir() && node != self.view {
            items.push(item(lang.menu_zoom(), "mZoom"));
        }
        items.push(item(lang.show_in(), "mReveal"));
        items.push(item(lang.menu_open(), "mOpen"));
        items.push(item(lang.menu_copy(), "mCopy"));
        if node != Tree::ROOT {
            items.push(Json::from("-".to_string()));
            if self.queue.contains(node) {
                items.push(item(lang.menu_unqueue(), "mUnqueue"));
            } else if self.queue.covers(tree, node) {
                items.push(off(lang.menu_covered().into()));
            } else {
                items.push(item(lang.menu_queue(), "mQueue"));
            }
        }
        Json::Array(items)
    }

    // ------------------------------------------------------------ chart events

    fn chart_hover(&mut self, cx: &mut Cx) {
        let hit = match (cx.pos, &self.layout) {
            (Some(p), Some(layout)) => sunburst::hit(layout, self.chart_size, p),
            _ => None,
        };
        self.chart_pointer = cx.pos;
        self.set_chart_hit(cx.data, hit);
        let target = match hit {
            Some(Hit::Sector(item)) => item.node(),
            _ => self.view,
        };
        if cx.pos.is_some() {
            self.set_menu_target(cx.data, target);
        }
    }

    fn chart_click(&mut self, cx: &mut Cx) {
        let Some(layout) = &self.layout else { return };
        let hit = cx.pos.and_then(|p| sunburst::hit(layout, self.chart_size, p));
        let tree = self.tree.as_ref().unwrap();
        match hit {
            Some(Hit::Sector(Item::Node(n))) if cx.clicks >= 2 => {
                if tree.node(n).is_dir() {
                    self.zoom(n);
                }
            }
            // Like Escape: first clear the selection, then go up.
            Some(Hit::Center) if self.selected.is_some() => self.select(None),
            Some(Hit::Center) => self.up(),
            Some(Hit::Sector(item)) => self.select(Some(item.node())),
            None => self.select(None),
        }
        self.sync(cx);
    }

    fn row_click(&mut self, cx: &mut Cx) {
        let Some(id) = key_id(cx.key) else { return };
        let t = now();
        let dir = |n: NodeId| self.tree.as_ref().is_some_and(|tr| tr.node(n).is_dir());
        match self.last_row_click {
            Some((prev, at)) if t - at < ROW_DOUBLE && dir(prev) => {
                self.zoom(prev);
                self.last_row_click = None;
            }
            _ => {
                self.select(Some(id));
                self.last_row_click = Some((id, t));
            }
        }
        self.sync(cx);
    }

    fn list_hover(&mut self, cx: &mut Cx) {
        let Some(id) = key_id(cx.key) else { return };
        if cx.pos.is_some() {
            self.list_hover = Some(id);
            self.set_menu_target(cx.data, id);
        } else if self.list_hover == Some(id) {
            self.list_hover = None;
        }
    }

    /// Target of a menu item: the row it was opened on, or what was under the cursor.
    fn menu_node(&self, cx: &Cx) -> Option<NodeId> {
        if cx.key.starts_with("row-") || cx.key.starts_with("q-") {
            return key_id(cx.key);
        }
        self.menu_target.or(Some(self.view))
    }
}

type Shared = Rc<RefCell<State>>;

/// Action handler: state + context.
fn on(ui: &mut Ui, s: &Shared, action: &str, f: impl Fn(&mut State, &mut Cx) + 'static) {
    let s = s.clone();
    ui.on(action, move |cx| f(&mut s.borrow_mut(), cx));
}

/// A simple action: change the state and update the screen.
fn act(ui: &mut Ui, s: &Shared, action: &str, f: impl Fn(&mut State, &mut Cx) + 'static) {
    on(ui, s, action, move |st, cx| {
        f(st, cx);
        st.sync(cx);
    });
}

/// The screen with all handlers; `path` is a folder to scan right away.
pub fn app(path: Option<PathBuf>, lang: Lang) -> Ui {
    build(path, false, lang)
}

/// For the console mode (`--console`): the system folder dialog is hidden — a process without a
/// window can't show it; quick folders and a folder from the command line remain.
pub fn app_console(path: Option<PathBuf>, lang: Lang) -> Ui {
    let mut ui = build(path, false, lang);
    ui.data.set("console", true);
    ui
}

/// The same, but the folder is already scanned (synchronously), for headless screenshots and measurements.
pub fn app_scanned(path: PathBuf, lang: Lang) -> Ui {
    build(Some(path), true, lang)
}

fn build(path: Option<PathBuf>, sync_scan: bool, lang: Lang) -> Ui {
    let mut ui = Ui::load(uison::ui::source!("../app.json")).unwrap_or_else(|e| panic!("app.json: {e}"));
    ui.load_components(uison::ui::source!("../components.json")).unwrap_or_else(|e| panic!("components.json: {e}"));
    ui.set_theme("dark");

    let mut quick: Vec<PathBuf> = platform::home_dir().into_iter().collect();
    quick.extend(platform::volumes());
    let rows: Vec<Quick> =
        quick.iter().enumerate().map(|(i, p)| Quick { i: i as u32, path: p.to_string_lossy().into_owned() }).collect();
    ui.data.set("quick", rows);
    ui.data.set("lang", lang.code());
    lang.apply(&mut ui.data);
    let s: Shared = Rc::new(RefCell::new(State { lang, quick, ..State::default() }));

    // Scanning.
    // macOS and Windows dialogs are modal to the window. On Linux zenity is another program: waiting for it here
    // would freeze the window, and GNOME offers to force quit it after a few seconds — wait in a thread instead.
    let (handle, picked) = (ui.handle(), std::sync::Arc::new(std::sync::Mutex::new(None)));
    let slot = picked.clone();
    on(&mut ui, &s, "open", move |st, cx| {
        if st.picking {
            return;
        }
        if cfg!(target_os = "linux") {
            st.picking = true;
            let (title, handle, slot) = (st.lang.choose_title(), handle.clone(), slot.clone());
            std::thread::spawn(move || {
                *slot.lock().unwrap() = platform::pick_folder(title);
                handle.post(|cx| cx.timer(0, "picked"));
            });
        } else if let Some(p) = platform::pick_folder(st.lang.choose_title()) {
            st.start_scan(cx, p);
        }
    });
    on(&mut ui, &s, "picked", move |st, cx| {
        st.picking = false;
        if let Some(p) = picked.lock().unwrap().take() {
            st.start_scan(cx, p);
        }
    });
    on(&mut ui, &s, "quick", |st, cx| {
        if let Some(p) = key_id(cx.key).and_then(|i| st.quick.get(i as usize).cloned()) {
            st.start_scan(cx, p);
        }
    });
    on(&mut ui, &s, "rescan", |st, cx| {
        if let Some(root) = st.tree.as_ref().filter(|_| st.scan.is_none()).map(|t| t.root_path().to_path_buf()) {
            st.start_scan(cx, root);
        }
    });
    on(&mut ui, &s, "cancel", |st, _| {
        if let Some(scan) = &st.scan {
            scan.cancel();
        }
    });
    on(&mut ui, &s, "scanTick", State::scan_tick);
    on(&mut ui, &s, "anim", |st, cx| match &st.transition {
        Some(t) if !t.finished(now()) => cx.timer(16, "anim"),
        _ => st.transition = None,
    });

    // Chart and lists.
    on(&mut ui, &s, "chartHover", State::chart_hover);
    on(&mut ui, &s, "chartClick", State::chart_click);
    on(&mut ui, &s, "row", State::row_click);
    on(&mut ui, &s, "rowHover", State::list_hover);
    on(&mut ui, &s, "queueHover", State::list_hover);
    act(&mut ui, &s, "locate", |st, cx| {
        if let Some(id) = key_id(cx.key) {
            st.locate(id);
        }
    });
    act(&mut ui, &s, "crumb", |st, cx| {
        if let Some(id) = key_id(cx.key).filter(|&id| id != st.view) {
            st.zoom(id);
        }
    });
    act(&mut ui, &s, "up", |st, _| st.up());

    // Selection panel.
    let target = |st: &State| st.selected.unwrap_or(st.view);
    act(&mut ui, &s, "zoomSel", move |st, _| st.zoom(target(st)));
    act(&mut ui, &s, "revealSel", move |st, _| {
        if let Some(p) = st.path(target(st)) {
            platform::reveal(&p);
        }
    });
    act(&mut ui, &s, "queueSel", move |st, _| {
        if let Some(tree) = &st.tree {
            st.queue.add(tree, target(st));
        }
    });
    act(&mut ui, &s, "unqueueSel", move |st, _| st.queue.remove(target(st)));

    // Trash queue.
    act(&mut ui, &s, "unqueue", |st, cx| {
        if let Some(id) = key_id(cx.key) {
            st.queue.remove(id);
        }
    });
    act(&mut ui, &s, "clearQueue", |st, _| st.queue.clear());
    act(&mut ui, &s, "trashQueued", |st, _| st.trash_queued());

    // Context menu.
    act(&mut ui, &s, "mZoom", |st, cx| {
        if let Some(n) = st.menu_node(cx) {
            st.zoom(n);
        }
    });
    act(&mut ui, &s, "mReveal", |st, cx| {
        if let Some(p) = st.menu_node(cx).and_then(|n| st.path(n)) {
            platform::reveal(&p);
        }
    });
    act(&mut ui, &s, "mOpen", |st, cx| {
        if let Some(p) = st.menu_node(cx).and_then(|n| st.path(n)) {
            platform::open(&p);
        }
    });
    act(&mut ui, &s, "mCopy", |st, cx| {
        if let Some(p) = st.menu_node(cx).and_then(|n| st.path(n)) {
            cx.copy(&p.to_string_lossy());
        }
    });
    act(&mut ui, &s, "mQueue", |st, cx| {
        if let (Some(n), Some(tree)) = (st.menu_node(cx), &st.tree) {
            st.queue.add(tree, n);
        }
    });
    act(&mut ui, &s, "mUnqueue", |st, cx| {
        if let Some(n) = st.menu_node(cx) {
            st.queue.remove(n);
        }
    });

    // Language switch ("lang-ru"): the screen texts here, the ones made in code by `sync`.
    act(&mut ui, &s, "lang", |st, cx| {
        st.lang = Lang::from_code(cx.key.rsplit('-').next().unwrap_or_default());
        cx.data.set("lang", st.lang.code());
        st.lang.apply(cx.data);
    });

    // About and keys.
    act(&mut ui, &s, "about", |st, _| st.about = true);
    act(&mut ui, &s, "closeAbout", |st, _| st.about = false);
    act(&mut ui, &s, "escape", |st, _| {
        if st.about {
            st.about = false;
        } else if st.selected.is_some() {
            st.select(None);
        } else {
            st.up();
        }
    });
    act(&mut ui, &s, "keyUp", |st, _| {
        if !st.about {
            st.up();
        }
    });
    act(&mut ui, &s, "keyZoom", |st, _| {
        if let Some(sel) = st.selected.filter(|_| !st.about) {
            st.zoom(sel);
        }
    });
    act(&mut ui, &s, "keyQueue", |st, _| {
        if let Some(sel) = st.selected.filter(|_| !st.about) {
            st.toggle_queue(sel);
        }
    });
    ui.shortcut("cmd+o", "open");
    ui.shortcut("cmd+r", "rescan");
    ui.shortcut("escape", "escape");
    ui.shortcut("backspace", "keyUp");
    ui.shortcut("enter", "keyZoom");
    ui.shortcut("delete", "keyQueue");
    ui.shortcut("cmd+backspace", "keyQueue");

    // Drawing: the chart and the scanning spinner.
    let st = s.clone();
    ui.draw("chart", move |d| {
        let st = &mut *st.borrow_mut();
        st.chart_size = Size::new(d.rect.w, d.rect.h);
        let (Some(tree), Some(layout)) = (&st.tree, &st.layout) else { return };
        let hovered = match st.chart_hit {
            Some(Hit::Sector(item)) => Some(item),
            _ => st.list_hover.map(Item::Node),
        };
        let hl = Highlight {
            selected: st.selected,
            hovered,
            hit: st.chart_hit,
            pointer: st.chart_pointer,
            queue: &st.queue,
            lang: st.lang,
        };
        sunburst::draw(d.w, d.m, d.rect, tree, layout, st.transition.as_ref(), &hl, now());
    });
    ui.draw("spinner", |d| sunburst::spinner(d.w, d.rect, now()));

    if let Some(path) = path {
        let s = s.clone();
        ui.on("~start", move |cx| {
            let st = &mut *s.borrow_mut();
            if !sync_scan {
                return st.start_scan(cx, path.clone());
            }
            let t = std::time::Instant::now();
            let progress = scan::Progress::default();
            match scan::scan(&path, &progress) {
                Ok(tree) => st.install(tree, progress.errors.load(Relaxed), t.elapsed()),
                Err(e) => st.message = Some(st.lang.cannot_read(&path, &e)),
            }
            st.sync(cx);
        });
        ui.run_action("~start");
    }
    ui
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has(ui: &mut Ui, text: &str) -> bool {
        ui.texts().iter().any(|t| t.contains(text))
    }

    #[test]
    fn switches_language_on_the_fly() {
        let mut ui = app(None, Lang::En);
        assert!(has(&mut ui, "Choose folder…"));
        assert!(ui.tap("lang-ru"));
        assert!(has(&mut ui, "Выбрать папку…"), "{:?}", ui.texts());
        assert!(ui.tap("lang-en"));
        assert!(has(&mut ui, "Choose folder…"));
    }

    #[test]
    fn scanned_view_in_russian() {
        let dir = std::env::temp_dir().join(format!("disk-usage-lang-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("a.bin"), vec![1u8; 4096]).unwrap();
        std::fs::write(dir.join("sub/b.bin"), vec![2u8; 4096]).unwrap();
        let mut ui = app_scanned(dir.clone(), Lang::Ru);
        // Status bar, panel and chart legend: plural forms and Russian units.
        assert!(has(&mut ui, "2 файла"), "{:?}", ui.texts());
        assert!(has(&mut ui, "КБ"));
        assert!(has(&mut ui, "доля в текущей папке"));
        // Back to English: the texts made in code follow.
        assert!(ui.tap("lang-en"));
        assert!(has(&mut ui, "2 files"));
        assert!(has(&mut ui, "share of view"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
