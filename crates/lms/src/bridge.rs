//! SCORM 1.2 and SCORM 2004 LMS bridge implementation.

use coursecraft_course::{CompletionStatus, Course, LmsStatus};
use coursecraft_logic::{Effect, PlayerState, decode_suspend, encode_suspend};
use coursecraft_scorm::{LessonStatus, Standard};

use crate::api::CmiApi;
use crate::interaction::{
    InteractionKind, InteractionRecord, Response, interaction_record, sanitize_identifier,
};
use crate::time::{format_scaled_score, format_session_time, format_session_time_iso};

/// Maximum number of characters allowed for SCORM 1.2 `cmi.suspend_data`.
pub const SUSPEND_LIMIT: usize = 4096;

/// Maximum number of characters allowed for SCORM 2004 `cmi.suspend_data`.
pub const SUSPEND_LIMIT_2004: usize = 64_000;

/// Maximum number of characters allowed for SCORM 1.2 `cmi.core.lesson_location`.
pub const LOCATION_LIMIT_12: usize = 255;

/// Maximum number of characters allowed for SCORM 2004 `cmi.location`.
pub const LOCATION_LIMIT_2004: usize = 1000;

#[derive(Debug, Clone, Copy)]
struct ScoreKeys {
    min: &'static str,
    max: &'static str,
    raw: &'static str,
    scaled: Option<&'static str>,
}

/// Per-standard `cmi.interactions` names, vocabulary, separators and limits.
#[derive(Debug, Clone, Copy)]
struct InteractionMapping {
    count_key: &'static str,
    id_limit: usize,
    other_type: &'static str,
    description: bool,
    description_limit: usize,
    learner_key: &'static str,
    result_correct: &'static str,
    result_wrong: &'static str,
    bool_true: &'static str,
    bool_false: &'static str,
    id_separator: &'static str,
    pair_separator: &'static str,
    pairs_separator: &'static str,
    /// Limit applied to every interaction value, including text (SCORM 1.2).
    value_limit: Option<usize>,
    /// Limit applied to text values only (SCORM 2004).
    text_limit: Option<usize>,
    escape_text: bool,
    case_prefix: bool,
}

/// Every standard-specific data-model name, value and limit in one place.
#[derive(Debug, Clone, Copy)]
struct CmiMapping {
    standard: Standard,
    /// Elements read to decide whether the learner already finished.
    status_keys: &'static [&'static str],
    location_key: &'static str,
    location_limit: Option<usize>,
    suspend_key: &'static str,
    suspend_limit: usize,
    learner_name_key: &'static str,
    exit_key: &'static str,
    exit_suspend: &'static str,
    exit_finished: &'static str,
    session_time_key: &'static str,
    score: ScoreKeys,
    interaction: InteractionMapping,
}

impl CmiMapping {
    fn for_standard(standard: Standard) -> Self {
        match standard {
            Standard::Scorm12 => Self {
                standard,
                status_keys: &["cmi.core.lesson_status"],
                location_key: "cmi.core.lesson_location",
                location_limit: Some(LOCATION_LIMIT_12),
                suspend_key: "cmi.suspend_data",
                suspend_limit: SUSPEND_LIMIT,
                learner_name_key: "cmi.core.student_name",
                exit_key: "cmi.core.exit",
                exit_suspend: "suspend",
                exit_finished: "",
                session_time_key: "cmi.core.session_time",
                score: ScoreKeys {
                    min: "cmi.core.score.min",
                    max: "cmi.core.score.max",
                    raw: "cmi.core.score.raw",
                    scaled: None,
                },
                interaction: InteractionMapping {
                    count_key: "cmi.interactions._count",
                    id_limit: 255,
                    other_type: "performance",
                    description: false,
                    description_limit: 0,
                    learner_key: "student_response",
                    result_correct: "correct",
                    result_wrong: "wrong",
                    bool_true: "t",
                    bool_false: "f",
                    id_separator: ",",
                    pair_separator: ".",
                    pairs_separator: ",",
                    value_limit: Some(255),
                    text_limit: None,
                    escape_text: false,
                    case_prefix: false,
                },
            },
            Standard::Scorm2004 => Self {
                standard,
                status_keys: &["cmi.completion_status", "cmi.success_status"],
                location_key: "cmi.location",
                location_limit: Some(LOCATION_LIMIT_2004),
                suspend_key: "cmi.suspend_data",
                suspend_limit: SUSPEND_LIMIT_2004,
                learner_name_key: "cmi.learner_name",
                exit_key: "cmi.exit",
                exit_suspend: "suspend",
                exit_finished: "normal",
                session_time_key: "cmi.session_time",
                score: ScoreKeys {
                    min: "cmi.score.min",
                    max: "cmi.score.max",
                    raw: "cmi.score.raw",
                    scaled: Some("cmi.score.scaled"),
                },
                interaction: InteractionMapping {
                    count_key: "cmi.interactions._count",
                    id_limit: 4000,
                    other_type: "other",
                    description: true,
                    description_limit: 250,
                    learner_key: "learner_response",
                    result_correct: "correct",
                    result_wrong: "incorrect",
                    bool_true: "true",
                    bool_false: "false",
                    id_separator: "[,]",
                    pair_separator: "[.]",
                    pairs_separator: "[,]",
                    value_limit: None,
                    text_limit: Some(250),
                    escape_text: true,
                    case_prefix: true,
                },
            },
        }
    }

    fn read_statuses(&self, api: &mut impl CmiApi) -> Vec<String> {
        self.status_keys
            .iter()
            .map(|key| api.get_value(key))
            .collect()
    }

    fn is_first_launch(&self, statuses: &[String]) -> bool {
        match self.standard {
            Standard::Scorm12 => statuses
                .first()
                .is_some_and(|s| matches!(s.as_str(), "not attempted" | "")),
            Standard::Scorm2004 => statuses
                .first()
                .is_some_and(|s| matches!(s.as_str(), "unknown" | "not attempted" | "")),
        }
    }

    fn is_finished(&self, statuses: &[String]) -> bool {
        match self.standard {
            Standard::Scorm12 => statuses
                .first()
                .is_some_and(|s| matches!(s.as_str(), "passed" | "failed" | "completed")),
            Standard::Scorm2004 => {
                statuses.first().is_some_and(|s| s == "completed")
                    || statuses
                        .get(1)
                        .is_some_and(|s| matches!(s.as_str(), "passed" | "failed"))
            }
        }
    }

    fn is_passed_for(&self, statuses: &[String], lms_status: LmsStatus) -> bool {
        match self.standard {
            Standard::Scorm12 => statuses.first().is_some_and(|s| match lms_status {
                LmsStatus::Auto
                | LmsStatus::AutoKeepCompleted
                | LmsStatus::PassedIncomplete
                | LmsStatus::PassedFailed => s == "passed",
                LmsStatus::CompletedIncomplete | LmsStatus::CompletedFailed => {
                    matches!(s.as_str(), "passed" | "completed")
                }
            }),
            Standard::Scorm2004 => statuses.get(1).is_some_and(|s| s == "passed"),
        }
    }

    fn write_status(
        &self,
        api: &mut impl CmiApi,
        status: CompletionStatus,
        last_status: &mut Option<CompletionStatus>,
        warnings: &mut Vec<String>,
        lms_status: LmsStatus,
    ) {
        if *last_status == Some(status) {
            return;
        }
        *last_status = Some(status);

        match self.standard {
            Standard::Scorm12 => set_cmi(
                api,
                "cmi.core.lesson_status",
                map_status(status, lms_status).as_cmi(),
                warnings,
            ),
            Standard::Scorm2004 => match status {
                CompletionStatus::Passed => {
                    set_cmi(api, "cmi.completion_status", "completed", warnings);
                    set_cmi(api, "cmi.success_status", "passed", warnings);
                }
                CompletionStatus::Failed => {
                    set_cmi(api, "cmi.completion_status", "completed", warnings);
                    set_cmi(api, "cmi.success_status", "failed", warnings);
                }
                CompletionStatus::Completed => {
                    set_cmi(api, "cmi.completion_status", "completed", warnings);
                }
                CompletionStatus::Incomplete => {
                    set_cmi(api, "cmi.completion_status", "incomplete", warnings);
                }
            },
        }
    }

    fn write_score(&self, api: &mut impl CmiApi, percent: u8, warnings: &mut Vec<String>) {
        set_cmi(api, self.score.min, "0", warnings);
        set_cmi(api, self.score.max, "100", warnings);
        set_cmi(api, self.score.raw, &percent.to_string(), warnings);
        if let Some(scaled_key) = self.score.scaled {
            set_cmi(api, scaled_key, &format_scaled_score(percent), warnings);
        }
    }

    fn write_location(&self, api: &mut impl CmiApi, slide: &str, warnings: &mut Vec<String>) {
        if let Some(limit) = self.location_limit {
            let length = slide.chars().count();
            if length > limit {
                warnings.push(format!("location length {length} exceeds limit {limit}"));
                return;
            }
        }
        set_cmi(api, self.location_key, slide, warnings);
    }

    fn write_session_time(
        &self,
        api: &mut impl CmiApi,
        session_ms: u64,
        warnings: &mut Vec<String>,
    ) {
        let time = match self.standard {
            Standard::Scorm12 => format_session_time(session_ms),
            Standard::Scorm2004 => format_session_time_iso(session_ms),
        };
        set_cmi(api, self.session_time_key, &time, warnings);
    }

    fn write_interaction(
        &self,
        api: &mut impl CmiApi,
        index: u64,
        record: &InteractionRecord,
        warnings: &mut Vec<String>,
    ) -> bool {
        let base = format!("cmi.interactions.{index}");
        let id = truncate_chars(&sanitize_identifier(&record.id), self.interaction.id_limit);
        if !api.set_value(&format!("{base}.id"), &id) {
            warnings.push(format!("set_value {base}.id failed: {}", api.last_error()));
            return false;
        }
        let kind_str = match record.kind {
            InteractionKind::Other => self.interaction.other_type,
            other => other.as_cmi_type(),
        };
        set_cmi(api, &format!("{base}.type"), kind_str, warnings);
        if self.interaction.description {
            let description = sanitize_text(&record.description, self.interaction.escape_text);
            let description = truncate_chars(&description, self.interaction.description_limit);
            set_cmi(api, &format!("{base}.description"), &description, warnings);
        }
        set_cmi(
            api,
            &format!("{base}.weighting"),
            &record.weighting.to_string(),
            warnings,
        );
        let cap = match record.kind {
            InteractionKind::FillIn
            | InteractionKind::Matching
            | InteractionKind::Sequencing
            | InteractionKind::Likert => 5,
            InteractionKind::Choice | InteractionKind::TrueFalse => 10,
            InteractionKind::Other => 1,
        };
        for (m, correct) in record.correct.iter().take(cap).enumerate() {
            let value = self.interaction_value(correct, true, record.case_sensitive);
            set_cmi(
                api,
                &format!("{base}.correct_responses.{m}.pattern"),
                &value,
                warnings,
            );
        }
        let learner = self.interaction_value(&record.learner, false, record.case_sensitive);
        set_cmi(
            api,
            &format!("{base}.{}", self.interaction.learner_key),
            &learner,
            warnings,
        );
        let result = if record.survey {
            "neutral"
        } else if record.is_correct {
            self.interaction.result_correct
        } else {
            self.interaction.result_wrong
        };
        set_cmi(api, &format!("{base}.result"), result, warnings);
        true
    }

    fn interaction_value(
        &self,
        response: &Response,
        correct: bool,
        case_sensitive: bool,
    ) -> String {
        let mapping = self.interaction;
        let (raw, is_text) = match response {
            Response::Bool(value) => {
                let text = if *value {
                    mapping.bool_true
                } else {
                    mapping.bool_false
                };
                (text.to_owned(), false)
            }
            Response::Ids(ids) => {
                let joined = ids
                    .iter()
                    .map(|id| sanitize_identifier(id))
                    .collect::<Vec<_>>()
                    .join(mapping.id_separator);
                (joined, false)
            }
            Response::Pairs(pairs) => {
                let joined = pairs
                    .iter()
                    .map(|(left, right)| {
                        format!(
                            "{}{}{}",
                            sanitize_identifier(left),
                            mapping.pair_separator,
                            sanitize_identifier(right)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(mapping.pairs_separator);
                (joined, false)
            }
            Response::Text(text) => {
                let mut value = sanitize_text(text, mapping.escape_text);
                if correct && case_sensitive && mapping.case_prefix {
                    value = format!("{{case_matters=true}}{value}");
                }
                (value, true)
            }
        };
        if let Some(limit) = mapping.value_limit {
            truncate_chars(&raw, limit)
        } else if is_text {
            let limit = mapping.text_limit.unwrap_or(usize::MAX);
            truncate_chars(&raw, limit)
        } else {
            raw
        }
    }
}

fn sanitize_text(value: &str, escape_sequences: bool) -> String {
    let text: String = value.chars().filter(|c| !c.is_control()).collect();
    if escape_sequences {
        text.replace("[,]", ",").replace("[.]", ".")
    } else {
        text
    }
}

fn truncate_chars(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_owned();
    }
    value.chars().take(max).collect()
}

impl Default for CmiMapping {
    fn default() -> Self {
        Self::for_standard(Standard::Scorm12)
    }
}

/// What the player app should do after starting the LMS bridge.
#[derive(Debug, Clone, PartialEq)]
pub struct StartPlan {
    /// Saved player state to resume from; `None` starts fresh from the start slide.
    pub resume: Option<PlayerState>,
    /// The learner's name supplied by the LMS, when it has one.
    pub learner_name: Option<String>,
}

/// Bridges runtime player events and effects to a SCORM LMS session.
#[derive(Debug, Default)]
pub struct LmsBridge {
    mapping: CmiMapping,
    lms_status: LmsStatus,
    active: bool,
    last_suspend: Option<String>,
    finished: bool,
    completed: bool,
    ever_completed: bool,
    /// SCORM 1.2: rank of the highest `cmi.core.lesson_status` reported (see `scorm12_rank`).
    scorm12_rank: u8,
    passed: bool,
    best_passed_score: Option<f64>,
    /// Next `cmi.interactions.n` index, or `None` when reporting is unavailable.
    interaction_index: Option<u64>,
}

impl LmsBridge {
    /// Create an uninitialized SCORM 1.2 LMS bridge.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an uninitialized LMS bridge for a specific standard.
    #[must_use]
    pub fn for_standard(standard: Standard) -> Self {
        Self {
            mapping: CmiMapping::for_standard(standard),
            ..Self::default()
        }
    }

    /// Configure how SCORM 1.2 maps runtime status to `cmi.core.lesson_status`.
    #[must_use]
    pub fn with_lms_status(mut self, lms_status: LmsStatus) -> Self {
        self.lms_status = lms_status;
        self
    }

    /// The standard this bridge writes.
    #[must_use]
    pub fn standard(&self) -> Standard {
        self.mapping.standard
    }

    /// Whether communication with the LMS has been successfully initialized.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Whether the LMS session has been finished.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Whether a passed, failed, or completed status was written to the LMS.
    #[must_use]
    pub fn is_completed(&self) -> bool {
        self.completed
    }

    /// Initialize the LMS connection, check lesson status, and determine
    /// whether to start fresh or resume.
    pub fn start(&mut self, api: &mut impl CmiApi, course: &Course) -> (StartPlan, Vec<String>) {
        let mut warnings = Vec::new();
        if !api.initialize() {
            self.active = false;
            warnings.push(format!("LMSInitialize failed: {}", api.last_error()));
            return (
                StartPlan {
                    resume: None,
                    learner_name: None,
                },
                warnings,
            );
        }
        self.active = true;
        self.lms_status = course
            .editor
            .last_publish
            .as_ref()
            .map_or(LmsStatus::Auto, |p| p.lms_status);

        let learner_name = {
            let name = api.get_value(self.mapping.learner_name_key);
            if name.trim().is_empty() {
                None
            } else {
                Some(name)
            }
        };

        let count = api.get_value(self.mapping.interaction.count_key);
        match count.trim().parse::<u64>() {
            Ok(index) => self.interaction_index = Some(index),
            Err(_) => {
                self.interaction_index = None;
                warnings.push(format!(
                    "{} is not a non-negative integer; interaction reporting disabled",
                    self.mapping.interaction.count_key
                ));
            }
        }

        let statuses = self.mapping.read_statuses(api);
        self.scorm12_rank = match self.mapping.standard {
            Standard::Scorm12 => statuses
                .first()
                .and_then(|s| parse_lesson_status(s))
                .map_or(0, |ls| scorm12_rank(ls, self.lms_status)),
            Standard::Scorm2004 => 0,
        };
        self.ever_completed = match self.mapping.standard {
            Standard::Scorm12 => matches!(
                statuses.first().map(String::as_str),
                Some("completed" | "passed")
            ),
            Standard::Scorm2004 => {
                statuses.first().is_some_and(|s| s == "completed")
                    || statuses.get(1).is_some_and(|s| s == "passed")
            }
        };
        if self.mapping.is_finished(&statuses) {
            self.completed = true;
        } else if self.mapping.is_first_launch(&statuses) {
            let mut last_status = None;
            self.mapping.write_status(
                api,
                CompletionStatus::Incomplete,
                &mut last_status,
                &mut warnings,
                self.lms_status,
            );
        }
        if self.mapping.is_passed_for(&statuses, self.lms_status) {
            self.passed = true;
            self.best_passed_score = api
                .get_value(self.mapping.score.raw)
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|&percent| percent.is_finite() && (0.0..=100.0).contains(&percent));
        }

        let suspend_data = api.get_value(self.mapping.suspend_key);
        if suspend_data.is_empty() {
            self.last_suspend = None;
            (
                StartPlan {
                    resume: None,
                    learner_name,
                },
                warnings,
            )
        } else {
            match decode_suspend(course, &suspend_data) {
                Ok(state) => {
                    self.last_suspend = Some(suspend_data);
                    (
                        StartPlan {
                            resume: Some(state),
                            learner_name,
                        },
                        warnings,
                    )
                }
                Err(err) => {
                    self.last_suspend = None;
                    warnings.push(format!("failed to decode suspend_data: {err}"));
                    (
                        StartPlan {
                            resume: None,
                            learner_name,
                        },
                        warnings,
                    )
                }
            }
        }
    }

    /// Update LMS state after an input is handled by the player.
    ///
    /// Effects are handled in order. Suspend data is written once at the end if
    /// durable state changed, followed by at most one commit if any effect
    /// required it. An `Exit` effect finishes the session using `session_ms`.
    pub fn after_input(
        &mut self,
        api: &mut impl CmiApi,
        course: &Course,
        state: &PlayerState,
        effects: &[Effect],
        session_ms: u64,
    ) -> Vec<String> {
        if !self.active || self.finished {
            return Vec::new();
        }

        let mut warnings = Vec::new();
        let mut durable_changed = false;
        let mut needs_commit = false;
        let mut should_finish = false;
        let mut last_status: Option<CompletionStatus> = None;

        for effect in effects {
            match effect {
                Effect::Score {
                    percent, status, ..
                } => {
                    self.handle_score(api, *percent, *status, &mut last_status, &mut warnings);
                    needs_commit = true;
                }
                Effect::Completion { status } => {
                    self.handle_completion(api, *status, &mut last_status, &mut warnings);
                    needs_commit = true;
                }
                Effect::SlideEntered { slide } => {
                    self.mapping.write_location(api, slide, &mut warnings);
                    durable_changed = true;
                    needs_commit = true;
                }
                Effect::StateChanged { .. } | Effect::VariableChanged { .. } => {
                    durable_changed = true;
                }
                // The lightboxed slide becomes visited, so progress is durable,
                // but `cmi.location` stays on the base slide.
                Effect::LightboxOpened { .. } | Effect::LightboxClosed { .. } => {
                    durable_changed = true;
                }
                Effect::QuestionAnswered { slide, .. } => {
                    self.write_interaction(api, course, state, slide, &mut warnings);
                    durable_changed = true;
                    needs_commit = true;
                }
                Effect::Exit => {
                    should_finish = true;
                }
                Effect::LayerShown { .. }
                | Effect::LayerHidden { .. }
                | Effect::Media { .. }
                | Effect::MediaSeek { .. }
                | Effect::CueReached { .. }
                | Effect::TimelinePaused { .. }
                | Effect::TimelineResumed { .. }
                | Effect::ChoiceSelected { .. }
                | Effect::TextEntryChanged { .. }
                | Effect::SequenceChanged { .. }
                | Effect::MatchChanged { .. }
                | Effect::HotspotMarked { .. }
                | Effect::ItemDropped { .. }
                | Effect::ObjectMoved { .. }
                | Effect::ObjectDropped { .. }
                | Effect::AnswerRequired
                | Effect::MarkerOpened { .. }
                | Effect::MarkerClosed { .. }
                | Effect::TimerStarted { .. }
                | Effect::TimerPaused { .. }
                | Effect::TimerEnded { .. }
                | Effect::TimerReset { .. }
                | Effect::OpenUrl { .. }
                | Effect::RunJavaScript { .. }
                | Effect::ReviewChanged { .. }
                | Effect::Print { .. }
                | Effect::Warning { .. } => {}
            }
        }

        if durable_changed {
            self.write_suspend_data(api, course, state, &mut warnings);
        }

        if should_finish {
            warnings.extend(self.finish_session(api, session_ms));
        } else if needs_commit && !api.commit() {
            warnings.push(format!("commit failed: {}", api.last_error()));
        }

        warnings
    }

    /// Finish the LMS session.
    ///
    /// Sets the standard's exit value to `"suspend"` while not completed, or
    /// `""` (1.2) / `"normal"` (2004) once passed, failed, or completed. Sets
    /// the session time, commits, and finishes. Subsequent calls do nothing.
    pub fn finish_session(&mut self, api: &mut impl CmiApi, session_ms: u64) -> Vec<String> {
        if !self.active || self.finished {
            return Vec::new();
        }
        self.finished = true;

        let mut warnings = Vec::new();

        let exit_value = if self.completed {
            self.mapping.exit_finished
        } else {
            self.mapping.exit_suspend
        };
        set_cmi(api, self.mapping.exit_key, exit_value, &mut warnings);

        self.mapping
            .write_session_time(api, session_ms, &mut warnings);

        if !api.commit() {
            warnings.push(format!("commit failed: {}", api.last_error()));
        }

        if !api.finish() {
            warnings.push(format!("finish failed: {}", api.last_error()));
        }

        warnings
    }

    fn handle_score(
        &mut self,
        api: &mut impl CmiApi,
        percent: u8,
        status: CompletionStatus,
        last_status: &mut Option<CompletionStatus>,
        warnings: &mut Vec<String>,
    ) {
        let percent_f = f64::from(percent);
        let status_allowed = match self.mapping.standard {
            Standard::Scorm12 => {
                is_protected_status(map_status(status, self.lms_status), self.lms_status)
            }
            Standard::Scorm2004 => status == CompletionStatus::Passed,
        };
        if self.passed
            && (!status_allowed || self.best_passed_score.is_some_and(|best| percent_f < best))
        {
            return;
        }
        self.mapping.write_score(api, percent, warnings);
        let allow_status_write = match self.mapping.standard {
            Standard::Scorm12 => {
                scorm12_rank(map_status(status, self.lms_status), self.lms_status)
                    >= self.scorm12_rank
            }
            Standard::Scorm2004 => !(self.ever_completed && status == CompletionStatus::Incomplete),
        };
        if allow_status_write {
            self.mapping
                .write_status(api, status, last_status, warnings, self.lms_status);
            self.raise_scorm12_rank(status);
            let now_ever_completed = match self.mapping.standard {
                Standard::Scorm12 => matches!(
                    map_status(status, self.lms_status),
                    LessonStatus::Completed | LessonStatus::Passed
                ),
                Standard::Scorm2004 => matches!(
                    status,
                    CompletionStatus::Passed
                        | CompletionStatus::Completed
                        | CompletionStatus::Failed
                ),
            };
            if now_ever_completed {
                self.ever_completed = true;
            }
        }
        if status_allowed {
            self.passed = true;
            self.best_passed_score = Some(
                self.best_passed_score
                    .map_or(percent_f, |best| best.max(percent_f)),
            );
        }
        let is_finished_status = match self.mapping.standard {
            Standard::Scorm12 => !matches!(
                map_status(status, self.lms_status),
                LessonStatus::Incomplete
            ),
            Standard::Scorm2004 => matches!(
                status,
                CompletionStatus::Passed | CompletionStatus::Failed | CompletionStatus::Completed
            ),
        };
        if is_finished_status {
            self.completed = true;
        }
    }

    fn raise_scorm12_rank(&mut self, status: CompletionStatus) {
        if self.mapping.standard == Standard::Scorm12 {
            let rank = scorm12_rank(map_status(status, self.lms_status), self.lms_status);
            self.scorm12_rank = self.scorm12_rank.max(rank);
        }
    }

    fn handle_completion(
        &mut self,
        api: &mut impl CmiApi,
        status: CompletionStatus,
        last_status: &mut Option<CompletionStatus>,
        warnings: &mut Vec<String>,
    ) {
        let protected_after_pass = match self.mapping.standard {
            Standard::Scorm12 => {
                !is_protected_status(map_status(status, self.lms_status), self.lms_status)
            }
            Standard::Scorm2004 => {
                matches!(
                    status,
                    CompletionStatus::Failed | CompletionStatus::Incomplete
                )
            }
        };
        if self.passed && protected_after_pass {
            return;
        }
        let downgrade = match self.mapping.standard {
            Standard::Scorm12 => {
                scorm12_rank(map_status(status, self.lms_status), self.lms_status)
                    < self.scorm12_rank
            }
            Standard::Scorm2004 => self.ever_completed && status == CompletionStatus::Incomplete,
        };
        if downgrade {
            return;
        }
        self.mapping
            .write_status(api, status, last_status, warnings, self.lms_status);
        self.raise_scorm12_rank(status);
        let now_ever_completed = match self.mapping.standard {
            Standard::Scorm12 => matches!(
                map_status(status, self.lms_status),
                LessonStatus::Completed | LessonStatus::Passed
            ),
            Standard::Scorm2004 => matches!(
                status,
                CompletionStatus::Passed | CompletionStatus::Completed | CompletionStatus::Failed
            ),
        };
        if now_ever_completed {
            self.ever_completed = true;
        }
        if status == CompletionStatus::Passed
            || (self.mapping.standard == Standard::Scorm12
                && is_protected_status(map_status(status, self.lms_status), self.lms_status))
        {
            self.passed = true;
        }
        let is_finished_status = match self.mapping.standard {
            Standard::Scorm12 => !matches!(
                map_status(status, self.lms_status),
                LessonStatus::Incomplete
            ),
            Standard::Scorm2004 => matches!(
                status,
                CompletionStatus::Passed | CompletionStatus::Failed | CompletionStatus::Completed
            ),
        };
        if is_finished_status {
            self.completed = true;
        }
    }

    fn write_interaction(
        &mut self,
        api: &mut impl CmiApi,
        course: &Course,
        state: &PlayerState,
        slide: &str,
        warnings: &mut Vec<String>,
    ) {
        let Some(index) = self.interaction_index else {
            return;
        };
        let Some(record) = interaction_record(course, slide, state) else {
            return;
        };
        if self
            .mapping
            .write_interaction(api, index, &record, warnings)
        {
            self.interaction_index = Some(index.saturating_add(1));
        }
    }

    fn write_suspend_data(
        &mut self,
        api: &mut impl CmiApi,
        course: &Course,
        state: &PlayerState,
        warnings: &mut Vec<String>,
    ) {
        let encoded = encode_suspend(course, state);
        let limit = self.mapping.suspend_limit;
        if encoded.len() > limit {
            warnings.push(format!(
                "suspend_data length {} exceeds limit {limit}",
                encoded.len()
            ));
        } else if self.last_suspend.as_deref() != Some(&encoded) {
            if api.set_value(self.mapping.suspend_key, &encoded) {
                self.last_suspend = Some(encoded);
            } else {
                warnings.push(format!(
                    "set_value {} failed: {}",
                    self.mapping.suspend_key,
                    api.last_error()
                ));
            }
        }
    }
}

/// SCORM 1.2 status order for `lms_status`: a write never lowers the rank already reported.
/// `Auto` lets a quiz result outrank a plain completion (`incomplete < completed < failed <
/// passed`); every other option keeps a completion (`incomplete < failed < completed < passed`).
fn scorm12_rank(written: LessonStatus, lms_status: LmsStatus) -> u8 {
    match (written, lms_status) {
        (LessonStatus::Passed, _) => 3,
        (LessonStatus::Failed, LmsStatus::Auto) => 2,
        (LessonStatus::Completed, LmsStatus::Auto) | (LessonStatus::Failed, _) => 1,
        (LessonStatus::Completed, _) => 2,
        (LessonStatus::Incomplete | LessonStatus::Browsed | LessonStatus::NotAttempted, _) => 0,
    }
}

fn parse_lesson_status(s: &str) -> Option<LessonStatus> {
    match s {
        "passed" => Some(LessonStatus::Passed),
        "completed" => Some(LessonStatus::Completed),
        "failed" => Some(LessonStatus::Failed),
        "incomplete" => Some(LessonStatus::Incomplete),
        "browsed" => Some(LessonStatus::Browsed),
        "not attempted" => Some(LessonStatus::NotAttempted),
        _ => None,
    }
}

fn is_protected_status(written: LessonStatus, lms_status: LmsStatus) -> bool {
    match lms_status {
        LmsStatus::Auto
        | LmsStatus::AutoKeepCompleted
        | LmsStatus::PassedIncomplete
        | LmsStatus::PassedFailed => written == LessonStatus::Passed,
        LmsStatus::CompletedIncomplete | LmsStatus::CompletedFailed => {
            matches!(written, LessonStatus::Passed | LessonStatus::Completed)
        }
    }
}

fn map_status(status: CompletionStatus, lms_status: LmsStatus) -> LessonStatus {
    match lms_status {
        LmsStatus::Auto | LmsStatus::AutoKeepCompleted => match status {
            CompletionStatus::Passed => LessonStatus::Passed,
            CompletionStatus::Failed => LessonStatus::Failed,
            CompletionStatus::Completed => LessonStatus::Completed,
            CompletionStatus::Incomplete => LessonStatus::Incomplete,
        },
        LmsStatus::PassedIncomplete => match status {
            CompletionStatus::Passed | CompletionStatus::Completed => LessonStatus::Passed,
            CompletionStatus::Failed | CompletionStatus::Incomplete => LessonStatus::Incomplete,
        },
        LmsStatus::PassedFailed => match status {
            CompletionStatus::Passed | CompletionStatus::Completed => LessonStatus::Passed,
            CompletionStatus::Failed => LessonStatus::Failed,
            CompletionStatus::Incomplete => LessonStatus::Incomplete,
        },
        LmsStatus::CompletedIncomplete => match status {
            CompletionStatus::Passed | CompletionStatus::Completed => LessonStatus::Completed,
            CompletionStatus::Failed | CompletionStatus::Incomplete => LessonStatus::Incomplete,
        },
        LmsStatus::CompletedFailed => match status {
            CompletionStatus::Passed | CompletionStatus::Completed => LessonStatus::Completed,
            CompletionStatus::Failed => LessonStatus::Failed,
            CompletionStatus::Incomplete => LessonStatus::Incomplete,
        },
    }
}

fn set_cmi(api: &mut impl CmiApi, key: &str, val: &str, warnings: &mut Vec<String>) {
    if !api.set_value(key, val) {
        warnings.push(format!("set_value {key} failed: {}", api.last_error()));
    }
}
