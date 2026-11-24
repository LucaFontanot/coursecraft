//! M21 step 5 guard rails for the normal test run.
//!
//! Timing tests are too noisy for CI, so these count work instead of clocks.
//! All counters live in `guard_counters` (editor work) and in
//! `coursecraft_render::media::test_counts` (picture decodes), and are compiled
//! out of release builds. See `docs/performance.md` for what each one protects.
//! The counters only exist in debug builds, so these tests do too.
#![cfg(debug_assertions)]

use std::sync::Arc;

use crate::guard_counters as gc;
use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use egui::{Event, Key, Modifiers, MouseWheelUnit, TouchPhase, pos2, vec2};

const HELLO_COURSE: &str = include_str!("../../../examples/hello-course/course.json");

fn hello() -> TestHarness {
    TestHarness::new(Course::from_json(HELLO_COURSE).expect("hello course parses"))
}

fn picture_decodes() -> u64 {
    coursecraft_render::media::test_counts::picture_decode_count()
}

fn reset_counts() {
    gc::reset();
    coursecraft_render::media::test_counts::reset_picture_decode_count();
}

/// A course with one picture per slide, all with distinct cache keys.
fn picture_course(n: usize) -> Course {
    let mut slides = Vec::with_capacity(n);
    for i in 0..n {
        slides.push(format!(
            r##"{{"id":"s-{i}","title":"Slide {i}","objects":[{{"id":"p{i}","kind":{{"type":"image","src":"img/p{i}.png","alt":"Test picture"}},"frame":{{"x":100,"y":100,"w":400,"h":225}}}}]}}"##
        ));
    }
    let json = format!(
        r#"{{"schema_version":1,"id":"pictures","title":"Pictures","slide_size":{{"w":1280,"h":720}},"scenes":[{{"id":"sc0","title":"Scene 0","slides":[{}]}}]}}"#,
        slides.join(",")
    );
    Course::from_json(&json).expect("generated picture course parses")
}

fn picture_media(n: usize) -> Vec<(String, Arc<[u8]>)> {
    (0..n)
        .map(|i| {
            let mut pixels = vec![0u8; 16 * 16 * 4];
            pixels[0] = (i & 0xFF) as u8;
            pixels[1] = ((i >> 8) & 0xFF) as u8;
            pixels[3] = 255;
            let png = Arc::from(
                crate::coursecraft_media::encode_png_rgba(16, 16, &pixels)
                    .expect("test picture encodes"),
            );
            (format!("img/p{i}.png"), png)
        })
        .collect()
}

fn wheel(dy: f32) -> Event {
    Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: vec2(0.0, dy),
        phase: TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: Modifiers::NONE,
    }
}

/// Twenty idle frames after settling copy nothing, spell-check nothing,
/// redraw no thumbnail and decode no picture.
#[test]
fn idle_frames_do_no_counted_work() {
    let mut h = hello();
    for _ in 0..20 {
        let _ = h.idle_frame();
    }
    reset_counts();
    for _ in 0..20 {
        let _ = h.idle_frame();
    }
    assert_eq!(gc::course_copies(), 0, "idle frames copy no course");
    assert_eq!(
        gc::speller_runs(),
        0,
        "unchanged text is served from the cache"
    );
    assert_eq!(
        gc::thumbnail_redraws(),
        0,
        "settled thumbnails are not redrawn"
    );
    assert_eq!(
        gc::thumbnail_composes(),
        0,
        "idle frames compose no thumbnails"
    );
    assert_eq!(picture_decodes(), 0, "no new picture is decoded while idle");
}

/// Typing one letter into the in-place editor checks the one edited text once.
/// It does not check every text box on the slide, and no suggestion is looked
/// up until a word menu or the Check Spelling window needs one.
#[test]
fn typing_one_letter_checks_one_text_and_no_suggestions() {
    let mut h = hello();
    h.editor.selection.slide = "s-intro".into();
    h.editor.ui.inline_edit = Some(crate::canvas::InlineEdit {
        slide: "s-intro".into(),
        object: "title".into(),
        buffer: "Teh cat".into(),
        original: "Teh cat".into(),
        is_button: false,
        is_shape: false,
        cell: None,
        just_entered: true,
        live_runs: Vec::new(),
        live_paragraphs: Vec::new(),
        last_revision: 0,
        state_name: None,
        extra_id: None,
    });
    // First frame focuses and selects all, the second clears `just_entered`,
    // then ArrowRight collapses the selection so typing lands in the buffer.
    h.idle_frame();
    h.idle_frame();
    h.press(Key::ArrowRight, Modifiers::NONE);
    assert!(
        h.editor
            .ui
            .spell_inline_debug
            .iter()
            .any(|word| word == "Teh"),
        "the test needs a misspelling in the edited text"
    );

    reset_counts();
    h.frame(vec![Event::Text("x".into())]);
    assert!(
        h.editor
            .ui
            .inline_edit
            .as_ref()
            .is_some_and(|edit| edit.buffer == "Teh catx"),
        "the typed letter should land in the in-place buffer"
    );
    assert_eq!(
        gc::speller_runs(),
        1,
        "one edited text means one misspellings call, not one per text box"
    );
    assert_eq!(
        gc::suggestion_lookups(),
        0,
        "suggestions are lazy: no word menu is open"
    );
    assert_eq!(
        gc::course_copies(),
        0,
        "typing in a buffer has not changed the course yet"
    );
}

/// Switching to a slide whose thumbnail is off screen decodes only that slide's
/// picture. Returning to the first slide decodes nothing: its texture is cached.
#[test]
fn switching_slides_decodes_only_the_new_pictures() {
    const SLIDES: usize = 12;
    let mut h = TestHarness::new(picture_course(SLIDES));
    h.editor.set_media(picture_media(SLIDES));
    for _ in 0..4 {
        let _ = h.idle_frame();
    }
    assert!(
        h.editor.media_cache.is_loaded_image("img/p0.png"),
        "the first slide's picture should be decoded while settling"
    );

    // s-8 is outside the thumbnail viewport, so its picture is not cached yet.
    reset_counts();
    h.editor.selection.slide = "s-8".into();
    let _ = h.idle_frame();
    let decodes = picture_decodes();
    assert_eq!(
        decodes, 1,
        "only s-8's one picture should decode, got {decodes}"
    );

    reset_counts();
    h.editor.selection.slide = "s-0".into();
    let _ = h.idle_frame();
    assert_eq!(picture_decodes(), 0, "the first slide's picture is cached");
}

/// GD-01: A nudge changes one slide's content, so only that slide's thumbnail is
/// recomposed, and the engine records a scoped slide delta with zero whole-course copies.
#[test]
fn moving_an_object_redraws_only_that_slide_thumbnail() {
    let mut h = hello();
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.object = Some("title".into());
    for _ in 0..4 {
        let _ = h.idle_frame();
    }
    reset_counts();
    h.press(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(
        gc::thumbnail_redraws(),
        1,
        "the other visible thumbnails still show the same slides"
    );
    assert_eq!(
        gc::course_copies(),
        0,
        "single-slide edits use a scoped slide delta with zero full-course clones"
    );
}

/// With 100 slides, a scroll frame only redraws and decodes the thumbnails in
/// the viewport, not all 100 rows.
#[test]
fn scrolling_the_list_only_touches_visible_thumbnails() {
    const SLIDES: usize = 100;
    const VISIBLE_LIMIT: u64 = 12;
    let mut h = TestHarness::new(picture_course(SLIDES));
    for _ in 0..4 {
        let _ = h.idle_frame();
    }
    let list_pos = h
        .find_text_containing("Slide 0")
        .unwrap_or_else(|| pos2(90.0, 300.0));
    h.frame(vec![Event::PointerMoved(list_pos)]);
    let top = h.editor.ui.slides_scroll_y;
    // Supply the pictures only now, so scrolling has real decodes to do.
    h.editor.set_media(picture_media(SLIDES));

    reset_counts();
    gc::reset_thumb_signatures();
    // One scroll frame: every row it draws is a fresh arrival.
    h.frame(vec![wheel(-8000.0)]);
    let bottom = h.editor.ui.slides_scroll_y;
    assert!(bottom > top, "the wheel event should scroll the slide list");
    let redraws = gc::thumbnail_redraws();
    let decodes = picture_decodes();
    assert!(
        (1..=VISIBLE_LIMIT).contains(&redraws),
        "only the rows in view should be redrawn, got {redraws}"
    );
    assert!(
        (1..=VISIBLE_LIMIT).contains(&decodes),
        "only the visible thumbnails' pictures should decode, got {decodes}"
    );
}

/// The Check Spelling window is the other place that needs suggestions: with
/// a misspelling on the current issue, opening it works them out once.
#[test]
fn spelling_window_asks_for_suggestions_when_open() {
    let json = r#"{"schema_version":1,"id":"typos","title":"Typos","slide_size":{"w":1280,"h":720},"scenes":[{"id":"sc0","title":"Scene 0","slides":[{"id":"s0","title":"Slide 0","objects":[{"id":"t0","kind":{"type":"text","text":"teh cat"},"frame":{"x":80,"y":80,"w":600,"h":120}}]}]}]}"#;
    let course = Course::from_json(json).expect("typo course parses");
    let mut h = TestHarness::new(course);
    for _ in 0..2 {
        let _ = h.idle_frame();
    }
    assert!(
        h.editor.ui.prefs.spell_check_as_you_type,
        "as-you-type checking is on by default"
    );
    reset_counts();
    crate::spell_check::open(&mut h.editor);
    let _ = h.idle_frame();
    assert!(
        !h.editor.ui.spell_check.issues.is_empty(),
        "the course-wide check should find the typo"
    );
    assert!(
        gc::suggestion_lookups() > 0,
        "the window asks for the current word's suggestions"
    );
}

/// Course validation runs once per course revision, not on every idle frame.
#[test]
fn validation_runs_once_per_revision() {
    let mut h = hello();
    h.editor.ui.bottom_tab = crate::BottomTab::Triggers;
    // Initial frame runs validation for revision 0.
    let _ = h.idle_frame();
    reset_counts();
    for _ in 0..20 {
        let _ = h.idle_frame();
    }
    assert_eq!(
        gc::validation_runs(),
        0,
        "idle frames with no revision change must not re-run course validation"
    );

    // One edit bumps the revision and triggers validation exactly once.
    let _ = h.editor.run(
        "object.delete",
        serde_json::json!({ "slide": "s-intro", "object": "btn-start" }),
    );
    for _ in 0..5 {
        let _ = h.idle_frame();
    }
    assert_eq!(
        gc::validation_runs(),
        1,
        "a single revision bump must run course validation exactly once"
    );
}

/// Ten idle frames with the Trigger Editor open copy no course.
#[test]
fn trigger_editor_idle_frames_copy_no_course() {
    let mut h = hello();
    h.editor.selection.slide = "s-intro".into();
    let slide = h
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("s-intro")
        .clone();
    let form =
        crate::triggers::TriggerFormState::new_for_owner(&slide, None, &h.editor.session.course);
    h.editor.ui.trigger_form = Some(form);
    let _ = h.idle_frame();
    assert!(
        h.find_text("New trigger").is_some(),
        "trigger editor window should be open"
    );

    reset_counts();
    for _ in 0..10 {
        let _ = h.idle_frame();
    }
    assert_eq!(
        gc::course_copies(),
        0,
        "idle frames with trigger editor open must copy no course"
    );
}

/// Ten quick edits run the accessibility audit once after 300 ms of quiet,
/// not ten times during the burst.
#[test]
fn audit_debounces_across_quick_edits() {
    let mut h = hello();
    let _ = h.frame_at_time(0.1);
    let _ = h.frame_at_time(0.5);
    reset_counts();

    for i in 0..10 {
        let res = h.editor.run(
            "slide.update",
            serde_json::json!({
                "slide": "s-intro",
                "title": format!("Intro {i}"),
            }),
        );
        assert!(res.is_some(), "slide.update should succeed");
        let _ = h.frame_at_time(1.0 + (i as f64) * 0.02);
    }

    assert_eq!(
        gc::validation_runs(),
        10,
        "validation still runs on every revision"
    );
    assert_eq!(
        gc::audit_runs(),
        0,
        "audit must stay deferred while edits are less than 300 ms apart"
    );

    // 400 ms after the last edit (1.18 + 0.40 = 1.58), the audit runs once.
    let _ = h.frame_at_time(1.60);
    assert_eq!(
        gc::audit_runs(),
        1,
        "audit must run once after 300 ms of quiet"
    );

    // Subsequent idle frames must not re-run the audit.
    let _ = h.frame_at_time(2.00);
    assert_eq!(gc::audit_runs(), 1, "audit must not re-run while idle");
}

#[test]
fn triggers_tab_idle_frames_do_not_clone_slide() {
    let mut h = hello();
    h.editor.ui.bottom_tab = crate::timeline::BottomTab::Triggers;
    for _ in 0..4 {
        let _ = h.idle_frame();
    }
    reset_counts();
    for _ in 0..10 {
        let _ = h.idle_frame();
    }
    assert_eq!(
        gc::slide_clones(),
        0,
        "triggers tab must not clone the slide on idle repaints"
    );
}

#[test]
fn question_tab_idle_frames_do_not_clone_slide() {
    let mut h = hello();
    h.editor.ui.bottom_tab = crate::timeline::BottomTab::Question;
    for _ in 0..4 {
        let _ = h.idle_frame();
    }
    reset_counts();
    for _ in 0..10 {
        let _ = h.idle_frame();
    }
    assert_eq!(
        gc::slide_clones(),
        0,
        "question tab must not clone the slide on idle repaints"
    );
}

#[test]
fn timeline_idle_frames_reuse_cached_lists() {
    let mut h = hello();
    h.editor.ui.bottom_tab = crate::timeline::BottomTab::Timeline;
    for _ in 0..4 {
        let _ = h.idle_frame();
    }
    reset_counts();
    for _ in 0..10 {
        let _ = h.idle_frame();
    }
    assert_eq!(
        gc::timeline_list_rebuilds(),
        0,
        "timeline must reuse cached object and cue lists on idle repaints"
    );
}

/// PERF-02: Slide with 50 objects × 6 states × 4 extras (all visible) completes
/// compose + paint on SlideView without panics, and idle editor frames perform
/// zero course copies, zero thumbnail redraws, zero slide clones, and zero decodes.
#[test]
fn perf02_fifty_object_stateful_slide_bounded_work() {
    let mut objects = Vec::with_capacity(50);
    let state_names = ["hover", "down", "selected", "visited", "custom1"];
    for i in 0..50 {
        let col = i % 10;
        let row = i / 10;
        let x = 20.0 + (col as f32) * 120.0;
        let y = 20.0 + (row as f32) * 130.0;
        let mut states = Vec::with_capacity(state_names.len());
        for sname in state_names {
            let mut extras = Vec::with_capacity(4);
            for e in 0..4 {
                extras.push(serde_json::json!({
                    "id": format!("ex-{i}-{sname}-{e}"),
                    "name": format!("Extra {e}"),
                    "kind": {
                        "type": "shape",
                        "shape": "rounded_rectangle",
                        "fill": "#2563EB",
                        "text": { "text": format!("E{e}") }
                    },
                    "frame": { "x": (e as f32) * 12.0, "y": -10.0, "w": 24.0, "h": 20.0 }
                }));
            }
            states.push(serde_json::json!({
                "name": sname,
                "fill": "#1D4ED8",
                "extras": extras
            }));
        }
        objects.push(serde_json::json!({
            "id": format!("obj-{i}"),
            "kind": {
                "type": "shape",
                "shape": "rounded_rectangle",
                "fill": "#E2E8F0",
                "text": { "text": format!("Card {i}") }
            },
            "frame": { "x": x, "y": y, "w": 100.0, "h": 90.0 },
            "states": states
        }));
    }
    let course_json = serde_json::json!({
        "schema_version": 1,
        "id": "perf02-stateful",
        "title": "Perf02 Stateful",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "sc0",
            "title": "Scene 0",
            "slides": [{
                "id": "s0",
                "title": "50 Stateful Objects",
                "objects": objects
            }]
        }]
    });
    let course = Course::from_json(&course_json.to_string()).expect("perf02 course parses");

    // 1. Editor idle frames must do zero counted work even on a 50-object stateful slide.
    let mut h = TestHarness::new(course.clone());
    h.editor.selection.slide = "s0".into();
    for _ in 0..10 {
        let _ = h.idle_frame();
    }
    reset_counts();
    for _ in 0..10 {
        let _ = h.idle_frame();
    }
    assert_eq!(gc::course_copies(), 0, "idle frames copy no course");
    assert_eq!(
        gc::thumbnail_redraws(),
        0,
        "settled thumbnails are not redrawn"
    );
    assert_eq!(gc::slide_clones(), 0, "idle frames clone no slide");
    assert_eq!(picture_decodes(), 0, "idle frames decode no pictures");

    // 2. Player SlideView compose + paint over 5 frames with hover transitions completes
    // cleanly with zero picture decodes and zero panics.
    let mut player = coursecraft_logic::Player::new(&course).expect("player starts");
    player.handle(&coursecraft_logic::Input::Start);
    let mut view = coursecraft_render::SlideView::new();
    let ctx = egui::Context::default();
    reset_counts();
    for step in 0..5 {
        let px = 70.0 + (step as f32) * 120.0;
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 720.0),
            )),
            events: vec![Event::PointerMoved(pos2(px, 65.0))],
            time: Some((step as f64) * 0.016),
            ..Default::default()
        };
        let mut out = ctx.run_ui(raw, |ui| {
            let _ = view.show(ui, &player, None);
        });
        out.textures_delta.clear();
    }
    assert_eq!(
        picture_decodes(),
        0,
        "vector/text stateful objects decode zero pictures"
    );
}

/// A multi-scene course matching `perf_tests.rs` scale (10 scenes × 30 slides
/// for `n = 300`, 6 objects per slide).
fn many_slide_course(n: usize) -> Course {
    let scene_count = 10usize;
    let per_scene = n.div_ceil(scene_count);
    let mut scenes = Vec::new();
    for scene in 0..scene_count {
        let start = scene * per_scene;
        let end = (start + per_scene).min(n);
        if start >= end {
            break;
        }
        let mut slides = Vec::with_capacity(end - start);
        for i in start..end {
            slides.push(format!(
                r##"{{"id":"s-{i}","title":"Slide {i}","duration_ms":3000,"objects":[
                {{"id":"t{i}","kind":{{"type":"text","text":"Slide {i} title","style":{{"size":44,"bold":true}}}},"frame":{{"x":60,"y":40,"w":1100,"h":80}}}},
                {{"id":"a{i}","kind":{{"type":"text","text":"First paragraph of body text."}},"frame":{{"x":60,"y":140,"w":500,"h":160}}}},
                {{"id":"b{i}","kind":{{"type":"text","text":"Second paragraph of body text."}},"frame":{{"x":60,"y":320,"w":500,"h":160}}}},
                {{"id":"p{i}","kind":{{"type":"image","src":"img/p{i}.png","alt":"Test picture"}},"frame":{{"x":600,"y":140,"w":560,"h":340}}}},
                {{"id":"h{i}","kind":{{"type":"shape","shape":"rounded_rectangle","fill":"#2E8B57"}},"frame":{{"x":60,"y":520,"w":200,"h":80}}}},
                {{"id":"btn{i}","kind":{{"type":"button","label":"Next"}},"frame":{{"x":1000,"y":600,"w":200,"h":60}}}}]}}"##
            ));
        }
        scenes.push(format!(
            r#"{{"id":"sc{scene}","title":"Scene {scene}","slides":[{}]}}"#,
            slides.join(",")
        ));
    }
    let json = format!(
        r#"{{"schema_version":1,"id":"many-{n}","title":"Many Slides","slide_size":{{"w":1280,"h":720}},"scenes":[{}]}}"#,
        scenes.join(",")
    );
    Course::from_json(&json).expect("generated many-slide course parses")
}

/// SL-01: Twenty idle frames on a settled 300-slide course perform zero
/// thumbnail composes and zero thumbnail redraws.
#[test]
fn idle_frames_do_zero_thumbnail_composes() {
    let mut h = TestHarness::new(many_slide_course(300));
    for _ in 0..4 {
        let _ = h.idle_frame();
    }
    reset_counts();
    for _ in 0..20 {
        let _ = h.idle_frame();
    }
    assert_eq!(
        gc::thumbnail_composes(),
        0,
        "idle frames on a 300-slide course must compose zero thumbnails"
    );
    assert_eq!(
        gc::thumbnail_redraws(),
        0,
        "idle frames on a 300-slide course must redraw zero thumbnails"
    );
}

/// SL-02: Nudging one object on slide 0 of a 300-slide course recomposes
/// exactly that one slide's thumbnail, not all visible rows or all 300 slides.
#[test]
fn moving_object_composes_only_one_thumbnail() {
    let mut h = TestHarness::new(many_slide_course(300));
    h.editor.selection.slide = "s-0".into();
    h.editor.selection.object = Some("t0".into());
    for _ in 0..4 {
        let _ = h.idle_frame();
    }
    reset_counts();
    h.press(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(
        gc::thumbnail_composes(),
        1,
        "nudging one object should compose only the edited slide's thumbnail"
    );
}

/// SL-03: Scrolling a 300-slide navigator composes only the newly visible rows
/// (1..=10), and scrolling back to already-cached rows at the same revision
/// performs zero thumbnail composes.
#[test]
fn scrolling_300_slide_list_composes_only_visible_rows() {
    let mut h = TestHarness::new(many_slide_course(300));
    for _ in 0..4 {
        let _ = h.idle_frame();
    }
    let list_pos = h.find_text("Slide 1").unwrap_or_else(|| pos2(90.0, 300.0));
    h.frame(vec![Event::PointerMoved(list_pos)]);
    let top = h.editor.ui.slides_scroll_y;

    reset_counts();
    h.frame(vec![wheel(-2000.0)]);
    // egui's ScrollArea updates its offset after the inner closure runs; render
    // the next frame so the newly scrolled-in rows are actually composed.
    let _ = h.idle_frame();
    let after_down = h.editor.ui.slides_scroll_y;
    assert!(
        after_down > top,
        "downward wheel should scroll the 300-slide list (top={top}, after={after_down})"
    );
    assert!(
        h.find_text("Slide 1").is_none(),
        "Slide 1 should be scrolled out of view after scrolling down (top={top}, after={after_down})"
    );
    let down_composes = gc::thumbnail_composes();
    assert!(
        (1..=10).contains(&down_composes),
        "scrolling down should compose only newly visible rows (1..=10), got {down_composes}"
    );

    // Let egui's smooth-scroll animation finish before measuring the return scroll.
    for _ in 0..20 {
        let prev = h.editor.ui.slides_scroll_y;
        let _ = h.idle_frame();
        if (h.editor.ui.slides_scroll_y - prev).abs() < 0.01 {
            break;
        }
    }

    // Scrolling back up to the top at the same revision hits the ThumbnailCache.
    reset_counts();
    h.frame(vec![wheel(6000.0)]);
    let _ = h.idle_frame();
    let after_up = h.editor.ui.slides_scroll_y;
    assert!(
        after_up <= top + 1.0,
        "upward wheel should return to the top (top={top}, after={after_up})"
    );
    assert!(
        h.find_text("Slide 1").is_some(),
        "Slide 1 should be visible again after scrolling back up (top={top}, after={after_up})"
    );
    assert_eq!(
        gc::thumbnail_composes(),
        0,
        "scrolling back to already-cached top rows at the same revision must compose zero thumbnails"
    );
}

/// SL-04: Editing a slide master or the course theme invalidates dependent
/// visible thumbnails, while changing selection between already-visible slides
/// without scrolling composes zero thumbnails.
#[test]
fn editing_master_or_theme_invalidates_visible_thumbnails() {
    let json = r##"{
        "schema_version": 1,
        "id": "master-theme-cache",
        "title": "Master Theme Cache",
        "slide_size": { "w": 1280, "h": 720 },
        "masters": [{
            "id": "m1",
            "title": "Main master",
            "objects": [{
                "id": "m-banner",
                "kind": { "type": "shape", "shape": "rectangle", "fill": "#1D4ED8" },
                "frame": { "x": 0, "y": 0, "w": 1280, "h": 80 }
            }]
        }],
        "scenes": [{
            "id": "sc0",
            "title": "Scene 0",
            "slides": [
                {
                    "id": "s-0",
                    "title": "Slide 0",
                    "master": "m1",
                    "objects": [{
                        "id": "t0",
                        "kind": { "type": "text", "text": "Uses master" },
                        "frame": { "x": 80, "y": 140, "w": 400, "h": 80 }
                    }]
                },
                {
                    "id": "s-1",
                    "title": "Slide 1",
                    "objects": [{
                        "id": "t1",
                        "kind": { "type": "text", "text": "Plain slide" },
                        "frame": { "x": 80, "y": 140, "w": 400, "h": 80 }
                    }]
                }
            ]
        }]
    }"##;
    let course = Course::from_json(json).expect("master/theme test course parses");
    let mut h = TestHarness::new(course);
    for _ in 0..4 {
        let _ = h.idle_frame();
    }

    // 1. Editing an object on master `m1` invalidates visible slide `s-0`.
    reset_counts();
    let res = h.editor.run(
        "object.update",
        serde_json::json!({
            "slide": "m1",
            "object": "m-banner",
            "kind": { "type": "shape", "shape": "rectangle", "fill": "#DC2626" }
        }),
    );
    assert!(res.is_some(), "updating object on master m1 should succeed");
    let _ = h.idle_frame();
    assert!(
        gc::thumbnail_composes() >= 1,
        "editing master m1 must recompose dependent visible thumbnails, got {}",
        gc::thumbnail_composes()
    );

    // 2. Changing course theme background invalidates visible thumbnails.
    for _ in 0..2 {
        let _ = h.idle_frame();
    }
    reset_counts();
    let res = h.editor.run(
        "course.set_theme",
        serde_json::json!({ "background": "#102030" }),
    );
    assert!(res.is_some(), "course.set_theme should succeed");
    let _ = h.idle_frame();
    assert!(
        gc::thumbnail_composes() >= 1,
        "changing course theme background must recompose visible thumbnails, got {}",
        gc::thumbnail_composes()
    );

    // 3. Changing slide selection between two already-visible slides without
    // scrolling composes zero thumbnails.
    for _ in 0..2 {
        let _ = h.idle_frame();
    }
    reset_counts();
    h.editor.selection.slide = "s-1".into();
    let _ = h.idle_frame();
    assert_eq!(
        gc::thumbnail_composes(),
        0,
        "selecting another already-visible slide must compose zero thumbnails"
    );
}

/// s1: `guard_counters::reset()` zeroes all new M34 counters (canvas composes,
/// renderer counters, and engine full-course validations), and running an editor
/// frame increments `canvas_composes()`.
#[test]
fn s1_guard_counters_reset_and_record_canvas_compose() {
    let mut h = hello();
    let _ = h.idle_frame();

    // Bump every new counter before resetting so we prove reset() zeroes them all.
    gc::note_canvas_compose();
    coursecraft_render::test_counts::note_font_def_clone();
    coursecraft_render::test_counts::note_text_layout_miss();
    coursecraft_render::test_counts::note_shape_mesh_miss();
    coursecraft_render::test_counts::note_full_res_thumbnail_decode();
    coursecraft_engine::test_counts::note_full_course_validation();

    assert!(gc::canvas_composes() > 0);
    assert!(gc::font_def_clones() > 0);
    assert!(gc::text_layout_misses() > 0);
    assert!(gc::shape_mesh_misses() > 0);
    assert!(gc::full_res_thumbnail_decodes() > 0);
    assert!(gc::full_course_validations() > 0);

    gc::reset();

    assert_eq!(gc::canvas_composes(), 0);
    assert_eq!(gc::font_def_clones(), 0);
    assert_eq!(gc::text_layout_misses(), 0);
    assert_eq!(gc::shape_mesh_misses(), 0);
    assert_eq!(gc::full_res_thumbnail_decodes(), 0);
    assert_eq!(gc::full_course_validations(), 0);
    assert_eq!(picture_decodes(), 0);

    let _ = h.idle_frame();
    assert!(
        gc::canvas_composes() >= 1,
        "an editor frame must record at least one canvas compose, got {}",
        gc::canvas_composes()
    );
}

/// G-03: Steady idle and scroll frames perform zero `FontDefinitions` clones on
/// both `hello-course` and a course with a registered custom font face.
#[test]
fn g03_steady_frames_zero_font_def_clones() {
    // 1. hello-course: 20 idle frames + 20 scroll frames after settling.
    let mut h = hello();
    for _ in 0..10 {
        let _ = h.idle_frame();
    }
    reset_counts();
    for _ in 0..20 {
        let _ = h.idle_frame();
    }
    assert_eq!(
        gc::font_def_clones(),
        0,
        "20 steady idle frames on hello-course must perform zero FontDefinitions clones"
    );

    let list_pos = pos2(90.0, 300.0);
    h.frame(vec![Event::PointerMoved(list_pos)]);
    reset_counts();
    for i in 0..20 {
        let dy = if i % 2 == 0 { -120.0 } else { 120.0 };
        h.frame(vec![wheel(dy)]);
    }
    assert_eq!(
        gc::font_def_clones(),
        0,
        "20 scroll frames on hello-course must perform zero FontDefinitions clones"
    );

    // 2. Course with a registered custom font face: 20 idle frames after settling.
    const TEST_FONT: &[u8] = include_bytes!("../../render/fonts/Inter-Bold.ttf");
    let custom_json = r##"{
        "schema_version": 1,
        "id": "custom-font-guard",
        "title": "Custom Font Guard",
        "slide_size": { "w": 1280, "h": 720 },
        "fonts": [{
            "family": "BrandDisplay",
            "style": "regular",
            "src": "fonts/brand.ttf"
        }],
        "scenes": [{
            "id": "sc0",
            "title": "Scene 0",
            "slides": [{
                "id": "s0",
                "title": "Brand Slide",
                "objects": [{
                    "id": "t0",
                    "kind": {
                        "type": "text",
                        "text": "Custom font headline",
                        "style": { "font": "BrandDisplay", "size": 42, "color": "#111827" }
                    },
                    "frame": { "x": 80, "y": 80, "w": 640, "h": 120 }
                }]
            }]
        }]
    }"##;
    let custom_course = Course::from_json(custom_json).expect("custom font course parses");
    let mut h_custom = TestHarness::new(custom_course);
    h_custom
        .editor
        .set_media(vec![("fonts/brand.ttf".into(), Arc::from(TEST_FONT))]);
    for _ in 0..10 {
        let _ = h_custom.idle_frame();
    }
    assert!(
        coursecraft_render::font_epoch(&h_custom.ctx) > 0,
        "custom font course should have installed fonts"
    );
    reset_counts();
    for _ in 0..20 {
        let _ = h_custom.idle_frame();
    }
    assert_eq!(
        gc::font_def_clones(),
        0,
        "20 steady idle frames with a registered custom font face must perform zero FontDefinitions clones"
    );
}

fn png_crc32(chunk_type: &[u8; 4], data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in chunk_type.iter().chain(data.iter()) {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/// Build `n` distinct valid `1280x720` PNGs quickly by encoding one base PNG
/// and splicing a unique `tEXt` chunk before `IEND` so content-hash dedup
/// never collapses them.
fn distinct_1280x720_pngs(n: usize) -> Vec<(String, Arc<[u8]>)> {
    let pixels = vec![64u8; 1280 * 720 * 4];
    let base = crate::coursecraft_media::encode_png_rgba(1280, 720, &pixels)
        .expect("encode base 1280x720 png");
    let iend_start = base.len().saturating_sub(12);
    let (prefix, iend) = base.split_at(iend_start);

    (0..n)
        .map(|i| {
            let text = format!("Comment\0slide-{i}");
            let data = text.as_bytes();
            let crc = png_crc32(b"tEXt", data);
            let mut png = Vec::with_capacity(base.len() + 12 + data.len());
            png.extend_from_slice(prefix);
            png.extend_from_slice(&(data.len() as u32).to_be_bytes());
            png.extend_from_slice(b"tEXt");
            png.extend_from_slice(data);
            png.extend_from_slice(&crc.to_be_bytes());
            png.extend_from_slice(iend);
            (format!("img/p{i}.png"), Arc::from(png))
        })
        .collect()
}

/// G-02: Scrolling a 100-slide picture course with distinct 1280×720 PNGs
/// decodes thumbnails at thumbnail tier (`total_bytes <= 8 MiB`,
/// `full_res_thumbnail_decodes == 0`), and selecting slide `s-50` on the main
/// canvas upgrades its texture cleanly.
#[test]
fn g02_scrolling_100_slide_pictures_stays_under_8_mib() {
    const SLIDES: usize = 100;
    let mut slides = Vec::with_capacity(SLIDES);
    for i in 0..SLIDES {
        slides.push(format!(
            r##"{{"id":"s-{i}","title":"Slide {i}","objects":[{{"id":"p{i}","kind":{{"type":"image","src":"img/p{i}.png","alt":"Photo {i}"}},"frame":{{"x":40,"y":40,"w":1200,"h":640}}}}]}}"##
        ));
    }
    let json = format!(
        r#"{{"schema_version":1,"id":"g02-pictures","title":"G02 Pictures","slide_size":{{"w":1280,"h":720}},"scenes":[{{"id":"sc0","title":"Scene 0","slides":[{}]}}]}}"#,
        slides.join(",")
    );
    let course = Course::from_json(&json).expect("g02 course parses");

    let mut h = TestHarness::new(course);
    h.editor.set_media(distinct_1280x720_pngs(SLIDES));
    reset_counts();

    // Settle initial viewport (top thumbnails + s-0 on canvas).
    for _ in 0..3 {
        let _ = h.idle_frame();
    }
    let list_pos = h
        .find_text_containing("Slide 0")
        .unwrap_or_else(|| pos2(90.0, 300.0));
    h.frame(vec![Event::PointerMoved(list_pos)]);
    let top = h.editor.ui.slides_scroll_y;

    // Scroll through the slide list (middle and bottom) without selecting every slide.
    h.frame(vec![wheel(-4000.0)]);
    let _ = h.idle_frame();
    h.frame(vec![wheel(-4000.0)]);
    let _ = h.idle_frame();

    assert!(
        h.editor.ui.slides_scroll_y > top,
        "wheel events must actually scroll the 100-slide list (top={top}, after={})",
        h.editor.ui.slides_scroll_y
    );
    let decoded_during_scroll = h.editor.media_cache.decoded_picture_count();
    assert!(
        decoded_during_scroll >= 10,
        "scrolling 100 distinct slides must decode at least 10 distinct thumbnails, got {decoded_during_scroll}"
    );
    let scroll_bytes = h.editor.media_cache.total_bytes();
    assert!(
        scroll_bytes <= 8 * 1024 * 1024,
        "scrolling 100-slide 1280x720 picture list must stay <= 8 MiB, got {scroll_bytes} bytes"
    );
    assert_eq!(
        gc::full_res_thumbnail_decodes(),
        0,
        "thumbnails must never trigger full-resolution decodes"
    );

    // Select slide s-50 on the main canvas and verify it upgrades cleanly and renders.
    let bytes_before_s50 = h.editor.media_cache.total_bytes();
    let tex_before_s50 = h.editor.media_cache.texture_id("img/p50.png");
    h.editor.selection.slide = "s-50".into();
    let _ = h.idle_frame();

    assert!(
        h.editor.media_cache.is_loaded_image("img/p50.png"),
        "s-50 picture must load and render on the main canvas"
    );
    assert!(
        matches!(
            h.editor.media_cache.get_entry("img/p50.png"),
            Some(coursecraft_render::MediaEntry::Texture(_, [1280, 720]))
        ),
        "s-50 picture must preserve its natural 1280x720 dimensions"
    );
    assert_ne!(
        h.editor.media_cache.texture_id("img/p50.png"),
        tex_before_s50,
        "selecting s-50 on the main canvas must upgrade/upload its canvas texture"
    );
    assert!(
        h.editor.media_cache.total_bytes() > bytes_before_s50,
        "upgrading s-50 for the main canvas should allocate its higher-tier texture"
    );
    assert_eq!(
        gc::full_res_thumbnail_decodes(),
        0,
        "upgrading s-50 on the canvas must not count as a full-res thumbnail decode"
    );
}

/// G-07 (`M34-07`): On a settled 300-slide course, nudging an object with an
/// arrow key uses scoped single-slide validation and cached engine issues with
/// zero full-course validations and exactly one validation refresh.
#[test]
fn g07_arrow_nudge_uses_cached_validation_with_zero_full_validations() {
    let mut h = TestHarness::new(many_slide_course(300));
    h.editor.selection.slide = "s-0".into();
    h.editor.selection.object = Some("t0".into());
    for _ in 0..4 {
        let _ = h.idle_frame();
    }

    let x_before = h
        .editor
        .session
        .course
        .slide("s-0")
        .and_then(|slide| slide.object("t0"))
        .map(|(obj, _)| obj.frame.x)
        .expect("t0 exists on s-0");

    reset_counts();
    h.press(Key::ArrowRight, Modifiers::NONE);

    let x_after = h
        .editor
        .session
        .course
        .slide("s-0")
        .and_then(|slide| slide.object("t0"))
        .map(|(obj, _)| obj.frame.x)
        .expect("t0 exists on s-0");
    assert!(
        x_after > x_before,
        "ArrowRight should nudge t0 to the right (before={x_before}, after={x_after})"
    );
    assert_eq!(
        gc::full_course_validations(),
        0,
        "arrow nudge on a 300-slide course must perform zero full-course validations"
    );
    assert_eq!(
        gc::validation_runs(),
        1,
        "arrow nudge must trigger exactly one cached validation refresh"
    );
}
