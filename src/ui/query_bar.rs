//! JSONPath query bar for the structured (tree) viewer.
//!
//! Queries run on a worker thread; only the newest request's result is shown.

use crate::devtools::query::{run_query, QueryError, QueryHit};
use crate::devtools::value::parse_to_value;
use crate::markdown::tree_viewer::StructuredFileType;
use eframe::egui::{self, Key, RichText, ScrollArea, TextEdit, Ui};
use rust_i18n::t;
use std::sync::{Arc, Mutex};

/// Re-run on every keystroke only up to this size; larger documents need Enter.
const LIVE_QUERY_LIMIT: usize = 1_000_000;

type Slot = Arc<Mutex<Option<(u64, Result<Vec<QueryHit>, String>)>>>;

/// State for one tree viewer's query bar.
#[derive(Debug, Clone, Default)]
pub struct QueryBarState {
    pub text: String,
    hits: Vec<QueryHit>,
    error: Option<String>,
    generation: u64,
    slot: Slot,
    running: bool,
}

/// Convert a JSON pointer into the tree viewer's `root.a.0` path.
pub fn pointer_to_tree_path(pointer: &str) -> String {
    let mut out = String::from("root");
    for seg in crate::devtools::locate::pointer_segments(pointer) {
        out.push('.');
        out.push_str(&seg);
    }
    out
}

impl QueryBarState {
    fn clear(&mut self) {
        self.hits.clear();
        self.error = None;
        self.running = false;
        self.generation += 1;
    }

    fn submit(&mut self, content: &str, ft: StructuredFileType) {
        if self.text.trim().is_empty() {
            self.clear();
            return;
        }
        self.generation += 1;
        self.running = true;
        let generation = self.generation;
        let expr = self.text.clone();
        let content = content.to_owned();
        let slot = Arc::clone(&self.slot);
        let spawned = std::thread::Builder::new()
            .name("devtools-query".into())
            .spawn(move || {
                let result = match parse_to_value(&content, ft) {
                    Err(e) => Err(e.to_string()),
                    Ok(v) => run_query(&v, &expr).map_err(|e| match e {
                        QueryError::Syntax(m) => t!("devtools.query.error", error = m).to_string(),
                        QueryError::TooManyHits(n) => {
                            t!("devtools.query.too_many", count = n).to_string()
                        }
                    }),
                };
                if let Ok(mut g) = slot.lock() {
                    // A slow, older job must never overwrite a newer result.
                    let newer_waiting = g.as_ref().is_some_and(|(have, _)| *have > generation);
                    if !newer_waiting {
                        *g = Some((generation, result));
                    }
                }
            });
        if spawned.is_err() {
            self.running = false;
            self.error = Some("could not start query thread".to_string());
        }
    }

    fn poll(&mut self) {
        let taken = self.slot.lock().ok().and_then(|mut g| g.take());
        if let Some((g, res)) = taken {
            if g == self.generation {
                self.running = false;
                match res {
                    Ok(h) => {
                        self.hits = h;
                        self.error = None;
                    }
                    Err(e) => {
                        self.hits.clear();
                        self.error = Some(e);
                    }
                }
            }
        }
    }

    /// Draw the bar. Returns the pointer of a clicked result.
    pub fn show(&mut self, ui: &mut Ui, content: &str, ft: StructuredFileType) -> Option<String> {
        self.poll();
        let mut clicked = None;
        ui.horizontal(|ui| {
            let resp = ui
                .add(
                    TextEdit::singleline(&mut self.text)
                        .hint_text(t!("devtools.query.placeholder").to_string())
                        .desired_width(f32::INFINITY),
                )
                .on_hover_text(t!("devtools.query.tooltip").to_string());
            let enter = resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
            let live = resp.changed() && content.len() <= LIVE_QUERY_LIMIT;
            if live || enter {
                self.submit(content, ft);
            }
        });
        if let Some(e) = &self.error {
            ui.colored_label(ui.visuals().error_fg_color, e);
        } else if !self.text.trim().is_empty() && !self.running {
            ui.weak(t!("devtools.query.results", count = self.hits.len()).to_string());
        }
        if self.running {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(50));
        }
        if !self.hits.is_empty() {
            ScrollArea::vertical()
                .id_salt("devtools_query_hits")
                .max_height(140.0)
                .show(ui, |ui| {
                    for h in &self.hits {
                        let text =
                            RichText::new(format!("{} = {}", h.pointer, h.preview)).monospace();
                        if ui.add(egui::Button::new(text).frame(false)).clicked() {
                            clicked = Some(h.pointer.clone());
                        }
                    }
                });
        }
        clicked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_maps_to_tree_path() {
        assert_eq!(pointer_to_tree_path(""), "root");
        assert_eq!(pointer_to_tree_path("/a/0/b"), "root.a.0.b");
    }

    #[test]
    fn query_runs_on_worker_and_newest_wins() {
        let mut s = QueryBarState::default();
        s.text = "$.a".into();
        s.submit("{\"a\":1}", StructuredFileType::Json);
        s.text = "$.b".into();
        s.submit("{\"a\":1,\"b\":2}", StructuredFileType::Json);
        for _ in 0..200 {
            s.poll();
            if !s.running {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(s.hits.len(), 1);
        assert_eq!(s.hits[0].pointer, "/b");
    }

    #[test]
    fn empty_document_and_empty_query_are_quiet() {
        let mut s = QueryBarState::default();
        s.text = "$.a".into();
        s.submit("", StructuredFileType::Json);
        for _ in 0..200 {
            s.poll();
            if !s.running {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(s.hits.is_empty() && s.error.is_none());
        s.text.clear();
        s.submit("{}", StructuredFileType::Json);
        assert!(!s.running);
    }

    #[test]
    fn bad_expression_sets_error() {
        let mut s = QueryBarState::default();
        s.text = "$.[[".into();
        s.submit("{}", StructuredFileType::Json);
        for _ in 0..200 {
            s.poll();
            if !s.running {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(s.error.is_some());
    }
}
