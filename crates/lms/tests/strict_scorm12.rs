//! Strict SCORM 1.2 bridge integration tests (`LmsBridge` -> `StrictScorm12`).

use coursecraft_course::{CompletionStatus, Course, QuestionKind, Value, VariableDef};
use coursecraft_lms::{CmiApi, LmsBridge, format_session_time};
use coursecraft_logic::{Answer, Effect, Input, LearnerResponse, Player, PlayerState};
use coursecraft_scorm::Standard;
use coursecraft_strict_lms::{Scorm12Config, StrictScorm12};

struct Strict12Adapter {
    inner: StrictScorm12,
    reject_interaction_id: bool,
}

impl Strict12Adapter {
    fn new(inner: StrictScorm12) -> Self {
        Self {
            inner,
            reject_interaction_id: false,
        }
    }
}

impl CmiApi for Strict12Adapter {
    fn initialize(&mut self) -> bool {
        self.inner.lms_initialize("") == "true"
    }

    fn get_value(&mut self, key: &str) -> String {
        self.inner.lms_get_value(key)
    }

    fn set_value(&mut self, key: &str, value: &str) -> bool {
        if self.reject_interaction_id
            && key.starts_with("cmi.interactions.")
            && key.ends_with(".id")
        {
            return false;
        }
        self.inner.lms_set_value(key, value) == "true"
    }

    fn commit(&mut self) -> bool {
        self.inner.lms_commit("") == "true"
    }

    fn finish(&mut self) -> bool {
        self.inner.lms_finish("") == "true"
    }

    fn last_error(&mut self) -> String {
        let code = self.inner.lms_get_last_error();
        let text = self.inner.lms_get_error_string(code);
        format!("{code}: {text}")
    }
}

fn hello_course() -> Course {
    let json = include_str!("../../../examples/hello-course/course.json");
    Course::from_json(json).expect("hello-course parses")
}

#[test]
fn a1_5_session_time_saturation_and_bridge() {
    let mut strict = StrictScorm12::new();
    assert_eq!(strict.lms_initialize(""), "true");
    for ms in [0, 10, 59_990, 3_661_050, 35_999_999_990, u64::MAX] {
        let formatted = format_session_time(ms);
        assert_eq!(
            strict.lms_set_value("cmi.core.session_time", &formatted),
            "true",
            "ms={ms} formatted={formatted}"
        );
    }
    assert_eq!(format_session_time(0), "0000:00:00.00");
    assert_eq!(format_session_time(u64::MAX), "9999:59:59.99");
    assert!(strict.violations().is_empty(), "{:?}", strict.violations());
}

#[test]
fn a1_6_location_255_boundary() {
    let course = hello_course();
    let state = PlayerState::default();

    // 255-char ASCII and UTF-8 succeed with 0 violations.
    let mut api = Strict12Adapter::new(StrictScorm12::new());
    let mut bridge = LmsBridge::new();
    let (_, w) = bridge.start(&mut api, &course);
    assert!(w.is_empty(), "{w:?}");

    let ascii_255 = "a".repeat(255);
    let w = bridge.after_input(
        &mut api,
        &course,
        &state,
        &[Effect::SlideEntered {
            slide: ascii_255.clone(),
        }],
        100,
    );
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(
        api.inner.lms_get_value("cmi.core.lesson_location"),
        ascii_255
    );

    let utf8_255 = "é".repeat(255);
    let w = bridge.after_input(
        &mut api,
        &course,
        &state,
        &[Effect::SlideEntered {
            slide: utf8_255.clone(),
        }],
        200,
    );
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(
        api.inner.lms_get_value("cmi.core.lesson_location"),
        utf8_255
    );

    // 256-char slide ID is skipped by LmsBridge with a warning and 0 LMS violations.
    let too_long = "x".repeat(256);
    let w = bridge.after_input(
        &mut api,
        &course,
        &state,
        &[Effect::SlideEntered { slide: too_long }],
        300,
    );
    assert_eq!(w.len(), 1);
    assert!(w[0].contains("255"), "{w:?}");
    assert!(
        api.inner.violations().is_empty(),
        "{:?}",
        api.inner.violations()
    );
}

#[test]
fn a1_7_suspend_data_4096_boundary() {
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
        .insert("big".to_owned(), Value::Text("z".repeat(5000)));

    let mut api = Strict12Adapter::new(StrictScorm12::new());
    let mut bridge = LmsBridge::new();
    let _ = bridge.start(&mut api, &course);
    let w = bridge.after_input(
        &mut api,
        &course,
        &state,
        &[Effect::VariableChanged {
            variable: "big".to_owned(),
            value: Value::Text("z".repeat(5000)),
        }],
        100,
    );
    assert!(w.iter().any(|msg| msg.contains("4096")), "{w:?}");
    assert!(
        api.inner.violations().is_empty(),
        "{:?}",
        api.inner.violations()
    );
}

#[test]
fn a1_8_decimal_score_resume_high_water() {
    let course = hello_course();
    let state = PlayerState::default();
    let mut strict = StrictScorm12::new();
    strict.seed_status("passed", Some("85.0"));

    let mut api = Strict12Adapter::new(strict);
    let mut bridge = LmsBridge::new();
    let (_, w) = bridge.start(&mut api, &course);
    assert!(w.is_empty(), "{w:?}");

    let w = bridge.after_input(
        &mut api,
        &course,
        &state,
        &[Effect::Score {
            raw: 60,
            max: 100,
            percent: 60,
            status: CompletionStatus::Passed,
        }],
        500,
    );
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(api.inner.lms_get_value("cmi.core.score.raw"), "85.0");
    assert!(
        api.inner.violations().is_empty(),
        "{:?}",
        api.inner.violations()
    );
}

#[test]
fn a1_9_all_12_question_kinds_through_bridge() {
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

    let mut api = Strict12Adapter::new(StrictScorm12::new());
    let mut bridge = LmsBridge::new();
    let (_, w) = bridge.start(&mut api, &course);
    assert!(w.is_empty(), "{w:?}");

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

    assert_eq!(api.inner.lms_get_value("cmi.interactions._count"), "12");
    let snap = api.inner.snapshot();
    assert_eq!(snap["interactions"][8]["kind"], "performance");
    assert!(
        api.inner.violations().is_empty(),
        "violations: {:?}",
        api.inner.violations()
    );
}

#[test]
fn a1_10_failed_id_write_and_255_id_limit() {
    let mut course = hello_course();
    let long_id = "q_".to_owned() + &"a".repeat(260);
    let mut slide = course.play_slide("s-quiz").expect("s-quiz").clone();
    slide.id = long_id.clone();
    course.scenes[0].slides.push(slide);

    let mut state = PlayerState {
        last_response: Some(LearnerResponse::Choices(vec!["a".into()])),
        ..PlayerState::default()
    };
    state.answers.insert(
        long_id.clone(),
        Answer {
            attempts: 1,
            correct: true,
            points: 100,
        },
    );
    state.answers.insert(
        "s-quiz".to_owned(),
        Answer {
            attempts: 1,
            correct: true,
            points: 100,
        },
    );

    // 260-char slide ID is truncated to <=255 chars and succeeds with 0 violations.
    let mut api = Strict12Adapter::new(StrictScorm12::new());
    let mut bridge = LmsBridge::new();
    let _ = bridge.start(&mut api, &course);
    let w = bridge.after_input(
        &mut api,
        &course,
        &state,
        &[Effect::QuestionAnswered {
            slide: long_id.clone(),
            correct: true,
            points: 100,
            attempts_left: None,
        }],
        100,
    );
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(api.inner.lms_get_value("cmi.interactions._count"), "1");
    assert!(
        api.inner.violations().is_empty(),
        "{:?}",
        api.inner.violations()
    );

    // When `.id` write fails, `interaction_index` does NOT increment and no orphan child fields are written.
    let mut api2 = Strict12Adapter::new(StrictScorm12::new());
    api2.reject_interaction_id = true;
    let mut bridge2 = LmsBridge::new();
    let _ = bridge2.start(&mut api2, &course);
    let w2 = bridge2.after_input(
        &mut api2,
        &course,
        &state,
        &[Effect::QuestionAnswered {
            slide: "s-quiz".to_owned(),
            correct: true,
            points: 100,
            attempts_left: None,
        }],
        100,
    );
    assert_eq!(w2.len(), 1, "{w2:?}");
    assert_eq!(api2.inner.lms_get_value("cmi.interactions._count"), "0");
    assert!(
        !api2
            .inner
            .calls()
            .iter()
            .any(|c| c.method == "LMSSetValue" && c.args[0] == "cmi.interactions.0.type")
    );
}

#[test]
fn a1_11_total_time_and_exit_relaunch() {
    let course = hello_course();
    let mut player = Player::new(&course).expect("player");
    let mut api = Strict12Adapter::new(StrictScorm12::new());
    let mut bridge = LmsBridge::new();
    let _ = bridge.start(&mut api, &course);

    let effects = player.handle(&Input::Start);
    let _ = bridge.after_input(&mut api, &course, player.state(), &effects, 1000);
    let w = bridge.finish_session(&mut api, 90_500);
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(api.inner.snapshot()["exit"], "suspend");

    api.inner.relaunch();
    let (plan2, w2) = bridge.start(&mut api, &course);
    assert!(w2.is_empty(), "{w2:?}");
    assert!(plan2.resume.is_some());
    assert_eq!(api.inner.lms_get_value("cmi.core.entry"), "resume");
    assert_eq!(
        api.inner.lms_get_value("cmi.core.total_time"),
        "0000:01:30.50"
    );
    assert!(
        api.inner.violations().is_empty(),
        "{:?}",
        api.inner.violations()
    );
}

#[test]
fn a1_12_empty_initial_status_and_failed_init() {
    let course = hello_course();
    let config = Scorm12Config {
        initial_lesson_status: String::new(),
        ..Scorm12Config::default()
    };
    let mut api = Strict12Adapter::new(StrictScorm12::with_config(config));
    let mut bridge = LmsBridge::for_standard(Standard::Scorm12);
    let (_, w) = bridge.start(&mut api, &course);
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(
        api.inner.lms_get_value("cmi.core.lesson_status"),
        "incomplete"
    );
    assert!(
        api.inner.violations().is_empty(),
        "{:?}",
        api.inner.violations()
    );

    let fail_cfg = Scorm12Config {
        fail_initialize: Some("101"),
        ..Scorm12Config::default()
    };
    let mut api_fail = Strict12Adapter::new(StrictScorm12::with_config(fail_cfg));
    let mut bridge_fail = LmsBridge::new();
    let (_, w_fail) = bridge_fail.start(&mut api_fail, &course);
    assert_eq!(w_fail.len(), 1);
    assert!(!bridge_fail.is_active());
    let state = PlayerState::default();
    let _ = bridge_fail.after_input(&mut api_fail, &course, &state, &[Effect::Exit], 100);
    assert_eq!(api_fail.inner.calls().len(), 1);
}
