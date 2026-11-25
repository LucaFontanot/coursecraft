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

fn shape_fill_and_style(
    harness: &TestHarness,
    id: &str,
) -> (String, Option<coursecraft_course::FillStyle>) {
    let (obj, _) = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object(id))
        .expect("shape");
    match &obj.kind {
        ObjectKind::Shape {
            fill, fill_style, ..
        } => (fill.0.clone(), fill_style.clone()),
        _ => panic!("expected shape"),
    }
}

fn button_fill(harness: &TestHarness, id: &str) -> String {
    let (obj, _) = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object(id))
        .expect("button");
    match &obj.kind {
        ObjectKind::Button { fill, .. } => fill.0.clone(),
        _ => panic!("expected button"),
    }
}

#[test]
fn shape_fill_toggle_restores_previous_colour_and_gradient() {
    let mut harness = TestHarness::new_with_size(hello(), (1280.0, 1200.0));
    add_shape(&mut harness, "shape-orange", "#F97316", None);
    select_object(&mut harness, "shape-orange");
    let panel = harness.right_panel_rect();
    let control_x = panel.min.x + 144.0;
    let fill_label = harness.find_text_in("Fill", panel).expect("Fill label");

    // Untick Fill -> becomes transparent.
    harness.click_at(pos2(control_x, fill_label.y));
    harness.idle_frame();
    assert_eq!(
        shape_fill_and_style(&harness, "shape-orange").0,
        "#00000000"
    );

    // Re-tick Fill -> restores previous orange colour.
    let fill_label = harness.find_text_in("Fill", panel).expect("Fill label");
    harness.click_at(pos2(control_x, fill_label.y));
    harness.idle_frame();
    assert_eq!(shape_fill_and_style(&harness, "shape-orange").0, "#F97316");

    // Undo / redo still transitions cleanly between transparent and orange.
    harness.editor.session.undo().expect("undo re-tick");
    harness.idle_frame();
    assert_eq!(
        shape_fill_and_style(&harness, "shape-orange").0,
        "#00000000"
    );
    harness.editor.session.redo().expect("redo re-tick");
    harness.idle_frame();
    assert_eq!(shape_fill_and_style(&harness, "shape-orange").0, "#F97316");

    // With a Linear gradient set on the orange shape: untick clears both, re-tick restores both.
    harness
        .editor
        .run(
            "object.set_fill_style",
            serde_json::json!({
                "slide": "s-intro",
                "object": "shape-orange",
                "fill_style": {
                    "type": "linear",
                    "angle": 90.0,
                    "stops": [
                        { "color": "#F97316", "position": 0.0 },
                        { "color": "#FFFFFF", "position": 1.0 }
                    ]
                }
            }),
        )
        .expect("set linear fill_style");
    harness.idle_frame();
    harness.idle_frame();

    let fill_label = harness.find_text_in("Fill", panel).expect("Fill label");
    harness.click_at(pos2(control_x, fill_label.y));
    harness.idle_frame();
    let (fill_off, style_off) = shape_fill_and_style(&harness, "shape-orange");
    assert_eq!(fill_off, "#00000000");
    assert!(style_off.is_none());

    let fill_label = harness.find_text_in("Fill", panel).expect("Fill label");
    harness.click_at(pos2(control_x, fill_label.y));
    harness.idle_frame();
    let (fill_on, style_on) = shape_fill_and_style(&harness, "shape-orange");
    assert_eq!(fill_on, "#F97316");
    assert!(matches!(
        style_on,
        Some(coursecraft_course::FillStyle::Linear { .. })
    ));
}

#[test]
fn button_fill_toggle_restores_previous_colour() {
    let mut harness = TestHarness::new_with_size(hello(), (1280.0, 1200.0));
    harness
        .editor
        .run(
            "object.update",
            serde_json::json!({
                "slide": "s-intro",
                "object": "btn-start",
                "kind": {
                    "type": "button",
                    "label": "Start",
                    "fill": "#F97316"
                }
            }),
        )
        .expect("set button orange");
    select_object(&mut harness, "btn-start");
    let panel = harness.right_panel_rect();
    let control_x = panel.min.x + 144.0;
    let fill_label = harness.find_text_in("Fill", panel).expect("Button Fill");

    harness.click_at(pos2(control_x, fill_label.y));
    harness.idle_frame();
    assert_eq!(button_fill(&harness, "btn-start"), "#00000000");

    let fill_label = harness.find_text_in("Fill", panel).expect("Button Fill");
    harness.click_at(pos2(control_x, fill_label.y));
    harness.idle_frame();
    assert_eq!(button_fill(&harness, "btn-start"), "#F97316");
}

#[test]
fn multi_select_fill_toggle_restores_each_objects_previous_colour() {
    let mut harness = TestHarness::new_with_size(hello(), (1280.0, 1200.0));
    add_shape(&mut harness, "s-orange", "#F97316", None);
    add_shape(&mut harness, "s-green", "#10B981", None);
    harness.editor.selection.slide = "s-intro".into();
    harness.editor.selection.set_many(["s-orange", "s-green"]);
    harness.editor.ui.inspector_tab = InspectorTab::Format;
    harness.idle_frame();
    harness.idle_frame();

    let panel = harness.right_panel_rect();
    let control_x = panel.min.x + 144.0;
    let fill_label = harness.find_text_in("Fill", panel).expect("Multi Fill");

    harness.click_at(pos2(control_x, fill_label.y));
    harness.idle_frame();
    assert_eq!(shape_fill_and_style(&harness, "s-orange").0, "#00000000");
    assert_eq!(shape_fill_and_style(&harness, "s-green").0, "#00000000");

    let fill_label = harness.find_text_in("Fill", panel).expect("Multi Fill");
    harness.click_at(pos2(control_x, fill_label.y));
    harness.idle_frame();
    assert_eq!(shape_fill_and_style(&harness, "s-orange").0, "#F97316");
    assert_eq!(shape_fill_and_style(&harness, "s-green").0, "#10B981");
}

#[test]
fn state_override_fill_toggle_restores_previous_colour() {
    let mut harness = TestHarness::new_with_size(hello(), (1280.0, 1200.0));
    add_shape(&mut harness, "s-state", "#2F6FEB", None);
    harness
        .editor
        .run(
            "object.add_state",
            serde_json::json!({
                "slide": "s-intro",
                "object": "s-state",
                "state": { "name": "hover", "fill": "#F97316" }
            }),
        )
        .expect("add hover state");
    select_object(&mut harness, "s-state");
    harness.editor.ui.bottom_tab = crate::BottomTab::States;
    harness.idle_frame();
    let hover_pos = harness.find_text("hover").expect("hover state card");
    harness.click_at(hover_pos);
    harness.idle_frame();
    let edit_btn = harness.find_text("Edit state").expect("Edit state button");
    harness.click_at(edit_btn);
    harness.idle_frame();
    assert!(harness.editor.ui.editing_state.is_some());

    let panel = harness.right_panel_rect();
    let control_x = panel.min.x + 144.0;
    let fill_label = harness.find_text_in("Fill", panel).expect("State Fill");

    harness.click_at(pos2(control_x, fill_label.y));
    harness.idle_frame();
    let state_fill = |h: &TestHarness| {
        h.editor
            .session
            .course
            .slide("s-intro")
            .and_then(|s| s.object("s-state"))
            .and_then(|(o, _)| o.states.iter().find(|st| st.name == "hover"))
            .and_then(|st| st.fill.as_ref().map(|c| c.0.clone()))
    };
    assert_eq!(state_fill(&harness).as_deref(), Some("#00000000"));

    let fill_label = harness.find_text_in("Fill", panel).expect("State Fill");
    harness.click_at(pos2(control_x, fill_label.y));
    harness.idle_frame();
    assert_eq!(state_fill(&harness).as_deref(), Some("#F97316"));
}

#[test]
fn shape_without_prior_fill_uses_default_when_ticked() {
    let mut harness = TestHarness::new_with_size(hello(), (1280.0, 1200.0));
    add_shape(&mut harness, "s-empty", "#00000000", None);
    select_object(&mut harness, "s-empty");
    let panel = harness.right_panel_rect();
    let control_x = panel.min.x + 144.0;
    let fill_label = harness.find_text_in("Fill", panel).expect("Fill label");

    harness.click_at(pos2(control_x, fill_label.y));
    harness.idle_frame();
    assert_eq!(shape_fill_and_style(&harness, "s-empty").0, "#D0D7DE");
}
