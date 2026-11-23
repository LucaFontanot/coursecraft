//! Reversible structural deltas for undo/redo in [`crate::Session`].
//!
//! Every [`SlideDelta::apply`] and [`CourseHeaderPatch::apply`] swaps old and
//! new values in place so each delta becomes its own exact inverse with zero
//! allocations and byte-identical `course.json` restoration.
//!
//! `Course`, `Slide`, and `Layer` are destructured exhaustively (no `..`) so
//! adding any future model field fails to compile until delta support handles
//! it.

use std::collections::BTreeMap;
use std::mem::size_of;

use coursecraft_course::{
    BackgroundAudio, BankDraw, Color, Course, EditorSettings, FontFace, GlossaryEntry, Layer,
    MotionPath, NamedTextStyle, Object, ObjectKind, PlayerSettings, Question, QuizSettings,
    Resource, RevisitMode, Size, Slide, SlideCue, SlidePlayerOverrides, StateDef, Theme,
    Transition, Trigger, VariableDef,
};

use crate::{EngineError, Result};

/// Identifies one slide anywhere inside a [`Course`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SlideLoc {
    SceneSlide { scene: usize, slide: usize },
    Master(usize),
    Template(usize),
    BankSlide { bank: usize, slide: usize },
}

impl SlideLoc {
    /// Locate a slide by id across scenes, masters, templates, and banks.
    #[must_use]
    pub(crate) fn find(course: &Course, slide_id: &str) -> Option<Self> {
        for (scene_idx, scene) in course.scenes.iter().enumerate() {
            if let Some(slide_idx) = scene.slides.iter().position(|s| s.id == slide_id) {
                return Some(Self::SceneSlide {
                    scene: scene_idx,
                    slide: slide_idx,
                });
            }
        }
        if let Some(idx) = course.masters.iter().position(|s| s.id == slide_id) {
            return Some(Self::Master(idx));
        }
        if let Some(idx) = course.templates.iter().position(|s| s.id == slide_id) {
            return Some(Self::Template(idx));
        }
        for (bank_idx, bank) in course.banks.iter().enumerate() {
            if let Some(slide_idx) = bank.slides.iter().position(|s| s.id == slide_id) {
                return Some(Self::BankSlide {
                    bank: bank_idx,
                    slide: slide_idx,
                });
            }
        }
        None
    }
}

/// Borrow the slide at `loc`, returning `None` if any index is out of range.
#[must_use]
pub(crate) fn slide_at(course: &Course, loc: SlideLoc) -> Option<&Slide> {
    match loc {
        SlideLoc::SceneSlide { scene, slide } => {
            course.scenes.get(scene).and_then(|s| s.slides.get(slide))
        }
        SlideLoc::Master(idx) => course.masters.get(idx),
        SlideLoc::Template(idx) => course.templates.get(idx),
        SlideLoc::BankSlide { bank, slide } => {
            course.banks.get(bank).and_then(|b| b.slides.get(slide))
        }
    }
}

/// Mutably borrow the slide at `loc`, returning `None` if any index is out of range.
#[must_use]
pub(crate) fn slide_at_mut(course: &mut Course, loc: SlideLoc) -> Option<&mut Slide> {
    match loc {
        SlideLoc::SceneSlide { scene, slide } => course
            .scenes
            .get_mut(scene)
            .and_then(|s| s.slides.get_mut(slide)),
        SlideLoc::Master(idx) => course.masters.get_mut(idx),
        SlideLoc::Template(idx) => course.templates.get_mut(idx),
        SlideLoc::BankSlide { bank, slide } => course
            .banks
            .get_mut(bank)
            .and_then(|b| b.slides.get_mut(slide)),
    }
}

fn swap_if_some<T>(patch: &mut Option<T>, target: &mut T) {
    if let Some(prev) = patch.as_mut() {
        std::mem::swap(prev, target);
    }
}

fn diff_field<T: PartialEq>(before: T, after: &T, changed: &mut bool) -> Option<T> {
    if before != *after {
        *changed = true;
        Some(before)
    } else {
        None
    }
}

/// Sparse reversible patch over the 13 non-`objects`, non-`layers` fields of [`Slide`].
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SlideFieldPatch {
    pub(crate) id: Option<String>,
    pub(crate) title: Option<String>,
    pub(crate) duration_ms: Option<u32>,
    pub(crate) background: Option<Option<Color>>,
    pub(crate) triggers: Option<Vec<Trigger>>,
    pub(crate) cue_points: Option<Vec<SlideCue>>,
    pub(crate) question: Option<Option<Question>>,
    pub(crate) draw: Option<Option<BankDraw>>,
    pub(crate) transition: Option<Option<Transition>>,
    pub(crate) notes: Option<String>,
    pub(crate) master: Option<Option<String>>,
    pub(crate) revisit: Option<RevisitMode>,
    pub(crate) player: Option<Option<SlidePlayerOverrides>>,
}

impl SlideFieldPatch {
    /// Swap every present field in place with `slide`, turning `self` into its own inverse.
    pub(crate) fn apply(&mut self, slide: &mut Slide) {
        let Self {
            id: p_id,
            title: p_title,
            duration_ms: p_duration_ms,
            background: p_background,
            triggers: p_triggers,
            cue_points: p_cue_points,
            question: p_question,
            draw: p_draw,
            transition: p_transition,
            notes: p_notes,
            master: p_master,
            revisit: p_revisit,
            player: p_player,
        } = self;
        let Slide {
            id,
            title,
            duration_ms,
            background,
            objects: _,
            layers: _,
            triggers,
            cue_points,
            question,
            draw,
            transition,
            notes,
            master,
            revisit,
            player,
        } = slide;
        swap_if_some(p_id, id);
        swap_if_some(p_title, title);
        swap_if_some(p_duration_ms, duration_ms);
        swap_if_some(p_background, background);
        swap_if_some(p_triggers, triggers);
        swap_if_some(p_cue_points, cue_points);
        swap_if_some(p_question, question);
        swap_if_some(p_draw, draw);
        swap_if_some(p_transition, transition);
        swap_if_some(p_notes, notes);
        swap_if_some(p_master, master);
        swap_if_some(p_revisit, revisit);
        swap_if_some(p_player, player);
    }

    #[must_use]
    pub(crate) fn estimated_bytes(&self) -> usize {
        let mut total = size_of::<Self>();
        if let Some(id) = &self.id {
            total = total.saturating_add(id.capacity());
        }
        if let Some(title) = &self.title {
            total = total.saturating_add(title.capacity());
        }
        if let Some(Some(bg)) = &self.background {
            total = total.saturating_add(bg.0.capacity());
        }
        if let Some(triggers) = &self.triggers {
            total = total.saturating_add(estimate_vec_plain(triggers));
        }
        if let Some(cues) = &self.cue_points {
            total = total.saturating_add(estimate_vec_plain(cues));
        }
        if let Some(Some(q)) = &self.question {
            total = total.saturating_add(estimate_question_heap(q));
        }
        if let Some(Some(draw)) = &self.draw {
            total = total.saturating_add(draw.bank.capacity());
        }
        if let Some(notes) = &self.notes {
            total = total.saturating_add(notes.capacity());
        }
        if let Some(Some(master)) = &self.master {
            total = total.saturating_add(master.capacity());
        }
        total
    }
}

/// Reversible change inside a single [`Slide`].
///
/// Calling [`SlideDelta::apply`] swaps old and new state in place so the delta
/// becomes its own exact inverse.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SlideDelta {
    /// One object modified in place (`layer: None` = base layer, `Some(li)` = `slide.layers[li]`).
    ObjectModified {
        layer: Option<usize>,
        index: usize,
        object: Box<Object>,
    },
    /// One object inserted at `index` (applying removes it -> becomes `ObjectRemoved`).
    ObjectInserted { layer: Option<usize>, index: usize },
    /// One object removed from `index` (applying inserts it -> becomes `ObjectInserted`).
    ObjectRemoved {
        layer: Option<usize>,
        index: usize,
        object: Box<Object>,
    },
    /// One object moved from `from` to `to` (applying moves `to -> from` and swaps `from`/`to`).
    ObjectReordered {
        layer: Option<usize>,
        from: usize,
        to: usize,
    },
    /// One or more slide metadata fields changed without replacing objects or layers.
    Fields(Box<SlideFieldPatch>),
    /// Fallback when multiple objects or layers changed in one step.
    FullSlide(Box<Slide>),
}

impl SlideDelta {
    /// Compute the smallest reversible delta that transforms `after` back into `before`.
    /// Returns `None` when `before == *after`.
    #[must_use]
    pub(crate) fn diff(before: Slide, after: &Slide) -> Option<Self> {
        let Slide {
            id: b_id,
            title: b_title,
            duration_ms: b_duration_ms,
            background: b_background,
            objects: b_objects,
            layers: mut b_layers,
            triggers: b_triggers,
            cue_points: b_cue_points,
            question: b_question,
            draw: b_draw,
            transition: b_transition,
            notes: b_notes,
            master: b_master,
            revisit: b_revisit,
            player: b_player,
        } = before;
        let Slide {
            id: a_id,
            title: a_title,
            duration_ms: a_duration_ms,
            background: a_background,
            objects: a_objects,
            layers: a_layers,
            triggers: a_triggers,
            cue_points: a_cue_points,
            question: a_question,
            draw: a_draw,
            transition: a_transition,
            notes: a_notes,
            master: a_master,
            revisit: a_revisit,
            player: a_player,
        } = after;

        let meta_changed = b_id != *a_id
            || b_title != *a_title
            || b_duration_ms != *a_duration_ms
            || b_background != *a_background
            || b_triggers != *a_triggers
            || b_cue_points != *a_cue_points
            || b_question != *a_question
            || b_draw != *a_draw
            || b_transition != *a_transition
            || b_notes != *a_notes
            || b_master != *a_master
            || b_revisit != *a_revisit
            || b_player != *a_player;
        let objects_eq = b_objects == *a_objects;
        let layers_eq = b_layers == *a_layers;

        if !meta_changed && objects_eq && layers_eq {
            return None;
        }

        if meta_changed && objects_eq && layers_eq {
            let mut any = false;
            let patch = SlideFieldPatch {
                id: diff_field(b_id, a_id, &mut any),
                title: diff_field(b_title, a_title, &mut any),
                duration_ms: diff_field(b_duration_ms, a_duration_ms, &mut any),
                background: diff_field(b_background, a_background, &mut any),
                triggers: diff_field(b_triggers, a_triggers, &mut any),
                cue_points: diff_field(b_cue_points, a_cue_points, &mut any),
                question: diff_field(b_question, a_question, &mut any),
                draw: diff_field(b_draw, a_draw, &mut any),
                transition: diff_field(b_transition, a_transition, &mut any),
                notes: diff_field(b_notes, a_notes, &mut any),
                master: diff_field(b_master, a_master, &mut any),
                revisit: diff_field(b_revisit, a_revisit, &mut any),
                player: diff_field(b_player, a_player, &mut any),
            };
            return Some(Self::Fields(Box::new(patch)));
        }

        let b_objects = if !meta_changed && !objects_eq && layers_eq {
            match diff_object_vec(None, b_objects, a_objects) {
                Ok(delta) => return Some(delta),
                Err(objs) => objs,
            }
        } else {
            b_objects
        };

        if !meta_changed
            && objects_eq
            && !layers_eq
            && let Some(delta) = diff_single_layer_objects(&mut b_layers, a_layers)
        {
            return Some(delta);
        }

        Some(Self::FullSlide(Box::new(Slide {
            id: b_id,
            title: b_title,
            duration_ms: b_duration_ms,
            background: b_background,
            objects: b_objects,
            layers: b_layers,
            triggers: b_triggers,
            cue_points: b_cue_points,
            question: b_question,
            draw: b_draw,
            transition: b_transition,
            notes: b_notes,
            master: b_master,
            revisit: b_revisit,
            player: b_player,
        })))
    }

    /// Apply this delta in place, mutating `self` into its own exact inverse.
    ///
    /// On `Err`, both `slide` and `self` are left untouched.
    pub(crate) fn apply(&mut self, slide: &mut Slide) -> Result<()> {
        match self {
            Self::ObjectModified {
                layer,
                index,
                object,
            } => {
                let objects = slide_objects_mut(slide, *layer)?;
                let len = objects.len();
                let slot = objects.get_mut(*index).ok_or_else(|| {
                    EngineError::Invalid(format!(
                        "undo delta object index {index} out of bounds for len {len}"
                    ))
                })?;
                std::mem::swap(slot, object.as_mut());
                Ok(())
            }
            Self::ObjectInserted { layer, index } => {
                let layer = *layer;
                let index = *index;
                let objects = slide_objects_mut(slide, layer)?;
                if index >= objects.len() {
                    return Err(EngineError::Invalid(format!(
                        "undo delta object index {index} out of bounds for len {}",
                        objects.len()
                    )));
                }
                let removed = objects.remove(index);
                *self = Self::ObjectRemoved {
                    layer,
                    index,
                    object: Box::new(removed),
                };
                Ok(())
            }
            Self::ObjectRemoved { layer, index, .. } => {
                let layer = *layer;
                let index = *index;
                let objects = slide_objects_mut(slide, layer)?;
                if index > objects.len() {
                    return Err(EngineError::Invalid(format!(
                        "undo delta object index {index} out of bounds for len {}",
                        objects.len()
                    )));
                }
                let prev = std::mem::replace(self, Self::ObjectInserted { layer, index });
                if let Self::ObjectRemoved { object, .. } = prev {
                    objects.insert(index, *object);
                }
                Ok(())
            }
            Self::ObjectReordered { layer, from, to } => {
                let objects = slide_objects_mut(slide, *layer)?;
                if *from >= objects.len() || *to >= objects.len() {
                    return Err(EngineError::Invalid(format!(
                        "undo delta reorder ({from} -> {to}) out of bounds for len {}",
                        objects.len()
                    )));
                }
                if *from != *to {
                    let item = objects.remove(*to);
                    objects.insert(*from, item);
                    std::mem::swap(from, to);
                }
                Ok(())
            }
            Self::Fields(patch) => {
                patch.apply(slide);
                Ok(())
            }
            Self::FullSlide(prev) => {
                std::mem::swap(slide, prev.as_mut());
                Ok(())
            }
        }
    }

    /// Rough heap + stack footprint in bytes for memory-budget diagnostics.
    #[must_use]
    pub(crate) fn estimated_bytes(&self) -> usize {
        let base = size_of::<Self>();
        match self {
            Self::ObjectModified { object, .. } | Self::ObjectRemoved { object, .. } => {
                base.saturating_add(estimate_object(object))
            }
            Self::ObjectInserted { .. } | Self::ObjectReordered { .. } => base,
            Self::Fields(patch) => base.saturating_add(patch.estimated_bytes()),
            Self::FullSlide(slide) => base.saturating_add(estimate_slide(slide)),
        }
    }
}

fn slide_objects_mut(slide: &mut Slide, layer: Option<usize>) -> Result<&mut Vec<Object>> {
    match layer {
        None => Ok(&mut slide.objects),
        Some(li) => {
            let layer_len = slide.layers.len();
            slide
                .layers
                .get_mut(li)
                .map(|l| &mut l.objects)
                .ok_or_else(|| {
                    EngineError::Invalid(format!(
                        "undo delta layer index {li} out of bounds for len {layer_len}"
                    ))
                })
        }
    }
}

/// Check whether two [`Layer`] values are identical in all 12 non-`objects` fields.
/// Destructures [`Layer`] exhaustively without `..`.
fn layer_metadata_eq(before: &Layer, after: &Layer) -> bool {
    let Layer {
        id: b_id,
        name: b_name,
        duration_ms: b_duration_ms,
        objects: _,
        triggers: b_triggers,
        hide_other_layers: b_hide_other_layers,
        pause_base_timeline: b_pause_base_timeline,
        prevent_base_clicks: b_prevent_base_clicks,
        hide_base_objects: b_hide_base_objects,
        hide_when_timeline_ends: b_hide_when_timeline_ends,
        allow_seek: b_allow_seek,
        revisit: b_revisit,
        cue_points: b_cue_points,
    } = before;
    let Layer {
        id: a_id,
        name: a_name,
        duration_ms: a_duration_ms,
        objects: _,
        triggers: a_triggers,
        hide_other_layers: a_hide_other_layers,
        pause_base_timeline: a_pause_base_timeline,
        prevent_base_clicks: a_prevent_base_clicks,
        hide_base_objects: a_hide_base_objects,
        hide_when_timeline_ends: a_hide_when_timeline_ends,
        allow_seek: a_allow_seek,
        revisit: a_revisit,
        cue_points: a_cue_points,
    } = after;
    b_id == a_id
        && b_name == a_name
        && b_duration_ms == a_duration_ms
        && b_triggers == a_triggers
        && b_hide_other_layers == a_hide_other_layers
        && b_pause_base_timeline == a_pause_base_timeline
        && b_prevent_base_clicks == a_prevent_base_clicks
        && b_hide_base_objects == a_hide_base_objects
        && b_hide_when_timeline_ends == a_hide_when_timeline_ends
        && b_allow_seek == a_allow_seek
        && b_revisit == a_revisit
        && b_cue_points == a_cue_points
}

/// If exactly one layer differs and only in its `objects` vec, try to produce a
/// compact object-level [`SlideDelta`]. Restores `b_layers` intact on fallback.
fn diff_single_layer_objects(b_layers: &mut [Layer], a_layers: &[Layer]) -> Option<SlideDelta> {
    if b_layers.len() != a_layers.len() {
        return None;
    }
    let mut changed_idx = None;
    for (i, (b, a)) in b_layers.iter().zip(a_layers.iter()).enumerate() {
        if b != a {
            if changed_idx.is_some() {
                return None;
            }
            changed_idx = Some(i);
        }
    }
    let li = changed_idx?;
    let b_layer = b_layers.get_mut(li)?;
    let a_layer = a_layers.get(li)?;
    if !layer_metadata_eq(b_layer, a_layer) {
        return None;
    }
    let before_objs = std::mem::take(&mut b_layer.objects);
    match diff_object_vec(Some(li), before_objs, &a_layer.objects) {
        Ok(delta) => Some(delta),
        Err(objs) => {
            b_layer.objects = objs;
            None
        }
    }
}

/// Diff two object lists for single modify, insert, remove, or reorder.
/// On fallback, returns `Err(before)` so the caller retains ownership with zero clones.
fn diff_object_vec(
    layer: Option<usize>,
    mut before: Vec<Object>,
    after: &[Object],
) -> std::result::Result<SlideDelta, Vec<Object>> {
    if before.len() == after.len() {
        return diff_object_vec_same_len(layer, before, after);
    }
    if after.len() == before.len().saturating_add(1) && after.len() > before.len() {
        let idx = before
            .iter()
            .zip(after.iter())
            .position(|(b, a)| b != a)
            .unwrap_or(before.len());
        if before.get(idx..) == after.get(idx.saturating_add(1)..) {
            return Ok(SlideDelta::ObjectInserted { layer, index: idx });
        }
        return Err(before);
    }
    if before.len() == after.len().saturating_add(1) && before.len() > after.len() {
        let idx = before
            .iter()
            .zip(after.iter())
            .position(|(b, a)| b != a)
            .unwrap_or(after.len());
        if before.get(idx.saturating_add(1)..) == after.get(idx..) && idx < before.len() {
            let removed = before.swap_remove(idx);
            return Ok(SlideDelta::ObjectRemoved {
                layer,
                index: idx,
                object: Box::new(removed),
            });
        }
        return Err(before);
    }
    Err(before)
}

fn diff_object_vec_same_len(
    layer: Option<usize>,
    mut before: Vec<Object>,
    after: &[Object],
) -> std::result::Result<SlideDelta, Vec<Object>> {
    let mut first = None;
    let mut last = 0;
    for (i, (b, a)) in before.iter().zip(after.iter()).enumerate() {
        if b != a {
            if first.is_none() {
                first = Some(i);
            }
            last = i;
        }
    }
    let Some(a) = first else {
        return Err(before);
    };
    if a == last && a < before.len() {
        let prev_obj = before.swap_remove(a);
        return Ok(SlideDelta::ObjectModified {
            layer,
            index: a,
            object: Box::new(prev_obj),
        });
    }
    let b = last;
    if before.get(a) == after.get(b) && before.get(a.saturating_add(1)..=b) == after.get(a..b) {
        return Ok(SlideDelta::ObjectReordered {
            layer,
            from: a,
            to: b,
        });
    }
    if before.get(b) == after.get(a) && before.get(a..b) == after.get(a.saturating_add(1)..=b) {
        return Ok(SlideDelta::ObjectReordered {
            layer,
            from: b,
            to: a,
        });
    }
    Err(before)
}

/// Snapshot of the 17 non-slide header fields of [`Course`].
///
/// Capturing this clones zero scenes, masters, templates, or question banks.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CourseHeaderSnapshot {
    pub(crate) schema_version: u32,
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) slide_size: Size,
    pub(crate) variables: Vec<VariableDef>,
    pub(crate) start_slide: Option<String>,
    pub(crate) quiz: Option<QuizSettings>,
    pub(crate) player: PlayerSettings,
    pub(crate) theme: Theme,
    pub(crate) language: Option<String>,
    pub(crate) strings: BTreeMap<String, String>,
    pub(crate) fonts: Vec<FontFace>,
    pub(crate) background_audio: Option<BackgroundAudio>,
    pub(crate) text_styles: Vec<NamedTextStyle>,
    pub(crate) resources: Vec<Resource>,
    pub(crate) glossary: Vec<GlossaryEntry>,
    pub(crate) editor: EditorSettings,
}

impl CourseHeaderSnapshot {
    /// Clone all 17 header fields from `course` without touching slide collections.
    #[must_use]
    pub(crate) fn capture(course: &Course) -> Self {
        let Course {
            schema_version,
            id,
            title,
            slide_size,
            variables,
            scenes: _,
            masters: _,
            templates: _,
            banks: _,
            start_slide,
            quiz,
            player,
            theme,
            language,
            strings,
            fonts,
            background_audio,
            text_styles,
            resources,
            glossary,
            editor,
        } = course;
        Self {
            schema_version: *schema_version,
            id: id.clone(),
            title: title.clone(),
            slide_size: *slide_size,
            variables: variables.clone(),
            start_slide: start_slide.clone(),
            quiz: *quiz,
            player: player.clone(),
            theme: theme.clone(),
            language: language.clone(),
            strings: strings.clone(),
            fonts: fonts.clone(),
            background_audio: background_audio.clone(),
            text_styles: text_styles.clone(),
            resources: resources.clone(),
            glossary: glossary.clone(),
            editor: editor.clone(),
        }
    }

    /// Move all 17 header fields back into `course` (used for O(1) error rollback).
    pub(crate) fn restore(self, course: &mut Course) {
        let Self {
            schema_version: s_schema_version,
            id: s_id,
            title: s_title,
            slide_size: s_slide_size,
            variables: s_variables,
            start_slide: s_start_slide,
            quiz: s_quiz,
            player: s_player,
            theme: s_theme,
            language: s_language,
            strings: s_strings,
            fonts: s_fonts,
            background_audio: s_background_audio,
            text_styles: s_text_styles,
            resources: s_resources,
            glossary: s_glossary,
            editor: s_editor,
        } = self;
        let Course {
            schema_version,
            id,
            title,
            slide_size,
            variables,
            scenes: _,
            masters: _,
            templates: _,
            banks: _,
            start_slide,
            quiz,
            player,
            theme,
            language,
            strings,
            fonts,
            background_audio,
            text_styles,
            resources,
            glossary,
            editor,
        } = course;
        *schema_version = s_schema_version;
        *id = s_id;
        *title = s_title;
        *slide_size = s_slide_size;
        *variables = s_variables;
        *start_slide = s_start_slide;
        *quiz = s_quiz;
        *player = s_player;
        *theme = s_theme;
        *language = s_language;
        *strings = s_strings;
        *fonts = s_fonts;
        *background_audio = s_background_audio;
        *text_styles = s_text_styles;
        *resources = s_resources;
        *glossary = s_glossary;
        *editor = s_editor;
    }
}

/// Sparse reversible patch over the 17 header fields of [`Course`].
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CourseHeaderPatch {
    pub(crate) schema_version: Option<u32>,
    pub(crate) id: Option<String>,
    pub(crate) title: Option<String>,
    pub(crate) slide_size: Option<Size>,
    pub(crate) variables: Option<Vec<VariableDef>>,
    pub(crate) start_slide: Option<Option<String>>,
    pub(crate) quiz: Option<Option<QuizSettings>>,
    pub(crate) player: Option<PlayerSettings>,
    pub(crate) theme: Option<Theme>,
    pub(crate) language: Option<Option<String>>,
    pub(crate) strings: Option<BTreeMap<String, String>>,
    pub(crate) fonts: Option<Vec<FontFace>>,
    pub(crate) background_audio: Option<Option<BackgroundAudio>>,
    pub(crate) text_styles: Option<Vec<NamedTextStyle>>,
    pub(crate) resources: Option<Vec<Resource>>,
    pub(crate) glossary: Option<Vec<GlossaryEntry>>,
    pub(crate) editor: Option<EditorSettings>,
}

impl CourseHeaderPatch {
    /// Diff a pre-edit header snapshot against `after`, moving only changed fields.
    /// Returns `None` if all 17 header fields are equal.
    #[must_use]
    pub(crate) fn diff(before: CourseHeaderSnapshot, after: &Course) -> Option<Self> {
        let CourseHeaderSnapshot {
            schema_version: b_schema_version,
            id: b_id,
            title: b_title,
            slide_size: b_slide_size,
            variables: b_variables,
            start_slide: b_start_slide,
            quiz: b_quiz,
            player: b_player,
            theme: b_theme,
            language: b_language,
            strings: b_strings,
            fonts: b_fonts,
            background_audio: b_background_audio,
            text_styles: b_text_styles,
            resources: b_resources,
            glossary: b_glossary,
            editor: b_editor,
        } = before;
        let Course {
            schema_version: a_schema_version,
            id: a_id,
            title: a_title,
            slide_size: a_slide_size,
            variables: a_variables,
            scenes: _,
            masters: _,
            templates: _,
            banks: _,
            start_slide: a_start_slide,
            quiz: a_quiz,
            player: a_player,
            theme: a_theme,
            language: a_language,
            strings: a_strings,
            fonts: a_fonts,
            background_audio: a_background_audio,
            text_styles: a_text_styles,
            resources: a_resources,
            glossary: a_glossary,
            editor: a_editor,
        } = after;

        let mut changed = false;
        let patch = Self {
            schema_version: diff_field(b_schema_version, a_schema_version, &mut changed),
            id: diff_field(b_id, a_id, &mut changed),
            title: diff_field(b_title, a_title, &mut changed),
            slide_size: diff_field(b_slide_size, a_slide_size, &mut changed),
            variables: diff_field(b_variables, a_variables, &mut changed),
            start_slide: diff_field(b_start_slide, a_start_slide, &mut changed),
            quiz: diff_field(b_quiz, a_quiz, &mut changed),
            player: diff_field(b_player, a_player, &mut changed),
            theme: diff_field(b_theme, a_theme, &mut changed),
            language: diff_field(b_language, a_language, &mut changed),
            strings: diff_field(b_strings, a_strings, &mut changed),
            fonts: diff_field(b_fonts, a_fonts, &mut changed),
            background_audio: diff_field(b_background_audio, a_background_audio, &mut changed),
            text_styles: diff_field(b_text_styles, a_text_styles, &mut changed),
            resources: diff_field(b_resources, a_resources, &mut changed),
            glossary: diff_field(b_glossary, a_glossary, &mut changed),
            editor: diff_field(b_editor, a_editor, &mut changed),
        };
        changed.then_some(patch)
    }

    /// Swap every present field in place with `course`, turning `self` into its own inverse.
    pub(crate) fn apply(&mut self, course: &mut Course) {
        let Self {
            schema_version: p_schema_version,
            id: p_id,
            title: p_title,
            slide_size: p_slide_size,
            variables: p_variables,
            start_slide: p_start_slide,
            quiz: p_quiz,
            player: p_player,
            theme: p_theme,
            language: p_language,
            strings: p_strings,
            fonts: p_fonts,
            background_audio: p_background_audio,
            text_styles: p_text_styles,
            resources: p_resources,
            glossary: p_glossary,
            editor: p_editor,
        } = self;
        let Course {
            schema_version,
            id,
            title,
            slide_size,
            variables,
            scenes: _,
            masters: _,
            templates: _,
            banks: _,
            start_slide,
            quiz,
            player,
            theme,
            language,
            strings,
            fonts,
            background_audio,
            text_styles,
            resources,
            glossary,
            editor,
        } = course;
        swap_if_some(p_schema_version, schema_version);
        swap_if_some(p_id, id);
        swap_if_some(p_title, title);
        swap_if_some(p_slide_size, slide_size);
        swap_if_some(p_variables, variables);
        swap_if_some(p_start_slide, start_slide);
        swap_if_some(p_quiz, quiz);
        swap_if_some(p_player, player);
        swap_if_some(p_theme, theme);
        swap_if_some(p_language, language);
        swap_if_some(p_strings, strings);
        swap_if_some(p_fonts, fonts);
        swap_if_some(p_background_audio, background_audio);
        swap_if_some(p_text_styles, text_styles);
        swap_if_some(p_resources, resources);
        swap_if_some(p_glossary, glossary);
        swap_if_some(p_editor, editor);
    }

    /// Rough heap + stack footprint in bytes for memory-budget diagnostics.
    #[must_use]
    pub(crate) fn estimated_bytes(&self) -> usize {
        let mut total = size_of::<Self>();
        if let Some(id) = &self.id {
            total = total.saturating_add(id.capacity());
        }
        if let Some(title) = &self.title {
            total = total.saturating_add(title.capacity());
        }
        if let Some(vars) = &self.variables {
            total = total.saturating_add(estimate_vec_plain(vars));
        }
        if let Some(Some(start)) = &self.start_slide {
            total = total.saturating_add(start.capacity());
        }
        if let Some(player) = &self.player
            && let Some(logo) = &player.logo
        {
            total = total.saturating_add(logo.capacity());
        }
        if let Some(theme) = &self.theme {
            total = total.saturating_add(theme.font.capacity());
            total = total.saturating_add(estimate_vec_plain(&theme.palette));
        }
        if let Some(Some(lang)) = &self.language {
            total = total.saturating_add(lang.capacity());
        }
        if let Some(strings) = &self.strings {
            for (k, v) in strings {
                total = total
                    .saturating_add(k.capacity())
                    .saturating_add(v.capacity())
                    .saturating_add(size_of::<(String, String)>());
            }
        }
        if let Some(fonts) = &self.fonts {
            total = total.saturating_add(estimate_vec_plain(fonts));
        }
        if let Some(Some(audio)) = &self.background_audio {
            total = total.saturating_add(audio.src.capacity());
        }
        if let Some(styles) = &self.text_styles {
            total = total.saturating_add(estimate_vec_plain(styles));
        }
        if let Some(resources) = &self.resources {
            total = total.saturating_add(estimate_vec_plain(resources));
        }
        if let Some(glossary) = &self.glossary {
            total = total.saturating_add(estimate_vec_plain(glossary));
        }
        if let Some(editor) = &self.editor {
            total = total.saturating_add(estimate_vec_plain(&editor.guides));
        }
        total
    }
}

// ------------------------------------------------------------------ size estimation

fn estimate_vec_plain<T>(vec: &[T]) -> usize {
    vec.len().saturating_mul(size_of::<T>())
}

fn estimate_opt_string(opt: &Option<String>) -> usize {
    opt.as_ref().map_or(0, String::capacity)
}

fn estimate_question_heap(q: &Question) -> usize {
    q.prompt
        .capacity()
        .saturating_add(estimate_vec_plain(&q.choices))
        .saturating_add(estimate_vec_plain(&q.accepted))
        .saturating_add(estimate_vec_plain(&q.pairs))
        .saturating_add(estimate_vec_plain(&q.regions))
        .saturating_add(estimate_vec_plain(&q.drag_items))
        .saturating_add(estimate_vec_plain(&q.drop_targets))
}

fn estimate_object_kind_heap(kind: &ObjectKind) -> usize {
    match kind {
        ObjectKind::Text {
            text,
            runs,
            paragraphs,
            text_style,
            ..
        } => text
            .capacity()
            .saturating_add(estimate_vec_plain(runs))
            .saturating_add(estimate_vec_plain(paragraphs))
            .saturating_add(estimate_opt_string(text_style)),
        ObjectKind::Shape { text, adjust, .. } => {
            let text_bytes = text.as_ref().map_or(0, |t| {
                t.text
                    .capacity()
                    .saturating_add(estimate_vec_plain(&t.runs))
                    .saturating_add(estimate_vec_plain(&t.paragraphs))
            });
            text_bytes.saturating_add(estimate_vec_plain(adjust))
        }
        ObjectKind::Image { src, alt, .. } => src.capacity().saturating_add(alt.capacity()),
        ObjectKind::Audio { src, captions, .. } => {
            src.capacity().saturating_add(estimate_opt_string(captions))
        }
        ObjectKind::Video {
            src,
            captions,
            cues,
            poster,
            ..
        } => src
            .capacity()
            .saturating_add(estimate_opt_string(captions))
            .saturating_add(estimate_vec_plain(cues))
            .saturating_add(estimate_opt_string(poster)),
        ObjectKind::Table { columns, rows, .. } => {
            let row_bytes = rows.iter().fold(estimate_vec_plain(rows), |acc, r| {
                acc.saturating_add(estimate_vec_plain(&r.cells))
            });
            estimate_vec_plain(columns).saturating_add(row_bytes)
        }
        ObjectKind::Path { path } => {
            estimate_vec_plain(&path.points).saturating_add(estimate_vec_plain(&path.subpaths))
        }
        ObjectKind::Marker {
            title,
            body,
            runs,
            paragraphs,
            ..
        } => title
            .capacity()
            .saturating_add(body.capacity())
            .saturating_add(estimate_vec_plain(runs))
            .saturating_add(estimate_vec_plain(paragraphs)),
        ObjectKind::Button { label, .. } => label.capacity(),
        ObjectKind::RadioGroup { variable, options } => variable
            .capacity()
            .saturating_add(estimate_vec_plain(options)),
        ObjectKind::Lottie { src, .. } => src.capacity(),
        ObjectKind::TextEntry {
            variable,
            placeholder,
            ..
        } => variable.capacity().saturating_add(placeholder.capacity()),
        ObjectKind::Checkbox { variable, label } => {
            variable.capacity().saturating_add(label.capacity())
        }
        ObjectKind::Slider { variable, .. }
        | ObjectKind::Dial { variable, .. }
        | ObjectKind::ProgressMeter { variable, .. } => variable.capacity(),
        ObjectKind::Web { .. } | ObjectKind::ScrollPanel { .. } | ObjectKind::Timer { .. } => 0,
    }
}

pub(crate) fn estimate_object(obj: &Object) -> usize {
    let state_bytes = obj.states.iter().fold(
        obj.states.capacity().saturating_mul(size_of::<StateDef>()),
        |acc, s| {
            acc.saturating_add(s.name.capacity())
                .saturating_add(estimate_opt_string(&s.text))
                .saturating_add(estimate_opt_string(&s.src))
                .saturating_add(estimate_vec_plain(&s.extras))
        },
    );
    let path_bytes = obj.motion_paths.iter().fold(
        obj.motion_paths
            .capacity()
            .saturating_mul(size_of::<MotionPath>()),
        |acc, mp| acc.saturating_add(estimate_vec_plain(&mp.points)),
    );
    size_of::<Object>()
        .saturating_add(obj.id.capacity())
        .saturating_add(obj.name.capacity())
        .saturating_add(obj.initial_state.capacity())
        .saturating_add(estimate_opt_string(&obj.description))
        .saturating_add(estimate_opt_string(&obj.group))
        .saturating_add(estimate_opt_string(&obj.panel))
        .saturating_add(state_bytes)
        .saturating_add(path_bytes)
        .saturating_add(estimate_object_kind_heap(&obj.kind))
}

fn estimate_layer(layer: &Layer) -> usize {
    let obj_bytes = layer
        .objects
        .iter()
        .fold(0usize, |acc, o| acc.saturating_add(estimate_object(o)));
    size_of::<Layer>()
        .saturating_add(layer.id.capacity())
        .saturating_add(layer.name.capacity())
        .saturating_add(obj_bytes)
        .saturating_add(estimate_vec_plain(&layer.triggers))
        .saturating_add(estimate_vec_plain(&layer.cue_points))
}

pub(crate) fn estimate_slide(slide: &Slide) -> usize {
    let obj_bytes = slide
        .objects
        .iter()
        .fold(0usize, |acc, o| acc.saturating_add(estimate_object(o)));
    let layer_bytes = slide
        .layers
        .iter()
        .fold(0usize, |acc, l| acc.saturating_add(estimate_layer(l)));
    size_of::<Slide>()
        .saturating_add(slide.id.capacity())
        .saturating_add(slide.title.capacity())
        .saturating_add(slide.notes.capacity())
        .saturating_add(estimate_opt_string(&slide.master))
        .saturating_add(obj_bytes)
        .saturating_add(layer_bytes)
        .saturating_add(estimate_vec_plain(&slide.triggers))
        .saturating_add(estimate_vec_plain(&slide.cue_points))
        .saturating_add(slide.question.as_ref().map_or(0, estimate_question_heap))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::Session;
    use coursecraft_course::{
        Action, Autofit, Choice, Event, FontStyle, Guide, QuestionBank, QuestionKind, Rect,
        ResourceTarget, Scoring, SeekMode, ShapeKind, SlideTarget, TextStyle, Timing,
        TransitionEffect, Value,
    };

    fn sample_object(id: &str, x: f32) -> Object {
        Object {
            id: id.to_owned(),
            name: format!("Obj {id}"),
            kind: ObjectKind::Shape {
                shape: ShapeKind::Rectangle,
                fill: Color("#2F6FEB".to_owned()),
                fill_style: None,
                outline: None,
                text: None,
                adjust: Vec::new(),
            },
            frame: Rect {
                x,
                y: 20.0,
                w: 100.0,
                h: 50.0,
            },
            timing: Timing::default(),
            states: Vec::new(),
            initial_state: "normal".to_owned(),
            entrance: None,
            exit: None,
            motion_paths: Vec::new(),
            focus_order: None,
            decorative: false,
            description: None,
            group: None,
            panel: None,
            rotation: 0.0,
            flip_h: false,
            flip_v: false,
            opacity: 1.0,
            shadow: None,
            glow: None,
            drag: None,
            locked: false,
            editor_hidden: false,
        }
    }

    fn sample_layer(id: &str, objects: Vec<Object>) -> Layer {
        Layer {
            id: id.to_owned(),
            name: format!("Layer {id}"),
            duration_ms: 5000,
            objects,
            triggers: Vec::new(),
            hide_other_layers: true,
            pause_base_timeline: false,
            prevent_base_clicks: false,
            hide_base_objects: false,
            hide_when_timeline_ends: false,
            allow_seek: SeekMode::Auto,
            revisit: RevisitMode::Auto,
            cue_points: Vec::new(),
        }
    }

    fn sample_slide() -> Slide {
        Slide {
            id: "slide1".to_owned(),
            title: "Slide 1".to_owned(),
            duration_ms: 5000,
            background: None,
            objects: vec![
                sample_object("o1", 10.0),
                sample_object("o2", 120.0),
                sample_object("o3", 230.0),
                sample_object("o4", 340.0),
            ],
            layers: vec![sample_layer(
                "layer1",
                vec![
                    sample_object("lo1", 10.0),
                    sample_object("lo2", 120.0),
                    sample_object("lo3", 230.0),
                    sample_object("lo4", 340.0),
                ],
            )],
            triggers: Vec::new(),
            cue_points: Vec::new(),
            question: None,
            draw: None,
            transition: None,
            notes: String::new(),
            master: None,
            revisit: RevisitMode::Auto,
            player: None,
        }
    }

    fn slide_json(slide: &Slide) -> String {
        serde_json::to_string_pretty(slide).unwrap()
    }

    fn assert_slide_roundtrip(before: Slide, mut after: Slide) -> SlideDelta {
        let before_json = slide_json(&before);
        let after_json = slide_json(&after);
        assert_ne!(before_json, after_json);

        let mut delta = SlideDelta::diff(before, &after).expect("expected Some(delta)");
        let returned_variant = delta.clone();

        delta.apply(&mut after).expect("undo apply must succeed");
        assert_eq!(
            slide_json(&after),
            before_json,
            "undo must restore exact JSON"
        );

        delta.apply(&mut after).expect("redo apply must succeed");
        assert_eq!(
            slide_json(&after),
            after_json,
            "redo must restore exact JSON"
        );

        returned_variant
    }

    #[test]
    fn slide_loc_find_and_lookup_all_collections() {
        let mut course = Session::blank("Loc Test").course;
        let mut master = sample_slide();
        master.id = "master1".to_owned();
        course.masters.push(master);

        let mut tmpl = sample_slide();
        tmpl.id = "tmpl1".to_owned();
        course.templates.push(tmpl);

        let mut bank_slide = sample_slide();
        bank_slide.id = "bank_q1".to_owned();
        course.banks.push(QuestionBank {
            id: "bank1".to_owned(),
            title: "Bank 1".to_owned(),
            slides: vec![bank_slide],
        });

        let scene_loc = SlideLoc::find(&course, "slide1").unwrap();
        assert_eq!(scene_loc, SlideLoc::SceneSlide { scene: 0, slide: 0 });
        assert_eq!(slide_at(&course, scene_loc).unwrap().id, "slide1");

        let master_loc = SlideLoc::find(&course, "master1").unwrap();
        assert_eq!(master_loc, SlideLoc::Master(0));
        assert_eq!(slide_at(&course, master_loc).unwrap().id, "master1");

        let tmpl_loc = SlideLoc::find(&course, "tmpl1").unwrap();
        assert_eq!(tmpl_loc, SlideLoc::Template(0));
        assert_eq!(slide_at(&course, tmpl_loc).unwrap().id, "tmpl1");

        let bank_loc = SlideLoc::find(&course, "bank_q1").unwrap();
        assert_eq!(bank_loc, SlideLoc::BankSlide { bank: 0, slide: 0 });
        assert_eq!(slide_at(&course, bank_loc).unwrap().id, "bank_q1");

        slide_at_mut(&mut course, bank_loc).unwrap().title = "Edited Bank".to_owned();
        assert_eq!(slide_at(&course, bank_loc).unwrap().title, "Edited Bank");

        assert_eq!(SlideLoc::find(&course, "missing"), None);
        assert!(
            slide_at(
                &course,
                SlideLoc::SceneSlide {
                    scene: 99,
                    slide: 0
                }
            )
            .is_none()
        );
        assert!(slide_at(&course, SlideLoc::Master(99)).is_none());
        assert!(slide_at(&course, SlideLoc::Template(99)).is_none());
        assert!(slide_at(&course, SlideLoc::BankSlide { bank: 99, slide: 0 }).is_none());
        assert!(slide_at_mut(&mut course, SlideLoc::Master(99)).is_none());
    }

    /// U-01: Single-object modify / insert / remove / reorder on base layer & slide layers.
    #[test]
    fn u_01_slide_delta_single_object_ops_base_and_layer() {
        let base = sample_slide();
        assert_eq!(SlideDelta::diff(base.clone(), &base), None);

        fn objects_mut(s: &mut Slide, layer: Option<usize>) -> &mut Vec<Object> {
            match layer {
                None => &mut s.objects,
                Some(li) => &mut s.layers[li].objects,
            }
        }

        for layer in [None, Some(0usize)] {
            // 1. Modify
            let mut after_mod = base.clone();
            objects_mut(&mut after_mod, layer)[1].frame.x = 777.0;
            let delta = assert_slide_roundtrip(base.clone(), after_mod);
            assert!(matches!(
                delta,
                SlideDelta::ObjectModified {
                    layer: l,
                    index: 1,
                    ..
                } if l == layer
            ));
            assert!(delta.estimated_bytes() < 4096);

            // 2. Insert at start, middle, end
            for insert_idx in [0usize, 2, 4] {
                let mut after_ins = base.clone();
                objects_mut(&mut after_ins, layer)
                    .insert(insert_idx, sample_object("new_obj", 555.0));
                let delta = assert_slide_roundtrip(base.clone(), after_ins);
                assert_eq!(
                    delta,
                    SlideDelta::ObjectInserted {
                        layer,
                        index: insert_idx
                    }
                );
                assert!(delta.estimated_bytes() < 128);
            }

            // 3. Remove from start, middle, end
            for remove_idx in [0usize, 2, 3] {
                let mut after_rem = base.clone();
                let removed = objects_mut(&mut after_rem, layer).remove(remove_idx);
                let delta = assert_slide_roundtrip(base.clone(), after_rem);
                assert!(matches!(
                    delta,
                    SlideDelta::ObjectRemoved {
                        layer: l,
                        index,
                        ref object
                    } if l == layer && index == remove_idx && **object == removed
                ));
            }

            // 4. Reorder forward (from < to): move index 0 -> index 3
            let mut after_fwd = base.clone();
            let item = objects_mut(&mut after_fwd, layer).remove(0);
            objects_mut(&mut after_fwd, layer).insert(3, item);
            let delta = assert_slide_roundtrip(base.clone(), after_fwd);
            assert_eq!(
                delta,
                SlideDelta::ObjectReordered {
                    layer,
                    from: 0,
                    to: 3
                }
            );
            assert!(delta.estimated_bytes() < 128);

            // 5. Reorder backward (from > to): move index 3 -> index 1
            let mut after_bwd = base.clone();
            let item = objects_mut(&mut after_bwd, layer).remove(3);
            objects_mut(&mut after_bwd, layer).insert(1, item);
            let delta = assert_slide_roundtrip(base.clone(), after_bwd);
            assert_eq!(
                delta,
                SlideDelta::ObjectReordered {
                    layer,
                    from: 3,
                    to: 1
                }
            );
            assert!(delta.estimated_bytes() < 128);

            // 6. Adjacent swap: index 1 <-> 2
            let mut after_swap = base.clone();
            objects_mut(&mut after_swap, layer).swap(1, 2);
            let delta = assert_slide_roundtrip(base.clone(), after_swap);
            assert!(matches!(
                delta,
                SlideDelta::ObjectReordered { layer: l, .. } if l == layer
            ));

            // 7. Multi-object change falls back to FullSlide
            let mut after_multi = base.clone();
            objects_mut(&mut after_multi, layer)[0].frame.x = 1.0;
            objects_mut(&mut after_multi, layer)[2].frame.x = 2.0;
            let delta = assert_slide_roundtrip(base.clone(), after_multi);
            assert!(matches!(delta, SlideDelta::FullSlide(_)));
        }

        // Layer metadata change falls back to FullSlide and roundtrips cleanly.
        let mut after_layer_meta = base.clone();
        after_layer_meta.layers[0].name = "Renamed Layer".to_owned();
        let delta = assert_slide_roundtrip(base, after_layer_meta);
        assert!(matches!(delta, SlideDelta::FullSlide(_)));
    }

    fn assert_slide_fields_roundtrip(base: &Slide, mutate: impl FnOnce(&mut Slide)) {
        let mut s = base.clone();
        mutate(&mut s);
        let delta = assert_slide_roundtrip(base.clone(), s);
        assert!(
            matches!(delta, SlideDelta::Fields(_)),
            "expected SlideDelta::Fields, got {delta:?}"
        );
        assert!(delta.estimated_bytes() > 0);
    }

    /// U-02: Mutate each of the 15 `Slide` fields individually and verify delta + exact roundtrip.
    #[test]
    fn u_02_slide_delta_all_15_fields() {
        let base = sample_slide();

        // 1. id
        assert_slide_fields_roundtrip(&base, |s| s.id = "slide_renamed".to_owned());
        // 2. title
        assert_slide_fields_roundtrip(&base, |s| s.title = "Updated Title".to_owned());
        // 3. duration_ms
        assert_slide_fields_roundtrip(&base, |s| s.duration_ms = 12_500);
        // 4. background
        assert_slide_fields_roundtrip(&base, |s| s.background = Some(Color("#112233".to_owned())));

        // 5. objects
        let mut s = base.clone();
        s.objects.push(sample_object("o_extra", 500.0));
        assert!(matches!(
            assert_slide_roundtrip(base.clone(), s),
            SlideDelta::ObjectInserted {
                layer: None,
                index: 4
            }
        ));

        // 6. layers (adding a new layer -> FullSlide; editing object in layer -> ObjectModified)
        let mut s = base.clone();
        s.layers.push(sample_layer("layer2", Vec::new()));
        assert!(matches!(
            assert_slide_roundtrip(base.clone(), s),
            SlideDelta::FullSlide(_)
        ));

        let mut s = base.clone();
        s.layers[0].objects[0].name = "Renamed in Layer".to_owned();
        assert!(matches!(
            assert_slide_roundtrip(base.clone(), s),
            SlideDelta::ObjectModified {
                layer: Some(0),
                index: 0,
                ..
            }
        ));

        // 7. triggers
        assert_slide_fields_roundtrip(&base, |s| {
            s.triggers.push(Trigger {
                action: Action::JumpToSlide {
                    target: SlideTarget::Next,
                },
                when: Event::TimelineEnds { layer: None },
                condition: None,
                disabled: false,
            });
        });
        // 8. cue_points
        assert_slide_fields_roundtrip(&base, |s| {
            s.cue_points.push(SlideCue {
                id: "cue1".to_owned(),
                name: "Intro".to_owned(),
                at_ms: 1500,
            });
        });
        // 9. question
        assert_slide_fields_roundtrip(&base, |s| {
            s.question = Some(Question {
                kind: QuestionKind::MultipleChoice,
                prompt: "2 + 2?".to_owned(),
                choices: vec![Choice {
                    id: "a".to_owned(),
                    text: "4".to_owned(),
                    correct: true,
                    points: 10,
                }],
                points: 10,
                scoring: Scoring::AllOrNothing,
                attempts: 1,
                shuffle: false,
                correct_layer: None,
                incorrect_layer: None,
                try_again_layer: None,
                accepted: Vec::new(),
                case_sensitive: false,
                pairs: Vec::new(),
                hotspot_image: None,
                regions: Vec::new(),
                markers: 1,
                drag_items: Vec::new(),
                drop_targets: Vec::new(),
                snap_to_center: true,
                return_if_wrong: false,
                one_per_target: true,
                correct_when: None,
            });
        });
        // 10. draw
        assert_slide_fields_roundtrip(&base, |s| {
            s.draw = Some(BankDraw {
                bank: "bank1".to_owned(),
                count: 3,
            });
        });
        // 11. transition
        assert_slide_fields_roundtrip(&base, |s| {
            s.transition = Some(Transition {
                effect: TransitionEffect::Fade,
                duration_ms: 400,
            });
        });
        // 12. notes
        assert_slide_fields_roundtrip(&base, |s| s.notes = "Speaker notes here.".to_owned());
        // 13. master
        assert_slide_fields_roundtrip(&base, |s| s.master = Some("master1".to_owned()));
        // 14. revisit
        assert_slide_fields_roundtrip(&base, |s| s.revisit = RevisitMode::Reset);
        // 15. player
        assert_slide_fields_roundtrip(&base, |s| {
            s.player = Some(SlidePlayerOverrides {
                show_bar: Some(false),
                next_requires: None,
                show_nav: Some(false),
                show_seekbar: None,
                show_menu: None,
            });
        });
    }

    fn assert_header_field_roundtrip(base: &Course, mutate: impl FnOnce(&mut Course)) {
        let before_json = base.to_json().unwrap();
        let mut course = base.clone();
        let snap = CourseHeaderSnapshot::capture(&course);
        mutate(&mut course);
        let after_json = course.to_json().unwrap();
        assert_ne!(before_json, after_json);

        // Verify restore()
        let mut for_restore = course.clone();
        snap.clone().restore(&mut for_restore);
        assert_eq!(for_restore.to_json().unwrap(), before_json);

        // Verify diff() + apply() undo -> redo
        let mut patch = CourseHeaderPatch::diff(snap, &course).expect("expected header patch");
        assert!(patch.estimated_bytes() > 0);

        patch.apply(&mut course);
        assert_eq!(
            course.to_json().unwrap(),
            before_json,
            "header undo mismatch"
        );

        patch.apply(&mut course);
        assert_eq!(
            course.to_json().unwrap(),
            after_json,
            "header redo mismatch"
        );
    }

    /// U-03: Mutate each of the 17 `Course` header fields individually and verify exact roundtrip.
    #[test]
    fn u_03_course_header_patch_all_17_fields() {
        let base = Session::blank("Header Test").course;
        let unchanged_snap = CourseHeaderSnapshot::capture(&base);
        assert_eq!(CourseHeaderPatch::diff(unchanged_snap, &base), None);

        // 1. schema_version
        assert_header_field_roundtrip(&base, |c| c.schema_version = 2);
        // 2. id
        assert_header_field_roundtrip(&base, |c| c.id = "course_v2".to_owned());
        // 3. title
        assert_header_field_roundtrip(&base, |c| c.title = "Renamed Course".to_owned());
        // 4. slide_size
        assert_header_field_roundtrip(&base, |c| c.slide_size = Size { w: 1920, h: 1080 });
        // 5. variables
        assert_header_field_roundtrip(&base, |c| {
            c.variables.push(VariableDef {
                name: "score".to_owned(),
                value: Value::Number(42.0),
            });
        });
        // 6. start_slide
        assert_header_field_roundtrip(&base, |c| c.start_slide = Some("slide1".to_owned()));
        // 7. quiz
        assert_header_field_roundtrip(&base, |c| {
            c.quiz = Some(QuizSettings {
                passing_percent: 80,
            });
        });
        // 8. player
        assert_header_field_roundtrip(&base, |c| {
            c.player.show_bar = false;
            c.player.logo = Some("logo.png".to_owned());
        });
        // 9. theme
        assert_header_field_roundtrip(&base, |c| {
            c.theme.accent = Color("#FF0000".to_owned());
            c.theme.font = "Lato".to_owned();
        });
        // 10. language
        assert_header_field_roundtrip(&base, |c| c.language = Some("fr-CA".to_owned()));
        // 11. strings
        assert_header_field_roundtrip(&base, |c| {
            c.strings.insert("play".to_owned(), "Lire".to_owned());
        });
        // 12. fonts
        assert_header_field_roundtrip(&base, |c| {
            c.fonts.push(FontFace {
                family: "Lato".to_owned(),
                style: FontStyle::Regular,
                src: "fonts/lato-regular.ttf".to_owned(),
            });
        });
        // 13. background_audio
        assert_header_field_roundtrip(&base, |c| {
            c.background_audio = Some(BackgroundAudio {
                src: "audio/bg.mp3".to_owned(),
                volume: 0.4,
                looping: true,
            });
        });
        // 14. text_styles
        assert_header_field_roundtrip(&base, |c| {
            c.text_styles.push(NamedTextStyle {
                id: "heading".to_owned(),
                name: "Heading".to_owned(),
                style: TextStyle::default(),
                paragraphs: None,
            });
        });
        // 15. resources
        assert_header_field_roundtrip(&base, |c| {
            c.resources.push(Resource {
                id: "res1".to_owned(),
                title: "Handbook".to_owned(),
                target: ResourceTarget::Url {
                    url: "https://example.com".to_owned(),
                },
            });
        });
        // 16. glossary
        assert_header_field_roundtrip(&base, |c| {
            c.glossary.push(GlossaryEntry {
                id: "g1".to_owned(),
                term: "LMS".to_owned(),
                definition: "Learning Management System".to_owned(),
            });
        });
        // 17. editor
        assert_header_field_roundtrip(&base, |c| {
            c.editor.show_rulers = true;
            c.editor.guides.push(Guide {
                vertical: true,
                at: 640.0,
            });
        });

        // Sanity check that `Autofit` import isn't needed elsewhere; verify a Text object estimate too.
        let mut text_obj = sample_object("t1", 0.0);
        text_obj.kind = ObjectKind::Text {
            text: "Hello CourseCraft".to_owned(),
            style: TextStyle::default(),
            runs: Vec::new(),
            paragraphs: Vec::new(),
            autofit: Autofit::None,
            text_style: None,
            outline: None,
        };
        assert!(estimate_object(&text_obj) > size_of::<Object>());
    }

    /// U-06: Hostile out-of-bounds `SlideDelta::apply` returns `Err(Invalid)` with zero panics
    /// and leaves both the slide and the delta untouched.
    #[test]
    fn u_06_slide_delta_hostile_out_of_bounds_no_panic() {
        let orig = sample_slide();
        let orig_json = slide_json(&orig);
        let obj = Box::new(sample_object("hostile", 0.0));

        let bad_deltas = vec![
            // Out-of-bounds base object indices
            SlideDelta::ObjectModified {
                layer: None,
                index: 4,
                object: obj.clone(),
            },
            SlideDelta::ObjectModified {
                layer: None,
                index: usize::MAX,
                object: obj.clone(),
            },
            SlideDelta::ObjectInserted {
                layer: None,
                index: 4,
            },
            SlideDelta::ObjectInserted {
                layer: None,
                index: usize::MAX,
            },
            SlideDelta::ObjectRemoved {
                layer: None,
                index: 5, // > len (4)
                object: obj.clone(),
            },
            SlideDelta::ObjectRemoved {
                layer: None,
                index: usize::MAX,
                object: obj.clone(),
            },
            SlideDelta::ObjectReordered {
                layer: None,
                from: 4,
                to: 0,
            },
            SlideDelta::ObjectReordered {
                layer: None,
                from: 0,
                to: 4,
            },
            SlideDelta::ObjectReordered {
                layer: None,
                from: usize::MAX,
                to: usize::MAX,
            },
            // Out-of-bounds layer index (layer 99 does not exist)
            SlideDelta::ObjectModified {
                layer: Some(99),
                index: 0,
                object: obj.clone(),
            },
            SlideDelta::ObjectInserted {
                layer: Some(99),
                index: 0,
            },
            SlideDelta::ObjectRemoved {
                layer: Some(99),
                index: 0,
                object: obj.clone(),
            },
            SlideDelta::ObjectReordered {
                layer: Some(99),
                from: 0,
                to: 1,
            },
            // Valid layer 0, out-of-bounds object index
            SlideDelta::ObjectModified {
                layer: Some(0),
                index: 4,
                object: obj.clone(),
            },
            SlideDelta::ObjectInserted {
                layer: Some(0),
                index: 4,
            },
            SlideDelta::ObjectRemoved {
                layer: Some(0),
                index: 5,
                object: obj,
            },
            SlideDelta::ObjectReordered {
                layer: Some(0),
                from: 1,
                to: 99,
            },
        ];

        for mut delta in bad_deltas {
            let delta_before = delta.clone();
            let mut slide = orig.clone();
            let res = delta.apply(&mut slide);
            assert!(
                matches!(res, Err(EngineError::Invalid(_))),
                "expected Err(Invalid), got {res:?} for {delta_before:?}"
            );
            assert_eq!(slide_json(&slide), orig_json, "slide mutated on error");
            assert_eq!(delta, delta_before, "delta mutated on error");
        }
    }
}
