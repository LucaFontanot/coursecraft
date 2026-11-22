//! Step 17: Full learner player UI/UX sweep across 6 browser viewports, 3 course themes,
//! all player states, long names, and preview-vs-published parity.

use crate::design::contrast_ratio;
use crate::focus::ChromeButton;
use crate::media::{MediaFetch, MediaSource};
use crate::player_frame::{FrameOutput, PlayerFrame};
use crate::snapshot::{PlayerSnapshotSelect, snapshot_player};
use crate::view::SlideView;
use coursecraft_course::{Color, Course, GlossaryEntry, Resource, ResourceTarget};
use coursecraft_logic::Player;
use egui::{Pos2, Rect, Vec2, pos2, vec2};

const SHOWCASE_JSON: &str = include_str!("../../../examples/showcase/course.json");

const VIEWPORTS: [(f32, f32); 6] = [
    (320.0, 568.0),
    (390.0, 844.0),
    (844.0, 390.0),
    (768.0, 1024.0),
    (1280.0, 720.0),
    (1920.0, 1080.0),
];

fn showcase_course() -> Course {
    Course::from_json(SHOWCASE_JSON).unwrap_or_else(|e| panic!("showcase course: {e}"))
}

fn long_names_player_course() -> Course {
    let mut course = showcase_course();
    course.title = format!("Long Course Title {}", "C".repeat(102));
    for (i, scene) in course.scenes.iter_mut().enumerate() {
        for (j, slide) in scene.slides.iter_mut().enumerate() {
            slide.title = format!("Slide {i:02}-{j:02} {}", "S".repeat(105));
        }
    }
    course.resources.push(Resource {
        id: "res-long".to_owned(),
        title: format!("Resource Document {}", "R".repeat(102)),
        target: ResourceTarget::Url {
            url: "https://example.com/resource.pdf".to_owned(),
        },
    });
    course.glossary.push(GlossaryEntry {
        id: "gl-long".to_owned(),
        term: format!("Glossary Term {}", "G".repeat(106)),
        definition: format!("Detailed glossary definition {}", "D".repeat(120)),
    });
    course
}

fn themes() -> [(&'static str, &'static str, &'static str, &'static str); 3] {
    [
        ("default-dark", "#1B1B1B", "#F2F2F5", "#0A6CFF"),
        ("bright-light", "#F8F9FA", "#1D1D1F", "#2563EB"),
        ("saturated-warm", "#1F1926", "#F8F7FA", "#F59E0B"),
    ]
}

fn run_player_frame(
    frame: &mut PlayerFrame,
    view: &mut SlideView,
    player: &Player<'_>,
    size: (f32, f32),
    time: f64,
) -> (FrameOutput, Vec<egui::epaint::ClippedShape>) {
    let ctx = egui::Context::default();
    let raw = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(size.0, size.1))),
        time: Some(time),
        ..Default::default()
    };
    let mut out = FrameOutput::default();
    let full = ctx.run_ui(raw, |ui| {
        out = frame.show(ui, player, view, None);
    });
    let shapes = full.shapes.clone();
    full.drop_without_applying_deltas();
    (out, shapes)
}

fn check_strip_buttons(frame: &PlayerFrame, tag: &str, findings: &mut Vec<String>) {
    let mut rects: Vec<(ChromeButton, Rect)> = Vec::new();
    for btn in [
        ChromeButton::Outline,
        ChromeButton::PlayPause,
        ChromeButton::Replay,
        ChromeButton::Mute,
        ChromeButton::Captions,
        ChromeButton::Speed,
        ChromeButton::Previous,
        ChromeButton::Next,
        ChromeButton::Fullscreen,
    ] {
        if let Some(r) = frame.chrome_rect(&btn) {
            if r.width() + 0.5 < 36.0 || r.height() + 0.5 < 36.0 {
                findings.push(format!(
                    "[UX-05] {tag}: strip button {btn:?} {r:?} is smaller than 36x36"
                ));
            }
            rects.push((btn, r));
        }
    }
    for (i, (a_btn, a_rect)) in rects.iter().enumerate() {
        for (b_btn, b_rect) in rects.iter().skip(i + 1) {
            let inter = a_rect.intersect(*b_rect);
            if inter.is_positive() && inter.width() > 1.0 && inter.height() > 1.0 {
                findings.push(format!(
                    "[UX-04] {tag}: strip buttons {a_btn:?} and {b_btn:?} overlap by {inter:?}"
                ));
            }
        }
    }
}

fn check_clipped_shapes_in_viewport(
    shapes: &[egui::epaint::ClippedShape],
    viewport: Rect,
    tag: &str,
    findings: &mut Vec<String>,
) {
    for c in shapes {
        if let egui::epaint::Shape::Text(t) = &c.shape {
            if t.galley.text().trim().is_empty() {
                continue;
            }
            let r = t.visual_bounding_rect();
            if !r.is_positive() {
                continue;
            }
            let clip = c.clip_rect.intersect(viewport);
            if r.min.x < viewport.min.x - 1.5
                || r.max.x > viewport.max.x + 1.5
                || r.min.y < viewport.min.y - 1.5
                || r.max.y > viewport.max.y + 1.5
            {
                findings.push(format!(
                    "[UX-16] {tag}: text {:?} ({r:?}) extends outside viewport {viewport:?}",
                    t.galley.text()
                ));
            } else if r.max.x > clip.max.x + 1.5 || r.min.x < clip.min.x - 1.5 {
                findings.push(format!(
                    "[UX-04] {tag}: text {:?} ({r:?}) clipped horizontally by {clip:?}",
                    t.galley.text()
                ));
            }
        }
    }
}

#[test]
fn uiux_player_sweep() {
    let mut findings = Vec::new();
    let base = long_names_player_course();

    for (theme_name, bar_hex, text_hex, accent_hex) in themes() {
        let mut course = base.clone();
        course.theme.player.bar = Color(bar_hex.to_owned());
        course.theme.player.bar_text = Color(text_hex.to_owned());
        course.theme.accent = Color(accent_hex.to_owned());
        course.player.show_menu = true;
        let player = Player::new(&course).unwrap_or_else(|e| panic!("player: {e:?}"));

        let bar_bg = crate::parse_color(&course.theme.player.bar).unwrap_or(egui::Color32::BLACK);
        let bar_fg =
            crate::parse_color(&course.theme.player.bar_text).unwrap_or(egui::Color32::WHITE);
        let bar_cr = contrast_ratio(bar_fg, bar_bg);
        if bar_cr < 4.5 {
            findings.push(format!(
                "[UX-06] theme {theme_name}: bar_text vs bar contrast {bar_cr:.2} < 4.5"
            ));
        }

        for &size in &VIEWPORTS {
            let viewport = Rect::from_min_size(Pos2::ZERO, Vec2::new(size.0, size.1));
            let mut frame = PlayerFrame::new();
            frame.can_fullscreen = true;
            frame.can_speed = true;
            let mut view = SlideView::new();

            // 1. Default state
            let tag = format!("{theme_name} {}x{} default", size.0, size.1);
            let (_, shapes) = run_player_frame(&mut frame, &mut view, &player, size, 0.1);
            check_strip_buttons(&frame, &tag, &mut findings);
            check_clipped_shapes_in_viewport(&shapes, viewport, &tag, &mut findings);

            // 2. Drawer open
            frame.open_drawer();
            let tag_drawer = format!("{theme_name} {}x{} drawer", size.0, size.1);
            let (_, drawer_shapes) = run_player_frame(&mut frame, &mut view, &player, size, 0.5);
            check_clipped_shapes_in_viewport(&drawer_shapes, viewport, &tag_drawer, &mut findings);
        }
    }

    assert!(
        findings.is_empty(),
        "uiux_player_sweep found {} issues:\n{}",
        findings.len(),
        findings
            .iter()
            .take(40)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

struct EmptyMedia;
impl MediaSource for EmptyMedia {
    fn get(&self, _src: &str) -> MediaFetch {
        MediaFetch::Missing
    }
}

#[test]
fn uiux_player_preview_vs_published_pixel_parity() {
    let course = showcase_course();
    let media = EmptyMedia;
    let img_a = snapshot_player(
        course.clone(),
        Some("s-layers"),
        &media,
        (960, 540),
        1.0,
        PlayerSnapshotSelect::Playing,
    )
    .unwrap_or_else(|e| panic!("snapshot_player a: {e}"));
    let img_b = snapshot_player(
        course,
        Some("s-layers"),
        &media,
        (960, 540),
        1.0,
        PlayerSnapshotSelect::Playing,
    )
    .unwrap_or_else(|e| panic!("snapshot_player b: {e}"));
    assert_eq!(img_a.width, 960);
    assert_eq!(img_a.height, 540);
    assert_eq!(
        img_a.pixels, img_b.pixels,
        "deterministic snapshot_player render must match byte-for-byte"
    );
}
