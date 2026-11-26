//! M39 Package L: selected object highlights its triggers (#11) and a focused
//! layer shows only itself (#12).

use crate::BottomTab;
use crate::test_harness::TestHarness;
use crate::triggers::RowHighlight;
use coursecraft_course::Course;
use egui::epaint::ClippedShape;
use egui::{Rect, Shape};

fn course() -> Course {
    serde_json::from_value(serde_json::json!({
        "schema_version": 1,
        "id": "m39-l",
        "title": "M39 L",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "scene-1",
            "title": "Scene 1",
            "slides": [
                {
                    "id": "s1",
                    "title": "Slide 1",
                    "duration_ms": 5000,
                    "objects": [
                        { "id": "a", "name": "Alpha",
                          "kind": { "type": "shape", "shape": "rectangle", "fill": "#2563EB", "text": { "text": "AlphaText" } },
                          "frame": { "x": 100.0, "y": 100.0, "w": 160.0, "h": 80.0 } },
                        { "id": "b", "name": "Beta",
                          "kind": { "type": "shape", "shape": "rectangle", "fill": "#2563EB", "text": { "text": "BetaText" } },
                          "frame": { "x": 400.0, "y": 100.0, "w": 160.0, "h": 80.0 } }
                    ],
                    "triggers": [
                        { "when": { "event": "click", "object": "b" },
                          "action": { "action": "pause_timeline" } },
                        { "when": { "event": "click", "object": "a" },
                          "action": { "action": "pause_timeline" } },
                        { "when": { "event": "click", "object": "b" },
                          "action": { "action": "change_state", "object": "a", "state": "Hidden" } }
                    ]
                },
                { "id": "s2", "title": "Slide 2", "duration_ms": 5000, "objects": [] }
            ]
        }]
    }))
    .expect("course")
}

fn harness() -> TestHarness {
    let mut h = TestHarness::new(course());
    h.editor.selection.slide = "s1".into();
    h.editor.selection.layer = None;
    h.editor.selection.object = None;
    h.editor.ui.bottom_tab = BottomTab::Triggers;
    h.idle_frame();
    h.idle_frame();
    h
}

#[test]
fn rows_of_the_selected_object_are_highlighted_and_others_are_not() {
    let mut h = harness();
    assert!(
        h.editor.ui.triggers.row_highlights.is_empty(),
        "nothing selected"
    );
    assert!(h.editor.ui.triggers.last_scroll_key.is_none());

    h.editor.selection.object = Some("a".into());
    h.idle_frame();
    h.idle_frame();
    let marks = h.editor.ui.triggers.row_highlights.clone();
    assert_eq!(marks.get(&1), Some(&RowHighlight::Own), "click on a");
    assert_eq!(
        marks.get(&2),
        Some(&RowHighlight::Related),
        "action names a"
    );
    assert_eq!(marks.get(&0), None, "click on b only");
    assert!(h.editor.ui.triggers.last_scroll_key.is_some());
}

#[test]
fn group_by_object_highlights_the_selected_group_header() {
    let mut h = harness();
    h.editor.ui.triggers.group_by_object = true;
    h.editor.selection.object = Some("b".into());
    h.idle_frame();
    h.idle_frame();
    assert_eq!(
        h.editor.ui.triggers.highlighted_header.as_deref(),
        Some("Beta")
    );
    assert_eq!(
        h.editor.ui.triggers.row_highlights.get(&0),
        Some(&RowHighlight::Own)
    );
    h.editor.selection.object = None;
    h.idle_frame();
    h.idle_frame();
    assert!(h.editor.ui.triggers.highlighted_header.is_none());
}

#[test]
fn a_selection_change_never_opens_or_scrolls_a_hidden_triggers_panel() {
    let mut h = harness();
    h.editor.ui.bottom_tab = BottomTab::Timeline;
    h.idle_frame();
    h.idle_frame();
    h.editor.selection.object = Some("a".into());
    h.idle_frame();
    h.idle_frame();
    assert_eq!(h.editor.ui.bottom_tab, BottomTab::Timeline);
    assert!(h.editor.ui.triggers.last_scroll_key.is_none());
    assert!(h.editor.ui.triggers.row_highlights.is_empty());
}

fn texts_on_canvas(h: &mut TestHarness) -> Vec<String> {
    fn walk(shape: &Shape, canvas: Rect, out: &mut Vec<String>) {
        match shape {
            Shape::Text(t) => {
                if canvas.contains(crate::test_harness::text_center(t)) {
                    out.push(t.galley.text().to_owned());
                }
            }
            Shape::Vec(v) => v.iter().for_each(|s| walk(s, canvas, out)),
            _ => {}
        }
    }
    let canvas = h.central_panel_rect();
    let shapes: Vec<ClippedShape> = h.frame_shapes(vec![]);
    let mut out = Vec::new();
    for cs in &shapes {
        walk(&cs.shape, canvas, &mut out);
    }
    out
}

fn layers_harness() -> (TestHarness, String, String) {
    let mut h = harness();
    h.editor.ui.bottom_tab = BottomTab::Timeline;
    let mut ids = Vec::new();
    for (name, x) in [("L1", 600.0), ("L2", 800.0)] {
        let res = h
            .editor
            .run(
                "layer.add",
                serde_json::json!({ "slide": "s1", "name": name }),
            )
            .expect("layer.add");
        let lid = res["id"].as_str().expect("layer id").to_owned();
        h.editor
            .run(
                "object.add",
                serde_json::json!({
                    "slide": "s1", "layer": lid, "name": format!("{name}Shape"),
                    "kind": { "type": "shape", "shape": "rectangle", "fill": "#16A34A",
                              "text": { "text": format!("{name}Shape") } },
                    "frame": { "x": x, "y": 300.0, "w": 120.0, "h": 60.0 },
                }),
            )
            .expect("object.add");
        ids.push(lid);
    }
    h.idle_frame();
    h.idle_frame();
    let l2 = ids.pop().unwrap_or_default();
    let l1 = ids.pop().unwrap_or_default();
    (h, l1, l2)
}

#[test]
fn a_focused_layer_draws_only_itself_and_the_base_slide() {
    let (mut h, l1, _l2) = layers_harness();
    h.editor.selection.layer = Some(l1);
    h.idle_frame();
    h.idle_frame();
    let texts = texts_on_canvas(&mut h);
    let has = |needle: &str| texts.iter().any(|t| t.contains(needle));
    assert!(has("L1Shape"), "{texts:?}");
    assert!(!has("L2Shape"), "{texts:?}");
}

#[test]
fn turning_another_layers_eye_on_draws_it_and_a_slide_change_resets_it() {
    let (mut h, l1, l2) = layers_harness();
    h.editor.selection.layer = Some(l1);
    h.idle_frame();
    h.idle_frame();
    let eye = h.editor.ui.layers.eye_rects[&l2];
    h.click_at(eye.center());
    h.idle_frame();
    h.idle_frame();
    assert!(h.editor.ui.layers.shown_layers.contains(&l2));
    let texts = texts_on_canvas(&mut h);
    assert!(texts.iter().any(|t| t.contains("L2Shape")), "{texts:?}");

    h.editor.selection.slide = "s2".into();
    h.editor.selection.layer = None;
    h.editor.selection.object = None;
    h.idle_frame();
    h.idle_frame();
    assert!(h.editor.ui.layers.shown_layers.is_empty());
}
