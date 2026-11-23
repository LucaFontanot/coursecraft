//! First-run onboarding coach marks.
//!
//! Three sequential dismissible popovers highlighting:
//! 1. Slide navigator: "Your slides live here. Right-click a slide for more options."
//! 2. Toolbar centre group: "Add text, shapes, media, buttons and questions."
//! 3. Inspector panel: "Select anything to edit it here."

use coursecraft_render::{
    CONTROL_COMPACT, RADIUS_LG, RADIUS_SM, SIZE_BODY, SIZE_SMALL, palette, popover_shadow,
    theme_mode,
};
use egui::{
    Align, Area, Color32, Context, CursorIcon, Id, Key, Layout, Modifiers, Pos2, Rect, RichText,
    Stroke, Ui, pos2, vec2,
};

use crate::Editor;

fn chrome_rects_id() -> Id {
    Id::unique("coursecraft_coach_chrome_rects")
}

/// Record the live toolbar, left slides panel, and right inspector panel rects for coach mark anchoring.
pub(crate) fn set_chrome_rects(ctx: &Context, toolbar: Rect, left_panel: Rect, right_panel: Rect) {
    ctx.data_mut(|d| d.insert_temp(chrome_rects_id(), (toolbar, left_panel, right_panel)));
}

/// Direction the coach mark caret arrow points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowDirection {
    Left,
    Up,
    Right,
}

/// Render the active coach mark popover, if any.
pub fn show_coach_marks(editor: &mut Editor, ui: &mut Ui) {
    if editor.ui.coach_marks_done || editor.ui.show_welcome {
        return;
    }

    let step = match editor.ui.current_coach_mark {
        Some(s @ 1..=3) => s,
        None => {
            editor.ui.current_coach_mark = Some(1);
            1
        }
        _ => return,
    };

    let p = palette(ui.ctx());
    let mode = theme_mode(ui.ctx());
    let screen = ui.ctx().content_rect();
    let (toolbar_rect, left_rect, right_rect) = ui
        .ctx()
        .data(|d| d.get_temp::<(Rect, Rect, Rect)>(chrome_rects_id()))
        .unwrap_or_else(|| {
            let top = screen.min.y + crate::TOP_BAR_HEIGHT;
            (
                Rect::from_min_size(
                    pos2(screen.min.x, top),
                    vec2(screen.width(), crate::toolbar::TOOLBAR_HEIGHT),
                ),
                Rect::from_min_size(
                    pos2(screen.min.x, top + crate::toolbar::TOOLBAR_HEIGHT),
                    vec2(crate::SLIDES_PANEL_WIDTH, 300.0),
                ),
                Rect::from_min_size(
                    pos2(
                        screen.max.x - crate::PROPERTIES_PANEL_WIDTH,
                        top + crate::toolbar::TOOLBAR_HEIGHT,
                    ),
                    vec2(crate::PROPERTIES_PANEL_WIDTH, 300.0),
                ),
            )
        });

    let popover_w = 260.0f32.min((screen.width() - 32.0).max(180.0));
    let est_h = 136.0f32;
    let caret_pad = 10.0f32;
    let (target_pos, raw_pos, arrow_dir, message) = match step {
        1 => {
            // Target: Slide navigator right edge
            let target_x = left_rect.max.x;
            let target_y = (left_rect.min.y + 80.0).min(screen.max.y - 80.0);
            let target = pos2(target_x, target_y);
            let pos = pos2(
                target_x + 14.0,
                (target_y - 24.0).max(toolbar_rect.max.y + 8.0),
            );
            (
                target,
                pos,
                ArrowDirection::Left,
                "Your slides live here. Right-click a slide for more options.",
            )
        }
        2 => {
            // Target: Toolbar centre group
            let center_x = toolbar_rect.center().x;
            let target = pos2(center_x, toolbar_rect.max.y);
            let pos = pos2(center_x - popover_w * 0.5, toolbar_rect.max.y + 12.0);
            (
                target,
                pos,
                ArrowDirection::Up,
                "Add text, shapes, media, buttons and questions.",
            )
        }
        3 => {
            // Target: Inspector panel left edge
            let inspector_x = right_rect.min.x;
            let target_y = (right_rect.min.y + 80.0).min(screen.max.y - 80.0);
            let target = pos2(inspector_x, target_y);
            let pos = pos2(
                inspector_x - popover_w - 14.0,
                (target_y - 24.0).max(toolbar_rect.max.y + 8.0),
            );
            (
                target,
                pos,
                ArrowDirection::Right,
                "Select anything to edit it here.",
            )
        }
        _ => return,
    };

    let min_x = screen.min.x + caret_pad;
    let max_x = (screen.max.x - popover_w - caret_pad).max(min_x);
    let min_y = screen.min.y + caret_pad;
    let max_y = (screen.max.y - est_h - caret_pad).max(min_y);
    let popover_pos = pos2(
        coursecraft_course::safe_clamp(raw_pos.x, min_x, max_x),
        coursecraft_course::safe_clamp(raw_pos.y, min_y, max_y),
    );

    let mut advance = false;
    let mut skip = !egui::Popup::is_any_open(ui.ctx())
        && ui
            .ctx()
            .input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));

    Area::new(Id::unique("coach_mark_popover"))
        .order(egui::Order::Foreground)
        .fade_in(false)
        .constrain_to(screen)
        .fixed_pos(popover_pos)
        .show(ui.ctx(), |ui| {
            // Draw the caret arrow pointing toward the target
            draw_caret_arrow(ui, popover_pos, popover_w, arrow_dir, p.raised, p.border);

            let frame = egui::Frame::NONE
                .fill(p.raised)
                .corner_radius(RADIUS_LG as u8)
                .stroke(Stroke::new(1.0, p.border))
                .shadow(popover_shadow(mode))
                .inner_margin(16.0);

            frame.show(ui, |ui| {
                ui.set_max_width(popover_w - 32.0);

                // Counter: "1 of 3"
                ui.label(
                    RichText::new(format!("{step} of 3"))
                        .size(SIZE_SMALL)
                        .color(p.text_tertiary),
                );
                ui.add_space(4.0);

                // Message text
                ui.label(RichText::new(message).size(SIZE_BODY).color(p.text));
                ui.add_space(14.0);

                // Actions: Skip (quiet) and Next/Done (primary small)
                ui.horizontal(|ui| {
                    if crate::quiet_button(ui, "Skip", crate::QuietTone::Secondary, true).clicked()
                    {
                        skip = true;
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let label = if step == 3 { "Done" } else { "Next" };
                        let btn = egui::Button::new(
                            RichText::new(label).color(Color32::WHITE).size(SIZE_SMALL),
                        )
                        .fill(p.accent)
                        .corner_radius(RADIUS_SM as u8)
                        .min_size(vec2(52.0, CONTROL_COMPACT));
                        let resp = ui.add(btn).on_hover_cursor(CursorIcon::PointingHand);
                        if resp.clicked() {
                            advance = true;
                        }
                    });
                });
            });
        });

    let _ = target_pos;

    if skip {
        editor.ui.coach_marks_done = true;
        editor.ui.current_coach_mark = None;
    } else if advance {
        if step >= 3 {
            editor.ui.coach_marks_done = true;
            editor.ui.current_coach_mark = None;
        } else {
            editor.ui.current_coach_mark = Some(step + 1);
        }
    }
}

fn draw_caret_arrow(
    ui: &mut Ui,
    popover_pos: Pos2,
    popover_w: f32,
    direction: ArrowDirection,
    fill_color: Color32,
    stroke_color: Color32,
) {
    let painter = ui.painter();
    let size = 8.0f32;

    match direction {
        ArrowDirection::Left => {
            let tip = pos2(popover_pos.x - size, popover_pos.y + 24.0);
            let top = pos2(popover_pos.x + 1.0, popover_pos.y + 24.0 - size);
            let bottom = pos2(popover_pos.x + 1.0, popover_pos.y + 24.0 + size);
            painter.add(egui::epaint::Shape::convex_polygon(
                vec![tip, top, bottom],
                fill_color,
                Stroke::NONE,
            ));
            painter.line_segment([top, tip], Stroke::new(1.0, stroke_color));
            painter.line_segment([tip, bottom], Stroke::new(1.0, stroke_color));
        }
        ArrowDirection::Up => {
            let mid_x = popover_pos.x + popover_w * 0.5;
            let tip = pos2(mid_x, popover_pos.y - size);
            let left = pos2(mid_x - size, popover_pos.y + 1.0);
            let right = pos2(mid_x + size, popover_pos.y + 1.0);
            painter.add(egui::epaint::Shape::convex_polygon(
                vec![tip, right, left],
                fill_color,
                Stroke::NONE,
            ));
            painter.line_segment([left, tip], Stroke::new(1.0, stroke_color));
            painter.line_segment([tip, right], Stroke::new(1.0, stroke_color));
        }
        ArrowDirection::Right => {
            let tip = pos2(popover_pos.x + popover_w + size, popover_pos.y + 24.0);
            let top = pos2(popover_pos.x + popover_w - 1.0, popover_pos.y + 24.0 - size);
            let bottom = pos2(popover_pos.x + popover_w - 1.0, popover_pos.y + 24.0 + size);
            painter.add(egui::epaint::Shape::convex_polygon(
                vec![tip, bottom, top],
                fill_color,
                Stroke::NONE,
            ));
            painter.line_segment([top, tip], Stroke::new(1.0, stroke_color));
            painter.line_segment([tip, bottom], Stroke::new(1.0, stroke_color));
        }
    }
}
