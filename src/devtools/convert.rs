//! Format, sort, minify and convert between JSON / YAML / TOML.

use super::value::parse_to_value;
use crate::markdown::tree_viewer::StructuredFileType;
use serde_json::{Map, Value};

/// Result of a conversion or reformat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Converted {
    pub text: String,
    /// Things that did not survive (comments, nulls, ...). Empty = lossless.
    pub lossy: Vec<String>,
}

/// Options for [`reformat`].
#[derive(Debug, Clone, Copy, Default)]
pub struct ReformatOpts {
    pub sort_keys: bool,
    /// JSON only; ignored for YAML and TOML.
    pub minify: bool,
}

fn has_comments(content: &str, ft: StructuredFileType) -> bool {
    matches!(ft, StructuredFileType::Yaml | StructuredFileType::Toml)
        && content.lines().any(|l| l.trim_start().starts_with('#'))
}

fn sort_value(v: Value) -> Value {
    match v {
        Value::Object(m) => {
            let mut entries: Vec<(String, Value)> = m.into_iter().collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            Value::Object(
                entries
                    .into_iter()
                    .map(|(k, v)| (k, sort_value(v)))
                    .collect::<Map<_, _>>(),
            )
        }
        Value::Array(a) => Value::Array(a.into_iter().map(sort_value).collect()),
        other => other,
    }
}

/// Remove nulls (TOML has none). Returns how many were dropped.
fn strip_nulls(v: &mut Value) -> usize {
    match v {
        Value::Object(m) => {
            let before = m.len();
            m.retain(|_, x| !x.is_null());
            let mut n = before - m.len();
            for x in m.values_mut() {
                n += strip_nulls(x);
            }
            n
        }
        Value::Array(a) => {
            let before = a.len();
            a.retain(|x| !x.is_null());
            let mut n = before - a.len();
            for x in a.iter_mut() {
                n += strip_nulls(x);
            }
            n
        }
        _ => 0,
    }
}

fn serialize(v: &Value, to: StructuredFileType, minify: bool) -> Result<String, String> {
    match to {
        StructuredFileType::Json => if minify {
            serde_json::to_string(v)
        } else {
            serde_json::to_string_pretty(v).map(|mut s| {
                s.push('\n');
                s
            })
        }
        .map_err(|e| e.to_string()),
        StructuredFileType::Yaml => serde_yaml::to_string(v).map_err(|e| e.to_string()),
        StructuredFileType::Toml => {
            if !v.is_object() {
                return Err("TOML root must be a table".to_string());
            }
            toml::to_string_pretty(v).map_err(|e| e.to_string())
        }
    }
}

/// Convert `content` from one format to another.
pub fn convert(
    content: &str,
    from: StructuredFileType,
    to: StructuredFileType,
) -> Result<Converted, String> {
    let mut value = parse_to_value(content, from).map_err(|e| e.to_string())?;
    let mut lossy = Vec::new();
    if has_comments(content, from) {
        lossy.push("comments are not preserved".to_string());
    }
    if to == StructuredFileType::Toml {
        let n = strip_nulls(&mut value);
        if n > 0 {
            lossy.push(format!("{n} null value(s) dropped (TOML has no null)"));
        }
    }
    let text = serialize(&value, to, false)?;
    Ok(Converted { text, lossy })
}

/// Pretty-print (or minify) a document, optionally sorting keys.
/// An empty document is returned unchanged.
pub fn reformat(
    content: &str,
    ft: StructuredFileType,
    o: ReformatOpts,
) -> Result<Converted, String> {
    let mut value = parse_to_value(content, ft).map_err(|e| e.to_string())?;
    if value.is_null() {
        return Ok(Converted {
            text: content.to_string(),
            lossy: Vec::new(),
        });
    }
    if o.sort_keys {
        value = sort_value(value);
    }
    let mut lossy = Vec::new();
    if has_comments(content, ft) {
        lossy.push("comments are not preserved".to_string());
    }
    let text = serialize(&value, ft, o.minify && ft == StructuredFileType::Json)?;
    Ok(Converted { text, lossy })
}

#[cfg(test)]
mod tests {
    use super::*;
    use StructuredFileType::*;

    #[test]
    fn json_to_yaml_round_trips_value() {
        let src = r#"{"b":1,"a":{"x":[1,2,3],"s":"hi"}}"#;
        let y = convert(src, Json, Yaml).unwrap();
        assert!(y.lossy.is_empty());
        let back = convert(&y.text, Yaml, Json).unwrap();
        assert_eq!(
            parse_to_value(&back.text, Json).unwrap(),
            parse_to_value(src, Json).unwrap()
        );
    }

    #[test]
    fn key_order_is_preserved_by_default() {
        let out = reformat(r#"{"b":1,"a":2}"#, Json, ReformatOpts::default()).unwrap();
        assert!(out.text.find("\"b\"").unwrap() < out.text.find("\"a\"").unwrap());
    }

    #[test]
    fn yaml_comments_reported_lossy() {
        let c = convert("# note\na: 1\n", Yaml, Json).unwrap();
        assert_eq!(c.lossy, vec!["comments are not preserved".to_string()]);
    }

    #[test]
    fn toml_null_reports_unrepresentable() {
        let c = convert(r#"{"a":1,"b":null,"c":[1,null]}"#, Json, Toml).unwrap();
        assert!(c.lossy.iter().any(|l| l.contains("2 null")));
        assert!(!c.text.contains("null"));
    }

    #[test]
    fn json_to_toml_handles_table_before_scalar() {
        let c = convert(r#"{"t":{"k":1},"s":"x"}"#, Json, Toml).unwrap();
        assert!(parse_to_value(&c.text, Toml).is_ok());
    }

    #[test]
    fn toml_root_must_be_table() {
        assert!(convert("[1,2]", Json, Toml).is_err());
    }

    #[test]
    fn sort_keys_is_stable_and_recursive() {
        let o = ReformatOpts {
            sort_keys: true,
            minify: true,
        };
        let out = reformat(r#"{"b":{"z":1,"y":2},"a":0}"#, Json, o).unwrap();
        assert_eq!(out.text, r#"{"a":0,"b":{"y":2,"z":1}}"#);
        let again = reformat(&out.text, Json, o).unwrap();
        assert_eq!(again.text, out.text);
    }

    #[test]
    fn minify_json_has_no_whitespace() {
        let o = ReformatOpts {
            sort_keys: false,
            minify: true,
        };
        let out = reformat("{ \"a\" : [ 1, 2 ] }", Json, o).unwrap();
        assert_eq!(out.text, r#"{"a":[1,2]}"#);
    }

    #[test]
    fn invalid_input_returns_error() {
        assert!(reformat("{nope", Json, ReformatOpts::default()).is_err());
    }

    #[test]
    fn empty_input_unchanged() {
        let out = reformat("  \n", Yaml, ReformatOpts::default()).unwrap();
        assert_eq!(out.text, "  \n");
    }
}
