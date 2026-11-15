//! Developer-tools commands: Problems panel toggle, sort/minify, format
//! conversion and comparison against the last commit.

use super::FerriteApp;
use crate::config::ShortcutCommand;
use crate::devtools::cli_target::OpenTarget;
use crate::devtools::convert::{convert, reformat, ReformatOpts};
use crate::devtools::diff::{diff_values, render_report, render_text_diff};
use crate::devtools::value::parse_to_value;
use crate::markdown::tree_viewer::StructuredFileType;
use crate::single_instance::WaitHandle;
use eframe::egui;
use rust_i18n::t;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// How long a waited file may stay unopened before the wait is released
/// (the file could not be opened at all).
const WAIT_OPEN_GRACE: Duration = Duration::from_secs(5);

type Changes = std::sync::Arc<Vec<Option<crate::devtools::linechanges::LineChange>>>;

/// Gutter change bars for the active tab (diff against git HEAD).
#[derive(Default)]
pub(crate) struct GitGutter {
    tab: Option<(usize, PathBuf)>,
    head: Option<String>,
    head_loaded: Option<Instant>,
    version: Option<u64>,
    computed: Option<Instant>,
    pub(crate) changes: Option<Changes>,
}

/// How often HEAD is re-read (commits made outside Ferrite).
const HEAD_REFRESH: Duration = Duration::from_secs(5);
/// Minimum time between diffs while typing.
const DIFF_INTERVAL: Duration = Duration::from_millis(300);

fn load_head(path: &Path) -> Option<String> {
    let repo = git2::Repository::discover(path).ok()?;
    let root = repo.workdir()?.to_path_buf();
    let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let root_canon = root.canonicalize().unwrap_or(root.clone());
    let rel = canon
        .strip_prefix(&root_canon)
        .or_else(|_| path.strip_prefix(&root))
        .ok()?
        .to_path_buf();
    crate::vcs::head_blob(&root, &rel).ok().flatten()
}

/// Files a `--wait` caller is blocked on.
pub(crate) struct WaitGroup {
    paths: Vec<PathBuf>,
    /// Connection of a forwarding secondary; `None` when this process was
    /// itself started with `--wait`.
    handle: Option<WaitHandle>,
    /// Close the window when done (this process was started for the wait).
    quit_when_done: bool,
    /// Seen open at least once.
    armed: bool,
    registered: Instant,
}

impl WaitGroup {
    /// Update from the currently open paths; true when the waiter is released
    /// (its files were open and are now all closed, or never opened in time).
    fn step(&mut self, open: &[PathBuf]) -> bool {
        let any_open = self.paths.iter().any(|p| open.contains(p));
        self.armed |= any_open;
        let gave_up = !self.armed && self.registered.elapsed() > WAIT_OPEN_GRACE;
        (self.armed && !any_open) || gave_up
    }
}

fn normalized(p: &Path) -> PathBuf {
    crate::path_utils::normalize_path(p.canonicalize().unwrap_or_else(|_| p.to_path_buf()))
}

fn ext_for(ft: StructuredFileType) -> &'static str {
    match ft {
        StructuredFileType::Json => "json",
        StructuredFileType::Yaml => "yaml",
        StructuredFileType::Toml => "toml",
    }
}

/// Sibling path with the new extension; never overwrites an existing file.
fn converted_path(src: &Path, to: StructuredFileType) -> PathBuf {
    let stem = src
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("converted");
    let dir = src.parent().unwrap_or(Path::new("."));
    let ext = ext_for(to);
    let mut candidate = dir.join(format!("{stem}.{ext}"));
    let mut n = 0;
    while candidate.exists() {
        n += 1;
        let tag = if n == 1 {
            "-converted".to_string()
        } else {
            format!("-converted{n}")
        };
        candidate = dir.join(format!("{stem}{tag}.{ext}"));
    }
    candidate
}

/// 1-based (line, col) → (clamped 1-based line, char index). A line past the
/// end clamps to the last line; a column past the line end clamps to its end.
fn target_position(text: &str, line: u32, col: Option<u32>) -> (usize, usize) {
    let lines: Vec<&str> = text.split('\n').collect();
    let idx = (line.max(1) as usize - 1).min(lines.len() - 1);
    let start: usize = lines[..idx].iter().map(|l| l.chars().count() + 1).sum();
    let len = lines[idx].trim_end_matches('\r').chars().count();
    let offset = col.map_or(0, |c| c.max(1) as usize - 1).min(len);
    (idx + 1, start + offset)
}

#[cfg(test)]
fn target_char_index(text: &str, line: u32, col: Option<u32>) -> usize {
    target_position(text, line, col).1
}

impl FerriteApp {
    fn devtools_toast(&mut self, msg: String) {
        let time = self.get_app_time();
        self.state.show_toast(msg, time, 3.0);
    }

    /// Active tab as (path, structured type, content).
    fn devtools_active_doc(&self) -> Option<(PathBuf, StructuredFileType, String)> {
        let tab = self.state.active_tab()?;
        let path = tab.path.clone()?;
        let ft = StructuredFileType::from_extension(path.extension()?.to_str()?)?;
        Some((path, ft, tab.content.clone()))
    }

    pub(crate) fn handle_devtools_command(&mut self, ctx: &egui::Context, cmd: ShortcutCommand) {
        match cmd {
            ShortcutCommand::ToggleProblems => {
                self.problems_panel.visible = !self.problems_panel.visible;
            }
            ShortcutCommand::FormatDocument => self.handle_format_structured_document(),
            ShortcutCommand::SortKeys | ShortcutCommand::MinifyJson => {
                let Some((_, ft, content)) = self.devtools_active_doc() else {
                    return self.devtools_toast(t!("notification.not_structured").to_string());
                };
                let minify = cmd == ShortcutCommand::MinifyJson;
                if minify && ft != StructuredFileType::Json {
                    return self
                        .devtools_toast(t!("devtools.convert.minify_json_only").to_string());
                }
                let opts = ReformatOpts {
                    sort_keys: !minify,
                    minify,
                };
                match reformat(&content, ft, opts) {
                    Ok(out) => {
                        if let Some(tab) = self.state.active_tab_mut() {
                            tab.set_content(out.text);
                        }
                        if !out.lossy.is_empty() {
                            self.devtools_toast(
                                t!("devtools.convert.lost", what = out.lossy.join(", "))
                                    .to_string(),
                            );
                        }
                    }
                    Err(e) => {
                        self.devtools_toast(t!("devtools.convert.failed", error = e).to_string())
                    }
                }
            }
            ShortcutCommand::ConvertToJson
            | ShortcutCommand::ConvertToYaml
            | ShortcutCommand::ConvertToToml => {
                let to = match cmd {
                    ShortcutCommand::ConvertToJson => StructuredFileType::Json,
                    ShortcutCommand::ConvertToYaml => StructuredFileType::Yaml,
                    _ => StructuredFileType::Toml,
                };
                let Some((path, from, content)) = self.devtools_active_doc() else {
                    return self.devtools_toast(t!("notification.not_structured").to_string());
                };
                if from == to {
                    return;
                }
                match convert(&content, from, to) {
                    Ok(out) => {
                        let dest = converted_path(&path, to);
                        match std::fs::write(&dest, out.text) {
                            Ok(()) => {
                                let mut msg =
                                    t!("devtools.convert.saved", path = dest.display().to_string())
                                        .to_string();
                                if !out.lossy.is_empty() {
                                    msg.push_str(&format!(
                                        " ({})",
                                        t!("devtools.convert.lost", what = out.lossy.join(", "))
                                    ));
                                }
                                self.devtools_toast(msg);
                                self.open_os_paths_in_focused_window(ctx, vec![dest], "convert");
                            }
                            Err(e) => self.devtools_toast(
                                t!("devtools.convert.failed", error = e.to_string()).to_string(),
                            ),
                        }
                    }
                    Err(e) => {
                        self.devtools_toast(t!("devtools.convert.failed", error = e).to_string())
                    }
                }
            }
            ShortcutCommand::CompareWithHead => self.compare_active_with_head(ctx),
            ShortcutCommand::NewReadme => {
                let name = self
                    .state
                    .workspace_root()
                    .and_then(|r| r.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("Project")
                    .to_string();
                self.open_untitled_with(crate::devtools::templates::readme(&name));
            }
            ShortcutCommand::NewChangelog => {
                self.open_untitled_with(crate::devtools::templates::changelog());
            }
            ShortcutCommand::NewAdr => self.new_adr(ctx),
            ShortcutCommand::CheckWorkspaceLinks => self.start_link_report(),
            _ => {}
        }
    }

    /// Compare the active tab (including unsaved edits) with `HEAD`; the report
    /// opens as a normal tab.
    fn compare_active_with_head(&mut self, ctx: &egui::Context) {
        let Some(tab) = self.state.active_tab() else {
            return;
        };
        let Some(path) = tab.path.clone() else { return };
        let content = tab.content.clone();
        let Ok(repo) = git2::Repository::discover(&path) else {
            return self.devtools_toast(t!("devtools.diff.no_head").to_string());
        };
        let Some(root) = repo.workdir().map(Path::to_path_buf) else {
            return self.devtools_toast(t!("devtools.diff.no_head").to_string());
        };
        let rel = path.strip_prefix(&root).unwrap_or(&path).to_path_buf();
        let head = match crate::vcs::head_blob(&root, &rel) {
            Ok(Some(h)) => h,
            Ok(None) => return self.devtools_toast(t!("devtools.diff.no_head").to_string()),
            Err(e) => {
                return self.devtools_toast(t!("devtools.convert.failed", error = e).to_string())
            }
        };
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
        let ft = path
            .extension()
            .and_then(|e| e.to_str())
            .and_then(StructuredFileType::from_extension);
        let structured = ft.and_then(|ft| {
            let a = parse_to_value(&head, ft).ok()?;
            let b = parse_to_value(&content, ft).ok()?;
            Some(render_report(
                &format!("HEAD:{name}"),
                name,
                &diff_values(&a, &b),
            ))
        });
        let report = structured
            .unwrap_or_else(|| render_text_diff(&format!("HEAD:{name}"), name, &head, &content));
        let Ok(file) = crate::devtools::scratch::write_new("diff", name, "diff", report.as_bytes())
        else {
            return self
                .devtools_toast(t!("devtools.convert.failed", error = "temp file").to_string());
        };
        self.open_os_paths_in_focused_window(ctx, vec![file], "compare");
    }

    /// Refresh gutter change bars for the active tab. Call once per frame.
    pub(crate) fn update_git_gutter(&mut self, ctx: &egui::Context) {
        let enabled = self.state.settings.devtools.git_gutter;
        let active = self
            .state
            .active_tab()
            .and_then(|t| t.path.clone().map(|p| (t.id, p, t.content_version())));
        let (Some((id, path, version)), true) = (active, enabled) else {
            self.git_gutter = GitGutter::default();
            return;
        };
        let g = &mut self.git_gutter;
        let now = Instant::now();
        let same_tab = g.tab.as_ref() == Some(&(id, path.clone()));
        if !same_tab
            || g.head_loaded
                .is_none_or(|t| now.duration_since(t) >= HEAD_REFRESH)
        {
            let head = load_head(&path);
            if !same_tab || head != g.head {
                g.version = None; // force a re-diff
            }
            g.head = head;
            g.head_loaded = Some(now);
            g.tab = Some((id, path));
        }
        if g.head.is_none() {
            g.changes = None;
            return;
        }
        if g.version == Some(version) {
            return;
        }
        if g.computed
            .is_some_and(|t| now.duration_since(t) < DIFF_INTERVAL)
        {
            ctx.request_repaint_after(DIFF_INTERVAL);
            return;
        }
        let Some(content) = self.state.active_tab().map(|t| t.content.clone()) else {
            return;
        };
        let g = &mut self.git_gutter;
        if let Some(head) = &g.head {
            g.changes = Some(std::sync::Arc::new(
                crate::devtools::linechanges::line_changes(head, &content),
            ));
        }
        g.version = Some(version);
        g.computed = Some(now);
    }

    /// New untitled tab holding `text` (unsaved, so the user picks where to save).
    fn open_untitled_with(&mut self, text: String) {
        self.state.new_tab();
        if let Some(tab) = self.state.active_tab_mut() {
            tab.set_content(text);
        }
    }

    /// In a workspace: create the next numbered ADR file and open it.
    /// Without a workspace: an untitled ADR tab.
    fn new_adr(&mut self, ctx: &egui::Context) {
        use crate::devtools::templates::{adr, adr_dir, next_adr_path, today};
        let title = "New decision";
        let Some(root) = self.state.workspace_root().cloned() else {
            return self.open_untitled_with(adr(1, title, &today()));
        };
        let dir = adr_dir(&root);
        let (n, path) = next_adr_path(&dir);
        let written = std::fs::create_dir_all(&dir)
            .and_then(|_| std::fs::write(&path, adr(n, title, &today())));
        match written {
            Ok(()) => self.open_os_paths_in_focused_window(ctx, vec![path], "adr"),
            Err(e) => self
                .devtools_toast(t!("devtools.templates.failed", error = e.to_string()).to_string()),
        }
    }

    /// Check every Markdown file in the workspace on a worker thread.
    fn start_link_report(&mut self) {
        if self.link_report_rx.is_some() {
            return self.devtools_toast(t!("devtools.links.running").to_string());
        }
        let Some(root) = self.state.workspace_root().cloned() else {
            return self.devtools_toast(t!("devtools.links.no_workspace").to_string());
        };
        let (tx, rx) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("devtools-links".into())
            .spawn(move || {
                let report = crate::devtools::links::workspace_report(&root, 5_000);
                let res = crate::devtools::scratch::write_new(
                    "links",
                    "link-check",
                    "md",
                    report.as_bytes(),
                )
                .map_err(|e| e.to_string());
                let _ = tx.send(res);
            });
        match spawned {
            Ok(_) => {
                self.link_report_rx = Some(rx);
                self.devtools_toast(t!("devtools.links.started").to_string());
            }
            Err(e) => {
                self.devtools_toast(t!("devtools.links.failed", error = e.to_string()).to_string())
            }
        }
    }

    /// Open the workspace link report once its job finishes. Call every frame.
    pub(crate) fn poll_link_report(&mut self, ctx: &egui::Context) {
        let Some(rx) = &self.link_report_rx else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(file)) => {
                self.link_report_rx = None;
                self.open_os_paths_in_focused_window(ctx, vec![file], "link-check");
            }
            Ok(Err(e)) => {
                self.link_report_rx = None;
                self.devtools_toast(t!("devtools.links.failed", error = e).to_string());
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                ctx.request_repaint_after(Duration::from_millis(200));
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => self.link_report_rx = None,
        }
    }

    /// Start tracking files a `--wait` caller is blocked on.
    pub(crate) fn register_wait(
        &mut self,
        paths: &[PathBuf],
        handle: Option<WaitHandle>,
        quit_when_done: bool,
    ) {
        self.wait_groups.push(WaitGroup {
            paths: paths.iter().map(|p| normalized(p)).collect(),
            handle,
            quit_when_done,
            armed: false,
            registered: Instant::now(),
        });
    }

    /// Release `--wait` callers whose files are all closed. Call every frame.
    pub(crate) fn poll_wait_groups(&mut self, ctx: &egui::Context) {
        if self.wait_groups.is_empty() {
            return;
        }
        let open: Vec<PathBuf> = self
            .state
            .tabs()
            .iter()
            .filter_map(|t| t.path.as_deref().map(normalized))
            .collect();
        let mut finished = Vec::new();
        for (i, g) in self.wait_groups.iter_mut().enumerate() {
            if g.step(&open) {
                finished.push(i);
            }
        }
        for i in finished.into_iter().rev() {
            let g = self.wait_groups.remove(i);
            if let Some(h) = &g.handle {
                h.notify_closed();
            }
            if g.quit_when_done {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        if !self.wait_groups.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
    }

    /// Move the caret to a CLI/IPC target's line and column.
    pub(crate) fn jump_to_target(&mut self, target: &OpenTarget) {
        let Some(line) = target.line else { return };
        let on_target = self
            .state
            .active_tab()
            .and_then(|t| t.path.as_deref())
            .map(|p| normalized(p) == normalized(&target.path))
            .unwrap_or(false);
        if !on_target {
            return;
        }
        let Some(tab) = self.state.active_tab() else {
            return;
        };
        let (clamped_line, idx) = target_position(&tab.content, line, target.col);
        // Scrolls to the line; the caret is then placed exactly below.
        self.handle_go_to_line(clamped_line);
        if let Some(tab) = self.state.active_tab_mut() {
            tab.cursors.set_single(crate::state::Selection::cursor(idx));
            tab.sync_cursor_from_primary();
        }
    }

    /// After startup paths are opened: jump to positions and honour `--wait`.
    pub(crate) fn apply_cli_targets(&mut self, targets: &[OpenTarget], wait: bool) {
        if let Some(t) = targets.iter().rev().find(|t| t.line.is_some()) {
            self.jump_to_target(t);
        }
        if wait && !targets.is_empty() {
            let paths: Vec<PathBuf> = targets.iter().map(|t| t.path.clone()).collect();
            self.register_wait(&paths, None, true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group(path: &str) -> WaitGroup {
        WaitGroup {
            paths: vec![PathBuf::from(path)],
            handle: None,
            quit_when_done: false,
            armed: false,
            registered: Instant::now(),
        }
    }

    #[test]
    fn two_waiters_on_different_files_independent() {
        let (a, b) = (PathBuf::from("/r/a.md"), PathBuf::from("/r/b.md"));
        let mut ga = group("/r/a.md");
        let mut gb = group("/r/b.md");
        assert!(!ga.step(&[a.clone(), b.clone()]) && !gb.step(&[a.clone(), b.clone()]));
        // Close a only: only its waiter is released.
        assert!(ga.step(std::slice::from_ref(&b)));
        assert!(!gb.step(std::slice::from_ref(&b)));
        assert!(gb.step(&[]));
    }

    #[test]
    fn waiter_not_released_before_file_first_opens() {
        let mut g = group("/r/slow.md");
        assert!(!g.step(&[]), "still inside the open grace period");
        g.registered = Instant::now() - WAIT_OPEN_GRACE - Duration::from_millis(1);
        assert!(
            g.step(&[]),
            "never opened: give up so the caller is not stuck"
        );
    }

    #[test]
    fn line_past_eof_clamps_to_last_line() {
        let text = "one\ntwo\nthree";
        // Line 40 of a 3-line file: start of the last line.
        assert_eq!(target_char_index(text, 40, None), 8);
        assert_eq!(target_char_index(text, 40, Some(3)), 10);
    }

    #[test]
    fn column_clamps_to_line_end_and_ignores_crlf() {
        let text = "ab\r\ncdef\r\n";
        assert_eq!(target_char_index(text, 1, Some(99)), 2);
        assert_eq!(target_char_index(text, 2, Some(3)), 6);
        assert_eq!(target_char_index(text, 2, None), 4);
        assert_eq!(target_char_index("", 5, Some(5)), 0);
    }
}
