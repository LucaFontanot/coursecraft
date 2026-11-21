//! M26 bug-hunt tests for A7: Video selection & decode safety.
//!
//! Ordered P1 -> P2 -> P3.

use crate::properties::InspectorTab;
use crate::test_harness::TestHarness;
use crate::timeline_playback::{TimelineMedia, TimelinePlayback};
use coursecraft_course::{Course, ObjectKind};
use coursecraft_playback::{MediaKind, VideoFrame};
use egui::{Key, Modifiers, pos2, vec2};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

fn blank_video_course() -> Course {
    let json = serde_json::json!({
        "schema_version": 1,
        "id": "c-vid",
        "title": "Video Test",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "sc1",
            "title": "Scene 1",
            "slides": [{
                "id": "s1",
                "title": "Slide 1",
                "duration_ms": 5000,
                "objects": [{
                    "id": "vid1",
                    "name": "Video 1",
                    "frame": { "x": 100.0, "y": 100.0, "w": 640.0, "h": 360.0 },
                    "kind": { "type": "video", "src": "broken.mp4", "autoplay": true }
                }]
            }]
        }]
    })
    .to_string();
    Course::from_json(&json).expect("valid")
}

#[derive(Default)]
struct StubState {
    play_calls: HashMap<String, usize>,
    ended: Vec<String>,
    errors: HashMap<String, String>,
    frames: HashMap<String, Arc<VideoFrame>>,
    fail_immediately: bool,
    push_ended_on_error: bool,
}

struct StubBackend {
    state: Arc<Mutex<StubState>>,
}

impl StubBackend {
    fn new(fail_immediately: bool, push_ended_on_error: bool) -> (Self, Arc<Mutex<StubState>>) {
        let state = Arc::new(Mutex::new(StubState {
            fail_immediately,
            push_ended_on_error,
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

impl TimelineMedia for StubBackend {
    fn play(&mut self, object: &str, _src: &str, _kind: MediaKind, _bytes: Arc<[u8]>) {
        if let Ok(mut s) = self.state.lock() {
            *s.play_calls.entry(object.to_owned()).or_insert(0) += 1;
            if s.fail_immediately && (!s.errors.contains_key(object) || s.push_ended_on_error) {
                s.ended.push(object.to_owned());
            }
        }
    }
    fn pause(&mut self, _object: &str) {}
    fn seek(&mut self, _object: &str, _ms: u64) {}
    fn position_ms(&self, _object: &str) -> Option<u64> {
        None
    }
    fn video_frame(&self, object: &str) -> Option<Arc<VideoFrame>> {
        self.state
            .lock()
            .ok()
            .and_then(|s| s.frames.get(object).cloned())
    }
    fn video_error(&self, object: &str) -> Option<String> {
        self.state
            .lock()
            .ok()
            .and_then(|s| s.errors.get(object).cloned())
    }
    fn take_ended(&mut self) -> Vec<String> {
        self.state
            .lock()
            .map(|mut s| std::mem::take(&mut s.ended))
            .unwrap_or_default()
    }
    fn stop_all(&mut self) {}
}

fn stretch_bytes() -> Vec<u8> {
    std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/healthy-habits/media/stretch.mp4"
    ))
    .expect("stretch.mp4")
}

// ============================================================================
// P1
// ============================================================================

#[test]
fn a7_1_corrupt_mp4_imported_and_selected_never_freezes() {
    let mut course = blank_video_course();
    course.scenes[0].slides[0].objects.clear();
    let mut h = TestHarness::new(course);
    h.editor.import_video("corrupt.mp4", b"not a real mp4 file");
    h.idle_frame();
    let canvas = h.central_panel_rect();
    h.click_at(canvas.center());
    h.idle_frame();
    h.editor.selection.clear_objects();
    h.idle_frame();
    h.click_at(canvas.center());

    let t0 = Instant::now();
    for i in 0..15 {
        h.frame_at_time(1.0 + i as f64 * 0.5);
    }
    assert!(t0.elapsed().as_secs_f32() < 5.0, "UI froze on corrupt mp4");
    assert!(h.editor.selection.object.is_some());
    assert!(!h.editor.timeline_playback.needs_repaint(0));
    let has_msg = h.find_text_containing("Could not decode video").is_some()
        || h.find_text_containing("timed out").is_some()
        || h.find_text_containing("video").is_some();
    assert!(
        has_msg,
        "expected plain error/placeholder message on canvas"
    );
}

#[test]
fn a7_2_empty_zero_byte_and_missing_media_video_safe() {
    let json = serde_json::json!({
        "schema_version": 1,
        "id": "c-empty-missing",
        "title": "Empty Missing",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "sc1",
            "title": "Scene 1",
            "slides": [{
                "id": "s1",
                "title": "Slide 1",
                "duration_ms": 5000,
                "objects": [
                    {
                        "id": "v-empty",
                        "frame": { "x": 80.0, "y": 80.0, "w": 300.0, "h": 200.0 },
                        "kind": { "type": "video", "src": "empty.mp4", "autoplay": true }
                    },
                    {
                        "id": "v-missing",
                        "frame": { "x": 480.0, "y": 80.0, "w": 300.0, "h": 200.0 },
                        "kind": { "type": "video", "src": "missing.mp4", "autoplay": true }
                    }
                ]
            }]
        }]
    })
    .to_string();
    let course = Course::from_json(&json).expect("valid");
    let mut h = TestHarness::new(course);
    h.editor
        .media
        .push(("empty.mp4".into(), Arc::from(&[][..])));
    h.editor.selection.slide = "s1".into();
    h.editor.selection.object = Some("v-empty".into());
    h.editor.ui.inspector_tab = InspectorTab::Format;
    for _ in 0..6 {
        h.idle_frame();
    }

    let panel = h.right_panel_rect();
    let auto_lbl = h.find_text_in("Autoplay", panel).expect("Autoplay");
    h.click_at(pos2(panel.min.x + 150.0, auto_lbl.y));
    h.idle_frame();

    let central = h.central_panel_rect();
    let vm_rect =
        crate::canvas::object_screen_rect(&h.editor.session.course, "s1", "v-missing", central)
            .expect("v-missing rect");
    h.click_at(vm_rect.center());
    for _ in 0..6 {
        h.idle_frame();
    }
    h.drag_by(vm_rect.center(), vec2(40.0, 20.0), 4, Modifiers::NONE);
    h.idle_frame();
    assert_eq!(h.editor.selection.object.as_deref(), Some("v-missing"));
}

#[test]
fn a7_3_explicit_backend_error_string_surfaces_on_canvas() {
    for push_ended in [true, false] {
        let mut h = TestHarness::new(blank_video_course());
        h.editor
            .media
            .push(("broken.mp4".into(), Arc::from(&b"fake"[..])));
        let (backend, state) = StubBackend::new(push_ended, push_ended);
        state
            .lock()
            .unwrap()
            .errors
            .insert("vid1".into(), "Unsupported codec: hevc".into());
        h.editor
            .set_timeline_playback(TimelinePlayback::with_backend(Box::new(backend)));
        h.idle_frame();
        h.click_at(h.central_panel_rect().center());
        for _ in 0..5 {
            h.idle_frame();
        }
        assert_eq!(
            state.lock().unwrap().play_calls.get("vid1").copied(),
            Some(1)
        );
        assert!(!h.editor.timeline_playback.needs_repaint(0));
        assert!(
            h.find_text_containing("Unsupported codec").is_some(),
            "push_ended={push_ended}: expected explicit error on canvas"
        );
    }
}

#[test]
fn a7_4_two_videos_good_and_bad_isolated_under_rapid_clicking() {
    let json = serde_json::json!({
        "schema_version": 1,
        "id": "c-two-vid",
        "title": "Two Videos",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "sc1",
            "title": "Scene 1",
            "slides": [{
                "id": "s1",
                "title": "Slide 1",
                "duration_ms": 5000,
                "objects": [
                    {
                        "id": "v-good",
                        "frame": { "x": 60.0, "y": 100.0, "w": 400.0, "h": 300.0 },
                        "kind": { "type": "video", "src": "good.mp4", "autoplay": true }
                    },
                    {
                        "id": "v-bad",
                        "frame": { "x": 600.0, "y": 100.0, "w": 400.0, "h": 300.0 },
                        "kind": { "type": "video", "src": "bad.mp4", "autoplay": true }
                    }
                ]
            }]
        }]
    })
    .to_string();
    let course = Course::from_json(&json).expect("valid");
    let mut h = TestHarness::new(course);
    h.editor
        .media
        .push(("good.mp4".into(), Arc::from(&b"good"[..])));
    h.editor
        .media
        .push(("bad.mp4".into(), Arc::from(&b"bad"[..])));
    let (backend, state) = StubBackend::new(false, false);
    state.lock().unwrap().frames.insert(
        "v-good".into(),
        Arc::new(VideoFrame {
            pts_ms: 0,
            width: 4,
            height: 4,
            rgba: [0, 255, 0, 255].repeat(16),
        }),
    );
    h.editor
        .set_timeline_playback(TimelinePlayback::with_backend(Box::new(backend)));
    h.idle_frame();

    let central = h.central_panel_rect();
    let r_good =
        crate::canvas::object_screen_rect(&h.editor.session.course, "s1", "v-good", central)
            .expect("v-good");
    let r_bad = crate::canvas::object_screen_rect(&h.editor.session.course, "s1", "v-bad", central)
        .expect("v-bad");

    for step in 0..14 {
        let t = 1.0 + step as f64 * 0.5;
        if step % 2 == 0 {
            h.click_at(r_bad.center());
        } else {
            h.click_at(r_good.center());
        }
        h.frame_at_time(t);
    }

    assert!(h.find_text_containing("timed out").is_some());
    assert!(
        h.editor
            .timeline_playback
            .video_frame_for("v-good", 0)
            .is_some()
    );
    assert!(
        h.editor
            .timeline_playback
            .video_error_for("v-good")
            .is_none()
    );
}

#[test]
fn a7_8_preview_modal_with_broken_and_real_video_closes_cleanly() {
    // 1. Broken video in Preview
    let mut h = TestHarness::new(blank_video_course());
    h.editor
        .media
        .push(("broken.mp4".into(), Arc::from(&b"garbage"[..])));
    h.idle_frame();
    h.editor.open_preview();
    for i in 0..10 {
        h.frame_at_time(1.0 + i as f64 * 0.1);
    }
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(h.editor.preview.is_none());

    // 2. Real stretch.mp4 in Preview
    let mut c2 = blank_video_course();
    if let ObjectKind::Video { src, .. } = &mut c2.scenes[0].slides[0].objects[0].kind {
        *src = "stretch.mp4".into();
    }
    let mut h2 = TestHarness::new(c2);
    h2.editor
        .media
        .push(("stretch.mp4".into(), Arc::from(stretch_bytes())));
    h2.idle_frame();
    h2.editor.open_preview();
    for i in 0..5 {
        h2.frame_at_time(1.0 + i as f64 * 0.1);
    }
    h2.press(Key::Escape, Modifiers::NONE);
    h2.idle_frame();
    assert!(h2.editor.preview.is_none());
}

// ============================================================================
// P2
// ============================================================================

#[test]
fn a7_5_delayed_video_scrubbed_onto_off_and_back_caches_failure() {
    let mut course = blank_video_course();
    course.scenes[0].slides[0].objects[0].timing.start_ms = 2000;
    course.scenes[0].slides[0].objects[0].timing.end_ms = Some(4000);
    let mut h = TestHarness::new(course);
    h.editor
        .media
        .push(("broken.mp4".into(), Arc::from(&b"bad"[..])));
    let (backend, state) = StubBackend::new(true, true);
    h.editor
        .set_timeline_playback(TimelinePlayback::with_backend(Box::new(backend)));
    h.idle_frame();
    assert_eq!(state.lock().unwrap().play_calls.get("vid1").copied(), None);

    h.editor.ui.timeline.playhead_ms = 2500;
    for _ in 0..4 {
        h.idle_frame();
    }
    assert_eq!(
        state.lock().unwrap().play_calls.get("vid1").copied(),
        Some(1)
    );

    h.editor.ui.timeline.playhead_ms = 500;
    h.idle_frame();
    h.idle_frame();
    h.editor.ui.timeline.playhead_ms = 3000;
    for _ in 0..4 {
        h.idle_frame();
    }
    assert_eq!(
        state.lock().unwrap().play_calls.get("vid1").copied(),
        Some(1)
    );
    assert!(h.find_text_containing("Could not decode video").is_some());
}

#[test]
fn a7_6_video_on_slide_layer_selects_and_shows_placeholder() {
    let json = serde_json::json!({
        "schema_version": 1,
        "id": "c-layer-vid",
        "title": "Layer Video",
        "slide_size": { "w": 1280, "h": 720 },
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
                    "objects": [{
                        "id": "vid-layer",
                        "frame": { "x": 100.0, "y": 100.0, "w": 640.0, "h": 360.0 },
                        "kind": { "type": "video", "src": "broken.mp4", "autoplay": true }
                    }]
                }]
            }]
        }]
    })
    .to_string();
    let course = Course::from_json(&json).expect("valid");
    let mut h = TestHarness::new(course);
    h.editor
        .media
        .push(("broken.mp4".into(), Arc::from(&b"bad"[..])));
    let (backend, state) = StubBackend::new(true, true);
    h.editor
        .set_timeline_playback(TimelinePlayback::with_backend(Box::new(backend)));
    h.idle_frame();

    let canvas = h.central_panel_rect();
    let pill = h.find_text_in("L1", canvas).expect("L1 pill");
    h.click_at(pill);
    h.idle_frame();
    h.click_at(canvas.center());
    for _ in 0..6 {
        h.idle_frame();
    }
    assert_eq!(h.editor.selection.object.as_deref(), Some("vid-layer"));
    assert_eq!(
        state.lock().unwrap().play_calls.get("vid-layer").copied(),
        Some(1)
    );
    assert!(h.find_text_containing("Could not decode video").is_some());
}

#[test]
fn a7_7_drag_duplicate_delete_and_undo_delete_of_video() {
    let mut h = TestHarness::new(blank_video_course());
    h.editor
        .media
        .push(("broken.mp4".into(), Arc::from(&b"bad"[..])));
    let (backend, _) = StubBackend::new(true, true);
    h.editor
        .set_timeline_playback(TimelinePlayback::with_backend(Box::new(backend)));
    h.idle_frame();

    let canvas = h.central_panel_rect();
    h.click_at(canvas.center());
    h.idle_frame();
    h.drag_by(canvas.center(), vec2(50.0, 30.0), 4, Modifiers::NONE);
    h.press(Key::D, Modifiers::COMMAND);
    h.idle_frame();
    assert_eq!(
        h.editor.session.course.slide("s1").unwrap().objects.len(),
        2
    );

    h.press(Key::Delete, Modifiers::NONE);
    h.idle_frame();
    assert_eq!(
        h.editor.session.course.slide("s1").unwrap().objects.len(),
        1
    );

    assert!(h.editor.session.undo().is_ok());
    for _ in 0..4 {
        h.idle_frame();
    }
    assert_eq!(
        h.editor.session.course.slide("s1").unwrap().objects.len(),
        2
    );
    assert!(!h.editor.timeline_playback.needs_repaint(0));
}

// ============================================================================
// P3
// ============================================================================

#[test]
fn a7_9_video_with_trim_and_cue_point_scrubbing_past_eof() {
    let mut course = blank_video_course();
    course.scenes[0].slides[0].duration_ms = 10_000;
    if let ObjectKind::Video { src, .. } = &mut course.scenes[0].slides[0].objects[0].kind {
        *src = "stretch.mp4".into();
    }
    let mut h = TestHarness::new(course);
    h.editor
        .media
        .push(("stretch.mp4".into(), Arc::from(stretch_bytes())));
    h.editor
        .session
        .execute(
            "object.set_media",
            &serde_json::json!({
                "slide": "s1",
                "object": "vid1",
                "trim": { "start_ms": 500, "end_ms": 2000 }
            }),
        )
        .expect("object.set_media");
    h.editor
        .session
        .execute(
            "video.add_cue",
            &serde_json::json!({
                "slide": "s1",
                "object": "vid1",
                "at_ms": 1200,
                "pause": true
            }),
        )
        .expect("video.add_cue");
    h.idle_frame();

    let canvas = h.central_panel_rect();
    for ms in [0, 1500, 5000, 9000] {
        h.editor.ui.timeline.playhead_ms = ms;
        h.idle_frame();
        h.click_at(canvas.center());
        h.idle_frame();
    }
    let issues = h.editor.session.course.validate();
    assert!(
        !issues
            .iter()
            .any(|i| i.severity == coursecraft_course::Severity::Error),
        "unexpected validation errors: {issues:?}"
    );
}
