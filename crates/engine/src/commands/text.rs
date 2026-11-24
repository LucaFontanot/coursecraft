//! Rich text, paragraph, hyperlink, and autofit commands.

use coursecraft_course::{
    Align, Autofit, Color, Link, ListKind, ObjectKind, Paragraph, ShapeText, TextRun, TextRunPatch,
    TextStyle, VAlign, align_paragraphs, apply_link_to_range, apply_style_to_range_with_base,
    apply_text_edit, is_valid_link_url, paragraphs_for, parse_color, relative_luminance,
};
use serde_json::Value as Json;

use super::{bad, find_object_mut, keep_valid, opt, req, slide_mut, text, validate_color};
use crate::{EngineError, Result, Session};

/// Default shape text style: a contrasting colour on the shape's fill, centred
/// both ways. Transparent fills read as the slide background, so use dark text.
fn shape_text_default_style(fill: &Color) -> TextStyle {
    let dark = Color("#1F1F1F".to_owned());
    let light = Color("#FFFFFF".to_owned());
    let color = match parse_color(fill) {
        Some(fill) if fill.a > 0 => {
            let luminance = relative_luminance(fill);
            if luminance < 0.45 { light } else { dark }
        }
        _ => dark,
    };
    TextStyle {
        color,
        align: Align::Center,
        valign: VAlign::Middle,
        ..TextStyle::default()
    }
}

/// Mutable parts of an object that carries text. A shape with no text yet gets
/// a default style when a text command touches it.
struct TextParts<'a> {
    text: &'a mut String,
    runs: &'a mut Vec<TextRun>,
    style: &'a mut TextStyle,
    paragraphs: &'a mut Vec<Paragraph>,
    autofit: Option<&'a mut Autofit>,
    /// A table cell keeps vertical alignment beside its style.
    valign: Option<&'a mut VAlign>,
}

fn text_parts(kind: &mut ObjectKind) -> Option<TextParts<'_>> {
    match kind {
        ObjectKind::Text {
            text,
            runs,
            style,
            paragraphs,
            autofit,
            ..
        } => Some(TextParts {
            text,
            runs,
            style,
            paragraphs,
            autofit: Some(autofit),
            valign: None,
        }),
        ObjectKind::Shape { fill, text, .. } => {
            if text.is_none() {
                *text = Some(ShapeText {
                    text: String::new(),
                    style: shape_text_default_style(fill),
                    runs: Vec::new(),
                    paragraphs: Vec::new(),
                    autofit: Autofit::None,
                });
            }
            let shape = text.as_mut()?;
            Some(TextParts {
                text: &mut shape.text,
                runs: &mut shape.runs,
                style: &mut shape.style,
                paragraphs: &mut shape.paragraphs,
                autofit: Some(&mut shape.autofit),
                valign: None,
            })
        }
        _ => None,
    }
}

/// Text parts of an object, or of the table cell named by `row` and `column`.
fn text_parts_at<'a>(
    kind: &'a mut ObjectKind,
    p: &Json,
    cmd: &str,
) -> Result<Option<TextParts<'a>>> {
    if let ObjectKind::Table { rows, .. } = kind {
        let row: usize =
            opt(cmd, p, "row")?.ok_or_else(|| bad(cmd, "a table cell needs `row` and `column`"))?;
        let column: usize = opt(cmd, p, "column")?
            .ok_or_else(|| bad(cmd, "a table cell needs `row` and `column`"))?;
        let cell = rows
            .get_mut(row)
            .and_then(|line| line.cells.get_mut(column))
            .ok_or_else(|| EngineError::NotFound(format!("no table cell {row},{column}")))?;
        return Ok(Some(TextParts {
            text: &mut cell.text,
            runs: &mut cell.runs,
            style: &mut cell.style,
            paragraphs: &mut cell.paragraphs,
            autofit: None,
            valign: Some(&mut cell.valign),
        }));
    }
    Ok(text_parts(kind))
}

pub(super) fn text_set(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "text.set";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let oid: String = req(C, p, "object")?;
    let new_text_raw: String = req(C, p, "text")?;
    let new_text = text(C, new_text_raw, "text")?;

    let explicit_runs: Option<Vec<coursecraft_course::TextRun>> = opt(C, p, "runs")?;
    let explicit_paragraphs: Option<Vec<coursecraft_course::Paragraph>> = opt(C, p, "paragraphs")?;

    let sl = slide_mut(s, &sid)?;
    let obj = find_object_mut(sl, &oid)?;
    let Some(parts) = text_parts_at(&mut obj.kind, p, C)? else {
        return Err(bad(C, format!("object {oid:?} is not a text object")));
    };
    // Diff against what is displayed: the runs when there are any.
    let old_plain: String = if parts.runs.is_empty() {
        parts.text.clone()
    } else {
        parts.runs.iter().map(|r| r.text.as_str()).collect()
    };
    let new_runs = match explicit_runs {
        Some(runs) if runs.iter().map(|r| r.text.as_str()).collect::<String>() == new_text => {
            coursecraft_course::normalize_runs(&runs)
        }
        _ => apply_text_edit(parts.runs, &old_plain, &new_text),
    };
    if new_runs.len() > 1000 {
        return Err(bad(C, "exceeded 1000 text runs"));
    }
    let new_paragraphs = match explicit_paragraphs {
        Some(paragraphs) => paragraphs,
        None => align_paragraphs(&old_plain, parts.paragraphs, &new_text),
    };
    *parts.runs = new_runs;
    *parts.paragraphs = new_paragraphs;
    *parts.text = new_text;
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

pub(super) fn text_style_range(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "text.style_range";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let oid: String = req(C, p, "object")?;
    let start: usize = req(C, p, "start")?;
    let end: usize = req(C, p, "end")?;
    let bold: Option<bool> = opt(C, p, "bold")?;
    let italic: Option<bool> = opt(C, p, "italic")?;
    let underline: Option<bool> = opt(C, p, "underline")?;
    let strikethrough: Option<bool> = opt(C, p, "strikethrough")?;
    let baseline: Option<coursecraft_course::Baseline> = opt(C, p, "baseline")?;
    let letter_spacing: Option<f32> = opt(C, p, "letter_spacing")?;
    if letter_spacing.is_some_and(|v| !v.is_finite()) {
        return Err(bad(C, "letter_spacing must be finite"));
    }
    let size: Option<f32> = opt(C, p, "size")?;
    let color_str: Option<String> = opt(C, p, "color")?;
    let font: Option<String> = opt(C, p, "font")?;

    if let Some(sz) = size
        && (!sz.is_finite() || sz <= 0.0)
    {
        return Err(bad(C, "size must be a finite positive number"));
    }
    let color = if let Some(cs) = color_str {
        let col = Color(cs);
        validate_color(C, &col)?;
        Some(col)
    } else {
        None
    };

    let sl = slide_mut(s, &sid)?;
    let obj = find_object_mut(sl, &oid)?;
    let Some(parts) = text_parts_at(&mut obj.kind, p, C)? else {
        return Err(bad(C, format!("object {oid:?} is not a text object")));
    };
    let total_chars = parts.text.chars().count();
    let (range_start, range_end) = if start == 0 && end == 0 {
        (0, 0)
    } else {
        let s_clamp = start.min(total_chars);
        let e_clamp = end.min(total_chars);
        (s_clamp.min(e_clamp), s_clamp.max(e_clamp))
    };

    let patch = TextRunPatch {
        bold,
        italic,
        underline,
        size,
        color,
        font,
        link: None,
        strikethrough,
        baseline,
        letter_spacing,
    };
    let new_runs = apply_style_to_range_with_base(
        parts.runs,
        parts.text,
        (range_start, range_end),
        &patch,
        Some(parts.style),
    );
    if new_runs.len() > 1000 {
        return Err(bad(C, "exceeded 1000 text runs"));
    }
    *parts.runs = new_runs;
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

pub(super) fn text_set_paragraph(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "text.set_paragraph";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let oid: String = req(C, p, "object")?;
    let align_str: Option<String> = opt(C, p, "align")?;
    let valign_str: Option<String> = opt(C, p, "valign")?;
    let wrap: Option<bool> = opt(C, p, "wrap")?;
    let dir_str: Option<String> = opt(C, p, "direction")?;
    let insets_val: Option<coursecraft_course::TextInsets> = opt(C, p, "insets")?;

    let align = match align_str.as_deref() {
        None => None,
        Some("left") => Some(Align::Left),
        Some("center") => Some(Align::Center),
        Some("right") => Some(Align::Right),
        Some("justify") => Some(Align::Justify),
        Some(other) => return Err(bad(C, format!("unknown align {other:?}"))),
    };
    let valign = match valign_str.as_deref() {
        None => None,
        Some("top") => Some(VAlign::Top),
        Some("middle") => Some(VAlign::Middle),
        Some("bottom") => Some(VAlign::Bottom),
        Some(other) => return Err(bad(C, format!("unknown valign {other:?}"))),
    };
    let direction = match dir_str.as_deref() {
        None => None,
        Some("horizontal") => Some(coursecraft_course::TextDirection::Horizontal),
        Some("vertical") => Some(coursecraft_course::TextDirection::Vertical),
        Some("vertical270") => Some(coursecraft_course::TextDirection::Vertical270),
        Some(other) => return Err(bad(C, format!("unknown direction {other:?}"))),
    };

    let sl = slide_mut(s, &sid)?;
    let obj = find_object_mut(sl, &oid)?;
    let Some(mut parts) = text_parts_at(&mut obj.kind, p, C)? else {
        return Err(bad(C, format!("object {oid:?} is not a text object")));
    };
    if let Some(a) = align {
        parts.style.align = a;
    }
    if let Some(v) = valign {
        if let Some(slot) = parts.valign.as_deref_mut() {
            *slot = v;
        } else {
            parts.style.valign = v;
        }
    }
    if let Some(w) = wrap {
        parts.style.wrap = w;
    }
    if let Some(d) = direction {
        parts.style.direction = d;
    }
    if let Some(ins) = insets_val {
        parts.style.insets = Some(ins);
    }
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

fn trim_paragraphs(paragraphs: &mut Vec<Paragraph>) {
    while paragraphs.last() == Some(&Paragraph::default()) {
        paragraphs.pop();
    }
}

pub(super) fn text_set_paragraphs(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "text.set_paragraphs";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let oid: String = req(C, p, "object")?;
    let start: Option<usize> = opt(C, p, "start")?;
    let end: Option<usize> = opt(C, p, "end")?;
    let list_str: Option<String> = opt(C, p, "list")?;
    let level: Option<u8> = opt(C, p, "level")?;
    let line_spacing: Option<f32> = opt(C, p, "line_spacing")?;
    let space_before: Option<f32> = opt(C, p, "space_before")?;
    let space_after: Option<f32> = opt(C, p, "space_after")?;

    let list = match list_str.as_deref() {
        None => None,
        Some("none") => Some(ListKind::None),
        Some("bullet") => Some(ListKind::Bullet),
        Some("number") => Some(ListKind::Number),
        Some(other) => return Err(bad(C, format!("unknown list {other:?}"))),
    };
    if let Some(level) = level
        && level > 4
    {
        return Err(bad(C, "level must be within 0..=4"));
    }
    if let Some(spacing) = line_spacing
        && (!spacing.is_finite() || !(0.8..=3.0).contains(&spacing))
    {
        return Err(bad(C, "line_spacing must be finite and within 0.8..=3"));
    }
    for (name, value) in [("space_before", space_before), ("space_after", space_after)] {
        if let Some(value) = value
            && (!value.is_finite() || !(0.0..=200.0).contains(&value))
        {
            return Err(bad(C, format!("{name} must be finite and within 0..=200")));
        }
    }

    let sl = slide_mut(s, &sid)?;
    let obj = find_object_mut(sl, &oid)?;
    let Some(parts) = text_parts_at(&mut obj.kind, p, C)? else {
        return Err(bad(C, format!("object {oid:?} is not a text object")));
    };
    let count = coursecraft_course::text::paragraph_count(parts.text);
    let first = start.unwrap_or(0).min(count);
    let last = end.unwrap_or(count).clamp(first, count);
    let mut paragraphs = paragraphs_for(parts.text, parts.paragraphs);
    for paragraph in paragraphs.iter_mut().skip(first).take(last - first) {
        if let Some(list) = list {
            paragraph.list = list;
        }
        if let Some(level) = level {
            paragraph.level = level;
        }
        if let Some(spacing) = line_spacing {
            paragraph.line_spacing = spacing;
        }
        if let Some(value) = space_before {
            paragraph.space_before = value;
        }
        if let Some(value) = space_after {
            paragraph.space_after = value;
        }
    }
    trim_paragraphs(&mut paragraphs);
    *parts.paragraphs = paragraphs;
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

pub(super) fn text_set_link(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "text.set_link";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let oid: String = req(C, p, "object")?;
    let start: Option<usize> = opt(C, p, "start")?;
    let end: Option<usize> = opt(C, p, "end")?;
    let link: Option<Link> = opt(C, p, "link")?;

    if let Some(link) = &link {
        match link {
            Link::Url { url } if !is_valid_link_url(url) => {
                return Err(bad(
                    C,
                    format!("link {url:?} must start with http://, https:// or mailto:"),
                ));
            }
            Link::Slide { slide } if s.course.play_slide(slide).is_none() => {
                return Err(bad(
                    C,
                    format!("link target slide {slide:?} does not exist"),
                ));
            }
            _ => {}
        }
    }

    let sl = slide_mut(s, &sid)?;
    let obj = find_object_mut(sl, &oid)?;
    let Some(parts) = text_parts_at(&mut obj.kind, p, C)? else {
        return Err(bad(C, format!("object {oid:?} is not a text object")));
    };
    let total_chars = parts.text.chars().count();
    let start = start.unwrap_or(0).min(total_chars);
    let end = end.unwrap_or(total_chars).min(total_chars);
    let (start, end) = if start == 0 && end == total_chars {
        (0, 0)
    } else {
        (start.min(end), start.max(end))
    };
    let new_runs = apply_link_to_range(parts.runs, parts.text, (start, end), link);
    if new_runs.len() > 1000 {
        return Err(bad(C, "exceeded 1000 text runs"));
    }
    *parts.runs = new_runs;
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}

pub(super) fn object_set_autofit(s: &mut Session, p: &Json) -> Result<Json> {
    const C: &str = "object.set_autofit";
    let was_valid = s.is_playable();
    let sid: String = req(C, p, "slide")?;
    let oid: String = req(C, p, "object")?;
    let autofit: Autofit = req(C, p, "autofit")?;

    let sl = slide_mut(s, &sid)?;
    let obj = find_object_mut(sl, &oid)?;
    let Some(parts) = text_parts_at(&mut obj.kind, p, C)? else {
        return Err(bad(C, format!("object {oid:?} is not a text object")));
    };
    let Some(current) = parts.autofit else {
        return Err(bad(C, "a table cell has no autofit setting"));
    };
    *current = autofit;
    keep_valid(s, was_valid)?;
    Ok(Json::Null)
}
