//! The program → a Vulkan fragment shader in SPIR-V 1.0, written word by word.
//!
//! A module is a list of 32-bit words: a header, then instructions `(words << 16) | opcode` with
//! their operands, in a fixed order of sections. Values are SSA ids; types and constants are
//! declared once and reused. Opcode and enum numbers are from `spirv.h` and `GLSL.std.450.h`.
//!
//! The interface (it matches `ui.vert` of `uison-vulkan-windows`):
//! - location 0, flat vec4: the item's box in window pixels (x0, y0, x1, y1);
//! - location 2, flat vec4: (time, pixels per point, 0, 0);
//! - location 3, flat vec4: (corner radius in pixels, …);
//! - output location 0: the color, premultiplied and multiplied by the box's coverage of the
//!   pixel, so edges and rounded corners are antialiased as in the rest of the frame.

use std::collections::HashMap;

use crate::lang::{Cmp, Error, Expr, Func, Input, Kind, Op, Program, Ty, parse};

/// Compile a program into a fragment shader module.
pub fn compile(src: &str) -> Result<Vec<u32>, Error> {
    Ok(fragment(&parse(src)?))
}

// Opcodes.
const OP_EXT_INST_IMPORT: u16 = 11;
const OP_EXT_INST: u16 = 12;
const OP_MEMORY_MODEL: u16 = 14;
const OP_ENTRY_POINT: u16 = 15;
const OP_EXECUTION_MODE: u16 = 16;
const OP_CAPABILITY: u16 = 17;
const OP_TYPE_VOID: u16 = 19;
const OP_TYPE_BOOL: u16 = 20;
const OP_TYPE_FLOAT: u16 = 22;
const OP_TYPE_VECTOR: u16 = 23;
const OP_TYPE_POINTER: u16 = 32;
const OP_TYPE_FUNCTION: u16 = 33;
const OP_CONSTANT: u16 = 43;
const OP_CONSTANT_COMPOSITE: u16 = 44;
const OP_FUNCTION: u16 = 54;
const OP_FUNCTION_END: u16 = 56;
const OP_VARIABLE: u16 = 59;
const OP_LOAD: u16 = 61;
const OP_STORE: u16 = 62;
const OP_DECORATE: u16 = 71;
const OP_VECTOR_SHUFFLE: u16 = 79;
const OP_COMPOSITE_CONSTRUCT: u16 = 80;
const OP_COMPOSITE_EXTRACT: u16 = 81;
const OP_F_NEGATE: u16 = 127;
const OP_F_ADD: u16 = 129;
const OP_F_SUB: u16 = 131;
const OP_F_MUL: u16 = 133;
const OP_F_DIV: u16 = 136;
const OP_F_MOD: u16 = 141;
const OP_DOT: u16 = 148;
const OP_LOGICAL_OR: u16 = 166;
const OP_LOGICAL_AND: u16 = 167;
const OP_LOGICAL_NOT: u16 = 168;
const OP_SELECT: u16 = 169;
const OP_F_ORD_EQUAL: u16 = 180;
const OP_F_ORD_NOT_EQUAL: u16 = 182;
const OP_F_ORD_LESS_THAN: u16 = 184;
const OP_F_ORD_GREATER_THAN: u16 = 186;
const OP_F_ORD_LESS_THAN_EQUAL: u16 = 188;
const OP_F_ORD_GREATER_THAN_EQUAL: u16 = 190;
const OP_LABEL: u16 = 248;
const OP_RETURN: u16 = 253;

// Enums.
const CAPABILITY_SHADER: u32 = 1;
const ADDRESSING_LOGICAL: u32 = 0;
const MEMORY_GLSL450: u32 = 1;
const EXECUTION_MODEL_FRAGMENT: u32 = 4;
const EXECUTION_MODE_ORIGIN_UPPER_LEFT: u32 = 7;
const STORAGE_INPUT: u32 = 1;
const STORAGE_OUTPUT: u32 = 3;
const DECORATION_BUILT_IN: u32 = 11;
const DECORATION_FLAT: u32 = 14;
const DECORATION_LOCATION: u32 = 30;
const BUILT_IN_FRAG_COORD: u32 = 15;

/// GLSL.std.450 instruction numbers.
fn glsl(f: Func) -> u32 {
    match f {
        Func::Round => 1,
        Func::Trunc => 3,
        Func::Abs => 4,
        Func::Sign => 6,
        Func::Floor => 8,
        Func::Ceil => 9,
        Func::Fract => 10,
        Func::Radians => 11,
        Func::Degrees => 12,
        Func::Sin => 13,
        Func::Cos => 14,
        Func::Tan => 15,
        Func::Asin => 16,
        Func::Acos => 17,
        Func::Atan => 18,
        Func::Sinh => 19,
        Func::Cosh => 20,
        Func::Tanh => 21,
        Func::Atan2 => 25,
        Func::Pow => 26,
        Func::Exp => 27,
        Func::Log => 28,
        Func::Exp2 => 29,
        Func::Log2 => 30,
        Func::Sqrt => 31,
        Func::InverseSqrt => 32,
        Func::Min => 37,
        Func::Max => 40,
        Func::Clamp => 43,
        Func::Mix => 46,
        Func::Step => 48,
        Func::SmoothStep => 49,
        Func::Length => 66,
        Func::Distance => 67,
        Func::Cross => 68,
        Func::Normalize => 69,
        Func::Reflect => 71,
        // Core instructions, not GLSL.std.450.
        Func::Mod | Func::Dot => unreachable!(),
    }
}

/// A type as SPIR-V sees it: bool vectors exist here (a vector `OpSelect` needs one in SPIR-V 1.0).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum T {
    Float(u8),
    Bool(u8),
}

fn t(ty: Ty) -> T {
    match ty {
        Ty::Float(n) => T::Float(n),
        Ty::Bool => T::Bool(1),
    }
}

fn op(out: &mut Vec<u32>, code: u16, args: &[u32]) {
    out.push(((args.len() as u32 + 1) << 16) | code as u32);
    out.extend_from_slice(args);
}

/// A string operand: UTF-8, NUL-terminated, padded to whole words.
fn string(s: &str) -> Vec<u32> {
    let mut bytes = s.as_bytes().to_vec();
    bytes.push(0);
    bytes.resize(bytes.len().next_multiple_of(4), 0);
    bytes.chunks(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
}

struct Module {
    next: u32,
    /// Types, constants and global variables: one section, definitions before uses.
    globals: Vec<u32>,
    code: Vec<u32>,
    types: HashMap<T, u32>,
    consts: HashMap<(T, Vec<u32>), u32>,
    glsl: u32,
}

impl Module {
    fn id(&mut self) -> u32 {
        self.next += 1;
        self.next
    }

    fn ty(&mut self, ty: T) -> u32 {
        if let Some(&id) = self.types.get(&ty) {
            return id;
        }
        let id = match ty {
            T::Float(1) => {
                let id = self.id();
                op(&mut self.globals, OP_TYPE_FLOAT, &[id, 32]);
                id
            }
            T::Bool(1) => {
                let id = self.id();
                op(&mut self.globals, OP_TYPE_BOOL, &[id]);
                id
            }
            T::Float(n) | T::Bool(n) => {
                let scalar = self.ty(if matches!(ty, T::Float(_)) { T::Float(1) } else { T::Bool(1) });
                let id = self.id();
                op(&mut self.globals, OP_TYPE_VECTOR, &[id, scalar, n as u32]);
                id
            }
        };
        self.types.insert(ty, id);
        id
    }

    fn float(&mut self, v: f32) -> u32 {
        self.constant(&[v])
    }

    /// A float or float-vector constant.
    fn constant(&mut self, v: &[f32]) -> u32 {
        let ty = T::Float(v.len() as u8);
        let key = (ty, v.iter().map(|x| x.to_bits()).collect::<Vec<_>>());
        if let Some(&id) = self.consts.get(&key) {
            return id;
        }
        let id = if v.len() == 1 {
            let tid = self.ty(ty);
            let id = self.id();
            op(&mut self.globals, OP_CONSTANT, &[tid, id, v[0].to_bits()]);
            id
        } else {
            let parts: Vec<u32> = v.iter().map(|&x| self.float(x)).collect();
            let tid = self.ty(ty);
            let id = self.id();
            let mut args = vec![tid, id];
            args.extend(parts);
            op(&mut self.globals, OP_CONSTANT_COMPOSITE, &args);
            id
        };
        self.consts.insert(key, id);
        id
    }

    /// An instruction with a result: `id = op args` of type `ty`.
    fn emit(&mut self, code: u16, ty: T, args: &[u32]) -> u32 {
        let tid = self.ty(ty);
        let id = self.id();
        let mut all = vec![tid, id];
        all.extend_from_slice(args);
        op(&mut self.code, code, &all);
        id
    }

    fn ext(&mut self, ty: T, f: u32, args: &[u32]) -> u32 {
        let mut all = vec![self.glsl, f];
        all.extend_from_slice(args);
        self.emit(OP_EXT_INST, ty, &all)
    }

    fn splat(&mut self, v: u32, ty: T) -> u32 {
        match ty {
            T::Float(1) | T::Bool(1) => v,
            T::Float(n) | T::Bool(n) => self.emit(OP_COMPOSITE_CONSTRUCT, ty, &vec![v; n as usize]),
        }
    }

    fn component(&mut self, v: u32, i: u32) -> u32 {
        self.emit(OP_COMPOSITE_EXTRACT, T::Float(1), &[v, i])
    }

    fn shuffle(&mut self, v: u32, idx: &[u32]) -> u32 {
        let mut args = vec![v, v];
        args.extend_from_slice(idx);
        self.emit(OP_VECTOR_SHUFFLE, T::Float(idx.len() as u8), &args)
    }

    fn arith(&mut self, code: u16, ty: T, a: u32, b: u32) -> u32 {
        self.emit(code, ty, &[a, b])
    }
}

struct Values {
    uv: u32,
    pos: u32,
    size: u32,
    time: u32,
    lets: Vec<u32>,
}

impl Module {
    fn expr(&mut self, e: &Expr, v: &Values) -> u32 {
        let ty = t(e.ty);
        match &e.kind {
            Kind::Num(x) => self.float(*x),
            Kind::Const(c) => self.constant(c),
            Kind::Input(Input::Uv) => v.uv,
            Kind::Input(Input::Pos) => v.pos,
            Kind::Input(Input::Size) => v.size,
            Kind::Input(Input::Time) => v.time,
            Kind::Let(i) => v.lets[*i],
            Kind::Neg(a) => {
                let a = self.expr(a, v);
                self.emit(OP_F_NEGATE, ty, &[a])
            }
            Kind::Not(a) => {
                let a = self.expr(a, v);
                self.emit(OP_LOGICAL_NOT, ty, &[a])
            }
            Kind::Splat(a) => {
                let a = self.expr(a, v);
                self.splat(a, ty)
            }
            Kind::Arith(o, a, b) => {
                let (a, b) = (self.expr(a, v), self.expr(b, v));
                let code = match o {
                    Op::Add => OP_F_ADD,
                    Op::Sub => OP_F_SUB,
                    Op::Mul => OP_F_MUL,
                    Op::Div => OP_F_DIV,
                };
                self.arith(code, ty, a, b)
            }
            Kind::Compare(c, a, b) => {
                let (a, b) = (self.expr(a, v), self.expr(b, v));
                let code = match c {
                    Cmp::Lt => OP_F_ORD_LESS_THAN,
                    Cmp::Le => OP_F_ORD_LESS_THAN_EQUAL,
                    Cmp::Gt => OP_F_ORD_GREATER_THAN,
                    Cmp::Ge => OP_F_ORD_GREATER_THAN_EQUAL,
                    Cmp::Eq => OP_F_ORD_EQUAL,
                    Cmp::Ne => OP_F_ORD_NOT_EQUAL,
                };
                self.arith(code, ty, a, b)
            }
            Kind::And(a, b) => {
                let (a, b) = (self.expr(a, v), self.expr(b, v));
                self.arith(OP_LOGICAL_AND, ty, a, b)
            }
            Kind::Or(a, b) => {
                let (a, b) = (self.expr(a, v), self.expr(b, v));
                self.arith(OP_LOGICAL_OR, ty, a, b)
            }
            Kind::Select(c, a, b) => {
                let (c, a, b) = (self.expr(c, v), self.expr(a, v), self.expr(b, v));
                // SPIR-V 1.0: a vector result needs a bool vector of the same size.
                let c = match ty {
                    T::Float(n) => self.splat(c, T::Bool(n)),
                    T::Bool(_) => c,
                };
                self.emit(OP_SELECT, ty, &[c, a, b])
            }
            Kind::Component(a, i) => {
                let a = self.expr(a, v);
                self.component(a, *i as u32)
            }
            Kind::Swizzle(a, idx) => {
                let a = self.expr(a, v);
                let idx: Vec<u32> = idx.iter().map(|&i| i as u32).collect();
                self.shuffle(a, &idx)
            }
            Kind::Construct(parts) => {
                let ids: Vec<u32> = parts.iter().map(|p| self.expr(p, v)).collect();
                self.emit(OP_COMPOSITE_CONSTRUCT, ty, &ids)
            }
            Kind::Call(f, args) => {
                let ids: Vec<u32> = args.iter().map(|a| self.expr(a, v)).collect();
                match f {
                    Func::Mod => self.arith(OP_F_MOD, ty, ids[0], ids[1]),
                    // OpDot takes vectors only.
                    Func::Dot if args[0].ty == Ty::FLOAT => self.arith(OP_F_MUL, ty, ids[0], ids[1]),
                    Func::Dot => self.arith(OP_DOT, ty, ids[0], ids[1]),
                    f => self.ext(ty, glsl(*f), &ids),
                }
            }
        }
    }
}

/// The program as a whole fragment shader.
pub fn fragment(p: &Program) -> Vec<u32> {
    let mut m = Module { next: 0, globals: Vec::new(), code: Vec::new(), types: HashMap::new(), consts: HashMap::new(), glsl: 0 };
    m.glsl = m.id();
    let main = m.id();
    let void = m.id();
    op(&mut m.globals, OP_TYPE_VOID, &[void]);
    let fn_ty = m.id();
    op(&mut m.globals, OP_TYPE_FUNCTION, &[fn_ty, void]);
    let (f1, f2, f4) = (T::Float(1), T::Float(2), T::Float(4));
    let vec4 = m.ty(f4);
    let in_ptr = m.id();
    op(&mut m.globals, OP_TYPE_POINTER, &[in_ptr, STORAGE_INPUT, vec4]);
    let out_ptr = m.id();
    op(&mut m.globals, OP_TYPE_POINTER, &[out_ptr, STORAGE_OUTPUT, vec4]);
    let var = |m: &mut Module, ptr: u32, storage: u32| {
        let id = m.id();
        op(&mut m.globals, OP_VARIABLE, &[ptr, id, storage]);
        id
    };
    let frag_in = var(&mut m, in_ptr, STORAGE_INPUT);
    let rect_in = var(&mut m, in_ptr, STORAGE_INPUT);
    let extra_in = var(&mut m, in_ptr, STORAGE_INPUT);
    let param_in = var(&mut m, in_ptr, STORAGE_INPUT);
    let out = var(&mut m, out_ptr, STORAGE_OUTPUT);

    // ---- main ----
    let label = m.id();
    op(&mut m.code, OP_FUNCTION, &[void, main, 0, fn_ty]);
    op(&mut m.code, OP_LABEL, &[label]);
    let frag = m.emit(OP_LOAD, f4, &[frag_in]);
    let rect = m.emit(OP_LOAD, f4, &[rect_in]);
    let extra = m.emit(OP_LOAD, f4, &[extra_in]);
    let param = m.emit(OP_LOAD, f4, &[param_in]);
    let px = m.shuffle(frag, &[0, 1]);
    let lo = m.shuffle(rect, &[0, 1]);
    let hi = m.shuffle(rect, &[2, 3]);
    let box_px = m.arith(OP_F_SUB, f2, hi, lo);
    let from_lo = m.arith(OP_F_SUB, f2, px, lo);
    let time = m.component(extra, 0);
    let scale = m.component(extra, 1);
    let scale2 = m.splat(scale, f2);
    let mut values = Values {
        uv: m.arith(OP_F_DIV, f2, from_lo, box_px),
        pos: m.arith(OP_F_DIV, f2, from_lo, scale2),
        size: m.arith(OP_F_DIV, f2, box_px, scale2),
        time,
        lets: Vec::new(),
    };
    for e in &p.lets {
        let id = m.expr(e, &values);
        values.lets.push(id);
    }
    let c = m.expr(&p.result, &values);
    // To vec4 with straight alpha, kept in 0..1.
    let one = m.float(1.0);
    let c = match p.result.ty {
        Ty::Float(1) => m.emit(OP_COMPOSITE_CONSTRUCT, f4, &[c, c, c, one]),
        Ty::Float(3) => m.emit(OP_COMPOSITE_CONSTRUCT, f4, &[c, one]),
        _ => c,
    };
    let (zero4, one4) = (m.constant(&[0.0; 4]), m.constant(&[1.0; 4]));
    let c = m.ext(f4, glsl(Func::Clamp), &[c, zero4, one4]);
    let alpha = m.component(c, 3);
    let rgb = m.shuffle(c, &[0, 1, 2]);
    let alpha3 = m.splat(alpha, T::Float(3));
    let rgb = m.arith(OP_F_MUL, T::Float(3), rgb, alpha3);
    let color = m.emit(OP_COMPOSITE_CONSTRUCT, f4, &[rgb, alpha]);

    // ---- coverage of the pixel by the box, as `box()` in ui.frag ----
    let (zero, half) = (m.float(0.0), m.float(0.5));
    let clamp01 = |m: &mut Module, x: u32| m.ext(f1, glsl(Func::Clamp), &[x, zero, one]);
    // Square corners: the overlap of the pixel and the box along each axis, exact for straight edges.
    let span = |m: &mut Module, axis: u32| {
        let pc = m.component(px, axis);
        let (l, h) = (m.component(lo, axis), m.component(hi, axis));
        let pa = m.arith(OP_F_ADD, f1, pc, half);
        let pb = m.arith(OP_F_SUB, f1, pc, half);
        let top = m.ext(f1, glsl(Func::Min), &[pa, h]);
        let bottom = m.ext(f1, glsl(Func::Max), &[pb, l]);
        let d = m.arith(OP_F_SUB, f1, top, bottom);
        clamp01(m, d)
    };
    let sx = span(&mut m, 0);
    let sy = span(&mut m, 1);
    let square = m.arith(OP_F_MUL, f1, sx, sy);
    // Rounded corners: a signed distance to the rounded box.
    let half2 = m.splat(half, f2);
    let b = m.arith(OP_F_MUL, f2, box_px, half2);
    let (bx, by) = (m.component(b, 0), m.component(b, 1));
    let bmin = m.ext(f1, glsl(Func::Min), &[bx, by]);
    let radius_in = m.component(param, 0);
    let rad = m.ext(f1, glsl(Func::Min), &[radius_in, bmin]);
    let sum = m.arith(OP_F_ADD, f2, lo, hi);
    let center = m.arith(OP_F_MUL, f2, sum, half2);
    let rel = m.arith(OP_F_SUB, f2, px, center);
    let rel = m.ext(f2, glsl(Func::Abs), &[rel]);
    let q = m.arith(OP_F_SUB, f2, rel, b);
    let rad2 = m.splat(rad, f2);
    let q = m.arith(OP_F_ADD, f2, q, rad2);
    let zero2 = m.constant(&[0.0; 2]);
    let outside = m.ext(f2, glsl(Func::Max), &[q, zero2]);
    let outside = m.ext(f1, glsl(Func::Length), &[outside]);
    let (qx, qy) = (m.component(q, 0), m.component(q, 1));
    let qmax = m.ext(f1, glsl(Func::Max), &[qx, qy]);
    let inside = m.ext(f1, glsl(Func::Min), &[qmax, zero]);
    let d = m.arith(OP_F_ADD, f1, outside, inside);
    let d = m.arith(OP_F_SUB, f1, d, rad);
    let d = m.arith(OP_F_SUB, f1, half, d);
    let round = clamp01(&mut m, d);
    let rounded = m.arith(OP_F_ORD_GREATER_THAN, T::Bool(1), rad, zero);
    let coverage = m.emit(OP_SELECT, f1, &[rounded, round, square]);
    let coverage = m.splat(coverage, f4);
    let result = m.arith(OP_F_MUL, f4, color, coverage);
    op(&mut m.code, OP_STORE, &[out, result]);
    op(&mut m.code, OP_RETURN, &[]);
    op(&mut m.code, OP_FUNCTION_END, &[]);

    // ---- the module, sections in the required order ----
    let mut w = vec![0x0723_0203, 0x0001_0000, 0, m.next + 1, 0];
    op(&mut w, OP_CAPABILITY, &[CAPABILITY_SHADER]);
    let mut args = vec![m.glsl];
    args.extend(string("GLSL.std.450"));
    op(&mut w, OP_EXT_INST_IMPORT, &args);
    op(&mut w, OP_MEMORY_MODEL, &[ADDRESSING_LOGICAL, MEMORY_GLSL450]);
    let mut args = vec![EXECUTION_MODEL_FRAGMENT, main];
    args.extend(string("main"));
    args.extend([frag_in, rect_in, extra_in, param_in, out]);
    op(&mut w, OP_ENTRY_POINT, &args);
    op(&mut w, OP_EXECUTION_MODE, &[main, EXECUTION_MODE_ORIGIN_UPPER_LEFT]);
    op(&mut w, OP_DECORATE, &[frag_in, DECORATION_BUILT_IN, BUILT_IN_FRAG_COORD]);
    for (var, location) in [(rect_in, 0), (extra_in, 2), (param_in, 3)] {
        op(&mut w, OP_DECORATE, &[var, DECORATION_LOCATION, location]);
        op(&mut w, OP_DECORATE, &[var, DECORATION_FLAT]);
    }
    op(&mut w, OP_DECORATE, &[out, DECORATION_LOCATION, 0]);
    w.extend(&m.globals);
    w.extend(&m.code);
    w[3] = m.next + 1; // the id bound: every id is below it
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Programs that use every part of the language.
    const PROGRAMS: &[&str] = &[
        "#3b5bdb",
        "uv.x",
        "vec3(uv, 0.5)",
        "let d = distance(uv, vec2(0.5)); mix(#3b5bdb, #9c36b5, smoothstep(0.2, 0.3, d))",
        "let a = atan(uv.y - 0.5, uv.x - 0.5) / (2.0 * pi) + 0.5; vec3(fract(a + time * 0.1), 0.6, 0.9).zyx",
        "uv.x > 0.5 && !(uv.y < 0.25) || time == 0.0 ? #fff : vec4(uv.yx, uv.x.xx).abgr * 0.5",
        "let g = pos / 8.0; mod(floor(g.x) + floor(g.y), 2.0) < 1.0 ? #eee : #ccc",
        "let n = normalize(vec3(uv * 2.0 - 1.0, 1.0)); vec3(dot(n, vec3(0.3)), length(cross(n, vec3(0, 1, 0))), reflect(n, n).z) * \
         clamp(pow(abs(sin(time)), 2.0), 0.0, 1.0) + vec3(step(0.5, size.x / size.y), exp2(-1.0), sqrt(inversesqrt(4.0)))",
        "abs(sign(-1.0)) * floor(1.5) * ceil(0.5) * round(0.4) + trunc(1.0) + cos(1.0) + tan(0.1) + asin(0.1) + acos(0.1) + \
         sinh(0.1) + cosh(0.1) + tanh(0.1) + exp(0.1) + log(2.0) + log2(2.0) + radians(1.0) + degrees(1.0) + min(1.0, 2.0) + \
         max(1.0, 2.0) + dot(2.0, 3.0) + length(-1.0) + distance(1.0, 2.0) + 1e-3 + .5",
    ];

    #[test]
    fn header_and_bound() {
        let w = compile("#fff").unwrap();
        assert_eq!(&w[..3], &[0x0723_0203, 0x0001_0000, 0]);
        // The bound is above every result id (a sanity check without the SDK).
        assert!(w[3] > 10);
        assert_eq!(w[4], 0);
    }

    #[test]
    fn same_constants_once() {
        let a = compile("vec3(0.5, 0.5, 0.5)").unwrap();
        let b = compile("vec3(0.5, 0.25, 0.75)").unwrap();
        assert!(a.len() < b.len());
    }

    /// Every program passes `spirv-val` from the Vulkan SDK (skipped where the SDK isn't installed).
    #[test]
    fn valid_spirv() {
        let Some(sdk) = std::env::var_os("VULKAN_SDK") else {
            eprintln!("VULKAN_SDK is not set: SPIR-V validation skipped");
            return;
        };
        let val = std::path::Path::new(&sdk).join("Bin").join(if cfg!(windows) { "spirv-val.exe" } else { "spirv-val" });
        let dir = std::env::temp_dir().join(format!("uison-shader-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for (i, src) in PROGRAMS.iter().enumerate() {
            let words = compile(src).unwrap_or_else(|e| panic!("{src}: {e}"));
            let path = dir.join(format!("{i}.spv"));
            std::fs::write(&path, words.iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<u8>>()).unwrap();
            let out = std::process::Command::new(&val).args(["--target-env", "vulkan1.0"]).arg(&path).output().unwrap();
            assert!(out.status.success(), "{src}\n{}", String::from_utf8_lossy(&out.stderr));
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
