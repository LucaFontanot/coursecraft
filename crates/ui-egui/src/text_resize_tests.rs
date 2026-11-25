//! Reproduction & regression tests for live text-box resizing while typing (0.5.1).
//!
//! Drives the real editor UI (both the 0.5.0 ribbon + docked-panel workspace and
//! the classic layout) through:
//! 1. Canvas click with `PlacementTool::TextBox` (point text),
//! 2. Ribbon Insert › "Text Box" button (click for point text),
//! 3. Double-clicking an existing point-text box (including one with a border outline
//!    and centre alignment).
//!
//! Checks DURING editing (before commit, including on the very frame of input)
//! that the drawn text box outline, the selection outline, all 8 resize handles,
//! and the inline `TextEdit` bounds all follow the text horizontally (wider/narrower)
//! and vertically (Enter for more lines, Backspace for fewer lines).

use crate::test_harness::TestHarness;
use crate::{PlacementTool, canvas_fit_rect, canvas_rect};
use coursecraft_course::{Course, ObjectKind};
use coursecraft_render::SlideTransform;
use egui::epaint::{ClippedShape, Shape};
use egui::{Color32, Event, Key, Modifiers, Pos2, Rect, pos2};

fn empty_slide_course() -> Course {
    let raw = serde_json::json!({
        "schema_version": 1,
        "id": "text-resize-test",
        "title": "Text Resize Live Test",
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

fn existing_point_text_course() -> Course {
    let raw = serde_json::json!({
        "schema_version": 1,
        "id": "text-resize-existing",
        "title": "Existing Point Text Resize Test",
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
                    "id": "pt1",
                    "name": "Label",
                    "frame": { "x": 500.0, "y": 240.0, "w": 60.0, "h": 30.0 },
                    "kind": {
                        "type": "text",
                        "text": "Hi",
                        "point_text": true,
                        "style": {
                            "color": "#222222",
                            "size": 20.0,
                            "wrap": false,
                            "align": "center"
                        },
                        "outline": {
                            "color": "#E11D48",
                            "width": 2.0
                        }
                    }
                }]
            }]
        }]
    });
    serde_json::from_value(raw).expect("valid course")
}

fn new_ribbon_dock_harness(course: Course) -> TestHarness {
    let mut h = TestHarness::new_with_size(course, (1600.0, 900.0));
    h.editor.set_layout_prefs(&crate::dock::LayoutPrefs {
        ribbon: true,
        docked: true,
        workspace: None,
    });
    h.editor.ui.snap_to_guides = false;
    for _ in 0..3 {
        h.idle_frame();
    }
    h
}

fn slide_to_screen(h: &TestHarness, slide_pt: Pos2) -> Pos2 {
    let slide_rect = h.editor.ui.last_slide_rect.expect("last_slide_rect");
    pos2(
        slide_rect.min.x + slide_rect.width() * (slide_pt.x / 1280.0),
        slide_rect.min.y + slide_rect.height() * (slide_pt.y / 720.0),
    )
}

fn slide_transform(h: &TestHarness) -> SlideTransform {
    SlideTransform::fit(
        canvas_fit_rect(canvas_rect(h.central_panel_rect())),
        h.editor.session.course.slide_size,
    )
    .expect("slide transform")
}

/// Find the 2 px solid accent selection outline drawn by `draw_handles_and_outline`.
fn selection_outline_rect(
    shapes: &[ClippedShape],
    accent: Color32,
    slide_rect: Rect,
) -> Option<Rect> {
    fn walk(shape: &Shape, accent: Color32, slide_rect: Rect) -> Option<Rect> {
        match shape {
            Shape::Path(p)
                if p.closed
                    && p.points.len() == 4
                    && p.stroke.color == egui::epaint::ColorMode::Solid(accent)
                    && (p.stroke.width - 2.0).abs() < 0.25 =>
            {
                let mut r = Rect::NOTHING;
                for &pt in &p.points {
                    r.extend_with(pt);
                }
                if r.intersects(slide_rect) {
                    Some(r)
                } else {
                    None
                }
            }
            Shape::Vec(v) => v.iter().find_map(|s| walk(s, accent, slide_rect)),
            _ => None,
        }
    }
    shapes
        .iter()
        .find_map(|cs| walk(&cs.shape, accent, slide_rect))
}

/// Find the 8 white 8×8 square resize handles drawn by `draw_handles_and_outline`
/// and return `(count, union_of_handle_centers)`.
fn resize_handles_bounds(
    shapes: &[ClippedShape],
    accent: Color32,
    slide_rect: Rect,
) -> (usize, Rect) {
    fn walk(shape: &Shape, accent: Color32, slide_rect: Rect, centers: &mut Vec<Pos2>) {
        match shape {
            Shape::Rect(r)
                if r.fill == Color32::TRANSPARENT
                    && r.stroke.color == accent
                    && (r.stroke.width - 1.0).abs() < 0.25
                    && (r.rect.width() - 8.0).abs() < 0.5
                    && (r.rect.height() - 8.0).abs() < 0.5
                    && slide_rect.expand(16.0).contains(r.rect.center()) =>
            {
                centers.push(r.rect.center());
            }
            Shape::Vec(v) => {
                for s in v {
                    walk(s, accent, slide_rect, centers);
                }
            }
            _ => {}
        }
    }
    let mut centers = Vec::new();
    for cs in shapes {
        walk(&cs.shape, accent, slide_rect, &mut centers);
    }
    // Each handle draws rect_filled + rect_stroke (two shapes or one combined); deduplicate close centers.
    let mut unique: Vec<Pos2> = Vec::new();
    for c in centers {
        if !unique.iter().any(|u| (*u - c).length() < 1.0) {
            unique.push(c);
        }
    }
    let mut union = Rect::NOTHING;
    for &c in &unique {
        union.extend_with(c);
    }
    (unique.len(), union)
}

/// Find the bounding rect of the painted outline mesh for a specific color (e.g. `#E11D48`).
fn painted_outline_mesh_bounds(
    shapes: &[ClippedShape],
    color: Color32,
    slide_rect: Rect,
) -> Option<Rect> {
    fn walk(shape: &Shape, color: Color32, slide_rect: Rect, out: &mut Rect) {
        match shape {
            Shape::Mesh(mesh) => {
                if !mesh.vertices.is_empty() && mesh.vertices.iter().any(|v| v.color == color) {
                    let mut r = Rect::NOTHING;
                    for v in &mesh.vertices {
                        if v.color == color {
                            r.extend_with(v.pos);
                        }
                    }
                    if r.intersects(slide_rect) {
                        *out = out.union(r);
                    }
                }
            }
            Shape::Vec(v) => {
                for s in v {
                    walk(s, color, slide_rect, out);
                }
            }
            _ => {}
        }
    }
    let mut bounds = Rect::NOTHING;
    for cs in shapes {
        walk(&cs.shape, color, slide_rect, &mut bounds);
    }
    bounds.is_positive().then_some(bounds)
}

/// Find the inline `TextEdit` widget rect on the canvas from AccessKit nodes.
fn inline_text_edit_rect(h: &TestHarness, slide_rect: Rect) -> Option<Rect> {
    h.last_widgets
        .iter()
        .find(|w| {
            matches!(
                w.role,
                egui::accesskit::Role::MultilineTextInput | egui::accesskit::Role::TextInput
            ) && slide_rect.expand(32.0).contains_rect(w.bounds)
        })
        .map(|w| w.bounds)
}

fn assert_rects_match(label: &str, a: Rect, b: Rect, tol: f32) {
    assert!(
        (a.min.x - b.min.x).abs() <= tol
            && (a.min.y - b.min.y).abs() <= tol
            && (a.max.x - b.max.x).abs() <= tol
            && (a.max.y - b.max.y).abs() <= tol,
        "{label}: rects disagree (tol={tol}): {a:?} vs {b:?}"
    );
}

#[test]
fn repro_point_text_canvas_click_live_resize_outline_handles_and_edit_rect() {
    let mut h = TestHarness::new(empty_slide_course());
    h.editor.ui.snap_to_guides = false;
    h.idle_frame();

    h.editor.ui.placement_tool = Some(PlacementTool::TextBox);
    h.idle_frame();
    let t = slide_transform(&h);
    h.click_at(t.to_screen_pos(pos2(260.0, 220.0)));
    let shapes_init = h.frame_shapes(Vec::new());
    let slide_rect = h.editor.ui.last_slide_rect.expect("slide rect");
    let accent = coursecraft_render::palette(&h.ctx).accent;

    let outline_init = selection_outline_rect(&shapes_init, accent, slide_rect)
        .expect("initial selection outline");
    let (hcount_init, _handles_init) = resize_handles_bounds(&shapes_init, accent, slide_rect);
    assert_eq!(hcount_init, 8, "expected 8 resize handles");
    let _edit_init = inline_text_edit_rect(&h, slide_rect).expect("initial inline TextEdit rect");

    // Type a much longer single-line string — check on the VERY NEXT frame AND after idle.
    let shapes_typed = h.frame_shapes(vec![Event::Text("A much wider single line".to_owned())]);
    let outline_typed_immediate =
        selection_outline_rect(&shapes_typed, accent, slide_rect).expect("outline on input frame");
    let (hcount_typed, handles_typed_immediate) =
        resize_handles_bounds(&shapes_typed, accent, slide_rect);
    assert_eq!(hcount_typed, 8);
    let edit_typed_immediate =
        inline_text_edit_rect(&h, slide_rect).expect("inline TextEdit on input frame");

    // Check after an idle frame too.
    let shapes_after = h.frame_shapes(Vec::new());
    let outline_after =
        selection_outline_rect(&shapes_after, accent, slide_rect).expect("outline after input");
    let (hcount_after, handles_after) = resize_handles_bounds(&shapes_after, accent, slide_rect);
    assert_eq!(hcount_after, 8);
    let edit_after = inline_text_edit_rect(&h, slide_rect).expect("inline TextEdit after input");

    // Selection outline, resize handles, AND inline TextEdit rect must ALL grow horizontally!
    assert!(
        outline_after.width() > outline_init.width() * 2.5,
        "selection outline must grow horizontally: after={outline_after:?}, init={outline_init:?}"
    );
    assert_rects_match(
        "handles vs selection outline after typing",
        handles_after,
        outline_after,
        2.5,
    );
    assert_rects_match(
        "inline TextEdit vs selection outline after typing",
        edit_after,
        outline_after,
        3.0,
    );
    // And on the immediate input frame (no 1-frame lag):
    assert_rects_match(
        "immediate input frame outline vs settled",
        outline_typed_immediate,
        outline_after,
        2.5,
    );
    assert_rects_match(
        "immediate input frame handles vs settled",
        handles_typed_immediate,
        outline_after,
        2.5,
    );
    assert_rects_match(
        "immediate input frame TextEdit vs settled",
        edit_typed_immediate,
        outline_after,
        3.0,
    );

    // Press Enter + type a second line -> must grow vertically!
    h.press(Key::Enter, Modifiers::NONE);
    h.type_text("Second line");
    let shapes_two = h.frame_shapes(Vec::new());
    let outline_two =
        selection_outline_rect(&shapes_two, accent, slide_rect).expect("outline two lines");
    let (hcount_two, handles_two) = resize_handles_bounds(&shapes_two, accent, slide_rect);
    assert_eq!(hcount_two, 8);
    let edit_two = inline_text_edit_rect(&h, slide_rect).expect("TextEdit two lines");

    assert!(
        outline_two.height() > outline_after.height() * 1.5,
        "selection outline must grow vertically on Enter: two={outline_two:?}, one={outline_after:?}"
    );
    assert_rects_match(
        "handles vs outline on two lines",
        handles_two,
        outline_two,
        2.5,
    );
    assert_rects_match(
        "TextEdit vs outline on two lines",
        edit_two,
        outline_two,
        3.0,
    );

    // Backspace all of "Second line" + newline + most of line 1 -> must shrink back!
    for _ in 0..32 {
        h.press(Key::Backspace, Modifiers::NONE);
    }
    let shapes_shrunk = h.frame_shapes(Vec::new());
    let outline_shrunk =
        selection_outline_rect(&shapes_shrunk, accent, slide_rect).expect("outline shrunk");
    let (hcount_shrunk, handles_shrunk) = resize_handles_bounds(&shapes_shrunk, accent, slide_rect);
    assert_eq!(hcount_shrunk, 8);
    let edit_shrunk = inline_text_edit_rect(&h, slide_rect).expect("TextEdit shrunk");

    assert!(
        outline_shrunk.height() < outline_two.height() * 0.75,
        "outline must shrink vertically after Backspace: shrunk={outline_shrunk:?}, two={outline_two:?}"
    );
    assert!(
        outline_shrunk.width() < outline_after.width() * 0.6,
        "outline must shrink horizontally after Backspace: shrunk={outline_shrunk:?}, wide={outline_after:?}"
    );
    assert_rects_match(
        "handles vs outline after Backspace",
        handles_shrunk,
        outline_shrunk,
        2.5,
    );
    assert_rects_match(
        "TextEdit vs outline after Backspace",
        edit_shrunk,
        outline_shrunk,
        3.0,
    );
}

#[test]
fn repro_ribbon_insert_text_box_in_docked_layout_resizes_live() {
    let mut h = new_ribbon_dock_harness(empty_slide_course());
    let insert_pos = h.find_text("Insert").expect("Insert ribbon tab");
    h.click_at(insert_pos);
    h.idle_frame();
    let tb_pos = h.find_text("Text Box").expect("Insert › Text Box button");
    h.click_at(tb_pos);
    h.idle_frame();
    assert_eq!(h.editor.ui.placement_tool, Some(PlacementTool::TextBox));

    let click_pt = slide_to_screen(&h, pos2(300.0, 200.0));
    h.click_at(click_pt);
    // Let follow_work open Panel::Text and settle layout.
    let shapes_init = h.frame_shapes(Vec::new());
    let shapes_init = if h.editor.ui.last_slide_rect.is_some() {
        h.frame_shapes(Vec::new())
    } else {
        shapes_init
    };
    let slide_rect = h.editor.ui.last_slide_rect.expect("slide rect");
    let accent = coursecraft_render::palette(&h.ctx).accent;
    assert!(
        h.editor.ui.inline_edit.is_some(),
        "ribbon Text Box click must open inline edit"
    );

    let outline_init =
        selection_outline_rect(&shapes_init, accent, slide_rect).expect("initial outline");
    h.type_text("Ribbon placed live label");
    let shapes_wide = h.frame_shapes(Vec::new());
    let outline_wide =
        selection_outline_rect(&shapes_wide, accent, slide_rect).expect("wide outline");
    let (hcount_wide, handles_wide) = resize_handles_bounds(&shapes_wide, accent, slide_rect);
    assert_eq!(hcount_wide, 8);

    assert!(
        outline_wide.width() > outline_init.width() * 2.0,
        "ribbon-placed point text must grow horizontally: wide={outline_wide:?}, init={outline_init:?}"
    );
    assert_rects_match(
        "ribbon-placed handles vs outline",
        handles_wide,
        outline_wide,
        2.5,
    );

    // Enter adds a line, Backspace removes it.
    h.press(Key::Enter, Modifiers::NONE);
    h.type_text("Line 2");
    let shapes_tall = h.frame_shapes(Vec::new());
    let outline_tall =
        selection_outline_rect(&shapes_tall, accent, slide_rect).expect("tall outline");
    let (_, handles_tall) = resize_handles_bounds(&shapes_tall, accent, slide_rect);
    assert!(outline_tall.height() > outline_wide.height() * 1.5);
    assert_rects_match(
        "ribbon tall handles vs outline",
        handles_tall,
        outline_tall,
        2.5,
    );
}

#[test]
fn repro_double_click_existing_point_text_resizes_drawn_border_handles_and_anchors_center() {
    let mut h = new_ribbon_dock_harness(existing_point_text_course());
    let accent = coursecraft_render::palette(&h.ctx).accent;
    let border_color = Color32::from_rgb(0xE1, 0x1D, 0x48);

    // Double-click the existing point-text object ("pt1" at x=500, y=240, w=60, h=30).
    let pt_center = slide_to_screen(&h, pos2(530.0, 255.0));
    h.double_click_at(pt_center);
    h.idle_frame();
    let shapes_init = h.frame_shapes(Vec::new());
    let slide_rect = h.editor.ui.last_slide_rect.expect("slide rect after dock");
    assert!(
        h.editor.ui.inline_edit.is_some(),
        "double-clicking existing point text must start inline edit"
    );

    let outline_init =
        selection_outline_rect(&shapes_init, accent, slide_rect).expect("initial outline");
    let init_center_x = outline_init.center().x;

    // Type a much longer string (select-all placeholder replaced).
    h.type_text("A centered point text label");
    let shapes_wide = h.frame_shapes(Vec::new());
    let outline_wide =
        selection_outline_rect(&shapes_wide, accent, slide_rect).expect("wide outline");
    let (hcount_wide, handles_wide) = resize_handles_bounds(&shapes_wide, accent, slide_rect);
    assert_eq!(hcount_wide, 8);
    let border_wide = painted_outline_mesh_bounds(&shapes_wide, border_color, slide_rect)
        .expect("painted box outline mesh");

    assert!(
        outline_wide.width() > outline_init.width() * 2.5,
        "existing point text outline must grow: wide={outline_wide:?}, init={outline_init:?}"
    );
    assert!(
        (outline_wide.center().x - init_center_x).abs() <= 3.0,
        "center-aligned point text must keep its center x: wide={}, init={}",
        outline_wide.center().x,
        init_center_x
    );
    assert_rects_match(
        "existing point text handles vs outline",
        handles_wide,
        outline_wide,
        2.5,
    );
    assert_rects_match(
        "existing point text painted border vs outline",
        border_wide,
        outline_wide,
        4.0,
    );

    // Commit and verify stored frame equals last live frame in 1 undo step.
    let undo_before = h.editor.undo_len_for_test();
    let last_live = h
        .editor
        .ui
        .inline_edit_live_rect
        .expect("last live rect before commit");
    h.press(Key::Enter, Modifiers::COMMAND);
    h.idle_frame();
    assert!(h.editor.ui.inline_edit.is_none());
    assert_eq!(h.editor.undo_len_for_test(), undo_before + 1);
    let committed = h
        .editor
        .session
        .course
        .slide("s1")
        .and_then(|s| s.object("pt1"))
        .map(|(o, _)| o.clone())
        .expect("pt1");
    assert!(matches!(
        committed.kind,
        ObjectKind::Text {
            point_text: true,
            ..
        }
    ));
    assert!(
        (committed.frame.x - last_live.min.x).abs() <= 1.0
            && (committed.frame.y - last_live.min.y).abs() <= 1.0
            && (committed.frame.w - last_live.width()).abs() <= 1.0
            && (committed.frame.h - last_live.height()).abs() <= 1.0,
        "committed frame {:?} must match last live rect {:?}",
        committed.frame,
        last_live
    );
}

#[test]
fn area_text_drag_defaults_to_grow_and_resizes_vertically_live() {
    let mut h = new_ribbon_dock_harness(empty_slide_course());
    let insert_pos = h.find_text("Insert").expect("Insert ribbon tab");
    h.click_at(insert_pos);
    h.idle_frame();
    let tb_pos = h.find_text("Text Box").expect("Insert › Text Box button");
    h.click_at(tb_pos);
    h.idle_frame();

    // Drag a fixed-width wrapping text box (~220 × 40 slide px).
    let d1 = slide_to_screen(&h, pos2(260.0, 180.0));
    let d2 = slide_to_screen(&h, pos2(480.0, 220.0));
    h.drag_by(d1, d2 - d1, 5, Modifiers::NONE);
    // Let follow_work open Panel::Text and settle layout.
    h.idle_frame();
    let shapes_init = h.frame_shapes(Vec::new());
    let slide_rect = h.editor.ui.last_slide_rect.expect("slide rect");
    let accent = coursecraft_render::palette(&h.ctx).accent;

    let edit = h
        .editor
        .ui
        .inline_edit
        .as_ref()
        .expect("dragged TextBox enters inline edit")
        .clone();
    let initial_obj = h
        .editor
        .session
        .course
        .slide("s1")
        .and_then(|s| s.object(&edit.object))
        .map(|(o, _)| o.clone())
        .expect("placed area text");
    match &initial_obj.kind {
        ObjectKind::Text {
            style,
            autofit,
            point_text,
            ..
        } => {
            assert!(!*point_text, "dragged box must be area text");
            assert!(style.wrap, "dragged box must wrap");
            assert_eq!(
                *autofit,
                coursecraft_course::Autofit::Grow,
                "new dragged area text box must default to Autofit::Grow"
            );
        }
        other => panic!("expected Text, got {other:?}"),
    }

    let outline_init =
        selection_outline_rect(&shapes_init, accent, slide_rect).expect("initial outline");
    let initial_screen_w = outline_init.width();

    // Type a long wrapping paragraph — must grow downward LIVE on the immediate input frame,
    // keeping width fixed!
    let long_para = "This wrapping paragraph has plenty of words to wrap across four or five lines inside a two hundred twenty pixel box.";
    let shapes_grown = h.frame_shapes(vec![Event::Text(long_para.to_owned())]);
    let outline_grown =
        selection_outline_rect(&shapes_grown, accent, slide_rect).expect("grown outline");
    let (hcount_grown, handles_grown) = resize_handles_bounds(&shapes_grown, accent, slide_rect);
    assert_eq!(hcount_grown, 8);
    let edit_grown = inline_text_edit_rect(&h, slide_rect).expect("grown inline TextEdit");

    assert!(
        (outline_grown.width() - initial_screen_w).abs() <= 2.5,
        "Grow box width must stay fixed: grown={} vs init={}",
        outline_grown.width(),
        initial_screen_w
    );
    assert!(
        outline_grown.height() > outline_init.height() * 2.0,
        "Grow box must grow downward live: grown={outline_grown:?}, init={outline_init:?}"
    );
    assert_rects_match("Grow handles vs outline", handles_grown, outline_grown, 2.5);
    assert_rects_match("Grow TextEdit vs outline", edit_grown, outline_grown, 3.0);

    // Replace with short text ("One") -> shrinks back up live, never below one line, width still fixed.
    h.press(Key::A, Modifiers::COMMAND);
    h.type_text("One");
    let shapes_shrunk = h.frame_shapes(Vec::new());
    let outline_shrunk =
        selection_outline_rect(&shapes_shrunk, accent, slide_rect).expect("shrunk outline");
    let (hcount_shrunk, handles_shrunk) = resize_handles_bounds(&shapes_shrunk, accent, slide_rect);
    assert_eq!(hcount_shrunk, 8);
    assert!(
        (outline_shrunk.width() - initial_screen_w).abs() <= 2.5,
        "Grow box width must stay fixed on shrink"
    );
    assert!(
        outline_shrunk.height() < outline_grown.height() * 0.5 && outline_shrunk.height() > 8.0,
        "Grow box must shrink back to ~1 line: shrunk={outline_shrunk:?}, grown={outline_grown:?}"
    );
    assert_rects_match(
        "Grow shrunk handles vs outline",
        handles_shrunk,
        outline_shrunk,
        2.5,
    );

    // Commit in 1 undo step -> stored frame matches last live rect, width unchanged.
    let undo_before = h.editor.undo_len_for_test();
    let last_live = h
        .editor
        .ui
        .inline_edit_live_rect
        .expect("last live rect for Grow");
    h.press(Key::Enter, Modifiers::COMMAND);
    h.idle_frame();
    assert!(h.editor.ui.inline_edit.is_none());
    assert_eq!(h.editor.undo_len_for_test(), undo_before + 1);
    let committed = h
        .editor
        .session
        .course
        .slide("s1")
        .and_then(|s| s.object(&edit.object))
        .map(|(o, _)| o.clone())
        .expect("committed Grow object");
    assert!(
        (committed.frame.w - initial_obj.frame.w).abs() <= 0.5,
        "committed Grow width must equal initial width: {} vs {}",
        committed.frame.w,
        initial_obj.frame.w
    );
    assert!(
        (committed.frame.h - last_live.height()).abs() <= 1.0,
        "committed Grow height must equal last live height: {} vs {}",
        committed.frame.h,
        last_live.height()
    );
}

#[test]
fn inspector_autofit_combo_offers_grow_and_refits_existing_area_text() {
    let raw = serde_json::json!({
        "schema_version": 1,
        "id": "grow-inspector-test",
        "title": "Grow Inspector Test",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "sc1",
            "title": "Main",
            "slides": [{
                "id": "s1",
                "title": "Slide 1",
                "objects": [{
                    "id": "area1",
                    "name": "Body",
                    "frame": { "x": 120.0, "y": 140.0, "w": 200.0, "h": 30.0 },
                    "kind": {
                        "type": "text",
                        "text": "A long wrapping body paragraph that overflows a tiny thirty pixel high text box at width two hundred"
                    }
                }]
            }]
        }]
    });
    let course: Course = serde_json::from_value(raw).expect("valid course");
    let mut h = TestHarness::new(course);
    h.editor.selection.set_many(["area1".to_owned()]);
    h.editor.ui.inspector_tab = crate::properties::InspectorTab::Format;
    h.idle_frame();
    h.idle_frame();

    // Existing object keeps Autofit::None (shown as "Off").
    let right = h.right_panel_rect();
    let off_pos = h
        .find_text_in("Off", right)
        .expect("Autofit combo showing Off");
    let undo_before = h.editor.undo_len_for_test();
    h.click_at(off_pos);
    h.idle_frame();

    // All three options must be visible: Off, Shrink text on overflow, Resize box to fit text.
    assert!(h.find_text("Shrink text on overflow").is_some());
    let grow_opt = h
        .find_text("Resize box to fit text")
        .expect("Resize box to fit text option in Autofit combo");
    h.click_at(grow_opt);
    h.idle_frame();

    assert_eq!(
        h.editor.undo_len_for_test(),
        undo_before + 1,
        "switching to Grow must be 1 undo step"
    );
    let obj = h
        .editor
        .session
        .course
        .slide("s1")
        .and_then(|s| s.object("area1"))
        .map(|(o, _)| o.clone())
        .expect("area1");
    assert_eq!(obj.frame.w, 200.0, "width must stay fixed");
    assert!(
        obj.frame.h > 80.0,
        "switching to Grow must refit height to fit wrapping text, got {}",
        obj.frame.h
    );
    assert!(matches!(
        obj.kind,
        ObjectKind::Text {
            autofit: coursecraft_course::Autofit::Grow,
            point_text: false,
            ..
        }
    ));
}
