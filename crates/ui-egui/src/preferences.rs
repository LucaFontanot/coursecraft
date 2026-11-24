//! Preferences window: autosave interval, appearance and the default slide size.
//!
//! The window holds a view of the saved settings. Every change applies at once
//! and goes to the app as an [`EditorRequest`], which saves it.

use crate::dialog::Dialog;
use coursecraft_render::{PerformanceMode, ThemePref};
use egui::DragValue;

use crate::slide_size_dialog::{
    DEFAULT_SLIDE_SIZE, PRESETS, SLIDE_H_RANGE, SLIDE_W_RANGE, format_aspect_ratio,
    is_valid_slide_size,
};
use crate::{Editor, EditorRequest};

/// Autosave interval bounds in minutes. 0 turns autosave off.
pub const MAX_AUTOSAVE_MINUTES: u32 = 60;

/// Most file versions a project can keep. 0 turns versions off.
pub const MAX_VERSIONS_TO_KEEP: u32 = 100;

/// What the window shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrefsState {
    pub open: bool,
    pub autosave_minutes: u32,
    /// How many earlier saves of a project to keep. 0 keeps none.
    pub versions_to_keep: u32,
    pub default_slide: (u32, u32),
    /// Underline misspellings while editing text.
    pub spell_check_as_you_type: bool,
    /// Editor performance profile preference.
    pub performance_mode: PerformanceMode,
}

impl Default for PrefsState {
    fn default() -> Self {
        Self {
            open: false,
            autosave_minutes: 1,
            versions_to_keep: 10,
            default_slide: DEFAULT_SLIDE_SIZE,
            spell_check_as_you_type: true,
            performance_mode: PerformanceMode::Auto,
        }
    }
}

impl Editor {
    /// Show the saved settings in the window.
    pub fn set_preferences(&mut self, autosave_minutes: u32, default_slide: (u32, u32)) {
        self.ui.prefs.autosave_minutes = autosave_minutes.min(MAX_AUTOSAVE_MINUTES);
        self.ui.prefs.default_slide = if is_valid_slide_size(default_slide.0, default_slide.1) {
            default_slide
        } else {
            DEFAULT_SLIDE_SIZE
        };
    }

    /// Show the saved number of versions to keep in the window.
    pub fn set_versions_to_keep(&mut self, versions: u32) {
        self.ui.prefs.versions_to_keep = versions.min(MAX_VERSIONS_TO_KEEP);
    }

    /// Set the active editor performance mode and immediately update editor cache ceilings.
    pub fn set_performance_mode(&mut self, mode: PerformanceMode) {
        self.performance_mode = mode;
        self.ui.prefs.performance_mode = mode;
        self.sync_perf_profile(None);
    }

    /// Open the Preferences window.
    pub fn open_preferences(&mut self) {
        self.ui.prefs.open = true;
    }
}

/// Show the window when open.
pub fn show_preferences_window(
    editor: &mut Editor,
    ctx: &egui::Context,
    requests: &mut Vec<EditorRequest>,
) {
    if !editor.ui.prefs.open {
        return;
    }
    let mut open = true;
    let before = editor.ui.prefs.clone();
    let mut theme = editor.theme_pref;
    Dialog::new("Preferences")
        .open(&mut open)
        .width(440.0)
        .show(ctx, |ui| {
            let prefs = &mut editor.ui.prefs;
            let form = crate::dialog::Form::new(ui, &["Autosave every", "Versions to keep"]);
            form.row(ui, "Autosave every", |ui, _| {
                ui.add(
                    DragValue::new(&mut prefs.autosave_minutes)
                        .range(0..=MAX_AUTOSAVE_MINUTES)
                        .suffix(" min"),
                );
            });
            ui.small("0 turns autosave off.");
            ui.separator();
            ui.label("Writing");
            ui.checkbox(
                &mut prefs.spell_check_as_you_type,
                "Check spelling as you type",
            );
            ui.separator();
            form.row(ui, "Versions to keep", |ui, _| {
                ui.add(
                    DragValue::new(&mut prefs.versions_to_keep)
                        .range(0..=MAX_VERSIONS_TO_KEEP)
                        .suffix(" per file"),
                );
            });
            ui.small("Each save keeps the file it replaced. 0 turns versions off.");
            ui.separator();
            ui.label("Appearance");
            for (pref, label) in [
                (ThemePref::System, "System"),
                (ThemePref::Light, "Light"),
                (ThemePref::Dark, "Dark"),
            ] {
                if ui.radio(theme == pref, label).clicked() {
                    theme = pref;
                }
            }
            ui.separator();
            ui.label("Performance");
            for (mode, label) in [
                (PerformanceMode::Auto, "Automatic"),
                (PerformanceMode::Standard, "Standard"),
                (PerformanceMode::Lite, "Lite"),
            ] {
                if ui.radio(prefs.performance_mode == mode, label).clicked() {
                    prefs.performance_mode = mode;
                }
            }
            ui.small(
                "Lite lowers editor cache sizes and preview detail on older hardware. Published courses are not affected.",
            );
            ui.separator();
            ui.label("Default slide size for new courses");
            let selected_preset = PRESETS.iter().find(|p| (p.w, p.h) == prefs.default_slide);
            let selected_label = selected_preset.map_or("Custom", |p| p.label);
            crate::widgets::styled_combo(ui, "prefs_default_slide_combo", 240.0)
                .selected_text(selected_label)
                .show_ui(ui, |ui| {
                    for preset in PRESETS {
                        if ui
                            .selectable_label(
                                prefs.default_slide == (preset.w, preset.h),
                                preset.label,
                            )
                            .clicked()
                        {
                            prefs.default_slide = (preset.w, preset.h);
                        }
                    }
                    let is_custom = selected_preset.is_none();
                    if ui.selectable_label(is_custom, "Custom").clicked() && !is_custom {
                        prefs.default_slide.0 = prefs.default_slide.0.saturating_add(1).min(7680);
                    }
                });
            ui.horizontal(|ui| {
                ui.label("W");
                ui.add(
                    DragValue::new(&mut prefs.default_slide.0)
                        .range(SLIDE_W_RANGE)
                        .suffix(" px"),
                );
                ui.label("×");
                ui.label("H");
                ui.add(
                    DragValue::new(&mut prefs.default_slide.1)
                        .range(SLIDE_H_RANGE)
                        .suffix(" px"),
                );
                ui.small(format_aspect_ratio(
                    prefs.default_slide.0,
                    prefs.default_slide.1,
                ));
            });
        });
    if theme != editor.theme_pref {
        editor.theme_pref = theme;
        requests.push(EditorRequest::SetTheme(theme));
    }
    if editor.ui.prefs != before {
        if editor.ui.prefs.performance_mode != editor.performance_mode {
            editor.performance_mode = editor.ui.prefs.performance_mode;
            editor.sync_perf_profile(Some(ctx));
        }
        requests.push(EditorRequest::SetPreferences {
            autosave_minutes: editor.ui.prefs.autosave_minutes,
            versions_to_keep: editor.ui.prefs.versions_to_keep,
            default_slide: editor.ui.prefs.default_slide,
            spell_check_as_you_type: editor.ui.prefs.spell_check_as_you_type,
            spell_words: editor.ui.spell_words.clone(),
            performance_mode: editor.ui.prefs.performance_mode,
        });
    }
    if !open {
        editor.ui.prefs.open = false;
    }
}
