//! Preferences window: autosave interval, spelling, versions, appearance, the
//! ribbon and docked panels, interface language and the default slide size.
//!
//! The window holds a view of the saved settings. Every change applies at once
//! and goes to the app as an [`EditorRequest`], which saves it (the ribbon and
//! docking settings through [`EditorRequest::SaveLayout`], sent by the editor).

use crate::dialog::Dialog;
use coursecraft_render::ThemePref;
use egui::DragValue;

use crate::i18n::{self, I18N};
use crate::slide_size_dialog::PRESETS;
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
}

impl Default for PrefsState {
    fn default() -> Self {
        Self {
            open: false,
            autosave_minutes: 1,
            versions_to_keep: 10,
            default_slide: (PRESETS[0].1, PRESETS[0].2),
            spell_check_as_you_type: true,
        }
    }
}

impl Editor {
    /// Show the saved settings in the window.
    pub fn set_preferences(&mut self, autosave_minutes: u32, default_slide: (u32, u32)) {
        self.ui.prefs.autosave_minutes = autosave_minutes.min(MAX_AUTOSAVE_MINUTES);
        self.ui.prefs.default_slide = if PRESETS.iter().any(|p| (p.1, p.2) == default_slide) {
            default_slide
        } else {
            (PRESETS[0].1, PRESETS[0].2)
        };
    }

    /// Show the saved number of versions to keep in the window.
    pub fn set_versions_to_keep(&mut self, versions: u32) {
        self.ui.prefs.versions_to_keep = versions.min(MAX_VERSIONS_TO_KEEP);
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
    let mut language = editor.language.clone();
    let autosave = tl!("Autosave every");
    let versions = tl!("Versions to keep");
    // Keyed by the English title, so the window keeps its place when the language changes.
    Dialog::new(tl!("Preferences"))
        .id(egui::Id::unique(("coursecraft_dialog", "Preferences")))
        .open(&mut open)
        .width(440.0)
        .show(ctx, |ui| {
            let form = crate::dialog::Form::new(ui, &[autosave, versions]);
            form.row(ui, autosave, |ui, _| {
                ui.add(
                    DragValue::new(&mut editor.ui.prefs.autosave_minutes)
                        .range(0..=MAX_AUTOSAVE_MINUTES)
                        .suffix(format!(" {}", tl!("min"))),
                );
            });
            ui.small(tl!("0 turns autosave off."));
            ui.separator();
            ui.label(tl!("Writing"));
            ui.checkbox(
                &mut editor.ui.prefs.spell_check_as_you_type,
                tl!("Check spelling as you type"),
            );
            ui.separator();
            form.row(ui, versions, |ui, _| {
                ui.add(
                    DragValue::new(&mut editor.ui.prefs.versions_to_keep)
                        .range(0..=MAX_VERSIONS_TO_KEEP)
                        .suffix(format!(" {}", tl!("per file"))),
                );
            });
            ui.small(tl!(
                "Each save keeps the file it replaced. 0 turns versions off."
            ));
            ui.separator();
            ui.label(tl!("Appearance"));
            for (pref, label) in [
                (ThemePref::System, "System"),
                (ThemePref::Light, "Light"),
                (ThemePref::Dark, "Dark"),
            ] {
                if ui.radio(theme == pref, tl!(label)).clicked() {
                    theme = pref;
                }
            }
            ui.separator();
            ui.label(tl!("Interface"));
            ui.checkbox(&mut editor.ui.ribbon.visible, tl!("Ribbon"))
                .on_hover_text(tl!(
                    "Commands in tabs above the slide. Off shows the older toolbar."
                ));
            ui.checkbox(&mut editor.ui.dock.enabled, tl!("Docked panels"))
                .on_hover_text(tl!(
                    "Move, tab, float and close the panels. Off keeps them fixed."
                ));
            ui.separator();
            language_picker(ui, &mut language);
            ui.separator();
            ui.label(tl!("Default slide size for new courses"));
            for (label, w, h) in PRESETS {
                if ui
                    .radio(editor.ui.prefs.default_slide == (w, h), tl!(label))
                    .clicked()
                {
                    editor.ui.prefs.default_slide = (w, h);
                }
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
        requests.push(EditorRequest::SetPreferences {
            autosave_minutes: editor.ui.prefs.autosave_minutes,
            versions_to_keep: editor.ui.prefs.versions_to_keep,
            default_slide: editor.ui.prefs.default_slide,
            spell_check_as_you_type: editor.ui.prefs.spell_check_as_you_type,
            spell_words: editor.ui.spell_words.clone(),
        });
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
