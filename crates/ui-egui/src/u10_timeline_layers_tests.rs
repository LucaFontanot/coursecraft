//! Step u10 regression tests: Timeline, Slide Layers pane & Layer Settings dialog (`Area 6`).

use crate::test_harness::TestHarness;
use crate::timeline::TRACK_HEADER_WIDTH;
use coursecraft_course::Course;
use coursecraft_render::ThemeMode;
use egui::{Key, Modifiers, pos2, vec2};

fn timeline_layers_course() -> Course {
    let long_obj = "Extraordinarily Long Object Name That Exceeds The Timeline Left Header Column Width 1234567890 ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let long_layer =
        "Extraordinarily Long Layer Name That Exceeds Narrow 200px Layers Pane Width 1234567890";
    let raw = serde_json::json!({
        "schema_version": 1,
        "id": "c-u10",
        "title": "Step u10 Course",
        "slide_size": { "w": 1280, "h": 720 },
        "masters": [{ "id": "m1", "title": "Default Master", "objects": [] }],
        "scenes": [{
            "id": "scene1",
            "title": "Main",
            "slides": [{
                "id": "s1",
                "title": "Slide 1",
                "duration_ms": 60000,
                "cue_points": [
                    { "id": "cue1", "name": "Cue One", "at_ms": 5000 }
                ],
                "objects": [
                    {
                        "id": "obj1",
                        "name": long_obj,
                        "frame": { "x": 100, "y": 100, "w": 240, "h": 120 },
                        "timing": { "start_ms": 0, "end_ms": 3000 },
                        "kind": {
                            "type": "shape",
                            "shape": "rectangle",
                            "fill": "#2563EB"
                        }
                    },
                    {
                        "id": "obj2",
                        "name": "Second Object",
                        "frame": { "x": 400, "y": 120, "w": 200, "h": 100 },
                        "timing": { "start_ms": 1000, "end_ms": 4000 },
                        "kind": {
                            "type": "shape",
                            "shape": "rectangle",
                            "fill": "#10B981"
                        }
                    }
                ],
                "layers": [
                    {
                        "id": "layer1",
                        "name": long_layer,
                        "duration_ms": 5000,
                        "objects": []
                    }
                ]
            }]
        }]
    });
    serde_json::from_value(raw).expect("valid course json")
}

#[test]
fn t6_1_bottom_panel_resize_bounds_preserve_200px_canvas_and_no_tab_overlap() {
    for (w, h_px) in [(1024.0, 600.0), (1440.0, 900.0)] {
        let mut h = TestHarness::new_with_size(timeline_layers_course(), (w, h_px));
        h.idle_frame();
        h.idle_frame();

        let top_y = h.bottom_panel_top();
        let panel_rect = h
            .editor
            .ui
            .timeline
            .panel_rect
            .expect("timeline panel rect");
        let drag_start = pos2(panel_rect.center().x, top_y);

        // Drag up to maximum.
        h.drag_by(drag_start, vec2(0.0, -500.0), 6, Modifiers::NONE);
        h.idle_frame();
        let canvas_after_max = h.central_panel_rect();
        assert!(
            canvas_after_max.height() >= 200.0 - 1.0,
            "at {w}x{h_px} max timeline drag left canvas height {} < 200",
            canvas_after_max.height()
        );

        // Grab zone (8 px from top of bottom panel) must not overlap tab buttons.
        let panel_top = h.bottom_panel_top();
        let grab_zone_bottom = panel_top + crate::timeline::PANEL_RESIZE_GRAB_PX - 2.0;
        for (tab_name, tab_rect) in &h.editor.ui.timeline.tab_rects {
            assert!(
                tab_rect.min.y >= grab_zone_bottom - 1.0,
                "at {w}x{h_px} tab '{tab_name}' min.y ({}) overlaps resize grab zone (< {grab_zone_bottom})",
                tab_rect.min.y
            );
        }

        // Drag down to minimum (140 px).
        let cur_top = h.bottom_panel_top();
        h.drag_by(
            pos2(panel_rect.center().x, cur_top),
            vec2(0.0, 500.0),
            6,
            Modifiers::NONE,
        );
        h.idle_frame();
        let min_rect = h
            .editor
            .ui
            .timeline
            .panel_rect
            .expect("timeline panel rect");
        assert!(
            min_rect.height() >= crate::timeline::TIMELINE_PANEL_MIN_HEIGHT - 1.0,
            "at {w}x{h_px} min timeline height {} < {}",
            min_rect.height(),
            crate::timeline::TIMELINE_PANEL_MIN_HEIGHT
        );
    }
}

#[test]
fn t6_2_timeline_and_layers_icon_buttons_meet_hit_targets_and_tooltips() {
    let mut h = TestHarness::new_with_size(timeline_layers_course(), (1280.0, 800.0));
    h.idle_frame();
    h.idle_frame();

    // Check Play button.
    let play_rect = h.editor.ui.timeline.play_button_rect.expect("play button");
    assert!(
        play_rect.width() >= 24.0 && play_rect.height() >= 24.0,
        "play button rect {:?} < 24x24",
        play_rect.size()
    );
    assert!(
        h.probe_tooltip(play_rect.center()).is_some(),
        "play button must have a tooltip"
    );

    // Check Timeline object Eye and Lock buttons (must be >= 24x24 pt and have tooltips).
    let row_rect = h
        .editor
        .ui
        .timeline
        .object_row_rects
        .get("obj1")
        .copied()
        .expect("obj1 row rect");
    let header_max_x = row_rect.min.x + TRACK_HEADER_WIDTH;
    // Lock is at the right end of header_row_rect, Eye is just left of Lock.
    let hit_issues = h.check_hit_targets(vec2(24.0, 24.0));
    let timeline_hit_issues: Vec<_> = hit_issues
        .into_iter()
        .filter(|msg| {
            msg.contains("Hide on canvas")
                || msg.contains("Show on canvas")
                || msg.contains("Lock")
                || msg.contains("Unlock")
                || msg.contains("Play timeline")
                || msg.contains("Duplicate layer")
                || msg.contains("Delete layer")
        })
        .collect();
    assert!(
        timeline_hit_issues.is_empty(),
        "timeline/layers icon buttons failed >= 24x24 hit target check: {timeline_hit_issues:#?}"
    );
    assert!(header_max_x > row_rect.min.x);

    // Check Layers Eye, Gear, Duplicate, Delete buttons.
    let eye_rect = *h
        .editor
        .ui
        .layers
        .eye_rects
        .get("layer1")
        .expect("layer1 eye rect");
    let gear_rect = *h
        .editor
        .ui
        .layers
        .gear_rects
        .get("layer1")
        .expect("layer1 gear rect");
    for (name, rect) in [("layer eye", eye_rect), ("layer gear", gear_rect)] {
        assert!(
            rect.width() >= 24.0 && rect.height() >= 24.0,
            "{name} rect {:?} < 24x24",
            rect.size()
        );
        assert!(
            h.probe_tooltip(rect.center()).is_some(),
            "{name} must have a tooltip"
        );
    }
}

#[test]
fn t6_3_timeline_long_object_names_truncate_with_tooltip_and_bars_stay_clipped() {
    let mut h = TestHarness::new_with_size(timeline_layers_course(), (1280.0, 800.0));
    h.idle_frame();
    h.idle_frame();

    let row_rect = h
        .editor
        .ui
        .timeline
        .object_row_rects
        .get("obj1")
        .copied()
        .expect("obj1 row rect");
    let header_area = pos2(row_rect.min.x + 40.0, row_rect.center().y);
    let tip = h.probe_tooltip(header_area).unwrap_or_default();
    assert!(
        tip.contains("Extraordinarily Long Object Name"),
        "truncated object name should show full name tooltip on hover, got {tip:?}"
    );

    // Zoom in and scroll horizontally; verify no layout errors and bars don't clip into header.
    h.editor.ui.timeline.zoom = 3.0;
    h.editor.ui.timeline.scroll_x = 120.0;
    h.idle_frame();
    let report = h.layout_report(1);
    let timeline_errors: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.contains("clipped") || f.contains("overlap"))
        .collect();
    assert!(
        timeline_errors.is_empty(),
        "timeline layout errors when zoomed/scrolled: {timeline_errors:#?}"
    );
}

#[test]
fn t6_4_timeline_trim_drag_commits_single_atomic_undo_step() {
    let mut h = TestHarness::new_with_size(timeline_layers_course(), (1280.0, 800.0));
    h.idle_frame();
    h.idle_frame();

    let orig_end = h
        .editor
        .session
        .course
        .slide("s1")
        .and_then(|s| s.object("obj1"))
        .and_then(|(o, _)| o.timing.end_ms);
    assert_eq!(orig_end, Some(3000));

    let handle_rect = *h
        .editor
        .ui
        .timeline
        .object_right_handle_rects
        .get("obj1")
        .expect("obj1 right handle rect");
    h.drag_by(handle_rect.center(), vec2(40.0, 0.0), 6, Modifiers::NONE);
    h.idle_frame();

    let new_end = h
        .editor
        .session
        .course
        .slide("s1")
        .and_then(|s| s.object("obj1"))
        .and_then(|(o, _)| o.timing.end_ms);
    assert_ne!(new_end, orig_end, "trim drag should change end_ms");

    assert!(
        h.editor.session.undo().is_ok(),
        "should have an undo step after trim drag"
    );
    let restored_end = h
        .editor
        .session
        .course
        .slide("s1")
        .and_then(|s| s.object("obj1"))
        .and_then(|(o, _)| o.timing.end_ms);
    assert_eq!(
        restored_end, orig_end,
        "one single undo step must restore exact prior end_ms"
    );
}

#[test]
fn t6_5_narrow_layers_pane_200px_truncates_long_names_with_tooltip_in_both_themes() {
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
        let mut h = TestHarness::new_with_theme(timeline_layers_course(), (880.0, 600.0), mode);
        h.idle_frame();
        h.idle_frame();

        let layers_panel = h.editor.ui.layers.panel_rect.expect("layers panel rect");
        assert!(
            (layers_panel.width() - crate::layers::LAYERS_PANEL_NARROW_WIDTH).abs() <= 2.0,
            "at 880x600 (< 900 px) layers pane should be narrow (~200 px), got {}",
            layers_panel.width()
        );

        let row_rect = *h
            .editor
            .ui
            .layers
            .layer_row_rects
            .get("layer1")
            .expect("layer1 row rect");
        let name_pos = pos2(row_rect.left() + 60.0, row_rect.center().y);
        let tip = h.probe_tooltip(name_pos).unwrap_or_default();
        assert!(
            tip.contains("Extraordinarily Long Layer Name"),
            "[{mode:?}] narrow layers pane should truncate long layer name with full tooltip, got {tip:?}"
        );

        // Footer controls (+ New layer, Duplicate, Delete, Dim) must all fit inside panel_rect.
        for (label, rect) in [
            ("add", h.editor.ui.layers.add_button_rect),
            ("duplicate", h.editor.ui.layers.duplicate_button_rect),
            ("delete", h.editor.ui.layers.delete_button_rect),
            ("dim", h.editor.ui.layers.dim_rect),
        ] {
            let rect = rect.expect(label);
            assert!(
                layers_panel.expand(1.5).contains_rect(rect),
                "[{mode:?}] layers footer '{label}' rect {rect:?} clipped outside {layers_panel:?}"
            );
        }
    }
}

#[test]
fn t6_6_layers_disabled_duplicate_delete_and_move_explain_why_on_hover() {
    let mut h = TestHarness::new_with_size(timeline_layers_course(), (1280.0, 800.0));
    h.editor.selection.layer = None; // Base layer selected
    h.idle_frame();
    h.idle_frame();

    let dup_rect = h
        .editor
        .ui
        .layers
        .duplicate_button_rect
        .expect("duplicate rect");
    let del_rect = h.editor.ui.layers.delete_button_rect.expect("delete rect");
    let dup_tip = h.probe_tooltip(dup_rect.center()).unwrap_or_default();
    let del_tip = h.probe_tooltip(del_rect.center()).unwrap_or_default();
    assert!(
        dup_tip.contains("Select a layer"),
        "disabled Duplicate tooltip should explain why, got {dup_tip:?}"
    );
    assert!(
        del_tip.contains("Select a layer"),
        "disabled Delete tooltip should explain why, got {del_tip:?}"
    );

    // Right-click the single layer row to open context menu; Bring forward & Send backward are disabled.
    let row_rect = *h
        .editor
        .ui
        .layers
        .layer_row_rects
        .get("layer1")
        .expect("layer1 row rect");
    h.right_click_at(pos2(row_rect.left() + 60.0, row_rect.center().y));
    h.idle_frame();
    h.idle_frame();
    assert!(
        egui::Popup::is_any_open(&h.ctx),
        "expected layer context menu to be open"
    );

    let fwd_pos = h
        .find_text_last("Bring forward")
        .expect("Bring forward menu item");
    h.hover_at(fwd_pos);
    let fwd_tip = h.probe_tooltip(fwd_pos).unwrap_or_default();
    assert!(
        !fwd_tip.is_empty(),
        "disabled 'Bring forward' menu item must show a reason on hover (popup_open={})",
        egui::Popup::is_any_open(&h.ctx)
    );
    let bwd_pos = h
        .find_text_last("Send backward")
        .expect("Send backward menu item");
    h.hover_at(bwd_pos);
    let bwd_tip = h.probe_tooltip(bwd_pos).unwrap_or_default();
    assert!(
        !bwd_tip.is_empty(),
        "disabled 'Send backward' menu item must show a reason on hover (popup_open={})",
        egui::Popup::is_any_open(&h.ctx)
    );
}

#[test]
fn t6_7_layer_settings_dialog_copy_combos_disabled_ok_and_escape_stack() {
    let mut h = TestHarness::new_with_size(timeline_layers_course(), (1280.0, 800.0));
    h.idle_frame();
    crate::layers::open_layer_settings(&mut h.editor, "s1", "layer1");
    h.idle_frame();
    h.idle_frame();

    // Zero trailing colons on section headers & no "Stop clicks" negative wording.
    for forbidden in [
        "While this layer shows:",
        "When its timeline ends:",
        "Learner seeking:",
        "When shown again:",
        "Stop clicks on the base layer",
    ] {
        assert!(
            h.find_text(forbidden).is_none(),
            "Layer Settings must not paint '{forbidden}'"
        );
    }

    // Combos must fill form.row width (>= 120) and have tall popups.
    let allow_seek_rect = h
        .editor
        .ui
        .layer_settings
        .allow_seek_rect
        .expect("allow_seek_rect");
    let revisit_rect = h
        .editor
        .ui
        .layer_settings
        .revisit_rect
        .expect("revisit_rect");
    assert!(
        allow_seek_rect.width() >= 120.0,
        "layer_allow_seek width {} < 120",
        allow_seek_rect.width()
    );
    assert!(
        revisit_rect.width() >= 120.0,
        "layer_revisit width {} < 120",
        revisit_rect.width()
    );

    // Open layer_allow_seek combo, press Esc once -> only combo closes, dialog stays open!
    h.click_at(allow_seek_rect.center());
    h.idle_frame();
    assert!(
        egui::Popup::is_any_open(&h.ctx),
        "allow_seek combo popup should be open"
    );
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(
        !egui::Popup::is_any_open(&h.ctx),
        "first Esc should close the combo popup"
    );
    assert!(
        h.editor.ui.layer_settings.open.is_some(),
        "first Esc must NOT close the Layer Settings dialog while a combo is open"
    );

    // Clear Name field -> OK button is disabled and explains why on hover.
    h.editor.ui.layer_settings.name.clear();
    h.idle_frame();
    let ok_rect = h.editor.ui.layer_settings.ok_rect.expect("ok_rect");
    let ok_tip = h.probe_tooltip(ok_rect.center()).unwrap_or_default();
    assert!(
        ok_tip.contains("Enter a layer name"),
        "disabled OK button must explain 'Enter a layer name' on hover, got {ok_tip:?}"
    );

    // Second Esc closes the dialog.
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(
        h.editor.ui.layer_settings.open.is_none(),
        "second Esc must close the Layer Settings dialog"
    );
}

#[test]
fn t6_8_rename_cue_dialog_bounds_enter_commit_undo_and_escape_cancel() {
    let mut h = TestHarness::new_with_size(timeline_layers_course(), (1280.0, 800.0));
    h.idle_frame();
    h.idle_frame();

    let cue_rect = *h
        .editor
        .ui
        .timeline
        .slide_cue_rects
        .get("cue1")
        .expect("cue1 marker rect");
    h.double_click_at(cue_rect.center());
    h.idle_frame();
    assert!(
        h.editor.ui.timeline.slide_cue_rename.is_some(),
        "double-clicking cue marker should open Rename timeline cue dialog"
    );

    // Esc cancels without changing cue name.
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(
        h.editor.ui.timeline.slide_cue_rename.is_none(),
        "Esc must cancel Rename timeline cue dialog"
    );

    // Reopen, rename via Enter, verify 1 undo step.
    h.advance_time(1.0);
    h.double_click_at(cue_rect.center());
    h.idle_frame();
    assert!(
        h.editor.ui.timeline.slide_cue_rename.is_some(),
        "second double-click should reopen Rename timeline cue dialog"
    );
    let field_pos = h.find_text("Cue One").expect("cue name text field");
    h.set_field(field_pos, "Cue Renamed");
    h.press(Key::Enter, Modifiers::NONE);
    h.idle_frame();

    let name_after = h
        .editor
        .session
        .course
        .slide("s1")
        .and_then(|s| s.cue_points.iter().find(|c| c.id == "cue1"))
        .map(|c| c.name.as_str());
    assert_eq!(name_after, Some("Cue Renamed"));

    assert!(h.editor.session.undo().is_ok());
    let name_undone = h
        .editor
        .session
        .course
        .slide("s1")
        .and_then(|s| s.cue_points.iter().find(|c| c.id == "cue1"))
        .map(|c| c.name.as_str());
    assert_eq!(name_undone, Some("Cue One"));
}
