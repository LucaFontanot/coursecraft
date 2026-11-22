//! M29 Step s2a tests: Bottom `States` tab, live thumbnails, compact Format row,
//! right-click `Edit states…`, `preview_state` theme/placeholder fix, and `PERF-01`.

use crate::BottomTab;
use crate::guard_counters as gc;
use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use egui::pos2;

fn course_with_states(n_custom: usize) -> Course {
    let mut states = Vec::with_capacity(n_custom);
    let preset = ["hover", "selected", "visited"];
    for i in 0..n_custom {
        let name = if i < preset.len() {
            preset[i].to_owned()
        } else {
            format!("state_{i}")
        };
        states.push(serde_json::json!({
            "name": name,
            "fill": format!("#{:02X}4488", (i * 23) % 255),
            "text": format!("Label {name}")
        }));
    }
    let raw = serde_json::json!({
        "schema_version": 1,
        "id": "c-m29-s2a",
        "title": "M29 S2a Test",
        "slide_size": { "w": 1280, "h": 720 },
        "theme": {
            "accent": "#1166CC",
            "text": "#222222",
            "background": "#F5F5F5",
            "font": "Inter"
        },
        "masters": [{
            "id": "m1",
            "title": "Master 1",
            "objects": [{
                "id": "m_rect",
                "name": "Master Rect",
                "frame": { "x": 40.0, "y": 40.0, "w": 120.0, "h": 60.0 },
                "kind": { "type": "shape", "shape": "rectangle", "fill": "#CCCCCC" }
            }]
        }],
        "scenes": [{
            "id": "scene1",
            "title": "Main",
            "slides": [{
                "id": "s1",
                "title": "Slide 1",
                "objects": [
                    {
                        "id": "btn1",
                        "name": "Primary Button",
                        "frame": { "x": 100.0, "y": 100.0, "w": 220.0, "h": 64.0 },
                        "kind": { "type": "button", "label": "Click Me", "fill": "#336699" },
                        "states": states
                    },
                    {
                        "id": "rect2",
                        "name": "Second Shape",
                        "frame": { "x": 400.0, "y": 100.0, "w": 140.0, "h": 80.0 },
                        "kind": { "type": "shape", "shape": "rectangle", "fill": "#AABBCC" }
                    }
                ]
            }]
        }]
    });
    serde_json::from_value(raw).expect("valid course")
}

#[test]
fn u01_states_tab_visible_on_normal_slide_and_hidden_in_master_view() {
    let mut h = TestHarness::new(course_with_states(2));
    h.idle_frame();

    let states_tab_pos = h
        .find_text("States")
        .expect("States tab must appear in the bottom tab strip on normal slides");
    h.click_at(states_tab_pos);
    h.idle_frame();
    assert_eq!(h.editor.ui.bottom_tab, BottomTab::States);

    h.editor.enter_master_view();
    h.idle_frame();
    assert_ne!(
        h.editor.ui.bottom_tab,
        BottomTab::States,
        "entering Master view must reset bottom_tab away from States"
    );
    assert_eq!(h.editor.ui.bottom_tab, BottomTab::Timeline);
    // The bottom tab strip should not offer "States" in Master view.
    assert!(
        h.find_text("Triggers").is_none(),
        "Triggers is hidden in Master view"
    );
    assert!(
        h.find_text("States").is_none(),
        "States tab must be hidden in Master view"
    );
}

#[test]
fn u02_states_tab_empty_and_multi_select_states() {
    let mut h = TestHarness::new(course_with_states(2));
    h.editor.selection.object = None;
    h.editor.ui.bottom_tab = BottomTab::States;
    h.idle_frame();

    assert!(
        h.find_text("Select an object on the slide to view and edit its states.")
            .is_some(),
        "empty selection should show explanatory empty state"
    );

    h.editor.selection.set_many(["btn1", "rect2"]);
    h.idle_frame();
    assert!(
        h.find_text("Select a single object to edit its states (2 selected).")
            .is_some(),
        "multi-select should show multi-select message"
    );
    assert!(
        h.find_text("+ Hover").is_none(),
        "quick-add chips should not be active in multi-select"
    );
}

#[test]
fn u03_live_side_by_side_thumbnails_and_culling() {
    let mut h = TestHarness::new(course_with_states(3));
    h.editor.selection.object = Some("btn1".to_owned());
    h.editor.ui.bottom_tab = BottomTab::States;
    h.idle_frame();

    for expected in ["Normal", "hover", "selected", "visited"] {
        assert!(
            h.find_text(expected).is_some(),
            "expected state card '{expected}' in States tab"
        );
    }

    // Off-screen culling check with 10 custom states (11 cards total).
    let mut h_many = TestHarness::new(course_with_states(10));
    h_many.editor.selection.object = Some("btn1".to_owned());
    h_many.editor.ui.bottom_tab = BottomTab::States;
    gc::reset();
    gc::reset_thumb_signatures();
    h_many.idle_frame();
    let drawn = gc::state_thumbnail_redraws();
    assert!(drawn > 0, "visible state thumbnails must be painted");
    assert!(
        drawn < 11,
        "off-screen state thumbnails must be culled (painted {drawn} of 11)"
    );
}

#[test]
fn u06_compact_format_row_and_context_menu_edit_states() {
    let mut h = TestHarness::new(course_with_states(2));
    h.editor.selection.object = Some("btn1".to_owned());
    h.editor.ui.inspector_tab = crate::InspectorTab::Format;
    h.idle_frame();

    // Scroll Format inspector to the States section.
    let mut open_btn = None;
    for _ in 0..15 {
        if let Some(pos) = h.find_text("Open States tab (2)") {
            open_btn = Some(pos);
            break;
        }
        h.scroll_over(pos2(1140.0, 350.0), -180.0);
        h.idle_frame();
    }
    let open_pos = open_btn.expect("compact Format row should have 'Open States tab (2)'");
    // Must not have the old duplicate "State" section header.
    assert!(
        h.find_text("State").is_none(),
        "duplicate 'State' section header must be removed"
    );

    h.click_at(open_pos);
    h.idle_frame();
    assert_eq!(h.editor.ui.bottom_tab, BottomTab::States);

    // Switch back to Timeline and test right-click -> "Edit states…"
    h.editor.ui.bottom_tab = BottomTab::Timeline;
    h.idle_frame();
    let row_pos = h
        .find_text_last("Primary Button")
        .expect("Primary Button row in timeline");
    h.right_click_at(row_pos);
    h.idle_frame();
    let edit_states_pos = h
        .find_text("Edit states…")
        .expect("context menu must include 'Edit states…'");
    h.click_at(edit_states_pos);
    h.idle_frame();
    assert_eq!(h.editor.ui.bottom_tab, BottomTab::States);
}

#[test]
fn perf01_states_tab_idle_frames_and_scroll_culling() {
    let mut h = TestHarness::new(course_with_states(10));
    h.editor.selection.object = Some("btn1".to_owned());
    h.editor.ui.bottom_tab = BottomTab::States;
    h.idle_frame();
    h.idle_frame();

    gc::reset();
    for _ in 0..5 {
        h.idle_frame();
    }
    assert_eq!(
        gc::course_copies(),
        0,
        "idle frames in States tab must record 0 course_copies"
    );
    assert_eq!(
        gc::state_thumbnail_redraws(),
        0,
        "idle frames in States tab must record 0 redundant state-thumbnail repaints"
    );
}

#[test]
fn sus03_canvas_preview_state_uses_course_theme() {
    let mut course = course_with_states(1);
    course.theme.font = "StudioFont".to_owned();
    course.theme.text = coursecraft_course::Color("#123456".to_owned());
    let txt: coursecraft_course::Object = serde_json::from_value(serde_json::json!({
        "id": "txt1",
        "name": "Text 1",
        "frame": { "x": 100.0, "y": 200.0, "w": 200.0, "h": 40.0 },
        "kind": { "type": "text", "text": "Normal" },
        "states": [{ "name": "hover", "text": "Hovered" }]
    }))
    .expect("valid text object");
    course.slide_mut("s1").expect("slide").objects.push(txt);
    let slide = course.slide("s1").expect("slide");
    let (obj, _) = slide.object("txt1").expect("txt1");
    let item = coursecraft_render::compose_object_with_value(
        obj,
        "hover",
        Some(&course.theme),
        crate::states::course_input_value(&course, obj),
    )
    .expect("composed item");
    match item.visual {
        coursecraft_render::Visual::Text { runs, .. } => {
            assert_eq!(runs[0].text, "Hovered");
            assert_eq!(runs[0].font.as_deref(), Some("StudioFont"));
        }
        other => panic!("expected Text visual, got {other:?}"),
    }
}
