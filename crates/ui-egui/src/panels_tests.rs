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
    for tab in spec::TABS
        .iter()
        .chain(spec::CONTEXT_TABS.iter().map(|c| &c.tab))
    {
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

#[test]
fn the_bottom_tabs_are_dock_panels_tabbed_with_the_timeline() {
    let h = harness(true);
    let group = group_of(&h, Panel::Timeline);
    let mut expected = Panel::BOTTOM.to_vec();
    expected.push(Panel::Notes);
    assert_eq!(group, expected, "one tab group: {group:?}");
    for panel in Panel::BOTTOM {
        assert_eq!(panels::family(panel), Family::Workspace, "{panel:?}");
    }
    assert!(
        !h.editor.ui.timeline.tab_rects.contains_key("Triggers"),
        "the timeline header has no tab strip of its own"
    );
}

#[test]
fn asking_for_a_bottom_tab_brings_its_panel_forward() {
    let mut h = harness(true);
    h.editor.ui.bottom_tab = crate::BottomTab::Triggers;
    frames(&mut h, 2);
    assert!(h.editor.ui.dock.rect(Panel::Triggers).is_some(), "in front");
    assert!(
        h.editor.ui.dock.rect(Panel::Timeline).is_none(),
        "same group"
    );
    dock::set_open(&mut h.editor, Panel::Variables, false);
    h.editor.ui.bottom_tab = crate::BottomTab::Variables;
    frames(&mut h, 2);
    assert!(dock::is_open(&h.editor, Panel::Variables), "reopened");
    assert!(h.editor.ui.dock.rect(Panel::Variables).is_some());
}

#[test]
fn in_master_view_the_bottom_panels_say_why_they_are_empty() {
    let mut h = harness(true);
    h.editor.ui.master_view = true;
    h.editor.ui.bottom_tab = crate::BottomTab::Triggers;
    frames(&mut h, 2);
    assert!(h.find_text("Slide masters have no triggers.").is_some());
}

#[test]
fn a_layout_saved_with_the_old_bottom_tab_strip_gains_the_new_panels() {
    let h = harness(true);
    let mut saved = serde_json::to_value(&h.editor.ui.dock.workspace).expect("serializes");
    let text = saved.to_string().replace(
        r#""timeline","states","triggers","variables","question""#,
        r#""timeline""#,
    );
    assert_ne!(text, saved.to_string(), "the group was found");
    saved = serde_json::from_str(&text).expect("still JSON");
    let loaded = dock::load_workspace(Some(&saved));
    for panel in Panel::BOTTOM {
        assert!(loaded.layout.contains(&panel), "{panel:?} added");
    }
}

#[test]
fn slide_layers_is_a_panel_of_its_own_beside_the_timeline() {
    let mut h = harness(true);
    frames(&mut h, 2);
    assert_eq!(group_of(&h, Panel::Layers), vec![Panel::Layers]);
    assert_eq!(panels::family(Panel::Layers), Family::Workspace);
    let layers = h.editor.ui.dock.rect(Panel::Layers).expect("drawn");
    let timeline = h.editor.ui.dock.rect(Panel::Timeline).expect("drawn");
    assert!(
        layers.min.x >= timeline.max.x,
        "to the right of the timeline"
    );
    // Closed, the timeline does not draw its own copy.
    dock::set_open(&mut h.editor, Panel::Layers, false);
    frames(&mut h, 2);
    assert!(h.editor.ui.dock.rect(Panel::Layers).is_none());
}

#[test]
fn the_notes_panel_edits_the_slides_notes_and_the_slide_inspector_leaves_them_to_it() {
    let mut h = harness(true);
    dock::reveal(&mut h.editor, Panel::Notes);
    frames(&mut h, 2);
    let rect = h.editor.ui.dock.rect(Panel::Notes).expect("drawn");
    h.click_at(rect.center());
    h.type_text("Say hello");
    // Commit by clicking away, on the canvas.
    let canvas = h.editor.ui.dock.rect(Panel::Canvas).expect("canvas");
    h.click_at(canvas.left_top() + egui::vec2(4.0, 4.0));
    frames(&mut h, 2);
    let slide = h.editor.selection.slide.clone();
    let notes = h
        .editor
        .session
        .course
        .slide(&slide)
        .map(|s| s.notes.clone())
        .unwrap_or_default();
    assert!(notes.contains("Say hello"), "notes: {notes:?}");
    h.editor.ui.inspector_tab = crate::InspectorTab::Slide;
    frames(&mut h, 2);
    let inspector = h.editor.ui.dock.rect(Panel::SlideInspector).expect("drawn");
    assert!(
        h.find_text_in("Narrate…", inspector).is_none(),
        "the Slide inspector leaves the notes to the Notes panel"
    );
}

#[test]
fn a_layout_saved_before_layers_and_notes_gains_them() {
    let h = harness(true);
    let saved = serde_json::to_value(&h.editor.ui.dock.workspace).expect("serializes");
    let text = saved
        .to_string()
        .replace(r#","notes""#, "")
        .replace(r#""layers""#, r#""future_panel""#);
    let value: serde_json::Value = serde_json::from_str(&text).expect("still JSON");
    let loaded = dock::load_workspace(Some(&value));
    assert_eq!(loaded.validate(), Ok(()));
    assert!(loaded.layout.contains(&Panel::Notes), "Notes added");
    assert!(loaded.layout.contains(&Panel::Layers), "Layers added");
}

#[test]
fn view_show_launcher_opens_grid_and_guides_and_its_settings_undo() {
    let mut h = harness(true);
    let launcher = crate::ribbon::spec::tab("view")
        .and_then(|t| t.groups.iter().find(|g| g.id == "view.show"))
        .and_then(|g| g.launcher);
    assert_eq!(
        launcher,
        Some(crate::ribbon::spec::Action::Ui(
            crate::ribbon::spec::UiAction::PanelAt(Panel::GridGuides, Some(panels::Section::Grid))
        ))
    );
    panels::open_at(
        &mut h.editor,
        Panel::GridGuides,
        Some(panels::Section::Grid),
    );
    frames(&mut h, 3);
    assert!(dock::is_open(&h.editor, Panel::GridGuides));
    assert_eq!(panels::family(Panel::GridGuides), Family::Settings);
    let rect = h.editor.ui.dock.rect(Panel::GridGuides).expect("drawn");
    let show = h.find_text_in("Show grid", rect).expect("Show grid");
    h.click_at(show);
    frames(&mut h, 1);
    assert!(h.editor.session.course.editor.show_grid);
    let add = h
        .find_text_in("Add vertical guide", rect)
        .expect("Add vertical guide");
    h.click_at(add);
    frames(&mut h, 2);
    assert_eq!(h.editor.session.course.editor.guides.len(), 1);
    let remove = h.find_text_in("×", rect).expect("the guide's ×");
    h.click_at(remove);
    frames(&mut h, 2);
    assert!(h.editor.session.course.editor.guides.is_empty());
    h.editor.run("edit.undo", serde_json::json!({}));
    assert_eq!(h.editor.session.course.editor.guides.len(), 1, "undone");
}

fn shape_harness() -> TestHarness {
    let json = serde_json::json!({
        "schema_version": 1,
        "id": "panels-shape",
        "title": "Shape panels",
        "slide_size": { "w": 1000, "h": 600 },
        "scenes": [{
            "id": "sc1",
            "title": "Scene 1",
            "slides": [{
                "id": "sl1",
                "title": "Slide 1",
                "duration_ms": 5000,
                "objects": [{
                    "id": "sh1",
                    "kind": { "type": "shape", "shape": "rectangle", "fill": "#112233" },
                    "frame": { "x": 100, "y": 100, "w": 200, "h": 120 }
                }]
            }]
        }]
    });
    let course = Course::from_json(&json.to_string()).expect("valid course");
    let mut h = TestHarness::new_with_size(course, (1600.0, 900.0));
    h.editor.ui.dock.enabled = true;
    h.editor.ui.ribbon.visible = true;
    h.editor.selection = crate::Selection::new("sl1");
    h.editor.selection.object = Some("sh1".into());
    frames(&mut h, 3);
    h
}

fn shape(h: &TestHarness) -> coursecraft_course::Object {
    let slide = h.editor.session.course.slide("sl1").expect("slide");
    slide.object("sh1").expect("shape").0.clone()
}

#[test]
fn size_and_position_sets_the_frame_with_the_aspect_locked_and_flips() {
    let mut h = shape_harness();
    let spec = crate::ribbon::spec::tab("animations")
        .and_then(|t| t.groups.iter().find(|g| g.id == "animations.size"))
        .expect("group");
    let opens = crate::ribbon::spec::Action::Ui(crate::ribbon::spec::UiAction::PanelAt(
        Panel::SizePosition,
        Some(panels::Section::Size),
    ));
    assert_eq!(spec.launcher, Some(opens));
    assert_eq!(spec.items[0].action, opens);
    panels::open_at(
        &mut h.editor,
        Panel::SizePosition,
        Some(panels::Section::Size),
    );
    frames(&mut h, 3);
    let rect = h.editor.ui.dock.rect(Panel::SizePosition).expect("drawn");
    let lock = h.find_text_in("Lock aspect ratio", rect).expect("lock");
    h.click_at(lock);
    let width = h
        .find_text_containing("200")
        .filter(|p| rect.contains(*p))
        .expect("the width field");
    h.set_field(width, "400");
    frames(&mut h, 2);
    let frame = shape(&h).frame;
    assert!((frame.w - 400.0).abs() < 0.01, "{frame:?}");
    assert!((frame.h - 240.0).abs() < 0.01, "height follows: {frame:?}");
    let flip = h.find_text_in("Flip horizontal", rect).expect("flip");
    h.click_at(flip);
    frames(&mut h, 2);
    assert!(shape(&h).flip_h);
    let locked = h
        .find_text_in("Lock position and size", rect)
        .expect("lock position");
    h.click_at(locked);
    frames(&mut h, 2);
    assert!(shape(&h).locked);
}

#[test]
fn shape_style_opens_from_drawing_and_edits_the_shapes_outline() {
    let mut h = shape_harness();
    let drawing = crate::ribbon::spec::tab("home")
        .and_then(|t| t.groups.iter().find(|g| g.id == "drawing"))
        .expect("group");
    assert_eq!(
        drawing.launcher,
        Some(crate::ribbon::spec::Action::Ui(
            crate::ribbon::spec::UiAction::PanelAt(Panel::ShapeStyle, Some(panels::Section::Fill))
        ))
    );
    panels::open_at(
        &mut h.editor,
        Panel::ShapeStyle,
        Some(panels::Section::Fill),
    );
    frames(&mut h, 3);
    let rect = h.editor.ui.dock.rect(Panel::ShapeStyle).expect("drawn");
    for heading in ["Fill", "Outline", "Effects"] {
        assert!(h.find_text_in(heading, rect).is_some(), "{heading}");
    }
    assert!(!matches!(
        shape(&h).kind,
        coursecraft_course::ObjectKind::Shape {
            outline: Some(_),
            ..
        }
    ));
    // The Outline row's checkbox sits to the right of its label.
    let outline = h.find_text_in("Outline", rect).expect("Outline");
    let rows = h.find_texts("Outline");
    let label = rows
        .into_iter()
        .find(|p| rect.contains(*p) && p.y > outline.y + 1.0)
        .expect("the Outline row under its heading");
    let checkbox = egui::pos2(label.x + crate::properties::LABEL_COL_WIDTH + 4.0, label.y);
    h.click_at(checkbox);
    frames(&mut h, 2);
    assert!(
        matches!(
            shape(&h).kind,
            coursecraft_course::ObjectKind::Shape {
                outline: Some(_),
                ..
            }
        ),
        "outline switched on"
    );
}
