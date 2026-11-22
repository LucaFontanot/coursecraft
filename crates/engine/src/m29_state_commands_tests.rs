use serde_json::{Value as Json, json};

use crate::Session;

fn ok(s: &mut Session, cmd: &str, params: Json) -> Json {
    s.execute(cmd, &params)
        .unwrap_or_else(|e| panic!("{cmd} failed: {e}"))
}

fn add_button(s: &mut Session, id: &str) {
    ok(
        s,
        "object.add",
        json!({
            "slide": "slide1",
            "id": id,
            "kind": { "type": "button", "label": "Go" },
            "frame": { "x": 40, "y": 40, "w": 160, "h": 48 }
        }),
    );
}

#[test]
fn duplicate_state_clones_overrides_after_source_and_numbers_copies() {
    let mut s = Session::blank("Dup State");
    add_button(&mut s, "btn");
    ok(
        &mut s,
        "object.add_state",
        json!({
            "slide": "slide1",
            "object": "btn",
            "state": { "name": "hover", "fill": "#112233", "text": "Hovered" }
        }),
    );
    ok(
        &mut s,
        "object.add_state",
        json!({
            "slide": "slide1",
            "object": "btn",
            "state": { "name": "visited", "fill": "#445566" }
        }),
    );

    let r1 = ok(
        &mut s,
        "object.duplicate_state",
        json!({ "slide": "slide1", "object": "btn", "name": "hover" }),
    );
    assert_eq!(r1["name"], "hover copy");

    let r2 = ok(
        &mut s,
        "object.duplicate_state",
        json!({ "slide": "slide1", "object": "btn", "name": "hover" }),
    );
    assert_eq!(r2["name"], "hover copy 2");

    let (obj, _) = s.course.slide("slide1").unwrap().object("btn").unwrap();
    let names: Vec<&str> = obj.states.iter().map(|st| st.name.as_str()).collect();
    assert_eq!(
        names,
        vec!["hover", "hover copy 2", "hover copy", "visited"]
    );
    assert_eq!(obj.states[1].fill.as_ref().unwrap().0, "#112233");
    assert_eq!(obj.states[1].text.as_deref(), Some("Hovered"));
}

#[test]
fn duplicate_state_undo_and_redo_restore_exact_course() {
    let mut s = Session::blank("Dup Undo");
    add_button(&mut s, "btn");
    ok(
        &mut s,
        "object.add_state",
        json!({
            "slide": "slide1",
            "object": "btn",
            "state": { "name": "hover", "fill": "#112233" }
        }),
    );
    let before = s.course.clone();
    ok(
        &mut s,
        "object.duplicate_state",
        json!({ "slide": "slide1", "object": "btn", "name": "hover" }),
    );
    let after = s.course.clone();
    assert_ne!(before, after);

    s.undo().unwrap();
    assert_eq!(s.course, before);

    s.redo().unwrap();
    assert_eq!(s.course, after);
}

#[test]
fn duplicate_state_refuses_when_at_max_states() {
    let mut s = Session::blank("Max States");
    add_button(&mut s, "btn");
    for i in 0..coursecraft_course::MAX_STATES_PER_OBJECT {
        ok(
            &mut s,
            "object.add_state",
            json!({
                "slide": "slide1",
                "object": "btn",
                "state": { "name": format!("s{i}") }
            }),
        );
    }
    assert!(
        s.execute(
            "object.duplicate_state",
            &json!({ "slide": "slide1", "object": "btn", "name": "s0" })
        )
        .is_err()
    );
}

#[test]
fn rename_state_rewrites_initial_triggers_conditions_and_correct_when() {
    let mut s = Session::blank("Rename State");
    add_button(&mut s, "btn");
    ok(
        &mut s,
        "object.add_state",
        json!({
            "slide": "slide1",
            "object": "btn",
            "state": { "name": "active", "fill": "#224466" }
        }),
    );
    ok(
        &mut s,
        "object.update",
        json!({
            "slide": "slide1",
            "object": "btn",
            "initial_state": "active"
        }),
    );

    // Base trigger: Event::StateChanges + nested Condition::All([Not(ObjectState)])
    ok(
        &mut s,
        "trigger.add",
        json!({
            "slide": "slide1",
            "trigger": {
                "when": { "event": "state_changes", "object": "btn", "to": "active" },
                "action": { "action": "pause_timeline" },
                "condition": {
                    "all": [{
                        "not": {
                            "object_state": { "object": "btn", "state": "active", "is": false }
                        }
                    }]
                }
            }
        }),
    );

    // Layer trigger: Action::ChangeState
    ok(
        &mut s,
        "layer.add",
        json!({ "slide": "slide1", "id": "layer1", "title": "L1" }),
    );
    ok(
        &mut s,
        "trigger.add",
        json!({
            "slide": "slide1",
            "layer": "layer1",
            "trigger": {
                "when": { "event": "timeline_starts" },
                "action": { "action": "change_state", "object": "btn", "state": "active" }
            }
        }),
    );

    // Freeform question with correct_when referencing ObjectState
    ok(
        &mut s,
        "question.set",
        json!({
            "slide": "slide1",
            "question": {
                "kind": "freeform",
                "prompt": "Make btn active",
                "choices": [],
                "correct_when": { "object_state": { "object": "btn", "state": "active", "is": true } }
            }
        }),
    );

    let before = s.course.clone();
    ok(
        &mut s,
        "object.rename_state",
        json!({ "slide": "slide1", "object": "btn", "from": "active", "to": "chosen" }),
    );

    let issues = s.course.validate();
    assert!(
        issues
            .iter()
            .all(|i| i.severity != coursecraft_course::Severity::Error),
        "renamed course must validate cleanly: {issues:?}"
    );

    let sl = s.course.slide("slide1").unwrap();
    let (obj, _) = sl.object("btn").unwrap();
    assert_eq!(obj.states[0].name, "chosen");
    assert_eq!(obj.initial_state, "chosen");

    let after = s.course.clone();
    s.undo().unwrap();
    assert_eq!(s.course, before);

    s.redo().unwrap();
    assert_eq!(s.course, after);
}

#[test]
fn rename_state_refuses_reserved_empty_too_long_or_duplicate_names() {
    let mut s = Session::blank("Rename Refusals");
    add_button(&mut s, "btn");
    ok(
        &mut s,
        "object.add_state",
        json!({
            "slide": "slide1",
            "object": "btn",
            "state": { "name": "active", "fill": "#112233" }
        }),
    );
    ok(
        &mut s,
        "object.add_state",
        json!({
            "slide": "slide1",
            "object": "btn",
            "state": { "name": "hover", "fill": "#445566" }
        }),
    );
    let before = s.course.clone();
    let long = "x".repeat(coursecraft_course::MAX_STATE_NAME_CHARS + 1);
    for bad_to in ["normal", "hidden", "", "   ", long.as_str(), "hover"] {
        assert!(
            s.execute(
                "object.rename_state",
                &json!({ "slide": "slide1", "object": "btn", "from": "active", "to": bad_to })
            )
            .is_err(),
            "should reject renaming to {bad_to:?}"
        );
        assert_eq!(s.course, before);
    }
}

#[test]
fn add_state_refuses_reserved_empty_too_long_duplicate_master_or_over_limit() {
    let mut s = Session::blank("Add Refusals");
    add_button(&mut s, "btn");
    let long = "x".repeat(coursecraft_course::MAX_STATE_NAME_CHARS + 1);
    for bad_name in ["normal", "hidden", "", "   ", long.as_str()] {
        assert!(
            s.execute(
                "object.add_state",
                &json!({ "slide": "slide1", "object": "btn", "state": { "name": bad_name } })
            )
            .is_err(),
            "add_state should reject {bad_name:?}"
        );
    }

    // Master slide refusal
    let m_id = ok(&mut s, "master.add", json!({ "title": "M" }))["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let m_obj = ok(
        &mut s,
        "object.add",
        json!({
            "slide": m_id,
            "kind": { "type": "button", "label": "M" },
            "frame": { "x": 10, "y": 10, "w": 100, "h": 40 }
        }),
    )["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(
        s.execute(
            "object.add_state",
            &json!({ "slide": m_id, "object": m_obj, "state": { "name": "hover" } })
        )
        .is_err()
    );

    // Limit refusal (> 32)
    for i in 0..coursecraft_course::MAX_STATES_PER_OBJECT {
        ok(
            &mut s,
            "object.add_state",
            json!({
                "slide": "slide1",
                "object": "btn",
                "state": { "name": format!("s{i}") }
            }),
        );
    }
    assert!(
        s.execute(
            "object.add_state",
            &json!({ "slide": "slide1", "object": "btn", "state": { "name": "s_extra" } })
        )
        .is_err()
    );
}

#[test]
fn reorder_state_moves_state_and_undoes_redoes() {
    let mut s = Session::blank("Reorder State");
    add_button(&mut s, "btn");
    for name in ["a", "b", "c"] {
        ok(
            &mut s,
            "object.add_state",
            json!({ "slide": "slide1", "object": "btn", "state": { "name": name } }),
        );
    }
    let before = s.course.clone();
    ok(
        &mut s,
        "object.reorder_state",
        json!({ "slide": "slide1", "object": "btn", "name": "c", "index": 0 }),
    );
    let after = s.course.clone();
    let (obj, _) = s.course.slide("slide1").unwrap().object("btn").unwrap();
    let names: Vec<&str> = obj.states.iter().map(|st| st.name.as_str()).collect();
    assert_eq!(names, vec!["c", "a", "b"]);

    s.undo().unwrap();
    assert_eq!(s.course, before);

    s.redo().unwrap();
    assert_eq!(s.course, after);
}

#[test]
fn reorder_state_refuses_out_of_range_or_unknown_state() {
    let mut s = Session::blank("Reorder Refusals");
    add_button(&mut s, "btn");
    ok(
        &mut s,
        "object.add_state",
        json!({ "slide": "slide1", "object": "btn", "state": { "name": "a" } }),
    );
    assert!(
        s.execute(
            "object.reorder_state",
            &json!({ "slide": "slide1", "object": "btn", "name": "a", "index": 1 })
        )
        .is_err()
    );
    assert!(
        s.execute(
            "object.reorder_state",
            &json!({ "slide": "slide1", "object": "btn", "name": "missing", "index": 0 })
        )
        .is_err()
    );
}

#[test]
fn paste_onto_master_strips_states_and_resets_initial_state() {
    let mut s = Session::blank("Master Paste");
    let m_id = ok(&mut s, "master.add", json!({ "title": "M" }))["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let pasted = ok(
        &mut s,
        "object.paste",
        json!({
            "slide": m_id,
            "objects": [{
                "id": "btn_src",
                "name": "Button",
                "kind": { "type": "button", "label": "Hi" },
                "frame": { "x": 10, "y": 10, "w": 120, "h": 40 },
                "initial_state": "active",
                "states": [{ "name": "active", "fill": "#112233" }]
            }]
        }),
    );
    let new_id = pasted["id"].as_str().unwrap();
    let master = s.course.master(&m_id).unwrap();
    let (obj, _) = master.object(new_id).unwrap();
    assert!(obj.states.is_empty());
    assert_eq!(obj.initial_state, "normal");
}
