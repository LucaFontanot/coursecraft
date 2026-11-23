//! Checks on the ribbon spec, and the ribbon drawn in the editor
//! (docs/ui/implementation.md § 5).

use std::collections::HashSet;

use super::spec::{self, Action, Kind, Menu, TABS};
use crate::test_harness::TestHarness;
use coursecraft_course::Course;

const HELLO_COURSE: &str = include_str!("../../../../examples/hello-course/course.json");

fn harness_with_ribbon() -> TestHarness {
    let course = Course::from_json(HELLO_COURSE).expect("hello course parses");
    let mut h = TestHarness::new_with_size(course, (1600.0, 900.0));
    h.editor.ui.ribbon.visible = true;
    h.idle_frame();
    h
}

#[test]
fn every_command_exists_in_the_engine_registry() {
    for item in spec::all_items() {
        if let Action::Command(id) = item.action {
            assert!(
                coursecraft_engine::registry().iter().any(|c| c.id == id),
                "{}: unknown command {id}",
                item.id
            );
        }
    }
}

#[test]
fn disabled_controls_say_why() {
    for item in spec::all_items() {
        if let Action::NotYet(reason) | Action::NotPlanned(reason) = item.action {
            assert!(!reason.trim().is_empty(), "{} has no reason", item.id);
        }
        assert!(!item.label.trim().is_empty(), "{} has no label", item.id);
    }
}

#[test]
fn item_ids_are_unique_within_each_variant() {
    // Shared groups (Publish) appear in both Home variants by design.
    for tab in TABS {
        for groups in [Some(tab.groups), tab.map_groups].into_iter().flatten() {
            let mut seen = HashSet::new();
            for item in groups.iter().flat_map(|g| g.items.iter()) {
                assert!(
                    seen.insert(item.id),
                    "{}: duplicate item id {}",
                    tab.id,
                    item.id
                );
                if let Menu::Items(entries) = item.menu {
                    for entry in entries {
                        assert!(
                            seen.insert(entry.id),
                            "{}: duplicate item id {}",
                            tab.id,
                            entry.id
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn only_split_controls_have_menus() {
    for item in spec::all_items() {
        let has_menu = !matches!(item.menu, Menu::None);
        if has_menu {
            assert!(item.kind.is_split(), "{} has a menu but no ▾", item.id);
        }
        if let Menu::Items(entries) = item.menu {
            assert!(!entries.is_empty(), "{} has an empty menu", item.id);
        }
    }
}

#[test]
fn tab_strip_matches_the_documented_order() {
    let ids: Vec<&str> = TABS.iter().map(|t| t.id).collect();
    assert_eq!(
        ids,
        [
            "home",
            "insert",
            "slides",
            "design",
            "transitions",
            "animations",
            "view",
            "help"
        ],
        "docs/ui/ribbon.md › Tab strip"
    );
}

#[test]
fn every_built_tab_ends_with_publish() {
    for tab in TABS {
        for groups in [Some(tab.groups), tab.map_groups].into_iter().flatten() {
            if let Some(last) = groups.last() {
                assert_eq!(last.label, "Publish", "{} must end with Publish", tab.id);
            }
        }
    }
    let home = spec::tab("home").expect("home tab");
    assert!(!home.groups.is_empty() && home.map_groups.is_some());
}

#[test]
fn combo_fields_have_a_width() {
    for item in spec::all_items() {
        if let Kind::Combo { width } = item.kind {
            assert!(width >= 40.0, "{} is too narrow", item.id);
        }
    }
}

#[test]
fn ribbon_is_off_until_view_ribbon() {
    let course = Course::from_json(HELLO_COURSE).expect("hello course parses");
    let mut h = TestHarness::new_with_size(course, (1600.0, 900.0));
    h.idle_frame();
    assert!(h.find_text("Clipboard").is_none());
    h.editor.ui.ribbon.visible = true;
    assert!(h.find_text("Clipboard").is_some());
}

#[test]
fn home_draws_its_groups_and_tab_strip() {
    let mut h = harness_with_ribbon();
    for text in [
        "File",
        "Home",
        "Insert",
        "Help",
        "Clipboard",
        "Slide",
        "AI",
        "Font",
        "Paragraph",
        "Drawing",
        "Publish",
        "New\nSlide",
        "Format Painter",
    ] {
        assert!(h.find_text(text).is_some(), "{text:?} is not drawn");
    }
}

#[test]
fn disabled_control_explains_and_does_nothing() {
    let mut h = harness_with_ribbon();
    let pos = h.find_text("Insert\nText").expect("Insert Text");
    let tip = h.probe_tooltip(pos).unwrap_or_default();
    assert!(
        tip.contains("Not available yet: AI text drafting"),
        "tooltip was {tip:?}"
    );
    let before = h.editor.session.course.clone();
    h.click_at(pos);
    h.idle_frame();
    assert_eq!(h.editor.session.course, before, "a disabled control ran");
}

#[test]
fn new_slide_adds_a_slide() {
    let mut h = harness_with_ribbon();
    let before = h.editor.session.course.slides().count();
    let pos = h.find_text("New\nSlide").expect("New Slide");
    h.click_at(pos);
    h.idle_frame();
    assert_eq!(h.editor.session.course.slides().count(), before + 1);
}

#[test]
fn course_map_shows_the_map_variant_of_home() {
    let mut h = harness_with_ribbon();
    h.editor.ui.story_view = true;
    assert!(h.find_text("New\nScene").is_some());
    assert!(h.find_text("Paragraph").is_none());
}

#[test]
fn slide_tabs_are_disabled_in_the_course_map() {
    let mut h = harness_with_ribbon();
    h.editor.ui.story_view = true;
    let blocked = super::availability(
        &h.editor,
        Action::Ui(spec::UiAction::Preview),
        Some("Open a slide first"),
    );
    assert_eq!(blocked, Err("Open a slide first".to_owned()));
}

#[test]
fn tabs_still_being_built_say_so() {
    let mut h = harness_with_ribbon();
    let pos = h.find_text("Design").expect("Design tab");
    h.click_at(pos);
    h.idle_frame();
    assert_eq!(h.editor.ui.ribbon.tab, "design");
    assert!(
        h.find_text("This tab is being built. Its commands are in the menu bar for now.")
            .is_some()
    );
}

#[test]
fn clicking_the_open_tab_collapses_the_ribbon() {
    let mut h = harness_with_ribbon();
    let pos = h.find_text("Home").expect("Home tab");
    h.click_at(pos);
    h.idle_frame();
    assert!(h.editor.ui.ribbon.collapsed);
    assert!(h.find_text("Clipboard").is_none());
}

#[test]
fn ribbon_strings_go_through_tl() {
    let mut h = harness_with_ribbon();
    h.editor.set_language("pseudo");
    for text in [
        "Home",
        "Clipboard",
        "New\nSlide",
        "Paragraph",
        "Format Painter",
    ] {
        assert!(
            h.find_text(&format!("⟦{text}⟧")).is_some(),
            "{text:?} is not drawn through tl!"
        );
        assert!(h.find_text(text).is_none(), "{text:?} is drawn without tl!");
    }
}
