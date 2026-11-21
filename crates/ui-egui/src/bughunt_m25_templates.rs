#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashSet;

use crate::Editor;
use coursecraft_course::{Course, Object};
use coursecraft_engine::Session;

fn image_object(id: &str, name: &str, src: &str) -> Object {
    serde_json::from_value(serde_json::json!({
        "id": id,
        "name": name,
        "kind": { "type": "image", "src": src },
        "description": "Hero image",
        "frame": { "x": 10.0, "y": 20.0, "w": 100.0, "h": 80.0 }
    }))
    .unwrap()
}

#[test]
fn insert_library_template_avoids_case_only_media_collision_for_folder_save() {
    let mut course: Course = Session::blank("Target Course").course;
    course.scenes[0].slides[0]
        .objects
        .push(image_object("img-existing", "Logo", "Logo.png"));
    let mut editor = Editor::new(course);
    editor.set_media(vec![("Logo.png".to_owned(), vec![1, 2, 3, 4].into())]);

    let mut template_slide = Session::blank("Template").course.scenes[0].slides[0].clone();
    template_slide
        .objects
        .push(image_object("img-tpl", "Lower Logo", "logo.png"));

    editor.insert_library_template(
        template_slide,
        vec![("logo.png".to_owned(), vec![5, 6, 7, 8])],
        None,
        None,
    );

    let used = editor.used_media();
    let mut seen_lower = HashSet::new();
    for (src, _) in &used {
        assert!(
            seen_lower.insert(src.to_ascii_lowercase()),
            "used_media contains case-only duplicate media path {src:?} in {used:?}, which causes save_project_folder to fail with DuplicateEntry"
        );
    }
}
