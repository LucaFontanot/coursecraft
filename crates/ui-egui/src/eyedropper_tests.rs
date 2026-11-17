//! Headless tests for the eyedropper in the shared colour picker.

use crate::eyedropper;
use crate::test_harness::TestHarness;
use crate::{canvas_fit_rect, canvas_rect};
use coursecraft_course::{Course, ObjectKind};
use coursecraft_render::SlideTransform;
use egui::{Color32, ColorImage, Event, Key, Modifiers, PointerButton, pos2};
use std::sync::Arc;

fn course_with_shape() -> Course {
    let json = serde_json::json!({
        "schema_version": 1,
        "id": "eyedropper-course",
        "title": "Colour pick course",
        "slide_size": { "w": 1000, "h": 600 },
        "scenes": [{
            "id": "sc1",
            "title": "Scene 1",
            "slides": [{
                "id": "sl1",
                "title": "Slide 1",
                "duration_ms": 5000,
                "objects": [{
                    "id": "sh1",
                    "kind": { "type": "shape", "shape": "rectangle", "fill": "#112233" },
                    "frame": { "x": 100, "y": 100, "w": 200, "h": 120 }
                }]
            }]
        }]
    });
    Course::from_json(&json.to_string()).expect("valid course")
}

fn shape_fill(harness: &TestHarness) -> String {
    harness
        .editor
        .session
        .course
        .slide("sl1")
        .and_then(|slide| slide.object("sh1"))
        .and_then(|(obj, _)| match &obj.kind {
            ObjectKind::Shape { fill, .. } => Some(fill.0.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

fn selected_shape(mut harness: TestHarness) -> TestHarness {
    harness.editor.selection.slide = "sl1".into();
    harness.editor.selection.object = Some("sh1".into());
    harness
}

fn empty_canvas_point(harness: &TestHarness) -> egui::Pos2 {
    let rect = canvas_fit_rect(canvas_rect(harness.central_panel_rect()));
    let transform = SlideTransform::fit(rect, harness.editor.session.course.slide_size)
        .expect("slide transform");
    transform.to_screen_pos(pos2(700.0, 400.0))
}

fn uniform_screenshot() -> Arc<ColorImage> {
    Arc::new(ColorImage::new(
        [64, 64],
        vec![Color32::from_rgb(0x12, 0x34, 0x56); 64 * 64],
    ))
}

#[test]
fn picking_a_colour_changes_the_shape_fill_in_one_undo_step_and_keeps_selection() {
    let mut harness = selected_shape(TestHarness::new(course_with_shape()));
    harness.idle_frame();
    let empty = empty_canvas_point(&harness);
    let rev_before = harness.editor.session.revision;

    eyedropper::start_pick(&harness.ctx, eyedropper::field_key(&"prop_shape_fill"));
    eyedropper::store_screenshot(&harness.ctx, uniform_screenshot());
    assert!(eyedropper::is_active(&harness.ctx));
    harness.idle_frame();

    harness.frame(vec![Event::PointerMoved(empty)]);
    harness.frame(vec![Event::PointerButton {
        pos: empty,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    }]);
    harness.frame(vec![Event::PointerButton {
        pos: empty,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    }]);
    harness.idle_frame();

    assert!(!eyedropper::is_active(&harness.ctx));
    assert_eq!(shape_fill(&harness), "#123456");
    assert_eq!(harness.editor.session.revision, rev_before + 1);
    assert_eq!(harness.editor.selection.object.as_deref(), Some("sh1"));

    // Input ownership is handed back: a later canvas click can deselect again.
    harness.click_at(empty);
    harness.idle_frame();
    assert_eq!(harness.editor.selection.object, None);
}

#[test]
fn escape_cancels_pick_and_leaves_the_fill_and_selection_untouched() {
    let mut harness = selected_shape(TestHarness::new(course_with_shape()));
    harness.idle_frame();
    let empty = empty_canvas_point(&harness);
    let rev_before = harness.editor.session.revision;
    let fill_before = shape_fill(&harness);

    eyedropper::start_pick(&harness.ctx, eyedropper::field_key(&"prop_shape_fill"));
    eyedropper::store_screenshot(&harness.ctx, uniform_screenshot());
    harness.idle_frame();
    harness.press(Key::Escape, Modifiers::NONE);
    harness.idle_frame();

    assert!(!eyedropper::is_active(&harness.ctx));
    assert_eq!(shape_fill(&harness), fill_before);
    assert_eq!(harness.editor.session.revision, rev_before);
    assert_eq!(harness.editor.selection.object.as_deref(), Some("sh1"));

    // A later click must not apply a stale pick.
    harness.frame(vec![Event::PointerMoved(empty)]);
    harness.frame(vec![Event::PointerButton {
        pos: empty,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    }]);
    harness.frame(vec![Event::PointerButton {
        pos: empty,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    }]);
    harness.idle_frame();
    assert_eq!(shape_fill(&harness), fill_before);
}

#[test]
fn picking_blocks_toolbar_clicks_and_still_applies_the_colour() {
    let mut harness = selected_shape(TestHarness::new(course_with_shape()));
    harness.idle_frame();

    // A labelled toolbar toggle with an observable effect.
    let story = harness
        .find_text("Story")
        .expect("the Story toolbar toggle");
    let story_before = harness.editor.ui.story_view;

    // Start a pick and give the full-window blocker a frame to register.
    eyedropper::start_pick(&harness.ctx, eyedropper::field_key(&"prop_shape_fill"));
    eyedropper::store_screenshot(&harness.ctx, uniform_screenshot());
    harness.idle_frame();
    assert!(eyedropper::is_active(&harness.ctx));

    // A click on the toolbar button while picking must not press that button ...
    harness.click_at(story);
    harness.idle_frame();
    assert_eq!(harness.editor.ui.story_view, story_before);

    // ... but it still picks the colour sampled from the screenshot.
    assert!(!eyedropper::is_active(&harness.ctx));
    assert_eq!(shape_fill(&harness), "#123456");

    // Escape still cancels a fresh pick while the blocker is up.
    let fill_before = shape_fill(&harness);
    let rev_before = harness.editor.session.revision;
    eyedropper::start_pick(&harness.ctx, eyedropper::field_key(&"prop_shape_fill"));
    eyedropper::store_screenshot(&harness.ctx, uniform_screenshot());
    harness.idle_frame();
    harness.press(Key::Escape, Modifiers::NONE);
    harness.idle_frame();
    assert!(!eyedropper::is_active(&harness.ctx));
    assert_eq!(shape_fill(&harness), fill_before);
    assert_eq!(harness.editor.session.revision, rev_before);

    // Once pick mode ends the very same button works again, so the earlier
    // non-trigger was the blocker, not a mis-aimed click.
    harness.click_at(story);
    harness.idle_frame();
    assert_eq!(harness.editor.ui.story_view, !story_before);
}

#[test]
fn eyedropper_button_in_the_popup_starts_pick_mode_for_that_field() {
    let course = Course::from_json(include_str!("../../../examples/hello-course/course.json"))
        .expect("course");
    let mut harness = TestHarness::new(course);
    harness.editor.selection.slide = "s-intro".into();
    harness.editor.selection.object = Some("btn-start".into());
    harness.idle_frame();
    // `color_rect` tracks the hex text field; the 18 px swatch sits just left of it.
    let hex_field = harness
        .editor
        .ui
        .properties
        .color_rect
        .expect("the selected button's fill field");
    let swatch = egui::pos2(hex_field.left() - 15.0, hex_field.center().y);
    harness.click_at(swatch);
    harness.idle_frame();
    let button = harness
        .find_text_containing("Eyedropper")
        .expect("the eyedropper button in the colour popup");
    harness.click_at(button);
    harness.idle_frame();

    assert!(eyedropper::is_active(&harness.ctx));
    let state_field = eyedropper::active_field(&harness.ctx);
    assert_eq!(state_field, Some(eyedropper::field_key(&"prop_btn_fill")));
}
