//! Headless UI test for the Question panel's "Add Try Again layer" button.

use crate::BottomTab;
use crate::test_harness::TestHarness;
use coursecraft_course::Course;

fn course_with_question(attempts: u32) -> Course {
    let mut session = coursecraft_engine::Session::blank("Try Again UI");
    session
        .execute("question.insert", &serde_json::json!({ "slide": "slide1" }))
        .expect("insert question");
    if let Some(q) = session
        .course
        .slide_mut("slide1")
        .and_then(|s| s.question.as_mut())
    {
        q.attempts = attempts;
    }
    session.course
}

fn open_tall_question_tab(harness: &mut TestHarness) {
    harness.editor.ui.bottom_tab = BottomTab::Question;
    harness.idle_frame();
    harness.idle_frame();
    if let Some(rect) = harness.editor.ui.timeline.panel_rect {
        harness.drag_by(
            egui::pos2(rect.center().x, rect.min.y + 2.0),
            egui::vec2(0.0, -220.0),
            4,
            egui::Modifiers::NONE,
        );
        harness.idle_frame();
        harness.idle_frame();
    }
}

#[test]
fn add_try_again_layer_button_appears_for_multiple_attempts_and_creates_layer() {
    // With 1 attempt, the button is hidden.
    let mut single = TestHarness::new_with_size(course_with_question(1), (1600.0, 1000.0));
    open_tall_question_tab(&mut single);
    assert!(
        single.find_text("Add Try Again layer").is_none(),
        "button must not appear when attempts == 1"
    );

    // With 2 attempts, the button appears and clicking it creates and wires the layer.
    let mut harness = TestHarness::new_with_size(course_with_question(2), (1600.0, 1000.0));
    open_tall_question_tab(&mut harness);

    let btn = harness
        .find_text("Add Try Again layer")
        .expect("Add Try Again layer button should be visible when attempts != 1");
    harness.click_at(btn);
    harness.idle_frame();
    harness.idle_frame();

    let slide = harness
        .editor
        .session
        .course
        .slide("slide1")
        .expect("slide1");
    let try_again_id = slide
        .question
        .as_ref()
        .and_then(|q| q.try_again_layer.as_deref())
        .expect("question.try_again_layer should be set");
    assert!(
        slide.layer(try_again_id).is_some(),
        "Try Again layer should exist on the slide"
    );
    assert!(
        harness.find_text("Add Try Again layer").is_none(),
        "button hides once try_again_layer is set"
    );
}
