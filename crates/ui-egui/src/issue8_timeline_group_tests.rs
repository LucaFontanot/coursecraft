//! Issue 8 Part A: groups show as collapsible folder rows on the timeline.

use crate::BottomTab;
use crate::test_harness::TestHarness;
use crate::timeline::{TimelineRow, timeline_rows};
use coursecraft_course::{Course, Timing};
use egui::{Modifiers, vec2};
use std::collections::BTreeSet;

fn group_course() -> Course {
    serde_json::from_value(serde_json::json!({
        "schema_version": 1,
        "id": "issue8",
        "title": "Issue 8",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "scene-1",
            "title": "Scene 1",
            "slides": [{
                "id": "s1",
                "title": "Slide 1",
                "duration_ms": 5000,
                "cue_points": [],
                "objects": [
                    { "id": "solo", "name": "solo", "kind": { "type": "shape", "shape": "rectangle", "fill": "#111111" }, "frame": { "x": 10.0, "y": 10.0, "w": 80.0, "h": 80.0 } },
                    { "id": "bg", "name": "bg", "kind": { "type": "shape", "shape": "rectangle", "fill": "#222222" }, "frame": { "x": 100.0, "y": 100.0, "w": 160.0, "h": 60.0 }, "timing": { "start_ms": 500, "end_ms": 2500 }, "group": "g1" },
                    { "id": "label", "name": "label", "kind": { "type": "text", "text": "Next" }, "frame": { "x": 110.0, "y": 110.0, "w": 100.0, "h": 40.0 }, "timing": { "start_ms": 1000 }, "group": "g1" }
                ]
            }]
        }]
    }))
    .expect("course json")
}

fn open_timeline(h: &mut TestHarness) {
    h.editor.ui.bottom_tab = BottomTab::Timeline;
    h.idle_frame();
    h.idle_frame();
}

fn object<'a>(h: &'a TestHarness, id: &str) -> &'a coursecraft_course::Object {
    h.editor
        .session
        .course
        .slide("s1")
        .expect("slide")
        .object(id)
        .expect("object")
        .0
}

fn timing(h: &TestHarness, id: &str) -> Timing {
    object(h, id).timing
}

fn undo(h: &mut TestHarness) -> bool {
    h.editor.run("edit.undo", serde_json::json!({})).is_some()
}

fn cached(id: &str, group: Option<&str>) -> crate::timeline::CachedTimelineObject {
    (
        id.to_owned(),
        id.to_owned(),
        Timing::default(),
        group.map(str::to_owned),
    )
}

#[test]
fn rows_put_a_folder_row_above_indented_members() {
    // Model order is back to front; the timeline shows front first.
    let objects = vec![
        cached("a", None),
        cached("b", Some("g1")),
        cached("c", Some("g1")),
        cached("d", Some("lonely")),
    ];
    let rows = timeline_rows(&objects, &BTreeSet::new());
    assert_eq!(
        rows,
        vec![
            TimelineRow::Object {
                model_idx: 3,
                indented: false
            },
            TimelineRow::Group {
                group: "g1".into(),
                members: vec![2, 1]
            },
            TimelineRow::Object {
                model_idx: 2,
                indented: true
            },
            TimelineRow::Object {
                model_idx: 1,
                indented: true
            },
            TimelineRow::Object {
                model_idx: 0,
                indented: false
            },
        ]
    );

    let collapsed: BTreeSet<String> = ["g1".to_owned()].into();
    let rows = timeline_rows(&objects, &collapsed);
    assert_eq!(rows.len(), 3, "collapsed members are not rows: {rows:?}");
}

#[test]
fn group_row_shows_and_collapses() {
    let mut h = TestHarness::new(group_course());
    open_timeline(&mut h);

    let tl = &h.editor.ui.timeline;
    let group_row = tl.group_row_rects["g1"];
    assert!(tl.object_row_rects["label"].min.y >= group_row.max.y - 0.5);
    assert!(tl.object_row_rects["bg"].min.y >= group_row.max.y - 0.5);

    let toggle = h.editor.ui.timeline.group_toggle_rects["g1"];
    h.click_at(toggle.center());
    h.idle_frame();
    let tl = &h.editor.ui.timeline;
    assert!(tl.collapsed_groups.contains("g1"));
    assert!(!tl.object_row_rects.contains_key("bg"));
    assert!(!tl.object_row_rects.contains_key("label"));
    assert!(tl.object_row_rects.contains_key("solo"));
    assert!(tl.group_bar_rects.contains_key("g1"));

    let toggle = h.editor.ui.timeline.group_toggle_rects["g1"];
    h.click_at(toggle.center());
    h.idle_frame();
    assert!(h.editor.ui.timeline.object_row_rects.contains_key("bg"));
}

#[test]
fn dragging_the_collapsed_bar_moves_all_members_in_one_undo_step() {
    let mut h = TestHarness::new(group_course());
    open_timeline(&mut h);
    h.editor
        .ui
        .timeline
        .collapsed_groups
        .insert("g1".to_owned());
    h.idle_frame();

    let bar = h.editor.ui.timeline.group_bar_rects["g1"];
    h.drag_by(bar.center(), vec2(60.0, 0.0), 6, Modifiers::NONE);
    h.idle_frame();

    let bg = timing(&h, "bg");
    let label = timing(&h, "label");
    let shift = i64::from(bg.start_ms) - 500;
    assert!(shift > 0, "the group moved right: {bg:?}");
    assert_eq!(shift % 100, 0, "the shift snaps to 100 ms");
    assert_eq!(i64::from(label.start_ms) - 1000, shift);
    assert_eq!(bg.end_ms.map(i64::from), Some(2500 + shift));
    assert_eq!(label.end_ms, None, "an open end stays open");

    assert!(undo(&mut h));
    assert_eq!(timing(&h, "bg").start_ms, 500);
    assert_eq!(timing(&h, "label").start_ms, 1000);
}

#[test]
fn group_eye_and_lock_apply_to_all_members() {
    let mut h = TestHarness::new(group_course());
    open_timeline(&mut h);

    let eye = h.editor.ui.timeline.group_eye_rects["g1"];
    h.click_at(eye.center());
    h.idle_frame();
    assert!(object(&h, "bg").editor_hidden && object(&h, "label").editor_hidden);
    assert!(!object(&h, "solo").editor_hidden);

    let eye = h.editor.ui.timeline.group_eye_rects["g1"];
    h.click_at(eye.center());
    h.idle_frame();
    assert!(!object(&h, "bg").editor_hidden && !object(&h, "label").editor_hidden);

    let lock = h.editor.ui.timeline.group_lock_rects["g1"];
    h.click_at(lock.center());
    h.idle_frame();
    assert!(object(&h, "bg").locked && object(&h, "label").locked);

    assert!(undo(&mut h));
    assert!(!object(&h, "bg").locked && !object(&h, "label").locked);
}
