use crate::test_harness::TestHarness;
use coursecraft_course::Course;

fn shadow_test_course() -> Course {
    let raw = serde_json::json!({
        "schema_version": 1,
        "id": "c-u23-shadow",
        "title": "U23 Shadow Opacity",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "scene-1",
            "title": "Scene 1",
            "slides": [{
                "id": "s1",
                "title": "Slide 1",
                "objects": [{
                    "id": "box-1",
                    "name": "Box 1",
                    "frame": { "x": 100, "y": 100, "w": 200, "h": 120 },
                    "kind": { "type": "shape", "shape": "rectangle", "fill": "#2F6FEB" },
                    "shadow": {
                        "color": "#000000FF",
                        "offset_x": 4.0,
                        "offset_y": 4.0,
                        "blur": 6.0
                    },
                    "glow": {
                        "color": "#FFAA00FF",
                        "size": 8.0
                    }
                }]
            }]
        }]
    });
    serde_json::from_value(raw).expect("valid course")
}

fn visible_texts(h: &mut TestHarness) -> Vec<String> {
    let mut out = Vec::new();
    for cs in h.frame_shapes(Vec::new()) {
        if let egui::epaint::Shape::Text(t) = cs.shape {
            out.push(t.galley.text().to_owned());
        }
    }
    out
}

fn scroll_to_effects(harness: &mut TestHarness) {
    for _ in 0..20 {
        if harness.find_text("Effects").is_some_and(|p| p.y < 450.0) {
            break;
        }
        harness.scroll_over(egui::pos2(1140.0, 400.0), -100.0);
        harness.idle_frame();
    }
}

#[test]
fn setting_shadow_opacity_to_40_percent_writes_alpha_0x66_and_undo_restores() {
    let mut harness = TestHarness::new(shadow_test_course());
    harness.editor.selection.set_many(["box-1"]);
    harness.idle_frame();
    scroll_to_effects(&mut harness);

    let texts = visible_texts(&mut harness);
    assert!(
        texts.windows(2).any(|w| w[0] == "100" && w[1] == " %"),
        "100 % shadow opacity should be displayed initially, got: {texts:?}"
    );

    // Edit the Shadow opacity DragValue to 40.
    let drag_rect = harness
        .find_text_last("100")
        .expect("Shadow opacity DragValue");
    harness.click_at(drag_rect);
    harness.idle_frame();
    harness.type_text("40");
    harness.press(egui::Key::Enter, egui::Modifiers::NONE);
    harness.idle_frame();

    let shadow = harness.editor.session.course.scenes[0].slides[0].objects[0]
        .shadow
        .as_ref()
        .expect("shadow present");
    assert_eq!(shadow.color.0, "#00000066");

    // Undo restores the previous alpha in one step.
    assert!(harness.editor.session.undo().is_ok());
    harness.idle_frame();
    let shadow_after_undo = harness.editor.session.course.scenes[0].slides[0].objects[0]
        .shadow
        .as_ref()
        .expect("shadow present after undo");
    assert_eq!(shadow_after_undo.color.0, "#000000FF");
}

#[test]
fn changing_shadow_colour_keeps_chosen_opacity() {
    let mut course = shadow_test_course();
    course.scenes[0].slides[0].objects[0]
        .shadow
        .as_mut()
        .expect("shadow")
        .color
        .0 = "#00000066".to_owned();

    let mut harness = TestHarness::new(course);
    harness.editor.selection.set_many(["box-1"]);
    harness.idle_frame();
    scroll_to_effects(&mut harness);

    // A 40 % shadow (alpha 0x66, e.g. from PPTX) must show 40 %.
    let texts = visible_texts(&mut harness);
    assert!(
        texts.windows(2).any(|w| w[0] == "40" && w[1] == " %"),
        "40 % shadow opacity should be displayed for #00000066, got: {texts:?}"
    );

    // Change shadow colour to #FF0000 (without alpha).
    let color_input = harness
        .find_text("#00000066")
        .or_else(|| harness.find_text("#000000"))
        .expect("shadow color input");
    harness.click_at(color_input);
    harness.idle_frame();
    harness.press(
        egui::Key::A,
        egui::Modifiers {
            command: true,
            ctrl: true,
            ..Default::default()
        },
    );
    harness.type_text("#FF0000");
    harness.press(egui::Key::Enter, egui::Modifiers::NONE);
    harness.idle_frame();

    let shadow = harness.editor.session.course.scenes[0].slides[0].objects[0]
        .shadow
        .as_ref()
        .expect("shadow present");
    assert_ne!(shadow.color.0, "#00000066");
    assert!(
        shadow.color.0.ends_with("66"),
        "expected alpha 0x66 to be preserved, got {}",
        shadow.color.0
    );
}

#[test]
fn setting_glow_opacity_to_40_percent_writes_alpha_0x66() {
    let mut course = shadow_test_course();
    course.scenes[0].slides[0].objects[0].shadow = None;

    let mut harness = TestHarness::new(course);
    harness.editor.selection.set_many(["box-1"]);
    harness.idle_frame();
    scroll_to_effects(&mut harness);

    let drag_rect = harness
        .find_text_last("100")
        .expect("Glow opacity DragValue");
    harness.click_at(drag_rect);
    harness.idle_frame();
    harness.type_text("40");
    harness.press(egui::Key::Enter, egui::Modifiers::NONE);
    harness.idle_frame();

    let glow = harness.editor.session.course.scenes[0].slides[0].objects[0]
        .glow
        .as_ref()
        .expect("glow present");
    assert_eq!(glow.color.0, "#FFAA0066");
}
