//! M29 Step s6 tests: Format inspector routing (`U-09`), override dots/reset & clear toggles (`U-10`),
//! inline text edit inside a state (`U-12`, `SUS-12`), image picker in Edit-State Mode (`U-13`),
//! and drag-scrubbing perf guard (`PERF-03`).

use crate::guard_counters as gc;
use crate::test_harness::TestHarness;
use crate::{EditorRequest, InspectorTab, PendingImagePick};
use coursecraft_course::{Course, ObjectKind, ShapeKind, StateDef};
use coursecraft_render::Icon;
use egui::{Event, Key, Modifiers, PointerButton, pos2, vec2};

fn png_bytes(val: u8) -> Vec<u8> {
    let pixels = vec![val; 4 * 4 * 4];
    coursecraft_media::encode_png_rgba(4, 4, &pixels).expect("png")
}

fn s6_course() -> Course {
    let raw = serde_json::json!({
        "schema_version": 1,
        "id": "c-m29-s6",
        "title": "M29 S6 Test",
        "slide_size": { "w": 1280, "h": 720 },
        "theme": {
            "accent": "#1166CC",
            "text": "#222222",
            "background": "#F5F5F5",
            "font": "Inter"
        },
        "scenes": [{
            "id": "scene1",
            "title": "Main",
            "slides": [{
                "id": "s1",
                "title": "Slide 1",
                "objects": [
                    {
                        "id": "shape1",
                        "name": "Hero Card",
                        "frame": { "x": 100.0, "y": 100.0, "w": 240.0, "h": 140.0 },
                        "shadow": { "color": "#00000040", "offset_x": 2.0, "offset_y": 4.0, "blur": 6.0 },
                        "glow": { "color": "#1166CC80", "size": 8.0 },
                        "kind": {
                            "type": "shape",
                            "shape": "rectangle",
                            "fill": "#336699",
                            "outline": { "color": "#111111", "width": 2.0 },
                            "text": {
                                "text": "Normal Card",
                                "style": { "font": "Inter", "size": 16.0, "color": "#FFFFFF" }
                            }
                        },
                        "states": [
                            { "name": "hover" },
                            { "name": "selected" }
                        ]
                    },
                    {
                        "id": "btn1",
                        "name": "Action Button",
                        "frame": { "x": 400.0, "y": 100.0, "w": 200.0, "h": 56.0 },
                        "kind": { "type": "button", "label": "Click Me", "fill": "#225588" },
                        "states": [
                            { "name": "selected" }
                        ]
                    },
                    {
                        "id": "img1",
                        "name": "Picture 1",
                        "frame": { "x": 100.0, "y": 300.0, "w": 200.0, "h": 200.0 },
                        "kind": { "type": "image", "src": "base.png" },
                        "states": [
                            { "name": "hover" }
                        ]
                    }
                ]
            }]
        }]
    });
    serde_json::from_value(raw).expect("valid course")
}

fn enter_edit_state(h: &mut TestHarness, object_id: &str, state_name: &str) {
    h.editor.selection.set_many([object_id.to_owned()]);
    h.editor.ui.inspector_tab = InspectorTab::Format;
    h.editor.enter_edit_state("s1", object_id, state_name);
    h.idle_frame();
}

fn get_state<'a>(h: &'a TestHarness, object_id: &str, state_name: &str) -> &'a StateDef {
    let (obj, _) = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object(object_id)
        .unwrap();
    obj.states
        .iter()
        .find(|s| s.name == state_name)
        .expect("state exists")
}

#[test]
fn u09_format_inspector_routes_shape_and_effect_overrides_to_patch_state() {
    let mut h = TestHarness::new(s6_course());
    enter_edit_state(&mut h, "shape1", "hover");

    let base_before = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object("shape1")
        .unwrap()
        .0
        .clone();

    // Change Opacity slider in Format panel via drag on opacity_rect.
    let opacity_rect = h
        .editor
        .ui
        .properties
        .opacity_rect
        .expect("opacity slider rect must be recorded");
    h.drag_by(
        opacity_rect.center(),
        vec2(-opacity_rect.width() * 0.25, 0.0),
        4,
        Modifiers::NONE,
    );
    h.idle_frame();

    let hover = get_state(&h, "shape1", "hover");
    assert!(
        hover.opacity.is_some(),
        "changing Opacity in Edit-State Mode must patch state.opacity"
    );

    // Scroll Format inspector until Rotation is visible in the viewport.
    let inspector = h.right_panel_rect();
    for _ in 0..8 {
        if h.find_text_in("Rotation", inspector).is_some() {
            break;
        }
        h.scroll_over(inspector.center(), -200.0);
        h.idle_frame();
    }

    // Change Rotation via rotation_rect.
    let rot_rect = h
        .editor
        .ui
        .properties
        .rotation_rect
        .expect("rotation DragValue rect must be recorded");
    h.drag_by(rot_rect.center(), vec2(30.0, 0.0), 4, Modifiers::NONE);
    h.idle_frame();

    let hover = get_state(&h, "shape1", "hover");
    assert!(
        hover.rotation.is_some(),
        "changing Rotation in Edit-State Mode must patch state.rotation"
    );

    for _ in 0..4 {
        if h.find_text_in("Flip", inspector).is_some() {
            break;
        }
        h.scroll_over(inspector.center(), -100.0);
        h.idle_frame();
    }

    // Click Flip horizontal icon.
    let flip_h = h
        .find_icon_last(Icon::ArrowsOut)
        .expect("Flip horizontal icon not found");
    h.click_at(flip_h);
    h.idle_frame();

    let hover = get_state(&h, "shape1", "hover");
    assert_eq!(
        hover.flip_h,
        Some(true),
        "Flip horizontal in Edit-State Mode must patch state.flip_h"
    );

    // Base object properties (opacity, rotation, flip_h) must be untouched.
    let base_after = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object("shape1")
        .unwrap()
        .0
        .clone();
    assert!((base_after.opacity - base_before.opacity).abs() < 1e-4);
    assert!((base_after.rotation - base_before.rotation).abs() < 1e-4);
    assert_eq!(base_after.flip_h, base_before.flip_h);

    // Undo reverts flip_h in 1 step.
    h.press(Key::Z, Modifiers::COMMAND);
    h.idle_frame();
    let hover = get_state(&h, "shape1", "hover");
    assert_eq!(
        hover.flip_h, None,
        "Ctrl+Z must revert flip_h override in 1 step"
    );
}

#[test]
fn u09_format_inspector_routes_image_shape_to_patch_state() {
    let mut h = TestHarness::new(s6_course());
    h.editor.set_media(vec![
        ("base.png".to_owned(), std::sync::Arc::from(png_bytes(180))),
        ("alt.png".to_owned(), std::sync::Arc::from(png_bytes(90))),
    ]);
    enter_edit_state(&mut h, "img1", "hover");

    // Image shape combo should be present in Image Format inspector.
    let shape_combo = h
        .find_text("Rectangle")
        .expect("Image shape combo should show Rectangle by default");
    h.click_at(shape_combo);
    h.idle_frame();
    let ellipse = h
        .find_text("Ellipse")
        .expect("Ellipse option in Image shape combo");
    h.click_at(ellipse);
    h.idle_frame();

    let hover = get_state(&h, "img1", "hover");
    assert_eq!(
        hover.image_shape,
        Some(ShapeKind::Ellipse),
        "selecting Ellipse in Image shape combo in Edit-State Mode must patch state.image_shape"
    );
    let (img_obj, _) = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object("img1")
        .unwrap();
    if let ObjectKind::Image { shape, .. } = &img_obj.kind {
        assert_eq!(*shape, None, "base image shape must remain untouched");
    } else {
        panic!("expected Image");
    }
}

#[test]
fn u10_override_dots_reset_buttons_and_clear_toggles_in_edit_state_mode() {
    let mut h = TestHarness::new(s6_course());
    enter_edit_state(&mut h, "shape1", "hover");

    // Scroll Format inspector until Effects (Shadow / Glow) are visible in the viewport.
    let inspector = h.right_panel_rect();
    for _ in 0..10 {
        if h.find_text_in("Shadow", inspector).is_some() {
            break;
        }
        h.scroll_over(inspector.center(), -200.0);
        h.idle_frame();
    }

    // Normal has shadow, glow, and outline. Toggling Shadow OFF in Edit-State Mode must set clear_shadow = true.
    let shadow_label = h
        .find_text_in("Shadow", inspector)
        .expect("Shadow row in Format inspector");
    // Click the checkbox to the right of the Shadow label.
    h.click_at(pos2(shadow_label.x + 110.0, shadow_label.y));
    h.idle_frame();

    let hover = get_state(&h, "shape1", "hover");
    assert!(
        hover.clear_shadow,
        "toggling Shadow OFF when Normal has shadow must set clear_shadow = true"
    );
    assert!(hover.shadow.is_none());

    // An override reset icon ("Reset Shadow to Normal") must now appear on the Shadow row.
    let shadow_row_y = h
        .find_text_in("Shadow", inspector)
        .expect("Shadow row still visible")
        .y;
    let glyph = Icon::ArrowCounterClockwise.codepoint().to_string();
    let reset_shadow = h
        .find_texts(&glyph)
        .into_iter()
        .find(|p| inspector.contains(*p) && (p.y - shadow_row_y).abs() < 16.0)
        .expect("Reset icon button must be painted on the Shadow row");
    h.click_at(reset_shadow);
    h.idle_frame();

    let hover = get_state(&h, "shape1", "hover");
    assert!(
        !hover.clear_shadow && hover.shadow.is_none(),
        "clicking Reset Shadow to Normal must restore Inherit"
    );
}

#[test]
fn u12_inline_text_edit_in_edit_state_mode_patches_state_and_undoes_cleanly() {
    let mut h = TestHarness::new(s6_course());
    enter_edit_state(&mut h, "shape1", "selected");

    let slide_rect = h.editor.ui.last_slide_rect.expect("slide rect");
    let shape1_screen = pos2(
        slide_rect.min.x + slide_rect.width() * (220.0 / 1280.0),
        slide_rect.min.y + slide_rect.height() * (170.0 / 720.0),
    );
    h.double_click_at(shape1_screen);
    assert!(
        h.editor.ui.inline_edit.is_some(),
        "double-clicking target shape in Edit-State Mode must open inline edit"
    );

    if let Some(edit) = h.editor.ui.inline_edit.as_mut() {
        edit.buffer = "Tadah!".to_owned();
    }
    crate::canvas::commit_inline_edit(&mut h.editor);
    h.idle_frame();

    let selected = get_state(&h, "shape1", "selected");
    assert_eq!(
        selected.text.as_deref(),
        Some("Tadah!"),
        "committing inline text edit in Edit-State Mode must update state.text"
    );

    // Base object text must be untouched.
    let (obj, _) = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object("shape1")
        .unwrap();
    if let ObjectKind::Shape { text: Some(st), .. } = &obj.kind {
        assert_eq!(
            st.text, "Normal Card",
            "base shape text must remain untouched"
        );
    } else {
        panic!("expected Shape with text");
    }

    // Ctrl+Z reverts state.text.
    h.press(Key::Z, Modifiers::COMMAND);
    h.idle_frame();
    let selected = get_state(&h, "shape1", "selected");
    assert_eq!(
        selected.text, None,
        "Cmd+Z must revert inline text override on state"
    );
}

#[test]
fn u13_image_picker_in_edit_state_mode_sets_clears_and_reuses_state_src() {
    let mut h = TestHarness::new(s6_course());
    h.editor.set_media(vec![
        ("base.png".to_owned(), std::sync::Arc::from(png_bytes(180))),
        (
            "proj-alt.png".to_owned(),
            std::sync::Arc::from(png_bytes(90)),
        ),
    ]);
    enter_edit_state(&mut h, "img1", "hover");

    let inspector = egui::Rect::from_min_max(pos2(980.0, 0.0), pos2(1280.0, 720.0));
    let choose = h
        .find_text_in("Choose image…", inspector)
        .expect("Choose image… button in Image Format inspector during Edit-State Mode");
    let reqs = h.click_at_requests(choose);
    assert_eq!(reqs, vec![EditorRequest::PickImage]);
    assert_eq!(
        h.editor.pending_image_pick,
        Some(PendingImagePick::StateImage {
            slide: "s1".to_owned(),
            object: "img1".to_owned(),
            state: "hover".to_owned(),
        })
    );

    // Import an image: must set hover.src and leave base img1.src untouched.
    h.editor
        .import_image("picked-hover.png", &png_bytes(42))
        .expect("import_image succeeds");
    h.idle_frame();

    let hover = get_state(&h, "img1", "hover");
    assert_eq!(hover.src.as_deref(), Some("picked-hover.png"));
    let (img_obj, _) = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object("img1")
        .unwrap();
    if let ObjectKind::Image { src, .. } = &img_obj.kind {
        assert_eq!(src, "base.png");
    } else {
        panic!("expected Image");
    }

    // Clear resets hover.src to None.
    let clear_btn = h
        .find_text_in("Clear", inspector)
        .expect("Clear button for state image");
    h.click_at(clear_btn);
    h.idle_frame();
    assert_eq!(get_state(&h, "img1", "hover").src, None);

    // From project… reuses proj-alt.png without duplicating media bytes.
    let media_count_before = h.editor.media.len();
    let from_proj = h
        .find_text_in("From project…", inspector)
        .expect("From project… menu in Image Format inspector");
    h.click_at(from_proj);
    h.idle_frame();
    let item = h
        .find_text("proj-alt.png")
        .expect("proj-alt.png in From project… menu");
    h.click_at(item);
    h.idle_frame();
    assert_eq!(
        get_state(&h, "img1", "hover").src.as_deref(),
        Some("proj-alt.png")
    );
    assert_eq!(h.editor.media.len(), media_count_before);
}

#[test]
fn perf03_drag_scrubbing_in_edit_state_mode_defers_audit_and_bounds_work() {
    let mut h = TestHarness::new(s6_course());
    enter_edit_state(&mut h, "shape1", "hover");
    // Flush initial audit/validation.
    h.advance_time(0.5);
    h.idle_frame();
    gc::reset();

    let opacity_rect = h
        .editor
        .ui
        .properties
        .opacity_rect
        .expect("opacity slider rect");
    let start = opacity_rect.center();
    h.frame(vec![
        Event::PointerMoved(start),
        Event::PointerButton {
            pos: start,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
    ]);
    for step in 1..=20 {
        let p = pos2(
            opacity_rect.min.x + opacity_rect.width() * (step as f32 / 25.0),
            opacity_rect.center().y,
        );
        h.advance_time(0.016);
        h.frame(vec![Event::PointerMoved(p)]);
    }

    assert_eq!(
        gc::audit_runs(),
        0,
        "audit must be deferred during 20 drag-scrubbing frames"
    );
    assert!(
        gc::validation_runs() <= 1,
        "validation must not run per drag frame, ran {}",
        gc::validation_runs()
    );

    // Release mouse to commit once.
    let end = pos2(
        opacity_rect.min.x + opacity_rect.width() * (20.0 / 25.0),
        opacity_rect.center().y,
    );
    h.frame(vec![Event::PointerButton {
        pos: end,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    }]);
    h.idle_frame();
    assert!(
        gc::validation_runs() <= 1,
        "releasing drag commits at most once"
    );
    assert_eq!(
        gc::audit_runs(),
        0,
        "audit remains deferred until 300ms quiet"
    );
}
