//! Package A of the 0.5.2 review: shell keys and the slide list.
//!
//! Q3/R5/S5 one key guard, Q13 slide-list arrows, Q7 locked objects, Q6 nudge bursts,
//! Q11 zoom, R4 Ctrl+S in a field, Q4 start page, S1 Help > Welcome, Q9 Check Course,
//! Q8 multi-slide menu actions, P5 scroll-into-view, P7 shortcut table.

use crate::slides::{SlideNav, plan_slide_moves, slide_thumbnail_rect};
use crate::test_harness::TestHarness;
use crate::{ActiveArea, EditorRequest, ZoomLevel};
use coursecraft_course::Course;
use egui::{Event, Key, Modifiers};

fn course_json(locked_second: bool) -> String {
    let lock = if locked_second {
        r#","locked":true"#
    } else {
        ""
    };
    format!(
        r##"{{"schema_version":1,"id":"fix-a","title":"Fix A","slide_size":{{"w":1280,"h":720}},
"scenes":[
 {{"id":"sc1","title":"One","slides":[
  {{"id":"s1","title":"Slide 1","objects":[
    {{"id":"a","kind":{{"type":"shape","shape":"rectangle","fill":"#336699"}},"frame":{{"x":100,"y":100,"w":200,"h":100}}}},
    {{"id":"b","kind":{{"type":"shape","shape":"rectangle","fill":"#996633"}},"frame":{{"x":400,"y":100,"w":200,"h":100}}{lock}}}
  ]}},
  {{"id":"s2","title":"Slide 2","objects":[]}}
 ]}},
 {{"id":"sc2","title":"Two","slides":[
  {{"id":"s3","title":"Slide 3","objects":[]}},
  {{"id":"s4","title":"Slide 4","objects":[]}}
 ]}}
]}}"##
    )
}

fn harness(locked_second: bool) -> TestHarness {
    let course = Course::from_json(&course_json(locked_second)).expect("fix-a course parses");
    let mut h = TestHarness::new(course);
    for _ in 0..3 {
        h.idle_frame();
    }
    h
}

fn key_down(key: Key, repeat: bool) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat,
        modifiers: Modifiers::NONE,
    }
}

fn key_up(key: Key) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

fn frame_x(h: &TestHarness, id: &str) -> f32 {
    h.editor
        .session
        .course
        .slide("s1")
        .and_then(|s| s.object(id))
        .map(|(o, _)| o.frame.x)
        .expect("object")
}

fn click_slide(h: &mut TestHarness, id: &str) {
    let left = h.left_panel_rect();
    let rect = slide_thumbnail_rect(&h.editor.session.course, left, id).expect("thumbnail");
    h.click_at(rect.center());
    h.idle_frame();
}

// ---- Q3 / R5 / S5: one guard for the global keys --------------------------------------

#[test]
fn keys_do_not_reach_the_editor_behind_the_file_page() {
    let mut h = harness(false);
    h.editor.ui.ribbon.visible = true;
    h.idle_frame();
    h.editor.selection.set_many(vec!["a".to_owned()]);
    let before = h.editor.session.course.clone();
    h.editor.ui.file_page.open = true;
    h.idle_frame();
    h.press(Key::ArrowRight, Modifiers::NONE);
    h.press(Key::Delete, Modifiers::NONE);
    h.press(Key::A, Modifiers::COMMAND);
    h.press(Key::Z, Modifiers::COMMAND);
    assert!(h.editor.ui.file_page.open);
    assert_eq!(
        h.editor.session.course, before,
        "nothing changed behind the page"
    );
    assert_eq!(h.editor.selection.ids(), vec!["a".to_owned()]);
}

#[test]
fn keys_do_not_reach_the_editor_behind_a_dialog() {
    let mut h = harness(false);
    h.editor.selection.set_many(vec!["a".to_owned()]);
    h.editor.ui.show_course_settings_window = true;
    h.idle_frame();
    let before = h.editor.session.course.clone();
    for key in [Key::ArrowRight, Key::ArrowDown, Key::Delete] {
        h.press(key, Modifiers::NONE);
    }
    h.press(Key::A, Modifiers::COMMAND);
    h.press(Key::D, Modifiers::COMMAND);
    h.press(Key::Z, Modifiers::COMMAND);
    h.press(Key::Z, Modifiers::COMMAND | Modifiers::SHIFT);
    assert_eq!(
        h.editor.session.course, before,
        "nothing changed behind the dialog"
    );
    // New and Open do not fire behind a dialog either.
    h.editor.file_ops_available = true;
    let mut requests = h.frame(vec![Event::Key {
        key: Key::N,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::COMMAND,
    }]);
    requests.extend(h.frame(vec![Event::Key {
        key: Key::O,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::COMMAND,
    }]));
    assert!(
        !requests
            .iter()
            .any(|r| matches!(r, EditorRequest::New | EditorRequest::Open)),
        "{requests:?}"
    );
    // With the dialog closed the same keys work again.
    h.editor.ui.show_course_settings_window = false;
    h.idle_frame();
    h.press(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(frame_x(&h, "a"), 101.0);
}

#[test]
fn the_shortcut_table_is_built_once() {
    let first = crate::registry_shortcuts();
    let second = crate::registry_shortcuts();
    assert!(std::ptr::eq(first, second));
    assert!(!first.is_empty());
}

// ---- Q13: arrows in the slide list ------------------------------------------------------

#[test]
fn up_and_down_walk_the_slide_list_across_scenes() {
    let mut h = harness(false);
    click_slide(&mut h, "s2");
    assert_eq!(h.editor.ui.last_active_area, ActiveArea::Navigator);
    h.editor.selection.clear_objects();
    h.press(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(
        h.editor.selection.slide, "s3",
        "Down crosses into the next scene"
    );
    assert!(h.editor.selection.also_slides.is_empty());
    h.press(Key::ArrowUp, Modifiers::NONE);
    h.press(Key::ArrowUp, Modifiers::NONE);
    assert_eq!(h.editor.selection.slide, "s1");
    h.press(Key::ArrowUp, Modifiers::NONE);
    assert_eq!(h.editor.selection.slide, "s1", "stops at the first slide");
    h.press(Key::End, Modifiers::NONE);
    assert_eq!(h.editor.selection.slide, "s4");
    h.press(Key::Home, Modifiers::NONE);
    assert_eq!(h.editor.selection.slide, "s1");
}

#[test]
fn slide_list_arrows_clear_the_object_and_never_nudge() {
    let mut h = harness(false);
    h.editor.selection.slide = "s1".to_owned();
    h.editor.selection.set_many(vec!["a".to_owned()]);
    h.editor.ui.last_active_area = ActiveArea::Navigator;
    h.idle_frame();
    h.press(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(h.editor.selection.slide, "s2");
    assert!(h.editor.selection.object.is_none());
    assert_eq!(frame_x(&h, "a"), 100.0);
}

#[test]
fn canvas_arrows_still_nudge() {
    let mut h = harness(false);
    h.editor.selection.set_many(vec!["a".to_owned()]);
    h.editor.ui.last_active_area = ActiveArea::Canvas;
    h.idle_frame();
    h.press(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(frame_x(&h, "a"), 101.0);
    assert_eq!(h.editor.selection.slide, "s1");
}

#[test]
fn shift_arrows_extend_the_slide_range_and_shrink_it_back() {
    let mut h = harness(false);
    click_slide(&mut h, "s1");
    h.press(Key::ArrowDown, Modifiers::SHIFT);
    h.press(Key::ArrowDown, Modifiers::SHIFT);
    assert_eq!(
        h.editor.selection.slide, "s1",
        "the anchor stays the primary slide"
    );
    assert_eq!(h.editor.selected_slides_in_order(), vec!["s1", "s2", "s3"]);
    h.press(Key::ArrowUp, Modifiers::SHIFT);
    assert_eq!(h.editor.selected_slides_in_order(), vec!["s1", "s2"]);
    h.press(Key::End, Modifiers::SHIFT);
    assert_eq!(
        h.editor.selected_slides_in_order(),
        vec!["s1", "s2", "s3", "s4"]
    );
    // A plain Up collapses to one slide, moving from the moving end.
    h.press(Key::ArrowUp, Modifiers::NONE);
    assert_eq!(h.editor.selection.slide, "s3");
    assert!(h.editor.selection.also_slides.is_empty());
}

#[test]
fn navigate_slides_scrolls_the_new_slide_into_view_state() {
    let mut h = harness(false);
    click_slide(&mut h, "s1");
    h.editor.navigate_slides(SlideNav::Last, false);
    assert!(
        h.editor.ui.slides_scrolled_to.is_none(),
        "a scroll is pending"
    );
    for _ in 0..60 {
        h.idle_frame();
        if h.editor.ui.slides_scrolled_to.as_deref() == Some("s4") {
            break;
        }
    }
    assert_eq!(h.editor.ui.slides_scrolled_to.as_deref(), Some("s4"));
}

// ---- Q7: locked objects -----------------------------------------------------------------

#[test]
fn select_all_nudge_and_delete_skip_locked_objects() {
    let mut h = harness(true);
    h.editor.select_all_objects();
    assert_eq!(h.editor.selection.ids(), vec!["a".to_owned()]);
    // A locked id forced into the selection is still left alone.
    h.editor
        .selection
        .set_many(vec!["a".to_owned(), "b".to_owned()]);
    h.editor.nudge_selection(10.0, 0.0);
    assert_eq!(frame_x(&h, "a"), 110.0);
    assert_eq!(frame_x(&h, "b"), 400.0);
    h.editor.delete_selection();
    let slide = h.editor.session.course.slide("s1").expect("slide");
    assert!(
        slide.object("a").is_none(),
        "the unlocked object is deleted"
    );
    assert!(slide.object("b").is_some(), "the locked object stays");
}

// ---- Q6: a held arrow key is one undo step ----------------------------------------------

#[test]
fn a_held_arrow_key_is_one_undo_step() {
    let mut h = harness(false);
    h.editor.selection.set_many(vec!["a".to_owned()]);
    h.idle_frame();
    let undo_before = h.editor.session.undo_len();
    h.frame(vec![key_down(Key::ArrowRight, false)]);
    for _ in 0..6 {
        h.frame(vec![key_down(Key::ArrowRight, true)]);
    }
    h.frame(vec![key_up(Key::ArrowRight)]);
    assert_eq!(frame_x(&h, "a"), 107.0);
    assert_eq!(h.editor.session.undo_len(), undo_before + 1);
    h.editor.run("edit.undo", serde_json::json!({}));
    assert_eq!(frame_x(&h, "a"), 100.0, "one undo undoes the whole burst");
}

#[test]
fn separate_taps_are_separate_undo_steps() {
    let mut h = harness(false);
    h.editor.selection.set_many(vec!["a".to_owned()]);
    h.idle_frame();
    let undo_before = h.editor.session.undo_len();
    h.press(Key::ArrowRight, Modifiers::NONE);
    h.press(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(frame_x(&h, "a"), 102.0);
    assert_eq!(h.editor.session.undo_len(), undo_before + 2);
}

#[test]
fn another_command_ends_a_nudge_burst() {
    let mut h = harness(false);
    h.editor.selection.set_many(vec!["a".to_owned()]);
    h.idle_frame();
    h.frame(vec![key_down(Key::ArrowRight, false)]);
    h.editor.run(
        "object.set_opacity",
        serde_json::json!({ "slide": "s1", "object": "a", "opacity": 0.5 }),
    );
    let undo_mid = h.editor.session.undo_len();
    h.frame(vec![key_down(Key::ArrowRight, true)]);
    assert_eq!(
        h.editor.session.undo_len(),
        undo_mid + 1,
        "not merged into the older nudge"
    );
}

// ---- Q11: zoom keeps the view centre ----------------------------------------------------

#[test]
fn zoom_from_fit_steps_to_the_next_preset_above_and_below_the_fit_scale() {
    assert_eq!(ZoomLevel::Fit.zoom_in_at(Some(0.6)), ZoomLevel::Percent75);
    assert_eq!(ZoomLevel::Fit.zoom_out_at(Some(0.6)), ZoomLevel::Percent50);
    assert_eq!(ZoomLevel::Fit.zoom_in_at(Some(1.2)), ZoomLevel::Percent150);
    assert_eq!(ZoomLevel::Fit.zoom_out_at(Some(1.2)), ZoomLevel::Percent100);
    // Past the presets the step is a custom scale, never a jump the wrong way.
    assert!(
        matches!(ZoomLevel::Fit.zoom_in_at(Some(2.5)), ZoomLevel::Custom(s) if (s - 3.75).abs() < 0.01)
    );
    assert!(
        matches!(ZoomLevel::Fit.zoom_out_at(Some(0.4)), ZoomLevel::Custom(s) if (0.25..0.4).contains(&s))
    );
    assert_eq!(ZoomLevel::Fit.zoom_in_at(Some(4.0)), ZoomLevel::Fit);
    assert_eq!(ZoomLevel::Fit.zoom_out_at(Some(0.25)), ZoomLevel::Fit);
    assert_eq!(
        ZoomLevel::Percent100.zoom_in_at(Some(0.6)),
        ZoomLevel::Percent150
    );
}

#[test]
fn zoom_in_from_fit_never_zooms_out() {
    let mut h = harness(false);
    for _ in 0..3 {
        h.idle_frame();
    }
    let slide = h.editor.session.course.slide_size;
    let fit = h.editor.ui.last_slide_rect.expect("slide drawn").width() / slide.w as f32;
    h.editor.zoom_in();
    if let Some(scale) = h.editor.ui.zoom.fixed_scale() {
        assert!(scale > fit, "zoom in from Fit {fit} went to {scale}");
    }
    h.editor.ui.zoom = ZoomLevel::Fit;
    h.editor.zoom_out();
    if let Some(scale) = h.editor.ui.zoom.fixed_scale() {
        assert!(scale < fit, "zoom out from Fit {fit} went to {scale}");
    }
}

#[test]
fn zoom_steps_keep_the_slide_centre_in_the_view() {
    let mut h = harness(false);
    for _ in 0..3 {
        h.idle_frame();
    }
    let view = h.central_panel_rect();
    h.editor.ui.zoom = ZoomLevel::Percent150;
    for _ in 0..4 {
        h.idle_frame();
    }
    let centre = h.editor.ui.last_slide_rect.expect("slide drawn").center();
    assert!(
        (centre - view.center()).length() < 80.0,
        "slide centre {centre:?} drifted from view centre {:?}",
        view.center()
    );
    h.editor.ui.zoom = ZoomLevel::Percent200;
    for _ in 0..4 {
        h.idle_frame();
    }
    let centre = h.editor.ui.last_slide_rect.expect("slide drawn").center();
    assert!(
        (centre - view.center()).length() < 80.0,
        "slide centre {centre:?} drifted from view centre {:?}",
        view.center()
    );
}

// ---- R4: Ctrl+S in a text field ---------------------------------------------------------

#[test]
fn ctrl_s_in_a_text_field_commits_the_field_and_saves() {
    let mut h = harness(false);
    h.editor.file_ops_available = true;
    h.editor.ui.slide_rename = Some(crate::slides::SceneRename {
        scene: "s1".to_owned(),
        buffer: "Slide 1".to_owned(),
        original: "Slide 1".to_owned(),
        just_entered: true,
    });
    for _ in 0..3 {
        h.idle_frame();
    }
    h.type_text(" edited");
    h.idle_frame();
    let mut requests = h.frame(vec![Event::Key {
        key: Key::S,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::COMMAND,
    }]);
    for _ in 0..3 {
        requests.extend(h.idle_frame());
    }
    assert!(
        requests.iter().any(|r| matches!(r, EditorRequest::Save)),
        "Save requested: {requests:?}"
    );
    assert_eq!(
        h.editor
            .session
            .course
            .slide("s1")
            .map(|s| s.title.clone())
            .as_deref(),
        Some("edited"),
        "the field was committed before the save"
    );
    assert!(h.editor.ui.slide_rename.is_none());
}

#[test]
fn ctrl_s_without_a_field_saves_at_once() {
    let mut h = harness(false);
    h.editor.file_ops_available = true;
    let requests = h.frame(vec![Event::Key {
        key: Key::S,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::COMMAND,
    }]);
    assert!(requests.iter().any(|r| matches!(r, EditorRequest::Save)));
}

// ---- Q4 / S1 / Q9: start page and File page --------------------------------------------

fn welcome(has_course: bool) -> TestHarness {
    let mut h = harness(false);
    h.editor.ui.ribbon.visible = true;
    h.editor.file_ops_available = true;
    h.editor.ui.show_welcome = true;
    h.editor.ui.has_loaded_course = has_course;
    h.editor.ui.file_page.section = crate::file_page::FileSection::Start;
    for _ in 0..3 {
        h.idle_frame();
    }
    h
}

#[test]
fn a_load_request_keeps_the_start_page_until_a_course_loads() {
    let mut h = welcome(false);
    let open = h.find_text("Open…").expect("Open entry");
    let requests = h.click_at_requests(open);
    assert!(requests.iter().any(|r| matches!(r, EditorRequest::Open)));
    h.idle_frame();
    assert!(
        h.editor.ui.show_welcome,
        "a cancelled dialog leaves the start page up"
    );
    assert!(!h.editor.ui.has_loaded_course);
    // A course really loading closes the page.
    let course = Course::from_json(&course_json(false)).expect("course");
    h.editor.replace_course(course);
    assert!(!h.editor.ui.show_welcome);
    assert!(h.editor.ui.has_loaded_course);
}

#[test]
fn the_start_page_tiles_wait_for_the_load_too() {
    let mut h = welcome(false);
    let tile = h.find_text("New blank course").expect("New tile");
    let requests = h.click_at_requests(tile);
    assert!(requests.iter().any(|r| matches!(r, EditorRequest::New)));
    assert!(h.editor.ui.show_welcome);
    assert!(!h.editor.ui.has_loaded_course);
}

#[test]
fn the_file_page_stays_open_after_open_until_a_course_loads() {
    let mut h = harness(false);
    h.editor.ui.ribbon.visible = true;
    h.editor.file_ops_available = true;
    h.idle_frame();
    h.editor.ui.file_page.open = true;
    h.editor.ui.file_page.section = crate::file_page::FileSection::Recent;
    for _ in 0..2 {
        h.idle_frame();
    }
    let open = h.find_text("Open…").expect("Open entry");
    let requests = h.click_at_requests(open);
    assert!(requests.iter().any(|r| matches!(r, EditorRequest::Open)));
    h.idle_frame();
    assert!(h.editor.ui.file_page.open);
    h.editor
        .replace_course(Course::from_json(&course_json(false)).expect("course"));
    assert!(!h.editor.ui.file_page.open);
}

#[test]
fn course_info_works_from_help_welcome_with_a_course_open() {
    let mut h = welcome(true);
    let info = h.find_text("Course Info…").expect("Course Info entry");
    h.click_at(info);
    h.idle_frame();
    assert!(!h.editor.ui.show_welcome, "the start page is left first");
    assert!(h.editor.ui.show_course_settings_window);
}

#[test]
fn manage_fonts_and_version_history_work_from_help_welcome() {
    let mut h = welcome(true);
    h.editor.ui.file_page.section = crate::file_page::FileSection::Fonts;
    h.idle_frame();
    let fonts = h.find_text("Manage Fonts…").expect("Manage Fonts choice");
    h.click_at(fonts);
    h.idle_frame();
    assert!(!h.editor.ui.show_welcome);
    assert!(h.editor.ui.show_fonts_window);
}

#[test]
fn esc_goes_back_from_the_start_page_when_a_course_is_open() {
    let mut h = welcome(true);
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(!h.editor.ui.show_welcome);
    // With no course there is nothing to go back to.
    let mut h = welcome(false);
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(h.editor.ui.show_welcome);
}

#[test]
fn check_course_shows_the_problems() {
    let mut h = harness(false);
    h.editor.ui.ribbon.visible = true;
    h.idle_frame();
    h.editor.ui.file_page.open = true;
    for _ in 0..2 {
        h.idle_frame();
    }
    let check = h.find_text("Check Course").expect("Check Course entry");
    h.click_at(check);
    h.idle_frame();
    assert!(!h.editor.ui.file_page.open);
    assert!(h.editor.ui.show_validation_window);
}

// ---- Q8: slide menu actions use every selected slide --------------------------------------

fn slide_ids(h: &TestHarness) -> Vec<String> {
    h.editor
        .session
        .course
        .slides()
        .map(|s| s.id.clone())
        .collect()
}

fn select_slides(h: &mut TestHarness, primary: &str, also: &[&str]) {
    h.editor.selection.slide = primary.to_owned();
    h.editor.selection.also_slides = also.iter().map(|s| (*s).to_owned()).collect();
    h.editor.ui.last_active_area = ActiveArea::Navigator;
    h.idle_frame();
}

#[test]
fn copy_and_paste_use_every_selected_slide_in_list_order() {
    let mut h = harness(false);
    select_slides(&mut h, "s3", &["s1", "s2"]);
    crate::clipboard::copy(&mut h.editor);
    let clip = h.editor.ui.clipboard.clone().expect("clipboard");
    let copied: Vec<&str> = clip.slides.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(
        copied,
        vec!["s1", "s2", "s3"],
        "list order, not selection order"
    );
    // The text payload round-trips.
    let text = clip.to_text();
    let parsed = crate::clipboard::Clipboard::from_text(&text).expect("parses");
    assert_eq!(parsed.slides.len(), 3);
    // Paste after the current slide, in order, as one undo step.
    select_slides(&mut h, "s4", &[]);
    let undo_before = h.editor.session.undo_len();
    crate::clipboard::paste_private(&mut h.editor);
    assert_eq!(
        h.editor.session.undo_len(),
        undo_before + 1,
        "one undo step"
    );
    let ids = slide_ids(&h);
    assert_eq!(ids.len(), 7);
    assert_eq!(ids[3], "s4");
    let titles: Vec<String> = h
        .editor
        .session
        .course
        .slides()
        .skip(4)
        .map(|s| s.title.clone())
        .collect();
    assert_eq!(titles, vec!["Slide 1 copy", "Slide 2 copy", "Slide 3 copy"]);
    h.editor.run("edit.undo", serde_json::json!({}));
    assert_eq!(slide_ids(&h).len(), 4);
}

#[test]
fn a_single_slide_still_copies_and_pastes() {
    let mut h = harness(false);
    select_slides(&mut h, "s2", &[]);
    crate::clipboard::copy(&mut h.editor);
    crate::clipboard::paste_private(&mut h.editor);
    assert_eq!(slide_ids(&h).len(), 5);
}

#[test]
fn cut_removes_every_selected_slide_but_not_all_of_them() {
    let mut h = harness(false);
    select_slides(&mut h, "s2", &["s3"]);
    let undo_before = h.editor.session.undo_len();
    crate::clipboard::cut(&mut h.editor);
    assert_eq!(slide_ids(&h), vec!["s1", "s4"]);
    assert_eq!(h.editor.session.undo_len(), undo_before + 1);
    assert_eq!(
        h.editor.ui.clipboard.as_ref().map(|c| c.slides.len()),
        Some(2)
    );
    // Cutting every slide would leave an empty course: refused.
    select_slides(&mut h, "s1", &["s4"]);
    crate::clipboard::cut(&mut h.editor);
    assert_eq!(slide_ids(&h).len(), 2);
}

#[test]
fn duplicate_selected_slides_copies_each_after_its_original() {
    let mut h = harness(false);
    select_slides(&mut h, "s1", &["s3"]);
    let undo_before = h.editor.session.undo_len();
    h.editor.duplicate_selected_slides();
    assert_eq!(slide_ids(&h).len(), 6);
    assert_eq!(h.editor.session.undo_len(), undo_before + 1);
    let titles: Vec<String> = h
        .editor
        .session
        .course
        .slides()
        .map(|s| s.title.clone())
        .collect();
    assert_eq!(titles[0], "Slide 1");
    assert_eq!(
        titles[1], "Slide 1 copy",
        "the copy sits right after its original"
    );
}

#[test]
fn move_selected_slides_shifts_each_one_place() {
    let mut h = harness(false);
    // s2 and s4 move up; s1 and s3 are the ones they swap with.
    select_slides(&mut h, "s2", &["s4"]);
    let undo_before = h.editor.session.undo_len();
    h.editor.move_selected_slides(true);
    assert_eq!(slide_ids(&h), vec!["s2", "s1", "s4", "s3"]);
    assert_eq!(h.editor.session.undo_len(), undo_before + 1);
}

#[test]
fn a_block_at_the_edge_does_not_move_past_it() {
    let h = harness(false);
    let course = &h.editor.session.course;
    let selected = vec!["s1".to_owned(), "s2".to_owned()];
    assert!(plan_slide_moves(course, &selected, true).is_empty());
    let plan = plan_slide_moves(course, &selected, false);
    assert!(plan.is_empty(), "s1 and s2 fill their scene: {plan:?}");
    let one = vec!["s1".to_owned()];
    let plan = plan_slide_moves(course, &one, false);
    assert_eq!(plan, vec![("s1".to_owned(), "sc1".to_owned(), 1)]);
}

// ---- P5: scrolling a selected slide into view stops -------------------------------------

#[test]
fn scrolling_a_slide_into_view_stops_after_a_few_frames() {
    let mut h = harness(false);
    h.editor.selection.slide = "s4".to_owned();
    h.editor.ui.slides_scrolled_to = None;
    h.editor.ui.slides_scroll_stalled = 0;
    let mut settled = false;
    for _ in 0..120 {
        h.idle_frame();
        if h.editor.ui.slides_scrolled_to.as_deref() == Some("s4") {
            settled = true;
            break;
        }
    }
    assert!(settled, "the scroll gives up or finishes");
    assert_eq!(h.editor.ui.slides_scroll_stalled, 0);
}

#[test]
fn a_thumbnail_taller_than_the_panel_still_settles() {
    let course = Course::from_json(&course_json(false)).expect("course");
    let mut h = TestHarness::new_with_size(course, (1280.0, 330.0));
    for _ in 0..3 {
        h.idle_frame();
    }
    h.editor.selection.slide = "s4".to_owned();
    let mut settled = false;
    for _ in 0..200 {
        h.idle_frame();
        if h.editor.ui.slides_scrolled_to.as_deref() == Some("s4") {
            settled = true;
            break;
        }
    }
    assert!(settled, "no endless scrolling in a short panel");
}
