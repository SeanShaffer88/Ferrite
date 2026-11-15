//! Developer tooling: schema validation, query, diff, convert, CLI helpers.
//!
//! Pure logic lives here and is unit-tested without any UI. UI hooks in other
//! modules call into this tree. Every feature has a toggle in
//! [`DevtoolsSettings`].

pub mod cli_input;
pub mod cli_target;
pub mod convert;
pub mod diff;
pub mod locate;
pub mod query;
pub mod schema;
pub mod schemas;
pub mod value;

use serde::{Deserialize, Serialize};

/// User settings for the developer tools. Missing keys fall back to defaults
/// so older config files keep loading.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DevtoolsSettings {
    /// Validate JSON/YAML/TOML against a schema and show problems.
    pub schema_validation: bool,
    /// Show the query bar in the tree viewer.
    pub query_bar: bool,
    /// Schema mappings: (file glob, schema path or URL). In-file references win.
    pub schema_mappings: Vec<(String, String)>,
    /// Column guide for git commit/tag/merge messages (`None` = off).
    pub git_editor_ruler: Option<u32>,
}

impl Default for DevtoolsSettings {
    fn default() -> Self {
        Self {
            schema_validation: true,
            query_bar: true,
            schema_mappings: Vec::new(),
            git_editor_ruler: Some(72),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_spec() {
        let d = DevtoolsSettings::default();
        assert!(d.schema_validation && d.query_bar);
        assert!(d.schema_mappings.is_empty());
        assert_eq!(d.git_editor_ruler, Some(72));
    }

    #[test]
    fn git_editor_ruler_can_be_turned_off() {
        let d: DevtoolsSettings = serde_json::from_str(r#"{"git_editor_ruler":null}"#).unwrap();
        assert_eq!(d.git_editor_ruler, None);
        let d: DevtoolsSettings = serde_json::from_str(r#"{"git_editor_ruler":50}"#).unwrap();
        assert_eq!(d.git_editor_ruler, Some(50));
    }

    #[test]
    fn partial_json_fills_defaults() {
        let d: DevtoolsSettings = serde_json::from_str(r#"{"query_bar":false}"#).unwrap();
        assert!(d.schema_validation);
        assert!(!d.query_bar);
    }
}
