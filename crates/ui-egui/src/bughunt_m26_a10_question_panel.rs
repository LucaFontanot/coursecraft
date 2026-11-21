//! M26 bug-hunt tests for A10: Question editor in right panel.
//!
//! Ordered P1 -> P2 -> P3.

use crate::test_harness::TestHarness;
use coursecraft_course::{Course, QuestionKind};
use egui::{pos2, vec2};

fn hello() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    Course::from_json(&json).expect("valid course")
}

fn png_bytes() -> Vec<u8> {
    coursecraft_media::encode_png_rgba(16, 16, &vec![180u8; 16 * 16 * 4]).expect("png")
}

// ============================================================================
// P1
// ============================================================================

#[test]
#[ignore = "BUG: Right panel Question editor clips/overlaps widgets in narrow right panel width on question kinds"]
fn a10_1_every_question_kind_renders_cleanly_in_right_panel() {
    let kinds = [
        "multiple_choice",
        "multiple_response",
        "true_false",
        "fill_in",
        "matching",
        "sequence",
        "hotspot",
        "drag_drop",
    ];
    for (w, h_px) in [(1280.0, 900.0), (1024.0, 720.0)] {
        for kind in kinds {
            let mut h = TestHarness::new_with_size(hello(), (w, h_px));
            h.editor.import_image("map.png", &png_bytes());
            let sid = h.editor.session.course.scenes[0].slides[0].id.clone();
            let mut params = serde_json::json!({ "slide": sid, "kind": kind });
            if kind == "hotspot" {
                params["hotspot_image"] = serde_json::json!("map.png");
            }
            let _ = h.editor.session.execute("question.insert", &params);
            h.editor.selection.slide = sid;
            h.editor.selection.object = None;
            h.idle_frame();
            h.idle_frame();

            let panel = h.right_panel_rect();
            let mut findings = Vec::new();
            for _ in 0..8 {
                let rep = h.layout_report(2);
                for f in rep.findings {
                    if f.starts_with("clipped-widget") || f.starts_with("overlap") {
                        findings.push(format!("{kind}@{w}x{h_px}: {f}"));
                    }
                }
                h.scroll_over(panel.center(), -200.0);
            }
            assert!(findings.is_empty(), "A10.1 findings: {findings:?}");
        }
    }
}

#[test]
fn a10_2_editing_choices_correct_add_remove_points_from_right_panel() {
    let mut h = TestHarness::new(hello());
    h.editor.selection.slide = "s-quiz".into();
    h.editor.selection.object = None;
    h.idle_frame();
    h.idle_frame();

    let panel = h.right_panel_rect();
    let before_len = h
        .editor
        .session
        .course
        .slide("s-quiz")
        .unwrap()
        .question
        .as_ref()
        .unwrap()
        .choices
        .len();

    let mut added = false;
    for _ in 0..8 {
        if let Some(add_btn) = h.find_text_in("+ Add choice", panel) {
            h.click_at(add_btn);
            h.idle_frame();
            added = true;
            break;
        }
        h.scroll_over(panel.center(), -200.0);
    }
    assert!(
        added,
        "Add choice button should be reachable in right panel"
    );
    let after_len = h
        .editor
        .session
        .course
        .slide("s-quiz")
        .unwrap()
        .question
        .as_ref()
        .unwrap()
        .choices
        .len();
    assert_eq!(after_len, before_len + 1);

    assert!(h.editor.session.undo().is_ok());
    let undone_len = h
        .editor
        .session
        .course
        .slide("s-quiz")
        .unwrap()
        .question
        .as_ref()
        .unwrap()
        .choices
        .len();
    assert_eq!(undone_len, before_len);
}

#[test]
fn a10_3_changing_question_kind_from_right_panel() {
    let mut h = TestHarness::new(hello());
    h.editor.selection.slide = "s-quiz".into();
    h.editor.selection.object = None;
    h.idle_frame();
    h.idle_frame();

    h.pick_combo("Multiple Choice", "True / False");
    assert_eq!(
        h.editor
            .session
            .course
            .slide("s-quiz")
            .unwrap()
            .question
            .as_ref()
            .unwrap()
            .kind,
        QuestionKind::TrueFalse
    );

    h.pick_combo("True / False", "Fill in");
    assert_eq!(
        h.editor
            .session
            .course
            .slide("s-quiz")
            .unwrap()
            .question
            .as_ref()
            .unwrap()
            .kind,
        QuestionKind::FillIn
    );

    assert!(h.editor.session.undo().is_ok());
    assert!(h.editor.session.undo().is_ok());
    assert_eq!(
        h.editor
            .session
            .course
            .slide("s-quiz")
            .unwrap()
            .question
            .as_ref()
            .unwrap()
            .kind,
        QuestionKind::MultipleChoice
    );
}

#[test]
#[ignore = "BUG: Selecting a Multiple Choice answer/prompt object on a question slide does not expose Question section in right panel (is_question_object only checks Submit/Hotspot/DragDrop)"]
fn a10_4_selecting_every_kind_of_question_object_shows_question_section() {
    let mut h = TestHarness::new(hello());
    h.editor.selection.slide = "s-quiz".into();
    // Select submit button -> shows Question
    h.editor.selection.object = Some("btn-submit".into());
    h.idle_frame();
    h.idle_frame();
    let panel = h.right_panel_rect();
    let mut found_submit = false;
    for _ in 0..8 {
        if h.find_text_in("Question", panel).is_some() {
            found_submit = true;
            break;
        }
        h.scroll_over(panel.center(), -200.0);
    }
    assert!(found_submit, "Submit button should show Question section");

    // Select a choice/prompt object on s-quiz -> should also show Question section
    let non_submit_id = h
        .editor
        .session
        .course
        .slide("s-quiz")
        .unwrap()
        .objects
        .iter()
        .find(|o| o.id != "btn-submit")
        .map(|o| o.id.clone())
        .expect("non-submit object on s-quiz");
    h.editor.selection.object = Some(non_submit_id);
    h.idle_frame();
    h.idle_frame();
    let mut found_choice = false;
    for _ in 0..8 {
        if h.find_text_in("Question", panel).is_some() {
            found_choice = true;
            break;
        }
        h.scroll_over(panel.center(), -200.0);
    }
    assert!(
        found_choice,
        "Question prompt/choice object should also show Question section"
    );
}

#[test]
fn a10_7_both_bottom_question_tab_and_right_panel_open_simultaneously() {
    let mut h = TestHarness::new(hello());
    h.editor.selection.slide = "s-quiz".into();
    h.editor.selection.object = None;
    h.editor.ui.bottom_tab = crate::BottomTab::Question;
    h.idle_frame();
    h.idle_frame();

    let rev = h.editor.session.revision;
    h.pick_combo("Multiple Choice", "True / False");
    assert_eq!(
        h.editor.session.revision,
        rev + 1,
        "combo change with both panels open should execute once"
    );
    assert_eq!(
        h.editor
            .session
            .course
            .slide("s-quiz")
            .unwrap()
            .question
            .as_ref()
            .unwrap()
            .kind,
        QuestionKind::TrueFalse
    );
}

// ============================================================================
// P2
// ============================================================================

#[test]
fn a10_5_question_slide_while_editing_feedback_layer() {
    let mut h = TestHarness::new(hello());
    h.editor.selection.slide = "s-quiz".into();
    h.idle_frame();
    h.idle_frame();
    let canvas = h.central_panel_rect();
    if let Some(pill) = h.find_text_in("Correct", canvas) {
        h.click_at(pill);
        h.idle_frame();
        let rep = h.layout_report(2);
        let bad: Vec<_> = rep
            .findings
            .into_iter()
            .filter(|f| f.starts_with("clipped-widget") || f.starts_with("overlap"))
            .collect();
        assert!(bad.is_empty(), "feedback layer findings: {bad:?}");
        let base = h.find_text_in("Base", canvas).expect("Base pill");
        h.click_at(base);
        h.idle_frame();
    }
    assert!(h.find_text_in("Question", h.right_panel_rect()).is_some());
}

#[test]
fn a10_6_question_bank_slide_vs_draw_slide_in_right_panel() {
    let mut h = TestHarness::new(hello());
    let bank = h
        .editor
        .session
        .execute("bank.create", &serde_json::json!({ "title": "Bank 1" }))
        .expect("bank.create");
    let bank_id = bank["id"].as_str().unwrap().to_owned();
    let qslide = h
        .editor
        .session
        .execute(
            "bank.add_question",
            &serde_json::json!({ "bank": bank_id, "kind": "multiple_choice" }),
        )
        .expect("bank.add_question");
    let qid = qslide["id"].as_str().unwrap().to_owned();
    let draw = h
        .editor
        .session
        .execute(
            "draw.insert",
            &serde_json::json!({ "scene": h.editor.session.course.scenes[0].id, "bank": bank_id, "count": 1 }),
        )
        .expect("draw.insert");
    let did = draw["id"].as_str().unwrap().to_owned();

    h.editor.selection.slide = qid;
    h.editor.selection.object = None;
    h.idle_frame();
    h.idle_frame();
    assert!(h.find_text_in("Question", h.right_panel_rect()).is_some());

    h.editor.selection.slide = did;
    h.editor.selection.object = None;
    h.idle_frame();
    h.idle_frame();
    assert!(h.find_text_in("Draw", h.right_panel_rect()).is_some());
    assert!(h.find_text_in("Question", h.right_panel_rect()).is_none());
}

#[test]
fn a10_8_deleting_and_inserting_question_updates_right_panel_immediately() {
    let mut h = TestHarness::new(hello());
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.object = None;
    h.idle_frame();
    h.idle_frame();
    assert!(h.find_text_in("Question", h.right_panel_rect()).is_none());

    h.editor
        .session
        .execute(
            "question.insert",
            &serde_json::json!({ "slide": "s-intro", "kind": "true_false" }),
        )
        .expect("question.insert");
    h.idle_frame();
    assert!(h.find_text_in("Question", h.right_panel_rect()).is_some());

    h.editor
        .session
        .execute(
            "question.delete",
            &serde_json::json!({ "slide": "s-intro" }),
        )
        .expect("question.delete");
    h.idle_frame();
    assert!(h.find_text_in("Question", h.right_panel_rect()).is_none());

    assert!(h.editor.session.undo().is_ok());
    h.idle_frame();
    assert!(h.find_text_in("Question", h.right_panel_rect()).is_some());
}

#[test]
fn a10_10_switching_directly_between_two_question_slides_no_bleed() {
    let course = hello();
    let s2_id = course.scenes[0].slides[1].id.clone();
    let s2_title = course.scenes[0].slides[1].title.clone();
    let mut sess = coursecraft_engine::Session::new(course);
    sess.execute(
        "question.insert",
        &serde_json::json!({ "slide": s2_id, "kind": "fill_in" }),
    )
    .expect("q2");
    let mut q2 = sess.course.slide(&s2_id).unwrap().question.clone().unwrap();
    q2.prompt = "Question Two?".into();
    sess.execute(
        "question.set",
        &serde_json::json!({ "slide": s2_id, "question": q2 }),
    )
    .expect("q2 prompt");
    let mut h = TestHarness::new(sess.course);

    h.editor.selection.slide = "s-quiz".into();
    h.editor.selection.object = None;
    h.idle_frame();
    h.idle_frame();

    let panel = h.right_panel_rect();
    let prompt_pos = h
        .find_text_in(
            "You see a spill in the corridor. What do you do first?",
            panel,
        )
        .expect("s-quiz prompt");
    h.click_at(prompt_pos);
    h.idle_frame();
    h.press(egui::Key::A, egui::Modifiers::COMMAND);
    h.type_text("Draft 1");

    // Immediately click Slide 2 in left panel without clicking away
    let s2_row = h
        .find_text_in(&s2_title, h.left_panel_rect())
        .expect("Slide 2");
    h.click_at(s2_row);
    h.idle_frame();
    h.idle_frame();

    let q2_prompt = h
        .editor
        .session
        .course
        .slide(&s2_id)
        .unwrap()
        .question
        .as_ref()
        .unwrap()
        .prompt
        .clone();
    assert_eq!(
        q2_prompt, "Question Two?",
        "Slide 1 draft bled into Slide 2!"
    );
}

// ============================================================================
// P3
// ============================================================================

#[test]
fn a10_9_multi_select_containing_question_object() {
    let mut h = TestHarness::new(hello());
    h.editor
        .session
        .execute(
            "object.add",
            &serde_json::json!({
                "slide": "s-quiz",
                "id": "extra-note",
                "kind": { "type": "text", "text": "Note" },
                "frame": { "x": 40.0, "y": 40.0, "w": 120.0, "h": 40.0 }
            }),
        )
        .expect("add extra");
    h.editor.selection.slide = "s-quiz".into();
    h.editor.selection.set_many(["btn-submit", "extra-note"]);
    h.idle_frame();
    let _ = h.layout_report(2);

    h.editor.selection.set_many(["btn-submit"]);
    h.idle_frame();
    let panel = h.right_panel_rect();
    let mut found = false;
    for _ in 0..8 {
        if h.find_text_in("Question", panel).is_some() {
            found = true;
            break;
        }
        h.scroll_over(panel.center(), -200.0);
    }
    assert!(found);
    let _ = (pos2(0.0, 0.0), vec2(0.0, 0.0));
}
