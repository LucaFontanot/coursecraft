//! The ribbon: tab strip and the open tab's groups, drawn from [`spec`].
//!
//! M30 step 1 (docs/ui/implementation.md § 6): the ribbon is drawn beside the old
//! menu bar and toolbar, behind View › Ribbon. Every control comes from the spec;
//! one whose feature is missing is drawn disabled with its reason in the tooltip.

pub mod spec;
pub mod widgets;

#[cfg(test)]
mod spec_tests;

use egui::{Align2, FontId, Rect, Response, Sense, Ui, pos2, vec2};
use serde_json::json;

use crate::properties::InspectorTab;
use crate::{Editor, EditorRequest};
use spec::{Action, Group, Item, Kind, Menu, Tab, TabWhen, UiAction, UiMenu};
use widgets::{CAPTION_H, CONTROLS_H, SMALL_H};

/// Height of the tab strip.
pub const STRIP_H: f32 = 26.0;
/// Height of the whole ribbon when open.
pub const RIBBON_H: f32 = STRIP_H + CONTROLS_H + CAPTION_H + 8.0;

/// What the ribbon shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RibbonState {
    /// Drawn at all (View › Ribbon).
    pub visible: bool,
    /// The open tab's id.
    pub tab: &'static str,
    /// Only the tab strip is shown.
    pub collapsed: bool,
}

impl Default for RibbonState {
    fn default() -> Self {
        Self {
            visible: false,
            tab: "home",
            collapsed: false,
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

fn ui_available(editor: &Editor, action: UiAction) -> Result<(), &'static str> {
    let master = editor.ui.master_view;
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
    }
}

/// Draw the ribbon into `ui` (a top panel of [`height`]).
pub fn show(editor: &mut Editor, ui: &mut Ui, requests: &mut Vec<EditorRequest>) {
    if spec::tab(editor.ui.ribbon.tab).is_none() {
        editor.ui.ribbon.tab = "home";
    }
    tab_strip(editor, ui);
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
    let mut body = ui.new_child(egui::UiBuilder::new().max_rect(area));
    egui::ScrollArea::horizontal()
        .id_salt("ribbon_groups")
        .show(&mut body, |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
                show_tab(editor, ui, tab, requests);
            });
        });
}

fn tab_strip(editor: &mut Editor, ui: &mut Ui) {
    let p = coursecraft_render::palette(ui.ctx());
    let font = FontId::proportional(13.0);
    let top = ui.max_rect().min;
    let mut x = top.x + 4.0;
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
    for group in groups {
        show_group(editor, ui, group, blocked, requests);
    }
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

fn show_group(
    editor: &mut Editor,
    ui: &mut Ui,
    group: &'static Group,
    blocked: Option<&'static str>,
    requests: &mut Vec<EditorRequest>,
) {
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
    let x = r.max.x + 4.0;
    ui.painter().line_segment(
        [pos2(x, r.min.y + 4.0), pos2(x, r.max.y - 2.0)],
        egui::Stroke::new(1.0, p.border),
    );
    ui.add_space(8.0);
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
    let selected =
        item.action == Action::Ui(UiAction::FormatPainter) && editor.ui.format_painter.is_some();
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
    let menu_only = item.action == Action::Ui(UiAction::Arrange);
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
        Menu::Ui(UiMenu::Arrange) => {
            let mut action = None;
            crate::menu::arrange_menu_items(editor, ui, &mut action, requests);
            if let Some((id, params)) = action {
                editor.run(id, params);
            }
        }
    }
}
