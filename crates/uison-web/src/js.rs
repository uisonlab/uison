//! Imports from `uison.js` (next to Cargo.toml). Colors are RGBA in u32 (0xRRGGBBAA),
//! strings are a pointer and length in WASM memory (UTF-8).
//!
//! Names in WASM are `uison_js_<name>`. A Rust program (`wasm32-unknown-unknown`) gets them
//! from the `uison` module built by `uison.js`. A C/C++ program via Emscripten — from `env`:
//! the `EM_JS` strings (`__em_js__uison_js_<name>`) live here too, from them emcc writes JS functions
//! that call `uison.js` — all JS is inside `libuison.a`, no flags or file paths needed.

macro_rules! imports {
    ($($(#[$m:meta])* fn $name:ident($($a:ident: $t:ty),*) $(-> $r:ty)?;)*) => {
        #[cfg_attr(not(target_os = "emscripten"), link(wasm_import_module = "uison"))]
        #[cfg_attr(target_os = "emscripten", link(wasm_import_module = "env"))]
        unsafe extern "C" {
            $($(#[$m])* #[link_name = concat!("uison_js_", stringify!($name))]
            pub fn $name($($a: $t),*) $(-> $r)?;)*
        }

        /// `EM_JS` strings: "(arguments)<::>{body}", the body calls a function from `uison.js`.
        #[cfg(target_os = "emscripten")]
        pub mod em {
            $(#[allow(non_upper_case_globals)]
            #[unsafe(export_name = concat!("__em_js__uison_js_", stringify!($name)))]
            #[used]
            pub static $name: [u8; em_js!($name($($a),*)).len() + 1] = super::cstr(em_js!($name($($a),*)));)*
        }
    };
}

#[cfg(target_os = "emscripten")]
macro_rules! em_js {
    ($name:ident($($a:ident),*)) => {
        concat!("(", stringify!($($a),*), ")<::>{return uisonJs.", stringify!($name), "(", stringify!($($a),*), ")}")
    };
}

/// A NUL-terminated string — that's how emcc reads `EM_JS` strings from memory.
#[cfg(target_os = "emscripten")]
const fn cstr<const N: usize>(s: &str) -> [u8; N] {
    let b = s.as_bytes();
    let mut a = [0; N];
    let mut i = 0;
    while i < b.len() {
        a[i] = b[i];
        i += 1;
    }
    a
}

imports! {
    fn now() -> f64;
    fn log(ptr: *const u8, len: usize, level: u32);
    fn set_title(ptr: *const u8, len: usize);
    fn set_cursor(kind: u32);
    fn request_tick(ms: f64);
    fn request_frame();
    /// Alarm for `Handle::post`: call `uison_wake` in the browser's next task.
    fn wake();
    /// Modifiers of the last input event: bits shift 1, ctrl 2, alt 4, cmd 8.
    fn mods() -> u32;
    fn clipboard_write(ptr: *const u8, len: usize);
    /// Read the clipboard; the answer comes later via `uison_paste` (or the browser's `paste` event).
    fn clipboard_read();
    fn open_url(ptr: *const u8, len: usize) -> u32;
    /// Draw image `src`; fit: 0 cover, 1 contain, 2 fill. 0 — JS hasn't seen this
    /// `mem:` source yet: pass the bytes (`image_data`). Readiness — a `uison_image_loaded` call.
    fn image(ptr: *const u8, len: usize, x: f32, y: f32, w: f32, h: f32, radius: f32, fit: u32) -> u32;
    fn image_data(ptr: *const u8, len: usize, data: *const u8, data_len: usize);
    /// Size of a loaded image: out[0], out[1]; 0 — not loaded yet (a file — start loading).
    fn image_size(ptr: *const u8, len: usize, out: *mut f32) -> u32;

    fn clip_push(x: f32, y: f32, w: f32, h: f32);
    fn clip_pop();
    fn fill_rect(x: f32, y: f32, w: f32, h: f32, radius: f32, color: u32);
    fn stroke_rect(x: f32, y: f32, w: f32, h: f32, radius: f32, width: f32, color: u32);
    fn fill_circle(cx: f32, cy: f32, r: f32, color: u32);
    fn stroke_circle(cx: f32, cy: f32, r: f32, width: f32, color: u32);
    /// Polygon: `n` points (x, y) in a row at `ptr`; color 0 — no fill or stroke.
    fn poly(ptr: *const f32, n: usize, fill: u32, stroke: u32, width: f32);
    fn line(x1: f32, y1: f32, x2: f32, y2: f32, width: f32, color: u32, round: u32);
    /// `y` is the baseline.
    fn text(ptr: *const u8, len: usize, x: f32, y: f32, font: u32, color: u32);
    /// Register a CSS font, return its index.
    fn font(ptr: *const u8, len: usize) -> u32;
    /// out[0] — width, out[1] — ascent, out[2] — descent.
    fn measure(ptr: *const u8, len: usize, font: u32, out: *mut f32);
}

// Emscripten: all of `uison.js` is the body of `uison_js_boot` (called once from `run`).
// names — comma-separated export names, fns — their indices in the function table (function pointers).
#[cfg(target_os = "emscripten")]
#[link(wasm_import_module = "env")]
unsafe extern "C" {
    #[link_name = "uison_js_boot"]
    pub fn boot(names: *const u8, names_len: usize, fns: *const u32, n: usize);
}

// Before leaving main via a trap: the glue (callMain → handleException) takes it as an exit from main.
#[cfg(target_os = "emscripten")]
#[link(wasm_import_module = "env")]
unsafe extern "C" {
    #[link_name = "uison_js_park"]
    pub fn park();
}

#[cfg(target_os = "emscripten")]
const PARK: &str = "()<::>{if(typeof handleException!='undefined'){const prev=handleException;\
    handleException=(e)=>{handleException=prev;return e instanceof WebAssembly.RuntimeError?0:prev(e)}}}";

#[cfg(target_os = "emscripten")]
#[unsafe(export_name = "__em_js__uison_js_park")]
#[used]
pub static PARK_JS: [u8; PARK.len() + 1] = cstr(PARK);

#[cfg(target_os = "emscripten")]
const BOOT: &str = concat!("(names, names_len, fns, n)<::>{", include_str!(concat!(env!("OUT_DIR"), "/uison.min.js")), "}");

#[cfg(target_os = "emscripten")]
#[unsafe(export_name = "__em_js__uison_js_boot")]
#[used]
pub static BOOT_JS: [u8; BOOT.len() + 1] = cstr(BOOT);
