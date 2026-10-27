//! A tiny browser: HTML + CSS → uison screen JSON (see Task9.md).
//!
//! `browser [address]` — a window; `browser --png dir [address…]` — headless screenshots (macOS);
//! `browser --json address` — the page's JSON to stdout.

pub mod convert;
pub mod css;
pub mod html;
#[cfg(not(target_arch = "wasm32"))]
pub mod net;
pub mod pretty;
pub mod style;
#[cfg(not(target_arch = "wasm32"))]
pub mod tls;
pub mod url;
#[cfg(windows)]
mod winhttp;

use std::cell::RefCell;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use uison::time::now;
use uison::ui::{Cx, Data, Handle, Json, Tree, Ui};

use convert::{Page, hex};
use css::Sheet;
use html::Dom;
use style::Env;

pub const HOME: &str = "about:home";

/// Load from the network right away, without a thread (`--png` screenshots, `--json`).
pub static SYNC: AtomicBool = AtomicBool::new(false);

/// Built-in `about:` pages.
pub fn builtin(url: &str) -> Option<&'static str> {
    Some(match url {
        "about:home" | "about:" => include_str!("../pages/home.html"),
        "about:css" => include_str!("../pages/css.html"),
        "about:article" => include_str!("../pages/article.html"),
        "about:retro" => include_str!("../pages/retro.html"),
        "about:blank" => "",
        _ => return None,
    })
}

/// The browser's default styles — parsed once.
fn ua() -> &'static Sheet {
    static UA: OnceLock<Sheet> = OnceLock::new();
    UA.get_or_init(|| Sheet::parse(style::UA))
}

/// The page and how long the stages took (µs): HTML parsing, CSS parsing, cascade, conversion to JSON.
pub struct Rendered {
    pub page: Page,
    pub us: [u32; 4],
}

/// A page: final address, HTML and the texts of external style sheets (by `href` from the markup).
pub struct Source {
    pub url: String,
    pub html: String,
    pub css: Vec<(String, String)>,
}

/// The page's external style sheets: `<link rel=stylesheet href>` (not for print).
fn stylesheet_links(dom: &Dom) -> impl Iterator<Item = &str> {
    dom.nodes.iter().filter_map(|n| {
        let rel = n.attr("rel")?.to_ascii_lowercase();
        // `media` is checked during parsing (depends on the window); here only "print" is skipped.
        let print = n.attr("media").is_some_and(|m| m.trim().eq_ignore_ascii_case("print"));
        (n.tag() == Some("link") && rel.split_whitespace().any(|r| r == "stylesheet") && !print).then(|| n.attr("href")).flatten()
    })
}

pub fn render(src: &Source, env: Env, image_prefix: &str) -> Rendered {
    let t0 = now();
    let dom = Dom::parse(&src.html);
    let t1 = now();
    // Styles in document order: <link> (already loaded) and <style>.
    let sheets: Vec<Sheet> = (0..dom.nodes.len())
        .filter_map(|i| {
            let n = &dom.nodes[i];
            // `media="(prefers-color-scheme: dark)"` etc. — same as an @media condition.
            if !css::media_matches(n.attr("media").unwrap_or(""), env.width, env.dark) {
                return None;
            }
            match n.tag()? {
                "style" => Some(Sheet::parse(&dom.text(i))),
                "link" => {
                    let href = n.attr("href")?;
                    src.css.iter().find(|(h, _)| h == href).map(|(_, text)| Sheet::parse(text))
                }
                _ => None,
            }
        })
        .collect();
    let t2 = now();
    let st = style::compute(&dom, ua(), &sheets, env);
    let t3 = now();
    let page = convert::convert(&dom, &st, image_prefix);
    let t4 = now();
    let us = |a: std::time::Duration, b: std::time::Duration| (b - a).as_micros() as u32;
    Rendered { page, us: [us(t0, t1), us(t1, t2), us(t2, t3), us(t3, t4)] }
}

/// Address from the input line: `example.com` → `http://example.com`.
fn normalize(input: &str) -> String {
    let u = input.trim();
    if u.is_empty() {
        HOME.to_string()
    } else if !u.contains(':') && !u.starts_with('/') {
        format!("http://{u}")
    } else {
        u.to_string()
    }
}

fn is_network(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

/// Load a page right away: final address, HTML (an error is a page too) and its external style sheets.
pub fn fetch(url: &str) -> Source {
    let (url, html) = fetch_html(url);
    let dom = Dom::parse(&html);
    let hrefs: Vec<String> = stylesheet_links(&dom).map(str::to_string).collect();
    let css = parallel(&hrefs, |href| {
        let u = url::resolve(&url, href);
        let bytes = if is_network(&u) { fetch_bytes(&u) } else { read_file(&u).ok() }?;
        Some(String::from_utf8_lossy(&bytes).into_owned())
    });
    let css = hrefs.into_iter().zip(css).filter_map(|(h, c)| Some((h, c?))).collect();
    Source { url, html, css }
}

/// `f` for each item on several threads (network: dozens of style sheets and images one after
/// another take seconds, each with its own TLS handshake). Results keep the input order.
fn parallel<T: Sync, R: Send>(items: &[T], f: impl Fn(&T) -> Option<R> + Sync) -> Vec<Option<R>> {
    const THREADS: usize = 8;
    if items.len() < 2 || cfg!(target_arch = "wasm32") {
        return items.iter().map(&f).collect();
    }
    let next = std::sync::atomic::AtomicUsize::new(0);
    let out: Vec<std::sync::Mutex<Option<R>>> = items.iter().map(|_| std::sync::Mutex::new(None)).collect();
    std::thread::scope(|sc| {
        for _ in 0..THREADS.min(items.len()) {
            sc.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(it) = items.get(i) else { break };
                    *out[i].lock().unwrap() = f(it);
                }
            });
        }
    });
    out.into_iter().map(|m| m.into_inner().unwrap()).collect()
}

fn fetch_html(url: &str) -> (String, String) {
    let res: Result<(String, String), String> = if let Some(src) = builtin(url) {
        Ok((url.to_string(), src.to_string()))
    } else if url.starts_with("about:") {
        Err(format!("No such built-in page: {url}"))
    } else if is_network(url) {
        fetch_net(url)
    } else {
        read_file(url).map(|b| (url.to_string(), String::from_utf8_lossy(&b).into_owned()))
    };
    res.unwrap_or_else(|e| (url.to_string(), error_page(url, &e)))
}

#[cfg(not(target_arch = "wasm32"))]
fn fetch_net(url: &str) -> Result<(String, String), String> {
    let r = net::get(url)?;
    Ok((r.url, net::decode_html(&r.body, &r.content_type)))
}

#[cfg(target_arch = "wasm32")]
fn fetch_net(_: &str) -> Result<(String, String), String> {
    Err("No network in the web build: the page itself runs in a browser.".into())
}

#[cfg(not(target_arch = "wasm32"))]
fn read_file(url: &str) -> Result<Vec<u8>, String> {
    let path = url.strip_prefix("file://").unwrap_or(url);
    std::fs::read(path).map_err(|e| format!("{path}: {e}"))
}

#[cfg(target_arch = "wasm32")]
fn read_file(url: &str) -> Result<Vec<u8>, String> {
    Err(format!("Can't open {url}"))
}

/// A page with the error message — HTML too.
fn error_page(url: &str, msg: &str) -> String {
    let esc = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;");
    format!(
        "<title>Error</title><body style='font-family: sans-serif; margin: 48px; color: #333'>\
         <h1 style='font-size: 26px; color: #c92a2a'>Couldn't open the page</h1>\
         <p style='color: #666'>{}</p><p>{}</p><p><a href='about:home'>Back to start page</a></p></body>",
        esc(url),
        esc(msg)
    )
}

/// Navigation history and the current page. Lives on the UI thread: network responses arrive
/// there via `Handle::post` (their closures are `Send` and hold no reference to the state).
#[derive(Default)]
struct Browser {
    history: Vec<String>,
    pos: usize,
    links: Vec<String>,
    url: String,
    /// Display number: the scroll gets its own `key` for each, so a new page opens at the top;
    /// it is also the prefix of the page's images.
    page_no: usize,
    /// Load number: a response to a stale request (we left the page) is not shown.
    seq: u64,
    handle: Option<Handle>,
}

thread_local! {
    static B: RefCell<Browser> = RefCell::new(Browser::default());
}

fn with<R>(f: impl FnOnce(&mut Browser) -> R) -> R {
    B.with(|b| f(&mut b.borrow_mut()))
}

/// Open an address: right away (built-in, file) or in the background (network).
fn open(tree: &mut Tree, data: &mut Data, url: &str) {
    let url = normalize(url);
    let seq = with(|b| {
        b.seq += 1;
        b.seq
    });
    data.set("address", url::display(&url));
    if is_network(&url) && !SYNC.load(Ordering::Relaxed) && !cfg!(target_arch = "wasm32") {
        data.set("loading", true);
        data.set("status", format!("Loading {url}…"));
        let handle = with(|b| b.handle.clone()).expect("handle");
        let t0 = now();
        std::thread::spawn(move || {
            let src = fetch(&url);
            let load = (now() - t0).as_micros() as u32;
            handle.post(move |cx| {
                if with(|b| b.seq) == seq {
                    show(&mut cx.tree, cx.data, &src, load);
                }
            });
        });
        return;
    }
    let t0 = now();
    let src = fetch(&url);
    show(tree, data, &src, (now() - t0).as_micros() as u32);
}

fn show(tree: &mut Tree, data: &mut Data, src: &Source, load_us: u32) {
    let url = src.url.as_str();
    let env = Env { width: data.num("env.width") as f32, height: data.num("env.height") as f32, dark: false };
    let env = if env.width > 0.0 { env } else { Env { width: 1100.0, height: 800.0, dark: false } };
    let page_no = with(|b| {
        b.page_no += 1;
        b.page_no
    });
    let prefix = format!("p{page_no}_");
    let r = render(src, env, &prefix);
    if let Some(mut s) = tree.find(&format!("scroll{}", page_no - 1)) {
        s.set("key", format!("scroll{page_no}"));
    }
    if let Some(mut p) = tree.find("page") {
        *p.children() = vec![r.page.root.clone()];
    }
    if let Some(mut bg) = tree.find("pagebg") {
        bg.set("fill", hex(r.page.background.unwrap_or(uison::Color::WHITE)));
    }
    let (can_back, can_fwd) = with(|b| {
        b.links = r.page.links.clone();
        b.url = url.to_string();
        // A redirect may have changed the address.
        if let Some(h) = b.history.get_mut(b.pos) {
            *h = url.to_string();
        }
        (b.pos > 0, b.pos + 1 < b.history.len())
    });
    let json = pretty::pretty(&r.page.root);
    let lines = |s: &str| Json::Array(s.lines().map(|l| Json::from(l.to_string())).collect());
    data.set("json", lines(&json));
    data.set("source", lines(&src.html));
    data.set("address", url::display(url));
    data.set("loading", false);
    data.set("form", Json::object());
    for (path, v) in &r.page.fields {
        data.set(path, v.clone());
    }
    data.set("canBack", can_back);
    data.set("canForward", can_fwd);
    let kb = |n: usize| n.div_ceil(1024);
    let title = if r.page.title.is_empty() { url } else { &r.page.title };
    data.set(
        "status",
        format!(
            "{title} — HTML {} KB, CSS {} KB → JSON {} nodes ({} KB) · load {} ms, parse {} µs, CSS {} µs, cascade {} µs, convert {} µs",
            kb(src.html.len()),
            kb(src.css.iter().map(|c| c.1.len()).sum()),
            r.page.nodes,
            kb(json.len()),
            load_us / 1000,
            r.us[0],
            r.us[1],
            r.us[2],
            r.us[3]
        ),
    );
    load_images(url, &prefix, &r.page.images);
}

/// The page's images: `data:` right away, files and network in the background; a finished image goes
/// into the registry (`mem:<prefix><number>`) and the frame is rebuilt.
fn load_images(base: &str, prefix: &str, images: &[String]) {
    let mut later = Vec::new();
    for (i, src) in images.iter().enumerate() {
        let name = format!("{prefix}{i}");
        if let Some(bytes) = src.strip_prefix("data:").and_then(data_url) {
            uison::image::put(&name, bytes);
        } else if !src.is_empty() {
            later.push((name, url::resolve(base, src)));
        }
    }
    if later.is_empty() || cfg!(target_arch = "wasm32") {
        return;
    }
    let base = base.to_string();
    let job = move || {
        parallel(&later, |(name, u)| {
            let bytes = if is_network(u) {
                fetch_bytes(u)
            } else {
                // A file next to the page.
                let path = if u.starts_with('/') || u.starts_with("file:") { u.clone() } else { dir_of(&base) + u };
                read_file(&path).ok()
            };
            if let Some(b) = bytes {
                uison::image::put(name, b);
            }
            Some(())
        });
    };
    if SYNC.load(Ordering::Relaxed) {
        job();
    } else if let Some(h) = with(|b| b.handle.clone()) {
        std::thread::spawn(move || {
            job();
            // Rebuild the frame: image sizes are known now.
            h.post(|cx| cx.data.update("imagesLoaded", |n| n.as_f64().unwrap_or(0.0) + 1.0));
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn fetch_bytes(u: &str) -> Option<Vec<u8>> {
    net::get(u).ok().map(|r| r.body)
}

#[cfg(target_arch = "wasm32")]
fn fetch_bytes(_: &str) -> Option<Vec<u8>> {
    None
}

fn dir_of(path: &str) -> String {
    let p = path.strip_prefix("file://").unwrap_or(path);
    p[..p.rfind('/').map_or(0, |i| i + 1)].to_string()
}

/// `data:image/png;base64,…` → bytes.
fn data_url(rest: &str) -> Option<Vec<u8>> {
    let (meta, payload) = rest.split_once(',')?;
    if !meta.ends_with(";base64") {
        return Some(payload.as_bytes().to_vec());
    }
    let val = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => return None,
        } as u32)
    };
    let mut out = Vec::with_capacity(payload.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0);
    for c in payload.bytes().filter(|c| !c.is_ascii_whitespace() && *c != b'=') {
        acc = (acc << 6 | val(c)?) & 0xFFFF;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

/// Navigate to an address, recording it in history.
fn go(cx: &mut Cx, url: &str) {
    with(|b| {
        let keep = (b.pos + 1).min(b.history.len());
        b.history.truncate(keep);
        b.history.push(normalize(url));
        b.pos = b.history.len() - 1;
    });
    open(&mut cx.tree, cx.data, url);
}

pub fn app() -> Ui {
    app_at(HOME)
}

/// A browser opened at address `start`.
pub fn app_at(start: &str) -> Ui {
    let mut ui = Ui::new(include_str!("../app.json")).unwrap_or_else(|e| panic!("app.json: {e}"));
    let handle = ui.handle();
    B.with(|b| *b.borrow_mut() = Browser { handle: Some(handle), ..Browser::default() });
    let start = start.to_string();
    ui.on("start", move |cx| go(cx, &start));
    ui.on("go", |cx| {
        let url = cx.data.str("address").to_string();
        go(cx, &url);
    });
    ui.on("nav", |cx| {
        // key of a link run: `a<number>.<n>` (may have a path prefix).
        let key = cx.key.rsplit('/').next().unwrap_or("");
        let Some(i) = key.strip_prefix('a').and_then(|k| k.split('.').next()).and_then(|n| n.parse::<usize>().ok()) else { return };
        let Some((href, base)) = with(|b| b.links.get(i).cloned().map(|h| (h, b.url.clone()))) else { return };
        if href.starts_with('#') || href.starts_with("javascript:") {
            return;
        }
        if href.starts_with("mailto:") {
            cx.open_url(&href);
            return;
        }
        go(cx, &url::resolve(&base, &href));
    });
    ui.on("back", |cx| {
        let url = with(|b| {
            (b.pos > 0).then(|| {
                b.pos -= 1;
                b.history[b.pos].clone()
            })
        });
        if let Some(url) = url {
            open(&mut cx.tree, cx.data, &url);
        }
    });
    ui.on("forward", |cx| {
        let url = with(|b| {
            (b.pos + 1 < b.history.len()).then(|| {
                b.pos += 1;
                b.history[b.pos].clone()
            })
        });
        if let Some(url) = url {
            open(&mut cx.tree, cx.data, &url);
        }
    });
    ui.on("home", |cx| go(cx, HOME));
    ui.on("reload", |cx| {
        let url = with(|b| b.url.clone());
        open(&mut cx.tree, cx.data, &url);
    });
    ui.shortcut("cmd+l", "focusAddress");
    ui.shortcut("cmd+r", "reload");
    ui.on("focusAddress", |cx| cx.focus("address/1/0"));
    ui.run_action("start");
    ui
}

#[cfg(test)]
mod tests {
    #[test]
    fn data_urls() {
        assert_eq!(super::data_url("image/png;base64,aGVsbG8="), Some(b"hello".to_vec()));
        assert_eq!(super::data_url("text/plain,hi"), Some(b"hi".to_vec()));
    }
}
