//! Tests for starter templates, the welcome screen template cards, and saved lock/hide.

use coursecraft_course::{Course, Severity};

use crate::test_harness::TestHarness;
use crate::welcome::{TemplateId, template_course};

#[test]
fn all_starter_templates_load_and_validate() {
    for &id in TemplateId::ALL {
        let course = template_course(id).unwrap_or_else(|e| panic!("{}: {e}", id.name()));
        let issues = course.validate();
        let errors: Vec<_> = issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .collect();
        assert!(
            errors.is_empty(),
            "{} template has validation errors: {errors:?}",
            id.name()
        );
        if id == TemplateId::Blank {
            assert!(
                issues.is_empty(),
                "Blank template must have no warnings either: {issues:?}"
            );
        }
    }
}

#[test]
fn welcome_screen_shows_four_template_cards_and_clicking_opens_course() {
    let mut h = TestHarness::new_with_size(
        template_course(TemplateId::Blank).expect("blank"),
        (1280.0, 900.0),
    );
    h.editor.set_welcome(true);
    h.idle_frame();

    assert!(
        h.find_text("Start from a template").is_some(),
        "welcome screen should show the 'Start from a template' heading"
    );
    for &id in TemplateId::ALL {
        assert!(
            h.find_text(id.name()).is_some(),
            "welcome screen should show template card '{}'",
            id.name()
        );
    }

    let card = h
        .find_text(TemplateId::QuickKnowledgeCheck.name())
        .expect("Quick Knowledge Check card");
    h.click_at(card);
    h.idle_frame();

    assert!(!h.editor.is_welcome(), "clicking a template closes welcome");
    assert_eq!(h.editor.session.course.id, "quick-knowledge-check");
    assert!(h.editor.session.course.quiz.is_some());

    // Open welcome again and click Branching Scenario.
    h.editor.set_welcome(true);
    h.idle_frame();
    let card = h
        .find_text(TemplateId::BranchingScenario.name())
        .expect("Branching Scenario card");
    h.click_at(card);
    h.idle_frame();

    assert!(!h.editor.is_welcome());
    assert_eq!(h.editor.session.course.id, "branching-scenario");
}

#[test]
fn lock_and_hide_survive_save_reload_and_undo_in_editor() {
    let course = Course::from_json(
        r##"{
            "schema_version": 1,
            "id": "lock-hide-save",
            "title": "Lock Hide Save",
            "scenes": [{
                "id": "sc1",
                "title": "Scene 1",
                "slides": [{
                    "id": "slide1",
                    "title": "Slide 1",
                    "duration_ms": 3000,
                    "objects": [
                        { "id": "a", "name": "Alpha", "kind": { "type": "shape", "shape": "rectangle", "fill": "#2F6FEB" }, "frame": { "x": 100, "y": 100, "w": 200, "h": 120 } }
                    ]
                }]
            }]
        }"##,
    )
    .expect("course");
    let mut h = TestHarness::new(course);
    h.idle_frame();

    h.editor.run(
        "object.lock",
        serde_json::json!({ "slide": "slide1", "object": "a", "locked": true }),
    );
    h.editor.run(
        "object.hide_in_editor",
        serde_json::json!({ "slide": "slide1", "object": "a", "hidden": true }),
    );

    let json = h.editor.session.course.to_json().expect("serialize");
    let reloaded = Course::from_json(&json).expect("reload");
    let obj = reloaded
        .slide("slide1")
        .expect("slide")
        .object("a")
        .expect("a")
        .0;
    assert!(obj.locked);
    assert!(obj.editor_hidden);

    // Undo hide, then undo lock.
    h.editor.run("edit.undo", serde_json::json!({}));
    assert!(
        !h.editor
            .session
            .course
            .slide("slide1")
            .expect("slide")
            .object("a")
            .expect("a")
            .0
            .editor_hidden
    );
    h.editor.run("edit.undo", serde_json::json!({}));
    assert!(
        !h.editor
            .session
            .course
            .slide("slide1")
            .expect("slide")
            .object("a")
            .expect("a")
            .0
            .locked
    );
}

fn click_requests(h: &mut TestHarness, pos: egui::Pos2) -> Vec<crate::EditorRequest> {
    h.frame(vec![egui::Event::PointerMoved(pos)]);
    let mut requests = h.frame(vec![egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Default::default(),
    }]);
    requests.extend(h.frame(vec![egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: Default::default(),
    }]));
    requests
}

#[test]
fn template_picker_shows_my_library_and_save_to_library_when_available() {
    let course = template_course(TemplateId::Blank).expect("blank");
    let mut h = TestHarness::new(course);
    h.editor.ui.show_welcome = false;
    h.editor.set_library_templates_available(true);
    h.editor
        .set_library_templates(vec![crate::LibraryTemplateEntry {
            id: "intro-card".to_owned(),
            title: "Intro Card".to_owned(),
        }]);
    let first_slide = h.editor.session.course.scenes[0].slides[0].id.clone();
    h.editor
        .run(
            "template.save",
            serde_json::json!({ "slide": first_slide, "title": "Course Tpl" }),
        )
        .expect("template saved");
    h.idle_frame();

    let caret = h
        .find_icon(coursecraft_render::Icon::CaretDown)
        .expect("toolbar caret");
    h.click_at(caret);
    h.idle_frame();

    assert!(h.find_text("Your templates").is_some());
    assert!(h.find_text("Save to my library").is_some());
    assert!(h.find_text("My library").is_some());
    assert!(h.find_text("Intro Card").is_some());

    let save_pos = h.find_text("Save to my library").expect("save button");
    let requests = click_requests(&mut h, save_pos);
    assert!(
        requests
            .iter()
            .any(|r| matches!(r, crate::EditorRequest::SaveTemplateToLibrary { .. })),
        "clicking Save to my library should request SaveTemplateToLibrary"
    );
}

#[test]
fn template_picker_delete_requires_confirm_before_requesting_delete() {
    let course = template_course(TemplateId::Blank).expect("blank");
    let mut h = TestHarness::new(course);
    h.editor.ui.show_welcome = false;
    h.editor.set_library_templates_available(true);
    h.editor
        .set_library_templates(vec![crate::LibraryTemplateEntry {
            id: "intro-card".to_owned(),
            title: "Intro Card".to_owned(),
        }]);
    h.idle_frame();

    let caret = h
        .find_icon(coursecraft_render::Icon::CaretDown)
        .expect("toolbar caret");
    h.click_at(caret);
    h.idle_frame();

    let del_pos = h.find_text("Delete").expect("Delete button");
    let requests = click_requests(&mut h, del_pos);
    assert!(
        !requests
            .iter()
            .any(|r| matches!(r, crate::EditorRequest::DeleteLibraryTemplate { .. })),
        "first click on Delete should ask for confirmation, not delete immediately"
    );
    h.idle_frame();
    assert!(h.find_text("Confirm").is_some());

    let confirm_pos = h.find_text("Confirm").expect("Confirm button");
    let requests = click_requests(&mut h, confirm_pos);
    assert!(
        requests.iter().any(|r| matches!(
            r,
            crate::EditorRequest::DeleteLibraryTemplate { id } if id == "intro-card"
        )),
        "clicking Confirm should request DeleteLibraryTemplate"
    );
}

#[test]
fn template_picker_hides_my_library_when_unavailable() {
    let course = template_course(TemplateId::Blank).expect("blank");
    let mut h = TestHarness::new(course);
    h.editor.ui.show_welcome = false;
    h.editor.set_library_templates_available(false);
    h.editor
        .set_library_templates(vec![crate::LibraryTemplateEntry {
            id: "intro-card".to_owned(),
            title: "Intro Card".to_owned(),
        }]);
    h.idle_frame();

    let caret = h
        .find_icon(coursecraft_render::Icon::CaretDown)
        .expect("toolbar caret");
    h.click_at(caret);
    h.idle_frame();

    assert!(h.find_text("My library").is_none());
    assert!(h.find_text("Save to my library").is_none());
}
