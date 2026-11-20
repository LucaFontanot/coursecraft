//! Bug hunt (2026-11-20, M24): inline rename, timeline seconds field, publish
//! memory and the deferred audit, tried with the edge cases the M24 tests skip.
//!
//! Tests that reveal a real bug assert the correct behaviour and are `#[ignore]`d
//! with a `BUG:` reason so the normal suite stays green. Run them with
//! `cargo test -p coursecraft-ui-egui --lib bughunt_m24 -j 4 -- --include-ignored`.

use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use egui::{Key, Modifiers};

fn hello() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    Course::from_json(&json).expect("valid course")
}

fn slide_title(h: &TestHarness, id: &str) -> String {
    h.editor
        .session
        .course
        .slide(id)
        .expect("slide exists")
        .title
        .clone()
}

fn second_slide_id(h: &TestHarness) -> String {
    h.editor
        .session
        .course
        .slides()
        .nth(1)
        .expect("hello course has a second slide")
        .id
        .clone()
}

/// Double-click the title of `id` in the left panel, which opens inline rename.
fn open_rename(h: &mut TestHarness, id: &str) {
    let title = slide_title(h, id);
    let left = h.left_panel_rect();
    let pos = h
        .find_text_in(&title, left)
        .expect("slide title in left panel");
    // Advance egui time so two separate double-clicks are not merged.
    static TICK: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(10);
    let t = TICK.fetch_add(10, std::sync::atomic::Ordering::SeqCst) as f64;
    h.frame_at_time(t);
    h.click_at(pos);
    h.frame_at_time(t + 0.1);
    h.click_at(pos);
    h.idle_frame();
    assert!(h.editor.ui.slide_rename.is_some(), "rename should open");
}

#[test]
fn rename_long_emoji_name_is_saved_trimmed_and_undoes() {
    let mut h = TestHarness::new(hello());
    h.idle_frame();
    let orig = slide_title(&h, "s-intro");
    open_rename(&mut h, "s-intro");

    let long = format!("  Café 🎉 {}  ", "x".repeat(300));
    h.type_text(&long);
    h.press(Key::Enter, Modifiers::NONE);
    h.idle_frame();

    let saved = slide_title(&h, "s-intro");
    assert_eq!(saved, long.trim(), "name should be saved trimmed");
    h.editor.run("edit.undo", serde_json::json!({}));
    assert_eq!(slide_title(&h, "s-intro"), orig);
}

#[test]
fn rename_same_title_adds_no_undo_step() {
    let mut h = TestHarness::new(hello());
    h.idle_frame();
    let orig = slide_title(&h, "s-intro");
    let before = h.editor.session.undo_len();
    open_rename(&mut h, "s-intro");
    // Retype the existing title exactly.
    h.frame(vec![egui::Event::Key {
        key: Key::A,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::COMMAND,
    }]);
    h.type_text(&orig);
    h.press(Key::Enter, Modifiers::NONE);
    h.idle_frame();
    assert_eq!(h.editor.session.undo_len(), before);
}

#[test]
fn two_renames_in_a_row_undo_one_at_a_time() {
    let mut h = TestHarness::new(hello());
    h.idle_frame();
    let orig = slide_title(&h, "s-intro");

    open_rename(&mut h, "s-intro");
    h.type_text("First");
    h.press(Key::Enter, Modifiers::NONE);
    h.idle_frame();
    open_rename(&mut h, "s-intro");
    h.type_text("Second");
    h.press(Key::Enter, Modifiers::NONE);
    h.idle_frame();
    assert_eq!(slide_title(&h, "s-intro"), "Second");

    h.editor.run("edit.undo", serde_json::json!({}));
    assert_eq!(slide_title(&h, "s-intro"), "First");
    h.editor.run("edit.undo", serde_json::json!({}));
    assert_eq!(slide_title(&h, "s-intro"), orig);
    h.editor.run("edit.redo", serde_json::json!({}));
    assert_eq!(slide_title(&h, "s-intro"), "First");
}

#[test]
fn rename_while_another_slide_is_selected_renames_the_double_clicked_slide() {
    let mut h = TestHarness::new(hello());
    let other = second_slide_id(&h);
    h.editor.selection.slide = other.clone();
    h.idle_frame();
    let other_before = slide_title(&h, &other);

    open_rename(&mut h, "s-intro");
    h.type_text("Picked Intro");
    h.press(Key::Enter, Modifiers::NONE);
    h.idle_frame();

    assert_eq!(slide_title(&h, "s-intro"), "Picked Intro");
    assert_eq!(slide_title(&h, &other), other_before);
}

#[test]
fn rename_then_click_another_slide_commits_to_the_renamed_slide() {
    let mut h = TestHarness::new(hello());
    h.idle_frame();
    let other = second_slide_id(&h);
    let other_before = slide_title(&h, &other);

    open_rename(&mut h, "s-intro");
    h.type_text("Mid Edit");
    // Click the other slide's title while still editing.
    let left = h.left_panel_rect();
    let pos = h
        .find_text_in(&other_before, left)
        .expect("other slide title in left panel");
    h.click_at(pos);
    h.idle_frame();

    assert!(
        h.editor.ui.slide_rename.is_none(),
        "click away should close rename"
    );
    assert_eq!(slide_title(&h, "s-intro"), "Mid Edit");
    assert_eq!(slide_title(&h, &other), other_before);
}

#[test]
#[ignore = "BUG: deleting the slide mid-rename leaves the rename open for a missing slide"]
fn rename_then_delete_the_slide_leaves_no_rename_open() {
    let mut h = TestHarness::new(hello());
    h.idle_frame();
    let other = second_slide_id(&h);
    h.editor.selection.slide = "s-intro".into();
    open_rename(&mut h, "s-intro");
    h.type_text("Doomed");
    // Delete the slide through the command path while the field is still open.
    let _ = h
        .editor
        .run("slide.delete", serde_json::json!({ "slide": "s-intro" }));
    h.idle_frame();
    h.press(Key::Enter, Modifiers::NONE);
    h.idle_frame();
    assert!(
        h.editor.ui.slide_rename.is_none(),
        "rename of a deleted slide must close"
    );
    assert!(h.editor.session.course.slide(&other).is_some());
}

#[test]
fn escape_after_typing_leaves_undo_stack_unchanged() {
    let mut h = TestHarness::new(hello());
    h.idle_frame();
    let before = h.editor.session.undo_len();
    open_rename(&mut h, "s-intro");
    h.type_text("Nope");
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert_eq!(h.editor.session.undo_len(), before);
}

/// Seconds field: typed values must never store 0 ms or a non-finite duration.
#[test]
fn timeline_duration_hostile_inputs_stay_in_range() {
    let inputs = ["0", "-5", "1e9", "abc", "2.5s", "2,5", "0.04", "NaN", "inf"];
    for input in inputs {
        let mut h = TestHarness::new_with_size(hello(), (1280.0, 800.0));
        h.editor.selection.slide = "s-intro".into();
        h.editor.run(
            "slide.update",
            serde_json::json!({ "slide": "s-intro", "duration_ms": 5000 }),
        );
        h.idle_frame();
        let rect = h
            .editor
            .ui
            .timeline
            .duration_edit_rect
            .expect("duration field");
        h.set_field(rect.center(), input);
        h.idle_frame();
        let ms = h
            .editor
            .session
            .course
            .slide("s-intro")
            .expect("slide")
            .duration_ms;
        assert!(
            (100..=86_400_000).contains(&ms),
            "input {input:?} stored duration {ms} ms, outside 100..=86400000"
        );
    }
}

#[test]
fn timeline_duration_seconds_undo_restores_previous_value() {
    let mut h = TestHarness::new_with_size(hello(), (1280.0, 800.0));
    h.editor.selection.slide = "s-intro".into();
    h.editor.run(
        "slide.update",
        serde_json::json!({ "slide": "s-intro", "duration_ms": 5000 }),
    );
    h.idle_frame();
    let rect = h.editor.ui.timeline.duration_edit_rect.expect("field");
    h.set_field(rect.center(), "0.5");
    h.idle_frame();
    assert_eq!(
        h.editor
            .session
            .course
            .slide("s-intro")
            .expect("s")
            .duration_ms,
        500
    );
    h.editor.run("edit.undo", serde_json::json!({}));
    assert_eq!(
        h.editor
            .session
            .course
            .slide("s-intro")
            .expect("s")
            .duration_ms,
        5000
    );
}

/// The Publish button stores its standard with `view.set`. Check whether that
/// adds a visible undo step (a Cmd+Z that changes nothing the author sees).
#[test]
#[ignore = "BUG: pressing Publish with a new standard adds an undo step (view.set last_publish)"]
fn publish_remembered_standard_is_not_an_undo_step() {
    let mut h = TestHarness::new(hello());
    h.editor.set_publish_available(true);
    h.idle_frame();
    h.editor.open_publish_dialog();
    h.idle_frame();
    let web = h.find_text("Web (any website, no LMS)").expect("Web radio");
    h.click_at(web);
    h.idle_frame();
    let before = h.editor.session.undo_len();
    let publish_btn = h.find_text_last("Publish").expect("Publish button");
    h.frame(vec![egui::Event::PointerMoved(publish_btn)]);
    h.frame(vec![egui::Event::PointerButton {
        pos: publish_btn,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Default::default(),
    }]);
    h.frame(vec![egui::Event::PointerButton {
        pos: publish_btn,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: Default::default(),
    }]);
    assert_eq!(
        h.editor.session.undo_len(),
        before,
        "remembering the publish standard should not add an undo step"
    );
}

/// Audit debounce: results opened mid-quiet-time must equal a direct audit run.
#[test]
fn audit_opened_mid_quiet_time_matches_direct_run() {
    let mut h = TestHarness::new(hello());
    let _ = h.frame_at_time(0.1);
    let _ = h.frame_at_time(0.5);
    for i in 0..5 {
        let _ = h.editor.run(
            "object.update",
            serde_json::json!({
                "slide": "s-intro",
                "object": "btn-start",
                "patch": { "alt": "" },
            }),
        );
        let _ = h.editor.run(
            "slide.update",
            serde_json::json!({ "slide": "s-intro", "title": format!("Quick {i}") }),
        );
        let _ = h.frame_at_time(1.0 + i as f64 * 0.05);
    }
    h.editor.ui.show_validation_window = true;
    let _ = h.frame_at_time(1.2);
    let _ = h.frame_at_time(1.25);

    let direct = h
        .editor
        .run("course.audit", serde_json::json!({}))
        .expect("audit result");
    let (findings, _) = crate::parse_audit_result(&direct);
    assert_eq!(
        format!("{:?}", h.editor.ui.audit_findings),
        format!("{findings:?}"),
        "panel audit must match a direct audit"
    );
}
