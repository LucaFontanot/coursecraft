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

#[test]
fn patch_state_partial_set_preserves_other_fields() {
    let mut s = Session::blank("Patch State");
    add_button(&mut s, "btn");
    ok(
        &mut s,
        "object.add_state",
        json!({
            "slide": "slide1",
            "object": "btn",
            "state": { "name": "hover", "text": "Keep me" }
        }),
    );

    ok(
        &mut s,
        "object.patch_state",
        json!({
            "slide": "slide1",
            "object": "btn",
            "name": "hover",
            "patch": {
                "fill_style": {
                    "type": "linear",
                    "angle": 90.0,
                    "stops": [
                        { "color": "#112233", "position": 0.0 },
                        { "color": "#445566", "position": 1.0 }
                    ]
                },
                "glow": { "color": "#2F6FEB", "size": 12.0 }
            }
        }),
    );

    let (obj, _) = s.course.slide("slide1").unwrap().object("btn").unwrap();
    let st = &obj.states[0];
    assert_eq!(st.text.as_deref(), Some("Keep me"));
    assert!(st.fill_style.is_some());
    assert_eq!(st.glow.as_ref().unwrap().size, 12.0);
}

#[test]
fn patch_state_clear_flag_and_null_reset_to_inherit() {
    let mut s = Session::blank("Patch Clear");
    add_button(&mut s, "btn");
    ok(
        &mut s,
        "object.add_state",
        json!({
            "slide": "slide1",
            "object": "btn",
            "state": {
                "name": "hover",
                "outline": { "color": "#000000", "width": 2.0 },
                "glow": { "color": "#2F6FEB", "size": 8.0 }
            }
        }),
    );

    ok(
        &mut s,
        "object.patch_state",
        json!({
            "slide": "slide1",
            "object": "btn",
            "name": "hover",
            "patch": {
                "clear_outline": true,
                "glow": null
            }
        }),
    );

    let (obj, _) = s.course.slide("slide1").unwrap().object("btn").unwrap();
    let st = &obj.states[0];
    assert!(st.outline.is_none());
    assert!(st.clear_outline);
    assert!(st.glow.is_none());

    // Setting outline: null clears clear_outline back to Inherit.
    ok(
        &mut s,
        "object.patch_state",
        json!({
            "slide": "slide1",
            "object": "btn",
            "name": "hover",
            "patch": { "outline": null }
        }),
    );
    let (obj, _) = s.course.slide("slide1").unwrap().object("btn").unwrap();
    assert!(!obj.states[0].clear_outline);
    assert!(obj.states[0].outline.is_none());
}

#[test]
fn patch_state_reset_all_clears_overrides_and_undoes_in_one_step() {
    let mut s = Session::blank("Patch Reset All");
    add_button(&mut s, "btn");
    ok(
        &mut s,
        "object.add_state",
        json!({
            "slide": "slide1",
            "object": "btn",
            "state": {
                "name": "hover",
                "fill": "#112233",
                "text": "Hover",
                "opacity": 0.5,
                "clear_shadow": true
            }
        }),
    );
    let before = s.course.clone();
    ok(
        &mut s,
        "object.patch_state",
        json!({
            "slide": "slide1",
            "object": "btn",
            "name": "hover",
            "reset_all": true,
            "patch": {}
        }),
    );
    let (obj, _) = s.course.slide("slide1").unwrap().object("btn").unwrap();
    assert_eq!(obj.states[0].name, "hover");
    assert!(obj.states[0].fill.is_none());
    assert!(obj.states[0].text.is_none());
    assert!(obj.states[0].opacity.is_none());
    assert!(!obj.states[0].clear_shadow);

    s.undo().unwrap();
    assert_eq!(s.course, before);
}

#[test]
fn patch_state_rejects_unknown_key_and_mutual_exclusion_without_mutating_course() {
    let mut s = Session::blank("Patch Refusal");
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

    let err_unknown = s
        .execute(
            "object.patch_state",
            &json!({
                "slide": "slide1",
                "object": "btn",
                "name": "hover",
                "patch": { "unknown_field": 42 }
            }),
        )
        .unwrap_err();
    assert!(err_unknown.to_string().contains("unknown"), "{err_unknown}");
    assert_eq!(s.course, before);

    let err_mutex = s
        .execute(
            "object.patch_state",
            &json!({
                "slide": "slide1",
                "object": "btn",
                "name": "hover",
                "patch": {
                    "outline": { "color": "#000000", "width": 2.0 },
                    "clear_outline": true
                }
            }),
        )
        .unwrap_err();
    assert!(
        err_mutex.to_string().contains("clear_outline"),
        "{err_mutex}"
    );
    assert_eq!(s.course, before);
}

#[test]
fn state_add_extra_adds_all_four_kinds_with_auto_id_and_undoes_redoes() {
    let mut s = Session::blank("Add Extra");
    add_button(&mut s, "btn");
    ok(
        &mut s,
        "object.add_state",
        json!({
            "slide": "slide1",
            "object": "btn",
            "state": { "name": "hover" }
        }),
    );

    let before = s.course.clone();
    let r1 = ok(
        &mut s,
        "object.state_add_extra",
        json!({
            "slide": "slide1",
            "object": "btn",
            "state": "hover",
            "extra": {
                "kind": { "type": "shape", "shape": "ellipse", "fill": "#22AA44" },
                "frame": { "x": -8.0, "y": -8.0, "w": 24.0, "h": 24.0 }
            }
        }),
    );
    assert_eq!(r1["id"], "extra_1");

    let r2 = ok(
        &mut s,
        "object.state_add_extra",
        json!({
            "slide": "slide1",
            "object": "btn",
            "state": "hover",
            "extra": {
                "kind": { "type": "image", "src": "check.png", "alt": "Done" },
                "frame": { "x": 10.0, "y": 10.0, "w": 16.0, "h": 16.0 }
            }
        }),
    );
    assert_eq!(r2["id"], "extra_2");

    let r3 = ok(
        &mut s,
        "object.state_add_extra",
        json!({
            "slide": "slide1",
            "object": "btn",
            "state": "hover",
            "extra": {
                "id": "custom_txt",
                "kind": { "type": "text", "text": "New!" },
                "frame": { "x": 0.0, "y": 30.0, "w": 50.0, "h": 20.0 }
            }
        }),
    );
    assert_eq!(r3["id"], "custom_txt");

    let r4 = ok(
        &mut s,
        "object.state_add_extra",
        json!({
            "slide": "slide1",
            "object": "btn",
            "state": "hover",
            "extra": {
                "kind": {
                    "type": "path",
                    "points": [{"x": 0.0, "y": 0.0}, {"x": 1.0, "y": 1.0}],
                    "outline": { "color": "#000000", "width": 2.0 }
                },
                "frame": { "x": 0.0, "y": 0.0, "w": 40.0, "h": 40.0 }
            }
        }),
    );
    assert_eq!(r4["id"], "extra_3");

    let after = s.course.clone();
    let (obj, _) = s.course.slide("slide1").unwrap().object("btn").unwrap();
    assert_eq!(obj.states[0].extras.len(), 4);

    for _ in 0..4 {
        s.undo().unwrap();
    }
    assert_eq!(s.course, before);

    for _ in 0..4 {
        s.redo().unwrap();
    }
    assert_eq!(s.course, after);
}

#[test]
fn state_add_extra_rejects_over_limit_and_unsafe_image_src() {
    let mut s = Session::blank("Add Extra Refusal");
    add_button(&mut s, "btn");
    ok(
        &mut s,
        "object.add_state",
        json!({
            "slide": "slide1",
            "object": "btn",
            "state": { "name": "hover" }
        }),
    );

    let before_unsafe = s.course.clone();
    assert!(
        s.execute(
            "object.state_add_extra",
            &json!({
                "slide": "slide1",
                "object": "btn",
                "state": "hover",
                "extra": {
                    "kind": { "type": "image", "src": "../secret.png" },
                    "frame": { "x": 0.0, "y": 0.0, "w": 20.0, "h": 20.0 }
                }
            })
        )
        .is_err()
    );
    assert_eq!(s.course, before_unsafe);

    for _ in 0..coursecraft_course::MAX_STATE_EXTRAS {
        ok(
            &mut s,
            "object.state_add_extra",
            json!({
                "slide": "slide1",
                "object": "btn",
                "state": "hover",
                "extra": {
                    "kind": { "type": "shape", "shape": "rectangle", "fill": "#112233" },
                    "frame": { "x": 0.0, "y": 0.0, "w": 10.0, "h": 10.0 }
                }
            }),
        );
    }

    let before_17th = s.course.clone();
    assert!(
        s.execute(
            "object.state_add_extra",
            &json!({
                "slide": "slide1",
                "object": "btn",
                "state": "hover",
                "extra": {
                    "kind": { "type": "shape", "shape": "rectangle", "fill": "#112233" },
                    "frame": { "x": 0.0, "y": 0.0, "w": 10.0, "h": 10.0 }
                }
            })
        )
        .is_err()
    );
    assert_eq!(s.course, before_17th);
}

#[test]
fn state_update_extra_patches_fields_clears_shadow_and_undoes_redoes() {
    let mut s = Session::blank("Update Extra");
    add_button(&mut s, "btn");
    ok(
        &mut s,
        "object.add_state",
        json!({ "slide": "slide1", "object": "btn", "state": { "name": "hover" } }),
    );
    ok(
        &mut s,
        "object.state_add_extra",
        json!({
            "slide": "slide1",
            "object": "btn",
            "state": "hover",
            "extra": {
                "id": "badge",
                "kind": { "type": "shape", "shape": "ellipse", "fill": "#22AA44" },
                "frame": { "x": 0.0, "y": 0.0, "w": 20.0, "h": 20.0 },
                "shadow": { "color": "#000000", "blur": 4.0, "offset_x": 1.0, "offset_y": 1.0 }
            }
        }),
    );

    let before = s.course.clone();
    ok(
        &mut s,
        "object.state_update_extra",
        json!({
            "slide": "slide1",
            "object": "btn",
            "state": "hover",
            "extra": "badge",
            "patch": {
                "name": "Star",
                "frame": { "x": -4.0, "y": -4.0, "w": 30.0, "h": 30.0 },
                "rotation": 30.0,
                "opacity": 0.75,
                "flip_h": true,
                "kind": { "type": "shape", "shape": "star_5", "fill": "#FFCC00" },
                "shadow": null
            }
        }),
    );
    let after = s.course.clone();
    let (obj, _) = s.course.slide("slide1").unwrap().object("btn").unwrap();
    let ex = &obj.states[0].extras[0];
    assert_eq!(ex.name, "Star");
    assert_eq!(ex.frame.x, -4.0);
    assert_eq!(ex.frame.w, 30.0);
    assert_eq!(ex.rotation, 30.0);
    assert_eq!(ex.opacity, 0.75);
    assert!(ex.flip_h);
    assert!(ex.shadow.is_none());

    s.undo().unwrap();
    assert_eq!(s.course, before);

    s.redo().unwrap();
    assert_eq!(s.course, after);
}

#[test]
fn state_update_extra_rejects_unknown_key_and_invalid_frame_without_mutating() {
    let mut s = Session::blank("Update Extra Refusal");
    add_button(&mut s, "btn");
    ok(
        &mut s,
        "object.add_state",
        json!({ "slide": "slide1", "object": "btn", "state": { "name": "hover" } }),
    );
    ok(
        &mut s,
        "object.state_add_extra",
        json!({
            "slide": "slide1",
            "object": "btn",
            "state": "hover",
            "extra": {
                "id": "badge",
                "kind": { "type": "shape", "shape": "ellipse", "fill": "#22AA44" },
                "frame": { "x": 0.0, "y": 0.0, "w": 20.0, "h": 20.0 }
            }
        }),
    );

    let before = s.course.clone();
    let err_unknown = s
        .execute(
            "object.state_update_extra",
            &json!({
                "slide": "slide1",
                "object": "btn",
                "state": "hover",
                "extra": "badge",
                "patch": { "bogus": 1 }
            }),
        )
        .unwrap_err();
    assert!(err_unknown.to_string().contains("unknown"), "{err_unknown}");
    assert_eq!(s.course, before);

    assert!(
        s.execute(
            "object.state_update_extra",
            &json!({
                "slide": "slide1",
                "object": "btn",
                "state": "hover",
                "extra": "badge",
                "patch": { "frame": { "x": 0.0, "y": 0.0, "w": -10.0, "h": 20.0 } }
            })
        )
        .is_err()
    );
    assert_eq!(s.course, before);
}

#[test]
fn state_delete_extra_and_reorder_extra_mutate_error_and_undo_redo() {
    let mut s = Session::blank("Delete & Reorder Extra");
    add_button(&mut s, "btn");
    ok(
        &mut s,
        "object.add_state",
        json!({ "slide": "slide1", "object": "btn", "state": { "name": "hover" } }),
    );
    for id in ["e0", "e1", "e2"] {
        ok(
            &mut s,
            "object.state_add_extra",
            json!({
                "slide": "slide1",
                "object": "btn",
                "state": "hover",
                "extra": {
                    "id": id,
                    "kind": { "type": "shape", "shape": "rectangle", "fill": "#112233" },
                    "frame": { "x": 0.0, "y": 0.0, "w": 10.0, "h": 10.0 }
                }
            }),
        );
    }

    // Reorder 0 -> 2
    let before_reorder = s.course.clone();
    ok(
        &mut s,
        "object.state_reorder_extra",
        json!({ "slide": "slide1", "object": "btn", "state": "hover", "extra": "e0", "index": 2 }),
    );
    let after_reorder = s.course.clone();
    let (obj, _) = s.course.slide("slide1").unwrap().object("btn").unwrap();
    let ids: Vec<&str> = obj.states[0].extras.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, vec!["e1", "e2", "e0"]);

    // Out-of-bounds index errors without mutating
    assert!(
        s.execute(
            "object.state_reorder_extra",
            &json!({ "slide": "slide1", "object": "btn", "state": "hover", "extra": "e1", "index": 3 })
        )
        .is_err()
    );
    assert_eq!(s.course, after_reorder);

    s.undo().unwrap();
    assert_eq!(s.course, before_reorder);
    s.redo().unwrap();
    assert_eq!(s.course, after_reorder);

    // Delete missing ID errors without mutating
    assert!(
        s.execute(
            "object.state_delete_extra",
            &json!({ "slide": "slide1", "object": "btn", "state": "hover", "extra": "missing" })
        )
        .is_err()
    );
    assert_eq!(s.course, after_reorder);

    // Delete e2 and undo/redo
    ok(
        &mut s,
        "object.state_delete_extra",
        json!({ "slide": "slide1", "object": "btn", "state": "hover", "extra": "e2" }),
    );
    let after_delete = s.course.clone();
    let (obj, _) = s.course.slide("slide1").unwrap().object("btn").unwrap();
    let ids: Vec<&str> = obj.states[0].extras.iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, vec!["e1", "e0"]);

    s.undo().unwrap();
    assert_eq!(s.course, after_reorder);
    s.redo().unwrap();
    assert_eq!(s.course, after_delete);
}
