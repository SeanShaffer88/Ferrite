//! Where the caret is inside a JSON / YAML / TOML document, as a path like
//! `$.jobs.build.steps[2].run`. Best effort, text-based (no full parse), so it
//! works on documents that don't currently parse.

use crate::markdown::tree_viewer::StructuredFileType;

/// One step of a path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Seg {
    Key(String),
    Index(usize),
}

/// Render segments as JSONPath (`$.a.b[0]`, `$["key with space"]`).
pub fn to_jsonpath(segs: &[Seg]) -> String {
    let mut out = String::from("$");
    for s in segs {
        match s {
            Seg::Index(i) => out.push_str(&format!("[{i}]")),
            Seg::Key(k) => {
                let simple = !k.is_empty()
                    && k.chars()
                        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
                    && !k.starts_with(|c: char| c.is_ascii_digit());
                if simple {
                    out.push('.');
                    out.push_str(k);
                } else {
                    out.push_str(&format!("[\"{}\"]", k.replace('"', "\\\"")));
                }
            }
        }
    }
    out
}

/// Path at byte offset `pos` (clamped). Empty for the document root.
pub fn path_at(content: &str, ft: StructuredFileType, pos: usize) -> Vec<Seg> {
    let mut pos = pos.min(content.len());
    while !content.is_char_boundary(pos) {
        pos -= 1;
    }
    let before = &content[..pos];
    match ft {
        StructuredFileType::Json => json_path(before),
        StructuredFileType::Yaml => yaml_path(content, before),
        StructuredFileType::Toml => toml_path(content, before),
    }
}

// ── JSON: token scan up to the caret ────────────────────────────────────────

enum Frame {
    Obj {
        key: Option<String>,
        expect_key: bool,
    },
    Arr {
        index: usize,
    },
}

fn json_path(before: &str) -> Vec<Seg> {
    let mut stack: Vec<Frame> = Vec::new();
    let mut chars = before.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                let mut s = String::new();
                while let Some(d) = chars.next() {
                    match d {
                        '\\' => {
                            if let Some(e) = chars.next() {
                                s.push(e);
                            }
                        }
                        '"' => break,
                        _ => s.push(d),
                    }
                }
                if let Some(Frame::Obj { key, expect_key }) = stack.last_mut() {
                    if *expect_key {
                        *key = Some(s);
                        *expect_key = false;
                    }
                }
            }
            '{' => stack.push(Frame::Obj {
                key: None,
                expect_key: true,
            }),
            '[' => stack.push(Frame::Arr { index: 0 }),
            '}' | ']' => {
                stack.pop();
            }
            ',' => match stack.last_mut() {
                Some(Frame::Obj { key, expect_key }) => {
                    *key = None;
                    *expect_key = true;
                }
                Some(Frame::Arr { index }) => *index += 1,
                None => {}
            },
            _ => {}
        }
    }
    stack
        .into_iter()
        .filter_map(|f| match f {
            Frame::Obj { key, .. } => key.map(Seg::Key),
            Frame::Arr { index } => Some(Seg::Index(index)),
        })
        .collect()
}

// ── YAML: indentation stack over lines up to the caret ──────────────────────

fn unquote(s: &str) -> String {
    let t = s.trim();
    t.strip_prefix('"')
        .and_then(|x| x.strip_suffix('"'))
        .or_else(|| t.strip_prefix('\'').and_then(|x| x.strip_suffix('\'')))
        .unwrap_or(t)
        .to_string()
}

/// `key:` at the start of `s` (outside quotes), returning the key.
fn yaml_key(s: &str) -> Option<String> {
    if s.starts_with('#') || s.is_empty() {
        return None;
    }
    let mut in_q: Option<char> = None;
    for (i, c) in s.char_indices() {
        match (in_q, c) {
            (None, '"' | '\'') if i == 0 => in_q = Some(c),
            (Some(q), c) if c == q => in_q = None,
            (None, ':') => {
                let rest = &s[i + 1..];
                if rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\t') {
                    return Some(unquote(&s[..i]));
                }
            }
            _ => {}
        }
    }
    None
}

fn yaml_path(content: &str, before: &str) -> Vec<Seg> {
    // Include the whole caret line so a key on it counts.
    let line_end = content[before.len()..]
        .find('\n')
        .map(|n| before.len() + n)
        .unwrap_or(content.len());
    let text = &content[..line_end];
    // (indent, segment) stack; list items use the dash column as indent.
    let mut stack: Vec<(usize, Seg)> = Vec::new();
    // Per-indent counters for sequence items.
    let mut seq_counts: Vec<(usize, usize)> = Vec::new();
    for raw in text.lines() {
        let line = raw.trim_end_matches('\r');
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed == "---" {
            continue;
        }
        let mut indent = line.len() - trimmed.len();
        let mut rest = trimmed;
        // A sequence item: `- ...` (possibly `- key: v`).
        if let Some(after) = rest
            .strip_prefix("- ")
            .or(if rest == "-" { Some("") } else { None })
        {
            stack.retain(|(i, _)| *i < indent);
            seq_counts.retain(|(i, _)| *i <= indent);
            let n = match seq_counts.iter_mut().find(|(i, _)| *i == indent) {
                Some((_, n)) => {
                    *n += 1;
                    *n
                }
                None => {
                    seq_counts.push((indent, 0));
                    0
                }
            };
            stack.push((indent, Seg::Index(n)));
            indent += 2;
            rest = after.trim_start();
            if rest.is_empty() {
                continue;
            }
        } else {
            seq_counts.retain(|(i, _)| *i < indent);
        }
        if let Some(key) = yaml_key(rest) {
            stack.retain(|(i, _)| *i < indent);
            stack.push((indent, Seg::Key(key)));
        }
    }
    stack.into_iter().map(|(_, s)| s).collect()
}

// ── TOML: last table header + key on the caret line ─────────────────────────

fn toml_keys(s: &str) -> Vec<Seg> {
    s.split('.')
        .map(|k| Seg::Key(unquote(k)))
        .filter(|s| !matches!(s, Seg::Key(k) if k.is_empty()))
        .collect()
}

fn toml_path(content: &str, before: &str) -> Vec<Seg> {
    let line_start = before.rfind('\n').map(|n| n + 1).unwrap_or(0);
    let mut table: Vec<Seg> = Vec::new();
    let mut array_counts: std::collections::HashMap<String, usize> = Default::default();
    for raw in content[..line_start].lines() {
        let t = raw.trim();
        if let Some(h) = t.strip_prefix("[[").and_then(|x| x.split("]]").next()) {
            let n = array_counts
                .entry(h.trim().to_string())
                .or_insert(usize::MAX);
            *n = n.wrapping_add(1);
            table = toml_keys(h);
            table.push(Seg::Index(*n));
        } else if let Some(h) = t.strip_prefix('[').and_then(|x| x.split(']').next()) {
            table = toml_keys(h);
        }
    }
    let line_end = content[line_start..]
        .find('\n')
        .map(|n| line_start + n)
        .unwrap_or(content.len());
    let line = content[line_start..line_end].trim();
    if line.starts_with('[') {
        let h = line.trim_start_matches('[').split(']').next().unwrap_or("");
        return toml_keys(h);
    }
    if let Some((k, _)) = line.split_once('=') {
        if !line.starts_with('#') {
            table.extend(toml_keys(k.trim()));
        }
    }
    table
}
