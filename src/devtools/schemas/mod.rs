//! Schemas bundled into the binary (offline). See `docs/devtools-credits.md`.

/// Embedded schema text for a file, matched by file name / path.
pub fn bundled_for(_path: &std::path::Path) -> Option<&'static str> {
    None
}
