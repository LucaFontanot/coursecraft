//! The command registry. Add a command: write a `fn(&mut Session, &Json) ->
//! Result<Json>`, add a row to `REGISTRY`, add a test. See `docs/contributing.md`.

use coursecraft_course::{
    Action, Align, Autofit, Choice, CmpOp, Color, Condition, Course, DashStyle, DragItem, Event,
    HotspotRegion, HotspotShape, Layer, MatchPair, Object, ObjectKind, Outline, Question,
    QuestionKind, Rect, RevisitMode, Scoring, SeekMode, ShapeKind, ShapeText, Size, Slide,
    StateDef, TextStyle, Theme, Timing, Trigger, VAlign, Value, VarOp, VariableDef,
};
use serde::de::DeserializeOwned;
use serde_json::{Value as Json, json};
use std::collections::BTreeSet;

use crate::{Command, EngineError, Result, Session};

mod course;
mod csv_import;
#[cfg(test)]
pub(crate) use csv_import::{MAX_CSV_BYTES, MAX_CSV_ROWS, parse_csv};
mod layer;
mod object;
use object::{all_group_ids, find_object_mut, objects_param};
mod object_states;
use object_states::remove_dangling_triggers;
mod slide;
use slide::{
    bank_create, clamp_frame, fresh_id_in, kind_prefix, new_bank_slide_with_id, next_fresh_id,
    remap_object_glue, remap_object_glues, remap_trigger, results_button,
};
mod styles;
mod table;
mod text;
mod trigger;
mod video_cues;

pub(super) const MAX_TEXT: usize = 4096;
pub(super) const MAX_DURATION_MS: u32 = 24 * 60 * 60 * 1000;

/// Every command, in menu order.
#[must_use]
pub fn registry() -> &'static [Command] {
    REGISTRY
}

/// Pre-edit snapshot scope for command `id`.
///
/// Unknown or unannotated commands default to [`crate::delta::UndoScope::FullCourse`].
#[must_use]
pub(crate) fn undo_scope(id: &str) -> crate::delta::UndoScope {
    use crate::delta::UndoScope;
    match id {
        // Verified course-header-only commands (zero slides, scenes, masters, templates, or banks mutated).
        "course.set_title"
        | "course.update"
        | "course.set_player"
        | "course.set_quiz"
        | "course.set_publish_settings"
        | "course.set_background_audio"
        | "course.set_theme"
        | "theme.set_palette"
        | "course.add_font"
        | "course.remove_font"
        | "view.set"
        | "view.guides.add"
        | "view.guides.move"
        | "view.guides.remove"
        | "resource.add"
        | "resource.update"
        | "resource.remove"
        | "resource.move"
        | "glossary.add"
        | "glossary.update"
        | "glossary.remove"
        | "glossary.move"
        | "glossary.sort"
        | "variable.add"
        | "variable.set_value" => UndoScope::CourseHeader,

        // Verified single-slide commands taking `"slide"`.
        "object.add"
        | "object.update"
        | "object.reorder"
        | "object.align"
        | "object.distribute"
        | "object.match_size"
        | "object.group"
        | "object.ungroup"
        | "object.duplicate"
        | "object.paste"
        | "object.set_frame"
        | "object.set_rotation"
        | "object.flip"
        | "object.lock"
        | "object.hide_in_editor"
        | "object.set_opacity"
        | "object.set_outline_join"
        | "object.set_fill_style"
        | "object.set_shadow"
        | "object.set_glow"
        | "object.apply_style"
        | "object.set_drag"
        | "object.set_lottie"
        | "object.set_web_source"
        | "object.set_input"
        | "object.set_marker"
        | "object.set_panel"
        | "object.set_scroll_panel"
        | "object.set_timer"
        | "object.set_meter"
        | "object.set_path"
        | "object.set_crop"
        | "object.set_captions"
        | "object.set_media"
        | "object.add_state"
        | "object.update_state"
        | "object.patch_state"
        | "object.delete_state"
        | "object.duplicate_state"
        | "object.rename_state"
        | "object.reorder_state"
        | "object.state_add_extra"
        | "object.state_update_extra"
        | "object.state_delete_extra"
        | "object.state_reorder_extra"
        | "object.delete"
        | "object.set_animation"
        | "object.add_motion_path"
        | "object.update_motion_path"
        | "object.remove_motion_path"
        | "object.set_autofit"
        | "text.set"
        | "text.style_range"
        | "text.set_paragraph"
        | "text.set_paragraphs"
        | "text.set_link"
        | "text_style.apply"
        | "table.set_cell"
        | "table.insert_row"
        | "table.delete_row"
        | "table.insert_column"
        | "table.delete_column"
        | "table.resize"
        | "table.merge"
        | "table.split"
        | "table.set_borders"
        | "table.set_header_row"
        | "layer.add"
        | "layer.update"
        | "layer.delete"
        | "layer.move"
        | "layer.duplicate"
        | "trigger.add"
        | "trigger.update"
        | "trigger.move"
        | "trigger.delete"
        | "video.add_cue"
        | "video.set_cue"
        | "video.remove_cue"
        | "video.add_question_cue"
        | "slide.update"
        | "slide.set_duration"
        | "slide.set_background"
        | "slide.set_transition"
        | "slide.set_player_overrides"
        | "slide.add_cue"
        | "slide.set_cue"
        | "slide.remove_cue"
        | "draw.update"
        | "question.insert"
        | "question.add_submit_button"
        | "question.add_feedback_layers"
        | "question.add_try_again_layer"
        | "question.set"
        | "question.delete"
        | "question.set_kind"
        | "question.add_accepted"
        | "question.remove_accepted"
        | "question.update_accepted"
        | "question.set_case_sensitive"
        | "question.add_pair"
        | "question.remove_pair"
        | "question.update_pair"
        | "question.reorder_pair"
        | "question.reorder_sequence"
        | "question.set_hotspot_image"
        | "question.add_region"
        | "question.update_region"
        | "question.remove_region"
        | "question.set_markers"
        | "question.add_drag_item"
        | "question.update_drag_item"
        | "question.remove_drag_item"
        | "question.set_drop_targets"
        | "question.set_drag_options" => UndoScope::SingleSlide,

        // Cross-slide, multi-slide, structural, or course+slide commands stay FullCourse.
        _ => UndoScope::FullCourse,
    }
}

/// Whether a `SingleSlide` command may use scoped single-slide validation
/// (`Course::validate_slide_errors`) inside `keep_valid` instead of full-course
/// `Course::first_error`.
///
/// Only commands whose mutation stays strictly inside one slide's geometry,
/// styling, text, tables, media/cues, layers, states, or slide settings are
/// listed. Commands touching triggers, questions, bank draws, or course-wide
/// invariants fall back to full `Course::first_error()`.
#[must_use]
pub(crate) fn can_use_slide_validation(id: &str) -> bool {
    matches!(
        id,
        "object.add"
            | "object.update"
            | "object.reorder"
            | "object.align"
            | "object.distribute"
            | "object.match_size"
            | "object.group"
            | "object.ungroup"
            | "object.duplicate"
            | "object.paste"
            | "object.set_frame"
            | "object.set_rotation"
            | "object.flip"
            | "object.lock"
            | "object.hide_in_editor"
            | "object.set_opacity"
            | "object.set_outline_join"
            | "object.set_fill_style"
            | "object.set_shadow"
            | "object.set_glow"
            | "object.apply_style"
            | "object.set_drag"
            | "object.set_lottie"
            | "object.set_web_source"
            | "object.set_input"
            | "object.set_marker"
            | "object.set_panel"
            | "object.set_scroll_panel"
            | "object.set_timer"
            | "object.set_meter"
            | "object.set_path"
            | "object.set_crop"
            | "object.set_captions"
            | "object.set_media"
            | "object.add_state"
            | "object.update_state"
            | "object.patch_state"
            | "object.delete_state"
            | "object.duplicate_state"
            | "object.rename_state"
            | "object.reorder_state"
            | "object.state_add_extra"
            | "object.state_update_extra"
            | "object.state_delete_extra"
            | "object.state_reorder_extra"
            | "object.delete"
            | "object.set_animation"
            | "object.add_motion_path"
            | "object.update_motion_path"
            | "object.remove_motion_path"
            | "object.set_autofit"
            | "text.set"
            | "text.style_range"
            | "text.set_paragraph"
            | "text.set_paragraphs"
            | "text.set_link"
            | "text_style.apply"
            | "table.set_cell"
            | "table.insert_row"
            | "table.delete_row"
            | "table.insert_column"
            | "table.delete_column"
            | "table.resize"
            | "table.merge"
            | "table.split"
            | "table.set_borders"
            | "table.set_header_row"
            | "layer.add"
            | "layer.update"
            | "layer.delete"
            | "layer.move"
            | "layer.duplicate"
            | "video.add_cue"
            | "video.set_cue"
            | "video.remove_cue"
            | "slide.update"
            | "slide.set_duration"
            | "slide.set_background"
            | "slide.set_transition"
            | "slide.set_player_overrides"
            | "slide.add_cue"
            | "slide.set_cue"
            | "slide.remove_cue"
    )
}

static REGISTRY: &[Command] = &[
    Command {
        id: "course.info",
        label: "Course info",
        menu: "",
        shortcut: None,
        params: "{}",
        mutates: false,
        run: course::course_info,
    },
    Command {
        id: "course.map",
        label: "Course map",
        menu: "",
        shortcut: None,
        params: "{}",
        mutates: false,
        run: course::course_map,
    },
    Command {
        id: "course.validate",
        label: "Check course",
        menu: "File/Check course",
        shortcut: None,
        params: "{}",
        mutates: false,
        run: course::course_validate,
    },
    Command {
        id: "course.audit",
        label: "Accessibility check",
        menu: "",
        shortcut: None,
        params: "{}",
        mutates: false,
        run: course::course_audit,
    },
    Command {
        id: "course.update",
        label: "Course settings",
        menu: "Project/Course settings…",
        shortcut: None,
        params: r#"{"title"?: string, "id"?: string, "start_slide"?: id|null, "language"?: string|null}"#,
        mutates: true,
        run: course::course_update,
    },
    Command {
        id: "course.set_slide_size",
        label: "Slide size",
        menu: "",
        shortcut: None,
        params: r#"{"w": number, "h": number, "mode": "scale"|"fill"|"keep"}"#,
        mutates: true,
        run: course::course_set_slide_size,
    },
    Command {
        id: "course.set_title",
        label: "Course title",
        menu: "",
        shortcut: None,
        params: r#"{"title": string}"#,
        mutates: true,
        run: course::course_set_title,
    },
    Command {
        id: "course.set_player",
        label: "Player settings",
        menu: "Project/Player settings…",
        shortcut: None,
        params: r#"{"show_bar"?: bool, "show_nav"?: bool, "show_menu"?: bool, "show_seekbar"?: bool, "show_speed"?: bool, "show_fullscreen"?: bool, "show_transcript"?: bool, "show_title"?: bool, "show_volume"?: bool, "show_captions"?: bool, "logo"?: string|null, "navigation"?: "free"|"restricted"|"locked", "next_requires"?: {"kind": "nothing"|"timeline_ends"|"variable", "name"?: string}, "seekbar"?: "free"|"locked"|"until_timeline_ends"}"#,
        mutates: true,
        run: course::course_set_player,
    },
    Command {
        id: "course.set_quiz",
        label: "Quiz settings",
        menu: "Project/Quiz settings…",
        shortcut: None,
        params: r#"{"passing_percent": 0..=100|null}"#,
        mutates: true,
        run: course::course_set_quiz,
    },
    Command {
        id: "course.set_publish_settings",
        label: "Publish settings",
        menu: "",
        shortcut: None,
        params: r#"{"standard"?: string, "lms_status"?: "auto"|"passed_incomplete"|"passed_failed"|"completed_incomplete"|"completed_failed"}"#,
        mutates: true,
        run: course::course_set_publish_settings,
    },
    Command {
        id: "course.set_background_audio",
        label: "Set background audio",
        menu: "",
        shortcut: None,
        params: r#"{"src"?: string | null, "volume"?: number, "loop"?: bool}"#,
        mutates: true,
        run: course::course_set_background_audio,
    },
    Command {
        id: "theme.set_palette",
        label: "Set theme palette",
        menu: "",
        shortcut: None,
        params: r##"{"colors": [{"id"?: id, "name": string, "color": "#RRGGBB"}]}"##,
        mutates: true,
        run: styles::theme_set_palette,
    },
    Command {
        id: "theme.set_palette_color",
        label: "Change palette colour",
        menu: "",
        shortcut: None,
        params: r##"{"id": id, "color": "#RRGGBB", "recolor"?: bool}"##,
        mutates: true,
        run: styles::theme_set_palette_color,
    },
    Command {
        id: "course.recolor",
        label: "Replace colour",
        menu: "",
        shortcut: None,
        params: r##"{"from": "#RRGGBB", "to": "#RRGGBB"}"##,
        mutates: true,
        run: styles::course_recolor,
    },
    Command {
        id: "text_style.create",
        label: "New text style",
        menu: "",
        shortcut: None,
        params: r#"{"name": string, "from_object"?: {"slide": id, "object": id}, "style"?: TextStyle, "paragraphs"?: Paragraph | null}"#,
        mutates: true,
        run: styles::text_style_create,
    },
    Command {
        id: "text_style.update",
        label: "Update text style",
        menu: "",
        shortcut: None,
        params: r#"{"id": id, "name"?: string, "style"?: TextStyle fields, "paragraphs"?: Paragraph | null}"#,
        mutates: true,
        run: styles::text_style_update,
    },
    Command {
        id: "text_style.delete",
        label: "Delete text style",
        menu: "",
        shortcut: None,
        params: r#"{"id": id}"#,
        mutates: true,
        run: styles::text_style_delete,
    },
    Command {
        id: "text_style.apply",
        label: "Apply text style",
        menu: "",
        shortcut: None,
        params: r#"{"id": id, "slide": id, "objects": id[], "clear_overrides"?: bool}"#,
        mutates: true,
        run: styles::text_style_apply,
    },
    Command {
        id: "course.add_finish_button",
        label: "Add finish button",
        menu: "",
        shortcut: None,
        params: "{}",
        mutates: true,
        run: course::course_add_finish_button,
    },
    Command {
        id: "course.set_theme",
        label: "Theme settings",
        menu: "",
        shortcut: None,
        params: r#"{"accent"?: color, "background"?: color, "text"?: color, "font"?: string, "bar"?: color, "bar_text"?: color}"#,
        mutates: true,
        run: course::course_set_theme,
    },
    Command {
        id: "course.add_font",
        label: "Add font",
        menu: "",
        shortcut: None,
        params: r#"{"src": "fonts/<file>.ttf", "family": string, "style": "regular"|"bold"|"italic"|"bold_italic"}"#,
        mutates: true,
        run: course::course_add_font,
    },
    Command {
        id: "course.remove_font",
        label: "Remove font",
        menu: "",
        shortcut: None,
        params: r#"{"family": string, "style"?: style} or {"src": "fonts/<file>.ttf"}"#,
        mutates: true,
        run: course::course_remove_font,
    },
    Command {
        id: "course.export_xliff",
        label: "Export translation",
        menu: "",
        shortcut: None,
        params: r#"{"target_language": string}"#,
        mutates: false,
        run: course::course_export_xliff,
    },
    Command {
        id: "course.import_xliff",
        label: "Import translation",
        menu: "",
        shortcut: None,
        params: r#"{"xliff": string}"#,
        mutates: true,
        run: course::course_import_xliff,
    },
    Command {
        id: "edit.undo",
        label: "Undo",
        menu: "Edit/Undo",
        shortcut: Some("Ctrl+Z"),
        params: "{}",
        mutates: false,
        run: course::edit_undo,
    },
    Command {
        id: "edit.redo",
        label: "Redo",
        menu: "Edit/Redo",
        shortcut: Some("Ctrl+Y"),
        params: "{}",
        mutates: false,
        run: course::edit_redo,
    },
    Command {
        id: "edit.history",
        label: "History",
        menu: "Edit/History…",
        shortcut: None,
        params: "{}",
        mutates: false,
        run: course::edit_history,
    },
    Command {
        id: "scene.add",
        label: "New scene",
        menu: "Insert/New scene",
        shortcut: None,
        params: r#"{"id"?: id, "title"?: string}"#,
        mutates: true,
        run: slide::scene_add,
    },
    Command {
        id: "scene.update",
        label: "Scene properties",
        menu: "",
        shortcut: None,
        params: r#"{"scene": id, "title"?: string}"#,
        mutates: true,
        run: slide::scene_update,
    },
    Command {
        id: "scene.delete",
        label: "Delete scene",
        menu: "Edit/Delete scene",
        shortcut: None,
        params: r#"{"scene": id}"#,
        mutates: true,
        run: slide::scene_delete,
    },
    Command {
        id: "slide.add",
        label: "New slide",
        menu: "Insert/New slide",
        shortcut: Some("Ctrl+M"),
        params: r#"{"id"?: id, "scene"?: id, "after"?: slide id, "title"?: string}"#,
        mutates: true,
        run: slide::slide_add,
    },
    Command {
        id: "slide.update",
        label: "Slide properties",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "title"?: string, "duration_ms"?: u32, "revisit"?: "auto"|"reset"|"resume", "notes"?: string}"#,
        mutates: true,
        run: slide::slide_update,
    },
    Command {
        id: "slide.delete",
        label: "Delete slide",
        menu: "Edit/Delete slide",
        shortcut: None,
        params: r#"{"slide": id}"#,
        mutates: true,
        run: slide::slide_delete,
    },
    Command {
        id: "slide.move",
        label: "Move slide",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "scene"?: id, "index": usize}"#,
        mutates: true,
        run: slide::slide_move,
    },
    Command {
        id: "slide.duplicate",
        label: "Duplicate slide",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "after"?: slide id}"#,
        mutates: true,
        run: slide::slide_duplicate,
    },
    Command {
        id: "slide.paste",
        label: "Paste slide",
        menu: "",
        shortcut: None,
        params: r#"{"scene"?: id, "after"?: slide id, "slide": Slide}"#,
        mutates: true,
        run: slide::slide_paste,
    },
    Command {
        id: "slide.set_duration",
        label: "Slide duration",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "ms": u32}"#,
        mutates: true,
        run: slide::slide_set_duration,
    },
    Command {
        id: "slide.set_background",
        label: "Slide background",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "background": color|null}"#,
        mutates: true,
        run: slide::slide_set_background,
    },
    Command {
        id: "slide.set_transition",
        label: "Slide transition",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "transition": Transition|null}"#,
        mutates: true,
        run: slide::slide_set_transition,
    },
    Command {
        id: "slide.set_player_overrides",
        label: "Slide player settings",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "next_requires"?: {"kind": "nothing"|"timeline_ends"|"variable", "name"?: string}|null, "show_bar"?: bool|null, "show_nav"?: bool|null, "show_seekbar"?: bool|null, "show_menu"?: bool|null}"#,
        mutates: true,
        run: slide::slide_set_player_overrides,
    },
    Command {
        id: "master.add",
        label: "New master",
        menu: "",
        shortcut: None,
        params: r#"{"title"?: string}"#,
        mutates: true,
        run: slide::master_add,
    },
    Command {
        id: "master.delete",
        label: "Delete master",
        menu: "",
        shortcut: None,
        params: r#"{"master": id}"#,
        mutates: true,
        run: slide::master_delete,
    },
    Command {
        id: "slide.set_master",
        label: "Apply master",
        menu: "",
        shortcut: None,
        params: r#"{"slides": id[], "master": id|null}"#,
        mutates: true,
        run: slide::slide_set_master,
    },
    Command {
        id: "bank.create",
        label: "New question bank",
        menu: "",
        shortcut: None,
        params: r#"{"title"?: string}"#,
        mutates: true,
        run: slide::bank_create,
    },
    Command {
        id: "bank.update",
        label: "Question bank properties",
        menu: "",
        shortcut: None,
        params: r#"{"bank": id, "title"?: string}"#,
        mutates: true,
        run: slide::bank_update,
    },
    Command {
        id: "bank.delete",
        label: "Delete question bank",
        menu: "",
        shortcut: None,
        params: r#"{"bank": id}"#,
        mutates: true,
        run: slide::bank_delete,
    },
    Command {
        id: "bank.add_question",
        label: "New bank question",
        menu: "",
        shortcut: None,
        params: r#"{"bank": id, "kind"?: QuestionKind, "title"?: string}"#,
        mutates: true,
        run: slide::bank_add_question,
    },
    Command {
        id: "bank.import_csv",
        label: "Import questions from CSV",
        menu: "",
        shortcut: None,
        params: r#"{"csv": string, "bank"?: string}"#,
        mutates: true,
        run: csv_import::bank_import_csv,
    },
    Command {
        id: "bank.move_slide",
        label: "Move slide to bank",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "bank": id, "index"?: usize}"#,
        mutates: true,
        run: slide::bank_move_slide,
    },
    Command {
        id: "bank.remove_slide",
        label: "Remove bank slide",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id}"#,
        mutates: true,
        run: slide::bank_remove_slide,
    },
    Command {
        id: "draw.insert",
        label: "New draw",
        menu: "",
        shortcut: None,
        params: r#"{"bank": id, "count": u32, "scene"?: id, "index"?: usize}"#,
        mutates: true,
        run: slide::draw_insert,
    },
    Command {
        id: "draw.update",
        label: "Draw properties",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "bank"?: id, "count"?: u32}"#,
        mutates: true,
        run: slide::draw_update,
    },
    Command {
        id: "slide.add_from_layout",
        label: "New slide from layout",
        menu: "",
        shortcut: None,
        params: r#"{"layout": "blank"|"title"|"title_content"|"two_columns"|"picture_text"|"question", "after"?: slide id, "scene"?: id, "master"?: id}"#,
        mutates: true,
        run: slide::slide_add_from_layout,
    },
    Command {
        id: "course.import_outline",
        label: "Slides from outline",
        menu: "File/Import/Slides from outline…",
        shortcut: None,
        params: r#"{"outline": {"title"?: string|null, "scenes": [{"title": string, "slides": [{"title": string, "body"?: [string], "notes"?: string|null, "question"?: {"prompt": string, "choices": [{"text": string, "correct": bool}]}}]}]}, "after"?: slide id}"#,
        mutates: true,
        run: slide::course_import_outline,
    },
    Command {
        id: "template.save",
        label: "Save as template",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "title"?: string}"#,
        mutates: true,
        run: slide::template_save,
    },
    Command {
        id: "template.add_slide",
        label: "New slide from template",
        menu: "",
        shortcut: None,
        params: r#"{"template": id, "after"?: slide id, "scene"?: id}"#,
        mutates: true,
        run: slide::template_add_slide,
    },
    Command {
        id: "template.delete",
        label: "Delete template",
        menu: "",
        shortcut: None,
        params: r#"{"template": id}"#,
        mutates: true,
        run: slide::template_delete,
    },
    Command {
        id: "layer.add",
        label: "New layer",
        menu: "Insert/New layer",
        shortcut: None,
        params: r#"{"id"?: id, "slide": id, "name"?: string}"#,
        mutates: true,
        run: layer::layer_add,
    },
    Command {
        id: "layer.update",
        label: "Layer properties",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "layer": id, "name"?: string, "duration_ms"?: u32, "hide_other_layers"?: bool, "pause_base_timeline"?: bool, "prevent_base_clicks"?: bool, "hide_base_objects"?: bool, "hide_when_timeline_ends"?: bool, "allow_seek"?: "auto"|"yes"|"no", "revisit"?: "auto"|"reset"|"resume"}"#,
        mutates: true,
        run: layer::layer_update,
    },
    Command {
        id: "layer.delete",
        label: "Delete layer",
        menu: "Edit/Delete layer",
        shortcut: None,
        params: r#"{"slide": id, "layer": id}"#,
        mutates: true,
        run: layer::layer_delete,
    },
    Command {
        id: "layer.move",
        label: "Move layer",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "layer": id, "to": usize}"#,
        mutates: true,
        run: layer::layer_move,
    },
    Command {
        id: "layer.duplicate",
        label: "Duplicate layer",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "layer": id}"#,
        mutates: true,
        run: layer::layer_duplicate,
    },
    Command {
        id: "object.add",
        label: "Insert object",
        menu: "",
        shortcut: None,
        params: r#"{"id"?: id, "slide": id, "layer"?: id, "kind": ObjectKind, "frame": Rect, "name"?: string, "timing"?: Timing}"#,
        mutates: true,
        run: object::object_add,
    },
    Command {
        id: "object.update",
        label: "Object properties",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "name"?: string, "kind"?: ObjectKind, "timing"?: Timing, "initial_state"?: string, "focus_order"?: u32|null, "decorative"?: bool, "description"?: string|null}"#,
        mutates: true,
        run: object::object_update,
    },
    Command {
        id: "object.reorder",
        label: "Reorder object",
        menu: "Arrange/Bring forward",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "index": usize}"#,
        mutates: true,
        run: object::object_reorder,
    },
    Command {
        id: "object.align",
        label: "Align objects",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "objects": id[], "edge": "left"|"center"|"right"|"top"|"middle"|"bottom", "to"?: "selection"|"slide"}"#,
        mutates: true,
        run: object::object_align,
    },
    Command {
        id: "object.distribute",
        label: "Distribute objects",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "objects": id[], "axis": "horizontal"|"vertical"}"#,
        mutates: true,
        run: object::object_distribute,
    },
    Command {
        id: "object.match_size",
        label: "Match size",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "objects": id[], "dimension": "width"|"height"|"both"}"#,
        mutates: true,
        run: object::object_match_size,
    },
    Command {
        id: "object.group",
        label: "Group",
        menu: "Arrange/Group",
        shortcut: Some("Ctrl+G"),
        params: r#"{"slide": id, "objects": id[]}"#,
        mutates: true,
        run: object::object_group,
    },
    Command {
        id: "object.ungroup",
        label: "Ungroup",
        menu: "Arrange/Ungroup",
        shortcut: Some("Ctrl+Shift+G"),
        params: r#"{"slide": id, "objects": id[]}"#,
        mutates: true,
        run: object::object_ungroup,
    },
    Command {
        id: "object.duplicate",
        label: "Duplicate",
        menu: "Edit/Duplicate",
        shortcut: Some("Ctrl+D"),
        params: r#"{"slide": id, "object"?: id, "objects"?: id[]}"#,
        mutates: true,
        run: object::object_duplicate,
    },
    Command {
        id: "object.paste",
        label: "Paste",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "layer"?: id, "objects": Object[], "triggers"?: Trigger[], "offset"?: number, "source_slide"?: id}"#,
        mutates: true,
        run: object::object_paste,
    },
    Command {
        id: "object.set_frame",
        label: "Move/resize object",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "frame": Rect}"#,
        mutates: true,
        run: object::object_set_frame,
    },
    Command {
        id: "object.set_rotation",
        label: "Rotate object",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object"?: id, "objects"?: id[], "degrees": number}"#,
        mutates: true,
        run: object::object_set_rotation,
    },
    Command {
        id: "object.flip",
        label: "Flip object",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object"?: id, "objects"?: id[], "axis": "horizontal"|"vertical"}"#,
        mutates: true,
        run: object::object_flip,
    },
    Command {
        id: "object.lock",
        label: "Lock object",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object"?: id, "objects"?: id[], "locked": bool}"#,
        mutates: true,
        run: object::object_lock,
    },
    Command {
        id: "object.hide_in_editor",
        label: "Hide object in editor",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object"?: id, "objects"?: id[], "hidden": bool}"#,
        mutates: true,
        run: object::object_hide_in_editor,
    },
    Command {
        id: "view.set",
        label: "Editor view settings",
        menu: "",
        shortcut: None,
        params: r#"{"show_rulers"?: bool, "show_grid"?: bool, "snap_to_grid"?: bool, "show_guides"?: bool, "base_slide_view"?: "show"|"dim"|"hide", "last_publish"?: {"standard": string, "lms_status"?: "auto"|"passed_incomplete"|"passed_failed"|"completed_incomplete"|"completed_failed"}}"#,
        mutates: true,
        run: course::view_set,
    },
    Command {
        id: "view.guides.add",
        label: "Add guide",
        menu: "",
        shortcut: None,
        params: r#"{"vertical": bool, "at": number}"#,
        mutates: true,
        run: course::view_guides_add,
    },
    Command {
        id: "view.guides.move",
        label: "Move guide",
        menu: "",
        shortcut: None,
        params: r#"{"index": usize, "at": number}"#,
        mutates: true,
        run: course::view_guides_move,
    },
    Command {
        id: "view.guides.remove",
        label: "Remove guide",
        menu: "",
        shortcut: None,
        params: r#"{"index": usize}"#,
        mutates: true,
        run: course::view_guides_remove,
    },
    Command {
        id: "object.set_opacity",
        label: "Set opacity",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object"?: id, "objects"?: id[], "opacity": number}"#,
        mutates: true,
        run: object::object_set_opacity,
    },
    Command {
        id: "object.set_outline_join",
        label: "Set outline corners",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object"?: id, "objects"?: id[], "join": "miter"|"round"|"bevel"}"#,
        mutates: true,
        run: object::object_set_outline_join,
    },
    Command {
        id: "object.set_fill_style",
        label: "Set fill style",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object"?: id, "objects"?: id[], "fill_style": FillStyle|null}"#,
        mutates: true,
        run: object::object_set_fill_style,
    },
    Command {
        id: "object.set_shadow",
        label: "Set shadow",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object"?: id, "objects"?: id[], "shadow": Shadow|null}"#,
        mutates: true,
        run: object::object_set_shadow,
    },
    Command {
        id: "object.set_glow",
        label: "Set glow",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object"?: id, "objects"?: id[], "glow": Glow|null}"#,
        mutates: true,
        run: object::object_set_glow,
    },
    Command {
        id: "object.apply_style",
        label: "Apply style",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "source_slide": id, "source": id, "objects": id[]}"#,
        mutates: true,
        run: object::object_apply_style,
    },
    Command {
        id: "object.set_drag",
        label: "Set drag",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object"?: id, "objects"?: id[], "drag": DragSpec|null}"#,
        mutates: true,
        run: object::object_set_drag,
    },
    Command {
        id: "object.set_lottie",
        label: "Set Lottie playback",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object"?: id, "objects"?: id[], "loop"?: bool, "autoplay"?: bool, "speed"?: number}"#,
        mutates: true,
        run: object::object_set_lottie,
    },
    Command {
        id: "object.set_web_source",
        label: "Set web source",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object"?: id, "objects"?: id[], "source": WebSource}"#,
        mutates: true,
        run: object::object_set_web_source,
    },
    Command {
        id: "object.set_input",
        label: "Set input",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object"?: id, "objects"?: id[], "patch": {variable?, placeholder?, multiline?, max_chars?, min?, max?, step?, orientation?, start_angle?, end_angle?, label?, options?}}"#,
        mutates: true,
        run: object::object_set_input,
    },
    Command {
        id: "object.set_marker",
        label: "Set marker",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object"?: id, "objects"?: id[], "patch": {title?, body?, runs?, paragraphs?, icon?, color?}}"#,
        mutates: true,
        run: object::object_set_marker,
    },
    Command {
        id: "object.set_panel",
        label: "Set object panel",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object"?: id, "objects"?: id[], "panel": id|null}"#,
        mutates: true,
        run: object::object_set_panel,
    },
    Command {
        id: "object.set_scroll_panel",
        label: "Set scrolling panel",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object"?: id, "objects"?: id[], "patch": {content_height?, bar?}}"#,
        mutates: true,
        run: object::object_set_scroll_panel,
    },
    Command {
        id: "object.set_timer",
        label: "Set timer",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object"?: id, "objects"?: id[], "patch": {duration_ms?, format?, autostart?, style?}}"#,
        mutates: true,
        run: object::object_set_timer,
    },
    Command {
        id: "object.set_meter",
        label: "Set progress meter",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object"?: id, "objects"?: id[], "patch": {variable?, min?, max?, style?, show_value?}}"#,
        mutates: true,
        run: object::object_set_meter,
    },
    Command {
        id: "object.set_path",
        label: "Edit path points",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "points": Point[], "start_glue"?: {object, site}|null, "end_glue"?: {object, site}|null}"#,
        mutates: true,
        run: object::object_set_path,
    },
    Command {
        id: "object.set_crop",
        label: "Crop image",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "crop": {x,y,w,h}|null, "frame"?: Rect}"#,
        mutates: true,
        run: object::object_set_crop,
    },
    Command {
        id: "object.set_captions",
        label: "Set captions",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "captions": string|null}"#,
        mutates: true,
        run: object::object_set_captions,
    },
    Command {
        id: "object.set_media",
        label: "Set media options",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "volume"?: number, "trim"?: {"start_ms", "end_ms"?} | null}"#,
        mutates: true,
        run: object::object_set_media,
    },
    Command {
        id: "video.add_cue",
        label: "Add cue point",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "at_ms"?: number, "name"?: string, "pause"?: bool, "id"?: id}"#,
        mutates: true,
        run: video_cues::video_add_cue,
    },
    Command {
        id: "video.set_cue",
        label: "Edit cue point",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "cue": id, "patch": {"at_ms"?: number, "name"?: string, "pause"?: bool}}"#,
        mutates: true,
        run: video_cues::video_set_cue,
    },
    Command {
        id: "video.remove_cue",
        label: "Delete cue point",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "cue": id}"#,
        mutates: true,
        run: video_cues::video_remove_cue,
    },
    Command {
        id: "video.add_question_cue",
        label: "Add question at cue",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "at_ms"?: number, "name"?: string}"#,
        mutates: true,
        run: video_cues::video_add_question_cue,
    },
    Command {
        id: "slide.add_cue",
        label: "Add timeline cue point",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "layer"?: id, "at_ms"?: number, "name"?: string, "id"?: id}"#,
        mutates: true,
        run: video_cues::slide_add_cue,
    },
    Command {
        id: "slide.set_cue",
        label: "Edit timeline cue point",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "layer"?: id, "cue": id, "patch": {"at_ms"?: number, "name"?: string}}"#,
        mutates: true,
        run: video_cues::slide_set_cue,
    },
    Command {
        id: "slide.remove_cue",
        label: "Delete timeline cue point",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "layer"?: id, "cue": id}"#,
        mutates: true,
        run: video_cues::slide_remove_cue,
    },
    Command {
        id: "object.add_state",
        label: "New state",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "state": StateDef}"#,
        mutates: true,
        run: object_states::object_add_state,
    },
    Command {
        id: "object.update_state",
        label: "Update state",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "state": StateDef}"#,
        mutates: true,
        run: object_states::object_update_state,
    },
    Command {
        id: "object.patch_state",
        label: "Edit state",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "name": string, "reset_all"?: bool, "patch"?: object}"#,
        mutates: true,
        run: object_states::object_patch_state,
    },
    Command {
        id: "object.delete_state",
        label: "Delete state",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "name": string}"#,
        mutates: true,
        run: object_states::object_delete_state,
    },
    Command {
        id: "object.duplicate_state",
        label: "Duplicate state",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "name": string, "new_name"?: string}"#,
        mutates: true,
        run: object_states::object_duplicate_state,
    },
    Command {
        id: "object.rename_state",
        label: "Rename state",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "from": string, "to": string}"#,
        mutates: true,
        run: object_states::object_rename_state,
    },
    Command {
        id: "object.reorder_state",
        label: "Reorder state",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "name": string, "index": usize}"#,
        mutates: true,
        run: object_states::object_reorder_state,
    },
    Command {
        id: "object.state_add_extra",
        label: "Add state item",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "state": string, "extra": StateExtra}"#,
        mutates: true,
        run: object_states::object_state_add_extra,
    },
    Command {
        id: "object.state_update_extra",
        label: "Update state item",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "state": string, "extra": id, "patch": object}"#,
        mutates: true,
        run: object_states::object_state_update_extra,
    },
    Command {
        id: "object.state_delete_extra",
        label: "Delete state item",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "state": string, "extra": id}"#,
        mutates: true,
        run: object_states::object_state_delete_extra,
    },
    Command {
        id: "object.state_reorder_extra",
        label: "Reorder state item",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "state": string, "extra": id, "index": usize}"#,
        mutates: true,
        run: object_states::object_state_reorder_extra,
    },
    Command {
        id: "object.delete",
        label: "Delete object",
        menu: "Edit/Delete",
        shortcut: Some("Delete"),
        params: r#"{"slide": id, "object": id}"#,
        mutates: true,
        run: object_states::object_delete,
    },
    Command {
        id: "object.set_animation",
        label: "Set animation",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "which": "entrance"|"exit", "animation": Animation|null}"#,
        mutates: true,
        run: object_states::object_set_animation,
    },
    Command {
        id: "object.add_motion_path",
        label: "Add motion path",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "points"?: Point[], "duration_ms"?: u32, "easing"?: Easing, "relative"?: bool}"#,
        mutates: true,
        run: object_states::object_add_motion_path,
    },
    Command {
        id: "object.update_motion_path",
        label: "Update motion path",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "path": id, "points"?: Point[], "duration_ms"?: u32, "easing"?: Easing, "relative"?: bool}"#,
        mutates: true,
        run: object_states::object_update_motion_path,
    },
    Command {
        id: "object.remove_motion_path",
        label: "Remove motion path",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "path": id}"#,
        mutates: true,
        run: object_states::object_remove_motion_path,
    },
    Command {
        id: "text.set",
        label: "Set text",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "text": string}"#,
        mutates: true,
        run: text::text_set,
    },
    Command {
        id: "text.style_range",
        label: "Style text range",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "start": usize, "end": usize, "bold"?: bool, "italic"?: bool, "underline"?: bool, "strikethrough"?: bool, "baseline"?: "normal"|"superscript"|"subscript", "letter_spacing"?: number, "size"?: number, "color"?: string, "font"?: string}"#,
        mutates: true,
        run: text::text_style_range,
    },
    Command {
        id: "text.set_paragraph",
        label: "Set paragraph style",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "align"?: "left"|"center"|"right", "valign"?: "top"|"middle"|"bottom"}"#,
        mutates: true,
        run: text::text_set_paragraph,
    },
    Command {
        id: "text.set_paragraphs",
        label: "Set paragraph formatting",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "start"?: usize, "end"?: usize, "list"?: "none"|"bullet"|"number", "level"?: 0..4, "line_spacing"?: 0.8..3, "space_before"?: 0..200, "space_after"?: 0..200}"#,
        mutates: true,
        run: text::text_set_paragraphs,
    },
    Command {
        id: "text.set_link",
        label: "Set link",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "start"?: usize, "end"?: usize, "link": Link|null}"#,
        mutates: true,
        run: text::text_set_link,
    },
    Command {
        id: "object.set_autofit",
        label: "Set autofit",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "autofit": "none"|"shrink"}"#,
        mutates: true,
        run: text::object_set_autofit,
    },
    Command {
        id: "table.set_cell",
        label: "Set table cell",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "row": usize, "column": usize, "text"?: string, "runs"?: TextRun[], "paragraphs"?: Paragraph[], "fill"?: Color|null, "valign"?: "top"|"middle"|"bottom"}"#,
        mutates: true,
        run: table::table_set_cell,
    },
    Command {
        id: "table.insert_row",
        label: "Insert table row",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "index"?: usize}"#,
        mutates: true,
        run: table::table_insert_row,
    },
    Command {
        id: "table.delete_row",
        label: "Delete table row",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "index": usize}"#,
        mutates: true,
        run: table::table_delete_row,
    },
    Command {
        id: "table.insert_column",
        label: "Insert table column",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "index"?: usize}"#,
        mutates: true,
        run: table::table_insert_column,
    },
    Command {
        id: "table.delete_column",
        label: "Delete table column",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "index": usize}"#,
        mutates: true,
        run: table::table_delete_column,
    },
    Command {
        id: "table.resize",
        label: "Resize table",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "columns"?: number[], "rows"?: number[]}"#,
        mutates: true,
        run: table::table_resize,
    },
    Command {
        id: "table.merge",
        label: "Merge table cells",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "row": usize, "column": usize, "rows": usize, "columns": usize}"#,
        mutates: true,
        run: table::table_merge,
    },
    Command {
        id: "table.split",
        label: "Split table cell",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "row": usize, "column": usize}"#,
        mutates: true,
        run: table::table_split,
    },
    Command {
        id: "table.set_borders",
        label: "Set table borders",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "outer"?: Outline|null, "inner"?: Outline|null}"#,
        mutates: true,
        run: table::table_set_borders,
    },
    Command {
        id: "table.set_header_row",
        label: "Set header row",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "object": id, "header_row": bool}"#,
        mutates: true,
        run: table::table_set_header_row,
    },
    Command {
        id: "trigger.add",
        label: "New trigger",
        menu: "Insert/Trigger",
        shortcut: None,
        params: r#"{"slide": id, "layer"?: id, "trigger": Trigger, "index"?: usize}"#,
        mutates: true,
        run: trigger::trigger_add,
    },
    Command {
        id: "trigger.update",
        label: "Update trigger",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "layer"?: id, "index": usize, "trigger": Trigger}"#,
        mutates: true,
        run: trigger::trigger_update,
    },
    Command {
        id: "trigger.move",
        label: "Move trigger",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "layer"?: id, "from": usize, "to": usize}"#,
        mutates: true,
        run: trigger::trigger_move,
    },
    Command {
        id: "trigger.delete",
        label: "Delete trigger",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "layer"?: id, "index": usize}"#,
        mutates: true,
        run: trigger::trigger_delete,
    },
    Command {
        id: "resource.add",
        label: "Add resource",
        menu: "",
        shortcut: None,
        params: r#"{"title": string, "target": {"kind": "url", "url": string}|{"kind": "file", "src": string}}"#,
        mutates: true,
        run: course::resource_add,
    },
    Command {
        id: "resource.update",
        label: "Edit resource",
        menu: "",
        shortcut: None,
        params: r#"{"id": id, "title"?: string, "target"?: {"kind": "url", "url": string}|{"kind": "file", "src": string}}"#,
        mutates: true,
        run: course::resource_update,
    },
    Command {
        id: "resource.remove",
        label: "Remove resource",
        menu: "",
        shortcut: None,
        params: r#"{"id": id}"#,
        mutates: true,
        run: course::resource_remove,
    },
    Command {
        id: "resource.move",
        label: "Move resource",
        menu: "",
        shortcut: None,
        params: r#"{"id": id, "to": usize}"#,
        mutates: true,
        run: course::resource_move,
    },
    Command {
        id: "glossary.add",
        label: "Add glossary entry",
        menu: "",
        shortcut: None,
        params: r#"{"term": string, "definition"?: string}"#,
        mutates: true,
        run: course::glossary_add,
    },
    Command {
        id: "glossary.update",
        label: "Edit glossary entry",
        menu: "",
        shortcut: None,
        params: r#"{"id": id, "term"?: string, "definition"?: string}"#,
        mutates: true,
        run: course::glossary_update,
    },
    Command {
        id: "glossary.remove",
        label: "Remove glossary entry",
        menu: "",
        shortcut: None,
        params: r#"{"id": id}"#,
        mutates: true,
        run: course::glossary_remove,
    },
    Command {
        id: "glossary.move",
        label: "Move glossary entry",
        menu: "",
        shortcut: None,
        params: r#"{"id": id, "to": usize}"#,
        mutates: true,
        run: course::glossary_move,
    },
    Command {
        id: "glossary.sort",
        label: "Sort glossary",
        menu: "",
        shortcut: None,
        params: r#"{}"#,
        mutates: true,
        run: course::glossary_sort,
    },
    Command {
        id: "variable.add",
        label: "New variable",
        menu: "Project/Variables",
        shortcut: None,
        params: r#"{"name": string, "value": bool|number|string}"#,
        mutates: true,
        run: course::variable_add,
    },
    Command {
        id: "variable.set_value",
        label: "Set variable value",
        menu: "",
        shortcut: None,
        params: r#"{"name": string, "value": bool|number|string}"#,
        mutates: true,
        run: course::variable_set_value,
    },
    Command {
        id: "variable.delete",
        label: "Delete variable",
        menu: "",
        shortcut: None,
        params: r#"{"name": string}"#,
        mutates: true,
        run: course::variable_delete,
    },
    Command {
        id: "interaction.insert",
        label: "Interaction",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "kind": "tabs"|"accordion"|"flip_cards"|"hotspot_reveal"|"process_steps"|"timeline"|"reveal_grid"|"scenario_choice"}"#,
        mutates: true,
        run: interaction_insert,
    },
    Command {
        id: "question.insert",
        label: "Question",
        menu: "Insert/Question",
        shortcut: None,
        params: r#"{"slide": id, "kind"?: QuestionKind}"#,
        mutates: true,
        run: question_insert,
    },
    Command {
        id: "results.insert",
        label: "Results slide",
        menu: "Insert/Results slide",
        shortcut: None,
        params: r#"{"after"?: slide id}"#,
        mutates: true,
        run: slide::results_insert,
    },
    Command {
        id: "certificate.insert",
        label: "Certificate slide",
        menu: "Insert/Certificate slide",
        shortcut: None,
        params: r#"{"after"?: slide id}"#,
        mutates: true,
        run: slide::certificate_insert,
    },
    Command {
        id: "question.add_submit_button",
        label: "Add submit button",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id}"#,
        mutates: true,
        run: question_add_submit_button,
    },
    Command {
        id: "question.add_feedback_layers",
        label: "Add feedback layers",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id}"#,
        mutates: true,
        run: question_add_feedback_layers,
    },
    Command {
        id: "question.add_try_again_layer",
        label: "Add try again layer",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id}"#,
        mutates: true,
        run: question_add_try_again_layer,
    },
    Command {
        id: "question.set",
        label: "Set question",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "question": Question}"#,
        mutates: true,
        run: question_set,
    },
    Command {
        id: "question.delete",
        label: "Delete question",
        menu: "Edit/Delete question",
        shortcut: None,
        params: r#"{"slide": id}"#,
        mutates: true,
        run: question_delete,
    },
    Command {
        id: "question.set_kind",
        label: "Set question kind",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "kind": QuestionKind}"#,
        mutates: true,
        run: question_set_kind,
    },
    Command {
        id: "question.add_accepted",
        label: "Add accepted answer",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "text"?: string}"#,
        mutates: true,
        run: question_add_accepted,
    },
    Command {
        id: "question.remove_accepted",
        label: "Remove accepted answer",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "index": number}"#,
        mutates: true,
        run: question_remove_accepted,
    },
    Command {
        id: "question.update_accepted",
        label: "Update accepted answer",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "index": number, "text": string}"#,
        mutates: true,
        run: question_update_accepted,
    },
    Command {
        id: "question.set_case_sensitive",
        label: "Set case sensitive",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "case_sensitive": bool}"#,
        mutates: true,
        run: question_set_case_sensitive,
    },
    Command {
        id: "question.add_pair",
        label: "Add match pair",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "left"?: string, "right"?: string}"#,
        mutates: true,
        run: question_add_pair,
    },
    Command {
        id: "question.remove_pair",
        label: "Remove match pair",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "id": string}"#,
        mutates: true,
        run: question_remove_pair,
    },
    Command {
        id: "question.update_pair",
        label: "Update match pair",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "id": string, "left"?: string, "right"?: string}"#,
        mutates: true,
        run: question_update_pair,
    },
    Command {
        id: "question.reorder_pair",
        label: "Reorder match pair",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "id": string, "to": number}"#,
        mutates: true,
        run: question_reorder_pair,
    },
    Command {
        id: "question.reorder_sequence",
        label: "Reorder sequence item",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "id": string, "to": number}"#,
        mutates: true,
        run: question_reorder_sequence,
    },
    Command {
        id: "question.set_hotspot_image",
        label: "Set hotspot image",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "image": string}"#,
        mutates: true,
        run: question_set_hotspot_image,
    },
    Command {
        id: "question.add_region",
        label: "Add hotspot region",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "shape"?: HotspotShape, "x"?: number, "y"?: number, "w"?: number, "h"?: number, "correct"?: bool}"#,
        mutates: true,
        run: question_add_region,
    },
    Command {
        id: "question.update_region",
        label: "Update hotspot region",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "id": string, "shape"?: HotspotShape, "x"?: number, "y"?: number, "w"?: number, "h"?: number, "correct"?: bool}"#,
        mutates: true,
        run: question_update_region,
    },
    Command {
        id: "question.remove_region",
        label: "Remove hotspot region",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "id": string}"#,
        mutates: true,
        run: question_remove_region,
    },
    Command {
        id: "question.set_markers",
        label: "Set hotspot markers",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "markers": number}"#,
        mutates: true,
        run: question_set_markers,
    },
    Command {
        id: "question.add_drag_item",
        label: "Add drag item",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "item": string, "target": string}"#,
        mutates: true,
        run: question_add_drag_item,
    },
    Command {
        id: "question.update_drag_item",
        label: "Update drag item",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "item": string, "target": string}"#,
        mutates: true,
        run: question_update_drag_item,
    },
    Command {
        id: "question.remove_drag_item",
        label: "Remove drag item",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "item": string}"#,
        mutates: true,
        run: question_remove_drag_item,
    },
    Command {
        id: "question.set_drop_targets",
        label: "Set drop targets",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "targets": [string]}"#,
        mutates: true,
        run: question_set_drop_targets,
    },
    Command {
        id: "question.set_drag_options",
        label: "Set drag options",
        menu: "",
        shortcut: None,
        params: r#"{"slide": id, "snap_to_center"?: bool, "return_if_wrong"?: bool, "one_per_target"?: bool}"#,
        mutates: true,
        run: question_set_drag_options,
    },
    Command {
        id: "course.find",
        label: "Find and replace",
        menu: "Edit/Find and replace…",
        shortcut: Some("Ctrl+F"),
        params: r#"{"query": string, "case"?: bool, "whole_word"?: bool}"#,
        mutates: false,
        run: course::course_find,
    },
    Command {
        id: "course.replace",
        label: "Replace",
        menu: "",
        shortcut: None,
        params: r#"{"query": string, "replacement": string, "case"?: bool, "whole_word"?: bool, "only"?: [{"slide"?: id, "object"?: id, "layer"?: id, "field": string}]}"#,
        mutates: true,
        run: course::course_replace,
    },
];

// ---------------------------------------------------------------- params

pub(super) fn bad(cmd: &str, message: impl Into<String>) -> EngineError {
    EngineError::BadParams {
        command: cmd.to_owned(),
        message: message.into(),
    }
}

pub(super) fn obj<'a>(cmd: &str, p: &'a Json) -> Result<&'a serde_json::Map<String, Json>> {
    p.as_object()
        .ok_or_else(|| bad(cmd, "params must be a JSON object"))
}

pub(super) fn opt<T: DeserializeOwned>(cmd: &str, p: &Json, key: &str) -> Result<Option<T>> {
    match obj(cmd, p)?.get(key) {
        None | Some(Json::Null) => Ok(None),
        Some(v) => serde_json::from_value(v.clone())
            .map(Some)
            .map_err(|e| bad(cmd, format!("`{key}`: {e}"))),
    }
}

pub(super) fn req<T: DeserializeOwned>(cmd: &str, p: &Json, key: &str) -> Result<T> {
    opt(cmd, p, key)?.ok_or_else(|| bad(cmd, format!("missing `{key}`")))
}

pub(super) fn text(cmd: &str, s: String, key: &str) -> Result<String> {
    if s.len() > MAX_TEXT {
        return Err(bad(cmd, format!("`{key}` is longer than {MAX_TEXT} bytes")));
    }
    Ok(s)
}

pub(super) fn validate_id(cmd: &str, id: &str) -> Result<()> {
    if id.is_empty() {
        return Err(bad(cmd, "id cannot be empty"));
    }
    if id.len() > 64 {
        return Err(bad(cmd, "id is longer than 64 characters"));
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(bad(
            cmd,
            format!("id {id:?} must contain only [A-Za-z0-9_-]"),
        ));
    }
    Ok(())
}

/// `prefix1`, `prefix2`, … : the first not already taken.
pub(super) fn fresh_id<'a>(prefix: &str, taken: impl Iterator<Item = &'a str>) -> String {
    let taken: std::collections::BTreeSet<&str> = taken.collect();
    (1u64..)
        .map(|n| format!("{prefix}{n}"))
        .find(|id| !taken.contains(id.as_str()))
        .unwrap_or_else(|| prefix.to_owned())
}

/// The first `prefix<N>_explored` name not already used by a course variable.
pub(super) fn fresh_explored_variable(course: &Course, prefix: &str) -> String {
    let taken: BTreeSet<&str> = course.variables.iter().map(|v| v.name.as_str()).collect();
    (1u64..)
        .map(|n| format!("{prefix}{n}_explored"))
        .find(|name| !taken.contains(name.as_str()))
        .unwrap_or_else(|| format!("{prefix}_explored"))
}

pub(super) fn slide_mut<'a>(s: &'a mut Session, id: &str) -> Result<&'a mut Slide> {
    s.course
        .slide_mut(id)
        .ok_or_else(|| EngineError::NotFound(format!("slide {id:?} does not exist")))
}

pub(super) fn layer_mut<'a>(slide: &'a mut Slide, id: &str) -> Result<&'a mut Layer> {
    let sid = slide.id.clone();
    slide
        .layer_mut(id)
        .ok_or_else(|| EngineError::NotFound(format!("layer {id:?} is not on slide {sid:?}")))
}

fn slide_loc_path_prefix(loc: crate::delta::SlideLoc) -> String {
    match loc {
        crate::delta::SlideLoc::SceneSlide { scene, slide } => {
            format!("/scenes/{scene}/slides/{slide}")
        }
        crate::delta::SlideLoc::Master(idx) => format!("/masters/{idx}"),
        crate::delta::SlideLoc::Template(idx) => format!("/templates/{idx}"),
        crate::delta::SlideLoc::BankSlide { bank, slide } => {
            format!("/banks/{bank}/slides/{slide}")
        }
    }
}

/// Reject the edit if it took a valid course to an invalid one.
pub(super) fn keep_valid(s: &mut Session, was_valid: bool) -> Result<()> {
    if !was_valid || s.skip_undo {
        return Ok(());
    }
    let first_error = if let Some(loc) = s.active_single_slide
        && let Some(slide) = crate::delta::slide_at(&s.course, loc)
    {
        let prefix = slide_loc_path_prefix(loc);
        s.course.validate_slide_errors(slide, &prefix)
    } else {
        #[cfg(debug_assertions)]
        crate::test_counts::note_full_course_validation();
        s.course.first_error()
    };
    match first_error {
        Some(i) => Err(EngineError::Invalid(format!("{} ({})", i.message, i.path))),
        None => {
            s.mark_validated_valid();
            Ok(())
        }
    }
}

// ---------------------------------------------------------------- commands

pub(super) fn validate_color(cmd: &'static str, c: &Color) -> Result<()> {
    if !c.is_valid() {
        return Err(bad(cmd, format!("invalid color hex {:?}", c.0)));
    }
    Ok(())
}

/// The default question `question.insert` starts from, matching the editor's old
/// `insert_question` defaults.
fn default_multiple_choice_question() -> Question {
    Question {
        kind: QuestionKind::MultipleChoice,
        prompt: "Enter question prompt here".to_owned(),
        choices: vec![
            Choice {
                id: "a".to_owned(),
                text: "Choice 1".to_owned(),
                correct: true,
                points: 0,
            },
            Choice {
                id: "b".to_owned(),
                text: "Choice 2".to_owned(),
                correct: false,
                points: 0,
            },
        ],
        points: 10,
        scoring: Scoring::default(),
        attempts: 1,
        shuffle: false,
        correct_layer: None,
        incorrect_layer: None,
        try_again_layer: None,
        accepted: Vec::new(),
        case_sensitive: false,
        pairs: Vec::new(),
        hotspot_image: None,
        regions: Vec::new(),
        markers: 1,
        drag_items: Vec::new(),
        drop_targets: Vec::new(),
        snap_to_center: true,
        return_if_wrong: false,
        one_per_target: true,
        correct_when: None,
    }
}

/// Add the placeholder objects a hotspot or drag-and-drop question needs so the
/// course stays valid before the author picks real content in the panel.
fn ensure_question_support(slide: &mut Slide, kind: QuestionKind, theme: &Theme) {
    match kind {
        QuestionKind::Hotspot => {
            let has_image = slide
                .objects
                .iter()
                .any(|object| matches!(object.kind, ObjectKind::Image { .. }));
            if !has_image {
                let id = fresh_id("image", slide.local_ids());
                slide.objects.push(Object {
                    id,
                    name: "Hotspot image".to_owned(),
                    kind: ObjectKind::Image {
                        src: "placeholder.png".to_owned(),
                        alt: "Choose an image for the hotspot".to_owned(),
                        crop: None,
                        outline: None,
                        shape: None,
                    },
                    frame: Rect {
                        x: 240.0,
                        y: 180.0,
                        w: 800.0,
                        h: 360.0,
                    },
                    timing: Timing::default(),
                    states: Vec::new(),
                    initial_state: "normal".into(),
                    entrance: None,
                    exit: None,
                    motion_paths: Vec::new(),
                    focus_order: None,
                    decorative: false,
                    description: None,
                    group: None,
                    panel: None,
                    rotation: 0.0,
                    flip_h: false,
                    flip_v: false,
                    opacity: 1.0,
                    shadow: None,
                    glow: None,
                    drag: None,
                    locked: false,
                    editor_hidden: false,
                });
            }
        }
        QuestionKind::DragDrop => {
            let has_named =
                |slide: &Slide, name: &str| slide.objects.iter().any(|object| object.name == name);
            if !has_named(slide, "Drag me") {
                let id = fresh_id("button", slide.local_ids());
                slide.objects.push(Object {
                    id,
                    name: "Drag me".to_owned(),
                    kind: ObjectKind::Button {
                        label: "Drag me".to_owned(),
                        fill: theme.accent.clone(),
                        fill_style: None,
                    },
                    frame: Rect {
                        x: 200.0,
                        y: 240.0,
                        w: 200.0,
                        h: 100.0,
                    },
                    timing: Timing::default(),
                    states: Vec::new(),
                    initial_state: "normal".into(),
                    entrance: None,
                    exit: None,
                    motion_paths: Vec::new(),
                    focus_order: None,
                    decorative: false,
                    description: None,
                    group: None,
                    panel: None,
                    rotation: 0.0,
                    flip_h: false,
                    flip_v: false,
                    opacity: 1.0,
                    shadow: None,
                    glow: None,
                    drag: None,
                    locked: false,
                    editor_hidden: false,
                });
            }
            if !has_named(slide, "Drop target") {
                let target_frame = Rect {
                    x: 560.0,
                    y: 210.0,
                    w: 280.0,
                    h: 160.0,
                };
                let id = fresh_id("shape", slide.local_ids());
                slide.objects.push(Object {
                    id,
                    name: "Drop target".to_owned(),
                    kind: ObjectKind::Shape {
                        shape: ShapeKind::RoundedRectangle,
                        fill: theme.accent.clone(),
                        fill_style: None,
                        outline: None,
                        text: None,
                        adjust: Vec::new(),
                    },
                    frame: target_frame,
                    timing: Timing::default(),
                    states: Vec::new(),
                    initial_state: "normal".into(),
                    entrance: None,
                    exit: None,
                    motion_paths: Vec::new(),
                    focus_order: None,
                    decorative: false,
                    description: None,
                    group: None,
                    panel: None,
                    rotation: 0.0,
                    flip_h: false,
                    flip_v: false,
                    opacity: 1.0,
                    shadow: None,
                    glow: None,
                    drag: None,
                    locked: false,
                    editor_hidden: false,
                });
                let label_id = fresh_id("text", slide.local_ids());
                slide.objects.push(Object {
                    id: label_id,
                    name: "Drop target label".to_owned(),
                    kind: ObjectKind::Text {
                        text: "Drop here".to_owned(),
                        style: coursecraft_course::TextStyle {
                            size: 18.0,
                            bold: true,
                            color: Color("#FFFFFF".to_owned()),
                            align: Align::Center,
                            valign: VAlign::Middle,
                            ..coursecraft_course::TextStyle::default()
                        },
                        runs: Vec::new(),
                        paragraphs: Vec::new(),
                        autofit: Autofit::None,
                        text_style: None,
                        outline: None,
                    },
                    frame: Rect {
                        x: target_frame.x + 40.0,
                        y: target_frame.y + target_frame.h * 0.5 - 20.0,
                        w: target_frame.w - 80.0,
                        h: 40.0,
                    },
                    timing: Timing::default(),
                    states: Vec::new(),
                    initial_state: "normal".into(),
                    entrance: None,
                    exit: None,
                    motion_paths: Vec::new(),
                    focus_order: None,
                    decorative: false,
                    description: None,
                    group: None,
                    panel: None,
                    rotation: 0.0,
                    flip_h: false,
                    flip_v: false,
                    opacity: 1.0,
                    shadow: None,
                    glow: None,
                    drag: None,
                    locked: false,
                    editor_hidden: false,
                });
            }
        }
        _ => {}
    }
}

fn submit_button_frame(slide_size: Size) -> Rect {
    const MARGIN: f32 = 40.0;
    const W: f32 = 160.0;
    const H: f32 = 48.0;
    let max_x = (slide_size.w as f32 - MARGIN - W).max(0.0);
    let max_y = (slide_size.h as f32 - MARGIN - H).max(0.0);
    Rect {
        x: max_x,
        y: max_y,
        w: W,
        h: H,
    }
}

fn frames_overlap(a: Rect, b: Rect) -> bool {
    let finite = |r: Rect| r.x.is_finite() && r.y.is_finite() && r.w.is_finite() && r.h.is_finite();
    finite(a)
        && finite(b)
        && a.w > 0.0
        && a.h > 0.0
        && b.w > 0.0
        && b.h > 0.0
        && a.x < b.x + b.w
        && b.x < a.x + a.w
        && a.y < b.y + b.h
        && b.y < a.y + a.h
}

/// Bottom-right Finish button, shifted left of any button it would cover.
fn finish_button_frame(slide: &Slide, slide_size: Size) -> Rect {
    const MARGIN: f32 = 40.0;
    const W: f32 = 160.0;
    const H: f32 = 48.0;
    const GAP: f32 = 12.0;
    let mut x = (slide_size.w as f32 - MARGIN - W).max(0.0);
    let y = (slide_size.h as f32 - MARGIN - H).max(0.0);
    for _ in 0..64 {
        let candidate = Rect { x, y, w: W, h: H };
        let covering = slide
            .objects
            .iter()
            .filter(|object| matches!(object.kind, ObjectKind::Button { .. }))
            .map(|object| object.frame)
            .find(|frame| frames_overlap(candidate, *frame));
        let Some(frame) = covering else {
            break;
        };
        x = (frame.x - GAP - W).max(MARGIN);
    }
    Rect { x, y, w: W, h: H }
}

fn add_finish_button(slide: &mut Slide, theme: &Theme, slide_size: Size) -> String {
    let id = fresh_id("button", slide.local_ids());
    slide.objects.push(Object {
        id: id.clone(),
        name: "Finish".to_owned(),
        kind: ObjectKind::Button {
            label: "Finish".to_owned(),
            fill: theme.accent.clone(),
            fill_style: None,
        },
        frame: finish_button_frame(slide, slide_size),
        timing: Timing::default(),
        states: Vec::new(),
        initial_state: "normal".into(),
        entrance: None,
        exit: None,
        motion_paths: Vec::new(),
        focus_order: None,
        decorative: false,
        description: None,
        group: None,
        panel: None,
        rotation: 0.0,
        flip_h: false,
        flip_v: false,
        opacity: 1.0,
        shadow: None,
        glow: None,
        drag: None,
        locked: false,
        editor_hidden: false,
    });
    slide.triggers.push(Trigger {
        action: Action::SubmitResults,
        when: Event::Click { object: id.clone() },
        condition: None,
        disabled: false,
    });
    slide.triggers.push(Trigger {
        action: Action::ExitCourse,
        when: Event::Click { object: id.clone() },
        condition: None,
        disabled: false,
    });
    id
}

/// Remove submit-event `submit_question` triggers, which no player control can
/// ever fire, and add a Submit button plus a click trigger that grades the question.
fn add_submit_button(slide: &mut Slide, theme: &Theme, slide_size: Size) -> String {
    let is_dead_submit = |trigger: &Trigger| {
        trigger.action == Action::SubmitQuestion && trigger.when == Event::Submit
    };
    slide.triggers.retain(|trigger| !is_dead_submit(trigger));
    for layer in &mut slide.layers {
        layer.triggers.retain(|trigger| !is_dead_submit(trigger));
    }

    let id = fresh_id("button", slide.local_ids());
    slide.objects.push(Object {
        id: id.clone(),
        name: "Submit".to_owned(),
        kind: ObjectKind::Button {
            label: "Submit".to_owned(),
            fill: theme.accent.clone(),
            fill_style: None,
        },
        frame: submit_button_frame(slide_size),
        timing: Timing::default(),
        states: Vec::new(),
        initial_state: "normal".into(),
        entrance: None,
        exit: None,
        motion_paths: Vec::new(),
        focus_order: None,
        decorative: false,
        description: None,
        group: None,
        panel: None,
        rotation: 0.0,
        flip_h: false,
        flip_v: false,
        opacity: 1.0,
        shadow: None,
        glow: None,
        drag: None,
        locked: false,
        editor_hidden: false,
    });
    slide.triggers.push(Trigger {
        action: Action::SubmitQuestion,
        when: Event::Click { object: id.clone() },
        condition: None,
        disabled: false,
    });
    id
}

/// Where a feedback message can sit without covering the question's response
/// rows. Falls back above the prompt when a tall question leaves no room below.
fn feedback_text_frame(slide_size: Size, question: Option<&Question>) -> Rect {
    const MARGIN: f32 = 24.0;
    const ROW_START: f32 = 160.0;
    const ROW_H: f32 = 56.0;
    const ROW_GAP: f32 = 16.0;
    const MIN_H: f32 = 28.0;
    const MIN_GAP: f32 = 4.0;
    let w = (slide_size.w as f32 - 160.0).clamp(0.0, 900.0);
    let slide_h = slide_size.h as f32;
    let below = question.map_or(ROW_START - 20.0, |q| {
        let rows = match q.kind {
            QuestionKind::MultipleChoice
            | QuestionKind::MultipleResponse
            | QuestionKind::TrueFalse
            | QuestionKind::Sequence
            | QuestionKind::Likert => q.choices.len(),
            QuestionKind::FillIn
            | QuestionKind::Matching
            | QuestionKind::ShortAnswer
            | QuestionKind::Essay => q.pairs.len().max(1),
            QuestionKind::Hotspot | QuestionKind::DragDrop => 0,
            QuestionKind::Freeform => 0,
        };
        if rows == 0 {
            ROW_START - 20.0
        } else {
            let available = (slide_h - ROW_START - MARGIN).max(0.0);
            let (row_h, stride) = if (rows as f32) * (ROW_H + ROW_GAP) <= available {
                (ROW_H, ROW_H + ROW_GAP)
            } else {
                let min_stride = MIN_H + MIN_GAP;
                if !available.is_finite() || (rows as f32) * min_stride > available {
                    (MIN_H, min_stride)
                } else {
                    let stride = available / rows as f32;
                    let h = (stride - MIN_GAP).clamp(MIN_H, ROW_H);
                    (h, h + (stride - h).max(0.0))
                }
            };
            ROW_START + stride * (rows.saturating_sub(1)) as f32 + row_h
        }
    });
    let h = 60.0;
    let y = below + 16.0;
    if y + h <= slide_h - MARGIN {
        Rect { x: 80.0, y, w, h }
    } else {
        Rect {
            x: 80.0,
            y: 10.0,
            w,
            h: 40.0,
        }
    }
}

/// Add the two hidden feedback layers and point the question at them.
fn add_feedback_layers(slide: &mut Slide, theme: &Theme, slide_size: Size) -> (String, String) {
    let frame = feedback_text_frame(slide_size, slide.question.as_ref());
    let make_layer = |slide: &mut Slide, name: &str, text: &str| {
        let layer_id = fresh_id(name, slide.local_ids());
        let text_id = fresh_id("feedback_text", slide.local_ids());
        slide.layers.push(Layer {
            id: layer_id.clone(),
            name: name.to_owned(),
            duration_ms: 5000,
            objects: vec![Object {
                id: text_id,
                name: format!("{name} feedback"),
                kind: ObjectKind::Text {
                    text: text.to_owned(),
                    style: coursecraft_course::TextStyle {
                        size: 28.0,
                        bold: true,
                        color: theme.text.clone(),
                        ..coursecraft_course::TextStyle::default()
                    },
                    runs: Vec::new(),
                    paragraphs: Vec::new(),
                    autofit: Autofit::None,
                    text_style: None,
                    outline: None,
                },
                frame,
                timing: Timing::default(),
                states: Vec::new(),
                initial_state: "normal".into(),
                entrance: None,
                exit: None,
                motion_paths: Vec::new(),
                focus_order: None,
                decorative: false,
                description: None,
                group: None,
                panel: None,
                rotation: 0.0,
                flip_h: false,
                flip_v: false,
                opacity: 1.0,
                shadow: None,
                glow: None,
                drag: None,
                locked: false,
                editor_hidden: false,
            }],
            triggers: Vec::new(),
            hide_other_layers: true,
            pause_base_timeline: false,
            prevent_base_clicks: false,
            hide_base_objects: false,
            hide_when_timeline_ends: false,
            allow_seek: SeekMode::Auto,
            revisit: RevisitMode::Auto,
            cue_points: Vec::new(),
        });
        layer_id
    };
    let correct = make_layer(slide, "Correct", "Correct!");
    let incorrect = make_layer(slide, "Incorrect", "Not quite.");
    if let Some(question) = slide.question.as_mut() {
        question.correct_layer = Some(correct.clone());
        question.incorrect_layer = Some(incorrect.clone());
    }
    (correct, incorrect)
}

/// Add a hidden Try Again layer with a message and a "Try Again" button wired
/// to hide the layer, and point the question's `try_again_layer` at it.
fn add_try_again_layer(slide: &mut Slide, theme: &Theme, slide_size: Size) -> String {
    let text_frame = feedback_text_frame(slide_size, slide.question.as_ref());
    let button_w = 160.0_f32.min(text_frame.w.max(80.0));
    let button_h = 44.0;
    let slide_h = slide_size.h.max(1) as f32;
    let button_y = (text_frame.y + text_frame.h + 12.0).min((slide_h - button_h - 16.0).max(0.0));
    let button_frame = Rect {
        x: text_frame.x,
        y: button_y,
        w: button_w,
        h: button_h,
    };
    let layer_id = fresh_id("try_again", slide.local_ids());
    let text_id = fresh_id("try_again_text", slide.local_ids());
    let button_id = fresh_id("try_again_button", slide.local_ids());
    let objects = vec![
        Object {
            id: text_id,
            name: "Try Again feedback".to_owned(),
            kind: ObjectKind::Text {
                text: "Not quite — give it another try.".to_owned(),
                style: coursecraft_course::TextStyle {
                    size: 28.0,
                    bold: true,
                    color: theme.text.clone(),
                    ..coursecraft_course::TextStyle::default()
                },
                runs: Vec::new(),
                paragraphs: Vec::new(),
                autofit: Autofit::None,
                text_style: None,
                outline: None,
            },
            frame: text_frame,
            timing: Timing::default(),
            states: Vec::new(),
            initial_state: "normal".into(),
            entrance: None,
            exit: None,
            motion_paths: Vec::new(),
            focus_order: None,
            decorative: false,
            description: None,
            group: None,
            panel: None,
            rotation: 0.0,
            flip_h: false,
            flip_v: false,
            opacity: 1.0,
            shadow: None,
            glow: None,
            drag: None,
            locked: false,
            editor_hidden: false,
        },
        results_button(
            button_id.clone(),
            "Try Again",
            theme.accent.clone(),
            button_frame,
        ),
    ];
    let triggers = vec![Trigger {
        when: Event::Click { object: button_id },
        action: Action::HideLayer { layer: None },
        condition: None,
        disabled: false,
    }];
    slide.layers.push(Layer {
        id: layer_id.clone(),
        name: "Try Again".to_owned(),
        duration_ms: 5000,
        objects,
        triggers,
        hide_other_layers: true,
        pause_base_timeline: false,
        prevent_base_clicks: true,
        hide_base_objects: false,
        hide_when_timeline_ends: false,
        allow_seek: SeekMode::Auto,
        revisit: RevisitMode::Auto,
        cue_points: Vec::new(),
    });
    if let Some(question) = slide.question.as_mut() {
        question.try_again_layer = Some(layer_id.clone());
    }
    layer_id
}

fn question_add_feedback_layers(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "question.add_feedback_layers";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let theme = s.course.theme.clone();
    let slide_size = s.course.slide_size;
    let slide = slide_mut(s, &sid)?;
    if slide.question.is_none() {
        return Err(EngineError::NotFound(format!(
            "slide {sid:?} has no question"
        )));
    }
    let (correct, incorrect) = add_feedback_layers(slide, &theme, slide_size);
    keep_valid(s, was_valid)?;
    Ok(json!({ "correct": correct, "incorrect": incorrect }))
}

fn question_add_try_again_layer(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "question.add_try_again_layer";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let theme = s.course.theme.clone();
    let slide_size = s.course.slide_size;
    let slide = slide_mut(s, &sid)?;
    if slide.question.is_none() {
        return Err(EngineError::NotFound(format!(
            "slide {sid:?} has no question"
        )));
    }
    let layer = add_try_again_layer(slide, &theme, slide_size);
    keep_valid(s, was_valid)?;
    Ok(json!({ "layer": layer }))
}

/// A plain text object for an inserted interaction.
fn interaction_text(id: String, name: String, text: String, frame: Rect, theme: &Theme) -> Object {
    Object {
        id,
        name,
        kind: ObjectKind::Text {
            text,
            style: TextStyle {
                size: 24.0,
                color: theme.text.clone(),
                ..TextStyle::default()
            },
            runs: Vec::new(),
            paragraphs: Vec::new(),
            autofit: Autofit::None,
            text_style: None,
            outline: None,
        },
        frame,
        timing: Timing::default(),
        entrance: None,
        exit: None,
        motion_paths: Vec::new(),
        states: Vec::new(),
        initial_state: "normal".into(),
        focus_order: None,
        decorative: false,
        description: None,
        group: None,
        panel: None,
        rotation: 0.0,
        flip_h: false,
        flip_v: false,
        opacity: 1.0,
        shadow: None,
        glow: None,
        drag: None,
        locked: false,
        editor_hidden: false,
    }
}

/// A button with a `selected` state in the theme accent.
fn interaction_button(
    id: String,
    name: String,
    label: String,
    frame: Rect,
    selected: bool,
    theme: &Theme,
) -> Object {
    // Button labels are white, so the unselected fill must stay dark enough to read.
    let normal = tint(theme.text.clone(), 0x88);
    Object {
        id,
        name,
        kind: ObjectKind::Button {
            label,
            fill: normal,
            fill_style: None,
        },
        frame,
        timing: Timing::default(),
        entrance: None,
        exit: None,
        motion_paths: Vec::new(),
        states: vec![StateDef {
            name: "selected".into(),
            fill: Some(theme.accent.clone()),
            ..Default::default()
        }],
        initial_state: if selected {
            "selected".into()
        } else {
            "normal".into()
        },
        focus_order: None,
        decorative: false,
        description: None,
        group: None,
        panel: None,
        rotation: 0.0,
        flip_h: false,
        flip_v: false,
        opacity: 1.0,
        shadow: None,
        glow: None,
        drag: None,
        locked: false,
        editor_hidden: false,
    }
}

/// A theme colour with its alpha replaced, so normal states stay readable on
/// any background without inventing a new palette.
fn tint(color: Color, alpha: u8) -> Color {
    let hex = color.0.strip_prefix('#').unwrap_or("");
    let first_six: String = hex.chars().take(6).collect();
    if first_six.len() == 6 && first_six.chars().all(|c| c.is_ascii_hexdigit()) {
        Color(format!("#{first_six}{alpha:02X}"))
    } else {
        color
    }
}

fn interaction_trigger(action: Action, when: Event) -> Trigger {
    Trigger {
        action,
        when,
        condition: None,
        disabled: false,
    }
}

/// A trigger that only fires when `condition` holds.
fn interaction_conditional_trigger(action: Action, when: Event, condition: Condition) -> Trigger {
    Trigger {
        action,
        when,
        condition: Some(condition),
        disabled: false,
    }
}

/// A shape with optional centred text and a caller-supplied set of states.
#[allow(clippy::too_many_arguments)]
fn interaction_shape(
    id: String,
    name: String,
    shape: ShapeKind,
    fill: Color,
    outline: Option<Outline>,
    text: Option<(String, f32, bool)>,
    states: Vec<StateDef>,
    frame: Rect,
    theme: &Theme,
) -> Object {
    Object {
        id,
        name,
        kind: ObjectKind::Shape {
            shape,
            fill,
            fill_style: None,
            outline,
            text: text.map(|(text, size, bold)| ShapeText {
                text,
                style: TextStyle {
                    font: theme.font.clone(),
                    size,
                    color: theme.text.clone(),
                    bold,
                    align: Align::Center,
                    valign: VAlign::Middle,
                    ..TextStyle::default()
                },
                runs: Vec::new(),
                paragraphs: Vec::new(),
                autofit: Autofit::None,
            }),
            adjust: Vec::new(),
        },
        frame,
        timing: Timing::default(),
        entrance: None,
        exit: None,
        motion_paths: Vec::new(),
        states,
        initial_state: "normal".into(),
        focus_order: None,
        decorative: false,
        description: None,
        group: None,
        panel: None,
        rotation: 0.0,
        flip_h: false,
        flip_v: false,
        opacity: 1.0,
        shadow: None,
        glow: None,
        drag: None,
        locked: false,
        editor_hidden: false,
    }
}

/// A button with no selected state, so a click never leaves it highlighted.
fn interaction_plain_button(
    id: String,
    name: String,
    label: String,
    frame: Rect,
    theme: &Theme,
) -> Object {
    Object {
        id,
        name,
        kind: ObjectKind::Button {
            label,
            fill: theme.accent.clone(),
            fill_style: None,
        },
        frame,
        timing: Timing::default(),
        entrance: None,
        exit: None,
        motion_paths: Vec::new(),
        states: Vec::new(),
        initial_state: "normal".into(),
        focus_order: None,
        decorative: false,
        description: None,
        group: None,
        panel: None,
        rotation: 0.0,
        flip_h: false,
        flip_v: false,
        opacity: 1.0,
        shadow: None,
        glow: None,
        drag: None,
        locked: false,
        editor_hidden: false,
    }
}

/// A callout panel with a title and one line of short, replaceable detail.
fn interaction_panel_objects(
    slide: &Slide,
    prefix: &str,
    label: &str,
    frame: Rect,
    theme: &Theme,
) -> Vec<Object> {
    let panel_id = fresh_id(prefix, slide.local_ids());
    let title_id = fresh_id(&format!("{prefix}_title"), slide.local_ids());
    let detail_id = fresh_id(&format!("{prefix}_detail"), slide.local_ids());
    let panel = interaction_shape(
        panel_id,
        format!("{label} panel"),
        ShapeKind::RoundedRectangle,
        tint(theme.accent.clone(), 0x22),
        Some(Outline {
            color: theme.accent.clone(),
            width: 2.0,
            dash: DashStyle::Solid,
            dash_pattern: None,
            join: Default::default(),
            miter_limit: None,
        }),
        None,
        Vec::new(),
        frame,
        theme,
    );
    let mut title = interaction_text(
        title_id,
        format!("{label} title"),
        label.to_owned(),
        Rect {
            x: frame.x + 16.0,
            y: frame.y + 10.0,
            w: (frame.w - 32.0).max(20.0),
            h: 36.0,
        },
        theme,
    );
    if let ObjectKind::Text { style, .. } = &mut title.kind {
        style.size = 26.0;
        style.bold = true;
    }
    let mut detail = interaction_text(
        detail_id,
        format!("{label} detail"),
        "Add the detail here.".to_owned(),
        Rect {
            x: frame.x + 16.0,
            y: frame.y + 52.0,
            w: (frame.w - 32.0).max(20.0),
            h: (frame.h - 64.0).max(24.0),
        },
        theme,
    );
    if let ObjectKind::Text { style, .. } = &mut detail.kind {
        style.size = 20.0;
    }
    vec![panel, title, detail]
}

/// A layer whose callout replaces the other callouts when shown.
fn interaction_callout_layer(
    id: String,
    name: String,
    objects: Vec<Object>,
    triggers: Vec<Trigger>,
) -> Layer {
    Layer {
        id,
        name,
        duration_ms: 5000,
        objects,
        triggers,
        hide_other_layers: true,
        pause_base_timeline: false,
        prevent_base_clicks: false,
        hide_base_objects: false,
        hide_when_timeline_ends: false,
        allow_seek: SeekMode::Auto,
        revisit: RevisitMode::Auto,
        cue_points: Vec::new(),
    }
}

#[allow(clippy::cognitive_complexity)]
fn interaction_insert(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "interaction.insert";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    if s.course.play_slide(&sid).is_none() {
        return Err(EngineError::NotFound(format!(
            "slide {sid:?} is not a playable slide"
        )));
    }
    let kind: String = req(C, p, "kind")?;
    let theme = s.course.theme.clone();
    let size = s.course.slide_size;
    let (w, h) = (size.w as f32, size.h as f32);
    if !w.is_finite() || !h.is_finite() || w <= 0.0 || h <= 0.0 {
        return Err(bad(C, "slide size is invalid"));
    }

    // Variables are course-wide, so create them before borrowing the slide.
    let mut flip_variables: Vec<String> = Vec::new();
    if kind == "flip_cards" {
        for _ in 0..3 {
            let name = fresh_id(
                "flip_card",
                s.course.variables.iter().map(|v| v.name.as_str()),
            );
            s.course.variables.push(VariableDef {
                name: name.clone(),
                value: Value::Bool(false),
            });
            flip_variables.push(name);
        }
    }
    let step_variable = if kind == "process_steps" {
        let name = fresh_id(
            "process_step",
            s.course.variables.iter().map(|v| v.name.as_str()),
        );
        s.course.variables.push(VariableDef {
            name: name.clone(),
            value: Value::Number(1.0),
        });
        Some(name)
    } else {
        None
    };
    // The explored counter comes first, then one flag per tile so a second click
    // on a revealed tile cannot count it twice.
    let mut reveal_variables: Vec<String> = Vec::new();
    if kind == "reveal_grid" {
        let explored = fresh_explored_variable(&s.course, "grid");
        s.course.variables.push(VariableDef {
            name: explored.clone(),
            value: Value::Number(0.0),
        });
        reveal_variables.push(explored);
        for _ in 0..6 {
            let name = fresh_id(
                "grid_tile",
                s.course.variables.iter().map(|v| v.name.as_str()),
            );
            s.course.variables.push(VariableDef {
                name: name.clone(),
                value: Value::Bool(false),
            });
            reveal_variables.push(name);
        }
    }

    let slide = slide_mut(s, &sid)?;
    let margin = (w * 0.06).clamp(20.0, 120.0);
    let top = (h * 0.12).max(20.0);
    let gap = (w * 0.012).clamp(6.0, 20.0);
    let mut base_ids: Vec<String> = Vec::new();
    let mut layer_ids: Vec<String> = Vec::new();
    let mut all_ids: Vec<String> = Vec::new();

    match kind.as_str() {
        "tabs" => {
            let count = 3usize;
            let tab_h = (h * 0.08).clamp(36.0, 64.0);
            let tab_w = ((w - 2.0 * margin - gap * (count as f32 - 1.0)) / count as f32).max(20.0);
            let content_y = top + tab_h + gap;
            let content_h = (h - content_y - top).max(40.0);
            let mut tab_ids = Vec::new();
            for i in 0..count {
                let x = margin + (tab_w + gap) * i as f32;
                let id = fresh_id("tab_button", slide.local_ids());
                tab_ids.push(id.clone());
                let button = interaction_button(
                    id.clone(),
                    format!("Tab {} button", i + 1),
                    format!("Tab {}", i + 1),
                    Rect {
                        x,
                        y: top,
                        w: tab_w,
                        h: tab_h,
                    },
                    i == 0,
                    &theme,
                );
                slide.objects.push(button);
                all_ids.push(id.clone());
                base_ids.push(id);
            }
            for i in 0..count {
                let layer_id = fresh_id("tab_layer", slide.local_ids());
                layer_ids.push(layer_id.clone());
                let text_id = fresh_id("tab_content", slide.local_ids());
                let text = interaction_text(
                    text_id.clone(),
                    format!("Tab {} content", i + 1),
                    format!("Content for Tab {}", i + 1),
                    Rect {
                        x: margin,
                        y: content_y,
                        w: (w - 2.0 * margin).max(20.0),
                        h: content_h,
                    },
                    &theme,
                );
                all_ids.push(text_id.clone());
                slide.layers.push(Layer {
                    id: layer_id.clone(),
                    name: format!("Tab {} content", i + 1),
                    duration_ms: 5000,
                    objects: vec![text],
                    triggers: Vec::new(),
                    hide_other_layers: true,
                    pause_base_timeline: false,
                    prevent_base_clicks: false,
                    hide_base_objects: false,
                    hide_when_timeline_ends: false,
                    allow_seek: SeekMode::Auto,
                    revisit: RevisitMode::Auto,
                    cue_points: Vec::new(),
                });
            }
            // Clicking a tab shows its layer and moves the selected state.
            for i in 0..count {
                let when = Event::Click {
                    object: tab_ids.get(i).cloned().unwrap_or_default(),
                };
                if let Some(layer) = layer_ids.get(i) {
                    slide.triggers.push(interaction_trigger(
                        Action::ShowLayer {
                            layer: layer.clone(),
                        },
                        when.clone(),
                    ));
                }
                for (j, id) in tab_ids.iter().enumerate() {
                    slide.triggers.push(interaction_trigger(
                        Action::ChangeState {
                            object: id.clone(),
                            state: if j == i { "selected" } else { "normal" }.to_owned(),
                        },
                        when.clone(),
                    ));
                }
            }
            if let Some(first) = layer_ids.first() {
                slide.triggers.push(interaction_trigger(
                    Action::ShowLayer {
                        layer: first.clone(),
                    },
                    Event::TimelineStarts { layer: None },
                ));
            }
        }
        "accordion" => {
            let count = 3usize;
            let header_h = (h * 0.08).clamp(36.0, 64.0);
            let header_w = (w - 2.0 * margin).max(20.0);
            let content_h = (h * 0.18).clamp(70.0, 160.0);
            let base_headers: Vec<Rect> = (0..count)
                .map(|i| Rect {
                    x: margin,
                    y: top + (header_h + gap) * i as f32,
                    w: header_w,
                    h: header_h,
                })
                .collect();
            for i in 0..count {
                layer_ids.push(fresh_id(
                    &format!("accordion_layer{}", i + 1),
                    slide.local_ids(),
                ));
            }
            let mut header_ids = Vec::new();
            for (i, frame) in base_headers.iter().copied().enumerate() {
                let id = fresh_id("accordion_header", slide.local_ids());
                header_ids.push(id.clone());
                slide.objects.push(interaction_button(
                    id.clone(),
                    format!("Header {} button", i + 1),
                    format!("Section {}", i + 1),
                    frame,
                    false,
                    &theme,
                ));
                all_ids.push(id.clone());
                base_ids.push(id);
            }
            for open in 0..count {
                let Some(layer_id) = layer_ids.get(open).cloned() else {
                    continue;
                };
                let mut layer = Layer {
                    id: layer_id.clone(),
                    name: format!("Section {} open", open + 1),
                    duration_ms: 5000,
                    objects: Vec::new(),
                    triggers: Vec::new(),
                    hide_other_layers: true,
                    pause_base_timeline: false,
                    prevent_base_clicks: true,
                    hide_base_objects: true,
                    hide_when_timeline_ends: false,
                    allow_seek: SeekMode::Auto,
                    revisit: RevisitMode::Auto,
                    cue_points: Vec::new(),
                };
                let mut layer_headers = Vec::new();
                for (i, base) in base_headers.iter().copied().enumerate() {
                    let shift = if i > open { content_h + gap } else { 0.0 };
                    let frame = Rect {
                        y: base.y + shift,
                        ..base
                    };
                    let id = fresh_id(
                        &format!("accordion_h{}_{}", open + 1, i + 1),
                        slide.local_ids(),
                    );
                    layer_headers.push((id.clone(), i, frame));
                    layer.objects.push(interaction_button(
                        id.clone(),
                        format!("Header {} button (section {} open)", i + 1, open + 1),
                        format!("Section {}", i + 1),
                        frame,
                        false,
                        &theme,
                    ));
                    all_ids.push(id);
                }
                let content_id = fresh_id("accordion_content", slide.local_ids());
                layer.objects.push(interaction_text(
                    content_id.clone(),
                    format!("Header {} content", open + 1),
                    format!("Content for Section {}", open + 1),
                    Rect {
                        x: margin,
                        y: base_headers
                            .get(open)
                            .map_or(top, |frame| frame.y + header_h + gap * 0.5),
                        w: header_w,
                        h: content_h,
                    },
                    &theme,
                ));
                all_ids.push(content_id);
                for (id, i, _) in &layer_headers {
                    let when = Event::Click { object: id.clone() };
                    if *i == open {
                        layer
                            .triggers
                            .push(interaction_trigger(Action::HideLayer { layer: None }, when));
                    } else if let Some(target) = layer_ids.get(*i) {
                        layer.triggers.push(interaction_trigger(
                            Action::ShowLayer {
                                layer: target.clone(),
                            },
                            when,
                        ));
                    }
                }
                slide.layers.push(layer);
            }
            for (i, id) in header_ids.iter().enumerate() {
                if let Some(layer) = layer_ids.get(i) {
                    slide.triggers.push(interaction_trigger(
                        Action::ShowLayer {
                            layer: layer.clone(),
                        },
                        Event::Click { object: id.clone() },
                    ));
                }
            }
        }
        "flip_cards" => {
            let count = 3usize;
            let card_h = (h * 0.36).clamp(120.0, 260.0);
            let card_w = ((w - 2.0 * margin - gap * (count as f32 - 1.0)) / count as f32).max(20.0);
            let card_y = top + ((h - top * 2.0 - card_h).max(0.0)) * 0.5;
            let mut card_ids = Vec::new();
            for i in 0..count {
                let id = fresh_id("flip_card", slide.local_ids());
                card_ids.push(id.clone());
                // The front is a plain card with an accent outline and the
                // back an accent tint, so the one text colour (the theme's)
                // reads on both sides whatever the accent is.
                let card = interaction_shape(
                    id.clone(),
                    format!("Card {}", i + 1),
                    ShapeKind::RoundedRectangle,
                    theme.background.clone(),
                    Some(Outline {
                        color: theme.accent.clone(),
                        width: 2.0,
                        dash: Default::default(),
                        dash_pattern: None,
                        join: Default::default(),
                        miter_limit: None,
                    }),
                    Some(("Card title".to_owned(), 26.0, true)),
                    vec![StateDef {
                        name: "Back".to_owned(),
                        fill: Some(tint(theme.accent.clone(), 0x55)),
                        text: Some("Add the detail here.".to_owned()),
                        ..Default::default()
                    }],
                    Rect {
                        x: margin + (card_w + gap) * i as f32,
                        y: card_y,
                        w: card_w,
                        h: card_h,
                    },
                    &theme,
                );
                slide.objects.push(card);
                all_ids.push(id.clone());
                base_ids.push(id);
            }
            for (card_id, variable) in card_ids.iter().zip(flip_variables.iter()) {
                let when = Event::Click {
                    object: card_id.clone(),
                };
                // Triggers for one event run in order, so a pair of conditions
                // on the same state would both fire. Toggle a boolean instead
                // and let variable_changes set the visible state.
                slide.triggers.push(interaction_trigger(
                    Action::SetVariable {
                        variable: variable.clone(),
                        op: VarOp::Toggle,
                        value: None,
                    },
                    when,
                ));
                for (value, state) in [(true, "Back"), (false, "normal")] {
                    slide.triggers.push(interaction_conditional_trigger(
                        Action::ChangeState {
                            object: card_id.clone(),
                            state: state.to_owned(),
                        },
                        Event::VariableChanges {
                            variable: variable.clone(),
                        },
                        Condition::Compare {
                            variable: variable.clone(),
                            op: CmpOp::Eq,
                            value: Value::Bool(value).into(),
                        },
                    ));
                }
            }
        }
        "hotspot_reveal" => {
            let picture = Rect {
                x: margin,
                y: top,
                w: (w - 2.0 * margin).max(20.0),
                h: (h - top - margin).max(40.0),
            };
            let placeholder_id = fresh_id("hotspot_picture", slide.local_ids());
            slide.objects.push(interaction_shape(
                placeholder_id.clone(),
                "Picture placeholder".to_owned(),
                ShapeKind::RoundedRectangle,
                tint(theme.accent.clone(), 0x22),
                Some(Outline {
                    color: theme.accent.clone(),
                    width: 2.0,
                    dash: DashStyle::Solid,
                    dash_pattern: None,
                    join: Default::default(),
                    miter_limit: None,
                }),
                Some(("Replace with your picture".to_owned(), 24.0, false)),
                Vec::new(),
                picture,
                &theme,
            ));
            base_ids.push(placeholder_id.clone());
            all_ids.push(placeholder_id);

            let marker_d = (w * 0.055).clamp(36.0, 60.0);
            let spots = [(0.28_f32, 0.38_f32), (0.72, 0.34), (0.50, 0.74)];
            let callout = Rect {
                x: picture.x + picture.w * 0.40,
                y: picture.y + picture.h * 0.16,
                w: (picture.w * 0.48).max(120.0),
                h: (picture.h * 0.48).max(110.0),
            };
            for (i, (px, py)) in spots.iter().copied().enumerate() {
                let marker_id = fresh_id("hotspot_marker", slide.local_ids());
                let marker = interaction_shape(
                    marker_id.clone(),
                    format!("Hotspot {} marker", i + 1),
                    ShapeKind::Ellipse,
                    theme.accent.clone(),
                    None,
                    Some(((i + 1).to_string(), 22.0, true)),
                    vec![StateDef {
                        name: "Visited".to_owned(),
                        fill: Some(tint(theme.accent.clone(), 0x99)),
                        ..Default::default()
                    }],
                    Rect {
                        x: picture.x + picture.w * px - marker_d * 0.5,
                        y: picture.y + picture.h * py - marker_d * 0.5,
                        w: marker_d,
                        h: marker_d,
                    },
                    &theme,
                );
                slide.objects.push(marker);
                base_ids.push(marker_id.clone());
                all_ids.push(marker_id.clone());

                let layer_id = fresh_id("hotspot_layer", slide.local_ids());
                layer_ids.push(layer_id.clone());
                let mut objects = interaction_panel_objects(
                    slide,
                    &format!("hotspot_callout{}", i + 1),
                    &format!("Hotspot {}", i + 1),
                    callout,
                    &theme,
                );
                let close_id = fresh_id("hotspot_close", slide.local_ids());
                objects.push(interaction_plain_button(
                    close_id.clone(),
                    format!("Close hotspot {}", i + 1),
                    "×".to_owned(),
                    Rect {
                        x: callout.x + callout.w - 44.0,
                        y: callout.y + 8.0,
                        w: 36.0,
                        h: 36.0,
                    },
                    &theme,
                ));
                all_ids.extend(objects.iter().map(|o| o.id.clone()));
                let layer_triggers = vec![interaction_trigger(
                    Action::HideLayer { layer: None },
                    Event::Click { object: close_id },
                )];
                slide.layers.push(interaction_callout_layer(
                    layer_id.clone(),
                    format!("Hotspot {}", i + 1),
                    objects,
                    layer_triggers,
                ));

                let when = Event::Click {
                    object: marker_id.clone(),
                };
                slide.triggers.push(interaction_trigger(
                    Action::ShowLayer { layer: layer_id },
                    when.clone(),
                ));
                slide.triggers.push(interaction_trigger(
                    Action::ChangeState {
                        object: marker_id,
                        state: "Visited".to_owned(),
                    },
                    when,
                ));
            }
        }
        "process_steps" => {
            let Some(variable) = step_variable.clone() else {
                return Err(bad(C, "process_steps needs a step variable"));
            };
            let count = 4usize;
            let circle_d = (w * 0.07).clamp(40.0, 76.0);
            let row_y = top;
            let step_w = (w - 2.0 * margin) / count as f32;
            let centres: Vec<f32> = (0..count)
                .map(|i| margin + step_w * (i as f32 + 0.5))
                .collect();
            for i in 0..count.saturating_sub(1) {
                let id = fresh_id("step_line", slide.local_ids());
                slide.objects.push(interaction_shape(
                    id.clone(),
                    format!("Step {} connector", i + 1),
                    ShapeKind::LineHorizontal,
                    theme.accent.clone(),
                    None,
                    None,
                    Vec::new(),
                    Rect {
                        x: centres[i] + circle_d * 0.5,
                        y: row_y + circle_d * 0.5,
                        w: (centres[i + 1] - centres[i] - circle_d).max(1.0),
                        h: 2.0,
                    },
                    &theme,
                ));
                all_ids.push(id.clone());
                base_ids.push(id);
            }
            let mut circle_ids = Vec::new();
            for (i, &centre) in centres.iter().enumerate() {
                let circle_id = fresh_id("step_circle", slide.local_ids());
                circle_ids.push(circle_id.clone());
                slide.objects.push(interaction_shape(
                    circle_id.clone(),
                    format!("Step {} circle", i + 1),
                    ShapeKind::Ellipse,
                    tint(theme.accent.clone(), 0x99),
                    None,
                    Some(((i + 1).to_string(), 24.0, true)),
                    vec![StateDef {
                        name: "Selected".to_owned(),
                        fill: Some(theme.accent.clone()),
                        ..Default::default()
                    }],
                    Rect {
                        x: centre - circle_d * 0.5,
                        y: row_y,
                        w: circle_d,
                        h: circle_d,
                    },
                    &theme,
                ));
                all_ids.push(circle_id.clone());
                base_ids.push(circle_id.clone());

                let label_id = fresh_id("step_label", slide.local_ids());
                let mut label = interaction_text(
                    label_id.clone(),
                    format!("Step {} label", i + 1),
                    format!("Step {}", i + 1),
                    Rect {
                        x: centre - step_w * 0.5,
                        y: row_y + circle_d + 6.0,
                        w: step_w,
                        h: 30.0,
                    },
                    &theme,
                );
                // The box is centred on its circle, so the text is too.
                if let ObjectKind::Text { style, .. } = &mut label.kind {
                    style.align = Align::Center;
                }
                slide.objects.push(label);
                all_ids.push(label_id.clone());
                base_ids.push(label_id);
            }

            let button_y = (h - margin - 44.0).max(row_y + circle_d + 60.0);
            let button_w = 110.0;
            let back_id = fresh_id("step_back", slide.local_ids());
            let next_id = fresh_id("step_next", slide.local_ids());
            slide.objects.push(interaction_plain_button(
                back_id.clone(),
                "Back".to_owned(),
                "Back".to_owned(),
                Rect {
                    x: margin,
                    y: button_y,
                    w: button_w,
                    h: 44.0,
                },
                &theme,
            ));
            slide.objects.push(interaction_plain_button(
                next_id.clone(),
                "Next".to_owned(),
                "Next".to_owned(),
                Rect {
                    x: (w - margin - button_w).max(margin + button_w + gap),
                    y: button_y,
                    w: button_w,
                    h: 44.0,
                },
                &theme,
            ));
            all_ids.push(back_id.clone());
            all_ids.push(next_id.clone());
            base_ids.push(back_id.clone());
            base_ids.push(next_id.clone());

            let panel = Rect {
                x: margin,
                y: row_y + circle_d + 48.0,
                w: (w - 2.0 * margin).max(20.0),
                h: (button_y - row_y - circle_d - 56.0).max(80.0),
            };
            for i in 0..count {
                let layer_id = fresh_id("step_layer", slide.local_ids());
                layer_ids.push(layer_id.clone());
                let objects = interaction_panel_objects(
                    slide,
                    &format!("step_panel{}", i + 1),
                    &format!("Step {}", i + 1),
                    panel,
                    &theme,
                );
                all_ids.extend(objects.iter().map(|o| o.id.clone()));
                slide.layers.push(interaction_callout_layer(
                    layer_id,
                    format!("Step {}", i + 1),
                    objects,
                    Vec::new(),
                ));
            }

            for (i, circle_id) in circle_ids.iter().enumerate() {
                let step = (i + 1) as f64;
                slide.triggers.push(interaction_trigger(
                    Action::SetVariable {
                        variable: variable.clone(),
                        op: VarOp::Assign,
                        value: Some(Value::Number(step).into()),
                    },
                    Event::Click {
                        object: circle_id.clone(),
                    },
                ));
            }
            slide.triggers.push(interaction_conditional_trigger(
                Action::SetVariable {
                    variable: variable.clone(),
                    op: VarOp::Add,
                    value: Some(Value::Number(1.0).into()),
                },
                Event::Click { object: next_id },
                Condition::Compare {
                    variable: variable.clone(),
                    op: CmpOp::Lt,
                    value: Value::Number(count as f64).into(),
                },
            ));
            slide.triggers.push(interaction_conditional_trigger(
                Action::SetVariable {
                    variable: variable.clone(),
                    op: VarOp::Subtract,
                    value: Some(Value::Number(1.0).into()),
                },
                Event::Click { object: back_id },
                Condition::Compare {
                    variable: variable.clone(),
                    op: CmpOp::Gt,
                    value: Value::Number(1.0).into(),
                },
            ));
            for i in 0..count {
                let step = (i + 1) as f64;
                let condition = Condition::Compare {
                    variable: variable.clone(),
                    op: CmpOp::Eq,
                    value: Value::Number(step).into(),
                };
                if let Some(layer_id) = layer_ids.get(i) {
                    slide.triggers.push(interaction_conditional_trigger(
                        Action::ShowLayer {
                            layer: layer_id.clone(),
                        },
                        Event::VariableChanges {
                            variable: variable.clone(),
                        },
                        condition.clone(),
                    ));
                }
                for (j, circle_id) in circle_ids.iter().enumerate() {
                    slide.triggers.push(interaction_conditional_trigger(
                        Action::ChangeState {
                            object: circle_id.clone(),
                            state: if j == i { "Selected" } else { "normal" }.to_owned(),
                        },
                        Event::VariableChanges {
                            variable: variable.clone(),
                        },
                        condition.clone(),
                    ));
                }
            }
            if let Some(first_layer) = layer_ids.first()
                && let Some(first_circle) = circle_ids.first()
            {
                // Revisiting the slide starts again at step 1, so the variable,
                // the shown layer and the selected circle always agree.
                slide.triggers.push(interaction_trigger(
                    Action::SetVariable {
                        variable: variable.clone(),
                        op: VarOp::Assign,
                        value: Some(Value::Number(1.0).into()),
                    },
                    Event::TimelineStarts { layer: None },
                ));
                slide.triggers.push(interaction_trigger(
                    Action::ShowLayer {
                        layer: first_layer.clone(),
                    },
                    Event::TimelineStarts { layer: None },
                ));
                slide.triggers.push(interaction_trigger(
                    Action::ChangeState {
                        object: first_circle.clone(),
                        state: "Selected".to_owned(),
                    },
                    Event::TimelineStarts { layer: None },
                ));
            }
        }
        "timeline" => {
            let count = 5usize;
            let point_d = (w * 0.05).clamp(28.0, 52.0);
            let line_y = top + 40.0;
            let step = (w - 2.0 * margin) / (count as f32 - 1.0);
            let centres: Vec<f32> = (0..count).map(|i| margin + step * i as f32).collect();

            let line_id = fresh_id("timeline_line", slide.local_ids());
            slide.objects.push(interaction_shape(
                line_id.clone(),
                "Timeline line".to_owned(),
                ShapeKind::LineHorizontal,
                theme.accent.clone(),
                None,
                None,
                Vec::new(),
                Rect {
                    x: margin,
                    y: line_y,
                    w: (w - 2.0 * margin).max(1.0),
                    h: 2.0,
                },
                &theme,
            ));
            all_ids.push(line_id.clone());
            base_ids.push(line_id);

            let mut point_ids = Vec::new();
            for (i, &centre) in centres.iter().enumerate() {
                let point_id = fresh_id("timeline_point", slide.local_ids());
                point_ids.push(point_id.clone());
                slide.objects.push(interaction_shape(
                    point_id.clone(),
                    format!("Point {}", i + 1),
                    ShapeKind::Ellipse,
                    tint(theme.accent.clone(), 0x99),
                    None,
                    None,
                    vec![StateDef {
                        name: "Visited".to_owned(),
                        fill: Some(theme.accent.clone()),
                        ..Default::default()
                    }],
                    Rect {
                        x: centre - point_d * 0.5,
                        y: line_y - point_d * 0.5,
                        w: point_d,
                        h: point_d,
                    },
                    &theme,
                ));
                all_ids.push(point_id.clone());
                base_ids.push(point_id.clone());

                let year_id = fresh_id("timeline_year", slide.local_ids());
                let mut year = interaction_text(
                    year_id.clone(),
                    format!("Point {} year", i + 1),
                    format!("{}", 2020 + i as u32),
                    Rect {
                        x: centre - step * 0.5,
                        y: top - 8.0,
                        w: step,
                        h: 30.0,
                    },
                    &theme,
                );
                if let ObjectKind::Text { style, .. } = &mut year.kind {
                    style.size = 18.0;
                    style.align = Align::Center;
                }
                slide.objects.push(year);
                all_ids.push(year_id.clone());
                base_ids.push(year_id);

                let title_id = fresh_id("timeline_title", slide.local_ids());
                let mut title = interaction_text(
                    title_id.clone(),
                    format!("Point {} title", i + 1),
                    format!("Title {}", i + 1),
                    Rect {
                        x: centre - step * 0.5,
                        y: line_y + point_d * 0.5 + 4.0,
                        w: step,
                        h: 30.0,
                    },
                    &theme,
                );
                if let ObjectKind::Text { style, .. } = &mut title.kind {
                    style.size = 18.0;
                    style.align = Align::Center;
                }
                slide.objects.push(title);
                all_ids.push(title_id.clone());
                base_ids.push(title_id);
            }

            let panel_y = line_y + point_d * 0.5 + 40.0;
            let panel = Rect {
                x: margin,
                y: panel_y,
                w: (w - 2.0 * margin).max(20.0),
                h: (h - panel_y - margin).max(80.0),
            };
            for (i, point_id) in point_ids.iter().enumerate() {
                let layer_id = fresh_id("timeline_layer", slide.local_ids());
                layer_ids.push(layer_id.clone());
                let objects = interaction_panel_objects(
                    slide,
                    &format!("timeline_panel{}", i + 1),
                    &format!("Point {}", i + 1),
                    panel,
                    &theme,
                );
                all_ids.extend(objects.iter().map(|o| o.id.clone()));
                slide.layers.push(interaction_callout_layer(
                    layer_id.clone(),
                    format!("Point {}", i + 1),
                    objects,
                    Vec::new(),
                ));

                let when = Event::Click {
                    object: point_id.clone(),
                };
                slide.triggers.push(interaction_trigger(
                    Action::ShowLayer { layer: layer_id },
                    when.clone(),
                ));
                slide.triggers.push(interaction_trigger(
                    Action::ChangeState {
                        object: point_id.clone(),
                        state: "Visited".to_owned(),
                    },
                    when,
                ));
            }
            // No layer is shown on slide start; each visit is remembered.
        }
        "reveal_grid" => {
            let Some(explored) = reveal_variables.first().cloned() else {
                return Err(bad(C, "reveal_grid needs an explored variable"));
            };
            let tile_vars: Vec<String> = reveal_variables.iter().skip(1).cloned().collect();
            let cols = 3usize;
            let rows = 2usize;
            let count = cols * rows;
            let counter_h = 32.0;
            let grid_w = (w - 2.0 * margin).max(20.0);
            let grid_h = (h - top - margin - counter_h - gap).max(120.0);
            let tile_w = (grid_w - gap * (cols as f32 - 1.0)) / cols as f32;
            let tile_h = (grid_h - gap * (rows as f32 - 1.0)) / rows as f32;

            let mut tile_ids = Vec::new();
            for i in 0..count {
                let id = fresh_id("grid_tile_shape", slide.local_ids());
                tile_ids.push(id.clone());
                slide.objects.push(interaction_shape(
                    id.clone(),
                    format!("Tile {}", i + 1),
                    ShapeKind::RoundedRectangle,
                    tint(theme.accent.clone(), 0x22),
                    Some(Outline {
                        color: theme.accent.clone(),
                        width: 2.0,
                        dash: DashStyle::Solid,
                        dash_pattern: None,
                        join: Default::default(),
                        miter_limit: None,
                    }),
                    Some(("Tap to reveal".to_owned(), 18.0, false)),
                    vec![StateDef {
                        name: "Revealed".to_owned(),
                        fill: Some(tint(theme.accent.clone(), 0x77)),
                        text: Some(format!("Detail {}", i + 1)),
                        ..Default::default()
                    }],
                    Rect {
                        x: margin + (tile_w + gap) * (i % cols) as f32,
                        y: top + (tile_h + gap) * (i / cols) as f32,
                        w: tile_w,
                        h: tile_h,
                    },
                    &theme,
                ));
                all_ids.push(id.clone());
                base_ids.push(id);
            }

            let counter_id = fresh_id("grid_counter", slide.local_ids());
            let mut counter = interaction_text(
                counter_id.clone(),
                "Explored counter".to_owned(),
                format!("%{explored}% of {count} explored"),
                Rect {
                    x: margin,
                    y: top + grid_h + gap,
                    w: grid_w,
                    h: counter_h,
                },
                &theme,
            );
            if let ObjectKind::Text { style, .. } = &mut counter.kind {
                style.align = Align::Center;
                style.size = 20.0;
            }
            slide.objects.push(counter);
            all_ids.push(counter_id.clone());
            base_ids.push(counter_id);

            for (tile_id, tile_var) in tile_ids.iter().zip(tile_vars.iter()) {
                // A click only flips the flag when the tile is still normal. The
                // reveal and the count then follow the flag's change, so a second
                // click on a revealed tile changes nothing.
                slide.triggers.push(interaction_conditional_trigger(
                    Action::SetVariable {
                        variable: tile_var.clone(),
                        op: VarOp::Assign,
                        value: Some(Value::Bool(true).into()),
                    },
                    Event::Click {
                        object: tile_id.clone(),
                    },
                    Condition::Compare {
                        variable: tile_var.clone(),
                        op: CmpOp::Eq,
                        value: Value::Bool(false).into(),
                    },
                ));
                let changed = Event::VariableChanges {
                    variable: tile_var.clone(),
                };
                let revealed = Condition::Compare {
                    variable: tile_var.clone(),
                    op: CmpOp::Eq,
                    value: Value::Bool(true).into(),
                };
                slide.triggers.push(interaction_conditional_trigger(
                    Action::ChangeState {
                        object: tile_id.clone(),
                        state: "Revealed".to_owned(),
                    },
                    changed.clone(),
                    revealed.clone(),
                ));
                slide.triggers.push(interaction_conditional_trigger(
                    Action::SetVariable {
                        variable: explored.clone(),
                        op: VarOp::Add,
                        value: Some(Value::Number(1.0).into()),
                    },
                    changed,
                    revealed,
                ));
                // Revisiting the slide resets both the flag and the visible state.
                slide.triggers.push(interaction_trigger(
                    Action::SetVariable {
                        variable: tile_var.clone(),
                        op: VarOp::Assign,
                        value: Some(Value::Bool(false).into()),
                    },
                    Event::TimelineStarts { layer: None },
                ));
                slide.triggers.push(interaction_trigger(
                    Action::ChangeState {
                        object: tile_id.clone(),
                        state: "normal".to_owned(),
                    },
                    Event::TimelineStarts { layer: None },
                ));
            }
            slide.triggers.push(interaction_trigger(
                Action::SetVariable {
                    variable: explored,
                    op: VarOp::Assign,
                    value: Some(Value::Number(0.0).into()),
                },
                Event::TimelineStarts { layer: None },
            ));
        }
        "scenario_choice" => {
            let situation = Rect {
                x: margin,
                y: top,
                w: (w - 2.0 * margin).max(20.0),
                h: (h * 0.22).clamp(90.0, 180.0),
            };
            for (id, name, fill, text, size, bold) in [
                (
                    fresh_id("scenario_panel", slide.local_ids()),
                    "Situation panel".to_owned(),
                    tint(theme.accent.clone(), 0x22),
                    None,
                    0.0,
                    false,
                ),
                (
                    fresh_id("scenario_title", slide.local_ids()),
                    "Situation title".to_owned(),
                    theme.text.clone(),
                    Some("What would you do?".to_owned()),
                    26.0,
                    true,
                ),
                (
                    fresh_id("scenario_body", slide.local_ids()),
                    "Situation sentence".to_owned(),
                    theme.text.clone(),
                    Some("Describe the situation here.".to_owned()),
                    20.0,
                    false,
                ),
            ] {
                all_ids.push(id.clone());
                base_ids.push(id.clone());
                if text.is_none() {
                    slide.objects.push(interaction_shape(
                        id,
                        name,
                        ShapeKind::RoundedRectangle,
                        fill,
                        Some(Outline {
                            color: theme.accent.clone(),
                            width: 2.0,
                            dash: DashStyle::Solid,
                            dash_pattern: None,
                            join: Default::default(),
                            miter_limit: None,
                        }),
                        None,
                        Vec::new(),
                        situation,
                        &theme,
                    ));
                } else {
                    slide.objects.push(interaction_text(
                        id,
                        name,
                        text.unwrap_or_default(),
                        Rect {
                            x: situation.x + 16.0,
                            y: situation.y + if bold { 10.0 } else { 52.0 },
                            w: (situation.w - 32.0).max(20.0),
                            h: 36.0,
                        },
                        &theme,
                    ));
                    if let Some(object) = slide.objects.last_mut()
                        && let ObjectKind::Text { style, .. } = &mut object.kind
                    {
                        style.size = size;
                        style.bold = bold;
                    }
                }
            }

            let choice_h = (h * 0.08).clamp(36.0, 56.0);
            let choice_y = situation.y + situation.h + gap;
            let mut choice_ids = Vec::new();
            for i in 0..3 {
                let id = fresh_id("scenario_choice", slide.local_ids());
                choice_ids.push(id.clone());
                slide.objects.push(interaction_plain_button(
                    id.clone(),
                    format!("Choice {}", i + 1),
                    format!("Choice {}", i + 1),
                    Rect {
                        x: situation.x,
                        y: choice_y + (choice_h + gap) * i as f32,
                        w: situation.w,
                        h: choice_h,
                    },
                    &theme,
                ));
                all_ids.push(id.clone());
                base_ids.push(id);
            }

            let panel = Rect {
                x: situation.x + situation.w * 0.10,
                y: choice_y,
                w: (situation.w * 0.80).max(20.0),
                h: (h - choice_y - margin).max(160.0),
            };
            let feedback = [
                (
                    "Think again",
                    "That is a reasonable start, but there is a better option. Consider the suggested answer and try again.",
                ),
                (
                    "Good choice",
                    "This is the suggested best answer because it balances the learner's needs with the goal of the situation.",
                ),
                (
                    "Think again",
                    "Not quite. Think about what the situation is asking, then try a different approach.",
                ),
            ];
            for (i, (heading, body)) in feedback.into_iter().enumerate() {
                let layer_id = fresh_id("scenario_layer", slide.local_ids());
                layer_ids.push(layer_id.clone());
                let heading_id = fresh_id("scenario_feedback_title", slide.local_ids());
                let body_id = fresh_id("scenario_feedback_text", slide.local_ids());
                let mut heading_object = interaction_text(
                    heading_id.clone(),
                    format!("Choice {} heading", i + 1),
                    heading.to_owned(),
                    Rect {
                        x: panel.x + 16.0,
                        y: panel.y + 12.0,
                        w: (panel.w - 32.0).max(20.0),
                        h: 36.0,
                    },
                    &theme,
                );
                if let ObjectKind::Text { style, .. } = &mut heading_object.kind {
                    style.size = 26.0;
                    style.bold = true;
                }
                let body_object = interaction_text(
                    body_id.clone(),
                    format!("Choice {} text", i + 1),
                    body.to_owned(),
                    Rect {
                        x: panel.x + 16.0,
                        y: panel.y + 56.0,
                        w: (panel.w - 32.0).max(20.0),
                        h: (panel.h - 120.0).max(24.0),
                    },
                    &theme,
                );
                let mut objects = vec![
                    interaction_shape(
                        fresh_id("scenario_feedback_panel", slide.local_ids()),
                        format!("Choice {} panel", i + 1),
                        ShapeKind::RoundedRectangle,
                        tint(theme.accent.clone(), 0x22),
                        Some(Outline {
                            color: theme.accent.clone(),
                            width: 2.0,
                            dash: DashStyle::Solid,
                            dash_pattern: None,
                            join: Default::default(),
                            miter_limit: None,
                        }),
                        None,
                        Vec::new(),
                        panel,
                        &theme,
                    ),
                    heading_object,
                    body_object,
                ];
                let layer_triggers = if i == 1 {
                    Vec::new()
                } else {
                    let retry_id = fresh_id("scenario_retry", slide.local_ids());
                    objects.push(interaction_plain_button(
                        retry_id.clone(),
                        format!("Try again {}", i + 1),
                        "Try again".to_owned(),
                        Rect {
                            x: panel.x + 16.0,
                            y: panel.y + panel.h - 52.0,
                            w: 120.0,
                            h: 40.0,
                        },
                        &theme,
                    ));
                    vec![interaction_trigger(
                        Action::HideLayer { layer: None },
                        Event::Click { object: retry_id },
                    )]
                };
                all_ids.extend(objects.iter().map(|o| o.id.clone()));
                slide.layers.push(interaction_callout_layer(
                    layer_id.clone(),
                    format!("Choice {}", i + 1),
                    objects,
                    layer_triggers,
                ));
                slide.triggers.push(interaction_trigger(
                    Action::ShowLayer { layer: layer_id },
                    Event::Click {
                        object: choice_ids[i].clone(),
                    },
                ));
            }
            // No question and no score: the best answer is marked by its text only.
        }
        _ => {
            return Err(bad(
                C,
                "kind must be \"tabs\", \"accordion\", \"flip_cards\", \"hotspot_reveal\", \"process_steps\", \"timeline\", \"reveal_grid\" or \"scenario_choice\"",
            ));
        }
    }

    keep_valid(s, was_valid)?;
    let mut variables = flip_variables;
    variables.extend(step_variable);
    variables.extend(reveal_variables);
    Ok(json!({
        "objects": base_ids,
        "layers": layer_ids,
        "all_objects": all_ids,
        "variables": variables,
    }))
}

/// The condition a new freeform question starts with: the first user variable
/// (if any) compared with zero, else the built-in score percent.
fn freeform_default_condition(course: &Course) -> Condition {
    match course.variables.first() {
        Some(v) => match &v.value {
            Value::Bool(_) => Condition::Compare {
                variable: v.name.clone(),
                op: CmpOp::Eq,
                value: Value::Bool(true).into(),
            },
            Value::Number(_) => Condition::Compare {
                variable: v.name.clone(),
                op: CmpOp::Ge,
                value: Value::Number(0.0).into(),
            },
            Value::Text(_) => Condition::Compare {
                variable: v.name.clone(),
                op: CmpOp::Eq,
                value: Value::Text(String::new()).into(),
            },
        },
        None => Condition::Compare {
            variable: "Course.ScorePercent".to_owned(),
            op: CmpOp::Ge,
            value: Value::Number(0.0).into(),
        },
    }
}

fn question_insert(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "question.insert";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let kind: QuestionKind = opt(C, p, "kind")?.unwrap_or(QuestionKind::MultipleChoice);
    let freeform_default = freeform_default_condition(&s.course);
    let theme = s.course.theme.clone();
    let slide_size = s.course.slide_size;

    let slide = slide_mut(s, &sid)?;
    if slide.question.is_some() {
        return Err(EngineError::Invalid(format!(
            "slide {sid:?} already has a question"
        )));
    }
    ensure_question_support(slide, kind, &theme);
    let mut question = default_multiple_choice_question();
    apply_question_kind(&slide.objects, &mut question, kind)?;
    if kind == QuestionKind::Freeform {
        question.correct_when = Some(freeform_default);
    }
    slide.question = Some(question);
    add_feedback_layers(slide, &theme, slide_size);
    let button = add_submit_button(slide, &theme, slide_size);
    keep_valid(s, was_valid)?;
    Ok(json!({ "button": button }))
}

fn question_add_submit_button(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "question.add_submit_button";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let theme = s.course.theme.clone();
    let slide_size = s.course.slide_size;
    let slide = slide_mut(s, &sid)?;
    if slide.question.is_none() {
        return Err(EngineError::NotFound(format!(
            "slide {sid:?} has no question"
        )));
    }
    let button = add_submit_button(slide, &theme, slide_size);
    keep_valid(s, was_valid)?;
    Ok(json!({ "button": button }))
}

fn question_set(s: &mut Session, p: &Json) -> Result<Json> {
    let was_valid = s.is_playable();
    let sid: String = req("question.set", p, "slide")?;
    let question: Question = req("question.set", p, "question")?;
    let slide = slide_mut(s, &sid)?;
    slide.question = Some(question);
    let graded = slide
        .triggers
        .iter()
        .chain(slide.layers.iter().flat_map(|l| l.triggers.iter()))
        .any(|t| t.action == Action::SubmitQuestion);
    if !graded {
        slide.triggers.push(Trigger {
            action: Action::SubmitQuestion,
            when: Event::Submit,
            condition: None,
            disabled: false,
        });
    }
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

fn question_of<'a>(s: &'a mut Session, sid: &str) -> Result<&'a mut Question> {
    let slide = slide_mut(s, sid)?;
    slide
        .question
        .as_mut()
        .ok_or_else(|| EngineError::NotFound(format!("slide {sid:?} has no question")))
}

fn require_kind(q: &Question, kind: QuestionKind, cmd: &str) -> Result<()> {
    if q.kind == kind {
        Ok(())
    } else {
        Err(bad(cmd, format!("{cmd} does not apply to this question")))
    }
}

fn index_of(cmd: &str, p: &Json, len: usize) -> Result<usize> {
    let raw: u64 = req(cmd, p, "index")?;
    let idx = usize::try_from(raw).map_err(|_| bad(cmd, "`index` is out of range"))?;
    if idx >= len {
        return Err(bad(cmd, "`index` is out of range"));
    }
    Ok(idx)
}

fn dest_index(cmd: &str, p: &Json, len: usize) -> Result<usize> {
    let raw: u64 = req(cmd, p, "to")?;
    let idx = usize::try_from(raw).map_err(|_| bad(cmd, "`to` is out of range"))?;
    if idx >= len {
        return Err(bad(cmd, "`to` is out of range"));
    }
    Ok(idx)
}

fn fresh_choice_id(choices: &[Choice]) -> String {
    for ch in 'a'..='z' {
        let id = ch.to_string();
        if !choices.iter().any(|c| c.id == id) {
            return id;
        }
    }
    fresh_id("choice_", choices.iter().map(|c| c.id.as_str()))
}

fn push_choice(choices: &mut Vec<Choice>, label: &str, correct: bool) {
    let n = choices.len().saturating_add(1);
    let id = fresh_choice_id(choices);
    choices.push(Choice {
        id,
        text: format!("{label} {n}"),
        correct,
        points: 0,
    });
}

fn base_object<'a>(slide: &'a Slide, id: &str) -> Option<&'a Object> {
    match slide.object(id) {
        Some((object, None)) => Some(object),
        _ => None,
    }
}

fn clear_kind_payload(q: &mut Question) {
    q.accepted.clear();
    q.pairs.clear();
    q.case_sensitive = false;
    q.hotspot_image = None;
    q.regions.clear();
    q.markers = 1;
    q.drag_items.clear();
    q.drop_targets.clear();
    q.snap_to_center = true;
    q.return_if_wrong = false;
    q.one_per_target = true;
    q.correct_when = None;
}

fn keep_one_correct(choices: &mut [Choice]) {
    let mut seen = false;
    for c in choices.iter_mut() {
        if c.correct && !seen {
            seen = true;
        } else {
            c.correct = false;
        }
    }
    if !seen && let Some(first) = choices.first_mut() {
        first.correct = true;
    }
}

fn keep_some_correct(choices: &mut [Choice]) {
    if choices.iter().any(|c| c.correct) {
        return;
    }
    if let Some(first) = choices.first_mut() {
        first.correct = true;
    }
}

fn apply_question_kind(
    base_objects: &[Object],
    q: &mut Question,
    kind: QuestionKind,
) -> Result<()> {
    if q.kind == kind {
        return Ok(());
    }
    match kind {
        QuestionKind::TrueFalse => {
            q.choices = vec![
                Choice {
                    id: "true".into(),
                    text: "True".into(),
                    correct: true,
                    points: 0,
                },
                Choice {
                    id: "false".into(),
                    text: "False".into(),
                    correct: false,
                    points: 0,
                },
            ];
            clear_kind_payload(q);
        }
        QuestionKind::FillIn => {
            q.choices.clear();
            clear_kind_payload(q);
            q.accepted = vec!["answer".into()];
            q.shuffle = false;
        }
        QuestionKind::Sequence => {
            for c in &mut q.choices {
                c.correct = false;
            }
            clear_kind_payload(q);
            q.shuffle = false;
            while q.choices.len() < 2 {
                push_choice(&mut q.choices, "Item", false);
            }
            if q.choices.len() > 20 {
                q.choices.truncate(20);
            }
        }
        QuestionKind::Matching => {
            q.choices.clear();
            clear_kind_payload(q);
            q.shuffle = false;
            q.pairs = vec![
                MatchPair {
                    id: "p1".into(),
                    left: "Left 1".into(),
                    right: "Right 1".into(),
                },
                MatchPair {
                    id: "p2".into(),
                    left: "Left 2".into(),
                    right: "Right 2".into(),
                },
            ];
        }
        QuestionKind::MultipleChoice => {
            clear_kind_payload(q);
            while q.choices.len() < 2 {
                let correct = !q.choices.iter().any(|c| c.correct);
                push_choice(&mut q.choices, "Choice", correct);
            }
            keep_one_correct(&mut q.choices);
        }
        QuestionKind::MultipleResponse => {
            clear_kind_payload(q);
            while q.choices.len() < 2 {
                let correct = !q.choices.iter().any(|c| c.correct);
                push_choice(&mut q.choices, "Choice", correct);
            }
            keep_some_correct(&mut q.choices);
        }
        QuestionKind::Hotspot => {
            let img = base_objects
                .iter()
                .find(|o| matches!(o.kind, ObjectKind::Image { .. }));
            let Some(img) = img else {
                return Err(EngineError::Invalid(
                    "hotspot requires at least one base-layer image on the slide".into(),
                ));
            };
            q.choices.clear();
            clear_kind_payload(q);
            let cx = img.frame.x + img.frame.w * 0.5;
            let cy = img.frame.y + img.frame.h * 0.5;
            let rx = (cx - 60.0).max(0.0);
            let ry = (cy - 40.0).max(0.0);
            q.hotspot_image = Some(img.id.clone());
            q.regions = vec![HotspotRegion {
                id: "r1".into(),
                shape: HotspotShape::Rect,
                x: rx,
                y: ry,
                w: 120.0,
                h: 80.0,
                correct: true,
            }];
            q.markers = 1;
            q.shuffle = false;
        }
        QuestionKind::DragDrop => {
            if base_objects.len() < 2 {
                return Err(EngineError::Invalid(
                    "drag_drop requires at least two base-layer objects on the slide".into(),
                ));
            }
            let by_name = |name: &str| {
                base_objects
                    .iter()
                    .find(|o| o.name == name)
                    .map(|o| o.id.clone())
            };
            let item_id = by_name("Drag me")
                .or_else(|| base_objects.first().map(|o| o.id.clone()))
                .ok_or_else(|| EngineError::Invalid("drag_drop requires a drag item".into()))?;
            let target_id = by_name("Drop target")
                .or_else(|| {
                    base_objects
                        .iter()
                        .map(|o| o.id.clone())
                        .find(|id| id != &item_id)
                })
                .ok_or_else(|| EngineError::Invalid("drag_drop requires a drop target".into()))?;
            q.choices.clear();
            clear_kind_payload(q);
            q.drag_items = vec![DragItem {
                item: item_id,
                target: target_id.clone(),
            }];
            q.drop_targets = vec![target_id];
            q.snap_to_center = true;
            q.return_if_wrong = false;
            q.one_per_target = true;
            q.shuffle = false;
        }
        QuestionKind::Likert => {
            q.choices = vec![
                Choice {
                    id: "1".into(),
                    text: "Strongly disagree".into(),
                    correct: false,
                    points: 0,
                },
                Choice {
                    id: "2".into(),
                    text: "Disagree".into(),
                    correct: false,
                    points: 0,
                },
                Choice {
                    id: "3".into(),
                    text: "Neutral".into(),
                    correct: false,
                    points: 0,
                },
                Choice {
                    id: "4".into(),
                    text: "Agree".into(),
                    correct: false,
                    points: 0,
                },
                Choice {
                    id: "5".into(),
                    text: "Strongly agree".into(),
                    correct: false,
                    points: 0,
                },
            ];
            clear_kind_payload(q);
            q.shuffle = false;
        }
        QuestionKind::ShortAnswer | QuestionKind::Essay => {
            q.choices.clear();
            clear_kind_payload(q);
            q.shuffle = false;
        }
        QuestionKind::Freeform => {
            q.choices.clear();
            clear_kind_payload(q);
            q.shuffle = false;
            q.correct_when = Some(Condition::Compare {
                variable: "Course.ScorePercent".into(),
                op: CmpOp::Ge,
                value: Value::Number(0.0).into(),
            });
        }
    }
    q.kind = kind;
    // Reset scoring if it is no longer allowed for the new kind.
    match q.scoring {
        Scoring::PerAnswer
            if !matches!(
                kind,
                QuestionKind::MultipleChoice
                    | QuestionKind::MultipleResponse
                    | QuestionKind::TrueFalse
            ) =>
        {
            q.scoring = Scoring::AllOrNothing;
        }
        Scoring::Partial
            if !matches!(
                kind,
                QuestionKind::MultipleResponse
                    | QuestionKind::Sequence
                    | QuestionKind::Matching
                    | QuestionKind::Hotspot
                    | QuestionKind::DragDrop
            ) =>
        {
            q.scoring = Scoring::AllOrNothing;
        }
        _ => {}
    }
    Ok(())
}

fn move_to<T>(items: &mut Vec<T>, from: usize, to: usize) {
    if from >= items.len() || to >= items.len() {
        return;
    }
    let item = items.remove(from);
    let dest = to.min(items.len());
    items.insert(dest, item);
}

fn question_set_kind(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "question.set_kind";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let kind: QuestionKind = req(C, p, "kind")?;
    let freeform_default = freeform_default_condition(&s.course);
    let slide = slide_mut(s, &sid)?;
    let q = slide
        .question
        .as_mut()
        .ok_or_else(|| EngineError::NotFound(format!("slide {sid:?} has no question")))?;
    apply_question_kind(&slide.objects, q, kind)?;
    if kind == QuestionKind::Freeform {
        q.correct_when = Some(freeform_default);
    }
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

fn question_add_accepted(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "question.add_accepted";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let value = match opt::<String>(C, p, "text")? {
        Some(t) => text(C, t, "text")?,
        None => "answer".to_owned(),
    };
    {
        let q = question_of(s, &sid)?;
        require_kind(q, QuestionKind::FillIn, C)?;
        q.accepted.push(value);
    }
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

fn question_remove_accepted(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "question.remove_accepted";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    {
        let q = question_of(s, &sid)?;
        require_kind(q, QuestionKind::FillIn, C)?;
        let idx = index_of(C, p, q.accepted.len())?;
        q.accepted.remove(idx);
    }
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

fn question_update_accepted(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "question.update_accepted";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let value = text(C, req(C, p, "text")?, "text")?;
    {
        let q = question_of(s, &sid)?;
        require_kind(q, QuestionKind::FillIn, C)?;
        let idx = index_of(C, p, q.accepted.len())?;
        if let Some(slot) = q.accepted.get_mut(idx) {
            *slot = value;
        }
    }
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

fn question_set_case_sensitive(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "question.set_case_sensitive";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let flag: bool = req(C, p, "case_sensitive")?;
    {
        let q = question_of(s, &sid)?;
        require_kind(q, QuestionKind::FillIn, C)?;
        q.case_sensitive = flag;
    }
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

fn question_add_pair(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "question.add_pair";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let left = match opt::<String>(C, p, "left")? {
        Some(v) => text(C, v, "left")?,
        None => "Left".to_owned(),
    };
    let right = match opt::<String>(C, p, "right")? {
        Some(v) => text(C, v, "right")?,
        None => "Right".to_owned(),
    };
    let id = {
        let q = question_of(s, &sid)?;
        require_kind(q, QuestionKind::Matching, C)?;
        let id = fresh_id("p", q.pairs.iter().map(|pair| pair.id.as_str()));
        q.pairs.push(MatchPair {
            id: id.clone(),
            left,
            right,
        });
        id
    };
    keep_valid(s, was_valid)?;
    Ok(json!({ "id": id }))
}

fn question_remove_pair(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "question.remove_pair";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let id: String = req(C, p, "id")?;
    {
        let q = question_of(s, &sid)?;
        require_kind(q, QuestionKind::Matching, C)?;
        let Some(pos) = q.pairs.iter().position(|pair| pair.id == id) else {
            return Err(EngineError::NotFound(format!("pair {id:?} does not exist")));
        };
        q.pairs.remove(pos);
    }
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

fn question_update_pair(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "question.update_pair";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let id: String = req(C, p, "id")?;
    let left: Option<String> = opt(C, p, "left")?;
    let right: Option<String> = opt(C, p, "right")?;
    if left.is_none() && right.is_none() {
        return Err(bad(C, "missing `left` or `right`"));
    }
    let left = left.map(|v| text(C, v, "left")).transpose()?;
    let right = right.map(|v| text(C, v, "right")).transpose()?;
    {
        let q = question_of(s, &sid)?;
        require_kind(q, QuestionKind::Matching, C)?;
        let Some(pair) = q.pairs.iter_mut().find(|pair| pair.id == id) else {
            return Err(EngineError::NotFound(format!("pair {id:?} does not exist")));
        };
        if let Some(v) = left {
            pair.left = v;
        }
        if let Some(v) = right {
            pair.right = v;
        }
    }
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

fn question_reorder_pair(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "question.reorder_pair";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let id: String = req(C, p, "id")?;
    {
        let q = question_of(s, &sid)?;
        require_kind(q, QuestionKind::Matching, C)?;
        let to = dest_index(C, p, q.pairs.len())?;
        let Some(from) = q.pairs.iter().position(|pair| pair.id == id) else {
            return Err(EngineError::NotFound(format!("pair {id:?} does not exist")));
        };
        move_to(&mut q.pairs, from, to);
    }
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

fn question_reorder_sequence(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "question.reorder_sequence";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let id: String = req(C, p, "id")?;
    {
        let q = question_of(s, &sid)?;
        require_kind(q, QuestionKind::Sequence, C)?;
        let to = dest_index(C, p, q.choices.len())?;
        let Some(from) = q.choices.iter().position(|c| c.id == id) else {
            return Err(EngineError::NotFound(format!(
                "choice {id:?} does not exist"
            )));
        };
        move_to(&mut q.choices, from, to);
    }
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

fn question_set_hotspot_image(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "question.set_hotspot_image";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let image: String = req(C, p, "image")?;
    let slide = slide_mut(s, &sid)?;
    let obj = base_object(slide, &image)
        .ok_or_else(|| bad(C, format!("image {image:?} must name a base-layer object")))?;
    if !matches!(obj.kind, ObjectKind::Image { .. }) {
        return Err(bad(C, format!("object {image:?} is not an image")));
    }
    let q = slide
        .question
        .as_mut()
        .ok_or_else(|| EngineError::NotFound(format!("slide {sid:?} has no question")))?;
    require_kind(q, QuestionKind::Hotspot, C)?;
    q.hotspot_image = Some(image);
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

fn question_add_region(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "question.add_region";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let shape = opt::<HotspotShape>(C, p, "shape")?.unwrap_or(HotspotShape::Rect);
    let slide_size = s.course.slide_size;
    let slide = slide_mut(s, &sid)?;
    let (default_x, default_y) = if let Some(q) = &slide.question
        && let Some(ref img_id) = q.hotspot_image
        && let Some(img) = base_object(slide, img_id)
    {
        let cx = img.frame.x + img.frame.w * 0.5;
        let cy = img.frame.y + img.frame.h * 0.5;
        ((cx - 60.0).max(0.0), (cy - 40.0).max(0.0))
    } else {
        (
            ((slide_size.w as f32) * 0.5 - 60.0).max(0.0),
            ((slide_size.h as f32) * 0.5 - 40.0).max(0.0),
        )
    };
    let x = opt::<f32>(C, p, "x")?.unwrap_or(default_x);
    let y = opt::<f32>(C, p, "y")?.unwrap_or(default_y);
    let w = opt::<f32>(C, p, "w")?.unwrap_or(120.0);
    let h = opt::<f32>(C, p, "h")?.unwrap_or(80.0);

    if !x.is_finite()
        || !y.is_finite()
        || !w.is_finite()
        || !h.is_finite()
        || x < -100_000.0
        || x > 100_000.0
        || y < -100_000.0
        || y > 100_000.0
        || w <= 0.0
        || w > 100_000.0
        || h <= 0.0
        || h > 100_000.0
    {
        return Err(bad(C, "invalid coordinates or dimensions"));
    }

    let q = slide
        .question
        .as_mut()
        .ok_or_else(|| EngineError::NotFound(format!("slide {sid:?} has no question")))?;
    require_kind(q, QuestionKind::Hotspot, C)?;
    if q.regions.len() >= 20 {
        return Err(bad(C, "cannot exceed 20 regions"));
    }
    let correct = opt::<bool>(C, p, "correct")?
        .unwrap_or(q.regions.is_empty() || !q.regions.iter().any(|r| r.correct));
    let id = fresh_id("r", q.regions.iter().map(|r| r.id.as_str()));
    q.regions.push(HotspotRegion {
        id: id.clone(),
        shape,
        x,
        y,
        w,
        h,
        correct,
    });
    keep_valid(s, was_valid)?;
    Ok(json!({ "id": id }))
}

fn question_update_region(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "question.update_region";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let id: String = req(C, p, "id")?;
    let shape: Option<HotspotShape> = opt(C, p, "shape")?;
    let x: Option<f32> = opt(C, p, "x")?;
    let y: Option<f32> = opt(C, p, "y")?;
    let w: Option<f32> = opt(C, p, "w")?;
    let h: Option<f32> = opt(C, p, "h")?;
    let correct: Option<bool> = opt(C, p, "correct")?;

    for v in [&x, &y].into_iter().flatten() {
        if !v.is_finite() || *v < -100_000.0 || *v > 100_000.0 {
            return Err(bad(C, "coordinate out of range or not finite"));
        }
    }
    for v in [&w, &h].into_iter().flatten() {
        if !v.is_finite() || *v <= 0.0 || *v > 100_000.0 {
            return Err(bad(C, "dimension must be > 0 and <= 100000"));
        }
    }

    let q = question_of(s, &sid)?;
    require_kind(q, QuestionKind::Hotspot, C)?;
    let Some(pos) = q.regions.iter().position(|r| r.id == id) else {
        return Err(EngineError::NotFound(format!(
            "region {id:?} does not exist"
        )));
    };
    if correct == Some(false)
        && q.regions[pos].correct
        && q.regions.iter().filter(|r| r.correct).count() <= 1
    {
        return Err(bad(C, "hotspot needs at least one correct region"));
    }
    let region = &mut q.regions[pos];
    if let Some(sh) = shape {
        region.shape = sh;
    }
    if let Some(v) = x {
        region.x = v;
    }
    if let Some(v) = y {
        region.y = v;
    }
    if let Some(v) = w {
        region.w = v;
    }
    if let Some(v) = h {
        region.h = v;
    }
    if let Some(c) = correct {
        region.correct = c;
    }
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

fn question_remove_region(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "question.remove_region";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let id: String = req(C, p, "id")?;
    let q = question_of(s, &sid)?;
    require_kind(q, QuestionKind::Hotspot, C)?;
    if q.regions.len() <= 1 {
        return Err(bad(C, "hotspot needs at least one region"));
    }
    let Some(pos) = q.regions.iter().position(|r| r.id == id) else {
        return Err(EngineError::NotFound(format!(
            "region {id:?} does not exist"
        )));
    };
    if q.regions[pos].correct && q.regions.iter().filter(|r| r.correct).count() <= 1 {
        return Err(bad(C, "cannot remove the only correct region"));
    }
    q.regions.remove(pos);
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

fn question_set_markers(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "question.set_markers";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let markers: u32 = req(C, p, "markers")?;
    if !(1..=10).contains(&markers) {
        return Err(bad(C, "markers must be between 1 and 10"));
    }
    let q = question_of(s, &sid)?;
    require_kind(q, QuestionKind::Hotspot, C)?;
    q.markers = markers;
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

fn question_add_drag_item(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "question.add_drag_item";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let item: String = req(C, p, "item")?;
    let target: String = req(C, p, "target")?;
    if item == target {
        return Err(bad(C, "drag item cannot target itself"));
    }
    let slide = slide_mut(s, &sid)?;
    if base_object(slide, &item).is_none() {
        return Err(bad(C, format!("item {item:?} is not a base-layer object")));
    }
    if base_object(slide, &target).is_none() {
        return Err(bad(
            C,
            format!("target {target:?} is not a base-layer object"),
        ));
    }
    let q = slide
        .question
        .as_mut()
        .ok_or_else(|| EngineError::NotFound(format!("slide {sid:?} has no question")))?;
    require_kind(q, QuestionKind::DragDrop, C)?;
    if q.drag_items.len() >= 20 {
        return Err(bad(C, "cannot exceed 20 drag items"));
    }
    if q.drag_items.iter().any(|d| d.item == item) {
        return Err(bad(C, format!("drag item {item:?} already exists")));
    }
    if q.drag_items.iter().any(|d| d.item == target) {
        return Err(bad(C, format!("target {target:?} is a drag item")));
    }
    if q.drop_targets.contains(&item) {
        return Err(bad(C, format!("item {item:?} is already a drop target")));
    }
    if !q.drop_targets.contains(&target) {
        if q.drop_targets.len() >= 20 {
            return Err(bad(C, "cannot exceed 20 drop targets"));
        }
        q.drop_targets.push(target.clone());
    }
    q.drag_items.push(DragItem { item, target });
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

fn question_update_drag_item(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "question.update_drag_item";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let item: String = req(C, p, "item")?;
    let target: String = req(C, p, "target")?;
    if item == target {
        return Err(bad(C, "drag item cannot target itself"));
    }
    let slide = slide_mut(s, &sid)?;
    if base_object(slide, &target).is_none() {
        return Err(bad(
            C,
            format!("target {target:?} is not a base-layer object"),
        ));
    }
    let q = slide
        .question
        .as_mut()
        .ok_or_else(|| EngineError::NotFound(format!("slide {sid:?} has no question")))?;
    require_kind(q, QuestionKind::DragDrop, C)?;
    if q.drag_items.iter().any(|d| d.item == target) {
        return Err(bad(C, format!("target {target:?} is a drag item")));
    }
    let Some(drag) = q.drag_items.iter_mut().find(|d| d.item == item) else {
        return Err(EngineError::NotFound(format!(
            "drag item {item:?} does not exist"
        )));
    };
    drag.target = target.clone();
    if !q.drop_targets.contains(&target) {
        if q.drop_targets.len() >= 20 {
            return Err(bad(C, "cannot exceed 20 drop targets"));
        }
        q.drop_targets.push(target);
    }
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

fn question_remove_drag_item(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "question.remove_drag_item";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let item: String = req(C, p, "item")?;
    let q = question_of(s, &sid)?;
    require_kind(q, QuestionKind::DragDrop, C)?;
    if q.drag_items.len() <= 1 {
        return Err(bad(C, "drag_drop needs at least one drag item"));
    }
    let Some(pos) = q.drag_items.iter().position(|d| d.item == item) else {
        return Err(EngineError::NotFound(format!(
            "drag item {item:?} does not exist"
        )));
    };
    q.drag_items.remove(pos);
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

fn question_set_drop_targets(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "question.set_drop_targets";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let targets: Vec<String> = req(C, p, "targets")?;
    if targets.is_empty() || targets.len() > 20 {
        return Err(bad(C, "drop_targets must have 1 to 20 targets"));
    }
    let mut set = std::collections::BTreeSet::new();
    for t in &targets {
        if !set.insert(t.as_str()) {
            return Err(bad(C, format!("duplicate drop target {t:?}")));
        }
    }
    let slide = slide_mut(s, &sid)?;
    for t in &targets {
        if base_object(slide, t).is_none() {
            return Err(bad(
                C,
                format!("drop target {t:?} is not a base-layer object"),
            ));
        }
    }
    let q = slide
        .question
        .as_mut()
        .ok_or_else(|| EngineError::NotFound(format!("slide {sid:?} has no question")))?;
    require_kind(q, QuestionKind::DragDrop, C)?;
    for d in &q.drag_items {
        if targets.contains(&d.item) {
            return Err(bad(
                C,
                format!(
                    "object {:?} cannot be both drag item and drop target",
                    d.item
                ),
            ));
        }
        if !targets.contains(&d.target) {
            return Err(bad(
                C,
                format!(
                    "target {:?} used by drag item {:?} must be in drop_targets",
                    d.target, d.item
                ),
            ));
        }
    }
    q.drop_targets = targets;
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

fn question_set_drag_options(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "question.set_drag_options";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let snap_to_center: Option<bool> = opt(C, p, "snap_to_center")?;
    let return_if_wrong: Option<bool> = opt(C, p, "return_if_wrong")?;
    let one_per_target: Option<bool> = opt(C, p, "one_per_target")?;

    let q = question_of(s, &sid)?;
    require_kind(q, QuestionKind::DragDrop, C)?;
    if let Some(v) = snap_to_center {
        q.snap_to_center = v;
    }
    if let Some(v) = return_if_wrong {
        q.return_if_wrong = v;
    }
    if let Some(v) = one_per_target {
        q.one_per_target = v;
    }
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

/// True when a trigger only makes sense on a slide that has a question.
///
/// Validation rejects `submit_question` on a slide without a question, and a
/// `submit` event can never fire once the question is gone.
fn trigger_needs_question(t: &Trigger) -> bool {
    t.action == Action::SubmitQuestion || t.when == Event::Submit
}

fn condition_references_object(c: &Condition, object: &str) -> bool {
    match c {
        Condition::All(list) | Condition::Any(list) => {
            list.iter().any(|c| condition_references_object(c, object))
        }
        Condition::Not(inner) => condition_references_object(inner, object),
        Condition::Compare { .. } => false,
        Condition::ObjectState { object: target, .. } => target == object,
    }
}

/// Every object a trigger names in its event, action or condition.
fn trigger_references_object(t: &Trigger, object: &str) -> bool {
    let in_event = match &t.when {
        Event::Click { object: target }
        | Event::HoverStart { object: target }
        | Event::HoverEnd { object: target }
        | Event::MediaEnds { object: target }
        | Event::StateChanges { object: target, .. }
        | Event::AnimationCompletes { object: target, .. } => target == object,
        _ => false,
    };
    let in_action = match &t.action {
        Action::ChangeState { object: target, .. }
        | Action::PlayMedia { object: target }
        | Action::PauseMedia { object: target }
        | Action::StopMedia { object: target }
        | Action::MoveOnPath { object: target, .. } => target == object,
        _ => false,
    };
    in_event
        || in_action
        || t.condition
            .as_ref()
            .is_some_and(|c| condition_references_object(c, object))
}

/// The object id a trigger's event fires from, when the event has one.
fn event_object(event: &Event) -> Option<&str> {
    match event {
        Event::Click { object }
        | Event::HoverStart { object }
        | Event::HoverEnd { object }
        | Event::MediaEnds { object }
        | Event::StateChanges { object, .. }
        | Event::AnimationCompletes { object, .. } => Some(object),
        _ => None,
    }
}

/// Remove a question and everything that cannot exist without it, in one edit:
/// submit triggers, dead submit events, and Submit buttons that have nothing
/// else to do. Layers, feedback layers and all other objects stay.
fn question_delete(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "question.delete";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let slide = slide_mut(s, &sid)?;
    if slide.question.is_none() {
        return Err(EngineError::NotFound(format!(
            "slide {sid:?} has no question"
        )));
    }
    slide.question = None;

    // Objects named by submit_question triggers may have been there only to
    // submit. Remember them before the triggers go.
    let mut submit_objects: BTreeSet<String> = BTreeSet::new();
    for trigger in slide
        .triggers
        .iter()
        .chain(slide.layers.iter().flat_map(|layer| layer.triggers.iter()))
    {
        if trigger.action == Action::SubmitQuestion
            && let Some(object) = event_object(&trigger.when)
        {
            submit_objects.insert(object.to_owned());
        }
    }

    slide.triggers.retain(|t| !trigger_needs_question(t));
    for layer in &mut slide.layers {
        layer.triggers.retain(|t| !trigger_needs_question(t));
    }

    // A button that had a submit_question trigger and now has no trigger (and
    // is not named by any other trigger) was only there to submit.
    let mut lonely_buttons: BTreeSet<String> = BTreeSet::new();
    let all_objects = slide
        .objects
        .iter()
        .chain(slide.layers.iter().flat_map(|layer| layer.objects.iter()));
    for object in all_objects {
        if !submit_objects.contains(&object.id) || !matches!(object.kind, ObjectKind::Button { .. })
        {
            continue;
        }
        let referenced = slide
            .triggers
            .iter()
            .chain(slide.layers.iter().flat_map(|layer| layer.triggers.iter()));
        if !referenced
            .into_iter()
            .any(|t| trigger_references_object(t, &object.id))
        {
            lonely_buttons.insert(object.id.clone());
        }
    }
    slide
        .objects
        .retain(|object| !lonely_buttons.contains(&object.id));
    for layer in &mut slide.layers {
        layer
            .objects
            .retain(|object| !lonely_buttons.contains(&object.id));
    }

    // Feedback layers made for this question, if nobody edited them or points at them.
    let unused_feedback: Vec<String> = slide
        .layers
        .iter()
        .filter(|layer| is_untouched_feedback_layer(layer))
        .map(|layer| layer.id.clone())
        .filter(|id| {
            !slide
                .triggers
                .iter()
                .chain(slide.layers.iter().flat_map(|layer| layer.triggers.iter()))
                .any(|t| {
                    matches!(&t.action, Action::ShowLayer { layer } if layer == id)
                        || matches!(&t.action, Action::HideLayer { layer: Some(layer) } if layer == id)
                })
        })
        .collect();
    slide
        .layers
        .retain(|layer| !unused_feedback.contains(&layer.id));

    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

/// A `Correct` or `Incorrect` layer still exactly as `add_feedback_layers` made it.
fn is_untouched_feedback_layer(layer: &Layer) -> bool {
    let expected = match layer.name.as_str() {
        "Correct" => "Correct!",
        "Incorrect" => "Not quite.",
        _ => return false,
    };
    layer.triggers.is_empty()
        && matches!(
            layer.objects.as_slice(),
            [Object { kind: ObjectKind::Text { text, .. }, .. }] if text == expected
        )
}
