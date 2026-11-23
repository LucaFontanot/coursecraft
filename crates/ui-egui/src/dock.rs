//! The editor's panels in a `craft_ui::docking` workspace (M30 step 1).
//!
//! Behind View › Docked Panels while it is being built: the slides panel, the
//! canvas, the timeline and the Inspector become dock panels the user can move,
//! stack, tab and float. The canvas is protected: it stays docked and open. The
//! fixed egui panels stay the default until this replaces them.
//!
//! Docking rather than `craft_ui::panes`: Storyline stacks panels (Triggers over
//! Slide Layers) and tabs them (Timeline, States, Notes), which panes don't do.

use craft_ui::docking::{
    Action, Column, DockArea, DockContent, DockStyle, Group, Layout, Node, PanelLimits,
    Permissions, Side, Workspace,
};
use craft_ui::layout::{SplitAxis, SplitSize};
use egui::{Rect, Ui, vec2};

use crate::Editor;

/// One dockable part of the editor window. Stable ids, never shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Panel {
    Slides,
    Canvas,
    Timeline,
    Inspector,
}

impl Panel {
    pub const ALL: [Panel; 4] = [
        Panel::Slides,
        Panel::Canvas,
        Panel::Timeline,
        Panel::Inspector,
    ];

    /// English tab label, looked up through `tl!` when drawn.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Panel::Slides => "Slides",
            Panel::Canvas => "Slide",
            Panel::Timeline => "Timeline",
            Panel::Inspector => "Inspector",
        }
    }

    #[must_use]
    pub fn permissions(self) -> Permissions {
        match self {
            // The canvas stays docked and open; panels may split beside it.
            Panel::Canvas => Permissions {
                accepts_splits: true,
                ..Permissions::PROTECTED
            },
            // Nothing reopens a closed panel yet, so none can close.
            Panel::Slides | Panel::Timeline | Panel::Inspector => Permissions {
                closable: false,
                ..Permissions::default()
            },
        }
    }

    fn limits(self) -> PanelLimits {
        let min = match self {
            Panel::Slides => vec2(180.0, 160.0),
            Panel::Canvas => vec2(320.0, 200.0),
            Panel::Timeline => vec2(320.0, crate::timeline::TIMELINE_PANEL_MIN_HEIGHT),
            Panel::Inspector => vec2(260.0, 200.0),
        };
        PanelLimits {
            min,
            ..PanelLimits::default()
        }
    }
}

/// Docking on or off, and the arrangement.
#[derive(Debug, Clone, PartialEq)]
pub struct DockState {
    /// View › Docked Panels.
    pub enabled: bool,
    pub workspace: Workspace<Panel>,
    /// Where each panel's body was drawn last frame.
    pub rects: Vec<(Panel, Rect)>,
}

impl Default for DockState {
    fn default() -> Self {
        Self {
            enabled: false,
            workspace: Workspace::new(default_layout()),
            rects: Vec::new(),
        }
    }
}

impl DockState {
    /// Where `panel`'s body was drawn last frame.
    #[must_use]
    pub fn rect(&self, panel: Panel) -> Option<Rect> {
        self.rects
            .iter()
            .find(|(p, _)| *p == panel)
            .map(|(_, r)| *r)
    }
}

fn group(panel: Panel) -> Group<Panel> {
    Group::new(vec![panel], 0)
}

/// Today's arrangement: slides at the start edge, the Inspector at the end
/// edge, the canvas over the timeline between them.
#[must_use]
pub fn default_layout() -> Layout<Panel> {
    let slides = Node::Dock {
        side: Side::Start,
        columns: vec![Column::new(vec![group(Panel::Slides)], 220.0)],
    };
    let inspector = Node::Dock {
        side: Side::End,
        columns: vec![Column::new(
            vec![group(Panel::Inspector)],
            crate::PROPERTIES_PANEL_WIDTH,
        )],
    };
    let centre = Node::Split {
        axis: SplitAxis::Vertical,
        size: SplitSize::FixedSecond(220.0),
        first: Box::new(Node::Tabs {
            panels: vec![Panel::Canvas],
            active: 0,
        }),
        second: Box::new(Node::Tabs {
            panels: vec![Panel::Timeline],
            active: 0,
        }),
    };
    Layout {
        root: Some(Node::Split {
            axis: SplitAxis::Horizontal,
            size: SplitSize::FixedFirst(220.0),
            first: Box::new(slides),
            second: Box::new(Node::Split {
                axis: SplitAxis::Horizontal,
                size: SplitSize::FixedSecond(crate::PROPERTIES_PANEL_WIDTH),
                first: Box::new(centre),
                second: Box::new(inspector),
            }),
        }),
        floating: Vec::new(),
    }
}

struct Content<'a> {
    editor: &'a mut Editor,
    rects: Vec<(Panel, Rect)>,
}

impl DockContent<Panel> for Content<'_> {
    fn body(&mut self, ui: &mut Ui, panel: &Panel) {
        self.rects.push((*panel, ui.max_rect()));
        match panel {
            Panel::Slides => self.editor.slides_body(ui),
            Panel::Canvas => self.editor.canvas_body(ui),
            Panel::Inspector => self.editor.inspector_body(ui),
            Panel::Timeline => {
                if self.editor.ui.master_view {
                    ui.add_space(8.0);
                    ui.label(tl!("Slide masters have no timeline."));
                } else {
                    self.editor.timeline_body(ui);
                }
            }
        }
    }

    fn logical_label(&self, panel: &Panel) -> String {
        tl!(panel.label()).to_owned()
    }

    fn permissions(&self, panel: &Panel) -> Permissions {
        panel.permissions()
    }

    fn limits(&self, panel: &Panel) -> PanelLimits {
        panel.limits()
    }
}

/// Draw the docked panels into `ui` (the space between the top and bottom bars)
/// and apply the user's layout changes.
pub fn show(editor: &mut Editor, ui: &mut Ui) {
    let style = DockStyle::from_ui(ui);
    let layout = editor.ui.dock.workspace.layout.clone();
    let mut content = Content {
        editor,
        rects: Vec::new(),
    };
    let output = DockArea::new(egui::Id::unique("coursecraft_dock")).show_content(
        ui,
        &layout,
        &style,
        &mut content,
    );
    let Content { editor, rects } = content;
    editor.ui.dock.rects = rects;
    apply(editor, output.actions);
    if let Some(err) = output.error {
        log::debug!("dock gesture refused: {err:?}");
    }
    record_timeline(editor, ui);
}

/// Apply layout changes through the workspace, all or none.
pub fn apply(editor: &mut Editor, actions: Vec<Action<Panel>>) {
    if actions.is_empty() {
        return;
    }
    if let Err(err) = editor
        .ui
        .dock
        .workspace
        .apply_all(actions, |p: &Panel| p.permissions())
    {
        log::debug!("dock change refused: {err:?}");
    }
}

/// Keep the timeline's rect and focus up to date, as its fixed panel does.
fn record_timeline(editor: &mut Editor, ui: &Ui) {
    let rect = editor.ui.dock.rect(Panel::Timeline);
    if editor.ui.master_view {
        editor.ui.timeline.panel_rect = None;
        editor.ui.timeline.tab_rects.clear();
        return;
    }
    editor.ui.timeline.panel_rect = rect;
    if let Some(rect) = rect {
        editor.ui.timeline.panel_height = rect.height();
        if ui.input(|i| i.pointer.primary_pressed())
            && let Some(pos) = ui.input(|i| i.pointer.interact_pos())
        {
            editor.ui.timeline.focused = rect.contains(pos);
        }
    }
}
