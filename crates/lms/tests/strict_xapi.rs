//! Strict xAPI 1.0.3 / cmi5 bridge integration tests (`XapiBridge` -> `StrictLrs`).

use std::collections::VecDeque;

use coursecraft_course::{CompletionStatus, Course, QuestionKind, Value as VarValue, VariableDef};
use coursecraft_lms::{
    HANDSHAKE_TIMEOUT_MS, LrsApi, LrsMethod, LrsRequest, LrsResponse, XapiBridge,
};
use coursecraft_logic::{Answer, Effect, LearnerResponse, Player, PlayerState};
use coursecraft_strict_lms::{Flavor, LaunchSpec, StrictLrs, build_cmi5_launch_data};
use coursecraft_xapi::LaunchParams;
use serde_json::{Value, json};

struct StrictLrsAdapter {
    lrs: StrictLrs,
    inbox: VecDeque<LrsResponse>,
    requests: Vec<LrsRequest>,
    uuid_counter: u64,
}

impl StrictLrsAdapter {
    fn new(lrs: StrictLrs) -> Self {
        Self {
            lrs,
            inbox: VecDeque::new(),
            requests: Vec::new(),
            uuid_counter: 0,
        }
    }
}

impl LrsApi for StrictLrsAdapter {
    fn send(&mut self, request: LrsRequest) {
        self.requests.push(request.clone());
        let method = match request.method {
            LrsMethod::Get => "GET",
            LrsMethod::Post => "POST",
            LrsMethod::Put => "PUT",
        };
        let body = request.body.as_deref().unwrap_or("").as_bytes();
        let (path, query) = request.url.split_once('?').unwrap_or((&request.url, ""));
        let resp = if let Some(fetch_id) = path
            .rsplit("/fetch/")
            .next()
            .filter(|_| path.contains("/fetch/"))
        {
            self.lrs.handle_fetch(method, fetch_id, &request.headers)
        } else if path.ends_with("/statements") {
            self.lrs.handle_statements(method, &request.headers, body)
        } else if path.ends_with("/activities/state") {
            self.lrs.handle_state(method, query, &request.headers, body)
        } else {
            coursecraft_strict_lms::LrsResponse {
                status: 404,
                headers: vec![],
                body: b"not found".to_vec(),
            }
        };
        self.inbox.push_back(LrsResponse {
            id: request.id,
            status: resp.status,
            body: String::from_utf8_lossy(&resp.body).into_owned(),
        });
    }

    fn poll(&mut self) -> Vec<LrsResponse> {
        self.inbox.drain(..).collect()
    }

    fn now_iso(&mut self) -> String {
        "2026-11-21T10:00:00Z".to_owned()
    }

    fn new_uuid(&mut self) -> String {
        self.uuid_counter += 1;
        format!("00000000-0000-4000-8000-{:012x}", self.uuid_counter)
    }
}

fn hello_course() -> Course {
    let json = include_str!("../../../examples/hello-course/course.json");
    Course::from_json(json).expect("hello-course parses")
}

fn cmi5_actor() -> Value {
    json!({
        "objectType": "Agent",
        "account": { "homePage": "https://lms.example", "name": "alice" },
        "name": "Alice"
    })
}

fn register_cmi5(
    lrs: &mut StrictLrs,
    launch_mode: &str,
    mastery_score: Option<f64>,
) -> LaunchParams {
    let fetch_id = "f1";
    let activity_id = "https://lms.example/activities/hello";
    let registration = "11111111-1111-4111-8111-111111111111";
    let launch_doc =
        build_cmi5_launch_data(activity_id, "sess-1", launch_mode, mastery_score, None);
    lrs.register_launch(
        LaunchSpec {
            flavor: Flavor::Cmi5,
            activity_id: activity_id.to_owned(),
            actor: cmi5_actor(),
            registration: Some(registration.to_owned()),
            launch_mode: launch_mode.to_owned(),
            mastery_score,
            session_id: "sess-1".to_owned(),
            grouping_ids: vec![activity_id.to_owned()],
            fetch_id: Some(fetch_id.to_owned()),
            auth_token: Some("tok-1".to_owned()),
        },
        Some(launch_doc),
    );
    let query = format!(
        "endpoint=https://lrs.example/xapi&fetch=https://lrs.example/fetch/{fetch_id}&actor=%7B%22objectType%22%3A%22Agent%22%2C%22account%22%3A%7B%22homePage%22%3A%22https%3A%2F%2Flms.example%22%2C%22name%22%3A%22alice%22%7D%2C%22name%22%3A%22Alice%22%7D&registration={registration}&activityId=https%3A%2F%2Flms.example%2Factivities%2Fhello"
    );
    LaunchParams::parse(&query, Flavor::Cmi5).expect("cmi5 launch params")
}

fn pump_ready(bridge: &mut XapiBridge, api: &mut StrictLrsAdapter, course: &Course) -> Vec<String> {
    bridge.start(api);
    let mut warnings = Vec::new();
    for step in 1..=5 {
        let tick = bridge.tick(api, course, step * 100);
        warnings.extend(tick.warnings);
        if tick.plan.is_some() {
            break;
        }
    }
    warnings
}

#[test]
fn a3_1_full_cmi5_launch_pass_terminate() {
    let mut course = hello_course();
    let mut lrs = StrictLrs::new();
    let params = register_cmi5(&mut lrs, "Normal", Some(0.8));
    let mut api = StrictLrsAdapter::new(lrs);
    let mut bridge = XapiBridge::new(params, Flavor::Cmi5, &course);

    let w = pump_ready(&mut bridge, &mut api, &course);
    assert!(w.is_empty(), "{w:?}");

    let kinds: &[(&str, Value, LearnerResponse)] = &[
        (
            "q-tf",
            json!({"kind":"true_false","prompt":"TF","choices":[{"id":"true","text":"T","correct":true},{"id":"false","text":"F","correct":false}],"points":10}),
            LearnerResponse::Choices(vec!["true".into()]),
        ),
        (
            "q-mc",
            json!({"kind":"multiple_choice","prompt":"MC","choices":[{"id":"a","text":"A","correct":true}],"points":10}),
            LearnerResponse::Choices(vec!["a".into()]),
        ),
        (
            "q-mr",
            json!({"kind":"multiple_response","prompt":"MR","choices":[{"id":"a","text":"A","correct":true},{"id":"b","text":"B","correct":true}],"points":10}),
            LearnerResponse::Choices(vec!["a".into(), "b".into()]),
        ),
        (
            "q-fill",
            json!({"kind":"fill_in","prompt":"Fill","choices":[],"accepted":["Rust"],"points":10}),
            LearnerResponse::Text("Rust".into()),
        ),
        (
            "q-seq",
            json!({"kind":"sequence","prompt":"Seq","choices":[{"id":"s1","text":"1"},{"id":"s2","text":"2"}],"points":10}),
            LearnerResponse::Sequence(vec!["s1".into(), "s2".into()]),
        ),
        (
            "q-match",
            json!({"kind":"matching","prompt":"Match","choices":[],"pairs":[{"id":"p1","left":"L","right":"R"}],"points":10}),
            LearnerResponse::Pairs(vec![("p1".into(), "p1".into())]),
        ),
        (
            "q-hotspot",
            json!({"kind":"hotspot","prompt":"Hot","choices":[],"regions":[{"id":"r1","shape":"rect","x":0.0,"y":0.0,"w":5.0,"h":5.0,"correct":true}],"points":10}),
            LearnerResponse::Regions(vec!["r1".into()]),
        ),
        (
            "q-drag",
            json!({"kind":"drag_drop","prompt":"Drag","choices":[],"drag_items":[{"item":"d1","target":"t1"}],"points":10}),
            LearnerResponse::Pairs(vec![("d1".into(), "t1".into())]),
        ),
        (
            "q-freeform",
            json!({"kind":"freeform","prompt":"Free","choices":[],"points":10,"correct_when":{"compare":{"variable":"tabs_seen","op":"ge","value":0}}}),
            LearnerResponse::Text("correct".into()),
        ),
        (
            "q-likert",
            json!({"kind":"likert","prompt":"Likert","choices":[{"id":"1","text":"1"}]}),
            LearnerResponse::Choices(vec!["1".into()]),
        ),
        (
            "q-short",
            json!({"kind":"short_answer","prompt":"Short","choices":[]}),
            LearnerResponse::Text("ok".into()),
        ),
        (
            "q-essay",
            json!({"kind":"essay","prompt":"Essay","choices":[]}),
            LearnerResponse::Text("essay".into()),
        ),
    ];

    let mut state = PlayerState::default();
    for (idx, (id, q_json, resp)) in kinds.iter().enumerate() {
        let mut slide = course.scenes[0].slides[0].clone();
        slide.id = (*id).to_owned();
        slide.question = Some(serde_json::from_value(q_json.clone()).expect("q"));
        course.scenes[0].slides.push(slide);
        state.last_response = Some(resp.clone());
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
            1000 + idx as u64 * 100,
            1000 + idx as u64 * 100,
        );
        assert!(w.is_empty(), "{w:?}");
    }

    let w = bridge.after_input(
        &mut api,
        &course,
        &state,
        &[
            Effect::Score {
                raw: 100,
                max: 100,
                percent: 100,
                status: CompletionStatus::Passed,
            },
            Effect::Exit,
        ],
        3000,
        3000,
    );
    assert!(w.is_empty(), "{w:?}");

    let verbs: Vec<&str> = api
        .lrs
        .statements()
        .iter()
        .filter_map(|s| s["verb"]["id"].as_str())
        .filter_map(|v| v.rsplit('/').next())
        .collect();
    assert_eq!(verbs.first().copied(), Some("initialized"));
    assert_eq!(verbs.iter().filter(|&&v| v == "answered").count(), 12);
    assert_eq!(
        verbs[verbs.len() - 3..],
        ["passed", "completed", "terminated"]
    );
    assert!(
        api.lrs.violations().is_empty(),
        "{:?}",
        api.lrs.violations()
    );
}

#[test]
fn a3_4_fetch_single_use_and_empty_auth_token() {
    let course = hello_course();
    let mut lrs = StrictLrs::new();
    let params = register_cmi5(&mut lrs, "Normal", None);
    let mut api = StrictLrsAdapter::new(lrs);
    let mut bridge = XapiBridge::new(params.clone(), Flavor::Cmi5, &course);
    let _ = pump_ready(&mut bridge, &mut api, &course);

    // Fetch request had zero Authorization header.
    let fetch_req = api
        .requests
        .iter()
        .find(|r| r.url.contains("/fetch/"))
        .expect("fetch request");
    assert!(
        !fetch_req
            .headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("Authorization"))
    );

    // Reusing the same fetch URL returns error-code "1" -> reporting_lost() == true.
    let mut bridge2 = XapiBridge::new(params, Flavor::Cmi5, &course);
    let w2 = pump_ready(&mut bridge2, &mut api, &course);
    assert!(!w2.is_empty());
    assert!(bridge2.reporting_lost());

    // Empty auth-token in fetch response turns off reporting (BUG-A10).
    struct EmptyTokenApi {
        inbox: VecDeque<LrsResponse>,
    }
    impl LrsApi for EmptyTokenApi {
        fn send(&mut self, request: LrsRequest) {
            self.inbox.push_back(LrsResponse {
                id: request.id,
                status: 200,
                body: r#"{"auth-token":""}"#.to_owned(),
            });
        }
        fn poll(&mut self) -> Vec<LrsResponse> {
            self.inbox.drain(..).collect()
        }
        fn now_iso(&mut self) -> String {
            "2026-11-21T10:00:00Z".to_owned()
        }
        fn new_uuid(&mut self) -> String {
            "00000000-0000-4000-8000-000000000099".to_owned()
        }
    }
    let params_empty = LaunchParams::parse(
        "endpoint=https://lrs.example/xapi&fetch=https://lrs.example/fetch/f-empty&actor=%7B%22mbox%22%3A%22mailto%3Aa%40b.c%22%7D&registration=22222222-2222-4222-8222-222222222222&activityId=https%3A%2F%2Flms.example%2Factivities%2Fhello",
        Flavor::Cmi5,
    )
    .expect("params");
    let mut bridge_empty = XapiBridge::new(params_empty, Flavor::Cmi5, &course);
    let mut api_empty = EmptyTokenApi {
        inbox: VecDeque::new(),
    };
    bridge_empty.start(&mut api_empty);
    let tick = bridge_empty.tick(&mut api_empty, &course, 100);
    assert!(!tick.warnings.is_empty());
    assert!(bridge_empty.reporting_lost());
}

#[test]
fn a3_5_browse_and_review_modes() {
    let course = hello_course();
    let state = PlayerState::default();
    for mode in ["Browse", "Review"] {
        let mut lrs = StrictLrs::new();
        let params = register_cmi5(&mut lrs, mode, Some(0.8));
        let mut api = StrictLrsAdapter::new(lrs);
        let mut bridge = XapiBridge::new(params, Flavor::Cmi5, &course);
        let w = pump_ready(&mut bridge, &mut api, &course);
        assert!(w.is_empty(), "{w:?}");
        let w = bridge.after_input(
            &mut api,
            &course,
            &state,
            &[
                Effect::QuestionAnswered {
                    slide: "s-quiz".into(),
                    correct: true,
                    points: 100,
                    attempts_left: None,
                },
                Effect::Score {
                    raw: 100,
                    max: 100,
                    percent: 100,
                    status: CompletionStatus::Passed,
                },
                Effect::Exit,
            ],
            1000,
            1000,
        );
        assert!(w.is_empty(), "{w:?}");
        assert!(!bridge.reporting_lost());
        let verbs: Vec<&str> = api
            .lrs
            .statements()
            .iter()
            .filter_map(|s| s["verb"]["id"].as_str())
            .filter_map(|v| v.rsplit('/').next())
            .collect();
        assert_eq!(verbs, ["initialized", "terminated"]);
        assert!(
            api.lrs.violations().is_empty(),
            "{:?}",
            api.lrs.violations()
        );
    }
}

#[test]
fn a3_6_hostile_launch_data_and_404() {
    let course = hello_course();
    let mut lrs = StrictLrs::new();
    let _params = register_cmi5(&mut lrs, "Normal", None);
    let mut api = StrictLrsAdapter::new(lrs);
    api.lrs.add_token("Basic dG9rLTE=".to_owned());
    let put_resp = api.lrs.handle_state(
        "PUT",
        "stateId=LMS.LaunchData&activityId=https://lms.example/activities/hello&agent=%7B%22mbox%22%3A%22mailto%3Aa%40b.c%22%7D",
        &[
            ("X-Experience-API-Version".into(), "1.0.3".into()),
            ("Authorization".into(), "Basic dG9rLTE=".into()),
        ],
        b"{}",
    );
    assert_eq!(put_resp.status, 403);

    let mut lrs2 = StrictLrs::new();
    lrs2.register_launch(
        LaunchSpec {
            flavor: Flavor::Cmi5,
            activity_id: "https://lms.example/activities/hello".into(),
            actor: cmi5_actor(),
            registration: Some("33333333-3333-4333-8333-333333333333".into()),
            launch_mode: "Normal".into(),
            mastery_score: None,
            session_id: "sess-hostile".into(),
            grouping_ids: vec![],
            fetch_id: Some("f-hostile".into()),
            auth_token: Some("tok-h".into()),
        },
        Some(json!({
            "masteryScore": 1.5,
            "returnURL": "javascript:alert(1)",
            "contextTemplate": {
                "extensions": {
                    "https://w3id.org/xapi/cmi5/context/extensions/sessionid": "sess-hostile"
                }
            }
        })),
    );
    let mut api2 = StrictLrsAdapter::new(lrs2);
    let p2 = LaunchParams::parse(
        "endpoint=https://lrs.example/xapi&fetch=https://lrs.example/fetch/f-hostile&actor=%7B%22objectType%22%3A%22Agent%22%2C%22account%22%3A%7B%22homePage%22%3A%22https%3A%2F%2Flms.example%22%2C%22name%22%3A%22alice%22%7D%7D&registration=33333333-3333-4333-8333-333333333333&activityId=https%3A%2F%2Flms.example%2Factivities%2Fhello",
        Flavor::Cmi5,
    )
    .expect("p2");
    let mut bridge2 = XapiBridge::new(p2, Flavor::Cmi5, &course);
    let w2 = pump_ready(&mut bridge2, &mut api2, &course);
    assert!(w2.iter().any(|m| m.contains("masteryScore")), "{w2:?}");
    assert!(w2.iter().any(|m| m.contains("returnURL")), "{w2:?}");
    assert!(bridge2.return_url().is_none());
}

#[test]
fn a3_9_plain_xapi_relaunch_same_registration_and_none() {
    let course = hello_course();
    let state = PlayerState::default();
    let mut lrs = StrictLrs::new();
    lrs.add_token("Basic dXNlcjpwYXNz".to_owned());

    let mut uuid_counter = 0;
    for (reg, sid) in [
        (Some("44444444-4444-4444-8444-444444444444"), "s1"),
        (Some("44444444-4444-4444-8444-444444444444"), "s2"),
        (None, "s3"),
    ] {
        let auth_hdr = format!("Basic tok-{sid}");
        lrs.register_launch(
            LaunchSpec {
                flavor: Flavor::Xapi,
                activity_id: "urn:coursecraft:activity:hello-course".to_owned(),
                actor: json!({"mbox": "mailto:a@b.c"}),
                registration: reg.map(str::to_owned),
                launch_mode: "Normal".to_owned(),
                mastery_score: None,
                session_id: sid.to_owned(),
                grouping_ids: vec![],
                fetch_id: None,
                auth_token: Some(auth_hdr.clone()),
            },
            None,
        );
        let mut q = format!(
            "endpoint=https://lrs.example/xapi&auth=Basic%20tok-{sid}&actor=%7B%22mbox%22%3A%22mailto%3Aa%40b.c%22%7D"
        );
        if let Some(r) = reg {
            q.push_str(&format!("&registration={r}"));
        }
        let params = LaunchParams::parse(&q, Flavor::Xapi).expect("xapi params");
        let mut api = StrictLrsAdapter::new(lrs);
        api.uuid_counter = uuid_counter;
        let mut bridge = XapiBridge::new(params, Flavor::Xapi, &course);
        let w = pump_ready(&mut bridge, &mut api, &course);
        assert!(w.is_empty(), "{sid}: {w:?}");
        let w = bridge.after_input(&mut api, &course, &state, &[Effect::Exit], 500, 500);
        assert!(w.is_empty(), "{sid}: {w:?}");
        assert!(
            api.lrs.violations().is_empty(),
            "{sid}: {:?}",
            api.lrs.violations()
        );
        uuid_counter = api.uuid_counter;
        lrs = api.lrs;
    }
}

#[test]
fn a3_10_keepalive_progress_cap_and_401_stop() {
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
        .insert("big".to_owned(), VarValue::Text("x".repeat(70_000)));

    let mut lrs = StrictLrs::new();
    let params = register_cmi5(&mut lrs, "Normal", None);
    let mut api = StrictLrsAdapter::new(lrs);
    let mut bridge = XapiBridge::new(params, Flavor::Cmi5, &course);
    let _ = pump_ready(&mut bridge, &mut api, &course);

    // Oversized keepalive progress is skipped with a warning (BUG-A9).
    let w = bridge.after_input(
        &mut api,
        &course,
        &state,
        &[
            Effect::VariableChanged {
                variable: "big".to_owned(),
                value: VarValue::Text("x".repeat(70_000)),
            },
            Effect::Exit,
        ],
        1000,
        1000,
    );
    assert!(w.iter().any(|m| m.contains("keepalive progress")), "{w:?}");
    assert!(
        api.lrs.violations().is_empty(),
        "{:?}",
        api.lrs.violations()
    );

    // Handshake timeout turns off reporting cleanly.
    let mut lrs2 = StrictLrs::new();
    let params2 = register_cmi5(&mut lrs2, "Normal", None);
    let mut bridge2 = XapiBridge::new(params2, Flavor::Cmi5, &course);
    struct DropApi;
    impl LrsApi for DropApi {
        fn send(&mut self, _: LrsRequest) {}
        fn poll(&mut self) -> Vec<LrsResponse> {
            Vec::new()
        }
        fn now_iso(&mut self) -> String {
            "2026-11-21T10:00:00Z".into()
        }
        fn new_uuid(&mut self) -> String {
            "00000000-0000-4000-8000-000000000001".into()
        }
    }
    let mut drop_api = DropApi;
    bridge2.start(&mut drop_api);
    let _ = bridge2.tick(&mut drop_api, &course, 0);
    let tick = bridge2.tick(&mut drop_api, &course, HANDSHAKE_TIMEOUT_MS + 1);
    assert!(tick.plan.is_some());
    assert!(bridge2.reporting_lost());
}

#[test]
fn a3_11_launch_params_hostless_url_and_redaction() {
    for bad_endpoint in ["http://", "https://", "https:///path"] {
        let q = format!(
            "endpoint={bad_endpoint}&auth=secret-tok&actor=%7B%22mbox%22%3A%22mailto%3Aa%40b.c%22%7D"
        );
        assert!(
            LaunchParams::parse(&q, Flavor::Xapi).is_err(),
            "should reject {bad_endpoint}"
        );
    }
    let q_bad_fetch = "endpoint=https://lrs.example/xapi&fetch=https://&actor=%7B%22mbox%22%3A%22mailto%3Aa%40b.c%22%7D&registration=r1&activityId=a1";
    assert!(LaunchParams::parse(q_bad_fetch, Flavor::Cmi5).is_err());

    let q_ok = "endpoint=https://lrs.example/xapi&auth=super-secret-auth&actor=%7B%22mbox%22%3A%5B%22mailto%3Aa%40b.c%22%5D%7D";
    let params = LaunchParams::parse(q_ok, Flavor::Xapi).expect("ok");
    let dbg = format!("{params:?}");
    assert!(!dbg.contains("super-secret-auth"));
}

#[test]
fn a3_12_lightbox_durability_parity() {
    let course = hello_course();
    let mut state = Player::new(&course).expect("player").state().clone();
    state.visited.insert("s-quiz".to_owned());

    let mut lrs = StrictLrs::new();
    let params = register_cmi5(&mut lrs, "Normal", None);
    let mut api = StrictLrsAdapter::new(lrs);
    let mut bridge = XapiBridge::new(params, Flavor::Cmi5, &course);
    let _ = pump_ready(&mut bridge, &mut api, &course);

    let w = bridge.after_input(
        &mut api,
        &course,
        &state,
        &[
            Effect::LightboxOpened {
                slide: "s-quiz".to_owned(),
            },
            Effect::LightboxClosed {
                slide: "s-quiz".to_owned(),
            },
            Effect::Exit,
        ],
        1000,
        1000,
    );
    assert!(w.is_empty(), "{w:?}");
    let saved = api
        .lrs
        .get_state(
            "coursecraft.progress",
            "https://lms.example/activities/hello",
            &cmi5_actor(),
            Some("11111111-1111-4111-8111-111111111111"),
        )
        .expect("progress saved");
    let doc: Value = serde_json::from_slice(saved).expect("json");
    let suspend = doc["suspend"].as_str().expect("suspend str");
    let decoded = coursecraft_logic::decode_suspend(&course, suspend).expect("decode");
    assert!(decoded.visited.contains("s-quiz"));
    assert!(
        api.lrs.violations().is_empty(),
        "{:?}",
        api.lrs.violations()
    );
}
