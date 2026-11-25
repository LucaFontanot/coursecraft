//! M37 Step s5 UI tests (`PT-08`–`PT-10`):
//! - `pt_08`: Canvas click vs drag placement for point text vs area text, plus live inline growth.
//! - `pt_09`: State Editor click vs drag placement for point text vs area text extras.
//! - `pt_10`: Inspector `"Wrap text"` checkbox and East resize handle conversion + 1-step undo.

use crate::properties::InspectorTab;
use crate::test_harness::TestHarness;
use crate::{PlacementTool, canvas_fit_rect, canvas_rect};
use coursecraft_course::{Course, ObjectKind, StateExtraKind};
use coursecraft_render::SlideTransform;
use egui::{Key, Modifiers, Pos2, pos2, vec2};

fn empty_slide_course() -> Course {
    let raw = serde_json::json!({
        "schema_version": 1,
        "id": "m37-s5-test",
        "title": "M37 S5 Point Text Test",
        "slide_size": { "w": 1280, "h": 720 },
        "theme": {
            "accent": "#1166CC",
            "text": "#222222",
            "background": "#FFFFFF",
            "font": "Inter"
        },
        "scenes": [{
            "id": "sc1",
            "title": "Main",
            "slides": [{
                "id": "s1",
                "title": "Slide 1",
                "objects": []
            }]
        }]
    });
    serde_json::from_value(raw).expect("valid course")
}

fn button_hover_course() -> Course {
    let raw = serde_json::json!({
        "schema_version": 1,
        "id": "m37-s5-state-test",
        "title": "M37 S5 State Extra Point Text Test",
        "slide_size": { "w": 1280, "h": 720 },
        "theme": {
            "accent": "#1166CC",
            "text": "#222222",
            "background": "#FFFFFF",
            "font": "Inter"
        },
        "scenes": [{
            "id": "sc1",
            "title": "Main",
            "slides": [{
                "id": "s1",
                "title": "Slide 1",
                "objects": [{
                    "id": "btn1",
                    "name": "Button 1",
                    "frame": { "x": 120.0, "y": 120.0, "w": 240.0, "h": 72.0 },
                    "kind": { "type": "button", "label": "Next", "fill": "#1166CC" },
                    "states": [
                        { "name": "hover", "fill": "#0E4F9E" }
                    ]
                }]
            }]
        }]
    });
    serde_json::from_value(raw).expect("valid course")
}

fn slide_transform(h: &TestHarness) -> SlideTransform {
    SlideTransform::fit(
        canvas_fit_rect(canvas_rect(h.central_panel_rect())),
        h.editor.session.course.slide_size,
    )
    .expect("slide transform")
}

fn slide_to_screen(h: &TestHarness, slide_pt: Pos2) -> Pos2 {
    let slide_rect = h.editor.ui.last_slide_rect.expect("last_slide_rect");
    pos2(
        slide_rect.min.x + slide_rect.width() * (slide_pt.x / 1280.0),
        slide_rect.min.y + slide_rect.height() * (slide_pt.y / 720.0),
    )
}

fn painted_galley_rows(shapes: &[egui::epaint::ClippedShape], text: &str) -> Option<usize> {
    fn walk(shape: &egui::epaint::Shape, text: &str) -> Option<usize> {
        match shape {
            egui::epaint::Shape::Text(t) if t.galley.text() == text => Some(t.galley.rows.len()),
            egui::epaint::Shape::Vec(v) => v.iter().find_map(|s| walk(s, text)),
            _ => None,
        }
    }
    shapes.iter().find_map(|cs| walk(&cs.shape, text))
}

#[test]
fn pt_08_canvas_click_places_point_text_and_drag_places_area_text() {
    let mut h = TestHarness::new(empty_slide_course());
    h.editor.ui.snap_to_guides = false;
    h.idle_frame();

    // 1. Click with PlacementTool::TextBox creates fitted point text and opens inline edit.
    h.editor.ui.placement_tool = Some(PlacementTool::TextBox);
    h.idle_frame();
    let t = slide_transform(&h);
    let click_slide = pos2(300.0, 220.0);
    h.click_at(t.to_screen_pos(click_slide));
    h.idle_frame();

    let edit = h
        .editor
        .ui
        .inline_edit
        .as_ref()
        .expect("clicking TextBox must enter inline edit")
        .clone();
    assert_eq!(edit.buffer, "Text");

    let initial_obj = h
        .editor
        .session
        .course
        .slide("s1")
        .and_then(|s| s.object(&edit.object))
        .map(|(o, _)| o.clone())
        .expect("placed point text");
    match &initial_obj.kind {
        ObjectKind::Text {
            text,
            style,
            point_text,
            ..
        } => {
            assert_eq!(text, "Text");
            assert!(*point_text, "click placement must set point_text = true");
            assert!(!style.wrap, "click placement must set style.wrap = false");
        }
        other => panic!("expected Text, got {other:?}"),
    }
    assert!(
        initial_obj.frame.w < 120.0 && initial_obj.frame.h < 45.0,
        "initial fitted frame should be tight around 'Text', got {:?}",
        initial_obj.frame
    );
    assert!(
        (initial_obj.frame.x - click_slide.x).abs() <= 2.0,
        "point text left edge {:?} should anchor near click x {}",
        initial_obj.frame.x,
        click_slide.x
    );
    assert!(
        (initial_obj.frame.y - (click_slide.y - initial_obj.frame.h * 0.5)).abs() <= 2.0,
        "point text vertical center should anchor near click y {}, frame={:?}",
        click_slide.y,
        initial_obj.frame
    );

    // 2. Typing a long single-line string grows the live inline rect without wrapping.
    let long_label = "A much longer single line label";
    h.type_text(long_label);
    let shapes = h.frame_shapes(Vec::new());
    let grown_rect = h
        .editor
        .ui
        .inline_edit_live_rect
        .expect("live inline rect while editing point text");
    assert!(
        grown_rect.width() > initial_obj.frame.w * 3.0,
        "live inline rect must grow for long text: grown={}, initial={}",
        grown_rect.width(),
        initial_obj.frame.w
    );
    assert!(
        grown_rect.height() < 45.0,
        "live inline rect must stay single-line height, got {}",
        grown_rect.height()
    );
    assert_eq!(
        painted_galley_rows(&shapes, long_label),
        Some(1),
        "point text inline editor must keep long text on one row without wrapping"
    );

    // 3. Replacing with "Next" shrinks the live inline rect and commits a fitted frame.
    h.press(Key::A, Modifiers::COMMAND);
    h.type_text("Next");
    h.idle_frame();
    let shrunk_rect = h
        .editor
        .ui
        .inline_edit_live_rect
        .expect("live inline rect after typing Next");
    assert!(
        shrunk_rect.width() < 120.0 && shrunk_rect.width() < grown_rect.width(),
        "live inline rect must shrink back for 'Next', got {shrunk_rect:?}"
    );
    assert!(
        shrunk_rect.height() < 45.0,
        "shrunk inline rect height must stay single line, got {}",
        shrunk_rect.height()
    );

    h.press(Key::Enter, Modifiers::COMMAND);
    h.idle_frame();
    assert!(h.editor.ui.inline_edit.is_none());

    let committed = h
        .editor
        .session
        .course
        .slide("s1")
        .and_then(|s| s.object(&edit.object))
        .map(|(o, _)| o.clone())
        .expect("committed point text");
    match &committed.kind {
        ObjectKind::Text {
            text,
            style,
            point_text,
            ..
        } => {
            assert_eq!(text, "Next");
            assert!(*point_text);
            assert!(!style.wrap);
        }
        other => panic!("expected Text, got {other:?}"),
    }
    assert!(
        committed.frame.w < 120.0 && committed.frame.h < 45.0,
        "committed 'Next' frame must be fitted (<120x45), got {:?}",
        committed.frame
    );

    // 4. Dragging with PlacementTool::TextBox creates fixed-width wrapping area text.
    h.editor.ui.placement_tool = Some(PlacementTool::TextBox);
    h.idle_frame();
    let t = slide_transform(&h);
    let scale = t.scale();
    let drag_start = t.to_screen_pos(pos2(300.0, 360.0));
    h.drag_by(
        drag_start,
        vec2(260.0 * scale, 80.0 * scale),
        4,
        Modifiers::NONE,
    );
    h.idle_frame();

    let drag_edit = h
        .editor
        .ui
        .inline_edit
        .as_ref()
        .expect("dragged TextBox enters inline edit")
        .clone();
    let area_obj = h
        .editor
        .session
        .course
        .slide("s1")
        .and_then(|s| s.object(&drag_edit.object))
        .map(|(o, _)| o.clone())
        .expect("dragged area text");
    match &area_obj.kind {
        ObjectKind::Text {
            style, point_text, ..
        } => {
            assert!(!*point_text, "drag placement must set point_text = false");
            assert!(style.wrap, "drag placement must keep style.wrap = true");
        }
        other => panic!("expected Text, got {other:?}"),
    }
    assert!(
        (area_obj.frame.w - 260.0).abs() <= 2.0,
        "dragged area text must keep dragged width (~260), got {}",
        area_obj.frame.w
    );
    assert!(
        (area_obj.frame.h - 80.0).abs() <= 2.0,
        "dragged area text must keep dragged height (~80), got {}",
        area_obj.frame.h
    );
}

#[test]
fn pt_09_state_editor_click_places_point_text_extra_and_drag_places_area_extra() {
    let mut h = TestHarness::new(button_hover_course());
    h.editor.ui.snap_to_guides = false;
    h.editor.selection.set_many(["btn1".to_owned()]);
    h.editor.enter_edit_state("s1", "btn1", "hover");
    h.idle_frame();

    // 1. Click with PlacementTool::TextBox inside State Editor -> fitted point text extra.
    h.editor.ui.placement_tool = Some(PlacementTool::TextBox);
    h.idle_frame();
    let click_pt = slide_to_screen(&h, pos2(160.0, 150.0));
    h.click_at(click_pt);
    h.idle_frame();

    let click_extra = {
        let slide = h.editor.session.course.slide("s1").expect("slide");
        let (btn, _) = slide.object("btn1").expect("btn1");
        let hover = btn
            .states
            .iter()
            .find(|s| s.name == "hover")
            .expect("hover");
        assert_eq!(hover.extras.len(), 1);
        hover.extras[0].clone()
    };
    match &click_extra.kind {
        StateExtraKind::Text {
            text,
            style,
            point_text,
            ..
        } => {
            assert_eq!(text, "Text");
            assert!(
                *point_text,
                "state editor click must create point_text extra"
            );
            assert!(!style.wrap, "state editor click must set wrap = false");
        }
        other => panic!("expected StateExtraKind::Text, got {other:?}"),
    }
    assert!(
        click_extra.frame.w < 120.0 && click_extra.frame.h < 45.0,
        "clicked state extra must be fitted, got {:?}",
        click_extra.frame
    );

    // Double-click the point-text extra, type a longer label, then commit.
    let world_rect = {
        let slide = h.editor.session.course.slide("s1").expect("slide");
        let (btn, _) = slide.object("btn1").expect("btn1");
        crate::canvas::state_extra_world_geometry(btn, "hover", &click_extra).0
    };
    h.advance_time(1.0);
    assert!(
        crate::canvas::pick_state_extra(&h.editor, world_rect.center()).is_some(),
        "pick_state_extra must hit {world_rect:?}"
    );
    h.double_click_at(slide_to_screen(&h, world_rect.center()));
    h.idle_frame();
    assert!(
        h.editor.ui.inline_edit.is_some(),
        "double-clicking text extra must open inline edit"
    );
    h.type_text("Go forward now");
    h.idle_frame();
    let live_rect = h
        .editor
        .ui
        .inline_edit_live_rect
        .expect("live rect for state extra point text");
    assert!(
        live_rect.width() > click_extra.frame.w * 1.8,
        "state extra point text must grow live while typing: {} vs {}",
        live_rect.width(),
        click_extra.frame.w
    );
    h.press(Key::Enter, Modifiers::COMMAND);
    h.idle_frame();
    assert!(h.editor.ui.inline_edit.is_none());

    let updated_extra = {
        let slide = h.editor.session.course.slide("s1").expect("slide");
        let (btn, _) = slide.object("btn1").expect("btn1");
        let hover = btn
            .states
            .iter()
            .find(|s| s.name == "hover")
            .expect("hover");
        hover.extras[0].clone()
    };
    assert!(
        updated_extra.frame.w > click_extra.frame.w * 1.8,
        "committed state extra frame must grow to fit 'Go forward now': {:?} vs {:?}",
        updated_extra.frame,
        click_extra.frame
    );

    // 2. Drag with PlacementTool::TextBox inside State Editor -> fixed-width wrapping area extra.
    h.editor.ui.placement_tool = Some(PlacementTool::TextBox);
    h.idle_frame();
    let d1 = slide_to_screen(&h, pos2(420.0, 200.0));
    let d2 = slide_to_screen(&h, pos2(640.0, 270.0));
    h.drag_by(d1, d2 - d1, 4, Modifiers::NONE);
    h.idle_frame();

    let drag_extra = {
        let slide = h.editor.session.course.slide("s1").expect("slide");
        let (btn, _) = slide.object("btn1").expect("btn1");
        let hover = btn
            .states
            .iter()
            .find(|s| s.name == "hover")
            .expect("hover");
        assert_eq!(hover.extras.len(), 2);
        hover.extras[1].clone()
    };
    match &drag_extra.kind {
        StateExtraKind::Text {
            style, point_text, ..
        } => {
            assert!(
                !*point_text,
                "state editor drag must create area text (point_text = false)"
            );
            assert!(style.wrap, "state editor drag must keep wrap = true");
        }
        other => panic!("expected StateExtraKind::Text, got {other:?}"),
    }
    assert!(
        (drag_extra.frame.w - 220.0).abs() <= 2.0,
        "dragged state extra must keep width ~220, got {}",
        drag_extra.frame.w
    );
    assert!(
        (drag_extra.frame.h - 70.0).abs() <= 2.0,
        "dragged state extra must keep height ~70, got {}",
        drag_extra.frame.h
    );
}

#[test]
fn pt_10_inspector_wrap_and_width_handle_convert_point_text() {
    let mut h = TestHarness::new(empty_slide_course());
    h.editor.ui.snap_to_guides = false;
    h.idle_frame();

    // Place a point-text object by clicking with PlacementTool::TextBox.
    h.editor.ui.placement_tool = Some(PlacementTool::TextBox);
    h.idle_frame();
    let t = slide_transform(&h);
    h.click_at(t.to_screen_pos(pos2(300.0, 240.0)));
    h.idle_frame();
    let obj_id = h
        .editor
        .ui
        .inline_edit
        .as_ref()
        .expect("inline edit")
        .object
        .clone();
    // Dismiss inline edit so the point-text object is cleanly selected.
    h.press(Key::Escape, Modifiers::NONE);
    h.editor.ui.inspector_tab = InspectorTab::Format;
    h.idle_frame();
    h.idle_frame();

    let original_frame = h
        .editor
        .session
        .course
        .slide("s1")
        .and_then(|s| s.object(&obj_id))
        .map(|(o, _)| o.frame)
        .expect("placed object");

    // 1. Inspector "Wrap text" tooltip explains point text and clicking converts in 1 undo step.
    let right = h.right_panel_rect();
    let wrap_pos = h
        .find_text_in("Wrap text", right)
        .expect("Wrap text checkbox in inspector");
    let tip = h.probe_tooltip(wrap_pos);
    assert!(
        tip.iter().any(|t| {
            t.contains("Fits width to text. Turn on wrap or drag a side handle for fixed width.")
        }),
        "expected point-text tooltip on Wrap text row, got {tip:?}"
    );

    let undo_before_wrap = h.editor.undo_len_for_test();
    h.click_at(wrap_pos);
    h.idle_frame();
    assert_eq!(
        h.editor.undo_len_for_test(),
        undo_before_wrap + 1,
        "checking Wrap text must be 1 undo step"
    );
    {
        let (obj, _) = h
            .editor
            .session
            .course
            .slide("s1")
            .and_then(|s| s.object(&obj_id))
            .expect("object after wrap");
        let ObjectKind::Text {
            style, point_text, ..
        } = &obj.kind
        else {
            panic!("expected Text");
        };
        assert!(!*point_text, "checking Wrap text must clear point_text");
        assert!(style.wrap, "checking Wrap text must set wrap = true");
    }

    // Ctrl+Z restores point_text = true, wrap = false in 1 undo step.
    h.press(Key::Z, Modifiers::COMMAND);
    h.idle_frame();
    {
        let (obj, _) = h
            .editor
            .session
            .course
            .slide("s1")
            .and_then(|s| s.object(&obj_id))
            .expect("object after undo wrap");
        let ObjectKind::Text {
            style, point_text, ..
        } = &obj.kind
        else {
            panic!("expected Text");
        };
        assert!(
            *point_text,
            "Ctrl+Z after Wrap text must restore point_text = true"
        );
        assert!(
            !style.wrap,
            "Ctrl+Z after Wrap text must restore wrap = false"
        );
    }

    // 2. Dragging the East (Right) resize handle converts point text to area text in 1 undo step.
    h.editor.selection.set_many([obj_id.clone()]);
    h.idle_frame();
    let t = slide_transform(&h);
    let scale = t.scale();
    let east_handle_slide = pos2(
        original_frame.x + original_frame.w,
        original_frame.y + original_frame.h * 0.5,
    );
    let east_handle_screen = t.to_screen_pos(east_handle_slide);

    let undo_before_resize = h.editor.undo_len_for_test();
    h.drag_by(
        east_handle_screen,
        vec2(160.0 * scale, 0.0),
        5,
        Modifiers::NONE,
    );
    h.idle_frame();
    assert_eq!(
        h.editor.undo_len_for_test(),
        undo_before_resize + 1,
        "dragging East handle must be 1 undo step"
    );
    {
        let (obj, _) = h
            .editor
            .session
            .course
            .slide("s1")
            .and_then(|s| s.object(&obj_id))
            .expect("object after resize");
        let ObjectKind::Text {
            style, point_text, ..
        } = &obj.kind
        else {
            panic!("expected Text");
        };
        assert!(!*point_text, "dragging East handle must clear point_text");
        assert!(style.wrap, "dragging East handle must enable wrap");
        assert!(
            obj.frame.w > original_frame.w + 140.0,
            "dragging East handle must widen frame: {} vs {}",
            obj.frame.w,
            original_frame.w
        );
    }

    // Ctrl+Z restores point_text = true, wrap = false, and original width.
    h.press(Key::Z, Modifiers::COMMAND);
    h.idle_frame();
    {
        let (obj, _) = h
            .editor
            .session
            .course
            .slide("s1")
            .and_then(|s| s.object(&obj_id))
            .expect("object after undo resize");
        let ObjectKind::Text {
            style, point_text, ..
        } = &obj.kind
        else {
            panic!("expected Text");
        };
        assert!(
            *point_text,
            "Ctrl+Z after East handle resize must restore point_text = true"
        );
        assert!(
            !style.wrap,
            "Ctrl+Z after East handle resize must restore wrap = false"
        );
        assert!(
            (obj.frame.w - original_frame.w).abs() <= 0.5,
            "Ctrl+Z must restore original fitted width: {} vs {}",
            obj.frame.w,
            original_frame.w
        );
    }
}
