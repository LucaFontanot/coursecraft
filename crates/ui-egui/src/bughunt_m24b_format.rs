//! Bug hunt (2026-11-20, M24, second pass): Format panel with a mixed multi-selection.

use crate::properties::InspectorTab;
use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use egui::Pos2;

fn mixed_course() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    let mut value: serde_json::Value = serde_json::from_str(&json).expect("json");
    let slide = find_slide_mut(&mut value, "s-intro");
    let objects = slide["objects"].as_array_mut().expect("objects");
    objects.push(serde_json::json!({
        "id": "mix-card",
        "kind": { "type": "shape", "shape": "rectangle", "fill": "#112233" },
        "frame": { "x": 10.0, "y": 300.0, "w": 80.0, "h": 60.0 }
    }));
    objects.push(serde_json::json!({
        "id": "mix-pic",
        "kind": { "type": "image", "src": "media/none.png", "alt": "" },
        "frame": { "x": 120.0, "y": 300.0, "w": 80.0, "h": 60.0 }
    }));
    Course::from_json(&value.to_string()).expect("valid mixed course")
}

fn find_slide_mut<'a>(value: &'a mut serde_json::Value, id: &str) -> &'a mut serde_json::Value {
    for scene in value["scenes"].as_array_mut().expect("scenes") {
        for slide in scene["slides"].as_array_mut().expect("slides") {
            if slide["id"] == id {
                return slide;
            }
        }
    }
    panic!("slide {id} not found");
}

fn object_json(course: &Course, id: &str) -> serde_json::Value {
    let (obj, _) = course
        .slide("s-intro")
        .and_then(|s| s.object(id))
        .unwrap_or_else(|| panic!("object {id}"));
    serde_json::to_value(obj).expect("json")
}

#[test]
fn mixed_selection_opacity_is_one_undo_step_and_undo_restores_every_object() {
    let mut h = TestHarness::new(mixed_course());
    h.editor.selection.slide = "s-intro".into();
    h.editor.selection.set_many(vec![
        "title".to_owned(),
        "btn-start".to_owned(),
        "mix-card".to_owned(),
        "mix-pic".to_owned(),
    ]);
    h.editor.ui.inspector_tab = InspectorTab::Format;
    h.idle_frame();
    h.idle_frame();

    let ids = ["title", "btn-start", "mix-card", "mix-pic"];
    let before: Vec<serde_json::Value> = ids
        .iter()
        .map(|id| object_json(&h.editor.session.course, id))
        .collect();
    let undo_before = h.editor.session.undo_len();

    let rect = h
        .editor
        .ui
        .properties
        .opacity_rect
        .expect("multi-select opacity slider");
    let target = Pos2::new(rect.min.x + rect.width() * 0.25, rect.center().y);
    h.click_at(target);
    h.idle_frame();

    assert_eq!(
        h.editor.session.undo_len(),
        undo_before + 1,
        "one opacity change is one undo step"
    );
    for id in ids {
        let after = object_json(&h.editor.session.course, id);
        assert_ne!(after, object_json_of(&before, &ids, id), "{id} changed");
    }

    h.editor.session.undo().expect("undo");
    h.idle_frame();
    for (id, expected) in ids.iter().zip(before.iter()) {
        assert_eq!(
            &object_json(&h.editor.session.course, id),
            expected,
            "undo restores {id} exactly"
        );
    }
}

fn object_json_of(before: &[serde_json::Value], ids: &[&str], id: &str) -> serde_json::Value {
    let index = ids.iter().position(|x| *x == id).expect("id");
    before[index].clone()
}
