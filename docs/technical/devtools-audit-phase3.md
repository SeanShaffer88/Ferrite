# Developer tools phase 3 (docs workbench) — audit note

Branch `claude/developer-edition-design`. Plan:
`docs/superpowers/plans/2026-11-15-developer-edition-phase3.md`. Date: 2026-11-15.

## How this phase was checked

Sean asked to **skip tests** for this phase. Verification was limited to:

- `cargo build --locked` — no errors.
- `cargo clippy --locked --all-targets` — passes; warning count at or below
  the existing baseline. `--all-targets` also confirms the existing test suite
  still compiles with the changed `DocInput` type.

**Not done:** no new unit tests, no test-suite run, no interactive run of the
app. Every behaviour below is unverified at runtime.

## What to check first

1. Link warnings land on the right line in a Markdown file with YAML
   frontmatter (frontmatter is blanked to newlines before parsing, so lines
   should match — but comrak's inline source positions are unverified).
2. Heading anchors match GitHub for headings with punctuation, emoji, inline
   code and duplicates (uses comrak's `Anchorizer`).
3. Links with `%20`, `?query`, Windows backslashes, and `<angle bracket>`
   targets.
4. Frontmatter problem lines are offset by one for the opening `---` — check
   a file with a BOM and with CRLF line endings.
5. **Check Links in Workspace** on a large folder: UI stays responsive; the
   report opens when done; running it twice shows "already running".
6. **New ADR** numbering when `docs/adr` holds `0001-x.md` … `0009-y.md` and
   non-ADR files; no file is overwritten.
7. Shortcuts Ctrl+Shift+Alt+R / C / A / L do not clash with other bindings
   (the existing shortcut-conflict test would catch this; it was not run).

## Known limits

- Root-absolute links (`/docs/x.md`) and links into other repos are skipped.
- Anchors defined only by non-heading syntax other than `id=` / `name=`
  HTML attributes and footnotes are not recognised.
- The workspace report is written to `<cache>/ferrite/generated/links/` and not
  cleaned up.
- Code-block lint (added after the first phase 3 commit) is likewise untested:
  check that the reported line is inside the fence for multi-line YAML errors.
- No bundled frontmatter schema.

## Deferred from the spec

- Git-aware preview (show uncommitted changes in the rendered view).
- Code-block lint/run extension (needs a decision on what "lint" means).
