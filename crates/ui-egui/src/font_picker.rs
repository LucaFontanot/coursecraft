//! Font picker popup for the text Format panel.
//!
//! Renders a single virtualised vertical scroll list with a sticky search bar
//! at the top, "In this course", "Recent", and "On this computer" (A–Z)
//! sections, and a pinned "Add font…" button at the bottom.

use std::collections::HashSet;

use coursecraft_render::{CONTROL_COMPACT, RADIUS_SM, SIZE_BODY, SIZE_SMALL, palette};
use egui::{
    Align, CornerRadius, FontId, Key, Popup, PopupCloseBehavior, Response, Sense, Stroke,
    StrokeKind, TextEdit, Ui, pos2, vec2,
};

/// Fixed popup width in points.
pub const FONT_PICKER_WIDTH: f32 = 380.0;
/// Maximum popup height in points (clamped to the window).
pub const FONT_PICKER_MAX_HEIGHT: f32 = 520.0;
/// Maximum number of recently picked fonts remembered in the session.
const MAX_RECENT_FONTS: usize = 8;

/// Persistent UI state for one font picker instance.
#[derive(Debug, Clone, Default)]
pub struct FontPickerState {
    pub was_open: bool,
    pub highlighted: Option<usize>,
    pub last_query: String,
    pub pending_scroll_offset: Option<f32>,
    pub scroll_apply_frames: u8,
    pub visible_range: std::ops::Range<usize>,
}

/// Which section a font row belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontSection {
    Course,
    Recent,
    System,
}

/// One virtualised row inside the font picker scroll area.
#[derive(Debug, Clone)]
enum FontRow {
    Header(&'static str),
    Font {
        name: String,
        section: FontSection,
        embedding_restricted: bool,
        is_current: bool,
    },
    Status(&'static str),
}

/// Outcome of showing the font picker for one frame.
#[derive(Debug, Clone)]
pub enum FontPickerAction {
    /// A course or recent font already available (or Inter) was chosen.
    PickCourseOrRecent(String),
    /// An installed system font was chosen and must be embedded/applied.
    PickSystem(String),
    /// User clicked "Add font…".
    AddFont,
}

/// Record a chosen font family in `recent_fonts` (newest first, deduplicated).
pub fn remember_recent_font(recent_fonts: &mut Vec<String>, family: &str) {
    let trimmed = family.trim();
    if trimmed.is_empty() {
        return;
    }
    recent_fonts.retain(|f| !f.eq_ignore_ascii_case(trimmed));
    recent_fonts.insert(0, trimmed.to_owned());
    recent_fonts.truncate(MAX_RECENT_FONTS);
}

fn combo_button(ui: &mut Ui, shown_font: &str, selected_font: FontId) -> Response {
    let width = ui.available_width().max(48.0);
    let (rect, response) = ui.allocate_exact_size(vec2(width, CONTROL_COMPACT), Sense::click());
    let p = palette(ui.ctx());
    let fill = if response.hovered() { p.hover } else { p.input };
    ui.painter()
        .rect_filled(rect, CornerRadius::same(RADIUS_SM as u8), fill);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(RADIUS_SM as u8),
        Stroke::new(1.0, p.border_strong),
        StrokeKind::Inside,
    );

    let text_max_w = (rect.width() - 26.0).max(8.0);
    let mut job = egui::text::LayoutJob::single_section(
        shown_font.to_owned(),
        egui::TextFormat {
            font_id: selected_font,
            color: p.text,
            ..Default::default()
        },
    );
    job.wrap = egui::text::TextWrapping {
        max_width: text_max_w,
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    let galley = ui.painter().layout_job(job);
    ui.painter().galley(
        pos2(rect.left() + 8.0, rect.center().y - galley.size().y * 0.5),
        galley,
        p.text,
    );

    // Small downward triangle indicator on the right.
    let cx = rect.right() - 10.0;
    let cy = rect.center().y;
    ui.painter().add(egui::Shape::convex_polygon(
        vec![
            pos2(cx - 3.5, cy - 1.5),
            pos2(cx + 3.5, cy - 1.5),
            pos2(cx, cy + 2.5),
        ],
        p.text_secondary,
        Stroke::NONE,
    ));

    response
}

fn build_rows(
    shown_font: &str,
    font_families: &[String],
    recent_fonts: &[String],
    system_fonts: Option<&[crate::SystemFontChoice]>,
    query: &str,
) -> Vec<FontRow> {
    let needle = query.trim().to_lowercase();
    let matches = |name: &str| needle.is_empty() || name.to_lowercase().contains(&needle);

    let mut rows = Vec::new();
    let mut restricted_lookup: HashSet<String> = HashSet::new();
    if let Some(choices) = system_fonts {
        for choice in choices {
            if choice.embedding_restricted {
                restricted_lookup.insert(choice.name.to_ascii_lowercase());
            }
        }
    }

    // 1. "In this course": Inter + course-embedded families (deduplicated).
    let mut course_names: Vec<String> = Vec::with_capacity(1 + font_families.len());
    course_names.push("Inter".to_owned());
    for fam in font_families {
        if !fam.eq_ignore_ascii_case("Inter")
            && !fam.eq_ignore_ascii_case("Open Sans")
            && !course_names.iter().any(|n| n.eq_ignore_ascii_case(fam))
        {
            course_names.push(fam.clone());
        }
    }
    let matching_course: Vec<String> = course_names
        .into_iter()
        .filter(|name| matches(name))
        .collect();
    if !matching_course.is_empty() {
        rows.push(FontRow::Header("In this course"));
        for name in matching_course {
            let restricted = restricted_lookup.contains(&name.to_ascii_lowercase());
            let is_current = name.eq_ignore_ascii_case(shown_font);
            rows.push(FontRow::Font {
                name,
                section: FontSection::Course,
                embedding_restricted: restricted,
                is_current,
            });
        }
    }

    // 2. "Recent"
    let matching_recent: Vec<&String> = recent_fonts.iter().filter(|name| matches(name)).collect();
    if !matching_recent.is_empty() {
        rows.push(FontRow::Header("Recent"));
        for name in matching_recent {
            let restricted = restricted_lookup.contains(&name.to_ascii_lowercase());
            let is_current = name.eq_ignore_ascii_case(shown_font);
            rows.push(FontRow::Font {
                name: name.clone(),
                section: FontSection::Recent,
                embedding_restricted: restricted,
                is_current,
            });
        }
    }

    // 3. "On this computer" (A–Z)
    match system_fonts {
        None => {
            rows.push(FontRow::Header("On this computer"));
            rows.push(FontRow::Status("Loading fonts…"));
        }
        Some([]) => {
            rows.push(FontRow::Header("On this computer"));
            rows.push(FontRow::Status("Not available here"));
        }
        Some(choices) => {
            let mut filtered: Vec<&crate::SystemFontChoice> = choices
                .iter()
                .filter(|choice| matches(&choice.name))
                .filter(|choice| {
                    !choice.name.eq_ignore_ascii_case("Inter")
                        && !font_families
                            .iter()
                            .any(|name| name.eq_ignore_ascii_case(&choice.name))
                })
                .collect();
            filtered.sort_by_cached_key(|c| c.name.to_lowercase());
            if !filtered.is_empty() {
                rows.push(FontRow::Header("On this computer"));
                for choice in filtered {
                    let is_current = choice.name.eq_ignore_ascii_case(shown_font);
                    rows.push(FontRow::Font {
                        name: choice.name.clone(),
                        section: FontSection::System,
                        embedding_restricted: choice.embedding_restricted,
                        is_current,
                    });
                }
            }
        }
    }

    if rows.is_empty() {
        rows.push(FontRow::Status("No matching fonts"));
    }

    rows
}

fn is_font_row(row: &FontRow) -> bool {
    matches!(row, FontRow::Font { .. })
}

fn next_font_row(rows: &[FontRow], from: Option<usize>, delta: isize) -> Option<usize> {
    if rows.is_empty() {
        return None;
    }
    if delta >= 0 {
        let start = from.map_or(0, |i| i.saturating_add(1));
        (start..rows.len())
            .find(|&i| is_font_row(&rows[i]))
            .or(from)
    } else {
        let end = from.unwrap_or(rows.len());
        (0..end).rfind(|&i| is_font_row(&rows[i])).or(from)
    }
}

/// Resolve a preview font for a visible row, falling back to the UI font if
/// loading a new system font exceeds the per-frame budget.
fn preview_font_for_row(
    ctx: &egui::Context,
    name: &str,
    section: FontSection,
    #[cfg_attr(target_arch = "wasm32", allow(unused_variables))] loads_this_frame: &mut usize,
    #[cfg_attr(target_arch = "wasm32", allow(unused_variables))] budget_exhausted: &mut bool,
) -> FontId {
    if section != FontSection::System || name.eq_ignore_ascii_case("Inter") {
        return coursecraft_render::slide_font_named(ctx, Some(name), false, false, SIZE_BODY);
    }

    if *budget_exhausted || *loads_this_frame >= 2 {
        return FontId::proportional(SIZE_BODY);
    }

    #[cfg(not(target_arch = "wasm32"))]
    let start = std::time::Instant::now();
    *loads_this_frame += 1;
    let font = coursecraft_render::slide_font_named(ctx, Some(name), false, false, SIZE_BODY);
    #[cfg(not(target_arch = "wasm32"))]
    if start.elapsed().as_millis() > 3 {
        *budget_exhausted = true;
    }
    font
}

/// Render the Font picker button and its large, single-scroll popup.
#[allow(clippy::too_many_arguments)]
pub fn show_font_picker(
    ui: &mut Ui,
    id_prefix: &str,
    shown_font: &str,
    font_families: &[String],
    recent_fonts: &mut Vec<String>,
    system_fonts: Option<&[crate::SystemFontChoice]>,
    system_fonts_list_requested: &mut bool,
    search_query: &mut String,
    font_requests: &mut Vec<crate::EditorRequest>,
) -> Option<FontPickerAction> {
    let selected_font =
        coursecraft_render::slide_font_named(ui.ctx(), Some(shown_font), false, false, SIZE_BODY);
    let button_resp = combo_button(ui, shown_font, selected_font);
    let popup_id = ui.make_persistent_id(("prop_font_popup", id_prefix));
    let state_id = ui.make_persistent_id(("prop_font_picker_state", id_prefix));
    let is_open = Popup::is_id_open(ui.ctx(), popup_id);

    let mut state: FontPickerState = ui.ctx().data(|d| d.get_temp(state_id)).unwrap_or_default();

    let just_opened = !state.was_open && is_open;
    let just_closed = state.was_open && !is_open;
    state.was_open = is_open;

    if just_closed {
        search_query.clear();
        state.highlighted = None;
        state.last_query.clear();
        state.pending_scroll_offset = None;
        state.scroll_apply_frames = 0;
    }

    let mut action: Option<FontPickerAction> = None;

    Popup::menu(&button_resp)
        .id(popup_id)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            if system_fonts.is_none() && !*system_fonts_list_requested {
                *system_fonts_list_requested = true;
                font_requests.push(crate::EditorRequest::ListSystemFonts);
            }

            let screen = ui.ctx().content_rect();
            let popup_w = FONT_PICKER_WIDTH.min((screen.width() - 16.0).max(220.0));
            let popup_max_h = FONT_PICKER_MAX_HEIGHT.min((screen.height() - 16.0).max(200.0));
            ui.set_min_width(popup_w);
            ui.set_max_width(popup_w);
            ui.set_max_height(popup_max_h);

            // Sticky search field at the top.
            let search_resp = ui.add(
                TextEdit::singleline(search_query)
                    .hint_text("Search fonts")
                    .desired_width(f32::INFINITY)
                    .id_salt(("font_picker_search", id_prefix)),
            );
            if just_opened {
                search_resp.request_focus();
            }
            ui.add_space(4.0);

            let rows = build_rows(
                shown_font,
                font_families,
                recent_fonts,
                system_fonts,
                search_query,
            );
            let row_height = ui.spacing().interact_size.y.max(22.0);
            let row_stride = row_height + ui.spacing().item_spacing.y;
            // Reserve vertical space for sticky search (~28px), spacing (4px),
            // bottom separator + "Add font…" button (~34px), and popup margins (~18px).
            let scroll_max_h = (popup_max_h - 84.0).max(100.0);

            if just_opened {
                let current_idx = rows
                    .iter()
                    .position(|r| {
                        matches!(
                            r,
                            FontRow::Font {
                                is_current: true,
                                ..
                            }
                        )
                    })
                    .or_else(|| rows.iter().position(is_font_row));
                state.highlighted = current_idx;
                state.last_query = search_query.clone();
                if let Some(idx) = current_idx {
                    #[allow(clippy::cast_precision_loss)]
                    let center_y = idx as f32 * row_stride;
                    state.pending_scroll_offset =
                        Some((center_y - (scroll_max_h - row_stride) * 0.5).max(0.0));
                    // Apply for 2 frames so ScrollArea has content_size on frame 2.
                    state.scroll_apply_frames = 2;
                }
            } else if *search_query != state.last_query {
                state.last_query = search_query.clone();
                state.highlighted = rows.iter().position(is_font_row);
                state.pending_scroll_offset = Some(0.0);
                state.scroll_apply_frames = 1;
            }

            // Keyboard navigation: Esc closes, Up/Down move highlight, Enter picks.
            let (esc, down, up, enter) = ui.input(|i| {
                (
                    i.key_pressed(Key::Escape),
                    i.key_pressed(Key::ArrowDown),
                    i.key_pressed(Key::ArrowUp),
                    i.key_pressed(Key::Enter),
                )
            });

            if esc {
                ui.close();
            } else {
                if down {
                    let next = next_font_row(&rows, state.highlighted, 1);
                    if let Some(idx) = next {
                        state.highlighted = Some(idx);
                        if idx + 1 > state.visible_range.end {
                            #[allow(clippy::cast_precision_loss)]
                            let target = ((idx + 1) as f32 * row_stride - scroll_max_h).max(0.0);
                            state.pending_scroll_offset = Some(target);
                            state.scroll_apply_frames = 1;
                        }
                    }
                } else if up {
                    let prev = next_font_row(&rows, state.highlighted, -1);
                    if let Some(idx) = prev {
                        state.highlighted = Some(idx);
                        if idx < state.visible_range.start {
                            #[allow(clippy::cast_precision_loss)]
                            let target = idx as f32 * row_stride;
                            state.pending_scroll_offset = Some(target);
                            state.scroll_apply_frames = 1;
                        }
                    }
                }

                if enter
                    && let Some(idx) = state.highlighted
                    && let Some(FontRow::Font { name, section, .. }) = rows.get(idx)
                {
                    let chosen_name = name.clone();
                    remember_recent_font(recent_fonts, &chosen_name);
                    action = Some(match section {
                        FontSection::System => FontPickerAction::PickSystem(chosen_name),
                        FontSection::Course | FontSection::Recent => {
                            if font_families
                                .iter()
                                .any(|f| f.eq_ignore_ascii_case(&chosen_name))
                                || chosen_name.eq_ignore_ascii_case("Inter")
                            {
                                FontPickerAction::PickCourseOrRecent(chosen_name)
                            } else {
                                FontPickerAction::PickSystem(chosen_name)
                            }
                        }
                    });
                    ui.close();
                }
            }

            // One single virtualised vertical scroll area.
            let mut scroll = egui::ScrollArea::vertical()
                .id_salt(("font_picker_scroll", id_prefix))
                .max_height(scroll_max_h)
                .auto_shrink([false, true]);
            if let Some(offset) = state.pending_scroll_offset {
                scroll = scroll.vertical_scroll_offset(offset);
                if state.scroll_apply_frames > 1 {
                    state.scroll_apply_frames -= 1;
                    ui.ctx().request_repaint();
                } else {
                    state.pending_scroll_offset = None;
                    state.scroll_apply_frames = 0;
                }
            }

            let mut loads_this_frame = 0usize;
            let mut budget_exhausted = false;
            let p = palette(ui.ctx());

            scroll.show_rows(ui, row_height, rows.len(), |ui, row_range| {
                state.visible_range = row_range.clone();
                let row_w = ui.available_width();
                for idx in row_range {
                    let row = &rows[idx];
                    match row {
                        FontRow::Header(title) => {
                            let (rect, _) =
                                ui.allocate_exact_size(vec2(row_w, row_height), Sense::hover());
                            let galley = ui.painter().layout_no_wrap(
                                (*title).to_owned(),
                                FontId::proportional(SIZE_SMALL),
                                p.text_secondary,
                            );
                            ui.painter().galley(
                                pos2(rect.left() + 6.0, rect.center().y - galley.size().y * 0.5),
                                galley,
                                p.text_secondary,
                            );
                        }
                        FontRow::Status(msg) => {
                            let (rect, _) =
                                ui.allocate_exact_size(vec2(row_w, row_height), Sense::hover());
                            let galley = ui.painter().layout_no_wrap(
                                (*msg).to_owned(),
                                FontId::proportional(SIZE_SMALL),
                                p.text_secondary,
                            );
                            ui.painter().galley(
                                pos2(rect.left() + 8.0, rect.center().y - galley.size().y * 0.5),
                                galley,
                                p.text_secondary,
                            );
                        }
                        FontRow::Font {
                            name,
                            section,
                            embedding_restricted,
                            is_current,
                        } => {
                            let (rect, resp) =
                                ui.allocate_exact_size(vec2(row_w, row_height), Sense::click());
                            let is_highlighted = state.highlighted == Some(idx);
                            if *is_current || is_highlighted {
                                ui.painter().rect_filled(
                                    rect,
                                    CornerRadius::same(RADIUS_SM as u8),
                                    p.accent_hover,
                                );
                            } else if resp.hovered() {
                                ui.painter().rect_filled(
                                    rect,
                                    CornerRadius::same(RADIUS_SM as u8),
                                    p.hover,
                                );
                            }

                            let badge_reserve = if *embedding_restricted { 68.0 } else { 12.0 };
                            let text_max_w = (rect.width() - badge_reserve - 8.0).max(20.0);
                            let font = preview_font_for_row(
                                ui.ctx(),
                                name,
                                *section,
                                &mut loads_this_frame,
                                &mut budget_exhausted,
                            );
                            let mut job = egui::text::LayoutJob::single_section(
                                name.clone(),
                                egui::TextFormat {
                                    font_id: font,
                                    color: p.text,
                                    ..Default::default()
                                },
                            );
                            job.wrap = egui::text::TextWrapping {
                                max_width: text_max_w,
                                max_rows: 1,
                                break_anywhere: true,
                                overflow_character: Some('…'),
                            };
                            let galley = ui.painter().layout_job(job);
                            ui.painter().galley(
                                pos2(rect.left() + 8.0, rect.center().y - galley.size().y * 0.5),
                                galley,
                                p.text,
                            );

                            if *embedding_restricted {
                                let badge_galley = ui.painter().layout_no_wrap(
                                    "restricted".to_owned(),
                                    FontId::proportional(SIZE_SMALL),
                                    p.text_secondary,
                                );
                                ui.painter().galley(
                                    pos2(
                                        rect.right() - badge_galley.size().x - 8.0,
                                        rect.center().y - badge_galley.size().y * 0.5,
                                    ),
                                    badge_galley,
                                    p.text_secondary,
                                );
                            }

                            if resp.clicked() {
                                remember_recent_font(recent_fonts, name);
                                action = Some(match section {
                                    FontSection::System => {
                                        FontPickerAction::PickSystem(name.clone())
                                    }
                                    FontSection::Course | FontSection::Recent => {
                                        if font_families
                                            .iter()
                                            .any(|f| f.eq_ignore_ascii_case(name))
                                            || name.eq_ignore_ascii_case("Inter")
                                        {
                                            FontPickerAction::PickCourseOrRecent(name.clone())
                                        } else {
                                            FontPickerAction::PickSystem(name.clone())
                                        }
                                    }
                                });
                                ui.close();
                            }
                        }
                    }
                }
            });

            // Pinned footer outside the scroll area.
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Add font…").clicked() {
                    action = Some(FontPickerAction::AddFont);
                    ui.close();
                }
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new("↑↓ Enter  Esc")
                            .size(SIZE_SMALL)
                            .color(p.text_secondary),
                    );
                });
            });
        });

    ui.ctx().data_mut(|d| d.insert_temp(state_id, state));
    action
}
