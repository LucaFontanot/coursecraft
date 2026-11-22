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

            let left_bounds: Vec<_> = [
                "Add slide to course",
                "More slide options",
                "Preview current slide",
                "More preview options",
                "Publish course",
                "Course map",
                "Format Painter",
                "Select an object to copy its format",
            ]
            .iter()
            .filter_map(|needle| {
                h.last_widgets
                    .iter()
                    .find(|w| w.label.as_deref().is_some_and(|l| l.contains(needle)))
                    .map(|w| (needle, w.bounds))
            })
            .collect();
            let center_bounds: Vec<_> = [
                "Insert Text Box",
                "Insert Shape",
                "Insert Image",
                "Insert Media",
                "Insert Button",
                "Insert Question",
                "Insert Trigger",
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
                "Format Inspector",
                "Animate Inspector",
                "Slide Inspector",
                "Course Inspector",
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
            h.find_text("Add slide").is_none(),
            "expected 'Add slide' label to collapse to icon-only at {size:?} scale {scale}"
        );

        for needle in [
            "Add slide to course",
            "Preview current slide",
            "Publish course",
            "Course map",
            "Format Painter",
            "Insert Text Box",
            "Insert Shape",
            "Insert Image",
            "Insert Media",
            "Insert Button",
            "Insert Question",
            "Insert Trigger",
        ] {
            let w = widget_with_label(&h, needle);
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
        "Add slide",
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
            Some("Not available in Master View"),
            "expected 'Not available in Master View' tooltip on {label:?}, got {tip:?}"
        );
    }

    let map_widget = widget_with_label(&h, "Not available in Master View");
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
        ("Insert Shape", "Rectangle"),
        ("Insert Media", "Video…"),
        ("Insert Question", "Multiple Choice"),
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

    for item in ["Group", "Ungroup", "Align Left", "Bring forward"] {
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
