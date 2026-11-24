//! Preferences window: autosave interval, spelling, versions, appearance,
//! performance, the ribbon and docked panels, interface language, the default
//! slide size and updates.
//!
//! The window holds a view of the saved settings. Every change applies at once
//! and goes to the app as an [`EditorRequest`], which saves it (the ribbon and
//! docking settings through [`EditorRequest::SaveLayout`], sent by the editor).

use crate::dialog::Dialog;
use coursecraft_render::{PerformanceMode, ThemePref};
use egui::DragValue;

use crate::i18n::{self, I18N};
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
    /// Check GitHub at most once a day on startup for a newer desktop version.
    pub auto_check_updates: bool,
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
            auto_check_updates: true,
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

    /// Show the saved automatic update check setting in the window.
    pub fn set_auto_check_updates(&mut self, enabled: bool) {
        self.ui.prefs.auto_check_updates = enabled;
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
    let updates_available = editor.updates_available;
    let mut auto_check = editor.ui.prefs.auto_check_updates;
    let mut language = editor.language.clone();
    let autosave = tl!("Autosave every");
    let versions = tl!("Versions to keep");
    // Keyed by the English title, so the window keeps its place when the language changes.
    Dialog::new(tl!("Preferences"))
        .id(egui::Id::unique(("coursecraft_dialog", "Preferences")))
        .open(&mut open)
        .width(440.0)
        .show(ctx, |ui| {
            let ui_state = &mut editor.ui;
            let prefs = &mut ui_state.prefs;
            let form = crate::dialog::Form::new(ui, &[autosave, versions]);
            form.row(ui, autosave, |ui, _| {
                ui.add(
                    DragValue::new(&mut prefs.autosave_minutes)
                        .range(0..=MAX_AUTOSAVE_MINUTES)
                        .suffix(format!(" {}", tl!("min"))),
                );
            });
            ui.small(tl!("0 turns autosave off."));
            ui.separator();
            ui.label(tl!("Writing"));
            ui.checkbox(
                &mut prefs.spell_check_as_you_type,
                tl!("Check spelling as you type"),
            );
            ui.separator();
            form.row(ui, versions, |ui, _| {
                ui.add(
                    DragValue::new(&mut prefs.versions_to_keep)
                        .range(0..=MAX_VERSIONS_TO_KEEP)
                        .suffix(format!(" {}", tl!("per file"))),
                );
            });
            ui.small(tl!(
                "Each save keeps the file it replaced. 0 turns versions off."
            ));
            ui.separator();
            ui.label(tl!("Appearance"));
            ui.horizontal_wrapped(|ui| {
                for (pref, label) in [
                    (ThemePref::System, "System"),
                    (ThemePref::Light, "Light"),
                    (ThemePref::Dark, "Dark"),
                ] {
                    if ui.radio(theme == pref, tl!(label)).clicked() {
                        theme = pref;
                    }
                }
            });
            ui.separator();
            ui.label(tl!("Performance"));
            ui.horizontal_wrapped(|ui| {
                for (mode, label) in [
                    (PerformanceMode::Auto, "Automatic"),
                    (PerformanceMode::Standard, "Standard"),
                    (PerformanceMode::Lite, "Lite"),
                ] {
                    if ui.radio(prefs.performance_mode == mode, tl!(label)).clicked() {
                        prefs.performance_mode = mode;
                    }
                }
            });
            ui.small(tl!(
                "Lite lowers editor cache sizes and preview detail on older hardware. Published courses are not affected."
            ));
            ui.separator();
            ui.label(tl!("Interface"));
            ui.horizontal_wrapped(|ui| {
                ui.checkbox(&mut ui_state.ribbon.visible, tl!("Ribbon"))
                    .on_hover_text(tl!(
                        "Commands in tabs above the slide. Off shows the older toolbar."
                    ));
                ui.checkbox(&mut ui_state.dock.enabled, tl!("Docked panels"))
                    .on_hover_text(tl!(
                        "Move, tab, float and close the panels. Off keeps them fixed."
                    ));
            });
            ui.separator();
            language_picker(ui, &mut language);
            ui.separator();
            ui.label(tl!("Default slide size for new courses"));
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
            if updates_available {
                ui.separator();
                ui.label(tl!("Updates"));
                ui.checkbox(&mut auto_check, tl!("Check for updates automatically"));
                ui.small(tl!(
                    "Checks GitHub once a day on startup. No personal data is sent."
                ));
            }
        });
    if language != editor.language {
        editor.set_language(&language);
        requests.push(EditorRequest::SetLanguage(editor.language.clone()));
    }
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
    if auto_check != editor.ui.prefs.auto_check_updates {
        editor.ui.prefs.auto_check_updates = auto_check;
        requests.push(EditorRequest::SetAutoCheckUpdates(auto_check));
    }
    if !open {
        editor.ui.prefs.open = false;
    }
}

/// Preferences › Language: Automatic (the system language) or one language the
/// build offers, each named in itself.
fn language_picker(ui: &mut egui::Ui, setting: &mut String) {
    ui.label(tl!("Language"));
    let languages = I18N.languages();
    let auto = tl!("Automatic (system)");
    let selected = if setting.as_str() == i18n::AUTO {
        auto.to_owned()
    } else {
        I18N.from_code(setting)
            .map_or_else(|| setting.clone(), |lang| I18N.name(lang).to_owned())
    };
    egui::ComboBox::from_id_salt("preferences_language")
        .selected_text(selected)
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(setting.as_str() == i18n::AUTO, auto)
                .clicked()
            {
                *setting = i18n::AUTO.to_owned();
            }
            for lang in &languages {
                let code = I18N.code(*lang);
                if ui
                    .selectable_label(setting.as_str() == code, I18N.name(*lang))
                    .clicked()
                {
                    *setting = code.to_owned();
                }
            }
        });
    if languages.len() <= 1 {
        ui.small(tl!(
            "This build has no translations yet, so the interface is in English."
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_harness::TestHarness;
    use coursecraft_course::Course;

    fn sample_course() -> Course {
        let json = include_str!("../../../examples/hello-course/course.json");
        Course::from_json(json).expect("hello-course fixture")
    }

    /// UI-05: Toggling "Check for updates automatically" emits `SetAutoCheckUpdates`
    /// without spuriously emitting `SetPreferences`.
    #[test]
    fn ui05_preferences_auto_check_toggle() {
        let mut h = TestHarness::new(sample_course());
        h.editor.set_updates_available(true);
        assert!(h.editor.ui.prefs.auto_check_updates);

        h.editor.open_preferences();
        h.idle_frame();
        h.idle_frame();

        let toggle_pos = h
            .find_text("Check for updates automatically")
            .expect("auto-check updates checkbox");
        let reqs = h.click_at_requests(toggle_pos);
        assert!(!h.editor.ui.prefs.auto_check_updates);
        assert!(
            reqs.contains(&EditorRequest::SetAutoCheckUpdates(false)),
            "expected SetAutoCheckUpdates(false), got {reqs:?}"
        );
        assert!(
            !reqs
                .iter()
                .any(|r| matches!(r, EditorRequest::SetPreferences { .. })),
            "toggling auto_check_updates must not emit SetPreferences, got {reqs:?}"
        );

        let toggle_pos = h
            .find_text("Check for updates automatically")
            .expect("auto-check updates checkbox");
        let reqs = h.click_at_requests(toggle_pos);
        assert!(h.editor.ui.prefs.auto_check_updates);
        assert!(
            reqs.contains(&EditorRequest::SetAutoCheckUpdates(true)),
            "expected SetAutoCheckUpdates(true), got {reqs:?}"
        );
        assert!(
            !reqs
                .iter()
                .any(|r| matches!(r, EditorRequest::SetPreferences { .. })),
            "toggling auto_check_updates must not emit SetPreferences, got {reqs:?}"
        );
    }
}
