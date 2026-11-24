//! Idle-repaint tests (M21 step 2).
//!
//! When nothing on screen changes, the editor must not ask egui for another
//! frame. These tests count repaint requests; they do not measure time, so
//! they run in the normal test suite. The welcome-screen CPU bursts reported
//! from the installed app were a symptom of an unconditional repaint request;
//! these tests pin the behaviour so it cannot creep back in.
//!
//! How the check works: after ten input-free settling frames we run one more
//! input-free frame and capture the `FullOutput` from `Context::run_ui`.
//! `requested_repaint_last_pass()` reports whether the settling frames asked
//! for an immediate repaint. `output.viewport_output[..].repaint_delay` is the
//! delay egui will wait before the next frame - `Duration::MAX` means no
//! repaint is pending, `Duration::ZERO` means a repaint was just requested,
//! and any other value is a delayed repaint. We allow a pending repaint only
//! when that delay is at least one second (timers and background work), which
//! we read straight from the output rather than guessing.

use std::time::Duration;

use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use egui::RawInput;

const HELLO_COURSE: &str = include_str!("../../../examples/hello-course/course.json");

fn hello() -> TestHarness {
    let course = Course::from_json(HELLO_COURSE).expect("hello course parses");
    TestHarness::new(course)
}

/// Run one input-free frame and return the repaint delay egui recorded for it.
fn idle_frame_delay(h: &mut TestHarness) -> Duration {
    let raw_input = RawInput {
        screen_rect: Some(h.screen_rect),
        ..Default::default()
    };
    let output = h.ctx.run_ui(raw_input, |ui| {
        let _ = h.editor.show(ui);
    });
    let delay = output
        .viewport_output
        .get(&h.ctx.viewport_id())
        .map_or(Duration::ZERO, |viewport| viewport.repaint_delay);
    output.drop_without_applying_deltas();
    delay
}

/// Settle, then assert that one more input-free frame asks for no repaint
/// (or only a delayed one of at least one second).
fn assert_quiet(h: &mut TestHarness, label: &str) {
    for _ in 0..10 {
        let _ = h.idle_frame();
    }
    let delay = idle_frame_delay(h);
    let pending = h.ctx.has_requested_repaint();
    assert!(
        !h.ctx.requested_repaint_last_pass(),
        "{label}: a repaint was requested during the settling frames"
    );
    if pending {
        assert!(
            delay >= Duration::from_secs(1),
            "{label}: a repaint is pending with delay {delay:?} (under 1 s, or 0 s = immediate)"
        );
    }
}

#[test]
fn editor_idle_with_slide_selected() {
    let mut h = hello();
    assert_quiet(&mut h, "editor with a slide selected");
}

#[test]
fn welcome_screen_idle() {
    let mut h = hello();
    h.editor.ui.show_welcome = true;
    h.editor.ui.has_loaded_course = false;
    assert_quiet(&mut h, "welcome screen");
}

#[test]
fn preferences_dialog_idle() {
    let mut h = hello();
    h.editor.open_preferences();
    assert_quiet(&mut h, "Preferences dialog left open");
}

#[test]
fn interaction_gallery_idle() {
    let mut h = hello();
    h.editor.ui.show_interaction_gallery = true;
    assert_quiet(&mut h, "Insert Interaction gallery left open");
}

#[test]
fn canvas_hover_idle() {
    let mut h = hello();
    // Settle first so slide ids and rects are known, then park the pointer
    // over the canvas and stop. Hover-only widgets stay visible while idle.
    for _ in 0..4 {
        let _ = h.idle_frame();
    }
    let canvas = h.central_panel_rect().center();
    h.hover_at(canvas);
    assert_quiet(&mut h, "editor after pointer stopped over the canvas");
}

#[test]
fn playback_requests_repaint_then_stops() {
    let mut h = hello();
    // Let the first frame bind the timeline to the selected slide before
    // starting Play, otherwise sync_slide sees a new slide and pauses it.
    for _ in 0..4 {
        let _ = h.idle_frame();
    }
    h.editor.toggle_timeline_playback(4000);
    for _ in 0..5 {
        let delay = idle_frame_delay(&mut h);
        assert_eq!(
            delay,
            Duration::ZERO,
            "timeline playback must request an immediate repaint while running"
        );
    }
    assert!(
        h.ctx.requested_repaint_last_pass(),
        "timeline playback must keep asking for repaints while running"
    );
    h.editor.toggle_timeline_playback(4000);
    assert_quiet(&mut h, "editor after playback stops");
}

/// G-01 (preview): Editor preview wakes while its slide timeline is running,
/// then stops requesting repaints once the timeline ends and settles.
#[test]
fn g01_preview_idle_after_timeline_ends_does_not_repaint() {
    let mut course = Course::from_json(HELLO_COURSE).expect("hello course parses");
    if let Some(slide) = course.slide_mut("s-intro") {
        slide.duration_ms = 300;
    }
    let mut h = TestHarness::new(course);
    h.editor.open_preview();
    if let Some(preview) = h.editor.preview.as_mut() {
        preview.frame.reduced_motion = true;
    }

    // While the 300 ms slide is running, preview must schedule a wake.
    let active_delay = idle_frame_delay(&mut h);
    assert!(
        active_delay < Duration::from_secs(1),
        "active preview timeline must schedule a wake, got {active_delay:?}"
    );

    // Advance virtual time well past the 300 ms duration so the slide finishes.
    for _ in 0..8 {
        h.advance_time(0.1);
        let _ = h.idle_frame();
    }
    let ended = h
        .editor
        .preview
        .as_ref()
        .and_then(|p| p.state.as_ref())
        .is_some_and(|st| st.base.ended);
    assert!(ended, "preview slide timeline should have ended");

    assert_quiet(&mut h, "editor preview after slide timeline ends");
}
