//! File > Slide Size: pick a preset or custom size and how content follows.

use crate::dialog::{Dialog, Form};
use crate::widgets::styled_combo;
use crate::{Editor, EditorRequest};
use egui::DragValue;
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
        };
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
    let current_size = editor.session.course.slide_size;
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
