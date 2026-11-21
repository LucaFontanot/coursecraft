//! Strict SCORM 1.2 Run-Time Environment engine (ADL SCORM 1.2 RTE §3).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    CallRecord, Violation, format_cmi_timespan_cs, is_valid_cmi_time, is_valid_scorm_identifier,
    parse_cmi_timespan_cs, parse_indexed_key, parse_plain_decimal,
};

/// Configuration for a [`StrictScorm12`] LMS instance.
#[derive(Debug, Clone)]
pub struct Scorm12Config {
    pub student_id: String,
    pub student_name: String,
    pub credit: String,
    pub lesson_mode: String,
    pub launch_data: String,
    pub comments_from_lms: String,
    pub mastery_score: Option<f64>,
    pub max_time_allowed: String,
    pub time_limit_action: String,
    /// Clear attempt data on relaunch when `cmi.core.exit != "suspend"`.
    pub clear_on_normal_exit: bool,
    /// When `true`, multi-character choice IDs in `cmi.interactions.n` are errors (`405`);
    /// when `false`, they are recorded as advisory warnings only.
    pub strict_12_single_char_ids: bool,
    /// Initial `cmi.core.lesson_status` (defaults to `"not attempted"`).
    pub initial_lesson_status: String,
    /// Optional error code to return when `LMSInitialize("")` is called (for failure tests).
    pub fail_initialize: Option<&'static str>,
}

impl Default for Scorm12Config {
    fn default() -> Self {
        Self {
            student_id: "learner-1".to_owned(),
            student_name: "Learner, Test".to_owned(),
            credit: "credit".to_owned(),
            lesson_mode: "normal".to_owned(),
            launch_data: String::new(),
            comments_from_lms: String::new(),
            mastery_score: None,
            max_time_allowed: String::new(),
            time_limit_action: String::new(),
            clear_on_normal_exit: true,
            strict_12_single_char_ids: false,
            initial_lesson_status: "not attempted".to_owned(),
            fail_initialize: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LifecycleState {
    NotInitialized,
    Running,
    Finished,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Scorm12Score {
    raw: String,
    min: String,
    max: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Scorm12Interaction {
    id: String,
    kind: String,
    time: String,
    weighting: String,
    student_response: String,
    result: String,
    latency: String,
    objectives: Vec<String>,
    correct_responses: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Scorm12Objective {
    id: String,
    score: Scorm12Score,
    status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedScorm12 {
    entry: String,
    lesson_status: String,
    lesson_location: String,
    suspend_data: String,
    exit: String,
    total_time_cs: u64,
    score: Scorm12Score,
    comments: String,
    student_preference: BTreeMap<String, String>,
    interactions: Vec<Scorm12Interaction>,
    objectives: Vec<Scorm12Objective>,
}

/// Strict SCORM 1.2 RTE conformance engine.
#[derive(Debug, Clone)]
pub struct StrictScorm12 {
    config: Scorm12Config,
    state: LifecycleState,
    last_error: &'static str,
    entry: String,
    lesson_status: String,
    lesson_location: String,
    suspend_data: String,
    exit: String,
    session_time_cs: Option<u64>,
    total_time_cs: u64,
    score: Scorm12Score,
    comments: String,
    student_preference: BTreeMap<String, String>,
    interactions: Vec<Scorm12Interaction>,
    objectives: Vec<Scorm12Objective>,
    calls: Vec<CallRecord>,
    violations: Vec<Violation>,
    warnings: Vec<Violation>,
}

impl StrictScorm12 {
    /// Create a new SCORM 1.2 strict LMS engine with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(Scorm12Config::default())
    }

    /// Create a new SCORM 1.2 strict LMS engine with custom configuration.
    #[must_use]
    pub fn with_config(config: Scorm12Config) -> Self {
        let lesson_status = config.initial_lesson_status.clone();
        Self {
            config,
            state: LifecycleState::NotInitialized,
            last_error: "0",
            entry: "ab-initio".to_owned(),
            lesson_status,
            lesson_location: String::new(),
            suspend_data: String::new(),
            exit: String::new(),
            session_time_cs: None,
            total_time_cs: 0,
            score: Scorm12Score::default(),
            comments: String::new(),
            student_preference: BTreeMap::new(),
            interactions: Vec::new(),
            objectives: Vec::new(),
            calls: Vec::new(),
            violations: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Seed initial `cmi.suspend_data` (for resume/corruption tests).
    pub fn seed_suspend_data(&mut self, data: impl Into<String>) {
        self.suspend_data = data.into();
    }

    /// Seed initial `cmi.core.lesson_location`.
    pub fn seed_lesson_location(&mut self, loc: impl Into<String>) {
        self.lesson_location = loc.into();
    }

    /// Seed initial `cmi.core.lesson_status` and optional `cmi.core.score.raw`.
    pub fn seed_status(&mut self, status: impl Into<String>, score_raw: Option<&str>) {
        self.lesson_status = status.into();
        if let Some(raw) = score_raw {
            self.score.raw = raw.to_owned();
        }
    }

    /// All MUST/SHALL specification violations recorded so far.
    #[must_use]
    pub fn violations(&self) -> &[Violation] {
        &self.violations
    }

    /// Advisory warnings (SHOULD-level or configurable checks).
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
            "standard": "scorm12",
            "state": format!("{:?}", self.state),
            "last_error": self.last_error,
            "entry": self.entry,
            "lesson_status": self.lesson_status,
            "lesson_location": self.lesson_location,
            "suspend_data": self.suspend_data,
            "exit": self.exit,
            "total_time": format_cmi_timespan_cs(self.total_time_cs),
            "score": self.score,
            "interactions": self.interactions,
            "objectives": self.objectives,
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
                self.lesson_status = self.config.initial_lesson_status.clone();
                self.lesson_location.clear();
                self.suspend_data.clear();
                self.score = Scorm12Score::default();
                self.interactions.clear();
                self.objectives.clear();
                self.comments.clear();
            }
        }
        self.state = LifecycleState::NotInitialized;
        self.last_error = "0";
        self.exit.clear();
        self.session_time_cs = None;
    }

    /// Export persisted learner attempt state as JSON (for `localStorage` in `mock-lms`).
    #[must_use]
    pub fn export_persisted_json(&self) -> String {
        let persisted = PersistedScorm12 {
            entry: self.entry.clone(),
            lesson_status: self.lesson_status.clone(),
            lesson_location: self.lesson_location.clone(),
            suspend_data: self.suspend_data.clone(),
            exit: self.exit.clone(),
            total_time_cs: self.total_time_cs,
            score: self.score.clone(),
            comments: self.comments.clone(),
            student_preference: self.student_preference.clone(),
            interactions: self.interactions.clone(),
            objectives: self.objectives.clone(),
        };
        serde_json::to_string(&persisted).unwrap_or_else(|_| "{}".to_owned())
    }

    /// Restore persisted learner attempt state from JSON (resets lifecycle to `NotInitialized`).
    pub fn import_persisted_json(&mut self, json: &str) -> Result<(), String> {
        let p: PersistedScorm12 =
            serde_json::from_str(json).map_err(|e| format!("invalid persisted state: {e}"))?;
        self.entry = p.entry;
        self.lesson_status = p.lesson_status;
        self.lesson_location = p.lesson_location;
        self.suspend_data = p.suspend_data;
        self.exit = p.exit;
        self.total_time_cs = p.total_time_cs;
        self.score = p.score;
        self.comments = p.comments;
        self.student_preference = p.student_preference;
        self.interactions = p.interactions;
        self.objectives = p.objectives;
        self.state = LifecycleState::NotInitialized;
        self.last_error = "0";
        self.session_time_cs = None;
        Ok(())
    }

    /// Flag a violation if the SCO is still `Running` without calling `LMSFinish("")`.
    pub fn assert_clean_finish(&mut self) -> bool {
        if self.state == LifecycleState::Running {
            let idx = self.calls.len() + 1;
            self.violations.push(Violation {
                standard: "scorm12",
                rule: "SCORM12-UNCLEAN-FINISH",
                call_index: idx,
                element: "LMSFinish".to_owned(),
                message: "Session ended while Running without calling LMSFinish(\"\")".to_owned(),
                error_code: None,
            });
            false
        } else {
            true
        }
    }

    /// `LMSInitialize(arg)`
    pub fn lms_initialize(&mut self, arg: &str) -> &'static str {
        let idx = self.next_call_index();
        let (res, err) = if !arg.is_empty() {
            self.record_violation(
                idx,
                "SCORM12-201-ARG",
                "LMSInitialize",
                format!("LMSInitialize argument must be empty string, got {arg:?}"),
                "201",
            );
            ("false", "201")
        } else if self.state != LifecycleState::NotInitialized {
            self.record_violation(
                idx,
                "SCORM12-101-ALREADY-INIT",
                "LMSInitialize",
                format!("LMSInitialize called in state {:?}", self.state),
                "101",
            );
            ("false", "101")
        } else if let Some(code) = self.config.fail_initialize {
            ("false", code)
        } else {
            self.state = LifecycleState::Running;
            ("true", "0")
        };
        self.finish_call(idx, "LMSInitialize", vec![arg.to_owned()], res, err);
        res
    }

    /// `LMSGetValue(element)`
    pub fn lms_get_value(&mut self, element: &str) -> String {
        let idx = self.next_call_index();
        if self.state != LifecycleState::Running {
            self.record_violation(
                idx,
                "SCORM12-301-NOT-INIT",
                element,
                format!("LMSGetValue({element:?}) called when not initialized"),
                "301",
            );
            self.finish_call(idx, "LMSGetValue", vec![element.to_owned()], "", "301");
            return String::new();
        }
        let (val, err) = self.get_element(idx, element);
        self.finish_call(idx, "LMSGetValue", vec![element.to_owned()], &val, err);
        val
    }

    /// `LMSSetValue(element, value)`
    pub fn lms_set_value(&mut self, element: &str, value: &str) -> &'static str {
        let idx = self.next_call_index();
        if self.state != LifecycleState::Running {
            self.record_violation(
                idx,
                "SCORM12-301-NOT-INIT",
                element,
                format!("LMSSetValue({element:?}) called when not initialized"),
                "301",
            );
            self.finish_call(
                idx,
                "LMSSetValue",
                vec![element.to_owned(), value.to_owned()],
                "false",
                "301",
            );
            return "false";
        }
        let err = self.set_element(idx, element, value);
        let res = if err == "0" { "true" } else { "false" };
        self.finish_call(
            idx,
            "LMSSetValue",
            vec![element.to_owned(), value.to_owned()],
            res,
            err,
        );
        res
    }

    /// `LMSCommit(arg)`
    pub fn lms_commit(&mut self, arg: &str) -> &'static str {
        let idx = self.next_call_index();
        let (res, err) = if self.state != LifecycleState::Running {
            self.record_violation(
                idx,
                "SCORM12-301-NOT-INIT",
                "LMSCommit",
                "LMSCommit called when not initialized".to_owned(),
                "301",
            );
            ("false", "301")
        } else if !arg.is_empty() {
            self.record_violation(
                idx,
                "SCORM12-201-ARG",
                "LMSCommit",
                format!("LMSCommit argument must be empty string, got {arg:?}"),
                "201",
            );
            ("false", "201")
        } else {
            ("true", "0")
        };
        self.finish_call(idx, "LMSCommit", vec![arg.to_owned()], res, err);
        res
    }

    /// `LMSFinish(arg)`
    pub fn lms_finish(&mut self, arg: &str) -> &'static str {
        let idx = self.next_call_index();
        let (res, err) = if self.state != LifecycleState::Running {
            self.record_violation(
                idx,
                "SCORM12-301-NOT-INIT",
                "LMSFinish",
                "LMSFinish called when not initialized".to_owned(),
                "301",
            );
            ("false", "301")
        } else if !arg.is_empty() {
            self.record_violation(
                idx,
                "SCORM12-201-ARG",
                "LMSFinish",
                format!("LMSFinish argument must be empty string, got {arg:?}"),
                "201",
            );
            ("false", "201")
        } else {
            if let Some(session_cs) = self.session_time_cs {
                self.total_time_cs = self.total_time_cs.saturating_add(session_cs);
            }
            if let Some(mastery) = self.config.mastery_score
                && self.lesson_status == "completed"
                && let Some(raw) = parse_plain_decimal(&self.score.raw)
            {
                self.lesson_status = if raw >= mastery {
                    "passed".to_owned()
                } else {
                    "failed".to_owned()
                };
            }
            self.state = LifecycleState::Finished;
            ("true", "0")
        };
        self.finish_call(idx, "LMSFinish", vec![arg.to_owned()], res, err);
        res
    }

    /// `LMSGetLastError()`
    #[must_use]
    pub fn lms_get_last_error(&self) -> &'static str {
        self.last_error
    }

    /// `LMSGetErrorString(code)`
    #[must_use]
    pub fn lms_get_error_string(&self, code: &str) -> &'static str {
        error_string(code)
    }

    /// `LMSGetDiagnostic(code)`
    #[must_use]
    pub fn lms_get_diagnostic(&self, code: &str) -> String {
        let effective = if code.is_empty() {
            self.last_error
        } else {
            code
        };
        format!("{effective}: {}", error_string(effective))
    }

    /// Unified string-based dispatch entrypoint (used by WASM parity tests and `mock-lms`).
    pub fn dispatch(&mut self, method: &str, args: &[&str]) -> String {
        let a0 = args.first().copied().unwrap_or("");
        let a1 = args.get(1).copied().unwrap_or("");
        match method {
            "LMSInitialize" => self.lms_initialize(a0).to_owned(),
            "LMSGetValue" => self.lms_get_value(a0),
            "LMSSetValue" => self.lms_set_value(a0, a1).to_owned(),
            "LMSCommit" => self.lms_commit(a0).to_owned(),
            "LMSFinish" => self.lms_finish(a0).to_owned(),
            "LMSGetLastError" => self.lms_get_last_error().to_owned(),
            "LMSGetErrorString" => self.lms_get_error_string(a0).to_owned(),
            "LMSGetDiagnostic" => self.lms_get_diagnostic(a0),
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
            standard: "scorm12",
            rule,
            call_index,
            element: element.to_owned(),
            message,
            error_code: Some(error_code.to_owned()),
        });
    }

    fn record_warning(
        &mut self,
        call_index: usize,
        rule: &'static str,
        element: &str,
        msg: String,
    ) {
        self.warnings.push(Violation {
            standard: "scorm12",
            rule,
            call_index,
            element: element.to_owned(),
            message: msg,
            error_code: None,
        });
    }

    fn get_element(&mut self, idx: usize, element: &str) -> (String, &'static str) {
        if let Some(val) = self.get_keyword(element) {
            return (val, "0");
        }
        match element {
            "cmi.core.student_id" => (self.config.student_id.clone(), "0"),
            "cmi.core.student_name" => (self.config.student_name.clone(), "0"),
            "cmi.core.credit" => (self.config.credit.clone(), "0"),
            "cmi.core.entry" => (self.entry.clone(), "0"),
            "cmi.core.lesson_mode" => (self.config.lesson_mode.clone(), "0"),
            "cmi.core.total_time" => (format_cmi_timespan_cs(self.total_time_cs), "0"),
            "cmi.launch_data" => (self.config.launch_data.clone(), "0"),
            "cmi.comments_from_lms" => (self.config.comments_from_lms.clone(), "0"),
            "cmi.student_data.mastery_score" => (
                self.config
                    .mastery_score
                    .map_or_else(String::new, |v| v.to_string()),
                "0",
            ),
            "cmi.student_data.max_time_allowed" => (self.config.max_time_allowed.clone(), "0"),
            "cmi.student_data.time_limit_action" => (self.config.time_limit_action.clone(), "0"),
            "cmi.core.lesson_status" => (self.lesson_status.clone(), "0"),
            "cmi.core.lesson_location" => (self.lesson_location.clone(), "0"),
            "cmi.core.score.raw" => (self.score.raw.clone(), "0"),
            "cmi.core.score.min" => (self.score.min.clone(), "0"),
            "cmi.core.score.max" => (self.score.max.clone(), "0"),
            "cmi.suspend_data" => (self.suspend_data.clone(), "0"),
            "cmi.comments" => (self.comments.clone(), "0"),
            "cmi.student_preference.audio"
            | "cmi.student_preference.language"
            | "cmi.student_preference.speed"
            | "cmi.student_preference.text" => (
                self.student_preference
                    .get(element)
                    .cloned()
                    .unwrap_or_default(),
                "0",
            ),
            "cmi.core.exit" | "cmi.core.session_time" => {
                self.record_violation(
                    idx,
                    "SCORM12-404-WRITE-ONLY",
                    element,
                    format!("{element} is write-only in SCORM 1.2"),
                    "404",
                );
                (String::new(), "404")
            }
            _ => self.get_indexed_element(idx, element),
        }
    }

    fn get_keyword(&self, element: &str) -> Option<String> {
        let val = match element {
            "cmi.core._children" => {
                "student_id,student_name,lesson_location,credit,lesson_status,entry,score,total_time,lesson_mode,exit,session_time"
                    .to_owned()
            }
            "cmi.core.score._children" => "raw,min,max".to_owned(),
            "cmi.interactions._children" => {
                "id,objectives,time,type,correct_responses,weighting,student_response,result,latency"
                    .to_owned()
            }
            "cmi.interactions._count" => self.interactions.len().to_string(),
            "cmi.objectives._children" => "id,score,status".to_owned(),
            "cmi.objectives._count" => self.objectives.len().to_string(),
            "cmi.student_data._children" => {
                "mastery_score,max_time_allowed,time_limit_action".to_owned()
            }
            "cmi.student_preference._children" => "audio,language,speed,text".to_owned(),
            _ => {
                if let Some((n, tail)) = parse_indexed_key(element, "cmi.interactions.") {
                    if tail == "objectives._count" {
                        return Some(
                            self.interactions
                                .get(n)
                                .map_or(0, |i| i.objectives.len())
                                .to_string(),
                        );
                    }
                    if tail == "correct_responses._count" {
                        return Some(
                            self.interactions
                                .get(n)
                                .map_or(0, |i| i.correct_responses.len())
                                .to_string(),
                        );
                    }
                }
                return None;
            }
        };
        Some(val)
    }

    fn get_indexed_element(&mut self, idx: usize, element: &str) -> (String, &'static str) {
        if let Some((_n, tail)) = parse_indexed_key(element, "cmi.interactions.")
            && is_known_interaction_field(tail)
        {
            self.record_violation(
                idx,
                "SCORM12-404-WRITE-ONLY",
                element,
                "cmi.interactions.n.* elements are write-only in SCORM 1.2".to_owned(),
                "404",
            );
            return (String::new(), "404");
        }
        if let Some((n, tail)) = parse_indexed_key(element, "cmi.objectives.") {
            return self.get_objective_field(idx, element, n, tail);
        }
        self.record_violation(
            idx,
            "SCORM12-401-UNKNOWN",
            element,
            format!("Unknown SCORM 1.2 data model element {element:?}"),
            "401",
        );
        (String::new(), "401")
    }

    fn get_objective_field(
        &mut self,
        idx: usize,
        element: &str,
        n: usize,
        tail: &str,
    ) -> (String, &'static str) {
        let Some(obj) = self.objectives.get(n) else {
            return match tail {
                "id" | "score.raw" | "score.min" | "score.max" | "status" => (String::new(), "0"),
                _ => {
                    self.record_violation(
                        idx,
                        "SCORM12-401-UNKNOWN",
                        element,
                        format!("Unknown objective field {element:?}"),
                        "401",
                    );
                    (String::new(), "401")
                }
            };
        };
        match tail {
            "id" => (obj.id.clone(), "0"),
            "score.raw" => (obj.score.raw.clone(), "0"),
            "score.min" => (obj.score.min.clone(), "0"),
            "score.max" => (obj.score.max.clone(), "0"),
            "status" => (obj.status.clone(), "0"),
            _ => {
                self.record_violation(
                    idx,
                    "SCORM12-401-UNKNOWN",
                    element,
                    format!("Unknown objective field {element:?}"),
                    "401",
                );
                (String::new(), "401")
            }
        }
    }

    fn set_element(&mut self, idx: usize, element: &str, value: &str) -> &'static str {
        if is_keyword_element(element) {
            self.record_violation(
                idx,
                "SCORM12-402-KEYWORD",
                element,
                format!("Cannot set keyword element {element}"),
                "402",
            );
            return "402";
        }
        if is_read_only_element(element) {
            self.record_violation(
                idx,
                "SCORM12-403-READ-ONLY",
                element,
                format!("Cannot set read-only element {element}"),
                "403",
            );
            return "403";
        }
        match element {
            "cmi.core.lesson_status" => self.set_lesson_status(idx, element, value),
            "cmi.core.lesson_location" => self.set_bounded_str(idx, element, value, 255, |s, v| {
                s.lesson_location = v.to_owned();
            }),
            "cmi.core.exit" => self.set_exit(idx, element, value),
            "cmi.core.score.raw" => self.set_score_field(idx, element, value, |s, v| {
                s.score.raw = v.to_owned();
            }),
            "cmi.core.score.min" => self.set_score_field(idx, element, value, |s, v| {
                s.score.min = v.to_owned();
            }),
            "cmi.core.score.max" => self.set_score_field(idx, element, value, |s, v| {
                s.score.max = v.to_owned();
            }),
            "cmi.core.session_time" => self.set_session_time(idx, element, value),
            "cmi.suspend_data" => self.set_bounded_str(idx, element, value, 4096, |s, v| {
                s.suspend_data = v.to_owned();
            }),
            "cmi.comments" => self.set_bounded_str(idx, element, value, 4096, |s, v| {
                s.comments = v.to_owned();
            }),
            "cmi.student_preference.audio"
            | "cmi.student_preference.language"
            | "cmi.student_preference.speed"
            | "cmi.student_preference.text" => {
                self.student_preference
                    .insert(element.to_owned(), value.to_owned());
                "0"
            }
            _ => self.set_indexed_element(idx, element, value),
        }
    }

    fn set_lesson_status(&mut self, idx: usize, element: &str, value: &str) -> &'static str {
        if matches!(
            value,
            "passed" | "completed" | "failed" | "incomplete" | "browsed"
        ) {
            self.lesson_status = value.to_owned();
            "0"
        } else {
            self.record_violation(
                idx,
                "SCORM12-405-VOCAB",
                element,
                format!("Invalid SCORM 1.2 lesson_status {value:?}"),
                "405",
            );
            "405"
        }
    }

    fn set_exit(&mut self, idx: usize, element: &str, value: &str) -> &'static str {
        if matches!(value, "" | "time-out" | "suspend" | "logout") {
            self.exit = value.to_owned();
            "0"
        } else {
            self.record_violation(
                idx,
                "SCORM12-405-VOCAB",
                element,
                format!("Invalid SCORM 1.2 exit {value:?}"),
                "405",
            );
            "405"
        }
    }

    fn set_bounded_str(
        &mut self,
        idx: usize,
        element: &str,
        value: &str,
        max_chars: usize,
        assign: fn(&mut Self, &str),
    ) -> &'static str {
        let len = value.chars().count();
        if len > max_chars {
            self.record_violation(
                idx,
                "SCORM12-405-LENGTH",
                element,
                format!("{element} length {len} exceeds {max_chars} chars"),
                "405",
            );
            "405"
        } else {
            assign(self, value);
            "0"
        }
    }

    fn set_score_field(
        &mut self,
        idx: usize,
        element: &str,
        value: &str,
        assign: fn(&mut Self, &str),
    ) -> &'static str {
        if value.is_empty() {
            assign(self, value);
            return "0";
        }
        let Some(num) = parse_plain_decimal(value) else {
            self.record_violation(
                idx,
                "SCORM12-405-DECIMAL",
                element,
                format!("Invalid CMIDecimal {value:?} for {element}"),
                "405",
            );
            return "405";
        };
        if !(0.0..=100.0).contains(&num) {
            self.record_violation(
                idx,
                "SCORM12-405-RANGE",
                element,
                format!("Score {num} out of SCORM 1.2 range 0..=100"),
                "405",
            );
            return "405";
        }
        assign(self, value);
        "0"
    }

    fn set_session_time(&mut self, idx: usize, element: &str, value: &str) -> &'static str {
        let Some(cs) = parse_cmi_timespan_cs(value) else {
            self.record_violation(
                idx,
                "SCORM12-405-TIMESPAN",
                element,
                format!("Invalid CMITimespan {value:?} (expected HHHH:MM:SS.SS with MM,SS <= 59)"),
                "405",
            );
            return "405";
        };
        self.session_time_cs = Some(cs);
        "0"
    }

    fn set_indexed_element(&mut self, idx: usize, element: &str, value: &str) -> &'static str {
        if let Some((n, tail)) = parse_indexed_key(element, "cmi.interactions.") {
            return self.set_interaction_field(idx, element, n, tail, value);
        }
        if let Some((n, tail)) = parse_indexed_key(element, "cmi.objectives.") {
            return self.set_objective_field(idx, element, n, tail, value);
        }
        self.record_violation(
            idx,
            "SCORM12-401-UNKNOWN",
            element,
            format!("Unknown SCORM 1.2 data model element {element:?}"),
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
        if !is_known_interaction_field(tail) {
            self.record_violation(
                idx,
                "SCORM12-401-UNKNOWN",
                element,
                format!("Unknown interaction element {element:?}"),
                "401",
            );
            return "401";
        }
        if n > self.interactions.len() {
            self.record_violation(
                idx,
                "SCORM12-201-INTERACTION-INDEX",
                element,
                format!(
                    "Interaction index {n} skips gap (current _count = {})",
                    self.interactions.len()
                ),
                "201",
            );
            return "201";
        }
        let existing_kind = self
            .interactions
            .get(n)
            .map(|i| i.kind.as_str())
            .unwrap_or("");
        if let Err((rule, msg)) = validate_interaction_value(
            tail,
            value,
            existing_kind,
            self.config.strict_12_single_char_ids,
        ) {
            self.record_violation(idx, rule, element, msg, "405");
            return "405";
        }
        if tail == "student_response"
            && existing_kind == "choice"
            && !self.config.strict_12_single_char_ids
            && value.split(',').any(|p| p.chars().count() > 1)
        {
            self.record_warning(
                idx,
                "SCORM12-CHOICE-MULTI-CHAR",
                element,
                format!("SCORM 1.2 choice response {value:?} uses multi-char IDs"),
            );
        }
        if let Some((m, pat_tail)) = parse_indexed_key(tail, "correct_responses.") {
            if pat_tail != "pattern" {
                self.record_violation(
                    idx,
                    "SCORM12-401-UNKNOWN",
                    element,
                    format!("Unknown correct_responses field {element:?}"),
                    "401",
                );
                return "401";
            }
            let cur_len = self
                .interactions
                .get(n)
                .map_or(0, |i| i.correct_responses.len());
            if m > cur_len {
                self.record_violation(
                    idx,
                    "SCORM12-201-INTERACTION-INDEX",
                    element,
                    format!("correct_responses index {m} skips gap (current _count = {cur_len})"),
                    "201",
                );
                return "201";
            }
        }
        if n == self.interactions.len() {
            self.interactions.push(Scorm12Interaction::default());
        }
        if let Some(inter) = self.interactions.get_mut(n) {
            apply_interaction_field(inter, tail, value);
        }
        "0"
    }

    fn set_objective_field(
        &mut self,
        idx: usize,
        element: &str,
        n: usize,
        tail: &str,
        value: &str,
    ) -> &'static str {
        if !matches!(
            tail,
            "id" | "score.raw" | "score.min" | "score.max" | "status"
        ) {
            self.record_violation(
                idx,
                "SCORM12-401-UNKNOWN",
                element,
                format!("Unknown objective element {element:?}"),
                "401",
            );
            return "401";
        }
        if n > self.objectives.len() {
            self.record_violation(
                idx,
                "SCORM12-201-OBJECTIVE-INDEX",
                element,
                format!("Objective index {n} skips gap"),
                "201",
            );
            return "201";
        }
        if tail == "id" && !is_valid_scorm_identifier(value, 255, true) {
            self.record_violation(
                idx,
                "SCORM12-405-IDENTIFIER",
                element,
                format!("Invalid objective identifier {value:?}"),
                "405",
            );
            return "405";
        }
        if matches!(tail, "score.raw" | "score.min" | "score.max")
            && !value.is_empty()
            && parse_plain_decimal(value).is_none_or(|v| !(0.0..=100.0).contains(&v))
        {
            self.record_violation(
                idx,
                "SCORM12-405-DECIMAL",
                element,
                format!("Invalid objective score {value:?}"),
                "405",
            );
            return "405";
        }
        if tail == "status"
            && !matches!(
                value,
                "passed" | "completed" | "failed" | "incomplete" | "browsed" | "not attempted"
            )
        {
            self.record_violation(
                idx,
                "SCORM12-405-VOCAB",
                element,
                format!("Invalid objective status {value:?}"),
                "405",
            );
            return "405";
        }
        if n == self.objectives.len() {
            self.objectives.push(Scorm12Objective::default());
        }
        if let Some(obj) = self.objectives.get_mut(n) {
            match tail {
                "id" => obj.id = value.to_owned(),
                "score.raw" => obj.score.raw = value.to_owned(),
                "score.min" => obj.score.min = value.to_owned(),
                "score.max" => obj.score.max = value.to_owned(),
                "status" => obj.status = value.to_owned(),
                _ => {}
            }
        }
        "0"
    }
}

impl Default for StrictScorm12 {
    fn default() -> Self {
        Self::new()
    }
}

fn is_keyword_element(element: &str) -> bool {
    if matches!(
        element,
        "cmi.core._children"
            | "cmi.core.score._children"
            | "cmi.interactions._children"
            | "cmi.interactions._count"
            | "cmi.objectives._children"
            | "cmi.objectives._count"
            | "cmi.student_data._children"
            | "cmi.student_preference._children"
    ) {
        return true;
    }
    if let Some((_, tail)) = parse_indexed_key(element, "cmi.interactions.") {
        return matches!(tail, "objectives._count" | "correct_responses._count");
    }
    false
}

fn is_read_only_element(element: &str) -> bool {
    matches!(
        element,
        "cmi.core.student_id"
            | "cmi.core.student_name"
            | "cmi.core.credit"
            | "cmi.core.entry"
            | "cmi.core.total_time"
            | "cmi.core.lesson_mode"
            | "cmi.launch_data"
            | "cmi.comments_from_lms"
            | "cmi.student_data.mastery_score"
            | "cmi.student_data.max_time_allowed"
            | "cmi.student_data.time_limit_action"
    )
}

fn is_known_interaction_field(tail: &str) -> bool {
    if matches!(
        tail,
        "id" | "time" | "type" | "weighting" | "student_response" | "result" | "latency"
    ) {
        return true;
    }
    if let Some((_, sub)) = parse_indexed_key(tail, "objectives.") {
        return sub == "id";
    }
    if let Some((_, sub)) = parse_indexed_key(tail, "correct_responses.") {
        return sub == "pattern";
    }
    false
}

fn validate_interaction_value(
    tail: &str,
    value: &str,
    kind: &str,
    strict_single_char_ids: bool,
) -> Result<(), (&'static str, String)> {
    match tail {
        "id" => is_valid_scorm_identifier(value, 255, true)
            .then_some(())
            .ok_or((
                "SCORM12-405-IDENTIFIER",
                format!("Invalid interaction id {value:?}"),
            )),
        "type" => matches!(
            value,
            "true-false"
                | "choice"
                | "fill-in"
                | "matching"
                | "performance"
                | "sequencing"
                | "likert"
                | "numeric"
        )
        .then_some(())
        .ok_or((
            "SCORM12-405-INTERACTION-TYPE",
            format!("Invalid SCORM 1.2 interaction type {value:?}"),
        )),
        "time" => is_valid_cmi_time(value)
            .then_some(())
            .ok_or(("SCORM12-405-TIME", format!("Invalid CMITime {value:?}"))),
        "latency" => parse_cmi_timespan_cs(value).map(|_| ()).ok_or((
            "SCORM12-405-TIMESPAN",
            format!("Invalid interaction latency {value:?}"),
        )),
        "weighting" => parse_plain_decimal(value).map(|_| ()).ok_or((
            "SCORM12-405-DECIMAL",
            format!("Invalid interaction weighting {value:?}"),
        )),
        "result" => {
            let valid = matches!(value, "correct" | "wrong" | "unanticipated" | "neutral")
                || parse_plain_decimal(value).is_some();
            valid.then_some(()).ok_or((
                "SCORM12-405-RESULT",
                format!("Invalid SCORM 1.2 interaction result {value:?}"),
            ))
        }
        "student_response" => validate_feedback(value, kind, strict_single_char_ids),
        _ => {
            if let Some((_, "pattern")) = parse_indexed_key(tail, "correct_responses.") {
                return validate_feedback(value, kind, strict_single_char_ids);
            }
            if let Some((_, "id")) = parse_indexed_key(tail, "objectives.") {
                return is_valid_scorm_identifier(value, 255, true)
                    .then_some(())
                    .ok_or((
                        "SCORM12-405-IDENTIFIER",
                        format!("Invalid interaction objective id {value:?}"),
                    ));
            }
            Ok(())
        }
    }
}

fn validate_feedback(
    value: &str,
    kind: &str,
    strict_single_char_ids: bool,
) -> Result<(), (&'static str, String)> {
    if value.chars().count() > 255 {
        return Err((
            "SCORM12-405-LENGTH",
            format!(
                "Interaction response exceeds 255 chars ({})",
                value.chars().count()
            ),
        ));
    }
    if kind == "true-false" && !matches!(value, "0" | "1" | "t" | "f") {
        return Err((
            "SCORM12-405-FEEDBACK-TF",
            format!("SCORM 1.2 true-false feedback must be 0|1|t|f, got {value:?}"),
        ));
    }
    if kind == "choice"
        && strict_single_char_ids
        && !value.is_empty()
        && value.split(',').any(|p| p.chars().count() != 1)
    {
        return Err((
            "SCORM12-405-FEEDBACK-CHOICE",
            format!("Strict SCORM 1.2 choice feedback requires single-char IDs, got {value:?}"),
        ));
    }
    Ok(())
}

fn apply_interaction_field(inter: &mut Scorm12Interaction, tail: &str, value: &str) {
    match tail {
        "id" => inter.id = value.to_owned(),
        "type" => inter.kind = value.to_owned(),
        "time" => inter.time = value.to_owned(),
        "weighting" => inter.weighting = value.to_owned(),
        "student_response" => inter.student_response = value.to_owned(),
        "result" => inter.result = value.to_owned(),
        "latency" => inter.latency = value.to_owned(),
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

fn error_string(code: &str) -> &'static str {
    match code {
        "0" => "No error",
        "101" => "General exception",
        "201" => "Invalid argument error",
        "202" => "Element cannot have children",
        "203" => "Element not an array - cannot have count",
        "301" => "Not initialized",
        "401" => "Not implemented error",
        "402" => "Invalid set value, element is a keyword",
        "403" => "Element is read only",
        "404" => "Element is write only",
        "405" => "Incorrect data type",
        _ => "",
    }
}
