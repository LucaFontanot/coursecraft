//! Trigger commands.

use coursecraft_course::{Slide, Trigger};
use serde_json::{Value as Json, json};

use super::{keep_valid, layer_mut, opt, req, slide_mut};
use crate::{EngineError, Result, Session};

fn triggers_mut<'a>(slide: &'a mut Slide, layer: Option<&str>) -> Result<&'a mut Vec<Trigger>> {
    match layer {
        Some(l) => Ok(&mut layer_mut(slide, l)?.triggers),
        None => Ok(&mut slide.triggers),
    }
}

pub(super) fn trigger_add(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "trigger.add";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let layer: Option<String> = opt(C, p, "layer")?;
    let trigger: Trigger = req(C, p, "trigger")?;
    let index: Option<usize> = opt(C, p, "index")?;
    let list = triggers_mut(slide_mut(s, &sid)?, layer.as_deref())?;
    let at = index.unwrap_or(list.len()).min(list.len());
    list.insert(at, trigger);
    keep_valid(s, was_valid)?;
    Ok(json!({ "index": at }))
}

pub(super) fn trigger_update(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "trigger.update";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let layer: Option<String> = opt(C, p, "layer")?;
    let index: usize = req(C, p, "index")?;
    let trigger: Trigger = req(C, p, "trigger")?;

    let list = triggers_mut(slide_mut(s, &sid)?, layer.as_deref())?;
    if index >= list.len() {
        return Err(EngineError::NotFound(format!(
            "no trigger at index {index}"
        )));
    }
    let target = list
        .get_mut(index)
        .ok_or_else(|| EngineError::NotFound(format!("no trigger at index {index}")))?;
    *target = trigger;
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

pub(super) fn trigger_move(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "trigger.move";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let layer: Option<String> = opt(C, p, "layer")?;
    let from: usize = req(C, p, "from")?;
    let to: usize = req(C, p, "to")?;

    let list = triggers_mut(slide_mut(s, &sid)?, layer.as_deref())?;
    if from >= list.len() {
        return Err(EngineError::NotFound(format!("no trigger at index {from}")));
    }
    let t = list.remove(from);
    let at = to.min(list.len());
    list.insert(at, t);
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

pub(super) fn trigger_delete(s: &mut Session, p: &Json) -> Result<Json> {
    let sid: String = req("trigger.delete", p, "slide")?;
    let layer: Option<String> = opt("trigger.delete", p, "layer")?;
    let index: usize = req("trigger.delete", p, "index")?;
    let list = triggers_mut(slide_mut(s, &sid)?, layer.as_deref())?;
    if index >= list.len() {
        return Err(EngineError::NotFound(format!(
            "no trigger at index {index}"
        )));
    }
    list.remove(index);
    Ok(Json::Null)
}
