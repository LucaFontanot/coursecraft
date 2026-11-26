//! Package C: inspector and question number fields. Typing in a field that is rebuilt every
//! frame must survive idle frames, and one drag or one typed number is one undo step.

use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use egui::{Pos2, pos2};

fn hello() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    Course::from_json(&json).expect("valid course")
}

fn field_next_to_label(h: &mut TestHarness, label: &str) -> Pos2 {
    let panel = h.right_panel_rect();
    for _ in 0..10 {
        if h.find_text_in(label, panel).is_some() {
            break;
        }
        h.scroll_over(panel.center(), -250.0);
    }
    let label_pos = h
        .find_text_in(label, panel)
        .unwrap_or_else(|| panic!("label '{label}' not found in right panel"));
    pos2(panel.min.x + 150.0, label_pos.y)
}

fn selected(h: &mut TestHarness, id: &str) {
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.object = Some(id.into());
    h.idle_frame();
    h.idle_frame();
}

fn object_focus_order(h: &TestHarness, id: &str) -> Option<u32> {
    h.editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object(id))
        .and_then(|(o, _)| o.focus_order)
}

#[test]
fn object_focus_order_keeps_typed_text_until_enter() {
    let mut h = TestHarness::new_with_size(hello(), (1280.0, 1400.0));
    selected(&mut h, "title");
    let field = field_next_to_label(&mut h, "Focus order");
    h.type_field_then_enter(field, "7");
    assert_eq!(object_focus_order(&h, "title"), Some(7));
}

fn undo_len(h: &TestHarness) -> usize {
    h.editor.session.undo_len()
}

fn id_of(value: &serde_json::Value) -> String {
    value["id"].as_str().expect("id").to_owned()
}

fn draw_slide(h: &mut TestHarness) -> String {
    let bank = id_of(
        &h.editor
            .run("bank.create", serde_json::json!({ "title": "Pool" }))
            .expect("bank.create"),
    );
    for i in 1..=3 {
        h.editor
            .run(
                "bank.add_question",
                serde_json::json!({ "bank": bank, "kind": "multiple_choice", "prompt": format!("Q{i}") }),
            )
            .expect("bank.add_question");
    }
    let scene = h.editor.session.course.scenes[0].id.clone();
    id_of(
        &h.editor
            .run(
                "draw.insert",
                serde_json::json!({ "scene": scene, "bank": bank, "count": 1 }),
            )
            .expect("draw.insert"),
    )
}

#[test]
fn draw_count_keeps_typed_text_until_enter() {
    let mut h = TestHarness::new_with_size(hello(), (1280.0, 1400.0));
    let draw = draw_slide(&mut h);
    h.editor.selection.slide = draw.clone();
    h.editor.selection.object = None;
    h.editor.ui.inspector_tab = crate::InspectorTab::Slide;
    h.idle_frame();
    h.idle_frame();
    let field = field_next_to_label(&mut h, "Count");
    h.type_field_then_enter(field, "2");
    let count = h
        .editor
        .session
        .course
        .slide(&draw)
        .and_then(|s| s.draw.as_ref())
        .map(|d| d.count);
    assert_eq!(count, Some(2));
}

#[test]
fn text_insets_drag_is_one_undo_step() {
    let mut h = TestHarness::new_with_size(hello(), (1280.0, 1400.0));
    selected(&mut h, "title");
    let panel = h.right_panel_rect();
    let label = field_next_to_label(&mut h, "Insets");
    // The L field sits just right of its caption.
    let field = pos2(panel.min.x + 150.0 + 14.0, label.y);
    let before = undo_len(&h);
    h.drag_value_by(field, 30.0, 12);
    assert_eq!(undo_len(&h), before + 1, "one drag is one undo step");
    let left = h
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object("title"))
        .and_then(|(o, _)| match &o.kind {
            coursecraft_course::ObjectKind::Text { style, .. } => style.insets,
            _ => None,
        })
        .map(|i| i.left)
        .unwrap_or(0.0);
    assert!(left > 0.0, "the drag changed the left inset, got {left}");
}

/// Add an object of `kind` to s-intro and select it, with the inspector laid out.
fn add_and_select(h: &mut TestHarness, kind: serde_json::Value) -> String {
    for (name, value) in [
        ("score", serde_json::json!(0.0)),
        ("answer", serde_json::json!("")),
    ] {
        let _ = h.editor.run(
            "variable.add",
            serde_json::json!({ "name": name, "value": value }),
        );
    }
    let id = id_of(
        &h.editor
            .run(
                "object.add",
                serde_json::json!({
                    "slide": "s-intro",
                    "kind": kind,
                    "frame": { "x": 100.0, "y": 100.0, "w": 200.0, "h": 120.0 },
                }),
            )
            .expect("object.add"),
    );
    selected(h, &id);
    id
}

fn object_kind(h: &TestHarness, id: &str) -> serde_json::Value {
    h.editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object(id))
        .map(|(o, _)| serde_json::to_value(&o.kind).expect("kind json"))
        .expect("object")
}

/// Drag the field next to `label` and check that the drag is exactly one undo step and that
/// `key` of the object kind changed.
fn assert_drag_is_one_undo_step(h: &mut TestHarness, id: &str, label: &str, key: &str) {
    let field = field_next_to_label(h, label);
    let before_value = object_kind(h, id)[key].clone();
    let before = undo_len(h);
    h.drag_value_by(field, 40.0, 12);
    assert_eq!(
        undo_len(h),
        before + 1,
        "{label}: one drag is one undo step"
    );
    assert_ne!(
        object_kind(h, id)[key],
        before_value,
        "{label}: the drag changed {key}"
    );
}

#[test]
fn input_max_chars_typed_number_is_one_undo_step() {
    let mut h = TestHarness::new_with_size(hello(), (1280.0, 1400.0));
    let id = add_and_select(
        &mut h,
        serde_json::json!({ "type": "text_entry", "variable": "answer" }),
    );
    let field = field_next_to_label(&mut h, "Max chars");
    let before = undo_len(&h);
    h.type_field_then_enter(field, "50");
    assert_eq!(object_kind(&h, &id)["max_chars"], 50);
    assert_eq!(
        undo_len(&h),
        before + 1,
        "one typed number is one undo step"
    );
}

#[test]
fn input_max_chars_drag_is_one_undo_step() {
    let mut h = TestHarness::new_with_size(hello(), (1280.0, 1400.0));
    let id = add_and_select(
        &mut h,
        serde_json::json!({ "type": "text_entry", "variable": "answer" }),
    );
    assert_drag_is_one_undo_step(&mut h, &id, "Max chars", "max_chars");
}

#[test]
fn dial_angles_and_range_drag_are_one_undo_step() {
    let mut h = TestHarness::new_with_size(hello(), (1280.0, 1400.0));
    let id = add_and_select(
        &mut h,
        serde_json::json!({
            "type": "dial", "variable": "score", "min": 0, "max": 100,
            "start_angle": 0, "end_angle": 270
        }),
    );
    assert_drag_is_one_undo_step(&mut h, &id, "Start angle", "start_angle");
    assert_drag_is_one_undo_step(&mut h, &id, "End angle", "end_angle");
    assert_drag_is_one_undo_step(&mut h, &id, "Max", "max");
    assert_drag_is_one_undo_step(&mut h, &id, "Step", "step");
}

#[test]
fn slider_range_drag_is_one_undo_step() {
    let mut h = TestHarness::new_with_size(hello(), (1280.0, 1400.0));
    let id = add_and_select(
        &mut h,
        serde_json::json!({
            "type": "slider", "variable": "score", "min": 0, "max": 100, "step": 1
        }),
    );
    assert_drag_is_one_undo_step(&mut h, &id, "Max", "max");
}

#[test]
fn scroll_panel_content_height_drag_is_one_undo_step() {
    let mut h = TestHarness::new_with_size(hello(), (1280.0, 1400.0));
    let id = add_and_select(
        &mut h,
        serde_json::json!({ "type": "scroll_panel", "content_height": 400 }),
    );
    assert_drag_is_one_undo_step(&mut h, &id, "Content height", "content_height");
}

#[test]
fn lottie_speed_drag_is_one_undo_step() {
    let mut h = TestHarness::new_with_size(hello(), (1280.0, 1400.0));
    let id = add_and_select(
        &mut h,
        serde_json::json!({ "type": "lottie", "src": "anim.json" }),
    );
    assert_drag_is_one_undo_step(&mut h, &id, "Speed", "speed");
}

#[test]
fn text_box_border_width_drag_is_one_undo_step() {
    let mut h = TestHarness::new_with_size(hello(), (1280.0, 1400.0));
    let mut kind = object_kind(&h, "title");
    kind["outline"] = serde_json::json!({ "color": "#1F1F1F", "width": 2.0 });
    h.editor
        .run(
            "object.update",
            serde_json::json!({ "slide": "s-intro", "object": "title", "kind": kind }),
        )
        .expect("object.update");
    selected(&mut h, "title");
    let row = field_next_to_label(&mut h, "Outline");
    h.idle_frame();
    // The width field sits right of the outline check box.
    let field = pos2(row.x + 20.0, row.y);
    let before = undo_len(&h);
    h.drag_value_by(field, 30.0, 12);
    assert_eq!(undo_len(&h), before + 1, "one drag is one undo step");
    let width = object_kind(&h, "title")["outline"]["width"].as_f64();
    assert!(width.is_some_and(|w| w > 2.0), "width grew, got {width:?}");
}

/// A hotspot question slide with one region, shown in the Question tab.
fn hotspot_slide(h: &mut TestHarness) -> (String, String) {
    h.editor.media.push(
        "media/map.png".into(),
        std::sync::Arc::from(vec![0x89, 0x50, 0x4E, 0x47]),
    );
    let slide = id_of(
        &h.editor
            .run(
                "slide.add",
                serde_json::json!({ "scene": "sc-main", "title": "Q hotspot" }),
            )
            .expect("slide.add"),
    );
    h.editor
        .run(
            "question.insert",
            serde_json::json!({ "slide": slide, "kind": "hotspot" }),
        )
        .expect("question.insert");
    let image = id_of(
        &h.editor
            .run(
                "object.add",
                serde_json::json!({
                    "slide": slide,
                    "kind": { "type": "image", "src": "media/map.png" },
                    "frame": { "x": 80.0, "y": 80.0, "w": 600.0, "h": 400.0 },
                }),
            )
            .expect("add image"),
    );
    h.editor
        .run(
            "question.set_hotspot_image",
            serde_json::json!({ "slide": slide, "image": image }),
        )
        .expect("set_hotspot_image");
    let region = id_of(
        &h.editor
            .run(
                "question.add_region",
                serde_json::json!({
                    "slide": slide, "shape": "rect",
                    "x": 100.0, "y": 100.0, "w": 120.0, "h": 80.0, "correct": true
                }),
            )
            .expect("add_region"),
    );
    h.editor.selection.slide = slide.clone();
    h.editor.selection.object = None;
    h.editor.ui.bottom_tab = crate::BottomTab::Question;
    h.idle_frame();
    h.idle_frame();
    (slide, region)
}

fn region_of(h: &TestHarness, slide: &str, region: &str) -> coursecraft_course::HotspotRegion {
    h.editor
        .session
        .course
        .slide(slide)
        .and_then(|s| s.question.as_ref())
        .and_then(|q| q.regions.iter().find(|r| r.id == region))
        .cloned()
        .expect("region")
}

#[test]
fn hotspot_region_x_and_markers_keep_typed_text_until_enter() {
    let mut h = TestHarness::new_with_size(hello(), (1600.0, 1000.0));
    let (slide, region) = hotspot_slide(&mut h);
    let top = h.bottom_panel_top();
    let area = egui::Rect::from_min_max(pos2(0.0, top), pos2(1600.0, 1000.0));

    let markers = h.find_text_in("Markers", area).expect("Markers label");
    h.type_field_then_enter(pos2(markers.x + 60.0, markers.y), "3");
    let stored = h
        .editor
        .session
        .course
        .slide(&slide)
        .and_then(|s| s.question.as_ref())
        .map(|q| q.markers);
    assert_eq!(stored, Some(3), "markers");

    for _ in 0..8 {
        if h.find_text_in("X", area).is_some() {
            break;
        }
        h.scroll_over(area.center(), -120.0);
    }
    // The region's X value is the first "100" painted in the region list (the Y value is
    // "100" too; X is the left one).
    h.scroll_over(area.center(), -150.0);
    h.idle_frame();
    let x_field = h
        .find_texts("100")
        .into_iter()
        .filter(|p| area.contains(*p))
        .min_by(|a, b| a.x.total_cmp(&b.x))
        .expect("the region X value");
    h.type_field_then_enter(x_field, "250");
    assert_eq!(region_of(&h, &slide, &region).x, 250.0, "region x");
}

#[test]
fn motion_path_duration_keeps_typed_text_until_enter() {
    let mut h = TestHarness::new_with_size(hello(), (1280.0, 1400.0));
    h.editor
        .session
        .course
        .slide_mut("s-intro")
        .and_then(|s| s.objects.iter_mut().find(|o| o.id == "title"))
        .expect("title")
        .motion_paths
        .push(coursecraft_course::MotionPath {
            id: "mp-1".into(),
            relative: true,
            points: vec![
                coursecraft_course::Point { x: 0.0, y: 0.0 },
                coursecraft_course::Point { x: 100.0, y: 0.0 },
            ],
            duration_ms: 1000,
            easing: Default::default(),
        });
    selected(&mut h, "title");
    h.editor.ui.inspector_tab = crate::InspectorTab::Animate;
    h.idle_frame();
    let _ = field_next_to_label(&mut h, "Duration");
    let panel = h.right_panel_rect();
    let durations = h.find_texts("Duration");
    let last = durations.last().copied().expect("a Duration row");
    // The motion path row is the last Duration row; its field sits left of the easing combo.
    h.type_field_then_enter(pos2(panel.min.x + 150.0, last.y), "2500");
    let stored = h
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object("title"))
        .and_then(|(o, _)| o.motion_paths.first().map(|p| p.duration_ms));
    assert_eq!(stored, Some(2500));
}
