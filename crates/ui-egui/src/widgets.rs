//! Icon button and toolbar widget primitives for CourseCraft.
//!
//! Provides the icon buttons and Keynote-style toolbar buttons specified in M5 step 2.

use coursecraft_render::{
    CONTROL_COMPACT, ICON_SM, ICON_TOOLBAR, Icon, RADIUS_MD, SIZE_SMALL, palette,
};
use egui::{
    Color32, CornerRadius, CursorIcon, FontFamily, FontId, Response, Sense, Ui, pos2, vec2,
};

/// Tone for icon buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IconTone {
    /// `text_secondary` at rest, `text` on hover/press.
    #[default]
    Secondary,
    /// `accent_text` at rest and on hover.
    Accent,
    /// `text_secondary` at rest, `danger` on hover.
    Danger,
}

/// Whether `ui`'s layer is the topmost layer under `pos`.
///
/// Panels that read raw pointer presses (the canvas, rulers, timeline rows,
/// slide layers) check this, so a click on a menu, popup or window drawn on
/// top of them is not also taken as a click on what lies underneath.
///
/// A press that began while a menu or popup was open never reaches them
/// either, up to and including its release: that press picks a menu item or
/// dismisses the menu, and the menu may already be gone by the time the
/// button comes up.
pub(crate) fn pointer_reaches(ui: &Ui, pos: egui::Pos2) -> bool {
    !press_began_over_popup(ui.ctx())
        && ui
            .ctx()
            .layer_id_at(pos)
            .is_none_or(|layer| layer == ui.layer_id())
}

fn popup_open_id() -> egui::Id {
    egui::Id::unique("coursecraft_popup_open_last_pass")
}

fn press_guard_id() -> egui::Id {
    egui::Id::unique("coursecraft_press_began_over_popup")
}

/// Whether the current press (or the click it just ended in) started while a
/// menu or popup was open.
pub(crate) fn press_began_over_popup(ctx: &egui::Context) -> bool {
    ctx.data(|data| data.get_temp::<bool>(press_guard_id()).unwrap_or(false))
}

/// Call at the start of every editor pass, before any panel reads the pointer.
pub(crate) fn begin_press_guard(ctx: &egui::Context) {
    let (pressed, down, released) = ctx.input(|input| {
        (
            input.pointer.any_pressed(),
            input.pointer.any_down(),
            input.pointer.any_released(),
        )
    });
    let popup_open = ctx.data(|data| data.get_temp::<bool>(popup_open_id()).unwrap_or(false))
        || egui::Popup::is_any_open(ctx);
    let mut guard = press_began_over_popup(ctx);
    if pressed {
        guard = popup_open;
    } else if !down && !released {
        // The press ended on an earlier pass; its click has been handled.
        guard = false;
    }
    ctx.data_mut(|data| data.insert_temp(press_guard_id(), guard));
}

/// Call at the end of every editor pass, after every menu has been drawn.
pub(crate) fn end_press_guard(ctx: &egui::Context) {
    let open = egui::Popup::is_any_open(ctx);
    ctx.data_mut(|data| data.insert_temp(popup_open_id(), open));
}

/// A compact 26×26 icon button.
///
/// Quiet at rest (no background fill), `hover` fill on hover, and `pressed` fill when pressed.
/// Displays the icon in `text_secondary`, turning `text` on hover.
pub fn icon_button(ui: &mut Ui, icon: Icon, tooltip: &str) -> Response {
    icon_button_impl(ui, icon, false, IconTone::Secondary, true, tooltip, tooltip)
}

/// A compact 26×26 icon button with explicit enabled state.
pub fn icon_button_enabled(ui: &mut Ui, icon: Icon, enabled: bool, tooltip: &str) -> Response {
    icon_button_impl(
        ui,
        icon,
        false,
        IconTone::Secondary,
        enabled,
        tooltip,
        tooltip,
    )
}

/// A compact 26×26 icon button with explicit enabled state and a separate disabled hover reason.
pub fn icon_button_enabled_reason(
    ui: &mut Ui,
    icon: Icon,
    enabled: bool,
    tooltip: &str,
    disabled_reason: &str,
) -> Response {
    icon_button_impl(
        ui,
        icon,
        false,
        IconTone::Secondary,
        enabled,
        tooltip,
        disabled_reason,
    )
}

/// A compact 26×26 icon button with danger tone on hover (e.g. delete actions).
pub fn icon_button_danger(ui: &mut Ui, icon: Icon, enabled: bool, tooltip: &str) -> Response {
    icon_button_impl(ui, icon, false, IconTone::Danger, enabled, tooltip, tooltip)
}

/// A compact 26×26 icon button with danger tone on hover and a separate disabled hover reason.
pub fn icon_button_danger_reason(
    ui: &mut Ui,
    icon: Icon,
    enabled: bool,
    tooltip: &str,
    disabled_reason: &str,
) -> Response {
    icon_button_impl(
        ui,
        icon,
        false,
        IconTone::Danger,
        enabled,
        tooltip,
        disabled_reason,
    )
}

/// A compact 26×26 icon toggle button.
///
/// When active, uses the filled icon in `accent_text` on `accent_subtle` background fill.
pub fn icon_toggle(ui: &mut Ui, icon: Icon, active: bool, tooltip: &str) -> Response {
    icon_button_impl(ui, icon, active, IconTone::Accent, true, tooltip, tooltip)
}

/// How long a control takes to fade between its rest, hover and active fills.
const HOVER_FADE_SECONDS: f32 = 0.1;

/// The fill for a quiet control, fading between rest, hover and active. Pressing is instant.
fn state_fill(
    ui: &Ui,
    resp: &Response,
    p: &coursecraft_render::Palette,
    hovered: bool,
    pressed: bool,
    active: bool,
) -> Color32 {
    let ctx = ui.ctx();
    let hover = ctx.animate_bool_with_time(resp.id.with("hover"), hovered, HOVER_FADE_SECONDS);
    let on = ctx.animate_bool_with_time(resp.id.with("active"), active, HOVER_FADE_SECONDS);
    let rest = Color32::TRANSPARENT.lerp_to_gamma(p.hover, hover);
    if pressed && !active {
        p.pressed
    } else {
        rest.lerp_to_gamma(p.accent_subtle, on)
    }
}

fn icon_button_impl(
    ui: &mut Ui,
    icon: Icon,
    active: bool,
    tone: IconTone,
    enabled: bool,
    tooltip: &str,
    disabled_reason: &str,
) -> Response {
    let p = palette(ui.ctx());
    let size = vec2(CONTROL_COMPACT, CONTROL_COMPACT);
    let resp = ui
        .add_enabled_ui(enabled, |ui| {
            let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
            resp.widget_info(|| {
                let mut info = egui::WidgetInfo::new(egui::Role::Button);
                info.enabled = enabled;
                info.label = Some(tooltip.to_owned());
                info.selected = active.then_some(true);
                info
            });

            if ui.is_rect_visible(rect) {
                let radius = CornerRadius::same(RADIUS_MD as u8);
                let hovered = enabled && resp.hovered();
                let pressed = enabled && resp.is_pointer_button_down_on();

                ui.painter().rect_filled(
                    rect,
                    radius,
                    state_fill(ui, &resp, &p, hovered, pressed, active),
                );

                // Icon color
                let icon_color = if !enabled {
                    p.text_disabled
                } else if active {
                    p.accent_text
                } else {
                    match tone {
                        IconTone::Danger if hovered => p.danger,
                        IconTone::Accent => p.accent_text,
                        _ if hovered || pressed => p.text,
                        _ => p.text_secondary,
                    }
                };

                let font_id = FontId::new(
                    ICON_SM,
                    if active {
                        FontFamily::Name("phosphor-fill".into())
                    } else {
                        FontFamily::Name("phosphor".into())
                    },
                );
                let galley =
                    ui.painter()
                        .layout_no_wrap(icon.codepoint().to_string(), font_id, icon_color);
                let pos = pos2(
                    rect.center().x - galley.size().x * 0.5,
                    rect.center().y - galley.size().y * 0.5,
                );
                ui.painter().galley(pos, galley, icon_color);
            }
            resp
        })
        .inner
        .on_hover_text(tooltip)
        .on_disabled_hover_text(disabled_reason);
    if enabled {
        resp.on_hover_cursor(CursorIcon::PointingHand)
    } else {
        resp
    }
}

/// A Keynote-style toolbar button with an 18 px icon above an 11 pt label.
///
/// 44 px tall, at least 52 px wide (or fits label), quiet at rest with `hover` fill on hover.
/// Below ≈1100 px window width, drops the label and shows icon-only with tooltip.
pub fn toolbar_button(ui: &mut Ui, icon: Icon, label: &str, tooltip: &str) -> Response {
    toolbar_button_impl(ui, icon, label, false, true, tooltip)
}

/// A Keynote-style toolbar toggle button.
///
/// When active, uses accent colours (`accent_text` on `accent_subtle`).
pub fn toolbar_toggle(
    ui: &mut Ui,
    icon: Icon,
    label: &str,
    active: bool,
    tooltip: &str,
) -> Response {
    toolbar_button_impl(ui, icon, label, active, true, tooltip)
}

/// A Keynote-style toolbar button with active state.
pub fn toolbar_button_active(
    ui: &mut Ui,
    icon: Icon,
    label: &str,
    active: bool,
    tooltip: &str,
) -> Response {
    toolbar_button_impl(ui, icon, label, active, true, tooltip)
}

/// A Keynote-style toolbar button with explicit enabled and active states.
pub fn toolbar_button_enabled(
    ui: &mut Ui,
    icon: Icon,
    label: &str,
    active: bool,
    enabled: bool,
    tooltip: &str,
) -> Response {
    toolbar_button_impl(ui, icon, label, active, enabled, tooltip)
}

const TOOLBAR_SHOW_LABELS_ID: &str = "coursecraft_toolbar_show_labels";

/// Record whether the current frame's toolbar buttons should render text labels.
pub fn set_toolbar_show_labels(ctx: &egui::Context, show: bool) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::unique(TOOLBAR_SHOW_LABELS_ID), show));
}

/// Whether toolbar buttons should render text labels on the current frame.
#[must_use]
pub fn toolbar_show_labels(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(egui::Id::unique(TOOLBAR_SHOW_LABELS_ID)))
        .unwrap_or_else(|| ctx.content_rect().width() >= 1100.0)
}

/// A Keynote-style toolbar button with an accent fill and white icon/label.
///
/// Used for the primary toolbar action (Publish). Below the responsive width threshold it
/// drops the label and shows the icon alone, like the other toolbar buttons.
pub fn toolbar_primary_button(ui: &mut Ui, icon: Icon, label: &str, tooltip: &str) -> Response {
    let p = palette(ui.ctx());
    let show_label = toolbar_show_labels(ui.ctx());
    let label_galley = if show_label {
        Some(ui.painter().layout_no_wrap(
            label.to_owned(),
            FontId::proportional(SIZE_SMALL),
            Color32::PLACEHOLDER,
        ))
    } else {
        None
    };
    let width = label_galley
        .as_ref()
        .map_or(38.0, |lg| (lg.size().x + 10.0).max(52.0));

    let (rect, resp) = ui.allocate_exact_size(vec2(width, 44.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tooltip.to_owned()));

    if ui.is_rect_visible(rect) {
        let radius = CornerRadius::same(RADIUS_MD as u8);
        let hover = ui.ctx().animate_bool_with_time(
            resp.id.with("hover"),
            resp.hovered(),
            HOVER_FADE_SECONDS,
        );
        let fill = if resp.is_pointer_button_down_on() {
            p.accent_pressed
        } else {
            p.accent.lerp_to_gamma(p.accent_hover, hover)
        };
        ui.painter().rect_filled(rect, radius, fill);

        let icon_font = FontId::new(ICON_TOOLBAR, FontFamily::Name("phosphor".into()));
        let icon_galley =
            ui.painter()
                .layout_no_wrap(icon.codepoint().to_string(), icon_font, Color32::WHITE);
        if let Some(lg) = label_galley {
            ui.painter().galley(
                pos2(
                    rect.center().x - icon_galley.size().x * 0.5,
                    rect.top() + 4.0,
                ),
                icon_galley,
                Color32::WHITE,
            );
            ui.painter().galley(
                pos2(
                    rect.center().x - lg.size().x * 0.5,
                    rect.bottom() - lg.size().y - 4.0,
                ),
                lg,
                Color32::WHITE,
            );
        } else {
            ui.painter().galley(
                pos2(
                    rect.center().x - icon_galley.size().x * 0.5,
                    rect.center().y - icon_galley.size().y * 0.5,
                ),
                icon_galley,
                Color32::WHITE,
            );
        }
    }

    resp.on_hover_text(tooltip)
        .on_hover_cursor(CursorIcon::PointingHand)
}

/// A small caret button companion for toolbar dropdowns (e.g. next to Add Slide).
pub fn toolbar_caret_button(ui: &mut Ui, tooltip: &str) -> Response {
    let p = palette(ui.ctx());
    let size = vec2(20.0, 44.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tooltip.to_owned()));

    if ui.is_rect_visible(rect) {
        let radius = CornerRadius::same(RADIUS_MD as u8);
        let hovered = resp.hovered();
        let pressed = resp.is_pointer_button_down_on();

        if pressed {
            ui.painter().rect_filled(rect, radius, p.pressed);
        } else if hovered {
            ui.painter().rect_filled(rect, radius, p.hover);
        }

        let icon_color = if hovered || pressed {
            p.text
        } else {
            p.text_secondary
        };

        let font_id = FontId::new(12.0, FontFamily::Name("phosphor".into()));
        let galley = ui.painter().layout_no_wrap(
            Icon::CaretDown.codepoint().to_string(),
            font_id,
            icon_color,
        );
        let pos = pos2(
            rect.center().x - galley.size().x * 0.5,
            rect.center().y - galley.size().y * 0.5,
        );
        ui.painter().galley(pos, galley, icon_color);
    }

    resp.on_hover_text(tooltip)
        .on_hover_cursor(CursorIcon::PointingHand)
}

fn toolbar_button_impl(
    ui: &mut Ui,
    icon: Icon,
    label: &str,
    active: bool,
    enabled: bool,
    tooltip: &str,
) -> Response {
    let p = palette(ui.ctx());
    // Drop labels and show icons only when narrow so nothing clips or overlaps.
    let show_label = toolbar_show_labels(ui.ctx());

    let height = 44.0;
    let label_font = FontId::proportional(SIZE_SMALL);
    let label_galley = if show_label {
        Some(
            ui.painter()
                .layout_no_wrap(label.to_owned(), label_font, Color32::PLACEHOLDER),
        )
    } else {
        None
    };

    let width = if let Some(ref lg) = label_galley {
        (lg.size().x + 10.0).max(52.0)
    } else {
        38.0
    };

    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), sense);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, tooltip.to_owned()));

    if ui.is_rect_visible(rect) {
        let radius = CornerRadius::same(RADIUS_MD as u8);
        let hovered = enabled && resp.hovered();
        let pressed = enabled && resp.is_pointer_button_down_on();

        ui.painter().rect_filled(
            rect,
            radius,
            state_fill(ui, &resp, &p, hovered, pressed, active),
        );

        // Colors
        let (icon_color, text_color) = if !enabled {
            (p.text_disabled, p.text_disabled)
        } else if active {
            (p.accent_text, p.accent_text)
        } else if hovered || pressed {
            (p.text, p.text)
        } else {
            (p.text_secondary, p.text_secondary)
        };

        // Icon
        let icon_font = FontId::new(
            ICON_TOOLBAR,
            if active {
                FontFamily::Name("phosphor-fill".into())
            } else {
                FontFamily::Name("phosphor".into())
            },
        );
        let icon_galley =
            ui.painter()
                .layout_no_wrap(icon.codepoint().to_string(), icon_font, icon_color);

        if let Some(lg) = label_galley {
            let icon_pos = pos2(
                rect.center().x - icon_galley.size().x * 0.5,
                rect.top() + 4.0,
            );
            ui.painter().galley(icon_pos, icon_galley, icon_color);

            let label_pos = pos2(
                rect.center().x - lg.size().x * 0.5,
                rect.bottom() - lg.size().y - 4.0,
            );
            ui.painter().galley(label_pos, lg, text_color);
        } else {
            let icon_pos = pos2(
                rect.center().x - icon_galley.size().x * 0.5,
                rect.center().y - icon_galley.size().y * 0.5,
            );
            ui.painter().galley(icon_pos, icon_galley, icon_color);
        }
    }

    let resp = resp.on_hover_text(tooltip);
    if enabled {
        resp.on_hover_cursor(CursorIcon::PointingHand)
    } else {
        resp
    }
}

/// Minimum width in points for [`styled_combo`] so labels never collapse into cramped boxes.
pub const COMBO_MIN_WIDTH: f32 = 120.0;

/// Minimum popup height in points for [`styled_combo`] so at least 12 items fit without scrolling.
pub const COMBO_MIN_POPUP_HEIGHT: f32 = 320.0;

/// A [`egui::ComboBox`] with an enforced minimum width and a tall popup (`>= 12` items).
pub fn styled_combo<'a>(
    ui: &Ui,
    id_salt: impl egui::AsIdSalt,
    min_width: f32,
) -> egui::ComboBox<'a> {
    let popup_height = (ui.ctx().content_rect().height() * 0.8).max(COMBO_MIN_POPUP_HEIGHT);
    egui::ComboBox::from_id_salt(id_salt)
        .width(min_width.max(COMBO_MIN_WIDTH))
        .height(popup_height)
}

/// Scrim colour for modal overlays (`UX-15`), meeting `alpha >= 0x99` in both themes.
#[must_use]
pub const fn modal_backdrop_color(mode: coursecraft_render::ThemeMode) -> Color32 {
    match mode {
        coursecraft_render::ThemeMode::Dark => Color32::from_black_alpha(0xB8),
        coursecraft_render::ThemeMode::Light => Color32::from_black_alpha(0x99),
    }
}

/// A tri-state toggle button supporting Off (`Some(false)`), On (`Some(true)`), and Mixed
/// (`None`) states with a hover tooltip and `accesskit::Toggled` accessibility state.
pub fn mixed_toggle(
    ui: &mut Ui,
    label: impl Into<egui::WidgetText>,
    state: Option<bool>,
    tooltip: &str,
) -> Response {
    let p = palette(ui.ctx());
    let widget_text: egui::WidgetText = label.into();
    let galley = widget_text.into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Button,
    );
    let width = (galley.size().x + 12.0).max(CONTROL_COMPACT);
    let size = vec2(width, CONTROL_COMPACT);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let accessible_name = if tooltip.is_empty() {
        galley.text().to_owned()
    } else {
        tooltip.to_owned()
    };
    resp.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::Role::Button,
            true,
            state == Some(true),
            accessible_name.clone(),
        )
    });
    if state.is_none() {
        ui.ctx().accesskit_node_builder(resp.id, |builder| {
            builder.set_toggled(egui::accesskit::Toggled::Mixed);
        });
    }

    if ui.is_rect_visible(rect) {
        let radius = CornerRadius::same(RADIUS_MD as u8);
        let hovered = resp.hovered();
        let pressed = resp.is_pointer_button_down_on();
        let fill = match state {
            Some(true) => {
                if pressed {
                    p.pressed
                } else {
                    p.accent_subtle
                }
            }
            Some(false) => {
                if pressed {
                    p.pressed
                } else if hovered {
                    p.hover
                } else {
                    Color32::TRANSPARENT
                }
            }
            None => {
                if pressed {
                    p.pressed
                } else if hovered {
                    p.hover
                } else {
                    p.raised
                }
            }
        };
        let stroke = match state {
            Some(true) => egui::Stroke::new(1.0, p.accent),
            None => egui::Stroke::new(1.0, p.border),
            Some(false) => egui::Stroke::NONE,
        };
        ui.painter()
            .rect(rect, radius, fill, stroke, egui::StrokeKind::Inside);

        let text_color = match state {
            Some(true) => p.accent_text,
            Some(false) => {
                if hovered || pressed {
                    p.text
                } else {
                    p.text_secondary
                }
            }
            None => p.text,
        };
        let text_pos = pos2(
            rect.center().x - galley.size().x * 0.5,
            rect.center().y - galley.size().y * 0.5,
        );
        ui.painter().galley(text_pos, galley, text_color);

        if state.is_none() {
            let bar = egui::Rect::from_center_size(
                pos2(rect.center().x, rect.bottom() - 3.5),
                vec2(10.0, 2.0),
            );
            ui.painter()
                .rect_filled(bar, CornerRadius::same(1), p.accent);
        }
    }

    resp.on_hover_text(tooltip)
        .on_hover_cursor(CursorIcon::PointingHand)
}

#[cfg(test)]
mod tests {
    use super::*;
    use coursecraft_render::ThemeMode;
    use egui::{Context, Pos2, RawInput, Rect};

    #[test]
    fn styled_combo_enforces_min_width_and_twelve_item_popup_height() {
        let ctx = Context::default();
        coursecraft_render::apply_style(&ctx, ThemeMode::Dark);
        let mut selected = 0usize;

        let run_frame = |ctx: &Context, selected: &mut usize, click_pos: Option<Pos2>| {
            let mut events = Vec::new();
            if let Some(pos) = click_pos {
                events.push(egui::Event::PointerMoved(pos));
                events.push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                });
                events.push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                });
            }
            let raw = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1024.0, 600.0))),
                events,
                ..Default::default()
            };
            let mut rect_out = Rect::NOTHING;
            let mut popup_layers = Vec::new();
            let output = ctx.run_ui(raw, |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    let resp = styled_combo(ui, "test_styled_combo", 30.0)
                        .selected_text(format!("Item {selected}"))
                        .show_ui(ui, |ui| {
                            for i in 0..20 {
                                ui.selectable_value(selected, i, format!("Item {i}"));
                            }
                        });
                    rect_out = resp.response.rect;
                });
                popup_layers = crate::test_harness::capture_layers(ui.ctx());
            });
            output.drop_without_applying_deltas();
            (rect_out, popup_layers)
        };

        let (combo_rect, _) = run_frame(&ctx, &mut selected, None);
        assert!(
            combo_rect.width() >= COMBO_MIN_WIDTH - 1.0,
            "styled_combo must enforce min width >= {COMBO_MIN_WIDTH}, got {}",
            combo_rect.width()
        );

        // Click to open popup and measure visible items on the next frame.
        let (_, _) = run_frame(&ctx, &mut selected, Some(combo_rect.center()));
        let (_, layers) = run_frame(&ctx, &mut selected, None);
        assert!(egui::Popup::is_any_open(&ctx), "combo popup should be open");

        let mut visible_items = 0usize;
        let mut popup_rect = Rect::NOTHING;
        for (layer_id, shapes) in &layers {
            if layer_id.order == egui::Order::Foreground {
                for clipped in shapes {
                    if let egui::Shape::Text(t) = &clipped.shape {
                        let text = t.galley.text();
                        if text.starts_with("Item ") {
                            let y = t.pos.y;
                            if y >= clipped.clip_rect.min.y - 1.0
                                && y + t.galley.size().y <= clipped.clip_rect.max.y + 1.0
                            {
                                visible_items += 1;
                                popup_rect = clipped.clip_rect;
                            }
                        }
                    }
                }
            }
        }
        assert!(
            visible_items >= 12,
            "styled_combo popup must display >= 12 items without scrolling, got {visible_items} (popup_rect={popup_rect:?})"
        );
    }

    #[test]
    fn modal_backdrop_color_meets_ux15_alpha_thresholds() {
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            let color = modal_backdrop_color(mode);
            assert_eq!(color.r(), 0);
            assert_eq!(color.g(), 0);
            assert_eq!(color.b(), 0);
            assert!(
                color.a() >= 0x99,
                "modal_backdrop_color({mode:?}) alpha {} must be >= 0x99",
                color.a()
            );
        }
    }

    #[test]
    fn mixed_toggle_supports_off_on_mixed_with_tooltip_and_accesskit_state() {
        let ctx = Context::default();
        ctx.enable_accesskit();
        coursecraft_render::apply_style(&ctx, ThemeMode::Dark);
        let raw = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 400.0))),
            ..Default::default()
        };
        let output = ctx.run_ui(raw, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                ui.horizontal(|ui| {
                    mixed_toggle(ui, "B", Some(false), "Bold off");
                    mixed_toggle(ui, "I", Some(true), "Italic on");
                    mixed_toggle(ui, "U", None, "Underline mixed");
                });
            });
        });
        let nodes = crate::test_harness::extract_widget_nodes(
            output.platform_output.accesskit_update.as_ref(),
        );
        output.drop_without_applying_deltas();

        let off = nodes
            .iter()
            .find(|n| n.label.as_deref() == Some("Bold off"))
            .expect("Off toggle node");
        assert_eq!(off.toggled, Some(egui::accesskit::Toggled::False));

        let on = nodes
            .iter()
            .find(|n| n.label.as_deref() == Some("Italic on"))
            .expect("On toggle node");
        assert_eq!(on.toggled, Some(egui::accesskit::Toggled::True));

        let mixed = nodes
            .iter()
            .find(|n| n.label.as_deref() == Some("Underline mixed"))
            .expect("Mixed toggle node");
        assert_eq!(mixed.toggled, Some(egui::accesskit::Toggled::Mixed));
    }
}
