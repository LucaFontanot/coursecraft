//! M27 Step D-8: Find & Replace (D11.1–D11.10) and Accessibility Panel (D12.1–D12.10).

use crate::properties::InspectorTab;
use crate::status::navigate_to_issue_path;
use crate::test_harness::TestHarness;
use crate::timeline::BottomTab;
use coursecraft_course::{Course, ObjectKind, Slide};
use egui::{Key, Modifiers};

fn base_course() -> Course {
    serde_json::from_value(serde_json::json!({
        "schema_version": 1,
        "id": "m27-find-a11y",
        "title": "M27 Find & Accessibility",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [
            {
                "id": "scene-1",
                "title": "Scene 1",
                "slides": [
                    { "id": "s1", "title": "Slide 1", "duration_ms": 5000, "objects": [] },
                    { "id": "s2", "title": "Slide 2", "duration_ms": 5000, "objects": [] }
                ]
            }
        ]
    }))
    .expect("base_course")
}

fn setup_harness(course: Course) -> TestHarness {
    let mut h = TestHarness::new(course);
    h.editor.ui.show_welcome = false;
    h.editor.selection.slide = "s1".into();
    h.editor.selection.also_slides.clear();
    h.editor.selection.layer = None;
    h.editor.selection.clear_objects();
    h.idle_frame();
    h.idle_frame();
    h
}

fn add_text(h: &mut TestHarness, slide: &str, id: &str, text: &str, y: f64) {
    h.editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": slide,
                "id": id,
                "kind": { "type": "text", "text": text },
                "frame": { "x": 40.0, "y": y, "w": 400.0, "h": 60.0 }
            }),
        )
        .expect("object.add text");
}

fn open_find_with_shortcut(h: &mut TestHarness) {
    h.press(Key::F, Modifiers::COMMAND);
    for _ in 0..3 {
        h.idle_frame();
    }
    assert!(h.editor.ui.find.open, "Find window should be open");
}

// ---------------------------------------------------------------------------
// D11. Find & Replace (10 Variations)
// ---------------------------------------------------------------------------

#[test]
fn d11_1_match_case_and_whole_word_live_counts() {
    let mut h = setup_harness(base_course());
    add_text(&mut h, "s1", "t1", "cat", 40.0);
    add_text(&mut h, "s1", "t2", "Cat", 110.0);
    add_text(&mut h, "s1", "t3", "category", 180.0);
    add_text(&mut h, "s1", "t4", "CAT", 250.0);

    open_find_with_shortcut(&mut h);
    h.editor.ui.find.query = "cat".into();
    h.idle_frame();
    assert!(
        h.find_text("4 matches on 1 slide").is_some(),
        "expected 4 matches initially"
    );

    // Toggle Match case -> "cat" and "category" (2 matches).
    let mc_pos = h.find_text("Match case").expect("Match case checkbox");
    h.click_at(mc_pos);
    h.idle_frame();
    assert!(h.editor.ui.find.case_sensitive);
    assert!(
        h.find_text("2 matches on 1 slide").is_some(),
        "expected 2 matches with Match case"
    );

    // Toggle Whole word -> "cat" only (1 match).
    let ww_pos = h.find_text("Whole word").expect("Whole word checkbox");
    h.click_at(ww_pos);
    h.idle_frame();
    assert!(h.editor.ui.find.whole_word);
    assert!(
        h.find_text("1 match on 1 slide").is_some(),
        "expected 1 match with Match case + Whole word"
    );

    // Clear query -> 0 hits.
    h.editor.ui.find.query.clear();
    h.idle_frame();
    assert!(
        h.find_text("Type to search every string in the course.")
            .is_some()
    );
}

#[test]
fn d11_2_replace_single_vs_replace_all_and_undo_clears_status() {
    let mut h = setup_harness(base_course());
    add_text(&mut h, "s1", "t1", "old one", 40.0);
    add_text(&mut h, "s1", "t2", "old two", 110.0);
    add_text(&mut h, "s1", "t3", "old three", 180.0);

    open_find_with_shortcut(&mut h);
    h.editor.ui.find.query = "old".into();
    h.editor.ui.find.replacement = "new".into();
    h.idle_frame();
    assert!(h.find_text("3 matches on 1 slide").is_some());

    let undo_before = h.editor.session.undo_len();

    // Click Replace on hit #1.
    let replace_pos = h.find_text_last("Replace").expect("Replace button");
    h.click_at(replace_pos);
    h.idle_frame();
    assert_eq!(h.editor.session.undo_len(), undo_before + 1);
    assert_eq!(h.editor.ui.find.message.as_deref(), Some("Replaced 1."));

    let s1 = h.editor.session.course.slide("s1").expect("s1");
    let text_of = |id: &str| match &s1.object(id).expect(id).0.kind {
        ObjectKind::Text { text, .. } => text.clone(),
        _ => panic!("expected text"),
    };
    assert_eq!(text_of("t1"), "new one");
    assert_eq!(text_of("t2"), "old two");
    assert_eq!(text_of("t3"), "old three");

    // Click Replace All for remaining 2.
    let replace_all_pos = h.find_text("Replace All").expect("Replace All button");
    h.click_at(replace_all_pos);
    h.idle_frame();
    assert_eq!(h.editor.session.undo_len(), undo_before + 2);
    assert_eq!(h.editor.ui.find.message.as_deref(), Some("Replaced 2."));

    // Undo x 2 reverts cleanly and clears stale status message.
    h.press(Key::Z, Modifiers::COMMAND);
    h.idle_frame();
    assert!(
        h.editor.ui.find.message.is_none(),
        "undo must clear stale Replaced status"
    );
    h.press(Key::Z, Modifiers::COMMAND);
    h.idle_frame();

    let s1 = h.editor.session.course.slide("s1").expect("s1");
    for (id, expected) in [("t1", "old one"), ("t2", "old two"), ("t3", "old three")] {
        match &s1.object(id).expect(id).0.kind {
            ObjectKind::Text { text, .. } => assert_eq!(text, expected),
            _ => panic!("expected text"),
        }
    }
}

#[test]
fn d11_3_variable_reference_protection() {
    let mut h = setup_harness(base_course());
    add_text(
        &mut h,
        "s1",
        "t1",
        "Value: %score% and %Course.ScorePercent% score",
        40.0,
    );

    open_find_with_shortcut(&mut h);
    h.editor.ui.find.query = "score".into();
    h.editor.ui.find.replacement = "points".into();
    h.idle_frame();

    let replace_all_pos = h.find_text("Replace All").expect("Replace All button");
    h.click_at(replace_all_pos);
    h.idle_frame();

    let s1 = h.editor.session.course.slide("s1").expect("s1");
    match &s1.object("t1").expect("t1").0.kind {
        ObjectKind::Text { text, .. } => {
            assert_eq!(text, "Value: %score% and %Course.ScorePercent% points");
        }
        _ => panic!("expected text"),
    }
    assert_eq!(
        h.editor.ui.find.message.as_deref(),
        Some("Replaced 1. Skipped 2 inside variable references.")
    );
}

#[test]
fn d11_4_multi_run_rich_text_formatting_preserved_on_replace() {
    let mut h = setup_harness(base_course());
    add_text(&mut h, "s1", "rich", "Hello bold world", 40.0);
    h.editor
        .run(
            "text.style_range",
            serde_json::json!({
                "slide": "s1",
                "object": "rich",
                "start": 6,
                "end": 10,
                "bold": true,
                "color": "#FF0000",
                "size": 32.0
            }),
        )
        .expect("style bold");
    h.editor
        .run(
            "text.style_range",
            serde_json::json!({
                "slide": "s1",
                "object": "rich",
                "start": 10,
                "end": 16,
                "italic": true
            }),
        )
        .expect("style italic");

    open_find_with_shortcut(&mut h);
    h.editor.ui.find.query = "bold".into();
    h.editor.ui.find.replacement = "STRONG".into();
    h.idle_frame();

    let replace_pos = h.find_text_last("Replace").expect("Replace button");
    h.click_at(replace_pos);
    h.idle_frame();

    let s1 = h.editor.session.course.slide("s1").expect("s1");
    match &s1.object("rich").expect("rich").0.kind {
        ObjectKind::Text { text, runs, .. } => {
            assert_eq!(text, "Hello STRONG world");
            let joined: String = runs.iter().map(|r| r.text.as_str()).collect();
            assert_eq!(joined, *text);
            let strong_run = runs
                .iter()
                .find(|r| r.text == "STRONG")
                .expect("STRONG run");
            assert_eq!(strong_run.bold, Some(true));
            assert_eq!(
                strong_run.color.as_ref().map(|c| c.0.as_str()),
                Some("#FF0000")
            );
            assert_eq!(strong_run.size, Some(32.0));
            let world_run = runs.iter().find(|r| r.text == " world").expect("world run");
            assert_eq!(world_run.italic, Some(true));
        }
        _ => panic!("expected text"),
    }
}

#[test]
fn d11_5_clicking_hits_across_all_10_target_scopes() {
    let mut course = base_course();
    course.title = "Course needle title".into();
    let mut h = TestHarness::new_with_size(course, (1280.0, 960.0));
    h.editor.ui.show_welcome = false;
    h.editor.selection.slide = "s1".into();
    h.editor.selection.also_slides.clear();
    h.editor.selection.layer = None;
    h.editor.selection.clear_objects();
    h.idle_frame();
    h.idle_frame();

    // 1. Base text box
    add_text(&mut h, "s1", "base_t", "base needle", 40.0);

    // 2. Slide layer object
    let layer_id = h
        .editor
        .run(
            "layer.add",
            serde_json::json!({ "slide": "s1", "name": "Tip Layer" }),
        )
        .expect("layer.add")["id"]
        .as_str()
        .expect("layer id")
        .to_owned();
    h.editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "layer": layer_id,
                "id": "layer_t",
                "kind": { "type": "text", "text": "layer needle" },
                "frame": { "x": 40.0, "y": 40.0, "w": 300.0, "h": 50.0 }
            }),
        )
        .expect("layer text");

    // 3. Master slide object
    let master_id = h
        .editor
        .run("master.add", serde_json::json!({ "title": "Corp" }))
        .expect("master.add")["id"]
        .as_str()
        .expect("master id")
        .to_owned();
    h.editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": master_id,
                "id": "master_t",
                "kind": { "type": "text", "text": "master needle" },
                "frame": { "x": 40.0, "y": 40.0, "w": 300.0, "h": 50.0 }
            }),
        )
        .expect("master text");

    // 4. Scene title
    h.editor
        .run(
            "scene.update",
            serde_json::json!({ "scene": "scene-1", "title": "Scene needle" }),
        )
        .expect("scene.update");

    // 5. Course title already has "needle".

    // 6. Question prompt/choice on s2
    h.editor
        .run(
            "question.set",
            serde_json::json!({
                "slide": "s2",
                "question": {
                    "kind": "multiple_choice",
                    "prompt": "Question needle prompt",
                    "choices": [
                        { "id": "c1", "text": "Option A", "correct": true },
                        { "id": "c2", "text": "Option B", "correct": false }
                    ]
                }
            }),
        )
        .expect("question.set");

    // 7. Slide notes
    h.editor
        .run(
            "slide.update",
            serde_json::json!({ "slide": "s1", "notes": "Notes needle" }),
        )
        .expect("slide.update notes");

    // 8. Table cell
    h.editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "id": "tbl1",
                "kind": {
                    "type": "table",
                    "columns": [0.5, 0.5],
                    "rows": [{ "height": 1.0, "cells": [{ "text": "cell needle" }, { "text": "b" }] }]
                },
                "frame": { "x": 40.0, "y": 140.0, "w": 300.0, "h": 80.0 }
            }),
        )
        .expect("table");

    // 9. Marker title/body
    h.editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "id": "mk1",
                "kind": {
                    "type": "marker",
                    "title": "Marker needle",
                    "body": "Body"
                },
                "frame": { "x": 400.0, "y": 140.0, "w": 36.0, "h": 36.0 }
            }),
        )
        .expect("marker");

    // 10. Glossary term/definition (not indexed by course.find, which covers the 9 scopes above).
    h.editor
        .run(
            "glossary.add",
            serde_json::json!({ "term": "Glossary needle", "definition": "Def" }),
        )
        .expect("glossary.add");

    open_find_with_shortcut(&mut h);
    h.editor.ui.find.query = "needle".into();
    for _ in 0..3 {
        h.idle_frame();
    }

    let snippets: Vec<(String, Option<String>, Option<String>, String)> = {
        let reply = h
            .editor
            .run(
                "course.find",
                serde_json::json!({ "query": "needle", "case": false, "whole_word": false }),
            )
            .expect("course.find");
        let hits: Vec<coursecraft_course::Hit> =
            serde_json::from_value(reply["hits"].clone()).expect("hits");
        assert!(hits.len() >= 9, "expected >= 9 hits, got {}", hits.len());
        hits.into_iter()
            .map(|hit| (hit.snippet, hit.slide, hit.object, hit.field))
            .collect()
    };

    let list_pos = h
        .find_text("Find and Replace")
        .map(|p| egui::pos2(p.x, p.y + 220.0))
        .expect("Find and Replace window");
    for (idx, (snippet, expected_slide, expected_obj, field)) in snippets.iter().enumerate() {
        if idx == 5 {
            h.scroll_over(list_pos, -160.0);
            h.idle_frame();
        }
        let pos = h
            .find_text_last(snippet)
            .unwrap_or_else(|| panic!("snippet not visible in Find list: {snippet}"));
        h.click_at(pos);
        h.idle_frame();

        if let Some(slide_id) = expected_slide {
            assert_eq!(&h.editor.selection.slide, slide_id, "field={field}");
            assert_eq!(
                h.editor.ui.master_view,
                h.editor.session.course.master(slide_id).is_some(),
                "master_view mismatch for {field}"
            );
            if let Some(obj_id) = expected_obj {
                assert_eq!(
                    h.editor.selection.object.as_deref(),
                    Some(obj_id.as_str()),
                    "object mismatch for {field}"
                );
            }
        } else if field.starts_with("scene/") {
            assert_eq!(h.editor.selection.slide, "s1");
        } else {
            assert_eq!(h.editor.ui.inspector_tab, InspectorTab::Course);
        }
        if field.starts_with("question/") {
            assert_eq!(h.editor.ui.bottom_tab, BottomTab::Question);
        }
    }
}

#[test]
fn d11_6_clicking_find_result_exits_story_view() {
    let mut h = setup_harness(base_course());
    add_text(&mut h, "s2", "s2_t", "target phrase", 40.0);

    // Enter Story View (Ctrl+Shift+M).
    h.press(Key::M, Modifiers::COMMAND | Modifiers::SHIFT);
    h.idle_frame();
    assert!(h.editor.ui.story_view, "must be in Story View");

    // Open Find and click hit on Slide 2.
    open_find_with_shortcut(&mut h);
    h.editor.ui.find.query = "target phrase".into();
    for _ in 0..3 {
        h.idle_frame();
    }

    let hit_pos = h
        .find_text_last("target phrase")
        .expect("hit snippet in Find");
    h.click_at(hit_pos);
    h.idle_frame();

    assert!(
        !h.editor.ui.story_view,
        "BUG-D8: clicking Find result must exit Story View"
    );
    assert_eq!(h.editor.selection.slide, "s2");
    assert_eq!(h.editor.selection.object.as_deref(), Some("s2_t"));
}

#[test]
fn d11_7_empty_replacement_and_max_text_bytes_guard() {
    let mut h = setup_harness(base_course());
    add_text(&mut h, "s1", "t1", "hello foo world", 40.0);

    open_find_with_shortcut(&mut h);
    h.editor.ui.find.query = "foo".into();
    h.editor.ui.find.replacement = String::new();
    h.idle_frame();

    // (a) Empty replacement deletes cleanly.
    let replace_all_pos = h.find_text("Replace All").expect("Replace All");
    h.click_at(replace_all_pos);
    h.idle_frame();
    let s1 = h.editor.session.course.slide("s1").expect("s1");
    match &s1.object("t1").expect("t1").0.kind {
        ObjectKind::Text { text, .. } => assert_eq!(text, "hello  world"),
        _ => panic!("expected text"),
    }

    // (b) 5000-byte replacement must disable Replace / Replace All without mutating or erroring.
    h.editor.ui.find.query = "hello".into();
    h.editor.ui.find.replacement = "x".repeat(5000);
    h.idle_frame();

    let err_count_before = h.editor.ui.error_history.len();
    let rev_before = h.editor.session.revision;
    if let Some(pos) = h.find_text("Replace All") {
        h.click_at(pos);
        h.idle_frame();
    }
    assert_eq!(
        h.editor.session.revision, rev_before,
        "BUG-D8: >4096 byte replacement must not mutate course"
    );
    assert_eq!(
        h.editor.ui.error_history.len(),
        err_count_before,
        "BUG-D8: Replace All button should be disabled so no doomed command error fires"
    );
}

#[test]
fn d11_8_self_containing_replacement_terminates() {
    let mut h = setup_harness(base_course());
    add_text(&mut h, "s1", "t1", "aaa", 40.0);

    open_find_with_shortcut(&mut h);
    h.editor.ui.find.query = "a".into();
    h.editor.ui.find.replacement = "aa".into();
    h.idle_frame();

    let pos = h.find_text("Replace All").expect("Replace All");
    h.click_at(pos);
    h.idle_frame();

    let s1 = h.editor.session.course.slide("s1").expect("s1");
    match &s1.object("t1").expect("t1").0.kind {
        ObjectKind::Text { text, .. } => assert_eq!(text, "aaaaaa"),
        _ => panic!("expected text"),
    }
}

#[test]
fn d11_9_regex_metacharacters_and_unicode_case_folding() {
    let mut h = setup_harness(base_course());
    add_text(&mut h, "s1", "t1", "[a-z].* (foo) $1\nabc", 40.0);
    add_text(&mut h, "s1", "t2", "café and CAFÉ", 120.0);

    open_find_with_shortcut(&mut h);
    h.editor.ui.find.query = "[a-z].*".into();
    h.idle_frame();
    assert!(
        h.find_text("1 match on 1 slide").is_some(),
        "regex metachars must be treated literally"
    );

    h.editor.ui.find.query = "café".into();
    h.idle_frame();
    assert!(
        h.find_text("2 matches on 1 slide").is_some(),
        "case-insensitive Unicode should match café and CAFÉ"
    );

    let mc_pos = h.find_text("Match case").expect("Match case");
    h.click_at(mc_pos);
    h.idle_frame();
    assert!(
        h.find_text("1 match on 1 slide").is_some(),
        "case-sensitive Unicode should match only café"
    );
}

#[test]
fn d11_10_max_hits_truncation_and_escape_close_reopen() {
    let mut h = setup_harness(base_course());
    // > 1000 hits (MAX_HITS = 1000) across 1005 text objects.
    for i in 0..1005 {
        add_text(&mut h, "s1", &format!("i{i}"), "item", 40.0);
    }
    h.editor.selection.object = Some("i0".into());

    open_find_with_shortcut(&mut h);
    h.editor.ui.find.query = "item".into();
    for _ in 0..3 {
        h.idle_frame();
    }
    assert!(
        h.find_text("Showing the first 1000 matches.").is_some(),
        "MAX_HITS truncation notice expected"
    );

    let report = h.layout_report(2);
    assert!(
        report.findings.is_empty(),
        "Find window layout_report clean: {report:?}"
    );

    // Esc closes Find without clearing object selection.
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(!h.editor.ui.find.open);
    assert_eq!(h.editor.selection.object.as_deref(), Some("i0"));

    // Reopen via Ctrl+F.
    open_find_with_shortcut(&mut h);
    assert!(h.editor.ui.find.focus_query || h.editor.ui.find.open);
}

// ---------------------------------------------------------------------------
// D12. Accessibility Panel — Check Course & Alt Text (10 Variations)
// ---------------------------------------------------------------------------

#[test]
fn d12_1_open_check_course_from_status_chip_and_severity_counts() {
    let course: Course = serde_json::from_value(serde_json::json!({
        "schema_version": 1,
        "id": "m27-find-a11y",
        "title": "M27 Find & Accessibility",
        "start_slide": "no_such_slide",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [
            {
                "id": "scene-1",
                "title": "Scene 1",
                "slides": [
                    {
                        "id": "s1",
                        "title": "Slide 1",
                        "duration_ms": 5000,
                        "objects": []
                    }
                ]
            }
        ]
    }))
    .expect("course with invalid start_slide");
    let mut h = setup_harness(course);
    // Missing alt text (error)
    h.editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "id": "img1",
                "kind": { "type": "image", "src": "pic.png" },
                "frame": { "x": 20.0, "y": 20.0, "w": 200.0, "h": 150.0 }
            }),
        )
        .expect("img1");
    // Low contrast + small text (<16px) + text over image (note)
    h.editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "id": "txt1",
                "kind": { "type": "text", "text": "Faint small", "style": { "color": "#EEEEEE", "size": 11.0 } },
                "frame": { "x": 30.0, "y": 30.0, "w": 150.0, "h": 40.0 }
            }),
        )
        .expect("txt1");
    // Audio without captions (warning)
    h.editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "id": "aud1",
                "kind": { "type": "audio", "src": "clip.mp3" },
                "frame": { "x": 300.0, "y": 20.0, "w": 40.0, "h": 40.0 }
            }),
        )
        .expect("aud1");

    h.editor.refresh_validation();
    h.idle_frame();

    // Click status bar chip.
    let chip_pos = h
        .find_text_containing("1 error")
        .expect("status bar error chip");
    h.click_at(chip_pos);
    h.idle_frame();
    assert!(h.editor.ui.show_validation_window);

    assert!(h.find_text("Accessibility").is_some());
    assert!(
        h.find_text("Automated checks only. These still need a person:")
            .is_some()
    );
    assert!(h.find_text_containing("error [").is_some());
    assert!(h.find_text_containing("warning [").is_some());
    assert!(h.find_text_containing("note [").is_some());

    let report = h.layout_report(2);
    assert!(
        report.findings.is_empty(),
        "Check Course layout_report clean: {report:?}"
    );
}

#[test]
fn d12_2_deferred_audit_debounce_vs_immediate_refresh_while_open() {
    let mut h = setup_harness(base_course());
    h.editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "id": "img1",
                "kind": { "type": "image", "src": "a.png" },
                "frame": { "x": 20.0, "y": 20.0, "w": 100.0, "h": 100.0 }
            }),
        )
        .expect("img1");
    h.editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "id": "img2",
                "kind": { "type": "image", "src": "b.png" },
                "frame": { "x": 140.0, "y": 20.0, "w": 100.0, "h": 100.0 }
            }),
        )
        .expect("img2");
    h.editor.refresh_validation();
    h.idle_frame();
    let alt_count = |h: &TestHarness| {
        h.editor
            .ui
            .audit_findings
            .iter()
            .filter(|f| f.check == "alt-text")
            .count()
    };
    assert_eq!(alt_count(&h), 2);

    // Fix img1 while Check Course is closed -> deferred 300ms.
    h.editor
        .run(
            "object.update",
            serde_json::json!({
                "slide": "s1",
                "object": "img1",
                "kind": { "type": "image", "src": "a.png", "alt": "First image" }
            }),
        )
        .expect("fix img1");
    h.idle_frame();
    h.advance_time(0.1);
    h.idle_frame();
    assert_eq!(alt_count(&h), 2, "audit should still be deferred at 0.1s");

    h.advance_time(0.35);
    h.idle_frame();
    assert_eq!(alt_count(&h), 1, "audit should run after 0.35s");

    // Reopen Check Course -> mark img2 decorative -> updates on very next frame!
    h.editor.ui.show_validation_window = true;
    h.idle_frame();
    h.editor
        .run(
            "object.update",
            serde_json::json!({ "slide": "s1", "object": "img2", "decorative": true }),
        )
        .expect("decorative img2");
    h.idle_frame();
    assert_eq!(
        alt_count(&h),
        0,
        "audit must refresh immediately when Check Course is open"
    );
}

#[test]
fn d12_3_clicking_findings_across_base_layer_and_master() {
    let mut h = setup_harness(base_course());
    h.editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "id": "base_img",
                "kind": { "type": "image", "src": "a.png" },
                "frame": { "x": 20.0, "y": 20.0, "w": 100.0, "h": 100.0 }
            }),
        )
        .expect("base_img");

    let layer_id = h
        .editor
        .run(
            "layer.add",
            serde_json::json!({ "slide": "s1", "name": "L1" }),
        )
        .expect("layer.add")["id"]
        .as_str()
        .expect("layer id")
        .to_owned();
    h.editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "layer": layer_id,
                "id": "layer_btn",
                "kind": { "type": "button", "label": "" },
                "frame": { "x": 20.0, "y": 140.0, "w": 100.0, "h": 40.0 }
            }),
        )
        .expect("layer_btn");

    let master_id = h
        .editor
        .run("master.add", serde_json::json!({ "title": "M1" }))
        .expect("master.add")["id"]
        .as_str()
        .expect("master id")
        .to_owned();
    h.editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": master_id,
                "id": "master_img",
                "kind": { "type": "image", "src": "m.png" },
                "frame": { "x": 20.0, "y": 260.0, "w": 100.0, "h": 100.0 }
            }),
        )
        .expect("master_img");

    h.editor.ui.show_validation_window = true;
    h.idle_frame();

    for (needle, exp_slide, exp_layer, exp_obj, exp_master) in [
        ("base_img", "s1", None, "base_img", false),
        (
            "layer_btn",
            "s1",
            Some(layer_id.as_str()),
            "layer_btn",
            false,
        ),
        ("master_img", master_id.as_str(), None, "master_img", true),
    ] {
        let pos = h
            .find_text_containing(needle)
            .unwrap_or_else(|| panic!("missing finding for {needle}"));
        h.click_at(pos);
        h.idle_frame();
        assert_eq!(h.editor.selection.slide, exp_slide);
        assert_eq!(h.editor.selection.layer.as_deref(), exp_layer);
        assert_eq!(h.editor.selection.object.as_deref(), Some(exp_obj));
        assert_eq!(h.editor.ui.master_view, exp_master);
    }
}

#[test]
fn d12_4_clicking_play_slide_finding_exits_master_and_story_view() {
    let mut h = setup_harness(base_course());
    h.editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "id": "s1_img",
                "kind": { "type": "image", "src": "a.png" },
                "frame": { "x": 20.0, "y": 20.0, "w": 100.0, "h": 100.0 }
            }),
        )
        .expect("s1_img");

    // 1. Start in Master View.
    h.editor.enter_master_view();
    h.editor.ui.show_validation_window = true;
    for _ in 0..3 {
        h.idle_frame();
    }
    assert!(h.editor.ui.master_view);

    let pos = h
        .find_text_containing("The image has no alt text.")
        .expect("finding for s1_img");
    h.click_at(pos);
    for _ in 0..3 {
        h.idle_frame();
    }
    assert!(
        !h.editor.ui.master_view,
        "BUG-D8: clicking play-slide finding must exit Master View"
    );
    assert_eq!(h.editor.selection.slide, "s1");
    assert_eq!(h.editor.selection.object.as_deref(), Some("s1_img"));

    // 2. Start in Story View.
    h.press(Key::M, Modifiers::COMMAND | Modifiers::SHIFT);
    for _ in 0..3 {
        h.idle_frame();
    }
    assert!(h.editor.ui.story_view);

    let pos = h
        .find_text_containing("The image has no alt text.")
        .expect("finding for s1_img");
    h.click_at(pos);
    for _ in 0..3 {
        h.idle_frame();
    }
    assert!(
        !h.editor.ui.story_view,
        "BUG-D8: clicking play-slide finding must exit Story View"
    );
    assert_eq!(h.editor.selection.slide, "s1");
}

#[test]
fn d12_5_clicking_course_level_finding_opens_course_inspector() {
    let mut course = base_course();
    course.title = String::new();
    let mut h = setup_harness(course);
    h.editor.ui.inspector_tab = InspectorTab::Slide;
    h.editor.ui.show_validation_window = true;
    for _ in 0..3 {
        h.idle_frame();
    }

    let pos = h
        .find_text_containing("The course title is empty.")
        .expect("course-title finding row");
    h.click_at(pos);
    h.idle_frame();

    assert_eq!(
        h.editor.ui.inspector_tab,
        InspectorTab::Course,
        "BUG-D8: clicking course-level finding must switch to InspectorTab::Course"
    );
}

#[test]
fn d12_6_bank_slide_validation_error_navigation() {
    let mut h = setup_harness(base_course());
    let bank_id = h
        .editor
        .run("bank.create", serde_json::json!({ "title": "Unit 1" }))
        .expect("bank.create")["id"]
        .as_str()
        .expect("bank id")
        .to_owned();
    let bank_slide_id = h
        .editor
        .run(
            "bank.add_question",
            serde_json::json!({
                "bank": bank_id,
                "question": {
                    "kind": "multiple_choice",
                    "prompt": "Q?",
                    "choices": [
                        { "id": "c1", "text": "Yes", "correct": true },
                        { "id": "c2", "text": "No", "correct": false }
                    ]
                }
            }),
        )
        .expect("bank.add_question")["id"]
        .as_str()
        .expect("bank slide id")
        .to_owned();

    assert!(
        navigate_to_issue_path(
            &h.editor.session.course,
            &mut h.editor.selection,
            "/banks/0/slides/0/question",
        ),
        "BUG-D8: navigate_to_issue_path must handle /banks/{{bi}}/slides/{{si}}/..."
    );
    assert_eq!(h.editor.selection.slide, bank_slide_id);
}

fn add_tiny_png(h: &mut TestHarness, key: &str) {
    let pixels = vec![200_u8; 4 * 4 * 4];
    let png = coursecraft_media::encode_png_rgba(4, 4, &pixels).expect("png");
    h.editor
        .media
        .push((key.to_owned(), std::sync::Arc::from(png)));
}

#[test]
fn d12_7_suggest_alt_text_gating_and_batch_apply_undo() {
    let mut h = setup_harness(base_course());
    h.editor.set_ai_config(Some(coursecraft_ai::AiConfig {
        base_url: "http://localhost:1234/v1".into(),
        model: "test-model".into(),
        api_key: String::new(),
        speech_to_text_model: "whisper-1".into(),
        text_to_speech_model: "tts-1".into(),
        voice: "alloy".into(),
    }));
    add_tiny_png(&mut h, "a.png");
    add_tiny_png(&mut h, "b.png");
    h.editor.ui.show_validation_window = true;
    h.idle_frame();
    assert!(
        h.find_text("Suggest alt text…").is_none(),
        "Suggest alt text button must be hidden when no alt-text findings exist"
    );

    h.editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "id": "img1",
                "kind": { "type": "image", "src": "a.png" },
                "frame": { "x": 20.0, "y": 20.0, "w": 100.0, "h": 100.0 }
            }),
        )
        .expect("img1");
    h.editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "id": "img2",
                "kind": { "type": "image", "src": "b.png" },
                "frame": { "x": 140.0, "y": 20.0, "w": 100.0, "h": 100.0 }
            }),
        )
        .expect("img2");
    h.idle_frame();

    let btn = h
        .find_text("Suggest alt text…")
        .expect("Suggest alt text button should appear");
    h.click_at(btn);
    h.editor.ui.show_validation_window = false;
    for _ in 0..3 {
        h.idle_frame();
    }
    assert!(h.editor.ui.alt.open);
    assert_eq!(h.editor.ui.alt.rows.len(), 2);

    // Edit manual alt text for both images and click Apply.
    h.editor.ui.alt.rows[0].suggestion = "Diagram of A".into();
    h.editor.ui.alt.rows[0].ticked = true;
    h.editor.ui.alt.rows[1].suggestion = "Diagram of B".into();
    h.editor.ui.alt.rows[1].ticked = true;
    for _ in 0..3 {
        h.idle_frame();
    }

    let undo_before = h.editor.session.undo_len();
    let apply_pos = h.find_text("Apply").expect("Apply button");
    h.click_at(apply_pos);
    h.idle_frame();
    h.advance_time(0.35);
    h.idle_frame();

    assert_eq!(
        h.editor.session.undo_len(),
        undo_before + 1,
        "batch apply must be 1 undo step"
    );
    assert!(
        !h.editor
            .ui
            .audit_findings
            .iter()
            .any(|f| f.check == "alt-text"),
        "both alt-text findings should be cleared"
    );

    h.editor.ui.alt.open = false;
    h.idle_frame();
    h.press(Key::Z, Modifiers::COMMAND);
    h.idle_frame();
    h.advance_time(0.35);
    h.idle_frame();
    assert_eq!(
        h.editor
            .ui
            .audit_findings
            .iter()
            .filter(|f| f.check == "alt-text")
            .count(),
        2
    );
}

#[test]
fn d12_8_all_13_audit_checks_triggered_and_cleared() {
    let mut course = base_course();
    course.title = String::new(); // 1. course-title
    if let Some(Slide { title, .. }) = course.slide_mut("s1") {
        *title = String::new(); // 2. slide-title
    }
    let mut h = setup_harness(course);

    // 3. alt-text, 12. focus-order (duplicate 1)
    h.editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "id": "img1",
                "kind": { "type": "image", "src": "photo.png" },
                "frame": { "x": 10.0, "y": 10.0, "w": 300.0, "h": 200.0 }
            }),
        )
        .expect("img1");
    h.editor
        .run(
            "object.update",
            serde_json::json!({ "slide": "s1", "object": "img1", "focus_order": 1 }),
        )
        .expect("img1 focus_order");
    // 4. control-name (empty label), 8. target-size (< 24x24), 13. decorative-interactive
    h.editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "id": "btn1",
                "kind": { "type": "button", "label": "" },
                "frame": { "x": 400.0, "y": 10.0, "w": 16.0, "h": 16.0 }
            }),
        )
        .expect("btn1");
    h.editor
        .run(
            "object.update",
            serde_json::json!({ "slide": "s1", "object": "btn1", "decorative": true, "focus_order": 1 }),
        )
        .expect("btn1 props");
    // 4. control-name (empty label), 8. target-size (< 24x24), 12. focus-order (duplicate 1)
    h.editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "id": "btn2",
                "kind": { "type": "button", "label": "" },
                "frame": { "x": 400.0, "y": 80.0, "w": 16.0, "h": 16.0 }
            }),
        )
        .expect("btn2");
    h.editor
        .run(
            "object.update",
            serde_json::json!({ "slide": "s1", "object": "btn2", "focus_order": 1 }),
        )
        .expect("btn2 focus_order");
    // 5. contrast (not over image)
    h.editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "id": "txt2",
                "kind": { "type": "text", "text": "Low contrast", "style": { "color": "#EEEEEE", "size": 18.0 } },
                "frame": { "x": 500.0, "y": 20.0, "w": 150.0, "h": 40.0 }
            }),
        )
        .expect("txt2");
    // 6. contrast-unknown (over image), 7. text-size (< 16px)
    h.editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "id": "txt1",
                "kind": { "type": "text", "text": "Low tiny", "style": { "color": "#EEEEEE", "size": 10.0 } },
                "frame": { "x": 20.0, "y": 20.0, "w": 150.0, "h": 40.0 }
            }),
        )
        .expect("txt1");
    // 9. video-captions, 11. autoplay-control
    h.editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "id": "vid1",
                "kind": { "type": "video", "src": "v.mp4", "autoplay": true },
                "frame": { "x": 10.0, "y": 240.0, "w": 200.0, "h": 120.0 }
            }),
        )
        .expect("vid1");
    // 10. audio-captions
    h.editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "id": "aud1",
                "kind": { "type": "audio", "src": "a.mp3" },
                "frame": { "x": 240.0, "y": 240.0, "w": 40.0, "h": 40.0 }
            }),
        )
        .expect("aud1");

    h.editor.ui.show_validation_window = true;
    h.idle_frame();

    let checks: std::collections::BTreeSet<&str> = h
        .editor
        .ui
        .audit_findings
        .iter()
        .map(|f| f.check.as_str())
        .collect();
    for expected in [
        "course-title",
        "slide-title",
        "alt-text",
        "control-name",
        "contrast",
        "contrast-unknown",
        "text-size",
        "target-size",
        "video-captions",
        "audio-captions",
        "autoplay-control",
        "focus-order",
        "decorative-interactive",
    ] {
        assert!(
            checks.contains(expected),
            "missing {expected} in {checks:?}"
        );
    }

    // Fix all except contrast-unknown (text over image).
    h.editor
        .run(
            "course.update",
            serde_json::json!({ "title": "Fixed Title" }),
        )
        .expect("fix course title");
    h.editor
        .run(
            "slide.update",
            serde_json::json!({ "slide": "s1", "title": "Fixed Slide" }),
        )
        .expect("fix slide title");
    h.editor
        .run(
            "object.update",
            serde_json::json!({ "slide": "s1", "object": "img1", "kind": { "type": "image", "src": "photo.png", "alt": "Photo" } }),
        )
        .expect("fix alt");
    h.editor
        .run(
            "object.update",
            serde_json::json!({
                "slide": "s1",
                "object": "btn1",
                "kind": { "type": "button", "label": "Next" },
                "decorative": false,
                "focus_order": 2
            }),
        )
        .expect("fix btn1");
    h.editor
        .run(
            "object.set_frame",
            serde_json::json!({
                "slide": "s1",
                "object": "btn1",
                "frame": { "x": 400.0, "y": 10.0, "w": 80.0, "h": 40.0 }
            }),
        )
        .expect("resize btn1");
    h.editor
        .run(
            "object.update",
            serde_json::json!({
                "slide": "s1",
                "object": "btn2",
                "kind": { "type": "button", "label": "Prev" },
                "focus_order": 3
            }),
        )
        .expect("fix btn2");
    h.editor
        .run(
            "object.set_frame",
            serde_json::json!({
                "slide": "s1",
                "object": "btn2",
                "frame": { "x": 400.0, "y": 80.0, "w": 80.0, "h": 40.0 }
            }),
        )
        .expect("resize btn2");
    h.editor
        .run(
            "object.update",
            serde_json::json!({
                "slide": "s1",
                "object": "txt2",
                "kind": { "type": "text", "text": "Low contrast", "style": { "color": "#111111", "size": 18.0 } }
            }),
        )
        .expect("fix txt2");
    h.editor
        .run(
            "object.update",
            serde_json::json!({
                "slide": "s1",
                "object": "txt1",
                "kind": { "type": "text", "text": "Low tiny", "style": { "color": "#111111", "size": 18.0 } }
            }),
        )
        .expect("fix txt1");
    h.editor
        .run(
            "object.update",
            serde_json::json!({
                "slide": "s1",
                "object": "vid1",
                "kind": { "type": "video", "src": "v.mp4", "autoplay": false, "captions": "v.vtt" }
            }),
        )
        .expect("fix vid1");
    h.editor
        .run(
            "object.update",
            serde_json::json!({
                "slide": "s1",
                "object": "aud1",
                "kind": { "type": "audio", "src": "a.mp3", "captions": "a.vtt" }
            }),
        )
        .expect("fix aud1");

    h.idle_frame();
    let remaining: Vec<&str> = h
        .editor
        .ui
        .audit_findings
        .iter()
        .map(|f| f.check.as_str())
        .collect();
    assert_eq!(
        remaining,
        vec!["contrast-unknown"],
        "only contrast-unknown note should remain"
    );
}

#[test]
fn d12_9_duplicate_focus_order_and_decorative_interactive_sync() {
    let mut h = setup_harness(base_course());
    h.editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "id": "b1",
                "kind": { "type": "button", "label": "One" },
                "frame": { "x": 20.0, "y": 20.0, "w": 80.0, "h": 40.0 }
            }),
        )
        .expect("b1");
    h.editor
        .run(
            "object.update",
            serde_json::json!({ "slide": "s1", "object": "b1", "focus_order": 1 }),
        )
        .expect("b1 focus_order");
    h.editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "id": "b2",
                "kind": { "type": "button", "label": "Two" },
                "frame": { "x": 120.0, "y": 20.0, "w": 80.0, "h": 40.0 }
            }),
        )
        .expect("b2");
    h.editor
        .run(
            "object.update",
            serde_json::json!({ "slide": "s1", "object": "b2", "focus_order": 1 }),
        )
        .expect("b2 focus_order");

    h.editor.ui.show_validation_window = true;
    h.idle_frame();
    assert!(
        h.editor
            .ui
            .audit_findings
            .iter()
            .any(|f| f.check == "focus-order")
    );

    h.editor
        .run(
            "object.update",
            serde_json::json!({ "slide": "s1", "object": "b2", "focus_order": 2 }),
        )
        .expect("fix focus_order");
    h.idle_frame();
    assert!(
        !h.editor
            .ui
            .audit_findings
            .iter()
            .any(|f| f.check == "focus-order")
    );

    h.editor
        .run(
            "object.update",
            serde_json::json!({ "slide": "s1", "object": "b1", "decorative": true }),
        )
        .expect("set decorative");
    h.idle_frame();
    assert!(
        h.editor
            .ui
            .audit_findings
            .iter()
            .any(|f| f.check == "decorative-interactive")
    );

    h.press(Key::Z, Modifiers::COMMAND);
    h.idle_frame();
    assert!(
        !h.editor
            .ui
            .audit_findings
            .iter()
            .any(|f| f.check == "decorative-interactive")
    );
}

#[test]
fn d12_10_error_history_window_and_status_bar_auto_expiry() {
    let mut h = setup_harness(base_course());
    for i in 0..5 {
        assert!(
            h.editor
                .run(
                    "slide.update",
                    serde_json::json!({ "slide": format!("missing_{i}"), "title": "X" }),
                )
                .is_none()
        );
    }
    assert_eq!(h.editor.ui.error_history.len(), 5);
    let err_msg = h
        .editor
        .ui
        .last_error
        .as_ref()
        .map(|(m, _)| m.clone())
        .expect("last_error");

    h.idle_frame();
    let err_btn = h
        .find_text(&err_msg)
        .expect("status bar error button should be visible");
    h.click_at(err_btn);
    h.idle_frame();
    assert!(h.editor.ui.show_error_history_window);

    for i in 1..=5 {
        assert!(
            h.find_text_containing(&format!("{i}. ")).is_some(),
            "missing entry {i} in Error History"
        );
    }
    let report = h.layout_report(2);
    assert!(
        report.findings.is_empty(),
        "Error History layout_report clean: {report:?}"
    );

    // Advance virtual clock 6.0s -> status bar banner clears while Error History retains entries.
    h.advance_time(6.0);
    h.idle_frame();
    assert!(
        h.find_text(&err_msg).is_none(),
        "status bar error banner must auto-expire after 5s"
    );
    assert_eq!(h.editor.ui.error_history.len(), 5);
}
