//! Resolve and apply JSON Schemas to JSON / YAML / TOML documents.
//!
//! Offline by design: a `Remote` schema is never fetched; the user gets one
//! info-level problem instead.

use super::locate::locate;
use super::schemas::bundled_for;
use super::value::parse_to_value;
use crate::markdown::tree_viewer::StructuredFileType;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Who produced a problem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProblemSource {
    Schema,
    Lint,
    /// Reserved for the (feature-gated) LSP client.
    #[allow(dead_code)]
    Lsp,
}

/// How serious a problem is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// One diagnostic shown in the Problems panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub source: ProblemSource,
    pub severity: Severity,
    /// 1-based line.
    pub line: usize,
    /// 1-based column.
    pub col: usize,
    pub message: String,
    /// JSON pointer into the document (empty for the root).
    pub pointer: String,
}

/// Where a schema comes from.
#[derive(Debug, Clone, PartialEq)]
pub enum SchemaRef {
    Inline(Value),
    Local(PathBuf),
    Remote(String),
    Bundled(&'static str),
}

fn is_remote(s: &str) -> bool {
    let l = s.to_ascii_lowercase();
    l.starts_with("http://") || l.starts_with("https://")
}

/// Meta-schema URLs describe the file itself, not a schema for its data.
fn is_meta_schema(s: &str) -> bool {
    s.contains("json-schema.org")
}

fn to_ref(raw: &str, doc_path: &Path) -> Option<SchemaRef> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    if is_remote(raw) {
        return if is_meta_schema(raw) {
            None
        } else {
            Some(SchemaRef::Remote(raw.to_string()))
        };
    }
    let raw = raw.strip_prefix("file://").unwrap_or(raw);
    let p = Path::new(raw);
    Some(SchemaRef::Local(if p.is_absolute() {
        p.to_path_buf()
    } else {
        doc_path.parent().unwrap_or(Path::new(".")).join(p)
    }))
}

/// Convert a simple glob (`*`, `**`, `?`) to an end-anchored regex.
fn glob_matches(pattern: &str, path: &str) -> bool {
    let mut re = String::from("(^|/)");
    let mut chars = pattern
        .replace('\\', "/")
        .chars()
        .collect::<Vec<_>>()
        .into_iter()
        .peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' if chars.peek() == Some(&'*') => {
                chars.next();
                re.push_str(".*");
            }
            '*' => re.push_str("[^/]*"),
            '?' => re.push_str("[^/]"),
            c => re.push_str(&regex::escape(&c.to_string())),
        }
    }
    re.push('$');
    regex::Regex::new(&re)
        .map(|r| r.is_match(&path.replace('\\', "/")))
        .unwrap_or(false)
}

/// Pick the schema for a document.
///
/// Order: in-file reference (`$schema`, `# yaml-language-server: $schema=`,
/// `#:schema`) → workspace mapping → bundled schema for the file name.
pub fn resolve_schema(
    path: &Path,
    content: &str,
    ft: StructuredFileType,
    mappings: &[(String, String)],
) -> Option<SchemaRef> {
    if let Some(r) = in_file_ref(path, content, ft) {
        return Some(r);
    }
    let p = path.to_string_lossy();
    for (glob, target) in mappings {
        if glob_matches(glob, &p) {
            if let Some(r) = to_ref(target, path) {
                return Some(r);
            }
        }
    }
    bundled_for(path).map(SchemaRef::Bundled)
}

fn in_file_ref(path: &Path, content: &str, ft: StructuredFileType) -> Option<SchemaRef> {
    match ft {
        StructuredFileType::Json => {
            let v: Value = serde_json::from_str(content.trim_start_matches('\u{feff}')).ok()?;
            to_ref(v.get("$schema")?.as_str()?, path)
        }
        StructuredFileType::Yaml | StructuredFileType::Toml => {
            for line in content.lines().take(10) {
                let t = line.trim();
                let found = if let Some(r) = t.strip_prefix("#:schema") {
                    Some(r)
                } else if let Some(r) = t.strip_prefix('#') {
                    r.trim()
                        .strip_prefix("yaml-language-server:")
                        .and_then(|r| r.trim().strip_prefix("$schema="))
                } else {
                    None
                };
                if let Some(r) = found {
                    return to_ref(r, path);
                }
            }
            None
        }
    }
}

fn info(message: String) -> Problem {
    Problem {
        source: ProblemSource::Schema,
        severity: Severity::Info,
        line: 1,
        col: 1,
        message,
        pointer: String::new(),
    }
}

/// Validate `content` against `schema`. Never touches the network.
pub fn validate(content: &str, ft: StructuredFileType, schema: &SchemaRef) -> Vec<Problem> {
    if content.trim().is_empty() {
        return Vec::new();
    }
    let instance = match parse_to_value(content, ft) {
        Ok(v) => v,
        Err(e) => {
            return vec![Problem {
                source: ProblemSource::Lint,
                severity: Severity::Error,
                line: e.line.unwrap_or(1),
                col: 1,
                message: e.message,
                pointer: String::new(),
            }]
        }
    };
    let schema_value = match schema {
        SchemaRef::Inline(v) => v.clone(),
        SchemaRef::Remote(url) => return vec![info(format!("remote schema not fetched: {url}"))],
        SchemaRef::Local(p) => match std::fs::read_to_string(p)
            .map_err(|e| e.to_string())
            .and_then(|s| serde_json::from_str::<Value>(&s).map_err(|e| e.to_string()))
        {
            Ok(v) => v,
            Err(e) => return vec![info(format!("schema file {} unusable: {e}", p.display()))],
        },
        SchemaRef::Bundled(text) => match serde_json::from_str(text) {
            Ok(v) => v,
            Err(e) => return vec![info(format!("bundled schema invalid: {e}"))],
        },
    };
    let validator = match jsonschema::validator_for(&schema_value) {
        Ok(v) => v,
        Err(e) => return vec![info(format!("schema could not be compiled: {e}"))],
    };
    let mut out: Vec<Problem> = validator
        .iter_errors(&instance)
        .map(|err| {
            let pointer = err.instance_path().to_string();
            let (line, col) = locate(content, ft, &pointer).unwrap_or((1, 1));
            Problem {
                source: ProblemSource::Schema,
                severity: Severity::Error,
                line,
                col,
                message: err.to_string(),
                pointer,
            }
        })
        .collect();
    out.sort_by_key(|p| (p.line, p.col));
    out
}

/// Full check for one document: resolve a schema, then validate. Without a
/// schema, only syntax errors are reported.
pub fn check_document(
    path: Option<&Path>,
    content: &str,
    ft: StructuredFileType,
    mappings: &[(String, String)],
) -> Vec<Problem> {
    let schema = path.and_then(|p| resolve_schema(p, content, ft, mappings));
    match schema {
        Some(s) => validate(content, ft, &s),
        None => validate(content, ft, &SchemaRef::Inline(Value::Bool(true))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use StructuredFileType::*;

    fn pkg_schema() -> SchemaRef {
        SchemaRef::Inline(json!({
            "type": "object",
            "required": ["name", "version"],
            "properties": {"name": {"type": "string"}, "count": {"type": "number"}}
        }))
    }

    #[test]
    fn required_property_missing_reports_line_of_parent() {
        let p = validate("{\n  \"name\": \"x\"\n}", Json, &pkg_schema());
        assert_eq!(p.len(), 1);
        assert!(p[0].message.contains("version"));
        assert_eq!(p[0].line, 1);
    }

    #[test]
    fn wrong_type_points_at_value_line() {
        let src = "{\n  \"name\": \"x\",\n  \"version\": \"1\",\n  \"count\": \"many\"\n}";
        let p = validate(src, Json, &pkg_schema());
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].line, 4);
        assert_eq!(p[0].pointer, "/count");
    }

    #[test]
    fn yaml_and_toml_validate_too() {
        let y = validate("name: x\ncount: no\n", Yaml, &pkg_schema());
        assert!(y.iter().any(|p| p.pointer == "/count" && p.line == 2));
        let t = validate(
            "name = \"x\"\nversion = \"1\"\ncount = \"no\"\n",
            Toml,
            &pkg_schema(),
        );
        assert!(t.iter().any(|p| p.pointer == "/count" && p.line == 3));
    }

    #[test]
    fn crlf_source_lines_match() {
        let src = "{\r\n  \"name\": \"x\",\r\n  \"version\": \"1\",\r\n  \"count\": \"many\"\r\n}";
        let p = validate(src, Json, &pkg_schema());
        assert_eq!(p[0].line, 4);
    }

    #[test]
    fn syntax_error_is_a_lint_problem_with_line() {
        let p = validate("{\n \"a\": ,\n}", Json, &pkg_schema());
        assert_eq!(p[0].source, ProblemSource::Lint);
        assert_eq!(p[0].line, 2);
    }

    #[test]
    fn remote_schema_url_is_not_fetched() {
        let r = SchemaRef::Remote("https://example.invalid/s.json".into());
        let p = validate("{}", Json, &r);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].severity, Severity::Info);
        assert!(p[0].message.contains("not fetched"));
    }

    #[test]
    fn missing_local_schema_one_problem() {
        let r = SchemaRef::Local(PathBuf::from("/nonexistent/dir/s.json"));
        let p = validate("{}", Json, &r);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].severity, Severity::Info);
    }

    #[test]
    fn no_schema_still_reports_syntax_error() {
        let p = check_document(Some(Path::new("/w/x.json")), "{\n\"a\": ,\n}", Json, &[]);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].source, ProblemSource::Lint);
        assert!(check_document(None, "{\"a\":1}", Json, &[]).is_empty());
    }

    #[test]
    fn empty_file_no_problems() {
        for ft in [Json, Yaml, Toml] {
            assert!(validate("", ft, &pkg_schema()).is_empty());
            assert!(validate("  \n", ft, &pkg_schema()).is_empty());
        }
    }

    #[test]
    fn resolve_in_file_ref_forms() {
        let p = Path::new("/w/a.json");
        let r = resolve_schema(p, r#"{"$schema":"./s.json"}"#, Json, &[]);
        assert_eq!(r, Some(SchemaRef::Local(PathBuf::from("/w/./s.json"))));
        let r = resolve_schema(
            Path::new("/w/a.yaml"),
            "# yaml-language-server: $schema=https://x.dev/s.json\na: 1\n",
            Yaml,
            &[],
        );
        assert_eq!(r, Some(SchemaRef::Remote("https://x.dev/s.json".into())));
        let r = resolve_schema(
            Path::new("/w/a.toml"),
            "#:schema ./s.json\na = 1\n",
            Toml,
            &[],
        );
        assert!(matches!(r, Some(SchemaRef::Local(_))));
    }

    #[test]
    fn json_schema_meta_url_is_not_a_data_schema() {
        let src = r#"{"$schema":"http://json-schema.org/draft-07/schema#"}"#;
        assert_eq!(resolve_schema(Path::new("/w/s.json"), src, Json, &[]), None);
    }

    #[test]
    fn workspace_mapping_matches_glob() {
        let m = vec![("config/*.yaml".to_string(), "/s/app.json".to_string())];
        let r = resolve_schema(Path::new("/w/config/app.yaml"), "a: 1", Yaml, &m);
        assert_eq!(r, Some(SchemaRef::Local(PathBuf::from("/s/app.json"))));
        assert_eq!(
            resolve_schema(Path::new("/w/other/app.yaml"), "a: 1", Yaml, &m),
            None
        );
    }

    #[test]
    fn resolve_prefers_in_file_over_mapping() {
        let m = vec![("*.json".to_string(), "/s/map.json".to_string())];
        let r = resolve_schema(
            Path::new("/w/a.json"),
            r#"{"$schema":"/s/own.json"}"#,
            Json,
            &m,
        );
        assert_eq!(r, Some(SchemaRef::Local(PathBuf::from("/s/own.json"))));
    }

    #[test]
    fn resolve_prefers_inline_schema_over_bundled() {
        let p = Path::new("/w/package.json");
        assert!(matches!(
            resolve_schema(p, "{}", Json, &[]),
            Some(SchemaRef::Bundled(_))
        ));
        let r = resolve_schema(p, r#"{"$schema":"./own.json"}"#, Json, &[]);
        assert_eq!(r, Some(SchemaRef::Local(PathBuf::from("/w/./own.json"))));
    }

    #[test]
    fn bundled_package_json_reports_bad_type_on_its_line() {
        let src = "{\n  \"name\": \"x\",\n  \"private\": \"yes\"\n}";
        let p = check_document(Some(Path::new("/w/package.json")), src, Json, &[]);
        assert!(
            p.iter()
                .any(|p| p.source == ProblemSource::Schema && p.line == 3),
            "{p:?}"
        );
        let ok = "{\n  \"name\": \"x\",\n  \"private\": true\n}";
        assert!(check_document(Some(Path::new("/w/package.json")), ok, Json, &[]).is_empty());
    }

    #[test]
    fn bundled_cargo_toml_accepts_this_repo_manifest() {
        let src = include_str!("../../Cargo.toml");
        let p = check_document(Some(Path::new("/w/Cargo.toml")), src, Toml, &[]);
        assert!(p.is_empty(), "{p:?}");
    }

    #[test]
    fn bundled_workflow_accepts_this_repo_ci_and_flags_bad_job() {
        let path = Path::new("/w/.github/workflows/ci.yml");
        for src in [
            include_str!("../../.github/workflows/ci.yml"),
            include_str!("../../.github/workflows/release.yml"),
        ] {
            let p = check_document(Some(path), src, Yaml, &[]);
            assert!(p.is_empty(), "{p:?}");
        }
        let bad = "on: push\njobs:\n  build:\n    runs-on: ubuntu-latest\n    steps: 3\n";
        let p = check_document(Some(path), bad, Yaml, &[]);
        // `oneOf` failures are reported at the job, not the inner key.
        assert!(
            p.iter().any(|p| p.line == 3 && p.pointer == "/jobs/build"),
            "{p:?}"
        );
    }

    #[test]
    fn locate_falls_back_to_line_1() {
        // A pointer the locator cannot find still yields a usable position.
        let schema = SchemaRef::Inline(json!({"required": ["x"]}));
        let p = validate("[1]", Json, &schema);
        assert!(p.is_empty());
        let schema = SchemaRef::Inline(json!({"properties": {"a": {"type": "string"}}}));
        let p = validate("{\"a\": 1}", Json, &schema);
        assert_eq!((p[0].line, p[0].col), (1, 7));
        let p = validate("{}", Json, &SchemaRef::Inline(json!({"required": ["x"]})));
        assert_eq!((p[0].line, p[0].col), (1, 1));
    }
}
