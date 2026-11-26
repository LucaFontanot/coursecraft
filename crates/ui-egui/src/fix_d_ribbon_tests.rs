//! Fixes of the 0.5.2 review, package D: the ribbon (Q10, S2, S3, S4, S5), the
//! ribbon text state (P4), the timeline (N10, P8) and state cards (P11).

use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use egui::{Key, Modifiers};

const HELLO_COURSE: &str = include_str!("../../../examples/hello-course/course.json");

fn harness() -> TestHarness {
    let course = Course::from_json(HELLO_COURSE).expect("hello course parses");
    let mut h = TestHarness::new_with_size(course, (1600.0, 900.0));
    h.editor.ui.ribbon.visible = true;
    h.idle_frame();
    h
}

#[test]
fn a_click_on_the_open_tab_keeps_the_ribbon_open() {
    let mut h = harness();
    let pos = h.find_text("Home").expect("Home tab");
    h.click_at(pos);
    h.advance_time(1.0);
    h.idle_frame();
    assert!(!h.editor.ui.ribbon.collapsed);
    assert!(h.find_text("Clipboard").is_some());
}

#[test]
fn a_double_click_on_the_open_tab_toggles_the_ribbon_once() {
    let mut h = harness();
    let pos = h.find_text("Home").expect("Home tab");
    h.double_click_at(pos);
    h.idle_frame();
    assert!(
        h.editor.ui.ribbon.collapsed,
        "collapsed by one double-click"
    );
    assert!(h.find_text("Clipboard").is_none());
    h.advance_time(1.0);
    h.double_click_at(pos);
    h.idle_frame();
    assert!(!h.editor.ui.ribbon.collapsed, "expanded by the next one");
    assert_eq!(h.editor.ui.ribbon.tab, "home");
}

#[test]
fn a_double_click_on_another_tab_opens_it_and_does_not_collapse() {
    let mut h = harness();
    let pos = h.find_text("Insert").expect("Insert tab");
    h.double_click_at(pos);
    h.idle_frame();
    assert_eq!(h.editor.ui.ribbon.tab, "insert");
    assert!(!h.editor.ui.ribbon.collapsed);
}

#[test]
fn a_click_on_a_tab_expands_a_collapsed_ribbon() {
    let mut h = harness();
    h.editor.ui.ribbon.collapsed = true;
    h.idle_frame();
    let pos = h.find_text("Insert").expect("Insert tab");
    h.click_at(pos);
    h.advance_time(1.0);
    h.idle_frame();
    assert!(!h.editor.ui.ribbon.collapsed);
    assert_eq!(h.editor.ui.ribbon.tab, "insert");
}

#[test]
fn a_new_object_does_not_expand_a_collapsed_ribbon() {
    let mut h = harness();
    h.editor.ui.ribbon.collapsed = true;
    h.idle_frame();
    crate::toolbar::insert_table(&mut h.editor);
    h.idle_frame();
    h.idle_frame();
    assert!(h.editor.ui.ribbon.collapsed);
    assert_eq!(h.editor.ui.ribbon.tab, "home");
}

#[test]
fn new_style_names_skip_the_ones_in_use() {
    let named = |name: &str| -> coursecraft_course::NamedTextStyle {
        serde_json::from_value(serde_json::json!({
            "id": name, "name": name, "style": {}
        }))
        .expect("a named style")
    };
    assert_eq!(super::ribbon::free_style_name(&[]), "Style 1");
    assert_eq!(
        super::ribbon::free_style_name(&[named("Style 2")]),
        "Style 1"
    );
    assert_eq!(
        super::ribbon::free_style_name(&[named("Style 1"), named("Style 3")]),
        "Style 2"
    );
}

#[test]
fn ribbon_tooltips_show_the_shortcut() {
    let mut h = harness();
    let pos = h.find_text("Paste").expect("Paste");
    let tip = h.probe_tooltip(pos).unwrap_or_default();
    // macOS shows Cmd where Windows and Linux show Ctrl.
    assert!(
        tip.contains("Ctrl+V") || tip.contains("Cmd+V"),
        "tooltip was {tip:?}"
    );
}

#[test]
fn ctrl_f1_collapses_unless_a_dialog_is_open() {
    let mut h = harness();
    h.press(Key::F1, Modifiers::CTRL);
    assert!(h.editor.ui.ribbon.collapsed);
    h.press(Key::F1, Modifiers::CTRL);
    assert!(!h.editor.ui.ribbon.collapsed);
    h.editor.ui.show_shortcuts_dialog = true;
    h.idle_frame();
    h.press(Key::F1, Modifiers::CTRL);
    assert!(
        !h.editor.ui.ribbon.collapsed,
        "Ctrl+F1 must not act behind a dialog"
    );
}

#[test]
fn state_cards_are_composed_once_until_the_course_changes() {
    let mut h = TestHarness::new(super::m29_states_tab_tests::course_with_states(3));
    h.editor.selection.object = Some("btn1".to_owned());
    h.editor.ui.bottom_tab = crate::BottomTab::States;
    h.idle_frame();
    h.idle_frame();
    let cached = h.editor.ui.states.thumb_cards.clone();
    assert!(!cached.is_empty(), "the visible cards were composed");
    h.idle_frame();
    assert_eq!(h.editor.ui.states.thumb_cards, cached, "reused as they are");
    h.editor.run(
        "object.set_opacity",
        serde_json::json!({ "slide": h.editor.selection.slide, "object": "btn1", "opacity": 0.5 }),
    );
    assert!(h.editor.ui.last_error.is_none());
    h.idle_frame();
    assert!(
        h.editor.ui.states.thumb_cards != cached,
        "an edit composes the cards again"
    );
}

fn layered_course() -> Course {
    let rect = |id: &str, hidden: bool, locked: bool| {
        serde_json::json!({
            "id": id,
            "kind": { "type": "shape", "shape": "rectangle", "fill": "#336699" },
            "frame": { "x": 10, "y": 10, "w": 50, "h": 40 },
            "editor_hidden": hidden,
            "locked": locked
        })
    };
    let json = serde_json::json!({
        "schema_version": 1,
        "id": "c",
        "title": "Layers",
        "slide_size": { "w": 1000, "h": 600 },
        "scenes": [{ "id": "sc1", "title": "Scene", "slides": [{
            "id": "sl1",
            "title": "Slide",
            "duration_ms": 5000,
            "objects": [rect("base", true, true)],
            "layers": [{
                "id": "ly1",
                "name": "Layer",
                "duration_ms": 5000,
                "objects": [rect("inlayer", true, true)]
            }]
        }]}]
    });
    Course::from_json(&json.to_string()).expect("valid course")
}

#[test]
fn show_all_and_unlock_all_cover_every_layer_in_one_undo_step() {
    let mut h = TestHarness::new_with_size(layered_course(), (2400.0, 900.0));
    h.editor.selection = crate::Selection::new("sl1");
    h.idle_frame();
    h.idle_frame();
    let flags = |h: &TestHarness| -> Vec<(bool, bool)> {
        let slide = h.editor.session.course.slide("sl1").expect("slide");
        slide
            .objects
            .iter()
            .chain(slide.layers.iter().flat_map(|l| l.objects.iter()))
            .map(|o| (o.editor_hidden, o.locked))
            .collect()
    };
    assert_eq!(flags(&h), [(true, true), (true, true)]);
    let show = h
        .find_text_containing("Show all (2)")
        .expect("Show all (2)");
    h.click_at(show);
    h.idle_frame();
    assert_eq!(flags(&h), [(false, true), (false, true)]);
    let unlock = h
        .find_text_containing("Unlock all (2)")
        .expect("Unlock all (2)");
    h.click_at(unlock);
    h.idle_frame();
    assert_eq!(flags(&h), [(false, false), (false, false)]);
    h.press(Key::Z, Modifiers::COMMAND);
    h.idle_frame();
    assert_eq!(flags(&h), [(false, true), (false, true)], "one undo step");
}
