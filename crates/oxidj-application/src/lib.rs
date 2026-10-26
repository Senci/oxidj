//! Shared application handlers for CLI and future UI callers.

use std::path::Path;

pub use oxidj_rekordbox::{Error as RekordboxError, Preview};

/// Read-only preview handler; parsing and vendor-specific semantics stay in the adapter.
pub fn preview_rekordbox_xml(path: impl AsRef<Path>) -> Result<Preview, RekordboxError> {
    oxidj_rekordbox::parse_file(path)
}
