//! Reproduction and regression tests for Aurelie A1: text size and spacing fields.

use crate::properties::InspectorTab;
use crate::test_harness::TestHarness;
use coursecraft_course::{Course, ObjectKind};
use egui::{Modifiers, Pos2, pos2, vec2};

fn hello() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    Course::from_json(&json).expect("valid course")
}

fn select_title(h: &mut TestHarness) {
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.object = Some("title".into());
    h.editor.ui.inspector_tab = InspectorTab::Format;
    h.idle_frame();
    h.idle_frame();
}

fn title_size_and_spacing(h: &TestHarness) -> (f32, f32, f32) {
    let slide = h.editor.session.course.slide("s-intro").expect("slide");
    let (obj, _) = slide.object("title").expect("title");
    match &obj.kind {
        ObjectKind::Text {
            style,
            runs,
            paragraphs,
            ..
        } => {
            let size = runs.first().and_then(|r| r.size).unwrap_or(style.size);
            let letter_spacing = runs
                .first()
                .and_then(|r| r.letter_spacing)
                .unwrap_or(style.letter_spacing);
            let line_spacing = paragraphs.first().map_or(1.0, |p| p.line_spacing);
            (size, letter_spacing, line_spacing)
        }
        other => panic!("expected text, got {other:?}"),
    }
}

fn field_next_to_label(h: &mut TestHarness, label: &str) -> Pos2 {
    let panel = h.right_panel_rect();
    let label_pos = h
        .find_text_in(label, panel)
        .unwrap_or_else(|| panic!("label '{label}' not found in right panel"));
    // LABEL_COL_WIDTH is 100.0, control starts to the right of the label column.
    pos2(panel.min.x + 150.0, label_pos.y)
}

#[test]
fn font_size_type_and_enter_updates_course() {
    let mut h = TestHarness::new(hello());
    select_title(&mut h);
    let field = field_next_to_label(&mut h, "Font size");
    h.set_field(field, "42");
    let (size, _, _) = title_size_and_spacing(&h);
    assert!(
        (size - 42.0).abs() < 0.01,
        "expected font size 42, got {size}"
    );
}

#[test]
fn font_size_type_and_click_away_updates_course() {
    let mut h = TestHarness::new(hello());
    select_title(&mut h);
    let field = field_next_to_label(&mut h, "Font size");
    h.click_at(field);
    h.idle_frame();
    h.press(egui::Key::A, Modifiers::COMMAND);
    h.type_text("44");
    let away = pos2(
        h.right_panel_rect().min.x + 10.0,
        h.right_panel_rect().min.y + 10.0,
    );
    h.click_at(away);
    h.idle_frame();
    let (size, _, _) = title_size_and_spacing(&h);
    assert!(
        (size - 44.0).abs() < 0.01,
        "expected font size 44, got {size}"
    );
}

#[test]
fn font_size_drag_updates_course() {
    let mut h = TestHarness::new(hello());
    select_title(&mut h);
    let (before, _, _) = title_size_and_spacing(&h);
    let field = field_next_to_label(&mut h, "Font size");
    h.drag_by(field, vec2(40.0, 0.0), 6, Modifiers::NONE);
    h.idle_frame();
    let (after, _, _) = title_size_and_spacing(&h);
    assert!(
        (after - before).abs() > 1.0,
        "expected font size to change from {before} after drag, got {after}"
    );
}

#[test]
fn letter_spacing_type_and_enter_updates_course() {
    let mut h = TestHarness::new(hello());
    select_title(&mut h);
    let field = field_next_to_label(&mut h, "Spacing");
    h.set_field(field, "3.5");
    let (_, spacing, _) = title_size_and_spacing(&h);
    assert!(
        (spacing - 3.5).abs() < 0.01,
        "expected letter spacing 3.5, got {spacing}"
    );
}

#[test]
fn letter_spacing_drag_updates_course() {
    let mut h = TestHarness::new(hello());
    select_title(&mut h);
    let (_, before, _) = title_size_and_spacing(&h);
    let field = field_next_to_label(&mut h, "Spacing");
    h.drag_by(field, vec2(40.0, 0.0), 6, Modifiers::NONE);
    h.idle_frame();
    let (_, after, _) = title_size_and_spacing(&h);
    assert!(
        (after - before).abs() > 0.1,
        "expected letter spacing to change from {before} after drag, got {after}"
    );
}

#[test]
fn font_size_with_inline_edit_open_updates_course() {
    let mut h = TestHarness::new(hello());
    select_title(&mut h);
    // Double-click title on canvas to open inline edit.
    let central = h.central_panel_rect();
    let title_rect =
        crate::canvas::object_screen_rect(&h.editor.session.course, "s-intro", "title", central)
            .expect("title rect");
    h.click_at(title_rect.center());
    h.click_at(title_rect.center());
    h.idle_frame();
    assert!(
        h.editor.ui.inline_edit.is_some(),
        "expected inline edit open"
    );

    let field = field_next_to_label(&mut h, "Font size");
    h.set_field(field, "32");
    let (size, _, _) = title_size_and_spacing(&h);
    assert!(
        (size - 32.0).abs() < 0.01,
        "expected font size 32 with inline edit open, got {size}"
    );
}

#[test]
fn multi_select_text_size_type_and_enter_updates_both() {
    let mut course = hello();
    let mut subtitle = course.scenes[0].slides[0].objects[0].clone();
    subtitle.id = "subtitle".into();
    if let ObjectKind::Text { text, style, .. } = &mut subtitle.kind {
        *text = "Sub".into();
        style.size = 24.0;
    }
    course.scenes[0].slides[0].objects.push(subtitle);
    let mut h = TestHarness::new(course);
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.set_many(["title", "subtitle"]);
    h.editor.ui.inspector_tab = InspectorTab::Format;
    h.idle_frame();
    h.idle_frame();

    let panel = h.right_panel_rect();
    let label_pos = h
        .find_text_last("Size")
        .expect("Text Size label in right panel");
    let field = pos2(panel.min.x + 150.0, label_pos.y);
    h.set_field(field, "30");

    let slide = h.editor.session.course.slide("s-intro").expect("slide");
    for id in ["title", "subtitle"] {
        let (obj, _) = slide.object(id).expect(id);
        match &obj.kind {
            ObjectKind::Text { style, runs, .. } => {
                let size = runs.first().and_then(|r| r.size).unwrap_or(style.size);
                assert!(
                    (size - 30.0).abs() < 0.01,
                    "expected {id} size 30, got {size}"
                );
            }
            other => panic!("expected text for {id}, got {other:?}"),
        }
    }
}

#[test]
fn line_spacing_custom_drag_updates_course() {
    let mut h = TestHarness::new(hello());
    select_title(&mut h);
    let panel = h.right_panel_rect();
    h.scroll_over(panel.center(), -2000.0);
    let combo = h.find_text_in("1.00x", panel).expect("line spacing combo");
    h.click_at(combo);
    h.idle_frame();
    let custom = h
        .find_text_containing("custom")
        .expect("custom line spacing drag value");
    h.drag_by(custom, vec2(40.0, 0.0), 6, Modifiers::NONE);
    h.idle_frame();
    let (_, _, line_spacing) = title_size_and_spacing(&h);
    assert!(
        (line_spacing - 1.0).abs() > 0.05,
        "expected line spacing to change from 1.0 after drag, got {line_spacing}"
    );
}

#[test]
fn letter_spacing_type_and_click_away_updates_course() {
    let mut h = TestHarness::new(hello());
    select_title(&mut h);
    let field = field_next_to_label(&mut h, "Spacing");
    h.click_at(field);
    h.idle_frame();
    h.press(egui::Key::A, Modifiers::COMMAND);
    h.type_text("4.5");
    let away = pos2(
        h.right_panel_rect().min.x + 10.0,
        h.right_panel_rect().min.y + 10.0,
    );
    h.click_at(away);
    h.idle_frame();
    let (_, spacing, _) = title_size_and_spacing(&h);
    assert!(
        (spacing - 4.5).abs() < 0.01,
        "expected letter spacing 4.5, got {spacing}"
    );
}

#[test]
fn shape_text_font_size_and_spacing_drag_updates_course() {
    let mut course = hello();
    let mut badge = course.scenes[0].slides[0].objects[0].clone();
    badge.id = "badge".into();
    badge.kind = ObjectKind::Shape {
        shape: coursecraft_course::ShapeKind::Rectangle,
        fill: coursecraft_course::Color("#2F6FEB".into()),
        fill_style: None,
        outline: None,
        text: Some(coursecraft_course::ShapeText {
            text: "Note".into(),
            style: coursecraft_course::TextStyle {
                size: 20.0,
                ..Default::default()
            },
            runs: Vec::new(),
            paragraphs: Vec::new(),
            autofit: Default::default(),
        }),
        adjust: Vec::new(),
    };
    course.scenes[0].slides[0].objects.push(badge);
    let mut h = TestHarness::new_with_size(course, (1280.0, 1400.0));
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.object = Some("badge".into());
    h.editor.ui.inspector_tab = InspectorTab::Format;
    h.idle_frame();
    h.idle_frame();

    let size_field = field_next_to_label(&mut h, "Font size");
    h.drag_by(size_field, vec2(30.0, 0.0), 5, Modifiers::NONE);
    h.idle_frame();

    let spacing_field = field_next_to_label(&mut h, "Spacing");
    h.drag_by(spacing_field, vec2(30.0, 0.0), 5, Modifiers::NONE);
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
            let size = t.runs.first().and_then(|r| r.size).unwrap_or(t.style.size);
            let spacing = t
                .runs
                .first()
                .and_then(|r| r.letter_spacing)
                .unwrap_or(t.style.letter_spacing);
            assert!(
                (size - 20.0).abs() > 1.0,
                "expected shape text size > 20, got {size}"
            );
            assert!(
                spacing.abs() > 0.1,
                "expected shape letter spacing != 0, got {spacing}"
            );
        }
        other => panic!("expected shape with text, got {other:?}"),
    }
}

#[test]
fn font_size_undo_redo_restores_value() {
    let mut h = TestHarness::new(hello());
    select_title(&mut h);
    let field = field_next_to_label(&mut h, "Font size");
    h.set_field(field, "60");
    assert!((title_size_and_spacing(&h).0 - 60.0).abs() < 0.01);

    assert!(h.editor.session.undo().is_ok());
    h.idle_frame();
    assert!((title_size_and_spacing(&h).0 - 48.0).abs() < 0.01);

    assert!(h.editor.session.redo().is_ok());
    h.idle_frame();
    assert!((title_size_and_spacing(&h).0 - 60.0).abs() < 0.01);
}

/// SC-03: Inspector Bold/Italic/Underline tooltips use platform-aware shortcut
/// formatting (`⌘B`/`⌘I`/`⌘U` or `Cmd+B` on macOS; `Ctrl+B`/`Ctrl+I`/`Ctrl+U` on
/// Windows) from the shared shortcut table (`M32-04`).
#[test]
fn inspector_bold_italic_underline_tooltips_follow_platform_shortcuts() {
    let mut h = TestHarness::new(hello());
    select_title(&mut h);
    let panel = h.right_panel_rect();

    // macOS formatting
    h.ctx.set_os(egui::os::OperatingSystem::Mac);
    h.idle_frame();
    h.idle_frame();
    for (btn, sym, fallback) in [
        ("B", "⌘B", "Cmd+B"),
        ("I", "⌘I", "Cmd+I"),
        ("U", "⌘U", "Cmd+U"),
    ] {
        let pos = h
            .find_text_in(btn, panel)
            .unwrap_or_else(|| panic!("missing {btn} button in Format inspector"));
        let tip = h
            .probe_tooltip(pos)
            .unwrap_or_else(|| panic!("missing macOS tooltip for {btn}"));
        assert!(
            tip.contains(sym) || tip.contains(fallback),
            "macOS {btn} tooltip {tip:?} must contain {sym:?} or {fallback:?}"
        );
    }

    // Windows formatting
    h.ctx.set_os(egui::os::OperatingSystem::Windows);
    h.idle_frame();
    h.idle_frame();
    for (btn, expected) in [("B", "Ctrl+B"), ("I", "Ctrl+I"), ("U", "Ctrl+U")] {
        let pos = h
            .find_text_in(btn, panel)
            .unwrap_or_else(|| panic!("missing {btn} button in Format inspector"));
        let tip = h
            .probe_tooltip(pos)
            .unwrap_or_else(|| panic!("missing Windows tooltip for {btn}"));
        assert!(
            tip.contains(expected),
            "Windows {btn} tooltip {tip:?} must contain {expected:?}"
        );
    }
}
