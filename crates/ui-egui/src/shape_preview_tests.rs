//! Headless UI tests for shape placement drag preview (item A5).

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

fn pick_shape_from_gallery(harness: &mut TestHarness, kind: ShapeKind) {
    harness.idle_frame();
    let shapes_btn = harness
        .find_icon(coursecraft_render::Icon::Shapes)
        .expect("toolbar Shape button");
    harness.click_at(shapes_btn);
    harness.idle_frame();

    let search = harness
        .editor
        .ui
        .shape_gallery_search_rect
        .expect("search field");
    harness.set_field(search.center(), shape_label(kind));
    harness.idle_frame();

    let icon_rect = harness
        .editor
        .ui
        .shape_gallery_rects
        .iter()
        .find_map(|(candidate, rect)| (*candidate == kind).then_some(*rect))
        .unwrap_or_else(|| panic!("{kind:?} is not in the gallery"));
    harness.click_at(icon_rect.center());
    harness.idle_frame();
    assert_eq!(
        harness.editor.ui.placement_tool,
        Some(PlacementTool::Shape(kind))
    );
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

fn all_painted_shapes(clipped: &[ClippedShape]) -> Vec<Shape> {
    let mut out = Vec::new();
    for c in clipped {
        collect_shapes(&c.shape, &mut out);
    }
    out
}

fn drag_preview_shapes(
    harness: &mut TestHarness,
    start_slide: Pos2,
    delta_slide: egui::Vec2,
    modifiers: Modifiers,
) -> (Rect, Vec<Shape>) {
    let transform = SlideTransform::fit(
        canvas_fit_rect(canvas_rect(harness.central_panel_rect())),
        harness.editor.session.course.slide_size,
    )
    .expect("transform");
    let start = transform.to_screen_pos(start_slide);
    let end = transform.to_screen_pos(start_slide + delta_slide);

    if modifiers != Modifiers::NONE {
        harness.frame(vec![Event::ModifiersChanged(modifiers)]);
    }
    harness.frame(vec![Event::PointerMoved(start)]);
    harness.frame(vec![Event::PointerButton {
        pos: start,
        button: PointerButton::Primary,
        pressed: true,
        modifiers,
    }]);
    let clipped = harness.frame_shapes(vec![Event::PointerMoved(end)]);
    let preview_slide_rect = harness
        .editor
        .ui
        .placement_drag
        .as_ref()
        .expect("active placement drag")
        .frame;
    let preview_screen_rect = transform.to_screen_rect(preview_slide_rect);
    (preview_screen_rect, all_painted_shapes(&clipped))
}

#[test]
fn shape_drag_preview_draws_chosen_shape_not_a_plain_rectangle() {
    for kind in [
        ShapeKind::Ellipse,
        ShapeKind::Heart,
        ShapeKind::Star5,
        ShapeKind::ArrowRight,
        ShapeKind::Triangle,
        ShapeKind::RoundedRectangle,
    ] {
        let mut harness = TestHarness::new(hello());
        harness.editor.ui.snap_to_guides = false;
        pick_shape_from_gallery(&mut harness, kind);

        let (preview_rect, painted) = drag_preview_shapes(
            &mut harness,
            pos2(400.0, 220.0),
            vec2(180.0, 140.0),
            Modifiers::NONE,
        );
        let p = coursecraft_render::palette(&harness.ctx);

        // Must NOT paint the old plain-rectangle preview stroke/fill at preview_rect.
        let has_plain_rect_preview = painted.iter().any(|shape| match shape {
            Shape::Rect(r) => {
                (r.rect.min - preview_rect.min).length() < 2.0
                    && (r.rect.max - preview_rect.max).length() < 2.0
                    && (r.stroke.color == p.accent || r.fill.a() == 24)
            }
            _ => false,
        });
        assert!(
            !has_plain_rect_preview,
            "{kind:?} drag preview should not paint a plain rectangle"
        );

        // Must paint a Mesh outline in p.accent whose bounds match preview_rect.
        let has_shape_outline_mesh = painted.iter().any(|shape| match shape {
            Shape::Mesh(mesh) => {
                let bounds = mesh.calc_bounds();
                let matches_rect = (bounds.center() - preview_rect.center()).length() < 4.0
                    && (bounds.width() - preview_rect.width()).abs() < 6.0
                    && (bounds.height() - preview_rect.height()).abs() < 6.0;
                let has_accent = mesh.vertices.iter().any(|v| v.color == p.accent);
                matches_rect && has_accent && mesh.vertices.len() >= 6
            }
            _ => false,
        });
        assert!(
            has_shape_outline_mesh,
            "{kind:?} drag preview should paint the shape's outline mesh"
        );
    }
}

#[test]
fn shape_drag_preview_with_shift_keeps_proportions_and_places_shape() {
    let mut harness = TestHarness::new(hello());
    harness.editor.ui.snap_to_guides = false;
    pick_shape_from_gallery(&mut harness, ShapeKind::Heart);

    let transform = SlideTransform::fit(
        canvas_fit_rect(canvas_rect(harness.central_panel_rect())),
        harness.editor.session.course.slide_size,
    )
    .expect("transform");
    let start_slide = pos2(400.0, 220.0);
    let delta_slide = vec2(200.0, 80.0);
    let (preview_rect, _) =
        drag_preview_shapes(&mut harness, start_slide, delta_slide, Modifiers::SHIFT);

    // Heart is square by default, so Shift keeps 1:1 proportions.
    assert!(
        (preview_rect.width() - preview_rect.height()).abs() < 2.0,
        "Shift-drag preview should keep 1:1 proportions for Heart, got {}x{}",
        preview_rect.width(),
        preview_rect.height()
    );

    // Release completes placement of the Heart shape.
    let end = transform.to_screen_pos(start_slide + delta_slide);
    harness.frame(vec![Event::PointerButton {
        pos: end,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::SHIFT,
    }]);
    harness.frame(vec![Event::ModifiersChanged(Modifiers::NONE)]);

    let slide = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("slide");
    let placed = slide.objects.last().expect("placed object");
    assert!(matches!(
        placed.kind,
        ObjectKind::Shape {
            shape: ShapeKind::Heart,
            ..
        }
    ));

    // Undo removes the placed shape; redo restores it.
    let before_undo = slide.objects.len();
    harness.editor.session.undo().expect("undo");
    harness.idle_frame();
    assert_eq!(
        harness
            .editor
            .session
            .course
            .slide("s-intro")
            .expect("slide")
            .objects
            .len(),
        before_undo - 1
    );
    harness.editor.session.redo().expect("redo");
    harness.idle_frame();
    assert_eq!(
        harness
            .editor
            .session
            .course
            .slide("s-intro")
            .expect("slide")
            .objects
            .len(),
        before_undo
    );
}

#[test]
fn shape_drag_preview_works_in_negative_direction() {
    let mut harness = TestHarness::new(hello());
    harness.editor.ui.snap_to_guides = false;
    pick_shape_from_gallery(&mut harness, ShapeKind::Star5);

    let (preview_rect, painted) = drag_preview_shapes(
        &mut harness,
        pos2(600.0, 400.0),
        vec2(-160.0, -120.0),
        Modifiers::NONE,
    );
    let p = coursecraft_render::palette(&harness.ctx);
    let has_shape_outline_mesh = painted.iter().any(|shape| match shape {
        Shape::Mesh(mesh) => {
            let bounds = mesh.calc_bounds();
            let matches_rect = (bounds.center() - preview_rect.center()).length() < 4.0
                && (bounds.width() - preview_rect.width()).abs() < 6.0
                && (bounds.height() - preview_rect.height()).abs() < 6.0;
            let has_accent = mesh.vertices.iter().any(|v| v.color == p.accent);
            matches_rect && has_accent && mesh.vertices.len() > 16
        }
        _ => false,
    });
    assert!(
        has_shape_outline_mesh,
        "Star5 negative-direction drag preview should paint the star outline mesh"
    );
}
