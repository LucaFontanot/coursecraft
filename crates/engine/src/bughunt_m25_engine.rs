#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::Session;
use coursecraft_course::{NextRequirement, SlidePlayerOverrides, TextRun};
use serde_json::json;

#[test]
fn slide_paste_from_other_course_drops_missing_slide_link() {
    let source_session = Session::blank("Course A");
    let s1 = source_session.course.scenes[0].slides[0].id.clone();

    let mut slide = source_session.course.slide(&s1).unwrap().clone();
    let mut obj: coursecraft_course::Object = serde_json::from_value(json!({
        "id": "txt-link",
        "name": "Link Box",
        "kind": {
            "type": "text",
            "text": "Go to details"
        },
        "frame": { "x": 40.0, "y": 40.0, "w": 300.0, "h": 60.0 }
    }))
    .unwrap();
    if let coursecraft_course::ObjectKind::Text { runs, .. } = &mut obj.kind {
        let run: TextRun = serde_json::from_value(json!({
            "text": "Go to details",
            "link": { "type": "slide", "slide": "s-only-in-course-a" }
        }))
        .unwrap();
        runs.push(run);
    }
    slide.objects.push(obj);

    let mut target_session = Session::blank("Course B");
    let res = target_session.execute("slide.paste", &json!({ "slide": slide }));
    assert!(
        res.is_ok(),
        "pasting a slide (or inserting a library template) with a Link::Slide to a slide not in the target course should strip the dead link and succeed, got: {res:?}"
    );
}

#[test]
fn slide_paste_from_other_course_clears_missing_next_requires_variable() {
    let source_session = Session::blank("Course A");
    let mut slide = source_session.course.scenes[0].slides[0].clone();
    slide.player = Some(SlidePlayerOverrides {
        next_requires: Some(NextRequirement::Variable {
            name: "course_a_flag".to_owned(),
        }),
        ..SlidePlayerOverrides::default()
    });

    let mut target_session = Session::blank("Course B");
    let res = target_session.execute("slide.paste", &json!({ "slide": slide }));
    assert!(
        res.is_ok(),
        "pasting a slide (or inserting a library template) with next_requires referencing a missing variable should clear it and succeed, got: {res:?}"
    );
}
