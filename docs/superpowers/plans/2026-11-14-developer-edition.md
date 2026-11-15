# Ferrite Developer Edition — Phases 1 & 2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a config/data editing toolkit (schema check, query, diff, convert) and an `$EDITOR`-grade CLI to Ferrite.

**Architecture:** New `src/devtools/` module tree holds all pure logic (value tree, schema, query, diff, convert), tested without UI. Thin UI hooks in existing files call it. CLI work extends `Cli` in `src/main.rs` and the single-instance protocol in `src/single_instance.rs`. Heavy work runs on `src/workers/`, never the UI thread.

**Tech Stack:** Rust 1.92+, egui/eframe, existing `serde_json`, `serde_yaml 0.9`, `toml 0.8`, `git2`, `clap 4`. New crates need approval (Task 0).

**Spec:** `docs/superpowers/specs/2026-11-14-developer-edition-design.md`. Phase 3 (docs workbench) gets its own plan later.

## Global Constraints

- Rust 1.92+, egui/eframe; reuse existing crates before adding any.
- **No new crate without Sean's explicit approval; pin exact versions (`=x.y.z`).**
- Offline-first: no network in code paths by default or in tests; schema catalog fetch opt-in only.
- New code lives in `src/devtools/`; each feature has a settings toggle; touch existing files minimally (surgical edits, no adjacent cleanup).
- No `unwrap()`/`expect()` in production paths; errors propagate.
- All user-visible strings via `locales/` (`rust-i18n`); add keys to `locales/en.yaml`.
- Diagnostics are source-tagged (`Schema | Lint | Lsp`) so LSP can plug in later; `lsp` stays behind its feature flag.
- Do not add mockups or demo content to the repo.
- Each task: `cargo fmt`, `cargo clippy`, full `cargo test --locked` pass before commit.
- Ship per phase: CHANGELOG, README section, docs page, audit note, credits (upstream OlaProeis MIT + new crates + schema sources).

## Review Focus

1. Empty or whitespace-only file: no panic, zero problems, query returns nothing. (T1, T2)
2. Large file (≈50 MB JSON): validate/query run off the UI thread, latest request wins, UI never blocks. (T4, T5)
3. `$schema` pointing to an `http(s)` URL or a missing local file: never fetched; one clear info/problem, not a crash. (T2)
4. Targets like `C:\a b\x.rs:12:3`, `x.rs:abc`, `x.rs:0`: drive-letter colon kept in path; bad suffix treated as part of the name. (T8)
5. `--wait` when the tab is closed via window X, the app crashes, or the primary dies: waiter exits (nonzero on crash), never hangs. (T10)

---

## File Structure

| File | Responsibility |
|---|---|
| `src/devtools/mod.rs` | module root, `DevtoolsSettings` re-export |
| `src/devtools/value.rs` | text → `serde_json::Value` for JSON/YAML/TOML |
| `src/devtools/locate.rs` | JSON pointer → (line, col) in source |
| `src/devtools/schema.rs` | schema resolve + validate → `Problem`s |
| `src/devtools/schemas/` + `assets/schemas/*.json` | bundled schemas |
| `src/devtools/query.rs` | JSONPath query |
| `src/devtools/convert.rs` | format/sort/minify/convert |
| `src/devtools/diff.rs` | structured diff |
| `src/devtools/cli_target.rs` | `path:line:col` parse, git-editor detect |
| `src/ui/problems_panel.rs` | Problems panel + squiggle feed |
| Modify: `src/config/settings.rs`, `src/ui/action_registry.rs`, `src/markdown/tree_viewer.rs` (query box), `src/main.rs` (`Cli`), `src/single_instance.rs`, `locales/en.yaml`, `src/vcs/git.rs` |

---

## Phase 1 — Config/data editor

### Task 0: Dependency approval gate

- [ ] **Step 1:** Ask Sean to approve, with exact pinned versions looked up on crates.io: a JSON Schema validator (`jsonschema`), a JSONPath crate (`serde_json_path`), a text diff crate (`similar`, needed only for non-structured `--diff` in T12). Record approved names and versions in this plan and `docs/devtools-credits.md`.
- [ ] **Step 2:** If a crate is refused, the owning task falls back: schema → hand-rolled subset (type/required/properties/enum) with the limit documented; query → dotted-path only; diff → structured files only.

### Task 1: Value tree and settings

**Files:** Create `src/devtools/mod.rs`, `src/devtools/value.rs`. Modify `src/config/settings.rs`, `src/main.rs` (`mod devtools;`). Test: unit tests in `value.rs`.

**Interfaces — Produces:**
`pub fn parse_to_value(content: &str, ft: StructuredFileType) -> Result<serde_json::Value, ParseError>` (reuse `StructuredFileType`, `ParseError` from `src/markdown/tree_viewer.rs`); `DevtoolsSettings { schema_validation: bool, query_bar: bool, git_editor_ruler: Option<u32> }` with defaults `true, true, Some(72)`, serde-defaulted so old configs load.

- [ ] **Step 1:** Tests `parses_json_yaml_toml_to_same_value`, `empty_input_returns_null_not_panic`, `whitespace_only_yaml_is_null`, `multi_document_yaml_uses_first_doc`, `invalid_json_returns_parse_error_with_line`.
- [ ] **Step 2:** Run `cargo test -p ferrite devtools::value` — expect FAIL (undefined).
- [ ] **Step 3:** Implement with `serde_json`, `serde_yaml::from_str`, `toml::from_str` → `Value`; empty/whitespace → `Value::Null`. Add `devtools: DevtoolsSettings` to settings with `#[serde(default)]`.
- [ ] **Step 4:** Run tests — PASS; load an old-config fixture test `settings_without_devtools_key_loads`.
- [ ] **Step 5:** Commit `feat(devtools): value tree and settings`.

### Task 2: Pointer locate + schema validation

**Files:** Create `src/devtools/locate.rs`, `src/devtools/schema.rs`. Test: unit tests in both.

**Interfaces — Consumes:** `parse_to_value`. **Produces:**
`pub struct Problem { pub source: ProblemSource, pub line: usize, pub col: usize, pub message: String, pub pointer: String }`;
`pub enum ProblemSource { Schema, Lint, Lsp }`;
`pub fn locate(content: &str, ft: StructuredFileType, pointer: &str) -> Option<(usize, usize)>` (1-based; best effort);
`pub enum SchemaRef { Inline(serde_json::Value), Local(PathBuf), Remote(String), Bundled(&'static str) }`;
`pub fn resolve_schema(path: &Path, content: &str, ft: StructuredFileType, mappings: &[(String, String)]) -> Option<SchemaRef>` — order: `$schema` / `# yaml-language-server: $schema=` / `#:schema` → workspace mapping → bundled filename match;
`pub fn validate(content: &str, ft: StructuredFileType, schema: &SchemaRef) -> Vec<Problem>` — never fetches `Remote`; returns one `Schema` info problem "remote schema not fetched" instead.

- [ ] **Step 1:** Tests: `required_property_missing_reports_line_of_parent`, `wrong_type_points_at_value_line`, `crlf_source_lines_match`, `utf8_bom_does_not_shift_columns`, `remote_schema_url_is_not_fetched` (assert no socket use: Remote path returns info problem), `missing_local_schema_one_problem`, `empty_file_no_problems`, `resolve_prefers_inline_schema_over_bundled`, `locate_falls_back_to_line_1`.
- [ ] **Step 2:** Run — FAIL.
- [ ] **Step 3:** Implement `locate` by scanning source for the key path (JSON token scan; YAML indentation scan; TOML table/key scan); fallback `(1,1)`. Implement `validate` with the approved validator; map each error's instance pointer through `locate`.
- [ ] **Step 4:** Run — PASS.
- [ ] **Step 5:** Commit `feat(devtools): schema resolve and validate`.

### Task 3: Bundled schemas

**Files:** Create `assets/schemas/*.json`, `src/devtools/schemas/mod.rs`, `docs/devtools-credits.md`. Test: unit test in `schemas/mod.rs`.

**Interfaces — Produces:** `pub fn bundled_for(file_name: &str) -> Option<&'static str>` (embedded via `include_str!`). Initial set: `package.json`, `tsconfig.json`, `Cargo.toml`, GitHub Actions workflows (`.github/workflows/*.yml`), `docker-compose.yml`, `pyproject.toml`.

- [ ] **Step 1:** Tests `bundled_names_resolve`, `every_bundled_schema_is_valid_json`, `total_bundled_size_under_500kb`.
- [ ] **Step 2:** Run — FAIL.
- [ ] **Step 3:** Obtain schemas from SchemaStore (Apache-2.0) during development (needs network; if blocked, ask Sean to supply files). Record source URL + license + retrieval date per file in credits. Wire `resolve_schema` bundled branch to `bundled_for`.
- [ ] **Step 4:** Run — PASS.
- [ ] **Step 5:** Commit `feat(devtools): bundled schema set`.

### Task 4: Problems panel and squiggles

**Files:** Create `src/ui/problems_panel.rs`. Modify `src/ui/mod.rs`, `src/main.rs`/`src/app/mod.rs` (hook on text change), `src/workers/` (job), `locales/en.yaml`. Test: unit tests on the job/debounce logic.

**Interfaces — Consumes:** `resolve_schema`, `validate`, `Problem`. **Produces:** `pub struct ProblemsState { problems: Vec<Problem>, generation: u64 }`; `pub fn submit_validation(state: &mut ProblemsState, content: String, ...)` — runs on a worker, discards results whose `generation` is stale.

Wireframe to confirm with Sean before UI code (kept out of the repo):
```
┌ Problems (3) ───────────────────────────────┐
│ ✖ L12:5  "version" is required   [schema]    │
│ ⚠ L30:9  expected number, got string         │
│ ℹ L1:1   remote schema not fetched           │
└──────────────────────────────────────────────┘
```

- [ ] **Step 1:** Tests `stale_generation_result_is_dropped`, `rapid_edits_debounced_to_one_job`, `toggle_off_clears_problems`, `large_input_does_not_block_submit` (submit returns < 5 ms on 50 MB string).
- [ ] **Step 2:** Run — FAIL.
- [ ] **Step 3:** Implement worker job + state; panel lists problems, click jumps to line; squiggle underline fed from `Problem` line/col. Toggle from `DevtoolsSettings.schema_validation`. Every control has a tooltip; Esc closes panel.
- [ ] **Step 4:** Run full tests — PASS; manual check on a real `package.json` with a bad type.
- [ ] **Step 5:** Commit `feat(ui): problems panel`.

### Task 5: Query bar

**Files:** Create `src/devtools/query.rs`. Modify `src/markdown/tree_viewer.rs` (query box + highlight), `locales/en.yaml`. Test: unit tests in `query.rs`.

**Interfaces — Produces:** `pub struct QueryHit { pub pointer: String, pub preview: String }`; `pub enum QueryError { Syntax(String), TooManyHits(usize) }`; `pub fn run_query(value: &serde_json::Value, expr: &str) -> Result<Vec<QueryHit>, QueryError>` (JSONPath; cap 10 000 hits).

- [ ] **Step 1:** Tests `dollar_dot_name_finds_key`, `wildcard_returns_all_items`, `bad_expression_returns_syntax_error`, `null_value_returns_empty`, `hit_cap_enforced`, `pointer_matches_tree_node_path`.
- [ ] **Step 2:** Run — FAIL.
- [ ] **Step 3:** Implement with approved JSONPath crate; convert node paths to JSON pointers. Tree viewer: query box (placeholder text via locale), results list, click expands path and scrolls. Run query on a worker for large docs (generation guard as in T4).
- [ ] **Step 4:** Run — PASS.
- [ ] **Step 5:** Commit `feat(devtools): query bar`.

### Task 6: Format and convert

**Files:** Create `src/devtools/convert.rs`. Modify `src/ui/action_registry.rs` (commands: Format, Sort Keys, Minify, Convert To…), `locales/en.yaml`. Test: unit tests in `convert.rs`.

**Interfaces — Produces:** `pub struct Converted { pub text: String, pub lossy: Vec<String> }`; `pub fn convert(content: &str, from: StructuredFileType, to: StructuredFileType) -> Result<Converted, String>`; `pub struct ReformatOpts { pub sort_keys: bool, pub minify: bool }`; `pub fn reformat(content: &str, ft: StructuredFileType, o: ReformatOpts) -> Result<String, String>`.

- [ ] **Step 1:** Tests `json_to_yaml_round_trips_value`, `yaml_comments_reported_lossy`, `toml_null_reports_unrepresentable`, `sort_keys_is_stable_and_recursive`, `minify_json_has_no_whitespace`, `invalid_input_returns_error_unchanged_buffer`.
- [ ] **Step 2:** Run — FAIL.
- [ ] **Step 3:** Implement via `Value` + serializers. Converting shows a confirm dialog listing `lossy` items before replacing the buffer (undoable as one edit).
- [ ] **Step 4:** Run — PASS.
- [ ] **Step 5:** Commit `feat(devtools): format and convert`.

### Task 7: Structured diff

**Files:** Create `src/devtools/diff.rs`. Modify `src/vcs/git.rs` (add `head_blob`), diff view in `src/ui/` (new `structured_diff.rs`), `locales/en.yaml`. Test: unit tests.

**Interfaces — Produces:**
`pub enum DiffKind { Added, Removed, Changed }`; `pub struct DiffEntry { pub pointer: String, pub kind: DiffKind, pub old: Option<String>, pub new: Option<String> }`; `pub fn diff_values(a: &Value, b: &Value) -> Vec<DiffEntry>`; `pub fn head_blob(repo_root: &Path, rel: &Path) -> Result<Option<String>, String>` in `src/vcs/git.rs`.

- [ ] **Step 1:** Tests `key_order_ignored`, `added_removed_changed_detected`, `array_index_change_reported`, `identical_values_empty_diff`, `head_blob_none_for_untracked_file`.
- [ ] **Step 2:** Run — FAIL.
- [ ] **Step 3:** Implement recursive diff; side-by-side list view grouped by pointer; entry point "Compare with HEAD" and "Compare with file…".
- [ ] **Step 4:** Run — PASS.
- [ ] **Step 5:** Commit `feat(devtools): structured diff`.

### Task 8a: Phase 1 release docs

- [ ] **Step 1:** README "For developers" section, `docs/devtools.md`, CHANGELOG entry, `docs/devtools-credits.md`, audit note `docs/technical/devtools-audit-phase1.md` (what was verified, what not).
- [ ] **Step 2:** Commit `docs: developer tools phase 1`.

---

## Phase 2 — `$EDITOR` replacement

### Task 8: Open-target parsing

**Files:** Create `src/devtools/cli_target.rs`. Modify `src/main.rs` (`Cli` at line ~162: `paths: Vec<String>` raw, parsed after). Test: unit tests.

**Interfaces — Produces:** `pub struct OpenTarget { pub path: PathBuf, pub line: Option<u32>, pub col: Option<u32> }`; `pub fn parse_open_target(arg: &str) -> OpenTarget` — strips trailing `:line[:col]` only when numeric and ≥ 1 and the remaining path is non-empty; keeps Windows drive prefix.

- [ ] **Step 1:** Tests `plain_path_unchanged`, `file_colon_line`, `file_colon_line_colon_col`, `windows_drive_path_kept`, `windows_drive_with_line` (`C:\a b\x.rs:12:3`), `non_numeric_suffix_part_of_name`, `line_zero_ignored`, `existing_file_named_with_colon_wins` (if the full string exists on disk, use it as-is).
- [ ] **Step 2:** Run — FAIL.
- [ ] **Step 3:** Implement; also `--goto <path:line[:col]>` alias.
- [ ] **Step 4:** Run — PASS.
- [ ] **Step 5:** Commit `feat(cli): path:line:col targets`.

### Task 9: Forward line/col through single-instance

**Files:** Modify `src/single_instance.rs` (forward + receive), `FerriteApp::handle_instance_paths` (jump to line/col). Test: unit tests in `single_instance.rs`.

**Interfaces — Consumes:** `OpenTarget`. Protocol: each line is `path` (old) or `path\t<line>\t<col>` (new); old senders/receivers stay compatible.

- [ ] **Step 1:** Tests `encode_decode_target_round_trip`, `legacy_plain_path_line_decodes`, `tab_in_path_is_escaped`, `line_past_eof_clamps_to_last_line`.
- [ ] **Step 2:** Run — FAIL.
- [ ] **Step 3:** Implement encode/decode helpers; receiver passes `OpenTarget` to the app, which opens the tab and moves the caret (clamped).
- [ ] **Step 4:** Run — PASS; manual: second `ferrite x.rs:40` focuses running window at line 40.
- [ ] **Step 5:** Commit `feat(cli): forward line and column`.

### Task 10: `--wait`

**Files:** Modify `src/main.rs` (`Cli.wait`), `src/single_instance.rs`, tab-close hook in `src/app/`. Test: unit tests + one integration test.

**Interfaces:** Protocol line `WAIT\t<path>` after the target line; primary replies `OK\n` immediately, then `CLOSED\n` when that tab closes. Exit code: `0` closed normally, `1` connection lost before `CLOSED` (primary crash/kill).

- [ ] **Step 1:** Tests `wait_returns_after_closed_message`, `wait_exits_1_when_primary_dies`, `wait_without_primary_runs_own_window_and_exits_on_close`, `closing_window_via_x_sends_closed`, `two_waiters_on_different_files_independent`.
- [ ] **Step 2:** Run — FAIL.
- [ ] **Step 3:** Implement waiter registry (path → streams) in the primary; fire `CLOSED` on tab close, window close, and app exit. Use a read timeout/keepalive so a dead primary is detected.
- [ ] **Step 4:** Run — PASS; manual: `GIT_EDITOR="ferrite --wait" git commit` blocks until tab closed, then commits.
- [ ] **Step 5:** Commit `feat(cli): --wait`.

### Task 11: Git editor mode

**Files:** Modify `src/devtools/cli_target.rs`, editor setup in `src/app/` (ruler, session exclusion). Test: unit tests.

**Interfaces — Produces:** `pub enum GitEditorKind { Commit, Merge, Tag, RebaseTodo, PatchEdit }`; `pub fn detect_git_editor_file(path: &Path) -> Option<GitEditorKind>` (match `COMMIT_EDITMSG`, `MERGE_MSG`, `TAG_EDITMSG`, `git-rebase-todo`, `addp-hunk-edit.diff` inside a `.git` dir).

- [ ] **Step 1:** Tests `commit_editmsg_detected`, `same_name_outside_dot_git_not_detected`, `rebase_todo_detected`, `git_editor_file_implies_wait`.
- [ ] **Step 2:** Run — FAIL.
- [ ] **Step 3:** Implement; for detected files: plain-text view, ruler at `git_editor_ruler`, skip session restore and autosave recovery, imply `--wait`.
- [ ] **Step 4:** Run — PASS.
- [ ] **Step 5:** Commit `feat(cli): git editor mode`.

### Task 12: `--diff` and stdin

**Files:** Modify `src/main.rs`, `src/devtools/diff.rs` (text fallback), new `src/devtools/text_diff.rs` only if `similar` approved. Test: unit tests.

**Interfaces:** `ferrite --diff <a> <b>`: both structured types → structured diff (T7); otherwise line diff if approved, else exit 2 with a clear message. `ferrite -` reads stdin to an untitled tab (read capped at 64 MB, UTF-8 lossy with a note).

- [ ] **Step 1:** Tests `diff_two_json_files_opens_structured`, `diff_mismatched_types_message`, `stdin_dash_creates_untitled_buffer`, `stdin_over_cap_is_truncated_with_notice`.
- [ ] **Step 2:** Run — FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run — PASS.
- [ ] **Step 5:** Commit `feat(cli): --diff and stdin`.

### Task 13: Phase 2 release docs

- [ ] **Step 1:** Update `docs/cli.md` (new args, exit codes, protocol note), add "Use Ferrite as $EDITOR" to `docs/devtools.md` (bash/zsh/PowerShell, `git config core.editor`, crontab, per-OS paths), README, CHANGELOG, credits, audit note `docs/technical/devtools-audit-phase2.md`.
- [ ] **Step 2:** Commit `docs: developer tools phase 2`.

---

## Self-review

- **Spec coverage:** schema validation (T2–T4), query bar (T5), structured diff (T7), format/convert (T6), CLI `file:line`, `--wait`, git editor, `--diff`, stdin, docs (T8–T13). Bundled schemas, opt-in catalog fetch: catalog fetch UI **not planned** — spec says opt-in; deferred to a follow-up task if Sean wants it.
- **Not covered here:** Phase 3 (B), LSP.
- **Open risks:** SchemaStore download needs network (T3); `--wait` IPC is the hardest piece (T10); `toml` drops comments on convert (reported as lossy, T6).
