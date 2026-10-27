// Real mouse and keyboard for testing a uison window under Wayland (like windrive / drive.swift).
// The pointer is virtual (zwlr_virtual_pointer_v1: Sway and other wlroots compositors),
// the keyboard is wtype. The protocol is the same custom client as in uison-linux (wire.rs, sys.rs).
// Build: rustc -O --edition 2024 -o target/lindrive tools/lindrive.rs   (tools/linux.sh builds it itself)
//
// lindrive [layout W H] [origin X Y] [map file] commands...
//   layout — size of the output layout in points (1600 1000 by default), origin — top left
//   of the window content within it; command coordinates are points inside the window.
// Commands: move x y | click x y | rclick x y | down x y | up x y | drag x0 y0 x1 y1 steps
//           wheel x y clicks | tap id | hover id | type text | key name | ctrl char | wait ms
//           shot file.png (screenshot via grim) | pan x y dy ms (touchpad, with inertia)

#![allow(dead_code)]

macro_rules! log {
    ($($t:tt)*) => { eprintln!($($t)*) };
}

#[path = "../crates/uison-linux/src/sys.rs"]
mod sys;
#[path = "../crates/uison-linux/src/wire.rs"]
mod wire;

use std::collections::HashMap;
use std::process::Command;
use std::time::{Duration, Instant};

use wire::{Arg, Body, Conn};

const BTN_LEFT: u32 = 0x110;
const BTN_RIGHT: u32 = 0x111;

struct Drive {
    c: Conn,
    ptr: u32,
    layout: (f32, f32),
    origin: (f32, f32),
    map: HashMap<String, (f32, f32)>,
    t0: Instant,
}

impl Drive {
    fn time(&self) -> u32 {
        self.t0.elapsed().as_millis() as u32
    }

    fn send(&mut self) {
        self.c.flush().expect("flush");
    }

    fn move_to(&mut self, x: f32, y: f32) {
        let (ax, ay) = (self.origin.0 + x, self.origin.1 + y);
        let (w, h) = self.layout;
        let t = self.time();
        // motion_absolute: fraction of the layout size, in whole extent units.
        self.c.request(self.ptr, 1, &[Arg::Uint(t), Arg::Uint((ax * 100.0) as u32), Arg::Uint((ay * 100.0) as u32), Arg::Uint((w * 100.0) as u32), Arg::Uint((h * 100.0) as u32)]);
        self.c.request(self.ptr, 4, &[]); // frame
        self.send();
    }

    fn button(&mut self, b: u32, down: bool) {
        let t = self.time();
        self.c.request(self.ptr, 2, &[Arg::Uint(t), Arg::Uint(b), Arg::Uint(down as u32)]);
        self.c.request(self.ptr, 4, &[]);
        self.send();
    }

    fn wheel(&mut self, clicks: i32) {
        let t = self.time();
        self.c.request(self.ptr, 5, &[Arg::Uint(0)]); // axis_source(wheel)
        // axis_discrete: value (15 per click, as in libinput) and number of clicks; down is positive.
        self.c.request(self.ptr, 7, &[Arg::Uint(t), Arg::Uint(0), Arg::Fixed(15.0 * clicks as f64), Arg::Int(clicks)]);
        self.c.request(self.ptr, 4, &[]);
        self.send();
    }

    /// Two-finger touchpad scroll: `axis` offsets (source = finger) and `axis_stop`.
    fn pan(&mut self, dy: f32, ms: u64) {
        let n = (ms / 16).max(1);
        for _ in 0..n {
            let t = self.time();
            self.c.request(self.ptr, 5, &[Arg::Uint(1)]); // axis_source(finger)
            // Finger up moves the content up: the Wayland value is positive (down the axis).
            self.c.request(self.ptr, 3, &[Arg::Uint(t), Arg::Uint(0), Arg::Fixed((-dy / n as f32) as f64)]);
            self.c.request(self.ptr, 4, &[]);
            self.send();
            pause(16);
        }
        let t = self.time();
        self.c.request(self.ptr, 5, &[Arg::Uint(1)]);
        self.c.request(self.ptr, 6, &[Arg::Uint(t), Arg::Uint(0)]); // axis_stop
        self.c.request(self.ptr, 4, &[]);
        self.send();
    }

    fn center(&self, id: &str) -> (f32, f32) {
        *self.map.get(id).unwrap_or_else(|| panic!("no '{id}' in the map"))
    }
}

fn pause(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

/// Each wtype run is a new virtual keyboard; the compositor delivers its first key press
/// before switching the active keyboard (the app gets only the release),
/// so a harmless Shift goes first.
fn wtype(args: &[&str]) {
    let ok = Command::new("wtype").args(["-k", "Shift_L"]).args(args).status().is_ok_and(|s| s.success());
    if !ok {
        eprintln!("lindrive: wtype {args:?} failed");
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut c = Conn::connect().expect("Wayland");
    // Registry: wl_seat and the virtual pointer manager.
    let reg = c.new_id();
    c.request(1, 1, &[Arg::New(reg)]);
    let sync = c.new_id();
    c.request(1, 0, &[Arg::New(sync)]);
    c.flush().unwrap();
    let (mut seat, mut mgr) = (0, 0);
    'outer: loop {
        c.read_blocking().unwrap();
        while let Some(m) = c.next() {
            if m.obj == sync {
                break 'outer;
            }
            if m.obj == reg && m.op == 0 {
                let mut b = Body::new(&m.body);
                let (name, iface, ver) = (b.u32(), b.str().to_string(), b.u32());
                let mut bind = |v: u32| {
                    let id = c.new_id();
                    c.request(reg, 0, &[Arg::Uint(name), Arg::NewAny(&iface, v.min(ver), id)]);
                    id
                };
                match iface.as_str() {
                    "wl_seat" if seat == 0 => seat = bind(1),
                    "zwlr_virtual_pointer_manager_v1" => mgr = bind(1),
                    _ => {}
                }
            }
        }
    }
    if mgr == 0 {
        eprintln!("lindrive: compositor lacks zwlr_virtual_pointer_manager_v1 (needs Sway / wlroots)");
        std::process::exit(1);
    }
    let ptr = c.new_id();
    c.request(mgr, 0, &[Arg::Obj(seat), Arg::New(ptr)]);
    c.flush().unwrap();
    let mut d = Drive { c, ptr, layout: (1600.0, 1000.0), origin: (0.0, 0.0), map: HashMap::new(), t0: Instant::now() };

    let mut i = 0;
    let num = |i: usize| -> f32 { args.get(i).and_then(|s| s.parse().ok()).unwrap_or_else(|| panic!("expected a number: argument {i}")) };
    let s = |i: usize| -> &str { args.get(i).map(String::as_str).unwrap_or_else(|| panic!("expected a string: argument {i}")) };
    while i < args.len() {
        match args[i].as_str() {
            "layout" => {
                d.layout = (num(i + 1), num(i + 2));
                i += 3;
            }
            "origin" => {
                d.origin = (num(i + 1), num(i + 2));
                i += 3;
            }
            "map" => {
                // UISON_HITMAP: id x y w h (window points)
                let text = std::fs::read_to_string(s(i + 1)).expect("map");
                for line in text.lines() {
                    let f: Vec<&str> = line.split('\t').collect();
                    if f.len() == 5 {
                        let v: Vec<f32> = f[1..].iter().map(|x| x.parse().unwrap_or(0.0)).collect();
                        d.map.insert(f[0].to_string(), (v[0] + v[2] / 2.0, v[1] + v[3] / 2.0));
                    }
                }
                i += 2;
            }
            "move" | "hover" => {
                let (x, y) = if args[i] == "hover" { d.center(s(i + 1)) } else { (num(i + 1), num(i + 2)) };
                d.move_to(x, y);
                i += if args[i] == "hover" { 2 } else { 3 };
                pause(60);
            }
            "click" | "rclick" | "tap" => {
                let (x, y) = if args[i] == "tap" { d.center(s(i + 1)) } else { (num(i + 1), num(i + 2)) };
                let b = if args[i] == "rclick" { BTN_RIGHT } else { BTN_LEFT };
                d.move_to(x, y);
                pause(40);
                d.button(b, true);
                pause(60);
                d.button(b, false);
                i += if args[i] == "tap" { 2 } else { 3 };
                pause(120);
            }
            "down" | "up" => {
                d.move_to(num(i + 1), num(i + 2));
                d.button(BTN_LEFT, args[i] == "down");
                i += 3;
                pause(60);
            }
            "drag" => {
                let (x0, y0, x1, y1, n) = (num(i + 1), num(i + 2), num(i + 3), num(i + 4), num(i + 5).max(1.0) as i32);
                d.move_to(x0, y0);
                pause(40);
                d.button(BTN_LEFT, true);
                // Like a human: the window gets time to respond to the press (moving/resizing via the frame).
                pause(150);
                for k in 1..=n {
                    let t = k as f32 / n as f32;
                    d.move_to(x0 + (x1 - x0) * t, y0 + (y1 - y0) * t);
                    pause(16);
                }
                d.button(BTN_LEFT, false);
                i += 6;
                pause(100);
            }
            "wheel" => {
                d.move_to(num(i + 1), num(i + 2));
                pause(30);
                d.wheel(num(i + 3) as i32);
                i += 4;
                pause(150);
            }
            "pan" => {
                // pan x y dy ms — with the touchpad (dy < 0 — content up)
                d.move_to(num(i + 1), num(i + 2));
                pause(30);
                d.pan(num(i + 3), num(i + 4) as u64);
                i += 5;
                pause(100);
            }
            "type" => {
                wtype(&[s(i + 1)]);
                i += 2;
                pause(100);
            }
            "key" => {
                wtype(&["-k", s(i + 1)]);
                i += 2;
                pause(100);
            }
            "ctrl" => {
                wtype(&["-M", "ctrl", s(i + 1), "-m", "ctrl"]);
                i += 2;
                pause(100);
            }
            "shot" => {
                // Screenshot while the virtual mouse is alive (hover disappears when it goes).
                let ok = Command::new("grim").arg(s(i + 1)).status().is_ok_and(|st| st.success());
                if !ok {
                    eprintln!("lindrive: grim failed");
                }
                i += 2;
            }
            "wait" => {
                pause(num(i + 1) as u64);
                i += 2;
            }
            other => panic!("unknown command '{other}'"),
        }
    }
    d.c.request(d.ptr, 8, &[]); // destroy
    d.send();
}
