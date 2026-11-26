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
    /// The single selected object's picture source and crop, when it is a picture.
    picture: Option<(String, Option<coursecraft_course::CropRect>)>,
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
    let picture = (ids.len() == 1)
        .then(|| slide.object(&ids[0]))
        .flatten()
        .and_then(|(o, _)| match &o.kind {
            coursecraft_course::ObjectKind::Image { src, crop, .. } => Some((src.clone(), *crop)),
            _ => None,
        });
    Some(Target {
        slide: slide.id.clone(),
        ids,
        one,
        all_locked,
        picture,
    })
}

/// The least and most a Scale field accepts, in percent.
const SCALE_RANGE: std::ops::RangeInclusive<f32> = 1.0..=10_000.0;

/// `frame` as a percentage of `base`; 100 when `base` is not positive.
pub(crate) fn scale_percent(frame: f32, base: f32) -> f32 {
    if base > 0.0 && base.is_finite() && frame.is_finite() {
        frame / base * 100.0
    } else {
        100.0
    }
}

/// The frame after Scale sets one side to `percent` of `base`. With
/// `lock_aspect` the other side follows by the same ratio. The top left stays.
pub(crate) fn scaled_frame(
    frame: Rect,
    base: (f32, f32),
    percent: f32,
    width_side: bool,
    lock_aspect: bool,
) -> Rect {
    let percent = percent.clamp(*SCALE_RANGE.start(), *SCALE_RANGE.end());
    let mut out = frame;
    if width_side {
        out.w = (base.0 * percent / 100.0).clamp(1.0, MAX_SPAN);
        if lock_aspect && frame.w > 0.0 {
            out.h = (frame.h * out.w / frame.w).clamp(1.0, MAX_SPAN);
        }
    } else {
        out.h = (base.1 * percent / 100.0).clamp(1.0, MAX_SPAN);
        if lock_aspect && frame.h > 0.0 {
            out.w = (frame.w * out.h / frame.h).clamp(1.0, MAX_SPAN);
        }
    }
    out
}

/// A picture's original size in slide pixels: the source's natural size
/// narrowed by its crop. `None` for a size that is not positive.
pub(crate) fn original_size(
    natural: (f32, f32),
    crop: Option<coursecraft_course::CropRect>,
) -> Option<(f32, f32)> {
    let (cw, ch) = crop.map_or((1.0, 1.0), |c| (c.w.clamp(0.0, 1.0), c.h.clamp(0.0, 1.0)));
    let (w, h) = (natural.0 * cw, natural.1 * ch);
    (w.is_finite() && h.is_finite() && w >= 1.0 && h >= 1.0).then_some((w, h))
}

/// The frame "Reset to original size" sets: the original size, shrunk (keeping
/// the aspect ratio) to fit the slide, around the old centre and moved back
/// inside the slide.
pub(crate) fn reset_frame(frame: Rect, original: (f32, f32), slide: (f32, f32)) -> Rect {
    let (mut w, mut h) = original;
    let fit = (slide.0 / w).min(slide.1 / h).min(1.0);
    if fit.is_finite() && fit > 0.0 {
        w = (w * fit).max(1.0);
        h = (h * fit).max(1.0);
    }
    let cx = frame.x + frame.w / 2.0;
    let cy = frame.y + frame.h / 2.0;
    let x = (cx - w / 2.0).clamp(0.0, (slide.0 - w).max(0.0));
    let y = (cy - h / 2.0).clamp(0.0, (slide.1 - h).max(0.0));
    Rect { x, y, w, h }
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

            // Scale: a percentage of the picture's original size, or for
            // other objects of the size they had when selected.
            let original = t.picture.as_ref().and_then(|(src, crop)| {
                editor
                    .loaded_image_size(src)
                    .and_then(|natural| original_size(natural, *crop))
            });
            let base_id = egui::Id::unique("size_panel_scale_base");
            let stored = ui.data(|d| d.get_temp::<(String, f32, f32)>(base_id));
            let base = match (original, stored) {
                (Some(o), _) => o,
                (None, Some((id, w, h))) if id == t.ids[0] => (w, h),
                _ => (frame.w, frame.h),
            };
            ui.data_mut(|d| d.insert_temp(base_id, (t.ids[0].clone(), base.0, base.1)));
            inspector_row(ui, tl!("Scale width"), |ui| {
                if let Some(pct) = committed_number(
                    ui,
                    egui::Id::unique("size_panel_scale_w"),
                    scale_percent(frame.w, base.0),
                    SCALE_RANGE,
                    "%",
                ) {
                    actions.push(set_frame(scaled_frame(frame, base, pct, true, lock_aspect)));
                }
            });
            inspector_row(ui, tl!("Scale height"), |ui| {
                if let Some(pct) = committed_number(
                    ui,
                    egui::Id::unique("size_panel_scale_h"),
                    scale_percent(frame.h, base.1),
                    SCALE_RANGE,
                    "%",
                ) {
                    actions.push(set_frame(scaled_frame(
                        frame,
                        base,
                        pct,
                        false,
                        lock_aspect,
                    )));
                }
            });
            if t.picture.is_some() {
                let slide_size = editor.session.course.slide_size;
                let reason =
                    tl!("The picture's original size is not known until the picture has loaded.");
                let mut button = ui.add_enabled(
                    original.is_some(),
                    egui::Button::new(tl!("Reset to original size")),
                );
                if original.is_none() {
                    button = button.on_disabled_hover_text(reason);
                }
                if button.clicked()
                    && let Some(o) = original
                {
                    #[allow(clippy::cast_precision_loss)]
                    let slide = (slide_size.w as f32, slide_size.h as f32);
                    actions.push(set_frame(reset_frame(frame, o, slide)));
                }
            }
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
