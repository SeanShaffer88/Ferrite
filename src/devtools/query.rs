//! JSONPath (RFC 9535) queries over a parsed document.

use serde_json::Value;
use serde_json_path::JsonPath;

/// Maximum number of hits returned before the query is rejected.
pub const MAX_HITS: usize = 10_000;

/// One query match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryHit {
    /// JSON pointer to the matched node.
    pub pointer: String,
    /// Short single-line preview of the matched value.
    pub preview: String,
}

/// Why a query failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryError {
    Syntax(String),
    TooManyHits(usize),
}

impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Syntax(m) => write!(f, "{m}"),
            Self::TooManyHits(n) => write!(f, "more than {n} matches"),
        }
    }
}

/// Short single-line preview (max 80 chars) of a value.
pub fn preview(v: &Value) -> String {
    let s = v.to_string();
    if s.chars().count() <= 80 {
        s
    } else {
        let cut: String = s.chars().take(79).collect();
        format!("{cut}…")
    }
}

/// Run `expr` against `value`. An empty expression matches nothing.
pub fn run_query(value: &Value, expr: &str) -> Result<Vec<QueryHit>, QueryError> {
    let expr = expr.trim();
    if expr.is_empty() {
        return Ok(Vec::new());
    }
    let path = JsonPath::parse(expr).map_err(|e| QueryError::Syntax(e.to_string()))?;
    let mut hits = Vec::new();
    for node in path.query_located(value) {
        if hits.len() >= MAX_HITS {
            return Err(QueryError::TooManyHits(MAX_HITS));
        }
        hits.push(QueryHit {
            pointer: node.location().to_json_pointer(),
            preview: preview(node.node()),
        });
    }
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn dollar_dot_name_finds_key() {
        let v = json!({"a": {"b": 1}});
        let h = run_query(&v, "$.a.b").unwrap();
        assert_eq!(
            h,
            vec![QueryHit {
                pointer: "/a/b".into(),
                preview: "1".into()
            }]
        );
    }

    #[test]
    fn wildcard_returns_all_items() {
        let v = json!({"xs": [10, 20, 30]});
        let h = run_query(&v, "$.xs[*]").unwrap();
        let p: Vec<_> = h.iter().map(|x| x.pointer.as_str()).collect();
        assert_eq!(p, ["/xs/0", "/xs/1", "/xs/2"]);
    }

    #[test]
    fn bad_expression_returns_syntax_error() {
        assert!(matches!(
            run_query(&json!({}), "$.[["),
            Err(QueryError::Syntax(_))
        ));
    }

    #[test]
    fn null_value_returns_empty() {
        assert!(run_query(&Value::Null, "$.a").unwrap().is_empty());
        assert!(run_query(&json!({}), "   ").unwrap().is_empty());
    }

    #[test]
    fn hit_cap_enforced() {
        let v = Value::Array((0..MAX_HITS + 1).map(Value::from).collect());
        assert_eq!(
            run_query(&v, "$[*]"),
            Err(QueryError::TooManyHits(MAX_HITS))
        );
    }

    #[test]
    fn long_preview_is_truncated() {
        let v = json!({"s": "x".repeat(500)});
        let h = run_query(&v, "$.s").unwrap();
        assert!(h[0].preview.chars().count() <= 80);
    }
}
