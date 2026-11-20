//! Headless tests for M24 Step 5: faster canvas work and multi-select editing.

use crate::ZoomLevel;
use crate::properties::InspectorTab;
use crate::test_harness::TestHarness;
use coursecraft_course::{Color, Course, FillStyle, GradientStop, ObjectKind};
use egui::{Event, Key, Modifiers, PointerButton, Pos2, vec2};

fn hello() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    Course::from_json(&json).expect("valid course")
}

fn object_center(harness: &TestHarness, slide_id: &str, obj_id: &str) -> (Pos2, f32) {
    let slide_rect = harness.editor.ui.last_slide_rect.expect("slide rect");
    let slide_w = harness.editor.session.course.slide_size.w as f32;
    let scale = slide_rect.width() / slide_w;
    let (obj, _) = harness
        .editor
        .session
        .course
        .slide(slide_id)
        .and_then(|s| s.object(obj_id))
        .expect("object");
    let center = slide_rect.min
        + vec2(
            (obj.frame.x + obj.frame.w * 0.5) * scale,
            (obj.frame.y + obj.frame.h * 0.5) * scale,
        );
    (center, scale)
}

#[test]
fn alt_drag_duplicates_leaving_original_and_one_undo_removes_copy() {
    let mut harness = TestHarness::new(hello());
    harness.editor.selection.slide = "s-intro".into();
    harness.editor.selection.object = Some("btn-start".into());
    harness.idle_frame();
    harness.idle_frame();

    let count_before = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("slide")
        .objects
        .len();
    let orig_frame = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object("btn-start"))
        .expect("btn-start")
        .0
        .frame;

    harness.editor.ui.snap_to_guides = false;
    let (start, scale) = object_center(&harness, "s-intro", "btn-start");
    harness.drag_by(start, vec2(40.0 * scale, 20.0 * scale), 3, Modifiers::ALT);
    harness.idle_frame();

    let slide = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("slide");
    assert_eq!(slide.objects.len(), count_before + 1, "one copy added");

    let orig_after = &slide.object("btn-start").expect("original").0.frame;
    assert!((orig_after.x - orig_frame.x).abs() < 0.5);
    assert!((orig_after.y - orig_frame.y).abs() < 0.5);

    let copy = slide
        .objects
        .iter()
        .find(|o| o.id != "btn-start" && (o.frame.x - (orig_frame.x + 40.0)).abs() < 2.0)
        .expect("copy at new spot");
    assert!((copy.frame.y - (orig_frame.y + 20.0)).abs() < 2.0);

    harness.editor.session.undo().expect("undo");
    let count_undone = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("slide")
        .objects
        .len();
    assert_eq!(count_undone, count_before, "one undo removes the copy");
}

#[test]
fn shift_drag_locks_to_dominant_axis() {
    let mut harness = TestHarness::new(hello());
    harness.editor.selection.slide = "s-intro".into();
    harness.editor.selection.object = Some("btn-start".into());
    harness.idle_frame();
    harness.idle_frame();

    let orig_frame = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object("btn-start"))
        .expect("btn-start")
        .0
        .frame;

    let (start, scale) = object_center(&harness, "s-intro", "btn-start");
    harness.frame(vec![Event::PointerMoved(start)]);
    harness.frame(vec![Event::PointerButton {
        pos: start,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    }]);
    harness.frame(vec![Event::ModifiersChanged(Modifiers::SHIFT)]);
    let end = start + vec2(40.0 * scale, 5.0 * scale);
    harness.frame(vec![Event::PointerMoved(end)]);
    harness.frame(vec![Event::PointerButton {
        pos: end,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::SHIFT,
    }]);
    harness.frame(vec![Event::ModifiersChanged(Modifiers::NONE)]);
    harness.idle_frame();

    let after = &harness
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object("btn-start"))
        .expect("btn-start")
        .0
        .frame;
    assert!(
        (after.x - (orig_frame.x + 40.0)).abs() < 2.0,
        "moved on x: {} -> {}",
        orig_frame.x,
        after.x
    );
    assert!(
        (after.y - orig_frame.y).abs() < 0.01,
        "locked on y: {} -> {}",
        orig_frame.y,
        after.y
    );
}

#[test]
fn space_drag_pans_without_moving_selection() {
    let mut harness = TestHarness::new(hello());
    harness.editor.selection.slide = "s-intro".into();
    harness.editor.selection.object = Some("btn-start".into());
    harness.editor.ui.zoom = ZoomLevel::Custom(2.0);
    harness.idle_frame();
    harness.idle_frame();

    let slide_before = harness.editor.ui.last_slide_rect.expect("slide rect");
    let orig_frame = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object("btn-start"))
        .expect("btn-start")
        .0
        .frame;
    let (start, _) = object_center(&harness, "s-intro", "btn-start");

    harness.frame(vec![Event::Key {
        key: Key::Space,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    harness.frame(vec![Event::PointerMoved(start)]);
    harness.frame(vec![Event::PointerButton {
        pos: start,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    }]);
    let end = start + vec2(-60.0, -40.0);
    harness.frame(vec![Event::PointerMoved(end)]);
    harness.frame(vec![Event::PointerButton {
        pos: end,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    }]);
    harness.frame(vec![Event::Key {
        key: Key::Space,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    harness.idle_frame();

    let slide_after = harness.editor.ui.last_slide_rect.expect("slide rect");
    assert!(
        (slide_after.min.x - slide_before.min.x).abs() > 5.0
            || (slide_after.min.y - slide_before.min.y).abs() > 5.0,
        "canvas panned: before={:?} after={:?}",
        slide_before.min,
        slide_after.min
    );
    let obj_after = &harness
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object("btn-start"))
        .expect("btn-start")
        .0
        .frame;
    assert!((obj_after.x - orig_frame.x).abs() < 0.01);
    assert!((obj_after.y - orig_frame.y).abs() < 0.01);
}

#[test]
fn cmd_b_during_inline_text_edit_makes_selection_bold() {
    let mut course = hello();
    if let Some(slide) = course.slide_mut("s-intro")
        && let Some(obj) = slide.objects.iter_mut().find(|o| o.id == "title")
        && let ObjectKind::Text { style, .. } = &mut obj.kind
    {
        style.bold = false;
    }
    let mut harness = TestHarness::new(course);
    harness.editor.selection.slide = "s-intro".into();
    harness.editor.selection.object = Some("title".into());
    harness.idle_frame();
    harness.idle_frame();

    let (center, _) = object_center(&harness, "s-intro", "title");
    harness.click_at(center);
    harness.click_at(center);
    harness.idle_frame();
    assert!(
        harness.editor.ui.inline_edit.is_some(),
        "entered inline edit"
    );

    harness.press(Key::A, Modifiers::COMMAND);
    harness.press(Key::B, Modifiers::COMMAND);
    harness.idle_frame();

    let (obj, _) = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object("title"))
        .expect("title");
    match &obj.kind {
        ObjectKind::Text { style, runs, .. } => {
            let any_bold = if runs.is_empty() {
                style.bold
            } else {
                runs.iter().any(|r| r.bold.unwrap_or(style.bold))
            };
            assert!(any_bold, "Cmd+B made the selection bold");
        }
        other => panic!("expected text, got {other:?}"),
    }
}

#[test]
fn multi_select_opacity_sets_both_objects() {
    let mut harness = TestHarness::new(hello());
    harness.editor.selection.slide = "s-intro".into();
    harness
        .editor
        .selection
        .set_many(vec!["title".to_owned(), "btn-start".to_owned()]);
    harness.editor.ui.inspector_tab = InspectorTab::Format;
    harness.idle_frame();
    harness.idle_frame();

    let rect = harness
        .editor
        .ui
        .properties
        .opacity_rect
        .expect("multi-select opacity slider rect");
    let target = Pos2::new(rect.min.x + rect.width() * 0.25, rect.center().y);
    harness.click_at(target);
    harness.idle_frame();

    let slide = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("slide");
    let o1 = slide.object("title").expect("title").0.opacity;
    let o2 = slide.object("btn-start").expect("btn-start").0.opacity;
    assert!(o1 < 0.99, "first object opacity updated: {o1}");
    assert!(
        (o1 - o2).abs() < 0.001,
        "both objects share new opacity: {o1} vs {o2}"
    );
}

#[test]
fn multi_select_button_fill_preserves_gradient_fill_style() {
    let mut course = hello();
    let grad = FillStyle::Linear {
        angle: 90.0,
        stops: vec![
            GradientStop {
                position: 0.0,
                color: Color("#112233".into()),
            },
            GradientStop {
                position: 1.0,
                color: Color("#445566".into()),
            },
        ],
    };
    {
        let slide = course.slide_mut("s-intro").expect("s-intro");
        if let Some(btn) = slide.objects.iter_mut().find(|o| o.id == "btn-start")
            && let ObjectKind::Button { fill_style, .. } = &mut btn.kind
        {
            *fill_style = Some(grad.clone());
        }
        let mut btn2 = slide
            .objects
            .iter()
            .find(|o| o.id == "btn-start")
            .expect("btn-start")
            .clone();
        btn2.id = "btn-second".to_owned();
        btn2.frame.y += 80.0;
        if let ObjectKind::Button { fill_style, .. } = &mut btn2.kind {
            *fill_style = None;
        }
        slide.objects.push(btn2);
    }

    let mut harness = TestHarness::new(course);
    harness.editor.selection.slide = "s-intro".into();
    harness
        .editor
        .selection
        .set_many(vec!["btn-start".to_owned(), "btn-second".to_owned()]);
    harness.editor.ui.inspector_tab = InspectorTab::Format;
    harness.idle_frame();
    harness.idle_frame();

    let panel = harness.right_panel_rect();
    let fill_label = harness
        .find_text_in("Fill", panel)
        .expect("Fill label in multi-select panel");
    let field_pos = Pos2::new(panel.center().x + 20.0, fill_label.y);
    harness.set_field(field_pos, "#123456");
    harness.idle_frame();

    let slide = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("slide");
    match &slide.object("btn-start").expect("btn-start").0.kind {
        ObjectKind::Button {
            fill, fill_style, ..
        } => {
            assert_eq!(fill.0, "#123456");
            assert_eq!(fill_style.as_ref(), Some(&grad));
        }
        other => panic!("expected button, got {other:?}"),
    }
    match &slide.object("btn-second").expect("btn-second").0.kind {
        ObjectKind::Button { fill, .. } => {
            assert_eq!(fill.0, "#123456");
        }
        other => panic!("expected button, got {other:?}"),
    }
}
