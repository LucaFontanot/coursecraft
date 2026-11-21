//! Headless UI tests for the "Base slide: Show / Dim / Hide" view toggle
//! shown next to the layer pills when a layer tab is active.

use crate::test_harness::TestHarness;
use coursecraft_course::{BaseSlideView, Course};
use egui::Color32;
use egui::epaint::{ClippedShape, Shape};

fn hello() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    Course::from_json(&json).expect("valid course")
}

/// Find the foreground color of any painted text containing `needle` inside `in_rect`.
fn text_color_in(shapes: &[ClippedShape], needle: &str, in_rect: egui::Rect) -> Option<Color32> {
    fn walk(shape: &Shape, needle: &str, in_rect: egui::Rect) -> Option<Color32> {
        match shape {
            Shape::Text(t) if t.galley.text().contains(needle) => {
                let center = crate::test_harness::text_center(t);
                if !in_rect.contains(center) {
                    return None;
                }
                let color = t
                    .galley
                    .job
                    .sections
                    .first()
                    .map(|s| s.format.color)
                    .unwrap_or_else(|| t.override_text_color.unwrap_or(Color32::TRANSPARENT));
                Some(color)
            }
            Shape::Vec(v) => v.iter().find_map(|s| walk(s, needle, in_rect)),
            _ => None,
        }
    }
    shapes
        .iter()
        .find_map(|cs| walk(&cs.shape, needle, in_rect))
}

#[test]
fn base_slide_toggle_controls_painting_and_selection_when_layer_active() {
    let mut harness = TestHarness::new(hello());
    harness.editor.selection.slide = "s-intro".into();
    harness.idle_frame();
    harness.idle_frame();

    let canvas = harness.central_panel_rect();
    // On Base, the toggle is hidden and the base button "Start" is painted at full alpha.
    assert!(
        harness.find_text_in("Base slide:", canvas).is_none(),
        "toggle should not show on Base"
    );
    let base_shapes = harness.frame_shapes(vec![]);
    let full_color = text_color_in(&base_shapes, "Start", canvas).expect("Start on Base");
    let start_pos = harness
        .find_text_in("Start", canvas)
        .expect("Start position on canvas");

    // Click the "About" layer pill above the slide.
    let about_pill = harness
        .find_text_in("About", canvas)
        .expect("About layer pill");
    harness.click_at(about_pill);
    harness.idle_frame();
    assert_eq!(harness.editor.selection.layer.as_deref(), Some("about"));
    assert_eq!(
        harness.editor.session.course.editor.base_slide_view,
        BaseSlideView::Dim
    );

    // Toggle appears next to the layer pills; Dim is default and reduces alpha (~35%).
    assert!(harness.find_text_in("Base slide:", canvas).is_some());
    let dim_shapes = harness.frame_shapes(vec![]);
    let dim_color = text_color_in(&dim_shapes, "Start", canvas).expect("Start dimmed");
    assert!(
        dim_color.a() < full_color.a(),
        "Dim should reduce alpha: full={}, dim={}",
        full_color.a(),
        dim_color.a()
    );

    // Base object is not selectable while editing a layer in Dim.
    harness.click_at(start_pos);
    harness.idle_frame();
    assert_eq!(harness.editor.selection.object, None);

    // Choose Hide: base objects are not painted and still not selectable.
    let hide_btn = harness.find_text_in("Hide", canvas).expect("Hide toggle");
    harness.click_at(hide_btn);
    harness.idle_frame();
    assert_eq!(
        harness.editor.session.course.editor.base_slide_view,
        BaseSlideView::Hide
    );
    let hide_shapes = harness.frame_shapes(vec![]);
    assert!(
        text_color_in(&hide_shapes, "Start", canvas).is_none(),
        "Hide should not paint base objects"
    );
    harness.click_at(start_pos);
    harness.idle_frame();
    assert_eq!(harness.editor.selection.object, None);

    // Choose Show: base objects are painted at full alpha, still not selectable on a layer.
    let show_btn = harness.find_text_in("Show", canvas).expect("Show toggle");
    harness.click_at(show_btn);
    harness.idle_frame();
    assert_eq!(
        harness.editor.session.course.editor.base_slide_view,
        BaseSlideView::Show
    );
    let show_shapes = harness.frame_shapes(vec![]);
    let show_color = text_color_in(&show_shapes, "Start", canvas).expect("Start shown");
    assert_eq!(show_color.a(), full_color.a());
    harness.click_at(start_pos);
    harness.idle_frame();
    assert_eq!(harness.editor.selection.object, None);

    // Switch back to Base: toggle disappears and everything is painted at full alpha.
    let base_pill = harness.find_text_in("Base", canvas).expect("Base pill");
    harness.click_at(base_pill);
    harness.idle_frame();
    assert_eq!(harness.editor.selection.layer, None);
    assert!(harness.find_text_in("Base slide:", canvas).is_none());
    let back_shapes = harness.frame_shapes(vec![]);
    let back_color = text_color_in(&back_shapes, "Start", canvas).expect("Start back on Base");
    assert_eq!(back_color.a(), full_color.a());
}

#[test]
fn base_slide_view_persists_undoes_and_does_not_affect_player() {
    let mut harness = TestHarness::new(hello());
    harness.editor.selection.slide = "s-intro".into();
    harness.idle_frame();
    harness.idle_frame();

    let canvas = harness.central_panel_rect();
    let about_pill = harness
        .find_text_in("About", canvas)
        .expect("About layer pill");
    harness.click_at(about_pill);
    harness.idle_frame();

    let hide_btn = harness.find_text_in("Hide", canvas).expect("Hide toggle");
    harness.click_at(hide_btn);
    harness.idle_frame();
    assert_eq!(
        harness.editor.session.course.editor.base_slide_view,
        BaseSlideView::Hide
    );

    // Undo / redo works on the editor view setting.
    harness.editor.session.undo().expect("undo");
    assert_eq!(
        harness.editor.session.course.editor.base_slide_view,
        BaseSlideView::Dim
    );
    harness.editor.session.redo().expect("redo");
    assert_eq!(
        harness.editor.session.course.editor.base_slide_view,
        BaseSlideView::Hide
    );

    // Save and reopen preserves the setting, and Player is unaffected.
    let saved = harness
        .editor
        .session
        .course
        .to_json()
        .expect("serialize course");
    let reopened = Course::from_json(&saved).expect("reopen course");
    assert_eq!(reopened.editor.base_slide_view, BaseSlideView::Hide);

    let player = coursecraft_logic::Player::new(&reopened).expect("player");
    let items = coursecraft_render::compose(&player);
    assert!(
        items
            .iter()
            .any(|it| it.hit == coursecraft_render::Hit::Object("btn-about".into())),
        "player still renders base objects regardless of editor base_slide_view"
    );
}

#[test]
fn hide_hides_multiple_base_object_kinds_while_layer_objects_stay_visible_and_selectable() {
    let mut harness = TestHarness::new(hello());
    harness.editor.selection.slide = "s-intro".into();
    harness.idle_frame();
    harness.idle_frame();

    let canvas = harness.central_panel_rect();
    // Base has both button ("Start", "About this course") and text/title objects.
    assert!(harness.find_text_in("Start", canvas).is_some());
    assert!(harness.find_text_in("About this course", canvas).is_some());

    // Switch to the "About" layer and select Hide.
    let about_pill = harness
        .find_text_in("About", canvas)
        .expect("About layer pill");
    harness.click_at(about_pill);
    harness.idle_frame();
    let hide_btn = harness.find_text_in("Hide", canvas).expect("Hide toggle");
    harness.click_at(hide_btn);
    harness.idle_frame();

    let hide_shapes = harness.frame_shapes(vec![]);
    assert!(text_color_in(&hide_shapes, "Start", canvas).is_none());
    assert!(text_color_in(&hide_shapes, "About this course", canvas).is_none());

    // Layer object ("Ten minutes. One quiz. 80% to pass.") is still painted at full alpha and selectable.
    let layer_text_color =
        text_color_in(&hide_shapes, "Ten minutes", canvas).expect("layer text visible");
    assert_eq!(layer_text_color.a(), 255);
    let layer_text_pos = harness
        .find_text_containing("Ten minutes")
        .expect("layer text pos");
    harness.click_at(layer_text_pos);
    harness.idle_frame();
    assert_eq!(
        harness.editor.selection.object.as_deref(),
        Some("about-text")
    );
}
