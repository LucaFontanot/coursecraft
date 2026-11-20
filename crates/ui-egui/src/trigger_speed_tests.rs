//! Tests for Step 4 (faster trigger authoring):
//! selected object default, double-click to edit, Esc/Cmd+Enter keys,
//! filter and grouping, and trigger reordering with undo.

use crate::BottomTab;
use crate::test_harness::TestHarness;
use crate::triggers::describe;
use coursecraft_course::{Action, Course, Event, SlideTarget, Trigger};
use egui::{Key, Modifiers};

fn hello() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    Course::from_json(&json).expect("valid course")
}

fn open_triggers_tab(harness: &mut TestHarness) {
    harness.editor.ui.bottom_tab = BottomTab::Triggers;
    harness.idle_frame();
    harness.idle_frame();
}

#[test]
fn new_trigger_defaults_to_selected_object() {
    let mut harness = TestHarness::new_with_size(hello(), (1280.0, 800.0));
    harness.editor.selection.slide = "s-intro".into();
    let slide = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("s-intro");
    assert!(
        slide.objects.len() >= 2,
        "hello-course s-intro needs at least 2 objects"
    );
    let second_obj_id = slide.objects[1].id.clone();
    assert_ne!(slide.objects[0].id, second_obj_id);

    harness.editor.selection.object = Some(second_obj_id.clone());
    open_triggers_tab(&mut harness);

    let add_btn = harness
        .find_text("+ Add trigger")
        .expect("+ Add trigger button");
    harness.click_at(add_btn);
    harness.idle_frame();

    let form = harness
        .editor
        .ui
        .trigger_form
        .as_ref()
        .expect("trigger editor opened");
    assert_eq!(form.event_object, second_obj_id);
}

#[test]
fn double_click_opens_trigger_editor() {
    let mut harness = TestHarness::new_with_size(hello(), (1280.0, 800.0));
    harness.editor.selection.slide = "s-intro".into();
    open_triggers_tab(&mut harness);

    let slide = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("s-intro")
        .clone();
    assert!(!slide.triggers.is_empty(), "s-intro has triggers");
    let first_desc = describe(&slide.triggers[0], &slide);

    let row_pos = harness.find_text(&first_desc).expect("first trigger row");
    harness.click_at(row_pos);
    harness.click_at(row_pos);
    harness.idle_frame();

    let form = harness
        .editor
        .ui
        .trigger_form
        .as_ref()
        .expect("double-click opens trigger editor");
    assert_eq!(form.editing_index, Some(0));
}

#[test]
fn esc_closes_trigger_editor_without_saving() {
    let mut harness = TestHarness::new_with_size(hello(), (1280.0, 800.0));
    harness.editor.selection.slide = "s-intro".into();
    let rev_before = harness.editor.session.revision;
    let count_before = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("s-intro")
        .triggers
        .len();

    open_triggers_tab(&mut harness);
    let add_btn = harness
        .find_text("+ Add trigger")
        .expect("+ Add trigger button");
    harness.click_at(add_btn);
    harness.idle_frame();
    assert!(harness.editor.ui.trigger_form.is_some());

    // Open the action combo popup first; Esc should close only the popup, not the dialog.
    let combo_pos = harness.find_text("Jump to slide").expect("action combo");
    harness.click_at(combo_pos);
    harness.idle_frame();
    assert!(egui::Popup::is_any_open(&harness.ctx));

    harness.press(Key::Escape, Modifiers::NONE);
    harness.idle_frame();
    assert!(
        harness.editor.ui.trigger_form.is_some(),
        "Esc closed the combo popup, leaving the trigger editor open"
    );

    // Second Esc (with no popup open) closes the trigger editor without saving.
    harness.press(Key::Escape, Modifiers::NONE);
    harness.idle_frame();
    assert!(harness.editor.ui.trigger_form.is_none());
    assert_eq!(harness.editor.session.revision, rev_before);
    let count_after = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("s-intro")
        .triggers
        .len();
    assert_eq!(count_after, count_before);
}

#[test]
fn cmd_enter_saves_trigger_editor() {
    let mut harness = TestHarness::new_with_size(hello(), (1280.0, 800.0));
    harness.editor.selection.slide = "s-intro".into();
    let count_before = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("s-intro")
        .triggers
        .len();

    open_triggers_tab(&mut harness);
    let add_btn = harness
        .find_text("+ Add trigger")
        .expect("+ Add trigger button");
    harness.click_at(add_btn);
    harness.idle_frame();
    assert!(harness.editor.ui.trigger_form.is_some());

    harness.press(Key::Enter, Modifiers::COMMAND);
    harness.idle_frame();

    assert!(
        harness.editor.ui.trigger_form.is_none(),
        "Cmd+Enter saves and closes the trigger editor"
    );
    let count_after = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("s-intro")
        .triggers
        .len();
    assert_eq!(count_after, count_before + 1);
}

#[test]
fn filter_hides_non_matching_rows() {
    let mut harness = TestHarness::new_with_size(hello(), (1280.0, 800.0));
    harness.editor.selection.slide = "s-intro".into();

    // Ensure two distinct triggers exist on s-intro.
    let extra = Trigger {
        when: Event::TimelineStarts { layer: None },
        condition: None,
        action: Action::JumpToSlide {
            target: SlideTarget::Previous,
        },
        disabled: false,
    };
    harness.editor.run(
        "trigger.add",
        serde_json::json!({ "slide": "s-intro", "trigger": extra }),
    );
    open_triggers_tab(&mut harness);

    let slide = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("s-intro")
        .clone();
    let click_desc = describe(&slide.triggers[0], &slide);
    let timeline_desc = describe(slide.triggers.last().expect("added trigger"), &slide);

    assert!(harness.find_text(&click_desc).is_some());
    assert!(harness.find_text(&timeline_desc).is_some());

    harness.editor.ui.triggers.filter = "PREVIOUS".to_owned();
    harness.idle_frame();

    assert!(
        harness.find_text(&timeline_desc).is_some(),
        "matching trigger stays visible (case-insensitive)"
    );
    assert!(
        harness.find_text(&click_desc).is_none(),
        "non-matching trigger is hidden"
    );
}

#[test]
fn moving_trigger_down_changes_order_and_undo_restores_it() {
    let mut harness = TestHarness::new_with_size(hello(), (1280.0, 800.0));
    harness.editor.selection.slide = "s-intro".into();

    let extra = Trigger {
        when: Event::TimelineStarts { layer: None },
        condition: None,
        action: Action::JumpToSlide {
            target: SlideTarget::Previous,
        },
        disabled: false,
    };
    harness.editor.run(
        "trigger.add",
        serde_json::json!({ "slide": "s-intro", "trigger": extra }),
    );
    open_triggers_tab(&mut harness);

    let original_triggers = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("s-intro")
        .triggers
        .clone();
    assert!(original_triggers.len() >= 2);
    let slide = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("s-intro")
        .clone();
    let first_desc = describe(&original_triggers[0], &slide);

    // Click the first row to make it active and reveal its action buttons.
    let row_pos = harness.find_text(&first_desc).expect("first trigger row");
    harness.click_at(row_pos);
    harness.idle_frame();

    let down_btn = harness
        .find_icon(coursecraft_render::Icon::ArrowDown)
        .expect("move down button");
    harness.click_at(down_btn);
    harness.idle_frame();

    let moved_triggers = &harness
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("s-intro")
        .triggers;
    assert_eq!(moved_triggers[0], original_triggers[1]);
    assert_eq!(moved_triggers[1], original_triggers[0]);

    // Undo restores the original order.
    assert!(harness.editor.session.undo().is_ok());
    harness.idle_frame();
    let restored_triggers = &harness
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("s-intro")
        .triggers;
    assert_eq!(restored_triggers, &original_triggers);
}
