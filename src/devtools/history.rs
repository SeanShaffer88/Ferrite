//! Local version history: snapshots of a file taken when it is saved.
//!
//! Layout: `<config>/history/<blake3(canonical path)>/` holds `path.txt`
//! (the file's path, for humans) and one `<unix-millis>[-o].snap` per
//! snapshot with the bytes as written. `-o` marks the original on-disk
//! contents captured before the first save; it is never coalesced away.
//!
//! Saves within [`COALESCE`] of the newest snapshot replace it, so auto-save
//! doesn't push meaningful versions out of the history.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Files larger than this are not snapshotted.
pub const MAX_FILE_BYTES: usize = 5 * 1024 * 1024;
/// Saves closer together than this replace the newest snapshot.
pub const COALESCE: Duration = Duration::from_secs(120);

/// One stored snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Milliseconds since the Unix epoch when the snapshot was taken.
    pub millis: u64,
    /// The original on-disk contents from before the first save.
    pub original: bool,
    pub size: u64,
    file: PathBuf,
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn root() -> Option<PathBuf> {
    crate::config::get_config_dir()
        .ok()
        .map(|d| d.join("history"))
}

/// Folder holding the snapshots of `path`.
pub fn dir_for(path: &Path) -> Option<PathBuf> {
    let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let key = crate::path_utils::normalize_path(canon);
    let hash = blake3::hash(key.to_string_lossy().as_bytes());
    Some(root()?.join(&hash.to_hex()[..32]))
}

fn parse(name: &str) -> Option<(u64, bool)> {
    let stem = name.strip_suffix(".snap")?;
    let (num, original) = match stem.strip_suffix("-o") {
        Some(n) => (n, true),
        None => (stem, false),
    };
    Some((num.parse().ok()?, original))
}

/// Snapshots of `path`, newest first.
pub fn list(path: &Path) -> Vec<Entry> {
    let Some(dir) = dir_for(path) else {
        return Vec::new();
    };
    let Ok(read) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out: Vec<Entry> = read
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let (millis, original) = parse(&name)?;
            let size = e.metadata().map(|m| m.len()).unwrap_or(0);
            Some(Entry {
                millis,
                original,
                size,
                file: e.path(),
            })
        })
        .collect();
    out.sort_by_key(|e| std::cmp::Reverse(e.millis));
    out
}

/// Contents of a snapshot (lossy UTF-8).
pub fn read(entry: &Entry) -> std::io::Result<String> {
    std::fs::read(&entry.file).map(|b| String::from_utf8_lossy(&b).into_owned())
}

/// Delete every snapshot of `path`.
pub fn clear(path: &Path) -> std::io::Result<()> {
    match dir_for(path) {
        Some(dir) if dir.exists() => std::fs::remove_dir_all(dir),
        _ => Ok(()),
    }
}

fn write_snapshot(dir: &Path, millis: u64, original: bool, bytes: &[u8]) -> std::io::Result<()> {
    let name = format!("{millis}{}.snap", if original { "-o" } else { "" });
    std::fs::write(dir.join(name), bytes)
}

/// Record a save of `path` that is about to write `new_bytes`. Call before
/// the file is overwritten: the first time a file is saved, its current disk
/// contents are kept as the original. `max` snapshots are kept (0 = off).
pub fn record_save(path: &Path, new_bytes: &[u8], max: usize) -> std::io::Result<()> {
    if max == 0 || new_bytes.len() > MAX_FILE_BYTES {
        return Ok(());
    }
    let Some(dir) = dir_for(path) else {
        return Ok(());
    };
    std::fs::create_dir_all(&dir)?;
    let marker = dir.join("path.txt");
    if !marker.exists() {
        std::fs::write(&marker, path.to_string_lossy().as_bytes())?;
    }

    let mut entries = list(path);
    let now = now_millis();

    if entries.is_empty() {
        if let Ok(old) = std::fs::read(path) {
            if old.len() <= MAX_FILE_BYTES && old != new_bytes {
                write_snapshot(&dir, now.saturating_sub(1), true, &old)?;
            }
        }
    } else if let Some(newest) = entries.first() {
        // Unchanged since the newest snapshot: nothing to do.
        if std::fs::read(&newest.file).is_ok_and(|b| b == new_bytes) {
            return Ok(());
        }
        // Rapid re-save (auto-save, Ctrl+S habit): replace the newest one.
        let recent = now.saturating_sub(newest.millis) < COALESCE.as_millis() as u64;
        if recent && !newest.original {
            let _ = std::fs::remove_file(&newest.file);
        }
    }
    write_snapshot(&dir, now, false, new_bytes)?;

    // Prune oldest beyond `max`, keeping the original as long as possible.
    entries = list(path);
    if entries.len() > max {
        let mut excess = entries.len() - max;
        for e in entries.iter().rev().filter(|e| !e.original) {
            if excess == 0 {
                break;
            }
            let _ = std::fs::remove_file(&e.file);
            excess -= 1;
        }
        for e in entries.iter().rev().filter(|e| e.original) {
            if excess == 0 {
                break;
            }
            let _ = std::fs::remove_file(&e.file);
            excess -= 1;
        }
    }
    Ok(())
}

/// Human-readable local time for an entry, e.g. `2026-11-15 14:03`.
pub fn label(entry: &Entry) -> String {
    use chrono::{Local, TimeZone};
    match Local.timestamp_millis_opt(entry.millis as i64) {
        chrono::LocalResult::Single(t) => t.format("%Y-%m-%d %H:%M:%S").to_string(),
        _ => entry.millis.to_string(),
    }
}
