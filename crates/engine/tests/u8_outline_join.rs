use coursecraft_course::{Course, LineJoin, ObjectKind};
use coursecraft_engine::Session;
use serde_json::json;

#[test]
fn object_set_outline_join_updates_shape_and_undoes_in_one_step() {
    let raw = r##"{
      "schema_version": 1,
      "id": "c-join",
      "title": "Join Test",
      "slide_size": { "w": 1280, "h": 720 },
      "scenes": [{
        "id": "scene1",
        "title": "Main",
        "slides": [{
          "id": "s1",
          "title": "Slide 1",
          "objects": [{
            "id": "sh1",
            "frame": { "x": 40, "y": 40, "w": 120, "h": 80 },
            "kind": {
              "type": "shape",
              "shape": "rectangle",
              "fill": "#3B82F6",
              "outline": { "color": "#111827", "width": 4 }
            }
          }]
        }]
      }]
    }"##;
    let course: Course = serde_json::from_str(raw).expect("valid course");
    let mut session = Session::new(course);
    let before = session.course.clone();

    let res = session
        .execute(
            "object.set_outline_join",
            &json!({ "slide": "s1", "object": "sh1", "join": "round" }),
        )
        .expect("set_outline_join");
    assert_eq!(res["changed"], 1);
    assert_eq!(res["join"], "round");

    let obj = &session.course.slide("s1").unwrap().objects[0];
    match &obj.kind {
        ObjectKind::Shape {
            outline: Some(out), ..
        } => assert_eq!(out.join, LineJoin::Round),
        other => panic!("unexpected kind: {other:?}"),
    }

    session.undo().expect("undo");
    assert_eq!(session.course, before);

    session.redo().expect("redo");
    let obj = &session.course.slide("s1").unwrap().objects[0];
    match &obj.kind {
        ObjectKind::Shape {
            outline: Some(out), ..
        } => assert_eq!(out.join, LineJoin::Round),
        other => panic!("unexpected kind: {other:?}"),
    }
}
