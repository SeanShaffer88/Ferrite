//! Schemas bundled into the binary (offline). See `docs/devtools-credits.md`.
//!
//! Regenerate with `python scripts/update_bundled_schemas.py` (needs network;
//! the app never fetches). Files are trimmed: annotations removed, remote
//! `$ref`s dropped so those sub-trees accept anything.

use std::path::Path;

/// (asset name, schema text), embedded at build time.
pub const BUNDLED: &[(&str, &str)] = &[
    (
        "package",
        include_str!("../../../assets/schemas/package.json"),
    ),
    (
        "tsconfig",
        include_str!("../../../assets/schemas/tsconfig.json"),
    ),
    ("cargo", include_str!("../../../assets/schemas/cargo.json")),
    (
        "pyproject",
        include_str!("../../../assets/schemas/pyproject.json"),
    ),
    (
        "docker-compose",
        include_str!("../../../assets/schemas/docker-compose.json"),
    ),
    (
        "github-workflow",
        include_str!("../../../assets/schemas/github-workflow.json"),
    ),
];

fn get(name: &str) -> Option<&'static str> {
    BUNDLED.iter().find(|(n, _)| *n == name).map(|(_, s)| *s)
}

/// Embedded schema text for a file, matched by file name / path.
pub fn bundled_for(path: &Path) -> Option<&'static str> {
    // Normalise separators so Windows paths match on every OS.
    let full = path.to_string_lossy().replace('\\', "/");
    let file = full.rsplit('/').next().unwrap_or("");
    let lower = file.to_ascii_lowercase();
    let yaml = lower.ends_with(".yml") || lower.ends_with(".yaml");

    if full.contains("/.github/workflows/") && yaml {
        return get("github-workflow");
    }
    let name = match file {
        "package.json" => "package",
        "Cargo.toml" => "cargo",
        "pyproject.toml" => "pyproject",
        f if f.starts_with("tsconfig") && f.ends_with(".json") => "tsconfig",
        _ if yaml && (lower.starts_with("docker-compose.") || lower.starts_with("compose.")) => {
            "docker-compose"
        }
        _ => return None,
    };
    get(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    const ALL: &[&str] = &[
        "package.json",
        "tsconfig.json",
        "tsconfig.build.json",
        "Cargo.toml",
        "pyproject.toml",
        "docker-compose.yml",
        "compose.yaml",
        "docker-compose.prod.yaml",
        "/repo/.github/workflows/ci.yml",
        "C:\\repo\\.github\\workflows\\release.yaml",
    ];

    #[test]
    fn bundled_names_resolve() {
        for name in ALL {
            assert!(
                bundled_for(Path::new(name)).is_some(),
                "{name} should resolve"
            );
        }
        for name in [
            "/repo/ci.yml",
            "/repo/.github/ci.yml",
            "cargo.toml.bak",
            "notpackage.json",
        ] {
            assert!(
                bundled_for(Path::new(name)).is_none(),
                "{name} should not resolve"
            );
        }
    }

    #[test]
    fn every_bundled_schema_is_valid_json() {
        for name in ALL {
            let text = bundled_for(Path::new(name)).unwrap_or_default();
            let v: serde_json::Value = serde_json::from_str(text).expect(name);
            // Must also compile offline: no unresolvable remote $ref left.
            jsonschema::validator_for(&v).unwrap_or_else(|e| panic!("{name}: {e}"));
        }
    }

    #[test]
    fn total_bundled_size_under_500kb() {
        let total: usize = BUNDLED.iter().map(|(_, s)| s.len()).sum();
        assert!(total < 500 * 1024, "bundled schemas are {total} bytes");
    }
}
