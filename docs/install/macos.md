# macOS installation and Gatekeeper (v0.3.0)

GitHub release builds of Forgepad for macOS ship as a proper `Forgepad.app` bundle (see [`macos-app-bundle-ci.md`](../technical/platform/macos-app-bundle-ci.md)). **v0.3.0** artifacts from CI are **not** signed with an Apple Developer ID and are **not** notarized. That matches how many open-source projects ship macOS binaries today, but **Gatekeeper**—especially on **macOS 15.x (Sequoia), including 15.6+**—may block or warn on first launch because the system treats the download as untrusted until you explicitly allow it.

GitHub CI builds are not Developer ID signed or notarized. Use the workarounds below, build from source, or install via Homebrew. Tracked: [GitHub issue #130](https://github.com/OlaProeis/Ferrite/issues/130).

---

## Why this happens

1. **CI-built `.app` bundles are ad hoc–signed or unsigned** for GitHub Releases—they do not use a paid Developer ID certificate or Apple’s notarization service.
2. **Downloaded archives get the quarantine flag** (`com.apple.quarantine`). Gatekeeper evaluates that flag together with code signatures; without Developer ID + notarization, macOS may refuse to open the app or show “damaged / unidentified developer” style messaging.

The app itself is the same open-source Forgepad you can [build from source](../building.md); the friction is **distribution policy**, not malware.

---

## Workarounds (pick one)

### 1. Terminal: remove the quarantine flag

```bash
xattr -dr com.apple.quarantine /Applications/Forgepad.app
# or, for the bare binary from a test build:
xattr -d com.apple.quarantine ./forgepad
```

(There is no Homebrew tap for Forgepad yet.)

### 2. Finder: Right-click → Open

1. Put `Forgepad.app` in **Applications** (or open it from wherever you copied it).
2. **Control-click** (or right-click) `Forgepad.app` → **Open**.
3. In the dialog, click **Open** again.

**Caveat:** On some **macOS 15.x** configurations this path is **not sufficient** by itself; if the app still refuses to launch, use **System Settings** or the **Terminal** option below.

### 3. System Settings → Privacy & Security

After a blocked launch attempt, open **System Settings → Privacy & Security**, scroll to the message about Forgepad, and use **Open Anyway** if Apple presents it.

### 4. Terminal: remove quarantine (reliable for local installs)

This removes the download quarantine extended attribute **recursively** from the app bundle. Adjust the path if you did not install under `/Applications`:

```bash
xattr -dr com.apple.quarantine /Applications/Forgepad.app
```

Then launch Forgepad normally (double-click or Spotlight).

**Path tip:** If the app lives elsewhere (e.g. `~/Applications/Forgepad.app`), substitute that path.

---

## Verify quarantine (optional)

```bash
xattr -l /Applications/Forgepad.app
```

If `com.apple.quarantine` is absent after `xattr -dr`, Gatekeeper should no longer treat the bundle as a freshly quarantined download.

---

## Opening files (Open With / double-click)

Forgepad registers markdown and related types so it can appear in Finder’s **Open With** menu. On macOS, paths arrive via Apple Events (not `argv`); Forgepad queues them and opens tabs in the focused document window.

**Cold launch** (Forgepad not running): double-click or Open With should open the file in a new tab after launch.

**Warm open** (Forgepad already running): Open With should add a tab in the last-focused window. If macOS launches a second process, the single-instance protocol forwards paths to the running copy.

### If Open With does not work

Forgepad uses a best-effort workaround for [winit 0.30](https://github.com/rust-windowing/winit/issues/1751) (no public app-delegate hook). If Finder shows an error or the app opens empty, use one of these:

```bash
# Terminal — open with the app bundle
open -a Forgepad ~/Documents/notes.md

# Or invoke the binary directly (adjust path if not in /Applications)
/Applications/Forgepad.app/Contents/MacOS/forgepad ~/Documents/notes.md
```

AppleScript:

```applescript
tell application "Forgepad" to open POSIX file "/Users/you/Documents/notes.md"
```

Technical detail and upstream tracker: [`macos-open-with.md`](../technical/platform/macos-open-with.md). Tracked: [GitHub issue #154](https://github.com/OlaProeis/Ferrite/issues/154).

---

## Related links

- [GitHub #130 — Gatekeeper / macOS 15.x](https://github.com/OlaProeis/Ferrite/issues/130)
- [Roadmap](../../ROADMAP.md)
- [Building from source](../building.md)
- [macOS `.app` CI packaging](../technical/platform/macos-app-bundle-ci.md)
- [macOS Open With path reception](../technical/platform/macos-open-with.md) — Apple Events, tab routing, CLI escape hatch ([#154](https://github.com/OlaProeis/Ferrite/issues/154))
