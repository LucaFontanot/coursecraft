//! The editor's panels in a `craft_ui::docking` workspace (M30 step 1).
//!
//! On by default (Preferences › Interface › Docked panels): the slides panel, the
//! canvas, the timeline and the Inspector are dock panels the user can move, stack,
//! tab, float, close and reopen (View › Panels). The canvas is protected: it stays
//! docked and open. The arrangement is saved in the app's preferences through
//! [`crate::EditorRequest::SaveLayout`]; the fixed egui panels remain when docking
//! is turned off.
//!
//! Docking rather than `craft_ui::panes`: Storyline stacks panels (Triggers over
//! Slide Layers) and tabs them (Timeline, States, Notes), which panes don't do.

use craft_ui::docking::{
    Action, Column, DockArea, DockContent, DockStyle, Group, Layout, Node, PanelLimits,
    Permissions, Side, Workspace,
};
use craft_ui::layout::{SplitAxis, SplitSize};
use egui::{Rect, Ui, vec2};
use serde::{Deserialize, Serialize};

use crate::{Editor, InspectorTab};

/// One dockable part of the editor window. Stable ids, saved in the preferences.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Panel {
    Slides,
    Canvas,
    Timeline,
    /// The four inspectors, each a panel of its own (they start as tabs of one
    /// group where the single Inspector used to be).
    Format,
    Animate,
    #[serde(rename = "slide_inspector")]
    SlideInspector,
    #[serde(rename = "course_inspector")]
    CourseInspector,
}

impl Panel {
    pub const ALL: [Panel; 7] = [
        Panel::Slides,
        Panel::Canvas,
        Panel::Timeline,
        Panel::Format,
        Panel::Animate,
        Panel::SlideInspector,
        Panel::CourseInspector,
    ];

    /// The inspector panels, in their tab order.
    pub const INSPECTORS: [Panel; 4] = [
        Panel::Format,
        Panel::Animate,
        Panel::SlideInspector,
        Panel::CourseInspector,
    ];

    /// The inspector this panel draws, if it is one.
    #[must_use]
    pub fn inspector(self) -> Option<InspectorTab> {
        match self {
            Panel::Format => Some(InspectorTab::Format),
            Panel::Animate => Some(InspectorTab::Animate),
            Panel::SlideInspector => Some(InspectorTab::Slide),
            Panel::CourseInspector => Some(InspectorTab::Course),
            Panel::Slides | Panel::Canvas | Panel::Timeline => None,
        }
    }

    /// The panel that draws `tab`.
    #[must_use]
    pub fn for_inspector(tab: InspectorTab) -> Panel {
        match tab {
            InspectorTab::Format => Panel::Format,
            InspectorTab::Animate => Panel::Animate,
            InspectorTab::Slide => Panel::SlideInspector,
            InspectorTab::Course => Panel::CourseInspector,
        }
    }

    /// English tab label, looked up through `tl!` when drawn.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Panel::Slides => "Slides",
            Panel::Canvas => "Canvas",
            Panel::Timeline => "Timeline",
            Panel::Format => "Format",
            Panel::Animate => "Animate",
            Panel::SlideInspector => "Slide",
            Panel::CourseInspector => "Course",
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
            // View › Panels reopens them.
            Panel::Slides
            | Panel::Timeline
            | Panel::Format
            | Panel::Animate
            | Panel::SlideInspector
            | Panel::CourseInspector => Permissions::default(),
        }
    }

    fn limits(self) -> PanelLimits {
        let min = match self {
            Panel::Slides => vec2(180.0, 160.0),
            Panel::Canvas => vec2(320.0, 200.0),
            Panel::Timeline => vec2(320.0, crate::timeline::TIMELINE_PANEL_MIN_HEIGHT),
            Panel::Format | Panel::Animate | Panel::SlideInspector | Panel::CourseInspector => {
                vec2(260.0, 200.0)
            }
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
    /// Preferences › Interface › Docked panels (also View › Docked Panels).
    pub enabled: bool,
    pub workspace: Workspace<Panel>,
    /// Where each panel's body was drawn last frame.
    pub rects: Vec<(Panel, Rect)>,
    /// The inspector last brought forward: when `ui.inspector_tab` moves away
    /// from it (a ribbon button, a new selection), that panel is opened and
    /// activated.
    pub shown_inspector: InspectorTab,
}

impl Default for DockState {
    fn default() -> Self {
        Self {
            enabled: true,
            workspace: Workspace::new(default_layout()),
            rects: Vec::new(),
            shown_inspector: InspectorTab::default(),
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

    /// Where the first drawn inspector panel was drawn last frame.
    #[must_use]
    pub fn inspector_rect(&self) -> Option<Rect> {
        self.rects
            .iter()
            .find(|(p, _)| p.inspector().is_some())
            .map(|(_, r)| *r)
    }
}

/// The ribbon and docking settings and the panel arrangement, as the app saves them.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutPrefs {
    /// The ribbon replaces the flat toolbar.
    pub ribbon: bool,
    /// The panels are docked (else the fixed panels).
    pub docked: bool,
    /// The saved `craft_ui::docking::Workspace`; `None` for the default.
    pub workspace: Option<serde_json::Value>,
}

impl Default for LayoutPrefs {
    fn default() -> Self {
        Self {
            ribbon: true,
            docked: true,
            workspace: None,
        }
    }
}

/// A saved workspace fitted to today's panels, or the default when it is
/// missing, does not parse or does not validate.
#[must_use]
pub fn load_workspace(saved: Option<&serde_json::Value>) -> Workspace<Panel> {
    let default = Workspace::new(default_layout());
    let Some(value) = saved else {
        return default;
    };
    match serde_json::from_value::<Workspace<Panel>>(value.clone()) {
        Ok(mut workspace) if workspace.version <= Workspace::<Panel>::VERSION => {
            workspace.reconcile(|_| true, |p: &Panel| p.permissions(), &default);
            workspace
        }
        Ok(_) | Err(_) => {
            log::warn!("saved panel layout ignored: it does not load in this version");
            default
        }
    }
}

/// Whether `panel` is shown in the workspace.
#[must_use]
pub fn is_open(editor: &Editor, panel: Panel) -> bool {
    editor.ui.dock.workspace.layout.contains(&panel)
}

/// Show or hide one panel (View › Panels). Shown panels come back where they
/// were closed; the canvas can't be hidden.
pub fn set_open(editor: &mut Editor, panel: Panel, open: bool) {
    let fallback = Some(Panel::Canvas).filter(|p| *p != panel);
    if let Err(err) = editor
        .ui
        .dock
        .workspace
        .set_visible(panel, open, fallback, |p: &Panel| p.permissions())
    {
        log::debug!("panel {panel:?} not changed: {err:?}");
    }
}

/// View › Reset Panel Layout: the default arrangement, every panel open.
pub fn reset(editor: &mut Editor) {
    editor.ui.dock.workspace = Workspace::new(default_layout());
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
            vec![Group::new(Panel::INSPECTORS.to_vec(), 0)],
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
            Panel::Format | Panel::Animate | Panel::SlideInspector | Panel::CourseInspector => {
                if let Some(tab) = panel.inspector() {
                    self.editor.inspector_body(ui, Some(tab));
                }
            }
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
    reveal_inspector(editor);
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
    follow_activated_inspector(editor, &output.actions);
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

/// Bring the inspector `ui.inspector_tab` names forward when it changed since
/// the last frame: opened if closed, then made the active tab of its group.
pub fn reveal_inspector(editor: &mut Editor) {
    let tab = editor.ui.inspector_tab;
    if editor.ui.dock.shown_inspector == tab {
        return;
    }
    editor.ui.dock.shown_inspector = tab;
    let panel = Panel::for_inspector(tab);
    if !is_open(editor, panel) {
        set_open(editor, panel, true);
    }
    apply(editor, vec![Action::Activate { panel }]);
}

/// A click on an inspector's dock tab makes it the current inspector, so the
/// ribbon and the selection follow the user's choice.
fn follow_activated_inspector(editor: &mut Editor, actions: &[Action<Panel>]) {
    for action in actions {
        if let Action::Activate { panel } = action
            && let Some(tab) = panel.inspector()
        {
            editor.ui.inspector_tab = tab;
            editor.ui.dock.shown_inspector = tab;
        }
    }
}
