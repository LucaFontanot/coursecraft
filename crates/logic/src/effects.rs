//! Sample functions for the animation effect registry (L2).
//!
//! [`coursecraft_course::EFFECTS`] lists the effects and their options; this module holds one
//! sample function per registry id in [`IMPLS`]. A test checks the two tables match, so an
//! effect can't be listed without a runtime or the other way round.
//!
//! A sample function gets the eased progress and the phase. It returns how the object looks
//! at that moment as an [`ObjectSample`]. Exits reuse the entrance function: `Exiting` runs the
//! entrance backwards, so each effect is written once.

use coursecraft_course::{AnimEffect, Direction, FALLBACK_EFFECT, Rect, Size};

use crate::animation::{AnimationPhase, ClipFrac, ObjectSample};

/// What a sample function gets besides the effect's options.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectCtx {
    pub phase: AnimationPhase,
    /// Eased progress in `[0, 1]`: 0 is the start of the animation, 1 its end.
    pub eased: f32,
    /// The object's frame on the slide.
    pub frame: Rect,
    pub slide_size: Size,
}

impl EffectCtx {
    /// How far the object is "in": 0 hidden/away, 1 at rest. Entering runs 0→1, exiting 1→0.
    #[must_use]
    pub fn shown(&self) -> f32 {
        match self.phase {
            AnimationPhase::Entering => self.eased,
            AnimationPhase::Exiting => 1.0 - self.eased,
        }
    }
}

/// One effect's sample function.
pub type SampleFn = fn(&AnimEffect, &EffectCtx) -> ObjectSample;

/// One registry id and its sample function.
pub struct EffectImpl {
    pub id: &'static str,
    pub sample: SampleFn,
}

/// Every effect the runtime can play. Must list exactly the ids of
/// [`coursecraft_course::EFFECTS`].
pub static IMPLS: &[EffectImpl] = &[
    EffectImpl {
        id: "fade",
        sample: fade,
    },
    EffectImpl {
        id: "fly_in",
        sample: fly_in,
    },
    EffectImpl {
        id: "zoom",
        sample: zoom,
    },
    EffectImpl {
        id: "wipe",
        sample: wipe,
    },
    EffectImpl {
        id: "float",
        sample: float,
    },
];

fn sampler(id: &str) -> Option<SampleFn> {
    IMPLS.iter().find(|i| i.id == id).map(|i| i.sample)
}

/// Sample `effect`; an unknown id plays as [`FALLBACK_EFFECT`].
#[must_use]
pub fn sample_effect(effect: &AnimEffect, ctx: &EffectCtx) -> ObjectSample {
    let sample = sampler(&effect.id)
        .or_else(|| sampler(FALLBACK_EFFECT))
        .unwrap_or(fade);
    sample(effect, ctx).sanitized()
}

/// The direction the effect plays from: the stored one if the effect allows it, else its
/// default, else bottom.
fn from(effect: &AnimEffect) -> Direction {
    effect.direction().unwrap_or(Direction::Bottom)
}

fn fade(_: &AnimEffect, ctx: &EffectCtx) -> ObjectSample {
    ObjectSample {
        opacity: ctx.shown(),
        ..ObjectSample::IDENTITY
    }
}

fn fly_in(effect: &AnimEffect, ctx: &EffectCtx) -> ObjectSample {
    let f = ctx.frame;
    let full = match from(effect) {
        Direction::Left => (-(f.x + f.w), 0.0),
        Direction::Right => (ctx.slide_size.w as f32 - f.x, 0.0),
        Direction::Top => (0.0, -(f.y + f.h)),
        Direction::Bottom => (0.0, ctx.slide_size.h as f32 - f.y),
    };
    let away = 1.0 - ctx.shown();
    ObjectSample {
        offset: (full.0 * away, full.1 * away),
        ..ObjectSample::IDENTITY
    }
}

fn zoom(_: &AnimEffect, ctx: &EffectCtx) -> ObjectSample {
    let s = ctx.shown();
    ObjectSample {
        scale: ObjectSample::uniform(s),
        opacity: s,
        ..ObjectSample::IDENTITY
    }
}

fn wipe(effect: &AnimEffect, ctx: &EffectCtx) -> ObjectSample {
    let s = ctx.shown();
    let clip = match from(effect) {
        Direction::Left => ClipFrac {
            min_x: 0.0,
            min_y: 0.0,
            max_x: s,
            max_y: 1.0,
        },
        Direction::Right => ClipFrac {
            min_x: 1.0 - s,
            min_y: 0.0,
            max_x: 1.0,
            max_y: 1.0,
        },
        Direction::Top => ClipFrac {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 1.0,
            max_y: s,
        },
        Direction::Bottom => ClipFrac {
            min_x: 0.0,
            min_y: 1.0 - s,
            max_x: 1.0,
            max_y: 1.0,
        },
    };
    let full = clip.min_x <= 0.0 && clip.min_y <= 0.0 && clip.max_x >= 1.0 && clip.max_y >= 1.0;
    ObjectSample {
        clip: if full { None } else { Some(clip) },
        ..ObjectSample::IDENTITY
    }
}

/// How far Float travels, in slide pixels.
const FLOAT_DISTANCE: f32 = 40.0;

fn float(effect: &AnimEffect, ctx: &EffectCtx) -> ObjectSample {
    let d = FLOAT_DISTANCE;
    let dir = match from(effect) {
        Direction::Left => (-d, 0.0),
        Direction::Right => (d, 0.0),
        Direction::Top => (0.0, -d),
        Direction::Bottom => (0.0, d),
    };
    let s = ctx.shown();
    let away = 1.0 - s;
    ObjectSample {
        offset: (dir.0 * away, dir.1 * away),
        opacity: s,
        ..ObjectSample::IDENTITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coursecraft_course::EFFECTS;

    #[test]
    fn every_registry_effect_has_a_sampler_and_back() {
        for def in EFFECTS {
            assert!(sampler(def.id).is_some(), "no sampler for `{}`", def.id);
        }
        for imp in IMPLS {
            assert!(
                coursecraft_course::effect_def(imp.id).is_some(),
                "sampler `{}` is not in the registry",
                imp.id
            );
        }
        assert!(sampler(FALLBACK_EFFECT).is_some());
    }

    #[test]
    fn every_effect_rests_at_identity_and_stays_finite() {
        let ctx = |phase, eased| EffectCtx {
            phase,
            eased,
            frame: Rect {
                x: 100.0,
                y: 100.0,
                w: 200.0,
                h: 100.0,
            },
            slide_size: Size { w: 960, h: 540 },
        };
        for def in EFFECTS {
            let e = AnimEffect::new(def.id);
            let rest = sample_effect(&e, &ctx(AnimationPhase::Entering, 1.0));
            assert_eq!(rest, ObjectSample::IDENTITY, "`{}` entered", def.id);
            let rest = sample_effect(&e, &ctx(AnimationPhase::Exiting, 0.0));
            assert_eq!(rest, ObjectSample::IDENTITY, "`{}` before exit", def.id);
            for p in [f32::NAN, -1.0, 0.0, 0.3, 0.5, 2.0] {
                let s = sample_effect(&e, &ctx(AnimationPhase::Entering, p));
                assert!(s.offset.0.is_finite() && s.offset.1.is_finite());
                assert!(s.scale.0.is_finite() && s.scale.1.is_finite());
                assert!((0.0..=1.0).contains(&s.opacity));
            }
        }
    }

    #[test]
    fn unknown_effect_plays_as_fallback() {
        let ctx = EffectCtx {
            phase: AnimationPhase::Entering,
            eased: 0.25,
            frame: Rect {
                x: 0.0,
                y: 0.0,
                w: 10.0,
                h: 10.0,
            },
            slide_size: Size { w: 960, h: 540 },
        };
        let unknown = AnimEffect {
            id: "from_the_future".to_owned(),
            from: None,
        };
        assert_eq!(
            sample_effect(&unknown, &ctx),
            sample_effect(&AnimEffect::new(FALLBACK_EFFECT), &ctx)
        );
    }

    #[test]
    fn sanitizing_repairs_every_new_field() {
        use crate::animation::{ConvexPoly, MAX_MASK_POLYS, Mask, Tint};
        let s = ObjectSample {
            scale: (f32::NAN, 1e9),
            rotation_deg: f32::INFINITY,
            tint: Some(Tint {
                rgb: [1, 2, 3],
                amount: f32::NAN,
            }),
            mask: Some(Mask::new(vec![
                ConvexPoly(vec![(0.0, 0.0), (1.0, 0.0)]),
                ConvexPoly(vec![(f32::NAN, 0.0), (1.0, 0.0), (1.0, 1.0)]),
                ConvexPoly(vec![(0.0, 0.0), (0.5, 0.5), (1.0, 1.0)]),
                ConvexPoly(vec![(-9.0, 0.0), (9.0, 0.0), (9.0, 1.0), (-9.0, 1.0)]),
            ])),
            ..ObjectSample::IDENTITY
        }
        .sanitized();
        assert_eq!(s.scale, (1.0, 100.0));
        assert_eq!(s.rotation_deg, 0.0);
        assert_eq!(s.tint, None);
        let mask = s.mask.expect("mask kept");
        assert_eq!(mask.polys().len(), 1, "only the real polygon survives");
        assert!(
            mask.polys()[0]
                .0
                .iter()
                .all(|&(x, _)| (-1.0..=2.0).contains(&x))
        );

        let many = vec![ConvexPoly::rect(0.0, 0.0, 1.0, 1.0); MAX_MASK_POLYS + 1];
        let s = ObjectSample {
            mask: Some(Mask::new(many)),
            ..ObjectSample::IDENTITY
        }
        .sanitized();
        assert_eq!(s.mask, None, "too many polygons show the object whole");
    }
}
