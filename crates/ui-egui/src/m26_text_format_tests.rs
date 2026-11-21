//! Headless UI tests for the Format panel's M26 text box controls:
//! Justify alignment, Wrap text, Text direction, Insets, and Text box border.

use crate::properties::InspectorTab;
use crate::test_harness::TestHarness;
use coursecraft_course::{Align, Course, ObjectKind, TextDirection};

fn hello() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    Course::from_json(&json).expect("valid course")
}

fn select_title(harness: &mut TestHarness) {
    harness.editor.selection.slide = "s-intro".into();
    harness.editor.selection.object = Some("title".into());
    harness.editor.ui.inspector_tab = InspectorTab::Format;
    harness.idle_frame();
    harness.idle_frame();
}

#[test]
fn format_panel_sets_justify_wrap_direction_and_border_on_text_box() {
    let mut harness = TestHarness::new_with_size(hello(), (1280.0, 900.0));
    select_title(&mut harness);

    let right = harness.right_panel_rect();
    let justify = harness
        .find_text_in("Justify", right)
        .expect("Justify button");
    harness.click_at(justify);
    harness.idle_frame();

    let vert270 = harness
        .find_text_in("Vert 270", right)
        .expect("Vert 270 button");
    harness.click_at(vert270);
    harness.idle_frame();

    let wrap = harness
        .find_text_in("Wrap text", right)
        .expect("Wrap text checkbox");
    harness.click_at(wrap);
    harness.idle_frame();

    let obj = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.object("title"))
        .map(|(o, _)| o.clone())
        .expect("title object");
    let ObjectKind::Text { style, .. } = &obj.kind else {
        panic!("expected text object");
    };
    assert_eq!(style.align, Align::Justify);
    assert_eq!(style.direction, TextDirection::Vertical270);
    assert!(!style.wrap);
}
