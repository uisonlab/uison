//! Component and theme library. The standard library is JSON in `lib/`,
//! embedded in the binary.

use std::collections::HashMap;

use crate::{Error, Json};

/// A template: parameters with defaults, and the body.
#[derive(Clone, Debug)]
pub struct Component {
    pub params: Vec<(String, Json)>,
    /// The whole body root is a clickable area (hover/pressed for the whole element).
    pub interactive: bool,
    pub body: Json,
}

/// Names of built-in nodes: a template can't override them.
pub const BUILTINS: [&str; 21] = [
    "Box", "Row", "Column", "Stack", "Center", "Align", "Spacer", "Wrap", "Flow", "Scroll", "List", "Text", "Input", "Image", "Rect",
    "Circle", "Line", "Tri", "Vector", "Canvas", "For",
];

#[derive(Default)]
pub struct Library {
    pub components: HashMap<String, Component>,
    pub themes: HashMap<String, Json>,
}

const STD_THEME: &str = include_str!("../lib/theme.json");
const STD_COMPONENTS: &str = include_str!("../lib/components.json");
const STD_ICONS: &str = include_str!("../lib/icons.json");

/// Sources of the standard library components (file, text), in file order.
pub(crate) const STD_SOURCES: [(&str, &str); 2] = [("lib/components.json", STD_COMPONENTS), ("lib/icons.json", STD_ICONS)];

impl Library {
    /// The standard library: `light`/`dark` themes, components, icons.
    pub fn standard() -> Library {
        let mut lib = Library::default();
        let load = |src: &str, file: &str| -> Json {
            uison_json::parse(src).unwrap_or_else(|e| panic!("{file}:{e}")).into_owned()
        };
        for (name, tokens) in load(STD_THEME, "lib/theme.json").as_object().unwrap() {
            lib.themes.insert(name.to_string(), tokens.clone());
        }
        for (file, src) in STD_SOURCES {
            lib.add_components(&load(src, file)).unwrap_or_else(|e| panic!("{file}: {e}"));
        }
        lib
    }

    /// Add templates from an object `{ Name: { params, interactive, body } }`.
    pub fn add_components(&mut self, defs: &Json) -> Result<(), Error> {
        let defs = defs.as_object().ok_or_else(|| Error::new("components must be an object"))?;
        for (name, def) in defs {
            self.add_component(name, def)?;
        }
        Ok(())
    }

    pub fn add_component(&mut self, name: &str, def: &Json) -> Result<(), Error> {
        let err = |m: &str| Error::new(format!("component {name}: {m}"));
        if !name.starts_with(|c: char| c.is_ascii_uppercase()) {
            return Err(err("name must start with an uppercase letter"));
        }
        if BUILTINS.contains(&name) {
            return Err(err("name is taken by a built-in node"));
        }
        let fields = def.as_object().ok_or_else(|| err("definition must be an object"))?;
        let mut comp = Component { params: Vec::new(), interactive: false, body: Json::Null };
        for (k, v) in fields {
            match &**k {
                "params" => {
                    let ps = v.as_object().ok_or_else(|| err("params must be an object"))?;
                    comp.params = ps.iter().map(|(k, v)| (k.to_string(), v.clone())).collect();
                }
                "interactive" => comp.interactive = v.truthy(),
                "body" => comp.body = v.clone(),
                _ => return Err(err(&format!("unknown key '{k}' (expected params, interactive, body)"))),
            }
        }
        if comp.body.is_null() {
            return Err(err("body is required"));
        }
        self.components.insert(name.to_string(), comp);
        Ok(())
    }
}
