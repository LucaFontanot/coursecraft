//! Permanent PowerPoint import fidelity integration test (M26 item A13).
//!
//! Imports `crates/pptx/tests/fidelity/fidelity.pptx`, renders each visible
//! slide at 640 px using the player render path, compares against the
//! PowerPoint-exported PNGs, and verifies structural import invariants.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use coursecraft_course::{FillStyle, ListKind, ObjectKind, ShapeKind};
use coursecraft_pptx::import_pptx;
use coursecraft_render::{
    MediaFetch, MediaSource, RenderMode, RenderOptions, RgbaImage, decode_png, render_slide_rgba,
};

struct InMemoryMedia(HashMap<String, Arc<[u8]>>);

impl MediaSource for InMemoryMedia {
    fn get(&self, src: &str) -> MediaFetch {
        match self.0.get(src) {
            Some(bytes) => MediaFetch::Ready(Arc::clone(bytes)),
            None => MediaFetch::Missing,
        }
    }
}

fn fidelity_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../crates/pptx/tests/fidelity")
}

/// Bilinearly resize an RGBA image to `(dst_w, dst_h)`.
fn resize_rgba(src: &RgbaImage, dst_w: u32, dst_h: u32) -> RgbaImage {
    if src.width == dst_w && src.height == dst_h {
        return src.clone();
    }
    let mut out = vec![0u8; (dst_w * dst_h * 4) as usize];
    let x_scale = src.width as f64 / dst_w as f64;
    let y_scale = src.height as f64 / dst_h as f64;

    for dy in 0..dst_h {
        let sy = ((dy as f64 + 0.5) * y_scale - 0.5).clamp(0.0, (src.height - 1) as f64);
        let y0 = sy.floor() as u32;
        let y1 = (y0 + 1).min(src.height - 1);
        let fy = sy - y0 as f64;

        for dx in 0..dst_w {
            let sx = ((dx as f64 + 0.5) * x_scale - 0.5).clamp(0.0, (src.width - 1) as f64);
            let x0 = sx.floor() as u32;
            let x1 = (x0 + 1).min(src.width - 1);
            let fx = sx - x0 as f64;

            let dst_idx = ((dy * dst_w + dx) * 4) as usize;
            for c in 0..4 {
                let p00 = src.pixels[((y0 * src.width + x0) * 4) as usize + c] as f64;
                let p10 = src.pixels[((y0 * src.width + x1) * 4) as usize + c] as f64;
                let p01 = src.pixels[((y1 * src.width + x0) * 4) as usize + c] as f64;
                let p11 = src.pixels[((y1 * src.width + x1) * 4) as usize + c] as f64;
                let top = p00 * (1.0 - fx) + p10 * fx;
                let bot = p01 * (1.0 - fx) + p11 * fx;
                let val = top * (1.0 - fy) + bot * fy;
                out[dst_idx + c] = val.round().clamp(0.0, 255.0) as u8;
            }
        }
    }

    RgbaImage {
        width: dst_w,
        height: dst_h,
        pixels: out,
    }
}

/// Similarity score in `0.0..=1.0` (1.0 = identical), based on mean absolute
/// difference per RGB channel.
fn rgb_similarity(a: &RgbaImage, b: &RgbaImage) -> f64 {
    assert_eq!(a.width, b.width, "image widths must match");
    assert_eq!(a.height, b.height, "image heights must match");
    let mut sum_abs_diff: u64 = 0;
    let pixel_count = (a.width as usize) * (a.height as usize);
    for i in 0..pixel_count {
        let base = i * 4;
        for c in 0..3 {
            sum_abs_diff += a.pixels[base + c].abs_diff(b.pixels[base + c]) as u64;
        }
    }
    let max_diff = (pixel_count as f64) * 3.0 * 255.0;
    1.0 - (sum_abs_diff as f64 / max_diff)
}

/// Per-slide minimum similarity thresholds (measured baseline minus 0.03).
const MIN_SCORES: [f64; 18] = [
    0.9390, // Slide 01 (measured 0.9690)
    0.9308, // Slide 02 (measured 0.9608)
    0.9409, // Slide 03 (measured 0.9709)
    0.9549, // Slide 04 (measured 0.9849)
    0.9498, // Slide 05 (measured 0.9798)
    0.8859, // Slide 06 (measured 0.9159)
    0.9401, // Slide 07 (measured 0.9701)
    0.9212, // Slide 08 (measured 0.9512)
    0.9008, // Slide 09 (measured 0.9308)
    0.9353, // Slide 10 (measured 0.9653)
    0.9402, // Slide 11 (measured 0.9702)
    0.9310, // Slide 12 (measured 0.9610)
    0.9453, // Slide 13 (measured 0.9753)
    0.9543, // Slide 14 (measured 0.9843)
    0.9632, // Slide 15 (measured 0.9932)
    0.9589, // Slide 16 (measured 0.9889)
    0.9600, // Slide 17 (measured 0.9900)
    0.9387, // Slide 18 (measured 0.9687)
];

#[test]
fn pptx_fidelity_pixel_similarity() {
    let dir = fidelity_dir();
    let pptx_bytes = std::fs::read(dir.join("fidelity.pptx")).expect("read fidelity.pptx");
    let imported = import_pptx(&pptx_bytes, "Fidelity").expect("import fidelity.pptx");

    let media_map: HashMap<String, Arc<[u8]>> = imported
        .media
        .into_iter()
        .map(|(name, bytes)| (name, Arc::<[u8]>::from(bytes)))
        .collect();
    let media = InMemoryMedia(media_map);

    let slides = &imported.course.scenes[0].slides;
    assert!(
        slides.len() >= 18,
        "expected at least 18 slides, got {}",
        slides.len()
    );

    let opts = RenderOptions {
        width: 640,
        mode: RenderMode::Player,
        layers: Vec::new(),
        at_ms: None,
    };

    let mut failures = Vec::new();
    for (idx, min_score) in MIN_SCORES.iter().enumerate() {
        let slide_num = idx + 1;
        let slide = &slides[idx];
        let rendered = render_slide_rgba(&imported.course, &slide.id, &opts, &media)
            .unwrap_or_else(|e| panic!("render slide {slide_num} ({}): {e}", slide.id));

        let ref_path = dir.join(format!("Slide{slide_num}.png"));
        let ref_bytes =
            std::fs::read(&ref_path).unwrap_or_else(|e| panic!("read {}: {e}", ref_path.display()));
        let ref_raw =
            decode_png(&ref_bytes).unwrap_or_else(|e| panic!("decode {}: {e}", ref_path.display()));
        let ref_img = resize_rgba(&ref_raw, rendered.width, rendered.height);

        let score = rgb_similarity(&rendered, &ref_img);
        println!("slide {slide_num:02}: score={score:.4} (min={min_score:.4})");
        if score < *min_score {
            failures.push(format!(
                "slide {slide_num:02}: score {score:.4} < min {min_score:.4}"
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "PPTX fidelity regressions:\n{}",
        failures.join("\n")
    );
}

#[test]
fn pptx_fidelity_structural_checks() {
    let dir = fidelity_dir();
    let pptx_bytes = std::fs::read(dir.join("fidelity.pptx")).expect("read fidelity.pptx");
    let imported = import_pptx(&pptx_bytes, "Fidelity").expect("import fidelity.pptx");
    let slides = &imported.course.scenes[0].slides;

    // 1. Slide count: 18 visible slides (hidden slide 19 skipped).
    assert_eq!(
        slides.len(),
        18,
        "expected 18 visible slides (hidden slide 19 skipped), got {}",
        slides.len()
    );

    // 2. Slide backgrounds: Slide 1 has solid background, Slide 16 has gradient
    //    (background or full-slide gradient shape), Slide 17 has picture background.
    assert!(
        slides[0].background.is_some(),
        "slide 1 should have a solid background"
    );
    let slide16_has_bg = slides[15].background.is_some()
        || slides[15].objects.iter().any(|o| {
            matches!(
                &o.kind,
                ObjectKind::Shape {
                    fill_style: Some(FillStyle::Linear { .. } | FillStyle::Radial { .. }),
                    ..
                }
            )
        });
    assert!(slide16_has_bg, "slide 16 should have a gradient background");
    let slide17_has_bg = slides[16].background.is_some()
        || slides[16].objects.iter().any(|o| {
            matches!(
                &o.kind,
                ObjectKind::Image { .. }
                    | ObjectKind::Shape {
                        fill_style: Some(FillStyle::Picture { .. }),
                        ..
                    }
            )
        });
    assert!(slide17_has_bg, "slide 17 should have a picture background");

    // 3. Gradient stop counts (2, 3, 5) on Slide 6.
    let mut stop_counts = BTreeSet::new();
    for obj in &slides[5].objects {
        if let ObjectKind::Shape {
            fill_style: Some(FillStyle::Linear { stops, .. } | FillStyle::Radial { stops, .. }),
            ..
        } = &obj.kind
        {
            stop_counts.insert(stops.len());
        }
    }
    for expected in [2usize, 3, 5] {
        assert!(
            stop_counts.contains(&expected),
            "slide 6 should have a {expected}-stop gradient, found stop counts: {stop_counts:?}"
        );
    }

    // 4. Cropped picture has its crop (Slide 12).
    let has_crop = slides[11]
        .objects
        .iter()
        .any(|o| matches!(&o.kind, ObjectKind::Image { crop: Some(_), .. }));
    assert!(has_crop, "slide 12 should contain a cropped Image object");

    // 5. Star and heart shapes are not rectangles (Slide 1).
    let slide1_shapes: Vec<ShapeKind> = slides[0]
        .objects
        .iter()
        .filter_map(|o| match &o.kind {
            ObjectKind::Shape { shape, .. } => Some(*shape),
            _ => None,
        })
        .collect();
    assert!(
        slide1_shapes.contains(&ShapeKind::Star5),
        "slide 1 should contain Star5, found: {slide1_shapes:?}"
    );
    assert!(
        slide1_shapes.contains(&ShapeKind::Star8),
        "slide 1 should contain Star8, found: {slide1_shapes:?}"
    );
    assert!(
        slide1_shapes.contains(&ShapeKind::Heart),
        "slide 1 should contain Heart, found: {slide1_shapes:?}"
    );

    // 6. Video object exists (Slide 15).
    let has_video = slides[14]
        .objects
        .iter()
        .any(|o| matches!(&o.kind, ObjectKind::Video { .. }));
    assert!(has_video, "slide 15 should contain a Video object");

    // 7. Table has its merged cell (Slide 14).
    let has_merged_cell = slides[13].objects.iter().any(|o| {
        if let ObjectKind::Table { rows, .. } = &o.kind {
            rows.iter()
                .flat_map(|r| &r.cells)
                .any(|c| c.col_span > 1 || c.row_span > 1)
        } else {
            false
        }
    });
    assert!(
        has_merged_cell,
        "slide 14 table should contain at least one merged cell"
    );

    // 8. Bullet levels present (Slide 10 and/or Slide 18).
    let mut bullet_levels = BTreeSet::new();
    for slide_idx in [9usize, 17] {
        for obj in &slides[slide_idx].objects {
            let paras = match &obj.kind {
                ObjectKind::Text { paragraphs, .. } => paragraphs.as_slice(),
                ObjectKind::Shape { text: Some(st), .. } => st.paragraphs.as_slice(),
                _ => &[],
            };
            for p in paras {
                if p.list == ListKind::Bullet {
                    bullet_levels.insert(p.level);
                }
            }
        }
    }
    assert!(
        bullet_levels.contains(&0) && bullet_levels.iter().any(|&lvl| lvl > 0),
        "slides 10/18 should have multi-level bullets (level 0 and >0), found: {bullet_levels:?}"
    );
}

#[test]
fn c2_9_slide_14_styleless_table_renders_black_gridlines() {
    let dir = fidelity_dir();
    let pptx_bytes = std::fs::read(dir.join("fidelity.pptx")).expect("read fidelity.pptx");
    let imported = import_pptx(&pptx_bytes, "Fidelity").expect("import fidelity.pptx");

    let media_map: HashMap<String, Arc<[u8]>> = imported
        .media
        .into_iter()
        .map(|(name, bytes)| (name, Arc::<[u8]>::from(bytes)))
        .collect();
    let media = InMemoryMedia(media_map);

    let slide14 = &imported.course.scenes[0].slides[13];
    let tables: Vec<_> = slide14
        .objects
        .iter()
        .filter_map(|o| match &o.kind {
            ObjectKind::Table {
                columns,
                rows,
                borders,
                ..
            } => Some((o.frame, columns, rows, borders)),
            _ => None,
        })
        .collect();
    assert!(
        tables.len() >= 2,
        "expected at least 2 tables on slide 14, got {}",
        tables.len()
    );

    // Table 2 is the style-less table; verify it has 1pt #000000 outer & inner borders.
    let (frame2, cols2, rows2, borders2) = tables[1];
    let outer = borders2
        .outer
        .as_ref()
        .expect("style-less Table 2 must have outer border");
    let inner = borders2
        .inner
        .as_ref()
        .expect("style-less Table 2 must have inner border");
    assert_eq!(outer.color.0, "#000000");
    assert_eq!(inner.color.0, "#000000");
    assert!(
        (outer.width - 1.333).abs() < 0.2,
        "outer width={}",
        outer.width
    );
    assert!(
        (inner.width - 1.333).abs() < 0.2,
        "inner width={}",
        inner.width
    );

    let opts = RenderOptions {
        width: 1280,
        mode: RenderMode::Player,
        layers: Vec::new(),
        at_ms: None,
    };
    let rendered = render_slide_rgba(&imported.course, &slide14.id, &opts, &media)
        .expect("render slide 14 at 1280px");
    assert_eq!(rendered.width, 1280);

    // Check that dark gridline pixels (r<30, g<30, b<30) appear along the first
    // inner column boundary and first inner row boundary of Table 2.
    assert!(cols2.len() >= 2 && rows2.len() >= 2);
    let x_boundary = (frame2.x + frame2.w * cols2[0]).round() as i32;
    let y_mid_row0 = (frame2.y + frame2.h * rows2[0].height * 0.5).round() as i32;
    let mut found_vertical_grid = false;
    for dx in -2..=2 {
        let px = (x_boundary + dx).clamp(0, rendered.width as i32 - 1) as u32;
        let py = y_mid_row0.clamp(0, rendered.height as i32 - 1) as u32;
        let idx = ((py * rendered.width + px) * 4) as usize;
        if rendered.pixels[idx] < 30
            && rendered.pixels[idx + 1] < 30
            && rendered.pixels[idx + 2] < 30
        {
            found_vertical_grid = true;
            break;
        }
    }
    assert!(
        found_vertical_grid,
        "expected dark gridline pixel near vertical cell boundary x={x_boundary}, y={y_mid_row0}"
    );

    let y_boundary = (frame2.y + frame2.h * rows2[0].height).round() as i32;
    let x_mid_col0 = (frame2.x + frame2.w * cols2[0] * 0.5).round() as i32;
    let mut found_horizontal_grid = false;
    for dy in -2..=2 {
        let px = x_mid_col0.clamp(0, rendered.width as i32 - 1) as u32;
        let py = (y_boundary + dy).clamp(0, rendered.height as i32 - 1) as u32;
        let idx = ((py * rendered.width + px) * 4) as usize;
        if rendered.pixels[idx] < 30
            && rendered.pixels[idx + 1] < 30
            && rendered.pixels[idx + 2] < 30
        {
            found_horizontal_grid = true;
            break;
        }
    }
    assert!(
        found_horizontal_grid,
        "expected dark gridline pixel near horizontal cell boundary x={x_mid_col0}, y={y_boundary}"
    );
}
