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

#[test]
fn narrow_panel_stacks_choice_buttons_and_keeps_wide_panel_single_row() {
    use coursecraft_render::ThemeMode;
    use egui::{Context, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, pos2, vec2};

    const LONG_CHOICE: &str = "Stand up, stretch and look at something far away for a minute";

    fn course_with_long_choice() -> Course {
        let mut course = hello();
        let quiz = course
            .slide_mut("s-quiz")
            .and_then(|s| s.question.as_mut())
            .expect("s-quiz question");
        assert!(
            quiz.choices.len() >= 3,
            "hello course s-quiz should have at least 3 choices"
        );
        quiz.choices[0].text = LONG_CHOICE.to_owned();
        course
    }

    struct PanelRunner {
        ctx: Context,
        editor: crate::Editor,
        screen: Rect,
        time: f64,
        as_tab: bool,
    }

    impl PanelRunner {
        fn new(width: f32, height: f32, as_tab: bool) -> Self {
            let ctx = Context::default();
            ctx.enable_accesskit();
            coursecraft_render::apply_style(&ctx, ThemeMode::Light);
            let mut style = (*ctx.global_style()).clone();
            style.animation_time = 0.0;
            ctx.set_global_style(style);

            let mut editor = crate::Editor::new(course_with_long_choice());
            editor.selection.slide = "s-quiz".into();
            editor.selection.object = None;
            Self {
                ctx,
                editor,
                screen: Rect::from_min_size(pos2(0.0, 0.0), vec2(width, height)),
                time: 0.0,
                as_tab,
            }
        }

        fn frame(&mut self, events: Vec<Event>) -> Option<egui::accesskit::TreeUpdate> {
            let time = self.time;
            self.time += 1.0 / 60.0;
            let raw = RawInput {
                screen_rect: Some(self.screen),
                time: Some(time),
                events,
                ..Default::default()
            };
            let mut actions = Vec::new();
            let output = self.ctx.run_ui(raw, |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    if self.as_tab {
                        crate::question::show_question_tab(&mut self.editor, ui, &mut actions);
                    } else {
                        crate::question::show_question_section(&mut self.editor, ui, &mut actions);
                    }
                });
            });
            for (cmd, params) in actions {
                let _ = self.editor.run(cmd, params);
            }
            let update = output.platform_output.accesskit_update.clone();
            output.drop_without_applying_deltas();
            update
        }

        fn click(&mut self, pos: Pos2) {
            self.frame(vec![Event::PointerMoved(pos)]);
            self.frame(vec![
                Event::PointerMoved(pos),
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::NONE,
                },
            ]);
            self.frame(vec![
                Event::PointerMoved(pos),
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: false,
                    modifiers: Modifiers::NONE,
                },
            ]);
            self.frame(Vec::new());
        }
    }

    fn choice_text_rect(update: &egui::accesskit::TreeUpdate, text: &str) -> Rect {
        use egui::accesskit::Role;
        update
            .nodes
            .iter()
            .find_map(|(_, node)| {
                let is_input = matches!(node.role(), Role::TextInput | Role::MultilineTextInput);
                if is_input && node.value() == Some(text) {
                    let b = node.bounds()?;
                    Some(Rect::from_min_max(
                        pos2(b.x0 as f32, b.y0 as f32),
                        pos2(b.x1 as f32, b.y1 as f32),
                    ))
                } else {
                    None
                }
            })
            .expect("choice text input node")
    }

    fn buttons_named(update: &egui::accesskit::TreeUpdate, name: &str) -> Vec<(Rect, bool)> {
        let mut out: Vec<(Rect, bool)> = crate::test_harness::extract_widget_nodes(Some(update))
            .into_iter()
            .filter(|n| n.role == egui::accesskit::Role::Button && n.label.as_deref() == Some(name))
            .map(|n| (n.bounds, n.disabled))
            .collect();
        out.sort_by(|a, b| a.0.min.y.total_cmp(&b.0.min.y));
        out
    }

    // 1. Narrow (~260 pt) panel: choice text field is >= 180 pt wide, controls stack below
    // indented under the text field, and Move up / Move down / Remove choice are clickable.
    let mut narrow = PanelRunner::new(260.0, 900.0, false);
    narrow.frame(Vec::new());
    let update = narrow.frame(Vec::new()).expect("accesskit update");

    let text_rect = choice_text_rect(&update, LONG_CHOICE);
    assert!(
        text_rect.width() >= 180.0,
        "choice text field in 260 pt panel must be >= 180 pt wide, got {}",
        text_rect.width()
    );

    let ups = buttons_named(&update, "Move up");
    let downs = buttons_named(&update, "Move down");
    let removes = buttons_named(&update, "Remove choice");
    assert_eq!(ups.len(), 3, "Move up button for every choice");
    assert_eq!(downs.len(), 3, "Move down button for every choice");
    assert_eq!(removes.len(), 3, "Remove choice button for every choice");

    let (up0, up0_disabled) = ups[0];
    let (down0, down0_disabled) = downs[0];
    let (remove0, remove0_disabled) = removes[0];
    assert!(up0_disabled, "first choice Move up is disabled");
    assert!(!down0_disabled, "first choice Move down is enabled");
    assert!(!remove0_disabled, "Remove choice is enabled with 3 choices");

    for (label, rect) in [
        ("Move up", up0),
        ("Move down", down0),
        ("Remove choice", remove0),
    ] {
        assert!(
            rect.min.y >= text_rect.max.y - 1.0,
            "{label} should sit on the second row below choice text ({rect:?} vs {text_rect:?})"
        );
    }
    assert!(
        (up0.min.x - text_rect.min.x).abs() <= 4.0,
        "stacked buttons should be indented under the text field ({up0:?} vs {text_rect:?})"
    );

    // Click Move down on choice 0 -> long choice moves to index 1.
    narrow.click(down0.center());
    let texts = |r: &PanelRunner| -> Vec<String> {
        r.editor
            .session
            .course
            .slide("s-quiz")
            .and_then(|s| s.question.as_ref())
            .map(|q| q.choices.iter().map(|c| c.text.clone()).collect())
            .unwrap_or_default()
    };
    assert_eq!(
        texts(&narrow)[1],
        LONG_CHOICE,
        "Move down swapped choice 0 to 1"
    );

    // Click Move up on choice 1 -> long choice moves back to index 0.
    let update = narrow
        .frame(Vec::new())
        .expect("accesskit update after move down");
    let (up1, up1_disabled) = buttons_named(&update, "Move up")[1];
    assert!(!up1_disabled, "second choice Move up is enabled");
    narrow.click(up1.center());
    assert_eq!(
        texts(&narrow)[0],
        LONG_CHOICE,
        "Move up swapped choice 1 back to 0"
    );

    // Click Remove choice on choice 0 -> choice count drops from 3 to 2.
    let update = narrow
        .frame(Vec::new())
        .expect("accesskit update after move up");
    let (rem0, rem0_disabled) = buttons_named(&update, "Remove choice")[0];
    assert!(!rem0_disabled);
    narrow.click(rem0.center());
    assert_eq!(texts(&narrow).len(), 2, "Remove choice deleted choice 0");

    // 2. Wide (900 pt) panel: single-row layout preserved exactly as before.
    let mut wide = PanelRunner::new(900.0, 400.0, true);
    wide.frame(Vec::new());
    let wide_update = wide.frame(Vec::new()).expect("wide accesskit update");

    let wide_text = choice_text_rect(&wide_update, LONG_CHOICE);
    let wide_up0 = buttons_named(&wide_update, "Move up")[0].0;
    let wide_down0 = buttons_named(&wide_update, "Move down")[0].0;
    let wide_rem0 = buttons_named(&wide_update, "Remove choice")[0].0;
    for (label, rect) in [
        ("Move up", wide_up0),
        ("Move down", wide_down0),
        ("Remove choice", wide_rem0),
    ] {
        assert!(
            rect.min.x >= wide_text.max.x - 1.0,
            "at 900 pt {label} should sit to the right of choice text ({rect:?} vs {wide_text:?})"
        );
        assert!(
            (rect.center().y - wide_text.center().y).abs() <= 8.0,
            "at 900 pt {label} should sit on the same row as choice text ({rect:?} vs {wide_text:?})"
        );
    }
}
