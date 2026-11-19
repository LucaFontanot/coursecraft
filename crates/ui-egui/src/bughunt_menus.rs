//! Bug hunt (2026-11-20): dropdowns in the Format panel, and menus opened over
//! other panels, on the healthy-habits course.

use crate::properties::InspectorTab;
use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use egui::Pos2;

fn healthy() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/healthy-habits/course.json"
    ))
    .expect("healthy habits");
    Course::from_json(&json).expect("valid course")
}

fn select_object(h: &mut TestHarness, slide: &str, object: &str, tab: InspectorTab) {
    h.editor.selection.slide = slide.into();
    h.editor.selection.layer = None;
    h.editor.selection.set_many([object]);
    h.editor.ui.inspector_tab = tab;
    h.idle_frame();
    h.idle_frame();
}

/// Each Fill type item on a button: the selection and tab must not change, the
/// course must change, and one undo must restore it exactly.
#[test]
fn every_fill_type_item_keeps_selection_and_undoes_cleanly() {
    let mut problems = Vec::new();
    for item in ["Solid", "Linear gradient", "Radial gradient"] {
        let start = healthy();
        let mut h = TestHarness::new_with_size(start.clone(), (1280.0, 800.0));
        select_object(&mut h, "welcome", "w_start", InspectorTab::Format);
        if item == "Solid" {
            // Switch away first so picking Solid is a real change.
            h.pick_combo("Solid", "Linear gradient");
        }
        let current = if item == "Solid" {
            "Linear gradient"
        } else {
            "Solid"
        };
        let before = h.editor.session.course.clone();
        let steps = h.editor.undo_len_for_test();
        h.pick_combo(current, item);
        if h.editor.selection.object.as_deref() != Some("w_start") {
            problems.push(format!(
                "{item}: selection changed to {:?}",
                h.editor.selection.object
            ));
        }
        if h.editor.ui.inspector_tab != InspectorTab::Format {
            problems.push(format!(
                "{item}: inspector tab changed to {:?}",
                h.editor.ui.inspector_tab
            ));
        }
        if h.editor.session.course == before {
            problems.push(format!("{item}: picking it changed nothing"));
            continue;
        }
        if h.editor.undo_len_for_test() != steps + 1 {
            problems.push(format!("{item}: did not add exactly one undo step"));
        }
        h.press(egui::Key::Z, egui::Modifiers::COMMAND);
        h.idle_frame();
        if h.editor.session.course != before {
            problems.push(format!("{item}: undo did not restore the course"));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// Opening the Fill type menu and clicking away on a panel underneath must only
/// close the menu. Checked over the canvas, the slide list, the timeline and the
/// layer rows.
#[test]
fn clicking_away_from_fill_menu_over_each_panel_changes_nothing() {
    type Spot = Box<dyn Fn(&TestHarness) -> Pos2>;
    let spots: Vec<(&str, Spot)> = vec![
        (
            "canvas",
            Box::new(|h: &TestHarness| h.central_panel_rect().center()),
        ),
        (
            "timeline",
            Box::new(|h: &TestHarness| {
                Pos2::new(
                    h.central_panel_rect().center().x,
                    h.bottom_panel_top() + 30.0,
                )
            }),
        ),
    ];
    let mut problems = Vec::new();
    for (name, spot) in spots {
        let mut h = TestHarness::new_with_size(healthy(), (1100.0, 700.0));
        select_object(&mut h, "welcome", "w_start", InspectorTab::Format);
        let before = h.editor.session.course.clone();
        let combo = h
            .find_text_in("Solid", h.right_panel_rect())
            .expect("fill combo");
        h.click_at(combo);
        h.idle_frame();
        let p = spot(&h);
        h.click_at(p);
        h.idle_frame();
        if h.editor.session.course != before {
            problems.push(format!("{name}: clicking away changed the course"));
        }
        if h.editor.selection.object.as_deref() != Some("w_start") {
            problems.push(format!(
                "{name}: selection changed to {:?}",
                h.editor.selection.object
            ));
        }
        if h.editor.ui.inspector_tab != InspectorTab::Format {
            problems.push(format!("{name}: inspector tab changed"));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// BUG: the press that closes an open Fill type menu, when it lands on the slide
/// list, also acts on the slide list: the object selection is dropped and the
/// inspector jumps to Slide. The same fix as for the Slide Layers panel is needed.
#[test]
#[ignore = "BUG: clicking away from an open menu over the slide list also acts on the slide list"]
fn clicking_away_from_fill_menu_over_the_slide_list_changes_nothing() {
    let mut h = TestHarness::new_with_size(healthy(), (1100.0, 700.0));
    select_object(&mut h, "welcome", "w_start", InspectorTab::Format);
    let combo = h
        .find_text_in("Solid", h.right_panel_rect())
        .expect("fill combo");
    h.click_at(combo);
    h.idle_frame();
    assert!(egui::Popup::is_any_open(&h.ctx), "the menu is open");
    let before = h.editor.session.course.clone();
    h.click_at(h.left_panel_rect().center());
    h.idle_frame();
    assert!(
        !egui::Popup::is_any_open(&h.ctx),
        "the press closed the menu"
    );
    assert_eq!(h.editor.session.course, before);
    assert_eq!(
        h.editor.selection.slide, "welcome",
        "the slide did not switch"
    );
    assert_eq!(h.editor.selection.object.as_deref(), Some("w_start"));
    assert_eq!(h.editor.ui.inspector_tab, InspectorTab::Format);
}

/// With the menu open over the slide list, picking a menu item must change only the
/// fill and must not switch the slide.
#[test]
fn clicking_a_menu_item_over_the_slide_list_does_not_switch_slide() {
    let mut h = TestHarness::new_with_size(healthy(), (1100.0, 700.0));
    select_object(&mut h, "welcome", "w_start", InspectorTab::Format);
    let combo = h
        .find_text_in("Solid", h.right_panel_rect())
        .expect("fill combo");
    h.click_at(combo);
    h.idle_frame();
    let item = h
        .find_text_last("Picture")
        .or_else(|| h.find_text_last("Solid"));
    if let Some(item) = item {
        h.click_at(item);
        h.idle_frame();
    }
    assert_eq!(
        h.editor.selection.slide, "welcome",
        "the slide did not change"
    );
}
