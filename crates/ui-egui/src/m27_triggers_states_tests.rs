//! M27 Step D-2: Triggers (D1.1–D1.12) and Object States (D2.1–D2.10) test suite.

use crate::test_harness::TestHarness;
use crate::triggers::{ActionTag, ConditionNode, EventKindTag, JumpTargetTag};
use crate::{BottomTab, InspectorTab};
use coursecraft_course::{
    Action, CmpOp, Color, Condition, Course, Event as CourseEvent, Severity, Slide, SlideTarget,
    StateDef, Trigger, Value, ValueSource, VarOp,
};
use egui::{Key, Modifiers, pos2};

fn make_trig(when: CourseEvent, action: Action) -> Trigger {
    Trigger {
        when,
        action,
        condition: None,
        disabled: false,
    }
}

fn make_base_course() -> Course {
    let course: Course = serde_json::from_value(serde_json::json!({
        "schema_version": 1,
        "id": "m27-triggers-states",
        "title": "M27 Triggers & States",
        "slide_size": { "w": 1280, "h": 720 },
        "variables": [
            { "name": "score", "value": 0.0 },
            { "name": "target", "value": 10.0 },
            { "name": "label", "value": "ok" },
            { "name": "other_label", "value": "OK" },
            { "name": "flag", "value": false }
        ],
        "scenes": [{
            "id": "scene-1",
            "title": "Scene 1",
            "slides": [
                {
                    "id": "s1",
                    "title": "Slide 1",
                    "duration_ms": 5000,
                    "cue_points": [{ "id": "cue-1", "name": "Cue 1", "at_ms": 1500 }],
                    "objects": [
                        {
                            "id": "btn-start",
                            "name": "btn-start",
                            "kind": { "type": "button", "label": "Start", "fill": "#2563EB" },
                            "frame": { "x": 100.0, "y": 100.0, "w": 160.0, "h": 48.0 },
                            "states": [{ "name": "visited", "fill": "#16A34A", "text": "Visited" }],
                            "drag": { "targets": ["shape-1"] }
                        },
                        {
                            "id": "shape-1",
                            "name": "shape-1",
                            "kind": { "type": "shape", "shape": "rectangle", "fill": "#9333EA", "text": { "text": "Shape" } },
                            "frame": { "x": 300.0, "y": 100.0, "w": 140.0, "h": 80.0 },
                            "entrance": { "effect": { "type": "fade" }, "duration_ms": 400 },
                            "motion_paths": [{
                                "id": "mp-1",
                                "points": [{ "x": 0.0, "y": 0.0 }, { "x": 100.0, "y": 0.0 }],
                                "duration_ms": 500
                            }]
                        },
                        {
                            "id": "txt-1",
                            "name": "txt-1",
                            "kind": { "type": "text", "text": "Hello" },
                            "frame": { "x": 100.0, "y": 200.0, "w": 200.0, "h": 40.0 }
                        },
                        {
                            "id": "img-1",
                            "name": "img-1",
                            "kind": { "type": "image", "src": "hero.png" },
                            "frame": { "x": 100.0, "y": 260.0, "w": 160.0, "h": 120.0 },
                            "description": "Image"
                        },
                        {
                            "id": "aud-1",
                            "name": "aud-1",
                            "kind": { "type": "audio", "src": "clip.wav" },
                            "frame": { "x": 100.0, "y": 400.0, "w": 48.0, "h": 48.0 }
                        },
                        {
                            "id": "vid-1",
                            "name": "vid-1",
                            "kind": {
                                "type": "video",
                                "src": "clip.mp4",
                                "cues": [{ "id": "vcue-1", "name": "VCue 1", "at_ms": 1500 }]
                            },
                            "frame": { "x": 200.0, "y": 400.0, "w": 160.0, "h": 90.0 }
                        },
                        {
                            "id": "path-1",
                            "name": "path-1",
                            "kind": {
                                "type": "path",
                                "points": [{ "x": 0.0, "y": 0.0 }, { "x": 1.0, "y": 0.0 }, { "x": 0.5, "y": 1.0 }],
                                "closed": true,
                                "fill": "#F59E0B",
                                "outline": { "color": "#1E3A8A", "width": 2 }
                            },
                            "frame": { "x": 500.0, "y": 100.0, "w": 80.0, "h": 60.0 }
                        },
                        {
                            "id": "mk-1",
                            "name": "mk-1",
                            "kind": { "type": "marker", "title": "Info", "body": "Details" },
                            "frame": { "x": 500.0, "y": 200.0, "w": 32.0, "h": 32.0 }
                        },
                        {
                            "id": "t1",
                            "name": "Quiz Timer",
                            "kind": { "type": "timer", "duration_ms": 60000 },
                            "frame": { "x": 500.0, "y": 260.0, "w": 120.0, "h": 40.0 }
                        }
                    ],
                    "layers": [{
                        "id": "l1",
                        "name": "Layer 1",
                        "objects": [
                            {
                                "id": "layer-txt",
                                "name": "layer-txt",
                                "kind": { "type": "text", "text": "On layer" },
                                "frame": { "x": 100.0, "y": 200.0, "w": 200.0, "h": 40.0 }
                            }
                        ]
                    }]
                },
                {
                    "id": "s2",
                    "title": "Slide 2",
                    "duration_ms": 5000,
                    "objects": [
                        {
                            "id": "btn-2",
                            "name": "btn-2",
                            "kind": { "type": "button", "label": "Next", "fill": "#2563EB" },
                            "frame": { "x": 100.0, "y": 100.0, "w": 160.0, "h": 48.0 }
                        }
                    ]
                }
            ]
        }],
        "masters": [{
            "id": "m1-l1",
            "title": "Content",
            "duration_ms": 5000,
            "objects": [
                {
                    "id": "master-shape",
                    "name": "master-shape",
                    "kind": { "type": "shape", "shape": "rectangle", "fill": "#9333EA" },
                    "frame": { "x": 100.0, "y": 100.0, "w": 140.0, "h": 80.0 }
                }
            ]
        }]
    }))
    .expect("valid base course json");
    course
}

fn open_triggers_tab(harness: &mut TestHarness) {
    harness.editor.ui.bottom_tab = BottomTab::Triggers;
    harness.idle_frame();
    harness.idle_frame();
}

fn open_states_panel(harness: &mut TestHarness) {
    harness.editor.ui.bottom_tab = crate::BottomTab::States;
    harness.editor.ui.states.show_new_state_input = true;
    harness.idle_frame();
}

fn clear_and_submit_field(harness: &mut TestHarness, pos: egui::Pos2, text: &str) {
    harness.click_at(pos);
    harness.press(Key::A, Modifiers::COMMAND);
    harness.press(Key::Backspace, Modifiers::NONE);
    if !text.is_empty() {
        harness.type_text(text);
    }
    harness.press(Key::Enter, Modifiers::NONE);
    harness.idle_frame();
}

fn assert_no_course_errors(course: &Course) {
    let errors: Vec<_> = course
        .validate()
        .into_iter()
        .filter(|i| i.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "unexpected course errors: {errors:?}");
}

fn undo_key(harness: &mut TestHarness) {
    harness.press(Key::Z, Modifiers::COMMAND);
    harness.idle_frame();
}

fn redo_key(harness: &mut TestHarness) {
    harness.press(Key::Z, Modifiers::COMMAND | Modifiers::SHIFT);
    harness.idle_frame();
}

// =========================================================================
// D1: Triggers (12 Variations)
// =========================================================================

#[test]
fn d1_1_trigger_full_lifecycle_default_object_and_undo_redo() {
    let mut harness = TestHarness::new(make_base_course());
    harness.editor.selection.slide = "s1".to_owned();
    harness.editor.selection.object = Some("btn-start".to_owned());
    open_triggers_tab(&mut harness);

    let new_pos = harness
        .find_text("+ Add trigger")
        .expect("+ Add trigger button");
    harness.click_at(new_pos);

    {
        let form = harness
            .editor
            .ui
            .trigger_form
            .as_mut()
            .expect("trigger form open");
        assert_eq!(form.event_tag, EventKindTag::Click);
        assert_eq!(form.event_object, "btn-start");
        form.action_tag = ActionTag::ChangeState;
        form.change_state_object = "btn-start".to_owned();
        form.change_state_state = "visited".to_owned();
    }
    harness.press(Key::Enter, Modifiers::COMMAND);
    harness.idle_frame();

    assert!(harness.editor.ui.trigger_form.is_none());
    assert_eq!(
        harness
            .editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .triggers
            .len(),
        1
    );
    assert_eq!(harness.editor.session.undo_len(), 1);

    // Edit the trigger row via row click + Edit button
    let row_pos = harness
        .find_text_containing("state to visited")
        .expect("trigger row text");
    harness.click_at(row_pos);
    let edit_pos = harness
        .find_icon(coursecraft_render::Icon::PencilSimple)
        .expect("edit trigger icon");
    harness.click_at(edit_pos);
    harness.idle_frame();
    {
        let form = harness
            .editor
            .ui
            .trigger_form
            .as_mut()
            .expect("edit form open");
        assert_eq!(form.editing_index, Some(0));
        form.action_tag = ActionTag::SetVariable;
        form.set_var_name = "score".to_owned();
        form.set_var_op = VarOp::Add;
        form.set_var_val_num = 5.0;
    }
    harness.press(Key::Enter, Modifiers::COMMAND);
    harness.idle_frame();

    assert_eq!(harness.editor.session.undo_len(), 2);
    assert!(matches!(
        harness.editor.session.course.slide("s1").unwrap().triggers[0].action,
        Action::SetVariable { .. }
    ));

    // Delete via row action
    let row_pos = harness
        .find_text_containing("Set score")
        .expect("updated trigger row");
    harness.click_at(row_pos);
    let del_pos = harness
        .find_icon(coursecraft_render::Icon::Trash)
        .expect("delete trigger icon");
    harness.click_at(del_pos);
    harness.idle_frame();

    assert!(
        harness
            .editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .triggers
            .is_empty()
    );
    assert_eq!(harness.editor.session.undo_len(), 3);

    // Undo x 3
    undo_key(&mut harness);
    assert_eq!(
        harness
            .editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .triggers
            .len(),
        1
    );
    assert!(matches!(
        harness.editor.session.course.slide("s1").unwrap().triggers[0].action,
        Action::SetVariable { .. }
    ));
    undo_key(&mut harness);
    assert!(matches!(
        harness.editor.session.course.slide("s1").unwrap().triggers[0].action,
        Action::ChangeState { .. }
    ));
    undo_key(&mut harness);
    assert!(
        harness
            .editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .triggers
            .is_empty()
    );

    // Redo x 3
    redo_key(&mut harness);
    assert!(matches!(
        harness.editor.session.course.slide("s1").unwrap().triggers[0].action,
        Action::ChangeState { .. }
    ));
    redo_key(&mut harness);
    assert!(matches!(
        harness.editor.session.course.slide("s1").unwrap().triggers[0].action,
        Action::SetVariable { .. }
    ));
    redo_key(&mut harness);
    assert!(
        harness
            .editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .triggers
            .is_empty()
    );
    assert_no_course_errors(&harness.editor.session.course);
}

#[test]
fn d1_2_all_action_and_event_tags_switch_and_commit_cleanly() {
    let mut harness = TestHarness::new(make_base_course());
    harness.editor.selection.slide = "s1".to_owned();
    open_triggers_tab(&mut harness);

    // Cycle through all 26 ActionTags + all 5 JumpTargetTags
    for &action_tag in &ActionTag::ALL {
        if matches!(
            action_tag,
            ActionTag::SubmitQuestion
                | ActionTag::SubmitResults
                | ActionTag::RetryQuiz
                | ActionTag::ReviewQuiz
                | ActionTag::ExitReview
        ) {
            // These quiz/results actions require a question or results slide;
            // verify the form switches to/from them cleanly without clipping.
            let new_pos = harness
                .find_text("+ Add trigger")
                .expect("+ Add trigger button");
            harness.click_at(new_pos);
            {
                let form = harness.editor.ui.trigger_form.as_mut().expect("form");
                form.action_tag = action_tag;
            }
            harness.idle_frame();
            let report = harness.layout_report(1);
            assert!(
                report.findings.is_empty(),
                "layout findings for {action_tag:?}: {:?}",
                report.findings
            );
            harness.press(Key::Escape, Modifiers::NONE);
            harness.idle_frame();
            continue;
        }
        let jump_targets: &[JumpTargetTag] = if action_tag == ActionTag::JumpToSlide {
            &[
                JumpTargetTag::Next,
                JumpTargetTag::Previous,
                JumpTargetTag::LastVisited,
                JumpTargetTag::SlideId,
                JumpTargetTag::Scene,
            ]
        } else {
            &[JumpTargetTag::Next]
        };

        for &jt in jump_targets {
            let new_pos = harness
                .find_text("+ Add trigger")
                .expect("+ Add trigger button");
            harness.click_at(new_pos);

            // Switch through unrelated tags first to verify stale-field immunity
            {
                let form = harness.editor.ui.trigger_form.as_mut().expect("form");
                form.action_tag = ActionTag::OpenUrl;
                form.open_url = "https://example.com".to_owned();
                form.action_tag = ActionTag::RunJavaScript;
                form.javascript_code = "console.log(1);".to_owned();
                form.action_tag = action_tag;
                form.jump_target_tag = jt;
                form.jump_slide_id = "s2".to_owned();
                form.jump_scene_id = "scene-1".to_owned();
                form.show_layer_id = "l1".to_owned();
                form.hide_layer_id = Some("l1".to_owned());
                form.change_state_object = "btn-start".to_owned();
                form.change_state_state = "visited".to_owned();
                form.set_var_name = "score".to_owned();
                form.set_var_op = VarOp::Assign;
                form.set_var_val_num = 10.0;
                form.media_object = "aud-1".to_owned();
                form.move_path_object = "shape-1".to_owned();
                form.move_path_id = "mp-1".to_owned();
                form.lightbox_slide_id = "s2".to_owned();
                form.timer_object = "t1".to_owned();
            }
            harness.idle_frame();
            let report = harness.layout_report(1);
            assert!(
                report.findings.is_empty(),
                "layout findings for {action_tag:?}: {:?}",
                report.findings
            );

            harness.press(Key::Enter, Modifiers::COMMAND);
            harness.idle_frame();
            assert!(
                harness.editor.ui.trigger_form.is_none(),
                "failed to commit {action_tag:?} ({jt:?})"
            );
        }
    }

    // Cycle through all 19 EventKindTags
    for &event_tag in &EventKindTag::ALL {
        let new_pos = harness
            .find_text("+ Add trigger")
            .expect("+ Add trigger button");
        harness.click_at(new_pos);
        {
            let form = harness.editor.ui.trigger_form.as_mut().expect("form");
            form.action_tag = ActionTag::ExitCourse;
            form.event_tag = event_tag;
            form.event_object = match event_tag {
                EventKindTag::MediaEnds => "aud-1".to_owned(),
                EventKindTag::CueReached => "vid-1".to_owned(),
                EventKindTag::MarkerOpened | EventKindTag::MarkerClosed => "mk-1".to_owned(),
                EventKindTag::AnimationCompletes => "shape-1".to_owned(),
                _ => "btn-start".to_owned(),
            };
            form.event_drop_target = "shape-1".to_owned();
            form.event_intersect_other = "shape-1".to_owned();
            form.anim_complete_object = "shape-1".to_owned();
            form.event_var = "score".to_owned();
            form.event_state_to = Some("visited".to_owned());
            form.event_key = "enter".to_owned();
            form.event_ms = 1000;
            form.timer_object = "t1".to_owned();
            form.event_cue = if event_tag == EventKindTag::CueReached {
                "vcue-1".to_owned()
            } else {
                "cue-1".to_owned()
            };
        }
        harness.idle_frame();
        harness.press(Key::Enter, Modifiers::COMMAND);
        harness.idle_frame();
        assert!(
            harness.editor.ui.trigger_form.is_none(),
            "failed to commit event {event_tag:?}"
        );
    }

    assert_no_course_errors(&harness.editor.session.course);
}

#[test]
fn d1_3_nested_condition_builder_round_trip_and_invalid_condition_error() {
    let mut harness = TestHarness::new(make_base_course());
    harness.editor.selection.slide = "s1".to_owned();
    open_triggers_tab(&mut harness);

    let new_pos = harness
        .find_text("+ Add trigger")
        .expect("+ Add trigger button");
    harness.click_at(new_pos);

    // Invalid empty condition field should show inline error and keep modal open
    {
        let form = harness.editor.ui.trigger_form.as_mut().expect("form");
        form.action_tag = ActionTag::ExitCourse;
        form.condition_enabled = true;
        form.condition_root = ConditionNode::Compare {
            variable: String::new(),
            op: CmpOp::Eq,
            value: ValueSource::Constant(Value::Number(1.0)),
        };
    }
    harness.press(Key::Enter, Modifiers::COMMAND);
    harness.idle_frame();
    assert!(harness.editor.ui.trigger_form.is_some());
    assert!(
        harness
            .editor
            .ui
            .trigger_form
            .as_ref()
            .unwrap()
            .error_message
            .is_some()
    );

    let expected_cond = Condition::All(vec![
        Condition::Compare {
            variable: "score".to_owned(),
            op: CmpOp::Ge,
            value: ValueSource::Constant(Value::Number(5.0)),
        },
        Condition::Compare {
            variable: "label".to_owned(),
            op: CmpOp::EqIgnoreCase,
            value: ValueSource::Variable {
                variable: "other_label".to_owned(),
            },
        },
        Condition::Not(Box::new(Condition::ObjectState {
            object: "btn-start".to_owned(),
            state: "visited".to_owned(),
            is: false,
        })),
    ]);
    {
        let form = harness.editor.ui.trigger_form.as_mut().expect("form");
        form.condition_root = ConditionNode::All(vec![
            ConditionNode::Compare {
                variable: "score".to_owned(),
                op: CmpOp::Ge,
                value: ValueSource::Constant(Value::Number(5.0)),
            },
            ConditionNode::Compare {
                variable: "label".to_owned(),
                op: CmpOp::EqIgnoreCase,
                value: ValueSource::Variable {
                    variable: "other_label".to_owned(),
                },
            },
            ConditionNode::Not(Box::new(ConditionNode::ObjectState {
                object: "btn-start".to_owned(),
                state: "visited".to_owned(),
                is: false,
            })),
        ]);
    }
    harness.press(Key::Enter, Modifiers::COMMAND);
    harness.idle_frame();

    assert!(harness.editor.ui.trigger_form.is_none());
    let stored = &harness.editor.session.course.slide("s1").unwrap().triggers[0];
    assert_eq!(stored.condition.as_ref(), Some(&expected_cond));

    // Reopen via row click + Edit button and verify exact round-trip
    let row_pos = harness
        .find_text_containing("Exit course")
        .expect("trigger row");
    harness.click_at(row_pos);
    let edit_pos = harness
        .find_icon(coursecraft_render::Icon::PencilSimple)
        .expect("edit trigger icon");
    harness.click_at(edit_pos);
    harness.idle_frame();
    let reopened = harness
        .editor
        .ui
        .trigger_form
        .as_ref()
        .expect("reopened form");
    assert!(reopened.condition_enabled);
    assert_eq!(
        reopened.condition_root.to_condition().unwrap(),
        expected_cond
    );

    // Cancel and undo
    harness.press(Key::Escape, Modifiers::NONE);
    undo_key(&mut harness);
    assert!(
        harness
            .editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .triggers
            .is_empty()
    );
}

#[test]
fn d1_4_bug_d1_move_up_down_with_filter_and_selected_object_sort() {
    let mut course = make_base_course();
    let slide = course.slide_mut("s1").unwrap();
    // 5 triggers across A ("shape-1") and B ("btn-start"): [A0, A1, B0, A2, B1]
    slide.triggers = vec![
        make_trig(
            CourseEvent::Click {
                object: "shape-1".to_owned(),
            },
            Action::ShowLayer {
                layer: "l1".to_owned(),
            },
        ),
        make_trig(
            CourseEvent::HoverStart {
                object: "shape-1".to_owned(),
            },
            Action::HideLayer {
                layer: Some("l1".to_owned()),
            },
        ),
        make_trig(
            CourseEvent::Click {
                object: "btn-start".to_owned(),
            },
            Action::JumpToSlide {
                target: SlideTarget::Next,
            },
        ),
        make_trig(
            CourseEvent::HoverEnd {
                object: "shape-1".to_owned(),
            },
            Action::PauseTimeline,
        ),
        make_trig(
            CourseEvent::HoverStart {
                object: "btn-start".to_owned(),
            },
            Action::ExitCourse,
        ),
    ];

    let mut harness = TestHarness::new(course);
    harness.editor.selection.slide = "s1".to_owned();
    harness.editor.selection.object = Some("btn-start".to_owned());
    harness.editor.ui.triggers.filter = "btn-start".to_owned();
    open_triggers_tab(&mut harness);

    // Click B's second trigger ("Exit course", raw idx 4) to reveal its action buttons
    let second_b = harness
        .find_text_containing("Exit course")
        .expect("second B trigger");
    harness.click_at(second_b);

    // Move Up should swap B1 (idx 4) with B0 (idx 2), NOT with hidden A2 (idx 3)
    let up_pos = harness
        .find_icon(coursecraft_render::Icon::ArrowUp)
        .expect("Move up icon");
    harness.click_at(up_pos);
    harness.idle_frame();

    let trigs = &harness.editor.session.course.slide("s1").unwrap().triggers;
    assert!(matches!(trigs[2].action, Action::ExitCourse));
    assert!(matches!(
        trigs[3].action,
        Action::JumpToSlide {
            target: SlideTarget::Next
        }
    ));
    assert!(matches!(trigs[4].action, Action::PauseTimeline));

    // Move Down on B1 (now at idx 2) should swap it back with B0 (idx 3)
    let down_pos = harness
        .find_icon(coursecraft_render::Icon::ArrowDown)
        .expect("Move down icon");
    harness.click_at(down_pos);
    harness.idle_frame();

    let trigs = &harness.editor.session.course.slide("s1").unwrap().triggers;
    assert!(matches!(
        trigs[2].action,
        Action::JumpToSlide {
            target: SlideTarget::Next
        }
    ));
    assert!(matches!(trigs[3].action, Action::ExitCourse));
    assert!(matches!(trigs[4].action, Action::PauseTimeline));

    // Undo restores original order
    undo_key(&mut harness);
    undo_key(&mut harness);
    let trigs = &harness.editor.session.course.slide("s1").unwrap().triggers;
    assert!(matches!(
        trigs[2].action,
        Action::JumpToSlide {
            target: SlideTarget::Next
        }
    ));
    assert!(matches!(trigs[4].action, Action::ExitCourse));
}

#[test]
fn d1_5_double_click_another_row_while_editor_already_open() {
    let mut course = make_base_course();
    course.slide_mut("s1").unwrap().triggers = vec![
        make_trig(
            CourseEvent::Click {
                object: "btn-start".to_owned(),
            },
            Action::ExitCourse,
        ),
        make_trig(
            CourseEvent::TimelineStarts { layer: None },
            Action::PauseTimeline,
        ),
    ];

    let mut harness = TestHarness::new(course);
    harness.editor.selection.slide = "s1".to_owned();
    open_triggers_tab(&mut harness);

    let row0 = harness.find_text_containing("Exit course").expect("row 0");
    harness.double_click_at(row0);
    {
        let form = harness.editor.ui.trigger_form.as_mut().expect("form open");
        assert_eq!(form.editing_index, Some(0));
        form.action_tag = ActionTag::PrintSlide;
    }
    harness.idle_frame();

    if let Some(row1) = harness.find_text_containing("Pause timeline") {
        harness.double_click_at(row1);
    }
    harness.idle_frame();

    let trigs = &harness.editor.session.course.slide("s1").unwrap().triggers;
    assert!(matches!(trigs[0].action, Action::ExitCourse));
    assert!(matches!(trigs[1].action, Action::PauseTimeline));
}

#[test]
fn d1_6_copy_and_paste_triggers_across_slides_and_layers_with_partial_skip() {
    let mut course = make_base_course();
    let s1 = course.slide_mut("s1").unwrap();
    s1.triggers = vec![
        make_trig(
            CourseEvent::TimelineStarts { layer: None },
            Action::ExitCourse,
        ),
        make_trig(
            CourseEvent::Click {
                object: "btn-start".to_owned(),
            },
            Action::PrintSlide,
        ),
        make_trig(
            CourseEvent::TimelineEnds { layer: None },
            Action::ShowLayer {
                layer: "l1".to_owned(),
            },
        ),
    ];

    let mut harness = TestHarness::new(course);
    harness.editor.selection.slide = "s1".to_owned();
    open_triggers_tab(&mut harness);

    harness.editor.ui.copied_triggers = harness
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .triggers
        .clone();

    // Switch to Slide 2 (lacks "btn-start" and "l1")
    harness.editor.selection.slide = "s2".to_owned();
    harness.idle_frame();

    let paste_pos = harness
        .find_text("Paste Triggers")
        .expect("Paste Triggers button");
    harness.click_at(paste_pos);
    harness.idle_frame();

    let s2_trigs = &harness.editor.session.course.slide("s2").unwrap().triggers;
    assert_eq!(s2_trigs.len(), 1);
    assert!(matches!(s2_trigs[0].action, Action::ExitCourse));
    let status = harness
        .editor
        .ui
        .status_message
        .as_ref()
        .map(|(m, _)| m.as_str())
        .unwrap_or("");
    assert!(
        status.contains("Pasted 1") && status.contains("skipped 2"),
        "unexpected paste status: {status:?}"
    );

    // Undo removes pasted trigger cleanly
    undo_key(&mut harness);
    assert!(
        harness
            .editor
            .session
            .course
            .slide("s2")
            .unwrap()
            .triggers
            .is_empty()
    );

    // Paste onto Layer "l1" on Slide 1
    harness.editor.selection.slide = "s1".to_owned();
    harness.editor.selection.layer = Some("l1".to_owned());
    harness.idle_frame();
    let paste_pos = harness
        .find_text("Paste Triggers")
        .expect("Paste Triggers on layer");
    harness.click_at(paste_pos);
    harness.idle_frame();
    assert!(
        !harness
            .editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .layer("l1")
            .unwrap()
            .triggers
            .is_empty()
    );
}

#[test]
fn d1_7_disable_enable_and_duplicate_via_more_menu() {
    let mut course = make_base_course();
    course.slide_mut("s1").unwrap().triggers = vec![make_trig(
        CourseEvent::Click {
            object: "btn-start".to_owned(),
        },
        Action::ExitCourse,
    )];

    let mut harness = TestHarness::new(course);
    harness.editor.selection.slide = "s1".to_owned();
    open_triggers_tab(&mut harness);

    let row_pos = harness.find_text_containing("Exit course").expect("row 0");
    harness.click_at(row_pos);

    // Open More menu -> Disable
    let more_pos = harness
        .find_icon(coursecraft_render::Icon::DotsThree)
        .expect("More button");
    harness.click_at(more_pos);
    let disable_pos = harness.find_text("Disable").expect("Disable menu item");
    harness.click_at(disable_pos);
    harness.idle_frame();

    assert!(harness.editor.session.course.slide("s1").unwrap().triggers[0].disabled);

    // Duplicate disabled trigger
    let more_pos = harness
        .find_icon(coursecraft_render::Icon::DotsThree)
        .expect("More button");
    harness.click_at(more_pos);
    let dup_pos = harness.find_text("Duplicate").expect("Duplicate menu item");
    harness.click_at(dup_pos);
    harness.idle_frame();

    let trigs = &harness.editor.session.course.slide("s1").unwrap().triggers;
    assert_eq!(trigs.len(), 2);
    assert!(trigs[1].disabled);

    // Re-enable original (row 0)
    let more_pos = harness
        .find_icon(coursecraft_render::Icon::DotsThree)
        .expect("More button");
    harness.click_at(more_pos);
    let enable_pos = harness.find_text("Enable").expect("Enable menu item");
    harness.click_at(enable_pos);
    harness.idle_frame();
    assert!(!harness.editor.session.course.slide("s1").unwrap().triggers[0].disabled);

    // Undo x 3
    undo_key(&mut harness);
    assert!(harness.editor.session.course.slide("s1").unwrap().triggers[0].disabled);
    undo_key(&mut harness);
    assert_eq!(
        harness
            .editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .triggers
            .len(),
        1
    );
    undo_key(&mut harness);
    assert!(!harness.editor.session.course.slide("s1").unwrap().triggers[0].disabled);
}

#[test]
fn d1_8_hostile_and_boundary_trigger_field_values_refused() {
    let mut harness = TestHarness::new(make_base_course());
    harness.editor.selection.slide = "s1".to_owned();
    let initial_rev = harness.editor.session.revision;
    open_triggers_tab(&mut harness);

    let new_pos = harness
        .find_text("+ Add trigger")
        .expect("+ Add trigger button");
    harness.click_at(new_pos);

    for bad_url in ["", "javascript:alert(1)"] {
        {
            let form = harness.editor.ui.trigger_form.as_mut().expect("form");
            form.action_tag = ActionTag::OpenUrl;
            form.open_url = bad_url.to_owned();
        }
        harness.press(Key::Enter, Modifiers::COMMAND);
        harness.idle_frame();
        assert!(harness.editor.ui.trigger_form.is_some());
        assert!(
            harness
                .editor
                .ui
                .trigger_form
                .as_ref()
                .unwrap()
                .error_message
                .is_some()
        );
        assert_eq!(harness.editor.session.revision, initial_rev);
    }

    for bad_js in [String::new(), "a".repeat(70 * 1024)] {
        {
            let form = harness.editor.ui.trigger_form.as_mut().expect("form");
            form.action_tag = ActionTag::RunJavaScript;
            form.javascript_code = bad_js;
        }
        harness.press(Key::Enter, Modifiers::COMMAND);
        harness.idle_frame();
        assert!(harness.editor.ui.trigger_form.is_some());
        assert!(
            harness
                .editor
                .ui
                .trigger_form
                .as_ref()
                .unwrap()
                .error_message
                .is_some()
        );
        assert_eq!(harness.editor.session.revision, initial_rev);
    }

    {
        let form = harness.editor.ui.trigger_form.as_mut().expect("form");
        form.action_tag = ActionTag::ExitCourse;
        form.event_tag = EventKindTag::KeyPress;
        form.event_key = String::new();
    }
    harness.press(Key::Enter, Modifiers::COMMAND);
    harness.idle_frame();
    assert!(harness.editor.ui.trigger_form.is_some());
    assert_eq!(harness.editor.session.revision, initial_rev);

    // Valid https URL succeeds
    {
        let form = harness.editor.ui.trigger_form.as_mut().expect("form");
        form.action_tag = ActionTag::OpenUrl;
        form.open_url = "https://example.com".to_owned();
        form.event_tag = EventKindTag::Click;
        form.event_object = "btn-start".to_owned();
    }
    harness.press(Key::Enter, Modifiers::COMMAND);
    harness.idle_frame();
    assert!(harness.editor.ui.trigger_form.is_none());
    assert_eq!(
        harness
            .editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .triggers
            .len(),
        1
    );
}

#[test]
fn d1_9_rapid_double_cmd_enter_and_open_esc_spam() {
    let mut harness = TestHarness::new(make_base_course());
    harness.editor.selection.slide = "s1".to_owned();
    open_triggers_tab(&mut harness);

    let new_pos = harness
        .find_text("+ Add trigger")
        .expect("+ Add trigger button");
    harness.click_at(new_pos);
    {
        let form = harness.editor.ui.trigger_form.as_mut().expect("form");
        form.action_tag = ActionTag::ExitCourse;
    }

    harness.press(Key::Enter, Modifiers::COMMAND);
    harness.press(Key::Enter, Modifiers::COMMAND);
    harness.idle_frame();

    assert_eq!(
        harness
            .editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .triggers
            .len(),
        1,
        "double Cmd+Enter must create exactly 1 trigger"
    );

    for _ in 0..20 {
        let add_pos = harness.find_text("+ Add trigger").expect("+ Add trigger");
        harness.click_at(add_pos);
        assert!(harness.editor.ui.trigger_form.is_some());
        harness.press(Key::Escape, Modifiers::NONE);
        harness.idle_frame();
        assert!(harness.editor.ui.trigger_form.is_none());
    }
    assert_eq!(
        harness
            .editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .triggers
            .len(),
        1
    );
}

#[test]
fn d1_10_layer_trigger_ownership_and_hide_layer_none() {
    let mut harness = TestHarness::new(make_base_course());
    harness.editor.selection.slide = "s1".to_owned();
    harness.editor.selection.layer = Some("l1".to_owned());
    open_triggers_tab(&mut harness);

    let new_pos = harness
        .find_text("+ Add trigger")
        .expect("+ Add trigger button");
    harness.click_at(new_pos);
    {
        let form = harness.editor.ui.trigger_form.as_mut().expect("form");
        form.action_tag = ActionTag::HideLayer;
        form.hide_layer_id = None;
    }
    harness.press(Key::Enter, Modifiers::COMMAND);
    harness.idle_frame();

    assert!(harness.editor.ui.trigger_form.is_none());
    assert!(
        harness
            .editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .triggers
            .is_empty()
    );
    let layer_trigs = &harness
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .layer("l1")
        .unwrap()
        .triggers;
    assert_eq!(layer_trigs.len(), 1);
    assert!(matches!(
        layer_trigs[0].action,
        Action::HideLayer { layer: None }
    ));

    // Base layer: HideLayer { layer: None } is refused
    harness.editor.selection.layer = None;
    harness.idle_frame();
    let new_pos = harness
        .find_text("+ Add trigger")
        .expect("+ Add trigger button");
    harness.click_at(new_pos);
    {
        let form = harness.editor.ui.trigger_form.as_mut().expect("form");
        form.action_tag = ActionTag::HideLayer;
        form.hide_layer_id = None;
    }
    harness.press(Key::Enter, Modifiers::COMMAND);
    harness.idle_frame();
    assert!(harness.editor.ui.trigger_form.is_some());
    assert_no_course_errors(&harness.editor.session.course);
}

#[test]
fn d1_11_problem_badge_detection_and_fix_the_first_one() {
    let mut course = make_base_course();
    let q_slide: Slide = serde_json::from_value(serde_json::json!({
        "id": "s-q",
        "title": "Question Slide",
        "duration_ms": 5000,
        "question": {
            "kind": "multiple_choice",
            "prompt": "Pick A",
            "choices": [
                { "id": "c1", "text": "A", "correct": true },
                { "id": "c2", "text": "B", "correct": false }
            ],
            "points": 10
        },
        "triggers": [
            { "when": { "event": "click", "object": "missing-btn" }, "action": { "action": "exit_course" } }
        ]
    }))
    .expect("valid question slide");
    course.scenes[0].slides.push(q_slide);

    let mut harness = TestHarness::new(course);
    harness.editor.selection.slide = "s-q".to_owned();
    open_triggers_tab(&mut harness);
    harness.advance_time(0.5);
    harness.idle_frame();

    let fix_pos = harness
        .find_text("Fix the first one")
        .expect("Fix the first one button");
    harness.click_at(fix_pos);
    harness.idle_frame();

    {
        let form = harness
            .editor
            .ui
            .trigger_form
            .as_mut()
            .expect("form opened by Fix the first one");
        assert_eq!(form.editing_index, Some(0));
        form.event_tag = EventKindTag::TimelineStarts;
        form.action_tag = ActionTag::ExitCourse;
    }
    harness.press(Key::Enter, Modifiers::COMMAND);
    harness.advance_time(0.5);
    harness.idle_frame();

    assert!(harness.find_text("Fix the first one").is_none());
}

#[test]
fn d1_12_undo_deletes_owner_slide_while_trigger_editor_open() {
    let mut harness = TestHarness::new(make_base_course());
    harness.editor.run(
        "slide.add",
        serde_json::json!({"scene": "scene-1", "title": "Temp Slide"}),
    );
    let temp_id = harness.editor.selection.slide.clone();
    harness.editor.run(
        "trigger.add",
        serde_json::json!({
            "slide": temp_id,
            "trigger": make_trig(CourseEvent::TimelineStarts { layer: None }, Action::ExitCourse),
        }),
    );
    open_triggers_tab(&mut harness);

    let row_pos = harness
        .find_text_containing("Exit course")
        .expect("trigger row on temp slide");
    harness.double_click_at(row_pos);
    assert!(harness.editor.ui.trigger_form.is_some());

    harness.editor.run("edit.undo", serde_json::json!({}));
    harness.editor.run("edit.undo", serde_json::json!({}));
    harness.idle_frame();
    harness.idle_frame();

    assert!(
        harness.editor.ui.trigger_form.is_none(),
        "Trigger Editor must close safely when owner disappears"
    );
}

// =========================================================================
// D2: Object States (10 Variations)
// =========================================================================

#[test]
fn d2_1_builtin_preset_states_no_duplicates_and_undo() {
    let mut course = make_base_course();
    course.slide_mut("s1").unwrap().objects[0].states.clear();

    let mut harness = TestHarness::new(course);
    harness.editor.selection.slide = "s1".to_owned();
    harness.editor.selection.object = Some("btn-start".to_owned());
    open_states_panel(&mut harness);

    for label in ["+ Hover", "+ Down", "+ Selected", "+ Visited", "+ Disabled"] {
        let pos = harness.find_text(label).expect(label);
        harness.click_at(pos);
        harness.idle_frame();
    }
    harness.editor.ui.states.show_new_state_input = true;
    harness.idle_frame();
    harness.editor.ui.states.new_state_name = "glow".to_owned();
    harness.idle_frame();
    let add_rect = harness
        .editor
        .ui
        .states
        .add_button_rect
        .expect("Add button rect");
    harness.click_at(add_rect.center());
    harness.idle_frame();

    let states = &harness.editor.session.course.slide("s1").unwrap().objects[0].states;
    assert_eq!(states.len(), 6);
    for expected in ["hover", "down", "selected", "visited", "disabled", "glow"] {
        assert!(states.iter().any(|s| s.name == expected));
    }

    let hover_pos = harness.find_text("+ Hover").expect("+ Hover");
    harness.click_at(hover_pos);
    harness.idle_frame();
    assert_eq!(
        harness.editor.session.course.slide("s1").unwrap().objects[0]
            .states
            .len(),
        6
    );

    for _ in 0..6 {
        undo_key(&mut harness);
    }
    assert!(
        harness.editor.session.course.slide("s1").unwrap().objects[0]
            .states
            .is_empty()
    );
}

#[test]
fn d2_2_custom_state_name_validation_and_enter_errors() {
    let mut harness = TestHarness::new(make_base_course());
    harness.editor.selection.slide = "s1".to_owned();
    harness.editor.selection.object = Some("btn-start".to_owned());
    open_states_panel(&mut harness);

    let name_rect = harness
        .editor
        .ui
        .states
        .new_state_name_rect
        .expect("new_state_name_rect");
    let rev_before = harness.editor.session.revision;

    for bad in ["", "   ", "normal", "Normal", &"x".repeat(200)] {
        harness.editor.ui.last_error = None;
        clear_and_submit_field(&mut harness, name_rect.center(), bad);
        assert_eq!(harness.editor.session.revision, rev_before);
        assert!(
            harness.editor.ui.last_error.is_some(),
            "expected status error for {bad:?}"
        );
    }

    harness.editor.ui.last_error = None;
    harness.editor.ui.states.show_new_state_input = true;
    harness.idle_frame();
    harness.set_field(name_rect.center(), "glow_1");
    harness.idle_frame();
    assert_eq!(harness.editor.session.revision, rev_before + 1);
    assert!(harness.editor.session.course.slide("s1").unwrap().objects[0].has_state("glow_1"));

    harness.editor.ui.last_error = None;
    harness.editor.ui.states.show_new_state_input = true;
    harness.idle_frame();
    harness.set_field(name_rect.center(), "glow_1");
    harness.idle_frame();
    assert_eq!(harness.editor.session.revision, rev_before + 1);
    assert!(harness.editor.ui.last_error.is_some());
}

#[test]
fn d2_3_edit_fill_text_and_src_overrides_across_object_kinds() {
    let mut harness = TestHarness::new(make_base_course());
    harness.editor.selection.slide = "s1".to_owned();

    // 1. Button: edit fill (valid, invalid "#ZZZ", clear) and text
    harness.editor.selection.object = Some("btn-start".to_owned());
    open_states_panel(&mut harness);
    harness.editor.ui.preview_state = Some("visited".to_owned());
    harness.idle_frame();
    let fill_rect = harness
        .editor
        .ui
        .states
        .edit_buffers
        .get("visited")
        .and_then(|b| b.fill_rect)
        .expect("visited fill rect");
    harness.set_field(fill_rect.center(), "#ZZZ");
    harness.idle_frame();
    assert_eq!(
        harness.editor.session.course.slide("s1").unwrap().objects[0].states[0]
            .fill
            .as_ref()
            .map(|c| c.0.as_str()),
        Some("#16A34A")
    );

    harness.set_field(fill_rect.center(), "#112233");
    harness.idle_frame();
    assert_eq!(
        harness.editor.session.course.slide("s1").unwrap().objects[0].states[0]
            .fill
            .as_ref()
            .map(|c| c.0.as_str()),
        Some("#112233")
    );

    clear_and_submit_field(&mut harness, fill_rect.center(), "");
    assert_eq!(
        harness.editor.session.course.slide("s1").unwrap().objects[0].states[0].fill,
        None
    );

    let text_rect = harness
        .editor
        .ui
        .states
        .edit_buffers
        .get("visited")
        .and_then(|b| b.text_rect)
        .expect("visited text rect");
    harness.set_field(text_rect.center(), "Done!");
    harness.idle_frame();
    assert_eq!(
        harness.editor.session.course.slide("s1").unwrap().objects[0].states[0]
            .text
            .as_deref(),
        Some("Done!")
    );

    // 2. Closed Path: fill override
    harness.editor.run(
        "object.add_state",
        serde_json::json!({
            "slide": "s1",
            "object": "path-1",
            "state": StateDef { name: "active".to_owned(), fill: None, text: None, src: None, ..Default::default() }
        }),
    );
    harness.editor.selection.object = Some("path-1".to_owned());
    open_states_panel(&mut harness);
    harness.editor.ui.preview_state = Some("active".to_owned());
    harness.idle_frame();
    let path_fill_rect = harness
        .editor
        .ui
        .states
        .edit_buffers
        .get("active")
        .and_then(|b| b.fill_rect)
        .expect("active fill rect");
    harness.set_field(path_fill_rect.center(), "#445566");
    harness.idle_frame();
    let path_obj = harness
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object("path-1")
        .unwrap()
        .0;
    assert_eq!(
        path_obj.states[0].fill.as_ref().map(|c| c.0.as_str()),
        Some("#445566")
    );

    // 3. Image: src override
    harness.editor.run(
        "object.add_state",
        serde_json::json!({
            "slide": "s1",
            "object": "img-1",
            "state": StateDef { name: "alt".to_owned(), fill: None, text: None, src: None, ..Default::default() }
        }),
    );
    harness.editor.selection.object = Some("img-1".to_owned());
    open_states_panel(&mut harness);
    harness.editor.ui.preview_state = Some("alt".to_owned());
    harness.idle_frame();
    let row_rect = harness
        .editor
        .ui
        .states
        .state_row_rects
        .get("alt")
        .copied()
        .expect("alt card rect");
    assert!(harness.find_text_in("Src", row_rect).is_none());
    assert!(harness.find_text("Choose image…").is_some());
}

#[test]
fn d2_4_preview_state_eye_toggle_and_switch_objects() {
    let mut harness = TestHarness::new(make_base_course());
    harness.editor.selection.slide = "s1".to_owned();
    harness.editor.selection.object = Some("btn-start".to_owned());
    open_states_panel(&mut harness);

    let visited_card = harness
        .editor
        .ui
        .states
        .state_row_rects
        .get("visited")
        .copied()
        .expect("visited card rect");
    harness.click_at(pos2(visited_card.center().x, visited_card.min.y + 30.0));
    assert_eq!(harness.editor.ui.preview_state.as_deref(), Some("visited"));

    let normal_card = harness
        .editor
        .ui
        .states
        .state_row_rects
        .get("normal")
        .copied()
        .expect("normal card rect");
    harness.click_at(pos2(normal_card.center().x, normal_card.min.y + 30.0));
    assert_eq!(harness.editor.ui.preview_state, None);

    harness.click_at(pos2(visited_card.center().x, visited_card.min.y + 30.0));
    assert_eq!(harness.editor.ui.preview_state.as_deref(), Some("visited"));
    harness.editor.selection.object = Some("shape-1".to_owned());
    harness.idle_frame();
    assert_eq!(harness.editor.ui.preview_state, None);
}

#[test]
fn d2_5_bug_d2_deleting_previewed_state_clears_preview_state() {
    let mut harness = TestHarness::new(make_base_course());
    harness.editor.selection.slide = "s1".to_owned();
    harness.editor.selection.object = Some("btn-start".to_owned());
    harness.editor.run(
        "object.add_state",
        serde_json::json!({
            "slide": "s1",
            "object": "btn-start",
            "state": StateDef { name: "custom1".to_owned(), fill: None, text: None, src: None, ..Default::default() }
        }),
    );
    open_states_panel(&mut harness);

    let custom_card = harness
        .editor
        .ui
        .states
        .state_row_rects
        .get("custom1")
        .copied()
        .expect("custom1 card rect");
    harness.click_at(pos2(custom_card.center().x, custom_card.min.y + 30.0));
    assert_eq!(harness.editor.ui.preview_state.as_deref(), Some("custom1"));

    let menu_pos = harness
        .find_text_in("⋯", custom_card)
        .expect("custom1 ⋯ menu");
    harness.click_at(menu_pos);
    harness.idle_frame();
    let del_pos = harness.find_text("Delete").expect("Delete menu item");
    harness.click_at(del_pos);
    harness.idle_frame();

    assert!(!harness.editor.session.course.slide("s1").unwrap().objects[0].has_state("custom1"));
    assert_eq!(
        harness.editor.ui.preview_state, None,
        "preview_state must be cleared when previewed state is deleted"
    );
}

#[test]
fn d2_6_bug_d2_deleting_initial_or_trigger_referenced_state_refused_with_hint() {
    let mut harness = TestHarness::new(make_base_course());
    harness.editor.selection.slide = "s1".to_owned();
    harness.editor.selection.object = Some("btn-start".to_owned());
    harness.editor.run(
        "object.add_state",
        serde_json::json!({
            "slide": "s1",
            "object": "btn-start",
            "state": StateDef { name: "custom1".to_owned(), fill: None, text: None, src: None, ..Default::default() }
        }),
    );
    harness.editor.run(
        "object.update",
        serde_json::json!({
            "slide": "s1",
            "object": "btn-start",
            "initial_state": "custom1",
        }),
    );
    harness.editor.run(
        "trigger.add",
        serde_json::json!({
            "slide": "s1",
            "trigger": make_trig(
                CourseEvent::Click { object: "btn-start".to_owned() },
                Action::ChangeState { object: "btn-start".to_owned(), state: "custom1".to_owned() },
            ),
        }),
    );
    open_states_panel(&mut harness);

    let custom_card = harness
        .editor
        .ui
        .states
        .state_row_rects
        .get("custom1")
        .copied()
        .expect("custom1 card rect");
    let menu_pos = harness
        .find_text_in("⋯", custom_card)
        .expect("custom1 ⋯ menu");
    harness.click_at(menu_pos);
    harness.idle_frame();
    let del_pos = harness.find_text("Delete").expect("Delete menu item");
    harness.click_at(del_pos);
    harness.idle_frame();

    assert!(harness.editor.session.course.slide("s1").unwrap().objects[0].has_state("custom1"));

    harness.editor.run(
        "object.update",
        serde_json::json!({
            "slide": "s1",
            "object": "btn-start",
            "initial_state": "normal",
        }),
    );
    harness.editor.run(
        "trigger.delete",
        serde_json::json!({ "slide": "s1", "index": 0 }),
    );
    open_states_panel(&mut harness);
    let custom_card = harness
        .editor
        .ui
        .states
        .state_row_rects
        .get("custom1")
        .copied()
        .expect("custom1 card rect");
    let menu_pos = harness
        .find_text_in("⋯", custom_card)
        .expect("custom1 ⋯ menu");
    harness.click_at(menu_pos);
    harness.idle_frame();
    let del_pos = harness.find_text("Delete").expect("Delete menu item");
    harness.click_at(del_pos);
    harness.idle_frame();

    assert!(!harness.editor.session.course.slide("s1").unwrap().objects[0].has_state("custom1"));
    undo_key(&mut harness);
    assert!(harness.editor.session.course.slide("s1").unwrap().objects[0].has_state("custom1"));
}

#[test]
fn d2_7_set_initial_state_toggle_and_undo() {
    let mut harness = TestHarness::new(make_base_course());
    harness.editor.selection.slide = "s1".to_owned();
    harness.editor.selection.object = Some("btn-start".to_owned());
    harness.editor.run(
        "object.add_state",
        serde_json::json!({
            "slide": "s1",
            "object": "btn-start",
            "state": StateDef { name: "disabled".to_owned(), fill: None, text: None, src: None, ..Default::default() }
        }),
    );
    open_states_panel(&mut harness);

    let disabled_card = harness
        .editor
        .ui
        .states
        .state_row_rects
        .get("disabled")
        .copied()
        .expect("disabled card rect");
    let menu_pos = harness
        .find_text_in("⋯", disabled_card)
        .expect("disabled ⋯ menu");
    harness.click_at(menu_pos);
    harness.idle_frame();
    let set_init = harness.find_text("Set as initial").expect("Set as initial");
    harness.click_at(set_init);
    harness.idle_frame();
    assert_eq!(
        harness.editor.session.course.slide("s1").unwrap().objects[0].initial_state,
        "disabled"
    );

    let normal_card = harness
        .editor
        .ui
        .states
        .state_row_rects
        .get("normal")
        .copied()
        .expect("normal card rect");
    let normal_menu_pos = harness
        .find_text_in("⋯", normal_card)
        .expect("normal ⋯ menu");
    harness.click_at(normal_menu_pos);
    harness.idle_frame();
    let normal_set_init = harness
        .find_text("Set as initial")
        .expect("Set as initial on normal");
    harness.click_at(normal_set_init);
    harness.idle_frame();
    assert_eq!(
        harness.editor.session.course.slide("s1").unwrap().objects[0].initial_state,
        "normal"
    );

    undo_key(&mut harness);
    assert_eq!(
        harness.editor.session.course.slide("s1").unwrap().objects[0].initial_state,
        "disabled"
    );
    undo_key(&mut harness);
    assert_eq!(
        harness.editor.session.course.slide("s1").unwrap().objects[0].initial_state,
        "normal"
    );
}

#[test]
fn d2_8_mixed_multi_selection_with_states_panel() {
    let mut harness = TestHarness::new(make_base_course());
    harness.editor.selection.slide = "s1".to_owned();
    harness.editor.selection.object = Some("btn-start".to_owned());
    harness.editor.selection.also = vec!["aud-1".to_owned()];
    harness.editor.ui.inspector_tab = InspectorTab::Format;
    harness.idle_frame();

    let report = harness.layout_report(1);
    assert!(report.findings.is_empty(), "{:?}", report.findings);
}

#[test]
fn d2_9_states_on_slide_layer_and_master_slide() {
    let mut harness = TestHarness::new(make_base_course());
    harness.editor.selection.slide = "s1".to_owned();
    harness.editor.selection.layer = Some("l1".to_owned());
    harness.editor.selection.object = Some("layer-txt".to_owned());
    open_states_panel(&mut harness);

    let name_rect = harness
        .editor
        .ui
        .states
        .new_state_name_rect
        .expect("new state name rect");
    harness.set_field(name_rect.center(), "highlight");
    harness.idle_frame();
    assert!(
        harness
            .editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .layer("l1")
            .unwrap()
            .objects[0]
            .has_state("highlight")
    );

    // Master View: States tab is hidden and bottom_tab resets to Timeline.
    harness.editor.ui.master_view = true;
    harness.editor.selection.layer = None;
    harness.editor.selection.slide = "m1-l1".to_owned();
    harness.editor.selection.object = Some("master-shape".to_owned());
    harness.idle_frame();
    assert_eq!(harness.editor.ui.bottom_tab, crate::BottomTab::Timeline);
    assert!(harness.find_text("States").is_none());
    assert_no_course_errors(&harness.editor.session.course);
}

#[test]
fn d2_10_rapid_add_edit_delete_undo_stress_20_states() {
    let mut course = make_base_course();
    course.slide_mut("s1").unwrap().objects[0].states.clear();
    let mut harness = TestHarness::new(course);
    harness.editor.selection.slide = "s1".to_owned();
    harness.editor.selection.object = Some("btn-start".to_owned());
    open_states_panel(&mut harness);

    for i in 0..20 {
        harness.editor.run(
            "object.add_state",
            serde_json::json!({
                "slide": "s1",
                "object": "btn-start",
                "state": StateDef {
                    name: format!("st_{i:02}"),
                    fill: Some(Color("#2563EB".to_owned())),
                    text: None,
                    src: None,
                    ..Default::default()
                }
            }),
        );
    }
    harness.idle_frame();

    let report = harness.layout_report(1);
    assert!(
        report.findings.is_empty(),
        "layout issues with 20 states: {:?}",
        report.findings
    );

    harness.editor.run(
        "object.update_state",
        serde_json::json!({
            "slide": "s1",
            "object": "btn-start",
            "state": StateDef {
                name: "st_10".to_owned(),
                fill: Some(Color("#EF4444".to_owned())),
                text: Some("Edited".to_owned()),
                src: None,
                ..Default::default()
            }
        }),
    );
    harness.editor.run(
        "object.delete_state",
        serde_json::json!({ "slide": "s1", "object": "btn-start", "name": "st_05" }),
    );
    harness.editor.run(
        "object.delete_state",
        serde_json::json!({ "slide": "s1", "object": "btn-start", "name": "st_15" }),
    );

    let after_states = harness.editor.session.course.slide("s1").unwrap().objects[0]
        .states
        .clone();
    assert_eq!(after_states.len(), 18);

    for _ in 0..23 {
        undo_key(&mut harness);
    }
    assert!(
        harness.editor.session.course.slide("s1").unwrap().objects[0]
            .states
            .is_empty()
    );

    for _ in 0..23 {
        redo_key(&mut harness);
    }
    assert_eq!(
        harness.editor.session.course.slide("s1").unwrap().objects[0].states,
        after_states
    );
}
