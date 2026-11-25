//! Step 17: Full UI/UX sweep across all surfaces, sizes, themes, scales, host modes, and fixtures.

use crate::snapshot::{SnapshotSelect, WINDOWS};
use crate::test_harness::TestHarness;
use crate::uiux_allowlist::match_allowlist;
use crate::{BottomTab, InspectorTab};
use coursecraft_course::{Course, Value, VariableDef};
use coursecraft_render::ThemeMode;
use std::collections::BTreeMap;

const SHOWCASE: &str = include_str!("../../../examples/showcase/course.json");

const MATRIX_SIZES: [(f32, f32); 3] = [(1024.0, 600.0), (1280.0, 800.0), (1920.0, 1080.0)];
const MATRIX_THEMES: [(ThemeMode, &str); 2] =
    [(ThemeMode::Dark, "dark"), (ThemeMode::Light, "light")];
const MATRIX_SCALES: [f32; 2] = [1.0, 1.5];

fn showcase() -> Course {
    Course::from_json(SHOWCASE).unwrap_or_else(|e| panic!("showcase course: {e}"))
}

fn empty_course() -> Course {
    let mut course = showcase();
    course.title = "Empty Course".to_owned();
    course.variables.clear();
    course.resources.clear();
    course.glossary.clear();
    course.scenes.truncate(1);
    if let Some(scene) = course.scenes.first_mut() {
        scene.title = "Scene 1".to_owned();
        scene.slides.truncate(1);
        if let Some(slide) = scene.slides.first_mut() {
            slide.title = "Slide 1".to_owned();
            slide.objects.clear();
            slide.triggers.clear();
            slide.layers.clear();
        }
    }
    course
}

fn long_names_course() -> Course {
    let mut course = showcase();
    let long_120 = "A".repeat(40) + " LongCourseTitle_Unicode_Café_日本語_Check_" + &"B".repeat(38);
    course.title = long_120;
    for (i, scene) in course.scenes.iter_mut().enumerate() {
        scene.title = format!("Scene {i:02} {}", "S".repeat(105));
        for (j, slide) in scene.slides.iter_mut().enumerate() {
            slide.title = format!("Slide {j:02} {}", "L".repeat(105));
            for (k, layer) in slide.layers.iter_mut().enumerate() {
                layer.name = format!("Layer {k:02} {}", "Y".repeat(105));
            }
            for (k, obj) in slide.objects.iter_mut().enumerate() {
                obj.name = format!("Object {k:02} {}", "O".repeat(105));
            }
        }
    }
    for i in 0..32 {
        course.variables.push(VariableDef {
            name: format!("long_variable_name_{i:02}_{}", "V".repeat(95)),
            value: Value::Text(format!("Initial value {i:02} {}", "T".repeat(60))),
        });
    }
    course
}

struct SurfaceCase {
    label: String,
    select: SnapshotSelect,
    shows: Option<&'static str>,
    extra_setup: Option<fn(&mut crate::Editor)>,
}

fn base_select() -> SnapshotSelect {
    SnapshotSelect {
        slide: Some("s-layers".to_owned()),
        object: Some("obj-layers-title".to_owned()),
        ..Default::default()
    }
}

fn all_surfaces() -> Vec<SurfaceCase> {
    let mut cases = Vec::new();
    for (name, tab) in [
        ("inspector-format", InspectorTab::Format),
        ("inspector-animate", InspectorTab::Animate),
        ("inspector-slide", InspectorTab::Slide),
        ("inspector-course", InspectorTab::Course),
    ] {
        cases.push(SurfaceCase {
            label: name.to_owned(),
            select: SnapshotSelect {
                inspector_tab: Some(tab),
                ..base_select()
            },
            shows: None,
            extra_setup: None,
        });
    }
    for (name, tab) in [
        ("bottom-timeline", BottomTab::Timeline),
        ("bottom-triggers", BottomTab::Triggers),
        ("bottom-variables", BottomTab::Variables),
        ("bottom-question", BottomTab::Question),
    ] {
        let select = if tab == BottomTab::Question {
            SnapshotSelect {
                slide: Some("s-q-dd".to_owned()),
                object: None,
                bottom_tab: Some(tab),
                ..Default::default()
            }
        } else {
            SnapshotSelect {
                bottom_tab: Some(tab),
                ..base_select()
            }
        };
        cases.push(SurfaceCase {
            label: name.to_owned(),
            select,
            shows: None,
            extra_setup: None,
        });
    }
    cases.push(SurfaceCase {
        label: "bottom-states-empty".to_owned(),
        select: SnapshotSelect {
            slide: Some("s-layers".to_owned()),
            object: None,
            bottom_tab: Some(BottomTab::States),
            ..Default::default()
        },
        shows: None,
        extra_setup: Some(|e| e.selection.clear_objects()),
    });
    cases.push(SurfaceCase {
        label: "bottom-states-multi".to_owned(),
        select: SnapshotSelect {
            slide: Some("s-layers".to_owned()),
            bottom_tab: Some(BottomTab::States),
            ..Default::default()
        },
        shows: None,
        extra_setup: Some(|e| {
            let ids: Vec<String> = e
                .session
                .course
                .slide("s-layers")
                .map(|s| s.objects.iter().take(2).map(|o| o.id.clone()).collect())
                .unwrap_or_default();
            if ids.len() >= 2 {
                e.selection.set_many(ids);
            }
        }),
    });
    cases.push(SurfaceCase {
        label: "bottom-states-1".to_owned(),
        select: SnapshotSelect {
            bottom_tab: Some(BottomTab::States),
            ..base_select()
        },
        shows: None,
        extra_setup: Some(|e| {
            if e.session.course.slide("s-layers").is_some() {
                let _ = e.run(
                    "object.add_state",
                    serde_json::json!({
                        "slide": "s-layers",
                        "object": "obj-layers-title",
                        "state": { "name": "hover", "opacity": 0.8 }
                    }),
                );
            }
        }),
    });
    cases.push(SurfaceCase {
        label: "bottom-states-8".to_owned(),
        select: SnapshotSelect {
            bottom_tab: Some(BottomTab::States),
            ..base_select()
        },
        shows: None,
        extra_setup: Some(|e| {
            for name in [
                "hover", "down", "visited", "selected", "disabled", "custom_1", "custom_2",
                "custom_3",
            ] {
                let _ = e.run(
                    "object.add_state",
                    serde_json::json!({
                        "slide": "s-layers",
                        "object": "obj-layers-title",
                        "state": { "name": name, "opacity": 0.9 }
                    }),
                );
            }
        }),
    });
    cases.push(SurfaceCase {
        label: "bottom-states-long".to_owned(),
        select: SnapshotSelect {
            bottom_tab: Some(BottomTab::States),
            ..base_select()
        },
        shows: None,
        extra_setup: Some(|e| {
            let long_state = "state_with_very_long_name_for_ui_stress_testing_1234567890abcdef";
            let _ = e.run(
                "object.add_state",
                serde_json::json!({
                    "slide": "s-layers",
                    "object": "obj-layers-title",
                    "state": { "name": long_state, "opacity": 0.85 }
                }),
            );
        }),
    });
    cases.push(SurfaceCase {
        label: "edit-state-mode".to_owned(),
        select: SnapshotSelect {
            inspector_tab: Some(InspectorTab::Format),
            bottom_tab: Some(BottomTab::States),
            ..base_select()
        },
        shows: None,
        extra_setup: Some(|e| {
            if e.session.course.slide("s-layers").is_some() {
                let _ = e.run(
                    "object.add_state",
                    serde_json::json!({
                        "slide": "s-layers",
                        "object": "obj-layers-title",
                        "state": { "name": "hover", "opacity": 0.8 }
                    }),
                );
                e.enter_edit_state("s-layers", "obj-layers-title", "hover");
            }
        }),
    });
    cases.push(SurfaceCase {
        label: "edit-state-mode-extra".to_owned(),
        select: SnapshotSelect {
            inspector_tab: Some(InspectorTab::Format),
            bottom_tab: Some(BottomTab::States),
            ..base_select()
        },
        shows: None,
        extra_setup: Some(|e| {
            if e.session.course.slide("s-layers").is_some() {
                let _ = e.run(
                    "object.add_state",
                    serde_json::json!({
                        "slide": "s-layers",
                        "object": "obj-layers-title",
                        "state": { "name": "hover", "opacity": 0.8 }
                    }),
                );
                if let Some(res) = e.run(
                    "object.state_add_extra",
                    serde_json::json!({
                        "slide": "s-layers",
                        "object": "obj-layers-title",
                        "state": "hover",
                        "extra": {
                            "name": "Badge",
                            "kind": {
                                "type": "shape",
                                "shape": "rectangle",
                                "fill": "#22C55E"
                            },
                            "frame": { "x": 12.0, "y": -8.0, "w": 64.0, "h": 28.0 }
                        }
                    }),
                ) {
                    let extra_id = res["id"].as_str().map(str::to_owned);
                    e.enter_edit_state("s-layers", "obj-layers-title", "hover");
                    if let (Some(session), Some(id)) = (e.ui.editing_state.as_mut(), extra_id) {
                        session.selected_extra = Some(id);
                    }
                } else {
                    e.enter_edit_state("s-layers", "obj-layers-title", "hover");
                }
            }
        }),
    });
    for slide in [
        "s-q-mc",
        "s-q-mr",
        "s-q-tf",
        "s-q-fi",
        "s-q-seq",
        "s-q-mat",
        "s-q-hs",
        "s-q-dd",
        "s-results",
    ] {
        cases.push(SurfaceCase {
            label: format!("question-{slide}"),
            select: SnapshotSelect {
                slide: Some(slide.to_owned()),
                bottom_tab: Some(BottomTab::Question),
                ..Default::default()
            },
            shows: None,
            extra_setup: None,
        });
    }
    cases.push(SurfaceCase {
        label: "course-map".to_owned(),
        select: base_select(),
        shows: None,
        extra_setup: Some(|e| e.ui.story_view = true),
    });
    cases.push(SurfaceCase {
        label: "master-view".to_owned(),
        select: base_select(),
        shows: None,
        extra_setup: Some(|e| e.ui.master_view = true),
    });
    cases.push(SurfaceCase {
        label: "canvas-rulers".to_owned(),
        select: base_select(),
        shows: None,
        extra_setup: Some(|e| e.session.course.editor.show_rulers = true),
    });
    cases.push(SurfaceCase {
        label: "window-history".to_owned(),
        select: base_select(),
        shows: Some("History"),
        extra_setup: Some(|e| e.ui.show_history = true),
    });
    for spec in WINDOWS {
        cases.push(SurfaceCase {
            label: format!("window-{}", spec.name),
            select: SnapshotSelect {
                slide: Some("s-layers".to_owned()),
                window: Some(spec.name.to_owned()),
                ..Default::default()
            },
            shows: Some(spec.shows),
            extra_setup: None,
        });
    }
    cases
}

fn parse_rule_prefix(finding: &str) -> (&str, &str) {
    if let Some(rest) = finding.strip_prefix('[')
        && let Some((rule, msg)) = rest.split_once("] ")
    {
        return (rule, msg);
    }
    ("UX-04", finding)
}

fn run_sweep_case(
    course: Course,
    case: &SurfaceCase,
    size: (f32, f32),
    mode: ThemeMode,
    scale: f32,
    file_ops: bool,
) -> (Vec<String>, Option<String>) {
    let mut h = TestHarness::new_with_config(course, size, mode, scale, file_ops);
    case.select.apply(&mut h.editor);
    if let Some(setup) = case.extra_setup {
        setup(&mut h.editor);
    }
    let report = h.audit_surface_report(3, mode);
    let unopened = if scale <= 1.0 {
        case.shows.and_then(|needle| {
            if !report.painted(needle) {
                Some(format!(
                    "{} {}x{} {:?} scale={scale}: \"{needle}\" was not painted",
                    case.label, size.0, size.1, mode
                ))
            } else {
                None
            }
        })
    } else {
        None
    };
    let mut violations = Vec::new();
    for f in report.findings {
        let (rule, detail) = parse_rule_prefix(&f);
        if match_allowlist(rule, &case.label, detail).is_some() {
            continue;
        }
        violations.push(format!(
            "[{rule}] {} {}x{} {:?} scale={scale} file_ops={file_ops}: {detail}",
            case.label, size.0, size.1, mode
        ));
    }
    (violations, unopened)
}

#[test]
fn uiux_sweep_all_surfaces() {
    let base_course = showcase();
    let surfaces = all_surfaces();
    let mut violations = Vec::new();
    let mut unopened = Vec::new();
    let mut by_rule: BTreeMap<String, usize> = BTreeMap::new();
    let mut total_frames = 0usize;

    for case in &surfaces {
        for &size in &MATRIX_SIZES {
            for &(mode, _) in &MATRIX_THEMES {
                for &scale in &MATRIX_SCALES {
                    // 1024x600 at 1.5x is 682x400 logical, below the 1024x600 minimum window size;
                    // test 1536x900 at 1.5x instead (exact 1024x600 logical).
                    let effective_size = if scale > 1.0 && size == (1024.0, 600.0) {
                        (1536.0, 900.0)
                    } else {
                        size
                    };
                    total_frames += 1;
                    let (v, u) = run_sweep_case(
                        base_course.clone(),
                        case,
                        effective_size,
                        mode,
                        scale,
                        true,
                    );
                    for item in &v {
                        if let Some(rest) = item.strip_prefix('[')
                            && let Some((rule, _)) = rest.split_once(']')
                        {
                            *by_rule.entry(rule.to_owned()).or_default() += 1;
                        }
                    }
                    violations.extend(v);
                    if let Some(msg) = u {
                        unopened.push(msg);
                    }
                }
            }
        }
        // Web mode (`file_ops_available = false`) at standard laptop viewport in Dark & Light.
        for &(mode, _) in &MATRIX_THEMES {
            total_frames += 1;
            let (v, _) =
                run_sweep_case(base_course.clone(), case, (1280.0, 800.0), mode, 1.0, false);
            for item in &v {
                if let Some(rest) = item.strip_prefix('[')
                    && let Some((rule, _)) = rest.split_once(']')
                {
                    *by_rule.entry(rule.to_owned()).or_default() += 1;
                }
            }
            violations.extend(v);
        }
    }

    // Best-effort JSON summary under target/ for Step 18 report generation.
    let summary_json = format!(
        "{{\n  \"surfaces\": {},\n  \"frames_audited\": {},\n  \"unopened\": {},\n  \"violations\": {}\n}}\n",
        surfaces.len(),
        total_frames,
        unopened.len(),
        violations.len()
    );
    let _ = std::fs::write("../../target/uiux-sweep-report.json", summary_json);

    assert!(
        unopened.is_empty(),
        "unopened surfaces ({}):\n{}",
        unopened.len(),
        unopened.join("\n")
    );
    let interesting: Vec<String> = violations.iter().take(50).cloned().collect();
    assert!(
        violations.is_empty(),
        "UI/UX sweep found {} violations across {total_frames} audited frames (by rule: {by_rule:?}):\n{}",
        violations.len(),
        if interesting.is_empty() {
            violations
                .iter()
                .take(50)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            interesting.join("\n")
        }
    );
}

#[test]
fn uiux_sweep_scale_2_and_fixtures() {
    let base_course = showcase();
    let empty = empty_course();
    let long = long_names_course();
    let core_labels = [
        "inspector-format",
        "inspector-slide",
        "inspector-course",
        "bottom-timeline",
        "bottom-states-1",
        "edit-state-mode",
        "edit-state-mode-extra",
        "bottom-triggers",
        "bottom-variables",
        "course-map",
        "window-course-settings",
        "window-slide-size",
        "window-theme",
        "window-publish",
        "window-find",
        "window-preferences",
        "window-preview",
    ];
    let surfaces = all_surfaces();
    let mut violations = Vec::new();

    // Scale 2.0 stress pass at 2048x1200 (1024x600 logical) and 1920x1080 for core panels & dialogs.
    for case in surfaces
        .iter()
        .filter(|c| core_labels.contains(&c.label.as_str()))
    {
        for size in [(2048.0, 1200.0), (1920.0, 1080.0)] {
            for &(mode, _) in &MATRIX_THEMES {
                let (v, _) = run_sweep_case(base_course.clone(), case, size, mode, 2.0, true);
                violations.extend(v);
            }
        }
    }

    // Empty course & long-names stress course across core surfaces.
    for (fixture_name, course) in [("empty_course", &empty), ("long_names_course", &long)] {
        for case in surfaces
            .iter()
            .filter(|c| core_labels.contains(&c.label.as_str()))
        {
            for size in [(1024.0, 600.0), (1280.0, 800.0)] {
                for &(mode, _) in &MATRIX_THEMES {
                    let (v, _) = run_sweep_case(course.clone(), case, size, mode, 1.0, true);
                    for item in v {
                        let (rule, detail) = parse_rule_prefix(&item);
                        if match_allowlist(rule, fixture_name, detail).is_some() {
                            continue;
                        }
                        violations.push(format!("[{fixture_name}] {item}"));
                    }
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "scale-2.0 / fixture sweep found {} violations:\n{}",
        violations.len(),
        violations
            .iter()
            .take(50)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn uiux_sweep_modals_and_tooltips() {
    for &(mode, name) in &MATRIX_THEMES {
        let mut h_prev = TestHarness::new_with_theme(showcase(), (1280.0, 800.0), mode);
        SnapshotSelect {
            slide: Some("s-layers".to_owned()),
            window: Some("preview".to_owned()),
            ..Default::default()
        }
        .apply(&mut h_prev.editor);
        for _ in 0..3 {
            h_prev.idle_frame();
        }
        let modal_findings = h_prev.check_modals_and_backdrops(true);
        assert!(
            modal_findings.is_empty(),
            "preview modal findings ({name}): {modal_findings:?}"
        );

        let mut h_short = TestHarness::new_with_theme(showcase(), (1280.0, 800.0), mode);
        SnapshotSelect {
            slide: Some("s-layers".to_owned()),
            window: Some("shortcuts".to_owned()),
            ..Default::default()
        }
        .apply(&mut h_short.editor);
        for _ in 0..3 {
            h_short.idle_frame();
        }
        let short_findings = h_short.check_modals_and_backdrops(false);
        assert!(
            short_findings.is_empty(),
            "shortcuts modal findings ({name}): {short_findings:?}"
        );
    }

    // Probe tooltips and disabled explanations on core editor view.
    let mut h = TestHarness::new_with_theme(showcase(), (1280.0, 800.0), ThemeMode::Dark);
    base_select().apply(&mut h.editor);
    let _ = h.audit_surface_report(3, ThemeMode::Dark);
    let tooltip_findings: Vec<String> = h
        .check_tooltips_and_disabled()
        .into_iter()
        .filter(|f| match_allowlist("UX-10", "editor-default", f).is_none())
        .filter(|f| match_allowlist("UX-08", "editor-default", f).is_none())
        .collect();
    assert!(
        tooltip_findings.is_empty(),
        "tooltip/disabled findings ({}):\n{}",
        tooltip_findings.len(),
        tooltip_findings
            .iter()
            .take(30)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

fn m37_sweep_course() -> (Course, String, String, Vec<String>) {
    let mut course = showcase();
    let slide_id = course
        .scenes
        .first()
        .and_then(|sc| sc.slides.first())
        .map(|sl| sl.id.clone())
        .unwrap_or_else(|| panic!("showcase must have a first slide"));
    if let Some(slide) = course
        .scenes
        .first_mut()
        .and_then(|sc| sc.slides.first_mut())
    {
        slide.objects.clear();
        slide.triggers.clear();
        slide.layers.clear();
        slide.groups.clear();
    }

    let mut editor = crate::Editor::new(course);
    let point_text_id = editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": slide_id,
                "id": "m37-pt-label",
                "name": "Next label",
                "kind": {
                    "type": "text",
                    "text": "Next",
                    "point_text": true,
                    "style": { "wrap": false }
                },
                "frame": { "x": 120.0, "y": 80.0, "w": 64.0, "h": 30.0 }
            }),
        )
        .unwrap_or_else(|| panic!("add point text"))
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("point text id"))
        .to_owned();

    let bg_id = editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": slide_id,
                "id": "m37-gb-bg",
                "name": "Back background",
                "kind": {
                    "type": "shape",
                    "shape": "rounded_rectangle",
                    "fill": "#2563EB"
                },
                "frame": { "x": 120.0, "y": 180.0, "w": 180.0, "h": 48.0 }
            }),
        )
        .unwrap_or_else(|| panic!("add group bg"))
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("group bg id"))
        .to_owned();
    let icon_id = editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": slide_id,
                "id": "m37-gb-icon",
                "name": "Back arrow",
                "kind": {
                    "type": "shape",
                    "shape": "triangle",
                    "fill": "#FFFFFF"
                },
                "frame": { "x": 136.0, "y": 194.0, "w": 16.0, "h": 20.0 }
            }),
        )
        .unwrap_or_else(|| panic!("add group icon"))
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("group icon id"))
        .to_owned();
    let label_id = editor
        .run(
            "object.add",
            serde_json::json!({
                "slide": slide_id,
                "id": "m37-gb-label",
                "name": "Back text",
                "kind": {
                    "type": "text",
                    "text": "Back",
                    "point_text": true,
                    "style": { "color": "#FFFFFF", "wrap": false }
                },
                "frame": { "x": 164.0, "y": 190.0, "w": 64.0, "h": 28.0 }
            }),
        )
        .unwrap_or_else(|| panic!("add group label"))
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("group label id"))
        .to_owned();

    assert!(
        editor
            .run(
                "object.add_state",
                serde_json::json!({
                    "slide": slide_id,
                    "object": bg_id,
                    "state": { "name": "hover", "opacity": 0.9 }
                }),
            )
            .is_some(),
        "add hover state on group bg"
    );
    assert!(
        editor
            .run(
                "object.add_state",
                serde_json::json!({
                    "slide": slide_id,
                    "object": bg_id,
                    "state": { "name": "selected", "opacity": 0.8 }
                }),
            )
            .is_some(),
        "add selected state on group bg"
    );

    let member_ids = vec![bg_id, icon_id, label_id];
    let group_id = editor
        .run(
            "object.group",
            serde_json::json!({
                "slide": slide_id,
                "objects": member_ids,
            }),
        )
        .unwrap_or_else(|| panic!("group 3 members"))
        .get("group")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("group id"))
        .to_owned();

    assert!(
        editor
            .run(
                "group.set_interactive",
                serde_json::json!({
                    "slide": slide_id,
                    "group": group_id,
                    "interactive": true,
                    "name": "Back button",
                }),
            )
            .is_some(),
        "make group interactive"
    );
    assert!(
        editor
            .run(
                "group.update",
                serde_json::json!({
                    "slide": slide_id,
                    "group": group_id,
                    "name": "Back button",
                    "description": "Return to previous slide",
                }),
            )
            .is_some(),
        "update interactive group metadata"
    );
    let group_trigger = coursecraft_course::Trigger {
        when: coursecraft_course::Event::Click {
            object: group_id.clone(),
        },
        action: coursecraft_course::Action::JumpToSlide {
            target: coursecraft_course::SlideTarget::Previous,
        },
        condition: None,
        disabled: false,
    };
    assert!(
        editor
            .run(
                "trigger.add",
                serde_json::json!({
                    "slide": slide_id,
                    "trigger": group_trigger,
                }),
            )
            .is_some(),
        "add click trigger on interactive group"
    );

    let problems = editor.session.course.validate();
    assert!(
        problems.is_empty(),
        "m37 sweep fixture must validate cleanly: {problems:?}"
    );

    (editor.session.course, slide_id, point_text_id, member_ids)
}

struct M37SweepFixture {
    course: Course,
    slide_id: String,
    point_text_id: String,
    member_ids: Vec<String>,
}

struct M37SweepCase {
    label: &'static str,
    bottom_tab: BottomTab,
    select_group: bool,
    scroll_target: Option<&'static str>,
    required_needles: &'static [&'static str],
}

#[derive(Clone, Copy)]
struct M37Viewport {
    size: (f32, f32),
    mode: ThemeMode,
    scale: f32,
    file_ops: bool,
}

fn setup_m37_harness(
    fx: &M37SweepFixture,
    bottom_tab: BottomTab,
    select_group: bool,
    scroll_target: Option<&str>,
    vp: M37Viewport,
) -> TestHarness {
    let mut h =
        TestHarness::new_with_config(fx.course.clone(), vp.size, vp.mode, vp.scale, vp.file_ops);
    SnapshotSelect {
        slide: Some(fx.slide_id.clone()),
        object: (!select_group).then(|| fx.point_text_id.clone()),
        inspector_tab: Some(InspectorTab::Format),
        bottom_tab: Some(bottom_tab),
        ..Default::default()
    }
    .apply(&mut h.editor);
    if select_group {
        h.editor.selection.set_many(fx.member_ids.clone());
    }
    if let Some(target_text) = scroll_target {
        h.idle_frame();
        h.idle_frame();
        let right = h.right_panel_rect();
        let inspector_pos = right.center();
        for _ in 0..8 {
            if h.find_text_in(target_text, right).is_some() {
                break;
            }
            h.scroll_over(inspector_pos, -80.0);
        }
    }
    h
}

fn check_m37_trailing_colons(texts: &[String], ctx: &str, violations: &mut Vec<String>) {
    for t in texts {
        let trimmed = t.trim();
        if matches!(
            trimmed,
            "Wrap:"
                | "Wrap text:"
                | "Act as one button:"
                | "Name:"
                | "Initial state:"
                | "Accessible description:"
                | "Focus order:"
        ) {
            violations.push(format!(
                "[UX-11] {ctx}: trailing colon on label \"{trimmed}\""
            ));
        }
    }
}

fn audit_m37_case_frame(
    fx: &M37SweepFixture,
    case: &M37SweepCase,
    vp: M37Viewport,
    violations: &mut Vec<String>,
    missing: &mut Vec<String>,
) {
    let mut h = setup_m37_harness(
        fx,
        case.bottom_tab,
        case.select_group,
        case.scroll_target,
        vp,
    );
    let report = h.audit_surface_report(3, vp.mode);
    let ctx = format!(
        "{} {}x{} {:?} scale={} file_ops={}",
        case.label, vp.size.0, vp.size.1, vp.mode, vp.scale, vp.file_ops
    );
    if vp.scale <= 1.0 && vp.file_ops {
        for &needle in case.required_needles {
            if !report.painted(needle) {
                missing.push(format!("{ctx}: \"{needle}\" was not painted"));
            }
        }
    }
    check_m37_trailing_colons(&report.texts, &ctx, violations);
    for f in report.findings {
        let (rule, detail) = parse_rule_prefix(&f);
        if match_allowlist(rule, case.label, detail).is_some() {
            continue;
        }
        violations.push(format!("[{rule}] {ctx}: {detail}"));
    }
}

fn audit_m37_tooltips_and_popups(
    fx: &M37SweepFixture,
    violations: &mut Vec<String>,
    missing: &mut Vec<String>,
) {
    for (label, select_group) in [
        ("m37-point-text-tooltips", false),
        ("m37-interactive-group-tooltips", true),
    ] {
        for &(mode, _) in &MATRIX_THEMES {
            let vp = M37Viewport {
                size: (1280.0, 800.0),
                mode,
                scale: 1.0,
                file_ops: true,
            };
            let mut h = setup_m37_harness(fx, BottomTab::Timeline, select_group, None, vp);
            let _ = h.audit_surface_report(3, mode);
            for f in h.check_tooltips_and_disabled() {
                violations.push(format!("[UX-08/UX-10] {label} {mode:?}: {f}"));
            }
        }
    }

    for &(mode, _) in &MATRIX_THEMES {
        let vp = M37Viewport {
            size: (1280.0, 800.0),
            mode,
            scale: 1.0,
            file_ops: true,
        };
        let mut h = setup_m37_harness(fx, BottomTab::Timeline, true, None, vp);
        h.idle_frame();
        h.idle_frame();
        for f in h.audit_combo("Normal") {
            violations.push(format!(
                "[UX-03/UX-13] m37-group-initial-state-combo {mode:?}: {f}"
            ));
        }

        if let Some(pos) = h.find_text("Add trigger…") {
            h.click_at(pos);
            let report = h.audit_surface_report(3, mode);
            if !report.painted("Back button (button group)") {
                missing.push(format!(
                    "m37-group-trigger-editor {mode:?}: \"Back button (button group)\" was not painted"
                ));
            }
            for f in report.findings {
                let (rule, detail) = parse_rule_prefix(&f);
                violations.push(format!(
                    "[{rule}] m37-group-trigger-editor 1280x800 {mode:?}: {detail}"
                ));
            }
            h.press(egui::Key::Escape, egui::Modifiers::NONE);
            h.idle_frame();
        } else {
            missing.push(format!(
                "m37-group-trigger-editor {mode:?}: \"Add trigger…\" button not found"
            ));
        }

        if let Some(arrange_pos) = h.find_text("Arrange") {
            h.click_at(arrange_pos);
            let report = h.audit_surface_report(2, mode);
            if !report.painted("Remove button behaviour") {
                missing.push(format!(
                    "m37-group-arrange-menu {mode:?}: \"Remove button behaviour\" was not painted"
                ));
            }
            for f in report.findings {
                let (rule, detail) = parse_rule_prefix(&f);
                violations.push(format!(
                    "[{rule}] m37-group-arrange-menu 1280x800 {mode:?}: {detail}"
                ));
            }
            h.press(egui::Key::Escape, egui::Modifiers::NONE);
            h.idle_frame();
        } else {
            missing.push(format!(
                "m37-group-arrange-menu {mode:?}: \"Arrange\" menu not found"
            ));
        }
    }
}

/// GB-14: M28 automated UI/UX sweep for point text (`Wrap text` row) and interactive
/// button groups (Group Inspector, Timeline folder row + `Button` badge, and Triggers tab)
/// across all matrix sizes, themes, scales, and web mode with 0 MUST violations.
#[test]
fn gb_14_m37_point_text_and_interactive_group_uiux_sweep() {
    let (course, slide_id, point_text_id, member_ids) = m37_sweep_course();
    let fx = M37SweepFixture {
        course,
        slide_id,
        point_text_id,
        member_ids,
    };
    let mut violations = Vec::new();
    let mut missing = Vec::new();
    let mut total_frames = 0usize;

    let cases = [
        M37SweepCase {
            label: "m37-point-text-format",
            bottom_tab: BottomTab::Timeline,
            select_group: false,
            scroll_target: None,
            required_needles: &[],
        },
        M37SweepCase {
            label: "m37-point-text-format-scrolled",
            bottom_tab: BottomTab::Timeline,
            select_group: false,
            scroll_target: Some("Wrap text"),
            required_needles: &["Wrap text"],
        },
        M37SweepCase {
            label: "m37-interactive-group-timeline",
            bottom_tab: BottomTab::Timeline,
            select_group: true,
            scroll_target: None,
            required_needles: &[
                "Act as one button",
                "Back button",
                "Button",
                "Initial state",
            ],
        },
        M37SweepCase {
            label: "m37-interactive-group-scrolled",
            bottom_tab: BottomTab::Timeline,
            select_group: true,
            scroll_target: Some("Add trigger…"),
            required_needles: &["Focus order", "Add trigger…"],
        },
        M37SweepCase {
            label: "m37-interactive-group-triggers",
            bottom_tab: BottomTab::Triggers,
            select_group: true,
            scroll_target: None,
            required_needles: &["Act as one button", "Back button (button group)"],
        },
    ];

    for case in &cases {
        for &size in &MATRIX_SIZES {
            for &(mode, _) in &MATRIX_THEMES {
                for &scale in &MATRIX_SCALES {
                    let effective_size = if scale > 1.0 && size == (1024.0, 600.0) {
                        (1536.0, 900.0)
                    } else {
                        size
                    };
                    total_frames += 1;
                    audit_m37_case_frame(
                        &fx,
                        case,
                        M37Viewport {
                            size: effective_size,
                            mode,
                            scale,
                            file_ops: true,
                        },
                        &mut violations,
                        &mut missing,
                    );
                }
            }
        }
        for &(mode, _) in &MATRIX_THEMES {
            total_frames += 1;
            audit_m37_case_frame(
                &fx,
                case,
                M37Viewport {
                    size: (1280.0, 800.0),
                    mode,
                    scale: 1.0,
                    file_ops: false,
                },
                &mut violations,
                &mut missing,
            );
        }
    }

    // Scale 2.0 stress pass for point text and interactive group.
    for case in [&cases[0], &cases[2]] {
        for size in [(2048.0, 1200.0), (1920.0, 1080.0)] {
            for &(mode, _) in &MATRIX_THEMES {
                total_frames += 1;
                audit_m37_case_frame(
                    &fx,
                    case,
                    M37Viewport {
                        size,
                        mode,
                        scale: 2.0,
                        file_ops: true,
                    },
                    &mut violations,
                    &mut missing,
                );
            }
        }
    }

    audit_m37_tooltips_and_popups(&fx, &mut violations, &mut missing);

    assert!(
        missing.is_empty(),
        "M37 UI/UX sweep missing expected painted controls ({}):\n{}",
        missing.len(),
        missing.join("\n")
    );
    assert!(
        violations.is_empty(),
        "M37 UI/UX sweep found {} violations across {total_frames} audited frames:\n{}",
        violations.len(),
        violations
            .iter()
            .take(50)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
