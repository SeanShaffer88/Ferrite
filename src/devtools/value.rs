//! Parse JSON / YAML / TOML text into one common [`serde_json::Value`] tree.
//!
//! Empty or whitespace-only input yields `Value::Null` (not an error) so that
//! a freshly created file never produces problems. YAML with several
//! documents uses the first document only.

use crate::markdown::tree_viewer::{ParseError, StructuredFileType};
use serde::Deserialize;
use serde_json::Value;

/// Parse `content` into a common value tree.
pub fn parse_to_value(content: &str, ft: StructuredFileType) -> Result<Value, ParseError> {
    if content.trim().is_empty() {
        return Ok(Value::Null);
    }
    match ft {
        StructuredFileType::Json => serde_json::from_str(content.trim_start_matches('\u{feff}'))
            .map_err(|e| ParseError {
                message: e.to_string(),
                line: Some(e.line()),
            }),
        StructuredFileType::Yaml => parse_yaml(content),
        StructuredFileType::Toml => parse_toml(content),
    }
}

fn parse_yaml(content: &str) -> Result<Value, ParseError> {
    let mut docs = serde_yaml::Deserializer::from_str(content);
    match docs.next() {
        None => Ok(Value::Null),
        Some(doc) => Value::deserialize(doc).map_err(|e| ParseError {
            message: e.to_string(),
            line: e.location().map(|l| l.line()),
        }),
    }
}

fn parse_toml(content: &str) -> Result<Value, ParseError> {
    let parsed: toml::Value = toml::from_str(content.trim_start_matches('\u{feff}')).map_err(|e| {
        let line = e
            .span()
            .map(|s| content[..s.start.min(content.len())].matches('\n').count() + 1);
        ParseError {
            message: e.message().to_string(),
            line,
        }
    })?;
    Ok(toml_to_json(parsed))
}

fn toml_to_json(v: toml::Value) -> Value {
    match v {
        toml::Value::String(s) => Value::String(s),
        toml::Value::Integer(i) => Value::from(i),
        toml::Value::Float(f) => serde_json::Number::from_f64(f)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        toml::Value::Boolean(b) => Value::Bool(b),
        toml::Value::Datetime(d) => Value::String(d.to_string()),
        toml::Value::Array(a) => Value::Array(a.into_iter().map(toml_to_json).collect()),
        toml::Value::Table(t) => {
            Value::Object(t.into_iter().map(|(k, v)| (k, toml_to_json(v))).collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_json_yaml_toml_to_same_value() {
        let want = json!({"name": "x", "n": 3, "list": [1, 2]});
        let j = parse_to_value(r#"{"name":"x","n":3,"list":[1,2]}"#, StructuredFileType::Json);
        let y = parse_to_value("name: x\nn: 3\nlist: [1, 2]\n", StructuredFileType::Yaml);
        let t = parse_to_value("name = \"x\"\nn = 3\nlist = [1, 2]\n", StructuredFileType::Toml);
        assert_eq!(j.unwrap(), want);
        assert_eq!(y.unwrap(), want);
        assert_eq!(t.unwrap(), want);
    }

    #[test]
    fn empty_input_returns_null_not_panic() {
        for ft in [
            StructuredFileType::Json,
            StructuredFileType::Yaml,
            StructuredFileType::Toml,
        ] {
            assert_eq!(parse_to_value("", ft).unwrap(), Value::Null);
        }
    }

    #[test]
    fn whitespace_only_yaml_is_null() {
        let v = parse_to_value("  \n\t\n", StructuredFileType::Yaml).unwrap();
        assert_eq!(v, Value::Null);
    }

    #[test]
    fn multi_document_yaml_uses_first_doc() {
        let v = parse_to_value("a: 1\n---\nb: 2\n", StructuredFileType::Yaml).unwrap();
        assert_eq!(v, json!({"a": 1}));
    }

    #[test]
    fn invalid_json_returns_parse_error_with_line() {
        let e = parse_to_value("{\n  \"a\": ,\n}", StructuredFileType::Json).unwrap_err();
        assert_eq!(e.line, Some(2));
    }

    #[test]
    fn toml_datetime_becomes_string() {
        let v = parse_to_value("d = 1979-05-27T07:32:00Z\n", StructuredFileType::Toml).unwrap();
        assert_eq!(v, json!({"d": "1979-05-27T07:32:00Z"}));
    }

    #[test]
    fn bom_is_ignored() {
        let v = parse_to_value("\u{feff}{\"a\":1}", StructuredFileType::Json).unwrap();
        assert_eq!(v, json!({"a": 1}));
    }
}
