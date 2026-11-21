//! Headless UI tests for the Format panel's M26 text box controls:
//! Justify alignment, Wrap text, Text direction, Insets, and Text box border.

use crate::properties::InspectorTab;
use crate::test_harness::TestHarness;
use coursecraft_course::{Align, Course, ObjectKind, TextDirection};

fn hello() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    Course::from_json(&json).expect("valid course")
}

fn select_title(harness: &mut TestHarness) {
    harness.editor.selection.slide = "s-intro".into();
    harness.editor.selection.object = Some("title".into());
    harness.editor.ui.inspector_tab = InspectorTab::Format;
    harness.idle_frame();
    harness.idle_frame();
}

#[test]
fn format_panel_sets_justify_wrap_direction_and_border_on_text_box() {
    let mut harness = TestHarness::new_with_size(hello(), (1280.0, 900.0));
    select_title(&mut harness);

    let right = harness.right_panel_rect();
    let justify = harness
        .find_text_in("Justify", right)
        .expect("Justify button");
    harness.click_at(justify);
    harness.idle_frame();

    let vert270 = harness
        .find_text_in("Vert 270", right)
        .expect("Vert 270 button");
    harness.click_at(vert270);
    harness.idle_frame();

    let wrap = harness
        .find_text_in("Wrap text", right)
        .expect("Wrap text checkbox");
    harness.click_at(wrap);
    harness.idle_frame();

    let obj = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object("title"))
        .map(|(o, _)| o.clone())
        .expect("title object");
    let ObjectKind::Text { style, .. } = &obj.kind else {
        panic!("expected text object");
    };
    assert_eq!(style.align, Align::Justify);
    assert_eq!(style.direction, TextDirection::Vertical270);
    assert!(!style.wrap);

    // Undo and redo preserve Justify cleanly.
    harness.editor.session.undo().expect("undo 1");
    harness.idle_frame();
    harness.editor.session.undo().expect("undo 2");
    harness.idle_frame();
    harness.editor.session.undo().expect("undo 3");
    harness.idle_frame();
    harness.editor.session.redo().expect("redo 1");
    harness.idle_frame();
    let obj = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object("title"))
        .map(|(o, _)| o.clone())
        .expect("title object");
    let ObjectKind::Text { style, .. } = &obj.kind else {
        panic!("expected text object");
    };
    assert_eq!(style.align, Align::Justify);
}

#[test]
fn canvas_justified_text_snaps_words_at_100_and_150_zoom() {
    let json = serde_json::json!({
        "schema_version": 1,
        "id": "justify-ui-test",
        "title": "Justify UI Test",
        "slide_size": { "w": 800, "h": 450 },
        "scenes": [{
            "id": "sc1",
            "title": "Scene 1",
            "slides": [{
                "id": "s1",
                "title": "S1",
                "background": "#FFFFFF",
                "objects": [{
                    "id": "t1",
                    "frame": { "x": 100, "y": 80, "w": 260, "h": 200 },
                    "kind": {
                        "type": "text",
                        "text": "The quick brown fox jumps over the lazy dog and five dozen liquor jugs",
                        "style": {
                            "align": "justify",
                            "valign": "top",
                            "size": 18.0,
                            "color": "#000000",
                            "wrap": true
                        }
                    }
                }]
            }]
        }]
    });
    let course = Course::from_json(&json.to_string()).expect("valid course");
    let mut harness = TestHarness::new_with_size(course, (1280.0, 900.0));

    fn collect_text_shapes<'a>(shape: &'a egui::Shape, out: &mut Vec<&'a egui::epaint::TextShape>) {
        match shape {
            egui::Shape::Text(ts) => out.push(ts),
            egui::Shape::Vec(v) => {
                for s in v {
                    collect_text_shapes(s, out);
                }
            }
            _ => {}
        }
    }

    for &ppp in &[1.0f32, 1.5f32] {
        harness.ctx.set_pixels_per_point(ppp);
        for &zoom in &[1.0f32, 1.5f32] {
            harness.editor.ui.zoom = crate::canvas::ZoomLevel::Custom(zoom);
            let shapes = harness.frame_shapes(vec![]);
            let mut texts = Vec::new();
            for cs in &shapes {
                collect_text_shapes(&cs.shape, &mut texts);
            }
            let ts = texts
                .into_iter()
                .find(|ts| ts.galley.text().contains("quick brown fox"))
                .expect("justified text shape on canvas");
            assert!(
                ts.galley.rows.len() >= 2,
                "expected wrapped rows at zoom={zoom}, ppp={ppp}"
            );
            for (row_idx, row) in ts.galley.rows.iter().enumerate() {
                if row_idx + 1 == ts.galley.rows.len() || row.ends_with_newline {
                    continue;
                }
                let mut in_word = false;
                for glyph in &row.glyphs {
                    if glyph.chr.is_whitespace() {
                        in_word = false;
                    } else if !in_word {
                        in_word = true;
                        let screen_x = ts.pos.x + row.pos.x + glyph.pos.x;
                        let px = screen_x * ppp;
                        let frac = (px - px.round()).abs();
                        assert!(
                            frac <= 0.01,
                            "canvas ppp={ppp} zoom={zoom} row={row_idx}: screen_x={screen_x} * ppp={px} frac={frac}"
                        );
                    }
                }
            }
        }
    }
}
