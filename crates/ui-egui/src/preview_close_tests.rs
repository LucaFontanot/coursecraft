//! Headless tests for closing the Preview modal by clicking outside or pressing Esc.

use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use egui::{Key, Modifiers, pos2};

fn simple_course() -> Course {
    let json = r#"{
        "schema_version": 1,
        "id": "preview-close-test",
        "title": "Preview Close Test",
        "slide_size": { "w": 1280, "h": 720 },
        "variables": [
            { "name": "answer", "value": "" }
        ],
        "scenes": [
            {
                "id": "sc-1",
                "title": "Main",
                "slides": [
                    {
                        "id": "s-1",
                        "title": "Slide 1",
                        "duration_ms": 5000,
                        "objects": [
                            {
                                "id": "txt-1",
                                "kind": { "type": "text", "text": "Hello Preview" },
                                "frame": { "x": 80, "y": 80, "w": 400, "h": 60 }
                            }
                        ],
                        "triggers": []
                    }
                ]
            }
        ]
    }"#;
    Course::from_json(json).expect("valid course")
}

fn text_input_course() -> Course {
    let json = r#"{
        "schema_version": 1,
        "id": "preview-close-input-test",
        "title": "Preview Close Input Test",
        "slide_size": { "w": 1280, "h": 720 },
        "variables": [
            { "name": "answer", "value": "" }
        ],
        "scenes": [
            {
                "id": "sc-1",
                "title": "Main",
                "slides": [
                    {
                        "id": "s-1",
                        "title": "Slide 1",
                        "duration_ms": 5000,
                        "objects": [
                            {
                                "id": "input-1",
                                "kind": {
                                    "type": "text_entry",
                                    "variable": "answer",
                                    "placeholder": "Type answer here",
                                    "multiline": false
                                },
                                "frame": { "x": 100, "y": 100, "w": 400, "h": 60 }
                            }
                        ],
                        "triggers": []
                    }
                ]
            }
        ]
    }"#;
    Course::from_json(json).expect("valid course")
}

fn open_preview(harness: &mut TestHarness) -> egui::Rect {
    harness.editor.open_preview();
    for _ in 0..3 {
        harness.idle_frame();
    }
    harness
        .editor
        .preview
        .as_ref()
        .and_then(|p| p.window_rect)
        .expect("preview window_rect")
}

#[test]
fn clicking_outside_preview_closes_it() {
    let mut harness = TestHarness::new_with_size(simple_course(), (1280.0, 800.0));
    let rect = open_preview(&mut harness);
    let outside = pos2(5.0, 5.0);
    assert!(
        !rect.contains(outside),
        "test point {outside:?} must be outside preview rect {rect:?}"
    );

    harness.click_at(outside);
    harness.idle_frame();

    assert!(
        harness.editor.preview.is_none(),
        "clicking outside the preview on the dimmed backdrop should close it"
    );
}

#[test]
fn clicking_inside_preview_does_not_close_it() {
    let mut harness = TestHarness::new_with_size(simple_course(), (1280.0, 800.0));
    let rect = open_preview(&mut harness);
    let inside = rect.center();
    assert!(rect.contains(inside));

    harness.click_at(inside);
    harness.idle_frame();

    assert!(
        harness.editor.preview.is_some(),
        "clicking inside the preview must never close it"
    );
}

#[test]
fn pressing_escape_closes_preview() {
    let mut harness = TestHarness::new_with_size(simple_course(), (1280.0, 800.0));
    open_preview(&mut harness);

    harness.press(Key::Escape, Modifiers::NONE);
    harness.idle_frame();

    assert!(
        harness.editor.preview.is_none(),
        "pressing Esc should close the preview"
    );
}

#[test]
fn pressing_escape_does_not_close_preview_when_text_input_has_focus() {
    let mut harness = TestHarness::new_with_size(text_input_course(), (1280.0, 800.0));
    open_preview(&mut harness);

    let input_pos = harness
        .find_text_last("Type answer here")
        .expect("text input placeholder in preview");
    harness.click_at(input_pos);
    harness.idle_frame();
    assert!(
        harness.ctx.egui_wants_keyboard_input(),
        "text input should have keyboard focus"
    );

    harness.press(Key::Escape, Modifiers::NONE);
    harness.idle_frame();

    assert!(
        harness.editor.preview.is_some(),
        "Esc while a text input inside the preview has focus should not close the preview"
    );
    assert!(
        !harness.ctx.egui_wants_keyboard_input(),
        "the first Esc should surrender focus from the text input"
    );

    harness.press(Key::Escape, Modifiers::NONE);
    harness.idle_frame();

    assert!(
        harness.editor.preview.is_none(),
        "a second Esc after the text input lost focus should close the preview"
    );
}

#[test]
fn clicking_outside_reopened_preview_closes_it_from_any_side() {
    let mut harness = TestHarness::new_with_size(simple_course(), (1280.0, 800.0));
    let rect = open_preview(&mut harness);
    let bottom_right = pos2(1275.0, 795.0);
    assert!(!rect.contains(bottom_right));

    harness.click_at(bottom_right);
    harness.idle_frame();
    assert!(
        harness.editor.preview.is_none(),
        "clicking bottom-right outside the preview should close it"
    );

    // Reopen and verify inside click still keeps it open, then Esc closes it.
    let rect = open_preview(&mut harness);
    let title_pos = harness
        .find_text_in("Hello Preview", rect)
        .expect("slide text in reopened preview");
    harness.click_at(title_pos);
    harness.idle_frame();
    assert!(harness.editor.preview.is_some());

    harness.press(Key::Escape, Modifiers::NONE);
    harness.idle_frame();
    assert!(harness.editor.preview.is_none());
}
