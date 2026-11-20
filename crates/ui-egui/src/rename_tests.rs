//! Headless tests for in-place slide rename, in-place timeline object rename,
//! and timeline duration in seconds.

use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use egui::{Key, Modifiers};

fn hello() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    Course::from_json(&json).expect("valid course")
}

fn slide_title(harness: &TestHarness, slide_id: &str) -> String {
    harness
        .editor
        .session
        .course
        .slide(slide_id)
        .expect("slide")
        .title
        .clone()
}

fn object_name(harness: &TestHarness, slide_id: &str, object_id: &str) -> String {
    harness
        .editor
        .session
        .course
        .slide(slide_id)
        .and_then(|slide| slide.object(object_id))
        .map(|(obj, _)| obj.name.clone())
        .expect("object")
}

#[test]
fn double_click_slide_title_renames_and_undoes() {
    let mut harness = TestHarness::new(hello());
    harness.idle_frame();

    let orig = slide_title(&harness, "s-intro");
    let left_panel = harness.left_panel_rect();
    let title_pos = harness
        .find_text_in(&orig, left_panel)
        .expect("slide title in left panel");

    harness.click_at(title_pos);
    harness.click_at(title_pos);
    harness.idle_frame();
    assert!(
        harness.editor.ui.slide_rename.is_some(),
        "double-click should open inline slide rename"
    );

    harness.type_text("Welcome");
    harness.press(Key::Enter, Modifiers::NONE);
    harness.idle_frame();

    assert!(harness.editor.ui.slide_rename.is_none());
    assert_eq!(slide_title(&harness, "s-intro"), "Welcome");

    harness.editor.run("edit.undo", serde_json::json!({}));
    assert_eq!(slide_title(&harness, "s-intro"), orig);
}

#[test]
fn slide_rename_escape_cancels() {
    let mut harness = TestHarness::new(hello());
    harness.idle_frame();

    let orig = slide_title(&harness, "s-intro");
    let left_panel = harness.left_panel_rect();
    let title_pos = harness
        .find_text_in(&orig, left_panel)
        .expect("slide title in left panel");

    harness.click_at(title_pos);
    harness.click_at(title_pos);
    harness.idle_frame();
    assert!(harness.editor.ui.slide_rename.is_some());

    harness.type_text("Should Cancel");
    harness.press(Key::Escape, Modifiers::NONE);
    harness.idle_frame();

    assert!(harness.editor.ui.slide_rename.is_none());
    assert_eq!(slide_title(&harness, "s-intro"), orig);
}

#[test]
fn slide_rename_empty_name_ignored() {
    let mut harness = TestHarness::new(hello());
    harness.idle_frame();

    let orig = slide_title(&harness, "s-intro");
    let left_panel = harness.left_panel_rect();
    let title_pos = harness
        .find_text_in(&orig, left_panel)
        .expect("slide title in left panel");

    harness.click_at(title_pos);
    harness.click_at(title_pos);
    harness.idle_frame();
    assert!(harness.editor.ui.slide_rename.is_some());

    harness.press(Key::Backspace, Modifiers::NONE);
    harness.type_text("   ");
    harness.press(Key::Enter, Modifiers::NONE);
    harness.idle_frame();

    assert!(harness.editor.ui.slide_rename.is_none());
    assert_eq!(slide_title(&harness, "s-intro"), orig);
}

#[test]
fn slide_rename_via_context_menu_and_f2() {
    let mut harness = TestHarness::new(hello());
    harness.idle_frame();

    let orig = slide_title(&harness, "s-intro");
    let left_panel = harness.left_panel_rect();
    let title_pos = harness
        .find_text_in(&orig, left_panel)
        .expect("slide title in left panel");

    // Right-click opens menu with Rename
    harness.right_click_at(title_pos);
    let rename_btn = harness
        .find_text("Rename")
        .expect("Rename item in slide context menu");
    harness.click_at(rename_btn);
    harness.idle_frame();
    assert!(harness.editor.ui.slide_rename.is_some());
    harness.press(Key::Escape, Modifiers::NONE);
    harness.idle_frame();
    assert!(harness.editor.ui.slide_rename.is_none());

    // F2 when slides panel has focus opens inline rename
    harness.click_at(title_pos);
    harness.press(Key::F2, Modifiers::NONE);
    harness.idle_frame();
    assert!(harness.editor.ui.slide_rename.is_some());
}

#[test]
fn timeline_object_rename_works_and_undoes() {
    let mut harness = TestHarness::new(hello());
    harness.editor.selection.slide = "s-intro".into();
    harness.idle_frame();

    let orig = object_name(&harness, "s-intro", "btn-start");
    let display = if orig.is_empty() {
        "btn-start".to_owned()
    } else {
        orig.clone()
    };
    let row_rect = harness
        .editor
        .ui
        .timeline
        .object_row_rects
        .get("btn-start")
        .copied()
        .expect("timeline row rect");
    let name_pos = harness
        .find_text_in(&display, row_rect)
        .expect("object name in timeline row");

    harness.click_at(name_pos);
    harness.click_at(name_pos);
    harness.idle_frame();
    assert!(
        harness.editor.ui.timeline.object_rename.is_some(),
        "double-click on timeline object name should open inline rename"
    );

    harness.type_text("Begin Button");
    harness.press(Key::Enter, Modifiers::NONE);
    harness.idle_frame();

    assert!(harness.editor.ui.timeline.object_rename.is_none());
    assert_eq!(
        object_name(&harness, "s-intro", "btn-start"),
        "Begin Button"
    );

    harness.editor.run("edit.undo", serde_json::json!({}));
    assert_eq!(object_name(&harness, "s-intro", "btn-start"), orig);
}

#[test]
fn timeline_duration_shows_seconds_and_edits_in_seconds() {
    let mut harness = TestHarness::new(hello());
    harness.editor.selection.slide = "s-intro".into();
    harness.editor.run(
        "slide.update",
        serde_json::json!({ "slide": "s-intro", "duration_ms": 5000 }),
    );
    harness.idle_frame();

    let dur_rect = harness
        .editor
        .ui
        .timeline
        .duration_edit_rect
        .expect("duration edit rect");
    assert!(
        harness.find_text_in("5.0 s", dur_rect).is_some(),
        "5000 ms duration should display as 5.0 s (dur_ms={})",
        harness.editor.ui.timeline.duration_ms,
    );

    harness.set_field(dur_rect.center(), "2.5");
    harness.idle_frame();

    let slide = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("slide");
    assert_eq!(slide.duration_ms, 2500);
}
