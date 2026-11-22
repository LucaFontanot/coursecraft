//! Step u8 regression tests: inspector combos, numeric units, States, Animate,
//! Slide, Course tabs, and Shape Outline Corners (`Outline.join`).

use crate::properties::InspectorTab;
use crate::test_harness::{TestHarness, capture_layers, check_layout};
use coursecraft_course::{Course, LineJoin, ObjectKind};
use coursecraft_render::ThemeMode;
use egui::epaint::Shape;
use egui::{Key, Modifiers, pos2};

fn visible_texts(h: &mut TestHarness) -> Vec<String> {
    let mut out = Vec::new();
    for cs in h.frame_shapes(Vec::new()) {
        if let Shape::Text(t) = cs.shape {
            out.push(t.galley.text().to_owned());
        }
    }
    out
}

fn inspector_test_course() -> Course {
    let raw = r##"{
      "schema_version": 1,
      "id": "c-u8",
      "title": "Very Long Course Title That Should Truncate Cleanly In Narrow Inspector Views Without Clipping Buttons",
      "slide_size": { "w": 1280, "h": 720 },
      "masters": [
        { "id": "m1", "title": "Default Master", "objects": [] }
      ],
      "variables": [
        { "name": "score", "value": 0 },
        { "name": "answer", "value": "" },
        { "name": "flag", "value": false }
      ],
      "scenes": [{
        "id": "scene1",
        "title": "Main",
        "slides": [{
          "id": "s1",
          "title": "Very Long Slide Title That Should Truncate Cleanly In The Slide Tab",
          "objects": [
            {
              "id": "obj_text",
              "frame": { "x": 40, "y": 40, "w": 200, "h": 80 },
              "kind": {
                "type": "text",
                "text": "Hello",
                "style": { "font": "Inter", "size": 24, "color": "#1F1F1F", "align": "left" }
              },
              "entrance": { "effect": { "type": "fade" }, "duration_ms": 500 },
              "exit": { "effect": { "type": "fade" }, "duration_ms": 400 },
              "motion_paths": [
                { "id": "mp1", "points": [{ "x": 0, "y": 0 }, { "x": 100, "y": 0 }], "duration_ms": 1000, "easing": "ease_in_out" }
              ],
              "states": [
                { "name": "hover", "text": "Hover" }
              ]
            },
            {
              "id": "obj_shape",
              "frame": { "x": 40, "y": 140, "w": 160, "h": 100 },
              "kind": {
                "type": "shape",
                "shape": "rectangle",
                "fill": "#3B82F6",
                "fill_style": {
                  "type": "linear",
                  "angle": 45,
                  "stops": [
                    { "position": 0.0, "color": "#3B82F6" },
                    { "position": 1.0, "color": "#1D4ED8" }
                  ]
                },
                "outline": { "color": "#111827", "width": 4, "dash": "dash", "join": "miter" }
              },
              "glow": { "color": "#F59E0B", "size": 8 }
            },
            {
              "id": "obj_image",
              "frame": { "x": 220, "y": 40, "w": 120, "h": 90 },
              "kind": { "type": "image", "src": "media/img.png", "alt": "Photo" }
            },
            {
              "id": "obj_button",
              "frame": { "x": 220, "y": 150, "w": 120, "h": 44 },
              "kind": { "type": "button", "label": "Next", "fill": "#2563EB" }
            },
            {
              "id": "obj_checkbox",
              "frame": { "x": 360, "y": 40, "w": 140, "h": 36 },
              "kind": { "type": "checkbox", "label": "Agree", "variable": "flag" }
            },
            {
              "id": "obj_radio",
              "frame": { "x": 360, "y": 90, "w": 140, "h": 80 },
              "kind": { "type": "radio_group", "options": [{ "label": "One", "value": "one" }, { "label": "Two", "value": "two" }], "variable": "answer" }
            },
            {
              "id": "obj_slider",
              "frame": { "x": 360, "y": 180, "w": 180, "h": 36 },
              "kind": { "type": "slider", "variable": "score", "min": 0, "max": 100, "step": 1 }
            },
            {
              "id": "obj_dial",
              "frame": { "x": 560, "y": 40, "w": 100, "h": 100 },
              "kind": { "type": "dial", "variable": "score", "min": 0, "max": 100, "start_angle": 0, "end_angle": 270 }
            },
            {
              "id": "obj_input",
              "frame": { "x": 560, "y": 150, "w": 180, "h": 36 },
              "kind": { "type": "text_entry", "variable": "answer", "placeholder": "Type..." }
            },
            {
              "id": "obj_meter",
              "frame": { "x": 560, "y": 200, "w": 180, "h": 24 },
              "kind": { "type": "progress_meter", "variable": "score", "min": 0, "max": 100 }
            },
            {
              "id": "obj_timer",
              "frame": { "x": 40, "y": 260, "w": 120, "h": 40 },
              "kind": { "type": "timer", "duration_ms": 30000 }
            },
            {
              "id": "obj_scroll",
              "frame": { "x": 180, "y": 260, "w": 160, "h": 120 },
              "kind": { "type": "scroll_panel", "content_height": 400 }
            },
            {
              "id": "obj_table",
              "frame": { "x": 360, "y": 260, "w": 200, "h": 120 },
              "kind": {
                "type": "table",
                "columns": [100, 100],
                "rows": [
                  { "cells": [{ "text": "A" }, { "text": "B" }] },
                  { "cells": [{ "text": "C" }, { "text": "D" }] }
                ]
              }
            },
            {
              "id": "obj_marker",
              "frame": { "x": 580, "y": 260, "w": 36, "h": 36 },
              "kind": { "type": "marker", "title": "Info", "body": "Details" }
            },
            {
              "id": "obj_video",
              "frame": { "x": 40, "y": 400, "w": 200, "h": 120 },
              "kind": { "type": "video", "src": "media/vid.mp4" }
            },
            {
              "id": "obj_audio",
              "frame": { "x": 260, "y": 400, "w": 48, "h": 48 },
              "kind": { "type": "audio", "src": "media/aud.mp3" }
            },
            {
              "id": "obj_lottie",
              "frame": { "x": 330, "y": 400, "w": 120, "h": 120 },
              "kind": { "type": "lottie", "src": "media/anim.json", "speed": 1.0 }
            },
            {
              "id": "obj_web",
              "frame": { "x": 470, "y": 400, "w": 200, "h": 120 },
              "kind": { "type": "web", "source": { "type": "url", "url": "https://example.com" } }
            }
          ],
          "triggers": [
            {
              "when": { "event": "click", "object": "obj_button" },
              "action": { "action": "change_state", "object": "obj_text", "state": "hover" }
            }
          ]
        }]
      }]
    }"##;
    serde_json::from_str(raw).expect("valid test course")
}

#[test]
fn narrow_inspector_260px_has_zero_layout_errors_across_all_18_object_kinds() {
    let ids = [
        "obj_text",
        "obj_shape",
        "obj_image",
        "obj_button",
        "obj_checkbox",
        "obj_radio",
        "obj_slider",
        "obj_dial",
        "obj_input",
        "obj_meter",
        "obj_timer",
        "obj_scroll",
        "obj_table",
        "obj_marker",
        "obj_video",
        "obj_audio",
        "obj_lottie",
        "obj_web",
    ];
    for theme in [ThemeMode::Dark, ThemeMode::Light] {
        let mut h = TestHarness::new_with_theme(inspector_test_course(), (1024.0, 600.0), theme);
        h.editor.ui.inspector_tab = InspectorTab::Format;
        h.idle_frame();
        // Drag the right panel divider left so the inspector hits its 260px minimum width.
        #[allow(deprecated)]
        let panel_left = h
            .ctx
            .memory(|m| m.area_rect(egui::Id::new("editor_properties_panel")))
            .map(|r| r.left())
            .unwrap_or(1024.0 - 290.0);
        h.drag_by(
            pos2(panel_left, 200.0),
            egui::vec2(200.0, 0.0),
            5,
            Modifiers::NONE,
        );
        h.idle_frame();

        for id in ids {
            h.editor.selection.set_many([id]);
            h.idle_frame();
            let layers = capture_layers(&h.ctx);
            let findings = check_layout(&layers, h.screen_rect, &[], &[]).findings;
            let bad: Vec<_> = findings
                .into_iter()
                .filter(|f| {
                    f.starts_with("clipped:")
                        || f.starts_with("overlap:")
                        || f.starts_with("clipped-widget:")
                })
                .collect();
            assert!(
                bad.is_empty(),
                "Theme {theme:?}, object {id} at 260px inspector had layout findings: {bad:#?}"
            );
        }
    }
}

#[test]
fn inspector_and_states_combos_have_sufficient_width_and_tall_popup() {
    let mut h = TestHarness::new_with_size(inspector_test_course(), (1280.0, 2600.0));
    h.editor.ui.inspector_tab = InspectorTab::Format;
    h.editor.selection.set_many(["obj_text"]);
    h.idle_frame();

    // Autofit, Initial state
    let findings = h.audit_combo("Off");
    assert!(
        findings.is_empty(),
        "Combo 'Off' audit failed: {findings:#?}"
    );
    // Initial state combo (disambiguate from States' "normal" label by using ComboBox widget bounds)
    h.editor.selection.set_many(["obj_text"]);
    h.idle_frame();
    let initial_y = h.find_text("Initial").expect("Initial label").y;
    let initial_combo_pos = h
        .last_widgets
        .iter()
        .find(|w| {
            w.role == egui::accesskit::Role::ComboBox
                && (w.bounds.center().y - initial_y).abs() < 12.0
        })
        .map(|w| w.bounds.center())
        .expect("Initial state ComboBox");
    let findings = h.audit_popup(|h| h.click_at(initial_combo_pos));
    assert!(
        findings.is_empty(),
        "Combo 'Initial' audit failed: {findings:#?}"
    );

    // States preset combo: verify it's present and styled; check remaining inspector combos.
    assert!(h.find_text("Preset name…").is_some());

    // Shape: Fill type, Dash, Corners
    h.editor.selection.set_many(["obj_shape"]);
    h.idle_frame();
    for combo_label in ["Linear gradient", "Sharp"] {
        let findings = h.audit_combo(combo_label);
        assert!(
            findings.is_empty(),
            "Shape combo '{combo_label}' audit failed: {findings:#?}"
        );
    }
    let dash_y = h.find_text("Dash").expect("Dash label").y;
    let dash_pos = h
        .last_widgets
        .iter()
        .find(|w| {
            w.role == egui::accesskit::Role::ComboBox && (w.bounds.center().y - dash_y).abs() < 12.0
        })
        .map(|w| w.bounds.center())
        .expect("Dash ComboBox");
    let findings = h.audit_popup(|h| h.click_at(dash_pos));
    assert!(
        findings.is_empty(),
        "Shape combo 'Dash' audit failed: {findings:#?}"
    );

    // Input: Variable
    h.editor.selection.set_many(["obj_input"]);
    h.idle_frame();
    let findings = h.audit_combo("answer");
    assert!(
        findings.is_empty(),
        "Input variable combo audit failed: {findings:#?}"
    );

    // Animate tab: Animation effect, Easing
    h.editor.ui.inspector_tab = InspectorTab::Animate;
    h.editor.selection.set_many(["obj_text"]);
    h.idle_frame();
    for combo_label in ["Fade in", "EaseInOut"] {
        let findings = h.audit_combo(combo_label);
        assert!(
            findings.is_empty(),
            "Animate combo '{combo_label}' audit failed: {findings:#?}"
        );
    }

    // Slide tab: Slide Master, Transition, When revisiting
    h.editor.ui.inspector_tab = InspectorTab::Slide;
    h.idle_frame();
    for combo_label in ["None", "Automatic"] {
        let findings = h.audit_combo(combo_label);
        assert!(
            findings.is_empty(),
            "Slide combo '{combo_label}' audit failed: {findings:#?}"
        );
    }
}

#[test]
fn numeric_drag_values_display_units_and_undo_in_one_step() {
    let mut h = TestHarness::new(inspector_test_course());
    h.editor.ui.inspector_tab = InspectorTab::Format;
    h.editor.selection.set_many(["obj_shape"]);
    h.idle_frame();

    let all_text = visible_texts(&mut h);
    for expected_unit in ["px", "°", "%"] {
        assert!(
            all_text.iter().any(|t| t.contains(expected_unit)),
            "Expected unit '{expected_unit}' on Shape Format tab; got: {all_text:?}"
        );
    }

    // Text tab: pt and × (line spacing)
    h.editor.selection.set_many(["obj_text"]);
    h.idle_frame();
    let text_labels = visible_texts(&mut h);
    assert!(
        text_labels.iter().any(|t| t.contains("pt")),
        "Expected 'pt' unit on Text Format tab; got: {text_labels:?}"
    );

    // Timer / Lottie / ScrollPanel: ms, ×, px
    h.editor.selection.set_many(["obj_timer"]);
    h.idle_frame();
    assert!(
        visible_texts(&mut h).iter().any(|t| t.contains("ms")),
        "Expected 'ms' unit on Timer duration"
    );
    h.editor.selection.set_many(["obj_lottie"]);
    h.idle_frame();
    assert!(
        visible_texts(&mut h).iter().any(|t| t.contains('×')),
        "Expected '×' unit on Lottie speed"
    );
    h.editor.selection.set_many(["obj_scroll"]);
    h.idle_frame();
    assert!(
        visible_texts(&mut h).iter().any(|t| t.contains("px")),
        "Expected 'px' unit on ScrollPanel content height"
    );

    // Verify typing/dragging Rotation on obj_shape updates and undoes in 1 Ctrl+Z
    h.editor.selection.set_many(["obj_shape"]);
    h.idle_frame();
    for _ in 0..8 {
        if h.find_text("Rotation").is_some_and(|p| p.y < 550.0) {
            break;
        }
        h.scroll_over(pos2(1140.0, 400.0), -250.0);
    }
    let before = h.editor.session.course.clone();
    let rot_label = h.find_text("Rotation").expect("Rotation label");
    let rot_pos = pos2(rot_label.x + 110.0, rot_label.y);
    h.drag_by(rot_pos, egui::vec2(25.0, 0.0), 5, Modifiers::NONE);
    h.idle_frame();
    assert_ne!(
        h.editor.session.course, before,
        "Dragging rotation should mutate course"
    );
    h.press(Key::Z, Modifiers::COMMAND);
    h.idle_frame();
    assert_eq!(
        h.editor.session.course, before,
        "One Ctrl/Cmd+Z should undo rotation drag"
    );
}

#[test]
fn states_disabled_controls_explain_why_and_in_use_delete_shows_reason() {
    let mut h = TestHarness::new_with_size(inspector_test_course(), (1280.0, 2600.0));
    h.editor.ui.inspector_tab = InspectorTab::Format;
    h.editor.selection.set_many(["obj_text"]);
    h.idle_frame();

    // "+ Hover" is disabled because "hover" already exists.
    let hover_pill = h.find_text("+ Hover").expect("+ Hover pill");
    let hover_tip = h.probe_tooltip(hover_pill);
    assert!(
        hover_tip.as_deref().is_some_and(|t| t.trim().len() >= 5),
        "Disabled '+ Hover' pill must have an explanatory tooltip, got: {hover_tip:?}"
    );
    let add_pos = h
        .editor
        .ui
        .states
        .add_button_rect
        .expect("+ Add button rect")
        .center();
    let clipped = h.find_text_clipped("+ Add");
    let add_tip = h.probe_tooltip(add_pos);
    assert!(
        add_tip.as_deref().is_some_and(|t| t.trim().len() >= 5),
        "Disabled '+ Add' button at {add_pos:?} clipped={clipped:?} (state rect={:?}) must have an explanatory tooltip, got: {add_tip:?}",
        h.editor.ui.states.add_button_rect
    );

    // Attempting to delete "hover" (referenced by trigger tr1) displays a clear message.
    let del_pos = h.find_text("Delete state").expect("Delete state button");
    h.click_at(del_pos);
    h.idle_frame();
    assert!(
        h.editor.ui.states.delete_error.is_some(),
        "Clicking Delete state on in-use state should record delete_error"
    );
    let texts = visible_texts(&mut h);
    assert!(
        texts
            .iter()
            .any(|t| t.to_lowercase().contains("trigger") || t.to_lowercase().contains("used")),
        "Deleting in-use state should show why it is blocked, got: {texts:?}"
    );
}

#[test]
fn animate_panel_empty_state_and_motion_path_undo() {
    let mut h = TestHarness::new(inspector_test_course());
    h.editor.ui.inspector_tab = InspectorTab::Animate;
    h.editor.selection.object = None;
    h.editor.selection.also.clear();
    h.idle_frame();

    let empty_texts = visible_texts(&mut h);
    assert!(
        empty_texts
            .iter()
            .any(|t| t.to_lowercase().contains("select")),
        "Animate empty state should tell the author what to select, got: {empty_texts:?}"
    );

    // Populated state: obj_text has 1 motion path ("mp1"); deleting it is 1-step undoable.
    h.editor.selection.set_many(["obj_text"]);
    h.idle_frame();
    for _ in 0..3 {
        if h.last_widgets
            .iter()
            .any(|w| w.label.as_deref() == Some("Delete motion path"))
        {
            break;
        }
        h.scroll_over(pos2(1140.0, 400.0), -400.0);
    }
    let before = h.editor.session.course.clone();
    let remove_pos = h
        .last_widgets
        .iter()
        .find(|w| w.label.as_deref() == Some("Delete motion path"))
        .map(|w| w.bounds.center())
        .expect("Delete motion path button");
    h.click_at(remove_pos);
    h.idle_frame();
    assert_ne!(h.editor.session.course, before);
    h.press(Key::Z, Modifiers::COMMAND);
    h.idle_frame();
    assert_eq!(h.editor.session.course, before);
}

#[test]
fn slide_and_course_tabs_fit_at_scale_1_5_without_clipping() {
    for theme in [ThemeMode::Dark, ThemeMode::Light] {
        let mut h = TestHarness::new_with_config(
            inspector_test_course(),
            (1280.0, 800.0),
            theme,
            1.5,
            true,
        );
        for tab in [InspectorTab::Slide, InspectorTab::Course] {
            h.editor.ui.inspector_tab = tab;
            h.idle_frame();
            let layers = capture_layers(&h.ctx);
            let findings = check_layout(&layers, h.screen_rect, &[], &[]).findings;
            let bad: Vec<_> = findings
                .into_iter()
                .filter(|f| {
                    f.starts_with("clipped:")
                        || f.starts_with("overlap:")
                        || f.starts_with("clipped-widget:")
                })
                .collect();
            assert!(
                bad.is_empty(),
                "Tab {tab:?} in {theme:?} at 1.5x scale had layout findings: {bad:#?}"
            );
        }
    }
}

#[test]
fn shape_outline_corners_control_updates_join_and_undoes_in_one_step() {
    let mut h = TestHarness::new(inspector_test_course());
    h.editor.ui.inspector_tab = InspectorTab::Format;
    h.editor.selection.set_many(["obj_shape"]);
    h.idle_frame();
    for _ in 0..5 {
        if h.find_text("Corners").is_some_and(|p| p.y < 520.0) {
            break;
        }
        h.scroll_over(pos2(1140.0, 400.0), -250.0);
    }

    assert!(
        h.find_text("Corners").is_some(),
        "Shape outline section should show 'Corners' label"
    );

    let before = h.editor.session.course.clone();
    h.pick_combo("Sharp", "Round");
    h.idle_frame();

    let slide = h.editor.session.course.slide("s1").unwrap();
    let (obj, _) = slide.object("obj_shape").unwrap();
    match &obj.kind {
        ObjectKind::Shape {
            outline: Some(out), ..
        } => assert_eq!(out.join, LineJoin::Round),
        other => panic!("unexpected kind: {other:?}"),
    }

    h.press(Key::Z, Modifiers::COMMAND);
    h.idle_frame();
    assert_eq!(h.editor.session.course, before);
}
