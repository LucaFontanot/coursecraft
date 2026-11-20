//! Bug hunt (2026-11-20): every Insert item, in small windows.
//!
//! For each insert the test arms the tool the way the toolbar does, places it with
//! a click on the canvas, selects the new object, checks the Format panel with
//! `layout_report`, then undoes with Cmd+Z and expects the course to be exactly
//! what it was before the insert.

use crate::properties::InspectorTab;
use crate::test_harness::TestHarness;
use crate::{Editor, toolbar};
use coursecraft_course::{Course, QuestionKind};
use egui::{Key, Modifiers, Pos2};

fn course(path: &str) -> Course {
    let json = std::fs::read_to_string(format!("{}/../../{path}", env!("CARGO_MANIFEST_DIR")))
        .expect("course file");
    Course::from_json(&json).expect("valid course")
}

fn first_slide_id(course: &Course) -> String {
    course.slides().next().expect("a slide").id.clone()
}

fn object_ids(course: &Course, slide: &str) -> Vec<String> {
    course
        .slide(slide)
        .map(|s| s.objects.iter().map(|o| o.id.clone()).collect())
        .unwrap_or_default()
}

fn canvas_centre(harness: &TestHarness) -> Pos2 {
    harness.central_panel_rect().center()
}

/// Arms, places by clicking the canvas, and returns the ids that appeared.
fn place(harness: &mut TestHarness, arm: impl FnOnce(&mut Editor)) -> Vec<String> {
    let slide = harness.editor.selection.slide.clone();
    let before = object_ids(&harness.editor.session.course, &slide);
    arm(&mut harness.editor);
    harness.idle_frame();
    harness.click_at(canvas_centre(harness));
    harness.idle_frame();
    harness.idle_frame();
    object_ids(&harness.editor.session.course, &slide)
        .into_iter()
        .filter(|id| !before.contains(id))
        .collect()
}

fn undo(harness: &mut TestHarness) {
    if harness.editor.ui.inline_edit.is_some() {
        harness.press(Key::Escape, Modifiers::NONE);
        harness.idle_frame();
    }
    harness.press(Key::Z, Modifiers::COMMAND);
    harness.idle_frame();
}

fn format_problems(harness: &mut TestHarness, id: &str) -> Vec<String> {
    let slide = harness.editor.selection.slide.clone();
    harness.editor.selection.slide = slide;
    harness.editor.selection.set_many([id]);
    harness.editor.ui.inspector_tab = InspectorTab::Format;
    let report = harness.layout_report(3);
    report
        .findings
        .into_iter()
        .filter(|f| {
            f.starts_with("clipped-widget")
                || f.starts_with("overlap")
                || f.starts_with("off-window")
        })
        .collect()
}

/// Shared body: insert, check the new object, check its Format panel at the given
/// window size, undo once, and expect the course to be unchanged.
fn insert_check_undo(
    name: &str,
    size: (f32, f32),
    arm: impl FnOnce(&mut Editor),
    expect_object: bool,
) -> Vec<String> {
    let mut problems = Vec::new();
    let start = course("examples/healthy-habits/course.json");
    let slide = first_slide_id(&start);
    let mut harness = TestHarness::new_with_size(start.clone(), size);
    harness.editor.selection.slide = slide.clone();
    harness.editor.selection.clear_objects();
    harness.idle_frame();
    let added = place(&mut harness, arm);
    if expect_object && added.is_empty() {
        problems.push(format!(
            "{name}: no object was added by a click on the canvas"
        ));
    }
    for id in &added {
        for p in format_problems(&mut harness, id) {
            problems.push(format!("{name} @{size:?}: {p}"));
        }
    }
    harness.editor.selection.clear_objects();
    undo(&mut harness);
    if harness.editor.session.course != start {
        problems.push(format!(
            "{name} @{size:?}: one undo did not restore the course exactly (objects now {:?})",
            object_ids(&harness.editor.session.course, &slide)
        ));
    }
    problems
}

const SIZES: [(f32, f32); 3] = [(1100.0, 700.0), (1280.0, 800.0), (1280.0, 900.0)];

fn run_all(name: &str, arm: fn(&mut Editor), expect_object: bool) {
    let mut problems = Vec::new();
    for size in SIZES {
        problems.extend(insert_check_undo(name, size, arm, expect_object));
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn insert_text_box_is_placed_formatted_and_undone() {
    run_all("text box", toolbar::insert_text_box, true);
}

#[test]
fn insert_button_is_placed_formatted_and_undone() {
    run_all("button", toolbar::insert_button, true);
}

#[test]
fn insert_rectangle_is_placed_formatted_and_undone() {
    run_all("rectangle", toolbar::insert_rectangle, true);
}

#[test]
fn insert_rounded_rectangle_is_placed_formatted_and_undone() {
    run_all("rounded rectangle", toolbar::insert_rounded_rectangle, true);
}

#[test]
fn insert_ellipse_is_placed_formatted_and_undone() {
    run_all("ellipse", toolbar::insert_ellipse, true);
}

#[test]
fn insert_table_is_formatted_and_undone() {
    let mut problems = Vec::new();
    for size in SIZES {
        let start = course("examples/healthy-habits/course.json");
        let slide = first_slide_id(&start);
        let mut harness = TestHarness::new_with_size(start.clone(), size);
        harness.editor.selection.slide = slide.clone();
        harness.idle_frame();
        toolbar::insert_table(&mut harness.editor);
        harness.idle_frame();
        let added: Vec<String> = object_ids(&harness.editor.session.course, &slide)
            .into_iter()
            .filter(|id| !object_ids(&start, &slide).contains(id))
            .collect();
        if added.is_empty() {
            problems.push(format!("table @{size:?}: no table was added"));
        }
        for id in &added {
            problems.extend(
                format_problems(&mut harness, id)
                    .into_iter()
                    .map(|p| format!("table @{size:?}: {p}")),
            );
        }
        undo(&mut harness);
        if harness.editor.session.course != start {
            problems.push(format!("table @{size:?}: undo did not restore the course"));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn insert_input_widgets_are_formatted_and_undone() {
    type Insert = fn(&mut Editor) -> Option<String>;
    let inserts: [(&str, Insert); 5] = [
        ("text entry", crate::input_insert::insert_text_entry),
        ("slider", crate::input_insert::insert_slider),
        ("dial", crate::input_insert::insert_dial),
        ("checkbox", crate::input_insert::insert_checkbox),
        ("radio group", crate::input_insert::insert_radio_group),
    ];
    let mut problems = Vec::new();
    for (name, insert) in inserts {
        for size in SIZES {
            let start = course("examples/healthy-habits/course.json");
            let slide = first_slide_id(&start);
            let mut harness = TestHarness::new_with_size(start.clone(), size);
            harness.editor.selection.slide = slide.clone();
            harness.idle_frame();
            let Some(id) = insert(&mut harness.editor) else {
                problems.push(format!("{name} @{size:?}: insert returned nothing"));
                continue;
            };
            harness.idle_frame();
            problems.extend(
                format_problems(&mut harness, &id)
                    .into_iter()
                    .map(|p| format!("{name} @{size:?}: {p}")),
            );
            undo(&mut harness);
            if harness.editor.session.course != start {
                problems.push(format!("{name} @{size:?}: undo did not restore the course"));
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn insert_interaction_gallery_items_are_undone() {
    type Insert = fn(&mut Editor) -> Option<String>;
    let inserts: [(&str, Insert); 4] = [
        ("marker", crate::interaction_insert::insert_marker),
        ("timer", crate::interaction_insert::insert_timer),
        (
            "progress meter",
            crate::interaction_insert::insert_progress_meter,
        ),
        (
            "scroll panel",
            crate::interaction_insert::insert_scroll_panel,
        ),
    ];
    let mut problems = Vec::new();
    for (name, insert) in inserts {
        for size in SIZES {
            let start = course("examples/healthy-habits/course.json");
            let slide = first_slide_id(&start);
            let mut harness = TestHarness::new_with_size(start.clone(), size);
            harness.editor.selection.slide = slide.clone();
            harness.idle_frame();
            let Some(id) = insert(&mut harness.editor) else {
                problems.push(format!("{name} @{size:?}: insert returned nothing"));
                continue;
            };
            harness.idle_frame();
            problems.extend(
                format_problems(&mut harness, &id)
                    .into_iter()
                    .map(|p| format!("{name} @{size:?}: {p}")),
            );
            undo(&mut harness);
            if harness.editor.session.course != start {
                problems.push(format!("{name} @{size:?}: undo did not restore the course"));
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn insert_question_types_are_added_and_undone() {
    let kinds = [
        QuestionKind::MultipleChoice,
        QuestionKind::MultipleResponse,
        QuestionKind::TrueFalse,
        QuestionKind::FillIn,
        QuestionKind::Sequence,
        QuestionKind::Matching,
        QuestionKind::Likert,
    ];
    let mut problems = Vec::new();
    for kind in kinds {
        let start = course("examples/healthy-habits/course.json");
        let slide = first_slide_id(&start);
        let mut harness = TestHarness::new_with_size(start.clone(), (1280.0, 800.0));
        harness.editor.selection.slide = slide.clone();
        harness.idle_frame();
        toolbar::insert_question(&mut harness.editor, kind);
        harness.idle_frame();
        if harness.editor.session.course == start {
            problems.push(format!("{kind:?}: insert changed nothing"));
        }
        undo(&mut harness);
        if harness.editor.session.course != start {
            problems.push(format!("{kind:?}: undo did not restore the course"));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn insert_then_second_undo_is_harmless() {
    // A second Cmd+Z with nothing left to undo must not change the course or panic.
    let start = course("examples/healthy-habits/course.json");
    let mut harness = TestHarness::new_with_size(start.clone(), (1280.0, 800.0));
    harness.editor.selection.slide = first_slide_id(&start);
    harness.idle_frame();
    toolbar::insert_rectangle(&mut harness.editor);
    harness.click_at(canvas_centre(&harness));
    harness.idle_frame();
    undo(&mut harness);
    undo(&mut harness);
    undo(&mut harness);
    assert_eq!(harness.editor.session.course, start);
}

/// BUG: the Correct-when condition editor of a Freeform question panics. Its
/// condition group calls `ui.indent` inside a `ui.horizontal` row (triggers.rs,
/// `render_condition_group`), and egui asserts that an indent is in a vertical layout.
/// The healthy-habits slide `q9` is a freeform question with a condition.
#[test]
fn freeform_question_condition_editor_renders_without_panicking() {
    let start = course("examples/healthy-habits/course.json");
    let mut harness = TestHarness::new_with_size(start, (1280.0, 800.0));
    harness.editor.selection.slide = "q9".into();
    harness.editor.ui.bottom_tab = crate::BottomTab::Question;
    harness.idle_frame();
    harness.idle_frame();
    harness.idle_frame();
}

/// BUG: inserting a Freeform question on a course whose first variable is a
/// Boolean is refused. The engine's default condition compares the first variable
/// with a number, so the validation fails and nothing is inserted (no visible
/// cause in the Question panel beyond the status line).
#[test]
fn freeform_question_insert_works_when_the_first_variable_is_a_boolean() {
    let start = course("examples/healthy-habits/course.json");
    let mut harness = TestHarness::new_with_size(start, (1280.0, 800.0));
    harness.editor.selection.slide = "welcome".into();
    harness.idle_frame();
    toolbar::insert_question(&mut harness.editor, QuestionKind::Freeform);
    harness.idle_frame();
    assert!(
        harness
            .editor
            .session
            .course
            .slide("welcome")
            .is_some_and(|s| s.question.is_some()),
        "the freeform question was added: {:?}",
        harness.editor.ui.last_error
    );
}
