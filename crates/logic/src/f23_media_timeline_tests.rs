//! Regression tests for f23: slide duration governs timers, timeline_ends,
//! and auto-advance even when a video or audio object ends earlier (or loops).

use crate::{Effect, Input, MediaOp, Player};
use coursecraft_course::Course;

fn two_slide_video_course(loop_video: bool) -> Course {
    let mut triggers = vec![serde_json::json!({
        "when": { "event": "timeline_ends" },
        "action": { "action": "jump_to_slide", "target": { "id": "s2" } }
    })];
    if loop_video {
        triggers.push(serde_json::json!({
            "when": { "event": "media_ends", "object": "vid1" },
            "action": { "action": "play_media", "object": "vid1" }
        }));
    }
    let json = serde_json::json!({
        "schema_version": 1,
        "id": "f23-logic",
        "title": "F23 Logic",
        "scenes": [{
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
                            "kind": { "type": "video", "src": "short.mp4", "autoplay": true },
                            "frame": { "x": 0.0, "y": 0.0, "w": 320.0, "h": 180.0 },
                            "timing": { "start_ms": 0 }
                        },
                        {
                            "id": "tm1",
                            "kind": { "type": "timer", "duration_ms": 5000 },
                            "frame": { "x": 0.0, "y": 200.0, "w": 120.0, "h": 40.0 },
                            "timing": { "start_ms": 0 }
                        }
                    ],
                    "triggers": triggers
                },
                {
                    "id": "s2",
                    "title": "Slide 2",
                    "duration_ms": 5000,
                    "objects": []
                }
            ]
        }]
    })
    .to_string();
    Course::from_json(&json).expect("valid course")
}

#[test]
fn slide_continues_to_slide_duration_after_shorter_video_ends() {
    let course = two_slide_video_course(false);
    let mut player = Player::new(&course).expect("valid player");
    let _ = player.handle(&Input::Start);

    // Advance 3000 ms and report the 3 s video ended.
    let _ = player.handle(&Input::Tick { ms: 3000 });
    let fx_end = player.handle(&Input::MediaEnded {
        object: "vid1".to_owned(),
    });
    assert!(
        !fx_end
            .iter()
            .any(|e| matches!(e, Effect::SlideEntered { .. })),
        "media ending at 3 s must not advance a 5 s slide"
    );
    assert_eq!(player.current_slide().map(|s| s.id.as_str()), Some("s1"));
    assert_eq!(player.timer_remaining_ms("tm1"), Some(2000));

    // Advance the remaining 2000 ms to reach the 5 s slide duration: timeline_ends fires and auto-advances.
    let fx_tl = player.handle(&Input::Tick { ms: 2000 });
    assert!(
        fx_tl
            .iter()
            .any(|e| matches!(e, Effect::SlideEntered { slide } if slide == "s2")),
        "timeline_ends at 5 s must advance to s2: {fx_tl:?}"
    );
    assert_eq!(player.current_slide().map(|s| s.id.as_str()), Some("s2"));
}

#[test]
fn looping_video_restarts_on_media_ends_while_slide_still_ends_at_duration() {
    let course = two_slide_video_course(true);
    let mut player = Player::new(&course).expect("valid player");
    let _ = player.handle(&Input::Start);

    let _ = player.handle(&Input::Tick { ms: 3000 });
    let fx_loop = player.handle(&Input::MediaEnded {
        object: "vid1".to_owned(),
    });
    assert!(
        fx_loop.iter().any(|e| matches!(
            e,
            Effect::Media { object, op: MediaOp::Play } if object == "vid1"
        )),
        "looping trigger must emit Media Play when vid1 ends: {fx_loop:?}"
    );
    assert_eq!(player.current_slide().map(|s| s.id.as_str()), Some("s1"));

    // At 5000 ms, slide still ends and stops the looping video before entering s2.
    let fx_tl = player.handle(&Input::Tick { ms: 2000 });
    assert!(fx_tl.iter().any(|e| matches!(
        e,
        Effect::Media { object, op: MediaOp::Stop } if object == "vid1"
    )));
    assert!(
        fx_tl
            .iter()
            .any(|e| matches!(e, Effect::SlideEntered { slide } if slide == "s2"))
    );
}
