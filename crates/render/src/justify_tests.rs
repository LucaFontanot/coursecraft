//! Tests for justified text layout and rendering.

use coursecraft_course::{Align, Autofit, Baseline, Course, Paragraph, VAlign};
use egui::{Color32, Rect, pos2, vec2};

use crate::compose::ResolvedTextRun;
use crate::media::{MediaFetch, MediaSource};
use crate::offscreen::{RenderMode, RenderOptions, render_slide_rgba};
use crate::text_layout::{layout_text, layout_text_with_wrap};

struct EmptyMedia;

impl MediaSource for EmptyMedia {
    fn get(&self, _src: &str) -> MediaFetch {
        MediaFetch::Missing
    }
}

fn text_ctx() -> egui::Context {
    let ctx = egui::Context::default();
    let output = ctx.run_ui(egui::RawInput::default(), |_ui| {});
    output.drop_without_applying_deltas();
    ctx
}

fn plain_run(text: &str, size: f32) -> ResolvedTextRun {
    ResolvedTextRun {
        text: text.to_owned(),
        size,
        color: Color32::BLACK,
        bold: false,
        italic: false,
        underline: false,
        link: None,
        font: None,
        strikethrough: false,
        baseline: Baseline::Normal,
        letter_spacing: 0.0,
    }
}

fn row_content_right(row: &egui::epaint::text::PlacedRow) -> f32 {
    let Some(last) = row.glyphs.iter().rposition(|g| !g.chr.is_whitespace()) else {
        return 0.0;
    };
    row.glyphs[..=last]
        .iter()
        .map(|g| g.pos.x + g.advance_width)
        .fold(0.0f32, f32::max)
}

fn row_mesh_right(row: &egui::epaint::text::PlacedRow) -> f32 {
    row.visuals
        .mesh
        .vertices
        .iter()
        .map(|v| v.pos.x)
        .fold(0.0f32, f32::max)
}

#[test]
fn justified_wrapped_lines_reach_box_width_in_mesh_and_pixels() {
    let ctx = text_ctx();
    let box_w = 240.0f32;
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(box_w, 300.0));
    let runs = vec![plain_run(
        "Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda",
        18.0,
    )];
    let laid = layout_text(
        &ctx,
        rect,
        1.0,
        &runs,
        &[Paragraph::default()],
        Align::Justify,
        VAlign::Top,
        Autofit::None,
    );
    let para = laid.paragraphs.first().expect("paragraph");
    assert!(
        para.galley.rows.len() >= 2,
        "expected wrapped lines, got {}",
        para.galley.rows.len()
    );
    for (idx, row) in para.galley.rows.iter().enumerate() {
        let is_last = idx + 1 == para.galley.rows.len();
        let content_r = row_content_right(row);
        let mesh_r = row_mesh_right(row);
        if is_last || row.ends_with_newline {
            assert!(
                content_r < box_w - 5.0,
                "last line should not be stretched (row {idx}: content_r={content_r}, box_w={box_w})"
            );
        } else {
            assert!(
                (content_r - box_w).abs() <= 0.5,
                "justified row {idx} content_right={content_r} should equal box_w={box_w} within 0.5 px"
            );
            assert!(
                (mesh_r - box_w).abs() <= 1.5,
                "justified row {idx} mesh_right={mesh_r} should reach box_w={box_w}"
            );
        }
    }

    // Verify via the real slide pixel renderer (render_slide_rgba):
    // A white slide with a 300px wide justified black-text box at x=100..400 (with 10px L/R insets -> 280px inner).
    let json = serde_json::json!({
        "schema_version": 1,
        "id": "justify-test",
        "title": "Justify Test",
        "slide_size": { "w": 600, "h": 200 },
        "scenes": [{
            "id": "sc1",
            "title": "Scene 1",
            "slides": [{
                "id": "s1",
                "title": "S1",
                "background": "#FFFFFF",
                "objects": [{
                    "id": "t1",
                    "frame": { "x": 100, "y": 20, "w": 300, "h": 160 },
                    "kind": {
                        "type": "text",
                        "text": "Alpha beta gamma delta epsilon zeta eta theta iota kappa",
                        "style": {
                            "align": "justify",
                            "valign": "top",
                            "size": 20.0,
                            "color": "#000000",
                            "wrap": true,
                            "insets": { "left": 10.0, "top": 0.0, "right": 10.0, "bottom": 0.0 }
                        }
                    }
                }]
            }]
        }]
    });
    let course = Course::from_json(&json.to_string()).expect("valid course");
    let opts = RenderOptions {
        width: 600,
        mode: RenderMode::Authoring,
        layers: Vec::new(),
        at_ms: None,
    };
    let img = render_slide_rgba(&course, "s1", &opts, &EmptyMedia).expect("render slide");

    // Check the first line's y-band (y in 20..48) near the right inner edge (x in 375..390).
    let mut dark_near_right = 0usize;
    for y in 20..48 {
        for x in 375..390 {
            let idx = ((y * img.width + x) * 4) as usize;
            if img.pixels[idx] < 128 {
                dark_near_right += 1;
            }
        }
    }
    assert!(
        dark_near_right > 0,
        "expected justified text pixels near right inner edge x=375..390, found 0"
    );
}

#[test]
fn justified_mixed_runs_fonts_and_sizes() {
    let ctx = text_ctx();
    let box_w = 260.0f32;
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(box_w, 300.0));
    let mut r1 = plain_run("Short ", 14.0);
    r1.bold = true;
    let mut r2 = plain_run("medium ", 22.0);
    r2.italic = true;
    r2.color = Color32::RED;
    let mut r3 = plain_run("words across multiple ", 16.0);
    r3.strikethrough = true;
    let r4 = plain_run("runs that wrap cleanly onto two lines", 18.0);

    let laid = layout_text(
        &ctx,
        rect,
        1.0,
        &[r1, r2, r3, r4],
        &[Paragraph::default()],
        Align::Justify,
        VAlign::Top,
        Autofit::None,
    );
    let para = laid.paragraphs.first().expect("paragraph");
    assert!(para.galley.rows.len() >= 2);
    let first = &para.galley.rows[0];
    let content_r = row_content_right(first);
    assert!(
        (content_r - box_w).abs() <= 0.5,
        "mixed-run justified line content_right={content_r} != box_w={box_w}"
    );
    let mesh_r = row_mesh_right(first);
    assert!(
        (mesh_r - box_w).abs() <= 6.0,
        "mixed-run justified line mesh_right={mesh_r} != box_w={box_w}"
    );
}

#[test]
fn justified_single_word_line_stays_left() {
    let ctx = text_ctx();
    let box_w = 180.0f32;
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(box_w, 300.0));
    let runs = vec![plain_run(
        "Supercalifragilisticexpialidocious nextword anotherword",
        16.0,
    )];
    let laid = layout_text(
        &ctx,
        rect,
        1.0,
        &runs,
        &[Paragraph::default()],
        Align::Justify,
        VAlign::Top,
        Autofit::None,
    );
    let para = laid.paragraphs.first().expect("paragraph");
    assert!(para.galley.rows.len() >= 2);
    let first = &para.galley.rows[0];
    assert!(
        first.glyphs[0].pos.x.abs() <= 0.5,
        "single-word line should stay left-aligned, got x={}",
        first.glyphs[0].pos.x
    );
}

#[test]
fn justified_very_narrow_box_and_nowrap_and_rtl() {
    let ctx = text_ctx();

    // Very narrow box: should not panic or produce NaN.
    let narrow = Rect::from_min_size(pos2(0.0, 0.0), vec2(12.0, 200.0));
    let laid_narrow = layout_text(
        &ctx,
        narrow,
        1.0,
        &[plain_run("Hello world test", 16.0)],
        &[Paragraph::default()],
        Align::Justify,
        VAlign::Top,
        Autofit::None,
    );
    assert!(laid_narrow.width.is_finite() && laid_narrow.height.is_finite());

    // Wrap text off: should not justify (stays single line).
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(120.0, 200.0));
    let laid_nowrap = layout_text_with_wrap(
        &ctx,
        rect,
        1.0,
        &[plain_run("One two three four five six seven", 16.0)],
        &[Paragraph::default()],
        Align::Justify,
        VAlign::Top,
        Autofit::None,
        false,
    );
    assert_eq!(laid_nowrap.paragraphs[0].galley.rows.len(), 1);

    // RTL text: should justify without panicking or losing glyphs.
    let rtl_rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(200.0, 200.0));
    let laid_rtl = layout_text(
        &ctx,
        rtl_rect,
        1.0,
        &[plain_run(
            "مرحبا بكم في هذا النص العربي الطويل الذي يلتف على عدة أسطر",
            18.0,
        )],
        &[Paragraph::default()],
        Align::Justify,
        VAlign::Top,
        Autofit::None,
    );
    assert!(laid_rtl.width.is_finite() && laid_rtl.height.is_finite());
}

#[test]
fn justified_rotated_box_renders_without_error() {
    let json = serde_json::json!({
        "schema_version": 1,
        "id": "justify-rot",
        "title": "Justify Rot",
        "slide_size": { "w": 400, "h": 300 },
        "scenes": [{
            "id": "sc1",
            "title": "Scene 1",
            "slides": [{
                "id": "s1",
                "title": "S1",
                "background": "#FFFFFF",
                "objects": [{
                    "id": "t1",
                    "frame": { "x": 80, "y": 60, "w": 240, "h": 140 },
                    "rotation": 20.0,
                    "kind": {
                        "type": "text",
                        "text": "Alpha beta gamma delta epsilon zeta eta theta iota",
                        "style": {
                            "align": "justify",
                            "valign": "top",
                            "size": 18.0,
                            "color": "#000000",
                            "wrap": true
                        }
                    }
                }]
            }]
        }]
    });
    let course = Course::from_json(&json.to_string()).expect("valid course");
    let opts = RenderOptions {
        width: 400,
        mode: RenderMode::Authoring,
        layers: Vec::new(),
        at_ms: None,
    };
    let img = render_slide_rgba(&course, "s1", &opts, &EmptyMedia).expect("render rotated");
    let has_dark = img.pixels.as_chunks::<4>().0.iter().any(|px| px[0] < 128);
    assert!(
        has_dark,
        "rotated justified text should render visible pixels"
    );
}

#[test]
fn justified_words_snap_to_physical_pixels_at_100_and_150_zoom() {
    let text = "The quick brown fox jumps over the lazy dog and five dozen liquor jugs";
    for &ppp in &[1.0f32, 1.5f32] {
        let ctx = egui::Context::default();
        ctx.set_pixels_per_point(ppp);
        let output = ctx.run_ui(egui::RawInput::default(), |_ui| {});
        output.drop_without_applying_deltas();

        for &zoom in &[1.0f32, 1.5f32] {
            for &origin_x in &[0.0f32, 100.37f32] {
                let box_w = 240.0 * zoom;
                let rect = Rect::from_min_size(pos2(origin_x, 20.0), vec2(box_w, 300.0 * zoom));
                let runs = vec![plain_run(text, 16.0)];
                let laid = layout_text(
                    &ctx,
                    rect,
                    zoom,
                    &runs,
                    &[Paragraph::default()],
                    Align::Justify,
                    VAlign::Top,
                    Autofit::None,
                );
                let para = laid.paragraphs.first().expect("paragraph");
                assert!(
                    para.galley.rows.len() >= 2,
                    "expected wrapped lines at zoom={zoom}, ppp={ppp}"
                );
                for (row_idx, row) in para.galley.rows.iter().enumerate() {
                    let is_last = row_idx + 1 == para.galley.rows.len();
                    if is_last || row.ends_with_newline {
                        continue;
                    }
                    let mut in_word = false;
                    let mut word_count = 0usize;
                    for glyph in &row.glyphs {
                        if glyph.chr.is_whitespace() {
                            in_word = false;
                        } else if !in_word {
                            in_word = true;
                            word_count += 1;
                            let screen_x = para.pos.x + row.pos.x + glyph.pos.x;
                            let px = screen_x * ppp;
                            let frac = (px - px.round()).abs();
                            assert!(
                                frac <= 0.01,
                                "ppp={ppp} zoom={zoom} origin_x={origin_x} row={row_idx} word={word_count}: \
                                 screen_x={screen_x} * ppp={px} is not integer (frac={frac})"
                            );
                        }
                    }
                    assert!(word_count >= 2);
                    let content_r = para.pos.x + row.pos.x + row_content_right(row);
                    assert!(
                        (content_r - rect.right()).abs() <= 1.0,
                        "ppp={ppp} zoom={zoom} origin_x={origin_x} row={row_idx}: \
                         content_r={content_r} != rect.right()={}",
                        rect.right()
                    );
                }
            }
        }
    }
}

#[test]
fn left_center_right_text_is_unchanged() {
    let ctx = text_ctx();
    let text = "The quick brown fox jumps over the lazy dog and five dozen liquor jugs";
    let runs = vec![plain_run(text, 16.0)];
    let rect_zero = Rect::from_min_size(pos2(0.0, 0.0), vec2(240.0, 300.0));
    let rect_frac = Rect::from_min_size(pos2(100.37, 20.0), vec2(240.0, 300.0));

    for align in [Align::Left, Align::Center, Align::Right] {
        let laid_zero = layout_text(
            &ctx,
            rect_zero,
            1.0,
            &runs,
            &[Paragraph::default()],
            align,
            VAlign::Top,
            Autofit::None,
        );
        let laid_frac = layout_text(
            &ctx,
            rect_frac,
            1.0,
            &runs,
            &[Paragraph::default()],
            align,
            VAlign::Top,
            Autofit::None,
        );
        let g_zero = &laid_zero.paragraphs[0].galley;
        let g_frac = &laid_frac.paragraphs[0].galley;
        assert_eq!(g_zero.rows.len(), g_frac.rows.len());
        for (r0, r1) in g_zero.rows.iter().zip(g_frac.rows.iter()) {
            assert_eq!(r0.glyphs.len(), r1.glyphs.len());
            for (g0, g1) in r0.glyphs.iter().zip(r1.glyphs.iter()) {
                assert!(
                    (g0.pos.x - g1.pos.x).abs() < 1e-4,
                    "align={align:?}: glyph pos.x should not depend on rect.min.x ({} vs {})",
                    g0.pos.x,
                    g1.pos.x
                );
            }
        }
    }
}
