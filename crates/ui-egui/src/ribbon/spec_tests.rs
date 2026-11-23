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
    // Help has no Publish group (ribbon.md › Help).
    for tab in TABS.iter().filter(|t| t.id != "help") {
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
fn ribbon_and_docking_are_the_default() {
    let editor = crate::Editor::new(Course::from_json(HELLO_COURSE).expect("parses"));
    assert!(editor.ui.ribbon.visible);
    assert!(editor.ui.dock.enabled);
    let prefs = crate::dock::LayoutPrefs::default();
    assert!(prefs.ribbon && prefs.docked && prefs.workspace.is_none());
}

#[test]
fn ribbon_follows_its_setting() {
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
fn every_main_tab_is_built() {
    for tab in TABS {
        assert!(!tab.groups.is_empty(), "{} has no groups", tab.id);
    }
}

#[test]
fn design_tab_opens_the_slide_size_dialog() {
    let mut h = harness_with_ribbon();
    let pos = h.find_text("Design").expect("Design tab");
    h.click_at(pos);
    h.idle_frame();
    assert_eq!(h.editor.ui.ribbon.tab, "design");
    let pos = h
        .find_text("Slide\nSize")
        .or_else(|| h.find_text("Slide Size"));
    h.click_at(pos.expect("Slide Size"));
    h.idle_frame();
    assert!(h.editor.ui.slide_size_dialog.open);
}

#[test]
fn help_tab_opens_the_shortcuts_sheet() {
    let mut h = harness_with_ribbon();
    h.editor.ui.ribbon.tab = "help";
    h.idle_frame();
    let pos = h
        .find_text("Keyboard\nShortcuts")
        .expect("Keyboard Shortcuts");
    h.click_at(pos);
    h.idle_frame();
    assert!(h.editor.ui.show_shortcuts_dialog);
}

#[test]
fn add_motion_path_needs_an_object() {
    let h = harness_with_ribbon();
    let state = super::availability(&h.editor, Action::Ui(spec::UiAction::AddMotionPath), None);
    assert_eq!(state, Err("Select an object first".to_owned()));
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

fn has_action(action: spec::UiAction) -> bool {
    spec::all_items().any(|i| i.action == Action::Ui(action))
}

#[test]
fn every_insert_menu_command_has_a_ribbon_home() {
    // The menu bar's Insert menu is hidden while the ribbon is on.
    use spec::UiAction as U;
    for cmd in coursecraft_engine::registry() {
        if !cmd.menu.starts_with("Insert/") {
            continue;
        }
        let home = match cmd.id {
            "slide.add" => U::NewSlide,
            "scene.add" => U::NewScene,
            "layer.add" => U::SlideLayer,
            "course.import_outline" => U::ImportOutline,
            "trigger.add" => U::Trigger,
            "question.insert" => U::Question,
            "results.insert" => U::ResultsSlide,
            "certificate.insert" => U::Certificate,
            other => panic!("{other} ({}) has no ribbon home", cmd.menu),
        };
        assert!(has_action(home), "{} is not on the ribbon", cmd.id);
    }
}

#[test]
fn every_flat_toolbar_and_insert_menu_extra_has_a_ribbon_home() {
    use spec::UiAction as U;
    for action in [
        // Toolbar
        U::NewSlide,
        U::NewScene,
        U::SlideLayer,
        U::Preview,
        U::Publish,
        U::CourseMap,
        U::FormatPainter,
        U::TextBox,
        U::Shape,
        U::Picture,
        U::Video,
        U::Audio,
        U::Lottie,
        U::RecordNarration,
        U::Button,
        U::Question,
        U::Trigger,
        // Insert menu extras
        U::Table,
        U::InputText,
        U::InputCheckbox,
        U::InputRadio,
        U::Slider,
        U::Dial,
        U::Interaction,
        U::WebObject,
        U::NarrationFromText,
        U::ImportDocument,
    ] {
        assert!(has_action(action), "{action:?} has no ribbon home");
    }
}

#[test]
fn insert_tab_draws_and_places_a_text_box() {
    let course = Course::from_json(HELLO_COURSE).expect("hello course parses");
    let mut h = TestHarness::new_with_size(course, (2600.0, 900.0));
    h.editor.ui.ribbon.visible = true;
    h.idle_frame();
    let pos = h.find_text("Insert").expect("Insert tab");
    h.click_at(pos);
    h.idle_frame();
    for text in ["Media", "Text", "Interactive Objects", "Text Box"] {
        assert!(h.find_text(text).is_some(), "{text:?} is not drawn");
    }
    let pos = h.find_text("Text Box").expect("Text Box");
    h.click_at(pos);
    h.idle_frame();
    assert_eq!(
        h.editor.ui.placement_tool,
        Some(crate::PlacementTool::TextBox)
    );
}

#[test]
fn view_tab_switches_to_the_course_map_and_back() {
    let mut h = harness_with_ribbon();
    h.editor.ui.ribbon.tab = "view";
    h.idle_frame();
    let pos = h.find_text("Course\nMap").expect("Course Map");
    h.click_at(pos);
    h.idle_frame();
    assert!(h.editor.ui.story_view);
    h.editor.ui.ribbon.tab = "view";
    h.idle_frame();
    let pos = h.find_text("Slide").expect("Slide view");
    h.click_at(pos);
    h.idle_frame();
    assert!(!h.editor.ui.story_view);
}

#[test]
fn view_tab_toggles_the_grid_through_the_command() {
    let mut h = harness_with_ribbon();
    h.editor.ui.ribbon.tab = "view";
    h.idle_frame();
    let before = h.editor.session.course.editor.show_grid;
    let pos = h.find_text("Grid").expect("Grid");
    h.click_at(pos);
    h.idle_frame();
    assert_eq!(h.editor.session.course.editor.show_grid, !before);
}

#[test]
fn the_ribbon_replaces_the_toolbar_and_the_insert_menu() {
    let mut h = harness_with_ribbon();
    for _ in 0..2 {
        h.idle_frame();
    }
    // The toolbar's "Add slide" and the inspector switch are gone; the
    // inspector draws its own tabs.
    assert!(h.find_text("Add slide").is_none());
    assert!(h.find_text("Animate").is_some(), "inspector tabs");
    h.editor.ui.ribbon.visible = false;
    h.idle_frame();
    assert!(
        h.find_text("Add slide").is_some(),
        "the old toolbar comes back"
    );
}

#[test]
fn inspector_tabs_switch_the_inspector() {
    let mut h = harness_with_ribbon();
    let pos = h.find_text("Course").expect("Course inspector tab");
    h.click_at(pos);
    h.idle_frame();
    assert_eq!(
        h.editor.ui.inspector_tab,
        crate::properties::InspectorTab::Course
    );
}

#[test]
fn quick_access_undo_reverts_the_last_change() {
    let mut h = harness_with_ribbon();
    let before = h.editor.session.course.slides().count();
    crate::toolbar::add_new_slide(&mut h.editor);
    h.idle_frame();
    assert_eq!(h.editor.session.course.slides().count(), before + 1);
    let undo = h
        .last_widgets
        .iter()
        .find(|w| w.label.as_deref().is_some_and(|l| l.starts_with("Undo")) && !w.disabled)
        .map(|w| w.bounds)
        .expect("Undo in the quick access toolbar");
    h.click_at(undo.center());
    h.idle_frame();
    assert_eq!(h.editor.session.course.slides().count(), before);
}

#[test]
fn groups_that_do_not_fit_shrink_to_one_button() {
    let course = Course::from_json(HELLO_COURSE).expect("hello course parses");
    let mut h = TestHarness::new_with_size(course, (1000.0, 800.0));
    h.editor.ui.ribbon.visible = true;
    h.editor.ui.ribbon.tab = "insert";
    for _ in 0..3 {
        h.idle_frame();
    }
    let tab = spec::tab("insert").expect("insert");
    let shrunk = super::shrunk_groups(&h.editor.ui.ribbon, tab.groups, 1000.0 - 30.0);
    assert!(shrunk.iter().any(|s| *s), "something shrinks at 1000 px");
    assert!(!shrunk[0], "the leftmost group stays whole");
    let wide = super::shrunk_groups(&h.editor.ui.ribbon, tab.groups, 5000.0);
    assert!(wide.iter().all(|s| !*s));
}
