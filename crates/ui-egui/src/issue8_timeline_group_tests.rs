//! Issue 8 Part A: groups show as collapsible folder rows on the timeline.

use crate::BottomTab;
use crate::test_harness::TestHarness;
use crate::timeline::{TimelineRow, timeline_rows};
use coursecraft_course::{Course, Timing};
use egui::{Event, Key, Modifiers, PointerButton, vec2};
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

#[test]
fn group_bar_drag_preview_keeps_open_end_pinned_to_slide_end() {
    let mut h = TestHarness::new(group_course());
    open_timeline(&mut h);

    let orig_group_bar = h.editor.ui.timeline.group_bar_rects["g1"];
    let orig_label_bar = h.editor.ui.timeline.object_bar_rects["label"];
    let start = orig_group_bar.center();
    let moved = start + vec2(60.0, 0.0);

    // Press and drag right without releasing so we inspect the live preview rects.
    h.frame(vec![
        Event::PointerMoved(start),
        Event::PointerButton {
            pos: start,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
    ]);
    h.frame(vec![Event::PointerMoved(moved)]);
    // One more frame while still held so painted rects reflect the active drag shift.
    h.frame(vec![Event::PointerMoved(moved)]);

    let tl = &h.editor.ui.timeline;
    assert!(
        tl.group_drag.as_ref().is_some_and(|d| d.delta_ms > 0),
        "expected an active positive group drag: {:?}",
        tl.group_drag
    );
    let preview_group_bar = tl.group_bar_rects["g1"];
    let preview_label_bar = tl.object_bar_rects["label"];
    assert!(
        preview_group_bar.min.x > orig_group_bar.min.x + 1.0,
        "group bar start should shift right during drag"
    );
    assert!(
        (preview_group_bar.max.x - orig_group_bar.max.x).abs() < 0.5,
        "open-ended group bar right edge must stay pinned to slide end: orig={} preview={}",
        orig_group_bar.max.x,
        preview_group_bar.max.x
    );
    assert!(
        (preview_label_bar.max.x - orig_label_bar.max.x).abs() < 0.5,
        "open-ended member bar right edge must stay pinned to slide end: orig={} preview={}",
        orig_label_bar.max.x,
        preview_label_bar.max.x
    );
}

#[test]
fn escape_cancels_group_bar_drag_without_clearing_selection() {
    let mut h = TestHarness::new(group_course());
    open_timeline(&mut h);

    let bar = h.editor.ui.timeline.group_bar_rects["g1"];
    let start = bar.center();
    let moved = start + vec2(60.0, 0.0);

    h.frame(vec![
        Event::PointerMoved(start),
        Event::PointerButton {
            pos: start,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
    ]);
    h.frame(vec![Event::PointerMoved(moved)]);
    assert!(h.editor.ui.timeline.group_drag.is_some());

    // Pressing Escape cancels the drag and must not leak to the canvas to clear selection.
    h.frame(vec![Event::Key {
        key: Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    h.frame(vec![Event::PointerButton {
        pos: moved,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    }]);
    h.idle_frame();

    assert!(h.editor.ui.timeline.group_drag.is_none());
    assert_eq!(timing(&h, "bg").start_ms, 500);
    assert_eq!(timing(&h, "label").start_ms, 1000);
    assert!(
        h.editor.selection.contains("bg") && h.editor.selection.contains("label"),
        "Escape cancelling group drag must keep the group selected: {:?}",
        h.editor.selection.ids()
    );
}

#[test]
fn hostile_values_never_panic_on_group_rows_and_drag() {
    for (duration_ms, objects) in [
        // Zero-length slide with a normal group and an empty-string group id.
        (
            0_u32,
            serde_json::json!([
                { "id": "a", "name": "a", "kind": { "type": "shape", "shape": "rectangle", "fill": "#111111" }, "frame": { "x": 10.0, "y": 10.0, "w": 40.0, "h": 40.0 }, "timing": { "start_ms": 0 }, "group": "" },
                { "id": "b", "name": "b", "kind": { "type": "shape", "shape": "rectangle", "fill": "#222222" }, "frame": { "x": 60.0, "y": 10.0, "w": 40.0, "h": 40.0 }, "timing": { "start_ms": 0, "end_ms": 0 }, "group": "" }
            ]),
        ),
        // Huge duration and inverted/max timings.
        (
            u32::MAX,
            serde_json::json!([
                { "id": "a", "name": "a", "kind": { "type": "shape", "shape": "rectangle", "fill": "#111111" }, "frame": { "x": 10.0, "y": 10.0, "w": 40.0, "h": 40.0 }, "timing": { "start_ms": u32::MAX, "end_ms": 100 }, "group": "g1" },
                { "id": "b", "name": "b", "kind": { "type": "shape", "shape": "rectangle", "fill": "#222222" }, "frame": { "x": 60.0, "y": 10.0, "w": 40.0, "h": 40.0 }, "timing": { "start_ms": 0 }, "group": "g1" }
            ]),
        ),
        // Empty slide.
        (5000_u32, serde_json::json!([])),
    ] {
        let course: Course = serde_json::from_value(serde_json::json!({
            "schema_version": 1,
            "id": "hostile",
            "title": "Hostile",
            "slide_size": { "w": 1280, "h": 720 },
            "scenes": [{
                "id": "scene-1",
                "title": "Scene 1",
                "slides": [{
                    "id": "s1",
                    "title": "Slide 1",
                    "duration_ms": duration_ms,
                    "cue_points": [],
                    "objects": objects
                }]
            }]
        }))
        .expect("hostile course");

        let mut h = TestHarness::new(course);
        open_timeline(&mut h);
        let group_bars: Vec<(String, egui::Rect)> = h
            .editor
            .ui
            .timeline
            .group_bar_rects
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        for (_, bar) in group_bars {
            assert!(bar.is_finite());
            h.drag_by(bar.center(), vec2(80.0, 0.0), 4, Modifiers::NONE);
            h.drag_by(bar.center(), vec2(-80.0, 0.0), 4, Modifiers::SHIFT);
            h.idle_frame();
        }
    }
}
