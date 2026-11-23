use crate::menu::{font_has_glyph, format_shortcut};
use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use coursecraft_engine::registry;
use coursecraft_render::{ThemeMode, contrast_ratio, palette};
use egui::pos2;

fn sample_course() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/healthy-habits/course.json"
    ))
    .expect("healthy habits");
    Course::from_json(&json).expect("valid course")
}

fn widget_with_label(h: &TestHarness, needle: &str) -> crate::test_harness::WidgetNode {
    h.last_widgets
        .iter()
        .find(|w| w.label.as_deref().is_some_and(|l| l.contains(needle)))
        .cloned()
        .unwrap_or_else(|| panic!("expected widget with label containing {needle:?}"))
}

fn toolbar_widget_with_label(h: &TestHarness, needle: &str) -> crate::test_harness::WidgetNode {
    let toolbar_bottom = crate::TOP_BAR_HEIGHT + crate::toolbar::TOOLBAR_HEIGHT;
    h.last_widgets
        .iter()
        .find(|w| {
            w.bounds.center().y <= toolbar_bottom
                && w.label.as_deref().is_some_and(|l| l.contains(needle))
        })
        .cloned()
        .unwrap_or_else(|| panic!("expected toolbar widget with label containing {needle:?}"))
}

#[test]
fn desktop_toolbar_fits_without_overlap_when_publish_is_visible() {
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
        for (w, h_px) in [(1100.0, 700.0), (1280.0, 800.0)] {
            let mut h = TestHarness::new_with_config(sample_course(), (w, h_px), mode, 1.0, true);
            let report = h.layout_report(2);
            let toolbar_findings: Vec<_> = report
                .findings
                .iter()
                .filter(|f| {
                    f.contains("overlap") || f.contains("clipped") || f.contains("off-window")
                })
                .collect();
            assert!(
                toolbar_findings.is_empty(),
                "expected 0 layout findings at ({w}, {h_px}) {mode:?}, got: {toolbar_findings:?}"
            );

            let toolbar_bottom = crate::TOP_BAR_HEIGHT + crate::toolbar::TOOLBAR_HEIGHT;
            let left_bounds: Vec<_> = [
                "New slide",
                "More slide options",
                "Preview this slide",
                "More preview options",
                "Publish course",
                "Course map",
                "Format painter",
                "Select an object to copy its format",
            ]
            .iter()
            .filter_map(|needle| {
                h.last_widgets
                    .iter()
                    .find(|w| {
                        w.bounds.center().y <= toolbar_bottom
                            && w.label.as_deref().is_some_and(|l| l.contains(needle))
                    })
                    .map(|w| (needle, w.bounds))
            })
            .collect();
            let center_bounds: Vec<_> = [
                "Insert text box",
                "Insert shape",
                "Insert image",
                "Insert media",
                "Insert button",
                "Insert question",
                "Insert trigger",
            ]
            .iter()
            .filter_map(|needle| {
                h.last_widgets
                    .iter()
                    .find(|w| w.label.as_deref().is_some_and(|l| l.contains(needle)))
                    .map(|w| (needle, w.bounds))
            })
            .collect();
            let right_bounds: Vec<_> = [
                "Undo",
                "Redo",
                "Format inspector",
                "Animate inspector",
                "Slide inspector",
                "Course inspector",
            ]
            .iter()
            .filter_map(|needle| {
                h.last_widgets
                    .iter()
                    .find(|w| w.label.as_deref().is_some_and(|l| l.contains(needle)))
                    .map(|w| (needle, w.bounds))
            })
            .collect();

            for (ln, lb) in &left_bounds {
                for (cn, cb) in &center_bounds {
                    assert!(
                        !lb.intersects(*cb),
                        "left toolbar item {ln} ({lb:?}) overlaps centre item {cn} ({cb:?}) at ({w}, {h_px}) {mode:?}"
                    );
                }
            }
            for (cn, cb) in &center_bounds {
                for (rn, rb) in &right_bounds {
                    assert!(
                        !cb.intersects(*rb),
                        "centre toolbar item {cn} ({cb:?}) overlaps right item {rn} ({rb:?}) at ({w}, {h_px}) {mode:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn toolbar_collapses_to_icon_only_at_minimum_width_with_tooltips() {
    for (size, scale) in [((1024.0, 600.0), 1.0), ((1440.0, 900.0), 1.5)] {
        let mut h =
            TestHarness::new_with_config(sample_course(), size, ThemeMode::Dark, scale, true);
        let first_obj = h.editor.session.course.scenes[0].slides[0].objects[0]
            .id
            .clone();
        h.editor.selection.set_many([first_obj.as_str()]);
        let report = h.layout_report(2);
        assert!(
            report.findings.is_empty(),
            "expected 0 layout findings at {size:?} scale {scale}, got: {:?}",
            report.findings
        );

        assert!(
            h.find_text("New slide").is_none(),
            "expected 'New slide' label to collapse to icon-only at {size:?} scale {scale}"
        );

        for needle in [
            "New slide",
            "Preview this slide",
            "Publish course",
            "Course map",
            "Format painter",
            "Insert text box",
            "Insert shape",
            "Insert image",
            "Insert media",
            "Insert button",
            "Insert question",
            "Insert trigger",
        ] {
            let w = toolbar_widget_with_label(&h, needle);
            assert!(
                w.bounds.width() >= 36.0 && w.bounds.height() >= 44.0,
                "toolbar button {needle:?} hit target {:?} smaller than 36x44 at {size:?} scale {scale}",
                w.bounds.size()
            );
            let tip = h.probe_tooltip(w.bounds.center());
            assert!(
                tip.as_deref().is_some_and(|t| !t.is_empty()),
                "expected tooltip when hovering {needle:?} at {size:?} scale {scale}"
            );
        }
    }
}

#[test]
fn course_map_toolbar_button_uses_clean_room_map_label() {
    let mut h =
        TestHarness::new_with_config(sample_course(), (1280.0, 800.0), ThemeMode::Dark, 1.0, true);
    h.layout_report(2);

    assert!(
        h.find_text("Story").is_none(),
        "clean-room violation: toolbar must not render 'Story'"
    );
    let map_pos = h
        .find_text("Map")
        .expect("toolbar should render 'Map' label at 1280x800");
    let tip = h
        .probe_tooltip(map_pos)
        .expect("Map button should show a tooltip");
    assert!(
        tip.starts_with("Course map ("),
        "expected tooltip 'Course map (...)', got {tip:?}"
    );
}

#[test]
fn master_view_disables_unavailable_toolbar_buttons_with_tooltip() {
    let mut h =
        TestHarness::new_with_config(sample_course(), (1280.0, 800.0), ThemeMode::Dark, 1.0, true);
    h.editor.ui.master_view = true;
    h.layout_report(2);

    for label in [
        "New slide",
        "Preview",
        "Map",
        "Paint",
        "Media",
        "Question",
        "Trigger",
    ] {
        let pos = h
            .find_text(label)
            .unwrap_or_else(|| panic!("expected toolbar button {label:?}"));
        let tip = h.probe_tooltip(pos);
        assert_eq!(
            tip.as_deref(),
            Some("Not available in slide masters"),
            "expected 'Not available in slide masters' tooltip on {label:?}, got {tip:?}"
        );
    }

    let map_widget = widget_with_label(&h, "Not available in slide masters");
    assert!(
        map_widget.disabled,
        "disabled toolbar buttons must expose disabled state"
    );
    let map_pos = h.find_text("Map").expect("Map label");
    h.click_at(map_pos);
    assert!(
        !h.editor.ui.story_view,
        "clicking Map in Master View must not enter Course map"
    );
}

#[test]
fn format_painter_shows_disabled_tooltip_when_nothing_selected() {
    let mut h =
        TestHarness::new_with_config(sample_course(), (1280.0, 800.0), ThemeMode::Dark, 1.0, true);
    h.editor.selection.clear_objects();
    h.layout_report(2);

    let paint_pos = h.find_text("Paint").expect("Paint toolbar button");
    let tip = h.probe_tooltip(paint_pos);
    assert_eq!(
        tip.as_deref(),
        Some("Select an object to copy its format"),
        "expected disabled tooltip on Format Painter when nothing is selected, got {tip:?}"
    );
}

#[test]
fn toolbar_carets_meet_hit_target_and_popups_clamp_and_close_on_escape() {
    let mut h =
        TestHarness::new_with_config(sample_course(), (1024.0, 600.0), ThemeMode::Dark, 1.0, true);
    let first_obj = h.editor.session.course.scenes[0].slides[0].objects[0]
        .id
        .clone();
    h.editor.selection.set_many([first_obj.as_str()]);
    h.layout_report(2);

    for caret_label in ["More slide options", "More preview options"] {
        let caret = widget_with_label(&h, caret_label);
        assert!(
            caret.bounds.width() >= 20.0 && caret.bounds.height() >= 44.0,
            "caret {caret_label:?} must be >= 20x44 pt, got {:?}",
            caret.bounds.size()
        );
    }

    for (needle, expected_item) in [
        ("More slide options", "New scene"),
        ("More preview options", "Preview this slide"),
        ("Insert shape", "Rectangle"),
        ("Insert media", "Video…"),
        ("Insert question", "Multiple choice"),
    ] {
        let w = widget_with_label(&h, needle);
        h.click_at(w.bounds.center());
        h.idle_frame();
        h.idle_frame();

        let item_pos = h
            .find_text(expected_item)
            .or_else(|| h.find_text_containing(expected_item))
            .unwrap_or_else(|| {
                panic!("popup for {needle:?} did not open (missing {expected_item:?})")
            });
        assert!(
            h.screen_rect.contains(item_pos),
            "popup for {needle:?} item {expected_item:?} at {item_pos:?} escaped screen_rect {:?}",
            h.screen_rect
        );
        let report = h.layout_report(0);
        let off_window: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.contains("off-window"))
            .collect();
        assert!(
            off_window.is_empty(),
            "popup for {needle:?} had off-window findings at 1024x600: {off_window:?}"
        );

        h.press(egui::Key::Escape, egui::Modifiers::NONE);
        h.idle_frame();
        assert_eq!(
            h.editor.selection.object.as_deref(),
            Some(first_obj.as_str()),
            "closing {needle:?} popup with Esc must preserve object selection"
        );
    }
}

#[test]
fn menu_bar_disabled_items_show_reason_tooltip_when_nothing_selected() {
    let mut h =
        TestHarness::new_with_config(sample_course(), (1280.0, 800.0), ThemeMode::Dark, 1.0, true);
    h.editor.selection.clear_objects();
    h.layout_report(2);

    let arrange_pos = h.find_text("Arrange").expect("Arrange menu");
    h.click_at(arrange_pos);
    h.idle_frame();

    for item in ["Group", "Ungroup", "Align left", "Bring forward"] {
        let pos = h
            .find_text(item)
            .unwrap_or_else(|| panic!("expected {item:?} in Arrange menu"));
        h.hover_at(pos);
        let tip = h.probe_tooltip(pos);
        assert!(
            tip.as_deref().is_some_and(|t| !t.is_empty()),
            "disabled menu item {item:?} should show a reason tooltip, got {tip:?}"
        );
    }
}

#[test]
fn canvas_object_context_menu_clamps_inside_screen_near_bottom_right() {
    let mut course = sample_course();
    let frame = &mut course.scenes[0].slides[0].objects[0].frame;
    frame.x = 700.0;
    frame.y = 420.0;
    frame.w = 240.0;
    frame.h = 100.0;

    let mut h = TestHarness::new_with_config(course, (1024.0, 600.0), ThemeMode::Dark, 1.0, true);
    h.layout_report(2);

    let central = h.central_panel_rect();
    let click_pos = pos2(central.max.x - 20.0, central.max.y - 20.0);
    h.right_click_at(click_pos);
    h.idle_frame();
    h.idle_frame();

    let paste_pos = h
        .find_text_last("Paste")
        .or_else(|| h.find_text_last("Select all"))
        .expect("context menu should open near bottom-right");
    assert!(
        h.screen_rect.contains(paste_pos),
        "context menu item {paste_pos:?} must be inside screen_rect {:?}",
        h.screen_rect
    );
    let report = h.layout_report(0);
    let off_window: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.contains("off-window"))
        .collect();
    assert!(
        off_window.is_empty(),
        "expected 0 off-window findings for bottom-right context menu, got {off_window:?}"
    );
}

#[test]
fn menu_shortcuts_render_without_missing_glyphs_and_pass_contrast() {
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
        let mut h = TestHarness::new_with_config(sample_course(), (1280.0, 800.0), mode, 1.0, true);
        h.layout_report(1);

        let p = palette(&h.ctx);
        let cr = contrast_ratio(p.text_secondary, p.raised);
        assert!(
            cr >= 4.5,
            "shortcut text contrast (text_secondary on raised) in {mode:?} is {cr:.2}, expected >= 4.5"
        );

        for cmd in registry() {
            if let Some(sc) = cmd.shortcut {
                for is_mac in [true, false] {
                    let formatted = format_shortcut(sc, is_mac, |c| font_has_glyph(&h.ctx, c));
                    assert!(
                        !formatted.is_empty(),
                        "shortcut {sc:?} formatted to empty string"
                    );
                    for ch in formatted.chars() {
                        assert!(
                            font_has_glyph(&h.ctx, ch),
                            "missing glyph {ch:?} in formatted shortcut {formatted:?} (from {sc:?}, is_mac={is_mac})"
                        );
                    }
                }
            }
        }
    }
}

/// L-02: Menus, toolbar, and navigator use canonical sentence-case names (`M32-01`, `M32-02`).
#[test]
fn menus_toolbar_and_navigator_use_canonical_sentence_case_names() {
    let mut h =
        TestHarness::new_with_config(sample_course(), (1280.0, 800.0), ThemeMode::Dark, 1.0, true);
    h.layout_report(2);

    // Toolbar uses "New slide" (never "Add slide").
    assert!(
        h.find_text("New slide").is_some(),
        "toolbar must render canonical 'New slide' label"
    );
    assert!(
        h.find_text("Add slide").is_none(),
        "toolbar must not render legacy 'Add slide' label"
    );

    // Preview caret dropdown uses "Preview this slide" and "Preview whole course".
    let preview_caret = widget_with_label(&h, "More preview options");
    h.click_at(preview_caret.bounds.center());
    h.idle_frame();
    h.idle_frame();
    assert!(
        h.find_text("Preview this slide").is_some(),
        "Preview dropdown must show 'Preview this slide'"
    );
    assert!(
        h.find_text("Preview whole course").is_some(),
        "Preview dropdown must show 'Preview whole course'"
    );
    assert!(
        h.find_text("Preview from the start").is_none(),
        "Preview dropdown must not show legacy 'Preview from the start'"
    );
    h.press(egui::Key::Escape, egui::Modifiers::NONE);
    h.idle_frame();

    // View menu shows "Slide masters" (never "Slide Masters" / "Master View").
    let view_pos = h.find_text("View").expect("View menu");
    h.click_at(view_pos);
    h.idle_frame();
    assert!(
        h.find_text("Slide masters").is_some(),
        "View menu must show 'Slide masters'"
    );
    assert!(
        h.find_text("Slide Masters").is_none(),
        "View menu must not show Title Case 'Slide Masters'"
    );
    assert!(
        h.find_text_containing("Master View").is_none(),
        "View menu must not mention 'Master View'"
    );
    h.press(egui::Key::Escape, egui::Modifiers::NONE);
    h.idle_frame();

    // Edit menu uses "Select all" and "Check spelling…".
    let edit_pos = h.find_text("Edit").expect("Edit menu");
    h.click_at(edit_pos);
    h.idle_frame();
    assert!(
        h.find_text("Select all").is_some(),
        "Edit menu must show 'Select all'"
    );
    assert!(
        h.find_text("Check spelling…").is_some(),
        "Edit menu must show 'Check spelling…'"
    );
    assert!(
        h.find_text("Select All").is_none(),
        "Edit menu must not show Title Case 'Select All'"
    );
    assert!(
        h.find_text("Check Spelling…").is_none(),
        "Edit menu must not show Title Case 'Check Spelling…'"
    );
    h.press(egui::Key::Escape, egui::Modifiers::NONE);
    h.idle_frame();

    // File menu must not have duplicate "Course Properties".
    let file_pos = h.find_text("File").expect("File menu");
    h.click_at(file_pos);
    h.idle_frame();
    assert!(
        h.find_text("Course Properties").is_none(),
        "File menu must not contain duplicate 'Course Properties'"
    );
    assert!(
        h.find_text("Open folder…").is_some(),
        "File menu must show sentence-case 'Open folder…'"
    );
    assert!(
        h.find_text("Save as…").is_some(),
        "File menu must show sentence-case 'Save as…'"
    );
    h.press(egui::Key::Escape, egui::Modifiers::NONE);
    h.idle_frame();

    // Project menu has canonical "Course settings…".
    let project_pos = h.find_text("Project").expect("Project menu");
    h.click_at(project_pos);
    h.idle_frame();
    assert!(
        h.find_text("Course settings…").is_some(),
        "Project menu must show 'Course settings…'"
    );
    assert!(
        h.find_text("AI settings…").is_some(),
        "Project menu must show 'AI settings…'"
    );
    assert!(
        h.find_text("Suggest alt text…").is_some(),
        "Project menu must show 'Suggest alt text…'"
    );
    assert!(
        h.find_text("Save as template").is_some(),
        "Project menu must show 'Save as template'"
    );
    h.press(egui::Key::Escape, egui::Modifiers::NONE);
    h.idle_frame();

    // Entering Slide masters shows "Close slide masters" (never "Close Master View").
    h.editor.enter_master_view();
    h.idle_frame();
    assert!(
        h.find_text("Close slide masters").is_some(),
        "navigator must show 'Close slide masters'"
    );
    assert!(
        h.find_text("Close Master View").is_none(),
        "navigator must not show legacy 'Close Master View'"
    );
}

fn open_file_submenu(h: &mut TestHarness, submenu: &str) {
    let file_pos = h.find_text("File").expect("File menu");
    h.click_at(file_pos);
    h.idle_frame();
    let sub_pos = h
        .find_text_last(submenu)
        .unwrap_or_else(|| panic!("{submenu:?} submenu under File"));
    h.click_at(sub_pos);
    h.idle_frame();
}

/// I-01: `File ▸ Import ▸` lists all 7 import actions in order and each triggers its
/// expected `EditorRequest` or dialog (`M32-08`).
#[test]
fn file_import_menu_lists_all_seven_items_in_order_and_triggers_actions() {
    let mut h =
        TestHarness::new_with_config(sample_course(), (1280.0, 800.0), ThemeMode::Dark, 1.0, true);
    h.idle_frame();

    let expected_items = [
        "PowerPoint (.pptx)…",
        "Course from document…",
        "Slides from outline…",
        "Questions from spreadsheet (CSV)…",
        "Translation (XLIFF)…",
        "Media files…",
        "Font…",
    ];

    open_file_submenu(&mut h, "Import");
    let mut prev_y = f32::NEG_INFINITY;
    for label in expected_items {
        let pos = h
            .find_text_last(label)
            .unwrap_or_else(|| panic!("expected {label:?} in File ▸ Import"));
        assert!(
            pos.y > prev_y,
            "expected {label:?} (y={}) below previous item (y={prev_y})",
            pos.y
        );
        prev_y = pos.y;
    }
    h.press(egui::Key::Escape, egui::Modifiers::NONE);
    h.idle_frame();

    // 1. PowerPoint (.pptx)… -> PickPptx
    open_file_submenu(&mut h, "Import");
    let pos = h
        .find_text_last("PowerPoint (.pptx)…")
        .expect("PowerPoint (.pptx)…");
    let reqs = h.click_at_requests(pos);
    assert!(
        reqs.iter()
            .any(|r| matches!(r, crate::EditorRequest::PickPptx)),
        "expected PickPptx request, got {reqs:?}"
    );

    // 2. Course from document… -> opens document dialog
    open_file_submenu(&mut h, "Import");
    let pos = h
        .find_text_last("Course from document…")
        .expect("Course from document…");
    h.click_at(pos);
    h.idle_frame();
    assert!(
        h.editor.document_dialog_open_for_test(),
        "expected Course from document dialog to open"
    );
    h.editor.ui.document.open = false;
    h.idle_frame();

    // 3. Slides from outline… -> opens outline dialog
    open_file_submenu(&mut h, "Import");
    let pos = h
        .find_text_last("Slides from outline…")
        .expect("Slides from outline…");
    h.click_at(pos);
    h.idle_frame();
    assert!(
        h.editor.outline_dialog_open_for_test(),
        "expected Slides from outline dialog to open"
    );
    h.editor.ui.show_outline_dialog = false;
    h.idle_frame();

    // 4. Questions from spreadsheet (CSV)… -> ImportQuestionsCsv
    open_file_submenu(&mut h, "Import");
    let pos = h
        .find_text_last("Questions from spreadsheet (CSV)…")
        .expect("Questions from spreadsheet (CSV)…");
    let reqs = h.click_at_requests(pos);
    assert!(
        reqs.iter()
            .any(|r| matches!(r, crate::EditorRequest::ImportQuestionsCsv)),
        "expected ImportQuestionsCsv request, got {reqs:?}"
    );

    // 5. Translation (XLIFF)… -> ImportTranslation
    open_file_submenu(&mut h, "Import");
    let pos = h
        .find_text_last("Translation (XLIFF)…")
        .expect("Translation (XLIFF)…");
    let reqs = h.click_at_requests(pos);
    assert!(
        reqs.iter()
            .any(|r| matches!(r, crate::EditorRequest::ImportTranslation)),
        "expected ImportTranslation request, got {reqs:?}"
    );

    // 6. Media files… -> PickMediaFiles
    open_file_submenu(&mut h, "Import");
    let pos = h.find_text_last("Media files…").expect("Media files…");
    let reqs = h.click_at_requests(pos);
    assert!(
        reqs.iter()
            .any(|r| matches!(r, crate::EditorRequest::PickMediaFiles)),
        "expected PickMediaFiles request, got {reqs:?}"
    );

    // 7. Font… -> PickFonts
    open_file_submenu(&mut h, "Import");
    let pos = h.find_text_last("Font…").expect("Font…");
    let reqs = h.click_at_requests(pos);
    assert!(
        reqs.iter()
            .any(|r| matches!(r, crate::EditorRequest::PickFonts)),
        "expected PickFonts request, got {reqs:?}"
    );

    // File ▸ Export ▸ Translation (XLIFF)… -> opens export translation dialog
    open_file_submenu(&mut h, "Export");
    let pos = h
        .find_text_last("Translation (XLIFF)…")
        .expect("Export Translation (XLIFF)…");
    h.click_at(pos);
    h.idle_frame();
    assert!(
        h.editor.ui.show_export_translation,
        "expected Export translation dialog to open"
    );
}

/// I-02: Old scattered import/export duplicates are gone from `File` and `Insert`,
/// and `File ▸ Export ▸` contains `Translation (XLIFF)…` (`M32-08`).
#[test]
fn old_scattered_import_and_export_duplicates_are_removed() {
    let mut h =
        TestHarness::new_with_config(sample_course(), (1280.0, 800.0), ThemeMode::Dark, 1.0, true);
    h.idle_frame();

    // Top-level File has no scattered duplicates and keeps Manage fonts….
    let file_pos = h.find_text("File").expect("File menu");
    h.click_at(file_pos);
    h.idle_frame();
    for removed in [
        "Import PowerPoint…",
        "Import translation…",
        "Export translation…",
        "Add font…",
        "Slides from outline…",
    ] {
        assert!(
            h.find_text(removed).is_none(),
            "top-level File menu must not contain legacy duplicate {removed:?}"
        );
    }
    assert!(
        h.find_text_last("Manage fonts…").is_some(),
        "top-level File menu must keep 'Manage fonts…'"
    );
    h.press(egui::Key::Escape, egui::Modifiers::NONE);
    h.idle_frame();

    // Insert menu has neither Slides from outline… nor Course from document….
    let insert_pos = h.find_text("Insert").expect("Insert menu");
    h.click_at(insert_pos);
    h.idle_frame();
    for removed in ["Slides from outline…", "Course from document…"] {
        assert!(
            h.find_text(removed).is_none(),
            "Insert menu must not contain moved import {removed:?}"
        );
    }
    h.press(egui::Key::Escape, egui::Modifiers::NONE);
    h.idle_frame();

    // File ▸ Export ▸ has Translation (XLIFF)… after Word for review….
    open_file_submenu(&mut h, "Export");
    let word_pos = h
        .find_text_last("Word for review…")
        .expect("Word for review… in File ▸ Export");
    let xliff_pos = h
        .find_text_last("Translation (XLIFF)…")
        .expect("Translation (XLIFF)… in File ▸ Export");
    assert!(
        xliff_pos.y > word_pos.y,
        "expected Translation (XLIFF)… (y={}) after Word for review… (y={})",
        xliff_pos.y,
        word_pos.y
    );
}

/// I-03: In web mode (`file_ops_available = false`), desktop-only Import and Export
/// items are disabled with tooltips while browser-safe imports stay enabled (`M32-08`).
#[test]
fn web_mode_disables_desktop_only_import_and_export_items_with_tooltips() {
    let mut h = TestHarness::new_with_config(
        sample_course(),
        (1280.0, 800.0),
        ThemeMode::Dark,
        1.0,
        false,
    );
    h.idle_frame();

    // Desktop-only items in File ▸ Import are disabled with desktop-only tooltips.
    for label in ["Translation (XLIFF)…", "Media files…"] {
        open_file_submenu(&mut h, "Import");
        let pos = h
            .find_text_last(label)
            .unwrap_or_else(|| panic!("expected disabled {label:?} in File ▸ Import"));
        h.hover_at(pos);
        let tip = h.probe_tooltip(pos);
        assert!(
            tip.as_deref().is_some_and(|t| t.contains("desktop-only")),
            "expected desktop-only tooltip for {label:?}, got {tip:?}"
        );
        let reqs = h.click_at_requests(pos);
        assert!(
            reqs.is_empty(),
            "clicking disabled {label:?} must not emit requests: {reqs:?}"
        );
        h.press(egui::Key::Escape, egui::Modifiers::NONE);
        h.idle_frame();
    }

    // Browser-safe imports stay enabled in web mode.
    open_file_submenu(&mut h, "Import");
    let pptx_pos = h
        .find_text_last("PowerPoint (.pptx)…")
        .expect("PowerPoint (.pptx)…");
    let reqs = h.click_at_requests(pptx_pos);
    assert!(
        reqs.iter()
            .any(|r| matches!(r, crate::EditorRequest::PickPptx)),
        "PowerPoint (.pptx)… must remain enabled in web mode"
    );

    open_file_submenu(&mut h, "Import");
    let doc_pos = h
        .find_text_last("Course from document…")
        .expect("Course from document…");
    h.click_at(doc_pos);
    h.idle_frame();
    assert!(
        h.editor.document_dialog_open_for_test(),
        "Course from document… must remain enabled in web mode"
    );
    h.editor.ui.document.open = false;
    h.idle_frame();

    open_file_submenu(&mut h, "Import");
    let outline_pos = h
        .find_text_last("Slides from outline…")
        .expect("Slides from outline…");
    h.click_at(outline_pos);
    h.idle_frame();
    assert!(
        h.editor.outline_dialog_open_for_test(),
        "Slides from outline… must remain enabled in web mode"
    );
    h.editor.ui.show_outline_dialog = false;
    h.idle_frame();

    open_file_submenu(&mut h, "Import");
    let csv_pos = h
        .find_text_last("Questions from spreadsheet (CSV)…")
        .expect("Questions from spreadsheet (CSV)…");
    let reqs = h.click_at_requests(csv_pos);
    assert!(
        reqs.iter()
            .any(|r| matches!(r, crate::EditorRequest::ImportQuestionsCsv)),
        "Questions from spreadsheet (CSV)… must remain enabled in web mode"
    );

    open_file_submenu(&mut h, "Import");
    let font_pos = h.find_text_last("Font…").expect("Font…");
    let reqs = h.click_at_requests(font_pos);
    assert!(
        reqs.iter()
            .any(|r| matches!(r, crate::EditorRequest::PickFonts)),
        "Font… must remain enabled in web mode"
    );

    // File ▸ Export ▸ Translation (XLIFF)… is disabled with desktop-only tooltip in web mode.
    open_file_submenu(&mut h, "Export");
    let export_xliff_pos = h
        .find_text_last("Translation (XLIFF)…")
        .expect("Translation (XLIFF)… in File ▸ Export");
    h.hover_at(export_xliff_pos);
    let tip = h.probe_tooltip(export_xliff_pos);
    assert!(
        tip.as_deref().is_some_and(|t| t.contains("desktop-only")),
        "expected desktop-only tooltip for Export Translation (XLIFF)…, got {tip:?}"
    );
    h.click_at(export_xliff_pos);
    h.idle_frame();
    assert!(
        !h.editor.ui.show_export_translation,
        "clicking disabled Export Translation (XLIFF)… must not open the dialog"
    );
}
