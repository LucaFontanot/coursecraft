//! M26 cross-cutting bug-hunt tests (CC.1, CC.4, CC.5, CC.6, CC.7).
//!
//! Ordered P1 -> P2.

use crate::properties::InspectorTab;
use crate::test_harness::TestHarness;
use coursecraft_course::{BaseSlideView, Course, ObjectKind};
use coursecraft_render::ThemeMode;
use egui::{Key, Modifiers, pos2};

fn hello() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    Course::from_json(&json).expect("valid course")
}

// ============================================================================
// P1
// ============================================================================

#[test]
fn cc_1_multi_feature_undo_redo_stack_atomicity() {
    let mut h = TestHarness::new(hello());
    h.idle_frame();
    h.idle_frame();
    let orig = h.editor.session.course.clone();

    // 1. Change text Font size (A1)
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.object = Some("title".into());
    h.editor.ui.inspector_tab = InspectorTab::Format;
    h.idle_frame();
    let panel = h.right_panel_rect();
    let lbl = h.find_text_in("Font size", panel).expect("Font size");
    h.set_field(pos2(panel.min.x + 150.0, lbl.y), "54");

    // 2. Add 3rd gradient stop (A2)
    h.editor.selection.object = Some("btn-start".into());
    h.idle_frame();
    h.pick_combo("Solid", "Linear gradient");
    let add = h.find_text_in("Add colour", panel).expect("Add colour");
    h.click_at(add);
    h.idle_frame();

    // 3. Switch Base slide to Hide on layer about (A9)
    let canvas = h.central_panel_rect();
    let about_pill = h.find_text_in("About", canvas).expect("About pill");
    h.click_at(about_pill);
    h.idle_frame();
    let hide = h.find_text_in("Hide", canvas).expect("Hide");
    h.click_at(hide);
    h.idle_frame();

    // 4. Undo all steps back to original
    while h.editor.session.can_undo() {
        assert!(h.editor.session.undo().is_ok());
        h.idle_frame();
    }
    assert_eq!(h.editor.session.course, orig);

    // 5. Redo all steps
    while h.editor.session.can_redo() {
        assert!(h.editor.session.redo().is_ok());
        h.idle_frame();
    }
    assert_eq!(
        h.editor.session.course.editor.base_slide_view,
        BaseSlideView::Hide
    );
}

#[test]
fn cc_5_layers_and_states_matrix_across_m26_features() {
    let mut h = TestHarness::new(hello());
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.layer = Some("about".into());
    h.editor.selection.object = Some("about-text".into());
    h.editor.ui.inspector_tab = InspectorTab::Format;
    h.idle_frame();
    h.idle_frame();

    let panel = h.right_panel_rect();
    let lbl = h.find_text_in("Font size", panel).expect("Font size");
    h.set_field(pos2(panel.min.x + 150.0, lbl.y), "28");

    let (obj, _) = h
        .editor
        .session
        .course
        .slide("s-intro")
        .unwrap()
        .object("about-text")
        .unwrap();
    match &obj.kind {
        ObjectKind::Text { style, runs, .. } => {
            let sz = runs.first().and_then(|r| r.size).unwrap_or(style.size);
            assert!((sz - 28.0).abs() < 0.01);
        }
        other => panic!("expected Text, got {other:?}"),
    }
}

#[test]
fn cc_6_window_size_and_theme_sweep_zero_clipping_audit() {
    for (w, h_px) in [(1024.0, 680.0), (1280.0, 900.0), (1920.0, 1080.0)] {
        for theme in [ThemeMode::Dark, ThemeMode::Light] {
            let mut h = TestHarness::new_with_theme(hello(), (w, h_px), theme);
            h.editor.selection.slide = "s-quiz".into();
            h.editor.selection.object = None;
            let rep = h.layout_report(3);
            let bad: Vec<_> = rep
                .findings
                .into_iter()
                .filter(|f| {
                    f.starts_with("clipped-widget")
                        || f.starts_with("overlap")
                        || f.starts_with("window-off-screen")
                        || f.starts_with("window-over-menu-bar")
                })
                .collect();
            assert!(bad.is_empty(), "{w}x{h_px} {theme:?} findings: {bad:?}");
        }
    }
}

// ============================================================================
// P2
// ============================================================================

#[test]
fn cc_4_multi_select_matrix_across_m26_features() {
    let mut h = TestHarness::new(hello());
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.set_many(["title", "btn-start"]);
    h.idle_frame();
    assert!(
        !h.editor.can_arm_format_painter(),
        "Paint must be disabled on multi-select"
    );
    let _ = h.layout_report(2);
}

#[test]
fn cc_7_escape_and_keyboard_focus_precedence_chain() {
    let mut h = TestHarness::new(hello());
    h.idle_frame();
    h.idle_frame();

    // 1. Dialog open -> Esc closes dialog
    h.editor.ui.show_interaction_gallery = true;
    h.idle_frame();
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(!h.editor.ui.show_interaction_gallery);

    // 2. Inline text edit open -> Esc closes inline edit, keeps selection
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.object = Some("title".into());
    h.idle_frame();
    let central = h.central_panel_rect();
    let rect =
        crate::canvas::object_screen_rect(&h.editor.session.course, "s-intro", "title", central)
            .unwrap();
    h.click_at(rect.center());
    h.click_at(rect.center());
    h.idle_frame();
    assert!(h.editor.ui.inline_edit.is_some());
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(h.editor.ui.inline_edit.is_none());
    assert_eq!(h.editor.selection.object.as_deref(), Some("title"));

    // 3. Format painter armed -> Esc disarms Paint, keeps selection
    let paint = h.find_text("Paint").expect("Paint");
    h.click_at(paint);
    h.idle_frame();
    assert!(h.editor.ui.format_painter.is_some());
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(h.editor.ui.format_painter.is_none());
    assert_eq!(h.editor.selection.object.as_deref(), Some("title"));
}
