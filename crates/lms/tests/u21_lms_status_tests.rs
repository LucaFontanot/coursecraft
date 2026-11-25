//! Strict SCORM 1.2 & 2004 tests for `LmsStatus` mapping (Step u21).

use coursecraft_course::{CompletionStatus, Course, LmsStatus, PublishSettings};
use coursecraft_lms::{CmiApi, LmsBridge};
use coursecraft_logic::{Effect, PlayerState};
use coursecraft_scorm::Standard;
use coursecraft_strict_lms::{StrictScorm12, StrictScorm2004};

struct Strict12Adapter {
    inner: StrictScorm12,
}

impl Strict12Adapter {
    fn new(inner: StrictScorm12) -> Self {
        Self { inner }
    }
}

impl CmiApi for Strict12Adapter {
    fn initialize(&mut self) -> bool {
        self.inner.lms_initialize("") == "true"
    }

    fn get_value(&mut self, key: &str) -> String {
        self.inner.lms_get_value(key)
    }

    fn set_value(&mut self, key: &str, value: &str) -> bool {
        self.inner.lms_set_value(key, value) == "true"
    }

    fn commit(&mut self) -> bool {
        self.inner.lms_commit("") == "true"
    }

    fn finish(&mut self) -> bool {
        self.inner.lms_finish("") == "true"
    }

    fn last_error(&mut self) -> String {
        let code = self.inner.lms_get_last_error();
        let text = self.inner.lms_get_error_string(code);
        format!("{code}: {text}")
    }
}

struct Strict2004Adapter {
    inner: StrictScorm2004,
}

impl Strict2004Adapter {
    fn new(inner: StrictScorm2004) -> Self {
        Self { inner }
    }
}

impl CmiApi for Strict2004Adapter {
    fn initialize(&mut self) -> bool {
        self.inner.initialize("") == "true"
    }

    fn get_value(&mut self, key: &str) -> String {
        self.inner.get_value(key)
    }

    fn set_value(&mut self, key: &str, value: &str) -> bool {
        self.inner.set_value(key, value) == "true"
    }

    fn commit(&mut self) -> bool {
        self.inner.commit("") == "true"
    }

    fn finish(&mut self) -> bool {
        self.inner.terminate("") == "true"
    }

    fn last_error(&mut self) -> String {
        let code = self.inner.get_last_error();
        let text = self.inner.get_error_string(code);
        format!("{code}: {text}")
    }
}

fn hello_course_with_lms_status(lms_status: LmsStatus) -> Course {
    let json = include_str!("../../../examples/hello-course/course.json");
    let mut course = Course::from_json(json).expect("hello-course parses");
    course.editor.last_publish = Some(PublishSettings {
        standard: "scorm12".to_string(),
        lms_status,
    });
    course
}

#[test]
fn scorm12_lms_status_table_maps_every_option_and_status_with_zero_violations() {
    let cases = [
        (LmsStatus::Auto, CompletionStatus::Passed, "passed"),
        (LmsStatus::Auto, CompletionStatus::Failed, "failed"),
        (LmsStatus::Auto, CompletionStatus::Completed, "completed"),
        (LmsStatus::Auto, CompletionStatus::Incomplete, "incomplete"),
        (
            LmsStatus::AutoKeepCompleted,
            CompletionStatus::Passed,
            "passed",
        ),
        (
            LmsStatus::AutoKeepCompleted,
            CompletionStatus::Failed,
            "failed",
        ),
        (
            LmsStatus::AutoKeepCompleted,
            CompletionStatus::Completed,
            "completed",
        ),
        (
            LmsStatus::AutoKeepCompleted,
            CompletionStatus::Incomplete,
            "incomplete",
        ),
        (
            LmsStatus::PassedIncomplete,
            CompletionStatus::Passed,
            "passed",
        ),
        (
            LmsStatus::PassedIncomplete,
            CompletionStatus::Completed,
            "passed",
        ),
        (
            LmsStatus::PassedIncomplete,
            CompletionStatus::Failed,
            "incomplete",
        ),
        (
            LmsStatus::PassedIncomplete,
            CompletionStatus::Incomplete,
            "incomplete",
        ),
        (LmsStatus::PassedFailed, CompletionStatus::Passed, "passed"),
        (
            LmsStatus::PassedFailed,
            CompletionStatus::Completed,
            "passed",
        ),
        (LmsStatus::PassedFailed, CompletionStatus::Failed, "failed"),
        (
            LmsStatus::PassedFailed,
            CompletionStatus::Incomplete,
            "incomplete",
        ),
        (
            LmsStatus::CompletedIncomplete,
            CompletionStatus::Passed,
            "completed",
        ),
        (
            LmsStatus::CompletedIncomplete,
            CompletionStatus::Completed,
            "completed",
        ),
        (
            LmsStatus::CompletedIncomplete,
            CompletionStatus::Failed,
            "incomplete",
        ),
        (
            LmsStatus::CompletedIncomplete,
            CompletionStatus::Incomplete,
            "incomplete",
        ),
        (
            LmsStatus::CompletedFailed,
            CompletionStatus::Passed,
            "completed",
        ),
        (
            LmsStatus::CompletedFailed,
            CompletionStatus::Completed,
            "completed",
        ),
        (
            LmsStatus::CompletedFailed,
            CompletionStatus::Failed,
            "failed",
        ),
        (
            LmsStatus::CompletedFailed,
            CompletionStatus::Incomplete,
            "incomplete",
        ),
    ];

    for (option, runtime_status, expected_cmi) in cases {
        let course = hello_course_with_lms_status(option);
        let state = PlayerState::default();
        let mut api = Strict12Adapter::new(StrictScorm12::new());
        let mut bridge = LmsBridge::for_standard(Standard::Scorm12);

        let (_plan, start_warnings) = bridge.start(&mut api, &course);
        assert!(
            start_warnings.is_empty(),
            "start warnings: {start_warnings:?}"
        );

        let warnings = bridge.after_input(
            &mut api,
            &course,
            &state,
            &[Effect::Completion {
                status: runtime_status,
            }],
            1000,
        );
        assert!(warnings.is_empty(), "after_input warnings: {warnings:?}");

        assert_eq!(
            api.inner.lms_get_value("cmi.core.lesson_status"),
            expected_cmi,
            "mismatch for {option:?} / {runtime_status:?}"
        );

        let finish_warnings = bridge.finish_session(&mut api, 2000);
        assert!(
            finish_warnings.is_empty(),
            "finish warnings: {finish_warnings:?}"
        );

        assert_eq!(
            api.inner.violations(),
            &[],
            "expected 0 strict SCORM 1.2 violations for {option:?} / {runtime_status:?}"
        );
    }
}

#[test]
fn scorm2004_output_is_identical_for_every_lms_status_option() {
    let options = [
        LmsStatus::Auto,
        LmsStatus::AutoKeepCompleted,
        LmsStatus::PassedIncomplete,
        LmsStatus::PassedFailed,
        LmsStatus::CompletedIncomplete,
        LmsStatus::CompletedFailed,
    ];
    let statuses = [
        CompletionStatus::Passed,
        CompletionStatus::Failed,
        CompletionStatus::Completed,
        CompletionStatus::Incomplete,
    ];

    for runtime_status in statuses {
        let mut baseline: Option<(String, String)> = None;
        for option in options {
            let course = hello_course_with_lms_status(option);
            let state = PlayerState::default();
            let mut api = Strict2004Adapter::new(StrictScorm2004::new());
            let mut bridge = LmsBridge::for_standard(Standard::Scorm2004);

            let (_plan, start_warnings) = bridge.start(&mut api, &course);
            assert!(start_warnings.is_empty());
            let warnings = bridge.after_input(
                &mut api,
                &course,
                &state,
                &[Effect::Completion {
                    status: runtime_status,
                }],
                1000,
            );
            assert!(warnings.is_empty());
            let pair = (
                api.inner.get_value("cmi.completion_status"),
                api.inner.get_value("cmi.success_status"),
            );
            let finish_warnings = bridge.finish_session(&mut api, 2000);
            assert!(finish_warnings.is_empty());

            assert_eq!(api.inner.violations(), &[]);
            match &baseline {
                None => baseline = Some(pair),
                Some(expected) => assert_eq!(
                    &pair, expected,
                    "SCORM 2004 output differed for {option:?} / {runtime_status:?}"
                ),
            }
        }
    }
}

fn assert_scorm12_in_session_non_downgrade(
    option: LmsStatus,
    initial: CompletionStatus,
    expected_cmi: &str,
) {
    let course = hello_course_with_lms_status(option);
    let state = PlayerState::default();
    let mut api = Strict12Adapter::new(StrictScorm12::new());
    let mut bridge = LmsBridge::for_standard(Standard::Scorm12);
    assert!(bridge.start(&mut api, &course).1.is_empty());

    let w = bridge.after_input(
        &mut api,
        &course,
        &state,
        &[Effect::Completion { status: initial }],
        1000,
    );
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(
        api.inner.lms_get_value("cmi.core.lesson_status"),
        expected_cmi
    );

    let w = bridge.after_input(
        &mut api,
        &course,
        &state,
        &[
            Effect::Score {
                raw: 40,
                max: 100,
                percent: 40,
                status: CompletionStatus::Failed,
            },
            Effect::Completion {
                status: CompletionStatus::Failed,
            },
            Effect::Completion {
                status: CompletionStatus::Incomplete,
            },
        ],
        2000,
    );
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(
        api.inner.lms_get_value("cmi.core.lesson_status"),
        expected_cmi
    );
    assert_eq!(api.inner.lms_get_value("cmi.core.score.raw"), "");

    assert!(bridge.finish_session(&mut api, 3000).is_empty());
    assert_eq!(
        api.inner.violations(),
        &[],
        "option={option:?} initial={initial:?}"
    );
}

#[test]
fn m31_14_scorm12_never_downgrades_reported_pass_or_completion_in_session() {
    for option in [LmsStatus::PassedIncomplete, LmsStatus::PassedFailed] {
        assert_scorm12_in_session_non_downgrade(option, CompletionStatus::Completed, "passed");
        assert_scorm12_in_session_non_downgrade(option, CompletionStatus::Passed, "passed");
    }

    for option in [LmsStatus::CompletedIncomplete, LmsStatus::CompletedFailed] {
        assert_scorm12_in_session_non_downgrade(option, CompletionStatus::Completed, "completed");
        assert_scorm12_in_session_non_downgrade(option, CompletionStatus::Passed, "completed");
    }

    // Auto: Passed writes "passed" and blocks Failed/Incomplete/Completed; Completed can still upgrade to Passed.
    let course = hello_course_with_lms_status(LmsStatus::Auto);
    let state = PlayerState::default();
    let mut api = Strict12Adapter::new(StrictScorm12::new());
    let mut bridge = LmsBridge::for_standard(Standard::Scorm12);
    assert!(bridge.start(&mut api, &course).1.is_empty());

    let w = bridge.after_input(
        &mut api,
        &course,
        &state,
        &[Effect::Completion {
            status: CompletionStatus::Completed,
        }],
        500,
    );
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(
        api.inner.lms_get_value("cmi.core.lesson_status"),
        "completed"
    );

    let w = bridge.after_input(
        &mut api,
        &course,
        &state,
        &[Effect::Completion {
            status: CompletionStatus::Passed,
        }],
        1000,
    );
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(api.inner.lms_get_value("cmi.core.lesson_status"), "passed");

    let w = bridge.after_input(
        &mut api,
        &course,
        &state,
        &[
            Effect::Score {
                raw: 40,
                max: 100,
                percent: 40,
                status: CompletionStatus::Failed,
            },
            Effect::Completion {
                status: CompletionStatus::Failed,
            },
            Effect::Completion {
                status: CompletionStatus::Completed,
            },
            Effect::Completion {
                status: CompletionStatus::Incomplete,
            },
        ],
        2000,
    );
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(api.inner.lms_get_value("cmi.core.lesson_status"), "passed");
    assert_eq!(api.inner.lms_get_value("cmi.core.score.raw"), "");

    assert!(bridge.finish_session(&mut api, 3000).is_empty());
    assert_eq!(api.inner.violations(), &[]);
}

#[test]
fn m31_15_scorm12_resume_protects_status_across_all_five_modes() {
    let state = PlayerState::default();
    let modes = [
        (LmsStatus::Auto, "passed"),
        (LmsStatus::AutoKeepCompleted, "passed"),
        (LmsStatus::PassedIncomplete, "passed"),
        (LmsStatus::PassedFailed, "passed"),
        (LmsStatus::CompletedIncomplete, "completed"),
        (LmsStatus::CompletedFailed, "completed"),
    ];

    for (option, seeded_status) in modes {
        let course = hello_course_with_lms_status(option);
        let mut strict = StrictScorm12::new();
        strict.seed_status(seeded_status, Some("90"));
        let mut api = Strict12Adapter::new(strict);
        let mut bridge = LmsBridge::for_standard(Standard::Scorm12);
        assert!(bridge.start(&mut api, &course).1.is_empty());

        // Later Failed/40 and lower Passed/80 are rejected.
        let w = bridge.after_input(
            &mut api,
            &course,
            &state,
            &[
                Effect::Score {
                    raw: 40,
                    max: 100,
                    percent: 40,
                    status: CompletionStatus::Failed,
                },
                Effect::Completion {
                    status: CompletionStatus::Failed,
                },
                Effect::Completion {
                    status: CompletionStatus::Incomplete,
                },
                Effect::Score {
                    raw: 80,
                    max: 100,
                    percent: 80,
                    status: CompletionStatus::Passed,
                },
            ],
            1000,
        );
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(
            api.inner.lms_get_value("cmi.core.lesson_status"),
            seeded_status
        );
        assert_eq!(api.inner.lms_get_value("cmi.core.score.raw"), "90");

        // Higher Passed/95 updates score and keeps protected status.
        let w = bridge.after_input(
            &mut api,
            &course,
            &state,
            &[Effect::Score {
                raw: 95,
                max: 100,
                percent: 95,
                status: CompletionStatus::Passed,
            }],
            2000,
        );
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(
            api.inner.lms_get_value("cmi.core.lesson_status"),
            seeded_status
        );
        assert_eq!(api.inner.lms_get_value("cmi.core.score.raw"), "95");

        assert!(bridge.finish_session(&mut api, 3000).is_empty());
        assert_eq!(api.inner.violations(), &[], "option={option:?}");
    }
}

#[test]
fn m31_16_scorm12_completed_then_passing_score_and_scorm2004_regression() {
    let state = PlayerState::default();

    // Scoreless Completed followed by Score(90, Passed) still records 90, while Score(40, Failed) is ignored.
    for (option, expected_status) in [
        (LmsStatus::PassedIncomplete, "passed"),
        (LmsStatus::PassedFailed, "passed"),
        (LmsStatus::CompletedIncomplete, "completed"),
        (LmsStatus::CompletedFailed, "completed"),
    ] {
        let course = hello_course_with_lms_status(option);
        let mut api = Strict12Adapter::new(StrictScorm12::new());
        let mut bridge = LmsBridge::for_standard(Standard::Scorm12);
        assert!(bridge.start(&mut api, &course).1.is_empty());

        let w = bridge.after_input(
            &mut api,
            &course,
            &state,
            &[Effect::Completion {
                status: CompletionStatus::Completed,
            }],
            1000,
        );
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(
            api.inner.lms_get_value("cmi.core.lesson_status"),
            expected_status
        );

        let w = bridge.after_input(
            &mut api,
            &course,
            &state,
            &[
                Effect::Score {
                    raw: 90,
                    max: 100,
                    percent: 90,
                    status: CompletionStatus::Passed,
                },
                Effect::Score {
                    raw: 40,
                    max: 100,
                    percent: 40,
                    status: CompletionStatus::Failed,
                },
            ],
            2000,
        );
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(
            api.inner.lms_get_value("cmi.core.lesson_status"),
            expected_status
        );
        assert_eq!(api.inner.lms_get_value("cmi.core.score.raw"), "90");

        assert!(bridge.finish_session(&mut api, 3000).is_empty());
        assert_eq!(api.inner.violations(), &[], "option={option:?}");
    }

    // SCORM 2004 regression: Completed + Score(90, Passed) then Score(40, Failed) + Completion(Failed) keeps passed/90.
    {
        let course = hello_course_with_lms_status(LmsStatus::PassedIncomplete);
        let mut api = Strict2004Adapter::new(StrictScorm2004::new());
        let mut bridge = LmsBridge::for_standard(Standard::Scorm2004);
        assert!(bridge.start(&mut api, &course).1.is_empty());

        let w = bridge.after_input(
            &mut api,
            &course,
            &state,
            &[
                Effect::Completion {
                    status: CompletionStatus::Completed,
                },
                Effect::Score {
                    raw: 90,
                    max: 100,
                    percent: 90,
                    status: CompletionStatus::Passed,
                },
                Effect::Score {
                    raw: 40,
                    max: 100,
                    percent: 40,
                    status: CompletionStatus::Failed,
                },
                Effect::Completion {
                    status: CompletionStatus::Failed,
                },
            ],
            1000,
        );
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(api.inner.get_value("cmi.completion_status"), "completed");
        assert_eq!(api.inner.get_value("cmi.success_status"), "passed");
        assert_eq!(api.inner.get_value("cmi.score.raw"), "90");
        assert!(bridge.finish_session(&mut api, 2000).is_empty());
        assert_eq!(api.inner.violations(), &[]);
    }
}
