# M44 Plan — Issue #9: LMS Tracking Options

**Role:** Architecture, Runtime Semantics & Test Plan (planning only — zero source files edited)  
**Target release:** 0.5.1 / M44  
**Source request:** **GitHub issue #9** (*LMS Tracking Options*) — *"Be able to determine how to report course completion to the LMS. So if the user has to view all slides or From the Quiz results slide or from a button (trigger). During the Publishing stage there was an option to choose how the tracking will be reported."*

**Clean-room commitment:** Matches the completion-tracking capabilities authors expect from established slide-based authoring tools (slides viewed, quiz result, and completion triggers, selectable in any combination), implemented from scratch in pure Rust with CourseCraft's own schema, runtime contract, UI tokens (`DESIGN.md` / `crates/render/src/design.rs`), `craft-i18n` (`tl!`) localization, and generic clean-room wording (`"Tracking"`, `"Slides viewed"`, `"All slides"`, `"At least"`, `"Quiz result"`, `"Results slide"`, `"Completion triggers"`). Zero competitor code, assets, screenshots, or coined terminology (`docs/contributing.md`).

> **Implementer Guardrails (`AGENTS.md`, `docs/contributing.md`, `docs/testing/m28-uiux-plan.md`, `docs/ui/README.md`)**
> 1. **Never crash:** Zero `unwrap`/`expect`/`panic!`/`unreachable!`/`todo!` in non-test code; `unsafe_code = "forbid"` everywhere except `coursecraft-video-native`. Every numeric count, slide id, and status input is hostile.
> 2. **Everything is a command:** All authoring mutations live in `crates/engine/src/commands/` with a single-row `REGISTRY` entry declaring `scope` and `slide_validation`. UIs only dispatch.
> 3. **One runtime:** Every play-time tracking rule lives **only** in `coursecraft-logic`. Editor preview (`crates/ui-egui/src/preview.rs`), CLI `simulate`, and published packages (`apps/coursecraft-player`) all run the exact same `Player`.
> 4. **Spec first (same step):** `docs/spec/course-model.md` MUST be updated in `S1` (model step). `docs/spec/triggers.md` MUST be updated in `S2` (logic step) and `S3` (LMS bridge status-lattice updates). `docs/cli.md` MUST be updated in `S4` (engine command step). `docs/ui/feature-parity.md` and `docs/ui/shell-panels-dialogs.md` MUST be updated in `S5` (UI step).
> 5. **Backward compatibility (hard gate):** Model/serde byte-identical goldens (`TR-01`) land **first** in `S1`, and `suspend_data` byte-identical goldens (`TR-03`) land **first** in `S2`. Existing courses lacking `tracking` MUST load unchanged, behave identically (triggers only), serialize **byte-identically**, and resume existing `cc1:` `suspend_data` without any format change.
> 6. **Layering & WASM:** Never break `cargo xtask layers` or `wasm32-unknown-unknown` for L0–L5, the web editor, or the web player.
> 7. **Registry & CLI lock:** Adding `course.set_tracking` MUST update `r5_01_registry_golden_lock` (`crates/engine/src/tests.rs:12247`), `m27_all_commands_undo_tests.rs`, and `docs/cli.md` in `S4`.
> 8. **New UI & i18n rules (`docs/ui/localization.md`, M28):** Every user-facing string in new/modified UI goes through `tl!` / `craft_i18n::fmt` / `trn`; single shared body of code across Course Settings and Publish; sentence case, positive labels, no trailing colons, palette tokens only (`UX-06`, `UX-07`), ≥ 24×24 pt compact hit targets (`UX-05`), zero nested `ScrollArea`s (`UX-02`), and green `uiux_sweep_tests.rs`.

---

## 1. Goals

1. **Course-level tracking model:** Store tracking rules on `Course` (`course.tracking`) so the player applies them identically across SCORM 1.2, SCORM 2004 4th, cmi5, xAPI, LTI 1.3, Web, in-editor Preview, and CLI `simulate` — never trapped only in `editor.last_publish`.
2. **Any combination of three options (completes when ANY chosen option is met):**
   - **Slides viewed:** complete when the learner has entered all slides in play order, or at least `N` distinct slides (`N` from `1` to the play-order slide count), reusing the existing `PlayerState.visited` set so resume works with **zero** `suspend_data` format change.
   - **Quiz result:** evaluate a chosen results slide or the course quiz on submit, reporting `Passed` or `Failed` (and score) using the existing `submit_results` scoring and `course.quiz.passing_percent`.
   - **Triggers:** the author's own `complete_course` and `submit_results` triggers always keep working (and remain the default when `tracking` is omitted).
3. **Consistent, non-downgrading LMS status reporting:** Define exact behaviour for `completed`/`incomplete` vs `passed`/`failed` (consistent with `lms_status` for SCORM 1.2) across SCORM 1.2, SCORM 2004 4th, cmi5, xAPI, and LTI 1.3 — including combined slides-viewed + quiz, fail-then-view-all-slides, retries, and resume after relaunch — guaranteeing a reported `passed` or `completed` status is **never** downgraded.
4. **Helpful validation:** Warn (never hard-block editing) when quiz-result tracking points at no quiz/results slide or missing passing score, or when `N` exceeds the course slide count; suppress false *"never reports a result"* warnings when valid tracking is enabled.
5. **Clean-room authoring & automation:** Editable **Tracking** section in Course Settings (and summary in the docked Course inspector panel) and directly in the **Publish** window via one shared localized (`tl!`) UI module, backed by an atomic undoable engine command (`course.set_tracking`) driveable from CLI and MCP.

---

## 2. Verified Current State

Every file, type, and line reference below was verified directly against the current repository:

### 2.1 Course Model & Validation (`crates/course`)
- `crates/course/src/model.rs:328-379`: `Course` (`#[serde(deny_unknown_fields)]`) has `schema_version`, `id`, `title`, `slide_size`, `variables`, `scenes`, `masters`, `templates`, `banks`, `start_slide`, `quiz: Option<QuizSettings>`, `player`, `theme`, `language`, `strings`, `fonts`, `background_audio`, `text_styles`, `resources`, `glossary`, `editor: EditorSettings`. No `tracking` field exists yet.
- `crates/course/src/model.rs:393-423`: `LmsStatus` (`Auto | PassedIncomplete | PassedFailed | CompletedIncomplete | CompletedFailed`) and `PublishSettings { standard, lms_status }` live under `Course.editor.last_publish`.
- `crates/course/src/model.rs:823-826`: `QuizSettings { pub passing_percent: u8 }`.
- `crates/course/src/model.rs:935-960`: `Slide` has no `hidden` boolean and no `is_results` flag; a results slide created by `results.insert` (`crates/engine/src/commands/slide.rs:2314-2319`) is a standard `Slide` carrying `Trigger { action: Action::SubmitResults, when: Event::TimelineStarts { layer: None } }`.
- `crates/course/src/lib.rs:422-435`: `Course::reports_result(&self)` returns `true` only when a slide or layer trigger uses `Action::SubmitResults` or `Action::CompleteCourse { .. }`.
- `crates/course/src/validate.rs:1009-1016`: `check_course_results` emits a warning at `"scenes"` whenever `!course.reports_result()`.
- `crates/course/src/diff.rs:909`: `diff` compares course header fields (`put("quiz", to_json(&course.quiz))`).

### 2.2 Runtime (`crates/logic`)
- `crates/logic/src/lib.rs:633-765`: `PlayerState` stores `visited: BTreeSet<String>` (`:666`), `quiz_attempts: u32` (`:645`), `answers: BTreeMap<String, Answer>` (`:681`), `draw_seed: u32` (`:641`).
- `crates/logic/src/lib.rs:1041-1093`: `Player::finish_resume` restores `PlayerState`, builds `self.slides` from `draw::play_order(course, state.draw_seed)` (optionally filtered by `preview_subset`), and seeds runtime-only `timeline_completed` from `visited`.
- `crates/logic/src/lib.rs:1682-1700`: `Course.TotalSlides` equals `self.slides.len()` and `Course.Progress` counts `self.slides.iter().filter(|s| self.st.visited.contains(&s.id)).count()`.
- `crates/logic/src/run.rs:223-243`: `Player::handle_into` dispatches every `Input` and drains the event queue.
- `crates/logic/src/run.rs:755-778`: `Action::SubmitQuestion` calls `self.grade(q, out)`; `Action::SubmitResults` calls `self.results(out)`; `Action::CompleteCourse { status }` pushes `Effect::Completion { status: *status }`.
- `crates/logic/src/run.rs:1583, 2450`: `self.st.visited.insert(slide.id.clone())` runs on both `enter_slide_inner` and `enter_lightbox_slide`.
- `crates/logic/src/run.rs:4129-4151`: `Player::results` increments `quiz_attempts`, computes `(raw, max)` via `score_points_and_max()`, computes `percent` (`100` when `max == 0`), maps `status` via `self.course.quiz` (`Passed` / `Failed` / `Completed`), and emits `Effect::Score` then `Effect::Completion`.
- `crates/logic/src/suspend.rs:13-62, 400-418, 537-575`: `cc1:` binary suspend format already persists `visited` slide indices and `quiz_attempts`.

### 2.3 LMS Bridges & Publishing (`crates/lms`, `crates/xapi`, `crates/lti`, `crates/publish`)
- **SCORM 1.2 & 2004 (`crates/lms/src/bridge.rs`):**
  - `CmiMapping::write_status` (`:204-241`) maps `CompletionStatus` through `map_status(status, lms_status)` (`:877-904`) for SCORM 1.2, and writes separate `cmi.completion_status` / `cmi.success_status` for SCORM 2004.
  - **Gap spotted in `bridge.rs:727-814, 866-875`:** `LmsBridge` tracks `self.passed` and `self.completed` (for `exit`), but under SCORM 1.2 `LmsStatus::Auto`, `is_protected_status` returns `true` **only** for `LessonStatus::Passed`, NOT `LessonStatus::Completed`. If a learner completes via slides-viewed (`lesson_status = "completed"`) and then fails a quiz (`Score(40, Failed)`), `handle_score` currently overwrites `"completed"` down to `"failed"`. Similarly, in SCORM 2004, `Completion(Incomplete)` after `Completed` is not blocked unless `self.passed` is true.
- **cmi5 & xAPI (`crates/lms/src/lrs.rs`, `crates/xapi/src/package.rs`):**
  - `XapiBridge` (`lrs.rs:326-341, 761-837`) already sends separate `completed` vs `passed`/`failed` statements and persists `completed` / `passed` flags in `coursecraft.progress`. `enqueue_passed` / `enqueue_failed` already respect `passed_saved` and cmi5's one-success-verdict-per-session rule (`failed_this_session`).
  - **Gap spotted in `crates/xapi/src/package.rs:28-32` & `crates/publish/src/lib.rs:377-381, 412-418`:** `cmi5_xml` chooses `moveOn` strictly from `mastery_score.is_some()` (`"Passed"` vs `"Completed"`), and `publish/src/lib.rs` passes `course.quiz.as_ref().map(|q| q.passing_percent)` unconditionally for both `cmi5.xml` and SCORM `imsmanifest.xml`. When a course with a quiz tracks by slides-viewed-only (or slides-viewed + quiz), `cmi5.xml` must emit `moveOn="Completed"` (no `masteryScore`) or `moveOn="CompletedOrPassed"` (already accepted by `crates/strict-lms/src/manifest.rs:139`), and SCORM manifests should omit `mastery_score` for slides-viewed-only courses.
- **LTI 1.3 (`crates/lms/src/lti.rs`, `crates/lti/src/lib.rs:256-285`):**
  - **Gaps spotted in `lti.rs:65-129`:**
    1. When `Score(40, Failed)` and `Completion(Completed)` arrive in the same batch (e.g. entering the last slide which is also a failed results slide), `if !has_score` drops `Completion(Completed)` and sends `"failed"`.
    2. When `Completion(Completed)` was sent first (`percent: None`), a later `Score(40, Failed)` (`percent: Some(40)`) currently overwrites `"completed"` down to `"failed"`.
    3. When `Score(40, Failed)` was sent first (`self.sent_final = true`), a later `Completion(Completed)` (`percent: None`) is currently dropped by `if self.sent_final && report.percent.is_none()`.

### 2.4 Engine, Undo, CLI & Editor UI (`crates/engine`, `apps/coursecraft-cli`, `crates/ui-egui`)
- `crates/engine/src/delta.rs:823-960`: `CourseHeaderSnapshot` and `CourseHeaderPatch` destructure all 17 header fields of `Course` exhaustively (no `..`). Adding `Course.tracking` requires adding it there so `UndoScope::CourseHeader` works with zero full-course clones.
- `crates/engine/src/commands/course.rs:200-250` & `commands/mod.rs:138-159`: `course.set_quiz` and `course.set_publish_settings` use `UndoScope::CourseHeader`.
- `crates/engine/src/tests.rs:12247`: `r5_01_registry_golden_lock` locks all 189 commands (`id`, `mutates`, `scope`, `slide_validation`).
- `apps/coursecraft-cli/src/tests.rs:1206-1220`: asserts every engine command in `REGISTRY` appears in `docs/cli.md`.
- `crates/ui-egui/src/course_settings.rs:239-753`: Course Settings dialog (`Dialog::new("Course settings")`, single `Form` + `ScrollArea`).
- `crates/ui-egui/src/publish.rs:242-501`: Publish dialog (`Dialog::new("Publish course")`, standard picker + SCORM 1.2 `Report status as` combo).
- `crates/ui-egui/src/properties.rs:4951-5004`: `show_course_tab` draws the docked `Panel::CourseInspector` (`Title`, `Slides`, `Slide size`, `Pass mark`, `Course settings…`, `Theme…`).
- `crates/ui-egui/src/i18n.rs:88-92` & `i18n_tests.rs:55-76`: `tl!` macro and `Lang::PSEUDO` verification harness.
- `docs/ui/feature-parity.md:238`: currently lists `Tracking by slides viewed / quiz | course.set_publish_settings | ✅` (needs updating to `course.set_tracking` in `S5`).

---

## 3. Design

### 3.1 Course Model (`crates/course/src/model.rs`)

Add one optional field on `Course`:
```rust
/// Course-level LMS completion tracking options.
/// Omitted when default (triggers only) so existing courses stay byte-identical.
#[serde(default, skip_serializing_if = "TrackingSettings::is_default")]
pub tracking: TrackingSettings,
```

Define the tracking types with `#[serde(deny_unknown_fields)]`:
```rust
/// How slide-viewing completion is measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum SlidesViewedTracking {
    /// Complete once the learner has entered every slide in play order.
    All,
    /// Complete once the learner has entered at least `count` distinct slides in play order (`1..=slide_count`).
    AtLeast { count: u32 },
}

/// How quiz-result completion is measured.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuizTracking {
    /// Optional slide id of the results (or question) slide that reports the quiz.
    /// `None` evaluates the course's graded quiz on submit (or when all graded questions lock).
    /// Omitted when `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slide: Option<String>,
}

/// Course-level LMS completion tracking options.
///
/// The course reports completion to the LMS when ANY enabled option is met.
/// Author triggers (`complete_course`, `submit_results`) always keep working.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackingSettings {
    /// Complete after viewing all slides or at least N slides. Omitted when disabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slides_viewed: Option<SlidesViewedTracking>,
    /// Complete (passed or failed) from a quiz / results slide on submit. Omitted when disabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quiz: Option<QuizTracking>,
}

impl TrackingSettings {
    #[must_use]
    pub fn is_default(&self) -> bool {
        self.slides_viewed.is_none() && self.quiz.is_none()
    }
}
```

**Compatibility guarantees:**
- `schema_version` remains `1` (same additive-optional policy as `groups`, `point_text`, `resources`, `glossary`).
- Existing `course.json` files without `"tracking"` deserialize with `TrackingSettings::default()` (`slides_viewed: None, quiz: None`) and re-serialize **byte-identically** (omitting `"tracking"`).
- Old apps with `deny_unknown_fields` refuse files that explicitly set a non-default `"tracking"` object loudly rather than silently ignoring it.
- Author triggers (`complete_course`, `submit_results`) are never suppressed, so there is no boolean that could break existing courses' triggers.

### 3.2 Slide Counting, Validation & Helpers (`crates/course`)

**How slides, scenes, banks, masters, templates, and unreachable slides count:**
- **Play-order slides count:** Every slide in `course.scenes[].slides[]` counts. For a bank-draw placeholder slide (`slide.draw = Some(BankDraw { bank, count })`), it expands at play time into `count` drawn bank slides, so `Course::play_slide_count()` sums `slide.draw.as_ref().map_or(1, |d| d.count as usize)` across all scenes (matching runtime `self.slides.len()` and `Course.TotalSlides`).
- **Masters, templates, and undrawn bank slides NEVER count:** They are not in play order (`self.slides`) and can never be visited.
- **Unreachable / branched scene slides DO count toward `total`:** CourseCraft has no `slide.hidden` property; any slide in any scene is part of `self.slides` (matching `Course.TotalSlides` and `Course.Progress`). Authors with optional branches, remediation scenes, or lightbox-only slides use **At least N slides** to set the exact threshold learners must reach.
- **Lightboxed slides:** Opening a scene slide in a lightbox already marks it in `PlayerState.visited` (`docs/spec/triggers.md:314`, `run.rs:2450`), so it counts as viewed once entered.

**Helpers on `Course` and `Slide` (`crates/course/src/lib.rs`):**
- `Course::play_slide_count(&self) -> usize`
- `Slide::has_submit_results_trigger(&self) -> bool` (enabled `Action::SubmitResults` on base or any layer)
- `Course::results_slides(&self) -> impl Iterator<Item = &Slide>`
- `Course::has_graded_questions(&self) -> bool` (any scene slide or non-empty drawn bank slide with `question.max_points() > 0`)
- `Course::reports_result(&self) -> bool`: returns `true` if any slide has `SubmitResults` / `CompleteCourse` trigger (existing rule), **or** (`tracking.slides_viewed.is_some()` && `play_slide_count() > 0`), **or** (`tracking.quiz` is `Some(QuizTracking { slide: Some(id) })` where `slide_index(id).is_some()`), **or** (`tracking.quiz` is `Some(QuizTracking { slide: None })` && `has_graded_questions()`).

**Validation rules (`crates/course/src/validate.rs` — `check_tracking`):**

| Condition | Severity | Path | Message |
|---|---|---|---|
| `slides_viewed == Some(AtLeast { count: 0 })` | **Error** | `/tracking/slides_viewed/count` | `"slides_viewed count must be at least 1"` |
| `slides_viewed == Some(AtLeast { count })` and `count > play_slide_count()` (when `play_slide_count() > 0`) | **Warning** | `/tracking/slides_viewed/count` | `"slides_viewed count ({count}) exceeds the course slide count ({total}); completion will trigger after viewing all {total} slides"` |
| `quiz == Some(QuizTracking { slide: Some(id) })` and `course.slide_index(id).is_none()` | **Warning** | `/tracking/quiz/slide` | `"quiz tracking slide {id:?} does not exist in play order"` |
| `quiz == Some(QuizTracking { slide: Some(id) })`, slide exists, has neither `submit_results` trigger nor `question` | **Warning** | `/tracking/quiz/slide` | `"quiz tracking slide {id:?} has no Submit results trigger or question; quiz score will be reported on slide entry"` |
| `quiz == Some(QuizTracking { slide: None })` and `!course.has_graded_questions()` and no results slide | **Warning** | `/tracking/quiz` | `"quiz result tracking is enabled, but the course has no graded questions or results slide"` |
| `quiz.is_some()` and `course.quiz.is_none()` | **Warning** | `/tracking/quiz` | `"quiz result tracking is enabled, but no passing score is set in Quiz settings"` |

Warnings never block `keep_valid` (so deleting a slide or results slide still succeeds and immediately surfaces a clear warning in Check Course and Publish).

### 3.3 Runtime Semantics (`coursecraft-logic` + `docs/spec/triggers.md`)

All play-time rules live exclusively in `coursecraft-logic` and are documented in `docs/spec/triggers.md` (`## Tracking`) in `S2`:

1. **Runtime-only state (`PlayerState`):**
   - Add `#[serde(default, skip_serializing)] pub slides_tracking_reported: bool`.
   - **Never written to `suspend_data`** (`cc1:` format is untouched and 100% byte-identical).
   - In `Player::finish_resume`: if `tracking.slides_viewed` is enabled, `preview_subset.is_none()`, and the restored `visited` set *already* satisfies the slides-viewed threshold (e.g., learner already completed on a prior session and relaunched), initialize `slides_tracking_reported = true` so relaunching does not emit a spurious duplicate `Effect::Completion`. If restored `visited` is below the threshold (e.g. 2 of 4 slides), `slides_tracking_reported` starts `false` and fires as soon as the learner visits the remaining slides.

2. **Evaluation point (end of `Player::handle_into`, after `drain(&mut q, out)`):**
   Record `attempts_before = self.st.quiz_attempts` and `slide_before` / `entered_slide_id` before running the input. After event processing (`drain`) finishes (and if not in `reviewing_from.is_some()`):
   - **Option 3 — Triggers (always active):** Any `Action::CompleteCourse` or `Action::SubmitResults` executed during `drain` has already emitted its `Effect::Completion` / `Effect::Score` normally.
   - **Option 2 — Quiz result (`tracking.quiz = Some(qt)`):**
     Evaluated **on submit**, reusing `self.results(out)` (`score_points_and_max()`, `course.quiz.passing_percent`, `Effect::Score` + `Effect::Completion`, `quiz_attempts += 1`):
     - If `self.st.quiz_attempts > attempts_before`, `Action::SubmitResults` already ran in this input — do **not** duplicate it.
     - Otherwise, if `qt.slide == Some(target_id)`:
       - If the target slide has an enabled `Action::SubmitResults` trigger (normal results slide from `results.insert` or author-built): that trigger owns when submit runs (on `timeline_starts` or button click) — do not auto-fire early.
       - Else if the target slide has a `question` and was just locked by `Effect::QuestionAnswered` in this input: call `self.results(out)`.
       - Else (custom summary slide with neither trigger nor question): if `target_id` was entered in this input, call `self.results(out)`.
     - Otherwise (`qt.slide == None` — course quiz):
       - If an `Effect::QuestionAnswered` occurred in this input, and there is at least one graded question (`max_points() > 0`) in `self.slides`, and **every** graded question in `self.slides` is now locked (`question_locked_on_slide`): call `self.results(out)`.
     - **Retry (`retry_quiz`):** `retry_quiz` clears `answers` (unlocking all questions), so when the learner re-submits the quiz or re-enters the results slide, `self.results(out)` runs again with the new score and emits updated `Effect::Score` + `Effect::Completion` (non-downgrade is enforced by the LMS bridges).
   - **Option 1 — Slides viewed (`tracking.slides_viewed = Some(sv)`):**
     - Applies when `!self.st.slides_tracking_reported` and `self.preview_subset.is_none()` (partial-slide preview cannot view the whole course; full-course preview has `preview_subset == None` and matches published 100%).
     - Count `total = self.slides.len()` and `visited = self.slides.iter().filter(|s| self.st.visited.contains(&s.id)).count()`.
     - Effective threshold: `All => total`; `AtLeast { count } => (count as usize).clamp(1, total)` (clamping `count > total` prevents trapping learners if a slide was deleted after configuring tracking).
     - When `total > 0 && visited >= threshold`:
       - Set `self.st.slides_tracking_reported = true`.
       - If this input has NOT already emitted `Effect::Completion { status: Passed | Completed }`, push `Effect::Completion { status: CompletionStatus::Completed }`. (Note: if this input emitted `Score(Failed)` + `Completion(Failed)` AND simultaneously crossed the slides-viewed threshold, `Completion(Completed)` **is** emitted after them so the LMS bridges know completion was satisfied by slides viewed while preserving the quiz score/success verdict!)
     - Does **not** emit `Effect::Exit` (learner stays on the slide and can keep reading or click Finish).

### 3.4 LMS Status Mapping & Never-Downgrade Rules (All 5 Standards)

**Core invariant:** Completion and success are evaluated monotonically; once a learner achieves `completed` (or `passed`), **neither a later failure, an incomplete status, a lower score, nor a relaunch/resume may ever downgrade it.**

#### Per-Standard Mapping Table

| Scenario | SCORM 1.2 (`auto`) | SCORM 1.2 (`passed_incomplete` / `passed_failed`) | SCORM 1.2 (`completed_incomplete` / `completed_failed`) | SCORM 2004 4th | cmi5 / xAPI | LTI 1.3 (AGS) |
|---|---|---|---|---|---|---|
| **Slides viewed met** (`Completion(Completed)`) | `lesson_status = "completed"` | `lesson_status = "passed"` | `lesson_status = "completed"` | `completion_status = "completed"` (`success_status` unchanged) | `completed` statement | `{percent: null, status: "completed"}` |
| **Quiz passed** (`Score(p, Passed)`) | `score.raw = p`, `lesson_status = "passed"` | `score.raw = p`, `lesson_status = "passed"` | `score.raw = p`, `lesson_status = "completed"` | `score.raw = p`, `scaled = p/100`, `completion = "completed"`, `success = "passed"` | `passed` (scaled `p/100`) + `completed` | `{percent: p, status: "passed"}` |
| **Quiz failed** (`Score(p, Failed)`) | `score.raw = p`, `lesson_status = "failed"` | `score.raw = p`, `lesson_status = "incomplete"` / `"failed"` | `score.raw = p`, `lesson_status = "incomplete"` / `"failed"` | `score.raw = p`, `scaled = p/100`, `completion = "completed"`, `success = "failed"` | `failed` (scaled `p/100`) + `completed` | `{percent: p, status: "failed"}` |
| **Both enabled: quiz fails (40%), THEN all slides viewed** | Upgrades `lesson_status` `"failed"` → `"completed"`; keeps `score.raw = 40` | Upgrades `lesson_status` → `"passed"`; keeps `score.raw = 40` | Upgrades `lesson_status` → `"completed"`; keeps `score.raw = 40` | `completion = "completed"`, `success = "failed"`, `score = 40` | `failed` (0.4) + `completed` (deduped) | Upgrades to `{percent: 40, status: "completed"}` |
| **Both enabled: all slides viewed FIRST, THEN quiz fails (40%)** | Writes `score.raw = 40`; `lesson_status` **stays `"completed"`** (no downgrade) | Protected (`passed` already set); no downgrade | Protected (`completed` already set); no downgrade | Writes `score.raw = 40`, `scaled = 0.4`, `success = "failed"`; `completion` stays `"completed"` | `completed` then `failed` (0.4) | Ignores `failed` (status stays `"completed"`, no downgrade) |
| **Both enabled: all slides viewed FIRST, THEN quiz passes (90%)** | Upgrades `lesson_status` `"completed"` → `"passed"`, `score.raw = 90` | Writes `score.raw = 90`, stays `"passed"` | Writes `score.raw = 90`, stays `"completed"` | Writes `score = 90`, `scaled = 0.9`, `completion = "completed"`, `success = "passed"` | `completed` then `passed` (0.9) | Upgrades to `{percent: 90, status: "passed"}` |
| **Retry after fail → passes (90%)** | Upgrades `"failed"`/`"completed"` → `"passed"`, `score.raw = 90` | Upgrades → `"passed"`, `score.raw = 90` | Upgrades → `"completed"`, `score.raw = 90` | Upgrades `success` `"failed"` → `"passed"`, `score = 90` | New session: sends `passed` (0.9). Same cmi5 session: cmi5 forbids both `failed` and `passed` in one session (`failed_this_session` guard) | Upgrades to `{percent: 90, status: "passed"}` |
| **Retry after pass (90%) → fails (20%) or lower pass (70%)** | Ignored (stays `"passed"`, score `90`) | Ignored (stays `"passed"`, score `90`) | Ignored (stays `"completed"`, score `90`) | Ignored (stays `"completed"` + `"passed"`, score `90`) | Ignored (`passed_saved == true`) | Ignored (`passed == true`, high-water score `90`) |
| **Resume after relaunch (already completed/passed)** | `ever_completed`/`passed` seeded from `lesson_status`; no downgrade, no duplicate completion | Same | Same | `ever_completed`/`passed` seeded from `completion_status`/`success_status`; no downgrade | `completed_saved`/`passed_saved` seeded from `coursecraft.progress`; no duplicate/downgrade | LTI has no suspend data (starts fresh per launch) |

#### Manifest Rules (`crates/xapi/src/package.rs`, `crates/publish/src/lib.rs`)
- **`cmi5.xml` `moveOn` & `masteryScore`:**
  - `tracking.is_default()` (existing courses): `Auto` → `"Passed"` + `masteryScore` if `course.quiz.is_some()`, else `"Completed"` (byte-identical to today).
  - `slides_viewed: Some(_), quiz: None`: `moveOn="Completed"`, omit `masteryScore`.
  - `slides_viewed: None, quiz: Some(_)`: `moveOn="Passed"` + `masteryScore` if `course.quiz.is_some()`, else `moveOn="Completed"`.
  - `slides_viewed: Some(_), quiz: Some(_)`: `moveOn="CompletedOrPassed"` + `masteryScore` if `course.quiz.is_some()`.
- **SCORM 1.2 & 2004 `imsmanifest.xml`:**
  - `slides_viewed: Some(_), quiz: None`: omit `mastery_score` so LMSs do not override slides-viewed completion with a missing-score failure.
  - Otherwise: pass `course.quiz.as_ref().map(|q| q.passing_percent)` (byte-identical to today).

### 3.5 Engine Command, Undo & CLI/MCP (`crates/engine`, `apps/coursecraft-cli`)

- **Command ID:** `course.set_tracking`
- **Scope:** `UndoScope::CourseHeader`, `mutates: true`, `slide_validation: false`
- **Params:**
  ```json
  {
    "slides_viewed"?: { "mode": "all" } | { "mode": "at_least", "count": 1.. } | null,
    "quiz"?: { "slide"?: id | null } | null
  }
  ```
- **Rules:**
  - At least one of `slides_viewed` or `quiz` must be supplied (rejects `{}` with `EngineError::Invalid`).
  - `slides_viewed.count == 0` or negative/non-integer is rejected with `EngineError::Invalid`.
  - `quiz.slide` (when `Some(id)`) is validated with `validate_id`. Missing slide IDs are allowed with a validation warning (never panicked; `keep_valid` passes because warnings don't block).
  - Returns `{"tracking": <TrackingSettings>}`.
- **Undo (`crates/engine/src/delta.rs`):** Add `tracking` to `CourseHeaderSnapshot` and `CourseHeaderPatch` so `course.set_tracking` undoes and redoes in 1 atomic delta step with zero full-course clones.
- **CLI & MCP:** Works directly via `coursecraft-cli run`, `coursecraft-cli batch`, `coursecraft-cli editor command.run`, and MCP `command_run`; `coursecraft-cli diff` reports tracking changes; `docs/cli.md` documents `course.set_tracking` in `S4`.

### 3.6 Editor UI (`crates/ui-egui`)

1. **Shared module (`crates/ui-egui/src/tracking_ui.rs`):**
   - `pub fn show_tracking_controls(editor: &mut Editor, ui: &mut egui::Ui, enabled: bool)`:
     - Heading/caption and helper note: `tl!("Report completion to the LMS when any selected option is met.")`
     - Checkbox `tl!("Slides viewed")`:
       - When checked, indented controls: radio `tl!("All slides")` (showing total slide count) and radio `tl!("At least")` + numeric `DragValue` (`1..=total.max(1)`) + `tl!("slides")`.
       - Inline `p.warning` note when `count > total`.
     - Checkbox `tl!("Quiz result")`:
       - When checked, indented `Results slide` combo listing every results slide (`course.results_slides()`) plus `tl!("Course quiz (all graded questions)")` (`slide: None`).
       - Quiet secondary note with current passing score (`course.quiz`), or inline `p.warning` note if no graded questions/results slide or no passing score is set.
     - Quiet active row (not a misleading disabled checkbox): check icon + `tl!("Completion triggers are always active")` with tooltip `tl!("Complete course and Submit results triggers always report to the LMS.")`.
     - Every user interaction immediately dispatches `course.set_tracking` (single undo step).
     - Zero nested `ScrollArea`s (`UX-02`), palette tokens only (`UX-06`, `UX-07`), positive sentence-case labels with no trailing colons (`UX-11`), ≥ 24×24 pt hit targets (`UX-05`), and every string through `tl!` / `craft_i18n::fmt` / `trn`.
   - `pub fn tracking_summary(course: &Course) -> String`: concise localized summary (e.g. `"Slides viewed (all), Quiz result, Triggers"` or `"Triggers only"`).
2. **Course Settings window (`crates/ui-egui/src/course_settings.rs`):**
   - Add a **Tracking** section right below **Quiz settings**, calling `tracking_ui::show_tracking_controls(editor, ui, true)`.
   - `needs_result = !editor.session.course.reports_result()` automatically clears the *"add a Finish button"* warning once valid tracking is enabled.
3. **Course Inspector dock panel (`crates/ui-egui/src/properties.rs:4951` — `show_course_tab`):**
   - Add a `Tracking` summary row displaying `tracking_ui::tracking_summary(&editor.session.course)` right below `Pass mark`, keeping the docked Course panel informative at a glance.
4. **Publish window (`crates/ui-egui/src/publish.rs`):**
   - For all LMS standards (`Scorm12`, `Scorm2004`, `Cmi5`, `Xapi`, `Lti`): display a `Tracking` section inside `dialog.body` (below the standard picker / SCORM 1.2 `Report status as` combo) calling `tracking_ui::show_tracking_controls(editor, ui, !busy)` (with disabled tooltip `tl!("Wait for publishing to finish")` when `busy`).
   - For `PublishStandard::Web`: show a quiet secondary note `tl!("Web packages do not report completion to an LMS.")`.

---

## 4. Watchlist of Risky Spots (`RISK-01`–`RISK-09`)

| ID | Area / File | Risk & Concrete Mitigation |
|---|---|---|
| **RISK-01** | `crates/lms/src/bridge.rs:727-814, 866-875` | Under SCORM 1.2 `LmsStatus::Auto`, `is_protected_status` currently protects only `Passed`, not `Completed` — a quiz fail after slides-viewed completion would downgrade `"completed"` to `"failed"`. **Mitigation:** Add `ever_completed: bool` seeded on `start()` from LMS status and enforce `Incomplete < Failed < Completed < Passed` in `S3`, locked by `TR-08` and `TR-09`. |
| **RISK-02** | `crates/lms/src/lti.rs:65-129` | Same-batch `[Score(Failed), Completion(Completed)]` drops `Completion`, and cross-batch `Failed` → `Completed` is blocked by `sent_final && percent.is_none()`, while `Completed` → `Failed` downgrades. **Mitigation:** Implement monotonic status handling in `LtiReporter::record` preserving existing `lti_tests.rs` invariants (`TR-11`). |
| **RISK-03** | Double-reporting on Results slide entry (`crates/logic/src/run.rs`) | A standard `results.insert` slide fires `Action::SubmitResults` on `timeline_starts`. If `tracking.quiz` also fired on slide entry, `quiz_attempts` would double-increment and emit duplicate `Score`/`Completion` effects. **Mitigation:** Track `attempts_before` per input and never auto-fire on slides that already own an enabled `SubmitResults` trigger (`TR-06`). |
| **RISK-04** | Spurious completion on resume after already completed (`crates/logic/src/lib.rs:1041`) | Resuming a session whose `visited` set already meets `slides_viewed` could re-emit `Effect::Completion` on `Input::Start`. **Mitigation:** Initialize `slides_tracking_reported = true` in `Player::finish_resume` when restored `visited` already meets the threshold, without touching `cc1:` `suspend_data` (`TR-03`, `TR-07`). |
| **RISK-05** | `cmi5.xml` `moveOn` mismatch (`crates/xapi/src/package.rs`, `crates/publish/src/lib.rs`) | Emitting `moveOn="Passed"` on a slides-viewed-only course (or without `CompletedOrPassed` on a slides+quiz course) breaks cmi5 LMS satisfaction. **Mitigation:** Derive `moveOn` and `masteryScore` from `(course.tracking, course.quiz)` while keeping `tracking.is_default()` byte-identical (`TR-12`). |
| **RISK-06** | `crates/engine/src/delta.rs:823-960` | `CourseHeaderSnapshot` / `CourseHeaderPatch` destructure `Course` exhaustively. Forgetting `tracking` breaks compilation or undo. **Mitigation:** Wire `tracking` in `S1` and verify with `m27_all_commands_undo_tests` in `S4`. |
| **RISK-07** | `r5_01_registry_golden_lock` (`crates/engine/src/tests.rs:12247`) & `docs/cli.md` | Adding `course.set_tracking` breaks the registry golden count (189 → 190) and CLI docs sync test. **Mitigation:** Update both in `S4` and verify with `coursecraft-engine` and `coursecraft-cli` tests. |
| **RISK-08** | Partial-slide Preview (`preview_subset.is_some()`) | Previewing 1 slide of a 10-slide course with `slides_viewed: All` would falsely trigger completion on slide 1 if checked against subset length. **Mitigation:** Skip automatic `slides_viewed` emission when `self.preview_subset.is_some()`, while keeping full-course preview 100% identical to published (`TR-05`, `TR-17`). |
| **RISK-09** | M28 UI/UX rules & `tl!` coverage (`crates/ui-egui`) | Publish dialog at `1024×600` scale `1.5` has tight vertical/horizontal space; any nested `ScrollArea`, hardcoded colour, trailing colon, or unwrapped English string breaks CI/audit. **Mitigation:** Single shared `tracking_ui.rs` using `tl!`, zero nested scrolls, compact layout, locked by `TR-16` and `TR-18`. |

---

## 5. Concrete Test Plan (`TR-01`–`TR-18`)

> **Ordering rule:** Backward-compatibility goldens (`TR-01` for `course.json`, `TR-03` for `suspend_data`) are written **first** before changing model/runtime code.

| ID | Crate / File | Category | Behaviour Tested & Expected Result |
|---|---|---|---|
| **TR-01** | `coursecraft-course` (`tests.rs`) | **Model/serde goldens (FIRST)** | `examples/hello-course/course.json` and `examples/showcase/course.json` load with `tracking == TrackingSettings::default()` and re-serialize **byte-identically** (no `"tracking"` key). Every `TrackingSettings` variant (`All`, `AtLeast { count }`, `QuizTracking { slide: Some/None }`, combined) round-trips cleanly; unknown fields inside `tracking` are rejected (`deny_unknown_fields`). |
| **TR-02** | `coursecraft-course` (`tests.rs`) | Validation, `reports_result`, `diff` | Rejects `AtLeast { count: 0 }` (error). Warns on: `count > play_slide_count()` (including bank-draw counts), `quiz.slide` missing/master/bank, `quiz.slide` without `submit_results`/question, `quiz: { slide: None }` with no graded questions, `quiz.is_some()` with `course.quiz.is_none()`. Confirms `reports_result()` returns `true` for valid `slides_viewed` or `quiz` tracking (suppressing false *"add a Finish button"* warning), and `coursecraft_course::diff` reports tracking changes. |
| **TR-03** | `coursecraft-logic` (`suspend.rs` / `tests.rs`) | **`suspend_data` golden (FIRST)** | Pre-M44 `cc1:` golden strings for `hello-course` and a plain multi-slide course decode cleanly on courses with and without `tracking`, and re-encode **byte-identically** (zero `suspend_data` format change). |
| **TR-04** | `coursecraft-logic` (`tests.rs`) | Default / triggers-only parity | With `tracking == default`, `examples/hello-course` and `examples/showcase` produce **identical** effect traces to pre-M44; `complete_course` and `submit_results` triggers keep working both alone and alongside `slides_viewed`/`quiz`. |
| **TR-05** | `coursecraft-logic` (`tests.rs`) | Slides-viewed runtime | Tests: (a) `All` on 3-slide course completes exactly on entering slide 3 (once, no duplicate on revisit); (b) `AtLeast { count: 2 }` on 4-slide course with unreachable scene completes on slide 2; (c) `AtLeast { count: 10 }` on 3-slide course clamps to 3 and completes when all 3 are viewed; (d) 1-slide course with `All` completes on `Input::Start`; (e) lightboxed slide counts as visited; (f) bank-draw course counts expanded `self.slides.len()`, never masters/templates/undrawn bank slides; (g) partial `preview_subset` does not falsely complete on slide 1. |
| **TR-06** | `coursecraft-logic` (`tests.rs`) | Quiz-result runtime | Tests: (a) `QuizTracking { slide: Some("results") }` on a `results.insert` slide emits `Score` + `Completion` **once** (no duplicate, `quiz_attempts == 1`); (b) custom results slide without `submit_results` trigger reports on entry; (c) `QuizTracking { slide: None }` waits until all graded questions lock (respecting multi-attempt questions), then emits `Score` + `Completion` (`Passed` vs `Failed`); (d) `retry_quiz` clears answers and re-evaluates on next submit; (e) survey-only / `course.quiz: None` degrades safely without panic. |
| **TR-07** | `coursecraft-logic` (`tests.rs`) | Combinations & resume | Tests with BOTH `slides_viewed` AND `quiz`: (a) pass quiz early → `Score(Passed)` + `Completion(Passed)`, then view remaining slides → `Completion(Completed)` emitted once; (b) fail quiz first, then view last slide → `Score(Failed)` + `Completion(Failed)` then `Completion(Completed)`; (c) view all slides first, then fail quiz; (d) enter last slide which is also a failed results slide → `Score(Failed)`, `Completion(Failed)`, `Completion(Completed)`; (e) suspend at 2/3 slides → resume → no premature completion on `Start`, completes on slide 3; (f) suspend at 3/3 slides → resume → `slides_tracking_reported == true`, no duplicate completion on `Start`. |
| **TR-08** | `coursecraft-lms` (`tests.rs`) | SCORM 1.2 bridge | Tests all 5 `LmsStatus` modes (`auto`, `passed_incomplete`, `passed_failed`, `completed_incomplete`, `completed_failed`) across slides-viewed, quiz pass/fail, fail-then-view-all-slides, view-all-slides-then-fail (never downgrades `"completed"` → `"failed"` under `auto`!), view-all-slides-then-pass (upgrades `"completed"` → `"passed"` + score), retry pass→fail (never downgrades), and resume after relaunch with `strict-lms` SCORM 1.2 validation. |
| **TR-09** | `coursecraft-lms` (`tests.rs`) | SCORM 2004 4th bridge | Tests separate `cmi.completion_status` and `cmi.success_status`: slides-viewed sets `completion_status = "completed"` only; quiz pass sets both `"completed"` + `"passed"` + scaled score; slides-viewed then quiz fail keeps `"completed"` + sets `"failed"` + score; retry to pass upgrades `"failed"` → `"passed"` + higher score; later fail never downgrades `"passed"` or `"completed"`; resume preserves both; verified with `strict-lms` SCORM 2004. |
| **TR-10** | `coursecraft-lms` (`lrs_tests.rs`) | cmi5 & xAPI bridge | Tests `XapiBridge` (both `Flavor::Cmi5` and `Flavor::Xapi`): slides-viewed emits `completed`; slides-viewed + quiz fail emits `completed` + `failed` (in either order, zero duplicate `completed`); slides-viewed + quiz pass emits `completed` + `passed`; relaunch after fail → retry pass emits `passed`; relaunch after pass → fail ignored (`passed_saved`); verified with `strict-lms` xAPI/cmi5 validator. |
| **TR-11** | `coursecraft-lms` (`lti_tests.rs`) | LTI 1.3 reporter | Tests `LtiReporter`: slides-viewed (`percent: None, status: "completed"`), fail-then-view-all-slides upgrades to `{percent: Some(40), status: "completed"}`, same-batch `[Score(40, Failed), Completion(Completed)]` emits `{percent: Some(40), status: "completed"}`, view-all-slides-then-fail never downgrades `"completed"` to `"failed"`, view-all-slides-then-pass upgrades to `{percent: Some(90), status: "passed"}`, pass→fail never downgrades. |
| **TR-12** | `coursecraft-publish` & `coursecraft-xapi` (`tests.rs`) | Manifest & publish parity | Verifies: (a) default `tracking` publishes byte-identical `course.json`, `imsmanifest.xml`, and `cmi5.xml`; (b) `slides_viewed` only emits `cmi5.xml` `moveOn="Completed"` (no `masteryScore`) and SCORM manifest without `mastery_score`; (c) `slides_viewed` + `quiz` emits `cmi5.xml` `moveOn="CompletedOrPassed"` with `masteryScore`; (d) `strict-lms` `validate_cmi5_xml` and SCORM manifest checks report 0 violations; (e) published `course.json` preserves `course.tracking`. |
| **TR-13** | `coursecraft-engine` (`tests.rs`, `m27_all_commands_undo_tests.rs`) | Engine command & undo | `course.set_tracking` sets/clears `slides_viewed` (`all`, `at_least`, `null`) and `quiz` (`slide`, `null`), rejects `{}` and `count: 0`, runs single-step undo/redo via `CourseHeaderPatch` (differential undo test green), passes `panic_hunt`, and matches updated `r5_01_registry_golden_lock` (190 commands). |
| **TR-14** | `coursecraft-cli` & `coursecraft-mcp` (`tests.rs`) | CLI, MCP & `docs/cli.md` | `coursecraft-cli run` executes `course.set_tracking` script → `validate` checks warnings → `simulate` verifies slides-viewed & quiz completion effects → `diff` reports tracking change; `docs/cli.md` sync test passes; MCP `command_list` / `command_run` executes `course.set_tracking`. |
| **TR-15** | `coursecraft-ui-egui` (`tests.rs`) | Editor UI & undo | Headless `TestHarness`: toggle **Slides viewed** (`All` and `At least N`), toggle **Quiz result** (pick results slide and course quiz) in **Course settings** → verify **Publish** window and docked **Course** inspector reflect the exact state → edit directly in **Publish** → verify 1-step `Ctrl+Z` / `Ctrl+Shift+Z` undo/redo → verify warning clears in Course Settings and displays cleanly when `N > slide_count` or no quiz exists. |
| **TR-16** | `coursecraft-ui-egui` (`i18n_tests.rs`) | `tl!` localization | Draws Course Settings and Publish window in `Lang::PSEUDO` (with both Slides viewed and Quiz result checked) and asserts every new Tracking label/hint/warning is rendered through `tl!` (`⟦…⟧`). |
| **TR-17** | `coursecraft-ui-egui` + `coursecraft-player` | Preview vs Published parity | Drives the same tracking course (slides-viewed, quiz-result, and combined) through Editor `Preview` (`crates/ui-egui/src/preview.rs`), `coursecraft-player`, and `Player::handle` (`simulate` path), asserting identical `Effect::Completion` and `Effect::Score` sequences and slide-entry counts. |
| **TR-18** | `coursecraft-ui-egui` (`uiux_sweep_tests.rs`) | M28 UI/UX sweep | Full sweep (`1024×600`, `1280×800`, `1920×1080` × Dark/Light × scale `1.0`/`1.5`) with non-default Tracking enabled across `course-settings`, `publish`, and `inspector-course`: **0 MUST violations** (`UX-01`–`UX-19`). |

---

## 6. Numbered Implementer Steps (6 Steps, One Hydrogen Run Each)

> **Gate cadence:** Run targeted `cargo test -p <crate> -j 4 <filter>` after every step. Run full `cargo xtask ci` after **S2, S3, S5, and S6** (plus `cargo xtask build-player` and release perf tests at **S6**).

- **S1 — Backward-Compatibility Goldens, `TrackingSettings` Model, Validation, `Course::diff`, Undo Header Delta & `docs/spec/course-model.md`**
  - **Files:** `crates/course/src/model.rs`, `crates/course/src/lib.rs`, `crates/course/src/validate.rs`, `crates/course/src/diff.rs`, `crates/course/src/tests.rs`, `crates/engine/src/delta.rs`, `docs/spec/course-model.md`, plus any exhaustive `Course { .. }` struct literals in workspace tests.
  - **Exact changes:**
    1. **First**, add golden tests (`TR-01`) asserting `examples/hello-course/course.json` and `examples/showcase/course.json` round-trip byte-identically without `"tracking"`.
    2. Add `SlidesViewedTracking`, `QuizTracking`, `TrackingSettings` (`is_default`), and `Course.tracking` (`#[serde(default, skip_serializing_if = "TrackingSettings::is_default")]`) in `crates/course/src/model.rs` and re-export in `crates/course/src/lib.rs`.
    3. Add `Course::play_slide_count`, `Course::results_slides`, `Course::has_graded_questions`, `Slide::has_submit_results_trigger`, and update `Course::reports_result` in `crates/course/src/lib.rs`.
    4. Add `check_tracking` to `crates/course/src/validate.rs` per §3.2 (error on `count == 0`, warnings on `count > play_slide_count()`, missing/non-results `quiz.slide`, no graded questions, or missing `course.quiz`).
    5. Add `put("tracking", to_json(&course.tracking))` in `crates/course/src/diff.rs` and wire `tracking: TrackingSettings` into `CourseHeaderSnapshot` and `CourseHeaderPatch` in `crates/engine/src/delta.rs`.
    6. Document `Course.tracking` in `docs/spec/course-model.md` (in the tree diagram and a new `## Tracking` section).
  - **Tests:** `TR-01`, `TR-02`; run `cargo test -p coursecraft-course -j 4` and `cargo test -p coursecraft-engine -j 4 delta`.
  - **Done when:** Existing course goldens are byte-identical, all validation warnings/errors and `reports_result` rules pass, `CourseHeaderPatch` round-trips `tracking`, and the workspace compiles cleanly.

- **S2 — Runtime Tracking Semantics in `coursecraft-logic` + `docs/spec/triggers.md` (Same Step!)**
  - **Files:** `docs/spec/triggers.md`, `crates/logic/src/lib.rs`, `crates/logic/src/run.rs`, `crates/logic/src/tests.rs`.
  - **Exact changes:**
    1. **First**, add `TR-03` asserting existing `cc1:` `suspend_data` goldens decode and re-encode byte-identically on courses with and without `tracking`.
    2. Add the normative `## Tracking` section to `docs/spec/triggers.md` documenting all three options, slide-counting rules (scenes, draws, unreachable slides, lightboxes, masters/templates), quiz evaluation on submit, combinations, retry, resume, and partial-preview behaviour (project rule: spec in same commit!).
    3. Add runtime-only `#[serde(default, skip_serializing)] pub slides_tracking_reported: bool` to `PlayerState` (`crates/logic/src/lib.rs`), and initialize it in `Player::finish_resume` when restored `visited` already meets the slides-viewed threshold.
    4. Implement end-of-input tracking evaluation in `Player::handle_into` (`crates/logic/src/run.rs`) per §3.3 (quiz evaluation on submit without duplicating `SubmitResults`, slides-viewed completion using `self.st.visited`, handling lightboxes, bank draws, retries, and `preview_subset`).
  - **Tests:** `TR-03`, `TR-04`, `TR-05`, `TR-06`, `TR-07`; run `cargo test -p coursecraft-logic -j 4` and **Gate #1:** `cargo xtask ci`.
  - **Done when:** Every runtime option, combination, retry, and resume test passes, `suspend_data` is byte-identical, `docs/spec/triggers.md` matches the implementation line-for-line, and `cargo xtask ci` is green.

- **S3 — LMS Bridges (SCORM 1.2, SCORM 2004, cmi5, xAPI, LTI 1.3), Publish Manifests & Spec Status Table**
  - **Files:** `crates/lms/src/bridge.rs`, `crates/lms/src/lti.rs`, `crates/lms/src/tests.rs`, `crates/lms/src/lrs_tests.rs`, `crates/lms/src/lti_tests.rs`, `crates/xapi/src/package.rs`, `crates/xapi/src/tests.rs`, `crates/publish/src/lib.rs`, `crates/publish/src/tests.rs`, `docs/spec/triggers.md`.
  - **Exact changes:**
    1. In `crates/lms/src/bridge.rs`: add `ever_completed: bool` seeded from LMS statuses on `start()`; enforce monotonic non-downgrade (`Incomplete < Failed < Completed < Passed` under SCORM 1.2 `Auto`, preserving score writes without downgrading `completed` → `failed`; independent non-downgrading `cmi.completion_status` and `cmi.success_status` under SCORM 2004).
    2. In `crates/lms/src/lti.rs`: add `ever_completed` and `last_failed_score` to `LtiReporter` so same-batch `[Score(Failed), Completion(Completed)]` emits `{percent: Some(p), status: "completed"}`, cross-batch `Failed` → `Completed` upgrades while preserving score, `Completed` is never downgraded by `Failed`/`Incomplete`, and `Completed` → `Passed` upgrades cleanly.
    3. In `crates/xapi/src/package.rs` and `crates/publish/src/lib.rs`: support `MoveOn` (`Auto | Completed | Passed | CompletedOrPassed`) on `PackageSpec`, emitting `Completed` (no `masteryScore`) for slides-viewed-only, `CompletedOrPassed` for slides+quiz, and keeping default `tracking` byte-identical; omit SCORM manifest `mastery_score` for slides-viewed-only courses.
    4. Update the LMS status-mapping and non-downgrade section in `docs/spec/triggers.md` with the per-standard rules from §3.4.
  - **Tests:** `TR-08`, `TR-09`, `TR-10`, `TR-11`, `TR-12`; run `cargo test -p coursecraft-lms -j 4`, `cargo test -p coursecraft-xapi -j 4`, `cargo test -p coursecraft-publish -j 4`, and **Gate #2:** `cargo xtask ci`.
  - **Done when:** All 5 LMS standards pass strict-lms and downgrade/resume/retry matrices, default manifests are byte-identical, and `cargo xtask ci` is green.

- **S4 — Engine Command (`course.set_tracking`), Registry Golden, CLI/MCP & `docs/cli.md`**
  - **Files:** `crates/engine/src/commands/course.rs`, `crates/engine/src/commands/mod.rs`, `crates/engine/src/tests.rs`, `crates/engine/src/m27_all_commands_undo_tests.rs`, `docs/cli.md`, `apps/coursecraft-cli/src/tests.rs`, `apps/coursecraft-mcp/src/tests.rs` (or existing MCP test module).
  - **Exact changes:**
    1. Implement `course_set_tracking` in `crates/engine/src/commands/course.rs` per §3.5 and register `"course.set_tracking"` in `REGISTRY` (`crates/engine/src/commands/mod.rs`) with `UndoScope::CourseHeader`, `mutates: true`, `slide_validation: false`.
    2. Update `r5_01_registry_golden_lock` in `crates/engine/src/tests.rs:12247` (`reg.len() == 190`) and add `course.set_tracking` to `m27_all_commands_undo_tests.rs`.
    3. Add the `course.set_tracking` line to `docs/cli.md` in the same step.
    4. Add CLI (`run` + `validate` + `simulate` + `diff`) and MCP (`command_list` + `command_run`) tests.
  - **Tests:** `TR-13`, `TR-14`; run `cargo test -p coursecraft-engine -j 4`, `cargo test -p coursecraft-cli -j 4`, and `cargo test -p coursecraft-mcp -j 4`.
  - **Done when:** `course.set_tracking` works with 1-step undo/redo, passes `panic_hunt`, registry lock, `docs/cli.md` sync, CLI end-to-end, and MCP execution.

- **S5 — Shared Localized Tracking UI (`tl!`), Course Settings, Course Inspector & Publish Window**
  - **Files:** `crates/ui-egui/src/tracking_ui.rs` (new), `crates/ui-egui/src/lib.rs`, `crates/ui-egui/src/course_settings.rs`, `crates/ui-egui/src/publish.rs`, `crates/ui-egui/src/properties.rs`, `crates/ui-egui/src/tests.rs`, `crates/ui-egui/src/i18n_tests.rs`, `docs/ui/feature-parity.md`, `docs/ui/shell-panels-dialogs.md`.
  - **Exact changes:**
    1. Create `crates/ui-egui/src/tracking_ui.rs` implementing `show_tracking_controls(editor, ui, enabled)` and `tracking_summary(course) -> String` per §3.6 using `tl!` / `craft_i18n::fmt` / `trn` for every string, palette tokens only, zero nested `ScrollArea`s, and dispatching `course.set_tracking`.
    2. Add the **Tracking** section to `show_course_settings_window` (`crates/ui-egui/src/course_settings.rs`).
    3. Add the **Tracking** section to `show_publish_dialog` (`crates/ui-egui/src/publish.rs`) for all LMS standards (with a quiet secondary note for Web, disabled with tooltip while `busy`).
    4. Add the **Tracking** summary row to `show_course_tab` (`crates/ui-egui/src/properties.rs:4951`) for the docked `Panel::CourseInspector`.
    5. Update `docs/ui/feature-parity.md:238` (`course.set_tracking`, Course settings & Publish dialog) and `docs/ui/shell-panels-dialogs.md:136` (Publish dialog Tracking).
  - **Tests:** `TR-15`, `TR-16`; run `cargo test -p coursecraft-ui-egui -j 4 tr_1`, `cargo test -p coursecraft-ui-egui -j 4 i18n`, and **Gate #3:** `cargo xtask ci`.
  - **Done when:** Authors can configure all tracking options from both Course Settings and Publish with single-step undo, Course Inspector shows live summary, every new UI string passes `Lang::PSEUDO`, and `cargo xtask ci` is green.

- **S6 — Preview-vs-Published Parity, M28 UI/UX Sweep, Release Perf, Report & Final Gate**
  - **Files:** `crates/ui-egui/src/u16_preview_tests.rs` (or `tests.rs`), `apps/coursecraft-player/src/tests.rs`, `crates/ui-egui/src/uiux_sweep_tests.rs`, `docs/testing/m44-report.md` (new).
  - **Exact changes:**
    1. Add Preview-vs-Published parity test (`TR-17`) verifying identical completion/score effects between Editor `Preview`, `coursecraft-player`, and `simulate` across slides-viewed, quiz-result, and combined tracking courses.
    2. Exercise non-default Tracking (`slides_viewed` + `quiz`) in `crates/ui-egui/src/uiux_sweep_tests.rs` (`TR-18`) across `course-settings`, `publish`, and `inspector-course` at all sizes/themes/scales (`0 MUST violations`); fix any layout, clipping, contrast, or tooltip findings surfaced.
    3. Run the full automated release gate (§7.1) and write concise `docs/testing/m44-report.md` summarizing shipped behaviour, test counts, perf results, and any follow-ups.
  - **Tests:** `TR-17`, `TR-18` + full release gate (§7.1).
  - **Done when:** All automated gates, M28 UI/UX sweep, WASM web player build, and release perf budgets pass with zero regressions.

---

## 7. Release Gate for M44

### 7.1 Automated Release Gate Commands
Run in order before tagging `0.5.1`:
1. `cargo xtask ci` (`fmt --all --check`, `clippy --workspace --all-targets -- -D warnings`, `test --workspace`, `layers`, `wasm` across all WASM crates)
2. `cargo xtask build-player` (verifies release WASM web player builds cleanly with the new tracking runtime and updated LMS bridges)
3. `cargo test -p coursecraft-ui-egui -j 4 uiux_sweep` (0 MUST violations under M28 `UX-01`–`UX-22`)
4. `cargo test --release -p coursecraft-ui-egui perf_tests -- --ignored --nocapture` (verifies zero editor hot-path regression against `docs/performance.md`)
5. `cargo test --release -p coursecraft-engine undo_perf_tests -- --ignored --nocapture --test-threads=1` (verifies `CourseHeaderPatch` with `Course.tracking` stays within undo memory and timing budgets)

### 7.2 Owner Verification Checklist (Mirrors Issue #9)

- [ ] **Slides viewed (all slides):** Create a 3-slide course with no Finish button, enable **Tracking ▸ Slides viewed ▸ All slides** in Course Settings (or Publish), publish to SCORM 1.2 / 2004, and play in an LMS / mock LMS. Confirm status stays `incomplete` on slides 1–2 and becomes `completed` upon entering slide 3.
- [ ] **Slides viewed (at least N slides):** Set **At least 2 slides** on a 4-slide course (including a branched/unvisited scene). Confirm completion reports upon entering the 2nd distinct slide.
- [ ] **Quiz result:** On a course with graded questions and a Results slide, enable **Tracking ▸ Quiz result** targeting the Results slide. Confirm passing reports `passed` + score and failing reports `failed` + score (respecting SCORM 1.2 **Report status as**), with no duplicate submission.
- [ ] **Triggers keep working:** Confirm a button with `Complete course` or `Submit results` continues reporting to the LMS whether or not Slides viewed / Quiz result are enabled.
- [ ] **Combined (Slides viewed + Quiz result):** Enable both on a 4-slide quiz course. Fail the quiz, then view the remaining slides: confirm SCORM 2004 reports `completed` + `failed` + score, SCORM 1.2 upgrades to `completed` (or `passed` under `passed_*`) without losing the score, and a retry that passes upgrades to `passed`. Confirm a subsequent lower/failing retry **never downgrades** a reported `passed` or `completed` status.
- [ ] **Resume after relaunch:** Close mid-course (2 of 3 slides viewed), relaunch, confirm it resumes without premature completion, then visit slide 3 and confirm completion reports. Relaunch again after completion and confirm status is preserved with no downgrade.
- [ ] **Publish window & Course Settings UX:** Verify Tracking is visible and editable directly in the **Publish** window as well as **Course Settings** (and summarized in the docked **Course** panel), supports `Ctrl+Z` / `⌘Z` undo, and shows clear warnings when `N > slide count` or Quiz result has no quiz/results slide.
- [ ] **Existing courses unchanged:** Open `examples/hello-course/course.json` and `examples/showcase/course.json`, confirm default is triggers-only, JSON stays byte-identical when unmodified, and `cargo xtask ci` passes.

---

## 8. Summary

M44 delivers GitHub issue #9 (LMS Tracking Options) cleanly across **6 focused, single-run steps (`S1`–`S6`)**:
- **Model, validation & delta (`S1`):** Adds backward-compatible `Course.tracking` (`SlidesViewedTracking`, `QuizTracking`) with byte-identical goldens first, play-order slide counting (handling bank draws, masters, templates, and unreachable scenes), validation warnings, `Course::reports_result`, `Course::diff`, `CourseHeaderPatch` undo support, and `docs/spec/course-model.md`.
- **Runtime & spec (`S2`):** Implements all play-time tracking rules exclusively in `coursecraft-logic` reusing `PlayerState.visited` (zero `suspend_data` format change, verified by goldens first) alongside the normative `docs/spec/triggers.md` contract.
- **LMS bridges & manifests (`S3`):** Unifies monotonic non-downgrading status mapping across SCORM 1.2 (all 5 `lms_status` modes), SCORM 2004 4th, cmi5, xAPI, and LTI 1.3, plus `cmi5.xml` `moveOn` (`Completed` / `Passed` / `CompletedOrPassed`) and SCORM manifest `mastery_score` alignment.
- **Engine command, CLI & MCP (`S4`):** Adds undoable `course.set_tracking`, updates `r5_01_registry_golden_lock`, `m27_all_commands_undo_tests`, and `docs/cli.md`, and verifies CLI & MCP workflows.
- **Editor UI & i18n (`S5`):** Builds one shared `tl!`-localized `tracking_ui.rs` mounted in both **Course Settings** and the **Publish** window, plus a live summary row in the docked **Course** inspector panel, and updates `docs/ui/feature-parity.md` and `docs/ui/shell-panels-dialogs.md`.
- **Verification (`S6`):** Closes with Preview-vs-Published parity (`TR-17`), M28 UI/UX sweep (`TR-18`), `cargo xtask ci`, `cargo xtask build-player`, release perf budgets, and `docs/testing/m44-report.md`.

**Total steps:** 6 (`S1`–`S6`).
