//! Object state, state extra, delete, animation, and motion-path commands.

use std::collections::BTreeSet;

use coursecraft_course::{
    Action, Animation, Condition, Easing, Event, MotionPath, ObjectKind, Point, StateDef, Trigger,
};
use serde_json::{Value as Json, json};

use super::{bad, find_object_mut, keep_valid, obj, opt, req, slide_mut};
use crate::{EngineError, Result, Session};

fn validate_state_name(cmd: &str, raw: &str) -> Result<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(bad(cmd, "state name cannot be empty"));
    }
    if trimmed.chars().count() > coursecraft_course::MAX_STATE_NAME_CHARS {
        return Err(bad(
            cmd,
            format!(
                "state name cannot exceed {} characters",
                coursecraft_course::MAX_STATE_NAME_CHARS
            ),
        ));
    }
    if trimmed == "normal" {
        return Err(bad(cmd, "`normal` is built in and cannot be redefined"));
    }
    if trimmed == "hidden" {
        return Err(bad(
            cmd,
            "`hidden` is built in and needs no state definition",
        ));
    }
    Ok(trimmed.to_owned())
}

fn refuse_master_states(cmd: &str, s: &Session, sid: &str) -> Result<()> {
    if s.course.master(sid).is_some() {
        return Err(bad(cmd, "master objects cannot have states"));
    }
    Ok(())
}

fn next_copy_state_name(base: &str, existing: &[StateDef]) -> String {
    let taken: BTreeSet<&str> = existing.iter().map(|s| s.name.as_str()).collect();
    let max_chars = coursecraft_course::MAX_STATE_NAME_CHARS;
    for n in 1usize.. {
        let suffix = if n == 1 {
            " copy".to_owned()
        } else {
            format!(" copy {n}")
        };
        let suffix_chars = suffix.chars().count();
        let max_base = max_chars.saturating_sub(suffix_chars);
        let prefix: String = base.chars().take(max_base).collect();
        let candidate = format!("{prefix}{suffix}");
        if !taken.contains(candidate.as_str()) {
            return candidate;
        }
    }
    format!("{base} copy")
}

fn rewrite_state_in_condition(cond: &mut Condition, oid: &str, from: &str, to: &str) {
    match cond {
        Condition::All(list) | Condition::Any(list) => {
            for item in list {
                rewrite_state_in_condition(item, oid, from, to);
            }
        }
        Condition::Not(inner) => rewrite_state_in_condition(inner, oid, from, to),
        Condition::ObjectState { object, state, .. } if object == oid && state == from => {
            *state = to.to_owned();
        }
        Condition::ObjectState { .. } | Condition::Compare { .. } => {}
    }
}

fn rewrite_state_in_triggers(triggers: &mut [Trigger], oid: &str, from: &str, to: &str) {
    for trigger in triggers {
        if let Event::StateChanges {
            object,
            to: Some(state),
        } = &mut trigger.when
            && object == oid
            && state == from
        {
            *state = to.to_owned();
        }
        if let Action::ChangeState { object, state } = &mut trigger.action
            && object == oid
            && state == from
        {
            *state = to.to_owned();
        }
        if let Some(cond) = &mut trigger.condition {
            rewrite_state_in_condition(cond, oid, from, to);
        }
    }
}

pub(super) fn object_add_state(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "object.add_state";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let oid: String = req(C, p, "object")?;
    let mut state: StateDef = req(C, p, "state")?;
    refuse_master_states(C, s, &sid)?;
    state.name = validate_state_name(C, &state.name)?;
    let obj = find_object_mut(slide_mut(s, &sid)?, &oid)?;
    if obj.states.len() >= coursecraft_course::MAX_STATES_PER_OBJECT {
        return Err(bad(
            C,
            format!(
                "an object may have at most {} states",
                coursecraft_course::MAX_STATES_PER_OBJECT
            ),
        ));
    }
    if obj
        .states
        .iter()
        .any(|existing| existing.name == state.name)
    {
        return Err(bad(C, format!("state {:?} already exists", state.name)));
    }
    obj.states.push(state);
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

pub(super) fn object_update_state(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "object.update_state";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let oid: String = req(C, p, "object")?;
    let state: StateDef = req(C, p, "state")?;
    refuse_master_states(C, s, &sid)?;

    let slide = slide_mut(s, &sid)?;
    let obj = find_object_mut(slide, &oid)?;
    let target = obj
        .states
        .iter_mut()
        .find(|s| s.name == state.name)
        .ok_or_else(|| {
            EngineError::NotFound(format!(
                "state {:?} not found on object {oid:?}",
                state.name
            ))
        })?;
    *target = state;
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

fn patch_opt<T: serde::de::DeserializeOwned>(
    cmd: &str,
    map: &serde_json::Map<String, Json>,
    key: &str,
    dst: &mut Option<T>,
) -> Result<()> {
    let Some(val) = map.get(key) else {
        return Ok(());
    };
    if val.is_null() {
        *dst = None;
    } else {
        *dst = Some(
            serde_json::from_value(val.clone())
                .map_err(|e| EngineError::Invalid(format!("{cmd}: invalid {key}: {e}")))?,
        );
    }
    Ok(())
}

fn patch_nullable<T: serde::de::DeserializeOwned>(
    cmd: &str,
    map: &serde_json::Map<String, Json>,
    key: &str,
    clear_key: &str,
    dst: &mut Option<T>,
    clear_dst: &mut bool,
) -> Result<()> {
    if let Some(val) = map.get(key) {
        if val.is_null() {
            *dst = None;
            *clear_dst = false;
        } else {
            *dst = Some(
                serde_json::from_value(val.clone())
                    .map_err(|e| EngineError::Invalid(format!("{cmd}: invalid {key}: {e}")))?,
            );
            *clear_dst = false;
        }
    }
    if let Some(clear_val) = map.get(clear_key) {
        let flag = clear_val
            .as_bool()
            .ok_or_else(|| EngineError::Invalid(format!("{cmd}: {clear_key} must be a boolean")))?;
        *clear_dst = flag;
        if flag {
            *dst = None;
        }
    }
    Ok(())
}

pub(super) fn object_patch_state(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "object.patch_state";
    const ALLOWED_KEYS: &[&str] = &[
        "fill",
        "text",
        "src",
        "fill_style",
        "clear_fill_style",
        "outline",
        "clear_outline",
        "text_style",
        "runs",
        "paragraphs",
        "autofit",
        "opacity",
        "shadow",
        "clear_shadow",
        "glow",
        "clear_glow",
        "crop",
        "clear_crop",
        "image_shape",
        "clear_image_shape",
        "rotation",
        "flip_h",
        "flip_v",
    ];
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let oid: String = req(C, p, "object")?;
    let name: String = req(C, p, "name")?;
    let reset_all: bool = opt(C, p, "reset_all")?.unwrap_or(false);
    let patch_val: Option<Json> = opt(C, p, "patch")?;
    let empty_map = serde_json::Map::new();
    let patch_map = match &patch_val {
        Some(v) => v
            .as_object()
            .ok_or_else(|| EngineError::Invalid(format!("{C}: patch must be a JSON object")))?,
        None => &empty_map,
    };

    for k in patch_map.keys() {
        if !ALLOWED_KEYS.contains(&k.as_str()) {
            return Err(EngineError::Invalid(format!(
                "{C}: unknown patch field {k:?}"
            )));
        }
    }
    for (field_key, clear_key) in [
        ("fill_style", "clear_fill_style"),
        ("outline", "clear_outline"),
        ("shadow", "clear_shadow"),
        ("glow", "clear_glow"),
        ("crop", "clear_crop"),
        ("image_shape", "clear_image_shape"),
    ] {
        let sets_val = patch_map.get(field_key).is_some_and(|v| !v.is_null());
        let sets_clear = patch_map.get(clear_key).and_then(Json::as_bool) == Some(true);
        if sets_val && sets_clear {
            return Err(EngineError::Invalid(format!(
                "{C}: cannot set both {field_key} and {clear_key}: true"
            )));
        }
    }

    refuse_master_states(C, s, &sid)?;

    let slide = slide_mut(s, &sid)?;
    let obj = find_object_mut(slide, &oid)?;
    let target = obj
        .states
        .iter_mut()
        .find(|st| st.name == name)
        .ok_or_else(|| {
            EngineError::NotFound(format!("state {name:?} not found on object {oid:?}"))
        })?;

    if reset_all {
        let extras = std::mem::take(&mut target.extras);
        *target = StateDef {
            name: target.name.clone(),
            extras,
            ..Default::default()
        };
    }

    patch_opt(C, patch_map, "fill", &mut target.fill)?;
    patch_opt(C, patch_map, "text", &mut target.text)?;
    patch_opt(C, patch_map, "src", &mut target.src)?;
    patch_nullable(
        C,
        patch_map,
        "fill_style",
        "clear_fill_style",
        &mut target.fill_style,
        &mut target.clear_fill_style,
    )?;
    patch_nullable(
        C,
        patch_map,
        "outline",
        "clear_outline",
        &mut target.outline,
        &mut target.clear_outline,
    )?;
    patch_opt(C, patch_map, "text_style", &mut target.text_style)?;
    patch_opt(C, patch_map, "runs", &mut target.runs)?;
    patch_opt(C, patch_map, "paragraphs", &mut target.paragraphs)?;
    patch_opt(C, patch_map, "autofit", &mut target.autofit)?;
    patch_opt(C, patch_map, "opacity", &mut target.opacity)?;
    if let Some(op) = target.opacity {
        target.opacity = Some(coursecraft_course::clamp_opacity(op));
    }
    patch_nullable(
        C,
        patch_map,
        "shadow",
        "clear_shadow",
        &mut target.shadow,
        &mut target.clear_shadow,
    )?;
    patch_nullable(
        C,
        patch_map,
        "glow",
        "clear_glow",
        &mut target.glow,
        &mut target.clear_glow,
    )?;
    patch_nullable(
        C,
        patch_map,
        "crop",
        "clear_crop",
        &mut target.crop,
        &mut target.clear_crop,
    )?;
    patch_nullable(
        C,
        patch_map,
        "image_shape",
        "clear_image_shape",
        &mut target.image_shape,
        &mut target.clear_image_shape,
    )?;
    patch_opt(C, patch_map, "rotation", &mut target.rotation)?;
    if let Some(rot) = target.rotation {
        target.rotation = Some(coursecraft_course::normalize_rotation(rot));
    }
    patch_opt(C, patch_map, "flip_h", &mut target.flip_h)?;
    patch_opt(C, patch_map, "flip_v", &mut target.flip_v)?;

    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

pub(super) fn object_delete_state(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "object.delete_state";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let oid: String = req(C, p, "object")?;
    let name: String = req(C, p, "name")?;

    let slide = slide_mut(s, &sid)?;
    let obj = find_object_mut(slide, &oid)?;
    let before = obj.states.len();
    obj.states.retain(|s| s.name != name);
    if obj.states.len() == before {
        return Err(EngineError::NotFound(format!(
            "state {name:?} not found on object {oid:?}"
        )));
    }
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

pub(super) fn object_duplicate_state(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "object.duplicate_state";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let oid: String = req(C, p, "object")?;
    let name: String = req(C, p, "name")?;
    let new_name: Option<String> = opt(C, p, "new_name")?;
    refuse_master_states(C, s, &sid)?;

    let obj = find_object_mut(slide_mut(s, &sid)?, &oid)?;
    if obj.states.len() >= coursecraft_course::MAX_STATES_PER_OBJECT {
        return Err(bad(
            C,
            format!(
                "an object may have at most {} states",
                coursecraft_course::MAX_STATES_PER_OBJECT
            ),
        ));
    }
    let idx = obj
        .states
        .iter()
        .position(|st| st.name == name)
        .ok_or_else(|| {
            EngineError::NotFound(format!("state {name:?} not found on object {oid:?}"))
        })?;
    let created_name = match new_name {
        Some(raw) => {
            let validated = validate_state_name(C, &raw)?;
            if obj.states.iter().any(|st| st.name == validated) {
                return Err(bad(C, format!("state {validated:?} already exists")));
            }
            validated
        }
        None => next_copy_state_name(&name, &obj.states),
    };
    let mut dup = obj.states[idx].clone();
    dup.name = created_name.clone();
    obj.states.insert(idx + 1, dup);
    keep_valid(s, was_valid)?;
    Ok(json!({ "name": created_name }))
}

pub(super) fn object_rename_state(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "object.rename_state";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let oid: String = req(C, p, "object")?;
    let from: String = req(C, p, "from")?;
    let to_raw: String = req(C, p, "to")?;
    refuse_master_states(C, s, &sid)?;
    let to = validate_state_name(C, &to_raw)?;

    let slide = slide_mut(s, &sid)?;
    {
        let obj = find_object_mut(slide, &oid)?;
        if obj.states.iter().any(|st| st.name == to) {
            return Err(bad(C, format!("state {to:?} already exists")));
        }
        let target = obj
            .states
            .iter_mut()
            .find(|st| st.name == from)
            .ok_or_else(|| {
                EngineError::NotFound(format!("state {from:?} not found on object {oid:?}"))
            })?;
        target.name = to.clone();
        if obj.initial_state == from {
            obj.initial_state = to.clone();
        }
    }
    rewrite_state_in_triggers(&mut slide.triggers, &oid, &from, &to);
    for layer in &mut slide.layers {
        rewrite_state_in_triggers(&mut layer.triggers, &oid, &from, &to);
    }
    if let Some(question) = &mut slide.question
        && let Some(cond) = &mut question.correct_when
    {
        rewrite_state_in_condition(cond, &oid, &from, &to);
    }
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

pub(super) fn object_reorder_state(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "object.reorder_state";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let oid: String = req(C, p, "object")?;
    let name: String = req(C, p, "name")?;
    let index: usize = req(C, p, "index")?;
    refuse_master_states(C, s, &sid)?;

    let obj = find_object_mut(slide_mut(s, &sid)?, &oid)?;
    let pos = obj
        .states
        .iter()
        .position(|st| st.name == name)
        .ok_or_else(|| {
            EngineError::NotFound(format!("state {name:?} not found on object {oid:?}"))
        })?;
    if index >= obj.states.len() {
        return Err(bad(
            C,
            format!("index {index} out of range (0..{})", obj.states.len()),
        ));
    }
    let item = obj.states.remove(pos);
    obj.states.insert(index, item);
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

fn next_extra_id(existing: &[coursecraft_course::StateExtra]) -> String {
    let taken: BTreeSet<&str> = existing.iter().map(|e| e.id.as_str()).collect();
    for n in 1usize.. {
        let candidate = format!("extra_{n}");
        if !taken.contains(candidate.as_str()) {
            return candidate;
        }
    }
    "extra_1".to_owned()
}

fn patch_req<T: serde::de::DeserializeOwned>(
    cmd: &str,
    map: &serde_json::Map<String, Json>,
    key: &str,
    dst: &mut T,
) -> Result<()> {
    let Some(val) = map.get(key) else {
        return Ok(());
    };
    *dst = serde_json::from_value(val.clone())
        .map_err(|e| EngineError::Invalid(format!("{cmd}: invalid {key}: {e}")))?;
    Ok(())
}

pub(super) fn object_state_add_extra(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "object.state_add_extra";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let oid: String = req(C, p, "object")?;
    let state_name: String = req(C, p, "state")?;
    let mut extra: coursecraft_course::StateExtra = req(C, p, "extra")?;
    refuse_master_states(C, s, &sid)?;

    let obj = find_object_mut(slide_mut(s, &sid)?, &oid)?;
    let st = obj
        .states
        .iter_mut()
        .find(|st| st.name == state_name)
        .ok_or_else(|| {
            EngineError::NotFound(format!("state {state_name:?} not found on object {oid:?}"))
        })?;
    if st.extras.len() >= coursecraft_course::MAX_STATE_EXTRAS {
        return Err(bad(
            C,
            format!(
                "a state may have at most {} extras",
                coursecraft_course::MAX_STATE_EXTRAS
            ),
        ));
    }
    if extra.id.trim().is_empty() {
        extra.id = next_extra_id(&st.extras);
    }
    let id = extra.id.clone();
    st.extras.push(extra);
    keep_valid(s, was_valid)?;
    Ok(serde_json::json!({ "id": id }))
}

pub(super) fn object_state_update_extra(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "object.state_update_extra";
    const ALLOWED_KEYS: &[&str] = &[
        "name", "frame", "kind", "rotation", "flip_h", "flip_v", "opacity", "shadow", "glow",
    ];
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let oid: String = req(C, p, "object")?;
    let state_name: String = req(C, p, "state")?;
    let extra_id: String = req(C, p, "extra")?;
    let patch_val: Json = req(C, p, "patch")?;
    let patch_map = patch_val
        .as_object()
        .ok_or_else(|| EngineError::Invalid(format!("{C}: patch must be a JSON object")))?;
    for k in patch_map.keys() {
        if !ALLOWED_KEYS.contains(&k.as_str()) {
            return Err(EngineError::Invalid(format!(
                "{C}: unknown patch field {k:?}"
            )));
        }
    }
    refuse_master_states(C, s, &sid)?;

    let obj = find_object_mut(slide_mut(s, &sid)?, &oid)?;
    let st = obj
        .states
        .iter_mut()
        .find(|st| st.name == state_name)
        .ok_or_else(|| {
            EngineError::NotFound(format!("state {state_name:?} not found on object {oid:?}"))
        })?;
    let target = st
        .extras
        .iter_mut()
        .find(|e| e.id == extra_id)
        .ok_or_else(|| {
            EngineError::NotFound(format!(
                "extra {extra_id:?} not found in state {state_name:?}"
            ))
        })?;

    patch_req(C, patch_map, "name", &mut target.name)?;
    patch_req(C, patch_map, "frame", &mut target.frame)?;
    patch_req(C, patch_map, "kind", &mut target.kind)?;
    if patch_map.contains_key("rotation") {
        patch_req(C, patch_map, "rotation", &mut target.rotation)?;
        if !target.rotation.is_finite() {
            return Err(EngineError::Invalid(format!(
                "{C}: rotation must be finite"
            )));
        }
        target.rotation = coursecraft_course::normalize_rotation(target.rotation);
    }
    patch_req(C, patch_map, "flip_h", &mut target.flip_h)?;
    patch_req(C, patch_map, "flip_v", &mut target.flip_v)?;
    if patch_map.contains_key("opacity") {
        patch_req(C, patch_map, "opacity", &mut target.opacity)?;
        if !target.opacity.is_finite() {
            return Err(EngineError::Invalid(format!("{C}: opacity must be finite")));
        }
        target.opacity = coursecraft_course::clamp_opacity(target.opacity);
    }
    patch_opt(C, patch_map, "shadow", &mut target.shadow)?;
    patch_opt(C, patch_map, "glow", &mut target.glow)?;

    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

pub(super) fn object_state_delete_extra(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "object.state_delete_extra";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let oid: String = req(C, p, "object")?;
    let state_name: String = req(C, p, "state")?;
    let extra_id: String = req(C, p, "extra")?;
    refuse_master_states(C, s, &sid)?;

    let obj = find_object_mut(slide_mut(s, &sid)?, &oid)?;
    let st = obj
        .states
        .iter_mut()
        .find(|st| st.name == state_name)
        .ok_or_else(|| {
            EngineError::NotFound(format!("state {state_name:?} not found on object {oid:?}"))
        })?;
    let before = st.extras.len();
    st.extras.retain(|e| e.id != extra_id);
    if st.extras.len() == before {
        return Err(EngineError::NotFound(format!(
            "extra {extra_id:?} not found in state {state_name:?}"
        )));
    }
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

pub(super) fn object_state_reorder_extra(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "object.state_reorder_extra";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let oid: String = req(C, p, "object")?;
    let state_name: String = req(C, p, "state")?;
    let extra_id: String = req(C, p, "extra")?;
    let index: usize = req(C, p, "index")?;
    refuse_master_states(C, s, &sid)?;

    let obj = find_object_mut(slide_mut(s, &sid)?, &oid)?;
    let st = obj
        .states
        .iter_mut()
        .find(|st| st.name == state_name)
        .ok_or_else(|| {
            EngineError::NotFound(format!("state {state_name:?} not found on object {oid:?}"))
        })?;
    let pos = st
        .extras
        .iter()
        .position(|e| e.id == extra_id)
        .ok_or_else(|| {
            EngineError::NotFound(format!(
                "extra {extra_id:?} not found in state {state_name:?}"
            ))
        })?;
    if index >= st.extras.len() {
        return Err(bad(
            C,
            format!("index {index} out of range (0..{})", st.extras.len()),
        ));
    }
    let item = st.extras.remove(pos);
    st.extras.insert(index, item);
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

pub(super) fn object_delete(s: &mut Session, p: &Json) -> Result<Json> {
    let was_valid = s.is_playable();
    let sid: String = req("object.delete", p, "slide")?;
    let oid: String = req("object.delete", p, "object")?;
    let slide = slide_mut(s, &sid)?;
    let is_panel = slide
        .object(&oid)
        .is_some_and(|(object, _)| matches!(object.kind, ObjectKind::ScrollPanel { .. }));
    let before = slide.local_ids().count();
    slide.objects.retain(|o| o.id != oid);
    for l in &mut slide.layers {
        l.objects.retain(|o| o.id != oid);
    }
    if slide.local_ids().count() == before {
        return Err(EngineError::NotFound(format!(
            "object {oid:?} is not on slide {sid:?}"
        )));
    }
    // Deleting a panel releases every child in the same undo step.
    if is_panel {
        for object in &mut slide.objects {
            if object.panel.as_deref() == Some(oid.as_str()) {
                object.panel = None;
            }
        }
        for layer in &mut slide.layers {
            for object in &mut layer.objects {
                if object.panel.as_deref() == Some(oid.as_str()) {
                    object.panel = None;
                }
            }
        }
    }
    // Triggers that start from the object or act on it go with it, in the
    // same undo step. Anything else that still needs it (a group, a
    // question) makes keep_valid refuse the delete and name it.
    let removed = if was_valid {
        remove_dangling_triggers(
            s,
            &sid,
            &[
                format!("object {oid:?} is not on slide"),
                format!("target {oid:?} is not on slide"),
            ],
        )
    } else {
        0
    };
    keep_valid(s, was_valid)?;
    Ok(json!({ "removed_triggers": removed }))
}

/// Remove the triggers on slide `sid` (base or layers) that validation
/// reports with a message starting with one of `needles`, one at a time
/// until none is left. Returns how many were removed.
///
/// Validation is the single source of truth for what a trigger refers to,
/// so the event, the action and the condition are all covered.
pub(super) fn remove_dangling_triggers(s: &mut Session, sid: &str, needles: &[String]) -> usize {
    let Some(slide_path) = s
        .course
        .scenes
        .iter()
        .enumerate()
        .find_map(|(si, scene)| {
            scene
                .slides
                .iter()
                .position(|slide| slide.id == sid)
                .map(|li| format!("/scenes/{si}/slides/{li}/"))
        })
        .or_else(|| {
            s.course.banks.iter().enumerate().find_map(|(bi, bank)| {
                bank.slides
                    .iter()
                    .position(|slide| slide.id == sid)
                    .map(|si| format!("/banks/{bi}/slides/{si}/"))
            })
        })
    else {
        return 0;
    };
    let targets: BTreeSet<(Option<usize>, usize)> = s
        .course
        .validate()
        .into_iter()
        .filter_map(|issue| {
            if issue.severity != coursecraft_course::Severity::Error
                || !needles
                    .iter()
                    .any(|n| issue.message.starts_with(n.as_str()))
            {
                return None;
            }
            let rest = issue.path.strip_prefix(slide_path.as_str())?;
            let mut parts = rest.split('/');
            match (parts.next(), parts.next(), parts.next(), parts.next()) {
                (Some("triggers"), Some(t), _, _) => Some((None, t.parse::<usize>().ok()?)),
                (Some("layers"), Some(l), Some("triggers"), Some(t)) => {
                    Some((Some(l.parse::<usize>().ok()?), t.parse::<usize>().ok()?))
                }
                _ => None,
            }
        })
        .collect();
    if targets.is_empty() {
        return 0;
    }
    let Some(slide) = s.course.slide_mut(sid) else {
        return 0;
    };
    let mut removed = 0;
    // Remove in descending index order so earlier removals do not shift later indices.
    for (layer, index) in targets.into_iter().rev() {
        let triggers = match layer {
            None => &mut slide.triggers,
            Some(l) => match slide.layers.get_mut(l) {
                Some(layer) => &mut layer.triggers,
                None => continue,
            },
        };
        if index < triggers.len() {
            triggers.remove(index);
            removed += 1;
        }
    }
    removed
}

pub(super) fn object_set_animation(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "object.set_animation";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let oid: String = req(C, p, "object")?;
    let which: String = req(C, p, "which")?;
    if which != "entrance" && which != "exit" {
        return Err(bad(
            C,
            format!("which must be 'entrance' or 'exit', got {which:?}"),
        ));
    }
    let map = obj(C, p)?;
    let anim_val = map
        .get("animation")
        .ok_or_else(|| bad(C, "missing `animation`"))?;
    let anim: Option<Animation> = match anim_val {
        Json::Null => None,
        val => {
            let a: Animation = serde_json::from_value(val.clone())
                .map_err(|e| bad(C, format!("invalid animation: {e}")))?;
            if a.duration_ms < 1 || a.duration_ms > 60000 {
                return Err(bad(C, "animation duration_ms must be 1..=60000"));
            }
            Some(a)
        }
    };
    let slide = slide_mut(s, &sid)?;
    let target = find_object_mut(slide, &oid)?;
    if which == "entrance" {
        target.entrance = anim;
    } else {
        target.exit = anim;
    }
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

pub(super) fn object_add_motion_path(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "object.add_motion_path";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let oid: String = req(C, p, "object")?;
    let points: Option<Vec<Point>> = opt(C, p, "points")?;
    let duration_ms: Option<u32> = opt(C, p, "duration_ms")?;
    let easing: Option<Easing> = opt(C, p, "easing")?;
    let relative: Option<bool> = opt(C, p, "relative")?;

    let points =
        points.unwrap_or_else(|| vec![Point { x: 0.0, y: 0.0 }, Point { x: 200.0, y: 0.0 }]);
    if points.len() < 2 || points.len() > 100 {
        return Err(bad(C, "motion path points must be 2..=100"));
    }
    for pt in &points {
        if !pt.x.is_finite() || !pt.y.is_finite() || pt.x.abs() > 100000.0 || pt.y.abs() > 100000.0
        {
            return Err(bad(
                C,
                "motion path points must be finite and within ±100000",
            ));
        }
    }
    let duration_ms = duration_ms.unwrap_or(1000);
    if !(1..=60000).contains(&duration_ms) {
        return Err(bad(C, "motion path duration_ms must be 1..=60000"));
    }
    let easing = easing.unwrap_or_default();
    let relative = relative.unwrap_or(true);

    let slide = slide_mut(s, &sid)?;
    let target = find_object_mut(slide, &oid)?;
    if target.motion_paths.len() >= 20 {
        return Err(bad(C, "object cannot have more than 20 motion paths"));
    }

    let next_num = (1..)
        .find(|n| {
            let candidate = format!("path{n}");
            !target.motion_paths.iter().any(|mp| mp.id == candidate)
        })
        .unwrap_or(1);
    let id = format!("path{next_num}");

    target.motion_paths.push(MotionPath {
        id: id.clone(),
        points,
        duration_ms,
        easing,
        relative,
    });
    keep_valid(s, was_valid)?;
    Ok(json!({ "id": id }))
}

pub(super) fn object_update_motion_path(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "object.update_motion_path";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let oid: String = req(C, p, "object")?;
    let path_id: String = req(C, p, "path")?;
    let points: Option<Vec<Point>> = opt(C, p, "points")?;
    let duration_ms: Option<u32> = opt(C, p, "duration_ms")?;
    let easing: Option<Easing> = opt(C, p, "easing")?;
    let relative: Option<bool> = opt(C, p, "relative")?;

    if let Some(ref pts) = points {
        if pts.len() < 2 || pts.len() > 100 {
            return Err(bad(C, "motion path points must be 2..=100"));
        }
        for pt in pts {
            if !pt.x.is_finite()
                || !pt.y.is_finite()
                || pt.x.abs() > 100000.0
                || pt.y.abs() > 100000.0
            {
                return Err(bad(
                    C,
                    "motion path points must be finite and within ±100000",
                ));
            }
        }
    }
    if let Some(d) = duration_ms
        && !(1..=60000).contains(&d)
    {
        return Err(bad(C, "motion path duration_ms must be 1..=60000"));
    }

    let slide = slide_mut(s, &sid)?;
    let target = find_object_mut(slide, &oid)?;
    let path = target
        .motion_paths
        .iter_mut()
        .find(|mp| mp.id == path_id)
        .ok_or_else(|| {
            EngineError::NotFound(format!(
                "motion path {path_id:?} not found on object {oid:?}"
            ))
        })?;

    if let Some(pts) = points {
        path.points = pts;
    }
    if let Some(d) = duration_ms {
        path.duration_ms = d;
    }
    if let Some(e) = easing {
        path.easing = e;
    }
    if let Some(r) = relative {
        path.relative = r;
    }

    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

pub(super) fn object_remove_motion_path(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "object.remove_motion_path";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let oid: String = req(C, p, "object")?;
    let path_id: String = req(C, p, "path")?;

    let slide = slide_mut(s, &sid)?;
    let target = find_object_mut(slide, &oid)?;
    let pos = target
        .motion_paths
        .iter()
        .position(|mp| mp.id == path_id)
        .ok_or_else(|| {
            EngineError::NotFound(format!(
                "motion path {path_id:?} not found on object {oid:?}"
            ))
        })?;
    target.motion_paths.remove(pos);

    // Triggers naming the removed path cannot run again: remove them in the
    // same undo step so the course stays valid.
    slide
        .triggers
        .retain(|t| !trigger_references_path(t, &oid, &path_id));
    for l in &mut slide.layers {
        l.triggers
            .retain(|t| !trigger_references_path(t, &oid, &path_id));
    }

    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

fn trigger_references_path(t: &Trigger, obj_id: &str, path_id: &str) -> bool {
    if let Action::MoveOnPath {
        ref object,
        ref path,
    } = t.action
        && object == obj_id
        && path == path_id
    {
        return true;
    }
    if let Event::AnimationCompletes {
        ref object,
        path: Some(ref path),
    } = t.when
        && object == obj_id
        && path == path_id
    {
        return true;
    }
    false
}
