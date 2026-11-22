use crate::{Course, Severity};

fn course_with_master_object(obj_json: serde_json::Value) -> Course {
    let raw = serde_json::json!({
        "schema_version": 1,
        "id": "c-m29-validate",
        "title": "M29 Validate Test",
        "slide_size": { "w": 1280, "h": 720 },
        "masters": [{
            "id": "m1",
            "title": "Title Master",
            "objects": [obj_json]
        }],
        "scenes": [{
            "id": "scene-1",
            "title": "Scene 1",
            "slides": [{
                "id": "s1",
                "title": "Slide 1",
                "objects": []
            }]
        }]
    });
    serde_json::from_value(raw).expect("valid course")
}

fn course_with_slide_object(obj_json: serde_json::Value) -> Course {
    let raw = serde_json::json!({
        "schema_version": 1,
        "id": "c-m29-validate",
        "title": "M29 Validate Test",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "scene-1",
            "title": "Scene 1",
            "slides": [{
                "id": "s1",
                "title": "Slide 1",
                "objects": [obj_json]
            }]
        }]
    });
    serde_json::from_value(raw).expect("valid course")
}

#[test]
fn master_object_with_states_produces_validation_warning_naming_master() {
    let course = course_with_master_object(serde_json::json!({
        "id": "btn-1",
        "name": "Master Button",
        "frame": { "x": 100, "y": 100, "w": 200, "h": 60 },
        "kind": { "type": "button", "label": "Next" },
        "states": [{ "name": "hover", "fill": "#FF0000" }]
    }));
    let issues = course.validate();
    assert!(
        issues.iter().all(|i| i.severity != Severity::Error),
        "existing courses with states on masters must still open (warning only): {issues:?}"
    );
    assert!(
        issues.iter().any(|i| {
            i.severity == Severity::Warning
                && i.message.contains("Title Master")
                && i.path.contains("/masters/0/objects/0/states")
        }),
        "expected warning naming the master: {issues:?}"
    );
}

#[test]
fn hidden_state_def_produces_validation_warning() {
    let course = course_with_slide_object(serde_json::json!({
        "id": "btn-1",
        "name": "Button 1",
        "frame": { "x": 100, "y": 100, "w": 200, "h": 60 },
        "kind": { "type": "button", "label": "Next" },
        "states": [{ "name": "hidden", "fill": "#FF0000" }]
    }));
    let issues = course.validate();
    assert!(
        issues.iter().all(|i| i.severity != Severity::Error),
        "hidden StateDef must warn, not error: {issues:?}"
    );
    assert!(
        issues
            .iter()
            .any(|i| { i.severity == Severity::Warning && i.path.ends_with("/states/0") }),
        "expected warning on /states/0 for hidden StateDef: {issues:?}"
    );
}

#[test]
fn more_than_32_states_produces_validation_warning() {
    let states: Vec<serde_json::Value> = (0..33)
        .map(|i| serde_json::json!({ "name": format!("s{i}") }))
        .collect();
    let course = course_with_slide_object(serde_json::json!({
        "id": "btn-1",
        "name": "Button 1",
        "frame": { "x": 100, "y": 100, "w": 200, "h": 60 },
        "kind": { "type": "button", "label": "Next" },
        "states": states
    }));
    let issues = course.validate();
    assert!(
        issues.iter().all(|i| i.severity != Severity::Error),
        "> 32 states must warn, not error: {issues:?}"
    );
    assert!(
        issues
            .iter()
            .any(|i| i.severity == Severity::Warning && i.path.ends_with("/states")),
        "expected warning on /states when > 32 states: {issues:?}"
    );
}

#[test]
fn state_name_over_64_chars_produces_validation_warning() {
    let long_name = "a".repeat(65);
    let course = course_with_slide_object(serde_json::json!({
        "id": "btn-1",
        "name": "Button 1",
        "frame": { "x": 100, "y": 100, "w": 200, "h": 60 },
        "kind": { "type": "button", "label": "Next" },
        "states": [{ "name": long_name }]
    }));
    let issues = course.validate();
    assert!(
        issues.iter().all(|i| i.severity != Severity::Error),
        "> 64 char state name must warn, not error: {issues:?}"
    );
    assert!(
        issues
            .iter()
            .any(|i| i.severity == Severity::Warning && i.path.ends_with("/states/0")),
        "expected warning on /states/0 for > 64 char state name: {issues:?}"
    );
}

#[test]
fn legacy_03x_showcase_and_reference_course_round_trip_states_unchanged() {
    for rel in [
        "../../examples/showcase/course.json",
        "../../examples/reference-course/course.json",
    ] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
        let raw = std::fs::read_to_string(&path).expect("read example course");
        let v_before: serde_json::Value = serde_json::from_str(&raw).expect("parse raw json");
        let course: Course = serde_json::from_str(&raw).expect("deserialize course");
        let reserialized = serde_json::to_value(&course).expect("reserialize course");

        for (si, (sb, sa)) in v_before["scenes"][0]["slides"]
            .as_array()
            .unwrap()
            .iter()
            .zip(reserialized["scenes"][0]["slides"].as_array().unwrap())
            .enumerate()
        {
            for (oi, (ob, oa)) in sb["objects"]
                .as_array()
                .unwrap()
                .iter()
                .zip(sa["objects"].as_array().unwrap())
                .enumerate()
            {
                assert_eq!(
                    ob.get("states"),
                    oa.get("states"),
                    "{rel} slide {si} object {oi} states changed on round-trip"
                );
            }
        }
    }
}

#[test]
fn legacy_state_def_refuses_new_m29_fields_loudly_and_accepts_legacy_only() {
    #[derive(Debug, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    #[allow(dead_code)]
    struct LegacyStateDef {
        name: String,
        #[serde(default)]
        fill: Option<crate::Color>,
        #[serde(default)]
        text: Option<String>,
        #[serde(default)]
        src: Option<String>,
    }

    let legacy_only = crate::StateDef {
        name: "hover".into(),
        fill: Some(crate::Color("#112233".into())),
        text: Some("Hi".into()),
        src: None,
        ..Default::default()
    };
    let legacy_json = serde_json::to_string(&legacy_only).unwrap();
    assert!(
        serde_json::from_str::<LegacyStateDef>(&legacy_json).is_ok(),
        "M29 StateDef using only legacy fields must still deserialize in 0.3.x"
    );

    let with_glow = crate::StateDef {
        name: "hover".into(),
        glow: Some(crate::Glow {
            color: crate::Color("#2F6FEB".into()),
            size: 12.0,
        }),
        ..Default::default()
    };
    let err_glow =
        serde_json::from_str::<LegacyStateDef>(&serde_json::to_string(&with_glow).unwrap())
            .unwrap_err()
            .to_string();
    assert!(
        err_glow.contains("unknown field"),
        "expected unknown field error for glow, got: {err_glow}"
    );

    let with_clear = crate::StateDef {
        name: "disabled".into(),
        clear_shadow: true,
        ..Default::default()
    };
    let err_clear =
        serde_json::from_str::<LegacyStateDef>(&serde_json::to_string(&with_clear).unwrap())
            .unwrap_err()
            .to_string();
    assert!(
        err_clear.contains("unknown field"),
        "expected unknown field error for clear_shadow, got: {err_clear}"
    );
}

#[test]
fn tri_state_overrides_and_clear_flags_round_trip_json() {
    let inherit = crate::StateDef {
        name: "s_inherit".into(),
        ..Default::default()
    };
    let inherit_val = serde_json::to_value(&inherit).unwrap();
    assert_eq!(inherit_val, serde_json::json!({ "name": "s_inherit" }));

    let full = crate::StateDef {
        name: "s_set".into(),
        fill: Some(crate::Color("#112233".into())),
        text: Some("Label".into()),
        src: Some("media/a.png".into()),
        fill_style: Some(crate::FillStyle::Linear {
            angle: 90.0,
            stops: vec![
                crate::GradientStop {
                    color: crate::Color("#112233".into()),
                    position: 0.0,
                },
                crate::GradientStop {
                    color: crate::Color("#445566".into()),
                    position: 1.0,
                },
            ],
        }),
        outline: Some(crate::Outline {
            color: crate::Color("#FFFFFF".into()),
            width: 2.0,
            dash: crate::DashStyle::Dash,
            dash_pattern: None,
            join: crate::LineJoin::Round,
            miter_limit: None,
        }),
        text_style: Some(crate::TextStyle::default()),
        runs: Some(vec![crate::TextRun {
            text: "Label".into(),
            bold: Some(true),
            italic: None,
            underline: None,
            strikethrough: None,
            baseline: None,
            size: None,
            color: None,
            font: None,
            letter_spacing: None,
            link: None,
        }]),
        paragraphs: Some(vec![crate::Paragraph::default()]),
        autofit: Some(crate::Autofit::Shrink),
        opacity: Some(0.5),
        shadow: Some(crate::Shadow {
            color: crate::Color("#00000080".into()),
            offset_x: 2.0,
            offset_y: 4.0,
            blur: 8.0,
        }),
        glow: Some(crate::Glow {
            color: crate::Color("#2F6FEB".into()),
            size: 10.0,
        }),
        crop: Some(crate::CropRect {
            x: 0.1,
            y: 0.1,
            w: 0.8,
            h: 0.8,
        }),
        image_shape: Some(crate::ShapeKind::Ellipse),
        rotation: Some(45.0),
        flip_h: Some(true),
        flip_v: Some(false),
        ..Default::default()
    };
    let full_json = serde_json::to_string(&full).unwrap();
    let full_back: crate::StateDef = serde_json::from_str(&full_json).unwrap();
    assert_eq!(full, full_back);

    let cleared = crate::StateDef {
        name: "s_clear".into(),
        clear_fill_style: true,
        clear_outline: true,
        clear_shadow: true,
        clear_glow: true,
        clear_crop: true,
        clear_image_shape: true,
        ..Default::default()
    };
    let cleared_val = serde_json::to_value(&cleared).unwrap();
    assert_eq!(cleared_val["clear_fill_style"], true);
    assert_eq!(cleared_val["clear_outline"], true);
    assert_eq!(cleared_val["clear_shadow"], true);
    assert_eq!(cleared_val["clear_glow"], true);
    assert_eq!(cleared_val["clear_crop"], true);
    assert_eq!(cleared_val["clear_image_shape"], true);
    assert!(cleared_val.get("fill_style").is_none());
    let cleared_back: crate::StateDef = serde_json::from_value(cleared_val).unwrap();
    assert_eq!(cleared, cleared_back);
}

#[test]
fn mutual_exclusion_between_nullable_field_and_clear_flag_errors() {
    let cases = [
        serde_json::json!({
            "name": "s1",
            "fill_style": { "type": "pattern", "preset": "dots", "fg": "#000000", "bg": "#FFFFFF" },
            "clear_fill_style": true
        }),
        serde_json::json!({
            "name": "s2",
            "outline": { "color": "#000000", "width": 2.0 },
            "clear_outline": true
        }),
        serde_json::json!({
            "name": "s3",
            "shadow": { "color": "#000000", "offset_x": 1.0, "offset_y": 1.0, "blur": 2.0 },
            "clear_shadow": true
        }),
        serde_json::json!({
            "name": "s4",
            "glow": { "color": "#2F6FEB", "size": 6.0 },
            "clear_glow": true
        }),
        serde_json::json!({
            "name": "s5",
            "crop": { "x": 0.0, "y": 0.0, "w": 0.5, "h": 0.5 },
            "clear_crop": true
        }),
        serde_json::json!({
            "name": "s6",
            "image_shape": "ellipse",
            "clear_image_shape": true
        }),
    ];
    for state_json in cases {
        let course = course_with_slide_object(serde_json::json!({
            "id": "btn-1",
            "name": "Button 1",
            "frame": { "x": 100, "y": 100, "w": 200, "h": 60 },
            "kind": { "type": "button", "label": "Next" },
            "states": [state_json]
        }));
        let issues = course.validate();
        assert!(
            issues.iter().any(|i| i.severity == Severity::Error),
            "expected error for mutual exclusion: {issues:?}"
        );
    }
}

#[test]
fn state_def_rejects_invalid_widened_fields() {
    let bad_states = [
        serde_json::json!({ "name": "s_bad_outline", "outline": { "color": "not-hex", "width": 2.0 } }),
        serde_json::json!({ "name": "s_bad_outline_w", "outline": { "color": "#000000", "width": 99.0 } }),
        serde_json::json!({ "name": "s_bad_glow", "glow": { "color": "bad", "size": 10.0 } }),
        serde_json::json!({ "name": "s_bad_shadow", "shadow": { "color": "#000000", "offset_x": 0.0, "offset_y": 0.0, "blur": 2.0 } }),
        serde_json::json!({ "name": "s_bad_crop", "crop": { "x": 0.5, "y": 0.5, "w": 0.8, "h": 0.8 } }),
        serde_json::json!({ "name": "s_bad_fill_path", "fill_style": { "type": "picture", "src": "../secret.png" } }),
        serde_json::json!({ "name": "s_bad_ts_color", "text_style": { "font": "Inter", "size": 16.0, "color": "bad" } }),
    ];
    for state_json in bad_states {
        let mut course = course_with_slide_object(serde_json::json!({
            "id": "btn-1",
            "name": "Button 1",
            "frame": { "x": 100, "y": 100, "w": 200, "h": 60 },
            "kind": { "type": "button", "label": "Next" },
            "states": [state_json.clone()]
        }));
        if state_json["name"] == "s_bad_shadow" {
            course.scenes[0].slides[0].objects[0].states[0]
                .shadow
                .as_mut()
                .unwrap()
                .offset_x = 999.0;
        }
        let issues = course.validate();
        assert!(
            issues.iter().any(|i| i.severity == Severity::Error),
            "expected validation error for {state_json}: {issues:?}"
        );
    }

    // In-memory out-of-range glow size is also caught by validate().
    let mut course = course_with_slide_object(serde_json::json!({
        "id": "btn-1",
        "name": "Button 1",
        "frame": { "x": 100, "y": 100, "w": 200, "h": 60 },
        "kind": { "type": "button", "label": "Next" },
        "states": [{ "name": "hover", "glow": { "color": "#000000", "size": 10.0 } }]
    }));
    course.scenes[0].slides[0].objects[0].states[0]
        .glow
        .as_mut()
        .unwrap()
        .size = 500.0;
    let issues = course.validate();
    assert!(
        issues
            .iter()
            .any(|i| i.severity == Severity::Error && i.path.ends_with("/glow/size")),
        "expected glow size error: {issues:?}"
    );
}

#[test]
fn state_def_deser_clamps_opacity_and_normalizes_rotation_and_validate_rejects_non_finite_floats() {
    let deser: crate::StateDef = serde_json::from_value(serde_json::json!({
        "name": "hover",
        "opacity": 1.5,
        "rotation": -45.0
    }))
    .unwrap();
    assert_eq!(deser.opacity, Some(1.0));
    assert_eq!(deser.rotation, Some(315.0));

    let mut course = course_with_slide_object(serde_json::json!({
        "id": "btn-1",
        "name": "Button 1",
        "frame": { "x": 100, "y": 100, "w": 200, "h": 60 },
        "kind": { "type": "button", "label": "Next" },
        "states": [{ "name": "hover" }]
    }));
    course.scenes[0].slides[0].objects[0].states[0].opacity = Some(f32::NAN);
    course.scenes[0].slides[0].objects[0].states[0].rotation = Some(f32::INFINITY);
    let issues = course.validate();
    assert!(
        issues
            .iter()
            .any(|i| i.severity == Severity::Error && i.path.ends_with("/opacity")),
        "expected opacity error: {issues:?}"
    );
    assert!(
        issues
            .iter()
            .any(|i| i.severity == Severity::Error && i.path.ends_with("/rotation")),
        "expected rotation error: {issues:?}"
    );
}

#[test]
fn state_def_rejects_unknown_json_fields() {
    let err = serde_json::from_value::<crate::StateDef>(serde_json::json!({
        "name": "hover",
        "bogus_key": 123
    }))
    .unwrap_err()
    .to_string();
    assert!(err.contains("unknown field"), "got: {err}");
}

#[test]
fn recolor_course_updates_widened_state_def_colors_and_preserves_alpha() {
    let course = course_with_slide_object(serde_json::json!({
        "id": "shp-1",
        "name": "Shape 1",
        "frame": { "x": 100, "y": 100, "w": 200, "h": 100 },
        "kind": { "type": "shape", "shape": "rectangle", "fill": "#112233" },
        "states": [{
            "name": "hover",
            "fill": "#AABBCC80",
            "fill_style": {
                "type": "linear",
                "angle": 90.0,
                "stops": [
                    { "position": 0.0, "color": "#AABBCC" },
                    { "position": 1.0, "color": "#000000" }
                ]
            },
            "outline": { "color": "#AABBCC40", "width": 2.0 },
            "shadow": { "color": "#AABBCC", "blur": 4.0, "offset_x": 1.0, "offset_y": 2.0 },
            "glow": { "color": "#AABBCC99", "size": 6.0 },
            "text_style": { "font": "Inter", "size": 16.0, "color": "#AABBCC" },
            "runs": [
                { "text": "Hi", "color": "#AABBCC20" }
            ]
        }]
    }));
    let (recolored, changed) =
        crate::recolor_course(&course, "#AABBCC", "#445566", false).expect("recolor");
    assert_eq!(changed, 7);
    let st = &recolored.scenes[0].slides[0].objects[0].states[0];
    assert_eq!(st.fill.as_ref().map(|c| c.0.as_str()), Some("#44556680"));
    assert_eq!(
        st.outline.as_ref().map(|o| o.color.0.as_str()),
        Some("#44556640")
    );
    assert_eq!(
        st.shadow.as_ref().map(|s| s.color.0.as_str()),
        Some("#445566")
    );
    assert_eq!(
        st.glow.as_ref().map(|g| g.color.0.as_str()),
        Some("#44556699")
    );
    assert_eq!(
        st.text_style.as_ref().map(|t| t.color.0.as_str()),
        Some("#445566")
    );
    assert_eq!(
        st.runs
            .as_ref()
            .and_then(|r| r[0].color.as_ref())
            .map(|c| c.0.as_str()),
        Some("#44556620")
    );
    if let Some(crate::FillStyle::Linear { stops, .. }) = &st.fill_style {
        assert_eq!(stops[0].color.0, "#445566");
        assert_eq!(stops[1].color.0, "#000000");
    } else {
        panic!("expected linear fill_style");
    }
}

#[test]
fn course_strings_and_replace_keep_state_runs_synced_and_protect_vars() {
    let mut course = course_with_slide_object(serde_json::json!({
        "id": "btn-1",
        "name": "Button 1",
        "frame": { "x": 100, "y": 100, "w": 200, "h": 60 },
        "kind": { "type": "button", "label": "Next" },
        "states": [{
            "name": "hover",
            "text": "Hello %user_name%, click next",
            "runs": [
                { "text": "Hello ", "bold": true },
                { "text": "%user_name%, click " },
                { "text": "next", "italic": true }
            ]
        }]
    }));
    let strings = crate::strings::course_strings(&course);
    let hover_item = strings
        .iter()
        .find(|s| s.location.field == "state/hover/text")
        .expect("hover text string item");
    assert_eq!(hover_item.text, "Hello %user_name%, click next");

    let target = crate::strings::string_mut(&mut course, &hover_item.location)
        .expect("mutable state text target");
    assert!(
        matches!(target, crate::strings::StringTarget::Runs { .. }),
        "state with runs must resolve to StringTarget::Runs"
    );

    // Replace "next" -> "forward" and also try replacing "user_name" (must be protected!)
    let outcome1 =
        crate::strings::replace_in_target(target, "next", "forward", true, false, usize::MAX);
    assert_eq!(outcome1.replaced, 1);
    let target2 = crate::strings::string_mut(&mut course, &hover_item.location)
        .expect("mutable state text target");
    let outcome2 =
        crate::strings::replace_in_target(target2, "user_name", "someone", true, false, usize::MAX);
    assert_eq!(outcome2.replaced, 0);
    assert_eq!(outcome2.skipped, 1);

    let st = &course.scenes[0].slides[0].objects[0].states[0];
    assert_eq!(st.text.as_deref(), Some("Hello %user_name%, click forward"));
    let runs = st.runs.as_ref().expect("runs kept");
    let joined: String = runs.iter().map(|r| r.text.as_str()).collect();
    assert_eq!(joined, "Hello %user_name%, click forward");
    assert_eq!(runs[0].bold, Some(true));
    assert_eq!(runs[2].italic, Some(true));
}

#[test]
fn media_references_includes_state_fill_style_picture() {
    let course = course_with_slide_object(serde_json::json!({
        "id": "shp-1",
        "name": "Shape 1",
        "frame": { "x": 100, "y": 100, "w": 200, "h": 100 },
        "kind": { "type": "shape", "shape": "rectangle", "fill": "#112233" },
        "states": [{
            "name": "hover",
            "fill_style": { "type": "picture", "src": "hover-bg.png" }
        }]
    }));
    let refs = crate::media_references(&course);
    assert!(
        refs.iter().any(|r| r.src == "hover-bg.png"),
        "expected hover-bg.png in media_references: {refs:?}"
    );
}

#[test]
fn unsafe_paths_in_state_src_and_fill_style_are_flagged_by_validate() {
    for bad in [
        "../secret.png",
        "C:\\Users\\a.png",
        "\"a.png\"",
        "\0bad.png",
        "https://evil/a.png",
    ] {
        let course = course_with_slide_object(serde_json::json!({
            "id": "img-1",
            "name": "Image 1",
            "frame": { "x": 100, "y": 100, "w": 200, "h": 100 },
            "kind": { "type": "image", "src": "ok.png", "alt": "ok" },
            "states": [{
                "name": "hover",
                "src": bad,
                "fill_style": { "type": "picture", "src": bad }
            }]
        }));
        let issues = course.validate();
        assert!(
            issues.iter().any(|i| i.path.ends_with("/states/0/src")),
            "expected state.src issue for {bad:?}: {issues:?}"
        );
        assert!(
            issues
                .iter()
                .any(|i| i.severity == Severity::Error
                    && i.path.ends_with("/states/0/fill_style/src")),
            "expected state.fill_style.src error for {bad:?}: {issues:?}"
        );
    }
}
