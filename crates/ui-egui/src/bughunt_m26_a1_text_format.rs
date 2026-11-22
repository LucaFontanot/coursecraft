//! M26 bug-hunt tests for A1: Text format panel.
//!
//! Ordered P1 -> P2 -> P3.

use crate::properties::InspectorTab;
use crate::test_harness::TestHarness;
use coursecraft_course::{
    Course, Object, ObjectKind, Rect, ShapeKind, ShapeText, TableBorders, TableCell, TableRow,
    TextRun, TextStyle, TimerFormat,
};
use egui::{Key, Modifiers, Pos2, pos2, vec2};

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
    let label_pos = h
        .find_text_in(label, panel)
        .unwrap_or_else(|| panic!("label '{label}' not found in right panel"));
    pos2(panel.min.x + 150.0, label_pos.y)
}

fn base_object(id: &str, frame: Rect, kind: ObjectKind) -> Object {
    let mut o = hello().scenes[0].slides[0].objects[0].clone();
    o.id = id.into();
    o.frame = frame;
    o.kind = kind;
    o.states.clear();
    o
}

// ============================================================================
// P1
// ============================================================================

#[test]
fn a1_1_timer_size_field_accepts_enter_blur_and_drag() {
    let mut course = hello();
    let timer = base_object(
        "t1",
        Rect {
            x: 100.0,
            y: 100.0,
            w: 160.0,
            h: 60.0,
        },
        ObjectKind::Timer {
            duration_ms: 30_000,
            format: TimerFormat::MmSs,
            autostart: true,
            style: TextStyle {
                size: 24.0,
                ..Default::default()
            },
        },
    );
    course.scenes[0].slides[0].objects.push(timer);

    let mut h = TestHarness::new(course);
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.object = Some("t1".into());
    h.editor.ui.inspector_tab = InspectorTab::Format;
    h.idle_frame();
    h.idle_frame();

    let timer_size = |h: &TestHarness| -> f32 {
        let (obj, _) = h
            .editor
            .session
            .course
            .slide("s-intro")
            .and_then(|s| s.object("t1"))
            .expect("t1");
        match &obj.kind {
            ObjectKind::Timer { style, .. } => style.size,
            other => panic!("expected Timer, got {other:?}"),
        }
    };

    // 1. Typing + Enter
    let panel = h.right_panel_rect();
    let field = h.find_text_in("24", panel).expect("Timer Size DragValue");
    let rev_before = h.editor.session.revision;
    h.set_field(field, "40");
    assert!(
        (timer_size(&h) - 40.0).abs() < 0.01,
        "expected timer size 40.0 on Enter, got {}",
        timer_size(&h)
    );
    assert_eq!(h.editor.session.revision, rev_before + 1);

    // 2. Typing + click-away (blur)
    h.click_at(field);
    h.idle_frame();
    h.press(Key::A, Modifiers::COMMAND);
    h.type_text("52");
    let away = pos2(
        h.right_panel_rect().min.x + 10.0,
        h.right_panel_rect().min.y + 10.0,
    );
    h.click_at(away);
    h.idle_frame();
    assert!(
        (timer_size(&h) - 52.0).abs() < 0.01,
        "expected timer size 52.0 on blur, got {}",
        timer_size(&h)
    );

    // 3. Drag
    let before_drag = timer_size(&h);
    h.drag_by(field, vec2(40.0, 0.0), 6, Modifiers::NONE);
    h.idle_frame();
    assert!(
        (timer_size(&h) - before_drag).abs() > 1.0,
        "expected timer size to change on drag from {before_drag}, got {}",
        timer_size(&h)
    );
}

#[test]
fn a1_2_table_cell_font_size_letter_spacing_and_line_spacing() {
    let mut course = hello();
    let table = base_object(
        "tbl1",
        Rect {
            x: 100.0,
            y: 120.0,
            w: 400.0,
            h: 200.0,
        },
        ObjectKind::Table {
            columns: vec![0.5, 0.5],
            rows: vec![
                TableRow {
                    height: 0.5,
                    cells: vec![
                        TableCell {
                            text: "Cell A".into(),
                            ..Default::default()
                        },
                        TableCell {
                            text: "Cell B".into(),
                            ..Default::default()
                        },
                    ],
                },
                TableRow {
                    height: 0.5,
                    cells: vec![
                        TableCell {
                            text: "Cell C".into(),
                            ..Default::default()
                        },
                        TableCell {
                            text: "Cell D".into(),
                            ..Default::default()
                        },
                    ],
                },
            ],
            borders: TableBorders::default(),
            header_row: false,
        },
    );
    course.scenes[0].slides[0].objects.push(table);

    let mut h = TestHarness::new_with_size(course, (1280.0, 1200.0));
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.object = Some("tbl1".into());
    h.editor.ui.table_cell = Some((0, 1));
    h.editor.ui.inspector_tab = InspectorTab::Format;
    h.idle_frame();
    h.idle_frame();

    let size_field = field_next_to_label(&mut h, "Font size");
    h.set_field(size_field, "28");

    let spacing_field = field_next_to_label(&mut h, "Spacing");
    h.set_field(spacing_field, "2.5");

    let panel = h.right_panel_rect();
    for _ in 0..8 {
        if h.find_text_in("1.00x", panel).is_some() {
            break;
        }
        h.scroll_over(panel.center(), -250.0);
    }
    let combo = h
        .find_text_in("1.00x", panel)
        .expect("line spacing combo in table cell inspector");
    h.click_at(combo);
    h.idle_frame();
    let custom = h
        .find_text_containing("custom")
        .expect("custom line spacing field");
    h.drag_by(custom, vec2(40.0, 0.0), 6, Modifiers::NONE);
    h.idle_frame();

    let cell_stats = |h: &TestHarness, row: usize, col: usize| -> (f32, f32, f32) {
        let (obj, _) = h
            .editor
            .session
            .course
            .slide("s-intro")
            .and_then(|s| s.object("tbl1"))
            .expect("tbl1");
        match &obj.kind {
            ObjectKind::Table { rows, .. } => {
                let c = &rows[row].cells[col];
                let sz = c.runs.first().and_then(|r| r.size).unwrap_or(c.style.size);
                let ls = c
                    .runs
                    .first()
                    .and_then(|r| r.letter_spacing)
                    .unwrap_or(c.style.letter_spacing);
                let ln = c.paragraphs.first().map_or(1.0, |p| p.line_spacing);
                (sz, ls, ln)
            }
            other => panic!("expected Table, got {other:?}"),
        }
    };

    let (b_sz, b_ls, b_ln) = cell_stats(&h, 0, 1);
    let (a_sz, a_ls, a_ln) = cell_stats(&h, 0, 0);
    assert!((b_sz - 28.0).abs() < 0.01, "cell (0,1) size={b_sz}");
    assert!(
        (b_ls - 2.5).abs() < 0.01,
        "cell (0,1) letter_spacing={b_ls}"
    );
    assert!((b_ln - 1.0).abs() > 0.05, "cell (0,1) line_spacing={b_ln}");
    assert!((a_sz - 28.0).abs() > 1.0, "cell (0,0) should be unchanged");
    assert!(a_ls.abs() < 0.01, "cell (0,0) letter_spacing unchanged");
    assert!(
        (a_ln - 1.0).abs() < 0.01,
        "cell (0,0) line_spacing unchanged"
    );

    assert!(h.editor.session.undo().is_ok());
    h.idle_frame();
    let (_, _, ln_after_undo) = cell_stats(&h, 0, 1);
    assert!((ln_after_undo - 1.0).abs() < 0.01);
}

#[test]
fn a1_3_shape_with_text_enter_blur_and_line_spacing() {
    let mut course = hello();
    let badge = base_object(
        "badge",
        Rect {
            x: 200.0,
            y: 200.0,
            w: 220.0,
            h: 120.0,
        },
        ObjectKind::Shape {
            shape: ShapeKind::Rectangle,
            fill: coursecraft_course::Color("#2F6FEB".into()),
            fill_style: None,
            outline: None,
            text: Some(ShapeText {
                text: "Line 1\nLine 2".into(),
                style: TextStyle {
                    size: 18.0,
                    ..Default::default()
                },
                runs: Vec::new(),
                paragraphs: Vec::new(),
                autofit: Default::default(),
            }),
            adjust: Vec::new(),
        },
    );
    course.scenes[0].slides[0].objects.push(badge);

    let mut h = TestHarness::new_with_size(course, (1280.0, 1400.0));
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.object = Some("badge".into());
    h.editor.ui.inspector_tab = InspectorTab::Format;
    h.idle_frame();
    h.idle_frame();

    let size_field = field_next_to_label(&mut h, "Font size");
    h.set_field(size_field, "34");

    let spacing_field = field_next_to_label(&mut h, "Spacing");
    h.click_at(spacing_field);
    h.idle_frame();
    h.press(Key::A, Modifiers::COMMAND);
    h.type_text("2.0");
    let away = pos2(
        h.right_panel_rect().min.x + 10.0,
        h.right_panel_rect().min.y + 10.0,
    );
    h.click_at(away);
    h.idle_frame();

    let panel = h.right_panel_rect();
    h.scroll_over(panel.center(), -2000.0);
    h.pick_combo("1.00x", "1.5x");
    let combo = h.find_text_in("1.50x", panel).expect("line spacing 1.50x");
    h.click_at(combo);
    h.idle_frame();
    let custom = h.find_text_containing("custom").expect("custom field");
    h.drag_by(custom, vec2(40.0, 0.0), 6, Modifiers::NONE);
    h.idle_frame();

    let (obj, _) = h
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object("badge"))
        .expect("badge");
    match &obj.kind {
        ObjectKind::Shape { text: Some(t), .. } => {
            let sz = t.runs.first().and_then(|r| r.size).unwrap_or(t.style.size);
            let ls = t
                .runs
                .first()
                .and_then(|r| r.letter_spacing)
                .unwrap_or(t.style.letter_spacing);
            let ln = t.paragraphs.first().map_or(1.0, |p| p.line_spacing);
            assert!((sz - 34.0).abs() < 0.01, "expected size 34, got {sz}");
            assert!(
                (ls - 2.0).abs() < 0.01,
                "expected letter spacing 2, got {ls}"
            );
            assert!(
                (ln - 1.5).abs() > 0.05,
                "expected custom line spacing != 1.5, got {ln}"
            );
        }
        other => panic!("expected Shape with text, got {other:?}"),
    }
}

#[test]
fn a1_4_multi_select_mixed_sizes_and_shape_text() {
    let mut course = hello();
    let t1 = base_object(
        "t1",
        Rect {
            x: 80.0,
            y: 80.0,
            w: 200.0,
            h: 60.0,
        },
        ObjectKind::Text {
            text: "First".into(),
            style: TextStyle {
                size: 48.0,
                ..Default::default()
            },
            runs: vec![
                serde_json::from_value::<TextRun>(serde_json::json!({
                    "text": "First",
                    "size": 20.0
                }))
                .expect("run"),
            ],
            paragraphs: Vec::new(),
            autofit: Default::default(),
            text_style: None,
            outline: None,
        },
    );
    let t2 = base_object(
        "t2",
        Rect {
            x: 80.0,
            y: 160.0,
            w: 200.0,
            h: 60.0,
        },
        ObjectKind::Text {
            text: "Second".into(),
            style: TextStyle {
                size: 24.0,
                ..Default::default()
            },
            runs: Vec::new(),
            paragraphs: Vec::new(),
            autofit: Default::default(),
            text_style: None,
            outline: None,
        },
    );
    let s1 = base_object(
        "s1",
        Rect {
            x: 80.0,
            y: 240.0,
            w: 200.0,
            h: 80.0,
        },
        ObjectKind::Shape {
            shape: ShapeKind::Rectangle,
            fill: coursecraft_course::Color("#336699".into()),
            fill_style: None,
            outline: None,
            text: Some(ShapeText {
                text: "Shape".into(),
                style: TextStyle {
                    size: 18.0,
                    ..Default::default()
                },
                runs: Vec::new(),
                paragraphs: Vec::new(),
                autofit: Default::default(),
            }),
            adjust: Vec::new(),
        },
    );
    course.scenes[0].slides[0].objects.extend([t1, t2, s1]);

    let mut h = TestHarness::new(course);
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.set_many(["t1", "t2"]);
    h.editor.ui.inspector_tab = InspectorTab::Format;
    h.idle_frame();
    h.idle_frame();

    let panel = h.right_panel_rect();
    assert!(
        h.find_text_in("Mixed", panel).is_some(),
        "expected Mixed hint when selected text sizes differ"
    );
    let field = h
        .find_text_in("Mixed", panel)
        .expect("Mixed text size field");

    // Click, type 36, and click away (blur)
    h.click_at(field);
    h.idle_frame();
    h.press(Key::A, Modifiers::COMMAND);
    h.type_text("36");
    let away = pos2(panel.min.x + 10.0, panel.min.y + 10.0);
    h.click_at(away);
    h.idle_frame();

    let text_size = |h: &TestHarness, id: &str| -> f32 {
        let (obj, _) = h
            .editor
            .session
            .course
            .slide("s-intro")
            .and_then(|s| s.object(id))
            .expect(id);
        match &obj.kind {
            ObjectKind::Text { style, runs, .. } => {
                runs.first().and_then(|r| r.size).unwrap_or(style.size)
            }
            other => panic!("expected Text, got {other:?}"),
        }
    };
    assert!((text_size(&h, "t1") - 36.0).abs() < 0.01);
    assert!((text_size(&h, "t2") - 36.0).abs() < 0.01);

    // Out-of-range negative input clamps to DragValue minimum (1.0); non-numeric input is ignored
    h.set_field(field, "-10");
    assert!((text_size(&h, "t1") - 1.0).abs() < 0.01);
    h.set_field(field, "abc");
    assert!((text_size(&h, "t2") - 1.0).abs() < 0.01);

    // Mixed Text + Shape with text: should not crash or corrupt
    h.editor.selection.set_many(["t1", "s1"]);
    h.idle_frame();
    h.idle_frame();
    assert!(h.editor.session.course.validate().is_empty());
}

#[test]
fn a1_6_line_spacing_custom_enter_blur_and_presets() {
    let mut course = hello();
    let body = base_object(
        "body",
        Rect {
            x: 100.0,
            y: 120.0,
            w: 300.0,
            h: 120.0,
        },
        ObjectKind::Text {
            text: "Para 1\nPara 2".into(),
            style: TextStyle::default(),
            runs: Vec::new(),
            paragraphs: Vec::new(),
            autofit: Default::default(),
            text_style: None,
            outline: None,
        },
    );
    course.scenes[0].slides[0].objects.push(body);

    let mut h = TestHarness::new(course);
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.object = Some("body".into());
    h.editor.ui.inspector_tab = InspectorTab::Format;
    h.idle_frame();
    h.idle_frame();

    let ls_of = |h: &TestHarness| -> f32 {
        let (obj, _) = h
            .editor
            .session
            .course
            .slide("s-intro")
            .and_then(|s| s.object("body"))
            .expect("body");
        match &obj.kind {
            ObjectKind::Text { paragraphs, .. } => {
                paragraphs.first().map_or(1.0, |p| p.line_spacing)
            }
            other => panic!("expected Text, got {other:?}"),
        }
    };

    let panel = h.right_panel_rect();
    h.scroll_over(panel.center(), -2000.0);

    // 1. Preset 1.5x
    h.pick_combo("1.00x", "1.5x");
    assert!((ls_of(&h) - 1.5).abs() < 0.01, "preset 1.5x failed");

    // 2. Custom field via typing + Enter
    let combo = h.find_text_in("1.50x", panel).expect("1.50x combo");
    h.click_at(combo);
    h.idle_frame();
    let custom = h.find_text_containing("custom").expect("custom DragValue");
    h.set_field(custom, "1.85");
    assert!(
        (ls_of(&h) - 1.85).abs() < 0.02,
        "expected line_spacing 1.85 on Enter, got {}",
        ls_of(&h)
    );

    // 3. Custom field via typing + click outside
    let combo2 = h
        .find_text_containing("1.85")
        .or_else(|| h.find_text_in("1.50x", panel))
        .expect("line spacing combo");
    h.click_at(combo2);
    h.idle_frame();
    let custom2 = h.find_text_containing("custom").expect("custom DragValue");
    h.click_at(custom2);
    h.idle_frame();
    h.press(Key::A, Modifiers::COMMAND);
    h.type_text("2.2");
    h.click_at(pos2(panel.min.x + 10.0, panel.min.y + 10.0));
    h.idle_frame();
    assert!(
        (ls_of(&h) - 2.2).abs() < 0.02,
        "expected line_spacing 2.2 on blur, got {}",
        ls_of(&h)
    );
}

// ============================================================================
// P2
// ============================================================================

#[test]
fn a1_5_inline_edit_partial_substring_selection() {
    let mut course = hello();
    let hw = base_object(
        "hw",
        Rect {
            x: 100.0,
            y: 120.0,
            w: 400.0,
            h: 80.0,
        },
        ObjectKind::Text {
            text: "Hello World".into(),
            style: TextStyle {
                size: 24.0,
                ..Default::default()
            },
            runs: Vec::new(),
            paragraphs: Vec::new(),
            autofit: Default::default(),
            text_style: None,
            outline: None,
        },
    );
    course.scenes[0].slides[0].objects.push(hw);

    let mut h = TestHarness::new(course);
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.object = Some("hw".into());
    h.editor.ui.inspector_tab = InspectorTab::Format;
    h.idle_frame();
    h.idle_frame();

    let central = h.central_panel_rect();
    let rect =
        crate::canvas::object_screen_rect(&h.editor.session.course, "s-intro", "hw", central)
            .expect("hw rect");
    h.click_at(rect.center());
    h.click_at(rect.center());
    h.idle_frame();
    assert!(h.editor.ui.inline_edit.is_some());

    // Select last 5 chars ("World") via End + 5x Shift+Left
    h.press(Key::End, Modifiers::NONE);
    for _ in 0..5 {
        h.press(Key::ArrowLeft, Modifiers::SHIFT);
    }

    let size_field = field_next_to_label(&mut h, "Font size");
    h.set_field(size_field, "40");
    let spacing_field = field_next_to_label(&mut h, "Spacing");
    h.set_field(spacing_field, "3.0");
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();

    let (obj, _) = h
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object("hw"))
        .expect("hw");
    match &obj.kind {
        ObjectKind::Text { runs, .. } => {
            assert!(runs.len() >= 2, "expected split runs, got {runs:?}");
            let world = runs
                .iter()
                .find(|r| r.text.contains("World"))
                .expect("World run");
            assert_eq!(world.size, Some(40.0));
            assert_eq!(world.letter_spacing, Some(3.0));
        }
        other => panic!("expected Text, got {other:?}"),
    }
}

#[test]
fn a1_8_text_box_on_slide_layer_and_master_slide() {
    let mut h = TestHarness::new(hello());
    // 1. Layer object "about-text" on layer "about"
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.layer = Some("about".into());
    h.editor.selection.object = Some("about-text".into());
    h.editor.ui.inspector_tab = InspectorTab::Format;
    h.idle_frame();
    h.idle_frame();

    let size_field = field_next_to_label(&mut h, "Font size");
    h.set_field(size_field, "26");
    let spacing_field = field_next_to_label(&mut h, "Spacing");
    h.set_field(spacing_field, "1.5");

    let (obj, _) = h
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object("about-text"))
        .expect("about-text");
    match &obj.kind {
        ObjectKind::Text { style, runs, .. } => {
            let sz = runs.first().and_then(|r| r.size).unwrap_or(style.size);
            let ls = runs
                .first()
                .and_then(|r| r.letter_spacing)
                .unwrap_or(style.letter_spacing);
            assert!((sz - 26.0).abs() < 0.01);
            assert!((ls - 1.5).abs() < 0.01);
        }
        other => panic!("expected Text, got {other:?}"),
    }

    // 2. Master slide text object
    let master_res = h
        .editor
        .session
        .execute("master.add", &serde_json::json!({ "title": "Main" }))
        .expect("master.add");
    let master_id = master_res["id"].as_str().expect("master id").to_owned();
    h.editor
        .session
        .execute(
            "object.add",
            &serde_json::json!({
                "slide": master_id,
                "id": "m-txt",
                "kind": { "type": "text", "text": "Footer", "style": { "size": 16.0 } },
                "frame": { "x": 50.0, "y": 650.0, "w": 200.0, "h": 40.0 }
            }),
        )
        .expect("object.add on master");
    h.editor.ui.master_view = true;
    h.editor.selection.slide = master_id.clone();
    h.editor.selection.layer = None;
    h.editor.selection.object = Some("m-txt".into());
    h.idle_frame();
    h.idle_frame();

    let size_field = field_next_to_label(&mut h, "Font size");
    h.set_field(size_field, "30");
    let spacing_field = field_next_to_label(&mut h, "Spacing");
    h.set_field(spacing_field, "2.0");

    let (mobj, _) = h
        .editor
        .session
        .course
        .slide(&master_id)
        .and_then(|s| s.object("m-txt"))
        .expect("m-txt");
    match &mobj.kind {
        ObjectKind::Text { style, runs, .. } => {
            let sz = runs.first().and_then(|r| r.size).unwrap_or(style.size);
            let ls = runs
                .first()
                .and_then(|r| r.letter_spacing)
                .unwrap_or(style.letter_spacing);
            assert!((sz - 30.0).abs() < 0.01);
            assert!((ls - 2.0).abs() < 0.01);
        }
        other => panic!("expected Text, got {other:?}"),
    }
}

#[test]
fn a1_9_rapid_selection_switch_while_numeric_field_focused_no_bleed() {
    let mut course = hello();
    let t1 = base_object(
        "t1",
        Rect {
            x: 60.0,
            y: 60.0,
            w: 180.0,
            h: 60.0,
        },
        ObjectKind::Text {
            text: "Box A".into(),
            style: TextStyle {
                size: 48.0,
                ..Default::default()
            },
            runs: Vec::new(),
            paragraphs: Vec::new(),
            autofit: Default::default(),
            text_style: None,
            outline: None,
        },
    );
    let t2 = base_object(
        "t2",
        Rect {
            x: 60.0,
            y: 420.0,
            w: 180.0,
            h: 60.0,
        },
        ObjectKind::Text {
            text: "Box B".into(),
            style: TextStyle {
                size: 18.0,
                ..Default::default()
            },
            runs: Vec::new(),
            paragraphs: Vec::new(),
            autofit: Default::default(),
            text_style: None,
            outline: None,
        },
    );
    course.scenes[0].slides[0].objects.extend([t1, t2]);

    let mut h = TestHarness::new(course);
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.object = Some("t1".into());
    h.editor.ui.inspector_tab = InspectorTab::Format;
    h.idle_frame();
    h.idle_frame();

    let field = field_next_to_label(&mut h, "Font size");
    h.click_at(field);
    h.idle_frame();
    h.press(Key::A, Modifiers::COMMAND);
    h.type_text("72");

    // Immediately click t2 on canvas without pressing Enter
    let central = h.central_panel_rect();
    let t2_rect =
        crate::canvas::object_screen_rect(&h.editor.session.course, "s-intro", "t2", central)
            .expect("t2 rect");
    h.click_at(t2_rect.center());
    h.idle_frame();
    h.idle_frame();

    assert_eq!(h.editor.selection.object.as_deref(), Some("t2"));
    let (t2_obj, _) = h
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object("t2"))
        .expect("t2");
    match &t2_obj.kind {
        ObjectKind::Text { style, runs, .. } => {
            let sz = runs.first().and_then(|r| r.size).unwrap_or(style.size);
            assert!(
                (sz - 18.0).abs() < 0.01,
                "t2 size bled from t1! expected 18.0, got {sz}"
            );
        }
        other => panic!("expected Text, got {other:?}"),
    }
}

// ============================================================================
// P3
// ============================================================================

#[test]
fn a1_7_extreme_boundary_and_hostile_numeric_inputs() {
    let mut h = TestHarness::new(hello());
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.object = Some("title".into());
    h.editor.ui.inspector_tab = InspectorTab::Format;
    h.idle_frame();
    h.idle_frame();

    let size_field = field_next_to_label(&mut h, "Font size");
    for val in ["1", "200", "0", "-5", "9999", "xyz"] {
        h.set_field(size_field, val);
    }

    let spacing_field = field_next_to_label(&mut h, "Spacing");
    for val in ["-10", "50", "-999", "999"] {
        h.set_field(spacing_field, val);
    }

    let issues = h.editor.session.course.validate();
    assert!(
        issues.is_empty(),
        "unexpected validation issues after hostile numeric inputs: {issues:?}"
    );
}
