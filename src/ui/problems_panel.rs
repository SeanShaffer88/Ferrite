//! Problems panel: schema / syntax diagnostics for JSON, YAML and TOML tabs.
//!
//! Validation runs on a worker thread. Edits are debounced, and a result is
//! only accepted if it belongs to the newest request (generation guard), so the
//! UI thread never blocks and never shows stale problems.

use crate::devtools::schema::{check_document, Problem, ProblemSource, Severity};
use crate::markdown::tree_viewer::StructuredFileType;
use eframe::egui::{self, RichText};
use rust_i18n::t;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

/// Quiet time after the last edit before validation starts.
const DEBOUNCE: Duration = Duration::from_millis(300);
/// Documents above this size are fingerprinted by sampling, not fully hashed.
const SAMPLE_THRESHOLD: usize = 8 * 1024 * 1024;
const SAMPLE_LEN: usize = 64 * 1024;

/// What the panel needs to know about the active document.
pub struct DocInput<'a> {
    pub path: Option<&'a Path>,
    pub content: &'a str,
    pub ft: StructuredFileType,
    pub mappings: &'a [(String, String)],
}

/// Result of showing the panel for one frame.
#[derive(Default)]
pub struct ProblemsOutput {
    /// 1-based line the user clicked.
    pub jump_to_line: Option<usize>,
    pub closed: bool,
}

/// Panel state plus the validation scheduler.
pub struct ProblemsPanel {
    pub visible: bool,
    problems: Vec<Problem>,
    generation: u64,
    fingerprint: Option<(Option<PathBuf>, usize, u64)>,
    pending_since: Option<Instant>,
    last_check: Option<Instant>,
    rx: Option<Receiver<(u64, Vec<Problem>)>>,
    /// Number of validation jobs started (tests and diagnostics).
    pub jobs_started: u64,
}

impl Default for ProblemsPanel {
    fn default() -> Self {
        Self::new()
    }
}

fn fingerprint(content: &str) -> u64 {
    let b = content.as_bytes();
    let mut h = std::collections::hash_map::DefaultHasher::new();
    if b.len() <= SAMPLE_THRESHOLD {
        b.hash(&mut h);
    } else {
        let mid = b.len() / 2;
        b[..SAMPLE_LEN].hash(&mut h);
        b[mid..mid + SAMPLE_LEN].hash(&mut h);
        b[b.len() - SAMPLE_LEN..].hash(&mut h);
    }
    h.finish()
}

impl ProblemsPanel {
    pub fn new() -> Self {
        Self {
            visible: false,
            problems: Vec::new(),
            generation: 0,
            fingerprint: None,
            pending_since: None,
            last_check: None,
            rx: None,
            jobs_started: 0,
        }
    }

    pub fn problems(&self) -> &[Problem] {
        &self.problems
    }

    /// Count of problems at error severity.
    pub fn error_count(&self) -> usize {
        self.problems
            .iter()
            .filter(|p| p.severity == Severity::Error)
            .count()
    }

    /// Accept a worker result. Returns false (and ignores it) if it is stale.
    pub fn accept(&mut self, generation: u64, problems: Vec<Problem>) -> bool {
        if generation != self.generation {
            return false;
        }
        self.problems = problems;
        true
    }

    fn clear(&mut self) {
        self.problems.clear();
        self.fingerprint = None;
        self.pending_since = None;
        self.rx = None;
        // Invalidate any in-flight job.
        self.generation += 1;
    }

    fn start_job(&mut self, doc: &DocInput<'_>) {
        self.generation += 1;
        self.jobs_started += 1;
        let generation = self.generation;
        let (tx, rx) = channel();
        self.rx = Some(rx);
        let path = doc.path.map(Path::to_path_buf);
        let content = doc.content.to_owned();
        let ft = doc.ft;
        let mappings = doc.mappings.to_vec();
        let spawned = std::thread::Builder::new()
            .name("devtools-validate".into())
            .spawn(move || {
                let problems = check_document(path.as_deref(), &content, ft, &mappings);
                let _ = tx.send((generation, problems));
            });
        if spawned.is_err() {
            log::warn!("devtools: could not start validation thread");
            self.rx = None;
        }
    }

    /// Drive the scheduler. Call every frame; returns true while a repaint is
    /// needed (job pending or running).
    pub fn tick(&mut self, doc: Option<DocInput<'_>>, enabled: bool, now: Instant) -> bool {
        let Some(doc) = doc.filter(|_| enabled) else {
            if !self.problems.is_empty() || self.fingerprint.is_some() || self.rx.is_some() {
                self.clear();
            }
            return false;
        };

        // Poll a finished job.
        if let Some(rx) = &self.rx {
            let results: Vec<_> = rx.try_iter().collect();
            for (g, p) in results {
                self.accept(g, p);
            }
        }

        // Cheap change detection: length every frame, content at most every 150 ms
        // when idle. While an edit is pending, check every tick so the debounce is
        // measured from the real last edit, not the last throttled check.
        let key_path = doc.path.map(Path::to_path_buf);
        let len = doc.content.len();
        let due = self.pending_since.is_some()
            || self
                .last_check
                .map(|t| now.duration_since(t) >= Duration::from_millis(150))
                .unwrap_or(true);
        let len_changed = self
            .fingerprint
            .as_ref()
            .map(|f| f.1 != len)
            .unwrap_or(true);
        let path_changed = self
            .fingerprint
            .as_ref()
            .map(|f| f.0 != key_path)
            .unwrap_or(true);
        if len_changed || path_changed || due {
            self.last_check = Some(now);
            let fp = (key_path, len, fingerprint(doc.content));
            if self.fingerprint.as_ref() != Some(&fp) {
                let first = self.fingerprint.is_none() || path_changed;
                self.fingerprint = Some(fp);
                // New document: validate at once. Edit: wait for a quiet period.
                self.pending_since = Some(if first { now - DEBOUNCE } else { now });
            }
        }

        if let Some(since) = self.pending_since {
            if now.duration_since(since) >= DEBOUNCE {
                self.pending_since = None;
                self.start_job(&doc);
            }
        }
        self.pending_since.is_some() || self.rx.is_some()
    }

    /// Draw the panel body (caller provides the surrounding bottom panel).
    pub fn show(&mut self, ui: &mut egui::Ui) -> ProblemsOutput {
        let mut out = ProblemsOutput::default();
        ui.horizontal(|ui| {
            ui.strong(t!("devtools.problems.title", count = self.problems.len()).to_string());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button("✕")
                    .on_hover_text(t!("devtools.problems.close").to_string())
                    .clicked()
                {
                    out.closed = true;
                }
            });
        });
        ui.separator();
        if self.problems.is_empty() {
            ui.weak(t!("devtools.problems.none").to_string());
            return out;
        }
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for p in &self.problems {
                    let icon = match p.severity {
                        Severity::Error => "✖",
                        Severity::Warning => "⚠",
                        Severity::Info => "ℹ",
                    };
                    let tag = match p.source {
                        ProblemSource::Schema => "schema",
                        ProblemSource::Lint => "syntax",
                        ProblemSource::Lsp => "lsp",
                    };
                    let text = format!("{icon} L{}:{}  {}  [{tag}]", p.line, p.col, p.message);
                    let resp = ui
                        .add(egui::Button::new(RichText::new(text)).frame(false))
                        .on_hover_text(t!("devtools.problems.jump_hint").to_string());
                    if resp.clicked() {
                        out.jump_to_line = Some(p.line);
                    }
                }
            });
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc<'a>(content: &'a str) -> DocInput<'a> {
        DocInput {
            path: Some(Path::new("/w/a.json")),
            content,
            ft: StructuredFileType::Json,
            mappings: &[],
        }
    }

    fn wait_for_result(p: &mut ProblemsPanel, mut now: Instant, content: &str) {
        for _ in 0..200 {
            now += Duration::from_millis(20);
            p.tick(Some(doc(content)), true, now);
            std::thread::sleep(Duration::from_millis(10));
            if p.rx.as_ref().map(|_| false).unwrap_or(true) {
                break;
            }
            if !p.problems.is_empty() {
                break;
            }
        }
    }

    #[test]
    fn stale_generation_result_is_dropped() {
        let mut p = ProblemsPanel::new();
        p.generation = 5;
        let stale = vec![];
        assert!(!p.accept(4, stale));
        assert!(p.accept(5, vec![]));
    }

    #[test]
    fn rapid_edits_debounced_to_one_job() {
        let mut p = ProblemsPanel::new();
        let t0 = Instant::now();
        // First sight of a document validates at once.
        p.tick(Some(doc("{\"a\":1}")), true, t0);
        assert_eq!(p.jobs_started, 1);
        // Many edits within the debounce window: no new job yet.
        let mut t = t0 + Duration::from_millis(500);
        for i in 0..10 {
            let c = format!("{{\"a\":{i}}}");
            p.tick(Some(doc(&c)), true, t);
            t += Duration::from_millis(20);
        }
        assert_eq!(p.jobs_started, 1);
        // Quiet period passes: exactly one more job.
        let c = "{\"a\":9}";
        p.tick(Some(doc(c)), true, t + Duration::from_millis(400));
        assert_eq!(p.jobs_started, 2);
    }

    #[test]
    fn syntax_error_reaches_panel() {
        let mut p = ProblemsPanel::new();
        wait_for_result(&mut p, Instant::now(), "{\"a\": ,}");
        assert_eq!(p.error_count(), 1);
    }

    #[test]
    fn toggle_off_clears_problems() {
        let mut p = ProblemsPanel::new();
        wait_for_result(&mut p, Instant::now(), "{\"a\": ,}");
        assert!(!p.problems().is_empty());
        p.tick(Some(doc("{\"a\": ,}")), false, Instant::now());
        assert!(p.problems().is_empty());
    }

    #[test]
    fn large_input_does_not_block_submit() {
        let big = format!("[{}0]", "0,".repeat(25 * 1024 * 1024));
        let mut p = ProblemsPanel::new();
        let start = Instant::now();
        p.tick(Some(doc(&big)), true, start);
        // Includes one 50 MB copy for the worker; must not parse or validate inline.
        assert!(
            start.elapsed() < Duration::from_millis(500),
            "{:?}",
            start.elapsed()
        );
        assert_eq!(p.jobs_started, 1);
    }

    #[test]
    fn empty_document_yields_no_problems() {
        let mut p = ProblemsPanel::new();
        wait_for_result(&mut p, Instant::now(), "");
        assert!(p.problems().is_empty());
    }
}
