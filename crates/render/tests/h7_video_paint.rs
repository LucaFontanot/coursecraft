use coursecraft_course::Course;
use coursecraft_logic::Player;
use coursecraft_render::{
    Highlight, MediaCache, SlideTransform, compose, compose_authoring, paint,
};
use egui::{Color32, Pos2, Rect, Shape, vec2};

fn collect_texts(shapes: &[egui::epaint::ClippedShape]) -> Vec<String> {
    fn walk(shape: &Shape, out: &mut Vec<String>) {
        match shape {
            Shape::Text(t) => out.push(t.galley.text().to_owned()),
            Shape::Vec(list) => {
                for s in list {
                    walk(s, out);
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    for cs in shapes {
        walk(&cs.shape, &mut out);
    }
    out
}

#[test]
fn video_placeholder_text_shows_on_editor_canvas_but_not_in_player() {
    let json = r#"{
      "schema_version": 1,
      "id": "vid-paint",
      "title": "Vid Paint",
      "scenes": [{
        "id": "sc1",
        "title": "Scene 1",
        "slides": [{
          "id": "s1",
          "title": "Slide 1",
          "duration_ms": 5000,
          "objects": [{
            "id": "vid",
            "kind": { "type": "video", "src": "clip.mp4" },
            "frame": { "x": 40, "y": 40, "w": 400, "h": 225 }
          }]
        }]
      }]
    }"#;
    let course = Course::from_json(json).expect("valid course");
    let slide = course.slide("s1").expect("slide");

    let ctx = egui::Context::default();
    let area = Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 720.0));
    let transform = SlideTransform::fit(area, course.slide_size).expect("fit");
    let mut cache = MediaCache::new();

    // Editor canvas: compose_authoring -> placeholder text is drawn.
    let editor_items = compose_authoring(&course, slide, None);
    let editor_out = ctx.run_ui(egui::RawInput::default(), |ui| {
        paint(
            ui.painter(),
            &transform,
            &editor_items,
            Highlight::default(),
            None,
            Some(&course.theme),
            slide.background.as_ref(),
        );
    });
    let editor_texts = collect_texts(&editor_out.shapes);
    assert!(
        editor_texts
            .iter()
            .any(|t| t.contains("Video plays in the published course")),
        "editor canvas must keep placeholder text: {editor_texts:?}"
    );
    editor_out.drop_without_applying_deltas();

    // Player / preview: compose -> placeholder text is NOT drawn.
    let mut player = Player::new(&course).expect("player");
    let _ = player.handle(&coursecraft_logic::Input::Start);
    let player_items = compose(&player);
    let player_out = ctx.run_ui(egui::RawInput::default(), |ui| {
        paint(
            ui.painter(),
            &transform,
            &player_items,
            Highlight::default(),
            None,
            Some(&course.theme),
            slide.background.as_ref(),
        );
        let _ = (Color32::BLACK, &mut cache);
    });
    let player_texts = collect_texts(&player_out.shapes);
    assert!(
        !player_texts
            .iter()
            .any(|t| t.contains("Video plays in the published course")),
        "player/preview must not show placeholder text: {player_texts:?}"
    );
    player_out.drop_without_applying_deltas();
}
