//! The File page, opened from the ribbon's File tab (ribbon.md › File).

use crate::EditorRequest;
use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use egui::{Key, Modifiers};

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

fn open_page(h: &mut TestHarness) {
    let pos = h.find_text("File").expect("File tab");
    h.click_at(pos);
    h.idle_frame();
    assert!(h.editor.ui.file_page.open, "the File tab opens the page");
}

#[test]
fn the_ribbon_replaces_the_menu_bar() {
    let mut h = with_ribbon();
    let file = h.find_text("File").expect("File tab");
    assert!(
        file.y < crate::ribbon::STRIP_H,
        "the File tab is at the very top: {file:?}"
    );
    assert!(h.find_text("Edit").is_none(), "no Edit menu");
}

#[test]
fn the_page_lists_the_file_commands() {
    let mut h = with_ribbon();
    open_page(&mut h);
    for text in [
        "Back",
        "New",
        "Save As…",
        "Recent",
        "Import",
        "Export",
        "Translation",
        "Publish…",
        "Preferences…",
        "Quit",
    ] {
        assert!(h.find_text(text).is_some(), "{text:?} missing");
    }
}

#[test]
fn escape_and_back_return_to_the_editor() {
    let mut h = with_ribbon();
    open_page(&mut h);
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(!h.editor.ui.file_page.open, "Esc closes");
    open_page(&mut h);
    let back = h.find_text("Back").expect("Back");
    h.click_at(back);
    h.idle_frame();
    assert!(!h.editor.ui.file_page.open, "Back closes");
}

#[test]
fn an_action_closes_the_page_and_runs() {
    let mut h = with_ribbon();
    open_page(&mut h);
    let prefs = h.find_text("Preferences…").expect("Preferences");
    h.click_at(prefs);
    h.idle_frame();
    assert!(!h.editor.ui.file_page.open);
    assert!(h.editor.ui.prefs.open, "Preferences opened");
}

#[test]
fn a_section_shows_its_choices() {
    let mut h = with_ribbon();
    open_page(&mut h);
    let import = h.find_text("Import").expect("Import entry");
    h.click_at(import);
    h.idle_frame();
    assert!(h.editor.ui.file_page.open, "a section keeps the page open");
    let pptx = h.find_text("PowerPoint…").expect("PowerPoint choice");
    let requests = h.click_at_requests(pptx);
    assert!(
        requests
            .iter()
            .any(|r| matches!(r, EditorRequest::PickPptx)),
        "{requests:?}"
    );
}
