//! The ribbon: tab strip and the open tab's groups, drawn from [`spec`].
//!
//! M30 step 1 (docs/ui/implementation.md § 6): the ribbon is drawn beside the old
//! menu bar and toolbar, behind View › Ribbon. Every control comes from the spec;
//! one whose feature is missing is drawn disabled with its reason in the tooltip.

pub mod spec;
pub mod widgets;

#[cfg(test)]
mod spec_tests;

use coursecraft_render::Icon;
use egui::{Align2, FontId, Rect, Response, Sense, Ui, pos2, vec2};
use serde_json::json;

use crate::properties::InspectorTab;
use crate::{Editor, EditorRequest};
use coursecraft_render::ThemePref;
use spec::{Action, Group, Item, Kind, Menu, Tab, TabWhen, UiAction, UiMenu};
use widgets::{CAPTION_H, CONTROLS_H, SMALL_H};

/// Height of the tab strip.
pub const STRIP_H: f32 = 26.0;
/// Height of the whole ribbon when open.
pub const RIBBON_H: f32 = STRIP_H + CONTROLS_H + CAPTION_H + 8.0;

/// What the ribbon shows.
#[derive(Debug, Clone, PartialEq)]
pub struct RibbonState {
    /// Drawn instead of the flat toolbar (Preferences › Interface › Ribbon).
    pub visible: bool,
    /// The open tab's id.
    pub tab: &'static str,
    /// Only the tab strip is shown.
    pub collapsed: bool,
    /// Each group's drawn width (group id, width), measured when it was last
    /// drawn whole; groups that don't fit shrink to one button, rightmost first.
    pub group_widths: Vec<(&'static str, f32)>,
}

impl Default for RibbonState {
    fn default() -> Self {
        Self {
            visible: true,
            tab: "home",
            collapsed: false,
            group_widths: Vec::new(),
        }
    }
}

/// Height the ribbon panel needs now.
#[must_use]
pub fn height(state: &RibbonState) -> f32 {
    if state.collapsed {
        STRIP_H + 4.0
    } else {
        RIBBON_H
    }
}

/// The tooltip of a disabled control.
#[must_use]
pub fn disabled_reason(action: Action) -> Option<String> {
    match action {
        Action::NotYet(reason) => Some(craft_i18n::fmt(
            tl!("Not available yet: {reason}"),
            &[("reason", tl!(reason))],
        )),
        Action::NotPlanned(reason) => Some(craft_i18n::fmt(
            tl!("Not planned: {reason}"),
            &[("reason", tl!(reason))],
        )),
        Action::Command(_) | Action::Ui(_) => None,
    }
}

/// Whether `action` can run now, with the reason when it cannot.
pub fn availability(
    editor: &Editor,
    action: Action,
    blocked: Option<&'static str>,
) -> Result<(), String> {
    if let Some(reason) = disabled_reason(action) {
        return Err(reason);
    }
    if let Some(reason) = blocked {
        return Err(tl!(reason).to_owned());
    }
    match action {
        Action::Command(id) => {
            if crate::menu::adapter(editor, id).is_some() {
                Ok(())
            } else {
                Err(tl!(crate::menu::adapter_reason(editor, id)).to_owned())
            }
        }
        Action::Ui(ui_action) => ui_available(editor, ui_action).map_err(|r| tl!(r).to_owned()),
        Action::NotYet(_) | Action::NotPlanned(_) => Ok(()),
    }
}

const MASTER: &str = "Not available in Master View";

const STATE: &str = "Not available inside a state";

fn ui_available(editor: &Editor, action: UiAction) -> Result<(), &'static str> {
    let master = editor.ui.master_view;
    let state = editor.ui.editing_state.is_some();
    match action {
        UiAction::Paste => crate::menu::clipboard_rules(editor).paste,
        UiAction::Cut => crate::menu::clipboard_rules(editor).cut,
        UiAction::Copy => crate::menu::clipboard_rules(editor).copy,
        UiAction::FormatPainter => {
            if editor.ui.editing_state.is_some() {
                Err(crate::STATE_LOCK_TOOLTIP)
            } else if master {
                Err(MASTER)
            } else if editor.ui.format_painter.is_some() || editor.can_arm_format_painter() {
                Ok(())
            } else {
                Err("Select an object to copy its format")
            }
        }
        UiAction::NewSlide | UiAction::NewScene | UiAction::Preview => {
            if master {
                Err(MASTER)
            } else {
                Ok(())
            }
        }
        UiAction::DuplicateSlide => {
            if master {
                Err(MASTER)
            } else if editor
                .session
                .course
                .slide(&editor.selection.slide)
                .is_some()
            {
                Ok(())
            } else {
                Err("Select a slide first")
            }
        }
        UiAction::Arrange => {
            if editor.selection.object.is_some() {
                Ok(())
            } else {
                Err("Select an object first")
            }
        }
        UiAction::Publish => {
            if editor.file_ops_available {
                Ok(())
            } else {
                Err(crate::menu::FILE_OPS_DISABLED_MSG)
            }
        }
        UiAction::NarrationFromText
        | UiAction::FindReplace
        | UiAction::Spelling
        | UiAction::FormatInspector
        | UiAction::Player => Ok(()),
        UiAction::SlideLayer | UiAction::Trigger | UiAction::Question | UiAction::Interaction => {
            if master {
                Err(MASTER)
            } else if state {
                Err(STATE)
            } else if editor
                .session
                .course
                .slide(&editor.selection.slide)
                .is_none()
            {
                Err("Select a slide first")
            } else {
                Ok(())
            }
        }
        UiAction::Table
        | UiAction::Button
        | UiAction::Audio
        | UiAction::Video
        | UiAction::Lottie
        | UiAction::WebObject
        | UiAction::InputText
        | UiAction::InputCheckbox
        | UiAction::InputRadio
        | UiAction::Slider
        | UiAction::Dial
        | UiAction::Marker
        | UiAction::ScrollPanel => {
            if state {
                Err(STATE)
            } else if master && !matches!(action, UiAction::Table | UiAction::Button) {
                Err(MASTER)
            } else {
                Ok(())
            }
        }
        UiAction::ResultsSlide | UiAction::Certificate | UiAction::CourseMap => {
            if master {
                Err(MASTER)
            } else {
                Ok(())
            }
        }
        UiAction::Picture
        | UiAction::Shape
        | UiAction::TextBox
        | UiAction::MediaLibrary
        | UiAction::RecordNarration
        | UiAction::SlideView
        | UiAction::SlideMaster
        | UiAction::ZoomFit
        | UiAction::ZoomIn
        | UiAction::ZoomOut
        | UiAction::Zoom100
        | UiAction::Rulers
        | UiAction::Grid
        | UiAction::Guides
        | UiAction::SnapToGrid
        | UiAction::SnapToGuides
        | UiAction::ThemeLight
        | UiAction::ThemeDark
        | UiAction::ThemeSystem
        | UiAction::ImportPptx
        | UiAction::ImportQuestions
        | UiAction::ImportDocument
        | UiAction::ImportOutline => Ok(()),
        UiAction::PanelSlides
        | UiAction::PanelTimeline
        | UiAction::PanelInspector
        | UiAction::ResetPanels => {
            if editor.ui.dock.enabled {
                Ok(())
            } else {
                Err("Turn on Docked Panels first")
            }
        }
    }
}

/// Run an enabled control's action.
fn run(editor: &mut Editor, action: Action, resp: &Response, requests: &mut Vec<EditorRequest>) {
    match action {
        Action::Command(id) => {
            if let Some(params) = crate::menu::adapter(editor, id) {
                editor.run(id, params);
            }
        }
        Action::Ui(ui_action) => run_ui(editor, ui_action, resp, requests),
        Action::NotYet(_) | Action::NotPlanned(_) => {}
    }
}

fn run_ui(
    editor: &mut Editor,
    action: UiAction,
    resp: &Response,
    requests: &mut Vec<EditorRequest>,
) {
    match action {
        UiAction::Paste => crate::menu::paste_now(editor, &resp.ctx),
        UiAction::Cut => crate::clipboard::cut(editor),
        UiAction::Copy => crate::clipboard::copy(editor),
        UiAction::FormatPainter => crate::toolbar::format_painter_clicked(editor, resp),
        UiAction::NewSlide => crate::toolbar::add_new_slide(editor),
        UiAction::NewScene => crate::toolbar::add_new_scene(editor),
        UiAction::DuplicateSlide => {
            let slide = editor.selection.slide.clone();
            editor.run("slide.duplicate", json!({ "slide": slide }));
        }
        UiAction::NarrationFromText => editor.open_speech_dialog(),
        UiAction::FindReplace => crate::find::open(editor),
        UiAction::Spelling => crate::spell_check::open(editor),
        UiAction::FormatInspector => editor.ui.inspector_tab = InspectorTab::Format,
        // Its menu opens on click.
        UiAction::Arrange => {}
        UiAction::Player => editor.ui.show_course_settings_window = true,
        UiAction::Preview => crate::toolbar::preview_slide(editor, requests),
        UiAction::Publish => editor.open_publish_dialog(),
        UiAction::SlideLayer => crate::toolbar::add_new_layer(editor),
        UiAction::Picture => crate::toolbar::insert_image(editor, requests),
        UiAction::MediaLibrary => editor.ui.show_media_library = true,
        UiAction::Shape => editor.ui.show_shape_gallery_window = true,
        UiAction::Video => crate::toolbar::insert_video(editor, requests),
        UiAction::Audio => crate::toolbar::insert_audio(editor, requests),
        UiAction::RecordNarration => editor.open_record_narration(),
        UiAction::WebObject => {
            editor.ui.web_error = None;
            editor.ui.show_web_window = true;
        }
        UiAction::Lottie => crate::toolbar::insert_lottie(editor, requests),
        UiAction::TextBox => crate::toolbar::insert_text_box(editor),
        UiAction::Table => crate::toolbar::insert_table(editor),
        UiAction::Button => crate::toolbar::insert_button(editor),
        UiAction::Slider => drop(crate::input_insert::insert_slider(editor)),
        UiAction::Dial => drop(crate::input_insert::insert_dial(editor)),
        UiAction::InputText => drop(crate::input_insert::insert_text_entry(editor)),
        UiAction::InputCheckbox => drop(crate::input_insert::insert_checkbox(editor)),
        UiAction::InputRadio => drop(crate::input_insert::insert_radio_group(editor)),
        UiAction::Marker => drop(crate::interaction_insert::insert_marker(editor)),
        UiAction::ScrollPanel => drop(crate::interaction_insert::insert_scroll_panel(editor)),
        UiAction::Trigger => crate::toolbar::insert_trigger(editor),
        UiAction::Interaction => editor.ui.show_interaction_gallery = true,
        // Its menu lists the kinds; a slide with a question opens its panel.
        UiAction::Question => editor.ui.bottom_tab = crate::BottomTab::Question,
        UiAction::ResultsSlide => crate::toolbar::insert_results_slide(editor),
        UiAction::Certificate => crate::toolbar::insert_certificate_slide(editor),
        UiAction::ImportPptx => requests.push(EditorRequest::PickPptx),
        UiAction::ImportQuestions => requests.push(EditorRequest::ImportQuestionsCsv),
        UiAction::ImportDocument => editor.open_document_dialog(),
        UiAction::ImportOutline => editor.open_outline_dialog(),
        UiAction::SlideMaster => editor.toggle_master_view(),
        UiAction::CourseMap => editor.ui.story_view = true,
        UiAction::SlideView => {
            editor.ui.story_view = false;
            if editor.ui.master_view {
                editor.toggle_master_view();
            }
        }
        UiAction::ZoomFit => editor.ui.zoom = crate::ZoomLevel::Fit,
        UiAction::ZoomIn => editor.ui.zoom = editor.ui.zoom.zoom_in(),
        UiAction::ZoomOut => editor.ui.zoom = editor.ui.zoom.zoom_out(),
        UiAction::Zoom100 => editor.ui.zoom = crate::ZoomLevel::Percent100,
        UiAction::Rulers => view_toggle(
            editor,
            "show_rulers",
            !editor.session.course.editor.show_rulers,
        ),
        UiAction::Grid => view_toggle(editor, "show_grid", !editor.session.course.editor.show_grid),
        UiAction::Guides => view_toggle(
            editor,
            "show_guides",
            !editor.session.course.editor.show_guides,
        ),
        UiAction::SnapToGrid => view_toggle(
            editor,
            "snap_to_grid",
            !editor.session.course.editor.snap_to_grid,
        ),
        UiAction::SnapToGuides => editor.ui.snap_to_guides = !editor.ui.snap_to_guides,
        UiAction::PanelSlides => toggle_panel(editor, crate::dock::Panel::Slides),
        UiAction::PanelTimeline => toggle_panel(editor, crate::dock::Panel::Timeline),
        UiAction::PanelInspector => toggle_panel(editor, crate::dock::Panel::Inspector),
        UiAction::ResetPanels => crate::dock::reset(editor),
        UiAction::ThemeLight => set_theme(editor, ThemePref::Light, requests),
        UiAction::ThemeDark => set_theme(editor, ThemePref::Dark, requests),
        UiAction::ThemeSystem => set_theme(editor, ThemePref::System, requests),
    }
}

fn view_toggle(editor: &mut Editor, key: &str, on: bool) {
    editor.run("view.set", json!({ key: on }));
}

fn toggle_panel(editor: &mut Editor, panel: crate::dock::Panel) {
    let open = crate::dock::is_open(editor, panel);
    crate::dock::set_open(editor, panel, !open);
}

fn set_theme(editor: &mut Editor, pref: ThemePref, requests: &mut Vec<EditorRequest>) {
    editor.theme_pref = pref;
    requests.push(EditorRequest::SetTheme(pref));
}

/// Whether a toggle-like control shows as on.
fn ui_selected(editor: &Editor, action: UiAction) -> bool {
    let view = &editor.session.course.editor;
    match action {
        UiAction::FormatPainter => editor.ui.format_painter.is_some(),
        UiAction::TextBox => editor.ui.placement_tool == Some(crate::PlacementTool::TextBox),
        UiAction::Button => editor.ui.placement_tool == Some(crate::PlacementTool::Button),
        UiAction::Shape => matches!(
            editor.ui.placement_tool,
            Some(crate::PlacementTool::Shape(_) | crate::PlacementTool::Draw(_))
        ),
        UiAction::CourseMap => editor.ui.story_view,
        UiAction::SlideView => !editor.ui.story_view && !editor.ui.master_view,
        UiAction::SlideMaster => editor.ui.master_view,
        UiAction::ZoomFit => editor.ui.zoom == crate::ZoomLevel::Fit,
        UiAction::Zoom100 => editor.ui.zoom == crate::ZoomLevel::Percent100,
        UiAction::Rulers => view.show_rulers,
        UiAction::Grid => view.show_grid,
        UiAction::Guides => view.show_guides,
        UiAction::SnapToGrid => view.snap_to_grid,
        UiAction::SnapToGuides => editor.ui.snap_to_guides,
        UiAction::PanelSlides => {
            editor.ui.dock.enabled && crate::dock::is_open(editor, crate::dock::Panel::Slides)
        }
        UiAction::PanelTimeline => {
            editor.ui.dock.enabled && crate::dock::is_open(editor, crate::dock::Panel::Timeline)
        }
        UiAction::PanelInspector => {
            editor.ui.dock.enabled && crate::dock::is_open(editor, crate::dock::Panel::Inspector)
        }
        UiAction::ThemeLight => editor.theme_pref == ThemePref::Light,
        UiAction::ThemeDark => editor.theme_pref == ThemePref::Dark,
        UiAction::ThemeSystem => editor.theme_pref == ThemePref::System,
        _ => false,
    }
}

/// Draw the ribbon into `ui` (a top panel of [`height`]).
pub fn show(editor: &mut Editor, ui: &mut Ui, requests: &mut Vec<EditorRequest>) {
    if spec::tab(editor.ui.ribbon.tab).is_none() {
        editor.ui.ribbon.tab = "home";
    }
    if ui.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::F1)) {
        editor.ui.ribbon.collapsed = !editor.ui.ribbon.collapsed;
    }
    tab_strip(editor, ui, requests);
    if editor.ui.ribbon.collapsed {
        return;
    }
    let Some(tab) = spec::tab(editor.ui.ribbon.tab) else {
        return;
    };
    let area = Rect::from_min_max(
        pos2(ui.max_rect().min.x, ui.max_rect().min.y + STRIP_H + 2.0),
        ui.max_rect().max,
    );
    let available = area.width() - 4.0;
    let mut body = ui.new_child(egui::UiBuilder::new().max_rect(area));
    egui::ScrollArea::horizontal()
        .id_salt("ribbon_groups")
        .show(&mut body, |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
                show_tab(editor, ui, tab, available, requests);
            });
        });
}

/// The quick access toolbar: Save, Undo, Redo, left of the tab strip.
/// Returns its right edge.
fn quick_access(editor: &mut Editor, ui: &mut Ui, requests: &mut Vec<EditorRequest>) -> f32 {
    let top = ui.max_rect().min;
    let rect = Rect::from_min_size(top, vec2(90.0, STRIP_H));
    let mut bar = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    bar.spacing_mut().item_spacing.x = 2.0;
    let save_tip = format!(
        "{} ({})",
        tl!("Save"),
        crate::menu::shortcut_label(ui.ctx(), "Ctrl+S")
    );
    let save_tip = if editor.file_ops_available {
        save_tip
    } else {
        tl!(crate::menu::FILE_OPS_DISABLED_MSG).to_owned()
    };
    if crate::widgets::icon_button_enabled(
        &mut bar,
        Icon::FloppyDisk,
        editor.file_ops_available,
        &save_tip,
    )
    .clicked()
    {
        requests.push(EditorRequest::Save);
    }
    for (id, icon, label, shortcut) in [
        ("edit.undo", Icon::ArrowCounterClockwise, "Undo", "Ctrl+Z"),
        ("edit.redo", Icon::ArrowClockwise, "Redo", "Ctrl+Y"),
    ] {
        let params = crate::menu::adapter(editor, id);
        let tip = format!(
            "{} ({})",
            tl!(label),
            crate::menu::shortcut_label(ui.ctx(), shortcut)
        );
        if crate::widgets::icon_button_enabled(&mut bar, icon, params.is_some(), &tip).clicked()
            && let Some(params) = params
        {
            editor.run(id, params);
        }
    }
    let right = bar.min_rect().max.x + 6.0;
    ui.painter().line_segment(
        [
            pos2(right - 3.0, top.y + 6.0),
            pos2(right - 3.0, top.y + STRIP_H - 6.0),
        ],
        egui::Stroke::new(1.0, coursecraft_render::palette(ui.ctx()).border),
    );
    right
}

fn tab_strip(editor: &mut Editor, ui: &mut Ui, requests: &mut Vec<EditorRequest>) {
    let p = coursecraft_render::palette(ui.ctx());
    let font = FontId::proportional(13.0);
    let top = ui.max_rect().min;
    let mut x = quick_access(editor, ui, requests) + 4.0;
    let mut tab_rect = |ui: &Ui, label: &str| {
        let w = ui
            .painter()
            .layout_no_wrap(label.to_owned(), font.clone(), p.text)
            .size()
            .x
            + 20.0;
        let r = Rect::from_min_size(pos2(x, top.y + 1.0), vec2(w, STRIP_H - 2.0));
        x += w + 2.0;
        r
    };

    // File: a full-window page (ribbon.md › File), not built yet.
    let file = tl!("File");
    let r = tab_rect(ui, file);
    let resp = ui.interact(r, ui.scope_id().with("ribbon_tab_file"), Sense::hover());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Tab, false, file));
    ui.painter().text(
        r.center(),
        Align2::CENTER_CENTER,
        file,
        font.clone(),
        p.text_disabled,
    );
    resp.on_hover_text(craft_i18n::fmt(
        tl!("Not available yet: {reason}"),
        &[("reason", tl!("the File page (use the File menu)"))],
    ));

    for tab in spec::TABS {
        let label = tl!(tab.label);
        let r = tab_rect(ui, label);
        let resp = ui.interact(
            r,
            ui.scope_id().with(("ribbon_tab", tab.id)),
            Sense::click(),
        );
        let active = editor.ui.ribbon.tab == tab.id;
        resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Tab, true, active, label));
        if resp.hovered() && !active {
            ui.painter().rect_filled(
                r.shrink(2.0),
                egui::CornerRadius::same(coursecraft_render::RADIUS_SM as u8),
                p.hover,
            );
        }
        let color = if active { p.text } else { p.text_secondary };
        ui.painter().text(
            r.center(),
            Align2::CENTER_CENTER,
            label,
            font.clone(),
            color,
        );
        if active {
            ui.painter().rect_filled(
                Rect::from_center_size(
                    pos2(r.center().x, r.max.y - 1.0),
                    vec2((r.width() - 16.0).max(12.0), 2.5),
                ),
                egui::CornerRadius::same(1),
                p.accent,
            );
        }
        if resp.double_clicked() || (resp.clicked() && active) {
            editor.ui.ribbon.collapsed = !editor.ui.ribbon.collapsed;
        } else if resp.clicked() {
            editor.ui.ribbon.tab = tab.id;
            editor.ui.ribbon.collapsed = false;
        }
    }
}

fn show_tab(
    editor: &mut Editor,
    ui: &mut Ui,
    tab: &'static Tab,
    available: f32,
    requests: &mut Vec<EditorRequest>,
) {
    let map = editor.ui.story_view;
    let groups = match tab.map_groups {
        Some(map_groups) if map => map_groups,
        _ => tab.groups,
    };
    if groups.is_empty() {
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(tl!(
                "This tab is being built. Its commands are in the menu bar for now."
            ))
            .color(coursecraft_render::palette(ui.ctx()).text_secondary),
        );
        return;
    }
    let blocked = (tab.when == TabWhen::SlideOpen && map).then_some("Open a slide first");
    let shrunk = shrunk_groups(&editor.ui.ribbon, groups, available);
    for (group, shrink) in groups.iter().zip(shrunk) {
        if shrink {
            shrunk_group(editor, ui, group, blocked, requests);
        } else {
            let width = show_group(editor, ui, group, blocked, requests);
            remember_width(&mut editor.ui.ribbon, group.id, width);
        }
    }
}

/// Width of a group drawn as one button.
const SHRUNK_W: f32 = 64.0;

/// Which groups to draw as one button so the rest fit in `available`,
/// rightmost first. A group not measured yet counts as a shrunk one.
fn shrunk_groups(state: &RibbonState, groups: &[Group], available: f32) -> Vec<bool> {
    let width = |g: &Group| {
        state
            .group_widths
            .iter()
            .find(|(id, _)| *id == g.id)
            .map_or(SHRUNK_W, |(_, w)| *w)
    };
    let mut shrunk = vec![false; groups.len()];
    let mut total: f32 = groups.iter().map(width).sum();
    for (i, group) in groups.iter().enumerate().rev() {
        if total <= available {
            break;
        }
        let w = width(group);
        if w > SHRUNK_W {
            shrunk[i] = true;
            total -= w - SHRUNK_W;
        }
    }
    shrunk
}

fn remember_width(state: &mut RibbonState, id: &'static str, width: f32) {
    if !width.is_finite() {
        return;
    }
    match state.group_widths.iter_mut().find(|(g, _)| *g == id) {
        Some(entry) => entry.1 = width,
        None => state.group_widths.push((id, width)),
    }
}

/// A group that doesn't fit: one large button with its caption, opening the
/// whole group in a popup.
fn shrunk_group(
    editor: &mut Editor,
    ui: &mut Ui,
    group: &'static Group,
    blocked: Option<&'static str>,
    requests: &mut Vec<EditorRequest>,
) {
    let icon = group.items.first().map_or(Icon::DotsThree, |i| i.icon);
    let caption = tl!(group.label);
    let inner = ui.vertical(|ui| {
        ui.set_width(SHRUNK_W - 8.0);
        let (main, arrow) = widgets::large(ui, icon, caption, true, true, false);
        let main = main.on_hover_text(caption);
        if let Some(arrow) = arrow {
            widgets::split_popup(&main, &arrow, true).show(|ui| {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
                    show_group(editor, ui, group, blocked, requests);
                });
            });
        }
    });
    separator(ui, inner.response.rect);
}

fn separator(ui: &mut Ui, r: Rect) {
    let p = coursecraft_render::palette(ui.ctx());
    let x = r.max.x + 4.0;
    ui.painter().line_segment(
        [pos2(x, r.min.y + 4.0), pos2(x, r.max.y - 2.0)],
        egui::Stroke::new(1.0, p.border),
    );
    ui.add_space(8.0);
}

/// How a run of items is laid out.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Run {
    Large,
    /// Small buttons in columns of three.
    Columns,
    /// Toggles and combo fields in rows.
    Rows,
}

fn run_of(kind: Kind) -> Run {
    match kind {
        Kind::Large | Kind::LargeSplit => Run::Large,
        Kind::Small | Kind::SmallSplit => Run::Columns,
        Kind::Toggle | Kind::Combo { .. } => Run::Rows,
    }
}

/// Draw a group whole; returns the width it took, separator included.
fn show_group(
    editor: &mut Editor,
    ui: &mut Ui,
    group: &'static Group,
    blocked: Option<&'static str>,
    requests: &mut Vec<EditorRequest>,
) -> f32 {
    let p = coursecraft_render::palette(ui.ctx());
    let caption = tl!(group.label);
    let caption_font = FontId::proportional(11.0);
    let caption_w = ui
        .painter()
        .layout_no_wrap(caption.to_owned(), caption_font.clone(), p.text)
        .size()
        .x
        + if group.launcher.is_some() { 22.0 } else { 8.0 };
    let inner = ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
        let controls = ui.horizontal_top(|ui| {
            ui.set_min_height(CONTROLS_H);
            ui.set_min_width(caption_w);
            let mut rest = group.items;
            while let Some(first) = rest.first() {
                let kind = run_of(first.kind);
                let len = rest.iter().take_while(|i| run_of(i.kind) == kind).count();
                let (run, tail) = rest.split_at(len);
                rest = tail;
                match kind {
                    Run::Large => {
                        for item in run {
                            control(editor, ui, item, blocked, requests);
                        }
                    }
                    Run::Columns => {
                        for column in run.chunks(3) {
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                                for item in column {
                                    control(editor, ui, item, blocked, requests);
                                }
                            });
                        }
                    }
                    Run::Rows => {
                        let rows = if run.len() <= 2 {
                            1
                        } else if run.len() <= 6 {
                            2
                        } else {
                            3
                        };
                        let per_row = run.len().div_ceil(rows);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing = vec2(1.0, 0.0);
                            for row in run.chunks(per_row.max(1)) {
                                ui.horizontal(|ui| {
                                    ui.set_height(SMALL_H);
                                    for item in row {
                                        control(editor, ui, item, blocked, requests);
                                    }
                                });
                            }
                        });
                    }
                }
            }
        });
        let width = controls.response.rect.width().max(caption_w);
        let (rect, _) = ui.allocate_exact_size(vec2(width, CAPTION_H), Sense::hover());
        let text_rect = if group.launcher.is_some() {
            Rect::from_min_max(rect.min, pos2(rect.max.x - 16.0, rect.max.y))
        } else {
            rect
        };
        ui.painter().text(
            text_rect.center(),
            Align2::CENTER_CENTER,
            caption,
            caption_font,
            p.text_secondary,
        );
        if let Some(action) = group.launcher {
            let r =
                Rect::from_center_size(pos2(rect.max.x - 8.0, rect.center().y), vec2(14.0, 14.0));
            let name = craft_i18n::fmt(tl!("{group} settings"), &[("group", caption)]);
            let state = availability(editor, action, blocked);
            let resp = widgets::launcher(ui, r, &name, state.is_ok());
            let resp = match &state {
                Ok(()) => resp.on_hover_text(&name),
                Err(reason) => resp.on_hover_text(reason),
            };
            if state.is_ok() && resp.clicked() {
                run(editor, action, &resp, requests);
            }
        }
    });
    // Separator between groups.
    let r = inner.response.rect;
    separator(ui, r);
    r.width() + 8.0 + ui.spacing().item_spacing.x
}

fn tooltip(item: &Item, state: &Result<(), String>) -> String {
    match state {
        Err(reason) => format!("{}\n{reason}", tl!(item.label)),
        Ok(()) if item.help.is_empty() => tl!(item.label).to_owned(),
        Ok(()) => format!("{}\n{}", tl!(item.label), tl!(item.help)),
    }
}

fn control(
    editor: &mut Editor,
    ui: &mut Ui,
    item: &'static Item,
    blocked: Option<&'static str>,
    requests: &mut Vec<EditorRequest>,
) {
    let state = availability(editor, item.action, blocked);
    let enabled = state.is_ok();
    let label = tl!(item.label);
    let selected = matches!(item.action, Action::Ui(a) if ui_selected(editor, a));
    let (main, arrow) = match item.kind {
        Kind::Large | Kind::LargeSplit => widgets::large(
            ui,
            item.icon,
            label,
            enabled,
            item.kind.is_split(),
            selected,
        ),
        Kind::Small | Kind::SmallSplit => widgets::small(
            ui,
            item.icon,
            label,
            enabled,
            selected,
            item.kind.is_split(),
        ),
        Kind::Toggle => (
            widgets::toggle(ui, item.icon, label, enabled, selected),
            None,
        ),
        Kind::Combo { width } => (widgets::combo_field(ui, label, "", width, enabled), None),
    };
    let tip = tooltip(item, &state);
    let main = main.on_hover_text(&tip);
    // These open their ▾ menu from the whole control.
    let menu_only = matches!(
        item.action,
        Action::Ui(UiAction::Arrange | UiAction::Shape | UiAction::Question)
    );
    if enabled && !menu_only && (main.clicked() || main.double_clicked()) {
        // A double click reaches the Format Painter (it keeps it on); other
        // actions run once, on the first click.
        if main.clicked() || item.action == Action::Ui(UiAction::FormatPainter) {
            run(editor, item.action, &main, requests);
        }
    }
    if let Some(arrow) = arrow {
        let arrow = arrow.on_hover_text(&tip);
        widgets::split_popup(&main, &arrow, menu_only && enabled).show(|ui| {
            menu(editor, ui, item, blocked, requests);
        });
    }
}

fn menu(
    editor: &mut Editor,
    ui: &mut Ui,
    item: &'static Item,
    blocked: Option<&'static str>,
    requests: &mut Vec<EditorRequest>,
) {
    match item.menu {
        Menu::None => {}
        Menu::Items(entries) => {
            for entry in entries {
                let state = availability(editor, entry.action, blocked);
                let resp = ui.add_enabled(state.is_ok(), egui::Button::new(tl!(entry.label)));
                let resp = match &state {
                    Ok(()) => resp,
                    Err(reason) => resp.on_disabled_hover_text(reason),
                };
                if resp.clicked() {
                    run(editor, entry.action, &resp, requests);
                    ui.close();
                }
            }
        }
        Menu::Ui(UiMenu::NewSlide) => crate::slides::show_new_slide_popup(ui, editor),
        Menu::Ui(UiMenu::Preview) => crate::toolbar::preview_menu(ui, editor, requests),
        Menu::Ui(UiMenu::Shapes) => shapes_menu(editor, ui),
        Menu::Ui(UiMenu::Questions) => {
            for kind in crate::question::ALL_QUESTION_KINDS {
                if ui
                    .button(crate::question::question_kind_name(kind))
                    .clicked()
                {
                    crate::toolbar::insert_question(editor, kind);
                    ui.close();
                }
            }
        }
        Menu::Ui(UiMenu::Arrange) => {
            let mut action = None;
            crate::menu::arrange_menu_items(editor, ui, &mut action, requests);
            if let Some((id, params)) = action {
                editor.run(id, params);
            }
        }
    }
}

/// Shape ▾: the shape gallery, as the old toolbar showed it.
fn shapes_menu(editor: &mut Editor, ui: &mut Ui) {
    let mut picked = None;
    let output = crate::shape_gallery::show_shape_gallery(
        ui,
        &mut editor.ui.shape_gallery_query,
        &mut |pick| picked = Some(pick),
    );
    editor.ui.shape_gallery_rects = output.icons;
    editor.ui.shape_gallery_draw_rects = output.draw_icons;
    editor.ui.shape_gallery_search_rect = output.search_rect;
    match picked {
        Some(crate::shape_gallery::GalleryPick::Shape(kind)) => {
            crate::toolbar::insert_shape(editor, kind);
            ui.close();
        }
        Some(crate::shape_gallery::GalleryPick::Draw(tool)) => {
            crate::toolbar::insert_draw(editor, tool);
            ui.close();
        }
        None => {}
    }
}
