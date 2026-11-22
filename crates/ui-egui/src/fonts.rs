//! Course font manager: list, remove and add custom font faces.

use crate::dialog::Dialog;
use coursecraft_course::FontStyle;
use egui::{Context, RichText};

use crate::Editor;

fn style_label(style: FontStyle) -> &'static str {
    match style {
        FontStyle::Regular => "Regular",
        FontStyle::Bold => "Bold",
        FontStyle::Italic => "Italic",
        FontStyle::BoldItalic => "Bold Italic",
    }
}

/// Licence and embedding facts for a course font file, read once and cached by
/// media path so the window does not re-parse the font on every frame.
fn font_facts(editor: &mut Editor, src: &str) -> Option<(bool, Option<String>)> {
    if let Some(fact) = editor.ui.font_facts.get(src) {
        return Some(fact.clone());
    }
    let bytes = editor
        .media
        .iter()
        .find(|(name, _)| name == src)
        .map(|(_, bytes)| bytes.clone())?;
    let info = coursecraft_media::font_info(&bytes).ok()?;
    let fact = (info.embedding_restricted, info.licence);
    editor.ui.font_facts.insert(src.to_owned(), fact.clone());
    Some(fact)
}

/// Render the fonts window when the editor asked for it.
pub fn show_fonts_window(editor: &mut Editor, ctx: &Context) {
    if !editor.ui.show_fonts_window {
        return;
    }
    let mut open = true;
    let mut action: Option<Action> = None;
    let has_confirm = editor.ui.pending_remove_font.is_some();
    Dialog::new("Fonts")
        .open(&mut open)
        .close_on_escape(!has_confirm)
        .width(420.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.label(
                RichText::new("Inter is always available. Add a .ttf or .otf to use its family in the course.")
                    .size(coursecraft_render::SIZE_SMALL),
            );
            ui.add_space(coursecraft_render::SPACE_SM);
            if editor.session.course.fonts.is_empty() {
                ui.label(
                    RichText::new("No custom fonts yet.")
                        .color(coursecraft_render::palette(ctx).text_secondary),
                );
            }
            let faces = editor.session.course.fonts.clone();
            let mut shown_families: Vec<String> = Vec::new();
            for face in faces {
                ui.horizontal(|ui| {
                    let font = coursecraft_render::slide_font_named(
                        ctx,
                        Some(&face.family),
                        matches!(face.style, FontStyle::Bold | FontStyle::BoldItalic),
                        matches!(face.style, FontStyle::Italic | FontStyle::BoldItalic),
                        coursecraft_render::SIZE_BODY,
                    );
                    ui.label(RichText::new(&face.family).font(font));
                    ui.label(
                        RichText::new(style_label(face.style))
                            .color(coursecraft_render::palette(ctx).text_secondary),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button("Remove")
                            .on_hover_text(format!(
                                "Remove {} ({})",
                                face.family,
                                style_label(face.style)
                            ))
                            .clicked()
                        {
                            action = Some(Action::Remove(face.family.clone(), face.style));
                        }
                    });
                });
                if !shown_families.contains(&face.family) {
                    shown_families.push(face.family.clone());
                    if let Some((restricted, licence)) = font_facts(editor, &face.src) {
                        if let Some(licence) = licence.filter(|text| !text.is_empty()) {
                            ui.label(
                                RichText::new(licence)
                                    .small()
                                    .color(coursecraft_render::palette(ctx).text_secondary),
                            );
                        }
                        if restricted {
                            ui.label(
                                RichText::new(crate::RESTRICTED_FONT_NOTE)
                                    .small()
                                    .color(coursecraft_render::palette(ctx).text_secondary),
                            );
                        }
                    }
                }
            }
            ui.separator();
            if ui.button("Add font…").clicked() {
                editor.pending_requests.push(crate::EditorRequest::PickFonts);
            }
        });

    if let Some(Action::Remove(family, style)) = action {
        if editor.font_family_in_use(&family) {
            editor.ui.pending_remove_font = Some((family, style));
        } else if editor
            .run(
                "course.remove_font",
                serde_json::json!({ "family": family, "style": style }),
            )
            .is_none()
        {
            // The command already recorded the error.
        }
    }

    if let Some((family, style)) = editor.ui.pending_remove_font.clone() {
        let mut confirm_open = true;
        Dialog::new("Remove font?")
            .open(&mut confirm_open)
            .show(ctx, |ui| {
                ui.label(format!(
                    "\"{family}\" is used by text in this course. Removing it leaves that text in Inter. Remove it anyway?"
                ));
                ui.add_space(coursecraft_render::SPACE_SM);
                ui.horizontal(|ui| {
                    if ui.button("Remove").clicked() {
                        editor.ui.pending_remove_font = None;
                        let _ = editor.run(
                            "course.remove_font",
                            serde_json::json!({ "family": family, "style": style }),
                        );
                    }
                    if ui.button("Cancel").clicked() {
                        editor.ui.pending_remove_font = None;
                    }
                });
            });
        if !confirm_open {
            editor.ui.pending_remove_font = None;
        }
    }

    if !open {
        editor.ui.show_fonts_window = false;
    }
}

enum Action {
    Remove(String, FontStyle),
}
