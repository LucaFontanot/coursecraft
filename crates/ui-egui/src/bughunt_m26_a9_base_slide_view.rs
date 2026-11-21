//! M26 bug-hunt tests for A9: Slide layers Base slide view (Show / Dim / Hide).
//!
//! Ordered P1 -> P2 -> P3.

use crate::test_harness::TestHarness;
use coursecraft_course::{
    BaseSlideView, Color, Course, FillStyle, GradientStop, ObjectKind, Rect, ShapeKind,
};
use coursecraft_logic::{Input, Player};
use coursecraft_render::compose;
use egui::epaint::{ClippedShape, Shape};
use egui::{Color32, Modifiers, vec2};

fn hello() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    Course::from_json(&json).expect("valid course")
}

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

// ============================================================================
// P1
// ============================================================================

#[test]
fn a9_1_base_slide_view_dim_works_even_when_dim_other_layers_off() {
    let mut h = TestHarness::new(hello());
    h.editor.selection.slide = "s-intro".into();
    h.idle_frame();
    h.idle_frame();

    let canvas = h.central_panel_rect();
    let base_shapes = h.frame_shapes(vec![]);
    let full_color = text_color_in(&base_shapes, "Start", canvas).expect("Start on Base");

    let about_pill = h.find_text_in("About", canvas).expect("About pill");
    h.click_at(about_pill);
    h.idle_frame();

    h.editor.ui.layers.dim_other_layers = false;
    let show_btn = h.find_text_in("Show", canvas).expect("Show toggle");
    h.click_at(show_btn);
    h.idle_frame();
    let dim_btn = h.find_text_in("Dim", canvas).expect("Dim toggle");
    h.click_at(dim_btn);
    h.idle_frame();

    let dim_shapes = h.frame_shapes(vec![]);
    let dim_color = text_color_in(&dim_shapes, "Start", canvas).expect("Start dimmed");
    assert!(
        dim_color.a() < full_color.a(),
        "explicit Dim must dim base even when dim_other_layers=false: full={}, dim={}",
        full_color.a(),
        dim_color.a()
    );
}

#[test]
fn a9_2_editor_base_slide_view_isolated_from_runtime_hide_base_objects() {
    let mut h = TestHarness::new(hello());
    h.editor
        .session
        .execute(
            "layer.update",
            &serde_json::json!({
                "slide": "s-intro",
                "layer": "about",
                "hide_base_objects": true
            }),
        )
        .expect("layer.update");
    h.editor.selection.slide = "s-intro".into();
    h.idle_frame();
    h.idle_frame();
    let canvas = h.central_panel_rect();
    let about_pill = h.find_text_in("About", canvas).expect("About pill");
    h.click_at(about_pill);
    h.idle_frame();
    let show_btn = h.find_text_in("Show", canvas).expect("Show");
    h.click_at(show_btn);
    h.idle_frame();
    assert_eq!(
        h.editor.session.course.editor.base_slide_view,
        BaseSlideView::Show
    );

    // Runtime obeys hide_base_objects=true even when editor base_slide_view=Show
    let mut p = Player::new(&h.editor.session.course).expect("player");
    let _ = p.handle(&Input::Start);
    let _ = p.handle(&Input::Click {
        object: "btn-about".into(),
    });
    let frame = compose(&p);
    assert!(
        !frame
            .iter()
            .any(|it| matches!(&it.hit, coursecraft_render::Hit::Object(id) if id == "title")),
        "runtime hide_base_objects=true should hide base objects"
    );

    // Flip hide_base_objects=false, base_slide_view=Hide: runtime STILL shows base objects
    h.editor
        .session
        .execute(
            "layer.update",
            &serde_json::json!({
                "slide": "s-intro",
                "layer": "about",
                "hide_base_objects": false
            }),
        )
        .expect("layer.update false");
    let hide_btn = h.find_text_in("Hide", canvas).expect("Hide");
    h.click_at(hide_btn);
    h.idle_frame();
    assert_eq!(
        h.editor.session.course.editor.base_slide_view,
        BaseSlideView::Hide
    );

    let mut p2 = Player::new(&h.editor.session.course).expect("player2");
    let _ = p2.handle(&Input::Start);
    let _ = p2.handle(&Input::Click {
        object: "btn-about".into(),
    });
    let frame2 = compose(&p2);
    assert!(
        frame2
            .iter()
            .any(|it| matches!(&it.hit, coursecraft_render::Hit::Object(id) if id == "title")),
        "editor BaseSlideView::Hide must not leak into runtime"
    );
}

#[test]
fn a9_4_dimming_and_hiding_all_base_object_kinds() {
    let mut course = hello();
    let mut grad = course.scenes[0].slides[0].objects[0].clone();
    grad.id = "base-grad".into();
    grad.frame = Rect {
        x: 500.0,
        y: 80.0,
        w: 220.0,
        h: 120.0,
    };
    grad.kind = ObjectKind::Shape {
        shape: ShapeKind::Rectangle,
        fill: Color("#FF0000".into()),
        fill_style: Some(FillStyle::Linear {
            angle: 0.0,
            stops: vec![
                GradientStop {
                    color: Color("#FF0000".into()),
                    position: 0.0,
                },
                GradientStop {
                    color: Color("#00FF00".into()),
                    position: 0.5,
                },
                GradientStop {
                    color: Color("#0000FF".into()),
                    position: 1.0,
                },
            ],
        }),
        outline: None,
        text: None,
        adjust: Vec::new(),
    };
    let tbl: coursecraft_course::Object = serde_json::from_value(serde_json::json!({
        "id": "base-tbl",
        "frame": { "x": 500.0, "y": 240.0, "w": 240.0, "h": 100.0 },
        "kind": {
            "type": "table",
            "columns": [0.5, 0.5],
            "rows": [{ "height": 1.0, "cells": [{ "text": "TblCell" }, { "text": "R" }] }]
        }
    }))
    .expect("tbl");
    course.scenes[0].slides[0].objects.extend([grad, tbl]);

    let mut h = TestHarness::new(course);
    h.editor.selection.slide = "s-intro".into();
    h.idle_frame();
    h.idle_frame();
    let canvas = h.central_panel_rect();
    let about_pill = h.find_text_in("About", canvas).expect("About pill");
    h.click_at(about_pill);
    h.idle_frame();

    // Dim: TblCell alpha reduced
    let dim_shapes = h.frame_shapes(vec![]);
    let dim_tbl = text_color_in(&dim_shapes, "TblCell", canvas).expect("TblCell dimmed");
    assert!(dim_tbl.a() < 200, "TblCell dim alpha: {}", dim_tbl.a());

    // Hide: TblCell not painted
    let hide_btn = h.find_text_in("Hide", canvas).expect("Hide");
    h.click_at(hide_btn);
    h.idle_frame();
    let hide_shapes = h.frame_shapes(vec![]);
    assert!(text_color_in(&hide_shapes, "TblCell", canvas).is_none());
}

#[test]
fn a9_6_switching_slides_and_layers_preserves_base_slide_view_and_hides_toggle_on_base() {
    let mut h = TestHarness::new(hello());
    h.editor.selection.slide = "s-intro".into();
    h.idle_frame();
    h.idle_frame();
    let canvas = h.central_panel_rect();
    assert!(h.find_text_in("Base slide:", canvas).is_none());

    let about_pill = h.find_text_in("About", canvas).expect("About pill");
    h.click_at(about_pill);
    h.idle_frame();
    let hide_btn = h.find_text_in("Hide", canvas).expect("Hide");
    h.click_at(hide_btn);
    h.idle_frame();
    assert_eq!(
        h.editor.session.course.editor.base_slide_view,
        BaseSlideView::Hide
    );

    // Switch to Slide 2 (Base)
    let s2_title = h.editor.session.course.scenes[0].slides[1].title.clone();
    let s2_row = h
        .find_text_in(&s2_title, h.left_panel_rect())
        .expect("Slide 2");
    h.click_at(s2_row);
    h.idle_frame();
    assert!(h.find_text_in("Base slide:", canvas).is_none());
    assert_eq!(
        h.editor.session.course.editor.base_slide_view,
        BaseSlideView::Hide
    );
}

#[test]
fn a9_7_marquee_selection_on_layer_never_selects_base_objects() {
    let mut h = TestHarness::new(hello());
    h.editor.selection.slide = "s-intro".into();
    h.idle_frame();
    h.idle_frame();
    let canvas = h.central_panel_rect();
    let about_pill = h.find_text_in("About", canvas).expect("About pill");
    h.click_at(about_pill);
    h.idle_frame();

    let layer_ids: std::collections::HashSet<String> = h
        .editor
        .session
        .course
        .slide("s-intro")
        .unwrap()
        .layer("about")
        .unwrap()
        .objects
        .iter()
        .map(|o| o.id.clone())
        .collect();

    for mode in ["Show", "Dim", "Hide"] {
        let btn = h.find_text_in(mode, canvas).expect(mode);
        h.click_at(btn);
        h.idle_frame();
        h.editor.selection.clear_objects();
        h.drag_by(
            canvas.min + vec2(10.0, 40.0),
            vec2(canvas.width() - 20.0, canvas.height() - 60.0),
            6,
            Modifiers::NONE,
        );
        h.idle_frame();
        for id in h.editor.selection.ids() {
            assert!(
                layer_ids.contains(&id),
                "mode {mode} marquee selected non-layer object {id}"
            );
        }
    }
}

// ============================================================================
// P2
// ============================================================================

#[test]
fn a9_3_master_slide_objects_under_show_dim_and_hide() {
    let mut h = TestHarness::new(hello());
    let mid = h
        .editor
        .session
        .execute("master.add", &serde_json::json!({ "title": "M" }))
        .expect("master")["id"]
        .as_str()
        .unwrap()
        .to_owned();
    h.editor
        .session
        .execute(
            "object.add",
            &serde_json::json!({
                "slide": mid,
                "id": "m-footer",
                "kind": { "type": "text", "text": "Master Footer" },
                "frame": { "x": 100.0, "y": 640.0, "w": 240.0, "h": 40.0 }
            }),
        )
        .expect("add master obj");
    h.editor
        .session
        .execute(
            "slide.set_master",
            &serde_json::json!({ "slides": ["s-intro"], "master": mid }),
        )
        .expect("set_master");

    h.editor.selection.slide = "s-intro".into();
    h.idle_frame();
    h.idle_frame();
    let canvas = h.central_panel_rect();
    let full_a = text_color_in(&h.frame_shapes(vec![]), "Master Footer", canvas)
        .expect("Master Footer on Base")
        .a();

    let about_pill = h.find_text_in("About", canvas).expect("About pill");
    h.click_at(about_pill);
    h.idle_frame();
    let dim_a = text_color_in(&h.frame_shapes(vec![]), "Master Footer", canvas)
        .expect("Master Footer dimmed")
        .a();
    assert!(
        dim_a < full_a,
        "Master Footer should dim: {dim_a} < {full_a}"
    );

    let hide_btn = h.find_text_in("Hide", canvas).expect("Hide");
    h.click_at(hide_btn);
    h.idle_frame();
    assert!(text_color_in(&h.frame_shapes(vec![]), "Master Footer", canvas).is_none());
}

#[test]
fn a9_5_pinned_other_layer_remains_visible_when_base_hidden() {
    let mut h = TestHarness::new(hello());
    h.editor
        .session
        .execute(
            "layer.add",
            &serde_json::json!({ "slide": "s-intro", "id": "layer-b", "name": "Layer B" }),
        )
        .expect("layer.add");
    h.editor
        .session
        .execute(
            "object.add",
            &serde_json::json!({
                "slide": "s-intro",
                "layer": "layer-b",
                "id": "pinned-b",
                "kind": { "type": "text", "text": "Pinned B" },
                "frame": { "x": 200.0, "y": 300.0, "w": 200.0, "h": 50.0 }
            }),
        )
        .expect("object.add on layer-b");

    h.editor.selection.slide = "s-intro".into();
    h.idle_frame();
    h.idle_frame();
    let canvas = h.central_panel_rect();
    let about_pill = h.find_text_in("About", canvas).expect("About pill");
    h.click_at(about_pill);
    h.editor.ui.layers.hidden_layers.remove("layer-b");
    let hide_btn = h.find_text_in("Hide", canvas).expect("Hide");
    h.click_at(hide_btn);
    h.idle_frame();

    let shapes = h.frame_shapes(vec![]);
    assert!(text_color_in(&shapes, "Start", canvas).is_none());
    assert!(text_color_in(&shapes, "Pinned B", canvas).is_some());
}

#[test]
fn a9_8_timeline_scrubbing_while_editing_layer_under_hide() {
    let mut h = TestHarness::new(hello());
    h.editor.selection.slide = "s-intro".into();
    h.idle_frame();
    h.idle_frame();
    let canvas = h.central_panel_rect();
    let about_pill = h.find_text_in("About", canvas).expect("About pill");
    h.click_at(about_pill);
    let hide_btn = h.find_text_in("Hide", canvas).expect("Hide");
    h.click_at(hide_btn);
    h.editor.ui.timeline.playhead_ms = 1500;
    h.idle_frame();

    let shapes = h.frame_shapes(vec![]);
    assert!(text_color_in(&shapes, "Start", canvas).is_none());
}

#[test]
fn a9_9_toggle_bar_layout_on_narrow_window_with_six_layers() {
    let mut h = TestHarness::new_with_size(hello(), (1024.0, 680.0));
    for (i, name) in [
        "Correct",
        "Incorrect",
        "Try Again",
        "Hint 1",
        "Hint 2",
        "Summary",
    ]
    .iter()
    .enumerate()
    {
        h.editor
            .session
            .execute(
                "layer.add",
                &serde_json::json!({
                    "slide": "s-intro",
                    "id": format!("l{i}"),
                    "name": name
                }),
            )
            .expect("layer.add");
    }
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.layer = Some("l5".into());
    h.idle_frame();
    h.idle_frame();

    let canvas = h.central_panel_rect();
    for pill in ["Show", "Dim", "Hide"] {
        assert!(
            h.find_text_in(pill, canvas).is_some(),
            "{pill} toggle should be visible inside central panel"
        );
    }
    let report = h.layout_report(3);
    assert!(
        report.findings.is_empty(),
        "narrow window 6-layer findings: {:?}",
        report.findings
    );
}
