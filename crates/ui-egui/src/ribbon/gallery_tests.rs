//! The ribbon's galleries drawn in the editor (ribbon.md › Design,
//! Transitions, Animations).

use crate::test_harness::TestHarness;
use coursecraft_course::{Course, Direction, TransitionEffect};

fn frames(h: &mut TestHarness, n: usize) {
    for _ in 0..n {
        h.idle_frame();
    }
}

fn harness() -> TestHarness {
    let json = serde_json::json!({
        "schema_version": 1,
        "id": "gal",
        "title": "Galleries",
        "slide_size": { "w": 1000, "h": 600 },
        "scenes": [{ "id": "sc1", "title": "Scene 1", "slides": [
            { "id": "sl1", "title": "One", "duration_ms": 5000, "objects": [{
                "id": "sh1",
                "kind": { "type": "shape", "shape": "rectangle", "fill": "#112233" },
                "frame": { "x": 100, "y": 100, "w": 200, "h": 120 }
            }] },
            { "id": "sl2", "title": "Two", "duration_ms": 5000, "objects": [] }
        ] }]
    });
    let course = Course::from_json(&json.to_string()).expect("valid course");
    let mut h = TestHarness::new_with_size(course, (2400.0, 900.0));
    h.editor.ui.ribbon.visible = true;
    h.editor.selection = crate::Selection::new("sl1");
    frames(&mut h, 2);
    h
}

fn open_tab(h: &mut TestHarness, id: &'static str) {
    h.editor.ui.ribbon.tab = id;
    frames(h, 3);
}

#[test]
fn a_theme_tile_sets_the_course_colours() {
    let mut h = harness();
    open_tab(&mut h, "design");
    let tile = h.find_text("Lagoon").expect("Lagoon tile");
    h.click_at(tile);
    frames(&mut h, 2);
    assert_eq!(h.editor.session.course.theme.accent.0, "#0E8A8A");
    h.editor.session.undo().expect("undo");
    assert_eq!(h.editor.session.course.theme.accent.0, "#2F6FEB");
}

#[test]
fn transitions_pick_a_direction_a_duration_and_apply_to_all() {
    let mut h = harness();
    open_tab(&mut h, "transitions");
    let push = h.find_text("Push").expect("Push tile");
    h.click_at(push);
    frames(&mut h, 2);
    let transition = |h: &TestHarness, id: &str| {
        h.editor
            .session
            .course
            .slide(id)
            .and_then(|s| s.transition.clone())
    };
    let t = transition(&h, "sl1").expect("a transition");
    assert!(matches!(t.effect, TransitionEffect::Push { .. }));
    super::gallery::run_effect(
        &mut h.editor,
        super::spec::EffectOp::From(super::spec::Motion::Transition, Direction::Top),
    );
    frames(&mut h, 2);
    assert_eq!(
        transition(&h, "sl1").map(|t| t.effect),
        Some(TransitionEffect::Push {
            from: Direction::Top
        })
    );
    let duration = h.find_text("Duration").expect("Duration label");
    let field = h
        .find_text_containing("0.5")
        .filter(|p| (p.y - duration.y).abs() < 4.0 && p.x > duration.x)
        .expect("duration field");
    h.set_field(field, "1.5");
    frames(&mut h, 2);
    assert_eq!(transition(&h, "sl1").map(|t| t.duration_ms), Some(1500));
    let all = h.find_text("Apply To All").expect("Apply To All");
    h.click_at(all);
    frames(&mut h, 2);
    assert_eq!(transition(&h, "sl2"), transition(&h, "sl1"));
}

#[test]
fn the_entrance_gallery_animates_the_selected_object() {
    let mut h = harness();
    h.editor.selection.object = Some("sh1".into());
    open_tab(&mut h, "animations");
    // Fade is near the start of the gallery, so its tile is always in view.
    let tiles = h.find_texts("Fade in");
    let fade = tiles.first().copied().expect("Fade in tile");
    h.click_at(fade);
    frames(&mut h, 2);
    let object = |h: &TestHarness| {
        h.editor
            .session
            .course
            .slide("sl1")
            .and_then(|s| s.object("sh1"))
            .map(|(o, _)| o.clone())
            .expect("shape")
    };
    assert!(matches!(
        object(&h).entrance.map(|a| a.effect),
        Some(e) if e.id == "fade"
    ));
    assert!(object(&h).exit.is_none(), "the first Fade in is Entrance");
}

#[test]
fn galleries_say_why_they_are_off() {
    let h = harness();
    assert!(super::gallery::available(&h.editor, super::spec::Gallery::Entrance).is_err());
    assert!(
        super::gallery::effect_available(
            &h.editor,
            super::spec::EffectOp::Options(super::spec::Motion::Transition)
        )
        .is_err(),
        "no transition yet"
    );
}

/// Give the shape `sh1` an entrance of `effect` (registry defaults).
fn with_entrance(h: &mut TestHarness, effect: &str) {
    h.editor.selection.object = Some("sh1".into());
    h.editor.run(
        "object.set_animation",
        serde_json::json!({
            "slide": "sl1", "object": "sh1", "which": "entrance",
            "animation": {
                "effect": coursecraft_course::AnimEffect::new(effect),
                "duration_ms": 500
            }
        }),
    );
    frames(h, 2);
}

fn entrance_effect(h: &TestHarness) -> coursecraft_course::AnimEffect {
    h.editor
        .session
        .course
        .slide("sl1")
        .and_then(|s| s.object("sh1"))
        .and_then(|(o, _)| o.entrance.clone())
        .map(|a| a.effect)
        .expect("an entrance")
}

#[test]
fn effect_options_follow_the_registry() {
    use super::effect_art::OptionValue;
    use super::spec::{EffectOp, Motion};
    let mut h = harness();
    with_entrance(&mut h, "fade");
    assert!(
        super::gallery::effect_available(&h.editor, EffectOp::Options(Motion::Entrance)).is_err(),
        "Fade has no options"
    );
    with_entrance(&mut h, "shape");
    assert!(
        super::gallery::effect_available(&h.editor, EffectOp::Options(Motion::Entrance)).is_ok()
    );
    super::gallery::set_option(
        &mut h.editor,
        Motion::Entrance,
        OptionValue::Shape(coursecraft_course::RevealShape::Diamond),
    );
    assert_eq!(
        entrance_effect(&h).shape(),
        Some(coursecraft_course::RevealShape::Diamond)
    );
    // An option the effect doesn't take changes nothing.
    let before = entrance_effect(&h);
    super::gallery::set_option(&mut h.editor, Motion::Entrance, OptionValue::Spokes(3));
    assert_eq!(entrance_effect(&h), before);
}

#[test]
fn the_effect_options_menu_sets_an_option() {
    let mut h = harness();
    with_entrance(&mut h, "shape");
    open_tab(&mut h, "animations");
    let before = h.find_texts("Shape").len();
    let options = h.find_texts("Effect Options");
    let first = options.first().copied().expect("Effect Options");
    h.click_at(first);
    frames(&mut h, 3);
    assert!(
        h.find_texts("Shape").len() > before,
        "the menu opens with its Shape section"
    );
    let diamond = h.find_text_last("Diamond").expect("Diamond row");
    h.click_at(diamond);
    frames(&mut h, 2);
    assert_eq!(
        entrance_effect(&h).shape(),
        Some(coursecraft_course::RevealShape::Diamond)
    );
    assert!(h.find_text("Diamond").is_none(), "the menu closes");
}

#[test]
fn the_emphasis_gallery_adds_an_emphasis_at_the_playhead() {
    let mut h = harness();
    h.editor.selection.object = Some("sh1".into());
    h.editor.ui.timeline.playhead_ms = 1500;
    open_tab(&mut h, "animations");
    let pulse = h.find_texts("Pulse");
    let tile = pulse.first().copied().expect("Pulse tile");
    h.click_at(tile);
    frames(&mut h, 2);
    let emphasis = h
        .editor
        .session
        .course
        .slide("sl1")
        .and_then(|s| s.object("sh1"))
        .map(|(o, _)| o.emphasis.clone())
        .expect("shape");
    assert_eq!(emphasis.len(), 1);
    assert_eq!(emphasis[0].effect.id, "pulse");
    assert_eq!(emphasis[0].start_ms, 1500);
    assert!(h.editor.ui.anim_preview.is_playing(), "a pick plays it");
}

#[test]
fn the_animation_painter_copies_animations_onto_a_clicked_object() {
    let json = serde_json::json!({
        "schema_version": 1, "id": "ap", "title": "Painter",
        "slide_size": { "w": 1000, "h": 600 },
        "scenes": [{ "id": "sc1", "title": "Scene 1", "slides": [
            { "id": "sl1", "title": "One", "duration_ms": 5000, "objects": [
                { "id": "src", "kind": { "type": "shape", "shape": "rectangle", "fill": "#112233" },
                  "frame": { "x": 100, "y": 100, "w": 200, "h": 120 },
                  "entrance": { "effect": { "type": "wipe" }, "duration_ms": 700 },
                  "emphasis": [{ "effect": { "type": "shake" }, "start_ms": 1000 }] },
                { "id": "dst", "kind": { "type": "shape", "shape": "rectangle", "fill": "#445566" },
                  "frame": { "x": 600, "y": 100, "w": 200, "h": 120 } }
            ] }
        ] }]
    });
    let course = Course::from_json(&json.to_string()).expect("valid course");
    let mut h = TestHarness::new_with_size(course, (2400.0, 900.0));
    h.editor.ui.ribbon.visible = true;
    h.editor.selection = crate::Selection::new("sl1");
    h.editor.selection.object = Some("src".into());
    frames(&mut h, 2);
    h.editor.arm_animation_painter(false);
    assert!(h.editor.animation_painter_armed());
    h.editor.apply_format_painter(&["dst".to_owned()]);
    let dst = h
        .editor
        .session
        .course
        .slide("sl1")
        .and_then(|s| s.object("dst"))
        .map(|(o, _)| o.clone())
        .expect("dst");
    assert_eq!(dst.entrance.map(|a| a.effect.id), Some("wipe".to_owned()));
    assert_eq!(dst.emphasis.len(), 1);
    assert!(h.editor.ui.format_painter.is_none(), "one use disarms");
    // The ribbon button reports why it is off with nothing selected.
    h.editor.selection.clear_objects();
    assert!(
        super::availability(
            &h.editor,
            super::spec::Action::Ui(super::spec::UiAction::AnimationPainter),
            None
        )
        .is_err()
    );
}

#[test]
fn effect_options_offer_a_sequence_for_text() {
    use super::spec::{EffectOp, Motion};
    let json = serde_json::json!({
        "schema_version": 1, "id": "sq", "title": "Sequence",
        "slide_size": { "w": 1000, "h": 600 },
        "scenes": [{ "id": "sc1", "title": "Scene 1", "slides": [
            { "id": "sl1", "title": "One", "duration_ms": 5000, "objects": [
                { "id": "tx", "kind": { "type": "text", "text": "One\nTwo" },
                  "frame": { "x": 100, "y": 100, "w": 300, "h": 120 },
                  "entrance": { "effect": { "type": "fade" }, "duration_ms": 800 } }
            ] }
        ] }]
    });
    let course = Course::from_json(&json.to_string()).expect("valid course");
    let mut h = TestHarness::new_with_size(course, (2400.0, 900.0));
    h.editor.ui.ribbon.visible = true;
    h.editor.selection = crate::Selection::new("sl1");
    h.editor.selection.object = Some("tx".into());
    frames(&mut h, 2);
    // Fade has no options, but text can play by paragraph.
    assert!(
        super::gallery::effect_available(&h.editor, EffectOp::Options(Motion::Entrance)).is_ok()
    );
    open_tab(&mut h, "animations");
    let options = h.find_texts("Effect Options");
    h.click_at(options.first().copied().expect("Effect Options"));
    frames(&mut h, 3);
    let by_paragraph = h.find_text_last("By paragraph").expect("By paragraph row");
    h.click_at(by_paragraph);
    frames(&mut h, 2);
    let a = h
        .editor
        .session
        .course
        .slide("sl1")
        .and_then(|s| s.object("tx"))
        .and_then(|(o, _)| o.entrance.clone())
        .expect("entrance");
    assert_eq!(a.sequence, coursecraft_course::Sequence::ByParagraph);
    assert_eq!(a.duration_ms, 800, "the rest is kept");
}
