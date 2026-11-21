//! LTI 1.3 score reporting as a small pure state machine.
//!
//! The published LTI package posts a score or completion report to the
//! CourseCraft LTI server on its own origin. This module turns runtime effects
//! into those reports, keeps only the latest unsent one, and remembers whether
//! the single retry was already spent. The player owns the HTTP transport, so
//! every rule here is testable without a browser or a network.
//!
//! LTI 1.3 has no suspend data: a relaunch starts the course fresh. Only the
//! final score and progress are reported.

use std::collections::VecDeque;

use coursecraft_course::CompletionStatus;
use coursecraft_logic::Effect;

/// One report for the LTI server's `/lti/score` endpoint.
///
/// A `percent` of `None` means the course completed without a score; the
/// server then omits `scoreGiven` and sends only the progress fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScoreReport {
    pub percent: Option<u8>,
    /// `passed`, `failed`, `completed` or `incomplete`.
    pub status: &'static str,
}

impl ScoreReport {
    /// The compact JSON body for `/lti/score`: `{"percent":…,"status":…}`.
    #[must_use]
    pub fn json_body(&self) -> String {
        match self.percent {
            Some(percent) => format!("{{\"percent\":{percent},\"status\":\"{}\"}}", self.status),
            None => format!("{{\"percent\":null,\"status\":\"{}\"}}", self.status),
        }
    }
}

/// Turns effects into at most one pending LTI report and tracks its delivery.
#[derive(Debug, Default)]
pub struct LtiReporter {
    pending: VecDeque<ScoreReport>,
    sent_final: bool,
    passed: bool,
    best_passed_score: Option<u8>,
    in_flight: Option<ScoreReport>,
    retried: bool,
    lost: bool,
    last_sent: Option<ScoreReport>,
}

impl LtiReporter {
    /// A reporter with nothing pending.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record the reports in one input's effects.
    ///
    /// A `Score` effect wins over a `Completion` effect in the same batch, so a
    /// quiz result is sent once with its percentage. A `Completion` on its own
    /// is sent with `percent: None`. Only the latest unsent report is kept, and
    /// a report identical to the last one sent is ignored.
    pub fn record(&mut self, effects: &[Effect]) {
        if self.lost {
            return;
        }
        let has_score = effects
            .iter()
            .any(|effect| matches!(effect, Effect::Score { .. }));
        let mut report = None;
        for effect in effects {
            match effect {
                Effect::Score {
                    percent, status, ..
                } => {
                    report = Some(ScoreReport {
                        percent: Some(*percent),
                        status: status_id(*status),
                    });
                }
                Effect::Completion { status } if !has_score => {
                    report = Some(ScoreReport {
                        percent: None,
                        status: status_id(*status),
                    });
                }
                _ => {}
            }
        }
        let Some(report) = report else {
            return;
        };
        if self.in_flight == Some(report) || self.last_sent == Some(report) {
            return;
        }
        if self.sent_final && report.percent.is_none() {
            // A final score already went out; a later bare completion must not
            // replace it with a progress-only report.
            return;
        }
        let pending_pass = self.pending.front().filter(|r| r.status == "passed");
        let in_flight_pass = self.in_flight.as_ref().filter(|r| r.status == "passed");
        let has_passed = self.passed || in_flight_pass.is_some() || pending_pass.is_some();
        if has_passed {
            if report.status != "passed" {
                return;
            }
            let best_score = [
                self.best_passed_score,
                in_flight_pass.and_then(|r| r.percent),
                pending_pass.and_then(|r| r.percent),
            ]
            .into_iter()
            .flatten()
            .max();
            match (report.percent, best_score) {
                (None, _) => return,
                (Some(new_percent), Some(best)) if new_percent <= best => return,
                _ => {}
            }
        }
        if self.in_flight.is_none() {
            self.retried = false;
        }
        self.pending.clear();
        self.pending.push_back(report);
    }

    /// Take the latest unsent report, if no other report is in flight.
    pub fn take_pending(&mut self) -> Option<ScoreReport> {
        if self.in_flight.is_some() {
            return None;
        }
        let report = self.pending.pop_front()?;
        self.in_flight = Some(report);
        Some(report)
    }

    /// The endpoint accepted the report (`204`).
    pub fn mark_sent(&mut self) {
        let Some(report) = self.in_flight.take() else {
            return;
        };
        if report.status != "incomplete" {
            self.sent_final = true;
        }
        if report.status == "passed" {
            self.passed = true;
            if let Some(percent) = report.percent {
                self.best_passed_score = Some(
                    self.best_passed_score
                        .map_or(percent, |best| best.max(percent)),
                );
            }
        }
        self.last_sent = Some(report);
        self.retried = false;
    }

    /// The endpoint refused the report, or the network failed.
    ///
    /// The report is re-queued once. A second failure gives up and sets
    /// [`LtiReporter::lost`], which the player turns into the lost-reporting
    /// notice.
    pub fn mark_failed(&mut self) {
        let Some(report) = self.in_flight.take() else {
            return;
        };
        if self.retried {
            self.lost = true;
            self.pending.clear();
        } else {
            self.retried = true;
            self.pending.push_front(report);
        }
    }

    /// Whether reporting was attempted and has now given up for this session.
    #[must_use]
    pub fn lost(&self) -> bool {
        self.lost
    }

    /// Whether a report is waiting to be sent.
    #[must_use]
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }
}

fn status_id(status: CompletionStatus) -> &'static str {
    match status {
        CompletionStatus::Passed => "passed",
        CompletionStatus::Failed => "failed",
        CompletionStatus::Completed => "completed",
        CompletionStatus::Incomplete => "incomplete",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a4_9_lti_reporter_high_water_dedup_and_retry_lost_reporting() {
        let mut reporter = LtiReporter::new();
        let mut sent = Vec::new();

        // Score(80, Passed) -> 204
        reporter.record(&[Effect::Score {
            raw: 80,
            max: 100,
            percent: 80,
            status: CompletionStatus::Passed,
        }]);
        let r1 = reporter.take_pending().expect("80 passed");
        assert_eq!(r1.percent, Some(80));
        assert_eq!(r1.status, "passed");
        sent.push(r1.percent);
        reporter.mark_sent();
        assert!(!reporter.lost());

        // Score(60, Failed) -> ignored (already passed)
        reporter.record(&[Effect::Score {
            raw: 60,
            max: 100,
            percent: 60,
            status: CompletionStatus::Failed,
        }]);
        assert!(reporter.take_pending().is_none());

        // Score(70, Passed) -> ignored (<= high-water 80)
        reporter.record(&[Effect::Score {
            raw: 70,
            max: 100,
            percent: 70,
            status: CompletionStatus::Passed,
        }]);
        assert!(reporter.take_pending().is_none());

        // Score(90, Passed) -> sent -> 500 -> retried once -> 500 -> lost() == true
        reporter.record(&[Effect::Score {
            raw: 90,
            max: 100,
            percent: 90,
            status: CompletionStatus::Passed,
        }]);
        let r2 = reporter.take_pending().expect("90 passed");
        assert_eq!(r2.percent, Some(90));
        sent.push(r2.percent);
        reporter.mark_failed();
        assert!(!reporter.lost());

        let r2_retry = reporter.take_pending().expect("90 retry");
        assert_eq!(r2_retry.percent, Some(90));
        sent.push(r2_retry.percent);
        reporter.mark_failed();
        assert!(reporter.lost());
        assert!(reporter.take_pending().is_none());

        assert_eq!(sent, vec![Some(80), Some(90), Some(90)]);
    }
}
