//! Real-UI-path tests for Step f22: positive Fill/Outline toggles and
//! adjustable dash spacing.

use crate::properties::InspectorTab;
use crate::test_harness::TestHarness;
use coursecraft_course::{Course, DashStyle, ObjectKind};
use egui::pos2;

fn hello() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    Course::from_json(&json).expect("valid course")
}

fn add_shape(harness: &mut TestHarness, id: &str, fill: &str, outline: Option<&str>) {
    let outline_json = outline.map_or(serde_json::Value::Null, |c| {
        serde_json::json!({
            "color": c,
            "width": 2.0,
            "dash": "solid"
        })
    });
    harness
        .editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s-intro",
                "id": id,
                "frame": { "x": 100.0, "y": 100.0, "w": 200.0, "h": 120.0 },
                "kind": {
                    "type": "shape",
                    "shape": "rectangle",
                    "fill": fill,
                    "outline": outline_json
                }
            }),
        )
        .expect("object.add");
}

fn select_object(harness: &mut TestHarness, id: &str) {
    harness.editor.selection.slide = "s-intro".into();
    harness.editor.selection.clear_objects();
    harness.editor.selection.object = Some(id.into());
    harness.editor.ui.inspector_tab = InspectorTab::Format;
    harness.idle_frame();
    harness.idle_frame();
}

fn shape_state(harness: &TestHarness, id: &str) -> (String, Option<coursecraft_course::Outline>) {
    let (obj, _) = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object(id))
        .expect("shape");
    match &obj.kind {
        ObjectKind::Shape { fill, outline, .. } => (fill.0.clone(), outline.clone()),
        _ => panic!("expected shape"),
    }
}

#[test]
fn shape_panel_uses_positive_fill_and_outline_toggles_and_undo_works() {
    let mut harness = TestHarness::new_with_size(hello(), (1280.0, 1200.0));
    add_shape(&mut harness, "shape-test", "#2F6FEB", Some("#1F1F1F"));
    select_object(&mut harness, "shape-test");
    let panel = harness.right_panel_rect();
    assert!(harness.find_text_in("No fill", panel).is_none());
    assert!(harness.find_text_in("No outline", panel).is_none());

    let control_x = panel.min.x + 144.0;
    // Untick Fill checkbox.
    let fill_label = harness.find_text_in("Fill", panel).expect("Fill label");
    harness.click_at(pos2(control_x, fill_label.y));
    harness.idle_frame();
    assert_eq!(shape_state(&harness, "shape-test").0, "#00000000");

    // Undo restores fill.
    harness.editor.session.undo().expect("undo fill");
    harness.idle_frame();
    assert_eq!(shape_state(&harness, "shape-test").0, "#2F6FEB");

    // Untick Outline checkbox.
    let outline_label = harness
        .find_text_in("Outline", panel)
        .expect("Outline label");
    harness.click_at(pos2(control_x, outline_label.y));
    harness.idle_frame();
    assert!(shape_state(&harness, "shape-test").1.is_none());

    // Undo restores outline.
    harness.editor.session.undo().expect("undo outline");
    harness.idle_frame();
    assert!(shape_state(&harness, "shape-test").1.is_some());
}

#[test]
fn button_and_text_panels_use_positive_fill_and_outline() {
    let mut harness = TestHarness::new_with_size(hello(), (1280.0, 1200.0));
    select_object(&mut harness, "btn-start");
    let panel = harness.right_panel_rect();
    let control_x = panel.min.x + 144.0;
    let fill_label = harness
        .find_text_in("Fill", panel)
        .expect("Button Fill label");
    harness.click_at(pos2(control_x, fill_label.y));
    harness.idle_frame();
    let btn_fill = match &harness
        .editor
        .session
        .course
        .slide("s-intro")
        .unwrap()
        .object("btn-start")
        .unwrap()
        .0
        .kind
    {
        ObjectKind::Button { fill, .. } => fill.0.clone(),
        _ => panic!("expected button"),
    };
    assert_eq!(btn_fill, "#00000000");
    harness.editor.session.undo().expect("undo button fill");
    harness.idle_frame();

    // Text box has positive "Outline" checkbox.
    harness
        .editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s-intro",
                "id": "txt-test",
                "frame": { "x": 100.0, "y": 250.0, "w": 200.0, "h": 60.0 },
                "kind": { "type": "text", "text": "Hello" }
            }),
        )
        .expect("add text");
    select_object(&mut harness, "txt-test");
    let panel = harness.right_panel_rect();
    let outline_label = harness
        .find_text_in("Outline", panel)
        .expect("Text Outline label");
    harness.click_at(pos2(control_x, outline_label.y));
    harness.idle_frame();
    let has_outline = matches!(
        &harness
            .editor
            .session
            .course
            .slide("s-intro")
            .unwrap()
            .object("txt-test")
            .unwrap()
            .0
            .kind,
        ObjectKind::Text {
            outline: Some(_),
            ..
        }
    );
    assert!(has_outline, "clicking Outline on text box adds an outline");
}

#[test]
fn multi_selection_mixed_fill_and_outline_toggles_both() {
    let mut harness = TestHarness::new_with_size(hello(), (1280.0, 1200.0));
    add_shape(&mut harness, "s-a", "#2F6FEB", Some("#1F1F1F"));
    add_shape(&mut harness, "s-b", "#00000000", None);
    harness.editor.selection.slide = "s-intro".into();
    harness.editor.selection.set_many(["s-a", "s-b"]);
    harness.editor.ui.inspector_tab = InspectorTab::Format;
    harness.idle_frame();
    harness.idle_frame();

    assert_eq!(harness.editor.ui.properties.multi_fill_enabled, None);
    assert_eq!(harness.editor.ui.properties.multi_outline_enabled, None);

    let panel = harness.right_panel_rect();
    let control_x = panel.min.x + 144.0;
    let fill_label = harness.find_text_in("Fill", panel).expect("Fill");
    harness.click_at(pos2(control_x, fill_label.y));
    harness.idle_frame();
    assert_ne!(shape_state(&harness, "s-a").0, "#00000000");
    assert_ne!(shape_state(&harness, "s-b").0, "#00000000");

    let outline_label = harness.find_text_in("Outline", panel).expect("Outline");
    harness.click_at(pos2(control_x, outline_label.y));
    harness.idle_frame();
    assert!(shape_state(&harness, "s-a").1.is_some());
    assert!(shape_state(&harness, "s-b").1.is_some());
}

#[test]
fn dash_panel_edits_dash_length_gap_and_custom_option() {
    let mut harness = TestHarness::new_with_size(hello(), (1280.0, 1200.0));
    add_shape(&mut harness, "shape-dash", "#2F6FEB", Some("#1F1F1F"));
    select_object(&mut harness, "shape-dash");
    harness.pick_combo("Solid", "Dash");
    let panel = harness.right_panel_rect();
    let control_x = panel.min.x + 160.0;
    let dash_len_label = harness
        .find_text_in("Dash length", panel)
        .expect("Dash length field");
    harness.set_field(pos2(control_x, dash_len_label.y), "5");
    harness.idle_frame();
    let gap_label = harness.find_text_in("Gap", panel).expect("Gap field");
    harness.set_field(pos2(control_x, gap_label.y), "4");
    harness.idle_frame();

    let (_, outline) = shape_state(&harness, "shape-dash");
    let outline = outline.expect("outline");
    assert_eq!(outline.dash, DashStyle::Dash);
    assert_eq!(outline.dash_pattern.as_deref(), Some(&[5.0, 4.0][..]));

    harness.pick_combo("Dash", "Custom");
    let (_, outline) = shape_state(&harness, "shape-dash");
    assert_eq!(outline.expect("outline").dash, DashStyle::Custom);
}
