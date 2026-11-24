//! M27 Step D-3: Timeline (D3.1–D3.12) and Media (D4.1–D4.11) test suite.

use crate::BottomTab;
use crate::media_library::MediaKindFilter;
use crate::test_harness::TestHarness;
use coursecraft_course::{Course, ObjectKind, Timing};
use egui::{Event, Key, Modifiers, PointerButton, Pos2, pos2, vec2};

const INTER_REGULAR: &[u8] = include_bytes!("../../render/fonts/Inter-Regular.ttf");
const INTER_BOLD: &[u8] = include_bytes!("../../render/fonts/Inter-Bold.ttf");

fn make_png_bytes(w: u32, h: u32, rgba: [u8; 4]) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((w * h * 4) as usize);
    for _ in 0..(w * h) {
        pixels.extend_from_slice(&rgba);
    }
    coursecraft_media::encode_png_rgba(w, h, &pixels).expect("encode png")
}

fn make_wav_bytes(duration_ms: u32) -> Vec<u8> {
    let sample_rate: u32 = 8000;
    let num_samples = (sample_rate * duration_ms) / 1000;
    let data_len = num_samples * 2;
    let file_len = 36 + data_len;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&file_len.to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    out.resize(44 + data_len as usize, 0);
    out
}

fn make_lottie_bytes() -> Vec<u8> {
    br#"{"v":"5.7.0","fr":30,"ip":0,"op":30,"w":240,"h":160,"layers":[]}"#.to_vec()
}

fn make_timeline_course() -> Course {
    serde_json::from_value(serde_json::json!({
        "schema_version": 1,
        "id": "m27-timeline-media",
        "title": "M27 Timeline & Media",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "scene-1",
            "title": "Scene 1",
            "slides": [
                {
                    "id": "s1",
                    "title": "Slide 1",
                    "duration_ms": 5000,
                    "cue_points": [],
                    "objects": [
                        {
                            "id": "obj1",
                            "name": "obj1",
                            "kind": { "type": "shape", "shape": "rectangle", "fill": "#2563EB" },
                            "frame": { "x": 100.0, "y": 100.0, "w": 140.0, "h": 80.0 },
                            "timing": { "start_ms": 500, "end_ms": 2500 }
                        },
                        {
                            "id": "obj2",
                            "name": "obj2",
                            "kind": { "type": "shape", "shape": "ellipse", "fill": "#16A34A" },
                            "frame": { "x": 150.0, "y": 120.0, "w": 120.0, "h": 80.0 },
                            "timing": { "start_ms": 1000 }
                        },
                        {
                            "id": "obj3",
                            "name": "obj3",
                            "kind": { "type": "text", "text": "Hello" },
                            "frame": { "x": 300.0, "y": 100.0, "w": 180.0, "h": 40.0 },
                            "timing": { "start_ms": 0, "end_ms": 3000 }
                        }
                    ],
                    "layers": [
                        {
                            "id": "l1",
                            "name": "L1",
                            "duration_ms": 4000,
                            "cue_points": [],
                            "objects": [
                                {
                                    "id": "lobj1",
                                    "name": "Layer Box",
                                    "kind": { "type": "shape", "shape": "rectangle", "fill": "#F59E0B" },
                                    "frame": { "x": 200.0, "y": 200.0, "w": 160.0, "h": 90.0 },
                                    "timing": { "start_ms": 400, "end_ms": 2000 }
                                }
                            ]
                        }
                    ]
                },
                {
                    "id": "s2",
                    "title": "Slide 2",
                    "duration_ms": 5000,
                    "objects": [
                        {
                            "id": "s2-txt",
                            "name": "s2-txt",
                            "kind": { "type": "text", "text": "Second Slide" },
                            "frame": { "x": 120.0, "y": 120.0, "w": 200.0, "h": 40.0 }
                        }
                    ]
                }
            ]
        }],
        "masters": [{
            "id": "m1",
            "title": "Master 1",
            "objects": [{
                "id": "m-shape",
                "name": "m-shape",
                "kind": { "type": "shape", "shape": "rectangle", "fill": "#334155" },
                "frame": { "x": 0.0, "y": 0.0, "w": 100.0, "h": 60.0 }
            }]
        }],
        "banks": [{
            "id": "bank-1",
            "title": "Bank 1",
            "slides": [{
                "id": "b1-s1",
                "title": "Bank Q1",
                "objects": [],
                "question": {
                    "kind": "multiple_choice",
                    "prompt": "Q1",
                    "choices": [
                        { "id": "c1", "text": "Yes", "correct": true },
                        { "id": "c2", "text": "No", "correct": false }
                    ]
                }
            }]
        }]
    }))
    .expect("valid timeline course")
}

fn drag_with_modifiers(
    h: &mut TestHarness,
    from: Pos2,
    to: Pos2,
    modifiers: Modifiers,
    steps: usize,
) {
    h.drag_by(from, to - from, steps, modifiers);
    h.idle_frame();
}

fn timing_of(h: &TestHarness, slide_id: &str, obj_id: &str) -> Timing {
    h.editor
        .session
        .course
        .slide(slide_id)
        .expect("slide")
        .object(obj_id)
        .expect("object")
        .0
        .timing
}

fn undo(h: &mut TestHarness) -> bool {
    h.editor.run("edit.undo", serde_json::json!({})).is_some()
}

fn open_timeline(h: &mut TestHarness) {
    h.editor.ui.bottom_tab = BottomTab::Timeline;
    h.idle_frame();
    h.idle_frame();
}

fn set_timeline_duration_ui(h: &mut TestHarness, text: &str) {
    let dur_rect = h
        .editor
        .ui
        .timeline
        .duration_edit_rect
        .expect("duration_edit_rect");
    h.click_at(dur_rect.center());
    h.frame(vec![Event::Key {
        key: Key::A,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::COMMAND,
    }]);
    h.frame(vec![Event::Text(text.to_owned())]);
    h.press(Key::Enter, Modifiers::NONE);
    h.idle_frame();
}

// ============================================================================
// D3. The Timeline (12 Variations)
// ============================================================================

#[test]
fn d3_1_drag_bar_move_100ms_snap_vs_shift_10ms_and_undo() {
    let mut h = TestHarness::new(make_timeline_course());
    open_timeline(&mut h);

    let bar = h.editor.ui.timeline.object_bar_rects["obj1"];
    let track_w = bar.width() / (2000.0 / 5000.0);
    let dx_155ms = (155.0 / 5000.0) * track_w;

    // Without Shift: snaps to 100ms grid, preserving 2000ms duration.
    drag_with_modifiers(
        &mut h,
        bar.center(),
        bar.center() + vec2(dx_155ms, 0.0),
        Modifiers::NONE,
        4,
    );
    let t1 = timing_of(&h, "s1", "obj1");
    assert_eq!(t1.start_ms % 100, 0, "expected 100ms snap: {t1:?}");
    assert_eq!(t1.end_ms.unwrap() - t1.start_ms, 2000);
    assert!(t1.start_ms > 500);

    // With Shift: 10ms fine snap.
    let bar2 = h.editor.ui.timeline.object_bar_rects["obj1"];
    let dx_34ms = (34.0 / 5000.0) * track_w;
    drag_with_modifiers(
        &mut h,
        bar2.center(),
        bar2.center() + vec2(dx_34ms, 0.0),
        Modifiers::SHIFT,
        4,
    );
    let t2 = timing_of(&h, "s1", "obj1");
    assert_eq!(t2.start_ms % 10, 0, "expected 10ms snap: {t2:?}");
    assert_eq!(t2.end_ms.unwrap() - t2.start_ms, 2000);
    assert_ne!(t2.start_ms, t1.start_ms);

    // Undo x 2 restores exact (500, Some(2500)).
    assert!(undo(&mut h));
    assert!(undo(&mut h));
    assert_eq!(
        timing_of(&h, "s1", "obj1"),
        Timing {
            start_ms: 500,
            end_ms: Some(2500)
        }
    );
}

#[test]
fn d3_2_trim_left_and_right_handles_with_clamping() {
    let mut h = TestHarness::new(make_timeline_course());
    open_timeline(&mut h);

    // Drag left handle of obj1 past its right edge.
    let left_h = h.editor.ui.timeline.object_left_handle_rects["obj1"];
    let right_h = h.editor.ui.timeline.object_right_handle_rects["obj1"];
    drag_with_modifiers(
        &mut h,
        left_h.center(),
        right_h.center() + vec2(80.0, 0.0),
        Modifiers::NONE,
        4,
    );
    let t = timing_of(&h, "s1", "obj1");
    assert!(
        t.start_ms < t.end_ms.unwrap(),
        "start must remain < end: {t:?}"
    );
    assert!(
        h.editor
            .session
            .course
            .validate()
            .into_iter()
            .all(|i| i.severity != coursecraft_course::Severity::Error)
    );

    // Drag right handle past its left edge.
    let left_h2 = h.editor.ui.timeline.object_left_handle_rects["obj1"];
    let right_h2 = h.editor.ui.timeline.object_right_handle_rects["obj1"];
    drag_with_modifiers(
        &mut h,
        right_h2.center(),
        left_h2.center() - vec2(80.0, 0.0),
        Modifiers::NONE,
        4,
    );
    let t2 = timing_of(&h, "s1", "obj1");
    assert!(t2.start_ms < t2.end_ms.unwrap());

    // Drag left handle far left of 0ms -> clamped to >= 0.
    let left_h3 = h.editor.ui.timeline.object_left_handle_rects["obj1"];
    drag_with_modifiers(
        &mut h,
        left_h3.center(),
        left_h3.center() - vec2(500.0, 0.0),
        Modifiers::NONE,
        4,
    );
    let t3 = timing_of(&h, "s1", "obj1");
    assert_eq!(t3.start_ms, 0);
    assert!(t3.end_ms.unwrap() > 0);
    assert!(
        h.editor
            .session
            .course
            .validate()
            .into_iter()
            .all(|i| i.severity != coursecraft_course::Severity::Error)
    );
}

#[test]
fn d3_3_show_until_end_move_extend_slide_trim_and_undo() {
    let mut h = TestHarness::new(make_timeline_course());
    open_timeline(&mut h);

    // obj2 starts at 1000 with end_ms: None ("Show until end").
    assert_eq!(
        timing_of(&h, "s1", "obj2"),
        Timing {
            start_ms: 1000,
            end_ms: None
        }
    );

    let bar = h.editor.ui.timeline.object_bar_rects["obj2"];
    drag_with_modifiers(
        &mut h,
        bar.center(),
        bar.center() + vec2(40.0, 0.0),
        Modifiers::NONE,
        4,
    );
    let moved = timing_of(&h, "s1", "obj2");
    assert!(moved.start_ms > 1000);
    assert_eq!(moved.end_ms, None, "moving must preserve end_ms: None");

    // Extend slide duration from 5s to 12s.
    set_timeline_duration_ui(&mut h, "12");
    assert_eq!(
        h.editor.session.course.slide("s1").unwrap().duration_ms,
        12_000
    );
    let extended_bar = h.editor.ui.timeline.object_bar_rects["obj2"];
    assert!(
        extended_bar.max.x >= bar.max.x - 1.0,
        "bar should extend to 12s"
    );

    // Drag right handle inward -> end_ms becomes Some(...).
    let right_h = h.editor.ui.timeline.object_right_handle_rects["obj2"];
    drag_with_modifiers(
        &mut h,
        right_h.center(),
        right_h.center() - vec2(80.0, 0.0),
        Modifiers::NONE,
        4,
    );
    let trimmed = timing_of(&h, "s1", "obj2");
    assert!(trimmed.end_ms.is_some(), "trimming right sets Some(end_ms)");

    // Undo restores end_ms: None.
    assert!(undo(&mut h));
    assert_eq!(timing_of(&h, "s1", "obj2").end_ms, None);
}

#[test]
fn d3_4_shorten_slide_duration_below_object_and_cues_validates_and_undoes() {
    let mut course = make_timeline_course();
    course.scenes[0].slides[0].duration_ms = 10_000;
    course.scenes[0].slides[0].objects[0].timing = Timing {
        start_ms: 3000,
        end_ms: Some(8000),
    };
    course.scenes[0].slides[0]
        .cue_points
        .push(coursecraft_course::SlideCue {
            id: "cue-6s".into(),
            name: "Six".into(),
            at_ms: 6000,
        });

    let mut h = TestHarness::new(course);
    open_timeline(&mut h);

    set_timeline_duration_ui(&mut h, "2");
    // Whether clamped/refused or accepted, course must remain valid and never panic.
    let errs: Vec<_> = h
        .editor
        .session
        .course
        .validate()
        .into_iter()
        .filter(|i| i.severity == coursecraft_course::Severity::Error)
        .collect();
    assert!(errs.is_empty(), "unexpected validation errors: {errs:?}");

    if h.editor.session.course.slide("s1").unwrap().duration_ms == 2000 {
        assert!(undo(&mut h));
        assert_eq!(
            h.editor.session.course.slide("s1").unwrap().duration_ms,
            10_000
        );
        assert_eq!(
            timing_of(&h, "s1", "obj1"),
            Timing {
                start_ms: 3000,
                end_ms: Some(8000)
            }
        );
        assert_eq!(
            h.editor.session.course.slide("s1").unwrap().cue_points[0].at_ms,
            6000
        );
    }
}

#[test]
fn d3_5_escape_cancels_bar_drag_and_row_reorder_drag() {
    let mut h = TestHarness::new(make_timeline_course());
    open_timeline(&mut h);
    let rev_before = h.editor.session.revision;
    let orig_timing = timing_of(&h, "s1", "obj1");
    let orig_order: Vec<String> = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .objects
        .iter()
        .map(|o| o.id.clone())
        .collect();

    // Start bar drag, move, press Escape before releasing pointer.
    let bar = h.editor.ui.timeline.object_bar_rects["obj1"];
    h.frame(vec![
        Event::PointerMoved(bar.center()),
        Event::PointerButton {
            pos: bar.center(),
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
    ]);
    h.frame(vec![Event::PointerMoved(bar.center() + vec2(80.0, 0.0))]);
    assert!(h.editor.ui.timeline.active_drag.is_some());
    h.press(Key::Escape, Modifiers::NONE);
    h.frame(vec![Event::PointerButton {
        pos: bar.center() + vec2(80.0, 0.0),
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    }]);
    h.idle_frame();
    assert_eq!(h.editor.session.revision, rev_before);
    assert_eq!(timing_of(&h, "s1", "obj1"), orig_timing);

    // Start row-reorder drag on obj1 row, move down, press Escape.
    let row = h.editor.ui.timeline.object_row_rects["obj1"];
    let grab = pos2(row.min.x + 60.0, row.center().y);
    h.frame(vec![
        Event::PointerMoved(grab),
        Event::PointerButton {
            pos: grab,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
    ]);
    h.frame(vec![Event::PointerMoved(grab + vec2(0.0, 40.0))]);
    assert!(h.editor.ui.timeline.active_row_drag.is_some());
    h.press(Key::Escape, Modifiers::NONE);
    h.frame(vec![Event::PointerButton {
        pos: grab + vec2(0.0, 40.0),
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    }]);
    h.idle_frame();
    assert_eq!(h.editor.session.revision, rev_before);
    let new_order: Vec<String> = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .objects
        .iter()
        .map(|o| o.id.clone())
        .collect();
    assert_eq!(new_order, orig_order);
}

#[test]
fn d3_6_slide_cue_points_c_shortcut_drag_rename_delete_and_undo() {
    let mut h = TestHarness::new(make_timeline_course());
    open_timeline(&mut h);

    // Focus timeline and set playhead to 2000ms.
    let panel_rect = h.editor.ui.timeline.panel_rect.expect("panel_rect");
    h.click_at(panel_rect.min + vec2(40.0, 40.0));
    h.editor.ui.timeline.playhead_ms = 2000;
    h.editor.ui.timeline.focused = true;
    h.press(Key::C, Modifiers::NONE);
    h.idle_frame();

    let cues = &h.editor.session.course.slide("s1").unwrap().cue_points;
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].at_ms, 2000);
    let cue_id = cues[0].id.clone();

    // Drag cue right to ~3200ms (move playhead away so it doesn't intercept cue drag).
    h.editor.ui.timeline.playhead_ms = 0;
    h.idle_frame();
    let cue_rect = h.editor.ui.timeline.slide_cue_rects[&cue_id];
    let bar = h.editor.ui.timeline.object_bar_rects["obj1"];
    let track_w = bar.width() / (2000.0 / 5000.0);
    let dx = (1200.0 / 5000.0) * track_w;
    drag_with_modifiers(
        &mut h,
        cue_rect.center(),
        cue_rect.center() + vec2(dx, 0.0),
        Modifiers::NONE,
        6,
    );
    let moved_ms = h.editor.session.course.slide("s1").unwrap().cue_points[0].at_ms;
    assert!(moved_ms > 2500, "expected cue moved right, got {moved_ms}");

    // Double-click cue to rename -> empty rejected -> Esc cancels -> reopen & rename to "Midpoint".
    h.advance_time(0.5);
    let cue_rect2 = h.editor.ui.timeline.slide_cue_rects[&cue_id];
    h.double_click_at(cue_rect2.center());
    h.idle_frame();
    assert!(h.editor.ui.timeline.slide_cue_rename.is_some());
    h.editor.ui.timeline.slide_cue_rename.as_mut().unwrap().text = String::new();
    let rename_btn = h.find_texts("Rename").into_iter().last().unwrap();
    h.click_at(rename_btn);
    h.idle_frame();
    assert!(
        h.editor
            .ui
            .timeline
            .slide_cue_rename
            .as_ref()
            .and_then(|r| r.error.as_ref())
            .is_some(),
        "empty cue name must show error"
    );
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(h.editor.ui.timeline.slide_cue_rename.is_none());

    h.advance_time(0.5);
    h.double_click_at(cue_rect2.center());
    h.idle_frame();
    h.editor.ui.timeline.slide_cue_rename.as_mut().unwrap().text = "Midpoint".to_owned();
    let rename_btn2 = h.find_texts("Rename").into_iter().last().unwrap();
    h.click_at(rename_btn2);
    h.idle_frame();
    assert_eq!(
        h.editor.session.course.slide("s1").unwrap().cue_points[0].name,
        "Midpoint"
    );

    // Select cue + Delete.
    h.click_at(cue_rect2.center());
    h.press(Key::Delete, Modifiers::NONE);
    h.idle_frame();
    assert!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .cue_points
            .is_empty()
    );

    // Undo x 3 restores cue name, position, then removes cue.
    assert!(undo(&mut h));
    assert_eq!(
        h.editor.session.course.slide("s1").unwrap().cue_points[0].name,
        "Midpoint"
    );
    assert!(undo(&mut h));
    assert_eq!(
        h.editor.session.course.slide("s1").unwrap().cue_points[0].at_ms,
        moved_ms
    );
    assert!(undo(&mut h));
    assert_eq!(
        h.editor.session.course.slide("s1").unwrap().cue_points[0].at_ms,
        2000
    );
}

#[test]
fn d3_7_timeline_bar_snaps_to_slide_cue_point() {
    let mut course = make_timeline_course();
    course.scenes[0].slides[0]
        .cue_points
        .push(coursecraft_course::SlideCue {
            id: "cue-snap".into(),
            name: "Snap".into(),
            at_ms: 2350,
        });
    let mut h = TestHarness::new(course);
    open_timeline(&mut h);

    let bar = h.editor.ui.timeline.object_bar_rects["obj1"];
    let track_w = bar.width() / (2000.0 / 5000.0);
    // obj1 starts at 500ms; drag +1840ms to land near 2350ms (not on 100ms grid).
    let dx = (1840.0 / 5000.0) * track_w;
    drag_with_modifiers(
        &mut h,
        bar.center(),
        bar.center() + vec2(dx, 0.0),
        Modifiers::NONE,
        6,
    );
    assert_eq!(timing_of(&h, "s1", "obj1").start_ms, 2350);
}

#[test]
fn d3_8_z_order_row_drag_to_reorder_single_group_and_multi_select() {
    let mut course = make_timeline_course();
    course.scenes[0].slides[0].objects = serde_json::from_value(serde_json::json!([
        { "id": "o1", "name": "o1", "kind": { "type": "shape", "shape": "rectangle", "fill": "#111111" }, "frame": { "x": 100.0, "y": 100.0, "w": 80.0, "h": 80.0 } },
        { "id": "o2", "name": "o2", "kind": { "type": "shape", "shape": "rectangle", "fill": "#222222" }, "frame": { "x": 110.0, "y": 110.0, "w": 80.0, "h": 80.0 }, "group": "g1" },
        { "id": "o3", "name": "o3", "kind": { "type": "shape", "shape": "rectangle", "fill": "#333333" }, "frame": { "x": 120.0, "y": 120.0, "w": 80.0, "h": 80.0 }, "group": "g1" },
        { "id": "o4", "name": "o4", "kind": { "type": "shape", "shape": "rectangle", "fill": "#444444" }, "frame": { "x": 130.0, "y": 130.0, "w": 80.0, "h": 80.0 } },
        { "id": "o5", "name": "o5", "kind": { "type": "shape", "shape": "rectangle", "fill": "#555555" }, "frame": { "x": 140.0, "y": 140.0, "w": 80.0, "h": 80.0 } }
    ]))
    .unwrap();

    let mut h = TestHarness::new(course);
    open_timeline(&mut h);

    // Drag bottom-most object (o1, displayed at bottom) above top-most (o5).
    let r_o1 = h.editor.ui.timeline.object_row_rects["o1"];
    let r_o5 = h.editor.ui.timeline.object_row_rects["o5"];
    drag_with_modifiers(
        &mut h,
        pos2(r_o1.min.x + 60.0, r_o1.center().y),
        pos2(r_o5.min.x + 60.0, r_o5.min.y + 2.0),
        Modifiers::NONE,
        6,
    );
    let ids: Vec<_> = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .objects
        .iter()
        .map(|o| o.id.as_str())
        .collect();
    assert_eq!(
        ids.last().copied(),
        Some("o1"),
        "o1 should now be frontmost: {ids:?}"
    );

    assert!(undo(&mut h));

    // Drag group member o2 above o5 -> whole group (o2, o3) moves together contiguously.
    let r_o2 = h.editor.ui.timeline.object_row_rects["o2"];
    let r_o5_b = h.editor.ui.timeline.object_row_rects["o5"];
    drag_with_modifiers(
        &mut h,
        pos2(r_o2.min.x + 60.0, r_o2.center().y),
        pos2(r_o5_b.min.x + 60.0, r_o5_b.min.y + 2.0),
        Modifiers::NONE,
        6,
    );
    let ids2: Vec<_> = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .objects
        .iter()
        .map(|o| o.id.as_str())
        .collect();
    let g_positions: Vec<_> = ids2
        .iter()
        .enumerate()
        .filter(|(_, id)| **id == "o2" || **id == "o3")
        .map(|(i, _)| i)
        .collect();
    assert_eq!(g_positions.len(), 2);
    assert_eq!(
        g_positions[1] - g_positions[0],
        1,
        "group must stay contiguous: {ids2:?}"
    );
    assert!(undo(&mut h));

    // Multi-select o1 & o4 and drag above o5.
    h.editor
        .selection
        .set_many(["o1".to_owned(), "o4".to_owned()]);
    h.idle_frame();
    let r_o1_c = h.editor.ui.timeline.object_row_rects["o1"];
    let r_o5_c = h.editor.ui.timeline.object_row_rects["o5"];
    drag_with_modifiers(
        &mut h,
        pos2(r_o1_c.min.x + 60.0, r_o1_c.center().y),
        pos2(r_o5_c.min.x + 60.0, r_o5_c.min.y - 2.0),
        Modifiers::NONE,
        6,
    );
    let ids3: Vec<_> = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .objects
        .iter()
        .map(|o| o.id.as_str())
        .collect();
    assert!(
        ids3.ends_with(&["o1", "o4"]) || ids3.ends_with(&["o4", "o1"]),
        "multi-selected rows moved to front: {ids3:?}"
    );
    assert!(undo(&mut h));
}

#[test]
fn d3_9_timeline_on_slide_layer_edits_layer_without_mutating_base() {
    let mut h = TestHarness::new(make_timeline_course());
    h.editor.selection.layer = Some("l1".to_owned());
    open_timeline(&mut h);

    let base_dur_before = h.editor.session.course.slide("s1").unwrap().duration_ms;

    // Add layer cue via C shortcut.
    let panel_rect = h.editor.ui.timeline.panel_rect.expect("panel_rect");
    h.click_at(panel_rect.min + vec2(40.0, 40.0));
    h.editor.ui.timeline.playhead_ms = 1200;
    h.editor.ui.timeline.focused = true;
    h.press(Key::C, Modifiers::NONE);
    h.idle_frame();

    let slide = h.editor.session.course.slide("s1").unwrap();
    assert!(
        slide.cue_points.is_empty(),
        "base cues must remain untouched"
    );
    assert_eq!(slide.layer("l1").unwrap().cue_points.len(), 1);

    // Drag layer object bar.
    let lbar = h.editor.ui.timeline.object_bar_rects["lobj1"];
    drag_with_modifiers(
        &mut h,
        lbar.center(),
        lbar.center() + vec2(50.0, 0.0),
        Modifiers::NONE,
        4,
    );
    assert!(timing_of(&h, "s1", "lobj1").start_ms > 400);

    // Change layer duration to 6s.
    set_timeline_duration_ui(&mut h, "6");
    let slide2 = h.editor.session.course.slide("s1").unwrap();
    assert_eq!(slide2.layer("l1").unwrap().duration_ms, 6000);
    assert_eq!(slide2.duration_ms, base_dur_before);

    // Switch back to Base.
    h.editor.selection.layer = None;
    h.idle_frame();
    assert_eq!(h.editor.ui.timeline.duration_ms, base_dur_before);
}

#[test]
fn d3_10_zoomed_and_horizontally_scrolled_timeline_coordinates() {
    let mut course = make_timeline_course();
    course.scenes[0].slides[0].duration_ms = 30_000;
    course.scenes[0].slides[0].objects[0].timing = Timing {
        start_ms: 18_000,
        end_ms: Some(21_000),
    };
    let mut h = TestHarness::new(course);
    open_timeline(&mut h);

    // Zoom 3x and scroll so 15s..25s is visible.
    h.editor.ui.timeline.zoom = 3.0;
    let panel = h.editor.ui.timeline.panel_rect.expect("panel");
    let approx_vis_w =
        panel.width() - crate::LAYERS_PANEL_WIDTH - crate::timeline::TRACK_HEADER_WIDTH;
    h.editor.ui.timeline.scroll_x = approx_vis_w * 1.5; // halfway through 3x track = 15s
    h.idle_frame();
    h.idle_frame();

    let bar = h.editor.ui.timeline.object_bar_rects["obj1"];
    let track_w = approx_vis_w * 3.0;
    let dx_1s = (1000.0 / 30_000.0) * track_w;
    drag_with_modifiers(
        &mut h,
        bar.center(),
        bar.center() + vec2(dx_1s, 0.0),
        Modifiers::NONE,
        4,
    );
    let t = timing_of(&h, "s1", "obj1");
    assert!(
        (18_500..=19_500).contains(&t.start_ms),
        "zoomed+scrolled drag should shift ~1000ms from 18000ms, got {t:?}"
    );
}

#[test]
fn d3_11_lock_hide_toggles_and_inline_object_rename_delete_mid_rename() {
    let mut h = TestHarness::new(make_timeline_course());
    open_timeline(&mut h);

    let row = h.editor.ui.timeline.object_row_rects["obj1"];
    let lock_pos = pos2(
        row.min.x + crate::timeline::TRACK_HEADER_WIDTH - 12.0,
        row.center().y,
    );
    let eye_pos = pos2(
        row.min.x + crate::timeline::TRACK_HEADER_WIDTH - 30.0,
        row.center().y,
    );

    h.click_at(lock_pos);
    h.idle_frame();
    assert!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .object("obj1")
            .unwrap()
            .0
            .locked
    );

    h.click_at(eye_pos);
    h.idle_frame();
    assert!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .object("obj1")
            .unwrap()
            .0
            .editor_hidden
    );

    // Double-click object name in left column -> rename to "Hero Banner" + Enter.
    h.advance_time(0.5);
    let name_pos = pos2(row.min.x + 50.0, row.center().y);
    h.double_click_at(name_pos);
    h.idle_frame();
    assert!(h.editor.ui.timeline.object_rename.is_some());
    h.frame(vec![Event::Key {
        key: Key::A,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::COMMAND,
    }]);
    h.frame(vec![Event::Text("Hero Banner".to_owned())]);
    h.press(Key::Enter, Modifiers::NONE);
    h.idle_frame();
    assert_eq!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .object("obj1")
            .unwrap()
            .0
            .name,
        "Hero Banner"
    );

    // Start rename again, then delete object mid-rename -> rename state clears cleanly.
    h.advance_time(0.5);
    h.double_click_at(name_pos);
    h.idle_frame();
    assert!(h.editor.ui.timeline.object_rename.is_some());
    h.editor.run(
        "object.delete",
        serde_json::json!({ "slide": "s1", "object": "obj1" }),
    );
    h.idle_frame();
    assert!(h.editor.ui.timeline.object_rename.is_none());
}

#[test]
fn d3_12_boundary_and_huge_timeline_durations_and_ruler_layout() {
    let mut h = TestHarness::new(make_timeline_course());
    open_timeline(&mut h);

    // Invalid inputs refused/clamped cleanly; valid 0.1s..86400s accepted.
    for bad in ["0", "-10", "0.05", "abc"] {
        let before = h.editor.session.course.slide("s1").unwrap().duration_ms;
        set_timeline_duration_ui(&mut h, bad);
        let after = h.editor.session.course.slide("s1").unwrap().duration_ms;
        assert!(
            (100..=86_400_000).contains(&after),
            "invalid duration {after} for {bad}"
        );
        if bad == "abc" {
            assert_eq!(after, before);
        }
    }

    // 9999999s is clamped to 86400s.
    set_timeline_duration_ui(&mut h, "9999999");
    assert_eq!(
        h.editor.session.course.slide("s1").unwrap().duration_ms,
        86_400_000
    );

    // At 86400s, ruler labels must have 0 overlaps / clips.
    h.idle_frame();
    let rects = &h.editor.ui.timeline.ruler_label_rects;
    for (i, a) in rects.iter().enumerate() {
        for b in rects.iter().skip(i + 1) {
            assert!(
                !a.intersects(*b),
                "ruler label overlap at 86400s: {a:?} vs {b:?}"
            );
        }
    }
    let report = h.layout_report(2);
    assert!(
        report.findings.is_empty(),
        "unexpected layout findings at 86400s: {report:?}"
    );
}

// ============================================================================
// D4. Media (11 Variations)
// ============================================================================

#[test]
fn d4_1_audio_and_video_insert_volume_trim_replace_and_undo() {
    let mut h = TestHarness::new(make_timeline_course());
    h.idle_frame();

    // Import audio and video, then place onto s1.
    let wav = make_wav_bytes(3000);
    let aud_src = h
        .editor
        .import_audio("voice.wav", &wav)
        .expect("import audio");
    let aud_id = h
        .editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "kind": { "type": "audio", "src": aud_src, "autoplay": false },
                "frame": { "x": 50.0, "y": 50.0, "w": 48.0, "h": 48.0 }
            }),
        )
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Valid volume (0.5) & trim (start=500, end=2000).
    assert!(
        h.editor
            .run(
                "object.set_media",
                serde_json::json!({ "slide": "s1", "object": aud_id, "volume": 0.5 })
            )
            .is_some()
    );
    assert!(
        h.editor
            .run(
                "object.set_media",
                serde_json::json!({ "slide": "s1", "object": aud_id, "trim": { "start_ms": 500, "end_ms": 2000 } })
            )
            .is_some()
    );

    // Invalid volume (-1, 2.0) and invalid trim (end <= start) refused.
    assert!(
        h.editor
            .run(
                "object.set_media",
                serde_json::json!({ "slide": "s1", "object": aud_id, "volume": -1.0 })
            )
            .is_none()
    );
    assert!(
        h.editor
            .run(
                "object.set_media",
                serde_json::json!({ "slide": "s1", "object": aud_id, "volume": 2.0 })
            )
            .is_none()
    );
    assert!(
        h.editor
            .run(
                "object.set_media",
                serde_json::json!({ "slide": "s1", "object": aud_id, "trim": { "start_ms": 2000, "end_ms": 500 } })
            )
            .is_none()
    );

    // Replace media bytes preserves object properties.
    let wav2 = make_wav_bytes(4000);
    assert!(h.editor.replace_media_bytes(&aud_src, &wav2));
    let obj = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object(&aud_id)
        .unwrap()
        .0;
    match &obj.kind {
        ObjectKind::Audio { volume, trim, .. } => {
            assert!((*volume - 0.5).abs() < 1e-4);
            assert_eq!(trim.as_ref().unwrap().start_ms, 500);
            assert_eq!(trim.as_ref().unwrap().end_ms, Some(2000));
        }
        other => panic!("expected audio, got {other:?}"),
    }

    // Undo x 3 reverts trim, volume, and object add.
    assert!(undo(&mut h));
    assert!(undo(&mut h));
    assert!(undo(&mut h));
    assert!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .object(&aud_id)
            .is_none()
    );
}

#[test]
fn d4_2_video_cue_points_and_add_question_cue_atomic_undo() {
    let mut h = TestHarness::new(make_timeline_course());
    let vid_id = h
        .editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "kind": { "type": "video", "src": "clip.mp4", "autoplay": false },
                "frame": { "x": 100.0, "y": 100.0, "w": 320.0, "h": 180.0 }
            }),
        )
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Add cue at 1500ms (pause: true), edit cue, add question cue.
    let cue_res = h
        .editor
        .run(
            "video.add_cue",
            serde_json::json!({ "slide": "s1", "object": vid_id, "at_ms": 1500, "name": "C1", "pause": true }),
        )
        .unwrap();
    let cue_id = cue_res["id"].as_str().unwrap().to_owned();
    assert!(
        h.editor
            .run(
                "video.set_cue",
                serde_json::json!({
                    "slide": "s1",
                    "object": vid_id,
                    "cue": cue_id,
                    "patch": { "name": "Edited", "at_ms": 1800 }
                })
            )
            .is_some()
    );

    let q_res = h
        .editor
        .run(
            "video.add_question_cue",
            serde_json::json!({ "slide": "s1", "object": vid_id, "at_ms": 2500 }),
        )
        .unwrap();
    let q_cue_id = q_res["cue"].as_str().unwrap().to_owned();
    assert_eq!(h.editor.session.course.slide("s1").unwrap().layers.len(), 2);

    // Undo single step reverts question cue + created layer + triggers.
    assert!(undo(&mut h));
    assert_eq!(h.editor.session.course.slide("s1").unwrap().layers.len(), 1);

    // Add a trigger referencing cue_id, then delete cue -> trigger auto-removed in same undo step.
    let trig_json = serde_json::to_value(coursecraft_course::Trigger {
        when: coursecraft_course::Event::CueReached {
            object: vid_id.clone(),
            cue: cue_id.clone(),
        },
        action: coursecraft_course::Action::PauseMedia {
            object: vid_id.clone(),
        },
        condition: None,
        disabled: false,
    })
    .unwrap();
    h.editor
        .run(
            "trigger.add",
            serde_json::json!({
                "slide": "s1",
                "trigger": trig_json,
            }),
        )
        .unwrap();
    assert_eq!(
        h.editor.session.course.slide("s1").unwrap().triggers.len(),
        1
    );
    h.editor
        .run(
            "video.remove_cue",
            serde_json::json!({ "slide": "s1", "object": vid_id, "cue": cue_id }),
        )
        .unwrap();
    assert!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .triggers
            .is_empty()
    );
    let _ = q_cue_id;
}

#[test]
fn d4_3_bug_d3_media_library_filter_insert_status_and_go_to_exits_story_view() {
    let mut h = TestHarness::new(make_timeline_course());
    let png = make_png_bytes(16, 16, [255, 0, 0, 255]);
    let wav = make_wav_bytes(1000);
    let lottie = make_lottie_bytes();
    let img_src = h.editor.import_image("hero.png", &png).unwrap();
    let _aud_src = h.editor.import_audio("sound.wav", &wav).unwrap();
    let _lot_src = h.editor.import_lottie("anim.json", &lottie).unwrap();
    let _vtt_src = h
        .editor
        .import_captions("subs.vtt", b"WEBVTT\n\n00:00.000 --> 00:01.000\nHi\n")
        .unwrap();
    h.editor.ui.placement_tool = None;

    // Open Media Library, filter by name and kind, double-click image to insert on slide.
    h.editor.ui.show_media_library = true;
    h.editor.ui.media_library.filter = "hero".to_owned();
    h.editor.ui.media_library.kind = MediaKindFilter::Images;
    h.idle_frame();
    let pos = h
        .find_texts(&img_src)
        .into_iter()
        .next()
        .expect("hero.png row in media library");
    let before_objs = h.editor.session.course.slide("s1").unwrap().objects.len();
    h.double_click_at(pos);
    h.idle_frame();
    assert_eq!(
        h.editor.session.course.slide("s1").unwrap().objects.len(),
        before_objs + 1
    );
    // BUG-D3: insert_on_slide must NOT falsely say "File operations are not undoable."
    let status = h
        .editor
        .ui
        .status_message
        .as_ref()
        .map(|(m, _)| m.as_str())
        .unwrap_or("");
    assert!(
        !status.contains("not undoable"),
        "BUG-D3: insert_on_slide must not claim object.add is not undoable; got {status:?}"
    );
    let inserted_id = h
        .editor
        .selection
        .object
        .clone()
        .expect("inserted object selected");

    // Enter Story View, select a different slide, and click "Go to" on the image usage.
    h.editor.ui.story_view = true;
    h.editor.selection.slide = "s2".to_owned();
    h.editor.selection.clear_objects();
    h.editor.ui.media_library.selected = Some(img_src.clone());
    h.idle_frame();
    let go_to_pos = h
        .find_text_containing("Slide 1 ▸")
        .expect("Slide 1 place button in media library");
    h.click_at(go_to_pos);
    h.idle_frame();
    // BUG-D3: Go to must exit Story View AND select the target object!
    assert!(
        !h.editor.ui.story_view,
        "BUG-D3: Go to must exit Story View"
    );
    assert_eq!(h.editor.selection.slide, "s1");
    assert_eq!(
        h.editor.selection.object.as_deref(),
        Some(inserted_id.as_str()),
        "BUG-D3: Go to must select the referencing object"
    );
}

#[test]
fn d4_4_media_library_delete_unused_and_undo_resurrected_missing_media() {
    let mut h = TestHarness::new(make_timeline_course());
    let png = make_png_bytes(8, 8, [0, 255, 0, 255]);
    let used_src = h.editor.import_image("used.png", &png).unwrap();
    let unused_src = h.editor.import_image("unused.png", &png).unwrap();
    h.editor.ui.placement_tool = None;

    let obj_id = h
        .editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "kind": { "type": "image", "src": used_src, "alt": "used" },
                "frame": { "x": 100.0, "y": 100.0, "w": 64.0, "h": 64.0 }
            }),
        )
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Open Media Library -> click "Delete unused" -> confirm.
    h.editor.ui.show_media_library = true;
    h.idle_frame();
    let del_unused = h.find_texts("Delete unused").into_iter().next().unwrap();
    h.click_at(del_unused);
    h.idle_frame();
    let confirm = h.find_texts("Delete").into_iter().last().unwrap();
    h.click_at(confirm);
    h.idle_frame();
    assert!(h.editor.media.iter().any(|(s, _)| s == &used_src));
    assert!(!h.editor.media.iter().any(|(s, _)| s == &unused_src));

    // Delete image object -> Delete unused -> Ctrl+Z resurrects object referencing deleted file!
    h.editor
        .run(
            "object.delete",
            serde_json::json!({ "slide": "s1", "object": obj_id }),
        )
        .unwrap();
    let del_unused2 = h.find_texts("Delete unused").into_iter().next().unwrap();
    h.click_at(del_unused2);
    h.idle_frame();
    let confirm2 = h.find_texts("Delete").into_iter().last().unwrap();
    h.click_at(confirm2);
    h.idle_frame();
    assert!(!h.editor.media.iter().any(|(s, _)| s == &used_src));

    assert!(undo(&mut h));
    assert!(h.editor.missing_media().contains(&used_src));
    h.editor.ui.show_media_library = false;
    h.idle_frame(); // Canvas renders missing placeholder with zero panics.
}

#[test]
fn d4_5_audio_video_lottie_blocked_in_master_view() {
    let mut h = TestHarness::new(make_timeline_course());
    let png = make_png_bytes(8, 8, [0, 0, 255, 255]);
    let wav = make_wav_bytes(500);
    let lottie = make_lottie_bytes();
    let aud = h.editor.import_audio("m_aud.wav", &wav).unwrap();
    let lot = h.editor.import_lottie("m_lot.json", &lottie).unwrap();
    let img = h.editor.import_image("m_img.png", &png).unwrap();
    h.editor.ui.placement_tool = None;

    h.editor.enter_master_view();
    assert_eq!(h.editor.selection.slide, "m1");

    // paste_image_png and Media Library double-click insert are blocked in Master View.
    assert!(!h.editor.paste_image_png(&png));
    let before_master_objs = h.editor.session.course.master("m1").unwrap().objects.len();
    h.editor.ui.show_media_library = true;
    h.idle_frame();
    if let Some(pos) = h.find_texts(&aud).into_iter().next() {
        h.double_click_at(pos);
        h.idle_frame();
    }
    if let Some(pos) = h.find_texts(&lot).into_iter().next() {
        h.double_click_at(pos);
        h.idle_frame();
    }
    assert_eq!(
        h.editor.session.course.master("m1").unwrap().objects.len(),
        before_master_objs
    );
    h.editor.ui.show_media_library = false;

    // Image is allowed on master where valid.
    assert!(
        h.editor
            .run(
                "object.add",
                serde_json::json!({
                    "slide": "m1",
                    "kind": { "type": "image", "src": img, "alt": "logo" },
                    "frame": { "x": 10.0, "y": 10.0, "w": 64.0, "h": 64.0 }
                })
            )
            .is_some()
    );
}

#[test]
fn d4_6_bug_d3_web_object_folder_inside_question_bank_retained_by_used_media() {
    let mut h = TestHarness::new(make_timeline_course());
    h.editor.media.push(
        "web/demo/index.html".into(),
        b"<h1>Demo</h1>".as_slice().into(),
    );
    h.editor.media.push(
        "web/demo/app.js".into(),
        b"console.log(1);".as_slice().into(),
    );
    h.editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "b1-s1",
                "kind": {
                    "type": "web",
                    "source": { "type": "folder", "path": "web/demo", "entry": "index.html" }
                },
                "frame": { "x": 100.0, "y": 100.0, "w": 400.0, "h": 300.0 }
            }),
        )
        .unwrap();

    let used: Vec<String> = h.editor.used_media().into_iter().map(|(s, _)| s).collect();
    assert!(used.contains(&"web/demo/index.html".to_owned()));
    assert!(
        used.contains(&"web/demo/app.js".to_owned()),
        "BUG-D3: web_folder_prefixes must scan course.banks; got {used:?}"
    );
}

#[test]
fn d4_7_interactive_image_crop_apply_reset_cancel_undo_and_slide_change() {
    let mut h = TestHarness::new(make_timeline_course());
    let png = make_png_bytes(100, 80, [200, 100, 50, 255]);
    let src = h.editor.import_image("photo.png", &png).unwrap();
    h.editor.ui.placement_tool = None;

    let img_id = h
        .editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "kind": { "type": "image", "src": src, "alt": "photo" },
                "frame": { "x": 100.0, "y": 100.0, "w": 200.0, "h": 160.0 }
            }),
        )
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Load texture into media_cache so loaded_image_size succeeds.
    h.idle_frame();

    assert!(h.editor.start_crop(&img_id));
    assert!(h.editor.ui.crop_edit.is_some());
    // Adjust crop window using CropHandle::Left and apply.
    let session = h.editor.ui.crop_edit.as_mut().unwrap();
    session.edit = session
        .edit
        .drag_handle(crate::crop::CropHandle::Left, vec2(20.0, 0.0), false);
    h.editor.apply_crop();
    assert!(h.editor.ui.crop_edit.is_none());
    let crop_after = match &h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object(&img_id)
        .unwrap()
        .0
        .kind
    {
        ObjectKind::Image { crop, .. } => *crop,
        _ => panic!("expected image"),
    };
    assert!(crop_after.is_some());

    // Reset crop -> returns crop: None, undoable.
    h.editor.selection.set_many([img_id.clone()]);
    h.editor.reset_crop();
    let crop_reset = match &h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object(&img_id)
        .unwrap()
        .0
        .kind
    {
        ObjectKind::Image { crop, .. } => *crop,
        _ => panic!("expected image"),
    };
    assert!(crop_reset.is_none());

    // Undo restores crop, second undo removes crop.
    assert!(undo(&mut h));
    assert!(undo(&mut h));

    // Reopen crop, mutate working rect, then cancel -> course unchanged.
    let rev_before = h.editor.session.revision;
    assert!(h.editor.start_crop(&img_id));
    h.editor.ui.crop_edit.as_mut().unwrap().edit.frame.min.x += 15.0;
    h.editor.cancel_crop();
    assert!(h.editor.ui.crop_edit.is_none());
    assert_eq!(h.editor.session.revision, rev_before);

    // Reopen crop and switch slides -> apply_crop/cancel_crop does not mutate s1.
    assert!(h.editor.start_crop(&img_id));
    h.editor.ui.crop_edit.as_mut().unwrap().edit.frame.min.x += 15.0;
    h.editor.selection.slide = "s2".to_owned();
    h.editor.apply_crop();
    assert_eq!(h.editor.session.revision, rev_before);
}

#[test]
fn d4_8_webvtt_captions_attach_replace_clear_and_hostile_path_rejection() {
    let mut h = TestHarness::new(make_timeline_course());
    let wav = make_wav_bytes(1000);
    let aud_src = h.editor.import_audio("narration.wav", &wav).unwrap();
    let aud_id = h
        .editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "kind": { "type": "audio", "src": aud_src, "autoplay": false },
                "frame": { "x": 100.0, "y": 100.0, "w": 48.0, "h": 48.0 }
            }),
        )
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Attach valid .vtt via import_captions.
    h.editor.pending_image_pick = Some(crate::PendingImagePick::ReplaceObject(aud_id.clone()));
    let vtt = b"WEBVTT\n\n00:00.000 --> 00:01.000\nHello\n";
    let cap_src = h
        .editor
        .import_captions("narration.vtt", vtt)
        .expect("valid vtt");
    assert_eq!(cap_src, "narration.vtt");
    assert!(
        h.editor
            .used_media()
            .iter()
            .any(|(s, _)| s == "narration.vtt")
    );

    // Invalid .txt rejected without leaking into editor.media; traversal path rejected by engine.
    h.editor.pending_image_pick = Some(crate::PendingImagePick::ReplaceObject(aud_id.clone()));
    assert!(h.editor.import_captions("notes.txt", b"not vtt").is_none());
    assert!(!h.editor.media.iter().any(|(s, _)| s == "notes.txt"));
    assert!(
        h.editor
            .run(
                "object.set_captions",
                serde_json::json!({ "slide": "s1", "object": aud_id, "captions": "../evil.vtt" })
            )
            .is_none()
    );

    // Clear captions and undo.
    h.editor
        .run(
            "object.set_captions",
            serde_json::json!({ "slide": "s1", "object": aud_id, "captions": null }),
        )
        .unwrap();
    assert!(undo(&mut h));
    match &h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object(&aud_id)
        .unwrap()
        .0
        .kind
    {
        ObjectKind::Audio { captions, .. } => {
            assert_eq!(captions.as_deref(), Some("narration.vtt"))
        }
        _ => panic!("expected audio"),
    }
}

#[test]
fn d4_9_background_audio_attach_edit_clear_and_bad_audio_rollback() {
    let mut h = TestHarness::new(make_timeline_course());
    let wav = make_wav_bytes(1000);
    let bg_src = h
        .editor
        .import_background_audio("bg.wav", &wav)
        .expect("bg audio");
    assert_eq!(
        h.editor
            .session
            .course
            .background_audio
            .as_ref()
            .unwrap()
            .src,
        bg_src
    );

    h.editor
        .run(
            "course.set_background_audio",
            serde_json::json!({ "src": bg_src, "volume": 0.3, "loop": false }),
        )
        .unwrap();
    let bg = h.editor.session.course.background_audio.as_ref().unwrap();
    assert!((bg.volume - 0.3).abs() < 1e-4);
    assert!(!bg.looping);

    h.editor
        .run(
            "course.set_background_audio",
            serde_json::json!({ "src": null }),
        )
        .unwrap();
    assert!(h.editor.session.course.background_audio.is_none());
    assert!(undo(&mut h));
    assert!(h.editor.session.course.background_audio.is_some());

    // Invalid empty/bad audio path does not leave orphaned bytes in editor.media.
    let media_len_before = h.editor.media.len();
    assert!(
        h.editor
            .run(
                "course.set_background_audio",
                serde_json::json!({ "src": "../evil.wav" })
            )
            .is_none()
    );
    assert_eq!(h.editor.media.len(), media_len_before);
}

#[test]
fn d4_10_hostile_media_decoding_and_case_insensitive_filename_collisions() {
    let mut h = TestHarness::new(make_timeline_course());

    // Corrupt PNG rejected cleanly.
    assert!(h.editor.import_image("bad.png", b"not a png").is_none());
    assert!(h.editor.ui.last_error.is_some());

    // SVG with <text> imports with author warning status.
    let svg_with_text = br#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"><text x="5" y="20">Hi</text></svg>"#;
    assert!(h.editor.import_image("label.svg", svg_with_text).is_some());
    assert!(
        h.editor
            .ui
            .status_message
            .as_ref()
            .is_some_and(|(m, _)| m.contains("SVG has text"))
    );

    // Invalid Lottie rejected cleanly.
    assert!(
        h.editor
            .import_lottie("bad.json", b"{not valid json")
            .is_none()
    );

    // Case-insensitive filename collision produces unique safe names without clobbering.
    let png1 = make_png_bytes(4, 4, [255, 0, 0, 255]);
    let png2 = make_png_bytes(4, 4, [0, 255, 0, 255]);
    let s1 = h.editor.import_image("photo.png", &png1).unwrap();
    let s2 = h.editor.import_image("Photo.PNG", &png2).unwrap();
    assert_eq!(s1, "photo.png");
    assert_ne!(s1.to_ascii_lowercase(), s2.to_ascii_lowercase());
    assert_eq!(
        h.editor
            .media
            .iter()
            .find(|(n, _)| n == &s1)
            .unwrap()
            .1
            .as_ref(),
        png1.as_slice()
    );
    assert_eq!(
        h.editor
            .media
            .iter()
            .find(|(n, _)| n == &s2)
            .unwrap()
            .1
            .as_ref(),
        png2.as_slice()
    );
}

#[test]
fn d4_11_bug_d3_custom_font_add_atomic_family_rename_and_remove_in_use() {
    let mut h = TestHarness::new(make_timeline_course());
    let f1 = h
        .editor
        .import_font("Inter-Regular.ttf", INTER_REGULAR)
        .expect("regular");
    let f2 = h
        .editor
        .import_font("Inter-Bold.ttf", INTER_BOLD)
        .expect("bold");
    assert_ne!(f1, f2);
    assert_eq!(h.editor.session.course.fonts.len(), 2);

    // Apply "Inter" to s2-txt.
    h.editor
        .run(
            "object.update",
            serde_json::json!({
                "slide": "s2",
                "object": "s2-txt",
                "text_style": { "font": "Inter" }
            }),
        )
        .unwrap();
    assert!(h.editor.font_family_in_use("Inter"));

    // BUG-D3: rename_font_family must be 1 atomic undo step across all faces!
    let undo_before = h.editor.session.undo_len();
    assert!(h.editor.rename_font_family("Inter", "Brand Sans"));
    assert_eq!(
        h.editor.session.undo_len(),
        undo_before + 1,
        "BUG-D3: rename_font_family must batch into 1 undo step"
    );
    assert!(
        h.editor
            .session
            .course
            .fonts
            .iter()
            .all(|f| f.family == "Brand Sans")
    );
    assert!(undo(&mut h));
    assert!(
        h.editor
            .session
            .course
            .fonts
            .iter()
            .all(|f| f.family == "Inter"),
        "BUG-D3: single undo must restore all faces to Inter"
    );

    // Remove font while text uses it -> validates cleanly with fallback warning/zero errors.
    assert!(h.editor.remove_font_family("Inter"));
    assert!(h.editor.session.course.fonts.is_empty());
    let errs: Vec<_> = h
        .editor
        .session
        .course
        .validate()
        .into_iter()
        .filter(|i| i.severity == coursecraft_course::Severity::Error)
        .collect();
    assert!(errs.is_empty());
}
