//! The ribbon's galleries (docs/ui/ribbon.md › Design, Transitions, Animations):
//! a row of tiles in the tab, all of them under ▾, and the options beside them
//! (effect direction, duration, Apply To All). A pick is one command.

use coursecraft_course::{
    AnimEffect, Animation, Direction, EFFECTS, Easing, Object, Slide, Transition, TransitionEffect,
};
use coursecraft_render::Icon;
use egui::{Align2, Color32, CornerRadius, FontFamily, FontId, Rect, Response, Sense, Stroke, Ui};
use serde_json::json;

use super::spec::{EffectOp, Gallery, Item, Motion};
use crate::Editor;

/// One tile of a gallery.
pub struct Tile {
    pub label: &'static str,
    pub icon: Icon,
    /// A theme's background, accent and text colours, drawn instead of the icon.
    pub swatch: Option<[Color32; 3]>,
    pub selected: bool,
}

/// A built-in theme: its name and the colours `course.set_theme` sets.
struct ThemePreset {
    name: &'static str,
    accent: &'static str,
    background: &'static str,
    text: &'static str,
}

/// CourseCraft's own theme presets (colours chosen for contrast, not copied).
const THEMES: &[ThemePreset] = &[
    ThemePreset {
        name: "CourseCraft",
        accent: "#2F6FEB",
        background: "#FFFFFF",
        text: "#1F1F1F",
    },
    ThemePreset {
        name: "Lagoon",
        accent: "#0E8A8A",
        background: "#F4FBFB",
        text: "#123536",
    },
    ThemePreset {
        name: "Meadow",
        accent: "#3B8A3E",
        background: "#F7FAF2",
        text: "#1E2D1A",
    },
    ThemePreset {
        name: "Ember",
        accent: "#D2522B",
        background: "#FFF8F3",
        text: "#33201A",
    },
    ThemePreset {
        name: "Plum",
        accent: "#7A3E9D",
        background: "#FBF7FD",
        text: "#2A1A33",
    },
    ThemePreset {
        name: "Slate",
        accent: "#4C6A88",
        background: "#F3F5F7",
        text: "#1C2630",
    },
    ThemePreset {
        name: "Night",
        accent: "#F2B13A",
        background: "#1B1F27",
        text: "#F1F3F6",
    },
    ThemePreset {
        name: "Ink",
        accent: "#E04F6D",
        background: "#101820",
        text: "#EDEFF2",
    },
];

/// The transitions a slide can have, in gallery order.
const TRANSITIONS: &[(&str, Icon)] = &[
    ("None", Icon::XCircle),
    ("Fade", Icon::Drop),
    ("Push", Icon::ArrowRight),
    ("Cover", Icon::Stack),
];

/// The gallery icon of an entrance/exit effect (registry id).
fn effect_icon(id: &str) -> Icon {
    match id {
        "fade" => Icon::Drop,
        "fly_in" => Icon::ArrowRight,
        "zoom" => Icon::MagnifyingGlassPlus,
        "wipe" => Icon::TextAlignLeft,
        "float" => Icon::ArrowUp,
        _ => Icon::Sparkle,
    }
}

fn hex(color: &str) -> Color32 {
    coursecraft_render::parse_color(&coursecraft_course::Color(color.to_owned()))
        .unwrap_or(Color32::GRAY)
}

fn same_color(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

fn slide(editor: &Editor) -> Option<&Slide> {
    editor.session.course.slide(&editor.selection.slide)
}

/// The selected objects of the current slide.
fn objects(editor: &Editor) -> Vec<&Object> {
    let Some(slide) = slide(editor) else {
        return Vec::new();
    };
    editor
        .selection
        .ids()
        .iter()
        .filter_map(|id| slide.object(id).map(|(o, _)| o))
        .collect()
}

fn animation(object: &Object, motion: Motion) -> Option<&Animation> {
    match motion {
        Motion::Entrance => object.entrance.as_ref(),
        Motion::Exit => object.exit.as_ref(),
        Motion::Transition => None,
    }
}

fn transition_index(t: Option<&Transition>) -> usize {
    match t.map(|t| &t.effect) {
        None => 0,
        Some(TransitionEffect::Fade) => 1,
        Some(TransitionEffect::Push { .. }) => 2,
        Some(TransitionEffect::Cover { .. }) => 3,
    }
}

/// Tile index of an animation: 0 is None, then the registry in order. An unknown effect
/// selects no tile.
fn effect_index(a: Option<&Animation>) -> Option<usize> {
    match a {
        None => Some(0),
        Some(a) => EFFECTS
            .iter()
            .position(|d| d.id == a.effect.id)
            .map(|i| i + 1),
    }
}

/// The tiles of `gallery`, with the current one selected.
#[must_use]
pub fn tiles(editor: &Editor, gallery: Gallery) -> Vec<Tile> {
    match gallery {
        Gallery::Themes => {
            let theme = &editor.session.course.theme;
            THEMES
                .iter()
                .map(|t| Tile {
                    label: t.name,
                    icon: Icon::Palette,
                    swatch: Some([hex(t.background), hex(t.accent), hex(t.text)]),
                    selected: same_color(&theme.accent.0, t.accent)
                        && same_color(&theme.background.0, t.background)
                        && same_color(&theme.text.0, t.text),
                })
                .collect()
        }
        Gallery::Transitions => {
            let current = slide(editor).map(|s| transition_index(s.transition.as_ref()));
            TRANSITIONS
                .iter()
                .enumerate()
                .map(|(i, (label, icon))| Tile {
                    label,
                    icon: *icon,
                    swatch: None,
                    selected: current == Some(i),
                })
                .collect()
        }
        Gallery::Entrance | Gallery::Exit => {
            let motion = motion_of(gallery);
            let current = objects(editor)
                .first()
                .and_then(|o| effect_index(animation(o, motion)));
            let exit = motion == Motion::Exit;
            let none = Tile {
                label: "None",
                icon: Icon::XCircle,
                swatch: None,
                selected: current == Some(0),
            };
            std::iter::once(none)
                .chain(EFFECTS.iter().enumerate().map(|(i, d)| Tile {
                    label: d.label(exit),
                    icon: effect_icon(d.id),
                    swatch: None,
                    selected: current == Some(i + 1),
                }))
                .collect()
        }
    }
}

fn motion_of(gallery: Gallery) -> Motion {
    match gallery {
        Gallery::Exit => Motion::Exit,
        Gallery::Entrance => Motion::Entrance,
        Gallery::Themes | Gallery::Transitions => Motion::Transition,
    }
}

/// Whether `gallery` can be used now, with the reason when it cannot.
pub fn available(editor: &Editor, gallery: Gallery) -> Result<(), &'static str> {
    match gallery {
        Gallery::Themes => Ok(()),
        Gallery::Transitions => transition_available(editor),
        Gallery::Entrance | Gallery::Exit => animation_available(editor),
    }
}

fn transition_available(editor: &Editor) -> Result<(), &'static str> {
    if editor.ui.master_view {
        Err(super::MASTER)
    } else if slide(editor).is_none() {
        Err("Select a slide first")
    } else {
        Ok(())
    }
}

fn animation_available(editor: &Editor) -> Result<(), &'static str> {
    if editor.ui.editing_state.is_some() {
        Err(super::STATE)
    } else if objects(editor).is_empty() {
        Err("Select an object first")
    } else {
        Ok(())
    }
}

/// Pick tile `index` of `gallery`.
pub fn apply(editor: &mut Editor, gallery: Gallery, index: usize) {
    if available(editor, gallery).is_err() {
        return;
    }
    match gallery {
        Gallery::Themes => {
            if let Some(t) = THEMES.get(index) {
                editor.run(
                    "course.set_theme",
                    json!({ "accent": t.accent, "background": t.background, "text": t.text }),
                );
            }
        }
        Gallery::Transitions => {
            let Some(s) = slide(editor) else {
                return;
            };
            let old = s.transition.as_ref();
            let duration_ms = old.map_or(500, |t| t.duration_ms);
            let from = old
                .and_then(|t| transition_from(&t.effect))
                .unwrap_or(Direction::Right);
            let effect = match index {
                1 => Some(TransitionEffect::Fade),
                2 => Some(TransitionEffect::Push { from }),
                3 => Some(TransitionEffect::Cover { from }),
                _ => None,
            };
            let transition = effect.map(|effect| Transition {
                effect,
                duration_ms,
            });
            let id = s.id.clone();
            set_transition(editor, &id, transition.as_ref());
        }
        Gallery::Entrance | Gallery::Exit => {
            let motion = motion_of(gallery);
            let changes: Vec<(String, String, Option<Animation>)> = {
                let Some(s) = slide(editor) else {
                    return;
                };
                objects(editor)
                    .iter()
                    .map(|o| {
                        let old = animation(o, motion);
                        let def = index.checked_sub(1).and_then(|i| EFFECTS.get(i));
                        let effect = def.map(|d| {
                            old.map_or_else(
                                || AnimEffect::new(d.id),
                                |a| a.effect.switched_to(d.id),
                            )
                        });
                        let animation = effect.map(|effect| Animation {
                            effect,
                            duration_ms: old
                                .map_or(def.map_or(500, |d| d.default_duration_ms), |a| {
                                    a.duration_ms
                                }),
                            easing: old.map_or(Easing::EaseOut, |a| a.easing),
                        });
                        (s.id.clone(), o.id.clone(), animation)
                    })
                    .collect()
            };
            for (slide, object, animation) in changes {
                set_animation(editor, &slide, &object, motion, animation.as_ref());
            }
        }
    }
}

fn set_transition(editor: &mut Editor, slide: &str, transition: Option<&Transition>) {
    editor.run(
        "slide.set_transition",
        json!({ "slide": slide, "transition": transition }),
    );
}

fn set_animation(
    editor: &mut Editor,
    slide: &str,
    object: &str,
    motion: Motion,
    animation: Option<&Animation>,
) {
    let which = if motion == Motion::Exit {
        "exit"
    } else {
        "entrance"
    };
    editor.run(
        "object.set_animation",
        json!({ "slide": slide, "object": object, "which": which, "animation": animation }),
    );
}

fn transition_from(effect: &TransitionEffect) -> Option<Direction> {
    match effect {
        TransitionEffect::Push { from } | TransitionEffect::Cover { from } => Some(*from),
        TransitionEffect::Fade => None,
    }
}

fn effect_from(effect: &AnimEffect) -> Option<Direction> {
    effect.direction()
}

/// The current direction of `motion`'s effect, when it has one.
fn current_from(editor: &Editor, motion: Motion) -> Option<Direction> {
    match motion {
        Motion::Transition => slide(editor)
            .and_then(|s| s.transition.as_ref())
            .and_then(|t| transition_from(&t.effect)),
        Motion::Entrance | Motion::Exit => objects(editor)
            .first()
            .and_then(|o| animation(o, motion))
            .and_then(|a| effect_from(&a.effect)),
    }
}

/// Whether `motion`'s current effect can come from `dir` (transitions take every side).
fn allows_from(editor: &Editor, motion: Motion, dir: Direction) -> bool {
    match motion {
        Motion::Transition => true,
        Motion::Entrance | Motion::Exit => objects(editor)
            .first()
            .and_then(|o| animation(o, motion))
            .and_then(|a| a.effect.def())
            .and_then(coursecraft_course::EffectDef::direction)
            .is_some_and(|(_, allowed)| allowed.contains(&dir)),
    }
}

/// The current duration of `motion`'s effect, in ms.
fn current_duration(editor: &Editor, motion: Motion) -> Option<u32> {
    match motion {
        Motion::Transition => slide(editor)
            .and_then(|s| s.transition.as_ref())
            .map(|t| t.duration_ms),
        Motion::Entrance | Motion::Exit => objects(editor)
            .first()
            .and_then(|o| animation(o, motion))
            .map(|a| a.duration_ms),
    }
}

/// Whether an effect option can be used now.
pub fn effect_available(editor: &Editor, op: EffectOp) -> Result<(), &'static str> {
    let base = |motion: Motion| match motion {
        Motion::Transition => transition_available(editor),
        Motion::Entrance | Motion::Exit => animation_available(editor),
    };
    match op {
        EffectOp::Options(motion) => {
            base(motion)?;
            if current_from(editor, motion).is_some() {
                Ok(())
            } else {
                Err("Pick an effect that comes from a side first")
            }
        }
        EffectOp::From(motion, dir) => {
            base(motion)?;
            if current_from(editor, motion).is_none() {
                return Err("Pick an effect that comes from a side first");
            }
            if allows_from(editor, motion, dir) {
                Ok(())
            } else {
                Err("This effect can't come from this side")
            }
        }
        EffectOp::Duration(motion) => {
            base(motion)?;
            if current_duration(editor, motion).is_some() {
                Ok(())
            } else {
                Err("Pick an effect first")
            }
        }
        EffectOp::TransitionToAll => transition_available(editor),
    }
}

/// Whether an effect option shows as on.
#[must_use]
pub fn effect_selected(editor: &Editor, op: EffectOp) -> bool {
    match op {
        EffectOp::From(motion, dir) => current_from(editor, motion) == Some(dir),
        _ => false,
    }
}

/// Run an effect option (the duration field commits its own value).
pub fn run_effect(editor: &mut Editor, op: EffectOp) {
    if effect_available(editor, op).is_err() {
        return;
    }
    match op {
        EffectOp::From(motion, dir) => set_from(editor, motion, dir),
        EffectOp::TransitionToAll => {
            let transition = slide(editor).and_then(|s| s.transition.clone());
            editor.run(
                "slide.set_transition_all",
                json!({ "transition": transition }),
            );
        }
        EffectOp::Options(_) | EffectOp::Duration(_) => {}
    }
}

fn set_from(editor: &mut Editor, motion: Motion, dir: Direction) {
    let with_from = |effect: &AnimEffect| match effect.def().and_then(|d| d.direction()) {
        Some((_, allowed)) if allowed.contains(&dir) => AnimEffect::with_from(&effect.id, dir),
        _ => effect.clone(),
    };
    update(
        editor,
        motion,
        |t| {
            t.effect = match t.effect {
                TransitionEffect::Push { .. } => TransitionEffect::Push { from: dir },
                TransitionEffect::Cover { .. } => TransitionEffect::Cover { from: dir },
                TransitionEffect::Fade => TransitionEffect::Fade,
            };
        },
        |a| a.effect = with_from(&a.effect),
    );
}

fn set_duration(editor: &mut Editor, motion: Motion, ms: u32) {
    update(
        editor,
        motion,
        |t| t.duration_ms = ms,
        |a| a.duration_ms = ms,
    );
}

/// Change the current slide's transition, or every selected object's
/// animation of `motion`, where it has one.
fn update(
    editor: &mut Editor,
    motion: Motion,
    transition: impl Fn(&mut Transition),
    anim: impl Fn(&mut Animation),
) {
    let Some(s) = slide(editor) else {
        return;
    };
    let id = s.id.clone();
    match motion {
        Motion::Transition => {
            if let Some(mut t) = s.transition.clone() {
                transition(&mut t);
                set_transition(editor, &id, Some(&t));
            }
        }
        Motion::Entrance | Motion::Exit => {
            let changes: Vec<(String, Animation)> = objects(editor)
                .iter()
                .filter_map(|o| {
                    let mut a = animation(o, motion)?.clone();
                    anim(&mut a);
                    Some((o.id.clone(), a))
                })
                .collect();
            for (object, a) in changes {
                set_animation(editor, &id, &object, motion, Some(&a));
            }
        }
    }
}

/// The Duration field of `motion`, in seconds; a committed edit is one command.
pub fn duration_field(
    editor: &mut Editor,
    ui: &mut Ui,
    item: &'static Item,
    motion: Motion,
    width: f32,
    enabled: bool,
) -> Response {
    let current = current_duration(editor, motion).unwrap_or(500);
    let inner = ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        let p = coursecraft_render::palette(ui.ctx());
        let color = if enabled { p.text } else { p.text_disabled };
        ui.label(egui::RichText::new(tl!(item.label)).size(11.0).color(color));
        ui.add_enabled_ui(enabled, |ui| {
            ui.set_width(width);
            crate::grid_panel::committed_number(
                ui,
                egui::Id::unique(("ribbon_duration", item.id)),
                current as f32 / 1000.0,
                0.1..=60.0,
                " s",
            )
        })
        .inner
    });
    if let Some(seconds) = inner.inner {
        let ms = (seconds * 1000.0).round().clamp(1.0, 60_000.0) as u32;
        if ms != current {
            set_duration(editor, motion, ms);
        }
    }
    inner.response
}

/// Size of one tile.
const TILE: egui::Vec2 = egui::vec2(58.0, 62.0);

/// Draw a tile; returns its response.
fn tile(ui: &mut Ui, t: &Tile, enabled: bool) -> Response {
    let p = coursecraft_render::palette(ui.ctx());
    let label = tl!(t.label);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(TILE, sense);
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, enabled, t.selected, label));
    let radius = CornerRadius::same(coursecraft_render::RADIUS_SM as u8);
    if t.selected {
        ui.painter().rect_filled(rect, radius, p.accent_subtle);
        ui.painter().rect_stroke(
            rect,
            radius,
            Stroke::new(1.0, p.accent),
            egui::StrokeKind::Inside,
        );
    } else if enabled && resp.hovered() {
        ui.painter().rect_filled(rect, radius, p.hover);
    }
    let ink = if enabled { p.text } else { p.text_disabled };
    let art = Rect::from_center_size(
        egui::pos2(rect.center().x, rect.min.y + 22.0),
        egui::vec2(40.0, 30.0),
    );
    match t.swatch {
        Some([background, accent, text]) => {
            ui.painter()
                .rect_filled(art, CornerRadius::same(3), background);
            ui.painter().rect_stroke(
                art,
                CornerRadius::same(3),
                Stroke::new(1.0, p.border),
                egui::StrokeKind::Inside,
            );
            ui.painter().rect_filled(
                Rect::from_min_size(art.min, egui::vec2(art.width(), 6.0)),
                CornerRadius::same(2),
                accent,
            );
            ui.painter().text(
                art.center() + egui::vec2(0.0, 3.0),
                Align2::CENTER_CENTER,
                "Aa",
                FontId::proportional(12.0),
                text,
            );
        }
        None => {
            ui.painter().text(
                art.center(),
                Align2::CENTER_CENTER,
                t.icon.codepoint(),
                FontId::new(22.0, FontFamily::Name("phosphor".into())),
                ink,
            );
        }
    }
    ui.painter().text(
        egui::pos2(rect.center().x, rect.max.y - 9.0),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(10.5),
        ink,
    );
    resp.on_hover_text(label)
}

/// Draw `gallery` in the ribbon: its first `shown` tiles and a ▾ for all.
pub fn show(
    editor: &mut Editor,
    ui: &mut Ui,
    item: &'static Item,
    gallery: Gallery,
    shown: u8,
    enabled: bool,
) -> Response {
    let tiles = tiles(editor, gallery);
    let mut picked = None;
    let inner = ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        // Keep the selected tile in view.
        let shown = usize::from(shown.max(1)).min(tiles.len());
        let start = tiles
            .iter()
            .position(|t| t.selected)
            .map_or(0, |i| i.saturating_sub(shown.saturating_sub(1)));
        for (i, t) in tiles.iter().enumerate().skip(start).take(shown) {
            if tile(ui, t, enabled).clicked() {
                picked = Some(i);
            }
        }
        let (more_rect, more) = ui.allocate_exact_size(
            egui::vec2(16.0, TILE.y),
            if enabled {
                Sense::click()
            } else {
                Sense::hover()
            },
        );
        let p = coursecraft_render::palette(ui.ctx());
        if enabled && more.hovered() {
            ui.painter().rect_filled(more_rect, 3.0, p.hover);
        }
        super::widgets::chevron(
            ui.painter(),
            more_rect.center(),
            4.0,
            if enabled { p.text } else { p.text_disabled },
        );
        let name = craft_i18n::fmt(tl!("All {gallery}"), &[("gallery", tl!(item.label))]);
        let more = more.on_hover_text(&name);
        more.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, &name));
        egui::Popup::menu(&more).show(|ui| {
            ui.set_max_width(TILE.x * 5.0 + 12.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
                for (i, t) in tiles.iter().enumerate() {
                    if tile(ui, t, enabled).clicked() {
                        picked = Some(i);
                        ui.close();
                    }
                }
            });
        });
    });
    if let Some(index) = picked {
        apply(editor, gallery, index);
    }
    inner.response
}
