//! Syntax checks for fenced JSON / YAML / TOML code blocks in Markdown.
//!
//! A block is checked when its info string starts with `json`, `yaml`,
//! `yml` or `toml`. Problems point at the line
//! inside the document where the parser stopped.

use super::links::{blank_frontmatter, options};
use super::schema::{Problem, ProblemSource, Severity};
use super::value::parse_to_value;
use crate::markdown::tree_viewer::StructuredFileType;
use comrak::nodes::NodeValue;
use comrak::{parse_document, Arena};

fn language(info: &str) -> Option<(StructuredFileType, &'static str)> {
    let word = info.split_whitespace().next()?.to_ascii_lowercase();
    // `json title="x"` / `{.json}` style attributes after the name are ignored.
    let word = word.trim_start_matches('{').trim_start_matches('.');
    match word.split(|c: char| !c.is_ascii_alphanumeric()).next()? {
        // Not jsonc/json5: they allow comments the strict parser rejects.
        "json" => Some((StructuredFileType::Json, "JSON")),
        "yaml" | "yml" => Some((StructuredFileType::Yaml, "YAML")),
        "toml" => Some((StructuredFileType::Toml, "TOML")),
        _ => None,
    }
}

/// Lint every fenced JSON/YAML/TOML block in `content`.
pub fn check_code_blocks(content: &str) -> Vec<Problem> {
    let arena = Arena::new();
    let text = blank_frontmatter(content);
    let root = parse_document(&arena, &text, &options());
    let mut out = Vec::new();
    for node in root.descendants() {
        let data = node.data.borrow();
        let NodeValue::CodeBlock(block) = &data.value else {
            continue;
        };
        if !block.fenced {
            continue;
        }
        let Some((ft, name)) = language(&block.info) else {
            continue;
        };
        if block.literal.trim().is_empty() {
            continue;
        }
        if let Err(e) = parse_to_value(&block.literal, ft) {
            // Body starts on the line after the opening fence.
            let fence = data.sourcepos.start.line.max(1);
            let line = fence + e.line.unwrap_or(1);
            out.push(Problem {
                source: ProblemSource::Lint,
                severity: Severity::Warning,
                line,
                col: data.sourcepos.start.column.max(1),
                message: format!("{name} code block: {}", e.message),
                pointer: String::new(),
            });
        }
    }
    out
}
