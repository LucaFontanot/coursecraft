//! Headless tests for Step h4: "Preview this slide" and "Preview selected slides".

use crate::preview::Preview;
use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use coursecraft_render::{MediaCache, MediaFetch, MediaSource};
use egui::{Key, Modifiers};
use std::sync::{Arc, Mutex};

fn multi_slide_course() -> Course {
    let json = r#"{
        "schema_version": 1,
        "id": "preview-subset-test",
        "title": "Preview Subset Test",
        "slide_size": { "w": 1280, "h": 720 },
        "variables": [
            { "name": "score", "value": 0 }
        ],
        "scenes": [
            {
                "id": "sc1",
                "title": "Main",
                "slides": [
                    {
                        "id": "s1",
                        "title": "Slide One",
                        "duration_ms": 5000,
                        "objects": [
                            {
                                "id": "txt_s1",
                                "kind": { "type": "text", "text": "Slide One Score: %score%" },
                                "frame": { "x": 80, "y": 60, "w": 400, "h": 50 }
                            },
                            {
                                "id": "btn_next_s1",
                                "kind": { "type": "button", "label": "Next Slide Btn" },
                                "frame": { "x": 80, "y": 140, "w": 200, "h": 48 }
                            },
                            {
                                "id": "btn_to_s3",
                                "kind": { "type": "button", "label": "Jump To Three" },
                                "frame": { "x": 80, "y": 210, "w": 200, "h": 48 }
                            },
                            {
                                "id": "btn_complete",
                                "kind": { "type": "button", "label": "Complete Course" },
                                "frame": { "x": 80, "y": 280, "w": 200, "h": 48 }
                            },
                            {
                                "id": "img_s1",
                                "kind": { "type": "image", "src": "media/s1.png", "alt": "s1" },
                                "frame": { "x": 500, "y": 60, "w": 120, "h": 120 }
                            }
                        ],
                        "triggers": [
                            {
                                "when": { "event": "click", "object": "btn_to_s3" },
                                "action": { "action": "set_variable", "variable": "score", "op": "add", "value": 5 }
                            },
                            {
                                "when": { "event": "click", "object": "btn_to_s3" },
                                "action": { "action": "jump_to_slide", "target": { "id": "s3" } }
                            },
                            {
                                "when": { "event": "click", "object": "btn_next_s1" },
                                "action": { "action": "jump_to_slide", "target": "next" }
                            },
                            {
                                "when": { "event": "click", "object": "btn_complete" },
                                "action": { "action": "complete_course", "status": "completed" }
                            }
                        ]
                    },
                    {
                        "id": "s2",
                        "title": "Slide Two",
                        "duration_ms": 5000,
                        "objects": [
                            {
                                "id": "txt_s2",
                                "kind": { "type": "text", "text": "Slide Two" },
                                "frame": { "x": 80, "y": 60, "w": 300, "h": 50 }
                            },
                            {
                                "id": "img_s2",
                                "kind": { "type": "image", "src": "media/s2.png", "alt": "s2" },
                                "frame": { "x": 500, "y": 60, "w": 120, "h": 120 }
                            }
                        ],
                        "triggers": []
                    },
                    {
                        "id": "s3",
                        "title": "Slide Three",
                        "duration_ms": 5000,
                        "objects": [
                            {
                                "id": "txt_s3",
                                "kind": { "type": "text", "text": "Slide Three" },
                                "frame": { "x": 80, "y": 60, "w": 300, "h": 50 }
                            },
                            {
                                "id": "btn_to_s2",
                                "kind": { "type": "button", "label": "Jump To Two" },
                                "frame": { "x": 80, "y": 140, "w": 200, "h": 48 }
                            },
                            {
                                "id": "img_s3",
                                "kind": { "type": "image", "src": "media/s3.png", "alt": "s3" },
                                "frame": { "x": 500, "y": 60, "w": 120, "h": 120 }
                            }
                        ],
                        "triggers": [
                            {
                                "when": { "event": "click", "object": "btn_to_s2" },
                                "action": { "action": "jump_to_slide", "target": { "id": "s2" } }
                            }
                        ]
                    }
                ]
            }
        ]
    }"#;
    Course::from_json(json).expect("valid course")
}

#[test]
fn single_slide_preview_shows_end_popup_on_next_prev_button_and_complete_and_dismisses_cleanly() {
    let mut harness = TestHarness::new(multi_slide_course());
    // Even with multiple slides selected, "Preview this slide" previews only the current slide.
    harness.editor.selection.slide = "s1".into();
    harness.editor.selection.also_slides = vec!["s3".into()];
    harness.idle_frame();

    harness.press(Key::F12, Modifiers::NONE);
    harness.idle_frame();
    harness.idle_frame();

    let preview = harness.editor.preview.as_ref().expect("preview open");
    assert!(preview.is_partial_preview());
    assert!(harness.find_text("Partial preview (1 slide)").is_some());
    assert!(harness.find_text("1 / 1").is_some());

    // Click player Next button -> end-of-preview popup appears, stays on s1.
    let next_btn = harness
        .find_icon_last(coursecraft_render::Icon::CaretRight)
        .expect("next button");
    harness.click_at(next_btn);
    harness.idle_frame();
    assert!(
        harness
            .editor
            .preview
            .as_ref()
            .is_some_and(|p| p.end_of_preview_popup)
    );
    assert!(
        harness
            .find_text("This is the end of the preview")
            .is_some()
    );
    assert!(harness.find_text("Replay slide(s)").is_some());
    assert!(harness.find_text("Close preview").is_some());

    // Esc closes the popup first, keeping preview open.
    harness.press(Key::Escape, Modifiers::NONE);
    harness.idle_frame();
    assert!(
        harness
            .editor
            .preview
            .as_ref()
            .is_some_and(|p| !p.end_of_preview_popup)
    );
    assert!(
        harness
            .find_text("This is the end of the preview")
            .is_none()
    );

    // Click player Previous button -> popup appears again.
    let prev_btn = harness
        .find_icon_last(coursecraft_render::Icon::CaretLeft)
        .expect("previous button");
    harness.click_at(prev_btn);
    harness.idle_frame();
    assert!(
        harness
            .find_text("This is the end of the preview")
            .is_some()
    );

    // Click popup's X button -> only closes the popup so learner can keep testing.
    let popup_x = harness
        .find_icon_last(coursecraft_render::Icon::X)
        .expect("popup X button");
    harness.click_at(popup_x);
    harness.idle_frame();
    assert!(
        harness
            .editor
            .preview
            .as_ref()
            .is_some_and(|p| !p.end_of_preview_popup)
    );

    // Click on-slide button that jumps to Next -> popup appears.
    let on_slide_next = harness
        .find_text_last("Next Slide Btn")
        .expect("on-slide next");
    harness.click_at(on_slide_next);
    harness.idle_frame();
    assert!(
        harness
            .find_text("This is the end of the preview")
            .is_some()
    );

    // Dismiss popup with Esc, then click on-slide Complete Course -> popup appears.
    harness.press(Key::Escape, Modifiers::NONE);
    harness.idle_frame();
    let complete_btn = harness
        .find_text_last("Complete Course")
        .expect("complete btn");
    harness.click_at(complete_btn);
    harness.idle_frame();
    assert!(
        harness
            .find_text("This is the end of the preview")
            .is_some()
    );

    // Second Esc after dismissing popup closes the preview.
    harness.press(Key::Escape, Modifiers::NONE);
    harness.idle_frame();
    assert!(harness.editor.preview.is_some());
    harness.press(Key::Escape, Modifiers::NONE);
    harness.idle_frame();
    assert!(harness.editor.preview.is_none());
}

#[test]
fn multi_slide_preview_jumps_between_selected_slides_and_replay_resets_vars() {
    let mut harness = TestHarness::new(multi_slide_course());
    harness.editor.selection.slide = "s1".into();
    harness.editor.selection.also_slides = vec!["s3".into()];
    harness.idle_frame();

    harness.editor.open_preview_selected();
    harness.idle_frame();
    harness.idle_frame();

    assert!(harness.find_text("Partial preview (2 slides)").is_some());
    assert!(harness.find_text("1 / 2").is_some());
    assert!(harness.find_text("Slide One Score: 0").is_some());

    // Click "Jump To Three": increments score to 5 and jumps to s3 (in subset).
    let jump_s3 = harness.find_text_last("Jump To Three").expect("jump to s3");
    harness.click_at(jump_s3);
    harness.idle_frame();
    assert!(
        harness
            .editor
            .preview
            .as_ref()
            .is_some_and(|p| !p.end_of_preview_popup)
    );
    assert!(harness.find_text("2 / 2").is_some());
    assert!(harness.find_text("Slide Three").is_some());

    // Click "Jump To Two": s2 is unselected -> popup appears and stays on s3.
    let jump_s2 = harness.find_text_last("Jump To Two").expect("jump to s2");
    harness.click_at(jump_s2);
    harness.idle_frame();
    assert!(
        harness
            .find_text("This is the end of the preview")
            .is_some()
    );
    assert!(harness.find_text("Slide Three").is_some());

    // Click "Replay slide(s)" -> restarts at s1 with fresh variables (score = 0).
    let replay = harness.find_text("Replay slide(s)").expect("replay btn");
    harness.click_at(replay);
    harness.idle_frame();
    assert!(
        harness
            .editor
            .preview
            .as_ref()
            .is_some_and(|p| !p.end_of_preview_popup)
    );
    assert!(harness.find_text("1 / 2").is_some());
    assert!(harness.find_text("Slide One Score: 0").is_some());

    // Next from s1 goes to s3 (2 / 2); Next from s3 triggers popup, then click "Close Preview".
    let next_btn = harness
        .find_icon_last(coursecraft_render::Icon::CaretRight)
        .expect("next button");
    harness.click_at(next_btn);
    harness.idle_frame();
    assert!(harness.find_text("2 / 2").is_some());
    let next_btn = harness
        .find_icon_last(coursecraft_render::Icon::CaretRight)
        .expect("next button");
    harness.click_at(next_btn);
    harness.idle_frame();
    let close_preview = harness.find_text("Close preview").expect("close preview");
    harness.click_at(close_preview);
    harness.idle_frame();
    assert!(harness.editor.preview.is_none());
}

#[test]
fn preview_selected_slides_menu_entries_and_shift_f12_whole_course() {
    let mut harness = TestHarness::new(multi_slide_course());
    harness.editor.selection.slide = "s1".into();
    harness.editor.selection.also_slides = vec!["s3".into()];
    harness.idle_frame();

    // Toolbar Preview dropdown contains "Preview selected slides".
    let caret = harness
        .find_icon(coursecraft_render::Icon::CaretDown)
        .expect("preview caret");
    harness.click_at(caret);
    harness.idle_frame();
    if let Some(pos) = harness.find_text("Preview selected slides") {
        harness.click_at(pos);
        harness.idle_frame();
        assert!(
            harness
                .editor
                .preview
                .as_ref()
                .is_some_and(Preview::is_partial_preview)
        );
        harness.press(Key::Escape, Modifiers::NONE);
        harness.idle_frame();
    }

    // Shift+F12 previews the whole course (not partial).
    harness.press(Key::F12, Modifiers::SHIFT);
    harness.idle_frame();
    let preview = harness.editor.preview.as_ref().expect("full preview");
    assert!(!preview.is_partial_preview());
    assert!(harness.find_text("1 / 3").is_some());
}

#[test]
fn partial_preview_only_requests_media_for_subset_slides() {
    struct TrackingSource {
        requested: Arc<Mutex<Vec<String>>>,
    }
    impl MediaSource for TrackingSource {
        fn get(&self, src: &str) -> MediaFetch {
            if let Ok(mut guard) = self.requested.lock() {
                guard.push(src.to_owned());
            }
            MediaFetch::Missing
        }
    }

    let course = multi_slide_course();
    let requested = Arc::new(Mutex::new(Vec::new()));
    let source = TrackingSource {
        requested: Arc::clone(&requested),
    };
    let mut cache = MediaCache::new();
    let mut preview = Preview::new(&course, Some("s1"));
    let ctx = egui::Context::default();
    let mut open = true;
    let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
        preview.show(ui.ctx(), &mut open, Some((&mut cache, &source)));
    });
    output.textures_delta.clear();

    let reqs = requested.lock().expect("lock").clone();
    assert!(
        reqs.iter().any(|s| s == "media/s1.png"),
        "subset slide media should be requested: {reqs:?}"
    );
    assert!(
        !reqs
            .iter()
            .any(|s| s == "media/s2.png" || s == "media/s3.png"),
        "unselected slide media must not be requested: {reqs:?}"
    );
}

fn four_slide_course_with_bank() -> Course {
    let json = r#"{
        "schema_version": 1,
        "id": "preview-from-here-test",
        "title": "Preview From Here Test",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [
            {
                "id": "sc1",
                "title": "Main",
                "slides": [
                    {
                        "id": "s1",
                        "title": "Slide One",
                        "duration_ms": 5000,
                        "objects": [
                            {
                                "id": "txt_s1",
                                "kind": { "type": "text", "text": "Slide One Body" },
                                "frame": { "x": 80, "y": 60, "w": 300, "h": 50 }
                            }
                        ]
                    },
                    {
                        "id": "s2",
                        "title": "Slide Two",
                        "duration_ms": 5000,
                        "objects": [
                            {
                                "id": "txt_s2",
                                "kind": { "type": "text", "text": "Slide Two Body" },
                                "frame": { "x": 80, "y": 60, "w": 300, "h": 50 }
                            }
                        ]
                    },
                    {
                        "id": "s3",
                        "title": "Slide Three",
                        "duration_ms": 5000,
                        "objects": [
                            {
                                "id": "txt_s3",
                                "kind": { "type": "text", "text": "Slide Three Body" },
                                "frame": { "x": 80, "y": 60, "w": 300, "h": 50 }
                            }
                        ]
                    },
                    {
                        "id": "s4",
                        "title": "Slide Four",
                        "duration_ms": 5000,
                        "objects": [
                            {
                                "id": "txt_s4",
                                "kind": { "type": "text", "text": "Slide Four Body" },
                                "frame": { "x": 80, "y": 60, "w": 300, "h": 50 }
                            }
                        ]
                    }
                ]
            }
        ],
        "banks": [
            {
                "id": "b1",
                "title": "Bank One",
                "slides": [
                    {
                        "id": "bs1",
                        "title": "Bank Slide",
                        "duration_ms": 5000,
                        "question": {
                            "kind": "multiple_choice",
                            "prompt": "Bank question?",
                            "choices": [
                                { "id": "a", "text": "Choice A", "correct": true },
                                { "id": "b", "text": "Choice B" }
                            ],
                            "attempts": 1
                        },
                        "triggers": [
                            {
                                "when": { "event": "submit" },
                                "action": { "action": "submit_question" }
                            }
                        ]
                    }
                ]
            }
        ]
    }"#;
    let course = Course::from_json(json).expect("valid 4-slide course with bank");
    let errors: Vec<_> = course
        .validate()
        .into_iter()
        .filter(|i| i.severity == coursecraft_course::Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "fixture should have no errors: {errors:?}"
    );
    course
}

fn open_top_preview_menu(harness: &mut TestHarness) {
    let menu_bar = egui::Rect::from_min_max(
        egui::pos2(0.0, 0.0),
        egui::pos2(harness.screen_rect.max.x, crate::TOP_BAR_HEIGHT),
    );
    let pos = harness
        .find_text_in("Preview", menu_bar)
        .expect("top-level Preview menu");
    harness.click_at(pos);
    harness.idle_frame();
    harness.idle_frame();
}

/// P-02: "Preview from here" via top-level Preview menu and `Ctrl+F12` starts on slide 2,
/// runs as a full preview (`is_partial_preview() == false`), advances to slide 3 with no
/// end-of-preview popup, and restarts back to slide 2 (`M32-07`).
#[test]
fn preview_from_here_starts_at_selected_slide_advances_and_restarts_there() {
    let mut harness = TestHarness::new(four_slide_course_with_bank());
    harness.editor.selection.slide = "s2".into();
    harness.idle_frame();

    // 1. Trigger via Preview ▸ Preview from here
    open_top_preview_menu(&mut harness);
    let from_here = harness
        .find_text_last("Preview from here")
        .expect("Preview from here menu item");
    harness.click_at(from_here);
    harness.idle_frame();
    harness.idle_frame();

    let preview = harness.editor.preview.as_ref().expect("preview open");
    assert!(!preview.is_partial_preview());
    assert!(harness.find_text("2 / 4").is_some());
    assert!(harness.find_text_last("Slide Two Body").is_some());

    // Clicking Next advances to slide 3 with no end-of-preview popup.
    let next_btn = harness
        .find_icon_last(coursecraft_render::Icon::CaretRight)
        .expect("next button");
    harness.click_at(next_btn);
    harness.idle_frame();
    assert!(
        harness
            .editor
            .preview
            .as_ref()
            .is_some_and(|p| !p.end_of_preview_popup)
    );
    assert!(
        harness
            .find_text("This is the end of the preview")
            .is_none()
    );
    assert!(harness.find_text("3 / 4").is_some());
    assert!(harness.find_text_last("Slide Three Body").is_some());

    // Clicking Restart returns to slide 2 (not slide 1).
    let restart_btn = harness
        .find_icon_last(coursecraft_render::Icon::ArrowClockwise)
        .expect("restart button");
    harness.click_at(restart_btn);
    harness.idle_frame();
    assert!(harness.find_text("2 / 4").is_some());
    assert!(harness.find_text_last("Slide Two Body").is_some());

    // Close preview.
    harness.press(Key::Escape, Modifiers::NONE);
    harness.idle_frame();
    assert!(harness.editor.preview.is_none());

    // 2. Trigger via Ctrl+F12 (Modifiers::COMMAND + Key::F12)
    harness.press(Key::F12, Modifiers::COMMAND);
    harness.idle_frame();
    harness.idle_frame();

    let preview = harness
        .editor
        .preview
        .as_ref()
        .expect("preview open via Ctrl+F12");
    assert!(!preview.is_partial_preview());
    assert!(harness.find_text("2 / 4").is_some());
    assert!(harness.find_text_last("Slide Two Body").is_some());

    let next_btn = harness
        .find_icon_last(coursecraft_render::Icon::CaretRight)
        .expect("next button");
    harness.click_at(next_btn);
    harness.idle_frame();
    assert!(
        harness
            .editor
            .preview
            .as_ref()
            .is_some_and(|p| !p.end_of_preview_popup)
    );
    assert!(harness.find_text("3 / 4").is_some());

    if let Some(preview) = harness.editor.preview.as_mut() {
        preview.restart();
    }
    harness.idle_frame();
    assert!(harness.find_text("2 / 4").is_some());
    assert!(harness.find_text_last("Slide Two Body").is_some());
}

/// P-03: Preview menu opens the right scope for "Preview this slide", "Preview selected slides",
/// and "Preview whole course"; bank slides disable current-slide modes with a clear reason;
/// slide masters disables all four modes with "Not available in slide masters" (`M32-06`).
#[test]
fn preview_menu_modes_and_bank_and_slide_masters_disabled_reasons() {
    let mut harness = TestHarness::new(four_slide_course_with_bank());
    harness.editor.selection.slide = "s2".into();
    harness.idle_frame();

    // 1. Preview this slide
    open_top_preview_menu(&mut harness);
    let item = harness
        .find_text_last("Preview this slide")
        .expect("Preview this slide");
    harness.click_at(item);
    harness.idle_frame();
    harness.idle_frame();
    assert!(
        harness
            .editor
            .preview
            .as_ref()
            .is_some_and(Preview::is_partial_preview)
    );
    assert!(harness.find_text("1 / 1").is_some());
    assert!(harness.find_text_last("Slide Two Body").is_some());
    harness.press(Key::Escape, Modifiers::NONE);
    harness.idle_frame();
    assert!(harness.editor.preview.is_none());

    // 2. Preview selected slides (s2 and s4)
    harness.editor.selection.slide = "s2".into();
    harness.editor.selection.also_slides = vec!["s4".into()];
    harness.idle_frame();
    open_top_preview_menu(&mut harness);
    let item = harness
        .find_text_last("Preview selected slides")
        .expect("Preview selected slides");
    harness.click_at(item);
    harness.idle_frame();
    harness.idle_frame();
    assert!(
        harness
            .editor
            .preview
            .as_ref()
            .is_some_and(Preview::is_partial_preview)
    );
    assert!(harness.find_text("1 / 2").is_some());
    assert!(harness.find_text_last("Slide Two Body").is_some());
    harness.press(Key::Escape, Modifiers::NONE);
    harness.idle_frame();
    assert!(harness.editor.preview.is_none());

    // 3. Preview whole course
    harness.editor.selection.also_slides.clear();
    harness.idle_frame();
    open_top_preview_menu(&mut harness);
    let item = harness
        .find_text_last("Preview whole course")
        .expect("Preview whole course");
    harness.click_at(item);
    harness.idle_frame();
    harness.idle_frame();
    assert!(
        harness
            .editor
            .preview
            .as_ref()
            .is_some_and(|p| !p.is_partial_preview())
    );
    assert!(harness.find_text("1 / 4").is_some());
    assert!(harness.find_text_last("Slide One Body").is_some());
    harness.press(Key::Escape, Modifiers::NONE);
    harness.idle_frame();
    assert!(harness.editor.preview.is_none());

    // 4. Question bank slide disables "Preview this slide" and "Preview from here"
    harness.editor.selection.slide = "bs1".into();
    harness.editor.selection.also_slides.clear();
    harness.idle_frame();
    for label in ["Preview this slide", "Preview from here"] {
        open_top_preview_menu(&mut harness);
        let pos = harness
            .find_text_last(label)
            .unwrap_or_else(|| panic!("expected {label:?} in Preview menu"));
        harness.hover_at(pos);
        let tip = harness.probe_tooltip(pos);
        assert_eq!(
            tip.as_deref(),
            Some("Question bank slides can't be previewed directly"),
            "expected bank-slide disabled reason for {label:?}, got {tip:?}"
        );
        harness.click_at(pos);
        harness.idle_frame();
        assert!(
            harness.editor.preview.is_none(),
            "clicking disabled {label:?} on a bank slide must not open preview"
        );
        harness.press(Key::Escape, Modifiers::NONE);
        harness.idle_frame();
    }

    // 5. Slide masters mode disables all 4 items with "Not available in slide masters"
    harness.editor.enter_master_view();
    harness.idle_frame();
    for label in [
        "Preview this slide",
        "Preview from here",
        "Preview selected slides",
        "Preview whole course",
    ] {
        open_top_preview_menu(&mut harness);
        let pos = harness
            .find_text_last(label)
            .unwrap_or_else(|| panic!("expected {label:?} in Preview menu (masters)"));
        harness.hover_at(pos);
        let tip = harness.probe_tooltip(pos);
        assert_eq!(
            tip.as_deref(),
            Some("Not available in slide masters"),
            "expected slide-masters disabled reason for {label:?}, got {tip:?}"
        );
        harness.click_at(pos);
        harness.idle_frame();
        assert!(
            harness.editor.preview.is_none(),
            "clicking disabled {label:?} in slide masters must not open preview"
        );
        harness.press(Key::Escape, Modifiers::NONE);
        harness.idle_frame();
    }
}
