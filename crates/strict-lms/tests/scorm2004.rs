//! Unit tests for `StrictScorm2004` (M27 Cases A2.1–A2.12).

use coursecraft_strict_lms::{Scorm2004Config, Scorm2004Edition, StrictScorm12, StrictScorm2004};

#[test]
fn a2_1_exact_lifecycle_error_codes() {
    let mut lms = StrictScorm2004::new();
    check_before_init_2004(&mut lms);

    assert_eq!(lms.initialize("bad"), "false");
    assert_eq!(lms.get_last_error(), "201");
    assert_eq!(lms.initialize(""), "true");
    assert_eq!(lms.get_last_error(), "0");
    assert_eq!(lms.initialize(""), "false");
    assert_eq!(lms.get_last_error(), "103");

    assert_eq!(lms.terminate(""), "true");
    check_after_terminate_2004(&mut lms);
    assert_eq!(lms.violations().len(), 11);
}

fn check_before_init_2004(lms: &mut StrictScorm2004) {
    assert_eq!(lms.get_value("cmi.learner_name"), "");
    assert_eq!(lms.get_last_error(), "122");
    assert_eq!(lms.set_value("cmi.location", "s1"), "false");
    assert_eq!(lms.get_last_error(), "132");
    assert_eq!(lms.commit(""), "false");
    assert_eq!(lms.get_last_error(), "142");
    assert_eq!(lms.terminate(""), "false");
    assert_eq!(lms.get_last_error(), "112");
}

fn check_after_terminate_2004(lms: &mut StrictScorm2004) {
    assert_eq!(lms.initialize(""), "false");
    assert_eq!(lms.get_last_error(), "104");
    assert_eq!(lms.get_value("cmi.learner_name"), "");
    assert_eq!(lms.get_last_error(), "123");
    assert_eq!(lms.set_value("cmi.location", "s1"), "false");
    assert_eq!(lms.get_last_error(), "133");
    assert_eq!(lms.commit(""), "false");
    assert_eq!(lms.get_last_error(), "143");
    assert_eq!(lms.terminate(""), "false");
    assert_eq!(lms.get_last_error(), "113");
}

#[test]
fn a2_2_access_and_uninitialized_codes() {
    let mut lms = StrictScorm2004::new();
    assert_eq!(lms.initialize(""), "true");

    // 401 undefined
    assert_eq!(lms.get_value("cmi.unknown"), "");
    assert_eq!(lms.get_last_error(), "401");

    // 404 read-only
    assert_eq!(lms.set_value("cmi.learner_name", "Bob"), "false");
    assert_eq!(lms.get_last_error(), "404");

    // 405 write-only
    assert_eq!(lms.get_value("cmi.exit"), "");
    assert_eq!(lms.get_last_error(), "405");

    // 406 type mismatch
    assert_eq!(lms.set_value("cmi.completion_status", "done"), "false");
    assert_eq!(lms.get_last_error(), "406");

    // 407 out of range
    assert_eq!(lms.set_value("cmi.score.scaled", "1.5"), "false");
    assert_eq!(lms.get_last_error(), "407");

    // 408 dependency not established (.type before .id)
    assert_eq!(lms.set_value("cmi.interactions.0.type", "choice"), "false");
    assert_eq!(lms.get_last_error(), "408");

    assert_eq!(lms.violations().len(), 6);

    // 403 uninitialized read is informational, NOT a SCO violation
    assert_eq!(lms.get_value("cmi.location"), "");
    assert_eq!(lms.get_last_error(), "403");
    assert_eq!(lms.violations().len(), 6);
    assert_eq!(lms.warnings().len(), 1);
}

#[test]
fn a2_3_completion_and_success_independence() {
    let mut lms = StrictScorm2004::new();
    assert_eq!(lms.initialize(""), "true");

    assert_eq!(lms.set_value("cmi.completion_status", "completed"), "true");
    assert_eq!(lms.get_value("cmi.success_status"), "unknown");

    assert_eq!(lms.set_value("cmi.success_status", "passed"), "true");
    assert_eq!(lms.get_value("cmi.success_status"), "passed");
    assert!(lms.violations().is_empty());
}

#[test]
fn a2_4_scores_and_scaled_bounds() {
    let mut lms = StrictScorm2004::new();
    assert_eq!(lms.initialize(""), "true");

    for ok in ["-1", "-0.5", "0", "0.85", "1"] {
        assert_eq!(lms.set_value("cmi.score.scaled", ok), "true", "scaled {ok}");
    }
    assert_eq!(lms.set_value("cmi.score.scaled", "-1.01"), "false");
    assert_eq!(lms.get_last_error(), "407");
    assert_eq!(lms.set_value("cmi.score.scaled", "1.01"), "false");
    assert_eq!(lms.get_last_error(), "407");
    assert_eq!(lms.set_value("cmi.score.scaled", "1e-2"), "false");
    assert_eq!(lms.get_last_error(), "406");

    // Arbitrary finite real(10,7) raw/min/max allowed (no 0..=100 cap)
    for ok in ["250", "-10", "99.5"] {
        assert_eq!(lms.set_value("cmi.score.raw", ok), "true", "raw {ok}");
    }
    assert_eq!(lms.violations().len(), 3);
}

#[test]
fn a2_5_iso8601_session_time() {
    let mut lms = StrictScorm2004::new();
    assert_eq!(lms.initialize(""), "true");

    for ok in ["PT0S", "PT0.05S", "PT1H1M1.05S", "P1DT2H3M4.5S"] {
        assert_eq!(lms.set_value("cmi.session_time", ok), "true", "iso {ok}");
    }
    for bad in ["PT", "P", "0000:01:00.00", "PT-5S", "PT1.234S"] {
        assert_eq!(lms.set_value("cmi.session_time", bad), "false", "bad {bad}");
        assert_eq!(lms.get_last_error(), "406");
    }
    assert_eq!(lms.violations().len(), 5);
}

#[test]
fn a2_6_suspend_data_and_location_limits() {
    let mut fourth = StrictScorm2004::new();
    assert_eq!(fourth.initialize(""), "true");
    assert_eq!(
        fourth.set_value("cmi.suspend_data", &"a".repeat(64_000)),
        "true"
    );
    assert_eq!(
        fourth.set_value("cmi.suspend_data", &"a".repeat(64_001)),
        "false"
    );
    assert_eq!(fourth.get_last_error(), "406");
    assert_eq!(fourth.set_value("cmi.location", &"l".repeat(1_000)), "true");
    assert_eq!(
        fourth.set_value("cmi.location", &"l".repeat(1_001)),
        "false"
    );
    assert_eq!(fourth.get_last_error(), "406");

    let mut third = StrictScorm2004::with_config(Scorm2004Config {
        edition: Scorm2004Edition::Third,
        ..Scorm2004Config::default()
    });
    assert_eq!(third.initialize(""), "true");
    assert_eq!(
        third.set_value("cmi.suspend_data", &"a".repeat(4_000)),
        "true"
    );
    assert_eq!(
        third.set_value("cmi.suspend_data", &"a".repeat(4_001)),
        "false"
    );
    assert_eq!(third.get_last_error(), "406");
}

#[test]
fn a2_7_interactions_dependency_and_per_type_formats() {
    let mut lms = StrictScorm2004::new();
    assert_eq!(lms.initialize(""), "true");
    check_interaction_tf_2004(&mut lms);
    check_interaction_other_types_2004(&mut lms);
}

fn check_interaction_tf_2004(lms: &mut StrictScorm2004) {
    // 0: true-false
    assert_eq!(lms.set_value("cmi.interactions.0.id", "q_tf"), "true");
    assert_eq!(
        lms.set_value("cmi.interactions.0.learner_response", "true"),
        "false"
    );
    assert_eq!(lms.get_last_error(), "408");
    assert_eq!(
        lms.set_value("cmi.interactions.0.type", "true-false"),
        "true"
    );
    assert_eq!(
        lms.set_value("cmi.interactions.0.correct_responses.0.pattern", "true"),
        "true"
    );
    assert_eq!(
        lms.set_value("cmi.interactions.0.correct_responses.1.pattern", "false"),
        "false"
    );
    assert_eq!(lms.get_last_error(), "351");
    assert_eq!(
        lms.set_value("cmi.interactions.0.learner_response", "t"),
        "false"
    );
    assert_eq!(lms.get_last_error(), "406");
    assert_eq!(
        lms.set_value("cmi.interactions.0.learner_response", "true"),
        "true"
    );
    assert_eq!(lms.set_value("cmi.interactions.0.result", "wrong"), "false");
    assert_eq!(lms.get_last_error(), "406");
    assert_eq!(
        lms.set_value("cmi.interactions.0.result", "incorrect"),
        "true"
    );
}

fn check_interaction_other_types_2004(lms: &mut StrictScorm2004) {
    // 1: choice
    assert_eq!(lms.set_value("cmi.interactions.1.id", "q_mc"), "true");
    assert_eq!(lms.set_value("cmi.interactions.1.type", "choice"), "true");
    assert_eq!(
        lms.set_value("cmi.interactions.1.learner_response", "a[,]a"),
        "false"
    );
    assert_eq!(lms.get_last_error(), "406");
    assert_eq!(
        lms.set_value("cmi.interactions.1.learner_response", "a[,]b"),
        "true"
    );

    // 2: matching
    assert_eq!(lms.set_value("cmi.interactions.2.id", "q_match"), "true");
    assert_eq!(lms.set_value("cmi.interactions.2.type", "matching"), "true");
    assert_eq!(
        lms.set_value("cmi.interactions.2.learner_response", "a[.]1[,]b[.]2"),
        "true"
    );

    // 3: fill-in
    assert_eq!(lms.set_value("cmi.interactions.3.id", "q_fill"), "true");
    assert_eq!(lms.set_value("cmi.interactions.3.type", "fill-in"), "true");
    assert_eq!(
        lms.set_value(
            "cmi.interactions.3.correct_responses.0.pattern",
            "{case_matters=true}Rust"
        ),
        "true"
    );

    // 4: other
    assert_eq!(lms.set_value("cmi.interactions.4.id", "q_free"), "true");
    assert_eq!(lms.set_value("cmi.interactions.4.type", "other"), "true");
    assert_eq!(
        lms.set_value("cmi.interactions.4.learner_response", "custom"),
        "true"
    );

    // Skipped index
    assert_eq!(lms.set_value("cmi.interactions.9.id", "q_gap"), "false");
    assert_eq!(lms.get_last_error(), "351");
}

#[test]
fn a2_8_interactions_readback() {
    let mut lms = StrictScorm2004::new();
    assert_eq!(lms.initialize(""), "true");
    assert_eq!(lms.set_value("cmi.interactions.0.id", "q1"), "true");
    assert_eq!(lms.set_value("cmi.interactions.0.type", "choice"), "true");
    assert_eq!(
        lms.set_value("cmi.interactions.0.correct_responses.0.pattern", "a"),
        "true"
    );
    assert_eq!(
        lms.set_value("cmi.interactions.0.learner_response", "a"),
        "true"
    );
    assert_eq!(
        lms.set_value("cmi.interactions.0.result", "correct"),
        "true"
    );

    assert_eq!(lms.get_value("cmi.interactions.0.id"), "q1");
    assert_eq!(lms.get_value("cmi.interactions.0.type"), "choice");
    assert_eq!(lms.get_value("cmi.interactions.0.learner_response"), "a");
    assert_eq!(lms.get_value("cmi.interactions.0.result"), "correct");
    assert_eq!(
        lms.get_value("cmi.interactions.0.correct_responses._count"),
        "1"
    );
    assert_eq!(lms.get_last_error(), "0");
    assert!(lms.violations().is_empty());
}

#[test]
fn a2_9_and_a2_10_exit_and_nav_request() {
    let mut lms = StrictScorm2004::new();
    assert_eq!(lms.initialize(""), "true");

    for ok in ["", "time-out", "suspend", "logout", "normal"] {
        assert_eq!(lms.set_value("cmi.exit", ok), "true");
    }
    for bad in ["timeout", "finish"] {
        assert_eq!(lms.set_value("cmi.exit", bad), "false");
        assert_eq!(lms.get_last_error(), "406");
    }

    for ok in [
        "_none_",
        "continue",
        "previous",
        "exit",
        "exitAll",
        "abandon",
        "abandonAll",
        "suspendAll",
        "{target=item_1}choice",
        "{target=item_2}jump",
    ] {
        assert_eq!(lms.set_value("adl.nav.request", ok), "true", "nav {ok}");
    }
    for bad in ["next", "{target=}choice"] {
        assert_eq!(
            lms.set_value("adl.nav.request", bad),
            "false",
            "bad nav {bad}"
        );
        assert_eq!(lms.get_last_error(), "406");
    }
}

#[test]
fn a2_11_description_limit() {
    let mut lms = StrictScorm2004::new();
    assert_eq!(lms.initialize(""), "true");
    assert_eq!(lms.set_value("cmi.interactions.0.id", "q1"), "true");
    assert_eq!(
        lms.set_value(
            "cmi.interactions.0.description",
            &format!("{{lang=en}}{}", "x".repeat(250))
        ),
        "true"
    );
    assert_eq!(
        lms.set_value("cmi.interactions.0.description", &"x".repeat(251)),
        "false"
    );
    assert_eq!(lms.get_last_error(), "406");
}

#[test]
fn a2_12_dispatch_parity() {
    let mut direct12 = StrictScorm12::new();
    let mut disp12 = StrictScorm12::new();
    assert_eq!(
        direct12.lms_initialize(""),
        disp12.dispatch("LMSInitialize", &[""])
    );
    assert_eq!(
        direct12.lms_set_value("cmi.core.lesson_status", "passed"),
        disp12.dispatch("LMSSetValue", &["cmi.core.lesson_status", "passed"])
    );
    assert_eq!(
        direct12.lms_set_value("cmi.core.session_time", "9999:99:99.99"),
        disp12.dispatch("LMSSetValue", &["cmi.core.session_time", "9999:99:99.99"])
    );
    assert_eq!(
        direct12.lms_get_last_error(),
        disp12.dispatch("LMSGetLastError", &[])
    );
    assert_eq!(direct12.violations(), disp12.violations());

    let mut direct04 = StrictScorm2004::new();
    let mut disp04 = StrictScorm2004::new();
    assert_eq!(
        direct04.initialize(""),
        disp04.dispatch("Initialize", &[""])
    );
    assert_eq!(
        direct04.set_value("cmi.score.scaled", "1.5"),
        disp04.dispatch("SetValue", &["cmi.score.scaled", "1.5"])
    );
    assert_eq!(
        direct04.get_last_error(),
        disp04.dispatch("GetLastError", &[])
    );
    assert_eq!(direct04.violations(), disp04.violations());
}
