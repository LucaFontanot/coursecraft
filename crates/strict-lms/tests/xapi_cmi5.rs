//! Direct strict-LRS conformance tests for xAPI 1.0.3 and cmi5 (Cases A3.1–A3.9).

use coursecraft_strict_lms::{Flavor, LaunchSpec, StrictLrs, build_cmi5_launch_data};
use coursecraft_xapi::{AnsweredInput, StatementContext};
use serde_json::{Value, json};

const ACTIVITY: &str = "urn:coursecraft:mock:activity:hello";
const REG: &str = "urn:coursecraft:mock:registration:cmi5-hello";
const SESSION_1: &str = "mock-session-1";
const SESSION_2: &str = "mock-session-2";
const TS: &str = "2026-11-21T12:00:00Z";

fn actor() -> Value {
    json!({
        "objectType": "Agent",
        "name": "Learner, Test",
        "account": {
            "homePage": "http://127.0.0.1",
            "name": "learner-1"
        }
    })
}

fn hdrs(auth: &str) -> Vec<(String, String)> {
    vec![
        ("X-Experience-API-Version".to_owned(), "1.0.3".to_owned()),
        ("Authorization".to_owned(), auth.to_owned()),
        ("Content-Type".to_owned(), "application/json".to_owned()),
    ]
}

fn setup_cmi5(mode: &str, mastery: Option<f64>) -> (StrictLrs, String, StatementContext) {
    let mut lrs = StrictLrs::new();
    let launch_doc = build_cmi5_launch_data(ACTIVITY, SESSION_1, mode, mastery, None);
    lrs.register_launch(
        LaunchSpec {
            flavor: Flavor::Cmi5,
            activity_id: ACTIVITY.to_owned(),
            actor: actor(),
            registration: Some(REG.to_owned()),
            launch_mode: mode.to_owned(),
            mastery_score: mastery,
            session_id: SESSION_1.to_owned(),
            grouping_ids: vec![ACTIVITY.to_owned()],
            fetch_id: Some("f1".to_owned()),
            auth_token: None,
        },
        Some(launch_doc.clone()),
    );
    let fetch_resp = lrs.handle_fetch("POST", "f1", &[]);
    assert_eq!(fetch_resp.status, 200);
    let fetch_json: Value = serde_json::from_slice(&fetch_resp.body).expect("fetch json");
    let token = fetch_json["auth-token"].as_str().expect("auth-token");
    let auth = format!("Basic {token}");

    let ctx = StatementContext {
        flavor: Flavor::Cmi5,
        actor: actor(),
        activity_id: ACTIVITY.to_owned(),
        activity_name: "Hello".to_owned(),
        registration: Some(REG.to_owned()),
        context_template: launch_doc.get("contextTemplate").cloned(),
        mastery_score: mastery,
    };
    (lrs, auth, ctx)
}

fn post_stmt(lrs: &mut StrictLrs, auth: &str, stmt: &Value) -> u16 {
    let body = serde_json::to_vec(&json!([stmt])).expect("json");
    lrs.handle_statements("POST", &hdrs(auth), &body).status
}

#[test]
fn a3_1_full_cmi5_pass_sequence_has_zero_violations() {
    let (mut lrs, auth, ctx) = setup_cmi5("Normal", Some(0.8));

    let init = ctx.initialized("11111111-1111-4111-8111-111111111111", TS);
    let ans = ctx.answered(
        "22222222-2222-4222-8222-222222222222",
        TS,
        &AnsweredInput {
            question_id: "q1".to_owned(),
            attempt: 1,
            interaction_type: "choice".to_owned(),
            description: "Pick A".to_owned(),
            response: "a".to_owned(),
            correct_patterns: vec!["a".to_owned()],
            success: true,
            points: 10,
            awarded: 10,
            survey: false,
        },
    );
    let passed = ctx.passed("33333333-3333-4333-8333-333333333333", TS, 0.9, "PT10S");
    let completed = ctx.completed("44444444-4444-4444-8444-444444444444", TS, "PT10S");
    let term = ctx.terminated("55555555-5555-4555-8555-555555555555", TS, "PT12S");

    for s in [&init, &ans, &passed, &completed, &term] {
        assert_eq!(post_stmt(&mut lrs, &auth, s), 200);
    }
    assert!(
        lrs.violations().is_empty(),
        "unexpected violations: {:?}",
        lrs.violations()
    );
    assert!(lrs.warnings().is_empty());
    assert_eq!(lrs.statements().len(), 5);
}

#[test]
fn a3_2_cmi5_verb_order_and_passed_after_failed_forbidden() {
    // 1. Statement before initialized + duplicate initialized + statement after terminated.
    let (mut lrs, auth, ctx) = setup_cmi5("Normal", None);
    let completed = ctx.completed("11111111-1111-4111-8111-111111111111", TS, "PT1S");
    post_stmt(&mut lrs, &auth, &completed);
    assert!(
        lrs.violations()
            .iter()
            .any(|v| v.rule == "CMI5-SEQ-NOT-INITIALIZED")
    );

    let init = ctx.initialized("22222222-2222-4222-8222-222222222222", TS);
    post_stmt(&mut lrs, &auth, &init);
    assert!(
        lrs.violations()
            .iter()
            .any(|v| v.rule == "CMI5-SEQ-INIT-NOT-FIRST")
    );

    let init2 = ctx.initialized("33333333-3333-4333-8333-333333333333", TS);
    post_stmt(&mut lrs, &auth, &init2);
    assert!(
        lrs.violations()
            .iter()
            .any(|v| v.rule == "CMI5-SEQ-DUP-INITIALIZED")
    );

    let term = ctx.terminated("44444444-4444-4444-8444-444444444444", TS, "PT5S");
    post_stmt(&mut lrs, &auth, &term);
    let after = ctx.completed("55555555-5555-4555-8555-555555555555", TS, "PT6S");
    post_stmt(&mut lrs, &auth, &after);
    assert!(
        lrs.violations()
            .iter()
            .any(|v| v.rule == "CMI5-SEQ-AFTER-TERMINATED")
    );

    // 2. passed after failed in same session (BUG-A13).
    let (mut lrs2, auth2, ctx2) = setup_cmi5("Normal", Some(0.8));
    post_stmt(
        &mut lrs2,
        &auth2,
        &ctx2.initialized("11111111-1111-4111-8111-111111111111", TS),
    );
    post_stmt(
        &mut lrs2,
        &auth2,
        &ctx2.failed("22222222-2222-4222-8222-222222222222", TS, 0.4, "PT1S"),
    );
    post_stmt(
        &mut lrs2,
        &auth2,
        &ctx2.passed("33333333-3333-4333-8333-333333333333", TS, 0.9, "PT2S"),
    );
    assert!(
        lrs2.violations()
            .iter()
            .any(|v| v.rule == "CMI5-9.3.1-PASS-AFTER-FAIL")
    );

    // 3. failed after passed across relaunch + second completed on same registration.
    let (mut lrs3, auth3, ctx3) = setup_cmi5("Normal", Some(0.8));
    post_stmt(
        &mut lrs3,
        &auth3,
        &ctx3.initialized("11111111-1111-4111-8111-111111111111", TS),
    );
    post_stmt(
        &mut lrs3,
        &auth3,
        &ctx3.passed("22222222-2222-4222-8222-222222222222", TS, 0.9, "PT1S"),
    );
    post_stmt(
        &mut lrs3,
        &auth3,
        &ctx3.completed("33333333-3333-4333-8333-333333333333", TS, "PT1S"),
    );
    post_stmt(
        &mut lrs3,
        &auth3,
        &ctx3.terminated("44444444-4444-4444-8444-444444444444", TS, "PT2S"),
    );

    // Relaunch on same registration with SESSION_2.
    let launch2 = build_cmi5_launch_data(ACTIVITY, SESSION_2, "Normal", Some(0.8), None);
    lrs3.register_launch(
        LaunchSpec {
            flavor: Flavor::Cmi5,
            activity_id: ACTIVITY.to_owned(),
            actor: actor(),
            registration: Some(REG.to_owned()),
            launch_mode: "Normal".to_owned(),
            mastery_score: Some(0.8),
            session_id: SESSION_2.to_owned(),
            grouping_ids: vec![ACTIVITY.to_owned()],
            fetch_id: Some("f2".to_owned()),
            auth_token: None,
        },
        Some(launch2.clone()),
    );
    let f2: Value = serde_json::from_slice(&lrs3.handle_fetch("POST", "f2", &[]).body).expect("f2");
    let auth3_b = format!("Basic {}", f2["auth-token"].as_str().expect("tok"));
    let ctx3_b = StatementContext {
        context_template: launch2.get("contextTemplate").cloned(),
        ..ctx3
    };
    post_stmt(
        &mut lrs3,
        &auth3_b,
        &ctx3_b.initialized("55555555-5555-4555-8555-555555555555", TS),
    );
    post_stmt(
        &mut lrs3,
        &auth3_b,
        &ctx3_b.failed("66666666-6666-4666-8666-666666666666", TS, 0.3, "PT1S"),
    );
    post_stmt(
        &mut lrs3,
        &auth3_b,
        &ctx3_b.completed("77777777-7777-4777-8777-777777777777", TS, "PT1S"),
    );
    assert!(
        lrs3.violations()
            .iter()
            .any(|v| v.rule == "CMI5-REG-FAIL-AFTER-PASS")
    );
    assert!(
        lrs3.violations()
            .iter()
            .any(|v| v.rule == "CMI5-REG-DUP-COMPLETED")
    );
}

#[test]
fn a3_3_cmi5_result_exclusions_and_mastery_agreement() {
    let (mut lrs, auth, ctx) = setup_cmi5("Normal", Some(0.8));

    let mut bad_init = ctx.initialized("11111111-1111-4111-8111-111111111111", TS);
    bad_init["result"] = json!({ "score": { "scaled": 1.0 } });
    post_stmt(&mut lrs, &auth, &bad_init);
    assert!(
        lrs.violations()
            .iter()
            .any(|v| v.rule == "CMI5-RESULT-INITIALIZED")
    );

    let mut bad_comp = ctx.completed("22222222-2222-4222-8222-222222222222", TS, "PT1S");
    bad_comp["result"]["score"] = json!({ "scaled": 0.9 });
    post_stmt(&mut lrs, &auth, &bad_comp);
    assert!(
        lrs.violations()
            .iter()
            .any(|v| v.rule == "CMI5-RESULT-COMPLETED")
    );

    let bad_pass = ctx.passed("33333333-3333-4333-8333-333333333333", TS, 0.5, "PT1S");
    post_stmt(&mut lrs, &auth, &bad_pass);
    assert!(
        lrs.violations()
            .iter()
            .any(|v| v.rule == "CMI5-RESULT-MASTERY-DISAGREES")
    );

    let mut bad_term = ctx.terminated("44444444-4444-4444-8444-444444444444", TS, "PT1S");
    bad_term["result"] = json!({});
    post_stmt(&mut lrs, &auth, &bad_term);
    assert!(
        lrs.violations()
            .iter()
            .any(|v| v.rule == "CMI5-RESULT-TERMINATED")
    );
}

#[test]
fn a3_4_fetch_single_use_and_auth_warning() {
    let mut lrs = StrictLrs::new();
    lrs.register_launch(
        LaunchSpec {
            flavor: Flavor::Cmi5,
            activity_id: ACTIVITY.to_owned(),
            actor: actor(),
            registration: Some(REG.to_owned()),
            launch_mode: "Normal".to_owned(),
            mastery_score: None,
            session_id: SESSION_1.to_owned(),
            grouping_ids: vec![ACTIVITY.to_owned()],
            fetch_id: Some("f-once".to_owned()),
            auth_token: None,
        },
        None,
    );

    let first = lrs.handle_fetch(
        "POST",
        "f-once",
        &[("Authorization".to_owned(), "Basic extra".to_owned())],
    );
    assert_eq!(first.status, 200);
    assert!(
        lrs.warnings()
            .iter()
            .any(|w| w.rule == "CMI5-FETCH-AUTH-HEADER")
    );
    assert!(lrs.violations().is_empty());

    let second = lrs.handle_fetch("POST", "f-once", &[]);
    assert_eq!(second.status, 200);
    let body: Value = serde_json::from_slice(&second.body).expect("json");
    assert_eq!(body["error-code"], "1");
    assert!(
        lrs.violations()
            .iter()
            .any(|v| v.rule == "CMI5-FETCH-REUSED")
    );
}

#[test]
fn a3_5_browse_and_review_modes_forbid_moveon_verbs_only() {
    for mode in ["Browse", "Review"] {
        let (mut lrs, auth, ctx) = setup_cmi5(mode, None);
        post_stmt(
            &mut lrs,
            &auth,
            &ctx.initialized("11111111-1111-4111-8111-111111111111", TS),
        );
        // answered is allowed in Browse/Review per cmi5 spec.
        post_stmt(
            &mut lrs,
            &auth,
            &ctx.answered(
                "22222222-2222-4222-8222-222222222222",
                TS,
                &AnsweredInput {
                    question_id: "q1".to_owned(),
                    attempt: 1,
                    interaction_type: "likert".to_owned(),
                    description: "Rate".to_owned(),
                    response: "5".to_owned(),
                    correct_patterns: vec![],
                    success: true,
                    points: 0,
                    awarded: 0,
                    survey: true,
                },
            ),
        );
        assert!(lrs.violations().is_empty());

        post_stmt(
            &mut lrs,
            &auth,
            &ctx.completed("33333333-3333-4333-8333-333333333333", TS, "PT1S"),
        );
        assert!(
            lrs.violations()
                .iter()
                .any(|v| v.rule == "CMI5-MODE-DISALLOWED"),
            "mode={mode}"
        );
    }
}

#[test]
fn a3_6_and_a3_7_state_api_ifi_matching_and_launch_data_protection() {
    let (mut lrs, auth, _) = setup_cmi5("Normal", None);
    let q_base = format!(
        "stateId=coursecraft.progress&activityId={ACTIVITY}&agent=%7B%22account%22%3A%7B%22homePage%22%3A%22http%3A%2F%2F127.0.0.1%22%2C%22name%22%3A%22learner-1%22%7D%2C%22name%22%3A%22Learner%2C%20Test%22%2C%22objectType%22%3A%22Agent%22%7D&registration={REG}"
    );
    let put = lrs.handle_state("PUT", &q_base, &hdrs(&auth), br#"{"slide":"s2"}"#);
    assert_eq!(put.status, 204);

    // Reordered JSON keys + omitted optional `name` and `objectType` (same canonical IFI!).
    let q_reordered = format!(
        "stateId=coursecraft.progress&activityId={ACTIVITY}&agent=%7B%22account%22%3A%7B%22name%22%3A%22learner-1%22%2C%22homePage%22%3A%22http%3A%2F%2F127.0.0.1%22%7D%7D&registration={REG}"
    );
    let get = lrs.handle_state("GET", &q_reordered, &hdrs(&auth), &[]);
    assert_eq!(get.status, 200);
    assert_eq!(get.body, br#"{"slide":"s2"}"#);

    // LMS.LaunchData is strictly read-only.
    let q_ld = format!(
        "stateId=LMS.LaunchData&activityId={ACTIVITY}&agent=%7B%22account%22%3A%7B%22homePage%22%3A%22http%3A%2F%2F127.0.0.1%22%2C%22name%22%3A%22learner-1%22%7D%7D&registration={REG}"
    );
    let put_ld = lrs.handle_state("PUT", &q_ld, &hdrs(&auth), b"{}");
    assert_eq!(put_ld.status, 403);
    assert!(
        lrs.violations()
            .iter()
            .any(|v| v.rule == "CMI5-STATE-LAUNCHDATA-READONLY")
    );
}

#[test]
fn a3_8_xapi_headers_uuid_timestamp_and_409_dedup() {
    let mut lrs = StrictLrs::new();
    let auth = "Basic tok-xapi";
    lrs.add_token(auth.to_owned());

    // Missing X-Experience-API-Version -> 400
    let r1 = lrs.handle_statements(
        "POST",
        &[
            ("Authorization".to_owned(), auth.to_owned()),
            ("Content-Type".to_owned(), "application/json".to_owned()),
        ],
        b"[]",
    );
    assert_eq!(r1.status, 400);

    // Missing Content-Type -> 400
    let r2 = lrs.handle_statements(
        "POST",
        &[
            ("X-Experience-API-Version".to_owned(), "1.0.3".to_owned()),
            ("Authorization".to_owned(), auth.to_owned()),
        ],
        b"[]",
    );
    assert_eq!(r2.status, 400);

    // Bad UUID -> 400
    let bad_uuid = json!({
        "id": "not-a-uuid",
        "timestamp": TS,
        "actor": actor(),
        "verb": { "id": "http://adlnet.gov/expapi/verbs/initialized", "display": { "en-US": "initialized" } },
        "object": { "id": ACTIVITY }
    });
    assert_eq!(post_stmt(&mut lrs, auth, &bad_uuid), 400);

    // Valid statement + identical retry (200) + mutated retry (409)
    let id = "11111111-1111-4111-8111-111111111111";
    let stmt = json!({
        "id": id,
        "timestamp": TS,
        "actor": actor(),
        "verb": { "id": "http://adlnet.gov/expapi/verbs/initialized", "display": { "en-US": "initialized" } },
        "object": { "id": ACTIVITY }
    });
    assert_eq!(post_stmt(&mut lrs, auth, &stmt), 200);
    assert_eq!(post_stmt(&mut lrs, auth, &stmt), 200);

    let mut mutated = stmt.clone();
    mutated["timestamp"] = json!("2026-11-21T12:05:00Z");
    assert_eq!(post_stmt(&mut lrs, auth, &mutated), 409);
    assert!(
        lrs.violations()
            .iter()
            .any(|v| v.rule == "XAPI-409-DUPLICATE-ID")
    );
}

#[test]
fn a3_9_plain_xapi_relaunch_same_registration_no_collision() {
    let mut lrs = StrictLrs::new();
    let reg = "urn:coursecraft:mock:registration:xapi-hello";
    for (sess, auth, u1, u2) in [
        (
            "xapi-s1",
            "Basic x1",
            "11111111-1111-4111-8111-111111111111",
            "22222222-2222-4222-8222-222222222222",
        ),
        (
            "xapi-s2",
            "Basic x2",
            "33333333-3333-4333-8333-333333333333",
            "44444444-4444-4444-8444-444444444444",
        ),
    ] {
        lrs.register_launch(
            LaunchSpec {
                flavor: Flavor::Xapi,
                activity_id: ACTIVITY.to_owned(),
                actor: actor(),
                registration: Some(reg.to_owned()),
                launch_mode: "Normal".to_owned(),
                mastery_score: None,
                session_id: sess.to_owned(),
                grouping_ids: vec![],
                fetch_id: None,
                auth_token: Some(auth.to_owned()),
            },
            None,
        );
        let ctx = StatementContext {
            flavor: Flavor::Xapi,
            actor: actor(),
            activity_id: ACTIVITY.to_owned(),
            activity_name: "Hello".to_owned(),
            registration: Some(reg.to_owned()),
            context_template: None,
            mastery_score: None,
        };
        assert_eq!(post_stmt(&mut lrs, auth, &ctx.initialized(u1, TS)), 200);
        assert_eq!(
            post_stmt(&mut lrs, auth, &ctx.terminated(u2, TS, "PT5S")),
            200
        );
    }
    assert!(
        lrs.violations().is_empty(),
        "plain xAPI relaunch had violations: {:?}",
        lrs.violations()
    );
}
