# Ferrite Command Line Interface

Ferrite supports command-line arguments for opening files and directories directly from the terminal.

## Usage

```bash
ferrite [OPTIONS] [PATH]...
```

## Arguments

### `[PATH]...` - Files or directories to open

Pass one or more file paths to open them as tabs, or a directory to open it as a workspace.

**Examples:**

```bash
# Open a single file
ferrite README.md

# Open multiple files as tabs
ferrite file1.md file2.md notes.md

# Open current directory as workspace
ferrite .

# Open a specific directory as workspace
ferrite ~/projects/my-project

# Mix of files (first file gets focus)
ferrite README.md CHANGELOG.md docs/getting-started.md
```

## Options

### `-h, --help`

Print help information and exit.

```bash
ferrite --help
ferrite -h
```

### `-V, --version`

Print version information and exit.

```bash
ferrite --version
ferrite -V
```

### `--log-level <LEVEL>`

Set the log level for debugging. This overrides the `log_level` setting in `config.json`.

Valid values: `debug`, `info`, `warn`, `error`, `off`

```bash
# Enable debug logging
ferrite --log-level debug

# Show only errors
ferrite --log-level error

# Disable all logging
ferrite --log-level off

# Combine with file arguments
ferrite --log-level debug README.md
```

**Log Level Precedence:**
1. CLI flag (`--log-level`)
2. Config file (`config.json`)
3. Built-in default (`warn`)

### `--new-instance`

*(v0.3.1)* Start a new Ferrite process even if another instance is already running. Without this flag, a second launch forwards its paths to the running instance and exits (unless **Settings → Files → Allow multiple Ferrite windows/instances** is on).

```bash
# Open a file in a separate Ferrite process
ferrite --new-instance notes.md
```

A `--new-instance` process does not write the single-instance lock/pid files and keeps its session and config writes separate from the primary instance. See [`technical/platform/single-instance.md`](technical/platform/single-instance.md) and [`technical/files/session-persistence.md`](technical/files/session-persistence.md).

### `PATH:LINE[:COL]` and `--goto <PATH:LINE[:COL]>`

*(Developer tools)* Open a file at a 1-based line and column.

```bash
ferrite src/main.rs:120
ferrite src/main.rs:120:9
ferrite --goto "C:\a b\x.rs:12:3"
```

Only trailing numbers ≥ 1 are treated as a position; `x.rs:abc` and `x.rs:0`
are file names. A Windows drive letter is never split. If the whole argument
names an existing file it is opened unchanged. A line past the end goes to the
last line; a column past the end of the line goes to its end. When Ferrite is
already running, the position is forwarded to that window.

### `-` (standard input)

Read standard input (up to 64 MB; invalid UTF-8 replaced) into a new file in
Ferrite's per-user cache folder and open it.

```bash
git log -p | ferrite -
```

### `-w, --wait`

Block until every tab opened by this command is closed (tab, window or app
close). Use it for `$EDITOR`, `$GIT_EDITOR` and `git config core.editor`.
Git's own message files (`COMMIT_EDITMSG`, `MERGE_MSG`, `TAG_EDITMSG`,
`git-rebase-todo`, `addp-hunk-edit.diff` inside `.git`) imply `--wait`.

```bash
git config --global core.editor "ferrite --wait"
```

Setup per shell and OS: [`devtools.md`](devtools.md#use-ferrite-as-editor).

### `--diff <OLD> <NEW>`

Compare two files and open the report as a tab. JSON, YAML and TOML pairs
(any mix) are compared by value — key order and formatting are ignored. Other
files get a unified line diff.

```bash
ferrite --diff old/config.yaml new/config.json
```

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Normal exit; with `--wait`, the files were closed |
| `1` | `--wait` only: the running Ferrite ended (crash or kill) before the files were closed |
| `2` | Bad input: unreadable stdin, or a `--diff` file is missing or does not parse |

### Single-instance protocol note

Each forwarded target is one line: a plain path (as older Ferrite versions
send), or `T<TAB>line<TAB>col<TAB>path` when a position is given. A `W` line
asks the running instance to reply `CLOSED` once the files are closed; it
replies `OK` first. Older senders and receivers keep working with plain paths.

## Behavior

### Opening Files

- Each file path is opened as a separate tab
- The first successfully opened file gets focus
- Non-existent files are logged as warnings and skipped
- Relative paths are resolved to absolute paths

### Opening Directories

- A single directory can be passed to open it as a workspace
- The workspace mode enables:
  - File tree navigation
  - Quick switcher (Ctrl+P)
  - Search in files (Ctrl+Shift+F)
  - Git status indicators
- If multiple directories are provided, only the first one is used

### Mixed Arguments

When both files and a directory are provided:

- The directory becomes the workspace root
- Files are opened as tabs within that workspace context

### Error Handling

- Non-existent paths produce a warning in the log but don't crash the app
- Permission errors are logged and the file is skipped
- Invalid paths (neither file nor directory) are skipped with a warning

## Examples

### Basic Usage

```bash
# Start with empty editor
ferrite

# Edit a markdown file
ferrite ~/Documents/notes.md

# Open project as workspace
ferrite ~/projects/my-rust-project
```

### Shell Integration

```bash
# Open file from pipe (not directly supported, use temp file)
# Instead, use shell redirection:
cat file.md | ferrite /dev/stdin  # Linux/macOS

# Edit git commit message
GIT_EDITOR="ferrite" git commit

# Open all markdown files in current directory
ferrite *.md
```

### Scripting

```bash
# Check version in scripts
VERSION=$(ferrite --version)
echo "Using $VERSION"

# Conditional launch
if [ -f "README.md" ]; then
    ferrite README.md
fi
```

## Environment Variables

Ferrite reads log configuration from `config.json` and supports CLI overrides. The `RUST_LOG` environment variable is no longer used; use `--log-level` instead.

| Method | Example | Priority |
|--------|---------|----------|
| CLI flag | `ferrite --log-level debug` | Highest |
| Config file | `"log_level": "debug"` in config.json | Medium |
| Built-in default | `warn` | Lowest |

## Platform Notes

### Windows

```powershell
# PowerShell
.\ferrite.exe README.md

# CMD
ferrite.exe README.md

# Open current directory
ferrite.exe .
```

### Linux/macOS

```bash
# Standard usage
./ferrite README.md

# If installed to PATH
ferrite README.md
```

## Related

- [Building from Source](building.md)
- [Configuration](../README.md#configuration)
