//! M26 bug-hunt integration tests for A13: PowerPoint import & visual fidelity.
//!
//! Ordered P1 -> P2 -> P3.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use coursecraft_course::{FillStyle, ListKind, ObjectKind, Severity, ShapeKind};
use coursecraft_logic::{Effect, Input, Player};
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
                let val =
                    (p00 * (1.0 - fx) + p10 * fx) * (1.0 - fy) + (p01 * (1.0 - fx) + p11 * fx) * fy;
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

fn rgb_similarity(a: &RgbaImage, b: &RgbaImage) -> f64 {
    let mut sum_abs_diff: u64 = 0;
    let pixel_count = (a.width as usize) * (a.height as usize);
    for i in 0..pixel_count {
        let base = i * 4;
        for c in 0..3 {
            sum_abs_diff += a.pixels[base + c].abs_diff(b.pixels[base + c]) as u64;
        }
    }
    1.0 - (sum_abs_diff as f64 / ((pixel_count as f64) * 3.0 * 255.0))
}

const THRESHOLDS: [f64; 18] = [
    0.91, 0.90, 0.92, 0.92, 0.93, 0.88, 0.93, 0.93, 0.89, 0.90, 0.90, 0.90, 0.91, 0.90, 0.80, 0.85,
    0.85, 0.90,
];

// ============================================================================
// P1
// ============================================================================

#[test]
fn a13_1_fidelity_pptx_visual_similarity_meets_thresholds() {
    let dir = fidelity_dir();
    let pptx_bytes = std::fs::read(dir.join("fidelity.pptx")).expect("read fidelity.pptx");
    let imported = import_pptx(&pptx_bytes, "Fidelity").expect("import fidelity.pptx");
    assert_eq!(imported.course.scenes[0].slides.len(), 18);

    let media_map: HashMap<String, Arc<[u8]>> = imported
        .media
        .into_iter()
        .map(|(k, v)| (k, Arc::<[u8]>::from(v)))
        .collect();
    let media = InMemoryMedia(media_map);
    let opts = RenderOptions {
        width: 640,
        mode: RenderMode::Authoring,
        ..Default::default()
    };

    let mut failures = Vec::new();
    for (idx, min_sim) in THRESHOLDS.iter().enumerate() {
        let slide_num = idx + 1;
        let slide = &imported.course.scenes[0].slides[idx];
        let rendered = render_slide_rgba(&imported.course, &slide.id, &opts, &media).unwrap();
        let ref_bytes = std::fs::read(dir.join(format!("Slide{slide_num}.png"))).unwrap();
        let ref_img = decode_png(&ref_bytes).unwrap();
        let ref_resized = resize_rgba(&ref_img, rendered.width, rendered.height);
        let sim = rgb_similarity(&rendered, &ref_resized);
        if sim < *min_sim {
            failures.push(format!(
                "Slide {slide_num:02}: sim={sim:.4} < min={min_sim:.4}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "Fidelity similarity regressions:\n{}",
        failures.join("\n")
    );
}

#[test]
fn a13_2_fidelity_pptx_structural_invariants() {
    let dir = fidelity_dir();
    let pptx_bytes = std::fs::read(dir.join("fidelity.pptx")).expect("read fidelity.pptx");
    let imported = import_pptx(&pptx_bytes, "Fidelity").expect("import fidelity.pptx");
    let slides = &imported.course.scenes[0].slides;
    assert_eq!(slides.len(), 18);

    // S01: solid bg + basic shapes
    assert!(slides[0].background.is_some());
    assert!(slides[0].objects.iter().any(|o| matches!(
        o.kind,
        ObjectKind::Shape {
            shape: ShapeKind::Heart,
            ..
        }
    )));

    // S06: linear/radial/picture fills
    assert!(slides[5].objects.iter().any(|o| matches!(
        &o.kind,
        ObjectKind::Shape {
            fill_style: Some(FillStyle::Linear { stops, .. }),
            ..
        } if stops.len() >= 3
    )));

    // S10: bullets & numbering
    assert!(slides[9].objects.iter().any(|o| {
        match &o.kind {
            ObjectKind::Text { paragraphs, .. } => paragraphs
                .iter()
                .any(|p| p.level >= 2 || matches!(p.list, ListKind::Number)),
            _ => false,
        }
    }));

    // S14: table with header_row & merged cell
    assert!(slides[13].objects.iter().any(|o| match &o.kind {
        ObjectKind::Table {
            rows, header_row, ..
        } => {
            *header_row
                && rows
                    .iter()
                    .flat_map(|r| r.cells.iter())
                    .any(|c| c.col_span > 1 || c.row_span > 1)
        }
        _ => false,
    }));

    // S15: video with poster
    assert!(slides[14].objects.iter().any(|o| matches!(
        &o.kind,
        ObjectKind::Video {
            poster: Some(_),
            ..
        }
    )));
}

#[test]
fn a13_3_fidelity_pptx_validates_plays_and_publishes() {
    let dir = fidelity_dir();
    let pptx_bytes = std::fs::read(dir.join("fidelity.pptx")).expect("read fidelity.pptx");
    let imported = import_pptx(&pptx_bytes, "Fidelity").expect("import fidelity.pptx");

    let errors: Vec<_> = imported
        .course
        .validate()
        .into_iter()
        .filter(|i| i.severity == Severity::Error)
        .collect();
    assert!(errors.is_empty(), "validation errors: {errors:?}");

    let mut player = Player::new(&imported.course).expect("player");
    let effs = player.handle(&Input::Start);
    assert!(!effs.iter().any(|e| matches!(e, Effect::Warning { .. })));
    for _ in 0..17 {
        let e1 = player.handle(&Input::Tick { ms: 500 });
        let e2 = player.handle(&Input::Next);
        assert!(!e1.iter().any(|e| matches!(e, Effect::Warning { .. })));
        assert!(!e2.iter().any(|e| matches!(e, Effect::Warning { .. })));
    }

    let ccraft_path = std::env::temp_dir().join("m26-bughunt-fid.ccraft");
    let media_refs: Vec<(String, Vec<u8>)> = imported
        .media
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let bytes =
        coursecraft_format::save_project(&imported.course, &media_refs).expect("save .ccraft");
    std::fs::write(&ccraft_path, &bytes).unwrap();
    let _ = coursecraft_format::open_project(&bytes).expect("reopen .ccraft");

    let player_dist = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/player-dist");
    if player_dist.exists() {
        let work_dir = std::env::temp_dir().join("m26-bughunt-fid-pub");
        let _ = std::fs::remove_dir_all(&work_dir);
        std::fs::create_dir_all(work_dir.join("media")).unwrap();
        let json_path = work_dir.join("course.json");
        std::fs::write(&json_path, imported.course.to_json().unwrap()).unwrap();
        for (name, bytes) in &imported.media {
            std::fs::write(work_dir.join("media").join(name), bytes).unwrap();
        }
        let out_zip = std::env::temp_dir().join("m26-bughunt-fid-web.zip");
        let bin = env!("CARGO_BIN_EXE_coursecraft-cli");
        assert!(
            std::process::Command::new(bin)
                .args([
                    "publish",
                    json_path.to_str().unwrap(),
                    "-o",
                    out_zip.to_str().unwrap(),
                    "--standard",
                    "web",
                ])
                .status()
                .unwrap()
                .success()
        );
        let _ = std::fs::remove_dir_all(&work_dir);
        let _ = std::fs::remove_file(&out_zip);
    }
    let _ = std::fs::remove_file(&ccraft_path);
}

#[test]
fn a13_9_fidelity_pptx_editable_in_session_and_roundtrips() {
    let dir = fidelity_dir();
    let pptx_bytes = std::fs::read(dir.join("fidelity.pptx")).expect("read fidelity.pptx");
    let imported = import_pptx(&pptx_bytes, "Fidelity").expect("import fidelity.pptx");
    let mut sess = coursecraft_engine::Session::new(imported.course);

    // Edit text on S08
    let s8 = &sess.course.scenes[0].slides[7];
    let s8_id = s8.id.clone();
    let txt_id = s8
        .objects
        .iter()
        .find(|o| matches!(o.kind, ObjectKind::Text { .. }))
        .map(|o| o.id.clone())
        .expect("S08 text");
    sess.execute(
        "text.style_range",
        &serde_json::json!({ "slide": s8_id, "object": txt_id, "start": 0, "end": 0, "size": 28.0 }),
    )
    .expect("style_range on imported text");
    assert!(sess.undo().is_ok());
    assert!(sess.redo().is_ok());
}

// ============================================================================
// P3
// ============================================================================

#[test]
fn a13_10_cli_import_validate_render_export_pdf_pipeline() {
    let dir = fidelity_dir();
    let pptx_path = dir.join("fidelity.pptx");
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/m26-cli-fid-test");
    std::fs::create_dir_all(&out_dir).unwrap();
    let ccraft = out_dir.join("fid.ccraft");
    let png = out_dir.join("s16.png");
    let pdf = out_dir.join("fid.pdf");

    let bin = env!("CARGO_BIN_EXE_coursecraft-cli");
    assert!(
        std::process::Command::new(bin)
            .args([
                "import-pptx",
                pptx_path.to_str().unwrap(),
                "-o",
                ccraft.to_str().unwrap()
            ])
            .status()
            .unwrap()
            .success()
    );
    assert!(
        std::process::Command::new(bin)
            .args(["validate", ccraft.to_str().unwrap()])
            .status()
            .unwrap()
            .success()
    );
    assert!(
        std::process::Command::new(bin)
            .args([
                "render",
                ccraft.to_str().unwrap(),
                "--slide",
                "slide16",
                "-o",
                png.to_str().unwrap(),
            ])
            .status()
            .unwrap()
            .success()
    );
    assert!(
        std::process::Command::new(bin)
            .args([
                "export-pdf",
                ccraft.to_str().unwrap(),
                "-o",
                pdf.to_str().unwrap()
            ])
            .status()
            .unwrap()
            .success()
    );
    let pdf_bytes = std::fs::read(&pdf).unwrap();
    assert!(pdf_bytes.starts_with(b"%PDF-"));
}
