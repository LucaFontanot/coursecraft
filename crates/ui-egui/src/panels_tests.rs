//! Settings panels (`docs/ui/panels.md`): launchers, the Text panel, opening by
//! itself for the work in hand, and saved layouts that survive new panels.

use crate::dock::{self, Panel};
use crate::panels::{self, Family, Section};
use crate::ribbon::spec::{self, Action, UiAction};
use crate::test_harness::TestHarness;
use coursecraft_course::Course;

const HELLO_COURSE: &str = include_str!("../../../examples/hello-course/course.json");

fn harness(docked: bool) -> TestHarness {
    let course = Course::from_json(HELLO_COURSE).expect("hello course parses");
    let mut h = TestHarness::new_with_size(course, (1600.0, 900.0));
    h.editor.ui.dock.enabled = docked;
    h.editor.ui.ribbon.visible = true;
    for _ in 0..3 {
        h.idle_frame();
    }
    h
}

fn select_title(h: &mut TestHarness) {
    h.editor.selection = crate::Selection::new("s-intro");
    h.editor.selection.object = Some("title".into());
    h.idle_frame();
}

fn start_editing_title(h: &mut TestHarness) {
    select_title(h);
    h.editor.ui.inline_edit = Some(crate::canvas::InlineEdit {
        slide: "s-intro".into(),
        object: "title".into(),
        buffer: "Welcome".into(),
        original: "Welcome".into(),
        is_button: false,
        is_shape: false,
        cell: None,
        just_entered: true,
        live_runs: Vec::new(),
        live_paragraphs: Vec::new(),
        last_revision: 0,
        state_name: None,
        extra_id: None,
    });
}

fn frames(h: &mut TestHarness, n: usize) {
    for _ in 0..n {
        h.idle_frame();
    }
}

/// The panels sharing a tab group with `panel`.
fn group_of(h: &TestHarness, panel: Panel) -> Vec<Panel> {
    let value = serde_json::to_value(&h.editor.ui.dock.workspace.layout).expect("serializes");
    let id = serde_json::to_value(panel).expect("id");
    let mut found = Vec::new();
    fn walk(v: &serde_json::Value, id: &serde_json::Value, found: &mut Vec<Panel>) {
        match v {
            serde_json::Value::Object(map) => {
                if let Some(serde_json::Value::Array(panels)) = map.get("panels")
                    && panels.contains(id)
                {
                    found.extend(
                        panels
                            .iter()
                            .filter_map(|p| serde_json::from_value::<Panel>(p.clone()).ok()),
                    );
                }
                map.values().for_each(|v| walk(v, id, found));
            }
            serde_json::Value::Array(items) => items.iter().for_each(|v| walk(v, id, found)),
            _ => {}
        }
    }
    walk(&value, &id, &mut found);
    found
}

#[test]
fn every_panel_has_a_family_and_settings_panels_have_sections() {
    for panel in Panel::ALL {
        let sections = panels::sections(panel);
        match panels::family(panel) {
            Family::Settings => {
                assert!(!sections.is_empty(), "{panel:?} has sections");
                for section in sections {
                    assert_eq!(section.panel(), panel, "{section:?} belongs to {panel:?}");
                }
            }
            Family::Workspace | Family::Inspectors => {
                assert!(sections.is_empty(), "{panel:?} has no sections");
            }
        }
    }
}

#[test]
fn every_launcher_opens_a_panel_or_says_why_not() {
    let mut launchers = 0;
    for tab in spec::TABS {
        for group in tab
            .groups
            .iter()
            .chain(tab.map_groups.unwrap_or(&[]).iter())
        {
            let Some(action) = group.launcher else {
                continue;
            };
            launchers += 1;
            match action {
                Action::Ui(UiAction::PanelAt(panel, section)) => {
                    if let Some(section) = section {
                        assert!(
                            panels::sections(panel).contains(&section),
                            "{}: {section:?} is a section of {panel:?}",
                            group.id
                        );
                    }
                }
                Action::NotYet(_) | Action::NotPlanned(_) => {}
                other => panic!("{}: a launcher opens a panel, not {other:?}", group.id),
            }
        }
    }
    assert!(launchers >= 5, "the launchers are all checked: {launchers}");
}

#[test]
fn font_and_paragraph_launchers_open_the_text_panel_at_their_section() {
    let home = spec::tab("home").expect("home tab");
    let launcher = |id: &str| {
        home.groups
            .iter()
            .find(|g| g.id == id)
            .and_then(|g| g.launcher)
    };
    assert_eq!(
        launcher("font"),
        Some(Action::Ui(UiAction::PanelAt(
            Panel::Text,
            Some(Section::Font)
        )))
    );
    assert_eq!(
        launcher("paragraph"),
        Some(Action::Ui(UiAction::PanelAt(
            Panel::Text,
            Some(Section::Paragraph)
        )))
    );
}

#[test]
fn a_launcher_opens_the_text_panel_beside_the_inspectors_and_scrolls_to_its_section() {
    let mut h = harness(true);
    assert!(
        !dock::is_open(&h.editor, Panel::Text),
        "not in the default layout"
    );
    panels::open_at(&mut h.editor, Panel::Text, Some(Section::Paragraph));
    assert_eq!(h.editor.ui.panels.focus, Some(Section::Paragraph));
    frames(&mut h, 2);
    assert!(dock::is_open(&h.editor, Panel::Text));
    assert!(
        h.editor.ui.dock.rect(Panel::Text).is_some(),
        "drawn in front"
    );
    assert!(
        group_of(&h, Panel::Text).contains(&Panel::Format),
        "a tab of the inspectors' group: {:?}",
        group_of(&h, Panel::Text)
    );
    assert_eq!(h.editor.ui.panels.focus, None, "scrolled to, once");
}

#[test]
fn a_launcher_never_closes_its_panel() {
    let mut h = harness(true);
    panels::open_at(&mut h.editor, Panel::Text, Some(Section::Font));
    frames(&mut h, 2);
    panels::open_at(&mut h.editor, Panel::Text, Some(Section::Font));
    frames(&mut h, 2);
    assert!(dock::is_open(&h.editor, Panel::Text));
}

#[test]
fn the_text_panel_closes_and_comes_back_from_view_panels() {
    let mut h = harness(true);
    panels::open_at(&mut h.editor, Panel::Text, None);
    frames(&mut h, 1);
    panels::close(&mut h.editor, Panel::Text);
    frames(&mut h, 1);
    assert!(!dock::is_open(&h.editor, Panel::Text));
    let view = spec::tab("view").expect("view tab");
    let toggle = view
        .groups
        .iter()
        .flat_map(|g| g.items.iter())
        .any(|item| item.action == Action::Ui(UiAction::Panel(Panel::Text)));
    assert!(toggle, "View › Panels lists Text");
    crate::panels::open_at(&mut h.editor, Panel::Text, None);
    frames(&mut h, 1);
    assert!(dock::is_open(&h.editor, Panel::Text));
}

#[test]
fn editing_text_opens_the_text_panel_by_itself() {
    let mut h = harness(true);
    start_editing_title(&mut h);
    frames(&mut h, 3);
    assert!(dock::is_open(&h.editor, Panel::Text));
    assert!(h.editor.ui.dock.rect(Panel::Text).is_some(), "in front");
}

#[test]
fn a_text_panel_closed_while_editing_stays_closed() {
    let mut h = harness(true);
    start_editing_title(&mut h);
    frames(&mut h, 3);
    panels::close(&mut h.editor, Panel::Text);
    frames(&mut h, 3);
    assert!(
        !dock::is_open(&h.editor, Panel::Text),
        "only the start of the work opens it"
    );
}

#[test]
fn with_docking_off_a_launcher_floats_the_text_panel_and_editing_does_not() {
    let mut h = harness(false);
    start_editing_title(&mut h);
    frames(&mut h, 3);
    assert!(
        h.editor.ui.panels.floating.is_empty(),
        "no window over the canvas being edited"
    );
    h.editor.ui.inline_edit = None;
    frames(&mut h, 1);
    panels::open_at(&mut h.editor, Panel::Text, Some(Section::TextBox));
    frames(&mut h, 3);
    assert_eq!(h.editor.ui.panels.floating, vec![Panel::Text], "one window");
    assert!(h.find_text("Columns").is_some(), "the panel is drawn");
    panels::close(&mut h.editor, Panel::Text);
    assert!(h.editor.ui.panels.floating.is_empty(), "closes");
}

#[test]
fn the_text_panel_holds_the_text_controls_and_the_ones_only_it_has() {
    let mut h = harness(true);
    select_title(&mut h);
    for (section, labels) in [
        (Section::Font, &["Caps", "Highlight"][..]),
        (
            Section::Paragraph,
            &["Space before", "Space after", "Tab stops"][..],
        ),
        (Section::TextBox, &["Text Box", "Columns"][..]),
    ] {
        panels::open_at(&mut h.editor, Panel::Text, Some(section));
        frames(&mut h, 3);
        for label in labels {
            assert!(h.find_text(label).is_some(), "{label} shown at {section:?}");
        }
    }
}

#[test]
fn with_nothing_to_edit_the_text_panel_says_what_to_select() {
    let mut h = harness(true);
    h.editor.selection = crate::Selection::new("s-intro");
    panels::open_at(&mut h.editor, Panel::Text, None);
    frames(&mut h, 3);
    assert!(
        h.find_text("Select a text box, or a shape that holds text, to change its text.")
            .is_some()
    );
    assert!(
        h.find_text("Columns").is_some(),
        "settings still drawn, disabled"
    );
}

#[test]
fn a_layout_saved_before_a_panel_existed_loads_unchanged() {
    let mut h = harness(true);
    dock::set_open(&mut h.editor, Panel::Slides, false);
    let saved = serde_json::to_value(&h.editor.ui.dock.workspace).expect("serializes");
    let loaded = dock::load_workspace(Some(&saved));
    assert!(
        !loaded.layout.contains(&Panel::Slides),
        "kept the user's layout"
    );
    assert!(
        !loaded.layout.contains(&Panel::Text),
        "settings panels open on demand"
    );
}

#[test]
fn a_layout_naming_a_panel_this_build_does_not_know_still_loads() {
    let mut h = harness(true);
    dock::set_open(&mut h.editor, Panel::Slides, false);
    panels::open_at(&mut h.editor, Panel::Text, None);
    let shown = serde_json::to_value(&h.editor.ui.dock.workspace).expect("serializes");
    panels::close(&mut h.editor, Panel::Text);
    let hidden = serde_json::to_value(&h.editor.ui.dock.workspace).expect("serializes");
    for saved in [shown, hidden] {
        let text = saved.to_string().replace("\"text\"", "\"future_panel\"");
        let value: serde_json::Value = serde_json::from_str(&text).expect("still JSON");
        let loaded = dock::load_workspace(Some(&value));
        assert_eq!(loaded.validate(), Ok(()));
        assert!(
            !loaded.layout.contains(&Panel::Slides),
            "not reset to the default: {value}"
        );
        assert!(loaded.layout.contains(&Panel::Format));
    }
}
