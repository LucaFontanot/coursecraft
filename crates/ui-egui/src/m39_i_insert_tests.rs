//! M39 package I: Insert tab text items (Slide Number, Symbol, Reference), the shared
//! "insert this text" helper, Escape and `dialog::any_open`.

use crate::canvas::InlineEdit;
use crate::insert_text::{self, SYMBOL_GROUPS};
use crate::ribbon::spec::{self, Action, Kind, Menu, UiAction};
use crate::test_harness::TestHarness;
use coursecraft_course::{Course, ObjectKind, ValueKind};
use egui::{FontFamily, FontId, Key, Modifiers};

fn course() -> Course {
    serde_json::from_value(serde_json::json!({
        "schema_version": 1,
        "id": "m39-i",
        "title": "M39 I",
        "slide_size": { "w": 1280, "h": 720 },
        "variables": [
            { "name": "Score", "value": 3.0 },
            { "name": "Passed", "value": true },
            { "name": "Learner_Alias", "value": "x" },
            { "name": "bad name", "value": "x" },
            { "name": "100%", "value": "x" }
        ],
        "scenes": [{
            "id": "sc1",
            "title": "Main",
            "slides": [{
                "id": "s1",
                "title": "Slide 1",
                "objects": [{
                    "id": "t1",
                    "frame": { "x": 100.0, "y": 100.0, "w": 400.0, "h": 80.0 },
                    "kind": { "type": "text", "text": "Hello world" }
                }]
            }]
        }]
    }))
    .expect("valid course")
}

fn harness() -> TestHarness {
    let mut h = TestHarness::new(course());
    h.editor.selection.slide = "s1".to_owned();
    h.idle_frame();
    h
}

fn text_of(h: &TestHarness, id: &str) -> Option<String> {
    let slide = h.editor.session.course.slide("s1")?;
    let (object, _) = slide.object(id)?;
    match &object.kind {
        ObjectKind::Text { text, .. } => Some(text.clone()),
        _ => None,
    }
}

fn last_text(h: &TestHarness) -> String {
    let slide = h.editor.session.course.slide("s1").expect("slide");
    match &slide.objects.last().expect("an object").kind {
        ObjectKind::Text { text, .. } => text.clone(),
        _ => panic!("the last object is a text box"),
    }
}

fn object_count(h: &TestHarness) -> usize {
    h.editor
        .session
        .course
        .slide("s1")
        .map_or(0, |s| s.objects.len())
}

fn start_edit(h: &mut TestHarness, buffer: &str) {
    h.editor.selection.object = Some("t1".to_owned());
    h.editor.ui.inline_edit = Some(InlineEdit {
        slide: "s1".to_owned(),
        object: "t1".to_owned(),
        buffer: buffer.to_owned(),
        original: buffer.to_owned(),
        is_button: false,
        is_shape: false,
        cell: None,
        just_entered: false,
        live_runs: Vec::new(),
        live_paragraphs: Vec::new(),
        last_revision: 0,
        state_name: None,
        extra_id: None,
    });
    h.idle_frame();
}

fn set_caret(h: &mut TestHarness, start: usize, end: usize) {
    let id = egui::Id::unique(("canvas_inline_edit", "s1", "t1"));
    let mut state = egui::TextEdit::load_state(&h.ctx, id).unwrap_or_default();
    state
        .cursor
        .set_char_range(Some(egui::text_selection::CCursorRange::two(
            egui::text::CCursor::new(start),
            egui::text::CCursor::new(end),
        )));
    state.store(&h.ctx, id);
}

fn buffer(h: &TestHarness) -> String {
    h.editor
        .ui
        .inline_edit
        .as_ref()
        .map(|e| e.buffer.clone())
        .unwrap_or_default()
}

/// A button of the open Symbol or Reference window (the toolbar has an "Insert" menu too).
fn window_button(h: &mut TestHarness, label: &str) -> egui::Pos2 {
    let ins = &h.editor.ui.insert_text;
    let rect = ins
        .symbol_rect
        .or(ins.reference_rect)
        .expect("a window is open");
    h.find_text_in(label, rect).expect("the button is drawn")
}

fn insert_button(h: &mut TestHarness) -> egui::Pos2 {
    window_button(h, "Insert")
}

fn undo(h: &mut TestHarness) {
    h.editor.run("edit.undo", serde_json::json!({}));
}

// ---------------------------------------------------------------- shared helper

#[test]
fn insert_without_edit_makes_a_selected_text_box_at_the_centre_in_one_undo_step() {
    let mut h = harness();
    let before = object_count(&h);
    assert!(insert_text::insert_text(&mut h.editor, "©"));
    assert_eq!(object_count(&h), before + 1);
    let slide = h.editor.session.course.slide("s1").expect("slide");
    let new = slide.objects.last().expect("new object");
    assert_eq!(last_text(&h), "©");
    let center = new.frame.x + new.frame.w * 0.5;
    assert!(
        (center - 640.0).abs() <= 1.0,
        "centred horizontally: {center}"
    );
    assert_eq!(h.editor.selection.object.as_deref(), Some(new.id.as_str()));
    undo(&mut h);
    assert_eq!(object_count(&h), before, "one undo removes the box");
}

#[test]
fn insert_with_a_text_object_selected_but_not_edited_still_makes_a_new_box() {
    let mut h = harness();
    h.editor.selection.object = Some("t1".to_owned());
    let before = object_count(&h);
    assert!(insert_text::insert_text(&mut h.editor, "×"));
    assert_eq!(object_count(&h), before + 1);
    assert_eq!(text_of(&h, "t1").as_deref(), Some("Hello world"));
}

#[test]
fn insert_empty_text_does_nothing() {
    let mut h = harness();
    let before = object_count(&h);
    assert!(!insert_text::insert_text(&mut h.editor, ""));
    assert_eq!(object_count(&h), before);
}

#[test]
fn insert_while_editing_goes_in_at_the_caret_and_commits_as_one_step() {
    let mut h = harness();
    start_edit(&mut h, "Hello world");
    set_caret(&mut h, 5, 5);
    assert!(insert_text::insert_text(&mut h.editor, "°"));
    h.idle_frame();
    assert_eq!(buffer(&h), "Hello° world");
    assert_eq!(object_count(&h), 1, "no new box while editing");
    crate::canvas::commit_inline_edit(&mut h.editor);
    assert_eq!(text_of(&h, "t1").as_deref(), Some("Hello° world"));
    undo(&mut h);
    assert_eq!(
        text_of(&h, "t1").as_deref(),
        Some("Hello world"),
        "one undo step for the whole edit"
    );
}

#[test]
fn insert_while_editing_replaces_the_selection() {
    let mut h = harness();
    start_edit(&mut h, "Hello world");
    set_caret(&mut h, 6, 11);
    insert_text::insert_text(&mut h.editor, "→");
    h.idle_frame();
    assert_eq!(buffer(&h), "Hello →");
}

#[test]
fn insert_while_editing_without_a_caret_appends() {
    let mut h = harness();
    start_edit(&mut h, "Hi");
    insert_text::insert_text(&mut h.editor, "™");
    h.idle_frame();
    assert_eq!(buffer(&h), "Hi™");
}

#[test]
fn two_inserts_in_a_row_follow_the_caret() {
    let mut h = harness();
    start_edit(&mut h, "ab");
    set_caret(&mut h, 1, 1);
    insert_text::insert_text(&mut h.editor, "1");
    h.idle_frame();
    insert_text::insert_text(&mut h.editor, "2");
    h.idle_frame();
    assert_eq!(buffer(&h), "a12b");
}

#[test]
fn splice_handles_multibyte_text() {
    let ctx = egui::Context::default();
    let id = egui::Id::unique("splice_test");
    let mut state = egui::TextEdit::load_state(&ctx, id).unwrap_or_default();
    state
        .cursor
        .set_char_range(Some(egui::text_selection::CCursorRange::two(
            egui::text::CCursor::new(1),
            egui::text::CCursor::new(2),
        )));
    state.store(&ctx, id);
    let mut buf = "αβγ".to_owned();
    insert_text::splice_into_buffer(&ctx, id, &mut buf, "★");
    assert_eq!(buf, "α★γ");
    let after = egui::TextEdit::load_state(&ctx, id).expect("state");
    assert_eq!(
        after.cursor.char_range().map(|r| r.primary.index.0),
        Some(2)
    );
}

// ---------------------------------------------------------------- slide number

#[test]
fn slide_number_is_a_split_button_with_two_entries() {
    let items: Vec<_> = spec::all_items().collect();
    let item = items
        .iter()
        .find(|i| i.id == "insert.slide_number")
        .expect("item");
    assert!(matches!(item.kind, Kind::LargeSplit));
    assert!(matches!(
        item.action,
        Action::Ui(UiAction::InsertSlideNumber)
    ));
    let Menu::Items(entries) = item.menu else {
        panic!("a menu of items");
    };
    assert_eq!(entries.len(), 2);
    assert!(matches!(
        entries[0].action,
        Action::Ui(UiAction::InsertSlideNumber)
    ));
    assert!(matches!(
        entries[1].action,
        Action::Ui(UiAction::InsertSlideNumberOfTotal)
    ));
}

#[test]
fn symbol_and_reference_items_are_wired() {
    let items: Vec<_> = spec::all_items().collect();
    let symbol = items
        .iter()
        .find(|i| i.id == "insert.symbol")
        .expect("symbol");
    assert!(matches!(symbol.action, Action::Ui(UiAction::InsertSymbol)));
    let reference = items
        .iter()
        .find(|i| i.id == "insert.reference")
        .expect("reference");
    assert!(matches!(
        reference.action,
        Action::Ui(UiAction::InsertReference)
    ));
}

#[test]
fn slide_number_inserts_the_reference_in_a_new_box() {
    let mut h = harness();
    assert!(insert_text::insert_slide_number(&mut h.editor, false));
    assert_eq!(last_text(&h), "%Course.SlideNumber%");
}

#[test]
fn slide_number_of_total_inserts_both_references_at_the_caret() {
    let mut h = harness();
    start_edit(&mut h, "Page ");
    set_caret(&mut h, 5, 5);
    insert_text::insert_slide_number(&mut h.editor, true);
    h.idle_frame();
    assert_eq!(
        buffer(&h),
        "Page %Course.SlideNumber% / %Course.TotalSlides%"
    );
}

#[test]
fn slide_number_references_are_built_in_variables() {
    for reference in [
        insert_text::SLIDE_NUMBER_REF,
        insert_text::SLIDE_NUMBER_OF_TOTAL_REF,
    ] {
        let spans = coursecraft_course::variable_reference_spans(reference);
        assert!(!spans.is_empty());
        for span in spans {
            let name = &reference[span.start + 1..span.end - 1];
            assert!(
                coursecraft_course::builtin_variable_kind(name).is_some(),
                "{name} is built in"
            );
        }
    }
}

#[test]
fn slide_number_controls_need_a_slide_to_insert_into() {
    let mut h = harness();
    assert!(insert_text::available(&h.editor).is_ok());
    h.editor.selection.slide = String::new();
    assert!(insert_text::available(&h.editor).is_err());
    assert!(!insert_text::insert_slide_number(&mut h.editor, false));
}

// ---------------------------------------------------------------- symbols

#[test]
fn every_offered_symbol_is_drawn_by_the_bundled_fonts() {
    let ctx = egui::Context::default();
    coursecraft_render::install_fonts(&ctx);
    // The fonts apply at the start of the next pass.
    for _ in 0..2 {
        ctx.run_ui(egui::RawInput::default(), |_| {})
            .drop_without_applying_deltas();
    }
    // Inter first, then the bundled fallbacks: the families the editor and the slides use.
    let families = [FontFamily::Proportional, FontFamily::Name("inter".into())];
    let mut missing = Vec::new();
    for group in SYMBOL_GROUPS {
        for &(c, _) in group.symbols {
            for family in &families {
                let id = FontId::new(18.0, family.clone());
                if !ctx.fonts_mut(|f| f.has_glyph(&id, c)) {
                    missing.push(format!(
                        "{c}{}",
                        if *family == FontFamily::Proportional {
                            ""
                        } else {
                            "(inter)"
                        }
                    ));
                }
            }
        }
    }
    assert!(missing.is_empty(), "glyphs the fonts lack: {missing:?}");
}

#[test]
fn symbol_set_covers_the_planned_characters() {
    let all: String = SYMBOL_GROUPS
        .iter()
        .flat_map(|g| g.symbols.iter().map(|&(c, _)| c))
        .collect();
    for c in "©®™°±×÷€£¥§¶•…—–«»¿¡←→↑↓✓●".chars() {
        assert!(all.contains(c), "{c} is offered");
    }
    let mut seen = std::collections::HashSet::new();
    for group in SYMBOL_GROUPS {
        assert!(!group.symbols.is_empty(), "{} has symbols", group.label);
        for &(c, _) in group.symbols {
            assert!(seen.insert(c), "{c} is listed twice");
        }
    }
}

#[test]
fn symbol_search_matches_names_and_characters() {
    let found = insert_text::matching_symbols("euro", 0);
    assert!(found.iter().any(|&(c, _)| c == '€'));
    let found = insert_text::matching_symbols("ARROW", 3);
    assert!(found.iter().any(|&(c, _)| c == '→'), "searches all groups");
    let found = insert_text::matching_symbols("€", 3);
    assert_eq!(found.len(), 1);
    assert!(insert_text::matching_symbols("zzzz-none", 0).is_empty());
    let group = insert_text::matching_symbols("", 3);
    assert_eq!(group.len(), SYMBOL_GROUPS[3].symbols.len());
}

#[test]
fn symbol_window_opens_inserts_and_remembers_recent() {
    let mut h = harness();
    insert_text::open_symbol_picker(&mut h.editor);
    h.idle_frame();
    assert!(h.editor.ui.insert_text.symbol_open);
    assert!(crate::dialog::any_open(&h.ctx));
    assert!(h.editor.modal_dialog_open(), "global keys stay blocked");
    let pos = h.find_text("€").expect("the euro button is drawn");
    h.click_at(pos);
    assert_eq!(h.editor.ui.insert_text.symbol_selected, Some('€'));
    let before = object_count(&h);
    let insert = insert_button(&mut h);
    h.click_at(insert);
    assert_eq!(object_count(&h), before + 1);
    assert_eq!(last_text(&h), "€");
    assert_eq!(h.editor.ui.insert_text.recent, vec!['€']);
    assert!(
        h.editor.ui.insert_text.symbol_open,
        "the window stays open to insert more"
    );
}

#[test]
fn recently_used_is_newest_first_without_duplicates_and_capped() {
    let mut h = harness();
    insert_text::open_symbol_picker(&mut h.editor);
    h.idle_frame();
    let symbols: Vec<char> = SYMBOL_GROUPS[0]
        .symbols
        .iter()
        .map(|&(c, _)| c)
        .take(insert_text::RECENT_MAX + 4)
        .collect();
    for &c in &symbols {
        h.editor.ui.insert_text.symbol_selected = Some(c);
        h.idle_frame();
        let insert = insert_button(&mut h);
        h.click_at(insert);
    }
    // Insert the first one again: it moves to the front.
    h.editor.ui.insert_text.symbol_selected = Some(symbols[0]);
    h.idle_frame();
    let insert = insert_button(&mut h);
    h.click_at(insert);
    let recent = &h.editor.ui.insert_text.recent;
    assert_eq!(recent.len(), insert_text::RECENT_MAX);
    let unique: std::collections::HashSet<_> = recent.iter().collect();
    assert_eq!(unique.len(), recent.len());
    assert_eq!(recent.first(), Some(&symbols[0]));
    assert_eq!(recent.get(1), symbols.last());
}

#[test]
fn symbol_window_inserts_into_the_text_being_edited() {
    let mut h = harness();
    start_edit(&mut h, "Hello world");
    set_caret(&mut h, 5, 5);
    insert_text::open_symbol_picker(&mut h.editor);
    h.idle_frame();
    h.editor.ui.insert_text.symbol_selected = Some('©');
    h.idle_frame();
    let insert = insert_button(&mut h);
    h.click_at(insert);
    h.idle_frame();
    assert!(
        h.editor.ui.inline_edit.is_some(),
        "clicking the window keeps the edit"
    );
    assert_eq!(buffer(&h), "Hello© world");
    assert_eq!(object_count(&h), 1);
}

#[test]
fn escape_closes_the_symbol_window_and_any_open_follows() {
    let mut h = harness();
    insert_text::open_symbol_picker(&mut h.editor);
    h.idle_frame();
    assert!(crate::dialog::any_open(&h.ctx));
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    h.idle_frame();
    assert!(!h.editor.ui.insert_text.symbol_open);
    assert!(!crate::dialog::any_open(&h.ctx));
    assert!(!h.editor.modal_dialog_open());
}

#[test]
fn escape_closes_the_window_but_not_the_edit_behind_it() {
    let mut h = harness();
    start_edit(&mut h, "Hello world");
    insert_text::open_symbol_picker(&mut h.editor);
    h.idle_frame();
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(!h.editor.ui.insert_text.symbol_open);
    assert!(h.editor.ui.inline_edit.is_some(), "the edit goes on");
    // With no window open, Escape cancels the edit as before.
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(h.editor.ui.inline_edit.is_none());
}

// ---------------------------------------------------------------- reference

#[test]
fn reference_names_follow_the_reference_rules() {
    assert!(insert_text::valid_reference_name("Score"));
    assert!(insert_text::valid_reference_name("Course.SlideNumber"));
    assert!(insert_text::valid_reference_name("a-b_c.1"));
    assert!(!insert_text::valid_reference_name(""));
    assert!(!insert_text::valid_reference_name("bad name"));
    assert!(!insert_text::valid_reference_name("100%"));
    assert!(!insert_text::valid_reference_name("%x"));
    assert!(!insert_text::valid_reference_name(&"a".repeat(129)));
}

#[test]
fn reference_entries_list_built_in_then_course_variables_without_invalid_names() {
    let entries = insert_text::reference_entries(&course());
    let builtin = coursecraft_course::BUILTIN_VARIABLES.len();
    assert!(entries[..builtin].iter().all(|e| e.builtin));
    assert!(entries[builtin..].iter().all(|e| !e.builtin));
    let names: Vec<&str> = entries[builtin..].iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["Score", "Passed", "Learner_Alias"]);
    assert_eq!(entries[builtin].kind, ValueKind::Number);
    assert_eq!(entries[builtin + 1].kind, ValueKind::Bool);
    assert_eq!(entries[builtin + 2].kind, ValueKind::Text);
    assert!(entries.iter().any(|e| e.name == "Course.TotalSlides"));
}

#[test]
fn reference_window_filters_and_inserts_the_percent_form() {
    let mut h = harness();
    insert_text::open_reference_picker(&mut h.editor);
    h.idle_frame();
    assert!(crate::dialog::any_open(&h.ctx));
    h.editor.ui.insert_text.reference_filter = "score".to_owned();
    h.idle_frame();
    assert!(h.find_text_containing("Course.ScorePoints").is_some());
    assert!(
        h.find_text_containing("Learner.Name").is_none(),
        "filtered out"
    );
    let row = h
        .find_text_containing("Course.ScorePercent")
        .expect("the row");
    h.click_at(row);
    assert_eq!(
        h.editor.ui.insert_text.reference_selected.as_deref(),
        Some("Course.ScorePercent")
    );
    let before = object_count(&h);
    let insert = insert_button(&mut h);
    h.click_at(insert);
    assert_eq!(object_count(&h), before + 1);
    assert_eq!(last_text(&h), "%Course.ScorePercent%");
}

#[test]
fn reference_window_never_lists_invalid_names() {
    let mut h = harness();
    insert_text::open_reference_picker(&mut h.editor);
    h.idle_frame();
    assert!(h.find_text_containing("Learner_Alias").is_some());
    assert!(h.find_text_containing("bad name").is_none());
    assert!(h.find_text_containing("100%").is_none());
}

#[test]
fn reference_window_inserts_into_the_text_being_edited() {
    let mut h = harness();
    start_edit(&mut h, "Score: ");
    set_caret(&mut h, 7, 7);
    insert_text::open_reference_picker(&mut h.editor);
    h.idle_frame();
    h.editor.ui.insert_text.reference_selected = Some("Score".to_owned());
    h.idle_frame();
    let insert = insert_button(&mut h);
    h.click_at(insert);
    h.idle_frame();
    assert_eq!(buffer(&h), "Score: %Score%");
}

#[test]
fn escape_closes_the_reference_window() {
    let mut h = harness();
    insert_text::open_reference_picker(&mut h.editor);
    h.idle_frame();
    assert!(h.editor.modal_dialog_open());
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    h.idle_frame();
    assert!(!h.editor.ui.insert_text.reference_open);
    assert!(!crate::dialog::any_open(&h.ctx));
}

#[test]
fn close_button_closes_each_window() {
    let mut h = harness();
    insert_text::open_symbol_picker(&mut h.editor);
    h.idle_frame();
    let close = window_button(&mut h, "Close");
    h.click_at(close);
    assert!(!h.editor.ui.insert_text.symbol_open);
    insert_text::open_reference_picker(&mut h.editor);
    h.idle_frame();
    let close = window_button(&mut h, "Close");
    h.click_at(close);
    assert!(!h.editor.ui.insert_text.reference_open);
}
