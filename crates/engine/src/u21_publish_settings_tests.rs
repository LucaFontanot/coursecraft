use coursecraft_course::{Course, LmsStatus};
use serde_json::json;

use crate::Session;

fn hello_course() -> Course {
    let json = include_str!("../../../examples/hello-course/course.json");
    Course::from_json(json).expect("hello course")
}

#[test]
fn course_set_publish_settings_updates_lms_status_and_is_undoable() {
    let mut session = Session::new(hello_course());
    assert!(session.course.editor.last_publish.is_none());

    session
        .execute(
            "course.set_publish_settings",
            &json!({ "lms_status": "passed_failed" }),
        )
        .expect("set lms_status");

    let lp = session
        .course
        .editor
        .last_publish
        .as_ref()
        .expect("last_publish populated");
    assert_eq!(lp.lms_status, LmsStatus::PassedFailed);
    assert_eq!(lp.standard, "scorm12");

    session.undo().expect("undo set_publish_settings");
    assert!(session.course.editor.last_publish.is_none());

    session.redo().expect("redo set_publish_settings");
    assert_eq!(
        session
            .course
            .editor
            .last_publish
            .as_ref()
            .map(|p| p.lms_status),
        Some(LmsStatus::PassedFailed)
    );
}

#[test]
fn course_set_publish_settings_rejects_empty_params() {
    let mut session = Session::new(hello_course());
    let err = session
        .execute("course.set_publish_settings", &json!({}))
        .expect_err("empty params must fail");
    assert!(!err.to_string().is_empty());
}
