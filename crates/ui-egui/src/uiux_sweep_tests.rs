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
        "bottom-timeline",
        "bottom-triggers",
        "bottom-variables",
        "course-map",
        "window-course-settings",
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
