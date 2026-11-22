//! M29 Step s10a tests: canvas insert, select, drag, resize, rotate, nudge,
//! and delete of `StateExtra`s inside Edit-State Mode (`U-14`, `U-15`).

use crate::test_harness::TestHarness;
use crate::{DrawTool, EditorRequest, PendingImagePick, PlacementTool};
use coursecraft_course::{Course, ShapeKind, StateExtraKind};
use egui::{Key, Modifiers, Pos2, pos2};

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
