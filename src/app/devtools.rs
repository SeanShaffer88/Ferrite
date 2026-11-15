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
        let dir = std::env::temp_dir().join("ferrite-diff");
        let file = dir.join(format!("{name}.diff"));
        if std::fs::create_dir_all(&dir)
            .and_then(|_| std::fs::write(&file, report))
            .is_err()
        {
            return self
                .devtools_toast(t!("devtools.convert.failed", error = "temp file").to_string());
        }
        self.open_os_paths_in_focused_window(ctx, vec![file], "compare");
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
            let any_open = g.paths.iter().any(|p| open.contains(p));
            g.armed |= any_open;
            let gave_up = !g.armed && g.registered.elapsed() > WAIT_OPEN_GRACE;
            if (g.armed && !any_open) || gave_up {
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
        self.handle_go_to_line(line as usize);
        if let Some(col) = target.col.filter(|c| *c > 1) {
            if let Some(tab) = self.state.active_tab_mut() {
                let line_idx = (line as usize).saturating_sub(1);
                let start = crate::app::helpers::line_col_to_char_index(&tab.content, line_idx, 0);
                let len = tab
                    .content
                    .lines()
                    .nth(line_idx)
                    .map(|l| l.trim_end_matches('\r').chars().count())
                    .unwrap_or(0);
                let idx = start + (col as usize - 1).min(len);
                tab.cursors.set_single(crate::state::Selection::cursor(idx));
                tab.sync_cursor_from_primary();
            }
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
