//! Video and slide timeline cue point commands.

use coursecraft_course::{
    Action, Cue, Event, Layer, MAX_CUES, Object, ObjectKind, Rect, RevisitMode, SeekMode, Slide,
    SlideCue, Trigger,
};
use serde_json::{Value as Json, json};

use super::{
    bad, find_object_mut, fresh_id, interaction_button, interaction_text, interaction_trigger,
    keep_valid, obj, opt, req, slide_mut, text, validate_id,
};
use crate::{EngineError, Result, Session};

pub(super) fn video_add_cue(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "video.add_cue";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let oid: String = req(C, p, "object")?;
    let slide = slide_mut(s, &sid)?;
    let duration = owner_duration(slide, &oid)
        .ok_or_else(|| EngineError::NotFound(format!("object {oid:?} is not on this slide")))?;
    let cues = video_cues_mut(slide, &oid, C)?;
    if cues.len() >= MAX_CUES {
        return Err(bad(C, format!("a video may have at most {MAX_CUES} cues")));
    }
    let at_ms = cue_at_ms(C, p, "at_ms")?.unwrap_or(0.0);
    check_cue_time(C, at_ms, duration)?;
    let name = match opt::<String>(C, p, "name")? {
        Some(name) => cue_name(C, name)?,
        None => format!("Cue {}", cues.len() + 1),
    };
    let pause: bool = opt(C, p, "pause")?.unwrap_or(false);
    let id = match opt::<String>(C, p, "id")? {
        Some(id) => {
            validate_id(C, &id)?;
            if cues.iter().any(|cue| cue.id == id) {
                return Err(bad(C, format!("cue id {id:?} is already used on {oid:?}")));
            }
            id
        }
        None => fresh_id("cue", cues.iter().map(|cue| cue.id.as_str())),
    };
    cues.push(Cue {
        id: id.clone(),
        at_ms,
        name,
        pause,
    });
    cues.sort_by(|a, b| a.at_ms.total_cmp(&b.at_ms));
    keep_valid(s, was_valid)?;
    Ok(json!({ "id": id, "cue": id }))
}

pub(super) fn video_set_cue(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "video.set_cue";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let oid: String = req(C, p, "object")?;
    let cue_id: String = req(C, p, "cue")?;
    let patch = obj(C, p)?
        .get("patch")
        .and_then(Json::as_object)
        .ok_or_else(|| bad(C, "missing `patch` object"))?;
    let slide = slide_mut(s, &sid)?;
    let duration = owner_duration(slide, &oid)
        .ok_or_else(|| EngineError::NotFound(format!("object {oid:?} is not on this slide")))?;
    let cues = video_cues_mut(slide, &oid, C)?;
    let cue = cues
        .iter_mut()
        .find(|cue| cue.id == cue_id)
        .ok_or_else(|| EngineError::NotFound(format!("cue {cue_id:?} is not on {oid:?}")))?;
    if let Some(value) = patch.get("at_ms") {
        let at_ms: f64 =
            serde_json::from_value(value.clone()).map_err(|e| bad(C, format!("`at_ms`: {e}")))?;
        check_cue_time(C, at_ms, duration)?;
        cue.at_ms = at_ms;
    }
    if let Some(value) = patch.get("name") {
        let name: String =
            serde_json::from_value(value.clone()).map_err(|e| bad(C, format!("`name`: {e}")))?;
        cue.name = cue_name(C, name)?;
    }
    if let Some(value) = patch.get("pause") {
        cue.pause =
            serde_json::from_value(value.clone()).map_err(|e| bad(C, format!("`pause`: {e}")))?;
    }
    cues.sort_by(|a, b| a.at_ms.total_cmp(&b.at_ms));
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

pub(super) fn video_remove_cue(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "video.remove_cue";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let oid: String = req(C, p, "object")?;
    let cue_id: String = req(C, p, "cue")?;
    let slide = slide_mut(s, &sid)?;
    {
        let cues = video_cues_mut(slide, &oid, C)?;
        let before = cues.len();
        cues.retain(|cue| cue.id != cue_id);
        if cues.len() == before {
            return Err(EngineError::NotFound(format!(
                "cue {cue_id:?} is not on {oid:?}"
            )));
        }
    }
    // A trigger naming the removed cue can never fire again: remove it in the
    // same undo step.
    let matches_cue = |trigger: &Trigger| {
        matches!(
            &trigger.when,
            Event::CueReached { object, cue } if object == &oid && cue == &cue_id
        )
    };
    slide.triggers.retain(|trigger| !matches_cue(trigger));
    for layer in &mut slide.layers {
        layer.triggers.retain(|trigger| !matches_cue(trigger));
    }
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

fn slide_cues_mut<'a>(
    slide: &'a mut Slide,
    layer: Option<&str>,
) -> Result<(u32, &'a mut Vec<SlideCue>)> {
    match layer {
        Some(lid) => {
            let l = slide
                .layers
                .iter_mut()
                .find(|l| l.id == lid)
                .ok_or_else(|| {
                    EngineError::NotFound(format!("layer {lid:?} is not on this slide"))
                })?;
            Ok((l.duration_ms, &mut l.cue_points))
        }
        None => Ok((slide.duration_ms, &mut slide.cue_points)),
    }
}

pub(super) fn slide_add_cue(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "slide.add_cue";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let lid: Option<String> = opt(C, p, "layer")?;
    let at_ms: u32 = opt(C, p, "at_ms")?.unwrap_or(0);
    let name_opt: Option<String> = opt(C, p, "name")?;
    let id_opt: Option<String> = opt(C, p, "id")?;
    let slide = slide_mut(s, &sid)?;
    let (duration, cues) = slide_cues_mut(slide, lid.as_deref())?;
    if cues.len() >= MAX_CUES {
        return Err(bad(
            C,
            format!("a timeline may have at most {MAX_CUES} cues"),
        ));
    }
    if at_ms > duration {
        return Err(bad(
            C,
            format!("cue time {at_ms} ms exceeds timeline duration {duration} ms"),
        ));
    }
    let name = match name_opt {
        Some(name) => cue_name(C, name)?,
        None => format!("Cue {}", cues.len() + 1),
    };
    let id = match id_opt {
        Some(id) => {
            validate_id(C, &id)?;
            if cues.iter().any(|cue| cue.id == id) {
                return Err(bad(C, format!("cue id {id:?} is already used")));
            }
            id
        }
        None => fresh_id("cue", cues.iter().map(|cue| cue.id.as_str())),
    };
    cues.push(SlideCue {
        id: id.clone(),
        name,
        at_ms,
    });
    cues.sort_by_key(|c| c.at_ms);
    keep_valid(s, was_valid)?;
    Ok(json!({ "id": id, "cue": id }))
}

pub(super) fn slide_set_cue(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "slide.set_cue";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let lid: Option<String> = opt(C, p, "layer")?;
    let cue_id: String = req(C, p, "cue")?;
    let patch = obj(C, p)?
        .get("patch")
        .and_then(Json::as_object)
        .ok_or_else(|| bad(C, "missing `patch` object"))?;
    let slide = slide_mut(s, &sid)?;
    let mut updated_at_ms: Option<u32> = None;
    {
        let (duration, cues) = slide_cues_mut(slide, lid.as_deref())?;
        let cue = cues
            .iter_mut()
            .find(|cue| cue.id == cue_id)
            .ok_or_else(|| EngineError::NotFound(format!("cue {cue_id:?} not found")))?;
        if let Some(value) = patch.get("at_ms") {
            let at_ms: u32 = serde_json::from_value(value.clone())
                .map_err(|e| bad(C, format!("`at_ms`: {e}")))?;
            if at_ms > duration {
                return Err(bad(
                    C,
                    format!("cue time {at_ms} ms exceeds timeline duration {duration} ms"),
                ));
            }
            cue.at_ms = at_ms;
            updated_at_ms = Some(at_ms);
        }
        if let Some(value) = patch.get("name") {
            let name: String = serde_json::from_value(value.clone())
                .map_err(|e| bad(C, format!("`name`: {e}")))?;
            cue.name = cue_name(C, name)?;
        }
        cues.sort_by_key(|c| c.at_ms);
    }
    if let Some(new_ms) = updated_at_ms {
        let sync_trigger = |t: &mut Trigger| {
            if let Event::TimelineReaches { ms, layer, cue } = &mut t.when
                && cue.as_deref() == Some(cue_id.as_str())
                && layer.as_deref() == lid.as_deref()
            {
                *ms = new_ms;
            }
        };
        for t in &mut slide.triggers {
            sync_trigger(t);
        }
        for l in &mut slide.layers {
            for t in &mut l.triggers {
                sync_trigger(t);
            }
        }
    }
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

pub(super) fn slide_remove_cue(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "slide.remove_cue";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let lid: Option<String> = opt(C, p, "layer")?;
    let cue_id: String = req(C, p, "cue")?;
    let slide = slide_mut(s, &sid)?;
    {
        let (_, cues) = slide_cues_mut(slide, lid.as_deref())?;
        let before = cues.len();
        cues.retain(|cue| cue.id != cue_id);
        if cues.len() == before {
            return Err(EngineError::NotFound(format!("cue {cue_id:?} not found")));
        }
    }
    let matches_cue = |trigger: &Trigger| {
        matches!(
            &trigger.when,
            Event::TimelineReaches { layer, cue: Some(c), .. }
                if c == &cue_id && layer.as_deref() == lid.as_deref()
        )
    };
    slide.triggers.retain(|trigger| !matches_cue(trigger));
    for layer in &mut slide.layers {
        layer.triggers.retain(|trigger| !matches_cue(trigger));
    }
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

pub(super) fn video_add_question_cue(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "video.add_question_cue";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let oid: String = req(C, p, "object")?;
    let theme = s.course.theme.clone();
    let size = s.course.slide_size;
    let slide = slide_mut(s, &sid)?;
    let duration = owner_duration(slide, &oid)
        .ok_or_else(|| EngineError::NotFound(format!("object {oid:?} is not on this slide")))?;
    let at_ms = cue_at_ms(C, p, "at_ms")?.unwrap_or(0.0);
    check_cue_time(C, at_ms, duration)?;
    let layer_id = fresh_id("cue_layer", slide.local_ids());
    let prompt_id = fresh_id("cue_prompt", slide.local_ids());
    let continue_id = fresh_id("cue_continue", slide.local_ids());
    let name = match opt::<String>(C, p, "name")? {
        Some(name) => cue_name(C, name)?,
        None => {
            let n = match &slide.object(&oid).map(|(obj, _)| &obj.kind) {
                Some(ObjectKind::Video { cues, .. }) => {
                    cues.iter().filter(|cue| cue.pause).count() + 1
                }
                _ => 1,
            };
            format!("Question {n}")
        }
    };
    let cues = video_cues_mut(slide, &oid, C)?;
    if cues.len() >= MAX_CUES {
        return Err(bad(C, format!("a video may have at most {MAX_CUES} cues")));
    }
    let cue_id = fresh_id("cue", cues.iter().map(|cue| cue.id.as_str()));
    cues.push(Cue {
        id: cue_id.clone(),
        at_ms,
        name: name.clone(),
        pause: true,
    });
    cues.sort_by(|a, b| a.at_ms.total_cmp(&b.at_ms));

    let w = size.w as f32;
    let h = size.h as f32;
    if !w.is_finite() || !h.is_finite() || w <= 0.0 || h <= 0.0 {
        return Err(bad(C, "slide size is invalid"));
    }
    let margin = (w * 0.08).clamp(24.0, 160.0);
    let panel_w = (w - 2.0 * margin).max(80.0);
    let top = (h * 0.12).max(24.0);
    let button_h = (h * 0.08).clamp(36.0, 64.0);
    let gap = (h * 0.025).clamp(8.0, 24.0);
    let prompt_h = (h * 0.16).clamp(56.0, 140.0);

    let mut objects = Vec::new();
    let prompt = interaction_text(
        prompt_id.clone(),
        format!("{name} prompt"),
        "Question prompt".to_owned(),
        Rect {
            x: margin,
            y: top,
            w: panel_w,
            h: prompt_h,
        },
        &theme,
    );
    objects.push(prompt);
    let mut choice_ids = Vec::new();
    let choice_top = top + prompt_h + gap;
    for i in 0..3 {
        let id = fresh_id(
            &format!("cue_choice{}", i + 1),
            slide
                .local_ids()
                .chain(objects.iter().map(|object: &Object| object.id.as_str())),
        );
        choice_ids.push(id.clone());
        objects.push(interaction_button(
            id,
            format!("{name} choice {}", i + 1),
            format!("Choice {}", i + 1),
            Rect {
                x: margin,
                y: choice_top + (button_h + gap) * i as f32,
                w: panel_w,
                h: button_h,
            },
            i == 0,
            &theme,
        ));
    }
    let continue_y = (choice_top + (button_h + gap) * 3.0)
        .min(h - margin - button_h)
        .max(0.0);
    objects.push(interaction_button(
        continue_id.clone(),
        format!("{name} Continue"),
        "Continue".to_owned(),
        Rect {
            x: (w - margin - (w * 0.2).clamp(120.0, 200.0)).max(0.0),
            y: continue_y,
            w: (w * 0.2).clamp(120.0, 200.0),
            h: button_h,
        },
        false,
        &theme,
    ));
    let layer = Layer {
        id: layer_id.clone(),
        name: name.clone(),
        duration_ms: 5000,
        objects,
        triggers: Vec::new(),
        hide_other_layers: true,
        pause_base_timeline: false,
        prevent_base_clicks: true,
        // The video element draws above the canvas, so it must hide or it covers the question.
        hide_base_objects: true,
        hide_when_timeline_ends: false,
        allow_seek: SeekMode::Auto,
        revisit: RevisitMode::Auto,
        cue_points: Vec::new(),
        groups: Vec::new(),
    };
    slide.layers.push(layer);
    slide.triggers.push(interaction_trigger(
        Action::ShowLayer {
            layer: layer_id.clone(),
        },
        Event::CueReached {
            object: oid.clone(),
            cue: cue_id.clone(),
        },
    ));
    slide.triggers.push(interaction_trigger(
        Action::HideLayer {
            layer: Some(layer_id.clone()),
        },
        Event::Click {
            object: continue_id.clone(),
        },
    ));
    slide.triggers.push(interaction_trigger(
        Action::PlayMedia {
            object: oid.clone(),
        },
        Event::Click {
            object: continue_id.clone(),
        },
    ));
    keep_valid(s, was_valid)?;
    Ok(json!({
        "id": cue_id,
        "cue": cue_id,
        "layer": layer_id,
        "objects": choice_ids,
        "prompt": prompt_id,
        "continue": continue_id,
    }))
}

fn owner_duration(slide: &Slide, object: &str) -> Option<u32> {
    match slide.object(object) {
        Some((_, None)) => Some(slide.duration_ms),
        Some((_, Some(layer))) => slide.layer(layer).map(|layer| layer.duration_ms),
        None => None,
    }
}

fn video_cues_mut<'a>(slide: &'a mut Slide, object: &str, cmd: &str) -> Result<&'a mut Vec<Cue>> {
    let obj = find_object_mut(slide, object)?;
    match &mut obj.kind {
        ObjectKind::Video { cues, .. } => Ok(cues),
        _ => Err(bad(cmd, format!("object {object:?} is not a video"))),
    }
}

fn cue_at_ms(cmd: &str, p: &Json, key: &str) -> Result<Option<f64>> {
    let Some(value) = opt::<f64>(cmd, p, key)? else {
        return Ok(None);
    };
    if !value.is_finite() || value < 0.0 {
        return Err(bad(cmd, format!("`{key}` must be finite and at least 0")));
    }
    Ok(Some(value))
}

fn check_cue_time(cmd: &str, at_ms: f64, duration: u32) -> Result<()> {
    if !at_ms.is_finite() || at_ms < 0.0 {
        return Err(bad(cmd, "cue time must be finite and at least 0"));
    }
    if at_ms > f64::from(duration) {
        return Err(bad(
            cmd,
            format!("cue time {at_ms} ms is past the video's known duration ({duration} ms)"),
        ));
    }
    Ok(())
}

fn cue_name(cmd: &str, name: String) -> Result<String> {
    let name = name.trim().to_owned();
    if name.is_empty() {
        return Err(bad(cmd, "cue name cannot be empty"));
    }
    text(cmd, name, "name")
}
