//! M26 bug-hunt tests for A11: Format painter (Paint).
//!
//! Ordered P1 -> P2 -> P3.

use crate::test_harness::TestHarness;
use coursecraft_course::{
    Color, Course, FillStyle, GradientStop, Object, ObjectKind, Rect, TextStyle,
};
use egui::{Key, Modifiers, pos2, vec2};

fn hello() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    Course::from_json(&json).expect("valid course")
}

fn two_slide_course() -> Course {
    let json = serde_json::json!({
        "schema_version": 1,
        "id": "fp-test",
        "title": "Format Painter",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "sc1",
            "title": "Scene 1",
            "slides": [
                {
                    "id": "s1",
                    "title": "Slide 1",
                    "duration_ms": 5000,
                    "objects": [
                        {
                            "id": "a",
                            "frame": { "x": 80.0, "y": 80.0, "w": 160.0, "h": 100.0 },
                            "opacity": 0.7,
                            "kind": {
                                "type": "shape",
                                "shape": "rectangle",
                                "fill": "#112233",
                                "outline": { "color": "#FFAA00", "width": 3.0 }
                            }
                        },
                        {
                            "id": "b",
                            "frame": { "x": 320.0, "y": 80.0, "w": 160.0, "h": 100.0 },
                            "kind": { "type": "button", "label": "BtnB", "fill": "#EEEEEE" }
                        },
                        {
                            "id": "c",
                            "frame": { "x": 560.0, "y": 80.0, "w": 160.0, "h": 100.0 },
                            "kind": { "type": "shape", "shape": "ellipse", "fill": "#CCCCCC" }
                        }
                    ]
                },
                {
                    "id": "s2",
                    "title": "Slide 2",
                    "duration_ms": 5000,
                    "objects": [
                        {
                            "id": "d",
                            "frame": { "x": 120.0, "y": 120.0, "w": 180.0, "h": 100.0 },
                            "kind": { "type": "shape", "shape": "rectangle", "fill": "#FFFFFF" }
                        }
                    ]
                }
            ]
        }]
    })
    .to_string();
    Course::from_json(&json).expect("valid")
}

// ============================================================================
// P1
// ============================================================================

#[test]
fn a11_1_cross_slide_format_painting() {
    let mut h = TestHarness::new(two_slide_course());
    h.idle_frame();
    h.idle_frame();
    let central = h.central_panel_rect();
    let a_rect =
        crate::canvas::object_screen_rect(&h.editor.session.course, "s1", "a", central).unwrap();
    h.click_at(a_rect.center());
    h.idle_frame();

    let paint = h.find_text("Paint").expect("Paint button");
    h.click_at(paint);
    h.idle_frame();
    assert!(h.editor.ui.format_painter.is_some());

    let s2_row = h
        .find_text_in("Slide 2", h.left_panel_rect())
        .expect("Slide 2");
    h.click_at(s2_row);
    h.idle_frame();
    assert!(
        h.editor.ui.format_painter.is_some(),
        "format painter should stay armed across slide switch"
    );

    let d_rect =
        crate::canvas::object_screen_rect(&h.editor.session.course, "s2", "d", central).unwrap();
    h.click_at(d_rect.center());
    h.idle_frame();
    assert!(h.editor.ui.format_painter.is_none());

    let d_obj = h
        .editor
        .session
        .course
        .slide("s2")
        .unwrap()
        .object("d")
        .unwrap()
        .0;
    assert!((d_obj.opacity - 0.7).abs() < 0.01);
    match &d_obj.kind {
        ObjectKind::Shape { fill, outline, .. } => {
            assert_eq!(fill.0, "#112233");
            assert!(outline.is_some());
        }
        other => panic!("expected Shape, got {other:?}"),
    }

    assert!(h.editor.session.undo().is_ok());
    let d_undone = h
        .editor
        .session
        .course
        .slide("s2")
        .unwrap()
        .object("d")
        .unwrap()
        .0;
    match &d_undone.kind {
        ObjectKind::Shape { fill, .. } => assert_eq!(fill.0, "#FFFFFF"),
        other => panic!("expected Shape, got {other:?}"),
    }
}

#[test]
fn a11_2_cross_object_kind_matrix() {
    let mut course = two_slide_course();
    if let ObjectKind::Shape { fill_style, .. } = &mut course.scenes[0].slides[0].objects[0].kind {
        *fill_style = Some(FillStyle::Linear {
            angle: 0.0,
            stops: vec![
                GradientStop {
                    color: Color("#FF0000".into()),
                    position: 0.0,
                },
                GradientStop {
                    color: Color("#00FF00".into()),
                    position: 0.5,
                },
                GradientStop {
                    color: Color("#0000FF".into()),
                    position: 1.0,
                },
            ],
        });
    }
    let mut h = TestHarness::new(course);
    h.idle_frame();
    h.idle_frame();
    let central = h.central_panel_rect();

    // Shape -> Button
    h.editor.selection.slide = "s1".into();
    h.editor.selection.object = Some("a".into());
    h.idle_frame();
    let paint = h.find_text("Paint").expect("Paint");
    h.click_at(paint);
    h.idle_frame();
    let b_rect =
        crate::canvas::object_screen_rect(&h.editor.session.course, "s1", "b", central).unwrap();
    h.click_at(b_rect.center());
    h.idle_frame();

    let b_obj = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object("b")
        .unwrap()
        .0;
    match &b_obj.kind {
        ObjectKind::Button {
            label, fill_style, ..
        } => {
            assert_eq!(label, "BtnB");
            assert!(
                matches!(fill_style, Some(FillStyle::Linear { stops, .. }) if stops.len() == 3)
            );
        }
        other => panic!("expected Button, got {other:?}"),
    }
}

#[test]
fn a11_4_keyboard_shortcuts_arm_and_apply_to_multi_select() {
    let mut h = TestHarness::new(two_slide_course());
    h.editor.selection.slide = "s1".into();
    h.editor.selection.object = Some("a".into());
    h.idle_frame();

    h.press(Key::C, Modifiers::COMMAND | Modifiers::ALT);
    h.idle_frame();
    assert!(h.editor.ui.format_painter.is_some());

    h.editor.selection.set_many(["b", "c"]);
    h.idle_frame();
    let rev = h.editor.session.revision;
    h.press(Key::V, Modifiers::COMMAND | Modifiers::ALT);
    h.idle_frame();
    assert_eq!(h.editor.session.revision, rev + 1);
    assert!(h.editor.ui.format_painter.is_none());

    assert!(h.editor.session.undo().is_ok());
    let c_obj = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object("c")
        .unwrap()
        .0;
    match &c_obj.kind {
        ObjectKind::Shape { fill, .. } => assert_eq!(fill.0, "#CCCCCC"),
        other => panic!("expected Shape, got {other:?}"),
    }
}

#[test]
fn a11_7_painting_across_base_and_slide_layer() {
    let mut course = hello();
    for obj in &mut course.scenes[0].slides[0].objects {
        if obj.id == "btn-start"
            && let ObjectKind::Button { fill, .. } = &mut obj.kind
        {
            *fill = Color("#123456".into());
        }
    }
    let mut h = TestHarness::new(course);
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.object = Some("btn-start".into());
    h.idle_frame();
    let paint = h.find_text("Paint").expect("Paint");
    h.click_at(paint);
    h.idle_frame();

    let canvas = h.central_panel_rect();
    let about_pill = h.find_text_in("About", canvas).expect("About pill");
    h.click_at(about_pill);
    h.idle_frame();
    let ap_rect = crate::canvas::object_screen_rect(
        &h.editor.session.course,
        "s-intro",
        "about-panel",
        canvas,
    )
    .expect("about-panel");
    h.click_at(ap_rect.min + vec2(10.0, 10.0));
    h.idle_frame();
    let ap = h
        .editor
        .session
        .course
        .slide("s-intro")
        .unwrap()
        .object("about-panel")
        .unwrap()
        .0;
    match &ap.kind {
        ObjectKind::Shape { fill, .. } => assert_eq!(fill.0, "#123456"),
        other => panic!("expected Shape, got {other:?}"),
    }
}

#[test]
fn a11_9_disabled_state_status_hints_and_single_vs_double_click_toggle() {
    let mut h = TestHarness::new(two_slide_course());
    h.editor.selection.slide = "s1".into();
    h.editor.selection.clear_objects();
    h.frame_at_time(1.0);
    let paint = h.find_text("Paint").expect("Paint");
    h.click_at(paint);
    h.frame_at_time(1.2);
    assert!(h.editor.ui.format_painter.is_none());

    // Single click arms, slow click disarms
    h.editor.selection.object = Some("a".into());
    h.frame_at_time(1.5);
    h.click_at(paint);
    h.frame_at_time(1.7);
    assert!(h.editor.ui.format_painter.is_some());
    assert!(
        h.find_text_containing("Click an object to copy the format")
            .is_some()
    );
    h.frame_at_time(3.0);
    h.click_at(paint);
    h.frame_at_time(3.2);
    assert!(h.editor.ui.format_painter.is_none());
}

// ============================================================================
// P2
// ============================================================================

#[test]
fn a11_3_painting_onto_text_box_preserves_hyperlinks() {
    let mut course = two_slide_course();
    let t1: Object = serde_json::from_value(serde_json::json!({
        "id": "t1",
        "frame": { "x": 80.0, "y": 240.0, "w": 200.0, "h": 60.0 },
        "kind": { "type": "text", "text": "Plain", "style": { "size": 28.0, "color": "#0044AA" } }
    }))
    .expect("t1");
    let t2: Object = serde_json::from_value(serde_json::json!({
        "id": "t2",
        "frame": { "x": 360.0, "y": 240.0, "w": 200.0, "h": 60.0 },
        "kind": {
            "type": "text",
            "text": "Visit Docs",
            "runs": [
                { "text": "Visit " },
                { "text": "Docs", "bold": true, "color": "#FF0000", "link": { "type": "url", "url": "https://example.com" } }
            ]
        }
    }))
    .expect("t2");
    course.scenes[0].slides[0].objects.extend([t1, t2]);

    let mut h = TestHarness::new(course);
    h.editor.selection.slide = "s1".into();
    h.editor.selection.object = Some("t1".into());
    h.idle_frame();
    let paint = h.find_text("Paint").expect("Paint");
    h.click_at(paint);
    h.idle_frame();
    let central = h.central_panel_rect();
    let t2_rect =
        crate::canvas::object_screen_rect(&h.editor.session.course, "s1", "t2", central).unwrap();
    h.click_at(t2_rect.center());
    h.idle_frame();

    let t2_obj = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object("t2")
        .unwrap()
        .0;
    match &t2_obj.kind {
        ObjectKind::Text { style, runs, .. } => {
            assert!((style.size - 28.0).abs() < 0.01);
            assert_eq!(style.color.0, "#0044AA");
            let docs = runs.iter().find(|r| r.text == "Docs").expect("Docs run");
            assert!(docs.bold.is_none());
            assert!(docs.link.is_some(), "hyperlink must be preserved");
        }
        other => panic!("expected Text, got {other:?}"),
    }
}

#[test]
#[ignore = "BUG: Clicking empty canvas while non-sticky Format Painter is armed does not disarm it"]
fn a11_5_clicking_self_empty_canvas_and_right_click_while_armed() {
    let mut h = TestHarness::new(two_slide_course());
    h.editor.selection.slide = "s1".into();
    h.editor.selection.object = Some("a".into());
    h.idle_frame();
    let central = h.central_panel_rect();
    let a_rect =
        crate::canvas::object_screen_rect(&h.editor.session.course, "s1", "a", central).unwrap();

    let paint = h.find_text("Paint").expect("Paint");
    h.click_at(paint);
    h.idle_frame();
    let rev = h.editor.session.revision;
    h.click_at(a_rect.center());
    h.idle_frame();
    assert_eq!(
        h.editor.session.revision, rev,
        "self-click must not push undo"
    );
    assert!(h.editor.ui.format_painter.is_none());

    // Empty canvas click disarms
    h.editor.selection.object = Some("a".into());
    h.idle_frame();
    h.click_at(paint);
    h.idle_frame();
    h.click_at(pos2(central.max.x - 20.0, central.max.y - 20.0));
    h.idle_frame();
    assert!(h.editor.ui.format_painter.is_none());
}

#[test]
fn a11_6_arming_paint_while_inline_edit_open_commits_text() {
    let mut course = two_slide_course();
    let template = course.scenes[0].slides[0].objects[0].clone();
    course.scenes[0].slides[0].objects.push(Object {
        id: "t1".into(),
        name: "T1".into(),
        frame: Rect {
            x: 80.0,
            y: 260.0,
            w: 200.0,
            h: 60.0,
        },
        kind: ObjectKind::Text {
            text: "Hi".into(),
            style: TextStyle {
                size: 32.0,
                bold: true,
                ..Default::default()
            },
            runs: Vec::new(),
            paragraphs: Vec::new(),
            autofit: Default::default(),
            text_style: None,
            outline: None,
        },
        ..template
    });
    let mut h = TestHarness::new(course);
    h.editor.selection.slide = "s1".into();
    h.editor.selection.object = Some("t1".into());
    h.idle_frame();
    let central = h.central_panel_rect();
    let t1_rect =
        crate::canvas::object_screen_rect(&h.editor.session.course, "s1", "t1", central).unwrap();
    h.click_at(t1_rect.center());
    h.click_at(t1_rect.center());
    h.idle_frame();
    assert!(h.editor.ui.inline_edit.is_some());
    h.type_text("!");

    let paint = h.find_text("Paint").expect("Paint");
    h.click_at(paint);
    h.idle_frame();
    assert!(h.editor.ui.inline_edit.is_none());
    assert!(h.editor.ui.format_painter.is_some());
}

#[test]
fn a11_10_paint_onto_rotated_target_with_jitter_does_not_move() {
    let mut course = two_slide_course();
    course.scenes[0].slides[0].objects[1].rotation = 45.0;
    let orig_frame = course.scenes[0].slides[0].objects[1].frame;
    let mut h = TestHarness::new(course);
    h.editor.selection.slide = "s1".into();
    h.editor.selection.object = Some("a".into());
    h.idle_frame();
    let paint = h.find_text("Paint").expect("Paint");
    h.click_at(paint);
    h.idle_frame();

    let central = h.central_panel_rect();
    let b_rect =
        crate::canvas::object_screen_rect(&h.editor.session.course, "s1", "b", central).unwrap();
    h.drag_by(b_rect.center(), vec2(5.0, 3.0), 3, Modifiers::NONE);
    h.idle_frame();

    let b_after = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object("b")
        .unwrap()
        .0;
    assert!((b_after.frame.x - orig_frame.x).abs() < 0.01);
    assert!((b_after.frame.y - orig_frame.y).abs() < 0.01);
    assert!((b_after.opacity - 0.7).abs() < 0.01);
}

// ============================================================================
// P3
// ============================================================================

#[test]
fn a11_8_source_deleted_while_paint_armed_no_crash() {
    let mut h = TestHarness::new(two_slide_course());
    h.editor.selection.slide = "s1".into();
    h.editor.selection.object = Some("a".into());
    h.idle_frame();
    let paint = h.find_text("Paint").expect("Paint");
    h.click_at(paint);
    h.idle_frame();
    h.editor
        .session
        .execute(
            "object.delete",
            &serde_json::json!({ "slide": "s1", "object": "a" }),
        )
        .expect("delete a");
    h.idle_frame();
    let central = h.central_panel_rect();
    let b_rect =
        crate::canvas::object_screen_rect(&h.editor.session.course, "s1", "b", central).unwrap();
    h.click_at(b_rect.center());
    h.idle_frame();
    let b_obj = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object("b")
        .unwrap()
        .0;
    match &b_obj.kind {
        ObjectKind::Button { fill, .. } => assert_eq!(fill.0, "#EEEEEE"),
        other => panic!("expected Button, got {other:?}"),
    }
}
