//! M29 Step 11: End-to-end tests for `examples/rich-states/` across runtime,
//! XLIFF, all 6 publish targets via `strict-lms` + `StrictLtiPlatform`,
//! Web suspend/resume, and CLI validate/render.

use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use coursecraft_course::{CompletionStatus, Course, Severity};
use coursecraft_format::xliff::{apply_xliff, export_xliff, parse_xliff};
use coursecraft_lms::{
    CmiApi, LmsBridge, LrsApi, LrsMethod, LrsRequest, LrsResponse, LtiReporter, StartPlan, Tick,
    XapiBridge,
};
use coursecraft_logic::{Effect, Input, Player, decode_suspend, encode_suspend};
use coursecraft_lti::ToolConfig;
use coursecraft_lti_server::server::Server as LtiServer;
use coursecraft_lti_server::strict_platform::StrictLtiPlatform;
use coursecraft_publish::{
    PackageFile, PackageInput, PackageStandard, build_package_standard_with_folders,
};
use coursecraft_scorm::Standard;
use coursecraft_strict_lms::{
    Flavor as StrictFlavor, LaunchSpec, PackageStandardKind, Scorm2004Config, Scorm2004Edition,
    StrictLrs, StrictScorm12, StrictScorm2004, build_cmi5_launch_data, validate_package,
};
use coursecraft_xapi::{Flavor, LaunchParams};
use serde_json::json;

fn rich_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/rich-states")
}

fn load_rich_course() -> (Course, String) {
    let path = rich_dir().join("course.json");
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
        "examples/rich-states/course.json validation errors: {errors:?}"
    );
    (course, json)
}

fn load_inputs(filename: &str) -> Vec<Input> {
    let path = rich_dir().join(filename);
    let json =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&json).unwrap_or_else(|e| panic!("parse inputs {}: {e}", path.display()))
}

fn package_and_validate(
    course: &Course,
    course_json: &str,
    standard: PackageStandard,
    kind: PackageStandardKind,
) -> Vec<PackageFile> {
    let player_files = vec![PackageFile {
        path: "index.html".to_owned(),
        bytes: std::sync::Arc::from(b"<!doctype html><html><body>player</body></html>".as_slice()),
    }];
    let media_root = rich_dir().join("media");
    let media = |src: &str| -> Option<std::sync::Arc<[u8]>> {
        let rel = src.strip_prefix("media/").unwrap_or(src);
        std::fs::read(media_root.join(rel))
            .ok()
            .map(std::sync::Arc::from)
    };
    let web_folder = |_folder: &str| -> Option<Vec<(String, std::sync::Arc<[u8]>)>> { None };
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
        .map(|f| (f.path.as_str(), f.bytes.as_ref()))
        .collect();
    let violations = validate_package(kind, &refs);
    assert!(
        violations.is_empty(),
        "package {kind:?} manifest violations: {violations:?}"
    );
    files
}

fn run_inputs_collect_effects<'c>(
    course: &'c Course,
    inputs: &[Input],
) -> (Player<'c>, Vec<Effect>) {
    let mut player = Player::new(course).expect("player");
    let mut all_fx = Vec::new();
    for input in inputs {
        all_fx.extend(player.handle(input));
    }
    (player, all_fx)
}

fn has_passed(effects: &[Effect]) -> bool {
    effects.iter().any(|e| {
        matches!(
            e,
            Effect::Score {
                status: CompletionStatus::Passed,
                ..
            }
        )
    })
}

// ---------------------------------------------------------------------------
// E2E-01: Slide 1 — Expressive character (runtime state transition)
// ---------------------------------------------------------------------------

#[test]
fn e2e01_expressive_character_runtime_selected() {
    let (course, _) = load_rich_course();
    let mut player = Player::new(&course).expect("player");
    player.handle(&Input::Start);
    assert_eq!(
        player.current_slide().map(|s| s.id.as_str()),
        Some("s01-character")
    );
    player.handle(&Input::Click {
        object: "obj-character".into(),
    });
    assert_eq!(
        player.object_state("obj-character"),
        Some("selected"),
        "clicking character must transition it to selected"
    );
}

// ---------------------------------------------------------------------------
// E2E-02: Slide 2 — Visited checklist buttons & disabled unlock
// ---------------------------------------------------------------------------

#[test]
fn e2e02_visited_checklist_buttons_and_disabled_unlock() {
    let (course, _) = load_rich_course();
    let mut player = Player::new(&course).expect("player");
    player.handle(&Input::Start);
    player.handle(&Input::Click {
        object: "s01-btn-next".into(),
    });
    assert_eq!(
        player.current_slide().map(|s| s.id.as_str()),
        Some("s02-checklist")
    );
    assert_eq!(player.object_state("s02-btn-continue"), Some("disabled"));

    // Clicking disabled Continue does not advance slide
    player.handle(&Input::Click {
        object: "s02-btn-continue".into(),
    });
    assert_eq!(
        player.current_slide().map(|s| s.id.as_str()),
        Some("s02-checklist"),
        "disabled Continue button must ignore clicks"
    );

    // Click all 3 topic buttons -> each becomes visited, Continue unlocks to normal
    for id in ["s02-topic-1", "s02-topic-2", "s02-topic-3"] {
        player.handle(&Input::Click { object: id.into() });
        assert_eq!(player.object_state(id), Some("visited"));
    }
    assert_eq!(
        player.object_state("s02-btn-continue"),
        Some("normal"),
        "Continue button must unlock to normal once all 3 topics are visited"
    );

    // Now clicking Continue advances to Slide 3
    player.handle(&Input::Click {
        object: "s02-btn-continue".into(),
    });
    assert_eq!(
        player.current_slide().map(|s| s.id.as_str()),
        Some("s03-cards-keep")
    );
}

// ---------------------------------------------------------------------------
// E2E-03: Slide 3 — Interactive flip cards & revisit modes (KeepSaved vs Reset)
// ---------------------------------------------------------------------------

#[test]
fn e2e03_flip_cards_revisit_modes_and_suspend_resume() {
    let (course, _) = load_rich_course();
    let (mut player, _) = run_inputs_collect_effects(&course, &load_inputs("resume-1.inputs.json"));
    assert_eq!(
        player.current_slide().map(|s| s.id.as_str()),
        Some("s03-cards-keep")
    );
    assert_eq!(player.object_state("s03-card-a"), Some("selected"));

    // Advance to s03b-cards-reset, select card A, jump back to s03-cards-keep, then return to s03b
    player.handle(&Input::Click {
        object: "s03-btn-next".into(),
    });
    assert_eq!(
        player.current_slide().map(|s| s.id.as_str()),
        Some("s03b-cards-reset")
    );
    player.handle(&Input::Click {
        object: "s03b-card-a".into(),
    });
    assert_eq!(player.object_state("s03b-card-a"), Some("selected"));

    // Jump back to s03-cards-keep (KeepSaved -> s03-card-a stays selected)
    player.handle(&Input::Click {
        object: "s03b-btn-back".into(),
    });
    assert_eq!(
        player.current_slide().map(|s| s.id.as_str()),
        Some("s03-cards-keep")
    );
    assert_eq!(
        player.object_state("s03-card-a"),
        Some("selected"),
        "KeepSaved slide must preserve rich selected state on revisit"
    );

    // Return to s03b-cards-reset (Reset -> s03b-card-a resets to normal)
    player.handle(&Input::Click {
        object: "s03-btn-next".into(),
    });
    assert_eq!(
        player.current_slide().map(|s| s.id.as_str()),
        Some("s03b-cards-reset")
    );
    assert_eq!(
        player.object_state("s03b-card-a"),
        Some("normal"),
        "Reset slide must reset s03b-card-a to normal on revisit"
    );

    // Suspend & resume via cc1: from resume-1.inputs.json
    let (p1, _) = run_inputs_collect_effects(&course, &load_inputs("resume-1.inputs.json"));
    let cc1 = encode_suspend(&course, p1.state());
    assert!(cc1.starts_with("cc1:"));
    let decoded = decode_suspend(&course, &cc1).expect("decode cc1");
    let mut p2 = Player::resume(&course, decoded).expect("resume p2");
    p2.handle(&Input::Start);
    assert_eq!(
        p2.current_slide().map(|s| s.id.as_str()),
        Some("s03-cards-keep")
    );
    assert_eq!(
        p2.object_state("s03-card-a"),
        Some("selected"),
        "cc1 suspend/resume must preserve KeepSaved rich selected state"
    );
}

// ---------------------------------------------------------------------------
// E2E-04 & P-07: Custom freeform question + XLIFF round-trip
// ---------------------------------------------------------------------------

#[test]
fn e2e04_and_p07_custom_freeform_question_and_xliff_roundtrip() {
    let (course, _) = load_rich_course();

    // P-07: Grade in simulate (Player)
    let (player, fx) = run_inputs_collect_effects(&course, &load_inputs("pass.inputs.json"));
    assert_eq!(
        player.current_slide().map(|s| s.id.as_str()),
        Some("s05-done")
    );
    assert!(
        has_passed(&fx),
        "pass.inputs.json must pass the course: {fx:?}"
    );

    // E2E-04: XLIFF export -> translate normal, state text, extra text, extra alt -> import
    let xliff_xml = export_xliff(&course, "fr").expect("export_xliff");
    assert!(
        xliff_xml.contains("Click me!"),
        "XLIFF must include StateExtra text 'Click me!'"
    );
    assert!(
        xliff_xml.contains("Selected info badge"),
        "XLIFF must include StateExtra image alt 'Selected info badge'"
    );
    let translated_xml = xliff_xml
        .replace(
            "<source>Click me!</source>",
            "<source>Click me!</source>\n        <target>Cliquez-moi !</target>",
        )
        .replace(
            "<source>Selected info badge</source>",
            "<source>Selected info badge</source>\n        <target>Badge info sélectionné</target>",
        )
        .replace(
            "<source>Chosen</source>",
            "<source>Chosen</source>\n        <target>Choisi</target>",
        );
    let doc = parse_xliff(translated_xml.as_bytes()).expect("parse translated XLIFF");
    let (translated_course, report) = apply_xliff(&course, &doc);
    assert!(
        report.unknown_ids.is_empty()
            && report.source_changed.is_empty()
            && report.placeholder_mismatch.is_empty(),
        "XLIFF import issues: unknown={:?} changed={:?} placeholder={:?}",
        report.unknown_ids,
        report.source_changed,
        report.placeholder_mismatch
    );
    let tr_json = serde_json::to_string(&translated_course).expect("serialize translated");
    assert!(tr_json.contains("Cliquez-moi !"));
    assert!(tr_json.contains("Badge info sélectionné"));
    assert!(tr_json.contains("Choisi"));
}

// ---------------------------------------------------------------------------
// Strict-LMS Adapters (shared pattern from m27_reference_course.rs)
// ---------------------------------------------------------------------------

struct Strict12Adapter {
    inner: StrictScorm12,
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
}

impl CmiApi for Strict2004Adapter {
    fn initialize(&mut self) -> bool {
        self.inner.initialize("") == "true"
    }
    fn get_value(&mut self, key: &str) -> String {
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

fn start_scorm_session<'a, A: CmiApi>(
    course: &'a Course,
    standard: Standard,
    api: &mut A,
) -> (Player<'a>, LmsBridge, StartPlan, Vec<String>) {
    let mut bridge = LmsBridge::for_standard(standard);
    let (plan, warnings) = bridge.start(api, course);
    let player = match plan.resume.clone() {
        Some(state) => {
            let mut p = Player::resume(course, state).expect("resume player");
            p.handle(&Input::Start);
            p
        }
        None => Player::with_draw_seed(course, 42).expect("fresh player"),
    };
    (player, bridge, plan, warnings)
}

fn drive_scorm_inputs<A: CmiApi>(
    player: &mut Player<'_>,
    bridge: &mut LmsBridge,
    api: &mut A,
    course: &Course,
    inputs: &[Input],
    start_ms: u64,
) -> (Vec<Effect>, u64) {
    let mut effects = Vec::new();
    let mut session_ms = start_ms;
    for input in inputs {
        if let Input::Tick { ms } = input {
            session_ms = session_ms.saturating_add(u64::from(*ms));
        } else {
            session_ms = session_ms.saturating_add(250);
        }
        let step_effects = player.handle(input);
        let bw = bridge.after_input(api, course, player.state(), &step_effects, session_ms);
        assert!(bw.is_empty(), "bridge warnings: {bw:?}");
        effects.extend(step_effects);
    }
    (effects, session_ms)
}

// ---------------------------------------------------------------------------
// E2E-05 & PUB-05: SCORM 1.2 + SCORM 2004 (3rd & 4th Ed.) via strict-lms
// ---------------------------------------------------------------------------

#[test]
fn e2e05_and_pub05_scorm12_and_scorm2004_strict_lms() {
    let (course, course_json) = load_rich_course();
    let pass_inputs = load_inputs("pass.inputs.json");
    let r1_inputs = load_inputs("resume-1.inputs.json");
    let r2_inputs = load_inputs("resume-2.inputs.json");

    // SCORM 1.2: validate package + full pass + mid-course suspend/resume
    package_and_validate(
        &course,
        &course_json,
        PackageStandard::Scorm(Standard::Scorm12),
        PackageStandardKind::Scorm12,
    );
    let mut api12 = Strict12Adapter {
        inner: StrictScorm12::new(),
    };
    let (mut p12, mut b12, _, w12) = start_scorm_session(&course, Standard::Scorm12, &mut api12);
    assert!(w12.is_empty());
    let (fx12, ms12) = drive_scorm_inputs(&mut p12, &mut b12, &mut api12, &course, &pass_inputs, 0);
    let fw12 = b12.finish_session(&mut api12, ms12);
    assert!(fw12.is_empty(), "finish warnings: {fw12:?}");
    assert!(has_passed(&fx12));
    assert!(
        api12.inner.violations().is_empty(),
        "SCORM 1.2 pass violations: {:?}",
        api12.inner.violations()
    );

    // SCORM 1.2 suspend -> resume
    let mut api12_r1 = Strict12Adapter {
        inner: StrictScorm12::new(),
    };
    let (mut p12_r1, mut b12_r1, _, _) =
        start_scorm_session(&course, Standard::Scorm12, &mut api12_r1);
    let (_, ms_r1) = drive_scorm_inputs(
        &mut p12_r1,
        &mut b12_r1,
        &mut api12_r1,
        &course,
        &r1_inputs,
        0,
    );
    let _ = b12_r1.finish_session(&mut api12_r1, ms_r1);
    assert!(api12_r1.inner.violations().is_empty());

    api12_r1.inner.relaunch();
    let mut api12_r2 = api12_r1;
    let (mut p12_r2, mut b12_r2, plan_r2, _) =
        start_scorm_session(&course, Standard::Scorm12, &mut api12_r2);
    assert!(plan_r2.resume.is_some(), "SCORM 1.2 relaunch must resume");
    let (fx12_r2, ms_r2) = drive_scorm_inputs(
        &mut p12_r2,
        &mut b12_r2,
        &mut api12_r2,
        &course,
        &r2_inputs,
        0,
    );
    let _ = b12_r2.finish_session(&mut api12_r2, ms_r2);
    assert!(has_passed(&fx12_r2));
    assert!(
        api12_r2.inner.violations().is_empty(),
        "SCORM 1.2 resume violations: {:?}",
        api12_r2.inner.violations()
    );

    // SCORM 2004 (3rd & 4th Edition)
    package_and_validate(
        &course,
        &course_json,
        PackageStandard::Scorm(Standard::Scorm2004),
        PackageStandardKind::Scorm2004,
    );
    for edition in [Scorm2004Edition::Third, Scorm2004Edition::Fourth] {
        let cfg = Scorm2004Config {
            edition,
            ..Scorm2004Config::default()
        };
        let mut api2004 = Strict2004Adapter {
            inner: StrictScorm2004::with_config(cfg.clone()),
        };
        let (mut p2004, mut b2004, _, _) =
            start_scorm_session(&course, Standard::Scorm2004, &mut api2004);
        let (fx2004, ms2004) = drive_scorm_inputs(
            &mut p2004,
            &mut b2004,
            &mut api2004,
            &course,
            &pass_inputs,
            0,
        );
        let _ = b2004.finish_session(&mut api2004, ms2004);
        assert!(has_passed(&fx2004));
        assert!(
            api2004.inner.violations().is_empty(),
            "SCORM 2004 {edition:?} pass violations: {:?}",
            api2004.inner.violations()
        );

        let mut api2004_r1 = Strict2004Adapter {
            inner: StrictScorm2004::with_config(cfg),
        };
        let (mut p2004_r1, mut b2004_r1, _, _) =
            start_scorm_session(&course, Standard::Scorm2004, &mut api2004_r1);
        let (_, ms2004_r1) = drive_scorm_inputs(
            &mut p2004_r1,
            &mut b2004_r1,
            &mut api2004_r1,
            &course,
            &r1_inputs,
            0,
        );
        let _ = b2004_r1.finish_session(&mut api2004_r1, ms2004_r1);
        assert!(api2004_r1.inner.violations().is_empty());

        api2004_r1.inner.relaunch();
        let mut api2004_r2 = api2004_r1;
        let (mut p2004_r2, mut b2004_r2, plan2004_r2, _) =
            start_scorm_session(&course, Standard::Scorm2004, &mut api2004_r2);
        assert!(plan2004_r2.resume.is_some());
        let (fx2004_r2, ms2004_r2) = drive_scorm_inputs(
            &mut p2004_r2,
            &mut b2004_r2,
            &mut api2004_r2,
            &course,
            &r2_inputs,
            0,
        );
        let _ = b2004_r2.finish_session(&mut api2004_r2, ms2004_r2);
        assert!(has_passed(&fx2004_r2));
        assert!(
            api2004_r2.inner.violations().is_empty(),
            "SCORM 2004 {edition:?} resume violations: {:?}",
            api2004_r2.inner.violations()
        );
    }
}

// ---------------------------------------------------------------------------
// E2E-05 & PUB-06: cmi5 & xAPI via StrictLrs
// ---------------------------------------------------------------------------

struct StrictLrsAdapter {
    lrs: StrictLrs,
    inbox: VecDeque<LrsResponse>,
    uuid_counter: u64,
}

impl StrictLrsAdapter {
    fn new(lrs: StrictLrs) -> Self {
        Self {
            lrs,
            inbox: VecDeque::new(),
            uuid_counter: 0,
        }
    }
}

impl LrsApi for StrictLrsAdapter {
    fn send(&mut self, request: LrsRequest) {
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
            self.inbox.push_back(LrsResponse {
                id: request.id,
                status: resp.status,
                body: String::from_utf8_lossy(&resp.body).into_owned(),
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

fn pump_xapi(
    bridge: &mut XapiBridge,
    api: &mut StrictLrsAdapter,
    course: &Course,
    now_ms: &mut u64,
) {
    for _ in 0..20 {
        if !bridge.is_busy() {
            break;
        }
        *now_ms += 1000;
        let tick = bridge.tick(api, course, *now_ms);
        assert!(
            tick.warnings.is_empty(),
            "xAPI tick warnings: {:?}",
            tick.warnings
        );
    }
}

fn start_xapi_session<'a>(
    course: &'a Course,
    params: LaunchParams,
    flavor: Flavor,
    api: &mut StrictLrsAdapter,
    now_ms: &mut u64,
) -> (Player<'a>, XapiBridge, Tick) {
    let mut bridge = XapiBridge::new(params, flavor, course);
    bridge.start(api);
    let mut plan_tick = None;
    for _ in 0..10 {
        *now_ms += 100;
        let tick = bridge.tick(api, course, *now_ms);
        assert!(
            tick.warnings.is_empty(),
            "xAPI start warnings: {:?}",
            tick.warnings
        );
        if tick.plan.is_some() {
            plan_tick = Some(tick);
            break;
        }
    }
    let tick = plan_tick.expect("xapi handshake plan");
    let player = match tick.plan.as_ref().and_then(|p| p.resume.clone()) {
        Some(state) => {
            let mut p = Player::resume(course, state).expect("resume player");
            p.handle(&Input::Start);
            p
        }
        None => Player::with_draw_seed(course, 42).expect("fresh player"),
    };
    (player, bridge, tick)
}

fn drive_xapi_inputs(
    player: &mut Player<'_>,
    bridge: &mut XapiBridge,
    api: &mut StrictLrsAdapter,
    course: &Course,
    inputs: &[Input],
    now_ms: &mut u64,
) -> Vec<Effect> {
    let mut effects = Vec::new();
    let mut session_ms = 0u64;
    for input in inputs {
        if let Input::Tick { ms } = input {
            session_ms = session_ms.saturating_add(u64::from(*ms));
            *now_ms = now_ms.saturating_add(u64::from(*ms));
        } else {
            session_ms = session_ms.saturating_add(250);
            *now_ms = now_ms.saturating_add(250);
        }
        let step_effects = player.handle(input);
        let bw = bridge.after_input(
            api,
            course,
            player.state(),
            &step_effects,
            session_ms,
            *now_ms,
        );
        assert!(bw.is_empty(), "xAPI bridge warnings: {bw:?}");
        effects.extend(step_effects);
        pump_xapi(bridge, api, course, now_ms);
    }
    effects
}

#[test]
fn e2e05_and_pub06_cmi5_and_xapi_strict_lrs() {
    let (course, course_json) = load_rich_course();
    let pass_inputs = load_inputs("pass.inputs.json");
    let r1_inputs = load_inputs("resume-1.inputs.json");
    let r2_inputs = load_inputs("resume-2.inputs.json");

    // cmi5
    package_and_validate(
        &course,
        &course_json,
        PackageStandard::Xapi(Flavor::Cmi5),
        PackageStandardKind::Cmi5,
    );
    let mut cmi5_lrs = StrictLrs::new();
    let act_id = "https://coursecraft.example/rich-states";
    let act_enc = "https%3A%2F%2Fcoursecraft.example%2Frich-states";
    let actor_enc = "%7B%22objectType%22%3A%22Agent%22%2C%22account%22%3A%7B%22homePage%22%3A%22https%3A%2F%2Flms.example%22%2C%22name%22%3A%22learner%22%7D%2C%22name%22%3A%22Learner%22%7D";
    let actor_json = json!({
        "objectType": "Agent",
        "account": { "homePage": "https://lms.example", "name": "learner" },
        "name": "Learner"
    });
    let registration = "11111111-1111-4111-8111-111111111111";
    let launch_doc = build_cmi5_launch_data(act_id, "sess-1", "Normal", Some(0.8), None);
    cmi5_lrs.register_launch(
        LaunchSpec {
            flavor: StrictFlavor::Cmi5,
            activity_id: act_id.to_owned(),
            actor: actor_json.clone(),
            registration: Some(registration.to_owned()),
            launch_mode: "Normal".to_owned(),
            mastery_score: Some(0.8),
            session_id: "sess-1".to_owned(),
            grouping_ids: vec![act_id.to_owned()],
            fetch_id: Some("f1".to_owned()),
            auth_token: Some("tok-1".to_owned()),
        },
        Some(launch_doc),
    );
    let cmi5_query = format!(
        "endpoint=https://lrs.example/xapi&fetch=https://lrs.example/fetch/f1&actor={actor_enc}&registration={registration}&activityId={act_enc}"
    );
    let cmi5_params = LaunchParams::parse(&cmi5_query, Flavor::Cmi5).expect("cmi5 params");
    let mut cmi5_api = StrictLrsAdapter::new(cmi5_lrs);
    let mut now = 1000u64;
    let (mut p_cmi5, mut b_cmi5, _) =
        start_xapi_session(&course, cmi5_params, Flavor::Cmi5, &mut cmi5_api, &mut now);
    let fx_cmi5 = drive_xapi_inputs(
        &mut p_cmi5,
        &mut b_cmi5,
        &mut cmi5_api,
        &course,
        &pass_inputs,
        &mut now,
    );
    let fw_cmi5 = b_cmi5.finish_session(&mut cmi5_api, &course, Some(p_cmi5.state()), now);
    assert!(fw_cmi5.is_empty(), "cmi5 finish warnings: {fw_cmi5:?}");
    pump_xapi(&mut b_cmi5, &mut cmi5_api, &course, &mut now);
    assert!(has_passed(&fx_cmi5));
    assert!(
        cmi5_api.lrs.violations().is_empty(),
        "cmi5 violations: {:?}",
        cmi5_api.lrs.violations()
    );

    // xAPI + resume
    package_and_validate(
        &course,
        &course_json,
        PackageStandard::Xapi(Flavor::Xapi),
        PackageStandardKind::Xapi,
    );
    let mut xapi_lrs = StrictLrs::new();
    let x_reg = "33333333-3333-4333-8333-333333333333";
    xapi_lrs.register_launch(
        LaunchSpec {
            flavor: StrictFlavor::Xapi,
            activity_id: act_id.to_owned(),
            actor: actor_json.clone(),
            registration: Some(x_reg.to_owned()),
            launch_mode: "Normal".to_owned(),
            mastery_score: None,
            session_id: "xsess-1".to_owned(),
            grouping_ids: Vec::new(),
            fetch_id: None,
            auth_token: Some("Basic dGVzdDp0ZXN0".to_owned()),
        },
        None,
    );
    let xapi_query = format!(
        "endpoint=https://lrs.example/xapi&auth=Basic%20dGVzdDp0ZXN0&actor={actor_enc}&activityId={act_enc}&registration={x_reg}"
    );
    let xapi_params = LaunchParams::parse(&xapi_query, Flavor::Xapi).expect("xapi params");
    let mut xapi_api = StrictLrsAdapter::new(xapi_lrs);
    let mut now_x = 1000u64;
    let (mut p_x1, mut b_x1, _) = start_xapi_session(
        &course,
        xapi_params.clone(),
        Flavor::Xapi,
        &mut xapi_api,
        &mut now_x,
    );
    drive_xapi_inputs(
        &mut p_x1,
        &mut b_x1,
        &mut xapi_api,
        &course,
        &r1_inputs,
        &mut now_x,
    );
    let fw_x1 = b_x1.finish_session(&mut xapi_api, &course, Some(p_x1.state()), now_x);
    assert!(fw_x1.is_empty(), "xAPI r1 finish warnings: {fw_x1:?}");
    pump_xapi(&mut b_x1, &mut xapi_api, &course, &mut now_x);
    assert!(xapi_api.lrs.violations().is_empty());

    xapi_api.lrs.register_launch(
        LaunchSpec {
            flavor: StrictFlavor::Xapi,
            activity_id: act_id.to_owned(),
            actor: actor_json,
            registration: Some(x_reg.to_owned()),
            launch_mode: "Normal".to_owned(),
            mastery_score: None,
            session_id: "xsess-2".to_owned(),
            grouping_ids: Vec::new(),
            fetch_id: None,
            auth_token: Some("Basic dGVzdDp0ZXN0".to_owned()),
        },
        None,
    );
    let (mut p_x2, mut b_x2, tick_x2) = start_xapi_session(
        &course,
        xapi_params,
        Flavor::Xapi,
        &mut xapi_api,
        &mut now_x,
    );
    assert!(
        tick_x2
            .plan
            .as_ref()
            .and_then(|p| p.resume.as_ref())
            .is_some(),
        "xAPI relaunch must resume"
    );
    let fx_x2 = drive_xapi_inputs(
        &mut p_x2,
        &mut b_x2,
        &mut xapi_api,
        &course,
        &r2_inputs,
        &mut now_x,
    );
    let fw_x2 = b_x2.finish_session(&mut xapi_api, &course, Some(p_x2.state()), now_x);
    assert!(fw_x2.is_empty(), "xAPI r2 finish warnings: {fw_x2:?}");
    pump_xapi(&mut b_x2, &mut xapi_api, &course, &mut now_x);
    assert!(has_passed(&fx_x2));
    assert!(
        xapi_api.lrs.violations().is_empty(),
        "xAPI resume violations: {:?}",
        xapi_api.lrs.violations()
    );
}

// ---------------------------------------------------------------------------
// E2E-05 & PUB-07: LTI 1.3 via StrictLtiPlatform
// ---------------------------------------------------------------------------

const LTI_CLIENT_ID: &str = "coursecraft-rich-states-client";
const LTI_DEPLOYMENT_ID: &str = "deployment-1";
const LTI_SCORE_SCOPE: &str = "https://purl.imsglobal.org/spec/lti-ags/scope/score";
const LTI_TEST_KEY: &[u8] = include_bytes!("../../coursecraft-lti/tests/fixtures/test_key.pk8");
static LTI_DIR_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn lti_temp_course_dir(files: &[PackageFile]) -> PathBuf {
    let n = LTI_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("coursecraft-m29-lti-{nanos}-{n}"));
    for f in files {
        let target = dir.join(&f.path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).expect("mkdir");
        }
        std::fs::write(&target, &f.bytes).expect("write lti file");
    }
    dir
}

fn lti_encode_component(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[test]
fn e2e05_and_pub07_lti_13_strict_platform() {
    let (course, course_json) = load_rich_course();
    let files = package_and_validate(
        &course,
        &course_json,
        PackageStandard::Lti,
        PackageStandardKind::Lti,
    );
    let course_dir = lti_temp_course_dir(&files);
    let platform = StrictLtiPlatform::start(LTI_CLIENT_ID).expect("start StrictLtiPlatform");
    let config = ToolConfig {
        issuer: platform.issuer(),
        client_id: LTI_CLIENT_ID.into(),
        deployment_id: LTI_DEPLOYMENT_ID.into(),
        auth_login_url: platform.auth_login_url(),
        auth_token_url: platform.auth_token_url(),
        jwks_url: platform.jwks_url(),
        tool_url: "https://tool.example".into(),
        private_key_pkcs8: LTI_TEST_KEY.to_vec(),
        key_id: "tool-key".into(),
    };
    let server =
        LtiServer::bind(config, course_dir.clone(), "127.0.0.1:0").expect("bind LtiServer");
    let server_addr = server.local_addr().expect("local_addr");
    std::thread::spawn(move || {
        let _ = server.run();
    });
    platform.set_tool_jwks_url(format!("http://{server_addr}/.well-known/jwks.json"));

    let agent = ureq::Agent::new_with_config(
        ureq::Agent::config_builder()
            .http_status_as_error(false)
            .max_redirects(0)
            .build(),
    );

    let target_link_uri = "https://tool.example/course/index.html";
    let login_url = format!(
        "http://{}/lti/login?iss={}&login_hint=user-1&target_link_uri={}&client_id={}",
        server_addr,
        lti_encode_component(&platform.issuer()),
        lti_encode_component(target_link_uri),
        lti_encode_component(LTI_CLIENT_ID),
    );
    let login_resp = agent.get(&login_url).call().expect("OIDC login");
    assert_eq!(login_resp.status(), 302);
    let location = login_resp
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .expect("location")
        .to_owned();
    let state_cookie = login_resp
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        .expect("state cookie")
        .to_owned();
    let oidc_resp = agent.get(&location).call().expect("OIDC auth");
    assert_eq!(oidc_resp.status(), 200);
    let params = platform.last_oidc_params().expect("oidc params");
    let state = params.get("state").expect("state").clone();
    let nonce = params.get("nonce").expect("nonce").clone();
    let lineitem = platform.lineitem_url("li-rich-states");
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(1_700_000_000);
    let claims = platform.default_claims(
        &nonce,
        now_secs,
        target_link_uri,
        Some(&lineitem),
        &[LTI_SCORE_SCOPE],
    );
    let id_token = platform.sign_id_token(&claims).expect("sign id_token");
    let launch_body = format!(
        "state={}&id_token={}",
        lti_encode_component(&state),
        lti_encode_component(&id_token)
    );
    let mut launch_resp = agent
        .post(&format!("http://{server_addr}/lti/launch"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Cookie", &state_cookie)
        .send(&launch_body)
        .expect("launch");
    let launch_status = launch_resp.status();
    if launch_status != 302 {
        let body = launch_resp.body_mut().read_to_string().unwrap_or_default();
        panic!("LTI launch failed with {launch_status}: {body}");
    }
    let session_cookie = launch_resp
        .headers()
        .get("set-cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        .expect("session cookie")
        .to_owned();

    // Drive course with pass.inputs.json and post score through LtiReporter
    let mut player = Player::new(&course).expect("player");
    let mut reporter = LtiReporter::new();
    for input in load_inputs("pass.inputs.json") {
        let fx = player.handle(&input);
        reporter.record(&fx);
    }
    let pending = reporter.take_pending().expect("pending LTI score");
    let score_resp = agent
        .post(&format!("http://{server_addr}/lti/score"))
        .header("Cookie", &session_cookie)
        .header("Content-Type", "application/json")
        .header("Origin", "https://tool.example")
        .send(&pending.json_body())
        .expect("post score");
    assert!(
        (200..300).contains(&score_resp.status().as_u16()),
        "unexpected /lti/score status: {}",
        score_resp.status()
    );
    let scores = platform.scores();
    assert_eq!(scores.len(), 1);
    assert_eq!(scores[0].payload["scoreGiven"].as_f64(), Some(100.0));
    assert!(
        platform.violations().is_empty(),
        "StrictLtiPlatform violations: {:?}",
        platform.violations()
    );
    let _ = std::fs::remove_dir_all(&course_dir);
}

// ---------------------------------------------------------------------------
// E2E-05 & PUB-08: Web target + Web suspend/resume
// ---------------------------------------------------------------------------

#[test]
fn e2e05_and_pub08_web_package_and_resume() {
    let (course, course_json) = load_rich_course();
    let _files = package_and_validate(
        &course,
        &course_json,
        PackageStandard::Web,
        PackageStandardKind::Web,
    );

    let key = format!("coursecraft:resume:{}", course.id);
    assert_eq!(key, "coursecraft:resume:rich-states");
    let mut storage: BTreeMap<String, String> = BTreeMap::new();

    let (p1, _) = run_inputs_collect_effects(&course, &load_inputs("resume-1.inputs.json"));
    let saved = encode_suspend(&course, p1.state());
    assert!(saved.starts_with("cc1:"));
    storage.insert(key.clone(), saved);

    let raw = storage.get(&key).expect("stored cc1");
    let decoded = decode_suspend(&course, raw).expect("decode cc1");
    let mut p2 = Player::resume(&course, decoded).expect("resume p2");
    p2.handle(&Input::Start);
    assert_eq!(
        p2.current_slide().map(|s| s.id.as_str()),
        Some("s03-cards-keep")
    );
    assert_eq!(p2.object_state("s03-card-a"), Some("selected"));

    let mut all_fx = Vec::new();
    for input in load_inputs("resume-2.inputs.json") {
        let fx = p2.handle(&input);
        assert!(
            !fx.iter().any(|e| matches!(e, Effect::Warning { .. })),
            "unexpected warning on web resume: {fx:?}"
        );
        all_fx.extend(fx);
    }
    assert_eq!(p2.current_slide().map(|s| s.id.as_str()), Some("s05-done"));
    assert!(has_passed(&all_fx));
}

// ---------------------------------------------------------------------------
// E2E-05: CLI validate + render every slide
// ---------------------------------------------------------------------------

#[test]
fn e2e05_cli_validate_and_render_all_slides() {
    let course_path = rich_dir().join("course.json");
    let val_out = std::process::Command::new(env!("CARGO_BIN_EXE_coursecraft-cli"))
        .args(["validate", &course_path.to_string_lossy()])
        .output()
        .expect("run validate");
    assert!(
        val_out.status.success(),
        "coursecraft-cli validate failed: {}",
        String::from_utf8_lossy(&val_out.stderr)
    );

    let tmp_dir = std::env::temp_dir().join("coursecraft-m29-cli-render");
    let _ = std::fs::create_dir_all(&tmp_dir);
    for slide_id in [
        "s01-character",
        "s02-checklist",
        "s03-cards-keep",
        "s03b-cards-reset",
        "s04-question",
        "s05-done",
    ] {
        let out_png = tmp_dir.join(format!("{slide_id}.png"));
        let render_out = std::process::Command::new(env!("CARGO_BIN_EXE_coursecraft-cli"))
            .args([
                "render",
                "--slide",
                slide_id,
                "--out",
                &out_png.to_string_lossy(),
                &course_path.to_string_lossy(),
            ])
            .output()
            .expect("run render");
        assert!(
            render_out.status.success(),
            "coursecraft-cli render {slide_id} failed: {}",
            String::from_utf8_lossy(&render_out.stderr)
        );
        let meta = std::fs::metadata(&out_png).expect("rendered png exists");
        assert!(meta.len() > 100, "rendered png {slide_id} is non-empty");
    }
    let _ = std::fs::remove_dir_all(&tmp_dir);
}
