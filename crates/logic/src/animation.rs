//! Animation and transition sampling math (L2).
//!
//! Pure functions for easing, entrance/exit animation sampling, motion path polyline
//! sampling, and slide transition sampling. Used by the player runtime, editor preview,
//! and headless renderers.

use coursecraft_course::{
    Animation, Direction, Easing, MotionPath, Point, Rect, Size, Transition, TransitionEffect,
};

/// Evaluate an easing curve on normalized time `t` in `[0, 1]`.
///
/// Clamps input to `[0.0, 1.0]`. Returns `0.0` for non-finite values.
#[must_use]
pub fn ease(easing: Easing, t: f32) -> f32 {
    if !t.is_finite() {
        return 0.0;
    }
    let t = t.clamp(0.0, 1.0);
    match easing {
        Easing::Linear => t,
        Easing::EaseIn => t * t * t,
        Easing::EaseOut => {
            let inv = 1.0 - t;
            1.0 - inv * inv * inv
        }
        Easing::EaseInOut => {
            if t < 0.5 {
                4.0 * t * t * t
            } else {
                let inv = 1.0 - t;
                1.0 - 4.0 * inv * inv * inv
            }
        }
    }
}

/// Clip rect as fractions `[0, 1]` of the object's frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipFrac {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

/// Sampled visual transform and clipping of an object at a point in time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObjectSample {
    pub offset: (f32, f32),
    pub scale: f32,
    pub opacity: f32,
    pub clip: Option<ClipFrac>,
}

impl ObjectSample {
    pub const IDENTITY: Self = Self {
        offset: (0.0, 0.0),
        scale: 1.0,
        opacity: 1.0,
        clip: None,
    };

    /// Repair non-finite numbers (to identity) and clamp opacity to `[0, 1]` and scale to
    /// `>= 0`, so a sample function can never hand the renderer hostile values.
    #[must_use]
    pub fn sanitized(self) -> Self {
        let fin = |v: f32, d: f32| if v.is_finite() { v } else { d };
        Self {
            offset: (fin(self.offset.0, 0.0), fin(self.offset.1, 0.0)),
            scale: fin(self.scale, 1.0).max(0.0),
            opacity: fin(self.opacity, 1.0).clamp(0.0, 1.0),
            clip: self.clip.map(|c| ClipFrac {
                min_x: fin(c.min_x, 0.0).clamp(0.0, 1.0),
                min_y: fin(c.min_y, 0.0).clamp(0.0, 1.0),
                max_x: fin(c.max_x, 1.0).clamp(0.0, 1.0),
                max_y: fin(c.max_y, 1.0).clamp(0.0, 1.0),
            }),
        }
    }
}

/// Whether the animation is entering or exiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationPhase {
    Entering,
    Exiting,
}

/// Sample an entrance or exit animation at progress `progress` in `[0, 1]`.
///
/// For `Entering`, progress 0 is off-screen/hidden, 1 is fully entered.
/// For `Exiting`, progress 0 is resting/visible, 1 is fully exited.
/// The effect is looked up in the registry ([`crate::effects`]); an unknown id plays as
/// [`coursecraft_course::FALLBACK_EFFECT`].
#[must_use]
pub fn sample_animation(
    anim: &Animation,
    phase: AnimationPhase,
    progress: f32,
    frame: Rect,
    slide_size: Size,
) -> ObjectSample {
    let ctx = crate::effects::EffectCtx {
        phase,
        eased: ease(anim.easing, progress),
        frame,
        slide_size,
    };
    crate::effects::sample_effect(&anim.effect, &ctx)
}

/// Sample an active motion path at `elapsed_ms`, returning the offset relative to authored frame.
#[must_use]
pub fn sample_motion_path(
    path: &MotionPath,
    elapsed_ms: u32,
    from: (f32, f32),
    frame: Rect,
) -> (f32, f32) {
    if path.points.is_empty() {
        return from;
    }
    if let [pt] = path.points.as_slice() {
        let pt = *pt;
        return if path.relative {
            (from.0 + pt.x, from.1 + pt.y)
        } else {
            (pt.x - frame.x, pt.y - frame.y)
        };
    }

    let mut seg_lens = Vec::with_capacity(path.points.len().saturating_sub(1));
    let mut total_len = 0.0;
    for w in path.points.windows(2) {
        let dx = w[1].x - w[0].x;
        let dy = w[1].y - w[0].y;
        let len = (dx * dx + dy * dy).sqrt();
        seg_lens.push(len);
        total_len += len;
    }

    if total_len == 0.0 || !total_len.is_finite() {
        let end = path.points.last().copied().unwrap_or_default();
        return if path.relative {
            (from.0 + end.x, from.1 + end.y)
        } else {
            (end.x - frame.x, end.y - frame.y)
        };
    }

    let t = if path.duration_ms == 0 {
        1.0
    } else {
        (elapsed_ms as f32 / path.duration_ms as f32).clamp(0.0, 1.0)
    };
    let eased_t = ease(path.easing, t);
    let target_dist = (eased_t * total_len).clamp(0.0, total_len);

    let mut accum = 0.0;
    let mut pt = path.points.last().copied().unwrap_or_default();

    for (i, &seg_len) in seg_lens.iter().enumerate() {
        if target_dist <= accum + seg_len || i == seg_lens.len().saturating_sub(1) {
            let frac = if seg_len > 0.0 {
                ((target_dist - accum) / seg_len).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let p0 = path.points.get(i).copied().unwrap_or_default();
            let p1 = path.points.get(i + 1).copied().unwrap_or_default();
            pt = Point {
                x: p0.x + frac * (p1.x - p0.x),
                y: p0.y + frac * (p1.y - p0.y),
            };
            break;
        }
        accum += seg_len;
    }

    if path.relative {
        (from.0 + pt.x, from.1 + pt.y)
    } else {
        (pt.x - frame.x, pt.y - frame.y)
    }
}

/// Result of sampling a slide transition between previous and next slide.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransitionSample {
    pub prev_offset: (f32, f32),
    pub prev_alpha: f32,
    pub next_offset: (f32, f32),
    pub next_alpha: f32,
}

/// Sample a slide transition at progress `progress` in `[0, 1]`.
#[must_use]
pub fn sample_transition(
    transition: &Transition,
    progress: f32,
    slide_size: Size,
) -> TransitionSample {
    let p = progress.clamp(0.0, 1.0);
    let w = slide_size.w as f32;
    let h = slide_size.h as f32;

    match transition.effect {
        TransitionEffect::Fade => TransitionSample {
            prev_offset: (0.0, 0.0),
            prev_alpha: 1.0 - p,
            next_offset: (0.0, 0.0),
            next_alpha: p,
        },
        TransitionEffect::Push { from } => {
            let (next_offset, prev_offset) = match from {
                Direction::Left => ((-(1.0 - p) * w, 0.0), (p * w, 0.0)),
                Direction::Right => (((1.0 - p) * w, 0.0), (-p * w, 0.0)),
                Direction::Top => ((0.0, -(1.0 - p) * h), (0.0, p * h)),
                Direction::Bottom => ((0.0, (1.0 - p) * h), (0.0, -p * h)),
            };
            TransitionSample {
                prev_offset,
                prev_alpha: 1.0,
                next_offset,
                next_alpha: 1.0,
            }
        }
        TransitionEffect::Cover { from } => {
            let next_offset = match from {
                Direction::Left => (-(1.0 - p) * w, 0.0),
                Direction::Right => ((1.0 - p) * w, 0.0),
                Direction::Top => (0.0, -(1.0 - p) * h),
                Direction::Bottom => (0.0, (1.0 - p) * h),
            };
            TransitionSample {
                prev_offset: (0.0, 0.0),
                prev_alpha: 1.0,
                next_offset,
                next_alpha: 1.0,
            }
        }
    }
}
