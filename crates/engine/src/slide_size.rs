//! Changing the slide size, with or without scaling the content.

use coursecraft_course::{
    Course, Glow, MAX_OBJECT_FRAME_SIZE, Object, ObjectKind, Outline, Paragraph, Shadow, ShapeText,
    Size, Slide, StateDef, StateExtra, StateExtraKind, TextRun, TextStyle, clamp_letter_spacing,
};

/// Smallest and largest slide sides the command accepts, in pixels.
pub(crate) const MIN_SIDE: u32 = 100;
pub(crate) const MAX_W: u32 = 7680;
pub(crate) const MAX_H: u32 = 4320;
const MAX_COORD: f32 = 100_000.0;

/// How a content-aware resize maps old coordinates to new ones.
#[derive(Clone, Copy)]
struct Map {
    factor: f32,
    dx: f32,
    dy: f32,
}

impl Map {
    fn len(self, v: f32) -> f32 {
        if v.is_finite() { v * self.factor } else { v }
    }

    fn x(self, v: f32) -> f32 {
        if v.is_finite() {
            v * self.factor + self.dx
        } else {
            v
        }
    }

    fn y(self, v: f32) -> f32 {
        if v.is_finite() {
            v * self.factor + self.dy
        } else {
            v
        }
    }
}

fn clamp_coord(v: f32) -> f32 {
    if v.is_finite() {
        v.clamp(-MAX_COORD, MAX_COORD)
    } else {
        0.0
    }
}

fn round_len(v: f32, min: f32) -> f32 {
    if v.is_finite() {
        v.round().clamp(min, MAX_OBJECT_FRAME_SIZE)
    } else {
        min
    }
}

fn scale_inset(v: f32, m: Map) -> f32 {
    let scaled = m.len(v);
    if scaled.is_finite() {
        scaled.clamp(0.0, MAX_COORD)
    } else {
        0.0
    }
}

fn scale_text_style(style: &mut TextStyle, m: Map) {
    style.size = m.len(style.size).clamp(1.0, 400.0);
    style.letter_spacing = clamp_letter_spacing(m.len(style.letter_spacing));
    if let Some(insets) = style.insets.as_mut() {
        insets.left = scale_inset(insets.left, m);
        insets.top = scale_inset(insets.top, m);
        insets.right = scale_inset(insets.right, m);
        insets.bottom = scale_inset(insets.bottom, m);
    }
}

fn scale_runs(runs: &mut [TextRun], m: Map) {
    for run in runs {
        if let Some(size) = run.size {
            run.size = Some(m.len(size).clamp(1.0, 400.0));
        }
        if let Some(spacing) = run.letter_spacing {
            run.letter_spacing = Some(clamp_letter_spacing(m.len(spacing)));
        }
    }
}

fn scale_outline(outline: &mut Outline, m: Map) {
    outline.width = m.len(outline.width).clamp(0.0, 50.0);
}

fn scale_paragraph(paragraph: &mut Paragraph, m: Map) {
    paragraph.space_before = m.len(paragraph.space_before).clamp(0.0, 200.0);
    paragraph.space_after = m.len(paragraph.space_after).clamp(0.0, 200.0);
    if let Some(bullet_size) = paragraph.bullet_size {
        paragraph.bullet_size = Some(m.len(bullet_size).clamp(1.0, 400.0));
    }
}

fn scale_paragraphs(paragraphs: &mut [Paragraph], m: Map) {
    for paragraph in paragraphs {
        scale_paragraph(paragraph, m);
    }
}

fn scale_shape_text(text: &mut ShapeText, m: Map) {
    scale_text_style(&mut text.style, m);
    scale_runs(&mut text.runs, m);
    scale_paragraphs(&mut text.paragraphs, m);
}

fn scale_shadow(shadow: &mut Shadow, m: Map) {
    shadow.offset_x = m.len(shadow.offset_x).clamp(-200.0, 200.0);
    shadow.offset_y = m.len(shadow.offset_y).clamp(-200.0, 200.0);
    shadow.blur = m.len(shadow.blur).clamp(0.0, 100.0);
}

fn scale_glow(glow: &mut Glow, m: Map) {
    glow.size = m.len(glow.size).clamp(0.0, 100.0);
}

fn scale_state_extra(extra: &mut StateExtra, m: Map) {
    // Parent-relative coordinates scale by `m.len` without slide centring offsets.
    extra.frame.x = clamp_coord(m.len(extra.frame.x).round());
    extra.frame.y = clamp_coord(m.len(extra.frame.y).round());
    extra.frame.w = round_len(m.len(extra.frame.w), 1.0);
    extra.frame.h = round_len(m.len(extra.frame.h), 1.0);
    if let Some(shadow) = extra.shadow.as_mut() {
        scale_shadow(shadow, m);
    }
    if let Some(glow) = extra.glow.as_mut() {
        scale_glow(glow, m);
    }
    match &mut extra.kind {
        StateExtraKind::Shape { outline, text, .. } => {
            if let Some(outline) = outline {
                scale_outline(outline, m);
            }
            if let Some(text) = text {
                scale_shape_text(text, m);
            }
        }
        StateExtraKind::Image { outline, .. } => {
            if let Some(outline) = outline {
                scale_outline(outline, m);
            }
        }
        StateExtraKind::Text {
            style,
            runs,
            paragraphs,
            outline,
            ..
        } => {
            scale_text_style(style, m);
            scale_runs(runs, m);
            scale_paragraphs(paragraphs, m);
            if let Some(outline) = outline {
                scale_outline(outline, m);
            }
        }
        StateExtraKind::Path { path } => {
            scale_outline(&mut path.outline, m);
            if let Some(text) = path.text.as_mut() {
                scale_shape_text(text, m);
            }
        }
    }
}

fn scale_state(state: &mut StateDef, m: Map) {
    if let Some(outline) = state.outline.as_mut() {
        scale_outline(outline, m);
    }
    if let Some(style) = state.text_style.as_mut() {
        scale_text_style(style, m);
    }
    if let Some(runs) = state.runs.as_mut() {
        scale_runs(runs, m);
    }
    if let Some(paragraphs) = state.paragraphs.as_mut() {
        scale_paragraphs(paragraphs, m);
    }
    if let Some(shadow) = state.shadow.as_mut() {
        scale_shadow(shadow, m);
    }
    if let Some(glow) = state.glow.as_mut() {
        scale_glow(glow, m);
    }
    for extra in &mut state.extras {
        scale_state_extra(extra, m);
    }
}

fn scale_object(o: &mut Object, m: Map) {
    let f = &mut o.frame;
    f.x = clamp_coord(m.x(f.x).round());
    f.y = clamp_coord(m.y(f.y).round());
    f.w = round_len(m.len(f.w), 1.0);
    f.h = round_len(m.len(f.h), 1.0);
    let frame_h = f.h;
    if let Some(shadow) = o.shadow.as_mut() {
        scale_shadow(shadow, m);
    }
    if let Some(glow) = o.glow.as_mut() {
        scale_glow(glow, m);
    }
    for path in &mut o.motion_paths {
        for point in &mut path.points {
            if path.relative {
                point.x = clamp_coord(m.len(point.x));
                point.y = clamp_coord(m.len(point.y));
            } else {
                point.x = clamp_coord(m.x(point.x));
                point.y = clamp_coord(m.y(point.y));
            }
        }
    }
    for state in &mut o.states {
        scale_state(state, m);
    }
    match &mut o.kind {
        ObjectKind::Text {
            style,
            runs,
            paragraphs,
            outline,
            ..
        } => {
            scale_text_style(style, m);
            scale_runs(runs, m);
            scale_paragraphs(paragraphs, m);
            if let Some(outline) = outline {
                scale_outline(outline, m);
            }
        }
        ObjectKind::Shape { outline, text, .. } => {
            if let Some(outline) = outline {
                scale_outline(outline, m);
            }
            if let Some(text) = text {
                scale_shape_text(text, m);
            }
        }
        ObjectKind::Image {
            outline: Some(outline),
            ..
        } => {
            scale_outline(outline, m);
        }
        ObjectKind::Path { path } => {
            scale_outline(&mut path.outline, m);
            if let Some(text) = path.text.as_mut() {
                scale_shape_text(text, m);
            }
        }
        ObjectKind::Timer { style, .. } => {
            scale_text_style(style, m);
        }
        ObjectKind::Marker {
            runs, paragraphs, ..
        } => {
            scale_runs(runs, m);
            scale_paragraphs(paragraphs, m);
        }
        ObjectKind::Table { rows, borders, .. } => {
            for row in rows {
                for cell in &mut row.cells {
                    scale_text_style(&mut cell.style, m);
                    scale_runs(&mut cell.runs, m);
                    scale_paragraphs(&mut cell.paragraphs, m);
                }
            }
            if let Some(outline) = borders.outer.as_mut() {
                scale_outline(outline, m);
            }
            if let Some(outline) = borders.inner.as_mut() {
                scale_outline(outline, m);
            }
        }
        ObjectKind::ScrollPanel { content_height, .. } => {
            *content_height = m
                .len(*content_height)
                .max(frame_h)
                .clamp(0.0, coursecraft_course::MAX_SCROLL_CONTENT_HEIGHT);
        }
        _ => {}
    }
}

fn scale_slide(slide: &mut Slide, m: Map) {
    for object in &mut slide.objects {
        scale_object(object, m);
    }
    for layer in &mut slide.layers {
        for object in &mut layer.objects {
            scale_object(object, m);
        }
    }
    if let Some(question) = slide.question.as_mut() {
        for region in &mut question.regions {
            region.x = clamp_coord(m.x(region.x).round());
            region.y = clamp_coord(m.y(region.y).round());
            region.w = round_len(m.len(region.w), 1.0);
            region.h = round_len(m.len(region.h), 1.0);
        }
    }
}

/// Check the requested size against the bounds.
pub(crate) fn size_in_bounds(w: u32, h: u32) -> bool {
    (MIN_SIDE..=MAX_W).contains(&w) && (MIN_SIDE..=MAX_H).contains(&h)
}

/// Change the slide size. With `scale`, content follows by one uniform factor
/// and is centred. Returns false when nothing changed.
pub(crate) fn set_slide_size(course: &mut Course, w: u32, h: u32, scale: bool) -> bool {
    let old = course.slide_size;
    if old.w == w && old.h == h {
        return false;
    }
    if scale && old.w > 0 && old.h > 0 {
        let factor = (w as f32 / old.w as f32).min(h as f32 / old.h as f32);
        if factor.is_finite() && factor > 0.0 {
            let m = Map {
                factor,
                dx: (w as f32 - old.w as f32 * factor) / 2.0,
                dy: (h as f32 - old.h as f32 * factor) / 2.0,
            };
            for scene in &mut course.scenes {
                for slide in &mut scene.slides {
                    scale_slide(slide, m);
                }
            }
            for slide in course.masters.iter_mut().chain(course.templates.iter_mut()) {
                scale_slide(slide, m);
            }
            for bank in &mut course.banks {
                for slide in &mut bank.slides {
                    scale_slide(slide, m);
                }
            }
            for guide in &mut course.editor.guides {
                guide.at = clamp_coord(if guide.vertical {
                    m.x(guide.at).round()
                } else {
                    m.y(guide.at).round()
                });
            }
            for named in &mut course.text_styles {
                scale_text_style(&mut named.style, m);
                if let Some(paragraph) = named.paragraphs.as_mut() {
                    scale_paragraph(paragraph, m);
                }
            }
        }
    }
    course.slide_size = Size { w, h };
    true
}
