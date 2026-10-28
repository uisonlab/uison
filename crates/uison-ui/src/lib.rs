//! Element library: complex JSON (templates, layout, data bindings)
//! compiles into the stage 1 primitive frame.
//!
//! - [`Ui`] — runtime: screen + data + action handlers in Rust.
//! - [`dsl`] — the same tree from Rust, Flutter-style.
//! - `lib/*.json` — the standard element library and themes.

pub mod dsl;
pub mod schema;

mod data;
mod edit;
mod el;
mod emit;
mod expand;
mod flow;
mod gesture;
mod layout;
mod library;
#[cfg(all(any(debug_assertions, feature = "reload"), not(target_arch = "wasm32")))]
mod reload;
mod sem;
mod state;
mod testing;
mod text;
mod ui;
mod values;

use std::fmt;

pub use data::{Data, FromJson, IntoJson, obj};
pub use library::{Component, Library};
pub use text::WrapMode;
pub use ui::{Cx, Draw, Handle, NodeMut, Stats, Tree, Ui};
pub use values::literal;

/// A UI description file: the embedded text and the name `UISON_RELOAD` uses to find the file on disk.
/// Created by the [`source!`] macro.
#[derive(Clone, Copy)]
pub struct Source {
    pub name: &'static str,
    pub text: &'static str,
}

/// `source!("../app.json")` — embed the file (`include_str!`) and remember its name for live reload.
#[macro_export]
macro_rules! source {
    ($path:literal) => {
        $crate::Source { name: $path, text: include_str!($path) }
    };
}

/// An owned JSON tree: a screen, data, properties.
pub type Json = uison_json::Value<'static>;

/// A UI description error, with the node location (`Button at root/0/3: ...`).
#[derive(Clone, Debug, PartialEq)]
pub struct Error {
    msg: String,
    located: bool,
}

impl Error {
    pub fn new(msg: impl Into<String>) -> Error {
        Error { msg: msg.into(), located: false }
    }

    /// Set the location unless it is already set (the deepest one matters).
    pub(crate) fn at(self, key: &str) -> Error {
        if self.located { self } else { Error { msg: format!("at {key}: {}", self.msg), located: true } }
    }

    pub(crate) fn in_node(self, kind: &str, key: &str) -> Error {
        if self.located { self } else { Error { msg: format!("{kind} at {key}: {}", self.msg), located: true } }
    }

    pub fn message(&self) -> &str {
        &self.msg
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.msg)
    }
}

impl std::error::Error for Error {}

impl From<uison_json::Error> for Error {
    fn from(e: uison_json::Error) -> Error {
        Error::new(e.to_string())
    }
}

#[cfg(test)]
mod tests;
