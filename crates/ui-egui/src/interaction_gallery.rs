//! Insert ▸ Interaction gallery: every ready-made interaction as a card with
//! a small, drawn preview. Choosing one runs the same insert functions the menu
//! used before the gallery existed.

use coursecraft_render::{Palette, palette};
use egui::{Align2, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use crate::Editor;
use crate::dialog::Dialog;

/// Width of the gallery window in points.
pub const GALLERY_WIDTH: f32 = 760.0;
const CARD_W: f32 = 224.0;
const CARD_H: f32 = 132.0;
const CARD_GAP: f32 = 12.0;

/// A tiny diagram that suggests what the interaction looks like on a slide.
/// The previews are drawn with boxes, lines and circles, never images.
#[derive(Debug, Clone, Copy)]
enum Preview {
    Tabs,
    Accordion,
    Marker,
    Scroll,
    Timer,
    Meter,
    Flip,
    Hotspot,
    Process,
    Timeline,
    RevealGrid,
    ScenarioChoice,
}

struct Card {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    preview: Preview,
}

const CARDS: &[Card] = &[
    Card {
        id: "tabs",
        name: "Tabs",
        description: "Switch between three panels.",
        preview: Preview::Tabs,
    },
    Card {
        id: "accordion",
        name: "Accordion",
        description: "Open one section at a time.",
        preview: Preview::Accordion,
    },
    Card {
        id: "marker",
        name: "Marker",
        description: "A labelled dot with a short note.",
        preview: Preview::Marker,
    },
    Card {
        id: "scroll_panel",
        name: "Scrolling panel",
        description: "A panel for long content.",
        preview: Preview::Scroll,
    },
    Card {
        id: "timer",
        name: "Timer",
        description: "A countdown that fires triggers.",
        preview: Preview::Timer,
    },
    Card {
        id: "progress",
        name: "Progress meter",
        description: "A bar or ring bound to a variable.",
        preview: Preview::Meter,
    },
    Card {
        id: "flip_cards",
        name: "Flip cards",
        description: "Three cards that turn over when clicked.",
        preview: Preview::Flip,
    },
    Card {
        id: "hotspot_reveal",
        name: "Hotspot reveal",
        description: "Numbered spots reveal one callout at a time.",
        preview: Preview::Hotspot,
    },
    Card {
        id: "process_steps",
        name: "Process steps",
        description: "Four steps joined to buttons by a variable.",
        preview: Preview::Process,
    },
    Card {
        id: "timeline",
        name: "Timeline",
        description: "Five dated points open details.",
        preview: Preview::Timeline,
    },
    Card {
        id: "reveal_grid",
        name: "Reveal grid",
        description: "Six tiles reveal as you explore.",
        preview: Preview::RevealGrid,
    },
    Card {
        id: "scenario_choice",
        name: "Scenario choice",
        description: "Three choices show feedback.",
        preview: Preview::ScenarioChoice,
    },
];

fn box_at(painter: &egui::Painter, rect: Rect, p: &Palette, filled: bool) {
    let radius = CornerRadius::same(3);
    if filled {
        painter.rect_filled(rect, radius, p.accent);
    } else {
        painter.rect_stroke(
            rect,
            radius,
            Stroke::new(1.0, p.border),
            StrokeKind::Outside,
        );
    }
}

fn draw_preview(painter: &egui::Painter, area: Rect, preview: Preview, p: &Palette) {
    let h = area.height();
    let line = |a: Pos2, b: Pos2, color| {
        painter.line_segment([a, b], Stroke::new(2.0, color));
    };
    match preview {
        Preview::Tabs => {
            let tab_w = area.width() / 3.0;
            for i in 0..3 {
                let r = Rect::from_min_size(
                    area.min + vec2(tab_w * i as f32, 0.0),
                    vec2(tab_w - 3.0, h * 0.28),
                );
                box_at(painter, r, p, i == 0);
            }
            box_at(
                painter,
                Rect::from_min_size(area.min + vec2(0.0, h * 0.36), vec2(area.width(), h * 0.62)),
                p,
                false,
            );
        }
        Preview::Accordion => {
            let row_h = h / 3.0;
            for i in 0..3 {
                let r = Rect::from_min_size(
                    area.min + vec2(0.0, row_h * i as f32),
                    vec2(area.width(), row_h - 3.0),
                );
                box_at(painter, r, p, i == 1);
            }
        }
        Preview::Marker => {
            box_at(
                painter,
                Rect::from_min_size(area.center() - vec2(70.0, h * 0.4), vec2(140.0, h * 0.8)),
                p,
                false,
            );
            painter.circle_filled(area.min + vec2(28.0, h * 0.5), 8.0, p.accent);
            box_at(
                painter,
                Rect::from_min_size(area.min + vec2(50.0, h * 0.28), vec2(100.0, h * 0.44)),
                p,
                true,
            );
        }
        Preview::Scroll => {
            box_at(painter, area, p, false);
            painter.rect_filled(
                Rect::from_min_size(
                    area.max - vec2(10.0, area.height() - 4.0),
                    vec2(4.0, area.height() * 0.5),
                ),
                CornerRadius::same(2),
                p.accent,
            );
        }
        Preview::Timer => {
            let r =
                Rect::from_min_size(area.center() - vec2(70.0, h * 0.32), vec2(140.0, h * 0.64));
            box_at(painter, r, p, false);
            line(
                r.min + vec2(12.0, r.height() * 0.5),
                r.min + vec2(64.0, r.height() * 0.5),
                p.accent,
            );
            painter.circle_filled(r.max - vec2(22.0, r.height() * 0.5), 10.0, p.accent);
        }
        Preview::Meter => {
            let bar =
                Rect::from_min_size(area.min + vec2(0.0, 10.0), vec2(area.width() * 0.7, 14.0));
            box_at(painter, bar, p, false);
            painter.rect_filled(
                Rect::from_min_size(bar.min, vec2(bar.width() * 0.6, bar.height())),
                CornerRadius::same(3),
                p.accent,
            );
            painter.circle_stroke(
                area.max - vec2(28.0, h * 0.5),
                20.0,
                Stroke::new(3.0, p.border),
            );
        }
        Preview::Flip => {
            let card_w = (area.width() - 12.0) / 3.0;
            for i in 0..3 {
                box_at(
                    painter,
                    Rect::from_min_size(
                        area.min + vec2((card_w + 6.0) * i as f32, 0.0),
                        vec2(card_w, area.height()),
                    ),
                    p,
                    i != 1,
                );
            }
        }
        Preview::Hotspot => {
            box_at(painter, area, p, false);
            for (i, pos) in [vec2(0.28, 0.30), vec2(0.72, 0.34), vec2(0.50, 0.72)]
                .into_iter()
                .enumerate()
            {
                painter.circle_filled(
                    pos2(area.min.x + area.width() * pos.x, area.min.y + h * pos.y),
                    8.0,
                    if i == 1 { p.accent } else { p.border_strong },
                );
            }
        }
        Preview::Process => {
            let step = area.width() / 4.0;
            for i in 0..4 {
                let c = pos2(area.min.x + step * (i as f32 + 0.5), area.center().y);
                if i > 0 {
                    let prev = pos2(area.min.x + step * (i as f32 - 0.5), c.y);
                    line(prev + vec2(12.0, 0.0), c - vec2(12.0, 0.0), p.border_strong);
                }
                painter.circle_filled(c, 11.0, if i == 0 { p.accent } else { p.border_strong });
            }
        }
        Preview::Timeline => {
            let y = area.center().y;
            let left = area.min.x + 10.0;
            let right = area.max.x - 10.0;
            line(pos2(left, y), pos2(right, y), p.border_strong);
            for i in 0..5 {
                let x = left + (right - left) * i as f32 / 4.0;
                painter.circle_filled(
                    pos2(x, y),
                    7.0,
                    if i == 2 { p.accent } else { p.border_strong },
                );
                box_at(
                    painter,
                    Rect::from_center_size(pos2(x, area.min.y + 8.0), vec2(26.0, 10.0)),
                    p,
                    false,
                );
            }
        }
        Preview::RevealGrid => {
            let cell_w = (area.width() - 12.0) / 3.0;
            let cell_h = (h - 8.0) / 2.0;
            for row in 0..2 {
                for col in 0..3 {
                    let r = Rect::from_min_size(
                        area.min + vec2((cell_w + 6.0) * col as f32, (cell_h + 8.0) * row as f32),
                        vec2(cell_w, cell_h),
                    );
                    box_at(painter, r, p, row == 0 && col == 1);
                }
            }
        }
        Preview::ScenarioChoice => {
            box_at(
                painter,
                Rect::from_min_size(area.min, vec2(area.width(), h * 0.36)),
                p,
                false,
            );
            for i in 0..3 {
                box_at(
                    painter,
                    Rect::from_min_size(
                        area.min + vec2(0.0, h * (0.48 + i as f32 * 0.17)),
                        vec2(area.width() * 0.7, h * 0.13),
                    ),
                    p,
                    i == 1,
                );
            }
        }
    }
}

/// Draw the gallery. Returns each card's kind and clickable rect for tests.
pub fn show_interaction_gallery(
    ui: &mut Ui,
    on_pick: &mut dyn FnMut(&'static str),
) -> Vec<(&'static str, Rect)> {
    let p = palette(ui.ctx());
    let mut rects = Vec::new();
    ui.set_min_width(GALLERY_WIDTH - 24.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(CARD_GAP, CARD_GAP);
        for card in CARDS {
            let (rect, response) = ui.allocate_exact_size(vec2(CARD_W, CARD_H), Sense::click());
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::Role::Button, true, card.name.to_owned())
            });
            let bg = if response.hovered() {
                p.hover
            } else {
                p.raised
            };
            ui.painter().rect_filled(rect, CornerRadius::same(8), bg);
            ui.painter().rect_stroke(
                rect,
                CornerRadius::same(8),
                Stroke::new(1.0, p.border),
                StrokeKind::Outside,
            );
            let preview = Rect::from_min_max(
                rect.min + vec2(12.0, 10.0),
                pos2(rect.max.x - 12.0, rect.min.y + 58.0),
            );
            draw_preview(ui.painter(), preview, card.preview, &p);
            ui.painter().text(
                rect.min + vec2(12.0, 68.0),
                Align2::LEFT_TOP,
                card.name,
                FontId::proportional(15.0),
                p.text,
            );
            ui.painter().text(
                rect.min + vec2(12.0, 92.0),
                Align2::LEFT_TOP,
                card.description,
                FontId::proportional(11.0),
                p.text_secondary,
            );
            rects.push((card.id, rect));
            if response.clicked() {
                on_pick(card.id);
            }
        }
    });
    rects
}

/// Run the insert function behind one gallery card.
fn insert_choice(editor: &mut Editor, id: &str) {
    match id {
        "marker" => {
            crate::interaction_insert::insert_marker(editor);
        }
        "scroll_panel" => {
            crate::interaction_insert::insert_scroll_panel(editor);
        }
        "timer" => {
            crate::interaction_insert::insert_timer(editor);
        }
        "progress" => {
            crate::interaction_insert::insert_progress_meter(editor);
        }
        "tabs" | "accordion" | "flip_cards" | "hotspot_reveal" | "process_steps" | "timeline"
        | "reveal_grid" | "scenario_choice" => {
            crate::interaction_insert::insert_interaction(editor, id);
        }
        _ => {}
    }
}

/// The Insert ▸ Interaction floating gallery.
pub fn show_interaction_gallery_window(editor: &mut Editor, ctx: &egui::Context) {
    if !editor.ui.show_interaction_gallery {
        return;
    }
    let mut open = true;
    let mut picked = None;
    Dialog::new("Insert Interaction")
        .open(&mut open)
        .width(GALLERY_WIDTH)
        .show(ctx, |ui| {
            editor.ui.interaction_gallery_rects =
                show_interaction_gallery(ui, &mut |kind| picked = Some(kind));
        });
    let escape = ctx.input(|input| input.key_pressed(egui::Key::Escape));
    if let Some(kind) = picked {
        insert_choice(editor, kind);
        editor.ui.show_interaction_gallery = false;
    } else if !open || escape {
        editor.ui.show_interaction_gallery = false;
    }
}
