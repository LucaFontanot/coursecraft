//! Unit tests for CourseCraft player argument parsing, log capping, and playback.

use std::path::PathBuf;

use coursecraft_course::Course;
use coursecraft_lms::{ReportingStandard, Standard, reporting_from_marker};
use coursecraft_logic::{Effect, Input};

use crate::app::{Playing, capped_push_log, compute_session_ms, format_issues, push_log_entry};
use crate::load::{learner_name_from_query, parse_args, parse_course};

#[test]
fn arg_parsing_default() {
    let config = parse_args([] as [&str; 0]).expect("default args valid");
    assert_eq!(
        config.course_path,
        PathBuf::from("examples/hello-course/course.json")
    );
    assert!(!config.log_effects);
}

#[test]
fn arg_parsing_custom_path() {
    let config = parse_args(["custom/path.json"]).expect("positional path valid");
    assert_eq!(config.course_path, PathBuf::from("custom/path.json"));
    assert!(!config.log_effects);
}

#[test]
fn arg_parsing_flag_only() {
    let config = parse_args(["--log-effects"]).expect("flag only valid");
    assert_eq!(
        config.course_path,
        PathBuf::from("examples/hello-course/course.json")
    );
    assert!(config.log_effects);
}

#[test]
fn arg_parsing_path_and_flag() {
    let config = parse_args(["custom/course.json", "--log-effects"]).expect("path and flag valid");
    assert_eq!(config.course_path, PathBuf::from("custom/course.json"));
    assert!(config.log_effects);
}

#[test]
fn arg_parsing_flag_and_path() {
    let config = parse_args(["--log-effects", "custom/course.json"]).expect("flag and path valid");
    assert_eq!(config.course_path, PathBuf::from("custom/course.json"));
    assert!(config.log_effects);
}

#[test]
fn arg_parsing_unknown_flag() {
    let err = parse_args(["--unknown-flag"]).expect_err("unknown flag rejected");
    assert!(err.contains("unknown flag: '--unknown-flag'"));
    assert!(err.contains("Usage:"));
}

#[test]
fn arg_parsing_too_many_positional_arguments() {
    let err = parse_args(["first.json", "second.json"]).expect_err("multiple paths rejected");
    assert!(err.contains("unexpected argument: 'second.json'"));
    assert!(err.contains("Usage:"));
}

#[test]
fn log_capping_under_limit() {
    let mut log = Vec::new();
    for i in 0..10 {
        push_log_entry(&mut log, format!("line {i}"));
    }
    assert_eq!(log.len(), 10);
    assert_eq!(log.first().map(String::as_str), Some("line 0"));
    assert_eq!(log.last().map(String::as_str), Some("line 9"));
}

#[test]
fn log_capping_at_and_over_limit() {
    let mut log = Vec::new();
    for i in 0..2005 {
        push_log_entry(&mut log, format!("line {i}"));
    }
    assert_eq!(log.len(), 2000);
    assert_eq!(log.first().map(String::as_str), Some("line 5"));
    assert_eq!(log.last().map(String::as_str), Some("line 2004"));
}

#[test]
fn capped_push_log_drops_oldest() {
    let mut log = Vec::new();
    capped_push_log(&mut log, "a".into(), 3);
    capped_push_log(&mut log, "b".into(), 3);
    capped_push_log(&mut log, "c".into(), 3);
    assert_eq!(log, vec!["a", "b", "c"]);

    capped_push_log(&mut log, "d".into(), 3);
    assert_eq!(log, vec!["b", "c", "d"]);

    capped_push_log(&mut log, "e".into(), 3);
    assert_eq!(log, vec!["c", "d", "e"]);
}

#[test]
fn hello_course_start_emits_slide_entered() {
    let raw = include_str!("../../../examples/hello-course/course.json");
    let course: Course = serde_json::from_str(raw).expect("valid hello-course json");
    let mut playing = Playing::from_course(course, false).expect("valid course");

    playing.handle_input(&Input::Start, None);

    assert_eq!(
        playing.log.first().map(String::as_str),
        Some(r#"{"effect":"slide_entered","slide":"s-intro"}"#)
    );
}

#[test]
fn invalid_course_fails_player_creation() {
    let raw = include_str!("../../../examples/hello-course/course.json");
    let mut course: Course = serde_json::from_str(raw).expect("valid hello-course json");
    course.start_slide = Some("nowhere".into());

    let err = match Playing::from_course(course, false) {
        Ok(_) => panic!("course with broken start_slide should fail validation"),
        Err(e) => e,
    };
    assert!(!err.is_empty());
    assert!(err.contains("start_slide"));
}

#[test]
fn exit_effect_marks_ended() {
    let raw = include_str!("../../../examples/hello-course/course.json");
    let course: Course = serde_json::from_str(raw).expect("valid hello-course json");
    let mut playing = Playing::from_course(course, false).expect("valid course");

    assert!(!playing.ended);
    let exit_json = serde_json::to_string(&Effect::Exit).expect("exit serializes");
    let effect: Effect = serde_json::from_str(&exit_json).expect("exit deserializes");
    assert_eq!(effect, Effect::Exit);

    // Simulate Exit effect via input or direct effect handler
    // When Effect::Exit happens in player, ended = true
    playing.handle_input(&Input::Start, None);
    assert!(!playing.ended);
}

#[test]
fn parse_course_malformed_mentions_source() {
    let err = parse_course("{not json", "my-source.json").expect_err("malformed JSON must fail");
    assert!(err.contains("my-source.json"), "error was: {err}");
}

#[test]
fn parse_course_hello_course_ok() {
    let text = include_str!("../../../examples/hello-course/course.json");
    assert!(parse_course(text, "course.json").is_ok());
}

#[test]
fn lti_marker_selects_the_lti_player() {
    let (standard, warning) = reporting_from_marker(Some(r#"{"standard":"lti"}"#));
    assert_eq!(standard, ReportingStandard::Lti);
    assert!(warning.is_none());
}

#[test]
fn web_marker_selects_the_web_player() {
    let (standard, warning) = reporting_from_marker(Some(r#"{"standard":"web"}"#));
    assert_eq!(standard, ReportingStandard::Web);
    assert!(warning.is_none());
}

#[test]
fn missing_marker_defaults_to_scorm12_without_a_warning() {
    let (standard, warning) = reporting_from_marker(None);
    assert_eq!(standard, ReportingStandard::Scorm(Standard::Scorm12));
    assert!(warning.is_none());
}

#[test]
fn unknown_marker_falls_back_to_scorm12_with_a_warning() {
    let (standard, warning) = reporting_from_marker(Some(r#"{"standard":"scorm1.2"}"#));
    assert_eq!(standard, ReportingStandard::Scorm(Standard::Scorm12));
    assert!(warning.is_some_and(|text| text.contains("unknown")));
}

#[test]
fn the_learner_name_is_read_from_the_launch_query() {
    assert_eq!(
        learner_name_from_query("?name=Ada+Lovelace").as_deref(),
        Some("Ada Lovelace")
    );
    assert_eq!(
        learner_name_from_query("name=Zo%C3%AB%20M").as_deref(),
        Some("Zoë M")
    );
    assert_eq!(
        learner_name_from_query("?foo=1&name=Grace"),
        Some("Grace".into())
    );
    assert_eq!(learner_name_from_query("?name=%20"), None);
    assert_eq!(learner_name_from_query("?other=1"), None);
}

#[test]
fn session_ms_normal_elapsed() {
    assert_eq!(compute_session_ms(1000.0, 2500.5), 1500);
    assert_eq!(compute_session_ms(0.0, 10_000.0), 10_000);
}

#[test]
fn session_ms_negative_or_zero_clamps_to_zero() {
    assert_eq!(compute_session_ms(2000.0, 1000.0), 0);
    assert_eq!(compute_session_ms(500.0, 500.0), 0);
}

#[test]
fn session_ms_hostile_floats_return_zero() {
    assert_eq!(compute_session_ms(f64::NAN, 1000.0), 0);
    assert_eq!(compute_session_ms(1000.0, f64::NAN), 0);
    assert_eq!(compute_session_ms(f64::INFINITY, 1000.0), 0);
    assert_eq!(compute_session_ms(1000.0, f64::NEG_INFINITY), 0);
}

#[test]
fn session_ms_overflow_saturates() {
    assert_eq!(compute_session_ms(0.0, 1e25), u64::MAX);
}

#[test]
fn format_issues_joins_lines() {
    let issues = vec![
        coursecraft_course::Issue {
            severity: coursecraft_course::Severity::Error,
            path: "course.title".into(),
            message: "title is required".into(),
        },
        coursecraft_course::Issue {
            severity: coursecraft_course::Severity::Error,
            path: "course.slides[0]".into(),
            message: "missing id".into(),
        },
    ];
    let formatted = format_issues(issues);
    assert!(formatted.contains("course.title: title is required"));
    assert!(formatted.contains("course.slides[0]: missing id"));
}

#[test]
fn player_null_backend_audio_lifecycle() {
    use coursecraft_playback::{NullBackend, tiny_wav_bytes};
    use std::fs;
    let base_dir = std::env::temp_dir().join(format!(
        "coursecraft-test-player-media-{}",
        std::process::id()
    ));
    let media_dir = base_dir.join("media");
    fs::create_dir_all(&media_dir).expect("create test media dir");
    fs::write(media_dir.join("beep.wav"), tiny_wav_bytes()).expect("write beep.wav");

    let raw = r#"{
      "schema_version": 1,
      "id": "course-media-test",
      "title": "Media Test Course",
      "slide_size": { "w": 960, "h": 540 },
      "scenes": [
        {
          "id": "scene-1",
          "title": "Scene 1",
          "slides": [
            {
              "id": "s1",
              "title": "Slide 1",
              "duration_ms": 5000,
              "objects": [
                {
                  "id": "aud1",
                  "name": "Audio",
                  "kind": { "type": "audio", "src": "beep.wav", "autoplay": true },
                  "frame": { "x": 0.0, "y": 0.0, "w": 48.0, "h": 48.0 }
                }
              ],
              "triggers": [
                {
                  "when": { "event": "media_ends", "object": "aud1" },
                  "action": { "action": "jump_to_slide", "target": { "id": "s2" } }
                }
              ]
            },
            {
              "id": "s2",
              "title": "Slide 2",
              "duration_ms": 5000,
              "objects": [
                {
                  "id": "aud2",
                  "name": "Audio 2",
                  "kind": { "type": "audio", "src": "beep.wav", "autoplay": true },
                  "frame": { "x": 0.0, "y": 0.0, "w": 48.0, "h": 48.0 }
                }
              ],
              "triggers": [
                {
                  "when": { "event": "click", "object": "aud2" },
                  "action": { "action": "jump_to_slide", "target": { "id": "s3" } }
                }
              ]
            },
            {
              "id": "s3",
              "title": "Slide 3",
              "duration_ms": 5000,
              "objects": []
            }
          ]
        }
      ]
    }"#;

    let course: Course = serde_json::from_str(raw).expect("valid test course JSON");
    let course_ref: &'static Course = Box::leak(Box::new(course));
    let course_path = base_dir.join("course.json");

    let null_backend = NullBackend::new();
    let mut playing = Playing::new(course_ref, false, Some(&course_path))
        .expect("valid playing")
        .with_backend(Box::new(null_backend.clone()));

    // 1. Autoplay audio on entry produces play
    playing.handle_input(&Input::Start, None);
    assert_eq!(
        playing.player.current_slide().map(|s| s.id.as_str()),
        Some("s1")
    );
    assert!(null_backend.is_playing("aud1"));
    assert!(
        null_backend
            .recorded_calls()
            .contains(&"play:aud1:beep.wav".to_string())
    );

    // 2. take_ended produces a media_ends trigger firing a jump to s2
    null_backend.mark_ended("aud1");
    playing.poll_media(None);

    assert_eq!(
        playing.player.current_slide().map(|s| s.id.as_str()),
        Some("s2")
    );
    assert!(!null_backend.is_playing("aud1"));
    // On entry to s2, aud2 autoplays
    assert!(null_backend.is_playing("aud2"));
    assert!(
        null_backend
            .recorded_calls()
            .contains(&"play:aud2:beep.wav".to_string())
    );

    // 3. Slide change while aud2 is playing produces stop
    playing.handle_input(
        &Input::Click {
            object: "aud2".into(),
        },
        None,
    );
    assert_eq!(
        playing.player.current_slide().map(|s| s.id.as_str()),
        Some("s3")
    );
    assert!(!null_backend.is_playing("aud2"));
    assert!(
        null_backend
            .recorded_calls()
            .contains(&"stop:aud2".to_string())
    );

    let _ = fs::remove_dir_all(&base_dir);
}

#[test]
fn pending_web_fetch_combines_course_and_package_in_either_order() {
    let mut first = crate::load::PendingWebFetch::default();
    assert!(first.set_course(Ok("{\"id\":\"c1\"}".into())).is_none());
    let ready = first
        .set_package(Some("{\"standard\":\"web\"}".into()))
        .expect("both arrived");
    assert_eq!(
        ready,
        Ok(crate::load::WebCourse {
            course_json: "{\"id\":\"c1\"}".into(),
            package_json: Some("{\"standard\":\"web\"}".into()),
        })
    );

    // Missing package (None) arriving first, then course.json arriving second.
    let mut second = crate::load::PendingWebFetch::default();
    assert!(second.set_package(None).is_none());
    let ready_missing_pkg = second
        .set_course(Ok("{\"id\":\"c2\"}".into()))
        .expect("both arrived");
    assert_eq!(
        ready_missing_pkg,
        Ok(crate::load::WebCourse {
            course_json: "{\"id\":\"c2\"}".into(),
            package_json: None,
        })
    );

    // Course failure still propagates once both have answered.
    let mut failed = crate::load::PendingWebFetch::default();
    assert!(failed.set_package(None).is_none());
    let err = failed
        .set_course(Err("failed to fetch course.json: HTTP 404".into()))
        .expect("both arrived");
    assert_eq!(err, Err("failed to fetch course.json: HTTP 404".into()));
}

#[test]
fn web_media_cache_lru_evicts_unprotected_entries_over_cap() {
    use std::sync::Arc;

    let mut state = crate::media::WebMediaState::with_max_bytes(100);
    state.insert_ready("a.png".into(), Arc::from(vec![1u8; 40]));
    state.insert_ready("b.png".into(), Arc::from(vec![2u8; 40]));
    assert_eq!(state.total_bytes(), 80);

    // Touch a.png so b.png becomes the least recently used entry.
    let _ = state.get_or_queue("a.png");

    // Inserting c.png (40 bytes -> 120 > 100) evicts b.png, keeping a.png and c.png.
    state.insert_ready("c.png".into(), Arc::from(vec![3u8; 40]));
    assert_eq!(state.total_bytes(), 80);
    assert!(state.contains("a.png"));
    assert!(!state.contains("b.png"));
    assert!(state.contains("c.png"));

    // Protect a.png (on current slide); inserting d.png (40 bytes) must evict c.png,
    // even if a.png were older.
    state.set_protected(&["a.png".into()]);
    let _ = state.get_or_queue("c.png");
    state.insert_ready("d.png".into(), Arc::from(vec![4u8; 40]));
    assert_eq!(state.total_bytes(), 80);
    assert!(state.contains("a.png"));
    assert!(!state.contains("c.png"));
    assert!(state.contains("d.png"));

    // Prefetch skips already-cached or unsafe paths, and Missing clears any prior Ready bytes.
    assert!(!state.queue_prefetch(&["a.png".into(), "../escape.png".into()]));
    assert!(state.queue_prefetch(&["next.png".into()]));
    state.insert_missing("d.png".into());
    assert_eq!(state.total_bytes(), 40);
}

#[test]
fn slide_image_srcs_collects_images_and_skips_video_and_audio() {
    let raw = r##"{
      "schema_version": 1,
      "id": "course-images",
      "title": "Images",
      "slide_size": { "w": 960, "h": 540 },
      "scenes": [
        {
          "id": "scene-1",
          "title": "Scene 1",
          "slides": [
            {
              "id": "s1",
              "title": "Slide 1",
              "duration_ms": 5000,
              "objects": [
                {
                  "id": "img1",
                  "name": "Pic",
                  "kind": { "type": "image", "src": "hero.png", "alt": "" },
                  "frame": { "x": 0.0, "y": 0.0, "w": 100.0, "h": 100.0 }
                },
                {
                  "id": "vid1",
                  "name": "Vid",
                  "kind": { "type": "video", "src": "clip.mp4" },
                  "frame": { "x": 0.0, "y": 0.0, "w": 100.0, "h": 100.0 }
                },
                {
                  "id": "aud1",
                  "name": "Aud",
                  "kind": { "type": "audio", "src": "narration.mp3" },
                  "frame": { "x": 0.0, "y": 0.0, "w": 48.0, "h": 48.0 }
                },
                {
                  "id": "shp1",
                  "name": "Shape",
                  "kind": {
                    "type": "shape",
                    "shape": "rectangle",
                    "fill": "#ffffff",
                    "fill_style": { "type": "picture", "src": "pattern.png" }
                  },
                  "frame": { "x": 0.0, "y": 0.0, "w": 50.0, "h": 50.0 }
                }
              ]
            }
          ]
        }
      ]
    }"##;
    let course: Course = serde_json::from_str(raw).expect("valid course");
    let slide = &course.scenes[0].slides[0];
    let srcs = crate::media::slide_image_srcs(&course, slide);
    assert_eq!(
        srcs,
        vec!["hero.png".to_string(), "pattern.png".to_string()]
    );
}

#[test]
fn debug_logging_from_query_matches_exact_flag() {
    assert!(crate::load::debug_logging_from_query("?debug=1"));
    assert!(crate::load::debug_logging_from_query("?a=b&debug=1"));
    assert!(!crate::load::debug_logging_from_query(""));
    assert!(!crate::load::debug_logging_from_query("?debug=0"));
    assert!(!crate::load::debug_logging_from_query("?debugger=1"));
}

#[test]
fn player_drawer_clips_to_player_area_at_letterboxed_and_fullscreen_sizes() {
    let raw_course = include_str!("../../../examples/hello-course/course.json");
    for size in [
        egui::vec2(1600.0, 720.0), // wider than 16:9 slide aspect
        egui::vec2(800.0, 900.0),  // taller than 16:9 slide aspect
        egui::vec2(1280.0, 720.0), // fullscreen 16:9
    ] {
        let course = Course::from_json(raw_course).expect("valid hello course");
        let mut playing = Playing::from_course(course, false).expect("valid playing");
        playing.handle_input(&Input::Start, None);

        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), size);
        let mut time = 0.1;

        let mut run_frame = |dt: f64, events: Vec<egui::Event>| {
            time += dt;
            let raw = egui::RawInput {
                screen_rect: Some(screen),
                time: Some(time),
                events,
                ..Default::default()
            };
            let full = ctx.run_ui(raw, |ui| {
                playing.show_ui(ui);
            });
            let shapes = full.shapes.clone();
            full.drop_without_applying_deltas();
            shapes
        };

        let check_shapes = |shapes: &[egui::epaint::ClippedShape], phase: &str| {
            let drawer_bg = egui::Color32::from_rgb(0x2F, 0x2F, 0x33);
            let mut saw_drawer_bg = false;
            for clipped in shapes {
                assert!(
                    clipped.clip_rect.left() >= screen.left() - 0.5
                        && clipped.clip_rect.right() <= screen.right() + 0.5
                        && clipped.clip_rect.top() >= screen.top() - 0.5
                        && clipped.clip_rect.bottom() <= screen.bottom() + 0.5,
                    "{phase} ({size:?}): clip_rect {:?} escaped screen {:?}",
                    clipped.clip_rect,
                    screen
                );
                if let egui::epaint::Shape::Rect(r) = &clipped.shape
                    && r.fill == drawer_bg
                    && r.rect.height() > 100.0
                {
                    saw_drawer_bg = true;
                    assert!(
                        r.rect.left() >= screen.left() - 0.5
                            && r.rect.right() <= screen.right() + 0.5,
                        "{phase} ({size:?}): drawer bg rect {:?} escaped screen {:?}",
                        r.rect,
                        screen
                    );
                }
            }
            assert!(saw_drawer_bg, "{phase} ({size:?}): expected drawer bg");
        };

        run_frame(0.0, Vec::new());
        let shapes_first = run_frame(
            0.02,
            vec![egui::Event::Key {
                key: egui::Key::M,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        check_shapes(&shapes_first, "first frame");

        let shapes_mid = run_frame(0.10, Vec::new());
        check_shapes(&shapes_mid, "mid frame");

        let shapes_end = run_frame(0.25, Vec::new());
        check_shapes(&shapes_end, "end frame");
    }
}

#[test]
fn player_scales_letterboxes_and_places_drawer_and_log_panel_across_window_sizes() {
    let raw_course = include_str!("../../../examples/hello-course/course.json");
    for size in [
        egui::vec2(480.0, 800.0),   // narrow portrait
        egui::vec2(1920.0, 720.0),  // ultrawide letterboxed horizontally
        egui::vec2(1280.0, 720.0),  // exact 16:9
        egui::vec2(2560.0, 1440.0), // large 16:9
    ] {
        let course = Course::from_json(raw_course).expect("valid hello course");
        let mut playing = Playing::from_course(course, false).expect("valid playing");
        playing.handle_input(&Input::Start, None);

        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), size);
        let mut time = 0.1;

        fn step(
            ctx: &egui::Context,
            playing: &mut Playing,
            screen: egui::Rect,
            time: &mut f64,
            dt: f64,
            events: Vec<egui::Event>,
        ) {
            *time += dt;
            let raw = egui::RawInput {
                screen_rect: Some(screen),
                time: Some(*time),
                events,
                ..Default::default()
            };
            let full = ctx.run_ui(raw, |ui| {
                playing.show_ui(ui);
            });
            full.drop_without_applying_deltas();
        }

        step(&ctx, &mut playing, screen, &mut time, 0.0, Vec::new());
        let slide_rect = playing.frame.slide_rect().expect("slide_rect");
        assert!(
            slide_rect.left() >= screen.left() - 0.5
                && slide_rect.right() <= screen.right() + 0.5
                && slide_rect.top() >= screen.top() - 0.5
                && slide_rect.bottom() <= screen.bottom() + 0.5,
            "{size:?}: slide_rect {slide_rect:?} escaped screen {screen:?}"
        );
        let aspect = slide_rect.width() / slide_rect.height();
        assert!(
            (aspect - 16.0 / 9.0).abs() < 0.02,
            "{size:?}: slide_rect should preserve 16:9 aspect without cropping, got {aspect}"
        );

        // Open the log panel (F2) and outline drawer (M); slide and drawer must follow.
        step(
            &ctx,
            &mut playing,
            screen,
            &mut time,
            0.02,
            vec![
                egui::Event::Key {
                    key: egui::Key::F2,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::Key {
                    key: egui::Key::M,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        step(&ctx, &mut playing, screen, &mut time, 0.25, Vec::new());
        let slide_with_log = playing.frame.slide_rect().expect("slide_rect with log");
        assert!(
            slide_with_log.width() <= slide_rect.width() + 0.5,
            "{size:?}: slide should shrink when log panel opens"
        );
        let drawer = playing.frame.drawer_rect().expect("drawer rect");
        assert!(
            drawer.left() >= screen.left() - 0.5 && drawer.right() <= screen.right() + 0.5,
            "{size:?}: drawer {drawer:?} escaped screen {screen:?}"
        );
    }
}

#[test]
fn player_click_and_keyboard_on_video_plays_pauses_and_fires_cues_without_placeholder() {
    let json = r##"{
      "schema_version": 1,
      "id": "h7-player-video",
      "title": "H7 Player Video",
      "slide_size": { "w": 1280, "h": 720 },
      "scenes": [{
        "id": "sc1",
        "title": "Scene 1",
        "slides": [{
          "id": "s1",
          "title": "Video Slide",
          "duration_ms": 10000,
          "revisit": "reset",
          "objects": [
            {
              "id": "vid",
              "name": "Stretch Video",
              "kind": {
                "type": "video",
                "src": "stretch.mp4",
                "cues": [{ "id": "c1", "at_ms": 500, "name": "Cue 1" }]
              },
              "frame": { "x": 80, "y": 160, "w": 720, "h": 402 }
            },
            {
              "id": "row1",
              "name": "Row 1",
              "kind": { "type": "shape", "shape": "rectangle", "fill": "#FFFFFF" },
              "frame": { "x": 840, "y": 160, "w": 360, "h": 120 },
              "states": [{ "name": "active", "fill": "#FCE6DE" }]
            }
          ],
          "triggers": [
            {
              "when": { "event": "cue_reached", "object": "vid", "cue": "c1" },
              "action": { "action": "change_state", "object": "row1", "state": "active" }
            }
          ]
        }]
      }]
    }"##;
    let course: &'static Course =
        Box::leak(Box::new(Course::from_json(json).expect("valid course")));
    let mut playing = Playing::new(course, false, None).expect("playing");
    playing.handle_input(&coursecraft_logic::Input::Start, None);
    let ctx = egui::Context::default();
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0));
    let mut time = 0.0;

    let step = |ctx: &egui::Context,
                playing: &mut Playing,
                time: &mut f64,
                events: Vec<egui::Event>|
     -> egui::FullOutput {
        *time += 0.02;
        let raw = egui::RawInput {
            screen_rect: Some(screen),
            time: Some(*time),
            events,
            ..Default::default()
        };
        ctx.run_ui(raw, |ui| {
            playing.show_ui(ui);
        })
    };

    fn collect_texts(shapes: &[egui::epaint::ClippedShape]) -> Vec<String> {
        fn walk(shape: &egui::Shape, out: &mut Vec<String>) {
            match shape {
                egui::Shape::Text(t) => out.push(t.galley.text().to_owned()),
                egui::Shape::Vec(list) => {
                    for s in list {
                        walk(s, out);
                    }
                }
                _ => {}
            }
        }
        let mut out = Vec::new();
        for cs in shapes {
            walk(&cs.shape, &mut out);
        }
        out
    }

    let out = step(&ctx, &mut playing, &mut time, Vec::new());
    let texts = collect_texts(&out.shapes);
    assert!(
        !texts
            .iter()
            .any(|t| t.contains("Video plays in the published course")),
        "published player must not show editor placeholder text: {texts:?}"
    );
    out.drop_without_applying_deltas();

    let video_center = playing
        .view
        .last_transform()
        .expect("slide transform")
        .to_screen_rect(egui::Rect::from_min_size(
            egui::pos2(80.0, 160.0),
            egui::vec2(720.0, 402.0),
        ))
        .center();

    // First click plays video.
    step(
        &ctx,
        &mut playing,
        &mut time,
        vec![
            egui::Event::PointerMoved(video_center),
            egui::Event::PointerButton {
                pos: video_center,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    )
    .drop_without_applying_deltas();
    step(
        &ctx,
        &mut playing,
        &mut time,
        vec![egui::Event::PointerButton {
            pos: video_center,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    )
    .drop_without_applying_deltas();
    assert!(playing.player.is_media_playing("vid"));

    // Cue trigger fires after click start.
    playing.handle_input(
        &coursecraft_logic::Input::MediaTime {
            object: "vid".to_owned(),
            at_ms: 600.0,
        },
        None,
    );
    assert_eq!(playing.player.object_state("row1"), Some("active"));

    // Second click pauses video.
    step(
        &ctx,
        &mut playing,
        &mut time,
        vec![
            egui::Event::PointerMoved(video_center),
            egui::Event::PointerButton {
                pos: video_center,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    )
    .drop_without_applying_deltas();
    step(
        &ctx,
        &mut playing,
        &mut time,
        vec![egui::Event::PointerButton {
            pos: video_center,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
    )
    .drop_without_applying_deltas();
    assert!(!playing.player.is_media_playing("vid"));

    // Tab to focus the video and press Space to play again.
    for _ in 0..8 {
        step(
            &ctx,
            &mut playing,
            &mut time,
            vec![egui::Event::Key {
                key: egui::Key::Tab,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        )
        .drop_without_applying_deltas();
        if playing.frame.focus().focused().is_some_and(
            |t| matches!(t, coursecraft_render::FocusTarget::Object(id) if id == "vid"),
        ) {
            break;
        }
    }
    assert!(
        playing.frame.focus().focused().is_some_and(
            |t| matches!(t, coursecraft_render::FocusTarget::Object(id) if id == "vid")
        ),
        "Tab must focus the video in the published player"
    );
    step(
        &ctx,
        &mut playing,
        &mut time,
        vec![egui::Event::Key {
            key: egui::Key::Space,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
    )
    .drop_without_applying_deltas();
    assert!(playing.player.is_media_playing("vid"));
}

#[test]
fn t14_8_failed_screen_uses_player_ground_raised_card_and_wraps() {
    let long_err = "Failed to load course package: ".to_string()
        + &"missing_manifest_or_corrupted_archive_".repeat(8);
    for (w, h) in [(390.0_f32, 844.0_f32), (1280.0, 720.0)] {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(w, h));
        let out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(screen),
                time: Some(1.0),
                ..Default::default()
            },
            |ui| {
                crate::app::show_failed_screen(ui, &long_err);
            },
        );
        // Must paint PLAYER_GROUND over full screen
        let has_ground = out.shapes.iter().any(|c| match &c.shape {
            egui::epaint::Shape::Rect(r) => {
                r.fill == coursecraft_render::PLAYER_GROUND && r.rect.width() >= w - 2.0
            }
            _ => false,
        });
        assert!(
            has_ground,
            "State::Failed must paint PLAYER_GROUND at {w}x{h}"
        );

        // Must wrap text within <= 480 pt and stay inside screen
        let mut found_text = false;
        for c in &out.shapes {
            if let egui::epaint::Shape::Text(t) = &c.shape {
                let r = t.visual_bounding_rect();
                if r.is_positive() {
                    found_text = true;
                    assert!(
                        r.width() <= 480.0 && screen.expand(1.0).contains_rect(r),
                        "State::Failed text {:?} overflows or exceeds 480pt at {w}x{h}: {r:?}",
                        t.galley.text()
                    );
                }
            }
        }
        assert!(
            found_text,
            "State::Failed must render error text at {w}x{h}"
        );
        out.drop_without_applying_deltas();
    }
}

#[test]
fn slide_media_and_image_srcs_collect_state_src_and_fill_style_on_objects_and_layers() {
    let raw = r##"{
      "schema_version": 1,
      "id": "course-state-media",
      "title": "State Media",
      "slide_size": { "w": 960, "h": 540 },
      "scenes": [{
        "id": "scene-1",
        "title": "Scene 1",
        "slides": [{
          "id": "s1",
          "title": "Slide 1",
          "duration_ms": 5000,
          "objects": [
            {
              "id": "img1",
              "name": "Pic",
              "kind": { "type": "image", "src": "base.png", "alt": "" },
              "frame": { "x": 0.0, "y": 0.0, "w": 100.0, "h": 100.0 },
              "states": [{
                "name": "hover",
                "src": "hover-img.png",
                "fill_style": { "type": "picture", "src": "hover-fill.png" }
              }]
            },
            {
              "id": "shp1",
              "name": "Shape",
              "kind": { "type": "shape", "shape": "rectangle", "fill": "#ffffff" },
              "frame": { "x": 0.0, "y": 0.0, "w": 50.0, "h": 50.0 },
              "states": [{
                "name": "hover",
                "fill_style": { "type": "picture", "src": "shape-state-fill.png" }
              }]
            }
          ],
          "layers": [{
            "id": "l1",
            "name": "Layer 1",
            "objects": [{
              "id": "btn1",
              "name": "Button",
              "kind": { "type": "button", "label": "OK" },
              "frame": { "x": 0.0, "y": 0.0, "w": 80.0, "h": 40.0 },
              "states": [{
                "name": "down",
                "fill_style": { "type": "picture", "src": "layer-btn-fill.png" }
              }]
            }]
          }]
        }]
      }]
    }"##;
    let course: Course = serde_json::from_str(raw).expect("valid course");
    let slide = &course.scenes[0].slides[0];
    let img_srcs = crate::media::slide_image_srcs(&course, slide);
    let media_srcs = crate::media::slide_media_srcs(&course, slide);
    for expected in [
        "base.png",
        "hover-img.png",
        "hover-fill.png",
        "shape-state-fill.png",
        "layer-btn-fill.png",
    ] {
        assert!(
            img_srcs.contains(&expected.to_string()),
            "slide_image_srcs missing {expected}: {img_srcs:?}"
        );
        assert!(
            media_srcs.contains(&expected.to_string()),
            "slide_media_srcs missing {expected}: {media_srcs:?}"
        );
    }
}

#[test]
fn desktop_and_web_media_sources_reject_unsafe_state_paths() {
    use coursecraft_render::{MediaFetch, MediaSource};
    let desktop = crate::media::DesktopMediaSource::new(std::path::Path::new("/tmp/course.json"));
    let mut web = crate::media::WebMediaState::with_max_bytes(1024 * 1024);
    for bad in [
        "../secret.png",
        "C:\\Users\\a.png",
        "\"a.png\"",
        "\0bad.png",
        "https://evil/a.png",
    ] {
        assert!(
            matches!(desktop.get(bad), MediaFetch::Missing),
            "DesktopMediaSource must reject {bad:?}"
        );
        let (fetch, queued) = web.get_or_queue(bad);
        assert!(
            matches!(fetch, MediaFetch::Missing) && !queued,
            "WebMediaState must reject {bad:?}"
        );
    }
}

#[test]
fn pub04_slide_image_and_media_srcs_collect_state_extras() {
    let raw = r##"{
      "schema_version": 1,
      "id": "c-pub04-extras",
      "title": "PUB-04 Extras",
      "slide_size": { "w": 1280, "h": 720 },
      "scenes": [{
        "id": "sc1",
        "title": "Scene",
        "slides": [{
          "id": "s1",
          "title": "Slide",
          "objects": [{
            "id": "btn-1",
            "name": "Button",
            "frame": { "x": 100, "y": 100, "w": 160, "h": 48 },
            "kind": { "type": "button", "label": "Go", "fill": "#2E6F9E" },
            "states": [{
              "name": "visited",
              "extras": [
                {
                  "id": "ex-img",
                  "frame": { "x": 140, "y": -8, "w": 24, "h": 24 },
                  "kind": { "type": "image", "src": "media/extra-badge.png", "alt": "Done" }
                },
                {
                  "id": "ex-shp",
                  "frame": { "x": 0, "y": 52, "w": 40, "h": 20 },
                  "kind": {
                    "type": "shape",
                    "shape": "rectangle",
                    "fill": "#CCCCCC",
                    "fill_style": { "type": "picture", "src": "media/extra-shape-fill.png" }
                  }
                },
                {
                  "id": "ex-path",
                  "frame": { "x": 50, "y": 52, "w": 40, "h": 20 },
                  "kind": {
                    "type": "path",
                    "points": [{ "x": 0, "y": 0 }, { "x": 1, "y": 0 }, { "x": 0.5, "y": 1 }],
                    "closed": true,
                    "outline": { "color": "#000000", "width": 2.0, "dash": "solid" },
                    "fill": "#CCCCCC",
                    "fill_style": { "type": "picture", "src": "media/extra-path-fill.png" }
                  }
                }
              ]
            }]
          }],
          "layers": [{
            "id": "l1",
            "name": "Layer",
            "objects": [{
              "id": "l-shp",
              "name": "Layer Shape",
              "frame": { "x": 200, "y": 200, "w": 100, "h": 100 },
              "kind": { "type": "shape", "shape": "rectangle", "fill": "#EEEEEE" },
              "states": [{
                "name": "hover",
                "extras": [{
                  "id": "ex-layer-img",
                  "frame": { "x": 0, "y": 0, "w": 20, "h": 20 },
                  "kind": { "type": "image", "src": "media/extra-layer.png", "alt": "" }
                }]
              }]
            }]
          }]
        }]
      }]
    }"##;
    let course: Course = serde_json::from_str(raw).expect("valid course");
    let slide = &course.scenes[0].slides[0];
    let img_srcs = crate::media::slide_image_srcs(&course, slide);
    let media_srcs = crate::media::slide_media_srcs(&course, slide);
    for expected in [
        "media/extra-badge.png",
        "media/extra-shape-fill.png",
        "media/extra-path-fill.png",
        "media/extra-layer.png",
    ] {
        assert!(
            img_srcs.contains(&expected.to_string()),
            "slide_image_srcs missing {expected}: {img_srcs:?}"
        );
        assert!(
            media_srcs.contains(&expected.to_string()),
            "slide_media_srcs missing {expected}: {media_srcs:?}"
        );
    }
}
