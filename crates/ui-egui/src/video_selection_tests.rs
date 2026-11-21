//! Headless UI tests for video selection, error/timeout handling, and
//! non-blocking decode in the editor (M26 A7).

use crate::test_harness::TestHarness;
use crate::timeline_playback::{TimelineMedia, TimelinePlayback};
use coursecraft_course::{Course, ObjectKind};
use coursecraft_playback::{MediaKind, VideoFrame};
use std::sync::{Arc, Mutex};

const COURSE_WITH_VIDEO: &str = r#"{
  "schema_version": 1,
  "id": "c-vid",
  "title": "Video Test",
  "slide_size": { "w": 1280, "h": 720 },
  "scenes": [
    {
      "id": "sc1",
      "title": "Scene 1",
      "slides": [
        {
          "id": "s1",
          "title": "Slide 1",
          "duration_ms": 5000,
          "objects": [
            {
              "id": "vid1",
              "name": "Video 1",
              "frame": { "x": 100, "y": 100, "w": 640, "h": 360 },
              "kind": { "type": "video", "src": "broken.mp4", "autoplay": true }
            }
          ]
        }
      ]
    }
  ]
}"#;

#[derive(Default)]
struct FakeState {
    play_calls: usize,
    seek_calls: usize,
    ended: Vec<String>,
    fail_immediately: bool,
}

struct StubTimelineMedia {
    state: Arc<Mutex<FakeState>>,
}

impl StubTimelineMedia {
    fn failing() -> (Self, Arc<Mutex<FakeState>>) {
        let state = Arc::new(Mutex::new(FakeState {
            fail_immediately: true,
            ..Default::default()
        }));
        (
            Self {
                state: Arc::clone(&state),
            },
            state,
        )
    }

    fn hanging() -> (Self, Arc<Mutex<FakeState>>) {
        let state = Arc::new(Mutex::new(FakeState {
            fail_immediately: false,
            ..Default::default()
        }));
        (
            Self {
                state: Arc::clone(&state),
            },
            state,
        )
    }
}

impl TimelineMedia for StubTimelineMedia {
    fn play(&mut self, object: &str, _src: &str, _kind: MediaKind, _bytes: Arc<[u8]>) {
        if let Ok(mut s) = self.state.lock() {
            s.play_calls += 1;
            if s.fail_immediately {
                s.ended.push(object.to_owned());
            }
        }
    }

    fn pause(&mut self, _object: &str) {}

    fn seek(&mut self, _object: &str, _ms: u64) {
        if let Ok(mut s) = self.state.lock() {
            s.seek_calls += 1;
        }
    }

    fn position_ms(&self, _object: &str) -> Option<u64> {
        None
    }

    fn video_frame(&self, _object: &str) -> Option<Arc<VideoFrame>> {
        None
    }

    fn take_ended(&mut self) -> Vec<String> {
        if let Ok(mut s) = self.state.lock() {
            std::mem::take(&mut s.ended)
        } else {
            Vec::new()
        }
    }

    fn stop_all(&mut self) {}
}

#[test]
fn failing_video_backend_stops_retrying_and_shows_placeholder_message() {
    let course = Course::from_json(COURSE_WITH_VIDEO).expect("valid course");
    let mut harness = TestHarness::new(course);
    let (backend, state) = StubTimelineMedia::failing();
    harness
        .editor
        .set_timeline_playback(TimelinePlayback::with_backend(Box::new(backend)));
    harness.idle_frame();

    // Click the video object on the canvas to select it.
    let canvas_rect = harness.central_panel_rect();
    harness.click_at(canvas_rect.center());
    for _ in 0..6 {
        harness.idle_frame();
    }

    assert_eq!(harness.editor.selection.object.as_deref(), Some("vid1"));
    let plays = state.lock().expect("lock").play_calls;
    assert_eq!(
        plays, 1,
        "a failed video must not be re-armed on every frame (got {plays} play calls)"
    );
    assert!(
        !harness.editor.timeline_playback.needs_repaint(0),
        "failed video must not keep requesting repaints forever"
    );
    assert!(
        harness.find_text("Could not decode video").is_some(),
        "canvas should show a plain error placeholder message"
    );
}

#[test]
fn hanging_video_backend_times_out_and_stops_repainting() {
    let course = Course::from_json(COURSE_WITH_VIDEO).expect("valid course");
    let mut harness = TestHarness::new(course);
    let (backend, _state) = StubTimelineMedia::hanging();
    harness
        .editor
        .set_timeline_playback(TimelinePlayback::with_backend(Box::new(backend)));

    harness.frame_at_time(0.0);
    let canvas_rect = harness.central_panel_rect();
    harness.click_at(canvas_rect.center());

    // Advance simulated time past the decode timeout (6 seconds).
    for step in 1..=12 {
        harness.frame_at_time(f64::from(step) * 0.5);
    }

    assert_eq!(harness.editor.selection.object.as_deref(), Some("vid1"));
    assert!(
        !harness.editor.timeline_playback.needs_repaint(0),
        "a hung video must time out and stop requesting repaints"
    );
    assert!(
        harness.find_text("Video preview timed out").is_some(),
        "canvas should show a plain timeout placeholder message"
    );
}

#[test]
fn real_mp4_selected_in_harness_stays_responsive() {
    let mp4_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/healthy-habits/media/stretch.mp4"
    );
    let Ok(video_bytes) = std::fs::read(mp4_path) else {
        panic!("missing test mp4 at {mp4_path}");
    };
    let course = Course::from_json(
        r#"{
          "schema_version": 1,
          "id": "c-real",
          "title": "Real Video",
          "slide_size": { "w": 1280, "h": 720 },
          "scenes": [{ "id": "sc1", "title": "S1", "slides": [{ "id": "s1", "title": "S1", "duration_ms": 5000, "objects": [] }] }]
        }"#,
    )
    .expect("valid course");
    let mut harness = TestHarness::new(course);
    harness.idle_frame();

    let src = harness
        .editor
        .import_video("stretch.mp4", &video_bytes)
        .expect("import_video");
    assert_eq!(src, "stretch.mp4");

    // Place video by clicking the canvas center.
    let canvas_rect = harness.central_panel_rect();
    harness.click_at(canvas_rect.center());
    harness.idle_frame();

    let vid_id = harness
        .editor
        .session
        .course
        .slide("s1")
        .and_then(|s| {
            s.objects
                .iter()
                .find(|o| matches!(&o.kind, ObjectKind::Video { src: s_src, .. } if s_src == "stretch.mp4"))
                .map(|o| o.id.clone())
        })
        .expect("placed video object");

    // Deselect, then click the video on the canvas to select it.
    harness.editor.selection.clear_objects();
    harness.idle_frame();
    harness.click_at(canvas_rect.center());
    harness.idle_frame();

    assert_eq!(
        harness.editor.selection.object.as_deref(),
        Some(vid_id.as_str())
    );
    assert!(
        harness
            .find_text_in("Autoplay", harness.right_panel_rect())
            .is_some(),
        "video properties should be shown when selected"
    );
}

#[test]
fn replacing_failed_video_src_clears_error_and_retries() {
    let course = Course::from_json(COURSE_WITH_VIDEO).expect("valid course");
    let mut harness = TestHarness::new(course);
    let (backend, state) = StubTimelineMedia::failing();
    harness
        .editor
        .set_timeline_playback(TimelinePlayback::with_backend(Box::new(backend)));
    harness.idle_frame();
    harness.idle_frame();

    assert_eq!(state.lock().expect("lock").play_calls, 1);
    assert!(harness.find_text("Could not decode video").is_some());

    // Replacing src via engine command should clear the cached failure and retry once.
    harness.editor.run(
        "object.update",
        serde_json::json!({
            "slide": "s1",
            "object": "vid1",
            "kind": { "type": "video", "src": "replacement.mp4", "autoplay": true }
        }),
    );
    harness.idle_frame();
    harness.idle_frame();

    assert_eq!(state.lock().expect("lock").play_calls, 2);
}
