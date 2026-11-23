//! File > Slide Size: pick a preset or custom size and how content follows.

use crate::dialog::Dialog;
use egui::DragValue;
use serde_json::json;

use crate::Editor;

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

/// Dialog state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlideSizeState {
    pub open: bool,
    pub w: u32,
    pub h: u32,
    /// Scale the content with the slide (true) or keep it as is.
    pub scale: bool,
}

impl Default for SlideSizeState {
    fn default() -> Self {
        Self {
            open: false,
            w: 1280,
            h: 720,
            scale: true,
        }
    }
}

impl SlideSizeState {
    /// Whether the entered size is inside the allowed range.
    #[must_use]
    pub fn valid(&self) -> bool {
        (100..=7680).contains(&self.w) && (100..=4320).contains(&self.h)
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
        self.ui.slide_size_dialog = SlideSizeState {
            open: true,
            w: size.w,
            h: size.h,
            scale: true,
        };
    }
}

/// Show the dialog when open.
pub fn show_slide_size_dialog(editor: &mut Editor, ctx: &egui::Context) {
    if !editor.ui.slide_size_dialog.open {
        return;
    }
    let mut open = true;
    let mut apply = false;
    let mut cancel = false;
    Dialog::new("Slide size").open(&mut open).show(ctx, |ui| {
        let state = &mut editor.ui.slide_size_dialog;
        ui.label("Size");
        for (idx, preset) in PRESETS.iter().enumerate() {
            if ui
                .radio(state.preset_index() == Some(idx), preset.label)
                .clicked()
            {
                state.w = preset.w;
                state.h = preset.h;
            }
        }
        let custom = state.preset_index().is_none();
        if ui.radio(custom, "Custom").clicked() && !custom {
            state.w = state.w.saturating_add(1).min(7680);
        }
        ui.horizontal(|ui| {
            ui.label("W");
            ui.add(
                DragValue::new(&mut state.w)
                    .range(0..=100_000)
                    .suffix(" px"),
            );
            ui.label("H");
            ui.add(
                DragValue::new(&mut state.h)
                    .range(0..=100_000)
                    .suffix(" px"),
            );
        });
        if !state.valid() {
            ui.colored_label(
                coursecraft_render::palette(ui.ctx()).danger,
                "Width 100 to 7680, height 100 to 4320.",
            );
        }
        ui.separator();
        ui.radio_value(&mut state.scale, true, "Scale content");
        ui.radio_value(&mut state.scale, false, "Keep content as is");
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Cancel").clicked() {
                cancel = true;
            }
            if ui
                .add_enabled(state.valid(), egui::Button::new("Apply"))
                .on_disabled_hover_text("Width must be 100 to 7680 px and height 100 to 4320 px.")
                .clicked()
            {
                apply = true;
            }
        });
    });
    if apply {
        let state = editor.ui.slide_size_dialog.clone();
        editor.run(
            "course.set_slide_size",
            json!({
                "w": state.w,
                "h": state.h,
                "mode": if state.scale { "scale" } else { "keep" },
            }),
        );
        editor.ui.slide_size_dialog.open = false;
    }
    if cancel || !open {
        editor.ui.slide_size_dialog.open = false;
    }
}
