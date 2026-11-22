use crate::{Course, Severity};

fn course_with_object(obj_json: serde_json::Value) -> Course {
    let raw = serde_json::json!({
        "schema_version": 1,
        "id": "c-u19-validate",
        "title": "U19 Validate Test",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "scene-1",
            "title": "Scene 1",
            "slides": [{
                "id": "s1",
                "title": "Slide 1",
                "objects": [obj_json]
            }]
        }]
    });
    serde_json::from_value(raw).expect("valid course")
}

#[test]
fn state_src_with_quoted_windows_path_warns() {
    let course = course_with_object(serde_json::json!({
        "id": "img-1",
        "name": "Picture 1",
        "frame": { "x": 100, "y": 100, "w": 200, "h": 200 },
        "kind": { "type": "image", "src": "base.png" },
        "states": [
            { "name": "hover", "src": "\"C:\\Users\\me\\Picture48.png\"" }
        ]
    }));
    let issues = course.validate();
    assert!(
        issues.iter().all(|i| i.severity != Severity::Error),
        "unsafe state src must warn, not error: {issues:?}"
    );
    assert!(
        issues.iter().any(|i| i.severity == Severity::Warning
            && i.message.contains("Picture 1")
            && i.message.contains("hover")
            && i.message.contains("outside the course")),
        "expected warning naming state and object: {issues:?}"
    );
}

#[test]
fn state_src_with_parent_traversal_warns() {
    let course = course_with_object(serde_json::json!({
        "id": "img-1",
        "name": "Picture 1",
        "frame": { "x": 100, "y": 100, "w": 200, "h": 200 },
        "kind": { "type": "image", "src": "base.png" },
        "states": [
            { "name": "hover", "src": "../escape.png" }
        ]
    }));
    let issues = course.validate();
    assert!(issues.iter().all(|i| i.severity != Severity::Error));
    assert!(
        issues
            .iter()
            .any(|i| i.severity == Severity::Warning && i.path.ends_with("/states/0/src")),
        "expected warning on /states/0/src: {issues:?}"
    );
}

#[test]
fn state_src_with_absolute_path_warns() {
    let course = course_with_object(serde_json::json!({
        "id": "img-1",
        "name": "Picture 1",
        "frame": { "x": 100, "y": 100, "w": 200, "h": 200 },
        "kind": { "type": "image", "src": "base.png" },
        "states": [
            { "name": "hover", "src": "/abs.png" }
        ]
    }));
    let issues = course.validate();
    assert!(issues.iter().all(|i| i.severity != Severity::Error));
    assert!(
        issues
            .iter()
            .any(|i| i.severity == Severity::Warning && i.path.ends_with("/states/0/src")),
        "expected warning on /states/0/src: {issues:?}"
    );
}

#[test]
fn state_src_with_safe_relative_path_does_not_warn() {
    let course = course_with_object(serde_json::json!({
        "id": "img-1",
        "name": "Picture 1",
        "frame": { "x": 100, "y": 100, "w": 200, "h": 200 },
        "kind": { "type": "image", "src": "base.png" },
        "states": [
            { "name": "hover", "src": "media/pic.png" }
        ]
    }));
    let issues = course.validate();
    assert!(
        !issues.iter().any(|i| i.path.contains("/states/")),
        "safe relative path must not warn or error on state: {issues:?}"
    );
}

#[test]
fn state_src_on_non_image_object_warns() {
    let course = course_with_object(serde_json::json!({
        "id": "txt-1",
        "name": "Text 1",
        "frame": { "x": 100, "y": 100, "w": 200, "h": 80 },
        "kind": { "type": "text", "text": "Hello" },
        "states": [
            { "name": "hover", "src": "pic.png" }
        ]
    }));
    let issues = course.validate();
    assert!(issues.iter().all(|i| i.severity != Severity::Error));
    assert!(
        issues.iter().any(|i| i.severity == Severity::Warning
            && i.path.ends_with("/states/0/src")
            && i.message.contains("Text 1")),
        "expected warning for state src on non-image object: {issues:?}"
    );
}
