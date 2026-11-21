//! M26 bug-hunt tests for A2: Gradients (UI & Format panel + render checks).
//!
//! Ordered P1 -> P2 -> P3.

use crate::properties::InspectorTab;
use crate::test_harness::TestHarness;
use coursecraft_course::{
    Color, Course, FillStyle, GradientStop, Object, ObjectKind, Rect, ShapeKind,
};
use coursecraft_render::{MediaFetch, MediaSource, RenderMode, RenderOptions, render_slide_rgba};
use egui::{Key, Modifiers, pos2};

struct NoMedia;
impl MediaSource for NoMedia {
    fn get(&self, _src: &str) -> MediaFetch {
        MediaFetch::Missing
    }
}

fn hello() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    Course::from_json(&json).expect("valid course")
}

fn select_object(h: &mut TestHarness, slide: &str, obj: &str) {
    h.editor.selection.slide = slide.into();
    h.editor.selection.layer = None;
    h.editor.selection.set_many([obj]);
    h.editor.ui.inspector_tab = InspectorTab::Format;
    h.idle_frame();
    h.idle_frame();
}

fn fill_style_of(h: &TestHarness, slide: &str, id: &str) -> Option<FillStyle> {
    let (obj, _) = h
        .editor
        .session
        .course
        .slide(slide)
        .and_then(|s| s.object(id))
        .expect(id);
    match &obj.kind {
        ObjectKind::Shape { fill_style, .. } | ObjectKind::Button { fill_style, .. } => {
            fill_style.clone()
        }
        ObjectKind::Path { path } => path.fill_style.clone(),
        _ => None,
    }
}

fn png_bytes() -> Vec<u8> {
    let pixels = vec![200u8; 4 * 4 * 4];
    coursecraft_media::encode_png_rgba(4, 4, &pixels).expect("png")
}

fn hex_field_for_colour(h: &mut TestHarness, row_label: &str) -> egui::Pos2 {
    let panel = h.right_panel_rect();
    let label_pos = h
        .find_text_in(row_label, panel)
        .unwrap_or_else(|| panic!("row '{row_label}' not found in right panel"));
    pos2(panel.min.x + 150.0, label_pos.y)
}

fn stops_of_fill(fs: &FillStyle) -> &[GradientStop] {
    match fs {
        FillStyle::Linear { stops, .. } | FillStyle::Radial { stops, .. } => stops,
        other => panic!("expected gradient FillStyle, got {other:?}"),
    }
}

// ============================================================================
// P1
// ============================================================================

#[test]
fn a2_1_four_eight_and_ten_stop_linear_and_radial_gradients() {
    let mut course = hello();
    let mut big = course.scenes[0].slides[0].objects[0].clone();
    big.id = "big-shape".into();
    big.frame = Rect {
        x: 100.0,
        y: 100.0,
        w: 800.0,
        h: 400.0,
    };
    big.kind = ObjectKind::Shape {
        shape: ShapeKind::Rectangle,
        fill: Color("#FF0000".into()),
        fill_style: None,
        outline: None,
        text: None,
        adjust: Vec::new(),
    };
    course.scenes[0].slides[0].objects.push(big);

    let mut h = TestHarness::new_with_size(course, (1280.0, 1100.0));
    select_object(&mut h, "s-intro", "big-shape");
    h.pick_combo("Solid", "Linear gradient");

    // Add 2 more stops (total 4)
    for _ in 0..2 {
        let add = h
            .find_text_in("Add colour", h.right_panel_rect())
            .expect("Add colour");
        h.click_at(add);
        h.idle_frame();
    }

    // Set 4 distinct stops via engine so we test exact 4-stop rendering & layout
    h.editor
        .session
        .execute(
            "object.set_fill_style",
            &serde_json::json!({
                "slide": "s-intro",
                "object": "big-shape",
                "fill_style": {
                    "type": "linear",
                    "angle": 0.0,
                    "stops": [
                        { "color": "#FF0000", "position": 0.0 },
                        { "color": "#00FF00", "position": 0.33 },
                        { "color": "#0000FF", "position": 0.66 },
                        { "color": "#FFFF00", "position": 1.0 }
                    ]
                }
            }),
        )
        .expect("set 4-stop linear");
    h.idle_frame();

    let report4 = h.layout_report(2);
    assert!(
        report4
            .findings
            .iter()
            .all(|f| !f.starts_with("clipped-widget") && !f.starts_with("overlap")),
        "4-stop layout findings: {:?}",
        report4.findings
    );

    let img = render_slide_rgba(
        &h.editor.session.course,
        "s-intro",
        &RenderOptions {
            width: 1280,
            mode: RenderMode::Player,
            ..Default::default()
        },
        &NoMedia,
    )
    .expect("render 4-stop linear");

    let px = |x: u32, y: u32| -> [u8; 3] {
        let i = ((y * img.width + x) * 4) as usize;
        [img.pixels[i], img.pixels[i + 1], img.pixels[i + 2]]
    };
    let y = 300;
    let p_red = px(110, y);
    let p_green = px(364, y);
    let p_blue = px(628, y);
    let p_yellow = px(890, y);
    assert!(
        p_red[0] > 200 && p_red[1] < 50 && p_red[2] < 50,
        "red: {p_red:?}"
    );
    assert!(
        p_green[1] > 200 && p_green[0] < 50 && p_green[2] < 50,
        "green: {p_green:?}"
    );
    assert!(
        p_blue[2] > 200 && p_blue[0] < 50 && p_blue[1] < 50,
        "blue: {p_blue:?}"
    );
    assert!(
        p_yellow[0] > 200 && p_yellow[1] > 200 && p_yellow[2] < 50,
        "yellow: {p_yellow:?}"
    );

    // Add 4 more (8 stops), then 2 more (10 stops = MAX_GRADIENT_STOPS)
    for _ in 0..6 {
        let panel = h.right_panel_rect();
        for _ in 0..5 {
            if let Some(add) = h.find_text_in("Add colour", panel) {
                h.click_at(add);
                h.idle_frame();
                break;
            }
            h.scroll_over(panel.center(), -150.0);
        }
    }
    let fs10 = fill_style_of(&h, "s-intro", "big-shape").expect("fill_style");
    assert_eq!(stops_of_fill(&fs10).len(), 10);
    assert!(
        h.find_text_in("Add colour", h.right_panel_rect()).is_none(),
        "Add colour should hide at 10 stops"
    );

    // 4-stop radial gradient pixel check
    h.editor
        .session
        .execute(
            "object.set_fill_style",
            &serde_json::json!({
                "slide": "s-intro",
                "object": "big-shape",
                "fill_style": {
                    "type": "radial",
                    "center": [0.5, 0.5],
                    "stops": [
                        { "color": "#FF0000", "position": 0.0 },
                        { "color": "#00FF00", "position": 0.33 },
                        { "color": "#0000FF", "position": 0.66 },
                        { "color": "#FFFF00", "position": 1.0 }
                    ]
                }
            }),
        )
        .expect("set 4-stop radial");
    let rimg = render_slide_rgba(
        &h.editor.session.course,
        "s-intro",
        &RenderOptions {
            width: 1280,
            mode: RenderMode::Player,
            ..Default::default()
        },
        &NoMedia,
    )
    .expect("render 4-stop radial");
    let rpx = |x: u32, y: u32| -> [u8; 3] {
        let i = ((y * rimg.width + x) * 4) as usize;
        [rimg.pixels[i], rimg.pixels[i + 1], rimg.pixels[i + 2]]
    };
    let c_center = rpx(500, 300);
    let c_corner = rpx(110, 110);
    assert!(
        c_center[0] > 200 && c_center[2] < 60,
        "radial center red: {c_center:?}"
    );
    assert!(
        c_corner[0] > 180 && c_corner[1] > 180,
        "radial corner yellow: {c_corner:?}"
    );
}

#[test]
fn a2_2_removing_middle_stop_and_two_stop_minimum() {
    let mut h = TestHarness::new(hello());
    select_object(&mut h, "s-intro", "btn-start");
    h.editor
        .session
        .execute(
            "object.set_fill_style",
            &serde_json::json!({
                "slide": "s-intro",
                "object": "btn-start",
                "fill_style": {
                    "type": "linear",
                    "angle": 0.0,
                    "stops": [
                        { "color": "#FF0000", "position": 0.0 },
                        { "color": "#00FF00", "position": 0.33 },
                        { "color": "#0000FF", "position": 0.66 },
                        { "color": "#FFFF00", "position": 1.0 }
                    ]
                }
            }),
        )
        .expect("set 4-stop");
    h.idle_frame();
    h.idle_frame();

    let panel = h.right_panel_rect();
    let xs = h.find_texts("×");
    let panel_xs: Vec<_> = xs.into_iter().filter(|p| panel.contains(*p)).collect();
    assert!(panel_xs.len() >= 2, "expected delete buttons for stops");
    // Delete stop 2 (#00FF00)
    h.click_at(panel_xs[1]);
    h.idle_frame();
    let fs3 = fill_style_of(&h, "s-intro", "btn-start").expect("fs3");
    let c3: Vec<&str> = stops_of_fill(&fs3)
        .iter()
        .map(|s| s.color.0.as_str())
        .collect();
    assert_eq!(c3, vec!["#FF0000", "#0000FF", "#FFFF00"]);

    // Delete stop 2 again (#0000FF)
    let panel_xs2: Vec<_> = h
        .find_texts("×")
        .into_iter()
        .filter(|p| panel.contains(*p))
        .collect();
    h.click_at(panel_xs2[1]);
    h.idle_frame();
    let fs2 = fill_style_of(&h, "s-intro", "btn-start").expect("fs2");
    let c2: Vec<&str> = stops_of_fill(&fs2)
        .iter()
        .map(|s| s.color.0.as_str())
        .collect();
    assert_eq!(c2, vec!["#FF0000", "#FFFF00"]);
    assert!(
        h.find_text_in("×", panel).is_none(),
        "remove button should hide at 2-stop minimum"
    );

    // Undo twice restores 4 stops
    assert!(h.editor.session.undo().is_ok());
    assert!(h.editor.session.undo().is_ok());
    let fs4 = fill_style_of(&h, "s-intro", "btn-start").expect("fs4");
    assert_eq!(stops_of_fill(&fs4).len(), 4);
}

#[test]
fn a2_3_adding_stop_and_immediately_editing_hex_after_sort() {
    let mut h = TestHarness::new_with_size(hello(), (1280.0, 1000.0));
    select_object(&mut h, "s-intro", "btn-start");
    h.editor
        .session
        .execute(
            "object.set_fill_style",
            &serde_json::json!({
                "slide": "s-intro",
                "object": "btn-start",
                "fill_style": {
                    "type": "linear",
                    "angle": 0.0,
                    "stops": [
                        { "color": "#FF0000", "position": 0.0 },
                        { "color": "#0000FF", "position": 1.0 }
                    ]
                }
            }),
        )
        .expect("set 2-stop");
    h.idle_frame();

    let add = h
        .find_text_in("Add colour", h.right_panel_rect())
        .expect("Add colour");
    h.click_at(add);
    h.idle_frame();

    let c2_hex = hex_field_for_colour(&mut h, "Colour 2");
    h.set_field(c2_hex, "#00FF00");
    let fs = fill_style_of(&h, "s-intro", "btn-start").expect("fs");
    let stops = stops_of_fill(&fs);
    assert_eq!(stops.len(), 3);
    assert_eq!(stops[0].color.0, "#FF0000");
    assert_eq!(stops[1].color.0, "#00FF00");
    assert_eq!(stops[2].color.0, "#0000FF");
}

#[test]
fn a2_6_fill_type_round_trip_and_undo() {
    let mut h = TestHarness::new(hello());
    select_object(&mut h, "s-intro", "btn-start");

    h.pick_combo("Solid", "Linear gradient");
    let add = h
        .find_text_in("Add colour", h.right_panel_rect())
        .expect("Add colour");
    h.click_at(add);
    h.idle_frame();
    assert_eq!(
        stops_of_fill(&fill_style_of(&h, "s-intro", "btn-start").expect("lin")).len(),
        3
    );

    h.pick_combo("Linear gradient", "Picture");
    h.editor.import_image("photo.png", &png_bytes());
    h.idle_frame();
    assert!(matches!(
        fill_style_of(&h, "s-intro", "btn-start"),
        Some(FillStyle::Picture { .. })
    ));

    h.pick_combo("Picture", "Radial gradient");
    assert!(matches!(
        fill_style_of(&h, "s-intro", "btn-start"),
        Some(FillStyle::Radial { .. })
    ));

    h.pick_combo("Radial gradient", "Solid");
    assert!(fill_style_of(&h, "s-intro", "btn-start").is_none());

    for _ in 0..4 {
        assert!(h.editor.session.undo().is_ok());
    }
    h.idle_frame();
    assert!(matches!(
        fill_style_of(&h, "s-intro", "btn-start"),
        Some(FillStyle::Linear { .. })
    ));
}

#[test]
fn a2_8_duplicate_copy_paste_and_format_painter_with_4_stop_gradient() {
    let mut course = hello();
    let s1 = Object {
        id: "s1".into(),
        name: "S1".into(),
        frame: Rect {
            x: 80.0,
            y: 80.0,
            w: 160.0,
            h: 100.0,
        },
        kind: ObjectKind::Shape {
            shape: ShapeKind::Rectangle,
            fill: Color("#FF0000".into()),
            fill_style: Some(FillStyle::Linear {
                angle: 135.0,
                stops: vec![
                    GradientStop {
                        color: Color("#FF0000".into()),
                        position: 0.0,
                    },
                    GradientStop {
                        color: Color("#00FF00".into()),
                        position: 0.33,
                    },
                    GradientStop {
                        color: Color("#0000FF".into()),
                        position: 0.66,
                    },
                    GradientStop {
                        color: Color("#FFFF00".into()),
                        position: 1.0,
                    },
                ],
            }),
            outline: None,
            text: None,
            adjust: Vec::new(),
        },
        ..course.scenes[0].slides[0].objects[0].clone()
    };
    let p1: Object = serde_json::from_value(serde_json::json!({
        "id": "p1",
        "name": "P1",
        "frame": { "x": 320.0, "y": 80.0, "w": 120.0, "h": 120.0 },
        "kind": {
            "type": "path",
            "points": [
                { "x": 0.0, "y": 0.0 },
                { "x": 120.0, "y": 0.0 },
                { "x": 60.0, "y": 120.0 }
            ],
            "closed": true,
            "outline": { "color": "#000000", "width": 2.0 },
            "fill": "#CCCCCC"
        }
    }))
    .expect("p1");
    course.scenes[0].slides[0].objects.extend([s1, p1]);

    let mut h = TestHarness::new(course);
    select_object(&mut h, "s-intro", "s1");

    // 1. Duplicate (Ctrl/Cmd+D)
    let count_before = h
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("s")
        .objects
        .len();
    h.press(Key::D, Modifiers::COMMAND);
    h.idle_frame();
    let slide = h.editor.session.course.slide("s-intro").expect("s");
    assert_eq!(slide.objects.len(), count_before + 1);
    let dup = slide.objects.last().expect("dup");
    assert_eq!(
        fill_style_of(&h, "s-intro", &dup.id),
        fill_style_of(&h, "s-intro", "s1")
    );

    // 2. Copy/Paste to Slide 2
    select_object(&mut h, "s-intro", "s1");
    h.frame(vec![egui::Event::Copy]);
    let copied_text = h.editor.ui.clipboard_text.clone().unwrap_or_default();
    h.idle_frame();
    let s2_id = h.editor.session.course.scenes[0].slides[1].id.clone();
    let s2_title = h.editor.session.course.scenes[0].slides[1].title.clone();
    let s2_before = h
        .editor
        .session
        .course
        .slide(&s2_id)
        .expect("s2")
        .objects
        .len();
    let s2_row = h
        .find_text_in(&s2_title, h.left_panel_rect())
        .expect("Slide 2 in left panel");
    h.click_at(s2_row);
    h.idle_frame();
    let central = h.central_panel_rect();
    h.click_at(central.min + egui::vec2(20.0, 20.0));
    h.idle_frame();
    h.frame(vec![egui::Event::Paste(copied_text)]);
    h.idle_frame();
    let s2_objs = &h.editor.session.course.slide(&s2_id).expect("s2").objects;
    assert_eq!(
        s2_objs.len(),
        s2_before + 1,
        "paste should add object to Slide 2"
    );
    let pasted = s2_objs.last().expect("pasted");
    assert_eq!(
        fill_style_of(&h, &s2_id, &pasted.id),
        fill_style_of(&h, "s-intro", "s1")
    );

    // 3. Format Painter onto Button btn-start
    let s1_title = h.editor.session.course.scenes[0].slides[0].title.clone();
    let s1_row = h
        .find_text_in(&s1_title, h.left_panel_rect())
        .expect("Slide 1 in left panel");
    h.click_at(s1_row);
    h.idle_frame();
    select_object(&mut h, "s-intro", "s1");
    let paint_btn = h.find_text("Paint").expect("Paint toolbar button");
    h.click_at(paint_btn);
    h.idle_frame();
    assert!(h.editor.ui.format_painter.is_some());
    let btn_rect = crate::canvas::object_screen_rect(
        &h.editor.session.course,
        "s-intro",
        "btn-start",
        central,
    )
    .expect("btn-start rect");
    h.click_at(btn_rect.center());
    h.idle_frame();
    assert_eq!(
        fill_style_of(&h, "s-intro", "btn-start"),
        fill_style_of(&h, "s-intro", "s1")
    );

    // 4. Format Painter onto closed Path p1
    select_object(&mut h, "s-intro", "s1");
    let paint_btn2 = h.find_text("Paint").expect("Paint toolbar button");
    h.click_at(paint_btn2);
    h.idle_frame();
    let p1_rect =
        crate::canvas::object_screen_rect(&h.editor.session.course, "s-intro", "p1", central)
            .expect("p1 rect");
    h.click_at(p1_rect.center());
    h.idle_frame();
    assert_eq!(
        fill_style_of(&h, "s-intro", "p1"),
        fill_style_of(&h, "s-intro", "s1"),
        "Format Painter should copy 4-stop gradient onto closed Path"
    );
}

// ============================================================================
// P2
// ============================================================================

#[test]
fn a2_4_same_position_hard_step_stops_and_reorder() {
    let mut course = hello();
    let mut s1 = course.scenes[0].slides[0].objects[0].clone();
    s1.id = "step-rect".into();
    s1.frame = Rect {
        x: 100.0,
        y: 100.0,
        w: 800.0,
        h: 200.0,
    };
    s1.kind = ObjectKind::Shape {
        shape: ShapeKind::Rectangle,
        fill: Color("#FF0000".into()),
        fill_style: Some(FillStyle::Linear {
            angle: 0.0,
            stops: vec![
                GradientStop {
                    color: Color("#FF0000".into()),
                    position: 0.0,
                },
                GradientStop {
                    color: Color("#FF0000".into()),
                    position: 0.5,
                },
                GradientStop {
                    color: Color("#0000FF".into()),
                    position: 0.5,
                },
                GradientStop {
                    color: Color("#0000FF".into()),
                    position: 1.0,
                },
            ],
        }),
        outline: None,
        text: None,
        adjust: Vec::new(),
    };
    course.scenes[0].slides[0].objects.push(s1);

    let mut h = TestHarness::new_with_size(course, (1280.0, 1000.0));
    select_object(&mut h, "s-intro", "step-rect");

    let img = render_slide_rgba(
        &h.editor.session.course,
        "s-intro",
        &RenderOptions {
            width: 1280,
            mode: RenderMode::Player,
            ..Default::default()
        },
        &NoMedia,
    )
    .expect("render hard step");
    let px = |x: u32, y: u32| -> [u8; 3] {
        let i = ((y * img.width + x) * 4) as usize;
        [img.pixels[i], img.pixels[i + 1], img.pixels[i + 2]]
    };
    let left_mid = px(490, 200);
    let right_mid = px(510, 200);
    assert!(
        left_mid[0] > 220 && left_mid[2] < 30,
        "left of 50%: {left_mid:?}"
    );
    assert!(
        right_mid[2] > 220 && right_mid[0] < 30,
        "right of 50%: {right_mid:?}"
    );

    // Click ↓ on Colour 2 to swap tied stops
    let panel = h.right_panel_rect();
    let downs: Vec<_> = h
        .find_texts("↓")
        .into_iter()
        .filter(|p| panel.contains(*p))
        .collect();
    assert!(downs.len() >= 2);
    h.click_at(downs[1]);
    h.idle_frame();
    assert!(h.editor.session.undo().is_ok());
}

#[test]
fn a2_9_multi_select_preserves_gradient_stops() {
    let mut course = hello();
    let mut s1 = course.scenes[0].slides[0].objects[0].clone();
    s1.id = "grad1".into();
    s1.kind = ObjectKind::Shape {
        shape: ShapeKind::Rectangle,
        fill: Color("#FF0000".into()),
        fill_style: Some(FillStyle::Linear {
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
        }),
        outline: None,
        text: None,
        adjust: Vec::new(),
    };
    let mut s2 = s1.clone();
    s2.id = "solid2".into();
    s2.kind = ObjectKind::Shape {
        shape: ShapeKind::Rectangle,
        fill: Color("#112233".into()),
        fill_style: None,
        outline: None,
        text: None,
        adjust: Vec::new(),
    };
    course.scenes[0].slides[0].objects.extend([s1, s2]);

    let mut h = TestHarness::new(course);
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.set_many(["grad1", "solid2"]);
    h.editor.ui.inspector_tab = InspectorTab::Format;
    h.idle_frame();
    h.idle_frame();

    let panel = h.right_panel_rect();
    let op_label = h.find_text_in("Opacity", panel).expect("Opacity label");
    let op_field = pos2(panel.min.x + 150.0, op_label.y);
    h.set_field(op_field, "80");

    let report = h.layout_report(2);
    assert!(
        report
            .findings
            .iter()
            .all(|f| !f.starts_with("clipped-widget") && !f.starts_with("overlap")),
        "multi-select findings: {:?}",
        report.findings
    );

    let fs = fill_style_of(&h, "s-intro", "grad1").expect("grad1 still has fill_style");
    assert_eq!(stops_of_fill(&fs).len(), 3);
}
