//! Command-line open targets (`path:line:col`), their wire encoding for the
//! single-instance protocol, and git-editor file detection.

use std::path::{Path, PathBuf};

/// A file to open, optionally at a line and column (both 1-based).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenTarget {
    pub path: PathBuf,
    pub line: Option<u32>,
    pub col: Option<u32>,
}

impl OpenTarget {
    /// Target with no position.
    pub fn plain(path: PathBuf) -> Self {
        Self { path, line: None, col: None }
    }
}

fn positive(s: &str) -> Option<u32> {
    s.parse::<u32>().ok().filter(|n| *n >= 1)
}

/// Parse `path`, `path:line` or `path:line:col`.
///
/// A trailing `:number` is only a position when it is a positive integer and a
/// non-empty path remains. A Windows drive colon is never split, and if the
/// whole string names an existing file it is used as-is.
pub fn parse_open_target(arg: &str) -> OpenTarget {
    if Path::new(arg).exists() {
        return OpenTarget::plain(PathBuf::from(arg));
    }
    let mut rest = arg;
    let mut nums: Vec<u32> = Vec::new();
    while nums.len() < 2 {
        let Some(idx) = rest.rfind(':') else { break };
        let head = &rest[..idx];
        let drive_only = head.len() == 1 && head.chars().all(|c| c.is_ascii_alphabetic());
        match positive(&rest[idx + 1..]) {
            Some(n) if !head.is_empty() && !drive_only => {
                nums.push(n);
                rest = head;
            }
            _ => break,
        }
    }
    let (line, col) = match nums.as_slice() {
        [] => (None, None),
        [l] => (Some(*l), None),
        [c, l] => (Some(*l), Some(*c)),
        _ => (None, None),
    };
    OpenTarget { path: PathBuf::from(rest), line, col }
}

/// Encode for the single-instance protocol. Targets without a position stay a
/// bare path line, so older receivers keep working.
pub fn encode_target(t: &OpenTarget) -> String {
    let path = t.path.to_string_lossy();
    if t.line.is_none() && t.col.is_none() {
        return path.into_owned();
    }
    let n = |v: Option<u32>| v.map(|x| x.to_string()).unwrap_or_default();
    format!("T\t{}\t{}\t{}", n(t.line), n(t.col), path)
}

/// Inverse of [`encode_target`]; any other line is a plain path.
pub fn decode_target(line: &str) -> OpenTarget {
    if let Some(body) = line.strip_prefix("T\t") {
        let mut it = body.splitn(3, '\t');
        if let (Some(l), Some(c), Some(p)) = (it.next(), it.next(), it.next()) {
            return OpenTarget { path: PathBuf::from(p), line: positive(l), col: positive(c) };
        }
    }
    OpenTarget::plain(PathBuf::from(line))
}

/// Files git hands to `$GIT_EDITOR`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitEditorKind {
    Commit,
    Merge,
    Tag,
    RebaseTodo,
    PatchEdit,
}

/// Detect a git editor file: a known file name living under a `.git` directory.
pub fn detect_git_editor_file(path: &Path) -> Option<GitEditorKind> {
    let kind = match path.file_name()?.to_str()? {
        "COMMIT_EDITMSG" => GitEditorKind::Commit,
        "MERGE_MSG" => GitEditorKind::Merge,
        "TAG_EDITMSG" => GitEditorKind::Tag,
        "git-rebase-todo" => GitEditorKind::RebaseTodo,
        "addp-hunk-edit.diff" => GitEditorKind::PatchEdit,
        _ => return None,
    };
    path.ancestors()
        .skip(1)
        .any(|a| a.file_name().map(|n| n == ".git").unwrap_or(false))
        .then_some(kind)
}

/// Whether to block until the tab closes: asked for, or a git editor file.
pub fn should_wait(explicit: bool, path: &Path) -> bool {
    explicit || detect_git_editor_file(path).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(arg: &str) -> (String, Option<u32>, Option<u32>) {
        let o = parse_open_target(arg);
        (o.path.to_string_lossy().into_owned(), o.line, o.col)
    }

    #[test]
    fn plain_path_unchanged() {
        assert_eq!(t("src/main.rs"), ("src/main.rs".into(), None, None));
    }

    #[test]
    fn file_colon_line() {
        assert_eq!(t("src/main.rs:42"), ("src/main.rs".into(), Some(42), None));
    }

    #[test]
    fn file_colon_line_colon_col() {
        assert_eq!(t("a.rs:12:3"), ("a.rs".into(), Some(12), Some(3)));
    }

    #[test]
    fn windows_drive_path_kept() {
        assert_eq!(t(r"C:\a\x.rs"), (r"C:\a\x.rs".into(), None, None));
    }

    #[test]
    fn windows_drive_with_line() {
        assert_eq!(t(r"C:\a b\x.rs:12:3"), (r"C:\a b\x.rs".into(), Some(12), Some(3)));
    }

    #[test]
    fn non_numeric_suffix_part_of_name() {
        assert_eq!(t("x.rs:abc"), ("x.rs:abc".into(), None, None));
    }

    #[test]
    fn line_zero_ignored() {
        assert_eq!(t("x.rs:0"), ("x.rs:0".into(), None, None));
    }

    #[test]
    fn only_numbers_after_colon_keeps_nonempty_path() {
        assert_eq!(t(":5"), (":5".into(), None, None));
        assert_eq!(t("C:12"), ("C:12".into(), None, None));
    }

    #[test]
    fn existing_file_named_with_colon_wins() {
        let dir = tempfile::TempDir::new().unwrap();
        let p = dir.path().join("x.rs:12");
        std::fs::write(&p, "").unwrap();
        let o = parse_open_target(p.to_str().unwrap());
        assert_eq!(o.path, p);
        assert_eq!(o.line, None);
    }

    #[test]
    fn encode_decode_target_round_trip() {
        let o = OpenTarget { path: PathBuf::from("a/b.rs"), line: Some(7), col: Some(2) };
        assert_eq!(decode_target(&encode_target(&o)), o);
        let l = OpenTarget { path: PathBuf::from("a.rs"), line: Some(9), col: None };
        assert_eq!(decode_target(&encode_target(&l)), l);
    }

    #[test]
    fn legacy_plain_path_line_decodes() {
        let o = decode_target(r"C:\Users\me\f.md");
        assert_eq!(o, OpenTarget::plain(PathBuf::from(r"C:\Users\me\f.md")));
        assert_eq!(encode_target(&o), r"C:\Users\me\f.md");
    }

    #[test]
    fn tab_in_path_survives() {
        let o = OpenTarget { path: PathBuf::from("a\tb.rs"), line: Some(1), col: None };
        assert_eq!(decode_target(&encode_target(&o)), o);
    }

    #[test]
    fn commit_editmsg_detected() {
        let p = Path::new("/repo/.git/COMMIT_EDITMSG");
        assert_eq!(detect_git_editor_file(p), Some(GitEditorKind::Commit));
    }

    #[test]
    fn same_name_outside_dot_git_not_detected() {
        assert_eq!(detect_git_editor_file(Path::new("/repo/docs/COMMIT_EDITMSG")), None);
    }

    #[test]
    fn rebase_todo_detected() {
        let p = Path::new("/r/.git/rebase-merge/git-rebase-todo");
        assert_eq!(detect_git_editor_file(p), Some(GitEditorKind::RebaseTodo));
    }

    #[test]
    fn git_editor_file_implies_wait() {
        assert!(should_wait(false, Path::new("/r/.git/MERGE_MSG")));
        assert!(!should_wait(false, Path::new("/r/a.txt")));
        assert!(should_wait(true, Path::new("/r/a.txt")));
    }
}
