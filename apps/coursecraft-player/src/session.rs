//! Pure, non-wasm player reporting session seam.
//!
//! Holds [`ReportingHost`], [`PlayerSession`], and [`sequence_input`], which
//! guarantees that outer runtime effects are forwarded to the reporting host
//! **before** recursive followup inputs (`RunJavaScript`, `MediaEnded`, etc.)
//! are processed.

use coursecraft_course::Course;
use coursecraft_lms::{CmiApi, LmsBridge, LrsApi, LtiReporter, Standard, StartPlan, XapiBridge};
use coursecraft_logic::{Effect, HostInfo, Input, Player, PlayerState};
use coursecraft_xapi::{Flavor, LaunchParams};

use crate::app::format_issues;
use crate::web_resume::{SuspendStorage, WebResume};

/// Transport abstraction for posting LTI 1.3 score reports to `/lti/score`.
pub trait LtiTransport {
    /// Queue an HTTP POST with `json_body` to `/lti/score`.
    fn send(&mut self, json_body: &str);
    /// Take all HTTP status codes that have arrived since the last call (`0` = network error).
    fn poll(&mut self) -> Vec<u16>;
}

impl<T: LtiTransport + ?Sized> LtiTransport for Box<T> {
    fn send(&mut self, json_body: &str) {
        (**self).send(json_body);
    }

    fn poll(&mut self) -> Vec<u16> {
        (**self).poll()
    }
}

/// Reporting backend for one playing course session.
pub enum ReportingHost {
    /// SCORM 1.2 or SCORM 2004 runtime bridge.
    Scorm {
        bridge: LmsBridge,
        api: Box<dyn CmiApi>,
    },
    /// cmi5 or plain xAPI LRS bridge.
    Xapi {
        bridge: Box<XapiBridge>,
        api: Box<dyn LrsApi>,
        last_state: Option<PlayerState>,
    },
    /// LTI 1.3 AGS score reporter.
    Lti {
        reporter: LtiReporter,
        transport: Box<dyn LtiTransport>,
    },
    /// Standalone web `localStorage` resume coordinator.
    Web(WebResume<Box<dyn SuspendStorage>>),
    /// Standalone playback with no reporting or resume persistence.
    Standalone,
}

impl std::fmt::Debug for ReportingHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scorm { .. } => f
                .debug_struct("ReportingHost::Scorm")
                .finish_non_exhaustive(),
            Self::Xapi { .. } => f
                .debug_struct("ReportingHost::Xapi")
                .finish_non_exhaustive(),
            Self::Lti { .. } => f.debug_struct("ReportingHost::Lti").finish_non_exhaustive(),
            Self::Web(_) => f.debug_struct("ReportingHost::Web").finish_non_exhaustive(),
            Self::Standalone => f.write_str("ReportingHost::Standalone"),
        }
    }
}

impl ReportingHost {
    /// Forward one input's effects to the active reporting backend.
    pub fn after_input(
        &mut self,
        course: &Course,
        state: &PlayerState,
        effects: &[Effect],
        session_ms: u64,
        now_ms: u64,
    ) -> Vec<String> {
        match self {
            Self::Scorm { bridge, api } => {
                bridge.after_input(api, course, state, effects, session_ms)
            }
            Self::Xapi {
                bridge,
                api,
                last_state,
            } => {
                *last_state = Some(state.clone());
                bridge.after_input(api, course, state, effects, session_ms, now_ms)
            }
            Self::Lti {
                reporter,
                transport,
            } => {
                reporter.record(effects);
                pump_lti(reporter, &mut **transport);
                Vec::new()
            }
            Self::Web(resume) => {
                resume.after_input(course, state, effects);
                Vec::new()
            }
            Self::Standalone => Vec::new(),
        }
    }

    /// Poll async reporting transports (xAPI/cmi5 retries, LTI responses).
    pub fn tick(&mut self, course: &Course, now_ms: u64) -> Vec<String> {
        match self {
            Self::Xapi { bridge, api, .. } => bridge.tick(api, course, now_ms).warnings,
            Self::Lti {
                reporter,
                transport,
            } => {
                for status in transport.poll() {
                    if status == 204 {
                        reporter.mark_sent();
                    } else {
                        reporter.mark_failed();
                    }
                    pump_lti(reporter, &mut **transport);
                }
                Vec::new()
            }
            Self::Scorm { .. } | Self::Web(_) | Self::Standalone => Vec::new(),
        }
    }

    /// Flush/finish the session on `pagehide` or `beforeunload` (idempotent).
    pub fn on_page_close(&mut self, course: &Course, session_ms: u64) -> Vec<String> {
        match self {
            Self::Scorm { bridge, api } => bridge.finish_session(api, session_ms),
            Self::Xapi {
                bridge,
                api,
                last_state,
            } => bridge.finish_session(api, course, last_state.as_ref(), session_ms),
            Self::Web(resume) => {
                resume.save_current(course);
                Vec::new()
            }
            Self::Lti {
                reporter,
                transport,
            } => {
                pump_lti(reporter, &mut **transport);
                Vec::new()
            }
            Self::Standalone => Vec::new(),
        }
    }

    /// Whether reporting was attempted and has now stopped for this session.
    #[must_use]
    pub fn reporting_lost(&self) -> bool {
        match self {
            Self::Xapi { bridge, .. } => bridge.reporting_lost(),
            Self::Lti { reporter, .. } => reporter.lost(),
            Self::Scorm { .. } | Self::Web(_) | Self::Standalone => false,
        }
    }

    /// Safe `returnURL` from cmi5 `LMS.LaunchData`, if present.
    #[must_use]
    pub fn return_url(&self) -> Option<&str> {
        match self {
            Self::Xapi { bridge, .. } => bridge.return_url(),
            _ => None,
        }
    }

    /// Clear saved standalone web resume data, if this is a Web session.
    pub fn clear_web_resume(&mut self) {
        if let Self::Web(resume) = self {
            resume.clear();
        }
    }
}

fn pump_lti(reporter: &mut LtiReporter, transport: &mut dyn LtiTransport) {
    if let Some(report) = reporter.take_pending() {
        transport.send(&report.json_body());
    }
}

/// Drive `input` through `player`, forward its `effects` **before** processing
/// any recursive followup inputs, and then recurse on those followups.
///
/// Both [`PlayerSession::handle_input`] and `Playing::handle_input` use this
/// function so effect-forwarding order is identical natively and on wasm.
pub fn sequence_input<'a>(
    player: &mut Player<'a>,
    input: &Input,
    forward_effects: &mut impl FnMut(&Player<'a>, &[Effect]),
    handle_effects: &mut impl FnMut(&Player<'a>, &[Effect]) -> Vec<Input>,
) {
    let effects = player.handle(input);
    forward_effects(player, &effects);
    let followups = handle_effects(player, &effects);
    for next in followups {
        sequence_input(player, &next, forward_effects, handle_effects);
    }
}

/// In-progress xAPI / cmi5 handshake before entering [`PlayerSession`].
pub struct XapiConnecting<'a> {
    course: &'a Course,
    bridge: Box<XapiBridge>,
    api: Box<dyn LrsApi>,
    warnings: Vec<String>,
}

impl std::fmt::Debug for XapiConnecting<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XapiConnecting").finish_non_exhaustive()
    }
}

impl<'a> XapiConnecting<'a> {
    /// Poll the handshake, consuming `self` if ready or returning `Err(self)` if still pending.
    ///
    /// # Errors
    /// Returns `Err(self)` if the handshake is still in progress.
    pub fn try_into_session(
        mut self,
        host_info: HostInfo,
        seed: u32,
        now_ms: u64,
    ) -> Result<Result<PlayerSession<'a>, String>, Self> {
        let tick = self.bridge.tick(&mut self.api, self.course, now_ms);
        self.warnings.extend(tick.warnings);
        let Some(plan) = tick.plan else {
            return Err(self);
        };
        Ok(finish_xapi_connect(
            self.course,
            self.bridge,
            self.api,
            self.warnings,
            plan,
            host_info,
            seed,
        ))
    }
}

fn finish_xapi_connect<'a>(
    course: &'a Course,
    bridge: Box<XapiBridge>,
    api: Box<dyn LrsApi>,
    mut warnings: Vec<String>,
    plan: StartPlan,
    host_info: HostInfo,
    seed: u32,
) -> Result<PlayerSession<'a>, String> {
    let mut player = build_player_from_plan(course, plan.resume, seed, &mut warnings)?;
    player.set_host_info(HostInfo {
        learner_name: plan.learner_name.or(host_info.learner_name),
        date: host_info.date,
    });
    let last_state = Some(player.state().clone());
    Ok(PlayerSession {
        course,
        player,
        host: ReportingHost::Xapi {
            bridge,
            api,
            last_state,
        },
        ended: false,
        warnings,
        return_url: None,
    })
}

fn build_player_from_plan<'a>(
    course: &'a Course,
    resume: Option<PlayerState>,
    seed: u32,
    warnings: &mut Vec<String>,
) -> Result<Player<'a>, String> {
    match resume {
        Some(state) => match Player::resume(course, state) {
            Ok(player) => Ok(player),
            Err(issues) => {
                warnings.push(format!("Player::resume failed: {}", format_issues(issues)));
                Player::with_draw_seed(course, seed).map_err(format_issues)
            }
        },
        None => Player::with_draw_seed(course, seed).map_err(format_issues),
    }
}

/// Complete native player session pairing [`Player`] with [`ReportingHost`].
pub struct PlayerSession<'a> {
    course: &'a Course,
    /// Core course runtime.
    pub player: Player<'a>,
    /// Active reporting backend.
    pub host: ReportingHost,
    /// Whether [`Effect::Exit`] has been emitted.
    pub ended: bool,
    /// Accumulated bridge/runtime warnings.
    pub warnings: Vec<String>,
    /// Captured `returnURL` on exit, if any.
    pub return_url: Option<String>,
}

impl std::fmt::Debug for PlayerSession<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlayerSession")
            .field("host", &self.host)
            .field("ended", &self.ended)
            .field("warnings", &self.warnings)
            .field("return_url", &self.return_url)
            .finish_non_exhaustive()
    }
}

impl<'a> PlayerSession<'a> {
    /// Start a SCORM 1.2 or SCORM 2004 session.
    ///
    /// # Errors
    /// Returns joined course validation issues if player creation fails.
    pub fn start_scorm(
        course: &'a Course,
        standard: Standard,
        mut api: Box<dyn CmiApi>,
        host_info: HostInfo,
        seed: u32,
    ) -> Result<Self, String> {
        let mut bridge = LmsBridge::for_standard(standard);
        let (plan, mut warnings) = bridge.start(&mut api, course);
        let mut player = build_player_from_plan(course, plan.resume, seed, &mut warnings)?;
        player.set_host_info(HostInfo {
            learner_name: plan.learner_name.or(host_info.learner_name),
            date: host_info.date,
        });
        Ok(Self {
            course,
            player,
            host: ReportingHost::Scorm { bridge, api },
            ended: false,
            warnings,
            return_url: None,
        })
    }

    /// Begin an xAPI / cmi5 handshake.
    #[must_use]
    pub fn begin_xapi(
        course: &'a Course,
        flavor: Flavor,
        params: LaunchParams,
        mut api: Box<dyn LrsApi>,
    ) -> XapiConnecting<'a> {
        let mut bridge = Box::new(XapiBridge::new(params, flavor, course));
        bridge.start(&mut api);
        XapiConnecting {
            course,
            bridge,
            api,
            warnings: Vec::new(),
        }
    }

    /// Start an LTI 1.3 session.
    ///
    /// # Errors
    /// Returns joined course validation issues if player creation fails.
    pub fn start_lti(
        course: &'a Course,
        transport: Box<dyn LtiTransport>,
        host_info: HostInfo,
        seed: u32,
    ) -> Result<Self, String> {
        let mut player = Player::with_draw_seed(course, seed).map_err(format_issues)?;
        player.set_host_info(host_info);
        Ok(Self {
            course,
            player,
            host: ReportingHost::Lti {
                reporter: LtiReporter::new(),
                transport,
            },
            ended: false,
            warnings: Vec::new(),
            return_url: None,
        })
    }

    /// Start a standalone web session with `SuspendStorage` resume.
    ///
    /// # Errors
    /// Returns joined course validation issues if player creation fails.
    pub fn start_web(
        course: &'a Course,
        storage: Box<dyn SuspendStorage>,
        host_info: HostInfo,
        seed: u32,
    ) -> Result<Self, String> {
        let resume = WebResume::new(storage, course);
        let mut player = resume.load_player(course, seed).map_err(format_issues)?;
        player.set_host_info(host_info);
        Ok(Self {
            course,
            player,
            host: ReportingHost::Web(resume),
            ended: false,
            warnings: Vec::new(),
            return_url: None,
        })
    }

    /// Start a standalone session with no reporting or resume persistence.
    ///
    /// # Errors
    /// Returns joined course validation issues if player creation fails.
    pub fn start_standalone(
        course: &'a Course,
        host_info: HostInfo,
        seed: u32,
    ) -> Result<Self, String> {
        let mut player = Player::with_draw_seed(course, seed).map_err(format_issues)?;
        player.set_host_info(host_info);
        Ok(Self {
            course,
            player,
            host: ReportingHost::Standalone,
            ended: false,
            warnings: Vec::new(),
            return_url: None,
        })
    }

    /// Handle one runtime input, forwarding effects before processing followups.
    pub fn handle_input(
        &mut self,
        input: &Input,
        session_ms: u64,
        now_ms: u64,
        on_js: &mut impl FnMut(&Player<'a>, &str) -> Vec<Input>,
        media_followups: &mut impl FnMut(&Player<'a>, &[Effect]) -> Vec<Input>,
    ) {
        let host = &mut self.host;
        let ended = &mut self.ended;
        let warnings = &mut self.warnings;
        sequence_input(
            &mut self.player,
            input,
            &mut |player, effects| {
                warnings.extend(host.after_input(
                    player.course(),
                    player.state(),
                    effects,
                    session_ms,
                    now_ms,
                ));
            },
            &mut |player, effects| {
                let mut followups = Vec::new();
                for effect in effects {
                    match effect {
                        Effect::Exit => {
                            *ended = true;
                        }
                        Effect::RunJavaScript { code } => {
                            followups.extend(on_js(player, code));
                        }
                        _ => {}
                    }
                }
                followups.extend(media_followups(player, effects));
                followups
            },
        );
        if self.ended && self.return_url.is_none() {
            self.return_url = self.host.return_url().map(str::to_owned);
        }
    }

    /// Poll async reporting transports.
    pub fn tick(&mut self, now_ms: u64) {
        let w = self.host.tick(self.course, now_ms);
        self.warnings.extend(w);
    }

    /// Flush/finish the session on `pagehide` or `beforeunload` (idempotent).
    pub fn on_page_close(&mut self, session_ms: u64) {
        let w = self.host.on_page_close(self.course, session_ms);
        self.warnings.extend(w);
    }

    /// Whether reporting was attempted and has now stopped for this session.
    #[must_use]
    pub fn reporting_lost(&self) -> bool {
        self.host.reporting_lost()
    }

    /// Clear web resume storage (if Web) and restart the course from slide 0.
    ///
    /// # Errors
    /// Returns joined course validation issues if player creation fails.
    pub fn restart(
        &mut self,
        host_info: HostInfo,
        seed: u32,
        session_ms: u64,
        now_ms: u64,
    ) -> Result<(), String> {
        self.host.clear_web_resume();
        let mut player = Player::with_draw_seed(self.course, seed).map_err(format_issues)?;
        player.set_host_info(host_info);
        self.player = player;
        self.ended = false;
        self.return_url = None;
        self.handle_input(
            &Input::Start,
            session_ms,
            now_ms,
            &mut |_, _| Vec::new(),
            &mut |_, _| Vec::new(),
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use coursecraft_course::Course;
    use coursecraft_lms::{
        LrsMethod, LrsRequest, LrsResponse, ReportingStandard, reporting_from_marker,
    };
    use coursecraft_logic::{HostInfo, Input};
    use coursecraft_strict_lms::{
        Flavor as StrictFlavor, LaunchSpec, Scorm12Config, Scorm2004Config, StrictLrs,
        StrictScorm12, StrictScorm2004, build_cmi5_launch_data,
    };
    use coursecraft_xapi::{Flavor, LaunchParams};
    use serde_json::json;

    use super::*;
    use crate::load::learner_name_from_query;
    use crate::web_resume::{MemoryStorage, storage_key};

    // -----------------------------------------------------------------------
    // Strict LMS adapters for native PlayerSession tests
    // -----------------------------------------------------------------------

    #[derive(Clone)]
    struct StrictScorm12Api(Rc<RefCell<StrictScorm12>>);

    impl CmiApi for StrictScorm12Api {
        fn initialize(&mut self) -> bool {
            self.0.borrow_mut().lms_initialize("") == "true"
        }
        fn get_value(&mut self, key: &str) -> String {
            self.0.borrow_mut().lms_get_value(key)
        }
        fn set_value(&mut self, key: &str, value: &str) -> bool {
            self.0.borrow_mut().lms_set_value(key, value) == "true"
        }
        fn commit(&mut self) -> bool {
            self.0.borrow_mut().lms_commit("") == "true"
        }
        fn finish(&mut self) -> bool {
            self.0.borrow_mut().lms_finish("") == "true"
        }
        fn last_error(&mut self) -> String {
            let lms = self.0.borrow_mut();
            let code = lms.lms_get_last_error();
            let msg = lms.lms_get_error_string(code);
            format!("{code}: {msg}")
        }
    }

    #[derive(Clone)]
    struct StrictScorm2004Api(Rc<RefCell<StrictScorm2004>>);

    impl CmiApi for StrictScorm2004Api {
        fn initialize(&mut self) -> bool {
            self.0.borrow_mut().initialize("") == "true"
        }
        fn get_value(&mut self, key: &str) -> String {
            self.0.borrow_mut().get_value(key)
        }
        fn set_value(&mut self, key: &str, value: &str) -> bool {
            self.0.borrow_mut().set_value(key, value) == "true"
        }
        fn commit(&mut self) -> bool {
            self.0.borrow_mut().commit("") == "true"
        }
        fn finish(&mut self) -> bool {
            self.0.borrow_mut().terminate("") == "true"
        }
        fn last_error(&mut self) -> String {
            let lms = self.0.borrow_mut();
            let code = lms.get_last_error();
            let msg = lms.get_error_string(code);
            format!("{code}: {msg}")
        }
    }

    #[derive(Clone)]
    struct StrictLrsApi {
        lrs: Rc<RefCell<StrictLrs>>,
        responses: Vec<LrsResponse>,
        seq: u32,
        forced_status: Option<u16>,
        sent_urls: Rc<RefCell<Vec<String>>>,
    }

    impl StrictLrsApi {
        fn new(lrs: Rc<RefCell<StrictLrs>>) -> Self {
            Self {
                lrs,
                responses: Vec::new(),
                seq: 0,
                forced_status: None,
                sent_urls: Rc::new(RefCell::new(Vec::new())),
            }
        }
    }

    impl LrsApi for StrictLrsApi {
        fn send(&mut self, request: LrsRequest) {
            self.sent_urls.borrow_mut().push(request.url.clone());
            if let Some(status) = self.forced_status {
                self.responses.push(LrsResponse {
                    id: request.id,
                    status,
                    body: String::new(),
                });
                return;
            }
            let method = match request.method {
                LrsMethod::Get => "GET",
                LrsMethod::Post => "POST",
                LrsMethod::Put => "PUT",
            };
            let body = request.body.as_deref().unwrap_or("").as_bytes();
            let (path, query) = request.url.split_once('?').unwrap_or((&request.url, ""));
            let mut lrs = self.lrs.borrow_mut();
            let resp = if let Some(fetch_id) = path
                .rsplit("/fetch/")
                .next()
                .filter(|_| path.contains("/fetch/"))
            {
                lrs.handle_fetch(method, fetch_id, &request.headers)
            } else if path.ends_with("/statements") {
                lrs.handle_statements(method, &request.headers, body)
            } else if path.ends_with("/activities/state") {
                lrs.handle_state(method, query, &request.headers, body)
            } else {
                coursecraft_strict_lms::LrsResponse {
                    status: 404,
                    headers: vec![],
                    body: b"not found".to_vec(),
                }
            };
            self.responses.push(LrsResponse {
                id: request.id,
                status: resp.status,
                body: String::from_utf8_lossy(&resp.body).into_owned(),
            });
        }

        fn poll(&mut self) -> Vec<LrsResponse> {
            std::mem::take(&mut self.responses)
        }

        fn now_iso(&mut self) -> String {
            "2026-11-21T12:00:00Z".to_owned()
        }

        fn new_uuid(&mut self) -> String {
            self.seq += 1;
            format!("00000000-0000-4000-8000-{:012x}", self.seq)
        }
    }

    #[derive(Clone, Default)]
    struct MockLtiTransport {
        sent: Rc<RefCell<Vec<String>>>,
        queued_statuses: Rc<RefCell<Vec<u16>>>,
    }

    impl LtiTransport for MockLtiTransport {
        fn send(&mut self, json_body: &str) {
            self.sent.borrow_mut().push(json_body.to_owned());
        }
        fn poll(&mut self) -> Vec<u16> {
            std::mem::take(&mut *self.queued_statuses.borrow_mut())
        }
    }

    fn js_jump_course() -> Course {
        let json = r#"{
            "schema_version": 1,
            "id": "bug-a8-course",
            "title": "BUG-A8 Course",
            "quiz": { "passing_percent": 70 },
            "scenes": [{
                "id": "sc1",
                "title": "Scene 1",
                "slides": [
                    {
                        "id": "s-1",
                        "title": "Start",
                        "objects": [
                            { "id": "btn", "kind": { "type": "button", "label": "Pass and Jump" }, "frame": { "x": 100, "y": 100, "w": 200, "h": 50 } }
                        ],
                        "triggers": [
                            {
                                "when": { "event": "click", "object": "btn" },
                                "action": { "action": "submit_results" }
                            },
                            {
                                "when": { "event": "click", "object": "btn" },
                                "action": { "action": "run_javascript", "code": "CourseCraft.jump('s-end')" }
                            }
                        ]
                    },
                    {
                        "id": "s-q",
                        "title": "Question",
                        "question": {
                            "kind": "true_false",
                            "prompt": "True?",
                            "points": 10,
                            "choices": [
                                { "id": "t", "text": "True", "correct": true },
                                { "id": "f", "text": "False", "correct": false }
                            ]
                        },
                        "triggers": [
                            {
                                "when": { "event": "submit" },
                                "action": { "action": "submit_question" }
                            }
                        ]
                    },
                    {
                        "id": "s-end",
                        "title": "End",
                        "triggers": [
                            {
                                "when": { "event": "timeline_starts" },
                                "action": { "action": "exit_course" }
                            }
                        ]
                    }
                ]
            }]
        }"#;
        Course::from_json(json).expect("valid bug-a8 course")
    }

    fn start_cmi5_session<'a>(
        course: &'a Course,
        lrs: Rc<RefCell<StrictLrs>>,
        learner_name: Option<&str>,
    ) -> (PlayerSession<'a>, StrictLrsApi) {
        let fetch_id = "f1";
        let activity_id = "https://example.com/courses/bug-a8";
        let registration = "11111111-1111-4111-8111-111111111111";
        let name = learner_name.unwrap_or("Ada Lovelace");
        let actor = json!({
            "objectType": "Agent",
            "account": { "homePage": "https://lms.example", "name": "ada" },
            "name": name,
        });
        let launch_doc = build_cmi5_launch_data(activity_id, "sess-1", "Normal", None, None);
        lrs.borrow_mut().register_launch(
            LaunchSpec {
                flavor: StrictFlavor::Cmi5,
                activity_id: activity_id.to_owned(),
                actor,
                registration: Some(registration.to_owned()),
                launch_mode: "Normal".to_owned(),
                mastery_score: None,
                session_id: "sess-1".to_owned(),
                grouping_ids: vec![activity_id.to_owned()],
                fetch_id: Some(fetch_id.to_owned()),
                auth_token: Some("tok-1".to_owned()),
            },
            Some(launch_doc),
        );
        let actor_param = format!(
            "%7B%22objectType%22%3A%22Agent%22%2C%22account%22%3A%7B%22homePage%22%3A%22https%3A%2F%2Flms.example%22%2C%22name%22%3A%22ada%22%7D%2C%22name%22%3A%22{}%22%7D",
            name.replace(' ', "%20")
        );
        let query = format!(
            "endpoint=https://lrs.example/xapi&fetch=https://lrs.example/fetch/{fetch_id}&actor={actor_param}&registration={registration}&activityId=https%3A%2F%2Fexample.com%2Fcourses%2Fbug-a8",
        );
        let params = LaunchParams::parse(&query, Flavor::Cmi5).expect("cmi5 params");
        let api = StrictLrsApi::new(lrs);
        let mut connecting =
            PlayerSession::begin_xapi(course, Flavor::Cmi5, params, Box::new(api.clone()));
        for step in 0..5 {
            match connecting.try_into_session(
                HostInfo {
                    learner_name: None,
                    date: Some("2026-11-21".to_owned()),
                },
                1,
                step * 10,
            ) {
                Ok(res) => return (res.expect("cmi5 session"), api),
                Err(next) => connecting = next,
            }
        }
        panic!("cmi5 handshake did not finish in 5 ticks");
    }

    // -----------------------------------------------------------------------
    // A5.5: RunJavaScript / media followup effect ordering (BUG-A8)
    // -----------------------------------------------------------------------

    #[test]
    fn a5_5_bug_a8_run_javascript_followup_for_scorm12_scorm2004_and_cmi5() {
        let course = js_jump_course();
        let host_info = HostInfo {
            learner_name: None,
            date: Some("2026-11-21".to_owned()),
        };
        let mut js_handler = |_player: &Player<'_>, code: &str| -> Vec<Input> {
            if code == "CourseCraft.jump('s-end')" {
                vec![Input::Jump {
                    slide: "s-end".to_owned(),
                }]
            } else {
                Vec::new()
            }
        };
        let mut no_media = |_: &Player<'_>, _: &[Effect]| Vec::new();

        // 1. SCORM 1.2
        let scorm12 = Rc::new(RefCell::new(StrictScorm12::new()));
        let mut s12 = PlayerSession::start_scorm(
            &course,
            Standard::Scorm12,
            Box::new(StrictScorm12Api(Rc::clone(&scorm12))),
            host_info.clone(),
            1,
        )
        .expect("scorm12 session");
        s12.handle_input(&Input::Start, 100, 100, &mut js_handler, &mut no_media);
        s12.handle_input(
            &Input::Jump {
                slide: "s-q".to_owned(),
            },
            200,
            200,
            &mut js_handler,
            &mut no_media,
        );
        s12.handle_input(
            &Input::SelectChoice {
                choice: "t".to_owned(),
            },
            250,
            250,
            &mut js_handler,
            &mut no_media,
        );
        s12.handle_input(&Input::Submit, 300, 300, &mut js_handler, &mut no_media);
        s12.handle_input(
            &Input::Jump {
                slide: "s-1".to_owned(),
            },
            400,
            400,
            &mut js_handler,
            &mut no_media,
        );
        s12.handle_input(
            &Input::Click {
                object: "btn".to_owned(),
            },
            500,
            500,
            &mut js_handler,
            &mut no_media,
        );
        assert!(s12.ended);
        {
            let lms = scorm12.borrow();
            assert!(
                lms.violations().is_empty(),
                "1.2 violations: {:?}",
                lms.violations()
            );
            let snap = lms.snapshot();
            assert_eq!(snap["lesson_status"], "passed");
            assert_eq!(snap["score"]["raw"], "100");
            assert_eq!(snap["lesson_location"], "s-end");
        }

        // 2. SCORM 2004
        let scorm2004 = Rc::new(RefCell::new(StrictScorm2004::new()));
        let mut s2004 = PlayerSession::start_scorm(
            &course,
            Standard::Scorm2004,
            Box::new(StrictScorm2004Api(Rc::clone(&scorm2004))),
            host_info,
            1,
        )
        .expect("scorm2004 session");
        s2004.handle_input(&Input::Start, 100, 100, &mut js_handler, &mut no_media);
        s2004.handle_input(
            &Input::Jump {
                slide: "s-q".to_owned(),
            },
            200,
            200,
            &mut js_handler,
            &mut no_media,
        );
        s2004.handle_input(
            &Input::SelectChoice {
                choice: "t".to_owned(),
            },
            250,
            250,
            &mut js_handler,
            &mut no_media,
        );
        s2004.handle_input(&Input::Submit, 300, 300, &mut js_handler, &mut no_media);
        s2004.handle_input(
            &Input::Jump {
                slide: "s-1".to_owned(),
            },
            400,
            400,
            &mut js_handler,
            &mut no_media,
        );
        s2004.handle_input(
            &Input::Click {
                object: "btn".to_owned(),
            },
            500,
            500,
            &mut js_handler,
            &mut no_media,
        );
        assert!(s2004.ended);
        {
            let lms = scorm2004.borrow();
            assert!(
                lms.violations().is_empty(),
                "2004 violations: {:?}",
                lms.violations()
            );
            let snap = lms.snapshot();
            assert_eq!(snap["success_status"], "passed");
            assert_eq!(snap["score"]["raw"], "100");
            assert_eq!(snap["location"], "s-end");
        }

        // 3. cmi5
        let lrs = Rc::new(RefCell::new(StrictLrs::new()));
        let (mut cmi5, _api) = start_cmi5_session(&course, Rc::clone(&lrs), None);
        cmi5.handle_input(&Input::Start, 100, 100, &mut js_handler, &mut no_media);
        cmi5.handle_input(
            &Input::Jump {
                slide: "s-q".to_owned(),
            },
            200,
            200,
            &mut js_handler,
            &mut no_media,
        );
        cmi5.handle_input(
            &Input::SelectChoice {
                choice: "t".to_owned(),
            },
            250,
            250,
            &mut js_handler,
            &mut no_media,
        );
        cmi5.handle_input(&Input::Submit, 300, 300, &mut js_handler, &mut no_media);
        cmi5.handle_input(
            &Input::Jump {
                slide: "s-1".to_owned(),
            },
            400,
            400,
            &mut js_handler,
            &mut no_media,
        );
        cmi5.handle_input(
            &Input::Click {
                object: "btn".to_owned(),
            },
            500,
            500,
            &mut js_handler,
            &mut no_media,
        );
        cmi5.tick(600);
        assert!(cmi5.ended);
        let lrs_ref = lrs.borrow();
        assert!(
            lrs_ref.violations().is_empty(),
            "cmi5 violations: {:?}",
            lrs_ref.violations()
        );
        let verbs: Vec<&str> = lrs_ref
            .statements()
            .iter()
            .filter_map(|s| s["verb"]["id"].as_str())
            .collect();
        let passed_idx = verbs
            .iter()
            .position(|v| v.ends_with("/passed"))
            .expect("passed emitted");
        let term_idx = verbs
            .iter()
            .position(|v| v.ends_with("/terminated"))
            .expect("terminated emitted");
        assert!(
            passed_idx < term_idx,
            "passed must arrive before terminated: {verbs:?}"
        );
    }

    // -----------------------------------------------------------------------
    // A5.6: package-marker fallback & hostile marker
    // -----------------------------------------------------------------------

    #[test]
    fn a5_6_package_marker_all_standards_and_hostile_inputs() {
        assert_eq!(
            reporting_from_marker(None),
            (ReportingStandard::Scorm(Standard::Scorm12), None)
        );
        assert_eq!(
            reporting_from_marker(Some(r#"{"standard":"scorm12"}"#)),
            (ReportingStandard::Scorm(Standard::Scorm12), None)
        );
        assert_eq!(
            reporting_from_marker(Some(r#"{"standard":"scorm2004"}"#)),
            (ReportingStandard::Scorm(Standard::Scorm2004), None)
        );
        assert_eq!(
            reporting_from_marker(Some(r#"{"standard":"cmi5"}"#)),
            (ReportingStandard::Xapi(Flavor::Cmi5), None)
        );
        assert_eq!(
            reporting_from_marker(Some(r#"{"standard":"xapi"}"#)),
            (ReportingStandard::Xapi(Flavor::Xapi), None)
        );
        assert_eq!(
            reporting_from_marker(Some(r#"{"standard":"lti"}"#)),
            (ReportingStandard::Lti, None)
        );
        assert_eq!(
            reporting_from_marker(Some(r#"{"standard":"web"}"#)),
            (ReportingStandard::Web, None)
        );

        let (std_unknown, warn_unknown) = reporting_from_marker(Some(r#"{"standard":"scorm99"}"#));
        assert_eq!(std_unknown, ReportingStandard::Scorm(Standard::Scorm12));
        assert!(warn_unknown.is_some());

        let (std_corrupt, warn_corrupt) = reporting_from_marker(Some("{corrupt"));
        assert_eq!(std_corrupt, ReportingStandard::Scorm(Standard::Scorm12));
        assert!(warn_corrupt.is_some());
    }

    // -----------------------------------------------------------------------
    // A5.7: WebResume lifecycle & Start Over
    // -----------------------------------------------------------------------

    #[test]
    fn a5_7_web_resume_lifecycle_and_start_over() {
        let hello = include_str!("../../../examples/hello-course/course.json");
        let course = Course::from_json(hello).expect("hello course");
        let storage = MemoryStorage::default();
        let key = storage_key(&course);

        #[derive(Clone)]
        struct SharedStorage(Rc<RefCell<MemoryStorage>>);
        impl SuspendStorage for SharedStorage {
            fn get(&self, k: &str) -> Option<String> {
                self.0.borrow().get(k)
            }
            fn set(&mut self, k: &str, v: &str) {
                self.0.borrow_mut().set(k, v);
            }
            fn remove(&mut self, k: &str) {
                self.0.borrow_mut().remove(k);
            }
        }

        let shared = Rc::new(RefCell::new(storage));
        let host_info = HostInfo {
            learner_name: None,
            date: Some("2026-11-21".to_owned()),
        };
        let mut no_js = |_: &Player<'_>, _: &str| Vec::new();
        let mut no_media = |_: &Player<'_>, _: &[Effect]| Vec::new();

        let second_slide_id;
        {
            let mut s1 = PlayerSession::start_web(
                &course,
                Box::new(SharedStorage(Rc::clone(&shared))),
                host_info.clone(),
                1,
            )
            .expect("start web");
            s1.handle_input(&Input::Start, 10, 10, &mut no_js, &mut no_media);
            let first_slide = s1.player.state().slide;
            s1.handle_input(&Input::Next, 20, 20, &mut no_js, &mut no_media);
            second_slide_id = s1.player.state().slide;
            assert_ne!(second_slide_id, first_slide);
            s1.on_page_close(30);
        }
        assert!(shared.borrow().raw(&key).is_some());

        let mut s2 = PlayerSession::start_web(
            &course,
            Box::new(SharedStorage(Rc::clone(&shared))),
            host_info.clone(),
            2,
        )
        .expect("resume web");
        assert_eq!(s2.player.state().slide, second_slide_id);

        s2.restart(host_info.clone(), 3, 40, 40).expect("restart");
        assert_eq!(s2.player.state().slide, 0);
        let s3 = PlayerSession::start_web(
            &course,
            Box::new(SharedStorage(Rc::clone(&shared))),
            host_info,
            4,
        )
        .expect("fresh after restart");
        assert_eq!(s3.player.state().slide, 0);
    }

    // -----------------------------------------------------------------------
    // A5.8: pagehide / beforeunload idempotence
    // -----------------------------------------------------------------------

    #[test]
    fn a5_8_page_close_is_idempotent_across_scorm12_scorm2004_and_cmi5() {
        let hello = include_str!("../../../examples/hello-course/course.json");
        let course = Course::from_json(hello).expect("hello course");
        let host_info = HostInfo {
            learner_name: None,
            date: Some("2026-11-21".to_owned()),
        };
        let mut no_js = |_: &Player<'_>, _: &str| Vec::new();
        let mut no_media = |_: &Player<'_>, _: &[Effect]| Vec::new();

        // SCORM 1.2
        let scorm12 = Rc::new(RefCell::new(StrictScorm12::new()));
        let mut s12 = PlayerSession::start_scorm(
            &course,
            Standard::Scorm12,
            Box::new(StrictScorm12Api(Rc::clone(&scorm12))),
            host_info.clone(),
            1,
        )
        .expect("s12");
        s12.handle_input(&Input::Start, 100, 100, &mut no_js, &mut no_media);
        s12.on_page_close(200);
        s12.on_page_close(210);
        {
            let lms = scorm12.borrow();
            let finish_calls = lms
                .calls()
                .iter()
                .filter(|c| c.method == "LMSFinish")
                .count();
            assert_eq!(finish_calls, 1);
            assert!(lms.violations().is_empty(), "{:?}", lms.violations());
        }

        // SCORM 2004
        let scorm2004 = Rc::new(RefCell::new(StrictScorm2004::new()));
        let mut s2004 = PlayerSession::start_scorm(
            &course,
            Standard::Scorm2004,
            Box::new(StrictScorm2004Api(Rc::clone(&scorm2004))),
            host_info,
            1,
        )
        .expect("s2004");
        s2004.handle_input(&Input::Start, 100, 100, &mut no_js, &mut no_media);
        s2004.on_page_close(200);
        s2004.on_page_close(210);
        {
            let lms = scorm2004.borrow();
            let term_calls = lms
                .calls()
                .iter()
                .filter(|c| c.method == "Terminate")
                .count();
            assert_eq!(term_calls, 1);
            assert!(lms.violations().is_empty(), "{:?}", lms.violations());
        }

        // cmi5
        let lrs = Rc::new(RefCell::new(StrictLrs::new()));
        let (mut cmi5, _api) = start_cmi5_session(&course, Rc::clone(&lrs), None);
        cmi5.handle_input(&Input::Start, 100, 100, &mut no_js, &mut no_media);
        cmi5.on_page_close(200);
        cmi5.on_page_close(210);
        let lrs_ref = lrs.borrow();
        let terminated_count = lrs_ref
            .statements()
            .iter()
            .filter(|s| {
                s["verb"]["id"]
                    .as_str()
                    .is_some_and(|v| v.ends_with("/terminated"))
            })
            .count();
        assert_eq!(terminated_count, 1);
        assert!(
            lrs_ref.violations().is_empty(),
            "{:?}",
            lrs_ref.violations()
        );
    }

    // -----------------------------------------------------------------------
    // A5.9: HostInfo (Learner.Name & Course.Date) across all 4 standards
    // -----------------------------------------------------------------------

    #[test]
    fn a5_9_host_info_resolves_learner_name_and_date_across_all_standards() {
        let hello = include_str!("../../../examples/hello-course/course.json");
        let course = Course::from_json(hello).expect("hello course");
        let date = Some("2026-11-21".to_owned());

        // SCORM 1.2
        let cfg12 = Scorm12Config {
            student_name: "Grace Hopper".to_owned(),
            ..Default::default()
        };
        let lms12 = StrictScorm12::with_config(cfg12);
        let s12 = PlayerSession::start_scorm(
            &course,
            Standard::Scorm12,
            Box::new(StrictScorm12Api(Rc::new(RefCell::new(lms12)))),
            HostInfo {
                learner_name: None,
                date: date.clone(),
            },
            1,
        )
        .expect("s12");
        assert_eq!(
            s12.player.display_text("%Learner.Name% %Course.Date%"),
            "Grace Hopper 2026-11-21"
        );

        // SCORM 2004
        let cfg2004 = Scorm2004Config {
            learner_name: "Alan Turing".to_owned(),
            ..Default::default()
        };
        let lms2004 = StrictScorm2004::with_config(cfg2004);
        let s2004 = PlayerSession::start_scorm(
            &course,
            Standard::Scorm2004,
            Box::new(StrictScorm2004Api(Rc::new(RefCell::new(lms2004)))),
            HostInfo {
                learner_name: None,
                date: date.clone(),
            },
            1,
        )
        .expect("s2004");
        assert_eq!(
            s2004.player.display_text("%Learner.Name% %Course.Date%"),
            "Alan Turing 2026-11-21"
        );

        // cmi5
        let lrs = Rc::new(RefCell::new(StrictLrs::new()));
        let (cmi5, _) = start_cmi5_session(&course, lrs, Some("Ada Lovelace"));
        assert_eq!(
            cmi5.player.display_text("%Learner.Name% %Course.Date%"),
            "Ada Lovelace 2026-11-21"
        );

        // LTI (URL-encoded + >200 char name)
        let long_name = "A".repeat(250);
        let query = format!("?name={long_name}%20End");
        let parsed_name = learner_name_from_query(&query);
        let lti = PlayerSession::start_lti(
            &course,
            Box::new(MockLtiTransport::default()),
            HostInfo {
                learner_name: parsed_name,
                date,
            },
            1,
        )
        .expect("lti");
        let rendered = lti.player.display_text("%Learner.Name% %Course.Date%");
        assert!(rendered.starts_with("AAAA"));
        assert!(rendered.ends_with(" 2026-11-21"));
    }

    // -----------------------------------------------------------------------
    // A5.10: reporting_lost() banner trigger across cmi5 & LTI
    // -----------------------------------------------------------------------

    #[test]
    fn a5_10_reporting_lost_triggers_on_cmi5_401_and_lti_double_500() {
        let course = js_jump_course();
        let mut no_js = |_: &Player<'_>, _: &str| Vec::new();
        let mut no_media = |_: &Player<'_>, _: &[Effect]| Vec::new();

        // cmi5 401 mid-session
        let lrs = Rc::new(RefCell::new(StrictLrs::new()));
        let (mut cmi5, mut api) = start_cmi5_session(&course, Rc::clone(&lrs), None);
        cmi5.handle_input(&Input::Start, 10, 10, &mut no_js, &mut no_media);
        assert!(!cmi5.reporting_lost());

        api.forced_status = Some(401);
        if let ReportingHost::Xapi {
            api: ref mut boxed_api,
            ..
        } = cmi5.host
        {
            *boxed_api = Box::new(api.clone());
        }
        cmi5.handle_input(
            &Input::Jump {
                slide: "s-q".to_owned(),
            },
            20,
            20,
            &mut no_js,
            &mut no_media,
        );
        cmi5.handle_input(
            &Input::SelectChoice {
                choice: "t".to_owned(),
            },
            22,
            22,
            &mut no_js,
            &mut no_media,
        );
        cmi5.handle_input(&Input::Submit, 25, 25, &mut no_js, &mut no_media);
        cmi5.tick(30);
        assert!(cmi5.reporting_lost());
        let requests_after_loss = api.sent_urls.borrow().len();
        cmi5.handle_input(
            &Input::Jump {
                slide: "s-1".to_owned(),
            },
            40,
            40,
            &mut no_js,
            &mut no_media,
        );
        cmi5.tick(50);
        assert_eq!(api.sent_urls.borrow().len(), requests_after_loss);

        // LTI 2x 500
        let transport = MockLtiTransport::default();
        let mut lti = PlayerSession::start_lti(
            &course,
            Box::new(transport.clone()),
            HostInfo {
                learner_name: None,
                date: Some("2026-11-21".to_owned()),
            },
            1,
        )
        .expect("lti");
        lti.handle_input(&Input::Start, 10, 10, &mut no_js, &mut no_media);
        lti.handle_input(
            &Input::Jump {
                slide: "s-q".to_owned(),
            },
            20,
            20,
            &mut no_js,
            &mut no_media,
        );
        lti.handle_input(
            &Input::SelectChoice {
                choice: "t".to_owned(),
            },
            25,
            25,
            &mut no_js,
            &mut no_media,
        );
        lti.handle_input(&Input::Submit, 30, 30, &mut no_js, &mut no_media);
        // Trigger SubmitResults via button click (without JS jump)
        lti.handle_input(
            &Input::Jump {
                slide: "s-1".to_owned(),
            },
            40,
            40,
            &mut no_js,
            &mut no_media,
        );
        lti.handle_input(
            &Input::Click {
                object: "btn".to_owned(),
            },
            50,
            50,
            &mut no_js,
            &mut no_media,
        );
        assert_eq!(transport.sent.borrow().len(), 1);
        assert!(!lti.reporting_lost());

        transport.queued_statuses.borrow_mut().push(500);
        lti.tick(60);
        assert_eq!(transport.sent.borrow().len(), 2, "retried once");
        assert!(!lti.reporting_lost());

        transport.queued_statuses.borrow_mut().push(500);
        lti.tick(70);
        assert!(lti.reporting_lost());
        assert_eq!(
            transport.sent.borrow().len(),
            2,
            "no further requests after lost"
        );
    }
}
