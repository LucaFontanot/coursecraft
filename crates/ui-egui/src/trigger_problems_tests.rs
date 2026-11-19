//! Headless UI tests for Step 4: validation problems shown on triggers.

use crate::BottomTab;
use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use coursecraft_render::Icon;
use egui::Event;

fn hello() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    Course::from_json(&json).expect("valid course")
}

fn warning_glyph() -> String {
    coursecraft_render::icon(Icon::Warning, coursecraft_render::ICON_SM)
        .text()
        .to_owned()
}

/// A course with no trigger problems shows neither a warning icon nor the
/// attention note in the Triggers tab.
#[test]
fn healthy_course_shows_no_trigger_warning_or_note() {
    let mut h = TestHarness::new(hello());
    h.editor.selection.slide = "s-intro".into();
    h.editor.ui.bottom_tab = BottomTab::Triggers;
    h.idle_frame();
    h.idle_frame();

    assert!(
        h.find_text(&warning_glyph()).is_none(),
        "no warning icon should be shown on healthy triggers"
    );
    assert!(
        h.find_text_containing("need attention").is_none()
            && h.find_text_containing("needs attention").is_none(),
        "no attention note should be shown when no triggers have issues"
    );
}

/// A trigger pointing at a deleted object shows a warning icon with the
/// validation message as a tooltip; fixing the trigger removes the icon and note.
#[test]
fn broken_trigger_shows_icon_and_tooltip_and_fixing_removes_them() {
    let json = r#"{
        "schema_version": 1,
        "id": "broken-trig",
        "title": "Broken Trigger",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "sc0",
            "title": "Scene 0",
            "slides": [{
                "id": "s0",
                "title": "Slide 0",
                "duration_ms": 4000,
                "objects": [],
                "triggers": [
                    { "when": { "event": "click", "object": "deleted-btn" }, "action": { "action": "pause_timeline" } }
                ]
            }]
        }]
    }"#;
    let course = Course::from_json(json).expect("course parses");
    let mut h = TestHarness::new(course);
    h.editor.selection.slide = "s0".into();
    h.editor.ui.bottom_tab = BottomTab::Triggers;
    h.idle_frame();
    h.idle_frame();

    // The note above the list should appear.
    assert!(
        h.find_text("1 trigger on this slide needs attention")
            .is_some(),
        "expected single-trigger attention note"
    );

    // The warning icon should be painted in the trigger row and show a tooltip on hover.
    let glyphs = h.find_texts(&warning_glyph());
    assert!(
        glyphs.len() >= 2,
        "expected warning icon on both the note and the trigger row, got {}",
        glyphs.len()
    );
    let row_icon = *glyphs.last().expect("row warning icon");
    h.frame(vec![Event::PointerMoved(row_icon)]);
    h.frame(vec![Event::PointerMoved(row_icon)]);
    assert!(
        h.find_text_containing("deleted-btn").is_some(),
        "hovering the warning icon should show the validation message mentioning deleted-btn"
    );

    // Fix the trigger by updating it to a valid timeline_starts trigger.
    let res = h.editor.run(
        "trigger.update",
        serde_json::json!({
            "slide": "s0",
            "index": 0,
            "trigger": {
                "when": { "event": "timeline_starts" },
                "action": { "action": "pause_timeline" }
            }
        }),
    );
    assert!(res.is_some(), "trigger.update should succeed");
    h.idle_frame();
    h.idle_frame();

    assert!(
        h.find_text(&warning_glyph()).is_none(),
        "fixing the trigger should remove the warning icon"
    );
    assert!(
        h.find_text_containing("needs attention").is_none(),
        "fixing the trigger should remove the attention note"
    );
}

/// When multiple triggers on a slide have problems, the note reports the count
/// and clicking "Fix the first one" selects the first broken trigger and opens its editor.
#[test]
fn multiple_broken_triggers_show_count_and_edit_first_opens_editor() {
    let json = r#"{
        "schema_version": 1,
        "id": "multi-broken",
        "title": "Multi Broken",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "sc0",
            "title": "Scene 0",
            "slides": [{
                "id": "s0",
                "title": "Slide 0",
                "duration_ms": 4000,
                "objects": [],
                "triggers": [
                    { "when": { "event": "click", "object": "deleted-btn" }, "action": { "action": "pause_timeline" } },
                    { "when": { "event": "timeline_starts" }, "action": { "action": "jump_to_slide", "target": { "id": "missing-slide" } } }
                ]
            }]
        }]
    }"#;
    let course = Course::from_json(json).expect("course parses");
    let mut h = TestHarness::new(course);
    h.editor.selection.slide = "s0".into();
    h.editor.ui.bottom_tab = BottomTab::Triggers;
    h.idle_frame();
    h.idle_frame();

    assert!(
        h.find_text("2 triggers on this slide need attention")
            .is_some(),
        "expected plural attention note for 2 broken triggers"
    );

    let edit_btn = h
        .find_text("Fix the first one")
        .expect("Fix the first one button");
    h.click_at(edit_btn);
    h.idle_frame();

    let form = h
        .editor
        .ui
        .trigger_form
        .as_ref()
        .expect("clicking Edit first should open the trigger editor");
    assert_eq!(
        form.editing_index,
        Some(0),
        "Edit first should open the first broken trigger (index 0)"
    );
}
