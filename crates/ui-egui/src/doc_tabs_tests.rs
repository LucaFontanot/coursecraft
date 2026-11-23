//! Document tabs under the ribbon: the Course Map, then the opened slides.

use crate::doc_tabs;
use crate::test_harness::TestHarness;
use coursecraft_course::Course;

const HELLO_COURSE: &str = include_str!("../../../examples/hello-course/course.json");

fn with_ribbon() -> TestHarness {
    let course = Course::from_json(HELLO_COURSE).expect("hello course parses");
    let mut h = TestHarness::new_with_size(course, (1600.0, 900.0));
    h.editor.ui.ribbon.visible = true;
    for _ in 0..3 {
        h.idle_frame();
    }
    h
}

fn slide_ids(h: &TestHarness) -> Vec<String> {
    crate::selection::slide_layout(&h.editor.session.course)
        .into_iter()
        .flat_map(|(_, slides)| slides)
        .collect()
}

#[test]
fn the_open_slide_gets_a_numbered_tab() {
    let mut h = with_ribbon();
    let first = slide_ids(&h)[0].clone();
    assert_eq!(h.editor.ui.doc_tabs.open, std::slice::from_ref(&first));
    let label = doc_tabs::slide_tab_label(&h.editor.session.course, &first);
    assert!(label.starts_with("1.1 "), "{label}");
    assert!(h.find_text(&label).is_some(), "tab drawn");
    assert!(h.find_text("Course Map").is_some(), "map tab drawn");
}

#[test]
fn tabs_switch_between_the_map_and_slides() {
    let mut h = with_ribbon();
    let ids = slide_ids(&h);
    let pos = h.find_text("Course Map").expect("map tab");
    h.click_at(pos);
    h.idle_frame();
    assert!(h.editor.ui.story_view, "map shown");
    doc_tabs::open_slide(&mut h.editor, &ids[1]);
    h.idle_frame();
    assert_eq!(h.editor.ui.doc_tabs.open, [ids[0].clone(), ids[1].clone()]);
    let label = doc_tabs::slide_tab_label(&h.editor.session.course, &ids[0]);
    let pos = h.find_text(&label).expect("first slide tab");
    h.click_at(pos);
    h.idle_frame();
    assert!(!h.editor.ui.story_view);
    assert_eq!(h.editor.selection.slide, ids[0]);
}

#[test]
fn closing_the_shown_tab_shows_its_neighbour_then_the_map() {
    let mut h = with_ribbon();
    let ids = slide_ids(&h);
    doc_tabs::open_slide(&mut h.editor, &ids[1]);
    h.idle_frame();
    doc_tabs::close(&mut h.editor, &ids[1]);
    h.idle_frame();
    assert_eq!(h.editor.selection.slide, ids[0], "neighbour shown");
    doc_tabs::close(&mut h.editor, &ids[0]);
    h.idle_frame();
    assert!(h.editor.ui.story_view, "the map when no tab is left");
    assert!(h.editor.ui.doc_tabs.open.is_empty());
}

#[test]
fn a_deleted_slide_loses_its_tab() {
    let mut h = with_ribbon();
    let ids = slide_ids(&h);
    doc_tabs::open_slide(&mut h.editor, &ids[1]);
    h.idle_frame();
    h.editor
        .run("slide.delete", serde_json::json!({ "slide": ids[1] }));
    h.idle_frame();
    assert!(!h.editor.ui.doc_tabs.open.contains(&ids[1]));
}

#[test]
fn no_tab_row_without_the_ribbon() {
    let course = Course::from_json(HELLO_COURSE).expect("hello course parses");
    let mut h = TestHarness::new_with_size(course, (1600.0, 900.0));
    h.idle_frame();
    assert!(h.editor.ui.doc_tabs.open.is_empty());
}
