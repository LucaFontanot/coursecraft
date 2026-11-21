//! Strict xAPI 1.0.3 and cmi5 conformance engine (`StrictLrs`).
//!
//! Pure L1 implementation: zero filesystem, zero sockets, zero `std::time`,
//! zero `unsafe`. Shared across native unit/integration tests and `xtask`'s
//! local mock LRS.

use std::collections::{BTreeMap, HashMap, HashSet};

pub use coursecraft_xapi::Flavor;
use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::Violation;

const CMI5_CATEGORY: &str = "https://w3id.org/xapi/cmi5/context/categories/cmi5";
const MOVEON_CATEGORY: &str = "https://w3id.org/xapi/cmi5/context/categories/moveon";
const SESSION_ID_EXTENSION: &str = "https://w3id.org/xapi/cmi5/context/extensions/sessionid";
const MASTERY_EXTENSION: &str = "https://w3id.org/xapi/cmi5/context/extensions/masteryscore";
const VERB_BASE: &str = "http://adlnet.gov/expapi/verbs/";
const INTERACTION_ACTIVITY_TYPE: &str = "http://adlnet.gov/expapi/activities/cmi.interaction";
const LAUNCH_DATA_STATE_ID: &str = "LMS.LaunchData";

/// Minimal HTTP response produced by [`StrictLrs`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LrsResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl LrsResponse {
    fn json(status: u16, value: &Value) -> Self {
        Self {
            status,
            headers: vec![
                ("Content-Type".to_owned(), "application/json".to_owned()),
                ("X-Experience-API-Version".to_owned(), "1.0.3".to_owned()),
            ],
            body: serde_json::to_vec(value).unwrap_or_default(),
        }
    }

    fn text(status: u16, msg: impl Into<String>) -> Self {
        Self {
            status,
            headers: vec![
                (
                    "Content-Type".to_owned(),
                    "text/plain; charset=utf-8".to_owned(),
                ),
                ("X-Experience-API-Version".to_owned(), "1.0.3".to_owned()),
            ],
            body: msg.into().into_bytes(),
        }
    }

    fn empty(status: u16) -> Self {
        Self {
            status,
            headers: vec![("X-Experience-API-Version".to_owned(), "1.0.3".to_owned())],
            body: Vec::new(),
        }
    }
}

/// Parameters for registering a cmi5 or plain xAPI launch in [`StrictLrs`].
#[derive(Debug, Clone)]
pub struct LaunchSpec {
    pub flavor: Flavor,
    pub activity_id: String,
    pub actor: Value,
    pub registration: Option<String>,
    pub launch_mode: String,
    pub mastery_score: Option<f64>,
    pub session_id: String,
    pub grouping_ids: Vec<String>,
    pub fetch_id: Option<String>,
    pub auth_token: Option<String>,
}

/// Public view of a stored State API key (for diagnostics / mock LRS logs).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StateKeyView {
    #[serde(rename = "stateId")]
    pub state_id: String,
    #[serde(rename = "activityId")]
    pub activity_id: String,
    pub agent: String,
    pub registration: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CanonicalStateKey {
    state_id: String,
    activity_id: String,
    agent_ifi: String,
    registration: Option<String>,
}

#[derive(Debug, Clone)]
struct StateEntry {
    raw_agent: String,
    body: Vec<u8>,
}

#[derive(Debug, Clone)]
struct RegistrationState {
    flavor: Flavor,
    launch_mode: String,
    mastery_score: Option<f64>,
    activity_id: String,
    actor_ifi: Option<String>,
    completed_count: u32,
    passed: bool,
}

#[derive(Debug, Clone)]
struct SessionLaunchInfo {
    flavor: Flavor,
    launch_mode: String,
    mastery_score: Option<f64>,
    activity_id: String,
    actor_ifi: Option<String>,
    grouping_ids: Vec<String>,
}

#[derive(Debug, Default)]
struct SessionState {
    initialized: bool,
    terminated: bool,
    verbs: HashSet<String>,
    passed: bool,
    failed: bool,
    completed: bool,
}

#[derive(Debug, Clone)]
struct FetchEntry {
    used: bool,
    token: String,
}

/// Strict xAPI 1.0.3 and cmi5 LRS conformance engine.
#[derive(Debug, Default)]
pub struct StrictLrs {
    call_index: usize,
    violations: Vec<Violation>,
    warnings: Vec<Violation>,
    registrations: HashMap<String, RegistrationState>,
    session_launches: HashMap<String, SessionLaunchInfo>,
    token_sessions: HashMap<String, String>,
    sessions: HashMap<String, SessionState>,
    fetches: HashMap<String, FetchEntry>,
    tokens: HashSet<String>,
    statements: Vec<Value>,
    statement_by_id: HashMap<String, Value>,
    state: HashMap<CanonicalStateKey, StateEntry>,
}

impl StrictLrs {
    /// Create an empty [`StrictLrs`] instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear all recorded launches, statements, state, and violations.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// All spec violations (MUST/SHALL failures) recorded so far.
    #[must_use]
    pub fn violations(&self) -> &[Violation] {
        &self.violations
    }

    /// All advisory warnings (SHOULD / non-fatal notices) recorded so far.
    #[must_use]
    pub fn warnings(&self) -> &[Violation] {
        &self.warnings
    }

    /// All accepted statements in arrival order.
    #[must_use]
    pub fn statements(&self) -> &[Value] {
        &self.statements
    }

    /// List all keys currently in the State API store.
    #[must_use]
    pub fn state_keys(&self) -> Vec<StateKeyView> {
        self.state
            .iter()
            .map(|(k, v)| StateKeyView {
                state_id: k.state_id.clone(),
                activity_id: k.activity_id.clone(),
                agent: v.raw_agent.clone(),
                registration: k.registration.clone(),
            })
            .collect()
    }

    /// Read a State API entry directly using canonical IFI agent matching.
    #[must_use]
    pub fn get_state(
        &self,
        state_id: &str,
        activity_id: &str,
        agent: &Value,
        registration: Option<&str>,
    ) -> Option<&[u8]> {
        let ifi = canonical_ifi(agent)?;
        let key = CanonicalStateKey {
            state_id: state_id.to_owned(),
            activity_id: activity_id.to_owned(),
            agent_ifi: ifi,
            registration: registration.map(str::to_owned),
        };
        self.state.get(&key).map(|entry| entry.body.as_slice())
    }

    /// Check whether an `Authorization` header value is registered.
    #[must_use]
    pub fn has_token(&self, auth: &str) -> bool {
        self.tokens.contains(auth)
    }

    /// Register an `Authorization` header value directly.
    pub fn add_token(&mut self, auth: String) {
        self.tokens.insert(auth);
    }

    /// Register a launch (cmi5 or plain xAPI), storing `LMS.LaunchData` if provided
    /// and associating fetch / auth tokens with the launch session.
    pub fn register_launch(&mut self, spec: LaunchSpec, launch_data: Option<Value>) {
        let actor_ifi = canonical_ifi(&spec.actor);
        if let Some(reg) = spec.registration.as_deref()
            && !reg.is_empty()
        {
            let entry =
                self.registrations
                    .entry(reg.to_owned())
                    .or_insert_with(|| RegistrationState {
                        flavor: spec.flavor,
                        launch_mode: spec.launch_mode.clone(),
                        mastery_score: spec.mastery_score,
                        activity_id: spec.activity_id.clone(),
                        actor_ifi: actor_ifi.clone(),
                        completed_count: 0,
                        passed: false,
                    });
            entry.flavor = spec.flavor;
            entry.launch_mode = spec.launch_mode.clone();
            entry.mastery_score = spec.mastery_score;
            entry.activity_id = spec.activity_id.clone();
            entry.actor_ifi = actor_ifi.clone();
        }

        self.session_launches.insert(
            spec.session_id.clone(),
            SessionLaunchInfo {
                flavor: spec.flavor,
                launch_mode: spec.launch_mode,
                mastery_score: spec.mastery_score,
                activity_id: spec.activity_id.clone(),
                actor_ifi: actor_ifi.clone(),
                grouping_ids: spec.grouping_ids,
            },
        );

        if let Some(token) = spec.auth_token {
            self.token_sessions
                .insert(token.clone(), spec.session_id.clone());
            self.tokens.insert(token);
        }

        if let (Some(fetch_id), Some(token)) = (spec.fetch_id, Some(spec.session_id.clone())) {
            let auth_token = format!("mock-token-{fetch_id}");
            let auth_header = format!("Basic {auth_token}");
            self.token_sessions.insert(auth_header, token);
            self.fetches.insert(
                fetch_id,
                FetchEntry {
                    used: false,
                    token: auth_token,
                },
            );
        }

        if let (Some(doc), Some(ifi)) = (launch_data, actor_ifi) {
            let key = CanonicalStateKey {
                state_id: LAUNCH_DATA_STATE_ID.to_owned(),
                activity_id: spec.activity_id,
                agent_ifi: ifi,
                registration: spec.registration,
            };
            self.state.insert(
                key,
                StateEntry {
                    raw_agent: spec.actor.to_string(),
                    body: serde_json::to_vec(&doc).unwrap_or_default(),
                },
            );
        }
    }

    /// Handle a cmi5 `POST /fetch/{id}` request.
    pub fn handle_fetch(
        &mut self,
        method: &str,
        fetch_id: &str,
        headers: &[(String, String)],
    ) -> LrsResponse {
        self.call_index = self.call_index.saturating_add(1);
        if !method.eq_ignore_ascii_case("POST") {
            return LrsResponse::text(405, "Method Not Allowed");
        }
        if header_value(headers, "Authorization").is_some_and(|v| !v.is_empty()) {
            self.record_warning(
                "cmi5",
                "CMI5-FETCH-AUTH-HEADER",
                "Authorization",
                "cmi5 fetch request should not carry an Authorization header",
                None,
            );
        }
        let Some(entry) = self.fetches.get_mut(fetch_id) else {
            return LrsResponse::text(404, "unknown fetch id");
        };
        if entry.used {
            self.record_violation(
                "cmi5",
                "CMI5-FETCH-REUSED",
                format!("fetch/{fetch_id}"),
                format!("fetch id {fetch_id} reused (single-use violation)"),
                Some("200"),
            );
            return LrsResponse::json(
                200,
                &json!({ "error-code": "1", "error-text": "Already in use" }),
            );
        }
        entry.used = true;
        let token = entry.token.clone();
        self.tokens.insert(format!("Basic {token}"));
        LrsResponse::json(200, &json!({ "auth-token": token }))
    }

    /// Handle `/statements` (`POST` or `GET`).
    pub fn handle_statements(
        &mut self,
        method: &str,
        headers: &[(String, String)],
        body: &[u8],
    ) -> LrsResponse {
        self.call_index = self.call_index.saturating_add(1);
        if let Some(resp) = self.check_xapi_headers(headers, false) {
            return resp;
        }
        if method.eq_ignore_ascii_case("GET") {
            return LrsResponse::json(200, &json!({ "statements": self.statements }));
        }
        if !method.eq_ignore_ascii_case("POST") {
            return LrsResponse::text(405, "Method Not Allowed");
        }
        if !has_json_content_type(headers) {
            self.record_violation(
                "xapi",
                "XAPI-HTTP-CONTENT-TYPE",
                "Content-Type",
                "POST /statements requires Content-Type starting with application/json",
                Some("400"),
            );
            return LrsResponse::text(400, "Content-Type: application/json required");
        }
        let Ok(value) = serde_json::from_slice::<Value>(body) else {
            self.record_violation(
                "xapi",
                "XAPI-HTTP-JSON",
                "body",
                "request body is not valid JSON",
                Some("400"),
            );
            return LrsResponse::text(400, "invalid JSON");
        };
        let statements = match value {
            Value::Array(items) => items,
            Value::Object(_) => vec![value],
            _ => {
                self.record_violation(
                    "xapi",
                    "XAPI-HTTP-BODY",
                    "body",
                    "statements body must be a JSON object or array",
                    Some("400"),
                );
                return LrsResponse::text(400, "statement must be an object or array");
            }
        };

        let auth_session = header_value(headers, "Authorization")
            .and_then(|auth| self.token_sessions.get(auth))
            .cloned();

        let mut ids = Vec::with_capacity(statements.len());
        for statement in statements {
            let Some(id) = statement.get("id").and_then(Value::as_str) else {
                self.record_violation(
                    "xapi",
                    "XAPI-STMT-ID-MISSING",
                    "id",
                    "statement is missing string `id`",
                    Some("400"),
                );
                return LrsResponse::text(400, "statement has no string id");
            };
            let id = id.to_owned();
            if !is_valid_uuid(&id) {
                self.record_violation(
                    "xapi",
                    "XAPI-STMT-ID-UUID",
                    "id",
                    format!("statement id `{id}` is not a valid RFC 4122 UUID"),
                    Some("400"),
                );
                return LrsResponse::text(400, "statement id must be a valid UUID");
            }
            if let Some(existing) = self.statement_by_id.get(&id) {
                if existing != &statement {
                    self.record_violation(
                        "xapi",
                        "XAPI-409-DUPLICATE-ID",
                        "id",
                        format!("duplicate statement id `{id}` with different content"),
                        Some("409"),
                    );
                    return LrsResponse::text(
                        409,
                        "Conflict: duplicate statement id with different content",
                    );
                }
                ids.push(id);
                continue;
            }
            self.validate_statement(&statement, auth_session.as_deref());
            self.statement_by_id.insert(id.clone(), statement.clone());
            self.statements.push(statement);
            ids.push(id);
        }
        LrsResponse::json(200, &json!(ids))
    }

    /// Handle `/activities/state` (`GET`, `PUT`, `DELETE`).
    pub fn handle_state(
        &mut self,
        method: &str,
        query: &str,
        headers: &[(String, String)],
        body: &[u8],
    ) -> LrsResponse {
        self.call_index = self.call_index.saturating_add(1);
        if let Some(resp) = self.check_xapi_headers(headers, false) {
            return resp;
        }
        let params = query_map(query);
        let Some(state_id) = params.get("stateId") else {
            return LrsResponse::text(400, "stateId required");
        };
        let Some(activity_id) = params.get("activityId") else {
            return LrsResponse::text(400, "activityId required");
        };
        let Some(agent_raw) = params.get("agent") else {
            return LrsResponse::text(400, "agent required");
        };
        let Ok(agent_val) = serde_json::from_str::<Value>(agent_raw) else {
            self.record_violation(
                "xapi",
                "XAPI-STATE-AGENT-JSON",
                "agent",
                "State API `agent` query parameter is not valid JSON",
                Some("400"),
            );
            return LrsResponse::text(400, "agent must be valid JSON");
        };
        let Some(agent_ifi) = canonical_ifi(&agent_val) else {
            self.record_violation(
                "xapi",
                "XAPI-STATE-AGENT-IFI",
                "agent",
                "State API `agent` must have exactly one IFI (mbox, mbox_sha1sum, openid, or account)",
                Some("400"),
            );
            return LrsResponse::text(400, "agent must have exactly one IFI");
        };

        let key = CanonicalStateKey {
            state_id: state_id.clone(),
            activity_id: activity_id.clone(),
            agent_ifi,
            registration: params.get("registration").cloned(),
        };

        if method.eq_ignore_ascii_case("GET") {
            return match self.state.get(&key) {
                Some(entry) => LrsResponse {
                    status: 200,
                    headers: vec![
                        ("Content-Type".to_owned(), "application/json".to_owned()),
                        ("X-Experience-API-Version".to_owned(), "1.0.3".to_owned()),
                    ],
                    body: entry.body.clone(),
                },
                None => LrsResponse::text(404, "Not Found"),
            };
        }

        if state_id == LAUNCH_DATA_STATE_ID {
            self.record_violation(
                "cmi5",
                "CMI5-STATE-LAUNCHDATA-READONLY",
                "stateId",
                "AU attempted to mutate read-only LMS.LaunchData",
                Some("403"),
            );
            return LrsResponse::text(403, "LMS.LaunchData is written by the LMS only");
        }

        if method.eq_ignore_ascii_case("PUT") || method.eq_ignore_ascii_case("POST") {
            self.state.insert(
                key,
                StateEntry {
                    raw_agent: agent_raw.clone(),
                    body: body.to_vec(),
                },
            );
            return LrsResponse::empty(204);
        }

        if method.eq_ignore_ascii_case("DELETE") {
            return if self.state.remove(&key).is_some() {
                LrsResponse::empty(204)
            } else {
                LrsResponse::text(404, "Not Found")
            };
        }

        LrsResponse::text(405, "Method Not Allowed")
    }

    /// Snapshot of current LRS state for test assertions.
    #[must_use]
    pub fn snapshot(&self) -> Value {
        json!({
            "call_index": self.call_index,
            "statement_count": self.statements.len(),
            "state_count": self.state.len(),
            "violation_count": self.violations.len(),
            "warning_count": self.warnings.len(),
        })
    }

    fn check_xapi_headers(
        &mut self,
        headers: &[(String, String)],
        require_auth: bool,
    ) -> Option<LrsResponse> {
        let version = header_value(headers, "X-Experience-API-Version").unwrap_or_default();
        if !is_valid_xapi_version(version) {
            self.record_violation(
                "xapi",
                "XAPI-HTTP-VERSION",
                "X-Experience-API-Version",
                format!("invalid or missing X-Experience-API-Version `{version}`"),
                Some("400"),
            );
            return Some(LrsResponse::text(
                400,
                "X-Experience-API-Version 1.0.x header required",
            ));
        }
        let auth = header_value(headers, "Authorization").unwrap_or_default();
        if (require_auth || !self.tokens.is_empty()) && !self.tokens.contains(auth) {
            return Some(LrsResponse::text(401, "Unauthorized"));
        }
        None
    }

    fn validate_statement(&mut self, statement: &Value, auth_session: Option<&str>) {
        self.validate_base_xapi(statement);

        let verb = verb_of(statement).to_owned();
        let registration = statement
            .pointer("/context/registration")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let session_id = statement
            .pointer(&format!(
                "/context/extensions/{}",
                escape_pointer(SESSION_ID_EXTENSION)
            ))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();

        let reg_info = self.registrations.get(&registration).cloned();
        let sess_info = if !session_id.is_empty() {
            self.session_launches.get(&session_id).cloned()
        } else {
            auth_session.and_then(|s| self.session_launches.get(s).cloned())
        };

        let categories = category_ids(statement);
        let inferred_cmi5 = !session_id.is_empty() || categories.contains(&CMI5_CATEGORY);
        let flavor = sess_info
            .as_ref()
            .map(|s| s.flavor)
            .or_else(|| reg_info.as_ref().map(|r| r.flavor))
            .unwrap_or(if inferred_cmi5 {
                Flavor::Cmi5
            } else {
                Flavor::Xapi
            });

        if flavor == Flavor::Cmi5 {
            if registration.is_empty() {
                self.record_violation(
                    "cmi5",
                    "CMI5-CONTEXT-REGISTRATION",
                    "context.registration",
                    format!("{verb}: missing context.registration"),
                    None,
                );
            }
            if session_id.is_empty() {
                self.record_violation(
                    "cmi5",
                    "CMI5-CONTEXT-SESSIONID",
                    "context.extensions.sessionid",
                    format!("{verb}: missing sessionid extension"),
                    None,
                );
            }
        }

        let session_key = if flavor == Flavor::Cmi5 && !session_id.is_empty() {
            format!("cmi5:{session_id}")
        } else if let Some(handle) = auth_session {
            format!("xapi:{handle}")
        } else {
            format!("reg:{registration}")
        };

        let after_terminated = self.validate_session_order(&session_key, &verb, flavor);
        if after_terminated {
            return;
        }

        if flavor == Flavor::Cmi5 {
            self.validate_cmi5_statement(statement, &verb, sess_info.as_ref(), reg_info.as_ref());
        }
        self.validate_registration_cardinality(&registration, &verb);
    }

    fn validate_base_xapi(&mut self, statement: &Value) {
        if let Some(ts) = statement.get("timestamp").and_then(Value::as_str) {
            if !is_valid_rfc3339(ts) {
                self.record_violation(
                    "xapi",
                    "XAPI-STMT-TIMESTAMP",
                    "timestamp",
                    format!("invalid RFC 3339 timestamp `{ts}`"),
                    None,
                );
            }
        } else {
            self.record_violation(
                "xapi",
                "XAPI-STMT-TIMESTAMP-MISSING",
                "timestamp",
                "statement is missing `timestamp`",
                None,
            );
        }

        let Some(actor) = statement.get("actor") else {
            self.record_violation(
                "xapi",
                "XAPI-STMT-ACTOR-MISSING",
                "actor",
                "statement is missing `actor`",
                None,
            );
            return;
        };
        if canonical_ifi(actor).is_none() {
            self.record_violation(
                "xapi",
                "XAPI-STMT-ACTOR-IFI",
                "actor",
                "actor must be an Agent with exactly one IFI",
                None,
            );
        }

        self.validate_verb(statement);
        self.validate_object(statement);
        self.validate_result_general(statement);
    }

    fn validate_verb(&mut self, statement: &Value) {
        let verb_id = statement
            .pointer("/verb/id")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !is_absolute_iri(verb_id) {
            self.record_violation(
                "xapi",
                "XAPI-STMT-VERB-ID",
                "verb.id",
                format!("verb.id `{verb_id}` is not a valid absolute IRI"),
                None,
            );
        }
        let has_display = statement
            .pointer("/verb/display")
            .and_then(Value::as_object)
            .is_some_and(|map| !map.is_empty() && map.values().all(Value::is_string));
        if !has_display {
            self.record_violation(
                "xapi",
                "XAPI-STMT-VERB-DISPLAY",
                "verb.display",
                "verb.display must be a non-empty language map",
                None,
            );
        }
    }

    fn validate_object(&mut self, statement: &Value) {
        let obj_id = statement
            .pointer("/object/id")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !is_absolute_iri(obj_id) {
            self.record_violation(
                "xapi",
                "XAPI-STMT-OBJECT-ID",
                "object.id",
                format!("object.id `{obj_id}` is not a valid absolute IRI"),
                None,
            );
        }
        if verb_of(statement) == "answered" {
            self.validate_answered_object(statement);
        }
    }

    fn validate_answered_object(&mut self, statement: &Value) {
        let def_type = statement
            .pointer("/object/definition/type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if def_type != INTERACTION_ACTIVITY_TYPE {
            self.record_violation(
                "xapi",
                "XAPI-ANSWERED-TYPE",
                "object.definition.type",
                format!("expected cmi.interaction type, got `{def_type}`"),
                None,
            );
        }
        let itype = statement
            .pointer("/object/definition/interactionType")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !matches!(
            itype,
            "true-false"
                | "choice"
                | "fill-in"
                | "long-fill-in"
                | "likert"
                | "matching"
                | "performance"
                | "sequencing"
                | "numeric"
                | "other"
        ) {
            self.record_violation(
                "xapi",
                "XAPI-ANSWERED-INTERACTION-TYPE",
                "object.definition.interactionType",
                format!("invalid interactionType `{itype}`"),
                None,
            );
        }
        let patterns = statement.pointer("/object/definition/correctResponsesPattern");
        let empty_patterns = patterns
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty);
        if (itype == "likert" || empty_patterns) && statement.pointer("/result/success").is_some() {
            self.record_violation(
                "xapi",
                "XAPI-SURVEY-SUCCESS",
                "result.success",
                "survey questions must omit result.success",
                None,
            );
        }
    }

    fn validate_result_general(&mut self, statement: &Value) {
        if let Some(dur) = statement
            .pointer("/result/duration")
            .and_then(Value::as_str)
            && !is_valid_xapi_duration(dur)
        {
            self.record_violation(
                "xapi",
                "XAPI-RESULT-DURATION",
                "result.duration",
                format!("invalid ISO 8601 duration `{dur}`"),
                None,
            );
        }
        let Some(score) = statement
            .pointer("/result/score")
            .and_then(Value::as_object)
        else {
            return;
        };
        if let Some(scaled) = score.get("scaled").and_then(Value::as_f64)
            && !(scaled.is_finite() && (-1.0..=1.0).contains(&scaled))
        {
            self.record_violation(
                "xapi",
                "XAPI-SCORE-SCALED",
                "result.score.scaled",
                format!("result.score.scaled {scaled} out of [-1.0, 1.0]"),
                None,
            );
        }
        let min = score.get("min").and_then(Value::as_f64);
        let max = score.get("max").and_then(Value::as_f64);
        let raw = score.get("raw").and_then(Value::as_f64);
        if let (Some(mn), Some(mx)) = (min, max)
            && mn > mx
        {
            self.record_violation(
                "xapi",
                "XAPI-SCORE-MIN-MAX",
                "result.score",
                format!("result.score.min ({mn}) > max ({mx})"),
                None,
            );
        }
        if let (Some(mn), Some(r)) = (min, raw)
            && r < mn
        {
            self.record_violation(
                "xapi",
                "XAPI-SCORE-RAW-MIN",
                "result.score.raw",
                format!("result.score.raw ({r}) < min ({mn})"),
                None,
            );
        }
        if let (Some(mx), Some(r)) = (max, raw)
            && r > mx
        {
            self.record_violation(
                "xapi",
                "XAPI-SCORE-RAW-MAX",
                "result.score.raw",
                format!("result.score.raw ({r}) > max ({mx})"),
                None,
            );
        }
    }

    fn validate_session_order(&mut self, session_key: &str, verb: &str, flavor: Flavor) -> bool {
        let mut new_violations: Vec<(&'static str, String, String)> = Vec::new();
        let after_terminated = {
            let session = self.sessions.entry(session_key.to_owned()).or_default();
            if session.terminated {
                new_violations.push((
                    "CMI5-SEQ-AFTER-TERMINATED",
                    "verb.id".to_owned(),
                    format!("{verb}: statement after terminated"),
                ));
                true
            } else {
                if verb == "initialized" {
                    if session.initialized {
                        new_violations.push((
                            "CMI5-SEQ-DUP-INITIALIZED",
                            "verb.id".to_owned(),
                            "repeated verb initialized".to_owned(),
                        ));
                    } else if !session.verbs.is_empty() {
                        new_violations.push((
                            "CMI5-SEQ-INIT-NOT-FIRST",
                            "verb.id".to_owned(),
                            "initialized: not the first statement".to_owned(),
                        ));
                    }
                    session.initialized = true;
                } else if !session.initialized {
                    new_violations.push((
                        "CMI5-SEQ-NOT-INITIALIZED",
                        "verb.id".to_owned(),
                        format!("{verb}: statement before initialized"),
                    ));
                }

                if flavor == Flavor::Cmi5 {
                    match verb {
                        "passed" => {
                            if session.failed {
                                new_violations.push((
                                    "CMI5-9.3.1-PASS-AFTER-FAIL",
                                    "verb.id".to_owned(),
                                    "passed after failed in same session".to_owned(),
                                ));
                            } else if session.passed {
                                new_violations.push((
                                    "CMI5-9.3.1-DUP-PASSED",
                                    "verb.id".to_owned(),
                                    "repeated verb passed".to_owned(),
                                ));
                            }
                            session.passed = true;
                        }
                        "failed" => {
                            if session.passed {
                                new_violations.push((
                                    "CMI5-9.3.1-FAIL-AFTER-PASS",
                                    "verb.id".to_owned(),
                                    "failed after passed in same session".to_owned(),
                                ));
                            } else if session.failed {
                                new_violations.push((
                                    "CMI5-9.3.1-DUP-FAILED",
                                    "verb.id".to_owned(),
                                    "repeated verb failed".to_owned(),
                                ));
                            }
                            session.failed = true;
                        }
                        "completed" => {
                            if session.completed {
                                new_violations.push((
                                    "CMI5-9.3.1-DUP-COMPLETED",
                                    "verb.id".to_owned(),
                                    "repeated verb completed".to_owned(),
                                ));
                            }
                            session.completed = true;
                        }
                        _ => {}
                    }
                } else if !session.verbs.insert(verb.to_owned()) && verb != "answered" {
                    new_violations.push((
                        "XAPI-SEQ-DUP-VERB",
                        "verb.id".to_owned(),
                        format!("repeated verb {verb}"),
                    ));
                }
                session.verbs.insert(verb.to_owned());
                if verb == "terminated" {
                    session.terminated = true;
                }
                false
            }
        };

        let std_name = if flavor == Flavor::Cmi5 {
            "cmi5"
        } else {
            "xapi"
        };
        for (rule, elem, msg) in new_violations {
            self.record_violation(std_name, rule, elem, msg, None);
        }
        after_terminated
    }

    fn validate_cmi5_statement(
        &mut self,
        statement: &Value,
        verb: &str,
        sess_info: Option<&SessionLaunchInfo>,
        reg_info: Option<&RegistrationState>,
    ) {
        self.validate_cmi5_categories(statement, verb);
        self.validate_cmi5_result(statement, verb);

        let mode = sess_info
            .map(|s| s.launch_mode.as_str())
            .or_else(|| reg_info.map(|r| r.launch_mode.as_str()))
            .unwrap_or("Normal");
        if mode != "Normal" && matches!(verb, "completed" | "passed" | "failed") {
            self.record_violation(
                "cmi5",
                "CMI5-MODE-DISALLOWED",
                "verb.id",
                format!("{mode} mode may not send {verb}"),
                None,
            );
        }

        let is_defined = matches!(
            verb,
            "initialized" | "completed" | "passed" | "failed" | "terminated"
        );
        if is_defined {
            let expected_activity = sess_info
                .map(|s| s.activity_id.as_str())
                .or_else(|| reg_info.map(|r| r.activity_id.as_str()));
            if let Some(exp) = expected_activity {
                let got = statement
                    .pointer("/object/id")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if got != exp {
                    self.record_violation(
                        "cmi5",
                        "CMI5-OBJECT-MISMATCH",
                        "object.id",
                        format!("expected launch activityId `{exp}`, got `{got}`"),
                        None,
                    );
                }
            }
            let expected_actor = sess_info
                .and_then(|s| s.actor_ifi.as_deref())
                .or_else(|| reg_info.and_then(|r| r.actor_ifi.as_deref()));
            if let Some(exp_ifi) = expected_actor {
                let got_ifi = statement.get("actor").and_then(canonical_ifi);
                if got_ifi.as_deref() != Some(exp_ifi) {
                    self.record_violation(
                        "cmi5",
                        "CMI5-ACTOR-MISMATCH",
                        "actor",
                        "cmi5 statement actor does not match launch actor IFI",
                        None,
                    );
                }
            }
        }

        if let Some(info) = sess_info
            && !info.grouping_ids.is_empty()
        {
            let got_grouping = grouping_ids(statement);
            for expected_id in &info.grouping_ids {
                if !got_grouping.contains(&expected_id.as_str()) {
                    self.record_violation(
                        "cmi5",
                        "CMI5-GROUPING-MISSING",
                        "context.contextActivities.grouping",
                        format!("missing contextTemplate grouping id `{expected_id}`"),
                        None,
                    );
                }
            }
        }

        let mastery = sess_info
            .and_then(|s| s.mastery_score)
            .or_else(|| reg_info.and_then(|r| r.mastery_score));
        if let Some(m) = mastery {
            self.validate_cmi5_mastery(statement, m, verb);
        }
    }

    fn validate_cmi5_categories(&mut self, statement: &Value, verb: &str) {
        let categories = category_ids(statement);
        let defined = matches!(
            verb,
            "initialized" | "completed" | "passed" | "failed" | "terminated"
        );
        if defined && !categories.contains(&CMI5_CATEGORY) {
            self.record_violation(
                "cmi5",
                "CMI5-CAT-MISSING",
                "context.contextActivities.category",
                format!("{verb}: missing cmi5 category"),
                None,
            );
        }
        if verb == "answered" && categories.contains(&CMI5_CATEGORY) {
            self.record_violation(
                "cmi5",
                "CMI5-CAT-UNEXPECTED",
                "context.contextActivities.category",
                "answered: unexpected cmi5 category",
                None,
            );
        }
        let move_on = matches!(verb, "completed" | "passed" | "failed");
        if move_on && !categories.contains(&MOVEON_CATEGORY) {
            self.record_violation(
                "cmi5",
                "CMI5-MOVEON-MISSING",
                "context.contextActivities.category",
                format!("{verb}: missing moveon category"),
                None,
            );
        }
        if !move_on && categories.contains(&MOVEON_CATEGORY) {
            self.record_violation(
                "cmi5",
                "CMI5-MOVEON-UNEXPECTED",
                "context.contextActivities.category",
                format!("{verb}: unexpected moveon category"),
                None,
            );
        }
    }

    fn validate_cmi5_result(&mut self, statement: &Value, verb: &str) {
        let has_score = statement.pointer("/result/score").is_some();
        let has_success = statement.pointer("/result/success").is_some();
        let has_completion = statement.pointer("/result/completion").is_some();
        let has_duration = statement
            .pointer("/result/duration")
            .and_then(Value::as_str)
            .is_some_and(|s| !s.is_empty());

        match verb {
            "initialized" => {
                if has_score || has_success || has_completion {
                    self.record_violation(
                        "cmi5",
                        "CMI5-RESULT-INITIALIZED",
                        "result",
                        "initialized must not carry score, success, or completion",
                        None,
                    );
                }
            }
            "terminated" => {
                if !has_duration || has_score || has_success || has_completion {
                    self.record_violation(
                        "cmi5",
                        "CMI5-RESULT-TERMINATED",
                        "result",
                        "terminated must have duration and must not carry score, success, or completion",
                        None,
                    );
                }
            }
            "completed" => {
                let comp_true = statement
                    .pointer("/result/completion")
                    .and_then(Value::as_bool)
                    == Some(true);
                if !comp_true || !has_duration || has_score || has_success {
                    self.record_violation(
                        "cmi5",
                        "CMI5-RESULT-COMPLETED",
                        "result",
                        "completed must have completion=true and duration, and must not carry score or success",
                        None,
                    );
                }
            }
            "passed" => {
                let succ_true = statement
                    .pointer("/result/success")
                    .and_then(Value::as_bool)
                    == Some(true);
                if !succ_true || !has_duration || has_completion {
                    self.record_violation(
                        "cmi5",
                        "CMI5-RESULT-PASSED",
                        "result",
                        "passed must have success=true and duration, and must not carry completion",
                        None,
                    );
                }
            }
            "failed" => {
                let succ_false = statement
                    .pointer("/result/success")
                    .and_then(Value::as_bool)
                    == Some(false);
                if !succ_false || !has_duration || has_completion {
                    self.record_violation(
                        "cmi5",
                        "CMI5-RESULT-FAILED",
                        "result",
                        "failed must have success=false and duration, and must not carry completion",
                        None,
                    );
                }
            }
            _ => {}
        }
    }

    fn validate_cmi5_mastery(&mut self, statement: &Value, mastery: f64, verb: &str) {
        if !matches!(verb, "passed" | "failed") {
            return;
        }
        let ext = statement
            .pointer(&format!(
                "/context/extensions/{}",
                escape_pointer(MASTERY_EXTENSION)
            ))
            .and_then(Value::as_f64);
        match ext {
            Some(v) if (v - mastery).abs() <= 1e-9 => {}
            Some(v) => {
                self.record_violation(
                    "cmi5",
                    "CMI5-RESULT-MASTERY-MISMATCH",
                    "context.extensions.masteryscore",
                    format!(
                        "{verb}: masteryscore {v} does not match launch masteryScore {mastery}"
                    ),
                    None,
                );
            }
            None => {
                self.record_violation(
                    "cmi5",
                    "CMI5-RESULT-MASTERY-MISSING",
                    "context.extensions.masteryscore",
                    format!("{verb}: missing masteryscore extension"),
                    None,
                );
            }
        }
        let Some(scaled) = statement
            .pointer("/result/score/scaled")
            .and_then(Value::as_f64)
        else {
            self.record_violation(
                "cmi5",
                "CMI5-RESULT-SCALED-MISSING",
                "result.score.scaled",
                format!("{verb}: missing scaled score"),
                None,
            );
            return;
        };
        let agrees = if verb == "passed" {
            scaled >= mastery
        } else {
            scaled < mastery
        };
        if !agrees {
            self.record_violation(
                "cmi5",
                "CMI5-RESULT-MASTERY-DISAGREES",
                "result.score.scaled",
                format!("{verb}: scaled score {scaled} disagrees with masteryScore {mastery}"),
                None,
            );
        }
    }

    fn validate_registration_cardinality(&mut self, registration: &str, verb: &str) {
        let mut msg: Option<(&'static str, &'static str)> = None;
        if let Some(entry) = self.registrations.get_mut(registration) {
            match verb {
                "completed" => {
                    entry.completed_count = entry.completed_count.saturating_add(1);
                    if entry.completed_count > 1 {
                        msg = Some((
                            "CMI5-REG-DUP-COMPLETED",
                            "second completed for registration",
                        ));
                    }
                }
                "passed" => {
                    if entry.passed {
                        msg = Some(("CMI5-REG-DUP-PASSED", "second passed for registration"));
                    }
                    entry.passed = true;
                }
                "failed" if entry.passed => {
                    msg = Some((
                        "CMI5-REG-FAIL-AFTER-PASS",
                        "failed after passed on registration",
                    ));
                }
                _ => {}
            }
        }
        if let Some((rule, text)) = msg {
            self.record_violation("cmi5", rule, "context.registration", text, None);
        }
    }

    fn record_violation(
        &mut self,
        standard: &'static str,
        rule: &'static str,
        element: impl Into<String>,
        message: impl Into<String>,
        error_code: Option<&str>,
    ) {
        self.violations.push(Violation {
            standard,
            rule,
            call_index: self.call_index,
            element: element.into(),
            message: message.into(),
            error_code: error_code.map(str::to_owned),
        });
    }

    fn record_warning(
        &mut self,
        standard: &'static str,
        rule: &'static str,
        element: impl Into<String>,
        message: impl Into<String>,
        error_code: Option<&str>,
    ) {
        self.warnings.push(Violation {
            standard,
            rule,
            call_index: self.call_index,
            element: element.into(),
            message: message.into(),
            error_code: error_code.map(str::to_owned),
        });
    }
}

/// Extract a canonical Inverse Functional Identifier string from an xAPI `Agent`.
/// Returns `None` if the value is not an `Agent` with exactly one valid IFI.
#[must_use]
pub fn canonical_ifi(agent: &Value) -> Option<String> {
    let obj = agent.as_object()?;
    if let Some(ot) = obj.get("objectType").and_then(Value::as_str)
        && ot != "Agent"
    {
        return None;
    }
    let mut found: Option<String> = None;
    let mut count = 0usize;
    if let Some(mbox) = obj.get("mbox").and_then(Value::as_str)
        && !mbox.is_empty()
    {
        found = Some(format!("mbox:{}", mbox.to_ascii_lowercase()));
        count += 1;
    }
    if let Some(sha) = obj.get("mbox_sha1sum").and_then(Value::as_str)
        && !sha.is_empty()
    {
        found = Some(format!("sha1:{}", sha.to_ascii_lowercase()));
        count += 1;
    }
    if let Some(openid) = obj.get("openid").and_then(Value::as_str)
        && !openid.is_empty()
    {
        found = Some(format!("openid:{openid}"));
        count += 1;
    }
    if let Some(acc) = obj.get("account").and_then(Value::as_object) {
        let hp = acc.get("homePage").and_then(Value::as_str).unwrap_or("");
        let name = acc.get("name").and_then(Value::as_str).unwrap_or("");
        if hp.is_empty() || name.is_empty() {
            return None;
        }
        found = Some(format!("account:{hp}\0{name}"));
        count += 1;
    }
    (count == 1).then_some(found).flatten()
}

fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

fn has_json_content_type(headers: &[(String, String)]) -> bool {
    header_value(headers, "Content-Type").is_some_and(|v| {
        v.trim_start()
            .get(..16)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("application/json"))
    })
}

fn is_valid_xapi_version(v: &str) -> bool {
    matches!(v, "1.0.0" | "1.0.1" | "1.0.2" | "1.0.3")
}

fn is_valid_uuid(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 36 {
        return false;
    }
    for (idx, &byte) in b.iter().enumerate() {
        if matches!(idx, 8 | 13 | 18 | 23) {
            if byte != b'-' {
                return false;
            }
        } else if !byte.is_ascii_hexdigit() {
            return false;
        }
    }
    true
}

fn is_absolute_iri(s: &str) -> bool {
    let Some((scheme, rest)) = s.split_once(':') else {
        return false;
    };
    !scheme.is_empty()
        && !rest.is_empty()
        && scheme
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'))
        && !s.chars().any(char::is_whitespace)
}

fn is_valid_rfc3339(s: &str) -> bool {
    let Some((date, time_and_tz)) = s.split_once('T').or_else(|| s.split_once('t')) else {
        return false;
    };
    let d: Vec<&str> = date.split('-').collect();
    if d.len() != 3
        || d[0].len() != 4
        || d[1].len() != 2
        || d[2].len() != 2
        || !d.iter().all(|p| p.bytes().all(|b| b.is_ascii_digit()))
    {
        return false;
    }
    let time_part = if let Some(t) = time_and_tz
        .strip_suffix('Z')
        .or_else(|| time_and_tz.strip_suffix('z'))
    {
        t
    } else if let Some(pos) = time_and_tz.rfind(['+', '-']) {
        let tz = &time_and_tz[pos + 1..];
        let tz_ok = (tz.len() == 5
            && tz.as_bytes()[2] == b':'
            && tz[..2].bytes().all(|b| b.is_ascii_digit())
            && tz[3..].bytes().all(|b| b.is_ascii_digit()))
            || (tz.len() == 4 && tz.bytes().all(|b| b.is_ascii_digit()));
        if !tz_ok {
            return false;
        }
        &time_and_tz[..pos]
    } else {
        return false;
    };
    let (hms, frac_ok) = match time_part.split_once('.') {
        Some((l, r)) => (l, !r.is_empty() && r.bytes().all(|b| b.is_ascii_digit())),
        None => (time_part, true),
    };
    if !frac_ok {
        return false;
    }
    let t: Vec<&str> = hms.split(':').collect();
    t.len() == 3
        && t.iter()
            .all(|p| p.len() == 2 && p.bytes().all(|b| b.is_ascii_digit()))
}

fn is_valid_xapi_duration(s: &str) -> bool {
    let Some(rest) = s.strip_prefix('P') else {
        return false;
    };
    if rest.is_empty() || rest == "T" {
        return false;
    }
    let (date_part, time_part) = match rest.split_once('T') {
        Some((d, t)) => {
            if t.is_empty() {
                return false;
            }
            (d, Some(t))
        }
        None => (rest, None),
    };
    let date_ok = date_part.is_empty() || parse_xapi_dur_parts(date_part, &['Y', 'M', 'D'], false);
    let time_ok = time_part.is_none_or(|tp| parse_xapi_dur_parts(tp, &['H', 'M', 'S'], true));
    date_ok && time_ok
}

fn parse_xapi_dur_parts(mut s: &str, order: &[char], allow_frac_last: bool) -> bool {
    let mut min_idx = 0usize;
    while !s.is_empty() {
        let Some(pos) = s.find(|c: char| order.contains(&c)) else {
            return false;
        };
        let num = &s[..pos];
        let Some(unit_ch) = s[pos..].chars().next() else {
            return false;
        };
        let Some(unit_idx) = order.iter().position(|&c| c == unit_ch) else {
            return false;
        };
        if unit_idx < min_idx {
            return false;
        }
        min_idx = unit_idx + 1;
        let is_last = unit_idx + 1 == order.len();
        let num_ok = if allow_frac_last && is_last {
            match num.split_once('.') {
                Some((w, f)) => {
                    !w.is_empty()
                        && !f.is_empty()
                        && w.bytes().all(|b| b.is_ascii_digit())
                        && f.bytes().all(|b| b.is_ascii_digit())
                }
                None => !num.is_empty() && num.bytes().all(|b| b.is_ascii_digit()),
            }
        } else {
            !num.is_empty() && num.bytes().all(|b| b.is_ascii_digit())
        };
        if !num_ok {
            return false;
        }
        s = &s[pos + unit_ch.len_utf8()..];
    }
    true
}

fn verb_of(statement: &Value) -> &str {
    statement
        .pointer("/verb/id")
        .and_then(Value::as_str)
        .and_then(|id| id.strip_prefix(VERB_BASE))
        .unwrap_or_default()
}

fn category_ids(statement: &Value) -> Vec<&str> {
    activity_list_ids(statement.pointer("/context/contextActivities/category"))
}

fn grouping_ids(statement: &Value) -> Vec<&str> {
    activity_list_ids(statement.pointer("/context/contextActivities/grouping"))
}

fn activity_list_ids(val: Option<&Value>) -> Vec<&str> {
    let mut out = Vec::new();
    let Some(val) = val else {
        return out;
    };
    match val {
        Value::Array(items) => {
            for item in items {
                if let Some(id) = item.get("id").and_then(Value::as_str) {
                    out.push(id);
                }
            }
        }
        Value::Object(_) => {
            if let Some(id) = val.get("id").and_then(Value::as_str) {
                out.push(id);
            }
        }
        _ => {}
    }
    out
}

fn escape_pointer(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

fn query_map(query: &str) -> BTreeMap<String, String> {
    let query = query.strip_prefix('?').unwrap_or(query);
    let mut out = BTreeMap::new();
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let key = percent_decode(key);
        if key.is_empty() {
            continue;
        }
        out.insert(key, percent_decode(value));
    }
    out
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while let Some(byte) = bytes.get(index).copied() {
        if byte == b'+' {
            out.push(b' ');
            index += 1;
        } else if byte == b'%' {
            match (bytes.get(index + 1), bytes.get(index + 2)) {
                (Some(high), Some(low)) => match (hex_val(*high), hex_val(*low)) {
                    (Some(h), Some(l)) => {
                        out.push(h * 16 + l);
                        index += 3;
                    }
                    _ => {
                        out.push(b'%');
                        index += 1;
                    }
                },
                _ => {
                    out.push(b'%');
                    index += 1;
                }
            }
        } else {
            out.push(byte);
            index += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Build a standard cmi5 `LMS.LaunchData` JSON document.
#[must_use]
pub fn build_cmi5_launch_data(
    activity_id: &str,
    session_id: &str,
    launch_mode: &str,
    mastery_score: Option<f64>,
    return_url: Option<&str>,
) -> Value {
    let mut doc = Map::new();
    doc.insert(
        "contextTemplate".to_owned(),
        json!({
            "contextActivities": {
                "grouping": [ { "id": activity_id } ],
            },
            "extensions": { SESSION_ID_EXTENSION: session_id },
        }),
    );
    doc.insert("launchMode".to_owned(), json!(launch_mode));
    doc.insert(
        "moveOn".to_owned(),
        json!(if mastery_score.is_some() {
            "Passed"
        } else {
            "Completed"
        }),
    );
    doc.insert(
        "entitlementKey".to_owned(),
        json!({ "alternate": "coursecraft-mock" }),
    );
    if let Some(score) = mastery_score {
        doc.insert("masteryScore".to_owned(), json!(score));
    }
    if let Some(url) = return_url {
        doc.insert("returnURL".to_owned(), json!(url));
    }
    Value::Object(doc)
}
