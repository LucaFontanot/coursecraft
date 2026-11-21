//! Headless tests for A10: Question editor reachable in the right side panel.

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
fn right_panel_shows_question_editor_for_question_slide_and_edits_prompt() {
    let mut harness = TestHarness::new(hello());
    harness.editor.selection.slide = "s-quiz".into();
    harness.editor.selection.object = None;
    harness.idle_frame();
    harness.idle_frame();

    let panel = harness.right_panel_rect();
    assert!(
        harness.find_text_in("Question", panel).is_some(),
        "right panel should show a Question section on a question slide"
    );
    let prompt_pos = harness
        .find_text_in(
            "You see a spill in the corridor. What do you do first?",
            panel,
        )
        .expect("question prompt field should be visible in the right panel");
    let click_away = harness
        .find_text_in("Question", panel)
        .expect("Question header");
    harness.set_multiline_field(prompt_pos, "Updated spill prompt", click_away);

    let prompt = harness
        .editor
        .session
        .course
        .slide("s-quiz")
        .and_then(|s| s.question.as_ref())
        .map(|q| q.prompt.as_str())
        .expect("question on s-quiz");
    assert_eq!(prompt, "Updated spill prompt");
}

#[test]
fn right_panel_shows_question_editor_for_question_object_and_hides_otherwise() {
    let mut harness = TestHarness::new(hello());

    // Non-question slide: no Question section in the right panel.
    harness.editor.selection.slide = "s-intro".into();
    harness.editor.selection.object = None;
    harness.idle_frame();
    harness.idle_frame();
    assert!(
        harness
            .find_text_in("Question", harness.right_panel_rect())
            .is_none(),
        "non-question slide should not show Question section"
    );

    // Non-question object on non-question slide: no Question section.
    harness.editor.selection.object = Some("btn-start".into());
    harness.idle_frame();
    harness.idle_frame();
    assert!(
        harness
            .find_text_in("Question", harness.right_panel_rect())
            .is_none(),
        "non-question object should not show Question section"
    );

    // Question-related object (btn-submit has SubmitQuestion trigger): Format tab shows Question section.
    harness.editor.selection.slide = "s-quiz".into();
    harness.editor.selection.object = Some("btn-submit".into());
    harness.idle_frame();
    harness.idle_frame();

    let panel = harness.right_panel_rect();
    for _ in 0..8 {
        if harness.find_text_in("Question", panel).is_some()
            && harness
                .find_text_in(
                    "You see a spill in the corridor. What do you do first?",
                    panel,
                )
                .is_some()
        {
            break;
        }
        harness.scroll_over(panel.center(), -250.0);
        harness.idle_frame();
    }
    assert!(
        harness.find_text_in("Question", panel).is_some(),
        "question-related object should show Question section in Format tab"
    );
    let prompt_pos = harness
        .find_text_in(
            "You see a spill in the corridor. What do you do first?",
            panel,
        )
        .expect("question prompt field should be visible in Format tab");
    let click_away = harness
        .find_text_in("Question", panel)
        .expect("Question header");
    harness.set_multiline_field(prompt_pos, "Edited from submit button", click_away);

    let prompt = harness
        .editor
        .session
        .course
        .slide("s-quiz")
        .and_then(|s| s.question.as_ref())
        .map(|q| q.prompt.as_str())
        .expect("question on s-quiz");
    assert_eq!(prompt, "Edited from submit button");
}

#[test]
fn bottom_question_tab_and_right_panel_both_work_together_and_undo_redo() {
    let mut harness = TestHarness::new(hello());
    harness.editor.selection.slide = "s-quiz".into();
    harness.editor.selection.object = None;
    harness.editor.ui.bottom_tab = crate::BottomTab::Question;
    harness.idle_frame();
    harness.idle_frame();

    // Edit in the right panel while the bottom Question tab is also open.
    let right_panel = harness.right_panel_rect();
    let right_prompt = harness
        .find_text_in(
            "You see a spill in the corridor. What do you do first?",
            right_panel,
        )
        .expect("prompt in right panel");
    let right_away = harness
        .find_text_in("Question", right_panel)
        .expect("Question header in right panel");
    harness.set_multiline_field(right_prompt, "Edited in right panel", right_away);

    assert_eq!(
        harness
            .editor
            .session
            .course
            .slide("s-quiz")
            .and_then(|s| s.question.as_ref())
            .map(|q| q.prompt.as_str()),
        Some("Edited in right panel")
    );

    // Edit in the bottom Question tab while the right panel is also open.
    let bottom_top = harness.bottom_panel_top();
    let bottom_rect = egui::Rect::from_min_max(
        egui::pos2(0.0, bottom_top),
        egui::pos2(right_panel.min.x, 900.0),
    );
    let bottom_prompt = harness
        .find_text_in("Edited in right panel", bottom_rect)
        .expect("prompt in bottom Question tab");
    let bottom_away = harness
        .find_text_in("Prompt", bottom_rect)
        .expect("Prompt label in bottom Question tab");
    harness.set_multiline_field(bottom_prompt, "Edited in bottom tab", bottom_away);

    assert_eq!(
        harness
            .editor
            .session
            .course
            .slide("s-quiz")
            .and_then(|s| s.question.as_ref())
            .map(|q| q.prompt.as_str()),
        Some("Edited in bottom tab")
    );

    // Undo and redo update both panels.
    harness.editor.session.undo().expect("undo");
    harness.idle_frame();
    harness.idle_frame();
    assert_eq!(
        harness
            .editor
            .session
            .course
            .slide("s-quiz")
            .and_then(|s| s.question.as_ref())
            .map(|q| q.prompt.as_str()),
        Some("Edited in right panel")
    );
    assert!(
        harness
            .find_text_in("Edited in right panel", right_panel)
            .is_some(),
        "right panel should reflect undone prompt"
    );

    harness.editor.session.redo().expect("redo");
    harness.idle_frame();
    harness.idle_frame();
    assert_eq!(
        harness
            .editor
            .session
            .course
            .slide("s-quiz")
            .and_then(|s| s.question.as_ref())
            .map(|q| q.prompt.as_str()),
        Some("Edited in bottom tab")
    );
}
