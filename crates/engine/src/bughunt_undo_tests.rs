//! Bug hunt (2026-11-20): engine-level undo and hostile input.

use serde_json::json;

use crate::Session;

fn hello() -> Session {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    let course = coursecraft_course::Course::from_json(&json).expect("valid course");
    Session::new(course)
}

fn add_rect(s: &mut Session, frame: serde_json::Value) -> Result<serde_json::Value, String> {
    s.execute(
        "object.add",
        &json!({
            "slide": "s-intro",
            "kind": { "type": "shape", "shape": "rectangle", "fill": "#D9D9D9" },
            "frame": frame,
        }),
    )
    .map_err(|e| e.to_string())
}

/// A command that fails must leave the course and the undo history as they were.
#[test]
fn failed_command_adds_no_undo_step() {
    let mut s = hello();
    let course = s.course.clone();
    let steps = s.undo_len();
    let result = s.execute(
        "object.delete",
        &json!({ "slide": "s-intro", "object": "no-such-object" }),
    );
    assert!(result.is_err(), "deleting a missing object is an error");
    assert_eq!(s.course, course, "the course is unchanged");
    assert_eq!(s.undo_len(), steps, "no undo step was recorded");
}

/// Undo with an empty history is an error, not a silent change.
#[test]
fn undo_with_empty_history_is_an_error_and_changes_nothing() {
    let mut s = hello();
    let course = s.course.clone();
    assert!(s.execute("edit.undo", &json!({})).is_err());
    assert_eq!(s.course, course);
}

/// One add, one undo: the course is back to the original exactly.
#[test]
fn add_then_undo_restores_the_course_exactly() {
    let mut s = hello();
    let course = s.course.clone();
    add_rect(
        &mut s,
        json!({ "x": 10.0, "y": 10.0, "w": 100.0, "h": 50.0 }),
    )
    .expect("add");
    assert_ne!(s.course, course);
    s.undo().expect("undo");
    assert_eq!(s.course, course);
    s.redo().expect("redo");
    assert_ne!(s.course, course, "redo reapplies the add");
}

/// Input-derived numbers are hostile (AGENTS.md golden rule 1). Huge and negative
/// frames must not panic, and the course must still serialise and validate.
#[test]
fn huge_frame_numbers_do_not_panic_and_the_course_still_validates() {
    for frame in [
        json!({ "x": 1e38, "y": 1e38, "w": 1e38, "h": 1e38 }),
        json!({ "x": -1e38, "y": -1e38, "w": 1e30, "h": 1e30 }),
        json!({ "x": 0.0, "y": 0.0, "w": -50.0, "h": -50.0 }),
    ] {
        let mut s = hello();
        let result = add_rect(&mut s, frame.clone());
        let _ = s.course.validate();
        let json = serde_json::to_string(&s.course);
        assert!(json.is_ok(), "frame {frame} made the course unserialisable");
        if result.is_ok() {
            s.undo().expect("undo");
        }
    }
}
