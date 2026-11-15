# Developer tools phase 1 — audit note

Branch `claude/developer-edition-design`. Plan:
`docs/superpowers/plans/2026-11-14-developer-edition.md`. Date: 2026-11-15.

## Verified

- `cargo build --locked`, `cargo fmt --check` on branch files, and
  `cargo clippy --locked --all-targets` pass on Windows 11 (Rust 1.100). Clippy
  warnings count is unchanged from the pre-existing baseline.
- `cargo test --locked`: full suite green, run several times to catch flakes.
- Unit tests per module: `devtools::{value, locate, schema, schemas, query,
  convert, diff}`, `ui::{problems_panel, query_bar}`, `vcs::git::head_blob`.
- Bundled schemas: each one compiles with `jsonschema` offline; this repo's own
  `Cargo.toml`, `.github/workflows/ci.yml` and `release.yml` validate clean;
  a bad `package.json` field and a bad workflow job are reported on the right
  line.
- Review focus from the plan:
  - Empty / whitespace-only files: no panic, no problems, empty query
    (`empty_file_no_problems`, `empty_input_returns_null_not_panic`,
    `empty_document_and_empty_query_are_quiet`).
  - Large input: validation and query run on worker threads with a generation
    guard; submit returns quickly on a 50 MB string
    (`large_input_does_not_block_submit`). An older query can no longer
    overwrite a newer result (fixed on this branch).
  - Remote `$schema`: never fetched, one info problem
    (`remote_schema_url_is_not_fetched`); `jsonschema` is built without default
    features, so it cannot load remote `$ref`s either.

## Bugs found and fixed while finishing the WIP

- Tolerant config load dropped valid keys under `serde_json/preserve_order`
  (`Map::remove` is a swap-remove there). Now `shift_remove`.
- Problems debounce measured from the throttled check, not the last edit.
- Query-bar worker race (stale result overwrote the newest).

## Not verified

- No interactive GUI run in this session: Problems panel layout, squiggle
  placement, query-bar result clicks, and the Format/Convert commands were not
  exercised by hand. The panel was built without the wireframe sign-off the
  plan asked for.
- Validation latency on a real 50 MB file in the running app.
- macOS and Linux.

## Known limits

- Bundled schemas drop sub-schemas that point at other online schemas
  (`eslintConfig`, `[lints.*]`, most `[tool.*]`); see
  [`devtools-credits.md`](../devtools-credits.md).
- `oneOf` failures (common in the workflow schema) are reported at the parent
  node with the validator's generic message.
- No opt-in schema catalog download (deferred in the plan).
