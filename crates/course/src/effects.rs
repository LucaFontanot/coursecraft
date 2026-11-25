//! The animation effect registry: one entry per entrance/exit effect.
//!
//! This table is the single source of truth for which effects exist, which options each one
//! takes, its defaults, its category and its English labels. The runtime
//! (`coursecraft-logic`), the validator, the editor's galleries and panels, and the
//! `animation.effects` command all read it; `coursecraft-logic` has a test that every entry here
//! has a sample function and the other way round. Adding an effect means adding one entry here
//! and one sample function there.
//!
//! Plan: `docs/plans/m46-animation-system.md`.

use serde::Serialize;

use crate::model::{AnimEffect, Direction};

/// The effect the runtime plays for an id this build does not know (a file from a newer
/// version). The validator warns about it.
pub const FALLBACK_EFFECT: &str = "fade";

/// Gallery group an effect is listed under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectCategory {
    Basic,
    Subtle,
    Moderate,
    Exciting,
}

impl EffectCategory {
    /// Every category, in gallery order.
    pub const ALL: [Self; 4] = [Self::Basic, Self::Subtle, Self::Moderate, Self::Exciting];

    /// English label (the editor passes it through `tl!`).
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Basic => "Basic",
            Self::Subtle => "Subtle",
            Self::Moderate => "Moderate",
            Self::Exciting => "Exciting",
        }
    }
}

/// One option an effect accepts, with its default and allowed values.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(tag = "option", rename_all = "snake_case")]
pub enum EffectOptionDef {
    /// `from`: the side the object enters from (or leaves toward, for exits).
    Direction {
        default: Direction,
        allowed: &'static [Direction],
    },
}

impl EffectOptionDef {
    /// The JSON field this option is stored in.
    #[must_use]
    pub const fn field(&self) -> &'static str {
        match self {
            Self::Direction { .. } => "from",
        }
    }
}

/// One entrance/exit effect.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct EffectDef {
    /// Stored id (`"fly_in"`), the `type` of an [`AnimEffect`].
    pub id: &'static str,
    pub category: EffectCategory,
    /// English name as an entrance ("Fly in").
    pub entrance_label: &'static str,
    /// English name as an exit ("Fly out").
    pub exit_label: &'static str,
    /// Options the effect accepts. Fields not listed here are ignored by the runtime and
    /// reported by the validator.
    pub options: &'static [EffectOptionDef],
    /// Duration the editor uses when the effect is first applied.
    pub default_duration_ms: u32,
}

impl EffectDef {
    /// The label for `exit` (true) or entrance (false).
    #[must_use]
    pub const fn label(&self, exit: bool) -> &'static str {
        if exit {
            self.exit_label
        } else {
            self.entrance_label
        }
    }

    /// The direction option, if the effect has one.
    #[must_use]
    #[allow(clippy::unnecessary_find_map)] // one option kind today; more arrive with effect pack 1
    pub fn direction(&self) -> Option<(Direction, &'static [Direction])> {
        self.options.iter().find_map(|o| match *o {
            EffectOptionDef::Direction { default, allowed } => Some((default, allowed)),
        })
    }

    /// Whether the effect takes the option stored in `field`.
    #[must_use]
    pub fn takes(&self, field: &str) -> bool {
        self.options.iter().any(|o| o.field() == field)
    }
}

const SIDES: &[Direction] = &[
    Direction::Left,
    Direction::Right,
    Direction::Top,
    Direction::Bottom,
];

const FROM_SIDE: EffectOptionDef = EffectOptionDef::Direction {
    default: Direction::Bottom,
    allowed: SIDES,
};

/// Every effect, in gallery order.
pub static EFFECTS: &[EffectDef] = &[
    EffectDef {
        id: "fade",
        category: EffectCategory::Basic,
        entrance_label: "Fade in",
        exit_label: "Fade out",
        options: &[],
        default_duration_ms: 500,
    },
    EffectDef {
        id: "fly_in",
        category: EffectCategory::Basic,
        entrance_label: "Fly in",
        exit_label: "Fly out",
        options: &[FROM_SIDE],
        default_duration_ms: 500,
    },
    EffectDef {
        id: "zoom",
        category: EffectCategory::Basic,
        entrance_label: "Zoom in",
        exit_label: "Zoom out",
        options: &[],
        default_duration_ms: 500,
    },
    EffectDef {
        id: "wipe",
        category: EffectCategory::Basic,
        entrance_label: "Wipe in",
        exit_label: "Wipe out",
        options: &[FROM_SIDE],
        default_duration_ms: 500,
    },
    EffectDef {
        id: "float",
        category: EffectCategory::Subtle,
        entrance_label: "Float in",
        exit_label: "Float out",
        options: &[FROM_SIDE],
        default_duration_ms: 500,
    },
];

/// The registry entry for `id`, if this build knows it.
#[must_use]
pub fn effect_def(id: &str) -> Option<&'static EffectDef> {
    EFFECTS.iter().find(|d| d.id == id)
}

impl AnimEffect {
    /// The effect `id` with every option at its default (direction written out, so the stored
    /// file says what plays).
    #[must_use]
    pub fn new(id: &str) -> Self {
        let from = effect_def(id)
            .and_then(EffectDef::direction)
            .map(|(d, _)| d);
        Self {
            id: id.to_owned(),
            from,
        }
    }

    /// The effect `id` coming from (or leaving toward) `from`.
    #[must_use]
    pub fn with_from(id: &str, from: Direction) -> Self {
        Self {
            id: id.to_owned(),
            from: Some(from),
        }
    }

    /// The registry entry, or `None` for an unknown id.
    #[must_use]
    pub fn def(&self) -> Option<&'static EffectDef> {
        effect_def(&self.id)
    }

    /// The direction to play: the stored one when the effect takes a direction and allows it,
    /// else the effect's default; `None` when the effect has no direction.
    #[must_use]
    pub fn direction(&self) -> Option<Direction> {
        let (default, allowed) = self.def()?.direction()?;
        Some(self.from.filter(|d| allowed.contains(d)).unwrap_or(default))
    }

    /// Change the effect, keeping the direction when the new effect takes one.
    #[must_use]
    pub fn switched_to(&self, id: &str) -> Self {
        let mut next = Self::new(id);
        if let (Some(old), Some((_, allowed))) =
            (self.from, next.def().and_then(EffectDef::direction))
            && allowed.contains(&old)
        {
            next.from = Some(old);
        }
        next
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Animation, Course, Severity};

    fn anim(json: &str) -> Result<Animation, serde_json::Error> {
        serde_json::from_str(json)
    }

    #[test]
    fn stored_shape_is_unchanged() {
        for json in [
            r#"{"effect":{"type":"fade"},"duration_ms":300,"easing":"ease_out"}"#,
            r#"{"effect":{"type":"fly_in","from":"left"},"duration_ms":300,"easing":"ease_out"}"#,
            r#"{"effect":{"type":"wipe","from":"top"},"duration_ms":500,"easing":"linear"}"#,
        ] {
            let a = anim(json).expect("parses");
            assert_eq!(serde_json::to_string(&a).expect("writes"), json);
        }
    }

    #[test]
    fn missing_direction_plays_the_default() {
        let a = anim(r#"{"effect":{"type":"float"}}"#).expect("parses");
        assert_eq!(a.effect.direction(), Some(Direction::Bottom));
        assert_eq!(AnimEffect::new("zoom").direction(), None);
    }

    #[test]
    fn unknown_field_is_refused_unknown_id_is_not() {
        assert!(anim(r#"{"effect":{"type":"fade","form":"left"}}"#).is_err());
        let a = anim(r#"{"effect":{"type":"from_the_future"}}"#).expect("parses");
        assert!(a.effect.def().is_none());
    }

    #[test]
    fn switching_keeps_an_allowed_direction() {
        let fly = AnimEffect::with_from("fly_in", Direction::Left);
        assert_eq!(fly.switched_to("wipe").from, Some(Direction::Left));
        assert_eq!(fly.switched_to("fade").from, None);
        assert_eq!(
            AnimEffect::new("fade").switched_to("float").from,
            Some(Direction::Bottom)
        );
    }

    #[test]
    fn registry_ids_are_unique_and_defaults_allowed() {
        for (i, d) in EFFECTS.iter().enumerate() {
            assert!(
                EFFECTS[..i].iter().all(|o| o.id != d.id),
                "duplicate `{}`",
                d.id
            );
            assert!((1..=60_000).contains(&d.default_duration_ms));
            if let Some((default, allowed)) = d.direction() {
                assert!(allowed.contains(&default), "`{}` default", d.id);
            }
        }
        assert!(effect_def(FALLBACK_EFFECT).is_some());
    }

    #[test]
    fn validator_warns_about_unknown_effects_and_options() {
        let mut c = Course::from_json(include_str!("../../../examples/hello-course/course.json"))
            .expect("hello course");
        let obj = c
            .scenes
            .iter_mut()
            .flat_map(|s| s.slides.iter_mut())
            .flat_map(|s| s.layers.iter_mut())
            .flat_map(|l| l.objects.iter_mut())
            .next()
            .expect("an object");
        obj.entrance = Some(Animation {
            effect: AnimEffect {
                id: "from_the_future".to_owned(),
                from: None,
            },
            duration_ms: 500,
            easing: crate::Easing::EaseOut,
        });
        obj.exit = Some(Animation {
            effect: AnimEffect::with_from("zoom", Direction::Left),
            duration_ms: 500,
            easing: crate::Easing::EaseOut,
        });
        let issues = c.validate();
        let warn = |needle: &str| {
            issues.iter().any(|i| {
                i.severity == Severity::Warning
                    && i.message.contains(needle)
                    && i.path.contains("effect")
            })
        };
        assert!(warn("unknown animation effect"), "{issues:?}");
        assert!(warn("takes no direction"), "{issues:?}");
        assert!(
            issues.iter().all(|i| i.severity != Severity::Error),
            "{issues:?}"
        );
    }

    #[test]
    fn examples_round_trip_their_animations() {
        for text in [
            include_str!("../../../examples/reference-course/course.json"),
            include_str!("../../../examples/showcase/course.json"),
        ] {
            let c = Course::from_json(text).expect("example parses");
            let again = Course::from_json(&c.to_json().expect("writes")).expect("reparses");
            assert_eq!(c, again);
        }
    }
}
