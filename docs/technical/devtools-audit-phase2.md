# Developer tools phase 2 — audit note

Branch `claude/developer-edition-design`. Plan:
`docs/superpowers/plans/2026-11-14-developer-edition.md`. Date: 2026-11-15.

## Verified (unit / socket tests, Windows 11)

- Target parsing (`devtools::cli_target`): plain paths, `:line`, `:line:col`,
  Windows drive paths with and without position (`C:\a b\x.rs:12:3`),
  non-numeric and zero suffixes kept in the name, existing file named with a
  colon wins, tab in path survives the wire format.
- Caret placement: a line past the end clamps to the last line; a column past
  the line end clamps; CRLF ignored (`app::devtools` tests).
- Single-instance protocol over real loopback sockets: legacy plain-path lines
  still decode; `OK` then `CLOSED` round trip; primary dropping the connection
  after `OK` → waiter reports interrupted (exit 1); `OK` and `CLOSED` arriving
  in one segment → normal close (bug fixed on this branch: two `BufReader`s
  lost the second line); a dropped `WaitHandle` sends `CLOSED` (app exit).
- Wait groups: two waiters on different files release independently; a file
  that never opens releases its waiter after a 5 s grace period.
- Git editor files: detected only inside a `.git` directory; imply `--wait`;
  excluded from session capture and recovery blobs.
- stdin cap (64 MB) and truncation flag; `--diff` structured vs text, missing
  file and parse errors.

## Not verified

- No interactive run: `GIT_EDITOR="ferrite --wait" git commit`, `crontab -e`,
  and a second `ferrite x.rs:40` focusing the running window were not run by
  hand. Plan's manual checks for Tasks 9 and 10 are still owed.
- Closing the window with the title-bar X while a forwarded waiter is blocked
  (covered only through the drop-on-exit path).
- `--wait` with no running instance (own window closes when the files close).
- The column guide's on-screen position (logic unit-tested, not looked at).
- macOS and Linux.

## Known limits

- A waiter whose file cannot be opened is released after 5 s with exit 0.
- `--diff` and stdin write temporary files under the OS temp directory
  (`ferrite-cli/`); they are not cleaned up automatically.
