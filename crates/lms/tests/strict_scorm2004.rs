//! Strict SCORM 2004 bridge integration tests (`LmsBridge` -> `StrictScorm2004`).

use coursecraft_course::{CompletionStatus, Course, QuestionKind, Value, VariableDef};
use coursecraft_lms::{CmiApi, LmsBridge};
use coursecraft_logic::{Answer, Effect, LearnerResponse, Player, PlayerState};
use coursecraft_scorm::Standard;
use coursecraft_strict_lms::{Scorm2004Config, Scorm2004Edition, StrictScorm2004};

struct Strict2004Adapter {
    inner: StrictScorm2004,
}

impl Strict2004Adapter {
    fn new(inner: StrictScorm2004) -> Self {
        Self { inner }
    }
}

impl CmiApi for Strict2004Adapter {
    fn initialize(&mut self) -> bool {
        self.inner.initialize("") == "true"
    }

    fn get_value(&mut self, key: &str) -> String {
        self.inner.get_value(key)
    }

    fn set_value(&mut self, key: &str, value: &str) -> bool {
        self.inner.set_value(key, value) == "true"
    }

    fn commit(&mut self) -> bool {
        self.inner.commit("") == "true"
    }

    fn finish(&mut self) -> bool {
        self.inner.terminate("") == "true"
    }

    fn last_error(&mut self) -> String {
        let code = self.inner.get_last_error();
        let text = self.inner.get_error_string(code);
        format!("{code}: {text}")
    }
}

fn hello_course() -> Course {
    let json = include_str!("../../../examples/hello-course/course.json");
    Course::from_json(json).expect("hello-course parses")
}

#[test]
fn a2_3_completion_and_success_independence() {
    let course = hello_course();
    let state = PlayerState::default();

    // Completion-only leaves success_status untouched ("unknown").
    let mut api = Strict2004Adapter::new(StrictScorm2004::new());
    let mut bridge = LmsBridge::for_standard(Standard::Scorm2004);
    let _ = bridge.start(&mut api, &course);
    let w = bridge.after_input(
        &mut api,
        &course,
        &state,
        &[Effect::Completion {
            status: CompletionStatus::Completed,
        }],
        100,
    );
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(api.inner.get_value("cmi.completion_status"), "completed");
    assert_eq!(api.inner.get_value("cmi.success_status"), "unknown");

    // Passed then Failed: Failed is ignored.
    let w = bridge.after_input(
        &mut api,
        &course,
        &state,
        &[
            Effect::Score {
                raw: 90,
                max: 100,
                percent: 90,
                status: CompletionStatus::Passed,
            },
            Effect::Score {
                raw: 20,
                max: 100,
                percent: 20,
                status: CompletionStatus::Failed,
            },
        ],
        200,
    );
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(api.inner.get_value("cmi.success_status"), "passed");
    assert_eq!(api.inner.get_value("cmi.score.raw"), "90");
    assert!(
        api.inner.violations().is_empty(),
        "{:?}",
        api.inner.violations()
    );
}

#[test]
fn a2_5_iso_session_time_through_bridge() {
    let course = hello_course();
    for ms in [0, 3_661_050, u64::MAX] {
        let mut api = Strict2004Adapter::new(StrictScorm2004::new());
        let mut bridge = LmsBridge::for_standard(Standard::Scorm2004);
        let _ = bridge.start(&mut api, &course);
        let w = bridge.finish_session(&mut api, ms);
        assert!(w.is_empty(), "ms={ms} w={w:?}");
        assert!(
            api.inner.violations().is_empty(),
            "ms={ms} violations={:?}",
            api.inner.violations()
        );
    }
}

#[test]
fn a2_6_suspend_and_location_limits() {
    let mut course = hello_course();
    let var_def: VariableDef = serde_json::from_value(serde_json::json!({
        "name": "big",
        "value": ""
    }))
    .expect("var def");
    course.variables.push(var_def);
    let mut state = Player::new(&course).expect("player").state().clone();
    state
        .variables
        .insert("big".to_owned(), Value::Text("a".repeat(6_000)));

    // 4th Ed. accepts >4096 and <=64000.
    let mut api = Strict2004Adapter::new(StrictScorm2004::new());
    let mut bridge = LmsBridge::for_standard(Standard::Scorm2004);
    let _ = bridge.start(&mut api, &course);
    let w = bridge.after_input(
        &mut api,
        &course,
        &state,
        &[Effect::VariableChanged {
            variable: "big".to_owned(),
            value: Value::Text("a".repeat(6_000)),
        }],
        100,
    );
    assert!(w.is_empty(), "{w:?}");
    assert!(api.inner.get_value("cmi.suspend_data").len() > 4096);

    // 1000-char location succeeds; 1001-char is skipped with warning.
    let w = bridge.after_input(
        &mut api,
        &course,
        &state,
        &[Effect::SlideEntered {
            slide: "s".repeat(1000),
        }],
        200,
    );
    assert!(w.is_empty(), "{w:?}");
    let w = bridge.after_input(
        &mut api,
        &course,
        &state,
        &[Effect::SlideEntered {
            slide: "s".repeat(1001),
        }],
        300,
    );
    assert_eq!(w.len(), 1);
    assert!(
        api.inner.violations().is_empty(),
        "{:?}",
        api.inner.violations()
    );

    // 3rd Ed. rejects >4000.
    let mut api3 = Strict2004Adapter::new(StrictScorm2004::with_config(Scorm2004Config {
        edition: Scorm2004Edition::Third,
        ..Scorm2004Config::default()
    }));
    let mut bridge3 = LmsBridge::for_standard(Standard::Scorm2004);
    let _ = bridge3.start(&mut api3, &course);
    let w3 = bridge3.after_input(
        &mut api3,
        &course,
        &state,
        &[Effect::VariableChanged {
            variable: "big".to_owned(),
            value: Value::Text("a".repeat(6_000)),
        }],
        100,
    );
    assert_eq!(w3.len(), 1);
    assert!(
        api3.inner
            .violations()
            .iter()
            .any(|v| v.error_code.as_deref() == Some("406"))
    );
}

#[test]
fn a2_7_a2_8_all_12_question_kinds_and_readback() {
    let mut course = hello_course();
    let kinds: &[(&str, serde_json::Value, LearnerResponse)] = &[
        (
            "q-tf",
            serde_json::json!({
                "kind": "true_false",
                "prompt": "TF?",
                "choices": [
                    {"id": "true", "text": "True", "correct": true},
                    {"id": "false", "text": "False", "correct": false}
                ],
                "points": 10
            }),
            LearnerResponse::Choices(vec!["true".into()]),
        ),
        (
            "q-mc",
            serde_json::json!({
                "kind": "multiple_choice",
                "prompt": "MC?",
                "choices": [
                    {"id": "a", "text": "A", "correct": true},
                    {"id": "b", "text": "B", "correct": false}
                ],
                "points": 10
            }),
            LearnerResponse::Choices(vec!["a".into()]),
        ),
        (
            "q-mr",
            serde_json::json!({
                "kind": "multiple_response",
                "prompt": "MR?",
                "choices": [
                    {"id": "a", "text": "A", "correct": true},
                    {"id": "b", "text": "B", "correct": true}
                ],
                "points": 10
            }),
            LearnerResponse::Choices(vec!["a".into(), "b".into()]),
        ),
        (
            "q-fill",
            serde_json::json!({
                "kind": "fill_in",
                "prompt": "Fill?",
                "choices": [],
                "accepted": ["Rust"],
                "case_sensitive": true,
                "points": 10
            }),
            LearnerResponse::Text("Rust".into()),
        ),
        (
            "q-seq",
            serde_json::json!({
                "kind": "sequence",
                "prompt": "Seq?",
                "choices": [
                    {"id": "s1", "text": "One"},
                    {"id": "s2", "text": "Two"}
                ],
                "points": 10
            }),
            LearnerResponse::Sequence(vec!["s1".into(), "s2".into()]),
        ),
        (
            "q-match",
            serde_json::json!({
                "kind": "matching",
                "prompt": "Match?",
                "choices": [],
                "pairs": [
                    {"id": "p1", "left": "L1", "right": "R1"},
                    {"id": "p2", "left": "L2", "right": "R2"}
                ],
                "points": 10
            }),
            LearnerResponse::Pairs(vec![("p1".into(), "p1".into()), ("p2".into(), "p2".into())]),
        ),
        (
            "q-hotspot",
            serde_json::json!({
                "kind": "hotspot",
                "prompt": "Hot?",
                "choices": [],
                "regions": [
                    {"id": "r1", "shape": "rect", "x": 0.0, "y": 0.0, "w": 10.0, "h": 10.0, "correct": true}
                ],
                "points": 10
            }),
            LearnerResponse::Regions(vec!["r1".into()]),
        ),
        (
            "q-drag",
            serde_json::json!({
                "kind": "drag_drop",
                "prompt": "Drag?",
                "choices": [],
                "drag_items": [
                    {"item": "d1", "target": "t1"}
                ],
                "points": 10
            }),
            LearnerResponse::Pairs(vec![("d1".into(), "t1".into())]),
        ),
        (
            "q-freeform",
            serde_json::json!({
                "kind": "freeform",
                "prompt": "Free?",
                "choices": [],
                "points": 10,
                "correct_when": { "compare": { "variable": "tabs_seen", "op": "ge", "value": 0 } }
            }),
            LearnerResponse::Text("correct".into()),
        ),
        (
            "q-likert",
            serde_json::json!({
                "kind": "likert",
                "prompt": "Likert?",
                "choices": [
                    {"id": "1", "text": "Low"},
                    {"id": "5", "text": "High"}
                ]
            }),
            LearnerResponse::Choices(vec!["5".into()]),
        ),
        (
            "q-short",
            serde_json::json!({
                "kind": "short_answer",
                "prompt": "Short?",
                "choices": []
            }),
            LearnerResponse::Text("ok".into()),
        ),
        (
            "q-essay",
            serde_json::json!({
                "kind": "essay",
                "prompt": "Essay?",
                "choices": []
            }),
            LearnerResponse::Text("long answer".into()),
        ),
    ];

    let mut api = Strict2004Adapter::new(StrictScorm2004::new());
    let mut bridge = LmsBridge::for_standard(Standard::Scorm2004);
    let _ = bridge.start(&mut api, &course);

    for (idx, (id, q_json, resp)) in kinds.iter().enumerate() {
        let mut slide = course.scenes[0].slides[0].clone();
        slide.id = (*id).to_owned();
        slide.question = Some(serde_json::from_value(q_json.clone()).expect("question"));
        course.scenes[0].slides.push(slide);

        let mut state = PlayerState {
            last_response: Some(resp.clone()),
            ..PlayerState::default()
        };
        let is_survey = matches!(
            course
                .play_slide(id)
                .and_then(|s| s.question.as_ref())
                .map(|q| q.kind),
            Some(QuestionKind::Likert | QuestionKind::ShortAnswer | QuestionKind::Essay)
        );
        state.answers.insert(
            (*id).to_owned(),
            Answer {
                attempts: 1,
                correct: !is_survey,
                points: if is_survey { 0 } else { 10 },
            },
        );

        let w = bridge.after_input(
            &mut api,
            &course,
            &state,
            &[Effect::QuestionAnswered {
                slide: (*id).to_owned(),
                correct: !is_survey,
                points: if is_survey { 0 } else { 10 },
                attempts_left: None,
            }],
            (idx as u64 + 1) * 100,
        );
        assert!(w.is_empty(), "slide {id} warnings: {w:?}");
    }

    assert_eq!(api.inner.get_value("cmi.interactions._count"), "12");
    assert_eq!(api.inner.get_value("cmi.interactions.0.id"), "q-tf_a1");
    assert_eq!(api.inner.get_value("cmi.interactions.0.type"), "true-false");
    assert_eq!(
        api.inner.get_value("cmi.interactions.0.learner_response"),
        "true"
    );
    assert_eq!(api.inner.get_value("cmi.interactions.8.type"), "other");
    assert_eq!(api.inner.get_value("cmi.interactions.9.result"), "neutral");
    assert!(
        api.inner.violations().is_empty(),
        "{:?}",
        api.inner.violations()
    );
}

#[test]
fn a2_9_exit_vocabulary() {
    let course = hello_course();
    let state = PlayerState::default();

    let mut api = Strict2004Adapter::new(StrictScorm2004::new());
    let mut bridge = LmsBridge::for_standard(Standard::Scorm2004);
    let _ = bridge.start(&mut api, &course);
    let _ = bridge.finish_session(&mut api, 100);
    assert_eq!(api.inner.snapshot()["exit"], "suspend");

    let mut api2 = Strict2004Adapter::new(StrictScorm2004::new());
    let mut bridge2 = LmsBridge::for_standard(Standard::Scorm2004);
    let _ = bridge2.start(&mut api2, &course);
    let _ = bridge2.after_input(
        &mut api2,
        &course,
        &state,
        &[
            Effect::Completion {
                status: CompletionStatus::Completed,
            },
            Effect::Exit,
        ],
        200,
    );
    assert_eq!(api2.inner.snapshot()["exit"], "normal");
    assert!(
        api.inner.violations().is_empty(),
        "{:?}",
        api.inner.violations()
    );
    assert!(
        api2.inner.violations().is_empty(),
        "{:?}",
        api2.inner.violations()
    );
}

#[test]
fn a2_11_description_and_text_escaping() {
    let mut course = hello_course();
    let mut slide = course.scenes[0].slides[0].clone();
    slide.id = "q-esc".to_owned();
    slide.question = Some(
        serde_json::from_value(serde_json::json!({
            "kind": "fill_in",
            "prompt": "Prompt a[,]b[.]c\u{0000}\n ".to_owned() + &"p".repeat(300),
            "choices": [],
            "accepted": ["a[,]b[.]c\u{0000}\n ".to_owned() + &"r".repeat(300)],
            "points": 10
        }))
        .expect("fill_in"),
    );
    course.scenes[0].slides.push(slide);

    let mut state = PlayerState {
        last_response: Some(LearnerResponse::Text(
            "a[,]b[.]c\u{0000}\n ".to_owned() + &"r".repeat(300),
        )),
        ..PlayerState::default()
    };
    state.answers.insert(
        "q-esc".to_owned(),
        Answer {
            attempts: 1,
            correct: true,
            points: 10,
        },
    );

    let mut api = Strict2004Adapter::new(StrictScorm2004::new());
    let mut bridge = LmsBridge::for_standard(Standard::Scorm2004);
    let _ = bridge.start(&mut api, &course);
    let w = bridge.after_input(
        &mut api,
        &course,
        &state,
        &[Effect::QuestionAnswered {
            slide: "q-esc".to_owned(),
            correct: true,
            points: 10,
            attempts_left: None,
        }],
        100,
    );
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(
        api.inner
            .get_value("cmi.interactions.0.description")
            .chars()
            .count(),
        250
    );
    assert!(
        api.inner.violations().is_empty(),
        "{:?}",
        api.inner.violations()
    );
}
