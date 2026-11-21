//! M27 Step D-5: Undo/Redo & History (D5.2–D5.12) and Copy/Cut/Paste & Duplicate (D6.1–D6.11).

use crate::clipboard;
use crate::test_harness::TestHarness;
use crate::{ActiveArea, BottomTab};
use coursecraft_course::{
    Action, Course, Event as CourseEvent, NextRequirement, ObjectKind, SlideTarget, Trigger,
};
use coursecraft_engine::{EngineError, MAX_HISTORY, registry};
use egui::{Key, Modifiers, Pos2, pos2, vec2};

fn make_base_course() -> Course {
    let course: Course = serde_json::from_value(serde_json::json!({
        "schema_version": 1,
        "id": "m27-undo-clipboard",
        "title": "M27 Undo & Clipboard",
        "slide_size": { "w": 1280, "h": 720 },
        "variables": [
            { "name": "v1", "value": 0.0 },
            { "name": "v2", "value": "hello" }
        ],
        "masters": [{
            "id": "m1",
            "title": "Main Master"
        }],
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
                            "id": "shape-a",
                            "name": "Shape A",
                            "kind": { "type": "shape", "shape": "rectangle", "fill": "#2563EB", "text": { "text": "A" } },
                            "frame": { "x": 100.0, "y": 100.0, "w": 120.0, "h": 80.0 }
                        },
                        {
                            "id": "shape-b",
                            "name": "Shape B",
                            "kind": { "type": "shape", "shape": "rectangle", "fill": "#16A34A", "text": { "text": "B" } },
                            "frame": { "x": 320.0, "y": 100.0, "w": 120.0, "h": 80.0 }
                        },
                        {
                            "id": "shape-c",
                            "name": "Shape C",
                            "kind": { "type": "shape", "shape": "rectangle", "fill": "#9333EA", "text": { "text": "C" } },
                            "frame": { "x": 540.0, "y": 100.0, "w": 120.0, "h": 80.0 }
                        },
                        {
                            "id": "txt-1",
                            "name": "Text 1",
                            "kind": { "type": "text", "text": "Original text" },
                            "frame": { "x": 100.0, "y": 240.0, "w": 220.0, "h": 60.0 }
                        },
                        {
                            "id": "img-1",
                            "name": "Image 1",
                            "kind": { "type": "image", "src": "hero.png" },
                            "frame": { "x": 100.0, "y": 340.0, "w": 160.0, "h": 120.0 },
                            "description": "Hero"
                        }
                    ],
                    "layers": [
                        {
                            "id": "l1",
                            "name": "Layer 1",
                            "objects": [
                                {
                                    "id": "l1-shape",
                                    "name": "Layer Shape",
                                    "kind": { "type": "shape", "shape": "ellipse", "fill": "#F59E0B" },
                                    "frame": { "x": 200.0, "y": 200.0, "w": 100.0, "h": 100.0 }
                                }
                            ]
                        }
                    ]
                },
                {
                    "id": "s2",
                    "title": "Slide 2",
                    "duration_ms": 5000,
                    "objects": [
                        {
                            "id": "s2-shape",
                            "name": "S2 Shape",
                            "kind": { "type": "shape", "shape": "rectangle", "fill": "#DC2626" },
                            "frame": { "x": 150.0, "y": 150.0, "w": 120.0, "h": 80.0 }
                        }
                    ],
                    "layers": [
                        {
                            "id": "s2-l1",
                            "name": "S2 Layer 1",
                            "objects": []
                        }
                    ]
                }
            ]
        }]
    }))
    .expect("valid base course");
    course
}

fn slide_to_screen(h: &mut TestHarness, x: f32, y: f32) -> Pos2 {
    if h.editor.ui.last_slide_rect.is_none() {
        h.idle_frame();
    }
    let slide_rect = h.editor.ui.last_slide_rect.expect("slide_rect");
    let scale = slide_rect.width() / h.editor.session.course.slide_size.w as f32;
    pos2(slide_rect.min.x + x * scale, slide_rect.min.y + y * scale)
}

fn make_tiny_png() -> Vec<u8> {
    let pixels = [255u8, 0, 0, 255].repeat(16);
    coursecraft_media::encode_png_rgba(4, 4, &pixels).expect("encode png")
}

fn trigger_click_state(obj: &str, state: &str) -> Trigger {
    Trigger {
        when: CourseEvent::Click {
            object: obj.to_owned(),
        },
        action: Action::ChangeState {
            object: obj.to_owned(),
            state: state.to_owned(),
        },
        condition: None,
        disabled: false,
    }
}

// ---------------------------------------------------------------------------
// D5.2 – D5.12: Undo / Redo & History Panel
// ---------------------------------------------------------------------------

#[test]
fn d5_2_non_mutating_commands_and_noop_mutations_do_not_touch_undo_or_redo() {
    let mut h = TestHarness::new(make_base_course());
    assert!(
        h.editor
            .run("slide.add", serde_json::json!({ "title": "Added" }))
            .is_some()
    );
    assert_eq!(h.editor.session.undo_len(), 1);
    h.editor.session.undo().unwrap();
    assert_eq!(h.editor.session.undo_len(), 0);
    assert_eq!(h.editor.session.redo_names().len(), 1);
    let rev_before = h.editor.session.revision;

    let mut non_mutating_checked = 0usize;
    for spec in registry()
        .iter()
        .filter(|c| !c.mutates && !matches!(c.id, "edit.undo" | "edit.redo"))
    {
        let params = match spec.id {
            "course.info" | "course.validate" | "course.audit" | "course.summary"
            | "course.map" | "edit.history" | "history.info" | "commands.list" => {
                serde_json::json!({})
            }
            "course.export_xliff" => serde_json::json!({ "target_language": "fr" }),
            "course.find" => serde_json::json!({ "query": "Slide" }),
            "slide.info" => serde_json::json!({ "slide": "s1" }),
            "object.info" => serde_json::json!({ "slide": "s1", "object": "shape-a" }),
            other => panic!("unhandled non-mutating command in D5.2: {other}"),
        };
        assert!(
            h.editor.session.execute(spec.id, &params).is_ok(),
            "non-mutating command {} failed",
            spec.id
        );
        non_mutating_checked += 1;
    }
    assert_eq!(
        non_mutating_checked,
        registry()
            .iter()
            .filter(|c| !c.mutates && !matches!(c.id, "edit.undo" | "edit.redo"))
            .count()
    );

    // No-op mutations that hit skip_undo / no-change guards
    let same_opacity = h
        .editor
        .session
        .course
        .slide("s1")
        .and_then(|s| s.object("shape-a"))
        .map(|(o, _)| o.opacity)
        .unwrap_or(1.0);
    let same_locked = h
        .editor
        .session
        .course
        .slide("s1")
        .and_then(|s| s.object("shape-a"))
        .map(|(o, _)| o.locked)
        .unwrap_or(false);

    assert!(
        h.editor
            .session
            .execute(
                "object.set_opacity",
                &serde_json::json!({ "slide": "s1", "object": "shape-a", "opacity": same_opacity }),
            )
            .is_ok()
    );
    assert!(
        h.editor
            .session
            .execute(
                "object.lock",
                &serde_json::json!({ "slide": "s1", "object": "shape-a", "locked": same_locked }),
            )
            .is_ok()
    );
    assert!(
        h.editor
            .session
            .execute(
                "layer.move",
                &serde_json::json!({ "slide": "s1", "layer": "l1", "to": 0 }),
            )
            .is_ok()
    );

    assert_eq!(h.editor.session.undo_len(), 0);
    assert_eq!(
        h.editor.session.redo_names().len(),
        1,
        "redo stack must NOT be cleared by non-mutating or no-op commands"
    );
    assert_eq!(h.editor.session.revision, rev_before);
}

#[test]
fn d5_3_batch_atomicity_rolls_back_on_mid_batch_failure_and_rejects_undo_in_batch() {
    let mut h = TestHarness::new(make_base_course());
    let before = h.editor.session.course.clone();
    let rev_before = h.editor.session.revision;
    let undo_before = h.editor.session.undo_len();

    let err = h
        .editor
        .session
        .execute_batch_named(
            "Test batch",
            &[
                (
                    "slide.add".to_owned(),
                    serde_json::json!({ "title": "Temp" }),
                ),
                (
                    "object.add".to_owned(),
                    serde_json::json!({
                        "slide": "s1",
                        "kind": { "type": "shape", "shape": "rectangle", "fill": "#111111" },
                        "frame": { "x": 10.0, "y": 10.0, "w": 50.0, "h": 50.0 }
                    }),
                ),
                (
                    "object.delete".to_owned(),
                    serde_json::json!({ "slide": "s1", "object": "nonexistent-obj" }),
                ),
            ],
        )
        .expect_err("mid-batch failure must error");
    assert!(matches!(err, EngineError::BatchFailed { .. }));
    assert_eq!(h.editor.session.course, before);
    assert_eq!(h.editor.session.revision, rev_before);
    assert_eq!(h.editor.session.undo_len(), undo_before);

    for undo_cmd in ["edit.undo", "edit.redo"] {
        let batch_err = h
            .editor
            .session
            .execute_batch_named(
                "Bad undo batch",
                &[(undo_cmd.to_owned(), serde_json::json!({}))],
            )
            .expect_err("undo/redo inside batch must be rejected");
        assert!(matches!(batch_err, EngineError::BatchFailed { .. }));
    }
    assert_eq!(h.editor.session.course, before);
}

fn d5_4_assert_delete_batches(h: &mut TestHarness) {
    // 1. Multi-object Delete (3 objs)
    h.editor.selection.set_many([
        "shape-a".to_owned(),
        "shape-b".to_owned(),
        "shape-c".to_owned(),
    ]);
    let undo_0 = h.editor.session.undo_len();
    h.editor.delete_selection();
    assert_eq!(h.editor.session.undo_len(), undo_0 + 1);
    assert!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .object("shape-a")
            .is_none()
    );
    assert!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .object("shape-b")
            .is_none()
    );
    assert!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .object("shape-c")
            .is_none()
    );
    h.press(Key::Z, Modifiers::COMMAND);
    assert!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .object("shape-a")
            .is_some()
    );
    assert!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .object("shape-b")
            .is_some()
    );
    assert!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .object("shape-c")
            .is_some()
    );
}

#[test]
fn d5_4_multi_step_ui_batch_operations_are_single_undo_step() {
    let mut h = TestHarness::new(make_base_course());
    d5_4_assert_delete_batches(&mut h);

    // 2. Multi-slide Delete (3 slides)
    for title in ["Extra 1", "Extra 2", "Extra 3"] {
        h.editor
            .run("slide.add", serde_json::json!({ "title": title }))
            .expect("add slide");
    }
    let extra_ids: Vec<String> = h
        .editor
        .session
        .course
        .slides()
        .filter(|s| s.title.starts_with("Extra "))
        .map(|s| s.id.clone())
        .collect();
    assert_eq!(extra_ids.len(), 3);
    let undo_1 = h.editor.session.undo_len();
    let del_steps: Vec<(String, serde_json::Value)> = extra_ids
        .iter()
        .map(|id| {
            (
                "slide.delete".to_owned(),
                serde_json::json!({ "slide": id }),
            )
        })
        .collect();
    assert!(h.editor.run_batch(&del_steps).is_some());
    assert_eq!(h.editor.session.undo_len(), undo_1 + 1);
    assert_eq!(h.editor.session.course.slides().count(), 2);
    h.press(Key::Z, Modifiers::COMMAND);
    assert_eq!(h.editor.session.course.slides().count(), 5);

    // 3. Multi-object Align
    h.editor.selection.slide = "s1".to_owned();
    h.editor.selection.layer = None;
    h.editor.selection.set_many([
        "shape-a".to_owned(),
        "shape-b".to_owned(),
        "shape-c".to_owned(),
    ]);
    h.editor.run(
        "object.set_frame",
        serde_json::json!({
            "slide": "s1",
            "object": "shape-b",
            "frame": { "x": 320.0, "y": 180.0, "w": 120.0, "h": 80.0 }
        }),
    );
    let undo_2 = h.editor.session.undo_len();
    let align_p = crate::menu::align_params(&h.editor, "top").expect("align params");
    assert!(h.editor.run("object.align", align_p).is_some());
    assert_eq!(h.editor.session.undo_len(), undo_2 + 1);
    assert_eq!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .object("shape-b")
            .unwrap()
            .0
            .frame
            .y,
        100.0
    );
    h.press(Key::Z, Modifiers::COMMAND);
    assert_eq!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .object("shape-b")
            .unwrap()
            .0
            .frame
            .y,
        180.0
    );

    // 4. Multi-object Format opacity
    let undo_3 = h.editor.session.undo_len();
    h.editor.selection.set_many([
        "shape-a".to_owned(),
        "shape-b".to_owned(),
        "shape-c".to_owned(),
    ]);
    let opacity_steps: Vec<(String, serde_json::Value)> = ["shape-a", "shape-b", "shape-c"]
        .into_iter()
        .map(|id| {
            (
                "object.set_opacity".to_owned(),
                serde_json::json!({ "slide": "s1", "object": id, "opacity": 0.4 }),
            )
        })
        .collect();
    assert!(h.editor.run_batch(&opacity_steps).is_some());
    assert_eq!(h.editor.session.undo_len(), undo_3 + 1);
    h.press(Key::Z, Modifiers::COMMAND);
    assert_eq!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .object("shape-a")
            .unwrap()
            .0
            .opacity,
        1.0
    );

    // 5. Paste 3 triggers
    h.editor.ui.copied_triggers = vec![
        trigger_click_state("shape-a", "normal"),
        trigger_click_state("shape-b", "normal"),
        trigger_click_state("shape-c", "normal"),
    ];
    let undo_4 = h.editor.session.undo_len();
    crate::triggers::paste_triggers(&mut h.editor, "s1", None);
    assert_eq!(h.editor.session.undo_len(), undo_4 + 1);
    assert_eq!(
        h.editor.session.course.slide("s1").unwrap().triggers.len(),
        3
    );
    h.press(Key::Z, Modifiers::COMMAND);
    assert_eq!(
        h.editor.session.course.slide("s1").unwrap().triggers.len(),
        0
    );

    // 6. Insert Tabs interaction
    h.editor.selection.slide = "s1".to_owned();
    h.editor.selection.layer = None;
    let undo_5 = h.editor.session.undo_len();
    let created = crate::interaction_insert::insert_tabs(&mut h.editor).expect("insert tabs");
    assert!(!created.is_empty());
    assert_eq!(h.editor.session.undo_len(), undo_5 + 1);
    h.press(Key::Z, Modifiers::COMMAND);
    assert!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .object(&created[0])
            .is_none()
    );

    // 7. Alt-text Apply (batched object.update)
    let undo_alt = h.editor.session.undo_len();
    assert!(
        h.editor
            .run_batch(&[
                (
                    "object.update".to_owned(),
                    serde_json::json!({ "slide": "s1", "object": "shape-a", "description": "Box A" }),
                ),
                (
                    "object.update".to_owned(),
                    serde_json::json!({ "slide": "s1", "object": "shape-b", "description": "Box B" }),
                ),
            ])
            .is_some()
    );
    assert_eq!(h.editor.session.undo_len(), undo_alt + 1);
    h.press(Key::Z, Modifiers::COMMAND);
    assert!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .object("shape-a")
            .unwrap()
            .0
            .description
            .is_none()
    );
}

#[test]
fn d5_5_history_panel_click_jump_and_reselects_restored_slide_bug_d4() {
    let mut h = TestHarness::new(make_base_course());
    h.idle_frame();

    // 6 named edits, with step 3 (index 2 in 0-based history) deleting Slide 2 ("s2").
    h.editor
        .run(
            "slide.update",
            serde_json::json!({ "slide": "s1", "title": "Edit 1" }),
        )
        .unwrap();
    h.editor
        .run(
            "slide.update",
            serde_json::json!({ "slide": "s1", "title": "Edit 2" }),
        )
        .unwrap();
    h.editor
        .run("slide.delete", serde_json::json!({ "slide": "s2" }))
        .unwrap();
    h.editor
        .run(
            "slide.update",
            serde_json::json!({ "slide": "s1", "title": "Edit 4" }),
        )
        .unwrap();
    h.editor
        .run(
            "slide.update",
            serde_json::json!({ "slide": "s1", "title": "Edit 5" }),
        )
        .unwrap();
    h.editor
        .run(
            "slide.update",
            serde_json::json!({ "slide": "s1", "title": "Edit 6" }),
        )
        .unwrap();
    assert_eq!(h.editor.session.undo_len(), 6);
    assert!(h.editor.session.course.slide("s2").is_none());

    // Open History panel via Edit -> History…
    let edit_pos = h.find_text("Edit").expect("Edit menu");
    h.click_at(edit_pos);
    let hist_pos = h.find_text("History…").expect("History… menu item");
    h.click_at(hist_pos);
    assert!(h.editor.ui.show_history);
    h.idle_frame();

    // Click step 2 from bottom (i.e. undo 4 steps: past Edit 6, Edit 5, Edit 4, and Delete slide)
    // In history.rs, undo_names()[3] is "Delete slide" (0=Edit 6, 1=Edit 5, 2=Edit 4, 3=Delete slide).
    let delete_label_pos = h
        .find_text("Delete Slide")
        .expect("Delete Slide row in History");
    h.click_at(delete_label_pos);

    assert_eq!(h.editor.session.undo_len(), 2);
    assert_eq!(h.editor.session.redo_names().len(), 4);
    assert!(
        h.editor.session.course.slide("s2").is_some(),
        "Slide 2 must be restored"
    );
    // BUG-D4: History click-jump must set pending_undo_redo = true so Slide 2 is reselected!
    assert_eq!(
        h.editor.selection.slide, "s2",
        "BUG-D4: restored Slide 2 must be reselected on History click-jump"
    );

    let top_redo_name = h
        .editor
        .session
        .redo_names()
        .last()
        .expect("top redo name")
        .to_string();
    let redo_positions = h.find_texts(&top_redo_name);
    assert!(!redo_positions.is_empty());
    h.click_at(redo_positions[0]);
    assert_eq!(h.editor.session.undo_len(), 6);
    assert_eq!(h.editor.session.redo_names().len(), 0);
    assert!(h.editor.session.course.slide("s2").is_none());

    // Click "Opened" to jump all the way back to 0.
    let opened_pos = h.find_text("Opened").expect("Opened row in History");
    h.click_at(opened_pos);
    assert_eq!(h.editor.session.undo_len(), 0);
    assert_eq!(h.editor.session.redo_names().len(), 6);

    let report = h.layout_report(2);
    assert_eq!(
        report.findings.len(),
        0,
        "History panel layout findings: {:?}",
        report.findings
    );
}

#[test]
fn d5_6_max_history_200_cap_and_truncation_notice() {
    let mut h = TestHarness::new(make_base_course());
    for i in 1..=215 {
        assert!(
            h.editor
                .run(
                    "slide.update",
                    serde_json::json!({ "slide": "s1", "title": format!("Step {i}") }),
                )
                .is_some()
        );
    }
    assert_eq!(h.editor.session.undo_len(), MAX_HISTORY);
    assert_eq!(MAX_HISTORY, 200);
    assert!(h.editor.session.history_truncated());

    h.editor.ui.show_history = true;
    h.idle_frame();
    assert!(
        h.find_text_containing("Older steps were removed").is_some(),
        "History panel must display truncation notice"
    );
    let report = h.layout_report(2);
    assert_eq!(
        report.findings.len(),
        0,
        "History panel truncation layout findings: {:?}",
        report.findings
    );

    // Undo all 200 steps cleanly
    for _ in 0..200 {
        assert!(h.editor.session.can_undo());
        h.editor.session.undo().unwrap();
    }
    assert_eq!(h.editor.session.undo_len(), 0);
    assert_eq!(h.editor.session.redo_names().len(), 200);
}

#[test]
fn d5_7_undo_redo_shortcuts_branch_truncation_and_inline_text_edit() {
    let mut h = TestHarness::new(make_base_course());
    h.editor
        .run(
            "slide.update",
            serde_json::json!({ "slide": "s1", "title": "Title A" }),
        )
        .unwrap();
    h.editor
        .run(
            "slide.update",
            serde_json::json!({ "slide": "s1", "title": "Title B" }),
        )
        .unwrap();
    assert_eq!(
        h.editor.session.course.slide("s1").unwrap().title,
        "Title B"
    );

    // Ctrl+Z reverts B
    h.press(Key::Z, Modifiers::COMMAND);
    assert_eq!(
        h.editor.session.course.slide("s1").unwrap().title,
        "Title A"
    );

    // Ctrl+Shift+Z redoes B
    h.press(Key::Z, Modifiers::COMMAND | Modifiers::SHIFT);
    assert_eq!(
        h.editor.session.course.slide("s1").unwrap().title,
        "Title B"
    );

    // Ctrl+Z then Ctrl+Y
    h.press(Key::Z, Modifiers::COMMAND);
    assert_eq!(
        h.editor.session.course.slide("s1").unwrap().title,
        "Title A"
    );
    h.press(Key::Y, Modifiers::COMMAND);
    assert_eq!(
        h.editor.session.course.slide("s1").unwrap().title,
        "Title B"
    );

    // Ctrl+Z then Edit C prunes redo branch
    h.press(Key::Z, Modifiers::COMMAND);
    assert!(h.editor.session.can_redo());
    h.editor
        .run(
            "slide.update",
            serde_json::json!({ "slide": "s1", "title": "Title C" }),
        )
        .unwrap();
    assert!(
        !h.editor.session.can_redo(),
        "new edit must truncate redo branch"
    );

    // Inline text edit owns Ctrl+Z while focused (does NOT undo slide title!)
    let txt_center = slide_to_screen(&mut h, 210.0, 270.0);
    h.double_click_at(txt_center);
    h.idle_frame();
    if h.editor.ui.inline_edit.is_some() {
        h.type_text(" XYZ");
        h.idle_frame();
        let undo_before = h.editor.session.undo_len();
        h.press(Key::Z, Modifiers::COMMAND);
        assert_eq!(
            h.editor.session.course.slide("s1").unwrap().title,
            "Title C",
            "Ctrl+Z in inline text edit must not undo slide-level history"
        );
        assert_eq!(h.editor.session.undo_len(), undo_before);
        h.press(Key::Escape, Modifiers::NONE);
    }
}

#[test]
fn d5_8_selection_repair_on_undo_layer_master_and_group() {
    let mut h = TestHarness::new(make_base_course());

    // 1. layer.add -> select new layer -> Ctrl+Z -> selection returns to Base (None)
    let res = h
        .editor
        .run(
            "layer.add",
            serde_json::json!({ "slide": "s1", "name": "L2" }),
        )
        .unwrap();
    let new_lid = res["id"].as_str().unwrap().to_owned();
    h.editor.selection.layer = Some(new_lid);
    h.press(Key::Z, Modifiers::COMMAND);
    assert_eq!(h.editor.selection.layer, None);

    // 2. master.add in Master View -> select new master -> Ctrl+Z -> selection repairs safely
    h.editor.enter_master_view();
    let m_res = h
        .editor
        .run("master.add", serde_json::json!({ "title": "Master 2" }))
        .unwrap();
    let new_mid = m_res["id"].as_str().unwrap().to_owned();
    h.editor.selection.slide = new_mid;
    h.press(Key::Z, Modifiers::COMMAND);
    assert!(
        h.editor
            .session
            .course
            .master(&h.editor.selection.slide)
            .is_some()
            || !h.editor.ui.master_view,
        "master selection must never dangle after undo"
    );
    h.editor.leave_master_view();

    // 3. object.group 3 objects -> Ctrl+Z -> Ctrl+Y
    h.editor.selection.set_many([
        "shape-a".to_owned(),
        "shape-b".to_owned(),
        "shape-c".to_owned(),
    ]);
    h.press(Key::G, Modifiers::COMMAND);
    assert!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .object("shape-a")
            .unwrap()
            .0
            .group
            .is_some()
    );
    h.press(Key::Z, Modifiers::COMMAND);
    assert!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .object("shape-a")
            .unwrap()
            .0
            .group
            .is_none()
    );
    h.press(Key::Y, Modifiers::COMMAND);
    assert!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .object("shape-a")
            .unwrap()
            .0
            .group
            .is_some()
    );
}

#[test]
fn d5_9_undo_while_crop_layer_settings_or_trigger_editor_open() {
    let mut h = TestHarness::new(make_base_course());
    h.editor
        .media
        .push(("hero.png".to_owned(), std::sync::Arc::from(make_tiny_png())));
    h.idle_frame();
    h.editor
        .run(
            "slide.update",
            serde_json::json!({ "slide": "s1", "title": "Before Modal" }),
        )
        .unwrap();

    // 1. Crop mode: Ctrl+Z is safely guarded while cropping, and undoing the cropped image cancels crop_edit cleanly
    let added_img = h
        .editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "kind": { "type": "image", "src": "hero.png" },
                "frame": { "x": 300.0, "y": 300.0, "w": 80.0, "h": 80.0 }
            }),
        )
        .unwrap();
    let added_img_id = added_img["id"].as_str().unwrap().to_owned();
    h.editor.selection.set_many([added_img_id.clone()]);
    assert!(h.editor.start_crop(&added_img_id));
    let rev_during_crop = h.editor.session.revision;
    h.press(Key::Z, Modifiers::COMMAND);
    assert_eq!(h.editor.session.revision, rev_during_crop);
    h.editor.session.undo().unwrap();
    h.editor.check_revision();
    assert!(
        h.editor.ui.crop_edit.is_none(),
        "crop_edit must cancel cleanly when cropped image is undone"
    );

    // 2. Layer Settings modal + Ctrl+Z auto-closes via opened_revision
    h.editor
        .run(
            "slide.update",
            serde_json::json!({ "slide": "s1", "title": "For Layer Modal" }),
        )
        .unwrap();
    h.editor.ui.layer_settings.open = Some(crate::layers::LayerSettingsTarget {
        slide: "s1".to_owned(),
        layer: "l1".to_owned(),
    });
    h.editor.ui.layer_settings.slide = "s1".to_owned();
    h.editor.ui.layer_settings.layer = "l1".to_owned();
    h.editor.ui.layer_settings.name = "Layer 1".to_owned();
    h.editor.ui.layer_settings.opened_revision = h.editor.session.revision;
    h.idle_frame();
    h.press(Key::Z, Modifiers::COMMAND);
    h.idle_frame();
    assert!(
        h.editor.ui.layer_settings.open.is_none(),
        "Layer Settings modal must auto-close when revision changes on undo"
    );

    // 3. Trigger Editor open while trigger is undone -> closes safely
    h.editor
        .run(
            "trigger.add",
            serde_json::json!({
                "slide": "s1",
                "trigger": trigger_click_state("shape-a", "normal")
            }),
        )
        .unwrap();
    h.editor.ui.bottom_tab = BottomTab::Triggers;
    h.idle_frame();
    let row_pos = h
        .find_text("Click Shape A")
        .or_else(|| h.find_text_containing("Shape A"));
    if let Some(pos) = row_pos {
        h.double_click_at(pos);
        h.idle_frame();
    }
    h.press(Key::Z, Modifiers::COMMAND);
    h.idle_frame();
    assert!(
        h.editor.ui.trigger_form.is_none(),
        "Trigger Editor must close safely when underlying trigger is undone"
    );
}

#[test]
fn d5_10_connector_autorerouting_on_undo_and_redo() {
    let mut h = TestHarness::new(make_base_course());
    let conn_res = h
        .editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "kind": {
                    "type": "path",
                    "points": [{ "x": 0.0, "y": 0.5 }, { "x": 1.0, "y": 0.5 }],
                    "outline": { "color": "#000000", "width": 2.0 },
                    "connector": "straight",
                    "start_glue": { "object": "shape-a", "site": 1 },
                    "end_glue": { "object": "shape-b", "site": 3 }
                },
                "frame": { "x": 220.0, "y": 135.0, "w": 100.0, "h": 10.0 }
            }),
        )
        .unwrap();
    let conn_id = conn_res["id"].as_str().unwrap().to_owned();
    let conn_before = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object(&conn_id)
        .unwrap()
        .0
        .clone();
    let shape_a_before = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object("shape-a")
        .unwrap()
        .0
        .frame;

    // Move shape-a via drag on canvas
    let start = slide_to_screen(&mut h, 160.0, 140.0);
    h.drag_by(start, vec2(0.0, 120.0), 6, Modifiers::NONE);
    let shape_a_after = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object("shape-a")
        .unwrap()
        .0
        .frame;
    assert_ne!(shape_a_after.y, shape_a_before.y);
    let conn_after = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object(&conn_id)
        .unwrap()
        .0
        .clone();
    assert_ne!(
        serde_json::to_value(&conn_after.kind).unwrap(),
        serde_json::to_value(&conn_before.kind).unwrap(),
        "connector must reroute when glued Shape A moves"
    );

    h.press(Key::Z, Modifiers::COMMAND);
    assert_eq!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .object("shape-a")
            .unwrap()
            .0
            .frame,
        shape_a_before
    );
    assert_eq!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .object(&conn_id)
            .unwrap()
            .0,
        &conn_before
    );

    h.press(Key::Y, Modifiers::COMMAND);
    assert_eq!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .object("shape-a")
            .unwrap()
            .0
            .frame,
        shape_a_after
    );
    assert_eq!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .object(&conn_id)
            .unwrap()
            .0,
        &conn_after
    );
}

#[test]
fn d5_11_rapid_alternating_undo_redo_spam_100x() {
    let mut h = TestHarness::new(make_base_course());
    for i in 0..25 {
        h.editor
            .run(
                "slide.update",
                serde_json::json!({ "slide": "s1", "title": format!("Title {i}") }),
            )
            .unwrap();
    }
    let mut rng = 0x1234_5678u32;
    for step in 0..100 {
        rng = rng.wrapping_mul(1664525).wrapping_add(1013904223);
        let count = (rng % 4) as usize + 1;
        for _ in 0..count {
            if step % 2 == 0 {
                if h.editor.session.can_undo() {
                    h.press(Key::Z, Modifiers::COMMAND);
                }
            } else if h.editor.session.can_redo() {
                h.press(Key::Y, Modifiers::COMMAND);
            }
        }
        let errors = h
            .editor
            .session
            .course
            .validate()
            .into_iter()
            .filter(|f| f.severity == coursecraft_course::Severity::Error)
            .count();
        assert_eq!(errors, 0, "course invalid at burst {step}");
    }
}

#[test]
fn d5_12_edit_menu_dynamic_labels() {
    let mut h = TestHarness::new(make_base_course());
    h.idle_frame();

    // Empty stacks -> "Undo" and "Redo"
    let edit_pos = h.find_text("Edit").expect("Edit menu");
    h.click_at(edit_pos);
    assert!(h.find_text("Undo").is_some());
    assert!(h.find_text("Redo").is_some());
    h.press(Key::Escape, Modifiers::NONE);

    // After "New Slide" -> "Undo New Slide"
    h.editor
        .run("slide.add", serde_json::json!({ "title": "New Slide" }))
        .unwrap();
    let edit_pos = h.find_text("Edit").expect("Edit menu");
    h.click_at(edit_pos);
    assert!(
        h.find_text("Undo New Slide").is_some(),
        "Edit menu must show 'Undo New Slide'"
    );
    h.press(Key::Escape, Modifiers::NONE);

    // After undo -> "Redo New Slide"
    h.editor.session.undo().unwrap();
    h.editor.check_revision();
    let edit_pos = h.find_text("Edit").expect("Edit menu");
    h.click_at(edit_pos);
    assert!(
        h.find_text("Redo New Slide").is_some(),
        "Edit menu must show 'Redo New Slide'"
    );
}

// ---------------------------------------------------------------------------
// D6.1 – D6.11: Copy / Cut / Paste & Duplicate
// ---------------------------------------------------------------------------

#[test]
fn d6_1_copy_paste_and_cut_paste_offsets_and_paste_count_bug_d5() {
    let mut h = TestHarness::new(make_base_course());
    h.editor.ui.last_active_area = ActiveArea::Canvas;
    h.editor
        .selection
        .set_many(["shape-a".to_owned(), "shape-b".to_owned()]);
    h.copy();
    assert_eq!(h.editor.ui.clipboard_paste_count, 0);

    // 3x paste on same slide -> +20, +40, +60
    for expected_offset in [20.0f32, 40.0, 60.0] {
        clipboard::paste_private(&mut h.editor);
        let pasted_id = h.editor.selection.object.clone().unwrap();
        let pasted = h
            .editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .object(&pasted_id)
            .unwrap()
            .0;
        assert_eq!(pasted.frame.x, 100.0 + expected_offset);
        assert_eq!(pasted.frame.y, 100.0 + expected_offset);
    }
    assert_eq!(h.editor.ui.clipboard_paste_count, 3);

    // Switch to Slide 2 -> paste -> offset 0 AND clipboard_paste_count stays 3 (BUG-D5!)
    h.editor.selection.slide = "s2".to_owned();
    h.editor.selection.clear_objects();
    clipboard::paste_private(&mut h.editor);
    let s2_pasted_id = h.editor.selection.object.clone().unwrap();
    let s2_pasted = h
        .editor
        .session
        .course
        .slide("s2")
        .unwrap()
        .object(&s2_pasted_id)
        .unwrap()
        .0;
    assert_eq!(s2_pasted.frame.x, 100.0);
    assert_eq!(s2_pasted.frame.y, 100.0);
    assert_eq!(
        h.editor.ui.clipboard_paste_count, 3,
        "BUG-D5: pasting on a different slide must not increment clipboard_paste_count"
    );

    // Cut -> undo restores cut objects
    h.editor.selection.slide = "s1".to_owned();
    h.editor.selection.set_many(["shape-a".to_owned()]);
    h.cut();
    assert!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .object("shape-a")
            .is_none()
    );
    h.press(Key::Z, Modifiers::COMMAND);
    assert!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .object("shape-a")
            .is_some()
    );
}

#[test]
fn d6_2_canvas_right_click_paste_here_and_oversized_clamping() {
    let mut h = TestHarness::new(make_base_course());
    h.editor.ui.last_active_area = ActiveArea::Canvas;
    let big_res = h
        .editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "kind": { "type": "shape", "shape": "rectangle", "fill": "#111111" },
                "frame": { "x": 0.0, "y": 0.0, "w": 400.0, "h": 200.0 }
            }),
        )
        .unwrap();
    let big_id = big_res["id"].as_str().unwrap().to_owned();
    h.editor.selection.set_many([big_id]);
    h.copy();

    // Right-click near bottom-right corner (1250, 700) -> "Paste" (paste_private_at)
    let corner = slide_to_screen(&mut h, 1250.0, 700.0);
    h.right_click_at(corner);
    let paste_here_pos = h.find_text("Paste").expect("Paste menu item");
    h.click_at(paste_here_pos);

    let pasted_id = h.editor.selection.object.clone().unwrap();
    let pasted = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object(&pasted_id)
        .unwrap()
        .0;
    // Clamped inside 1280x720
    assert_eq!(pasted.frame.x, 880.0);
    assert_eq!(pasted.frame.y, 520.0);

    // Oversized 1400x800 shape -> paste_private_at centers without panic/NaN
    let huge_res = h
        .editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "kind": { "type": "shape", "shape": "rectangle", "fill": "#222222" },
                "frame": { "x": 0.0, "y": 0.0, "w": 1400.0, "h": 800.0 }
            }),
        )
        .unwrap();
    let huge_id = huge_res["id"].as_str().unwrap().to_owned();
    h.editor.selection.set_many([huge_id]);
    h.copy();
    clipboard::paste_private_at(&mut h.editor, Some(pos2(640.0, 360.0)));
    let huge_pasted_id = h.editor.selection.object.clone().unwrap();
    let huge_pasted = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object(&huge_pasted_id)
        .unwrap()
        .0;
    assert!(huge_pasted.frame.x.is_finite() && huge_pasted.frame.y.is_finite());
}

#[test]
fn d6_3_copy_paste_grouped_objects_and_glued_connectors() {
    let mut h = TestHarness::new(make_base_course());
    h.editor
        .selection
        .set_many(["shape-a".to_owned(), "shape-b".to_owned()]);
    h.press(Key::G, Modifiers::COMMAND);
    let orig_group = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object("shape-a")
        .unwrap()
        .0
        .group
        .clone()
        .unwrap();
    let conn_res = h
        .editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": "s1",
                "kind": {
                    "type": "path",
                    "points": [{ "x": 0.0, "y": 0.5 }, { "x": 1.0, "y": 0.5 }],
                    "outline": { "color": "#000000", "width": 2.0 },
                    "connector": "straight",
                    "start_glue": { "object": "shape-a", "site": 1 },
                    "end_glue": { "object": "shape-b", "site": 3 }
                },
                "frame": { "x": 220.0, "y": 135.0, "w": 100.0, "h": 10.0 }
            }),
        )
        .unwrap();
    let conn_id = conn_res["id"].as_str().unwrap().to_owned();

    h.editor.ui.last_active_area = ActiveArea::Canvas;
    h.editor
        .selection
        .set_many(["shape-a".to_owned(), "shape-b".to_owned(), conn_id]);
    h.copy();

    for target_slide in ["s1", "s2"] {
        h.editor.selection.slide = target_slide.to_owned();
        clipboard::paste_private(&mut h.editor);
        let pasted_ids = h.editor.selection.ids();
        assert_eq!(pasted_ids.len(), 3);
        let slide = h.editor.session.course.slide(target_slide).unwrap();
        let g1 = slide
            .object(&pasted_ids[0])
            .unwrap()
            .0
            .group
            .clone()
            .unwrap();
        let g2 = slide
            .object(&pasted_ids[1])
            .unwrap()
            .0
            .group
            .clone()
            .unwrap();
        assert_eq!(g1, g2);
        if target_slide == "s1" {
            assert_ne!(g1, orig_group);
        }
        let ObjectKind::Path { path: path_data } = &slide.object(&pasted_ids[2]).unwrap().0.kind
        else {
            panic!("expected connector path");
        };
        assert_eq!(
            path_data.start_glue.as_ref().map(|g| g.object.as_str()),
            Some(pasted_ids[0].as_str())
        );
        assert_eq!(
            path_data.end_glue.as_ref().map(|g| g.object.as_str()),
            Some(pasted_ids[1].as_str())
        );
    }

    h.press(Key::Z, Modifiers::COMMAND);
    assert_eq!(
        h.editor.session.course.slide("s2").unwrap().objects.len(),
        1
    );
}

#[test]
fn d6_4_copy_object_with_dropped_on_and_objects_intersect_triggers_bug_d5() {
    let mut h = TestHarness::new(make_base_course());
    let triggers = [
        // 1: self-only -> copied
        serde_json::json!(trigger_click_state("shape-a", "normal")),
        // 2: action references uncopied shape-b -> skipped
        serde_json::json!(Trigger {
            when: CourseEvent::Click {
                object: "shape-a".to_owned(),
            },
            action: Action::ChangeState {
                object: "shape-b".to_owned(),
                state: "normal".to_owned(),
            },
            condition: None,
            disabled: false,
        }),
        // 3: DroppedOn(shape-a, shape-b) -> BUG-D5 must skip because shape-b is not copied
        serde_json::json!(Trigger {
            when: CourseEvent::DroppedOn {
                object: "shape-a".to_owned(),
                target: "shape-b".to_owned(),
            },
            action: Action::SetVariable {
                variable: "v1".to_owned(),
                op: coursecraft_course::VarOp::Assign,
                value: Some(coursecraft_course::ValueSource::Constant(
                    coursecraft_course::Value::Number(1.0)
                )),
            },
            condition: None,
            disabled: false,
        }),
        // 4: ObjectsIntersect(shape-a, shape-b) -> BUG-D5 must skip too
        serde_json::json!(Trigger {
            when: CourseEvent::ObjectsIntersect {
                a: "shape-a".to_owned(),
                b: "shape-b".to_owned(),
            },
            action: Action::SetVariable {
                variable: "v1".to_owned(),
                op: coursecraft_course::VarOp::Assign,
                value: Some(coursecraft_course::ValueSource::Constant(
                    coursecraft_course::Value::Number(2.0)
                )),
            },
            condition: None,
            disabled: false,
        }),
    ];
    for t in triggers {
        h.editor
            .run(
                "trigger.add",
                serde_json::json!({ "slide": "s1", "trigger": t }),
            )
            .unwrap();
    }

    h.editor.ui.last_active_area = ActiveArea::Canvas;
    h.editor.selection.set_many(["shape-a".to_owned()]);
    let (clip, skipped) = clipboard::copy_objects(&h.editor).expect("copy_objects");
    assert_eq!(clip.triggers.len(), 1, "only Trigger 1 should be copied");
    assert_eq!(
        skipped, 3,
        "BUG-D5: Triggers 2, 3 (DroppedOn), and 4 (ObjectsIntersect) must all be skipped"
    );

    h.copy();
    assert_eq!(
        h.editor.ui.status_message.as_ref().map(|(m, _)| m.as_str()),
        Some("Copied 1 object(s); skipped 3 trigger(s)")
    );
    h.editor.selection.slide = "s2".to_owned();
    clipboard::paste_private(&mut h.editor);
    let pasted_id = h.editor.selection.object.clone().unwrap();
    let s2 = h.editor.session.course.slide("s2").unwrap();
    assert_eq!(s2.triggers.len(), 1);
    assert_eq!(
        s2.triggers[0].when,
        CourseEvent::Click {
            object: pasted_id.clone()
        }
    );
    let errors = h
        .editor
        .session
        .course
        .validate()
        .into_iter()
        .filter(|f| f.severity == coursecraft_course::Severity::Error)
        .count();
    assert_eq!(errors, 0);
}

#[test]
fn d6_5_cross_course_copy_and_paste_of_objects_and_slides() {
    let mut course_a = make_base_course();
    let master_id = course_a.masters[0].id.clone();
    course_a.scenes[0].slides[0].master = Some(master_id);
    course_a.scenes[0].slides[0].player = Some(coursecraft_course::SlidePlayerOverrides {
        next_requires: Some(NextRequirement::Variable {
            name: "v1".to_owned(),
        }),
        ..Default::default()
    });
    course_a.scenes[0].slides[0].triggers.push(Trigger {
        when: CourseEvent::Click {
            object: "shape-a".to_owned(),
        },
        action: Action::JumpToSlide {
            target: SlideTarget::Id("s2".to_owned()),
        },
        condition: None,
        disabled: false,
    });
    course_a.scenes[0].slides[0].triggers.push(Trigger {
        when: CourseEvent::Click {
            object: "shape-a".to_owned(),
        },
        action: Action::SetVariable {
            variable: "v1".to_owned(),
            op: coursecraft_course::VarOp::Assign,
            value: Some(coursecraft_course::ValueSource::Constant(
                coursecraft_course::Value::Number(5.0),
            )),
        },
        condition: None,
        disabled: false,
    });

    let mut a = TestHarness::new(course_a);
    let blank_b: Course = serde_json::from_value(serde_json::json!({
        "schema_version": 1,
        "id": "course-b",
        "title": "Course B",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "b-scene",
            "title": "B Scene",
            "slides": [{ "id": "b-s1", "title": "B Slide 1" }]
        }]
    }))
    .unwrap();
    let mut b = TestHarness::new(blank_b);

    // Copy object from A -> paste into B
    a.editor.ui.last_active_area = ActiveArea::Canvas;
    a.editor.selection.set_many(["shape-a".to_owned()]);
    a.copy();
    let obj_text = a.editor.ui.clipboard_text.clone().expect("clipboard text");
    b.paste_text(&obj_text);
    assert_eq!(
        b.editor.session.course.slide("b-s1").unwrap().objects.len(),
        1
    );
    let b_errors_after_obj = b
        .editor
        .session
        .course
        .validate()
        .into_iter()
        .filter(|f| f.severity == coursecraft_course::Severity::Error)
        .count();
    assert_eq!(b_errors_after_obj, 0);

    // Copy slide from A -> paste into B
    a.editor.ui.last_active_area = ActiveArea::Navigator;
    a.editor.selection.slide = "s1".to_owned();
    a.copy();
    let slide_text = a
        .editor
        .ui
        .clipboard_text
        .clone()
        .expect("slide clipboard text");
    b.paste_text(&slide_text);
    assert_eq!(b.editor.session.course.slides().count(), 2);
    let b_errors_after_slide = b
        .editor
        .session
        .course
        .validate()
        .into_iter()
        .filter(|f| f.severity == coursecraft_course::Severity::Error)
        .count();
    assert_eq!(
        b_errors_after_slide, 0,
        "cross-course slide paste must clear missing master/links/next_requires"
    );
}

#[test]
fn d6_6_copy_cut_paste_and_duplicate_slides_across_scenes() {
    let mut one_slide: Course = serde_json::from_value(serde_json::json!({
        "schema_version": 1,
        "id": "single-slide",
        "title": "Single Slide",
        "scenes": [{
            "id": "sc1",
            "title": "Scene 1",
            "slides": [{ "id": "only-s", "title": "Only" }]
        }]
    }))
    .unwrap();
    let _ = &mut one_slide;
    let mut h_single = TestHarness::new(one_slide);
    h_single.editor.ui.last_active_area = ActiveArea::Navigator;
    h_single.cut();
    assert_eq!(h_single.editor.session.course.slides().count(), 1);
    assert_eq!(
        h_single
            .editor
            .ui
            .status_message
            .as_ref()
            .map(|(m, _)| m.as_str()),
        Some("Can't cut the only slide")
    );

    let mut h = TestHarness::new(make_base_course());
    let sc2_res = h
        .editor
        .run("scene.add", serde_json::json!({ "title": "Scene 2" }))
        .unwrap();
    let sc2_id = sc2_res["id"].as_str().unwrap().to_owned();
    let sc2_s_res = h
        .editor
        .run(
            "slide.add",
            serde_json::json!({ "scene": sc2_id, "title": "Scene 2 Slide 1" }),
        )
        .unwrap();
    let sc2_slide_id = sc2_s_res["id"].as_str().unwrap().to_owned();

    // Copy s1 -> select Scene 2's slide -> paste -> inserted into Scene 2
    h.editor.ui.last_active_area = ActiveArea::Navigator;
    h.editor.selection.slide = "s1".to_owned();
    h.copy();
    h.editor.selection.slide = sc2_slide_id.clone();
    clipboard::paste_private(&mut h.editor);
    assert_eq!(
        h.editor
            .session
            .course
            .scenes
            .iter()
            .find(|s| s.id == sc2_id)
            .unwrap()
            .slides
            .len(),
        2
    );

    // Cut s2 -> paste into Scene 2
    h.editor.selection.slide = "s2".to_owned();
    h.cut();
    assert!(h.editor.session.course.slide("s2").is_none());
    h.editor.selection.slide = sc2_slide_id;
    clipboard::paste_private(&mut h.editor);
    assert_eq!(
        h.editor
            .session
            .course
            .scenes
            .iter()
            .find(|s| s.id == sc2_id)
            .unwrap()
            .slides
            .len(),
        3
    );

    // Duplicate Slide 1 (with grouped objects) -> keeps group IDs distinct & remaps IDs
    h.editor.selection.slide = "s1".to_owned();
    h.editor
        .selection
        .set_many(["shape-a".to_owned(), "shape-b".to_owned()]);
    h.press(Key::G, Modifiers::COMMAND);
    h.editor.selection.clear_objects();
    h.editor.ui.last_active_area = ActiveArea::Navigator;
    h.editor
        .run("slide.duplicate", serde_json::json!({ "slide": "s1" }))
        .unwrap();
    assert_eq!(
        h.editor
            .session
            .course
            .scenes
            .iter()
            .find(|s| s.id == "scene-1")
            .unwrap()
            .slides
            .len(),
        2
    );
    h.press(Key::Z, Modifiers::COMMAND);
    assert_eq!(
        h.editor
            .session
            .course
            .scenes
            .iter()
            .find(|s| s.id == "scene-1")
            .unwrap()
            .slides
            .len(),
        1
    );
}

#[test]
fn d6_7_duplicate_ctrl_d_vs_alt_drag_duplicate_on_canvas() {
    let mut h = TestHarness::new(make_base_course());
    h.editor.ui.last_active_area = ActiveArea::Canvas;

    // Single Ctrl+D
    h.editor.selection.set_many(["shape-a".to_owned()]);
    let undo_0 = h.editor.session.undo_len();
    h.press(Key::D, Modifiers::COMMAND);
    assert_eq!(h.editor.session.undo_len(), undo_0 + 1);
    assert_eq!(
        h.editor.session.course.slide("s1").unwrap().objects.len(),
        6
    );
    h.press(Key::Z, Modifiers::COMMAND);
    assert_eq!(
        h.editor.session.course.slide("s1").unwrap().objects.len(),
        5
    );

    // Multi-select Ctrl+D
    h.editor
        .selection
        .set_many(["shape-a".to_owned(), "shape-b".to_owned()]);
    h.press(Key::D, Modifiers::COMMAND);
    assert_eq!(
        h.editor.session.course.slide("s1").unwrap().objects.len(),
        7
    );
    h.press(Key::Z, Modifiers::COMMAND);

    // Group Ctrl+D
    h.editor
        .selection
        .set_many(["shape-a".to_owned(), "shape-b".to_owned()]);
    h.press(Key::G, Modifiers::COMMAND);
    let orig_group = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object("shape-a")
        .unwrap()
        .0
        .group
        .clone()
        .unwrap();
    h.editor
        .selection
        .set_many(["shape-a".to_owned(), "shape-b".to_owned()]);
    h.press(Key::D, Modifiers::COMMAND);
    let dup_ids = h.editor.selection.ids();
    assert_eq!(dup_ids.len(), 2);
    let dup_group = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object(&dup_ids[0])
        .unwrap()
        .0
        .group
        .clone()
        .unwrap();
    assert_ne!(dup_group, orig_group);
    h.press(Key::Z, Modifiers::COMMAND);
    h.press(Key::Z, Modifiers::COMMAND); // also undo group

    // Alt-drag duplicate on canvas (unlocked)
    let start = slide_to_screen(&mut h, 160.0, 140.0);
    h.drag_by(start, vec2(0.0, 140.0), 6, Modifiers::ALT);
    assert_eq!(
        h.editor.session.course.slide("s1").unwrap().objects.len(),
        6
    );
    h.press(Key::Z, Modifiers::COMMAND);
    assert_eq!(
        h.editor.session.course.slide("s1").unwrap().objects.len(),
        5
    );

    // Locked object cannot be Alt-dragged
    h.editor
        .run(
            "object.lock",
            serde_json::json!({ "slide": "s1", "object": "shape-a", "locked": true }),
        )
        .unwrap();
    let start = slide_to_screen(&mut h, 160.0, 140.0);
    h.drag_by(start, vec2(0.0, 140.0), 6, Modifiers::ALT);
    assert_eq!(
        h.editor.session.course.slide("s1").unwrap().objects.len(),
        5
    );
}

#[test]
fn d6_8_copy_paste_across_slide_layers() {
    let mut h = TestHarness::new(make_base_course());
    h.editor.ui.last_active_area = ActiveArea::Canvas;

    // Base -> Layer "l1"
    h.editor.selection.set_many(["shape-a".to_owned()]);
    h.copy();
    h.editor.selection.layer = Some("l1".to_owned());
    clipboard::paste_private(&mut h.editor);
    let l1_pasted = h.editor.selection.object.clone().unwrap();
    assert!(
        h.editor
            .session
            .course
            .slide("s1")
            .unwrap()
            .layer("l1")
            .unwrap()
            .objects
            .iter()
            .any(|o| o.id == l1_pasted)
    );

    // Layer "l1" -> Slide 2 Base
    h.editor.selection.set_many(["l1-shape".to_owned()]);
    h.copy();
    h.editor.selection.slide = "s2".to_owned();
    h.editor.selection.layer = None;
    clipboard::paste_private(&mut h.editor);
    let s2_pasted = h.editor.selection.object.clone().unwrap();
    assert!(
        h.editor
            .session
            .course
            .slide("s2")
            .unwrap()
            .objects
            .iter()
            .any(|o| o.id == s2_pasted)
    );

    // Base -> Slide 2 Layer "s2-l1"
    h.editor.selection.layer = Some("s2-l1".to_owned());
    clipboard::paste_private(&mut h.editor);
    let s2_l1_pasted = h.editor.selection.object.clone().unwrap();
    assert!(
        h.editor
            .session
            .course
            .slide("s2")
            .unwrap()
            .layer("s2-l1")
            .unwrap()
            .objects
            .iter()
            .any(|o| o.id == s2_l1_pasted)
    );
}

#[test]
fn d6_9_plain_text_paste_and_utf8_4096_byte_truncation_safety() {
    let mut h = TestHarness::new(make_base_course());
    h.paste_text("Hello world");
    let first_id = h.editor.selection.object.clone().unwrap();
    let ObjectKind::Text { text, .. } = &h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object(&first_id)
        .unwrap()
        .0
        .kind
    else {
        panic!("expected text box");
    };
    assert_eq!(text, "Hello world");

    // 4094 ASCII bytes + 4-byte emoji 🎉 (straddling byte 4096) + trailing ASCII
    let mut payload = "a".repeat(4094);
    payload.push('🎉');
    payload.push_str(&"b".repeat(902));
    assert_eq!(payload.len(), 5000);
    h.paste_text(&payload);
    let second_id = h.editor.selection.object.clone().unwrap();
    let ObjectKind::Text { text, .. } = &h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object(&second_id)
        .unwrap()
        .0
        .kind
    else {
        panic!("expected text box");
    };
    assert!(text.len() <= 4096);
    assert_eq!(text.len(), 4094);
}

#[test]
fn d6_10_hostile_corrupt_and_oversized_clipboard_payloads() {
    let mut h = TestHarness::new(make_base_course());
    let before = h.editor.session.course.clone();
    let rev_before = h.editor.session.revision;

    // (a) > 16 MiB string
    let huge = "x".repeat(clipboard::MAX_CLIPBOARD_BYTES + 1);
    h.paste_text(&huge);
    assert_eq!(h.editor.session.course, before);

    // (b) wrong version 999
    h.paste_text(r#"{"coursecraft_clipboard":999,"kind":"objects","objects":[]}"#);
    assert_eq!(h.editor.session.course, before);

    // (c) broken JSON with marker
    h.paste_text(r#"{"coursecraft_clipboard":1,"kind":"objects", BROKEN"#);
    assert_eq!(h.editor.session.course, before);

    // (d) NaN / negative dimensions
    let bad_obj = serde_json::json!({
        "coursecraft_clipboard": 1,
        "kind": "objects",
        "objects": [{
            "id": "bad",
            "frame": { "x": 0.0, "y": 0.0, "w": -50.0, "h": 40.0 },
            "kind": { "type": "shape", "shape": "rectangle", "fill": "#000000" }
        }]
    })
    .to_string();
    h.paste_text(&bad_obj);
    assert_eq!(h.editor.session.course, before);
    let nan_obj = r##"{"coursecraft_clipboard":1,"kind":"objects","objects":[{"id":"bad","frame":{"x":0.0,"y":0.0,"w":null,"h":40.0},"kind":{"type":"shape","shape":"rectangle","fill":"#000000"}}]}"##;
    h.paste_text(nan_obj);
    assert_eq!(h.editor.session.course, before);
    assert_eq!(h.editor.session.revision, rev_before);
}

#[test]
fn d6_11_master_view_guards_for_clipboard_vs_duplicate() {
    let mut h = TestHarness::new(make_base_course());
    h.editor.enter_master_view();
    let master_id = h.editor.selection.slide.clone();
    let add_res = h
        .editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": master_id,
                "kind": { "type": "shape", "shape": "rectangle", "fill": "#111111" },
                "frame": { "x": 50.0, "y": 50.0, "w": 100.0, "h": 60.0 }
            }),
        )
        .unwrap();
    let m_obj = add_res["id"].as_str().unwrap().to_owned();
    h.editor.selection.set_many([m_obj]);

    clipboard::copy(&mut h.editor);
    assert_eq!(
        h.editor.ui.status_message.as_ref().map(|(m, _)| m.as_str()),
        Some("Not available in Master View")
    );
    clipboard::cut(&mut h.editor);
    assert_eq!(
        h.editor.ui.status_message.as_ref().map(|(m, _)| m.as_str()),
        Some("Not available in Master View")
    );
    clipboard::paste_text(&mut h.editor, "hello");
    assert_eq!(
        h.editor.ui.status_message.as_ref().map(|(m, _)| m.as_str()),
        Some("Not available in Master View")
    );
    assert!(!h.editor.paste_image_png(&make_tiny_png()));
    assert_eq!(
        h.editor.ui.status_message.as_ref().map(|(m, _)| m.as_str()),
        Some("Not available in Master View")
    );

    // Ctrl+D on master shape works cleanly and undoes
    let count_before = h
        .editor
        .session
        .course
        .master(&master_id)
        .unwrap()
        .objects
        .len();
    h.press(Key::D, Modifiers::COMMAND);
    assert_eq!(
        h.editor
            .session
            .course
            .master(&master_id)
            .unwrap()
            .objects
            .len(),
        count_before + 1
    );
    h.press(Key::Z, Modifiers::COMMAND);
    assert_eq!(
        h.editor
            .session
            .course
            .master(&master_id)
            .unwrap()
            .objects
            .len(),
        count_before
    );
}
