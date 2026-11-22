//! M29 Step s7 tests: Find & Check Course state navigation (`U-17`, `SUS-16`),
//! Media Library & `.ccraft` save/replace for `state.fill_style` (`U-19`),
//! and copy/paste/duplicate/template preservation of widened states (`E-15`).

use crate::test_harness::TestHarness;
use crate::{BottomTab, InspectorTab};
use coursecraft_course::{Course, FillStyle, StateDef};

fn png_bytes(val: u8) -> Vec<u8> {
    let pixels = vec![val; 4 * 4 * 4];
    coursecraft_media::encode_png_rgba(4, 4, &pixels).expect("png")
}

fn s7_course() -> Course {
    let raw = serde_json::json!({
        "schema_version": 1,
        "id": "c-m29-s7",
        "title": "M29 S7 Test",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "scene1",
            "title": "Main",
            "slides": [
                {
                    "id": "s1",
                    "title": "Slide 1",
                    "objects": [{
                        "id": "t1",
                        "name": "Intro Label",
                        "frame": { "x": 40.0, "y": 40.0, "w": 200.0, "h": 60.0 },
                        "kind": {
                            "type": "text",
                            "text": "Normal",
                            "style": { "font": "Inter", "size": 18.0, "color": "#111111" }
                        }
                    }]
                },
                {
                    "id": "s2",
                    "title": "Slide 2",
                    "objects": [
                        {
                            "id": "btn1",
                            "name": "Visited Card",
                            "frame": { "x": 100.0, "y": 100.0, "w": 240.0, "h": 120.0 },
                            "kind": {
                                "type": "shape",
                                "shape": "rectangle",
                                "fill": "#224466",
                                "text": {
                                    "text": "Base text",
                                    "style": { "font": "Inter", "size": 16.0, "color": "#FFFFFF" }
                                }
                            },
                            "states": [
                                {
                                    "name": "visited",
                                    "text": "Visited text",
                                    "fill_style": { "type": "picture", "src": "media/state-bg.png" },
                                    "outline": { "color": "#FFAA00", "width": 3.0 },
                                    "text_style": { "font": "Inter", "size": 20.0, "color": "#FFEECC" },
                                    "opacity": 0.85,
                                    "shadow": { "color": "#00000055", "offset_x": 2.0, "offset_y": 3.0, "blur": 5.0 },
                                    "glow": { "color": "#00FFAA66", "size": 6.0 },
                                    "rotation": 15.0,
                                    "flip_h": true
                                }
                            ]
                        },
                        {
                            "id": "img1",
                            "name": "Picture 1",
                            "frame": { "x": 400.0, "y": 100.0, "w": 200.0, "h": 200.0 },
                            "kind": { "type": "image", "src": "media/base.png" },
                            "states": [
                                { "name": "hover", "src": "media/missing-hover.png" }
                            ]
                        }
                    ]
                }
            ]
        }],
        "masters": [{
            "id": "m1",
            "title": "Master 1",
            "objects": []
        }]
    });
    serde_json::from_value(raw).expect("valid s7 course")
}

#[test]
fn u17_find_hit_in_state_text_opens_states_tab_and_previews_state() {
    let mut h = TestHarness::new(s7_course());
    h.editor.selection.slide = "s1".to_owned();
    h.editor.selection.set_many(["t1".to_owned()]);
    h.editor.ui.bottom_tab = BottomTab::Timeline;
    h.editor.ui.preview_state = None;
    h.editor.ui.find.open = true;
    h.editor.ui.find.query = "Visited text".to_owned();
    h.idle_frame();

    let hit_pos = h
        .find_text_last("Visited text")
        .expect("Find result row for Visited text");
    h.click_at(hit_pos);
    h.idle_frame();

    assert_eq!(h.editor.selection.slide, "s2");
    assert_eq!(h.editor.selection.object.as_deref(), Some("btn1"));
    assert_eq!(h.editor.ui.bottom_tab, BottomTab::States);
    assert_eq!(h.editor.ui.preview_state.as_deref(), Some("visited"));
}

#[test]
fn u17_check_course_issue_on_state_path_opens_states_tab_and_previews_state() {
    let mut h = TestHarness::new(s7_course());
    h.editor.selection.slide = "s1".to_owned();
    h.editor.selection.set_many(["t1".to_owned()]);
    h.editor.ui.bottom_tab = BottomTab::Timeline;
    h.editor.ui.preview_state = None;
    h.editor.ui.show_validation_window = true;
    h.editor.ui.validation_issues = vec![crate::ValidationIssueSummary {
        severity: "warning".to_owned(),
        path: "scenes/0/slides/1/objects/1/states/0/src".to_owned(),
        message: "State image missing".to_owned(),
    }];
    h.idle_frame();

    let btn = h
        .find_text_containing("scenes/0/slides/1/objects/1/states/0/src")
        .expect("issue button in Check Course");
    h.click_at(btn);
    h.idle_frame();

    assert_eq!(h.editor.selection.slide, "s2");
    assert_eq!(h.editor.selection.object.as_deref(), Some("img1"));
    assert_eq!(h.editor.ui.bottom_tab, BottomTab::States);
    assert_eq!(h.editor.ui.preview_state.as_deref(), Some("hover"));
}

#[test]
fn u17_check_course_missing_state_media_issue_opens_states_tab_and_previews_state() {
    let mut h = TestHarness::new(s7_course());
    h.editor.set_media(vec![
        (
            "media/base.png".to_owned(),
            std::sync::Arc::from(png_bytes(10).as_slice()),
        ),
        (
            "media/state-bg.png".to_owned(),
            std::sync::Arc::from(png_bytes(20).as_slice()),
        ),
    ]);
    h.editor.selection.slide = "s1".to_owned();
    h.editor.selection.set_many(["t1".to_owned()]);
    h.editor.ui.bottom_tab = BottomTab::Timeline;
    h.editor.ui.preview_state = None;
    h.editor.refresh_validate_only();
    h.editor.ui.show_validation_window = true;
    h.idle_frame();

    let btn = h
        .find_text_containing("Picture 1 ▸ State \"hover\"")
        .expect("missing state media issue button in Check Course");
    h.click_at(btn);
    h.idle_frame();

    assert_eq!(h.editor.selection.slide, "s2");
    assert_eq!(h.editor.selection.object.as_deref(), Some("img1"));
    assert_eq!(h.editor.ui.bottom_tab, BottomTab::States);
    assert_eq!(h.editor.ui.preview_state.as_deref(), Some("hover"));
}

#[test]
fn u19_used_media_and_media_library_places_include_state_fill_style_picture() {
    let mut h = TestHarness::new(s7_course());
    let old_bytes = png_bytes(42);
    let new_bytes = png_bytes(99);
    h.editor.set_media(vec![
        (
            "media/base.png".to_owned(),
            std::sync::Arc::from(png_bytes(10).as_slice()),
        ),
        (
            "media/state-bg.png".to_owned(),
            std::sync::Arc::from(old_bytes.as_slice()),
        ),
    ]);

    // Save-time `used_media()` retains images referenced only by `state.fill_style`.
    let used = h.editor.used_media();
    assert!(
        used.iter().any(|(src, _)| src == "media/state-bg.png"),
        "used_media() must keep media/state-bg.png referenced only by state.fill_style"
    );

    // Media Library shows it in "Places using this file" and clicking it opens States tab + previews "visited".
    h.editor.ui.show_media_library = true;
    h.editor.ui.media_library.selected = Some("media/state-bg.png".to_owned());
    h.idle_frame();
    assert!(
        h.find_text("Places using this file").is_some(),
        "expected Places using this file section"
    );
    let place_btn = h
        .find_text_containing("Visited Card ▸ State \"visited\"")
        .expect("place button for state.fill_style");
    h.click_at(place_btn);
    h.idle_frame();
    assert_eq!(h.editor.selection.slide, "s2");
    assert_eq!(h.editor.selection.object.as_deref(), Some("btn1"));
    assert_eq!(h.editor.ui.bottom_tab, BottomTab::States);
    assert_eq!(h.editor.ui.preview_state.as_deref(), Some("visited"));

    // Replacing the media bytes updates the stored bytes while preserving the state fill_style reference.
    assert!(
        h.editor
            .replace_media_bytes("media/state-bg.png", &new_bytes)
    );
    let replaced = h
        .editor
        .used_media()
        .into_iter()
        .find(|(src, _)| src == "media/state-bg.png")
        .map(|(_, bytes)| bytes.to_vec());
    assert_eq!(replaced.as_deref(), Some(new_bytes.as_slice()));
    let state = h
        .editor
        .session
        .course
        .slide("s2")
        .and_then(|s| s.object("btn1"))
        .and_then(|(o, _)| o.states.first())
        .expect("visited state");
    assert_eq!(
        state.fill_style.as_ref().and_then(FillStyle::picture_src),
        Some("media/state-bg.png")
    );
}

#[test]
fn e15_copy_paste_duplicate_and_template_preserve_widened_state_overrides() {
    let mut h = TestHarness::new(s7_course());
    h.editor.selection.slide = "s2".to_owned();
    h.editor.selection.set_many(["btn1".to_owned()]);

    // 1. object.duplicate preserves widened StateDef overrides.
    let dup_reply = h
        .editor
        .run(
            "object.duplicate",
            serde_json::json!({ "slide": "s2", "object": "btn1" }),
        )
        .expect("object.duplicate");
    let dup_id = dup_reply["id"].as_str().expect("dup id").to_owned();
    let dup_obj = h
        .editor
        .session
        .course
        .slide("s2")
        .and_then(|s| s.object(&dup_id))
        .expect("duplicated object")
        .0;
    assert_widened_visited(&dup_obj.states[0]);

    // 2. object.paste onto a normal slide preserves widened StateDef overrides.
    let btn1_json = serde_json::to_value(
        h.editor
            .session
            .course
            .slide("s2")
            .and_then(|s| s.object("btn1"))
            .expect("btn1")
            .0,
    )
    .expect("serialize btn1");
    let paste_reply = h
        .editor
        .run(
            "object.paste",
            serde_json::json!({ "slide": "s1", "objects": [btn1_json.clone()] }),
        )
        .expect("object.paste onto s1");
    let pasted_id = paste_reply["ids"][0]
        .as_str()
        .expect("pasted id")
        .to_owned();
    let pasted_obj = h
        .editor
        .session
        .course
        .slide("s1")
        .and_then(|s| s.object(&pasted_id))
        .expect("pasted object")
        .0;
    assert_widened_visited(&pasted_obj.states[0]);

    // 3. object.paste onto a master slide strips states and resets initial_state = "normal".
    let master_paste = h
        .editor
        .run(
            "object.paste",
            serde_json::json!({ "slide": "m1", "objects": [btn1_json] }),
        )
        .expect("object.paste onto m1");
    let master_obj_id = master_paste["ids"][0]
        .as_str()
        .expect("master pasted id")
        .to_owned();
    let master_obj = h
        .editor
        .session
        .course
        .slide("m1")
        .and_then(|s| s.object(&master_obj_id))
        .expect("master pasted object")
        .0;
    assert!(master_obj.states.is_empty());
    assert_eq!(master_obj.initial_state, "normal");

    // 4. slide.duplicate preserves widened StateDef overrides.
    let sdup_reply = h
        .editor
        .run("slide.duplicate", serde_json::json!({ "slide": "s2" }))
        .expect("slide.duplicate");
    let sdup_id = sdup_reply["id"].as_str().expect("sdup id").to_owned();
    let sdup_obj = h
        .editor
        .session
        .course
        .slide(&sdup_id)
        .and_then(|s| s.objects.iter().find(|o| o.name == "Visited Card"))
        .expect("duplicated slide Visited Card");
    assert_widened_visited(&sdup_obj.states[0]);

    // 5. template.save + template.add_slide preserve widened StateDef overrides.
    let tpl_reply = h
        .editor
        .run("template.save", serde_json::json!({ "slide": "s2" }))
        .expect("template.save");
    let tpl_id = tpl_reply["id"].as_str().expect("tpl id").to_owned();
    let add_tpl_reply = h
        .editor
        .run(
            "template.add_slide",
            serde_json::json!({ "scene": "scene1", "template": tpl_id }),
        )
        .expect("template.add_slide");
    let new_slide_id = add_tpl_reply["id"]
        .as_str()
        .expect("new slide id")
        .to_owned();
    let tpl_obj = h
        .editor
        .session
        .course
        .slide(&new_slide_id)
        .and_then(|s| s.objects.iter().find(|o| o.name == "Visited Card"))
        .expect("template slide Visited Card");
    assert_widened_visited(&tpl_obj.states[0]);

    // Keep InspectorTab referenced so clippy is happy if unused in other tests.
    let _ = InspectorTab::Format;
}

fn assert_widened_visited(st: &StateDef) {
    assert_eq!(st.name, "visited");
    assert_eq!(st.text.as_deref(), Some("Visited text"));
    assert_eq!(
        st.fill_style.as_ref().and_then(FillStyle::picture_src),
        Some("media/state-bg.png")
    );
    assert!(st.outline.is_some());
    assert!(st.text_style.is_some());
    assert_eq!(st.opacity, Some(0.85));
    assert!(st.shadow.is_some());
    assert!(st.glow.is_some());
    assert_eq!(st.rotation, Some(15.0));
    assert_eq!(st.flip_h, Some(true));
}
