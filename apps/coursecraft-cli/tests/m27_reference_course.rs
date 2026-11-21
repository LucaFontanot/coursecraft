//! M27 Part B: Automated multi-standard runs on `examples/reference-course/`.
//!
//! Step B-3: Shared runner harness + SCORM 1.2 (B1.1–B1.8) + SCORM 2004 (B2.1–B2.8).

use std::path::{Path, PathBuf};

use coursecraft_course::{CompletionStatus, Course, Severity, Value};
use coursecraft_lms::{CmiApi, LmsBridge, StartPlan};
use coursecraft_logic::{Effect, Input, Player};
use coursecraft_publish::{
    PackageFile, PackageInput, PackageStandard, build_package_standard_with_folders,
};
use coursecraft_scorm::Standard;
use coursecraft_strict_lms::{
    PackageStandardKind, Scorm12Config, Scorm2004Config, Scorm2004Edition, StrictScorm12,
    StrictScorm2004, validate_package,
};

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
