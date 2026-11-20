//! Bug hunt (2026-11-20, M24, second pass): trigger list and trigger editor.
//!
//! Filter edge cases, group by object, Esc and Cmd+Enter in the trigger editor, move
//! up and down at the ends, reorder with redo, and a new trigger on a layer object.

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

fn open_triggers(h: &mut TestHarness, slide: &str) {
    h.editor.selection.slide = slide.into();
    h.editor.ui.bottom_tab = BottomTab::Triggers;
    h.idle_frame();
    h.idle_frame();
}

fn add_extra_trigger(h: &mut TestHarness, slide: &str) {
    let extra = Trigger {
        when: Event::TimelineStarts { layer: None },
        condition: None,
        action: Action::JumpToSlide {
            target: SlideTarget::Previous,
        },
        disabled: false,
    };
    h.editor.run(
        "trigger.add",
        serde_json::json!({ "slide": slide, "trigger": extra }),
    );
}

fn trigger_count(h: &TestHarness, slide: &str) -> usize {
    h.editor
        .session
        .course
        .slide(slide)
        .expect("slide")
        .triggers
        .len()
}

#[test]
fn filter_with_no_matches_shows_the_message() {
    let mut h = TestHarness::new_with_size(hello(), (1280.0, 800.0));
    open_triggers(&mut h, "s-intro");
    h.editor.ui.triggers.filter = "zzqqxx".to_owned();
    h.idle_frame();
    assert!(h.find_text("No triggers match the filter.").is_some());
}

#[test]
fn filter_treats_regex_like_characters_literally() {
    let mut h = TestHarness::new_with_size(hello(), (1280.0, 800.0));
    open_triggers(&mut h, "s-intro");
    for needle in [".*", "[(", ".*[(", "\\", "(?i)"] {
        h.editor.ui.triggers.filter = needle.to_owned();
        h.idle_frame();
        assert!(
            h.find_text("No triggers match the filter.").is_some(),
            "needle {needle:?} matches nothing literally"
        );
    }
}

#[test]
fn filter_is_case_insensitive_for_mixed_case_text() {
    let mut h = TestHarness::new_with_size(hello(), (1280.0, 800.0));
    open_triggers(&mut h, "s-intro");
    add_extra_trigger(&mut h, "s-intro");
    h.idle_frame();
    let slide = h
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("slide")
        .clone();
    let desc = describe(slide.triggers.last().expect("trigger"), &slide);
    let mixed: String = desc
        .chars()
        .enumerate()
        .map(|(i, c)| {
            if i % 2 == 0 {
                c.to_ascii_uppercase()
            } else {
                c.to_ascii_lowercase()
            }
        })
        .collect();
    h.editor.ui.triggers.filter = mixed;
    h.idle_frame();
    assert!(h.find_text(&desc).is_some(), "mixed-case filter matches");
}

#[test]
fn group_by_object_shows_a_slide_header_for_slide_triggers() {
    let mut h = TestHarness::new_with_size(hello(), (1280.0, 800.0));
    open_triggers(&mut h, "s-intro");
    add_extra_trigger(&mut h, "s-intro");
    h.idle_frame();
    let pos = h.find_text("Group by object").expect("group checkbox");
    h.click_at(pos);
    h.idle_frame();
    assert!(h.editor.ui.triggers.group_by_object);
    assert!(
        h.find_text("Slide").is_some(),
        "slide-level triggers appear under a Slide header"
    );
}

#[test]
fn cmd_enter_on_a_new_trigger_never_saves_an_invalid_trigger() {
    let mut h = TestHarness::new_with_size(hello(), (1280.0, 800.0));
    open_triggers(&mut h, "s-intro");
    let issues_before = h.editor.session.course.validate().len();
    let count_before = trigger_count(&h, "s-intro");

    let add = h.find_text("+ Add trigger").expect("add button");
    h.click_at(add);
    h.idle_frame();
    h.press(Key::Enter, Modifiers::COMMAND);
    h.idle_frame();

    let issues_after = h.editor.session.course.validate().len();
    let count_after = trigger_count(&h, "s-intro");
    if count_after > count_before {
        assert!(
            issues_after <= issues_before,
            "a saved trigger must not add validation problems"
        );
    } else {
        assert!(
            h.editor.ui.trigger_form.is_some(),
            "an invalid trigger keeps the editor open with its error"
        );
    }
}

#[test]
fn cmd_enter_pressed_twice_saves_exactly_one_trigger() {
    let mut h = TestHarness::new_with_size(hello(), (1280.0, 800.0));
    open_triggers(&mut h, "s-intro");
    let count_before = trigger_count(&h, "s-intro");
    let add = h.find_text("+ Add trigger").expect("add button");
    h.click_at(add);
    h.idle_frame();

    h.press(Key::Enter, Modifiers::COMMAND);
    h.press(Key::Enter, Modifiers::COMMAND);
    h.idle_frame();

    assert_eq!(trigger_count(&h, "s-intro"), count_before + 1);
}

#[test]
fn move_up_on_the_first_row_changes_nothing_and_adds_no_undo_step() {
    let mut h = TestHarness::new_with_size(hello(), (1280.0, 800.0));
    open_triggers(&mut h, "s-intro");
    add_extra_trigger(&mut h, "s-intro");
    h.idle_frame();
    let before = h
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("s")
        .triggers
        .clone();
    let undo_before = h.editor.session.undo_len();

    let first_desc = describe(
        &before[0],
        h.editor.session.course.slide("s-intro").expect("s"),
    );
    let row = h.find_text(&first_desc).expect("first row");
    h.click_at(row);
    h.hover_at(row);
    h.idle_frame();
    if let Some(up) = h.find_icon(coursecraft_render::Icon::ArrowUp) {
        h.click_at(up);
        h.idle_frame();
    }

    assert_eq!(
        h.editor
            .session
            .course
            .slide("s-intro")
            .expect("s")
            .triggers,
        before
    );
    assert_eq!(h.editor.session.undo_len(), undo_before, "no undo step");
}

#[test]
fn move_down_on_the_last_row_changes_nothing_and_adds_no_undo_step() {
    let mut h = TestHarness::new_with_size(hello(), (1280.0, 800.0));
    open_triggers(&mut h, "s-intro");
    add_extra_trigger(&mut h, "s-intro");
    h.idle_frame();
    let before = h
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("s")
        .triggers
        .clone();
    let undo_before = h.editor.session.undo_len();
    let last_desc = describe(
        before.last().expect("last"),
        h.editor.session.course.slide("s-intro").expect("s"),
    );
    let row = h.find_text(&last_desc).expect("last row");
    h.click_at(row);
    h.hover_at(row);
    h.idle_frame();
    if let Some(down) = h.find_icon(coursecraft_render::Icon::ArrowDown) {
        h.click_at(down);
        h.idle_frame();
    }
    assert_eq!(
        h.editor
            .session
            .course
            .slide("s-intro")
            .expect("s")
            .triggers,
        before
    );
    assert_eq!(h.editor.session.undo_len(), undo_before, "no undo step");
}

#[test]
fn reorder_then_undo_then_redo_restores_each_order() {
    let mut h = TestHarness::new_with_size(hello(), (1280.0, 800.0));
    open_triggers(&mut h, "s-intro");
    add_extra_trigger(&mut h, "s-intro");
    h.idle_frame();
    let original = h
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("s")
        .triggers
        .clone();

    h.editor.run(
        "trigger.move",
        serde_json::json!({ "slide": "s-intro", "from": 0, "to": 1 }),
    );
    let moved = h
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("s")
        .triggers
        .clone();
    assert_ne!(moved, original);

    h.editor.session.undo().expect("undo");
    assert_eq!(
        h.editor
            .session
            .course
            .slide("s-intro")
            .expect("s")
            .triggers,
        original
    );
    h.editor.session.redo().expect("redo");
    assert_eq!(
        h.editor
            .session
            .course
            .slide("s-intro")
            .expect("s")
            .triggers,
        moved
    );
}

#[test]
fn new_trigger_defaults_to_an_object_on_a_layer() {
    let mut h = TestHarness::new_with_size(hello(), (1280.0, 800.0));
    let layer_obj = h
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("s-intro")
        .layers
        .iter()
        .flat_map(|l| l.objects.iter())
        .map(|o| o.id.clone())
        .next()
        .expect("hello s-intro has a layer object");
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.object = Some(layer_obj.clone());
    h.editor.ui.bottom_tab = BottomTab::Triggers;
    h.idle_frame();
    h.idle_frame();

    let add = h.find_text("+ Add trigger");
    if let Some(pos) = add {
        h.click_at(pos);
        h.idle_frame();
    } else {
        // Slide already has triggers: the toolbar button is the second place to look.
        let pos = h
            .find_text_containing("Add trigger")
            .expect("add trigger control");
        h.click_at(pos);
        h.idle_frame();
    }
    let form = h.editor.ui.trigger_form.as_ref().expect("editor open");
    assert_eq!(
        form.event_object, layer_obj,
        "defaults to the selected layer object"
    );
}

#[test]
fn trigger_tab_filter_row_fits_a_narrow_window() {
    let mut h = TestHarness::new_with_size(hello(), (1100.0, 700.0));
    open_triggers(&mut h, "s-intro");
    add_extra_trigger(&mut h, "s-intro");
    h.editor.ui.triggers.filter = "a filter that is fairly long".to_owned();
    let report = h.layout_report(3);
    let problems: Vec<&String> = report
        .findings
        .iter()
        .filter(|f| f.to_ascii_lowercase().contains("filter"))
        .collect();
    assert!(
        problems.is_empty(),
        "filter row layout problems: {problems:?}"
    );
}
