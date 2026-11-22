use coursecraft_course::{Course, Object, ShapeKind};
use coursecraft_logic::{Input, Player};
use egui::{Event, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};

use crate::compose::{DrawItem, Hit, Visual, compose, compose_object_items, compose_with_pointer};
use crate::media::{MediaFetch, MediaSource};
use crate::offscreen::{RenderOptions, render_slide_rgba};
use crate::view::SlideView;

struct EmptyMedia;
impl MediaSource for EmptyMedia {
    fn get(&self, _src: &str) -> MediaFetch {
        MediaFetch::Missing
    }
}

struct SolidPngMedia;
impl MediaSource for SolidPngMedia {
    fn get(&self, src: &str) -> MediaFetch {
        if src == "media/badge.png" {
            // 2x2 solid red RGBA PNG encoded via offscreen::encode_png or raw bytes
            let img = crate::offscreen::RgbaImage {
                width: 2,
                height: 2,
                pixels: vec![
                    220, 40, 40, 255, 220, 40, 40, 255, 220, 40, 40, 255, 220, 40, 40, 255,
                ],
            };
            let bytes = crate::offscreen::encode_png(&img).expect("png");
            MediaFetch::Ready(std::sync::Arc::from(bytes))
        } else {
            MediaFetch::Missing
        }
    }
}

fn single_object_course(obj_json: serde_json::Value) -> Course {
    slide_course(serde_json::json!({
        "id": "s1",
        "title": "Slide 1",
        "objects": [obj_json]
    }))
}

fn slide_course(slide_json: serde_json::Value) -> Course {
    let raw = serde_json::json!({
        "schema_version": 1,
        "id": "c-m29-extras",
        "title": "M29 Extras Render Test",
        "slide_size": { "w": 1280, "h": 720 },
        "theme": {
            "accent": "#2E6F9E",
            "text": "#111111",
            "background": "#FFFFFF",
            "font": "Inter"
        },
        "scenes": [{
            "id": "scene-1",
            "title": "Scene 1",
            "slides": [slide_json]
        }]
    });
    serde_json::from_value(raw).expect("valid course")
}

fn run_slide_frame(
    ctx: &egui::Context,
    view: &mut SlideView,
    player: &Player<'_>,
    events: Vec<Event>,
    time: f64,
) -> Vec<Input> {
    let mut inputs = Vec::new();
    let raw = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 720.0))),
        events,
        time: Some(time),
        ..Default::default()
    };
    let mut out = ctx.run_ui(raw, |ui| {
        inputs = view.show(ui, player, None);
    });
    out.textures_delta.clear();
    inputs
}

fn pointer_move(x: f32, y: f32) -> Vec<Event> {
    vec![Event::PointerMoved(egui::pos2(x, y))]
}

fn pointer_press(x: f32, y: f32) -> Vec<Event> {
    vec![
        Event::PointerMoved(egui::pos2(x, y)),
        Event::PointerButton {
            pos: egui::pos2(x, y),
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
    ]
}

fn pointer_release(x: f32, y: f32) -> Vec<Event> {
    vec![
        Event::PointerMoved(egui::pos2(x, y)),
        Event::PointerButton {
            pos: egui::pos2(x, y),
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        },
    ]
}

// ---------------------------------------------------------------------------
// R-09: StateExtra parent-relative placement & overhang
// ---------------------------------------------------------------------------

#[test]
fn r09_state_extra_parent_relative_placement_and_overhang() {
    let obj: Object = serde_json::from_value(serde_json::json!({
        "id": "btn-1",
        "name": "Button 1",
        "frame": { "x": 100, "y": 200, "w": 160, "h": 48 },
        "kind": { "type": "button", "label": "Next", "fill": "#2E6F9E" },
        "states": [{
            "name": "visited",
            "extras": [{
                "id": "badge-1",
                "name": "Checkmark",
                "frame": { "x": 144, "y": -8, "w": 24, "h": 24 },
                "kind": {
                    "type": "shape",
                    "shape": "ellipse",
                    "fill": "#22C55E"
                }
            }]
        }]
    }))
    .expect("object");

    let items: Vec<DrawItem> = compose_object_items(&obj, "visited", None, None);
    assert_eq!(items.len(), 2, "parent + 1 extra");
    assert!(!items[0].is_state_extra);
    assert_eq!(items[0].hit, Hit::Object("btn-1".into()));

    assert!(items[1].is_state_extra);
    assert_eq!(items[1].hit, Hit::Object("btn-1".into()));
    assert!((items[1].rect.min.x - 244.0).abs() < 0.01);
    assert!((items[1].rect.min.y - 192.0).abs() < 0.01);
    assert!((items[1].rect.width() - 24.0).abs() < 0.01);
    assert!((items[1].rect.height() - 24.0).abs() < 0.01);
}

// ---------------------------------------------------------------------------
// R-10: StateExtra hit-testing routes to parent (ST-11)
// ---------------------------------------------------------------------------

#[test]
fn r10_state_extra_overhanging_hit_testing_routes_to_parent() {
    let course = single_object_course(serde_json::json!({
        "id": "btn-1",
        "name": "Button 1",
        "frame": { "x": 100, "y": 200, "w": 160, "h": 48 },
        "initial_state": "visited",
        "kind": { "type": "button", "label": "Next", "fill": "#2E6F9E" },
        "states": [
            {
                "name": "visited",
                "extras": [{
                    "id": "badge-1",
                    "frame": { "x": 144, "y": -8, "w": 24, "h": 24 },
                    "kind": { "type": "shape", "shape": "ellipse", "fill": "#22C55E" }
                }]
            },
            {
                "name": "hover",
                "opacity": 0.8
            }
        ]
    }));
    let mut player = Player::new(&course).expect("player");
    player.handle(&Input::Start);
    let ctx = egui::Context::default();
    let mut view = SlideView::new();

    // Overhanging corner of extra: slide rect is (244..268, 192..216).
    // (262, 195) is inside the extra and outside the parent (100..260, 200..248).
    let inputs_hover = run_slide_frame(&ctx, &mut view, &player, pointer_move(262.0, 195.0), 0.0);
    assert!(
        inputs_hover
            .iter()
            .any(|i| matches!(i, Input::HoverStart { object } if object == "btn-1")),
        "hovering overhanging extra must emit HoverStart for parent, got {inputs_hover:?}"
    );
    assert!(
        (view.last_items()[0].opacity - 0.8).abs() < 0.01,
        "hovering overhanging extra must apply parent hover state"
    );

    run_slide_frame(&ctx, &mut view, &player, pointer_press(262.0, 195.0), 0.1);
    let inputs_click =
        run_slide_frame(&ctx, &mut view, &player, pointer_release(262.0, 195.0), 0.2);
    assert!(
        inputs_click
            .iter()
            .any(|i| matches!(i, Input::Click { object } if object == "btn-1")),
        "clicking overhanging extra must emit Click for parent, got {inputs_click:?}"
    );

    let inputs_leave = run_slide_frame(&ctx, &mut view, &player, pointer_move(10.0, 10.0), 0.3);
    assert!(
        inputs_leave
            .iter()
            .any(|i| matches!(i, Input::HoverEnd { object } if object == "btn-1")),
        "leaving overhanging extra must emit HoverEnd for parent, got {inputs_leave:?}"
    );
}

// ---------------------------------------------------------------------------
// R-11: StateExtra parent rotation & flip inheritance (ST-12)
// ---------------------------------------------------------------------------

#[test]
fn r11_state_extra_inherits_parent_rotation_and_flip() {
    // Parent: (100, 100, 100, 40) -> centre (150, 120).
    // Extra: relative (90, -10, 20, 20) -> unrotated slide rect (190, 90, 20, 20), centre (200, 100).
    // Vector from parent centre to extra centre: (+50, -20).
    let obj_rot: Object = serde_json::from_value(serde_json::json!({
        "id": "btn-rot",
        "name": "Button Rot",
        "frame": { "x": 100, "y": 100, "w": 100, "h": 40 },
        "rotation": 90.0,
        "kind": { "type": "shape", "shape": "rectangle", "fill": "#2E6F9E" },
        "states": [{
            "name": "visited",
            "extras": [{
                "id": "b1",
                "frame": { "x": 90, "y": -10, "w": 20, "h": 20 },
                "rotation": 15.0,
                "flip_h": true,
                "kind": { "type": "shape", "shape": "rectangle", "fill": "#FF0000" }
            }]
        }]
    }))
    .expect("obj_rot");

    let items_rot = compose_object_items(&obj_rot, "visited", None, None);
    assert_eq!(items_rot.len(), 2);
    // 90 deg CW: (+50, -20) -> (+20, +50) => centre (170, 170).
    let extra_center = items_rot[1].rect.center();
    assert!(
        (extra_center.x - 170.0).abs() < 0.1 && (extra_center.y - 170.0).abs() < 0.1,
        "expected rotated centre (170, 170), got {extra_center:?}"
    );
    assert!((items_rot[1].rotation - 105.0).abs() < 0.1);
    assert!(items_rot[1].flip_h);
    assert!(!items_rot[1].flip_v);

    // Flip H on parent: (+50, -20) -> (-50, -20) => centre (100, 100).
    // flip_sign = -1 -> world_rotation = normalize(0 + -15) = 345, world_flip_h = true ^ true = false.
    let obj_flip: Object = serde_json::from_value(serde_json::json!({
        "id": "btn-flip",
        "name": "Button Flip",
        "frame": { "x": 100, "y": 100, "w": 100, "h": 40 },
        "flip_h": true,
        "kind": { "type": "shape", "shape": "rectangle", "fill": "#2E6F9E" },
        "states": [{
            "name": "visited",
            "extras": [{
                "id": "b1",
                "frame": { "x": 90, "y": -10, "w": 20, "h": 20 },
                "rotation": 15.0,
                "flip_h": true,
                "kind": { "type": "shape", "shape": "rectangle", "fill": "#FF0000" }
            }]
        }]
    }))
    .expect("obj_flip");

    let items_flip = compose_object_items(&obj_flip, "visited", None, None);
    assert_eq!(items_flip.len(), 2);
    let flip_center = items_flip[1].rect.center();
    assert!(
        (flip_center.x - 100.0).abs() < 0.1 && (flip_center.y - 100.0).abs() < 0.1,
        "expected flipped centre (100, 100), got {flip_center:?}"
    );
    assert!((items_flip[1].rotation - 345.0).abs() < 0.1);
    assert!(!items_flip[1].flip_h);
}

// ---------------------------------------------------------------------------
// R-12: StateExtra parent animation & motion-path inheritance (ST-12)
// ---------------------------------------------------------------------------

#[test]
fn r12_state_extra_inherits_animation_opacity_scale_clip_and_motion_offset() {
    // Zoom entrance (1000ms) + motion path offset (40, 20) over 1000ms.
    let course = slide_course(serde_json::json!({
        "id": "s1",
        "title": "Anim",
        "triggers": [{
            "when": { "event": "timeline_starts" },
            "action": { "action": "move_on_path", "object": "shp-anim", "path": "mp-1" }
        }],
        "objects": [{
            "id": "shp-anim",
            "name": "Animated",
            "frame": { "x": 100, "y": 100, "w": 100, "h": 100 },
            "initial_state": "visited",
            "entrance": { "effect": { "type": "zoom" }, "duration_ms": 1000, "easing": "linear" },
            "motion_paths": [{
                "id": "mp-1",
                "duration_ms": 1000,
                "easing": "linear",
                "points": [{ "x": 0.0, "y": 0.0 }, { "x": 40.0, "y": 20.0 }]
            }],
            "states": [{
                "name": "visited",
                "opacity": 0.8,
                "extras": [{
                    "id": "b1",
                    "frame": { "x": 80, "y": 0, "w": 20, "h": 20 },
                    "opacity": 0.5,
                    "kind": { "type": "shape", "shape": "ellipse", "fill": "#FF0000" }
                }]
            }],
            "kind": { "type": "shape", "shape": "rectangle", "fill": "#2E6F9E" }
        }]
    }));

    let mut player = Player::new(&course).expect("player");
    player.handle(&Input::Start);
    player.handle(&Input::Tick { ms: 500 });

    let items = compose(&player);
    assert_eq!(items.len(), 2);
    let parent = &items[0];
    let extra = &items[1];
    assert!(extra.is_state_extra);

    // Motion path offset at t=500ms is (20, 10), so parent centre is (150+20, 150+10) = (170, 160).
    assert!(
        (parent.rect.center().x - 170.0).abs() < 0.2
            && (parent.rect.center().y - 160.0).abs() < 0.2,
        "expected parent centre (170, 160), got {:?}",
        parent.rect.center()
    );

    let expected_opacity = (parent.opacity * 0.5).clamp(0.0, 1.0);
    assert!(
        (extra.opacity - expected_opacity).abs() < 0.01,
        "extra opacity {} should be parent {} * 0.5",
        extra.opacity,
        parent.opacity
    );
    // Parent rect scaled around parent centre: extra's scaled width / 20.0 should equal parent's scaled width / 100.0
    let scale = parent.rect.width() / 100.0;
    assert!(
        scale > 0.1 && scale < 1.0,
        "expected partial zoom scale, got {scale}"
    );
    assert!(
        (extra.rect.width() - 20.0 * scale).abs() < 0.2,
        "extra width {} should scale by {scale}",
        extra.rect.width()
    );
    // Extra centre relative to parent centre should also scale by `scale`
    let rel = extra.rect.center() - parent.rect.center();
    // Unscaled rel is (90 - 50, 10 - 50) = (40, -40)
    assert!(
        (rel.x - 40.0 * scale).abs() < 0.2 && (rel.y - (-40.0 * scale)).abs() < 0.2,
        "extra rel centre {rel:?} should equal (40, -40) * {scale}"
    );

    // Also verify wipe clip inheritance
    let wipe_course = single_object_course(serde_json::json!({
        "id": "shp-wipe",
        "name": "Wipe",
        "frame": { "x": 100, "y": 100, "w": 100, "h": 100 },
        "initial_state": "visited",
        "entrance": { "effect": { "type": "wipe", "from": "left" }, "duration_ms": 1000, "easing": "linear" },
        "states": [{
            "name": "visited",
            "extras": [{
                "id": "b1",
                "frame": { "x": 80, "y": 0, "w": 20, "h": 20 },
                "kind": { "type": "shape", "shape": "ellipse", "fill": "#FF0000" }
            }]
        }],
        "kind": { "type": "shape", "shape": "rectangle", "fill": "#2E6F9E" }
    }));
    let mut wipe_player = Player::new(&wipe_course).expect("player");
    wipe_player.handle(&Input::Start);
    wipe_player.handle(&Input::Tick { ms: 500 });
    let wipe_items = compose(&wipe_player);
    assert_eq!(wipe_items.len(), 2);
    assert!(wipe_items[0].clip.is_some());
    assert_eq!(wipe_items[1].clip, wipe_items[0].clip);
}

// ---------------------------------------------------------------------------
// R-13: StateExtra inside ScrollPanel & DragDrop (SUS-02)
// ---------------------------------------------------------------------------

#[test]
fn r13_state_extra_scroll_panel_drag_item_and_target_label_sus02() {
    // Part A: ScrollPanel child with extra
    let scroll_course = slide_course(serde_json::json!({
        "id": "s1",
        "title": "Scroll",
        "objects": [
            {
                "id": "panel-1",
                "name": "Panel",
                "frame": { "x": 50, "y": 50, "w": 300, "h": 200 },
                "kind": { "type": "scroll_panel", "content_height": 600 }
            },
            {
                "id": "child-1",
                "name": "Child",
                "panel": "panel-1",
                "frame": { "x": 60, "y": 100, "w": 120, "h": 40 },
                "initial_state": "visited",
                "kind": { "type": "button", "label": "Item", "fill": "#2E6F9E" },
                "states": [{
                    "name": "visited",
                    "extras": [{
                        "id": "b1",
                        "frame": { "x": 100, "y": -5, "w": 20, "h": 20 },
                        "kind": { "type": "shape", "shape": "ellipse", "fill": "#22C55E" }
                    }]
                }]
            }
        ]
    }));
    let mut sp_player = Player::new(&scroll_course).expect("player");
    sp_player.handle(&Input::Start);
    sp_player.handle(&Input::PanelScrolled {
        object: "panel-1".into(),
        offset: 40.0,
    });
    let sp_items = compose(&sp_player);
    let extra = sp_items
        .iter()
        .find(|i| i.is_state_extra)
        .expect("extra in scroll panel");
    // Unscrolled extra y = 100 + (-5) = 95; scrolled by -40 -> 55.
    assert!((extra.rect.min.y - 55.0).abs() < 0.1);
    assert_eq!(
        extra.clip_rect,
        Some(Rect::from_min_size(
            Pos2::new(50.0, 50.0),
            vec2(300.0, 200.0)
        ))
    );

    // Part B: DragDrop — extra on drag item gets Hit::DragItem, and Text extra on drop target
    // is NOT misclassified as a target label pill (SUS-02)!
    let dd_course = slide_course(serde_json::json!({
        "id": "s1",
        "title": "DragDrop",
        "objects": [
            {
                "id": "target-1",
                "name": "Target",
                "frame": { "x": 400, "y": 200, "w": 200, "h": 150 },
                "initial_state": "highlighted",
                "kind": { "type": "shape", "shape": "rectangle", "fill": "#EEEEEE" },
                "states": [{
                    "name": "highlighted",
                    "extras": [{
                        "id": "txt-extra",
                        "frame": { "x": 10, "y": 10, "w": 180, "h": 30 },
                        "kind": { "type": "text", "text": "Drop zone badge" }
                    }]
                }]
            },
            {
                "id": "drag-1",
                "name": "Drag 1",
                "frame": { "x": 420, "y": 210, "w": 100, "h": 50 },
                "initial_state": "picked",
                "kind": { "type": "shape", "shape": "rectangle", "fill": "#2E6F9E" },
                "states": [{
                    "name": "picked",
                    "extras": [{
                        "id": "drag-badge",
                        "frame": { "x": 85, "y": -5, "w": 20, "h": 20 },
                        "kind": { "type": "shape", "shape": "ellipse", "fill": "#F59E0B" }
                    }]
                }]
            }
        ],
        "question": {
            "kind": "drag_drop",
            "prompt": "Drag item to target",
            "choices": [],
            "drag_items": [{ "item": "drag-1", "target": "target-1" }],
            "drop_targets": ["target-1"]
        }
    }));
    let mut dd_player = Player::new(&dd_course).expect("player");
    dd_player.handle(&Input::Start);
    let dd_items = compose(&dd_player);

    let drag_extra = dd_items
        .iter()
        .find(|i| i.is_state_extra && matches!(&i.hit, Hit::DragItem(id) if id == "drag-1"))
        .expect("drag item extra must be retargeted to Hit::DragItem");
    assert!((drag_extra.rect.min.x - 505.0).abs() < 0.1);

    // SUS-02: Target's Text extra must keep Hit::Object("target-1") and stay at (410, 210),
    // NOT be turned into a moved target-label pill with Hit::None!
    let target_text_extra = dd_items
        .iter()
        .find(|i| i.is_state_extra && matches!(&i.hit, Hit::Object(id) if id == "target-1"))
        .expect("target text extra must NOT be reclassified as target label pill");
    assert!((target_text_extra.rect.min.x - 410.0).abs() < 0.1);
    assert!((target_text_extra.rect.min.y - 210.0).abs() < 0.1);
}

// ---------------------------------------------------------------------------
// R-14: StateExtra %var% interpolation
// ---------------------------------------------------------------------------

#[test]
fn r14_state_extra_variable_interpolation() {
    let raw = serde_json::json!({
        "schema_version": 1,
        "id": "c-m29-var",
        "title": "Var Test",
        "slide_size": { "w": 1280, "h": 720 },
        "theme": { "accent": "#2E6F9E", "text": "#111111", "background": "#FFFFFF", "font": "Inter" },
        "variables": [{ "name": "score", "value": 42 }],
        "scenes": [{
            "id": "scene-1",
            "title": "Scene 1",
            "slides": [{
                "id": "s1",
                "title": "Slide 1",
                "objects": [{
                    "id": "btn-1",
                    "name": "Button",
                    "frame": { "x": 100, "y": 100, "w": 200, "h": 60 },
                    "initial_state": "visited",
                    "kind": { "type": "button", "label": "Check", "fill": "#2E6F9E" },
                    "states": [{
                        "name": "visited",
                        "extras": [
                            {
                                "id": "txt-1",
                                "frame": { "x": 0, "y": -30, "w": 120, "h": 24 },
                                "kind": { "type": "text", "text": "%score% pts" }
                            },
                            {
                                "id": "shp-1",
                                "frame": { "x": 130, "y": -30, "w": 80, "h": 24 },
                                "kind": {
                                    "type": "shape",
                                    "shape": "rounded_rectangle",
                                    "fill": "#22C55E",
                                    "text": { "text": "+%score%" }
                                }
                            }
                        ]
                    }]
                }]
            }]
        }]
    });
    let course: Course = serde_json::from_value(raw).expect("course");
    let mut player = Player::new(&course).expect("player");
    player.handle(&Input::Start);
    let items = compose(&player);
    assert_eq!(items.len(), 3);

    match &items[1].visual {
        Visual::Text { runs, .. } => {
            let joined: String = runs.iter().map(|r| r.text.as_str()).collect();
            assert_eq!(joined, "42 pts");
        }
        other => panic!("expected Text extra, got {other:?}"),
    }
    match &items[2].visual {
        Visual::Shape {
            text: Some(shape_text),
            ..
        } => {
            let joined: String = shape_text.runs.iter().map(|r| r.text.as_str()).collect();
            assert_eq!(joined, "+42");
        }
        other => panic!("expected Shape with text extra, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// R-15: StateExtra two-layer stacking (visited + hover extras)
// ---------------------------------------------------------------------------

#[test]
fn r15_state_extra_two_layer_stacking_visited_then_hover() {
    let course = single_object_course(serde_json::json!({
        "id": "btn-1",
        "name": "Button",
        "frame": { "x": 100, "y": 100, "w": 160, "h": 48 },
        "initial_state": "visited",
        "kind": { "type": "button", "label": "Next", "fill": "#2E6F9E" },
        "states": [
            {
                "name": "visited",
                "extras": [{
                    "id": "v1",
                    "frame": { "x": 140, "y": -6, "w": 20, "h": 20 },
                    "kind": { "type": "text", "text": "Visited" }
                }]
            },
            {
                "name": "hover",
                "extras": [{
                    "id": "h1",
                    "frame": { "x": -10, "y": -6, "w": 20, "h": 20 },
                    "kind": { "type": "text", "text": "Hover" }
                }]
            }
        ]
    }));
    let mut player = Player::new(&course).expect("player");
    player.handle(&Input::Start);

    // Without hover: parent + visited extra
    let base_items = compose_with_pointer(&player, None, None);
    assert_eq!(base_items.len(), 2);

    // With hover: parent, then visited extra, then hover extra
    let hover_items = compose_with_pointer(&player, Some("btn-1"), None);
    assert_eq!(hover_items.len(), 3);
    assert!(!hover_items[0].is_state_extra);
    assert!(hover_items[1].is_state_extra);
    assert!(hover_items[2].is_state_extra);
    assert!(matches!(
        &hover_items[1].visual,
        Visual::Text { runs, .. } if runs.iter().any(|r| r.text == "Visited")
    ));
    assert!(matches!(
        &hover_items[2].visual,
        Visual::Text { runs, .. } if runs.iter().any(|r| r.text == "Hover")
    ));

    // Leaving hover removes only hover extras
    let after_items = compose_with_pointer(&player, None, None);
    assert_eq!(after_items.len(), 2);
}

// ---------------------------------------------------------------------------
// R-16: Offscreen pixel render of all 4 StateExtraKinds
// ---------------------------------------------------------------------------

#[test]
fn r16_offscreen_pixel_render_all_four_state_extra_kinds() {
    let course = single_object_course(serde_json::json!({
        "id": "parent-1",
        "name": "Parent",
        "frame": { "x": 100, "y": 100, "w": 200, "h": 100 },
        "initial_state": "active",
        "kind": { "type": "shape", "shape": "rectangle", "fill": "#EEEEEE" },
        "states": [{
            "name": "active",
            "extras": [
                {
                    "id": "ex-shape",
                    "frame": { "x": 220, "y": 0, "w": 60, "h": 60 },
                    "kind": { "type": "shape", "shape": "rectangle", "fill": "#1D4ED8" }
                },
                {
                    "id": "ex-image",
                    "frame": { "x": 300, "y": 0, "w": 60, "h": 60 },
                    "kind": { "type": "image", "src": "media/badge.png", "alt": "Badge" }
                },
                {
                    "id": "ex-text",
                    "frame": { "x": 380, "y": 0, "w": 120, "h": 60 },
                    "kind": {
                        "type": "text",
                        "text": "HELLO",
                        "style": { "color": "#111111", "size": 28.0 }
                    }
                },
                {
                    "id": "ex-path",
                    "frame": { "x": 520, "y": 0, "w": 80, "h": 60 },
                    "kind": {
                        "type": "path",
                        "points": [{ "x": 0.0, "y": 0.5 }, { "x": 1.0, "y": 0.5 }],
                        "closed": false,
                        "outline": { "color": "#16A34A", "width": 8.0 }
                    }
                }
            ]
        }]
    }));
    let mut player = Player::new(&course).expect("player");
    player.handle(&Input::Start);
    let img = render_slide_rgba(
        &course,
        "s1",
        &RenderOptions {
            width: 1280,
            ..Default::default()
        },
        &SolidPngMedia,
    )
    .expect("render");
    assert_eq!(img.width, 1280);
    assert_eq!(img.height, 720);

    let pixel_at = |x: u32, y: u32| -> &[u8] {
        let idx = ((y * img.width + x) * 4) as usize;
        &img.pixels[idx..idx + 4]
    };

    // Check non-white pixels at each extra's centre:
    // ex-shape: (100+220+30, 100+30) = (350, 130) -> blue
    let p_shape = pixel_at(350, 130);
    assert!(
        p_shape[2] > 180 && p_shape[0] < 80,
        "expected blue shape pixel, got {p_shape:?}"
    );

    // ex-image: (100+300+30, 100+30) = (430, 130) -> red
    let p_img = pixel_at(430, 130);
    assert!(
        p_img[0] > 180 && p_img[2] < 80,
        "expected red image pixel, got {p_img:?}"
    );

    // ex-text: (100+380..500, 100..160) = (480..600, 100..160) -> at least one dark pixel
    let has_text_px = (485..595).any(|x| (105..155).any(|y| pixel_at(x, y)[0] < 150));
    assert!(has_text_px, "expected rendered text extra pixels");

    // ex-path: (100+520+40, 100+30) = (660, 130) -> green
    let p_path = pixel_at(660, 130);
    assert!(
        p_path[1] > 120 && p_path[0] < 80,
        "expected green path pixel, got {p_path:?}"
    );

    let _ = (ShapeKind::Rectangle, EmptyMedia);
}
