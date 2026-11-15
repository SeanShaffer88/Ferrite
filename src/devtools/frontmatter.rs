//! Markdown document checks: YAML frontmatter against a schema, and
//! relative links.

use super::links::{check_links, AnchorCache};
use super::schema::{resolve_schema, validate, Problem, SchemaRef};
use crate::markdown::tree_viewer::StructuredFileType;
use std::path::Path;

/// Validate the frontmatter of a Markdown document.
///
/// The schema comes from a `# yaml-language-server: $schema=…` or
/// `#:schema …` comment inside the frontmatter, else a
/// `devtools.schema_mappings` glob matching the `.md` path. Without a schema
/// only YAML syntax errors are reported. Lines are document lines.
pub fn check_frontmatter(
    doc_path: &Path,
    content: &str,
    mappings: &[(String, String)],
) -> Vec<Problem> {
    let Some((yaml, _)) = crate::ui::extract_frontmatter(content) else {
        return Vec::new();
    };
    let schema = resolve_schema(doc_path, &yaml, StructuredFileType::Yaml, mappings)
        .unwrap_or(SchemaRef::Inline(serde_json::Value::Bool(true)));
    // The YAML body starts on line 2 (after the opening `---`).
    validate(&yaml, StructuredFileType::Yaml, &schema)
        .into_iter()
        .map(|mut p| {
            p.line += 1;
            p
        })
        .collect()
}

/// Which Markdown checks to run.
#[derive(Debug, Clone, Copy, Default)]
pub struct MarkdownChecks {
    pub frontmatter: bool,
    pub links: bool,
    pub code_blocks: bool,
}

/// All checks for a Markdown tab, sorted by position.
pub fn check_markdown(
    doc_path: Option<&Path>,
    content: &str,
    mappings: &[(String, String)],
    checks: MarkdownChecks,
) -> Vec<Problem> {
    let mut out = Vec::new();
    if checks.code_blocks {
        out.extend(super::codeblocks::check_code_blocks(content));
    }
    // Untitled documents have no base directory for schemas or relative links.
    if let Some(path) = doc_path {
        if checks.frontmatter {
            out.extend(check_frontmatter(path, content, mappings));
        }
        if checks.links {
            out.extend(check_links(path, content, &mut AnchorCache::default()));
        }
    }
    out.sort_by_key(|p| (p.line, p.col));
    out
}
