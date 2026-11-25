//! Document tabs under the ribbon: the Course Map, then one tab per opened slide.
//!
//! The first tab is always the Course Map; a slide gets a tab when it is opened
//! in the editor and keeps it until its × is clicked or the slide is deleted.
//! Clicking a tab switches the centre between the map and that slide.

use coursecraft_render::Icon;
use egui::{RichText, Ui};

use crate::{Editor, Selection};

/// Height of the tab row.
pub const DOC_TABS_H: f32 = 28.0;

/// The opened slides, in the order they were opened.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DocTabs {
    pub open: Vec<String>,
}

/// "2.3", the slide's scene and place in it; `None` for a slide in no scene.
#[must_use]
pub fn slide_number(course: &coursecraft_course::Course, slide_id: &str) -> Option<String> {
    crate::selection::slide_layout(course)
        .iter()
        .enumerate()
        .find_map(|(scene, (_, slides))| {
            slides
                .iter()
                .position(|s| s == slide_id)
                .map(|i| format!("{}.{}", scene + 1, i + 1))
        })
}

/// A slide tab's text: "1.2 Title".
#[must_use]
pub fn slide_tab_label(course: &coursecraft_course::Course, slide_id: &str) -> String {
    let title = course
        .slide(slide_id)
        .map_or(slide_id, |s| s.title.as_str());
    match slide_number(course, slide_id) {
        Some(number) => format!("{number} {title}"),
        None => title.to_owned(),
    }
}

/// Keep the tabs in step with the course and the editor: a slide being edited
/// gets a tab, a deleted slide loses its tab.
pub fn sync(editor: &mut Editor) {
    let course = &editor.session.course;
    editor
        .ui
        .doc_tabs
        .open
        .retain(|id| course.slide(id).is_some());
    let current = &editor.selection.slide;
    if !editor.ui.story_view
        && !editor.ui.master_view
        && course.slide(current).is_some()
        && !editor.ui.doc_tabs.open.contains(current)
    {
        editor.ui.doc_tabs.open.push(current.clone());
    }
}

/// Show the Course Map in the centre.
pub fn open_map(editor: &mut Editor) {
    if editor.ui.master_view {
        editor.toggle_master_view();
    }
    editor.ui.story_view = true;
}

/// Edit `slide_id` in the centre.
pub fn open_slide(editor: &mut Editor, slide_id: &str) {
    if editor.ui.master_view {
        editor.toggle_master_view();
    }
    editor.ui.story_view = false;
    if editor.selection.slide != slide_id || editor.selection.is_multi() {
        editor.selection = Selection::new(slide_id);
    }
}

/// Close a slide's tab; closing the shown slide shows its neighbour, or the
/// Course Map when it was the last one.
pub fn close(editor: &mut Editor, slide_id: &str) {
    let tabs = &mut editor.ui.doc_tabs.open;
    let Some(index) = tabs.iter().position(|id| id == slide_id) else {
        return;
    };
    tabs.remove(index);
    let shown = !editor.ui.story_view && editor.selection.slide == slide_id;
    if !shown {
        return;
    }
    let next = tabs
        .get(index)
        .or_else(|| index.checked_sub(1).and_then(|i| tabs.get(i)))
        .cloned();
    match next {
        Some(id) => open_slide(editor, &id),
        None => open_map(editor),
    }
}

/// The tab row, drawn under the ribbon.
pub fn show(editor: &mut Editor, ui: &mut Ui) {
    sync(editor);
    let p = coursecraft_render::palette(ui.ctx());
    let mut open_map_now = false;
    let mut open_now: Option<String> = None;
    let mut close_now: Option<String> = None;
    egui::ScrollArea::horizontal()
        .id_salt("doc_tabs_scroll")
        .auto_shrink([false, true])
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                let map_active = editor.ui.story_view && !editor.ui.master_view;
                ui.label(coursecraft_render::icon(Icon::SquaresFour, 14.0));
                let map_text = RichText::new(tl!("Course Map"));
                if ui
                    .add(egui::Button::selectable(map_active, map_text))
                    .on_hover_text(tl!("Every scene and slide, and how they connect"))
                    .clicked()
                {
                    open_map_now = true;
                }
                let course = &editor.session.course;
                for id in &editor.ui.doc_tabs.open {
                    let active = !editor.ui.story_view
                        && !editor.ui.master_view
                        && editor.selection.slide == *id;
                    ui.separator();
                    let label = slide_tab_label(course, id);
                    let text = if active {
                        RichText::new(label).strong()
                    } else {
                        RichText::new(label)
                    };
                    if ui
                        .add(egui::Button::selectable(active, text).truncate())
                        .clicked()
                    {
                        open_now = Some(id.clone());
                    }
                    let tip = tl!("Close this slide's tab");
                    if crate::widgets::icon_button(ui, Icon::X, tip).clicked() {
                        close_now = Some(id.clone());
                    }
                }
            });
        });
    ui.painter().hline(
        ui.max_rect().x_range(),
        ui.max_rect().bottom(),
        egui::Stroke::new(1.0, p.border),
    );
    if let Some(id) = close_now {
        close(editor, &id);
    } else if let Some(id) = open_now {
        open_slide(editor, &id);
    } else if open_map_now {
        open_map(editor);
    }
}
