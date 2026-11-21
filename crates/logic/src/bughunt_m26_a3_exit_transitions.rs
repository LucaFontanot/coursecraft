//! M26 bug-hunt tests for A3: Exit animations & slide transitions.
//!
//! Ordered P1 -> P2 -> P3.

use crate::{Effect, Input, Player};
use coursecraft_course::{AnimEffect, Course, Direction};

fn two_slide_course(
    s2_dur: u32,
    trans_json: serde_json::Value,
    s2_objects: serde_json::Value,
) -> Course {
    let json = serde_json::json!({
        "schema_version": 1,
        "id": "a3-test",
        "title": "A3 Test",
        "slide_size": { "w": 1280, "h": 720 },
        "variables": [{ "name": "n", "value": 0.0 }],
        "scenes": [{
            "id": "sc1",
            "title": "Scene 1",
            "slides": [
                {
                    "id": "s1",
                    "title": "Slide 1",
                    "duration_ms": 2000,
                    "objects": [{
                        "id": "b1",
                        "kind": { "type": "button", "label": "Next" },
                        "frame": { "x": 100.0, "y": 100.0, "w": 120.0, "h": 40.0 }
                    }]
                },
                {
                    "id": "s2",
                    "title": "Slide 2",
                    "duration_ms": s2_dur,
                    "transition": trans_json,
                    "objects": s2_objects
                },
                {
                    "id": "s3",
                    "title": "Slide 3",
                    "duration_ms": 2000,
                    "objects": []
                }
            ]
        }]
    })
    .to_string();
    Course::from_json(&json).expect("valid course")
}

// ============================================================================
// P1
// ============================================================================

#[test]
fn a3_1_real_multi_slide_transition_and_every_exit_effect() {
    let transitions = [
        serde_json::json!({ "type": "fade", "duration_ms": 500 }),
        serde_json::json!({ "type": "push", "from": "left", "duration_ms": 500 }),
        serde_json::json!({ "type": "cover", "from": "right", "duration_ms": 500 }),
    ];
    let exits: [(&str, AnimEffect); 5] = [
        ("fade", AnimEffect::Fade),
        (
            "fly_left",
            AnimEffect::FlyIn {
                from: Direction::Left,
            },
        ),
        ("zoom", AnimEffect::Zoom),
        (
            "wipe_right",
            AnimEffect::Wipe {
                from: Direction::Right,
            },
        ),
        (
            "float_bottom",
            AnimEffect::Float {
                from: Direction::Bottom,
            },
        ),
    ];

    for trans in transitions {
        for (name, exit_eff) in &exits {
            let eff_json = serde_json::to_value(exit_eff).unwrap();
            let course = two_slide_course(
                5000,
                trans.clone(),
                serde_json::json!([
                    {
                        "id": "obj-exit",
                        "kind": { "type": "shape", "shape": "rectangle", "fill": "#FF0000" },
                        "frame": { "x": 400.0, "y": 200.0, "w": 200.0, "h": 100.0 },
                        "exit": { "effect": eff_json, "duration_ms": 1000, "easing": "linear" }
                    },
                    {
                        "id": "obj-fade",
                        "kind": { "type": "shape", "shape": "rectangle", "fill": "#00FF00" },
                        "frame": { "x": 700.0, "y": 200.0, "w": 200.0, "h": 100.0 },
                        "exit": { "effect": { "type": "fade" }, "duration_ms": 1000, "easing": "linear" }
                    }
                ]),
            );
            let mut p = Player::new(&course).expect("player");
            let _ = p.handle(&Input::Start);
            let _ = p.handle(&Input::GoTo { slide: "s2".into() });
            let _ = p.handle(&Input::Tick { ms: 500 });

            let _ = p.handle(&Input::Tick { ms: 4000 }); // t = 4500ms
            let s_exit = p.sample_object("obj-exit").expect("obj-exit mid-exit");
            let s_fade = p.sample_object("obj-fade").expect("obj-fade mid-exit");
            assert_ne!(
                s_exit,
                crate::ObjectSample::IDENTITY,
                "{name}: exit should animate"
            );
            assert!(
                s_fade.opacity < 0.9,
                "{name}: fade exit opacity={}",
                s_fade.opacity
            );

            let _ = p.handle(&Input::Tick { ms: 500 }); // t = 5000ms
            assert!(p.sample_object("obj-exit").is_none());
            assert!(p.sample_object("obj-fade").is_none());
        }
    }
}

#[test]
fn a3_3_explicit_end_ms_vs_slide_end_exit_with_transition() {
    let course = two_slide_course(
        5000,
        serde_json::json!({ "type": "fade", "duration_ms": 500 }),
        serde_json::json!([
            {
                "id": "early",
                "kind": { "type": "shape", "shape": "rectangle", "fill": "#FF0000" },
                "frame": { "x": 200.0, "y": 200.0, "w": 160.0, "h": 80.0 },
                "timing": { "start_ms": 0, "end_ms": 3000 },
                "exit": { "effect": { "type": "fly_in", "from": "left" }, "duration_ms": 1000, "easing": "linear" }
            },
            {
                "id": "late",
                "kind": { "type": "shape", "shape": "rectangle", "fill": "#0000FF" },
                "frame": { "x": 500.0, "y": 200.0, "w": 160.0, "h": 80.0 },
                "exit": { "effect": { "type": "fly_in", "from": "left" }, "duration_ms": 1000, "easing": "linear" }
            }
        ]),
    );
    let mut p = Player::new(&course).expect("player");
    let _ = p.handle(&Input::Start);
    let _ = p.handle(&Input::GoTo { slide: "s2".into() });

    let _ = p.handle(&Input::Tick { ms: 2500 });
    assert!(p.sample_object("early").unwrap().offset.0.abs() > 1.0);
    assert_eq!(
        p.sample_object("late").unwrap(),
        crate::ObjectSample::IDENTITY
    );

    let _ = p.handle(&Input::Tick { ms: 500 }); // 3000ms
    assert!(p.sample_object("early").is_none());
    assert_eq!(
        p.sample_object("late").unwrap(),
        crate::ObjectSample::IDENTITY
    );

    let _ = p.handle(&Input::Tick { ms: 1500 }); // 4500ms
    assert!(p.sample_object("late").unwrap().offset.0.abs() > 1.0);

    let _ = p.handle(&Input::Tick { ms: 500 }); // 5000ms
    assert!(p.sample_object("late").is_none());
}

#[test]
fn a3_4_exit_animations_on_slide_layer_while_base_has_transition() {
    for pause_base in [false, true] {
        let mut course = two_slide_course(
            5000,
            serde_json::json!({ "type": "cover", "from": "left", "duration_ms": 500 }),
            serde_json::json!([
                {
                    "id": "show-l1",
                    "kind": { "type": "button", "label": "Show" },
                    "frame": { "x": 100.0, "y": 100.0, "w": 120.0, "h": 40.0 }
                }
            ]),
        );
        course.scenes[0].slides[1].triggers.push(
            serde_json::from_value(serde_json::json!({
                "when": { "event": "click", "object": "show-l1" },
                "action": { "action": "show_layer", "layer": "l1" }
            }))
            .unwrap(),
        );
        course.scenes[0].slides[1].layers.push(
            serde_json::from_value(serde_json::json!({
                "id": "l1",
                "name": "L1",
                "duration_ms": 3000,
                "pause_base_timeline": pause_base,
                "objects": [{
                    "id": "l-fly",
                    "kind": { "type": "shape", "shape": "rectangle", "fill": "#FFAA00" },
                    "frame": { "x": 300.0, "y": 200.0, "w": 160.0, "h": 80.0 },
                    "exit": { "effect": { "type": "fly_in", "from": "left" }, "duration_ms": 1000, "easing": "linear" }
                }]
            }))
            .unwrap(),
        );

        let mut p = Player::new(&course).expect("player");
        let _ = p.handle(&Input::Start);
        let _ = p.handle(&Input::GoTo { slide: "s2".into() });
        let _ = p.handle(&Input::Tick { ms: 1000 });
        let _ = p.handle(&Input::Click {
            object: "show-l1".into(),
        });

        let _ = p.handle(&Input::Tick { ms: 1500 });
        assert_eq!(
            p.sample_object("l-fly").unwrap(),
            crate::ObjectSample::IDENTITY
        );

        let _ = p.handle(&Input::Tick { ms: 1000 }); // layer clock 2500ms
        assert!(p.sample_object("l-fly").unwrap().offset.0.abs() > 1.0);

        let _ = p.handle(&Input::Tick { ms: 500 }); // layer clock 3000ms
        assert!(p.sample_object("l-fly").is_none());
    }
}

// ============================================================================
// P2
// ============================================================================

#[test]
fn a3_2_transition_overlapping_exit_animation_window() {
    let course = two_slide_course(
        1500,
        serde_json::json!({ "type": "push", "from": "right", "duration_ms": 1000 }),
        serde_json::json!([{
            "id": "obj",
            "kind": { "type": "shape", "shape": "rectangle", "fill": "#FF0000" },
            "frame": { "x": 400.0, "y": 200.0, "w": 200.0, "h": 100.0 },
            "exit": { "effect": { "type": "fly_in", "from": "left" }, "duration_ms": 800, "easing": "linear" }
        }]),
    );
    let mut p = Player::new(&course).expect("player");
    let _ = p.handle(&Input::Start);
    let _ = p.handle(&Input::GoTo { slide: "s2".into() });
    let _ = p.handle(&Input::Tick { ms: 850 });
    assert!(p.sample_object("obj").unwrap().offset.0 < -1.0);

    let _ = p.handle(&Input::Tick { ms: 250 }); // t = 1100ms
    assert!(p.sample_object("obj").unwrap().offset.0 < -100.0);

    let _ = p.handle(&Input::Tick { ms: 400 }); // t = 1500ms
    assert!(p.sample_object("obj").is_none());
}

#[test]
fn a3_6_seek_pause_resume_and_speed_inside_exit_window() {
    let course = two_slide_course(
        5000,
        serde_json::json!({ "type": "fade", "duration_ms": 500 }),
        serde_json::json!([{
            "id": "obj",
            "kind": { "type": "shape", "shape": "rectangle", "fill": "#FF0000" },
            "frame": { "x": 400.0, "y": 200.0, "w": 200.0, "h": 100.0 },
            "exit": { "effect": { "type": "fly_in", "from": "left" }, "duration_ms": 1000, "easing": "linear" }
        }]),
    );
    let mut p = Player::new(&course).expect("player");
    let _ = p.handle(&Input::Start);
    let _ = p.handle(&Input::GoTo { slide: "s2".into() });
    let _ = p.handle(&Input::Tick { ms: 5000 });
    assert!(p.sample_object("obj").is_none());

    let _ = p.handle(&Input::PlayerSeek { ms: 4500 });
    let off_4500 = p
        .sample_object("obj")
        .expect("re-materialized at 4500")
        .offset
        .0;
    assert!(off_4500 < -10.0);

    let _ = p.handle(&Input::PlayerPause);
    let _ = p.handle(&Input::Tick { ms: 1000 });
    assert!((p.sample_object("obj").unwrap().offset.0 - off_4500).abs() < 0.01);

    let _ = p.handle(&Input::PlayerResume);
    let _ = p.handle(&Input::SetSpeed { rate: 2.0 });
    let _ = p.handle(&Input::Tick { ms: 125 }); // +250ms -> 4750ms
    let off_4750 = p.sample_object("obj").unwrap().offset.0;
    assert!(off_4750 < off_4500 - 10.0);

    let _ = p.handle(&Input::PlayerSeek { ms: 2000 });
    assert_eq!(
        p.sample_object("obj").unwrap(),
        crate::ObjectSample::IDENTITY
    );
}

#[test]
fn a3_7_replay_and_revisit_reset_vs_resume_after_exit() {
    for revisit in [
        coursecraft_course::RevisitMode::Reset,
        coursecraft_course::RevisitMode::Resume,
    ] {
        let mut course = two_slide_course(
            5000,
            serde_json::json!({ "type": "fade", "duration_ms": 500 }),
            serde_json::json!([{
                "id": "obj",
                "kind": { "type": "shape", "shape": "rectangle", "fill": "#FF0000" },
                "frame": { "x": 400.0, "y": 200.0, "w": 200.0, "h": 100.0 },
                "exit": { "effect": { "type": "fly_in", "from": "left" }, "duration_ms": 1000, "easing": "linear" }
            }]),
        );
        course.scenes[0].slides[1].revisit = revisit;
        let mut p = Player::new(&course).expect("player");
        let _ = p.handle(&Input::Start);
        let _ = p.handle(&Input::GoTo { slide: "s2".into() });
        let _ = p.handle(&Input::Tick { ms: 5000 });
        assert!(p.sample_object("obj").is_none());

        let _ = p.handle(&Input::GoTo { slide: "s3".into() });
        let _ = p.handle(&Input::GoTo { slide: "s2".into() });
        if revisit == coursecraft_course::RevisitMode::Reset {
            let _ = p.handle(&Input::Tick { ms: 4500 });
            assert!(p.sample_object("obj").unwrap().offset.0 < -10.0);
        } else {
            assert!(p.sample_object("obj").is_none());
            let _ = p.handle(&Input::PlayerSeek { ms: 0 });
            let _ = p.handle(&Input::Tick { ms: 4500 });
            assert!(p.sample_object("obj").unwrap().offset.0 < -10.0);
        }
    }
}

#[test]
fn a3_9_animation_completes_exit_trigger_fires_once_and_blocks_clicks() {
    let mut course = two_slide_course(
        5000,
        serde_json::json!({ "type": "fade", "duration_ms": 500 }),
        serde_json::json!([{
            "id": "obj-fly",
            "kind": { "type": "button", "label": "Fly" },
            "frame": { "x": 400.0, "y": 200.0, "w": 200.0, "h": 100.0 },
            "timing": { "start_ms": 0, "end_ms": 4000 },
            "entrance": { "effect": { "type": "fade" }, "duration_ms": 500, "easing": "linear" },
            "exit": { "effect": { "type": "fly_in", "from": "left" }, "duration_ms": 1000, "easing": "linear" }
        }]),
    );
    course.scenes[0].slides[1].triggers.push(
        serde_json::from_value(serde_json::json!({
            "when": { "event": "animation_completes", "object": "obj-fly" },
            "action": { "action": "set_variable", "variable": "n", "op": "add", "value": 1.0 }
        }))
        .unwrap(),
    );
    let mut p = Player::new(&course).expect("player");
    let _ = p.handle(&Input::Start);
    let _ = p.handle(&Input::GoTo { slide: "s2".into() });
    let _ = p.handle(&Input::Tick { ms: 3500 });
    assert_eq!(
        p.variable("n"),
        Some(&coursecraft_course::Value::Number(1.0))
    );

    let _ = p.handle(&Input::Tick { ms: 500 }); // t = 4000ms
    assert_eq!(
        p.variable("n"),
        Some(&coursecraft_course::Value::Number(1.0))
    );

    let _ = p.handle(&Input::Tick { ms: 500 }); // t = 4500ms
    assert_eq!(
        p.variable("n"),
        Some(&coursecraft_course::Value::Number(1.0))
    );
    let effs = p.handle(&Input::Click {
        object: "obj-fly".into(),
    });
    assert!(effs.iter().any(|e| matches!(e, Effect::Warning { .. })));
}

// ============================================================================
// P3
// ============================================================================

#[test]
fn a3_8_exit_animation_combined_with_motion_path_on_transitioned_slide() {
    let mut course = two_slide_course(
        5000,
        serde_json::json!({ "type": "fade", "duration_ms": 500 }),
        serde_json::json!([{
            "id": "obj",
            "kind": { "type": "shape", "shape": "rectangle", "fill": "#FF0000" },
            "frame": { "x": 200.0, "y": 200.0, "w": 100.0, "h": 100.0 },
            "motion_paths": [{
                "id": "mp1",
                "points": [{ "x": 0.0, "y": 0.0 }, { "x": 200.0, "y": 0.0 }],
                "duration_ms": 1000
            }],
            "exit": { "effect": { "type": "fly_in", "from": "left" }, "duration_ms": 1000, "easing": "linear" }
        }]),
    );
    course.scenes[0].slides[1].triggers.push(
        serde_json::from_value(serde_json::json!({
            "when": { "event": "timeline_starts" },
            "action": { "action": "move_on_path", "object": "obj", "path": "mp1" }
        }))
        .unwrap(),
    );
    let mut p = Player::new(&course).expect("player");
    let _ = p.handle(&Input::Start);
    let _ = p.handle(&Input::GoTo { slide: "s2".into() });
    let _ = p.handle(&Input::Tick { ms: 3000 });
    assert!((p.sample_object("obj").unwrap().offset.0 - 200.0).abs() < 1.0);

    let _ = p.handle(&Input::Tick { ms: 1500 }); // t = 4500ms
    let expected = 200.0 - 0.5 * (200.0 + 100.0);
    assert!((p.sample_object("obj").unwrap().offset.0 - expected).abs() < 2.0);
}
