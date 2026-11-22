use coursecraft_course::{Course, LineJoin, ShapeKind};

use crate::compose::{flip_point, rotate_point};
use crate::media::{MediaFetch, MediaSource};
use crate::offscreen::{RenderOptions, RgbaImage, render_slide_rgba};
use crate::shape_geometry::shape_contours_adjusted;

struct NoMedia;
impl MediaSource for NoMedia {
    fn get(&self, _src: &str) -> MediaFetch {
        MediaFetch::Missing
    }
}

#[allow(clippy::too_many_arguments)]
fn make_shape_course(
    kind: ShapeKind,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    rotation: f32,
    flip_h: bool,
    flip_v: bool,
    outline_width: f32,
    join: LineJoin,
    miter_limit: Option<f32>,
) -> Course {
    let json = serde_json::json!({
        "schema_version": 1,
        "id": "outline-test",
        "title": "Outline Test",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "sc1",
            "title": "S",
            "slides": [{
                "id": "slide-1",
                "title": "S1",
                "background": "#FFFFFF",
                "duration_ms": 5000,
                "objects": [{
                    "id": "obj-1",
                    "frame": { "x": x, "y": y, "w": w, "h": h },
                    "rotation": rotation,
                    "flip_h": flip_h,
                    "flip_v": flip_v,
                    "kind": {
                        "type": "shape",
                        "shape": kind,
                        "fill": "#2ECC71",
                        "outline": {
                            "color": "#111111",
                            "width": outline_width,
                            "join": join,
                            "miter_limit": miter_limit
                        }
                    }
                }]
            }]
        }]
    })
    .to_string();
    Course::from_json(&json).expect("valid course json")
}

fn render(course: &Course) -> RgbaImage {
    render_slide_rgba(
        course,
        "slide-1",
        &RenderOptions {
            width: 1280,
            ..Default::default()
        },
        &NoMedia,
    )
    .expect("render slide")
}

fn is_outline_pixel(img: &RgbaImage, x: i32, y: i32) -> bool {
    if x < 0 || y < 0 || x >= img.width as i32 || y >= img.height as i32 {
        return false;
    }
    let idx = ((y as u32 * img.width + x as u32) * 4) as usize;
    let r = img.pixels[idx];
    let g = img.pixels[idx + 1];
    let b = img.pixels[idx + 2];
    r < 80 && g < 80 && b < 80
}

fn count_outline_pixels_near(img: &RgbaImage, cx: i32, cy: i32, half_box: i32) -> usize {
    let x0 = (cx - half_box).max(0);
    let x1 = (cx + half_box).min(img.width as i32 - 1);
    let y0 = (cy - half_box).max(0);
    let y1 = (cy + half_box).min(img.height as i32 - 1);
    (y0..=y1)
        .flat_map(|y| (x0..=x1).map(move |x| (x, y)))
        .filter(|&(x, y)| is_outline_pixel(img, x, y))
        .count()
}

fn measure_span_at(img: &RgbaImage, cx: i32, cy: i32, dx: i32, dy: i32, radius: i32) -> i32 {
    (-radius..=radius)
        .filter(|&step| is_outline_pixel(img, cx + step * dx, cy + step * dy))
        .count() as i32
}

#[allow(clippy::too_many_arguments)]
fn transform_pt(
    pt: (f32, f32),
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    rotation: f32,
    flip_h: bool,
    flip_v: bool,
) -> (f32, f32) {
    let centre = egui::pos2(x + w * 0.5, y + h * 0.5);
    let p = egui::pos2(x + pt.0, y + pt.1);
    let r = rotate_point(p, centre, rotation);
    let f = flip_point(r, centre, flip_h, flip_v);
    (f.x, f.y)
}

#[test]
fn rectangle_outline_thickness_equal_on_all_sides_and_rotations() {
    let x: f32 = 440.0;
    let y: f32 = 210.0;
    let w: f32 = 240.0;
    let h: f32 = 140.0;
    let cx = (x + w * 0.5).round() as i32;
    let cy = (y + h * 0.5).round() as i32;

    for &width in &[1.0, 6.0, 20.0] {
        let mut zero_deg_sides = None;
        for &rot in &[0.0, 90.0, 180.0, 270.0] {
            for &(fh, fv) in &[(false, false), (true, false), (false, true), (true, true)] {
                let course = make_shape_course(
                    ShapeKind::Rectangle,
                    x,
                    y,
                    w,
                    h,
                    rot,
                    fh,
                    fv,
                    width,
                    LineJoin::Miter,
                    None,
                );
                let img = render(&course);
                let (rw, rh) = if (rot - 90.0).abs() < 1.0 || (rot - 270.0).abs() < 1.0 {
                    (h, w)
                } else {
                    (w, h)
                };
                let top =
                    measure_span_at(&img, cx, (cy as f32 - rh * 0.5).round() as i32, 0, 1, 25);
                let bottom =
                    measure_span_at(&img, cx, (cy as f32 + rh * 0.5).round() as i32, 0, 1, 25);
                let left =
                    measure_span_at(&img, (cx as f32 - rw * 0.5).round() as i32, cy, 1, 0, 25);
                let right =
                    measure_span_at(&img, (cx as f32 + rw * 0.5).round() as i32, cy, 1, 0, 25);
                for (side, measured) in [
                    ("top", top),
                    ("bottom", bottom),
                    ("left", left),
                    ("right", right),
                ] {
                    assert!(
                        (measured as f32 - width).abs() <= 1.5,
                        "Rectangle w={width} rot={rot} fh={fh} fv={fv} {side} thickness={measured}, expected ~{width}"
                    );
                }
                if rot == 0.0 && !fh && !fv {
                    zero_deg_sides = Some((top, bottom, left, right));
                } else if (rot - 90.0).abs() < 1.0 && !fh && !fv {
                    let (t0, b0, l0, r0) = zero_deg_sides.expect("0 deg measured");
                    assert!(
                        (top - r0).abs() <= 1
                            && (bottom - l0).abs() <= 1
                            && (left - t0).abs() <= 1
                            && (right - b0).abs() <= 1,
                        "Rectangle w={width} 90° thickness ({top},{bottom},{left},{right}) differs from 0° ({t0},{b0},{l0},{r0})"
                    );
                }
            }
        }
    }
}

#[test]
fn sharp_shapes_preserve_outline_area_and_uncut_tips_across_rotations_flips_and_widths() {
    let x: f32 = 460.0;
    let y: f32 = 230.0;
    let w: f32 = 200.0;
    let h: f32 = 120.0;
    let cx = (x + w * 0.5).round() as i32;
    let cy = (y + h * 0.5).round() as i32;

    let shapes = [
        ShapeKind::ArrowRight,
        ShapeKind::ArrowLeft,
        ShapeKind::ArrowUp,
        ShapeKind::ArrowDown,
        ShapeKind::ArrowPentagon,
        ShapeKind::ArrowChevron,
        ShapeKind::Triangle,
        ShapeKind::Diamond,
        ShapeKind::Star5,
        ShapeKind::CalloutRectangle,
    ];

    for &width in &[1.0, 6.0, 20.0] {
        for &kind in &shapes {
            let base_course = make_shape_course(
                kind,
                x,
                y,
                w,
                h,
                0.0,
                false,
                false,
                width,
                LineJoin::Miter,
                None,
            );
            let base_img = render(&base_course);
            let base_area = count_outline_pixels_near(&base_img, cx, cy, 220);
            assert!(
                base_area > 0,
                "{kind:?} w={width} should have outline pixels"
            );

            let contours = shape_contours_adjusted(kind, w, h, &[]);
            let tip_local = contours.first().and_then(|c| {
                c.iter()
                    .max_by(|a, b| {
                        let da = (a.0 - w * 0.5).hypot(a.1 - h * 0.5);
                        let db = (b.0 - w * 0.5).hypot(b.1 - h * 0.5);
                        da.total_cmp(&db)
                    })
                    .copied()
            });

            // Check all 6 rotations × 4 flip combinations on thick outlines, and representative
            // rotations/flips on 1px/6px to keep runtime fast while covering every combination.
            let flips: &[(bool, bool)] = if width >= 20.0 {
                &[(false, false), (true, false), (false, true), (true, true)]
            } else {
                &[(false, false), (true, false), (false, true)]
            };
            let rotations: &[f32] = if width >= 20.0 {
                &[0.0, 30.0, 45.0, 90.0, 180.0, 270.0]
            } else {
                &[0.0, 45.0, 90.0, 180.0]
            };

            for &rot in rotations {
                for &(fh, fv) in flips {
                    let course = make_shape_course(
                        kind,
                        x,
                        y,
                        w,
                        h,
                        rot,
                        fh,
                        fv,
                        width,
                        LineJoin::Miter,
                        None,
                    );
                    let img = render(&course);
                    let area = count_outline_pixels_near(&img, cx, cy, 220);
                    assert!(
                        area > 0,
                        "{kind:?} w={width} rot={rot} fh={fh} fv={fv}: no outline pixels"
                    );
                    if width >= 6.0 {
                        let rel_diff = (area as f32 - base_area as f32).abs() / base_area as f32;
                        assert!(
                            rel_diff <= 0.12,
                            "{kind:?} w={width} rot={rot} fh={fh} fv={fv}: outline area {area} differs from 0° base {base_area} by {:.1}%",
                            rel_diff * 100.0
                        );
                    }

                    if width >= 6.0
                        && let Some(tl) = tip_local
                    {
                        let tip = transform_pt(tl, x, y, w, h, rot, fh, fv);
                        let dx = tip.0 - (x + w * 0.5);
                        let dy = tip.1 - (y + h * 0.5);
                        let len = dx.hypot(dy).max(1e-3);
                        let probe_dist = if width >= 20.0 { 5.0 } else { 2.0 };
                        let probe_x = (tip.0 + (dx / len) * probe_dist).round() as i32;
                        let probe_y = (tip.1 + (dy / len) * probe_dist).round() as i32;
                        assert!(
                            is_outline_pixel(&img, probe_x, probe_y),
                            "{kind:?} w={width} rot={rot} fh={fh} fv={fv}: missing tip pixel at ({probe_x}, {probe_y}) past vertex ({:.1}, {:.1})",
                            tip.0,
                            tip.1
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn line_join_miter_round_and_bevel_behave_distinctly_on_acute_tip() {
    // Narrow tall triangle: top vertex has an acute angle (~22°), so miter_len ≈ 5.2 * half.
    // With default PowerPoint miter limit (8.0), miter stays sharp; with bevel or miter_limit=4.0,
    // the tip is cut flat; with round, the tip extends by ~half (10px) smoothly.
    let x: f32 = 580.0;
    let y: f32 = 200.0;
    let w: f32 = 80.0;
    let h: f32 = 200.0;
    let width = 20.0;
    let tip_x = (x + w * 0.5).round() as i32;
    let tip_y = y.round() as i32;

    let miter_img = render(&make_shape_course(
        ShapeKind::Triangle,
        x,
        y,
        w,
        h,
        0.0,
        false,
        false,
        width,
        LineJoin::Miter,
        None,
    ));
    let bevel_img = render(&make_shape_course(
        ShapeKind::Triangle,
        x,
        y,
        w,
        h,
        0.0,
        false,
        false,
        width,
        LineJoin::Bevel,
        None,
    ));
    let round_img = render(&make_shape_course(
        ShapeKind::Triangle,
        x,
        y,
        w,
        h,
        0.0,
        false,
        false,
        width,
        LineJoin::Round,
        None,
    ));

    // At 30px above the vertex, only Miter (limit 8 -> ~52px) reaches; Bevel & Round do not.
    assert!(
        is_outline_pixel(&miter_img, tip_x, tip_y - 30),
        "Miter (limit 8) should reach 30px past acute vertex"
    );
    assert!(
        !is_outline_pixel(&bevel_img, tip_x, tip_y - 30),
        "Bevel should not reach 30px past acute vertex"
    );
    assert!(
        !is_outline_pixel(&round_img, tip_x, tip_y - 30),
        "Round should not reach 30px past acute vertex"
    );

    // At 8px above the vertex (within half=10px), Round reaches; Bevel (cut at ~2px) does not.
    assert!(
        is_outline_pixel(&round_img, tip_x, tip_y - 8),
        "Round should reach 8px past vertex"
    );
    assert!(
        !is_outline_pixel(&bevel_img, tip_x, tip_y - 8),
        "Bevel should be cut flat before 8px past vertex"
    );
}
