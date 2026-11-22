//! Step u13 regression tests: Core Authoring Dialogs (`Area 10`).

use crate::test_harness::{TestHarness, capture_layers, check_contrast, check_nested_scrolls};
use crate::version_history::VersionRow;
use coursecraft_course::Course;
use coursecraft_render::{ThemeMode, ThemePref, palette};
use egui::{Id, Key, Modifiers, Rect, Vec2};

fn base_course() -> Course {
    let raw = serde_json::json!({
        "schema_version": 1,
        "id": "c-u13",
        "title": "Step u13 Course",
        "slide_size": { "w": 1280, "h": 720 },
        "masters": [{ "id": "m1", "title": "Default Master", "objects": [] }],
        "scenes": [{
            "id": "scene1",
            "title": "Main",
            "slides": [{
                "id": "s1",
                "title": "Slide 1",
                "duration_ms": 60000,
                "objects": [
                    {
                        "id": "t1",
                        "name": "Text 1",
                        "frame": { "x": 100.0, "y": 100.0, "w": 300.0, "h": 80.0 },
                        "kind": { "type": "text", "text": "Helo wrld from alpha" }
                    }
                ],
                "layers": [],
                "triggers": []
            }]
        }]
    });
    serde_json::from_value(raw).expect("valid course json")
}

fn dialog_rect_by_id(h: &TestHarness, id: Id) -> Option<Rect> {
    h.ctx.memory(|m| m.area_rect(id))
}

fn dialog_rect(h: &TestHarness, title: &str) -> Option<Rect> {
    dialog_rect_by_id(h, Id::unique(("coursecraft_dialog", title)))
}

#[test]
fn t10_1_uniform_sentence_case_dialog_titles_and_form_alignment() {
    let mut h = TestHarness::new_with_size(base_course(), (1280.0, 800.0));

    // Course settings
    h.editor.ui.show_course_settings_window = true;
    h.idle_frame();
    assert!(
        h.find_text("Course settings").is_some(),
        "Expected sentence-case title 'Course settings'"
    );
    assert!(
        h.find_text("Course title").is_some(),
        "Expected sentence-case label 'Course title'"
    );
    assert!(
        h.find_text("Player chrome").is_some(),
        "Expected sentence-case heading 'Player chrome'"
    );
    assert!(
        h.find_text("Show seekbar").is_some(),
        "Expected sentence-case checkbox 'Show seekbar'"
    );
    h.editor.ui.show_course_settings_window = false;
    h.idle_frame();

    // Theme
    h.editor.ui.show_theme_window = true;
    h.idle_frame();
    assert!(h.find_text("Theme").is_some());
    assert!(
        h.find_text("Slide background").is_some(),
        "Expected sentence-case label 'Slide background'"
    );
    assert!(
        h.find_text("Player chrome").is_some(),
        "Expected sentence-case heading 'Player chrome'"
    );
    h.editor.ui.show_theme_window = false;
    h.idle_frame();

    // Export video & Export for review
    h.editor.open_export_dialog();
    h.idle_frame();
    assert!(
        h.find_text("Export video").is_some(),
        "Expected sentence-case title 'Export video'"
    );
    h.editor.ui.show_export_video = false;
    h.idle_frame();

    h.editor
        .open_export_review(crate::export_dialog::ReviewFormat::Pdf);
    h.idle_frame();
    assert!(
        h.find_text("Export for review").is_some(),
        "Expected sentence-case title 'Export for review'"
    );
    h.editor.ui.show_export_review = false;
    h.idle_frame();

    // Version history
    h.editor.show_version_history(vec![VersionRow {
        id: "v1".to_owned(),
        when: "2026-11-22 10:00:00 UTC".to_owned(),
        size: "1.0 MB".to_owned(),
        slides: Some(1),
    }]);
    h.idle_frame();
    assert!(
        h.find_text("Version history").is_some(),
        "Expected sentence-case title 'Version history'"
    );
    assert!(
        h.find_text("Open as copy").is_some(),
        "Expected sentence-case button 'Open as copy'"
    );
    h.editor.ui.versions.open = false;
    h.idle_frame();

    // Find and replace
    crate::find::open(&mut h.editor);
    h.idle_frame();
    assert!(
        h.find_text("Find and replace").is_some(),
        "Expected sentence-case title 'Find and replace'"
    );
    assert!(
        h.find_text("Replace all").is_some(),
        "Expected sentence-case button 'Replace all'"
    );
    h.editor.ui.find.open = false;
    h.idle_frame();

    // Check spelling
    h.editor.ui.spell_check.open = true;
    h.idle_frame();
    assert!(
        h.find_text("Check spelling").is_some(),
        "Expected sentence-case title 'Check spelling'"
    );
    assert!(
        h.find_text("Change all").is_some(),
        "Expected sentence-case button 'Change all'"
    );
    assert!(
        h.find_text("Add to dictionary").is_some(),
        "Expected sentence-case button 'Add to dictionary'"
    );
    h.editor.ui.spell_check.open = false;
    h.idle_frame();

    // Slide size
    h.editor.open_slide_size_dialog();
    h.idle_frame();
    assert!(
        h.find_text("Slide size").is_some(),
        "Expected sentence-case title 'Slide size'"
    );
}

#[test]
fn t10_2_centralized_dialog_escape_and_popup_first() {
    let mut h = TestHarness::new_with_size(base_course(), (1280.0, 800.0));

    // Course settings: open Navigation combo, first Esc closes combo, second Esc closes dialog.
    h.editor.ui.show_course_settings_window = true;
    h.idle_frame();
    let combo_pos = h
        .find_text("Free")
        .expect("Navigation combo in Course settings");
    h.click_at(combo_pos);
    h.idle_frame();
    assert!(
        egui::Popup::is_any_open(&h.ctx),
        "Navigation combo popup should be open"
    );
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(
        h.editor.ui.show_course_settings_window,
        "First Esc must close only the open combo, not Course settings"
    );
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(
        !h.editor.ui.show_course_settings_window,
        "Second Esc must close Course settings"
    );

    // Theme
    h.editor.ui.show_theme_window = true;
    h.idle_frame();
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(
        !h.editor.ui.show_theme_window,
        "Esc must close Theme dialog"
    );

    // Slide size
    h.editor.open_slide_size_dialog();
    h.idle_frame();
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(
        !h.editor.ui.slide_size_dialog.open,
        "Esc must close Slide size dialog"
    );

    // Preferences
    h.editor.open_preferences();
    h.idle_frame();
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(!h.editor.ui.prefs.open, "Esc must close Preferences dialog");

    // Publish
    h.editor.ui.publish.open = true;
    h.idle_frame();
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(!h.editor.ui.publish.open, "Esc must close Publish dialog");

    // Version history
    h.editor.show_version_history(vec![]);
    h.idle_frame();
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(
        !h.editor.ui.versions.open,
        "Esc must close Version history dialog"
    );

    // Find and replace
    crate::find::open(&mut h.editor);
    h.idle_frame();
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(
        !h.editor.ui.find.open,
        "Esc must close Find and replace dialog"
    );

    // Export video (with FPS combo open first)
    h.editor.open_export_dialog();
    h.idle_frame();
    let fps_pos = h.find_text("30 fps").expect("FPS combo in Export video");
    h.click_at(fps_pos);
    h.idle_frame();
    assert!(egui::Popup::is_any_open(&h.ctx));
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(
        h.editor.ui.show_export_video,
        "First Esc must close FPS combo only"
    );
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(
        !h.editor.ui.show_export_video,
        "Second Esc must close Export video"
    );

    // Check spelling
    h.editor.ui.spell_check.open = true;
    h.idle_frame();
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(
        !h.editor.ui.spell_check.open,
        "Esc must close Check spelling dialog"
    );

    // History
    h.editor.ui.show_history = true;
    h.idle_frame();
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(!h.editor.ui.show_history, "Esc must close History dialog");
}

#[test]
fn t10_3_course_settings_resizable_combos_and_resources_growth() {
    let mut course = base_course();
    for i in 0..10 {
        course.resources.push(coursecraft_course::Resource {
            id: format!("res_{i}"),
            title: format!("Resource {i}"),
            target: coursecraft_course::ResourceTarget::Url {
                url: format!("https://example.com/{i}"),
            },
        });
    }
    for i in 0..15 {
        course.glossary.push(coursecraft_course::GlossaryEntry {
            id: format!("gl_{i}"),
            term: format!("Term {i}"),
            definition: format!("Definition for term {i}"),
        });
    }

    let mut h = TestHarness::new_with_size(course, (1280.0, 900.0));
    h.editor.ui.show_course_settings_window = true;
    h.idle_frame();
    h.idle_frame();

    let rect_before = dialog_rect(&h, "Course settings").expect("Course settings rect");
    h.drag_by(
        rect_before.right_bottom() - Vec2::new(3.0, 3.0),
        Vec2::new(100.0, 0.0),
        4,
        Modifiers::NONE,
    );
    h.idle_frame();
    let rect_after = dialog_rect(&h, "Course settings").expect("Course settings rect after");
    assert!(
        rect_after.width() > rect_before.width() + 20.0,
        "Course settings must be resizable: before={rect_before:?}, after={rect_after:?}"
    );

    // Disabled Clear button next to Logo combo must explain why on hover.
    let clear_pos = h.find_text("Clear").expect("Clear logo button");
    let clear_tip = h
        .probe_tooltip(clear_pos)
        .expect("Disabled Clear logo button must explain why on hover");
    assert!(
        clear_tip.contains("No logo is selected"),
        "Unexpected Clear tooltip: {clear_tip}"
    );

    // Check Navigation combo exists and is clickable.
    let nav_pos = h.find_text("Free").expect("Navigation combo");
    assert!(rect_after.contains(nav_pos));
}

#[test]
fn t10_4_publish_dialog_validation_feedback_disabled_tooltip_and_footer() {
    let mut h = TestHarness::new_with_size(base_course(), (1024.0, 600.0));
    h.ctx.set_pixels_per_point(1.5);
    h.editor.ui.validation_errors = 2;
    h.editor.ui.publish.open = true;
    h.idle_frame();

    let win_rect = dialog_rect(&h, "Publish course").expect("Publish course rect");
    assert!(
        win_rect.height() <= 600.0,
        "Publish dialog must fit 1024x600 at 1.5x scale: {win_rect:?}"
    );

    // Publish button should be disabled and explain why on hover.
    let publish_pos = h.find_text_last("Publish").expect("Publish button");
    let publish_tip = h
        .probe_tooltip(publish_pos)
        .expect("Disabled Publish button must explain why on hover");
    assert!(
        publish_tip.contains("Fix validation problems"),
        "Unexpected Publish tooltip: {publish_tip}"
    );

    // Footer Cancel button must be visible inside the dialog rect.
    let win_rect = dialog_rect(&h, "Publish course").expect("Publish course rect");
    let cancel_pos = h.find_text("Cancel").expect("Cancel button");
    assert!(
        win_rect.contains(cancel_pos),
        "Footer Cancel button {cancel_pos:?} must stay inside Publish dialog {win_rect:?}"
    );
}

#[test]
fn t10_5_version_history_single_scroll_resizable_growth_and_sentence_case() {
    let mut h = TestHarness::new_with_size(base_course(), (1280.0, 900.0));
    let mut rows = Vec::new();
    for i in 0..25 {
        rows.push(VersionRow {
            id: format!("v{i}"),
            when: format!("2026-11-{:02} 12:00:00 UTC", (i % 28) + 1),
            size: "1.2 MB".to_owned(),
            slides: if i == 0 { None } else { Some(i + 1) },
        });
    }
    h.editor.show_version_history(rows);
    h.idle_frame();

    let layers = capture_layers(&h.ctx);
    let scrolls = crate::scroll_audit::last_recorded_scrolls(&h.ctx);
    let nested = check_nested_scrolls(&layers, &scrolls);
    assert!(
        nested.is_empty(),
        "Version history must have zero nested scroll areas: {nested:?}"
    );
    assert!(
        h.find_text("Open as copy").is_some(),
        "Expected sentence case 'Open as copy'"
    );

    let scroll_h_before = scrolls.first().map(|s| s.rect.height()).unwrap_or(0.0);
    let win_rect = dialog_rect(&h, "Version history").expect("Version history rect");
    h.drag_by(
        win_rect.right_bottom() - Vec2::new(3.0, 3.0),
        Vec2::new(40.0, 200.0),
        4,
        Modifiers::NONE,
    );
    h.idle_frame();

    let scrolls_after = crate::scroll_audit::last_recorded_scrolls(&h.ctx);
    let scroll_h_after = scrolls_after
        .first()
        .map(|s| s.rect.height())
        .unwrap_or(0.0);
    assert!(
        scroll_h_after > scroll_h_before + 80.0,
        "Version history scroll area must grow when resized taller: before={scroll_h_before}, after={scroll_h_after}"
    );

    // Empty and folder-project states give clear guidance.
    h.editor.set_version_rows(vec![]);
    h.idle_frame();
    assert!(h.find_text_containing("No earlier versions yet").is_some());

    h.editor.set_project_is_folder(true);
    h.idle_frame();
    assert!(h.find_text_containing("git is its history").is_some());
}

#[test]
fn t10_6_find_and_replace_single_scroll_resizable_remembered_size_and_disabled_tooltips() {
    let mut course = base_course();
    for i in 0..50 {
        course.scenes[0].slides[0].objects.push(
            serde_json::from_value(serde_json::json!({
                "id": format!("obj_{i}"),
                "name": format!("Item {i}"),
                "frame": { "x": 10.0, "y": 10.0, "w": 100.0, "h": 40.0 },
                "kind": { "type": "text", "text": format!("alpha item {i}") }
            }))
            .expect("valid text object"),
        );
    }

    let mut h = TestHarness::new_with_size(course, (1280.0, 900.0));
    crate::find::open(&mut h.editor);
    h.editor.ui.find.query = "alpha".to_owned();
    h.idle_frame();

    let layers = capture_layers(&h.ctx);
    let scrolls = crate::scroll_audit::last_recorded_scrolls(&h.ctx);
    let nested = check_nested_scrolls(&layers, &scrolls);
    assert!(
        nested.is_empty(),
        "Find and replace must have zero nested scroll areas: {nested:?}"
    );

    let win_rect = dialog_rect_by_id(&h, Id::unique("find_and_replace_window"))
        .or_else(|| dialog_rect(&h, "Find and replace"))
        .expect("Find and replace rect");
    h.drag_by(
        win_rect.right_bottom() - Vec2::new(3.0, 3.0),
        Vec2::new(80.0, 120.0),
        4,
        Modifiers::NONE,
    );
    h.idle_frame();
    let resized_rect = dialog_rect_by_id(&h, Id::unique("find_and_replace_window"))
        .or_else(|| dialog_rect(&h, "Find and replace"))
        .expect("Resized Find and replace rect");
    assert!(
        resized_rect.width() > win_rect.width() + 20.0,
        "Find and replace must be resizable: before={win_rect:?}, after={resized_rect:?}"
    );

    // Close and reopen: must remember size!
    h.editor.ui.find.open = false;
    h.idle_frame();
    crate::find::open(&mut h.editor);
    h.idle_frame();
    let reopened_rect = dialog_rect_by_id(&h, Id::unique("find_and_replace_window"))
        .or_else(|| dialog_rect(&h, "Find and replace"))
        .expect("Reopened Find and replace rect");
    assert!(
        (reopened_rect.width() - resized_rect.width()).abs() < 4.0,
        "Find and replace must remember its size across reopen: resized={resized_rect:?}, reopened={reopened_rect:?}"
    );

    // Clear query so Replace / Replace all are disabled, then hover to check tooltip.
    h.editor.ui.find.query.clear();
    h.idle_frame();
    let replace_all_pos = h.find_text("Replace all").expect("Replace all button");
    let replace_all_tip = h
        .probe_tooltip(replace_all_pos)
        .expect("Disabled Replace all must explain why on hover");
    assert!(
        replace_all_tip.contains("Enter search text"),
        "Unexpected Replace all tooltip: {replace_all_tip}"
    );
}

#[test]
fn t10_7_preferences_form_alignment_width_and_immediate_theme_apply() {
    let mut h = TestHarness::new_with_size(base_course(), (1024.0, 600.0));
    h.ctx.set_pixels_per_point(1.5);
    h.editor.open_preferences();
    h.idle_frame();

    let win_rect = dialog_rect(&h, "Preferences").expect("Preferences rect");
    assert!(
        win_rect.width() >= 400.0,
        "Preferences dialog must have comfortable width >= 400 px, got {}",
        win_rect.width()
    );

    // Switch theme radio to Light.
    let light_pos = h.find_text("Light").expect("Light theme radio");
    h.click_at(light_pos);
    h.idle_frame();
    assert_eq!(
        h.editor.theme_pref,
        ThemePref::Light,
        "Switching theme in Preferences must apply immediately"
    );

    // Esc closes Preferences.
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(!h.editor.ui.prefs.open);
}

#[test]
fn t10_8_slide_size_and_export_units_and_disabled_states() {
    let mut h = TestHarness::new_with_size(base_course(), (1280.0, 800.0));
    h.editor.file_ops_available = false;

    // Slide size: invalid w = 0, check " px" unit and disabled Apply tooltip.
    h.editor.open_slide_size_dialog();
    h.editor.ui.slide_size_dialog.w = 0;
    h.idle_frame();
    assert!(
        h.find_text_containing(" px").is_some(),
        "Slide size DragValue must show ' px' unit"
    );
    let apply_pos = h.find_text("Apply").expect("Apply button");
    let apply_tip = h
        .probe_tooltip(apply_pos)
        .expect("Disabled Apply in Slide size must explain valid range on hover");
    assert!(
        apply_tip.contains("Width must be 100"),
        "Unexpected Apply tooltip: {apply_tip}"
    );
    h.editor.ui.slide_size_dialog.open = false;
    h.idle_frame();

    // Export video: disabled Export… explains why.
    h.editor.open_export_dialog();
    h.idle_frame();
    let export_pos = h.find_text("Export…").expect("Export button");
    let export_tip = h
        .probe_tooltip(export_pos)
        .expect("Disabled Export… must explain why on hover");
    assert!(
        export_tip.contains("Available in the desktop app"),
        "Unexpected Export video tooltip: {export_tip}"
    );
    h.editor.ui.show_export_video = false;
    h.idle_frame();

    // Export for review: disabled Export… explains why.
    h.editor
        .open_export_review(crate::export_dialog::ReviewFormat::Pdf);
    h.idle_frame();
    let export_review_pos = h.find_text("Export…").expect("Export review button");
    let export_review_tip = h
        .probe_tooltip(export_review_pos)
        .expect("Disabled Export… in Export for review must explain why on hover");
    assert!(
        export_review_tip.contains("Available in the desktop app"),
        "Unexpected Export review tooltip: {export_review_tip}"
    );
}

#[test]
fn t10_9_check_spelling_and_theme_contrast_and_undo() {
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
        let mut h = TestHarness::new_with_config(base_course(), (1280.0, 800.0), mode, 1.0, true);
        let p = palette(&h.ctx);
        h.editor.ui.spell_check.open = true;
        h.idle_frame();

        let layers = capture_layers(&h.ctx);
        let contrast = check_contrast(&layers, &[], &p, &[]);
        assert!(
            contrast.is_empty(),
            "Check spelling contrast violations in {mode:?}: {contrast:?}"
        );
        assert_eq!(
            crate::spell_check::squiggle_color(&h.ctx),
            p.danger,
            "Spelling squiggle color must use palette(ctx).danger in {mode:?}"
        );
    }

    // Theme dialog: change accent and verify undo.
    let mut h = TestHarness::new_with_size(base_course(), (1280.0, 800.0));
    let original_accent = h.editor.session.course.theme.accent.0.clone();
    h.editor.ui.show_theme_window = true;
    h.idle_frame();
    h.editor.run(
        "course.set_theme",
        serde_json::json!({ "accent": "#FF5500" }),
    );
    h.idle_frame();
    assert_eq!(h.editor.session.course.theme.accent.0, "#FF5500");
    h.editor.session.undo().expect("undo theme change");
    h.idle_frame();
    assert_eq!(h.editor.session.course.theme.accent.0, original_accent);
}
