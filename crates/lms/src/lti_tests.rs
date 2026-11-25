//! Tests for turning effects into LTI score reports.

use coursecraft_course::CompletionStatus;
use coursecraft_logic::Effect;

use crate::lti::{LtiReporter, ScoreReport};

fn score(percent: u8, status: CompletionStatus) -> Effect {
    Effect::Score {
        raw: u32::from(percent),
        max: 100,
        percent,
        status,
    }
}

fn completion(status: CompletionStatus) -> Effect {
    Effect::Completion { status }
}

#[test]
fn a_score_effect_becomes_a_reported_percentage() {
    let mut reporter = LtiReporter::new();
    reporter.record(&[score(87, CompletionStatus::Passed)]);

    assert_eq!(
        reporter.take_pending(),
        Some(ScoreReport {
            percent: Some(87),
            status: "passed",
        })
    );
    assert!(!reporter.has_pending());
}

#[test]
fn a_completion_without_a_score_reports_progress_only() {
    let mut reporter = LtiReporter::new();
    reporter.record(&[completion(CompletionStatus::Completed)]);

    assert_eq!(
        reporter.take_pending(),
        Some(ScoreReport {
            percent: None,
            status: "completed",
        })
    );
}

#[test]
fn score_and_completion_in_one_batch_are_one_score_report() {
    let mut reporter = LtiReporter::new();
    reporter.record(&[
        score(55, CompletionStatus::Failed),
        completion(CompletionStatus::Failed),
    ]);

    assert_eq!(
        reporter.take_pending(),
        Some(ScoreReport {
            percent: Some(55),
            status: "failed",
        })
    );
    assert!(!reporter.has_pending());
}

#[test]
fn only_the_latest_unsent_report_is_kept() {
    let mut reporter = LtiReporter::new();
    reporter.record(&[completion(CompletionStatus::Incomplete)]);
    reporter.record(&[completion(CompletionStatus::Completed)]);

    assert_eq!(
        reporter.take_pending(),
        Some(ScoreReport {
            percent: None,
            status: "completed",
        })
    );
    assert!(!reporter.has_pending());
}

#[test]
fn an_identical_report_after_one_was_sent_is_deduplicated() {
    let mut reporter = LtiReporter::new();
    reporter.record(&[score(70, CompletionStatus::Passed)]);
    assert!(reporter.take_pending().is_some());
    reporter.mark_sent();

    reporter.record(&[score(70, CompletionStatus::Passed)]);
    assert_eq!(reporter.take_pending(), None);
}

#[test]
fn a_bare_completion_cannot_replace_a_sent_final_score() {
    let mut reporter = LtiReporter::new();
    reporter.record(&[score(70, CompletionStatus::Passed)]);
    assert!(reporter.take_pending().is_some());
    reporter.mark_sent();

    reporter.record(&[completion(CompletionStatus::Completed)]);
    assert_eq!(reporter.take_pending(), None);
}

#[test]
fn a_failure_is_retried_once_then_given_up_as_lost() {
    let mut reporter = LtiReporter::new();
    reporter.record(&[score(42, CompletionStatus::Failed)]);

    let first = reporter.take_pending().expect("first attempt");
    reporter.mark_failed();
    assert!(!reporter.lost());
    assert_eq!(
        reporter.take_pending(),
        Some(first),
        "the report is retried"
    );

    reporter.mark_failed();
    assert!(reporter.lost());
    assert_eq!(reporter.take_pending(), None);
}

#[test]
fn a_lost_reporter_ignores_later_effects() {
    let mut reporter = LtiReporter::new();
    reporter.record(&[completion(CompletionStatus::Completed)]);
    assert!(reporter.take_pending().is_some());
    reporter.mark_failed();
    assert!(reporter.take_pending().is_some());
    reporter.mark_failed();
    assert!(reporter.lost());

    reporter.record(&[score(100, CompletionStatus::Passed)]);
    assert_eq!(reporter.take_pending(), None);
}

#[test]
fn report_json_uses_a_null_percentage_for_progress_only() {
    assert_eq!(
        ScoreReport {
            percent: Some(12),
            status: "passed",
        }
        .json_body(),
        r#"{"percent":12,"status":"passed"}"#
    );
    assert_eq!(
        ScoreReport {
            percent: None,
            status: "completed",
        }
        .json_body(),
        r#"{"percent":null,"status":"completed"}"#
    );
}

#[test]
fn a_sent_pass_cannot_be_overwritten_by_a_later_fail_or_lower_score() {
    let mut reporter = LtiReporter::new();
    reporter.record(&[score(85, CompletionStatus::Passed)]);
    assert!(reporter.take_pending().is_some());
    reporter.mark_sent();

    reporter.record(&[score(40, CompletionStatus::Failed)]);
    assert_eq!(reporter.take_pending(), None);

    reporter.record(&[score(70, CompletionStatus::Passed)]);
    assert_eq!(reporter.take_pending(), None);

    reporter.record(&[score(95, CompletionStatus::Passed)]);
    assert_eq!(
        reporter.take_pending(),
        Some(ScoreReport {
            percent: Some(95),
            status: "passed",
        })
    );
}

#[test]
fn tr_11_lti_reporter_monotonic_and_batch_combination() {
    // 1. Slides-viewed alone -> {percent: None, status: "completed"}.
    let mut r = LtiReporter::new();
    r.record(&[completion(CompletionStatus::Completed)]);
    assert_eq!(
        r.take_pending(),
        Some(ScoreReport {
            percent: None,
            status: "completed",
        })
    );
    r.mark_sent();

    // View-all-slides-then-fail: never downgrades "completed" to "failed".
    r.record(&[
        score(40, CompletionStatus::Failed),
        completion(CompletionStatus::Failed),
    ]);
    assert_eq!(r.take_pending(), None);

    // View-all-slides-then-pass: upgrades to {Some(90), "passed"}.
    r.record(&[
        score(90, CompletionStatus::Passed),
        completion(CompletionStatus::Passed),
    ]);
    assert_eq!(
        r.take_pending(),
        Some(ScoreReport {
            percent: Some(90),
            status: "passed",
        })
    );
    r.mark_sent();

    // Pass -> fail: never downgrades.
    r.record(&[score(20, CompletionStatus::Failed)]);
    assert_eq!(r.take_pending(), None);

    // 2. Fail-then-view-all-slides across batches: upgrades to {Some(40), "completed"}.
    let mut r2 = LtiReporter::new();
    r2.record(&[
        score(40, CompletionStatus::Failed),
        completion(CompletionStatus::Failed),
    ]);
    assert_eq!(
        r2.take_pending(),
        Some(ScoreReport {
            percent: Some(40),
            status: "failed",
        })
    );
    r2.mark_sent();

    r2.record(&[completion(CompletionStatus::Completed)]);
    assert_eq!(
        r2.take_pending(),
        Some(ScoreReport {
            percent: Some(40),
            status: "completed",
        })
    );

    // 3. Same-batch [Score(40, Failed), Completion(Completed)] -> {Some(40), "completed"}.
    let mut r3 = LtiReporter::new();
    r3.record(&[
        score(40, CompletionStatus::Failed),
        completion(CompletionStatus::Failed),
        completion(CompletionStatus::Completed),
    ]);
    assert_eq!(
        r3.take_pending(),
        Some(ScoreReport {
            percent: Some(40),
            status: "completed",
        })
    );
}
