//! Headless tests for Publish dialog memory and Open in browser button.

use crate::EditorRequest;
use crate::publish::PublishStandard;
use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use egui::{Event, PointerButton, Pos2};

fn hello() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    Course::from_json(&json).expect("valid course")
}

fn click_for_requests(harness: &mut TestHarness, pos: Pos2) -> Vec<EditorRequest> {
    harness.frame(vec![Event::PointerMoved(pos)]);
    harness.frame(vec![Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Default::default(),
    }]);
    harness.frame(vec![Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Default::default(),
    }])
}

#[test]
fn publish_dialog_remembers_last_standard_across_save_and_reopen() {
    let mut harness = TestHarness::new(hello());
    harness.editor.set_publish_available(true);
    harness.idle_frame();

    harness.editor.open_publish_dialog();
    harness.idle_frame();
    assert_eq!(harness.editor.ui.publish.standard, PublishStandard::Scorm12);

    let web_radio = harness
        .find_text("Web (any website, no LMS)")
        .expect("Web radio button");
    harness.click_at(web_radio);
    harness.idle_frame();
    assert_eq!(harness.editor.ui.publish.standard, PublishStandard::Web);

    let publish_btn = harness.find_text_last("Publish").expect("Publish button");
    let requests = click_for_requests(&mut harness, publish_btn);
    assert!(
        requests.iter().any(|r| matches!(
            r,
            EditorRequest::Publish {
                standard: PublishStandard::Web
            }
        )),
        "clicking Publish emits Publish request for Web"
    );

    let saved_json = harness
        .editor
        .session
        .course
        .to_json()
        .expect("serialize course");
    let reloaded = Course::from_json(&saved_json).expect("reload course");
    assert_eq!(
        reloaded
            .editor
            .last_publish
            .as_ref()
            .map(|lp| lp.standard.as_str()),
        Some("web")
    );

    let mut reopened = TestHarness::new(reloaded);
    reopened.editor.set_publish_available(true);
    reopened.idle_frame();
    reopened.editor.open_publish_dialog();
    reopened.idle_frame();
    assert_eq!(reopened.editor.ui.publish.standard, PublishStandard::Web);
}

#[test]
fn publish_success_shows_open_in_browser_button_with_lms_preview_label() {
    let mut harness = TestHarness::new(hello());
    harness.editor.set_publish_available(true);
    harness.editor.open_publish_dialog();
    harness.editor.ui.publish.standard = PublishStandard::Scorm12;
    harness
        .editor
        .set_publish_success("course-scorm12.zip", 1024);
    harness.idle_frame();

    let preview_btn = harness
        .find_text("Open in browser (preview, no LMS)")
        .expect("preview button for SCORM");
    let requests = click_for_requests(&mut harness, preview_btn);
    assert!(
        requests
            .iter()
            .any(|r| matches!(r, EditorRequest::OpenPublishedInBrowser)),
        "clicking preview button emits OpenPublishedInBrowser"
    );

    harness.editor.ui.publish.standard = PublishStandard::Web;
    harness.idle_frame();
    assert!(
        harness.find_text("Open in browser").is_some(),
        "Web publish shows plain Open in browser label"
    );
}
