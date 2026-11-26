//! The Focus Order dialog (Home › Slide › Focus Order).
//!
//! It lists every object and interactive group of the current slide that can take
//! keyboard focus, in the order the learner's Tab key visits them. The list comes from
//! [`coursecraft_render::focus_order_candidates`], the same rules the player uses.
//! The author moves items up and down or drags them, then OK writes `focus_order`
//! 1..n for every item in one undo step.

use serde_json::{Value as Json, json};

use crate::Editor;
use crate::dialog::Dialog;
use coursecraft_course::Slide;
use coursecraft_render::{FocusCandidate, focus_order_candidates};

/// One row of the dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusOrderItem {
    /// Object id, or interactive group id when `is_group`.
    pub id: String,
    pub is_group: bool,
    /// What the list shows: the name, plus the layer for layer objects.
    pub label: String,
}

/// State of the Focus Order window.
#[derive(Debug, Clone, Default)]
pub struct FocusOrderState {
    pub open: bool,
    /// The slide the list belongs to.
    pub slide: String,
    pub items: Vec<FocusOrderItem>,
    /// The row Move Up and Move Down act on.
    pub selected: Option<usize>,
    /// The list was changed since it was opened or reset.
    pub dirty: bool,
    /// "Reset to default order" was pressed: OK clears every `focus_order`.
    pub cleared: bool,
    /// The row being dragged.
    drag_from: Option<usize>,
}

fn item_of(slide: &Slide, candidate: FocusCandidate) -> FocusOrderItem {
    FocusOrderItem {
        label: crate::triggers::object_label(slide, &candidate.id),
        id: candidate.id,
        is_group: candidate.is_group,
    }
}

impl FocusOrderState {
    /// Open the window for `slide`, listing its items in the current order.
    pub fn open_for(&mut self, slide: &Slide) {
        *self = Self {
            open: true,
            slide: slide.id.clone(),
            items: focus_order_candidates(slide)
                .into_iter()
                .map(|candidate| item_of(slide, candidate))
                .collect(),
            ..Self::default()
        };
    }

    /// Move the row at `index` one place up. Returns whether anything moved.
    pub fn move_up(&mut self, index: usize) -> bool {
        match index.checked_sub(1) {
            Some(to) => self.move_to(index, to),
            None => false,
        }
    }

    /// Move the row at `index` one place down. Returns whether anything moved.
    pub fn move_down(&mut self, index: usize) -> bool {
        self.move_to(index, index.saturating_add(1))
    }

    /// Move the row at `from` so it ends up at `to`. Returns whether anything moved.
    pub fn move_to(&mut self, from: usize, to: usize) -> bool {
        if from == to || from >= self.items.len() || to >= self.items.len() {
            return false;
        }
        let item = self.items.remove(from);
        self.items.insert(to, item);
        if self.selected == Some(from) {
            self.selected = Some(to);
        }
        self.dirty = true;
        self.cleared = false;
        true
    }

    /// Show the default order again: no author numbers, base layer first, then
    /// each layer. OK then clears every `focus_order` on the slide.
    pub fn reset_to_default(&mut self, slide: &Slide) {
        let mut plain = slide.clone();
        for object in &mut plain.objects {
            object.focus_order = None;
        }
        for group in &mut plain.groups {
            group.focus_order = None;
        }
        for layer in &mut plain.layers {
            for object in &mut layer.objects {
                object.focus_order = None;
            }
            for group in &mut layer.groups {
                group.focus_order = None;
            }
        }
        self.items = focus_order_candidates(&plain)
            .into_iter()
            .map(|candidate| item_of(slide, candidate))
            .collect();
        self.selected = None;
        self.dirty = true;
        self.cleared = true;
    }

    /// The commands OK runs, as one batch: `focus_order` 1..n in list order, or
    /// null for every item after a reset.
    #[must_use]
    pub fn steps(&self) -> Vec<(String, Json)> {
        self.items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let order = if self.cleared {
                    Json::Null
                } else {
                    json!(index.saturating_add(1))
                };
                if item.is_group {
                    (
                        "group.update".to_owned(),
                        json!({ "slide": self.slide, "group": item.id, "focus_order": order }),
                    )
                } else {
                    (
                        "object.update".to_owned(),
                        json!({ "slide": self.slide, "object": item.id, "focus_order": order }),
                    )
                }
            })
            .collect()
    }
}

impl Editor {
    /// Open the Focus Order window for the selected slide.
    pub fn open_focus_order(&mut self) {
        if self.ui.master_view {
            self.set_status(tl!("Not available in slide masters"));
            return;
        }
        let Some(slide) = self.session.course.slide(&self.selection.slide).cloned() else {
            self.set_status(tl!("Select a slide first"));
            return;
        };
        self.ui.focus_order.open_for(&slide);
    }

    /// Write the dialog's order as one undo step.
    pub fn apply_focus_order(&mut self) -> bool {
        let steps = self.ui.focus_order.steps();
        if steps.is_empty() {
            return true;
        }
        self.run_batch(&steps).is_some()
    }
}

/// Draw the Focus Order window.
pub fn show_focus_order_window(editor: &mut Editor, ctx: &egui::Context) {
    if !editor.ui.focus_order.open {
        return;
    }
    // The slide changed or vanished under the window: close it.
    let slide = editor
        .session
        .course
        .slide(&editor.ui.focus_order.slide)
        .cloned();
    let Some(slide) = slide else {
        editor.ui.focus_order.open = false;
        return;
    };
    let mut open = true;
    let mut ok = false;
    let mut cancel = false;
    Dialog::new(tl!("Focus Order"))
        .open(&mut open)
        .width(380.0)
        .show_with_footer(ctx, |dialog| {
            dialog.body(|ui| {
                ui.label(tl!(
                    "Tab visits these in order. Drag a row, or use Move Up and Move Down."
                ));
                ui.add_space(4.0);
                if editor.ui.focus_order.items.is_empty() {
                    ui.label(tl!("Nothing on this slide can take keyboard focus."));
                }
                draw_rows(ui, &mut editor.ui.focus_order);
            });
            dialog.footer(|ui| {
                ui.horizontal(|ui| {
                    let state = &mut editor.ui.focus_order;
                    let selected = state.selected;
                    if ui
                        .add_enabled(
                            selected.is_some_and(|i| i > 0),
                            egui::Button::new(tl!("Move Up")),
                        )
                        .on_disabled_hover_text(tl!("Select a row below the first row"))
                        .clicked()
                        && let Some(i) = selected
                    {
                        state.move_up(i);
                    }
                    if ui
                        .add_enabled(
                            selected.is_some_and(|i| i.saturating_add(1) < state.items.len()),
                            egui::Button::new(tl!("Move Down")),
                        )
                        .on_disabled_hover_text(tl!("Select a row above the last row"))
                        .clicked()
                        && let Some(i) = selected
                    {
                        state.move_down(i);
                    }
                    if ui.button(tl!("Reset to default order")).clicked() {
                        state.reset_to_default(&slide);
                    }
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(editor.ui.focus_order.dirty, egui::Button::new(tl!("OK")))
                        .on_disabled_hover_text(tl!("Change the order first"))
                        .clicked()
                    {
                        ok = true;
                    }
                    if ui.button(tl!("Cancel")).clicked() {
                        cancel = true;
                    }
                });
            });
        });
    if ok && editor.apply_focus_order() {
        open = false;
    }
    if cancel {
        open = false;
    }
    if !open {
        editor.ui.focus_order.open = false;
    }
}

fn draw_rows(ui: &mut egui::Ui, state: &mut FocusOrderState) {
    let mut drop: Option<(usize, usize)> = None;
    for index in 0..state.items.len() {
        let Some(item) = state.items.get(index) else {
            continue;
        };
        let text = format!("{}.  {}", index.saturating_add(1), item.label);
        let response = ui.add(
            egui::Button::selectable(state.selected == Some(index), text)
                .sense(egui::Sense::click_and_drag())
                .min_size(egui::vec2(ui.available_width(), 0.0)),
        );
        if response.clicked() {
            state.selected = Some(index);
        }
        if response.drag_started() {
            state.drag_from = Some(index);
            state.selected = Some(index);
        }
        if response.drag_stopped()
            && let Some(from) = state.drag_from.take()
            && let Some(pos) = ui.ctx().pointer_interact_pos()
        {
            // Dropped on the row under the pointer.
            drop = Some((
                from,
                row_at(ui, state.items.len(), pos.y, response.rect, index),
            ));
        }
    }
    if let Some((from, to)) = drop {
        state.move_to(from, to);
    }
}

/// The row index under pointer height `y`, given one known row `rect` at `known`.
fn row_at(ui: &egui::Ui, len: usize, y: f32, rect: egui::Rect, known: usize) -> usize {
    let height = rect.height() + ui.spacing().item_spacing.y;
    if height <= 0.0 || !height.is_finite() {
        return known;
    }
    let offset = ((y - rect.center().y) / height).round();
    let shifted = known as f32 + offset;
    if shifted.is_finite() && shifted > 0.0 {
        (shifted as usize).min(len.saturating_sub(1))
    } else {
        0
    }
}
