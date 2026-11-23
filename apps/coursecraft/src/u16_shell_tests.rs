//! Step u16 regression tests: Desktop App Shell & Web Editor (`T13.6`–`T13.8`).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use coursecraft_course::Course;
use coursecraft_ui_egui::coursecraft_render::{ThemeMode, apply_style};
use egui::{Context, Event, Key, Modifiers, RawInput, Rect, pos2, vec2};

use crate::{
    ConflictPrompt, EditorApp, PendingAction, RestorePrompt, editor_viewport_builder,
    resolve_web_theme,
};

const HELLO_JSON: &str = include_str!("../../../examples/hello-course/course.json");

struct ShellHarness {
    ctx: Context,
    app: EditorApp,
    screen_rect: Rect,
    time: f64,
}

impl ShellHarness {
    fn new(mode: ThemeMode) -> Self {
        let ctx = Context::default();
        apply_style(&ctx, mode);
        let mut style = (*ctx.global_style()).clone();
        style.animation_time = 0.0;
        ctx.set_global_style(style);
        let course = Course::from_json(HELLO_JSON).unwrap();
        let mut app = EditorApp::new_desktop(course, None);
        app.editor.ui.show_welcome = false;
        app.editor.ui.has_loaded_course = true;
        app.prefs.theme = match mode {
            ThemeMode::Dark => crate::prefs::ThemePref::Dark,
            ThemeMode::Light => crate::prefs::ThemePref::Light,
        };
        Self {
            ctx,
            app,
            screen_rect: Rect::from_min_size(pos2(0.0, 0.0), vec2(1024.0, 600.0)),
            time: 1.0,
        }
    }

    fn frame(&mut self, events: Vec<Event>) -> egui::FullOutput {
        self.time += 1.0 / 60.0;
        let raw = RawInput {
            screen_rect: Some(self.screen_rect),
            time: Some(self.time),
            events,
            ..Default::default()
        };
        self.ctx.run_ui(raw, |ui| {
            self.app.show_app(ui);
        })
    }

    fn press_escape(&mut self) {
        let out = self.frame(vec![Event::Key {
            key: Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }]);
        out.drop_without_applying_deltas();
        let out2 = self.frame(Vec::new());
        out2.drop_without_applying_deltas();
    }
}

fn collect_backdrop_alphas(shapes: &[egui::epaint::ClippedShape], screen: Rect) -> Vec<u8> {
    fn walk(shape: &egui::epaint::Shape, screen: Rect, out: &mut Vec<u8>) {
        match shape {
            egui::epaint::Shape::Rect(r) => {
                if r.rect.width() >= screen.width() - 2.0
                    && r.rect.height() >= screen.height() - 2.0
                    && r.fill.a() > 0
                {
                    out.push(r.fill.a());
                }
            }
            egui::epaint::Shape::Vec(v) => {
                for s in v {
                    walk(s, screen, out);
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in shapes {
        walk(&clipped.shape, screen, &mut out);
    }
    out
}

fn max_modal_text_width(shapes: &[egui::epaint::ClippedShape], screen: Rect) -> f32 {
    fn walk(shape: &egui::epaint::Shape, screen: Rect, max_w: &mut f32) {
        match shape {
            egui::epaint::Shape::Text(t) => {
                let r = t.visual_bounding_rect();
                if r.is_positive() && r.width() < screen.width() - 40.0 {
                    *max_w = (*max_w).max(r.width());
                }
            }
            egui::epaint::Shape::Vec(v) => {
                for s in v {
                    walk(s, screen, max_w);
                }
            }
            _ => {}
        }
    }
    let mut max_w = 0.0_f32;
    for clipped in shapes {
        walk(&clipped.shape, screen, &mut max_w);
    }
    max_w
}

#[test]
fn t13_6_desktop_minimum_window_size_is_1024x600() {
    let vp = editor_viewport_builder();
    assert_eq!(
        vp.min_inner_size,
        Some(vec2(1024.0, 600.0)),
        "ViewportBuilder must enforce 1024x600 minimum inner size"
    );
}

#[test]
fn t13_7_shell_modals_backdrop_width_clamp_and_esc() {
    let long_name = "very_long_course_filename_that_would_otherwise_stretch_past_the_1024_window_width_without_wrapping_or_max_width_clamp_0123456789_0123456789_0123456789.ccraft";

    for (mode, min_alpha) in [(ThemeMode::Dark, 0x99u8), (ThemeMode::Light, 0x99u8)] {
        let mut h = ShellHarness::new(mode);

        // 1. Unsaved changes modal
        h.app.pending_action = Some(PendingAction::New);
        h.frame(Vec::new()).drop_without_applying_deltas();
        let out = h.frame(Vec::new());
        let alphas = collect_backdrop_alphas(&out.shapes, h.screen_rect);
        out.drop_without_applying_deltas();
        assert!(
            alphas.iter().any(|&a| a >= min_alpha),
            "unsaved_changes_modal ({mode:?}) must paint dimmed backdrop >= {min_alpha:#x}, got {alphas:?}"
        );
        h.press_escape();
        assert!(
            h.app.pending_action.is_none(),
            "Esc must dismiss unsaved_changes_modal"
        );

        // 2. Changed on disk modal
        h.app.conflict_prompt = Some(ConflictPrompt {
            files: vec![long_name.to_string()],
            then: None,
        });
        h.frame(Vec::new()).drop_without_applying_deltas();
        let out = h.frame(Vec::new());
        let alphas = collect_backdrop_alphas(&out.shapes, h.screen_rect);
        let max_w = max_modal_text_width(&out.shapes, h.screen_rect);
        out.drop_without_applying_deltas();
        assert!(
            alphas.iter().any(|&a| a >= min_alpha),
            "changed_on_disk_modal ({mode:?}) must paint dimmed backdrop >= {min_alpha:#x}, got {alphas:?}"
        );
        assert!(
            max_w <= 480.0,
            "changed_on_disk_modal text width {max_w} must wrap within 480 pt"
        );
        h.press_escape();
        assert!(
            h.app.conflict_prompt.is_none(),
            "Esc must dismiss changed_on_disk_modal"
        );

        // 3. Restore autosave modal
        h.app.restore_prompt = Some(RestorePrompt {
            autosave_path: std::path::PathBuf::from(long_name),
            file_path: None,
            time_str: "12:34".to_string(),
        });
        h.frame(Vec::new()).drop_without_applying_deltas();
        let out = h.frame(Vec::new());
        let alphas = collect_backdrop_alphas(&out.shapes, h.screen_rect);
        out.drop_without_applying_deltas();
        assert!(
            alphas.iter().any(|&a| a >= min_alpha),
            "restore_autosave_modal ({mode:?}) must paint dimmed backdrop >= {min_alpha:#x}, got {alphas:?}"
        );
        h.app.restore_prompt = None;

        // 4. Error modal with 150-char message
        h.app.error_modal = Some(format!("Failed to open {long_name}"));
        h.frame(Vec::new()).drop_without_applying_deltas();
        let out = h.frame(Vec::new());
        let alphas = collect_backdrop_alphas(&out.shapes, h.screen_rect);
        let max_w = max_modal_text_width(&out.shapes, h.screen_rect);
        out.drop_without_applying_deltas();
        assert!(
            alphas.iter().any(|&a| a >= min_alpha),
            "error_modal ({mode:?}) must paint dimmed backdrop >= {min_alpha:#x}, got {alphas:?}"
        );
        assert!(
            max_w <= 480.0,
            "error_modal text width {max_w} must wrap within 480 pt"
        );
        h.press_escape();
        assert!(h.app.error_modal.is_none(), "Esc must dismiss error_modal");
    }
}

#[test]
fn t13_8_web_theme_resolution_follows_system_theme() {
    let ctx = Context::default();
    ctx.run_ui(
        RawInput {
            system_theme: Some(egui::Theme::Light),
            ..Default::default()
        },
        |_| {},
    )
    .drop_without_applying_deltas();
    assert_eq!(resolve_web_theme(&ctx), ThemeMode::Light);

    ctx.run_ui(
        RawInput {
            system_theme: Some(egui::Theme::Dark),
            ..Default::default()
        },
        |_| {},
    )
    .drop_without_applying_deltas();
    assert_eq!(resolve_web_theme(&ctx), ThemeMode::Dark);
}

#[test]
fn m31_03_desktop_file_shortcuts_route_dirty_prompts_and_block_during_modals() {
    let mut h = ShellHarness::new(ThemeMode::Dark);
    h.frame(Vec::new()).drop_without_applying_deltas();

    // Mark course dirty by running an edit command.
    h.app.editor.run(
        "course.set_title",
        serde_json::json!({ "title": "Edited Title" }),
    );
    assert!(
        h.app.project.is_dirty(&h.app.editor.session),
        "course must be dirty after edit"
    );

    let key_event = |key: Key, modifiers: Modifiers| {
        vec![Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }]
    };

    // Ctrl/Cmd+N on dirty course opens unsaved-changes prompt.
    h.frame(key_event(Key::N, Modifiers::COMMAND))
        .drop_without_applying_deltas();
    assert!(
        matches!(h.app.pending_action, Some(PendingAction::New)),
        "Cmd/Ctrl+N on dirty course must set PendingAction::New"
    );

    // While pending_action modal is open, subsequent file shortcuts are ignored.
    for (key, mods) in [
        (Key::O, Modifiers::COMMAND),
        (Key::S, Modifiers::COMMAND),
        (Key::S, Modifiers::COMMAND | Modifiers::SHIFT),
        (Key::N, Modifiers::COMMAND),
    ] {
        h.frame(key_event(key, mods)).drop_without_applying_deltas();
        assert!(
            matches!(h.app.pending_action, Some(PendingAction::New)),
            "file shortcut ({key:?}) must be ignored while pending_action modal is open"
        );
    }

    // Dismiss pending_action, then test Ctrl/Cmd+O on dirty course.
    h.app.pending_action = None;
    h.frame(key_event(Key::O, Modifiers::COMMAND))
        .drop_without_applying_deltas();
    assert!(
        matches!(h.app.pending_action, Some(PendingAction::Open)),
        "Cmd/Ctrl+O on dirty course must set PendingAction::Open"
    );
    h.app.pending_action = None;

    // Blocking modals (conflict_prompt, restore_prompt, error_modal) also block file shortcuts.
    h.app.error_modal = Some("Something failed".to_string());
    h.frame(key_event(Key::N, Modifiers::COMMAND))
        .drop_without_applying_deltas();
    assert!(
        h.app.pending_action.is_none(),
        "error_modal must block Cmd/Ctrl+N"
    );
    h.app.error_modal = None;

    h.app.conflict_prompt = Some(ConflictPrompt {
        files: vec!["course.json".to_string()],
        then: None,
    });
    h.frame(key_event(Key::O, Modifiers::COMMAND))
        .drop_without_applying_deltas();
    assert!(
        h.app.pending_action.is_none(),
        "conflict_prompt must block Cmd/Ctrl+O"
    );
    h.app.conflict_prompt = None;

    h.app.restore_prompt = Some(RestorePrompt {
        autosave_path: std::path::PathBuf::from("autosave.ccraft"),
        file_path: None,
        time_str: "12:00".to_string(),
    });
    h.frame(key_event(Key::N, Modifiers::COMMAND))
        .drop_without_applying_deltas();
    assert!(
        h.app.pending_action.is_none(),
        "restore_prompt must block Cmd/Ctrl+N"
    );
    h.app.restore_prompt = None;

    // Web browser shortcut guard: prevents only Cmd/Ctrl+S, Cmd/Ctrl+Shift+S, Cmd/Ctrl+O, Cmd/Ctrl+N.
    assert!(crate::should_prevent_browser_file_shortcut(
        true, false, false, "s"
    ));
    assert!(crate::should_prevent_browser_file_shortcut(
        true, false, true, "S"
    ));
    assert!(crate::should_prevent_browser_file_shortcut(
        true, false, false, "o"
    ));
    assert!(crate::should_prevent_browser_file_shortcut(
        true, false, false, "n"
    ));
    assert!(!crate::should_prevent_browser_file_shortcut(
        true, false, true, "o"
    ));
    assert!(!crate::should_prevent_browser_file_shortcut(
        true, false, true, "n"
    ));
    assert!(!crate::should_prevent_browser_file_shortcut(
        true, false, false, "c"
    ));
    assert!(!crate::should_prevent_browser_file_shortcut(
        true, false, false, "v"
    ));
    assert!(!crate::should_prevent_browser_file_shortcut(
        true, false, false, "a"
    ));
    assert!(!crate::should_prevent_browser_file_shortcut(
        true, true, false, "s"
    ));
    assert!(!crate::should_prevent_browser_file_shortcut(
        false, false, false, "s"
    ));
}
