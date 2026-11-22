use crate::publish::PublishStandard;
use crate::test_harness::TestHarness;
use coursecraft_course::{Course, LmsStatus};

fn hello() -> Course {
    let json = include_str!("../../../examples/hello-course/course.json");
    Course::from_json(json).expect("hello course")
}

#[test]
fn report_status_combo_appears_only_for_scorm12_and_has_tooltip() {
    let mut harness = TestHarness::new(hello());
    harness.editor.set_publish_available(true);
    harness.editor.open_publish_dialog();
    harness.idle_frame();

    assert_eq!(harness.editor.ui.publish.standard, PublishStandard::Scorm12);
    assert!(
        harness.find_text("Report status as").is_some(),
        "Report status as label must appear when SCORM 1.2 is selected"
    );
    assert!(
        harness.find_text("Automatic").is_some(),
        "Automatic combo text must appear when SCORM 1.2 is selected"
    );

    let scorm2004_radio = harness
        .find_text("SCORM 2004 (4th edition)")
        .expect("SCORM 2004 radio");
    harness.click_at(scorm2004_radio);
    harness.idle_frame();
    assert_eq!(
        harness.editor.ui.publish.standard,
        PublishStandard::Scorm2004
    );
    assert!(
        harness.find_text("Report status as").is_none(),
        "Report status as must be hidden when SCORM 2004 is selected"
    );

    let web_radio = harness
        .find_text("Web (any website, no LMS)")
        .expect("Web radio");
    harness.click_at(web_radio);
    harness.idle_frame();
    assert!(
        harness.find_text("Report status as").is_none(),
        "Report status as must be hidden when Web is selected"
    );

    let scorm12_radio = harness.find_text("SCORM 1.2").expect("SCORM 1.2 radio");
    harness.click_at(scorm12_radio);
    harness.idle_frame();
    assert!(
        harness.find_text("Report status as").is_some(),
        "Report status as must reappear when SCORM 1.2 is reselected"
    );
}

#[test]
fn clicking_publish_preserves_chosen_lms_status() {
    let mut harness = TestHarness::new(hello());
    harness.editor.set_publish_available(true);
    harness.editor.run(
        "course.set_publish_settings",
        serde_json::json!({ "lms_status": "passed_failed" }),
    );
    harness.editor.open_publish_dialog();
    harness.idle_frame();

    let publish_btn = harness.find_text_last("Publish").expect("Publish button");
    harness.click_at(publish_btn);
    harness.idle_frame();

    assert_eq!(
        harness
            .editor
            .session
            .course
            .editor
            .last_publish
            .as_ref()
            .map(|p| p.lms_status),
        Some(LmsStatus::PassedFailed)
    );
}
