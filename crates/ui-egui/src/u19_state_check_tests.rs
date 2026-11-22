use crate::test_harness::TestHarness;
use coursecraft_course::Course;

fn course_with_missing_state_image() -> Course {
    let raw = serde_json::json!({
        "schema_version": 1,
        "id": "c-u19-ui",
        "title": "U19 UI Test",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "scene1",
            "title": "Main",
            "slides": [{
                "id": "s1",
                "title": "Slide 1",
                "objects": [{
                    "id": "img1",
                    "name": "Picture 1",
                    "frame": { "x": 100.0, "y": 100.0, "w": 200.0, "h": 200.0 },
                    "kind": { "type": "image", "src": "base.png" },
                    "states": [
                        { "name": "hover", "src": "missing-hover.png" }
                    ]
                }]
            }]
        }]
    });
    serde_json::from_value(raw).expect("valid course")
}

#[test]
fn missing_state_image_appears_in_check_course_issues() {
    let mut h = TestHarness::new(course_with_missing_state_image());
    h.editor.set_media(vec![(
        "base.png".to_owned(),
        std::sync::Arc::from(b"png".as_slice()),
    )]);
    h.editor.refresh_validate_only();
    assert!(
        h.editor.ui.validation_issues.iter().any(|issue| {
            issue.severity == "warning"
                && issue.message.contains("Picture 1")
                && issue.message.contains("hover")
                && issue.message.contains("missing-hover.png")
        }),
        "expected missing state image warning in validation_issues, got {:?}",
        h.editor.ui.validation_issues
    );
}

#[test]
fn eye_preview_of_missing_state_image_shows_placeholder() {
    let mut h = TestHarness::new(course_with_missing_state_image());
    h.editor.set_media(vec![(
        "base.png".to_owned(),
        std::sync::Arc::from(b"png".as_slice()),
    )]);
    h.editor.selection.object = Some("img1".to_owned());
    h.idle_frame();
    h.editor.ui.preview_state = Some("hover".to_owned());
    assert!(
        h.find_text("No image yet").is_some(),
        "expected 'No image yet' placeholder when eye-previewing a missing state image"
    );
}
