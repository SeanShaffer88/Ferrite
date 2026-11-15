# Ferrite for Developers — Design

Status: DRAFT, awaiting approval · Date: 2026-11-14 · Size: project (3 phases)

## Intent and why

Make Ferrite the light, fast editor developers reach for when they touch
config and docs files — the work VS Code is heavy for. Sean did not state a
detailed "why"; this reading is Claude's proposal from the repo and the
chosen order **A + C, then B**. Correct it if wrong.

Ferrite today: Rust/egui editor, MD/JSON/YAML/TOML, tree viewer, CSV viewer,
workspace + quick switcher, terminal, git (git2), Code Run, frontmatter
card, single-instance, basic CLI (`ferrite [PATH]...`, `--new-instance`).
LSP exists behind the `lsp` Cargo feature, deferred to v0.3.2+.

## Goals

1. **Phase 1 (A) — Config/data editor.** Schema validation, query bar,
   structured diff, format/convert.
2. **Phase 2 (C) — `$EDITOR` replacement.** CLI and git-editor behavior.
3. **Phase 3 (B) — Docs workbench.** README/ADR/changelog tooling.
4. Stay fast, light, offline-first (no network calls by default).

## Non-goals

- Becoming an IDE: no debugger, no bundled language servers, no plugins.
- Shipping LSP in this effort (stays behind flag; see Constraints).
- New mockups or demo content in the repo.

## Anti-goals (failure even if it "works")

- Slower startup or higher idle memory for non-dev users.
- Features that only work with network access.
- A diverged fork that can't merge upstream OlaProeis/Ferrite.

## Constraints

- Rust 1.92+, egui/eframe. Existing crates reused where possible
  (serde_json, comrak, git2, portable-pty).
- Offline-first: schemas come from the file's `$schema`, local paths, or a
  small bundled set. Fetching a schema catalog is opt-in only.
- Diagnostics UI built generically (source: schema | lint | later LSP) so LSP
  can plug in later without rework.
- New code in new modules (`src/devtools/…`) behind settings toggles, to keep
  upstream merges clean. Existing files touched minimally.
- Each phase ships with updated CHANGELOG, README section, docs page, an
  audit note, and credits (upstream OlaProeis, MIT, plus new crates).
- Every user-visible string goes through `locales/`.
- Tests: unit tests per feature; no network in tests.

## Approaches considered

| | Path | Trade-off |
|---|---|---|
| A | Config/data editor | Most new code; strongest hook |
| B | Docs workbench | Closest to existing; many rivals |
| C | `$EDITOR` replacement | Cheap; weak alone |

**Picked: A + C, then B** (Sean). Reason: A is the differentiator, C is cheap
glue that puts Ferrite in the daily dev loop, B builds on what exists.

## Design

### Phase 1 — Config/data editor (A)

- **Schema validation.** JSON Schema for JSON, YAML, TOML (parsed to a common
  value tree). Schema source order: `$schema` / `# yaml-language-server:
  $schema=` / `#:schema` comment → workspace mapping in settings → bundled
  set (Cargo.toml, package.json, tsconfig, GitHub Actions, docker-compose,
  pyproject). Errors shown as squiggles + a Problems panel with click-to-line.
- **Query bar.** JSONPath (jq-style syntax as stretch) over the current
  JSON/YAML/TOML doc; results list in the tree viewer; click jumps to node.
- **Structured diff.** Compare two files or file vs git HEAD by value tree,
  ignoring key order/format, shown side by side.
- **Format/convert.** Format, sort keys, minify; convert JSON⇄YAML⇄TOML with
  a warning when data does not round-trip (comments lost).

### Phase 2 — `$EDITOR` replacement (C)

- `ferrite path:line[:col]`, `--goto`, stdin via `-`.
- `--wait`: block until the tab closes (so `git commit`, `crontab -e` work).
- Git editor mode: detect `COMMIT_EDITMSG`, `git-rebase-todo`, `MERGE_MSG`;
  open minimal chrome with line-length ruler (72) and no autosave-to-session.
- `--diff a b` opens structured/text diff (reuses Phase 1 diff).
- Docs: how to set `$EDITOR`/`$GIT_EDITOR`/`core.editor` per OS.
- Builds on existing single-instance: second call forwards to the running
  window; `--wait` needs a return channel (builder to design).

### Phase 3 — Docs workbench (B)

- README/ADR/changelog templates (Keep a Changelog, MADR).
- Link checker for relative links and anchors in workspace Markdown.
- Frontmatter validation against a schema (reuses Phase 1 engine).
- Git-aware preview: show uncommitted changes in Markdown rendered view.
- Code-block lint/run extends existing Code Run.

## Decided by Sean

- Order: A + C first, then B.

## Claude's calls (check these)

- Keep the name "Ferrite"; no rebrand now.
- Keep MIT; credit upstream in README + credits file.
- New `src/devtools/` module tree; settings toggle per feature.
- JSONPath first, jq syntax later.
- Bundled schema set is small; catalog fetch opt-in.
- LSP stays out of scope.
- No separate builder-review subagent run (token budget); doc self-checked.

## Left to the builder

- Crate choices (JSONPath, JSON Schema, YAML/TOML value trees).
- `--wait` IPC with single-instance.
- Problems panel layout within the existing UI style.
- Exact bundled schema list and sizes.
