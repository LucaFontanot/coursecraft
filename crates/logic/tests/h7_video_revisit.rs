use coursecraft_course::{Course, RevisitMode};
use coursecraft_logic::{Effect, Input, MediaOp, Player};

fn video_course(extra_triggers: &str) -> Course {
    let json = format!(
        r#"{{
          "schema_version": 1,
          "id": "video-click",
          "title": "Video Click",
          "variables": [
            {{ "name": "cues_hit", "value": 0 }},
            {{ "name": "author_clicks", "value": 0 }}
          ],
          "scenes": [{{
            "id": "sc1",
            "title": "Scene 1",
            "slides": [{{
              "id": "s1",
              "title": "Slide 1",
              "duration_ms": 10000,
              "objects": [
                {{
                  "id": "vid",
                  "kind": {{
                    "type": "video",
                    "src": "clip.mp4",
                    "cues": [{{ "id": "c1", "at_ms": 500, "name": "Cue 1" }}]
                  }},
                  "frame": {{ "x": 40, "y": 40, "w": 400, "h": 225 }}
                }}
              ],
              "triggers": [
                {{
                  "when": {{ "event": "cue_reached", "object": "vid", "cue": "c1" }},
                  "action": {{ "action": "set_variable", "variable": "cues_hit", "op": "add", "value": 1 }}
                }}{extra_triggers}
              ]
            }}]
          }}]
        }}"#
    );
    Course::from_json(&json).expect("valid course")
}

#[test]
fn click_on_video_plays_pauses_and_fires_cues() {
    let course = video_course("");
    let mut player = Player::new(&course).expect("player");
    let _ = player.handle(&Input::Start);

    assert!(
        player.keyboard_objects().contains(&"vid".to_owned()),
        "video must be keyboard focusable"
    );
    assert!(!player.is_media_playing("vid"));

    // First click starts playback.
    let fx = player.handle(&Input::Click {
        object: "vid".to_owned(),
    });
    assert!(
        fx.contains(&Effect::Media {
            object: "vid".to_owned(),
            op: MediaOp::Play,
        }),
        "first click must emit Media Play: {fx:?}"
    );
    assert!(player.is_media_playing("vid"));

    // Advancing media time fires cue triggers.
    let cue_fx = player.handle(&Input::MediaTime {
        object: "vid".to_owned(),
        at_ms: 600.0,
    });
    assert!(
        cue_fx.contains(&Effect::CueReached {
            object: "vid".to_owned(),
            cue: "c1".to_owned(),
        }),
        "cue must fire after click start: {cue_fx:?}"
    );

    // Second click pauses playback.
    let fx2 = player.handle(&Input::Click {
        object: "vid".to_owned(),
    });
    assert!(
        fx2.contains(&Effect::Media {
            object: "vid".to_owned(),
            op: MediaOp::Pause,
        }),
        "second click must emit Media Pause: {fx2:?}"
    );
    assert!(!player.is_media_playing("vid"));

    // Third click resumes playback.
    let fx3 = player.handle(&Input::Click {
        object: "vid".to_owned(),
    });
    assert!(fx3.contains(&Effect::Media {
        object: "vid".to_owned(),
        op: MediaOp::Play,
    }));
    assert!(player.is_media_playing("vid"));
}

#[test]
fn author_click_trigger_on_video_wins_without_double_toggle() {
    let course = video_course(
        r#",
        {
          "when": { "event": "click", "object": "vid" },
          "action": { "action": "play_media", "object": "vid" }
        },
        {
          "when": { "event": "click", "object": "vid" },
          "action": { "action": "set_variable", "variable": "author_clicks", "op": "add", "value": 1 }
        }"#,
    );
    let mut player = Player::new(&course).expect("player");
    let _ = player.handle(&Input::Start);

    let fx = player.handle(&Input::Click {
        object: "vid".to_owned(),
    });
    let play_count = fx
        .iter()
        .filter(|e| matches!(e, Effect::Media { object, op: MediaOp::Play } if object == "vid"))
        .count();
    let pause_count = fx
        .iter()
        .filter(|e| matches!(e, Effect::Media { object, op: MediaOp::Pause } if object == "vid"))
        .count();
    assert_eq!(play_count, 1, "expected exactly one Play: {fx:?}");
    assert_eq!(pause_count, 0, "must not double-toggle to Pause: {fx:?}");
    assert!(player.is_media_playing("vid"));
}

fn run_first_visit_to_end(player: &mut Player<'_>) {
    let _ = player.handle(&Input::Start);
    let _ = player.handle(&Input::GoTo {
        slide: "video".to_owned(),
    });
    let _ = player.handle(&Input::Click {
        object: "v_video".to_owned(),
    });
    let _ = player.handle(&Input::MediaTime {
        object: "v_video".to_owned(),
        at_ms: 400.0,
    });
    assert_eq!(player.object_state("v_row_neck"), Some("active"));
    let _ = player.handle(&Input::MediaTime {
        object: "v_video".to_owned(),
        at_ms: 4800.0,
    });
    assert_eq!(player.object_state("v_row_neck"), Some("done"));
    assert_eq!(player.object_state("v_row_shoulder"), Some("active"));
    let _ = player.handle(&Input::MediaTime {
        object: "v_video".to_owned(),
        at_ms: 9200.0,
    });
    assert_eq!(player.object_state("v_row_shoulder"), Some("done"));
    assert_eq!(player.object_state("v_row_twist"), Some("active"));
    let _ = player.handle(&Input::MediaEnded {
        object: "v_video".to_owned(),
    });
    assert_eq!(player.object_state("v_row_twist"), Some("done"));

    let _ = player.handle(&Input::Next);
    let _ = player.handle(&Input::GoTo {
        slide: "video".to_owned(),
    });
}

#[test]
fn healthy_habits_video_slide_revisit_reset_keeps_cues_in_order_and_auto_shows_trap() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/healthy-habits/course.json"
    );
    let json = std::fs::read_to_string(path).expect("read healthy-habits");
    let course = Course::from_json(&json).expect("parse healthy-habits");
    let video_slide = course.slide("video").expect("video slide");
    assert_eq!(
        video_slide.revisit,
        RevisitMode::Reset,
        "healthy-habits video slide must use revisit: reset"
    );

    let mut reset_player = Player::new(&course).expect("player");
    run_first_visit_to_end(&mut reset_player);
    assert_eq!(reset_player.object_state("v_row_neck"), Some("normal"));
    assert_eq!(reset_player.object_state("v_row_shoulder"), Some("normal"));
    assert_eq!(reset_player.object_state("v_row_twist"), Some("normal"));
    assert_eq!(reset_player.object_state("v_num_neck"), Some("normal"));
    assert_eq!(reset_player.object_state("v_num_shoulder"), Some("normal"));
    assert_eq!(reset_player.object_state("v_num_twist"), Some("normal"));

    let _ = reset_player.handle(&Input::Click {
        object: "v_video".to_owned(),
    });
    let _ = reset_player.handle(&Input::MediaTime {
        object: "v_video".to_owned(),
        at_ms: 400.0,
    });
    assert_eq!(reset_player.object_state("v_row_neck"), Some("active"));
    assert_eq!(reset_player.object_state("v_row_shoulder"), Some("normal"));
    assert_eq!(reset_player.object_state("v_row_twist"), Some("normal"));

    let _ = reset_player.handle(&Input::MediaTime {
        object: "v_video".to_owned(),
        at_ms: 4800.0,
    });
    assert_eq!(reset_player.object_state("v_row_neck"), Some("done"));
    assert_eq!(reset_player.object_state("v_row_shoulder"), Some("active"));
    assert_eq!(reset_player.object_state("v_row_twist"), Some("normal"));

    let _ = reset_player.handle(&Input::MediaTime {
        object: "v_video".to_owned(),
        at_ms: 9200.0,
    });
    assert_eq!(reset_player.object_state("v_row_shoulder"), Some("done"));
    assert_eq!(reset_player.object_state("v_row_twist"), Some("active"));

    let _ = reset_player.handle(&Input::MediaEnded {
        object: "v_video".to_owned(),
    });
    assert_eq!(reset_player.object_state("v_row_neck"), Some("done"));
    assert_eq!(reset_player.object_state("v_row_shoulder"), Some("done"));
    assert_eq!(reset_player.object_state("v_row_twist"), Some("done"));

    let mut auto_course = course;
    if let Some(s) = auto_course.slide_mut("video") {
        s.revisit = RevisitMode::Auto;
    }
    let mut auto_player = Player::new(&auto_course).expect("player");
    run_first_visit_to_end(&mut auto_player);
    // Trap with revisit: auto — rows remain stale "done" on re-entry.
    assert_eq!(auto_player.object_state("v_row_neck"), Some("done"));
    assert_eq!(auto_player.object_state("v_row_shoulder"), Some("done"));
    assert_eq!(auto_player.object_state("v_row_twist"), Some("done"));
}
