//! Inputs for the command line that are not plain files: standard input and
//! `--diff`. Both are turned into ordinary files so the single-instance
//! protocol and tab opening need no special cases.

use super::diff::{diff_values, render_report, render_text_diff};
use super::value::parse_to_value;
use crate::markdown::tree_viewer::StructuredFileType;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Largest stdin payload kept (bytes).
pub const STDIN_CAP: usize = 64 * 1024 * 1024;

/// Read at most `cap` bytes. The flag is true when input was cut off.
pub fn read_capped<R: Read>(reader: R, cap: usize) -> std::io::Result<(Vec<u8>, bool)> {
    let mut buf = Vec::new();
    reader.take(cap as u64 + 1).read_to_end(&mut buf)?;
    let truncated = buf.len() > cap;
    buf.truncate(cap);
    Ok((buf, truncated))
}

/// Write `data` (UTF-8, lossy) to `dir/stdin-<pid>.txt`.
pub fn write_stdin_file(dir: &Path, data: &[u8]) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(format!("stdin-{}.txt", std::process::id()));
    std::fs::write(&path, String::from_utf8_lossy(data).as_bytes())?;
    Ok(path)
}

fn structured(path: &Path) -> Option<StructuredFileType> {
    StructuredFileType::from_extension(path.extension()?.to_str()?)
}

/// Compare two files and write the report to `out_dir`. JSON/YAML/TOML pairs are
/// compared by value; anything else gets a unified line diff.
pub fn build_diff_file(a: &Path, b: &Path, out_dir: &Path) -> Result<PathBuf, String> {
    let read = |p: &Path| {
        std::fs::read_to_string(p).map_err(|e| format!("cannot read {}: {e}", p.display()))
    };
    let (ta, tb) = (read(a)?, read(b)?);
    let name = |p: &Path| {
        p.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
            .to_string()
    };
    let (na, nb) = (name(a), name(b));
    let report = match (structured(a), structured(b)) {
        (Some(fa), Some(fb)) => {
            let va = parse_to_value(&ta, fa).map_err(|e| format!("{}: {e}", a.display()))?;
            let vb = parse_to_value(&tb, fb).map_err(|e| format!("{}: {e}", b.display()))?;
            render_report(&na, &nb, &diff_values(&va, &vb))
        }
        _ => render_text_diff(&na, &nb, &ta, &tb),
    };
    std::fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let out = out_dir.join(format!("diff-{na}-{nb}.diff"));
    std::fs::write(&out, report).map_err(|e| e.to_string())?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn stdin_over_cap_is_truncated_with_flag() {
        let (buf, cut) = read_capped(&b"abcdef"[..], 4).unwrap();
        assert_eq!(buf, b"abcd");
        assert!(cut);
        let (buf, cut) = read_capped(&b"abcd"[..], 4).unwrap();
        assert_eq!(buf, b"abcd");
        assert!(!cut);
    }

    #[test]
    fn stdin_dash_writes_lossy_utf8_file() {
        let dir = TempDir::new().unwrap();
        let p = write_stdin_file(dir.path(), b"hi \xff there").unwrap();
        let text = std::fs::read_to_string(p).unwrap();
        assert!(text.starts_with("hi ") && text.ends_with(" there"));
    }

    #[test]
    fn empty_stdin_writes_empty_file() {
        let dir = TempDir::new().unwrap();
        let p = write_stdin_file(dir.path(), b"").unwrap();
        assert_eq!(std::fs::read_to_string(p).unwrap(), "");
    }

    #[test]
    fn diff_two_json_files_creates_structured_report() {
        let dir = TempDir::new().unwrap();
        let (a, b) = (dir.path().join("a.json"), dir.path().join("b.json"));
        std::fs::write(&a, r#"{"x":1,"y":2}"#).unwrap();
        std::fs::write(&b, r#"{"y":2,"x":5}"#).unwrap();
        let out = build_diff_file(&a, &b, &dir.path().join("out")).unwrap();
        let r = std::fs::read_to_string(out).unwrap();
        assert!(r.contains("~ /x: 1 -> 5") && !r.contains("/y"));
    }

    #[test]
    fn diff_json_vs_yaml_is_structured() {
        let dir = TempDir::new().unwrap();
        let (a, b) = (dir.path().join("a.json"), dir.path().join("b.yaml"));
        std::fs::write(&a, r#"{"x":1}"#).unwrap();
        std::fs::write(&b, "x: 1\n").unwrap();
        let out = build_diff_file(&a, &b, &dir.path().join("out")).unwrap();
        assert!(std::fs::read_to_string(out)
            .unwrap()
            .contains("No differences."));
    }

    #[test]
    fn diff_text_files_is_unified() {
        let dir = TempDir::new().unwrap();
        let (a, b) = (dir.path().join("a.txt"), dir.path().join("b.txt"));
        std::fs::write(&a, "one\ntwo\n").unwrap();
        std::fs::write(&b, "one\nthree\n").unwrap();
        let out = build_diff_file(&a, &b, &dir.path().join("out")).unwrap();
        let r = std::fs::read_to_string(out).unwrap();
        assert!(r.contains("-two") && r.contains("+three"));
    }

    #[test]
    fn diff_missing_file_errors() {
        let dir = TempDir::new().unwrap();
        let e = build_diff_file(
            &dir.path().join("no.json"),
            &dir.path().join("b.json"),
            dir.path(),
        );
        assert!(e.unwrap_err().contains("cannot read"));
    }

    #[test]
    fn diff_invalid_json_errors_with_path() {
        let dir = TempDir::new().unwrap();
        let (a, b) = (dir.path().join("a.json"), dir.path().join("b.json"));
        std::fs::write(&a, "{nope").unwrap();
        std::fs::write(&b, "{}").unwrap();
        assert!(build_diff_file(&a, &b, dir.path())
            .unwrap_err()
            .contains("a.json"));
    }
}
