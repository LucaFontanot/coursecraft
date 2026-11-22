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

#[test]
fn colour_picker_popup_clamps_inside_screen_when_swatch_near_bottom_right() {
    let mut harness = TestHarness::new_with_size(course_with_shape(), (900.0, 520.0));
    harness.editor.selection.slide = "sl1".into();
    harness.editor.selection.object = Some("sh1".into());
    harness.editor.ui.inspector_tab = crate::properties::InspectorTab::Format;
    harness.idle_frame();
    harness.idle_frame();

    // Scroll inspector down until `color_rect` sits near the bottom-right of the screen.
    let panel = harness.right_panel_rect();
    for _ in 0..8 {
        if let Some(r) = harness.editor.ui.properties.color_rect
            && r.center().y <= panel.bottom() - 12.0
            && r.center().y >= panel.bottom() - 90.0
        {
            break;
        }
        harness.scroll_over(panel.center(), -60.0);
    }
    let hex_field = harness
        .editor
        .ui
        .properties
        .color_rect
        .expect("shape colour hex field");
    assert!(
        panel.contains(hex_field.center()),
        "hex_field {hex_field:?} should be visible in {panel:?}"
    );
    let swatch = egui::pos2(hex_field.left() - 15.0, hex_field.center().y);
    harness.click_at(swatch);
    harness.idle_frame();
    harness.idle_frame();

    let shapes = harness.frame_shapes(vec![]);
    let eyedropper_pos = harness
        .find_text_containing("Eyedropper")
        .expect("Eyedropper button in colour popup");
    let mut popup_rect = egui::Rect::NOTHING;
    for cs in &shapes {
        let mut rects = Vec::new();
        fn collect_rects(s: &egui::epaint::Shape, out: &mut Vec<egui::Rect>) {
            match s {
                egui::epaint::Shape::Rect(r) => out.push(r.rect),
                egui::epaint::Shape::Vec(v) => {
                    for child in v {
                        collect_rects(child, out);
                    }
                }
                _ => {}
            }
        }
        collect_rects(&cs.shape, &mut rects);
        for r in rects {
            if r.contains(eyedropper_pos)
                && r.width() >= 180.0
                && r.width() <= 400.0
                && r.height() >= 150.0
                && r.height() <= 520.0
            {
                popup_rect = r;
            }
        }
    }
    assert!(
        popup_rect.is_positive(),
        "colour picker popup rect not found"
    );
    let screen = harness.screen_rect;
    assert!(
        popup_rect.min.x >= screen.min.x - 1.0
            && popup_rect.min.y >= screen.min.y - 1.0
            && popup_rect.max.x <= screen.max.x + 1.0
            && popup_rect.max.y <= screen.max.y + 1.0,
        "colour picker popup {popup_rect:?} should clamp inside screen {screen:?}"
    );
    assert!(harness.find_text("Theme").is_some());
}

#[test]
fn colour_picker_hsv_drag_is_one_undo_step_and_invalid_hex_gives_feedback() {
    let course = Course::from_json(include_str!("../../../examples/hello-course/course.json"))
        .expect("course");
    let mut harness = TestHarness::new(course);
    harness.editor.selection.slide = "s-intro".into();
    harness.editor.selection.object = Some("btn-start".into());
    harness.editor.ui.inspector_tab = crate::properties::InspectorTab::Format;
    harness.idle_frame();
    harness.idle_frame();

    let btn_fill = |h: &TestHarness| -> String {
        h.editor
            .session
            .course
            .slide("s-intro")
            .and_then(|s| s.object("btn-start"))
            .and_then(|(o, _)| match &o.kind {
                ObjectKind::Button { fill, .. } => Some(fill.0.clone()),
                _ => None,
            })
            .unwrap_or_default()
    };

    let hex_field = harness
        .editor
        .ui
        .properties
        .color_rect
        .expect("button fill hex field");
    let swatch = egui::pos2(hex_field.left() - 15.0, hex_field.center().y);
    let fill_before = btn_fill(&harness);
    let rev_before = harness.editor.session.revision;

    // Open colour picker and drag inside the HSV area (below the "Theme" row).
    harness.click_at(swatch);
    harness.idle_frame();
    let theme_pos = harness
        .find_text("Theme")
        .expect("Theme row in colour picker");
    let hsv_start = egui::pos2(theme_pos.x + 60.0, theme_pos.y + 60.0);
    harness.drag_by(hsv_start, egui::vec2(40.0, 20.0), 10, Modifiers::NONE);
    harness.idle_frame();

    let fill_after_drag = btn_fill(&harness);
    assert_ne!(fill_after_drag, fill_before, "HSV drag should update fill");
    assert_eq!(
        harness.editor.session.revision,
        rev_before + 1,
        "10-frame HSV drag must commit exactly 1 undo step on release"
    );

    // Undo with Ctrl+Z restores previous colour.
    harness.press(Key::Escape, Modifiers::NONE);
    harness.idle_frame();
    harness.press(Key::Z, Modifiers::COMMAND);
    harness.idle_frame();
    assert_eq!(btn_fill(&harness), fill_before);

    // Type invalid hex "#zzz" -> no crash, clear feedback, then valid "#2288CC" -> updates.
    let hex_field = harness
        .editor
        .ui
        .properties
        .color_rect
        .expect("button fill hex field");
    harness.set_field(hex_field.center(), "#zzz");
    harness.idle_frame();
    assert_eq!(btn_fill(&harness), fill_before);
    assert_eq!(
        harness.editor.ui.properties.button_fill, "#zzz",
        "button_fill should hold #zzz"
    );
    harness.hover_at(egui::pos2(10.0, 10.0));
    let tip = harness.probe_tooltip(hex_field.center());
    assert!(
        tip.as_deref()
            .is_some_and(|t| t.to_ascii_lowercase().contains("hex")
                || t.to_ascii_lowercase().contains("invalid")),
        "invalid hex field should show clear feedback tooltip, got {tip:?}"
    );

    harness.set_field(hex_field.center(), "#2288CC");
    harness.idle_frame();
    assert_eq!(btn_fill(&harness), "#2288CC");
}

#[test]
fn eyedropper_loupe_stays_inside_screen_near_edges_and_esc_cancels() {
    let mut harness = selected_shape(TestHarness::new(course_with_shape()));
    harness.editor.ui.inspector_tab = crate::properties::InspectorTab::Format;
    harness.idle_frame();
    harness.idle_frame();

    let panel = harness.right_panel_rect();
    for _ in 0..6 {
        if let Some(r) = harness.editor.ui.properties.color_rect
            && r.center().y <= panel.bottom() - 16.0
        {
            break;
        }
        harness.scroll_over(panel.center(), -80.0);
    }
    let hex_field = harness
        .editor
        .ui
        .properties
        .color_rect
        .expect("shape colour hex field");
    let swatch = egui::pos2(hex_field.left() - 15.0, hex_field.center().y);
    harness.click_at(swatch);
    harness.idle_frame();
    let button = harness
        .find_text_containing("Eyedropper")
        .expect("Eyedropper button");
    harness.click_at(button);
    harness.idle_frame();

    // Colour picker closes immediately.
    assert!(!egui::Popup::is_any_open(&harness.ctx));
    assert!(eyedropper::is_active(&harness.ctx));

    // Move pointer near bottom-right corner and check sampling loupe stays inside screen.
    eyedropper::store_screenshot(&harness.ctx, uniform_screenshot());
    let corner = egui::pos2(
        harness.screen_rect.max.x - 8.0,
        harness.screen_rect.max.y - 8.0,
    );
    harness.frame(vec![Event::PointerMoved(corner)]);
    let shapes = harness.frame_shapes(vec![]);
    let hex_pos = harness
        .find_text("#123456")
        .expect("sampling loupe hex readout");
    let mut loupe_rect = egui::Rect::NOTHING;
    for cs in &shapes {
        if let egui::epaint::Shape::Rect(r) = &cs.shape
            && r.rect.contains(hex_pos)
            && r.rect.width() >= 40.0
            && r.rect.width() < 250.0
        {
            loupe_rect = r.rect;
        }
    }
    assert!(loupe_rect.is_positive(), "loupe rect not found");
    let screen = harness.screen_rect;
    assert!(
        loupe_rect.max.x <= screen.max.x + 1.0 && loupe_rect.max.y <= screen.max.y + 1.0,
        "eyedropper loupe {loupe_rect:?} must stay inside screen {screen:?}"
    );

    // Esc cancels cleanly without modifying colour or clearing selection.
    let fill_before = shape_fill(&harness);
    harness.press(Key::Escape, Modifiers::NONE);
    harness.idle_frame();
    assert!(!eyedropper::is_active(&harness.ctx));
    assert_eq!(shape_fill(&harness), fill_before);
    assert_eq!(harness.editor.selection.object.as_deref(), Some("sh1"));
}
