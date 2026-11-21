//! LTI 1.3 reporting for the web player (wasm only).
//!
//! The published LTI package talks to the CourseCraft LTI server on its own
//! origin. A report is posted to `../lti/score`, relative to `index.html`, so a
//! tool mounted under a path prefix works without configuration. The launch
//! cookie travels with the request.
//!
//! The transport is `ehttp` and the retry is owned by [`LtiReporter`]:
//! `204` means sent, anything else (including a network error) spends the one
//! retry, and a second failure shows the lost-reporting notice. LTI 1.3 has no
//! suspend data, so a relaunch starts the course fresh.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use coursecraft_course::Course;
use coursecraft_logic::{Effect, HostInfo, Player};

use coursecraft_lms::LtiReporter;
use web_sys::wasm_bindgen::JsCast;
use web_sys::wasm_bindgen::JsValue;
use web_sys::wasm_bindgen::closure::Closure;

/// Reference-counted, mutable handle to the reporter and its transport.
pub type LtiHandle = Rc<RefCell<LtiHandleInner>>;

/// The reporter, the statuses that have arrived, and the repaint context.
pub struct LtiHandleInner {
    reporter: LtiReporter,
    responses: Arc<Mutex<Vec<u16>>>,
    ctx: egui::Context,
}

impl std::fmt::Debug for LtiHandleInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LtiHandleInner").finish_non_exhaustive()
    }
}

/// Initialize the standalone player plus an LTI reporter for a fetched course.
///
/// # Errors
/// Returns joined issue descriptions if course validation or player creation fails.
pub fn init_player(
    course: &'static Course,
    ctx: egui::Context,
) -> Result<(Player<'static>, LtiHandle, f64), String> {
    let start_ms = crate::lms::now_ms();
    let mut player = crate::lms::fresh_player(course)?;
    let name = web_sys::window()
        .and_then(|window| window.location().search().ok())
        .and_then(|search| crate::load::learner_name_from_query(&search));
    player.set_host_info(HostInfo {
        learner_name: name,
        date: Some(crate::lms::today_date()),
    });
    Ok((player, new_handle(ctx), start_ms))
}

/// Wrap a fresh reporter and an empty response queue in a shared handle.
#[must_use]
pub fn new_handle(ctx: egui::Context) -> LtiHandle {
    Rc::new(RefCell::new(LtiHandleInner {
        reporter: LtiReporter::new(),
        responses: Arc::new(Mutex::new(Vec::new())),
        ctx,
    }))
}

/// Record one input's effects and send the latest report if one is waiting.
pub fn after_input(handle: &LtiHandle, effects: &[Effect]) {
    let Ok(mut guard) = handle.try_borrow_mut() else {
        return;
    };
    guard.reporter.record(effects);
    pump(&mut guard);
}

/// Collect finished requests, apply their results and send any retry.
pub fn update(handle: &LtiHandle) {
    let Ok(mut guard) = handle.try_borrow_mut() else {
        return;
    };
    let statuses = {
        let mut responses = guard
            .responses
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        std::mem::take(&mut *responses)
    };
    for status in statuses {
        if status == 204 {
            guard.reporter.mark_sent();
        } else {
            guard.reporter.mark_failed();
        }
        pump(&mut guard);
    }
}

/// Whether reporting has given up for this session.
#[must_use]
pub fn lost(handle: &LtiHandle) -> bool {
    handle
        .try_borrow()
        .map(|guard| guard.reporter.lost())
        .unwrap_or(false)
}

/// Register `pagehide` and `beforeunload` once, flushing any pending score with `keepalive`.
pub fn register_page_close(handle: &LtiHandle) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let handle = Rc::clone(handle);
    let closure = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_event| {
        flush_keepalive(&handle);
    }));
    let js_fn: &js_sys::Function = closure.as_ref().unchecked_ref();
    let _ = window.add_event_listener_with_callback("pagehide", js_fn);
    let _ = window.add_event_listener_with_callback("beforeunload", js_fn);
    closure.forget();
}

fn flush_keepalive(handle: &LtiHandle) {
    let Ok(mut guard) = handle.try_borrow_mut() else {
        return;
    };
    let Some(report) = guard.reporter.take_pending() else {
        return;
    };
    send_keepalive(&report.json_body());
}

fn send_keepalive(body: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let global: &JsValue = window.as_ref();
    let Ok(fetch) = js_sys::Reflect::get(global, &JsValue::from_str("fetch")) else {
        return;
    };
    if !fetch.is_function() {
        return;
    }
    let fetch: js_sys::Function = fetch.unchecked_into();

    let init = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &init,
        &JsValue::from_str("method"),
        &JsValue::from_str("POST"),
    );
    let _ = js_sys::Reflect::set(
        &init,
        &JsValue::from_str("credentials"),
        &JsValue::from_str("include"),
    );
    let _ = js_sys::Reflect::set(&init, &JsValue::from_str("keepalive"), &JsValue::TRUE);

    let headers = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &headers,
        &JsValue::from_str("Content-Type"),
        &JsValue::from_str("application/json"),
    );
    let _ = js_sys::Reflect::set(&init, &JsValue::from_str("headers"), &headers);
    let _ = js_sys::Reflect::set(&init, &JsValue::from_str("body"), &JsValue::from_str(body));

    let Ok(promise) = fetch.call2(global, &JsValue::from_str("../lti/score"), &init) else {
        return;
    };
    let promise: js_sys::Promise = promise.unchecked_into();
    wasm_bindgen_futures::spawn_local(async move {
        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
    });
}

fn pump(handle: &mut LtiHandleInner) {
    let Some(report) = handle.reporter.take_pending() else {
        return;
    };
    let headers = ehttp::Headers {
        headers: vec![("Content-Type".to_string(), "application/json".to_string())],
    };
    let mut request = ehttp::Request::new(ehttp::Method::POST, "../lti/score", headers);
    request.body = report.json_body().into_bytes();
    request.credentials = ehttp::Credentials::Include;

    let responses = Arc::clone(&handle.responses);
    let ctx = handle.ctx.clone();
    ehttp::fetch(request, move |result| {
        let status = match result {
            Ok(response) => response.status,
            Err(_) => 0,
        };
        let mut guard = responses
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.push(status);
        ctx.request_repaint();
    });
}
