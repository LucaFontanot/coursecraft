//! History panel: list recorded undo/redo steps and jump to any step in one go.

use crate::Editor;
use crate::dialog::Dialog;
use egui::{Context, RichText};

enum HistoryAction {
    Undo(usize),
    Redo(usize),
}

/// Render the History window when the editor asked for it.
pub fn show_history_window(editor: &mut Editor, ctx: &Context) {
    if !editor.ui.show_history {
        return;
    }

    let redo_names: Vec<String> = editor
        .session
        .redo_names()
        .into_iter()
        .map(str::to_owned)
        .collect();
    let undo_names: Vec<String> = editor
        .session
        .undo_names()
        .into_iter()
        .map(str::to_owned)
        .collect();

    let mut open = true;
    let mut action: Option<HistoryAction> = None;

    Dialog::new("History")
        .open(&mut open)
        .width(300.0)
        .resizable(true)
        .show(ctx, |ui| {
            let p = coursecraft_render::palette(ui.ctx());

            if editor.session.history_truncated() {
                ui.label(
                    RichText::new(format!(
                        "Older steps were removed (CourseCraft keeps the last {}).",
                        coursecraft_engine::MAX_HISTORY
                    ))
                    .size(coursecraft_render::SIZE_SMALL)
                    .color(p.text_secondary),
                );
                ui.add_space(4.0);
            }

            for (idx, name) in redo_names.iter().enumerate().rev() {
                if ui
                    .push_id(("redo", idx), |ui| {
                        ui.selectable_label(false, RichText::new(name).color(p.text_disabled))
                    })
                    .inner
                    .clicked()
                {
                    action = Some(HistoryAction::Redo(idx + 1));
                }
            }

            egui::Frame::NONE
                .fill(p.accent_subtle)
                .corner_radius(4.0)
                .inner_margin(egui::Margin::symmetric(6, 3))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.label(RichText::new("Now").strong().color(p.accent_text));
                });

            for (idx, name) in undo_names.iter().enumerate() {
                if ui
                    .push_id(("undo", idx), |ui| {
                        ui.selectable_label(false, RichText::new(name).color(p.text))
                    })
                    .inner
                    .clicked()
                {
                    action = Some(HistoryAction::Undo(idx + 1));
                }
            }

            if undo_names.is_empty() {
                ui.label(RichText::new("Opened").color(p.text_secondary));
            } else if ui
                .push_id("opened", |ui| {
                    ui.selectable_label(false, RichText::new("Opened").color(p.text_secondary))
                })
                .inner
                .clicked()
            {
                action = Some(HistoryAction::Undo(undo_names.len()));
            }
        });

    if let Some(action) = action {
        let res = match action {
            HistoryAction::Undo(n) => editor.session.undo_steps(n),
            HistoryAction::Redo(n) => editor.session.redo_steps(n),
        };
        match res {
            Ok(()) => {
                editor.ui.pending_undo_redo = true;
                editor.check_revision();
            }
            Err(e) => {
                let msg = e.to_string();
                editor.ui.last_error = Some((msg.clone(), editor.ui.current_time));
                editor.ui.error_history.push(msg);
            }
        }
    }

    if !open {
        editor.ui.show_history = false;
    }
}
