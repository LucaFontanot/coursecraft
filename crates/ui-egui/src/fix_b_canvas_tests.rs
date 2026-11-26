//! Stage editing fixes (review 0.5.2, package B): modifier clicks, multi-selection
//! clicks and drags, straight lines, handles, ghost outlines, thin-line hits.

use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use egui::{Modifiers, Pos2, vec2};

fn hello() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    Course::from_json(&json).expect("valid course")
}

fn object_center(h: &TestHarness, slide_id: &str, obj_id: &str) -> Pos2 {
    let slide_rect = h.editor.ui.last_slide_rect.expect("slide rect");
    let slide_w = h.editor.session.course.slide_size.w as f32;
    let scale = slide_rect.width() / slide_w;
    let (obj, _) = h
        .editor
        .session
        .course
        .slide(slide_id)
        .and_then(|s| s.object(obj_id))
        .expect("object");
    slide_rect.min
        + vec2(
            (obj.frame.x + obj.frame.w * 0.5) * scale,
            (obj.frame.y + obj.frame.h * 0.5) * scale,
        )
}

fn frame_x(h: &TestHarness, id: &str) -> f32 {
    h.editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object(id))
        .map(|(o, _)| o.frame.x)
        .expect("object")
}

fn harness() -> TestHarness {
    let mut h = TestHarness::new(hello());
    h.idle_frame();
    h.idle_frame();
    h
}

#[test]
fn q1_ctrl_click_adds_an_unselected_object_and_removes_a_selected_one() {
    let mut h = harness();
    let title = object_center(&h, "s-intro", "title");
    let start = object_center(&h, "s-intro", "btn-start");
    h.click_at(title);
    assert_eq!(h.editor.selection.ids(), vec!["title"]);
    h.click_at_with(start, Modifiers::COMMAND);
    assert_eq!(h.editor.selection.ids(), vec!["title", "btn-start"]);
    h.click_at_with(start, Modifiers::COMMAND);
    assert_eq!(h.editor.selection.ids(), vec!["title"]);
}

#[test]
fn c1_shift_drag_moves_the_whole_selection() {
    let mut h = harness();
    let title = object_center(&h, "s-intro", "title");
    let start = object_center(&h, "s-intro", "btn-start");
    h.click_at(title);
    h.click_at_with(start, Modifiers::SHIFT);
    let (t0, s0) = (frame_x(&h, "title"), frame_x(&h, "btn-start"));
    h.drag_by(start, vec2(60.0, 0.0), 6, Modifiers::SHIFT);
    assert!(frame_x(&h, "title") > t0, "title did not move");
    assert!(frame_x(&h, "btn-start") > s0, "button did not move");
    assert_eq!(h.editor.selection.ids().len(), 2);
}

#[test]
fn c6_plain_click_on_one_of_a_multi_selection_selects_only_it() {
    let mut h = harness();
    let title = object_center(&h, "s-intro", "title");
    let start = object_center(&h, "s-intro", "btn-start");
    h.click_at(title);
    h.click_at_with(start, Modifiers::SHIFT);
    assert_eq!(h.editor.selection.ids().len(), 2);
    h.click_at(start);
    assert_eq!(h.editor.selection.ids(), vec!["btn-start"]);
}

#[test]
fn c6_plain_drag_of_one_of_a_multi_selection_still_moves_all() {
    let mut h = harness();
    let title = object_center(&h, "s-intro", "title");
    let start = object_center(&h, "s-intro", "btn-start");
    h.click_at(title);
    h.click_at_with(start, Modifiers::SHIFT);
    let (t0, s0) = (frame_x(&h, "title"), frame_x(&h, "btn-start"));
    h.drag_by(start, vec2(60.0, 0.0), 6, Modifiers::NONE);
    assert!(frame_x(&h, "title") > t0);
    assert!(frame_x(&h, "btn-start") > s0);
    assert_eq!(h.editor.selection.ids().len(), 2);
}

fn scale_of(h: &TestHarness) -> f32 {
    let slide_rect = h.editor.ui.last_slide_rect.expect("slide rect");
    slide_rect.width() / h.editor.session.course.slide_size.w as f32
}

/// Draw a horizontal straight line from (400, 300) of `len` slide px; returns its id.
fn draw_horizontal_line(h: &mut TestHarness, len: f32) -> String {
    use crate::{DrawTool, PlacementTool};
    h.editor.ui.snap_to_guides = false;
    h.editor.ui.placement_tool = Some(PlacementTool::Draw(DrawTool::Line));
    h.idle_frame();
    let slide_rect = h.editor.ui.last_slide_rect.expect("slide rect");
    let scale = scale_of(h);
    let start = slide_rect.min + vec2(400.0, 300.0) * scale;
    h.drag_by(start, vec2(len * scale, 0.0), 4, Modifiers::NONE);
    h.editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.objects.last())
        .map(|o| o.id.clone())
        .expect("line")
}

#[test]
fn q12_dragging_the_middle_of_a_straight_line_moves_it() {
    let mut h = harness();
    let id = draw_horizontal_line(&mut h, 200.0);
    h.idle_frame();
    let frame = |h: &TestHarness| {
        h.editor
            .session
            .course
            .slide("s-intro")
            .and_then(|s| s.object(&id))
            .map(|(o, _)| o.frame)
            .expect("line")
    };
    let before = frame(&h);
    assert!(before.h < 2.0, "line is not flat: {before:?}");
    let slide_rect = h.editor.ui.last_slide_rect.expect("slide rect");
    let scale = scale_of(&h);
    let mid = slide_rect.min + vec2((before.x + before.w * 0.5) * scale, before.y * scale);
    h.drag_by(mid, vec2(0.0, 40.0 * scale), 6, Modifiers::NONE);
    let after = frame(&h);
    assert!(after.h < 5.0, "line was resized: {after:?}");
    assert!(after.y > before.y + 20.0, "line did not move: {after:?}");
    assert!((after.w - before.w).abs() < 0.5);
}

fn count_segments(shapes: &[egui::epaint::ClippedShape]) -> usize {
    fn walk(shape: &egui::Shape) -> usize {
        match shape {
            egui::Shape::LineSegment { .. } => 1,
            egui::Shape::Vec(v) => v.iter().map(walk).sum(),
            _ => 0,
        }
    }
    shapes.iter().map(|c| walk(&c.shape)).sum()
}

/// Press on the button, move part way, and count line segments painted in that frame.
fn segments_mid_drag(modifiers: Modifiers) -> usize {
    use egui::{Event, PointerButton};
    let mut h = harness();
    let title = object_center(&h, "s-intro", "title");
    let start = object_center(&h, "s-intro", "btn-start");
    h.click_at(title);
    h.click_at(start);
    h.frame(vec![Event::ModifiersChanged(modifiers)]);
    h.frame(vec![Event::PointerMoved(start)]);
    h.frame(vec![Event::PointerButton {
        pos: start,
        button: PointerButton::Primary,
        pressed: true,
        modifiers,
    }]);
    h.frame(vec![Event::PointerMoved(start + vec2(30.0, 10.0))]);
    let shapes = h.frame_shapes(vec![Event::PointerMoved(start + vec2(60.0, 20.0))]);
    count_segments(&shapes)
}

#[test]
fn c3_alt_drag_draws_a_ghost_outline_of_the_copies() {
    let plain = segments_mid_drag(Modifiers::NONE);
    let alt = segments_mid_drag(Modifiers::ALT);
    assert!(
        alt > plain + 4,
        "no ghost outline: {alt} segments with Alt, {plain} without"
    );
}

/// An image at (400, 200) 200x160 with the given rotation and flip, crop mode open.
fn cropping_image(rotation: f32, flip: Option<&str>) -> (TestHarness, String) {
    let mut h = harness();
    let pixels = vec![200u8; 100 * 80 * 4];
    let png = coursecraft_media::encode_png_rgba(100, 80, &pixels).expect("png");
    let src = h.editor.import_image("photo.png", &png).expect("import");
    let id = h
        .editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s-intro",
                "kind": { "type": "image", "src": src, "alt": "photo" },
                "frame": { "x": 400.0, "y": 200.0, "w": 200.0, "h": 160.0 }
            }),
        )
        .and_then(|v| v["id"].as_str().map(str::to_owned))
        .expect("object id");
    h.idle_frame();
    if rotation != 0.0 {
        h.editor.run(
            "object.set_rotation",
            serde_json::json!({ "slide": "s-intro", "object": id, "degrees": rotation }),
        );
    }
    if let Some(axis) = flip {
        h.editor.run(
            "object.flip",
            serde_json::json!({ "slide": "s-intro", "object": id, "axis": axis }),
        );
    }
    h.idle_frame();
    assert!(h.editor.start_crop(&id), "crop mode did not open");
    h.idle_frame();
    (h, id)
}

fn image_crop_and_frame(h: &TestHarness, id: &str) -> (coursecraft_course::CropRect, [f32; 4]) {
    let (object, _) = h
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object(id))
        .expect("image");
    let coursecraft_course::ObjectKind::Image { crop, .. } = &object.kind else {
        panic!("expected image");
    };
    let f = object.frame;
    (crop.expect("crop set"), [f.x, f.y, f.w, f.h])
}

#[test]
fn c5_crop_handles_follow_a_rotated_picture() {
    let (mut h, id) = cropping_image(90.0, None);
    let slide_rect = h.editor.ui.last_slide_rect.expect("slide rect");
    let scale = scale_of(&h);
    // The local right-middle handle (600, 280) turns to (500, 380) at 90 degrees.
    let world = slide_rect.min + vec2(500.0, 380.0) * scale;
    h.drag_by(world, vec2(0.0, -40.0 * scale), 6, Modifiers::NONE);
    assert!(h.editor.ui.crop_edit.is_some(), "crop mode was left");
    h.editor.apply_crop();
    let (crop, f) = image_crop_and_frame(&h, &id);
    assert!((crop.w - 0.8).abs() < 0.02, "crop {crop:?}");
    assert!((crop.x).abs() < 0.01);
    // Local frame (400..560, 200..360) has its centre at (480, 280); turned
    // about (500, 280) by 90 degrees that is (500, 260).
    assert!(
        (f[2] - 160.0).abs() < 1.5 && (f[3] - 160.0).abs() < 1.5,
        "{f:?}"
    );
    assert!((f[0] + f[2] * 0.5 - 500.0).abs() < 1.5, "{f:?}");
    assert!((f[1] + f[3] * 0.5 - 260.0).abs() < 1.5, "{f:?}");
}

#[test]
fn c5_crop_handles_follow_a_flipped_picture() {
    let (mut h, id) = cropping_image(0.0, Some("horizontal"));
    let slide_rect = h.editor.ui.last_slide_rect.expect("slide rect");
    let scale = scale_of(&h);
    // The local left-middle handle (400, 280) mirrors to (600, 280).
    let world = slide_rect.min + vec2(600.0, 280.0) * scale;
    h.drag_by(world, vec2(-40.0 * scale, 0.0), 6, Modifiers::NONE);
    assert!(h.editor.ui.crop_edit.is_some(), "crop mode was left");
    h.editor.apply_crop();
    let (crop, f) = image_crop_and_frame(&h, &id);
    assert!((crop.x - 0.2).abs() < 0.02, "crop {crop:?}");
    assert!((crop.w - 0.8).abs() < 0.02, "crop {crop:?}");
    // Local frame 440..600 mirrors to 400..560 about x = 500.
    assert!(
        (f[0] - 400.0).abs() < 1.5 && (f[2] - 160.0).abs() < 1.5,
        "{f:?}"
    );
}

#[test]
fn p3_point_text_is_measured_when_the_text_changes_not_every_frame() {
    use crate::PlacementTool;
    let mut h = harness();
    h.editor.ui.snap_to_guides = false;
    h.editor.ui.placement_tool = Some(PlacementTool::TextBox);
    h.idle_frame();
    let slide_rect = h.editor.ui.last_slide_rect.expect("slide rect");
    let scale = scale_of(&h);
    h.click_at(slide_rect.min + vec2(300.0, 400.0) * scale);
    h.idle_frame();
    assert!(h.editor.ui.inline_edit.is_some(), "no inline edit");
    // Settle the entry frames, then count over a run of idle frames.
    for _ in 0..3 {
        h.idle_frame();
    }
    let before = crate::canvas::inline_measure_count();
    for _ in 0..20 {
        h.idle_frame();
    }
    let idle = crate::canvas::inline_measure_count() - before;
    assert_eq!(idle, 0, "{idle} measurements over 20 idle frames");

    // Typing still grows the box.
    let width = |h: &TestHarness| h.editor.ui.inline_edit_live_rect.map(|r| r.width());
    let w0 = width(&h).expect("live rect");
    h.type_text("a much longer line of text");
    h.idle_frame();
    let w1 = width(&h).expect("live rect");
    assert!(w1 > w0 * 2.0, "box did not grow: {w0} -> {w1}");
    let before = crate::canvas::inline_measure_count();
    for _ in 0..20 {
        h.idle_frame();
    }
    assert_eq!(crate::canvas::inline_measure_count() - before, 0);
}

#[test]
fn p6_editor_hidden_objects_are_not_drawn_or_picked() {
    let mut course = hello();
    if let Some(slide) = course.slide_mut("s-intro")
        && let Some(object) = slide.objects.iter_mut().find(|o| o.id == "btn-start")
    {
        object.editor_hidden = true;
    }
    let mut h = TestHarness::new(course);
    h.idle_frame();
    h.idle_frame();
    let start = object_center(&h, "s-intro", "btn-start");
    let about = object_center(&h, "s-intro", "btn-about");
    h.click_at(start);
    assert!(h.editor.selection.ids().is_empty(), "hidden object picked");
    h.click_at(about);
    assert_eq!(h.editor.selection.ids(), vec!["btn-about"]);
}

#[test]
fn c7_thin_line_hit_tolerance_is_six_screen_pixels() {
    let mut h = harness();
    let id = draw_horizontal_line(&mut h, 200.0);
    h.idle_frame();
    let slide_rect = h.editor.ui.last_slide_rect.expect("slide rect");
    let scale = scale_of(&h);
    assert!(scale < 0.8, "test needs a zoomed-out slide, scale {scale}");
    // Deselect, then click 5 screen px above the line: inside 6 screen px, but
    // outside 6 slide px once the scale is below 0.83.
    h.click_at(slide_rect.min + vec2(50.0, 50.0));
    assert!(h.editor.selection.ids().is_empty());
    let on_line = slide_rect.min + vec2(500.0 * scale, 300.0 * scale);
    h.click_at(on_line - vec2(0.0, 5.0));
    assert_eq!(h.editor.selection.ids(), vec![id]);
}

#[test]
fn c4_resize_cursor_follows_the_handle_angle_and_the_rotation() {
    use crate::canvas::{ResizeHandle as H, handle_cursor};
    use egui::CursorIcon as C;
    assert_eq!(handle_cursor(H::Right, 0.0), C::ResizeHorizontal);
    assert_eq!(handle_cursor(H::Right, 90.0), C::ResizeVertical);
    assert_eq!(handle_cursor(H::TopLeft, 0.0), C::ResizeNwSe);
    assert_eq!(handle_cursor(H::TopLeft, 90.0), C::ResizeNeSw);
    assert_eq!(handle_cursor(H::Top, 45.0), C::ResizeNeSw);
    assert_eq!(handle_cursor(H::Top, 180.0), C::ResizeVertical);
    assert_eq!(handle_cursor(H::Bottom, f32::NAN), C::ResizeVertical);
}

#[test]
fn c4_handle_hit_area_is_twelve_pixels() {
    let mut h = harness();
    let title = object_center(&h, "s-intro", "title");
    h.click_at(title);
    h.idle_frame();
    let slide_rect = h.editor.ui.last_slide_rect.expect("slide rect");
    let scale = scale_of(&h);
    let (obj, _) = h
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object("title"))
        .expect("title");
    let (w0, x0) = (obj.frame.w, obj.frame.x);
    // 5 px outside the right-middle handle centre: outside the drawn 8 px square,
    // inside the 12 px hit area.
    let right_mid = slide_rect.min
        + vec2(
            (obj.frame.x + obj.frame.w) * scale,
            (obj.frame.y + obj.frame.h * 0.5) * scale,
        );
    let start = right_mid + vec2(5.0, 0.0);
    h.drag_by(start, vec2(-40.0 * scale, 0.0), 6, Modifiers::NONE);
    let (obj, _) = h
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object("title"))
        .expect("title");
    assert!(obj.frame.w < w0 - 20.0, "no resize: w {}", obj.frame.w);
    assert!((obj.frame.x - x0).abs() < 0.5, "moved instead of resized");
}
