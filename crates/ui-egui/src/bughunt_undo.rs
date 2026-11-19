//! Bug hunt (2026-11-20): undo and redo after common actions, driven by keys
//! where the app has a key, on the healthy-habits course.
//!
//! For each action: it must change the course, add exactly one undo step, undo
//! must restore the course exactly, and redo must reapply the same course.

use crate::test_harness::TestHarness;
use crate::toolbar;
use coursecraft_course::Course;
use egui::{Key, Modifiers};

fn healthy() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/healthy-habits/course.json"
    ))
    .expect("healthy habits");
    Course::from_json(&json).expect("valid course")
}

fn key(harness: &mut TestHarness, key: Key, modifiers: Modifiers) {
    harness.press(key, modifiers);
    harness.idle_frame();
}

fn select_welcome(harness: &mut TestHarness, objects: &[&str]) {
    harness.editor.selection.slide = "welcome".into();
    harness.editor.selection.layer = None;
    harness.editor.selection.set_many(objects.iter().copied());
    harness.idle_frame();
    harness.idle_frame();
}

type Action = fn(&mut TestHarness);

fn nudge(h: &mut TestHarness) {
    select_welcome(h, &["w_title"]);
    key(h, Key::ArrowRight, Modifiers::NONE);
}

fn delete(h: &mut TestHarness) {
    select_welcome(h, &["w_chip"]);
    key(h, Key::Delete, Modifiers::NONE);
}

fn duplicate(h: &mut TestHarness) {
    select_welcome(h, &["w_chip"]);
    key(h, Key::D, Modifiers::COMMAND);
}

/// Copy and paste arrive as platform events (Event::Copy, Event::Paste) in the
/// app, so the harness sends those, as the platform does for Cmd+C and Cmd+V.
fn copy_paste(h: &mut TestHarness) {
    select_welcome(h, &["w_chip"]);
    h.frame(vec![egui::Event::Copy]);
    let text = h.editor.ui.clipboard_text.clone().unwrap_or_default();
    h.frame(vec![egui::Event::Paste(text)]);
    h.idle_frame();
}

fn group(h: &mut TestHarness) {
    select_welcome(h, &["w_title", "w_sub"]);
    key(h, Key::G, Modifiers::COMMAND);
}

fn add_slide(h: &mut TestHarness) {
    select_welcome(h, &[]);
    toolbar::add_new_slide(&mut h.editor);
    h.idle_frame();
}

fn add_scene(h: &mut TestHarness) {
    select_welcome(h, &[]);
    toolbar::add_new_scene(&mut h.editor);
    h.idle_frame();
}

fn add_layer(h: &mut TestHarness) {
    select_welcome(h, &[]);
    toolbar::add_new_layer(&mut h.editor);
    h.idle_frame();
}

fn insert_text_box(h: &mut TestHarness) {
    select_welcome(h, &[]);
    toolbar::insert_text_box(&mut h.editor);
    h.idle_frame();
    let c = h.central_panel_rect().center();
    h.click_at(c);
    h.idle_frame();
}

fn check_action(name: &str, act: Action) -> Vec<String> {
    let mut problems = Vec::new();
    let start = healthy();
    let mut h = TestHarness::new_with_size(start.clone(), (1280.0, 800.0));
    select_welcome(&mut h, &[]);
    let steps_before = h.editor.undo_len_for_test();
    act(&mut h);
    let after = h.editor.session.course.clone();
    let steps_after = h.editor.undo_len_for_test();
    if after == start {
        problems.push(format!("{name}: the action changed nothing"));
        return problems;
    }
    if steps_after != steps_before + 1 {
        problems.push(format!(
            "{name}: expected one undo step, got {}",
            steps_after as i64 - steps_before as i64
        ));
    }
    key(&mut h, Key::Z, Modifiers::COMMAND);
    if h.editor.session.course != start {
        problems.push(format!("{name}: Cmd+Z did not restore the course exactly"));
    }
    // Redo is bound to Ctrl+Y in the command registry (Cmd+Y on a Mac). Cmd+Shift+Z
    // is checked by its own test below.
    key(&mut h, Key::Y, Modifiers::COMMAND);
    if h.editor.session.course != after {
        problems.push(format!("{name}: Ctrl+Y did not reapply the action"));
    }
    problems
}

fn run_actions(actions: &[(&str, Action)]) {
    let mut problems = Vec::new();
    for (name, act) in actions {
        problems.extend(check_action(name, *act));
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn undo_redo_after_common_object_actions() {
    run_actions(&[
        ("nudge with arrow key", nudge),
        ("delete with Delete key", delete),
        ("duplicate with Cmd+D", duplicate),
        ("copy and paste", copy_paste),
        ("group with Cmd+G", group),
        ("insert text box", insert_text_box),
    ]);
}

#[test]
fn undo_redo_after_common_structure_actions() {
    run_actions(&[
        ("add slide", add_slide),
        ("add scene", add_scene),
        ("add layer", add_layer),
    ]);
}

/// Undo with nothing to undo must not panic and must not change the course.
#[test]
fn undo_with_empty_history_is_a_no_op() {
    let start = healthy();
    let mut h = TestHarness::new_with_size(start.clone(), (1280.0, 800.0));
    key(&mut h, Key::Z, Modifiers::COMMAND);
    key(&mut h, Key::Z, Modifiers::COMMAND | Modifiers::SHIFT);
    assert_eq!(h.editor.session.course, start);
}

/// A new action after an undo must clear the redo stack, so an old redo cannot
/// overwrite the new work.
#[test]
fn a_new_action_after_undo_clears_redo() {
    let start = healthy();
    let mut h = TestHarness::new_with_size(start.clone(), (1280.0, 800.0));
    delete(&mut h);
    key(&mut h, Key::Z, Modifiers::COMMAND);
    nudge(&mut h);
    let after_nudge = h.editor.session.course.clone();
    key(&mut h, Key::Y, Modifiers::COMMAND);
    assert_eq!(
        h.editor.session.course, after_nudge,
        "redo after a new action must do nothing"
    );
}

/// BUG: Cmd+Shift+Z (the standard Mac redo) does not redo. Redo is only bound to Ctrl+Y
/// in the command registry, and the Shift+Z press is handled as an undo.
#[test]
#[ignore = "BUG: Cmd+Shift+Z undoes instead of redoing; redo is only bound to Ctrl+Y"]
fn cmd_shift_z_redoes_after_common_actions() {
    let start = healthy();
    let mut problems = Vec::new();
    for (name, act) in [
        ("nudge with arrow key", nudge as Action),
        ("delete with Delete key", delete as Action),
        ("insert text box", insert_text_box as Action),
    ] {
        let mut h = TestHarness::new_with_size(start.clone(), (1280.0, 800.0));
        select_welcome(&mut h, &[]);
        act(&mut h);
        let after = h.editor.session.course.clone();
        key(&mut h, Key::Z, Modifiers::COMMAND);
        key(&mut h, Key::Z, Modifiers::COMMAND | Modifiers::SHIFT);
        if h.editor.session.course == start {
            problems.push(format!(
                "{name}: Cmd+Shift+Z undid the action again instead of redoing it"
            ));
        } else if h.editor.session.course != after {
            problems.push(format!("{name}: Cmd+Shift+Z did not reapply the action"));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
