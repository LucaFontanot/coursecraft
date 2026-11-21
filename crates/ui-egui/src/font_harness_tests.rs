use std::sync::Arc;

use coursecraft_course::Course;
use egui::epaint::{ClippedShape, Shape};

use crate::test_harness::TestHarness;

fn course_with_run_font(font: &str, text: &str) -> Course {
    let json = serde_json::json!({
        "schema_version": 1,
        "id": "ui-font-test",
        "title": "UI Font Test",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "s1",
            "title": "Scene",
            "slides": [{
                "id": "slide1",
                "title": "Slide",
                "objects": [{
                    "id": "t1",
                    "name": "Text 1",
                    "kind": {
                        "type": "text",
                        "text": text,
                        "style": { "font": "Inter", "size": 36, "color": "#000000" },
                        "runs": [{ "text": text, "font": font }]
                    },
                    "frame": { "x": 40, "y": 40, "w": 600, "h": 120 }
                }]
            }]
        }]
    });
    Course::from_json(&json.to_string()).expect("valid course")
}

fn find_text_width(shapes: &[ClippedShape], needle: &str) -> Option<f32> {
    fn walk(shape: &Shape, needle: &str) -> Option<f32> {
        match shape {
            Shape::Text(t) if t.galley.text() == needle => Some(t.galley.size().x),
            Shape::Vec(v) => v.iter().find_map(|s| walk(s, needle)),
            _ => None,
        }
    }
    shapes.iter().find_map(|cs| walk(&cs.shape, needle))
}

#[test]
fn installed_system_font_renders_with_different_glyph_widths_than_inter() {
    let dir = std::env::temp_dir().join("coursecraft-ui-egui-sysfont-test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp font dir");

    let dm_serif =
        include_bytes!("../../../examples/healthy-habits/media/fonts/dm-serif-display-regular.ttf");
    std::fs::write(dir.join("renamed-font.ttf"), dm_serif).expect("write font");

    let index = Arc::new(
        crate::coursecraft_media::system_fonts::SystemFontIndex::from_folders(
            std::slice::from_ref(&dir),
        ),
    );
    let provider: Arc<dyn coursecraft_render::SystemFontProvider> =
        Arc::new(coursecraft_render::IndexFontProvider(index));

    let phrase = "WAVE HAND";

    let mut inter_harness = TestHarness::new(course_with_run_font("Inter", phrase));
    let _ = inter_harness.frame(vec![]);
    let inter_shapes = inter_harness.frame_shapes(vec![]);
    let inter_width = find_text_width(&inter_shapes, phrase).expect("Inter text width");

    let mut sys_harness = TestHarness::new(course_with_run_font("DM Serif Display", phrase));
    sys_harness.editor.set_system_font_provider(Some(provider));
    let _ = sys_harness.frame(vec![]);
    let sys_shapes = sys_harness.frame_shapes(vec![]);
    let sys_width = find_text_width(&sys_shapes, phrase).expect("DM Serif Display text width");

    assert!(
        (inter_width - sys_width).abs() > 1.0,
        "installed system font should have different glyph width than Inter ({inter_width} vs {sys_width})"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn font_picker_handles_over_1000_installed_fonts_fast() {
    let mut harness = TestHarness::new(course_with_run_font("Inter", "Hello"));
    harness.editor.ui.system_fonts = Some(
        (0..1200)
            .map(|i| crate::SystemFontChoice {
                name: format!("Font Family {i:04}"),
                embedding_restricted: i % 10 == 0,
            })
            .collect(),
    );
    // Select the text object so the text panel is shown.
    harness.editor.selection.object = Some("t1".into());
    let _ = harness.frame(vec![]);
    let combo_pos = harness
        .find_text("Inter")
        .expect("font picker combo visible");
    harness.click_at(combo_pos);
    let _ = harness.frame(vec![]);
    assert!(harness.find_text("On this computer").is_some());
}
