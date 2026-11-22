//! Headless tests for the Format panel's fill type, gradient colours and
//! picture fill, from a tester's report on the hello course.

use crate::properties::InspectorTab;
use crate::test_harness::TestHarness;
use coursecraft_course::{Course, FillStyle, ObjectKind};

fn hello() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    Course::from_json(&json).expect("valid course")
}

fn select_start_button(harness: &mut TestHarness) {
    harness.editor.selection.slide = "s-intro".into();
    harness.editor.selection.object = Some("btn-start".into());
    harness.editor.ui.inspector_tab = InspectorTab::Format;
    harness.idle_frame();
    harness.idle_frame();
}

fn start_button_kind(harness: &TestHarness) -> ObjectKind {
    harness
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|slide| slide.object("btn-start"))
        .map(|(object, _)| object.kind.clone())
        .expect("start button")
}

fn fill_style(kind: &ObjectKind) -> Option<FillStyle> {
    match kind {
        ObjectKind::Button { fill_style, .. } | ObjectKind::Shape { fill_style, .. } => {
            fill_style.clone()
        }
        _ => None,
    }
}

fn png() -> Vec<u8> {
    let pixels = vec![200u8; 4 * 4 * 4];
    coursecraft_media::encode_png_rgba(4, 4, &pixels).expect("png")
}

/// In a short window the Fill type menu opens over the Slide Layers panel. The
/// click on "Radial gradient" used to select the layer row underneath too: the
/// layer's content showed over the slide and the inspector jumped to Slide.
#[test]
fn radial_gradient_menu_over_the_layers_panel_keeps_the_selection() {
    let mut harness = TestHarness::new_with_size(hello(), (1100.0, 700.0));
    select_start_button(&mut harness);
    let combo = harness
        .find_text_in("Solid", harness.right_panel_rect())
        .expect("fill type combo");
    harness.click_at(combo);
    harness.idle_frame();
    let radial = harness
        .find_text_last("Radial gradient")
        .expect("radial gradient item");
    assert!(
        radial.y > harness.right_panel_rect().max.y,
        "the menu item sits over the panel below the inspector, as in the report"
    );
    let revision = harness.editor.session.revision;
    harness.click_at(radial);
    harness.idle_frame();

    assert_eq!(
        harness.editor.selection.layer, None,
        "no layer was selected"
    );
    assert_eq!(
        harness.editor.selection.object.as_deref(),
        Some("btn-start")
    );
    assert_eq!(harness.editor.ui.inspector_tab, InspectorTab::Format);
    assert!(matches!(
        fill_style(&start_button_kind(&harness)),
        Some(FillStyle::Radial { .. })
    ));
    assert_eq!(
        harness.editor.session.revision,
        revision + 1,
        "one undo step"
    );
}

/// Both gradient colours show in the panel, with their position fields inside it.
#[test]
fn linear_gradient_shows_two_colours_inside_the_panel() {
    let mut harness = TestHarness::new(hello());
    select_start_button(&mut harness);
    harness.pick_combo("Solid", "Linear gradient");
    let panel = harness.right_panel_rect();
    for label in ["Colour 1", "Colour 2"] {
        assert!(
            harness.find_text_in(label, panel).is_some(),
            "{label} row is visible"
        );
    }
    let report = harness.layout_report(2);
    assert!(
        report.findings.is_empty(),
        "nothing is clipped or overlapping: {:?}",
        report.findings
    );
}

/// With no picture in the course, choosing Picture opens the file picker, and the
/// chosen file becomes the fill. The button stays a button.
#[test]
fn picture_fill_with_no_pictures_opens_the_file_picker_and_fills_the_button() {
    let mut harness = TestHarness::new(hello());
    select_start_button(&mut harness);
    let objects_before = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .map(|slide| slide.objects.len());
    harness.pick_combo("Solid", "Picture");
    assert_eq!(
        harness.editor.pending_image_pick,
        Some(crate::PendingImagePick::PictureFill {
            slide: "s-intro".to_owned(),
            object: "btn-start".to_owned(),
        })
    );
    assert_eq!(
        fill_style(&start_button_kind(&harness)),
        None,
        "the fill waits for the file, so cancelling the picker changes nothing"
    );

    harness.editor.import_image("photo.png", &png());
    harness.idle_frame();

    let kind = start_button_kind(&harness);
    assert!(matches!(kind, ObjectKind::Button { .. }), "still a button");
    assert!(
        matches!(fill_style(&kind), Some(FillStyle::Picture { ref src, .. }) if src == "photo.png"),
        "picture fill uses the chosen file: {kind:?}"
    );
    assert!(
        harness.editor.ui.placement_tool.is_none(),
        "nothing to place"
    );
    assert!(harness.editor.pending_image_pick.is_none());
    assert_eq!(
        harness
            .editor
            .session
            .course
            .slide("s-intro")
            .map(|slide| slide.objects.len()),
        objects_before
    );
}

/// The Image menu always offers "Choose a file…", and lists only pictures.
#[test]
fn picture_menu_offers_choose_a_file_and_lists_only_pictures() {
    let mut harness = TestHarness::new(hello());
    harness.editor.import_image("photo.png", &png());
    harness.editor.ui.placement_tool = None;
    harness
        .editor
        .media
        .push(("voice.mp3".to_owned(), std::sync::Arc::from(&b"ID3"[..])));
    select_start_button(&mut harness);
    harness.pick_combo("Solid", "Picture");
    assert!(
        matches!(fill_style(&start_button_kind(&harness)), Some(FillStyle::Picture { ref src, .. }) if src == "photo.png"),
        "the course's picture is used straight away"
    );
    assert!(harness.editor.pending_image_pick.is_none());

    let combo = harness.find_text_last("photo.png").expect("image combo");
    harness.click_at(combo);
    harness.idle_frame();
    assert!(harness.find_text_last("Choose a file…").is_some());
    assert!(
        harness.find_text_last("voice.mp3").is_none(),
        "audio is not offered"
    );
    let choose = harness
        .find_text_last("Choose a file…")
        .expect("choose item");
    harness.click_at(choose);
    harness.idle_frame();
    assert_eq!(
        harness.editor.pending_image_pick,
        Some(crate::PendingImagePick::PictureFill {
            slide: "s-intro".to_owned(),
            object: "btn-start".to_owned(),
        })
    );
}

/// A picture-fill request left over from a cancelled picker does not hijack a
/// later Insert ▸ Image once the button is no longer selected.
#[test]
fn a_stale_picture_fill_request_does_not_capture_a_later_image() {
    let mut harness = TestHarness::new(hello());
    select_start_button(&mut harness);
    harness.pick_combo("Solid", "Picture");
    assert!(harness.editor.pending_image_pick.is_some());
    // The picker was cancelled; the author clicks away and inserts a picture.
    harness.editor.selection.clear_objects();
    harness.idle_frame();
    harness.editor.import_image("photo.png", &png());
    assert!(
        harness.editor.ui.placement_tool.is_some(),
        "the picture is placed as usual"
    );
    assert!(
        !matches!(
            fill_style(&start_button_kind(&harness)),
            Some(FillStyle::Picture { ref src, .. }) if src == "photo.png"
        ),
        "the button did not take the picture"
    );
}

/// Every object type's Format panel, scrolled top to bottom, keeps its controls
/// inside the panel and its labels apart. One row wider than the panel used to
/// widen the whole panel, pushing every field past its edge. Text longer than
/// its own field (a long alt text in a one-line field) is fine and not counted.
#[test]
fn format_panel_controls_stay_inside_the_panel_for_every_object_type() {
    let mut problems = Vec::new();
    for path in [
        "examples/showcase/course.json",
        "examples/healthy-habits/course.json",
    ] {
        let json = std::fs::read_to_string(format!("{}/../../{path}", env!("CARGO_MANIFEST_DIR")))
            .expect("course");
        let course = Course::from_json(&json).expect("valid course");
        let mut seen = std::collections::HashSet::new();
        let mut targets = Vec::new();
        for slide in course.slides() {
            let layers = std::iter::once((None, &slide.objects)).chain(
                slide
                    .layers
                    .iter()
                    .map(|layer| (Some(layer.id.clone()), &layer.objects)),
            );
            for (layer, objects) in layers {
                for object in objects {
                    let kind = serde_json::to_value(&object.kind)
                        .ok()
                        .and_then(|v| v.get("type").and_then(|t| t.as_str()).map(str::to_owned))
                        .unwrap_or_default();
                    if seen.insert(kind.clone()) {
                        targets.push((slide.id.clone(), layer.clone(), object.id.clone(), kind));
                    }
                }
            }
        }
        for (slide, layer, object, kind) in targets {
            let mut harness = TestHarness::new_with_size(course.clone(), (1280.0, 800.0));
            harness.editor.selection.slide = slide;
            harness.editor.selection.layer = layer;
            harness.editor.selection.set_many([object.as_str()]);
            harness.editor.ui.inspector_tab = InspectorTab::Format;
            for _ in 0..8 {
                let report = harness.layout_report(3);
                problems.extend(
                    report
                        .findings
                        .into_iter()
                        .filter(|f| f.starts_with("clipped-widget") || f.starts_with("overlap"))
                        .map(|f| format!("{kind} {object}: {f}")),
                );
                let panel = harness.right_panel_rect();
                harness.scroll_over(panel.center(), -200.0);
            }
        }
    }
    problems.sort();
    problems.dedup();
    assert!(
        problems.is_empty(),
        "Format panel problems:\n{}",
        problems.join("\n")
    );
}

fn about_layer_row(harness: &mut TestHarness) -> egui::Pos2 {
    let panel = harness.right_panel_rect();
    harness
        .find_texts("About")
        .into_iter()
        .rfind(|pos| pos.x > panel.min.x && pos.y > panel.max.y)
        .expect("About row in the Slide Layers panel")
}

/// A press that starts while a menu is open only closes the menu. It used to
/// reach the Slide Layers row underneath as well, so dismissing the Fill type
/// menu opened a layer and switched the inspector to Slide. A later, ordinary
/// click on the row still selects the layer.
#[test]
fn clicking_away_from_an_open_menu_does_not_click_the_layer_underneath() {
    let mut harness = TestHarness::new_with_size(hello(), (1100.0, 700.0));
    select_start_button(&mut harness);
    let row = about_layer_row(&mut harness);
    let combo = harness
        .find_text_in("Solid", harness.right_panel_rect())
        .expect("fill type combo");
    harness.click_at(combo);
    harness.idle_frame();
    assert!(egui::Popup::is_any_open(&harness.ctx), "the menu is open");

    harness.click_at(row);
    harness.idle_frame();
    assert!(!egui::Popup::is_any_open(&harness.ctx), "the menu closed");
    assert_eq!(harness.editor.selection.layer, None, "no layer was opened");
    assert_eq!(
        harness.editor.selection.object.as_deref(),
        Some("btn-start")
    );
    assert_eq!(harness.editor.ui.inspector_tab, InspectorTab::Format);

    harness.click_at(row);
    harness.idle_frame();
    assert!(
        harness.editor.selection.layer.is_some(),
        "with no menu open the row takes the click"
    );
}

struct NoMedia;
impl coursecraft_render::MediaSource for NoMedia {
    fn get(&self, _src: &str) -> coursecraft_render::MediaFetch {
        coursecraft_render::MediaFetch::Missing
    }
}

fn select_about_panel(harness: &mut TestHarness) {
    harness.editor.selection.slide = "s-intro".into();
    harness.editor.selection.layer = Some("about".into());
    harness.editor.selection.object = Some("about-panel".into());
    harness.editor.ui.inspector_tab = InspectorTab::Format;
    harness.idle_frame();
    harness.idle_frame();
}

fn scroll_to_text_in_panel(harness: &mut TestHarness, label: &str) -> egui::Pos2 {
    for _ in 0..40 {
        let panel = harness.right_panel_rect();
        if let Some(pos) = harness.find_text_in(label, panel) {
            return pos;
        }
        harness.scroll_over(panel.center(), -120.0);
    }
    panic!("'{label}' not found in right panel");
}

#[test]
fn three_stop_linear_gradient_on_shape_renders_all_three_colours() {
    let mut harness = TestHarness::new_with_size(hello(), (1280.0, 960.0));
    select_about_panel(&mut harness);

    harness.pick_combo("Solid", "Linear gradient");
    let add_btn = scroll_to_text_in_panel(&mut harness, "Add colour");
    harness.click_at(add_btn);
    harness.idle_frame();

    for (label, current_hex, hex) in [
        ("Colour 1", "#FFFFFFF0", "#0000FF"),
        ("Colour 2", "#FFFFFF", "#FF0000"),
        ("Colour 3", "#FFFFFF", "#FFFF00"),
    ] {
        let label_pos = scroll_to_text_in_panel(&mut harness, label);
        let field_pos = harness
            .find_texts(current_hex)
            .into_iter()
            .find(|p| (p.y - label_pos.y).abs() < 12.0 && p.x > label_pos.x)
            .unwrap_or_else(|| panic!("hex field '{current_hex}' on row {label}"));
        harness.set_field(field_pos, hex);
    }

    let kind = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .and_then(|s| s.layer("about"))
        .and_then(|l| l.objects.iter().find(|o| o.id == "about-panel"))
        .map(|o| o.kind.clone())
        .expect("about-panel");
    let ObjectKind::Shape {
        fill_style: Some(FillStyle::Linear { ref stops, .. }),
        ..
    } = kind
    else {
        panic!("expected linear gradient, got {kind:?}");
    };
    assert_eq!(stops.len(), 3, "course should have three stops");

    // Render the layer/slide and check that blue, red, and yellow all appear in about-panel (x=240..1040, y=140..580).
    let mut course = harness.editor.session.course.clone();
    // Move about-panel to base slide for render_slide_rgba or render via compose_authoring_layers.
    let obj = course
        .slide("s-intro")
        .unwrap()
        .layer("about")
        .unwrap()
        .objects
        .iter()
        .find(|o| o.id == "about-panel")
        .unwrap()
        .clone();
    course.scenes[0].slides[0].objects.push(obj);
    let img = coursecraft_render::render_slide_rgba(
        &course,
        "s-intro",
        &coursecraft_render::RenderOptions::default(),
        &NoMedia,
    )
    .expect("render slide");

    let mut has_blue = false;
    let mut has_red = false;
    let mut has_yellow = false;
    for x in (250..1030).step_by(10) {
        let idx = ((360 * img.width + x) * 4) as usize;
        let p = &img.pixels[idx..idx + 4];
        if p[2] > 200 && p[0] < 60 && p[1] < 60 {
            has_blue = true;
        }
        if p[0] > 200 && p[1] < 60 && p[2] < 60 {
            has_red = true;
        }
        if p[0] > 200 && p[1] > 200 && p[2] < 60 {
            has_yellow = true;
        }
    }
    assert!(
        has_blue && has_red && has_yellow,
        "expected blue={has_blue}, red={has_red}, yellow={has_yellow} in rendered gradient; stops={stops:?}"
    );
}

#[test]
fn three_stop_radial_gradient_on_button_renders_all_three_colours() {
    let mut harness = TestHarness::new_with_size(hello(), (1280.0, 960.0));
    select_start_button(&mut harness);
    harness.pick_combo("Solid", "Radial gradient");
    let add_btn = scroll_to_text_in_panel(&mut harness, "Add colour");
    harness.click_at(add_btn);
    harness.idle_frame();

    for (label, current_hex, hex) in [
        ("Colour 1", "#2F6FEB", "#0000FF"),
        ("Colour 2", "#FFFFFF", "#FF0000"),
        ("Colour 3", "#FFFFFF", "#FFFF00"),
    ] {
        let label_pos = scroll_to_text_in_panel(&mut harness, label);
        let field_pos = harness
            .find_texts(current_hex)
            .into_iter()
            .find(|p| (p.y - label_pos.y).abs() < 12.0 && p.x > label_pos.x)
            .unwrap_or_else(|| panic!("hex field '{current_hex}' on row {label}"));
        harness.set_field(field_pos, hex);
    }

    let img = coursecraft_render::render_slide_rgba(
        &harness.editor.session.course,
        "s-intro",
        &coursecraft_render::RenderOptions::default(),
        &NoMedia,
    )
    .expect("render slide");

    // btn-start is at x=80..280, y=520..580. Radial centre is (180, 550), corner is (83, 523).
    let mut has_blue = false;
    let mut has_red = false;
    let mut has_yellow = false;
    for x in (84..276).step_by(4) {
        for y in [523, 550, 576] {
            let idx = ((y * img.width + x) * 4) as usize;
            let p = &img.pixels[idx..idx + 4];
            if p[2] > 200 && p[0] < 60 && p[1] < 60 {
                has_blue = true;
            }
            if p[0] > 200 && p[1] < 60 && p[2] < 60 {
                has_red = true;
            }
            if p[0] > 200 && p[1] > 200 && p[2] < 60 {
                has_yellow = true;
            }
        }
    }
    assert!(
        has_blue && has_red && has_yellow,
        "expected blue={has_blue}, red={has_red}, yellow={has_yellow} on button radial gradient"
    );
}

#[test]
fn gradient_stops_can_be_reordered_positions_edited_undone_and_saved() {
    let mut harness = TestHarness::new_with_size(hello(), (1280.0, 960.0));
    select_start_button(&mut harness);
    harness.pick_combo("Solid", "Linear gradient");
    let add_btn = scroll_to_text_in_panel(&mut harness, "Add colour");
    harness.click_at(add_btn);
    harness.idle_frame();

    for (label, current_hex, hex) in [
        ("Colour 1", "#2F6FEB", "#0000FF"),
        ("Colour 2", "#FFFFFF", "#FF0000"),
        ("Colour 3", "#FFFFFF", "#FFFF00"),
    ] {
        let label_pos = scroll_to_text_in_panel(&mut harness, label);
        let field_pos = harness
            .find_texts(current_hex)
            .into_iter()
            .find(|p| (p.y - label_pos.y).abs() < 12.0 && p.x > label_pos.x)
            .unwrap_or_else(|| panic!("hex field '{current_hex}' on row {label}"));
        harness.set_field(field_pos, hex);
    }

    // With 3 stops (including the × button), nothing in the Format panel is clipped or overlapping.
    let report = harness.layout_report(2);
    assert!(
        report.findings.is_empty(),
        "three-stop gradient row fits inside panel: {:?}",
        report.findings
    );

    // Move Colour 2 (Red) up to Colour 1 -> order becomes Red, Blue, Yellow.
    let c2_pos = scroll_to_text_in_panel(&mut harness, "Colour 2");
    let up_btn = harness
        .find_texts("↑")
        .into_iter()
        .find(|p| (p.y - c2_pos.y).abs() < 12.0)
        .expect("move up button on Colour 2");
    harness.click_at(up_btn);
    harness.idle_frame();

    let stops = match fill_style(&start_button_kind(&harness)) {
        Some(FillStyle::Linear { stops, .. }) => stops,
        other => panic!("expected linear fill, got {other:?}"),
    };
    assert_eq!(
        stops.iter().map(|s| s.color.0.as_str()).collect::<Vec<_>>(),
        vec!["#FF0000", "#0000FF", "#FFFF00"]
    );

    // Edit Colour 2's position from 50% to 25%.
    let c2_pos = scroll_to_text_in_panel(&mut harness, "Colour 2");
    let up_c2 = harness
        .find_texts("↑")
        .into_iter()
        .find(|p| (p.y - c2_pos.y).abs() < 12.0)
        .expect("move up button on Colour 2");
    let pct_pos = egui::pos2(up_c2.x - 28.0, c2_pos.y);
    harness.set_field(pct_pos, "25");
    let stops = match fill_style(&start_button_kind(&harness)) {
        Some(FillStyle::Linear { stops, .. }) => stops,
        other => panic!("expected linear fill, got {other:?}"),
    };
    assert!(
        (stops[1].position - 0.25).abs() < 0.01,
        "position updated to 25%: {stops:?}"
    );

    // Undo restores 50%, then undo again restores Blue, Red, Yellow.
    let _ = harness.editor.session.undo();
    harness.idle_frame();
    let stops = match fill_style(&start_button_kind(&harness)) {
        Some(FillStyle::Linear { stops, .. }) => stops,
        other => panic!("expected linear fill, got {other:?}"),
    };
    assert!((stops[1].position - 0.5).abs() < 0.01);

    let _ = harness.editor.session.undo();
    harness.idle_frame();
    let stops = match fill_style(&start_button_kind(&harness)) {
        Some(FillStyle::Linear { stops, .. }) => stops,
        other => panic!("expected linear fill, got {other:?}"),
    };
    assert_eq!(
        stops.iter().map(|s| s.color.0.as_str()).collect::<Vec<_>>(),
        vec!["#0000FF", "#FF0000", "#FFFF00"]
    );

    // Redo + save & reopen preserves all 3 stops.
    let _ = harness.editor.session.redo();
    harness.idle_frame();
    let saved = harness.editor.session.course.to_json().expect("save");
    let reopened = Course::from_json(&saved).expect("reopen");
    let reopened_stops = match reopened
        .slide("s-intro")
        .and_then(|s| s.object("btn-start"))
        .map(|(o, _)| &o.kind)
    {
        Some(ObjectKind::Button {
            fill_style: Some(FillStyle::Linear { stops, .. }),
            ..
        }) => stops.clone(),
        other => panic!("expected linear fill after reopen, got {other:?}"),
    };
    assert_eq!(
        reopened_stops
            .iter()
            .map(|s| s.color.0.as_str())
            .collect::<Vec<_>>(),
        vec!["#FF0000", "#0000FF", "#FFFF00"]
    );
}
