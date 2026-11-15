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

/// All checks for a Markdown tab, sorted by position.
pub fn check_markdown(
    doc_path: Option<&Path>,
    content: &str,
    mappings: &[(String, String)],
    frontmatter: bool,
    links: bool,
) -> Vec<Problem> {
    let Some(path) = doc_path else {
        // Untitled: relative links have no base directory.
        return Vec::new();
    };
    let mut out = if frontmatter {
        check_frontmatter(path, content, mappings)
    } else {
        Vec::new()
    };
    if links {
        out.extend(check_links(path, content, &mut AnchorCache::default()));
    }
    out.sort_by_key(|p| (p.line, p.col));
    out
}
