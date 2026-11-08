//! uison's shading language: what the frame's `shader` item is painted with. A small GLSL-like
//! expression language, so one program means the same thing on every back end; `lang` turns
//! the text into a typed tree, `spirv` turns the tree into a Vulkan fragment shader. Other
//! targets (MSL, HLSL, AGSL, a CPU interpreter) will read the same tree.
//!
//! A program is `let` bindings and an expression giving the pixel's color:
//!
//! ```text
//! let d = distance(uv, vec2(0.5));
//! mix(#3b5bdb, #9c36b5, smoothstep(0.25, 0.5, d + 0.05 * sin(time * 3.0)))
//! ```
//!
//! - Types: `float`, `vec2`, `vec3`, `vec4`; `bool` from comparisons, for `?:`, `&&`, `||`, `!`.
//! - Literals: numbers (`1`, `0.5`, `.5`, `2e-3`), colors `#RGB`, `#RGBA`, `#RRGGBB`, `#RRGGBBAA` (vec4).
//! - Inputs: `uv` (vec2, 0..1 across the item, (0, 0) at the top left), `pos` (vec2, points from
//!   the top left), `size` (vec2, the item's size in points), `time` (float, the item's `time`); `pi`.
//! - Operators: `+ - * /` on floats and vectors (a float next to a vector applies to every
//!   component), unary `-`, comparisons `< <= > >= == !=` of floats, `&& || !`, `c ? a : b`,
//!   components `.x .xy .zyx .rgba` (one set, one to four letters).
//! - Constructors: `vec2(…)`, `vec3(…)`, `vec4(…)` from floats and vectors, or one float for all components.
//! - Functions, as in GLSL: abs sign floor ceil fract round trunc, sin cos tan asin acos atan
//!   (`atan(y, x)` too) sinh cosh tanh, pow exp log exp2 log2 sqrt inversesqrt, radians degrees,
//!   mod min max clamp mix step smoothstep, length distance dot cross normalize reflect.
//! - The result: `vec4` (rgba, straight alpha), `vec3` (opaque rgb) or `float` (gray), each
//!   component 0..1. `//` starts a comment.
//!
//! Mistakes come back as [`Error`] with the column, and nothing is drawn.

pub mod cpu;
pub mod lang;
mod pool;
pub mod spirv;

pub use lang::Error;
