//! The Grid and Guides settings panel (`docs/ui/panels.md`): the grid, its
//! spacing and snapping, the guides one by one, and the rulers. Opened by
//! View › Show ↘. Every change is a `view.*` command, so it undoes.

use coursecraft_course::EditorSettings;
use egui::{DragValue, Response, Ui};
use serde_json::{Value as Json, json};

use crate::Editor;
use crate::panels::{Section, section_header};
use crate::properties::{inspector_row, inspector_separator};

/// Draw the panel's body.
pub fn show(editor: &mut Editor, ui: &mut Ui) {
    let mut actions: Vec<(&'static str, Json)> = Vec::new();
    egui::ScrollArea::vertical()
        .id_salt("grid_panel_scroll")
        .max_height(ui.available_height())
        .animated(false)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Frame::NONE
                .inner_margin(egui::Margin::same(coursecraft_render::PANEL_MARGIN as i8))
                .show(ui, |ui| sections(editor, ui, &mut actions));
        });
    for (cmd, params) in actions {
        editor.run(cmd, params);
    }
}

fn sections(editor: &mut Editor, ui: &mut Ui, actions: &mut Vec<(&'static str, Json)>) {
    let view = editor.session.course.editor.clone();
    let size = editor.session.course.slide_size;

    section_header(&mut editor.ui.panels, ui, Section::Grid);
    view_toggle(ui, tl!("Show grid"), view.show_grid, "show_grid", actions);
    view_toggle(
        ui,
        tl!("Snap to grid"),
        view.snap_to_grid,
        "snap_to_grid",
        actions,
    );
    inspector_row(ui, tl!("Spacing"), |ui| {
        let range = EditorSettings::GRID_SPACING_RANGE;
        if let Some(v) = committed_value(
            ui,
            egui::Id::unique("grid_panel_spacing"),
            view.grid_step(),
            *range.start()..=*range.end(),
        ) {
            actions.push(("view.set", json!({ "grid_spacing": v })));
        }
    });

    inspector_separator(ui);
    section_header(&mut editor.ui.panels, ui, Section::Guides);
    view_toggle(
        ui,
        tl!("Show guides"),
        view.show_guides,
        "show_guides",
        actions,
    );
    // Snapping to guides is a setting of this editor, not of the course.
    ui.checkbox(&mut editor.ui.snap_to_guides, tl!("Snap to guides"))
        .on_hover_text(tl!(
            "Moved and resized objects line up with guides and other objects."
        ));
    ui.add_space(4.0);
    if view.guides.is_empty() {
        ui.label(
            egui::RichText::new(tl!(
                "No guides yet. Add one here, or drag one out of a ruler."
            ))
            .color(coursecraft_render::palette(ui.ctx()).text_secondary),
        );
    }
    let (w, h) = (size.w as f32, size.h as f32);
    for (index, guide) in view.guides.iter().enumerate() {
        let label = if guide.vertical {
            tl!("Vertical")
        } else {
            tl!("Horizontal")
        };
        inspector_row(ui, label, |ui| {
            let span = if guide.vertical { w } else { h };
            if let Some(at) = committed_value(
                ui,
                egui::Id::unique(("grid_panel_guide", index)),
                guide.at,
                -span..=span * 2.0,
            ) {
                actions.push(("view.guides.move", json!({ "index": index, "at": at })));
            }
            if ui
                .small_button("×")
                .on_hover_text(tl!("Remove this guide"))
                .clicked()
            {
                actions.push(("view.guides.remove", json!({ "index": index })));
            }
        });
    }
    ui.horizontal_wrapped(|ui| {
        if ui.button(tl!("Add vertical guide")).clicked() {
            actions.push((
                "view.guides.add",
                json!({ "vertical": true, "at": w / 2.0 }),
            ));
        }
        if ui.button(tl!("Add horizontal guide")).clicked() {
            actions.push((
                "view.guides.add",
                json!({ "vertical": false, "at": h / 2.0 }),
            ));
        }
    });

    inspector_separator(ui);
    section_header(&mut editor.ui.panels, ui, Section::Rulers);
    view_toggle(
        ui,
        tl!("Show rulers"),
        view.show_rulers,
        "show_rulers",
        actions,
    );
    inspector_row(ui, tl!("Units"), |ui| {
        ui.add_enabled_ui(false, |ui| ui.label(tl!("Pixels")))
            .response
            .on_disabled_hover_text(tl!("Not available yet: other ruler units"));
    });
}

/// A course view setting as a checkbox: a change runs `view.set`.
fn view_toggle(
    ui: &mut Ui,
    label: &str,
    on: bool,
    key: &str,
    actions: &mut Vec<(&'static str, Json)>,
) {
    let mut value = on;
    if ui.checkbox(&mut value, label).changed() {
        actions.push(("view.set", json!({ key: value })));
    }
}

/// A number in slide pixels, edited by dragging or typing. While the author
/// drags or types, the value lives here; it comes back once, when the drag
/// ends or the field loses focus, so one edit is one undo step.
pub(crate) fn committed_value(
    ui: &mut Ui,
    id: egui::Id,
    current: f32,
    range: std::ops::RangeInclusive<f32>,
) -> Option<f32> {
    committed_number(ui, id, current, range, " px")
}

/// [`committed_value`] with its own unit suffix (" px", "°", "%").
pub(crate) fn committed_number(
    ui: &mut Ui,
    id: egui::Id,
    current: f32,
    range: std::ops::RangeInclusive<f32>,
    suffix: &str,
) -> Option<f32> {
    let mut value = ui
        .data(|d| d.get_temp::<f32>(id))
        .unwrap_or(current)
        .clamp(*range.start(), *range.end());
    let resp: Response = ui.add(
        DragValue::new(&mut value)
            .range(range)
            .speed(1.0)
            .max_decimals(1)
            .suffix(suffix),
    );
    let editing = resp.dragged() || resp.has_focus();
    let done = resp.drag_stopped()
        || resp.lost_focus()
        || (resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
    if editing && !done {
        ui.data_mut(|d| d.insert_temp(id, value));
        return None;
    }
    ui.data_mut(|d| d.remove::<f32>(id));
    (done && value.is_finite() && (value - current).abs() > f32::EPSILON).then_some(value)
}
