//! M46 phase 4: effect and option icons drawn in code (docs/plans/m46-animation-system.md ›
//! Editor).
//!
//! An effect's icon is a small card run through the effect's own sample function
//! (`coursecraft_logic::effects::sample_effect`), so every registry effect gets an icon that
//! shows what it does: a ghost trail for movement, the mask for reveals. The colour says the
//! kind (entrance green, exit red). Option icons are Phosphor glyphs, turned where the bundled
//! subset has no diagonal or diamond, and a drawn wheel for spokes.

use coursecraft_course::{
    AnimEffect, Direction, EffectOptionDef, InOut, Orientation, Rect as SlideRect, RevealShape,
    Size,
};
use coursecraft_logic::effects::{EffectCtx, sample_effect};
use coursecraft_logic::{AnimationPhase, ObjectSample};
use coursecraft_render::Icon;
use egui::epaint::TextShape;
use egui::{Color32, FontFamily, FontId, Painter, Pos2, Rect, Shape, Stroke, Vec2, pos2, vec2};

/// The small slide the icon card plays on, in its own units.
const STAGE: Vec2 = vec2(120.0, 90.0);
/// The card's resting frame on that stage.
const CARD: SlideRect = SlideRect {
    x: 36.0,
    y: 27.0,
    w: 48.0,
    h: 36.0,
};

/// One drawn piece of an effect icon: a convex polygon in stage units, with its opacity
/// and colour wash.
#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    pub points: Vec<Pos2>,
    pub alpha: f32,
    pub tint: Option<([u8; 3], f32)>,
}

/// The sample of `effect` when the object is `shown` of the way in (0 away, 1 at rest).
fn sample_at(effect: &AnimEffect, exit: bool, shown: f32) -> ObjectSample {
    let (phase, eased) = if exit {
        (AnimationPhase::Exiting, 1.0 - shown)
    } else {
        (AnimationPhase::Entering, shown)
    };
    sample_effect(
        effect,
        &EffectCtx {
            phase,
            eased,
            frame: CARD,
            slide_size: Size {
                w: STAGE.x as u32,
                h: STAGE.y as u32,
            },
        },
    )
}

/// Whether `s` moves, scales or turns the card (a trail is worth drawing).
fn moves(s: &ObjectSample) -> bool {
    let ObjectSample {
        offset,
        scale,
        rotation_deg,
        ..
    } = s;
    offset.0.abs() + offset.1.abs() > 1.0
        || (scale.0 - 1.0).abs() + (scale.1 - 1.0).abs() > 0.05
        || rotation_deg.abs() > 2.0
}

/// Clip a convex polygon (frame fractions) to the half-plane `keep(p) >= 0`.
fn clip(points: &[(f32, f32)], keep: impl Fn((f32, f32)) -> f32) -> Vec<(f32, f32)> {
    let mut out = Vec::with_capacity(points.len() + 2);
    for (i, &a) in points.iter().enumerate() {
        let b = points[(i + 1) % points.len()];
        let (ka, kb) = (keep(a), keep(b));
        if ka >= 0.0 {
            out.push(a);
        }
        if (ka >= 0.0) != (kb >= 0.0) {
            let t = ka / (ka - kb);
            out.push((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t));
        }
    }
    out
}

/// Clip to the rectangle `[x0, x1] × [y0, y1]` (frame fractions).
fn clip_rect(points: &[(f32, f32)], x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<(f32, f32)> {
    let p = clip(points, |p| p.0 - x0);
    let p = clip(&p, |p| x1 - p.0);
    let p = clip(&p, |p| p.1 - y0);
    clip(&p, |p| y1 - p.1)
}

/// The card as `s` draws it: its visible polygons in stage units.
fn pieces_of(s: &ObjectSample, alpha_floor: f32) -> Vec<Piece> {
    let s = s.clone().sanitized();
    let unit = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
    let mut polys: Vec<Vec<(f32, f32)>> = match &s.mask {
        Some(mask) => mask.polys().iter().map(|p| p.0.clone()).collect(),
        None => vec![unit],
    };
    let (x0, y0, x1, y1) = s.clip.map_or((0.0, 0.0, 1.0, 1.0), |c| {
        (c.min_x, c.min_y, c.max_x, c.max_y)
    });
    polys = polys
        .iter()
        .map(|p| clip_rect(p, x0.max(0.0), y0.max(0.0), x1.min(1.0), y1.min(1.0)))
        .filter(|p| p.len() >= 3)
        .collect();
    let centre = pos2(CARD.x + CARD.w / 2.0, CARD.y + CARD.h / 2.0);
    let (sin, cos) = s.rotation_deg.to_radians().sin_cos();
    let alpha = s.opacity.clamp(0.0, 1.0).max(alpha_floor);
    let tint = s.tint.map(|t| (t.rgb, t.amount.clamp(0.0, 1.0)));
    polys
        .into_iter()
        .map(|poly| Piece {
            points: poly
                .into_iter()
                .map(|(u, v)| {
                    let d = vec2(
                        (CARD.x + u * CARD.w - centre.x) * s.scale.0,
                        (CARD.y + v * CARD.h - centre.y) * s.scale.1,
                    );
                    let turned = vec2(d.x * cos - d.y * sin, d.x * sin + d.y * cos);
                    centre + turned + vec2(s.offset.0, s.offset.1)
                })
                .collect(),
            alpha,
            tint,
        })
        .collect()
}

/// What an effect icon draws, in stage units: `None` shows the still icon (a ghost of the
/// way in when the effect moves, then the card part way in), `Some(shown)` one frame of the
/// hover loop.
#[must_use]
pub fn effect_pieces(effect: &AnimEffect, exit: bool, at: Option<f32>) -> Vec<Piece> {
    match at {
        Some(shown) => pieces_of(&sample_at(effect, exit, shown.clamp(0.0, 1.0)), 0.0),
        None => {
            let early = sample_at(effect, exit, 0.25);
            let mid = sample_at(effect, exit, 0.6);
            let mut out = Vec::new();
            if moves(&early) {
                out.extend(pieces_of(&early, 0.0).into_iter().map(|p| Piece {
                    alpha: p.alpha.min(0.3),
                    ..p
                }));
            }
            out.extend(pieces_of(&mid, 0.45));
            out
        }
    }
}

/// The hover loop's frame at `time` seconds: 0→1 over a second, then a short hold at rest.
#[must_use]
pub fn hover_shown(time: f64) -> f32 {
    let t = (time % 1.4) as f32;
    (t / 1.0).clamp(0.0, 1.0)
}

fn washed(color: Color32, tint: Option<([u8; 3], f32)>, alpha: f32) -> Color32 {
    let [r, g, b, _] = color.to_array();
    let (r, g, b) = match tint {
        Some((rgb, amount)) => {
            let mix = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * amount) as u8;
            (mix(r, rgb[0]), mix(g, rgb[1]), mix(b, rgb[2]))
        }
        None => (r, g, b),
    };
    Color32::from_rgba_unmultiplied(r, g, b, (alpha.clamp(0.0, 1.0) * 255.0) as u8)
}

/// Paint `effect`'s icon into `art`: the card's resting outline in `rest`, the effect in
/// `ink`. `at` as in [`effect_pieces`].
pub fn paint_effect(
    painter: &Painter,
    art: Rect,
    effect: &AnimEffect,
    exit: bool,
    ink: Color32,
    rest: Color32,
    at: Option<f32>,
) {
    let k = (art.width() / STAGE.x).min(art.height() / STAGE.y);
    let origin = art.center() - STAGE * k / 2.0;
    let map = |p: Pos2| origin + p.to_vec2() * k;
    let painter = painter.with_clip_rect(art.intersect(painter.clip_rect()));
    let outline = Rect::from_min_max(
        map(pos2(CARD.x, CARD.y)),
        map(pos2(CARD.x + CARD.w, CARD.y + CARD.h)),
    );
    painter.rect_stroke(
        outline,
        2.0,
        Stroke::new(1.0, rest),
        egui::StrokeKind::Inside,
    );
    for piece in effect_pieces(effect, exit, at) {
        let points: Vec<Pos2> = piece.points.iter().map(|&p| map(p)).collect();
        if points.iter().all(|p| p.x.is_finite() && p.y.is_finite()) {
            let color = washed(ink, piece.tint, piece.alpha);
            // Opaque pieces get a hairline edge in their own colour, which hides the
            // anti-aliasing seams between neighbouring mask pieces.
            let edge = if piece.alpha >= 0.99 {
                Stroke::new(0.6, color)
            } else {
                Stroke::NONE
            };
            painter.add(Shape::convex_polygon(points, color, edge));
        }
    }
}

/// An option icon.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Glyph {
    Icon(Icon),
    /// A Phosphor glyph turned clockwise by this many degrees.
    Turned(Icon, f32),
    /// A wheel with this many spokes.
    Spokes(u8),
}

/// Paint `glyph` centred on `centre`, `size` points tall.
pub fn paint_glyph(painter: &Painter, centre: Pos2, size: f32, color: Color32, glyph: Glyph) {
    let font = FontId::new(size, FontFamily::Name("phosphor".into()));
    match glyph {
        Glyph::Icon(icon) => {
            painter.text(
                centre,
                egui::Align2::CENTER_CENTER,
                icon.codepoint(),
                font,
                color,
            );
        }
        Glyph::Turned(icon, degrees) => {
            let galley = painter.layout_no_wrap(icon.codepoint().to_string(), font, color);
            let half = galley.size() / 2.0;
            let angle = degrees.to_radians();
            let (sin, cos) = angle.sin_cos();
            let turned = vec2(half.x * cos - half.y * sin, half.x * sin + half.y * cos);
            painter.add(TextShape::new(centre - turned, galley, color).with_angle(angle));
        }
        Glyph::Spokes(n) => {
            let r = size * 0.4;
            let stroke = Stroke::new(1.5, color);
            painter.circle_stroke(centre, r, stroke);
            let n = n.max(1);
            for i in 0..n {
                let a = (-90.0 + f32::from(i) * 360.0 / f32::from(n)).to_radians();
                painter.line_segment([centre, centre + vec2(a.cos(), a.sin()) * r], stroke);
            }
        }
    }
}

/// One value of an effect option, as the Effect Options menu offers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionValue {
    From(Direction),
    Shape(RevealShape),
    InOut(InOut),
    Spokes(u8),
    Orientation(Orientation),
}

impl OptionValue {
    /// Every value `option` allows, in menu order.
    #[must_use]
    pub fn all(option: &EffectOptionDef) -> Vec<Self> {
        match *option {
            EffectOptionDef::Direction { allowed, .. } => {
                allowed.iter().copied().map(Self::From).collect()
            }
            EffectOptionDef::Shape { allowed, .. } => {
                allowed.iter().copied().map(Self::Shape).collect()
            }
            EffectOptionDef::InOut { .. } => InOut::ALL.into_iter().map(Self::InOut).collect(),
            EffectOptionDef::Spokes { allowed, .. } => {
                allowed.iter().copied().map(Self::Spokes).collect()
            }
            EffectOptionDef::Orientation { .. } => Orientation::ALL
                .into_iter()
                .map(Self::Orientation)
                .collect(),
        }
    }

    /// The English label (shown through `tl!`); spokes are counted by the caller.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::From(d) => d.label(),
            Self::Shape(s) => s.label(),
            Self::InOut(InOut::In) => "In",
            Self::InOut(InOut::Out) => "Out",
            Self::Spokes(1) => "1 spoke",
            Self::Spokes(_) => "{n} spokes",
            Self::Orientation(o) => o.label(),
        }
    }

    /// The icon. A direction's arrow shows the way the object moves: away from the side for
    /// an entrance, toward it for an exit.
    #[must_use]
    pub fn glyph(self, exit: bool) -> Glyph {
        match self {
            Self::From(d) => {
                let (x, y) = d.vector();
                let (x, y) = if exit { (x, y) } else { (-x, -y) };
                Glyph::Turned(Icon::ArrowRight, y.atan2(x).to_degrees())
            }
            Self::Shape(RevealShape::Circle) => Glyph::Icon(Icon::Circle),
            Self::Shape(RevealShape::Box) => Glyph::Icon(Icon::Square),
            Self::Shape(RevealShape::Diamond) => Glyph::Turned(Icon::Square, 45.0),
            Self::Shape(RevealShape::Plus) => Glyph::Icon(Icon::Plus),
            Self::InOut(InOut::In) => Glyph::Icon(Icon::ArrowsIn),
            Self::InOut(InOut::Out) => Glyph::Icon(Icon::ArrowsOut),
            Self::Spokes(n) => Glyph::Spokes(n),
            Self::Orientation(Orientation::Horizontal) => Glyph::Icon(Icon::Rows),
            Self::Orientation(Orientation::Vertical) => Glyph::Turned(Icon::Rows, 90.0),
        }
    }

    /// Whether `effect` plays with this value (stored or default).
    #[must_use]
    pub fn is_set(self, effect: &AnimEffect) -> bool {
        match self {
            Self::From(d) => effect.direction() == Some(d),
            Self::Shape(s) => effect.shape() == Some(s),
            Self::InOut(v) => effect.inout() == Some(v),
            Self::Spokes(n) => effect.spokes() == Some(n),
            Self::Orientation(o) => effect.orientation() == Some(o),
        }
    }

    /// Store this value in `effect`.
    pub fn set(self, effect: &mut AnimEffect) {
        match self {
            Self::From(d) => effect.from = Some(d),
            Self::Shape(s) => effect.shape = Some(s),
            Self::InOut(v) => effect.inout = Some(v),
            Self::Spokes(n) => effect.spokes = Some(n),
            Self::Orientation(o) => effect.orientation = Some(o),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coursecraft_course::EFFECTS;

    fn variants() -> Vec<AnimEffect> {
        let mut out = Vec::new();
        for def in EFFECTS {
            out.push(AnimEffect::new(def.id));
            for option in def.options {
                for value in OptionValue::all(option) {
                    let mut e = AnimEffect::new(def.id);
                    value.set(&mut e);
                    out.push(e);
                }
            }
        }
        out
    }

    #[test]
    fn every_effect_draws_a_finite_card_on_the_stage() {
        for e in variants() {
            for exit in [false, true] {
                for at in [None, Some(0.0), Some(0.5), Some(1.0)] {
                    let pieces = effect_pieces(&e, exit, at);
                    for p in &pieces {
                        assert!(p.points.len() >= 3, "`{}` {at:?}", e.id);
                        assert!(
                            p.points.iter().all(|q| q.x.is_finite() && q.y.is_finite()),
                            "`{}` {at:?}",
                            e.id
                        );
                    }
                    if at.is_none() || at == Some(1.0) {
                        assert!(!pieces.is_empty(), "`{}` draws nothing", e.id);
                    }
                }
            }
        }
    }

    #[test]
    fn icons_tell_effects_apart() {
        let pieces: Vec<(&str, Vec<Piece>)> = EFFECTS
            .iter()
            .map(|d| (d.id, effect_pieces(&AnimEffect::new(d.id), false, None)))
            .collect();
        let distinct = pieces
            .iter()
            .enumerate()
            .filter(|(i, (_, p))| pieces[..*i].iter().all(|(_, q)| q != p))
            .count();
        // Appear and Fade both show a half-faded card; every other icon is its own.
        assert!(distinct >= EFFECTS.len() - 2, "{distinct} distinct icons");
        // A reveal is drawn through its mask, not as a whole card.
        let wipe = effect_pieces(&AnimEffect::new("wipe"), false, None);
        let area = |p: &Piece| {
            let n = p.points.len();
            (0..n)
                .map(|i| {
                    let (a, b) = (p.points[i], p.points[(i + 1) % n]);
                    a.x * b.y - b.x * a.y
                })
                .sum::<f32>()
                .abs()
                / 2.0
        };
        let shown: f32 = wipe.iter().map(area).sum();
        assert!(shown > 1.0 && shown < CARD.w * CARD.h - 1.0, "{shown}");
    }

    #[test]
    fn option_values_round_trip_through_the_effect() {
        for def in EFFECTS {
            for option in def.options {
                for value in OptionValue::all(option) {
                    let mut e = AnimEffect::new(def.id);
                    value.set(&mut e);
                    assert!(value.is_set(&e), "{} {value:?}", def.id);
                    assert!(def.option_problems(&e).is_empty(), "{} {value:?}", def.id);
                }
            }
        }
    }

    #[test]
    fn entrance_arrows_point_away_from_the_side() {
        let from_left = OptionValue::From(Direction::Left);
        assert_eq!(from_left.glyph(false), Glyph::Turned(Icon::ArrowRight, 0.0));
        let Glyph::Turned(_, exit) = from_left.glyph(true) else {
            panic!("a turned arrow");
        };
        assert!((exit.abs() - 180.0).abs() < 0.01);
        let Glyph::Turned(_, corner) = OptionValue::From(Direction::TopLeft).glyph(false) else {
            panic!("a turned arrow");
        };
        assert!((corner - 45.0).abs() < 0.01, "down-right");
    }

    #[test]
    fn the_hover_loop_holds_at_rest() {
        assert!(hover_shown(0.0).abs() < 1e-6);
        assert!((hover_shown(0.5) - 0.5).abs() < 1e-6);
        assert!((hover_shown(1.2) - 1.0).abs() < 1e-6);
        assert!(hover_shown(1.45) < 0.1);
    }
}
