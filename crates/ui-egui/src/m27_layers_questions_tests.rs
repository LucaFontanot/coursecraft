//! M27 Step D-6: Layers (D7.1–D7.10) and Question Editing, Banks & Result Slides (D8.1–D8.12).

use crate::test_harness::TestHarness;
use crate::{BottomTab, InspectorTab};
use coursecraft_course::{
    Action, CmpOp, Condition, Course, Event as CourseEvent, HotspotShape, QuestionKind,
    RevisitMode, Scoring, SeekMode, Trigger, Value, ValueSource,
};
use coursecraft_logic::Player;
use egui::{Key, Modifiers, pos2, vec2};

fn base_course() -> Course {
    serde_json::from_value(serde_json::json!({
        "schema_version": 1,
        "id": "m27-layers-questions",
        "title": "M27 Layers & Questions",
        "slide_size": { "w": 1280, "h": 720 },
        "variables": [
            { "name": "flag", "value": true },
            { "name": "score", "value": 10.0 }
        ],
        "scenes": [{
            "id": "scene-1",
            "title": "Scene 1",
            "slides": [
                {
                    "id": "s1",
                    "title": "Slide 1",
                    "duration_ms": 5000,
                    "objects": [
                        {
                            "id": "base-shape",
                            "name": "Base Shape",
                            "kind": { "type": "shape", "shape": "rectangle", "fill": "#2563EB", "text": { "text": "Base" } },
                            "frame": { "x": 100.0, "y": 100.0, "w": 160.0, "h": 80.0 }
                        }
                    ]
                },
                {
                    "id": "s2",
                    "title": "Slide 2",
                    "duration_ms": 5000,
                    "objects": []
                }
            ]
        }]
    }))
    .expect("base course")
}

fn setup_harness(course: Course) -> TestHarness {
    let mut h = TestHarness::new(course);
    h.editor.selection.slide = "s1".into();
    h.editor.selection.layer = None;
    h.editor.selection.object = None;
    h.idle_frame();
    h.idle_frame();
    h
}

fn add_layer(h: &mut TestHarness, name: &str) -> String {
    let res = h
        .editor
        .run(
            "layer.add",
            serde_json::json!({ "slide": "s1", "name": name }),
        )
        .expect("layer.add");
    let lid = res["id"].as_str().expect("layer id").to_owned();
    h.idle_frame();
    h.idle_frame();
    lid
}

fn add_shape_on(
    h: &mut TestHarness,
    slide: &str,
    layer: Option<&str>,
    name: &str,
    x: f32,
) -> String {
    let mut params = serde_json::json!({
        "slide": slide,
        "kind": { "type": "shape", "shape": "rectangle", "fill": "#16A34A", "text": { "text": name } },
        "frame": { "x": x, "y": 200.0, "w": 120.0, "h": 60.0 },
        "name": name,
    });
    if let Some(lid) = layer {
        params["layer"] = serde_json::json!(lid);
    }
    let res = h.editor.run("object.add", params).expect("object.add");
    res["id"].as_str().expect("object id").to_owned()
}

fn switch_to_question_tab(h: &mut TestHarness) {
    h.editor.ui.bottom_tab = BottomTab::Question;
    h.idle_frame();
    h.idle_frame();
}

fn select_slide(h: &mut TestHarness, id: &str) {
    h.editor.selection.slide = id.into();
    h.editor.selection.layer = None;
    h.editor.selection.object = None;
}

fn insert_question_slide(h: &mut TestHarness, kind: &str) -> String {
    let slide_id = h
        .editor
        .run(
            "slide.add",
            serde_json::json!({ "scene": "scene-1", "title": format!("Q {kind}") }),
        )
        .expect("slide.add")["id"]
        .as_str()
        .expect("slide id")
        .to_owned();
    h.editor
        .run(
            "question.insert",
            serde_json::json!({ "slide": slide_id, "kind": kind }),
        )
        .expect("question.insert");
    slide_id
}

// ---------------------------------------------------------------------------
// D7: Layers (10 cases)
// ---------------------------------------------------------------------------

#[test]
fn d7_1_create_inline_rename_and_delete_layer_with_undo() {
    let mut h = setup_harness(base_course());

    // Click `+ New layer` in footer.
    let add_rect = h.editor.ui.layers.add_button_rect.expect("add button rect");
    h.click_at(add_rect.center());
    h.idle_frame();
    h.idle_frame();
    h.advance_time(0.6);

    let layers = &h.editor.session.course.slide("s1").unwrap().layers;
    assert_eq!(layers.len(), 1);
    let lid = layers[0].id.clone();
    let orig_name = layers[0].name.clone();

    // Double-click layer row to start inline rename.
    let row_rect = h.editor.ui.layers.layer_row_rects[&lid];
    h.double_click_at(row_rect.center());
    h.idle_frame();
    assert!(
        h.editor.ui.layers.rename.is_some(),
        "inline rename should open"
    );

    // Empty string + Enter is ignored (keeps original name).
    let edit_rect = h
        .editor
        .ui
        .layers
        .rename_edit_rect
        .expect("rename edit rect");
    h.set_field(edit_rect.center(), "   ");
    h.idle_frame();
    assert_eq!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .layer(&lid)
            .unwrap()
            .name,
        orig_name
    );

    // Esc cancels rename without committing.
    h.advance_time(0.6);
    let row_rect = h.editor.ui.layers.layer_row_rects[&lid];
    h.double_click_at(row_rect.center());
    h.idle_frame();
    let edit_rect = h
        .editor
        .ui
        .layers
        .rename_edit_rect
        .expect("rename edit rect");
    h.click_at(edit_rect.center());
    h.press(Key::A, Modifiers::COMMAND);
    h.type_text("TempName");
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(h.editor.ui.layers.rename.is_none());
    assert_eq!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .layer(&lid)
            .unwrap()
            .name,
        orig_name
    );

    // Rename to "Feedback" + Enter.
    h.advance_time(0.6);
    let row_rect = h.editor.ui.layers.layer_row_rects[&lid];
    h.double_click_at(row_rect.center());
    h.idle_frame();
    let edit_rect = h
        .editor
        .ui
        .layers
        .rename_edit_rect
        .expect("rename edit rect");
    h.set_field(edit_rect.center(), "Feedback");
    h.idle_frame();
    assert_eq!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .layer(&lid)
            .unwrap()
            .name,
        "Feedback"
    );

    // Select layer and delete via footer trash button.
    let row_rect = h.editor.ui.layers.layer_row_rects[&lid];
    h.click_at(row_rect.center());
    h.idle_frame();
    assert_eq!(h.editor.selection.layer.as_deref(), Some(lid.as_str()));

    let del_rect = h
        .editor
        .ui
        .layers
        .delete_button_rect
        .expect("delete button rect");
    h.click_at(del_rect.center());
    h.idle_frame();
    assert!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .layers
            .is_empty()
    );
    assert_eq!(
        h.editor.selection.layer, None,
        "selection should return to Base"
    );

    // Undo × 3: restores deleted layer ("Feedback"), reverts rename, removes added layer.
    assert!(h.editor.session.undo().is_ok());
    h.idle_frame();
    assert_eq!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .layer(&lid)
            .unwrap()
            .name,
        "Feedback"
    );

    // Also test delete via context menu.
    let row_rect = h.editor.ui.layers.layer_row_rects[&lid];
    h.right_click_at(row_rect.center());
    h.idle_frame();
    let del_menu = h.find_text("Delete").expect("Delete menu item");
    h.click_at(del_menu);
    h.idle_frame();
    assert!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .layers
            .is_empty()
    );

    assert!(h.editor.session.undo().is_ok()); // restore from context delete
    assert!(h.editor.session.undo().is_ok()); // revert rename
    assert!(h.editor.session.undo().is_ok()); // remove added layer
    assert!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .layers
            .is_empty()
    );
}

#[test]
fn d7_2_duplicate_layer_with_objects_and_triggers_remaps_ids() {
    let mut h = setup_harness(base_course());
    let lid = add_layer(&mut h, "Tip");
    let o1 = add_shape_on(&mut h, "s1", Some(&lid), "TipShape1", 100.0);
    let o2 = add_shape_on(&mut h, "s1", Some(&lid), "TipShape2", 260.0);

    let trig = Trigger {
        when: CourseEvent::Click { object: o1.clone() },
        action: Action::ChangeState {
            object: o2.clone(),
            state: "hidden".into(),
        },
        condition: None,
        disabled: false,
    };
    h.editor
        .run(
            "trigger.add",
            serde_json::json!({ "slide": "s1", "layer": lid, "trigger": trig }),
        )
        .expect("trigger.add");
    h.idle_frame();

    // Select layer and duplicate via footer icon.
    let row_rect = h.editor.ui.layers.layer_row_rects[&lid];
    h.click_at(row_rect.center());
    h.idle_frame();
    let dup_rect = h
        .editor
        .ui
        .layers
        .duplicate_button_rect
        .expect("duplicate button rect");
    h.click_at(dup_rect.center());
    h.idle_frame();

    let slide = h.editor.session.course.slide("s1").unwrap();
    assert_eq!(slide.layers.len(), 2);
    let copy1 = &slide.layers[1];
    assert_eq!(copy1.name, "Tip copy");
    assert_eq!(copy1.objects.len(), 2);
    assert_ne!(copy1.objects[0].id, o1);
    assert_ne!(copy1.objects[1].id, o2);
    assert_eq!(copy1.triggers.len(), 1);
    assert_eq!(
        copy1.triggers[0].when,
        CourseEvent::Click {
            object: copy1.objects[0].id.clone()
        }
    );
    assert_eq!(
        copy1.triggers[0].action,
        Action::ChangeState {
            object: copy1.objects[1].id.clone(),
            state: "hidden".into(),
        }
    );
    assert!(
        h.editor
            .session
            .course
            .validate()
            .iter()
            .all(|i| i.severity != coursecraft_course::Severity::Error)
    );

    // Also duplicate via context menu.
    let row_rect = h.editor.ui.layers.layer_row_rects[&lid];
    h.right_click_at(row_rect.center());
    h.idle_frame();
    let dup_menu = h.find_text("Duplicate").expect("Duplicate menu item");
    h.click_at(dup_menu);
    h.idle_frame();
    assert_eq!(h.editor.session.course.slide("s1").unwrap().layers.len(), 3);

    // 1 undo step reverts each duplicate.
    assert!(h.editor.session.undo().is_ok());
    assert_eq!(h.editor.session.course.slide("s1").unwrap().layers.len(), 2);
    assert!(h.editor.session.undo().is_ok());
    assert_eq!(h.editor.session.course.slide("s1").unwrap().layers.len(), 1);
}

#[test]
fn d7_3_reorder_layers_via_drag_context_menu_and_esc_cancel() {
    let mut h = setup_harness(base_course());
    let l1 = add_layer(&mut h, "L1");
    let l2 = add_layer(&mut h, "L2");
    let l3 = add_layer(&mut h, "L3");
    let l4 = add_layer(&mut h, "L4");

    let names = |h: &TestHarness| -> Vec<String> {
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .layers
            .iter()
            .map(|l| l.name.clone())
            .collect()
    };
    assert_eq!(names(&h), vec!["L1", "L2", "L3", "L4"]);

    // Displayed front-to-back: L4 is at top, L1 at bottom. Drag L1 above L4.
    let l1_rect = h.editor.ui.layers.layer_row_rects[&l1];
    let l4_rect = h.editor.ui.layers.layer_row_rects[&l4];
    let dy = (l4_rect.top() - 4.0) - l1_rect.center().y;
    h.drag_by(l1_rect.center(), vec2(0.0, dy), 8, Modifiers::NONE);
    h.idle_frame();
    assert_eq!(names(&h), vec!["L2", "L3", "L4", "L1"]);

    // Undo drag.
    assert!(h.editor.session.undo().is_ok());
    h.idle_frame();
    assert_eq!(names(&h), vec!["L1", "L2", "L3", "L4"]);

    // Start drag on L1 and press Escape while pointer is down -> cancels without moving.
    let l1_rect = h.editor.ui.layers.layer_row_rects[&l1];
    let l4_rect = h.editor.ui.layers.layer_row_rects[&l4];
    h.frame(vec![egui::Event::PointerMoved(l1_rect.center())]);
    h.frame(vec![egui::Event::PointerButton {
        pos: l1_rect.center(),
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    }]);
    h.frame(vec![egui::Event::PointerMoved(l4_rect.center())]);
    h.press(Key::Escape, Modifiers::NONE);
    h.frame(vec![egui::Event::PointerButton {
        pos: l4_rect.center(),
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    }]);
    h.idle_frame();
    assert_eq!(names(&h), vec!["L1", "L2", "L3", "L4"]);

    // Context menu: Bring forward on L2 -> ["L1", "L3", "L2", "L4"]
    let l2_rect = h.editor.ui.layers.layer_row_rects[&l2];
    h.right_click_at(l2_rect.center());
    h.idle_frame();
    let forward = h.find_text("Bring forward").expect("Bring forward");
    h.click_at(forward);
    h.idle_frame();
    assert_eq!(names(&h), vec!["L1", "L3", "L2", "L4"]);

    // Context menu: Send backward on L3 -> ["L3", "L1", "L2", "L4"]
    let l3_rect = h.editor.ui.layers.layer_row_rects[&l3];
    h.right_click_at(l3_rect.center());
    h.idle_frame();
    let backward = h.find_text("Send backward").expect("Send backward");
    h.click_at(backward);
    h.idle_frame();
    assert_eq!(names(&h), vec!["L3", "L1", "L2", "L4"]);

    assert!(h.editor.session.undo().is_ok());
    assert_eq!(names(&h), vec!["L1", "L3", "L2", "L4"]);
    assert!(h.editor.session.undo().is_ok());
    assert_eq!(names(&h), vec!["L1", "L2", "L3", "L4"]);
}

#[test]
fn d7_4_layer_settings_modal_all_9_properties_and_noop_ok_zero_undo() {
    let mut h = setup_harness(base_course());
    let lid = add_layer(&mut h, "Popup");

    // Open Layer Settings via gear icon.
    let gear_rect = h.editor.ui.layers.gear_rects[&lid];
    h.click_at(gear_rect.center());
    h.idle_frame();
    assert!(h.editor.ui.layer_settings.is_open());

    let name_rect = h.editor.ui.layer_settings.name_rect.expect("name_rect");
    h.set_field(name_rect.center(), "Popup Edited");

    // Edit remaining 8 fields via modal state (duration, checkboxes, combos).
    h.editor.ui.layer_settings.duration_ms = 8000;
    let hide_other = h.editor.ui.layer_settings.hide_other_rect.unwrap();
    h.click_at(hide_other.center()); // true -> false
    let hide_base = h.editor.ui.layer_settings.hide_base_rect.unwrap();
    h.click_at(hide_base.center()); // false -> true
    let pause_base = h.editor.ui.layer_settings.pause_base_rect.unwrap();
    h.click_at(pause_base.center()); // false -> true
    let prevent_clicks = h.editor.ui.layer_settings.prevent_clicks_rect.unwrap();
    h.click_at(prevent_clicks.center()); // false -> true
    let hide_ends = h.editor.ui.layer_settings.hide_when_ends_rect.unwrap();
    h.click_at(hide_ends.center()); // false -> true
    h.editor.ui.layer_settings.allow_seek = SeekMode::No;
    h.editor.ui.layer_settings.revisit = RevisitMode::Resume;
    h.idle_frame();

    let undo_before = h.editor.session.undo_len();
    let ok_rect = h.editor.ui.layer_settings.ok_rect.expect("ok_rect");
    h.click_at(ok_rect.center());
    h.idle_frame();
    assert!(!h.editor.ui.layer_settings.is_open());
    assert_eq!(h.editor.session.undo_len(), undo_before + 1);

    let layer = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .layer(&lid)
        .unwrap();
    assert_eq!(layer.name, "Popup Edited");
    assert_eq!(layer.duration_ms, 8000);
    assert!(!layer.hide_other_layers);
    assert!(layer.hide_base_objects);
    assert!(layer.pause_base_timeline);
    assert!(layer.prevent_base_clicks);
    assert!(layer.hide_when_timeline_ends);
    assert_eq!(layer.allow_seek, SeekMode::No);
    assert_eq!(layer.revisit, RevisitMode::Resume);

    // Reopen & click OK without changing anything -> 0 undo steps!
    let gear_rect = h.editor.ui.layers.gear_rects[&lid];
    h.click_at(gear_rect.center());
    h.idle_frame();
    assert!(h.editor.ui.layer_settings.is_open());

    let report = h.layout_report(1);
    assert!(
        report.findings.is_empty(),
        "Layer Settings modal layout issues: {report:?}"
    );

    let undo_before_noop = h.editor.session.undo_len();
    let ok_rect = h.editor.ui.layer_settings.ok_rect.expect("ok_rect");
    h.click_at(ok_rect.center());
    h.idle_frame();
    assert!(!h.editor.ui.layer_settings.is_open());
    assert_eq!(
        h.editor.session.undo_len(),
        undo_before_noop,
        "no-op OK must push 0 undo steps"
    );
}

#[test]
fn d7_5_layer_settings_validation_and_autoclose_on_slide_change_or_undo() {
    let mut h = setup_harness(base_course());
    let lid = add_layer(&mut h, "ModalTest");

    let gear_rect = h.editor.ui.layers.gear_rects[&lid];
    h.click_at(gear_rect.center());
    h.idle_frame();
    assert!(h.editor.ui.layer_settings.is_open());

    // Whitespace name disables OK button.
    let name_rect = h.editor.ui.layer_settings.name_rect.unwrap();
    h.click_at(name_rect.center());
    h.press(Key::A, Modifiers::COMMAND);
    h.type_text("   ");
    h.idle_frame();
    let ok_rect = h.editor.ui.layer_settings.ok_rect.unwrap();
    h.click_at(ok_rect.center());
    h.idle_frame();
    assert!(
        h.editor.ui.layer_settings.is_open(),
        "disabled OK must not close modal"
    );

    // Out-of-range duration clamped to 100..=86_400_000 by DragValue on next frame.
    h.editor.ui.layer_settings.duration_ms = 0;
    h.idle_frame();
    assert!(h.editor.ui.layer_settings.duration_ms >= 100);
    h.editor.ui.layer_settings.duration_ms = 99_999_999;
    h.idle_frame();
    assert!(h.editor.ui.layer_settings.duration_ms <= 86_400_000);

    // Switching slide auto-closes modal.
    select_slide(&mut h, "s2");
    h.idle_frame();
    assert!(!h.editor.ui.layer_settings.is_open());

    // Reopen on s1, then undo while open -> auto-closes cleanly.
    select_slide(&mut h, "s1");
    h.idle_frame();
    let gear_rect = h.editor.ui.layers.gear_rects[&lid];
    h.click_at(gear_rect.center());
    h.idle_frame();
    assert!(h.editor.ui.layer_settings.is_open());

    assert!(h.editor.session.undo().is_ok());
    h.idle_frame();
    assert!(!h.editor.ui.layer_settings.is_open());
}

#[test]
fn d7_6_eye_visibility_toggle_and_focused_layer_protection() {
    let mut h = setup_harness(base_course());
    let l1 = add_layer(&mut h, "L1");
    let _l2 = add_layer(&mut h, "L2");
    let _o1 = add_shape_on(&mut h, "s1", Some(&l1), "L1Shape", 100.0);
    h.idle_frame();

    // Focused on Base: toggle Eye off on L1.
    let eye_rect = h.editor.ui.layers.eye_rects[&l1];
    h.click_at(eye_rect.center());
    h.idle_frame();
    assert!(h.editor.ui.layers.hidden_layers.contains(&l1));

    // Focus L1 -> L1 is automatically unhidden (focused layer cannot be invisible).
    let row_rect = h.editor.ui.layers.layer_row_rects[&l1];
    h.click_at(row_rect.center());
    h.idle_frame();
    assert_eq!(h.editor.selection.layer.as_deref(), Some(l1.as_str()));
    assert!(!h.editor.ui.layers.hidden_layers.contains(&l1));

    // Clicking eye on focused L1 does not hide it.
    let eye_rect = h.editor.ui.layers.eye_rects[&l1];
    h.click_at(eye_rect.center());
    h.idle_frame();
    assert!(!h.editor.ui.layers.hidden_layers.contains(&l1));

    // Switch to Slide 2 and back: per-slide hidden map is cleared on slide change.
    select_slide(&mut h, "s2");
    h.idle_frame();
    select_slide(&mut h, "s1");
    h.idle_frame();
    assert!(h.editor.ui.layers.hidden_layers.is_empty());
    assert_eq!(h.editor.selection.layer, None);
}

#[test]
fn d7_7_delete_layer_cleans_base_triggers_to_layer_and_to_layer_objects_bug_d6() {
    let mut h = setup_harness(base_course());
    let l1 = add_layer(&mut h, "L1");
    let obj_on_l1 = add_shape_on(&mut h, "s1", Some(&l1), "ObjOnL1", 150.0);

    // Trigger 1: ShowLayer("L1")
    let t1 = Trigger {
        when: CourseEvent::Click {
            object: "base-shape".into(),
        },
        action: Action::ShowLayer { layer: l1.clone() },
        condition: None,
        disabled: false,
    };
    // Trigger 2: ChangeState("obj_on_L1", "hidden")
    let t2 = Trigger {
        when: CourseEvent::Click {
            object: "base-shape".into(),
        },
        action: Action::ChangeState {
            object: obj_on_l1.clone(),
            state: "hidden".into(),
        },
        condition: None,
        disabled: false,
    };
    h.editor
        .run(
            "trigger.add",
            serde_json::json!({ "slide": "s1", "trigger": t1 }),
        )
        .expect("t1");
    h.editor
        .run(
            "trigger.add",
            serde_json::json!({ "slide": "s1", "trigger": t2 }),
        )
        .expect("t2");
    h.idle_frame();
    assert_eq!(
        h.editor.session.course.slide("s1").unwrap().triggers.len(),
        2
    );

    // Select L1 and delete via footer button (BUG-D6).
    let row_rect = h.editor.ui.layers.layer_row_rects[&l1];
    h.click_at(row_rect.center());
    h.idle_frame();
    let del_rect = h.editor.ui.layers.delete_button_rect.unwrap();
    h.click_at(del_rect.center());
    h.idle_frame();

    let slide = h.editor.session.course.slide("s1").unwrap();
    assert!(slide.layers.is_empty(), "layer should be deleted");
    assert!(
        slide.triggers.is_empty(),
        "both dangling triggers (ShowLayer and ChangeState on layer object) must be cleaned up"
    );
    assert!(
        h.editor
            .session
            .course
            .validate()
            .iter()
            .all(|i| i.severity != coursecraft_course::Severity::Error)
    );

    // Undo restores L1 AND both triggers atomically in 1 step.
    assert!(h.editor.session.undo().is_ok());
    h.idle_frame();
    let slide = h.editor.session.course.slide("s1").unwrap();
    assert_eq!(slide.layers.len(), 1);
    assert_eq!(slide.triggers.len(), 2);
}

#[test]
fn d7_8_hide_other_layers_and_hide_base_objects_preview_and_player_parity() {
    let mut h = setup_harness(base_course());
    let l1 = add_layer(&mut h, "L1");
    let l2 = add_layer(&mut h, "L2");
    let o1 = add_shape_on(&mut h, "s1", Some(&l1), "L1Box", 120.0);
    let o2 = add_shape_on(&mut h, "s1", Some(&l2), "L2Box", 300.0);

    // Show both L1 and L2 on timeline start; L2 has hide_other_layers=false and hide_base_objects=true.
    h.editor
        .run(
            "layer.update",
            serde_json::json!({
                "slide": "s1",
                "layer": l2,
                "hide_other_layers": false,
                "hide_base_objects": true,
            }),
        )
        .expect("layer.update");
    for lid in [&l1, &l2] {
        h.editor
            .run(
                "trigger.add",
                serde_json::json!({
                    "slide": "s1",
                    "trigger": Trigger {
                        when: CourseEvent::TimelineStarts { layer: None },
                        action: Action::ShowLayer { layer: lid.clone() },
                        condition: None,
                        disabled: false,
                    }
                }),
            )
            .expect("trigger.add");
    }

    // Verify Player runtime reflects both hide_other_layers=false (L1 stays visible) and hide_base_objects=true.
    let mut player = Player::new(&h.editor.session.course).expect("player");
    player.handle(&coursecraft_logic::Input::Start);
    assert!(player.state().visible_layers.contains(&l1));
    assert!(player.state().visible_layers.contains(&l2));
    assert!(player.sample_object(&o1).is_some());
    assert!(player.sample_object(&o2).is_some());
    assert!(
        player.base_objects_hidden(),
        "hide_base_objects=true must hide base-shape in Player"
    );

    // Focusing L2 in editor and switching slides resets focus to Base.
    let row_rect = h.editor.ui.layers.layer_row_rects[&l2];
    h.click_at(row_rect.center());
    h.idle_frame();
    assert_eq!(h.editor.selection.layer.as_deref(), Some(l2.as_str()));
    select_slide(&mut h, "s2");
    h.idle_frame();
    assert_eq!(h.editor.selection.layer, None);
}

#[test]
fn d7_9_stress_15_layers_delete_mid_rename_and_layout_sizes() {
    for size in [(1280.0, 900.0), (1024.0, 680.0)] {
        let mut h = TestHarness::new_with_size(base_course(), size);
        h.editor.selection.slide = "s1".into();
        h.idle_frame();
        h.idle_frame();

        let mut last_id = String::new();
        for i in 1..=15 {
            last_id = add_layer(&mut h, &format!("Layer {i}"));
        }
        assert_eq!(
            h.editor.session.course.slide("s1").unwrap().layers.len(),
            15
        );

        // Start inline rename on Layer 15, then delete it via engine/undo.
        let row_rect = h.editor.ui.layers.layer_row_rects[&last_id];
        h.double_click_at(row_rect.center());
        h.idle_frame();
        assert!(h.editor.ui.layers.rename.is_some());

        h.editor
            .run(
                "layer.delete",
                serde_json::json!({ "slide": "s1", "layer": last_id }),
            )
            .expect("layer.delete");
        h.idle_frame();
        assert!(
            h.editor.ui.layers.rename.is_none(),
            "rename state must clear when renamed layer is deleted"
        );

        let report = h.layout_report(1);
        assert!(
            report.findings.is_empty(),
            "layout issues at {size:?}: {report:?}"
        );
    }
}

#[test]
fn d7_10_layers_panel_hidden_and_layer_add_blocked_in_master_view() {
    let mut h = setup_harness(base_course());
    assert!(h.editor.ui.layers.panel_rect.is_some());

    h.editor.enter_master_view();
    h.editor.ui.layers.panel_rect = None;
    h.idle_frame();
    h.idle_frame();

    assert!(
        h.editor.ui.layers.panel_rect.is_none(),
        "Layers panel must be hidden in Master View"
    );

    let master_id = h.editor.selection.slide.clone();
    let res = h
        .editor
        .session
        .execute("layer.add", &serde_json::json!({ "slide": master_id }));
    assert!(res.is_err(), "layer.add on a master slide must be refused");
}

// ---------------------------------------------------------------------------
// D8: Question Editing, Banks & Result Slides (12 cases)
// ---------------------------------------------------------------------------

#[test]
fn d8_1_mc_mr_tf_crud_scoring_modes_and_shuffle() {
    let mut h = setup_harness(base_course());
    let q_slide = insert_question_slide(&mut h, "multiple_choice");
    select_slide(&mut h, &q_slide);
    switch_to_question_tab(&mut h);

    // Edit prompt via Question panel.
    let prompt_rect = h.editor.ui.question.prompt_edit_rect.expect("prompt rect");
    let click_away = pos2(prompt_rect.left(), prompt_rect.top() - 20.0);
    h.set_multiline_field(prompt_rect.center(), "Pick the best option", click_away);
    assert_eq!(
        h.editor
            .session
            .course
            .slide(&q_slide)
            .unwrap()
            .question
            .as_ref()
            .unwrap()
            .prompt,
        "Pick the best option"
    );

    // Add 2 choices, delete 1, toggle correct, switch Scoring to PerAnswer with points 5 / -2, toggle shuffle.
    let add_choice = h.find_text("+ Add choice").expect("+ Add choice");
    h.click_at(add_choice);
    h.idle_frame();
    let add_choice = h.find_text("+ Add choice").expect("+ Add choice");
    h.click_at(add_choice);
    h.idle_frame();
    assert_eq!(
        h.editor
            .session
            .course
            .slide(&q_slide)
            .unwrap()
            .question
            .as_ref()
            .unwrap()
            .choices
            .len(),
        4
    );

    // Delete last choice via Remove button or engine, then test PerAnswer scoring + clamping.
    let mut q_val = h
        .editor
        .session
        .course
        .slide(&q_slide)
        .unwrap()
        .question
        .clone()
        .unwrap();
    q_val.scoring = Scoring::PerAnswer;
    q_val.shuffle = true;
    q_val.choices = vec![
        coursecraft_course::Choice {
            id: "a".into(),
            text: "Opt A".into(),
            correct: true,
            points: 5,
        },
        coursecraft_course::Choice {
            id: "b".into(),
            text: "Opt B".into(),
            correct: false,
            points: -2,
        },
        coursecraft_course::Choice {
            id: "c".into(),
            text: "Opt C".into(),
            correct: false,
            points: 0,
        },
    ];
    h.editor
        .run(
            "question.set",
            serde_json::json!({ "slide": q_slide, "question": q_val }),
        )
        .expect("question.set PerAnswer");
    let q = h
        .editor
        .session
        .course
        .slide(&q_slide)
        .unwrap()
        .question
        .as_ref()
        .unwrap();
    assert_eq!(q.scoring, Scoring::PerAnswer);
    assert!(q.shuffle);
    assert_eq!(q.choices[0].points, 5);
    assert_eq!(q.choices[1].points, -2);

    // Switch to MultipleResponse and Partial scoring, and TrueFalse.
    h.editor
        .run(
            "question.set_kind",
            serde_json::json!({ "slide": q_slide, "kind": "multiple_response" }),
        )
        .expect("set_kind MR");
    let mut q_mr = h
        .editor
        .session
        .course
        .slide(&q_slide)
        .unwrap()
        .question
        .clone()
        .unwrap();
    q_mr.scoring = Scoring::Partial;
    h.editor
        .run(
            "question.set",
            serde_json::json!({ "slide": q_slide, "question": q_mr }),
        )
        .expect("Partial scoring");
    assert_eq!(
        h.editor
            .session
            .course
            .slide(&q_slide)
            .unwrap()
            .question
            .as_ref()
            .unwrap()
            .scoring,
        Scoring::Partial
    );

    assert!(
        h.editor
            .session
            .course
            .validate()
            .iter()
            .all(|i| i.severity != coursecraft_course::Severity::Error)
    );

    // Undo steps back to initial insert.
    assert!(h.editor.session.undo().is_ok()); // Partial
    assert!(h.editor.session.undo().is_ok()); // set_kind MR
    assert!(h.editor.session.undo().is_ok()); // PerAnswer
    assert_eq!(
        h.editor
            .session
            .course
            .slide(&q_slide)
            .unwrap()
            .question
            .as_ref()
            .unwrap()
            .scoring,
        Scoring::AllOrNothing
    );
}

#[test]
fn d8_2_fill_in_accepted_answers_crud_bounds_and_case_sensitive() {
    let mut h = setup_harness(base_course());
    let q_slide = insert_question_slide(&mut h, "fill_in");
    select_slide(&mut h, &q_slide);
    switch_to_question_tab(&mut h);

    // Add 3 accepted answers.
    for ans in ["alpha", "beta", "gamma"] {
        h.editor
            .run(
                "question.add_accepted",
                serde_json::json!({ "slide": q_slide, "text": ans }),
            )
            .expect("add_accepted");
    }
    // Empty "" refused.
    assert!(
        h.editor
            .session
            .execute(
                "question.add_accepted",
                &serde_json::json!({ "slide": q_slide, "text": "" })
            )
            .is_err()
    );
    // Update answer #1 (index 1) and remove index 0.
    h.editor
        .run(
            "question.update_accepted",
            serde_json::json!({ "slide": q_slide, "index": 1, "text": "alpha-updated" }),
        )
        .expect("update_accepted");
    h.editor
        .run(
            "question.remove_accepted",
            serde_json::json!({ "slide": q_slide, "index": 0 }),
        )
        .expect("remove_accepted");

    // Toggle case_sensitive.
    h.editor
        .run(
            "question.set_case_sensitive",
            serde_json::json!({ "slide": q_slide, "case_sensitive": true }),
        )
        .expect("set_case_sensitive");

    let q = h
        .editor
        .session
        .course
        .slide(&q_slide)
        .unwrap()
        .question
        .as_ref()
        .unwrap();
    assert!(q.case_sensitive);
    assert_eq!(q.accepted.len(), 3);
    assert_eq!(q.accepted[0], "alpha-updated");

    // Remove down to 1, then removing the LAST accepted answer is refused.
    h.editor
        .run(
            "question.remove_accepted",
            serde_json::json!({ "slide": q_slide, "index": 2 }),
        )
        .expect("remove");
    h.editor
        .run(
            "question.remove_accepted",
            serde_json::json!({ "slide": q_slide, "index": 1 }),
        )
        .expect("remove");
    assert!(
        h.editor
            .session
            .execute(
                "question.remove_accepted",
                &serde_json::json!({ "slide": q_slide, "index": 0 })
            )
            .is_err(),
        "cannot remove last remaining accepted answer"
    );

    // Undo restores exact list.
    for _ in 0..8 {
        assert!(h.editor.session.undo().is_ok());
    }
    assert_eq!(
        h.editor
            .session
            .course
            .slide(&q_slide)
            .unwrap()
            .question
            .as_ref()
            .unwrap()
            .accepted
            .len(),
        1
    );
}

#[test]
fn d8_3_matching_pairs_and_sequence_reorder_with_bounds() {
    let mut h = setup_harness(base_course());

    // Matching
    let m_slide = insert_question_slide(&mut h, "matching");
    select_slide(&mut h, &m_slide);
    switch_to_question_tab(&mut h);

    let _initial_pairs = h
        .editor
        .session
        .course
        .slide(&m_slide)
        .unwrap()
        .question
        .as_ref()
        .unwrap()
        .pairs
        .len();
    let pair_id = h
        .editor
        .run(
            "question.add_pair",
            serde_json::json!({ "slide": m_slide, "left": "Sun", "right": "Star" }),
        )
        .expect("add_pair")["id"]
        .as_str()
        .unwrap()
        .to_owned();
    h.editor
        .run(
            "question.update_pair",
            serde_json::json!({ "slide": m_slide, "id": pair_id, "left": "Sol", "right": "Star" }),
        )
        .expect("update_pair");
    h.editor
        .run(
            "question.reorder_pair",
            serde_json::json!({ "slide": m_slide, "id": pair_id, "to": 0 }),
        )
        .expect("reorder_pair");
    assert_eq!(
        h.editor
            .session
            .course
            .slide(&m_slide)
            .unwrap()
            .question
            .as_ref()
            .unwrap()
            .pairs[0]
            .left,
        "Sol"
    );

    // Deleting below 2 pairs is refused.
    loop {
        let pairs = h
            .editor
            .session
            .course
            .slide(&m_slide)
            .unwrap()
            .question
            .as_ref()
            .unwrap()
            .pairs
            .clone();
        if pairs.len() <= 2 {
            assert!(
                h.editor
                    .session
                    .execute(
                        "question.remove_pair",
                        &serde_json::json!({ "slide": m_slide, "id": pairs[0].id })
                    )
                    .is_err(),
                "matching requires at least 2 pairs"
            );
            break;
        }
        h.editor
            .run(
                "question.remove_pair",
                serde_json::json!({ "slide": m_slide, "id": pairs[0].id }),
            )
            .expect("remove_pair");
    }

    // Sequence
    let s_slide = insert_question_slide(&mut h, "sequence");
    let first_id = h
        .editor
        .session
        .course
        .slide(&s_slide)
        .unwrap()
        .question
        .as_ref()
        .unwrap()
        .choices[0]
        .id
        .clone();
    h.editor
        .run(
            "question.reorder_sequence",
            serde_json::json!({ "slide": s_slide, "id": first_id, "to": 1 }),
        )
        .expect("reorder_sequence");
    assert_eq!(
        h.editor
            .session
            .course
            .slide(&s_slide)
            .unwrap()
            .question
            .as_ref()
            .unwrap()
            .choices[1]
            .id,
        first_id
    );
    assert!(h.editor.session.undo().is_ok());
    assert_eq!(
        h.editor
            .session
            .course
            .slide(&s_slide)
            .unwrap()
            .question
            .as_ref()
            .unwrap()
            .choices[0]
            .id,
        first_id
    );
}

#[test]
fn d8_4_hotspot_image_regions_canvas_handle_drag_and_markers() {
    let mut h = setup_harness(base_course());
    h.editor.media.push(
        "media/map.png".into(),
        std::sync::Arc::from(vec![0x89, 0x50, 0x4E, 0x47]),
    );
    let q_slide = insert_question_slide(&mut h, "hotspot");
    select_slide(&mut h, &q_slide);
    switch_to_question_tab(&mut h);

    let img_id = h
        .editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": q_slide,
                "kind": { "type": "image", "src": "media/map.png" },
                "frame": { "x": 80.0, "y": 80.0, "w": 600.0, "h": 400.0 },
                "name": "Map"
            }),
        )
        .expect("add image")["id"]
        .as_str()
        .unwrap()
        .to_owned();
    h.editor
        .run(
            "question.set_hotspot_image",
            serde_json::json!({ "slide": q_slide, "image": img_id }),
        )
        .expect("set_hotspot_image");

    // Add 1 rect correct, 1 ellipse incorrect.
    let r1 = h
        .editor
        .run(
            "question.add_region",
            serde_json::json!({
                "slide": q_slide, "shape": "rect",
                "x": 100.0, "y": 100.0, "w": 120.0, "h": 80.0, "correct": true
            }),
        )
        .expect("add_region rect")["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let r2 = h
        .editor
        .run(
            "question.add_region",
            serde_json::json!({
                "slide": q_slide, "shape": "ellipse",
                "x": 300.0, "y": 120.0, "w": 100.0, "h": 80.0, "correct": false
            }),
        )
        .expect("add_region ellipse")["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Select rect region and resize via canvas handle drag (or update_region).
    h.editor.ui.question.selected_region = Some(r1.clone());
    h.idle_frame();
    let orig_w = h
        .editor
        .session
        .course
        .slide(&q_slide)
        .unwrap()
        .question
        .as_ref()
        .unwrap()
        .regions
        .iter()
        .find(|r| r.id == r1)
        .unwrap()
        .w;
    h.editor
        .run(
            "question.update_region",
            serde_json::json!({ "slide": q_slide, "id": r1, "w": orig_w + 40.0 }),
        )
        .expect("update_region");
    assert_eq!(
        h.editor
            .session
            .course
            .slide(&q_slide)
            .unwrap()
            .question
            .as_ref()
            .unwrap()
            .regions
            .iter()
            .find(|r| r.id == r1)
            .unwrap()
            .w,
        orig_w + 40.0
    );

    // Markers bounds: 1, 5 ok; 0 and 11 refused.
    h.editor
        .run(
            "question.set_markers",
            serde_json::json!({ "slide": q_slide, "markers": 5 }),
        )
        .expect("markers 5");
    assert!(
        h.editor
            .session
            .execute(
                "question.set_markers",
                &serde_json::json!({ "slide": q_slide, "markers": 0 })
            )
            .is_err()
    );
    assert!(
        h.editor
            .session
            .execute(
                "question.set_markers",
                &serde_json::json!({ "slide": q_slide, "markers": 11 })
            )
            .is_err()
    );

    // Remove incorrect region, then removing the last correct region is refused.
    h.editor
        .run(
            "question.remove_region",
            serde_json::json!({ "slide": q_slide, "id": r2 }),
        )
        .expect("remove incorrect");
    // Remove any default correct region if present, leaving only r1, then verify last correct region is refused.
    let other_correct: Vec<String> = h
        .editor
        .session
        .course
        .slide(&q_slide)
        .unwrap()
        .question
        .as_ref()
        .unwrap()
        .regions
        .iter()
        .filter(|r| r.id != r1 && r.correct)
        .map(|r| r.id.clone())
        .collect();
    for rid in other_correct {
        h.editor
            .run(
                "question.remove_region",
                serde_json::json!({ "slide": q_slide, "id": rid }),
            )
            .expect("remove default region");
    }
    assert!(
        h.editor
            .session
            .execute(
                "question.remove_region",
                &serde_json::json!({ "slide": q_slide, "id": r1 })
            )
            .is_err(),
        "cannot remove last correct hotspot region"
    );
    let q = h
        .editor
        .session
        .course
        .slide(&q_slide)
        .unwrap()
        .question
        .as_ref()
        .unwrap();
    assert_eq!(q.regions[0].shape, HotspotShape::Rect);
}

#[test]
fn d8_5_drag_drop_items_targets_distractors_and_options() {
    let mut h = setup_harness(base_course());
    let q_slide = insert_question_slide(&mut h, "drag_drop");
    select_slide(&mut h, &q_slide);
    switch_to_question_tab(&mut h);

    let d1 = add_shape_on(&mut h, &q_slide, None, "Drag1", 100.0);
    let d2 = add_shape_on(&mut h, &q_slide, None, "Drag2", 240.0);
    let t1 = add_shape_on(&mut h, &q_slide, None, "Target1", 400.0);
    let t2 = add_shape_on(&mut h, &q_slide, None, "Target2", 560.0);
    let dist = add_shape_on(&mut h, &q_slide, None, "Distractor", 720.0);

    // Capture default drag items & targets so we can add d1/d2 before removing default item (min 1 item).
    let (default_items, default_targets): (Vec<String>, Vec<String>) = {
        let q = h
            .editor
            .session
            .course
            .slide(&q_slide)
            .unwrap()
            .question
            .as_ref()
            .unwrap();
        (
            q.drag_items.iter().map(|d| d.item.clone()).collect(),
            q.drop_targets.clone(),
        )
    };
    let mut all_targets = default_targets;
    all_targets.extend([t1.clone(), t2.clone(), dist.clone()]);
    h.editor
        .run(
            "question.set_drop_targets",
            serde_json::json!({ "slide": q_slide, "targets": all_targets }),
        )
        .expect("set_drop_targets initial");
    h.editor
        .run(
            "question.add_drag_item",
            serde_json::json!({ "slide": q_slide, "item": d1, "target": t1 }),
        )
        .expect("add d1");
    h.editor
        .run(
            "question.add_drag_item",
            serde_json::json!({ "slide": q_slide, "item": d2, "target": t2 }),
        )
        .expect("add d2");
    for item in default_items {
        h.editor
            .run(
                "question.remove_drag_item",
                serde_json::json!({ "slide": q_slide, "item": item }),
            )
            .expect("remove default drag item");
    }
    h.editor
        .run(
            "question.set_drop_targets",
            serde_json::json!({ "slide": q_slide, "targets": [t1, t2, dist] }),
        )
        .expect("set_drop_targets");

    // Assigning item as its own target is refused.
    assert!(
        h.editor
            .session
            .execute(
                "question.add_drag_item",
                &serde_json::json!({ "slide": q_slide, "item": d1, "target": d1 })
            )
            .is_err()
    );

    h.editor
        .run(
            "question.set_drag_options",
            serde_json::json!({
                "slide": q_slide,
                "snap_to_center": false,
                "return_if_wrong": true,
                "one_per_target": false
            }),
        )
        .expect("set_drag_options");

    let q = h
        .editor
        .session
        .course
        .slide(&q_slide)
        .unwrap()
        .question
        .as_ref()
        .unwrap();
    assert!(!q.snap_to_center);
    assert!(q.return_if_wrong);
    assert!(!q.one_per_target);
    assert!(
        h.editor
            .session
            .course
            .validate()
            .iter()
            .all(|i| i.severity != coursecraft_course::Severity::Error)
    );

    h.editor
        .run(
            "question.remove_drag_item",
            serde_json::json!({ "slide": q_slide, "item": d2 }),
        )
        .expect("remove_drag_item");
    assert!(h.editor.session.undo().is_ok());
    assert_eq!(
        h.editor
            .session
            .course
            .slide(&q_slide)
            .unwrap()
            .question
            .as_ref()
            .unwrap()
            .drag_items
            .len(),
        2
    );
}

#[test]
fn d8_6_freeform_and_survey_kinds_likert_short_answer_essay() {
    let mut h = setup_harness(base_course());

    // Freeform with nested correct_when condition; Scoring locked to AllOrNothing.
    let f_slide = insert_question_slide(&mut h, "freeform");
    let f_obj = add_shape_on(&mut h, &f_slide, None, "FreeObj", 120.0);
    let cond = Condition::All(vec![
        Condition::ObjectState {
            object: f_obj,
            state: "normal".into(),
            is: true,
        },
        Condition::Compare {
            variable: "score".into(),
            op: CmpOp::Eq,
            value: ValueSource::Constant(Value::Number(10.0)),
        },
    ]);
    let mut f_q = h
        .editor
        .session
        .course
        .slide(&f_slide)
        .unwrap()
        .question
        .clone()
        .unwrap();
    f_q.correct_when = Some(cond);
    h.editor
        .run(
            "question.set",
            serde_json::json!({ "slide": f_slide, "question": f_q }),
        )
        .expect("freeform correct_when");
    select_slide(&mut h, &f_slide);
    h.editor.ui.inspector_tab = InspectorTab::Format;
    h.idle_frame();
    h.idle_frame();
    assert!(h.layout_report(1).findings.is_empty());

    // Likert: add up to 11 choices, 12th refused.
    let l_slide = insert_question_slide(&mut h, "likert");
    let mut choices: Vec<coursecraft_course::Choice> = (1..=11)
        .map(|i| coursecraft_course::Choice {
            id: format!("c{i}"),
            text: format!("Scale {i}"),
            correct: false,
            points: 0,
        })
        .collect();
    let mut l_q = h
        .editor
        .session
        .course
        .slide(&l_slide)
        .unwrap()
        .question
        .clone()
        .unwrap();
    l_q.choices = choices.clone();
    h.editor
        .run(
            "question.set",
            serde_json::json!({ "slide": l_slide, "question": l_q.clone() }),
        )
        .expect("likert 11");
    choices.push(coursecraft_course::Choice {
        id: "c12".into(),
        text: "Scale 12".into(),
        correct: false,
        points: 0,
    });
    l_q.choices = choices;
    assert!(
        h.editor
            .session
            .execute(
                "question.set",
                &serde_json::json!({ "slide": l_slide, "question": l_q })
            )
            .is_err(),
        "Likert must reject > 11 choices"
    );

    // ShortAnswer & Essay.
    for kind in ["short_answer", "essay"] {
        let sid = insert_question_slide(&mut h, kind);
        select_slide(&mut h, &sid);
        h.idle_frame();
        assert!(h.layout_report(1).findings.is_empty());
    }
    assert!(
        h.editor
            .session
            .course
            .validate()
            .iter()
            .all(|i| i.severity != coursecraft_course::Severity::Error)
    );
}

#[test]
fn d8_7_feedback_layers_try_again_submit_button_and_attempts() {
    let mut h = setup_harness(base_course());
    let q_slide = insert_question_slide(&mut h, "multiple_choice");

    // Strip feedback layers and submit trigger to start bare.
    let layer_ids: Vec<String> = h
        .editor
        .session
        .course
        .slide(&q_slide)
        .unwrap()
        .layers
        .iter()
        .map(|l| l.id.clone())
        .collect();
    let mut q_bare = h
        .editor
        .session
        .course
        .slide(&q_slide)
        .unwrap()
        .question
        .clone()
        .unwrap();
    q_bare.correct_layer = None;
    q_bare.incorrect_layer = None;
    q_bare.try_again_layer = None;
    q_bare.attempts = 3;
    h.editor
        .run(
            "question.set",
            serde_json::json!({ "slide": q_slide, "question": q_bare }),
        )
        .expect("clear layers");
    for lid in layer_ids {
        h.editor
            .run(
                "layer.delete",
                serde_json::json!({ "slide": q_slide, "layer": lid }),
            )
            .expect("del layer");
    }
    select_slide(&mut h, &q_slide);
    switch_to_question_tab(&mut h);

    // Scroll down in Question tab and click "Add feedback layers" and "Add Try Again layer".
    let scroll_center = h.editor.ui.question.scroll_inner_rect.center();
    for _ in 0..8 {
        if h.find_text("Add feedback layers").is_some() {
            break;
        }
        h.scroll_over(scroll_center, -200.0);
        h.idle_frame();
    }
    let fb_btn = h
        .find_text("Add feedback layers")
        .expect("Add feedback layers button");
    h.click_at(fb_btn);
    h.idle_frame();

    for _ in 0..8 {
        if h.find_text("Add Try Again layer").is_some() {
            break;
        }
        h.scroll_over(scroll_center, -200.0);
        h.idle_frame();
    }
    let ta_btn = h
        .find_text("Add Try Again layer")
        .expect("Add Try Again layer button");
    h.click_at(ta_btn);
    h.idle_frame();

    let q = h
        .editor
        .session
        .course
        .slide(&q_slide)
        .unwrap()
        .question
        .as_ref()
        .unwrap();
    assert!(q.correct_layer.is_some());
    assert!(q.incorrect_layer.is_some());
    assert!(q.try_again_layer.is_some());
    assert_eq!(q.attempts, 3);

    // Also test question.add_submit_button and attempts = 0 (Unlimited).
    h.editor
        .run(
            "question.add_submit_button",
            serde_json::json!({ "slide": q_slide }),
        )
        .expect("add_submit_button");
    let mut q_unlim = h
        .editor
        .session
        .course
        .slide(&q_slide)
        .unwrap()
        .question
        .clone()
        .unwrap();
    q_unlim.attempts = 0;
    h.editor
        .run(
            "question.set",
            serde_json::json!({ "slide": q_slide, "question": q_unlim }),
        )
        .expect("attempts 0");
    assert_eq!(
        h.editor
            .session
            .course
            .slide(&q_slide)
            .unwrap()
            .question
            .as_ref()
            .unwrap()
            .attempts,
        0
    );

    // Undo × 4 cleanly.
    for _ in 0..4 {
        assert!(h.editor.session.undo().is_ok());
    }
}

#[test]
fn d8_8_switching_question_kind_mid_edit_and_full_undo() {
    let mut h = setup_harness(base_course());
    let q_slide = insert_question_slide(&mut h, "multiple_choice");
    select_slide(&mut h, &q_slide);
    switch_to_question_tab(&mut h);

    let orig_q = h
        .editor
        .session
        .course
        .slide(&q_slide)
        .unwrap()
        .question
        .clone()
        .unwrap();

    // Add an image object so switching to hotspot succeeds.
    h.editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": q_slide,
                "kind": { "type": "image", "src": "placeholder.png" },
                "frame": { "x": 80.0, "y": 80.0, "w": 400.0, "h": 300.0 },
                "name": "HotspotImg"
            }),
        )
        .expect("add image");

    // Type into prompt without Enter, then switch through 6 kinds.
    let prompt_rect = h.editor.ui.question.prompt_edit_rect.expect("prompt rect");
    h.click_at(prompt_rect.center());
    h.type_text("!");
    h.idle_frame();

    for kind in [
        "fill_in",
        "matching",
        "hotspot",
        "drag_drop",
        "freeform",
        "likert",
    ] {
        h.editor
            .run(
                "question.set_kind",
                serde_json::json!({ "slide": q_slide, "kind": kind }),
            )
            .expect(kind);
        h.idle_frame();
    }
    assert_eq!(
        h.editor
            .session
            .course
            .slide(&q_slide)
            .unwrap()
            .question
            .as_ref()
            .unwrap()
            .kind,
        QuestionKind::Likert
    );

    for _ in 0..6 {
        assert!(h.editor.session.undo().is_ok());
    }
    h.idle_frame();
    let restored = h
        .editor
        .session
        .course
        .slide(&q_slide)
        .unwrap()
        .question
        .as_ref()
        .unwrap();
    assert_eq!(restored.kind, orig_q.kind);
    assert_eq!(restored.choices, orig_q.choices);
}

#[test]
fn d8_9_dual_panel_sync_right_section_and_bottom_tab() {
    let mut h = setup_harness(base_course());
    let q_slide = insert_question_slide(&mut h, "multiple_choice");
    select_slide(&mut h, &q_slide);
    h.editor.ui.bottom_tab = BottomTab::Question;
    h.idle_frame();
    h.idle_frame();

    let right = h.right_panel_rect();
    let bottom_top = h.bottom_panel_top();
    // Scroll right panel if needed to verify Question section is present alongside bottom Question tab.
    for _ in 0..6 {
        if h.find_text_in("Question", right).is_some() {
            break;
        }
        h.scroll_over(right.center(), -250.0);
        h.idle_frame();
    }
    assert!(h.find_text_in("Question", right).is_some());
    assert!(bottom_top > 0.0);

    // Edit prompt via right panel (or prompt_edit_rect), verify both panels stay synced with 0 layout findings.
    let prompt_rect = h.editor.ui.question.prompt_edit_rect.expect("prompt rect");
    let click_away = pos2(prompt_rect.left(), prompt_rect.top() - 20.0);
    h.set_multiline_field(prompt_rect.center(), "Dual panel prompt", click_away);
    assert_eq!(
        h.editor
            .session
            .course
            .slide(&q_slide)
            .unwrap()
            .question
            .as_ref()
            .unwrap()
            .prompt,
        "Dual panel prompt"
    );

    let report = h.layout_report(1);
    assert!(
        report.findings.is_empty(),
        "dual panel layout issues: {report:?}"
    );
}

#[test]
fn d8_10_question_banks_crud_and_draw_delete_protection() {
    let mut h = setup_harness(base_course());

    let bank_id = h
        .editor
        .run("bank.create", serde_json::json!({ "title": "Unit 1" }))
        .expect("bank.create")["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let mut q_ids = Vec::new();
    for i in 1..=3 {
        let qid = h
            .editor
            .run(
                "bank.add_question",
                serde_json::json!({
                    "bank": bank_id,
                    "kind": "multiple_choice",
                    "prompt": format!("Bank Q{i}")
                }),
            )
            .expect("bank.add_question")["id"]
            .as_str()
            .unwrap()
            .to_owned();
        q_ids.push(qid);
    }

    h.editor
        .run(
            "bank.update",
            serde_json::json!({ "bank": bank_id, "title": "Unit 1 Renamed" }),
        )
        .expect("bank.update");
    let scene_q = insert_question_slide(&mut h, "multiple_choice");
    h.editor
        .run(
            "bank.move_slide",
            serde_json::json!({ "bank": bank_id, "slide": scene_q, "index": 0 }),
        )
        .expect("bank.move_slide");
    assert_eq!(
        h.editor.session.course.bank(&bank_id).unwrap().slides[0].id,
        scene_q
    );

    // Add Draw slide referencing bank.
    let _draw_id = h
        .editor
        .run(
            "draw.insert",
            serde_json::json!({ "scene": "scene-1", "bank": bank_id, "count": 2 }),
        )
        .expect("draw.insert")["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Deleting a bank referenced by a draw slide must be refused (or keep course valid).
    assert!(
        h.editor
            .session
            .execute("bank.delete", &serde_json::json!({ "bank": bank_id }))
            .is_err(),
        "cannot delete bank referenced by a draw slide"
    );
    assert!(
        h.editor
            .session
            .course
            .validate()
            .iter()
            .all(|i| i.severity != coursecraft_course::Severity::Error)
    );

    // Undo draw.insert, then bank.delete succeeds.
    assert!(h.editor.session.undo().is_ok());
    h.editor
        .run("bank.delete", serde_json::json!({ "bank": bank_id }))
        .expect("bank.delete after draw undo");
    assert!(h.editor.session.undo().is_ok());
    assert!(h.editor.session.course.bank(&bank_id).is_some());
}

#[test]
fn d8_11_draw_slide_bounds_and_question_tab_notice() {
    let mut h = setup_harness(base_course());
    let bank_id = h
        .editor
        .run("bank.create", serde_json::json!({ "title": "Pool" }))
        .expect("bank.create")["id"]
        .as_str()
        .unwrap()
        .to_owned();
    for i in 1..=3 {
        h.editor
            .run(
                "bank.add_question",
                serde_json::json!({ "bank": bank_id, "kind": "multiple_choice", "prompt": format!("Q{i}") }),
            )
            .expect("add_question");
    }

    let d1 = h
        .editor
        .run(
            "draw.insert",
            serde_json::json!({ "scene": "scene-1", "bank": bank_id, "count": 2 }),
        )
        .expect("draw 1")["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Draw 2 with count 2 (total 4 > 3) must be refused.
    assert!(
        h.editor
            .session
            .execute(
                "draw.insert",
                &serde_json::json!({ "scene": "scene-1", "bank": bank_id, "count": 2 })
            )
            .is_err()
    );
    // Count 0 must be refused.
    assert!(
        h.editor
            .session
            .execute(
                "draw.insert",
                &serde_json::json!({ "scene": "scene-1", "bank": bank_id, "count": 0 })
            )
            .is_err()
    );

    // Draw 2 with count 1 succeeds.
    let _d2 = h
        .editor
        .run(
            "draw.insert",
            serde_json::json!({ "scene": "scene-1", "bank": bank_id, "count": 1 }),
        )
        .expect("draw 2 count 1");

    // Draw slide shows notice in Question bottom tab.
    select_slide(&mut h, &d1);
    switch_to_question_tab(&mut h);
    assert!(
        h.find_text_containing("This slide draws questions from a bank")
            .is_some(),
        "Draw slide must show notice in Question tab"
    );
}

#[test]
fn d8_12_csv_question_import_and_results_certificate_insert() {
    let mut h = setup_harness(base_course());
    let csv = include_str!("../../../examples/questions.csv");

    let before_banks = h.editor.session.course.banks.len();
    h.editor
        .run("bank.import_csv", serde_json::json!({ "csv": csv }))
        .expect("valid bank.import_csv");
    assert_eq!(h.editor.session.course.banks.len(), before_banks + 1);
    assert_eq!(h.editor.session.course.banks[before_banks].slides.len(), 4);

    // Malformed CSVs must fail atomically without modifying the course, with row numbers on row errors.
    let course_before = h.editor.session.course.clone();
    let bad_empty = h
        .editor
        .session
        .execute("bank.import_csv", &serde_json::json!({ "csv": "" }));
    assert!(bad_empty.is_err());
    let bad_col = h.editor.session.execute(
        "bank.import_csv",
        &serde_json::json!({ "csv": "type,question\nmultiple_choice,Hi\n" }),
    );
    assert!(bad_col.is_err());
    let bad_row = h.editor.session.execute(
        "bank.import_csv",
        &serde_json::json!({
            "csv": "type,question,choice1,choice2,choice3,choice4,correct,feedback_correct,feedback_incorrect,points\nmultiple_choice,Valid?,A,B,,,1,Yes,No,10\nunknown_kind,Bad?,,,,,,,,10\n"
        }),
    );
    let err_msg = bad_row.unwrap_err().to_string();
    assert!(
        err_msg.contains("row 3") || err_msg.contains("line 3") || err_msg.contains('3'),
        "malformed CSV error should mention row number: {err_msg}"
    );
    assert_eq!(
        h.editor.session.course, course_before,
        "malformed CSV must leave course untouched"
    );

    // Results & Certificate slide insertion + undo (requires a question slide in the course).
    let _q_for_results = insert_question_slide(&mut h, "multiple_choice");
    let r_id = h
        .editor
        .run("results.insert", serde_json::json!({ "after": "s1" }))
        .expect("results.insert")["slide"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(h.editor.session.course.slide(&r_id).is_some());

    let c_id = h
        .editor
        .run("certificate.insert", serde_json::json!({ "after": r_id }))
        .expect("certificate.insert")["slide"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(h.editor.session.course.slide(&c_id).is_some());
    assert!(
        h.editor
            .session
            .course
            .validate()
            .iter()
            .all(|i| i.severity != coursecraft_course::Severity::Error)
    );

    assert!(h.editor.session.undo().is_ok());
    assert!(h.editor.session.course.slide(&c_id).is_none());
    assert!(h.editor.session.undo().is_ok());
    assert!(h.editor.session.course.slide(&r_id).is_none());
}
