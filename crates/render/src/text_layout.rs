//! Paragraph layout for slide text.
//!
//! One function lays out bullets, numbering, indents, line spacing, spacing
//! before and after, links and shrink-to-fit text. The renderer, the hit
//! regions and the tests all use it, so a bullet in the canvas is the same
//! bullet in preview and in the published player.

use std::sync::Arc;

use coursecraft_course::{Align, Autofit, Link, ListKind, NumberScheme, Paragraph, VAlign};
use egui::epaint::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{Color32, Context, Pos2, Rect, Stroke, pos2};

use crate::compose::ResolvedTextRun;
use crate::compose::{DrawItem, ResolvedLinkRect, Visual, validate_frame};

/// Smallest scale shrink-to-fit may use.
pub const AUTOFIT_MIN_SCALE: f32 = 0.4;
/// Indent added for each paragraph level, in slide pixels before scaling.
pub const INDENT_STEP: f32 = 24.0;
/// Default marker colour when a paragraph has no run.
const FALLBACK_COLOR: Color32 = Color32::from_rgb(0x1F, 0x1F, 0x1F);
const FALLBACK_SIZE: f32 = 20.0;

/// A vector-drawn bullet when the bundled font lacks the symbol glyph.
#[derive(Debug, Clone, Copy)]
pub struct VectorBullet {
    pub ch: char,
    pub size: f32,
    pub color: Color32,
}

/// One laid-out marker (bullet or number).
#[derive(Debug, Clone)]
pub struct LaidMarker {
    pub galley: Arc<egui::Galley>,
    /// Screen-space top-left passed to `Painter::galley`. For a right-aligned
    /// marker this is its right edge, as egui expects.
    pub pos: Pos2,
    /// When `Some`, draw this vector symbol instead of `galley`.
    pub vector: Option<VectorBullet>,
}

/// One laid-out paragraph.
#[derive(Debug, Clone)]
pub struct LaidParagraph {
    pub galley: Arc<egui::Galley>,
    /// Screen-space position passed to `Painter::galley`. For centre and right
    /// alignment this is the content centre or right edge.
    pub pos: Pos2,
    pub marker: Option<LaidMarker>,
    pub indent: f32,
    pub height: f32,
    /// Underlined character ranges of this paragraph and their strokes. Drawn by
    /// [`underline_segments`] at the baseline, because egui puts an underline at
    /// the bottom of the line box, which line spacing pushes away from the text.
    pub underlines: Vec<(usize, usize, Stroke)>,
}

/// A link region in screen space.
#[derive(Debug, Clone, PartialEq)]
pub struct LaidLink {
    pub link: Link,
    pub text: String,
    pub rect: Rect,
}

/// The result of laying out a text box.
#[derive(Debug, Clone)]
pub struct LaidText {
    pub paragraphs: Vec<LaidParagraph>,
    pub links: Vec<LaidLink>,
    pub height: f32,
    pub width: f32,
    pub shrink: f32,
}

impl LaidText {
    /// Whether every paragraph fits inside `rect`.
    #[must_use]
    pub fn fits(&self, rect: Rect) -> bool {
        self.height <= rect.height().max(0.0) + 0.5 && self.width <= rect.width().max(0.0) + 0.5
    }
}

/// Lay out `runs` inside `rect`, applying paragraph formatting and shrink-to-fit.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn layout_text(
    ctx: &Context,
    rect: Rect,
    scale: f32,
    runs: &[ResolvedTextRun],
    paragraphs: &[Paragraph],
    align: Align,
    valign: VAlign,
    autofit: Autofit,
) -> LaidText {
    layout_text_with_wrap(
        ctx, rect, scale, runs, paragraphs, align, valign, autofit, true,
    )
}

/// Lay out `runs` inside `rect`, with explicit `wrap` control.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn layout_text_with_wrap(
    ctx: &Context,
    rect: Rect,
    scale: f32,
    runs: &[ResolvedTextRun],
    paragraphs: &[Paragraph],
    align: Align,
    valign: VAlign,
    autofit: Autofit,
    wrap: bool,
) -> LaidText {
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    let rect = if [rect.min.x, rect.min.y, rect.max.x, rect.max.y]
        .iter()
        .all(|value| value.is_finite())
        && rect.width() > 0.0
        && rect.height() > 0.0
    {
        rect
    } else {
        Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(1.0, 1.0))
    };

    let full = layout_at(ctx, rect, scale, runs, paragraphs, align, valign, 1.0, wrap);
    if autofit != Autofit::Shrink || full.fits(rect) {
        return full;
    }

    let floor = layout_at(
        ctx,
        rect,
        scale,
        runs,
        paragraphs,
        align,
        valign,
        AUTOFIT_MIN_SCALE,
        wrap,
    );
    if !floor.fits(rect) {
        return floor;
    }

    // Largest scale in the floor..=1 range that fits.
    let mut low = AUTOFIT_MIN_SCALE;
    let mut high = 1.0;
    for _ in 0..20 {
        let mid = (low + high) * 0.5;
        let candidate = layout_at(ctx, rect, scale, runs, paragraphs, align, valign, mid, wrap);
        if candidate.fits(rect) {
            low = mid;
        } else {
            high = mid;
        }
    }
    layout_at(ctx, rect, scale, runs, paragraphs, align, valign, low, wrap)
}

/// Split runs at newlines. Every paragraph has at least one run so an empty
/// line still gets the font's row height.
fn split_paragraphs(runs: &[ResolvedTextRun]) -> Vec<Vec<ResolvedTextRun>> {
    let mut paragraphs: Vec<Vec<ResolvedTextRun>> = vec![Vec::new()];
    for run in runs.iter().take(2000) {
        let mut first = true;
        for piece in run.text.split('\n') {
            if !first {
                paragraphs.push(Vec::new());
            }
            first = false;
            if !piece.is_empty() {
                let mut part = run.clone();
                part.text = piece.to_owned();
                if let Some(target) = paragraphs.last_mut() {
                    target.push(part);
                }
            }
        }
    }
    for paragraph in &mut paragraphs {
        if paragraph.is_empty() {
            paragraph.push(ResolvedTextRun {
                text: " ".to_owned(),
                size: FALLBACK_SIZE,
                color: FALLBACK_COLOR,
                bold: false,
                italic: false,
                underline: false,
                font: None,
                link: None,
                strikethrough: false,
                baseline: coursecraft_course::Baseline::Normal,
                letter_spacing: 0.0,
            });
        }
    }
    paragraphs
}

/// Superscript and subscript text draws at this fraction of its size.
pub const SCRIPT_SCALE: f32 = 0.65;

#[derive(Clone, Copy, PartialEq, Eq)]
struct LevelState {
    count: u32,
    scheme: NumberScheme,
    start_at: Option<u32>,
}

impl Default for LevelState {
    fn default() -> Self {
        Self {
            count: 0,
            scheme: NumberScheme::ArabicPeriod,
            start_at: None,
        }
    }
}

fn default_scheme_for_level(level: usize) -> NumberScheme {
    match level % 3 {
        0 => NumberScheme::ArabicPeriod,
        1 => NumberScheme::AlphaLcPeriod,
        _ => NumberScheme::RomanLcPeriod,
    }
}

fn marker_for(
    levels: &mut [LevelState; 5],
    previous_list: &mut Option<ListKind>,
    style: &Paragraph,
    level: u8,
) -> Option<String> {
    let level_idx = usize::from(level.min(4));
    match style.list {
        ListKind::None => {
            *levels = [LevelState::default(); 5];
            *previous_list = Some(ListKind::None);
            None
        }
        ListKind::Bullet => {
            *levels = [LevelState::default(); 5];
            *previous_list = Some(ListKind::Bullet);
            Some(if let Some(custom) = &style.bullet {
                custom.clone()
            } else {
                match level_idx % 3 {
                    0 => "•",
                    1 => "◦",
                    _ => "▪",
                }
                .to_owned()
            })
        }
        ListKind::Number => {
            let scheme = style
                .number_scheme
                .unwrap_or_else(|| default_scheme_for_level(level_idx));
            let in_number_list = *previous_list == Some(ListKind::Number);
            let slot = levels.get_mut(level_idx)?;
            let same_list = in_number_list
                && slot.count > 0
                && slot.scheme == scheme
                && (style.start_at.is_none() || slot.start_at == style.start_at);
            if same_list {
                slot.count = slot.count.saturating_add(1);
            } else {
                slot.count = style.start_at.unwrap_or(1).max(1);
                slot.scheme = scheme;
                slot.start_at = style.start_at;
            }
            let number = slot.count;
            for deeper in levels.iter_mut().skip(level_idx.saturating_add(1)) {
                *deeper = LevelState::default();
            }
            *previous_list = Some(ListKind::Number);
            Some(format_number_scheme(number, scheme))
        }
    }
}

fn format_number_scheme(number: u32, scheme: NumberScheme) -> String {
    let value = number.max(1);
    match scheme {
        NumberScheme::ArabicPeriod => format!("{value}."),
        NumberScheme::ArabicParenR => format!("{value})"),
        NumberScheme::AlphaLcPeriod => format!("{}.", alpha(value)),
        NumberScheme::AlphaLcParenR => format!("{})", alpha(value)),
        NumberScheme::AlphaUcPeriod => format!("{}.", alpha(value).to_ascii_uppercase()),
        NumberScheme::RomanLcPeriod => format!("{}.", roman(value)),
        NumberScheme::RomanUcPeriod => format!("{}.", roman(value).to_ascii_uppercase()),
    }
}

fn alpha(mut value: u32) -> String {
    let mut out = String::new();
    while value > 0 {
        let digit = ((value - 1) % 26) as u8;
        out.insert(0, char::from(b'a' + digit));
        value = (value - 1) / 26;
    }
    if out.is_empty() {
        out.push('a');
    }
    out
}

fn roman(value: u32) -> String {
    const PAIRS: &[(u32, &str)] = &[
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];
    let mut value = value;
    let mut out = String::new();
    for (amount, glyph) in PAIRS {
        while value >= *amount {
            out.push_str(glyph);
            value -= amount;
        }
    }
    if out.is_empty() {
        out.push('i');
    }
    out
}

fn halign(align: Align) -> egui::Align {
    match align {
        Align::Left | Align::Justify => egui::Align::LEFT,
        Align::Center => egui::Align::Center,
        Align::Right => egui::Align::RIGHT,
    }
}

/// Spread extra horizontal space on wrapped lines across inter-word spaces
/// so each non-final line fills `target_width`.
///
/// Updates both `row.glyphs` (for hit testing, cursor placement, underlines and
/// links) and `row.visuals.mesh` (which egui actually renders), plus the row
/// and galley bounding boxes.
pub fn justify_galley(galley: &mut Arc<egui::Galley>, target_width: f32) {
    if !target_width.is_finite() || target_width <= 0.0 {
        return;
    }
    let g = Arc::make_mut(galley);
    let row_count = g.rows.len();
    for (row_idx, placed_row) in g.rows.iter_mut().enumerate() {
        let is_last = row_idx + 1 == row_count;
        if is_last || placed_row.ends_with_newline {
            continue;
        }
        let glyphs = &placed_row.glyphs;
        let Some(first_non_ws) = glyphs.iter().position(|glyph| !glyph.chr.is_whitespace()) else {
            continue;
        };
        let Some(last_non_ws) = glyphs.iter().rposition(|glyph| !glyph.chr.is_whitespace()) else {
            continue;
        };
        let effective_end = last_non_ws + 1;
        let content_right = glyphs[..effective_end]
            .iter()
            .map(|glyph| glyph.pos.x + glyph.advance_width)
            .fold(0.0f32, f32::max);
        let extra = target_width - content_right;
        if extra <= 0.5 || !extra.is_finite() {
            continue;
        }
        let space_count = glyphs[first_non_ws..effective_end]
            .iter()
            .filter(|glyph| glyph.chr == ' ')
            .count();
        if space_count == 0 {
            continue;
        }
        let per_space = extra / space_count as f32;
        let row = Arc::make_mut(&mut placed_row.row);
        let glyph_vertex_end = row
            .visuals
            .glyph_vertex_range
            .end
            .min(row.visuals.mesh.vertices.len());
        let mut shift = 0.0f32;
        for (idx, glyph) in row.glyphs.iter_mut().enumerate() {
            glyph.pos.x += shift;
            if shift != 0.0 && !glyph.uv_rect.is_nothing() {
                let v_start = glyph.first_vertex as usize;
                let v_end = v_start.saturating_add(4);
                if v_end <= glyph_vertex_end {
                    for vertex in &mut row.visuals.mesh.vertices[v_start..v_end] {
                        vertex.pos.x += shift;
                    }
                }
            }
            if idx >= first_non_ws && idx < effective_end && glyph.chr == ' ' {
                shift += per_space;
            }
        }
        // Stretch any row-wide strikethrough/underline hline vertices that
        // epaint appended after the glyph vertices.
        if glyph_vertex_end < row.visuals.mesh.vertices.len() && content_right > 0.0 {
            let content_left = row.glyphs[first_non_ws..effective_end]
                .iter()
                .map(|glyph| glyph.pos.x)
                .fold(f32::INFINITY, f32::min);
            let old_span = content_right - content_left;
            if old_span > 0.5 && old_span.is_finite() {
                let scale_x = (target_width - content_left) / old_span;
                for vertex in &mut row.visuals.mesh.vertices[glyph_vertex_end..] {
                    vertex.pos.x = content_left + (vertex.pos.x - content_left) * scale_x;
                }
            }
        }
        row.visuals.mesh_bounds = row.visuals.mesh.calc_bounds();
        row.size.x = target_width;
    }

    let mut rect = egui::Rect::ZERO;
    let mut mesh_bounds = egui::Rect::NOTHING;
    for placed_row in &g.rows {
        rect |= placed_row.rect();
        mesh_bounds |= placed_row
            .visuals
            .mesh_bounds
            .translate(placed_row.pos.to_vec2());
    }
    g.rect = rect;
    g.mesh_bounds = mesh_bounds;
}

fn is_vector_bullet_char(ch: char) -> bool {
    matches!(
        ch,
        '★' | '☆'
            | '✔'
            | '✓'
            | '➤'
            | '➢'
            | '➔'
            | '→'
            | '◆'
            | '◇'
            | '►'
            | '▶'
            | '▪'
            | '■'
            | '□'
            | '●'
            | '○'
            | '•'
    )
}

fn marker_needs_vector(galley: &egui::Galley, ch: char) -> bool {
    if is_vector_bullet_char(ch) {
        return true;
    }
    galley
        .rows
        .first()
        .and_then(|row| row.glyphs.first())
        .is_some_and(|glyph| glyph.chr == '\u{FFFD}' || glyph.uv_rect.size.x <= 0.0)
}

/// Draw one bullet or number marker (vector symbol when the font lacks it,
/// otherwise the laid-out galley). Shared across text boxes, shapes and tables.
pub fn paint_marker(painter: &egui::Painter, marker: &LaidMarker, fallback_color: Color32) {
    if let Some(vb) = marker.vector {
        let r = (vb.size * 0.28).max(1.8);
        let cx = marker.pos.x - r * 1.1;
        let cy = marker.pos.y + vb.size * 0.48;
        let color = vb.color;
        let stroke = Stroke::new((r * 0.35).max(1.0), color);
        match vb.ch {
            '★' | '☆' => {
                let mut pts = Vec::with_capacity(10);
                for i in 0..10 {
                    let angle =
                        -std::f32::consts::FRAC_PI_2 + (i as f32) * std::f32::consts::PI / 5.0;
                    let rad = if i % 2 == 0 { r * 1.25 } else { r * 0.52 };
                    pts.push(pos2(cx + rad * angle.cos(), cy + rad * angle.sin()));
                }
                if vb.ch == '★' {
                    painter.add(egui::Shape::convex_polygon(pts, color, Stroke::NONE));
                } else {
                    painter.add(egui::Shape::closed_line(pts, stroke));
                }
            }
            '✔' | '✓' => {
                let pts = vec![
                    pos2(cx - r * 0.9, cy + r * 0.05),
                    pos2(cx - r * 0.2, cy + r * 0.75),
                    pos2(cx + r * 1.0, cy - r * 0.65),
                ];
                painter.add(egui::Shape::line(
                    pts,
                    Stroke::new((r * 0.45).max(1.2), color),
                ));
            }
            '➤' | '➢' | '►' | '▶' => {
                let pts = vec![
                    pos2(cx - r * 0.75, cy - r * 0.85),
                    pos2(cx + r * 0.95, cy),
                    pos2(cx - r * 0.75, cy + r * 0.85),
                ];
                painter.add(egui::Shape::convex_polygon(pts, color, Stroke::NONE));
            }
            '➔' | '→' => {
                let line_y = cy;
                painter.line_segment(
                    [pos2(cx - r * 0.95, line_y), pos2(cx + r * 0.5, line_y)],
                    stroke,
                );
                let head = vec![
                    pos2(cx + r * 0.2, cy - r * 0.6),
                    pos2(cx + r * 1.0, cy),
                    pos2(cx + r * 0.2, cy + r * 0.6),
                ];
                painter.add(egui::Shape::convex_polygon(head, color, Stroke::NONE));
            }
            '◆' | '◇' => {
                let pts = vec![
                    pos2(cx, cy - r),
                    pos2(cx + r, cy),
                    pos2(cx, cy + r),
                    pos2(cx - r, cy),
                ];
                if vb.ch == '◆' {
                    painter.add(egui::Shape::convex_polygon(pts, color, Stroke::NONE));
                } else {
                    painter.add(egui::Shape::closed_line(pts, stroke));
                }
            }
            '▪' | '■' | '□' => {
                let side = if vb.ch == '▪' { r * 0.72 } else { r * 0.9 };
                let rect = Rect::from_center_size(pos2(cx, cy), egui::vec2(side * 2.0, side * 2.0));
                if vb.ch == '□' {
                    painter.rect_stroke(
                        rect,
                        egui::CornerRadius::ZERO,
                        stroke,
                        egui::StrokeKind::Inside,
                    );
                } else {
                    painter.rect_filled(rect, egui::CornerRadius::ZERO, color);
                }
            }
            '○' => {
                painter.circle_stroke(pos2(cx, cy), r * 0.75, stroke);
            }
            _ => {
                let rad = if vb.ch == '•' { r * 0.65 } else { r * 0.8 };
                painter.circle_filled(pos2(cx, cy), rad, color);
            }
        }
    } else {
        painter.galley(marker.pos, marker.galley.clone(), fallback_color);
    }
}

/// Distance from a galley's top to the baseline of its first glyph.
fn first_baseline(galley: &egui::Galley) -> Option<f32> {
    let row = galley.rows.first()?;
    let glyph = row.glyphs.first()?;
    let y = row.pos.y + glyph.pos.y;
    y.is_finite().then_some(y)
}

fn marker_text(
    ctx: &Context,
    text: &str,
    run: &ResolvedTextRun,
    scale: f32,
    shrink: f32,
    marker_size: f32,
    color: Color32,
) -> Option<(Arc<egui::Galley>, f32)> {
    let size = (marker_size.clamp(1.0, 400.0) * scale * shrink).max(1.0);
    let font_id =
        crate::font::slide_font_named(ctx, run.font.as_deref(), run.bold, run.italic, size);
    let mut job = LayoutJob {
        halign: egui::Align::RIGHT,
        break_on_newline: false,
        ..Default::default()
    };
    job.append(
        text,
        0.0,
        TextFormat {
            font_id,
            color,
            ..Default::default()
        },
    );
    let galley = ctx.fonts_mut(|fonts| fonts.layout_job(job));
    let width = galley.size().x;
    Some((galley, width))
}

#[allow(clippy::too_many_arguments)]
fn layout_at(
    ctx: &Context,
    rect: Rect,
    scale: f32,
    runs: &[ResolvedTextRun],
    paragraphs: &[Paragraph],
    align: Align,
    valign: VAlign,
    shrink: f32,
    wrap: bool,
) -> LaidText {
    let shrink = if shrink.is_finite() {
        shrink.clamp(AUTOFIT_MIN_SCALE, 1.0)
    } else {
        1.0
    };
    let split = split_paragraphs(runs);
    let mut laid = Vec::with_capacity(split.len());
    let mut links = Vec::new();
    let mut y = 0.0f32;
    let mut max_width = 0.0f32;
    let mut levels = [LevelState::default(); 5];
    let mut previous_list: Option<ListKind> = None;

    for (index, paragraph_runs) in split.iter().enumerate() {
        let style = paragraphs.get(index).cloned().unwrap_or_default();
        let level = style.level.min(4);
        let spacing = if style.line_spacing.is_finite() {
            style.line_spacing.clamp(0.8, 3.0)
        } else {
            1.0
        };
        let space_before = if style.space_before.is_finite() {
            style.space_before.clamp(0.0, 200.0)
        } else {
            0.0
        };
        let space_after = if style.space_after.is_finite() {
            style.space_after.clamp(0.0, 200.0)
        } else {
            0.0
        };
        let step = INDENT_STEP * scale * shrink;
        let indent = f32::from(level) * step;
        let text_indent = if style.list == ListKind::None {
            indent
        } else {
            indent + step
        };
        let content_width = (rect.width() - text_indent).max(1.0);
        let marker = marker_for(&mut levels, &mut previous_list, &style, level);

        let mut job = LayoutJob {
            halign: halign(align),
            wrap: TextWrapping {
                max_width: if wrap { content_width } else { f32::INFINITY },
                ..Default::default()
            },
            break_on_newline: false,
            ..Default::default()
        };
        let mut spans: Vec<(usize, usize, Link, String)> = Vec::new();
        let mut underlines: Vec<(usize, usize, Stroke)> = Vec::new();
        let mut char_count = 0usize;
        for run in paragraph_runs {
            let start = char_count;
            char_count = char_count.saturating_add(run.text.chars().count());
            let size = (run.size.clamp(1.0, 400.0) * scale * shrink).max(1.0);
            // Superscript and subscript draw smaller; the format's valign moves them.
            let (size, valign) = match run.baseline {
                coursecraft_course::Baseline::Normal => (size, egui::Align::BOTTOM),
                coursecraft_course::Baseline::Superscript => {
                    ((size * SCRIPT_SCALE).max(1.0), egui::Align::TOP)
                }
                coursecraft_course::Baseline::Subscript => {
                    ((size * SCRIPT_SCALE).max(1.0), egui::Align::BOTTOM)
                }
            };
            let letter_spacing =
                coursecraft_course::clamp_letter_spacing(run.letter_spacing) * scale * shrink;
            let strike = if run.strikethrough {
                Stroke::new((scale * shrink).max(1.0), run.color)
            } else {
                Stroke::NONE
            };
            let font_id =
                crate::font::slide_font_named(ctx, run.font.as_deref(), run.bold, run.italic, size);
            let row_height = ctx.fonts_mut(|fonts| fonts.row_height(&font_id));
            let line_height = (row_height * spacing).max(size * 0.5);
            let underline = if run.underline || run.link.is_some() {
                Stroke::new((scale * shrink).max(1.0), run.color)
            } else {
                Stroke::NONE
            };
            job.append(
                &run.text,
                0.0,
                TextFormat {
                    font_id,
                    color: run.color,
                    line_height: Some(line_height),
                    strikethrough: strike,
                    extra_letter_spacing: letter_spacing,
                    valign,
                    ..Default::default()
                },
            );
            if underline != Stroke::NONE {
                underlines.push((start, char_count, underline));
            }
            if let Some(link) = &run.link {
                spans.push((start, char_count, link.clone(), run.text.clone()));
            }
        }
        let mut galley = ctx.fonts_mut(|fonts| fonts.layout_job(job));
        if align == Align::Justify && wrap {
            justify_galley(&mut galley, content_width);
        }
        let galley_h = galley.size().y;
        let indent_width = text_indent + galley.size().x;
        max_width = max_width.max(indent_width);

        let x = match align {
            Align::Left | Align::Justify => rect.left() + text_indent,
            Align::Center => rect.left() + text_indent + content_width * 0.5,
            Align::Right => rect.right(),
        };
        let top = y + space_before;
        let paragraph_pos = pos2(x, rect.top() + top);
        let paragraph_galley = galley.clone();
        let marker = marker.and_then(|text| {
            let marker_run = paragraph_runs.first()?;
            let marker_color = style
                .bullet_color
                .as_ref()
                .and_then(crate::color::parse_color)
                .unwrap_or(marker_run.color);
            let marker_size = style.bullet_size.unwrap_or(marker_run.size);
            let (galley, width) = marker_text(
                ctx,
                &text,
                marker_run,
                scale,
                shrink,
                marker_size,
                marker_color,
            )?;
            let right = rect.left() + text_indent - (6.0 * scale * shrink).max(1.0);
            let _ = width;
            // Sit the marker on the first line's baseline. Line spacing makes the
            // paragraph's rows taller than the marker's, so equal tops would
            // leave the marker floating above the text.
            let shift = match (first_baseline(&paragraph_galley), first_baseline(&galley)) {
                (Some(text), Some(mark)) => text - mark,
                _ => 0.0,
            };
            let mut chars = text.chars();
            let vector = match (chars.next(), chars.next()) {
                (Some(ch), None)
                    if style.list == ListKind::Bullet && marker_needs_vector(&galley, ch) =>
                {
                    Some(VectorBullet {
                        ch,
                        size: (marker_size.clamp(1.0, 400.0) * scale * shrink).max(1.0),
                        color: marker_color,
                    })
                }
                _ => None,
            };
            Some(LaidMarker {
                galley,
                pos: pos2(right, rect.top() + top + shift),
                vector,
            })
        });
        laid.push(LaidParagraph {
            galley: galley.clone(),
            pos: paragraph_pos,
            marker,
            indent: text_indent,
            height: space_before + galley_h + space_after,
            underlines,
        });

        links.extend(collect_links(&galley, paragraph_pos, &spans));
        y = y + space_before + galley_h + space_after;
    }

    let y_offset = match valign {
        VAlign::Top => 0.0,
        VAlign::Middle => ((rect.height() - y).max(0.0)) * 0.5,
        VAlign::Bottom => (rect.height() - y).max(0.0),
    };
    if y_offset > 0.0 {
        for paragraph in &mut laid {
            paragraph.pos.y += y_offset;
            if let Some(marker) = &mut paragraph.marker {
                marker.pos.y += y_offset;
            }
        }
        for link in &mut links {
            link.rect = link.rect.translate(egui::vec2(0.0, y_offset));
        }
    }

    LaidText {
        paragraphs: laid,
        links,
        height: y,
        width: max_width,
        shrink,
    }
}

/// Turn per-character glyph boxes into one rect per linked span per row.
/// Screen-space underline strokes for a laid paragraph drawn at `origin`:
/// one segment per run of underlined glyphs on a row, just below the baseline.
#[must_use]
pub fn underline_segments(paragraph: &LaidParagraph, origin: Pos2) -> Vec<([Pos2; 2], Stroke)> {
    let mut out = Vec::new();
    if paragraph.underlines.is_empty() {
        return out;
    }
    let mut char_index = 0usize;
    for row in &paragraph.galley.rows {
        let mut active: Option<(f32, f32, f32, Stroke)> = None;
        for glyph in &row.glyphs {
            let stroke = paragraph
                .underlines
                .iter()
                .find(|(start, end, _)| char_index >= *start && char_index < *end)
                .map(|(_, _, stroke)| *stroke);
            let left = origin.x + row.pos.x + glyph.pos.x;
            let right = left + glyph.advance_width;
            let descent = (glyph.font_height - glyph.font_ascent).max(0.0);
            let y = origin.y + row.pos.y + glyph.pos.y + (descent * 0.45).max(1.0);
            match (stroke, &mut active) {
                (Some(stroke), Some((_, end, _, current))) if *current == stroke => {
                    *end = right;
                }
                (Some(stroke), _) => {
                    if let Some((start, end, y0, s0)) = active.take() {
                        out.push(([pos2(start, y0), pos2(end, y0)], s0));
                    }
                    active = Some((left, right, y, stroke));
                }
                (None, _) => {
                    if let Some((start, end, y0, s0)) = active.take() {
                        out.push(([pos2(start, y0), pos2(end, y0)], s0));
                    }
                }
            }
            char_index = char_index.saturating_add(1);
        }
        if let Some((start, end, y0, s0)) = active.take() {
            out.push(([pos2(start, y0), pos2(end, y0)], s0));
        }
    }
    out
}

fn collect_links(
    galley: &egui::Galley,
    origin: Pos2,
    spans: &[(usize, usize, Link, String)],
) -> Vec<LaidLink> {
    let mut out: Vec<LaidLink> = Vec::new();
    let mut char_index = 0usize;
    for row in &galley.rows {
        let mut active: Option<(usize, Rect)> = None;
        for glyph in &row.glyphs {
            let span = spans
                .iter()
                .position(|(start, end, _, _)| char_index >= *start && char_index < *end);
            if let Some(span_index) = span {
                let rect = Rect::from_min_size(
                    pos2(origin.x + row.pos.x + glyph.pos.x, origin.y + row.pos.y),
                    egui::vec2(glyph.advance_width, row.size.y),
                );
                match &mut active {
                    Some((index, current)) if *index == span_index => {
                        *current = current.union(rect);
                    }
                    _ => {
                        flush_link(&mut out, &mut active, spans);
                        active = Some((span_index, rect));
                    }
                }
            } else {
                flush_link(&mut out, &mut active, spans);
            }
            char_index = char_index.saturating_add(1);
        }
        flush_link(&mut out, &mut active, spans);
    }
    out
}

fn flush_link(
    out: &mut Vec<LaidLink>,
    active: &mut Option<(usize, Rect)>,
    spans: &[(usize, usize, Link, String)],
) {
    if let Some((index, rect)) = active.take()
        && let Some((_, _, link, text)) = spans.get(index)
    {
        out.push(LaidLink {
            link: link.clone(),
            text: text.clone(),
            rect,
        });
    }
}

/// Fill each text item's link regions from the same layout the painter uses.
///
/// Runs in slide coordinates with `scale = 1`, then converts the screen-space
/// result back to fractions of the item frame so hit testing, rotation and the
/// focus ring all agree with what is drawn. No play-time decision is made here.
pub fn fill_item_links(ctx: &Context, items: &mut [DrawItem]) {
    for item in items {
        let mut found = Vec::new();
        let frame = if item.rect.is_positive() {
            item.rect
        } else {
            validate_frame(&coursecraft_course::Rect {
                x: item.rect.min.x,
                y: item.rect.min.y,
                w: item.rect.width(),
                h: item.rect.height(),
            })
            .unwrap_or(Rect::NOTHING)
        };
        if !frame.is_positive() {
            item.links = Vec::new();
            continue;
        }
        match &item.visual {
            Visual::Text {
                runs,
                align,
                valign,
                paragraphs,
                autofit,
                ..
            } => {
                let laid =
                    layout_text(ctx, frame, 1.0, runs, paragraphs, *align, *valign, *autofit);
                found.extend(to_fractions(frame, laid.links));
            }
            Visual::Shape {
                shape,
                adjust,
                text: Some(text),
                ..
            } => {
                let (tx, ty, tw, th) = crate::shape_geometry::shape_text_rect(
                    *shape,
                    frame.width(),
                    frame.height(),
                    adjust,
                );
                let text_rect = Rect::from_min_size(
                    pos2(frame.min.x + tx, frame.min.y + ty),
                    egui::vec2(tw, th),
                );
                if text_rect.is_positive() {
                    let laid = layout_text(
                        ctx,
                        text_rect,
                        1.0,
                        &text.runs,
                        &text.paragraphs,
                        text.align,
                        text.valign,
                        text.autofit,
                    );
                    found.extend(to_fractions(frame, laid.links));
                }
            }
            _ => {}
        }
        item.links = found;
    }
}

fn to_fractions(frame: Rect, links: Vec<LaidLink>) -> Vec<ResolvedLinkRect> {
    let width = frame.width();
    let height = frame.height();
    links
        .into_iter()
        .filter_map(|link| {
            if !(width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0) {
                return None;
            }
            let rect = Rect::from_min_max(
                pos2(
                    ((link.rect.min.x - frame.min.x) / width).clamp(0.0, 1.0),
                    ((link.rect.min.y - frame.min.y) / height).clamp(0.0, 1.0),
                ),
                pos2(
                    ((link.rect.max.x - frame.min.x) / width).clamp(0.0, 1.0),
                    ((link.rect.max.y - frame.min.y) / height).clamp(0.0, 1.0),
                ),
            );
            rect.is_positive().then_some(ResolvedLinkRect {
                link: link.link,
                rect,
                text: link.text,
            })
        })
        .collect()
}

/// Build an [`egui::text::LayoutJob`] for inline text editing that preserves
/// per-run fonts, sizes, colours, bold/italic, underline, strikethrough,
/// superscript/subscript, letter spacing, and paragraph alignment/line spacing
/// while keeping `job.text` character-for-character identical to the buffer.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn inline_edit_layout_job(
    ctx: &Context,
    scale: f32,
    shrink: f32,
    runs: &[ResolvedTextRun],
    paragraphs: &[Paragraph],
    align: Align,
    wrap_width: f32,
    multiline: bool,
) -> LayoutJob {
    let shrink = if shrink.is_finite() {
        shrink.clamp(AUTOFIT_MIN_SCALE, 1.0)
    } else {
        1.0
    };
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    let mut job = LayoutJob {
        halign: halign(align),
        wrap: TextWrapping {
            max_width: if multiline {
                wrap_width.max(1.0)
            } else {
                f32::INFINITY
            },
            ..Default::default()
        },
        break_on_newline: multiline,
        ..Default::default()
    };
    let mut paragraph_index = 0usize;
    for run in runs {
        let size = (run.size.clamp(1.0, 400.0) * scale * shrink).max(1.0);
        let (size, valign) = match run.baseline {
            coursecraft_course::Baseline::Normal => (size, egui::Align::BOTTOM),
            coursecraft_course::Baseline::Superscript => {
                ((size * SCRIPT_SCALE).max(1.0), egui::Align::TOP)
            }
            coursecraft_course::Baseline::Subscript => {
                ((size * SCRIPT_SCALE).max(1.0), egui::Align::BOTTOM)
            }
        };
        let letter_spacing =
            coursecraft_course::clamp_letter_spacing(run.letter_spacing) * scale * shrink;
        let strike = if run.strikethrough {
            Stroke::new((scale * shrink).max(1.0), run.color)
        } else {
            Stroke::NONE
        };
        let underline = if run.underline || run.link.is_some() {
            Stroke::new((scale * shrink).max(1.0), run.color)
        } else {
            Stroke::NONE
        };
        let font_id =
            crate::font::slide_font_named(ctx, run.font.as_deref(), run.bold, run.italic, size);
        let row_height = ctx.fonts_mut(|fonts| fonts.row_height(&font_id));

        if run.text.is_empty() {
            let style = paragraphs.get(paragraph_index).cloned().unwrap_or_default();
            let spacing = if style.line_spacing.is_finite() {
                style.line_spacing.clamp(0.8, 3.0)
            } else {
                1.0
            };
            let line_height = (row_height * spacing).max(size * 0.5);
            job.append(
                "",
                0.0,
                TextFormat {
                    font_id,
                    color: run.color,
                    line_height: Some(line_height),
                    strikethrough: strike,
                    underline,
                    extra_letter_spacing: letter_spacing,
                    valign,
                    ..Default::default()
                },
            );
            continue;
        }

        for piece in run.text.split_inclusive('\n') {
            let style = paragraphs.get(paragraph_index).cloned().unwrap_or_default();
            let spacing = if style.line_spacing.is_finite() {
                style.line_spacing.clamp(0.8, 3.0)
            } else {
                1.0
            };
            let line_height = (row_height * spacing).max(size * 0.5);
            job.append(
                piece,
                0.0,
                TextFormat {
                    font_id: font_id.clone(),
                    color: run.color,
                    line_height: Some(line_height),
                    strikethrough: strike,
                    underline,
                    extra_letter_spacing: letter_spacing,
                    valign,
                    ..Default::default()
                },
            );
            if piece.ends_with('\n') {
                paragraph_index = paragraph_index.saturating_add(1);
            }
        }
    }
    job
}
