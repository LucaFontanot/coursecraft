use crate::{Course, Severity};

fn course_with_master_object(obj_json: serde_json::Value) -> Course {
    let raw = serde_json::json!({
        "schema_version": 1,
        "id": "c-m29-validate",
        "title": "M29 Validate Test",
        "slide_size": { "w": 1280, "h": 720 },
        "masters": [{
            "id": "m1",
            "title": "Title Master",
            "objects": [obj_json]
        }],
        "scenes": [{
            "id": "scene-1",
            "title": "Scene 1",
            "slides": [{
                "id": "s1",
                "title": "Slide 1",
                "objects": []
            }]
        }]
    });
    serde_json::from_value(raw).expect("valid course")
}

fn course_with_slide_object(obj_json: serde_json::Value) -> Course {
    let raw = serde_json::json!({
        "schema_version": 1,
        "id": "c-m29-validate",
        "title": "M29 Validate Test",
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
fn master_object_with_states_produces_validation_warning_naming_master() {
    let course = course_with_master_object(serde_json::json!({
        "id": "btn-1",
        "name": "Master Button",
        "frame": { "x": 100, "y": 100, "w": 200, "h": 60 },
        "kind": { "type": "button", "label": "Next" },
        "states": [{ "name": "hover", "fill": "#FF0000" }]
    }));
    let issues = course.validate();
    assert!(
        issues.iter().all(|i| i.severity != Severity::Error),
        "existing courses with states on masters must still open (warning only): {issues:?}"
    );
    assert!(
        issues.iter().any(|i| {
            i.severity == Severity::Warning
                && i.message.contains("Title Master")
                && i.path.contains("/masters/0/objects/0/states")
        }),
        "expected warning naming the master: {issues:?}"
    );
}

#[test]
fn hidden_state_def_produces_validation_warning() {
    let course = course_with_slide_object(serde_json::json!({
        "id": "btn-1",
        "name": "Button 1",
        "frame": { "x": 100, "y": 100, "w": 200, "h": 60 },
        "kind": { "type": "button", "label": "Next" },
        "states": [{ "name": "hidden", "fill": "#FF0000" }]
    }));
    let issues = course.validate();
    assert!(
        issues.iter().all(|i| i.severity != Severity::Error),
        "hidden StateDef must warn, not error: {issues:?}"
    );
    assert!(
        issues
            .iter()
            .any(|i| { i.severity == Severity::Warning && i.path.ends_with("/states/0") }),
        "expected warning on /states/0 for hidden StateDef: {issues:?}"
    );
}

#[test]
fn more_than_32_states_produces_validation_warning() {
    let states: Vec<serde_json::Value> = (0..33)
        .map(|i| serde_json::json!({ "name": format!("s{i}") }))
        .collect();
    let course = course_with_slide_object(serde_json::json!({
        "id": "btn-1",
        "name": "Button 1",
        "frame": { "x": 100, "y": 100, "w": 200, "h": 60 },
        "kind": { "type": "button", "label": "Next" },
        "states": states
    }));
    let issues = course.validate();
    assert!(
        issues.iter().all(|i| i.severity != Severity::Error),
        "> 32 states must warn, not error: {issues:?}"
    );
    assert!(
        issues
            .iter()
            .any(|i| i.severity == Severity::Warning && i.path.ends_with("/states")),
        "expected warning on /states when > 32 states: {issues:?}"
    );
}

#[test]
fn state_name_over_64_chars_produces_validation_warning() {
    let long_name = "a".repeat(65);
    let course = course_with_slide_object(serde_json::json!({
        "id": "btn-1",
        "name": "Button 1",
        "frame": { "x": 100, "y": 100, "w": 200, "h": 60 },
        "kind": { "type": "button", "label": "Next" },
        "states": [{ "name": long_name }]
    }));
    let issues = course.validate();
    assert!(
        issues.iter().all(|i| i.severity != Severity::Error),
        "> 64 char state name must warn, not error: {issues:?}"
    );
    assert!(
        issues
            .iter()
            .any(|i| i.severity == Severity::Warning && i.path.ends_with("/states/0")),
        "expected warning on /states/0 for > 64 char state name: {issues:?}"
    );
}
