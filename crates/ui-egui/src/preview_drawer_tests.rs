//! Headless tests for the preview outline drawer placement and clipping.

use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use egui::{Color32, Event, Key, Modifiers, Rect, pos2};

fn multi_slide_course() -> Course {
    let json = r#"{
        "schema_version": 1,
        "id": "preview-drawer-test",
        "title": "Preview Drawer Test",
        "slide_size": { "w": 1280, "h": 720 },
        "variables": [
            { "name": "score", "value": 0 }
        ],
        "resources": [
            { "id": "res-1", "title": "Guide", "target": { "kind": "url", "url": "https://example.com" } }
        ],
        "glossary": [
            { "id": "gl-1", "term": "Term", "definition": "Definition" }
        ],
        "player": { "show_transcript": true },
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
                    },
                    {
                        "id": "s-2",
                        "title": "Slide 2",
                        "duration_ms": 5000,
                        "objects": [],
                        "triggers": []
                    }
                ]
            }
        ]
    }"#;
    Course::from_json(json).expect("valid course")
}

fn open_preview_and_focus(harness: &mut TestHarness) -> Rect {
    harness.editor.open_preview();
    for _ in 0..3 {
        harness.idle_frame();
    }
    let window_rect = harness
        .editor
        .preview
        .as_ref()
        .and_then(|p| p.window_rect)
        .expect("preview window_rect");
    // Click inside the preview so keyboard focus belongs to it.
    let inside = pos2(window_rect.center().x, window_rect.center().y);
    harness.frame(vec![
        Event::PointerMoved(inside),
        Event::PointerButton {
            pos: inside,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
        Event::PointerButton {
            pos: inside,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        },
    ]);
    window_rect
}

fn assert_drawer_clipped_to_window(
    shapes: &[egui::epaint::ClippedShape],
    window_rect: Rect,
    phase: &str,
) {
    let drawer_bg = Color32::from_rgb(0x2F, 0x2F, 0x33);
    let mut saw_drawer = false;
    for clipped in shapes {
        let is_drawer_bg = matches!(
            &clipped.shape,
            egui::epaint::Shape::Rect(r) if r.fill == drawer_bg && r.rect.height() > 100.0
        );
        let is_drawer_text = matches!(
            &clipped.shape,
            egui::epaint::Shape::Text(t)
                if t.galley.text() == "Preview Drawer Test"
                    && t.pos.y >= window_rect.top() - 0.5
                    && t.pos.y <= window_rect.bottom() + 0.5
        );
        if is_drawer_bg || is_drawer_text {
            saw_drawer = true;
            assert!(
                clipped.clip_rect.left() >= window_rect.left() - 0.5
                    && clipped.clip_rect.right() <= window_rect.right() + 0.5
                    && clipped.clip_rect.top() >= window_rect.top() - 0.5
                    && clipped.clip_rect.bottom() <= window_rect.bottom() + 0.5,
                "{phase}: drawer shape clip_rect {:?} escaped preview window {:?}",
                clipped.clip_rect,
                window_rect
            );
            if let egui::epaint::Shape::Rect(r) = &clipped.shape {
                assert!(
                    r.rect.left() >= window_rect.left() - 0.5
                        && r.rect.right() <= window_rect.right() + 0.5,
                    "{phase}: drawer rect {:?} escaped preview window {:?}",
                    r.rect,
                    window_rect
                );
            }
        }
    }
    assert!(saw_drawer, "{phase}: expected drawer to be painted");
}

#[test]
fn preview_drawer_clips_to_preview_window_across_sizes_and_animation() {
    for (w, h) in [(1280.0, 900.0), (1600.0, 1000.0), (1000.0, 700.0)] {
        let mut harness = TestHarness::new_with_size(multi_slide_course(), (w, h));
        let window_rect = open_preview_and_focus(&mut harness);
        assert!(
            window_rect.left() > 10.0,
            "preview window {window_rect:?} should not be at screen left edge for ({w}, {h})"
        );

        // Press M to open the outline drawer (first animation frame).
        harness.advance_time(0.02);
        let shapes_first = harness.frame_shapes(vec![Event::Key {
            key: Key::M,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }]);
        assert_drawer_clipped_to_window(
            &shapes_first,
            window_rect,
            &format!("first frame ({w}, {h})"),
        );

        // Middle of animation.
        harness.advance_time(0.10);
        let shapes_mid = harness.frame_shapes(Vec::new());
        assert_drawer_clipped_to_window(&shapes_mid, window_rect, &format!("mid ({w}, {h})"));

        // End of animation.
        harness.advance_time(0.25);
        let shapes_end = harness.frame_shapes(Vec::new());
        assert_drawer_clipped_to_window(&shapes_end, window_rect, &format!("end ({w}, {h})"));

        let drawer_rect = harness
            .editor
            .preview
            .as_ref()
            .and_then(|p| p.frame.drawer_rect())
            .expect("drawer_rect at end");
        assert!(
            drawer_rect.left() >= window_rect.left() - 0.5,
            "settled drawer_rect {drawer_rect:?} left of window {window_rect:?}"
        );
    }
}

#[test]
fn preview_drawer_with_variables_panel_stays_inside_window() {
    let mut harness = TestHarness::new_with_size(multi_slide_course(), (1280.0, 900.0));
    let window_rect = open_preview_and_focus(&mut harness);

    // Toggle variables panel with F3, then open drawer with M.
    harness.frame(vec![Event::Key {
        key: Key::F3,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    harness.advance_time(0.02);
    let shapes_first = harness.frame_shapes(vec![Event::Key {
        key: Key::M,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    assert_drawer_clipped_to_window(&shapes_first, window_rect, "vars first frame");

    harness.advance_time(0.10);
    let shapes_mid = harness.frame_shapes(Vec::new());
    assert_drawer_clipped_to_window(&shapes_mid, window_rect, "vars mid frame");

    harness.advance_time(0.25);
    let shapes_end = harness.frame_shapes(Vec::new());
    assert_drawer_clipped_to_window(&shapes_end, window_rect, "vars end frame");
}
