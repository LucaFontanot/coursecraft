//! Tests for the spell check UI: underline geometry, as-you-type recording and
//! the course-wide check window's actions.

use crate::test_harness::TestHarness;
use crate::{Editor, canvas::InlineEdit, spell_check};
use coursecraft_course::Course;
use coursecraft_spell::Misspelling;
use egui::{
    Color32, FontId, pos2,
    text::{CCursor, LayoutJob},
};
use serde_json::json;

fn course_with_typos() -> Course {
    let data = json!({
        "schema_version": 1,
        "id": "spell-typos",
        "title": "Spell Course",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "sc",
            "title": "Scene",
            "slides": [
                { "id": "s1", "title": "Intro", "duration_ms": 3000,
                  "objects": [{ "id": "t1", "kind": { "type": "text", "text": "A correct sentence." },
                                "frame": { "x": 80, "y": 80, "w": 600, "h": 120 } }] },
                { "id": "s2", "title": "Second", "duration_ms": 3000, "notes": "teh again",
                  "objects": [{ "id": "t2", "kind": { "type": "text", "text": "teh" },
                                "frame": { "x": 80, "y": 80, "w": 600, "h": 120 } }] },
                { "id": "s3", "title": "Third", "duration_ms": 3000,
                  "objects": [{ "id": "t3", "kind": { "type": "text", "text": "recieve" },
                                "frame": { "x": 80, "y": 80, "w": 600, "h": 120 } }] }
            ]
        }]
    });
    Course::from_json(&data.to_string()).expect("valid spell course")
}

fn galley_for(harness: &TestHarness, text: &str, wrap: f32) -> std::sync::Arc<egui::Galley> {
    harness.ctx.fonts_mut(|fonts| {
        fonts.layout_job(LayoutJob::simple(
            text.to_owned(),
            FontId::proportional(16.0),
            Color32::WHITE,
            wrap,
        ))
    })
}

#[test]
fn underline_rects_are_drawn_under_the_right_words() {
    let mut harness = TestHarness::new(coursecraft_engine::Session::blank("Rects").course);
    harness.idle_frame();
    let text = "Teh cat";
    let galley = galley_for(&harness, text, 400.0);
    let origin = pos2(10.0, 20.0);
    let miss = Misspelling {
        start: 0,
        end: 3,
        word: "Teh".to_owned(),
    };
    let words = spell_check::underline_rects(&galley, origin, text, &[miss]);
    assert_eq!(words.len(), 1);
    assert_eq!(words[0].word, "Teh");
    let rect = words[0].rects[0];
    let left = galley.pos_from_cursor(CCursor::new(0)).left() + origin.x;
    let right = galley.pos_from_cursor(CCursor::new(3)).right() + origin.x;
    assert!((rect.left() - left).abs() < 0.5, "left {rect:?} vs {left}");
    assert!(
        (rect.right() - right).abs() < 0.5,
        "right {rect:?} vs {right}"
    );
    let space_x = galley.pos_from_cursor(CCursor::new(4)).right() + origin.x;
    assert!(
        rect.right() <= space_x,
        "the underline stops before the space"
    );

    // A different misspelling gets a rect further to the right, on the same row.
    let cat = Misspelling {
        start: 4,
        end: 7,
        word: "cat".to_owned(),
    };
    let two = spell_check::underline_rects(&galley, origin, text, &[cat]);
    assert_eq!(two.len(), 1);
    assert!(
        two[0].rects[0].left() > rect.right(),
        "second word is to the right"
    );
}

#[test]
fn a_word_that_wraps_gets_one_rect_per_row() {
    let mut harness = TestHarness::new(coursecraft_engine::Session::blank("Wrap").course);
    harness.idle_frame();
    let text = "supercalifragilisticexpialidocious";
    let galley = galley_for(&harness, text, 60.0);
    let miss = Misspelling {
        start: 0,
        end: text.len(),
        word: text.to_owned(),
    };
    let words = spell_check::underline_rects(&galley, pos2(0.0, 0.0), text, &[miss]);
    assert_eq!(words.len(), 1);
    assert!(
        words[0].rects.len() >= 2,
        "the long word wrapped: {:?}",
        words[0].rects
    );
    for pair in words[0].rects.windows(2) {
        assert!(
            pair[0].bottom() <= pair[1].bottom(),
            "rows run top to bottom"
        );
    }
}

#[test]
fn canvas_records_misspellings_only_while_checking_is_on() {
    let mut harness = TestHarness::new(course_with_typos());
    harness.editor.selection.slide = "s2".to_owned();
    harness.editor.ui.inline_edit = Some(InlineEdit {
        slide: "s2".to_owned(),
        object: "t2".to_owned(),
        buffer: "Teh cat".to_owned(),
        original: String::new(),
        is_button: false,
        is_shape: false,
        cell: None,
        just_entered: false,
        live_runs: Vec::new(),
        live_paragraphs: Vec::new(),
        last_revision: 0,
        state_name: None,
        extra_id: None,
    });
    harness.idle_frame();
    assert_eq!(
        harness.editor.ui.spell_inline_debug,
        vec!["Teh".to_owned()],
        "Teh is underlined while as-you-type checking is on"
    );

    harness.editor.ui.prefs.spell_check_as_you_type = false;
    harness.idle_frame();
    assert!(
        harness.editor.ui.spell_inline_debug.is_empty(),
        "nothing is underlined when the preference is off"
    );
}

#[test]
fn inspector_text_box_receives_misspellings_for_its_buffer() {
    let mut harness = TestHarness::new(course_with_typos());
    harness.editor.selection.slide = "s2".to_owned();
    harness.editor.selection.object = Some("t2".to_owned());
    harness.editor.ui.inspector_tab = crate::InspectorTab::Format;
    harness.idle_frame();
    assert_eq!(harness.editor.ui.properties.text_body, "teh");
    assert!(
        harness
            .editor
            .ui
            .properties
            .spell_words
            .iter()
            .any(|word| word.word == "teh"),
        "the inspector text box underlines the typo too"
    );
}

#[test]
fn right_clicking_a_squiggle_opens_the_word_menu() {
    let mut harness = TestHarness::new(course_with_typos());
    harness.editor.selection.slide = "s2".to_owned();
    harness.editor.ui.inline_edit = Some(InlineEdit {
        slide: "s2".to_owned(),
        object: "t2".to_owned(),
        buffer: "Teh cat".to_owned(),
        original: String::new(),
        is_button: false,
        is_shape: false,
        cell: None,
        just_entered: false,
        live_runs: Vec::new(),
        live_paragraphs: Vec::new(),
        last_revision: 0,
        state_name: None,
        extra_id: None,
    });
    harness.idle_frame();
    let center = harness.editor.ui.spell_inline_rects[0].rects[0].center();
    harness.right_click_at(center);
    harness.idle_frame();
    assert!(
        harness.find_text("Add to Dictionary").is_some(),
        "egui's text-edit context menu is used for the word actions"
    );
    assert!(harness.find_text("Ignore").is_some());

    let add = harness
        .find_text("Add to Dictionary")
        .expect("add to dictionary item");
    harness.click_at(add);
    harness.idle_frame();
    assert!(
        harness
            .editor
            .ui
            .spell_words
            .iter()
            .any(|word| word == "Teh"),
        "the chosen word is added to the personal dictionary"
    );
}

#[test]
fn choosing_a_suggestion_replaces_the_word_and_keeps_editing() {
    let mut harness = TestHarness::new(course_with_typos());
    harness.editor.selection.slide = "s2".to_owned();
    harness.editor.ui.inline_edit = Some(InlineEdit {
        slide: "s2".to_owned(),
        object: "t2".to_owned(),
        buffer: "Teh cat".to_owned(),
        original: String::new(),
        is_button: false,
        is_shape: false,
        cell: None,
        just_entered: false,
        live_runs: Vec::new(),
        live_paragraphs: Vec::new(),
        last_revision: 0,
        state_name: None,
        extra_id: None,
    });
    harness.idle_frame();
    let suggestion = harness
        .editor
        .spell_suggestions("Teh")
        .into_iter()
        .next()
        .expect("a suggestion for Teh");
    let center = harness.editor.ui.spell_inline_rects[0].rects[0].center();
    harness.right_click_at(center);
    harness.idle_frame();
    let item = harness
        .find_text(&suggestion)
        .expect("the suggestion is in the word menu");
    harness.click_at(item);
    harness.idle_frame();
    let edit = harness
        .editor
        .ui
        .inline_edit
        .as_ref()
        .expect("the text edit is still open");
    assert_eq!(edit.buffer, format!("{suggestion} cat"));
}

#[test]
fn f7_opens_the_check_spelling_window() {
    let mut harness = TestHarness::new(course_with_typos());
    harness.press(egui::Key::F7, egui::Modifiers::NONE);
    harness.idle_frame();
    assert!(harness.editor.ui.spell_check.open);
    assert!(harness.find_text("Check spelling").is_some());
}

#[test]
fn check_spelling_says_complete_when_no_words_are_left() {
    let mut harness = TestHarness::new(coursecraft_engine::Session::blank("Clean").course);
    spell_check::open(&mut harness.editor);
    harness.idle_frame();
    assert!(harness.find_text("Spelling check complete.").is_some());
}

#[test]
fn check_spelling_walks_in_order_and_changes_one_location() {
    let mut editor = Editor::new(course_with_typos());
    let issues = spell_check::collect_issues(&mut editor);
    let words: Vec<&str> = issues
        .iter()
        .map(|issue| issue.misspelling.word.as_str())
        .collect();
    assert_eq!(words, vec!["teh", "teh", "recieve"]);
    assert!(
        issues[0].item.label.contains("Slide 2"),
        "label: {}",
        issues[0].item.label
    );
    assert!(
        issues[2].item.label.contains("Slide 3"),
        "label: {}",
        issues[2].item.label
    );

    let state = &mut editor.ui.spell_check;
    state.issues = issues;
    state.index = 0;
    let revision = editor.session.revision;
    assert!(spell_check::change_one(&mut editor, 0, "the"));
    assert_eq!(
        editor.session.revision,
        revision.saturating_add(1),
        "one Change is one undo step"
    );
    let slide = editor.session.course.slide("s2").expect("slide 2");
    assert_eq!(slide.notes, "the again");
    assert_eq!(
        slide
            .object("t2")
            .and_then(|(object, _)| match &object.kind {
                coursecraft_course::ObjectKind::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .as_deref(),
        Some("teh"),
        "the other location is untouched"
    );

    editor.run("edit.undo", json!({}));
    let slide = editor.session.course.slide("s2").expect("slide 2");
    assert_eq!(slide.notes, "teh again", "undo restores the first change");
}

#[test]
fn check_spelling_change_all_and_add_to_dictionary() {
    let mut editor = Editor::new(course_with_typos());
    let issues = spell_check::collect_issues(&mut editor);
    editor.ui.spell_check.issues = issues;
    assert!(spell_check::change_all(&mut editor, 0, "the"));
    let slide = editor.session.course.slide("s2").expect("slide 2");
    assert_eq!(slide.notes, "the again");
    assert_eq!(
        slide
            .object("t2")
            .and_then(|(object, _)| match &object.kind {
                coursecraft_course::ObjectKind::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .as_deref(),
        Some("the"),
        "Change All fixes every `teh`"
    );

    let issues = spell_check::collect_issues(&mut editor);
    assert!(
        issues
            .iter()
            .any(|issue| issue.misspelling.word == "recieve")
    );
    editor.ui.spell_check.issues = issues;
    let index = editor
        .ui
        .spell_check
        .issues
        .iter()
        .position(|issue| issue.misspelling.word == "recieve")
        .expect("recieve is still misspelled");
    spell_check::add_issue_word(&mut editor, index);
    assert!(editor.ui.spell_words.iter().any(|word| word == "recieve"));
    let remaining = spell_check::collect_issues(&mut editor);
    assert!(
        remaining
            .iter()
            .all(|issue| issue.misspelling.word != "recieve"),
        "an added word is no longer reported"
    );
}

#[test]
fn spell_check_flags_misspelled_words_in_glossary_definitions() {
    let mut session = coursecraft_engine::Session::blank("Glossary Spell");
    session
        .execute(
            "glossary.add",
            &serde_json::json!({
                "id": "g1",
                "term": "Widget",
                "definition": "A teh widget definition."
            }),
        )
        .expect("add glossary");
    let mut editor = Editor::new(session.course);
    let issues = spell_check::collect_issues(&mut editor);
    assert!(
        issues.iter().any(|issue| {
            issue.item.location.field.starts_with("glossary/")
                && issue.item.location.field.ends_with("/definition")
                && issue.misspelling.word == "teh"
        }),
        "expected misspelled word in glossary definition: {issues:?}"
    );
}
