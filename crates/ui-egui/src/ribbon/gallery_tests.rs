//! The ribbon's galleries drawn in the editor (ribbon.md › Design,
//! Transitions, Animations).

use crate::test_harness::TestHarness;
use coursecraft_course::{AnimEffect, Course, Direction, TransitionEffect};

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
    let tiles = h.find_texts("Fly In");
    let fly = tiles.first().copied().expect("Fly In tile");
    h.click_at(fly);
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
        Some(AnimEffect::FlyIn { .. })
    ));
    assert!(object(&h).exit.is_none(), "the first Fly In is Entrance");
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
