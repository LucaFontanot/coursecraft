//! Regression tests for dialogs that close themselves after a choice and
//! reopen at full size (A6).

use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use egui::{Id, Rect};

fn hello() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    Course::from_json(&json).expect("valid course")
}

fn open_insert_interaction(h: &mut TestHarness) {
    let insert = h.find_text("Insert").expect("Insert menu");
    h.click_at(insert);
    h.idle_frame();
    let item = h.find_text("Interaction…").expect("Interaction… menu item");
    h.click_at(item);
    h.idle_frame();
    h.idle_frame();
}

fn dialog_rect(h: &TestHarness, title: &str) -> Rect {
    let id = Id::unique(("coursecraft_dialog", title));
    h.ctx.memory(|m| m.area_rect(id)).expect("dialog area_rect")
}

#[test]
fn insert_interaction_reopens_full_size_after_picking_an_item() {
    let mut h = TestHarness::new(hello());
    h.idle_frame();
    open_insert_interaction(&mut h);
    let first_rect = dialog_rect(&h, "Insert interaction");
    assert!(
        first_rect.width() >= 700.0 && first_rect.height() >= 300.0,
        "first open should be full size, got {first_rect:?}"
    );

    let tabs = h.find_text("Tabs").expect("Tabs card");
    h.click_at(tabs);
    h.idle_frame();
    assert!(!h.editor.ui.show_interaction_gallery);

    open_insert_interaction(&mut h);
    let second_rect = dialog_rect(&h, "Insert interaction");
    assert!(
        second_rect.width() >= 700.0 && second_rect.height() >= 300.0,
        "second open should be full size (first={first_rect:?}, second={second_rect:?})"
    );

    let marker = h.find_text("Marker").expect("Marker card");
    h.click_at(marker);
    h.idle_frame();
    assert!(!h.editor.ui.show_interaction_gallery);

    open_insert_interaction(&mut h);
    let third_rect = dialog_rect(&h, "Insert interaction");
    assert!(
        third_rect.width() >= 700.0 && third_rect.height() >= 300.0,
        "third open should be full size (third={third_rect:?})"
    );
}

#[test]
fn insert_shape_reopens_full_size_after_picking_a_shape() {
    let mut h = TestHarness::new(hello());
    h.idle_frame();

    let open_shapes = |h: &mut TestHarness| {
        let insert = h.find_text("Insert").expect("Insert menu");
        h.click_at(insert);
        h.idle_frame();
        let item = h
            .find_texts("Shape")
            .into_iter()
            .max_by(|a, b| a.y.total_cmp(&b.y))
            .expect("Shape menu item");
        h.click_at(item);
        h.idle_frame();
        h.idle_frame();
    };

    open_shapes(&mut h);
    let first_rect = dialog_rect(&h, "Shapes");
    assert!(
        first_rect.width() >= 300.0 && first_rect.height() >= 200.0,
        "first open should be full size, got {first_rect:?}"
    );

    let (_, rect_btn) = h
        .editor
        .ui
        .shape_gallery_rects
        .iter()
        .find(|(kind, _)| *kind == coursecraft_course::ShapeKind::Rectangle)
        .copied()
        .expect("Rectangle in shape gallery");
    h.click_at(rect_btn.center());
    h.idle_frame();
    assert!(!h.editor.ui.show_shape_gallery_window);

    open_shapes(&mut h);
    let second_rect = dialog_rect(&h, "Shapes");
    assert!(
        second_rect.width() >= 300.0 && second_rect.height() >= 200.0,
        "second open should be full size (first={first_rect:?}, second={second_rect:?})"
    );

    h.press(egui::Key::Escape, egui::Modifiers::NONE);
    h.idle_frame();
    assert!(!h.editor.ui.show_shape_gallery_window);

    open_shapes(&mut h);
    let third_rect = dialog_rect(&h, "Shapes");
    assert!(
        third_rect.width() >= 300.0 && third_rect.height() >= 200.0,
        "third open after Escape should be full size (third={third_rect:?})"
    );
}
