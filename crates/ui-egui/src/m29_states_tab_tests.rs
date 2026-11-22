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

#[test]
fn sus03_canvas_preview_state_emits_extras_and_placeholders() {
    let mut course = course_with_states(0);
    let btn: coursecraft_course::Object = serde_json::from_value(serde_json::json!({
        "id": "btn_extra",
        "name": "Button Extra",
        "frame": { "x": 100.0, "y": 100.0, "w": 160.0, "h": 48.0 },
        "kind": { "type": "button", "label": "Next", "fill": "#2E6F9E" },
        "states": [{
            "name": "visited",
            "extras": [
                {
                    "id": "ex-badge",
                    "frame": { "x": 140.0, "y": -6.0, "w": 24.0, "h": 24.0 },
                    "kind": { "type": "shape", "shape": "ellipse", "fill": "#16A34A" }
                },
                {
                    "id": "ex-img",
                    "frame": { "x": 170.0, "y": -6.0, "w": 24.0, "h": 24.0 },
                    "kind": { "type": "image", "src": "media/missing.png", "alt": "Missing" }
                }
            ]
        }]
    }))
    .expect("valid button");
    course.slide_mut("s1").expect("slide").objects.push(btn);

    let h = TestHarness::new(course);
    let slide = h.editor.session.course.slide("s1").expect("slide").clone();
    let (obj, _) = slide.object("btn_extra").expect("btn_extra");

    let mut items = coursecraft_render::compose_authoring_layers(
        &h.editor.session.course,
        &slide,
        None,
        &[],
        false,
    );
    crate::canvas::apply_canvas_state_preview(
        &mut items,
        obj,
        "visited",
        Some(&h.editor.session.course.theme),
        crate::states::course_input_value(&h.editor.session.course, obj),
    );
    crate::canvas::missing_images_as_placeholders(&mut items, &h.editor.media);

    let btn_items: Vec<_> = items
        .iter()
        .filter(|it| matches!(&it.hit, coursecraft_render::Hit::Object(id) if id == "btn_extra"))
        .collect();
    assert_eq!(
        btn_items.len(),
        3,
        "expected parent + 2 extras, got {btn_items:?}"
    );
    assert!(!btn_items[0].is_state_extra);
    assert!(btn_items[1].is_state_extra);
    assert!(btn_items[2].is_state_extra);
    assert!(
        matches!(
            btn_items[2].visual,
            coursecraft_render::Visual::ImagePlaceholder { .. }
        ),
        "missing extra image must become ImagePlaceholder"
    );
}

#[test]
fn u04_quick_add_chips_and_custom_name_validation_no_hidden_chip() {
    let mut h = TestHarness::new(course_with_states(0));
    h.editor.selection.object = Some("btn1".to_owned());
    h.editor.ui.bottom_tab = BottomTab::States;
    h.idle_frame();

    assert!(
        h.find_text("+ Hidden").is_none(),
        "Hidden must not appear as a quick-add chip"
    );

    let hover_chip = h
        .find_text("+ Hover")
        .expect("+ Hover chip should be visible");
    h.click_at(hover_chip);
    h.idle_frame();
    let slide = h.editor.session.course.slide("s1").unwrap();
    let (obj, _) = slide.object("btn1").unwrap();
    assert!(obj.has_state("hover"), "+ Hover must add 'hover' state");
    assert_eq!(
        h.editor.ui.preview_state.as_deref(),
        Some("hover"),
        "newly added state should become preview_state"
    );
    let hover_node = h
        .last_widgets
        .iter()
        .find(|w| w.role == egui::accesskit::Role::Button && w.label.as_deref() == Some("+ Hover"))
        .expect("+ Hover accesskit node");
    assert!(
        hover_node.disabled,
        "+ Hover chip must be disabled once added"
    );
    let hover_bounds = hover_node.bounds;
    h.hover_at(pos2(0.0, 0.0));
    h.idle_frame();
    let tip = h.probe_tooltip(hover_bounds.center());
    assert_eq!(tip.as_deref(), Some("Already added"));

    // Add custom state via "+ New state" + Enter
    let new_btn = h.find_text("+ New state").expect("+ New state button");
    h.click_at(new_btn);
    h.idle_frame();
    let input_rect = h
        .editor
        .ui
        .states
        .new_state_name_rect
        .expect("new state input rect");
    h.click_at(input_rect.center());
    h.type_text("custom_1");
    h.press(egui::Key::Enter, egui::Modifiers::NONE);
    h.idle_frame();
    let slide = h.editor.session.course.slide("s1").unwrap();
    let (obj, _) = slide.object("btn1").unwrap();
    assert!(
        obj.has_state("custom_1"),
        "typing 'custom_1' + Enter must add 'custom_1'"
    );

    // Reserved names "normal" and "hidden" show inline hint and disable Add
    let new_btn = h.find_text("+ New state").expect("+ New state button");
    h.click_at(new_btn);
    h.idle_frame();
    for reserved in ["normal", "hidden"] {
        let input_rect = h
            .editor
            .ui
            .states
            .new_state_name_rect
            .expect("new state input rect");
        h.set_field(input_rect.center(), reserved);
        h.idle_frame();
        let add_rect = h.editor.ui.states.add_button_rect.expect("Add button rect");
        let add_node = h
            .last_widgets
            .iter()
            .find(|w| w.role == egui::accesskit::Role::Button && w.bounds.intersects(add_rect))
            .expect("Add button accesskit node");
        assert!(
            add_node.disabled,
            "Add button must be disabled for reserved name '{reserved}'"
        );
        h.hover_at(pos2(0.0, 0.0));
        h.idle_frame();
        assert!(
            h.probe_tooltip(add_rect.center()).is_some(),
            "disabled Add button must show validation tooltip for '{reserved}'"
        );
    }
}

#[test]
fn u05_card_actions_duplicate_inline_rename_reorder_set_initial_delete_and_undo() {
    let mut course = course_with_states(2); // hover, selected
    // Add a freeform question with correct_when referencing "selected" (SUS-09)
    course.slide_mut("s1").unwrap().question = Some(
        serde_json::from_value(serde_json::json!({
            "kind": "freeform",
            "prompt": "Make btn1 selected",
            "choices": [],
            "correct_when": { "object_state": { "object": "btn1", "state": "selected", "is": true } }
        }))
        .unwrap(),
    );

    let mut h = TestHarness::new(course);
    h.editor.selection.object = Some("btn1".to_owned());
    h.editor.ui.bottom_tab = BottomTab::States;
    h.idle_frame();

    // 1. Duplicate "hover" via card ⋯ menu -> "hover copy" inserted right after "hover"
    let hover_card = *h
        .editor
        .ui
        .states
        .state_row_rects
        .get("hover")
        .expect("hover card rect");
    let menu_pos = pos2(hover_card.max.x - 18.0, hover_card.max.y - 16.0);
    h.click_at(menu_pos);
    h.idle_frame();
    let dup_pos = h.find_text("Duplicate").expect("Duplicate in card menu");
    h.click_at(dup_pos);
    h.idle_frame();
    let names: Vec<&str> = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object("btn1")
        .unwrap()
        .0
        .states
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert_eq!(names, vec!["hover", "hover copy", "selected"]);

    // Single-step Undo / Redo
    h.press(egui::Key::Z, egui::Modifiers::COMMAND);
    h.idle_frame();
    let names_after_undo: Vec<&str> = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object("btn1")
        .unwrap()
        .0
        .states
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert_eq!(names_after_undo, vec!["hover", "selected"]);

    h.press(
        egui::Key::Z,
        egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
    );
    h.idle_frame();
    let names_after_redo: Vec<&str> = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object("btn1")
        .unwrap()
        .0
        .states
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert_eq!(names_after_redo, vec!["hover", "hover copy", "selected"]);

    // 2. Rename "hover copy" via card ⋯ menu -> "Rename…"
    let copy_card = *h
        .editor
        .ui
        .states
        .state_row_rects
        .get("hover copy")
        .expect("hover copy card rect");
    h.click_at(pos2(copy_card.max.x - 18.0, copy_card.max.y - 16.0));
    h.idle_frame();
    let rename_pos = h.find_text("Rename…").expect("Rename… in card menu");
    h.click_at(rename_pos);
    h.idle_frame();
    h.press(egui::Key::A, egui::Modifiers::COMMAND);
    h.type_text("active");
    h.press(egui::Key::Enter, egui::Modifiers::NONE);
    h.idle_frame();
    let names: Vec<&str> = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object("btn1")
        .unwrap()
        .0
        .states
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert_eq!(names, vec!["hover", "active", "selected"]);

    // 3. Move right on "active" -> ["hover", "selected", "active"]
    let active_card = *h
        .editor
        .ui
        .states
        .state_row_rects
        .get("active")
        .expect("active card rect");
    h.click_at(pos2(active_card.max.x - 18.0, active_card.max.y - 16.0));
    h.idle_frame();
    let move_right = h.find_text("Move right").expect("Move right in card menu");
    h.click_at(move_right);
    h.idle_frame();
    let names: Vec<&str> = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object("btn1")
        .unwrap()
        .0
        .states
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert_eq!(names, vec!["hover", "selected", "active"]);

    // 4. Set as initial on "active"
    let active_card = *h
        .editor
        .ui
        .states
        .state_row_rects
        .get("active")
        .expect("active card rect");
    h.click_at(pos2(active_card.max.x - 18.0, active_card.max.y - 16.0));
    h.idle_frame();
    let set_init = h
        .find_text("Set as initial")
        .expect("Set as initial in card menu");
    h.click_at(set_init);
    h.idle_frame();
    assert_eq!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .object("btn1")
            .unwrap()
            .0
            .initial_state,
        "active"
    );

    // 5. SUS-09: Delete on "selected" (referenced by question.correct_when) must be disabled with blocker tooltip
    let selected_card = *h
        .editor
        .ui
        .states
        .state_row_rects
        .get("selected")
        .expect("selected card rect");
    h.click_at(pos2(selected_card.max.x - 18.0, selected_card.max.y - 16.0));
    h.idle_frame();
    let del_pos = h.find_text("Delete").expect("Delete in card menu");
    let del_node = h
        .last_widgets
        .iter()
        .find(|w| {
            w.role == egui::accesskit::Role::Button
                && w.label.as_deref() == Some("Delete")
                && w.bounds.contains(del_pos)
        })
        .expect("Delete button node");
    assert!(
        del_node.disabled,
        "Delete must be disabled when state is referenced by question.correct_when (SUS-09)"
    );
    h.hover_at(pos2(0.0, 0.0));
    let del_tip = h
        .probe_tooltip(del_pos)
        .expect("disabled Delete must show blocker tooltip");
    assert!(
        del_tip.to_lowercase().contains("question"),
        "blocker tooltip should mention question reference, got: {del_tip}"
    );
    h.press(egui::Key::Escape, egui::Modifiers::NONE);
    h.idle_frame();

    // 6. Delete on unblocked "hover" succeeds and undoes
    let hover_card = *h
        .editor
        .ui
        .states
        .state_row_rects
        .get("hover")
        .expect("hover card rect");
    h.click_at(pos2(hover_card.max.x - 18.0, hover_card.max.y - 16.0));
    h.idle_frame();
    let del_pos = h.find_text("Delete").expect("Delete in card menu");
    h.click_at(del_pos);
    h.idle_frame();
    assert!(
        !h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .object("btn1")
            .unwrap()
            .0
            .has_state("hover")
    );
    h.press(egui::Key::Z, egui::Modifiers::COMMAND);
    h.idle_frame();
    assert!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .object("btn1")
            .unwrap()
            .0
            .has_state("hover")
    );
}

#[test]
fn u07_keyboard_navigation_and_accesskit_in_states_tab() {
    let mut h = TestHarness::new(course_with_states(2)); // hover, selected
    h.editor.selection.object = Some("btn1".to_owned());
    h.editor.ui.bottom_tab = BottomTab::States;
    h.idle_frame();

    // Click Normal card to focus the card strip
    let normal_card = *h
        .editor
        .ui
        .states
        .state_row_rects
        .get("normal")
        .expect("normal card rect");
    h.click_at(normal_card.center());
    h.idle_frame();

    // ArrowRight moves active card to "hover", then "selected"
    h.press(egui::Key::ArrowRight, egui::Modifiers::NONE);
    h.idle_frame();
    assert_eq!(h.editor.ui.preview_state.as_deref(), Some("hover"));

    h.press(egui::Key::ArrowRight, egui::Modifiers::NONE);
    h.idle_frame();
    assert_eq!(h.editor.ui.preview_state.as_deref(), Some("selected"));

    // ArrowLeft moves back to "hover"
    h.press(egui::Key::ArrowLeft, egui::Modifiers::NONE);
    h.idle_frame();
    assert_eq!(h.editor.ui.preview_state.as_deref(), Some("hover"));

    // F2 starts inline rename on "hover", Esc cancels rename without clearing preview_state
    h.press(egui::Key::F2, egui::Modifiers::NONE);
    h.idle_frame();
    assert_eq!(
        h.editor.ui.states.renaming_state.as_deref(),
        Some("hover"),
        "F2 should start inline rename on active custom state"
    );
    h.press(egui::Key::Escape, egui::Modifiers::NONE);
    h.idle_frame();
    assert!(
        h.editor.ui.states.renaming_state.is_none(),
        "Esc should cancel inline rename"
    );
    assert_eq!(
        h.editor.ui.preview_state.as_deref(),
        Some("hover"),
        "Esc cancelling rename must not clear preview_state"
    );

    // Delete removes unblocked "hover"
    h.press(egui::Key::Delete, egui::Modifiers::NONE);
    h.idle_frame();
    assert!(
        !h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .object("btn1")
            .unwrap()
            .0
            .has_state("hover"),
        "Delete key should remove unblocked active custom state"
    );

    // AccessKit labels & >= 24x24 hit targets on visible state cards and ⋯ buttons
    let selected_card = *h
        .editor
        .ui
        .states
        .state_row_rects
        .get("selected")
        .expect("selected card rect");
    let menu_btn_node = h
        .last_widgets
        .iter()
        .find(|w| {
            w.role == egui::accesskit::Role::Button
                && selected_card.contains(w.bounds.center())
                && w.bounds.width() < 40.0
        })
        .expect("⋯ menu button should have an accesskit Button node");
    assert!(
        menu_btn_node
            .label
            .as_deref()
            .is_some_and(|l| !l.trim().is_empty()),
        "⋯ menu button must have a non-empty accesskit label"
    );
    assert!(
        menu_btn_node.bounds.width() >= 24.0 && menu_btn_node.bounds.height() >= 24.0,
        "⋯ menu button must be >= 24x24 pt, got {:?}",
        menu_btn_node.bounds.size()
    );
    let card_node = h
        .last_widgets
        .iter()
        .find(|w| {
            w.role == egui::accesskit::Role::Button
                && w.bounds.width() >= 100.0
                && selected_card.contains(w.bounds.center())
        })
        .expect("state card must have an accesskit Button node");
    assert!(
        card_node
            .label
            .as_deref()
            .is_some_and(|l| !l.trim().is_empty()),
        "state card must have a non-empty accesskit label"
    );
}
