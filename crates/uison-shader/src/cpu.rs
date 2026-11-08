//! The program run on the CPU: every backend that can draw a bitmap can show a `shader` item,
//! and the result is the reference the GPU back ends are compared with.
//!
//! The typed tree is compiled once into a flat list of instructions over registers; each
//! register holds a value for a run of `LANES` pixels (up to four components, `LANES` floats
//! each). An instruction is one tight loop over arrays the compiler vectorizes, so the cost of
//! interpreting is paid per run, not per pixel, and nothing is allocated while painting.
//! Large areas are split across threads. The edge and rounded-corner coverage is the same as
//! in the SPIR-V shader (`spirv.rs`).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::lang::{Cmp, Error, Expr, Func, Input, Kind, Op, Program, parse};

/// Pixels per run.
const LANES: usize = 64;
/// Floats per register: four components.
const REG: usize = 4 * LANES;
/// Pixels per thread at least: below this, starting a thread costs more than it saves.
const PER_THREAD: usize = 4 * 1024;

/// Premultiplied RGBA pixels of the area `x..x + w`, `y..y + h` (device pixels), row by row.
pub struct Image {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

impl Image {
    /// Swap red and blue: premultiplied BGRA, as Direct2D, Cairo and Windows bitmaps want it.
    pub fn into_bgra(mut self) -> Image {
        for px in self.rgba.chunks_exact_mut(4) {
            px.swap(0, 2);
        }
        self
    }

    /// Straight (not premultiplied) RGBA, as the browser's `ImageData` wants it.
    pub fn into_straight(mut self) -> Image {
        for px in self.rgba.chunks_exact_mut(4) {
            let a = px[3] as u32;
            if a != 0 && a != 255 {
                for c in &mut px[..3] {
                    *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
                }
            }
        }
        self
    }
}

/// Where and when to paint.
#[derive(Clone, Copy, Debug)]
pub struct Target {
    /// The item's box in device pixels: x0, y0, x1, y1.
    pub rect: [f32; 4],
    /// Device pixels per point (`pos` and `size` are in points).
    pub scale: f32,
    pub time: f32,
    /// Corner radius in device pixels.
    pub radius: f32,
    /// Only pixels inside this area (device pixels, x0, y0, x1, y1) are computed: the visible or damaged part.
    pub clip: [i32; 4],
}

/// Registers of the inputs; the rest are allocated by the compiler.
const UV: usize = 0;
const POS: usize = 1;
const SIZE: usize = 2;
const TIME: usize = 3;
const FIRST_FREE: usize = 4;

/// One-argument operations, component by component.
#[derive(Clone, Copy)]
enum U1 {
    Neg,
    Not,
    F(Func),
}

/// Two-argument operations, component by component (comparisons and logic give 0 or 1).
#[derive(Clone, Copy)]
enum U2 {
    Op(Op),
    Cmp(Cmp),
    And,
    Or,
    F(Func),
}

enum Ins {
    /// `d = op(a)` over `n` components.
    Map(U1, usize, usize, usize),
    Zip(U2, usize, usize, usize, usize),
    /// clamp, mix, smoothstep.
    Zip3(Func, usize, usize, usize, usize, usize),
    /// `d.k = a.0` for `k < n`.
    Splat(usize, usize, usize),
    /// `d.dk = a.ak`.
    Copy(usize, usize, usize, usize),
    /// `d = c ? a : b`, `n` components.
    Select(usize, usize, usize, usize, usize),
    /// Functions of whole vectors: `d = f(a, b)` with `n`-component arguments.
    Vector(Func, usize, usize, usize, usize),
}

/// A program ready to run: instructions, constants, where the color ends up.
pub struct Shader {
    code: Vec<Ins>,
    /// Registers filled once per thread: constants (register, components).
    consts: Vec<(usize, Vec<f32>)>,
    regs: usize,
    result: usize,
    /// Components of the result: 1 (gray), 3 (rgb) or 4 (rgba).
    n: usize,
}

impl Shader {
    pub fn new(p: &Program) -> Shader {
        let mut c = Compiler { code: Vec::new(), consts: Vec::new(), regs: FIRST_FREE, lets: Vec::new() };
        for e in &p.lets {
            let r = c.expr(e);
            c.lets.push(r);
        }
        let result = c.expr(&p.result);
        Shader { code: c.code, consts: c.consts, regs: c.regs, result, n: p.result.ty.n() as usize }
    }
}

struct Compiler {
    code: Vec<Ins>,
    consts: Vec<(usize, Vec<f32>)>,
    regs: usize,
    lets: Vec<usize>,
}

impl Compiler {
    fn reg(&mut self) -> usize {
        self.regs += 1;
        self.regs - 1
    }

    fn constant(&mut self, v: Vec<f32>) -> usize {
        let r = self.reg();
        self.consts.push((r, v));
        r
    }

    /// The register holding the value of `e`.
    fn expr(&mut self, e: &Expr) -> usize {
        let n = e.ty.n().max(1) as usize;
        match &e.kind {
            Kind::Num(x) => self.constant(vec![*x]),
            Kind::Const(v) => self.constant(v.clone()),
            Kind::Input(Input::Uv) => UV,
            Kind::Input(Input::Pos) => POS,
            Kind::Input(Input::Size) => SIZE,
            Kind::Input(Input::Time) => TIME,
            Kind::Let(i) => self.lets[*i],
            Kind::Neg(a) => self.map(U1::Neg, a, n),
            Kind::Not(a) => self.map(U1::Not, a, n),
            Kind::Splat(a) => {
                let a = self.expr(a);
                let d = self.reg();
                self.code.push(Ins::Splat(d, a, n));
                d
            }
            Kind::Arith(op, a, b) => self.zip(U2::Op(*op), a, b, n),
            Kind::Compare(c, a, b) => self.zip(U2::Cmp(*c), a, b, 1),
            Kind::And(a, b) => self.zip(U2::And, a, b, 1),
            Kind::Or(a, b) => self.zip(U2::Or, a, b, 1),
            Kind::Select(c, a, b) => {
                let (c, a, b) = (self.expr(c), self.expr(a), self.expr(b));
                let d = self.reg();
                self.code.push(Ins::Select(d, c, a, b, n));
                d
            }
            Kind::Component(a, k) => {
                let a = self.expr(a);
                let d = self.reg();
                self.code.push(Ins::Copy(d, 0, a, *k as usize));
                d
            }
            Kind::Swizzle(a, idx) => {
                let a = self.expr(a);
                let d = self.reg();
                for (dk, &ak) in idx.iter().enumerate() {
                    self.code.push(Ins::Copy(d, dk, a, ak as usize));
                }
                d
            }
            Kind::Construct(parts) => {
                let d = self.reg();
                let mut dk = 0;
                for p in parts {
                    let a = self.expr(p);
                    for ak in 0..p.ty.n() as usize {
                        self.code.push(Ins::Copy(d, dk, a, ak));
                        dk += 1;
                    }
                }
                d
            }
            Kind::Call(f, args) => {
                let a: Vec<usize> = args.iter().map(|x| self.expr(x)).collect();
                let an = args[0].ty.n() as usize;
                let d = self.reg();
                self.code.push(match f {
                    Func::Atan2 | Func::Pow | Func::Mod | Func::Min | Func::Max | Func::Step => Ins::Zip(U2::F(*f), d, a[0], a[1], n),
                    Func::Clamp | Func::Mix | Func::SmoothStep => Ins::Zip3(*f, d, a[0], a[1], a[2], n),
                    Func::Length | Func::Normalize => Ins::Vector(*f, d, a[0], a[0], an),
                    Func::Distance | Func::Dot | Func::Cross | Func::Reflect => Ins::Vector(*f, d, a[0], a[1], an),
                    _ => Ins::Map(U1::F(*f), d, a[0], n),
                });
                d
            }
        }
    }

    fn map(&mut self, op: U1, a: &Expr, n: usize) -> usize {
        let a = self.expr(a);
        let d = self.reg();
        self.code.push(Ins::Map(op, d, a, n));
        d
    }

    fn zip(&mut self, op: U2, a: &Expr, b: &Expr, n: usize) -> usize {
        let (a, b) = (self.expr(a), self.expr(b));
        let d = self.reg();
        self.code.push(Ins::Zip(op, d, a, b, n));
        d
    }
}

/// Source → compiled program or its mistake.
type Programs = HashMap<Box<str>, Result<Rc<Shader>, Error>>;

thread_local! {
    /// Compiled programs by source, two generations: a frame repeats the same few programs.
    static PROGRAMS: RefCell<(Programs, Programs)> =
        RefCell::new(Default::default());
}

const GENERATION: usize = 64;

/// The compiled program for `src`, from a cache. `Err(Some(e))` the first time a program fails,
/// `Err(None)` afterwards: a backend reports a mistake once, not every frame.
pub fn program(src: &str) -> Result<Rc<Shader>, Option<Error>> {
    PROGRAMS.with_borrow_mut(|(cur, prev)| {
        if let Some(p) = cur.get(src) {
            return p.clone().map_err(|_| None);
        }
        let (p, first) = match prev.remove(src) {
            Some(p) => (p, false),
            None => (parse(src).map(|p| Rc::new(Shader::new(&p))), true),
        };
        if cur.len() >= GENERATION {
            *prev = std::mem::take(cur);
        }
        cur.insert(src.into(), p.clone());
        p.map_err(|e| first.then_some(e))
    })
}

/// Paint the program into the item's pixels (inside `t.clip`). `None` — nothing to paint.
pub fn render(s: &Shader, t: &Target) -> Option<Image> {
    render_on(s, t, std::thread::available_parallelism().map_or(1, |n| n.get()))
}

/// `render` on at most `cores` threads.
fn render_on(s: &Shader, t: &Target, cores: usize) -> Option<Image> {
    let [x0, y0, x1, y1] = t.rect;
    let ix0 = (x0.floor() as i32).max(t.clip[0]);
    let iy0 = (y0.floor() as i32).max(t.clip[1]);
    let ix1 = (x1.ceil() as i32).min(t.clip[2]);
    let iy1 = (y1.ceil() as i32).min(t.clip[3]);
    if ix1 <= ix0 || iy1 <= iy0 || x1 <= x0 || y1 <= y0 {
        return None;
    }
    let (w, h) = ((ix1 - ix0) as u32, (iy1 - iy0) as u32);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    let row_bytes = (w * 4) as usize;
    let rows = |first: i32, out: &mut [u8]| {
        let mut run = Run::new(s, t);
        for (k, line) in out.chunks_exact_mut(row_bytes).enumerate() {
            run.row(ix0, iy0 + first + k as i32, line);
        }
    };
    let parts = cores.min(crate::pool::threads()).min(h as usize).min((w * h) as usize / PER_THREAD).max(1);
    if parts > 1 {
        // Small bands taken from a counter: fast cores take more of them than slow ones
        // (performance and efficiency cores, a busy core), so no thread waits for another.
        let band = (PER_THREAD / w as usize).clamp(1, 16);
        let bands = (h as usize).div_ceil(band);
        let next = std::sync::atomic::AtomicUsize::new(0);
        let out = Rows(rgba.as_mut_ptr());
        crate::pool::run(parts, &|_| {
            let mut run = Run::new(s, t);
            loop {
                let b = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if b >= bands {
                    return;
                }
                let (first, last) = (b * band, ((b + 1) * band).min(h as usize));
                // SAFETY: every band is taken once, so the bands write disjoint rows of `rgba`, which outlives `run`.
                let part = unsafe { std::slice::from_raw_parts_mut(out.at(first * row_bytes), (last - first) * row_bytes) };
                for (k, line) in part.chunks_exact_mut(row_bytes).enumerate() {
                    run.row(ix0, iy0 + (first + k) as i32, line);
                }
            }
        });
    } else {
        rows(0, &mut rgba);
    }
    Some(Image { x: ix0, y: iy0, w, h, rgba })
}

/// The output buffer shared by the parts; each writes its own rows.
struct Rows(*mut u8);

// SAFETY: the parts write disjoint ranges (see `render_on`).
unsafe impl Sync for Rows {}

impl Rows {
    fn at(&self, offset: usize) -> *mut u8 {
        // SAFETY: `offset` is inside the buffer (a row start).
        unsafe { self.0.add(offset) }
    }
}

/// Parse and compile (with the cache), then paint. A mistake gives `Err(Some(e))` once, then `Err(None)`.
pub fn paint(src: &str, t: &Target) -> Result<Option<Image>, Option<Error>> {
    let s = program(src)?;
    Ok(render(&s, t))
}

/// One thread's registers.
struct Run<'a> {
    s: &'a Shader,
    t: &'a Target,
    r: Vec<f32>,
}

impl<'a> Run<'a> {
    fn new(s: &'a Shader, t: &'a Target) -> Run<'a> {
        let mut r = vec![0.0; s.regs * REG];
        let [x0, y0, x1, y1] = t.rect;
        r[SIZE * REG..SIZE * REG + LANES].fill((x1 - x0) / t.scale);
        r[SIZE * REG + LANES..SIZE * REG + 2 * LANES].fill((y1 - y0) / t.scale);
        r[TIME * REG..TIME * REG + LANES].fill(t.time);
        for (reg, v) in &s.consts {
            for (k, &x) in v.iter().enumerate() {
                r[reg * REG + k * LANES..][..LANES].fill(x);
            }
        }
        Run { s, t, r }
    }

    /// Pixels `x..x + line.len() / 4` of row `y`.
    fn row(&mut self, x: i32, y: i32, line: &mut [u8]) {
        let [x0, y0, x1, y1] = self.t.rect;
        let (bw, bh, sc) = (x1 - x0, y1 - y0, self.t.scale);
        let cy = y as f32 + 0.5;
        for (chunk, out) in line.chunks_mut(LANES * 4).enumerate() {
            let first = x + (chunk * LANES) as i32;
            // Pixel centers; lanes past the row's end are computed and dropped.
            let r = &mut self.r;
            for i in 0..LANES {
                let cx = first as f32 + i as f32 + 0.5;
                r[UV * REG + i] = (cx - x0) / bw;
                r[UV * REG + LANES + i] = (cy - y0) / bh;
                r[POS * REG + i] = (cx - x0) / sc;
                r[POS * REG + LANES + i] = (cy - y0) / sc;
            }
            for ins in &self.s.code {
                exec(&mut self.r, ins);
            }
            let c = &self.r[self.s.result * REG..][..REG];
            for (i, px) in out.chunks_exact_mut(4).enumerate() {
                let (r, g, b, a) = match self.s.n {
                    1 => (c[i], c[i], c[i], 1.0),
                    3 => (c[i], c[LANES + i], c[2 * LANES + i], 1.0),
                    _ => (c[i], c[LANES + i], c[2 * LANES + i], c[3 * LANES + i]),
                };
                let a = clamp01(a) * coverage(self.t, first as f32 + i as f32 + 0.5, cy);
                let u8 = |v: f32| (v * 255.0 + 0.5) as u8;
                px.copy_from_slice(&[u8(clamp01(r) * a), u8(clamp01(g) * a), u8(clamp01(b) * a), u8(a)]);
            }
        }
    }
}

/// How much of the pixel centered at (`x`, `y`) the box covers: the same as `box()` in ui.frag.
fn coverage(t: &Target, x: f32, y: f32) -> f32 {
    let [x0, y0, x1, y1] = t.rect;
    let (hx, hy) = ((x1 - x0) * 0.5, (y1 - y0) * 0.5);
    let rad = t.radius.min(hx.min(hy));
    if rad <= 0.0 {
        let span = |p: f32, lo: f32, hi: f32| clamp01((p + 0.5).min(hi) - (p - 0.5).max(lo));
        return span(x, x0, x1) * span(y, y0, y1);
    }
    let qx = (x - (x0 + x1) * 0.5).abs() - hx + rad;
    let qy = (y - (y0 + y1) * 0.5).abs() - hy + rad;
    let outside = if qx > 0.0 && qy > 0.0 { (qx * qx + qy * qy).sqrt() } else { qx.max(qy).max(0.0) };
    let d = outside + qx.max(qy).min(0.0) - rad;
    clamp01(0.5 - d)
}

fn clamp01(x: f32) -> f32 {
    // NaN → 0, like a GPU writing it to an 8-bit target.
    if x >= 0.0 { x.min(1.0) } else { 0.0 }
}

/// Register `d` to write and registers `a`, `b`, `c` to read. `d` is always a register of its own
/// (the compiler gives every instruction a new one), so it never overlaps the others.
fn regs(r: &mut [f32], d: usize, a: usize, b: usize, c: usize) -> (&mut [f32], &[f32], &[f32], &[f32]) {
    assert!(d != a && d != b && d != c && (d + 1) * REG <= r.len() && [a, b, c].iter().all(|&x| (x + 1) * REG <= r.len()));
    let p = r.as_mut_ptr();
    // SAFETY: the four ranges are inside `r` and `d`'s does not overlap the others (checked above);
    // the read-only ones may overlap each other.
    unsafe {
        (
            std::slice::from_raw_parts_mut(p.add(d * REG), REG),
            std::slice::from_raw_parts(p.add(a * REG), REG),
            std::slice::from_raw_parts(p.add(b * REG), REG),
            std::slice::from_raw_parts(p.add(c * REG), REG),
        )
    }
}

/// `d[i] = expr` for `i < len`, the match outside the loop so that each loop is vectorized.
/// The slices are cut to `len` first: the compiler then sees every index is in bounds.
macro_rules! each {
    ($len:expr, $d:ident $(, $s:ident)*; |$i:ident| $e:expr) => {{
        let len = $len;
        let $d = &mut $d[..len];
        $(let $s = &$s[..len];)*
        for $i in 0..len {
            $d[$i] = $e;
        }
    }};
}

fn exec(r: &mut [f32], ins: &Ins) {
    match *ins {
        Ins::Map(op, d, a, n) => {
            let (d, a, _, _) = regs(r, d, a, a, a);
            let len = n * LANES;
            match op {
                U1::Neg => each!(len, d, a; |i| -a[i]),
                U1::Not => each!(len, d, a; |i| 1.0 - a[i]),
                U1::F(f) => {
                    let f: fn(f32) -> f32 = match f {
                        Func::Abs => f32::abs,
                        Func::Sign => |v| if v > 0.0 { 1.0 } else if v < 0.0 { -1.0 } else { 0.0 },
                        Func::Floor => f32::floor,
                        Func::Ceil => f32::ceil,
                        Func::Fract => |v| v - v.floor(),
                        Func::Round => f32::round,
                        Func::Trunc => f32::trunc,
                        Func::Sin => f32::sin,
                        Func::Cos => f32::cos,
                        Func::Tan => f32::tan,
                        Func::Asin => f32::asin,
                        Func::Acos => f32::acos,
                        Func::Atan => f32::atan,
                        Func::Sinh => f32::sinh,
                        Func::Cosh => f32::cosh,
                        Func::Tanh => f32::tanh,
                        Func::Exp => f32::exp,
                        Func::Log => f32::ln,
                        Func::Exp2 => f32::exp2,
                        Func::Log2 => f32::log2,
                        Func::Sqrt => f32::sqrt,
                        Func::InverseSqrt => |v| 1.0 / v.sqrt(),
                        Func::Radians => f32::to_radians,
                        Func::Degrees => f32::to_degrees,
                        _ => unreachable!("not a one-argument function"),
                    };
                    each!(len, d, a; |i| f(a[i]));
                }
            }
        }
        Ins::Zip(op, d, a, b, n) => {
            let (d, a, b, _) = regs(r, d, a, b, b);
            let len = n * LANES;
            let bit = |c: bool| c as u8 as f32;
            match op {
                U2::Op(Op::Add) => each!(len, d, a, b; |i| a[i] + b[i]),
                U2::Op(Op::Sub) => each!(len, d, a, b; |i| a[i] - b[i]),
                U2::Op(Op::Mul) => each!(len, d, a, b; |i| a[i] * b[i]),
                U2::Op(Op::Div) => each!(len, d, a, b; |i| a[i] / b[i]),
                U2::Cmp(Cmp::Lt) => each!(len, d, a, b; |i| bit(a[i] < b[i])),
                U2::Cmp(Cmp::Le) => each!(len, d, a, b; |i| bit(a[i] <= b[i])),
                U2::Cmp(Cmp::Gt) => each!(len, d, a, b; |i| bit(a[i] > b[i])),
                U2::Cmp(Cmp::Ge) => each!(len, d, a, b; |i| bit(a[i] >= b[i])),
                U2::Cmp(Cmp::Eq) => each!(len, d, a, b; |i| bit(a[i] == b[i])),
                // Ordered, as OpFOrdNotEqual: false when either is NaN (`!=` would be true), as on the GPU.
                #[allow(clippy::double_comparisons)]
                U2::Cmp(Cmp::Ne) => each!(len, d, a, b; |i| bit(a[i] < b[i] || a[i] > b[i])),
                U2::And => each!(len, d, a, b; |i| a[i] * b[i]),
                U2::Or => each!(len, d, a, b; |i| a[i].max(b[i])),
                U2::F(Func::Min) => each!(len, d, a, b; |i| a[i].min(b[i])),
                U2::F(Func::Max) => each!(len, d, a, b; |i| a[i].max(b[i])),
                U2::F(Func::Pow) => each!(len, d, a, b; |i| a[i].powf(b[i])),
                // GLSL's mod follows the divisor's sign.
                U2::F(Func::Mod) => each!(len, d, a, b; |i| a[i] - b[i] * (a[i] / b[i]).floor()),
                U2::F(Func::Atan2) => each!(len, d, a, b; |i| a[i].atan2(b[i])),
                U2::F(Func::Step) => each!(len, d, a, b; |i| if b[i] < a[i] { 0.0 } else { 1.0 }),
                U2::F(_) => unreachable!("not a two-argument function"),
            }
        }
        Ins::Zip3(f, d, a, b, c, n) => {
            let (d, a, b, c) = regs(r, d, a, b, c);
            let len = n * LANES;
            match f {
                Func::Clamp => each!(len, d, a, b, c; |i| a[i].max(b[i]).min(c[i])),
                Func::Mix => each!(len, d, a, b, c; |i| a[i] * (1.0 - c[i]) + b[i] * c[i]),
                Func::SmoothStep => each!(len, d, a, b, c; |i| {
                    let t = ((c[i] - a[i]) / (b[i] - a[i])).clamp(0.0, 1.0);
                    t * t * (3.0 - 2.0 * t)
                }),
                _ => unreachable!("not a three-argument function"),
            }
        }
        Ins::Splat(d, a, n) => {
            let (d, a, _, _) = regs(r, d, a, a, a);
            for k in 0..n {
                d[k * LANES..(k + 1) * LANES].copy_from_slice(&a[..LANES]);
            }
        }
        Ins::Copy(d, dk, a, ak) => {
            let (d, a, _, _) = regs(r, d, a, a, a);
            d[dk * LANES..(dk + 1) * LANES].copy_from_slice(&a[ak * LANES..(ak + 1) * LANES]);
        }
        Ins::Select(d, c, a, b, n) => {
            let (d, a, b, c) = regs(r, d, a, b, c);
            for k in 0..n {
                for i in 0..LANES {
                    d[k * LANES + i] = if c[i] != 0.0 { a[k * LANES + i] } else { b[k * LANES + i] };
                }
            }
        }
        Ins::Vector(f, d, a, b, n) => {
            let (d, a, b, _) = regs(r, d, a, b, b);
            let dot = |i: usize, x: &[f32], y: &[f32]| (0..n).map(|k| x[k * LANES + i] * y[k * LANES + i]).sum::<f32>();
            match f {
                Func::Dot => each!(LANES, d; |i| dot(i, a, b)),
                Func::Length => each!(LANES, d; |i| dot(i, a, a).sqrt()),
                Func::Distance => each!(LANES, d; |i| (0..n).map(|k| (a[k * LANES + i] - b[k * LANES + i]).powi(2)).sum::<f32>().sqrt()),
                Func::Normalize => {
                    for i in 0..LANES {
                        let len = dot(i, a, a).sqrt();
                        for k in 0..n {
                            d[k * LANES + i] = a[k * LANES + i] / len;
                        }
                    }
                }
                // I - 2 * dot(N, I) * N
                Func::Reflect => {
                    for i in 0..LANES {
                        let dn = dot(i, b, a);
                        for k in 0..n {
                            d[k * LANES + i] = a[k * LANES + i] - 2.0 * dn * b[k * LANES + i];
                        }
                    }
                }
                Func::Cross => {
                    for i in 0..LANES {
                        let (p0, p1, p2) = (a[i], a[LANES + i], a[2 * LANES + i]);
                        let (q0, q1, q2) = (b[i], b[LANES + i], b[2 * LANES + i]);
                        d[i] = p1 * q2 - p2 * q1;
                        d[LANES + i] = p2 * q0 - p0 * q2;
                        d[2 * LANES + i] = p0 * q1 - p1 * q0;
                    }
                }
                _ => unreachable!("not a vector function"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(w: f32, h: f32) -> Target {
        Target { rect: [0.0, 0.0, w, h], scale: 1.0, time: 0.0, radius: 0.0, clip: [i32::MIN, i32::MIN, i32::MAX, i32::MAX] }
    }

    fn px(img: &Image, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * img.w + x) * 4) as usize;
        img.rgba[i..i + 4].try_into().unwrap()
    }

    #[test]
    fn gradient_and_inputs() {
        let img = paint("mix(#000, #fff, uv.x)", &target(4.0, 1.0)).unwrap().unwrap();
        assert_eq!((img.x, img.y, img.w, img.h), (0, 0, 4, 1));
        // Pixel centers at uv.x = 0.125, 0.375, 0.625, 0.875.
        assert_eq!([px(&img, 0, 0)[0], px(&img, 1, 0)[0], px(&img, 2, 0)[0], px(&img, 3, 0)[0]], [32, 96, 159, 223]);
        // pos and size are in points: at scale 2, a 4-pixel box is 2 points.
        let t = Target { scale: 2.0, ..target(4.0, 2.0) };
        let img = paint("vec3(pos.x / size.x, 0.0, 0.0)", &t).unwrap().unwrap();
        assert_eq!(px(&img, 1, 0), [96, 0, 0, 255]);
    }

    #[test]
    fn alpha_is_premultiplied_and_corners_are_round() {
        let img = paint("#ff000080", &target(2.0, 2.0)).unwrap().unwrap();
        assert_eq!(px(&img, 0, 0), [128, 0, 0, 128]);
        let t = Target { radius: 10.0, ..target(20.0, 20.0) };
        let img = paint("#fff", &t).unwrap().unwrap();
        assert_eq!(px(&img, 0, 0)[3], 0);
        assert_eq!(px(&img, 10, 10)[3], 255);
    }

    #[test]
    fn half_pixels_and_clip() {
        // A box from 0.5 to 2.5: the edge pixels are half covered.
        let t = Target { rect: [0.5, 0.0, 2.5, 1.0], ..target(0.0, 0.0) };
        let img = paint("#fff", &t).unwrap().unwrap();
        assert_eq!((img.x, img.w), (0, 3));
        assert_eq!([px(&img, 0, 0)[3], px(&img, 1, 0)[3], px(&img, 2, 0)[3]], [128, 255, 128]);
        let t = Target { clip: [1, 0, 2, 1], ..t };
        let img = paint("#fff", &t).unwrap().unwrap();
        assert_eq!((img.x, img.w), (1, 1));
        assert!(paint("#fff", &Target { clip: [5, 5, 6, 6], ..t }).unwrap().is_none());
    }

    #[test]
    fn functions_match_glsl() {
        let one = |src: &str| px(&paint(src, &target(1.0, 1.0)).unwrap().unwrap(), 0, 0)[0];
        assert_eq!(one("mod(-0.25, 1.0)"), 191); // 0.75: GLSL's mod follows the divisor's sign
        assert_eq!(one("smoothstep(0.0, 1.0, 0.5)"), 128);
        assert_eq!(one("step(0.5, 0.4)"), 0);
        assert_eq!(one("length(vec2(0.3, 0.4))"), 128);
        assert_eq!(one("dot(vec3(1.0), vec3(0.1, 0.2, 0.2))"), 128);
        assert_eq!(one("cross(vec3(1, 0, 0), vec3(0, 1, 0)).z"), 255);
        assert_eq!(one("reflect(vec2(1.0, -1.0), vec2(0.0, 1.0)).y"), 255);
        assert_eq!(one("1.0 != 1.0 ? 1.0 : 0.25"), 64);
        assert_eq!(one("let a = 0.2; let a = a * 2.0; a + 0.1"), 128);
    }

    #[test]
    fn a_mistake_is_reported_once() {
        let t = target(1.0, 1.0);
        assert!(matches!(paint("vec2(1.0)", &t), Err(Some(_))));
        assert!(matches!(paint("vec2(1.0)", &t), Err(None)));
    }

    #[test]
    fn threads_give_the_same_pixels() {
        let src = "let p = pos / 20.0; 0.5 + 0.5 * sin(vec3(p.x + p.y, p.x - p.y, p.x * p.y))";
        let big = paint(src, &target(300.0, 200.0)).unwrap().unwrap();
        // A small slice is painted on one thread.
        let t = Target { clip: [0, 150, 300, 160], ..target(300.0, 200.0) };
        let slice = paint(src, &t).unwrap().unwrap();
        assert_eq!(&big.rgba[150 * 300 * 4..160 * 300 * 4], &slice.rgba[..]);
    }
}

#[cfg(test)]
mod bench {
    use super::*;

    /// `cargo test -p uison-shader --release -- --ignored --nocapture bench`
    #[test]
    #[ignore]
    fn bench() {
        let progs = [
            ("gradient", "mix(#3b5bdb, #9c36b5, uv.x)"),
            ("rings", "let d = distance(uv, vec2(0.5)); mix(#0b7285, #c5f6fa, 0.5 + 0.5 * sin(d * 40.0 - time * 4.0))"),
            ("plasma", "let p = pos / 20.0; let v = (sin(p.x + time) + sin(p.y * 1.3 - time * 0.7) + sin((p.x + p.y) * 0.7 + time * 1.3)) * pi; 0.5 + 0.5 * sin(vec3(v, v + 2.0, v + 4.0))"),
        ];
        for (name, src) in progs {
            let p = program(src).unwrap();
            for (label, cores) in [("1 thread ", 1), ("threads  ", 64)] {
                let clip = [0, 0, 440, 300];
                let t = Target { rect: [0.0, 0.0, 440.0, 300.0], scale: 2.0, time: 1.0, radius: 24.0, clip };
                let n = 20;
                let t0 = std::time::Instant::now();
                for _ in 0..n {
                    std::hint::black_box(render_on(&p, &t, cores));
                }
                let px = ((clip[2] - clip[0]) * (clip[3] - clip[1])) as f64;
                let ns = t0.elapsed().as_nanos() as f64 / n as f64 / px;
                eprintln!("{name:9} {label} {ns:6.1} ns/px");
            }
        }
    }
}
