//! Headless tests for the History panel.

use crate::test_harness::TestHarness;
use coursecraft_course::Course;

fn hello() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    Course::from_json(&json).expect("valid course")
}

#[test]
fn history_panel_jumps_to_undo_and_redo_steps_and_passes_layout() {
    let mut harness = TestHarness::new_with_size(hello(), (1100.0, 700.0));
    harness.idle_frame();

    let original = harness.editor.session.course.clone();
    let first_scene = original.scenes[0].id.clone();

    assert!(
        harness
            .editor
            .run(
                "course.update",
                serde_json::json!({ "title": "Renamed Course" }),
            )
            .is_some()
    );
    assert!(
        harness
            .editor
            .run("scene.add", serde_json::json!({ "title": "Extra Scene" }),)
            .is_some()
    );
    assert!(
        harness
            .editor
            .run(
                "slide.add",
                serde_json::json!({ "scene": first_scene, "title": "Extra Slide" }),
            )
            .is_some()
    );

    let after_three = harness.editor.session.course.clone();
    assert_ne!(after_three, original);
    assert_eq!(
        harness.editor.session.undo_names(),
        vec!["New slide", "New scene", "Course settings"]
    );

    // Open via Edit ▸ History…
    let edit_menu = harness.find_text("Edit").expect("Edit menu");
    harness.click_at(edit_menu);
    harness.idle_frame();
    let history_item = harness.find_text("History…").expect("History… menu item");
    harness.click_at(history_item);
    harness.idle_frame();
    harness.idle_frame();

    assert!(harness.editor.ui.show_history);
    assert!(harness.find_text("Now").is_some());
    assert!(harness.find_text("Opened").is_some());
    assert!(harness.find_text("New slide").is_some());
    assert!(harness.find_text("New scene").is_some());
    assert!(harness.find_text("Course settings").is_some());

    // Layout check at 1100×700: no clipped or overlapping text.
    let report = harness.layout_report(2);
    let bad: Vec<&String> = report
        .findings
        .iter()
        .filter(|f| f.contains("clipped") || f.contains("overlap"))
        .collect();
    assert!(bad.is_empty(), "unexpected layout findings: {bad:#?}");

    // Click the oldest undo row ("Course settings") -> jumps back to original.
    let oldest = harness
        .find_text_last("Course settings")
        .expect("oldest undo row");
    harness.click_at(oldest);
    harness.idle_frame();
    harness.idle_frame();

    assert_eq!(harness.editor.session.course, original);
    assert!(harness.editor.session.undo_names().is_empty());
    assert_eq!(
        harness.editor.session.redo_names(),
        vec!["Course settings", "New scene", "New slide"]
    );
    assert!(harness.find_text("New slide").is_some());

    // Click the topmost redo row ("New slide") -> course comes back to after_three.
    let redo_top = harness
        .find_text_last("New slide")
        .expect("topmost redo row");
    harness.click_at(redo_top);
    harness.idle_frame();
    harness.idle_frame();

    assert_eq!(harness.editor.session.course, after_three);
    assert_eq!(
        harness.editor.session.undo_names(),
        vec!["New slide", "New scene", "Course settings"]
    );
    assert!(harness.editor.session.redo_names().is_empty());
}
