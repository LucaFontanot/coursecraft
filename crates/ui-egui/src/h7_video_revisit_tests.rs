//! Headless tests for Step h7: clickable video in preview and revisit warning in Check Course.

use crate::properties::InspectorTab;
use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use coursecraft_logic::MediaPlaybackState;
use egui::{Key, Modifiers};

fn video_cue_course(revisit: &str) -> Course {
    let json = format!(
        r##"{{
          "schema_version": 1,
          "id": "h7-preview-video",
          "title": "H7 Preview Video",
          "slide_size": {{ "w": 1280, "h": 720 }},
          "scenes": [{{
            "id": "sc1",
            "title": "Scene 1",
            "slides": [{{
              "id": "s1",
              "title": "Stretch Video",
              "duration_ms": 10000,
              "revisit": "{revisit}",
              "objects": [
                {{
                  "id": "vid",
                  "name": "Stretch Video",
                  "kind": {{
                    "type": "video",
                    "src": "stretch.mp4",
                    "cues": [{{ "id": "c1", "at_ms": 500, "name": "Cue 1" }}]
                  }},
                  "frame": {{ "x": 80, "y": 160, "w": 720, "h": 402 }}
                }},
                {{
                  "id": "row1",
                  "name": "Row 1",
                  "kind": {{ "type": "shape", "shape": "rectangle", "fill": "#FFFFFF" }},
                  "frame": {{ "x": 840, "y": 160, "w": 360, "h": 120 }},
                  "states": [{{ "name": "active", "fill": "#FCE6DE" }}]
                }}
              ],
              "triggers": [
                {{
                  "when": {{ "event": "cue_reached", "object": "vid", "cue": "c1" }},
                  "action": {{ "action": "change_state", "object": "row1", "state": "active" }}
                }}
              ]
            }}]
          }}]
        }}"##
    );
    Course::from_json(&json).expect("valid course")
}

#[test]
fn preview_video_click_plays_pauses_fires_cues_keyboard_starts_and_no_placeholder_text() {
    let mut harness = TestHarness::new(video_cue_course("reset"));
    harness.idle_frame();

    // Editor canvas still shows the placeholder text.
    assert!(
        harness
            .find_text_containing("Video plays in the published course")
            .is_some(),
        "editor canvas must show video placeholder text"
    );

    // Open desktop preview.
    harness.press(Key::F12, Modifiers::NONE);
    harness.idle_frame();

    // Preview must NOT show the placeholder text (its composed video item has show_placeholder = false).
    {
        let preview = harness.editor.preview.as_ref().expect("preview open");
        let has_placeholder = preview.view.last_items().iter().any(|item| {
            matches!(
                item.visual,
                coursecraft_render::Visual::Video {
                    show_placeholder: true,
                    ..
                }
            )
        });
        assert!(
            !has_placeholder,
            "preview must not compose video with show_placeholder = true"
        );
    }

    let video_center = {
        let preview = harness.editor.preview.as_ref().expect("preview open");
        let t = preview.view.last_transform().expect("slide transform");
        t.to_screen_rect(egui::Rect::from_min_size(
            egui::pos2(80.0, 160.0),
            egui::vec2(720.0, 402.0),
        ))
        .center()
    };

    // First click on the video starts playback.
    harness.click_at(video_center);
    harness.idle_frame();
    {
        let preview = harness.editor.preview.as_ref().expect("preview open");
        let st = preview.state.as_ref().expect("player state");
        assert_eq!(
            st.media_playback.get("vid"),
            Some(&MediaPlaybackState::Playing),
            "clicking video in preview must start playback"
        );
    }

    // Advance simulated media time past cue c1 -> cue trigger fires.
    {
        let preview = harness.editor.preview.as_mut().expect("preview open");
        let mut player = coursecraft_logic::Player::resume(
            &preview.course,
            preview.state.take().expect("state"),
        )
        .expect("resume");
        let _ = player.handle(&coursecraft_logic::Input::MediaTime {
            object: "vid".to_owned(),
            at_ms: 600.0,
        });
        assert_eq!(player.object_state("row1"), Some("active"));
        preview.state = Some(player.state().clone());
    }

    // Second click on the video pauses playback.
    harness.click_at(video_center);
    harness.idle_frame();
    {
        let preview = harness.editor.preview.as_ref().expect("preview open");
        let st = preview.state.as_ref().expect("player state");
        assert_eq!(
            st.media_playback.get("vid"),
            Some(&MediaPlaybackState::Paused),
            "second click on video in preview must pause playback"
        );
    }

    // Keyboard focus on the video + Enter starts playback again.
    for _ in 0..8 {
        harness.press(Key::Tab, Modifiers::NONE);
        let focused_vid = harness
            .editor
            .preview
            .as_ref()
            .and_then(|p| p.frame.focus().focused())
            .is_some_and(
                |t| matches!(t, coursecraft_render::FocusTarget::Object(id) if id == "vid"),
            );
        if focused_vid {
            break;
        }
    }
    let focused_vid = harness
        .editor
        .preview
        .as_ref()
        .and_then(|p| p.frame.focus().focused())
        .is_some_and(|t| matches!(t, coursecraft_render::FocusTarget::Object(id) if id == "vid"));
    assert!(focused_vid, "Tab must reach the video object in preview");

    harness.press(Key::Enter, Modifiers::NONE);
    harness.idle_frame();
    {
        let preview = harness.editor.preview.as_ref().expect("preview open");
        let st = preview.state.as_ref().expect("player state");
        assert_eq!(
            st.media_playback.get("vid"),
            Some(&MediaPlaybackState::Playing),
            "Enter on focused video must start playback"
        );
    }
}

#[test]
fn check_course_revisit_warning_navigates_to_slide_tab() {
    let mut harness = TestHarness::new(video_cue_course("auto"));
    harness.editor.ui.inspector_tab = InspectorTab::Format;
    harness.editor.ui.show_validation_window = true;
    harness.idle_frame();

    let warning_pos = harness
        .find_text_containing("changes object states from its timeline or media")
        .expect("revisit warning button in Check Course");
    harness.click_at(warning_pos);
    harness.idle_frame();

    assert_eq!(harness.editor.selection.slide, "s1");
    assert_eq!(harness.editor.ui.inspector_tab, InspectorTab::Slide);
}
