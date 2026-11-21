//! Local strict LTI 1.3 platform for automated conformance testing.
//!
//! Speaks real HTTP on `127.0.0.1:0` and validates every OIDC login redirect,
//! OAuth `client_credentials` token assertion, and AGS score POST against the
//! LTI 1.3 Core & Assignment and Grade Services specifications.

use std::collections::{BTreeMap, HashSet, VecDeque};
use std::net::{SocketAddr, TcpListener};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use coursecraft_lti::{LtiError, jwt};
use ring::signature::RsaKeyPair;
use serde::Serialize;
use serde_json::{Value, json};

use crate::http::{Request, Response, parse_form, read_request};

const TEST_KEY: &[u8] = include_bytes!("../tests/fixtures/test_key.pk8");
const SCORE_CONTENT_TYPE: &str = "application/vnd.ims.lis.v1.score+json";
const ASSERTION_TYPE: &str = "urn:ietf:params:oauth:client-assertion-type:jwt-bearer";

/// A structured LTI 1.3 specification violation recorded by [`StrictLtiPlatform`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Violation {
    pub standard: &'static str,
    pub rule: &'static str,
    pub call_index: usize,
    pub element: String,
    pub message: String,
    pub error_code: Option<String>,
}

/// An AGS score POST received by [`StrictLtiPlatform`].
#[derive(Clone, Debug, PartialEq)]
pub struct AgsScoreRecord {
    pub path: String,
    pub query: BTreeMap<String, String>,
    pub payload: Value,
}

struct State {
    client_id: String,
    active_kid: String,
    tool_jwks_url: Option<String>,
    call_counter: usize,
    jwks_requests: usize,
    oidc_requests: usize,
    token_requests: usize,
    last_oidc_params: Option<BTreeMap<String, String>>,
    seen_jtis: HashSet<String>,
    issued_tokens: HashSet<String>,
    queued_score_statuses: VecDeque<u16>,
    scores: Vec<AgsScoreRecord>,
    violations: Vec<Violation>,
}

/// A local strict LTI 1.3 platform running on `127.0.0.1:0`.
pub struct StrictLtiPlatform {
    addr: SocketAddr,
    state: Arc<Mutex<State>>,
    shutdown: Arc<AtomicBool>,
}

impl Drop for StrictLtiPlatform {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }
}

impl StrictLtiPlatform {
    /// Start a strict platform bound to `127.0.0.1:0` for `client_id`.
    ///
    /// # Errors
    /// Returns an I/O error if binding the loopback listener fails.
    pub fn start(client_id: impl Into<String>) -> std::io::Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let addr = listener.local_addr()?;
        let state = Arc::new(Mutex::new(State {
            client_id: client_id.into(),
            active_kid: "kid-1".into(),
            tool_jwks_url: None,
            call_counter: 0,
            jwks_requests: 0,
            oidc_requests: 0,
            token_requests: 0,
            last_oidc_params: None,
            seen_jtis: HashSet::new(),
            issued_tokens: HashSet::new(),
            queued_score_statuses: VecDeque::new(),
            scores: Vec::new(),
            violations: Vec::new(),
        }));
        let shutdown = Arc::new(AtomicBool::new(false));
        let thread_state = Arc::clone(&state);
        let thread_shutdown = Arc::clone(&shutdown);
        std::thread::spawn(move || {
            while !thread_shutdown.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let st = Arc::clone(&thread_state);
                        std::thread::spawn(move || {
                            let _ = stream.set_nonblocking(false);
                            let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                            let Ok(request) = read_request(&mut stream) else {
                                return;
                            };
                            let response = handle_request(&st, addr, &request);
                            let _ = response.write(&mut stream);
                        });
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            addr,
            state,
            shutdown,
        })
    }

    #[must_use]
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    #[must_use]
    pub fn issuer(&self) -> String {
        format!("http://{}", self.addr)
    }

    #[must_use]
    pub fn jwks_url(&self) -> String {
        format!("http://{}/jwks", self.addr)
    }

    #[must_use]
    pub fn auth_login_url(&self) -> String {
        format!("http://{}/oidc/auth", self.addr)
    }

    #[must_use]
    pub fn auth_token_url(&self) -> String {
        format!("http://{}/oauth/token", self.addr)
    }

    #[must_use]
    pub fn lineitem_url(&self, id: &str) -> String {
        format!("http://{}/ags/lineitems/{id}", self.addr)
    }

    pub fn set_tool_jwks_url(&self, url: impl Into<String>) {
        lock(&self.state).tool_jwks_url = Some(url.into());
    }

    pub fn rotate_key(&self, new_kid: impl Into<String>) {
        lock(&self.state).active_kid = new_kid.into();
    }

    #[must_use]
    pub fn active_kid(&self) -> String {
        lock(&self.state).active_kid.clone()
    }

    pub fn inject_score_status(&self, status: u16) {
        lock(&self.state).queued_score_statuses.push_back(status);
    }

    /// Sign an `id_token` with the currently active `kid`.
    ///
    /// # Errors
    /// Returns [`LtiError`] if RSA signing fails.
    pub fn sign_id_token(&self, claims: &Value) -> Result<String, LtiError> {
        let kid = self.active_kid();
        self.sign_id_token_with_kid(&kid, claims)
    }

    /// Sign an `id_token` with an explicit `kid`.
    ///
    /// # Errors
    /// Returns [`LtiError`] if RSA signing fails.
    pub fn sign_id_token_with_kid(&self, kid: &str, claims: &Value) -> Result<String, LtiError> {
        let header = json!({"alg": "RS256", "typ": "JWT", "kid": kid});
        jwt::sign_rs256(TEST_KEY, &header, claims)
    }

    /// Build a valid LTI 1.3 `LtiResourceLinkRequest` claims payload.
    #[must_use]
    pub fn default_claims(
        &self,
        nonce: &str,
        now_unix: i64,
        target_link_uri: &str,
        lineitem: Option<&str>,
        scopes: &[&str],
    ) -> Value {
        let client_id = lock(&self.state).client_id.clone();
        let mut claims = json!({
            "iss": self.issuer(),
            "aud": client_id,
            "sub": "user-1",
            "name": "Ada Lovelace",
            "exp": now_unix + 600,
            "iat": now_unix - 10,
            "nonce": nonce,
            "https://purl.imsglobal.org/spec/lti/claim/message_type": "LtiResourceLinkRequest",
            "https://purl.imsglobal.org/spec/lti/claim/version": "1.3.0",
            "https://purl.imsglobal.org/spec/lti/claim/deployment_id": "deployment-1",
            "https://purl.imsglobal.org/spec/lti/claim/target_link_uri": target_link_uri,
            "https://purl.imsglobal.org/spec/lti/claim/context": {"id": "context-1"},
            "https://purl.imsglobal.org/spec/lti/claim/resource_link": {"id": "link-1"},
            "https://purl.imsglobal.org/spec/lti/claim/roles": [
                "http://purl.imsglobal.org/vocab/lis/v2/membership#Learner"
            ],
        });
        if let Some(lineitem) = lineitem {
            claims["https://purl.imsglobal.org/spec/lti-ags/claim/endpoint"] = json!({
                "lineitem": lineitem,
                "lineitems": format!("http://{}/ags/lineitems", self.addr),
                "scope": scopes,
            });
        }
        claims
    }

    #[must_use]
    pub fn violations(&self) -> Vec<Violation> {
        lock(&self.state).violations.clone()
    }

    #[must_use]
    pub fn scores(&self) -> Vec<AgsScoreRecord> {
        lock(&self.state).scores.clone()
    }

    #[must_use]
    pub fn token_requests_count(&self) -> usize {
        lock(&self.state).token_requests
    }

    #[must_use]
    pub fn jwks_requests_count(&self) -> usize {
        lock(&self.state).jwks_requests
    }

    #[must_use]
    pub fn oidc_requests_count(&self) -> usize {
        lock(&self.state).oidc_requests
    }

    #[must_use]
    pub fn last_oidc_params(&self) -> Option<BTreeMap<String, String>> {
        lock(&self.state).last_oidc_params.clone()
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn unix_now() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => i64::try_from(d.as_secs()).unwrap_or(i64::MAX),
        Err(_) => 0,
    }
}

fn record_violation(
    state: &Arc<Mutex<State>>,
    call_index: usize,
    rule: &'static str,
    element: impl Into<String>,
    message: impl Into<String>,
    error_code: Option<&str>,
) {
    lock(state).violations.push(Violation {
        standard: "lti13",
        rule,
        call_index,
        element: element.into(),
        message: message.into(),
        error_code: error_code.map(str::to_string),
    });
}

fn handle_request(state: &Arc<Mutex<State>>, addr: SocketAddr, request: &Request) -> Response {
    let call_index = {
        let mut guard = lock(state);
        guard.call_counter += 1;
        guard.call_counter
    };

    if request.method == "GET" && request.path == "/jwks" {
        return handle_jwks(state, call_index);
    }
    if request.method == "GET" && request.path == "/oidc/auth" {
        return handle_oidc_auth(state, call_index, request);
    }
    if request.method == "POST" && request.path == "/oauth/token" {
        return handle_oauth_token(state, addr, call_index, request);
    }
    if request.method == "POST" && request.path.starts_with("/ags/lineitems/") {
        return handle_ags_score(state, call_index, request);
    }
    Response::text(404, "not found")
}

fn handle_jwks(state: &Arc<Mutex<State>>, call_index: usize) -> Response {
    let active_kid = {
        let mut guard = lock(state);
        guard.jwks_requests += 1;
        guard.active_kid.clone()
    };
    let Ok(pair) = RsaKeyPair::from_pkcs8(TEST_KEY) else {
        record_violation(
            state,
            call_index,
            "LTI13-PLATFORM-KEY",
            "/jwks",
            "failed to parse platform RSA key",
            Some("500"),
        );
        return Response::text(500, "invalid platform key");
    };
    let public = ring::rsa::PublicKeyComponents::<Vec<u8>>::from(pair.public());
    let jwks = json!({
        "keys": [{
            "kty": "RSA",
            "alg": "RS256",
            "use": "sig",
            "kid": active_kid,
            "n": jwt::base64url_encode(&public.n),
            "e": jwt::base64url_encode(&public.e),
        }]
    });
    Response::json(200, &jwks)
}

fn handle_oidc_auth(state: &Arc<Mutex<State>>, call_index: usize, request: &Request) -> Response {
    let expected_client_id = {
        let mut guard = lock(state);
        guard.oidc_requests += 1;
        guard.last_oidc_params = Some(request.query.clone());
        guard.client_id.clone()
    };

    let exact_checks = [
        ("scope", "openid"),
        ("response_type", "id_token"),
        ("response_mode", "form_post"),
        ("prompt", "none"),
        ("client_id", expected_client_id.as_str()),
    ];
    for (param, expected) in exact_checks {
        if request.query.get(param).map(String::as_str) != Some(expected) {
            record_violation(
                state,
                call_index,
                "LTI13-OIDC-PARAM",
                param,
                format!(
                    "expected {param}={expected:?}, got {:?}",
                    request.query.get(param)
                ),
                Some("400"),
            );
            return Response::text(400, format!("invalid {param}"));
        }
    }
    for param in ["redirect_uri", "state", "nonce", "login_hint"] {
        if request
            .query
            .get(param)
            .is_none_or(|value| value.is_empty())
        {
            record_violation(
                state,
                call_index,
                "LTI13-OIDC-PARAM",
                param,
                format!("missing or empty {param}"),
                Some("400"),
            );
            return Response::text(400, format!("missing {param}"));
        }
    }
    Response::text(200, "oidc ok")
}

fn handle_oauth_token(
    state: &Arc<Mutex<State>>,
    addr: SocketAddr,
    call_index: usize,
    request: &Request,
) -> Response {
    let (expected_client_id, tool_jwks_url) = {
        let mut guard = lock(state);
        guard.token_requests += 1;
        (guard.client_id.clone(), guard.tool_jwks_url.clone())
    };
    let Some(body) = request.body_text() else {
        record_violation(
            state,
            call_index,
            "LTI13-OAUTH-BODY",
            "body",
            "token request body is not UTF-8",
            Some("400"),
        );
        return Response::text(400, "invalid body");
    };
    let form = parse_form(body);
    if form.get("grant_type").map(String::as_str) != Some("client_credentials") {
        record_violation(
            state,
            call_index,
            "LTI13-OAUTH-GRANT",
            "grant_type",
            format!("unexpected grant_type {:?}", form.get("grant_type")),
            Some("400"),
        );
        return Response::text(400, "invalid grant_type");
    }
    if form.get("client_assertion_type").map(String::as_str) != Some(ASSERTION_TYPE) {
        record_violation(
            state,
            call_index,
            "LTI13-OAUTH-ASSERTION-TYPE",
            "client_assertion_type",
            format!(
                "unexpected client_assertion_type {:?}",
                form.get("client_assertion_type")
            ),
            Some("400"),
        );
        return Response::text(400, "invalid client_assertion_type");
    }
    let Some(scope) = form.get("scope").filter(|s| !s.is_empty()).cloned() else {
        record_violation(
            state,
            call_index,
            "LTI13-OAUTH-SCOPE",
            "scope",
            "missing or empty scope",
            Some("400"),
        );
        return Response::text(400, "missing scope");
    };
    let Some(assertion) = form.get("client_assertion") else {
        record_violation(
            state,
            call_index,
            "LTI13-OAUTH-ASSERTION",
            "client_assertion",
            "missing client_assertion",
            Some("400"),
        );
        return Response::text(400, "missing client_assertion");
    };
    let Some(jwks_url) = tool_jwks_url else {
        record_violation(
            state,
            call_index,
            "LTI13-TOOL-JWKS",
            "tool_jwks_url",
            "tool_jwks_url not configured on StrictLtiPlatform",
            Some("500"),
        );
        return Response::text(500, "tool_jwks_url not configured");
    };
    let Ok(jwks_text) = fetch_text(&jwks_url) else {
        record_violation(
            state,
            call_index,
            "LTI13-TOOL-JWKS",
            "tool_jwks_url",
            format!("failed to fetch tool JWKS from {jwks_url}"),
            Some("502"),
        );
        return Response::text(502, "failed to fetch tool JWKS");
    };
    let expected_aud = format!("http://{addr}/oauth/token");
    if let Err((rule, element, message)) = verify_client_assertion(
        state,
        assertion,
        &jwks_text,
        &expected_client_id,
        &expected_aud,
    ) {
        record_violation(state, call_index, rule, element, message, Some("400"));
        return Response::text(400, "invalid client_assertion");
    }

    static TOKEN_SEQ: AtomicUsize = AtomicUsize::new(1);
    let token = format!("lti-tok-{}", TOKEN_SEQ.fetch_add(1, Ordering::SeqCst));
    lock(state).issued_tokens.insert(token.clone());
    Response::json(
        200,
        &json!({
            "access_token": token,
            "token_type": "Bearer",
            "expires_in": 3600,
            "scope": scope,
        }),
    )
}

fn verify_client_assertion(
    state: &Arc<Mutex<State>>,
    assertion: &str,
    tool_jwks: &str,
    expected_client_id: &str,
    expected_aud: &str,
) -> Result<(), (&'static str, &'static str, String)> {
    let (header_b64, payload_b64, sig_b64) = jwt::split_jwt(assertion)
        .map_err(|e| ("LTI13-JWT-FORMAT", "client_assertion", e.to_string()))?;
    let header_bytes = jwt::base64url_decode(header_b64)
        .map_err(|e| ("LTI13-JWT-HEADER", "header", e.to_string()))?;
    let header: Value = serde_json::from_slice(&header_bytes)
        .map_err(|e| ("LTI13-JWT-HEADER", "header", e.to_string()))?;
    if header.get("alg").and_then(Value::as_str) != Some("RS256") {
        return Err((
            "LTI13-JWT-ALG",
            "header.alg",
            format!("expected RS256, got {:?}", header.get("alg")),
        ));
    }
    let kid = header.get("kid").and_then(Value::as_str);
    let jwks: Value =
        serde_json::from_str(tool_jwks).map_err(|e| ("LTI13-TOOL-JWKS", "jwks", e.to_string()))?;
    let keys = jwks.get("keys").and_then(Value::as_array).ok_or((
        "LTI13-TOOL-JWKS",
        "jwks.keys",
        "missing keys array".into(),
    ))?;
    let jwk = keys
        .iter()
        .find(|k| kid.is_none_or(|want| k.get("kid").and_then(Value::as_str) == Some(want)))
        .ok_or((
            "LTI13-TOOL-JWKS",
            "jwks.keys",
            format!("kid {kid:?} not found"),
        ))?;
    let n = jwk.get("n").and_then(Value::as_str).ok_or((
        "LTI13-TOOL-JWKS",
        "jwk.n",
        "missing n".into(),
    ))?;
    let e = jwk.get("e").and_then(Value::as_str).ok_or((
        "LTI13-TOOL-JWKS",
        "jwk.e",
        "missing e".into(),
    ))?;
    let n_bytes =
        jwt::base64url_decode(n).map_err(|err| ("LTI13-TOOL-JWKS", "jwk.n", err.to_string()))?;
    let e_bytes =
        jwt::base64url_decode(e).map_err(|err| ("LTI13-TOOL-JWKS", "jwk.e", err.to_string()))?;
    let sig_bytes = jwt::base64url_decode(sig_b64)
        .map_err(|err| ("LTI13-JWT-SIG", "signature", err.to_string()))?;
    let signing_input = format!("{header_b64}.{payload_b64}");
    jwt::verify_rs256(&n_bytes, &e_bytes, signing_input.as_bytes(), &sig_bytes)
        .map_err(|err| ("LTI13-JWT-SIG", "signature", err.to_string()))?;

    let payload_bytes = jwt::base64url_decode(payload_b64)
        .map_err(|err| ("LTI13-JWT-PAYLOAD", "payload", err.to_string()))?;
    let claims: Value = serde_json::from_slice(&payload_bytes)
        .map_err(|err| ("LTI13-JWT-PAYLOAD", "payload", err.to_string()))?;

    if claims.get("iss").and_then(Value::as_str) != Some(expected_client_id) {
        return Err((
            "LTI13-JWT-ISS",
            "iss",
            format!(
                "expected {expected_client_id:?}, got {:?}",
                claims.get("iss")
            ),
        ));
    }
    if claims.get("sub").and_then(Value::as_str) != Some(expected_client_id) {
        return Err((
            "LTI13-JWT-SUB",
            "sub",
            format!(
                "expected {expected_client_id:?}, got {:?}",
                claims.get("sub")
            ),
        ));
    }
    if claims.get("aud").and_then(Value::as_str) != Some(expected_aud) {
        return Err((
            "LTI13-JWT-AUD",
            "aud",
            format!("expected {expected_aud:?}, got {:?}", claims.get("aud")),
        ));
    }
    let iat = claims.get("iat").and_then(Value::as_i64).ok_or((
        "LTI13-JWT-EXP",
        "iat",
        "missing iat".into(),
    ))?;
    let exp = claims.get("exp").and_then(Value::as_i64).ok_or((
        "LTI13-JWT-EXP",
        "exp",
        "missing exp".into(),
    ))?;
    let now = unix_now();
    if exp <= iat || exp - iat > 300 || exp <= now {
        return Err((
            "LTI13-JWT-EXP",
            "exp",
            format!("invalid lifetime iat={iat}, exp={exp}, now={now}"),
        ));
    }
    let jti = claims
        .get("jti")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or(("LTI13-JWT-JTI", "jti", "missing or empty jti".into()))?;
    let mut guard = lock(state);
    if !guard.seen_jtis.insert(jti.to_string()) {
        return Err(("LTI13-JWT-JTI", "jti", format!("reused jti {jti:?}")));
    }
    Ok(())
}

fn handle_ags_score(state: &Arc<Mutex<State>>, call_index: usize, request: &Request) -> Response {
    if let Some(status) = lock(state).queued_score_statuses.pop_front() {
        return Response::text(status, format!("injected status {status}"));
    }
    if request.path.contains("//") {
        record_violation(
            state,
            call_index,
            "LTI13-AGS-DOUBLE-SLASH",
            "url",
            format!("AGS score URL contains '//': {}", request.path),
            Some("400"),
        );
        return Response::text(400, "double slash in AGS URL");
    }
    if !request.path.ends_with("/scores") {
        return Response::text(404, "not found");
    }

    let auth = request.headers.get("authorization").map(String::as_str);
    let token = auth.and_then(|v| v.strip_prefix("Bearer "));
    let token_ok = token.is_some_and(|t| lock(state).issued_tokens.contains(t));
    if !token_ok {
        record_violation(
            state,
            call_index,
            "LTI13-AGS-AUTH",
            "authorization",
            "missing or unknown Bearer token",
            Some("401"),
        );
        return Response::text(401, "unauthorized");
    }

    let content_type = request
        .headers
        .get("content-type")
        .map(String::as_str)
        .unwrap_or("");
    let media_type = content_type.split(';').next().map(str::trim).unwrap_or("");
    if !media_type.eq_ignore_ascii_case(SCORE_CONTENT_TYPE) {
        record_violation(
            state,
            call_index,
            "LTI13-AGS-CONTENT-TYPE",
            "content-type",
            format!("expected {SCORE_CONTENT_TYPE}, got {content_type:?}"),
            Some("415"),
        );
        return Response::text(415, "unsupported media type");
    }

    let payload: Value = match serde_json::from_slice(&request.body) {
        Ok(value) => value,
        Err(err) => {
            record_violation(
                state,
                call_index,
                "LTI13-AGS-JSON",
                "body",
                format!("invalid JSON: {err}"),
                Some("400"),
            );
            return Response::text(400, "invalid JSON");
        }
    };

    if let Err((rule, element, message)) = validate_score_payload(&payload) {
        record_violation(state, call_index, rule, element, message, Some("400"));
        return Response::text(400, "invalid AGS score payload");
    }

    lock(state).scores.push(AgsScoreRecord {
        path: request.path.clone(),
        query: request.query.clone(),
        payload,
    });
    Response::status(204)
}

fn validate_score_payload(payload: &Value) -> Result<(), (&'static str, &'static str, String)> {
    if payload
        .get("userId")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
    {
        return Err((
            "LTI13-AGS-USERID",
            "userId",
            "missing or empty userId".into(),
        ));
    }
    let activity = payload.get("activityProgress").and_then(Value::as_str);
    if !matches!(
        activity,
        Some("Initialized" | "Started" | "InProgress" | "Submitted" | "Completed")
    ) {
        return Err((
            "LTI13-AGS-ACTIVITY-PROGRESS",
            "activityProgress",
            format!("invalid activityProgress {activity:?}"),
        ));
    }
    let grading = payload.get("gradingProgress").and_then(Value::as_str);
    if !matches!(
        grading,
        Some("FullyGraded" | "Pending" | "PendingManual" | "Failed" | "NotReady")
    ) {
        return Err((
            "LTI13-AGS-GRADING-PROGRESS",
            "gradingProgress",
            format!("invalid gradingProgress {grading:?}"),
        ));
    }
    let ts = payload
        .get("timestamp")
        .and_then(Value::as_str)
        .unwrap_or("");
    if !is_rfc3339_like(ts) {
        return Err((
            "LTI13-AGS-TIMESTAMP",
            "timestamp",
            format!("invalid RFC 3339 timestamp {ts:?}"),
        ));
    }
    match (payload.get("scoreGiven"), payload.get("scoreMaximum")) {
        (Some(given), Some(max)) => {
            let Some(g) = given.as_f64() else {
                return Err((
                    "LTI13-AGS-SCORE",
                    "scoreGiven",
                    "scoreGiven is not a number".into(),
                ));
            };
            let Some(m) = max.as_f64() else {
                return Err((
                    "LTI13-AGS-SCORE",
                    "scoreMaximum",
                    "scoreMaximum is not a number".into(),
                ));
            };
            if g < 0.0 || m <= 0.0 {
                return Err((
                    "LTI13-AGS-SCORE",
                    "scoreGiven",
                    format!("invalid score range scoreGiven={g}, scoreMaximum={m}"),
                ));
            }
        }
        (Some(_), None) => {
            return Err((
                "LTI13-AGS-SCORE",
                "scoreMaximum",
                "scoreGiven requires scoreMaximum".into(),
            ));
        }
        (None, Some(_)) => {
            return Err((
                "LTI13-AGS-SCORE",
                "scoreMaximum",
                "scoreMaximum requires scoreGiven".into(),
            ));
        }
        (None, None) => {}
    }
    Ok(())
}

fn is_rfc3339_like(ts: &str) -> bool {
    let Some((date, time)) = ts.split_once('T') else {
        return false;
    };
    let date_parts: Vec<&str> = date.split('-').collect();
    if date_parts.len() != 3
        || date_parts[0].len() != 4
        || date_parts[1].len() != 2
        || date_parts[2].len() != 2
        || !date_parts
            .iter()
            .all(|part| part.bytes().all(|b| b.is_ascii_digit()))
    {
        return false;
    }
    time.ends_with('Z') && time.len() >= 9
}

fn fetch_text(url: &str) -> Result<String, String> {
    let mut response = ureq::get(url)
        .config()
        .http_status_as_error(false)
        .build()
        .call()
        .map_err(|e| e.to_string())?;
    let bytes = response
        .body_mut()
        .with_config()
        .limit(1024 * 1024)
        .read_to_vec()
        .map_err(|e| e.to_string())?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}
