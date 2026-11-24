//! The contextual tabs drawn in the editor (docs/ui/contextual-tabs.md).

use super::spec::{self, CONTEXT_TABS, Context, ContextOp};
use crate::test_harness::TestHarness;
use coursecraft_course::{Course, ObjectKind};

fn frames(h: &mut TestHarness, n: usize) {
    for _ in 0..n {
        h.idle_frame();
    }
}

fn harness(objects: serde_json::Value, question: bool) -> TestHarness {
    let mut slide = serde_json::json!({
        "id": "sl1",
        "title": "Slide 1",
        "duration_ms": 5000,
        "objects": objects,
    });
    if question {
        slide["question"] = serde_json::json!({
            "kind": "multiple_choice",
            "prompt": "Pick one",
            "choices": [
                { "id": "a", "text": "A", "correct": true },
                { "id": "b", "text": "B", "correct": false }
            ]
        });
    }
    let json = serde_json::json!({
        "schema_version": 1,
        "id": "ctx",
        "title": "Contextual tabs",
        "slide_size": { "w": 1000, "h": 600 },
        "scenes": [{ "id": "sc1", "title": "Scene 1", "slides": [slide] }]
    });
    let course = Course::from_json(&json.to_string()).expect("valid course");
    let mut h = TestHarness::new_with_size(course, (2400.0, 900.0));
    h.editor.ui.ribbon.visible = true;
    h.editor.selection = crate::Selection::new("sl1");
    frames(&mut h, 2);
    h
}

fn shape() -> serde_json::Value {
    serde_json::json!([{
        "id": "sh1",
        "kind": { "type": "shape", "shape": "rectangle", "fill": "#112233" },
        "frame": { "x": 100, "y": 100, "w": 200, "h": 120 }
    }])
}

#[test]
fn every_context_has_its_tabs_and_they_end_with_publish() {
    for context in [
        Context::Drawing,
        Context::Button,
        Context::Picture,
        Context::Video,
        Context::Audio,
        Context::Table,
        Context::Question,
    ] {
        let tabs: Vec<_> = CONTEXT_TABS
            .iter()
            .filter(|t| t.context == context)
            .collect();
        assert!(!tabs.is_empty(), "{context:?} has no tab");
        for t in tabs {
            assert_eq!(
                t.tab.groups.last().map(|g| g.label),
                Some("Publish"),
                "{} must end with Publish",
                t.tab.id
            );
            assert!(spec::tab(t.tab.id).is_some());
        }
    }
}

#[test]
fn selecting_a_shape_shows_drawing_tools_without_switching() {
    let mut h = harness(shape(), false);
    assert!(h.find_text("Drawing Tools").is_none());
    h.editor.selection.object = Some("sh1".into());
    frames(&mut h, 2);
    assert!(h.find_text("Drawing Tools").is_some(), "caption drawn");
    assert_eq!(h.editor.ui.ribbon.tab, "home", "selection keeps the tab");
    let caption = h.find_text("Drawing Tools").expect("caption");
    let format = h
        .find_texts("Format")
        .into_iter()
        .find(|p| (p.y - caption.y).abs() < 4.0 && p.x > caption.x)
        .expect("Format tab");
    h.click_at(format);
    frames(&mut h, 2);
    assert_eq!(h.editor.ui.ribbon.tab, "drawing_format");
    for text in [
        "Insert Shapes",
        "Shape Styles",
        "Arrange and Align",
        "Width",
    ] {
        assert!(h.find_text(text).is_some(), "{text:?} is not drawn");
    }
    // Deselecting hides the tab and goes back to Home.
    h.editor.selection.object = None;
    frames(&mut h, 2);
    assert_eq!(h.editor.ui.ribbon.tab, "home");
    assert!(h.find_text("Drawing Tools").is_none());
}

#[test]
fn shape_fill_opens_the_shape_style_panel() {
    let mut h = harness(shape(), false);
    h.editor.ui.dock.enabled = true;
    h.editor.selection.object = Some("sh1".into());
    h.editor.ui.ribbon.tab = "drawing_format";
    frames(&mut h, 3);
    let fill = h.find_text("Shape Fill").expect("Shape Fill");
    h.click_at(fill);
    frames(&mut h, 3);
    assert!(crate::panels::is_shown(
        &h.editor,
        crate::dock::Panel::ShapeStyle
    ));
}

#[test]
fn the_width_field_sets_the_frame_in_one_step() {
    let mut h = harness(shape(), false);
    h.editor.selection.object = Some("sh1".into());
    h.editor.ui.ribbon.tab = "drawing_format";
    frames(&mut h, 3);
    let width = h.find_text("Width").expect("Width label");
    let field = h
        .find_text_containing("200")
        .filter(|p| (p.y - width.y).abs() < 4.0 && p.x > width.x)
        .expect("width field");
    h.set_field(field, "320");
    frames(&mut h, 2);
    let slide = h.editor.session.course.slide("sl1").expect("slide");
    let frame = slide.object("sh1").expect("shape").0.frame;
    assert!((frame.w - 320.0).abs() < 0.01, "{frame:?}");
    assert!((frame.h - 120.0).abs() < 0.01, "{frame:?}");
}

#[test]
fn an_inserted_table_opens_table_design_and_layout_adds_a_row() {
    let mut h = harness(serde_json::json!([]), false);
    crate::toolbar::insert_table(&mut h.editor);
    frames(&mut h, 3);
    assert_eq!(h.editor.ui.ribbon.tab, "table_design", "just inserted");
    assert!(h.find_text("Table Tools").is_some());
    let rows = |h: &TestHarness| {
        let slide = h.editor.session.course.slide("sl1").expect("slide");
        slide
            .objects
            .iter()
            .find_map(|o| match &o.kind {
                ObjectKind::Table { rows, .. } => Some(rows.len()),
                _ => None,
            })
            .expect("a table")
    };
    let before = rows(&h);
    let layout = h.find_text("Layout").expect("Layout tab");
    h.click_at(layout);
    frames(&mut h, 2);
    let below = h.find_text("Insert Below").expect("Insert Below");
    h.click_at(below);
    frames(&mut h, 2);
    assert_eq!(rows(&h), before + 1);
}

#[test]
fn a_question_slide_shows_question_tools_and_shuffle_toggles() {
    let mut h = harness(serde_json::json!([]), true);
    assert!(h.find_text("Question Tools").is_some());
    let shuffled = |h: &TestHarness| {
        h.editor
            .session
            .course
            .slide("sl1")
            .and_then(|s| s.question.as_ref())
            .is_some_and(|q| q.shuffle)
    };
    let before = shuffled(&h);
    assert_eq!(
        super::contextual::available(&h.editor, ContextOp::QuestionShuffle),
        Ok(())
    );
    super::contextual::run(&mut h.editor, ContextOp::QuestionShuffle, &mut Vec::new());
    assert_ne!(shuffled(&h), before);
}

#[test]
fn picture_controls_need_a_picture() {
    let h = harness(shape(), false);
    assert!(super::contextual::available(&h.editor, ContextOp::Crop).is_err());
    assert!(super::contextual::available(&h.editor, ContextOp::ReplaceMedia).is_err());
}
