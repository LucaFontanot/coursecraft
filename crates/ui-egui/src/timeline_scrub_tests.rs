//! Headless UI tests for timeline scrubbing and canvas time-based preview (A4).

use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use egui::{Pos2, Rect, pos2};

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
                        "entrance": {
                            "effect": { "type": "fade" },
                            "duration_ms": 1000,
                            "easing": "linear"
                        }
                    },
                    {
                        "id": "exiting",
                        "name": "Exiting",
                        "kind": { "type": "text", "text": "Exiting Text" },
                        "frame": { "x": 100.0, "y": 340.0, "w": 220.0, "h": 60.0 },
                        "timing": { "start_ms": 0, "end_ms": 4000 },
                        "exit": {
                            "effect": { "type": "fade" },
                            "duration_ms": 1000,
                            "easing": "linear"
                        }
                    }
                ]
            }]
        }]
    })
    .to_string();
    Course::from_json(&json).expect("valid scrub course")
}

/// Click on the timeline ruler at `target_ms` using the painted `0s` and `5s`
/// ruler labels so we exercise the real pointer-scrubbing path.
fn scrub_ruler_to(harness: &mut TestHarness, target_ms: u32, duration_ms: u32) -> Pos2 {
    let panel = harness
        .editor
        .ui
        .timeline
        .panel_rect
        .expect("timeline panel rect");
    let label_0 = harness.find_text_in("0s", panel).expect("0s ruler label");
    let label_end = harness.find_text_in("5s", panel).expect("5s ruler label");
    // The 0s label is drawn slightly to the right of `track_rect.min.x`, and
    // the 5s label slightly to the left of `track_rect.max.x`, so use the
    // ruler_label_rects span or interpolate directly from the label positions.
    let frac = (target_ms as f32 / duration_ms as f32).clamp(0.0, 1.0);
    let x = label_0.x + (label_end.x - label_0.x) * frac;
    let pos = pos2(x, label_0.y);
    harness.click_at(pos);
    harness.idle_frame();
    pos
}

fn painted_text_alpha_in(harness: &mut TestHarness, label: &str, region: Rect) -> Option<u8> {
    fn walk(shape: &egui::epaint::Shape, label: &str, region: Rect) -> Option<u8> {
        match shape {
            egui::epaint::Shape::Text(t)
                if t.galley.text() == label
                    && region.contains(crate::test_harness::text_center(t)) =>
            {
                let a = t
                    .galley
                    .job
                    .sections
                    .first()
                    .map(|s| s.format.color.a())
                    .unwrap_or_else(|| t.override_text_color.map_or(255, |c| c.a()));
                Some(a)
            }
            egui::epaint::Shape::Vec(v) => v.iter().find_map(|s| walk(s, label, region)),
            _ => None,
        }
    }
    let raw_input = egui::RawInput {
        screen_rect: Some(harness.screen_rect),
        ..Default::default()
    };
    let output = harness.ctx.run_ui(raw_input, |ui| {
        let _ = harness.editor.show(ui);
    });
    let alpha = output
        .shapes
        .iter()
        .find_map(|cs| walk(&cs.shape, label, region));
    output.drop_without_applying_deltas();
    alpha
}

#[test]
fn scrubbing_ruler_hides_before_start_shows_after_start_and_fades_at_midpoint() {
    let mut harness = TestHarness::new(scrub_course());
    harness.idle_frame();
    harness.idle_frame();

    let canvas = harness.central_panel_rect();

    // At playhead 0 (not playing), static design view shows the delayed object.
    assert_eq!(harness.editor.ui.timeline.playhead_ms, 0);
    assert!(
        harness.find_text_in("Delayed Hello", canvas).is_some(),
        "at playhead 0, delayed object is visible for editing"
    );

    // Scrub to ~1 s: delayed object (start 2 s) must not be drawn on canvas.
    scrub_ruler_to(&mut harness, 1000, 5000);
    assert!(
        (800..=1200).contains(&harness.editor.ui.timeline.playhead_ms),
        "expected playhead near 1000ms, got {}",
        harness.editor.ui.timeline.playhead_ms
    );
    assert!(
        harness.find_text_in("Delayed Hello", canvas).is_none(),
        "object with start 2s must not be drawn at 1s"
    );

    // Scrub to ~3 s: delayed object (start 2 s) must be drawn on canvas.
    scrub_ruler_to(&mut harness, 3000, 5000);
    assert!(
        (2800..=3200).contains(&harness.editor.ui.timeline.playhead_ms),
        "expected playhead near 3000ms, got {}",
        harness.editor.ui.timeline.playhead_ms
    );
    assert!(
        harness.find_text_in("Delayed Hello", canvas).is_some(),
        "object with start 2s must be drawn at 3s"
    );

    // Scrub to 500 ms (midpoint of 1000 ms linear fade-in): alpha should be ~half.
    scrub_ruler_to(&mut harness, 500, 5000);
    let alpha =
        painted_text_alpha_in(&mut harness, "Fade Text", canvas).expect("Fade Text drawn at 500ms");
    assert!(
        (100..=160).contains(&alpha),
        "fade-in at midpoint should be ~half-transparent (alpha ~128), got {alpha}"
    );

    // Scrub to 3500 ms (midpoint of 1000 ms linear fade-out ending at 4000 ms).
    scrub_ruler_to(&mut harness, 3500, 5000);
    let exit_alpha = painted_text_alpha_in(&mut harness, "Exiting Text", canvas)
        .expect("Exiting Text drawn at 3500ms");
    assert!(
        (100..=160).contains(&exit_alpha),
        "fade-out at midpoint should be ~half-transparent (alpha ~128), got {exit_alpha}"
    );

    // Scrub to 4500 ms (after end_ms 4000 ms): Exiting Text must not be drawn.
    scrub_ruler_to(&mut harness, 4500, 5000);
    assert!(
        harness.find_text_in("Exiting Text", canvas).is_none(),
        "object with end_ms 4s must not be drawn at 4.5s"
    );

    // Scrub back to 0 s by dragging the ruler to the left edge: static design
    // view returns for editing.
    let panel = harness
        .editor
        .ui
        .timeline
        .panel_rect
        .expect("timeline panel rect");
    let label_0 = harness.find_text_in("0s", panel).expect("0s ruler label");
    harness.drag_by(label_0, egui::vec2(-30.0, 0.0), 3, egui::Modifiers::NONE);
    harness.idle_frame();
    assert_eq!(harness.editor.ui.timeline.playhead_ms, 0);
    assert!(
        harness.find_text_in("Delayed Hello", canvas).is_some(),
        "scrubbing back to 0s restores static design view"
    );
}

#[test]
fn selecting_and_dragging_works_when_scrubbed_and_hidden_objects_are_not_picked() {
    let mut harness = TestHarness::new(scrub_course());
    harness.idle_frame();
    harness.idle_frame();

    let canvas = harness.central_panel_rect();
    let delayed_pos = harness
        .find_text_in("Delayed Hello", canvas)
        .expect("delayed object visible at 0s");

    // Scrub to 1 s: delayed object is hidden, so clicking its spot must not select it.
    scrub_ruler_to(&mut harness, 1000, 5000);
    harness.click_at(delayed_pos);
    harness.idle_frame();
    assert_eq!(
        harness.editor.selection.object, None,
        "hidden object must not be selectable at 1s"
    );

    // Scrub to 3 s: delayed object is visible, so clicking selects it and dragging moves it.
    scrub_ruler_to(&mut harness, 3000, 5000);
    let delayed_pos = harness
        .find_text_in("Delayed Hello", canvas)
        .expect("delayed object visible at 3s");
    harness.click_at(delayed_pos);
    harness.idle_frame();
    assert_eq!(
        harness.editor.selection.object.as_deref(),
        Some("delayed"),
        "visible object is selectable at 3s"
    );

    let orig_x = harness
        .editor
        .session
        .course
        .slide("s1")
        .and_then(|s| s.object("delayed"))
        .expect("delayed object")
        .0
        .frame
        .x;
    harness.drag_by(delayed_pos, egui::vec2(60.0, 0.0), 4, egui::Modifiers::NONE);
    harness.idle_frame();
    let new_x = harness
        .editor
        .session
        .course
        .slide("s1")
        .and_then(|s| s.object("delayed"))
        .expect("delayed object")
        .0
        .frame
        .x;
    assert!(
        new_x > orig_x + 10.0,
        "dragging visible object while scrubbed must move it ({orig_x} -> {new_x})"
    );
}

#[test]
fn timeline_play_shows_time_based_view_from_zero() {
    let mut harness = TestHarness::new(scrub_course());
    harness.idle_frame();
    harness.idle_frame();

    let canvas = harness.central_panel_rect();
    let play_btn = harness
        .editor
        .ui
        .timeline
        .play_button_rect
        .expect("timeline play button")
        .center();
    harness.click_at(play_btn);
    harness.idle_frame();

    assert!(
        harness.editor.timeline_playback.is_playing(),
        "clicking Play starts timeline playback"
    );
    assert!(
        harness.find_text_in("Delayed Hello", canvas).is_none(),
        "during Play at 0s, object starting at 2s must not be drawn"
    );
}
