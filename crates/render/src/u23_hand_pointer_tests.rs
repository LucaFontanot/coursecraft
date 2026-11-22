use coursecraft_course::Course;
use coursecraft_logic::{Input, Player};
use egui::{Color32, CursorIcon, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};

use crate::compose::compose;
use crate::view::SlideView;

fn run_slide_frame_cursor(
    ctx: &egui::Context,
    view: &mut SlideView,
    player: &Player<'_>,
    events: Vec<Event>,
    time: f64,
) -> CursorIcon {
    let raw = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 720.0))),
        events,
        time: Some(time),
        ..Default::default()
    };
    let mut out = ctx.run_ui(raw, |ui| {
        let _ = view.show(ui, player, None);
    });
    out.textures_delta.clear();
    out.platform_output.cursor_icon
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

fn course_from_slide(slide_json: serde_json::Value) -> Course {
    let raw = serde_json::json!({
        "schema_version": 1,
        "id": "c-u23-cursor",
        "title": "U23 Cursor Test",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "scene-1",
            "title": "Scene 1",
            "slides": [slide_json]
        }]
    });
    serde_json::from_value(raw).expect("valid course")
}

#[test]
fn cursor_is_pointing_hand_over_clickable_shape_and_button() {
    let course = course_from_slide(serde_json::json!({
        "id": "s1",
        "title": "Slide 1",
        "objects": [
            {
                "id": "shape-click",
                "name": "Clickable Shape",
                "frame": { "x": 100, "y": 100, "w": 200, "h": 100 },
                "kind": { "type": "shape", "shape": "rectangle", "fill": "#2F6FEB" }
            },
            {
                "id": "btn-1",
                "name": "Button 1",
                "frame": { "x": 400, "y": 100, "w": 200, "h": 60 },
                "kind": { "type": "button", "label": "Go" }
            }
        ],
        "triggers": [
            {
                "when": { "event": "click", "object": "shape-click" },
                "action": { "action": "jump_to_slide", "target": "next" }
            }
        ]
    }));
    let mut player = Player::new(&course).expect("player");
    let _ = player.handle(&Input::Start);
    let ctx = egui::Context::default();
    let mut view = SlideView::default();

    let cursor_shape =
        run_slide_frame_cursor(&ctx, &mut view, &player, pointer_move(200.0, 150.0), 0.1);
    assert_eq!(cursor_shape, CursorIcon::PointingHand);

    let cursor_button =
        run_slide_frame_cursor(&ctx, &mut view, &player, pointer_move(500.0, 130.0), 0.2);
    assert_eq!(cursor_button, CursorIcon::PointingHand);
}

#[test]
fn cursor_is_arrow_over_plain_shape_disabled_button_and_hover_only_object() {
    let course = course_from_slide(serde_json::json!({
        "id": "s1",
        "title": "Slide 1",
        "objects": [
            {
                "id": "plain-shape",
                "name": "Plain Shape",
                "frame": { "x": 100, "y": 100, "w": 200, "h": 100 },
                "kind": { "type": "shape", "shape": "rectangle", "fill": "#CCCCCC" }
            },
            {
                "id": "disabled-btn",
                "name": "Disabled Button",
                "frame": { "x": 400, "y": 100, "w": 200, "h": 60 },
                "initial_state": "disabled",
                "states": [{ "name": "disabled" }],
                "kind": { "type": "button", "label": "Disabled" }
            },
            {
                "id": "hover-only",
                "name": "Hover Only",
                "frame": { "x": 700, "y": 100, "w": 200, "h": 100 },
                "kind": { "type": "shape", "shape": "rectangle", "fill": "#EEEEEE" }
            }
        ],
        "triggers": [
            {
                "when": { "event": "hover_start", "object": "hover-only" },
                "action": { "action": "change_state", "object": "plain-shape", "state": "normal" }
            }
        ]
    }));
    let mut player = Player::new(&course).expect("player");
    let _ = player.handle(&Input::Start);
    let ctx = egui::Context::default();
    let mut view = SlideView::default();

    let cursor_plain =
        run_slide_frame_cursor(&ctx, &mut view, &player, pointer_move(200.0, 150.0), 0.1);
    assert_eq!(cursor_plain, CursorIcon::Default);

    let cursor_disabled =
        run_slide_frame_cursor(&ctx, &mut view, &player, pointer_move(500.0, 130.0), 0.2);
    assert_eq!(cursor_disabled, CursorIcon::Default);

    let cursor_hover_only =
        run_slide_frame_cursor(&ctx, &mut view, &player, pointer_move(800.0, 150.0), 0.3);
    assert_eq!(cursor_hover_only, CursorIcon::Default);
}

#[test]
fn cursor_is_arrow_over_covered_base_slide_under_lightbox() {
    let raw = serde_json::json!({
        "schema_version": 1,
        "id": "c-u23-lb",
        "title": "U23 Lightbox Cursor Test",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "scene-1",
            "title": "Scene 1",
            "slides": [
                {
                    "id": "base",
                    "title": "Base",
                    "objects": [
                        {
                            "id": "open-lb",
                            "name": "Open",
                            "frame": { "x": 20, "y": 20, "w": 120, "h": 60 },
                            "kind": { "type": "button", "label": "Open" }
                        }
                    ],
                    "triggers": [
                        {
                            "when": { "event": "click", "object": "open-lb" },
                            "action": { "action": "lightbox_slide", "slide": "box" }
                        }
                    ]
                },
                {
                    "id": "box",
                    "title": "Box",
                    "objects": [
                        {
                            "id": "box-btn",
                            "name": "Box Button",
                            "frame": { "x": 500, "y": 300, "w": 200, "h": 60 },
                            "kind": { "type": "button", "label": "Inside" }
                        }
                    ]
                }
            ]
        }]
    });
    let course: Course = serde_json::from_value(raw).expect("valid course");
    let mut player = Player::new(&course).expect("player");
    let _ = player.handle(&Input::Start);
    let _ = player.handle(&Input::Click {
        object: "open-lb".to_owned(),
    });
    assert!(player.lightbox_open());

    let ctx = egui::Context::default();
    let mut view = SlideView::default();

    // (40, 40) is over the base slide's button, outside the 85% lightbox box.
    let cursor_base =
        run_slide_frame_cursor(&ctx, &mut view, &player, pointer_move(40.0, 40.0), 0.1);
    assert_eq!(cursor_base, CursorIcon::Default);

    // Inside the lightbox box over `box-btn`.
    let box_rect = crate::view::lightbox_box_rect(
        Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 720.0)),
        course.slide_size,
    )
    .expect("box rect");
    let t = crate::layout::SlideTransform::fit(box_rect, course.slide_size).expect("transform");
    let btn_center = t.to_screen_pos(Pos2::new(600.0, 330.0));
    let cursor_box_btn = run_slide_frame_cursor(
        &ctx,
        &mut view,
        &player,
        pointer_move(btn_center.x, btn_center.y),
        0.2,
    );
    assert_eq!(cursor_box_btn, CursorIcon::PointingHand);
}

#[test]
fn cursor_is_grab_over_drag_item_and_grabbing_while_dragging() {
    let course = course_from_slide(serde_json::json!({
        "id": "s1",
        "title": "Drag Slide",
        "objects": [
            {
                "id": "drag-a",
                "name": "Drag A",
                "frame": { "x": 100, "y": 200, "w": 160, "h": 80 },
                "kind": { "type": "shape", "shape": "rectangle", "fill": "#2F6FEB" }
            },
            {
                "id": "drop-1",
                "name": "Drop 1",
                "frame": { "x": 600, "y": 200, "w": 200, "h": 120 },
                "kind": { "type": "shape", "shape": "rectangle", "fill": "#DDDDDD" }
            }
        ],
        "question": {
            "kind": "drag_drop",
            "prompt": "Drag A to Drop 1",
            "choices": [],
            "drag_items": [
                { "item": "drag-a", "target": "drop-1" }
            ],
            "drop_targets": ["drop-1"]
        }
    }));
    let mut player = Player::new(&course).expect("player");
    let _ = player.handle(&Input::Start);
    let ctx = egui::Context::default();
    let mut view = SlideView::default();

    let cursor_hover =
        run_slide_frame_cursor(&ctx, &mut view, &player, pointer_move(180.0, 240.0), 0.1);
    assert_eq!(cursor_hover, CursorIcon::Grab);

    let cursor_dragging =
        run_slide_frame_cursor(&ctx, &mut view, &player, pointer_press(180.0, 240.0), 0.2);
    assert_eq!(cursor_dragging, CursorIcon::Grabbing);
}

#[test]
fn rendered_shadow_uses_alpha_byte_from_course() {
    let course = course_from_slide(serde_json::json!({
        "id": "s1",
        "title": "Shadow Slide",
        "objects": [
            {
                "id": "shadow-shape",
                "name": "Shadow Shape",
                "frame": { "x": 100, "y": 100, "w": 200, "h": 100 },
                "shadow": {
                    "color": "#00000066",
                    "offset_x": 4.0,
                    "offset_y": 4.0,
                    "blur": 6.0
                },
                "kind": { "type": "shape", "shape": "rectangle", "fill": "#2F6FEB" }
            }
        ]
    }));
    let player = Player::new(&course).expect("player");
    let items = compose(&player);
    let item = items
        .iter()
        .find(|it| matches!(&it.hit, crate::compose::Hit::Object(id) if id == "shadow-shape"))
        .expect("shadow shape item");
    let shadow = item.shadow.as_ref().expect("resolved shadow");
    assert_eq!(shadow.color, Color32::from_rgba_unmultiplied(0, 0, 0, 0x66));
}
