//! Structured diff: compare two documents by value, ignoring key order and
//! formatting.

use super::query::preview;
use serde_json::Value;

/// What changed at a pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffKind {
    Added,
    Removed,
    Changed,
}

/// One difference between two documents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffEntry {
    pub pointer: String,
    pub kind: DiffKind,
    pub old: Option<String>,
    pub new: Option<String>,
}

fn esc(k: &str) -> String {
    k.replace('~', "~0").replace('/', "~1")
}

/// Differences between `a` (old) and `b` (new), in document order.
pub fn diff_values(a: &Value, b: &Value) -> Vec<DiffEntry> {
    let mut out = Vec::new();
    walk(a, b, String::new(), &mut out);
    out
}

fn walk(a: &Value, b: &Value, ptr: String, out: &mut Vec<DiffEntry>) {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for (k, av) in x {
                let p = format!("{ptr}/{}", esc(k));
                match y.get(k) {
                    Some(bv) => walk(av, bv, p, out),
                    None => out.push(DiffEntry {
                        pointer: p,
                        kind: DiffKind::Removed,
                        old: Some(preview(av)),
                        new: None,
                    }),
                }
            }
            for (k, bv) in y {
                if !x.contains_key(k) {
                    out.push(DiffEntry {
                        pointer: format!("{ptr}/{}", esc(k)),
                        kind: DiffKind::Added,
                        old: None,
                        new: Some(preview(bv)),
                    });
                }
            }
        }
        (Value::Array(x), Value::Array(y)) => {
            for i in 0..x.len().max(y.len()) {
                let p = format!("{ptr}/{i}");
                match (x.get(i), y.get(i)) {
                    (Some(av), Some(bv)) => walk(av, bv, p, out),
                    (Some(av), None) => out.push(DiffEntry {
                        pointer: p,
                        kind: DiffKind::Removed,
                        old: Some(preview(av)),
                        new: None,
                    }),
                    (None, Some(bv)) => out.push(DiffEntry {
                        pointer: p,
                        kind: DiffKind::Added,
                        old: None,
                        new: Some(preview(bv)),
                    }),
                    (None, None) => {}
                }
            }
        }
        _ if a == b => {}
        _ => out.push(DiffEntry {
            pointer: ptr,
            kind: DiffKind::Changed,
            old: Some(preview(a)),
            new: Some(preview(b)),
        }),
    }
}

/// Plain-text report of structured differences.
pub fn render_report(old_name: &str, new_name: &str, entries: &[DiffEntry]) -> String {
    let mut out = format!("# Structured diff: {old_name} -> {new_name}\n\n");
    if entries.is_empty() {
        out.push_str("No differences.\n");
        return out;
    }
    for e in entries {
        let line = match e.kind {
            DiffKind::Added => format!("+ {}: {}", e.pointer, e.new.as_deref().unwrap_or("")),
            DiffKind::Removed => format!("- {}: {}", e.pointer, e.old.as_deref().unwrap_or("")),
            DiffKind::Changed => format!(
                "~ {}: {} -> {}",
                e.pointer,
                e.old.as_deref().unwrap_or(""),
                e.new.as_deref().unwrap_or("")
            ),
        };
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// Unified line diff for files that are not structured data.
pub fn render_text_diff(old_name: &str, new_name: &str, old: &str, new: &str) -> String {
    let diff = similar::TextDiff::from_lines(old, new);
    let text = diff
        .unified_diff()
        .context_radius(3)
        .header(old_name, new_name)
        .to_string();
    if text.is_empty() {
        "No differences.\n".to_string()
    } else {
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn key_order_ignored() {
        assert!(diff_values(&json!({"a":1,"b":2}), &json!({"b":2,"a":1})).is_empty());
    }

    #[test]
    fn added_removed_changed_detected() {
        let d = diff_values(&json!({"a":1,"b":2}), &json!({"a":9,"c":3}));
        let kinds: Vec<_> = d.iter().map(|e| (e.pointer.as_str(), e.kind)).collect();
        assert_eq!(
            kinds,
            [
                ("/a", DiffKind::Changed),
                ("/b", DiffKind::Removed),
                ("/c", DiffKind::Added)
            ]
        );
        assert_eq!(d[0].old.as_deref(), Some("1"));
        assert_eq!(d[0].new.as_deref(), Some("9"));
    }

    #[test]
    fn array_index_change_reported() {
        let d = diff_values(&json!({"x":[1,2,3]}), &json!({"x":[1,5]}));
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].pointer, "/x/1");
        assert_eq!(d[1].pointer, "/x/2");
        assert_eq!(d[1].kind, DiffKind::Removed);
    }

    #[test]
    fn identical_values_empty_diff() {
        let v = json!({"a":[1,{"b":null}]});
        assert!(diff_values(&v, &v).is_empty());
    }

    #[test]
    fn pointer_escapes_slash_and_tilde() {
        let d = diff_values(&json!({"a/b":1}), &json!({"a/b":2}));
        assert_eq!(d[0].pointer, "/a~1b");
    }

    #[test]
    fn report_lists_each_kind() {
        let d = diff_values(&json!({"a":1,"b":2}), &json!({"a":9,"c":3}));
        let r = render_report("old", "new", &d);
        assert!(r.contains("~ /a: 1 -> 9") && r.contains("- /b: 2") && r.contains("+ /c: 3"));
        assert!(render_report("o", "n", &[]).contains("No differences."));
    }

    #[test]
    fn text_diff_unified_and_empty() {
        let d = render_text_diff("a", "b", "x\ny\n", "x\nz\n");
        assert!(d.contains("-y") && d.contains("+z"));
        assert_eq!(
            render_text_diff("a", "b", "same\n", "same\n"),
            "No differences.\n"
        );
    }

    #[test]
    fn type_change_is_changed() {
        let d = diff_values(&json!({"a":1}), &json!({"a":"1"}));
        assert_eq!(d[0].kind, DiffKind::Changed);
    }
}
