//! Map a JSON pointer to a (line, column) position in source text.
//!
//! Best effort: JSON is scanned exactly; YAML and TOML use indentation / header
//! heuristics. When nothing matches, the answer is `None` and callers fall back
//! to line 1.

use crate::markdown::tree_viewer::StructuredFileType;

/// Split a JSON pointer (`/a/0/b`) into unescaped segments.
pub fn pointer_segments(pointer: &str) -> Vec<String> {
    pointer
        .split('/')
        .skip(1)
        .map(|s| s.replace("~1", "/").replace("~0", "~"))
        .collect()
}

/// Locate `pointer` in `content`. Returns 1-based `(line, col)`.
pub fn locate(content: &str, ft: StructuredFileType, pointer: &str) -> Option<(usize, usize)> {
    let content = content.trim_start_matches('\u{feff}');
    let segs = pointer_segments(pointer);
    match ft {
        StructuredFileType::Json => {
            let off = if segs.is_empty() {
                first_non_ws(content.as_bytes(), 0)
            } else {
                json_offset(content.as_bytes(), 0, &segs)?
            };
            Some(offset_to_line_col(content, off))
        }
        StructuredFileType::Yaml => yaml_locate(content, &segs),
        StructuredFileType::Toml => toml_locate(content, &segs),
    }
}

fn offset_to_line_col(content: &str, off: usize) -> (usize, usize) {
    let off = off.min(content.len());
    let before = &content[..off];
    let line = before.matches('\n').count() + 1;
    let col = before
        .rsplit('\n')
        .next()
        .map(|l| l.chars().count())
        .unwrap_or(0)
        + 1;
    (line, col)
}

// ── JSON ────────────────────────────────────────────────────────────────────

fn first_non_ws(b: &[u8], mut i: usize) -> usize {
    while i < b.len() && b[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

/// Offset just past the string that starts at `i` (which must be `"`).
fn skip_string(b: &[u8], mut i: usize) -> Option<usize> {
    i += 1;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 2,
            b'"' => return Some(i + 1),
            _ => i += 1,
        }
    }
    None
}

fn skip_value(b: &[u8], i: usize) -> Option<usize> {
    let i = first_non_ws(b, i);
    match *b.get(i)? {
        b'"' => skip_string(b, i),
        b'{' | b'[' => {
            let mut depth = 0usize;
            let mut j = i;
            while j < b.len() {
                match b[j] {
                    b'"' => {
                        j = skip_string(b, j)?;
                        continue;
                    }
                    b'{' | b'[' => depth += 1,
                    b'}' | b']' => {
                        depth -= 1;
                        if depth == 0 {
                            return Some(j + 1);
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
            None
        }
        _ => {
            let mut j = i;
            while j < b.len() && !matches!(b[j], b',' | b'}' | b']') && !b[j].is_ascii_whitespace()
            {
                j += 1;
            }
            Some(j)
        }
    }
}

fn json_offset(b: &[u8], start: usize, segs: &[String]) -> Option<usize> {
    let i = first_non_ws(b, start);
    let Some((seg, rest)) = segs.split_first() else {
        return Some(i);
    };
    match *b.get(i)? {
        b'{' => {
            let mut j = i + 1;
            loop {
                j = first_non_ws(b, j);
                if *b.get(j)? != b'"' {
                    return None;
                }
                let key_end = skip_string(b, j)?;
                let key = serde_json::from_slice::<String>(&b[j..key_end]).ok()?;
                let mut k = first_non_ws(b, key_end);
                if *b.get(k)? != b':' {
                    return None;
                }
                k += 1;
                if &key == seg {
                    return json_offset(b, k, rest);
                }
                j = skip_value(b, k)?;
                j = first_non_ws(b, j);
                if *b.get(j)? == b',' {
                    j += 1;
                } else {
                    return None;
                }
            }
        }
        b'[' => {
            let idx: usize = seg.parse().ok()?;
            let mut j = i + 1;
            for n in 0.. {
                j = first_non_ws(b, j);
                if n == idx {
                    return json_offset(b, j, rest);
                }
                j = skip_value(b, j)?;
                j = first_non_ws(b, j);
                if *b.get(j)? == b',' {
                    j += 1;
                } else {
                    return None;
                }
            }
            None
        }
        _ => None,
    }
}

// ── YAML ────────────────────────────────────────────────────────────────────

struct Ln<'a> {
    indent: usize,
    text: &'a str,
    no: usize,
}

fn yaml_lines(content: &str) -> Vec<Ln<'_>> {
    content
        .lines()
        .enumerate()
        .filter_map(|(i, l)| {
            let t = l.trim_start();
            if t.is_empty() || t.starts_with('#') || t == "---" {
                return None;
            }
            Some(Ln {
                indent: l.len() - t.len(),
                text: t,
                no: i + 1,
            })
        })
        .collect()
}

fn yaml_key_matches(text: &str, key: &str) -> bool {
    let t = text.trim_start_matches(['"', '\'']);
    t.strip_prefix(key)
        .map(|r| r.trim_start_matches(['"', '\'']).starts_with(':'))
        .unwrap_or(false)
}

fn yaml_locate(content: &str, segs: &[String]) -> Option<(usize, usize)> {
    let lines = yaml_lines(content);
    let first = lines.first()?;
    let (no, indent) = yaml_walk(&lines, first.indent, segs)?;
    Some((no, indent + 1))
}

/// Find the position of `segs` in `lines`, a block whose entries sit at `indent`.
fn yaml_walk(lines: &[Ln<'_>], indent: usize, segs: &[String]) -> Option<(usize, usize)> {
    let Some((seg, rest)) = segs.split_first() else {
        let f = lines.first()?;
        return Some((f.no, f.indent));
    };
    let is_seq = lines
        .first()
        .map(|l| l.text.starts_with("- ") || l.text == "-")
        .unwrap_or(false);
    if is_seq {
        let idx: usize = seg.parse().ok()?;
        let starts: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, l)| l.indent == indent && (l.text.starts_with("- ") || l.text == "-"))
            .map(|(i, _)| i)
            .collect();
        let s = *starts.get(idx)?;
        let e = starts.get(idx + 1).copied().unwrap_or(lines.len());
        let item = &lines[s..e];
        let first = &item[0];
        let after = first.text[1..].trim_start();
        if rest.is_empty() {
            return Some((first.no, first.indent));
        }
        let content_indent = first.indent + (first.text.len() - after.len());
        let mut sub: Vec<Ln<'_>> = Vec::with_capacity(item.len());
        if !after.is_empty() {
            sub.push(Ln {
                indent: content_indent,
                text: after,
                no: first.no,
            });
        }
        sub.extend(item[1..].iter().map(|l| Ln {
            indent: l.indent,
            text: l.text,
            no: l.no,
        }));
        let ci = sub.first()?.indent;
        return yaml_walk(&sub, ci, rest);
    }
    let pos = lines
        .iter()
        .position(|l| l.indent == indent && yaml_key_matches(l.text, seg))?;
    let line = &lines[pos];
    if rest.is_empty() {
        return Some((line.no, line.indent));
    }
    let end = lines[pos + 1..]
        .iter()
        .position(|l| l.indent <= indent && !l.text.starts_with("- "))
        .map(|n| pos + 1 + n)
        .unwrap_or(lines.len());
    let child = &lines[pos + 1..end];
    match child.first() {
        Some(c) => yaml_walk(child, c.indent, rest).or(Some((line.no, line.indent))),
        None => Some((line.no, line.indent)),
    }
}

// ── TOML ────────────────────────────────────────────────────────────────────

fn toml_path(s: &str) -> Vec<String> {
    s.split('.')
        .map(|p| p.trim().trim_matches(['"', '\'']).to_string())
        .collect()
}

fn toml_locate(content: &str, segs: &[String]) -> Option<(usize, usize)> {
    let mut table: Vec<String> = Vec::new();
    let mut array_counts: Vec<(Vec<String>, usize)> = Vec::new();
    let mut best: Option<(usize, usize, usize)> = None; // (matched_len, line, col)
    let mut consider = |path: &[String], no: usize, col: usize| {
        let n = path.len().min(segs.len());
        if path[..n] == segs[..n] && best.map(|b| n >= b.0).unwrap_or(true) {
            best = Some((n, no, col));
        }
    };
    for (i, raw) in content.lines().enumerate() {
        let t = raw.trim_start();
        let col = raw.len() - t.len() + 1;
        let no = i + 1;
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if let Some(h) = t.strip_prefix("[[") {
            let name = h.split("]]").next().unwrap_or("");
            let base = toml_path(name);
            let cnt = match array_counts.iter_mut().find(|(p, _)| *p == base) {
                Some((_, c)) => {
                    *c += 1;
                    *c
                }
                None => {
                    array_counts.push((base.clone(), 0));
                    0
                }
            };
            table = base;
            table.push(cnt.to_string());
            consider(&table, no, col);
        } else if let Some(h) = t.strip_prefix('[') {
            table = toml_path(h.split(']').next().unwrap_or(""));
            consider(&table, no, col);
        } else if let Some((k, _)) = t.split_once('=') {
            let mut path = table.clone();
            path.extend(toml_path(k));
            consider(&path, no, col);
        }
    }
    best.map(|(_, l, c)| (l, c))
}

#[cfg(test)]
mod tests {
    use super::*;
    use StructuredFileType::*;

    #[test]
    fn json_nested_key() {
        let src = "{\n  \"a\": {\n    \"b\": 1,\n    \"c\": [10, 20]\n  }\n}";
        assert_eq!(locate(src, Json, "/a/b"), Some((3, 10)));
        // `    "c": [10, 20]`: the `2` of 20 is column 15.
        assert_eq!(locate(src, Json, "/a/c/1"), Some((4, 15)));
    }

    #[test]
    fn json_root_pointer() {
        assert_eq!(locate("\n  {}", Json, ""), Some((2, 3)));
    }

    #[test]
    fn crlf_source_lines_match() {
        let src = "{\r\n  \"a\": 1,\r\n  \"b\": 2\r\n}";
        assert_eq!(locate(src, Json, "/b").map(|p| p.0), Some(3));
    }

    #[test]
    fn utf8_bom_does_not_shift_columns() {
        let src = "\u{feff}{\"a\": 1}";
        assert_eq!(locate(src, Json, "/a"), Some((1, 7)));
    }

    #[test]
    fn json_escaped_pointer_segments() {
        let src = "{\"a/b\": {\"~x\": 5}}";
        assert!(locate(src, Json, "/a~1b/~0x").is_some());
    }

    #[test]
    fn yaml_nested_and_sequence() {
        let src = "name: x\nitems:\n  - id: 1\n  - id: 2\n    tag: t\nother: 3\n";
        assert_eq!(locate(src, Yaml, "/name"), Some((1, 1)));
        assert_eq!(locate(src, Yaml, "/items/1/tag"), Some((5, 5)));
        assert_eq!(locate(src, Yaml, "/other"), Some((6, 1)));
    }

    #[test]
    fn toml_tables_and_arrays_of_tables() {
        let src = "[package]\nname = \"x\"\n\n[[bin]]\nname = \"a\"\n\n[[bin]]\nname = \"b\"\n";
        assert_eq!(locate(src, Toml, "/package/name"), Some((2, 1)));
        assert_eq!(locate(src, Toml, "/bin/1/name"), Some((8, 1)));
    }

    #[test]
    fn missing_pointer_is_none_for_json() {
        assert_eq!(locate("{\"a\":1}", Json, "/zzz"), None);
    }

    #[test]
    fn empty_content_does_not_panic() {
        for ft in [Json, Yaml, Toml] {
            let _ = locate("", ft, "/a");
            let _ = locate("   \n", ft, "");
        }
    }
}
