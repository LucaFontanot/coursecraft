use crate::{Course, LmsStatus, PublishSettings};

fn hello_json() -> &'static str {
    include_str!("../../../examples/hello-course/course.json")
}

#[test]
fn publish_settings_defaults_lms_status_to_auto_and_omits_from_json() {
    let settings: PublishSettings =
        serde_json::from_str(r#"{"standard":"scorm12"}"#).expect("deserialize");
    assert_eq!(settings.lms_status, LmsStatus::Auto);

    let json = serde_json::to_string(&settings).expect("serialize");
    assert!(
        !json.contains("lms_status"),
        "default Auto must be omitted from JSON: {json}"
    );
}

#[test]
fn publish_settings_round_trips_every_lms_status_variant() {
    let cases = [
        (LmsStatus::Auto, "auto"),
        (LmsStatus::PassedIncomplete, "passed_incomplete"),
        (LmsStatus::PassedFailed, "passed_failed"),
        (LmsStatus::CompletedIncomplete, "completed_incomplete"),
        (LmsStatus::CompletedFailed, "completed_failed"),
    ];

    for (variant, expected_name) in cases {
        let settings = PublishSettings {
            standard: "scorm12".to_string(),
            lms_status: variant,
        };
        let json = serde_json::to_string(&settings).expect("serialize");
        if variant == LmsStatus::Auto {
            assert!(!json.contains("lms_status"));
        } else {
            assert!(
                json.contains(&format!(r#""lms_status":"{expected_name}""#)),
                "expected {expected_name} in {json}"
            );
        }
        let decoded: PublishSettings = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded, settings);
    }
}

#[test]
fn old_course_json_without_lms_status_round_trips_unchanged() {
    let original = hello_json();
    let course = Course::from_json(original).expect("parse hello course");
    let reserialized = course.to_json().expect("serialize hello course");
    let reparsed = Course::from_json(&reserialized).expect("reparse hello course");
    assert_eq!(course, reparsed);
    assert!(
        !reserialized.contains("lms_status"),
        "hello-course without lms_status must not gain lms_status: {reserialized}"
    );
}
