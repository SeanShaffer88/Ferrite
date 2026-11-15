# Ferrite Developer Edition — Phase 3 (Docs workbench) Plan

**Goal:** Tools for the Markdown docs that live next to code: templates, a
link checker, and frontmatter validation.

**Status (2026-11-15):** T1–T5 built; build + clippy clean; not run or tested (see `docs/technical/devtools-audit-phase3.md`).

**Spec:** `docs/superpowers/specs/2026-11-14-developer-edition-design.md`
(Phase 3). Builds on Phase 1–2 (`2026-11-14-developer-edition.md`).

**Sean's direction (2026-11-15):** continue development, **skip tests**. Each
task is checked by `cargo build` + `cargo clippy` only; no new unit tests and
no test runs. This is a deliberate trade-off; the audit note says so.

## Scope decisions

| Spec item | Decision |
|---|---|
| README / ADR / changelog templates | **Build** (T1) |
| Link checker (relative links + anchors) | **Build** (T2, T3) |
| Frontmatter validation (Phase 1 engine) | **Build** (T4) |
| Git-aware preview (uncommitted changes in rendered view) | **Partly built:** gutter change bars vs `HEAD` in Raw and Split view. Marking changes inside the rendered (WYSIWYG) view is still deferred |
| Code-block lint/run (extend Code Run) | **Built (narrow):** syntax check of fenced json/yaml/toml blocks; "run" is the existing Code Run. Other languages need Sean's call |

## Global constraints (same as Phase 1–2)

Offline; new code in `src/devtools/`; strings via `locales/en.yaml`; no new
crates (comrak, serde_yaml, regex already present); heavy work off the UI
thread; no `unwrap()` in production paths; surgical edits to existing files.

## Tasks

### T1: Templates

`src/devtools/templates.rs`: `readme(name)`, `changelog()`, `adr(number,
title, date)` (MADR 3 minimal), `next_adr_path(root) -> PathBuf`
(`docs/adr/NNNN-new-decision.md`, next free number, never overwrites).
Commands: **New README from Template** (Ctrl+Shift+Alt+R), **New Changelog
from Template** (Ctrl+Shift+Alt+C) → untitled tab with content; **New ADR**
(Ctrl+Shift+Alt+A) → in a workspace, creates the next ADR file and opens it;
otherwise an untitled tab.

### T2: Link checker core

`src/devtools/links.rs`: `check_links(doc_path, content) -> Vec<Problem>`.
Walks comrak's AST for links and images (with source line/column). Checks
only relative targets: file exists (relative to the document, `%XX`
decoded); `#anchor` exists in the target Markdown file (or this document)
using comrak's GitHub-style anchorizer plus explicit `id=`/`name=` HTML
anchors. Skips URLs with a scheme, `mailto:`, `//host`, root-absolute `/x`.
Problems are `ProblemSource::Lint`, severity Warning.

### T3: Wire into Problems panel + workspace report

`ProblemsPanel`'s `DocInput.ft` becomes `DocKind { Structured(ft), Markdown }`.
Markdown jobs run `check_markdown` (links + frontmatter) on the worker.
Command **Check Links in Workspace** (Ctrl+Shift+Alt+L): walks workspace
`*.md` (skips `.git`, `target`, `node_modules`, hidden dirs; cap 5 000
files) on a thread and opens a report tab. Setting
`devtools.link_check: bool` (default true).

### T4: Frontmatter validation

`src/devtools/frontmatter.rs`: frontmatter YAML resolved via the Phase 1
order (in-frontmatter `# yaml-language-server: $schema=` comment, then
`schema_mappings` globs against the `.md` path) and validated; line numbers
shifted by the opening `---`. No bundled frontmatter schema.

### T5: Docs, build, clippy, commit, push

`docs/devtools.md` section, CHANGELOG, audit note
`docs/technical/devtools-audit-phase3.md`.
