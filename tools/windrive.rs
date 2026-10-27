// Real mouse, keyboard and touch for debugging a uison window on Windows (like drive.swift).
// Build: rustc -O -o target/windrive.exe tools/windrive.rs   (tools/win.ps1 builds it itself)
//
// windrive <part of the window title> commands... — coordinates in points inside the window (without the frame).
// Commands: where | front | map file | tap id | hover id | move x y | click x y | rclick x y
//           down x y | up x y | dragto x0 y0 x1 y1 steps | scroll x y clicks | type text
//           key name (Enter Escape Tab Backspace Delete Left Right Up Down Home End PageUp PageDown Menu F10)
//           ctrl char | shift key | touch x y | swipe x y dy ms | touchid id | wait ms
//           resizecheck step steps (flicker while resizing by the frame)
// windrive dpi [percent [DISPLAYn]] — monitors; change the scale (of the primary one by default).
// dragwin dx dy steps — drag the window by its title bar (screen pixels) | wininfo — window position and scale
//
// By element ids (robust to layout changes):
//   $env:UISON_HITMAP="map.txt"; target\release\todo.exe
//   target\windrive.exe todo map map.txt tap draft/1/0 type "Bread" key Enter

use std::ffi::c_void;
use std::time::Duration;

type HWND = *mut c_void;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct POINT {
    x: i32,
    y: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct RECT {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct MouseInput {
    dx: i32,
    dy: i32,
    data: u32,
    flags: u32,
    time: u32,
    extra: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct KeybdInput {
    vk: u16,
    scan: u16,
    flags: u32,
    time: u32,
    extra: usize,
}

#[repr(C)]
union InputU {
    mi: MouseInput,
    ki: KeybdInput,
}

#[repr(C)]
struct Input {
    kind: u32,
    u: InputU,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct PointerInfo {
    pointer_type: u32,
    pointer_id: u32,
    frame_id: u32,
    pointer_flags: u32,
    source_device: *mut c_void,
    hwnd_target: HWND,
    pt_pixel: POINT,
    pt_himetric: POINT,
    pt_pixel_raw: POINT,
    pt_himetric_raw: POINT,
    time: u32,
    history_count: u32,
    input_data: i32,
    key_states: u32,
    performance_count: u64,
    button_change: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct PointerTouchInfo {
    info: PointerInfo,
    touch_flags: u32,
    touch_mask: u32,
    contact: RECT,
    contact_raw: RECT,
    orientation: u32,
    pressure: u32,
}

#[link(name = "shcore")]
unsafe extern "system" {
    fn GetDpiForMonitor(m: *mut c_void, kind: u32, x: *mut u32, y: *mut u32) -> i32;
}

#[link(name = "gdi32")]
unsafe extern "system" {
    fn CreateCompatibleDC(dc: *mut c_void) -> *mut c_void;
    fn CreateCompatibleBitmap(dc: *mut c_void, w: i32, h: i32) -> *mut c_void;
    fn SelectObject(dc: *mut c_void, o: *mut c_void) -> *mut c_void;
    fn BitBlt(dst: *mut c_void, x: i32, y: i32, w: i32, h: i32, src: *mut c_void, sx: i32, sy: i32, rop: u32) -> i32;
    fn GetPixel(dc: *mut c_void, x: i32, y: i32) -> u32;
    fn DeleteDC(dc: *mut c_void) -> i32;
    fn DeleteObject(o: *mut c_void) -> i32;
}

#[link(name = "user32")]
unsafe extern "system" {
    fn SetProcessDpiAwarenessContext(ctx: isize) -> i32;
    fn EnumWindows(f: unsafe extern "system" fn(HWND, isize) -> i32, l: isize) -> i32;
    fn GetClassNameW(h: HWND, buf: *mut u16, n: i32) -> i32;
    fn GetWindowTextW(h: HWND, buf: *mut u16, n: i32) -> i32;
    fn IsWindowVisible(h: HWND) -> i32;
    fn ClientToScreen(h: HWND, p: *mut POINT) -> i32;
    fn GetDpiForWindow(h: HWND) -> u32;
    fn GetSystemMetrics(i: i32) -> i32;
    fn SendInput(n: u32, inputs: *const Input, size: i32) -> u32;
    fn SetForegroundWindow(h: HWND) -> i32;
    fn GetForegroundWindow() -> HWND;
    fn MonitorFromPoint(p: POINT, flags: u32) -> *mut c_void;
    fn EnumDisplaySettingsW(name: *const u16, mode: u32, dm: *mut c_void) -> i32;
    fn GetDisplayConfigBufferSizes(flags: u32, paths: *mut u32, modes: *mut u32) -> i32;
    fn QueryDisplayConfig(flags: u32, np: *mut u32, paths: *mut u8, nm: *mut u32, modes: *mut u8, topo: *mut c_void) -> i32;
    fn DisplayConfigGetDeviceInfo(p: *mut c_void) -> i32;
    fn DisplayConfigSetDeviceInfo(p: *const c_void) -> i32;
    fn GetDC(h: HWND) -> *mut c_void;
    fn ReleaseDC(h: HWND, dc: *mut c_void) -> i32;
    fn GetClientRect(h: HWND, r: *mut RECT) -> i32;
    fn GetWindowRect(h: HWND, r: *mut RECT) -> i32;
    fn InitializeTouchInjection(max: u32, mode: u32) -> i32;
    fn InjectTouchInput(n: u32, info: *const PointerTouchInfo) -> i32;
}

const INPUT_MOUSE: u32 = 0;
const INPUT_KEYBOARD: u32 = 1;
const MOUSEEVENTF_MOVE: u32 = 0x0001;
const MOUSEEVENTF_LEFTDOWN: u32 = 0x0002;
const MOUSEEVENTF_LEFTUP: u32 = 0x0004;
const MOUSEEVENTF_RIGHTDOWN: u32 = 0x0008;
const MOUSEEVENTF_RIGHTUP: u32 = 0x0010;
const MOUSEEVENTF_WHEEL: u32 = 0x0800;
const MOUSEEVENTF_VIRTUALDESK: u32 = 0x4000;
const MOUSEEVENTF_ABSOLUTE: u32 = 0x8000;
const KEYEVENTF_KEYUP: u32 = 0x0002;
const KEYEVENTF_UNICODE: u32 = 0x0004;
const VK_CONTROL: u16 = 0x11;
const VK_SHIFT: u16 = 0x10;
const PT_TOUCH: u32 = 2;
const POINTER_FLAG_INRANGE: u32 = 0x2;
const POINTER_FLAG_INCONTACT: u32 = 0x4;
const POINTER_FLAG_DOWN: u32 = 0x10000;
const POINTER_FLAG_UPDATE: u32 = 0x20000;
const POINTER_FLAG_UP: u32 = 0x40000;

static mut FOUND: HWND = std::ptr::null_mut();
static mut NEEDLE: Vec<u16> = Vec::new();

unsafe extern "system" fn enum_cb(h: HWND, _: isize) -> i32 {
    unsafe {
        let mut buf = [0u16; 256];
        let n = GetClassNameW(h, buf.as_mut_ptr(), 256) as usize;
        if String::from_utf16_lossy(&buf[..n]) != "UisonWindow" || IsWindowVisible(h) == 0 {
            return 1;
        }
        let n = GetWindowTextW(h, buf.as_mut_ptr(), 256) as usize;
        let title = String::from_utf16_lossy(&buf[..n]).to_lowercase();
        let needle = String::from_utf16_lossy(&*(&raw const NEEDLE)).to_lowercase();
        if title.contains(&needle) {
            FOUND = h;
            return 0;
        }
        1
    }
}

fn sleep(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

struct Win {
    hwnd: HWND,
    origin: POINT,
    scale: f64,
}

impl Win {
    fn px(&self, x: f64, y: f64) -> (i32, i32) {
        (self.origin.x + (x * self.scale).round() as i32, self.origin.y + (y * self.scale).round() as i32)
    }
}

fn mouse(flags: u32, x: i32, y: i32, data: u32) {
    send_mouse(flags, x, y, data);
    sleep(30);
}

fn send_mouse(flags: u32, x: i32, y: i32, data: u32) {
    let (vx, vy) = unsafe { (GetSystemMetrics(76), GetSystemMetrics(77)) };
    let (vw, vh) = unsafe { (GetSystemMetrics(78), GetSystemMetrics(79)) };
    // Absolute coordinates are 0..65535 across the whole virtual screen.
    let nx = (((x - vx) as i64 * 65536 + vw as i64 / 2) / vw as i64) as i32;
    let ny = (((y - vy) as i64 * 65536 + vh as i64 / 2) / vh as i64) as i32;
    let mi = MouseInput { dx: nx, dy: ny, data, flags: flags | MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK, time: 0, extra: 0 };
    let input = Input { kind: INPUT_MOUSE, u: InputU { mi } };
    unsafe { SendInput(1, &input, size_of::<Input>() as i32) };
}

fn key(vk: u16, scan: u16, flags: u32) {
    let ki = KeybdInput { vk, scan, flags, time: 0, extra: 0 };
    let input = Input { kind: INPUT_KEYBOARD, u: InputU { ki } };
    unsafe { SendInput(1, &input, size_of::<Input>() as i32) };
}

fn press(vk: u16) {
    key(vk, 0, 0);
    key(vk, 0, KEYEVENTF_KEYUP);
}

fn vk_of(name: &str) -> u16 {
    match name {
        "Enter" => 0x0D,
        "Escape" | "Esc" => 0x1B,
        "Tab" => 0x09,
        "Backspace" => 0x08,
        "Delete" => 0x2E,
        "Left" => 0x25,
        "Up" => 0x26,
        "Right" => 0x27,
        "Down" => 0x28,
        "Home" => 0x24,
        "End" => 0x23,
        "PageUp" => 0x21,
        "PageDown" => 0x22,
        "Menu" => 0x5D,
        "F10" => 0x79,
        n if n.len() == 1 && n.as_bytes()[0].is_ascii_alphanumeric() => n.as_bytes()[0].to_ascii_uppercase() as u16,
        n => n.parse().unwrap_or_else(|_| {
            eprintln!("unknown key '{n}'");
            std::process::exit(2)
        }),
    }
}

fn touch(_hwnd: HWND, flags: u32, x: i32, y: i32) {
    let info = PointerInfo {
        pointer_type: PT_TOUCH,
        pointer_id: 0,
        frame_id: 0,
        pointer_flags: flags,
        source_device: std::ptr::null_mut(),
        // The system finds the window by the point itself; setting it here gives error 87.
        hwnd_target: std::ptr::null_mut(),
        pt_pixel: POINT { x, y },
        pt_himetric: POINT::default(),
        pt_pixel_raw: POINT::default(),
        pt_himetric_raw: POINT::default(),
        time: 0,
        history_count: 0,
        input_data: 0,
        key_states: 0,
        performance_count: 0,
        button_change: 0,
    };
    // Contact area 4×4 px, pressure and orientation like a real finger (TOUCH_MASK_* = 1 | 2 | 4).
    let contact = RECT { left: x - 2, top: y - 2, right: x + 2, bottom: y + 2 };
    let t = PointerTouchInfo { info, touch_flags: 0, touch_mask: 7, contact, contact_raw: RECT::default(), orientation: 90, pressure: 32000 };
    if unsafe { InjectTouchInput(1, &t) } == 0 {
        eprintln!("InjectTouchInput failed: {}", std::io::Error::last_os_error());
    }
}

/// Center of a hit area by id from the UISON_HITMAP map (re-read before every step).
fn area(map: &Option<String>, id: &str) -> (f64, f64) {
    let Some(path) = map else {
        eprintln!("no hit map: use 'map <file>' and run the app with UISON_HITMAP=<file>");
        std::process::exit(1)
    };
    let text = std::fs::read_to_string(path).unwrap_or_default();
    // The last one is topmost (popups are drawn later).
    for line in text.lines().rev() {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() == 5 && f[0] == id {
            let n: Vec<f64> = f[1..].iter().map(|v| v.parse().unwrap_or(0.0)).collect();
            return (n[0] + n[2] / 2.0, n[1] + n[3] / 2.0);
        }
    }
    eprintln!("no hit area '{id}'");
    std::process::exit(1)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: windrive <window title part> commands...");
        std::process::exit(2);
    }
    unsafe {
        SetProcessDpiAwarenessContext(-4); // PER_MONITOR_AWARE_V2: coordinates are physical pixels
    }
    // windrive dpi [percent [DISPLAYn]] — monitors and their scale (no window needed).
    if args[1] == "dpi" {
        dpi_command(&args);
        return;
    }
    unsafe {
        NEEDLE = args[1].encode_utf16().collect();
        EnumWindows(enum_cb, 0);
    }
    let hwnd = unsafe { FOUND };
    if hwnd.is_null() {
        eprintln!("window '{}' not found", args[1]);
        std::process::exit(1);
    }
    let mut origin = POINT::default();
    unsafe { ClientToScreen(hwnd, &mut origin) };
    let w = Win { hwnd, origin, scale: unsafe { GetDpiForWindow(hwnd) } as f64 / 96.0 };
    let mut touch_ready = false;
    let mut map: Option<String> = None;
    let mut i = 2;
    let num = |i: usize| -> f64 {
        args.get(i).and_then(|s| s.parse().ok()).unwrap_or_else(|| {
            eprintln!("expected a number at argument {i}");
            std::process::exit(2)
        })
    };
    while i < args.len() {
        let c = args[i].as_str();
        i += 1;
        match c {
            "where" => println!("client origin {} {} px, scale {}%", w.origin.x, w.origin.y, (w.scale * 100.0).round()),
            "front" => front(w.hwnd),
            "map" => {
                map = Some(args[i].clone());
                i += 1;
            }
            "tap" | "hover" | "touchid" => {
                let (x, y) = area(&map, &args[i]);
                i += 1;
                let (px, py) = w.px(x, y);
                if c == "touchid" {
                    touch_ready = touch_ready || unsafe { InitializeTouchInjection(1, 1) } != 0;
                    touch(w.hwnd, POINTER_FLAG_DOWN | POINTER_FLAG_INRANGE | POINTER_FLAG_INCONTACT, px, py);
                    sleep(60);
                    touch(w.hwnd, POINTER_FLAG_UP, px, py);
                    sleep(60);
                    continue;
                }
                mouse(0, px, py, 0);
                if c == "tap" {
                    mouse(MOUSEEVENTF_LEFTDOWN, px, py, 0);
                    mouse(MOUSEEVENTF_LEFTUP, px, py, 0);
                }
            }
            "move" | "click" | "rclick" | "down" | "up" => {
                let (px, py) = w.px(num(i), num(i + 1));
                i += 2;
                match c {
                    "move" => mouse(0, px, py, 0),
                    "down" => mouse(MOUSEEVENTF_LEFTDOWN, px, py, 0),
                    "up" => mouse(MOUSEEVENTF_LEFTUP, px, py, 0),
                    "click" => {
                        mouse(0, px, py, 0);
                        mouse(MOUSEEVENTF_LEFTDOWN, px, py, 0);
                        mouse(MOUSEEVENTF_LEFTUP, px, py, 0);
                    }
                    _ => {
                        mouse(0, px, py, 0);
                        mouse(MOUSEEVENTF_RIGHTDOWN, px, py, 0);
                        mouse(MOUSEEVENTF_RIGHTUP, px, py, 0);
                    }
                }
            }
            "dragto" => {
                let a = w.px(num(i), num(i + 1));
                let b = w.px(num(i + 2), num(i + 3));
                let n = num(i + 4).max(1.0) as i32;
                i += 5;
                mouse(0, a.0, a.1, 0);
                mouse(MOUSEEVENTF_LEFTDOWN, a.0, a.1, 0);
                for k in 1..=n {
                    let x = a.0 + (b.0 - a.0) * k / n;
                    let y = a.1 + (b.1 - a.1) * k / n;
                    mouse(0, x, y, 0);
                }
                mouse(MOUSEEVENTF_LEFTUP, b.0, b.1, 0);
            }
            "scroll" => {
                // scroll x y clicks: > 0 — wheel away from you (content down), fractional — touchpad.
                let (px, py) = w.px(num(i), num(i + 1));
                let clicks = num(i + 2);
                i += 3;
                mouse(0, px, py, 0);
                mouse(MOUSEEVENTF_WHEEL, px, py, (clicks * 120.0).round() as i32 as u32);
            }
            "type" => {
                front(w.hwnd);
                for u in args[i].encode_utf16() {
                    key(0, u, KEYEVENTF_UNICODE);
                    key(0, u, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP);
                    sleep(40);
                }
                i += 1;
            }
            "key" => {
                front(w.hwnd);
                press(vk_of(&args[i]));
                i += 1;
                sleep(30);
            }
            "ctrl" | "shift" => {
                front(w.hwnd);
                let m = if c == "ctrl" { VK_CONTROL } else { VK_SHIFT };
                key(m, 0, 0);
                press(vk_of(&args[i]));
                key(m, 0, KEYEVENTF_KEYUP);
                i += 1;
                sleep(30);
            }
            "touch" | "swipe" => {
                touch_ready = touch_ready || unsafe { InitializeTouchInjection(1, 1) } != 0;
                if !touch_ready {
                    eprintln!("InitializeTouchInjection failed: {}", std::io::Error::last_os_error());
                }
                let (x, y) = (num(i), num(i + 1));
                let (px, py) = w.px(x, y);
                i += 2;
                touch(w.hwnd, POINTER_FLAG_DOWN | POINTER_FLAG_INRANGE | POINTER_FLAG_INCONTACT, px, py);
                let mut end = (px, py);
                if c == "swipe" {
                    // swipe x y dy ms: the finger moves dy points over ms, in 16 ms frames.
                    let (dy, ms) = (num(i), num(i + 1));
                    i += 2;
                    let n = (ms / 16.0).max(1.0) as i32;
                    for k in 1..=n {
                        sleep(16);
                        end = w.px(x, y + dy * k as f64 / n as f64);
                        touch(w.hwnd, POINTER_FLAG_UPDATE | POINTER_FLAG_INRANGE | POINTER_FLAG_INCONTACT, end.0, end.1);
                    }
                } else {
                    sleep(60);
                }
                touch(w.hwnd, POINTER_FLAG_UP, end.0, end.1);
                sleep(60);
            }
            "dragwin" => {
                // Window title bar, as by hand: press, drag in per-frame steps, release.
                let (dx, dy, n) = (num(i) as i32, num(i + 1) as i32, num(i + 2).max(1.0) as i32);
                i += 3;
                let mut r = RECT::default();
                let mut origin = POINT::default();
                unsafe {
                    GetWindowRect(w.hwnd, &mut r);
                    ClientToScreen(w.hwnd, &mut origin);
                }
                let (x0, y0) = ((r.left + r.right) / 2, (r.top + origin.y) / 2);
                mouse(0, x0, y0, 0);
                mouse(MOUSEEVENTF_LEFTDOWN, x0, y0, 0);
                for k in 1..=n {
                    send_mouse(0, x0 + dx * k / n, y0 + dy * k / n, 0);
                    sleep(16);
                }
                sleep(100);
                mouse(MOUSEEVENTF_LEFTUP, x0 + dx, y0 + dy, 0);
                sleep(300);
            }
            "wininfo" => unsafe {
                let mut r = RECT::default();
                GetWindowRect(w.hwnd, &mut r);
                let mut c = RECT::default();
                GetClientRect(w.hwnd, &mut c);
                let dpi = GetDpiForWindow(w.hwnd);
                println!(
                    "window {},{} {}x{} px, client {}x{} px = {}x{} pt, scale {}%",
                    r.left, r.top, r.right - r.left, r.bottom - r.top, c.right, c.bottom,
                    c.right * 96 / dpi as i32, c.bottom * 96 / dpi as i32, dpi * 100 / 96
                );
            },
            "resizecheck" => {
                // resizecheck step_pt steps: drag the right border and immediately capture a column (wrong color / black)
                // at the new right edge of the client area — how many pixels differ from the color
                // that was there before starting (flicker, "black bars").
                let (step, n) = (num(i), num(i + 1) as i32);
                i += 2;
                resize_check(&w, step, n);
            }
            "wait" => {
                sleep(num(i) as u64);
                i += 1;
            }
            _ => {
                eprintln!("unknown command '{c}'");
                std::process::exit(2);
            }
        }
    }
}

/// Bring the window to the foreground, or keys go to another window. First
/// SetForegroundWindow (the system doesn't always allow it), then a click on the window title bar.
/// If that fails, exit with an error; no keys are sent.
fn front(hwnd: HWND) {
    unsafe {
        if GetForegroundWindow() == hwnd {
            return;
        }
        SetForegroundWindow(hwnd);
        sleep(100);
        if GetForegroundWindow() != hwnd {
            let mut r = RECT::default();
            GetWindowRect(hwnd, &mut r);
            let mut origin = POINT::default();
            ClientToScreen(hwnd, &mut origin);
            // Middle of the title bar: above the client area, away from the window buttons.
            let (x, y) = ((r.left + r.right) / 2, (r.top + origin.y) / 2);
            mouse(MOUSEEVENTF_LEFTDOWN, x, y, 0);
            mouse(MOUSEEVENTF_LEFTUP, x, y, 0);
            sleep(150);
        }
        if GetForegroundWindow() != hwnd {
            eprintln!("window is not in the foreground: keys not sent");
            std::process::exit(3);
        }
    }
}

/// A column of screen pixels (x, y..y+h) as 0xBBGGRR colors.
fn screen_column(x: i32, y: i32, h: i32) -> Vec<u32> {
    unsafe {
        let screen = GetDC(std::ptr::null_mut());
        let mem = CreateCompatibleDC(screen);
        let bmp = CreateCompatibleBitmap(screen, 1, h);
        let old = SelectObject(mem, bmp);
        BitBlt(mem, 0, 0, 1, h, screen, x, y, 0x00CC0020); // SRCCOPY
        let col = (0..h).map(|r| GetPixel(mem, 0, r)).collect();
        SelectObject(mem, old);
        DeleteObject(bmp);
        DeleteDC(mem);
        ReleaseDC(std::ptr::null_mut(), screen);
        col
    }
}

fn resize_check(w: &Win, step: f64, n: i32) {
    let mut rc = RECT::default();
    unsafe { GetClientRect(w.hwnd, &mut rc) };
    let (cw, ch) = (rc.right, rc.bottom);
    // Reference: a column 6 px from the right edge — background in the examples, also after resizing.
    let reference = screen_column(w.origin.x + cw - 6, w.origin.y, ch);
    let (mut x, y) = (w.origin.x + cw + 3, w.origin.y + ch / 2);
    mouse(0, x, y, 0);
    mouse(MOUSEEVENTF_LEFTDOWN, x, y, 0);
    let dx = (step * w.scale).round() as i32;
    let mut bad_total = 0;
    for k in 1..=n {
        x += dx;
        send_mouse(0, x, y, 0);
        sleep(2);
        let edge = x - 3 - 6;
        let mut line = format!("step {k}:");
        for delay in [0u64, 8, 16, 33] {
            if delay > 0 {
                sleep(delay);
            }
            let col = screen_column(edge, w.origin.y, ch);
            let bad = col.iter().zip(&reference).filter(|(a, b)| a != b).count();
            let black = col.iter().filter(|&&c| (c & 0xFF) < 16 && (c >> 8 & 0xFF) < 16 && (c >> 16 & 0xFF) < 16).count();
            bad_total += bad;
            line.push_str(&format!(" +{delay}ms {bad}/{black}"));
        }
        println!("{line}");
    }
    mouse(MOUSEEVENTF_LEFTUP, x, y, 0);
    println!("mismatched pixels total: {bad_total} (column {ch} px)");
}

/// Monitor: DisplayConfig source, desktop position, refresh rate, scale.
struct Display {
    name: String,
    luid: [u8; 8],
    id: u32,
    /// Recommended scale as an index into `STEPS`.
    recommended: i32,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    hz: u32,
    scale: u32,
}

const STEPS: [u32; 12] = [100, 125, 150, 175, 200, 225, 250, 300, 350, 400, 450, 500];

fn info_header(kind: i32, size: u32, luid: &[u8; 8], id: u32) -> Vec<u8> {
    let mut h = Vec::with_capacity(size as usize);
    h.extend_from_slice(&kind.to_le_bytes());
    h.extend_from_slice(&size.to_le_bytes());
    h.extend_from_slice(luid);
    h.extend_from_slice(&id.to_le_bytes());
    h.resize(size as usize, 0);
    h
}

/// Monitors as in Settings → Display. Scale via DisplayConfig{Get,Set}DeviceInfo
/// with types -3/-4 (undocumented): an index relative to the recommended one in the series
/// 100, 125, 150 … %; the "current" field comes back as garbage — take the current one from the monitor.
fn displays() -> Vec<Display> {
    let mut out = Vec::new();
    unsafe {
        let (mut np, mut nm) = (0u32, 0u32);
        GetDisplayConfigBufferSizes(2, &mut np, &mut nm); // QDC_ONLY_ACTIVE_PATHS
        let mut paths = vec![0u8; np as usize * 72];
        let mut modes = vec![0u8; nm as usize * 64];
        QueryDisplayConfig(2, &mut np, paths.as_mut_ptr(), &mut nm, modes.as_mut_ptr(), std::ptr::null_mut());
        for p in paths.chunks_exact(72).take(np as usize) {
            // DISPLAYCONFIG_PATH_SOURCE_INFO at the start of the path: adapter LUID, source id.
            let luid: [u8; 8] = p[0..8].try_into().unwrap();
            let id = u32::from_le_bytes(p[8..12].try_into().unwrap());
            // DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME: GDI name (\.\DISPLAY1).
            let mut src = info_header(1, 84, &luid, id);
            DisplayConfigGetDeviceInfo(src.as_mut_ptr().cast());
            let wname: Vec<u16> = src[20..84].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&c| c != 0).collect();
            let name = String::from_utf16_lossy(&wname);
            let mut get = info_header(-3, 32, &luid, id);
            DisplayConfigGetDeviceInfo(get.as_mut_ptr().cast());
            let recommended = -i32::from_le_bytes(get[20..24].try_into().unwrap());
            // DEVMODEW: position (dmPosition), size, refresh rate.
            let mut dm = vec![0u8; 220];
            dm[68..70].copy_from_slice(&220u16.to_le_bytes());
            let mut wn = wname.clone();
            wn.push(0);
            EnumDisplaySettingsW(wn.as_ptr(), -1i32 as u32, dm.as_mut_ptr().cast());
            let rd = |o: usize| u32::from_le_bytes(dm[o..o + 4].try_into().unwrap());
            let (x, y) = (rd(76) as i32, rd(80) as i32);
            let (mut dx, mut dy) = (96u32, 96u32);
            GetDpiForMonitor(MonitorFromPoint(POINT { x: x + 1, y: y + 1 }, 2), 0, &mut dx, &mut dy);
            out.push(Display { name, luid, id, recommended, x, y, w: rd(172), h: rd(176), hz: rd(184), scale: dx * 100 / 96 });
        }
    }
    out
}

/// `windrive dpi` — list monitors; `windrive dpi 150 [DISPLAY1]` — set a monitor's scale
/// (the primary one by default).
fn dpi_command(args: &[String]) {
    let list = displays();
    if let Some(p) = args.get(2) {
        let p: u32 = p.parse().unwrap_or(100);
        let Some(idx) = STEPS.iter().position(|&s| s == p) else {
            eprintln!("scale must be one of {STEPS:?}");
            std::process::exit(2)
        };
        let d = match args.get(3) {
            Some(n) => list.iter().find(|d| d.name.ends_with(n.as_str())),
            None => list.iter().find(|d| d.x == 0 && d.y == 0),
        };
        let Some(d) = d else {
            eprintln!("no such display");
            std::process::exit(2)
        };
        let mut s = info_header(-4, 24, &d.luid, d.id);
        s[20..24].copy_from_slice(&(idx as i32 - d.recommended).to_le_bytes());
        let r = unsafe { DisplayConfigSetDeviceInfo(s.as_ptr().cast()) };
        if r != 0 {
            eprintln!("DisplayConfigSetDeviceInfo failed: {r}");
        }
        sleep(1500);
    }
    for d in displays() {
        let primary = if d.x == 0 && d.y == 0 { ", primary" } else { "" };
        let rec = STEPS[d.recommended.clamp(0, 11) as usize];
        println!("{} {}x{} at {},{} {} Hz, scale {}% (recommended {rec}%){primary}", d.name, d.w, d.h, d.x, d.y, d.hz, d.scale);
    }
}
