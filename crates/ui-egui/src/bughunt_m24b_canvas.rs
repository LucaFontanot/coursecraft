//! Bug hunt (2026-11-20, M24, second pass): canvas text editing and pan.
//!
//! Covers Space while a text box is in edit mode, Cmd+B/I/U with a partial selection,
//! a caret only and no selection, undo after a style toggle, middle-mouse pan, and
//! Cmd+B on an object that is not being edited.

use crate::test_harness::TestHarness;
use coursecraft_course::{Course, ObjectKind};
use egui::{Event, Key, Modifiers, PointerButton, Pos2, vec2};

fn hello() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    Course::from_json(&json).expect("valid course")
}

fn object_center(h: &TestHarness, slide_id: &str, obj_id: &str) -> Pos2 {
    let slide_rect = h.editor.ui.last_slide_rect.expect("slide rect");
    let slide_w = h.editor.session.course.slide_size.w as f32;
    let scale = slide_rect.width() / slide_w;
    let (obj, _) = h
        .editor
        .session
        .course
        .slide(slide_id)
        .and_then(|s| s.object(obj_id))
        .expect("object");
    slide_rect.min
        + vec2(
            (obj.frame.x + obj.frame.w * 0.5) * scale,
            (obj.frame.y + obj.frame.h * 0.5) * scale,
        )
}

/// Make the title a plain, non-bold text box on s-intro with a known text.
fn plain_title_course() -> Course {
    let mut course = hello();
    if let Some(slide) = course.slide_mut("s-intro")
        && let Some(obj) = slide.objects.iter_mut().find(|o| o.id == "title")
        && let ObjectKind::Text { style, runs, .. } = &mut obj.kind
    {
        style.bold = false;
        runs.clear();
    }
    course
}

fn title_text_and_bold(h: &TestHarness) -> (String, Vec<(usize, bool)>) {
    let (obj, _) = h
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object("title"))
        .expect("title");
    match &obj.kind {
        ObjectKind::Text {
            text, style, runs, ..
        } => {
            if runs.is_empty() {
                (text.clone(), vec![(text.chars().count(), style.bold)])
            } else {
                (
                    runs.iter().map(|r| r.text.as_str()).collect(),
                    runs.iter()
                        .map(|r| (r.text.chars().count(), r.bold.unwrap_or(style.bold)))
                        .collect(),
                )
            }
        }
        other => panic!("expected text, got {other:?}"),
    }
}

/// Count of characters that are bold, from the stored runs (or the base style).
fn bold_chars(h: &TestHarness) -> usize {
    title_text_and_bold(h)
        .1
        .iter()
        .filter(|(_, bold)| *bold)
        .map(|(n, _)| *n)
        .sum()
}

fn enter_title_edit(h: &mut TestHarness) {
    let center = object_center(h, "s-intro", "title");
    h.click_at(center);
    h.click_at(center);
    h.idle_frame();
    assert!(h.editor.ui.inline_edit.is_some(), "entered inline edit");
}

fn prep(h: &mut TestHarness) {
    h.editor.ui.snap_to_guides = false;
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.object = Some("title".into());
    h.idle_frame();
    h.idle_frame();
}

#[test]
fn space_while_editing_types_a_space_and_does_not_pan() {
    let mut h = TestHarness::new(plain_title_course());
    prep(&mut h);
    enter_title_edit(&mut h);
    let original = h
        .editor
        .ui
        .inline_edit
        .as_ref()
        .expect("edit")
        .original
        .clone();
    let slide_before = h.editor.ui.last_slide_rect.expect("slide rect").min;

    // Collapse the select-all placeholder to the end, then press Space.
    h.press(Key::ArrowRight, Modifiers::NONE);
    h.frame(vec![
        Event::Key {
            key: Key::Space,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        },
        Event::Text(" ".to_owned()),
    ]);
    h.frame(vec![Event::Key {
        key: Key::Space,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    h.idle_frame();

    let buffer = h
        .editor
        .ui
        .inline_edit
        .as_ref()
        .expect("still editing")
        .buffer
        .clone();
    assert_eq!(buffer, format!("{original} "), "Space typed a space");
    let slide_after = h.editor.ui.last_slide_rect.expect("slide rect").min;
    assert!(
        (slide_after - slide_before).length() < 0.5,
        "Space while editing did not pan the canvas"
    );
}

#[test]
fn space_drag_while_editing_does_not_move_the_object() {
    let mut h = TestHarness::new(plain_title_course());
    prep(&mut h);
    enter_title_edit(&mut h);
    let frame_before = h
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object("title"))
        .expect("title")
        .0
        .frame;
    let slide_before = h.editor.ui.last_slide_rect.expect("slide rect").min;
    let start = object_center(&h, "s-intro", "title");

    h.frame(vec![Event::Key {
        key: Key::Space,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    h.frame(vec![Event::PointerMoved(start)]);
    h.frame(vec![Event::PointerButton {
        pos: start,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    }]);
    let end = start + vec2(-60.0, -40.0);
    h.frame(vec![Event::PointerMoved(end)]);
    h.frame(vec![Event::PointerButton {
        pos: end,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    }]);
    h.frame(vec![Event::Key {
        key: Key::Space,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    h.idle_frame();

    let frame_after = h
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object("title"))
        .expect("title")
        .0
        .frame;
    assert_eq!(frame_after, frame_before, "object did not move");
    let slide_after = h.editor.ui.last_slide_rect.expect("slide rect").min;
    assert!(
        (slide_after - slide_before).length() < 0.5,
        "canvas did not pan"
    );
}

#[test]
#[ignore = "BUG: Cmd+B with a caret and no selection bolds the whole text box (apply_style_to_range treats an empty range as the whole text)"]
fn cmd_b_with_caret_only_does_not_bold_the_existing_text() {
    // Expected: with no selection, Cmd+B changes the typing style only. The existing
    // text keeps its weight. Typing after it is the only thing that should turn bold.
    let mut h = TestHarness::new(plain_title_course());
    prep(&mut h);
    enter_title_edit(&mut h);
    h.press(Key::ArrowRight, Modifiers::NONE);
    h.idle_frame();
    assert_eq!(bold_chars(&h), 0);

    h.press(Key::B, Modifiers::COMMAND);
    h.idle_frame();

    assert_eq!(
        bold_chars(&h),
        0,
        "Cmd+B with a caret and no selection must not bold the existing text"
    );
}

#[test]
fn cmd_b_with_partial_selection_bolds_only_the_selected_range() {
    let mut h = TestHarness::new(plain_title_course());
    prep(&mut h);
    enter_title_edit(&mut h);
    let (text, _) = title_text_and_bold(&h);
    assert!(text.chars().count() > 6, "title is long enough: {text:?}");

    h.press(Key::Home, Modifiers::NONE);
    for _ in 0..5 {
        h.press(Key::ArrowRight, Modifiers::SHIFT);
    }
    h.press(Key::B, Modifiers::COMMAND);
    h.idle_frame();

    let (_, parts) = title_text_and_bold(&h);
    let first: usize = parts.first().map(|(n, _)| *n).unwrap_or(0);
    assert!(first >= 5, "first five characters are bold: {parts:?}");
    assert_eq!(
        bold_chars(&h),
        5,
        "only the selected five characters are bold"
    );
}

#[test]
fn cmd_b_then_undo_restores_the_course_exactly() {
    let mut h = TestHarness::new(plain_title_course());
    prep(&mut h);
    enter_title_edit(&mut h);
    let before = serde_json::to_value(&h.editor.session.course).expect("json");
    let undo_before = h.editor.session.undo_len();

    h.press(Key::Home, Modifiers::NONE);
    for _ in 0..5 {
        h.press(Key::ArrowRight, Modifiers::SHIFT);
    }
    h.press(Key::B, Modifiers::COMMAND);
    h.idle_frame();
    assert_eq!(
        h.editor.session.undo_len(),
        undo_before + 1,
        "one style step"
    );

    h.editor.session.undo().expect("undo");
    h.idle_frame();
    let after = serde_json::to_value(&h.editor.session.course).expect("json");
    assert_eq!(after, before, "undo restores the course exactly");
}

#[test]
fn cmd_b_on_an_object_not_being_edited_changes_nothing() {
    let mut h = TestHarness::new(hello());
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.object = Some("btn-start".into());
    h.idle_frame();
    h.idle_frame();
    let before = serde_json::to_value(&h.editor.session.course).expect("json");
    let undo_before = h.editor.session.undo_len();

    h.press(Key::B, Modifiers::COMMAND);
    h.press(Key::I, Modifiers::COMMAND);
    h.press(Key::U, Modifiers::COMMAND);
    h.idle_frame();

    assert!(h.editor.ui.inline_edit.is_none());
    assert_eq!(h.editor.session.undo_len(), undo_before);
    assert_eq!(
        serde_json::to_value(&h.editor.session.course).expect("json"),
        before
    );
}

#[test]
#[ignore = "BUG (unconfirmed, low confidence): middle-button drag does not pan the canvas in the harness; not yet ruled out as a harness artefact"]
fn middle_mouse_drag_pans_without_moving_the_object() {
    let mut h = TestHarness::new(hello());
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.object = Some("btn-start".into());
    h.idle_frame();
    h.idle_frame();
    let slide_before = h.editor.ui.last_slide_rect.expect("slide rect").min;
    let frame_before = h
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object("btn-start"))
        .expect("btn")
        .0
        .frame;
    let start = h.editor.ui.last_slide_rect.expect("slide rect").center();
    let undo_before = h.editor.session.undo_len();

    h.frame(vec![Event::PointerMoved(start)]);
    h.frame(vec![Event::PointerButton {
        pos: start,
        button: PointerButton::Middle,
        pressed: true,
        modifiers: Modifiers::NONE,
    }]);
    let end = start + vec2(-50.0, -30.0);
    h.frame(vec![Event::PointerMoved(end)]);
    h.frame(vec![Event::PointerButton {
        pos: end,
        button: PointerButton::Middle,
        pressed: false,
        modifiers: Modifiers::NONE,
    }]);
    h.idle_frame();

    let slide_after = h.editor.ui.last_slide_rect.expect("slide rect").min;
    assert!(
        (slide_after - slide_before).length() > 5.0,
        "middle drag panned the canvas: before={slide_before:?} after={slide_after:?}"
    );
    let frame_after = h
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object("btn-start"))
        .expect("btn")
        .0
        .frame;
    assert_eq!(frame_after, frame_before, "object did not move");
    assert_eq!(
        h.editor.session.undo_len(),
        undo_before,
        "pan adds no undo step"
    );
}
