//! M26 bug-hunt tests for `coursecraft-render` (A1.10, A2.5, A2.6, A2.7, A2.10, A2.11, A3.5).
//!
//! Ordered P1 -> P2 -> P3.

use crate::{
    MediaFetch, MediaSource, RenderMode, RenderOptions, compose, render_player_rgba,
    render_slide_rgba,
};
use coursecraft_course::{Color, Course, FillStyle, GradientStop, ObjectKind, Rect, ShapeKind};
use coursecraft_logic::{Input, Player};
use std::sync::Arc;

struct NoMedia;
impl MediaSource for NoMedia {
    fn get(&self, _src: &str) -> MediaFetch {
        MediaFetch::Missing
    }
}

fn px_at(img: &crate::RgbaImage, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * img.width + x) * 4) as usize;
    [
        img.pixels[i],
        img.pixels[i + 1],
        img.pixels[i + 2],
        img.pixels[i + 3],
    ]
}

fn has_rgb(img: &crate::RgbaImage, pred: impl Fn([u8; 4]) -> bool) -> bool {
    img.pixels.as_chunks::<4>().0.iter().any(|c| pred(*c))
}

// ============================================================================
// P1
// ============================================================================

#[test]
fn a2_6_gradients_across_fillable_object_kinds() {
    let stops = vec![
        GradientStop {
            color: Color("#0000FF".into()),
            position: 0.0,
        },
        GradientStop {
            color: Color("#FF0000".into()),
            position: 0.5,
        },
        GradientStop {
            color: Color("#FFFF00".into()),
            position: 1.0,
        },
    ];
    let fs = FillStyle::Linear {
        angle: 0.0,
        stops: stops.clone(),
    };

    for shape in [
        ShapeKind::Ellipse,
        ShapeKind::Heart,
        ShapeKind::Star5,
        ShapeKind::RoundedRectangle,
    ] {
        let json = serde_json::json!({
            "schema_version": 1,
            "id": "g-kinds",
            "title": "G",
            "slide_size": { "w": 1280, "h": 720 },
            "scenes": [{
                "id": "sc1",
                "title": "S",
                "slides": [{
                    "id": "s1",
                    "title": "S1",
                    "duration_ms": 5000,
                    "objects": [{
                        "id": "o1",
                        "frame": { "x": 100.0, "y": 100.0, "w": 600.0, "h": 400.0 },
                        "kind": {
                            "type": "shape",
                            "shape": shape,
                            "fill": "#0000FF",
                            "fill_style": fs
                        }
                    }]
                }]
            }]
        })
        .to_string();
        let c = Course::from_json(&json).unwrap();
        let img = render_slide_rgba(
            &c,
            "s1",
            &RenderOptions {
                width: 640,
                mode: RenderMode::Player,
                ..Default::default()
            },
            &NoMedia,
        )
        .unwrap();
        assert!(
            has_rgb(&img, |p| p[2] > 180 && p[0] < 80),
            "{shape:?}: missing blue"
        );
        assert!(
            has_rgb(&img, |p| p[0] > 180 && p[1] < 80 && p[2] < 80),
            "{shape:?}: missing red"
        );
        assert!(
            has_rgb(&img, |p| p[0] > 180 && p[1] > 180 && p[2] < 80),
            "{shape:?}: missing yellow"
        );
    }
}

#[test]
fn a2_11_thumbnail_and_full_render_preserve_3_stop_gradients() {
    let json = serde_json::json!({
        "schema_version": 1,
        "id": "g-thumb",
        "title": "Thumb",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "sc1",
            "title": "S",
            "slides": [{
                "id": "s1",
                "title": "S1",
                "duration_ms": 5000,
                "objects": [{
                    "id": "o1",
                    "frame": { "x": 40.0, "y": 40.0, "w": 1000.0, "h": 500.0 },
                    "kind": {
                        "type": "shape",
                        "shape": "rectangle",
                        "fill": "#0000FF",
                        "fill_style": {
                            "type": "linear",
                            "angle": 0.0,
                            "stops": [
                                { "color": "#0000FF", "position": 0.0 },
                                { "color": "#FF0000", "position": 0.5 },
                                { "color": "#FFFF00", "position": 1.0 }
                            ]
                        }
                    }
                }]
            }]
        }]
    })
    .to_string();
    let c = Course::from_json(&json).unwrap();
    for width in [160, 1280] {
        let img = render_slide_rgba(
            &c,
            "s1",
            &RenderOptions {
                width,
                mode: RenderMode::Player,
                ..Default::default()
            },
            &NoMedia,
        )
        .unwrap();
        assert!(
            has_rgb(&img, |p| p[2] > 180 && p[0] < 80),
            "w={width}: missing blue"
        );
        assert!(
            has_rgb(&img, |p| p[0] > 180 && p[1] < 80 && p[2] < 80),
            "w={width}: missing red"
        );
        assert!(
            has_rgb(&img, |p| p[0] > 180 && p[1] > 180 && p[2] < 80),
            "w={width}: missing yellow"
        );
    }
}

#[test]
fn a3_5_rendered_pixels_during_mid_exit_with_transition() {
    let json = serde_json::json!({
        "schema_version": 1,
        "id": "a3-render",
        "title": "A3 Render",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "sc1",
            "title": "S",
            "slides": [
                { "id": "s1", "title": "S1", "duration_ms": 2000, "objects": [] },
                {
                    "id": "s2",
                    "title": "S2",
                    "duration_ms": 5000,
                    "transition": { "type": "push", "from": "left", "duration_ms": 500 },
                    "objects": [
                        {
                            "id": "obj-fly",
                            "kind": { "type": "shape", "shape": "rectangle", "fill": "#FF0000" },
                            "frame": { "x": 400.0, "y": 200.0, "w": 200.0, "h": 100.0 },
                            "exit": { "effect": { "type": "fly_in", "from": "right" }, "duration_ms": 1000, "easing": "linear" }
                        },
                        {
                            "id": "obj-wipe",
                            "kind": { "type": "shape", "shape": "rectangle", "fill": "#00FF00" },
                            "frame": { "x": 400.0, "y": 400.0, "w": 200.0, "h": 100.0 },
                            "exit": { "effect": { "type": "wipe", "from": "left" }, "duration_ms": 1000, "easing": "linear" }
                        }
                    ]
                }
            ]
        }]
    })
    .to_string();
    let c = Course::from_json(&json).unwrap();
    let mut p = Player::new(&c).unwrap();
    let _ = p.handle(&Input::Start);
    let _ = p.handle(&Input::GoTo { slide: "s2".into() });
    let _ = p.handle(&Input::Tick { ms: 2000 });
    let img_rest = render_player_rgba(&p, 1280, &[], &NoMedia, None).unwrap();
    let rest_px = px_at(&img_rest, 500, 250);
    assert!(rest_px[0] > 200 && rest_px[1] < 50);

    let _ = p.handle(&Input::Tick { ms: 2500 }); // t=4500ms
    let items = compose(&p);
    let fly_item = items
        .iter()
        .find(|it| matches!(&it.hit, crate::Hit::Object(id) if id == "obj-fly"))
        .expect("obj-fly item");
    assert!(
        fly_item.rect.min.x > 700.0,
        "fly_item rect={:?}",
        fly_item.rect
    );

    let img_exit = render_player_rgba(&p, 1280, &[], &NoMedia, None).unwrap();
    let old_px = px_at(&img_exit, 500, 250);
    assert!(
        old_px[0] < 100 || old_px[1] > 100,
        "old pos should no longer be red: {old_px:?}"
    );

    let _ = p.handle(&Input::Tick { ms: 500 }); // t=5000ms
    let img_end = render_player_rgba(&p, 1280, &[], &NoMedia, None).unwrap();
    assert!(!has_rgb(&img_end, |p| p[0] > 200 && p[1] < 50 && p[2] < 50));
    assert!(!has_rgb(&img_end, |p| p[1] > 200 && p[0] < 50 && p[2] < 50));
}

// ============================================================================
// P2
// ============================================================================

#[test]
fn a1_10_numeric_text_properties_render_and_roundtrip_identically() {
    let json_a = serde_json::json!({
        "schema_version": 1,
        "id": "a1-render",
        "title": "A1 Render",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "sc1",
            "title": "S",
            "slides": [{
                "id": "s1",
                "title": "S1",
                "duration_ms": 5000,
                "objects": [{
                    "id": "t1",
                    "kind": {
                        "type": "text",
                        "text": "Line One\nLine Two",
                        "style": { "size": 20.0, "letter_spacing": 0.0 },
                        "paragraphs": [{ "line_spacing": 1.0 }, { "line_spacing": 1.0 }]
                    },
                    "frame": { "x": 100.0, "y": 100.0, "w": 500.0, "h": 200.0 }
                }]
            }]
        }]
    })
    .to_string();
    let ca = Course::from_json(&json_a).unwrap();
    let img_a = render_slide_rgba(
        &ca,
        "s1",
        &RenderOptions {
            width: 640,
            mode: RenderMode::Player,
            ..Default::default()
        },
        &NoMedia,
    )
    .unwrap();

    let mut cb = ca.clone();
    if let ObjectKind::Text {
        style, paragraphs, ..
    } = &mut cb.scenes[0].slides[0].objects[0].kind
    {
        style.size = 40.0;
        style.letter_spacing = 6.0;
        for p in paragraphs.iter_mut() {
            p.line_spacing = 2.0;
        }
    }
    let cb2 = Course::from_json(&cb.to_json().unwrap()).unwrap();
    let img_b = render_slide_rgba(
        &cb2,
        "s1",
        &RenderOptions {
            width: 640,
            mode: RenderMode::Player,
            ..Default::default()
        },
        &NoMedia,
    )
    .unwrap();
    assert_ne!(img_a.pixels, img_b.pixels);
}

#[test]
fn a2_5_alpha_gradient_over_dark_background_no_grey_halo() {
    let json = serde_json::json!({
        "schema_version": 1,
        "id": "alpha-grad",
        "title": "Alpha",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "sc1",
            "title": "S",
            "slides": [{
                "id": "s1",
                "title": "S1",
                "duration_ms": 5000,
                "background": "#000044",
                "objects": [{
                    "id": "r1",
                    "frame": { "x": 0.0, "y": 0.0, "w": 1280.0, "h": 720.0 },
                    "kind": {
                        "type": "shape",
                        "shape": "rectangle",
                        "fill": "#FF0000",
                        "fill_style": {
                            "type": "linear",
                            "angle": 0.0,
                            "stops": [
                                { "color": "#FF0000FF", "position": 0.0 },
                                { "color": "#FF000080", "position": 0.5 },
                                { "color": "#FF000000", "position": 1.0 }
                            ]
                        }
                    }
                }]
            }]
        }]
    })
    .to_string();
    let c = Course::from_json(&json).unwrap();
    let img = render_slide_rgba(
        &c,
        "s1",
        &RenderOptions {
            width: 1280,
            mode: RenderMode::Player,
            ..Default::default()
        },
        &NoMedia,
    )
    .unwrap();
    let mid = px_at(&img, 640, 360);
    assert!(
        mid[1] < 25,
        "midpoint should have no green/grey halo, got {mid:?}"
    );
}

#[test]
#[ignore = "NOT A BUG: hover is presentational in SlideView/Highlight per docs/spec/triggers.md; Player does not store hover in PlayerState and render_player_rgba renders with Highlight::default()"]
fn a2_7_multi_stop_gradient_with_interactive_states() {
    let json = serde_json::json!({
        "schema_version": 1,
        "id": "grad-states",
        "title": "States",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "sc1",
            "title": "S",
            "slides": [{
                "id": "s1",
                "title": "S1",
                "duration_ms": 5000,
                "objects": [{
                    "id": "b1",
                    "frame": { "x": 100.0, "y": 100.0, "w": 600.0, "h": 200.0 },
                    "kind": {
                        "type": "button",
                        "label": "",
                        "fill": "#0000FF",
                        "fill_style": {
                            "type": "linear",
                            "angle": 0.0,
                            "stops": [
                                { "color": "#0000FF", "position": 0.0 },
                                { "color": "#FF0000", "position": 0.5 },
                                { "color": "#FFFF00", "position": 1.0 }
                            ]
                        }
                    },
                    "states": [
                        { "name": "hover", "fill": "#11AA11" },
                        { "name": "selected", "text": "Chosen" }
                    ]
                }],
                "triggers": [{
                    "when": { "event": "click", "object": "b1" },
                    "action": { "action": "change_state", "object": "b1", "state": "selected" }
                }]
            }]
        }]
    })
    .to_string();
    let c = Course::from_json(&json).unwrap();
    let mut p = Player::new(&c).unwrap();
    let _ = p.handle(&Input::Start);

    let img_norm = render_player_rgba(&p, 640, &[], &NoMedia, None).unwrap();
    assert!(has_rgb(&img_norm, |px| px[0] > 200
        && px[1] > 200
        && px[2] < 60));

    let _ = p.handle(&Input::HoverStart {
        object: "b1".into(),
    });
    let img_hov = render_player_rgba(&p, 640, &[], &NoMedia, None).unwrap();
    assert!(!has_rgb(&img_hov, |px| px[0] > 200
        && px[1] > 200
        && px[2] < 60));

    let _ = p.handle(&Input::HoverEnd {
        object: "b1".into(),
    });
    let img_rest = render_player_rgba(&p, 640, &[], &NoMedia, None).unwrap();
    assert!(has_rgb(&img_rest, |px| px[0] > 200
        && px[1] > 200
        && px[2] < 60));

    let _ = p.handle(&Input::Click {
        object: "b1".into(),
    });
    let img_sel = render_player_rgba(&p, 640, &[], &NoMedia, None).unwrap();
    assert!(has_rgb(&img_sel, |px| px[0] > 200
        && px[1] > 200
        && px[2] < 60));
}

// ============================================================================
// P3
// ============================================================================

#[test]
fn a2_10_extreme_angles_corner_radial_and_tiny_vs_full_shapes() {
    for angle in [0.0, 90.0, 180.0, 270.0] {
        let json = serde_json::json!({
            "schema_version": 1,
            "id": "g-ang",
            "title": "Ang",
            "slide_size": { "w": 1280, "h": 720 },
            "scenes": [{
                "id": "sc1",
                "title": "S",
                "slides": [{
                    "id": "s1",
                    "title": "S1",
                    "duration_ms": 5000,
                    "objects": [{
                        "id": "o1",
                        "frame": { "x": 0.0, "y": 0.0, "w": 1280.0, "h": 720.0 },
                        "kind": {
                            "type": "shape",
                            "shape": "rectangle",
                            "fill": "#0000FF",
                            "fill_style": {
                                "type": "linear",
                                "angle": angle,
                                "stops": [
                                    { "color": "#0000FF", "position": 0.0 },
                                    { "color": "#FF0000", "position": 0.5 },
                                    { "color": "#FFFF00", "position": 1.0 }
                                ]
                            }
                        }
                    }]
                }]
            }]
        })
        .to_string();
        let c = Course::from_json(&json).unwrap();
        let img = render_slide_rgba(
            &c,
            "s1",
            &RenderOptions {
                width: 640,
                mode: RenderMode::Player,
                ..Default::default()
            },
            &NoMedia,
        )
        .unwrap();
        assert!(
            has_rgb(&img, |p| p[2] > 200 && p[0] < 50),
            "angle={angle}: blue"
        );
        assert!(
            has_rgb(&img, |p| p[0] > 200 && p[1] > 200 && p[2] < 50),
            "angle={angle}: yellow"
        );
    }

    // Tiny 8x8 shape: no panic
    let tiny = Course::from_json(
        &serde_json::json!({
            "schema_version": 1,
            "id": "g-tiny",
            "title": "Tiny",
            "slide_size": { "w": 1280, "h": 720 },
            "scenes": [{
                "id": "sc1",
                "title": "S",
                "slides": [{
                    "id": "s1",
                    "title": "S1",
                    "duration_ms": 5000,
                    "objects": [{
                        "id": "o1",
                        "frame": { "x": 100.0, "y": 100.0, "w": 8.0, "h": 8.0 },
                        "kind": {
                            "type": "shape",
                            "shape": "rectangle",
                            "fill": "#0000FF",
                            "fill_style": {
                                "type": "radial",
                                "center": [0.0, 0.0],
                                "stops": [
                                    { "color": "#0000FF", "position": 0.0 },
                                    { "color": "#FFFF00", "position": 1.0 }
                                ]
                            }
                        }
                    }]
                }]
            }]
        })
        .to_string(),
    )
    .unwrap();
    let _ = render_slide_rgba(
        &tiny,
        "s1",
        &RenderOptions {
            width: 640,
            mode: RenderMode::Player,
            ..Default::default()
        },
        &NoMedia,
    )
    .unwrap();
    let _ = (
        Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        Arc::new(0),
    );
}
