//! Tests for M25 Step 9 editor input fixes (review 3.9, 3.10, 3.11, 3.12).

use crate::BottomTab;
use crate::test_harness::TestHarness;
use crate::triggers::TriggerFormState;
use coursecraft_course::{Course, MotionPath, Point, ShapeKind};
use egui::{Event, Key, Modifiers, PointerButton, Pos2, vec2};

fn hello() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    Course::from_json(&json).expect("valid course")
}

fn slide_scale_and_min(h: &TestHarness) -> (Pos2, f32) {
    let slide_rect = h.editor.ui.last_slide_rect.expect("slide rect");
    let slide_w = h.editor.session.course.slide_size.w as f32;
    (slide_rect.min, slide_rect.width() / slide_w)
}

fn to_screen(h: &TestHarness, slide_pos: Pos2) -> Pos2 {
    let (min, scale) = slide_scale_and_min(h);
    min + vec2(slide_pos.x * scale, slide_pos.y * scale)
}

fn drag(h: &mut TestHarness, from: Pos2, to: Pos2) {
    h.frame(vec![Event::PointerMoved(from)]);
    h.frame(vec![Event::PointerButton {
        pos: from,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    }]);
    h.frame(vec![Event::PointerMoved(to)]);
    h.frame(vec![Event::PointerButton {
        pos: to,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    }]);
    h.idle_frame();
}

// ---------------------------------------------------------------------------
// 3.9: Shortcuts with a modifier must not fire while typing
// ---------------------------------------------------------------------------

#[test]
fn ctrl_z_while_inline_editing_undoes_typing_not_course() {
    let mut h = TestHarness::new(hello());
    // Create a course undo step before entering inline edit.
    h.editor
        .run("slide.add", serde_json::json!({ "title": "Extra" }));
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.object = Some("title".into());
    h.idle_frame();
    h.idle_frame();

    let undo_before = h.editor.session.undo_len();
    assert!(undo_before >= 1);

    let (min, scale) = slide_scale_and_min(&h);
    let obj_frame = h
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object("title"))
        .map(|(o, _)| o.frame)
        .expect("title");
    let center = min
        + vec2(
            (obj_frame.x + obj_frame.w * 0.5) * scale,
            (obj_frame.y + obj_frame.h * 0.5) * scale,
        );
    h.click_at(center);
    h.click_at(center);
    h.idle_frame();
    assert!(h.editor.ui.inline_edit.is_some(), "entered inline edit");

    let original = h
        .editor
        .ui
        .inline_edit
        .as_ref()
        .expect("edit")
        .original
        .clone();

    // Collapse selection and type "!".
    h.press(Key::ArrowRight, Modifiers::NONE);
    h.frame(vec![Event::Text("!".to_owned())]);
    h.idle_frame();
    assert_eq!(
        h.editor.ui.inline_edit.as_ref().expect("edit").buffer,
        format!("{original}!")
    );

    // Cmd/Ctrl+Z must undo the typing in egui, not the course edit.
    h.press(Key::Z, Modifiers::COMMAND);
    h.idle_frame();

    assert_eq!(
        h.editor.session.undo_len(),
        undo_before,
        "course undo count must not change while inline editing"
    );
    assert!(
        h.editor.ui.inline_edit.is_some(),
        "still in inline edit after Cmd/Ctrl+Z"
    );
    assert_eq!(
        h.editor.ui.inline_edit.as_ref().expect("edit").buffer,
        original,
        "typing was undone by egui"
    );
}

#[test]
fn ctrl_m_and_ctrl_d_while_inline_editing_do_not_add_slide_or_duplicate() {
    let mut h = TestHarness::new(hello());
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.object = Some("title".into());
    h.idle_frame();
    h.idle_frame();

    let slides_before = h.editor.session.course.slides().count();
    let objects_before = h
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("s-intro")
        .objects
        .len();

    let (min, scale) = slide_scale_and_min(&h);
    let obj_frame = h
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object("title"))
        .map(|(o, _)| o.frame)
        .expect("title");
    let center = min
        + vec2(
            (obj_frame.x + obj_frame.w * 0.5) * scale,
            (obj_frame.y + obj_frame.h * 0.5) * scale,
        );
    h.click_at(center);
    h.click_at(center);
    h.idle_frame();
    assert!(h.editor.ui.inline_edit.is_some());

    h.press(Key::M, Modifiers::COMMAND);
    h.press(Key::D, Modifiers::COMMAND);
    h.idle_frame();

    assert_eq!(
        h.editor.session.course.slides().count(),
        slides_before,
        "Ctrl+M must not add a slide while inline editing"
    );
    assert_eq!(
        h.editor
            .session
            .course
            .slide("s-intro")
            .expect("s-intro")
            .objects
            .len(),
        objects_before,
        "Ctrl+D must not duplicate while inline editing"
    );
}

// ---------------------------------------------------------------------------
// 3.10: Edit Trigger window disables list row buttons & closes if owner changes
// ---------------------------------------------------------------------------

#[test]
fn trigger_row_delete_disabled_while_edit_form_open() {
    let mut h = TestHarness::new(hello());
    h.editor.selection.slide = "s-intro".into();
    h.editor.ui.bottom_tab = BottomTab::Triggers;
    // Ensure s-intro has at least two triggers.
    h.editor.run(
        "trigger.add",
        serde_json::json!({
            "slide": "s-intro",
            "trigger": {
                "action": { "kind": "jump_to_slide", "slide": "s-intro" },
                "when": { "kind": "timeline_starts" }
            }
        }),
    );
    h.editor.run(
        "trigger.add",
        serde_json::json!({
            "slide": "s-intro",
            "trigger": {
                "action": { "kind": "jump_to_slide", "slide": "s-intro" },
                "when": { "kind": "timeline_ends" }
            }
        }),
    );
    h.idle_frame();
    h.idle_frame();

    let count_before = h
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("s-intro")
        .triggers
        .len();
    assert!(count_before >= 2);

    // Click the first trigger row so its action buttons are visible.
    let first_row = h
        .find_text_containing("Timeline")
        .expect("first trigger row");
    h.click_at(first_row);
    h.idle_frame();

    let trash_pos = h
        .find_icon(coursecraft_render::Icon::Trash)
        .expect("delete trigger icon");

    // Now open the Edit Trigger form on trigger 1.
    let slide = h
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("s-intro")
        .clone();
    h.editor.ui.trigger_form = Some(TriggerFormState::from_existing(
        1,
        &slide.triggers[1],
        &slide,
        None,
        &h.editor.session.course,
    ));
    h.idle_frame();

    // Clicking the delete button on the list row must be disabled while the form is open.
    h.click_at(trash_pos);
    h.idle_frame();

    let count_after = h
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("s-intro")
        .triggers
        .len();
    assert_eq!(
        count_after, count_before,
        "list row delete must be disabled while Edit Trigger form is open"
    );
}

#[test]
fn edit_trigger_form_closes_when_slide_changes_underneath() {
    let mut h = TestHarness::new(hello());
    h.editor.selection.slide = "s-intro".into();
    h.idle_frame();

    let slide = h
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("s-intro")
        .clone();
    h.editor.ui.trigger_form = Some(TriggerFormState::new_for_owner(
        &slide,
        None,
        &h.editor.session.course,
    ));
    h.idle_frame();
    assert!(h.editor.ui.trigger_form.is_some());

    // Switch to another slide underneath the open form.
    let other_slide = h
        .editor
        .session
        .course
        .slides()
        .find(|s| s.id != "s-intro")
        .expect("another slide")
        .id
        .clone();
    h.editor.selection.slide = other_slide;
    h.idle_frame();

    assert!(
        h.editor.ui.trigger_form.is_none(),
        "trigger form must close when its owner slide changes"
    );
}

// ---------------------------------------------------------------------------
// 3.11: Active drag continues and releases over HUD / ruler / guide
// ---------------------------------------------------------------------------

#[test]
fn drag_continues_and_releases_when_pointer_moves_over_zoom_hud() {
    let mut h = TestHarness::new(hello());
    h.editor.ui.snap_to_guides = false;
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.object = Some("btn-start".into());
    h.idle_frame();
    h.idle_frame();

    let orig_frame = h
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object("btn-start"))
        .map(|(o, _)| o.frame)
        .expect("btn-start");
    let start_pos = to_screen(
        &h,
        Pos2::new(
            orig_frame.x + orig_frame.w * 0.5,
            orig_frame.y + orig_frame.h * 0.5,
        ),
    );
    let release_in_hud = h.find_text("Fit").expect("zoom HUD Fit button");

    drag(&mut h, start_pos, release_in_hud);

    assert!(
        h.editor.ui.canvas_drag.is_none(),
        "releasing over zoom HUD must end the active drag"
    );
    let moved_frame = h
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object("btn-start"))
        .map(|(o, _)| o.frame)
        .expect("btn-start");
    assert!(
        (moved_frame.x - orig_frame.x).abs() > 1.0 || (moved_frame.y - orig_frame.y).abs() > 1.0,
        "releasing over zoom HUD must commit the drag at that position"
    );
}

// ---------------------------------------------------------------------------
// 3.12: PathPoint, Adjust, MotionPathPoint press offset & 3 px threshold
// ---------------------------------------------------------------------------

fn add_line_path(h: &mut TestHarness) -> String {
    let res = h
        .editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s-intro",
                "frame": { "x": 100.0, "y": 100.0, "w": 200.0, "h": 100.0 },
                "kind": {
                    "type": "path",
                    "points": [
                        { "x": 0.0, "y": 0.0 },
                        { "x": 1.0, "y": 1.0 }
                    ],
                    "outline": { "color": "#000000", "width": 2.0 }
                }
            }),
        )
        .expect("add path");
    res["id"].as_str().expect("object id").to_owned()
}

#[test]
fn clicking_path_point_without_moving_does_not_commit_undo_step() {
    let mut h = TestHarness::new(hello());
    h.editor.ui.snap_to_guides = false;
    let obj_id = add_line_path(&mut h);
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.object = Some(obj_id);
    h.idle_frame();
    h.idle_frame();

    let undo_before = h.editor.session.undo_len();
    let rev_before = h.editor.session.revision;
    let pt0_screen = to_screen(&h, Pos2::new(100.0, 100.0));
    // Click 2 px off-centre without moving.
    let press_pos = pt0_screen + vec2(2.0, 0.0);
    drag(&mut h, press_pos, press_pos);

    assert_eq!(h.editor.session.undo_len(), undo_before);
    assert_eq!(h.editor.session.revision, rev_before);
}

#[test]
fn dragging_path_point_applies_delta_from_point_centre() {
    let mut h = TestHarness::new(hello());
    h.editor.ui.snap_to_guides = false;
    let obj_id = add_line_path(&mut h);
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.object = Some(obj_id.clone());
    h.idle_frame();
    h.idle_frame();

    let (_, scale) = slide_scale_and_min(&h);
    let pt0_screen = to_screen(&h, Pos2::new(100.0, 100.0));
    // Press 2 px off-centre in X, then drag +20 px in X (0 in Y).
    let press_pos = pt0_screen + vec2(2.0, 0.0);
    let release_pos = press_pos + vec2(20.0, 0.0);
    drag(&mut h, press_pos, release_pos);

    let (obj, _) = h
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object(&obj_id))
        .expect("path");
    let expected_x = 100.0 + 20.0 / scale;
    assert!(
        (obj.frame.x - expected_x).abs() < 0.5,
        "point x should move by drag delta ({expected_x}), got {}",
        obj.frame.x
    );
    assert!(
        (obj.frame.y - 100.0).abs() < 0.5,
        "point y should stay at 100.0, got {}",
        obj.frame.y
    );
}

#[test]
fn clicking_adjust_handle_without_moving_does_not_commit_undo_step() {
    let mut h = TestHarness::new(hello());
    h.editor.ui.snap_to_guides = false;
    let res = h
        .editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s-intro",
                "frame": { "x": 100.0, "y": 100.0, "w": 200.0, "h": 100.0 },
                "kind": {
                    "type": "shape",
                    "shape": "rounded_rectangle",
                    "fill": "#336699",
                    "adjust": [0.25]
                }
            }),
        )
        .expect("add shape");
    let obj_id = res["id"].as_str().expect("object id").to_owned();
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.object = Some(obj_id);
    h.idle_frame();
    h.idle_frame();

    let hits =
        coursecraft_render::adjust_handles(ShapeKind::RoundedRectangle, 200.0, 100.0, &[0.25]);
    let (hx, hy) = hits[0];
    let handle_screen = to_screen(&h, Pos2::new(100.0 + hx, 100.0 + hy));

    let undo_before = h.editor.session.undo_len();
    let rev_before = h.editor.session.revision;
    let press_pos = handle_screen + vec2(2.0, 0.0);
    drag(&mut h, press_pos, press_pos);

    assert_eq!(h.editor.session.undo_len(), undo_before);
    assert_eq!(h.editor.session.revision, rev_before);
}

#[test]
fn clicking_motion_path_point_without_moving_does_not_commit_undo_step() {
    let mut h = TestHarness::new(hello());
    h.editor.ui.snap_to_guides = false;
    let slide = h
        .editor
        .session
        .course
        .slide_mut("s-intro")
        .expect("s-intro");
    let btn = slide
        .objects
        .iter_mut()
        .find(|o| o.id == "btn-start")
        .expect("btn-start");
    btn.motion_paths.push(MotionPath {
        id: "mp-1".into(),
        relative: true,
        points: vec![Point { x: 0.0, y: 0.0 }, Point { x: 100.0, y: 0.0 }],
        duration_ms: 1000,
        easing: Default::default(),
    });
    let obj_x = btn.frame.x;
    let obj_y = btn.frame.y;
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.object = Some("btn-start".into());
    h.idle_frame();
    h.idle_frame();

    let pt_screen = to_screen(&h, Pos2::new(obj_x + 100.0, obj_y));
    let undo_before = h.editor.session.undo_len();
    let rev_before = h.editor.session.revision;
    let press_pos = pt_screen + vec2(2.0, 0.0);
    drag(&mut h, press_pos, press_pos);

    assert_eq!(h.editor.session.undo_len(), undo_before);
    assert_eq!(h.editor.session.revision, rev_before);
}
