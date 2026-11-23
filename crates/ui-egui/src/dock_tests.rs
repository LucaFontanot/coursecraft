//! The docked panel workspace (M30 step 1): default arrangement, drawing, and
//! layout changes through the workspace.

use craft_ui::docking::{Action, Placement};

use crate::dock::{self, Panel};
use crate::test_harness::TestHarness;
use coursecraft_course::Course;

const HELLO_COURSE: &str = include_str!("../../../examples/hello-course/course.json");

fn docked() -> TestHarness {
    let course = Course::from_json(HELLO_COURSE).expect("hello course parses");
    let mut h = TestHarness::new_with_size(course, (1600.0, 900.0));
    h.editor.ui.dock.enabled = true;
    for _ in 0..3 {
        h.idle_frame();
    }
    h
}

#[test]
fn default_layout_is_valid_and_has_every_panel_once() {
    let state = dock::DockState::default();
    assert!(!state.enabled, "docking is opt-in while it is being built");
    assert_eq!(state.workspace.validate(), Ok(()));
    let panels: Vec<Panel> = state
        .workspace
        .layout
        .panels()
        .into_iter()
        .copied()
        .collect();
    for panel in Panel::ALL {
        assert_eq!(
            panels.iter().filter(|p| **p == panel).count(),
            1,
            "{panel:?} must be in the layout once"
        );
    }
}

#[test]
fn every_panel_is_drawn_where_today_s_panels_are() {
    let h = docked();
    let rect = |p| {
        h.editor
            .ui
            .dock
            .rect(p)
            .unwrap_or_else(|| panic!("{p:?} not drawn"))
    };
    let (slides, canvas, timeline, inspector) = (
        rect(Panel::Slides),
        rect(Panel::Canvas),
        rect(Panel::Timeline),
        rect(Panel::Inspector),
    );
    assert!(
        slides.max.x <= canvas.min.x + 1.0,
        "slides left of the canvas"
    );
    assert!(
        canvas.max.x <= inspector.min.x + 1.0,
        "Inspector right of the canvas"
    );
    assert!(
        canvas.max.y <= timeline.min.y + 1.0,
        "timeline under the canvas"
    );
    assert!(
        canvas.width() > 400.0 && canvas.height() > 200.0,
        "canvas has room: {canvas:?}"
    );
    assert_eq!(h.editor.ui.timeline.panel_rect, Some(timeline));
}

#[test]
fn panel_contents_still_draw_inside_the_dock() {
    let mut h = docked();
    // The Inspector's tabs and the timeline's header come from the panel bodies.
    assert!(h.find_text("Format").is_some(), "Inspector body");
    assert!(h.find_text("Timeline").is_some(), "timeline tab");
}

#[test]
fn panels_move_through_the_workspace() {
    let mut h = docked();
    dock::apply(
        &mut h.editor,
        vec![Action::Move {
            panel: Panel::Inspector,
            anchor: Panel::Slides,
            placement: Placement::Tab { before: None },
        }],
    );
    h.idle_frame();
    let location: Vec<Panel> = h
        .editor
        .ui
        .dock
        .workspace
        .layout
        .panels()
        .into_iter()
        .copied()
        .collect();
    assert!(location.contains(&Panel::Inspector));
    // Tabbed with Slides, only the active one of the two is drawn.
    let drawn: Vec<Panel> = h.editor.ui.dock.rects.iter().map(|(p, _)| *p).collect();
    assert!(drawn.contains(&Panel::Canvas));
    assert!(!(drawn.contains(&Panel::Slides) && drawn.contains(&Panel::Inspector)));
}

#[test]
fn the_canvas_stays_docked_and_open() {
    let mut h = docked();
    let before = h.editor.ui.dock.workspace.clone();
    dock::apply(
        &mut h.editor,
        vec![Action::Close {
            panel: Panel::Canvas,
        }],
    );
    dock::apply(
        &mut h.editor,
        vec![Action::Move {
            panel: Panel::Canvas,
            anchor: Panel::Slides,
            placement: Placement::Tab { before: None },
        }],
    );
    assert_eq!(h.editor.ui.dock.workspace, before);
}

#[test]
fn no_panel_can_be_closed_yet() {
    let mut h = docked();
    let before = h.editor.ui.dock.workspace.clone();
    for panel in [Panel::Slides, Panel::Timeline, Panel::Inspector] {
        dock::apply(&mut h.editor, vec![Action::Close { panel }]);
    }
    assert_eq!(
        h.editor.ui.dock.workspace, before,
        "nothing reopens a closed panel yet"
    );
}

#[test]
fn master_view_keeps_the_timeline_panel_but_empties_it() {
    let mut h = docked();
    h.editor.toggle_master_view();
    h.idle_frame();
    assert!(h.find_text("Slide masters have no timeline.").is_some());
    assert_eq!(h.editor.ui.timeline.panel_rect, None);
}
