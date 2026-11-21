//! M26 bug-hunt tests for A4: Timeline scrubbing & canvas preview.
//!
//! Ordered P1 -> P2 -> P3.

use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use egui::epaint::{ClippedShape, Shape};
use egui::{Key, Modifiers, Pos2, Rect, pos2, vec2};

fn scrub_course() -> Course {
    let json = serde_json::json!({
        "schema_version": 1,
        "id": "scrub-test",
        "title": "Scrub Test",
        "scenes": [{
            "id": "sc1",
            "title": "Scene 1",
            "slides": [{
                "id": "s1",
                "title": "Slide 1",
                "duration_ms": 5000,
                "objects": [
                    {
                        "id": "delayed",
                        "name": "Delayed",
                        "kind": { "type": "text", "text": "Delayed Hello" },
                        "frame": { "x": 100.0, "y": 100.0, "w": 220.0, "h": 60.0 },
                        "timing": { "start_ms": 2000 }
                    },
                    {
                        "id": "fade",
                        "name": "Fade",
                        "kind": { "type": "text", "text": "Fade Text" },
                        "frame": { "x": 100.0, "y": 220.0, "w": 220.0, "h": 60.0 },
                        "timing": { "start_ms": 0 },
                        "entrance": { "effect": { "type": "fade" }, "duration_ms": 1000, "easing": "linear" }
                    },
                    {
                        "id": "exiting",
                        "name": "Exiting",
                        "kind": { "type": "text", "text": "Exiting Text" },
                        "frame": { "x": 100.0, "y": 340.0, "w": 220.0, "h": 60.0 },
                        "timing": { "start_ms": 0, "end_ms": 4000 },
                        "exit": { "effect": { "type": "fade" }, "duration_ms": 1000, "easing": "linear" }
                    }
                ]
            }]
        }]
    })
    .to_string();
    Course::from_json(&json).expect("valid scrub course")
}

fn scrub_ruler_to(
    harness: &mut TestHarness,
    target_ms: u32,
    duration_ms: u32,
    end_label: &str,
) -> Pos2 {
    let panel = harness
        .editor
        .ui
        .timeline
        .panel_rect
        .expect("timeline panel rect");
    let label_0 = harness.find_text_in("0s", panel).expect("0s ruler label");
    let label_end = harness
        .find_text_in(end_label, panel)
        .unwrap_or_else(|| panic!("{end_label} ruler label not found"));
    let frac = (target_ms as f32 / duration_ms as f32).clamp(0.0, 1.0);
    let x = label_0.x + (label_end.x - label_0.x) * frac;
    let pos = pos2(x, label_0.y);
    harness.click_at(pos);
    harness.idle_frame();
    pos
}

fn text_alpha_in(shapes: &[ClippedShape], label: &str, in_rect: Rect) -> Option<u8> {
    fn walk(shape: &Shape, label: &str, in_rect: Rect) -> Option<u8> {
        match shape {
            Shape::Text(t)
                if t.galley.text() == label
                    && in_rect.contains(crate::test_harness::text_center(t)) =>
            {
                let c = t
                    .galley
                    .job
                    .sections
                    .first()
                    .map(|s| s.format.color)
                    .or(t.override_text_color)
                    .unwrap_or(egui::Color32::WHITE);
                Some(c.a())
            }
            Shape::Vec(v) => v.iter().find_map(|s| walk(s, label, in_rect)),
            _ => None,
        }
    }
    shapes.iter().find_map(|cs| walk(&cs.shape, label, in_rect))
}

fn text_pos_in(shapes: &[ClippedShape], label: &str, in_rect: Rect) -> Option<Pos2> {
    fn walk(shape: &Shape, label: &str, in_rect: Rect) -> Option<Pos2> {
        match shape {
            Shape::Text(t) if t.galley.text() == label => {
                let c = crate::test_harness::text_center(t);
                in_rect.contains(c).then_some(c)
            }
            Shape::Vec(v) => v.iter().find_map(|s| walk(s, label, in_rect)),
            _ => None,
        }
    }
    shapes.iter().find_map(|cs| walk(&cs.shape, label, in_rect))
}

// ============================================================================
// P1
// ============================================================================

#[test]
fn a4_1_all_entrance_and_exit_animation_types_while_scrubbing() {
    let json = serde_json::json!({
        "schema_version": 1,
        "id": "anim-scrub",
        "title": "Anim Scrub",
        "scenes": [{
            "id": "sc1",
            "title": "Scene 1",
            "slides": [{
                "id": "s1",
                "title": "Slide 1",
                "duration_ms": 5000,
                "objects": [
                    {
                        "id": "fly",
                        "kind": { "type": "text", "text": "FlyWord" },
                        "frame": { "x": 400.0, "y": 80.0, "w": 200.0, "h": 50.0 },
                        "entrance": { "effect": { "type": "fly_in", "from": "left" }, "duration_ms": 1000, "easing": "linear" },
                        "exit": { "effect": { "type": "fly_in", "from": "right" }, "duration_ms": 1000, "easing": "linear" }
                    },
                    {
                        "id": "zoom",
                        "kind": { "type": "text", "text": "ZoomWord" },
                        "frame": { "x": 400.0, "y": 180.0, "w": 200.0, "h": 50.0 },
                        "entrance": { "effect": { "type": "zoom" }, "duration_ms": 1000, "easing": "linear" },
                        "exit": { "effect": { "type": "zoom" }, "duration_ms": 1000, "easing": "linear" }
                    },
                    {
                        "id": "wipe",
                        "kind": { "type": "text", "text": "WipeWord" },
                        "frame": { "x": 400.0, "y": 280.0, "w": 200.0, "h": 50.0 },
                        "entrance": { "effect": { "type": "wipe", "from": "left" }, "duration_ms": 1000, "easing": "linear" },
                        "exit": { "effect": { "type": "wipe", "from": "left" }, "duration_ms": 1000, "easing": "linear" }
                    },
                    {
                        "id": "flt",
                        "kind": { "type": "text", "text": "FloatWord" },
                        "frame": { "x": 400.0, "y": 380.0, "w": 200.0, "h": 50.0 },
                        "entrance": { "effect": { "type": "float", "from": "bottom" }, "duration_ms": 1000, "easing": "linear" },
                        "exit": { "effect": { "type": "float", "from": "bottom" }, "duration_ms": 1000, "easing": "linear" }
                    }
                ]
            }]
        }]
    })
    .to_string();
    let course = Course::from_json(&json).expect("valid");
    let mut h = TestHarness::new(course);
    h.idle_frame();
    h.idle_frame();
    let canvas = h.central_panel_rect();

    scrub_ruler_to(&mut h, 2500, 5000, "5s");
    let rest_shapes = h.frame_shapes(vec![]);
    let fly_rest = text_pos_in(&rest_shapes, "FlyWord", canvas).expect("FlyWord rest");
    let flt_rest = text_pos_in(&rest_shapes, "FloatWord", canvas).expect("FloatWord rest");

    scrub_ruler_to(&mut h, 500, 5000, "5s");
    let ent_shapes = h.frame_shapes(vec![]);
    let fly_ent = text_pos_in(&ent_shapes, "FlyWord", canvas).expect("FlyWord ent");
    let flt_ent_alpha = text_alpha_in(&ent_shapes, "FloatWord", canvas).expect("FloatWord ent");
    assert!(
        fly_ent.x < fly_rest.x - 10.0,
        "FlyIn ent: {fly_ent:?} vs {fly_rest:?}"
    );
    assert!(flt_ent_alpha < 200, "Float ent alpha: {flt_ent_alpha}");

    scrub_ruler_to(&mut h, 4500, 5000, "5s");
    let exit_shapes = h.frame_shapes(vec![]);
    let fly_exit = text_pos_in(&exit_shapes, "FlyWord", canvas).expect("FlyWord exit");
    let flt_exit = text_pos_in(&exit_shapes, "FloatWord", canvas).expect("FloatWord exit");
    assert!(
        fly_exit.x > fly_rest.x + 10.0,
        "FlyIn exit: {fly_exit:?} vs {fly_rest:?}"
    );
    assert!(
        flt_exit.y > flt_rest.y + 1.0,
        "Float exit: {flt_exit:?} vs {flt_rest:?}"
    );
}

#[test]
fn a4_2_scrubbing_slide_layer_with_different_duration() {
    let json = serde_json::json!({
        "schema_version": 1,
        "id": "layer-scrub",
        "title": "Layer Scrub",
        "scenes": [{
            "id": "sc1",
            "title": "Scene 1",
            "slides": [{
                "id": "s1",
                "title": "Slide 1",
                "duration_ms": 5000,
                "objects": [],
                "layers": [{
                    "id": "l1",
                    "name": "L1",
                    "duration_ms": 2000,
                    "objects": [
                        {
                            "id": "l-delayed",
                            "kind": { "type": "text", "text": "LayerDelayed" },
                            "frame": { "x": 100.0, "y": 100.0, "w": 200.0, "h": 50.0 },
                            "timing": { "start_ms": 1000 }
                        },
                        {
                            "id": "l-exit",
                            "kind": { "type": "text", "text": "LayerExit" },
                            "frame": { "x": 100.0, "y": 200.0, "w": 200.0, "h": 50.0 },
                            "timing": { "start_ms": 0 },
                            "exit": { "effect": { "type": "fade" }, "duration_ms": 500, "easing": "linear" }
                        }
                    ]
                }]
            }]
        }]
    })
    .to_string();
    let course = Course::from_json(&json).expect("valid");
    let mut h = TestHarness::new(course);
    h.idle_frame();
    h.idle_frame();

    let canvas = h.central_panel_rect();
    let pill = h.find_text_in("L1", canvas).expect("L1 pill");
    h.click_at(pill);
    h.idle_frame();

    scrub_ruler_to(&mut h, 500, 2000, "2s");
    assert!(h.find_text_in("LayerDelayed", canvas).is_none());
    assert!(h.find_text_in("LayerExit", canvas).is_some());

    scrub_ruler_to(&mut h, 1250, 2000, "2s");
    assert!(h.find_text_in("LayerDelayed", canvas).is_some());

    scrub_ruler_to(&mut h, 1750, 2000, "2s");
    let shapes = h.frame_shapes(vec![]);
    let alpha = text_alpha_in(&shapes, "LayerExit", canvas).expect("LayerExit");
    assert!(
        (80..=180).contains(&alpha),
        "expected ~half alpha, got {alpha}"
    );
}

#[test]
fn a4_3_exact_boundary_timestamps() {
    let mut h = TestHarness::new(scrub_course());
    h.idle_frame();
    h.idle_frame();
    let canvas = h.central_panel_rect();

    // At 0ms paused: static design view shows delayed object
    assert_eq!(h.editor.ui.timeline.playhead_ms, 0);
    assert!(h.find_text_in("Delayed Hello", canvas).is_some());

    // At 1999ms: delayed hidden
    h.editor.ui.timeline.playhead_ms = 1999;
    h.idle_frame();
    assert!(h.find_text_in("Delayed Hello", canvas).is_none());

    // At 2000ms: delayed visible
    h.editor.ui.timeline.playhead_ms = 2000;
    h.idle_frame();
    assert!(h.find_text_in("Delayed Hello", canvas).is_some());

    // At 4000ms (end_ms): exiting hidden
    h.editor.ui.timeline.playhead_ms = 4000;
    h.idle_frame();
    assert!(h.find_text_in("Exiting Text", canvas).is_none());

    // At 5000ms (duration_ms): delayed (end_ms=None, no exit) still visible
    h.editor.ui.timeline.playhead_ms = 5000;
    h.idle_frame();
    assert!(h.find_text_in("Delayed Hello", canvas).is_some());
}

#[test]
fn a4_7_timeline_play_advances_pauses_resumes_and_stops_at_end() {
    let mut h = TestHarness::new(scrub_course());
    h.frame_at_time(1.0);
    h.frame_at_time(1.0);
    let canvas = h.central_panel_rect();
    let panel = h.editor.ui.timeline.panel_rect.expect("timeline panel");

    let play_glyph = coursecraft_render::Icon::Play.codepoint().to_string();
    let pause_glyph = coursecraft_render::Icon::Pause.codepoint().to_string();
    let play_btn = h
        .find_text_in(&play_glyph, panel)
        .expect("timeline Play button");
    h.click_at(play_btn);
    h.frame_at_time(1.5);
    assert!(h.editor.timeline_playback.is_playing());
    assert!(h.find_text_in("Delayed Hello", canvas).is_none());

    h.frame_at_time(4.0);
    assert!(h.editor.ui.timeline.playhead_ms >= 2200);
    assert!(h.find_text_in("Delayed Hello", canvas).is_some());

    let pause_btn = h
        .find_text_in(&pause_glyph, panel)
        .expect("timeline Pause button");
    h.click_at(pause_btn);
    h.frame_at_time(4.5);
    let paused_ms = h.editor.ui.timeline.playhead_ms;
    h.frame_at_time(6.0);
    assert!(!h.editor.timeline_playback.is_playing());
    assert_eq!(h.editor.ui.timeline.playhead_ms, paused_ms);

    // Resume to end (5000ms)
    let play_btn2 = h
        .find_text_in(&play_glyph, panel)
        .expect("timeline Play button");
    h.click_at(play_btn2);
    h.frame_at_time(10.0);
    assert!(!h.editor.timeline_playback.is_playing());
    assert_eq!(h.editor.ui.timeline.playhead_ms, 5000);

    // Play again from end restarts at 0
    let play_btn3 = h
        .find_text_in(&play_glyph, panel)
        .expect("timeline Play button");
    h.click_at(play_btn3);
    h.frame_at_time(10.5);
    assert!(h.editor.timeline_playback.is_playing());
    assert!(
        h.editor.ui.timeline.playhead_ms < 1000,
        "expected restart near 0ms, got {}",
        h.editor.ui.timeline.playhead_ms
    );
}

#[test]
fn a4_8_switching_slides_or_shortening_duration_clamps_playhead() {
    let mut course = scrub_course();
    let s2: coursecraft_course::Slide = serde_json::from_value(serde_json::json!({
        "id": "s2",
        "title": "Slide 2",
        "duration_ms": 2000,
        "objects": [{
            "id": "s2-obj",
            "name": "S2Obj",
            "kind": { "type": "text", "text": "Slide Two" },
            "frame": { "x": 100.0, "y": 100.0, "w": 200.0, "h": 60.0 }
        }]
    }))
    .expect("s2");
    course.scenes[0].slides.push(s2);

    let mut h = TestHarness::new(course);
    h.idle_frame();
    h.idle_frame();
    scrub_ruler_to(&mut h, 4200, 5000, "5s");
    assert!(h.editor.ui.timeline.playhead_ms > 3500);

    let s2_row = h
        .find_text_in("Slide 2", h.left_panel_rect())
        .expect("Slide 2 in left panel");
    h.click_at(s2_row);
    h.idle_frame();
    assert!(!h.editor.timeline_playback.is_playing());
    assert!(h.editor.ui.timeline.playhead_ms <= 2000);
    assert!(
        h.find_text_in("Slide Two", h.central_panel_rect())
            .is_some()
    );

    // Return to s1, scrub to 4200, shorten duration to 2000
    let s1_row = h
        .find_text_in("Slide 1", h.left_panel_rect())
        .expect("Slide 1");
    h.click_at(s1_row);
    h.idle_frame();
    scrub_ruler_to(&mut h, 4200, 5000, "5s");
    h.editor
        .session
        .execute(
            "slide.set_duration",
            &serde_json::json!({ "slide": "s1", "ms": 2000 }),
        )
        .expect("set_duration");
    h.idle_frame();
    assert!(h.editor.ui.timeline.playhead_ms <= 2000);
}

// ============================================================================
// P2
// ============================================================================

#[test]
fn a4_4_continuous_drag_scrubbing_clamps_to_ruler_edges() {
    let mut h = TestHarness::new(scrub_course());
    h.idle_frame();
    h.idle_frame();
    let panel = h.editor.ui.timeline.panel_rect.expect("panel");
    let l0 = h.find_text_in("0s", panel).expect("0s");
    let l5 = h.find_text_in("5s", panel).expect("5s");

    // Drag from 0s to 4.5s
    h.drag_by(l0, vec2((l5.x - l0.x) * 0.9, 0.0), 10, Modifiers::NONE);
    h.idle_frame();
    assert!(
        h.find_text_in("Exiting Text", h.central_panel_rect())
            .is_none()
    );

    // Drag back past 0s
    let cur_x = l0.x + (l5.x - l0.x) * 0.9;
    h.drag_by(
        pos2(cur_x, l0.y),
        vec2(-(l5.x - l0.x) - 40.0, 0.0),
        10,
        Modifiers::NONE,
    );
    h.idle_frame();
    assert_eq!(h.editor.ui.timeline.playhead_ms, 0);
    assert!(
        h.find_text_in("Delayed Hello", h.central_panel_rect())
            .is_some()
    );

    // Drag past 5s
    h.drag_by(l0, vec2((l5.x - l0.x) + 80.0, 0.0), 10, Modifiers::NONE);
    h.idle_frame();
    assert_eq!(h.editor.ui.timeline.playhead_ms, 5000);
}

#[test]
fn a4_5_hidden_object_at_scrub_time_cannot_be_dragged() {
    let mut h = TestHarness::new(scrub_course());
    h.idle_frame();
    h.idle_frame();
    scrub_ruler_to(&mut h, 3000, 5000, "5s");

    let central = h.central_panel_rect();
    let rect =
        crate::canvas::object_screen_rect(&h.editor.session.course, "s1", "delayed", central)
            .expect("delayed rect");
    h.click_at(rect.center());
    h.idle_frame();
    assert_eq!(h.editor.selection.object.as_deref(), Some("delayed"));
    let orig_x = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object("delayed")
        .unwrap()
        .0
        .frame
        .x;

    scrub_ruler_to(&mut h, 1000, 5000, "5s");
    h.drag_by(rect.center(), vec2(80.0, 0.0), 5, Modifiers::NONE);
    h.idle_frame();

    let after_x = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object("delayed")
        .unwrap()
        .0
        .frame
        .x;
    assert!(
        (after_x - orig_x).abs() < 0.01,
        "hidden object moved: {orig_x} -> {after_x}"
    );
}

#[test]
fn a4_6_inline_edit_commits_and_closes_when_scrubbing_ruler() {
    let mut h = TestHarness::new(scrub_course());
    h.idle_frame();
    h.idle_frame();
    let central = h.central_panel_rect();
    let rect =
        crate::canvas::object_screen_rect(&h.editor.session.course, "s1", "delayed", central)
            .expect("delayed rect");
    h.click_at(rect.center());
    h.click_at(rect.center());
    h.idle_frame();
    assert!(h.editor.ui.inline_edit.is_some());
    h.type_text("!");

    scrub_ruler_to(&mut h, 1000, 5000, "5s");
    assert!(h.editor.ui.inline_edit.is_none());
    assert!(h.find_text_in("Delayed Hello!", central).is_none());
}

#[test]
fn a4_9_master_objects_stay_visible_and_editor_hidden_stays_hidden_when_scrubbed() {
    let course = scrub_course();
    let master_res = coursecraft_engine::Session::new(course.clone())
        .execute("master.add", &serde_json::json!({ "title": "M" }))
        .expect("master.add");
    let mut session = coursecraft_engine::Session::new(course);
    let mid = session
        .execute("master.add", &serde_json::json!({ "title": "M" }))
        .expect("master")["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let _ = master_res;
    session
        .execute(
            "object.add",
            &serde_json::json!({
                "slide": mid,
                "id": "m-logo",
                "kind": { "type": "text", "text": "Master Logo" },
                "frame": { "x": 500.0, "y": 500.0, "w": 200.0, "h": 40.0 }
            }),
        )
        .expect("add master obj");
    session
        .execute(
            "slide.set_master",
            &serde_json::json!({ "slides": ["s1"], "master": mid }),
        )
        .expect("set_master");
    let mut c = session.course;
    c.scenes[0].slides[0].objects[1].editor_hidden = true; // hide "Fade Text"

    let mut h = TestHarness::new(c);
    h.idle_frame();
    h.idle_frame();
    scrub_ruler_to(&mut h, 2500, 5000, "5s");
    let canvas = h.central_panel_rect();
    assert!(h.find_text_in("Master Logo", canvas).is_some());
    assert!(h.find_text_in("Fade Text", canvas).is_none());
}

// ============================================================================
// P3
// ============================================================================

#[test]
fn a4_10_spacebar_toggles_play_only_when_no_text_input_focused() {
    let mut h = TestHarness::new(scrub_course());
    h.idle_frame();
    h.idle_frame();

    h.press(Key::Space, Modifiers::NONE);
    h.idle_frame();
    assert!(h.editor.timeline_playback.is_playing());
    h.press(Key::Space, Modifiers::NONE);
    h.idle_frame();
    assert!(!h.editor.timeline_playback.is_playing());

    // Open inline edit on an object visible at t=0 ("fade")
    let central = h.central_panel_rect();
    let rect = crate::canvas::object_screen_rect(&h.editor.session.course, "s1", "fade", central)
        .expect("fade rect");
    h.click_at(rect.center());
    h.click_at(rect.center());
    h.idle_frame();
    assert!(h.editor.ui.inline_edit.is_some());
    h.press(Key::Space, Modifiers::NONE);
    h.idle_frame();
    assert!(!h.editor.timeline_playback.is_playing());
}
