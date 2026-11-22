//! Regression tests for rich-text style indicator buttons (B/I/U/S/x²/x₂),
//! caret pending style, and clean run splitting/merging on toggle.

use crate::canvas::InlineEdit;
use crate::properties::{style_at_cursor, style_for_range};
use crate::test_harness::TestHarness;
use coursecraft_course::{Baseline, Course, ObjectKind, TextRun, TextStyle};

const BASE_COURSE: &str = r##"{
  "schema_version": 1,
  "id": "c-style",
  "title": "Style Toggles",
  "slide_size": { "w": 1280, "h": 720 },
  "scenes": [{
    "id": "scene1",
    "title": "Main",
    "slides": [{
      "id": "s1",
      "title": "Slide 1",
      "objects": [{
        "id": "t1",
        "kind": {
          "type": "text",
          "text": "bold+ italic + underline\nsecond line",
          "style": {
            "font": "Inter",
            "size": 24,
            "color": "#1F1F1F",
            "bold": false,
            "italic": false,
            "underline": false,
            "align": "left"
          }
        },
        "frame": { "x": 80, "y": 80, "w": 600, "h": 160 }
      }]
    }]
  }]
}"##;

fn setup_mixed_runs(harness: &mut TestHarness) -> Vec<TextRun> {
    // "bold+ " (0..6): bold + italic + underline
    // "italic" (6..12): bold only
    // " + " (12..15): italic only
    // "underline" (15..24): italic + underline
    // "\nsecond line" (24..36): plain
    harness
        .editor
        .run(
            "text.style_range",
            serde_json::json!({ "slide": "s1", "object": "t1", "start": 0, "end": 6, "bold": true, "italic": true, "underline": true }),
        )
        .expect("style bold+");
    harness
        .editor
        .run(
            "text.style_range",
            serde_json::json!({ "slide": "s1", "object": "t1", "start": 6, "end": 12, "bold": true }),
        )
        .expect("style italic");
    harness
        .editor
        .run(
            "text.style_range",
            serde_json::json!({ "slide": "s1", "object": "t1", "start": 12, "end": 15, "italic": true }),
        )
        .expect("style +");
    harness
        .editor
        .run(
            "text.style_range",
            serde_json::json!({ "slide": "s1", "object": "t1", "start": 15, "end": 24, "italic": true, "underline": true }),
        )
        .expect("style underline");
    text_runs(harness)
}

fn text_runs(harness: &TestHarness) -> Vec<TextRun> {
    let slide = harness.editor.session.course.slide("s1").expect("slide");
    let (obj, _) = slide.object("t1").expect("t1");
    match &obj.kind {
        ObjectKind::Text { runs, .. } => runs.clone(),
        other => panic!("expected text, got {other:?}"),
    }
}

fn is_button_active(harness: &mut TestHarness, label: &str) -> bool {
    let panel = harness.right_panel_rect();
    let pos = harness
        .find_text_in(label, panel)
        .unwrap_or_else(|| panic!("button {label:?} not found in inspector"));
    let active_bg = coursecraft_render::palette(&harness.ctx).accent_subtle;
    harness.frame_shapes(vec![]).iter().any(|clipped| {
        if let egui::epaint::Shape::Rect(rect_shape) = &clipped.shape {
            rect_shape.rect.contains(pos) && rect_shape.fill == active_bg
        } else {
            false
        }
    })
}

fn enter_inline_with_selection(harness: &mut TestHarness, start: usize, end: usize) {
    let slide = harness.editor.session.course.slide("s1").expect("slide");
    let (obj, _) = slide.object("t1").expect("t1");
    let (plain, runs, paragraphs) = match &obj.kind {
        ObjectKind::Text {
            text,
            runs,
            paragraphs,
            ..
        } => (text.clone(), runs.clone(), paragraphs.clone()),
        other => panic!("expected text, got {other:?}"),
    };
    harness.editor.selection.slide = "s1".into();
    harness.editor.selection.object = Some("t1".into());
    harness.editor.ui.inline_edit = Some(InlineEdit {
        slide: "s1".into(),
        object: "t1".into(),
        buffer: plain.clone(),
        original: plain,
        is_button: false,
        is_shape: false,
        cell: None,
        just_entered: false,
        live_runs: runs,
        live_paragraphs: paragraphs,
        last_revision: harness.editor.session.revision,
        state_name: None,
        extra_id: None,
    });
    harness.idle_frame();
    let edit_id = egui::Id::unique(("canvas_inline_edit", "s1", "t1"));
    let mut state = egui::TextEdit::load_state(&harness.ctx, edit_id).unwrap_or_default();
    state
        .cursor
        .set_char_range(Some(egui::text_selection::CCursorRange::two(
            egui::text::CCursor::new(start),
            egui::text::CCursor::new(end),
        )));
    state.store(&harness.ctx, edit_id);
    harness.editor.ui.inline_edit_selection = Some((
        "s1".into(),
        "t1".into(),
        (start != end).then_some((start.min(end), start.max(end))),
        start,
    ));
    harness.idle_frame();
}

#[test]
fn style_query_ignores_neighbours_and_handles_caret_boundaries() {
    let course = Course::from_json(BASE_COURSE).expect("valid course");
    let mut harness = TestHarness::new(course);
    let runs = setup_mixed_runs(&mut harness);
    let base = TextStyle::default();

    // Selection of "italic" (6..12): neighbours (0..6 and 12..15) are italic+underline,
    // but "italic" itself is ONLY bold.
    assert_eq!(
        style_for_range(&runs, &base, (6, 12)),
        (true, false, false, base.size, base.color.0.clone())
    );
    // Selection inside "italic" (7..11) and at its left/right edges (6..8, 10..12).
    assert!(!style_for_range(&runs, &base, (7, 11)).1);
    assert!(!style_for_range(&runs, &base, (6, 8)).1);
    assert!(!style_for_range(&runs, &base, (10, 12)).1);

    // Selection spanning italic and non-italic text (4..8): bold is true everywhere,
    // italic & underline are false because not all selected chars have them.
    let span = style_for_range(&runs, &base, (4, 8));
    assert!(span.0, "every char in 4..8 is bold");
    assert!(!span.1, "not every char in 4..8 is italic");
    assert!(!span.2, "not every char in 4..8 is underlined");

    // Caret boundaries: typing uses the char before the caret (or first run at 0).
    assert!(style_at_cursor(&runs, &base, 0).1); // first run "bold+ "
    assert!(style_at_cursor(&runs, &base, 6).1); // char 5 is ' ' in "bold+ "
    assert!(!style_at_cursor(&runs, &base, 7).1); // char 6 is 'i' in "italic"
    assert!(!style_at_cursor(&runs, &base, 12).1); // char 11 is 'c' in "italic"
    assert!(style_at_cursor(&runs, &base, 13).1); // char 12 is ' ' in " + "
}

#[test]
fn inspector_style_buttons_reflect_selection_and_toggle_cleanly_with_undo() {
    let course = Course::from_json(BASE_COURSE).expect("valid course");
    let mut harness = TestHarness::new(course);
    let original_runs = setup_mixed_runs(&mut harness);

    // Select "italic" (6..12) in inline edit: B must be active, I and U must be off!
    enter_inline_with_selection(&mut harness, 6, 12);
    assert!(is_button_active(&mut harness, "B"));
    assert!(!is_button_active(&mut harness, "I"));
    assert!(!is_button_active(&mut harness, "U"));

    // Toggle U ON, then OFF, then ON, then Undo: must restore exact runs.
    let panel = harness.right_panel_rect();
    let u_pos = harness.find_text_in("U", panel).expect("U button");
    harness.click_at(u_pos);
    harness.idle_frame();
    assert!(is_button_active(&mut harness, "U"));

    harness.click_at(u_pos);
    harness.idle_frame();
    assert!(!is_button_active(&mut harness, "U"));
    assert_eq!(
        text_runs(&harness),
        original_runs,
        "toggling U on and off must merge runs back to original"
    );

    // Toggle I ON and undo: undo restores original runs.
    let i_pos = harness.find_text_in("I", panel).expect("I button");
    harness.click_at(i_pos);
    harness.idle_frame();
    assert!(is_button_active(&mut harness, "I"));
    harness.editor.run("edit.undo", serde_json::json!({}));
    harness.idle_frame();
    assert_eq!(text_runs(&harness), original_runs);
}

#[test]
fn selection_across_two_paragraphs_toggles_and_restores() {
    let course = Course::from_json(BASE_COURSE).expect("valid course");
    let mut harness = TestHarness::new(course);
    let original_runs = setup_mixed_runs(&mut harness);

    // Select "derline\nsecond" (17..31), spanning paragraph 0 (underline=true) and paragraph 1 (underline=false).
    enter_inline_with_selection(&mut harness, 17, 31);
    assert!(
        !is_button_active(&mut harness, "U"),
        "mixed selection across paragraphs must not show U active"
    );

    let panel = harness.right_panel_rect();
    let u_pos = harness.find_text_in("U", panel).expect("U button");
    // First click applies underline=true to all of 17..31; second click turns it off for 17..31.
    harness.click_at(u_pos);
    harness.idle_frame();
    assert!(is_button_active(&mut harness, "U"));

    // Also test Superscript & Strikethrough across two paragraphs.
    let s_pos = harness.find_text_in("S", panel).expect("S button");
    harness.click_at(s_pos);
    harness.idle_frame();
    assert!(is_button_active(&mut harness, "S"));
    harness.click_at(s_pos);
    harness.idle_frame();
    assert!(!is_button_active(&mut harness, "S"));

    let sup_pos = harness
        .find_text_in("x\u{b2}", panel)
        .expect("Superscript button");
    harness.click_at(sup_pos);
    harness.idle_frame();
    assert!(is_button_active(&mut harness, "x\u{b2}"));
    harness.click_at(sup_pos);
    harness.idle_frame();
    assert!(!is_button_active(&mut harness, "x\u{b2}"));

    // Undo all 5 style toggles (x² off, x² on, S off, S on, U on) -> restores exact original runs.
    for _ in 0..5 {
        harness.editor.run("edit.undo", serde_json::json!({}));
    }
    harness.idle_frame();
    assert_eq!(text_runs(&harness), original_runs);
}

#[test]
fn caret_pending_style_applies_to_next_typed_characters() {
    let course = Course::from_json(BASE_COURSE).expect("valid course");
    let mut harness = TestHarness::new(course);
    setup_mixed_runs(&mut harness);

    // Place caret at index 9 inside "italic" (no selection).
    enter_inline_with_selection(&mut harness, 9, 9);
    assert!(is_button_active(&mut harness, "B"));
    assert!(!is_button_active(&mut harness, "I"));
    assert!(!is_button_active(&mut harness, "U"));

    // Click I and U in inspector -> buttons flip via pending_caret_style, without modifying existing text runs.
    let runs_before = text_runs(&harness);
    let panel = harness.right_panel_rect();
    let i_pos = harness.find_text_in("I", panel).expect("I button");
    let u_pos = harness.find_text_in("U", panel).expect("U button");
    harness.click_at(i_pos);
    harness.idle_frame();
    harness.click_at(u_pos);
    harness.idle_frame();
    assert!(is_button_active(&mut harness, "I"));
    assert!(is_button_active(&mut harness, "U"));
    assert_eq!(text_runs(&harness), runs_before);

    // Type "XY" at caret 9 and commit inline edit.
    let edit_id = egui::Id::unique(("canvas_inline_edit", "s1", "t1"));
    harness.ctx.memory_mut(|m| m.request_focus(edit_id));
    harness.frame(vec![egui::Event::Text("XY".to_owned())]);
    crate::canvas::commit_inline_edit(&mut harness.editor);
    harness.idle_frame();

    let runs_after = text_runs(&harness);
    let xy_run = runs_after
        .iter()
        .find(|r| r.text.contains("XY"))
        .expect("XY run");
    assert_eq!(xy_run.italic, Some(true));
    assert_eq!(xy_run.underline, Some(true));
    assert_eq!(xy_run.bold, Some(true));
    // Neighbouring "ita" and "lic" must remain non-italic, non-underlined.
    let ita_run = runs_after
        .iter()
        .find(|r| r.text == "ita")
        .expect("ita run");
    assert_eq!(ita_run.italic, None);
    assert_eq!(ita_run.underline, None);
    // Baseline check: not superscript.
    assert_ne!(xy_run.baseline, Some(Baseline::Superscript));
}

fn widget_toggled(
    harness: &TestHarness,
    accessible_label: &str,
) -> Option<egui::accesskit::Toggled> {
    harness
        .last_widgets
        .iter()
        .find(|w| w.label.as_deref() == Some(accessible_label))
        .and_then(|w| w.toggled)
}

#[test]
fn multi_run_mixed_selection_shows_mixed_state_and_applies_on_click() {
    let course = Course::from_json(BASE_COURSE).expect("valid course");
    let mut harness = TestHarness::new(course);
    // Run 0 "bold+ " (0..6): bold=true, italic=true
    // Run 1 "italic" (6..12): bold=true, italic=false
    // Run 2 " + " (12..15): bold=false, italic=true
    setup_mixed_runs(&mut harness);

    // Select 6..15 ("italic + "), which has bold mixed (true in 6..12, false in 12..15)
    // and italic mixed (false in 6..12, true in 12..15).
    enter_inline_with_selection(&mut harness, 6, 15);
    assert_eq!(
        widget_toggled(&harness, "Bold (Ctrl+B)"),
        Some(egui::accesskit::Toggled::Mixed),
        "B must report Toggled::Mixed when runs disagree"
    );
    assert_eq!(
        widget_toggled(&harness, "Italic (Ctrl+I)"),
        Some(egui::accesskit::Toggled::Mixed),
        "I must report Toggled::Mixed when runs disagree"
    );

    // Clicking B sets the whole range bold in one undoable command.
    let panel = harness.right_panel_rect();
    let b_pos = harness.find_text_in("B", panel).expect("B button");
    harness.click_at(b_pos);
    harness.idle_frame();
    assert_eq!(
        widget_toggled(&harness, "Bold (Ctrl+B)"),
        Some(egui::accesskit::Toggled::True),
        "clicking Mixed B must set all selected runs bold"
    );
}

#[test]
fn multi_object_mixed_selection_shows_mixed_toggles_font_picker_and_drag_size() {
    let json = r##"{
      "schema_version": 1,
      "id": "c-multi",
      "title": "Multi Text",
      "slide_size": { "w": 1280, "h": 720 },
      "scenes": [{
        "id": "scene1",
        "title": "Main",
        "slides": [{
          "id": "s1",
          "title": "Slide 1",
          "objects": [
            {
              "id": "t1",
              "kind": {
                "type": "text",
                "text": "One",
                "style": { "font": "Inter", "size": 18, "color": "#1F1F1F", "italic": true }
              },
              "frame": { "x": 80, "y": 80, "w": 200, "h": 60 }
            },
            {
              "id": "t2",
              "kind": {
                "type": "text",
                "text": "Two",
                "style": { "font": "Lora", "size": 24, "color": "#1F1F1F", "italic": false }
              },
              "frame": { "x": 80, "y": 180, "w": 200, "h": 60 }
            }
          ]
        }]
      }]
    }"##;
    let course = Course::from_json(json).expect("valid course");
    let mut harness = TestHarness::new(course);
    harness.editor.selection.slide = "s1".into();
    harness.editor.selection.set_many(["t1", "t2"]);
    harness.idle_frame();

    // Mixed italic must report Toggled::Mixed (not False).
    assert_eq!(
        widget_toggled(&harness, "Italic (Ctrl+I)"),
        Some(egui::accesskit::Toggled::Mixed),
        "multi-select Italic must report Toggled::Mixed"
    );

    // Size must be a DragValue showing "Mixed" (role SpinButton) that supports dragging/typing.
    let panel = harness.right_panel_rect();
    let mixed_positions = harness.find_texts("Mixed");
    assert!(
        mixed_positions
            .iter()
            .filter(|p| panel.contains(**p))
            .count()
            >= 2,
        "multi-select Font and Size must both show Mixed"
    );
    assert!(
        harness
            .last_widgets
            .iter()
            .any(|w| panel.contains(w.bounds.center())
                && w.role == egui::accesskit::Role::SpinButton),
        "multi-select Size must be a DragValue (SpinButton)"
    );

    // Font control must show "Mixed" and open the unified FontPicker popup.
    let font_pos = harness
        .find_text_in("Mixed", panel)
        .expect("Mixed font button in multi-select");
    harness.click_at(font_pos);
    harness.idle_frame();
    assert!(
        harness.find_text("In this course").is_some() || harness.find_text("Add font").is_some(),
        "multi-select Font must open unified FontPicker popup"
    );
}

#[test]
fn style_buttons_have_tooltips_and_accessible_names_everywhere() {
    let json = r##"{
      "schema_version": 1,
      "id": "c-tips",
      "title": "Tips",
      "slide_size": { "w": 1280, "h": 720 },
      "scenes": [{
        "id": "scene1",
        "title": "Main",
        "slides": [{
          "id": "s1",
          "title": "Slide 1",
          "objects": [
            {
              "id": "t1",
              "kind": {
                "type": "text",
                "text": "Hello",
                "style": { "font": "Inter", "size": 24, "color": "#1F1F1F" }
              },
              "frame": { "x": 80, "y": 80, "w": 200, "h": 60 }
            },
            {
              "id": "timer1",
              "kind": {
                "type": "timer",
                "duration_ms": 10000,
                "format": "mm_ss",
                "autostart": true,
                "style": { "font": "Inter", "size": 24, "color": "#1F1F1F" }
              },
              "frame": { "x": 80, "y": 180, "w": 200, "h": 60 }
            }
          ]
        }]
      }]
    }"##;
    let course = Course::from_json(json).expect("valid course");
    let mut harness = TestHarness::new(course);

    // 1. Single-select text: all 6 style buttons have tooltips and accesskit labels.
    harness.editor.selection.slide = "s1".into();
    harness.editor.selection.object = Some("t1".into());
    harness.idle_frame();
    let panel = harness.right_panel_rect();
    for (btn_text, expected_tip) in [
        ("B", "Bold (Ctrl+B)"),
        ("I", "Italic (Ctrl+I)"),
        ("U", "Underline (Ctrl+U)"),
        ("S", "Strikethrough"),
        ("x\u{b2}", "Superscript"),
        ("x\u{2082}", "Subscript"),
    ] {
        let pos = harness
            .find_text_in(btn_text, panel)
            .unwrap_or_else(|| panic!("missing button {btn_text}"));
        let tip = harness.probe_tooltip(pos).unwrap_or_default();
        assert!(
            tip.contains(expected_tip),
            "single-select {btn_text} tooltip {tip:?} must contain {expected_tip:?}"
        );
        assert!(
            widget_toggled(&harness, expected_tip).is_some(),
            "single-select {btn_text} must have accesskit label {expected_tip:?}"
        );
    }

    // 2. Timer: B/I/U have tooltips & accesskit labels, and Font opens FontPicker.
    harness.editor.selection.object = Some("timer1".into());
    harness.idle_frame();
    for (btn_text, expected_tip) in [
        ("B", "Bold (Ctrl+B)"),
        ("I", "Italic (Ctrl+I)"),
        ("U", "Underline (Ctrl+U)"),
    ] {
        let pos = harness
            .find_text_in(btn_text, panel)
            .unwrap_or_else(|| panic!("missing timer button {btn_text}"));
        let tip = harness.probe_tooltip(pos).unwrap_or_default();
        assert!(
            tip.contains(expected_tip),
            "timer {btn_text} tooltip {tip:?} must contain {expected_tip:?}"
        );
        assert!(
            widget_toggled(&harness, expected_tip).is_some(),
            "timer {btn_text} must have accesskit label {expected_tip:?}"
        );
    }
    let timer_font_pos = harness
        .find_text_in("Inter", panel)
        .expect("timer font picker button");
    harness.click_at(timer_font_pos);
    harness.idle_frame();
    assert!(
        harness.find_text("In this course").is_some() || harness.find_text("Add font").is_some(),
        "timer Font must open unified FontPicker popup"
    );
}
