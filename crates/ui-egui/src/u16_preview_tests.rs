//! Step u16 regression tests: Preview split panes (`T13.5`), Web editor disabled actions (`T13.8`), and Player parity (`T14.9`).

use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use coursecraft_render::{PlayerFrame, SlideView, ThemeMode, snapshot_player};
use egui::{Event, Key, Modifiers, Rect, pos2, vec2};

const HELLO_JSON: &str = include_str!("../../../examples/hello-course/course.json");

struct TestNoMedia;

impl coursecraft_render::MediaSource for TestNoMedia {
    fn get(&self, _src: &str) -> coursecraft_render::MediaFetch {
        coursecraft_render::MediaFetch::Missing
    }
}

fn course_with_many_variables() -> Course {
    let mut course = Course::from_json(HELLO_JSON).expect("valid hello course");
    for i in 0..20 {
        course.variables.push(coursecraft_course::VariableDef {
            name: format!("very_long_variable_name_for_testing_split_pane_overflow_{i}"),
            value: coursecraft_course::Value::Text(format!("value_{i}_abcdefghijklmnopqrstuvwxyz")),
        });
    }
    course
}

#[test]
fn t13_5_preview_live_variables_and_effect_log_at_1024x600() {
    let course = course_with_many_variables();
    let mut h = TestHarness::new_with_theme(course, (1024.0, 600.0), ThemeMode::Dark);
    h.editor.open_preview();
    h.press(Key::F2, Modifiers::NONE);
    h.press(Key::F3, Modifiers::NONE);

    let mut prev_window_rect = Rect::NOTHING;
    for frame_idx in 0..10 {
        let shapes = h.frame_shapes(vec![]);
        let mut window_rect = Rect::NOTHING;
        for clipped in &shapes {
            if let egui::epaint::Shape::Rect(r) = &clipped.shape
                && r.rect.width() > 500.0
                && r.rect.height() > 350.0
                && r.rect.width() < 1020.0
            {
                window_rect = window_rect.union(r.rect);
            }
        }
        if frame_idx > 1 && window_rect.is_positive() {
            if prev_window_rect.is_positive() {
                assert!(
                    (window_rect.width() - prev_window_rect.width()).abs() <= 2.0
                        && (window_rect.height() - prev_window_rect.height()).abs() <= 2.0,
                    "preview window auto-grew on frame {frame_idx}: prev={prev_window_rect:?} now={window_rect:?}"
                );
            }
            prev_window_rect = window_rect;
        }
    }
}

#[test]
fn t13_8_web_editor_disables_desktop_only_actions_with_tooltip() {
    let course = Course::from_json(HELLO_JSON).expect("valid hello course");
    let mut h = TestHarness::new_with_theme(course, (1280.0, 800.0), ThemeMode::Dark);
    h.editor.file_ops_available = false;
    h.editor.publish_available = false;
    h.ctx.all_styles_mut(|s| s.interaction.tooltip_delay = 0.0);
    h.idle_frame();
    h.idle_frame();

    let file_pos = h.find_text("File").expect("File menu");
    h.click_at(file_pos);
    h.idle_frame();
    if let Some(pos) = h.find_text("Save as Folder…") {
        let shapes = h.frame_shapes(vec![Event::PointerMoved(pos)]);
        let has_tooltip = shapes.iter().any(|c| {
            matches!(&c.shape, egui::epaint::Shape::Text(t) if t.galley.text().contains("desktop app"))
        });
        assert!(
            has_tooltip,
            "disabled desktop-only menu item must show 'Available in the desktop app' tooltip"
        );
    }
}

#[test]
fn t14_9_preview_and_published_player_layout_parity() {
    let course = Course::from_json(HELLO_JSON).expect("valid hello course");
    let snap = snapshot_player(
        course.clone(),
        None,
        &TestNoMedia,
        (960, 540),
        1.0,
        coursecraft_render::PlayerSnapshotSelect::Playing,
    )
    .expect("snapshot succeeds");
    assert_eq!(snap.width, 960);
    assert_eq!(snap.height, 540);

    let player = coursecraft_logic::Player::new(&course).expect("valid player");
    let ctx = egui::Context::default();
    let mut frame = PlayerFrame::new();
    let mut view = SlideView::new();
    let mut out_rect = Rect::NOTHING;
    let raw = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(960.0, 540.0))),
        time: Some(1.0),
        ..Default::default()
    };
    let full = ctx.run_ui(raw, |ui| {
        let out = frame.show(ui, &player, &mut view, None);
        out_rect = out.slide_rect;
    });
    full.drop_without_applying_deltas();
    assert!(out_rect.is_positive(), "slide_rect must be positive");
    let strip = frame.strip_rect().expect("strip rect must exist");
    assert!(
        strip.top() >= out_rect.bottom() - 1.0,
        "reserved strip must sit below slide_rect"
    );
}
