//! Tests for the mock LMS/LRS request handler.

use std::path::PathBuf;

use serde_json::{Map, Value, json};

use super::*;

const CMI5_CATEGORY: &str = "https://w3id.org/xapi/cmi5/context/categories/cmi5";
const MOVEON_CATEGORY: &str = "https://w3id.org/xapi/cmi5/context/categories/moveon";
const SESSION_ID_EXTENSION: &str = "https://w3id.org/xapi/cmi5/context/extensions/sessionid";
const MASTERY_EXTENSION: &str = "https://w3id.org/xapi/cmi5/context/extensions/masteryscore";
const VERB_BASE: &str = "http://adlnet.gov/expapi/verbs/";
const LAUNCH_DATA: &str = "LMS.LaunchData";
const TS: &str = "2026-11-21T12:00:00Z";

fn empty_store() -> LrsStore {
    LrsStore::new(8091, PathBuf::from("target/mock-lrs"))
}

fn uuid_for(tag: &str) -> String {
    let mut hex = String::with_capacity(12);
    for b in tag.bytes().take(6) {
        hex.push_str(&format!("{b:02x}"));
    }
    while hex.len() < 12 {
        hex.push('0');
    }
    format!("00000000-0000-4000-8000-{hex}")
}

fn request(method: &str, path: &str, query: &str) -> Request {
    Request {
        method: method.to_string(),
        path: path.to_string(),
        query: query.to_string(),
        headers: Vec::new(),
        body: Vec::new(),
    }
}

fn launch_request(
    flavor: &str,
    course: &str,
    mode: &str,
    mastery: Option<&str>,
    want_return: bool,
) -> Request {
    let mut query = format!("flavor={flavor}&course={course}&mode={mode}");
    if let Some(mastery) = mastery {
        query.push_str("&mastery=");
        query.push_str(mastery);
    }
    if want_return {
        query.push_str("&return=1");
    }
    request("POST", "/mock/launch", &query)
}

fn xapi_request(method: &str, path: &str, query: &str, body: Vec<u8>) -> Request {
    Request {
        method: method.to_string(),
        path: path.to_string(),
        query: query.to_string(),
        headers: vec![
            ("X-Experience-API-Version".to_string(), "1.0.3".to_string()),
            ("Authorization".to_string(), "Basic mock-xapi".to_string()),
            ("Content-Type".to_string(), "application/json".to_string()),
        ],
        body,
    }
}

fn first_registration(store: &LrsStore) -> String {
    store
        .registrations
        .values()
        .next()
        .cloned()
        .expect("a registration")
}

fn post_statements(store: &mut LrsStore, statements: &[Value]) -> Response {
    let body = serde_json::to_vec(&Value::Array(statements.to_vec())).unwrap_or_default();
    handle(store, &xapi_request("POST", "/xapi/statements", "", body))
}

fn cmi5_statement(verb: &str, registration: &str, session: &str, id: &str) -> Value {
    let mut result = Map::new();
    if matches!(verb, "completed" | "passed" | "failed" | "terminated") {
        result.insert("duration".into(), json!("PT1M"));
    }
    if verb == "completed" {
        result.insert("completion".into(), json!(true));
    }
    if verb == "passed" {
        result.insert("success".into(), json!(true));
        result.insert("score".into(), json!({ "scaled": 0.9 }));
    }
    if verb == "failed" {
        result.insert("success".into(), json!(false));
        result.insert("score".into(), json!({ "scaled": 0.4 }));
    }
    let mut categories = Vec::new();
    if matches!(
        verb,
        "initialized" | "completed" | "passed" | "failed" | "terminated"
    ) {
        categories.push(json!({ "id": CMI5_CATEGORY }));
    }
    if matches!(verb, "completed" | "passed" | "failed") {
        categories.push(json!({ "id": MOVEON_CATEGORY }));
    }
    let mut extensions = Map::new();
    extensions.insert(SESSION_ID_EXTENSION.into(), json!(session));
    let mut object = json!({ "id": "urn:coursecraft:mock:activity:hello" });
    if verb == "answered" {
        object = json!({
            "id": "urn:coursecraft:mock:activity:hello/questions/q1",
            "definition": {
                "type": "http://adlnet.gov/expapi/activities/cmi.interaction",
                "interactionType": "choice",
                "correctResponsesPattern": ["a"]
            }
        });
    }
    json!({
        "id": uuid_for(id),
        "timestamp": TS,
        "actor": actor_json(),
        "verb": { "id": format!("{VERB_BASE}{verb}"), "display": { "en-US": verb } },
        "object": object,
        "result": Value::Object(result),
        "context": {
            "registration": registration,
            "extensions": Value::Object(extensions),
            "contextActivities": { "category": categories },
        },
    })
}

fn add_extension(mut statement: Value, key: &str, value: Value) -> Value {
    if let Some(extensions) = statement
        .pointer_mut("/context/extensions")
        .and_then(Value::as_object_mut)
    {
        extensions.insert(key.to_string(), value);
    }
    statement
}

fn category_ids_of(statement: &mut Value) -> &mut Vec<Value> {
    statement
        .pointer_mut("/context/contextActivities/category")
        .and_then(Value::as_array_mut)
        .expect("category array")
}

fn cmi5_registration(store: &mut LrsStore, mastery: Option<&str>) -> String {
    handle(
        store,
        &launch_request("cmi5", "hello", "Normal", mastery, false),
    );
    first_registration(store)
}

fn has_violation(store: &LrsStore, needle: &str) -> bool {
    store
        .strict
        .violations()
        .iter()
        .any(|v| v.message.contains(needle) || v.rule.contains(needle))
}

#[test]
fn launch_returns_cmi5_url_and_writes_launch_data() {
    let mut store = empty_store();
    let response = handle(
        &mut store,
        &launch_request("cmi5", "hello", "Normal", Some("0.8"), true),
    );
    assert_eq!(response.status, 200);
    let body: Value = serde_json::from_slice(&response.body).expect("launch json");
    let url = body.get("url").and_then(Value::as_str).expect("url");
    assert!(url.contains("/cmi5/hello/index.html?"));
    assert!(url.contains("endpoint="));
    assert!(url.contains("fetch="));
    assert!(url.contains("activityId="));

    let reg = first_registration(&store);
    let doc_bytes = store
        .strict
        .get_state(
            LAUNCH_DATA,
            "urn:coursecraft:mock:activity:hello",
            &actor_json(),
            Some(&reg),
        )
        .expect("LMS.LaunchData");
    let document: Value = serde_json::from_slice(doc_bytes).expect("launch data json");
    assert_eq!(
        document.get("launchMode").and_then(Value::as_str),
        Some("Normal")
    );
    assert_eq!(
        document.get("masteryScore").and_then(Value::as_f64),
        Some(0.8)
    );
    assert!(
        document
            .get("returnURL")
            .and_then(Value::as_str)
            .is_some_and(|url| url.ends_with("/mock/returned"))
    );
    let template = document.get("contextTemplate").expect("template");
    assert!(
        template
            .pointer("/extensions/https:~1~1w3id.org~1xapi~1cmi5~1context~1extensions~1sessionid")
            .is_some()
    );
    assert!(
        template
            .pointer("/contextActivities/grouping/0/id")
            .is_some()
    );
}

#[test]
fn launch_returns_xapi_url() {
    let mut store = empty_store();
    let response = handle(
        &mut store,
        &launch_request("xapi", "showcase", "Normal", None, false),
    );
    assert_eq!(response.status, 200);
    let body: Value = serde_json::from_slice(&response.body).expect("launch json");
    let url = body.get("url").and_then(Value::as_str).expect("url");
    assert!(url.contains("/xapi/showcase/index.html?"));
    assert!(url.contains("auth=Basic%20mock-xapi"));
}

#[test]
fn fetch_token_once_then_reuse_error() {
    let mut store = empty_store();
    let resp = handle(
        &mut store,
        &launch_request("cmi5", "hello", "Normal", None, false),
    );
    let url = serde_json::from_slice::<Value>(&resp.body)
        .expect("json")
        .get("url")
        .and_then(Value::as_str)
        .expect("url")
        .to_string();
    let id = url.rsplit("%2F").next().expect("fetch id");
    let path = format!("/fetch/{id}");

    let first = handle(&mut store, &request("POST", &path, ""));
    assert_eq!(first.status, 200);
    let body: Value = serde_json::from_slice(&first.body).expect("fetch json");
    assert!(
        body.get("auth-token")
            .and_then(Value::as_str)
            .is_some_and(|token| token == format!("mock-token-{id}"))
    );

    let second = handle(&mut store, &request("POST", &path, ""));
    assert_eq!(second.status, 200);
    let body: Value = serde_json::from_slice(&second.body).expect("fetch json");
    assert_eq!(body.get("error-code").and_then(Value::as_str), Some("1"));
    assert!(has_violation(&store, "reused"));

    let get = handle(&mut store, &request("GET", &path, ""));
    assert_eq!(get.status, 405);

    let stmt_body = serde_json::to_vec(&json!({
        "id": uuid_for("after"),
        "timestamp": TS,
        "actor": actor_json(),
        "verb": { "id": format!("{VERB_BASE}initialized"), "display": { "en-US": "initialized" } },
        "object": { "id": "urn:coursecraft:mock:activity:hello" },
    }))
    .expect("json");
    let statement = Request {
        method: "POST".to_string(),
        path: "/xapi/statements".to_string(),
        query: String::new(),
        headers: vec![
            ("X-Experience-API-Version".to_string(), "1.0.3".to_string()),
            (
                "Authorization".to_string(),
                format!("Basic mock-token-{id}"),
            ),
            ("Content-Type".to_string(), "application/json".to_string()),
        ],
        body: stmt_body,
    };
    assert_eq!(handle(&mut store, &statement).status, 200);
}

#[test]
fn xapi_auth_is_required() {
    let mut store = empty_store();
    let stmt_body = serde_json::to_vec(&json!({
        "id": uuid_for("auth1"),
        "timestamp": TS,
        "actor": actor_json(),
        "verb": { "id": format!("{VERB_BASE}initialized"), "display": { "en-US": "initialized" } },
        "object": { "id": "urn:act" },
    }))
    .expect("json");
    let no_auth = Request {
        method: "POST".to_string(),
        path: "/xapi/statements".to_string(),
        query: String::new(),
        headers: vec![
            ("X-Experience-API-Version".to_string(), "1.0.3".to_string()),
            ("Content-Type".to_string(), "application/json".to_string()),
        ],
        body: stmt_body.clone(),
    };
    assert_eq!(handle(&mut store, &no_auth).status, 401);

    let with_auth = xapi_request("POST", "/xapi/statements", "", stmt_body);
    assert_eq!(handle(&mut store, &with_auth).status, 200);
}

#[test]
fn xapi_version_is_required() {
    let mut store = empty_store();
    let no_version = Request {
        method: "POST".to_string(),
        path: "/xapi/statements".to_string(),
        query: String::new(),
        headers: vec![("Authorization".to_string(), "Basic mock-xapi".to_string())],
        body: br#"[]"#.to_vec(),
    };
    assert_eq!(handle(&mut store, &no_version).status, 400);
}

#[test]
fn state_round_trip_with_encoded_agent_and_reordered_keys() {
    let mut store = empty_store();
    let agent = actor_json().to_string();
    let query = format!(
        "stateId={}&activityId={}&agent={}&registration={}",
        encode("coursecraft.progress"),
        encode("urn:coursecraft:mock:activity:hello"),
        encode(&agent),
        encode("urn:coursecraft:mock:registration:cmi5-hello"),
    );
    let body = br#"{"suspend":"cc1:abc","completed":false,"passed":false}"#.to_vec();
    let put = handle(
        &mut store,
        &xapi_request("PUT", "/xapi/activities/state", &query, body.clone()),
    );
    assert_eq!(put.status, 204);

    // BUG-A13: reordered keys in agent JSON still match by canonical IFI.
    let reordered_agent =
        r#"{"account":{"name":"learner-1","homePage":"http://127.0.0.1"},"objectType":"Agent"}"#;
    let reordered_query = format!(
        "stateId={}&activityId={}&agent={}&registration={}",
        encode("coursecraft.progress"),
        encode("urn:coursecraft:mock:activity:hello"),
        encode(reordered_agent),
        encode("urn:coursecraft:mock:registration:cmi5-hello"),
    );
    let get = handle(
        &mut store,
        &xapi_request(
            "GET",
            "/xapi/activities/state",
            &reordered_query,
            Vec::new(),
        ),
    );
    assert_eq!(get.status, 200);
    assert_eq!(get.body, body);

    let missing = handle(
        &mut store,
        &xapi_request(
            "GET",
            "/xapi/activities/state",
            &format!(
                "stateId={}&activityId={}&agent={}",
                encode("nope"),
                encode("urn:x"),
                encode(&agent)
            ),
            Vec::new(),
        ),
    );
    assert_eq!(missing.status, 404);
}

#[test]
fn duplicate_statement_ids_idempotent_or_409_conflict() {
    let mut store = empty_store();
    let statement = json!({
        "id": uuid_for("dup"),
        "timestamp": TS,
        "actor": actor_json(),
        "verb": { "id": format!("{VERB_BASE}initialized"), "display": { "en-US": "initialized" } },
        "object": { "id": "urn:act" },
    });
    let first = post_statements(&mut store, std::slice::from_ref(&statement));
    assert_eq!(first.status, 200);
    let second = post_statements(&mut store, std::slice::from_ref(&statement));
    assert_eq!(second.status, 200);
    assert_eq!(store.strict.statements().len(), 1);
    let ids: Value = serde_json::from_slice(&second.body).expect("ids json");
    assert_eq!(ids.as_array().map(Vec::len), Some(1));

    // BUG-A13: same id with mutated content returns 409 Conflict.
    let mut mutated = statement;
    mutated["timestamp"] = json!("2026-11-21T12:05:00Z");
    let conflict = post_statements(&mut store, &[mutated]);
    assert_eq!(conflict.status, 409);
    assert!(has_violation(&store, "XAPI-409-DUPLICATE-ID"));
}

#[test]
fn lms_launch_data_is_protected() {
    let mut store = empty_store();
    let query = format!(
        "stateId={}&activityId={}&agent={}",
        encode(LAUNCH_DATA),
        encode("urn:act"),
        encode(&actor_json().to_string())
    );
    let response = handle(
        &mut store,
        &xapi_request("PUT", "/xapi/activities/state", &query, b"{}".to_vec()),
    );
    assert_eq!(response.status, 403);
    assert!(has_violation(&store, "LMS.LaunchData"));
}

#[test]
fn path_traversal_is_refused() {
    let mut store = empty_store();
    assert_eq!(
        handle(&mut store, &request("GET", "/..%2fCargo.toml", "")).status,
        404
    );
    assert_eq!(
        handle(&mut store, &request("GET", "/a/../../Cargo.toml", "")).status,
        404
    );
    assert_eq!(
        handle(&mut store, &request("GET", "/..%5cCargo.toml", "")).status,
        404
    );
}

#[test]
fn fail_injection_fails_the_next_statement_posts() {
    let mut store = empty_store();
    let response = handle(
        &mut store,
        &request("POST", "/mock/fail", "count=1&status=503"),
    );
    assert_eq!(response.status, 200);
    let statement = json!({
        "id": uuid_for("fail1"),
        "timestamp": TS,
        "actor": actor_json(),
        "verb": { "id": format!("{VERB_BASE}initialized"), "display": { "en-US": "initialized" } },
        "object": { "id": "urn:act" },
    });
    let failed = post_statements(&mut store, std::slice::from_ref(&statement));
    assert_eq!(failed.status, 503);
    let accepted = post_statements(&mut store, std::slice::from_ref(&statement));
    assert_eq!(accepted.status, 200);
}

#[test]
fn good_cmi5_sequence_has_no_problems() {
    let mut store = empty_store();
    let registration = cmi5_registration(&mut store, None);
    let statements = vec![
        cmi5_statement("initialized", &registration, "session-1", "s1"),
        cmi5_statement("completed", &registration, "session-1", "s2"),
        cmi5_statement("passed", &registration, "session-1", "s3"),
        cmi5_statement("terminated", &registration, "session-1", "s4"),
    ];
    let response = post_statements(&mut store, &statements);
    assert_eq!(response.status, 200);
    assert!(
        store.strict.violations().is_empty(),
        "{:?}",
        store.strict.violations()
    );
}

#[test]
fn repeated_answered_statements_are_allowed() {
    let mut store = empty_store();
    let registration = cmi5_registration(&mut store, None);
    let statements = vec![
        cmi5_statement("initialized", &registration, "session-1", "s1"),
        cmi5_statement("answered", &registration, "session-1", "s2"),
        cmi5_statement("answered", &registration, "session-1", "s3"),
        cmi5_statement("terminated", &registration, "session-1", "s4"),
    ];
    let response = post_statements(&mut store, &statements);
    assert_eq!(response.status, 200);
    assert!(
        store.strict.violations().is_empty(),
        "{:?}",
        store.strict.violations()
    );
}

#[test]
fn initialized_must_be_first() {
    let mut store = empty_store();
    let registration = cmi5_registration(&mut store, None);
    let statements = vec![cmi5_statement(
        "completed",
        &registration,
        "session-1",
        "s1",
    )];
    post_statements(&mut store, &statements);
    assert!(has_violation(&store, "before initialized"));
}

#[test]
fn nothing_may_follow_terminated() {
    let mut store = empty_store();
    let registration = cmi5_registration(&mut store, None);
    let statements = vec![
        cmi5_statement("initialized", &registration, "session-1", "s1"),
        cmi5_statement("terminated", &registration, "session-1", "s2"),
        cmi5_statement("completed", &registration, "session-1", "s3"),
    ];
    post_statements(&mut store, &statements);
    assert!(has_violation(&store, "after terminated"));
}

#[test]
fn cmi5_requires_sessionid_and_registration() {
    let mut store = empty_store();
    let registration = cmi5_registration(&mut store, None);
    let mut no_session = cmi5_statement("initialized", &registration, "session-1", "s1");
    if let Some(extensions) = no_session
        .pointer_mut("/context/extensions")
        .and_then(Value::as_object_mut)
    {
        extensions.remove(SESSION_ID_EXTENSION);
    }
    post_statements(&mut store, &[no_session]);
    assert!(has_violation(&store, "missing sessionid"));

    let mut no_registration = cmi5_statement("initialized", &registration, "session-2", "s2");
    if let Some(context) = no_registration
        .pointer_mut("/context")
        .and_then(Value::as_object_mut)
    {
        context.remove("registration");
    }
    post_statements(&mut store, &[no_registration]);
    assert!(has_violation(&store, "missing context.registration"));
}

#[test]
fn cmi5_category_rules() {
    let mut store = empty_store();
    let registration = cmi5_registration(&mut store, None);
    let mut missing = cmi5_statement("initialized", &registration, "session-1", "s1");
    category_ids_of(&mut missing).clear();
    post_statements(&mut store, &[missing]);
    assert!(has_violation(&store, "missing cmi5 category"));

    let mut answered = cmi5_statement("answered", &registration, "session-2", "s2");
    category_ids_of(&mut answered).push(json!({ "id": CMI5_CATEGORY }));
    let statements = vec![
        cmi5_statement("initialized", &registration, "session-2", "s3"),
        answered,
    ];
    post_statements(&mut store, &statements);
    assert!(has_violation(&store, "unexpected cmi5 category"));
}

#[test]
fn moveon_category_rules() {
    let mut store = empty_store();
    let registration = cmi5_registration(&mut store, None);
    let mut missing = cmi5_statement("completed", &registration, "session-1", "s1");
    category_ids_of(&mut missing)
        .retain(|entry| entry.get("id").and_then(Value::as_str) != Some(MOVEON_CATEGORY));
    let statements = vec![
        cmi5_statement("initialized", &registration, "session-1", "s0"),
        missing,
    ];
    post_statements(&mut store, &statements);
    assert!(has_violation(&store, "missing moveon category"));

    let mut unexpected = cmi5_statement("initialized", &registration, "session-2", "s2");
    category_ids_of(&mut unexpected).push(json!({ "id": MOVEON_CATEGORY }));
    post_statements(&mut store, &[unexpected]);
    assert!(has_violation(&store, "unexpected moveon category"));
}

#[test]
fn duration_rules() {
    let mut store = empty_store();
    let registration = cmi5_registration(&mut store, None);
    let mut completed = cmi5_statement("completed", &registration, "session-1", "s1");
    if let Some(result) = completed
        .pointer_mut("/result")
        .and_then(Value::as_object_mut)
    {
        result.remove("duration");
    }
    let statements = vec![
        cmi5_statement("initialized", &registration, "session-1", "s0"),
        completed,
    ];
    post_statements(&mut store, &statements);
    assert!(has_violation(&store, "CMI5-RESULT-COMPLETED"));

    let mut terminated = cmi5_statement("terminated", &registration, "session-2", "s2");
    if let Some(result) = terminated
        .pointer_mut("/result")
        .and_then(Value::as_object_mut)
    {
        result.remove("duration");
    }
    let statements = vec![
        cmi5_statement("initialized", &registration, "session-2", "s3"),
        terminated,
    ];
    post_statements(&mut store, &statements);
    assert!(has_violation(&store, "CMI5-RESULT-TERMINATED"));
}

#[test]
fn completed_must_not_carry_a_score() {
    let mut store = empty_store();
    let registration = cmi5_registration(&mut store, None);
    let mut completed = cmi5_statement("completed", &registration, "session-1", "s1");
    if let Some(result) = completed
        .pointer_mut("/result")
        .and_then(Value::as_object_mut)
    {
        result.insert("score".into(), json!({ "scaled": 0.5 }));
    }
    let statements = vec![
        cmi5_statement("initialized", &registration, "session-1", "s0"),
        completed,
    ];
    post_statements(&mut store, &statements);
    assert!(has_violation(&store, "CMI5-RESULT-COMPLETED"));
}

#[test]
fn completed_and_passed_only_once_per_registration() {
    let mut store = empty_store();
    let registration = cmi5_registration(&mut store, None);
    let statements = vec![
        cmi5_statement("initialized", &registration, "session-1", "s1"),
        cmi5_statement("completed", &registration, "session-1", "s2"),
        cmi5_statement("passed", &registration, "session-1", "s3"),
        cmi5_statement("terminated", &registration, "session-1", "s4"),
        cmi5_statement("initialized", &registration, "session-2", "s5"),
        cmi5_statement("completed", &registration, "session-2", "s6"),
        cmi5_statement("passed", &registration, "session-2", "s7"),
        cmi5_statement("terminated", &registration, "session-2", "s8"),
    ];
    post_statements(&mut store, &statements);
    assert!(has_violation(&store, "second completed"));
    assert!(has_violation(&store, "second passed"));
}

#[test]
fn passed_after_failed_and_failed_after_passed_are_violations() {
    let mut store = empty_store();
    let registration = cmi5_registration(&mut store, None);
    let statements = vec![
        cmi5_statement("initialized", &registration, "session-1", "s1"),
        cmi5_statement("passed", &registration, "session-1", "s2"),
        cmi5_statement("terminated", &registration, "session-1", "s3"),
        cmi5_statement("initialized", &registration, "session-2", "s4"),
        cmi5_statement("failed", &registration, "session-2", "s5"),
        cmi5_statement("terminated", &registration, "session-2", "s6"),
    ];
    post_statements(&mut store, &statements);
    assert!(has_violation(&store, "failed after passed"));

    // BUG-A13: passed after failed in same cmi5 session.
    let mut store2 = empty_store();
    let reg2 = cmi5_registration(&mut store2, None);
    let s2 = vec![
        cmi5_statement("initialized", &reg2, "session-1", "p1"),
        cmi5_statement("failed", &reg2, "session-1", "p2"),
        cmi5_statement("passed", &reg2, "session-1", "p3"),
    ];
    post_statements(&mut store2, &s2);
    assert!(has_violation(&store2, "CMI5-9.3.1-PASS-AFTER-FAIL"));
}

#[test]
fn plain_xapi_relaunch_does_not_collide_on_same_registration() {
    let mut store = empty_store();
    handle(
        &mut store,
        &launch_request("xapi", "hello", "Normal", None, false),
    );
    let reg = first_registration(&store);
    let xapi_stmt = |verb: &str, id: &str| {
        json!({
            "id": uuid_for(id),
            "timestamp": TS,
            "actor": actor_json(),
            "verb": { "id": format!("{VERB_BASE}{verb}"), "display": { "en-US": verb } },
            "object": { "id": "urn:coursecraft:mock:activity:hello" },
            "context": { "registration": reg },
        })
    };
    post_statements(
        &mut store,
        &[
            xapi_stmt("initialized", "x1"),
            xapi_stmt("terminated", "x2"),
        ],
    );
    // Relaunch plain xAPI on same registration.
    handle(
        &mut store,
        &launch_request("xapi", "hello", "Normal", None, false),
    );
    post_statements(
        &mut store,
        &[
            xapi_stmt("initialized", "x3"),
            xapi_stmt("terminated", "x4"),
        ],
    );
    assert!(
        store.strict.violations().is_empty(),
        "unexpected violations: {:?}",
        store.strict.violations()
    );
}

#[test]
fn a_verb_may_not_repeat_in_a_session() {
    let mut store = empty_store();
    let registration = cmi5_registration(&mut store, None);
    let statements = vec![
        cmi5_statement("initialized", &registration, "session-1", "s1"),
        cmi5_statement("initialized", &registration, "session-1", "s2"),
    ];
    post_statements(&mut store, &statements);
    assert!(has_violation(&store, "repeated verb"));
}

#[test]
fn browse_may_only_send_initialized_and_terminated() {
    let mut store = empty_store();
    handle(
        &mut store,
        &launch_request("cmi5", "hello", "Browse", None, false),
    );
    let registration = first_registration(&store);
    let statements = vec![
        cmi5_statement("initialized", &registration, "session-1", "s1"),
        cmi5_statement("completed", &registration, "session-1", "s2"),
    ];
    post_statements(&mut store, &statements);
    assert!(has_violation(&store, "CMI5-MODE-DISALLOWED"));
}

#[test]
fn mastery_score_and_scaled_must_agree() {
    let mut store = empty_store();
    let registration = cmi5_registration(&mut store, Some("0.8"));
    let statements = vec![
        cmi5_statement("initialized", &registration, "session-1", "s1"),
        cmi5_statement("completed", &registration, "session-1", "s2"),
        cmi5_statement("passed", &registration, "session-1", "s3"),
    ];
    post_statements(&mut store, &statements);
    assert!(has_violation(&store, "missing masteryscore"));

    let mut second = empty_store();
    let registration = cmi5_registration(&mut second, Some("0.8"));
    let mut passed = add_extension(
        cmi5_statement("passed", &registration, "session-1", "s3"),
        MASTERY_EXTENSION,
        json!(0.8),
    );
    if let Some(score) = passed
        .pointer_mut("/result/score")
        .and_then(Value::as_object_mut)
    {
        score.insert("scaled".into(), json!(0.5));
    }
    let statements = vec![
        cmi5_statement("initialized", &registration, "session-1", "s1"),
        cmi5_statement("completed", &registration, "session-1", "s2"),
        passed,
    ];
    post_statements(&mut second, &statements);
    assert!(has_violation(&second, "disagrees with masteryScore"));
}

#[test]
fn xapi_package_paths_are_static_files_not_lrs_requests() {
    let mut store = empty_store();
    // No version header: an LRS route would answer 400. A package page is a file.
    let page = request("GET", "/xapi/hello/index.html", "");
    assert_eq!(handle(&mut store, &page).status, 404);
    let statements = request("GET", "/xapi/statements", "");
    assert_eq!(handle(&mut store, &statements).status, 400);
}
