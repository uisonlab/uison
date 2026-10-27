//! UI state maintained by the runtime (not the data model):
//! hover, press, focus, caret, scrolling.

use std::collections::HashMap;

/// State flags for `{ state: {...} }` selectors.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Flags {
    pub hover: bool,
    pub pressed: bool,
    pub focus: bool,
    pub disabled: bool,
}

#[derive(Default)]
pub struct UiState {
    /// Node under the cursor and its interactive ancestors.
    pub hover: Vec<String>,
    pub pressed: Option<String>,
    pub focus: Option<String>,
    /// Caret in the focused field (byte index) and its visibility (blinking).
    pub caret: usize,
    pub caret_on: bool,
    /// Other end of the selection in the focused field; equal to the caret — no selection.
    pub anchor: usize,
    /// Field in which a selection is being dragged with the mouse.
    pub selecting: Option<String>,
    /// Horizontal shift of long text in the focused field (key, pt) — from the previous frame.
    pub input_shift: Option<(String, f32)>,
    /// Scroll offset by key.
    pub scroll: HashMap<String, f32>,
    /// Overscroll past the edge (bounce): < 0 — above the start, > 0 — past the end.
    pub over: HashMap<String, f32>,
    /// Scroll area height from the previous layout — for List virtualization.
    pub viewport: HashMap<String, f32>,
    /// Node being dragged (Slider).
    pub dragging: Option<String>,
}

/// `k` is the `key` itself or its descendant (`key/...`).
fn within(k: &str, key: &str) -> bool {
    k.len() >= key.len() && k.starts_with(key) && (k.len() == key.len() || k.as_bytes()[key.len()] == b'/')
}

impl UiState {
    /// Element state: hover, press and focus also count for descendants
    /// (like `:hover`/`:focus-within` in CSS) — a focused input field highlights
    /// the border of the whole TextField.
    pub fn flags(&self, key: &str, disabled: bool) -> Flags {
        Flags {
            hover: !disabled && self.hover.iter().any(|h| within(h, key)),
            pressed: !disabled && self.pressed.as_deref().is_some_and(|p| within(p, key)),
            focus: !disabled && self.focus.as_deref().is_some_and(|f| within(f, key)),
            disabled,
        }
    }
}
