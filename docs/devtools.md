# Developer tools

Tools for the config and data files developers edit every day, plus the
command-line behaviour needed to use Ferrite as `$EDITOR`. Everything works
offline. Credits and licenses: [`devtools-credits.md`](devtools-credits.md).

All settings live under `"devtools"` in `config.json`:

```json
"devtools": {
  "schema_validation": true,
  "query_bar": true,
  "schema_mappings": [["**/deploy/*.yaml", "schemas/deploy.json"]],
  "git_editor_ruler": 72
}
```

## Schema validation and the Problems panel

JSON, YAML and TOML files are checked against a JSON Schema while you type.
Checking runs off the UI thread after a short pause in typing, so large files
never freeze the editor.

**Which schema is used** (first match wins):

1. A reference in the file itself:
   - JSON: `"$schema": "./schema.json"`
   - YAML: `# yaml-language-server: $schema=./schema.json`
   - TOML or YAML: `#:schema ./schema.json` (in the first 10 lines)
2. `devtools.schema_mappings`: `[glob, schema path]` pairs. Relative schema
   paths resolve against the document's folder.
3. A bundled schema, by file name:

| File | Schema |
|---|---|
| `package.json` | npm package manifest |
| `tsconfig*.json` | TypeScript config |
| `Cargo.toml` | Cargo manifest |
| `pyproject.toml` | Python project |
| `.github/workflows/*.yml` / `.yaml` | GitHub Actions workflow |
| `docker-compose*.yml`, `compose*.yaml` | Compose file |

A schema given as an `http(s)://` URL is **never downloaded**; you get one
info line, "remote schema not fetched", instead. Download the schema and point
to the local copy. A missing or broken local schema also gives one info line.
Bundled schemas skip parts that refer to other online schemas (for example
`[tool.ruff]` in `pyproject.toml`); see the credits page for the list.

Without any schema, syntax errors are still reported.

**Problems panel.** Toggle with **Ctrl+Shift+Alt+P** or by clicking the
problem count in the status bar. Each row shows line:column, the message and
its source (`schema` or `lint`). Click a row to jump to it. Close it with ✕,
or Esc while the mouse is over the panel. Turn the feature off with `devtools.schema_validation: false`.

## Query bar

In the tree viewer of a JSON, YAML or TOML file, type a
[JSONPath (RFC 9535)](https://www.rfc-editor.org/rfc/rfc9535) expression:

```text
$.dependencies
$.jobs[*].runs-on
$..version
$.services[?@.image]
```

Results list each match's location and a short preview; click one to jump to
that node. Small files update as you type; files over 1 MB run on Enter.
Queries returning more than 10 000 matches are stopped with a message. Turn
the bar off with `devtools.query_bar: false`.

## Format, sort and convert

From the command palette (or shortcuts):

| Command | Shortcut | Notes |
|---|---|---|
| Format Document | Ctrl+Shift+Alt+F | Pretty-prints, keeping key order |
| Sort Keys | Ctrl+Shift+Alt+S | Recursive, stable |
| Minify JSON | Ctrl+Shift+Alt+M | JSON only |
| Convert to JSON / YAML / TOML | Ctrl+Shift+Alt+J / Y / T | Writes a new file next to the original |

Conversion never overwrites a file: `config.yaml` → `config.json`, or
`config-converted.json` if that exists. When the data does not survive the
trip, the notification says what was lost: YAML/TOML comments, `null` values
(TOML has none), or a non-table root (TOML needs one).

Edits made by Format, Sort Keys and Minify can be undone.

## Compare

**Compare with Last Commit** (Ctrl+Shift+Alt+H) diffs the current buffer,
including unsaved edits, against the file in git `HEAD`. JSON/YAML/TOML are
compared by value: key order and formatting do not count, and the report
lists each added, removed and changed path. Other files get a line diff. The
report opens as a new tab.

From the terminal: `ferrite --diff old.json new.json` (see below).

## Command line

```text
ferrite file.rs:40          open at line 40
ferrite file.rs:40:5        line 40, column 5
ferrite --goto file.rs:40   same, explicit
ferrite -                   read standard input into a new tab
ferrite --wait file         return only when the tab is closed
ferrite --diff a b          compare two files
```

- `path:line[:col]` only splits off numbers ≥ 1, so `x.rs:abc`, `x.rs:0` and
  Windows drive letters (`C:\a b\x.rs:12:3`) work as expected. If a file
  literally named `x:12` exists, it is opened as-is.
- A line past the end goes to the last line; a column past the line end goes
  to the end of the line.
- If Ferrite is already running, the file opens in that window, at the given
  position.
- `-` reads up to 64 MB of standard input (invalid UTF-8 is replaced) into a
  temporary file and opens it.

Full reference: [`cli.md`](cli.md).

## Use Ferrite as `$EDITOR`

`--wait` (`-w`) makes the command block until you close the tab(s) it opened,
which is what git, crontab and similar tools need. Closing the tab, the
window, or quitting Ferrite all count as closing. If Ferrite crashes or is
killed first, the waiting command exits with code **1**, so git aborts the
commit instead of using a half-written message.

Git's own files (`COMMIT_EDITMSG`, `MERGE_MSG`, `TAG_EDITMSG`,
`git-rebase-todo`, `addp-hunk-edit.diff`) imply `--wait`, get a column guide at
`devtools.git_editor_ruler` (default 72, `null` to hide), and are never
reopened by session restore.

**Git (all platforms):**

```bash
git config --global core.editor "ferrite --wait"
```

**bash / zsh** (`~/.bashrc` or `~/.zshrc`):

```bash
export EDITOR="ferrite --wait"
export VISUAL="$EDITOR"
```

**PowerShell** (`$PROFILE`):

```powershell
$env:EDITOR = "ferrite --wait"
```

**crontab:** with `EDITOR` set as above, `crontab -e` opens in Ferrite.

**Where the binary is:** if `ferrite` is not on `PATH`, use the full path:

| OS | Typical path |
|---|---|
| Windows (MSI) | `C:\Program Files\Ferrite\ferrite.exe` |
| macOS (DMG) | `/Applications/Ferrite.app/Contents/MacOS/ferrite` |
| Linux (.deb / .rpm) | `/usr/bin/ferrite` |

On Windows, quote paths with spaces for git:
`git config --global core.editor "'C:/Program Files/Ferrite/ferrite.exe' --wait"`.

## Turning things off

- `devtools.schema_validation: false` — no checking, Problems panel stays empty.
- `devtools.query_bar: false` — no query box in the tree viewer.
- `devtools.git_editor_ruler: null` — no column guide.
- The command-line features cost nothing unless you use them.
