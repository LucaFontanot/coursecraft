//! Shared test harness for `coursecraft-ui-egui` headless UI tests.

use crate::*;
use coursecraft_course::Course;
use egui::epaint::{ClippedShape, Shape};
use egui::{
    Context, Event, Key, LayerId, Modifiers, Order, PointerButton, Pos2, RawInput, Rect, pos2, vec2,
};

pub(crate) fn text_center(t: &egui::epaint::TextShape) -> Pos2 {
    let r = t.visual_bounding_rect();
    if r.is_positive() && r.center().x.is_finite() && r.center().y.is_finite() {
        r.center()
    } else {
        t.galley.rect.translate(t.pos.to_vec2()).center()
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(crate) struct WidgetNode {
    pub id: egui::accesskit::NodeId,
    pub role: egui::accesskit::Role,
    pub label: Option<String>,
    pub bounds: Rect,
    pub disabled: bool,
    pub toggled: Option<egui::accesskit::Toggled>,
    pub layer: Option<LayerId>,
}

pub struct TestHarness {
    pub ctx: Context,
    pub editor: Editor,
    pub screen_rect: Rect,
    pub time: f64,
    pub(crate) last_widgets: Vec<WidgetNode>,
}

#[allow(dead_code)]
impl TestHarness {
    pub fn new(course: Course) -> Self {
        Self::new_with_size(course, (1280.0, 900.0))
    }

    pub fn new_with_size(course: Course, size: (f32, f32)) -> Self {
        Self::new_with_theme(course, size, coursecraft_render::ThemeMode::Dark)
    }

    pub fn new_with_theme(
        course: Course,
        (w, h): (f32, f32),
        mode: coursecraft_render::ThemeMode,
    ) -> Self {
        Self::new_with_config(course, (w, h), mode, 1.0, true)
    }

    pub fn new_with_config(
        course: Course,
        (w, h): (f32, f32),
        mode: coursecraft_render::ThemeMode,
        zoom_factor: f32,
        file_ops: bool,
    ) -> Self {
        let ctx = Context::default();
        ctx.enable_accesskit();
        let zoom = if zoom_factor.is_finite() && zoom_factor > 0.0 {
            zoom_factor
        } else {
            1.0
        };
        ctx.set_zoom_factor(zoom);
        coursecraft_render::apply_style(&ctx, mode);
        // Like the snapshot renderer: no animation, so a window is at its final size and
        // position after the warm-up frames.
        let mut style = (*ctx.global_style()).clone();
        style.animation_time = 0.0;
        ctx.set_global_style(style);
        let screen_rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(w / zoom, h / zoom));
        let mut editor = Editor::new(course);
        editor.set_theme_pref(match mode {
            coursecraft_render::ThemeMode::Dark => coursecraft_render::ThemePref::Dark,
            coursecraft_render::ThemeMode::Light => coursecraft_render::ThemePref::Light,
        });
        editor.set_file_ops_available(file_ops);
        Self {
            ctx,
            editor,
            screen_rect,
            time: 0.0,
            last_widgets: Vec::new(),
        }
    }

    fn next_raw_input(&mut self, events: Vec<Event>) -> RawInput {
        let ctx_time = self.ctx.input(|i| i.time);
        if ctx_time > self.time {
            self.time = ctx_time + 1.0 / 60.0;
        }
        let time = self.time;
        self.time += 1.0 / 60.0;
        RawInput {
            screen_rect: Some(self.screen_rect),
            time: Some(time),
            events,
            ..Default::default()
        }
    }

    /// Advance the virtual clock monotonically by `secs` seconds.
    pub fn advance_time(&mut self, secs: f64) {
        self.time += secs.max(0.0);
    }

    pub fn frame(&mut self, events: Vec<Event>) -> Vec<EditorRequest> {
        let raw_input = self.next_raw_input(events);
        let mut requests = Vec::new();
        let output = self.ctx.run_ui(raw_input, |ui| {
            requests = self.editor.show(ui);
        });
        self.last_widgets = extract_widget_nodes(output.platform_output.accesskit_update.as_ref());
        output.drop_without_applying_deltas();
        requests
    }

    /// Run a frame and return every clipped shape painted during it.
    pub fn frame_shapes(&mut self, events: Vec<Event>) -> Vec<ClippedShape> {
        let raw_input = self.next_raw_input(events);
        let output = self.ctx.run_ui(raw_input, |ui| {
            let _ = self.editor.show(ui);
        });
        self.last_widgets = extract_widget_nodes(output.platform_output.accesskit_update.as_ref());
        let shapes = output.shapes.clone();
        output.drop_without_applying_deltas();
        shapes
    }

    /// Run a frame with an explicit egui input `time` (seconds).
    pub fn frame_at_time(&mut self, time: f64) -> Vec<EditorRequest> {
        self.time = self.time.max(time);
        let raw_input = RawInput {
            screen_rect: Some(self.screen_rect),
            time: Some(time),
            ..Default::default()
        };
        let mut requests = Vec::new();
        let output = self.ctx.run_ui(raw_input, |ui| {
            requests = self.editor.show(ui);
        });
        self.last_widgets = extract_widget_nodes(output.platform_output.accesskit_update.as_ref());
        output.drop_without_applying_deltas();
        requests
    }

    /// Run an idle frame and return the centers of every painted text equal to
    /// `label` (searching every layer, including open popups).
    pub fn find_texts(&mut self, label: &str) -> Vec<Pos2> {
        fn walk(shape: &egui::epaint::Shape, label: &str, out: &mut Vec<Pos2>) {
            match shape {
                egui::epaint::Shape::Text(t) if t.galley.text() == label => {
                    out.push(text_center(t));
                }
                egui::epaint::Shape::Vec(v) => {
                    for shape in v {
                        walk(shape, label, out);
                    }
                }
                _ => {}
            }
        }
        let raw_input = self.next_raw_input(Vec::new());
        let output = self.ctx.run_ui(raw_input, |ui| {
            let _ = self.editor.show(ui);
        });
        let mut found = Vec::new();
        for clipped in &output.shapes {
            walk(&clipped.shape, label, &mut found);
        }
        output.drop_without_applying_deltas();
        found
    }

    /// Run an idle frame and return the center of the first painted text equal to
    /// `label` (searching every layer, including open popups).
    pub fn find_text(&mut self, label: &str) -> Option<Pos2> {
        fn walk(shape: &egui::epaint::Shape, label: &str) -> Option<Pos2> {
            match shape {
                egui::epaint::Shape::Text(t) if t.galley.text() == label => Some(text_center(t)),
                egui::epaint::Shape::Vec(v) => v.iter().find_map(|s| walk(s, label)),
                _ => None,
            }
        }
        let raw_input = self.next_raw_input(Vec::new());
        let output = self.ctx.run_ui(raw_input, |ui| {
            let _ = self.editor.show(ui);
        });
        let found = output.shapes.iter().find_map(|cs| walk(&cs.shape, label));
        output.drop_without_applying_deltas();
        found
    }

    /// Run an idle frame and return `(center, clip_rect)` of the first painted text equal to
    /// `label`, where `clip_rect` is the clip the text was painted with (so callers can tell
    /// "laid out" from "actually visible").
    pub fn find_text_clipped(&mut self, label: &str) -> Option<(Pos2, Rect)> {
        fn walk(shape: &egui::epaint::Shape, label: &str) -> Option<Pos2> {
            match shape {
                egui::epaint::Shape::Text(t) if t.galley.text() == label => Some(text_center(t)),
                egui::epaint::Shape::Vec(v) => v.iter().find_map(|s| walk(s, label)),
                _ => None,
            }
        }
        let raw_input = self.next_raw_input(Vec::new());
        let output = self.ctx.run_ui(raw_input, |ui| {
            let _ = self.editor.show(ui);
        });
        let found = output
            .shapes
            .iter()
            .find_map(|cs| walk(&cs.shape, label).map(|p| (p, cs.clip_rect)));
        output.drop_without_applying_deltas();
        found
    }

    /// Run an idle frame and return the screen-space center of the last painted text equal to
    /// `label` (useful when a modal dialog or popup overlays background content with the same text).
    pub fn find_text_last(&mut self, label: &str) -> Option<Pos2> {
        fn walk(shape: &egui::epaint::Shape, label: &str) -> Option<Pos2> {
            match shape {
                egui::epaint::Shape::Text(t) if t.galley.text() == label => Some(text_center(t)),
                egui::epaint::Shape::Vec(v) => v.iter().rev().find_map(|s| walk(s, label)),
                _ => None,
            }
        }
        let raw_input = self.next_raw_input(Vec::new());
        let output = self.ctx.run_ui(raw_input, |ui| {
            let _ = self.editor.show(ui);
        });
        let found = output.shapes.iter().rev().find_map(|cs| {
            let pos = walk(&cs.shape, label)?;
            cs.clip_rect.contains(pos).then_some(pos)
        });
        output.drop_without_applying_deltas();
        found
    }

    /// Run an idle frame and find the center of the first painted text within a specific bounding rect.
    pub fn find_text_in(&mut self, label: &str, in_rect: Rect) -> Option<Pos2> {
        fn walk(shape: &egui::epaint::Shape, label: &str, in_rect: Rect) -> Option<Pos2> {
            match shape {
                egui::epaint::Shape::Text(t) if t.galley.text() == label => {
                    let center = text_center(t);
                    if in_rect.contains(center) {
                        Some(center)
                    } else {
                        None
                    }
                }
                egui::epaint::Shape::Vec(v) => v.iter().find_map(|s| walk(s, label, in_rect)),
                _ => None,
            }
        }
        let raw_input = self.next_raw_input(Vec::new());
        let output = self.ctx.run_ui(raw_input, |ui| {
            let _ = self.editor.show(ui);
        });
        let found = output
            .shapes
            .iter()
            .find_map(|cs| walk(&cs.shape, label, in_rect));
        output.drop_without_applying_deltas();
        found
    }

    /// Run an idle frame and return the center of the first text containing `sub`.
    pub fn find_text_containing(&mut self, sub: &str) -> Option<Pos2> {
        fn walk(shape: &egui::epaint::Shape, sub: &str) -> Option<Pos2> {
            match shape {
                egui::epaint::Shape::Text(t) if t.galley.text().contains(sub) => {
                    Some(text_center(t))
                }
                egui::epaint::Shape::Vec(v) => v.iter().find_map(|s| walk(s, sub)),
                _ => None,
            }
        }
        let raw_input = self.next_raw_input(Vec::new());
        let output = self.ctx.run_ui(raw_input, |ui| {
            let _ = self.editor.show(ui);
        });
        let found = output.shapes.iter().find_map(|cs| walk(&cs.shape, sub));
        output.drop_without_applying_deltas();
        found
    }

    /// Run an idle frame and return the center of the first painted icon matching `icon`.
    pub fn find_icon(&mut self, icon: coursecraft_render::Icon) -> Option<Pos2> {
        let text = icon.codepoint().to_string();
        self.find_text(&text)
    }

    /// Run an idle frame and return the center of the last painted icon matching `icon`.
    pub fn find_icon_last(&mut self, icon: coursecraft_render::Icon) -> Option<Pos2> {
        let text = icon.codepoint().to_string();
        self.find_text_last(&text)
    }

    pub fn idle_frame(&mut self) -> Vec<EditorRequest> {
        self.frame(Vec::new())
    }

    /// Move the pointer to `pos` and run a frame so hover-only UI is laid out.
    pub fn hover_at(&mut self, pos: Pos2) {
        self.frame(vec![Event::PointerMoved(pos)]);
        self.idle_frame();
    }

    pub fn click_at(&mut self, pos: Pos2) {
        let _ = self.click_at_requests(pos);
    }

    /// Click at `pos` and return all [`EditorRequest`]s emitted across the pointer frames.
    pub fn click_at_requests(&mut self, pos: Pos2) -> Vec<EditorRequest> {
        let mut requests = self.frame(vec![Event::PointerMoved(pos)]);
        requests.extend(self.frame(vec![Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Default::default(),
        }]));
        requests.extend(self.frame(vec![Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Default::default(),
        }]));
        requests
    }

    /// Double-click at `pos` with a 50ms gap so egui registers `double_clicked()`.
    pub fn double_click_at(&mut self, pos: Pos2) {
        self.click_at(pos);
        self.advance_time(0.05);
        self.click_at(pos);
    }

    pub fn click_at_with(&mut self, pos: Pos2, modifiers: Modifiers) {
        self.frame(vec![Event::PointerMoved(pos)]);
        if modifiers != Modifiers::NONE {
            self.frame(vec![Event::ModifiersChanged(modifiers)]);
        }
        self.frame(vec![Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: true,
            modifiers,
        }]);
        self.frame(vec![Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: false,
            modifiers,
        }]);
        if modifiers != Modifiers::NONE {
            self.frame(vec![Event::ModifiersChanged(Modifiers::NONE)]);
        }
    }

    pub fn right_click_at(&mut self, pos: Pos2) {
        self.frame(vec![Event::PointerMoved(pos)]);
        self.frame(vec![Event::PointerButton {
            pos,
            button: PointerButton::Secondary,
            pressed: true,
            modifiers: Default::default(),
        }]);
        self.frame(vec![Event::PointerButton {
            pos,
            button: PointerButton::Secondary,
            pressed: false,
            modifiers: Default::default(),
        }]);
    }

    pub fn drag_by(&mut self, start: Pos2, delta: egui::Vec2, steps: usize, modifiers: Modifiers) {
        if modifiers != Modifiers::NONE {
            self.frame(vec![Event::ModifiersChanged(modifiers)]);
        }
        self.frame(vec![Event::PointerMoved(start)]);
        self.frame(vec![Event::PointerButton {
            pos: start,
            button: PointerButton::Primary,
            pressed: true,
            modifiers,
        }]);
        let steps = steps.max(1);
        for i in 1..=steps {
            let t = i as f32 / steps as f32;
            let current = start + delta * t;
            self.frame(vec![Event::PointerMoved(current)]);
        }
        let end_pos = start + delta;
        self.frame(vec![Event::PointerButton {
            pos: end_pos,
            button: PointerButton::Primary,
            pressed: false,
            modifiers,
        }]);
        if modifiers != Modifiers::NONE {
            self.frame(vec![Event::ModifiersChanged(Modifiers::NONE)]);
        }
    }

    pub fn press(&mut self, key: Key, modifiers: Modifiers) {
        if modifiers != Modifiers::NONE {
            self.frame(vec![Event::ModifiersChanged(modifiers)]);
        }
        self.frame(vec![Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }]);
        self.frame(vec![Event::Key {
            key,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers,
        }]);
        if modifiers != Modifiers::NONE {
            self.frame(vec![Event::ModifiersChanged(Modifiers::NONE)]);
        }
    }

    pub fn type_text(&mut self, text: &str) {
        self.frame(vec![Event::Text(text.to_owned())]);
    }

    /// Dispatch OS-style `Event::Copy`.
    pub fn copy(&mut self) {
        self.frame(vec![Event::Copy]);
    }

    /// Dispatch OS-style `Event::Cut`.
    pub fn cut(&mut self) {
        self.frame(vec![Event::Cut]);
    }

    /// Dispatch OS-style `Event::Paste` with `text`.
    pub fn paste_text(&mut self, text: &str) {
        self.frame(vec![Event::Paste(text.to_owned())]);
    }

    /// Click a text field / DragValue at `pos`, select all, type `text`, and press Enter.
    pub fn set_field(&mut self, pos: Pos2, text: &str) {
        self.click_at(pos);
        self.frame(vec![
            Event::Key {
                key: Key::A,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::COMMAND,
            },
            Event::Text(text.to_owned()),
            Event::Key {
                key: Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            },
        ]);
        self.idle_frame();
    }

    /// Click a multiline text edit at `pos`, select all, type `text`, and click away to commit on blur.
    pub fn set_multiline_field(&mut self, pos: Pos2, text: &str, click_away_pos: Pos2) {
        self.click_at(pos);
        self.idle_frame();
        self.press(Key::A, Modifiers::COMMAND);
        self.type_text(text);
        self.click_at(click_away_pos);
        self.idle_frame();
    }

    /// Open a combo box showing `combo_label` and select `item_label` in the popup.
    pub fn pick_combo(&mut self, combo_label: &str, item_label: &str) {
        let combo_pos = self
            .find_text_last(combo_label)
            .unwrap_or_else(|| panic!("combo '{combo_label}' not found"));
        self.click_at(combo_pos);
        self.idle_frame();
        let item_pos = self.find_text_last(item_label).unwrap_or_else(|| {
            panic!("item '{item_label}' not found in popup for '{combo_label}'")
        });
        self.click_at(item_pos);
        self.idle_frame();
    }

    /// Deliver a mouse wheel scroll event over `pos`. Negative `dy` scrolls downward.
    pub fn scroll_over(&mut self, pos: Pos2, dy: f32) {
        self.frame(vec![Event::PointerMoved(pos)]);
        self.frame(vec![Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: vec2(0.0, dy),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Wheel,
            modifiers: Modifiers::NONE,
        }]);
        for _ in 0..8 {
            self.idle_frame();
        }
    }

    /// Top edge of the bottom panel as laid out in the last frame.
    pub fn bottom_panel_top(&self) -> f32 {
        self.editor.ui.timeline.panel_rect.map_or(
            self.screen_rect.max.y - STATUS_BAR_HEIGHT - TIMELINE_PANEL_HEIGHT,
            |r| r.min.y,
        )
    }

    /// Total height of top panels (menu bar + toolbar).
    pub fn top_bars_height(&self) -> f32 {
        TOP_BAR_HEIGHT + crate::toolbar::TOOLBAR_HEIGHT
    }

    pub fn left_panel_rect(&self) -> Rect {
        Rect::from_min_max(
            pos2(0.0, self.top_bars_height()),
            pos2(SLIDES_PANEL_WIDTH, self.bottom_panel_top()),
        )
    }

    pub fn central_panel_rect(&self) -> Rect {
        Rect::from_min_max(
            pos2(SLIDES_PANEL_WIDTH, self.top_bars_height()),
            pos2(
                self.screen_rect.max.x - PROPERTIES_PANEL_WIDTH,
                self.bottom_panel_top(),
            ),
        )
    }

    pub fn right_panel_rect(&self) -> Rect {
        Rect::from_min_max(
            pos2(
                self.screen_rect.max.x - PROPERTIES_PANEL_WIDTH,
                self.top_bars_height(),
            ),
            pos2(self.screen_rect.max.x, self.bottom_panel_top()),
        )
    }
}

/// What one layout-checked frame found.
#[derive(Debug, Default)]
pub(crate) struct LayoutReport {
    /// One readable line per problem.
    pub findings: Vec<String>,
    /// Every non-blank text painted in the frame (to prove a window really was on screen).
    pub texts: Vec<String>,
    /// Every window (dialog) the checker saw, as a layer name and its rect.
    pub windows: Vec<(String, Rect)>,
}

impl LayoutReport {
    pub fn painted(&self, needle: &str) -> bool {
        self.texts.iter().any(|t| t.contains(needle))
    }
}

#[allow(dead_code)]
impl TestHarness {
    /// Run `warmup` idle frames, then one more frame whose painted shapes are checked for
    /// clipped, overlapping and off-window text. See [`check_layout`].
    pub fn layout_report(&mut self, warmup: usize) -> LayoutReport {
        for _ in 0..warmup {
            self.idle_frame();
        }
        let raw_input = self.next_raw_input(Vec::new());
        let screen = self.screen_rect;
        let mut layers: Vec<(LayerId, Vec<ClippedShape>)> = Vec::new();
        let mut regions = Vec::new();
        let mut windows = Vec::new();
        let output = self.ctx.run_ui(raw_input, |ui| {
            let _ = self.editor.show(ui);
            let ctx = ui.ctx().clone();
            regions = crate::content_region::content_regions(&ctx);
            layers = capture_layers(&ctx);
            windows = window_rects(&ctx, &layers);
        });
        let mut report = check_layout(&layers, screen, &regions, &windows);
        let menu_bar = Rect::from_min_size(screen.min, vec2(screen.width(), crate::TOP_BAR_HEIGHT));
        report
            .findings
            .extend(check_windows(&windows, screen, menu_bar));
        // The per-layer walk must see every text the final output has, or a layer was missed.
        let from_layers: usize = layers
            .iter()
            .map(|(_, shapes)| shapes.iter().map(|c| count_texts(&c.shape)).sum::<usize>())
            .sum();
        let from_output: usize = output.shapes.iter().map(|c| count_texts(&c.shape)).sum();
        if from_layers != from_output {
            report.findings.push(format!(
                "internal: layer walk saw {from_layers} texts, final output has {from_output}"
            ));
        }
        self.last_widgets = extract_widget_nodes(output.platform_output.accesskit_update.as_ref());
        output.drop_without_applying_deltas();
        report
    }

    /// Run `warmup` idle frames, then one audited frame checking layout (`UX-04`/`UX-14`/`UX-16`),
    /// contrast (`UX-06`), nested vertical scrolls (`UX-02`), and hit targets (`UX-05`).
    pub fn audit_surface_report(
        &mut self,
        warmup: usize,
        mode: coursecraft_render::ThemeMode,
    ) -> LayoutReport {
        for _ in 0..warmup {
            self.idle_frame();
        }
        let raw_input = self.next_raw_input(Vec::new());
        let screen = self.screen_rect;
        let mut layers: Vec<(LayerId, Vec<ClippedShape>)> = Vec::new();
        let mut regions = Vec::new();
        let mut windows = Vec::new();
        let mut scrolls = Vec::new();
        let output = self.ctx.run_ui(raw_input, |ui| {
            let _ = self.editor.show(ui);
            let ctx = ui.ctx().clone();
            regions = crate::content_region::content_regions(&ctx);
            layers = capture_layers(&ctx);
            windows = window_rects(&ctx, &layers);
            scrolls = crate::scroll_audit::recorded_scrolls(&ctx);
        });
        let mut report = check_layout(&layers, screen, &regions, &windows);
        let menu_bar = Rect::from_min_size(screen.min, vec2(screen.width(), crate::TOP_BAR_HEIGHT));
        let mut tagged: Vec<String> = report
            .findings
            .drain(..)
            .map(|f| {
                let rule = if f.starts_with("off-window:") {
                    "UX-16"
                } else {
                    "UX-04"
                };
                format!("[{rule}] {f}")
            })
            .collect();
        for f in check_windows(&windows, screen, menu_bar) {
            tagged.push(format!("[UX-14] {f}"));
        }
        let from_layers: usize = layers
            .iter()
            .map(|(_, shapes)| shapes.iter().map(|c| count_texts(&c.shape)).sum::<usize>())
            .sum();
        let from_output: usize = output.shapes.iter().map(|c| count_texts(&c.shape)).sum();
        if from_layers != from_output {
            tagged.push(format!(
                "[UX-04] internal: layer walk saw {from_layers} texts, final output has {from_output}"
            ));
        }
        self.last_widgets = extract_widget_nodes(output.platform_output.accesskit_update.as_ref());
        output.drop_without_applying_deltas();

        let palette = coursecraft_render::Palette::for_mode(mode);
        for f in check_contrast(&layers, &regions, &palette, &self.last_widgets) {
            tagged.push(format!("[UX-06] {f}"));
        }
        for f in check_nested_scrolls(&layers, &scrolls) {
            tagged.push(format!("[UX-02] {f}"));
        }
        for f in check_hit_targets(&self.last_widgets, vec2(24.0, 24.0)) {
            tagged.push(format!("[UX-05] {f}"));
        }
        report.findings = tagged;
        report
    }

    /// Check hit target sizes on the most recently captured `WidgetNode`s.
    pub fn check_hit_targets(&self, min_size: egui::Vec2) -> Vec<String> {
        check_hit_targets(&self.last_widgets, min_size)
    }

    /// Hover at `pos` for 1.0s and return any `Order::Tooltip` text painted.
    pub fn probe_tooltip(&mut self, pos: Pos2) -> Option<String> {
        self.hover_at(pos);
        self.advance_time(1.0);
        self.idle_frame();
        let raw_input = self.next_raw_input(Vec::new());
        let mut layers = Vec::new();
        let output = self.ctx.run_ui(raw_input, |ui| {
            let _ = self.editor.show(ui);
            layers = capture_layers(ui.ctx());
        });
        output.drop_without_applying_deltas();
        let mut texts = Vec::new();
        for (layer, shapes) in &layers {
            if layer.order == Order::Tooltip {
                let mut items = Vec::new();
                for c in shapes {
                    collect_texts(&c.shape, c.clip_rect, &mut items);
                }
                texts.extend(items.into_iter().map(|i| i.text));
            }
        }
        let joined = texts.join(" ").trim().to_owned();
        if joined.is_empty() {
            None
        } else {
            Some(joined)
        }
    }

    /// Check icon/short-label tooltips + accessible names and disabled control tooltips
    /// for widgets currently visible inside `screen_rect`.
    pub fn check_tooltips_and_disabled(&mut self) -> Vec<String> {
        let screen = self.screen_rect;
        let widgets: Vec<WidgetNode> = self
            .last_widgets
            .iter()
            .filter(|n| screen.contains(n.bounds.center()))
            .cloned()
            .collect();
        check_tooltips_and_disabled_with(&widgets, |pos| self.probe_tooltip(pos))
    }

    /// Open a popup via `open_fn`, check bounds (`UX-16`), scroll stability (`UX-03`),
    /// and verify `Escape` dismisses only the popup (`UX-13`).
    pub fn audit_popup(&mut self, open_fn: impl FnOnce(&mut Self)) -> Vec<String> {
        open_fn(self);
        self.idle_frame();
        self.idle_frame();
        let mut findings = Vec::new();
        let layers = self.capture_current_layers();
        let Some((popup_layer, r0)) = foreground_popup_rect(&layers) else {
            findings.push("popup-did-not-open: no Order::Foreground popup layer found".to_owned());
            return findings;
        };
        if r0.min.x < self.screen_rect.min.x - 1.0
            || r0.min.y < self.screen_rect.min.y - 1.0
            || r0.max.x > self.screen_rect.max.x + 1.0
            || r0.max.y > self.screen_rect.max.y + 1.0
        {
            findings.push(format!(
                "popup-off-screen: {} extends outside {}",
                fmt_rect(r0),
                fmt_rect(self.screen_rect)
            ));
        }
        let mut widths = vec![r0.width()];
        let mut scrollbar_xs = vec![foreground_scrollbar_x(&layers, popup_layer, r0)];
        for _ in 0..5 {
            self.scroll_over(r0.center(), -180.0);
            let step_layers = self.capture_current_layers();
            if let Some((l, r)) = foreground_popup_rect(&step_layers) {
                widths.push(r.width());
                scrollbar_xs.push(foreground_scrollbar_x(&step_layers, l, r));
            }
        }
        findings.extend(check_popup_stability(&widths, &scrollbar_xs));
        self.press(Key::Escape, Modifiers::NONE);
        self.idle_frame();
        let after_layers = self.capture_current_layers();
        if foreground_popup_rect(&after_layers).is_some() {
            findings.push("popup-esc-did-not-close: Escape left foreground popup open".to_owned());
        }
        findings
    }

    /// Open a combo box by its visible label and audit its popup (`UX-03`, `UX-13`, `UX-16`).
    pub fn audit_combo(&mut self, combo_label: &str) -> Vec<String> {
        let Some(pos) = self.find_text(combo_label) else {
            return vec![format!("combo-not-found: \"{combo_label}\"")];
        };
        self.audit_popup(|h| {
            h.click_at(pos);
        })
    }

    /// Check modal backdrop alpha (`UX-15`: `>= 0xCC` for preview, `>= 0x99` otherwise) and
    /// verify clicking outside the modal does not trigger editor requests.
    pub fn check_modals_and_backdrops(&mut self, is_preview: bool) -> Vec<String> {
        let layers = self.capture_current_layers();
        let min_alpha = if is_preview { 0xCC } else { 0x99 };
        let mut findings = check_backdrop(&layers, self.screen_rect, min_alpha);
        let probe_pos = pos2(20.0, (self.top_bars_height() - 10.0).max(4.0));
        let reqs = self.click_at_requests(probe_pos);
        if !reqs.is_empty() {
            findings.push(format!(
                "modal-click-through: clicking at ({:.0}, {:.0}) behind modal emitted {} request(s)",
                probe_pos.x,
                probe_pos.y,
                reqs.len()
            ));
        }
        findings
    }

    fn capture_current_layers(&mut self) -> Vec<(LayerId, Vec<ClippedShape>)> {
        let raw_input = self.next_raw_input(Vec::new());
        let mut layers = Vec::new();
        let output = self.ctx.run_ui(raw_input, |ui| {
            let _ = self.editor.show(ui);
            layers = capture_layers(ui.ctx());
        });
        self.last_widgets = extract_widget_nodes(output.platform_output.accesskit_update.as_ref());
        output.drop_without_applying_deltas();
        layers
    }
}

/// Every layer's painted shapes so far this pass. Only valid inside the `run_ui` closure: the
/// final output has already lost the layer ids.
pub(crate) fn capture_layers(ctx: &Context) -> Vec<(LayerId, Vec<ClippedShape>)> {
    let mut ids: Vec<LayerId> = ctx.memory(|m| m.layer_ids().collect());
    ids.push(LayerId::background());
    ids.sort_by_key(|id| (id.order, id.id.value()));
    ids.dedup();
    let mut layers = Vec::new();
    ctx.graphics(|g| {
        for id in ids {
            if let Some(list) = g.get(id) {
                layers.push((id, list.all_entries().cloned().collect()));
            }
        }
    });
    layers
}

fn count_texts(shape: &Shape) -> usize {
    match shape {
        Shape::Text(_) => 1,
        Shape::Vec(v) => v.iter().map(count_texts).sum(),
        _ => 0,
    }
}

struct BoxItem {
    rect: Rect,
    clip: Rect,
}

struct TextItem {
    text: String,
    /// Where the whole galley sits.
    full: Rect,
    /// The part of it a viewer can see.
    visible: Rect,
    clip: Rect,
}

fn is_invisible(t: &egui::epaint::TextShape) -> bool {
    if t.opacity_factor <= 0.0 {
        return true;
    }
    let colors_clear = t
        .galley
        .job
        .sections
        .iter()
        .all(|s| s.format.color == egui::Color32::TRANSPARENT);
    colors_clear && t.override_text_color.is_none_or(|c| c.a() == 0)
}

fn collect_boxes(shape: &Shape, clip: Rect, out: &mut Vec<BoxItem>) {
    match shape {
        Shape::Rect(r) => {
            let drawn = r.fill.a() > 0 || (r.stroke.width > 0.0 && r.stroke.color.a() > 0);
            // Small framed widgets only: panel and card backgrounds are meant to bleed.
            if drawn && r.rect.is_positive() && r.rect.height() < 120.0 && r.rect.width() < 600.0 {
                out.push(BoxItem { rect: r.rect, clip });
            }
        }
        Shape::Vec(v) => {
            for s in v {
                collect_boxes(s, clip, out);
            }
        }
        _ => {}
    }
}

fn collect_texts(shape: &Shape, clip: Rect, out: &mut Vec<TextItem>) {
    match shape {
        Shape::Text(t) => {
            let text = t.galley.text().to_owned();
            if text.trim().is_empty() || is_invisible(t) {
                return;
            }
            let full = t.galley.rect.translate(t.pos.to_vec2());
            let visible = full.intersect(clip);
            if !visible.is_positive() {
                return;
            }
            out.push(TextItem {
                text,
                full,
                visible,
                clip,
            });
        }
        Shape::Vec(v) => {
            for s in v {
                collect_texts(s, clip, out);
            }
        }
        _ => {}
    }
}

fn layer_name(id: LayerId) -> String {
    id.short_debug_format()
}

fn fmt_rect(r: Rect) -> String {
    format!(
        "[{:.0},{:.0} .. {:.0},{:.0}]",
        r.min.x, r.min.y, r.max.x, r.max.y
    )
}

fn shorten(text: &str) -> String {
    let one_line: String = text
        .chars()
        .map(|c| if c == '\n' { ' ' } else { c })
        .collect();
    if one_line.chars().count() > 40 {
        let cut: String = one_line.chars().take(40).collect();
        format!("{cut}...")
    } else {
        one_line
    }
}

/// The windows (dialogs) the checker knows about: every `Order::Middle` layer that painted
/// something, with its rect from egui's area memory.
pub(crate) fn window_rects(
    ctx: &Context,
    layers: &[(LayerId, Vec<ClippedShape>)],
) -> Vec<(LayerId, Rect)> {
    layers
        .iter()
        .filter(|(layer, shapes)| layer.order == Order::Middle && !shapes.is_empty())
        .filter_map(|(layer, _)| {
            ctx.memory(|m| m.area_rect(layer.id))
                .filter(|r| r.is_positive())
                .map(|r| (*layer, r))
        })
        .collect()
}

/// Windows must lie inside the screen and must not cover the menu bar.
pub(crate) fn check_windows(
    windows: &[(LayerId, Rect)],
    screen: Rect,
    menu_bar: Rect,
) -> Vec<String> {
    const TOLERANCE: f32 = 1.0;
    let mut findings = Vec::new();
    for (layer, rect) in windows {
        // Full-screen modal backdrops intentionally cover and dim the menu bar.
        if (rect.width() - screen.width()).abs() <= TOLERANCE
            && (rect.height() - screen.height()).abs() <= TOLERANCE
        {
            continue;
        }
        let name = layer_name(*layer);
        if rect.min.x < screen.min.x - TOLERANCE
            || rect.min.y < screen.min.y - TOLERANCE
            || rect.max.x > screen.max.x + TOLERANCE
            || rect.max.y > screen.max.y + TOLERANCE
        {
            findings.push(format!(
                "window-off-screen: {name} {} is outside the window {}",
                fmt_rect(*rect),
                fmt_rect(screen)
            ));
        }
        if rect.min.y < menu_bar.max.y - TOLERANCE && rect.min.x < menu_bar.max.x {
            findings.push(format!(
                "window-over-menu-bar: {name} {} overlaps the menu bar {}",
                fmt_rect(*rect),
                fmt_rect(menu_bar)
            ));
        }
    }
    findings
}

/// Look for editor-chrome text that is cut off, runs under other text, or leaves the window.
///
/// `layers` holds every layer's painted shapes (tooltips are skipped). Text whose visible centre
/// is inside a registered slide-content region of its own layer is left alone, because slide
/// content may overlap by design. Reports:
/// - **clipped:** the label extends more than 1 pt past the clip rect it was painted with, left
///   or right, so part of it is cut off;
/// - **overlap:** two labels of one layer whose visible rects intersect by more than 2 pt in
///   both axes;
/// - **off-window:** visible text beyond `screen`;
/// - **clipped-widget:** a button or other framed widget cut off at the right by its clip, or, inside a window in `windows`, cut off at the bottom or top by the window's own
///   edge (a body that scrolls inside the window clips by design and is left alone).
pub(crate) fn check_layout(
    layers: &[(LayerId, Vec<ClippedShape>)],
    screen: Rect,
    regions: &[(LayerId, Rect)],
    windows: &[(LayerId, Rect)],
) -> LayoutReport {
    const CLIP_TOLERANCE: f32 = 1.0;
    const OVERLAP_TOLERANCE: f32 = 2.0;
    let mut findings = Vec::new();
    let mut texts = Vec::new();
    for (layer, shapes) in layers {
        if layer.order == Order::Tooltip {
            continue;
        }
        let name = layer_name(*layer);
        let mut items = Vec::new();
        for clipped in shapes {
            collect_texts(&clipped.shape, clipped.clip_rect, &mut items);
        }
        let mut boxes = Vec::new();
        for clipped in shapes {
            collect_boxes(&clipped.shape, clipped.clip_rect, &mut boxes);
        }
        for b in &boxes {
            let visible = b.rect.intersect(b.clip);
            if !visible.is_positive()
                || regions
                    .iter()
                    .any(|(l, r)| l == layer && r.contains(visible.center()))
            {
                continue;
            }
            if b.rect.max.x - b.clip.max.x > 1.5 && visible.width() > 4.0 {
                findings.push(format!(
                    "clipped-widget: {name} box {} is cut off at the right by its clip {}",
                    fmt_rect(b.rect),
                    fmt_rect(b.clip)
                ));
            }
            // Top and bottom: a window's own edge. Anything clipped by a scroll viewport
            // inside the window ends well above the window's lower edge (the window margin
            // and the footer sit below it), so only a clip that reaches the window's edge
            // counts.
            if let Some((_, win)) = windows.iter().find(|(l, _)| l == layer) {
                let is_window_shadow = (b.rect.width() - win.width()).abs() < 1.0
                    && (b.rect.height() - win.height()).abs() < 1.0
                    && (b.rect.min.x - win.min.x).abs() < 1.0;
                if is_window_shadow {
                    continue;
                }
                if b.rect.max.y - b.clip.max.y > 1.5
                    && visible.height() > 4.0
                    && b.clip.max.y >= win.max.y - 4.0
                {
                    findings.push(format!(
                        "clipped-widget: {name} box {} is cut off at the bottom of its window {}",
                        fmt_rect(b.rect),
                        fmt_rect(*win)
                    ));
                }
                if b.clip.min.y - b.rect.min.y > 1.5
                    && visible.height() > 4.0
                    && b.clip.min.y <= win.min.y + 4.0
                {
                    findings.push(format!(
                        "clipped-widget: {name} box {} is cut off at the top of its window {}",
                        fmt_rect(b.rect),
                        fmt_rect(*win)
                    ));
                }
            }
        }
        texts.extend(items.iter().map(|item| item.text.clone()));
        items.retain(|item| {
            !regions
                .iter()
                .any(|(l, r)| l == layer && r.contains(item.visible.center()))
        });
        for item in &items {
            let over_right = item.full.max.x - item.clip.max.x;
            let over_left = item.clip.min.x - item.full.min.x;
            if over_right > CLIP_TOLERANCE || over_left > CLIP_TOLERANCE {
                findings.push(format!(
                    "clipped: {name} \"{}\" {} is cut off by its clip {}",
                    shorten(&item.text),
                    fmt_rect(item.full),
                    fmt_rect(item.clip)
                ));
            }
            let v = item.visible;
            if item.full.max.x > screen.max.x + CLIP_TOLERANCE
                || item.full.min.x < screen.min.x - CLIP_TOLERANCE
                || v.max.y > screen.max.y + CLIP_TOLERANCE
                || v.min.y < screen.min.y - CLIP_TOLERANCE
            {
                findings.push(format!(
                    "off-window: {name} \"{}\" {} is outside the window {}",
                    shorten(&item.text),
                    fmt_rect(item.full),
                    fmt_rect(screen)
                ));
            }
        }
        // Sweep by left edge so a busy layer stays cheap.
        let mut order: Vec<usize> = (0..items.len()).collect();
        order.sort_by(|a, b| items[*a].visible.min.x.total_cmp(&items[*b].visible.min.x));
        for (pos, &i) in order.iter().enumerate() {
            let a = &items[i];
            for &j in &order[pos + 1..] {
                let b = &items[j];
                if b.visible.min.x >= a.visible.max.x - OVERLAP_TOLERANCE {
                    break;
                }
                let both = a.visible.intersect(b.visible);
                if both.width() <= OVERLAP_TOLERANCE || both.height() <= OVERLAP_TOLERANCE {
                    continue;
                }
                // The same text drawn twice in one place (a faux-bold or a doubled label) is
                // not something a viewer can see.
                let same_spot = a.text == b.text
                    && (a.visible.min - b.visible.min).length() < 1.0
                    && (a.visible.max - b.visible.max).length() < 1.0;
                if same_spot {
                    continue;
                }
                findings.push(format!(
                    "overlap: {name} \"{}\" {} runs under \"{}\" {}",
                    shorten(&a.text),
                    fmt_rect(a.visible),
                    shorten(&b.text),
                    fmt_rect(b.visible)
                ));
            }
        }
    }
    LayoutReport {
        findings,
        texts,
        windows: windows
            .iter()
            .map(|(layer, rect)| (layer_name(*layer), *rect))
            .collect(),
    }
}

#[allow(dead_code)]
pub(crate) fn extract_widget_nodes(
    update: Option<&egui::accesskit::TreeUpdate>,
) -> Vec<WidgetNode> {
    let Some(update) = update else {
        return Vec::new();
    };
    update
        .nodes
        .iter()
        .filter_map(|(id, node)| {
            let b = node.bounds()?;
            let bounds = Rect::from_min_max(
                pos2(b.x0 as f32, b.y0 as f32),
                pos2(b.x1 as f32, b.y1 as f32),
            );
            Some(WidgetNode {
                id: *id,
                role: node.role(),
                label: node.label().map(ToOwned::to_owned),
                bounds,
                disabled: node.is_disabled(),
                toggled: node.toggled(),
                layer: None,
            })
        })
        .collect()
}

fn is_interactive_role(role: egui::accesskit::Role) -> bool {
    use egui::accesskit::Role;
    matches!(
        role,
        Role::Button
            | Role::CheckBox
            | Role::RadioButton
            | Role::Tab
            | Role::ComboBox
            | Role::MenuItem
            | Role::MenuItemCheckBox
            | Role::MenuItemRadio
            | Role::Switch
    )
}

fn is_icon_or_short_symbol(label: Option<&str>) -> bool {
    let Some(raw) = label else {
        return true;
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return true;
    }
    if matches!(trimmed, "OK" | "No" | "On") {
        return false;
    }
    if trimmed.chars().count() <= 2 {
        return true;
    }
    trimmed
        .chars()
        .all(|c| c.is_whitespace() || ('\u{E000}'..='\u{F8FF}').contains(&c))
}

#[allow(dead_code)]
pub(crate) fn check_hit_targets(nodes: &[WidgetNode], min_size: egui::Vec2) -> Vec<String> {
    const EPS: f32 = 0.05;
    let mut findings = Vec::new();
    for node in nodes {
        if !is_interactive_role(node.role) || !node.bounds.is_positive() {
            continue;
        }
        // Toolbar split-button carets are explicitly specified as >= 20x44 pt in UX-05.
        let is_toolbar_caret = node
            .label
            .as_deref()
            .is_some_and(|l| l.starts_with("More ") && l.ends_with(" options"))
            && node.bounds.width() + EPS >= 20.0
            && node.bounds.height() + EPS >= 44.0;
        if is_toolbar_caret {
            continue;
        }
        // egui::Window's built-in titlebar close button is hardcoded to 14x14 inside egui.
        if node.label.as_deref() == Some("Close window") {
            continue;
        }
        if node.bounds.width() + EPS < min_size.x || node.bounds.height() + EPS < min_size.y {
            findings.push(format!(
                "hit-target-too-small: {:?} \"{}\" {} is smaller than {}x{}",
                node.role,
                shorten(node.label.as_deref().unwrap_or("")),
                fmt_rect(node.bounds),
                min_size.x,
                min_size.y
            ));
        }
    }
    findings
}

#[allow(dead_code)]
pub(crate) fn check_tooltips_and_disabled_with(
    nodes: &[WidgetNode],
    mut probe_tooltip: impl FnMut(Pos2) -> Option<String>,
) -> Vec<String> {
    let mut findings = Vec::new();
    for node in nodes {
        if !is_interactive_role(node.role) || !node.bounds.is_positive() {
            continue;
        }
        let icon_or_short = node.role == egui::accesskit::Role::Button
            && is_icon_or_short_symbol(node.label.as_deref());
        let name = shorten(node.label.as_deref().unwrap_or(""));
        if icon_or_short {
            findings.push(format!(
                "missing-accessible-name: {:?} \"{}\" {}",
                node.role,
                name,
                fmt_rect(node.bounds)
            ));
        }
        if icon_or_short || node.disabled {
            let tip = probe_tooltip(node.bounds.center());
            let has_tip = tip
                .as_deref()
                .is_some_and(|t| t.trim().chars().count() >= 3);
            if node.disabled && !has_tip {
                findings.push(format!(
                    "missing-disabled-tooltip: {:?} \"{}\" {}",
                    node.role,
                    name,
                    fmt_rect(node.bounds)
                ));
            } else if icon_or_short && !has_tip {
                findings.push(format!(
                    "missing-tooltip: {:?} \"{}\" {}",
                    node.role,
                    name,
                    fmt_rect(node.bounds)
                ));
            }
        }
    }
    findings
}

fn blend_over(src: egui::Color32, dst: egui::Color32) -> egui::Color32 {
    if src.a() == 255 {
        return egui::Color32::from_rgb(src.r(), src.g(), src.b());
    }
    let a = src.a() as f32 / 255.0;
    let inv = 1.0 - a;
    let r = (src.r() as f32 + dst.r() as f32 * inv)
        .round()
        .clamp(0.0, 255.0) as u8;
    let g = (src.g() as f32 + dst.g() as f32 * inv)
        .round()
        .clamp(0.0, 255.0) as u8;
    let b = (src.b() as f32 + dst.b() as f32 * inv)
        .round()
        .clamp(0.0, 255.0) as u8;
    egui::Color32::from_rgb(r, g, b)
}

fn collect_contrast_shapes<'a>(
    shape: &'a Shape,
    clip: Rect,
    bg_rects: &mut Vec<(Rect, egui::Color32)>,
    texts: &mut Vec<(&'a egui::epaint::TextShape, Rect)>,
) {
    match shape {
        Shape::Vec(v) => {
            for s in v {
                collect_contrast_shapes(s, clip, bg_rects, texts);
            }
        }
        Shape::Rect(r) => {
            let visible = r.rect.intersect(clip);
            if r.fill.a() > 0 && visible.is_positive() {
                bg_rects.push((visible, r.fill));
            }
        }
        Shape::Text(t) => {
            texts.push((t, clip));
        }
        _ => {}
    }
}

fn is_phosphor_only(text: &str) -> bool {
    let trimmed = text.trim();
    !trimmed.is_empty()
        && trimmed
            .chars()
            .all(|c| c.is_whitespace() || ('\u{E000}'..='\u{F8FF}').contains(&c))
}

#[allow(dead_code)]
pub(crate) fn check_contrast(
    layers: &[(LayerId, Vec<ClippedShape>)],
    regions: &[(LayerId, Rect)],
    palette: &coursecraft_render::Palette,
    nodes: &[WidgetNode],
) -> Vec<String> {
    let mut findings = Vec::new();
    let mut bg_rects: Vec<(Rect, egui::Color32)> = Vec::new();

    for (layer, shapes) in layers {
        let name = layer_name(*layer);
        for clipped in shapes {
            let mut layer_texts = Vec::new();
            collect_contrast_shapes(
                &clipped.shape,
                clipped.clip_rect,
                &mut bg_rects,
                &mut layer_texts,
            );
            for (t, clip) in layer_texts {
                if is_invisible(t) {
                    continue;
                }
                let visible = t.visual_bounding_rect().intersect(clip);
                if !visible.is_positive() {
                    continue;
                }
                let center = visible.center();
                if regions
                    .iter()
                    .any(|(l, r)| l == layer && r.contains(center))
                {
                    continue;
                }

                let mut bg = palette.app;
                for (rect, fill) in &bg_rects {
                    if rect.contains(center) {
                        bg = blend_over(*fill, bg);
                    }
                }

                let first_sec = t.galley.job.sections.first();
                let raw_fg = t
                    .override_text_color
                    .filter(|c| *c != egui::Color32::PLACEHOLDER)
                    .or_else(|| {
                        first_sec
                            .map(|s| s.format.color)
                            .filter(|c| *c != egui::Color32::PLACEHOLDER)
                    })
                    .unwrap_or_else(|| {
                        if t.fallback_color != egui::Color32::PLACEHOLDER
                            && t.fallback_color != egui::Color32::TRANSPARENT
                        {
                            t.fallback_color
                        } else {
                            palette.text
                        }
                    });
                let has_disabled_owner = nodes.iter().any(|n| {
                    is_interactive_role(n.role) && n.bounds.contains(center) && n.disabled
                });
                if raw_fg == palette.text_disabled {
                    if !nodes.is_empty()
                        && !has_disabled_owner
                        && let Some(owner) = nodes.iter().find(|n| {
                            is_interactive_role(n.role) && n.bounds.contains(center) && !n.disabled
                        })
                    {
                        findings.push(format!(
                            "disabled-style-on-enabled-widget: {name} text \"{}\" uses text_disabled on enabled {:?} ({:?}) at {}",
                            shorten(t.galley.text()),
                            owner.role,
                            owner.label.as_deref().unwrap_or(""),
                            fmt_rect(visible)
                        ));
                    }
                    continue;
                }
                if has_disabled_owner {
                    continue;
                }

                let fg_scaled = if t.opacity_factor < 1.0 {
                    raw_fg.gamma_multiply(t.opacity_factor)
                } else {
                    raw_fg
                };
                let fg = blend_over(fg_scaled, bg);
                let font_size = first_sec.map_or(13.0, |s| s.format.font_id.size);
                let min_cr = if font_size >= 18.0 || is_phosphor_only(t.galley.text()) {
                    3.0
                } else {
                    4.5
                };
                let cr = coursecraft_render::contrast_ratio(fg, bg);
                if cr + 0.05 < min_cr {
                    findings.push(format!(
                        "low-contrast: {name} text \"{}\" has contrast {cr:.2}:1 (need {min_cr:.1}:1, fg={fg:?} bg={bg:?}) at {}",
                        shorten(t.galley.text()),
                        fmt_rect(visible)
                    ));
                }
            }
        }
    }
    findings
}

fn collect_clip_rects_and_scrollbars(
    shape: &Shape,
    clip: Rect,
    text_clips: &mut Vec<Rect>,
    thin_v_rects: &mut Vec<Rect>,
) {
    match shape {
        Shape::Vec(v) => {
            for s in v {
                collect_clip_rects_and_scrollbars(s, clip, text_clips, thin_v_rects);
            }
        }
        Shape::Text(t) => {
            if !is_invisible(t) && clip.is_positive() {
                text_clips.push(clip);
            }
        }
        Shape::Rect(r) if r.fill.a() > 0 && r.rect.width() <= 14.0 && r.rect.height() >= 16.0 => {
            thin_v_rects.push(r.rect);
        }
        _ => {}
    }
}

fn rect_contains_with_tol(outer: Rect, inner: Rect, tol: f32) -> bool {
    outer.min.x - tol <= inner.min.x
        && outer.min.y - tol <= inner.min.y
        && inner.max.x <= outer.max.x + tol
        && inner.max.y <= outer.max.y + tol
}

#[allow(dead_code)]
pub(crate) fn check_nested_scrolls(
    layers: &[(LayerId, Vec<ClippedShape>)],
    scrolls: &[crate::scroll_audit::ScrollRecord],
) -> Vec<String> {
    let mut findings = Vec::new();
    // 1. Pairwise check among explicitly recorded vertical scroll viewports on the same layer.
    for (i, a) in scrolls.iter().enumerate() {
        if !a.vertical {
            continue;
        }
        for b in scrolls.iter().skip(i + 1) {
            if !b.vertical || a.layer != b.layer {
                continue;
            }
            let (outer, inner) = if a.rect.area() >= b.rect.area() {
                (a.rect, b.rect)
            } else {
                (b.rect, a.rect)
            };
            if inner.height() + 2.0 < outer.height()
                && (rect_contains_with_tol(outer, inner, 1.0)
                    || outer.intersect(inner).area() > 0.8 * inner.area())
            {
                findings.push(format!(
                    "nested-scroll: {} has nested vertical ScrollArea {} inside {}",
                    layer_name(a.layer),
                    fmt_rect(inner),
                    fmt_rect(outer)
                ));
            }
        }
    }

    // 2. Inspect clip-rect hierarchy inside DialogUi::body vertical scroll viewports to catch
    //    any inner vertical ScrollArea without requiring every caller to be wrapped.
    for s in scrolls {
        if !s.vertical || !s.from_dialog_body {
            continue;
        }
        let Some((_, shapes)) = layers.iter().find(|(l, _)| *l == s.layer) else {
            continue;
        };
        let mut text_clips = Vec::new();
        let mut thin_v_rects = Vec::new();
        for c in shapes {
            collect_clip_rects_and_scrollbars(
                &c.shape,
                c.clip_rect,
                &mut text_clips,
                &mut thin_v_rects,
            );
        }
        let mut seen_clips: Vec<Rect> = Vec::new();
        for clip in &text_clips {
            if seen_clips.iter().any(|p| {
                (p.min.x - clip.min.x).abs() < 0.5
                    && (p.min.y - clip.min.y).abs() < 0.5
                    && (p.max.x - clip.max.x).abs() < 0.5
                    && (p.max.y - clip.max.y).abs() < 0.5
            }) {
                continue;
            }
            if rect_contains_with_tol(s.rect, *clip, 1.0)
                && clip.height() >= 32.0
                && clip.height() + 4.0 < s.rect.height()
            {
                let text_count = text_clips.iter().filter(|c| *c == clip).count();
                let has_v_bar = thin_v_rects.iter().any(|r| {
                    (r.min.x - clip.max.x).abs() <= 14.0
                        && r.min.y >= clip.min.y - 2.0
                        && r.max.y <= clip.max.y + 2.0
                });
                if has_v_bar || (clip.height() + 24.0 < s.rect.height() && text_count >= 2) {
                    seen_clips.push(*clip);
                    findings.push(format!(
                        "nested-scroll: {} has inner vertical scroll viewport {} inside DialogUi::body {}",
                        layer_name(s.layer),
                        fmt_rect(*clip),
                        fmt_rect(s.rect)
                    ));
                }
            }
        }
    }
    findings
}

fn foreground_popup_rect(layers: &[(LayerId, Vec<ClippedShape>)]) -> Option<(LayerId, Rect)> {
    for (layer, shapes) in layers.iter().rev() {
        if layer.order != Order::Foreground {
            continue;
        }
        let mut bounds = Rect::NOTHING;
        for c in shapes {
            let mut boxes = Vec::new();
            collect_boxes(&c.shape, c.clip_rect, &mut boxes);
            for b in boxes {
                bounds = bounds.union(b.rect.intersect(b.clip));
            }
        }
        if bounds.is_positive() {
            return Some((*layer, bounds));
        }
    }
    None
}

fn foreground_scrollbar_x(
    layers: &[(LayerId, Vec<ClippedShape>)],
    layer: LayerId,
    popup_rect: Rect,
) -> Option<f32> {
    let (_, shapes) = layers.iter().find(|(l, _)| *l == layer)?;
    let mut best_x: Option<f32> = None;
    for c in shapes {
        if let Shape::Rect(r) = &c.shape
            && r.fill.a() > 0
            && r.rect.width() <= 14.0
            && r.rect.height() >= 16.0
            && r.rect.max.x >= popup_rect.max.x - 18.0
        {
            best_x = Some(best_x.map_or(r.rect.min.x, |x| x.max(r.rect.min.x)));
        }
    }
    best_x
}

#[allow(dead_code)]
pub(crate) fn check_popup_stability(widths: &[f32], scrollbar_xs: &[Option<f32>]) -> Vec<String> {
    let mut findings = Vec::new();
    if let (Some(&first), Some(&last)) = (widths.first(), widths.last()) {
        let _ = (first, last);
        let min_w = widths.iter().copied().fold(f32::INFINITY, f32::min);
        let max_w = widths.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        if max_w - min_w > 1.0 {
            findings.push(format!(
                "popup-width-unstable: width drifted by {:.2} pt during scroll (min={min_w:.1}, max={max_w:.1})",
                max_w - min_w
            ));
        }
    }
    let xs: Vec<f32> = scrollbar_xs.iter().flatten().copied().collect();
    if xs.len() >= 2 {
        let min_x = xs.iter().copied().fold(f32::INFINITY, f32::min);
        let max_x = xs.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        if max_x - min_x > 1.0 {
            findings.push(format!(
                "scrollbar-position-unstable: scrollbar x drifted by {:.2} pt during scroll (min={min_x:.1}, max={max_x:.1})",
                max_x - min_x
            ));
        }
    }
    findings
}

fn collect_backdrop_rects(shape: &Shape, clip: Rect, screen: Rect, out: &mut Vec<egui::Color32>) {
    match shape {
        Shape::Vec(v) => {
            for s in v {
                collect_backdrop_rects(s, clip, screen, out);
            }
        }
        Shape::Rect(r) => {
            let vis = r.rect.intersect(clip);
            if vis.width() >= screen.width() * 0.95 && vis.height() >= screen.height() * 0.95 {
                out.push(r.fill);
            }
        }
        _ => {}
    }
}

#[allow(dead_code)]
pub(crate) fn check_backdrop(
    layers: &[(LayerId, Vec<ClippedShape>)],
    screen: Rect,
    min_alpha: u8,
) -> Vec<String> {
    let mut alphas = Vec::new();
    for (layer, shapes) in layers {
        if layer.order == Order::Background || layer.order == Order::Tooltip {
            continue;
        }
        for c in shapes {
            collect_backdrop_rects(&c.shape, c.clip_rect, screen, &mut alphas);
        }
    }
    let Some(&fill) = alphas.last() else {
        return vec![
            "missing-modal-backdrop: no full-screen backdrop rect found above Background order"
                .to_owned(),
        ];
    };
    if fill.a() < min_alpha {
        vec![format!(
            "weak-modal-backdrop: backdrop alpha {} (0x{:02X}) is below required {min_alpha} (0x{min_alpha:02X})",
            fill.a(),
            fill.a()
        )]
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hello_course() -> Course {
        let json = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/hello-course/course.json"
        ))
        .expect("hello course");
        Course::from_json(&json).expect("valid course")
    }

    #[test]
    fn virtual_clock_is_monotonic() {
        let mut h = TestHarness::new(hello_course());
        assert_eq!(h.time, 0.0);
        h.idle_frame();
        assert!((h.time - 1.0 / 60.0).abs() < 1e-9);
        let t1 = h.time;
        h.advance_time(0.5);
        assert!((h.time - (t1 + 0.5)).abs() < 1e-9);
        h.advance_time(-2.0);
        assert!((h.time - (t1 + 0.5)).abs() < 1e-9);
    }

    #[test]
    fn clipboard_helpers_copy_cut_and_paste_text() {
        let mut h = TestHarness::new(hello_course());
        h.idle_frame();
        let initial_len = h.editor.session.course.scenes[0].slides[0].objects.len();

        h.paste_text("Hello harness");
        h.idle_frame();
        assert_eq!(
            h.editor.session.course.scenes[0].slides[0].objects.len(),
            initial_len + 1
        );

        let obj_id = h.editor.session.course.scenes[0].slides[0]
            .objects
            .last()
            .expect("pasted text object")
            .id
            .clone();
        h.editor.selection.object = Some(obj_id);
        h.copy();
        assert!(h.editor.ui.clipboard_text.is_some());

        h.cut();
        h.idle_frame();
        assert_eq!(
            h.editor.session.course.scenes[0].slides[0].objects.len(),
            initial_len
        );

        let copied = h.editor.ui.clipboard_text.clone().unwrap_or_default();
        h.paste_text(&copied);
        h.idle_frame();
        assert_eq!(
            h.editor.session.course.scenes[0].slides[0].objects.len(),
            initial_len + 1
        );
    }

    #[test]
    fn click_at_requests_captures_emitted_requests() {
        let mut h = TestHarness::new(hello_course());
        h.idle_frame();
        let file_pos = h.find_text("File").expect("File menu");
        h.click_at(file_pos);
        h.idle_frame();
        let save_as_pos = h.find_text_last("Save as…").expect("Save as menu item");
        let reqs = h.click_at_requests(save_as_pos);
        assert!(reqs.iter().any(|r| matches!(r, EditorRequest::SaveAs)));
    }

    #[test]
    fn double_click_at_registers_egui_double_click() {
        let mut h = TestHarness::new(hello_course());
        h.editor.selection.slide = "s-intro".into();
        h.editor.ui.bottom_tab = BottomTab::Triggers;
        h.idle_frame();
        h.idle_frame();
        let slide = h
            .editor
            .session
            .course
            .slide("s-intro")
            .expect("s-intro")
            .clone();
        let first_desc = crate::triggers::describe(&slide.triggers[0], &slide);
        let row_pos = h.find_text(&first_desc).expect("first trigger row");
        assert!(h.editor.ui.trigger_form.is_none());
        h.double_click_at(row_pos);
        h.idle_frame();
        assert!(h.editor.ui.trigger_form.is_some());
    }

    fn run_synthetic_scene(
        size: (f32, f32),
        zoom_factor: f32,
        mut ui_fn: impl FnMut(&mut egui::Ui),
    ) -> (Vec<WidgetNode>, LayoutReport, Vec<String>) {
        let ctx = Context::default();
        ctx.enable_accesskit();
        ctx.set_zoom_factor(zoom_factor);
        let mut style = (*ctx.global_style()).clone();
        style.animation_time = 0.0;
        ctx.set_global_style(style);
        let screen = Rect::from_min_size(
            pos2(0.0, 0.0),
            vec2(size.0 / zoom_factor, size.1 / zoom_factor),
        );
        let mut time = 0.0;
        let mut layers = Vec::new();
        let mut nodes = Vec::new();
        for _ in 0..2 {
            let raw = RawInput {
                screen_rect: Some(screen),
                time: Some(time),
                ..Default::default()
            };
            time += 1.0 / 60.0;
            let out = ctx.run_ui(raw, |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    ui_fn(ui);
                });
                layers = capture_layers(ui.ctx());
            });
            nodes = extract_widget_nodes(out.platform_output.accesskit_update.as_ref());
            out.drop_without_applying_deltas();
        }
        let report = check_layout(&layers, screen, &[], &[]);
        let tooltip_findings = check_tooltips_and_disabled_with(&nodes, |pos| {
            let mut tip_layers = Vec::new();
            let raw_move = RawInput {
                screen_rect: Some(screen),
                time: Some(time),
                events: vec![Event::PointerMoved(pos)],
                ..Default::default()
            };
            time += 1.0;
            let out1 = ctx.run_ui(raw_move, |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    ui_fn(ui);
                });
            });
            out1.drop_without_applying_deltas();
            for _ in 0..2 {
                let raw_hover = RawInput {
                    screen_rect: Some(screen),
                    time: Some(time),
                    ..Default::default()
                };
                time += 1.0 / 60.0;
                let out2 = ctx.run_ui(raw_hover, |ui| {
                    egui::CentralPanel::default().show(ui, |ui| {
                        ui_fn(ui);
                    });
                    tip_layers = capture_layers(ui.ctx());
                });
                out2.drop_without_applying_deltas();
            }
            let mut texts = Vec::new();
            for (layer, shapes) in &tip_layers {
                if layer.order == Order::Tooltip {
                    let mut items = Vec::new();
                    for c in shapes {
                        collect_texts(&c.shape, c.clip_rect, &mut items);
                    }
                    texts.extend(items.into_iter().map(|i| i.text));
                }
            }
            let joined = texts.join(" ").trim().to_owned();
            if joined.is_empty() {
                None
            } else {
                Some(joined)
            }
        });
        (nodes, report, tooltip_findings)
    }

    #[test]
    fn harness_catches_small_hit_target_button() {
        let (nodes, _, _) = run_synthetic_scene((400.0, 200.0), 1.0, |ui| {
            let _ = ui.add_sized(vec2(18.0, 18.0), egui::Button::new("Tiny"));
        });
        let findings = check_hit_targets(&nodes, vec2(24.0, 24.0));
        assert!(
            findings.iter().any(|f| f.contains("hit-target-too-small")),
            "expected hit-target-too-small finding, got: {findings:?}"
        );

        let (ok_nodes, _, _) = run_synthetic_scene((400.0, 200.0), 1.0, |ui| {
            let _ = ui.add_sized(vec2(28.0, 28.0), egui::Button::new("Good"));
        });
        let ok_findings = check_hit_targets(&ok_nodes, vec2(24.0, 24.0));
        assert!(
            ok_findings.is_empty(),
            "expected no findings, got: {ok_findings:?}"
        );
    }

    #[test]
    fn harness_catches_icon_button_missing_tooltip_or_accessible_name() {
        let (_, _, findings) = run_synthetic_scene((400.0, 200.0), 1.0, |ui| {
            let _ = ui.add_sized(
                vec2(28.0, 28.0),
                egui::Button::new(coursecraft_render::Icon::Trash.codepoint().to_string()),
            );
        });
        assert!(
            findings
                .iter()
                .any(|f| f.contains("missing-tooltip") || f.contains("missing-accessible-name")),
            "expected missing-tooltip or missing-accessible-name, got: {findings:?}"
        );
    }

    #[test]
    fn harness_catches_disabled_button_missing_disabled_tooltip() {
        let (_, _, findings) = run_synthetic_scene((400.0, 200.0), 1.0, |ui| {
            let _ = ui.add_enabled(false, egui::Button::new("Publish Course"));
        });
        assert!(
            findings
                .iter()
                .any(|f| f.contains("missing-disabled-tooltip")),
            "expected missing-disabled-tooltip, got: {findings:?}"
        );

        let (_, _, ok_findings) = run_synthetic_scene((400.0, 200.0), 1.0, |ui| {
            let _ = ui
                .add_enabled(false, egui::Button::new("Publish Course"))
                .on_disabled_hover_text("Save the course before publishing.");
        });
        assert!(
            ok_findings.is_empty(),
            "expected no findings when disabled tooltip is present, got: {ok_findings:?}"
        );
    }

    #[test]
    fn harness_catches_clipped_control_at_scale_1_5() {
        let label = "Button label that fits at 1.0x";
        let (_, report_1x, _) = run_synthetic_scene((260.0, 200.0), 1.0, |ui| {
            ui.horizontal(|ui| {
                let _ = ui.button(label);
            });
        });
        assert!(
            !report_1x.findings.iter().any(|f| f.contains("clipped")),
            "expected no clipped finding at 1.0x scale, got: {:?}",
            report_1x.findings
        );

        let (_, report_15x, _) = run_synthetic_scene((260.0, 200.0), 1.5, |ui| {
            ui.horizontal(|ui| {
                let _ = ui.button(label);
            });
        });
        assert!(
            report_15x.findings.iter().any(|f| f.contains("clipped")),
            "expected clipped finding at 1.5x scale, got: {:?}",
            report_15x.findings
        );
    }

    #[test]
    fn new_with_config_applies_zoom_factor_and_file_ops() {
        let mut h_web = TestHarness::new_with_config(
            hello_course(),
            (1200.0, 900.0),
            coursecraft_render::ThemeMode::Dark,
            1.5,
            false,
        );
        h_web.idle_frame();
        assert!((h_web.ctx.zoom_factor() - 1.5).abs() < 1e-4);
        assert!((h_web.screen_rect.width() - 800.0).abs() < 1e-2);
        assert!((h_web.screen_rect.height() - 600.0).abs() < 1e-2);
        assert!(!h_web.editor.file_ops_available);
        assert!(!h_web.last_widgets.is_empty());
        assert!(
            h_web.find_text("Publish").is_none(),
            "Publish button should not appear when file_ops=false"
        );

        let mut h_desktop = TestHarness::new_with_config(
            hello_course(),
            (1280.0, 800.0),
            coursecraft_render::ThemeMode::Light,
            1.0,
            true,
        );
        h_desktop.idle_frame();
        assert!(h_desktop.editor.file_ops_available);
        assert!(
            h_desktop.find_text("Publish").is_some(),
            "Publish button should appear when file_ops=true"
        );
        let _ = h_desktop.check_hit_targets(vec2(24.0, 24.0));
    }

    #[test]
    fn harness_catches_low_contrast_dark_on_blue() {
        let ctx = Context::default();
        coursecraft_render::apply_style(&ctx, coursecraft_render::ThemeMode::Light);
        let p = coursecraft_render::palette(&ctx);
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 200.0));
        let mut layers = Vec::new();
        let out = ctx.run_ui(
            RawInput {
                screen_rect: Some(screen),
                time: Some(0.0),
                ..Default::default()
            },
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    let rect = Rect::from_min_size(pos2(20.0, 20.0), vec2(180.0, 32.0));
                    ui.painter().rect_filled(rect, 4.0, p.accent);
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "Selected Font",
                        egui::FontId::proportional(13.0),
                        p.text,
                    );
                });
                layers = capture_layers(ui.ctx());
            },
        );
        out.drop_without_applying_deltas();
        let findings = check_contrast(&layers, &[], &p, &[]);
        assert!(
            findings.iter().any(|f| f.contains("low-contrast")),
            "expected low-contrast finding for dark-on-blue, got: {findings:?}"
        );

        let mut ok_layers = Vec::new();
        let ok_out = ctx.run_ui(
            RawInput {
                screen_rect: Some(screen),
                time: Some(0.1),
                ..Default::default()
            },
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    let rect = Rect::from_min_size(pos2(20.0, 20.0), vec2(180.0, 32.0));
                    ui.painter().rect_filled(rect, 4.0, p.panel);
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "Readable Text",
                        egui::FontId::proportional(13.0),
                        p.text,
                    );
                });
                ok_layers = capture_layers(ui.ctx());
            },
        );
        ok_out.drop_without_applying_deltas();
        let ok_findings = check_contrast(&ok_layers, &[], &p, &[]);
        assert!(
            ok_findings.is_empty(),
            "expected no low-contrast findings for p.text on p.panel, got: {ok_findings:?}"
        );
    }

    #[test]
    fn harness_catches_nested_vertical_scroll_in_dialog_body() {
        let ctx = Context::default();
        coursecraft_render::apply_style(&ctx, coursecraft_render::ThemeMode::Dark);
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0));
        let mut layers = Vec::new();
        let mut scrolls = Vec::new();
        let mut open = true;
        for frame_idx in 0..3 {
            let out = ctx.run_ui(
                RawInput {
                    screen_rect: Some(screen),
                    time: Some(frame_idx as f64 * (1.0 / 60.0)),
                    ..Default::default()
                },
                |ui| {
                    crate::dialog::Dialog::new("Nested Scroll Test")
                        .open(&mut open)
                        .width(420.0)
                        .show_with_footer(ui.ctx(), |d| {
                            d.body(|ui| {
                                egui::ScrollArea::vertical()
                                    .id_salt("inner_vert")
                                    .max_height(100.0)
                                    .show(ui, |ui| {
                                        for i in 0..20 {
                                            ui.label(format!("Inner row {i}"));
                                        }
                                    });
                            });
                        });
                    layers = capture_layers(ui.ctx());
                    scrolls = crate::scroll_audit::recorded_scrolls(ui.ctx());
                },
            );
            out.drop_without_applying_deltas();
        }
        let findings = check_nested_scrolls(&layers, &scrolls);
        assert!(
            findings.iter().any(|f| f.contains("nested-scroll")),
            "expected nested-scroll finding for ScrollArea inside DialogUi::body, got: {findings:?}"
        );

        // Horizontal ScrollArea inside DialogUi::body is allowed (Reviewer Note 5).
        let mut h_layers = Vec::new();
        let mut h_scrolls = Vec::new();
        let mut h_open = true;
        for frame_idx in 0..3 {
            let out = ctx.run_ui(
                RawInput {
                    screen_rect: Some(screen),
                    time: Some((frame_idx + 10) as f64 * (1.0 / 60.0)),
                    ..Default::default()
                },
                |ui| {
                    crate::dialog::Dialog::new("Horizontal Inside Body")
                        .open(&mut h_open)
                        .width(420.0)
                        .show_with_footer(ui.ctx(), |d| {
                            d.body(|ui| {
                                egui::ScrollArea::horizontal().id_salt("inner_horiz").show(
                                    ui,
                                    |ui| {
                                        ui.horizontal(|ui| {
                                            for i in 0..20 {
                                                ui.label(format!("Col {i}"));
                                            }
                                        });
                                    },
                                );
                            });
                        });
                    h_layers = capture_layers(ui.ctx());
                    h_scrolls = crate::scroll_audit::recorded_scrolls(ui.ctx());
                },
            );
            out.drop_without_applying_deltas();
        }
        let h_findings = check_nested_scrolls(&h_layers, &h_scrolls);
        assert!(
            h_findings.is_empty(),
            "expected horizontal ScrollArea inside DialogUi::body to be allowed, got: {h_findings:?}"
        );
    }

    #[test]
    fn harness_catches_width_jumping_popup_scrollbar() {
        let findings = check_popup_stability(
            &[140.0, 140.0, 220.0, 220.0, 220.0],
            &[
                Some(132.0),
                Some(132.0),
                Some(212.0),
                Some(212.0),
                Some(212.0),
            ],
        );
        assert!(
            findings.iter().any(|f| f.contains("popup-width-unstable")),
            "expected popup-width-unstable finding, got: {findings:?}"
        );
        let ok = check_popup_stability(
            &[220.0, 220.0, 220.0, 220.0, 220.0],
            &[
                Some(212.0),
                Some(212.0),
                Some(212.0),
                Some(212.0),
                Some(212.0),
            ],
        );
        assert!(ok.is_empty(), "expected stable popup to pass, got: {ok:?}");
    }

    #[test]
    fn harness_catches_light_or_transparent_modal_backdrop() {
        let ctx = Context::default();
        coursecraft_render::apply_style(&ctx, coursecraft_render::ThemeMode::Light);
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0));
        let mut layers = Vec::new();
        for frame_idx in 0..2 {
            let out = ctx.run_ui(
                RawInput {
                    screen_rect: Some(screen),
                    time: Some(frame_idx as f64 * (1.0 / 60.0)),
                    ..Default::default()
                },
                |ui| {
                    egui::CentralPanel::default().show(ui, |ui| {
                        ui.label("Background");
                    });
                    egui::Area::new(egui::Id::unique("weak_backdrop"))
                        .order(Order::Middle)
                        .fixed_pos(pos2(0.0, 0.0))
                        .show(ui.ctx(), |ui| {
                            let (rect, _) =
                                ui.allocate_exact_size(screen.size(), egui::Sense::click());
                            ui.painter().rect_filled(
                                rect,
                                0.0,
                                egui::Color32::from_black_alpha(0x40),
                            );
                        });
                    layers = capture_layers(ui.ctx());
                },
            );
            out.drop_without_applying_deltas();
        }
        let findings = check_backdrop(&layers, screen, 0x99);
        assert!(
            findings.iter().any(|f| f.contains("weak-modal-backdrop")),
            "expected weak-modal-backdrop finding, got: {findings:?}"
        );

        let mut h = TestHarness::new(hello_course());
        h.idle_frame();
        h.editor.open_preview();
        h.idle_frame();
        h.idle_frame();
        let preview_findings = h.check_modals_and_backdrops(true);
        assert!(
            preview_findings.is_empty(),
            "expected Preview backdrop (>= 0xCC) to pass, got: {preview_findings:?}"
        );
    }
}
