# Developer tools — credits and third-party material

Ferrite is MIT-licensed and builds on the upstream project
[OlaProeis/Ferrite](https://github.com/OlaProeis/Ferrite) (MIT). The developer
tools add the crates and schema files below.

## Crates

Approved by Sean on 2026-11-14 and pinned to exact versions in `Cargo.toml`.

| Crate | Version | License | Used for |
|---|---|---|---|
| [`jsonschema`](https://crates.io/crates/jsonschema) | `=0.61.2` (default features off: no HTTP or file `$ref` loading) | MIT | Schema validation |
| [`serde_json_path`](https://crates.io/crates/serde_json_path) | `=0.7.2` | MIT | JSONPath (RFC 9535) query bar |
| [`similar`](https://crates.io/crates/similar) | `=3.2.1` | Apache-2.0 | Line diff for `--diff` on non-structured files |

The existing `serde_json` and `toml` dependencies now enable their
`preserve_order` feature, so Format and Convert keep the file's key order.

## Bundled JSON Schemas

Files in `assets/schemas/` are embedded into the binary. They are retrieved at
development time by `scripts/update_bundled_schemas.py` and **modified**:
annotation keywords (descriptions, examples, titles, defaults) are removed,
`$ref`s to other URLs are dropped (those sub-trees then accept any value), and
the JSON is minified. Validation rules are otherwise unchanged.

Retrieved 2026-11-15.

| File | Source | License |
|---|---|---|
| `package.json` | <https://www.schemastore.org/package.json> | Apache-2.0 (SchemaStore) |
| `tsconfig.json` | <https://www.schemastore.org/tsconfig.json> | Apache-2.0 (SchemaStore) |
| `cargo.json` | <https://www.schemastore.org/cargo.json> | Apache-2.0 (SchemaStore) |
| `pyproject.json` | <https://raw.githubusercontent.com/SchemaStore/schemastore/master/src/schemas/json/pyproject.json> | Apache-2.0 (SchemaStore) |
| `github-workflow.json` | <https://www.schemastore.org/github-workflow.json> | Apache-2.0 (SchemaStore) |
| `docker-compose.json` | <https://raw.githubusercontent.com/compose-spec/compose-go/master/schema/compose-spec.json> | Apache-2.0 (Compose Specification) |

SchemaStore: <https://github.com/SchemaStore/schemastore>.
Compose Specification: <https://github.com/compose-spec/compose-spec>.

### What the trimming loses

Sub-schemas that pointed at other SchemaStore files are not checked:
for example `package.json` → `eslintConfig`, `prettier`; `Cargo.toml` →
`[lints.*]`; `pyproject.toml` → most `[tool.*]` tables (ruff, mypy, poetry,
uv, …). Those keys accept any value.
