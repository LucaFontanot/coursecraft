//! Bug hunt (2026-11-20, M24, second pass): States preset buttons, the History limit
//! note, and the Find & Replace status after Undo.

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

fn hover_states(h: &TestHarness) -> usize {
    h.editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object("btn-start"))
        .map(|(o, _)| o.states.iter().filter(|s| s.name == "hover").count())
        .unwrap_or(0)
}

#[test]
fn plus_hover_clicked_twice_adds_one_state_and_one_undo_step() {
    let mut h = TestHarness::new_with_size(hello(), (1280.0, 1400.0));
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.object = Some("btn-start".into());
    h.editor.ui.bottom_tab = crate::BottomTab::States;
    h.idle_frame();
    h.idle_frame();
    let undo_before = h.editor.session.undo_len();
    let pos = h.find_text("+ Hover").expect("+ Hover");
    h.click_at(pos);
    h.idle_frame();
    let names_after_one: Vec<String> = h
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object("btn-start"))
        .map(|(o, _)| o.states.iter().map(|s| s.name.clone()).collect())
        .unwrap_or_default();
    assert_eq!(
        names_after_one,
        vec!["hover".to_owned()],
        "first click adds only hover"
    );
    // Two more clicks where the button used to be. The row reflows once "+ Hover" is gone.
    h.click_at(pos);
    h.idle_frame();
    h.click_at(pos);
    h.idle_frame();

    let names: Vec<String> = h
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object("btn-start"))
        .map(|(o, _)| o.states.iter().map(|s| s.name.clone()).collect())
        .unwrap_or_default();
    assert_eq!(
        names,
        vec!["hover".to_owned()],
        "extra clicks on the same spot add no other state"
    );
    assert_eq!(
        h.editor.session.undo_len(),
        undo_before + 1,
        "one undo step"
    );

    h.editor.session.undo().expect("undo");
    assert_eq!(hover_states(&h), 0, "undo removes the hover state");
}

#[test]
fn history_note_appears_after_more_than_the_limit_and_stays_after_undo() {
    let mut h = TestHarness::new_with_size(hello(), (1280.0, 900.0));
    let limit = coursecraft_engine::MAX_HISTORY;
    for i in 0..(limit + 5) {
        h.editor.run(
            "course.set_title",
            serde_json::json!({ "title": format!("Title {i}") }),
        );
    }
    assert!(h.editor.session.history_truncated());
    assert_eq!(h.editor.session.undo_len(), limit);

    h.editor.ui.show_history = true;
    h.idle_frame();
    h.idle_frame();
    let note = format!("Older steps were removed (CourseCraft keeps the last {limit}).");
    assert!(h.find_text(&note).is_some(), "history note shown");

    for _ in 0..10 {
        h.editor.session.undo().expect("undo");
    }
    h.idle_frame();
    assert!(
        h.find_text(&note).is_some(),
        "note stays: the dropped steps are still gone"
    );
}

#[test]
fn find_replace_status_clears_after_undo() {
    let mut h = TestHarness::new_with_size(hello(), (1280.0, 900.0));
    h.editor.run(
        "course.set_title",
        serde_json::json!({ "title": "Before text" }),
    );
    h.editor.ui.find.open = true;
    // The status message is tied to the course revision it was set at.
    h.editor.ui.find.message = Some("Replaced 1.".to_owned());
    h.editor.ui.find.message_revision = Some(h.editor.session.revision);
    h.idle_frame();
    h.editor.session.undo().expect("undo");
    h.idle_frame();
    assert!(
        h.editor.ui.find.message.is_none(),
        "the Replaced status clears after Undo"
    );
}
