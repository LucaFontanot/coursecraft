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

fn pixel_at(img: &crate::offscreen::RgbaImage, x: u32, y: u32) -> [u8; 4] {
    let idx = ((y * img.width + x) * 4) as usize;
    [
        img.pixels[idx],
        img.pixels[idx + 1],
        img.pixels[idx + 2],
        img.pixels[idx + 3],
    ]
}

fn render_items_offscreen(
    course: &Course,
    items: &[crate::compose::DrawItem],
    highlight: crate::paint::Highlight,
) -> crate::offscreen::RgbaImage {
    let slide = &course.scenes[0].slides[0];
    let screen = Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 720.0));
    let transform = crate::layout::SlideTransform::fit(screen, course.slide_size).expect("fit");
    crate::offscreen::render_ui(
        1280,
        720,
        1.0,
        |_| {},
        |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::background());
            crate::paint::paint(
                &painter,
                &transform,
                items,
                highlight.clone(),
                None,
                Some(&course.theme),
                Some(course.slide_background(slide)),
            );
        },
    )
    .expect("offscreen render")
}

#[test]
fn gb_06_interactive_group_shared_hover_down_and_disabled() {
    let raw = serde_json::json!({
        "schema_version": 1,
        "id": "c-gb-06",
        "title": "GB-06 Interactive Group States",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "scene-1",
            "title": "Scene 1",
            "slides": [{
                "id": "s1",
                "title": "Slide 1",
                "background": "#FFFFFF",
                "groups": [{
                    "id": "g-btn",
                    "name": "Grouped Button",
                    "interactive": true
                }],
                "objects": [
                    {
                        "id": "m-rect",
                        "name": "Rect",
                        "group": "g-btn",
                        "frame": { "x": 100, "y": 100, "w": 120, "h": 100 },
                        "kind": { "type": "shape", "shape": "rectangle", "fill": "#224488" },
                        "states": [
                            { "name": "hover", "fill": "#11AA22" },
                            { "name": "down", "fill": "#DD3311" }
                        ]
                    },
                    {
                        "id": "m-icon",
                        "name": "Icon",
                        "group": "g-btn",
                        "frame": { "x": 260, "y": 100, "w": 120, "h": 100 },
                        "kind": { "type": "shape", "shape": "rectangle", "fill": "#335599" },
                        "states": [
                            { "name": "hover", "fill": "#22BBCC" }
                        ]
                    },
                    {
                        "id": "m-text",
                        "name": "Label",
                        "group": "g-btn",
                        "frame": { "x": 420, "y": 100, "w": 220, "h": 100 },
                        "kind": {
                            "type": "text",
                            "text": "MMMM",
                            "style": { "size": 48, "bold": true, "color": "#112233" }
                        },
                        "states": [
                            { "name": "hover", "fill": "#FF00AA" }
                        ]
                    },
                    {
                        "id": "m-plain",
                        "name": "Plain",
                        "group": "g-btn",
                        "frame": { "x": 680, "y": 100, "w": 120, "h": 100 },
                        "kind": { "type": "shape", "shape": "rectangle", "fill": "#776655" }
                    },
                    {
                        "id": "btn-disable",
                        "name": "Disable",
                        "frame": { "x": 100, "y": 320, "w": 120, "h": 60 },
                        "kind": { "type": "button", "label": "Disable" }
                    }
                ],
                "triggers": [
                    {
                        "when": { "event": "click", "object": "btn-disable" },
                        "action": { "action": "change_state", "object": "g-btn", "state": "disabled" }
                    }
                ]
            }]
        }]
    });
    let course: Course = serde_json::from_value(raw).expect("valid course");
    let mut player = Player::new(&course).expect("player");
    player.handle(&Input::Start);

    let has_magenta_text_pixel = |img: &crate::offscreen::RgbaImage| -> bool {
        (100..200).any(|y| {
            (420..640).any(|x| {
                let [r, g, b, _] = pixel_at(img, x, y);
                r > 180 && g < 80 && b > 110
            })
        })
    };

    // Baseline (unhovered) pixels.
    let normal_items = crate::compose::compose_with_pointer(&player, None, None);
    let normal_img =
        render_items_offscreen(&course, &normal_items, crate::paint::Highlight::default());
    assert_eq!(pixel_at(&normal_img, 160, 150), [0x22, 0x44, 0x88, 0xFF]);
    assert_eq!(pixel_at(&normal_img, 320, 150), [0x33, 0x55, 0x99, 0xFF]);
    assert!(!has_magenta_text_pixel(&normal_img));
    assert_eq!(pixel_at(&normal_img, 740, 150), [0x77, 0x66, 0x55, 0xFF]);

    // Hovering ANY member (test member 4, which has no hover state of its own, and member 2)
    // puts members 1–3 into hover while member 4 stays normal.
    let ctx = egui::Context::default();
    let mut view = SlideView::new();
    for (i, (mx, my)) in [(740.0, 150.0), (320.0, 150.0)].into_iter().enumerate() {
        run_slide_frame(
            &ctx,
            &mut view,
            &player,
            pointer_move(mx, my),
            i as f64 * 0.1,
        );
        assert_eq!(view.hovered_object(), Some("g-btn"));
        let hover_img = render_items_offscreen(
            &course,
            view.last_items(),
            crate::paint::Highlight {
                hovered: Some(crate::compose::Hit::Object("g-btn".to_owned())),
                pressed: None,
            },
        );
        assert_eq!(
            pixel_at(&hover_img, 160, 150),
            [0x11, 0xAA, 0x22, 0xFF],
            "member 1 must render hover fill when hovering ({mx}, {my})"
        );
        assert_eq!(
            pixel_at(&hover_img, 320, 150),
            [0x22, 0xBB, 0xCC, 0xFF],
            "member 2 must render hover fill when hovering ({mx}, {my})"
        );
        assert!(
            has_magenta_text_pixel(&hover_img),
            "member 3 text must render hover colour when hovering ({mx}, {my})"
        );
        assert_eq!(
            pixel_at(&hover_img, 740, 150),
            [0x77, 0x66, 0x55, 0xFF],
            "member 4 with no hover state must stay normal"
        );
    }

    // Pressing any member renders `down` pixels via `resolve_pointer_fill` (member 1 uses down
    // fill, member 2 keeps hover fill while pressed+hovered, member 4 stays normal).
    run_slide_frame(&ctx, &mut view, &player, pointer_press(320.0, 150.0), 0.3);
    let down_img = render_items_offscreen(
        &course,
        view.last_items(),
        crate::paint::Highlight {
            hovered: Some(crate::compose::Hit::Object("g-btn".to_owned())),
            pressed: Some(crate::compose::Hit::Object("g-btn".to_owned())),
        },
    );
    assert_eq!(
        pixel_at(&down_img, 160, 150),
        [0xDD, 0x33, 0x11, 0xFF],
        "member 1 must render down fill while pressed"
    );
    assert_eq!(
        pixel_at(&down_img, 320, 150),
        [0x22, 0xBB, 0xCC, 0xFF],
        "member 2 without down state keeps hover fill via resolve_pointer_fill while pressed+hovered"
    );
    assert_eq!(
        pixel_at(&down_img, 740, 150),
        [0x77, 0x66, 0x55, 0xFF],
        "member 4 without down/hover states stays normal while pressed"
    );

    // Disabling the group ignores hover and down pixels on EVERY member.
    player.handle(&Input::Click {
        object: "btn-disable".to_owned(),
    });
    assert_eq!(player.object_state("g-btn"), Some("disabled"));
    run_slide_frame(&ctx, &mut view, &player, pointer_press(160.0, 150.0), 0.4);
    let disabled_img = render_items_offscreen(
        &course,
        view.last_items(),
        crate::paint::Highlight {
            hovered: Some(crate::compose::Hit::Object("g-btn".to_owned())),
            pressed: Some(crate::compose::Hit::Object("g-btn".to_owned())),
        },
    );
    assert_eq!(
        pixel_at(&disabled_img, 160, 150),
        [0x22, 0x44, 0x88, 0xFF],
        "disabled member 1 must ignore hover/down"
    );
    assert_eq!(
        pixel_at(&disabled_img, 320, 150),
        [0x33, 0x55, 0x99, 0xFF],
        "disabled member 2 must ignore hover/down"
    );
    assert!(
        !has_magenta_text_pixel(&disabled_img),
        "disabled member 3 text must ignore hover colour"
    );
    assert_eq!(
        pixel_at(&disabled_img, 740, 150),
        [0x77, 0x66, 0x55, 0xFF],
        "disabled member 4 stays normal"
    );

    // PrintSlide exclude omits all members of an excluded interactive group.
    struct EmptyMedia;
    impl crate::media::MediaSource for EmptyMedia {
        fn get(&self, _src: &str) -> crate::media::MediaFetch {
            crate::media::MediaFetch::Missing
        }
    }
    let empty_media = EmptyMedia;
    let excluded_img = crate::offscreen::render_player_rgba(
        &player,
        1280,
        &["g-btn".to_owned()],
        &empty_media,
        None,
    )
    .expect("excluded render");
    assert_eq!(pixel_at(&excluded_img, 160, 150), [0xFF, 0xFF, 0xFF, 0xFF]);
    assert_eq!(pixel_at(&excluded_img, 320, 150), [0xFF, 0xFF, 0xFF, 0xFF]);
    assert_eq!(pixel_at(&excluded_img, 740, 150), [0xFF, 0xFF, 0xFF, 0xFF]);
}
