//! Generated files (diff reports, stdin buffers, link reports).
//!
//! They live in a per-user cache folder rather than the shared system temp
//! directory, and each file is created exclusively with a unique name, so an
//! existing file or planted symlink is never followed or overwritten.

use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// Per-user folder for generated files: `<cache>/ferrite/generated/<sub>`
/// (falls back to the system temp directory when no cache dir is known).
pub fn dir(sub: &str) -> PathBuf {
    dirs::cache_dir()
        .map(|c| c.join("ferrite").join("generated"))
        .unwrap_or_else(|| std::env::temp_dir().join("ferrite-generated"))
        .join(sub)
}

/// Keep only characters that are safe in a file name.
fn safe(stem: &str) -> String {
    let s: String = stem
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .take(80)
        .collect();
    if s.is_empty() {
        "file".to_string()
    } else {
        s
    }
}

/// Write `data` to a new file `<sub>/<stem>-<unique>.<ext>` and return its path.
pub fn write_new(sub: &str, stem: &str, ext: &str, data: &[u8]) -> std::io::Result<PathBuf> {
    write_new_in(&dir(sub), stem, ext, data)
}

/// Like [`write_new`], in an explicit directory.
pub fn write_new_in(
    dir: &std::path::Path,
    stem: &str,
    ext: &str,
    data: &[u8],
) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let stem = safe(stem);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    for _ in 0..100 {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let name = format!("{stem}-{}-{nanos:08x}-{n}.{ext}", std::process::id());
        let path = dir.join(name);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut f) => {
                f.write_all(data)?;
                return Ok(path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "could not pick a unique file name",
    ))
}
