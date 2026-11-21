# M27 Master Test & Quality Plan

**Role:** QA Plan (specification for implementation — zero source code edited here)  
**Target release bar:** Zero bugs a real learner, course author, or LMS administrator could hit across all six publish targets (SCORM 1.2, SCORM 2004 3rd/4th Ed., cmi5, xAPI, LTI 1.3, Web), PowerPoint import round 3, and every core editor surface.

> **Implementer Constraints (from `AGENTS.md` and `xtask/src/main.rs`)**
> 1. **Never crash:** No `unwrap`/`expect`/`panic!`/`todo!`/`unreachable!` in non-test code; `unsafe_code = "forbid"` everywhere outside `coursecraft-video-native`.
> 2. **Layering & WASM:** Every new workspace library crate MUST be registered in `LAYERS` (and `WASM_CRATES` if L0–L5) in `xtask/src/main.rs:28-147`. `cargo xtask ci` (`fmt`, `clippy -D warnings`, `test --workspace`, `layers`, `wasm`) must pass after every step.
> 3. **Spec first:** Any runtime or LMS bridge contract change updates `docs/spec/triggers.md` or `docs/spec/course-model.md` in the same step.
> 4. **Zero network, zero installs:** Every automated test in Parts A–D runs purely locally (in-memory or `127.0.0.1:0` loopback), with no Docker, PHP, Moodle, or internet access.
> 5. **Artifact hygiene:** All test-generated zips, PNGs, and `.ccraft` files go under `target/` or `std::env::temp_dir()`, never into tracked directories.

---

## Executive Summary & Coverage Index

| Part | Scope | Feature Areas | Concrete Cases | Primary Surfaces | Sessions |
|---|---|---:|---:|---|---:|
| **Part A** | Strict Test LMS (single source of rules, offline) | 5 | **56** | `crates/strict-lms` (new), `crates/lms`, `crates/xapi`, `crates/lti`, `apps/coursecraft-player`, `apps/coursecraft-lti`, `xtask/mock-lms`, `xtask/src/mock_lrs.rs` | 6 |
| **Part B** | Reference Course (`examples/reference-course/`) & Automated Runs | 5 | **42** | `examples/reference-course/`, `crates/logic`, `crates/lms`, `crates/publish`, `apps/coursecraft-cli`, `apps/coursecraft-lti` | 5 |
| **Part C** | PowerPoint Import Round 3 (Animations + Style-Less Tables) | 2 | **24** | `crates/pptx`, `crates/pptx/tests/fidelity/`, `apps/coursecraft-cli/tests/pptx_fidelity.rs` | 5 |
| **Part D** | Core Editor Sweep (`TestHarness` + Desktop `EditorApp`) | 13 | **142** | `crates/ui-egui/src/test_harness.rs`, `crates/ui-egui/src/*`, `crates/engine`, `apps/coursecraft` | 9 |
| **Total** | | **25** | **264** | | **25** |

---

## Pre-Audit Defect Watchlist (Confirmed in Source Code Before Writing Tests)

Thorough inspection of the current codebase uncovered **27 concrete latent bugs or spec violations**. The planned tests below intentionally expose every single one of them; the implementer should expect these tests to fail first (red) and fix each root cause cleanly (green):

| ID | Part | File & Lines | Defect / Spec Violation |
|---|---|---|---|
| **BUG-A1** | A | `crates/lms/src/interaction.rs:175`, `crates/lms/src/bridge.rs:274` | `QuestionKind::Freeform` emits `cmi.interactions.n.type = "other"`, which is **illegal in SCORM 1.2** (SCORM 1.2 vocabulary stops at `true-false\|choice\|fill-in\|matching\|performance\|sequencing\|likert\|numeric`; `"other"` was added in SCORM 2004). Strict SCORM 1.2 LMSs return `405`. |
| **BUG-A2** | A | `crates/lms/src/time.rs:18-20` | `format_session_time` saturates at `"9999:99:99.99"`, which is invalid `CMITimespan` (`MM` and `SS` must be `00..=59`; valid max is `"9999:59:59.99"`). |
| **BUG-A3** | A | `crates/lms/src/bridge.rs:78` | SCORM 1.2 sets `location_limit: None`, allowing `cmi.core.lesson_location` writes > 255 chars (`CMIString255` violation -> `405`). |
| **BUG-A4** | A | `crates/lms/src/bridge.rs:265-270, 770` | `cmi.interactions.n.id` is not bounded to 255 (1.2) / 4000 (2004) chars, and `interaction_index` increments even when `.id` write returns `false`. Also `QuestionAnswered` (`:610-613`) sets `durable_changed = true` without `needs_commit = true`. |
| **BUG-A5** | A | `crates/lms/src/bridge.rs:517-522` | Resume high-water check parses `score.raw` with `.parse::<u8>()`, failing on valid LMS decimals like `"85.0"` or `"85.5"`, silently dropping `best_passed_score`. |
| **BUG-A6** | A | `crates/lms/src/bridge.rs:156-163` | `is_first_launch` does not handle empty status `""` returned by some LMSs on first launch, skipping initial `"incomplete"`. |
| **BUG-A7** | A | `crates/publish/src/lib.rs:515-516`, `crates/scorm/src/lib.rs:106-111` | `imsmanifest.xml` lists itself as `<file href="imsmanifest.xml"/>` inside `<resource>`. `valid_identifier` also allows leading digits/dots/hyphens (invalid XML `NCName`/`xs:ID`). |
| **BUG-A8** | A | `apps/coursecraft-player/src/app.rs:452-539` | `Playing::handle_input` runs `RunJavaScript` (`:457`) and `media.apply` (`:504`) recursive followups **before** forwarding the outer `effects` to `lms::after_input`/`lrs::after_input`/`lti::after_input` (`:506-523`), reversing LMS effect order (and an inner `Exit` drops outer `Score`/`Completion`). |
| **BUG-A9** | A | `crates/lms/src/lrs.rs:324-328, 959-978` | `XapiBridge::after_input` omits `LightboxOpened`/`LightboxClosed` from `durable_changed` (unlike `LmsBridge`), and `send_keepalive_progress` has no `MAX_KEEPALIVE_BODY` guard. |
| **BUG-A10** | A | `crates/xapi/src/launch.rs:153-156`, `crates/lms/src/lrs.rs:482` | `normalize_endpoint` accepts hostless `"http://"` / `"https://"`; `handle_fetch` accepts empty `"auth-token": ""`. |
| **BUG-A11** | A | `crates/lti/src/lib.rs:287-292, 340-421`, `apps/coursecraft-lti/src/server.rs:651-666` | `lineitem_score_url` produces `//scores` when `lineitem` has a trailing `/`; `/lti/score` never verifies `launch.ags.scope` contains `SCORE_SCOPE`; `validate_id_token` omits `target_link_uri` and required `roles` validation; `apps/coursecraft-player/src/lti.rs` lacks `pagehide`/keepalive flush. |
| **BUG-A12** | A | `xtask/mock-lms/index.html:136-145, 217-271, 341-363` | Conflates SCORM 1.2 and 2004 error codes (returns 403/404/406 with 1.2 meanings in 2004), accepts `"timeout"` instead of `"time-out"` for `cmi.exit`, wrongly caps 2004 `cmi.score.raw/min/max` to `0..=100`, lets `cmi.interactions.n.*` be read in 1.2, and accepts arbitrary unknown keys/values in 1.2 `LMSSetValue`. |
| **BUG-A13** | A | `xtask/src/mock_lrs.rs:414, 592, 666-670, 762-784` | Compares State API `agent` by raw JSON string instead of IFI equivalence; does not forbid `passed` after `failed` in the same cmi5 session (cmi5 §9.3.1); collides plain xAPI sessions across relaunches on the same registration; returns `200` instead of `409` on duplicate statement `id` with different content. |
| **BUG-B1** | B | `crates/logic/src/lib.rs:524-565`, `crates/logic/src/run.rs:2233-2254, 2308-2310, 2368-2390` | Opening a lightbox clears `cue_progress`, `cue_held`, and `cue_layers` without saving/restoring them in `Lightbox`, resetting base video cues on lightbox close. |
| **BUG-B2** | B | `crates/logic/src/run.rs:353-363, 796-799` | `jump()` calls `self.close_lightbox(true, out)`, emitting a spurious `Media { Play }` for base media immediately before `enter_slide` emits `Media { Stop }`. |
| **BUG-B3** | B | `crates/publish/src/lib.rs:95-137` | `collect_object_media` omits `Video.poster` and `Path.fill_style.picture_src()`, unlike `Course::media_references()` (`crates/course/src/lib.rs:232-255`), so published packages miss those files. |
| **BUG-C1** | C | `crates/pptx/src/lib.rs:3174-3182, 3342-3351, 3416-3443` | Tables without `<a:tableStyleId>` (e.g., `fidelity.pptx` Slide 14 Table 2) produce `TableBorders { outer: None, inner: None }` instead of PowerPoint's default 1pt `#000000` grid; `table_side_outline` conflates missing `<a:ln*>` with explicit `<a:noFill/>`. |
| **BUG-C2** | C | `crates/pptx/src/lib.rs:1030-1031, 2762-2841` | `SlideTracker.pptx_ids` is `HashMap<String, String>` (loses multi-object shapes), misses `cxnSp`, and never maps `grpSp` IDs to flattened children, breaking `p:spTgt@spid` animation targeting. |
| **BUG-D1** | D | `crates/ui-egui/src/triggers.rs:1726-1731, 1917-1968` | Moving a trigger Up/Down while a filter or selected-object sort is active shifts raw `idx ± 1`, jumping across non-adjacent visual rows. |
| **BUG-D2** | D | `crates/ui-egui/src/states.rs:334-344` | Deleting the currently previewed state does not clear `editor.ui.preview_state`; deleting `initial_state` or trigger-referenced state fails in `keep_valid` with no States-panel hint. |
| **BUG-D3** | D | `crates/ui-egui/src/lib.rs:4390-4417`, `crates/ui-egui/src/media_library.rs:517-537`, `lib.rs:2772-2804` | `web_folder_prefixes` omits `course.banks`; `insert_on_slide` falsely reports `"File operations are not undoable"` even on failure; `go_to_place` ignores object ID and leaves `story_view = true`; `rename_font_family` uses unbatched `self.run` calls. |
| **BUG-D4** | D | `crates/ui-egui/src/history.rs:99-107` | Clicking a History panel step does not set `editor.ui.pending_undo_redo = true`, failing to reselect restored slides on undo of `slide.delete`. |
| **BUG-D5** | D | `crates/ui-egui/src/clipboard.rs:137-138, 495-534` | `event_object_ref` ignores `target`/`b` on `DroppedOn`/`ObjectsIntersect`, copying dangling triggers when only one object is copied; pasting on a different slide still increments `clipboard_paste_count`. |
| **BUG-D6** | D | `crates/engine/src/commands.rs:3772-3776` | `layer_delete` only cleans triggers mentioning the layer ID, not base-slide triggers referencing objects living on the deleted layer. |
| **BUG-D7** | D | `crates/ui-egui/src/course_map.rs:370-400, 778-835`, `crates/engine/src/commands.rs:2290-2308` | Story View click doesn't clear `also_slides`; `handle_map_keys` doesn't guard against open Preview/modals; `scene.delete` doesn't strip cross-scene `JumpToSlide`/`LightboxSlide` triggers. |
| **BUG-D8** | D | `crates/ui-egui/src/find.rs:243, 375-417`, `crates/ui-egui/src/status.rs:12-146, 367-396` | `Find::go_to` doesn't exit Story View; `can_replace` doesn't cap `replacement.len()`; `status.rs` finding navigation doesn't exit Master View or Story View, ignores `/banks/...` paths, and ignores course-level findings (`slide: None`). |
| **BUG-D9** | D | `apps/coursecraft/src/main.rs:793-826, 862, 928, 933` | `finish_open`/`OpenExample`/`OpenTemplate` reset `Autosaver::new()` without preserving `prefs.autosave_minutes_clamped()`; `perform_save_as`/`perform_save_as_folder` mutate `self.project.path`/`folder`/`disk_stamp` before checking whether `perform_save()` succeeded. |

---

# Part A: Strict Test LMS (No Installs, No Network)

## A. Architecture: One Source of Truth Across Native Tests, Browser Mock LMS, and Mock LRS

### Where It Lives (Layering & WASM Compliant)

Create a new pure library crate **`crates/strict-lms`** (`coursecraft-strict-lms`):
- **Layer registration (`xtask/src/main.rs:28-49`):** Register `("strict-lms", 1, false)` (depends only on L0 `coursecraft-scorm`, `coursecraft-xapi`, `serde`, `serde_json`, `quick-xml` — zero filesystem, zero sockets, zero `std::time`, zero `ring`, zero `unsafe`).
- **WASM registration (`xtask/src/main.rs:130-147`):** Add `"coursecraft-strict-lms"` to `WASM_CRATES`.
- **Consumers (zero duplicated rules):**
  1. **Native automated tests** in `crates/lms` (via `[dev-dependencies]` — explicitly exempted by `check_layers` at `xtask/src/main.rs:93`), `apps/coursecraft-player` (via a new native seam below), and `apps/coursecraft-cli/tests/`: wrap `StrictScorm12` / `StrictScorm2004` in a ~30-line `impl CmiApi` adapter and `StrictLrs` in a ~30-line `impl LrsApi` adapter.
  2. **Browser manual testing (`xtask/mock-lms/index.html`):** Add a tiny `wasm-bindgen` cdylib crate `xtask/strict-lms-wasm` (exempt in `xtask/src/main.rs:52-59`, or `crates/strict-lms` optional `wasm-api` feature) compiled by `cargo xtask package-example` into `target/mock-lms/strict-lms-wasm/`. `xtask/mock-lms/index.html` becomes a thin UI shell whose `window.API` and `window.API_1484_11` methods call synchronously into the compiled Rust `StrictScorm12` / `StrictScorm2004` engine and render `engine.violations()` in a live red badge + Violations tab beside the call log. **Not one SCORM validation rule is hand-written in JavaScript.**
  3. **`xtask/src/mock_lrs.rs`:** Replace `LrsStore`'s hand-rolled `validate*` functions (`xtask/src/mock_lrs.rs:625-830`) and state map (`:578-623`) with a direct `coursecraft_strict_lms::StrictLrs` field. HTTP parsing/static serving stays in `mock_lrs.rs`; every xAPI/cmi5 rule comes from `strict-lms`.
  4. **LTI 1.3 Strict Mock Platform:** Because `coursecraft-lti` uses `ring` (non-wasm), place `StrictLtiPlatform` in `apps/coursecraft-lti/src/strict_platform.rs` (compiled under `#[cfg(test)]` or as a `pub mod strict_platform` in `apps/coursecraft-lti/src/lib.rs`), speaking real HTTP over `127.0.0.1:0` against `coursecraft_lti_server::Server` and recording structured `Vec<Violation>` for every OIDC login, JWKS fetch, token assertion, and AGS score POST.

### Native Seam in `apps/coursecraft-player` (Closing the `wasm32`-Only Gap)

Today `apps/coursecraft-player/Cargo.toml:25-27` gates `coursecraft-lms` and `coursecraft-xapi` behind `[target.'cfg(target_arch = "wasm32")'.dependencies]`, and `Playing::handle_input` (`apps/coursecraft-player/src/app.rs:452-539`) only compiles LMS/LRS/LTI forwarding on `wasm32`. To test the **player's real LMS wiring natively without a browser**:
1. Move `coursecraft-lms` and `coursecraft-xapi` to normal `[dependencies]` in `apps/coursecraft-player/Cargo.toml` (both are pure Rust and already compile natively).
2. Extract a pure, non-wasm `pub mod session` (`apps/coursecraft-player/src/session.rs`) holding `ReportingHost` (enum over `Scorm(LmsBridge, Box<dyn CmiApi>)`, `Xapi(XapiBridge, Box<dyn LrsApi>)`, `Lti(LtiReporter, Box<dyn LtiTransport>)`, `Web(WebResume<Box<dyn SuspendStorage>>)`, `Standalone`) and the exact effect-forwarding + `RunJavaScript` / media-followup sequencing loop used by `Playing::handle_input`.
3. Fix **BUG-A8** in that shared loop so outer `effects` are forwarded to `ReportingHost` **before** recursive followup inputs (`Input::SetVariable`, `Input::Jump`, `Input::MediaEnded`), and share `coursecraft_lms::reporting_from_marker` (extended with `Lti` and `Web` variants in `crates/lms/src/marker.rs`) so `apps/coursecraft-player/src/load.rs:18-76` stops duplicating package-marker parsing.

### Unified Violation Report Shape (`crates/strict-lms/src/lib.rs`)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Violation {
    pub standard: &'static str,   // "scorm12" | "scorm2004" | "cmi5" | "xapi" | "lti13" | "manifest"
    pub rule: &'static str,       // stable ID, e.g. "SCORM12-405-TIMESPAN", "CMI5-9.3.1-PASS-AFTER-FAIL"
    pub call_index: usize,        // 1-based API call or HTTP request index
    pub element: String,          // CMI key, xAPI path, header, or XML XPath
    pub message: String,          // concrete actual vs expected
    pub error_code: Option<String>, // exact SCORM error code ("405", "408", ...) or HTTP status
}
```
Every strict engine exposes `pub fn violations(&self) -> &[Violation]`, `pub fn calls(&self) -> &[CallRecord]`, `pub fn snapshot(&self) -> Value`, and `pub fn relaunch(&mut self)` (simulating browser close + new LMS launch using persisted attempt data and honoring `exit == "suspend"` vs non-suspend attempt reset!).

---

## A. Complete Specification Rule Matrix (Enforced by `crates/strict-lms`)

### 1. SCORM 1.2 RTE (`crates/strict-lms/src/scorm12.rs`)
- **Lifecycle & Call Order:**
  - States: `NotInitialized -> Running -> Finished`.
  - `LMSInitialize(arg)`: requires `arg == ""` (`201` otherwise). Fails with `101` if `Running` or `Finished` (relaunch requires `relaunch()`).
  - `LMSGetValue`, `LMSSetValue`, `LMSCommit`, `LMSFinish`: fail with `301` ("Not initialized") if `NotInitialized` or `Finished`.
  - `LMSCommit(arg)`, `LMSFinish(arg)`: require `arg == ""` (`201`).
  - `LMSGetLastError()`, `LMSGetErrorString(code)`, `LMSGetDiagnostic(code)`: legal in all states; exact SCORM 1.2 error codes ONLY: `0, 101, 201, 202, 203, 301, 401, 402, 403, 404, 405` (never 2004 codes `406`–`408`!).
  - Unclean close detection: `assert_clean_finish()` flags violation if session ended while `Running` without `LMSFinish("")`.
- **Data Model Elements, Access, Types & Limits (SCORM 1.2 RTE §3.4):**
  - `_children` / `_count` / `_version`: Read-only; `LMSSetValue` returns `402` ("Invalid set value, element is a keyword"). Supported keywords: `cmi.core._children`, `cmi.core.score._children`, `cmi.interactions._children`, `cmi.interactions._count`, `cmi.interactions.n.objectives._count`, `cmi.interactions.n.correct_responses._count`, `cmi.objectives._children`, `cmi.objectives._count`, `cmi.student_data._children`, `cmi.student_preference._children`.
  - Read-only (`403` on `LMSSetValue`): `cmi.core.student_id`, `cmi.core.student_name`, `cmi.core.credit`, `cmi.core.entry`, `cmi.core.total_time`, `cmi.core.lesson_mode`, `cmi.launch_data`, `cmi.comments_from_lms`, `cmi.student_data.mastery_score`, `cmi.student_data.max_time_allowed`, `cmi.student_data.time_limit_action`.
  - Write-only (`404` on `LMSGetValue`): `cmi.core.exit`, `cmi.core.session_time`, and **ALL** `cmi.interactions.n.*` elements (`id`, `time`, `type`, `weighting`, `student_response`, `result`, `latency`, `objectives.m.id`, `correct_responses.m.pattern`).
  - `cmi.core.lesson_status` (RW): `passed | completed | failed | incomplete | browsed | not attempted` (`not attempted` rejected on `LMSSetValue` with `405` per strict RTE!).
  - `cmi.core.lesson_location` (RW): `CMIString255` (max 255 chars; `405` if > 255).
  - `cmi.core.exit` (WO): `"" | "time-out" | "suspend" | "logout"` (`405` otherwise).
  - `cmi.core.score.raw`, `.min`, `.max` (RW): `CMIDecimal` (`""` or plain finite decimal in `0.0..=100.0`, no scientific notation, `405` otherwise).
  - `cmi.core.session_time` (WO): `CMITimespan` `^\d{2,4}:[0-5]\d:[0-5]\d(\.\d{1,2})?$` (`HHHH:MM:SS.SS`, `HH` 2–4 digits, `MM` 00–59, `SS` 00–59, optional `.S` or `.SS`; `405` otherwise — catches **BUG-A2** `"9999:99:99.99"`).
  - `cmi.suspend_data` (RW): `CMIString4096` (max 4096 chars; `405` if > 4096).
  - `cmi.interactions.n.*` (WO):
    - Sequential `n`: `n <= _count` (gaps rejected `201` / `405`).
    - `.id`: `CMIIdentifier` (1..=255 printable ASCII, no whitespace).
    - `.type`: strictly `true-false | choice | fill-in | matching | performance | sequencing | likert | numeric` (`"other"` rejected `405` — catches **BUG-A1**!).
    - `.weighting`: `CMIDecimal` (finite plain decimal, no exponent).
    - `.result`: `correct | wrong | unanticipated | neutral` or `CMIDecimal` (`"incorrect"` rejected `405`).
    - `.student_response` & `.correct_responses.m.pattern`: max 255 chars; `m <= correct_responses._count`; format checked per `.type` (e.g., `true-false` must match `^[tf01]$` / `true`/`false`? Strict SCORM 1.2 `CMIFeedback` allows `0|1|t|f` — flag `"true"`/`"false"` as `405`; record configurable `strict_12_single_char_ids` warning vs error for multi-char choice IDs per M25 open decision 4.9!).
    - `.time`: `CMITime` `HH:MM:SS(.SS)?`; `.latency`: `CMITimespan`.
  - Unknown element name -> `401` ("Not implemented error") + violation.
  - **LMS finish & relaunch semantics:** On `LMSFinish("")`, accumulate `session_time` into `total_time`; if `mastery_score` is configured and `score.raw` was set while `lesson_status == "completed"`, optionally check mastery override; on `relaunch()`, if last `cmi.core.exit == "suspend"`, keep `suspend_data`/`lesson_location`/`lesson_status` and set `cmi.core.entry = "resume"`, else clear attempt data (configurable `clear_on_normal_exit: bool`, default `true` matching Moodle/SCORM Cloud) and set `cmi.core.entry = "ab-initio"`.

### 2. SCORM 2004 3rd & 4th Edition RTE (`crates/strict-lms/src/scorm2004.rs`)
- **Edition flag:** `Scorm2004Edition::{Third, Fourth}` (4th Ed. raises `cmi.suspend_data` from 4000 [3rd] to 64000 [4th] and `cmi.interactions.n.learner_response` limits — test **both** 3rd and 4th ed. limits where applicable!).
- **Exact SCORM 2004 Error Codes (RTE §3.1.7):**
  - `0` No Error; `101` General Exception; `102` General Init Failure; `103` Already Initialized; `104` Content Instance Terminated; `111` General Termination Failure; `112` Termination Before Init; `113` Termination After Termination; `122` Retrieve Before Init; `123` Retrieve After Termination; `132` Store Before Init; `133` Store After Termination; `142` Commit Before Init; `143` Commit After Termination; `201` General Argument Error; `301` General Get Failure; `351` General Set Failure; `391` General Commit Failure; `401` Undefined Data Model Element; `402` Unimplemented Data Model Element; `403` Data Model Element Value Not Initialized; `404` Data Model Element Is Read Only; `405` Data Model Element Is Write Only; `406` Data Model Element Type Mismatch; `407` Data Model Element Value Out Of Range; `408` Data Model Dependency Not Established.
- **Lifecycle:** `NotInitialized -> Running -> Terminated`. Calling `Initialize("")` when `Running` -> `103`; when `Terminated` -> `104`! Calling `GetValue`/`SetValue`/`Commit`/`Terminate` before init -> `122`/`132`/`142`/`112`; after terminate -> `123`/`133`/`143`/`113`. Non-empty arg on `Initialize`/`Terminate`/`Commit` -> `201`.
- **Data Model Elements & Rules:**
  - Read-only (`404`): `cmi._version` (`"1.0"`), `cmi.learner_id`, `cmi.learner_name`, `cmi.entry`, `cmi.mode`, `cmi.credit`, `cmi.total_time`, `cmi.launch_data`, `cmi.scaled_passing_score`, `cmi.max_time_allowed`, `cmi.time_limit_action`, `cmi.completion_threshold`, all `_children`/`_count` keywords, `adl.nav.request_valid.*`.
  - Write-only (`405`): `cmi.exit`, `cmi.session_time`.
  - Uninitialized read (`403`): reading `cmi.location`, `cmi.suspend_data`, `cmi.score.raw/min/max/scaled`, `cmi.interactions.n.description` before written returns `""` with error `403` (note: reading `cmi.suspend_data` on fresh launch in SCORM 2004 commonly gets `403` and `""`, which is legal for a SCO to call and treat as empty, so `403` on fresh `GetValue("cmi.suspend_data")` is recorded as an informational LMS status, NOT a SCO violation!).
  - `cmi.completion_status`: `completed | incomplete | not attempted | unknown` (`406` otherwise).
  - `cmi.success_status`: `passed | failed | unknown` (`406` otherwise).
  - `cmi.score.scaled`: `real(10,7)` in `-1.0..=1.0` (`406` non-decimal, `407` out of range).
  - `cmi.score.raw`, `.min`, `.max`: any finite `real(10,7)` (`406` non-decimal; NO artificial `0..=100` cap!).
  - `cmi.progress_measure`: `real(10,7)` in `0.0..=1.0` (`407` out of range).
  - `cmi.session_time`: ISO 8601 `timeinterval(second,10,2)`: `^P(?:(\d+)Y)?(?:(\d+)M)?(?:(\d+)D)?(?:T(?:(\d+)H)?(?:(\d+)M)?(?:(\d+(?:\.\d{1,2})?)S)?)?$` with at least one component and never bare `"P"` or `"PT"` (`406` otherwise).
  - `cmi.suspend_data`: max 64,000 chars (4th Ed.) / 4,000 chars (3rd Ed.) (`406` if exceeded).
  - `cmi.location`: max 1,000 chars (`406` if exceeded).
  - `cmi.exit`: `"" | "time-out" | "suspend" | "logout" | "normal"` (`"timeout"` rejected `406`!).
  - `adl.nav.request`: `_none_ | continue | previous | exit | exitAll | abandon | abandonAll | suspendAll | {target=<uri>}choice | {target=<uri>}jump` (`406` otherwise).
  - `cmi.interactions.n.*` (RW in 2004!):
    - Index `n` must satisfy `0 <= n <= _count` (`351` if skipped).
    - Dependency (`408`): `.id` must be set before ANY other field of `n`; `.type` must be set before `.learner_response` or `.correct_responses.m.pattern`.
    - `.id`: `long_identifier_type` (1..=4000 chars, valid URI/URN chars, no whitespace).
    - `.type`: `true-false | choice | fill-in | long-fill-in | likert | matching | performance | sequencing | numeric | other` (`406` otherwise).
    - `.description`: `localized_string_type(250)` (max 250 chars after optional `{lang=...}`).
    - `.weighting`: `real(10,7)`.
    - `.result`: `correct | incorrect | unanticipated | neutral` or `real(10,7)` (`"wrong"` rejected `406`!).
    - `.correct_responses.m.pattern` & `.learner_response` per-type validation:
      - `true-false`: strictly `"true"` or `"false"`, max 1 correct_response (`m == 0`; `m >= 1` -> `351`).
      - `choice`: `short_identifier_type` (1..=250 chars, no whitespace, no `[,]`) separated by `[,]`, unique within a single response (`406` on duplicate or empty token when non-empty).
      - `fill-in`: optional `{case_matters=true|false}{order_matters=true|false}` prefix on `correct_responses`, optional `{lang=...}` per item, separated by `[,]`, each <= 250 chars (4th ed. max 10 items).
      - `long-fill-in`: <= 4000 chars.
      - `matching`: `source[.]target` pairs separated by `[,]`, each side `short_identifier_type`.
      - `sequencing`: `short_identifier_type` list separated by `[,]`, unique items (`406` on duplicates in 4th ed.).
      - `likert`: single `short_identifier_type` (NO `[,]`), max 1 correct_response.
      - `other`: `characterstring(4000)`, max 1 correct_response.
      - `correct_responses.m`: sequential `0 <= m <= _count` (`351` on gap).

### 3. cmi5 & xAPI 1.0.3 (`crates/strict-lms/src/xapi_lrs.rs`)
- **HTTP & Headers:**
  - Every LRS request requires `X-Experience-API-Version` matching `^1\.0\.[0-3]$` (`400` + violation); response echoes `X-Experience-API-Version: 1.0.3`.
  - Requests with JSON body require `Content-Type` starting with `application/json` (`400` + violation).
  - `Authorization` must match an issued cmi5 `Basic <token>` or configured xAPI auth (`401`).
  - cmi5 `/fetch/{id}`: `POST` only, **no `Authorization` header allowed** (cmi5 §8.2 violation if auth sent!), single-use (second call returns `200 {"error-code":"1","error-text":"Already in use"}` + violation).
- **xAPI 1.0.3 Statement Validation:**
  - `id`: valid UUID v4/RFC 4122 (`[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}`). Duplicate `id` with byte-identical semantic content -> `200` (idempotent); duplicate `id` with **different** content -> `409 Conflict` + violation!
  - `timestamp`: valid RFC 3339 / ISO 8601 timestamp with timezone offset or `Z`.
  - `actor`: valid `Agent` with exactly ONE Inverse Functional Identifier (`mbox`, `mbox_sha1sum`, `openid`, or `account { homePage, name }`).
  - `verb`: valid absolute IRI `id` + non-empty language map `display`.
  - `object`: valid `Activity` with absolute IRI `id`; for `answered`, `definition.type == "http://adlnet.gov/expapi/activities/cmi.interaction"`, valid `interactionType`, valid `correctResponsesPattern` array matching `interactionType`, and survey questions omit `result.success`.
  - `result.duration` (when present): valid ISO 8601 duration.
  - `result.score` (when present): `scaled` in `-1.0..=1.0`; if `min`/`max`/`raw` present, `min <= raw <= max` and `min < max` (or `min <= max`).
- **cmi5 Profile Rules (cmi5 1.0 §9):**
  - Launch sequence: `launched` (recorded by LMS at launch) -> `initialized` (MUST be first AU statement) -> arbitrary allowed statements (`answered`, etc.) -> `terminated` (MUST be last AU statement; nothing after `terminated`).
  - `registration`: required UUID/non-empty string matching launch registration.
  - `context.extensions["https://w3id.org/xapi/cmi5/context/extensions/sessionid"]`: required on EVERY statement (cmi5-defined AND cmi5-allowed `answered`), matching the session from `LMS.LaunchData.contextTemplate`.
  - `context.contextActivities.grouping`: must preserve all entries from `LMS.LaunchData.contextTemplate`.
  - `context.contextActivities.category`:
    - `initialized`, `completed`, `passed`, `failed`, `terminated` MUST contain `https://w3id.org/xapi/cmi5/context/categories/cmi5`.
    - `answered` MUST NOT contain `cmi5` or `moveon` category.
    - `completed`, `passed`, `failed` MUST contain `https://w3id.org/xapi/cmi5/context/categories/moveon`.
    - `initialized`, `terminated` MUST NOT contain `moveon`.
  - `object.id` & `actor` on cmi5-defined statements (`initialized`, `completed`, `passed`, `failed`, `terminated`) MUST match the launch `activityId` and launch `actor` (by IFI).
  - `result` constraints per cmi5 verb:
    - `initialized`: MUST NOT have `result.score`, `result.success`, `result.completion`.
    - `terminated`: MUST have `result.duration`, MUST NOT have `result.score`, `result.success`, `result.completion`.
    - `completed`: MUST have `result.completion == true` and `result.duration`; MUST NOT have `result.score` or `result.success`.
    - `passed`: MUST have `result.success == true` and `result.duration`; MUST NOT have `result.completion`; if `masteryScore` set in launch data, MUST include `context.extensions["https://w3id.org/xapi/cmi5/context/extensions/masteryscore"] == masteryScore` and `result.score.scaled >= masteryScore`.
    - `failed`: MUST have `result.success == false` and `result.duration`; MUST NOT have `result.completion`; if `masteryScore` set, MUST include `masteryscore` extension and `result.score.scaled < masteryScore`.
  - **Session & Registration Cardinality (cmi5 §9.3.1):**
    - At most ONE of `passed` OR `failed` per session (forbids BOTH `failed` after `passed` AND `passed` after `failed` in one session!).
    - At most ONE `completed` and ONE `passed` per registration across relaunches; never `failed` after a prior `passed` on the same registration.
    - `launchMode` `"Browse"` or `"Review"`: ONLY `initialized` and `terminated` allowed; any `answered`/`completed`/`passed`/`failed` or `PUT activities/state` is a violation!
- **State API (`activities/state`):**
  - Matches `agent` by **canonical IFI equality** (not raw JSON string equality — fixes **BUG-A13**).
  - `LMS.LaunchData`: strictly read-only (`PUT`/`POST`/`DELETE` -> `403` + violation).
  - `coursecraft.progress`: `PUT` stores JSON, `GET` returns `200` (or `404` before first write), `DELETE` returns `204`.
  - Plain xAPI sessions keyed per launch handle (not colliding across relaunches on the same registration — fixes **BUG-A13**).

### 4. LTI 1.3 Local Strict Platform (`apps/coursecraft-lti/src/strict_platform.rs`)
- Runs on `127.0.0.1:0` with generated/fixture RSA keys (supports **2 keys for key-rotation testing**), exposing:
  - `GET /jwks` (returns current platform JWKS, rotatable mid-test).
  - `GET /oidc/auth` (validates every parameter of `login_redirect`: `scope=openid`, `response_type=id_token`, `response_mode=form_post`, `prompt=none`, `client_id`, `redirect_uri`, `state`, `nonce`, `login_hint`, `lti_message_hint`).
  - `POST /oauth/token` (verifies tool's `client_assertion` JWT signature against tool's own `GET /.well-known/jwks.json`, verifies `iss == sub == client_id`, `aud == auth_token_url`, `exp <= iat + 300`, non-reused `jti`, `grant_type=client_credentials`, `client_assertion_type=urn:ietf:params:oauth:client-assertion-type:jwt-bearer`, requested `scope`).
  - `POST /ags/lineitems/{id}/scores` (verifies `Authorization: Bearer <token>`, `Content-Type: application/vnd.ims.lis.v1.score+json`, valid `userId`, `activityProgress` in `Initialized|Started|InProgress|Submitted|Completed`, `gradingProgress` in `FullyGraded|Pending|PendingManual|Failed|NotReady`, valid RFC 3339 `timestamp`, `scoreGiven`/`scoreMaximum` consistency, no double-slash URL).
- Records `Vec<Violation>` and every received AGS score payload for assertions.

---

## A. Concrete Test Cases (56 Cases)

### A1. SCORM 1.2 RTE Strict Conformance (`crates/strict-lms/tests/scorm12.rs` & `crates/lms/tests/strict_scorm12.rs`)

| Case | Variation | Steps / Inputs | Expected Result |
|---|---|---|---|
| **A1.1** | Call order violations (before init / double init / after finish) | Call `LMSGetValue`, `LMSSetValue`, `LMSCommit`, `LMSFinish` before `LMSInitialize`; call `LMSInitialize("")` twice; call `LMSSetValue` and `LMSCommit` after `LMSFinish("")`. | Before init -> `301` + violation; double init -> `101` + violation; after finish -> `301` + violation. |
| **A1.2** | Non-empty argument on lifecycle calls | Call `LMSInitialize("x")`, `LMSCommit("x")`, `LMSFinish("x")`. | Each returns `"false"`, `last_error == "201"`, and logs `SCORM12-201-ARG` violation. |
| **A1.3** | Read-only, write-only, keyword, and unknown elements | `LMSSetValue` on `cmi.core.student_name` (`403`), `cmi.interactions._count` (`402`), `cmi.unknown.foo` (`401`); `LMSGetValue` on `cmi.core.exit` (`404`), `cmi.core.session_time` (`404`), `cmi.interactions.0.result` (`404`). | Exact SCORM 1.2 error codes (`403`, `402`, `401`, `404`) and 6 violations; no 2004 codes (`406`/`408`) ever returned. |
| **A1.4** | `lesson_status` & `exit` vocabularies | Write all valid statuses (`passed`, `completed`, `failed`, `incomplete`, `browsed`) vs invalid (`not attempted`, `unknown`, `PASSED`, `""`); write valid `exit` (`""`, `suspend`, `logout`, `time-out`) vs invalid (`normal`, `timeout`). | Valid values return `"true"` (`0` violations); invalid values return `"false"`, error `405`, + violation. |
| **A1.5** | `CMITimespan` (`session_time`) & saturation bug (**BUG-A2**) | Test `format_session_time` at `0`, `10`, `59_990`, `3_661_050`, `35_999_999_990`, `u64::MAX` ms against `StrictScorm12::set_value("cmi.core.session_time", ...)`. | `0` -> `"0000:00:00.00"`; `u64::MAX` -> `"9999:59:59.99"` (after fixing BUG-A2); `"9999:99:99.99"`, `"PT5S"`, `"00:60:00.00"` all rejected with `405`. |
| **A1.6** | `lesson_location` 255-char boundary (**BUG-A3**) | Write 255-char ASCII, 255-char multi-byte UTF-8, and 256-char slide IDs through `LmsBridge::after_input` -> `StrictScorm12`. | 255-char succeeds (`0` violations); 256-char is skipped by `LmsBridge` with warning (after fixing BUG-A3), never reaching `LMSSetValue` with a `405` violation. Direct strict call with 256 chars returns `405`. |
| **A1.7** | `suspend_data` 4096-char boundary | Write 4096-char `cc1:` payload vs 4097-char payload through `LmsBridge` and directly to `StrictScorm12`. | 4096 succeeds (`0` violations); 4097 skipped by `LmsBridge` with warning (`0` LMS violations); direct call with 4097 returns `405`. |
| **A1.8** | `score.raw/min/max` `CMIDecimal` & decimal resume (**BUG-A5**) | Set valid `0`, `85`, `85.5`, `100` vs invalid `-1`, `100.1`, `1e2`, `NaN`, `"abc"`. Resume `LmsBridge` when LMS returns `cmi.core.lesson_status = "passed"` and `cmi.core.score.raw = "85.0"`, then emit `Effect::Score { percent: 60, Passed }`. | Invalid decimals rejected `405`; after fixing BUG-A5, `"85.0"` parses as `best_passed_score = Some(85)` so `60` does NOT overwrite `85.0`. |
| **A1.9** | All 12 `QuestionKind`s -> `cmi.interactions.n.*` (**BUG-A1**) | Grade every one of the 12 question kinds through `LmsBridge(Scorm12)` -> `StrictScorm12` (including `Freeform`, `Likert`, `ShortAnswer`, `Essay`, `Hotspot`, `DragDrop`, `Sequence`, `Matching`). | Zero `405` violations once **BUG-A1** maps `Freeform` in SCORM 1.2 to a legal 1.2 type (e.g. `fill-in` or `performance` per spec decision in `docs/spec/triggers.md`); `cmi.interactions._count` increments to 12; `.result` uses `wrong` (never `incorrect`) and `neutral` for surveys; `.description` is NOT written in 1.2. |
| **A1.10** | Interaction index gap, failed `.id` write, & 255-char `.id` limit (**BUG-A4**) | Direct strict test writes `cmi.interactions.2.id` when `_count == 0`; bridge test simulates LMS refusing `.id` write (`405`) and tests a 260-char slide ID. | Gap returns `201`/`405` violation; bridge truncates/guards `.id` to 255 chars and does not increment `interaction_index` or write orphan child fields when `.id` fails. |
| **A1.11** | `total_time` accumulation & `exit` -> `entry` relaunch semantics | Session 1 sets `session_time = 0000:01:30.50`, `exit = suspend`, `suspend_data = "cc1:..."`, `LMSFinish("")` -> `relaunch()`. Session 2 sees `entry == "resume"`, `total_time == "0000:01:30.50"`, finishes with `exit = ""` -> `relaunch()`. | Session 2 resumes; Session 3 sees `entry == "ab-initio"` and cleared `suspend_data` (when `clear_on_normal_exit = true`). |
| **A1.12** | Empty initial status `""` & `LMSInitialize` failure (**BUG-A6**) | LMS returns `cmi.core.lesson_status = ""` on launch; second sub-case fails `LMSInitialize` with `101`. | Empty status writes `"incomplete"` (after fixing BUG-A6); failed init sets `active = false`, emits warning, and sends zero subsequent `LMSSetValue`/`LMSFinish` calls (`0` post-failure violations). |

### A2. SCORM 2004 3rd & 4th Edition RTE Strict Conformance (`crates/strict-lms/tests/scorm2004.rs` & `crates/lms/tests/strict_scorm2004.rs`)

| Case | Variation | Steps / Inputs | Expected Result |
|---|---|---|---|
| **A2.1** | Exact SCORM 2004 lifecycle error codes | Before init: `GetValue`(`122`), `SetValue`(`132`), `Commit`(`142`), `Terminate`(`112`). While running: `Initialize`(`103`). After `Terminate("")`: `Initialize`(`104`), `GetValue`(`123`), `SetValue`(`133`), `Commit`(`143`), `Terminate`(`113`). | Every single error code matches SCORM 2004 RTE §3.1.7 verbatim (no 1.2 `301`/`101` codes!). |
| **A2.2** | Access & uninitialized error codes (`401`–`408`) | Unknown key (`401`), `SetValue` on `cmi.learner_name` (`404`), `GetValue` on `cmi.exit` (`405`), `SetValue` bad type (`406`), `SetValue` out of range (`407`), `SetValue("cmi.interactions.0.type", "choice")` before `.id` (`408`), `GetValue("cmi.location")` before set (`403`). | Exact codes `401`, `403`, `404`, `405`, `406`, `407`, `408`. |
| **A2.3** | `completion_status` & `success_status` independence | Drive `LmsBridge(Scorm2004)` through `Incomplete`, `Completed` (no quiz), `Passed`, `Failed`, and `Passed` followed by `Failed`. | `Completed` leaves `success_status` untouched (`unknown`); `Passed` writes `completed` + `passed`; `Failed` writes `completed` + `failed`; later `Failed` after `Passed` is ignored; zero violations. |
| **A2.4** | `score.scaled` range & `score.raw/min/max` real(10,7) | Set `scaled` to `-1`, `-0.5`, `0`, `0.85`, `1` (valid) vs `-1.01`, `1.01` (`407`), `1e-2` (`406`). Set `score.raw` to `250`, `-10`, `99.5` (all valid in 2004!). | `scaled` bounds enforced with `407`; arbitrary finite `raw`/`min`/`max` accepted (unlike buggy `mock-lms/index.html`). |
| **A2.5** | ISO 8601 `cmi.session_time` | Test `format_session_time_iso` at `0`, `50`, `1_000`, `3_661_050`, `u64::MAX` ms, plus direct strict checks on `P1DT2H3M4.5S` (valid) vs `PT`, `P`, `0000:01:00.00`, `PT-5S`, `PT1.234S` (invalid `406`). | All `LmsBridge` outputs accepted with 0 violations; invalid strings rejected with `406`. |
| **A2.6** | `suspend_data` 64,000 (4th Ed.) vs 4,000 (3rd Ed.) & `cmi.location` 1,000 | Test `suspend_data` at 4,000, 4,001, 64,000, 64,001 chars under both `Third` and `Fourth` editions; test `cmi.location` at 1,000 vs 1,001 chars. | 4th Ed. accepts up to 64,000 and rejects 64,001 (`406`); 3rd Ed. rejects 4,001 (`406`); `LmsBridge` skips 1,001-char `cmi.location` with warning (`0` LMS violations). |
| **A2.7** | `cmi.interactions.n` dependency, sequential index, & per-type formats | Drive all 12 `QuestionKind`s through `LmsBridge(Scorm2004)` -> `StrictScorm2004`, checking: `.id` first, `.type` second, `.description` <=250, `true-false` (`true`/`false`), `choice` (`a[,]b`), `matching` (`a[.]b[,]c[.]d`), `sequencing` (`a[,]b`), `fill-in` (`{case_matters=true}...`), `likert` (single short ID), `other` (`Freeform`), `.result` (`correct`/`incorrect`/`neutral`). | Zero violations across all 12 kinds; direct negative checks catch `wrong` (`406`), `t`/`f` (`406`), skipped index `n=2` (`351`), duplicate choice ID `a[,]a` (`406`), and `true-false` with 2 correct patterns (`351`). |
| **A2.8** | `cmi.interactions.n` readback in SCORM 2004 | After writing interaction `0`, call `GetValue("cmi.interactions.0.id")`, `.type`, `.learner_response`, `.result`, `.correct_responses._count`. | Unlike SCORM 1.2 (write-only), SCORM 2004 returns all written values with error `0` and `0` violations! |
| **A2.9** | `cmi.exit` vocabulary (`time-out` vs `timeout`, `normal` vs `""`) | Test `""`, `time-out`, `suspend`, `logout`, `normal` (valid) vs `timeout`, `finish` (`406`). Verify `LmsBridge(Scorm2004)` writes `"suspend"` before completion and `"normal"` after completion. | Zero violations on bridge; `"timeout"` rejected with `406`. |
| **A2.10** | `adl.nav.request` vocabulary | Write `_none_`, `continue`, `previous`, `exit`, `exitAll`, `abandon`, `abandonAll`, `suspendAll`, `{target=item_1}choice`, `{target=item_2}jump` vs invalid `next`, `{target=}choice`. | Valid tokens accepted; invalid tokens rejected `406` + violation. |
| **A2.11** | `cmi.interactions.n.description` & text escaping `[,]` / `[.]` | Question prompt and fill-in learner response containing literal `"a[,]b[.]c\x00\n"` and 300-char string. | Control chars stripped, `[,]`/`[.]` escaped, truncated on char boundary to <=250 chars, `0` violations. |
| **A2.12** | Browser `mock-lms` WASM parity test | Run the same scripted SCORM 1.2 + 2004 call sequences through `StrictScorm12`/`StrictScorm2004` JSON/string dispatch entrypoints that `xtask/mock-lms` calls via WASM. | Identical return strings, error codes, and violation lists to native `CmiApi` calls. |

### A3. cmi5 & xAPI 1.0.3 Strict Conformance (`crates/strict-lms/tests/xapi_cmi5.rs` & `crates/lms/tests/strict_xapi.rs`)

| Case | Variation | Steps / Inputs | Expected Result |
|---|---|---|---|
| **A3.1** | Full cmi5 normal launch -> pass -> terminate | Drive `XapiBridge(Cmi5)` against `StrictLrs`: `POST /fetch/{id}` -> `GET LMS.LaunchData` -> `initialized` -> `GET coursecraft.progress` (404) -> `answered` (all 12 kinds) -> `passed` -> `completed` -> `PUT progress` -> `terminated`. | Zero violations; exact verb order `[initialized, answered×12, passed, completed, terminated]`; `cmi5` & `moveon` categories, `sessionid`, `grouping`, `masteryscore` all exact. |
| **A3.2** | cmi5 verb order & forbidden transitions (cmi5 §9.3.1) | Direct `StrictLrs` tests: statement before `initialized`; duplicate `initialized`; statement after `terminated`; **`passed` after `failed` in same session**; `failed` after `passed` in same session; `failed` after `passed` across relaunch; second `completed` on same registration. | Every single forbidden sequence produces a distinct, named `CMI5-*` violation (fixing **BUG-A13**). |
| **A3.3** | cmi5 `result` field exclusions & `masteryScore` agreement | Direct tests: `completed` carrying `result.score` or `result.success`; `passed` carrying `result.completion` or `scaled < masteryScore` or missing `masteryscore` extension; `initialized` carrying `score`; `terminated` missing `duration`. | Each triggers a named `CMI5-RESULT-*` violation. |
| **A3.4** | cmi5 `fetch` single-use & no-Auth header rule | `XapiBridge` calls `POST /fetch/{id}` -> verify zero `Authorization` header on fetch request. Replay same fetch URL -> `200 {"error-code":"1"}`, bridge sets `reporting_lost() == true` and emits 0 statements. Also test empty `"auth-token": ""` (**BUG-A10**). | No auth header on fetch; reused/empty token disables reporting cleanly with 0 LRS violations. |
| **A3.5** | cmi5 `Browse` and `Review` launch modes | Launch with `launchMode: "Browse"` and `"Review"`; answer questions, submit results, complete course, exit. | Only `initialized` and `terminated` sent; zero `answered`/`passed`/`failed`/`completed` statements; zero `PUT coursecraft.progress`; `reporting_lost() == false`; zero violations. |
| **A3.6** | `LMS.LaunchData` protection & hostile launch data | Attempt `PUT activities/state?stateId=LMS.LaunchData` -> `403` + violation. Test `XapiBridge` with malformed `masteryScore: 1.5`, `returnURL: "javascript:alert(1)"`, non-object JSON, and 404 on `LMS.LaunchData`. | `LMS.LaunchData` write refused `403`; bridge rejects `javascript:` URL and out-of-range mastery with warnings, never panics, and sends valid cmi5 statements. |
| **A3.7** | State API IFI-based `agent` matching & resume (**BUG-A13**) | Store `coursecraft.progress` with agent keys in order `{"account":{...},"name":"...","objectType":"Agent"}`; fetch with reordered JSON keys `{"objectType":"Agent","account":{...}}`. | `StrictLrs` matches by canonical IFI (returns `200` with saved progress) instead of failing raw-string lookup. |
| **A3.8** | xAPI 1.0.3 headers, UUID, timestamp, `409 Conflict` deduplication | Send missing/bad `X-Experience-API-Version` (`400`), missing `Content-Type` (`400`), bad UUID `id`, bad RFC 3339 timestamp, duplicate `id` with identical body (`200`), duplicate `id` with mutated body (`409` + violation). | All xAPI 1.0.3 protocol rules strictly enforced. |
| **A3.9** | Plain xAPI relaunch on same registration & optional registration | Run plain `Flavor::Xapi` session 1 (with `registration`), finish (`terminated`), relaunch session 2 with same `registration`; also run session 3 with `registration: None`. | No false-positive `statement after terminated` collision across relaunches (fixing **BUG-A13**); zero `cmi5`/`moveon` categories emitted in plain xAPI; zero violations. |
| **A3.10** | Network failures, 5xx exponential backoff, 401/403 stop, 400 drop, keepalive chunking | Inject 3× HTTP 500 (verify 1s, 2s, 4s backoff and same UUIDs reused), HTTP 409 (treated as sent), HTTP 400 (batch dropped, continues), HTTP 401 (stops reporting, `reporting_lost() == true`), and >60 KB finish flush + oversized progress (**BUG-A9**). | Retries preserve statement order and UUIDs; keepalive splits stay under `MAX_KEEPALIVE_BODY`; zero violations. |
| **A3.11** | Launch parameter parser & secret redaction (**BUG-A10**) | Test `LaunchParams::parse` with >16 KiB query, missing each required param, hostless `"https://"`, Tin Can array-valued actor `{"mbox":["mailto:a@b.c"]}`, invalid UTF-8 escapes, and verify `Debug`/`Display`/warnings NEVER leak `auth` or `fetch` values. | Hostless URL rejected (after fixing BUG-A10); Tin Can array unwrapped; zero secret leaks in any error or debug string. |
| **A3.12** | Lightbox durability parity in `XapiBridge` (**BUG-A9**) | Open and close a lightbox slide in `XapiBridge::after_input`, finish session, resume from `coursecraft.progress`. | Lightboxed slide is persisted in `visited` inside `coursecraft.progress` (after fixing BUG-A9) and restored on resume. |

### A4. LTI 1.3 Local Strict Platform & Security Conformance (`apps/coursecraft-lti/src/tests.rs` & `crates/lti/src/tests.rs`)

| Case | Variation | Steps / Inputs | Expected Result |
|---|---|---|---|
| **A4.1** | Full zero-network OIDC login -> launch -> course -> AGS pass score | `StrictLtiPlatform` (`127.0.0.1:0`) -> `GET /lti/login` -> verify redirect params -> `POST /lti/launch` with RS256 `id_token` + state cookie -> follow redirect (`?name=...`) -> fetch `/course/index.html` + `course.json` -> `POST /lti/score` (`85, "passed"`) -> verify `/oauth/token` client assertion + AGS score POST. | `204` from `/lti/score`; `StrictLtiPlatform` records `0` violations and exact AGS payload (`scoreGiven: 85, scoreMaximum: 100, activityProgress: "Completed", gradingProgress: "FullyGraded"`). |
| **A4.2** | Completion-only (no score) & `incomplete` AGS payloads | Post `{"percent": null, "status": "completed"}` and `{"percent": null, "status": "incomplete"}`. | `completed` omits `scoreGiven`/`scoreMaximum` (`Completed`/`FullyGraded`); `incomplete` sends `InProgress`/`Pending`; `0` violations. |
| **A4.3** | Trailing slash on `lineitem` URL & query string preservation (**BUG-A11**) | Launch with `ags.lineitem = "http://127.0.0.1:PORT/ags/lineitems/42/"` and `".../lineitems/42?ctx=9"`. | Score posts to `.../42/scores` (NOT `.../42//scores`) and `.../42/scores?ctx=9` (after fixing BUG-A11). |
| **A4.4** | Missing AGS `score` scope rejection (**BUG-A11**) | Launch with `ags.lineitem` set but `ags.scope = ["https://purl.imsglobal.org/spec/lti-ags/scope/lineitem.readonly"]` (no `scope/score`). | `/lti/score` returns `403` without requesting a token (after fixing BUG-A11). |
| **A4.5** | `id_token` negative security matrix | Test tampered signature, `alg: "none"`, `alg: "HS256"`, expired `exp`, future `iat > now+300`, wrong `iss`, wrong `aud`, multi-`aud` with wrong `azp`, wrong `nonce`, replayed `state`, missing/mismatched `cc_lti_state` cookie, wrong `message_type`, wrong `version`, wrong `deployment_id`, missing `target_link_uri` / `roles` (**BUG-A11**). | Every invalid token/launch rejected with `400`; zero sessions created. |
| **A4.6** | Platform JWKS key rotation + cooldown | Rotate `StrictLtiPlatform` signing key from `kid-1` to `kid-2`; launch with `kid-2` -> server refreshes JWKS once and succeeds; immediately launch with unknown `kid-3` within 60s cooldown -> server rejects without hammering `/jwks`. | `kid-2` succeeds on first try via single refresh; `kid-3` blocked by cooldown (`400`). |
| **A4.7** | Tool JWKS & `client_assertion` verification | Fetch `GET /.well-known/jwks.json`; verify `StrictLtiPlatform` validates tool's `client_assertion` signature, `iss == sub == client_id`, `aud == auth_token_url`, `exp - iat <= 300`, unique `jti`, and caches access token across two `/lti/score` calls. | Only 1 `/oauth/token` call for 2 score posts; `0` violations. |
| **A4.8** | `/lti/score` CSRF & session security | Send wrong `Origin`, missing `Origin`, `Content-Type: text/plain`, missing `cc_lti` cookie, unknown/expired cookie, extra unknown JSON field (`deny_unknown_fields`), `percent: 101`. | CSRF -> `403`, auth -> `401`, bad JSON/percent -> `400`; platform receives 0 requests. |
| **A4.9** | `LtiReporter` high-water pass, deduplication, & retry/lost-reporting | Feed `LtiReporter` sequence: `Score(80, Passed)` -> `204` -> `Score(60, Failed)` (ignored) -> `Score(70, Passed)` (ignored) -> `Score(90, Passed)` (sent) -> `500` (retried once) -> `500` (`lost() == true`). | Only `80` and `90` (with 1 retry) sent; `lost() == true` after second failure. |
| **A4.10** | Path prefix mount & static course Range/ETag/traversal safety | Mount `tool_url = "https://tool.example/sub/app"`; verify launch redirect `/sub/app/course/index.html?name=...`, `ETag`/`304`, `Range: bytes=0-9` (`206`), unsatisfiable `Range` (`416`), and `/course/../secret` (`404`). | All status codes and headers exact; path traversal blocked. |

### A5. Package Manifests & Player Native-Seam Wiring (`crates/strict-lms/tests/manifests.rs` & `apps/coursecraft-player/src/session.rs`)

| Case | Variation | Steps / Inputs | Expected Result |
|---|---|---|---|
| **A5.1** | SCORM 1.2 `imsmanifest.xml` strict validation (**BUG-A7**) | Publish course -> validate XML structure, namespaces (`imscp_rootv1p1p2`, `adlcp_rootv1p2`), `<schemaversion>1.2</schemaversion>`, `adlcp:scormtype="sco"`, valid XML `NCName` IDs, every `<file href>` exists in zip, every package file except `imsmanifest.xml` is listed (and `imsmanifest.xml` does NOT list itself once BUG-A7 is fixed). | Zero manifest violations. |
| **A5.2** | SCORM 2004 4th Ed. `imsmanifest.xml` strict validation | Publish course as `scorm2004` -> validate `imscp_v1p1`, `adlcp_v1p3`, `adlcp:scormType="sco"` (capital `T`), `<schemaversion>2004 4th Edition</schemaversion>`, `coursecraft-package.json` `{"standard":"scorm2004"}`, absence of `masteryscore`/`imsss:sequencing`, complete file manifest. | Zero manifest violations. |
| **A5.3** | `cmi5.xml` & `tincan.xml` strict validation | Publish course as `cmi5` and `xapi` -> verify no `imsmanifest.xml`, valid `coursecraft-package.json`, well-formed XML, valid AU/activity IRI, `masteryScore` in `0..=1` (when quiz present), `launchMethod`, `moveOn`, relative launch URL `index.html`. | Zero manifest violations. |
| **A5.4** | LTI & Web package marker & manifest absence | Publish course as `lti` and `web` -> verify `{"standard":"lti"}`/`{"standard":"web"}`, zero `imsmanifest.xml`/`cmi5.xml`/`tincan.xml`, templates stripped from `course.json`, all media/fonts/captions/web-folders/video-posters/path-picture-fills (**BUG-B3**) present. | Zero missing files; deterministic byte-identical zip across two runs. |
| **A5.5** | Native player seam: `RunJavaScript` / media followup effect ordering (**BUG-A8**) | Drive `apps/coursecraft-player::session::PlayerSession` natively with a slide whose trigger emits `Score(100, Passed)` + `RunJavaScript("CourseCraft.jump('s-end')")` (where `s-end` exits). | `Score(100, Passed)` reaches `StrictScorm12`/`StrictScorm2004`/`StrictLrs` **before** `SlideEntered(s-end)` and `Exit`; `Score` is NOT dropped! |
| **A5.6** | Native player seam: `package-marker` fallback & hostile marker | Parse missing marker (defaults SCORM 1.2), valid markers (`scorm12`, `scorm2004`, `cmi5`, `xapi`, `lti`, `web`), unknown standard `"scorm99"`, corrupt JSON. | Single parser (`coursecraft_lms::reporting_from_marker`) handles all 6 standards and falls back cleanly on unknown/corrupt JSON with 1 warning. |
| **A5.7** | Native player seam: `WebResume` lifecycle & Start Over | Drive `PlayerSession` with `Standard::Web` + `MemoryStorage`: advance slides, update vars/states, simulate `pagehide` -> relaunch -> verify exact resume -> trigger Start Over -> verify storage cleared & restart at slide 0. | Exact resume without any LMS calls; Start Over resets cleanly. |
| **A5.8** | Native player seam: `pagehide` / `beforeunload` idempotence | Fire `on_page_close()` twice (simulating both `pagehide` and `beforeunload` firing in browser) on SCORM 1.2, SCORM 2004, and cmi5. | `LMSFinish`/`Terminate`/`terminated` sent **exactly once**; zero duplicate-finish violations. |
| **A5.9** | Native player seam: `HostInfo` (`Learner.Name` & `Course.Date`) across all standards | Launch with learner name from `cmi.core.student_name` (1.2), `cmi.learner_name` (2004), xAPI `actor.name`, LTI `?name=...` (with URL encoding & >200 char truncation). | `Player::display_text("%Learner.Name% %Course.Date%")` resolves accurately in all 4 standards. |
| **A5.10** | Native player seam: `reporting_lost()` banner trigger across cmi5 & LTI | Trigger 401 mid-session on cmi5 and 2× 500 on LTI via `PlayerSession`. | `session.reporting_lost()` flips to `true`, course stays playable without crashing, and zero further network/LRS requests are sent. |

### A. Files / Crates to Touch & Ordered Implementation Steps

- **Step A-1 (1 session):** Create `crates/strict-lms` (`Cargo.toml`, `src/lib.rs`, `src/scorm12.rs`, `src/scorm2004.rs`, unit tests `tests/scorm12.rs`, `tests/scorm2004.rs`); register in workspace `Cargo.toml` and `xtask/src/main.rs` (`LAYERS` L1, `WASM_CRATES`).
- **Step A-2 (1 session):** Add `crates/strict-lms/src/xapi_lrs.rs` and `src/manifest.rs` + unit tests (`tests/xapi_cmi5.rs`, `tests/manifests.rs`); migrate `xtask/src/mock_lrs.rs` to delegate validation & state storage to `coursecraft_strict_lms::StrictLrs` (fixing **BUG-A13**).
- **Step A-3 (1 session):** Fix SCORM/xAPI/publish defects exposed by `strict-lms` (**BUG-A1**–**BUG-A7**, **BUG-A9**, **BUG-A10**, **BUG-B3**) in `crates/lms/src/{bridge.rs,interaction.rs,time.rs,lrs.rs}`, `crates/xapi/src/launch.rs`, `crates/scorm/src/lib.rs`, `crates/publish/src/lib.rs`, updating `docs/spec/triggers.md` in the same commit; add `crates/lms/tests/strict_scorm12.rs`, `strict_scorm2004.rs`, `strict_xapi.rs`.
- **Step A-4 (1 session):** Wire `xtask/mock-lms/index.html` to `crates/strict-lms` via a thin `wasm-bindgen` wrapper (`xtask/strict-lms-wasm`, built in `xtask/src/main.rs` `package_example`), replacing hand-written JS validation rules (fixing **BUG-A12**) and adding a Violations panel to `mock-lms/index.html`.
- **Step A-5 (1 session):** Extract `apps/coursecraft-player/src/session.rs` native seam, unify `reporting_from_marker` (`crates/lms/src/marker.rs` + `apps/coursecraft-player/src/load.rs`), fix **BUG-A8** (`app.rs:452-539`), and add native seam tests **A5.5–A5.10**.
- **Step A-6 (1 session):** Add `apps/coursecraft-lti/src/strict_platform.rs`, fix **BUG-A11** (`crates/lti/src/lib.rs`, `apps/coursecraft-lti/src/server.rs`, `apps/coursecraft-player/src/lti.rs`), and add tests **A4.1–A4.10**.

---

# Part B: Reference Course & Automated Multi-Standard Runs

## B. Reference Course Design (`examples/reference-course/`)

Create **`examples/reference-course/`** with:
- `course.json` — valid under `Course::validate()` with **0 errors and 0 unintended warnings** (except the intentional `run_javascript` info warning and intentional `disabled` trigger test).
- `media/` — tiny deterministic, self-contained fixtures (< 100 KB total so repo & CI stay fast):
  - `hero.png` (64×64 PNG), `alt-state.png` (64×64 PNG for `StateDef.src`), `hotspot.png` (400×300 PNG), `logo.png` (48×24 PNG for `player.logo`), `poster.png` (160×90 PNG for `Video.poster`), `tile.png` (16×16 PNG for `FillStyle::Picture`), `narration.wav` (tiny valid 1.5s PCM WAV), `narration.vtt` (WebVTT captions), `bg-loop.wav` (tiny valid 0.5s PCM WAV for `background_audio`), `clip.mp4` (copy of tiny `examples/showcase/media/intro.mp4`), `clip.vtt`, `anim.json` (tiny valid 1s non-looping Lottie JSON), `web/mini/index.html` (minimal local HTML web object), `guide.txt` (resource file).
- `README.md` — feature index and commands (`validate`, `simulate --strict`).
- Scripted input files: `pass.inputs.json`, `fail.inputs.json`, `partial-1.inputs.json`, `partial-2-resume.inputs.json`, `retry-review.inputs.json`, `completion-only.inputs.json` (for a companion non-quiz variant `examples/reference-course/completion-only.json` or generated test course).

### 100% Runtime Feature Coverage Matrix (`examples/reference-course/course.json`)

Every single enum variant in `crates/course/src/model.rs` is placed on an explicit slide so automated runs assert every one fired:

| Scene / Slide(s) | Features Exercised (Exhaustive Inventory) |
|---|---|
| **Course root** | `schema_version: 1`, `id: "reference-course"`, `language: "en-US"`, `strings` override (`nextLabel`), `theme` + `palette` + `text_styles`, `quiz: { passing_percent: 70 }`, `background_audio: { src: "bg-loop.wav", volume: 0.4, loop: true }`, `resources` (1 `Url` + 1 `File`), `glossary` (2 terms), `masters` (1 master with logo + background), `templates` (1 template — verified stripped on publish), `player: { show_nav, show_menu, show_seekbar, show_speed, show_fullscreen, show_transcript: true, logo: "logo.png", navigation: "restricted", next_requires: { kind: "nothing" } }`, `start_slide: "s01-intro"`. |
| **Variables** (6 user + 12 static built-ins + 1 dynamic timer var) | User vars: `v_flag: bool(false)`, `v_ready: bool(false)`, `v_score_bonus: number(10)`, `v_step: number(0)`, `v_tag: text("Alpha")`, `v_input_text: text("")`, `v_slider: number(20)`, `v_dial: number(50)`, `v_check: bool(false)`, `v_radio: text("opt1")`. Text interpolation `%var%` & `%%` covering all **12 static built-ins** (`Course.SlideNumber`, `Course.TotalSlides`, `Course.Progress`, `Course.ScorePoints`, `Course.ScorePercent`, `Quiz.Passed`, `Quiz.Attempts`, `Quiz.Reviewing`, `Slide.ElapsedMs`, `Learner.Name`, `Course.Title`, `Course.Date`) + dynamic `%Timer.t_main.Remaining%`. |
| **Scene 1 — Core Runtime (`sc-core`)** | |
| `s01-intro` (`master: "m-main"`, `transition: Fade`, `revisit: auto`, `notes` for transcript) | **Events:** `timeline_starts`, `timeline_reaches { ms: 500 }`, `timeline_reaches { cue: "cue-mid" }` (`SlideCue`), `timeline_ends`, `click`, `hover_start`, `hover_end`, `state_changes { to: None }`, `state_changes { to: Some("active") }`, `variable_changes`. **Actions:** `change_state` (built-in `selected`/`visited`/`disabled`/`hidden` + custom `active` with `fill`+`text`+`src`), `set_variable` (`assign`, `add`, `subtract`, `multiply`, `divide`, `toggle`, + var-to-var `ValueSource::Variable`!), `open_url`, `run_javascript` (`CourseCraft.setVar(...)`), `disabled: true` trigger (must never fire!). **Conditions:** `all`, `any`, `not`, `object_state { is: true/false }`, `compare` with all **9 `CmpOp`s** (`eq`, `ne`, `lt`, `le`, `gt`, `ge`, `contains`, `eq_ignore_case`, `contains_ignore_case`) including var-to-var compare. **Slide override:** `next_requires: { kind: "timeline_ends" }`. |
| `s02-layers` (`transition: Push`, `revisit: resume`) | **All 7 layer options:** `l-modal` (`hide_other_layers: true`, `pause_base_timeline: true`, `prevent_base_clicks: true`, `allow_seek: no`, `revisit: resume`), `l-overlay` (`hide_other_layers: false`, `hide_base_objects: true`, `hide_when_timeline_ends: true`, `allow_seek: yes`, `revisit: reset`), `l-pause` (`pause_timeline` / `resume_timeline` on layer). **Actions:** `show_layer`, `hide_layer { layer: None }` (self-hide), `hide_layer { layer: Some("l-modal") }`, `pause_timeline`, `resume_timeline`. **Slide override:** `next_requires: { kind: "variable", name: "v_ready" }`. |
| `s03-inputs-widgets` (`transition: Cover`, `revisit: reset`) | **All remaining `ObjectKind`s:** `text` (rich runs + `Link::Url` + `Link::Slide`), `shape` (gradient + `ShapeText`), `image` (crop + state `src`), `button`, `path` (closed with picture `fill_style` — tests **BUG-B3**!), `table` (merged cells + `header_row`), `web` (`WebSource::Folder`), `text_entry` (`max_chars: 20`), `slider`, `dial`, `checkbox`, `radio_group`, `marker` (`marker_opened` / `marker_closed`), `scroll_panel` (child object inside panel), `progress_meter` (bound to `v_slider`). |
| `s04-motion-drag-timer` | **Animations & Motion:** entrance (`fade`, `fly_in`, `zoom`, `wipe`, `float`), exit (`fly_in`), `motion_paths` (relative + absolute) + `move_on_path` + `animation_completes { path: None }` & `{ path: Some("p1") }`. **Draggable & Intersection:** freeform `drag { targets, snap: true, return_on_miss: true }` -> `dropped_on`, `objects_intersect`, `objects_stop_intersecting`. **Key press:** `key_pressed { key: "k", object: None }` & `key_pressed { key: "enter", object: Some("btn-key") }`. **Timer:** `timer` (`autostart: false`, `duration_ms: 3000`) with `start_timer`, `pause_timer`, `reset_timer`, `timer_ends`, and `variable_changes { variable: "Timer.t_main.Remaining" }`. |
| `s05-media-cues` | **Media:** `audio` (trimmed `start_ms: 200, end_ms: 800`, `captions: "narration.vtt"`), `video` (`poster: "poster.png"`, `captions: "clip.vtt"`, pausing cue `c1 @ 1000ms` + non-pausing cue `c2 @ 2000ms`), `lottie` (`loop: false`, `autoplay: true`, `speed: 1.0`), plus `Course.BackgroundAudio` control. **Events:** `cue_reached`, `media_ends` (audio, video, lottie, background audio). **Actions:** `play_media`, `pause_media`, `stop_media`. |
| `s06-branch-lightbox` + `s06b-branch-alt` + `s06c-lightbox` | **Branching & Navigation:** `jump_to_slide` (`next`, `previous`, `id`, `last_visited`, `scene("sc-quiz")`), `lightbox_slide { slide: "s06c-lightbox" }` (opened from a slide with a video/cues to also guard **BUG-B1** & **BUG-B2**!), `close_lightbox`, slide `Link::Slide`. |
| **Scene 2 — Comprehensive Quiz (`sc-quiz`)** | |
| `s07-q-mc` | `MultipleChoice`, `scoring: PerAnswer` (choice points `10 / 5 / 0`), `attempts: 2`, `try_again_layer`, `correct_layer`, `incorrect_layer`, `shuffle: true`. |
| `s08-q-mr` | `MultipleResponse`, `scoring: Partial`, `points: 10`, `attempts: 1`, `Submit` trigger (scripted) + click Submit button. |
| `s09-q-tf` | `TrueFalse`, `scoring: AllOrNothing`, `points: 10`, `attempts: 0` (unlimited!). |
| `s10-q-fill` | `FillIn`, `case_sensitive: true`, `accepted: ["CourseCraft", "Rust"]`, `points: 10`. |
| `s11-q-seq` | `Sequence`, `scoring: Partial`, `points: 10` (tests deterministic FNV-1a/SplitMix64 shuffle). |
| `s12-q-match` | `Matching`, `scoring: Partial`, `points: 10`, 3 pairs. |
| `s13-q-hotspot` | `Hotspot`, `markers: 2`, 2 rect/ellipse regions (`correct: true` + distractor), `scoring: Partial`, `points: 10`. |
| `s14-q-drag` | `DragDrop`, `snap_to_center: true`, `return_if_wrong: true`, `one_per_target: true`, `scoring: Partial`, `points: 10`, 2 items + 3 targets (1 distractor). |
| `s15-q-freeform` | `Freeform`, `correct_when: Some(All([ObjectState("card1","selected",true), Compare("v_flag", Eq, true)]))`, `points: 10`, `revisit: reset` (so retry resets card state). |
| `s16-q-surveys` (3 slides: `s16a-likert`, `s16b-short`, `s16c-essay`) | All 3 survey kinds (`Likert`, `ShortAnswer`, `Essay`), 0 points, `neutral` interaction result, xAPI omits `result.success`. |
| `s17-draw-1` & `s18-draw-2` (from `bank-pool` of 4 questions) | **Question Bank & Draws:** `bank-pool` has 4 distinct questions (`bk-q1`..`bk-q4`); `s17-draw-1` draws `count: 2` and `s18-draw-2` draws `count: 1` (verifies no duplicate draws across two draw slides and deterministic seed + new draw on `RetryQuiz`!). |
| **Scene 3 — Results & Certificate (`sc-end`)** | |
| `s19-results` (`player: { show_seekbar: false }`) | `SubmitResults` (on `timeline_starts` or button), displays `%Course.ScorePoints%`, `%Course.ScorePercent%%%`, `%Quiz.Attempts%`, `%Quiz.Passed%`, `%Learner.Name%`; `RetryQuiz` button; `ReviewQuiz` button; `JumpToSlide(s20-cert)` (conditional on `Quiz.Passed == true`); `ExitCourse` button. |
| `s20-cert` | `PrintSlide` button (verifies `Effect::Print { slide: "s20-cert", exclude: ["btn-print"] }`), `CompleteCourse { status: Completed }`, `ExitCourse`. |

---

## B. Automated Multi-Standard Runner Design

Create an integration test module **`apps/coursecraft-cli/tests/m27_reference_course.rs`** (or `crates/lms/tests/m27_reference_runs.rs` — `coursecraft-cli` has direct access to `coursecraft-course`, `coursecraft-logic`, `coursecraft-lms`, `coursecraft-publish`, `coursecraft-strict-lms`, and `coursecraft-lti` / `coursecraft-lti-server`!).

Each automated scenario drives:
1. `Course::from_json` + `course.validate()` (0 errors).
2. `build_package_standard_with_folders` for the target standard + `strict_lms::validate_package(...)` (0 manifest/packaging violations).
3. Real `coursecraft_logic::Player` driven step-by-step by scripted `Vec<Input>` (plus simulated `session_ms` ticks) wired to the real bridge (`LmsBridge`, `XapiBridge`, `LtiReporter` + local `coursecraft-lti-server`, or `WebResume`) wired to the strict test LMS (`StrictScorm12`, `StrictScorm2004`, `StrictLrs`, `StrictLtiPlatform`).
4. On every step: assert **zero `Effect::Warning`** (unless specifically testing a warning case) and **zero `strict_lms.violations()`**.
5. On completion / relaunch: assert exact final LMS state, exact interaction records/statements, exact `suspend_data` decode, and exact post-resume runtime state (`current_slide`, `visited`, `variables`, `object_states`, `answers`, `quiz_attempts`, `draw_seed`, expanded bank slide order, and re-shown feedback layer on answered locked question).

---

## B. Concrete Automated Scenarios (42 Cases)

### B1. SCORM 1.2 Automated Runs (`examples/reference-course/`)

| Case | Path / Variation | Scripted Actions & Assertions |
|---|---|---|
| **B1.1** | Full Pass (100%) | Walk Scenes 1–3 triggering all 19 events & 26 actions, answer all 12 graded+survey + 3 drawn bank questions correctly, `SubmitResults`, `PrintSlide`, `ExitCourse`. Assert: `0` violations, `lesson_status == "passed"`, `score.raw == "100"`, `score.min == "0"`, `score.max == "100"`, `interactions._count == 15`, `exit == ""`, valid `session_time`, `LMSFinish` called once. |
| **B1.2** | Full Fail (below 70% passing) | Answer graded questions wrong (triggering `try_again_layer` on `s07-q-mc`, exhausting attempts, partial credit `30%` on `s08-q-mr`), `SubmitResults`, `ExitCourse`. Assert: `0` violations, `lesson_status == "failed"`, `score.raw == "<70"`, all interactions recorded with `result == "wrong"` (surveys `"neutral"`), `exit == ""`. |
| **B1.3** | Partial -> Suspend -> Relaunch -> Resume -> Pass | Complete Scene 1 + first 5 questions (`s07`–`s11`, locking `s11`), close session (`pagehide` -> `finish_session`). Assert `lesson_status == "incomplete"`, `exit == "suspend"`, `suspend_data.len() <= 4096`. Relaunch into `StrictScorm12::relaunch()`: assert `entry == "resume"`, `Player` resumes on `s11-q-seq` with its outcome feedback layer re-shown, all user variables/states/visited slides/bank `draw_seed` match pre-suspend exactly, `interactions._count` continues appending from `5` -> `15`, finish with `lesson_status == "passed"`. |
| **B1.4** | Fail -> RetryQuiz -> Bank Redraw -> Pass -> ReviewQuiz -> ExitReview | Fail attempt 1 (`Quiz.Attempts == 1`, `lesson_status == "failed"`), click `RetryQuiz` (verify new `draw_seed` & cleared `answers`, variables preserved, `s15` freeform reset), pass attempt 2 (`Quiz.Attempts == 2`, `lesson_status == "passed"`, `score.raw == "100"`), enter `ReviewQuiz` (walk answered slides verifying locked questions & restored `review_answers`, `Next` past last exits review), `ExitCourse`. Assert `0` violations, interactions appended across both attempts with `_a1`/`_a2` IDs. |
| **B1.5** | High-Water Pass Protection (Pass -> Retry -> Fail / Lower Pass) | Pass attempt 1 with `90%` -> `SubmitResults` -> `RetryQuiz` -> fail with `20%` -> `SubmitResults` -> `RetryQuiz` -> pass with `75%` -> `SubmitResults` -> `RetryQuiz` -> pass with `100%` -> `SubmitResults`. Assert LMS stays `"passed"` throughout, ignores `20%` and `75%`, updates to `100%`; `0` violations. |
| **B1.6** | Completion-only course (no `quiz` block) | Run companion course without `course.quiz`, trigger `CompleteCourse { status: Completed }` + `ExitCourse`. Assert `lesson_status == "completed"`, NO `score.*` written, `exit == ""`, `0` violations. |
| **B1.7** | Suspend data overflow (> 4096 chars) graceful degradation | Populate long text variable (`v_input_text`) until `encode_suspend` > 4096 bytes. Assert `LmsBridge` emits 1 warning and skips `LMSSetValue("cmi.suspend_data", ...)`, `StrictScorm12` records `0` violations, and scoring/completion still succeed. |
| **B1.8** | Resume on corrupted `suspend_data` & post-completion relaunch | (a) Seed LMS with `"cc1:CORRUPT!!!"` -> bridge warns and starts fresh at `s01-intro` (`0` violations). (b) Relaunch after `passed` with `suspend_data` intact -> lands on `s20-cert`, does NOT reset status to `incomplete`, `0` violations. |

### B2. SCORM 2004 Automated Runs (`examples/reference-course/`)

| Case | Path / Variation | Scripted Actions & Assertions |
|---|---|---|
| **B2.1** | Full Pass (4th Ed.) | Full walkthrough -> `0` violations, `completion_status == "completed"`, `success_status == "passed"`, `score.raw == "100"`, `score.scaled == "1"`, `exit == "normal"`, ISO `session_time`, all 15 interactions (with `.description`, `true`/`false`, `[,]`, `[.]`, `{case_matters=true}`, `Freeform` as `other`) verified readable via `GetValue`. |
| **B2.2** | Full Fail (4th Ed.) | Fail quiz -> `0` violations, `completion_status == "completed"`, `success_status == "failed"`, `score.scaled == "0.3"` (plain decimal, no exponent), interactions `.result == "incorrect"` (surveys `"neutral"`), `exit == "normal"`. |
| **B2.3** | Partial -> Suspend -> Resume -> Pass (including >4096 char suspend data) | Store 6,000-byte text variable (between 4,096 and 64,000) -> suspend -> relaunch -> verify `cmi.suspend_data` persisted and decoded cleanly under 4th Ed. (`0` violations), complete & pass. |
| **B2.4** | SCORM 2004 3rd Ed. strict suspend limit (4,000 chars) | Run same 6,000-byte state against `StrictScorm2004(Third)` -> assert `StrictScorm2004` flags `406` if >4,000 written, verifying 3rd vs 4th ed. boundary behavior explicitly. |
| **B2.5** | Completion-only (no quiz) leaves `success_status == "unknown"` | Run non-quiz course -> `CompleteCourse(Completed)` -> `ExitCourse`. Assert `completion_status == "completed"`, `success_status` NEVER written (remains `"unknown"`), `exit == "normal"`, `0` violations. |
| **B2.6** | Fail -> Retry -> Pass & High-Water Protection | Fail attempt 1 (`failed`) -> retry -> pass attempt 2 (`85%` -> `scaled = "0.85"`, `passed`) -> retry -> fail (`10%`, ignored) -> relaunch (with LMS returning `score.raw = "85.0"`) -> pass `75%` (ignored) -> pass `95%` (`scaled = "0.95"`). Assert `0` violations. |
| **B2.7** | Multi-attempt `try_again_layer` + unlimited attempts (`s09-q-tf`) + partial scoring | Submit wrong on `s07-q-mc` (attempt 1 -> `try_again_layer`, `attempts_left: Some(1)`), hide layer (clears selection), submit partial-credit choice (attempt 2 -> `PerAnswer` points awarded). Submit wrong 3× on `s09-q-tf` (`attempts: 0`), then right on 4th. Assert all interaction records sequential (`_a1`, `_a2`, ...) and `0` violations. |
| **B2.8** | `cmi.location` > 1000 chars & `cmi.interactions._count` corrupt | Slide ID > 1000 chars skipped with warning (`0` violations); corrupt `_count = "nan"` disables interaction writes with 1 warning while score/completion/suspend still succeed (`0` violations). |

### B3. cmi5 & xAPI 1.0.3 Automated Runs (`examples/reference-course/`)

| Case | Path / Variation | Scripted Actions & Assertions |
|---|---|---|
| **B3.1** | cmi5 Full Pass (default mastery vs LMS `masteryScore: 0.8`) | Run with `masteryScore: 0.8`, score `100%`: assert `0` violations, verbs `[initialized, answered×15, passed, completed, terminated]`, `masteryscore == 0.8` on `passed` ONLY, `result.score.scaled == 1.0`, `cmi5` & `moveon` categories exact, `returnURL` captured. |
| **B3.2** | cmi5 LMS `masteryScore` overrides course `passing_percent` | Course `passing_percent: 70`, LMS `masteryScore: 0.9`, learner scores `80%`: bridge MUST emit `failed` (since `0.8 < 0.9`) with `masteryscore: 0.9` + `completed`, `0` violations! |
| **B3.3** | cmi5 Fail -> Relaunch -> Resume -> Pass (across 2 sessions!) | Session 1 fails (`initialized`, `answered...`, `failed`, `completed`, `terminated`, `progress.completed = true, passed = false`). Session 2 relaunches with new `sessionid` & `fetch` token, resumes, retries quiz, passes (`initialized`, `answered...`, `passed`, `terminated` — NO second `completed`!). Assert `0` violations. |
| **B3.4** | cmi5 Fail -> Retry -> Pass in ONE session (cmi5 §9.3.1 guard) | Attempt 1 fails (`failed` emitted), learner clicks `RetryQuiz` in the SAME session and passes (`Score(100, Passed)`): verify `XapiBridge` does NOT emit `passed` after `failed` in the same session (per `lrs.rs:767` `failed_this_session`), so `StrictLrs` records `0` violations! |
| **B3.5** | cmi5 Partial -> Suspend -> Resume -> Pass | Pause mid-course (including after opening/closing a lightbox — **BUG-A9**!), `finish_session` -> relaunch with new fetch token -> `GET coursecraft.progress` restores exact state (slide, vars, states, visited including lightbox, bank `draw_seed`) -> finish & pass -> `0` violations. |
| **B3.6** | cmi5 `Browse` and `Review` modes | Run full pass script under `launchMode: "Browse"` and `"Review"`. Assert ONLY `[initialized, terminated]` emitted, zero `PUT progress`, `reporting_lost() == false`, `0` violations. |
| **B3.7** | cmi5 5xx retry storm + 401 mid-session lost reporting | Inject 3× HTTP 500 on `/statements` -> verify exponential backoff & zero dropped statements -> inject HTTP 401 -> verify `reporting_lost() == true` and zero post-401 requests -> `0` violations. |
| **B3.8** | cmi5 handshake timeout (`HANDSHAKE_TIMEOUT_MS = 8000`) & reused fetch URL | Hold `POST /fetch` past 8000ms -> starts `Fresh`, `reporting_lost() == true`, late response ignored. Second sub-case: reused fetch -> `error-code: "1"`, `reporting_lost() == true`. |
| **B3.9** | Plain xAPI Pass, Fail, Resume, & Relaunch (with and without `registration`) | Run `Flavor::Xapi` through pass, fail, partial+resume, and relaunch on same registration + launch with `registration: None`. Assert `0` violations, zero `cmi5`/`moveon` categories, exact `answered` survey/graded shapes. |
| **B3.10** | xAPI completion-only course (no quiz) with LMS `masteryScore` | Run non-quiz course under cmi5 with `masteryScore: 0.8`: assert `completed` ONLY (neither `passed` nor `failed`, per `triggers.md:882`), `0` violations. |

### B4. LTI 1.3 & Standalone Web Automated Runs (`examples/reference-course/`)

| Case | Path / Variation | Scripted Actions & Assertions |
|---|---|---|
| **B4.1** | LTI 1.3 Full Pass end-to-end (`Player` -> `LtiReporter` -> `coursecraft-lti-server` -> `StrictLtiPlatform`) | Full OIDC login -> launch -> walk reference course to 100% pass -> POST `/lti/score` -> verify `StrictLtiPlatform` has `0` violations and `scoreGiven == 100, comment == "passed"`. |
| **B4.2** | LTI 1.3 Full Fail -> Retry -> Pass (same session) | Attempt 1 fails (`30%` -> AGS `scoreGiven: 30, comment: "failed"`), retry & pass (`100%` -> AGS `scoreGiven: 100, comment: "passed"`), retry & fail (`10%` -> suppressed by `LtiReporter` high-water!). Assert `0` violations. |
| **B4.3** | LTI 1.3 Completion-only (no quiz) | Non-quiz course emits `Completion(Completed)` -> AGS payload omits `scoreGiven`/`scoreMaximum`, sets `Completed`/`FullyGraded`, `0` violations. |
| **B4.4** | LTI 1.3 Relaunch always starts fresh & `?name=` populates `Learner.Name` | Launch with `name: "Ada Lovelace"`, complete slide 1, relaunch -> starts fresh at `s01-intro` (`0` suspend data) with `%Learner.Name% == "Ada Lovelace"`. |
| **B4.5** | LTI 1.3 Transient 502 retry succeeds vs double failure triggers `reporting_lost` | Inject 1× `502` from platform -> retry succeeds (`204`, `lost == false`). Inject 2× `502` -> `lost == true`. |
| **B4.6** | Standalone Web: Full Pass, Fail, & `localStorage` Resume (`WebResume<MemoryStorage>`) | Walk half of reference course -> `pagehide` persists `cc1:` under `coursecraft:resume:reference-course` -> relaunch -> resumes exact slide/vars/states/answers/bank `draw_seed` -> finishes pass. |
| **B4.7** | Standalone Web: Start Over & storage-quota/corrupt resilience | Trigger Start Over -> clears storage, restarts fresh at `s01-intro`. Seed corrupt `cc1:bad` or failing `MemoryStorage` (throws quota error) -> never panics, plays fresh cleanly. |
| **B4.8** | Cross-standard runtime parity check | Assert the `Player` effect stream on `examples/reference-course/pass.inputs.json` is **100% identical** across `coursecraft-cli simulate --strict`, SCORM 1.2, SCORM 2004, cmi5, xAPI, LTI 1.3, and Web! |

### B5. Runtime Edge-Case & Cross-Feature Stress Scenarios (`crates/logic/tests/m27_reference_edges.rs`)

| Case | Variation | Steps & Expected Result |
|---|---|---|
| **B5.1** | Lightbox over video with cue points (**BUG-B1** & **BUG-B2**) | Play video past cue `c1`, open lightbox `s06c-lightbox`, close lightbox -> assert `c1` stays fired (does NOT re-fire on resume). Open lightbox again and trigger `JumpToSlide` -> assert NO spurious `Media { Play }` emitted before `Media { Stop }`. |
| **B5.2** | `NavMode::Restricted` & `Locked` + `NextRequirement::TimelineEnds` & `Variable` | Verify `GoTo` to unvisited slide warns in `Restricted`, `GoTo` to visited works, `GoTo` to any slide warns in `Locked`, while author `JumpToSlide` ALWAYS succeeds. Verify `Next` blocks until `duration_ms` on `s01-intro` and until `v_ready == true` on `s02-layers`, and stays unlocked on revisit. |
| **B5.3** | Playback speed (`0.5x`, `1.5x`, `2.0x`) + quarter-ms carry + timer + Lottie + pause/seek | Run 1000× `Tick { ms: 1 }` at `1.5x` -> exact `1500ms` advance on base clock, timer, and Lottie; `PlayerPause` freezes all clocks; `PlayerSeek` forward fires crossed cues in order, backward re-arms cues without undoing vars/states. |
| **B5.4** | Layer `allow_seek` (`Auto`/`Yes`/`No`) & `revisit` (`Auto`/`Reset`/`Resume`) + `hide_when_timeline_ends` | Verify `allow_seek: No` refuses seek with warning; `allow_seek: Yes` seeks layer clock; `hide_when_timeline_ends` fires `timeline_ends` then hides layer and resumes base media; `revisit: Resume` restores layer clock except auto-hiding ended layer restarts at 0. |
| **B5.5** | `revisit: Reset` vs `Resume` on slides + `SlideTarget::LastVisited` history cap (50) | Leave `s02-layers` (`resume`) mid-clock with layer open -> return -> clock & visible layer restored. Leave `s03` (`reset`) -> return -> states reset to `initial_state`. Push 60 jumps -> `visit_history` capped at 50; `LastVisited` pops without pushing current. |
| **B5.6** | Loop guard (1024 events) & divide-by-zero / NaN hostility | Trigger mutually recursive `variable_changes` loop -> halts at 1024 events with 1 `Effect::Warning`, course stays alive. `Divide` by `0.0`, non-finite `HotspotClicked(NaN, Inf)`, `ObjectDropped(NaN, 0)` -> warn, zero panics, zero state corruption. |
| **B5.7** | All 6 `VarOp`s + var-to-var + `%var%` 64 KiB cap & split-run rule | Verify `assign`/`add`/`subtract`/`multiply`/`divide`/`toggle` with constants & `{ "variable": ... }`; verify `%%` escape, unknown `%nope%` literal, and recursive text expansion capped cleanly at 65,536 bytes. |
| **B5.8** | Question bank determinism across seeds & post-resume `ReviewQuiz` contract | Verify `seed = 0` vs `seed = 42` produce different draws; resuming from `suspend_data` preserves the exact drawn slides of `seed = 42`; `ReviewQuiz` after resume locks all answered questions cleanly without crashing on empty `review_answers`. |

### B. Files / Crates to Touch & Ordered Implementation Steps

- **Step B-1 (1 session):** Fix runtime/publish bugs **BUG-B1** (`crates/logic/src/lib.rs`, `run.rs`), **BUG-B2** (`crates/logic/src/run.rs`), and **BUG-B3** (if not done in A-3, `crates/publish/src/lib.rs`); add unit tests **B5.1–B5.8** in `crates/logic/tests/m27_reference_edges.rs`.
- **Step B-2 (1 session):** Author `examples/reference-course/` (`course.json`, tiny deterministic `media/` assets, `README.md`, `pass.inputs.json`, `fail.inputs.json`, `completion-only.json`) covering 100% of the matrix above; verify `coursecraft-cli validate` (0 errors) and `coursecraft-cli simulate --strict` (0 warnings).
- **Step B-3 (1 session):** Implement the shared automated multi-standard runner harness in `apps/coursecraft-cli/tests/m27_reference_course.rs` and wire **B1.1–B1.8** (SCORM 1.2) + **B2.1–B2.8** (SCORM 2004).
- **Step B-4 (1 session):** Add **B3.1–B3.10** (cmi5 & xAPI 1.0.3) automated runs to `apps/coursecraft-cli/tests/m27_reference_course.rs`.
- **Step B-5 (1 session):** Add **B4.1–B4.8** (LTI 1.3 via local `coursecraft-lti-server` + `StrictLtiPlatform`, Web via `WebResume`, and cross-standard parity assertion) and register `reference-course` in `xtask/src/main.rs` `package-example` (optional smoke publish).

---

# Part C: PowerPoint Import Round 3 (Animations & Style-Less Tables)

## C1. PowerPoint Animations (`p:timing`) Import Design

### Current State & Required Infrastructure Fixes
- Today `crates/pptx/src/lib.rs:1325-1328` skips all `<p:timing>` with a single `SkipKind::Animation` note.
- **Target resolution fix (**BUG-C2**):** In `crates/pptx/src/lib.rs:1030-1031, 2531-2556, 2755-2841`:
  1. Change `SlideTracker.pptx_ids` from `HashMap<String, String>` to `HashMap<String, Vec<String>>` so multi-object imports (e.g., multi-subpath custom geometry) keep every emitted CourseCraft `object.id`.
  2. Call `record_pptx_id` for `p:cxnSp` (`:2833`) as well as `p:sp`, `p:pic`, and `p:graphicFrame` (`a:tbl`).
  3. In `process_group` (`:2762-2832`), snapshot `cx.objects.len()` before recursing and map the group's `nvGrpSpPr/cNvPr@id` to all newly emitted child object IDs, so a PowerPoint animation targeting a group `spid` applies uniformly to every flattened child of that group.

### OOXML Timing Tree Traversal (`crates/pptx/src/timing.rs` — new module)
Parse `<p:timing>` under `slide_tree` (bounded by existing `limits.max_xml_depth` and a cap of `MAX_ANIM_NODES = 1000` per slide):
1. Locate `<p:tnLst>` -> root `<p:par>/<p:cTn nodeType="tmRoot">/<p:childTnLst>`:
   - **Main sequence:** `<p:seq>/<p:cTn nodeType="mainSeq">/<p:childTnLst>` holds ordered click-step `<p:par>` wrappers; inside each click-step are with-previous `<p:par>` groups; inside those are individual effect `<p:par>` nodes carrying `<p:cTn presetClass="..." presetID="..." presetSubtype="..." nodeType="clickEffect|withEffect|afterEffect" dur="..." accel="..." decel="..." repeatCount="...">`.
   - **Interactive sequences:** `<p:seq>/<p:cTn nodeType="interactiveSeq">` where `<p:stCondLst>/<p:cond evt="onClick"><p:tgtEl><p:spTgt spid="TRIGGER_ID"/>` defines an explicit **"trigger on click of object X"** sequence!
2. For each effect `<p:cTn>`:
   - **Target:** descend into `<p:childTnLst>` (`p:set`, `p:animEffect`, `p:anim`, `p:animMotion`, `p:animScale`, `p:animRot`, `p:animClr`) -> `<p:cBhvr>/<p:tgtEl>/<p:spTgt spid="N"/>`. If `<p:txEl><p:pRg>` (paragraph-level animation) is present, apply to the whole object and emit an informational report note (`"Paragraph-level animation applied to whole text box on <name>"`).
   - **Duration (`dur`):** read from effect `cTn@dur`, else max child behavior `cBhvr/cTn@dur` (ignoring `"indefinite"` and `0`/`1`ms visibility toggles on `<p:set>`). Default to `500` ms if omitted; clamp to `1..=60_000` ms (`crates/course/src/validate.rs:2040-2057`).
   - **Delay:** sum `stCondLst/cond@delay` (ignoring `"indefinite"`) on the effect `cTn` and its enclosing with-previous group `cTn`; clamp non-negative `u32` (cap `86_400_000` ms).
   - **Easing:** inspect child `cBhvr/cTn@accel` and `@decel` (0..=100_000): `accel > 0 && decel > 0` -> `Easing::EaseInOut`; `accel > 0` -> `Easing::EaseIn`; `decel > 0` -> `Easing::EaseOut`; both `0` -> `Easing::Linear`; omitted -> `Easing::EaseOut` (CourseCraft default).
   - **Repeat (`repeatCount`):** `"1000"` (or omitted) = 1× (no note). Any finite repeat `> 1000` (e.g. `"2000"`, `"3000"`) or `"indefinite"`: CourseCraft `Animation` & `MotionPath` (`crates/course/src/model.rs:1344-1440`) have no `repeat` field — play once and emit a `SkipKind::Animation` diagnostic (`"Animation repeat (repeatCount=...) not supported on <name>; plays once"`).

### Complete PowerPoint Preset Mapping Table (`presetClass` + `presetID` + `presetSubtype`)

#### Direction Subtypes (`presetSubtype` bitmask in ECMA-376 / Office)
- Standard cardinal directions: `1` = `Top` (Up), `2` = `Right`, `4` = `Bottom` (Down), `8` = `Left`.
- Diagonal subtypes (`3` Top-Right, `6` Bottom-Right, `9` Top-Left, `12` Bottom-Left): map to nearest cardinal (`Top`/`Bottom`) and emit a non-fatal note (`"Diagonal animation direction mapped to cardinal on <name>"`).
- **Direction polarity check:** In CourseCraft `crates/logic/src/animation.rs:102-118`, `FlyIn { from }` moves FROM `from` on entrance and TOWARD `from` on exit. In PowerPoint `Fly In` (`presetID=2`): `subtype=4` ("From Bottom") -> `Direction::Bottom`, `8` ("From Left") -> `Direction::Left`, `2` ("From Right") -> `Direction::Right`, `1` ("From Top") -> `Direction::Top`. For `Wipe` (`presetID=22`): `subtype=4` ("From Bottom") -> `Direction::Bottom`, `8` ("From Left") -> `Direction::Left`, `2` ("From Right") -> `Direction::Right`, `1` ("From Top") -> `Direction::Top`. Verify both entrance and exit polarity against `animation.rs:102-219`!

#### 1. Entrance (`presetClass="entr"`) & Exit (`presetClass="exit"`) Presets

| PowerPoint Effect | `presetID` | CourseCraft Mapping | Notes / Diagnostics |
|---|---:|---|---|
| **Appear / Disappear** | `1` | `timing.start_ms` / `timing.end_ms` only (`entrance: None` / `exit: None`) | Instant visibility on timeline — exact match, no fade blur. |
| **Fly In / Fly Out** | `2` | `AnimEffect::FlyIn { from }` | Direction from `presetSubtype` (`1` Top, `2` Right, `4` Bottom, `8` Left; default `Bottom`). |
| **Fade** | `10` | `AnimEffect::Fade` | Exact match. |
| **Wipe** | `22` | `AnimEffect::Wipe { from }` | Direction from `presetSubtype` (default `Bottom`). |
| **Zoom** | `23` | `AnimEffect::Zoom` | Exact match. |
| **Basic Zoom / Grow & Turn / Swivel / Bounce / Center Revolve** | `53`, `37`, `24` | `AnimEffect::Zoom` (fallback for `53`) / or **Skip** with note for exotic physics (`Bounce`/`Swivel`) | Map `53` (Grow & Turn) -> `Zoom` with note; report `Bounce`/`Swivel`/`Random Bars`/`Wheel` (`3`, `4`, `5`, `6`, `7`, `8`, `9`, `11`, `12`, `14`, `16`, `17`, `18`, `19`, `20`, `21`, `25`, `27`+) -> fallback to `AnimEffect::Fade` (preserving timing!) + report `SkipKind::Animation` note naming the exact unsupported preset ID & object so the object's timing is NOT broken while the author knows the visual effect approximated to Fade! |
| **Float In / Float Out** | `42` (Float Up/Down), `26` (Rise Up / Sink Down), `47` (Descend), `39` (Ascend) | `AnimEffect::Float { from }` | `42`/`26` -> `Bottom` (entr) / `Top` or subtype; `47` -> `Top`; `39` -> `Bottom`. |
| **Peek In / Peek Out** | `12` | `AnimEffect::Wipe { from }` | Closest clean-room match (`Wipe` clips at frame edge) + diagnostic note. |
| **Split** | `16` | `AnimEffect::Wipe { from }` (or `Fade`) + diagnostic note | Report `"Entrance preset Split (id=16) approximated as Fade on <name>"`. |

*(If an object has >1 entrance or >1 exit in `p:timing`, keep the first entrance and last exit, reporting the extra one as skipped with `SkipKind::Animation`.)*

#### 2. Emphasis (`presetClass="emph"`) Presets
CourseCraft `schema_version: 1` (`crates/course/src/model.rs:974-993`) has **no emphasis animation field** (`Object` has `entrance`, `exit`, `motion_paths` only). To keep schema v1 compatible without breaking older builds:
- Every `presetClass="emph"` node (e.g., `1` Fill Color, `2` Font, `3` Font Color, `4` Font Size, `5` Font Style, `6` Grow/Shrink, `7` Line Color, `8` Spin, `9` Transparency, `10` Bold Flash, `14` Pulse, `15` Color Pulse, `26` Teeter, `27` Flicker, `32` Wave, `33` Blink) MUST be reported explicitly via `tracker.report_skip(SkipKind::Animation, format!("Emphasis animation ({preset_name}, presetID={id}) skipped on {obj_name}"))` — **never silently dropped**, and **never mis-mapped as an entrance or exit** that would hide the object!
- Its timeline slot duration still advances the running cursor for subsequent `afterEffect` animations so downstream timing stays accurate.

#### 3. Motion Paths (`presetClass="path"`)
- Parse `<p:animMotion path="..." origin="layout|parent" pathEditMode="relative|fixed" ptsTypes="...">` inside `<p:childTnLst>`:
  - Parse OOXML mini-path grammar (`M x y`, `L x y`, `C x1 y1 x2 y2 x y`, `Z`, `E`) into polyline `Vec<Point>` (sampling cubic Bézier `C` segments adaptively into 8–12 points, capped at `MAX_MOTION_PATH_POINTS = 100` per `validate.rs:2092`!).
  - Scale normalized coordinates (`x * slide_w`, `y * slide_h`, because OOXML `p:animMotion` expresses `path` as fractions of slide width/height!).
  - Set `MotionPath { id: format!("pptx_path_{n}"), points, duration_ms, easing, relative: true }` (when first point is `(0, 0)` and `pathEditMode != "fixed"`, else compute relative offsets from `(0, 0)` so `validate.rs:2125` emits zero warnings!).
  - Attach a trigger to play it (since CourseCraft motion paths only run via `Action::MoveOnPath`, `crates/course/src/model.rs:3973-3976`):
    - At `t == 0` -> `Event::TimelineStarts { layer: None }` -> `Action::MoveOnPath { object, path }`.
    - At `t > 0` -> `Event::TimelineReaches { ms: t, layer: None, cue: None }` -> `Action::MoveOnPath { object, path }`.
    - In an `interactiveSeq` triggered by click on `spid_trigger` -> `Event::Click { object: trigger_obj_id }` -> `Action::MoveOnPath { object, path }`!

### Scheduling Rules: On-Click vs With-Previous vs After-Previous & Slide Duration
Because CourseCraft has a continuous slide timeline (`Slide.duration_ms`) and no `StartEntrance` trigger action:
- **Timeline cursor algorithm (for `mainSeq`):**
  - Maintain `cursor_ms: u32 = 0` and `last_group_start_ms: u32 = 0`, `last_end_ms: u32 = 0`.
  - **`withEffect`** (With Previous): `start_ms = last_group_start_ms.saturating_add(delay_ms)`.
  - **`afterEffect`** (After Previous): `start_ms = last_end_ms.saturating_add(delay_ms)`.
  - **`clickEffect`** (On Click):
    - First `clickEffect` after auto effects (or each click step): advance `start_ms = last_end_ms.saturating_add(CLICK_STEP_GAP_MS)` (e.g. `500` ms stagger, plus `delay_ms`) so objects stagger deterministically on the timeline instead of all colliding at `0ms`, AND record a single concise import note (`"On-click animation steps converted to staggered timeline timings (500 ms step)"`).
    - For `interactiveSeq` (explicit `evt="onClick"` on a specific shape `X`):
      - If the effect is a **Motion Path**: wire directly to `Trigger { when: Event::Click { object: X }, action: Action::MoveOnPath { object, path } }` (exact interactive parity!).
      - If the effect is an **Entrance / Exit**: CourseCraft cannot trigger entrance/exit animations on click without a layer, so set `initial_state: "hidden"` (for entrance) and wire `Trigger { when: Event::Click { object: X }, action: Action::ChangeState { object, state: "normal" } }` (or `"hidden"` for exit) + report note that click-triggered animation visibility was mapped to state change.
  - **Updating `Object.timing` & `Slide.duration_ms`:**
    - Entrance sets `obj.timing.start_ms = start_ms` and `obj.entrance = Some(...)`.
    - Exit sets `obj.timing.end_ms = Some(start_ms.saturating_add(duration_ms))` and `obj.exit = Some(...)`. (Guard: if `end_ms <= obj.timing.start_ms`, bump `end_ms = obj.timing.start_ms + duration_ms` so `validate.rs:1862` NEVER rejects!)
    - Extend `slide.duration_ms = slide.duration_ms.max(max_effect_end_ms.saturating_add(500))` (clamped to `MAX_TIMELINE_DURATION_MS`) so no imported entrance/exit/motion-path is clipped by the default `5000ms` slide duration!
  - **Existing skip-report test compatibility (`crates/pptx/src/tests.rs:1014-1020`):** An empty `<p:timing><p:tnLst/></p:timing>` with 0 animations has nothing to skip; update `test_all_skip_kinds_recorded_in_report` to include an actual skipped emphasis animation (`presetClass="emph" presetID="8"`) so `SkipKind::Animation` continues to be genuinely exercised!

---

## C2. Fidelity Deck Animated Slide Plan (Slide 20)

Because `python-pptx` has no API for `<p:timing>`, add a self-contained reproducible Python patch script **`crates/pptx/tests/fidelity/add_animated_slide.py`** (standard library `zipfile` + `xml.etree.ElementTree` only — zero external installs) that appends **Slide 20** (after existing Slide 19 hidden slide, leaving Slides 1–19 byte-untouched!) into `fidelity.pptx`:
- **Slide 20 XML (`ppt/slides/slide20.xml`, registered in `ppt/presentation.xml`, `ppt/_rels/presentation.xml.rels`, `[Content_Types].xml`):**
  1. Title box (`spid=2`): `"Slide 20: Animations & Motion Paths"` (no animation; visible from `t=0`).
  2. Blue rect (`spid=3`): **Entrance Fade** (`presetClass="entr"`, `presetID="10"`, `dur="600"`, `nodeType="withEffect"`, `delay="0"`).
  3. Green rounded rect (`spid=4`): **Entrance Fly In from Left** (`presetClass="entr"`, `presetID="2"`, `presetSubtype="8"`, `dur="500"`, `nodeType="afterEffect"`, `delay="200"` -> `start_ms=800`).
  4. Orange shape (`spid=5`): **Entrance Wipe from Bottom** (`presetClass="entr"`, `presetID="22"`, `presetSubtype="4"`, `dur="400"`, `nodeType="withEffect"`, `delay="100"` -> `start_ms=900`) **AND Exit Fly Out to Right** (`presetClass="exit"`, `presetID="2"`, `presetSubtype="2"`, `dur="500"`, `nodeType="afterEffect"` -> `end_ms=2500`).
  5. Purple circle (`spid=6`): **Entrance Zoom** (`presetID="23"`, `dur="500"`, `nodeType="clickEffect"`) + **Motion Path** (`presetClass="path"`, `<p:animMotion path="M 0 0 L 0.25 0.0 L 0.25 0.20 E" origin="layout" pathEditMode="relative"/>`, `dur="1000"`, `nodeType="afterEffect"`).
  6. Red star (`spid=7`): **Entrance Float In** (`presetID="42"`, `dur="500"`) + **Emphasis Spin** (`presetClass="emph"`, `presetID="8"`, `dur="800"`, `repeatCount="2000"` — tests both emphasis skip diagnostic and repeat skip diagnostic!).
- **Verification in `apps/coursecraft-cli/tests/pptx_fidelity.rs`:**
  - Keep existing `Slide1.png`..`Slide18.png` pixel assertions intact (now `slides.len() == 19` visible slides since Slide 19 is hidden and Slide 20 is visible).
  - For Slide 20, do **NOT** rely on `at_ms: None` (which ticks to `slide.duration_ms` where exit-animated `spid=5` has already vanished!). Instead:
    1. Assert structural import: `spid=3` has `Fade` @ `0ms`, `spid=4` has `FlyIn(Left)` @ `800ms`, `spid=5` has `Wipe(Bottom)` + `FlyIn(Right)` exit, `spid=6` has `Zoom` + `MotionPath` + `MoveOnPath` trigger, `spid=7` has `Float` + `report` contains `SkipKind::Animation` mentioning Spin/Emphasis.
    2. Drive `Player` on Slide 20 and call `render_slide_rgba` at multiple timestamps (`t = 300ms` mid-fade, `t = 1050ms` mid-fly-in, `t = 1500ms` all entered & `spid=5` visible, `t = 2250ms` `spid=5` mid-exit, `t = 2600ms` `spid=5` gone, `t = motion_end` `spid=6` resting at `(+0.25*1280, +0.20*720)`) and assert pixel deltas at each object's expected location!
    3. Export `Slide20.png` from PowerPoint (static/authoring frame) when regenerated, and compare `RenderMode::Authoring` (which ignores timing and draws all objects at rest) against `Slide20.png`.

---

## C3. Tables Without a Style Design (**BUG-C1**)

### Exact Root Cause (`crates/pptx/src/lib.rs:3167-3351, 3416-3443`)
1. `table_side_outline` (`:3416-3443`) returns `Option<Outline>`, returning `None` BOTH when `<a:lnL>` is absent AND when `<a:lnL><a:noFill/></a:lnL>` explicitly disables the border.
2. `process_table` (`:3174-3182`) sets `table_style = None` when `<a:tableStyleId>` is absent (or unrecognized), leaving `style_border = None` (`:3342-3347`) and therefore `TableBorders { outer: None, inner: None }` (`:3348-3351`).
3. PowerPoint's actual behavior (verified directly in `crates/pptx/tests/fidelity/fidelity.pptx` Slide 14 Table 2 `<a:tblPr firstRow="0" bandRow="0"/>` vs `Slide14.png`):
   - **No `<a:tableStyleId>`** (and no `<a:tblStyleLst def="...">` overriding it, OR default fallback): PowerPoint renders a **1 pt solid black (`#000000`) outer AND inner grid**! (Width = `1.0 * (96.0 / 72.0) * layout.scale` slide px, i.e., `1.333` px on a standard 16:9 1280×720 slide — matching `parse_emu(12700)` used by `table_side_outline` for 1pt lines!)
   - **`{5940675A-B579-460E-94D1-54222C63F5DA}`** ("No Style, Table Grid"): 1 pt solid `#000000` outer & inner grid (update `builtin_table_style:3029` to scale 1pt consistently with `layout.scale`).
   - **`{2D5ABB26-0587-4C30-8999-92F81FD0307C}`** ("No Style, No Grid"): **explicitly borderless** (`outer: None, inner: None`) — MUST NOT receive the black fallback grid!
   - **Explicit `<a:lnL/R/T/B><a:noFill/></a:ln*>`:** Introduce a 3-state enum `SideLine { Solid(Outline), ExplicitNone, Inherited }` in `table_side_outline`. When `table_style` is `None`:
     - If any outer side has `Solid(o)` -> use `dominant_outline`; else if ALL outer sides are `ExplicitNone` -> `None`; else -> default 1pt `#000000` outline!
     - Same rule independently for inner sides!
   - **Bonus (`a:tcPr@anchor`):** At `crates/pptx/src/lib.rs:3297`, read `props.and_then(|p| p.attr("anchor"))` (`"t"` -> `Top`, `"ctr"` -> `Middle`, `"b"` -> `Bottom`) as fallback when `<a:bodyPr@anchor>` is unset, fixing vertical text centering in Slide 14 Table 2!

---

## C. Concrete Test Cases (24 Cases)

### C-Anim. PowerPoint Animation & Motion Path Import (`crates/pptx/src/m27_anim_tests.rs` & `apps/coursecraft-cli/tests/pptx_fidelity.rs`)

| Case | Variation | Input PPTX XML | Expected Result |
|---|---|---|---|
| **C1.1** | All 5 mapped entrance effects + directions | `<p:timing>` with `Fade` (10), `FlyIn` (2, subtypes `1/2/4/8`), `Wipe` (22, `1/2/4/8`), `Zoom` (23), `Float` (42/26). | Each object gets exact `AnimEffect` + `Direction` + `duration_ms`; `Course::validate()` 0 errors; 0 `SkipKind::Animation` reports. |
| **C1.2** | Appear (`presetID=1`) & Disappear (`presetClass="exit", presetID=1`) | Entrance Appear @ `delay=1000` and Exit Disappear @ `delay=3000`. | `timing.start_ms == 1000, entrance: None`; `timing.end_ms == Some(3000), exit: None`; `Player` hides object before 1000ms and at >=3000ms. |
| **C1.3** | All 5 mapped exit effects + polarity check | `presetClass="exit"` with Fade, Fly Out (`subtype=2` Right), Wipe (`subtype=8` Left), Zoom, Float Out. | `obj.exit == Some(Animation { .. })`, `timing.end_ms == Some(start + dur)`, `slide.duration_ms >= end_ms`; `Player::sample_object` animates exit cleanly and hides at `end_ms`. |
| **C1.4** | `withEffect` vs `afterEffect` vs `clickEffect` + delays | Chain: A (`withEffect`, delay 0, dur 500) -> B (`withEffect`, delay 200, dur 400) -> C (`afterEffect`, delay 100, dur 600) -> D (`clickEffect`, delay 0, dur 500). | `A.start_ms == 0`, `B.start_ms == 200`, `C.start_ms == 700` (max end 600 + 100), `D.start_ms == 1800` (1300 + 500 gap); `slide.duration_ms >= 2800`. |
| **C1.5** | Emphasis effects (`presetClass="emph"`) skipped cleanly with diagnostics | `Spin` (`id=8`), `Pulse` (`id=14`), `Grow/Shrink` (`id=6`) on visible objects, followed by `afterEffect` entrance on shape B. | Emphasized objects keep `entrance: None, exit: None, timing.start_ms: 0` (NOT hidden!); report contains 3 `SkipKind::Animation` entries naming preset & object; shape B's `afterEffect` still starts after emphasis duration. |
| **C1.6** | Motion path (`M`, `L`, `C`, `Z`, `E`) + `MoveOnPath` trigger | `<p:animMotion path="M 0 0 L 0.25 0 C 0.25 0.1 0.1 0.2 0 0.2 E" dur="1200"/>` at `t=0` and `t=1500`. | `obj.motion_paths.len() == 1`, scaled to `1280×720` pixels (`x=320`, `y=144`), `points.len()` in `2..=100`, `TimelineStarts` (at `t=0`) or `TimelineReaches(1500)` trigger with `Action::MoveOnPath`; `Player` moves object to `(0, 144)` at `t=1200`/`2700`. |
| **C1.7** | Interactive click-triggered sequence (`nodeType="interactiveSeq"`) | Shape B has motion path triggered by `evt="onClick"` on button A (`spid=10`); shape C has entrance triggered by click on A. | Generates `Trigger { when: Click("obj_A"), action: MoveOnPath("obj_B", "pptx_path_1") }` and `ChangeState` for C (`initial_state: "hidden"`); clicking A in `Player` starts B's motion path and reveals C. |
| **C1.8** | Group (`p:grpSp`) & Connector (`p:cxnSp`) animation targeting (**BUG-C2**) | `<p:spTgt spid="grp1"/>` targeting a group of 3 shapes; `<p:spTgt spid="cxn1"/>` targeting a connector. | All 3 flattened group children receive the Fade entrance & identical `timing.start_ms`; connector receives the Wipe entrance. |
| **C1.9** | Easing (`accel`/`decel`) & `repeatCount` handling | 4 effects with `(0,0)`, `(50000,0)`, `(0,50000)`, `(50000,50000)`; 2 effects with `repeatCount="3000"` and `repeatCount="indefinite"`. | Maps to `Linear`, `EaseIn`, `EaseOut`, `EaseInOut`; repeat > 1 plays once and emits `SkipKind::Animation` repeat diagnostic. |
| **C1.10** | Unsupported entrance preset fallback & diagonal direction | `Bounce` (`presetID=26`/exotic `presetID=37`) and `FlyIn` with `presetSubtype="6"` (Bottom-Right diagonal). | Exotic preset falls back to `Fade` (keeping `timing.start_ms`) + emits `SkipKind::Animation` note; diagonal maps to cardinal + note; 0 validation errors. |
| **C1.11** | Combined entrance + motion path + exit on the same object | Object enters `[500..1000]`, moves on path `[1200..2200]`, exits `[3000..3500]`. | All three fields populated (`start_ms=500`, `end_ms=3500`, `entrance`, `motion_paths[0]`, `exit`), valid under `Course::validate()`, verified in `Player` at 4 timestamps. |
| **C1.12** | Overlapping/conflicting exit <= start & multiple entrances on same object | Malformed PPTX where exit `delay=100, dur=200` precedes entrance `delay=1000`, plus a second entrance on same `spid`. | Importer clamps `end_ms > start_ms` (passes `Course::validate()`!), keeps first entrance and reports second as skipped; zero panics. |
| **C1.13** | Hostile / malformed `<p:timing>` resilience | Deeply nested `<p:par>` (>64 levels), `dur="999999999999"`, `delay="-500"`, `delay="NaN"`, invalid `spid="99999"`, huge motion path with 5,000 `L` segments, malformed `path="M foo bar"`. | Zero panics; invalid values clamped or safely skipped with report notes; motion path points capped <= 100; resulting course validates with 0 errors. |
| **C1.14** | Fidelity deck Slide 20 end-to-end visual + runtime test | Import updated `crates/pptx/tests/fidelity/fidelity.pptx`; verify Slide 20 structure, `Player` sampling, multi-timestamp `render_slide_rgba` pixel assertions, and `RenderMode::Authoring` vs `Slide20.png`. | All 19 visible slides pass similarity thresholds; Slide 20 dynamic frames verify entrance, motion path travel, and exit disappearance. |

### C-Tbl. PowerPoint Style-Less & Bordered Tables (`crates/pptx/src/m27_table_tests.rs` & `apps/coursecraft-cli/tests/pptx_fidelity.rs`)

| Case | Variation | Input PPTX XML | Expected Result |
|---|---|---|---|
| **C2.1** | Plain table with NO `<a:tableStyleId>` and NO `<a:ln*>` (**BUG-C1**) | `<a:tblPr firstRow="0" bandRow="0"/>` with 3×3 cells (same shape as `fidelity.pptx` Slide 14 Table 2). | `borders.outer == Some(#000000, ~1.33px Solid)` and `borders.inner == Some(#000000, ~1.33px Solid)`. |
| **C2.2** | Table with NO `<a:tblPr>` element at all | `<a:tbl>` directly containing `<a:tblGrid>` + `<a:tr>` (omits `<a:tblPr>`). | Still gets default 1pt `#000000` outer & inner grid. |
| **C2.3** | `"No Style, Table Grid"` GUID `{5940675A-B579-460E-94D1-54222C63F5DA}` | Explicit `tableStyleId` set to Table Grid GUID, no cell `<a:ln*>`. | 1pt `#000000` outer & inner grid, scaled by `layout.scale`. |
| **C2.4** | `"No Style, No Grid"` GUID `{2D5ABB26-0587-4C30-8999-92F81FD0307C}` | Explicit `tableStyleId` set to No Grid GUID, no cell `<a:ln*>`. | `borders.outer == None` and `borders.inner == None` (no black grid forced!). |
| **C2.5** | Explicit `<a:noFill/>` on cell border lines without a style | No `tableStyleId`, every cell has `<a:lnL/R/T/B><a:noFill/></a:ln*>`. | `borders.outer == None` and `borders.inner == None` (explicit noFill respected!). |
| **C2.6** | Partial explicit borders (outer red 2pt, inner omitted, no style) | No `tableStyleId`; outer edges have `<a:ln* w="25400"><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></a:ln*>`; inner edges omitted. | `borders.outer == Some(#FF0000, ~2.67px)` and `borders.inner == Some(#000000, ~1.33px)` (inner inherits default grid!). |
| **C2.7** | Styled table (`Medium Style 2 - Accent 1`) unaffected | `<a:tableStyleId>{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}</a:tableStyleId>`. | Keeps white 1pt style borders + accent header + banded rows (no regression on Slide 14 Table 1!). |
| **C2.8** | Unrecognized `tableStyleId` not in `tableStyles.xml` | `<a:tableStyleId>{DEADBEEF-0000-0000-0000-000000000000}</a:tableStyleId>`, no cell borders. | Falls back cleanly to default 1pt `#000000` grid instead of invisible borderless cells. |
| **C2.9** | Rendered pixel check & Slide 14 fidelity score improvement | Render imported Slide 14 Table 2 via `render_slide_rgba` and assert dark gridline pixels (`r<30, g<30, b<30`) appear on cell boundaries between colored cells; check `pptx_fidelity_pixel_similarity` Slide 14 score increases vs baseline `0.9843`. | Visible 1pt black gridlines rendered; Slide 14 similarity score improves and threshold in `pptx_fidelity.rs:108` can be tightened. |
| **C2.10** | `a:tcPr@anchor` vertical alignment + merged cells with default grid | Style-less 3×3 table with `gridSpan="2"`, `rowSpan="2"`, and `<a:tcPr anchor="ctr">` / `anchor="b"`. | Grid draws cleanly without bisecting merged cells (`paint_table` span skip); `cell.valign == Middle` / `Bottom`. |

### C. Files / Crates to Touch & Ordered Implementation Steps

- **Step C-1 (1 session):** Fix style-less tables (**BUG-C1**) + `SideLine` 3-state border parsing + `{2D5ABB26-...}` No-Grid GUID + `a:tcPr@anchor` in `crates/pptx/src/lib.rs:3004-3443`; add `crates/pptx/src/m27_table_tests.rs` (**C2.1–C2.10**).
- **Step C-2 (1 session):** Fix `SlideTracker.pptx_ids` (`HashMap<String, Vec<String>>`, `cxnSp`, `grpSp` mapping — **BUG-C2**) in `crates/pptx/src/lib.rs:1030-1031, 2531-2556, 2755-2841` and build `crates/pptx/src/timing.rs` for entrance/exit/emphasis parsing + timeline scheduling.
- **Step C-3 (1 session):** Add motion-path (`p:animMotion` `M/L/C/Z/E`) parsing, `interactiveSeq` click trigger wiring, `repeatCount`/easing handling, and slide duration extension to `crates/pptx/src/timing.rs`; update `crates/pptx/src/lib.rs:1325-1328` and `crates/pptx/src/tests.rs:1014-1020`.
- **Step C-4 (1 session):** Add `crates/pptx/src/m27_anim_tests.rs` covering **C1.1–C1.13** and update `docs/pptx-import.md` to document supported animations, presets, motion paths, and style-less table defaults.
- **Step C-5 (1 session):** Author `crates/pptx/tests/fidelity/add_animated_slide.py`, append Slide 20 to `fidelity.pptx`, update `manifest.md`, add multi-timestamp Slide 20 assertions (**C1.14**) and tightened Slide 14 table checks in `apps/coursecraft-cli/tests/pptx_fidelity.rs`.

---

# Part D: Editor Sweep, Tested to the Core

## D. Harness & Test Architecture

### Small Extensions to `crates/ui-egui/src/test_harness.rs` (Step D-1)
Keep all existing tests 100% unchanged and add 5 tiny, reusable helpers to `TestHarness`:
1. **Monotonic virtual clock (`pub time: f64` on `TestHarness`):** `frame()` passes `time: Some(self.time)` and increments `self.time += 1.0 / 60.0` (plus `advance_time(secs: f64)`), enabling real tests of double-click, 300ms deferred audit debounce (`lib.rs:3602`), and 5s status-bar expiry.
2. **`double_click_at(&mut self, pos: Pos2)`:** sends two primary clicks separated by `0.05s` so egui registers `response.double_clicked()`.
3. **`click_at_requests(&mut self, pos: Pos2) -> Vec<EditorRequest>`:** collects requests emitted during `click_at` (needed for media pickers, save/open, version history).
4. **`copy(&mut self)`, `cut(&mut self)`, `paste_text(&mut self, text: &str)`:** dispatches `Event::Copy`, `Event::Cut`, `Event::Paste(text.to_owned())` matching OS clipboard delivery in `crates/ui-egui/src/lib.rs:3683-3714`.
5. **Desktop `EditorApp` headless helper (`apps/coursecraft/src/m27_files_ui_tests.rs`):** drives `EditorApp::new_desktop` + `egui::Context::run_ui(|ui| app.ui(ui))` so desktop modals (`unsaved_changes_modal`, `changed_on_disk_modal`, `restore_autosave_modal`, `error_modal`) and preferences/autosave/version-history flows are tested through real UI frames without OS file dialogs.

### Test Module Layout
- `crates/ui-egui/src/m27_triggers_states_tests.rs` — **D1** (Triggers) & **D2** (Object States)
- `crates/ui-egui/src/m27_timeline_media_tests.rs` — **D3** (Timeline) & **D4** (Media)
- `crates/ui-egui/src/m27_undo_clipboard_tests.rs` — **D5** (Undo/Redo & History) & **D6** (Copy/Cut/Paste/Duplicate)
- `crates/engine/src/m27_all_commands_undo_tests.rs` — **D5.1** exhaustive automated sweep across all **164 mutating commands**!
- `crates/ui-egui/src/m27_layers_questions_tests.rs` — **D7** (Layers) & **D8** (Question Editing, Banks & Result Slides)
- `crates/ui-egui/src/m27_masters_scenes_tests.rs` — **D9** (Slide Master & Layouts) & **D10** (Scenes & Story View)
- `crates/ui-egui/src/m27_find_a11y_tests.rs` — **D11** (Find & Replace) & **D12** (Accessibility Panel)
- `apps/coursecraft/src/m27_files_ui_tests.rs` — **D13** (Save / Open / Autosave / Version History)

---

## D1. Triggers (12 Variations)
*UI: `crates/ui-egui/src/triggers.rs:384-3559` | Engine: `commands.rs:8219-8320`*

| Case | Variation | Steps (via `TestHarness`) | Expected Result |
|---|---|---|---|
| **D1.1** | Full create/edit/delete lifecycle + default selected object + undo at each step | Select `"btn-start"`, open Triggers tab, click `+ New`, verify `When` defaults to `Click("btn-start")`, set `Action` to `ChangeState("btn-start", "visited")`, `Cmd+Enter` -> double-click row to edit -> change to `SetVariable` -> `Cmd+Enter` -> delete via row menu -> Undo × 3 -> Redo × 3. | Each commit pushes exactly 1 undo step; undo/redo restores exact trigger list & contents; `course.validate()` 0 errors. |
| **D1.2** | Every `ActionTag` & `EventKindTag` switch mid-edit (stale-field immunity) | Open Trigger Editor, cycle through ALL 26 `Action` kinds (`JumpToSlide` all 5 targets, `ShowLayer`, `HideLayer`, `ChangeState`, `SetVariable` all 6 ops + var-to-var, `Play/Pause/StopMedia`, `MoveOnPath`, `Pause/ResumeTimeline`, `SubmitQuestion`, `SubmitResults`, `Retry/Review/ExitReview`, `PrintSlide`, `CompleteCourse`, `ExitCourse`, `OpenUrl`, `RunJavaScript`, `LightboxSlide`, `CloseLightbox`, `Start/Pause/ResetTimer`) and ALL 19 `Event` kinds, committing valid samples for each. | Switching action/event kinds resets incompatible fields cleanly; zero panics, zero invalid JSON, zero `layout_report` clipping in Trigger Editor modal. |
| **D1.3** | Nested condition builder (`All` / `Any` / `Not` + `ObjectState` + var-to-var `Compare`) | Add condition -> add 3 clauses (`Compare` literal, `Compare` variable-to-variable `EqIgnoreCase`, `ObjectState(is=false)`) -> toggle `All`/`Any` -> wrap/nest `Not` -> remove middle clause -> commit -> reopen via double-click -> verify exact round-trip -> undo. | Round-trip preserves exact condition tree; empty/invalid condition fields show inline error without closing modal. |
| **D1.4** | Reorder Move Up / Move Down while filter or selected-object sort is active (**BUG-D1**) | Slide with 5 triggers across objects `A` and `B`. Select `B` (so `B`'s triggers float to top of list), then type filter `"B"`, click Move Up / Move Down on `B`'s second trigger. | Reorder swaps with the adjacent matching trigger (or disables/acts predictably without jumping over unrelated hidden rows); Undo restores original order. |
| **D1.5** | Double-click another trigger row while Trigger Editor is already open | Open Trigger 0 for edit, change its Action (unsaved), double-click Trigger 1 in Triggers tab. | Does not silently corrupt Trigger 1 or leave orphaned state; clearly switches or guards unsaved edit cleanly, zero panics. |
| **D1.6** | Copy & Paste triggers across slides and layers (partial skip handling) | Copy 3 triggers on Slide 1 (1 valid everywhere `ExitCourse`, 1 referencing Slide 1's `"btn-1"`, 1 referencing Layer `"l1"`), switch to Slide 2 (lacks `"btn-1"` and `"l1"`), paste triggers; also paste onto a layer. | Valid trigger pastes in 1 undo step; invalid ones skipped with status `"Pasted 1; skipped 2..."`; Undo removes pasted trigger cleanly. |
| **D1.7** | Disable / Enable & Duplicate trigger via `More (…)` menu | Disable Trigger 0 (`disabled == true`), duplicate it (verify copy preserves `disabled`), re-enable original; undo each step. | `disabled` toggles accurately, duplicated trigger appears adjacent and validates cleanly, undo reverts each step. |
| **D1.8** | Empty / huge / hostile field values in Trigger Editor | Enter `OpenUrl` with `""`, `"javascript:alert(1)"`, `"https://example.com"`; `RunJavaScript` with `""` and 70 KiB string (>65,536 byte cap); `TimelineReaches` with `-5`, `"abc"`, `999999999`; `KeyPressed` with `"CTRL+ALT"`. | Invalid URLs / >64 KiB JS / bad keys refused cleanly with visible error (modal stays open, course unchanged, `revision` unchanged). |
| **D1.9** | Rapid repeats: double-tap `Cmd+Enter`, spam `+ New` / `Esc` 20× | Open new trigger, fire two `Cmd+Enter` events in consecutive frames; then open/Esc 20× rapidly. | Double-tap `Cmd+Enter` creates exactly **1** trigger (not 2); 20× open/Esc leaks zero triggers and zero layout regressions. |
| **D1.10** | Base layer vs Slide layer trigger ownership & self-hiding `HideLayer(None)` | Switch to Layer `"L1"`, create `HideLayer { layer: None }` -> succeeds; switch to Base layer, verify `HideLayer` requires explicit layer (or refuses `None`). | Layer-owned triggers live in `layer.triggers`, base triggers in `slide.triggers`; validator 0 errors. |
| **D1.11** | Problem badge detection & one-click `"Fix the first one"` | Create a slide with a question whose only submit trigger uses `Event::Submit` (or broken target), verify warning badge + tooltip, click `"Fix the first one"`. | Opens offending trigger directly in Trigger Editor; fixing clears badge immediately. |
| **D1.12** | Undo deleting the slide/object while Trigger Editor modal is open | Open Trigger 0 for edit on Slide 2 -> press `Ctrl+Z` (or undo `slide.add` / `object.add`) so the trigger's owner disappears -> run 2 `idle_frame()`s. | Trigger Editor closes safely without panicking or writing to the wrong trigger index on another slide. |

## D2. Object States (10 Variations)
*UI: `crates/ui-egui/src/states.rs:32-465`, `canvas.rs` (`preview_state`) | Engine: `commands.rs:7212-7266`*

| Case | Variation | Steps (via `TestHarness`) | Expected Result |
|---|---|---|---|
| **D2.1** | Built-in preset states (`hover`, `down`, `visited`, `selected`, `disabled`, `hidden`) | Select button `"btn-1"`, add all 6 built-in presets via preset buttons/combo; verify each appears once and clicking an already-added preset does NOT duplicate it; undo each step. | All 6 states stored cleanly; duplicate preset click is no-op; undo removes each cleanly. |
| **D2.2** | Custom state creation: valid, empty, whitespace, `"normal"`, duplicate, 200-char name | Type `""`, `"   "`, `"normal"`, `"Normal"`, `"glow_1"`, `"glow_1"` (duplicate), and 200-char string + `Enter`. | Empty/whitespace/`normal`/duplicate/too-long rejected with status error (`revision` unchanged); `"glow_1"` succeeds. |
| **D2.3** | Editing `Fill`, `Text`, and `Src` overrides across object kinds | Edit state `fill` on Shape, Button, and closed `Path` (valid hex `#112233`, invalid `"#ZZZ"`, color picker, clear to empty); edit state `text` on Text, Button, Shape (with `ShapeText`); edit state `src` on Image (valid media vs missing path). | Valid overrides commit in 1 undo step each; invalid hex rejected without corrupting state; clearing removes override (`None`). |
| **D2.4** | State View / Eye preview toggle on canvas & switching objects | Click Eye preview on `"visited"` state (assert `editor.ui.preview_state == Some("visited")` and canvas renders `"visited"` fill/text) -> click Eye again to turn off -> turn on again and select a different object `"obj-2"`. | Canvas paints previewed state; toggling off restores Normal; selecting `"obj-2"` clears `preview_state`. |
| **D2.5** | Deleting the currently previewed state (**BUG-D2**) | Enable Eye preview on `"custom1"`, then click Delete on `"custom1"`. | `"custom1"` deleted AND `editor.ui.preview_state` cleared to `None` (no stale preview state left active!). |
| **D2.6** | Deleting a state that is `initial_state` or referenced by a trigger (**BUG-D2**) | Set `initial_state = "custom1"` on `"btn-1"` and add `ChangeState("btn-1", "custom1")` trigger -> click Delete on `"custom1"`. | Refusal surfaces clear explanation (neither crashes nor leaves broken `initial_state` / dangling trigger); after clearing `initial_state` and trigger, Delete succeeds and Undo restores it. |
| **D2.7** | `Set Initial` state toggle & reset back to `"normal"` + undo | Click `"Set initial"` on `"disabled"` -> verify `obj.initial_state == "disabled"` -> click `"Set initial"` on `"normal"` -> verify `"normal"` -> Undo × 2. | `initial_state` switches and undoes cleanly; `"normal"` can never be deleted. |
| **D2.8** | Mixed multi-selection with States panel | Select 2 objects (one Button with 3 states, one Audio with 0 states). | States panel hides or degrades cleanly with zero crashes and zero `layout_report` findings. |
| **D2.9** | States on objects inside a Slide Layer & on a Master Slide | Select `"about-text"` on layer `"about"` -> add/edit/delete state; enter Master View -> select master shape -> add/edit state. | Layer object states work identically; master object state behavior validates cleanly (`course.validate()` 0 errors). |
| **D2.10** | Rapid add/edit/delete/undo stress (20 custom states) | Add 20 custom states, edit fill on #10, delete #5 and #15, Undo × 23, Redo × 23. | Scrollable States list has `0` `layout_report` overlaps/clips; full undo/redo returns byte-identical `Object.states`. |

## D3. The Timeline (12 Variations)
*UI: `crates/ui-egui/src/timeline.rs:141-2594`, `timeline_playback.rs` | Engine: `commands.rs:2521, 6741-7006`*

| Case | Variation | Steps (via `TestHarness`) | Expected Result |
|---|---|---|---|
| **D3.1** | Drag bar move (`Move`) with 100ms snap vs `Shift` 10ms fine snap + undo | Drag `"obj1"` (`start: 500, end: Some(2500)`) right by 60px without Shift, then drag with `Modifiers::SHIFT`; undo × 2. | Without Shift snaps to 100ms grid (`duration` preserved); with Shift snaps to 10ms; Undo restores exact `(500, Some(2500))`. |
| **D3.2** | Trim left (`ResizeStart`) and trim right (`ResizeEnd`) including clamping (`start < end`) | Drag left handle of `"obj1"` past its right edge; drag right handle past its left edge; drag left handle left of `0ms`. | Clamped to `0 <= start_ms < end_ms <= slide.duration_ms` (minimum bar width enforced); `course.validate()` 0 errors. |
| **D3.3** | `"Show until end"` (`end_ms: None`) move, trim, & slide duration extension | Object with `start_ms: 1000, end_ms: None`: drag bar right (verify `end_ms` stays `None`!), then increase slide duration from `5s` to `12s`, then drag right handle inward (`end_ms` becomes `Some(...)`), then undo. | Moving preserves `end_ms: None`; extending slide extends bar visually to 12s; trimming right converts to `Some(...)`; undo restores `None`. |
| **D3.4** | Shortening slide duration below object `start_ms`/`end_ms` and cue points | Object at `[3000..8000]`, slide cue at `6000ms`, slide duration `10s`; set slide duration to `2s` via timeline duration input. | Duration change either clamps/refuses cleanly or validates without panicking; undo restores `10s` and intact timings/cues. |
| **D3.5** | `Escape` cancels bar drag, trim drag, and row-reorder drag mid-flight | Start dragging bar (`PointerButton::Primary` pressed + `PointerMoved`), press `Key::Escape` before release; repeat for row reorder drag. | Zero commands dispatched (`revision` unchanged), object timing and z-order untouched. |
| **D3.6** | Slide cue points: `C` shortcut add, drag cue, double-click rename (empty/Esc/valid), `Delete` | Scrub playhead to `2000ms`, press `Key::C` -> drag cue to `3200ms` -> double-click cue -> test empty name (error), `Esc` (cancel), `"Midpoint"` + `Enter` -> select cue + press `Delete` -> Undo × 3. | Cue created at `2000ms`, moved to `3200ms`, renamed, deleted, and restored cleanly on Undo; triggers referencing cue keep valid `cue` ID. |
| **D3.7** | Timeline bar snapping to slide cue point | Place cue at `2350ms`; drag `"obj1"` bar start near `2350ms` (within snap tolerance). | Bar snaps to `2350ms` (`start_ms == 2350`). |
| **D3.8** | Overlapping objects & z-order row drag-to-reorder (single, multi-select, group) | Slide with 5 overlapping objects (2 grouped). Drag bottom row to top; drag group row; drag 2 multi-selected rows; undo each. | `slide.objects` order updates accurately (group stays contiguous!), single undo step per drag, canvas draw order matches. |
| **D3.9** | Timeline on a Slide Layer (`layer.duration_ms`, layer cues, layer objects) | Select Layer `"L1"` (`duration_ms: 4000`), add layer cue with `C`, drag layer object bar, change layer duration to `6s`, switch back to Base. | Edits mutate `layer.objects` / `layer.cue_points` / `layer.duration_ms` without affecting base slide timings. |
| **D3.10** | Zoomed & horizontally scrolled timeline (`zoom > 1.0`, `scroll_x > 0`) | Set slide duration `30s`, zoom timeline in 3×, scroll horizontally to `15s`–`25s`, drag an object bar at `18s` and scrub playhead to `20s`. | Screen-to-ms coordinate conversion accounts for both `zoom` and `scroll_x` accurately (zero offset drift!). |
| **D3.11** | Timeline row Lock & Eye-hide toggles + inline object rename | Click Lock icon (verify `obj.locked == true` and canvas refuses drag), click Eye icon (`obj.editor_hidden == true`), double-click object name in left column -> type `"Hero Banner"` + `Enter`; delete object mid-rename. | Lock/hide persist and undo; rename commits cleanly; deleting mid-rename cancels rename state cleanly (`lib.rs:3557`). |
| **D3.12** | Boundary & huge timeline durations (`0`, `-10`, `0.05s`, `86400s`, `9999999s`, `"abc"`) | Type each boundary into slide duration field + `Enter`. | Rejects `<=0` / `"abc"` / `>86400s`; accepts `0.1s`..`86400s`; `layout_report` has 0 clipped ruler labels at `86400s`. |

## D4. Media (11 Variations)
*UI: `crates/ui-egui/src/media_library.rs:103-644`, `lib.rs:1728-2804, 4345-4578`, `crop.rs`, `properties.rs` | Engine: `commands.rs:814-957`*

| Case | Variation | Steps (via `TestHarness`) | Expected Result |
|---|---|---|---|
| **D4.1** | Audio & Video insert, replace, volume (`0.0`–`1.0`), & trim (`start_ms`/`end_ms`) | Trigger Insert Audio/Video (assert `EditorRequest::PickAudio`/`PickVideo`), call `import_audio`/`import_video`, edit volume (`0.5`, `-1` refused, `2.0` refused) and trim (`start=500, end=2000`; `end <= start` refused) via inspector, replace file via `replace_media_bytes`, undo × 4. | `object.set_media` commits trim & volume in 1 undo step each; invalid ranges refused; replace preserves object props & triggers. |
| **D4.2** | Video cue points & `"Add question cue"` (`video.add_question_cue`) | Select video, add cue at `1500ms` (`pause: true`), edit cue name/time, add question cue (`video.add_question_cue`), delete cue (verify `cue_reached` triggers auto-removed in same undo step!), undo. | Single undo step reverts question cue + created layer + triggers; deleting cue cleans up `cue_reached` trigger atomically. |
| **D4.3** | Media Library: filter, kind dropdown, double-click insert, & `Go to` (**BUG-D3**) | Populate project with image, audio, video, Lottie, VTT. Open Media Library -> filter by name -> filter by kind -> double-click image to insert on slide (assert accurate status message, NOT false `"not undoable"` — **BUG-D3**!) -> enter Story View -> click `"Go to"` on a media usage -> verify Story View exits AND target object is selected (**BUG-D3**!). | Filter/insert/navigation work cleanly; undo removes inserted object; `Go to` lands directly on the referencing object. |
| **D4.4** | Media Library: Delete Selected, Delete Unused, & Undo-resurrected missing media | Add unused PNG + used PNG -> click `"Delete unused"` -> confirm -> assert only unused removed. Delete image object -> `"Delete unused"` -> `Ctrl+Z` (resurrects object referencing deleted PNG!). | `editor.missing_media()` immediately reports the resurrected PNG, Publish dialog blocks with missing-media error, canvas shows missing placeholder (zero panic). |
| **D4.5** | Media Library / `import_*` / `paste_image_png` blocked in Master View | Enter Master View -> attempt `import_audio`, `import_video`, `import_lottie`, `paste_image_png`, and Media Library double-click insert of audio/video. | Audio/Video/Lottie refused cleanly on Master (allowed only on play slides, per `course-model.md:448`); image allowed on master where valid. |
| **D4.6** | Web object folder inside a Question Bank slide (**BUG-D3**) | Add a `WebSource::Folder { path: "web/demo", entry: "index.html" }` object with extra asset `web/demo/app.js` onto a Question Bank slide; call `editor.used_media()`. | `used_media()` retains BOTH `web/demo/index.html` AND `web/demo/app.js` (after fixing `web_folder_prefixes` at `lib.rs:4390-4417` to scan `course.banks`!). |
| **D4.7** | Interactive Image Crop lifecycle: drag handle, pan image, Reset, Cancel, Undo, & slide-change cancel | Select image -> `start_crop` -> drag crop handle -> `apply_crop` -> verify `crop: Some(...)` -> Undo -> reopen crop -> switch slides mid-crop (or press `Esc`). | Crop commits in 1 undo step, Reset returns `crop: None`, Esc/slide-change cancels cleanly without mutating course. |
| **D4.8** | WebVTT captions attach, replace, remove, & hostile path rejection | Attach `narration.vtt` via `import_captions` -> verify `captions: Some("narration.vtt")` -> attempt invalid `.txt` or `../evil.vtt` -> clear captions -> undo. | Non-`.vtt` / traversal rejected; valid attach/clear undoable; `used_media()` tracks `.vtt`. |
| **D4.9** | Background audio attach, volume/loop edit, clear, & rollback on bad audio | Call `import_background_audio` with valid WAV -> edit volume `0.3` & `loop: false` -> clear -> attempt `import_background_audio` with empty/invalid path. | `course.background_audio` updates & undoes; failed import rolls back cleanly leaving zero orphaned bytes in `editor.media`. |
| **D4.10** | Hostile media decoding: corrupt PNG, >5 MiB SVG, SVG with `<text>`, invalid Lottie, >200 MiB file, filename collisions | Feed corrupt bytes, SVG containing `<text>Hi</text>`, malformed Lottie JSON, and two distinct PNGs both named `"photo.png"` (and `"Photo.PNG"` case collision!). | Corrupt/oversized rejected cleanly; SVG `<text>` warns author; case-insensitive name collision produces unique safe filename without clobbering existing asset. |
| **D4.11** | Custom font add, rename family (**BUG-D3**), & remove used font | Import TTF (`import_font`), call `rename_font_family` (multiple faces), verify single undo step reverts all faces (**BUG-D3**!), remove font while text uses it -> confirm -> verify text falls back cleanly. | Atomic undo on family rename; zero crashes on remove; `course.validate()` warns cleanly on fallback. |

## D5. Undo / Redo & History Panel (12 Variations)
*UI: `crates/ui-egui/src/history.rs:1-119`, `menu.rs:896-920`, `lib.rs:3338-3572, 3770-3785` | Engine: `crates/engine/src/lib.rs:219-421`, `commands.rs:38-1614`*

| Case | Variation | Steps (via `TestHarness` & `Session`) | Expected Result |
|---|---|---|---|
| **D5.1** | **Exhaustive 164-command undo/redo round-trip sweep** (`crates/engine/src/m27_all_commands_undo_tests.rs`) | Build a fixture course with every object kind, layer, master, template, bank, draw, question kind, table, path, video cue, variable, resource, glossary, font, text style, palette. Assert the test table covers **every single `mutates == true` entry in `REGISTRY` (`commands.rs:38-1614`, 164 commands)** — test FAILS if a new command is added without an entry! For each command: snapshot `before`, execute valid params -> assert `course != before` & `course.validate()` 0 errors -> `edit.undo` -> assert `course == before` -> `edit.redo` -> assert `course == after` -> `edit.undo` -> assert `course == before`. | 100% of all 164 mutating commands round-trip losslessly through undo/redo and stay valid! |
| **D5.2** | Non-mutating commands & `skip_undo` no-ops never touch undo/redo stacks | Execute 1 mutation (`undo_len == 1`), `undo()` (`redo_len == 1`). Execute all 9 `mutates: false` commands (`course.info`, `course.validate`, `course.find`, ...) AND no-op mutations (`object.set_opacity` with same opacity, `object.lock` same bool, `layer.move` same index). | `undo_len` stays `0`, `redo_len` stays `1` (redo stack is NOT cleared by no-ops!), `revision` unchanged. |
| **D5.3** | Batch atomicity: mid-batch failure rolls back entire course | Execute `execute_batch_named("Test batch", [valid_add_slide, valid_add_object, INVALID_command])`. | Returns `BatchFailed`; `course` is byte-identical to pre-batch snapshot, `undo_len` & `revision` unchanged. Also verify `edit.undo`/`edit.redo` inside batch is rejected (`lib.rs:292-300`). |
| **D5.4** | Multi-step UI batch operations are 1 undo step | Perform each batched UI action: multi-object Delete (3 objs), multi-slide Delete (3 slides), multi-object Align, multi-object Format opacity/font, Paste 3 triggers, Insert Tabs interaction, Alt-text Apply. | Each creates exactly **1** named undo step; single `Ctrl+Z` reverts all affected items at once. |
| **D5.5** | History panel click-jump backward & forward + `"Opened"` reset + slide selection restore (**BUG-D4**) | Perform 6 named edits including `slide.delete` on Slide 2. Open History panel (`Edit ▸ History…`). Click step 2 (jumps back 4 steps past `slide.delete`) -> assert Slide 2 is **reselected** (**BUG-D4**!) -> click step 6 (redo 4 steps) -> click `"Opened"` (jumps to 0). | `undo_steps`/`redo_steps` jump accurately; restored slide is reselected via `pending_undo_redo = true`; zero `layout_report` findings. |
| **D5.6** | `MAX_HISTORY = 200` truncation cap & History UI truncation notice | Push 215 distinct mutations -> assert `undo_len() == 200` and `history_truncated() == true` -> open History panel -> verify truncation notice painted and zero layout clips -> undo all 200 steps. | Hard cap at 200 steps; oldest 15 dropped; History panel displays truncation hint cleanly and scrolls without clipping. |
| **D5.7** | Undo/Redo shortcuts (`Ctrl+Z`, `Ctrl+Y`, `Ctrl+Shift+Z`) & branch truncation | Edit A -> Edit B -> `Ctrl+Z` (reverts B) -> `Ctrl+Shift+Z` (redoes B) -> `Ctrl+Z` -> Edit C -> assert `can_redo() == false` (B pruned). Also verify `Ctrl+Z` while inline text edit is active undoes text (not the whole slide!). | All 3 shortcuts route accurately; new edit clears redo branch; text edit owns undo while focused. |
| **D5.8** | Selection repair on Undo of `layer.add`, `master.add`, `object.group`, `object.ungroup` | Add layer `"L2"` (selected) -> `Ctrl+Z` (selection returns to Base); add Master 2 in Master View (selected) -> `Ctrl+Z` (selection repairs to Master 1 or exits Master View safely); group 3 objs -> `Ctrl+Z` -> `Ctrl+Y`. | Selection never holds a dangling slide/layer/object/master ID after any undo/redo. |
| **D5.9** | Undo/Redo while transient windows/modes are open | Open Crop mode -> `Ctrl+Z` (cancels crop cleanly); open Layer Settings modal -> `Ctrl+Z` (`opened_revision` auto-closes modal); open Trigger Editor -> `Ctrl+Z`. | Zero crashes or stale modal commits targeting deleted items. |
| **D5.10** | Connector auto-rerouting on Undo/Redo | Glue connector between Shape A and Shape B -> move Shape A -> verify connector rerouted -> `Ctrl+Z` -> verify Shape A AND connector points return to exact pre-move coordinates -> `Ctrl+Y`. | Exact coordinate round-trip on both moved target and glued connector. |
| **D5.11** | Rapid alternating Undo/Redo spam (100×) | Perform 25 varied edits, then alternate `Ctrl+Z` / `Ctrl+Y` 100 times with random step counts and verify `course.validate()` after every burst. | Zero crashes, zero validation errors, exact state at every index. |
| **D5.12** | Edit menu dynamic labels (`"Undo <label>"` / `"Redo <label>"`) | Open `Edit` menu when empty (disabled `"Undo"`/`"Redo"`), after `"Add slide"` (shows `"Undo Add slide"`), after undo (shows `"Redo Add slide"`). | Menu labels reflect top undo/redo step names dynamically. |

## D6. Copy / Cut / Paste & Duplicate (11 Variations)
*UI: `crates/ui-egui/src/clipboard.rs:36-658`, `lib.rs:2273-2336, 3023-3052, 3683-3714` | Engine: `commands.rs:2489-2520, 5157-5390`*

| Case | Variation | Steps (via `TestHarness`) | Expected Result |
|---|---|---|---|
| **D6.1** | Copy/Paste & Cut/Paste single and multi-selected objects on same slide & different slide | Select 2 objects -> `copy()` -> `paste()` 3× on same slide (verify +20, +40, +60px offsets) -> switch to Slide 2 -> `paste()` (verify offset 0 on new slide, and verify `clipboard_paste_count` behavior — **BUG-D5**!) -> `cut()` -> undo restores cut objects! | Fresh unique IDs, exact offsets, cut is 1 undo step, paste selects newly created objects. |
| **D6.2** | Canvas right-click `"Paste here"` (`paste_private_at`) & oversized object clamping | Copy a 400×200 shape -> right-click near bottom-right corner `(1250, 700)` -> `"Paste here"`; also copy a 1400×800 oversized shape -> `"Paste here"`. | Normal shape centered on click & clamped inside slide; oversized shape centered without panicking or NaN. |
| **D6.3** | Copy/Paste grouped objects & glued connectors | Copy a group of 2 shapes + a connector glued between them -> paste on Slide 1 and Slide 2 -> undo. | Pasted copies get a **fresh shared group ID** distinct from original; connector glues remap to the **pasted** shape IDs (NOT still glued to originals!). |
| **D6.4** | Copy/Cut object with triggers including `DroppedOn` / `ObjectsIntersect` (**BUG-D5**) | Shape A has 3 triggers: `Click(A)->ChangeState(A)`, `Click(A)->ChangeState(B)`, and `DroppedOn(A, B)->SetVariable` (where B is NOT selected). Copy A -> paste on Slide 2. | Trigger 1 pastes with remapped ID; Trigger 2 AND Trigger 3 (`DroppedOn` referencing uncopied `B` — **BUG-D5**!) are skipped with accurate skip count (`course.validate()` 0 errors!). |
| **D6.5** | **Cross-course Copy & Paste** of objects and slides (two `TestHarness` instances) | Course A (has variable `v1`, master `m1`, slide link to `s2`, `next_requires: Variable(v1)`): `copy()` object with trigger on `v1`, and `copy()` slide referencing `m1`/`s2`/`v1`. Read `A.editor.ui.clipboard_text` and inject via `B.paste_text(...)` into blank Course B! | Course B pastes safely: missing `master`/`Link::Slide`/`next_requires` cleared or skipped cleanly with status message; Course B passes `course.validate()` with 0 errors! |
| **D6.6** | Copy/Cut/Paste & Duplicate slides across scenes (single, last slide, & multi-slide) | Cut Slide 1 of a 1-slide course (refused!). Add Scene 2 + Slide 2 -> copy Slide 1 -> select Scene 2 -> paste -> cut Slide 2 -> paste -> duplicate Slide 1 -> undo each. | Cutting only slide refused cleanly; cross-scene paste inserts into target scene; duplicated slide keeps group IDs and remaps slide/layer/object IDs. |
| **D6.7** | Duplicate (`Ctrl+D`) vs Alt-drag duplicate on canvas (single, multi, group, locked) | `Ctrl+D` on single object, multi-select, group, and a locked object; Alt-drag a shape on canvas; undo each. | `Ctrl+D` duplicates in 1 undo step; Alt-drag duplicates at drop position in 1 undo step; locked object cannot be Alt-dragged. |
| **D6.8** | Copy/Paste across Slide Layers (base -> layer, layer -> base, layer -> another slide's layer) | Copy object on Base -> switch to Layer `"L1"` -> paste -> copy from `"L1"` -> switch to Slide 2 Base -> paste. | Pasted object lands in active layer (`layer.objects` vs `slide.objects`) with unique slide-wide ID. |
| **D6.9** | Plain text paste creates Text box & UTF-8 >4096-byte truncation safety | Call `paste_text("Hello world")` -> creates Text box; call `paste_text` with 5,000 bytes containing 4-byte emoji `"🎉"` straddling byte 4096. | Creates valid Text box; truncates on UTF-8 char boundary (`<= 4096` bytes) without panicking. |
| **D6.10** | Hostile / corrupt / oversized clipboard payloads | Inject via `paste_text`: (a) 17 MiB string (>16 MiB cap), (b) `{"coursecraft_clipboard":999,...}` wrong version, (c) `{"coursecraft_clipboard":1,"kind":"objects", BROKEN`, (d) object with `frame.w: NaN` / `-50`. | All rejected gracefully with status error; zero panics, zero course mutation. |
| **D6.11** | Master View guards for Copy / Cut / Paste / Paste Image vs Duplicate | Enter Master View -> try `copy()`, `cut()`, `paste_text(...)`, `paste_image_png(...)` -> verify `"Not available in Master View"`; then try `Ctrl+D` on a master shape. | Clipboard ops cleanly blocked in Master View; `Ctrl+D` on master shape works cleanly and undoes. |

## D7. Layers (10 Variations)
*UI: `crates/ui-egui/src/layers.rs:46-1274`, `canvas.rs` (`show_layer_pills`, `BaseSlideView`) | Engine: `commands.rs:3758-3900`*

| Case | Variation | Steps (via `TestHarness`) | Expected Result |
|---|---|---|---|
| **D7.1** | Create, inline rename (`Enter`/`Esc`/blur/empty), & delete layer + undo | Click `+ New layer` -> double-click layer name -> test empty string (ignored), `Esc` (cancel), `"Feedback"` + `Enter` -> delete via footer & context menu -> Undo × 3. | Layer added, renamed, deleted, and restored cleanly; selection returns to Base on delete. |
| **D7.2** | Duplicate layer with objects & triggers (ID remapping) | Create Layer `"Tip"` containing 2 shapes + 1 click trigger referencing shape 1 -> click Duplicate (footer icon + context menu). | New layer `"Tip copy"` created with fresh unique object IDs and remapped trigger targets (`course.validate()` 0 errors); 1 undo step. |
| **D7.3** | Reorder layers via drag-and-drop & `Bring forward` / `Send backward` + `Esc` cancel | Slide with 4 layers (`L1`..`L4`, displayed front-to-back `L4` top -> `L1` bottom). Drag `L1` above `L4`; start drag & press `Esc`; use context menu `Bring forward` / `Send backward`; undo each. | `slide.layers` back-to-front index matches visual drag accurately; `Esc` leaves order unchanged; Undo reverts. |
| **D7.4** | Layer Settings modal: all 9 properties & no-op OK zero-undo check | Open Layer Settings (`gear` / context menu): edit `name`, `duration_ms` (`8000`), `hide_other_layers: false`, `hide_base_objects: true`, `pause_base_timeline: true`, `prevent_base_clicks: true`, `hide_when_timeline_ends: true`, `allow_seek: No`, `revisit: Resume` -> click OK (1 undo step) -> reopen & click OK without changing anything -> assert `undo_len` unchanged! | All 9 fields persist verbatim; no-op OK pushes 0 undo steps; `layout_report` 0 findings. |
| **D7.5** | Layer Settings modal validation & auto-close on slide change / undo | Open Layer Settings -> clear `name` to `"   "` (assert OK button disabled), enter `duration_ms = 0` and `99_999_999` (clamped `100..=86_400_000`) -> switch to another slide or trigger `Ctrl+Z` while modal is open. | Whitespace name disables OK; out-of-range duration clamped; switching slides or bumping `revision` auto-closes modal cleanly. |
| **D7.6** | Eye visibility toggle (`hidden_layers`) & focused-layer protection | Slide with `L1` and `L2` (focused on Base): toggle Eye off on `L1` (verify `L1` hidden in canvas), focus `L1` (verify `L1` automatically unhidden / cannot hide active layer), switch to Slide 2 & back. | Eye hides non-focused layers cleanly; focused layer is never invisible; per-slide hidden map stays isolated. |
| **D7.7** | Deleting a layer referenced by a base-slide trigger OR containing an object referenced by a base-slide trigger (**BUG-D6**) | Base slide has Trigger 1 (`ShowLayer("L1")`) and Trigger 2 (`ChangeState("obj_on_L1", "hidden")`). Delete `"L1"`. | Both dangling triggers are cleaned up atomically (after fixing **BUG-D6** at `commands.rs:3772-3776`!), `layer.delete` succeeds in 1 undo step, and Undo restores `"L1"` AND both triggers! |
| **D7.8** | `hide_other_layers: true` vs `false` + `BaseSlideView` (`Show`/`Dim`/`Hide`) parity | Toggle `hide_other_layers` in Layer Settings and verify canvas & `Player` preview reflect both `hide_other_layers` and `hide_base_objects` accurately. | Preview and `Player` match authored settings; switching slides resets canvas focus to Base. |
| **D7.9** | Rapid create/delete/rename stress (15 layers) + deleting layer mid-rename | Create 15 layers, start inline rename on Layer 15, delete Layer 15 via shortcut/undo, verify scroll & pill bar layout at `1280×900` and `1024×680`. | Zero panics, rename state cleared, `layout_report` 0 overlaps/clips. |
| **D7.10** | Layers panel hidden & `layer.add` blocked in Master View | Enter Master View -> assert Layers panel is hidden and dispatching `layer.add` on master slide is refused by engine (`course-model.md:448`). | Master slides can never acquire layers. |

## D8. Question Editing, Banks & Result Slides (12 Variations)
*UI: `crates/ui-egui/src/question.rs:70-2079`, `slides.rs`, `properties.rs` | Engine: `commands.rs:418-498, 1345-1595`, `commands/csv_import.rs`*

| Case | Variation | Steps (via `TestHarness`) | Expected Result |
|---|---|---|---|
| **D8.1** | Multiple Choice, Multiple Response, & True/False CRUD + scoring modes | Insert MC -> edit prompt, add 2 choices, delete 1, toggle correct, switch `Scoring` (`AllOrNothing` -> `PerAnswer` with choice points `5`/`-2` -> `Partial` on MR), toggle `shuffle`, undo each step. | All edits commit cleanly; `PerAnswer` negative/out-of-range choice points clamped `-1000..=1000`; `course.validate()` 0 errors. |
| **D8.2** | Fill-in-the-Blank accepted answers CRUD & `case_sensitive` | Insert `FillIn` -> add 3 accepted answers -> try adding empty `""` (refused) -> edit answer #2 -> delete answer #1 -> try deleting the LAST remaining accepted answer (refused: min 1!) -> toggle `case_sensitive` -> undo all. | Enforces 1..=50 accepted answers; empty/last-delete refused cleanly; undo restores exact list. |
| **D8.3** | Matching pairs CRUD & reorder + Sequence reorder | Insert `Matching` -> add pair, edit left/right, reorder pair up/down, try deleting below 2 pairs (refused); insert `Sequence` -> add item, reorder up/down, undo all. | Pair & sequence indices stay valid (`2..=20` enforced); undo restores exact order. |
| **D8.4** | Hotspot image, regions CRUD (rect/ellipse), canvas handle drag, & max markers | Insert `Hotspot` -> assign image (`question.set_hotspot_image`) -> add 2 regions (1 rect correct, 1 ellipse incorrect) -> select region & drag resize handle on canvas (`question.update_region`) -> set `markers` (`1`, `5`, `0` refused, `11` refused) -> delete incorrect region -> try deleting last correct region (refused) -> undo. | Enforces >=1 correct region and `markers` in `1..=10`; canvas drag commits 1 undo step. |
| **D8.5** | Drag & Drop items, drop targets, distractors, & all 3 drag options | Insert `DragDrop` -> add 2 shapes as drag items, 3 shapes as targets (1 distractor) -> try assigning an item as its own target (refused) -> toggle `snap_to_center`, `return_if_wrong`, `one_per_target` -> remove item -> undo. | Engine rejects self-target & item-as-target; options toggle cleanly; `course.validate()` 0 errors. |
| **D8.6** | Freeform & all 3 Survey kinds (`Likert`, `ShortAnswer`, `Essay`) | Insert `Freeform` -> build nested `correct_when` condition (`All` + `ObjectState` + `Compare`) -> verify `Scoring` locked to `AllOrNothing`. Insert `Likert` (add up to 11 choices, try 12th refused; none `correct`), `ShortAnswer`, `Essay`. | All 4 kinds edit cleanly in right panel with 0 `layout_report` findings and 0 validation errors. |
| **D8.7** | Feedback layers, `"Add Try Again layer"`, `"Add Submit button"` helper, & attempts (`0`, `1`, `3`) | Create bare question without submit/feedback -> verify warning & click `"Add Submit button"`, `"Add Feedback layers"`, set `attempts = 3`, click `"Add Try Again layer"` -> verify layers & triggers created -> Undo × 4. | Each helper creates valid layers/triggers in 1 undo step; `attempts = 0` displays Unlimited. |
| **D8.8** | Switching `QuestionKind` mid-edit (blur-on-same-frame race + undo) | On MC slide, type into choice #1 text field (no Enter), immediately switch Kind combo to `FillIn` -> `Matching` -> `Hotspot` -> `DragDrop` -> `Freeform` -> `Likert` -> `Ctrl+Z` × 6. | Zero panics or stale field bleed; full undo chain restores original MC choices & prompt byte-identically. |
| **D8.9** | Dual-panel sync: Right panel (`show_question_section`) + Bottom tab (`show_question_tab`) | Open Question tab in bottom panel while right panel ALSO shows Question section -> edit prompt in right panel, toggle shuffle in bottom panel -> verify both stay synced with 0 duplicate widgets/ID clashes. | Zero egui ID collisions, immediate mutual sync, 0 `layout_report` findings. |
| **D8.10** | Question Banks CRUD (`bank.create/update/add_question/move_slide/remove_slide/delete`) | Create bank `"Unit 1"` -> add 3 questions -> rename bank -> move question 3 to index 0 -> add Draw slide referencing bank (`count: 2`) -> attempt `bank.delete` while draw references it (refused or cleans up cleanly) -> undo. | Bank operations validate cleanly; cannot leave dangling draw referencing deleted bank. |
| **D8.11** | Draw slide bounds (`count: 0`, `count > bank.len()`, two draws exceeding bank size) | Bank has 3 questions. Insert Draw 1 (`count: 2`) -> insert Draw 2 (`count: 2`, total 4 > 3 -> refused!) -> update Draw 2 to `count: 1` (succeeds) -> try `count: 0` (refused) -> verify Draw slide shows notice in Question editor. | Enforces `sum(counts) <= bank.slides.len()` and `count >= 1`. |
| **D8.12** | CSV Question Import (`bank.import_csv`) & Results / Certificate insertion (`results.insert`, `certificate.insert`) | Import valid CSV (`examples/questions.csv`), malformed CSV (missing column, bad row, empty file — verify all-or-nothing rollback with row-numbered errors!), then run `results.insert` & `certificate.insert` and undo each. | Valid CSV imports atomically in 1 undo step; malformed CSV leaves course untouched with row numbers; Results & Certificate slides pass `course.validate()`. |

## D9. Slide Master, Layouts & Templates (10 Variations)
*UI: `crates/ui-egui/src/slides.rs:167-824`, `lib.rs:2842-2921`, `properties.rs`, `canvas.rs` | Engine: `commands.rs:499-542, 2646-2740`*

| Case | Variation | Steps (via `TestHarness`) | Expected Result |
|---|---|---|---|
| **D9.1** | Enter Master View -> edit -> Leave Master View restores return slide | Select Slide 3 -> click Master View -> add master `"Corporate"` -> add logo shape & background -> leave Master View. | Returns to Slide 3 (not Slide 1!); master objects paint behind Slide 3 when applied and are non-selectable on Slide 3 canvas. |
| **D9.2** | Apply master to single slide, multi-selected slides, clear master (`None`), & undo | Multi-select Slides 1 & 2 -> apply `"Corporate"` master via inspector -> clear master on Slide 1 -> undo × 2. | `slide.master` updates on all selected slides; clearing falls back to `theme.background`; undo restores cleanly. |
| **D9.3** | Delete master in use: confirmation dialog count & full undo restoration | Assign `"Corporate"` to 2 slides AND 1 template -> click Delete on `"Corporate"` -> verify confirmation dialog shows accurate usage count -> confirm -> verify slides & template have `master == None` -> `Ctrl+Z`. | Confirmation dialog shows count; confirm clears references cleanly; single `Ctrl+Z` restores master AND re-attaches all 2 slides + 1 template! |
| **D9.4** | Inline master rename (`Enter`, `Esc`, blur, empty/huge name) | Double-click master title -> test `""` (ignored), `Esc` (cancelled), `"Dark Hero"` + `Enter`, and 300-char title. | Clean rename + undo; huge title truncated or validated cleanly without clipping panel (`layout_report`). |
| **D9.5** | Forbidden content on Master slides (Audio, Video, Lottie, Layer, Trigger, Question, Draw) | In Master View, verify UI disables/hides Insert Question/Audio/Video/Lottie/Layer/Trigger, AND directly test engine commands `object.add(Audio/Video/Lottie)`, `layer.add`, `trigger.add`, `question.set`, `draw.insert` on master ID. | Every forbidden construct is refused cleanly (`course-model.md:448`); `course.validate()` stays 0 errors. |
| **D9.6** | Refusing `slide.set_master` on a Question Bank slide | Execute `slide.set_master` targeting a slide inside `course.banks`. | Refused cleanly by `commands.rs:2723-2728` with clear error. |
| **D9.7** | All 6 built-in layouts (`blank`, `title`, `title_content`, `two_columns`, `picture_text`, `question`) | Open `+ New slide` popup (`show_new_slide_popup`) -> insert every one of the 6 layouts (with and without an active master) -> verify objects, theme colors, `course.validate()`, and undo × 6. | Every layout creates valid, non-overlapping objects matching theme tokens; undo removes each cleanly. |
| **D9.8** | In-course templates: Save slide as template, insert slide from template, delete template | Save Slide 2 (with objects, layer, triggers, master) via `template.save` -> insert new slide via `template.add_slide` (verify fresh IDs + kept `master`) -> delete template -> undo × 3. | Template deep-clones with zero ID collisions; `publish` strips `course.templates` completely. |
| **D9.9** | Shared template library requests & delete confirmation (`templates_tests` expansion) | Trigger Save to Library, Insert from Library, and Delete from Library (Cancel vs Confirm) through the `+ New slide` popup UI. | Emits exact `EditorRequest` variants; Cancel does nothing; Confirm emits delete; zero `layout_report` findings. |
| **D9.10** | Master View modal/shortcut isolation | While in Master View, press `Delete` (must not delete play slides!), `Ctrl+Shift+M` (Story View blocked in Master View), `Ctrl+M` (adds master or blocked cleanly, never silently adds hidden play slide). | Zero accidental mutation of play slides while editing masters. |

## D10. Scenes & Story View / Course Map (10 Variations)
*UI: `crates/ui-egui/src/course_map.rs:100-835`, `crates/ui-egui/src/slides.rs:107-2159` | Engine: `commands.rs:2260-2520`*

| Case | Variation | Steps (via `TestHarness`) | Expected Result |
|---|---|---|---|
| **D10.1** | Toggle Story View (`Ctrl+Shift+M`), keyboard arrow navigation, & `Enter` / double-click exit | Multi-scene course (3 scenes, 8 slides). Press `Ctrl+Shift+M` -> press `ArrowRight`/`ArrowDown`/`ArrowLeft`/`ArrowUp` -> press `Enter` (and test double-clicking thumbnail). | Story View opens, arrows move selection across slides/scenes, `Enter`/double-click returns to Slide Editor on that exact slide. |
| **D10.2** | Clicking a slide in Story View clears stale `also_slides` multi-selection (**BUG-D7**) | In Slide Navigator, `Shift+Click` Slides 1–3 (`also_slides.len() == 2`). Enter Story View -> click Slide 2 -> press `Delete`. | Clicking Slide 2 in Story View MUST clear `also_slides` (**BUG-D7** at `course_map.rs:370-400`), so `Delete` removes ONLY Slide 2 (not Slides 1–3!). |
| **D10.3** | Story View keys blocked while Preview or modal dialog is open (**BUG-D7**) | Enter Story View -> open Find (`Ctrl+F`) or Preview or Course Settings -> press `ArrowRight`, `ArrowDown`, `Enter`. | `handle_map_keys` ignores all keys while any modal or Preview is open (**BUG-D7** at `course_map.rs:778-835`); Story View stays open and selection unchanged. |
| **D10.4** | Dragging slides in Story View: within scene, across scenes, & onto an **empty scene header** | Create empty Scene 3 -> in Story View, drag Slide 2 inside Scene 1, drag Slide 2 into Scene 2, drag Slide 2 onto Scene 3's empty header (`map_drop_target:658-673`) -> Undo × 3. | Slide moves accurately into Scene 3, `course_map` cache refreshes immediately, Undo restores exact original positions. |
| **D10.5** | Inline scene rename in BOTH Story View and Slide Navigator (`Enter`, `Esc`, blur, empty) | Double-click Scene 1 header in Story View -> test empty (ignored), `Esc` (cancel), `"Module 1"` + `Enter`; repeat in Slide Navigator context menu. | Scene title updates, undoable, reflected in both views. |
| **D10.6** | `scene.delete`: last scene refused, non-empty scene confirm, & cross-scene trigger cleanup (**BUG-D7**) | (a) Try deleting only scene -> refused. (b) Scene 1 has `JumpToSlide(s2_in_scene2)` and `LightboxSlide(s2_in_scene2)` -> delete Scene 2 via confirm dialog. | Deleting last scene refused; deleting Scene 2 either cleans up dangling cross-scene triggers cleanly or shows a clear actionable refusal naming the referencing slide/trigger (**BUG-D7** at `commands.rs:2290-2308`). |
| **D10.7** | Story View edge arrows: Jump, Lightbox, Link, Next, Conditional, self-loop, & >12 outgoing `+N more` chip | Build slide with 15 outgoing jump/lightbox/link triggers (including self-jump & conditional) + 1 unreachable slide. Open Story View & run `layout_report(2)`. | Arrows & `+N more` badge render cleanly, unreachable slide shows `!` badge, zero panics or `layout_report` overlaps. |
| **D10.8** | Navigator drag-reordering slides within & between scenes + `Shift+Click` / `Cmd+Click` multi-select | In Slide Navigator, drag Slide 3 before Slide 1; drag Slide 1 into Scene 2; `Shift+Click` range select & `Cmd+Click` toggle -> verify selection count -> undo. | Navigator drag matches Story View `slide.move` semantics cleanly. |
| **D10.9** | Empty scene + deleting the only non-empty scene | Course with Scene 1 (1 slide) and Scene 2 (0 slides). Attempt to delete Scene 1 or its only slide. | Course can never end up with 0 total slides (`validate()` requires >=1 slide); clean refusal with message. |
| **D10.10** | Large Story View stress (50 slides across 10 scenes) + zoom/scroll | Build 50-slide course -> open Story View -> scroll, drag slide from Scene 10 to Scene 1, press arrows, check `layout_report(2)`. | Smooth render, zero clipped scene headers or panics. |

## D11. Find & Replace (10 Variations)
*UI: `crates/ui-egui/src/find.rs:28-443` | Engine: `commands.rs:2165-2224`, `crates/course/src/strings.rs`*

| Case | Variation | Steps (via `TestHarness`) | Expected Result |
|---|---|---|---|
| **D11.1** | `Match case` & `Whole word` toggles + live count accuracy | Slide with `"cat"`, `"Cat"`, `"category"`, `"CAT"`. Search `"cat"` (4 hits) -> toggle `Match case` (2 hits) -> toggle `Whole word` (1 hit) -> clear query (0 hits). | Exact match counts update immediately on every toggle. |
| **D11.2** | `Replace` (single hit) vs `Replace All` + undo & status clearing | Search `"old"` across 3 objects -> click `Replace` on hit #1 (only hit #1 changes, 1 undo step) -> click `Replace All` (remaining 2 change in 1 undo step) -> `Ctrl+Z` × 2. | Single replace touches only that location; Replace All is 1 undo step; undo reverts cleanly and clears stale `"Replaced N"` status. |
| **D11.3** | Variable reference protection (`%var%` AND `%Course.ScorePercent%`) | Text `"Score: %score% and %Course.ScorePercent% score"`. Search `"score"` -> `Replace All` with `"points"`. | Replaces ONLY `"score"` outside `%...%`; `%score%` and `%Course.ScorePercent%` untouched; displays `"Replaced 1. Skipped 2 inside variable references."`. |
| **D11.4** | Multi-run rich text formatting preservation on Replace | Text box with 3 runs: `"Hello "` (regular) + `"bold"` (bold red 32px) + `" world"` (italic). Replace `"bold"` with `"STRONG"`. | Resulting runs preserve bold red 32px on `"STRONG"` and italic on `" world"`; `text == concat(runs)`. |
| **D11.5** | Clicking hits across ALL 10 target scopes | Plant `"needle"` in: (1) Base text box, (2) Slide Layer object, (3) Master slide object, (4) Scene title, (5) Course title, (6) Question prompt/choice, (7) Slide notes, (8) Table cell, (9) Marker title/body, (10) Glossary term/definition. Click each hit in Find window. | Each click navigates directly to the right slide/layer/Master View/Course tab/Question tab/object without panicking. |
| **D11.6** | Clicking a Find result while in Story View (**BUG-D8**) | Enter Story View (`Ctrl+Shift+M`) -> open Find (`Ctrl+F`) -> click a hit on Slide 2. | Exits Story View (`editor.ui.story_view == false` — **BUG-D8** at `find.rs:375-417`!) and selects Slide 2 + target object on canvas. |
| **D11.7** | Replacement length > `MAX_TEXT_BYTES` (4096) & empty replacement `""` (**BUG-D8**) | (a) Replace `"foo"` with `""` (deletes occurrences cleanly). (b) Enter 5,000-byte replacement string -> assert `can_replace` guards >4096 bytes (**BUG-D8** at `find.rs:243`!) or reports clean validation error without mutating course. | Empty replacement works; >4096 bytes never produces silent corruption or unhandled error. |
| **D11.8** | Self-containing replacement (`"a"` -> `"aa"`) non-infinite loop | Replace All `"a"` with `"aa"` on `"aaa"`. | Terminates immediately producing `"aaaaaa"` (no infinite loop!). |
| **D11.9** | Special regex-like characters (`[a-z]`, `.*`, `(foo)`, `$1`, `\n`) & Unicode (`café`/`CAFÉ`) | Search literal `"[a-z].*"` and `"café"` (case-insensitive vs match-case). | Treated as literal text (never regex crash); Unicode case folding works cleanly. |
| **D11.10** | `MAX_HITS` truncation notice & `Esc` close / reopen at full size | Course with 600 occurrences of `"item"` -> open Find -> verify truncation banner & `layout_report(2)` 0 clips -> press `Esc` -> reopen `Ctrl+F`. | Truncation notice shown; `Esc` closes cleanly without affecting selection; reopens at full size. |

## D12. Accessibility Panel — Check Course & Alt Text (10 Variations)
*UI: `crates/ui-egui/src/status.rs:12-440`, `alt_text.rs:87-943`, `lib.rs:3575-3620` | Engine: `commands.rs:58-75`*

| Case | Variation | Steps (via `TestHarness`) | Expected Result |
|---|---|---|---|
| **D12.1** | Open Check Course from status bar chip & `File ▸ Check Course` + severity counts | Course with 1 validation error, 2 a11y errors (missing alt text, low contrast), 2 warnings (small text <16px, missing audio captions), 1 note (text over image). Click status bar chip -> verify counts, badges, and manual checks list -> close -> reopen via `File ▸ Check Course`. | All severities (`error`, `warning`, `note`) and manual checks render cleanly with 0 `layout_report` findings. |
| **D12.2** | Live fix + 300ms deferred audit debounce vs immediate refresh while open | Close Check Course -> fix missing `alt` text on image -> advance `0.1s` (audit deferred) -> advance `0.35s` (audit runs, count drops!). Reopen Check Course -> mark another image `decorative: true` -> assert finding disappears on the very next frame! | Debounce skips work within 300ms when closed; immediate refresh when Check Course window is open. |
| **D12.3** | Clicking findings across base object, layer object, & master object | Click findings on a base image, a layer button, and a master image -> verify exact selection (`slide`, `layer`, `object`, and `master_view` entered for master finding). | Every click jumps to the exact offending object. |
| **D12.4** | Clicking a play-slide finding while in Master View or Story View (**BUG-D8**) | Enter Master View -> open Check Course -> click a finding on Slide 1 -> assert `editor.ui.master_view == false` and Slide 1 stays selected across 3 `idle_frame()`s! Repeat starting from Story View (`story_view == false`). | Cleanly exits Master View and Story View instead of snapping back to Master 1 (**BUG-D8** at `status.rs:367-396`)! |
| **D12.5** | Clicking course-level findings (`slide: None`, e.g., empty course title / missing `language`) (**BUG-D8**) | Clear course title / check course-level finding -> click row in Check Course. | Switches right inspector to `InspectorTab::Course` so the author can immediately fix it (**BUG-D8** at `status.rs:124-146`). |
| **D12.6** | Validation error navigation on `/banks/{bi}/slides/{si}/...` (**BUG-D8**) | Introduce a validation issue on a Question Bank slide (or click a bank finding) via `navigate_to_issue_path`. | Navigates cleanly to the bank slide without panicking or silently doing nothing (**BUG-D8** at `status.rs:12-119`). |
| **D12.7** | `"Suggest alt text…"` button gating & Alt Text batch window | Verify `"Suggest alt text…"` button appears ONLY on `alt-text` findings; open Alt Text window -> edit manual alt text for 2 images -> click Apply -> verify 1 undo step updates both images and removes both findings -> `Ctrl+Z`. | Batch apply is 1 undo step; undo restores both findings. |
| **D12.8** | All 13 automated `Course::audit()` checks triggered & cleared | Construct 1 slide triggering every check in `course-model.md:481-495` (`course-title`, `slide-title`, `alt-text`, `control-name`, `contrast`, `contrast-unknown`, `text-size`, `target-size`, `video-captions`, `audio-captions`, `autoplay-control`, `focus-order`, `decorative-interactive`), verify all 13 appear in Check Course UI, fix each via UI/commands, assert count drops to 0 (except `contrast-unknown` note). | 100% of audit rules surfaced and clearable cleanly. |
| **D12.9** | Duplicate `focus_order` & `decorative` on interactive button (property edits + audit) | Set `focus_order = 1` on two objects (warning appears) -> change second to `2` (clears); mark Button `decorative = true` (`decorative-interactive` error appears) -> uncheck (clears); undo each. | Properties panel edits sync with audit immediately. |
| **D12.10** | Error History window (`status.rs:403-419`) & status-bar error auto-expiry | Trigger 5 refused commands -> open Error History window -> verify all 5 timestamped entries listed with 0 clips -> advance virtual clock `6.0s` -> verify status bar banner clears while Error History retains entries. | Full error visibility with clean status bar expiry. |

## D13. Save / Open / Autosave / Version History (12 Variations)
*Files: `apps/coursecraft/src/{main.rs,files.rs,autosave.rs,versions.rs}`, `crates/ui-egui/src/version_history.rs` | Test file: `apps/coursecraft/src/m27_files_ui_tests.rs`*

| Case | Variation | Steps (via headless `EditorApp` + temp dirs) | Expected Result |
|---|---|---|---|
| **D13.1** | `.ccraft` Save -> Edit -> Save -> Reopen byte/media fidelity & atomic temp cleanup | Create course with image + audio + captions + web folder + custom font -> `perform_save` to `proj.ccraft` -> edit -> save again -> verify zero leftover `.tmp` files -> reopen in fresh `EditorApp`. | All course JSON, media, web folders, and fonts round-trip identically; `saved_revision == session.revision`. |
| **D13.2** | Folder project (`Save As Folder` -> incremental save -> reopen -> `pack`/`unpack` parity) | Save project as folder `proj-dir/` -> edit title only (media untouched) -> save again -> assert unchanged media files keep identical mtime (incremental write!) -> reopen from `proj-dir/`. | Incremental folder save writes only changed files; reopening loads all media & course intact. |
| **D13.3** | `Save As` / `Save As Folder` failure MUST NOT corrupt `self.project` path (**BUG-D9**) | Open saved `good.ccraft` -> call `perform_save_as` / `perform_save_as_folder` targeting an unwritable/non-existent parent or read-only path (force error). | Save returns error modal/status, AND `app.project.path`, `app.project.folder`, and `app.project.disk_stamp` STILL point to `good.ccraft` (**BUG-D9** at `main.rs:793-826`!). |
| **D13.4** | `finish_open` / `OpenExample` / `OpenTemplate` preserves Preferences autosave interval (**BUG-D9**) | Set `prefs.autosave_minutes = 0` (Off) or `5` (5 min) -> open a `.ccraft` file (`finish_open`), open an example (`OpenExample`), open a starter template (`OpenTemplate`). | `app.autosaver.interval_minutes` stays `Some(0)` / `Some(5)` in ALL three paths (NOT reset to default `1` min — **BUG-D9** at `main.rs:862, 928, 933`!). |
| **D13.5** | Untitled autosave write, startup recovery modal (`Restore` vs `Discard`), & clean cleanup on save | Dirty untitled project -> advance `autosaver` -> assert untitled `.autosave.ccraft` written -> launch fresh `EditorApp` -> assert `restore_autosave_modal` shown -> test `Restore` (loads dirty course) AND test `Discard` (deletes autosave, opens blank). | Both Restore and Discard work cleanly through real modal UI; explicit Save removes autosave file. |
| **D13.6** | Named `.ccraft` AND folder-project autosave recovery (`is_autosave_newer`) | Save `named.ccraft` (and `folder-proj/`), mutate & trigger `update_autosave` -> reopen -> assert `restore_autosave_modal` triggers -> click `Restore` -> verify `saved_revision == u64::MAX` (dirty!) -> `perform_save` writes back to `named.ccraft` / `folder-proj/` cleanly. | Named and folder autosaves both detect newer timestamp, restore dirty, and save back to original location cleanly. |
| **D13.7** | Changed-on-disk conflict modal (`Reload`, `Overwrite`, `Save As…`) for `.ccraft` AND folder | Open `conflict.ccraft` -> modify file externally on disk -> make an edit in app -> `perform_save` -> assert `changed_on_disk_modal` appears -> test all 3 buttons (`Reload` loads external changes, `Overwrite` replaces disk file, `Save As` leaves external file intact). Repeat for folder project (`course.json` modified externally). | External edits are never silently clobbered; all 3 conflict resolutions work cleanly. |
| **D13.8** | Unsaved-changes modal (`Save`, `Don't Save`, `Cancel`) on New / Open / Quit / Import PPTX | Dirty project -> trigger `New` / `Open` / `Quit` -> assert `unsaved_changes_modal` renders (`layout_report` 0 clips) -> test `Cancel` (aborts), `Don't Save` (executes pending action), `Save` (saves then executes pending action). | All 3 branches execute accurately with zero lost edits. |
| **D13.9** | Version History lifecycle: snapshot on 2nd save, dedup identical saves, prune > `keep_N`, `keep == 0` | Save 1 (no snapshot yet) -> edit + Save 2 (1 snapshot) -> Save 3 with identical bytes (deduped, still 1) -> edit + save 12 more times with `keep = 10` -> verify pruned to 10 -> open Version History window (`show_version_history_window`). | Exact snapshot count, deduplication, pruning, and clean UI rows (`layout_report` 0 clips). |
| **D13.10** | Version History: `Open as Copy` & `Delete` + `VERSION_CACHE` invalidation (**BUG-D9**) | Open Version History -> click `Open as Copy` on v1 -> verify untitled dirty project opens with v1 contents (`project.path == None`) while current file untouched; click `Delete` on v2 -> verify file removed AND `VERSION_CACHE` invalidated (**BUG-D9** at `versions.rs:192-233`). | `Open as Copy` never overwrites active file; deleted version disappears immediately and never serves stale cache. |
| **D13.11** | Damaged / truncated / future-schema `.ccraft` open + latest-version recovery offer | (a) Open truncated zip `corrupt.ccraft` that has a valid version snapshot -> verify plain error message AND offer to restore latest readable version. (b) Open `course.json` with `schema_version: 99` -> verify plain `"made with a newer CourseCraft"` message (never raw serde error!). | Zero panics; actionable user-facing recovery; future schema handled per `course-model.md:9`. |
| **D13.12** | Rapid 20× save burst & crash-interrupted atomic write recovery | Save 20 edits rapidly; also plant a stale `.coursecraft-tmp-*` beside project -> open -> verify stale temp removed and valid save opens cleanly; verify `verify_ccraft_file` blocks replacing a good save if newly written file fails validation. | Last known-good file on disk is never corrupted. |

---

## D. Files / Crates to Touch & Ordered Implementation Steps

- **Step D-1 (1 session):** Extend `crates/ui-egui/src/test_harness.rs` with monotonic `time`, `advance_time`, `double_click_at`, `click_at_requests`, `copy`/`cut`/`paste_text`; verify all existing `coursecraft-ui-egui` tests remain green.
- **Step D-2 (1 session):** Implement **D1** (Triggers, 12 cases) and **D2** (Object States, 10 cases) in `crates/ui-egui/src/m27_triggers_states_tests.rs`; fix **BUG-D1** (`triggers.rs:1726-1968`) and **BUG-D2** (`states.rs:334-344`).
- **Step D-3 (1 session):** Implement **D3** (Timeline, 12 cases) and **D4** (Media, 11 cases) in `crates/ui-egui/src/m27_timeline_media_tests.rs`; fix **BUG-D3** (`lib.rs:2772-2804, 4390-4417`, `media_library.rs:517-537`).
- **Step D-4 (1 session):** Implement **D5.1** exhaustive 164-command undo/redo sweep in `crates/engine/src/m27_all_commands_undo_tests.rs` (asserting 100% coverage of `REGISTRY` `mutates == true` commands).
- **Step D-5 (1 session):** Implement **D5.2–D5.12** (Undo/Redo & History) and **D6.1–D6.11** (Copy/Cut/Paste & Duplicate, including cross-course paste!) in `crates/ui-egui/src/m27_undo_clipboard_tests.rs`; fix **BUG-D4** (`history.rs:99-107`) and **BUG-D5** (`clipboard.rs:137-138, 495-534`).
- **Step D-6 (1 session):** Implement **D7** (Layers, 10 cases) and **D8** (Question Editing, Banks & Result Slides, 12 cases) in `crates/ui-egui/src/m27_layers_questions_tests.rs`; fix **BUG-D6** (`crates/engine/src/commands.rs:3772-3776`).
- **Step D-7 (1 session):** Implement **D9** (Slide Master, Layouts & Templates, 10 cases) and **D10** (Scenes & Story View, 10 cases) in `crates/ui-egui/src/m27_masters_scenes_tests.rs`; fix **BUG-D7** (`course_map.rs:370-400, 778-835`, `commands.rs:2290-2308`).
- **Step D-8 (1 session):** Implement **D11** (Find & Replace, 10 cases) and **D12** (Accessibility Panel, 10 cases) in `crates/ui-egui/src/m27_find_a11y_tests.rs`; fix **BUG-D8** (`find.rs:243, 375-417`, `status.rs:12-146, 367-396`).
- **Step D-9 (1 session):** Implement **D13** (Save / Open / Autosave / Version History, 12 cases) in `apps/coursecraft/src/m27_files_ui_tests.rs`; fix **BUG-D9** (`apps/coursecraft/src/main.rs:793-826, 862, 928, 933`, `apps/coursecraft/src/versions.rs:192-233`).

---

# Master Implementation Order & Final Gate

Each step is sized for **one focused implementer session** and leaves `cargo xtask ci` green at the end of the session:

| Order | Step | Part | Key Deliverable | Primary Gate Command |
|---:|---|---|---|---|
| 1 | **A-1** | A | `crates/strict-lms` SCORM 1.2 + 2004 engines + unit tests | `cargo test -p coursecraft-strict-lms && cargo xtask layers && cargo xtask wasm` |
| 2 | **A-2** | A | `strict-lms` xAPI/cmi5 + manifests; migrate `xtask/src/mock_lrs.rs` | `cargo test -p coursecraft-strict-lms -p xtask` |
| 3 | **A-3** | A | Fix **BUG-A1..A7, A9, A10, B3** + `docs/spec/triggers.md` + `crates/lms/tests/strict_*` | `cargo test -p coursecraft-lms -p coursecraft-scorm -p coursecraft-xapi -p coursecraft-publish` |
| 4 | **A-4** | A | `xtask/strict-lms-wasm` + `xtask/mock-lms/index.html` single-source wiring | `cargo xtask wasm && cargo xtask layers` |
| 5 | **A-5** | A | `apps/coursecraft-player/src/session.rs` native seam + fix **BUG-A8** | `cargo test -p coursecraft-player && cargo xtask wasm` |
| 6 | **A-6** | A | `apps/coursecraft-lti/src/strict_platform.rs` + fix **BUG-A11** + **A4.1–A4.10** | `cargo test -p coursecraft-lti -p coursecraft-lti-server` |
| 7 | **B-1** | B | Fix **BUG-B1, B2** + `crates/logic/tests/m27_reference_edges.rs` (**B5.1–B5.8**) | `cargo test -p coursecraft-logic` |
| 8 | **B-2** | B | Author `examples/reference-course/` + minimal `media/` + scripts | `cargo run -p coursecraft-cli -- validate examples/reference-course/course.json && cargo run -p coursecraft-cli -- simulate examples/reference-course/course.json examples/reference-course/pass.inputs.json --strict` |
| 9 | **B-3** | B | Multi-standard runner + **B1.1–B1.8** (SCORM 1.2) + **B2.1–B2.8** (SCORM 2004) | `cargo test -p coursecraft-cli --test m27_reference_course` |
| 10 | **B-4** | B | **B3.1–B3.10** (cmi5 & xAPI) automated runs | `cargo test -p coursecraft-cli --test m27_reference_course` |
| 11 | **B-5** | B | **B4.1–B4.8** (LTI 1.3, Web, cross-standard parity) | `cargo test -p coursecraft-cli --test m27_reference_course` |
| 12 | **C-1** | C | Fix style-less tables (**BUG-C1**) + `m27_table_tests.rs` (**C2.1–C2.10**) | `cargo test -p coursecraft-pptx && cargo test -p coursecraft-cli --test pptx_fidelity` |
| 13 | **C-2** | C | Fix `pptx_ids` (**BUG-C2**) + `crates/pptx/src/timing.rs` entrance/exit/emphasis | `cargo test -p coursecraft-pptx` |
| 14 | **C-3** | C | Motion paths, `interactiveSeq`, easing/repeat in `timing.rs` + update `lib.rs` | `cargo test -p coursecraft-pptx` |
| 15 | **C-4** | C | `crates/pptx/src/m27_anim_tests.rs` (**C1.1–C1.13**) + `docs/pptx-import.md` | `cargo test -p coursecraft-pptx` |
| 16 | **C-5** | C | `add_animated_slide.py` + Slide 20 in `fidelity.pptx` + **C1.14** | `cargo test -p coursecraft-cli --test pptx_fidelity` |
| 17 | **D-1** | D | `TestHarness` helpers (`time`, `double_click_at`, `click_at_requests`, clipboard) | `cargo test -p coursecraft-ui-egui` |
| 18 | **D-2** | D | **D1** Triggers + **D2** Object States + fix **BUG-D1, D2** | `cargo test -p coursecraft-ui-egui m27_triggers_states` |
| 19 | **D-3** | D | **D3** Timeline + **D4** Media + fix **BUG-D3** | `cargo test -p coursecraft-ui-egui m27_timeline_media` |
| 20 | **D-4** | D | **D5.1** 164-command undo/redo sweep (`crates/engine`) | `cargo test -p coursecraft-engine m27_all_commands_undo` |
| 21 | **D-5** | D | **D5.2–D5.12** Undo/History + **D6** Clipboard/Duplicate + fix **BUG-D4, D5** | `cargo test -p coursecraft-ui-egui m27_undo_clipboard` |
| 22 | **D-6** | D | **D7** Layers + **D8** Questions/Banks/Results + fix **BUG-D6** | `cargo test -p coursecraft-ui-egui m27_layers_questions` |
| 23 | **D-7** | D | **D9** Masters/Layouts + **D10** Scenes/Story View + fix **BUG-D7** | `cargo test -p coursecraft-ui-egui m27_masters_scenes` |
| 24 | **D-8** | D | **D11** Find & Replace + **D12** Accessibility Panel + fix **BUG-D8** | `cargo test -p coursecraft-ui-egui m27_find_a11y` |
| 25 | **D-9** | D | **D13** Save/Open/Autosave/Version History + fix **BUG-D9** + final full CI | `cargo test -p coursecraft && cargo xtask ci` |

### Final Release Sign-Off Criteria
1. `cargo xtask ci` passes with **0 warnings** (`fmt`, `clippy --workspace --all-targets -D warnings`, `test --workspace`, `layers`, `wasm`).
2. All **264 planned M27 cases** pass with **0 `#[ignore]` regressions** left behind.
3. All **27 pre-audit watchlist defects (`BUG-A1`..`BUG-D9`)** are fixed with dedicated regression assertions and updated spec prose where contracts changed.
4. `examples/reference-course/` validates cleanly, passes `simulate --strict` with 0 warnings, and passes all 6 standard runners with **0 strict LMS violations**.
5. `crates/pptx/tests/fidelity/fidelity.pptx` passes all 19 visible slides (including animated Slide 20 multi-timestamp checks and Slide 14 style-less Table 2 gridlines).
