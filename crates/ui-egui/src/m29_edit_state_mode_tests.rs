//! M29 Step s5 tests: Canvas Edit-State Mode chrome, dimming, locking,
//! `Esc` stack (`SUS-13`, `UX-13`), rotate/crop routing, and undo/delete safety (`U-08`, `U-11`, `U-18`).

use crate::BottomTab;
use crate::InspectorTab;
use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use egui::{Key, Modifiers, pos2, vec2};

fn course_with_objects() -> Course {
    let raw = serde_json::json!({
        "schema_version": 1,
        "id": "c-m29-s5",
        "title": "M29 S5 Test",
        "slide_size": { "w": 1280, "h": 720 },
        "theme": {
            "accent": "#1166CC",
            "text": "#222222",
            "background": "#F5F5F5",
            "font": "Inter"
        },
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
                        "states": [
                            { "name": "hover", "fill": "#1E40AF", "text": "Hover Me" },
                            { "name": "selected", "fill": "#0F766E" }
                        ]
                    },
                    {
                        "id": "rect2",
                        "name": "Second Shape",
                        "frame": { "x": 500.0, "y": 200.0, "w": 160.0, "h": 100.0 },
                        "kind": { "type": "shape", "shape": "rectangle", "fill": "#AABBCC" }
                    }
                ]
            }]
        }]
    });
    serde_json::from_value(raw).expect("valid course")
}

#[test]
fn u08_enter_and_exit_edit_state_mode_with_esc_stack() {
    let mut h = TestHarness::new(course_with_objects());
    h.editor.selection.set_many(["btn1".to_owned()]);
    h.editor.ui.bottom_tab = BottomTab::States;
    h.idle_frame();

    // Select "hover" card, then click "Edit state".
    let hover_pos = h
        .find_text("hover")
        .expect("hover state card must appear in States tab");
    h.click_at(hover_pos);
    h.idle_frame();

    let edit_btn = h
        .find_text("Edit state")
        .expect("Edit state button must appear");
    h.click_at(edit_btn);
    h.idle_frame();

    assert!(
        h.editor.ui.editing_state.is_some(),
        "clicking Edit state must enter StateEditSession"
    );
    assert_eq!(
        h.editor
            .ui
            .editing_state
            .as_ref()
            .map(|s| s.state_name.as_str()),
        Some("hover")
    );
    assert!(
        h.find_text_containing("Editing state").is_some(),
        "top banner must show Editing state label"
    );
    assert!(
        h.find_text("Done editing").is_some(),
        "top banner must show Done editing button"
    );
    assert!(
        h.find_text("Reset overrides").is_some(),
        "top banner must show Reset overrides button"
    );

    // Clicking non-target object rect2 on canvas must be ignored.
    let slide_rect = h.editor.ui.last_slide_rect.expect("slide rect");
    let rect2_screen = pos2(
        slide_rect.min.x + slide_rect.width() * (580.0 / 1280.0),
        slide_rect.min.y + slide_rect.height() * (250.0 / 720.0),
    );
    h.click_at(rect2_screen);
    h.idle_frame();
    h.advance_time(1.0);
    h.idle_frame();
    assert_eq!(
        h.editor.selection.object.as_deref(),
        Some("btn1"),
        "clicking non-target object in edit-state mode must not change selection"
    );
    assert!(h.editor.ui.editing_state.is_some());

    // Esc stack: inline edit -> selected_extra -> editing_state -> object selection.
    let btn1_screen = pos2(
        slide_rect.min.x + slide_rect.width() * (210.0 / 1280.0),
        slide_rect.min.y + slide_rect.height() * (132.0 / 720.0),
    );
    h.double_click_at(btn1_screen);
    assert!(
        h.editor.ui.inline_edit.is_some(),
        "double-clicking text-bearing target in edit-state mode opens inline edit (slide_rect={slide_rect:?}, btn1_screen={btn1_screen:?})"
    );

    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(
        h.editor.ui.inline_edit.is_none(),
        "first Esc must close inline edit"
    );
    assert!(
        h.editor.ui.editing_state.is_some(),
        "Esc closing inline edit must NOT exit editing_state"
    );

    if let Some(session) = h.editor.ui.editing_state.as_mut() {
        session.selected_extra = Some("extra_1".to_owned());
    }
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert_eq!(
        h.editor
            .ui
            .editing_state
            .as_ref()
            .and_then(|s| s.selected_extra.as_deref()),
        None,
        "Esc must clear selected_extra before exiting editing_state"
    );
    assert!(
        h.editor.ui.editing_state.is_some(),
        "Esc clearing selected_extra must NOT exit editing_state"
    );

    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(
        h.editor.ui.editing_state.is_none(),
        "Esc must exit editing_state before clearing object selection"
    );
    assert_eq!(
        h.editor.selection.object.as_deref(),
        Some("btn1"),
        "Esc exiting editing_state must keep object selection"
    );

    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert_eq!(
        h.editor.selection.object, None,
        "final Esc clears object selection"
    );

    // Re-enter via "Edit state" and exit via "Done editing".
    h.editor.selection.set_many(["btn1".to_owned()]);
    h.idle_frame();
    let hover_pos = h.find_text("hover").expect("hover card");
    h.click_at(hover_pos);
    h.idle_frame();
    let edit_btn = h
        .find_text("Edit state")
        .expect("Edit state button must appear");
    h.click_at(edit_btn);
    h.idle_frame();
    assert!(h.editor.ui.editing_state.is_some());

    let done_pos = h
        .find_text("Done editing")
        .expect("Done editing button must appear");
    h.click_at(done_pos);
    h.idle_frame();
    assert!(
        h.editor.ui.editing_state.is_none(),
        "Done editing button must exit editing_state"
    );
}

#[test]
fn u08_reset_overrides_button_clears_state_overrides() {
    let mut h = TestHarness::new(course_with_objects());
    h.editor.selection.set_many(["btn1".to_owned()]);
    h.editor.ui.bottom_tab = BottomTab::States;
    h.idle_frame();

    let hover_pos = h.find_text("hover").expect("hover card");
    h.click_at(hover_pos);
    h.idle_frame();
    let edit_btn = h.find_text("Edit state").expect("Edit state button");
    h.click_at(edit_btn);
    h.idle_frame();

    let reset_pos = h
        .find_text("Reset overrides")
        .expect("Reset overrides button");
    h.click_at(reset_pos);
    h.idle_frame();

    let obj = h
        .editor
        .session
        .course
        .slide("s1")
        .and_then(|s| s.object("btn1"))
        .map(|(o, _)| o)
        .expect("btn1");
    let hover = obj
        .states
        .iter()
        .find(|s| s.name == "hover")
        .expect("hover state");
    assert!(hover.fill.is_none(), "Reset overrides must clear fill");
    assert!(hover.text.is_none(), "Reset overrides must clear text");

    h.hover_at(pos2(0.0, 0.0));
    h.idle_frame();
    let reset_pos = h
        .find_text("Reset overrides")
        .expect("Reset overrides button after reset");
    let tip = h.probe_tooltip(reset_pos);
    assert_eq!(
        tip.as_deref(),
        Some("No overrides to reset"),
        "disabled Reset overrides must show explanatory tooltip"
    );
}

#[test]
fn u11_parent_frame_and_object_controls_locked_in_edit_state_mode() {
    let mut h = TestHarness::new(course_with_objects());
    h.editor.selection.set_many(["btn1".to_owned()]);
    h.editor.ui.bottom_tab = BottomTab::States;
    h.idle_frame();

    let hover_pos = h.find_text("hover").expect("hover card");
    h.click_at(hover_pos);
    h.idle_frame();
    let edit_btn = h.find_text("Edit state").expect("Edit state button");
    h.click_at(edit_btn);
    h.idle_frame();

    // Dragging parent on canvas must NOT move obj.frame.
    let slide_rect = h.editor.ui.last_slide_rect.expect("slide rect");
    let btn1_screen = pos2(
        slide_rect.min.x + slide_rect.width() * (210.0 / 1280.0),
        slide_rect.min.y + slide_rect.height() * (132.0 / 720.0),
    );
    h.drag_by(btn1_screen, vec2(80.0, 40.0), 5, Modifiers::NONE);
    h.idle_frame();

    let frame = h
        .editor
        .session
        .course
        .slide("s1")
        .and_then(|s| s.object("btn1"))
        .map(|(o, _)| o.frame)
        .expect("btn1");
    assert!(
        (frame.x - 100.0).abs() < 0.01 && (frame.y - 100.0).abs() < 0.01,
        "parent frame must remain locked during edit-state mode, got {frame:?}"
    );

    // Check Format tab locked controls (Name, X/Y/W/H, Timing, Arrange) have explanatory tooltip.
    let right_rect = h.right_panel_rect();
    let x_pos = h
        .find_text_in("Position", right_rect)
        .expect("Position row in Format tab");
    let tip = h.probe_tooltip(x_pos + vec2(80.0, 0.0));
    assert_eq!(
        tip.as_deref(),
        Some("Shared across all states — exit state editing to change"),
        "Position X/Y must be disabled with shared-across-states tooltip"
    );

    let name_pos = h
        .find_text_in("Name", right_rect)
        .expect("Name row in Format tab");
    let name_tip = h.probe_tooltip(name_pos + vec2(80.0, 0.0));
    assert_eq!(
        name_tip.as_deref(),
        Some("Shared across all states — exit state editing to change"),
        "Name must be disabled with shared-across-states tooltip"
    );

    // Animate tab controls must also be disabled with shared-across-states tooltip.
    h.editor.ui.inspector_tab = InspectorTab::Animate;
    h.idle_frame();
    assert!(
        h.find_text_containing("Shared across all states").is_some(),
        "Animate tab must indicate controls are shared across all states"
    );
}

#[test]
fn u18_undo_or_delete_edited_state_exits_edit_state_mode() {
    let mut h = TestHarness::new(course_with_objects());
    h.editor.selection.set_many(["btn1".to_owned()]);
    h.editor.run(
        "object.add_state",
        serde_json::json!({
            "slide": "s1",
            "object": "btn1",
            "state": { "name": "custom_s", "fill": "#445566" }
        }),
    );
    h.editor.ui.bottom_tab = BottomTab::States;
    h.idle_frame();

    let custom_pos = h.find_text("custom_s").expect("custom_s card");
    h.click_at(custom_pos);
    h.idle_frame();
    let edit_btn = h.find_text("Edit state").expect("Edit state button");
    h.click_at(edit_btn);
    h.idle_frame();
    assert!(h.editor.ui.editing_state.is_some());

    // Undo past object.add_state must cleanly exit editing_state.
    let _ = h.editor.session.undo();
    h.idle_frame();
    assert!(
        h.editor.ui.editing_state.is_none(),
        "undoing past object.add_state must exit editing_state"
    );
    assert!(
        h.editor.ui.preview_state.is_none(),
        "undoing past object.add_state must clear preview_state"
    );

    // Deleting the edited state must also cleanly exit editing_state.
    let _ = h.editor.session.redo();
    h.idle_frame();
    let custom_pos = h.find_text("custom_s").expect("custom_s card");
    h.click_at(custom_pos);
    h.idle_frame();
    let edit_btn = h.find_text("Edit state").expect("Edit state button");
    h.click_at(edit_btn);
    h.idle_frame();
    assert!(h.editor.ui.editing_state.is_some());

    h.editor.run(
        "object.delete_state",
        serde_json::json!({
            "slide": "s1",
            "object": "btn1",
            "name": "custom_s"
        }),
    );
    h.idle_frame();
    assert!(
        h.editor.ui.editing_state.is_none(),
        "deleting edited state must exit editing_state"
    );
    assert!(
        h.editor.ui.preview_state.is_none(),
        "deleting edited state must clear preview_state"
    );
}

#[test]
fn s5_canvas_rotate_handle_patches_state_in_edit_state_mode() {
    let mut h = TestHarness::new(course_with_objects());
    h.idle_frame();
    h.editor.selection.set_many(["btn1".to_owned()]);
    h.editor.enter_edit_state("s1", "btn1", "hover");
    h.idle_frame();

    let slide_rect = h.editor.ui.last_slide_rect.expect("slide rect");
    let scale = slide_rect.width() / 1280.0;
    // Rotate handle is 20 pt above the top-center of btn1 (100, 100, 220, 64).
    let handle_screen = pos2(
        slide_rect.min.x + 210.0 * scale,
        slide_rect.min.y + 100.0 * scale - 20.0,
    );
    h.drag_by(handle_screen, vec2(40.0, 20.0), 5, Modifiers::NONE);
    h.idle_frame();

    let obj = h
        .editor
        .session
        .course
        .slide("s1")
        .and_then(|s| s.object("btn1"))
        .map(|(o, _)| o)
        .expect("btn1");
    assert!(
        obj.rotation.abs() < 0.01,
        "base object rotation must remain 0 in edit-state mode, got {}",
        obj.rotation
    );
    let hover = obj
        .states
        .iter()
        .find(|s| s.name == "hover")
        .expect("hover state");
    assert!(
        hover.rotation.is_some_and(|r| r > 0.5),
        "dragging rotate handle in edit-state mode must patch hover.rotation, got {:?}",
        hover.rotation
    );
}

#[test]
fn s5_crop_mode_patches_state_in_edit_state_mode() {
    let raw = serde_json::json!({
        "schema_version": 1,
        "id": "c-m29-s5-crop",
        "title": "M29 S5 Crop Test",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "scene1",
            "title": "Main",
            "slides": [{
                "id": "s1",
                "title": "Slide 1",
                "objects": [{
                    "id": "img1",
                    "name": "Hero Image",
                    "frame": { "x": 100.0, "y": 100.0, "w": 200.0, "h": 150.0 },
                    "kind": { "type": "image", "src": "media/hero.png", "alt": "Hero" },
                    "states": [{ "name": "hover" }]
                }]
            }]
        }]
    });
    let course: Course = serde_json::from_value(raw).expect("valid course");
    let mut h = TestHarness::new(course);
    // Register a 1x1 PNG in memory media so start_crop succeeds.
    const TINY_PNG: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];
    h.editor
        .media
        .push(("media/hero.png".to_owned(), std::sync::Arc::from(TINY_PNG)));

    h.editor.enter_edit_state("s1", "img1", "hover");
    h.idle_frame();
    assert!(h.editor.start_crop("img1"), "start_crop should succeed");

    if let Some(session) = h.editor.ui.crop_edit.as_mut() {
        session.edit.crop = coursecraft_course::Rect {
            x: 0.1,
            y: 0.1,
            w: 0.8,
            h: 0.8,
        };
    }
    h.editor.apply_crop();
    h.idle_frame();

    let obj = h
        .editor
        .session
        .course
        .slide("s1")
        .and_then(|s| s.object("img1"))
        .map(|(o, _)| o)
        .expect("img1");
    if let coursecraft_course::ObjectKind::Image { crop, .. } = &obj.kind {
        assert!(crop.is_none(), "base image crop must remain None");
    } else {
        panic!("expected image");
    }
    let hover = obj
        .states
        .iter()
        .find(|s| s.name == "hover")
        .expect("hover state");
    assert!(
        hover.crop.is_some(),
        "apply_crop in edit-state mode must patch hover.crop"
    );

    h.editor.reset_crop();
    h.idle_frame();
    let obj = h
        .editor
        .session
        .course
        .slide("s1")
        .and_then(|s| s.object("img1"))
        .map(|(o, _)| o)
        .expect("img1");
    let hover = obj
        .states
        .iter()
        .find(|s| s.name == "hover")
        .expect("hover state");
    assert!(
        hover.crop.is_none(),
        "reset_crop in edit-state mode must clear hover.crop"
    );
}

#[test]
fn b05_renaming_active_state_in_states_tab_keeps_edit_state_mode() {
    let mut course = course_with_objects();
    course.slide_mut("s1").expect("s1").objects[0]
        .states
        .push(coursecraft_course::StateDef {
            name: "active".to_owned(),
            ..Default::default()
        });

    let mut h = TestHarness::new(course);
    h.editor.ui.bottom_tab = crate::BottomTab::States;
    h.editor.selection.set_many(["btn1".to_owned()]);
    h.editor.enter_edit_state("s1", "btn1", "active");
    h.idle_frame();

    let label_pos = h.find_text("active").expect("active state card label");
    h.double_click_at(label_pos);
    h.idle_frame();
    assert_eq!(h.editor.ui.states.renaming_state.as_deref(), Some("active"));

    h.editor.ui.states.rename_buffer.clear();
    h.type_text("chosen");
    h.press(egui::Key::Enter, Modifiers::NONE);
    h.idle_frame();
    h.idle_frame();

    assert_eq!(
        h.editor
            .ui
            .editing_state
            .as_ref()
            .map(|s| s.state_name.as_str()),
        Some("chosen"),
        "renaming the active state in Edit-State Mode must keep Edit-State Mode active"
    );
    assert_eq!(
        h.editor.ui.preview_state.as_deref(),
        Some("chosen"),
        "renaming the active state must update preview_state to the new name"
    );
}
