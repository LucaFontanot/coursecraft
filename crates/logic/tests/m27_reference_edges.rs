use coursecraft_course::{Course, NavMode, Severity, SlideTarget, Value};
use coursecraft_logic::{
    Effect, Input, MediaOp, Player, PlayerState, decode_suspend, encode_suspend,
};

fn build_course(mut json: serde_json::Value) -> Course {
    if let Some(obj) = json.as_object_mut()
        && let Some(slides) = obj.remove("slides")
        && !obj.contains_key("scenes")
    {
        obj.insert(
            "scenes".into(),
            serde_json::json!([{ "id": "sc1", "title": "Main", "slides": slides }]),
        );
    }
    let course: Course = serde_json::from_value(json).expect("valid course json");
    let errors: Vec<_> = course
        .validate()
        .into_iter()
        .filter(|i| i.severity == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "test course failed validation: {errors:#?}"
    );
    course
}

fn has_warning(effects: &[Effect]) -> bool {
    effects.iter().any(|e| matches!(e, Effect::Warning { .. }))
}

fn current_id<'a>(p: &'a Player<'a>) -> &'a str {
    p.current_slide().expect("current slide").id.as_str()
}

/// B5.1: Lightbox over video with cue points (BUG-B1 & BUG-B2).
#[test]
fn b5_1_lightbox_preserves_video_cues_and_jump_skips_spurious_play() {
    let course = build_course(serde_json::json!({
        "schema_version": 1,
        "id": "b5-1",
        "title": "B5.1 Lightbox Cues",
        "slides": [
            {
                "id": "s-video",
                "title": "Video",
                "duration_ms": 10000,
                "objects": [
                    {
                        "id": "vid",
                        "kind": {
                            "type": "video",
                            "src": "media/clip.mp4",
                            "autoplay": true,
                            "cues": [
                                { "id": "c1", "name": "Cue 1", "at_ms": 1000, "pause": false },
                                { "id": "c2", "name": "Cue 2", "at_ms": 2000, "pause": false }
                            ]
                        },
                        "frame": { "x": 0.0, "y": 0.0, "w": 640.0, "h": 360.0 }
                    },
                    {
                        "id": "btn-lb",
                        "kind": { "type": "button", "label": "Open Lightbox" },
                        "frame": { "x": 10.0, "y": 380.0, "w": 160.0, "h": 40.0 }
                    }
                ],
                "triggers": [
                    {
                        "when": { "event": "click", "object": "btn-lb" },
                        "action": { "action": "lightbox_slide", "slide": "s06c-lightbox" }
                    }
                ]
            },
            {
                "id": "s06c-lightbox",
                "title": "Lightbox",
                "duration_ms": 5000,
                "objects": [
                    {
                        "id": "btn-jump",
                        "kind": { "type": "button", "label": "Jump Out" },
                        "frame": { "x": 20.0, "y": 20.0, "w": 160.0, "h": 40.0 }
                    }
                ],
                "triggers": [
                    {
                        "when": { "event": "click", "object": "btn-jump" },
                        "action": { "action": "jump_to_slide", "target": { "id": "s-other" } }
                    }
                ]
            },
            {
                "id": "s-other",
                "title": "Other",
                "duration_ms": 5000,
                "objects": []
            }
        ]
    }));

    let mut p = Player::new(&course).expect("player");
    let fx = p.handle(&Input::Start);
    assert!(fx.contains(&Effect::Media {
        object: "vid".into(),
        op: MediaOp::Play,
    }));

    // Advance video past cue c1 (1000ms).
    let fx = p.handle(&Input::MediaTime {
        object: "vid".into(),
        at_ms: 1500.0,
    });
    assert!(fx.contains(&Effect::CueReached {
        object: "vid".into(),
        cue: "c1".into(),
    }));

    // Open lightbox -> pauses base video.
    let fx = p.handle(&Input::Click {
        object: "btn-lb".into(),
    });
    assert!(fx.contains(&Effect::Media {
        object: "vid".into(),
        op: MediaOp::Pause,
    }));
    assert!(fx.contains(&Effect::LightboxOpened {
        slide: "s06c-lightbox".into(),
    }));

    // Close lightbox -> resumes base video.
    let fx = p.handle(&Input::CloseLightbox);
    assert!(fx.contains(&Effect::LightboxClosed {
        slide: "s06c-lightbox".into(),
    }));
    assert!(fx.contains(&Effect::Media {
        object: "vid".into(),
        op: MediaOp::Play,
    }));

    // BUG-B1 check: c1 must stay fired and NOT re-fire on resume; c2 fires at 2100ms.
    let fx = p.handle(&Input::MediaTime {
        object: "vid".into(),
        at_ms: 1600.0,
    });
    assert!(
        !fx.iter()
            .any(|e| matches!(e, Effect::CueReached { cue, .. } if cue == "c1")),
        "c1 must not re-fire after closing lightbox: {fx:?}"
    );
    let fx = p.handle(&Input::MediaTime {
        object: "vid".into(),
        at_ms: 2100.0,
    });
    assert!(fx.contains(&Effect::CueReached {
        object: "vid".into(),
        cue: "c2".into(),
    }));

    // BUG-B2 check: open lightbox again and trigger JumpToSlide -> NO spurious Play before Stop.
    p.handle(&Input::Click {
        object: "btn-lb".into(),
    });
    let fx = p.handle(&Input::Click {
        object: "btn-jump".into(),
    });
    assert!(fx.contains(&Effect::SlideEntered {
        slide: "s-other".into(),
    }));
    assert!(
        !fx.contains(&Effect::Media {
            object: "vid".into(),
            op: MediaOp::Play,
        }),
        "JumpToSlide from lightbox must not emit spurious Media::Play for base video: {fx:?}"
    );
    assert!(fx.contains(&Effect::Media {
        object: "vid".into(),
        op: MediaOp::Stop,
    }));
}

/// B5.2: NavMode::Restricted & Locked + NextRequirement::TimelineEnds & Variable.
#[test]
fn b5_2_nav_modes_and_next_requirements() {
    let mut course = build_course(serde_json::json!({
        "schema_version": 1,
        "id": "b5-2",
        "title": "B5.2 Nav & Next",
        "variables": [
            { "name": "v_ready", "value": false }
        ],
        "player": {
            "navigation": "restricted",
            "show_menu": true
        },
        "slides": [
            {
                "id": "s01-intro",
                "title": "Intro",
                "duration_ms": 2000,
                "player": {
                    "next_requires": { "kind": "timeline_ends" }
                },
                "objects": [
                    {
                        "id": "btn-author-jump",
                        "kind": { "type": "button", "label": "Jump" },
                        "frame": { "x": 10.0, "y": 10.0, "w": 120.0, "h": 40.0 }
                    }
                ],
                "triggers": [
                    {
                        "when": { "event": "click", "object": "btn-author-jump" },
                        "action": { "action": "jump_to_slide", "target": { "id": "s03-end" } }
                    }
                ]
            },
            {
                "id": "s02-layers",
                "title": "Layers",
                "duration_ms": 2000,
                "player": {
                    "next_requires": { "kind": "variable", "name": "v_ready" }
                },
                "objects": []
            },
            {
                "id": "s03-end",
                "title": "End",
                "duration_ms": 2000,
                "objects": []
            }
        ]
    }));

    let mut p = Player::new(&course).expect("player");
    p.handle(&Input::Start);

    // GoTo to unvisited s03-end warns in Restricted.
    let fx = p.handle(&Input::GoTo {
        slide: "s03-end".into(),
    });
    assert!(has_warning(&fx));
    assert_eq!(current_id(&p), "s01-intro");

    // Next blocks until duration_ms (2000ms) on s01-intro.
    let fx = p.handle(&Input::Next);
    assert!(has_warning(&fx));
    assert_eq!(current_id(&p), "s01-intro");

    p.handle(&Input::Tick { ms: 2000 });
    let fx = p.handle(&Input::Next);
    assert!(!has_warning(&fx));
    assert_eq!(current_id(&p), "s02-layers");

    // GoTo to visited s01-intro works in Restricted.
    let fx = p.handle(&Input::GoTo {
        slide: "s01-intro".into(),
    });
    assert!(!has_warning(&fx));
    assert_eq!(current_id(&p), "s01-intro");

    // Revisit of s01-intro keeps Next unlocked immediately.
    let fx = p.handle(&Input::Next);
    assert!(!has_warning(&fx));
    assert_eq!(current_id(&p), "s02-layers");

    // On s02-layers, Next blocks until v_ready == true.
    let fx = p.handle(&Input::Next);
    assert!(has_warning(&fx));
    assert_eq!(current_id(&p), "s02-layers");

    p.handle(&Input::SetVariable {
        variable: "v_ready".into(),
        value: Value::Bool(true),
    });
    let fx = p.handle(&Input::Next);
    assert!(!has_warning(&fx));
    assert_eq!(current_id(&p), "s03-end");

    // Locked mode: GoTo to any slide warns (even visited), while author JumpToSlide succeeds.
    course.player.navigation = NavMode::Locked;
    let mut p_locked = Player::new(&course).expect("player");
    p_locked.handle(&Input::Start);
    let fx = p_locked.handle(&Input::GoTo {
        slide: "s01-intro".into(),
    });
    assert!(has_warning(&fx));

    let fx = p_locked.handle(&Input::Click {
        object: "btn-author-jump".into(),
    });
    assert!(!has_warning(&fx));
    assert_eq!(current_id(&p_locked), "s03-end");
}

/// B5.3: Playback speed (0.5x, 1.5x, 2.0x) + quarter-ms carry + timer + Lottie + pause/seek.
#[test]
fn b5_3_playback_speed_carry_pause_and_seek() {
    let course = build_course(serde_json::json!({
        "schema_version": 1,
        "id": "b5-3",
        "title": "B5.3 Speed & Seek",
        "variables": [
            { "name": "v_cue_fired", "value": 0.0 }
        ],
        "player": {
            "show_speed": true
        },
        "slides": [
            {
                "id": "s1",
                "title": "Slide 1",
                "duration_ms": 10000,
                "objects": [
                    {
                        "id": "timer1",
                        "kind": { "type": "timer", "duration_ms": 5000, "autostart": true },
                        "frame": { "x": 0.0, "y": 0.0, "w": 100.0, "h": 40.0 }
                    },
                    {
                        "id": "lot1",
                        "kind": { "type": "lottie", "src": "media/anim.json", "autoplay": true },
                        "frame": { "x": 0.0, "y": 50.0, "w": 100.0, "h": 100.0 }
                    },
                    {
                        "id": "vid1",
                        "kind": {
                            "type": "video",
                            "src": "media/clip.mp4",
                            "autoplay": true,
                            "cues": [
                                { "id": "c1", "name": "Cue 1", "at_ms": 1000, "pause": false },
                                { "id": "c2", "name": "Cue 2", "at_ms": 2500, "pause": false }
                            ]
                        },
                        "frame": { "x": 120.0, "y": 0.0, "w": 320.0, "h": 180.0 }
                    }
                ],
                "triggers": [
                    {
                        "when": { "event": "timeline_reaches", "ms": 2000 },
                        "action": { "action": "set_variable", "variable": "v_cue_fired", "op": "add", "value": 1.0 }
                    }
                ]
            }
        ]
    }));

    let mut p = Player::new(&course).expect("player");
    p.handle(&Input::Start);

    // 1.5x speed: 1000 x 1ms ticks -> exact 1500ms advance on base clock, timer, and Lottie.
    p.set_speed(1.5);
    for _ in 0..1000 {
        p.handle(&Input::Tick { ms: 1 });
    }
    assert_eq!(p.state().base.ms, 1500);
    assert_eq!(p.timer_remaining_ms("timer1"), Some(3500));
    assert_eq!(p.state().media_clocks.get("lot1").copied(), Some(1500));

    // 0.5x speed: 100 x 1ms ticks -> +50ms.
    p.set_speed(0.5);
    for _ in 0..100 {
        p.handle(&Input::Tick { ms: 1 });
    }
    assert_eq!(p.state().base.ms, 1550);

    // 2.0x speed: 100 x 1ms ticks -> +200ms.
    p.set_speed(2.0);
    for _ in 0..100 {
        p.handle(&Input::Tick { ms: 1 });
    }
    assert_eq!(p.state().base.ms, 1750);

    // PlayerPause freezes all clocks.
    p.handle(&Input::PlayerPause);
    p.handle(&Input::Tick { ms: 500 });
    assert_eq!(p.state().base.ms, 1750);
    assert_eq!(p.timer_remaining_ms("timer1"), Some(3250));
    assert_eq!(p.state().media_clocks.get("lot1").copied(), Some(1750));

    // Seek forward past t=2000ms fires timeline_reaches cue.
    p.handle(&Input::PlayerResume);
    // First test video MediaTime forward (c1, c2) & backward re-arming without undoing vars.
    let fx = p.handle(&Input::MediaTime {
        object: "vid1".into(),
        at_ms: 2600.0,
    });
    let cue_ids: Vec<&str> = fx
        .iter()
        .filter_map(|e| match e {
            Effect::CueReached { cue, .. } => Some(cue.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(cue_ids, vec!["c1", "c2"]);

    p.handle(&Input::PlayerSeek { ms: 3000 });
    assert_eq!(p.variable("v_cue_fired"), Some(&Value::Number(1.0)));

    // Backward MediaTime re-arms c2 (at 2500ms) while keeping v_cue_fired == 1.0.
    p.handle(&Input::MediaTime {
        object: "vid1".into(),
        at_ms: 1500.0,
    });
    assert_eq!(p.variable("v_cue_fired"), Some(&Value::Number(1.0)));
    let fx = p.handle(&Input::MediaTime {
        object: "vid1".into(),
        at_ms: 2600.0,
    });
    assert!(fx.contains(&Effect::CueReached {
        object: "vid1".into(),
        cue: "c2".into(),
    }));
}

/// B5.4: Layer allow_seek (Auto/Yes/No) & revisit (Auto/Reset/Resume) + hide_when_timeline_ends.
#[test]
fn b5_4_layer_allow_seek_revisit_and_auto_hide() {
    let course = build_course(serde_json::json!({
        "schema_version": 1,
        "id": "b5-4",
        "title": "B5.4 Layer Seek & Revisit",
        "variables": [
            { "name": "v_ended", "value": false }
        ],
        "slides": [
            {
                "id": "s1",
                "title": "Slide 1",
                "duration_ms": 10000,
                "objects": [
                    {
                        "id": "vid",
                        "kind": { "type": "video", "src": "media/clip.mp4", "autoplay": true },
                        "frame": { "x": 0.0, "y": 0.0, "w": 320.0, "h": 180.0 }
                    },
                    {
                        "id": "btn-no",
                        "kind": { "type": "button", "label": "NoSeek" },
                        "frame": { "x": 0.0, "y": 200.0, "w": 100.0, "h": 40.0 }
                    },
                    {
                        "id": "btn-hide-no",
                        "kind": { "type": "button", "label": "HideNo" },
                        "frame": { "x": 110.0, "y": 200.0, "w": 100.0, "h": 40.0 }
                    },
                    {
                        "id": "btn-yes",
                        "kind": { "type": "button", "label": "YesSeek" },
                        "frame": { "x": 220.0, "y": 200.0, "w": 100.0, "h": 40.0 }
                    }
                ],
                "layers": [
                    {
                        "id": "l_no",
                        "name": "No Seek",
                        "duration_ms": 2000,
                        "allow_seek": "no",
                        "revisit": "resume",
                        "pause_base_timeline": true,
                        "objects": []
                    },
                    {
                        "id": "l_yes",
                        "name": "Yes Seek",
                        "duration_ms": 2000,
                        "allow_seek": "yes",
                        "revisit": "resume",
                        "hide_when_timeline_ends": true,
                        "pause_base_timeline": true,
                        "objects": []
                    }
                ],
                "triggers": [
                    {
                        "when": { "event": "click", "object": "btn-no" },
                        "action": { "action": "show_layer", "layer": "l_no" }
                    },
                    {
                        "when": { "event": "click", "object": "btn-hide-no" },
                        "action": { "action": "hide_layer", "layer": "l_no" }
                    },
                    {
                        "when": { "event": "click", "object": "btn-yes" },
                        "action": { "action": "show_layer", "layer": "l_yes" }
                    },
                    {
                        "when": { "event": "timeline_ends", "layer": "l_yes" },
                        "action": { "action": "set_variable", "variable": "v_ended", "op": "assign", "value": true }
                    }
                ]
            }
        ]
    }));

    let mut p = Player::new(&course).expect("player");
    p.handle(&Input::Start);

    // Show l_no -> allow_seek: No refuses seek with warning.
    p.handle(&Input::Click {
        object: "btn-no".into(),
    });
    let fx = p.handle(&Input::PlayerSeek { ms: 500 });
    assert!(has_warning(&fx));
    assert_eq!(p.state().layer_clocks.get("l_no").map(|c| c.ms), Some(0));

    // Tick l_no 700ms, hide, re-show -> revisit: Resume restores 700ms.
    p.handle(&Input::Tick { ms: 700 });
    assert_eq!(p.state().layer_clocks.get("l_no").map(|c| c.ms), Some(700));
    p.handle(&Input::Click {
        object: "btn-hide-no".into(),
    });
    p.handle(&Input::Click {
        object: "btn-no".into(),
    });
    assert_eq!(p.state().layer_clocks.get("l_no").map(|c| c.ms), Some(700));
    p.handle(&Input::Click {
        object: "btn-hide-no".into(),
    });

    // Show l_yes -> allow_seek: Yes seeks layer clock.
    p.handle(&Input::Click {
        object: "btn-yes".into(),
    });
    let fx = p.handle(&Input::PlayerSeek { ms: 500 });
    assert!(!has_warning(&fx));
    assert_eq!(p.state().layer_clocks.get("l_yes").map(|c| c.ms), Some(500));

    // Tick to end (1500ms) -> fires timeline_ends, hides layer, resumes base media.
    let fx = p.handle(&Input::Tick { ms: 1500 });
    assert_eq!(p.variable("v_ended"), Some(&Value::Bool(true)));
    assert!(!p.state().visible_layers.contains(&"l_yes".to_owned()));
    assert!(fx.contains(&Effect::Media {
        object: "vid".into(),
        op: MediaOp::Play,
    }));

    // Re-show auto-hiding ended layer -> restarts at 0 despite revisit: Resume.
    p.handle(&Input::Click {
        object: "btn-yes".into(),
    });
    assert_eq!(p.state().layer_clocks.get("l_yes").map(|c| c.ms), Some(0));
}

/// B5.5: revisit: Reset vs Resume on slides + SlideTarget::LastVisited history cap (50).
#[test]
fn b5_5_slide_revisit_and_visit_history_cap() {
    let course = build_course(serde_json::json!({
        "schema_version": 1,
        "id": "b5-5",
        "title": "B5.5 Revisit & History",
        "slides": [
            {
                "id": "s01",
                "title": "S1",
                "duration_ms": 5000,
                "objects": [
                    {
                        "id": "btn-last",
                        "kind": { "type": "button", "label": "Back" },
                        "frame": { "x": 0.0, "y": 0.0, "w": 100.0, "h": 40.0 }
                    }
                ],
                "triggers": [
                    {
                        "when": { "event": "click", "object": "btn-last" },
                        "action": { "action": "jump_to_slide", "target": "last_visited" }
                    }
                ]
            },
            {
                "id": "s02-layers",
                "title": "S2 Resume",
                "duration_ms": 5000,
                "revisit": "resume",
                "objects": [
                    {
                        "id": "btn-open",
                        "kind": { "type": "button", "label": "Open" },
                        "frame": { "x": 0.0, "y": 0.0, "w": 100.0, "h": 40.0 }
                    }
                ],
                "layers": [
                    { "id": "l1", "name": "Layer 1", "duration_ms": 3000, "objects": [] }
                ],
                "triggers": [
                    {
                        "when": { "event": "click", "object": "btn-open" },
                        "action": { "action": "show_layer", "layer": "l1" }
                    }
                ]
            },
            {
                "id": "s03",
                "title": "S3 Reset",
                "duration_ms": 5000,
                "revisit": "reset",
                "objects": [
                    {
                        "id": "card",
                        "kind": { "type": "shape", "shape": "rectangle", "fill": "#336699" },
                        "frame": { "x": 0.0, "y": 0.0, "w": 100.0, "h": 100.0 },
                        "initial_state": "normal",
                        "states": [
                            { "name": "visited", "text": "Visited" }
                        ]
                    }
                ],
                "triggers": [
                    {
                        "when": { "event": "click", "object": "card" },
                        "action": { "action": "change_state", "object": "card", "state": "visited" }
                    }
                ]
            }
        ]
    }));

    let mut p = Player::new(&course).expect("player");
    p.handle(&Input::Start);

    // Visit s02-layers, open l1, tick 600ms, leave to s03.
    p.handle(&Input::Next);
    assert_eq!(current_id(&p), "s02-layers");
    p.handle(&Input::Click {
        object: "btn-open".into(),
    });
    p.handle(&Input::Tick { ms: 600 });
    p.handle(&Input::Next);
    assert_eq!(current_id(&p), "s03");

    // Change card state on s03.
    p.handle(&Input::Click {
        object: "card".into(),
    });
    assert_eq!(p.object_state("card"), Some("visited"));

    // Return to s02-layers -> clock & visible layer restored.
    p.handle(&Input::Previous);
    assert_eq!(current_id(&p), "s02-layers");
    assert_eq!(p.state().base.ms, 600);
    assert!(p.state().visible_layers.contains(&"l1".to_owned()));

    // Return to s03 -> revisit: reset restores initial_state "normal".
    p.handle(&Input::Next);
    assert_eq!(current_id(&p), "s03");
    assert_eq!(p.object_state("card"), Some("normal"));

    // Push 60 jumps -> visit_history capped at 50; LastVisited pops without pushing current.
    for i in 0..60 {
        let target = if i % 2 == 0 { "s01" } else { "s03" };
        p.handle(&Input::GoTo {
            slide: target.into(),
        });
    }
    assert_eq!(p.state().visit_history.len(), 50);
    // Ensure we are on s01, then click btn-last (LastVisited) -> pops from 50 to 49.
    p.handle(&Input::GoTo {
        slide: "s01".into(),
    });
    let len_before = p.state().visit_history.len();
    p.handle(&Input::Click {
        object: "btn-last".into(),
    });
    assert_eq!(current_id(&p), "s03");
    assert_eq!(p.state().visit_history.len(), len_before - 1);
}

/// B5.6: Loop guard (1024 events) & divide-by-zero / NaN hostility.
#[test]
fn b5_6_loop_guard_and_hostile_math_inputs() {
    let course = build_course(serde_json::json!({
        "schema_version": 1,
        "id": "b5-6",
        "title": "B5.6 Loop & Hostility",
        "variables": [
            { "name": "v_a", "value": 0.0 },
            { "name": "v_b", "value": 0.0 },
            { "name": "v_num", "value": 42.0 },
            { "name": "v_zero", "value": 0.0 },
            { "name": "v_alive", "value": false }
        ],
        "slides": [
            {
                "id": "s1",
                "title": "Slide 1",
                "duration_ms": 5000,
                "objects": [
                    {
                        "id": "btn-loop",
                        "kind": { "type": "button", "label": "Loop" },
                        "frame": { "x": 0.0, "y": 0.0, "w": 100.0, "h": 40.0 }
                    },
                    {
                        "id": "btn-div0",
                        "kind": { "type": "button", "label": "Div0" },
                        "frame": { "x": 110.0, "y": 0.0, "w": 100.0, "h": 40.0 }
                    },
                    {
                        "id": "btn-alive",
                        "kind": { "type": "button", "label": "Alive" },
                        "frame": { "x": 220.0, "y": 0.0, "w": 100.0, "h": 40.0 }
                    }
                ],
                "triggers": [
                    {
                        "when": { "event": "click", "object": "btn-loop" },
                        "action": { "action": "set_variable", "variable": "v_a", "op": "add", "value": 1.0 }
                    },
                    {
                        "when": { "event": "variable_changes", "variable": "v_a" },
                        "action": { "action": "set_variable", "variable": "v_b", "op": "add", "value": 1.0 }
                    },
                    {
                        "when": { "event": "variable_changes", "variable": "v_b" },
                        "action": { "action": "set_variable", "variable": "v_a", "op": "add", "value": 1.0 }
                    },
                    {
                        "when": { "event": "click", "object": "btn-div0" },
                        "action": { "action": "set_variable", "variable": "v_num", "op": "divide", "value": { "variable": "v_zero" } }
                    },
                    {
                        "when": { "event": "click", "object": "btn-alive" },
                        "action": { "action": "set_variable", "variable": "v_alive", "op": "assign", "value": true }
                    }
                ]
            }
        ]
    }));

    let mut p = Player::new(&course).expect("player");
    p.handle(&Input::Start);

    // Mutually recursive variable_changes loop halts at 1024 events with 1 Warning.
    let fx = p.handle(&Input::Click {
        object: "btn-loop".into(),
    });
    let warnings: Vec<_> = fx
        .iter()
        .filter(|e| matches!(e, Effect::Warning { .. }))
        .collect();
    assert_eq!(warnings.len(), 1, "expected 1 loop-guard warning: {fx:?}");

    // Course stays alive afterwards.
    p.handle(&Input::Click {
        object: "btn-alive".into(),
    });
    assert_eq!(p.variable("v_alive"), Some(&Value::Bool(true)));

    // Divide by 0.0 warns and leaves v_num unchanged.
    let fx = p.handle(&Input::Click {
        object: "btn-div0".into(),
    });
    assert!(has_warning(&fx));
    assert_eq!(p.variable("v_num"), Some(&Value::Number(42.0)));

    // Hostile non-finite coordinates never panic or corrupt state.
    let _ = p.handle(&Input::HotspotClicked {
        x: f32::NAN,
        y: f32::INFINITY,
    });
    let _ = p.handle(&Input::ObjectDropped {
        object: "btn-alive".into(),
        x: f32::NAN,
        y: 0.0,
    });
    assert_eq!(p.variable("v_alive"), Some(&Value::Bool(true)));
}

/// B5.7: All 6 VarOps + var-to-var + %var% 64 KiB cap & split-run rule.
#[test]
fn b5_7_var_ops_and_display_text_cap() {
    let course = build_course(serde_json::json!({
        "schema_version": 1,
        "id": "b5-7",
        "title": "B5.7 VarOps & Text",
        "variables": [
            { "name": "v_x", "value": 10.0 },
            { "name": "v_y", "value": 5.0 },
            { "name": "v_flag", "value": false },
            { "name": "v_big", "value": "seed" }
        ],
        "slides": [
            {
                "id": "s1",
                "title": "Slide 1",
                "duration_ms": 5000,
                "objects": [
                    {
                        "id": "btn-ops",
                        "kind": { "type": "button", "label": "Ops" },
                        "frame": { "x": 0.0, "y": 0.0, "w": 100.0, "h": 40.0 }
                    }
                ],
                "triggers": [
                    {
                        "when": { "event": "click", "object": "btn-ops" },
                        "action": { "action": "set_variable", "variable": "v_x", "op": "assign", "value": { "variable": "v_y" } }
                    },
                    {
                        "when": { "event": "click", "object": "btn-ops" },
                        "action": { "action": "set_variable", "variable": "v_x", "op": "add", "value": { "variable": "v_y" } }
                    },
                    {
                        "when": { "event": "click", "object": "btn-ops" },
                        "action": { "action": "set_variable", "variable": "v_x", "op": "subtract", "value": 2.0 }
                    },
                    {
                        "when": { "event": "click", "object": "btn-ops" },
                        "action": { "action": "set_variable", "variable": "v_x", "op": "multiply", "value": { "variable": "v_y" } }
                    },
                    {
                        "when": { "event": "click", "object": "btn-ops" },
                        "action": { "action": "set_variable", "variable": "v_x", "op": "divide", "value": 4.0 }
                    },
                    {
                        "when": { "event": "click", "object": "btn-ops" },
                        "action": { "action": "set_variable", "variable": "v_flag", "op": "toggle" }
                    }
                ]
            }
        ]
    }));

    let mut p = Player::new(&course).expect("player");
    p.handle(&Input::Start);
    p.handle(&Input::Click {
        object: "btn-ops".into(),
    });
    // (((5 + 5) - 2) * 5) / 4 = 10.0
    assert_eq!(p.variable("v_x"), Some(&Value::Number(10.0)));
    assert_eq!(p.variable("v_flag"), Some(&Value::Bool(true)));

    // %% escape & unknown %nope% literal.
    assert_eq!(
        p.display_text("Score: %v_x%%% and %nope%"),
        "Score: 10% and %nope%"
    );

    // 64 KiB cap (65,536 bytes).
    let chunk = "A".repeat(20_000);
    p.handle(&Input::SetVariable {
        variable: "v_big".into(),
        value: Value::Text(chunk),
    });
    let expanded = p.display_text("%v_big%%v_big%%v_big%%v_big%");
    assert_eq!(expanded.len(), 65_536);
}

/// B5.8: Question bank determinism across seeds & post-resume ReviewQuiz contract.
#[test]
fn b5_8_question_bank_determinism_and_post_resume_review() {
    let bank_slide = |id: &str, title: &str| {
        serde_json::json!({
            "id": id,
            "title": title,
            "objects": [],
            "question": {
                "kind": "true_false",
                "prompt": format!("{title}?"),
                "choices": [
                    { "id": "true", "text": "True", "correct": true },
                    { "id": "false", "text": "False" }
                ],
                "points": 10
            },
            "triggers": [
                { "when": { "event": "submit" }, "action": { "action": "submit_question" } }
            ]
        })
    };

    let course = build_course(serde_json::json!({
        "schema_version": 1,
        "id": "b5-8",
        "title": "B5.8 Bank & Review",
        "player": { "show_menu": true },
        "quiz": { "passing_percent": 70 },
        "banks": [
            {
                "id": "bank-pool",
                "title": "Pool",
                "slides": [
                    bank_slide("bk-q1", "Q1"),
                    bank_slide("bk-q2", "Q2"),
                    bank_slide("bk-q3", "Q3"),
                    bank_slide("bk-q4", "Q4")
                ]
            }
        ],
        "slides": [
            {
                "id": "s-draw",
                "title": "Draw",
                "draw": { "bank": "bank-pool", "count": 2 },
                "objects": []
            },
            {
                "id": "s-results",
                "title": "Results",
                "objects": [
                    {
                        "id": "btn-review",
                        "kind": { "type": "button", "label": "Review" },
                        "frame": { "x": 0.0, "y": 0.0, "w": 100.0, "h": 40.0 }
                    }
                ],
                "triggers": [
                    {
                        "when": { "event": "timeline_starts" },
                        "action": { "action": "submit_results" }
                    },
                    {
                        "when": { "event": "click", "object": "btn-review" },
                        "action": { "action": "review_quiz" }
                    }
                ]
            }
        ]
    }));

    let p0 = Player::with_draw_seed(&course, 0).expect("p0");
    let p42 = Player::with_draw_seed(&course, 42).expect("p42");
    let drawn_0: Vec<&str> = p0.play_order().iter().map(|s| s.id.as_str()).collect();
    let drawn_42: Vec<&str> = p42.play_order().iter().map(|s| s.id.as_str()).collect();
    assert_ne!(
        drawn_0, drawn_42,
        "seed 0 and seed 42 should produce different draws"
    );

    // Answer the first drawn question on seed 42, then suspend & resume.
    let mut p = p42;
    p.handle(&Input::Start);
    p.handle(&Input::SelectChoice {
        choice: "true".into(),
    });
    p.handle(&Input::Submit);
    let first_q_id = current_id(&p).to_owned();
    p.handle(&Input::GoTo {
        slide: "s-results".into(),
    });

    let encoded = encode_suspend(&course, p.state());
    let decoded: PlayerState = decode_suspend(&course, &encoded).expect("decode suspend");
    assert!(decoded.review_answers.is_empty());

    let mut resumed = Player::resume(&course, decoded).expect("resume");
    resumed.handle(&Input::Start);
    let drawn_resumed: Vec<&str> = resumed.play_order().iter().map(|s| s.id.as_str()).collect();
    assert_eq!(drawn_resumed, drawn_42);

    // ReviewQuiz after resume locks answered questions without crashing on empty review_answers.
    let fx = resumed.handle(&Input::Click {
        object: "btn-review".into(),
    });
    assert!(!has_warning(&fx), "unexpected warning on review: {fx:?}");
    assert_eq!(current_id(&resumed), first_q_id);
    // Attempting to re-submit while in review/locked state warns rather than panicking.
    let fx = resumed.handle(&Input::SelectChoice {
        choice: "false".into(),
    });
    assert!(has_warning(&fx));
    let _ = SlideTarget::Next;
}
