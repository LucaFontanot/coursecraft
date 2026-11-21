//! Reproduction and regression tests for f23:
//! 1. Timeline plays to the slide's end past a shorter video (and holds last frame).
//! 2. Timeline vertical scrollbar does not cover the end handle of bars ending at slide duration.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::test_harness::TestHarness;
use crate::timeline_playback::{TimelineMedia, TimelinePlayback};
use coursecraft_course::Course;
use coursecraft_playback::{MediaKind, VideoFrame};
use egui::{Modifiers, pos2, vec2};

/// A deterministic media backend simulating finite-duration media (e.g. a 3 s video on a 5 s slide).
/// Models `RodioBackend` behaviour: `position_ms` advances with `advance_clock` up to `media_dur_ms`,
/// at which point `take_ended()` reports the object and stops it while keeping its last frame.
struct FiniteMediaBackend {
    durations_ms: HashMap<String, u64>,
    state: Mutex<FiniteState>,
}

#[derive(Default)]
struct FiniteState {
    playing: HashSet<String>,
    positions: HashMap<String, u64>,
    frames: HashMap<String, Arc<VideoFrame>>,
    ended: Vec<String>,
    play_counts: HashMap<String, usize>,
}

impl FiniteMediaBackend {
    fn new(durations_ms: Vec<(&str, u64)>) -> Self {
        Self {
            durations_ms: durations_ms
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v))
                .collect(),
            state: Mutex::new(FiniteState::default()),
        }
    }

    fn make_frame(pts_ms: u64) -> Arc<VideoFrame> {
        Arc::new(VideoFrame {
            pts_ms,
            width: 16,
            height: 16,
            rgba: vec![200; 16 * 16 * 4],
        })
    }

    fn play_count(&self, object: &str) -> usize {
        self.state
            .lock()
            .expect("lock")
            .play_counts
            .get(object)
            .copied()
            .unwrap_or(0)
    }

    fn reset_play_counts(&self) {
        self.state.lock().expect("lock").play_counts.clear();
    }
}

impl TimelineMedia for FiniteMediaBackend {
    fn play(&mut self, object: &str, _src: &str, kind: MediaKind, _bytes: Arc<[u8]>) {
        let mut st = self.state.lock().expect("lock");
        st.playing.insert(object.to_owned());
        *st.play_counts.entry(object.to_owned()).or_insert(0) += 1;
        let pos = *st.positions.entry(object.to_owned()).or_insert(0);
        if kind == MediaKind::Video {
            st.frames.insert(object.to_owned(), Self::make_frame(pos));
        }
    }

    fn pause(&mut self, object: &str) {
        let mut st = self.state.lock().expect("lock");
        st.playing.remove(object);
    }

    fn seek(&mut self, object: &str, ms: u64) {
        let dur = self.durations_ms.get(object).copied().unwrap_or(u64::MAX);
        let clamped = ms.min(dur);
        let mut st = self.state.lock().expect("lock");
        st.positions.insert(object.to_owned(), clamped);
        st.frames
            .insert(object.to_owned(), Self::make_frame(clamped));
        if st.playing.contains(object) && ms >= dur {
            st.playing.remove(object);
            if !st.ended.contains(&object.to_owned()) {
                st.ended.push(object.to_owned());
            }
        }
    }

    fn position_ms(&self, object: &str) -> Option<u64> {
        let st = self.state.lock().expect("lock");
        if st.playing.contains(object) {
            st.positions.get(object).copied()
        } else {
            None
        }
    }

    fn video_frame(&self, object: &str) -> Option<Arc<VideoFrame>> {
        self.state.lock().expect("lock").frames.get(object).cloned()
    }

    fn take_ended(&mut self) -> Vec<String> {
        let mut st = self.state.lock().expect("lock");
        std::mem::take(&mut st.ended)
    }

    fn stop_all(&mut self) {
        let mut st = self.state.lock().expect("lock");
        st.playing.clear();
        st.positions.clear();
        st.ended.clear();
    }

    fn advance_clock(&mut self, delta_ms: u64) {
        let mut st = self.state.lock().expect("lock");
        let playing: Vec<String> = st.playing.iter().cloned().collect();
        for object in playing {
            let dur = self.durations_ms.get(&object).copied().unwrap_or(u64::MAX);
            let cur = st.positions.get(&object).copied().unwrap_or(0);
            let next = cur.saturating_add(delta_ms).min(dur);
            st.positions.insert(object.clone(), next);
            st.frames.insert(object.clone(), Self::make_frame(next));
            if next >= dur {
                st.playing.remove(&object);
                if !st.ended.contains(&object) {
                    st.ended.push(object);
                }
            }
        }
    }
}

struct SharedBackend(Arc<Mutex<FiniteMediaBackend>>);
impl TimelineMedia for SharedBackend {
    fn play(&mut self, object: &str, src: &str, kind: MediaKind, bytes: Arc<[u8]>) {
        self.0.lock().expect("lock").play(object, src, kind, bytes);
    }
    fn pause(&mut self, object: &str) {
        self.0.lock().expect("lock").pause(object);
    }
    fn seek(&mut self, object: &str, ms: u64) {
        self.0.lock().expect("lock").seek(object, ms);
    }
    fn position_ms(&self, object: &str) -> Option<u64> {
        self.0.lock().expect("lock").position_ms(object)
    }
    fn video_frame(&self, object: &str) -> Option<Arc<VideoFrame>> {
        self.0.lock().expect("lock").video_frame(object)
    }
    fn take_ended(&mut self) -> Vec<String> {
        self.0.lock().expect("lock").take_ended()
    }
    fn stop_all(&mut self) {
        self.0.lock().expect("lock").stop_all();
    }
    fn advance_clock(&mut self, delta_ms: u64) {
        self.0.lock().expect("lock").advance_clock(delta_ms);
    }
}

fn course_with_media(duration_ms: u32, objects: Vec<serde_json::Value>) -> Course {
    let json = serde_json::json!({
        "schema_version": 1,
        "id": "f23-video-slide",
        "title": "F23 Video Slide",
        "scenes": [{
            "id": "sc1",
            "title": "Scene 1",
            "slides": [{
                "id": "s1",
                "title": "Slide 15",
                "duration_ms": duration_ms,
                "objects": objects
            }]
        }]
    })
    .to_string();
    Course::from_json(&json).expect("valid course")
}

fn click_timeline_play(h: &mut TestHarness) {
    let panel = h.editor.ui.timeline.panel_rect.expect("timeline panel");
    let play_glyph = coursecraft_render::Icon::Play.codepoint().to_string();
    let play_btn = h
        .find_text_in(&play_glyph, panel)
        .expect("timeline Play button");
    h.click_at(play_btn);
}

fn step_playback(
    h: &mut TestHarness,
    backend: &Arc<Mutex<FiniteMediaBackend>>,
    start_t: f64,
    steps: usize,
    step_s: f64,
) -> f64 {
    let mut t = start_t;
    let delta_ms = (step_s * 1000.0).round() as u64;
    for _ in 0..steps {
        t += step_s;
        backend.lock().expect("lock").advance_clock(delta_ms);
        h.frame_at_time(t);
    }
    t
}

#[test]
fn f23_play_in_timeline_continues_past_3s_video_to_5s_slide_end_and_replays() {
    let course = course_with_media(
        5000,
        vec![serde_json::json!({
            "id": "vid1",
            "name": "Short Video",
            "kind": { "type": "video", "src": "short.mp4", "autoplay": true },
            "frame": { "x": 100.0, "y": 100.0, "w": 400.0, "h": 225.0 },
            "timing": { "start_ms": 0 }
        })],
    );
    let mut h = TestHarness::new(course);
    let backend = Arc::new(Mutex::new(FiniteMediaBackend::new(vec![("vid1", 3000)])));
    h.editor
        .set_timeline_playback(TimelinePlayback::with_backend(Box::new(SharedBackend(
            Arc::clone(&backend),
        ))));
    h.frame_at_time(1.0);
    h.frame_at_time(1.0);
    backend.lock().expect("lock").reset_play_counts();

    click_timeline_play(&mut h);
    let t = step_playback(&mut h, &backend, 1.0, 12, 0.5);

    assert!(!h.editor.timeline_playback.is_playing());
    assert_eq!(h.editor.ui.timeline.playhead_ms, 5000);
    assert_eq!(
        backend.lock().expect("lock").play_count("vid1"),
        1,
        "ended video must not be re-played while the playhead continues from 3 s to 5 s"
    );
    // Video holds its last frame at 3000 ms while playhead is at 5000 ms.
    let held = h
        .editor
        .timeline_playback
        .video_frame_for("vid1", 4000)
        .expect("last frame held after video ends");
    assert_eq!(held.pts_ms, 3000);

    // Replay from the end restarts at 0 and plays the video again.
    backend.lock().expect("lock").reset_play_counts();
    click_timeline_play(&mut h);
    let _ = step_playback(&mut h, &backend, t, 12, 0.5);
    assert!(!h.editor.timeline_playback.is_playing());
    assert_eq!(h.editor.ui.timeline.playhead_ms, 5000);
    assert_eq!(
        backend.lock().expect("lock").play_count("vid1"),
        1,
        "replaying the slide from the end must play the video once from 0"
    );
}

#[test]
fn f23_longer_video_stops_at_slide_duration() {
    let course = course_with_media(
        5000,
        vec![serde_json::json!({
            "id": "vid1",
            "name": "Long Video",
            "kind": { "type": "video", "src": "long.mp4", "autoplay": true },
            "frame": { "x": 100.0, "y": 100.0, "w": 400.0, "h": 225.0 },
            "timing": { "start_ms": 0 }
        })],
    );
    let mut h = TestHarness::new(course);
    let backend = Arc::new(Mutex::new(FiniteMediaBackend::new(vec![("vid1", 8000)])));
    h.editor
        .set_timeline_playback(TimelinePlayback::with_backend(Box::new(SharedBackend(
            Arc::clone(&backend),
        ))));
    h.frame_at_time(1.0);
    h.frame_at_time(1.0);

    click_timeline_play(&mut h);
    let _ = step_playback(&mut h, &backend, 1.0, 12, 0.5);

    assert!(!h.editor.timeline_playback.is_playing());
    assert_eq!(h.editor.ui.timeline.playhead_ms, 5000);
}

#[test]
fn f23_audio_ending_early_and_two_videos_reach_slide_end() {
    let course = course_with_media(
        5000,
        vec![
            serde_json::json!({
                "id": "aud1",
                "name": "Short Audio",
                "kind": { "type": "audio", "src": "note.wav", "autoplay": true },
                "frame": { "x": 40.0, "y": 40.0, "w": 48.0, "h": 48.0 },
                "timing": { "start_ms": 0 }
            }),
            serde_json::json!({
                "id": "vid1",
                "name": "Video 1",
                "kind": { "type": "video", "src": "v1.mp4", "autoplay": true },
                "frame": { "x": 100.0, "y": 100.0, "w": 200.0, "h": 150.0 },
                "timing": { "start_ms": 0 }
            }),
            serde_json::json!({
                "id": "vid2",
                "name": "Video 2",
                "kind": { "type": "video", "src": "v2.mp4", "autoplay": true },
                "frame": { "x": 320.0, "y": 100.0, "w": 200.0, "h": 150.0 },
                "timing": { "start_ms": 500 }
            }),
        ],
    );
    let mut h = TestHarness::new(course);
    let backend = Arc::new(Mutex::new(FiniteMediaBackend::new(vec![
        ("aud1", 1500),
        ("vid1", 2000),
        ("vid2", 3500),
    ])));
    h.editor
        .set_timeline_playback(TimelinePlayback::with_backend(Box::new(SharedBackend(
            Arc::clone(&backend),
        ))));
    h.frame_at_time(1.0);
    h.frame_at_time(1.0);
    backend.lock().expect("lock").reset_play_counts();

    click_timeline_play(&mut h);
    let _ = step_playback(&mut h, &backend, 1.0, 12, 0.5);

    assert!(!h.editor.timeline_playback.is_playing());
    assert_eq!(h.editor.ui.timeline.playhead_ms, 5000);
    let guard = backend.lock().expect("lock");
    assert_eq!(guard.play_count("aud1"), 1);
    assert_eq!(guard.play_count("vid1"), 1);
    assert_eq!(guard.play_count("vid2"), 1);
}

#[test]
fn f23_seeking_past_video_end_and_back_works() {
    let course = course_with_media(
        5000,
        vec![serde_json::json!({
            "id": "vid1",
            "name": "Short Video",
            "kind": { "type": "video", "src": "short.mp4", "autoplay": true },
            "frame": { "x": 100.0, "y": 100.0, "w": 400.0, "h": 225.0 },
            "timing": { "start_ms": 0 }
        })],
    );
    let mut h = TestHarness::new(course);
    let backend = Arc::new(Mutex::new(FiniteMediaBackend::new(vec![("vid1", 3000)])));
    h.editor
        .set_timeline_playback(TimelinePlayback::with_backend(Box::new(SharedBackend(
            Arc::clone(&backend),
        ))));
    h.frame_at_time(1.0);
    h.frame_at_time(1.0);

    // Seek past the 3 s video end to 4000 ms while paused.
    h.editor.ui.timeline.playhead_ms = 4000;
    h.editor.timeline_playback.seek_to(4000, 5000, 1000);
    h.frame_at_time(1.1);
    let held = h
        .editor
        .timeline_playback
        .video_frame_for("vid1", 4000)
        .expect("last frame after seeking past video end");
    assert_eq!(held.pts_ms, 3000);

    // Seek back before the video end (1500 ms) and press Play: video plays to 3000, slide plays to 5000.
    h.editor.ui.timeline.playhead_ms = 1500;
    h.editor.timeline_playback.seek_to(1500, 5000, 1100);
    h.frame_at_time(1.2);
    backend.lock().expect("lock").reset_play_counts();

    click_timeline_play(&mut h);
    let _ = step_playback(&mut h, &backend, 1.2, 10, 0.5);
    assert!(!h.editor.timeline_playback.is_playing());
    assert_eq!(h.editor.ui.timeline.playhead_ms, 5000);
    assert_eq!(backend.lock().expect("lock").play_count("vid1"), 1);
}

fn many_objects_course(count: usize) -> Course {
    let mut objects = Vec::new();
    for i in 0..count {
        objects.push(serde_json::json!({
            "id": format!("obj{i}"),
            "name": format!("Object {i}"),
            "kind": { "type": "text", "text": format!("Item {i}") },
            "frame": { "x": 50.0, "y": 50.0 + (i as f64) * 20.0, "w": 200.0, "h": 40.0 },
            "timing": { "start_ms": 0 }
        }));
    }
    course_with_media(5000, objects)
}

#[test]
fn f23_end_handle_at_slide_duration_is_draggable_scrollable_and_not_scrollable() {
    for count in [16usize, 2usize] {
        let mut h = TestHarness::new(many_objects_course(count));
        h.idle_frame();
        h.idle_frame();

        let right_h = *h
            .editor
            .ui
            .timeline
            .object_right_handle_rects
            .get("obj0")
            .expect("obj0 right handle rect");

        // Hover the right edge of the handle first so a floating scrollbar would expand if present.
        let handle_grab = pos2(right_h.max.x - 1.0, right_h.center().y);
        h.hover_at(handle_grab);
        h.idle_frame();
        h.idle_frame();

        h.drag_by(handle_grab, vec2(-120.0, 0.0), 6, Modifiers::NONE);
        h.idle_frame();

        let slide = h.editor.session.course.slide("s1").expect("s1");
        let obj0 = slide.object("obj0").expect("obj0");
        assert!(
            obj0.0.timing.end_ms.is_some_and(|e| e < 5000),
            "count={count}: dragging the end handle at slide duration must change end_ms, got {:?}",
            obj0.0.timing.end_ms
        );
    }
}
