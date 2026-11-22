use super::*;
use coursecraft_course::{CompletionStatus, Course};

fn three_slide_course() -> Course {
    let json = r#"{
        "schema_version": 1,
        "id": "subset-logic-test",
        "title": "Subset Logic Test",
        "slide_size": { "w": 1280, "h": 720 },
        "variables": [
            { "name": "count", "value": 10 }
        ],
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
                                "id": "btn_next",
                                "kind": { "type": "button", "label": "Go S2" },
                                "frame": { "x": 100, "y": 100, "w": 120, "h": 40 }
                            },
                            {
                                "id": "btn_s3",
                                "kind": { "type": "button", "label": "Go S3" },
                                "frame": { "x": 100, "y": 160, "w": 120, "h": 40 }
                            },
                            {
                                "id": "btn_complete",
                                "kind": { "type": "button", "label": "Finish" },
                                "frame": { "x": 100, "y": 220, "w": 120, "h": 40 }
                            },
                            {
                                "id": "btn_exit",
                                "kind": { "type": "button", "label": "Exit" },
                                "frame": { "x": 100, "y": 280, "w": 120, "h": 40 }
                            },
                            {
                                "id": "btn_lb_s2",
                                "kind": { "type": "button", "label": "LB S2" },
                                "frame": { "x": 100, "y": 340, "w": 120, "h": 40 }
                            }
                        ],
                        "triggers": [
                            {
                                "when": { "event": "click", "object": "btn_next" },
                                "action": { "action": "jump_to_slide", "target": "next" }
                            },
                            {
                                "when": { "event": "click", "object": "btn_s3" },
                                "action": { "action": "jump_to_slide", "target": { "id": "s3" } }
                            },
                            {
                                "when": { "event": "click", "object": "btn_complete" },
                                "action": { "action": "complete_course", "status": "completed" }
                            },
                            {
                                "when": { "event": "click", "object": "btn_exit" },
                                "action": { "action": "exit_course" }
                            },
                            {
                                "when": { "event": "click", "object": "btn_lb_s2" },
                                "action": { "action": "lightbox_slide", "slide": "s2" }
                            }
                        ]
                    },
                    {
                        "id": "s2",
                        "title": "Slide 2",
                        "duration_ms": 5000,
                        "objects": [],
                        "triggers": []
                    },
                    {
                        "id": "s3",
                        "title": "Slide 3",
                        "duration_ms": 5000,
                        "objects": [
                            {
                                "id": "btn_jump_s2",
                                "kind": { "type": "button", "label": "Jump S2" },
                                "frame": { "x": 100, "y": 100, "w": 120, "h": 40 }
                            }
                        ],
                        "triggers": [
                            {
                                "when": { "event": "click", "object": "btn_jump_s2" },
                                "action": { "action": "jump_to_slide", "target": { "id": "s2" } }
                            }
                        ]
                    }
                ]
            }
        ]
    }"#;
    Course::from_json(json).expect("valid course")
}

#[test]
fn single_slide_subset_hits_boundary_on_next_prev_jump_lightbox_and_course_end() {
    let course = three_slide_course();
    let mut player =
        Player::with_preview_subset(&course, 0, &["s1".to_owned()]).expect("valid subset");
    assert!(player.is_partial_preview());
    assert_eq!(player.play_order().len(), 1);
    player.handle(&Input::Start);
    assert_eq!(player.current_slide().map(|s| s.id.as_str()), Some("s1"));
    assert!(!player.take_preview_boundary());

    // Next leaves subset
    player.handle(&Input::Next);
    assert!(player.take_preview_boundary());
    assert_eq!(player.current_slide().map(|s| s.id.as_str()), Some("s1"));

    // Previous leaves subset
    player.handle(&Input::Previous);
    assert!(player.take_preview_boundary());
    assert_eq!(player.current_slide().map(|s| s.id.as_str()), Some("s1"));

    // On-slide button jumping to Next
    player.handle(&Input::Click {
        object: "btn_next".into(),
    });
    assert!(player.take_preview_boundary());
    assert_eq!(player.current_slide().map(|s| s.id.as_str()), Some("s1"));

    // On-slide button jumping to unselected s3
    player.handle(&Input::Click {
        object: "btn_s3".into(),
    });
    assert!(player.take_preview_boundary());
    assert_eq!(player.current_slide().map(|s| s.id.as_str()), Some("s1"));

    // Lightbox to unselected s2
    player.handle(&Input::Click {
        object: "btn_lb_s2".into(),
    });
    assert!(player.take_preview_boundary());
    assert!(!player.lightbox_open());

    // CompleteCourse
    let fx = player.handle(&Input::Click {
        object: "btn_complete".into(),
    });
    assert!(fx.contains(&Effect::Completion {
        status: CompletionStatus::Completed
    }));
    assert!(player.take_preview_boundary());

    // ExitCourse stays on slide and does not emit Effect::Exit
    let fx = player.handle(&Input::Click {
        object: "btn_exit".into(),
    });
    assert!(!fx.contains(&Effect::Exit));
    assert!(!player.state().exited);
    assert!(player.take_preview_boundary());
}

#[test]
fn multi_slide_subset_navigates_within_subset_and_blocks_outside() {
    let course = three_slide_course();
    // Pass out-of-order subset [s3, s1] -> should be in course order [s1, s3]
    let mut player = Player::with_preview_subset(&course, 0, &["s3".to_owned(), "s1".to_owned()])
        .expect("valid subset");
    let order: Vec<&str> = player.play_order().iter().map(|s| s.id.as_str()).collect();
    assert_eq!(order, vec!["s1", "s3"]);

    player.handle(&Input::Start);
    assert_eq!(player.current_slide().map(|s| s.id.as_str()), Some("s1"));

    // Jump to s3 (in subset) works
    player.handle(&Input::Click {
        object: "btn_s3".into(),
    });
    assert!(!player.take_preview_boundary());
    assert_eq!(player.current_slide().map(|s| s.id.as_str()), Some("s3"));

    // Jump from s3 to unselected s2 hits boundary and stays on s3
    player.handle(&Input::Click {
        object: "btn_jump_s2".into(),
    });
    assert!(player.take_preview_boundary());
    assert_eq!(player.current_slide().map(|s| s.id.as_str()), Some("s3"));

    // Previous from s3 goes back to s1
    player.handle(&Input::Previous);
    assert!(!player.take_preview_boundary());
    assert_eq!(player.current_slide().map(|s| s.id.as_str()), Some("s1"));

    // Resume trusted preserves preview_subset
    let plan = player.play_plan();
    let resumed = Player::resume_trusted(&course, player.state().clone(), &plan).expect("resume");
    assert!(resumed.is_partial_preview());
    assert_eq!(
        resumed
            .play_order()
            .iter()
            .map(|s| s.id.as_str())
            .collect::<Vec<_>>(),
        vec!["s1", "s3"]
    );
}
