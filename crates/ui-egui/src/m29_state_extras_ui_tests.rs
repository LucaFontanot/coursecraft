//! M29 Step s10a tests: canvas insert, select, drag, resize, rotate, nudge,
//! and delete of `StateExtra`s inside Edit-State Mode (`U-14`, `U-15`).

use crate::test_harness::TestHarness;
use crate::{DrawTool, EditorRequest, PendingImagePick, PlacementTool};
use coursecraft_course::{Course, ShapeKind, StateExtraKind};
use egui::{Key, Modifiers, Pos2, pos2, vec2};

fn course_with_button_and_hover_state() -> Course {
    let raw = serde_json::json!({
        "schema_version": 1,
        "id": "c-m29-s10a",
        "title": "M29 S10a Test",
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
                            { "name": "hover", "fill": "#1E40AF" }
                        ]
                    }
                ]
            }]
        }]
    });
    serde_json::from_value(raw).expect("valid course")
}

fn course_with_hover_shape_extra() -> Course {
    let raw = serde_json::json!({
        "schema_version": 1,
        "id": "c-m29-s10a-extra",
        "title": "M29 S10a Extra Test",
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
                            {
                                "name": "hover",
                                "fill": "#1E40AF",
                                "extras": [
                                    {
                                        "id": "extra_1",
                                        "name": "Badge",
                                        "kind": {
                                            "type": "shape",
                                            "shape": "rectangle",
                                            "fill": "#FF0000"
                                        },
                                        "frame": { "x": 20.0, "y": 10.0, "w": 40.0, "h": 30.0 }
                                    }
                                ]
                            }
                        ]
                    }
                ]
            }]
        }]
    });
    serde_json::from_value(raw).expect("valid course")
}

fn slide_to_screen(h: &TestHarness, slide_pt: Pos2) -> Pos2 {
    let slide_rect = h.editor.ui.last_slide_rect.expect("last_slide_rect");
    pos2(
        slide_rect.min.x + slide_rect.width() * (slide_pt.x / 1280.0),
        slide_rect.min.y + slide_rect.height() * (slide_pt.y / 720.0),
    )
}

fn enter_hover_edit_mode(h: &mut TestHarness) {
    h.editor.selection.set_many(["btn1".to_owned()]);
    h.editor.enter_edit_state("s1", "btn1", "hover");
    h.idle_frame();
}

fn tiny_png_bytes() -> Vec<u8> {
    let pixels = vec![180u8; 4 * 4 * 4];
    coursecraft_media::encode_png_rgba(4, 4, &pixels).expect("encode png")
}

#[test]
fn u14_insert_shape_text_and_path_in_edit_state_mode_creates_parent_relative_extras() {
    let mut h = TestHarness::new(course_with_button_and_hover_state());
    enter_hover_edit_mode(&mut h);

    // 1. Insert Shape (Ellipse) by dragging on canvas from (150, 110) to (190, 150).
    crate::toolbar::insert_shape(&mut h.editor, ShapeKind::Ellipse);
    let p1 = slide_to_screen(&h, pos2(150.0, 110.0));
    let p2 = slide_to_screen(&h, pos2(190.0, 150.0));
    h.drag_by(p1, p2 - p1, 4, Modifiers::NONE);
    h.idle_frame();

    // 2. Insert TextBox by clicking at (120, 120).
    crate::toolbar::insert_text_box(&mut h.editor);
    let pt_text = slide_to_screen(&h, pos2(120.0, 120.0));
    h.click_at(pt_text);
    h.idle_frame();

    // 3. Insert Path (Line) by dragging from (110, 110) to (170, 130).
    crate::toolbar::insert_draw(&mut h.editor, DrawTool::Line);
    let d1 = slide_to_screen(&h, pos2(110.0, 110.0));
    let d2 = slide_to_screen(&h, pos2(170.0, 130.0));
    h.drag_by(d1, d2 - d1, 4, Modifiers::NONE);
    h.idle_frame();

    let slide = h.editor.session.course.slide("s1").expect("slide");
    let (btn, _) = slide.object("btn1").expect("btn1");
    assert_eq!(
        slide.objects.len(),
        1,
        "must not add top-level slide objects"
    );

    let hover = btn
        .states
        .iter()
        .find(|s| s.name == "hover")
        .expect("hover");
    assert_eq!(hover.extras.len(), 3, "must have 3 state extras");

    assert!(matches!(
        hover.extras[0].kind,
        StateExtraKind::Shape {
            shape: ShapeKind::Ellipse,
            ..
        }
    ));
    assert!((hover.extras[0].frame.x - 50.0).abs() <= 2.0);
    assert!((hover.extras[0].frame.y - 10.0).abs() <= 2.0);
    assert!((hover.extras[0].frame.w - 40.0).abs() <= 2.0);
    assert!((hover.extras[0].frame.h - 40.0).abs() <= 2.0);

    assert!(matches!(hover.extras[1].kind, StateExtraKind::Text { .. }));
    assert!(matches!(
        &hover.extras[2].kind,
        StateExtraKind::Path { path } if path.connector.is_none()
    ));
    assert_eq!(
        h.editor
            .ui
            .editing_state
            .as_ref()
            .and_then(|s| s.selected_extra.as_deref()),
        Some(hover.extras[2].id.as_str())
    );
}

#[test]
fn u14_insert_image_in_edit_state_mode_uses_pending_state_extra_image() {
    let mut h = TestHarness::new(course_with_button_and_hover_state());
    enter_hover_edit_mode(&mut h);

    let img_btn = h.find_text("Image").expect("Image toolbar button");
    let reqs = h.click_at_requests(img_btn);
    assert!(
        reqs.iter().any(|r| matches!(r, EditorRequest::PickImage)),
        "clicking Image must request PickImage"
    );
    assert_eq!(
        h.editor.pending_image_pick,
        Some(PendingImagePick::StateExtraImage {
            slide: "s1".to_owned(),
            object: "btn1".to_owned(),
            state: "hover".to_owned(),
        })
    );

    let imported = h.editor.import_image("badge.png", &tiny_png_bytes());
    assert_eq!(imported.as_deref(), Some("badge.png"));

    let (btn, _) = h
        .editor
        .session
        .course
        .slide("s1")
        .and_then(|s| s.object("btn1"))
        .expect("btn1");
    let hover = btn
        .states
        .iter()
        .find(|s| s.name == "hover")
        .expect("hover");
    assert_eq!(hover.extras.len(), 1);
    assert!(matches!(
        &hover.extras[0].kind,
        StateExtraKind::Image { src, .. } if src == "badge.png"
    ));
    assert_eq!(
        h.editor
            .ui
            .editing_state
            .as_ref()
            .and_then(|s| s.selected_extra.as_deref()),
        Some(hover.extras[0].id.as_str())
    );
}

#[test]
fn u14_non_visual_toolbar_buttons_are_disabled_inside_edit_state_mode() {
    let mut h = TestHarness::new(course_with_button_and_hover_state());
    enter_hover_edit_mode(&mut h);

    let toolbar_rect = egui::Rect::from_min_max(pos2(0.0, 0.0), pos2(1280.0, h.top_bars_height()));
    for label in ["Media", "Button", "Question", "Trigger"] {
        let pos = h.find_text_in(label, toolbar_rect).expect("toolbar button");
        let tip = h.probe_tooltip(pos);
        assert!(
            tip.iter()
                .any(|t| t.contains("Not available inside a state")),
            "hovering {label} in edit-state mode must show disabled tooltip, got {tip:?}"
        );
        h.click_at(pos);
        h.hover_at(pos2(0.0, 0.0));
        h.idle_frame();
    }

    assert_ne!(
        h.editor.ui.placement_tool,
        Some(PlacementTool::Button),
        "Button tool must not arm in edit-state mode"
    );
    assert!(
        h.editor.ui.trigger_form.is_none(),
        "Trigger form must not open in edit-state mode"
    );
}

#[test]
fn u15_click_extra_selects_and_click_parent_deselects_extra() {
    let mut h = TestHarness::new(course_with_hover_shape_extra());
    enter_hover_edit_mode(&mut h);

    // Extra is at parent (100,100) + (20,10,40,30) => slide (120,110)..(160,140).
    let extra_center = slide_to_screen(&h, pos2(140.0, 125.0));
    h.click_at(extra_center);
    h.idle_frame();
    assert_eq!(
        h.editor
            .ui
            .editing_state
            .as_ref()
            .and_then(|s| s.selected_extra.as_deref()),
        Some("extra_1")
    );

    // Click on parent outside extra at slide (280, 130).
    let parent_spot = slide_to_screen(&h, pos2(280.0, 130.0));
    h.click_at(parent_spot);
    h.idle_frame();
    assert_eq!(
        h.editor
            .ui
            .editing_state
            .as_ref()
            .and_then(|s| s.selected_extra.as_deref()),
        None
    );
}

#[test]
fn u15_drag_move_extra_updates_parent_relative_frame() {
    let mut h = TestHarness::new(course_with_hover_shape_extra());
    enter_hover_edit_mode(&mut h);

    let start = slide_to_screen(&h, pos2(140.0, 125.0));
    let end = slide_to_screen(&h, pos2(170.0, 145.0));
    h.drag_by(start, end - start, 4, Modifiers::NONE);
    h.idle_frame();

    let (btn, _) = h
        .editor
        .session
        .course
        .slide("s1")
        .and_then(|s| s.object("btn1"))
        .expect("btn1");
    assert_eq!(btn.frame.x, 100.0, "parent frame must stay locked");
    assert_eq!(btn.frame.y, 100.0);

    let extra = &btn.states[0].extras[0];
    assert!(
        (extra.frame.x - 50.0).abs() <= 2.0,
        "got x={}",
        extra.frame.x
    );
    assert!(
        (extra.frame.y - 30.0).abs() <= 2.0,
        "got y={}",
        extra.frame.y
    );
}

#[test]
fn u15_resize_and_rotate_extra_updates_extra_frame_and_rotation() {
    let mut h = TestHarness::new(course_with_hover_shape_extra());
    enter_hover_edit_mode(&mut h);
    if let Some(session) = h.editor.ui.editing_state.as_mut() {
        session.selected_extra = Some("extra_1".to_owned());
    }
    h.idle_frame();

    // Bottom-right corner of extra_1 is slide (160, 140). Drag by (+20, +10).
    let br = slide_to_screen(&h, pos2(160.0, 140.0));
    let br_end = slide_to_screen(&h, pos2(180.0, 150.0));
    h.drag_by(br, br_end - br, 4, Modifiers::NONE);
    h.idle_frame();

    let (btn, _) = h
        .editor
        .session
        .course
        .slide("s1")
        .and_then(|s| s.object("btn1"))
        .expect("btn1");
    let extra = &btn.states[0].extras[0];
    assert!(
        (extra.frame.w - 60.0).abs() <= 2.0,
        "got w={}",
        extra.frame.w
    );
    assert!(
        (extra.frame.h - 40.0).abs() <= 2.0,
        "got h={}",
        extra.frame.h
    );
    assert_eq!(btn.frame.w, 220.0);

    // Rotate handle is 24 screen px above top-center of extra_1.
    let top_center = slide_to_screen(
        &h,
        pos2(
            100.0 + extra.frame.x + extra.frame.w * 0.5,
            100.0 + extra.frame.y,
        ),
    );
    let rot_handle = pos2(top_center.x, top_center.y - 24.0);
    h.drag_by(rot_handle, egui::vec2(30.0, 20.0), 4, Modifiers::NONE);
    h.idle_frame();

    let (btn, _) = h
        .editor
        .session
        .course
        .slide("s1")
        .and_then(|s| s.object("btn1"))
        .expect("btn1");
    assert!(
        btn.states[0].extras[0].rotation.abs() > 1.0,
        "extra rotation must update"
    );
    assert_eq!(btn.rotation, 0.0, "parent rotation must remain unchanged");
}

#[test]
fn u15_arrow_nudge_and_delete_extra_with_undo_safety() {
    let mut h = TestHarness::new(course_with_hover_shape_extra());
    enter_hover_edit_mode(&mut h);
    if let Some(session) = h.editor.ui.editing_state.as_mut() {
        session.selected_extra = Some("extra_1".to_owned());
    }
    h.idle_frame();

    h.press(Key::ArrowRight, Modifiers::NONE);
    h.press(Key::ArrowDown, Modifiers::SHIFT);
    h.idle_frame();

    let (btn, _) = h
        .editor
        .session
        .course
        .slide("s1")
        .and_then(|s| s.object("btn1"))
        .expect("btn1");
    assert_eq!(btn.states[0].extras[0].frame.x, 21.0);
    assert_eq!(btn.states[0].extras[0].frame.y, 20.0);

    h.press(Key::Delete, Modifiers::NONE);
    h.idle_frame();

    let slide = h.editor.session.course.slide("s1").expect("slide");
    let (btn, _) = slide
        .object("btn1")
        .expect("parent btn1 must not be deleted");
    assert!(btn.states[0].extras.is_empty(), "extra_1 must be deleted");
    assert_eq!(
        h.editor
            .ui
            .editing_state
            .as_ref()
            .and_then(|s| s.selected_extra.as_deref()),
        None
    );

    h.editor.run("edit.undo", serde_json::json!({}));
    h.idle_frame();
    let (btn, _) = h
        .editor
        .session
        .course
        .slide("s1")
        .and_then(|s| s.object("btn1"))
        .expect("btn1");
    assert_eq!(btn.states[0].extras.len(), 1, "undo must restore extra_1");
}

#[test]
fn u16_items_in_this_state_list_selects_reorders_and_deletes_extras() {
    let mut course = course_with_hover_shape_extra();
    let second_extra: coursecraft_course::StateExtra = serde_json::from_value(serde_json::json!({
        "id": "extra_2",
        "name": "Caption",
        "kind": { "type": "text", "text": "Info" },
        "frame": { "x": 70.0, "y": 10.0, "w": 60.0, "h": 24.0 }
    }))
    .expect("valid extra_2");
    course.slide_mut("s1").expect("s1").objects[0].states[0]
        .extras
        .push(second_extra);

    let mut h = TestHarness::new(course);
    h.editor.enter_edit_state("s1", "btn1", "hover");
    h.editor.ui.inspector_tab = crate::InspectorTab::Format;
    h.idle_frame();

    assert!(
        h.find_text("Items in this state").is_some(),
        "Format inspector must show 'Items in this state' when extras exist"
    );
    let caption_pos = h
        .find_text_containing("Caption")
        .expect("Caption row in Items in this state");
    h.click_at(caption_pos);
    h.idle_frame();
    assert_eq!(
        h.editor
            .ui
            .editing_state
            .as_ref()
            .and_then(|s| s.selected_extra.as_deref()),
        Some("extra_2"),
        "clicking item in list must select extra_2"
    );

    let up_pos = h.find_text_last("↑").expect("move up button for extra_2");
    h.click_at(up_pos);
    h.idle_frame();
    let extras = &h.editor.session.course.slide("s1").expect("s1").objects[0].states[0].extras;
    assert_eq!(extras[0].id, "extra_2", "↑ must reorder extra_2 to index 0");

    let badge_pos = h
        .find_text_containing("Badge")
        .expect("Badge row in Items in this state");
    let row_rect = egui::Rect::from_min_max(
        pos2(badge_pos.x, badge_pos.y - 16.0),
        pos2(badge_pos.x + 300.0, badge_pos.y + 16.0),
    );
    let del_pos = h
        .find_text_in("×", row_rect)
        .expect("delete button for extra_1");
    h.click_at(del_pos);
    h.idle_frame();
    let extras = &h.editor.session.course.slide("s1").expect("s1").objects[0].states[0].extras;
    assert_eq!(extras.len(), 1);
    assert_eq!(extras[0].id, "extra_2");
    assert_eq!(
        h.editor
            .ui
            .editing_state
            .as_ref()
            .and_then(|s| s.selected_extra.as_deref()),
        Some("extra_2")
    );
}

#[test]
fn u16_inspector_routes_selected_extra_to_state_update_extra() {
    let mut h = TestHarness::new(course_with_hover_shape_extra());
    h.editor.enter_edit_state("s1", "btn1", "hover");
    if let Some(session) = h.editor.ui.editing_state.as_mut() {
        session.selected_extra = Some("extra_1".to_owned());
    }
    h.editor.ui.inspector_tab = crate::InspectorTab::Format;
    h.idle_frame();

    assert!(
        h.find_text("extra_1").is_some(),
        "Format inspector must show selected extra ID"
    );
    assert!(
        h.find_text("Edit parent").is_some(),
        "Format inspector must offer 'Edit parent' when an extra is selected"
    );
    // Editing extra name in inspector routes to object.state_update_extra, leaving parent name untouched.
    let name_pos = h.find_text("Badge").expect("extra name TextEdit");
    h.click_at(name_pos);
    h.press(Key::A, Modifiers::COMMAND);
    h.type_text("Star Badge");
    h.press(Key::Enter, Modifiers::NONE);
    h.idle_frame();

    let obj = &h.editor.session.course.slide("s1").expect("s1").objects[0];
    assert_eq!(
        obj.name, "Primary Button",
        "parent name must stay untouched"
    );
    assert_eq!(obj.states[0].extras[0].name, "Star Badge");
}

#[test]
fn u16_double_click_extra_inline_text_edit_commits_to_state_update_extra() {
    let mut course = course_with_button_and_hover_state();
    let text_extra: coursecraft_course::StateExtra = serde_json::from_value(serde_json::json!({
        "id": "extra_1",
        "name": "Status Label",
        "kind": { "type": "text", "text": "Old Badge" },
        "frame": { "x": 20.0, "y": 10.0, "w": 120.0, "h": 30.0 }
    }))
    .expect("valid text extra");
    course.slide_mut("s1").expect("s1").objects[0].states[0]
        .extras
        .push(text_extra);

    let mut h = TestHarness::new(course);
    h.editor.enter_edit_state("s1", "btn1", "hover");
    h.idle_frame();

    let extra_center = slide_to_screen(&h, pos2(100.0 + 20.0 + 60.0, 100.0 + 10.0 + 15.0));
    h.double_click_at(extra_center);
    h.idle_frame();

    let edit = h
        .editor
        .ui
        .inline_edit
        .as_mut()
        .expect("double-clicking text extra must open InlineEdit");
    assert_eq!(edit.state_name.as_deref(), Some("hover"));
    assert_eq!(edit.extra_id.as_deref(), Some("extra_1"));
    assert_eq!(edit.buffer, "Old Badge");

    edit.buffer = "New Badge".to_owned();
    crate::canvas::commit_inline_edit(&mut h.editor);
    h.idle_frame();

    let obj = &h.editor.session.course.slide("s1").expect("s1").objects[0];
    match &obj.kind {
        coursecraft_course::ObjectKind::Button { label, .. } => {
            assert_eq!(label, "Click Me", "parent button label must stay untouched");
        }
        other => panic!("expected button, got {other:?}"),
    }
    match &obj.states[0].extras[0].kind {
        StateExtraKind::Text { text, .. } => assert_eq!(text, "New Badge"),
        other => panic!("expected text extra, got {other:?}"),
    }

    h.editor.run("edit.undo", serde_json::json!({}));
    let obj = &h.editor.session.course.slide("s1").expect("s1").objects[0];
    match &obj.states[0].extras[0].kind {
        StateExtraKind::Text { text, .. } => assert_eq!(text, "Old Badge"),
        other => panic!("expected text extra, got {other:?}"),
    }
}

#[test]
fn u16_extra_context_menu_duplicate_reorder_and_delete() {
    let mut h = TestHarness::new(course_with_hover_shape_extra());
    h.editor.enter_edit_state("s1", "btn1", "hover");
    h.idle_frame();

    let extra_center = slide_to_screen(&h, pos2(100.0 + 20.0 + 20.0, 100.0 + 10.0 + 15.0));
    h.right_click_at(extra_center);
    h.idle_frame();

    let dup_pos = h
        .find_text("Duplicate extra")
        .expect("Duplicate extra in extra context menu");
    assert!(h.find_text("Bring forward").is_some());
    assert!(h.find_text("Send backward").is_some());
    assert!(h.find_text("Delete extra").is_some());

    h.click_at(dup_pos);
    h.idle_frame();

    let extras = &h.editor.session.course.slide("s1").expect("s1").objects[0].states[0].extras;
    assert_eq!(extras.len(), 2, "Duplicate extra must add a second extra");
    assert_eq!(
        h.editor
            .ui
            .editing_state
            .as_ref()
            .and_then(|s| s.selected_extra.as_deref()),
        Some(extras[1].id.as_str())
    );
}

#[test]
fn u16_live_state_thumbnail_composes_state_extras() {
    let mut course = course_with_hover_shape_extra();
    // Make extra overhang outside parent bounds so composed bounds differ if extras are included.
    course.slide_mut("s1").expect("s1").objects[0].states[0].extras[0].frame =
        coursecraft_course::Rect {
            x: 240.0,
            y: -20.0,
            w: 40.0,
            h: 40.0,
        };
    let obj = &course.slide("s1").expect("s1").objects[0];
    let items = crate::states::compose_thumbnail_items(&course, obj, "hover");
    assert_eq!(
        items.len(),
        2,
        "state thumbnail composition must include parent + StateExtra items"
    );
}

#[test]
fn u17_find_hit_in_state_extra_text_opens_states_tab_and_previews_state() {
    let mut course = course_with_button_and_hover_state();
    course.slide_mut("s1").expect("s1").objects[0].states.push(
        serde_json::from_value(serde_json::json!({
            "name": "visited",
            "extras": [{
                "id": "e1",
                "kind": { "type": "text", "text": "Checkmark Done" },
                "frame": { "x": 10.0, "y": 10.0, "w": 80.0, "h": 24.0 }
            }]
        }))
        .expect("visited state"),
    );

    let mut h = TestHarness::new(course);
    h.editor.ui.bottom_tab = crate::BottomTab::Timeline;
    h.editor.ui.preview_state = None;
    h.editor.ui.find.open = true;
    h.editor.ui.find.query = "Checkmark Done".to_owned();
    h.idle_frame();

    let hit_pos = h
        .find_text_last("Checkmark Done")
        .expect("Find result row for Checkmark Done");
    h.click_at(hit_pos);
    h.idle_frame();

    assert_eq!(h.editor.selection.slide, "s1");
    assert_eq!(h.editor.selection.object.as_deref(), Some("btn1"));
    assert_eq!(h.editor.ui.bottom_tab, crate::BottomTab::States);
    assert_eq!(h.editor.ui.preview_state.as_deref(), Some("visited"));
}

#[test]
fn u18_undo_past_state_add_extra_clears_selected_extra_without_panic() {
    let mut h = TestHarness::new(course_with_button_and_hover_state());
    h.editor.enter_edit_state("s1", "btn1", "hover");
    let res = h
        .editor
        .run(
            "object.state_add_extra",
            serde_json::json!({
                "slide": "s1",
                "object": "btn1",
                "state": "hover",
                "extra": {
                    "kind": { "type": "shape", "shape": "rectangle", "fill": "#FF0000" },
                    "frame": { "x": 10.0, "y": 10.0, "w": 30.0, "h": 30.0 }
                }
            }),
        )
        .expect("add extra");
    let new_id = res["id"].as_str().expect("id").to_owned();
    if let Some(session) = h.editor.ui.editing_state.as_mut() {
        session.selected_extra = Some(new_id);
    }
    h.idle_frame();

    h.editor.run("edit.undo", serde_json::json!({}));
    h.idle_frame();

    let session = h
        .editor
        .ui
        .editing_state
        .as_ref()
        .expect("editing_state must remain active after undoing extra add");
    assert_eq!(
        session.selected_extra, None,
        "undoing past object.state_add_extra must clear selected_extra"
    );
}

#[test]
fn u19_used_media_and_media_library_places_include_state_extra_image() {
    let mut course = course_with_button_and_hover_state();
    course.slide_mut("s1").expect("s1").objects[0].states.push(
        serde_json::from_value(serde_json::json!({
            "name": "visited",
            "extras": [{
                "id": "e_img",
                "kind": { "type": "image", "src": "media/badge.png", "alt": "Done" },
                "frame": { "x": 10.0, "y": 10.0, "w": 24.0, "h": 24.0 }
            }]
        }))
        .expect("visited state"),
    );

    let mut h = TestHarness::new(course);
    let pixels = vec![42u8; 4 * 4 * 4];
    let bytes = coursecraft_media::encode_png_rgba(4, 4, &pixels).expect("png");
    h.editor.set_media(vec![(
        "media/badge.png".to_owned(),
        std::sync::Arc::from(bytes.as_slice()),
    )]);

    let used = h.editor.used_media();
    assert!(
        used.iter().any(|(src, _)| src == "media/badge.png"),
        "used_media() must keep media/badge.png referenced only by state.extras"
    );

    h.editor.ui.show_media_library = true;
    h.editor.ui.media_library.selected = Some("media/badge.png".to_owned());
    h.idle_frame();

    assert!(
        h.find_text("Places using this file").is_some(),
        "Media Library must show Places using this file for state extra image"
    );
}

#[test]
fn b04_extra_inspector_cache_key_includes_state_name() {
    let mut course = course_with_button_and_hover_state();
    let extra_1: coursecraft_course::StateExtra = serde_json::from_value(serde_json::json!({
        "id": "extra_1",
        "name": "Hover Badge",
        "kind": { "type": "text", "text": "Hover" },
        "frame": { "x": 10.0, "y": 10.0, "w": 80.0, "h": 24.0 }
    }))
    .expect("valid text extra");
    course.slide_mut("s1").expect("s1").objects[0].states[0]
        .extras
        .push(extra_1);

    let mut h = TestHarness::new(course);
    h.editor
        .run(
            "object.duplicate_state",
            serde_json::json!({
                "slide": "s1",
                "object": "btn1",
                "name": "hover",
                "new_name": "visited",
            }),
        )
        .expect("duplicate_state");
    h.editor
        .run(
            "object.state_update_extra",
            serde_json::json!({
                "slide": "s1",
                "object": "btn1",
                "state": "visited",
                "extra": "extra_1",
                "patch": {
                    "frame": { "x": 90.0, "y": 10.0, "w": 80.0, "h": 24.0 },
                    "kind": { "type": "text", "text": "Visited" },
                }
            }),
        )
        .expect("state_update_extra");

    h.editor.enter_edit_state("s1", "btn1", "hover");
    if let Some(session) = h.editor.ui.editing_state.as_mut() {
        session.selected_extra = Some("extra_1".to_owned());
    }
    h.idle_frame();
    assert_eq!(h.editor.ui.properties.extra_x, 10.0);
    assert_eq!(h.editor.ui.properties.extra_text, "Hover");

    // Switch to "visited" at the exact same course revision.
    let rev_before = h.editor.session.revision;
    if let Some(session) = h.editor.ui.editing_state.as_mut() {
        session.state_name = "visited".to_owned();
        session.selected_extra = Some("extra_1".to_owned());
    }
    h.editor.ui.preview_state = Some("visited".to_owned());
    h.idle_frame();

    assert_eq!(h.editor.session.revision, rev_before);
    assert_eq!(
        h.editor.ui.properties.extra_x, 90.0,
        "inspector must refresh extra_x when switching states at the same revision"
    );
    assert_eq!(
        h.editor.ui.properties.extra_text, "Visited",
        "inspector must refresh extra_text when switching states at the same revision"
    );
}

#[test]
fn b06_state_extra_placement_inline_edit_and_rotate_preview_respect_parent_transform() {
    let mut course = course_with_button_and_hover_state();
    {
        let obj = &mut course.slide_mut("s1").expect("s1").objects[0];
        obj.frame = coursecraft_course::Rect {
            x: 300.0,
            y: 200.0,
            w: 200.0,
            h: 80.0,
        };
        obj.rotation = 90.0;
        obj.flip_h = true;
    }

    let mut h = TestHarness::new(course);
    h.editor.enter_edit_state("s1", "btn1", "hover");
    h.idle_frame();

    // 1. Place a Shape extra on the 90°-rotated, horizontally-flipped parent at slide point P.
    let p_shape = pos2(620.0, 260.0);
    let shape_size = vec2(80.0, 40.0);
    h.editor.ui.placement_tool = Some(PlacementTool::Shape(
        coursecraft_course::ShapeKind::Rectangle,
    ));
    let drag_start = slide_to_screen(&h, p_shape - shape_size * 0.5);
    let drag_end = slide_to_screen(&h, p_shape + shape_size * 0.5);
    h.drag_by(drag_start, drag_end - drag_start, 5, Modifiers::NONE);
    h.idle_frame();

    let obj = &h.editor.session.course.slide("s1").expect("s1").objects[0];
    let shape_extra = obj.states[0].extras.last().expect("placed shape extra");
    let (shape_world, shape_rot) =
        crate::canvas::state_extra_world_geometry(obj, "hover", shape_extra);
    assert!(
        (shape_world.center().x - p_shape.x).abs() <= 2.0
            && (shape_world.center().y - p_shape.y).abs() <= 2.0,
        "placed shape extra world center {:?} must match {p_shape:?}",
        shape_world.center()
    );
    assert!(
        shape_rot.abs() < 0.01 || (shape_rot - 360.0).abs() < 0.01,
        "placed shape extra world rotation must be 0, got {shape_rot}"
    );
    let picked = crate::canvas::pick_state_extra(&h.editor, p_shape);
    assert_eq!(
        picked.as_ref().map(|(id, _, _)| id.as_str()),
        Some(shape_extra.id.as_str()),
        "pick_state_extra must hit newly placed shape extra at {p_shape:?}"
    );

    // 2. Place a Path extra via Scribble draw tool at slide point P2.
    let p_path = pos2(700.0, 340.0);
    h.editor.ui.placement_tool = Some(PlacementTool::Draw(DrawTool::Scribble));
    let path_start = slide_to_screen(&h, pos2(p_path.x - 40.0, p_path.y - 20.0));
    let path_end = slide_to_screen(&h, pos2(p_path.x + 40.0, p_path.y + 20.0));
    h.drag_by(path_start, path_end - path_start, 6, Modifiers::NONE);
    h.idle_frame();

    let obj = &h.editor.session.course.slide("s1").expect("s1").objects[0];
    assert_eq!(obj.states[0].extras.len(), 2, "path extra must be added");
    let path_extra = &obj.states[0].extras[1];
    let (path_world, _) = crate::canvas::state_extra_world_geometry(obj, "hover", path_extra);
    assert!(
        (path_world.center().x - p_path.x).abs() <= 2.0
            && (path_world.center().y - p_path.y).abs() <= 2.0,
        "placed path extra world center {:?} must match {p_path:?}",
        path_world.center()
    );
    let picked_path = crate::canvas::pick_state_extra(&h.editor, p_path);
    assert_eq!(
        picked_path.as_ref().map(|(id, _, _)| id.as_str()),
        Some(path_extra.id.as_str()),
        "pick_state_extra must hit newly placed path extra at {p_path:?}"
    );

    // 3. Double-click a Text extra on the rotated+flipped parent -> inline_edit_slide_rect uses world geometry.
    let p_text = pos2(520.0, 160.0);
    h.editor.ui.placement_tool = Some(PlacementTool::TextBox);
    let text_start = slide_to_screen(&h, pos2(p_text.x - 50.0, p_text.y - 16.0));
    let text_end = slide_to_screen(&h, pos2(p_text.x + 50.0, p_text.y + 16.0));
    h.drag_by(text_start, text_end - text_start, 5, Modifiers::NONE);
    h.idle_frame();

    let text_world = {
        let obj = &h.editor.session.course.slide("s1").expect("s1").objects[0];
        let text_extra = obj.states[0].extras.last().expect("text extra");
        crate::canvas::state_extra_world_geometry(obj, "hover", text_extra).0
    };
    h.double_click_at(slide_to_screen(&h, text_world.center()));
    h.idle_frame();

    let obj = &h.editor.session.course.slide("s1").expect("s1").objects[0];
    let edit = h
        .editor
        .ui
        .inline_edit
        .as_ref()
        .expect("double-clicking text extra must open InlineEdit");
    let edit_rect = crate::canvas::inline_edit_slide_rect(obj, edit).expect("inline edit rect");
    assert!(
        (edit_rect.center().x - text_world.center().x).abs() <= 1.0
            && (edit_rect.center().y - text_world.center().y).abs() <= 1.0,
        "inline_edit_slide_rect center {:?} must match world rect center {:?}",
        edit_rect.center(),
        text_world.center()
    );
    h.editor.ui.inline_edit = None;

    // 4. Live parent rotate preview must leave is_state_extra item rotations intact.
    h.editor.exit_edit_state();
    let obj = &h.editor.session.course.slide("s1").expect("s1").objects[0];
    let mut items = coursecraft_render::compose_object_items(
        obj,
        "hover",
        Some(&h.editor.session.course.theme),
        crate::states::course_input_value(&h.editor.session.course, obj),
    );
    let initial_extra_rotations: Vec<f32> = items
        .iter()
        .filter(|it| it.is_state_extra)
        .map(|it| it.rotation)
        .collect();
    assert_eq!(initial_extra_rotations.len(), 3);

    crate::canvas::apply_rotate_preview(&mut items, "btn1", 135.0);

    let parent_item = items
        .iter()
        .find(|it| !it.is_state_extra && it.hit == coursecraft_render::Hit::Object("btn1".into()))
        .expect("parent draw item");
    assert_eq!(parent_item.rotation, 135.0);
    let after_extra_rotations: Vec<f32> = items
        .iter()
        .filter(|it| it.is_state_extra)
        .map(|it| it.rotation)
        .collect();
    assert_eq!(
        after_extra_rotations, initial_extra_rotations,
        "live parent rotate preview must not overwrite is_state_extra item rotations"
    );
}
