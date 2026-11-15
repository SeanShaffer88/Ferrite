//! Per-line change markers against the last commit (gutter change bars).

use similar::{DiffTag, TextDiff};

/// How a line of the current buffer differs from `HEAD`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineChange {
    Added,
    Modified,
    /// One or more `HEAD` lines were removed just above this line.
    DeletedAbove,
}

/// Skip diffing documents larger than this (bytes); keeps the UI thread cheap.
pub const MAX_DIFF_BYTES: usize = 2 * 1024 * 1024;

/// One entry per line of `current` (`None` = unchanged).
pub fn line_changes(head: &str, current: &str) -> Vec<Option<LineChange>> {
    let line_count = current.split('\n').count();
    let mut out = vec![None; line_count];
    if head.len() > MAX_DIFF_BYTES || current.len() > MAX_DIFF_BYTES {
        return out;
    }
    let diff = TextDiff::from_lines(head, current);
    for op in diff.ops() {
        let new = op.new_range();
        match op.tag() {
            DiffTag::Equal => {}
            DiffTag::Insert => {
                for i in new {
                    if let Some(slot) = out.get_mut(i) {
                        *slot = Some(LineChange::Added);
                    }
                }
            }
            DiffTag::Replace => {
                for i in new {
                    if let Some(slot) = out.get_mut(i) {
                        *slot = Some(LineChange::Modified);
                    }
                }
            }
            DiffTag::Delete => {
                // Mark the line that now sits where the deleted lines were.
                let at = new.start.min(line_count.saturating_sub(1));
                if let Some(slot) = out.get_mut(at) {
                    if slot.is_none() {
                        *slot = Some(LineChange::DeletedAbove);
                    }
                }
            }
        }
    }
    out
}
