//! Headless tests for the desktop preview window: backdrop dimming,
//! edge/corner resizing with min/max clamps, session size persistence,
//! full screen toggle + Esc order, and slide scaling + variables panel fit.

use crate::preview::{
    PREVIEW_BACKDROP_COLOR, PREVIEW_MARGIN, PREVIEW_MIN_HEIGHT, PREVIEW_MIN_WIDTH,
};
use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use egui::{Color32, Key, Modifiers, Rect, pos2, vec2};

fn test_course() -> Course {
    let json = r#"{
        "schema_version": 1,
        "id": "preview-window-test",
        "title": "Preview Window Test",
        "slide_size": { "w": 1280, "h": 720 },
        "variables": [
            { "name": "score", "value": 42 },
            { "name": "answered", "value": false }
        ],
        "scenes": [
            {
                "id": "sc-1",
                "title": "Main",
                "slides": [
                    {
                        "id": "s-1",
                        "title": "Slide 1",
                        "duration_ms": 5000,
                        "objects": [
                            {
                                "id": "txt-1",
                                "kind": { "type": "text", "text": "Score: %score%" },
                                "frame": { "x": 80, "y": 80, "w": 400, "h": 60 }
                            }
                        ],
                        "triggers": []
                    }
                ]
            }
        ]
    }"#;
    Course::from_json(json).expect("valid course")
}

fn open_preview(harness: &mut TestHarness) -> Rect {
    harness.editor.open_preview();
    for _ in 0..3 {
        harness.idle_frame();
    }
    harness
        .editor
        .preview
        .as_ref()
        .and_then(|p| p.window_rect)
        .expect("preview window_rect")
}

fn current_rect(harness: &TestHarness) -> Rect {
    harness
        .editor
        .preview
        .as_ref()
        .and_then(|p| p.window_rect)
        .expect("preview window_rect")
}

fn find_rect_with_fill(shape: &egui::epaint::Shape, fill: Color32) -> Option<Rect> {
    match shape {
        egui::epaint::Shape::Rect(r)
            if r.fill == fill && r.rect.width() >= 1279.0 && r.rect.height() >= 799.0 =>
        {
            Some(r.rect)
        }
        egui::epaint::Shape::Vec(v) => v.iter().find_map(|s| find_rect_with_fill(s, fill)),
        _ => None,
    }
}

#[test]
fn preview_backdrop_dims_to_80_percent_black_in_dark_and_light_themes() {
    for pref in [
        coursecraft_render::ThemePref::Dark,
        coursecraft_render::ThemePref::Light,
    ] {
        let mut harness = TestHarness::new_with_size(test_course(), (1280.0, 800.0));
        harness.editor.set_theme_pref(pref);
        open_preview(&mut harness);
        let shapes = harness.frame_shapes(Vec::new());
        let expected_alpha = PREVIEW_BACKDROP_COLOR.a();
        assert!(
            (200..=210).contains(&expected_alpha),
            "backdrop alpha should be ~80% (204), got {expected_alpha}"
        );
        let backdrop_rect = shapes.iter().find_map(|clipped| {
            find_rect_with_fill(&clipped.shape, Color32::from_black_alpha(expected_alpha))
        });
        assert!(
            backdrop_rect.is_some(),
            "theme {pref:?}: expected full-screen ~80% black backdrop rect"
        );
    }
}

#[test]
fn preview_window_resizes_from_all_edges_and_corners_with_min_max_clamps() {
    let mut harness = TestHarness::new_with_size(test_course(), (1440.0, 900.0));
    let initial = open_preview(&mut harness);

    // Drag each of the 4 edges outward by 40px and verify only that edge moves.
    harness.drag_by(
        pos2(initial.min.x, initial.center().y),
        vec2(-40.0, 0.0),
        4,
        Modifiers::NONE,
    );
    let after_left = current_rect(&harness);
    assert!(
        (after_left.min.x - (initial.min.x - 40.0)).abs() < 2.0,
        "left edge: {after_left:?}"
    );
    assert!((after_left.max.x - initial.max.x).abs() < 0.5);

    harness.drag_by(
        pos2(after_left.max.x, after_left.center().y),
        vec2(40.0, 0.0),
        4,
        Modifiers::NONE,
    );
    let after_right = current_rect(&harness);
    assert!(
        (after_right.max.x - (after_left.max.x + 40.0)).abs() < 2.0,
        "right edge: {after_right:?}"
    );
    assert!((after_right.min.x - after_left.min.x).abs() < 0.5);

    harness.drag_by(
        pos2(after_right.center().x, after_right.min.y),
        vec2(0.0, -30.0),
        4,
        Modifiers::NONE,
    );
    let after_top = current_rect(&harness);
    assert!(
        (after_top.min.y - (after_right.min.y - 30.0)).abs() < 2.0,
        "top edge: {after_top:?}"
    );
    assert!((after_top.max.y - after_right.max.y).abs() < 0.5);

    harness.drag_by(
        pos2(after_top.center().x, after_top.max.y),
        vec2(0.0, 30.0),
        4,
        Modifiers::NONE,
    );
    let after_bottom = current_rect(&harness);
    assert!(
        (after_bottom.max.y - (after_top.max.y + 30.0)).abs() < 2.0,
        "bottom edge: {after_bottom:?}"
    );
    assert!((after_bottom.min.y - after_top.min.y).abs() < 0.5);

    // Drag each of the 4 corners inward by 20px and verify both adjacent edges move.
    harness.drag_by(
        pos2(after_bottom.min.x, after_bottom.min.y),
        vec2(20.0, 20.0),
        4,
        Modifiers::NONE,
    );
    let after_tl = current_rect(&harness);
    assert!((after_tl.min.x - (after_bottom.min.x + 20.0)).abs() < 2.0);
    assert!((after_tl.min.y - (after_bottom.min.y + 20.0)).abs() < 2.0);

    harness.drag_by(
        pos2(after_tl.max.x, after_tl.min.y),
        vec2(-20.0, 20.0),
        4,
        Modifiers::NONE,
    );
    let after_tr = current_rect(&harness);
    assert!((after_tr.max.x - (after_tl.max.x - 20.0)).abs() < 2.0);
    assert!((after_tr.min.y - (after_tl.min.y + 20.0)).abs() < 2.0);

    harness.drag_by(
        pos2(after_tr.min.x, after_tr.max.y),
        vec2(20.0, -20.0),
        4,
        Modifiers::NONE,
    );
    let after_bl = current_rect(&harness);
    assert!((after_bl.min.x - (after_tr.min.x + 20.0)).abs() < 2.0);
    assert!((after_bl.max.y - (after_tr.max.y - 20.0)).abs() < 2.0);

    harness.drag_by(
        pos2(after_bl.max.x, after_bl.max.y),
        vec2(-20.0, -20.0),
        4,
        Modifiers::NONE,
    );
    let after_br = current_rect(&harness);
    assert!((after_br.max.x - (after_bl.max.x - 20.0)).abs() < 2.0);
    assert!((after_br.max.y - (after_bl.max.y - 20.0)).abs() < 2.0);

    // Drag far outward to test max bounds clamp (screen shrunk by PREVIEW_MARGIN).
    harness.drag_by(
        pos2(after_br.min.x, after_br.min.y),
        vec2(-500.0, -500.0),
        5,
        Modifiers::NONE,
    );
    let r1 = current_rect(&harness);
    harness.drag_by(
        pos2(r1.max.x, r1.max.y),
        vec2(500.0, 500.0),
        5,
        Modifiers::NONE,
    );
    let maxed = current_rect(&harness);
    assert!(
        (maxed.min.x - PREVIEW_MARGIN).abs() < 1.0,
        "maxed={maxed:?}"
    );
    assert!(
        (maxed.min.y - PREVIEW_MARGIN).abs() < 1.0,
        "maxed={maxed:?}"
    );
    assert!(
        (maxed.max.x - (1440.0 - PREVIEW_MARGIN)).abs() < 1.0,
        "maxed={maxed:?}"
    );
    assert!(
        (maxed.max.y - (900.0 - PREVIEW_MARGIN)).abs() < 1.0,
        "maxed={maxed:?}"
    );

    // Drag far inward to test min size clamp.
    harness.drag_by(
        pos2(maxed.max.x, maxed.max.y),
        vec2(-1200.0, -800.0),
        6,
        Modifiers::NONE,
    );
    let minned = current_rect(&harness);
    assert!(
        (minned.width() - PREVIEW_MIN_WIDTH).abs() < 1.0,
        "minned={minned:?}"
    );
    assert!(
        (minned.height() - PREVIEW_MIN_HEIGHT).abs() < 1.0,
        "minned={minned:?}"
    );

    // Resize to a custom size, close preview, reopen, and verify size is remembered.
    harness.drag_by(
        pos2(minned.max.x, minned.max.y),
        vec2(220.0, 140.0),
        4,
        Modifiers::NONE,
    );
    let custom = current_rect(&harness);
    harness.press(Key::Escape, Modifiers::NONE);
    harness.idle_frame();
    assert!(harness.editor.preview.is_none());

    let reopened = open_preview(&mut harness);
    assert!(
        (reopened.width() - custom.width()).abs() < 1.0
            && (reopened.height() - custom.height()).abs() < 1.0,
        "remembered size mismatch: custom={:?} reopened={:?}",
        custom.size(),
        reopened.size()
    );
}

#[test]
fn preview_fullscreen_toggle_and_esc_order() {
    let mut harness = TestHarness::new_with_size(test_course(), (1280.0, 800.0));
    let initial = open_preview(&mut harness);

    // Toggle full screen via F11 while preview has focus.
    harness.press(Key::F11, Modifiers::NONE);
    harness.idle_frame();
    assert!(
        harness
            .editor
            .preview
            .as_ref()
            .is_some_and(|p| p.fullscreen)
    );
    let fs_rect = current_rect(&harness);
    assert!((fs_rect.width() - 1280.0).abs() < 1.0 && (fs_rect.height() - 800.0).abs() < 1.0);

    // First Esc leaves full screen, does not close preview.
    harness.press(Key::Escape, Modifiers::NONE);
    harness.idle_frame();
    assert!(harness.editor.preview.is_some());
    assert!(
        !harness
            .editor
            .preview
            .as_ref()
            .is_some_and(|p| p.fullscreen)
    );
    let restored = current_rect(&harness);
    assert!((restored.width() - initial.width()).abs() < 1.0);

    // Toggle full screen via Cmd+Ctrl+F.
    harness.press(Key::F, Modifiers::MAC_CMD | Modifiers::CTRL);
    harness.idle_frame();
    assert!(
        harness
            .editor
            .preview
            .as_ref()
            .is_some_and(|p| p.fullscreen)
    );

    // Clicking near the edge while full screen does not close preview.
    harness.click_at(pos2(5.0, 5.0));
    harness.idle_frame();
    assert!(harness.editor.preview.is_some());

    // First Esc exits full screen; second Esc closes preview.
    harness.press(Key::Escape, Modifiers::NONE);
    harness.idle_frame();
    assert!(harness.editor.preview.is_some());
    assert!(
        !harness
            .editor
            .preview
            .as_ref()
            .is_some_and(|p| p.fullscreen)
    );

    harness.press(Key::Escape, Modifiers::NONE);
    harness.idle_frame();
    assert!(harness.editor.preview.is_none());

    // Reopen and toggle full screen via the title-bar button.
    open_preview(&mut harness);
    let fs_btn = harness
        .find_icon(coursecraft_render::Icon::CornersOut)
        .expect("Full screen button in preview title bar");
    harness.click_at(fs_btn);
    harness.idle_frame();
    assert!(
        harness
            .editor
            .preview
            .as_ref()
            .is_some_and(|p| p.fullscreen)
    );
    let exit_btn = harness
        .find_icon(coursecraft_render::Icon::CornersIn)
        .expect("Exit full screen button in preview title bar");
    harness.click_at(exit_btn);
    harness.idle_frame();
    assert!(
        !harness
            .editor
            .preview
            .as_ref()
            .is_some_and(|p| p.fullscreen)
    );
}

#[test]
fn preview_slide_scales_and_variables_panel_fits_at_three_sizes() {
    let mut harness = TestHarness::new_with_size(test_course(), (1440.0, 900.0));
    open_preview(&mut harness);
    harness.press(Key::F3, Modifiers::NONE);
    for _ in 0..3 {
        harness.idle_frame();
    }

    // 1. Default size (~960x640)
    let default_rect = current_rect(&harness);
    let default_slide = harness
        .editor
        .preview
        .as_ref()
        .expect("preview")
        .frame
        .slide_rect()
        .expect("default slide_rect");

    // 2. Minimum size (480x320)
    harness.drag_by(
        pos2(default_rect.max.x, default_rect.max.y),
        vec2(-600.0, -400.0),
        6,
        Modifiers::NONE,
    );
    harness.idle_frame();
    let min_rect = current_rect(&harness);
    let min_slide = harness
        .editor
        .preview
        .as_ref()
        .expect("preview")
        .frame
        .slide_rect()
        .expect("min slide_rect");
    let score_pos_min = harness.find_text("score").expect("score in min preview");
    assert!(
        min_rect.contains(score_pos_min),
        "variables panel must stay inside min preview"
    );

    // 3. Full screen (1440x900)
    harness.press(Key::F11, Modifiers::NONE);
    for _ in 0..2 {
        harness.idle_frame();
    }
    let fs_rect = current_rect(&harness);
    let fs_slide = harness
        .editor
        .preview
        .as_ref()
        .expect("preview")
        .frame
        .slide_rect()
        .expect("fs slide_rect");
    let score_pos_fs = harness.find_text("score").expect("score in fs preview");
    assert!(
        fs_rect.contains(score_pos_fs),
        "variables panel must stay inside fs preview"
    );

    // Slide scales monotonically and preserves 16:9 aspect ratio at all 3 sizes.
    assert!(min_slide.width() < default_slide.width());
    assert!(default_slide.width() < fs_slide.width());
    for s in [min_slide, default_slide, fs_slide] {
        let aspect = s.width() / s.height();
        assert!(
            (aspect - 16.0 / 9.0).abs() < 0.02,
            "expected 16:9 aspect, got {aspect}"
        );
    }
}

#[test]
fn preview_player_strip_fullscreen_button_toggles_preview_fullscreen() {
    let mut harness = TestHarness::new_with_size(test_course(), (1280.0, 800.0));
    open_preview(&mut harness);
    let fs_rect = harness
        .editor
        .preview
        .as_ref()
        .expect("preview")
        .frame
        .chrome_rect(&coursecraft_render::ChromeButton::Fullscreen)
        .expect("player strip fullscreen button should be enabled in preview");

    harness.click_at(fs_rect.center());
    harness.idle_frame();
    assert!(
        harness
            .editor
            .preview
            .as_ref()
            .is_some_and(|p| p.fullscreen),
        "clicking player strip fullscreen button must enter fullscreen"
    );

    let fs_rect2 = harness
        .editor
        .preview
        .as_ref()
        .expect("preview")
        .frame
        .chrome_rect(&coursecraft_render::ChromeButton::Fullscreen)
        .expect("player strip fullscreen button");
    harness.click_at(fs_rect2.center());
    harness.idle_frame();
    assert!(
        harness
            .editor
            .preview
            .as_ref()
            .is_some_and(|p| !p.fullscreen),
        "clicking player strip fullscreen button again must exit fullscreen"
    );
}

#[test]
fn preview_escape_hierarchy_closes_speed_popup_drawer_lightbox_before_preview() {
    let mut course = test_course();
    // Add a second slide and a button on slide 1 that opens slide 2 as a lightbox.
    let mut slide2 = course.scenes[0].slides[0].clone();
    slide2.id = "s2".into();
    slide2.title = "Lightbox Slide".into();
    course.scenes[0].slides.push(slide2);
    course.scenes[0].slides[0]
        .triggers
        .push(coursecraft_course::Trigger {
            when: coursecraft_course::Event::Click {
                object: "txt-1".into(),
            },
            action: coursecraft_course::Action::LightboxSlide { slide: "s2".into() },
            condition: None,
            disabled: false,
        });

    let mut harness = TestHarness::new_with_size(course, (1280.0, 800.0));
    harness.editor.open_preview_at(None);
    for _ in 0..3 {
        harness.idle_frame();
    }

    // 1. Speed popup open -> Esc closes speed popup, preview stays open.
    let speed_rect = harness
        .editor
        .preview
        .as_ref()
        .expect("preview")
        .frame
        .chrome_rect(&coursecraft_render::ChromeButton::Speed)
        .expect("speed button");
    harness.click_at(speed_rect.center());
    harness.idle_frame();
    assert!(harness.find_text("2×").is_some(), "speed popup should open");
    harness.press(Key::Escape, Modifiers::NONE);
    harness.idle_frame();
    assert!(
        harness.editor.preview.is_some(),
        "Esc must close speed popup without closing preview"
    );
    assert!(
        harness.find_text("2×").is_none(),
        "speed popup should close"
    );

    // 2. Drawer open -> Esc closes drawer, preview stays open.
    let outline_rect = harness
        .editor
        .preview
        .as_ref()
        .expect("preview")
        .frame
        .chrome_rect(&coursecraft_render::ChromeButton::Outline)
        .expect("outline button");
    harness.click_at(outline_rect.center());
    harness.idle_frame();
    assert!(
        harness
            .editor
            .preview
            .as_ref()
            .is_some_and(|p| p.frame.drawer_open())
    );
    harness.press(Key::Escape, Modifiers::NONE);
    harness.idle_frame();
    assert!(
        harness.editor.preview.is_some(),
        "Esc must close drawer without closing preview"
    );
    assert!(
        harness
            .editor
            .preview
            .as_ref()
            .is_some_and(|p| !p.frame.drawer_open())
    );
    // Let the drawer close animation finish so the scrim no longer covers the slide.
    for _ in 0..20 {
        harness.idle_frame();
    }

    // 3. Lightbox open -> Esc closes lightbox, preview stays open.
    let s_rect = harness
        .editor
        .preview
        .as_ref()
        .and_then(|p| p.frame.slide_rect())
        .expect("slide_rect");
    let txt_pos = s_rect.min
        + egui::vec2(
            s_rect.width() * (280.0 / 1280.0),
            s_rect.height() * (110.0 / 720.0),
        );
    harness.click_at(txt_pos);
    harness.idle_frame();
    assert!(
        harness.editor.preview.as_ref().is_some_and(|p| p
            .frame
            .chrome_rect(&coursecraft_render::ChromeButton::CloseLightbox)
            .is_some()),
        "lightbox should be open"
    );
    harness.press(Key::Escape, Modifiers::NONE);
    harness.idle_frame();
    assert!(
        harness.editor.preview.is_some(),
        "Esc must close lightbox without closing preview"
    );
    assert!(
        harness.editor.preview.as_ref().is_some_and(|p| p
            .frame
            .chrome_rect(&coursecraft_render::ChromeButton::CloseLightbox)
            .is_none()),
        "lightbox should close on Esc"
    );

    // 4. Clean slide -> Esc closes preview.
    harness.press(Key::Escape, Modifiers::NONE);
    harness.idle_frame();
    assert!(
        harness.editor.preview.is_none(),
        "Esc on clean slide closes preview"
    );
}
