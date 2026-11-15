//! Relative link and anchor checking for Markdown documents.
//!
//! Only links that point inside the file system are checked: a target with a
//! URL scheme, `mailto:`, `//host` or a root-absolute `/path` is skipped. No
//! network access.

use super::schema::{Problem, ProblemSource, Severity};
use comrak::nodes::{AstNode, NodeValue};
use comrak::{parse_document, Anchorizer, Arena, Options};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Markdown file extensions whose anchors can be checked.
const MD_EXTS: &[&str] = &["md", "markdown", "mdown", "mkd", "mdx"];

fn options() -> Options {
    let mut o = Options::default();
    o.extension.table = true;
    o.extension.strikethrough = true;
    o.extension.autolink = true;
    o.extension.footnotes = true;
    o
}

/// Replace YAML frontmatter with blank lines so it is not parsed as Markdown
/// and every later line keeps its number.
fn blank_frontmatter(content: &str) -> std::borrow::Cow<'_, str> {
    match crate::ui::extract_frontmatter(content) {
        Some((_, end)) if end <= content.len() && content.is_char_boundary(end) => {
            let lines = content[..end].matches('\n').count();
            std::borrow::Cow::Owned(format!("{}{}", "\n".repeat(lines), &content[end..]))
        }
        _ => std::borrow::Cow::Borrowed(content),
    }
}

/// A link found in a document.
#[derive(Debug, Clone)]
pub struct FoundLink {
    pub url: String,
    pub line: usize,
    pub col: usize,
}

fn collect_text<'a>(node: &'a AstNode<'a>, out: &mut String) {
    for d in node.descendants() {
        match &d.data.borrow().value {
            NodeValue::Text(t) => out.push_str(t),
            NodeValue::Code(c) => out.push_str(&c.literal),
            _ => {}
        }
    }
}

/// Links (and images) plus the set of anchors this document defines.
pub fn scan(content: &str) -> (Vec<FoundLink>, HashSet<String>) {
    let arena = Arena::new();
    let text = blank_frontmatter(content);
    let root = parse_document(&arena, &text, &options());
    let mut links = Vec::new();
    let mut anchors = HashSet::new();
    let mut anchorizer = Anchorizer::new();
    let html_anchor = regex::Regex::new(r#"(?i)\b(?:id|name)\s*=\s*["']([^"']+)["']"#).ok();

    for node in root.descendants() {
        let data = node.data.borrow();
        match &data.value {
            NodeValue::Link(l) | NodeValue::Image(l) => links.push(FoundLink {
                url: l.url.clone(),
                line: data.sourcepos.start.line.max(1),
                col: data.sourcepos.start.column.max(1),
            }),
            NodeValue::Heading(_) => {
                let mut text = String::new();
                collect_text(node, &mut text);
                anchors.insert(anchorizer.anchorize(text));
            }
            NodeValue::HtmlInline(html) => {
                if let Some(re) = &html_anchor {
                    anchors.extend(re.captures_iter(html).map(|c| c[1].to_string()));
                }
            }
            NodeValue::HtmlBlock(b) => {
                if let Some(re) = &html_anchor {
                    anchors.extend(re.captures_iter(&b.literal).map(|c| c[1].to_string()));
                }
            }
            NodeValue::FootnoteDefinition(f) => {
                anchors.insert(format!("fn-{}", f.name));
            }
            _ => {}
        }
    }
    (links, anchors)
}

/// Decode `%XX` escapes; invalid sequences are kept as-is.
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            let hex = std::str::from_utf8(&b[i + 1..i + 3]).ok();
            if let Some(v) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// True for targets this checker does not look at.
fn is_external(url: &str) -> bool {
    let u = url.trim();
    if u.is_empty() || u.starts_with("//") || u.starts_with('/') {
        return true;
    }
    // A scheme is letters/digits/+-. followed by ':' before any '/', '?' or '#'.
    match u.find(':') {
        Some(i) => {
            let scheme = &u[..i];
            // `C:\x` and `C:/x` are Windows paths, not schemes.
            let drive = scheme.len() == 1 && scheme.chars().all(|c| c.is_ascii_alphabetic());
            !drive
                && !scheme.is_empty()
                && scheme
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
        }
        None => false,
    }
}

fn is_markdown(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|e| MD_EXTS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

/// Anchors of other files, read lazily and cached for one check run.
#[derive(Default)]
pub struct AnchorCache(HashMap<PathBuf, Option<HashSet<String>>>);

impl AnchorCache {
    fn anchors(&mut self, path: &Path) -> Option<&HashSet<String>> {
        self.0
            .entry(path.to_path_buf())
            .or_insert_with(|| std::fs::read_to_string(path).ok().map(|c| scan(&c).1))
            .as_ref()
    }
}

fn warn(line: usize, col: usize, message: String) -> Problem {
    Problem {
        source: ProblemSource::Lint,
        severity: Severity::Warning,
        line,
        col,
        message,
        pointer: String::new(),
    }
}

/// Check every relative link in `content`, a document stored at `doc_path`.
pub fn check_links(doc_path: &Path, content: &str, cache: &mut AnchorCache) -> Vec<Problem> {
    let (links, own_anchors) = scan(content);
    let dir = doc_path.parent().unwrap_or(Path::new("."));
    let mut out = Vec::new();

    for link in links {
        if is_external(&link.url) {
            continue;
        }
        let (path_part, anchor) = match link.url.split_once('#') {
            Some((p, a)) => (p, Some(percent_decode(a))),
            None => (link.url.as_str(), None),
        };
        // Drop a query string (`file.md?plain=1`).
        let path_part = path_part.split('?').next().unwrap_or("");

        if path_part.is_empty() {
            if let Some(a) = anchor.filter(|a| !a.is_empty()) {
                if !own_anchors.contains(&a) {
                    out.push(warn(
                        link.line,
                        link.col,
                        format!("no heading or anchor `#{a}` in this document"),
                    ));
                }
            }
            continue;
        }

        let target = dir.join(percent_decode(path_part));
        if !target.exists() {
            out.push(warn(
                link.line,
                link.col,
                format!("link target not found: {path_part}"),
            ));
            continue;
        }
        if let Some(a) = anchor.filter(|a| !a.is_empty()) {
            if target.is_file() && is_markdown(&target) {
                let found = cache.anchors(&target).map(|set| set.contains(&a));
                if found == Some(false) {
                    out.push(warn(
                        link.line,
                        link.col,
                        format!("no heading or anchor `#{a}` in {path_part}"),
                    ));
                }
            }
        }
    }
    out
}

/// Markdown files under `root`, skipping hidden, `target` and `node_modules`
/// directories. Stops after `cap` files; the flag is true when it did.
pub fn workspace_markdown_files(root: &Path, cap: usize) -> (Vec<PathBuf>, bool) {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let name = e.file_name().to_string_lossy().into_owned();
            let path = e.path();
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_dir() {
                if !name.starts_with('.') && name != "target" && name != "node_modules" {
                    stack.push(path);
                }
            } else if ft.is_file() && is_markdown(&path) {
                if files.len() >= cap {
                    return (files, true);
                }
                files.push(path);
            }
        }
    }
    (files, false)
}

/// Check all Markdown files under `root` and render a Markdown report.
pub fn workspace_report(root: &Path, cap: usize) -> String {
    let (files, capped) = workspace_markdown_files(root, cap);
    let mut cache = AnchorCache::default();
    let mut broken = 0usize;
    let mut body = String::new();
    for file in &files {
        let Ok(content) = std::fs::read_to_string(file) else {
            continue;
        };
        let problems = check_links(file, &content, &mut cache);
        if problems.is_empty() {
            continue;
        }
        broken += problems.len();
        let rel = file.strip_prefix(root).unwrap_or(file);
        let rel = rel.to_string_lossy().replace('\\', "/");
        body.push_str(&format!("\n## {rel}\n\n"));
        for p in problems {
            body.push_str(&format!("- L{}:{} — {}\n", p.line, p.col, p.message));
        }
    }
    let mut out = format!(
        "# Link check\n\n{} Markdown files checked, {broken} broken relative link(s).\n",
        files.len()
    );
    if capped {
        out.push_str(&format!("\nStopped after {cap} files.\n"));
    }
    if broken == 0 {
        out.push_str("\nNo broken relative links found.\n");
    }
    out.push_str(&body);
    out
}
