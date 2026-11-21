//! Headless UI harness tests for the Format panel's Font picker (Step f20).

use std::time::Instant;

use crate::SystemFontChoice;
use crate::properties::InspectorTab;
use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use egui::{Key, Modifiers, Popup, Rect, epaint::Shape};

fn course_with_text(font: &str, extra_fonts: &[&str]) -> Course {
    let fonts_json: Vec<serde_json::Value> = extra_fonts
        .iter()
        .map(|f| {
            serde_json::json!({
                "family": f,
                "style": "regular",
                "src": format!("fonts/{}.ttf", f.to_lowercase().replace(' ', "_"))
            })
        })
        .collect();
    let json = serde_json::json!({
        "schema_version": 1,
        "id": "font-picker-test",
        "title": "Font Picker Test",
        "slide_size": { "w": 1280, "h": 720 },
        "fonts": fonts_json,
        "scenes": [{
            "id": "s1",
            "title": "Scene",
            "slides": [{
                "id": "slide1",
                "title": "Slide",
                "objects": [{
                    "id": "t1",
                    "name": "Text 1",
                    "kind": {
                        "type": "text",
                        "text": "Hello",
                        "style": { "font": font, "size": 24, "color": "#000000" }
                    },
                    "frame": { "x": 40, "y": 40, "w": 400, "h": 100 }
                }]
            }]
        }]
    });
    Course::from_json(&json.to_string()).expect("valid course")
}

fn open_font_picker(harness: &mut TestHarness, current_label: &str) {
    harness.editor.selection.slide = "slide1".into();
    harness.editor.selection.object = Some("t1".into());
    harness.editor.ui.inspector_tab = InspectorTab::Format;
    harness.idle_frame();
    harness.idle_frame();
    let panel = harness.right_panel_rect();
    let combo = harness
        .find_text_in(current_label, panel)
        .expect("font picker button in Format panel");
    harness.click_at(combo);
    harness.idle_frame();
}

fn walk_rects(shape: &Shape, out: &mut Vec<Rect>) {
    match shape {
        Shape::Rect(r) => out.push(r.rect),
        Shape::Vec(v) => {
            for s in v {
                walk_rects(s, out);
            }
        }
        _ => {}
    }
}

/// Inspect the popup containing `"Add font…"`.
/// Returns `(popup_rect, distinct_scroll_clip_rects, scrollbar_rects)`.
fn inspect_popup(harness: &mut TestHarness) -> (Rect, Vec<Rect>, Vec<Rect>) {
    let shapes = harness.frame_shapes(vec![]);
    let add_font_pos = harness
        .find_text("Add font…")
        .expect("Add font… button must be visible in open popup");

    // Find the topmost popup background rect containing "Add font…".
    let mut popup_rect = Rect::NOTHING;
    for cs in &shapes {
        let mut rects = Vec::new();
        walk_rects(&cs.shape, &mut rects);
        for r in rects {
            if r.contains(add_font_pos)
                && r.width() < 600.0
                && r.height() < 650.0
                && r.width() >= 200.0
                && r.height() >= 150.0
            {
                popup_rect = r;
            }
        }
    }

    let mut scroll_clips: Vec<Rect> = Vec::new();
    let mut scrollbar_rects: Vec<Rect> = Vec::new();
    for cs in &shapes {
        let shape_bounds = cs.shape.visual_bounding_rect();
        if cs.clip_rect.top() >= popup_rect.top() - 8.0
            && cs.clip_rect.bottom() <= popup_rect.bottom() + 8.0
            && cs.clip_rect.height() < popup_rect.height() - 20.0
            && cs.clip_rect.height() > 50.0
            && shape_bounds.is_positive()
            && popup_rect.contains(shape_bounds.center())
            && !scroll_clips.contains(&cs.clip_rect)
        {
            scroll_clips.push(cs.clip_rect);
        }
        let mut rects = Vec::new();
        walk_rects(&cs.shape, &mut rects);
        for r in rects {
            if popup_rect.contains(r.center())
                && r.bottom() <= add_font_pos.y
                && r.width() >= 2.0
                && r.width() <= 18.0
                && r.height() > 20.0
            {
                scrollbar_rects.push(r);
            }
        }
    }

    (popup_rect, scroll_clips, scrollbar_rects)
}

#[test]
fn font_picker_has_one_scroll_area_and_large_fixed_popup() {
    let mut harness = TestHarness::new(course_with_text("Inter", &["Lato"]));
    harness.editor.ui.system_fonts = Some(
        (0..40)
            .map(|i| SystemFontChoice {
                name: format!("SysFont {i:02}"),
                embedding_restricted: false,
            })
            .collect(),
    );
    open_font_picker(&mut harness, "Inter");

    let (popup_rect, scroll_clips, _) = inspect_popup(&mut harness);
    assert!(
        popup_rect.width() >= 370.0 && popup_rect.width() <= 410.0,
        "popup width should be ~380 px, got {}",
        popup_rect.width()
    );
    assert!(
        popup_rect.height() >= 350.0 && popup_rect.height() <= 540.0,
        "popup height should be up to 520 px, got {}",
        popup_rect.height()
    );
    let in_course = harness.find_text_clipped("In this course");
    assert_eq!(
        scroll_clips.len(),
        1,
        "expected exactly 1 ScrollArea in font picker popup (popup_rect={popup_rect:?}, in_course={in_course:?}), got {}: {scroll_clips:?}",
        scroll_clips.len()
    );
    assert!(
        harness.find_text("In this course").is_some(),
        "expected 'In this course' section header"
    );

    // Clamping on a short window (height < 520).
    let mut small =
        TestHarness::new_with_size(course_with_text("Inter", &["Lato"]), (1100.0, 480.0));
    small.editor.ui.system_fonts = Some(
        (0..40)
            .map(|i| SystemFontChoice {
                name: format!("SysFont {i:02}"),
                embedding_restricted: false,
            })
            .collect(),
    );
    small.editor.selection.slide = "slide1".into();
    small.editor.selection.object = Some("t1".into());
    small.editor.ui.inspector_tab = InspectorTab::Format;
    small.idle_frame();
    small.idle_frame();
    small.scroll_over(small.right_panel_rect().center(), -150.0);
    small.idle_frame();
    let combo = small
        .find_text_in("Inter", small.right_panel_rect())
        .expect("font picker button in Format panel");
    small.click_at(combo);
    small.idle_frame();
    let (small_rect, _, _) = inspect_popup(&mut small);
    assert!(
        small_rect.width() <= 410.0 && small_rect.height() <= 480.0,
        "popup must clamp to short window, got {small_rect:?}"
    );
}

#[test]
fn font_picker_popup_width_and_scrollbar_stay_constant_with_short_and_long_names() {
    let mut choices: Vec<SystemFontChoice> = (0..50)
        .map(|i| SystemFontChoice {
            name: format!("A{i:02}"),
            embedding_restricted: false,
        })
        .collect();
    choices.extend((0..50).map(|i| SystemFontChoice {
        name: format!(
            "Z Very Long Font Family Name Number {i:02} That Should Not Resize Popup Or Scrollbar"
        ),
        embedding_restricted: false,
    }));

    let mut harness = TestHarness::new(course_with_text("Inter", &[]));
    harness.editor.ui.system_fonts = Some(choices);
    open_font_picker(&mut harness, "Inter");

    let (top_popup, top_clips, top_bars) = inspect_popup(&mut harness);
    assert_eq!(top_clips.len(), 1);
    assert!(!top_bars.is_empty(), "scrollbar should be painted");

    // Scroll down to the long font names (scroll inside the popup's ScrollArea).
    let scroll_pos = egui::pos2(top_popup.center().x, top_clips[0].center().y);
    for _ in 0..8 {
        harness.scroll_over(scroll_pos, -300.0);
    }
    let (bot_popup, bot_clips, bot_bars) = inspect_popup(&mut harness);
    assert_eq!(bot_clips.len(), 1);
    assert!(!bot_bars.is_empty(), "scrollbar should still be painted");

    assert!(
        (top_popup.width() - bot_popup.width()).abs() < 1.0,
        "popup width changed: {} vs {}",
        top_popup.width(),
        bot_popup.width()
    );
    assert!(
        (top_clips[0].height() - bot_clips[0].height()).abs() < 1.0,
        "scroll clip height changed: {} vs {}",
        top_clips[0].height(),
        bot_clips[0].height()
    );
    let top_bar = top_bars.last().expect("top scrollbar");
    let bot_bar = bot_bars.last().expect("bottom scrollbar");
    assert!(
        (top_bar.left() - bot_bar.left()).abs() < 1.0
            && (top_bar.height() - bot_bar.height()).abs() < 1.0,
        "scrollbar x or height jumped: {top_bar:?} vs {bot_bar:?}"
    );
}

#[test]
fn font_picker_search_filters_across_sections() {
    let mut harness = TestHarness::new(course_with_text("Inter", &["Lato"]));
    harness.editor.ui.properties.recent_fonts = vec!["Roboto".into()];
    harness.editor.ui.system_fonts = Some(vec![
        SystemFontChoice {
            name: "Georgia".into(),
            embedding_restricted: false,
        },
        SystemFontChoice {
            name: "Arial".into(),
            embedding_restricted: false,
        },
    ]);
    open_font_picker(&mut harness, "Inter");

    assert!(harness.find_text("In this course").is_some());
    assert!(harness.find_text("Recent").is_some());
    assert!(harness.find_text("On this computer").is_some());

    // Search field is focused on open; typing filters all three sections.
    harness.type_text("o");
    harness.idle_frame();
    assert!(harness.find_text("Lato").is_some());
    assert!(harness.find_text("Roboto").is_some());
    assert!(harness.find_text("Georgia").is_some());
    assert!(harness.find_text("Arial").is_none());

    // No matches.
    harness.type_text("zzz");
    harness.idle_frame();
    assert!(harness.find_text("No matching fonts").is_some());
    assert!(
        harness.find_text("Add font…").is_some(),
        "Add font… stays pinned"
    );
}

#[test]
fn font_picker_1200_fonts_render_quickly() {
    let mut harness = TestHarness::new(course_with_text("Inter", &[]));
    harness.editor.ui.system_fonts = Some(
        (0..1200)
            .map(|i| SystemFontChoice {
                name: format!("Font Family {i:04}"),
                embedding_restricted: i % 10 == 0,
            })
            .collect(),
    );

    let start = Instant::now();
    open_font_picker(&mut harness, "Inter");
    for _ in 0..4 {
        harness.idle_frame();
    }
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_secs_f32() < 2.0,
        "1200 fonts took too long to render: {elapsed:?}"
    );
    // Virtualised: far-down font should not be painted while at top.
    assert!(harness.find_text("Font Family 1199").is_none());
}

#[test]
fn font_picker_keyboard_navigation_and_selection() {
    let mut harness = TestHarness::new(course_with_text("Inter", &[]));
    harness.editor.ui.system_fonts = Some(vec![
        SystemFontChoice {
            name: "Arial".into(),
            embedding_restricted: false,
        },
        SystemFontChoice {
            name: "Georgia".into(),
            embedding_restricted: false,
        },
    ]);
    open_font_picker(&mut harness, "Inter");

    // Esc closes without changing font.
    harness.press(Key::Escape, Modifiers::NONE);
    harness.idle_frame();
    assert!(!Popup::is_any_open(&harness.ctx));

    // Reopen, type "Geor", press Enter -> picks Georgia.
    open_font_picker(&mut harness, "Inter");
    harness.type_text("Geor");
    harness.idle_frame();
    harness.press(Key::Enter, Modifiers::NONE);
    let reqs = harness.idle_frame();
    // Note: press(Key::Enter) already ran one frame; check via frame or recent_fonts.
    let _ = reqs;
    assert!(!Popup::is_any_open(&harness.ctx));
    assert!(
        harness
            .editor
            .ui
            .properties
            .recent_fonts
            .iter()
            .any(|f| f == "Georgia"),
        "Enter should have selected Georgia into recent_fonts: {:?}",
        harness.editor.ui.properties.recent_fonts
    );
}

#[test]
fn font_picker_current_font_scrolled_into_view() {
    let extra: Vec<String> = (0..100).map(|i| format!("CourseFont {i:03}")).collect();
    let extra_refs: Vec<&str> = extra.iter().map(String::as_str).collect();
    let mut harness = TestHarness::new(course_with_text("CourseFont 080", &extra_refs));
    harness.editor.ui.system_fonts = Some(vec![]);

    open_font_picker(&mut harness, "CourseFont 080");
    let (_, scroll_clips, _) = inspect_popup(&mut harness);
    assert_eq!(scroll_clips.len(), 1);
    let scroll_clip = scroll_clips[0];

    let matches = harness.find_texts("CourseFont 080");
    assert!(
        matches
            .iter()
            .any(|pos| scroll_clip.top() <= pos.y && pos.y <= scroll_clip.bottom()),
        "current font CourseFont 080 should be scrolled into view ({scroll_clip:?}), found at {matches:?}"
    );
}

#[test]
fn font_picker_restricted_font_keeps_marker() {
    let mut harness = TestHarness::new(course_with_text("Inter", &[]));
    harness.editor.ui.system_fonts = Some(vec![SystemFontChoice {
        name: "Georgia".into(),
        embedding_restricted: true,
    }]);
    open_font_picker(&mut harness, "Inter");
    assert!(harness.find_text("restricted").is_some());
}
