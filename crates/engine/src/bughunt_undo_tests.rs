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

fn assert_hostile_frame_parity(frame: serde_json::Value) {
    let mut s = hello();
    let before_json = s.course.to_json().expect("before json");
    let before_rev = s.revision;
    match add_rect(&mut s, frame.clone()) {
        Err(_) => {
            assert_eq!(
                s.course.to_json().expect("rollback json"),
                before_json,
                "failed hostile frame {frame} must roll back byte-identically"
            );
            assert!(s.undo_names().is_empty());
            assert!(s.redo_names().is_empty());
            assert_eq!(s.revision, before_rev);
        }
        Ok(_) => {
            let after_json = s.course.to_json().expect("after json");
            let after_undo_names: Vec<String> =
                s.undo_names().into_iter().map(str::to_owned).collect();
            let expected_names: Vec<&str> = after_undo_names.iter().map(String::as_str).collect();
            assert_eq!(after_undo_names.len(), 1);
            assert!(s.redo_names().is_empty());
            assert_eq!(s.revision, before_rev + 1);

            s.undo().expect("undo hostile frame");
            assert_eq!(
                s.course.to_json().expect("undone json"),
                before_json,
                "undo after hostile frame {frame} must restore byte-identical JSON"
            );
            assert!(s.undo_names().is_empty());
            assert_eq!(s.redo_names(), expected_names);
            assert_eq!(s.revision, before_rev + 2);

            s.redo().expect("redo hostile frame");
            assert_eq!(
                s.course.to_json().expect("redone json"),
                after_json,
                "redo after hostile frame {frame} must restore byte-identical JSON"
            );
            assert_eq!(s.undo_names(), expected_names);
            assert!(s.redo_names().is_empty());
            assert_eq!(s.revision, before_rev + 3);
        }
    }
}

/// DF-04: Byte-identical JSON, undo/redo name lists, and revision parity on
/// failed commands, empty undo/redo, and hostile huge/negative frames.
#[test]
fn df_04_failed_and_hostile_commands_byte_identical_parity() {
    let mut s = hello();
    let baseline_json = s.course.to_json().expect("baseline json");
    let baseline_rev = s.revision;

    // Empty undo and redo fail without altering course JSON, history names, or revision.
    assert_eq!(s.undo(), Err(crate::EngineError::NothingTo("undo")));
    assert_eq!(s.redo(), Err(crate::EngineError::NothingTo("redo")));
    assert_eq!(
        s.course.to_json().expect("after empty undo/redo"),
        baseline_json
    );
    assert!(s.undo_names().is_empty());
    assert!(s.redo_names().is_empty());
    assert_eq!(s.revision, baseline_rev);

    // Failed mutating commands roll back byte-identically with no history or revision change.
    assert!(
        s.execute(
            "object.delete",
            &json!({ "slide": "s-intro", "object": "missing-id" })
        )
        .is_err()
    );
    assert!(
        s.execute(
            "trigger.add",
            &json!({
                "slide": "s-intro",
                "trigger": {
                    "action": { "action": "show_layer", "layer": "ghost-layer" },
                    "when": { "event": "submit" }
                }
            })
        )
        .is_err()
    );
    assert_eq!(
        s.course.to_json().expect("after failed cmds"),
        baseline_json
    );
    assert!(s.undo_names().is_empty());
    assert!(s.redo_names().is_empty());
    assert_eq!(s.revision, baseline_rev);

    // Hostile huge/negative frames: either cleanly rejected with byte-identical rollback,
    // or accepted with byte-identical undo/redo roundtrips.
    for frame in [
        json!({ "x": 1e38, "y": 1e38, "w": 1e38, "h": 1e38 }),
        json!({ "x": -1e38, "y": -1e38, "w": 1e30, "h": 1e30 }),
        json!({ "x": 0.0, "y": 0.0, "w": -50.0, "h": -50.0 }),
        json!({ "x": -1000.0, "y": -1000.0, "w": 200.0, "h": 100.0 }),
    ] {
        assert_hostile_frame_parity(frame);
    }
}
