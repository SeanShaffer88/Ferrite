//! Settings rows for the developer tools (`settings.devtools`).

use eframe::egui::{DragValue, Ui};
use rust_i18n::t;

use super::{SettingsCtx, SettingsPanel};

fn toggle(ui: &mut Ui, value: &mut bool, label: &str, tooltip: &str) -> bool {
    ui.checkbox(value, t!(label))
        .on_hover_text(t!(tooltip))
        .changed()
}

pub(super) fn render_schema_validation(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    toggle(
        ui,
        &mut ctx.settings.devtools.schema_validation,
        "settings.devtools.schema_validation",
        "settings.devtools.schema_validation_tooltip",
    )
}

pub(super) fn render_query_bar(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    toggle(
        ui,
        &mut ctx.settings.devtools.query_bar,
        "settings.devtools.query_bar",
        "settings.devtools.query_bar_tooltip",
    )
}

pub(super) fn render_link_check(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    toggle(
        ui,
        &mut ctx.settings.devtools.link_check,
        "settings.devtools.link_check",
        "settings.devtools.link_check_tooltip",
    )
}

pub(super) fn render_code_block_lint(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    toggle(
        ui,
        &mut ctx.settings.devtools.code_block_lint,
        "settings.devtools.code_block_lint",
        "settings.devtools.code_block_lint_tooltip",
    )
}

pub(super) fn render_git_gutter(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    toggle(
        ui,
        &mut ctx.settings.devtools.git_gutter,
        "settings.devtools.git_gutter",
        "settings.devtools.git_gutter_tooltip",
    )
}

pub(super) fn render_breadcrumb(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    toggle(
        ui,
        &mut ctx.settings.devtools.breadcrumb,
        "settings.devtools.breadcrumb",
        "settings.devtools.breadcrumb_tooltip",
    )
}

/// How many save snapshots to keep per file (0 = version history off).
pub(super) fn render_history_max(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(t!("settings.devtools.history_max"))
            .on_hover_text(t!("settings.devtools.history_max_tooltip"));
        changed = ui
            .add(DragValue::new(&mut ctx.settings.devtools.history_max).range(0..=100))
            .on_hover_text(t!("settings.devtools.history_max_tooltip"))
            .changed();
    });
    changed
}

/// Column guide for git commit messages: on/off plus the column.
pub(super) fn render_git_editor_ruler(
    _panel: &mut SettingsPanel,
    ui: &mut Ui,
    ctx: &mut SettingsCtx<'_>,
) -> bool {
    let ruler = &mut ctx.settings.devtools.git_editor_ruler;
    let mut on = ruler.is_some();
    let mut col = ruler.unwrap_or(72);
    let mut changed = false;
    ui.horizontal(|ui| {
        changed |= toggle(
            ui,
            &mut on,
            "settings.devtools.git_editor_ruler",
            "settings.devtools.git_editor_ruler_tooltip",
        );
        ui.add_enabled_ui(on, |ui| {
            changed |= ui
                .add(DragValue::new(&mut col).range(20..=200).suffix(" col"))
                .changed();
        });
    });
    if changed {
        *ruler = on.then_some(col);
    }
    changed
}
