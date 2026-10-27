//! Live reload of the description (`UISON_RELOAD=folder`): the files `Ui` was built from
//! are read from the folder and checked every 300 ms (polling the modification time works the same
//! on all desktop systems). Debug builds only: release has none of this code.

use std::time::{Duration, SystemTime};

use crate::Source;

/// How often to look at the files.
pub const POLL: Duration = Duration::from_millis(300);

/// What the file was for `Ui`: a screen or a set of components.
#[derive(Clone, Copy, PartialEq)]
pub enum Role {
    Screen,
    Components,
}

pub struct Watched {
    pub role: Role,
    pub path: std::path::PathBuf,
    pub modified: Option<SystemTime>,
}

/// The folder from `UISON_RELOAD`, if set.
pub fn dir() -> Option<std::path::PathBuf> {
    std::env::var_os("UISON_RELOAD").filter(|d| !d.is_empty()).map(Into::into)
}

/// The source file in the reload folder (by the name from `source!`).
pub fn path(src: &Source) -> Option<std::path::PathBuf> {
    let name = std::path::Path::new(src.name).file_name()?;
    Some(dir()?.join(name))
}

pub fn modified(p: &std::path::Path) -> Option<SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

/// Source text: from disk if reload is on and the file exists, otherwise the embedded one.
pub fn text(src: &Source) -> String {
    match path(src).map(|p| std::fs::read_to_string(&p).map_err(|e| (p, e))) {
        Some(Ok(t)) => t,
        Some(Err((p, e))) => {
            eprintln!("uison-ui: UISON_RELOAD: {}: {e} — using the built-in copy", p.display());
            src.text.to_string()
        }
        None => src.text.to_string(),
    }
}
