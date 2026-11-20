//! Bug hunt (2026-11-20, M24): canvas Alt-drag edge cases and variable entry.

use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use egui::{Event, Modifiers, Pos2, vec2};

fn hello() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    Course::from_json(&json).expect("valid course")
}

fn object_center(h: &TestHarness, slide_id: &str, obj_id: &str) -> (Pos2, f32) {
    let slide_rect = h.editor.ui.last_slide_rect.expect("slide rect");
    let slide_w = h.editor.session.course.slide_size.w as f32;
    let scale = slide_rect.width() / slide_w;
    let (obj, _) = h
        .editor
        .session
        .course
        .slide(slide_id)
        .and_then(|s| s.object(obj_id))
        .expect("object");
    let center = slide_rect.min
        + vec2(
            (obj.frame.x + obj.frame.w * 0.5) * scale,
            (obj.frame.y + obj.frame.h * 0.5) * scale,
        );
    (center, scale)
}

fn count(h: &TestHarness, slide: &str) -> usize {
    h.editor
        .session
        .course
        .slide(slide)
        .expect("slide")
        .objects
        .len()
}

fn frame_of(h: &TestHarness, slide: &str, obj: &str) -> coursecraft_course::Rect {
    h.editor
        .session
        .course
        .slide(slide)
        .and_then(|s| s.object(obj))
        .expect("object")
        .0
        .frame
}

fn prep(h: &mut TestHarness) {
    h.editor.ui.snap_to_guides = false;
    h.idle_frame();
    h.idle_frame();
}

#[test]
fn alt_drag_multi_selection_duplicates_both_and_one_undo_removes_both() {
    let mut h = TestHarness::new(hello());
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.set_many(["btn-start", "btn-about"]);
    prep(&mut h);
    let before = count(&h, "s-intro");

    let (start, scale) = object_center(&h, "s-intro", "btn-start");
    h.drag_by(start, vec2(30.0 * scale, 0.0), 3, Modifiers::ALT);
    h.idle_frame();
    assert_eq!(count(&h, "s-intro"), before + 2, "two copies expected");

    h.editor.session.undo().expect("undo");
    assert_eq!(count(&h, "s-intro"), before, "one undo removes both copies");
}

#[test]
fn alt_drag_then_undo_then_redo_restores_the_copy() {
    let mut h = TestHarness::new(hello());
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.object = Some("btn-start".into());
    prep(&mut h);
    let before = count(&h, "s-intro");

    let (start, scale) = object_center(&h, "s-intro", "btn-start");
    h.drag_by(start, vec2(30.0 * scale, 10.0 * scale), 3, Modifiers::ALT);
    h.idle_frame();
    assert_eq!(count(&h, "s-intro"), before + 1);

    h.editor.session.undo().expect("undo");
    assert_eq!(count(&h, "s-intro"), before);
    h.editor.session.redo().expect("redo");
    assert_eq!(
        count(&h, "s-intro"),
        before + 1,
        "redo brings the copy back"
    );
}

#[test]
fn alt_drag_of_a_locked_object_does_not_move_or_copy_it() {
    let mut h = TestHarness::new(hello());
    h.editor.run(
        "object.lock",
        serde_json::json!({ "slide": "s-intro", "object": "btn-start", "locked": true }),
    );
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.object = Some("btn-start".into());
    prep(&mut h);
    let before_count = count(&h, "s-intro");
    let before_frame = frame_of(&h, "s-intro", "btn-start");

    let (start, scale) = object_center(&h, "s-intro", "btn-start");
    h.drag_by(start, vec2(30.0 * scale, 10.0 * scale), 3, Modifiers::ALT);
    h.idle_frame();

    let after_frame = frame_of(&h, "s-intro", "btn-start");
    assert!(
        (after_frame.x - before_frame.x).abs() < 0.5
            && (after_frame.y - before_frame.y).abs() < 0.5,
        "locked object must not move"
    );
    assert_eq!(
        count(&h, "s-intro"),
        before_count,
        "a locked object must not be duplicated by Alt-drag"
    );
}

#[test]
fn variable_add_duplicate_name_is_refused_by_engine() {
    let mut h = TestHarness::new(hello());
    h.editor.run(
        "variable.add",
        serde_json::json!({ "name": "score", "value": 0.0 }),
    );
    let before = h.editor.session.course.variables.len();
    let _ = h.editor.run(
        "variable.add",
        serde_json::json!({ "name": "score", "value": 1.0 }),
    );
    let named = h
        .editor
        .session
        .course
        .variables
        .iter()
        .filter(|v| v.name == "score")
        .count();
    assert_eq!(named, 1, "duplicate variable name must not be added twice");
    assert!(h.editor.session.course.variables.len() <= before + 1);
}

#[test]
fn variable_add_with_spaces_and_emoji_name_is_refused_with_a_message() {
    let mut h = TestHarness::new(hello());
    let res = h.editor.run(
        "variable.add",
        serde_json::json!({ "name": "my score 🎉", "value": 0.0 }),
    );
    assert!(res.is_none(), "names outside [A-Za-z0-9_-] are refused");
    assert!(
        h.editor
            .ui
            .last_error
            .as_ref()
            .is_some_and(|(m, _)| m.contains("1-128 chars")),
        "refusal should say what names are allowed"
    );
}

#[test]
fn locked_object_plain_drag_does_not_move_it() {
    let mut h = TestHarness::new(hello());
    let _ = h.editor.run(
        "object.lock",
        serde_json::json!({ "slide": "s-intro", "object": "btn-start", "locked": true }),
    );
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.object = Some("btn-start".into());
    prep(&mut h);
    let before = frame_of(&h, "s-intro", "btn-start");
    let (start, scale) = object_center(&h, "s-intro", "btn-start");
    h.drag_by(start, vec2(30.0 * scale, 10.0 * scale), 3, Modifiers::NONE);
    h.idle_frame();
    let after = frame_of(&h, "s-intro", "btn-start");
    assert!(
        (after.x - before.x).abs() < 0.5 && (after.y - before.y).abs() < 0.5,
        "plain drag must not move a locked object"
    );
}

#[test]
fn canvas_space_key_without_text_edit_does_not_change_text() {
    // Space outside a text box is a pan key; it must never type into the course.
    let mut h = TestHarness::new(hello());
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.object = Some("btn-start".into());
    prep(&mut h);
    let before = h.editor.session.course.clone();
    h.frame(vec![Event::Key {
        key: egui::Key::Space,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    h.frame(vec![Event::Key {
        key: egui::Key::Space,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    h.idle_frame();
    assert_eq!(
        h.editor.session.course.to_json().expect("json"),
        before.to_json().expect("json"),
        "a bare Space press must not edit the course"
    );
}
