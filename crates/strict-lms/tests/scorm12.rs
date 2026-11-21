//! Unit tests for `StrictScorm12` (M27 Cases A1.1–A1.12).

use coursecraft_strict_lms::{Scorm12Config, StrictScorm12};

#[test]
fn a1_1_call_order_violations() {
    let mut lms = StrictScorm12::new();

    assert_eq!(lms.lms_get_value("cmi.core.student_name"), "");
    assert_eq!(lms.lms_get_last_error(), "301");
    assert_eq!(
        lms.lms_set_value("cmi.core.lesson_status", "incomplete"),
        "false"
    );
    assert_eq!(lms.lms_get_last_error(), "301");
    assert_eq!(lms.lms_commit(""), "false");
    assert_eq!(lms.lms_get_last_error(), "301");
    assert_eq!(lms.lms_finish(""), "false");
    assert_eq!(lms.lms_get_last_error(), "301");

    assert_eq!(lms.lms_initialize(""), "true");
    assert_eq!(lms.lms_get_last_error(), "0");
    assert_eq!(lms.lms_initialize(""), "false");
    assert_eq!(lms.lms_get_last_error(), "101");

    assert_eq!(lms.lms_finish(""), "true");
    assert_eq!(
        lms.lms_set_value("cmi.core.lesson_status", "completed"),
        "false"
    );
    assert_eq!(lms.lms_get_last_error(), "301");
    assert_eq!(lms.lms_commit(""), "false");
    assert_eq!(lms.lms_get_last_error(), "301");

    assert_eq!(lms.violations().len(), 7);
    assert!(lms.assert_clean_finish());

    let mut unclean = StrictScorm12::new();
    assert_eq!(unclean.lms_initialize(""), "true");
    assert!(!unclean.assert_clean_finish());
    assert_eq!(unclean.violations().len(), 1);
    assert_eq!(unclean.violations()[0].rule, "SCORM12-UNCLEAN-FINISH");
}

#[test]
fn a1_2_non_empty_lifecycle_args() {
    let mut lms = StrictScorm12::new();
    assert_eq!(lms.lms_initialize("x"), "false");
    assert_eq!(lms.lms_get_last_error(), "201");

    assert_eq!(lms.lms_initialize(""), "true");
    assert_eq!(lms.lms_commit("x"), "false");
    assert_eq!(lms.lms_get_last_error(), "201");
    assert_eq!(lms.lms_finish("x"), "false");
    assert_eq!(lms.lms_get_last_error(), "201");

    assert_eq!(lms.violations().len(), 3);
    assert!(lms.violations().iter().all(|v| v.rule == "SCORM12-201-ARG"));
}

#[test]
fn a1_3_access_and_unknown_elements() {
    let mut lms = StrictScorm12::new();
    assert_eq!(lms.lms_initialize(""), "true");

    assert_eq!(lms.lms_set_value("cmi.core.student_name", "Bob"), "false");
    assert_eq!(lms.lms_get_last_error(), "403");

    assert_eq!(lms.lms_set_value("cmi.interactions._count", "1"), "false");
    assert_eq!(lms.lms_get_last_error(), "402");

    assert_eq!(lms.lms_set_value("cmi.unknown.foo", "bar"), "false");
    assert_eq!(lms.lms_get_last_error(), "401");

    assert_eq!(lms.lms_get_value("cmi.core.exit"), "");
    assert_eq!(lms.lms_get_last_error(), "404");

    assert_eq!(lms.lms_get_value("cmi.core.session_time"), "");
    assert_eq!(lms.lms_get_last_error(), "404");

    assert_eq!(lms.lms_get_value("cmi.interactions.0.result"), "");
    assert_eq!(lms.lms_get_last_error(), "404");

    assert_eq!(lms.violations().len(), 6);
    for v in lms.violations() {
        assert!(!matches!(
            v.error_code.as_deref(),
            Some("406" | "407" | "408")
        ));
    }
}

#[test]
fn a1_4_lesson_status_and_exit_vocab() {
    let mut lms = StrictScorm12::new();
    assert_eq!(lms.lms_initialize(""), "true");

    for ok in ["passed", "completed", "failed", "incomplete", "browsed"] {
        assert_eq!(
            lms.lms_set_value("cmi.core.lesson_status", ok),
            "true",
            "status {ok}"
        );
    }
    for bad in ["not attempted", "unknown", "PASSED", ""] {
        assert_eq!(
            lms.lms_set_value("cmi.core.lesson_status", bad),
            "false",
            "bad {bad}"
        );
        assert_eq!(lms.lms_get_last_error(), "405");
    }

    for ok in ["", "suspend", "logout", "time-out"] {
        assert_eq!(lms.lms_set_value("cmi.core.exit", ok), "true", "exit {ok}");
    }
    for bad in ["normal", "timeout"] {
        assert_eq!(
            lms.lms_set_value("cmi.core.exit", bad),
            "false",
            "bad exit {bad}"
        );
        assert_eq!(lms.lms_get_last_error(), "405");
    }
    assert_eq!(lms.violations().len(), 6);
}

#[test]
fn a1_5_cmi_timespan_validation() {
    let mut lms = StrictScorm12::new();
    assert_eq!(lms.lms_initialize(""), "true");

    for ok in [
        "0000:00:00.00",
        "00:00:59.99",
        "01:01:01.05",
        "9999:59:59.99",
    ] {
        assert_eq!(
            lms.lms_set_value("cmi.core.session_time", ok),
            "true",
            "timespan {ok}"
        );
    }
    for bad in [
        "9999:99:99.99",
        "PT5S",
        "00:60:00.00",
        "00:00:60.00",
        "1:00:00",
    ] {
        assert_eq!(
            lms.lms_set_value("cmi.core.session_time", bad),
            "false",
            "bad {bad}"
        );
        assert_eq!(lms.lms_get_last_error(), "405");
    }
    assert_eq!(lms.violations().len(), 5);
}

#[test]
fn a1_6_lesson_location_255_boundary() {
    let mut lms = StrictScorm12::new();
    assert_eq!(lms.lms_initialize(""), "true");

    let ascii_255 = "a".repeat(255);
    assert_eq!(
        lms.lms_set_value("cmi.core.lesson_location", &ascii_255),
        "true"
    );

    let utf8_255 = "é".repeat(255);
    assert_eq!(
        lms.lms_set_value("cmi.core.lesson_location", &utf8_255),
        "true"
    );

    let ascii_256 = "a".repeat(256);
    assert_eq!(
        lms.lms_set_value("cmi.core.lesson_location", &ascii_256),
        "false"
    );
    assert_eq!(lms.lms_get_last_error(), "405");
    assert_eq!(lms.violations().len(), 1);
}

#[test]
fn a1_7_suspend_data_4096_boundary() {
    let mut lms = StrictScorm12::new();
    assert_eq!(lms.lms_initialize(""), "true");

    let ok = "x".repeat(4096);
    assert_eq!(lms.lms_set_value("cmi.suspend_data", &ok), "true");
    assert!(lms.violations().is_empty());

    let too_long = "x".repeat(4097);
    assert_eq!(lms.lms_set_value("cmi.suspend_data", &too_long), "false");
    assert_eq!(lms.lms_get_last_error(), "405");
    assert_eq!(lms.violations().len(), 1);
}

#[test]
fn a1_8_score_cmi_decimal() {
    let mut lms = StrictScorm12::new();
    assert_eq!(lms.lms_initialize(""), "true");

    for ok in ["", "0", "85", "85.5", "100"] {
        assert_eq!(
            lms.lms_set_value("cmi.core.score.raw", ok),
            "true",
            "score {ok}"
        );
    }
    for bad in ["-1", "100.1", "1e2", "NaN", "abc"] {
        assert_eq!(
            lms.lms_set_value("cmi.core.score.raw", bad),
            "false",
            "bad {bad}"
        );
        assert_eq!(lms.lms_get_last_error(), "405");
    }
    assert_eq!(lms.violations().len(), 5);
}

#[test]
fn a1_9_interactions_types_and_results() {
    let mut lms = StrictScorm12::new();
    assert_eq!(lms.lms_initialize(""), "true");

    let legal_types = [
        "true-false",
        "choice",
        "fill-in",
        "matching",
        "performance",
        "sequencing",
        "likert",
        "numeric",
    ];
    for (idx, kind) in legal_types.iter().enumerate() {
        assert_eq!(
            lms.lms_set_value(&format!("cmi.interactions.{idx}.id"), &format!("q_{idx}")),
            "true"
        );
        assert_eq!(
            lms.lms_set_value(&format!("cmi.interactions.{idx}.type"), kind),
            "true"
        );
        let resp = if *kind == "true-false" { "t" } else { "a" };
        assert_eq!(
            lms.lms_set_value(&format!("cmi.interactions.{idx}.student_response"), resp),
            "true"
        );
        assert_eq!(
            lms.lms_set_value(&format!("cmi.interactions.{idx}.result"), "correct"),
            "true"
        );
    }
    assert_eq!(lms.lms_get_value("cmi.interactions._count"), "8");
    assert!(lms.violations().is_empty());

    // "other" is illegal in SCORM 1.2
    assert_eq!(lms.lms_set_value("cmi.interactions.8.id", "q_8"), "true");
    assert_eq!(
        lms.lms_set_value("cmi.interactions.8.type", "other"),
        "false"
    );
    assert_eq!(lms.lms_get_last_error(), "405");

    // "incorrect" is illegal in SCORM 1.2 (must be "wrong")
    assert_eq!(
        lms.lms_set_value("cmi.interactions.8.result", "incorrect"),
        "false"
    );
    assert_eq!(lms.lms_get_last_error(), "405");

    // "true"/"false" rejected for true-false in strict SCORM 1.2
    assert_eq!(
        lms.lms_set_value("cmi.interactions.0.student_response", "true"),
        "false"
    );
    assert_eq!(lms.lms_get_last_error(), "405");

    // .description does not exist in SCORM 1.2
    assert_eq!(
        lms.lms_set_value("cmi.interactions.0.description", "Prompt"),
        "false"
    );
    assert_eq!(lms.lms_get_last_error(), "401");
}

#[test]
fn a1_10_interaction_gap_and_invalid_id() {
    let mut lms = StrictScorm12::new();
    assert_eq!(lms.lms_initialize(""), "true");

    // Gap rejected
    assert_eq!(lms.lms_set_value("cmi.interactions.2.id", "q2"), "false");
    assert_eq!(lms.lms_get_last_error(), "201");
    assert_eq!(lms.lms_get_value("cmi.interactions._count"), "0");

    // Invalid .id (>255 chars or whitespace) does NOT increment _count
    let too_long = "q".repeat(256);
    assert_eq!(
        lms.lms_set_value("cmi.interactions.0.id", &too_long),
        "false"
    );
    assert_eq!(lms.lms_get_last_error(), "405");
    assert_eq!(lms.lms_get_value("cmi.interactions._count"), "0");

    assert_eq!(lms.lms_set_value("cmi.interactions.0.id", "q 0"), "false");
    assert_eq!(lms.lms_get_last_error(), "405");
    assert_eq!(lms.lms_get_value("cmi.interactions._count"), "0");
}

#[test]
fn a1_11_total_time_and_relaunch_semantics() {
    let mut lms = StrictScorm12::new();
    assert_eq!(lms.lms_initialize(""), "true");
    assert_eq!(lms.lms_get_value("cmi.core.entry"), "ab-initio");
    assert_eq!(
        lms.lms_set_value("cmi.core.session_time", "0000:01:30.50"),
        "true"
    );
    assert_eq!(lms.lms_set_value("cmi.core.exit", "suspend"), "true");
    assert_eq!(lms.lms_set_value("cmi.suspend_data", "cc1:saved"), "true");
    assert_eq!(lms.lms_finish(""), "true");

    lms.relaunch();
    assert_eq!(lms.lms_initialize(""), "true");
    assert_eq!(lms.lms_get_value("cmi.core.entry"), "resume");
    assert_eq!(lms.lms_get_value("cmi.core.total_time"), "0000:01:30.50");
    assert_eq!(lms.lms_get_value("cmi.suspend_data"), "cc1:saved");
    assert_eq!(lms.lms_set_value("cmi.core.exit", ""), "true");
    assert_eq!(lms.lms_finish(""), "true");

    lms.relaunch();
    assert_eq!(lms.lms_initialize(""), "true");
    assert_eq!(lms.lms_get_value("cmi.core.entry"), "ab-initio");
    assert_eq!(lms.lms_get_value("cmi.suspend_data"), "");
    assert!(lms.violations().is_empty());
}

#[test]
fn a1_12_empty_initial_status_and_failed_init() {
    let mut lms = StrictScorm12::with_config(Scorm12Config {
        initial_lesson_status: String::new(),
        ..Scorm12Config::default()
    });
    assert_eq!(lms.lms_initialize(""), "true");
    assert_eq!(lms.lms_get_value("cmi.core.lesson_status"), "");

    let mut failing = StrictScorm12::with_config(Scorm12Config {
        fail_initialize: Some("101"),
        ..Scorm12Config::default()
    });
    assert_eq!(failing.lms_initialize(""), "false");
    assert_eq!(failing.lms_get_last_error(), "101");
    assert!(failing.violations().is_empty());
}
