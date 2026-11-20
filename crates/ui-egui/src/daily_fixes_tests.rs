//! Headless tests for M24 step 7 daily fixes.

use crate::properties::InspectorTab;
use crate::test_harness::TestHarness;
use crate::timeline::BottomTab;
use coursecraft_course::{Course, ObjectKind, Value};
use coursecraft_engine::MAX_HISTORY;

fn hello() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    Course::from_json(&json).expect("valid course")
}

#[test]
fn states_panel_shortcut_buttons_add_and_hide() {
    let mut harness = TestHarness::new_with_size(hello(), (1280.0, 1400.0));
    harness.editor.selection.slide = "s-intro".into();
    harness.editor.selection.object = Some("btn-start".into());
    harness.editor.ui.inspector_tab = InspectorTab::Format;
    harness.idle_frame();
    harness.idle_frame();

    let panel_center = harness.right_panel_rect().center();
    for _ in 0..8 {
        if harness
            .find_text_in("+ Hover", harness.right_panel_rect())
            .is_some()
        {
            break;
        }
        harness.scroll_over(panel_center, -200.0);
        harness.idle_frame();
    }

    let hover_pos = harness
        .find_text_in("+ Hover", harness.right_panel_rect())
        .expect("+ Hover button should be visible");
    assert!(
        harness
            .find_text_in("+ Selected", harness.right_panel_rect())
            .is_some()
    );
    assert!(
        harness
            .find_text_in("+ Visited", harness.right_panel_rect())
            .is_some()
    );
    assert!(
        harness
            .find_text_in("+ Disabled", harness.right_panel_rect())
            .is_some()
    );

    harness.click_at(hover_pos);
    harness.idle_frame();

    let obj = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object("btn-start"))
        .map(|(o, _)| o)
        .expect("btn-start");
    assert!(
        obj.states.iter().any(|s| s.name == "hover"),
        "clicking + Hover should add the hover state"
    );
    assert!(
        harness
            .find_text_in("+ Hover", harness.right_panel_rect())
            .is_some(),
        "+ Hover should stay in place (disabled) once the object has a hover state"
    );
    harness.click_at(hover_pos);
    harness.idle_frame();
    let obj_after = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object("btn-start"))
        .map(|(o, _)| o)
        .expect("btn-start");
    assert_eq!(
        obj_after.states.len(),
        1,
        "clicking disabled + Hover again should not add another state"
    );
}

#[test]
fn enter_adds_variable_in_variables_tab() {
    let mut harness = TestHarness::new_with_size(hello(), (1280.0, 800.0));
    harness.editor.ui.bottom_tab = BottomTab::Variables;
    harness.idle_frame();
    harness.idle_frame();

    let name_pos = harness
        .find_text("Name")
        .expect("new variable name hint text");
    harness.click_at(name_pos);
    harness.type_text("score_bonus");
    harness.press(egui::Key::Enter, egui::Modifiers::NONE);
    harness.idle_frame();

    assert!(
        harness
            .editor
            .session
            .course
            .variables
            .iter()
            .any(|v| v.name == "score_bonus" && v.value == Value::Number(0.0)),
        "pressing Enter in the variable name field should add the variable"
    );
}

#[test]
fn double_click_inserts_media_on_current_slide() {
    let mut harness = TestHarness::new_with_size(hello(), (1280.0, 800.0));
    let png = {
        let pixels = vec![180u8; 4 * 4 * 4];
        coursecraft_media::encode_png_rgba(4, 4, &pixels).expect("png")
    };
    harness.editor.import_image("badge.png", &png);
    harness.editor.ui.placement_tool = None;
    harness.editor.ui.show_media_library = true;
    harness.idle_frame();
    harness.idle_frame();

    let initial_count = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("s-intro")
        .objects
        .len();

    let row_pos = harness
        .find_text("badge.png")
        .expect("badge.png row in media library");
    harness.click_at(row_pos);
    harness.click_at(row_pos);
    harness.idle_frame();

    let slide = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("s-intro");
    assert_eq!(slide.objects.len(), initial_count + 1);
    assert!(slide.objects.iter().any(|o| matches!(
        &o.kind,
        ObjectKind::Image { src, .. } if src == "badge.png"
    )));
}

#[test]
fn autofit_is_in_format_text_section() {
    let mut harness = TestHarness::new_with_size(hello(), (1280.0, 800.0));
    harness.editor.selection.slide = "s-intro".into();
    harness.editor.selection.object = Some("title".into());
    harness.editor.ui.inspector_tab = InspectorTab::Format;
    harness.idle_frame();
    harness.idle_frame();

    let font_pos = harness
        .find_text_in("Font", harness.right_panel_rect())
        .expect("Font row in Format panel");
    let autofit_pos = harness
        .find_text_in("Autofit", harness.right_panel_rect())
        .expect("Autofit row in Format panel");
    let style_pos = harness
        .find_text_in("Style", harness.right_panel_rect())
        .expect("Style row in Format panel");

    assert!(
        autofit_pos.y > font_pos.y && autofit_pos.y < style_pos.y,
        "Autofit should appear in the Text section between Font and Style"
    );
}

#[test]
fn svg_with_text_shows_status_warning() {
    let mut harness = TestHarness::new_with_size(hello(), (1280.0, 800.0));
    let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="40"><text x="10" y="20">Hi</text></svg>"#;
    let imported = harness.editor.import_image("label.svg", svg);
    assert!(imported.is_some());
    assert_eq!(
        harness
            .editor
            .ui
            .status_message
            .as_ref()
            .map(|(m, _)| m.as_str()),
        Some(
            "This SVG has text, which CourseCraft draws only as shapes. Convert text to outlines in your drawing app."
        )
    );
}

#[test]
fn history_panel_shows_limit_note_after_exceeding_max_history() {
    let mut harness = TestHarness::new_with_size(hello(), (1280.0, 800.0));
    assert!(!harness.editor.session.history_truncated());

    for i in 0..=MAX_HISTORY {
        harness.editor.run(
            "course.update",
            serde_json::json!({ "title": format!("Title {i}") }),
        );
    }
    assert!(harness.editor.session.history_truncated());

    harness.editor.ui.show_history = true;
    harness.idle_frame();
    harness.idle_frame();

    assert!(
        harness
            .find_text("Older steps were removed (CourseCraft keeps the last 200).")
            .is_some(),
        "History panel should show the limit note when older steps were removed"
    );
}
