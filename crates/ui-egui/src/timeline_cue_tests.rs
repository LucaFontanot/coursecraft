//! Tests for timeline slide cue points, cue snapping, and audio waveform caching.

use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use egui::{Event, Key, Modifiers};

fn hello() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    Course::from_json(&json).expect("valid course")
}

#[test]
fn add_move_rename_delete_slide_cue_and_undo() {
    let mut harness = TestHarness::new(hello());
    harness.editor.selection.slide = "s-intro".into();
    harness.editor.ui.bottom_tab = crate::BottomTab::Timeline;
    harness.idle_frame();

    // Focus the timeline by clicking inside its panel rect, then press C at playhead 1500ms.
    let panel_center = harness
        .editor
        .ui
        .timeline
        .panel_rect
        .expect("timeline panel rect")
        .center();
    harness.click_at(panel_center);
    harness.editor.ui.timeline.playhead_ms = 1500;
    harness.frame(vec![Event::Key {
        key: Key::C,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);

    let cues = &harness
        .editor
        .session
        .course
        .slide("s-intro")
        .unwrap()
        .cue_points;
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].at_ms, 1500);
    let cue_id = cues[0].id.clone();
    let orig_name = cues[0].name.clone();

    // Move cue to 2500ms.
    harness.editor.run(
        "slide.set_cue",
        serde_json::json!({
            "slide": "s-intro",
            "cue": cue_id,
            "patch": { "at_ms": 2500 },
        }),
    );
    assert_eq!(
        harness
            .editor
            .session
            .course
            .slide("s-intro")
            .unwrap()
            .cue_points[0]
            .at_ms,
        2500
    );

    // Rename cue.
    harness.editor.run(
        "slide.set_cue",
        serde_json::json!({
            "slide": "s-intro",
            "cue": cue_id,
            "patch": { "name": "Verse 1" },
        }),
    );
    assert_eq!(
        harness
            .editor
            .session
            .course
            .slide("s-intro")
            .unwrap()
            .cue_points[0]
            .name,
        "Verse 1"
    );

    // Delete cue via Delete key while selected.
    harness.editor.ui.timeline.slide_cue_selected = Some((None, cue_id.clone()));
    harness.frame(vec![Event::Key {
        key: Key::Delete,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    assert!(
        harness
            .editor
            .session
            .course
            .slide("s-intro")
            .unwrap()
            .cue_points
            .is_empty()
    );

    // Undo delete -> cue is back with renamed name at 2500ms.
    harness.editor.session.undo().unwrap();
    let cues = &harness
        .editor
        .session
        .course
        .slide("s-intro")
        .unwrap()
        .cue_points;
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].name, "Verse 1");
    assert_eq!(cues[0].at_ms, 2500);

    // Undo rename -> original name.
    harness.editor.session.undo().unwrap();
    assert_eq!(
        harness
            .editor
            .session
            .course
            .slide("s-intro")
            .unwrap()
            .cue_points[0]
            .name,
        orig_name
    );

    // Undo move -> back to 1500ms.
    harness.editor.session.undo().unwrap();
    assert_eq!(
        harness
            .editor
            .session
            .course
            .slide("s-intro")
            .unwrap()
            .cue_points[0]
            .at_ms,
        1500
    );

    // Undo add -> empty again.
    harness.editor.session.undo().unwrap();
    assert!(
        harness
            .editor
            .session
            .course
            .slide("s-intro")
            .unwrap()
            .cue_points
            .is_empty()
    );
}

#[test]
fn object_drag_snaps_to_slide_cue_point() {
    let mut harness = TestHarness::new(hello());
    harness.editor.selection.slide = "s-intro".into();
    harness.editor.ui.bottom_tab = crate::BottomTab::Timeline;
    harness.editor.run(
        "slide.add_cue",
        serde_json::json!({
            "slide": "s-intro",
            "at_ms": 2450,
            "name": "SnapTarget",
        }),
    );
    harness.idle_frame();

    let bar_rect = *harness
        .editor
        .ui
        .timeline
        .object_bar_rects
        .get("btn-start")
        .expect("btn-start bar rect");
    let right_handle = *harness
        .editor
        .ui
        .timeline
        .object_right_handle_rects
        .get("btn-start")
        .expect("btn-start right handle");

    // Drag right handle to ~2450ms (slide duration is 5000ms).
    let track_width = bar_rect.width();
    let target_x = bar_rect.min.x + (2450.0 / 5000.0) * track_width;
    let start_pos = right_handle.center();
    let end_pos = egui::pos2(target_x, start_pos.y);

    harness.frame(vec![
        Event::PointerMoved(start_pos),
        Event::PointerButton {
            pos: start_pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
    ]);
    harness.frame(vec![Event::PointerMoved(end_pos)]);
    harness.frame(vec![Event::PointerButton {
        pos: end_pos,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    }]);

    let (obj, _) = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .unwrap()
        .object("btn-start")
        .unwrap();
    assert_eq!(obj.timing.end_ms, Some(2450));
}

#[test]
fn audio_waveform_is_cached_on_import() {
    let mut harness = TestHarness::new(hello());
    // Minimal 16-bit mono PCM WAV (0.5s @ 8kHz)
    let sample_rate: u32 = 8_000;
    let num_samples: usize = 4_000;
    let data_bytes = (num_samples * 2) as u32;
    let mut wav = Vec::with_capacity(44 + data_bytes as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&1u16.to_le_bytes()); // mono
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_bytes.to_le_bytes());
    for i in 0..num_samples {
        let phase = std::f64::consts::TAU * 440.0 * i as f64 / f64::from(sample_rate);
        let s = (phase.sin() * 30_000.0) as i16;
        wav.extend_from_slice(&s.to_le_bytes());
    }

    let src = harness
        .editor
        .import_audio("sine.wav", &wav)
        .expect("import_audio");
    let cached = harness
        .editor
        .audio_waveforms
        .get(&src)
        .expect("waveform should be cached by media id")
        .as_ref()
        .expect("synthetic WAV should decode to peaks");
    assert!(!cached.is_empty() && cached.len() <= coursecraft_playback::MAX_WAVEFORM_PEAKS);
    assert!(cached.iter().all(|p| (0.0..=1.0).contains(p)));
}
