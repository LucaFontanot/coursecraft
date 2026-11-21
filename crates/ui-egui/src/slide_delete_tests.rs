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
        .find_text_last("Delete Slide")
        .expect("Delete Slide menu item");
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
    let del_pos = h.find_text_last("Delete Slide").expect("Delete Slide");
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
