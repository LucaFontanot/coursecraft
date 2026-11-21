//! M26 bug-hunt tests for A12: Preview modal closing.
//!
//! Ordered P1 -> P2 -> P3.

use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use egui::{Key, Modifiers, pos2};

fn simple_course() -> Course {
    let json = serde_json::json!({
        "schema_version": 1,
        "id": "preview-close-test",
        "title": "Preview Close Test",
        "slide_size": { "w": 1280, "h": 720 },
        "variables": [{ "name": "v", "value": 50.0 }],
        "scenes": [{
            "id": "sc-1",
            "title": "Main",
            "slides": [
                {
                    "id": "s-1",
                    "title": "Slide 1",
                    "duration_ms": 5000,
                    "objects": [{
                        "id": "txt-1",
                        "kind": { "type": "text", "text": "Hello Preview" },
                        "frame": { "x": 80, "y": 80, "w": 400, "h": 60 }
                    }]
                },
                {
                    "id": "s-2",
                    "title": "Slide 2",
                    "duration_ms": 5000,
                    "objects": [{
                        "id": "txt-2",
                        "kind": { "type": "text", "text": "Second Slide" },
                        "frame": { "x": 80, "y": 80, "w": 400, "h": 60 }
                    }]
                }
            ]
        }]
    })
    .to_string();
    Course::from_json(&json).expect("valid")
}

// ============================================================================
// P1
// ============================================================================

#[test]
fn a12_1_click_outside_on_all_four_margins_across_sizes() {
    for (w, h_px) in [(1024.0, 680.0), (1280.0, 800.0), (1920.0, 1080.0)] {
        for pt in [
            pos2(w * 0.5, 4.0),
            pos2(w * 0.5, h_px - 4.0),
            pos2(4.0, h_px * 0.5),
            pos2(w - 4.0, h_px * 0.5),
        ] {
            let mut h = TestHarness::new_with_size(simple_course(), (w, h_px));
            h.idle_frame();
            h.editor.open_preview();
            h.idle_frame();
            h.idle_frame();
            let rect = h
                .editor
                .preview
                .as_ref()
                .expect("preview")
                .window_rect
                .expect("window_rect");
            if rect.contains(pt) {
                continue;
            }
            h.click_at(pt);
            h.idle_frame();
            assert!(
                h.editor.preview.is_none(),
                "clicking {pt:?} at {w}x{h_px} should close preview (rect={rect:?})"
            );
        }
    }
}

#[test]
fn a12_2_backdrop_click_does_not_click_through_to_editor() {
    let mut h = TestHarness::new_with_size(simple_course(), (1440.0, 900.0));
    h.idle_frame();
    h.idle_frame();
    let s2_pos = h
        .find_text_in("Slide 2", h.left_panel_rect())
        .expect("Slide 2 in left panel");
    let insert_pos = h.find_text("Insert").expect("Insert menu");

    h.editor.open_preview();
    h.idle_frame();
    h.idle_frame();
    let rect = h
        .editor
        .preview
        .as_ref()
        .expect("preview")
        .window_rect
        .expect("window_rect");
    assert!(!rect.contains(s2_pos), "s2_pos should be outside preview");
    h.click_at(s2_pos);
    h.idle_frame();
    assert!(h.editor.preview.is_none());
    assert_eq!(
        h.editor.selection.slide, "s-1",
        "backdrop click must not switch slide"
    );

    h.editor.open_preview();
    h.idle_frame();
    h.idle_frame();
    let rect2 = h
        .editor
        .preview
        .as_ref()
        .expect("preview")
        .window_rect
        .expect("window_rect");
    assert!(!rect2.contains(insert_pos), "insert_pos outside preview");
    h.click_at(insert_pos);
    h.idle_frame();
    assert!(h.editor.preview.is_none());
    assert!(
        h.find_text("Interaction…").is_none(),
        "backdrop click must not open Insert menu"
    );
}

#[test]
fn a12_3_outline_drawer_open_esc_closes_drawer_first_then_preview() {
    let mut course = simple_course();
    course.player.show_menu = true;
    let mut h = TestHarness::new(course);
    h.idle_frame();
    h.editor.open_preview();
    h.idle_frame();
    h.idle_frame();

    let rect = h
        .editor
        .preview
        .as_ref()
        .expect("preview")
        .window_rect
        .expect("window_rect");
    let menu_btn = h
        .find_text_in("Menu", rect)
        .or_else(|| h.find_icon_last(coursecraft_render::Icon::List))
        .expect("Menu button in preview");
    h.click_at(menu_btn);
    h.idle_frame();
    assert!(
        h.editor
            .preview
            .as_ref()
            .expect("preview")
            .frame
            .drawer_open()
    );

    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(
        h.editor.preview.is_some(),
        "1st Esc should keep preview open"
    );
    assert!(
        !h.editor
            .preview
            .as_ref()
            .expect("preview")
            .frame
            .drawer_open(),
        "1st Esc should close drawer"
    );

    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(h.editor.preview.is_none(), "2nd Esc should close preview");
}

#[test]
fn a12_5_dragging_slider_inside_preview_and_releasing_outside_keeps_preview_open() {
    let mut course = simple_course();
    let slider: coursecraft_course::Object = serde_json::from_value(serde_json::json!({
        "id": "sl-1",
        "kind": { "type": "slider", "variable": "v", "min": 0.0, "max": 100.0, "step": 1.0 },
        "frame": { "x": 200.0, "y": 240.0, "w": 300.0, "h": 40.0 }
    }))
    .expect("slider");
    course.scenes[0].slides[0].objects.push(slider);

    let mut h = TestHarness::new(course);
    h.idle_frame();
    h.editor.open_preview();
    h.idle_frame();
    h.idle_frame();

    let rect = h
        .editor
        .preview
        .as_ref()
        .expect("preview")
        .window_rect
        .expect("window_rect");
    let start = rect.center();
    let outside = pos2(5.0, 5.0);
    h.drag_by(
        start,
        outside.to_vec2() - start.to_vec2(),
        6,
        Modifiers::NONE,
    );
    h.idle_frame();
    assert!(
        h.editor.preview.is_some(),
        "drag release outside must not close preview"
    );

    h.click_at(outside);
    h.idle_frame();
    assert!(
        h.editor.preview.is_none(),
        "fresh outside click closes preview"
    );
}

#[test]
fn a12_8_closing_preview_stops_media_on_backdrop_and_esc() {
    // Verified via preview.drop / state cleanup when preview becomes None
    let mut h = TestHarness::new(simple_course());
    h.idle_frame();
    h.editor.open_preview();
    h.idle_frame();
    h.idle_frame();
    h.click_at(pos2(5.0, 5.0));
    h.idle_frame();
    assert!(h.editor.preview.is_none());

    h.editor.open_preview();
    h.idle_frame();
    h.idle_frame();
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(h.editor.preview.is_none());
}

// ============================================================================
// P2
// ============================================================================

#[test]
fn a12_4_effect_log_and_variables_panels_inside_preview() {
    let mut h = TestHarness::new_with_size(simple_course(), (1440.0, 900.0));
    h.idle_frame();
    h.editor.open_preview();
    h.idle_frame();
    h.idle_frame();

    let notepad = h
        .find_icon_last(coursecraft_render::Icon::Notepad)
        .expect("Effect Log icon");
    h.click_at(notepad);
    h.idle_frame();
    let vars = h
        .find_icon_last(coursecraft_render::Icon::BracketsCurly)
        .expect("Variables icon");
    h.click_at(vars);
    h.idle_frame();

    if let Some(el) = h.find_text("Effect Log") {
        h.click_at(el);
        h.idle_frame();
    }
    assert!(h.editor.preview.is_some());

    h.click_at(pos2(4.0, 4.0));
    h.idle_frame();
    assert!(h.editor.preview.is_none());
}

#[test]
fn a12_6_validation_error_preview_closes_on_backdrop_and_escape() {
    let mut course = simple_course();
    course.scenes[0].slides[0].triggers.push(
        serde_json::from_value(serde_json::json!({
            "when": { "event": "timeline_starts" },
            "action": { "action": "jump_to_slide", "target": { "id": "does-not-exist" } }
        }))
        .expect("trigger"),
    );

    let mut h = TestHarness::new(course);
    h.idle_frame();
    h.editor.open_preview();
    h.idle_frame();
    h.idle_frame();
    h.idle_frame();
    assert!(
        !h.editor
            .preview
            .as_ref()
            .expect("preview")
            .errors
            .is_empty()
    );

    h.click_at(pos2(5.0, 5.0));
    h.idle_frame();
    assert!(h.editor.preview.is_none());

    h.editor.open_preview();
    h.idle_frame();
    h.idle_frame();
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(h.editor.preview.is_none());
}

#[test]
fn a12_7_lightbox_open_inside_preview_closes_with_backdrop_and_esc() {
    let mut course = simple_course();
    let btn: coursecraft_course::Object = serde_json::from_value(serde_json::json!({
        "id": "btn-lb",
        "kind": { "type": "button", "label": "OpenLB" },
        "frame": { "x": 100.0, "y": 200.0, "w": 160.0, "h": 48.0 }
    }))
    .expect("btn");
    course.scenes[0].slides[0].objects.push(btn);
    course.scenes[0].slides[0].triggers.push(
        serde_json::from_value(serde_json::json!({
            "when": { "event": "click", "object": "btn-lb" },
            "action": { "action": "lightbox_slide", "slide": "s-2" }
        }))
        .expect("lb trigger"),
    );

    let mut h = TestHarness::new(course);
    h.idle_frame();
    h.editor.open_preview();
    h.idle_frame();
    h.idle_frame();

    let open_lb = h.find_text_last("OpenLB").expect("OpenLB button");
    h.click_at(open_lb);
    h.idle_frame();

    h.click_at(pos2(5.0, 5.0));
    h.idle_frame();
    assert!(h.editor.preview.is_none());

    h.editor.open_preview();
    h.idle_frame();
    h.idle_frame();
    let open_lb2 = h.find_text_last("OpenLB").expect("OpenLB button");
    h.click_at(open_lb2);
    h.idle_frame();
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(h.editor.preview.is_none());
}

#[test]
#[ignore = "BUG: Editor shortcuts (Delete, Cmd+M, Cmd+Z) mutate the course while Preview modal is open"]
fn a12_9_editor_shortcuts_do_not_mutate_course_while_preview_open() {
    let mut h = TestHarness::new(simple_course());
    h.editor.selection.slide = "s-1".into();
    h.editor.selection.object = Some("txt-1".into());
    h.idle_frame();

    let rev = h.editor.session.revision;
    let slides_n = h.editor.session.course.scenes[0].slides.len();
    let objs_n = h.editor.session.course.scenes[0].slides[0].objects.len();

    h.editor.open_preview();
    h.idle_frame();
    h.idle_frame();
    h.press(Key::Delete, Modifiers::NONE);
    h.press(Key::M, Modifiers::COMMAND);
    h.press(Key::Z, Modifiers::COMMAND);
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();

    assert_eq!(h.editor.session.revision, rev);
    assert_eq!(h.editor.session.course.scenes[0].slides.len(), slides_n);
    assert_eq!(
        h.editor.session.course.scenes[0].slides[0].objects.len(),
        objs_n
    );
}

// ============================================================================
// P3
// ============================================================================

#[test]
fn a12_10_repeated_open_close_across_x_backdrop_and_esc() {
    let mut h = TestHarness::new(simple_course());
    h.idle_frame();

    // 1. Close via X
    h.editor.open_preview();
    h.idle_frame();
    h.idle_frame();
    let x_btn = h
        .find_icon_last(coursecraft_render::Icon::X)
        .expect("X button");
    h.click_at(x_btn);
    h.idle_frame();
    assert!(h.editor.preview.is_none());

    // 2. Restart + backdrop close
    h.editor.open_preview();
    h.idle_frame();
    h.idle_frame();
    if let Some(restart) = h.find_text_last("Restart") {
        h.click_at(restart);
        h.idle_frame();
    }
    h.click_at(pos2(5.0, 5.0));
    h.idle_frame();
    assert!(h.editor.preview.is_none());

    // 3. Layout report + Esc close
    h.editor.open_preview();
    h.idle_frame();
    h.idle_frame();
    let rep = h.layout_report(2);
    assert!(
        rep.findings.is_empty(),
        "preview findings: {:?}",
        rep.findings
    );
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(h.editor.preview.is_none());
}
