//! Eyedropper (pick-a-colour-from-the-window) support shared by CourseCraft's colour fields.
//!
//! Pick mode lives in egui temp data so the free-standing colour swatch and the
//! editor frame loop can both reach it. In egui 0.37 screenshots arrive through
//! a [`egui::ScreenshotCallback`] sent with [`egui::Context::request_screenshot`]
//! rather than through an input event, so the callback stores the newest image
//! in temp data for the frame loop to sample.

use egui::{Color32, ColorImage, Context, Event, Key, Order, PointerButton, Pos2, vec2};
use std::sync::Arc;

/// Stable key for one inspector field, derived from the colour picker's `id_salt`.
pub(crate) fn field_key(id_salt: &impl std::fmt::Debug) -> String {
    format!("{id_salt:?}")
}

#[derive(Clone, Debug, Default)]
struct PickState {
    /// `field_key` of the field whose picker started the pick, if pick mode is on.
    field: Option<String>,
    /// Colour sampled by a click, waiting for that field's picker to apply it.
    pending: Option<(String, Color32)>,
    /// Remaining frames in which a primary press/release must be swallowed so
    /// the picking click cannot also select or deselect an object on the canvas.
    swallow_primary: u8,
}

fn state_id() -> egui::Id {
    egui::Id::unique("coursecraft_eyedropper_state_v1")
}

/// The latest screenshot is kept under its own id: the callback can fire from
/// the integration thread and must not overwrite the rest of the pick state.
fn screenshot_id() -> egui::Id {
    egui::Id::unique("coursecraft_eyedropper_screenshot_v1")
}

fn read(ctx: &Context) -> PickState {
    ctx.data(|d| d.get_temp::<PickState>(state_id()))
        .unwrap_or_default()
}

fn write(ctx: &Context, state: PickState) {
    ctx.data_mut(|d| d.insert_temp(state_id(), state));
}

pub(crate) fn is_active(ctx: &Context) -> bool {
    read(ctx).field.is_some()
}

/// True while the canvas must ignore pointer and Escape input: pick mode is
/// on, or a picking click is still being swallowed on the following frames.
pub(crate) fn swallows_canvas_input(ctx: &Context) -> bool {
    let state = read(ctx);
    state.field.is_some() || state.swallow_primary > 0
}

/// The field key currently picking, if any. Used by headless tests.
#[cfg(test)]
pub(crate) fn active_field(ctx: &Context) -> Option<String> {
    read(ctx).field
}

pub(crate) fn start_pick(ctx: &Context, field: String) {
    let mut state = read(ctx);
    state.field = Some(field);
    state.pending = None;
    write(ctx, state);
    ctx.data_mut(|d| d.remove_temp::<Arc<ColorImage>>(screenshot_id()));
}

pub(crate) fn cancel_pick(ctx: &Context) {
    write(ctx, PickState::default());
    ctx.data_mut(|d| d.remove_temp::<Arc<ColorImage>>(screenshot_id()));
}

/// Store a screenshot. Called by the egui callback and by headless tests, which
/// have no real viewport to capture.
pub(crate) fn store_screenshot(ctx: &Context, image: Arc<ColorImage>) {
    ctx.data_mut(|d| d.insert_temp(screenshot_id(), image));
}

fn screenshot(ctx: &Context) -> Option<Arc<ColorImage>> {
    ctx.data(|d| d.get_temp::<Arc<ColorImage>>(screenshot_id()))
}

/// Take the colour a click sampled, if it belongs to `field`.
pub(crate) fn take_pending(ctx: &Context, field: &str) -> Option<Color32> {
    let mut state = read(ctx);
    let color = state
        .pending
        .take()
        .filter(|(f, _)| f == field)
        .map(|(_, c)| c);
    if color.is_some() {
        write(ctx, state);
    }
    color
}

/// Sample the viewport image at a pointer position given in egui points.
///
/// `pos` is multiplied by `pixels_per_point` to get a physical pixel, rounded
/// to the nearest pixel and clamped to the image. Returns `None` when there is
/// no image to sample or the inputs are not finite.
pub(crate) fn sample_color(
    image: &ColorImage,
    pos: Pos2,
    pixels_per_point: f32,
) -> Option<Color32> {
    let width = image.width();
    let height = image.height();
    if width == 0 || height == 0 || image.pixels.is_empty() {
        return None;
    }
    if !pos.x.is_finite() || !pos.y.is_finite() {
        return None;
    }
    let scale = pixels_per_point;
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let x = ((pos.x * scale).round() as isize).clamp(0, width as isize - 1);
    let y = ((pos.y * scale).round() as isize).clamp(0, height as isize - 1);
    let index = y * width as isize + x;
    if index < 0 {
        return None;
    }
    image.pixels.get(index as usize).copied()
}

/// Run once per frame, before the canvas and inspector are laid out.
///
/// Requests a fresh screenshot, consumes Escape and the picking click so they
/// cannot reach the canvas, and records the sampled colour for the field's
/// picker to apply later in the same frame.
pub(crate) fn process_frame(ctx: &Context) {
    let state = read(ctx);
    let mut next = state.clone();
    let active = state.field.clone();
    let shot = screenshot(ctx);
    let scale = ctx.pixels_per_point();
    let swallow = state.swallow_primary;
    let mut escape = false;
    let mut click_pos = None;

    ctx.input_mut(|input| {
        input.events.retain(|event| match event {
            Event::Key {
                key: Key::Escape,
                pressed: true,
                ..
            } if active.is_some() => {
                escape = true;
                false
            }
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: true,
                ..
            } if active.is_some() => {
                click_pos = Some(*pos);
                false
            }
            Event::PointerButton {
                button: PointerButton::Primary,
                ..
            } if active.is_some() || swallow > 0 => false,
            _ => true,
        });
    });

    next.swallow_primary = swallow.saturating_sub(1);

    if active.is_some() {
        let callback_ctx = ctx.clone();
        ctx.request_screenshot(move |image| {
            store_screenshot(&callback_ctx, image);
            callback_ctx.request_repaint();
        });
    }

    if escape {
        cancel_pick(ctx);
        return;
    }

    if let (Some(pos), Some(field)) = (click_pos, active)
        && let Some(sampled) = shot.as_ref().and_then(|img| sample_color(img, pos, scale))
    {
        // A picked swatch is always opaque, whatever the screenshot's alpha.
        let color = Color32::from_rgb(sampled.r(), sampled.g(), sampled.b());
        next.field = None;
        next.pending = Some((field, color));
        next.swallow_primary = 2;
    }
    write(ctx, next);
}

/// Draw the crosshair cursor and the small swatch that follows the pointer.
/// Called after the rest of the UI so the crosshair wins over widget cursors.
pub(crate) fn paint_overlay(ctx: &Context) {
    if !is_active(ctx) {
        return;
    }
    ctx.set_cursor_icon(egui::CursorIcon::Crosshair);
    let pos = ctx.input(|i| i.pointer.latest_pos());
    let (Some(pos), Some(shot)) = (pos, screenshot(ctx)) else {
        return;
    };
    let Some(color) = sample_color(&shot, pos, ctx.pixels_per_point()) else {
        return;
    };

    egui::Area::new(egui::Id::unique("coursecraft_eyedropper_swatch_v1"))
        .order(Order::Tooltip)
        .fixed_pos(pos + vec2(16.0, 16.0))
        .interactable(false)
        .show(ctx, |ui| {
            let p = coursecraft_render::palette(ctx);
            egui::Frame::new()
                .fill(p.panel)
                .corner_radius(egui::CornerRadius::same(6))
                .inner_margin(egui::Margin::symmetric(8, 6))
                .stroke(egui::Stroke::new(1.0, p.border_strong))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        let (rect, _) =
                            ui.allocate_exact_size(vec2(18.0, 18.0), egui::Sense::hover());
                        ui.painter()
                            .rect_filled(rect, egui::CornerRadius::same(4), color);
                        ui.painter().rect_stroke(
                            rect,
                            egui::CornerRadius::same(4),
                            egui::Stroke::new(1.0, p.border_strong),
                            egui::StrokeKind::Outside,
                        );
                        ui.label(
                            egui::RichText::new(crate::properties::color_to_hex(color)).monospace(),
                        );
                    });
                });
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{ColorImage, pos2};

    fn image(w: usize, h: usize, color: Color32) -> ColorImage {
        ColorImage::new([w, h], vec![color; w * h])
    }

    #[test]
    fn sample_color_reads_the_exact_pixel_at_one_and_two_times_scale() {
        let one = image(2, 2, Color32::RED);
        assert_eq!(sample_color(&one, pos2(0.4, 1.6), 1.0), Some(Color32::RED));

        let mut two = image(4, 4, Color32::BLUE);
        two.pixels[2 * 4 + 2] = Color32::GREEN;
        assert_eq!(
            sample_color(&two, pos2(1.0, 1.0), 2.0),
            Some(Color32::GREEN)
        );
    }

    #[test]
    fn sample_color_clamps_to_the_image_edges() {
        let img = image(3, 3, Color32::YELLOW);
        assert_eq!(
            sample_color(&img, pos2(-50.0, -50.0), 1.0),
            Some(Color32::YELLOW)
        );
        assert_eq!(
            sample_color(&img, pos2(500.0, 500.0), 1.0),
            Some(Color32::YELLOW)
        );
    }

    #[test]
    fn sample_color_returns_none_for_an_empty_image() {
        assert_eq!(
            sample_color(&image(0, 0, Color32::WHITE), pos2(0.0, 0.0), 1.0),
            None
        );
    }
}
