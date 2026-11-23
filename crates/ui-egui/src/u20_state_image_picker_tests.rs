use crate::test_harness::TestHarness;
use crate::{EditorRequest, PendingImagePick};
use coursecraft_course::{Course, ObjectKind};
use egui::pos2;

fn png_bytes() -> Vec<u8> {
    let pixels = vec![180u8; 4 * 4 * 4];
    coursecraft_media::encode_png_rgba(4, 4, &pixels).expect("png")
}

fn image_course(state_src: Option<&str>) -> Course {
    let state_json = match state_src {
        Some(src) => serde_json::json!({ "name": "hover", "src": src }),
        None => serde_json::json!({ "name": "hover" }),
    };
    let raw = serde_json::json!({
        "schema_version": 1,
        "id": "c-u20",
        "title": "U20 State Image Picker",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "scene1",
            "title": "Main",
            "slides": [{
                "id": "s1",
                "title": "Slide 1",
                "objects": [{
                    "id": "img1",
                    "name": "Picture 1",
                    "frame": { "x": 100.0, "y": 100.0, "w": 200.0, "h": 200.0 },
                    "kind": { "type": "image", "src": "base.png" },
                    "states": [state_json]
                }]
            }]
        }]
    });
    serde_json::from_value(raw).expect("valid course")
}

fn open_image_states(h: &mut TestHarness) {
    h.editor.set_media(vec![
        ("base.png".to_owned(), std::sync::Arc::from(png_bytes())),
        ("proj-alt.png".to_owned(), std::sync::Arc::from(png_bytes())),
    ]);
    h.editor.selection.slide = "s1".to_owned();
    h.editor.selection.object = Some("img1".to_owned());
    h.editor.ui.bottom_tab = crate::BottomTab::States;
    h.idle_frame();
    h.editor.ui.preview_state = Some("hover".to_owned());
    h.idle_frame();
}

fn state_src(h: &TestHarness, state_name: &str) -> Option<String> {
    h.editor
        .session
        .course
        .slide("s1")
        .and_then(|s| s.object("img1"))
        .and_then(|(o, _)| o.states.iter().find(|st| st.name == state_name))
        .and_then(|st| st.src.clone())
}

fn object_src(h: &TestHarness) -> String {
    let (obj, _) = h
        .editor
        .session
        .course
        .slide("s1")
        .unwrap()
        .object("img1")
        .unwrap();
    match &obj.kind {
        ObjectKind::Image { src, .. } => src.clone(),
        other => panic!("expected Image, got {other:?}"),
    }
}

#[test]
fn state_card_has_no_src_text_input_and_shows_same_as_normal() {
    let mut h = TestHarness::new(image_course(None));
    open_image_states(&mut h);

    let bottom = egui::Rect::from_min_max(pos2(0.0, h.bottom_panel_top()), pos2(1280.0, 720.0));
    assert!(
        h.find_text_in("Src", bottom).is_none(),
        "raw Src text box label must be removed from States tab"
    );
    assert!(
        h.find_text("Same as normal").is_some(),
        "empty state src should show muted 'Same as normal'"
    );
    let _choose = h
        .find_text("Choose image…")
        .expect("Choose image… button should be visible");
    assert!(
        h.find_text("From project…").is_some(),
        "From project… menu should be visible"
    );

    // Hit target height >= 24 pt
    assert!(
        h.editor.ui.states.state_row_rects.contains_key("hover"),
        "hover state card rect"
    );
    let hit_violations = h.check_hit_targets(egui::vec2(24.0, 24.0));
    assert!(
        hit_violations.is_empty(),
        "expected no hit-target violations >= 24 pt, got {hit_violations:?}"
    );
}

#[test]
fn choose_image_in_state_card_emits_pick_image_with_state_pending_pick() {
    let mut h = TestHarness::new(image_course(None));
    open_image_states(&mut h);

    let choose = h.find_text("Choose image…").expect("Choose image…");
    let requests = h.click_at_requests(choose);

    assert_eq!(
        h.editor.pending_image_pick,
        Some(PendingImagePick::StateImage {
            slide: "s1".to_owned(),
            object: "img1".to_owned(),
            state: "hover".to_owned(),
        })
    );
    assert!(
        requests
            .iter()
            .any(|r| matches!(r, EditorRequest::PickImage)),
        "expected EditorRequest::PickImage, got {requests:?}"
    );
}

#[test]
fn import_image_sets_only_state_src_leaves_object_src_unchanged_and_undoes() {
    let mut h = TestHarness::new(image_course(None));
    open_image_states(&mut h);

    let choose = h.find_text("Choose image…").expect("Choose image…");
    h.click_at(choose);
    h.idle_frame();

    let imported = h
        .editor
        .import_image("hover-pic.png", &png_bytes())
        .expect("import_image should succeed");
    h.idle_frame();

    assert_eq!(state_src(&h, "hover").as_deref(), Some(imported.as_str()));
    assert_eq!(
        object_src(&h),
        "base.png",
        "object's own src must not change"
    );
    assert!(
        h.editor.media.iter().any(|(name, _)| name == &imported),
        "imported bytes must be stored in media"
    );
    assert!(h.editor.pending_image_pick.is_none());

    // One undo restores state.src to None
    h.editor.session.undo().expect("undo");
    h.idle_frame();
    assert_eq!(state_src(&h, "hover"), None);
}

#[test]
fn clear_button_resets_state_src_in_one_undo_step() {
    let mut h = TestHarness::new(image_course(Some("proj-alt.png")));
    open_image_states(&mut h);

    assert!(
        h.find_text("Replace…").is_some(),
        "when state src is set, button should say Replace…"
    );
    let clear = h
        .find_text("Clear")
        .expect("Clear button should be visible");
    h.click_at(clear);
    h.idle_frame();

    assert_eq!(state_src(&h, "hover"), None);

    h.editor.session.undo().expect("undo");
    h.idle_frame();
    assert_eq!(state_src(&h, "hover").as_deref(), Some("proj-alt.png"));
}

#[test]
fn from_project_menu_sets_state_src_without_copying_bytes() {
    let mut h = TestHarness::new(image_course(None));
    open_image_states(&mut h);
    let media_count_before = h.editor.media.len();

    let menu = h.find_text("From project…").expect("From project…");
    h.click_at(menu);
    h.idle_frame();

    let item = h
        .find_text_last("proj-alt.png")
        .expect("proj-alt.png in From project… menu");
    h.click_at(item);
    h.idle_frame();

    assert_eq!(state_src(&h, "hover").as_deref(), Some("proj-alt.png"));
    assert_eq!(
        h.editor.media.len(),
        media_count_before,
        "From project… must not copy/duplicate media bytes"
    );
}

#[test]
fn deleting_state_before_import_image_returns_changes_nothing() {
    let mut h = TestHarness::new(image_course(None));
    open_image_states(&mut h);

    let choose = h.find_text("Choose image…").expect("Choose image…");
    h.click_at(choose);
    h.idle_frame();

    // Delete the state before the file dialog returns
    h.editor.run(
        "object.delete_state",
        serde_json::json!({
            "slide": "s1",
            "object": "img1",
            "name": "hover",
        }),
    );
    let media_before = h.editor.media.len();
    let rev_before = h.editor.session.revision;

    let result = h.editor.import_image("late.png", &png_bytes());
    h.idle_frame();

    assert!(result.is_none());
    assert_eq!(h.editor.media.len(), media_before);
    assert_eq!(h.editor.session.revision, rev_before);
    assert_eq!(object_src(&h), "base.png");
    assert!(h.editor.ui.placement_tool.is_none());
}

#[test]
fn state_card_shows_unsafe_path_in_danger_with_warning() {
    let mut h = TestHarness::new(image_course(Some("/Users/noe/Desktop/cat.png")));
    open_image_states(&mut h);

    assert!(
        h.find_text("File not in the course. Choose image…")
            .is_some(),
        "unsafe path must show 'File not in the course. Choose image…'"
    );
    assert!(
        h.find_text_containing("cat.png").is_some(),
        "unsafe path should be displayed"
    );
    assert!(
        h.find_text("Choose image…").is_some(),
        "unsafe path should offer Choose image…"
    );
}
