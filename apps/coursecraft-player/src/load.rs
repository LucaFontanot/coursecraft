//! Course loading for desktop and web.

#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
#[cfg(any(not(target_arch = "wasm32"), test))]
use std::path::PathBuf;

/// Usage string printed when invalid arguments are provided.
#[cfg(any(not(target_arch = "wasm32"), test))]
pub const USAGE: &str = "Usage: coursecraft-player [COURSE_JSON] [--log-effects]";

/// The learner's name from a launch query string, with `+` as space and
/// percent escapes decoded. Empty or missing gives `None`.
#[cfg(any(target_arch = "wasm32", test))]
#[must_use]
pub fn learner_name_from_query(search: &str) -> Option<String> {
    let query = search.strip_prefix('?').unwrap_or(search);
    for pair in query.split('&') {
        let Some((key, value)) = pair.split_once('=') else {
            continue;
        };
        if key == "name" {
            let name = decode_query_component(value);
            return (!name.trim().is_empty()).then_some(name);
        }
    }
    None
}

/// Whether the page query string enables effect logging (`debug=1`).
#[cfg(any(target_arch = "wasm32", test))]
#[must_use]
pub fn debug_logging_from_query(search: &str) -> bool {
    let query = search.strip_prefix('?').unwrap_or(search);
    query.split('&').any(|pair| pair == "debug=1")
}

/// Read `debug=1` from the browser's query string; returns `false` if unavailable.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn web_log_effects() -> bool {
    web_sys::window()
        .and_then(|window| window.location().search().ok())
        .is_some_and(|search| debug_logging_from_query(&search))
}

#[cfg(any(target_arch = "wasm32", test))]
fn decode_query_component(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while let Some(&byte) = bytes.get(index) {
        match byte {
            b'+' => {
                out.push(b' ');
                index += 1;
            }
            b'%' => {
                let high = bytes.get(index + 1).and_then(|byte| hex_value(*byte));
                let low = bytes.get(index + 2).and_then(|byte| hex_value(*byte));
                if let (Some(high), Some(low)) = (high, low) {
                    out.push(high.saturating_mul(16).saturating_add(low));
                    index += 3;
                } else {
                    out.push(byte);
                    index += 1;
                }
            }
            _ => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(any(target_arch = "wasm32", test))]
fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Command-line configuration for the player.
#[cfg(any(not(target_arch = "wasm32"), test))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Path to the course JSON file.
    pub course_path: PathBuf,
    /// Whether to log each effect as a JSON line to stdout.
    pub log_effects: bool,
}

/// Parse command-line arguments into a [`Config`].
///
/// Accepts an optional positional `COURSE_JSON` (defaults to
/// `examples/hello-course/course.json`) and the flag `--log-effects`.
///
/// # Errors
/// Returns an error message with usage instructions if unknown flags or multiple
/// positional arguments are encountered.
#[cfg(any(not(target_arch = "wasm32"), test))]
pub fn parse_args<I, S>(args: I) -> Result<Config, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut course_path = None;
    let mut log_effects = false;

    for arg in args {
        let s = arg.as_ref();
        if s == "--log-effects" {
            log_effects = true;
        } else if s.starts_with('-') {
            return Err(format!("unknown flag: '{s}'\n{USAGE}"));
        } else if course_path.is_none() {
            course_path = Some(PathBuf::from(s));
        } else {
            return Err(format!("unexpected argument: '{s}'\n{USAGE}"));
        }
    }

    Ok(Config {
        course_path: course_path
            .unwrap_or_else(|| PathBuf::from("examples/hello-course/course.json")),
        log_effects,
    })
}

/// Read and parse a course synchronously from disk on desktop platforms.
///
/// # Errors
/// Returns an error if the file cannot be read or fails to parse as valid JSON.
#[cfg(not(target_arch = "wasm32"))]
pub fn load_course_sync(path: &Path) -> Result<coursecraft_course::Course, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read course file '{}': {e}", path.display()))?;
    parse_course(&content, &path.display().to_string())
}

/// Parse course JSON text; `source` names where the text came from (for errors).
///
/// # Errors
/// Returns a message mentioning `source` if the text is not a valid course.
pub fn parse_course(text: &str, source: &str) -> Result<coursecraft_course::Course, String> {
    coursecraft_course::Course::from_json(text)
        .map_err(|e| format!("failed to parse course JSON from '{source}': {e}"))
}

/// Course text plus the optional package-standard marker.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebCourse {
    pub course_json: String,
    pub package_json: Option<String>,
}

/// Collects the two concurrent start-up fetches (`course.json` and
/// `coursecraft-package.json`) and produces the combined result once both have
/// answered.
#[cfg(any(target_arch = "wasm32", test))]
#[derive(Debug, Default)]
pub struct PendingWebFetch {
    course: Option<Result<String, String>>,
    package: Option<Option<String>>,
}

#[cfg(any(target_arch = "wasm32", test))]
impl PendingWebFetch {
    /// Record the `course.json` fetch result. Returns `Some` once the package
    /// marker fetch has also answered.
    pub fn set_course(
        &mut self,
        result: Result<String, String>,
    ) -> Option<Result<WebCourse, String>> {
        self.course = Some(result);
        self.take_ready()
    }

    /// Record the `coursecraft-package.json` fetch result (`None` when missing
    /// or unreadable). Returns `Some` once `course.json` has also answered.
    pub fn set_package(
        &mut self,
        package_json: Option<String>,
    ) -> Option<Result<WebCourse, String>> {
        self.package = Some(package_json);
        self.take_ready()
    }

    fn take_ready(&mut self) -> Option<Result<WebCourse, String>> {
        if self.course.is_none() || self.package.is_none() {
            return None;
        }
        let course_res = self.course.take()?;
        let package_json = self.package.take()?;
        Some(course_res.map(|course_json| WebCourse {
            course_json,
            package_json,
        }))
    }
}

/// Web loader delivering the fetched course and package marker.
#[cfg(target_arch = "wasm32")]
pub type WebLoader = std::sync::Arc<std::sync::Mutex<Option<Result<WebCourse, String>>>>;

#[cfg(target_arch = "wasm32")]
fn store_web_result(
    loader: &std::sync::Mutex<Option<Result<WebCourse, String>>>,
    result: Result<WebCourse, String>,
) {
    let mut guard = loader
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *guard = Some(result);
}

/// Start concurrent asynchronous fetches of `course.json` and `coursecraft-package.json`.
///
/// A missing or unreadable marker is [`None`], which the player treats as
/// SCORM 1.2. The course fetch failing still fails playback.
#[cfg(target_arch = "wasm32")]
pub fn start_web_fetch(ctx: &egui::Context) -> WebLoader {
    use std::sync::{Arc, Mutex};

    let loader = Arc::new(Mutex::new(None));
    let pending = Arc::new(Mutex::new(PendingWebFetch::default()));

    // Same-origin fetches must send cookies: LMSs such as SCORM Cloud protect
    // package files with sign-in cookies, and ehttp's web default omits them.
    let loader_course = Arc::clone(&loader);
    let pending_course = Arc::clone(&pending);
    let ctx_course = ctx.clone();
    #[allow(unused_mut)]
    let mut request = ehttp::Request::get("course.json");
    #[cfg(target_arch = "wasm32")]
    {
        request.credentials = ehttp::Credentials::SameOrigin;
    }
    ehttp::fetch(request, move |response| {
        let course_result = match response {
            Ok(resp) => {
                if resp.status != 200 {
                    Err(format!("failed to fetch course.json: HTTP {}", resp.status))
                } else {
                    String::from_utf8(resp.bytes)
                        .map_err(|e| format!("course.json is not valid UTF-8: {e}"))
                }
            }
            Err(err) => Err(format!("failed to fetch course.json: {err}")),
        };
        let ready = pending_course
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .set_course(course_result);
        if let Some(result) = ready {
            store_web_result(&loader_course, result);
            ctx_course.request_repaint();
        }
    });

    let loader_pkg = Arc::clone(&loader);
    let pending_pkg = Arc::clone(&pending);
    let ctx_pkg = ctx.clone();
    #[allow(unused_mut)]
    let mut package_request = ehttp::Request::get("coursecraft-package.json");
    #[cfg(target_arch = "wasm32")]
    {
        package_request.credentials = ehttp::Credentials::SameOrigin;
    }
    ehttp::fetch(package_request, move |response| {
        let package_json = match response {
            Ok(resp) if resp.status == 200 => String::from_utf8(resp.bytes).ok(),
            _ => None,
        };
        let ready = pending_pkg
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .set_package(package_json);
        if let Some(result) = ready {
            store_web_result(&loader_pkg, result);
            ctx_pkg.request_repaint();
        }
    });

    loader
}
