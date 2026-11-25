//! Theme window dialog: accent, background, text color, player bar colors, font name.

use crate::EditorRequest;
use crate::dialog::Dialog;
use crate::font_picker::{FontPickerAction, show_font_picker};
use crate::properties::{is_committed, show_color_field};
use coursecraft_course::{Course, Theme};

/// Local buffers for editing the Course Theme.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ThemeDialogState {
    pub last_synced_revision: crate::SyncRevision,
    pub accent: String,
    pub background: String,
    pub text: String,
    pub font: String,
    pub bar: String,
    pub bar_text: String,
}

impl ThemeDialogState {
    /// Resync state buffers from course data on revision change.
    pub fn sync(&mut self, course: &Course, revision: u64) {
        if self.last_synced_revision.is(revision) {
            return;
        }
        self.last_synced_revision.set(revision);
        self.accent = course.theme.accent.0.clone();
        self.background = course.theme.background.0.clone();
        self.text = course.theme.text.0.clone();
        self.font = course.theme.font.clone();
        self.bar = course.theme.player.bar.0.clone();
        self.bar_text = course.theme.player.bar_text.0.clone();
    }
}

/// Render the Theme modal window dialog.
pub fn show_theme_dialog_window(editor: &mut crate::Editor, ctx: &egui::Context) {
    if !editor.ui.show_theme_window {
        return;
    }

    editor
        .ui
        .theme_dialog
        .sync(&editor.session.course, editor.session.revision);
    editor.ui.properties_lists.ensure(
        editor.session.revision,
        &editor.session.course,
        &editor.media,
    );

    let mut open = true;
    let mut close = false;
    let mut update_theme: Option<serde_json::Value> = None;
    let mut palette_actions = Vec::new();

    Dialog::new("Theme")
        .open(&mut open)
        .width(420.0)
        .show_with_footer(ctx, |dialog| {
            dialog.body(|ui| {
                ui.heading("Colors");

                let (cr_acc, valid_acc) = show_color_field(
                    ui,
                    "Accent",
                    &mut editor.ui.theme_dialog.accent,
                    "theme_accent",
                );
                if is_committed(&cr_acc, ui)
                    && valid_acc
                    && editor.ui.theme_dialog.accent.trim() != editor.session.course.theme.accent.0
                {
                    update_theme = Some(serde_json::json!({
                        "accent": editor.ui.theme_dialog.accent.trim(),
                    }));
                }

                let (cr_bg, valid_bg) = show_color_field(
                    ui,
                    "Slide background",
                    &mut editor.ui.theme_dialog.background,
                    "theme_bg",
                );
                if is_committed(&cr_bg, ui)
                    && valid_bg
                    && editor.ui.theme_dialog.background.trim()
                        != editor.session.course.theme.background.0
                {
                    update_theme = Some(serde_json::json!({
                        "background": editor.ui.theme_dialog.background.trim(),
                    }));
                }

                let (cr_txt, valid_txt) = show_color_field(
                    ui,
                    "Text color",
                    &mut editor.ui.theme_dialog.text,
                    "theme_text",
                );
                if is_committed(&cr_txt, ui)
                    && valid_txt
                    && editor.ui.theme_dialog.text.trim() != editor.session.course.theme.text.0
                {
                    update_theme = Some(serde_json::json!({
                        "text": editor.ui.theme_dialog.text.trim(),
                    }));
                }

                ui.add_space(8.0);
                ui.separator();
                ui.heading("Player chrome");

                let (cr_bar, valid_bar) =
                    show_color_field(ui, "Bar fill", &mut editor.ui.theme_dialog.bar, "theme_bar");
                if is_committed(&cr_bar, ui)
                    && valid_bar
                    && editor.ui.theme_dialog.bar.trim() != editor.session.course.theme.player.bar.0
                {
                    update_theme = Some(serde_json::json!({
                        "bar": editor.ui.theme_dialog.bar.trim(),
                    }));
                }

                let (cr_bar_txt, valid_bar_txt) = show_color_field(
                    ui,
                    "Bar text",
                    &mut editor.ui.theme_dialog.bar_text,
                    "theme_bar_text",
                );
                if is_committed(&cr_bar_txt, ui)
                    && valid_bar_txt
                    && editor.ui.theme_dialog.bar_text.trim()
                        != editor.session.course.theme.player.bar_text.0
                {
                    update_theme = Some(serde_json::json!({
                        "bar_text": editor.ui.theme_dialog.bar_text.trim(),
                    }));
                }

                ui.add_space(8.0);
                ui.separator();
                palette_actions =
                    crate::text_styles_ui::show_palette_section(ui, &editor.session.course);

                ui.add_space(8.0);
                ui.separator();
                ui.heading(tl!("Typography"));

                let state = &mut editor.ui;
                let picked = crate::properties::inspector_row(ui, tl!("Font name"), |ui| {
                    show_font_picker(
                        ui,
                        "theme_font",
                        &state.theme_dialog.font,
                        &state.properties_lists.font_families,
                        &mut state.properties.recent_fonts,
                        state.system_fonts.as_deref(),
                        &mut state.properties.system_fonts_list_requested,
                        &mut state.properties.system_font_search,
                        &mut editor.pending_requests,
                    )
                });
                match picked {
                    Some(FontPickerAction::PickCourseOrRecent(name))
                        if state.theme_dialog.font != name =>
                    {
                        state.theme_dialog.font = name;
                        update_theme = Some(serde_json::json!({ "font": state.theme_dialog.font }));
                    }
                    Some(FontPickerAction::PickSystem(name)) => {
                        // The host applies the command only after embedding succeeds.
                        // The dialog then picks up the new font on revision sync.
                        editor.pending_requests.push(EditorRequest::AddSystemFont {
                            family: name.clone(),
                            apply: Some((
                                "course.set_theme".to_owned(),
                                serde_json::json!({ "font": name }),
                            )),
                        });
                    }
                    Some(FontPickerAction::AddFont) => {
                        editor.pending_requests.push(EditorRequest::PickFonts);
                    }
                    Some(FontPickerAction::PickCourseOrRecent(_)) | None => {}
                }
            });
            dialog.footer(|ui| {
                ui.horizontal(|ui| {
                    if ui.button("Reset to defaults").clicked() {
                        let def = Theme::default();
                        editor.ui.theme_dialog.accent = def.accent.0.clone();
                        editor.ui.theme_dialog.background = def.background.0.clone();
                        editor.ui.theme_dialog.text = def.text.0.clone();
                        editor.ui.theme_dialog.font = def.font.clone();
                        editor.ui.theme_dialog.bar = def.player.bar.0.clone();
                        editor.ui.theme_dialog.bar_text = def.player.bar_text.0.clone();
                        update_theme = Some(serde_json::json!({
                            "accent": def.accent.0,
                            "background": def.background.0,
                            "text": def.text.0,
                            "font": def.font,
                            "bar": def.player.bar.0,
                            "bar_text": def.player.bar_text.0,
                        }));
                    }

                    if crate::primary_button(ui, "Close").clicked() {
                        close = true;
                    }
                });
            });
        });

    if let Some(params) = update_theme {
        editor.run("course.set_theme", params);
    }

    for (id, params) in palette_actions {
        editor.run(id, params);
    }

    if close || !open {
        editor.ui.show_theme_window = false;
    }
}
