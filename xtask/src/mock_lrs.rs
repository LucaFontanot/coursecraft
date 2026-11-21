//! A small mock LMS and LRS for testing xAPI / cmi5 packages.
//!
//! The server serves the published packages under `target/mock-lrs/`, hands out
//! cmi5 fetch tokens, and delegates xAPI / cmi5 validation and State API
//! storage to [`coursecraft_strict_lms::StrictLrs`].
//!
//! Everything is test tooling. It binds to `127.0.0.1` only, closes each
//! connection after one request, and never panics on a bad request.

use std::collections::{BTreeMap, HashMap};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use coursecraft_strict_lms::{Flavor, LaunchSpec, LrsResponse, StrictLrs, build_cmi5_launch_data};
use serde_json::{Value, json};

const MAX_BODY: usize = 16 * 1024 * 1024;
const MAX_HEADER: usize = 64 * 1024;
const READ_TIMEOUT: Duration = Duration::from_secs(10);

#[cfg(test)]
mod tests;

/// One HTTP request, already parsed from the socket.
#[derive(Debug, Clone)]
pub struct Request {
    /// Uppercase HTTP method.
    pub method: String,
    /// Path without the query string.
    pub path: String,
    /// Query string without the leading `?`.
    pub query: String,
    /// Header name/value pairs.
    pub headers: Vec<(String, String)>,
    /// Raw request body.
    pub body: Vec<u8>,
}

/// One HTTP response.
#[derive(Debug, Clone)]
pub struct Response {
    /// Status code.
    pub status: u16,
    /// Header name/value pairs.
    pub headers: Vec<(String, String)>,
    /// Raw body.
    pub body: Vec<u8>,
}

impl Response {
    pub(crate) fn text(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            headers: vec![(
                "Content-Type".to_string(),
                "text/plain; charset=utf-8".to_string(),
            )],
            body: body.into().into_bytes(),
        }
    }

    pub(crate) fn json(status: u16, value: &Value) -> Self {
        Self {
            status,
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body: serde_json::to_vec(value).unwrap_or_default(),
        }
    }

    pub(crate) fn empty(status: u16) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    fn from_lrs(resp: LrsResponse) -> Self {
        Self {
            status: resp.status,
            headers: resp.headers,
            body: resp.body,
        }
    }
}

/// Everything the mock server remembers between requests.
pub struct LrsStore {
    port: u16,
    base_dir: PathBuf,
    registrations: HashMap<(String, &'static str), String>,
    pub(crate) strict: StrictLrs,
    fail_remaining: u32,
    fail_status: u16,
    next_session: u64,
    next_fetch: u64,
}

impl LrsStore {
    /// Create an empty store for a server on `port` serving `base_dir`.
    #[must_use]
    pub fn new(port: u16, base_dir: PathBuf) -> Self {
        let mut strict = StrictLrs::new();
        strict.add_token("Basic mock-xapi".to_string());
        Self {
            port,
            base_dir,
            registrations: HashMap::new(),
            strict,
            fail_remaining: 0,
            fail_status: 500,
            next_session: 0,
            next_fetch: 0,
        }
    }
}

/// Run the server until the process ends.
///
/// # Errors
/// Returns an error if the listener cannot bind.
pub fn run(port: u16) -> Result<(), String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("xtask has no parent directory")?;
    let base_dir = root.join("target/mock-lrs");
    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|err| format!("bind 127.0.0.1:{port}: {err}"))?;
    let local = listener
        .local_addr()
        .map_err(|err| format!("local_addr: {err}"))?;
    let store = Arc::new(Mutex::new(LrsStore::new(local.port(), base_dir)));
    println!("mock-lrs: http://127.0.0.1:{}/", local.port());

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let store = Arc::clone(&store);
                std::thread::spawn(move || serve(stream, &store));
            }
            Err(err) => eprintln!("mock-lrs: accept: {err}"),
        }
    }
    Ok(())
}

fn serve(mut stream: TcpStream, store: &Arc<Mutex<LrsStore>>) {
    let _ = stream.set_read_timeout(Some(READ_TIMEOUT));
    let _ = stream.set_write_timeout(Some(READ_TIMEOUT));
    let response = match read_request(&mut stream) {
        Ok(request) => {
            let mut guard = store
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            handle(&mut guard, &request)
        }
        Err(response) => response,
    };
    let _ = write_response(&mut stream, &response);
}

/// Handle one request. Pure: it never touches a socket, so tests can drive it.
pub fn handle(store: &mut LrsStore, request: &Request) -> Response {
    let path = request.path.as_str();
    match path {
        "/mock/launch" => launch(store, request),
        "/mock/reset" => reset(store, request),
        "/mock/log" => log(store, request),
        "/mock/fail" => fail(store, request),
        "/mock/returned" => returned(request),
        _ => {
            if let Some(id) = path.strip_prefix("/fetch/") {
                return Response::from_lrs(store.strict.handle_fetch(
                    &request.method,
                    id,
                    &request.headers,
                ));
            }
            // Only the LRS resources: xAPI packages are also served under `/xapi/`.
            if let Some(rest) = path.strip_prefix("/xapi/")
                && matches!(rest, "statements" | "activities/state")
            {
                return xapi(store, request, rest);
            }
            static_file(store, path)
        }
    }
}

fn is_method(request: &Request, method: &str) -> bool {
    request.method.eq_ignore_ascii_case(method)
}

fn launch(store: &mut LrsStore, request: &Request) -> Response {
    if !is_method(request, "POST") {
        return Response::text(405, "Method Not Allowed");
    }
    let params = query_map(&request.query);
    let flavor = match params.get("flavor").map(String::as_str) {
        Some("cmi5") => Flavor::Cmi5,
        Some("xapi") => Flavor::Xapi,
        _ => return Response::text(400, "flavor must be cmi5 or xapi"),
    };
    let course = match params.get("course").map(String::as_str) {
        Some("showcase") => "showcase",
        Some("hello") => "hello",
        _ => return Response::text(400, "course must be showcase or hello"),
    };
    let mode = params.get("mode").map(String::as_str).unwrap_or("Normal");
    if !matches!(mode, "Normal" | "Browse" | "Review") {
        return Response::text(400, "mode must be Normal, Browse or Review");
    }
    let mastery = match params.get("mastery") {
        Some(text) => match text.parse::<f64>() {
            Ok(value) if value.is_finite() && (0.0..=1.0).contains(&value) => Some(value),
            _ => return Response::text(400, "mastery must be a number in 0..=1"),
        },
        None => None,
    };
    let want_return = params.get("return").is_some_and(|value| value == "1");

    let registration = registration_for(store, course, flavor);
    let base = format!("http://127.0.0.1:{}", store.port);
    let endpoint = format!("{base}/xapi/");
    let activity = format!("urn:coursecraft:mock:activity:{course}");
    let actor = actor_json();
    let actor_text = actor.to_string();

    store.next_session = store.next_session.saturating_add(1);
    let session_id = format!("mock-session-{}", store.next_session);

    match flavor {
        Flavor::Cmi5 => {
            store.next_fetch = store.next_fetch.saturating_add(1);
            let fetch_id = format!("mock-fetch-{}", store.next_fetch);
            let fetch_url = format!("{base}/fetch/{fetch_id}");
            let return_url = want_return.then(|| format!("{base}/mock/returned"));
            let launch_doc = build_cmi5_launch_data(
                &activity,
                &session_id,
                mode,
                mastery,
                return_url.as_deref(),
            );
            store.strict.register_launch(
                LaunchSpec {
                    flavor: Flavor::Cmi5,
                    activity_id: activity.clone(),
                    actor,
                    registration: Some(registration.clone()),
                    launch_mode: mode.to_string(),
                    mastery_score: mastery,
                    session_id,
                    grouping_ids: vec![activity.clone()],
                    fetch_id: Some(fetch_id),
                    auth_token: None,
                },
                Some(launch_doc),
            );
            let url = format!(
                "{base}/cmi5/{course}/index.html?endpoint={}&actor={}&registration={}&activityId={}&fetch={}",
                encode(&endpoint),
                encode(&actor_text),
                encode(&registration),
                encode(&activity),
                encode(&fetch_url),
            );
            Response::json(200, &json!({ "url": url }))
        }
        Flavor::Xapi => {
            store.strict.register_launch(
                LaunchSpec {
                    flavor: Flavor::Xapi,
                    activity_id: activity.clone(),
                    actor,
                    registration: Some(registration.clone()),
                    launch_mode: mode.to_string(),
                    mastery_score: mastery,
                    session_id,
                    grouping_ids: Vec::new(),
                    fetch_id: None,
                    auth_token: Some("Basic mock-xapi".to_string()),
                },
                None,
            );
            let url = format!(
                "{base}/xapi/{course}/index.html?endpoint={}&actor={}&registration={}&activity_id={}&auth={}",
                encode(&endpoint),
                encode(&actor_text),
                encode(&registration),
                encode(&activity),
                encode("Basic mock-xapi"),
            );
            Response::json(200, &json!({ "url": url }))
        }
    }
}

fn registration_for(store: &mut LrsStore, course: &str, flavor: Flavor) -> String {
    let flavor_id = match flavor {
        Flavor::Cmi5 => "cmi5",
        Flavor::Xapi => "xapi",
    };
    let key = (course.to_string(), flavor_id);
    if let Some(id) = store.registrations.get(&key) {
        return id.clone();
    }
    let id = format!("urn:coursecraft:mock:registration:{flavor_id}-{course}");
    store.registrations.insert(key, id.clone());
    id
}

fn actor_json() -> Value {
    json!({
        "objectType": "Agent",
        "name": "Learner, Test",
        "account": {
            "homePage": "http://127.0.0.1",
            "name": "learner-1",
        },
    })
}

fn reset(store: &mut LrsStore, request: &Request) -> Response {
    if !is_method(request, "POST") {
        return Response::text(405, "Method Not Allowed");
    }
    store.registrations.clear();
    store.strict.reset();
    store.strict.add_token("Basic mock-xapi".to_string());
    store.fail_remaining = 0;
    store.fail_status = 500;
    store.next_session = 0;
    store.next_fetch = 0;
    Response::json(200, &json!({ "ok": true }))
}

fn log(store: &LrsStore, request: &Request) -> Response {
    if !is_method(request, "GET") {
        return Response::text(405, "Method Not Allowed");
    }
    let problems: Vec<String> = store
        .strict
        .violations()
        .iter()
        .map(|v| v.message.clone())
        .collect();
    Response::json(
        200,
        &json!({
            "statements": store.strict.statements(),
            "problems": problems,
            "stateKeys": store.strict.state_keys(),
        }),
    )
}

fn fail(store: &mut LrsStore, request: &Request) -> Response {
    if !is_method(request, "POST") {
        return Response::text(405, "Method Not Allowed");
    }
    let params = query_map(&request.query);
    let count = params
        .get("count")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(3);
    let status = params
        .get("status")
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(500);
    store.fail_remaining = count;
    store.fail_status = status;
    Response::json(
        200,
        &json!({ "ok": true, "count": count, "status": status }),
    )
}

fn returned(request: &Request) -> Response {
    if !is_method(request, "GET") {
        return Response::text(405, "Method Not Allowed");
    }
    Response {
        status: 200,
        headers: vec![(
            "Content-Type".to_string(),
            "text/html; charset=utf-8".to_string(),
        )],
        body: b"<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>Returned</title></head><body style=\"font:16px system-ui;padding:2rem\"><h1>Returned</h1><p>The learner returned to the LMS.</p></body></html>".to_vec(),
    }
}

fn xapi(store: &mut LrsStore, request: &Request, rest: &str) -> Response {
    match rest {
        "statements" => {
            if is_method(request, "POST") && store.fail_remaining > 0 {
                store.fail_remaining = store.fail_remaining.saturating_sub(1);
                return Response::text(store.fail_status, "injected failure");
            }
            Response::from_lrs(store.strict.handle_statements(
                &request.method,
                &request.headers,
                &request.body,
            ))
        }
        "activities/state" => Response::from_lrs(store.strict.handle_state(
            &request.method,
            &request.query,
            &request.headers,
            &request.body,
        )),
        _ => Response::text(404, "Not Found"),
    }
}

fn static_file(store: &LrsStore, path: &str) -> Response {
    let decoded = percent_decode(path);
    let Some(relative) = decoded.strip_prefix('/') else {
        return Response::text(404, "Not Found");
    };
    if relative.starts_with('/')
        || relative.contains('\\')
        || relative.as_bytes().get(1).is_some_and(|byte| *byte == b':')
        || relative.split('/').any(|part| part == "..")
    {
        return Response::text(404, "Not Found");
    }
    let relative = if relative.is_empty() {
        "index.html"
    } else {
        relative
    };
    let Some(content_type) = content_type(relative) else {
        return Response::text(404, "Not Found");
    };
    match std::fs::read(store.base_dir.join(relative)) {
        Ok(body) => Response {
            status: 200,
            headers: vec![("Content-Type".to_string(), content_type.to_string())],
            body,
        },
        Err(_) => Response::text(404, "Not Found"),
    }
}

fn content_type(path: &str) -> Option<&'static str> {
    let (_, extension) = path.rsplit_once('.')?;
    match extension.to_ascii_lowercase().as_str() {
        "html" => Some("text/html; charset=utf-8"),
        "js" => Some("text/javascript; charset=utf-8"),
        "wasm" => Some("application/wasm"),
        "json" => Some("application/json"),
        "css" => Some("text/css; charset=utf-8"),
        "xml" => Some("application/xml"),
        "png" => Some("image/png"),
        "jpg" => Some("image/jpeg"),
        "svg" => Some("image/svg+xml"),
        "mp3" => Some("audio/mpeg"),
        "mp4" => Some("video/mp4"),
        "vtt" => Some("text/vtt; charset=utf-8"),
        "woff2" => Some("font/woff2"),
        _ => None,
    }
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
                (Some(high), Some(low)) => match (hex_value(*high), hex_value(*low)) {
                    (Some(high), Some(low)) => {
                        out.push(high * 16 + low);
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

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

pub(crate) fn read_request(stream: &mut TcpStream) -> Result<Request, Response> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 8192];
    let header_end = loop {
        if let Some(position) = find_header_end(&buffer) {
            break position;
        }
        if buffer.len() > MAX_HEADER {
            return Err(Response::text(431, "Request Header Fields Too Large"));
        }
        match stream.read(&mut chunk) {
            Ok(0) => return Err(Response::text(400, "Bad Request")),
            Ok(read) => buffer.extend_from_slice(chunk.get(..read).unwrap_or_default()),
            Err(_) => return Err(Response::text(400, "Bad Request")),
        }
    };

    let header_text = String::from_utf8_lossy(buffer.get(..header_end).unwrap_or_default());
    let mut lines = header_text.split("\r\n");
    let request_line = lines.next().unwrap_or_default();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let target = parts.next().unwrap_or_default().to_string();
    if method.is_empty() || target.is_empty() {
        return Err(Response::text(400, "Bad Request"));
    }
    let (path, query) = target
        .split_once('?')
        .map_or((target.as_str(), ""), |(path, query)| (path, query));

    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.push((name.trim().to_string(), value.trim().to_string()));
        }
    }

    let content_length = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.parse::<usize>().ok())
        .unwrap_or(0);
    if content_length > MAX_BODY {
        return Err(Response::text(413, "Payload Too Large"));
    }

    let body_start = header_end.saturating_add(4);
    let mut body = buffer.get(body_start..).unwrap_or_default().to_vec();
    while body.len() < content_length {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(read) => body.extend_from_slice(chunk.get(..read).unwrap_or_default()),
            Err(_) => return Err(Response::text(400, "Bad Request")),
        }
    }
    body.truncate(content_length);

    Ok(Request {
        method,
        path: path.to_string(),
        query: query.to_string(),
        headers,
        body,
    })
}

pub(crate) fn write_response(stream: &mut TcpStream, response: &Response) -> std::io::Result<()> {
    let mut head = format!(
        "HTTP/1.1 {} {}\r\n",
        response.status,
        reason_phrase(response.status)
    );
    for (name, value) in &response.headers {
        head.push_str(name);
        head.push_str(": ");
        head.push_str(value);
        head.push_str("\r\n");
    }
    head.push_str("Connection: close\r\n");
    head.push_str(&format!("Content-Length: {}\r\n\r\n", response.body.len()));
    stream.write_all(head.as_bytes())?;
    stream.write_all(&response.body)?;
    stream.flush()
}

fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        413 => "Payload Too Large",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        _ => "Status",
    }
}
