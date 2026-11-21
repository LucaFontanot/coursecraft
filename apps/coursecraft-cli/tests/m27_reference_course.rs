//! M27 Part B: Automated multi-standard runs on `examples/reference-course/`.
//!
//! Step B-3: Shared runner harness + SCORM 1.2 (B1.1–B1.8) + SCORM 2004 (B2.1–B2.8).
//! Step B-4: cmi5 & xAPI 1.0.3 (B3.1–B3.10).

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use coursecraft_course::{CompletionStatus, Course, Severity, Value};
use coursecraft_lms::{
    CmiApi, HANDSHAKE_TIMEOUT_MS, LmsBridge, LrsApi, LrsMethod, LrsRequest, LrsResponse, StartPlan,
    Tick, XapiBridge,
};
use coursecraft_logic::{Effect, Input, Player};
use coursecraft_publish::{
    PackageFile, PackageInput, PackageStandard, build_package_standard_with_folders,
};
use coursecraft_scorm::Standard;
use coursecraft_strict_lms::{
    Flavor as StrictFlavor, LaunchSpec, PackageStandardKind, Scorm12Config, Scorm2004Config,
    Scorm2004Edition, StrictLrs, StrictScorm12, StrictScorm2004, build_cmi5_launch_data,
    validate_package,
};
use coursecraft_xapi::{Flavor, LaunchParams};
use serde_json::{Value as JsonValue, json};

// ---------------------------------------------------------------------------
// Shared adapters & reference-course loading helpers
// ---------------------------------------------------------------------------

struct Strict12Adapter {
    inner: StrictScorm12,
}

impl Strict12Adapter {
    fn new(inner: StrictScorm12) -> Self {
        Self { inner }
    }
}

impl CmiApi for Strict12Adapter {
    fn initialize(&mut self) -> bool {
        self.inner.lms_initialize("") == "true"
    }

    fn get_value(&mut self, key: &str) -> String {
        self.inner.lms_get_value(key)
    }

    fn set_value(&mut self, key: &str, value: &str) -> bool {
        self.inner.lms_set_value(key, value) == "true"
    }

    fn commit(&mut self) -> bool {
        self.inner.lms_commit("") == "true"
    }

    fn finish(&mut self) -> bool {
        self.inner.lms_finish("") == "true"
    }

    fn last_error(&mut self) -> String {
        let code = self.inner.lms_get_last_error();
        let text = self.inner.lms_get_error_string(code);
        format!("{code}: {text}")
    }
}

struct Strict2004Adapter {
    inner: StrictScorm2004,
    corrupt_interactions_count: bool,
}

impl Strict2004Adapter {
    fn new(inner: StrictScorm2004) -> Self {
        Self {
            inner,
            corrupt_interactions_count: false,
        }
    }
}

impl CmiApi for Strict2004Adapter {
    fn initialize(&mut self) -> bool {
        self.inner.initialize("") == "true"
    }

    fn get_value(&mut self, key: &str) -> String {
        if self.corrupt_interactions_count && key == "cmi.interactions._count" {
            return "nan".to_owned();
        }
        self.inner.get_value(key)
    }

    fn set_value(&mut self, key: &str, value: &str) -> bool {
        self.inner.set_value(key, value) == "true"
    }

    fn commit(&mut self) -> bool {
        self.inner.commit("") == "true"
    }

    fn finish(&mut self) -> bool {
        self.inner.terminate("") == "true"
    }

    fn last_error(&mut self) -> String {
        let code = self.inner.get_last_error();
        let text = self.inner.get_error_string(code);
        format!("{code}: {text}")
    }
}

fn ref_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/reference-course")
}

fn load_course(filename: &str) -> (Course, String) {
    let path = ref_dir().join(filename);
    let json =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let course =
        Course::from_json(&json).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
    let issues = course.validate();
    let errors: Vec<_> = issues
        .iter()
        .filter(|i| i.severity == Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "{filename} validation errors: {errors:?}"
    );
    (course, json)
}

fn load_inputs(filename: &str) -> Vec<Input> {
    let path = ref_dir().join(filename);
    let json =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&json).unwrap_or_else(|e| panic!("parse inputs {}: {e}", path.display()))
}

fn collect_folder_recursive(
    base: &Path,
    dir: &Path,
    out: &mut Vec<(String, Vec<u8>)>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        if path.is_dir() {
            collect_folder_recursive(base, &path, out)?;
        } else {
            let rel = path
                .strip_prefix(base)
                .map_err(|e| std::io::Error::other(e.to_string()))?
                .to_string_lossy()
                .replace('\\', "/");
            let bytes = std::fs::read(&path)?;
            out.push((rel, bytes));
        }
    }
    Ok(())
}

fn package_and_validate(
    course: &Course,
    course_json: &str,
    standard: PackageStandard,
    kind: PackageStandardKind,
) -> Vec<PackageFile> {
    let player_files = vec![PackageFile {
        path: "index.html".to_owned(),
        bytes: b"<!doctype html><html><body>player</body></html>".to_vec(),
    }];
    let media_root = ref_dir().join("media");
    let media = |src: &str| -> Option<Vec<u8>> {
        let rel = src.strip_prefix("media/").unwrap_or(src);
        std::fs::read(media_root.join(rel)).ok()
    };
    let web_folder = |folder: &str| -> Option<Vec<(String, Vec<u8>)>> {
        let rel = folder.strip_prefix("media/").unwrap_or(folder);
        let dir = media_root.join(rel);
        if !dir.is_dir() {
            return None;
        }
        let mut out = Vec::new();
        collect_folder_recursive(&media_root, &dir, &mut out).ok()?;
        Some(out)
    };
    let input = PackageInput {
        course,
        course_json,
        player_files: &player_files,
        media: &media,
    };
    let files = build_package_standard_with_folders(&input, standard, &web_folder)
        .unwrap_or_else(|e| panic!("package {kind:?}: {e}"));
    let refs: Vec<(&str, &[u8])> = files
        .iter()
        .map(|f| (f.path.as_str(), f.bytes.as_slice()))
        .collect();
    let violations = validate_package(kind, &refs);
    assert!(
        violations.is_empty(),
        "package {kind:?} manifest violations: {violations:?}"
    );
    files
}

struct RunResult {
    effects: Vec<Effect>,
    runtime_warnings: Vec<String>,
    bridge_warnings: Vec<String>,
    session_ms: u64,
}

fn drive_scorm_inputs<A: CmiApi>(
    player: &mut Player<'_>,
    bridge: &mut LmsBridge,
    api: &mut A,
    course: &Course,
    inputs: &[Input],
    start_ms: u64,
) -> RunResult {
    let mut effects = Vec::new();
    let mut runtime_warnings = Vec::new();
    let mut bridge_warnings = Vec::new();
    let mut session_ms = start_ms;

    for input in inputs {
        if let Input::Tick { ms } = input {
            session_ms = session_ms.saturating_add(u64::from(*ms));
        } else {
            session_ms = session_ms.saturating_add(250);
        }
        let step_effects = player.handle(input);
        for eff in &step_effects {
            if let Effect::Warning { message } = eff {
                runtime_warnings.push(message.clone());
            }
        }
        let bw = bridge.after_input(api, course, player.state(), &step_effects, session_ms);
        bridge_warnings.extend(bw);
        effects.extend(step_effects);
    }

    RunResult {
        effects,
        runtime_warnings,
        bridge_warnings,
        session_ms,
    }
}

fn start_scorm_session<'a, A: CmiApi>(
    course: &'a Course,
    standard: Standard,
    api: &mut A,
    seed: u32,
) -> (Player<'a>, LmsBridge, StartPlan, Vec<String>) {
    let mut bridge = LmsBridge::for_standard(standard);
    let (plan, warnings) = bridge.start(api, course);
    let player = match plan.resume.clone() {
        Some(state) => Player::resume(course, state).expect("resume player"),
        None => Player::with_draw_seed(course, seed).expect("fresh player"),
    };
    (player, bridge, plan, warnings)
}

fn is_plain_decimal(s: &str) -> bool {
    !s.is_empty()
        && !s.contains('e')
        && !s.contains('E')
        && s.parse::<f64>().is_ok_and(f64::is_finite)
}

fn incompressible_ascii(len: usize) -> String {
    let mut state: u32 = 0x1234_5678;
    let mut out = String::with_capacity(len);
    for _ in 0..len {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let byte = b'!' + ((state >> 24) % 90) as u8;
        out.push(char::from(byte));
    }
    out
}

fn assert_resumed_state_matches(
    pre_state: &coursecraft_logic::PlayerState,
    post_state: &coursecraft_logic::PlayerState,
) {
    assert_eq!(post_state.slide, pre_state.slide);
    assert_eq!(post_state.visited, pre_state.visited);
    assert_eq!(post_state.variables, pre_state.variables);
    assert_eq!(post_state.object_states, pre_state.object_states);
    assert_eq!(post_state.draw_seed, pre_state.draw_seed);
    assert!(
        post_state.visible_layers.contains(&"correct".to_owned()),
        "locked s11 outcome feedback layer should be re-shown: {:?}",
        post_state.visible_layers
    );
}

fn verify_scorm2004_interactions_readable(strict: &mut StrictScorm2004) {
    let mut saw_other = false;
    let mut saw_case_prefix = false;
    let mut saw_pair = false;
    let mut saw_multi_id = false;
    for i in 0..15 {
        let id = strict.get_value(&format!("cmi.interactions.{i}.id"));
        let kind = strict.get_value(&format!("cmi.interactions.{i}.type"));
        let desc = strict.get_value(&format!("cmi.interactions.{i}.description"));
        let resp = strict.get_value(&format!("cmi.interactions.{i}.learner_response"));
        let pat = strict.get_value(&format!("cmi.interactions.{i}.correct_responses.0.pattern"));
        assert!(!id.is_empty());
        assert!(!desc.is_empty(), "interaction {i} missing description");
        if kind == "other" {
            saw_other = true;
        }
        if pat.starts_with("{case_matters=true}") {
            saw_case_prefix = true;
        }
        if resp.contains("[.]") {
            saw_pair = true;
        }
        if resp.contains("[,]") {
            saw_multi_id = true;
        }
    }
    assert!(saw_other, "expected Freeform interaction mapped to 'other'");
    assert!(saw_case_prefix, "expected case_matters=true on FillIn");
    assert!(saw_pair, "expected [.] separator on Matching/DragDrop");
    assert!(
        saw_multi_id,
        "expected [,] separator on MultipleResponse/Sequence"
    );
}

// ---------------------------------------------------------------------------
// B1. SCORM 1.2 Automated Runs
// ---------------------------------------------------------------------------

#[test]
fn b1_1_scorm12_full_pass() {
    let (course, course_json) = load_course("course.json");
    let files = package_and_validate(
        &course,
        &course_json,
        PackageStandard::Scorm(Standard::Scorm12),
        PackageStandardKind::Scorm12,
    );
    // Templates must be stripped from packaged course.json.
    let pkg_course_bytes = files
        .iter()
        .find(|f| f.path == "course.json")
        .expect("packaged course.json")
        .bytes
        .as_slice();
    let pkg_course: Course =
        serde_json::from_slice(pkg_course_bytes).expect("parse packaged course.json");
    assert!(
        pkg_course.templates.is_empty(),
        "templates must be stripped"
    );

    let inputs = load_inputs("pass.inputs.json");
    let mut api = Strict12Adapter::new(StrictScorm12::new());
    let (mut player, mut bridge, plan, start_w) =
        start_scorm_session(&course, Standard::Scorm12, &mut api, 42);
    assert!(start_w.is_empty(), "start warnings: {start_w:?}");
    assert!(plan.resume.is_none());

    let res = drive_scorm_inputs(&mut player, &mut bridge, &mut api, &course, &inputs, 0);
    assert!(
        res.runtime_warnings.is_empty(),
        "runtime warnings: {:?}",
        res.runtime_warnings
    );
    assert!(
        res.bridge_warnings.is_empty(),
        "bridge warnings: {:?}",
        res.bridge_warnings
    );
    assert!(
        res.effects.iter().any(|e| matches!(
            e,
            Effect::Print { slide, exclude } if slide == "s20-cert" && exclude == &vec!["btn-print".to_owned()]
        )),
        "expected Print effect for s20-cert"
    );
    assert!(
        api.inner.assert_clean_finish(),
        "LMSFinish should have been called on ExitCourse"
    );
    let snap = api.inner.snapshot();
    assert_eq!(snap["lesson_status"], "passed");
    assert_eq!(snap["score"]["raw"], "100");
    assert_eq!(snap["score"]["min"], "0");
    assert_eq!(snap["score"]["max"], "100");
    assert_eq!(snap["interactions"].as_array().map(Vec::len), Some(15));
    assert_eq!(snap["exit"], "");
    assert!(
        api.inner
            .calls()
            .iter()
            .filter(|c| c.method == "LMSFinish")
            .count()
            == 1
    );
    assert!(
        api.inner.calls().iter().any(|c| c.method == "LMSSetValue"
            && c.args.first().map(String::as_str) == Some("cmi.core.session_time")),
        "expected cmi.core.session_time write"
    );
    assert!(
        api.inner.violations().is_empty(),
        "{:?}",
        api.inner.violations()
    );
}

#[test]
fn b1_2_scorm12_full_fail() {
    let (course, _) = load_course("course.json");
    let inputs = load_inputs("fail.inputs.json");
    let mut api = Strict12Adapter::new(StrictScorm12::new());
    let (mut player, mut bridge, _, start_w) =
        start_scorm_session(&course, Standard::Scorm12, &mut api, 42);
    assert!(start_w.is_empty(), "{start_w:?}");

    let res = drive_scorm_inputs(&mut player, &mut bridge, &mut api, &course, &inputs, 0);
    assert!(
        res.runtime_warnings.is_empty(),
        "{:?}",
        res.runtime_warnings
    );
    assert!(res.bridge_warnings.is_empty(), "{:?}", res.bridge_warnings);
    assert!(api.inner.assert_clean_finish());
    let snap = api.inner.snapshot();
    assert_eq!(snap["lesson_status"], "failed");
    let raw: f64 = snap["score"]["raw"]
        .as_str()
        .expect("score.raw str")
        .parse()
        .expect("numeric score.raw");
    assert!(raw < 70.0, "expected failing score < 70, got {raw}");
    assert_eq!(snap["exit"], "");
    let interactions = snap["interactions"].as_array().expect("interactions array");
    assert!(!interactions.is_empty());
    // Graded wrong answers use "wrong" in SCORM 1.2, surveys use "neutral".
    let survey_count = interactions
        .iter()
        .filter(|i| i["result"] == "neutral")
        .count();
    let wrong_count = interactions
        .iter()
        .filter(|i| i["result"] == "wrong")
        .count();
    assert_eq!(
        survey_count, 3,
        "expected 3 neutral survey results: {interactions:?}"
    );
    assert!(
        wrong_count >= 12,
        "expected wrong graded results: {interactions:?}"
    );
    assert!(
        api.inner.violations().is_empty(),
        "{:?}",
        api.inner.violations()
    );
}

#[test]
fn b1_3_scorm12_partial_suspend_relaunch_resume_pass() {
    let (course, _) = load_course("course.json");
    let part1 = load_inputs("partial-1.inputs.json");
    let part2 = load_inputs("partial-2-resume.inputs.json");

    let mut api = Strict12Adapter::new(StrictScorm12::new());
    let (mut player1, mut bridge1, _, start_w) =
        start_scorm_session(&course, Standard::Scorm12, &mut api, 77);
    assert!(start_w.is_empty(), "{start_w:?}");

    let res1 = drive_scorm_inputs(&mut player1, &mut bridge1, &mut api, &course, &part1, 0);
    assert!(
        res1.runtime_warnings.is_empty(),
        "{:?}",
        res1.runtime_warnings
    );
    assert!(
        res1.bridge_warnings.is_empty(),
        "{:?}",
        res1.bridge_warnings
    );

    // Close session (pagehide) without ExitCourse -> suspend.
    let fin_w = bridge1.finish_session(&mut api, res1.session_ms);
    assert!(fin_w.is_empty(), "{fin_w:?}");
    assert!(api.inner.assert_clean_finish());
    let snap1 = api.inner.snapshot();
    assert_eq!(snap1["lesson_status"], "incomplete");
    assert_eq!(snap1["exit"], "suspend");
    let suspend_len = snap1["suspend_data"].as_str().map_or(0, str::len);
    assert!(
        suspend_len > 0 && suspend_len <= 4096,
        "suspend_len={suspend_len}"
    );
    assert_eq!(snap1["interactions"].as_array().map(Vec::len), Some(5));

    let pre_state = player1.state().clone();
    assert_eq!(
        player1.current_slide().map(|s| s.id.as_str()),
        Some("s11-q-seq")
    );

    // Relaunch into StrictScorm12.
    api.inner.relaunch();
    let (mut player2, mut bridge2, plan2, start_w2) =
        start_scorm_session(&course, Standard::Scorm12, &mut api, 999);
    assert!(start_w2.is_empty(), "{start_w2:?}");
    assert_eq!(api.inner.lms_get_value("cmi.core.entry"), "resume");
    assert!(plan2.resume.is_some(), "expected resume plan");
    let start_fx = player2.handle(&Input::Start);
    let _ = bridge2.after_input(&mut api, &course, player2.state(), &start_fx, 0);

    assert_resumed_state_matches(&pre_state, player2.state());

    let res2 = drive_scorm_inputs(&mut player2, &mut bridge2, &mut api, &course, &part2, 0);
    assert!(
        res2.runtime_warnings.is_empty(),
        "{:?}",
        res2.runtime_warnings
    );
    assert!(
        res2.bridge_warnings.is_empty(),
        "{:?}",
        res2.bridge_warnings
    );
    assert!(api.inner.assert_clean_finish());
    let snap2 = api.inner.snapshot();
    assert_eq!(snap2["interactions"].as_array().map(Vec::len), Some(15));
    assert_eq!(snap2["lesson_status"], "passed");
    assert!(
        api.inner.violations().is_empty(),
        "{:?}",
        api.inner.violations()
    );
}

#[test]
fn b1_4_scorm12_fail_retry_bank_redraw_pass_review_exit() {
    let (course, _) = load_course("course.json");
    let inputs = load_inputs("retry-review.inputs.json");
    let mut api = Strict12Adapter::new(StrictScorm12::new());
    let (mut player, mut bridge, _, start_w) =
        start_scorm_session(&course, Standard::Scorm12, &mut api, 42);
    assert!(start_w.is_empty(), "{start_w:?}");

    let res = drive_scorm_inputs(&mut player, &mut bridge, &mut api, &course, &inputs, 0);
    assert!(
        res.runtime_warnings.is_empty(),
        "{:?}",
        res.runtime_warnings
    );
    assert!(res.bridge_warnings.is_empty(), "{:?}", res.bridge_warnings);
    assert!(api.inner.assert_clean_finish());

    assert!(player.state().quiz_attempts >= 2);
    let snap = api.inner.snapshot();
    assert_eq!(snap["lesson_status"], "passed");
    assert_eq!(snap["score"]["raw"], "100");
    let interactions = snap["interactions"].as_array().expect("interactions");
    assert!(
        interactions
            .iter()
            .any(|i| i["id"].as_str().is_some_and(|s| s.ends_with("_a1"))),
        "expected _a1 interaction IDs"
    );
    assert!(
        interactions
            .iter()
            .any(|i| i["id"].as_str().is_some_and(|s| s.ends_with("_a2"))),
        "expected _a2 interaction IDs"
    );
    assert!(
        api.inner.violations().is_empty(),
        "{:?}",
        api.inner.violations()
    );
}

#[test]
fn b1_5_scorm12_high_water_pass_protection() {
    let (course, _) = load_course("course.json");
    let mut api = Strict12Adapter::new(StrictScorm12::new());
    let (player, mut bridge, _, start_w) =
        start_scorm_session(&course, Standard::Scorm12, &mut api, 42);
    assert!(start_w.is_empty(), "{start_w:?}");
    let state = player.state();

    // 90% Pass
    let w = bridge.after_input(
        &mut api,
        &course,
        state,
        &[Effect::Score {
            raw: 90,
            max: 100,
            percent: 90,
            status: CompletionStatus::Passed,
        }],
        100,
    );
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(api.inner.lms_get_value("cmi.core.lesson_status"), "passed");
    assert_eq!(api.inner.lms_get_value("cmi.core.score.raw"), "90");

    // Retry -> 20% Fail (must be ignored)
    let w = bridge.after_input(
        &mut api,
        &course,
        state,
        &[Effect::Score {
            raw: 20,
            max: 100,
            percent: 20,
            status: CompletionStatus::Failed,
        }],
        200,
    );
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(api.inner.lms_get_value("cmi.core.lesson_status"), "passed");
    assert_eq!(api.inner.lms_get_value("cmi.core.score.raw"), "90");

    // Retry -> 75% Pass (lower than 90%, must be ignored)
    let w = bridge.after_input(
        &mut api,
        &course,
        state,
        &[Effect::Score {
            raw: 75,
            max: 100,
            percent: 75,
            status: CompletionStatus::Passed,
        }],
        300,
    );
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(api.inner.lms_get_value("cmi.core.lesson_status"), "passed");
    assert_eq!(api.inner.lms_get_value("cmi.core.score.raw"), "90");

    // Retry -> 100% Pass (higher than 90%, must update)
    let w = bridge.after_input(
        &mut api,
        &course,
        state,
        &[Effect::Score {
            raw: 100,
            max: 100,
            percent: 100,
            status: CompletionStatus::Passed,
        }],
        400,
    );
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(api.inner.lms_get_value("cmi.core.lesson_status"), "passed");
    assert_eq!(api.inner.lms_get_value("cmi.core.score.raw"), "100");
    assert!(
        api.inner.violations().is_empty(),
        "{:?}",
        api.inner.violations()
    );
}

#[test]
fn b1_6_scorm12_completion_only() {
    let (course, course_json) = load_course("completion-only.json");
    let _ = package_and_validate(
        &course,
        &course_json,
        PackageStandard::Scorm(Standard::Scorm12),
        PackageStandardKind::Scorm12,
    );
    let inputs = load_inputs("completion-only.inputs.json");
    let mut api = Strict12Adapter::new(StrictScorm12::new());
    let (mut player, mut bridge, _, start_w) =
        start_scorm_session(&course, Standard::Scorm12, &mut api, 1);
    assert!(start_w.is_empty(), "{start_w:?}");

    let res = drive_scorm_inputs(&mut player, &mut bridge, &mut api, &course, &inputs, 0);
    assert!(
        res.runtime_warnings.is_empty(),
        "{:?}",
        res.runtime_warnings
    );
    assert!(res.bridge_warnings.is_empty(), "{:?}", res.bridge_warnings);
    assert!(api.inner.assert_clean_finish());
    let snap = api.inner.snapshot();
    assert_eq!(snap["lesson_status"], "completed");
    assert_eq!(snap["score"]["raw"], "");
    assert_eq!(snap["exit"], "");
    assert!(
        api.inner.violations().is_empty(),
        "{:?}",
        api.inner.violations()
    );
}

#[test]
fn b1_7_scorm12_suspend_overflow_graceful() {
    let (course, _) = load_course("course.json");
    let mut api = Strict12Adapter::new(StrictScorm12::new());
    let (mut player, mut bridge, _, start_w) =
        start_scorm_session(&course, Standard::Scorm12, &mut api, 42);
    assert!(start_w.is_empty(), "{start_w:?}");

    let _ = player.handle(&Input::Start);
    let huge = incompressible_ascii(6000);
    let effs = player.handle(&Input::SetVariable {
        variable: "v_input_text".to_owned(),
        value: Value::Text(huge),
    });
    let w = bridge.after_input(&mut api, &course, player.state(), &effs, 100);
    let fin_w = bridge.finish_session(&mut api, 150);
    let all_w: Vec<String> = w.into_iter().chain(fin_w).collect();
    assert_eq!(
        all_w.len(),
        1,
        "expected 1 suspend overflow warning: {all_w:?}"
    );
    assert!(all_w[0].contains("4096"), "{all_w:?}");

    // Relaunch fresh bridge for score check since finish_session ended the first session.
    api.inner.relaunch();
    let mut bridge2 = LmsBridge::for_standard(Standard::Scorm12);
    let _ = bridge2.start(&mut api, &course);
    let w2 = bridge2.after_input(
        &mut api,
        &course,
        player.state(),
        &[Effect::Score {
            raw: 100,
            max: 100,
            percent: 100,
            status: CompletionStatus::Passed,
        }],
        200,
    );
    assert!(w2.is_empty(), "{w2:?}");
    assert_eq!(api.inner.lms_get_value("cmi.core.lesson_status"), "passed");
    assert_eq!(api.inner.lms_get_value("cmi.core.score.raw"), "100");
    assert!(
        api.inner.violations().is_empty(),
        "{:?}",
        api.inner.violations()
    );
}

#[test]
fn b1_8_scorm12_corrupt_suspend_and_post_completion_relaunch() {
    let (course, _) = load_course("course.json");

    // (a) Corrupt suspend_data -> bridge warns and starts fresh at s01-intro.
    let mut strict = StrictScorm12::new();
    strict.seed_suspend_data("cc1:CORRUPT!!!");
    let mut api = Strict12Adapter::new(strict);
    let (player, _, plan, start_w) = start_scorm_session(&course, Standard::Scorm12, &mut api, 42);
    assert_eq!(
        start_w.len(),
        1,
        "expected corrupt suspend warning: {start_w:?}"
    );
    assert!(plan.resume.is_none());
    assert_eq!(
        player.current_slide().map(|s| s.id.as_str()),
        Some("s01-intro")
    );
    assert!(
        api.inner.violations().is_empty(),
        "{:?}",
        api.inner.violations()
    );

    // (b) Relaunch after passed with suspend_data intact -> lands on s20-cert, stays passed.
    let inputs = load_inputs("pass.inputs.json");
    let strict_keep = StrictScorm12::with_config(Scorm12Config {
        clear_on_normal_exit: false,
        ..Scorm12Config::default()
    });
    let mut api2 = Strict12Adapter::new(strict_keep);
    let (mut p1, mut b1, _, _) = start_scorm_session(&course, Standard::Scorm12, &mut api2, 42);
    let _ = drive_scorm_inputs(&mut p1, &mut b1, &mut api2, &course, &inputs, 0);
    assert_eq!(api2.inner.snapshot()["lesson_status"], "passed");

    api2.inner.relaunch();
    let (p2, _, plan2, w2) = start_scorm_session(&course, Standard::Scorm12, &mut api2, 42);
    assert!(w2.is_empty(), "{w2:?}");
    assert!(plan2.resume.is_some());
    assert_eq!(p2.current_slide().map(|s| s.id.as_str()), Some("s20-cert"));
    assert_eq!(api2.inner.lms_get_value("cmi.core.lesson_status"), "passed");
    assert!(
        api2.inner.violations().is_empty(),
        "{:?}",
        api2.inner.violations()
    );
}

// ---------------------------------------------------------------------------
// B2. SCORM 2004 Automated Runs
// ---------------------------------------------------------------------------

#[test]
fn b2_1_scorm2004_full_pass_4th_ed() {
    let (course, course_json) = load_course("course.json");
    let _ = package_and_validate(
        &course,
        &course_json,
        PackageStandard::Scorm(Standard::Scorm2004),
        PackageStandardKind::Scorm2004,
    );
    let inputs = load_inputs("pass.inputs.json");
    let mut api = Strict2004Adapter::new(StrictScorm2004::new());
    let (mut player, mut bridge, _, start_w) =
        start_scorm_session(&course, Standard::Scorm2004, &mut api, 42);
    assert!(start_w.is_empty(), "{start_w:?}");

    let (last, before_exit) = inputs.split_last().expect("non-empty pass inputs");
    let res = drive_scorm_inputs(&mut player, &mut bridge, &mut api, &course, before_exit, 0);
    assert!(
        res.runtime_warnings.is_empty(),
        "{:?}",
        res.runtime_warnings
    );
    assert!(res.bridge_warnings.is_empty(), "{:?}", res.bridge_warnings);

    assert_eq!(api.inner.get_value("cmi.completion_status"), "completed");
    assert_eq!(api.inner.get_value("cmi.success_status"), "passed");
    assert_eq!(api.inner.get_value("cmi.score.raw"), "100");
    assert_eq!(api.inner.get_value("cmi.score.scaled"), "1");
    assert_eq!(api.inner.get_value("cmi.interactions._count"), "15");

    // Verify all 15 interactions are readable via GetValue (including Freeform as "other").
    verify_scorm2004_interactions_readable(&mut api.inner);

    let res_end = drive_scorm_inputs(
        &mut player,
        &mut bridge,
        &mut api,
        &course,
        std::slice::from_ref(last),
        res.session_ms,
    );
    assert!(
        res_end.runtime_warnings.is_empty(),
        "{:?}",
        res_end.runtime_warnings
    );
    assert!(
        res_end.bridge_warnings.is_empty(),
        "{:?}",
        res_end.bridge_warnings
    );
    assert!(api.inner.assert_clean_terminate());
    assert_eq!(api.inner.snapshot()["exit"], "normal");
    assert!(
        api.inner.calls().iter().any(|c| c.method == "SetValue"
            && c.args.first().map(String::as_str) == Some("cmi.session_time")
            && c.args.get(1).is_some_and(|v| v.starts_with("PT"))),
        "expected ISO 8601 cmi.session_time"
    );
    assert!(
        api.inner.violations().is_empty(),
        "{:?}",
        api.inner.violations()
    );
}

#[test]
fn b2_2_scorm2004_full_fail_4th_ed() {
    let (course, _) = load_course("course.json");
    let inputs = load_inputs("fail.inputs.json");
    let mut api = Strict2004Adapter::new(StrictScorm2004::new());
    let (mut player, mut bridge, _, start_w) =
        start_scorm_session(&course, Standard::Scorm2004, &mut api, 42);
    assert!(start_w.is_empty(), "{start_w:?}");

    let res = drive_scorm_inputs(&mut player, &mut bridge, &mut api, &course, &inputs, 0);
    assert!(
        res.runtime_warnings.is_empty(),
        "{:?}",
        res.runtime_warnings
    );
    assert!(res.bridge_warnings.is_empty(), "{:?}", res.bridge_warnings);
    assert!(api.inner.assert_clean_terminate());

    let snap = api.inner.snapshot();
    assert_eq!(snap["completion_status"], "completed");
    assert_eq!(snap["success_status"], "failed");
    let scaled = snap["score"]["scaled"].as_str().expect("scaled score str");
    assert!(
        is_plain_decimal(scaled),
        "expected plain decimal scaled score, got {scaled}"
    );
    assert_eq!(snap["exit"], "normal");
    let interactions = snap["interactions"].as_array().expect("interactions");
    assert!(interactions.iter().any(|i| i["result"] == "incorrect"));
    assert_eq!(
        interactions
            .iter()
            .filter(|i| i["result"] == "neutral")
            .count(),
        3
    );
    assert!(
        api.inner.violations().is_empty(),
        "{:?}",
        api.inner.violations()
    );
}

#[test]
fn b2_3_scorm2004_partial_suspend_resume_over_4096_bytes() {
    let (course, _) = load_course("course.json");
    let part1 = load_inputs("partial-1.inputs.json");
    let part2 = load_inputs("partial-2-resume.inputs.json");

    let mut api = Strict2004Adapter::new(StrictScorm2004::new());
    let (mut player1, mut bridge1, _, start_w) =
        start_scorm_session(&course, Standard::Scorm2004, &mut api, 77);
    assert!(start_w.is_empty(), "{start_w:?}");

    let res1 = drive_scorm_inputs(&mut player1, &mut bridge1, &mut api, &course, &part1, 0);
    assert!(
        res1.runtime_warnings.is_empty(),
        "{:?}",
        res1.runtime_warnings
    );
    assert!(
        res1.bridge_warnings.is_empty(),
        "{:?}",
        res1.bridge_warnings
    );

    // Store a 6,000-byte incompressible text variable (between 4,096 and 64,000).
    let big_val = incompressible_ascii(6000);
    let effs = player1.handle(&Input::SetVariable {
        variable: "v_input_text".to_owned(),
        value: Value::Text(big_val.clone()),
    });
    let w = bridge1.after_input(
        &mut api,
        &course,
        player1.state(),
        &effs,
        res1.session_ms + 50,
    );
    assert!(w.is_empty(), "{w:?}");

    let fin_w = bridge1.finish_session(&mut api, res1.session_ms + 100);
    assert!(fin_w.is_empty(), "{fin_w:?}");
    assert!(api.inner.assert_clean_terminate());

    let snap1 = api.inner.snapshot();
    let suspend = snap1["suspend_data"].as_str().unwrap_or_default();
    assert!(
        suspend.len() > 4096 && suspend.len() <= 64_000,
        "expected 4096 < suspend.len() <= 64000, got {}",
        suspend.len()
    );

    api.inner.relaunch();
    let (mut player2, mut bridge2, plan2, start_w2) =
        start_scorm_session(&course, Standard::Scorm2004, &mut api, 999);
    assert!(start_w2.is_empty(), "{start_w2:?}");
    assert!(plan2.resume.is_some());
    let start_fx = player2.handle(&Input::Start);
    let _ = bridge2.after_input(&mut api, &course, player2.state(), &start_fx, 0);
    assert_eq!(
        player2.state().variables.get("v_input_text"),
        Some(&Value::Text(big_val))
    );

    let res2 = drive_scorm_inputs(&mut player2, &mut bridge2, &mut api, &course, &part2, 0);
    assert!(
        res2.runtime_warnings.is_empty(),
        "{:?}",
        res2.runtime_warnings
    );
    assert!(
        res2.bridge_warnings.is_empty(),
        "{:?}",
        res2.bridge_warnings
    );
    assert_eq!(api.inner.snapshot()["success_status"], "passed");
    assert!(
        api.inner.violations().is_empty(),
        "{:?}",
        api.inner.violations()
    );
}

#[test]
fn b2_4_scorm2004_3rd_ed_strict_4000_char_limit() {
    let (course, _) = load_course("course.json");
    let mut api = Strict2004Adapter::new(StrictScorm2004::with_config(Scorm2004Config {
        edition: Scorm2004Edition::Third,
        ..Scorm2004Config::default()
    }));
    let (mut player, mut bridge, _, _) =
        start_scorm_session(&course, Standard::Scorm2004, &mut api, 42);

    let _ = player.handle(&Input::Start);
    let big_val = incompressible_ascii(6000);
    let effs = player.handle(&Input::SetVariable {
        variable: "v_input_text".to_owned(),
        value: Value::Text(big_val),
    });
    let _ = bridge.after_input(&mut api, &course, player.state(), &effs, 100);
    let _ = bridge.finish_session(&mut api, 200);
    assert!(
        api.inner
            .violations()
            .iter()
            .any(|v| v.error_code.as_deref() == Some("406") && v.element == "cmi.suspend_data"),
        "3rd Ed. must flag 406 when >4000 chars written: {:?}",
        api.inner.violations()
    );
}

#[test]
fn b2_5_scorm2004_completion_only_leaves_success_unknown() {
    let (course, _) = load_course("completion-only.json");
    let inputs = load_inputs("completion-only.inputs.json");
    let mut api = Strict2004Adapter::new(StrictScorm2004::new());
    let (mut player, mut bridge, _, start_w) =
        start_scorm_session(&course, Standard::Scorm2004, &mut api, 1);
    assert!(start_w.is_empty(), "{start_w:?}");

    let res = drive_scorm_inputs(&mut player, &mut bridge, &mut api, &course, &inputs, 0);
    assert!(
        res.runtime_warnings.is_empty(),
        "{:?}",
        res.runtime_warnings
    );
    assert!(res.bridge_warnings.is_empty(), "{:?}", res.bridge_warnings);
    assert!(api.inner.assert_clean_terminate());
    let snap = api.inner.snapshot();
    assert_eq!(snap["completion_status"], "completed");
    assert_eq!(snap["success_status"], "unknown");
    assert!(
        !api.inner.calls().iter().any(|c| c.method == "SetValue"
            && c.args.first().map(String::as_str) == Some("cmi.success_status")),
        "cmi.success_status must never be written on completion-only course"
    );
    assert_eq!(snap["exit"], "normal");
    assert!(
        api.inner.violations().is_empty(),
        "{:?}",
        api.inner.violations()
    );
}

#[test]
fn b2_6_scorm2004_fail_retry_pass_and_high_water_protection() {
    let (course, _) = load_course("course.json");
    let mut api = Strict2004Adapter::new(StrictScorm2004::with_config(Scorm2004Config {
        clear_on_normal_exit: false,
        ..Scorm2004Config::default()
    }));
    let (player, mut bridge, _, start_w) =
        start_scorm_session(&course, Standard::Scorm2004, &mut api, 42);
    assert!(start_w.is_empty(), "{start_w:?}");
    let state = player.state();

    // Attempt 1: fail (30%)
    let w = bridge.after_input(
        &mut api,
        &course,
        state,
        &[Effect::Score {
            raw: 30,
            max: 100,
            percent: 30,
            status: CompletionStatus::Failed,
        }],
        100,
    );
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(api.inner.get_value("cmi.success_status"), "failed");

    // Attempt 2: pass (85%)
    let w = bridge.after_input(
        &mut api,
        &course,
        state,
        &[Effect::Score {
            raw: 85,
            max: 100,
            percent: 85,
            status: CompletionStatus::Passed,
        }],
        200,
    );
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(api.inner.get_value("cmi.success_status"), "passed");
    assert_eq!(api.inner.get_value("cmi.score.scaled"), "0.85");

    // Attempt 3: fail (10%) -> ignored
    let w = bridge.after_input(
        &mut api,
        &course,
        state,
        &[Effect::Score {
            raw: 10,
            max: 100,
            percent: 10,
            status: CompletionStatus::Failed,
        }],
        300,
    );
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(api.inner.get_value("cmi.success_status"), "passed");
    assert_eq!(api.inner.get_value("cmi.score.scaled"), "0.85");
    let _ = bridge.finish_session(&mut api, 350);

    // Relaunch with LMS returning score.raw = "85.0", success_status = "passed".
    api.inner.relaunch();
    api.inner.seed_status("passed", Some("85.0"));
    let (_, mut bridge2, _, start_w2) =
        start_scorm_session(&course, Standard::Scorm2004, &mut api, 42);
    assert!(start_w2.is_empty(), "{start_w2:?}");

    // Pass 75% -> ignored
    let w = bridge2.after_input(
        &mut api,
        &course,
        state,
        &[Effect::Score {
            raw: 75,
            max: 100,
            percent: 75,
            status: CompletionStatus::Passed,
        }],
        400,
    );
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(api.inner.get_value("cmi.score.raw"), "85.0");

    // Pass 95% -> updates
    let w = bridge2.after_input(
        &mut api,
        &course,
        state,
        &[Effect::Score {
            raw: 95,
            max: 100,
            percent: 95,
            status: CompletionStatus::Passed,
        }],
        500,
    );
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(api.inner.get_value("cmi.score.raw"), "95");
    assert_eq!(api.inner.get_value("cmi.score.scaled"), "0.95");
    assert!(
        api.inner.violations().is_empty(),
        "{:?}",
        api.inner.violations()
    );
}

#[test]
fn b2_7_scorm2004_multi_attempt_try_again_and_unlimited_tf() {
    let (course, _) = load_course("course.json");
    let mut api = Strict2004Adapter::new(StrictScorm2004::new());
    let (mut player, mut bridge, _, start_w) =
        start_scorm_session(&course, Standard::Scorm2004, &mut api, 42);
    assert!(start_w.is_empty(), "{start_w:?}");

    // Walk to s07-q-mc via partial-1 prefix (first 11 inputs).
    let part1 = load_inputs("partial-1.inputs.json");
    let res0 = drive_scorm_inputs(&mut player, &mut bridge, &mut api, &course, &part1[..11], 0);
    assert!(
        res0.runtime_warnings.is_empty(),
        "{:?}",
        res0.runtime_warnings
    );
    assert_eq!(
        player.current_slide().map(|s| s.id.as_str()),
        Some("s07-q-mc")
    );

    // Attempt 1: wrong choice "c" -> try_again layer, attempts_left: Some(1).
    let step1 = drive_scorm_inputs(
        &mut player,
        &mut bridge,
        &mut api,
        &course,
        &[
            Input::SelectChoice {
                choice: "c".to_owned(),
            },
            Input::Click {
                object: "s07-submit".to_owned(),
            },
        ],
        res0.session_ms,
    );
    assert!(
        step1.runtime_warnings.is_empty(),
        "{:?}",
        step1.runtime_warnings
    );
    assert!(
        step1.effects.iter().any(|e| matches!(
            e,
            Effect::QuestionAnswered {
                correct: false,
                attempts_left: Some(1),
                ..
            }
        )),
        "expected try_again with attempts_left Some(1): {:?}",
        step1.effects
    );
    assert!(
        player
            .state()
            .visible_layers
            .contains(&"try_again".to_owned())
    );

    // Hide try_again layer, then submit partial-credit choice "b" (5 pts).
    let step2 = drive_scorm_inputs(
        &mut player,
        &mut bridge,
        &mut api,
        &course,
        &[
            Input::Click {
                object: "s07-ta-btn".to_owned(),
            },
            Input::SelectChoice {
                choice: "b".to_owned(),
            },
            Input::Click {
                object: "s07-submit".to_owned(),
            },
            Input::Click {
                object: "s07-no-btn".to_owned(),
            },
        ],
        step1.session_ms,
    );
    assert!(
        step2.runtime_warnings.is_empty(),
        "{:?}",
        step2.runtime_warnings
    );
    assert_eq!(
        player.state().answers.get("s07-q-mc").map(|a| a.points),
        Some(5)
    );

    // Advance past s08-q-mr to s09-q-tf (unlimited attempts).
    let step3 = drive_scorm_inputs(
        &mut player,
        &mut bridge,
        &mut api,
        &course,
        &[
            Input::SelectChoice {
                choice: "a".to_owned(),
            },
            Input::Submit,
            Input::Click {
                object: "s08-next".to_owned(),
            },
        ],
        step2.session_ms,
    );
    assert!(
        step3.runtime_warnings.is_empty(),
        "{:?}",
        step3.runtime_warnings
    );
    assert_eq!(
        player.current_slide().map(|s| s.id.as_str()),
        Some("s09-q-tf")
    );

    // Submit wrong 3x on s09-q-tf, then right on 4th.
    let mut tf_inputs = Vec::new();
    for _ in 0..3 {
        tf_inputs.push(Input::SelectChoice {
            choice: "false".to_owned(),
        });
        tf_inputs.push(Input::Click {
            object: "s09-submit".to_owned(),
        });
    }
    tf_inputs.push(Input::SelectChoice {
        choice: "true".to_owned(),
    });
    tf_inputs.push(Input::Click {
        object: "s09-submit".to_owned(),
    });
    let step4 = drive_scorm_inputs(
        &mut player,
        &mut bridge,
        &mut api,
        &course,
        &tf_inputs,
        step3.session_ms,
    );
    assert!(
        step4.runtime_warnings.is_empty(),
        "{:?}",
        step4.runtime_warnings
    );
    assert_eq!(
        player.state().answers.get("s09-q-tf").map(|a| a.correct),
        Some(true)
    );

    // Verify sequential interaction IDs (_a1, _a2, ...) and 0 violations.
    let count: usize = api
        .inner
        .get_value("cmi.interactions._count")
        .parse()
        .expect("count");
    assert_eq!(count, 7);
    let id0 = api.inner.get_value("cmi.interactions.0.id");
    let id1 = api.inner.get_value("cmi.interactions.1.id");
    assert!(id0.ends_with("_a1"), "id0={id0}");
    assert!(id1.ends_with("_a2"), "id1={id1}");
    assert!(
        api.inner.violations().is_empty(),
        "{:?}",
        api.inner.violations()
    );
}

#[test]
fn b2_8_scorm2004_location_limit_and_corrupt_interactions_count() {
    let (course, _) = load_course("course.json");

    // (a) Slide ID > 1000 chars skipped with warning and 0 violations.
    let mut api = Strict2004Adapter::new(StrictScorm2004::new());
    let (player, mut bridge, _, start_w) =
        start_scorm_session(&course, Standard::Scorm2004, &mut api, 42);
    assert!(start_w.is_empty(), "{start_w:?}");
    let w = bridge.after_input(
        &mut api,
        &course,
        player.state(),
        &[Effect::SlideEntered {
            slide: "s".repeat(1001),
        }],
        100,
    );
    assert_eq!(w.len(), 1, "expected location limit warning: {w:?}");
    assert!(
        api.inner.violations().is_empty(),
        "{:?}",
        api.inner.violations()
    );

    // (b) Corrupt cmi.interactions._count = "nan" disables interaction writes with 1 warning,
    // while score/completion/suspend still succeed with 0 violations.
    let mut api2 = Strict2004Adapter::new(StrictScorm2004::new());
    api2.corrupt_interactions_count = true;
    let mut bridge2 = LmsBridge::for_standard(Standard::Scorm2004);
    let (_, w2) = bridge2.start(&mut api2, &course);
    assert_eq!(w2.len(), 1, "expected 1 warning for corrupt _count: {w2:?}");

    let w3 = bridge2.after_input(
        &mut api2,
        &course,
        player.state(),
        &[Effect::Score {
            raw: 100,
            max: 100,
            percent: 100,
            status: CompletionStatus::Passed,
        }],
        200,
    );
    assert!(w3.is_empty(), "{w3:?}");
    assert_eq!(api2.inner.get_value("cmi.success_status"), "passed");
    assert_eq!(api2.inner.get_value("cmi.score.raw"), "100");
    assert!(
        api2.inner.violations().is_empty(),
        "{:?}",
        api2.inner.violations()
    );
}

// ---------------------------------------------------------------------------
// B3. cmi5 & xAPI 1.0.3 Automated Runs
// ---------------------------------------------------------------------------

struct StrictLrsAdapter {
    lrs: StrictLrs,
    inbox: VecDeque<LrsResponse>,
    held_fetch: VecDeque<LrsResponse>,
    hold_fetch: bool,
    inject_statement_statuses: VecDeque<u16>,
    requests: Vec<LrsRequest>,
    uuid_counter: u64,
}

impl StrictLrsAdapter {
    fn new(lrs: StrictLrs) -> Self {
        Self {
            lrs,
            inbox: VecDeque::new(),
            held_fetch: VecDeque::new(),
            hold_fetch: false,
            inject_statement_statuses: VecDeque::new(),
            requests: Vec::new(),
            uuid_counter: 0,
        }
    }
}

impl LrsApi for StrictLrsAdapter {
    fn send(&mut self, request: LrsRequest) {
        self.requests.push(request.clone());
        let method = match request.method {
            LrsMethod::Get => "GET",
            LrsMethod::Post => "POST",
            LrsMethod::Put => "PUT",
        };
        let body = request.body.as_deref().unwrap_or("").as_bytes();
        let (path, query) = request.url.split_once('?').unwrap_or((&request.url, ""));
        if let Some(fetch_id) = path
            .rsplit("/fetch/")
            .next()
            .filter(|_| path.contains("/fetch/"))
        {
            let resp = self.lrs.handle_fetch(method, fetch_id, &request.headers);
            let lrs_resp = LrsResponse {
                id: request.id,
                status: resp.status,
                body: String::from_utf8_lossy(&resp.body).into_owned(),
            };
            if self.hold_fetch {
                self.held_fetch.push_back(lrs_resp);
            } else {
                self.inbox.push_back(lrs_resp);
            }
            return;
        }
        if path.ends_with("/statements")
            && let Some(status) = self.inject_statement_statuses.pop_front()
        {
            self.inbox.push_back(LrsResponse {
                id: request.id,
                status,
                body: format!("{{\"injected\":{status}}}"),
            });
            return;
        }
        let resp = if path.ends_with("/statements") {
            self.lrs.handle_statements(method, &request.headers, body)
        } else if path.ends_with("/activities/state") {
            self.lrs.handle_state(method, query, &request.headers, body)
        } else {
            coursecraft_strict_lms::LrsResponse {
                status: 404,
                headers: vec![],
                body: b"not found".to_vec(),
            }
        };
        self.inbox.push_back(LrsResponse {
            id: request.id,
            status: resp.status,
            body: String::from_utf8_lossy(&resp.body).into_owned(),
        });
    }

    fn poll(&mut self) -> Vec<LrsResponse> {
        self.inbox.drain(..).collect()
    }

    fn now_iso(&mut self) -> String {
        "2026-11-21T10:00:00Z".to_owned()
    }

    fn new_uuid(&mut self) -> String {
        self.uuid_counter += 1;
        format!("00000000-0000-4000-8000-{:012x}", self.uuid_counter)
    }
}

fn cmi5_actor_json() -> JsonValue {
    json!({
        "objectType": "Agent",
        "account": { "homePage": "https://lms.example", "name": "alice" },
        "name": "Alice"
    })
}

const CMI5_ACTOR_ENCODED: &str = "%7B%22objectType%22%3A%22Agent%22%2C%22account%22%3A%7B%22homePage%22%3A%22https%3A%2F%2Flms.example%22%2C%22name%22%3A%22alice%22%7D%2C%22name%22%3A%22Alice%22%7D";
const REF_ACTIVITY_ID: &str = "https://lms.example/activities/reference-course";
const REF_ACTIVITY_ENCODED: &str = "https%3A%2F%2Flms.example%2Factivities%2Freference-course";

fn register_cmi5_launch(
    lrs: &mut StrictLrs,
    fetch_id: &str,
    session_id: &str,
    auth_token: &str,
    launch_mode: &str,
    mastery_score: Option<f64>,
    return_url: Option<&str>,
) -> LaunchParams {
    let registration = "11111111-1111-4111-8111-111111111111";
    let launch_doc = build_cmi5_launch_data(
        REF_ACTIVITY_ID,
        session_id,
        launch_mode,
        mastery_score,
        return_url,
    );
    lrs.register_launch(
        LaunchSpec {
            flavor: StrictFlavor::Cmi5,
            activity_id: REF_ACTIVITY_ID.to_owned(),
            actor: cmi5_actor_json(),
            registration: Some(registration.to_owned()),
            launch_mode: launch_mode.to_owned(),
            mastery_score,
            session_id: session_id.to_owned(),
            grouping_ids: vec![REF_ACTIVITY_ID.to_owned()],
            fetch_id: Some(fetch_id.to_owned()),
            auth_token: Some(auth_token.to_owned()),
        },
        Some(launch_doc),
    );
    let query = format!(
        "endpoint=https://lrs.example/xapi&fetch=https://lrs.example/fetch/{fetch_id}&actor={CMI5_ACTOR_ENCODED}&registration={registration}&activityId={REF_ACTIVITY_ENCODED}"
    );
    LaunchParams::parse(&query, Flavor::Cmi5).expect("cmi5 launch params")
}

fn register_xapi_launch(
    lrs: &mut StrictLrs,
    registration: Option<&str>,
    session_id: &str,
    auth_token: &str,
) -> LaunchParams {
    lrs.register_launch(
        LaunchSpec {
            flavor: StrictFlavor::Xapi,
            activity_id: REF_ACTIVITY_ID.to_owned(),
            actor: cmi5_actor_json(),
            registration: registration.map(str::to_owned),
            launch_mode: "Normal".to_owned(),
            mastery_score: None,
            session_id: session_id.to_owned(),
            grouping_ids: Vec::new(),
            fetch_id: None,
            auth_token: Some(auth_token.to_owned()),
        },
        None,
    );
    let mut query = format!(
        "endpoint=https://lrs.example/xapi&auth={auth_token}&actor={CMI5_ACTOR_ENCODED}&activityId={REF_ACTIVITY_ENCODED}"
    );
    if let Some(reg) = registration {
        query.push_str("&registration=");
        query.push_str(reg);
    }
    LaunchParams::parse(&query, Flavor::Xapi).expect("xapi launch params")
}

fn start_xapi_session<'a>(
    course: &'a Course,
    params: LaunchParams,
    flavor: Flavor,
    api: &mut StrictLrsAdapter,
    seed: u32,
    now_ms: &mut u64,
) -> (Player<'a>, XapiBridge, Tick, Vec<String>) {
    let mut bridge = XapiBridge::new(params, flavor, course);
    bridge.start(api);
    let mut warnings = Vec::new();
    let mut plan_tick = None;
    for _ in 0..10 {
        *now_ms += 100;
        let tick = bridge.tick(api, course, *now_ms);
        warnings.extend(tick.warnings.clone());
        if tick.plan.is_some() {
            plan_tick = Some(tick);
            break;
        }
    }
    let tick = plan_tick.expect("xapi handshake plan");
    let player = match tick.plan.as_ref().and_then(|p| p.resume.clone()) {
        Some(state) => Player::resume(course, state).expect("resume player"),
        None => Player::with_draw_seed(course, seed).expect("fresh player"),
    };
    (player, bridge, tick, warnings)
}

fn pump_xapi(
    bridge: &mut XapiBridge,
    api: &mut StrictLrsAdapter,
    course: &Course,
    now_ms: &mut u64,
    warnings: &mut Vec<String>,
) {
    for _ in 0..20 {
        if !bridge.is_busy() {
            break;
        }
        *now_ms += 1000;
        let tick = bridge.tick(api, course, *now_ms);
        warnings.extend(tick.warnings);
    }
}

fn drive_xapi_inputs(
    player: &mut Player<'_>,
    bridge: &mut XapiBridge,
    api: &mut StrictLrsAdapter,
    course: &Course,
    inputs: &[Input],
    start_ms: u64,
    now_ms: &mut u64,
) -> RunResult {
    let mut effects = Vec::new();
    let mut runtime_warnings = Vec::new();
    let mut bridge_warnings = Vec::new();
    let mut session_ms = start_ms;

    for input in inputs {
        if let Input::Tick { ms } = input {
            session_ms = session_ms.saturating_add(u64::from(*ms));
            *now_ms = now_ms.saturating_add(u64::from(*ms));
        } else {
            session_ms = session_ms.saturating_add(250);
            *now_ms = now_ms.saturating_add(250);
        }
        let step_effects = player.handle(input);
        for eff in &step_effects {
            if let Effect::Warning { message } = eff {
                runtime_warnings.push(message.clone());
            }
        }
        let bw = bridge.after_input(
            api,
            course,
            player.state(),
            &step_effects,
            session_ms,
            *now_ms,
        );
        bridge_warnings.extend(bw);
        effects.extend(step_effects);
        pump_xapi(bridge, api, course, now_ms, &mut bridge_warnings);
    }

    RunResult {
        effects,
        runtime_warnings,
        bridge_warnings,
        session_ms,
    }
}

fn verb_short(stmt: &JsonValue) -> &str {
    stmt.pointer("/verb/id")
        .and_then(JsonValue::as_str)
        .and_then(|s| s.rsplit('/').next())
        .unwrap_or("")
}

fn category_ids(stmt: &JsonValue) -> Vec<&str> {
    stmt.pointer("/context/contextActivities/category")
        .and_then(JsonValue::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.get("id").and_then(JsonValue::as_str))
                .collect()
        })
        .unwrap_or_default()
}

fn build_80_percent_inputs() -> Vec<Input> {
    // 12 graded questions (9 in sc-quiz + 3 drawn from bank-pool = 120 pts).
    // Answer s07-s15 + 1st bank draw right (100/120 = 83% >= 70% course pass, < 90% LMS mastery),
    // and last 2 bank draws wrong ("b").
    let mut inputs = load_inputs("partial-1.inputs.json");
    let mut part2 = load_inputs("partial-2-resume.inputs.json");
    // Replace the last 2 bank answers ("a" -> "b") before btn-cert/btn-exit-cert.
    let len = part2.len();
    assert!(len >= 8);
    part2[len - 8] = Input::SelectChoice {
        choice: "b".to_owned(),
    };
    part2[len - 5] = Input::SelectChoice {
        choice: "b".to_owned(),
    };
    inputs.extend(part2);
    inputs
}

fn assert_cmi5_pass_statements(stmts: &[JsonValue]) {
    let verbs: Vec<&str> = stmts.iter().map(verb_short).collect();
    assert_eq!(verbs.first().copied(), Some("initialized"));
    assert_eq!(verbs.last().copied(), Some("terminated"));
    assert_eq!(verbs.iter().filter(|&&v| v == "answered").count(), 15);
    assert_eq!(verbs.iter().filter(|&&v| v == "passed").count(), 1);
    assert_eq!(verbs.iter().filter(|&&v| v == "completed").count(), 1);

    const MASTERY_EXT: &str = "https://w3id.org/xapi/cmi5/context/extensions/masteryscore";
    const CMI5_CAT: &str = "https://w3id.org/xapi/cmi5/context/categories/cmi5";
    const MOVEON_CAT: &str = "https://w3id.org/xapi/cmi5/context/categories/moveon";
    let mastery_ptr = format!("/context/extensions/{}", MASTERY_EXT.replace('/', "~1"));

    for stmt in stmts {
        let v = verb_short(stmt);
        let cats = category_ids(stmt);
        let has_mastery = stmt.pointer(&mastery_ptr).is_some();
        match v {
            "passed" => {
                assert!(has_mastery, "passed must include masteryscore");
                assert_eq!(
                    stmt.pointer(&mastery_ptr).and_then(JsonValue::as_f64),
                    Some(0.8)
                );
                assert_eq!(
                    stmt.pointer("/result/score/scaled")
                        .and_then(JsonValue::as_f64),
                    Some(1.0)
                );
                assert!(cats.contains(&CMI5_CAT));
                assert!(cats.contains(&MOVEON_CAT));
            }
            "completed" => {
                assert!(!has_mastery, "masteryscore must be on passed/failed only");
                assert!(cats.contains(&CMI5_CAT));
                assert!(cats.contains(&MOVEON_CAT));
            }
            "initialized" | "terminated" => {
                assert!(!has_mastery);
                assert!(cats.contains(&CMI5_CAT));
                assert!(!cats.contains(&MOVEON_CAT));
            }
            "answered" => {
                assert!(!has_mastery);
                assert!(!cats.contains(&CMI5_CAT));
                assert!(!cats.contains(&MOVEON_CAT));
            }
            _ => {}
        }
    }
}

#[test]
fn b3_1_cmi5_full_pass() {
    let (course, course_json) = load_course("course.json");
    package_and_validate(
        &course,
        &course_json,
        PackageStandard::Xapi(Flavor::Cmi5),
        PackageStandardKind::Cmi5,
    );

    let mut lrs = StrictLrs::new();
    let params = register_cmi5_launch(
        &mut lrs,
        "f1",
        "sess-1",
        "tok-1",
        "Normal",
        Some(0.8),
        Some("https://lms.example/return"),
    );
    let mut api = StrictLrsAdapter::new(lrs);
    let mut now_ms = 0;
    let (mut player, mut bridge, _tick, start_w) =
        start_xapi_session(&course, params, Flavor::Cmi5, &mut api, 42, &mut now_ms);
    assert!(start_w.is_empty(), "start warnings: {start_w:?}");
    assert_eq!(bridge.return_url(), Some("https://lms.example/return"));

    let inputs = load_inputs("pass.inputs.json");
    let res = drive_xapi_inputs(
        &mut player,
        &mut bridge,
        &mut api,
        &course,
        &inputs,
        0,
        &mut now_ms,
    );
    assert!(
        res.runtime_warnings.is_empty(),
        "{:?}",
        res.runtime_warnings
    );
    assert!(res.bridge_warnings.is_empty(), "{:?}", res.bridge_warnings);
    assert!(!bridge.reporting_lost());
    assert!(
        api.lrs.violations().is_empty(),
        "cmi5 violations: {:?}",
        api.lrs.violations()
    );

    assert_cmi5_pass_statements(api.lrs.statements());
}

#[test]
fn b3_2_cmi5_lms_mastery_overrides_course_passing() {
    let (course, _) = load_course("course.json");
    let mut lrs = StrictLrs::new();
    let params = register_cmi5_launch(&mut lrs, "f1", "sess-1", "tok-1", "Normal", Some(0.9), None);
    let mut api = StrictLrsAdapter::new(lrs);
    let mut now_ms = 0;
    let (mut player, mut bridge, _tick, start_w) =
        start_xapi_session(&course, params, Flavor::Cmi5, &mut api, 42, &mut now_ms);
    assert!(start_w.is_empty(), "{start_w:?}");

    let inputs = build_80_percent_inputs();
    let res = drive_xapi_inputs(
        &mut player,
        &mut bridge,
        &mut api,
        &course,
        &inputs,
        0,
        &mut now_ms,
    );
    assert!(
        res.runtime_warnings.is_empty(),
        "{:?}",
        res.runtime_warnings
    );
    assert!(res.bridge_warnings.is_empty(), "{:?}", res.bridge_warnings);
    assert!(
        api.lrs.violations().is_empty(),
        "violations: {:?}",
        api.lrs.violations()
    );

    let verbs: Vec<&str> = api.lrs.statements().iter().map(verb_short).collect();
    assert!(verbs.contains(&"failed"), "expected failed: {verbs:?}");
    assert!(
        !verbs.contains(&"passed"),
        "must not emit passed: {verbs:?}"
    );
    assert!(
        verbs.contains(&"completed"),
        "expected completed: {verbs:?}"
    );

    let failed_stmt = api
        .lrs
        .statements()
        .iter()
        .find(|s| verb_short(s) == "failed")
        .expect("failed statement");
    assert_eq!(
        failed_stmt
            .pointer("/context/extensions/https:~1~1w3id.org~1xapi~1cmi5~1context~1extensions~1masteryscore")
            .and_then(JsonValue::as_f64),
        Some(0.9)
    );
}

#[test]
fn b3_3_cmi5_fail_relaunch_resume_pass_two_sessions() {
    let (course, _) = load_course("course.json");
    let mut lrs = StrictLrs::new();
    let params1 =
        register_cmi5_launch(&mut lrs, "f1", "sess-1", "tok-1", "Normal", Some(0.7), None);
    let mut api = StrictLrsAdapter::new(lrs);
    let mut now_ms = 0;
    let (mut player1, mut bridge1, _tick1, start_w1) =
        start_xapi_session(&course, params1, Flavor::Cmi5, &mut api, 42, &mut now_ms);
    assert!(start_w1.is_empty(), "{start_w1:?}");

    let mut fail_no_exit = load_inputs("fail.inputs.json");
    fail_no_exit.pop(); // btn-exit
    let res1 = drive_xapi_inputs(
        &mut player1,
        &mut bridge1,
        &mut api,
        &course,
        &fail_no_exit,
        0,
        &mut now_ms,
    );
    assert!(
        res1.runtime_warnings.is_empty(),
        "{:?}",
        res1.runtime_warnings
    );
    assert!(
        res1.bridge_warnings.is_empty(),
        "{:?}",
        res1.bridge_warnings
    );

    // Click btn-retry before closing Session 1 so Session 2 resumes at s07-q-mc rather
    // than re-entering s19-results (whose timeline_starts submit_results would re-emit
    // failed in Session 2).
    let retry_fx = player1.handle(&Input::Click {
        object: "btn-retry".to_owned(),
    });
    let _ = bridge1.after_input(
        &mut api,
        &course,
        player1.state(),
        &retry_fx,
        res1.session_ms + 250,
        now_ms,
    );
    let fin_w1 = bridge1.finish_session(
        &mut api,
        &course,
        Some(player1.state()),
        res1.session_ms + 250,
    );
    assert!(fin_w1.is_empty(), "{fin_w1:?}");

    let verbs1: Vec<&str> = api.lrs.statements().iter().map(verb_short).collect();
    assert!(verbs1.contains(&"failed"));
    assert!(verbs1.contains(&"completed"));
    assert!(verbs1.contains(&"terminated"));

    let progress_json = api
        .lrs
        .get_state(
            "coursecraft.progress",
            REF_ACTIVITY_ID,
            &cmi5_actor_json(),
            Some("11111111-1111-4111-8111-111111111111"),
        )
        .expect("saved progress state");
    let progress_val: JsonValue = serde_json::from_slice(progress_json).expect("progress json");
    assert_eq!(
        progress_val.get("completed").and_then(JsonValue::as_bool),
        Some(true)
    );
    assert_eq!(
        progress_val.get("passed").and_then(JsonValue::as_bool),
        Some(false)
    );

    // Session 2: relaunch with new session_id & fetch token, resume, retry & pass.
    let s1_count = api.lrs.statements().len();
    let params2 = register_cmi5_launch(
        &mut api.lrs,
        "f2",
        "sess-2",
        "tok-2",
        "Normal",
        Some(0.7),
        None,
    );
    let (mut player2, mut bridge2, tick2, start_w2) =
        start_xapi_session(&course, params2, Flavor::Cmi5, &mut api, 99, &mut now_ms);
    assert!(start_w2.is_empty(), "{start_w2:?}");
    assert!(
        tick2
            .plan
            .as_ref()
            .and_then(|p| p.resume.as_ref())
            .is_some()
    );
    let start_fx2 = player2.handle(&Input::Start);
    let _ = bridge2.after_input(
        &mut api,
        &course,
        player2.state(),
        &start_fx2,
        res1.session_ms + 250,
        now_ms,
    );

    // Skip start..btn-lb-jump in pass.inputs.json, replay quiz + cert + exit.
    let pass_all = load_inputs("pass.inputs.json");
    let quiz_start = pass_all
        .iter()
        .position(|i| matches!(i, Input::Click { object } if object == "btn-lb-jump"))
        .expect("btn-lb-jump")
        + 1;
    let pass_retry = pass_all[quiz_start..].to_vec();

    let res2 = drive_xapi_inputs(
        &mut player2,
        &mut bridge2,
        &mut api,
        &course,
        &pass_retry,
        res1.session_ms,
        &mut now_ms,
    );
    assert!(
        res2.runtime_warnings.is_empty(),
        "{:?}",
        res2.runtime_warnings
    );
    assert!(
        res2.bridge_warnings.is_empty(),
        "{:?}",
        res2.bridge_warnings
    );
    assert!(
        api.lrs.violations().is_empty(),
        "violations across 2 sessions: {:?}",
        api.lrs.violations()
    );

    let verbs2: Vec<&str> = api.lrs.statements()[s1_count..]
        .iter()
        .map(verb_short)
        .collect();
    assert!(verbs2.contains(&"initialized"));
    assert!(verbs2.contains(&"passed"));
    assert!(verbs2.contains(&"terminated"));
    assert!(
        !verbs2.contains(&"completed"),
        "must not emit second completed in session 2: {verbs2:?}"
    );
}

#[test]
fn b3_4_cmi5_fail_retry_pass_same_session_guard() {
    let (course, _) = load_course("course.json");
    let mut lrs = StrictLrs::new();
    let params = register_cmi5_launch(&mut lrs, "f1", "sess-1", "tok-1", "Normal", Some(0.7), None);
    let mut api = StrictLrsAdapter::new(lrs);
    let mut now_ms = 0;
    let (mut player, mut bridge, _tick, start_w) =
        start_xapi_session(&course, params, Flavor::Cmi5, &mut api, 42, &mut now_ms);
    assert!(start_w.is_empty(), "{start_w:?}");

    // Fail attempt 1 (stop before btn-exit so we stay in the same session).
    let mut fail_no_exit = load_inputs("fail.inputs.json");
    fail_no_exit.pop(); // btn-exit
    let res1 = drive_xapi_inputs(
        &mut player,
        &mut bridge,
        &mut api,
        &course,
        &fail_no_exit,
        0,
        &mut now_ms,
    );
    assert!(
        res1.runtime_warnings.is_empty(),
        "{:?}",
        res1.runtime_warnings
    );
    assert!(
        res1.bridge_warnings.is_empty(),
        "{:?}",
        res1.bridge_warnings
    );

    // Retry and pass in the SAME session.
    let mut retry_pass = vec![Input::Click {
        object: "btn-retry".to_owned(),
    }];
    let pass_all = load_inputs("pass.inputs.json");
    let quiz_start = pass_all
        .iter()
        .position(|i| matches!(i, Input::Click { object } if object == "btn-lb-jump"))
        .expect("btn-lb-jump")
        + 1;
    retry_pass.extend_from_slice(&pass_all[quiz_start..]);

    let res2 = drive_xapi_inputs(
        &mut player,
        &mut bridge,
        &mut api,
        &course,
        &retry_pass,
        res1.session_ms,
        &mut now_ms,
    );
    assert!(
        res2.runtime_warnings.is_empty(),
        "{:?}",
        res2.runtime_warnings
    );
    assert!(
        res2.bridge_warnings.is_empty(),
        "{:?}",
        res2.bridge_warnings
    );
    assert!(
        api.lrs.violations().is_empty(),
        "cmi5 §9.3.1 guard must prevent passed-after-failed violations: {:?}",
        api.lrs.violations()
    );

    let verbs: Vec<&str> = api.lrs.statements().iter().map(verb_short).collect();
    assert!(verbs.contains(&"failed"));
    assert!(
        !verbs.contains(&"passed"),
        "XapiBridge must suppress passed after failed in same session: {verbs:?}"
    );
}

#[test]
fn b3_5_cmi5_partial_suspend_resume_with_lightbox() {
    let (course, _) = load_course("course.json");
    let mut lrs = StrictLrs::new();
    let params1 =
        register_cmi5_launch(&mut lrs, "f1", "sess-1", "tok-1", "Normal", Some(0.7), None);
    let mut api = StrictLrsAdapter::new(lrs);
    let mut now_ms = 0;
    let (mut player1, mut bridge1, _tick1, start_w1) =
        start_xapi_session(&course, params1, Flavor::Cmi5, &mut api, 42, &mut now_ms);
    assert!(start_w1.is_empty(), "{start_w1:?}");

    // Walk partial-1.inputs.json, inserting lightbox open+close on s06-branch-lightbox (BUG-A9).
    let mut partial1 = load_inputs("partial-1.inputs.json");
    let to_quiz_idx = partial1
        .iter()
        .position(|i| matches!(i, Input::Click { object } if object == "btn-to-quiz"))
        .expect("btn-to-quiz");
    partial1.splice(
        to_quiz_idx..to_quiz_idx,
        [
            Input::Click {
                object: "btn-open-lb".to_owned(),
            },
            Input::Click {
                object: "btn-close-lb".to_owned(),
            },
        ],
    );

    let res1 = drive_xapi_inputs(
        &mut player1,
        &mut bridge1,
        &mut api,
        &course,
        &partial1,
        0,
        &mut now_ms,
    );
    assert!(
        res1.runtime_warnings.is_empty(),
        "{:?}",
        res1.runtime_warnings
    );
    assert!(
        res1.bridge_warnings.is_empty(),
        "{:?}",
        res1.bridge_warnings
    );

    let pre_state = player1.state().clone();
    assert!(pre_state.visited.contains("s06c-lightbox"));

    let finish_w = bridge1.finish_session(&mut api, &course, Some(&pre_state), res1.session_ms);
    assert!(finish_w.is_empty(), "{finish_w:?}");

    // Relaunch in Session 2.
    let params2 = register_cmi5_launch(
        &mut api.lrs,
        "f2",
        "sess-2",
        "tok-2",
        "Normal",
        Some(0.7),
        None,
    );
    let (mut player2, mut bridge2, tick2, start_w2) =
        start_xapi_session(&course, params2, Flavor::Cmi5, &mut api, 99, &mut now_ms);
    assert!(start_w2.is_empty(), "{start_w2:?}");
    assert!(
        tick2
            .plan
            .as_ref()
            .and_then(|p| p.resume.as_ref())
            .is_some()
    );
    let start_fx2 = player2.handle(&Input::Start);
    let _ = bridge2.after_input(
        &mut api,
        &course,
        player2.state(),
        &start_fx2,
        res1.session_ms,
        now_ms,
    );
    assert_resumed_state_matches(&pre_state, player2.state());

    let partial2 = load_inputs("partial-2-resume.inputs.json");
    let res2 = drive_xapi_inputs(
        &mut player2,
        &mut bridge2,
        &mut api,
        &course,
        &partial2,
        res1.session_ms,
        &mut now_ms,
    );
    assert!(
        res2.runtime_warnings.is_empty(),
        "{:?}",
        res2.runtime_warnings
    );
    assert!(
        res2.bridge_warnings.is_empty(),
        "{:?}",
        res2.bridge_warnings
    );
    assert!(
        api.lrs.violations().is_empty(),
        "violations: {:?}",
        api.lrs.violations()
    );

    let verbs: Vec<&str> = api.lrs.statements().iter().map(verb_short).collect();
    assert!(verbs.contains(&"passed"));
    assert!(verbs.contains(&"completed"));
}

#[test]
fn b3_6_cmi5_browse_and_review_modes() {
    let (course, _) = load_course("course.json");
    let inputs = load_inputs("pass.inputs.json");

    for mode in ["Browse", "Review"] {
        let mut lrs = StrictLrs::new();
        let params = register_cmi5_launch(&mut lrs, "f1", "sess-1", "tok-1", mode, Some(0.8), None);
        let mut api = StrictLrsAdapter::new(lrs);
        let mut now_ms = 0;
        let (mut player, mut bridge, _tick, start_w) =
            start_xapi_session(&course, params, Flavor::Cmi5, &mut api, 42, &mut now_ms);
        assert!(start_w.is_empty(), "{mode} start warnings: {start_w:?}");

        let res = drive_xapi_inputs(
            &mut player,
            &mut bridge,
            &mut api,
            &course,
            &inputs,
            0,
            &mut now_ms,
        );
        assert!(
            res.runtime_warnings.is_empty(),
            "{mode}: {:?}",
            res.runtime_warnings
        );
        assert!(
            res.bridge_warnings.is_empty(),
            "{mode}: {:?}",
            res.bridge_warnings
        );
        assert!(
            !bridge.reporting_lost(),
            "{mode} must not mark reporting_lost"
        );
        assert!(
            api.lrs.violations().is_empty(),
            "{mode} violations: {:?}",
            api.lrs.violations()
        );

        let verbs: Vec<&str> = api.lrs.statements().iter().map(verb_short).collect();
        assert_eq!(
            verbs,
            vec!["initialized", "terminated"],
            "{mode} must emit only initialized + terminated"
        );
        let put_state_count = api
            .requests
            .iter()
            .filter(|r| r.method == LrsMethod::Put && r.url.contains("/activities/state"))
            .count();
        assert_eq!(put_state_count, 0, "{mode} must not PUT progress");
    }
}

#[test]
fn b3_7_cmi5_5xx_retry_storm_and_401_lost_reporting() {
    let (course, _) = load_course("course.json");
    let mut lrs = StrictLrs::new();
    let params = register_cmi5_launch(&mut lrs, "f1", "sess-1", "tok-1", "Normal", Some(0.8), None);
    let mut api = StrictLrsAdapter::new(lrs);
    // Inject 3x HTTP 500 on /statements (will hit initialized batch).
    api.inject_statement_statuses.push_back(500);
    api.inject_statement_statuses.push_back(500);
    api.inject_statement_statuses.push_back(500);

    let mut now_ms = 0;
    let (mut player, mut bridge, _tick, start_w) =
        start_xapi_session(&course, params, Flavor::Cmi5, &mut api, 42, &mut now_ms);
    assert!(start_w.is_empty(), "{start_w:?}");

    // Pump backoff ticks so all 3x 500s recover and initialized lands.
    let mut pump_w = Vec::new();
    for _ in 0..20 {
        if api
            .lrs
            .statements()
            .iter()
            .any(|s| verb_short(s) == "initialized")
        {
            break;
        }
        now_ms += 2000;
        let t = bridge.tick(&mut api, &course, now_ms);
        pump_w.extend(t.warnings);
    }
    assert!(
        api.lrs
            .statements()
            .iter()
            .any(|s| verb_short(s) == "initialized"),
        "initialized must succeed after 3x 500 retries"
    );
    assert!(!bridge.reporting_lost());

    // Now inject 401 on the next /statements POST.
    api.inject_statement_statuses.push_back(401);
    let step_effects = player.handle(&Input::Tick { ms: 2100 });
    let w_after = bridge.after_input(
        &mut api,
        &course,
        player.state(),
        &step_effects,
        2100,
        now_ms,
    );
    let _ = w_after;

    // Trigger an answered or completion effect to force a /statements POST that hits 401.
    let step_effects2 = player.handle(&Input::Click {
        object: "s01-btn-action".to_owned(),
    });
    let _ = bridge.after_input(
        &mut api,
        &course,
        player.state(),
        &step_effects2,
        2350,
        now_ms,
    );
    let w_comp = bridge.after_input(
        &mut api,
        &course,
        player.state(),
        &[Effect::Completion {
            status: CompletionStatus::Completed,
        }],
        2400,
        now_ms,
    );
    now_ms += 1000;
    let tick_after_401 = bridge.tick(&mut api, &course, now_ms);
    assert!(
        !w_comp.is_empty() || !tick_after_401.warnings.is_empty() || bridge.reporting_lost(),
        "expected 401 to trigger reporting_lost"
    );
    assert!(bridge.reporting_lost(), "expected reporting_lost after 401");

    let reqs_at_lost = api.requests.len();
    let _ = bridge.after_input(
        &mut api,
        &course,
        player.state(),
        &[Effect::Score {
            raw: 100,
            max: 100,
            percent: 100,
            status: CompletionStatus::Passed,
        }],
        2500,
        now_ms,
    );
    let _ = bridge.finish_session(&mut api, &course, Some(player.state()), 2600);
    assert_eq!(
        api.requests.len(),
        reqs_at_lost,
        "zero HTTP requests allowed after 401 reporting_lost"
    );
    assert!(
        api.lrs.violations().is_empty(),
        "violations: {:?}",
        api.lrs.violations()
    );
}

#[test]
fn b3_8_cmi5_handshake_timeout_and_reused_fetch() {
    let (course, _) = load_course("course.json");

    // (a) Handshake timeout past HANDSHAKE_TIMEOUT_MS (8000ms).
    let mut lrs_a = StrictLrs::new();
    let params_a = register_cmi5_launch(
        &mut lrs_a,
        "f1",
        "sess-1",
        "tok-1",
        "Normal",
        Some(0.8),
        None,
    );
    let mut api_a = StrictLrsAdapter::new(lrs_a);
    api_a.hold_fetch = true;

    let mut bridge_a = XapiBridge::new(params_a, Flavor::Cmi5, &course);
    bridge_a.start(&mut api_a);
    let t1 = bridge_a.tick(&mut api_a, &course, 100);
    assert!(t1.plan.is_none());
    let t2 = bridge_a.tick(&mut api_a, &course, 100 + HANDSHAKE_TIMEOUT_MS);
    assert!(t2.plan.is_some(), "expected Fresh plan on timeout");
    assert!(t2.plan.as_ref().and_then(|p| p.resume.as_ref()).is_none());
    assert!(bridge_a.reporting_lost());

    // Deliver the held late fetch response; bridge must ignore it.
    let late = api_a.held_fetch.drain(..).collect::<Vec<_>>();
    api_a.inbox.extend(late);
    let reqs_before = api_a.requests.len();
    let _ = bridge_a.tick(&mut api_a, &course, 200 + HANDSHAKE_TIMEOUT_MS);
    assert_eq!(
        api_a.requests.len(),
        reqs_before,
        "late fetch response after timeout must be ignored"
    );

    // (b) Reused fetch URL returns error-code "1".
    let mut lrs_b = StrictLrs::new();
    let params_b = register_cmi5_launch(
        &mut lrs_b,
        "f-reused",
        "sess-1",
        "tok-1",
        "Normal",
        Some(0.8),
        None,
    );
    // Pre-consume the single-use fetch URL.
    let first = lrs_b.handle_fetch("POST", "f-reused", &[]);
    assert_eq!(first.status, 200);

    let mut api_b = StrictLrsAdapter::new(lrs_b);
    let mut now_ms = 0;
    let (_player_b, bridge_b, tick_b, _w_b) =
        start_xapi_session(&course, params_b, Flavor::Cmi5, &mut api_b, 42, &mut now_ms);
    assert!(
        tick_b
            .plan
            .as_ref()
            .and_then(|p| p.resume.as_ref())
            .is_none()
    );
    assert!(bridge_b.reporting_lost());
}

#[test]
fn b3_9_plain_xapi_pass_fail_resume_relaunch_and_no_registration() {
    let (course, course_json) = load_course("course.json");
    package_and_validate(
        &course,
        &course_json,
        PackageStandard::Xapi(Flavor::Xapi),
        PackageStandardKind::Xapi,
    );

    // 1. Pass with registration.
    {
        let mut lrs = StrictLrs::new();
        let params = register_xapi_launch(
            &mut lrs,
            Some("22222222-2222-4222-8222-222222222222"),
            "xapi-pass",
            "Basic dG9r",
        );
        let mut api = StrictLrsAdapter::new(lrs);
        let mut now_ms = 0;
        let (mut player, mut bridge, _tick, start_w) =
            start_xapi_session(&course, params, Flavor::Xapi, &mut api, 42, &mut now_ms);
        assert!(start_w.is_empty(), "{start_w:?}");
        let res = drive_xapi_inputs(
            &mut player,
            &mut bridge,
            &mut api,
            &course,
            &load_inputs("pass.inputs.json"),
            0,
            &mut now_ms,
        );
        assert!(
            res.runtime_warnings.is_empty(),
            "{:?}",
            res.runtime_warnings
        );
        assert!(res.bridge_warnings.is_empty(), "{:?}", res.bridge_warnings);
        assert!(
            api.lrs.violations().is_empty(),
            "xapi pass violations: {:?}",
            api.lrs.violations()
        );
        for stmt in api.lrs.statements() {
            let cats = category_ids(stmt);
            assert!(
                !cats
                    .iter()
                    .any(|c| c.contains("cmi5") || c.contains("moveon")),
                "plain xAPI must not emit cmi5/moveon categories: {cats:?}"
            );
        }
    }

    // 2. Fail with registration.
    {
        let mut lrs = StrictLrs::new();
        let params = register_xapi_launch(
            &mut lrs,
            Some("33333333-3333-4333-8333-333333333333"),
            "xapi-fail",
            "Basic dG9r",
        );
        let mut api = StrictLrsAdapter::new(lrs);
        let mut now_ms = 0;
        let (mut player, mut bridge, _tick, start_w) =
            start_xapi_session(&course, params, Flavor::Xapi, &mut api, 42, &mut now_ms);
        assert!(start_w.is_empty(), "{start_w:?}");
        let res = drive_xapi_inputs(
            &mut player,
            &mut bridge,
            &mut api,
            &course,
            &load_inputs("fail.inputs.json"),
            0,
            &mut now_ms,
        );
        assert!(
            res.runtime_warnings.is_empty(),
            "{:?}",
            res.runtime_warnings
        );
        assert!(res.bridge_warnings.is_empty(), "{:?}", res.bridge_warnings);
        assert!(
            api.lrs.violations().is_empty(),
            "{:?}",
            api.lrs.violations()
        );
        let verbs: Vec<&str> = api.lrs.statements().iter().map(verb_short).collect();
        assert!(verbs.contains(&"failed"));
        assert!(verbs.contains(&"completed"));
    }

    // 3. Partial + Resume + Relaunch on same registration.
    {
        let mut lrs = StrictLrs::new();
        let reg = "44444444-4444-4444-8444-444444444444";
        let params1 = register_xapi_launch(&mut lrs, Some(reg), "xapi-s1", "Basic dG9rMQ==");
        let mut api = StrictLrsAdapter::new(lrs);
        let mut now_ms = 0;
        let (mut player1, mut bridge1, _tick1, _) =
            start_xapi_session(&course, params1, Flavor::Xapi, &mut api, 42, &mut now_ms);
        let res1 = drive_xapi_inputs(
            &mut player1,
            &mut bridge1,
            &mut api,
            &course,
            &load_inputs("partial-1.inputs.json"),
            0,
            &mut now_ms,
        );
        let _ = bridge1.finish_session(&mut api, &course, Some(player1.state()), res1.session_ms);

        let params2 = register_xapi_launch(&mut api.lrs, Some(reg), "xapi-s2", "Basic dG9rMg==");
        let (mut player2, mut bridge2, tick2, _) =
            start_xapi_session(&course, params2, Flavor::Xapi, &mut api, 99, &mut now_ms);
        assert!(
            tick2
                .plan
                .as_ref()
                .and_then(|p| p.resume.as_ref())
                .is_some()
        );
        let start_fx2 = player2.handle(&Input::Start);
        let _ = bridge2.after_input(
            &mut api,
            &course,
            player2.state(),
            &start_fx2,
            res1.session_ms,
            now_ms,
        );
        let res2 = drive_xapi_inputs(
            &mut player2,
            &mut bridge2,
            &mut api,
            &course,
            &load_inputs("partial-2-resume.inputs.json"),
            res1.session_ms,
            &mut now_ms,
        );
        assert!(
            res2.runtime_warnings.is_empty(),
            "{:?}",
            res2.runtime_warnings
        );
        assert!(
            res2.bridge_warnings.is_empty(),
            "{:?}",
            res2.bridge_warnings
        );
        assert!(
            api.lrs.violations().is_empty(),
            "relaunch on same registration violations: {:?}",
            api.lrs.violations()
        );
    }

    // 4. Launch with registration: None.
    {
        let mut lrs = StrictLrs::new();
        let params = register_xapi_launch(&mut lrs, None, "xapi-noreg", "Basic dG9r");
        let mut api = StrictLrsAdapter::new(lrs);
        let mut now_ms = 0;
        let (mut player, mut bridge, _tick, start_w) =
            start_xapi_session(&course, params, Flavor::Xapi, &mut api, 42, &mut now_ms);
        assert!(start_w.is_empty(), "{start_w:?}");
        let res = drive_xapi_inputs(
            &mut player,
            &mut bridge,
            &mut api,
            &course,
            &load_inputs("pass.inputs.json"),
            0,
            &mut now_ms,
        );
        assert!(
            res.runtime_warnings.is_empty(),
            "{:?}",
            res.runtime_warnings
        );
        assert!(res.bridge_warnings.is_empty(), "{:?}", res.bridge_warnings);
        assert!(
            api.lrs.violations().is_empty(),
            "{:?}",
            api.lrs.violations()
        );
    }
}

#[test]
fn b3_10_cmi5_completion_only_with_mastery_score() {
    let (course, _) = load_course("completion-only.json");
    let mut lrs = StrictLrs::new();
    let params = register_cmi5_launch(&mut lrs, "f1", "sess-1", "tok-1", "Normal", Some(0.8), None);
    let mut api = StrictLrsAdapter::new(lrs);
    let mut now_ms = 0;
    let (mut player, mut bridge, _tick, start_w) =
        start_xapi_session(&course, params, Flavor::Cmi5, &mut api, 42, &mut now_ms);
    assert!(start_w.is_empty(), "{start_w:?}");

    let res = drive_xapi_inputs(
        &mut player,
        &mut bridge,
        &mut api,
        &course,
        &load_inputs("completion-only.inputs.json"),
        0,
        &mut now_ms,
    );
    assert!(
        res.runtime_warnings.is_empty(),
        "{:?}",
        res.runtime_warnings
    );
    assert!(res.bridge_warnings.is_empty(), "{:?}", res.bridge_warnings);
    assert!(
        api.lrs.violations().is_empty(),
        "violations: {:?}",
        api.lrs.violations()
    );

    let verbs: Vec<&str> = api.lrs.statements().iter().map(verb_short).collect();
    assert!(
        verbs.contains(&"completed"),
        "expected completed: {verbs:?}"
    );
    assert!(
        !verbs.contains(&"passed"),
        "must not emit passed: {verbs:?}"
    );
    assert!(
        !verbs.contains(&"failed"),
        "must not emit failed: {verbs:?}"
    );
}
