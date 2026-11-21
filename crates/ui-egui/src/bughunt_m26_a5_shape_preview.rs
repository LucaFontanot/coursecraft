//! M26 bug-hunt tests for A5: Shape drawing drag preview.
//!
//! Ordered P1 -> P2 -> P3.

use crate::PlacementTool;
use crate::canvas::{canvas_fit_rect, canvas_rect};
use crate::test_harness::TestHarness;
use coursecraft_course::{Course, ObjectKind, ShapeKind, shape_label};
use coursecraft_render::SlideTransform;
use egui::epaint::{ClippedShape, Shape};
use egui::{Event, Modifiers, PointerButton, Pos2, Rect, pos2, vec2};

fn hello() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    Course::from_json(&json).expect("valid course")
}

fn pick_shape(h: &mut TestHarness, kind: ShapeKind) {
    h.idle_frame();
    let btn = h
        .find_icon(coursecraft_render::Icon::Shapes)
        .expect("Shapes button");
    h.click_at(btn);
    h.idle_frame();
    let search = h
        .editor
        .ui
        .shape_gallery_search_rect
        .expect("shape gallery search");
    h.set_field(search.center(), shape_label(kind));
    h.idle_frame();
    let icon_rect = h
        .editor
        .ui
        .shape_gallery_rects
        .iter()
        .find_map(|(c, r)| (*c == kind).then_some(*r))
        .unwrap_or_else(|| panic!("{kind:?} not in gallery"));
    h.click_at(icon_rect.center());
    h.idle_frame();
}

fn collect_shapes(shape: &Shape, out: &mut Vec<Shape>) {
    match shape {
        Shape::Vec(v) => {
            for s in v {
                collect_shapes(s, out);
            }
        }
        other => out.push(other.clone()),
    }
}

fn all_shapes(clipped: &[ClippedShape]) -> Vec<Shape> {
    let mut out = Vec::new();
    for c in clipped {
        collect_shapes(&c.shape, &mut out);
    }
    out
}

fn drag_preview(
    h: &mut TestHarness,
    start_slide: Pos2,
    delta_slide: egui::Vec2,
    modifiers: Modifiers,
) -> (Rect, Vec<Shape>) {
    let transform = SlideTransform::fit(
        canvas_fit_rect(canvas_rect(h.central_panel_rect())),
        h.editor.session.course.slide_size,
    )
    .expect("transform");
    let start = transform.to_screen_pos(start_slide);
    let end = transform.to_screen_pos(start_slide + delta_slide);
    if modifiers != Modifiers::NONE {
        h.frame(vec![Event::ModifiersChanged(modifiers)]);
    }
    h.frame(vec![Event::PointerMoved(start)]);
    h.frame(vec![Event::PointerButton {
        pos: start,
        button: PointerButton::Primary,
        pressed: true,
        modifiers,
    }]);
    let clipped = h.frame_shapes(vec![Event::PointerMoved(end)]);
    let preview_rect = Rect::from_two_pos(start, end);
    (preview_rect, all_shapes(&clipped))
}

fn release_at(h: &mut TestHarness, pos: Pos2, modifiers: Modifiers) {
    h.frame(vec![Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed: false,
        modifiers,
    }]);
    if modifiers != Modifiers::NONE {
        h.frame(vec![Event::ModifiersChanged(Modifiers::NONE)]);
    }
    h.idle_frame();
}

// ============================================================================
// P1
// ============================================================================

#[test]
fn a5_1_broad_shape_kind_sweep_draws_outline_mesh() {
    let kinds = [
        ShapeKind::Diamond,
        ShapeKind::Pentagon,
        ShapeKind::Hexagon,
        ShapeKind::Star8,
        ShapeKind::ArrowLeft,
        ShapeKind::ArrowLeftRight,
        ShapeKind::CalloutOval,
        ShapeKind::Cloud,
        ShapeKind::Moon,
        ShapeKind::Plus,
        ShapeKind::Donut,
        ShapeKind::BlockArc,
    ];
    for kind in kinds {
        let mut h = TestHarness::new(hello());
        pick_shape(&mut h, kind);
        let (preview_rect, shapes) = drag_preview(
            &mut h,
            pos2(360.0, 200.0),
            vec2(180.0, 140.0),
            Modifiers::NONE,
        );
        let accent = coursecraft_render::palette(&h.ctx).accent;
        let meshes: Vec<_> = shapes
            .iter()
            .filter_map(|s| match s {
                Shape::Mesh(m) if m.vertices.iter().any(|v| v.color == accent) => Some(m),
                _ => None,
            })
            .collect();
        assert!(!meshes.is_empty(), "expected outline mesh for {kind:?}");
        if kind == ShapeKind::Donut {
            let total_verts: usize = meshes.iter().map(|m| m.vertices.len()).sum();
            assert!(
                total_verts >= 24,
                "Donut should have outer+inner ring verts, got {total_verts}"
            );
        }
        let has_plain_rect = shapes.iter().any(|s| {
            matches!(s, Shape::Rect(r) if (r.rect.width() - preview_rect.width()).abs() < 4.0 && r.stroke.color == accent)
        });
        assert!(
            !has_plain_rect,
            "unexpected plain Rect preview for {kind:?}"
        );
    }
}

#[test]
fn a5_2_all_four_drag_quadrants_with_and_without_shift() {
    let deltas = [
        vec2(180.0, 120.0),
        vec2(-180.0, 120.0),
        vec2(180.0, -120.0),
        vec2(-180.0, -120.0),
    ];
    for delta in deltas {
        for mods in [Modifiers::NONE, Modifiers::SHIFT] {
            let mut h = TestHarness::new(hello());
            pick_shape(&mut h, ShapeKind::Heart);
            let (_, shapes) = drag_preview(&mut h, pos2(640.0, 360.0), delta, mods);
            let accent = coursecraft_render::palette(&h.ctx).accent;
            assert!(
                shapes.iter().any(
                    |s| matches!(s, Shape::Mesh(m) if m.vertices.iter().any(|v| v.color == accent))
                ),
                "missing preview mesh for delta {delta:?} mods {mods:?}"
            );
            let drag = h.editor.ui.placement_drag.as_ref().expect("placement_drag");
            if mods == Modifiers::SHIFT {
                assert!((drag.frame.width() - drag.frame.height()).abs() < 2.0);
            }
            let expected = drag.frame;
            let t = SlideTransform::fit(
                canvas_fit_rect(canvas_rect(h.central_panel_rect())),
                h.editor.session.course.slide_size,
            )
            .expect("t");
            let end_screen = t.to_screen_pos(pos2(640.0, 360.0) + delta);
            release_at(&mut h, end_screen, mods);
            let placed = h
                .editor
                .session
                .course
                .slide("s-intro")
                .unwrap()
                .objects
                .last()
                .unwrap();
            assert!(placed.frame.w > 0.0 && placed.frame.h > 0.0);
            assert!((placed.frame.w - expected.width()).abs() < 2.0);
        }
    }
}

#[test]
fn a5_3_degenerate_drags_and_single_click_placement() {
    for delta in [vec2(0.0, 140.0), vec2(140.0, 0.0)] {
        let mut h = TestHarness::new(hello());
        pick_shape(&mut h, ShapeKind::Star5);
        let (_, shapes) = drag_preview(&mut h, pos2(400.0, 200.0), delta, Modifiers::NONE);
        for s in &shapes {
            if let Shape::Mesh(m) = s {
                for v in &m.vertices {
                    assert!(
                        v.pos.x.is_finite() && v.pos.y.is_finite(),
                        "NaN vertex in {delta:?}"
                    );
                }
            }
        }
        let t = SlideTransform::fit(
            canvas_fit_rect(canvas_rect(h.central_panel_rect())),
            h.editor.session.course.slide_size,
        )
        .expect("t");
        release_at(
            &mut h,
            t.to_screen_pos(pos2(400.0, 200.0) + delta),
            Modifiers::NONE,
        );
        let placed = h
            .editor
            .session
            .course
            .slide("s-intro")
            .unwrap()
            .objects
            .last()
            .unwrap();
        assert!(placed.frame.w >= 1.0 && placed.frame.h >= 1.0);
    }

    // Single click
    let mut h = TestHarness::new(hello());
    pick_shape(&mut h, ShapeKind::Star5);
    let central = h.central_panel_rect();
    h.click_at(central.center());
    h.idle_frame();
    let placed = h
        .editor
        .session
        .course
        .slide("s-intro")
        .unwrap()
        .objects
        .last()
        .unwrap();
    assert!(placed.frame.w > 40.0 && placed.frame.h > 40.0);
}

#[test]
fn a5_4_cancel_mid_drag_with_escape() {
    let mut h = TestHarness::new(hello());
    let before = h
        .editor
        .session
        .course
        .slide("s-intro")
        .unwrap()
        .objects
        .len();
    pick_shape(&mut h, ShapeKind::Heart);
    let _ = drag_preview(
        &mut h,
        pos2(400.0, 220.0),
        vec2(180.0, 140.0),
        Modifiers::NONE,
    );
    assert!(h.editor.ui.placement_drag.is_some());

    h.press(egui::Key::Escape, Modifiers::NONE);
    let t = SlideTransform::fit(
        canvas_fit_rect(canvas_rect(h.central_panel_rect())),
        h.editor.session.course.slide_size,
    )
    .expect("t");
    release_at(&mut h, t.to_screen_pos(pos2(580.0, 360.0)), Modifiers::NONE);
    assert!(h.editor.ui.placement_drag.is_none());
    assert!(h.editor.ui.placement_tool.is_none());
    assert_eq!(
        h.editor
            .session
            .course
            .slide("s-intro")
            .unwrap()
            .objects
            .len(),
        before
    );
}

// ============================================================================
// P2
// ============================================================================

#[test]
fn a5_5_shape_preview_while_zoomed_and_panned() {
    for zoom in [2.0, 0.5] {
        let mut h = TestHarness::new(hello());
        h.editor.ui.zoom = crate::canvas::ZoomLevel::Custom(zoom);
        h.idle_frame();
        pick_shape(&mut h, ShapeKind::Ellipse);
        let (_, shapes) = drag_preview(
            &mut h,
            pos2(300.0, 200.0),
            vec2(120.0, 90.0),
            Modifiers::NONE,
        );
        let accent = coursecraft_render::palette(&h.ctx).accent;
        assert!(
            shapes.iter().any(
                |s| matches!(s, Shape::Mesh(m) if m.vertices.iter().any(|v| v.color == accent))
            )
        );
    }
}

#[test]
fn a5_6_shape_preview_on_layer_and_master_slide() {
    let mut h = TestHarness::new(hello());
    let about_pill = h
        .find_text_in("About", h.central_panel_rect())
        .expect("About pill");
    h.click_at(about_pill);
    h.idle_frame();
    let base_before = h
        .editor
        .session
        .course
        .slide("s-intro")
        .unwrap()
        .objects
        .len();
    let layer_before = h
        .editor
        .session
        .course
        .slide("s-intro")
        .unwrap()
        .layer("about")
        .unwrap()
        .objects
        .len();

    pick_shape(&mut h, ShapeKind::Cloud);
    let (_, shapes) = drag_preview(
        &mut h,
        pos2(300.0, 200.0),
        vec2(160.0, 120.0),
        Modifiers::NONE,
    );
    let accent = coursecraft_render::palette(&h.ctx).accent;
    assert!(
        shapes
            .iter()
            .any(|s| matches!(s, Shape::Mesh(m) if m.vertices.iter().any(|v| v.color == accent)))
    );

    let t = SlideTransform::fit(
        canvas_fit_rect(canvas_rect(h.central_panel_rect())),
        h.editor.session.course.slide_size,
    )
    .expect("t");
    release_at(&mut h, t.to_screen_pos(pos2(460.0, 320.0)), Modifiers::NONE);
    assert_eq!(
        h.editor
            .session
            .course
            .slide("s-intro")
            .unwrap()
            .objects
            .len(),
        base_before
    );
    assert_eq!(
        h.editor
            .session
            .course
            .slide("s-intro")
            .unwrap()
            .layer("about")
            .unwrap()
            .objects
            .len(),
        layer_before + 1
    );
}

#[test]
fn a5_7_snap_to_grid_active_during_shape_drag_preview() {
    let mut h = TestHarness::new(hello());
    h.editor
        .session
        .execute("view.set", &serde_json::json!({ "snap_to_grid": true }))
        .expect("snap_to_grid");
    h.idle_frame();
    pick_shape(&mut h, ShapeKind::Triangle);
    let (_, _) = drag_preview(
        &mut h,
        pos2(403.0, 217.0),
        vec2(173.0, 129.0),
        Modifiers::NONE,
    );
    let preview_frame = h.editor.ui.placement_drag.as_ref().expect("drag").frame;
    let t = SlideTransform::fit(
        canvas_fit_rect(canvas_rect(h.central_panel_rect())),
        h.editor.session.course.slide_size,
    )
    .expect("t");
    release_at(&mut h, t.to_screen_pos(pos2(576.0, 346.0)), Modifiers::NONE);
    let placed = h
        .editor
        .session
        .course
        .slide("s-intro")
        .unwrap()
        .objects
        .last()
        .unwrap();
    assert!((placed.frame.w - preview_frame.width()).abs() < 1.0);
    assert!((placed.frame.h - preview_frame.height()).abs() < 1.0);
}

#[test]
fn a5_8_light_theme_and_text_placement_tool_preview() {
    let mut h = TestHarness::new_with_theme(
        hello(),
        (1280.0, 900.0),
        coursecraft_render::ThemeMode::Light,
    );
    pick_shape(&mut h, ShapeKind::Heart);
    let (_, shapes) = drag_preview(
        &mut h,
        pos2(300.0, 200.0),
        vec2(140.0, 120.0),
        Modifiers::NONE,
    );
    let light_accent = coursecraft_render::palette(&h.ctx).accent;
    assert!(shapes.iter().any(
        |s| matches!(s, Shape::Mesh(m) if m.vertices.iter().any(|v| v.color == light_accent))
    ));

    // Text tool still draws rectangular preview
    h.editor.ui.placement_drag = None;
    h.editor.ui.placement_tool = Some(PlacementTool::TextBox);
    h.idle_frame();
    let (preview_rect, tshapes) = drag_preview(
        &mut h,
        pos2(300.0, 200.0),
        vec2(180.0, 60.0),
        Modifiers::NONE,
    );
    assert!(tshapes.iter().any(
        |s| matches!(s, Shape::Rect(r) if (r.rect.width() - preview_rect.width()).abs() < 6.0)
    ));
}

// ============================================================================
// P3
// ============================================================================

#[test]
fn a5_9_draw_tool_line_and_curved_connector_preview() {
    for tool in [crate::DrawTool::Line, crate::DrawTool::CurvedConnector] {
        let mut h = TestHarness::new(hello());
        h.editor.ui.placement_tool = Some(PlacementTool::Draw(tool));
        h.idle_frame();
        let before = h
            .editor
            .session
            .course
            .slide("s-intro")
            .unwrap()
            .objects
            .len();
        let (_, shapes) = drag_preview(
            &mut h,
            pos2(200.0, 200.0),
            vec2(300.0, 150.0),
            Modifiers::NONE,
        );
        assert!(
            shapes
                .iter()
                .any(|s| matches!(s, Shape::Path(_) | Shape::LineSegment { .. }))
        );
        let t = SlideTransform::fit(
            canvas_fit_rect(canvas_rect(h.central_panel_rect())),
            h.editor.session.course.slide_size,
        )
        .expect("t");
        release_at(&mut h, t.to_screen_pos(pos2(500.0, 350.0)), Modifiers::NONE);
        let slide = h.editor.session.course.slide("s-intro").unwrap();
        assert_eq!(slide.objects.len(), before + 1);
        assert!(matches!(
            slide.objects.last().unwrap().kind,
            ObjectKind::Path { .. }
        ));
    }
}
