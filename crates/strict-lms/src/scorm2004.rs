//! Strict SCORM 2004 (3rd & 4th Edition) Run-Time Environment engine (RTE §3).

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    CallRecord, Violation, is_valid_iso8601_duration, is_valid_scorm_identifier, parse_indexed_key,
    parse_plain_decimal,
};

/// SCORM 2004 edition (controls `cmi.suspend_data` and interaction limits).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Scorm2004Edition {
    Third,
    #[default]
    Fourth,
}

impl Scorm2004Edition {
    fn suspend_data_limit(self) -> usize {
        match self {
            Self::Third => 4_000,
            Self::Fourth => 64_000,
        }
    }
}

/// Configuration for a [`StrictScorm2004`] LMS instance.
#[derive(Debug, Clone)]
pub struct Scorm2004Config {
    pub edition: Scorm2004Edition,
    pub learner_id: String,
    pub learner_name: String,
    pub credit: String,
    pub mode: String,
    pub launch_data: String,
    pub scaled_passing_score: Option<f64>,
    pub max_time_allowed: String,
    pub time_limit_action: String,
    pub completion_threshold: Option<f64>,
    pub clear_on_normal_exit: bool,
}

impl Default for Scorm2004Config {
    fn default() -> Self {
        Self {
            edition: Scorm2004Edition::Fourth,
            learner_id: "learner-1".to_owned(),
            learner_name: "Learner, Test".to_owned(),
            credit: "credit".to_owned(),
            mode: "normal".to_owned(),
            launch_data: String::new(),
            scaled_passing_score: None,
            max_time_allowed: String::new(),
            time_limit_action: String::new(),
            completion_threshold: None,
            clear_on_normal_exit: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LifecycleState {
    NotInitialized,
    Running,
    Terminated,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Scorm2004Score {
    raw: Option<String>,
    min: Option<String>,
    max: Option<String>,
    scaled: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Scorm2004Interaction {
    id: String,
    kind: Option<String>,
    description: Option<String>,
    weighting: Option<String>,
    learner_response: Option<String>,
    result: Option<String>,
    timestamp: Option<String>,
    latency: Option<String>,
    objectives: Vec<String>,
    correct_responses: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedScorm2004 {
    entry: String,
    completion_status: String,
    success_status: String,
    location: Option<String>,
    suspend_data: Option<String>,
    progress_measure: Option<String>,
    exit: String,
    total_time_cs: u64,
    nav_request: String,
    score: Scorm2004Score,
    interactions: Vec<Scorm2004Interaction>,
}

/// Strict SCORM 2004 3rd/4th Edition RTE conformance engine.
#[derive(Debug, Clone)]
pub struct StrictScorm2004 {
    config: Scorm2004Config,
    state: LifecycleState,
    last_error: &'static str,
    entry: String,
    completion_status: String,
    success_status: String,
    location: Option<String>,
    suspend_data: Option<String>,
    progress_measure: Option<String>,
    exit: String,
    session_time: Option<String>,
    total_time_cs: u64,
    nav_request: String,
    score: Scorm2004Score,
    interactions: Vec<Scorm2004Interaction>,
    calls: Vec<CallRecord>,
    violations: Vec<Violation>,
    warnings: Vec<Violation>,
}

impl StrictScorm2004 {
    /// Create a new SCORM 2004 4th Edition strict LMS engine.
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(Scorm2004Config::default())
    }

    /// Create a new SCORM 2004 strict LMS engine with custom configuration.
    #[must_use]
    pub fn with_config(config: Scorm2004Config) -> Self {
        Self {
            config,
            state: LifecycleState::NotInitialized,
            last_error: "0",
            entry: "ab-initio".to_owned(),
            completion_status: "unknown".to_owned(),
            success_status: "unknown".to_owned(),
            location: None,
            suspend_data: None,
            progress_measure: None,
            exit: String::new(),
            session_time: None,
            total_time_cs: 0,
            nav_request: "_none_".to_owned(),
            score: Scorm2004Score::default(),
            interactions: Vec::new(),
            calls: Vec::new(),
            violations: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Seed initial `cmi.suspend_data`.
    pub fn seed_suspend_data(&mut self, data: impl Into<String>) {
        self.suspend_data = Some(data.into());
    }

    /// Seed initial `cmi.success_status` and optional `cmi.score.raw`.
    pub fn seed_status(&mut self, success_status: impl Into<String>, score_raw: Option<&str>) {
        self.success_status = success_status.into();
        if let Some(raw) = score_raw {
            self.score.raw = Some(raw.to_owned());
        }
    }

    /// All MUST/SHALL specification violations recorded so far.
    #[must_use]
    pub fn violations(&self) -> &[Violation] {
        &self.violations
    }

    /// Advisory / informational notes (including legal `403` uninitialized reads).
    #[must_use]
    pub fn warnings(&self) -> &[Violation] {
        &self.warnings
    }

    /// All API calls recorded so far.
    #[must_use]
    pub fn calls(&self) -> &[CallRecord] {
        &self.calls
    }

    /// Snapshot of current LMS state as JSON.
    #[must_use]
    pub fn snapshot(&self) -> serde_json::Value {
        json!({
            "standard": "scorm2004",
            "edition": format!("{:?}", self.config.edition),
            "state": format!("{:?}", self.state),
            "last_error": self.last_error,
            "entry": self.entry,
            "completion_status": self.completion_status,
            "success_status": self.success_status,
            "location": self.location,
            "suspend_data": self.suspend_data,
            "progress_measure": self.progress_measure,
            "exit": self.exit,
            "total_time": format_iso_duration_cs(self.total_time_cs),
            "nav_request": self.nav_request,
            "score": self.score,
            "interactions": self.interactions,
            "violations_count": self.violations.len(),
            "warnings_count": self.warnings.len(),
        })
    }

    /// Simulate browser close + new LMS launch using persisted attempt data.
    pub fn relaunch(&mut self) {
        if self.exit == "suspend" {
            self.entry = "resume".to_owned();
        } else {
            self.entry = "ab-initio".to_owned();
            if self.config.clear_on_normal_exit {
                self.completion_status = "unknown".to_owned();
                self.success_status = "unknown".to_owned();
                self.location = None;
                self.suspend_data = None;
                self.progress_measure = None;
                self.score = Scorm2004Score::default();
                self.interactions.clear();
            }
        }
        self.state = LifecycleState::NotInitialized;
        self.last_error = "0";
        self.exit.clear();
        self.session_time = None;
        self.nav_request = "_none_".to_owned();
    }

    /// Export persisted learner attempt state as JSON (for `localStorage` in `mock-lms`).
    #[must_use]
    pub fn export_persisted_json(&self) -> String {
        let persisted = PersistedScorm2004 {
            entry: self.entry.clone(),
            completion_status: self.completion_status.clone(),
            success_status: self.success_status.clone(),
            location: self.location.clone(),
            suspend_data: self.suspend_data.clone(),
            progress_measure: self.progress_measure.clone(),
            exit: self.exit.clone(),
            total_time_cs: self.total_time_cs,
            nav_request: self.nav_request.clone(),
            score: self.score.clone(),
            interactions: self.interactions.clone(),
        };
        serde_json::to_string(&persisted).unwrap_or_else(|_| "{}".to_owned())
    }

    /// Restore persisted learner attempt state from JSON (resets lifecycle to `NotInitialized`).
    pub fn import_persisted_json(&mut self, json: &str) -> Result<(), String> {
        let p: PersistedScorm2004 =
            serde_json::from_str(json).map_err(|e| format!("invalid persisted state: {e}"))?;
        self.entry = p.entry;
        self.completion_status = p.completion_status;
        self.success_status = p.success_status;
        self.location = p.location;
        self.suspend_data = p.suspend_data;
        self.progress_measure = p.progress_measure;
        self.exit = p.exit;
        self.total_time_cs = p.total_time_cs;
        self.nav_request = p.nav_request;
        self.score = p.score;
        self.interactions = p.interactions;
        self.state = LifecycleState::NotInitialized;
        self.last_error = "0";
        self.session_time = None;
        Ok(())
    }

    /// Flag a violation if the SCO is still `Running` without calling `Terminate("")`.
    pub fn assert_clean_terminate(&mut self) -> bool {
        if self.state == LifecycleState::Running {
            let idx = self.calls.len() + 1;
            self.violations.push(Violation {
                standard: "scorm2004",
                rule: "SCORM2004-UNCLEAN-TERMINATE",
                call_index: idx,
                element: "Terminate".to_owned(),
                message: "Session ended while Running without calling Terminate(\"\")".to_owned(),
                error_code: None,
            });
            false
        } else {
            true
        }
    }

    /// `Initialize(arg)`
    pub fn initialize(&mut self, arg: &str) -> &'static str {
        let idx = self.next_call_index();
        let (res, err) = if !arg.is_empty() {
            self.record_violation(
                idx,
                "SCORM2004-201-ARG",
                "Initialize",
                format!("Initialize argument must be empty string, got {arg:?}"),
                "201",
            );
            ("false", "201")
        } else {
            match self.state {
                LifecycleState::NotInitialized => {
                    self.state = LifecycleState::Running;
                    ("true", "0")
                }
                LifecycleState::Running => {
                    self.record_violation(
                        idx,
                        "SCORM2004-103-ALREADY-INIT",
                        "Initialize",
                        "Initialize called when already Running".to_owned(),
                        "103",
                    );
                    ("false", "103")
                }
                LifecycleState::Terminated => {
                    self.record_violation(
                        idx,
                        "SCORM2004-104-TERMINATED",
                        "Initialize",
                        "Initialize called after Terminate".to_owned(),
                        "104",
                    );
                    ("false", "104")
                }
            }
        };
        self.finish_call(idx, "Initialize", vec![arg.to_owned()], res, err);
        res
    }

    /// `GetValue(element)`
    pub fn get_value(&mut self, element: &str) -> String {
        let idx = self.next_call_index();
        match self.state {
            LifecycleState::NotInitialized => {
                self.record_violation(
                    idx,
                    "SCORM2004-122-GET-BEFORE-INIT",
                    element,
                    format!("GetValue({element:?}) called before Initialize"),
                    "122",
                );
                self.finish_call(idx, "GetValue", vec![element.to_owned()], "", "122");
                return String::new();
            }
            LifecycleState::Terminated => {
                self.record_violation(
                    idx,
                    "SCORM2004-123-GET-AFTER-TERM",
                    element,
                    format!("GetValue({element:?}) called after Terminate"),
                    "123",
                );
                self.finish_call(idx, "GetValue", vec![element.to_owned()], "", "123");
                return String::new();
            }
            LifecycleState::Running => {}
        }
        let (val, err) = self.get_element(idx, element);
        self.finish_call(idx, "GetValue", vec![element.to_owned()], &val, err);
        val
    }

    /// `SetValue(element, value)`
    pub fn set_value(&mut self, element: &str, value: &str) -> &'static str {
        let idx = self.next_call_index();
        match self.state {
            LifecycleState::NotInitialized => {
                self.record_violation(
                    idx,
                    "SCORM2004-132-SET-BEFORE-INIT",
                    element,
                    format!("SetValue({element:?}) called before Initialize"),
                    "132",
                );
                self.finish_call(
                    idx,
                    "SetValue",
                    vec![element.to_owned(), value.to_owned()],
                    "false",
                    "132",
                );
                return "false";
            }
            LifecycleState::Terminated => {
                self.record_violation(
                    idx,
                    "SCORM2004-133-SET-AFTER-TERM",
                    element,
                    format!("SetValue({element:?}) called after Terminate"),
                    "133",
                );
                self.finish_call(
                    idx,
                    "SetValue",
                    vec![element.to_owned(), value.to_owned()],
                    "false",
                    "133",
                );
                return "false";
            }
            LifecycleState::Running => {}
        }
        let err = self.set_element(idx, element, value);
        let res = if err == "0" { "true" } else { "false" };
        self.finish_call(
            idx,
            "SetValue",
            vec![element.to_owned(), value.to_owned()],
            res,
            err,
        );
        res
    }

    /// `Commit(arg)`
    pub fn commit(&mut self, arg: &str) -> &'static str {
        let idx = self.next_call_index();
        let (res, err) = match self.state {
            LifecycleState::NotInitialized => {
                self.record_violation(
                    idx,
                    "SCORM2004-142-COMMIT-BEFORE-INIT",
                    "Commit",
                    "Commit called before Initialize".to_owned(),
                    "142",
                );
                ("false", "142")
            }
            LifecycleState::Terminated => {
                self.record_violation(
                    idx,
                    "SCORM2004-143-COMMIT-AFTER-TERM",
                    "Commit",
                    "Commit called after Terminate".to_owned(),
                    "143",
                );
                ("false", "143")
            }
            LifecycleState::Running => {
                if !arg.is_empty() {
                    self.record_violation(
                        idx,
                        "SCORM2004-201-ARG",
                        "Commit",
                        format!("Commit argument must be empty string, got {arg:?}"),
                        "201",
                    );
                    ("false", "201")
                } else {
                    ("true", "0")
                }
            }
        };
        self.finish_call(idx, "Commit", vec![arg.to_owned()], res, err);
        res
    }

    /// `Terminate(arg)`
    pub fn terminate(&mut self, arg: &str) -> &'static str {
        let idx = self.next_call_index();
        let (res, err) = match self.state {
            LifecycleState::NotInitialized => {
                self.record_violation(
                    idx,
                    "SCORM2004-112-TERM-BEFORE-INIT",
                    "Terminate",
                    "Terminate called before Initialize".to_owned(),
                    "112",
                );
                ("false", "112")
            }
            LifecycleState::Terminated => {
                self.record_violation(
                    idx,
                    "SCORM2004-113-TERM-AFTER-TERM",
                    "Terminate",
                    "Terminate called after Terminate".to_owned(),
                    "113",
                );
                ("false", "113")
            }
            LifecycleState::Running => {
                if !arg.is_empty() {
                    self.record_violation(
                        idx,
                        "SCORM2004-201-ARG",
                        "Terminate",
                        format!("Terminate argument must be empty string, got {arg:?}"),
                        "201",
                    );
                    ("false", "201")
                } else {
                    if let Some(st) = &self.session_time {
                        self.total_time_cs =
                            self.total_time_cs.saturating_add(parse_iso_duration_cs(st));
                    }
                    self.state = LifecycleState::Terminated;
                    ("true", "0")
                }
            }
        };
        self.finish_call(idx, "Terminate", vec![arg.to_owned()], res, err);
        res
    }

    /// `GetLastError()`
    #[must_use]
    pub fn get_last_error(&self) -> &'static str {
        self.last_error
    }

    /// `GetErrorString(code)`
    #[must_use]
    pub fn get_error_string(&self, code: &str) -> &'static str {
        error_string_2004(code)
    }

    /// `GetDiagnostic(code)`
    #[must_use]
    pub fn get_diagnostic(&self, code: &str) -> String {
        let effective = if code.is_empty() {
            self.last_error
        } else {
            code
        };
        format!("{effective}: {}", error_string_2004(effective))
    }

    /// Unified string-based dispatch entrypoint (used by WASM parity tests and `mock-lms`).
    pub fn dispatch(&mut self, method: &str, args: &[&str]) -> String {
        let a0 = args.first().copied().unwrap_or("");
        let a1 = args.get(1).copied().unwrap_or("");
        match method {
            "Initialize" => self.initialize(a0).to_owned(),
            "GetValue" => self.get_value(a0),
            "SetValue" => self.set_value(a0, a1).to_owned(),
            "Commit" => self.commit(a0).to_owned(),
            "Terminate" => self.terminate(a0).to_owned(),
            "GetLastError" => self.get_last_error().to_owned(),
            "GetErrorString" => self.get_error_string(a0).to_owned(),
            "GetDiagnostic" => self.get_diagnostic(a0),
            _ => String::new(),
        }
    }

    fn next_call_index(&self) -> usize {
        self.calls.len() + 1
    }

    fn finish_call(
        &mut self,
        index: usize,
        method: &str,
        args: Vec<String>,
        result: &str,
        error_code: &'static str,
    ) {
        self.last_error = error_code;
        self.calls.push(CallRecord {
            index,
            method: method.to_owned(),
            args,
            result: result.to_owned(),
            error_code: error_code.to_owned(),
        });
    }

    fn record_violation(
        &mut self,
        call_index: usize,
        rule: &'static str,
        element: &str,
        message: String,
        error_code: &'static str,
    ) {
        self.violations.push(Violation {
            standard: "scorm2004",
            rule,
            call_index,
            element: element.to_owned(),
            message,
            error_code: Some(error_code.to_owned()),
        });
    }

    fn record_uninit_note(&mut self, call_index: usize, element: &str) {
        self.warnings.push(Violation {
            standard: "scorm2004",
            rule: "SCORM2004-403-UNINIT",
            call_index,
            element: element.to_owned(),
            message: format!("{element} read before initialization (returns \"\" with 403)"),
            error_code: Some("403".to_owned()),
        });
    }

    fn get_element(&mut self, idx: usize, element: &str) -> (String, &'static str) {
        if let Some(val) = self.get_keyword_or_ro(element) {
            return (val, "0");
        }
        match element {
            "cmi.completion_status" => (self.completion_status.clone(), "0"),
            "cmi.success_status" => (self.success_status.clone(), "0"),
            "adl.nav.request" => (self.nav_request.clone(), "0"),
            "cmi.location" => self.opt_get(idx, element, self.location.clone()),
            "cmi.suspend_data" => self.opt_get(idx, element, self.suspend_data.clone()),
            "cmi.progress_measure" => self.opt_get(idx, element, self.progress_measure.clone()),
            "cmi.score.raw" => self.opt_get(idx, element, self.score.raw.clone()),
            "cmi.score.min" => self.opt_get(idx, element, self.score.min.clone()),
            "cmi.score.max" => self.opt_get(idx, element, self.score.max.clone()),
            "cmi.score.scaled" => self.opt_get(idx, element, self.score.scaled.clone()),
            "cmi.exit" | "cmi.session_time" => {
                self.record_violation(
                    idx,
                    "SCORM2004-405-WRITE-ONLY",
                    element,
                    format!("{element} is write-only in SCORM 2004"),
                    "405",
                );
                (String::new(), "405")
            }
            _ => self.get_indexed_or_unknown(idx, element),
        }
    }

    fn opt_get(
        &mut self,
        idx: usize,
        element: &str,
        val: Option<String>,
    ) -> (String, &'static str) {
        match val {
            Some(v) => (v, "0"),
            None => {
                self.record_uninit_note(idx, element);
                (String::new(), "403")
            }
        }
    }

    fn get_keyword_or_ro(&self, element: &str) -> Option<String> {
        let s = match element {
            "cmi._version" => "1.0".to_owned(),
            "cmi.learner_id" => self.config.learner_id.clone(),
            "cmi.learner_name" => self.config.learner_name.clone(),
            "cmi.entry" => self.entry.clone(),
            "cmi.mode" => self.config.mode.clone(),
            "cmi.credit" => self.config.credit.clone(),
            "cmi.total_time" => format_iso_duration_cs(self.total_time_cs),
            "cmi.launch_data" => self.config.launch_data.clone(),
            "cmi.scaled_passing_score" => self
                .config
                .scaled_passing_score
                .map_or_else(String::new, |v| v.to_string()),
            "cmi.max_time_allowed" => self.config.max_time_allowed.clone(),
            "cmi.time_limit_action" => self.config.time_limit_action.clone(),
            "cmi.completion_threshold" => self
                .config
                .completion_threshold
                .map_or_else(String::new, |v| v.to_string()),
            "cmi.score._children" => "scaled,raw,min,max".to_owned(),
            "cmi.interactions._children" => {
                "id,type,objectives,timestamp,correct_responses,weighting,learner_response,result,latency,description"
                    .to_owned()
            }
            "cmi.interactions._count" => self.interactions.len().to_string(),
            "adl.nav.request_valid.continue" | "adl.nav.request_valid.previous" => {
                "unknown".to_owned()
            }
            _ => {
                if let Some((n, tail)) = parse_indexed_key(element, "cmi.interactions.") {
                    if tail == "correct_responses._count" {
                        return Some(
                            self.interactions
                                .get(n)
                                .map_or(0, |i| i.correct_responses.len())
                                .to_string(),
                        );
                    }
                    if tail == "objectives._count" {
                        return Some(
                            self.interactions
                                .get(n)
                                .map_or(0, |i| i.objectives.len())
                                .to_string(),
                        );
                    }
                }
                if element.starts_with("adl.nav.request_valid.choice.")
                    || element.starts_with("adl.nav.request_valid.jump.")
                {
                    return Some("unknown".to_owned());
                }
                return None;
            }
        };
        Some(s)
    }

    fn get_indexed_or_unknown(&mut self, idx: usize, element: &str) -> (String, &'static str) {
        if let Some((n, tail)) = parse_indexed_key(element, "cmi.interactions.")
            && is_known_interaction_field_2004(tail)
        {
            let Some(inter) = self.interactions.get(n) else {
                self.record_uninit_note(idx, element);
                return (String::new(), "403");
            };
            let val = match tail {
                "id" => Some(inter.id.clone()),
                "type" => inter.kind.clone(),
                "description" => inter.description.clone(),
                "weighting" => inter.weighting.clone(),
                "learner_response" => inter.learner_response.clone(),
                "result" => inter.result.clone(),
                "timestamp" => inter.timestamp.clone(),
                "latency" => inter.latency.clone(),
                _ => {
                    if let Some((m, "pattern")) = parse_indexed_key(tail, "correct_responses.") {
                        inter.correct_responses.get(m).cloned()
                    } else if let Some((m, "id")) = parse_indexed_key(tail, "objectives.") {
                        inter.objectives.get(m).cloned()
                    } else {
                        None
                    }
                }
            };
            return self.opt_get(idx, element, val);
        }
        self.record_violation(
            idx,
            "SCORM2004-401-UNDEFINED",
            element,
            format!("Undefined SCORM 2004 element {element:?}"),
            "401",
        );
        (String::new(), "401")
    }

    fn set_element(&mut self, idx: usize, element: &str, value: &str) -> &'static str {
        if is_read_only_2004(element) {
            self.record_violation(
                idx,
                "SCORM2004-404-READ-ONLY",
                element,
                format!("Cannot set read-only element {element}"),
                "404",
            );
            return "404";
        }
        match element {
            "cmi.completion_status" => {
                if matches!(
                    value,
                    "completed" | "incomplete" | "not attempted" | "unknown"
                ) {
                    self.completion_status = value.to_owned();
                    "0"
                } else {
                    self.record_violation(
                        idx,
                        "SCORM2004-406-TYPE",
                        element,
                        format!("Invalid completion_status {value:?}"),
                        "406",
                    );
                    "406"
                }
            }
            "cmi.success_status" => {
                if matches!(value, "passed" | "failed" | "unknown") {
                    self.success_status = value.to_owned();
                    "0"
                } else {
                    self.record_violation(
                        idx,
                        "SCORM2004-406-TYPE",
                        element,
                        format!("Invalid success_status {value:?}"),
                        "406",
                    );
                    "406"
                }
            }
            "cmi.location" => {
                let len = value.chars().count();
                if len > 1_000 {
                    self.record_violation(
                        idx,
                        "SCORM2004-406-LENGTH",
                        element,
                        format!("cmi.location length {len} exceeds 1000 chars"),
                        "406",
                    );
                    "406"
                } else {
                    self.location = Some(value.to_owned());
                    "0"
                }
            }
            "cmi.suspend_data" => {
                let limit = self.config.edition.suspend_data_limit();
                let len = value.chars().count();
                if len > limit {
                    self.record_violation(
                        idx,
                        "SCORM2004-406-SUSPEND-LIMIT",
                        element,
                        format!("cmi.suspend_data length {len} exceeds {limit} chars"),
                        "406",
                    );
                    "406"
                } else {
                    self.suspend_data = Some(value.to_owned());
                    "0"
                }
            }
            "cmi.exit" => {
                if matches!(value, "" | "time-out" | "suspend" | "logout" | "normal") {
                    self.exit = value.to_owned();
                    "0"
                } else {
                    self.record_violation(
                        idx,
                        "SCORM2004-406-TYPE",
                        element,
                        format!("Invalid cmi.exit {value:?}"),
                        "406",
                    );
                    "406"
                }
            }
            "cmi.session_time" => {
                if is_valid_iso8601_duration(value) {
                    self.session_time = Some(value.to_owned());
                    "0"
                } else {
                    self.record_violation(
                        idx,
                        "SCORM2004-406-TIMEINTERVAL",
                        element,
                        format!("Invalid ISO 8601 session_time {value:?}"),
                        "406",
                    );
                    "406"
                }
            }
            "cmi.score.scaled" => self.set_bounded_real(idx, element, value, -1.0, 1.0, |s, v| {
                s.score.scaled = Some(v.to_owned());
            }),
            "cmi.progress_measure" => {
                self.set_bounded_real(idx, element, value, 0.0, 1.0, |s, v| {
                    s.progress_measure = Some(v.to_owned());
                })
            }
            "cmi.score.raw" => self.set_unbounded_real(idx, element, value, |s, v| {
                s.score.raw = Some(v.to_owned());
            }),
            "cmi.score.min" => self.set_unbounded_real(idx, element, value, |s, v| {
                s.score.min = Some(v.to_owned());
            }),
            "cmi.score.max" => self.set_unbounded_real(idx, element, value, |s, v| {
                s.score.max = Some(v.to_owned());
            }),
            "adl.nav.request" => {
                if is_valid_nav_request(value) {
                    self.nav_request = value.to_owned();
                    "0"
                } else {
                    self.record_violation(
                        idx,
                        "SCORM2004-406-NAV-REQUEST",
                        element,
                        format!("Invalid adl.nav.request {value:?}"),
                        "406",
                    );
                    "406"
                }
            }
            _ => self.set_indexed_or_unknown(idx, element, value),
        }
    }

    fn set_bounded_real(
        &mut self,
        idx: usize,
        element: &str,
        value: &str,
        min: f64,
        max: f64,
        assign: fn(&mut Self, &str),
    ) -> &'static str {
        let Some(num) = parse_plain_decimal(value) else {
            self.record_violation(
                idx,
                "SCORM2004-406-TYPE",
                element,
                format!("Invalid real(10,7) {value:?} for {element}"),
                "406",
            );
            return "406";
        };
        if !(min..=max).contains(&num) {
            self.record_violation(
                idx,
                "SCORM2004-407-RANGE",
                element,
                format!("{element} value {num} out of range {min}..={max}"),
                "407",
            );
            return "407";
        }
        assign(self, value);
        "0"
    }

    fn set_unbounded_real(
        &mut self,
        idx: usize,
        element: &str,
        value: &str,
        assign: fn(&mut Self, &str),
    ) -> &'static str {
        if parse_plain_decimal(value).is_none() {
            self.record_violation(
                idx,
                "SCORM2004-406-TYPE",
                element,
                format!("Invalid real(10,7) {value:?} for {element}"),
                "406",
            );
            return "406";
        }
        assign(self, value);
        "0"
    }

    fn set_indexed_or_unknown(&mut self, idx: usize, element: &str, value: &str) -> &'static str {
        if let Some((n, tail)) = parse_indexed_key(element, "cmi.interactions.") {
            return self.set_interaction_field(idx, element, n, tail, value);
        }
        self.record_violation(
            idx,
            "SCORM2004-401-UNDEFINED",
            element,
            format!("Undefined SCORM 2004 element {element:?}"),
            "401",
        );
        "401"
    }

    fn set_interaction_field(
        &mut self,
        idx: usize,
        element: &str,
        n: usize,
        tail: &str,
        value: &str,
    ) -> &'static str {
        if !is_known_interaction_field_2004(tail) {
            self.record_violation(
                idx,
                "SCORM2004-401-UNDEFINED",
                element,
                format!("Undefined interaction element {element:?}"),
                "401",
            );
            return "401";
        }
        if n > self.interactions.len() {
            self.record_violation(
                idx,
                "SCORM2004-351-INTERACTION-INDEX",
                element,
                format!(
                    "Interaction index {n} skips gap (current _count = {})",
                    self.interactions.len()
                ),
                "351",
            );
            return "351";
        }
        if n == self.interactions.len() && tail != "id" {
            self.record_violation(
                idx,
                "SCORM2004-408-DEPENDENCY",
                element,
                format!("cmi.interactions.{n}.id must be set before {tail}"),
                "408",
            );
            return "408";
        }
        if matches!(tail, "learner_response") || tail.starts_with("correct_responses.") {
            let has_type = self.interactions.get(n).is_some_and(|i| i.kind.is_some());
            if !has_type {
                self.record_violation(
                    idx,
                    "SCORM2004-408-DEPENDENCY",
                    element,
                    format!("cmi.interactions.{n}.type must be set before {tail}"),
                    "408",
                );
                return "408";
            }
        }
        if let Some((m, "pattern")) = parse_indexed_key(tail, "correct_responses.") {
            let cur_len = self
                .interactions
                .get(n)
                .map_or(0, |i| i.correct_responses.len());
            if m > cur_len {
                self.record_violation(
                    idx,
                    "SCORM2004-351-CORRECT-INDEX",
                    element,
                    format!("correct_responses index {m} skips gap (current _count = {cur_len})"),
                    "351",
                );
                return "351";
            }
            let kind = self
                .interactions
                .get(n)
                .and_then(|i| i.kind.as_deref())
                .unwrap_or("");
            if matches!(kind, "true-false" | "likert" | "other") && m >= 1 {
                self.record_violation(
                    idx,
                    "SCORM2004-351-MAX-CORRECT",
                    element,
                    format!("Interaction type {kind} allows at most 1 correct_response"),
                    "351",
                );
                return "351";
            }
        }
        let kind = self
            .interactions
            .get(n)
            .and_then(|i| i.kind.as_deref())
            .unwrap_or("");
        if let Err((code, rule, msg)) =
            validate_interaction_2004(tail, value, kind, self.config.edition)
        {
            self.record_violation(idx, rule, element, msg, code);
            return code;
        }
        if n == self.interactions.len() {
            self.interactions.push(Scorm2004Interaction {
                id: value.to_owned(),
                ..Scorm2004Interaction::default()
            });
            return "0";
        }
        if let Some(inter) = self.interactions.get_mut(n) {
            apply_interaction_field_2004(inter, tail, value);
        }
        "0"
    }
}

impl Default for StrictScorm2004 {
    fn default() -> Self {
        Self::new()
    }
}

fn is_read_only_2004(element: &str) -> bool {
    matches!(
        element,
        "cmi._version"
            | "cmi.learner_id"
            | "cmi.learner_name"
            | "cmi.entry"
            | "cmi.mode"
            | "cmi.credit"
            | "cmi.total_time"
            | "cmi.launch_data"
            | "cmi.scaled_passing_score"
            | "cmi.max_time_allowed"
            | "cmi.time_limit_action"
            | "cmi.completion_threshold"
            | "cmi.score._children"
            | "cmi.interactions._children"
            | "cmi.interactions._count"
    ) || element.starts_with("adl.nav.request_valid.")
        || parse_indexed_key(element, "cmi.interactions.")
            .is_some_and(|(_, t)| matches!(t, "correct_responses._count" | "objectives._count"))
}

fn is_known_interaction_field_2004(tail: &str) -> bool {
    if matches!(
        tail,
        "id" | "type"
            | "description"
            | "weighting"
            | "learner_response"
            | "result"
            | "timestamp"
            | "latency"
    ) {
        return true;
    }
    if let Some((_, sub)) = parse_indexed_key(tail, "correct_responses.") {
        return sub == "pattern";
    }
    if let Some((_, sub)) = parse_indexed_key(tail, "objectives.") {
        return sub == "id";
    }
    false
}

fn is_valid_nav_request(value: &str) -> bool {
    if matches!(
        value,
        "_none_"
            | "continue"
            | "previous"
            | "exit"
            | "exitAll"
            | "abandon"
            | "abandonAll"
            | "suspendAll"
    ) {
        return true;
    }
    if let Some(rest) = value.strip_prefix("{target=")
        && let Some((target, op)) = rest.split_once('}')
    {
        return !target.is_empty()
            && !target.chars().any(char::is_whitespace)
            && matches!(op, "choice" | "jump");
    }
    false
}

fn validate_interaction_2004(
    tail: &str,
    value: &str,
    kind: &str,
    edition: Scorm2004Edition,
) -> Result<(), (&'static str, &'static str, String)> {
    match tail {
        "id" => is_valid_scorm_identifier(value, 4_000, false)
            .then_some(())
            .ok_or((
                "406",
                "SCORM2004-406-IDENTIFIER",
                format!("Invalid long_identifier_type {value:?}"),
            )),
        "type" => matches!(
            value,
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
        )
        .then_some(())
        .ok_or((
            "406",
            "SCORM2004-406-INTERACTION-TYPE",
            format!("Invalid SCORM 2004 interaction type {value:?}"),
        )),
        "description" => {
            let body = strip_lang_prefix(value);
            (body.chars().count() <= 250).then_some(()).ok_or((
                "406",
                "SCORM2004-406-DESCRIPTION",
                "Interaction description exceeds 250 chars".to_owned(),
            ))
        }
        "weighting" => parse_plain_decimal(value).map(|_| ()).ok_or((
            "406",
            "SCORM2004-406-TYPE",
            format!("Invalid interaction weighting {value:?}"),
        )),
        "result" => {
            let ok = matches!(value, "correct" | "incorrect" | "unanticipated" | "neutral")
                || parse_plain_decimal(value).is_some();
            ok.then_some(()).ok_or((
                "406",
                "SCORM2004-406-RESULT",
                format!("Invalid SCORM 2004 interaction result {value:?}"),
            ))
        }
        "latency" => is_valid_iso8601_duration(value).then_some(()).ok_or((
            "406",
            "SCORM2004-406-TIMEINTERVAL",
            format!("Invalid interaction latency {value:?}"),
        )),
        "timestamp" => is_valid_iso_timestamp(value).then_some(()).ok_or((
            "406",
            "SCORM2004-406-TIMESTAMP",
            format!("Invalid interaction timestamp {value:?}"),
        )),
        "learner_response" => validate_response_2004(value, kind, false, edition),
        _ => {
            if let Some((_, "pattern")) = parse_indexed_key(tail, "correct_responses.") {
                return validate_response_2004(value, kind, true, edition);
            }
            if let Some((_, "id")) = parse_indexed_key(tail, "objectives.") {
                return is_valid_scorm_identifier(value, 4_000, false)
                    .then_some(())
                    .ok_or((
                        "406",
                        "SCORM2004-406-IDENTIFIER",
                        format!("Invalid objective id {value:?}"),
                    ));
            }
            Ok(())
        }
    }
}

fn validate_response_2004(
    value: &str,
    kind: &str,
    is_pattern: bool,
    edition: Scorm2004Edition,
) -> Result<(), (&'static str, &'static str, String)> {
    match kind {
        "true-false" => matches!(value, "true" | "false").then_some(()).ok_or((
            "406",
            "SCORM2004-406-TF",
            format!("SCORM 2004 true-false must be \"true\" or \"false\", got {value:?}"),
        )),
        "choice" => validate_short_id_list(value, true, "choice"),
        "sequencing" => {
            validate_short_id_list(value, edition == Scorm2004Edition::Fourth, "sequencing")
        }
        "likert" => {
            if value.contains("[,]") || !is_short_id(value) {
                return Err((
                    "406",
                    "SCORM2004-406-LIKERT",
                    format!("likert must be a single short_identifier_type, got {value:?}"),
                ));
            }
            Ok(())
        }
        "matching" => validate_matching(value),
        "fill-in" => validate_fill_in(value, is_pattern, edition),
        "long-fill-in" => {
            let body = strip_fill_in_prefixes(value, is_pattern);
            (body.chars().count() <= 4_000).then_some(()).ok_or((
                "406",
                "SCORM2004-406-LENGTH",
                "long-fill-in exceeds 4000 chars".to_owned(),
            ))
        }
        "numeric" => validate_numeric_response(value, is_pattern),
        "other" => (value.chars().count() <= 4_000).then_some(()).ok_or((
            "406",
            "SCORM2004-406-LENGTH",
            "other response exceeds 4000 chars".to_owned(),
        )),
        _ => Ok(()),
    }
}

fn is_short_id(s: &str) -> bool {
    is_valid_scorm_identifier(s, 250, false) && !s.contains("[,]") && !s.contains("[.]")
}

fn validate_short_id_list(
    value: &str,
    require_unique: bool,
    kind: &str,
) -> Result<(), (&'static str, &'static str, String)> {
    if value.is_empty() {
        return Ok(());
    }
    let mut seen = HashSet::new();
    for item in value.split("[,]") {
        if !is_short_id(item) {
            return Err((
                "406",
                "SCORM2004-406-SHORT-ID",
                format!("Invalid short_identifier_type {item:?} in {kind}"),
            ));
        }
        if require_unique && !seen.insert(item) {
            return Err((
                "406",
                "SCORM2004-406-DUPLICATE",
                format!("Duplicate item {item:?} in {kind}"),
            ));
        }
    }
    Ok(())
}

fn validate_matching(value: &str) -> Result<(), (&'static str, &'static str, String)> {
    if value.is_empty() {
        return Ok(());
    }
    for pair in value.split("[,]") {
        let Some((src, dst)) = pair.split_once("[.]") else {
            return Err((
                "406",
                "SCORM2004-406-MATCHING",
                format!("Matching pair {pair:?} missing [.] separator"),
            ));
        };
        if !is_short_id(src) || !is_short_id(dst) {
            return Err((
                "406",
                "SCORM2004-406-MATCHING",
                format!("Invalid short_identifier_type in matching pair {pair:?}"),
            ));
        }
    }
    Ok(())
}

fn validate_fill_in(
    value: &str,
    is_pattern: bool,
    edition: Scorm2004Edition,
) -> Result<(), (&'static str, &'static str, String)> {
    let body = strip_fill_in_prefixes(value, is_pattern);
    let items: Vec<&str> = body.split("[,]").collect();
    if edition == Scorm2004Edition::Fourth && items.len() > 10 {
        return Err((
            "406",
            "SCORM2004-406-FILLIN-COUNT",
            format!("fill-in exceeds 10 items ({})", items.len()),
        ));
    }
    for item in items {
        let text = strip_lang_prefix(item);
        if text.chars().count() > 250 {
            return Err((
                "406",
                "SCORM2004-406-FILLIN-LEN",
                "fill-in item exceeds 250 chars".to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_numeric_response(
    value: &str,
    is_pattern: bool,
) -> Result<(), (&'static str, &'static str, String)> {
    if !is_pattern {
        return parse_plain_decimal(value).map(|_| ()).ok_or((
            "406",
            "SCORM2004-406-NUMERIC",
            format!("Invalid numeric learner_response {value:?}"),
        ));
    }
    if let Some((min_s, max_s)) = value.split_once("[:]") {
        let min_ok = min_s.is_empty() || parse_plain_decimal(min_s).is_some();
        let max_ok = max_s.is_empty() || parse_plain_decimal(max_s).is_some();
        if (min_s.is_empty() && max_s.is_empty()) || !min_ok || !max_ok {
            return Err((
                "406",
                "SCORM2004-406-NUMERIC",
                format!("Invalid numeric pattern {value:?}"),
            ));
        }
        return Ok(());
    }
    parse_plain_decimal(value).map(|_| ()).ok_or((
        "406",
        "SCORM2004-406-NUMERIC",
        format!("Invalid numeric pattern {value:?}"),
    ))
}

fn strip_lang_prefix(s: &str) -> &str {
    if let Some(rest) = s.strip_prefix("{lang=")
        && let Some((_, after)) = rest.split_once('}')
    {
        return after;
    }
    s
}

fn strip_fill_in_prefixes(mut s: &str, is_pattern: bool) -> &str {
    if is_pattern {
        for prefix in [
            "{case_matters=true}",
            "{case_matters=false}",
            "{order_matters=true}",
            "{order_matters=false}",
        ] {
            if let Some(rest) = s.strip_prefix(prefix) {
                s = rest;
            }
        }
        for prefix in ["{case_matters=true}", "{case_matters=false}"] {
            if let Some(rest) = s.strip_prefix(prefix) {
                s = rest;
            }
        }
    }
    strip_lang_prefix(s)
}

fn is_valid_iso_timestamp(s: &str) -> bool {
    s.len() >= 10 && s.as_bytes()[0..4].iter().all(u8::is_ascii_digit)
}

fn apply_interaction_field_2004(inter: &mut Scorm2004Interaction, tail: &str, value: &str) {
    match tail {
        "id" => inter.id = value.to_owned(),
        "type" => inter.kind = Some(value.to_owned()),
        "description" => inter.description = Some(value.to_owned()),
        "weighting" => inter.weighting = Some(value.to_owned()),
        "learner_response" => inter.learner_response = Some(value.to_owned()),
        "result" => inter.result = Some(value.to_owned()),
        "timestamp" => inter.timestamp = Some(value.to_owned()),
        "latency" => inter.latency = Some(value.to_owned()),
        _ => {
            if let Some((m, "pattern")) = parse_indexed_key(tail, "correct_responses.") {
                if m == inter.correct_responses.len() {
                    inter.correct_responses.push(value.to_owned());
                } else if let Some(slot) = inter.correct_responses.get_mut(m) {
                    *slot = value.to_owned();
                }
            } else if let Some((m, "id")) = parse_indexed_key(tail, "objectives.") {
                if m == inter.objectives.len() {
                    inter.objectives.push(value.to_owned());
                } else if let Some(slot) = inter.objectives.get_mut(m) {
                    *slot = value.to_owned();
                }
            }
        }
    }
}

fn parse_iso_duration_cs(s: &str) -> u64 {
    let Some(rest) = s.strip_prefix('P') else {
        return 0;
    };
    let (_, time_part) = rest.split_once('T').unwrap_or((rest, ""));
    let mut total_cs = 0u64;
    let mut num = String::new();
    for ch in time_part.chars() {
        if ch.is_ascii_digit() || ch == '.' {
            num.push(ch);
        } else {
            let val: f64 = num.parse().unwrap_or(0.0);
            num.clear();
            let mult = match ch {
                'H' => 360_000.0,
                'M' => 6_000.0,
                'S' => 100.0,
                _ => 0.0,
            };
            let add = (val * mult).round();
            if add.is_finite() && add >= 0.0 {
                total_cs = total_cs.saturating_add(add as u64);
            }
        }
    }
    total_cs
}

fn format_iso_duration_cs(total_cs: u64) -> String {
    let cs = total_cs % 100;
    let total_secs = total_cs / 100;
    let sec = total_secs % 60;
    let total_mins = total_secs / 60;
    let min = total_mins % 60;
    let hours = total_mins / 60;
    if cs == 0 {
        format!("PT{hours}H{min}M{sec}S")
    } else {
        format!("PT{hours}H{min}M{sec}.{cs:02}S")
    }
}

fn error_string_2004(code: &str) -> &'static str {
    match code {
        "0" => "No Error",
        "101" => "General Exception",
        "102" => "General Initialization Failure",
        "103" => "Already Initialized",
        "104" => "Content Instance Terminated",
        "111" => "General Termination Failure",
        "112" => "Termination Before Initialization",
        "113" => "Termination After Termination",
        "122" => "Retrieve Data Before Initialization",
        "123" => "Retrieve Data After Termination",
        "132" => "Store Data Before Initialization",
        "133" => "Store Data After Termination",
        "142" => "Commit Before Initialization",
        "143" => "Commit After Termination",
        "201" => "General Argument Error",
        "301" => "General Get Failure",
        "351" => "General Set Failure",
        "391" => "General Commit Failure",
        "401" => "Undefined Data Model Element",
        "402" => "Unimplemented Data Model Element",
        "403" => "Data Model Element Value Not Initialized",
        "404" => "Data Model Element Is Read Only",
        "405" => "Data Model Element Is Write Only",
        "406" => "Data Model Element Type Mismatch",
        "407" => "Data Model Element Value Out Of Range",
        "408" => "Data Model Dependency Not Established",
        _ => "",
    }
}
