//! The Size and Position settings panel (`docs/ui/panels.md`): the selected
//! object's width and height with an aspect lock, rotation and flips, its place
//! measured from the top left or the centre, and the lock. Opened by
//! Animations › Size and Position (button and ↘). Every change is one command,
//! so it undoes in one step.

use coursecraft_course::Rect;
use egui::Ui;
use serde_json::{Value as Json, json};

use crate::Editor;
use crate::grid_panel::{committed_number, committed_value};
use crate::panels::{Section, section_header};
use crate::properties::{inspector_row, inspector_separator};

/// Largest width, height or position offset the panel offers, in slide pixels.
const MAX_SPAN: f32 = 20_000.0;

/// Draw the panel's body.
pub fn show(editor: &mut Editor, ui: &mut Ui) {
    let mut actions: Vec<(&'static str, Json)> = Vec::new();
    egui::ScrollArea::vertical()
        .id_salt("size_panel_scroll")
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

/// What the panel edits: the selected objects of the current slide.
struct Target {
    slide: String,
    ids: Vec<String>,
    /// The single selected object's frame, rotation, flips and lock.
    one: Option<(Rect, f32, bool, bool, bool)>,
    /// Every selected object is locked.
    all_locked: bool,
}

fn target(editor: &Editor) -> Option<Target> {
    let slide = editor.session.course.slide(&editor.selection.slide)?;
    let ids: Vec<String> = editor
        .selection
        .ids()
        .into_iter()
        .filter(|id| slide.object(id).is_some())
        .collect();
    if ids.is_empty() {
        return None;
    }
    let one = (ids.len() == 1)
        .then(|| slide.object(&ids[0]))
        .flatten()
        .map(|(o, _)| (o.frame, o.rotation, o.flip_h, o.flip_v, o.locked));
    let all_locked = ids
        .iter()
        .all(|id| slide.object(id).is_some_and(|(o, _)| o.locked));
    Some(Target {
        slide: slide.id.clone(),
        ids,
        one,
        all_locked,
    })
}

/// Where Position measures from, kept for the session.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Origin {
    #[default]
    TopLeft,
    Centre,
}

fn sections(editor: &mut Editor, ui: &mut Ui, actions: &mut Vec<(&'static str, Json)>) {
    let Some(t) = target(editor) else {
        section_header(&mut editor.ui.panels, ui, Section::Size);
        ui.label(
            egui::RichText::new(tl!("Select an object to set its size and position."))
                .color(coursecraft_render::palette(ui.ctx()).text_secondary),
        );
        return;
    };
    let objects = json!(t.ids);
    let lock_id = egui::Id::unique("size_panel_lock_aspect");
    let origin_id = egui::Id::unique("size_panel_origin");
    let mut lock_aspect = ui.data(|d| d.get_temp::<bool>(lock_id)).unwrap_or(false);
    let mut origin = ui
        .data(|d| d.get_temp::<Origin>(origin_id))
        .unwrap_or_default();
    let one_only = tl!("Select one object to set its size and position.");

    section_header(&mut editor.ui.panels, ui, Section::Size);
    match t.one {
        Some((frame, ..)) => {
            let set_frame = |frame: Rect| {
                (
                    "object.set_frame",
                    json!({ "slide": t.slide, "object": t.ids[0], "frame": frame }),
                )
            };
            inspector_row(ui, tl!("Width"), |ui| {
                if let Some(w) = committed_value(
                    ui,
                    egui::Id::unique("size_panel_w"),
                    frame.w,
                    1.0..=MAX_SPAN,
                ) {
                    let h = if lock_aspect && frame.w > 0.0 {
                        frame.h * w / frame.w
                    } else {
                        frame.h
                    };
                    actions.push(set_frame(Rect { w, h, ..frame }));
                }
            });
            inspector_row(ui, tl!("Height"), |ui| {
                if let Some(h) = committed_value(
                    ui,
                    egui::Id::unique("size_panel_h"),
                    frame.h,
                    1.0..=MAX_SPAN,
                ) {
                    let w = if lock_aspect && frame.h > 0.0 {
                        frame.w * h / frame.h
                    } else {
                        frame.w
                    };
                    actions.push(set_frame(Rect { w, h, ..frame }));
                }
            });
            ui.checkbox(&mut lock_aspect, tl!("Lock aspect ratio"));
        }
        None => {
            ui.label(
                egui::RichText::new(one_only)
                    .color(coursecraft_render::palette(ui.ctx()).text_secondary),
            );
        }
    }

    inspector_separator(ui);
    section_header(&mut editor.ui.panels, ui, Section::Rotate);
    let rotation = t.one.map_or(0.0, |(_, r, ..)| r);
    inspector_row(ui, tl!("Rotation"), |ui| {
        if let Some(degrees) = committed_number(
            ui,
            egui::Id::unique("size_panel_rotation"),
            rotation,
            -360.0..=360.0,
            "°",
        ) {
            actions.push((
                "object.set_rotation",
                json!({ "slide": t.slide, "objects": objects, "degrees": degrees }),
            ));
        }
    });
    ui.horizontal_wrapped(|ui| {
        for (label, axis) in [
            (tl!("Flip horizontal"), "horizontal"),
            (tl!("Flip vertical"), "vertical"),
        ] {
            if ui.button(label).clicked() {
                actions.push((
                    "object.flip",
                    json!({ "slide": t.slide, "objects": objects, "axis": axis }),
                ));
            }
        }
    });

    inspector_separator(ui);
    section_header(&mut editor.ui.panels, ui, Section::Position);
    if let Some((frame, ..)) = t.one {
        inspector_row(ui, tl!("From"), |ui| {
            egui::ComboBox::from_id_salt("size_panel_origin_combo")
                .selected_text(match origin {
                    Origin::TopLeft => tl!("Top left corner"),
                    Origin::Centre => tl!("Centre"),
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut origin, Origin::TopLeft, tl!("Top left corner"));
                    ui.selectable_value(&mut origin, Origin::Centre, tl!("Centre"));
                });
        });
        let (dx, dy) = match origin {
            Origin::TopLeft => (0.0, 0.0),
            Origin::Centre => (frame.w / 2.0, frame.h / 2.0),
        };
        let set_pos = |x: f32, y: f32| {
            (
                "object.set_frame",
                json!({ "slide": t.slide, "object": t.ids[0], "frame": Rect { x, y, ..frame } }),
            )
        };
        inspector_row(ui, tl!("Horizontal"), |ui| {
            if let Some(x) = committed_value(
                ui,
                egui::Id::unique(("size_panel_x", origin == Origin::Centre)),
                frame.x + dx,
                -MAX_SPAN..=MAX_SPAN,
            ) {
                actions.push(set_pos(x - dx, frame.y));
            }
        });
        inspector_row(ui, tl!("Vertical"), |ui| {
            if let Some(y) = committed_value(
                ui,
                egui::Id::unique(("size_panel_y", origin == Origin::Centre)),
                frame.y + dy,
                -MAX_SPAN..=MAX_SPAN,
            ) {
                actions.push(set_pos(frame.x, y - dy));
            }
        });
    } else {
        ui.label(
            egui::RichText::new(one_only)
                .color(coursecraft_render::palette(ui.ctx()).text_secondary),
        );
    }

    inspector_separator(ui);
    let mut locked = t.all_locked;
    if ui
        .checkbox(&mut locked, tl!("Lock position and size"))
        .on_hover_text(tl!(
            "Locked objects cannot be moved or resized on the canvas."
        ))
        .changed()
    {
        actions.push((
            "object.lock",
            json!({ "slide": t.slide, "objects": objects, "locked": locked }),
        ));
    }

    ui.data_mut(|d| {
        d.insert_temp(lock_id, lock_aspect);
        d.insert_temp(origin_id, origin);
    });
}
