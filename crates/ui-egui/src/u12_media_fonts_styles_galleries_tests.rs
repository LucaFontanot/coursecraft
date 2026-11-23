//! Step u12 regression tests: Media Library, Fonts Manager, Text Styles & Shape/Interaction/Web Galleries (`Area 9`).

use crate::test_harness::{TestHarness, capture_layers, check_contrast, check_nested_scrolls};
use coursecraft_course::{Course, NamedTextStyle, TextStyle};
use coursecraft_render::ThemeMode;
use egui::{Id, Key, Modifiers, Rect, Vec2};

fn base_course() -> Course {
    let raw = serde_json::json!({
        "schema_version": 1,
        "id": "c-u12",
        "title": "Step u12 Course",
        "slide_size": { "w": 1280, "h": 720 },
        "masters": [{ "id": "m1", "title": "Default Master", "objects": [] }],
        "scenes": [{
            "id": "scene1",
            "title": "Main",
            "slides": [{
                "id": "s1",
                "title": "Slide 1",
                "duration_ms": 60000,
                "objects": [],
                "layers": [],
                "triggers": []
            }]
        }]
    });
    serde_json::from_value(raw).expect("valid course json")
}

fn minimal_png_bytes() -> std::sync::Arc<[u8]> {
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ]
    .into()
}

fn dialog_rect_by_id(h: &TestHarness, id: Id) -> Option<Rect> {
    h.ctx.memory(|m| m.area_rect(id))
}

fn dialog_rect(h: &TestHarness, title: &str) -> Option<Rect> {
    dialog_rect_by_id(h, Id::unique(("coursecraft_dialog", title)))
}

#[test]
fn t9_1_media_library_has_zero_nested_scrolls_and_grows_table_when_resized() {
    let mut course = base_course();
    let mut media = Vec::new();
    let mut objects = Vec::new();
    for i in 0..30 {
        let name = format!("media/image_{i:02}.png");
        media.push((name.clone(), minimal_png_bytes()));
        if i == 0 {
            for j in 0..10 {
                objects.push(serde_json::json!({
                    "id": format!("img_{j}"),
                    "name": format!("Used Image {j}"),
                    "frame": { "x": 10.0 * j as f32, "y": 10.0, "w": 60.0, "h": 60.0 },
                    "kind": { "type": "image", "src": name }
                }));
            }
        }
    }
    course.scenes[0].slides[0].objects =
        serde_json::from_value(serde_json::Value::Array(objects)).expect("objects");

    let mut h = TestHarness::new_with_size(course, (1280.0, 900.0));
    h.editor.media = media;
    h.editor.ui.show_media_library = true;
    h.editor.ui.media_library.selected = Some("media/image_00.png".to_owned());
    h.idle_frame();

    let layers = capture_layers(&h.ctx);
    let scrolls = crate::scroll_audit::last_recorded_scrolls(&h.ctx);
    let nested = check_nested_scrolls(&layers, &scrolls);
    assert!(
        nested.is_empty(),
        "Media Library must have zero nested scroll areas: {nested:?}"
    );

    let table_h_before = scrolls.first().map(|s| s.rect.height()).unwrap_or(0.0);
    assert!(
        table_h_before > 120.0,
        "Expected media_library_table scroll rect, got {scrolls:?}"
    );

    let win_rect = dialog_rect_by_id(&h, Id::unique("media_library_window"))
        .expect("Media library window rect");
    h.drag_by(
        win_rect.right_bottom() - Vec2::new(3.0, 3.0),
        Vec2::new(160.0, 200.0),
        4,
        Modifiers::NONE,
    );
    h.idle_frame();

    let scrolls_after = crate::scroll_audit::last_recorded_scrolls(&h.ctx);
    let table_h_after = scrolls_after
        .first()
        .map(|s| s.rect.height())
        .unwrap_or(0.0);
    assert!(
        table_h_after > table_h_before + 30.0,
        "Resizing Media library taller must expand the table scroll area (before={table_h_before}, after={table_h_after})"
    );
}

#[test]
fn t9_2_media_library_kind_combo_long_filename_and_disabled_tooltips() {
    let long_name = format!("media/{}.png", "very_long_media_filename_".repeat(4));
    let mut h = TestHarness::new_with_config(
        base_course(),
        (1280.0, 800.0),
        ThemeMode::Dark,
        1.0,
        false, // web mode: Import… & Replace file… disabled
    );
    h.editor.media = vec![(long_name.clone(), minimal_png_bytes())];
    h.editor.ui.show_media_library = true;
    h.idle_frame();

    // Kind combo must pass combo audit (explicit width & height).
    let combo_findings = h.audit_combo("All");
    assert!(
        combo_findings.is_empty(),
        "Kind combo failed audit: {combo_findings:?}"
    );

    // Long filename should be truncated with full-name tooltip on hover, without clipping Size/Used by.
    let report = h.layout_report(1);
    assert!(
        report.findings.is_empty(),
        "Long filename in Media library must not clip: {:?}",
        report.findings
    );
    if let Some(pos) = h.find_text_containing("very_long_media_filename") {
        let tip = h
            .probe_tooltip(pos)
            .expect("Hovering truncated long filename must show tooltip");
        assert!(
            tip.contains(&long_name),
            "Tooltip must show full filename, got: {tip}"
        );
    } else {
        panic!("Expected truncated filename row to be painted");
    }

    // Hover disabled buttons: Delete, Import…, Replace file….
    for label in ["Delete", "Import…", "Replace file…"] {
        let pos = h.find_text(label).expect(label);
        let tip = h.probe_tooltip(pos);
        assert!(
            tip.as_deref().is_some_and(|t| !t.is_empty()),
            "Disabled '{label}' button must show an explanatory tooltip"
        );
    }

    // Test Insert on slide in Master View and Delete unused when all files are used.
    h.editor.ui.master_view = true;
    h.editor.ui.media_library.selected = Some(long_name.clone());
    h.editor.session.course.scenes[0].slides[0].objects =
        serde_json::from_value(serde_json::json!([{
            "id": "img1",
            "name": "Used",
            "frame": { "x": 0.0, "y": 0.0, "w": 100.0, "h": 100.0 },
            "kind": { "type": "image", "src": long_name }
        }]))
        .expect("objects");
    h.editor.session.revision += 1;
    h.idle_frame();

    let insert_pos = h.find_text("Insert on slide").expect("Insert on slide");
    let insert_tip = h
        .probe_tooltip(insert_pos)
        .expect("Disabled 'Insert on slide' in Master View must show tooltip");
    assert!(
        insert_tip.contains("slide masters"),
        "Expected slide masters tooltip, got: {insert_tip}"
    );

    let del_unused_pos = h.find_text("Delete unused").expect("Delete unused");
    let del_unused_tip = h.probe_tooltip(del_unused_pos);
    assert!(
        del_unused_tip.as_deref().is_some_and(|t| !t.is_empty()),
        "Disabled 'Delete unused' when all files are used must show an explanatory tooltip"
    );
}

#[test]
fn t9_3_media_library_empty_state_and_delete_confirmation_escape() {
    let mut h = TestHarness::new_with_size(base_course(), (1280.0, 800.0));
    h.editor.media.clear();
    h.editor.ui.show_media_library = true;
    h.idle_frame();

    assert!(
        h.find_text_containing("No media files").is_some(),
        "Empty Media library must explain how to import media"
    );

    // Add an unused file, trigger Delete, and verify confirmation dialog is sentence-case and Esc closes only the confirmation.
    h.editor.media = vec![("media/unused.png".to_owned(), minimal_png_bytes())];
    h.editor.ui.media_library.selected = Some("media/unused.png".to_owned());
    h.idle_frame();

    let del_pos = h.find_text("Delete").expect("Delete button");
    h.click_at(del_pos);
    h.idle_frame();
    assert!(
        h.editor.ui.media_library.confirm.is_some(),
        "Delete confirmation must open"
    );
    assert!(
        h.find_text("Delete files").is_some(),
        "Delete confirmation title must be sentence-case"
    );

    // First Esc closes confirmation only, keeping Media library open.
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(
        h.editor.ui.media_library.confirm.is_none(),
        "First Esc must close Delete confirmation"
    );
    assert!(
        h.editor.ui.show_media_library,
        "First Esc must keep Media library open"
    );
}

#[test]
fn t9_4_fonts_manager_single_scroll_remove_tooltip_and_confirm_escape_undo() {
    let mut course = base_course();
    course.fonts = (0..15)
        .map(|i| {
            serde_json::json!({
                "family": format!("Custom Font {i:02}"),
                "style": "regular",
                "src": format!("media/font_{i:02}.ttf"),
            })
        })
        .map(|v| serde_json::from_value(v).expect("font face"))
        .collect();
    course.scenes[0].slides[0].objects = serde_json::from_value(serde_json::json!([{
        "id": "t1",
        "name": "Text 1",
        "frame": { "x": 100.0, "y": 100.0, "w": 300.0, "h": 80.0 },
        "kind": {
            "type": "text",
            "text": "Hello",
            "runs": [{ "text": "Hello", "font": "Custom Font 00" }]
        }
    }]))
    .expect("text object");

    let mut h = TestHarness::new_with_size(course, (1280.0, 800.0));
    h.editor.ui.show_fonts_window = true;
    h.idle_frame();

    let layers = capture_layers(&h.ctx);
    let scrolls = crate::scroll_audit::last_recorded_scrolls(&h.ctx);
    let nested = check_nested_scrolls(&layers, &scrolls);
    assert!(
        nested.is_empty(),
        "Fonts Manager must have zero nested scroll areas: {nested:?}"
    );

    let remove_pos = h
        .find_texts("Remove")
        .into_iter()
        .next()
        .expect("Remove button");
    let remove_tip = h.probe_tooltip(remove_pos);
    assert!(
        remove_tip.as_deref().is_some_and(|t| !t.is_empty()),
        "Fonts Manager Remove button must have a descriptive tooltip"
    );

    // Click Remove on used font -> opens Remove font? confirmation.
    h.click_at(remove_pos);
    h.idle_frame();
    assert!(
        h.editor.ui.pending_remove_font.is_some(),
        "Remove font? confirmation must open"
    );

    // First Esc closes only the confirmation dialog, not Fonts.
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(
        h.editor.ui.pending_remove_font.is_none(),
        "Esc must close Remove font? confirmation"
    );
    assert!(
        h.editor.ui.show_fonts_window,
        "Esc on confirmation must keep Fonts window open"
    );

    // Removing an unused font (Custom Font 01) undoes in 1 step.
    let count_before = h.editor.session.course.fonts.len();
    let remove_btns = h.find_texts("Remove");
    h.click_at(remove_btns[1]);
    h.idle_frame();
    assert_eq!(h.editor.session.course.fonts.len(), count_before - 1);
    h.press(Key::Z, Modifiers::COMMAND);
    h.idle_frame();
    assert_eq!(
        h.editor.session.course.fonts.len(),
        count_before,
        "Removing a font must undo in 1 step"
    );
}

#[test]
fn t9_5_text_styles_dialog_is_resizable_single_scroll_and_closes_on_escape() {
    let mut course = base_course();
    for i in 0..15 {
        course.text_styles.push(NamedTextStyle {
            id: format!("ts_{i}"),
            name: format!("Style {i:02}"),
            style: TextStyle::default(),
            paragraphs: None,
        });
    }

    let mut h = TestHarness::new_with_size(course, (1280.0, 900.0));
    h.editor.ui.open_text_styles_window = true;
    h.idle_frame();

    let layers = capture_layers(&h.ctx);
    let scrolls = crate::scroll_audit::last_recorded_scrolls(&h.ctx);
    let nested = check_nested_scrolls(&layers, &scrolls);
    assert!(
        nested.is_empty(),
        "Text styles dialog must have zero nested scroll areas: {nested:?}"
    );

    let win_before = dialog_rect(&h, "Text styles").expect("Text styles window");
    h.drag_by(
        win_before.right_bottom() - Vec2::new(3.0, 3.0),
        Vec2::new(100.0, 140.0),
        4,
        Modifiers::NONE,
    );
    h.idle_frame();
    let win_after = dialog_rect(&h, "Text styles").expect("Text styles window after resize");
    assert!(
        win_after.width() > win_before.width() + 60.0,
        "Text styles dialog must be resizable (before={win_before:?}, after={win_after:?})"
    );

    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(
        h.find_text("No text styles yet").is_none() && h.find_text("Style 00").is_none(),
        "Esc must close Text styles dialog"
    );
}

#[test]
fn t9_6_shape_gallery_window_zero_nested_scroll_search_focus_tooltips_and_escape() {
    let mut h = TestHarness::new_with_size(base_course(), (1280.0, 800.0));
    h.editor.ui.show_shape_gallery_window = true;
    h.idle_frame();

    let layers = capture_layers(&h.ctx);
    let scrolls = crate::scroll_audit::last_recorded_scrolls(&h.ctx);
    let nested = check_nested_scrolls(&layers, &scrolls);
    assert!(
        nested.is_empty(),
        "Shapes dialog must have zero nested scroll areas: {nested:?}"
    );

    // Search field must auto-focus so typing "arrow" immediately filters.
    h.type_text("arrow");
    h.idle_frame();
    assert_eq!(
        h.editor.ui.shape_gallery_query, "arrow",
        "Shape gallery search must auto-focus when opened"
    );

    if let Some((_, rect)) = h.editor.ui.shape_gallery_rects.first().copied() {
        let tip = h.probe_tooltip(rect.center());
        assert!(
            tip.as_deref().is_some_and(|t| !t.is_empty()),
            "Shape gallery icon button must show a tooltip on hover"
        );
    } else {
        panic!("Expected at least one matching arrow shape");
    }

    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(
        !h.editor.ui.show_shape_gallery_window,
        "Esc must close Shapes dialog"
    );
}

#[test]
fn t9_7_interaction_gallery_resizable_contrast_and_escape_in_both_themes() {
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
        for size in [(1024.0, 600.0), (1440.0, 900.0)] {
            let mut h = TestHarness::new_with_config(base_course(), size, mode, 1.0, true);
            h.editor.ui.show_interaction_gallery = true;
            h.idle_frame();

            let layers = capture_layers(&h.ctx);
            let scrolls = crate::scroll_audit::last_recorded_scrolls(&h.ctx);
            let nested = check_nested_scrolls(&layers, &scrolls);
            assert!(
                nested.is_empty(),
                "Interaction gallery must have zero nested scrolls ({mode:?}, {size:?}): {nested:?}"
            );

            let p = coursecraft_render::palette(&h.ctx);
            let contrast = check_contrast(&layers, &[], &p, &[]);
            assert!(
                contrast.is_empty(),
                "Interaction gallery must pass >= 4.5:1 contrast ({mode:?}, {size:?}): {contrast:?}"
            );
        }
    }

    let mut h = TestHarness::new_with_size(base_course(), (1440.0, 900.0));
    h.editor.ui.show_interaction_gallery = true;
    h.idle_frame();
    let win_before = dialog_rect(&h, "Insert interaction")
        .or_else(|| dialog_rect(&h, "Insert Interaction"))
        .expect("Interaction gallery window");
    h.drag_by(
        win_before.right_bottom() - Vec2::new(3.0, 3.0),
        Vec2::new(80.0, 100.0),
        4,
        Modifiers::NONE,
    );
    h.idle_frame();
    let win_after = dialog_rect(&h, "Insert interaction")
        .or_else(|| dialog_rect(&h, "Insert Interaction"))
        .expect("Interaction gallery window after resize");
    assert!(
        win_after.width() > win_before.width() + 40.0,
        "Insert interaction gallery must be resizable"
    );

    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(
        !h.editor.ui.show_interaction_gallery,
        "Esc must close Insert interaction gallery"
    );
}

#[test]
fn t9_8_web_insert_dialog_validation_disabled_tooltip_and_escape() {
    let mut h = TestHarness::new_with_size(base_course(), (1280.0, 800.0));
    h.editor.ui.show_web_window = true;
    h.editor.ui.web_url.clear();
    h.idle_frame();

    // Empty URL -> Insert address disabled with explanatory tooltip.
    let insert_pos = h
        .find_text("Insert address")
        .expect("Insert address button");
    let tip = h.probe_tooltip(insert_pos);
    assert!(
        tip.as_deref().is_some_and(|t| !t.is_empty()),
        "Disabled 'Insert address' with empty URL must explain why on hover"
    );

    // Invalid URL "ftp://bad" -> actionable validation message + disabled tooltip.
    h.editor.ui.web_url = "ftp://bad".to_owned();
    h.idle_frame();
    assert!(
        h.find_text_containing("https://").is_some(),
        "Invalid URL must show actionable validation message mentioning https://"
    );
    let insert_pos = h
        .find_text("Insert address")
        .expect("Insert address button");
    let tip = h.probe_tooltip(insert_pos);
    assert!(
        tip.as_deref().is_some_and(|t| !t.is_empty()),
        "Disabled 'Insert address' with invalid URL must explain why on hover"
    );

    // Esc closes dialog.
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(
        !h.editor.ui.show_web_window,
        "Esc must close Insert web object dialog"
    );
}
