//! Headless harness test for equal-spacing guides while dragging on the canvas.

use crate::canvas::object_screen_rect;
use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use egui::{Modifiers, vec2};

fn minimal_course() -> Course {
    let json = serde_json::json!({
        "schema_version": 1,
        "id": "spacing-test",
        "title": "Spacing Test",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "scene-1",
            "title": "Scene 1",
            "slides": [{
                "id": "s-1",
                "title": "Slide 1",
                "objects": [
                    {
                        "id": "box-left",
                        "kind": { "type": "shape", "shape": "rectangle", "fill": "#D0D7DE" },
                        "frame": { "x": 200, "y": 300, "w": 60, "h": 40 }
                    },
                    {
                        "id": "box-mid",
                        "kind": { "type": "shape", "shape": "rectangle", "fill": "#D0D7DE" },
                        "frame": { "x": 340, "y": 300, "w": 60, "h": 40 }
                    },
                    {
                        "id": "box-right",
                        "kind": { "type": "shape", "shape": "rectangle", "fill": "#D0D7DE" },
                        "frame": { "x": 500, "y": 300, "w": 60, "h": 40 }
                    }
                ]
            }]
        }]
    })
    .to_string();
    Course::from_json(&json).expect("valid course")
}

#[test]
fn dragging_a_box_between_two_others_snaps_to_equal_spacing() {
    let mut harness = TestHarness::new(minimal_course());
    harness.editor.selection.set_many(["box-mid"]);
    harness.editor.ui.snap_to_guides = true;
    harness.idle_frame();

    let central = harness.central_panel_rect();
    let mid_rect = object_screen_rect(&harness.editor.session.course, "s-1", "box-mid", central)
        .expect("box-mid screen rect");
    let left_rect = object_screen_rect(&harness.editor.session.course, "s-1", "box-left", central)
        .expect("box-left screen rect");
    let right_rect =
        object_screen_rect(&harness.editor.session.course, "s-1", "box-right", central)
            .expect("box-right screen rect");

    // Equal gap puts box-mid centred between left_rect.right() and right_rect.left().
    let target_center_x = (left_rect.right() + right_rect.left()) * 0.5;
    // Drag to 2 screen px left of the exact midpoint, well within the 6 px snap threshold.
    let delta = vec2((target_center_x - 2.0) - mid_rect.center().x, 0.0);
    harness.drag_by(mid_rect.center(), delta, 4, Modifiers::NONE);

    let final_x = harness
        .editor
        .session
        .course
        .slide("s-1")
        .and_then(|s| s.object("box-mid"))
        .map(|(obj, _)| obj.frame.x)
        .expect("box-mid");
    assert_eq!(final_x, 350.0);
}
