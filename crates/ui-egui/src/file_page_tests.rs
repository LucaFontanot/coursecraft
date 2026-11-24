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

fn welcome() -> TestHarness {
    let mut h = with_ribbon();
    h.editor.ui.show_welcome = true;
    h.editor.ui.has_loaded_course = false;
    h.editor.ui.file_page.section = crate::file_page::FileSection::Start;
    for _ in 0..2 {
        h.idle_frame();
    }
    h
}

#[test]
fn the_welcome_screen_is_the_file_page_at_start() {
    let mut h = welcome();
    for text in [
        "Start",
        "New blank course",
        "Explore an example",
        "Start from a template",
        "Recent courses",
        "Preferences…",
    ] {
        assert!(h.find_text(text).is_some(), "{text:?} missing");
    }
    // Nothing to save or go back to yet.
    let save = h.find_text("Save").expect("Save entry");
    let reqs = h.click_at_requests(save);
    assert!(!reqs.iter().any(|r| matches!(r, EditorRequest::Save)));
    let back = h.find_text("Back to course").expect("Back entry");
    h.click_at(back);
    h.idle_frame();
    assert!(h.editor.ui.show_welcome, "Back is off without a course");
    // Esc doesn't leave the welcome screen either.
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(h.editor.ui.show_welcome);
}

#[test]
fn new_on_the_welcome_page_starts_a_course() {
    let mut h = welcome();
    let new = h.find_text("New").expect("New entry");
    let reqs = h.click_at_requests(new);
    assert!(reqs.iter().any(|r| matches!(r, EditorRequest::New)));
    assert!(!h.editor.ui.show_welcome);
    assert!(h.editor.ui.has_loaded_course);
}

#[test]
fn a_template_on_the_start_section_opens_from_the_file_tab_too() {
    let mut h = with_ribbon();
    h.editor.ui.file_page.section = crate::file_page::FileSection::Start;
    open_page(&mut h);
    let name = crate::welcome::TemplateId::QuickKnowledgeCheck.name();
    let card = h.find_text(name).expect("template card");
    h.click_at(card);
    h.idle_frame();
    assert!(!h.editor.ui.file_page.open, "the page closes");
    assert_eq!(h.editor.session.course.id, "quick-knowledge-check");
}
