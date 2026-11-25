//! Text formatting for the ribbon's Font and Paragraph groups and the
//! editing shortcuts (Ctrl+B, Ctrl+I, Ctrl+U).
//!
//! One place answers the three questions every text control asks:
//!
//! - **What does it act on?** [`targets`]: the text being edited on the canvas
//!   (the selected characters, or the caret), else every selected object that
//!   holds text, whole.
//! - **What is it now?** [`state`]: font, size, bold… at the target, `None`
//!   where the target is mixed.
//! - **How does it change?** [`apply`]: one [`Change`], turned into engine
//!   commands (`text.style_range`, `text.set_paragraph`, `text.set_paragraphs`)
//!   run as one undo step. Typed text not yet committed is committed in the
//!   same step, and a style picked at a caret waits for the next typed text.
//!
//! DeckCraft does this in its engine (`format.state`, `format.*`) because its
//! engine owns the text selection. Here the in-place editor's buffer and caret
//! live in the editor, so the resolver lives here, and the engine keeps its
//! explicit, id-addressed commands.

use coursecraft_course::{
    Align, Course, ListKind, ObjectKind, Paragraph, TextRun, TextStyle, VAlign,
};
use serde_json::{Value as Json, json};

use crate::Editor;
use crate::properties::{
    PendingCaretStyle, default_shape_text_style, effects_tri_for_range, font_at_cursor,
    style_for_range, style_tri_for_range,
};

/// Which characters of one object a change applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// The object's whole text.
    Whole,
    /// Characters `start..end` of the text being edited.
    Range(usize, usize),
    /// The caret of the text being edited: character changes wait for the
    /// next typed text, paragraph changes apply to its paragraph.
    Caret(usize),
}

/// One object's text a change applies to.
#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub slide: String,
    pub object: String,
    /// A table cell being edited, as `(row, column)`.
    pub cell: Option<(usize, usize)>,
    pub scope: Scope,
}

/// The formatting at the targets; `None` where they differ.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TextState {
    pub font: Option<String>,
    pub size: Option<f32>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub strikethrough: Option<bool>,
    pub color: Option<String>,
    pub align: Option<Align>,
    pub valign: Option<VAlign>,
    pub wrap: Option<bool>,
    pub list: Option<ListKind>,
    pub level: Option<u8>,
    pub line_spacing: Option<f32>,
}

/// One formatting change, as the ribbon and the shortcuts ask for it.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    Bold,
    Italic,
    Underline,
    Strikethrough,
    Font(String),
    Size(f32),
    Grow,
    Shrink,
    Color(String),
    /// Extra space after each letter, in points.
    Spacing(f32),
    Align(Align),
    VAlign(VAlign),
    /// Toggle wrapping text inside the box width (converts point text to area text).
    Wrap,
    /// Bullets or Numbering: on, or off when every target paragraph has it.
    List(ListKind),
    /// One indent level in (`1`) or out (`-1`).
    Indent(i8),
    LineSpacing(f32),
}

/// The font sizes offered, and the steps of Grow Font and Shrink Font.
pub const SIZES: [f32; 22] = [
    8.0, 9.0, 10.0, 11.0, 12.0, 14.0, 16.0, 18.0, 20.0, 24.0, 28.0, 32.0, 36.0, 40.0, 44.0, 48.0,
    54.0, 60.0, 66.0, 72.0, 80.0, 96.0,
];

/// Line spacing multiples offered.
pub const LINE_SPACINGS: [f32; 6] = [1.0, 1.15, 1.5, 2.0, 2.5, 3.0];

/// Character spacing presets (label, points).
pub const SPACINGS: [(&str, f32); 5] = [
    ("Very Tight", -3.0),
    ("Tight", -1.5),
    ("Normal", 0.0),
    ("Loose", 3.0),
    ("Very Loose", 6.0),
];

const NO_TEXT: &str = "Select a text box, or a shape that holds text";
const IN_STATE: &str = "Use the Text panel to format text in a state";
const BUTTON: &str = "Use the Format inspector to format a button's label";

/// What the text controls act on, or why they can't act.
pub fn targets(editor: &Editor) -> Result<Vec<Target>, &'static str> {
    if editor.ui.story_view {
        return Err("Open a slide first");
    }
    if editor.ui.editing_state.is_some() {
        return Err(IN_STATE);
    }
    if let Some(edit) = &editor.ui.inline_edit {
        if edit.state_name.is_some() || edit.extra_id.is_some() {
            return Err(IN_STATE);
        }
        if edit.is_button {
            return Err(BUTTON);
        }
        let scope = match &editor.ui.inline_edit_selection {
            Some((slide, object, range, caret))
                if *slide == edit.slide && *object == edit.object =>
            {
                match range {
                    Some((a, b)) if a != b => Scope::Range(*a.min(b), *a.max(b)),
                    _ => Scope::Caret(*caret),
                }
            }
            _ => Scope::Caret(edit.buffer.chars().count()),
        };
        return Ok(vec![Target {
            slide: edit.slide.clone(),
            object: edit.object.clone(),
            cell: edit.cell,
            scope,
        }]);
    }
    let course = &editor.session.course;
    let slide = &editor.selection.slide;
    let found: Vec<Target> = editor
        .selection
        .ids()
        .into_iter()
        .filter(|id| stored_text(course, slide, id, None).is_some())
        .map(|object| Target {
            slide: slide.clone(),
            object,
            cell: None,
            scope: Scope::Whole,
        })
        .collect();
    if found.is_empty() {
        Err(NO_TEXT)
    } else {
        Ok(found)
    }
}

/// One object's text as stored, or as typed while it is being edited.
struct Parts {
    style: TextStyle,
    runs: Vec<TextRun>,
    text: String,
    paragraphs: Vec<Paragraph>,
}

fn stored_text(
    course: &Course,
    slide: &str,
    object: &str,
    cell: Option<(usize, usize)>,
) -> Option<Parts> {
    let (obj, _) = course.slide(slide)?.object(object)?;
    match (&obj.kind, cell) {
        (
            ObjectKind::Text {
                text,
                style,
                runs,
                paragraphs,
                ..
            },
            None,
        ) => Some(Parts {
            style: style.clone(),
            runs: runs.clone(),
            text: text.clone(),
            paragraphs: paragraphs.clone(),
        }),
        (
            ObjectKind::Shape {
                shape, fill, text, ..
            },
            None,
        ) if coursecraft_course::shape_allows_text(*shape) => Some(match text {
            Some(t) => Parts {
                style: t.style.clone(),
                runs: t.runs.clone(),
                text: t.text.clone(),
                paragraphs: t.paragraphs.clone(),
            },
            None => Parts {
                style: default_shape_text_style(fill),
                runs: Vec::new(),
                text: String::new(),
                paragraphs: Vec::new(),
            },
        }),
        (ObjectKind::Table { rows, .. }, Some((row, column))) => {
            let cell = rows.get(row)?.cells.get(column)?;
            Some(Parts {
                style: cell.style.clone(),
                runs: cell.runs.clone(),
                text: cell.text.clone(),
                paragraphs: cell.paragraphs.clone(),
            })
        }
        _ => None,
    }
}

/// The target's text, with what is being typed in place of what is stored.
fn parts(editor: &Editor, target: &Target) -> Option<Parts> {
    let mut parts = stored_text(
        &editor.session.course,
        &target.slide,
        &target.object,
        target.cell,
    )?;
    if let Some(edit) =
        editor.ui.inline_edit.as_ref().filter(|e| {
            e.slide == target.slide && e.object == target.object && e.cell == target.cell
        })
    {
        parts.text.clone_from(&edit.buffer);
        let live: String = edit.live_runs.iter().map(|r| r.text.as_str()).collect();
        if live == edit.buffer {
            parts.runs.clone_from(&edit.live_runs);
        }
        if !edit.live_paragraphs.is_empty() {
            parts.paragraphs.clone_from(&edit.live_paragraphs);
        }
    }
    Some(parts)
}

/// The character range the target covers, `(start, end)`, empty at a caret.
fn char_range(scope: Scope, text: &str) -> (usize, usize) {
    match scope {
        Scope::Whole => (0, text.chars().count()),
        Scope::Range(a, b) => (a, b),
        Scope::Caret(c) => (c, c),
    }
}

/// The paragraphs the target covers, `(first, end)`.
fn paragraph_range(scope: Scope, text: &str) -> (usize, usize) {
    match scope {
        Scope::Whole => (0, coursecraft_course::text::paragraph_count(text)),
        Scope::Range(a, b) => coursecraft_course::text::paragraph_range_at(text, a, b),
        Scope::Caret(c) => coursecraft_course::text::paragraph_range_at(text, c, c),
    }
}

fn merge<T: PartialEq>(acc: &mut Option<Option<T>>, value: Option<T>) {
    match acc {
        None => *acc = Some(value),
        Some(seen) if *seen != value => *seen = None,
        Some(_) => {}
    }
}

fn one_state(editor: &Editor, target: &Target) -> Option<TextState> {
    let parts = parts(editor, target)?;
    let range = char_range(target.scope, &parts.text);
    let (bold, italic, underline) = style_tri_for_range(&parts.runs, &parts.style, range);
    let (strikethrough, _, _) = effects_tri_for_range(&parts.runs, &parts.style, range);
    let (_, _, _, size, color) = style_for_range(&parts.runs, &parts.style, range);
    let font = font_at_cursor(&parts.runs, range.0).unwrap_or_else(|| parts.style.font.clone());
    let (first, end) = paragraph_range(target.scope, &parts.text);
    let paragraphs = coursecraft_course::text::paragraphs_for(&parts.text, &parts.paragraphs);
    let covered: Vec<&Paragraph> = paragraphs
        .iter()
        .skip(first)
        .take(end.saturating_sub(first).max(1))
        .collect();
    let same = |f: &dyn Fn(&Paragraph) -> f32| {
        let first = covered.first().map(|p| f(p));
        first.filter(|v| covered.iter().all(|p| (f(p) - v).abs() < f32::EPSILON))
    };
    let list = covered.first().map(|p| p.list);
    let list = list.filter(|l| covered.iter().all(|p| p.list == *l));
    let level = covered.first().map(|p| p.level);
    let mut state = TextState {
        font: Some(font),
        size: Some(size),
        bold,
        italic,
        underline,
        strikethrough,
        color: Some(color),
        align: Some(parts.style.align),
        valign: Some(parts.style.valign),
        wrap: Some(parts.style.wrap),
        list: list.or(covered.is_empty().then_some(ListKind::None)),
        level: level.or(Some(0)),
        line_spacing: same(&|p| p.line_spacing).or(covered.is_empty().then_some(1.0)),
    };
    // A style picked at the caret shows until text is typed with it.
    if let Scope::Caret(caret) = target.scope
        && let Some(pending) = editor
            .ui
            .properties
            .pending_caret_style
            .as_ref()
            .filter(|p| {
                p.slide == target.slide
                    && p.object == target.object
                    && p.cell == target.cell
                    && p.cursor == caret
            })
    {
        let patch = &pending.patch;
        state.bold = patch.bold.or(state.bold);
        state.italic = patch.italic.or(state.italic);
        state.underline = patch.underline.or(state.underline);
        state.strikethrough = patch.strikethrough.or(state.strikethrough);
        state.size = patch.size.or(state.size);
        state.font = patch.font.clone().or(state.font);
        state.color = patch.color.as_ref().map(|c| c.0.clone()).or(state.color);
    }
    Some(state)
}

/// The formatting at what the text controls act on; `None` when they can't act.
#[must_use]
pub fn state(editor: &Editor) -> Option<TextState> {
    let targets = targets(editor).ok()?;
    let states: Vec<TextState> = targets
        .iter()
        .filter_map(|t| one_state(editor, t))
        .collect();
    let first = states.first()?.clone();
    if states.len() == 1 {
        return Some(first);
    }
    let mut font = None;
    let mut size = None;
    let mut bold = None;
    let mut italic = None;
    let mut underline = None;
    let mut strike = None;
    let mut color = None;
    let mut align = None;
    let mut valign = None;
    let mut wrap = None;
    let mut list = None;
    let mut level = None;
    let mut spacing = None;
    for s in states {
        merge(&mut font, s.font);
        merge(&mut size, s.size);
        merge(&mut bold, s.bold);
        merge(&mut italic, s.italic);
        merge(&mut underline, s.underline);
        merge(&mut strike, s.strikethrough);
        merge(&mut color, s.color);
        merge(&mut align, s.align);
        merge(&mut valign, s.valign);
        merge(&mut wrap, s.wrap);
        merge(&mut list, s.list);
        merge(&mut level, s.level);
        merge(&mut spacing, s.line_spacing);
    }
    Some(TextState {
        font: font.flatten(),
        size: size.flatten(),
        bold: bold.flatten(),
        italic: italic.flatten(),
        underline: underline.flatten(),
        strikethrough: strike.flatten(),
        color: color.flatten(),
        align: align.flatten(),
        valign: valign.flatten(),
        wrap: wrap.flatten(),
        list: list.flatten(),
        level: level.flatten(),
        line_spacing: spacing.flatten(),
    })
}

/// The next size up (`up`) or down from `size` in [`SIZES`]; past either end,
/// one point (down) or ten percent (up).
#[must_use]
pub fn step_size(size: f32, up: bool) -> f32 {
    if up {
        SIZES
            .iter()
            .copied()
            .find(|s| *s > size + 0.01)
            .unwrap_or((size * 1.1).round().min(400.0))
    } else {
        SIZES
            .iter()
            .rev()
            .copied()
            .find(|s| *s < size - 0.01)
            .unwrap_or((size - 1.0).max(1.0))
    }
}

fn address(target: &Target) -> Json {
    let mut params = json!({ "slide": target.slide, "object": target.object });
    if let Some((row, column)) = target.cell {
        params["row"] = json!(row);
        params["column"] = json!(column);
    }
    params
}

fn align_name(align: Align) -> &'static str {
    match align {
        Align::Left => "left",
        Align::Center => "center",
        Align::Right => "right",
        Align::Justify => "justify",
    }
}

fn valign_name(valign: VAlign) -> &'static str {
    match valign {
        VAlign::Top => "top",
        VAlign::Middle => "middle",
        VAlign::Bottom => "bottom",
    }
}

fn list_name(list: ListKind) -> &'static str {
    match list {
        ListKind::None => "none",
        ListKind::Bullet => "bullet",
        ListKind::Number => "number",
    }
}

/// The character fields `change` sets, as `text.style_range` parameters.
fn character_fields(change: &Change, now: &TextState, size: f32) -> Option<Json> {
    Some(match change {
        Change::Bold => json!({ "bold": now.bold != Some(true) }),
        Change::Italic => json!({ "italic": now.italic != Some(true) }),
        Change::Underline => json!({ "underline": now.underline != Some(true) }),
        Change::Strikethrough => json!({ "strikethrough": now.strikethrough != Some(true) }),
        Change::Font(name) => json!({ "font": name }),
        Change::Size(size) => json!({ "size": size }),
        Change::Grow => json!({ "size": step_size(size, true) }),
        Change::Shrink => json!({ "size": step_size(size, false) }),
        Change::Color(color) => json!({ "color": color }),
        Change::Spacing(points) => json!({ "letter_spacing": points }),
        Change::Align(_)
        | Change::VAlign(_)
        | Change::Wrap
        | Change::List(_)
        | Change::Indent(_)
        | Change::LineSpacing(_) => return None,
    })
}

fn patch_pending(patch: &mut coursecraft_course::TextRunPatch, fields: &Json) {
    let flag = |key: &str| fields.get(key).and_then(Json::as_bool);
    let number = |key: &str| fields.get(key).and_then(Json::as_f64).map(|v| v as f32);
    if let Some(v) = flag("bold") {
        patch.bold = Some(v);
    }
    if let Some(v) = flag("italic") {
        patch.italic = Some(v);
    }
    if let Some(v) = flag("underline") {
        patch.underline = Some(v);
    }
    if let Some(v) = flag("strikethrough") {
        patch.strikethrough = Some(v);
    }
    if let Some(v) = number("size") {
        patch.size = Some(v);
    }
    if let Some(v) = number("letter_spacing") {
        patch.letter_spacing = Some(v);
    }
    if let Some(v) = fields.get("font").and_then(Json::as_str) {
        patch.font = Some(v.to_owned());
    }
    if let Some(v) = fields.get("color").and_then(Json::as_str) {
        patch.color = Some(coursecraft_course::Color(v.to_owned()));
    }
}

/// Apply `change` to what the text controls act on, as one undo step.
/// Returns whether anything changed.
pub fn apply(editor: &mut Editor, change: &Change) -> bool {
    let Ok(targets) = targets(editor) else {
        return false;
    };
    let Some(now) = state(editor) else {
        return false;
    };
    let mut steps: Vec<(String, Json)> = Vec::new();
    // Typed text not yet committed goes first, so ranges match what is shown.
    let dirty = editor
        .ui
        .inline_edit
        .as_ref()
        .filter(|e| e.buffer != e.original)
        .map(|e| {
            let mut params = json!({ "slide": e.slide, "object": e.object, "text": e.buffer });
            if let Some((row, column)) = e.cell {
                params["row"] = json!(row);
                params["column"] = json!(column);
            }
            params
        });
    for target in &targets {
        let Some(parts) = parts(editor, target) else {
            continue;
        };
        let own = one_state(editor, target).unwrap_or_default();
        let size = own.size.or(now.size).unwrap_or(parts.style.size);
        if let Some(fields) = character_fields(change, &now, size) {
            match target.scope {
                Scope::Caret(caret) => {
                    let pending =
                        editor
                            .ui
                            .properties
                            .pending_caret_style
                            .get_or_insert_with(|| PendingCaretStyle {
                                slide: target.slide.clone(),
                                object: target.object.clone(),
                                cell: target.cell,
                                cursor: caret,
                                patch: coursecraft_course::TextRunPatch::default(),
                            });
                    if pending.slide != target.slide
                        || pending.object != target.object
                        || pending.cell != target.cell
                        || pending.cursor != caret
                    {
                        *pending = PendingCaretStyle {
                            slide: target.slide.clone(),
                            object: target.object.clone(),
                            cell: target.cell,
                            cursor: caret,
                            patch: coursecraft_course::TextRunPatch::default(),
                        };
                    }
                    patch_pending(&mut pending.patch, &fields);
                }
                Scope::Whole | Scope::Range(..) => {
                    let (start, end) = match target.scope {
                        Scope::Range(a, b) => (a, b),
                        _ => (0, 0),
                    };
                    let mut params = address(target);
                    params["start"] = json!(start);
                    params["end"] = json!(end);
                    if let (Json::Object(to), Json::Object(from)) = (&mut params, fields) {
                        to.extend(from);
                    }
                    steps.push(("text.style_range".to_owned(), params));
                }
            }
            continue;
        }
        let mut params = address(target);
        match change {
            Change::Align(align) => params["align"] = json!(align_name(*align)),
            Change::VAlign(valign) => params["valign"] = json!(valign_name(*valign)),
            Change::Wrap => params["wrap"] = json!(now.wrap != Some(true)),
            Change::List(_) | Change::Indent(_) | Change::LineSpacing(_) => {
                let (first, end) = paragraph_range(target.scope, &parts.text);
                if target.scope != Scope::Whole {
                    params["start"] = json!(first);
                    params["end"] = json!(end.max(first + 1));
                }
                match change {
                    Change::List(kind) => {
                        let next = if now.list == Some(*kind) {
                            ListKind::None
                        } else {
                            *kind
                        };
                        params["list"] = json!(list_name(next));
                    }
                    Change::Indent(delta) => {
                        let level = i16::from(own.level.unwrap_or(0)) + i16::from(*delta);
                        params["level"] = json!(level.clamp(0, 4));
                    }
                    Change::LineSpacing(lines) => params["line_spacing"] = json!(lines),
                    _ => {}
                }
                steps.push(("text.set_paragraphs".to_owned(), params));
                continue;
            }
            _ => continue,
        }
        steps.push(("text.set_paragraph".to_owned(), params));
    }
    if steps.is_empty() {
        // A caret style: nothing to run until text is typed.
        return editor.ui.properties.pending_caret_style.is_some();
    }
    if let Some(text) = dirty {
        steps.insert(0, ("text.set".to_owned(), text));
    }
    let done = editor.run_batch(&steps).is_some();
    if done && let Some(edit) = editor.ui.inline_edit.as_mut() {
        edit.original = edit.buffer.clone();
    }
    done
}
