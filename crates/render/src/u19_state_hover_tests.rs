use coursecraft_course::Course;
use coursecraft_logic::{Input, Player};
use egui::{Color32, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};

use crate::compose::{ResolvedTextRun, Visual};
use crate::view::SlideView;

fn run_slide_frame(
    ctx: &egui::Context,
    view: &mut SlideView,
    player: &Player<'_>,
    events: Vec<Event>,
    time: f64,
) {
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

fn item_visual<'a>(view: &'a SlideView, id: &str) -> &'a Visual {
    &view
        .last_items()
        .iter()
        .find(|it| matches!(&it.hit, crate::compose::Hit::Object(oid) if oid == id))
        .expect("object item")
        .visual
}

fn single_object_course(obj_json: serde_json::Value) -> Course {
    let raw = serde_json::json!({
        "schema_version": 1,
        "id": "c-u19-render",
        "title": "U19 Render Test",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "scene-1",
            "title": "Scene 1",
            "slides": [{
                "id": "s1",
                "title": "Slide 1",
                "objects": [obj_json]
            }]
        }]
    });
    serde_json::from_value(raw).expect("valid course")
}

#[test]
fn hovering_image_with_hover_src_swaps_src_and_leaving_restores_it() {
    let course = single_object_course(serde_json::json!({
        "id": "img-1",
        "name": "Picture 1",
        "frame": { "x": 100, "y": 100, "w": 200, "h": 200 },
        "kind": { "type": "image", "src": "base.png" },
        "states": [
            { "name": "hover", "src": "hover.png" }
        ]
    }));
    let mut player = Player::new(&course).expect("player");
    player.handle(&Input::Start);
    let ctx = egui::Context::default();
    let mut view = SlideView::new();

    run_slide_frame(&ctx, &mut view, &player, vec![], 0.0);
    assert!(
        matches!(item_visual(&view, "img-1"), Visual::Image { src, .. } if src == "base.png"),
        "expected base.png initially"
    );

    run_slide_frame(&ctx, &mut view, &player, pointer_move(200.0, 200.0), 0.1);
    assert!(
        matches!(item_visual(&view, "img-1"), Visual::Image { src, .. } if src == "hover.png"),
        "expected hover.png when hovered, got {:?}",
        item_visual(&view, "img-1")
    );

    run_slide_frame(&ctx, &mut view, &player, pointer_move(10.0, 10.0), 0.2);
    assert!(
        matches!(item_visual(&view, "img-1"), Visual::Image { src, .. } if src == "base.png"),
        "expected base.png after leaving"
    );
}

#[test]
fn disabled_image_keeps_base_src_when_hovered() {
    let course = single_object_course(serde_json::json!({
        "id": "img-1",
        "name": "Picture 1",
        "frame": { "x": 100, "y": 100, "w": 200, "h": 200 },
        "initial_state": "disabled",
        "kind": { "type": "image", "src": "base.png" },
        "states": [
            { "name": "disabled" },
            { "name": "hover", "src": "hover.png" }
        ]
    }));
    let mut player = Player::new(&course).expect("player");
    player.handle(&Input::Start);
    let ctx = egui::Context::default();
    let mut view = SlideView::new();

    run_slide_frame(&ctx, &mut view, &player, pointer_move(200.0, 200.0), 0.0);
    assert!(
        matches!(item_visual(&view, "img-1"), Visual::Image { src, .. } if src == "base.png"),
        "disabled image must keep base.png on hover"
    );
}

#[test]
fn visited_text_and_hover_fill_both_apply() {
    let course = single_object_course(serde_json::json!({
        "id": "txt-1",
        "name": "Text 1",
        "frame": { "x": 100, "y": 100, "w": 300, "h": 100 },
        "initial_state": "visited",
        "kind": { "type": "text", "text": "Base", "style": { "color": "#111111" } },
        "states": [
            { "name": "visited", "text": "Visited" },
            { "name": "hover", "fill": "#FF0000" }
        ]
    }));
    let mut player = Player::new(&course).expect("player");
    player.handle(&Input::Start);
    let ctx = egui::Context::default();
    let mut view = SlideView::new();

    run_slide_frame(&ctx, &mut view, &player, pointer_move(200.0, 150.0), 0.0);
    match item_visual(&view, "txt-1") {
        Visual::Text { runs, .. } => {
            assert_eq!(runs.len(), 1);
            assert_eq!(runs[0].text, "Visited");
            assert_eq!(runs[0].color, Color32::from_rgb(0xFF, 0x00, 0x00));
        }
        other => panic!("expected Visual::Text, got {other:?}"),
    }
}

#[test]
fn pressing_uses_down_state_over_hover() {
    let course = single_object_course(serde_json::json!({
        "id": "img-1",
        "name": "Picture 1",
        "frame": { "x": 100, "y": 100, "w": 200, "h": 200 },
        "kind": { "type": "image", "src": "base.png" },
        "states": [
            { "name": "hover", "src": "hover.png" },
            { "name": "down", "src": "down.png" }
        ]
    }));
    let mut player = Player::new(&course).expect("player");
    player.handle(&Input::Start);
    let ctx = egui::Context::default();
    let mut view = SlideView::new();

    run_slide_frame(&ctx, &mut view, &player, pointer_press(200.0, 200.0), 0.0);
    assert!(
        matches!(item_visual(&view, "img-1"), Visual::Image { src, .. } if src == "down.png"),
        "expected down.png while pressed, got {:?}",
        item_visual(&view, "img-1")
    );
}

#[test]
fn hovering_text_object_with_hover_text_swaps_text() {
    let course = single_object_course(serde_json::json!({
        "id": "txt-1",
        "name": "Text 1",
        "frame": { "x": 100, "y": 100, "w": 300, "h": 100 },
        "kind": { "type": "text", "text": "Normal" },
        "states": [
            { "name": "hover", "text": "Hovered" }
        ]
    }));
    let mut player = Player::new(&course).expect("player");
    player.handle(&Input::Start);
    let ctx = egui::Context::default();
    let mut view = SlideView::new();

    run_slide_frame(&ctx, &mut view, &player, pointer_move(200.0, 150.0), 0.0);
    match item_visual(&view, "txt-1") {
        Visual::Text { runs, .. } => {
            let text: String = runs
                .iter()
                .map(|r: &ResolvedTextRun| r.text.as_str())
                .collect();
            assert_eq!(text, "Hovered");
        }
        other => panic!("expected Visual::Text, got {other:?}"),
    }
}

#[test]
fn hovering_path_with_hover_fill_swaps_fill() {
    let course = single_object_course(serde_json::json!({
        "id": "path-1",
        "name": "Path 1",
        "frame": { "x": 100, "y": 100, "w": 200, "h": 200 },
        "kind": {
            "type": "path",
            "points": [
                { "x": 0.0, "y": 0.0 },
                { "x": 1.0, "y": 0.0 },
                { "x": 0.5, "y": 1.0 }
            ],
            "closed": true,
            "fill": "#112233",
            "outline": { "color": "#000000", "width": 2.0 }
        },
        "states": [
            { "name": "hover", "fill": "#445566" }
        ]
    }));
    let mut player = Player::new(&course).expect("player");
    player.handle(&Input::Start);
    let ctx = egui::Context::default();
    let mut view = SlideView::new();

    run_slide_frame(&ctx, &mut view, &player, pointer_move(200.0, 180.0), 0.0);
    match item_visual(&view, "path-1") {
        Visual::Path(p) => {
            assert_eq!(p.fill, Some(Color32::from_rgb(0x44, 0x55, 0x66)));
        }
        other => panic!("expected Visual::Path, got {other:?}"),
    }
}

#[test]
fn button_hover_and_down_fill_behaviour_unchanged() {
    let course = single_object_course(serde_json::json!({
        "id": "btn-1",
        "name": "Button 1",
        "frame": { "x": 100, "y": 100, "w": 200, "h": 60 },
        "kind": { "type": "button", "label": "Go", "fill": "#2F6FEB" },
        "states": [
            { "name": "hover", "fill": "#334455", "text": "Hover Go" },
            { "name": "down", "fill": "#667788", "text": "Down Go" }
        ]
    }));
    let mut player = Player::new(&course).expect("player");
    player.handle(&Input::Start);
    let ctx = egui::Context::default();
    let mut view = SlideView::new();

    run_slide_frame(&ctx, &mut view, &player, pointer_move(200.0, 130.0), 0.0);
    match item_visual(&view, "btn-1") {
        Visual::Button {
            label,
            fill,
            hover_fill,
            down_fill,
            ..
        } => {
            assert_eq!(label, "Hover Go");
            assert_eq!(*fill, Color32::from_rgb(0x33, 0x44, 0x55));
            assert_eq!(*hover_fill, Some(Color32::from_rgb(0x33, 0x44, 0x55)));
            assert_eq!(*down_fill, Some(Color32::from_rgb(0x66, 0x77, 0x88)));
        }
        other => panic!("expected Visual::Button, got {other:?}"),
    }

    run_slide_frame(&ctx, &mut view, &player, pointer_press(200.0, 130.0), 0.1);
    match item_visual(&view, "btn-1") {
        Visual::Button { label, fill, .. } => {
            assert_eq!(label, "Down Go");
            assert_eq!(*fill, Color32::from_rgb(0x66, 0x77, 0x88));
        }
        other => panic!("expected Visual::Button, got {other:?}"),
    }
}
