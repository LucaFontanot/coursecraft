//! File > Slide Size: pick a preset or custom size and how content follows.

use crate::dialog::{Dialog, Form};
use crate::widgets::styled_combo;
use crate::{Editor, EditorRequest};
use coursecraft_course::{Course, MAX_OBJECT_FRAME_SIZE, Object, ObjectKind, Rect};
use egui::{Color32, DragValue, Pos2, StrokeKind, pos2, vec2};
use serde_json::json;

/// Default slide size for new courses (16:9 HD).
pub const DEFAULT_SLIDE_SIZE: (u32, u32) = (1280, 720);

/// Minimum and maximum slide width in pixels.
pub const SLIDE_W_RANGE: std::ops::RangeInclusive<u32> = 100..=7680;

/// Minimum and maximum slide height in pixels.
pub const SLIDE_H_RANGE: std::ops::RangeInclusive<u32> = 100..=4320;

/// Whether `(w, h)` is within the allowed slide size bounds.
#[must_use]
pub fn is_valid_slide_size(w: u32, h: u32) -> bool {
    SLIDE_W_RANGE.contains(&w) && SLIDE_H_RANGE.contains(&h)
}

/// A named slide size preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlideSizePreset {
    pub label: &'static str,
    pub w: u32,
    pub h: u32,
}

/// Shared slide size presets used by Slide Size and Preferences.
pub const PRESETS: [SlideSizePreset; 7] = [
    SlideSizePreset {
        label: "16:9 Full HD (1920 × 1080)",
        w: 1920,
        h: 1080,
    },
    SlideSizePreset {
        label: "16:9 HD (1280 × 720)",
        w: 1280,
        h: 720,
    },
    SlideSizePreset {
        label: "16:10 Widescreen (1280 × 800)",
        w: 1280,
        h: 800,
    },
    SlideSizePreset {
        label: "4:3 Standard (960 × 720)",
        w: 960,
        h: 720,
    },
    SlideSizePreset {
        label: "4:3 Classic (1024 × 768)",
        w: 1024,
        h: 768,
    },
    SlideSizePreset {
        label: "1:1 Square (1080 × 1080)",
        w: 1080,
        h: 1080,
    },
    SlideSizePreset {
        label: "9:16 Portrait (1080 × 1920)",
        w: 1080,
        h: 1920,
    },
];

fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

/// Format the aspect ratio of `(w, h)` as a reduced integer ratio or decimal ratio.
#[must_use]
pub fn format_aspect_ratio(w: u32, h: u32) -> String {
    if w == 0 || h == 0 {
        return "—".to_owned();
    }
    let g = gcd(w, h);
    let (rw, rh) = match (w / g, h / g) {
        (8, 5) => (16, 10),
        (5, 8) => (10, 16),
        pair => pair,
    };
    if rw <= 32 && rh <= 32 {
        format!("{rw}:{rh}")
    } else if w >= h {
        format!("{:.2}:1", f64::from(w) / f64::from(h))
    } else {
        format!("1:{:.2}", f64::from(h) / f64::from(w))
    }
}

/// How existing slide content adapts when the slide size changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResizeModeChoice {
    #[default]
    Scale,
    Fill,
    Keep,
}

impl ResizeModeChoice {
    /// Command parameter value for `course.set_slide_size`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scale => "scale",
            Self::Fill => "fill",
            Self::Keep => "keep",
        }
    }
}

/// Cached key for the course-wide resize impact summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImpactCacheKey {
    pub revision: u64,
    pub w: u32,
    pub h: u32,
    pub mode: ResizeModeChoice,
}

/// Course-wide impact summary for a candidate slide size and resize mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlideSizeImpact {
    pub outside: usize,
    pub clipped: usize,
    pub same_size: bool,
    pub same_aspect: bool,
    pub scale_pct: u32,
    pub hint: String,
}

/// Dialog state.
#[derive(Debug, Clone, PartialEq)]
pub struct SlideSizeState {
    pub open: bool,
    pub w: u32,
    pub h: u32,
    /// How existing content adapts to the new slide size.
    pub mode: ResizeModeChoice,
    /// Whether editing W or H preserves the locked aspect ratio.
    pub lock_aspect: bool,
    /// Locked `w / h` ratio captured when opening or toggling lock ON.
    pub locked_ratio: f64,
    /// Inline confirmation after saving the current dimensions as default.
    pub saved_default_notice: bool,
    /// Cached course-wide impact summary keyed on `(revision, w, h, mode)`.
    pub impact_cache: Option<(ImpactCacheKey, SlideSizeImpact)>,
    /// Number of times the impact summary was recomputed (for idle-frame caching tests).
    #[cfg(test)]
    pub impact_recomputes: usize,
}

impl Default for SlideSizeState {
    fn default() -> Self {
        Self {
            open: false,
            w: 1280,
            h: 720,
            mode: ResizeModeChoice::Scale,
            lock_aspect: true,
            locked_ratio: 1280.0 / 720.0,
            saved_default_notice: false,
            impact_cache: None,
            #[cfg(test)]
            impact_recomputes: 0,
        }
    }
}

impl SlideSizeState {
    /// Whether the entered size is inside the allowed range.
    #[must_use]
    pub fn valid(&self) -> bool {
        is_valid_slide_size(self.w, self.h)
    }

    #[must_use]
    pub fn preset_index(&self) -> Option<usize> {
        PRESETS.iter().position(|p| p.w == self.w && p.h == self.h)
    }
}

impl Editor {
    /// Open the dialog on the course's current size.
    pub fn open_slide_size_dialog(&mut self) {
        let size = self.session.course.slide_size;
        let locked_ratio = if size.h > 0 {
            f64::from(size.w) / f64::from(size.h)
        } else {
            16.0 / 9.0
        };
        self.ui.slide_size_dialog = SlideSizeState {
            open: true,
            w: size.w,
            h: size.h,
            mode: ResizeModeChoice::Scale,
            lock_aspect: true,
            locked_ratio,
            saved_default_notice: false,
            impact_cache: None,
            #[cfg(test)]
            impact_recomputes: 0,
        };
    }
}

#[derive(Clone, Copy)]
struct ResizeMap {
    factor: f32,
    dx: f32,
    dy: f32,
}

impl ResizeMap {
    fn new(old_w: u32, old_h: u32, new_w: u32, new_h: u32, mode: ResizeModeChoice) -> Self {
        let ow = (old_w.max(1)) as f32;
        let oh = (old_h.max(1)) as f32;
        let nw = new_w as f32;
        let nh = new_h as f32;
        let factor = match mode {
            ResizeModeChoice::Scale => (nw / ow).min(nh / oh),
            ResizeModeChoice::Fill => (nw / ow).max(nh / oh),
            ResizeModeChoice::Keep => {
                return Self {
                    factor: 1.0,
                    dx: 0.0,
                    dy: 0.0,
                };
            }
        };
        Self {
            factor,
            dx: (nw - ow * factor) * 0.5,
            dy: (nh - oh * factor) * 0.5,
        }
    }

    fn project_frame(self, f: Rect, mode: ResizeModeChoice) -> Rect {
        if mode == ResizeModeChoice::Keep {
            return f;
        }
        Rect {
            x: (f.x * self.factor + self.dx).clamp(-100_000.0, 100_000.0),
            y: (f.y * self.factor + self.dy).clamp(-100_000.0, 100_000.0),
            w: (f.w * self.factor)
                .round()
                .clamp(1.0, MAX_OBJECT_FRAME_SIZE),
            h: (f.h * self.factor)
                .round()
                .clamp(1.0, MAX_OBJECT_FRAME_SIZE),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ObjectPlacement {
    Inside,
    Clipped,
    Outside,
}

fn rotated_aabb(frame: Rect, rotation_deg: f32) -> (f32, f32, f32, f32) {
    let cx = frame.x + frame.w * 0.5;
    let cy = frame.y + frame.h * 0.5;
    let hw = frame.w * 0.5;
    let hh = frame.h * 0.5;
    let (sin, cos) = rotation_deg.to_radians().sin_cos();
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for (lx, ly) in [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)] {
        let x = cx + lx * cos - ly * sin;
        let y = cy + lx * sin + ly * cos;
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    (min_x, max_x, min_y, max_y)
}

fn classify_placement(frame: Rect, rotation_deg: f32, new_w: f32, new_h: f32) -> ObjectPlacement {
    if ![frame.x, frame.y, frame.w, frame.h, rotation_deg]
        .iter()
        .all(|v| v.is_finite())
        || frame.w < 0.0
        || frame.h < 0.0
    {
        return ObjectPlacement::Outside;
    }
    let (min_x, max_x, min_y, max_y) = rotated_aabb(frame, rotation_deg);
    if max_x <= 0.0 || min_x >= new_w || max_y <= 0.0 || min_y >= new_h {
        ObjectPlacement::Outside
    } else if min_x >= -0.5 && max_x <= new_w + 0.5 && min_y >= -0.5 && max_y <= new_h + 0.5 {
        ObjectPlacement::Inside
    } else {
        ObjectPlacement::Clipped
    }
}

fn is_previewable_object(obj: &Object) -> bool {
    !matches!(obj.kind, ObjectKind::Audio { .. }) && obj.panel.is_none()
}

fn compute_impact(
    course: &Course,
    new_w: u32,
    new_h: u32,
    mode: ResizeModeChoice,
) -> SlideSizeImpact {
    let old_w = course.slide_size.w;
    let old_h = course.slide_size.h;
    let same_size = new_w == old_w && new_h == old_h;
    let same_aspect = u64::from(new_w) * u64::from(old_h) == u64::from(old_w) * u64::from(new_h);
    let map = ResizeMap::new(old_w, old_h, new_w, new_h, mode);
    let scale_pct = (map.factor * 100.0).round().clamp(0.0, 100_000.0) as u32;

    let hint = if same_size {
        "Current size (no change).".to_owned()
    } else if mode == ResizeModeChoice::Keep {
        "Keeps current object positions and sizes.".to_owned()
    } else if same_aspect {
        format!("Scales cleanly at {scale_pct}% with no cropping or margins.")
    } else {
        let sx = new_w as f32 / (old_w.max(1)) as f32;
        let sy = new_h as f32 / (old_h.max(1)) as f32;
        match mode {
            ResizeModeChoice::Scale => {
                if sx < sy {
                    "Adds top/bottom margins.".to_owned()
                } else {
                    "Adds left/right margins.".to_owned()
                }
            }
            ResizeModeChoice::Fill => {
                if sx > sy {
                    "Crops top/bottom edges.".to_owned()
                } else {
                    "Crops left/right edges.".to_owned()
                }
            }
            ResizeModeChoice::Keep => unreachable_hint(),
        }
    };

    let nw = new_w as f32;
    let nh = new_h as f32;
    let mut outside = 0usize;
    let mut clipped = 0usize;
    for slide in course
        .slides()
        .chain(course.masters.iter())
        .chain(course.templates.iter())
        .chain(course.banks.iter().flat_map(|bank| bank.slides.iter()))
    {
        for obj in slide
            .objects
            .iter()
            .chain(slide.layers.iter().flat_map(|l| l.objects.iter()))
        {
            if !is_previewable_object(obj) {
                continue;
            }
            let projected = map.project_frame(obj.frame, mode);
            match classify_placement(projected, obj.rotation, nw, nh) {
                ObjectPlacement::Outside => outside += 1,
                ObjectPlacement::Clipped => clipped += 1,
                ObjectPlacement::Inside => {}
            }
        }
    }

    SlideSizeImpact {
        outside,
        clipped,
        same_size,
        same_aspect,
        scale_pct,
        hint,
    }
}

fn unreachable_hint() -> String {
    String::new()
}

fn draw_placement_preview(
    ui: &mut egui::Ui,
    course: &Course,
    current_slide_id: &str,
    new_w: u32,
    new_h: u32,
    mode: ResizeModeChoice,
    p: &coursecraft_render::Palette,
) {
    let (preview_rect, _resp) = ui.allocate_exact_size(vec2(220.0, 132.0), egui::Sense::hover());
    let painter = ui.painter_at(preview_rect);
    painter.rect_filled(preview_rect, coursecraft_render::RADIUS_SM, p.canvas);
    painter.rect_stroke(
        preview_rect,
        coursecraft_render::RADIUS_SM,
        egui::Stroke::new(1.0, p.border),
        StrokeKind::Inside,
    );

    let nw = (new_w.max(1)) as f32;
    let nh = (new_h.max(1)) as f32;
    let ow = (course.slide_size.w.max(1)) as f32;
    let oh = (course.slide_size.h.max(1)) as f32;
    let map = ResizeMap::new(course.slide_size.w, course.slide_size.h, new_w, new_h, mode);

    let old_proj_x = map.dx;
    let old_proj_y = map.dy;
    let old_proj_w = ow * map.factor;
    let old_proj_h = oh * map.factor;

    let min_x = 0.0_f32.min(old_proj_x);
    let min_y = 0.0_f32.min(old_proj_y);
    let max_x = nw.max(old_proj_x + old_proj_w);
    let max_y = nh.max(old_proj_y + old_proj_h);
    let union_w = (max_x - min_x).max(1.0);
    let union_h = (max_y - min_y).max(1.0);
    let union_cx = (min_x + max_x) * 0.5;
    let union_cy = (min_y + max_y) * 0.5;

    let inner = preview_rect.shrink(10.0);
    let scale = (inner.width() / union_w).min(inner.height() / union_h);
    let to_screen = |x: f32, y: f32| -> Pos2 {
        pos2(
            inner.center().x + (x - union_cx) * scale,
            inner.center().y + (y - union_cy) * scale,
        )
    };
    let rect_to_screen = |x: f32, y: f32, w: f32, h: f32| -> egui::Rect {
        egui::Rect::from_min_max(to_screen(x, y), to_screen(x + w, y + h))
    };

    let current_slide = course.slide(current_slide_id);
    let slide_bg = current_slide
        .and_then(|s| coursecraft_render::parse_color(course.slide_background(s)))
        .unwrap_or(Color32::WHITE);

    let new_slide_rect = rect_to_screen(0.0, 0.0, nw, nh);
    painter.rect_filled(new_slide_rect, 2.0, slide_bg);
    painter.rect_stroke(
        new_slide_rect,
        2.0,
        egui::Stroke::new(1.5, p.accent),
        StrokeKind::Outside,
    );

    if let Some(slide) = current_slide {
        for obj in &slide.objects {
            if !is_previewable_object(obj) {
                continue;
            }
            let proj = map.project_frame(obj.frame, mode);
            let placement = classify_placement(proj, obj.rotation, nw, nh);
            let (fill, stroke) = match placement {
                ObjectPlacement::Inside => {
                    (p.accent.gamma_multiply(0.35), p.accent.gamma_multiply(0.75))
                }
                ObjectPlacement::Clipped => (p.warning.gamma_multiply(0.55), p.warning),
                ObjectPlacement::Outside => (p.danger.gamma_multiply(0.65), p.danger),
            };
            let obj_rect = rect_to_screen(proj.x, proj.y, proj.w, proj.h);
            painter.rect_filled(obj_rect, 2.0, fill);
            painter.rect_stroke(
                obj_rect,
                2.0,
                egui::Stroke::new(1.0, stroke),
                StrokeKind::Inside,
            );
        }
    }

    let old_rect = rect_to_screen(old_proj_x, old_proj_y, old_proj_w, old_proj_h);
    let corners = [
        old_rect.left_top(),
        old_rect.right_top(),
        old_rect.right_bottom(),
        old_rect.left_bottom(),
        old_rect.left_top(),
    ];
    for shape in egui::Shape::dashed_line(
        &corners,
        egui::Stroke::new(1.0, p.text_secondary.gamma_multiply(0.7)),
        4.0,
        3.0,
    ) {
        painter.add(shape);
    }
}

/// Show the dialog when open.
pub fn show_slide_size_dialog(
    editor: &mut Editor,
    ctx: &egui::Context,
    requests: &mut Vec<EditorRequest>,
) {
    if !editor.ui.slide_size_dialog.open {
        return;
    }
    let mut open = true;
    let mut apply = false;
    let mut cancel = false;
    let course = &editor.session.course;
    let revision = editor.session.revision;
    let current_slide_id = editor.selection.slide.clone();
    let current_size = course.slide_size;
    let default_slide = editor.ui.prefs.default_slide;
    let autosave_minutes = editor.ui.prefs.autosave_minutes;
    let versions_to_keep = editor.ui.prefs.versions_to_keep;
    let spell_check_as_you_type = editor.ui.prefs.spell_check_as_you_type;
    let spell_words = editor.ui.spell_words.clone();
    let p = coursecraft_render::palette(ctx);
    let was_editing_text = ctx.egui_wants_keyboard_input();

    Dialog::new("Slide size")
        .open(&mut open)
        .width(500.0)
        .show_with_footer(ctx, |dialog| {
            let mut save_default = false;
            dialog.body(|ui| {
                let state = &mut editor.ui.slide_size_dialog;
                let form = Form::new(ui, &["Preset", "Dimensions", "Aspect ratio"]);

                form.row(ui, "Preset", |ui, _| {
                    let selected_label = state.preset_index().map_or("Custom", |i| PRESETS[i].label);
                    styled_combo(ui, "slide_size_preset_combo", 260.0)
                        .selected_text(selected_label)
                        .show_ui(ui, |ui| {
                            for (idx, preset) in PRESETS.iter().enumerate() {
                                if ui
                                    .selectable_label(state.preset_index() == Some(idx), preset.label)
                                    .clicked()
                                {
                                    state.w = preset.w;
                                    state.h = preset.h;
                                    state.locked_ratio = f64::from(preset.w) / f64::from(preset.h);
                                    state.saved_default_notice = false;
                                }
                            }
                            let is_custom = state.preset_index().is_none();
                            if ui.selectable_label(is_custom, "Custom").clicked() && !is_custom {
                                state.w = state.w.saturating_add(1).min(7680);
                                if state.lock_aspect && state.locked_ratio > 0.0 {
                                    state.h = (f64::from(state.w) / state.locked_ratio)
                                        .round()
                                        .clamp(100.0, 4320.0) as u32;
                                }
                                state.saved_default_notice = false;
                            }
                        });
                });

                ui.add_space(coursecraft_render::SPACE_XS);

                let prev_w = state.w;
                let prev_h = state.h;
                form.row(ui, "Dimensions", |ui, _| {
                    ui.horizontal(|ui| {
                        ui.label("W");
                        ui.add(
                            DragValue::new(&mut state.w)
                                .range(0..=100_000)
                                .suffix(" px"),
                        );
                        ui.label(egui::RichText::new("×").color(p.text_secondary));
                        ui.label("H");
                        ui.add(
                            DragValue::new(&mut state.h)
                                .range(0..=100_000)
                                .suffix(" px"),
                        );
                    });
                });
                let w_changed = state.w != prev_w;
                let h_changed = state.h != prev_h;
                if w_changed || h_changed {
                    state.saved_default_notice = false;
                    if state.lock_aspect && state.locked_ratio > 0.0 {
                        if w_changed && !h_changed {
                            state.h = (f64::from(state.w) / state.locked_ratio)
                                .round()
                                .clamp(100.0, 4320.0) as u32;
                        } else if h_changed && !w_changed {
                            state.w = (f64::from(state.h) * state.locked_ratio)
                                .round()
                                .clamp(100.0, 7680.0) as u32;
                        }
                    }
                }
                if !state.valid() {
                    ui.colored_label(
                        p.danger,
                        "Width must be 100 to 7680 px and height 100 to 4320 px.",
                    );
                }

                ui.add_space(coursecraft_render::SPACE_XS);

                form.row(ui, "Aspect ratio", |ui, _| {
                    ui.horizontal(|ui| {
                        let lock_resp = ui.checkbox(&mut state.lock_aspect, "Lock aspect ratio");
                        if lock_resp.changed() && state.lock_aspect && state.h > 0 {
                            state.locked_ratio = f64::from(state.w) / f64::from(state.h);
                        }
                        let ratio_text = format_aspect_ratio(state.w, state.h);
                        egui::Frame::new()
                            .fill(p.input)
                            .stroke(egui::Stroke::new(1.0, p.border))
                            .corner_radius(coursecraft_render::RADIUS_SM as u8)
                            .inner_margin(egui::Margin::symmetric(6, 2))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(ratio_text)
                                        .small()
                                        .color(p.text_secondary),
                                );
                            });
                    });
                });

                ui.add_space(coursecraft_render::SPACE_SM);

                ui.horizontal(|ui| {
                    let is_already_default = (state.w, state.h) == default_slide;
                    let can_save_default = state.valid() && !is_already_default;
                    let disabled_tip = if !state.valid() {
                        "Width must be 100 to 7680 px and height 100 to 4320 px."
                    } else {
                        "Already the default for new courses"
                    };
                    if ui
                        .add_enabled(
                            can_save_default,
                            egui::Button::new("Use as default for new courses"),
                        )
                        .on_disabled_hover_text(disabled_tip)
                        .clicked()
                    {
                        save_default = true;
                        state.saved_default_notice = true;
                    }
                    if state.saved_default_notice {
                        ui.colored_label(p.success, "Saved as default");
                    }
                });

                ui.add_space(coursecraft_render::SPACE_SM);
                ui.separator();
                ui.add_space(coursecraft_render::SPACE_SM);

                ui.label(egui::RichText::new("Existing content").strong());
                ui.add_space(coursecraft_render::SPACE_XS);
                for (mode, label, helper) in [
                    (
                        ResizeModeChoice::Scale,
                        "Scale to fit",
                        "Fits every slide inside the new size, adding margins if the aspect ratio changes.",
                    ),
                    (
                        ResizeModeChoice::Fill,
                        "Fill slide",
                        "Fills the new size edge to edge; edges may extend past the slide if the aspect ratio changes.",
                    ),
                    (
                        ResizeModeChoice::Keep,
                        "Keep as is",
                        "Changes only the slide size; objects stay at their current positions and sizes.",
                    ),
                ] {
                    ui.radio_value(&mut state.mode, mode, label);
                    ui.indent(label, |ui| {
                        ui.label(egui::RichText::new(helper).small().color(p.text_secondary));
                    });
                    ui.add_space(2.0);
                }

                ui.add_space(coursecraft_render::SPACE_SM);
                ui.separator();
                ui.add_space(coursecraft_render::SPACE_SM);

                if state.valid() {
                    let key = ImpactCacheKey {
                        revision,
                        w: state.w,
                        h: state.h,
                        mode: state.mode,
                    };
                    let cache_hit = state
                        .impact_cache
                        .as_ref()
                        .is_some_and(|(cached_key, _)| *cached_key == key);
                    if !cache_hit {
                        let impact = compute_impact(course, state.w, state.h, state.mode);
                        state.impact_cache = Some((key, impact));
                        #[cfg(test)]
                        {
                            state.impact_recomputes += 1;
                        }
                    }

                    ui.horizontal(|ui| {
                        draw_placement_preview(
                            ui,
                            course,
                            &current_slide_id,
                            state.w,
                            state.h,
                            state.mode,
                            &p,
                        );
                        ui.add_space(coursecraft_render::SPACE_SM);
                        if let Some((_, impact)) = &state.impact_cache {
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new("Preview").strong());
                                ui.add_space(2.0);
                                ui.label(
                                    egui::RichText::new(&impact.hint)
                                        .small()
                                        .color(p.text_secondary),
                                );
                                if !impact.same_size || impact.outside > 0 || impact.clipped > 0 {
                                    ui.add_space(4.0);
                                    if impact.outside == 0 && impact.clipped == 0 {
                                        ui.label(
                                            egui::RichText::new("0 outside, 0 clipped")
                                                .small()
                                                .color(p.text_secondary),
                                        );
                                    } else {
                                        if impact.outside > 0 {
                                            let noun = if impact.outside == 1 {
                                                "object"
                                            } else {
                                                "objects"
                                            };
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{} outside ({} {} will sit outside the slide)",
                                                    impact.outside, impact.outside, noun
                                                ))
                                                .small()
                                                .color(p.warning),
                                            );
                                        } else {
                                            ui.label(
                                                egui::RichText::new("0 outside")
                                                    .small()
                                                    .color(p.text_secondary),
                                            );
                                        }
                                        if impact.clipped > 0 {
                                            let noun = if impact.clipped == 1 {
                                                "object"
                                            } else {
                                                "objects"
                                            };
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{} clipped ({} {} will extend past an edge)",
                                                    impact.clipped, impact.clipped, noun
                                                ))
                                                .small()
                                                .color(p.warning),
                                            );
                                        }
                                    }
                                }
                            });
                        }
                    });
                }
            });

            if save_default {
                let state = &editor.ui.slide_size_dialog;
                editor.ui.prefs.default_slide = (state.w, state.h);
                requests.push(EditorRequest::SetPreferences {
                    autosave_minutes,
                    versions_to_keep,
                    default_slide: (state.w, state.h),
                    spell_check_as_you_type,
                    spell_words,
                });
            }

            let state = &editor.ui.slide_size_dialog;
            let same_size = state.w == current_size.w && state.h == current_size.h;
            let can_apply = state.valid() && !same_size;
            let disabled_reason = if !state.valid() {
                "Width must be 100 to 7680 px and height 100 to 4320 px."
            } else {
                "Slide size is already at these dimensions"
            };

            dialog.footer(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let apply_resp = if can_apply {
                        crate::primary_button(ui, "Apply")
                    } else {
                        let btn = egui::Button::new("Apply")
                            .corner_radius(coursecraft_render::RADIUS_MD as u8)
                            .min_size(egui::vec2(0.0, coursecraft_render::CONTROL_PRIMARY));
                        ui.add_enabled(false, btn)
                            .on_disabled_hover_text(disabled_reason)
                    };
                    if apply_resp.clicked() {
                        apply = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });

            if can_apply
                && !was_editing_text
                && !egui::Popup::is_any_open(ctx)
                && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter))
            {
                apply = true;
            }
        });

    if apply {
        let state = editor.ui.slide_size_dialog.clone();
        editor.run(
            "course.set_slide_size",
            json!({
                "w": state.w,
                "h": state.h,
                "mode": state.mode.as_str(),
            }),
        );
        editor.ui.slide_size_dialog.open = false;
    }
    if cancel || !open {
        editor.ui.slide_size_dialog.open = false;
    }
}
