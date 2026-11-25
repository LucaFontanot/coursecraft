//! Reproduction and regression tests for A14: deleting a slide keeps your place.

use crate::test_harness::TestHarness;
use coursecraft_course::{Course, Slide};
use coursecraft_engine::Session;
use egui::{Key, Modifiers};

fn course_with_slides(count: usize) -> Course {
    let mut course = Session::blank("Forty Slides").course;
    course.scenes[0].slides.clear();
    for i in 1..=count {
        course.scenes[0].slides.push(Slide {
            id: format!("s{i}"),
            title: format!("Slide {i}"),
            duration_ms: 5000,
            background: None,
            objects: Vec::new(),
            layers: Vec::new(),
            groups: Vec::new(),
            triggers: Vec::new(),
            cue_points: Vec::new(),
            transition: None,
            question: None,
            draw: None,
            notes: String::new(),
            master: None,
            revisit: coursecraft_course::RevisitMode::Auto,
            player: None,
        });
    }
    course
}

fn select_slide_in_list(h: &mut TestHarness, label: &str) {
    let panel = h.left_panel_rect();
    if let Some(pos) = h.find_text_in(label, panel) {
        h.click_at(pos);
        h.idle_frame();
        return;
    }
    // Scroll to top first so we can scan downward reliably.
    for _ in 0..80 {
        if h.editor.ui.slides_scroll_y == 0.0 {
            break;
        }
        h.scroll_over(panel.center(), 400.0);
        h.idle_frame();
    }
    for _ in 0..80 {
        if let Some(pos) = h.find_text_in(label, panel) {
            h.click_at(pos);
            h.idle_frame();
            return;
        }
        h.scroll_over(panel.center(), -250.0);
        h.idle_frame();
    }
    panic!("slide {label:?} never scrolled into view");
}

#[test]
fn delete_slide_35_via_edit_menu_keeps_position_and_scroll() {
    let mut h = TestHarness::new(course_with_slides(40));
    h.idle_frame();
    select_slide_in_list(&mut h, "Slide 35");
    assert_eq!(h.editor.selection.slide, "s35");
    assert!(h.editor.ui.slides_scroll_y > 0.0);

    let edit_pos = h.find_text("Edit").expect("Edit menu");
    h.click_at(edit_pos);
    h.idle_frame();
    let del_pos = h
        .find_text_last("Delete slide")
        .expect("Delete slide menu item");
    h.click_at(del_pos);
    h.idle_frame();
    h.idle_frame();

    assert_eq!(h.editor.selection.slide, "s36");
    assert!(
        h.editor.ui.slides_scroll_y > 0.0,
        "slide list should stay scrolled near slide 35, not jump to top"
    );
    assert_eq!(
        h.editor.ui.slides_scrolled_to.as_deref(),
        Some("s36"),
        "slide list should scroll to keep the replacement slide visible"
    );
}

#[test]
fn delete_slide_35_via_keyboard_delete_in_slide_list() {
    let mut h = TestHarness::new(course_with_slides(40));
    h.idle_frame();
    select_slide_in_list(&mut h, "Slide 35");
    assert_eq!(h.editor.selection.slide, "s35");

    h.press(Key::Delete, Modifiers::NONE);
    h.idle_frame();
    h.idle_frame();

    assert_eq!(h.editor.selection.slide, "s36");
    assert_eq!(h.editor.session.course.slides().count(), 39);
    assert_eq!(h.editor.ui.slides_scrolled_to.as_deref(), Some("s36"));
}

#[test]
fn delete_slide_35_via_context_menu_keeps_position_and_scroll() {
    let mut h = TestHarness::new(course_with_slides(40));
    h.idle_frame();
    select_slide_in_list(&mut h, "Slide 35");
    let panel = h.left_panel_rect();
    let pos = h
        .find_text_in("Slide 35", panel)
        .expect("Slide 35 visible in slide list");
    h.right_click_at(pos);
    h.idle_frame();
    let del_pos = h
        .find_text_last("Delete")
        .expect("Delete context menu item");
    h.click_at(del_pos);
    h.idle_frame();
    h.idle_frame();

    assert_eq!(h.editor.selection.slide, "s36");
    assert_eq!(h.editor.ui.slides_scrolled_to.as_deref(), Some("s36"));
}

fn multi_scene_course(counts: &[usize]) -> Course {
    use coursecraft_course::Scene;
    let mut course = Session::blank("Multi Scene").course;
    course.scenes.clear();
    let mut n = 1usize;
    for (si, &count) in counts.iter().enumerate() {
        let mut slides = Vec::new();
        for _ in 0..count {
            slides.push(Slide {
                id: format!("s{n}"),
                title: format!("Slide {n}"),
                duration_ms: 5000,
                background: None,
                objects: Vec::new(),
                layers: Vec::new(),
                groups: Vec::new(),
                triggers: Vec::new(),
                cue_points: Vec::new(),
                transition: None,
                question: None,
                draw: None,
                notes: String::new(),
                master: None,
                revisit: coursecraft_course::RevisitMode::Auto,
                player: None,
            });
            n += 1;
        }
        course.scenes.push(Scene {
            id: format!("scene{}", si + 1),
            title: format!("Scene {}", si + 1),
            slides,
        });
    }
    course
}

#[test]
fn delete_last_slide_selects_new_last_slide() {
    let mut h = TestHarness::new(course_with_slides(40));
    h.idle_frame();
    select_slide_in_list(&mut h, "Slide 40");
    assert_eq!(h.editor.selection.slide, "s40");

    h.press(Key::Delete, Modifiers::NONE);
    h.idle_frame();
    h.idle_frame();

    assert_eq!(h.editor.selection.slide, "s39");
    assert_eq!(h.editor.ui.slides_scrolled_to.as_deref(), Some("s39"));
}

#[test]
fn delete_slide_35_while_scrolled_to_top_scrolls_back_to_replacement() {
    let mut h = TestHarness::new(course_with_slides(40));
    h.idle_frame();
    select_slide_in_list(&mut h, "Slide 35");
    let panel = h.left_panel_rect();
    for _ in 0..40 {
        h.scroll_over(panel.center(), 400.0);
        h.idle_frame();
    }
    assert_eq!(h.editor.ui.slides_scroll_y, 0.0);

    let edit_pos = h.find_text("Edit").expect("Edit menu");
    h.click_at(edit_pos);
    h.idle_frame();
    let del_pos = h.find_text_last("Delete slide").expect("Delete slide");
    h.click_at(del_pos);
    for _ in 0..40 {
        h.idle_frame();
        if h.editor.ui.slides_scrolled_to.as_deref() == Some("s36") {
            break;
        }
    }

    assert_eq!(h.editor.selection.slide, "s36");
    assert!(h.editor.ui.slides_scroll_y > 0.0);
    assert_eq!(h.editor.ui.slides_scrolled_to.as_deref(), Some("s36"));
}

#[test]
fn delete_several_selected_slides_keeps_place_and_is_one_undo() {
    let mut h = TestHarness::new(course_with_slides(40));
    h.idle_frame();
    select_slide_in_list(&mut h, "Slide 34");
    let panel = h.left_panel_rect();
    // Scroll slightly so Slide 36 is also visible, then Shift+click it.
    for _ in 0..5 {
        if let Some(pos) = h.find_text_in("Slide 36", panel) {
            h.click_at_with(pos, Modifiers::SHIFT);
            h.idle_frame();
            break;
        }
        h.scroll_over(panel.center(), -150.0);
        h.idle_frame();
    }
    assert!(h.editor.selection.is_slide_selected("s34"));
    assert!(h.editor.selection.is_slide_selected("s35"));
    assert!(h.editor.selection.is_slide_selected("s36"));

    let rev_before = h.editor.session.revision;
    h.press(Key::Delete, Modifiers::NONE);
    h.idle_frame();
    h.idle_frame();

    assert_eq!(h.editor.session.course.slides().count(), 37);
    assert_eq!(h.editor.selection.slide, "s37");
    assert!(h.editor.selection.also_slides.is_empty());
    assert_eq!(h.editor.session.revision, rev_before + 1);

    // Undo restores all three slides and reselects them.
    h.press(Key::Z, Modifiers::COMMAND);
    h.idle_frame();
    h.idle_frame();
    assert_eq!(h.editor.session.course.slides().count(), 40);
    assert!(h.editor.selection.is_slide_selected("s34"));
    assert!(h.editor.selection.is_slide_selected("s35"));
    assert!(h.editor.selection.is_slide_selected("s36"));

    // Redo deletes them again and selects s37.
    h.press(Key::Y, Modifiers::COMMAND);
    h.idle_frame();
    h.idle_frame();
    assert_eq!(h.editor.session.course.slides().count(), 37);
    assert_eq!(h.editor.selection.slide, "s37");
}

#[test]
fn delete_inside_second_scene_and_last_slide_of_scene() {
    // Scene 1: s1..=s5, Scene 2: s6..=s10
    let mut h = TestHarness::new(multi_scene_course(&[5, 5]));
    h.idle_frame();

    // Delete middle slide of scene 2 (s8) -> selects s9.
    select_slide_in_list(&mut h, "Slide 8");
    h.press(Key::Delete, Modifiers::NONE);
    h.idle_frame();
    assert_eq!(h.editor.selection.slide, "s9");

    // Delete last slide of scene 1 (s5) -> stays in scene 1 on s4.
    select_slide_in_list(&mut h, "Slide 5");
    h.press(Key::Delete, Modifiers::NONE);
    h.idle_frame();
    assert_eq!(h.editor.selection.slide, "s4");

    // Delete last slide of scene 2 (s10) -> stays in scene 2 on s9.
    select_slide_in_list(&mut h, "Slide 10");
    h.press(Key::Delete, Modifiers::NONE);
    h.idle_frame();
    assert_eq!(h.editor.selection.slide, "s9");
}

#[test]
fn undo_and_redo_single_slide_delete_reselects_restored_slide() {
    let mut h = TestHarness::new(course_with_slides(40));
    h.idle_frame();
    select_slide_in_list(&mut h, "Slide 35");
    h.press(Key::Delete, Modifiers::NONE);
    h.idle_frame();
    h.idle_frame();
    assert_eq!(h.editor.selection.slide, "s36");

    // Undo reselects restored s35 and scrolls to keep it visible.
    h.press(Key::Z, Modifiers::COMMAND);
    h.idle_frame();
    h.idle_frame();
    assert_eq!(h.editor.selection.slide, "s35");
    assert_eq!(h.editor.ui.slides_scrolled_to.as_deref(), Some("s35"));

    // Redo deletes s35 again and selects s36.
    h.press(Key::Y, Modifiers::COMMAND);
    h.idle_frame();
    h.idle_frame();
    assert_eq!(h.editor.selection.slide, "s36");
    assert_eq!(h.editor.ui.slides_scrolled_to.as_deref(), Some("s36"));
}

#[test]
fn delete_first_slide_selects_new_first_and_scrolls() {
    let mut h = TestHarness::new(course_with_slides(40));
    h.idle_frame();
    assert_eq!(h.editor.selection.slide, "s1");

    let panel = h.left_panel_rect();
    let pos = h
        .find_text_in("Slide 1", panel)
        .expect("Slide 1 visible in slide list");
    h.click_at(pos);
    h.idle_frame();

    h.press(Key::Delete, Modifiers::NONE);
    h.idle_frame();
    h.idle_frame();

    assert_eq!(h.editor.selection.slide, "s2");
    assert_eq!(h.editor.session.course.slides().count(), 39);
    assert_eq!(h.editor.ui.slides_scrolled_to.as_deref(), Some("s2"));
}

#[test]
fn delete_middle_of_300_slides_keeps_position_and_scrolls() {
    let mut h = TestHarness::new(course_with_slides(300));
    h.idle_frame();
    let panel = h.left_panel_rect();
    let mut clicked = false;
    for _ in 0..200 {
        if let Some(pos) = h.find_text_in("Slide 180", panel)
            && pos.y < panel.max.y - 30.0
        {
            h.click_at(pos);
            h.idle_frame();
            clicked = true;
            break;
        }
        h.scroll_over(panel.center(), -400.0);
        h.idle_frame();
    }
    assert!(clicked, "Slide 180 scrolled into view");
    assert_eq!(h.editor.selection.slide, "s180");
    assert!(h.editor.ui.slides_scroll_y > 0.0);

    h.press(Key::Delete, Modifiers::NONE);
    h.idle_frame();
    h.idle_frame();

    assert_eq!(h.editor.selection.slide, "s181");
    assert_eq!(h.editor.session.course.slides().count(), 299);
    assert!(h.editor.ui.slides_scroll_y > 0.0);
    assert_eq!(h.editor.ui.slides_scrolled_to.as_deref(), Some("s181"));
}

#[test]
fn delete_non_contiguous_selected_slides_and_undo_redo() {
    let mut h = TestHarness::new(course_with_slides(40));
    h.idle_frame();
    select_slide_in_list(&mut h, "Slide 33");
    let panel = h.left_panel_rect();
    for _ in 0..5 {
        if let Some(pos36) = h.find_text_in("Slide 36", panel) {
            h.click_at_with(pos36, Modifiers::COMMAND);
            h.idle_frame();
            break;
        }
        h.scroll_over(panel.center(), -150.0);
        h.idle_frame();
    }
    assert!(h.editor.selection.is_slide_selected("s33"));
    assert!(h.editor.selection.is_slide_selected("s36"));
    assert!(!h.editor.selection.is_slide_selected("s34"));

    h.press(Key::Delete, Modifiers::NONE);
    h.idle_frame();
    h.idle_frame();

    assert_eq!(h.editor.session.course.slides().count(), 38);
    assert!(h.editor.session.course.slide("s33").is_none());
    assert!(h.editor.session.course.slide("s36").is_none());
    assert!(h.editor.session.course.slide("s34").is_some());

    h.press(Key::Z, Modifiers::COMMAND);
    h.idle_frame();
    h.idle_frame();
    assert_eq!(h.editor.session.course.slides().count(), 40);
    assert!(h.editor.selection.is_slide_selected("s33"));
    assert!(h.editor.selection.is_slide_selected("s36"));

    h.press(Key::Y, Modifiers::COMMAND);
    h.idle_frame();
    h.idle_frame();
    assert_eq!(h.editor.session.course.slides().count(), 38);
}

#[test]
fn delete_only_slide_of_second_scene_moves_to_adjacent_scene() {
    // Scene 1: s1, s2; Scene 2: s3 (only slide in scene 2)
    let mut h = TestHarness::new(multi_scene_course(&[2, 1]));
    h.idle_frame();
    select_slide_in_list(&mut h, "Slide 3");
    assert_eq!(h.editor.selection.slide, "s3");

    h.press(Key::Delete, Modifiers::NONE);
    h.idle_frame();
    h.idle_frame();

    assert_eq!(h.editor.session.course.slides().count(), 2);
    assert!(h.editor.session.course.slide("s3").is_none());
    assert_eq!(h.editor.selection.slide, "s2");
}

#[test]
fn delete_slide_while_layer_open_selects_replacement_and_clears_layer() {
    use coursecraft_course::Layer;
    let mut course = course_with_slides(10);
    let layer: Layer = serde_json::from_value(serde_json::json!({
        "id": "l1",
        "name": "Layer 1",
    }))
    .expect("layer");
    course.scenes[0].slides[4].layers.push(layer);
    let mut h = TestHarness::new(course);
    h.idle_frame();
    select_slide_in_list(&mut h, "Slide 5");
    h.editor.selection.layer = Some("l1".into());
    h.idle_frame();

    h.press(Key::Delete, Modifiers::NONE);
    h.idle_frame();
    h.idle_frame();

    assert_eq!(h.editor.selection.slide, "s6");
    assert_eq!(h.editor.selection.layer, None);
    assert_eq!(h.editor.session.course.slides().count(), 9);
}

#[test]
fn delete_key_while_master_view_open_does_not_delete_slide() {
    let mut h = TestHarness::new(course_with_slides(5));
    h.idle_frame();
    select_slide_in_list(&mut h, "Slide 3");
    h.editor.enter_master_view();
    h.idle_frame();
    assert!(h.editor.ui.master_view);

    h.press(Key::Delete, Modifiers::NONE);
    h.idle_frame();
    h.idle_frame();

    assert_eq!(h.editor.session.course.slides().count(), 5);
    assert!(h.editor.session.course.slide("s3").is_some());
}
