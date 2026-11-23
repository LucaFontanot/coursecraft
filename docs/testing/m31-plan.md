# M31 Urgent Bug-Fix Release (`0.4.2`) — Audit & Implementation Plan

**Role:** Planner (planning only — zero source files edited)  
**Target release:** `0.4.2` / M31  
**Scope:** 6 **Must fix** items (data loss, crashes, wrong LMS results, navigation loop) + 2 **Should fix** items (web editor data loss & UI freeze), broken into **9 small implementer steps (`S1`–`S9`)**. All 8 reported items were confirmed in the codebase; **0 dropped**.

> **Implementer Guardrails (`AGENTS.md`, `docs/contributing.md`)**
> 1. **Never crash:** Zero `unwrap`/`expect`/`panic!`/`unreachable!`/`todo!` in non-test code; `unsafe_code = "forbid"` everywhere except `coursecraft-video-native`. Input-derived numbers are hostile.
> 2. **One runtime, spec first:** Play-time and LMS bridge changes (`S6`, `S7`) update `docs/spec/triggers.md` in the same step.
> 3. **Layering & WASM:** `cargo xtask layers` and `cargo xtask wasm` (L0–L5) stay green; run `cargo xtask build-player` after `coursecraft-logic` / `coursecraft-lms` / `coursecraft-render` changes (`S4`, `S6`, `S7`, `S9`).
> 4. **Test first (red → green):** Write each step's failing test first, confirm it fails on the bug, then apply the minimal fix.

---

## 1. Confirmed Findings (Items 1–8)

### Item 1 — `Ctrl+S` does nothing (nor `Ctrl+O`, `Ctrl+N`, `Ctrl+Shift+S`) `[Must fix → S1]`
- **Cause:**
  - `crates/ui-egui/src/lib.rs:4333-4335` comments that *"Save, Save As, Open, Preview, and Cmd+B/I/U are handled outside the registry"*, yet **none** of `Ctrl+S`, `Ctrl+Shift+S`, `Ctrl+O`, or `Ctrl+N` is bound anywhere in `crates/ui-egui/src/lib.rs` or `apps/coursecraft/src/main.rs`.
  - `crates/ui-egui/src/menu.rs:572, 582, 625, 635`: File menu buttons (`"New"`, `"Open…"`, `"Save"`, `"Save As…"`) are plain `ui.button(...)` with no `.shortcut_text(...)`.
  - `crates/ui-egui/src/shortcuts.rs:245-354`: `build_shortcut_groups()` has no `File` shortcut rows for New, Open, Save, or Save As (`Publish course` sits alone in `General`).
- **Evidence:** Confirmed by workspace-wide search (`rg "Key::S|Key::O|Key::N|EditorRequest::Save" crates/ui-egui/src/ apps/coursecraft/src/`): zero keyboard handlers emit `EditorRequest::Save`, `SaveAs`, `Open`, or `New`.
  - *Failing-test sketch (`M31-01`):* In `TestHarness`, press `Modifiers::COMMAND + Key::S` (and `SHIFT+COMMAND+S`, `COMMAND+O`, `COMMAND+N`) → assert returned `EditorRequest` contains `Save` (`SaveAs`, `Open`, `New`); fails today with empty requests.
- **Fix:**
  1. In `crates/ui-egui/src/lib.rs` (`Editor::show`), hoist `let mut requests = Vec::new();` above the shortcut block (before `:4261`) and bind file shortcuts using `menu::parse_shortcut`:
     - Guard: `!preview_has_focus && !ui.ctx().egui_wants_keyboard_input() && self.ui.inline_edit.is_none() && !self.is_cropping() && !self.ui.ai.open && !self.ui.document.open && !self.ui.alt.open && !self.ui.translate.open && !self.ui.show_shortcuts_dialog && !matches!(self.ui.publish.phase, PublishPhase::Busy)` (note: uses `!egui_wants_keyboard_input()` + `inline_edit.is_none()`, NOT `memory().focused().is_some()`, matching `Ctrl+Shift+P` at `:4406` so clicking a toolbar/inspector button does not block `Ctrl+S`, while typing in any `TextEdit` or inline canvas text editor never fires file shortcuts).
     - **Consume `Ctrl+Shift+S` BEFORE `Ctrl+S`** (egui matches `COMMAND` logically with extra `Shift` ignored, like `Ctrl+Shift+Z` at `:4315-4330`):
       - `Ctrl+Shift+S` (when `self.file_ops_available && !self.ui.show_welcome`) → `requests.push(EditorRequest::SaveAs)`
       - `Ctrl+S` (when `self.file_ops_available && !self.ui.show_welcome`) → `requests.push(EditorRequest::Save)`
       - `Ctrl+O` (when `self.file_ops_available`) → `requests.push(EditorRequest::Open)`
       - `Ctrl+N` → `requests.push(EditorRequest::New)`
     - On desktop (`apps/coursecraft/src/main.rs:1107-1126, 1202`), ignore `New`/`Open`/`Save`/`SaveAs` requests while a blocking shell modal (`pending_action`, `conflict_prompt`, `restore_prompt`, `error_modal`) is already open, so pressing `Ctrl+N` twice never clobbers an open prompt. Existing desktop save/open/new flows (`guard_unsaved`, `perform_save` → `perform_save_as` for untitled, `conflict_prompt`) already run automatically once `EditorRequest` is emitted; on web (`file_ops_available == true`), `New`/`Open`/`Save`/`SaveAs` route straight into the web handlers (with unsaved-changes guard added in `S8`).
  2. In `crates/ui-egui/src/menu.rs:572-642`, add `.shortcut_text(shortcut_label(ui.ctx(), ...))` to `"New"` (`"Ctrl+N"`), `"Open…"` (`"Ctrl+O"`), `"Save"` (`"Ctrl+S"`), and `"Save As…"` (`"Ctrl+Shift+S"`).
  3. In `crates/ui-egui/src/menu.rs:31-159` (`format_shortcut`), split compound `" / "` strings (e.g. `"Ctrl+S / Ctrl+Shift+S"`) so every segment renders native macOS `⌘`/`⇧` glyphs instead of falling back to raw `"Ctrl+..."`.
  4. In `crates/ui-egui/src/shortcuts.rs:245-354`, add a `"File"` group at the top with compact rows (`"New / Open"` → `"Ctrl+N / Ctrl+O"`, `"Save / Save As"` → `"Ctrl+S / Ctrl+Shift+S"`, `"Publish course"` → `"Ctrl+Shift+P"` moved out of `General`) so total dialog height stays under the `720 pt` cap at `1440×900` (`RISK-01`).
- **Tests:** `M31-01` (all 4 shortcuts emit exact `EditorRequest`s, Shift+S beats S), `M31-02` (suppressed while typing in a `TextEdit`, inline text editor, preview focus, or welcome screen for Save/SaveAs), `M31-03` (desktop `EditorApp` routes `Ctrl+S` on untitled → Save As, `Ctrl+N`/`Ctrl+O` on dirty → unsaved prompt, blocked while modal open), `M31-04` (File menu shortcut hints + `format_shortcut` macOS/non-macOS compound strings + `shortcut_dialog_shows_every_row_without_scrolling_at_1440x900`).

---

### Item 2 — MCP drops media on `--course x.ccraft` startup `[Must fix → S2]`
- **Cause:**
  - `apps/coursecraft-mcp/src/lib.rs:57-72, 248-258`: `load_course(path)` calls `coursecraft_format::open_project(&bytes)` and returns only `Ok(proj.course)`, silently discarding `proj.media` (and fails if `path` is a `.ccraft-folder` directory). `HeadlessHost::new(session, Some(course_path.clone()))` starts with an empty `HeadlessMediaSource.in_memory` map (`crates/control/src/host.rs:146`). The next `file_save` (`host.rs:407`) calls `self.media_source.snapshot_in_memory()`, writing a `.ccraft` archive with **0 media entries**.
  - **Audit of every other MCP open/save path:**
    1. Headless `file_open` (`crates/control/src/host.rs:471-506`): `.ccraft` (`:483-485`) and project folders (`:487-491`) already call `self.media_source.set_in_memory(...)`. **However**, opening a bare `course.json` (`:492-496`) sets `in_memory` empty; if the user then calls `file_save { path: "out.ccraft" }` (or a folder), sibling `<course_dir>/media/<src>` files are dropped because `save()` only reads `snapshot_in_memory()`! Also `HeadlessHost::save` (`:466`) updates `self.path` without calling `self.media_source.set_path(Some(target.clone()))`.
    2. `--attach` (`apps/coursecraft/src/main.rs:2620-2733`, `EditorHostAdapter`): `open` calls `files::open_from` / `files::open_folder_from` + `self.editor.set_media(...)`, and `save` calls `self.editor.used_media()` + `files::save_to_arc` / `files::save_folder_to_arc`. Media referenced by the course is preserved.
- **Evidence:** Confirmed by code audit + existing `coursecraft-mcp` tests (`cargo test -p coursecraft-mcp -j 4`: 8 passed, none test opening a `.ccraft` with media via `load_course`/startup).
  - *Failing-test sketch (`M31-05`):* Save a `.ccraft` with `media/pic.png` via `coursecraft_format::save_project`; construct the MCP headless host using the startup helper (`open_headless_host(path)`); call `file_save` (with no path, overwriting in place); reopen with `coursecraft_format::open_project` → assert `reopened.media` still contains `("pic.png", png_bytes)` (fails today with `reopened.media == []`).
- **Fix:**
  1. In `crates/control/src/host.rs`:
     - Add `HeadlessHost::open_path(path: &Path) -> Result<Self, HostError>` that creates a blank host and delegates directly to `ControlHost::open(&mut host, path, true)` — one single open path for `.ccraft`, `.ccraft-folder`, and `course.json`.
     - In `HeadlessHost::save` (`:406-455`): add helper `collect_media_for_save(&self)` that starts with `self.media_source.snapshot_in_memory()` and, for any `r.src` in `self.session.course.used_media()` not already in memory, resolves it via `self.media_source.get(&r.src)` (so opening `course.json` beside `media/` and saving to `.ccraft` or folder preserves sibling media too). Also call `self.media_source.set_path(Some(target.clone()))` at `:466`.
  2. In `apps/coursecraft-mcp/src/lib.rs:57-72, 248-258`: remove the broken `load_course` function and call `HeadlessHost::open_path(course_path)` directly (extracting a small `pub(crate) fn open_headless_host(path: &Path) -> Result<HeadlessHost, String>` so tests exercise the exact startup path).
- **Tests:** `M31-05` (MCP startup open `.ccraft` with media → `command_run` edit → `file_save` → reopen confirms media bytes intact), `M31-06` (MCP startup open `.ccraft-folder` with media → `file_save` → media intact; and `course.json` with sibling `media/` → `file_save` to `.ccraft` preserves media), `M31-07` (`--attach` / `EditorHostAdapter` round-trip open `.ccraft` with media → edit → `file.save` → media intact).

---

### Item 3 — Editor panics on tiny slides (`< 20 px`), huge scroll panels (`> 20,000 px`), & workspace `.clamp(` audit `[Must fix → S3, S4]`
- **Cause (reported crashes — all 3 confirmed):**
  1. `crates/ui-egui/src/input_insert.rs:22-25`: `centred_frame` runs `width.clamp(20.0, size.w as f32)` and `height.clamp(20.0, size.h as f32)`. Because `validate.rs:101-106` allows `slide_size` down to `1×1 px`, any slide with `w < 20` or `h < 20` panics (`min > max`) on `insert_text_entry`, `insert_slider`, `insert_dial`, `insert_checkbox`, and `insert_radio_group`.
  2. `crates/ui-egui/src/interaction_insert.rs:8-11`: identical `width.clamp(20.0, size.w as f32)` / `height.clamp(20.0, size.h as f32)` panics on `insert_marker`, `insert_timer`, `insert_progress_meter`, and `insert_scroll_panel`.
  3. `crates/ui-egui/src/properties.rs:7188-7192`: `"Fit to content"` runs `fit_height.clamp(frame_height.max(0.0), coursecraft_course::MAX_SCROLL_CONTENT_HEIGHT)`. Since `MAX_OBJECT_FRAME_SIZE` is `100_000.0` while `MAX_SCROLL_CONTENT_HEIGHT` is `20_000.0`, any scroll panel taller than `20_000 px` panics (`min > max`). Note that `crates/engine/src/commands.rs:6566` (`object_set_scroll_panel`) already uses `coursecraft_course::safe_clamp`, and `properties.rs:7158` passes `.range(frame_height..=MAX_SCROLL_CONTENT_HEIGHT)` (inverted when `frame_height > 20_000.0`).
- **Workspace-wide `.clamp(` audit (323 non-test calls inspected; 83 with non-constant bounds):**
  `coursecraft_course::safe_clamp(value: f32, lo: f32, hi: f32) -> f32` **already exists** at `crates/course/src/model.rs:1902` (handles `lo > hi` and `NaN` without panicking). Every risky or hostile-bound call found across the workspace:

  | Risk | File & Line(s) | Current `.clamp(...)` | How bounds can cross / turn `NaN` |
  |---|---|---|---|
  | **Crash (Confirmed)** | `crates/ui-egui/src/input_insert.rs:24-25` | `clamp(20.0, size.w/h as f32)` | Valid `slide_size` `1..19 px` → `min > max`. |
  | **Crash (Confirmed)** | `crates/ui-egui/src/interaction_insert.rs:10-11` | `clamp(20.0, size.w/h as f32)` | Valid `slide_size` `1..19 px` → `min > max`. |
  | **Crash (Confirmed)** | `crates/ui-egui/src/properties.rs:7189` (+ `:7158`) | `clamp(frame_height.max(0.0), MAX_SCROLL_CONTENT_HEIGHT)` | Scroll panel `frame.h > 20_000` → `min > max`. |
  | **High (Hostile/Edge)** | `crates/render/src/shape_fill.rs:1056` | `(half_dash - phase).clamp(0.0, step)` | Floating-point `t > len` makes `step = remaining.min(len - t) < 0.0` → panics. |
  | **High (Hostile/Edge)** | `crates/render/src/player_frame.rs:2486, 2501, 3046` | `clamp(rect.left(), rect.right())`; `clamp(min, max)`; `clamp(clip.top(), clip.bottom())` | Inverted `Rect::NOTHING` (`top = +∞, bottom = -∞`) or tiny viewport → panics. |
  | **High (Hostile/Edge)** | `crates/render/src/focus.rs:2414, 2419` | `x.clamp(rect.left(), rect.right())`, `y.clamp(rect.top(), rect.bottom())` | Inverted / `Rect::NOTHING` rect → panics. |
  | **High (Hostile/Edge)** | `crates/render/src/shape_geometry.rs:973, 1134, 2870-2873, 2881-2882` | `clamp(0.0, w * 0.95)`, `clamp(0.0, w)`, `clamp(0.0, w - x)`, etc. | Negative or `NaN` `w`/`h` → panics. |
  | **Medium (`NaN` bounds)** | `crates/course/src/model.rs:1375` (`finite_clamp`), `crates/course/src/shapes.rs:1761`, `crates/render/src/access.rs:763` | `clamp(min, max)` / `clamp(min.min(max), min.max(max))` | `NaN` in `min`/`max` panics in `f32::clamp` (`!min.is_nan() && !max.is_nan()`). |
  | **Medium (`NaN` / tiny UI)** | `crates/ui-egui/src/preview.rs:841-842, 1065, 1419-1420, 1433-1434`; `crates/ui-egui/src/translate_ai.rs:641`; `crates/ui-egui/src/coach.rs:133` | `clamp(min_size, max_size)`, `clamp(40.0, table_height)`, `clamp(min_x, max_x)` | Non-finite viewport/bounds or direct helper call with `min > max`. |
  | **Low (Defense-in-depth)** | `crates/render/src/compose.rs:3106, 3108`; `crates/render/src/paint.rs:3715, 3728, 3740`; `crates/render/src/view.rs:697, 780-781`; `crates/render/src/player_frame.rs:1300, 2142, 2144` | `clamp(0.0, max_x)`, `clamp(0.0, count - 1.0)`, etc. | Guarded today for finite inputs, fragile if `NaN` propagates. |
- **Evidence:** Direct code inspection confirms all 3 reported panics immediately; `validate.rs:101-106` allows `1..=8192` px slides while `input_insert.rs:24` and `interaction_insert.rs:10` hardcode `min = 20.0`.
  - *Failing-test sketch (`M31-08`):* Create a valid `10×10` course; call every `input_insert::insert_*` and `interaction_insert::insert_*` and `toolbar::insert_*` → panics today at `input_insert.rs:24` / `interaction_insert.rs:10`. Create a course with a `ScrollPanel` of `frame.h = 25_000.0` and click `"Fit to content"` in Format inspector → panics today at `properties.rs:7189`.
- **Fix:**
  - **S3 (Reported crashes + insert tests):**
    1. In `input_insert.rs:22-32` and `interaction_insert.rs:8-18`: use `coursecraft_course::safe_clamp(width, 1.0, (size.w as f32).max(1.0))` (or `safe_clamp(width, 20.0_f32.min(size.w as f32).max(1.0), (size.w as f32).max(1.0))` — note: on normal slides `>= 20 px`, `20.0..=size.w` is unchanged; on `1..19 px` slides, the inserted frame fits the slide `w`/`h` cleanly and passes `validate()`!).
    2. In `properties.rs:7158, 7189`: compute `let lo = frame_height.max(0.0).min(coursecraft_course::MAX_SCROLL_CONTENT_HEIGHT);` for `DragValue::range(lo..=coursecraft_course::MAX_SCROLL_CONTENT_HEIGHT)` and use `coursecraft_course::safe_clamp(fit_height, frame_height.max(0.0), coursecraft_course::MAX_SCROLL_CONTENT_HEIGHT)` on `"Fit to content"`.
    3. Add tests covering tiny (`1×1`, `10×10`) and huge (`8192×8192`) slides for every UI insert helper AND every engine insert command (`object.add`, `question.insert`, `interaction.insert`, `results.insert`, `certificate.insert`), plus `"Fit to content"` on a `25_000 px` scroll panel.
  - **S4 (Workspace-wide `safe_clamp` sweep):** Replace every risky `.clamp(` in the audit table above (`course`, `render`, `ui-egui`) with `coursecraft_course::safe_clamp` (fixing `finite_clamp` in `model.rs:1375` to delegate to `safe_clamp` too), and add targeted hostile-input unit tests (`Rect::NOTHING`, negative `w`/`h`, `NaN` bounds, tiny viewport).
- **Tests:** `M31-08` (UI inserts on `1×1`, `10×10`, `8192×8192` slides), `M31-09` (ScrollPanel `"Fit to content"` + inspector render at `frame.h = 25_000.0`), `M31-10` (engine `panic_hunt` extended with `1×1`, `10×10`, `8192×8192` slides for all insert commands), `M31-11` (hostile-bounds unit tests for `safe_clamp` call sites in `course`, `render`, and `ui-egui`).

---

### Item 4 — Publish fails on an empty state image (`src: ""`) `[Must fix → S5]`
- **Cause:**
  - `crates/publish/src/lib.rs:167-173` (`collect_object_media`):
    ```rust
    for state in &obj.states {
        if let Some(src) = &state.src {
            out.push(MediaRef { object_id: &obj.id, src });
        }
    ```
    Unlike `state.fill_style` (`:178`), `state.extras` (`:198`), `Video.poster` (`:132`), `crates/course/src/lib.rs:274` (`used_media`), and the player (`crates/render/src/compose.rs:1320`: `state.src.as_deref().filter(|s| !s.is_empty())`), line 168 does **not** skip `src.is_empty()`. When `state.src == Some("")`, `build_package_standard_with_folders` hits `:512` (`!is_safe_relative_path("")`) and fails with `PublishError::UnsafePath { object, src: "" }`.
  - Meanwhile `crates/course/src/validate.rs:2194-2195` explicitly skips `src.is_empty()`, so the course validates with 0 errors and 0 warnings, plays cleanly, and fails only at publish time!
  - Audited all other `MediaRef` pushes in `collect_object_media` / `collect_resource_media`: `Image.src`, `Lottie.src`, `Audio.src/captions`, `Video.src/captions`, `background_audio.src`, `fonts[].src`, `player.logo`, and `resources[].target` are all rejected as `Severity::Error` by `validate.rs` when empty — `state.src` is the sole mismatch!
- **Evidence:** Confirmed by direct comparison of `validate.rs:2194`, `compose.rs:1320`, `course/src/lib.rs:274`, and `publish/src/lib.rs:168`.
  - *Failing-test sketch (`M31-12`):* Build a valid course with an `Image` object whose `"hover"` state has `src: Some(String::new())`; assert `course.is_playable() == true`, then call `build_package(&input)` → fails today with `Err(PublishError::UnsafePath { object, src: "" })`.
- **Fix:**
  1. In `crates/publish/src/lib.rs:168`, guard with `if let Some(src) = &state.src && !src.is_empty()` (matching the player and `used_media`).
  2. In `crates/course/src/validate.rs:2194-2220`, when `let Some(src) = &s.src` and `src.trim().is_empty()`, emit `Severity::Warning` at `format!("{p}/states/{i}/src")` (e.g. `State "hover" on "Photo" has an empty image path; the base image will be used.`) so Check Course flags it while keeping the course valid and publishable.
- **Tests:** `M31-12` (`crates/publish`: course with `state.src: Some("")` builds & zips cleanly with zero errors and still ships the base image), `M31-13` (`crates/course`: `validate()` emits `Severity::Warning` — not `Error` — for `state.src: Some("")`, so `is_playable()` remains `true`).

---

### Item 5 — The LMS can lose a pass (`lms_status` downgrade) `[Must fix → S6]`
- **Cause:**
  - In `crates/lms/src/bridge.rs:720-794`:
    - `handle_score` (`:738`) and `handle_completion` (`:778`) set `self.passed = true` **only** when `status == CompletionStatus::Passed`.
    - Under SCORM 1.2 `LmsStatus::PassedIncomplete`, `map_status(CompletionStatus::Completed, LmsStatus::PassedIncomplete)` (`:854-855`) writes **`"passed"`** to `cmi.core.lesson_status` when the course completes — yet `self.passed` stays **`false`**!
    - When a quiz is later failed (`Effect::Score { status: Failed }` / `Effect::Completion { status: Failed }`), `if self.passed && ...` is false, so `write_status` writes `map_status(Failed, PassedIncomplete)` = **`"incomplete"`**, **overwriting `"passed"`**!
  - **Audit of all other `LmsStatus` modes & standards:**
    1. **SCORM 1.2 `PassedFailed`:** **Exact same bug!** `Completed` writes `"passed"` while `self.passed` stays `false`; a later `Failed` overwrites `"passed"` with `"failed"`.
    2. **SCORM 1.2 `CompletedIncomplete` & `CompletedFailed`:** **Same flaw in-session AND on resume!** Both `Passed` and `Completed` map to `"completed"`. In-session, `Completed` followed by `Failed` overwrites `"completed"` with `"incomplete"`/`"failed"`. Across resume (`start()` at `:540`), `is_passed(&statuses)` checks only `status == "passed"`, so a learner who **passed** in session 1 (stored as `"completed"` in the LMS) resumes with `self.passed = false`, and a later quiz fail overwrites `"completed"` with `"incomplete"`/`"failed"` (and lowers `cmi.core.score.raw`)!
    3. **SCORM 1.2 `Auto`:** `Passed` writes `"passed"` and sets `self.passed = true`; later `Failed`/`Incomplete`/`Completed` are already blocked. (`Completed` → `Passed` remains an allowed upgrade; `Completed` → `Failed` when never passed is intentional in `Auto`.)
    4. **SCORM 2004 (`bridge.rs:193, 217-232, 729, 767`):** Reports `completion_status` and `success_status` separately; `is_passed` reads `cmi.success_status == "passed"`, and `self.passed` blocks any later `Failed`/`Incomplete` or lower score. **No pass-loss bug.**
    5. **cmi5 / xAPI (`crates/lms/src/lrs.rs:593-595, 788, 806`):** `passed_saved` is set on pass and restored from `coursecraft.progress`; `enqueue_failed` returns early if `self.passed_saved` is true. **No pass-loss bug.**
    6. **LTI (`crates/lms/src/lti.rs:103-123`):** `has_passed` checks `passed`, `in_flight`, and `pending`; rejects non-pass and lower/equal scores. **No pass-loss bug.**
- **Evidence:** Confirmed by code inspection of `bridge.rs:540-548, 720-794, 846-873` + running `cargo test -p coursecraft-lms -j 4 lms_status` (existing `u21_lms_status_tests.rs` only tests single-effect mapping, never a sequence `Completed` → `Failed` or resume after `Completed` in `CompletedIncomplete`).
  - *Failing-test sketch (`M31-14`):* SCORM 1.2 + `StrictScorm12` with `LmsStatus::PassedIncomplete`: send `Effect::Completion { status: Completed }` (`lesson_status == "passed"`), then `Effect::Score { percent: 40, status: Failed }` + `Effect::Completion { status: Failed }` → assert `lesson_status == "passed"` (fails today: becomes `"incomplete"`).
- **Fix:**
  1. Define the clean rule for SCORM 1.2 in `crates/lms/src/bridge.rs`:
     - Add helper `is_protected_status(written: LessonStatus, lms_status: LmsStatus) -> bool`:
       - `Auto | PassedIncomplete | PassedFailed` → `written == LessonStatus::Passed`
       - `CompletedIncomplete | CompletedFailed` → `matches!(written, LessonStatus::Passed | LessonStatus::Completed)`
     - In `start()` (`:540`), detect prior protected pass/completion using `is_passed_for(&statuses, self.lms_status)` (for SCORM 1.2, checks `Passed` in `Auto`/`Passed*` modes, and `Passed | Completed` in `Completed*` modes; unchanged for SCORM 2004).
     - In `handle_score` (`:720-758`) and `handle_completion` (`:760-794`):
       - Once `self.passed` is true, any effect whose mapped status is NOT a protected status (e.g. `Failed`/`Incomplete`, or `Completed` under `Auto`), or a `Score` with `percent < best_passed_score`, is ignored. (Note: if `self.passed` was set by a scoreless `Completion { Completed }` under `PassedIncomplete`, `best_passed_score` is `None`, so a later passing `Score { Passed, 90 }` still records the score `90`, while a failing `Score { Failed, 40 }` is completely ignored!)
       - After writing status, set `self.passed = true` whenever `status == CompletionStatus::Passed` OR `(self.mapping.standard == Standard::Scorm12 && is_protected_status(map_status(status, self.lms_status), self.lms_status))`.
  2. Update `docs/spec/triggers.md:778` to state explicitly that in SCORM 1.2, once `passed` has been reported (including via `Completed` in `passed_incomplete`/`passed_failed`), or `completed` in `completed_incomplete`/`completed_failed`, a later fail, incomplete, or lower score never downgrades it (within the session or after resume).
- **Tests:** `M31-14` (`StrictScorm12`: `Completed` → `Score(40, Failed)` + `Completion(Failed)` keeps `"passed"` in `PassedIncomplete` and `PassedFailed`, and keeps `"completed"` in `CompletedIncomplete` and `CompletedFailed`, with 0 strict violations), `M31-15` (SCORM 1.2 resume tests: resuming `"completed"` under `CompletedIncomplete`/`CompletedFailed` with score `90` rejects a later `Failed`/`40` and lower `Passed`/`80`, accepts `Passed`/`95`), `M31-16` (verify `Completed` → `Score(90, Passed)` under `PassedIncomplete` still records score `90`, and regression checks on SCORM 2004 & cmi5).

---

### Item 6 — `"Last visited slide"` can loop `[Must fix → S7]`
- **Cause:**
  - `crates/logic/src/run.rs:807-824` (`Player::jump` for `SlideTarget::LastVisited`):
    ```rust
    if matches!(target, SlideTarget::LastVisited) {
        let prev_idx = self
            .st
            .visit_history
            .iter()
            .rev()
            .find_map(|id| self.slide_position(id))
            .or_else(|| cur.checked_sub(1));
        if let Some(idx) = prev_idx {
            self.st.visit_history.pop();
            ...
            self.enter_slide_inner(idx, false, q, out);
        }
    ```
  - **Two bugs in those 9 lines:**
    1. `iter().rev().find_map(...)` skips trailing entries not present in `self.slides` (for example, bank slides replaced when `retry_quiz` redraws a question bank at `run.rs:3983-3993`, or slides outside a `preview_subset`) and resolves an older slide at index `k < len - 1`, **but line 816 only pops the last element (`len - 1`)** — leaving the resolved slide at `k` still on `visit_history`!
    2. It never checks `idx != cur`, so on the next `LastVisited` it resolves to the slide the learner is already on (or keeps bouncing/re-entering the same slide for every skipped stale entry).
  - **Exact looping sequence:**
    - Course with `s-intro` → bank draw (`bank-q1`, `bank-q2`) → `s-results` (`retry_quiz` redraws bank to `bank-q3`, `bank-q4`).
    - Visit `s-intro` → `bank-q1` → `bank-q2` → `s-results` (`visit_history = ["s-intro", "bank-q1", "bank-q2"]`).
    - Trigger `retry_quiz` → redraws bank (`bank-q1`/`bank-q2` leave `self.slides`), pushes `"s-results"`, enters `bank-q3` (`visit_history = ["s-intro", "bank-q1", "bank-q2", "s-results"]`).
    - `LastVisited` #1 on `bank-q3` → pops `"s-results"`, enters `s-results` (`visit_history = ["s-intro", "bank-q1", "bank-q2"]`).
    - `LastVisited` #2 on `s-results` → `find_map` skips `bank-q2` and `bank-q1`, resolves `"s-intro"`, **pops `"bank-q2"`**, enters `s-intro` (`visit_history = ["s-intro", "bank-q1"]`!).
    - `LastVisited` #3 on `s-intro` → `find_map` skips `bank-q1`, resolves `"s-intro"`, pops `"bank-q1"`, **re-enters `s-intro`**!
    - `LastVisited` #4 on `s-intro` → resolves `"s-intro"`, pops `"s-intro"`, **re-enters `s-intro` again**!
    - (And if the learner jumps `s-intro` → `s-results` between #2 and #3, `s-results` → `LastVisited` → `s-intro` keeps leaving stale entries underneath.)
- **Evidence:** Confirmed in `crates/logic/src/run.rs:807-824` + `cargo test -p coursecraft-logic -j 4 jump_to_scene_and_last_visited` (existing test only tests a single clean pop with no stale entries or self-matches).
  - *Failing-test sketch (`M31-17`):* Run the exact bank-redraw sequence above (and a `coursecraft-cli simulate` fixture); assert `LastVisited` #2 enters `s-intro` and drains `visit_history` to `[]`, and `LastVisited` #3 on `s-intro` (index 0) does **not** re-enter `s-intro` (fails today: re-enters `s-intro` twice).
- **Fix:**
  1. In `crates/logic/src/run.rs:807-824`, pop entries until a valid slide different from `cur` is found (discarding any missing or current-slide entries as we pop):
     ```rust
     if matches!(target, SlideTarget::LastVisited) {
         let mut prev_idx = None;
         while let Some(id) = self.st.visit_history.pop() {
             if let Some(idx) = self.slide_position(&id)
                 && idx != cur
             {
                 prev_idx = Some(idx);
                 break;
             }
         }
         let prev_idx = prev_idx.or_else(|| cur.checked_sub(1));
         if let Some(idx) = prev_idx {
             if self.st.lightbox.is_some() {
                 self.close_lightbox(false, out);
             }
             self.enter_slide_inner(idx, false, q, out);
         } else if self.preview_subset.is_some() {
             self.preview_boundary_hit = true;
         }
         return;
     }
     ```
  2. Update `docs/spec/triggers.md:380` to document that `jump_to_slide last_visited` pops entries from `visit_history` until it finds a slide currently in play order different from the current slide (discarding missing or same-slide entries along the way), falling back to `previous` if none remain.
- **Tests:** `M31-17` (`crates/logic`: bank-redraw `retry_quiz` + `LastVisited` drains stale entries and never loops onto the current slide; also tests consecutive duplicates / preview subset), `M31-18` (`apps/coursecraft-cli`: `simulate` test driving a course with `LastVisited` after bank redraw / multi-hop jumps).

---

### Item 7 — Web editor: New or Open replaces a changed course with no warning `[Should fix → S8]`
- **Cause:**
  - `apps/coursecraft/src/main.rs:217-236, 3241-3290, 3337-3384, 3601-3635`: `Project`, `PendingAction`, and the `"Save changes to {name}?"` modal are all gated behind `#[cfg(not(target_arch = "wasm32"))]`.
  - On `wasm32`, `EditorRequest::New` (`:3281`), `EditorRequest::Open` (`:3337` → `open_web_project_bytes` at `:3601`), `OpenExample` (`:3285`), `OpenTemplate` (`:3288`), and `PickPptx` (`:3241`) replace the current course immediately with zero dirty tracking and zero confirmation modal.
- **Evidence:** Confirmed directly in `apps/coursecraft/src/main.rs:3281-3290, 3337-3355, 3601-3634`.
  - *Failing-test sketch (`M31-19`):* Mutate a web-session state (revision advances past `saved_revision`), request `New` or `Open` → assert the guard transitions to `Pending(action)` instead of executing immediately; clicking `Cancel` clears pending without mutating; clicking `Don't Save` executes the action; clicking `Save` downloads `.ccraft`, updates `saved_revision`, and executes the action.
- **Fix:**
  1. Add a small platform-independent `web_guard` module (or helper struct `WebUnsavedState` compiled on both `wasm32` and `#[cfg(test)]`) in `apps/coursecraft/src/main.rs` (or `apps/coursecraft/src/web_state.rs`) holding:
     - `saved_revision: u64`, `file_name: Option<String>`, `pending: Option<WebPendingAction>` (`New`, `Open`, `OpenExample(ExampleId)`, `OpenTemplate(TemplateId)`, `ImportPptx`).
     - Pure methods `is_dirty(rev) -> bool`, `mark_saved(rev)`, `mark_dirty_after_import()`, `request_or_run(action, rev) -> Option<WebPendingAction>` (returns `Some(action)` to run immediately when clean, or stores `pending` and returns `None` when dirty).
  2. Wire `WebUnsavedState` into wasm `EditorApp`:
     - Update `self.editor.set_title_info(name, is_dirty)` every frame on web so the dirty indicator works there too.
     - Route `EditorRequest::New`, `Open`, `OpenExample`, `OpenTemplate`, `PickPptx` through `web_unsaved.request_or_run(...)`.
     - Update `saved_revision` on `Save`/`SaveAs` download success, `New`, `OpenExample`, `OpenTemplate`, and `open_web_project_bytes` success (and set `u64::MAX` on `finish_import_pptx`, matching desktop `:2432`).
     - Render the exact same `"Save changes to {name}?"` `egui::Modal` (`Save`, `Don't Save`, `Cancel` + `Escape`) on `wasm32`. Note: executing `Open` / `PickPptx` directly inside the `"Don't Save"` / `"Save"` button click handler preserves the browser's user-gesture requirement for `rfd::AsyncFileDialog::pick_file()` (`RISK-04`).
- **Tests:** `M31-19` (native unit tests in `apps/coursecraft` covering `WebUnsavedState`: clean vs dirty `New`/`Open`/`OpenExample`/`OpenTemplate`/`ImportPptx`, `Save` marks clean & continues, `Don't Save` discards & continues, `Cancel`/Esc aborts, PPTX import marks dirty) + `cargo xtask wasm` compile check.

---

### Item 8 — Web editor: Publish freezes the tab `[Should fix → S9]`
- **Cause:**
  - `apps/coursecraft/src/main.rs:3385-3412`:
    ```rust
    EditorRequest::Publish { standard } => {
        self.editor.set_publish_busy();
        let pkg_standard = publish::package_standard(standard);
        let player = player_bundle::player_files();
        match publish::run_publish_bytes(...) { ... }
    ```
    Calling `set_publish_busy()` and `run_publish_bytes()` synchronously in the same `eframe::App::ui` frame means the `"Publishing…"` spinner is **never painted**, and assembling the package + deflating the multi-MB `coursecraft_player_bg.wasm` + media runs in a single blocking call on the browser's single JS UI thread, freezing the tab.
- **Evidence:** Confirmed directly in `apps/coursecraft/src/main.rs:3385-3412` and `apps/coursecraft/src/publish.rs:108-116`.
  - *Failing-test sketch (`M31-20`):* Drive `PackageZipper::new(files)` in `256 KiB` chunks until `ZipStep::Done(bytes)` → assert `steps > 1` on a multi-MB payload, progress advances monotonically (`0..=total`), and resulting bytes are **byte-for-byte identical** to `zip_package(&files)`.
- **Fix (minimal, safe, zero new dependencies, desktop untouched):**
  1. In `crates/publish/src/lib.rs`, add `PackageZipper` + `ZipStep`:
     ```rust
     pub enum ZipStep {
         InProgress { bytes_done: u64, bytes_total: u64 },
         Done(Vec<u8>),
     }
     pub struct PackageZipper {
         files: Vec<PackageFile>,
         writer: Option<zip::ZipWriter<std::io::Cursor<Vec<u8>>>>,
         file_index: usize,
         file_offset: usize,
         bytes_done: u64,
         bytes_total: u64,
     }
     ```
     - `PackageZipper::new(files: Vec<PackageFile>) -> Self` pre-allocates the output `Vec<u8>` like `zip_package`.
     - `PackageZipper::step(&mut self, max_bytes: usize) -> Result<ZipStep, PublishError>` writes up to `max_bytes.max(1)` bytes per call across `start_file` / `write_all(&chunk)` using the exact same `coursecraft_format::zip_options_for(&file.path)`, calling `writer.finish()` when `file_index == files.len()` and returning `ZipStep::Done(bytes)`.
     - Desktop `zip_package` and `zip_package_to` stay 100% unchanged!
  2. In `crates/ui-egui/src/publish.rs`: add an optional `pub progress: Option<f32>` (0.0..=1.0) on `PublishState` (or `set_publish_progress(Option<f32>)`) so while `PublishPhase::Busy`, the footer shows `"Publishing… 45%"` (when `Some`) or `"Publishing…"` (when `None`, keeping desktop identical).
  3. In `apps/coursecraft/src/main.rs` (`#[cfg(target_arch = "wasm32")]`):
     - Add `web_publish: Option<WebPublishJob>` (`Preparing { standard }`, `Zipping { standard, zipper }`).
     - On `EditorRequest::Publish { standard }`: ignore if `web_publish.is_some()`; call `self.editor.set_publish_busy()`, store `WebPublishJob::Preparing { standard }`, call `ui.ctx().request_repaint()`, and return immediately so frame 0 paints the busy spinner!
     - Each subsequent frame (`step_web_publish`):
       - `Preparing` → build `files = publish::package_files_from(...)`, transition to `Zipping { standard, zipper: PackageZipper::new(files) }`, request repaint.
       - `Zipping` → run `zipper.step(256 * 1024)` (~10–25 ms per frame), update `set_publish_progress(Some(done / total))`, request repaint; on `Done(bytes)` trigger `web_download_bytes(...)` and call `set_publish_success(...)` (or `set_publish_failure(...)` on error).
  - **What's left (future, not needed for 0.4.2):** True off-main-thread zipping via a Web Worker would require separate wasm-bindgen worker entry packaging in `xtask`/Trunk; frame-chunked `PackageZipper` keeps the UI thread responsive and shows live progress today with zero build-pipeline risk.
- **Tests:** `M31-20` (`crates/publish`: `PackageZipper` chunked at 1 KiB, 64 KiB, and 256 KiB produces bytes **identical** to `zip_package`, handles empty files, empty package, and precompressed vs deflated files), `M31-21` (`crates/ui-egui`: `PublishPhase::Busy` renders percentage when `progress` is set and plain `"Publishing…"` when `None`), plus `cargo xtask wasm` verification.

---

## 2. Watchlist of Risky Spots (`RISK-01`–`RISK-06`)

| ID | File & Lines | Risk & Mitigation |
|---|---|---|
| **RISK-01** | `crates/ui-egui/src/tests.rs:7512` (`shortcut_dialog_shows_every_row_without_scrolling_at_1440x900`) | Adding 4 separate File rows could push `dialog_height(&groups, 900.0)` over the `720 pt` 80% cap. Move `"Publish course"` out of `General` into `File` and use compact paired rows (`"New / Open"`, `"Save / Save As"`, `"Publish course"`), AND teach `format_shortcut` to format `" / "`-separated shortcuts so macOS chips stay native. Run `shortcut_dialog_shows_every_row_without_scrolling_at_1440x900` right in `S1`. |
| **RISK-02** | `crates/ui-egui/src/lib.rs:4298-4330` | egui's `consume_shortcut` for `Modifiers::COMMAND` matches even when `Shift` is held. **Always** check `Ctrl+Shift+S` before `Ctrl+S`, and test both orders in `M31-01`. |
| **RISK-03** | `crates/ui-egui/src/lib.rs:4175-4184` | Do **not** gate `Ctrl+S`/`O`/`N` on `ctx.memory(|m| m.focused().is_some())` — clicking a toolbar button leaves non-text focus set. Gate on `!ctx.egui_wants_keyboard_input() && self.ui.inline_edit.is_none()` + dialog/preview/crop checks (same pattern as `Ctrl+Shift+P`). |
| **RISK-04** | `apps/coursecraft/src/main.rs` (`wasm32`) | Browsers only allow `<input type="file">` (`AsyncFileDialog::pick_file`) inside a user click gesture. Trigger `spawn_local(AsyncFileDialog::pick_file())` directly inside the `"Don't Save"` / `"Save"` modal button `.clicked()` branch (and keep `web_download_bytes` synchronous before it). |
| **RISK-05** | `crates/lms/src/bridge.rs:720-794` | Under `PassedIncomplete`/`PassedFailed`, `Completion { Completed }` sets `self.passed = true` with `best_passed_score = None`. Verify that a *later* passing quiz (`Score { Passed, 90 }`) still writes `90` (because `best_passed_score` is `None`), while a failing quiz (`Score { Failed, 40 }`) is ignored. |
| **RISK-06** | `crates/publish/src/lib.rs:168` & `crates/course/src/validate.rs:2194` | Check `crates/course/src/m29_state_validation_tests.rs` to ensure existing tests for `state.src` expect a warning (not 0 issues) or update only if an existing test explicitly asserted 0 warnings for `src: Some("")`. |

---

## 3. Numbered Implementer Steps (`S1`–`S9`)

> **Reviewer notes (Claude, binding for implementers)**
> - **S1, web editor:** Ctrl/Cmd+S must not also open the browser's own "Save page" dialog (and Ctrl+O/N must not open the browser's file/new-window actions where the page can stop them). Check what eframe does with consumed keys on the web; if the browser default still fires, prevent it for these four shortcuts only. Text fields keep their normal keys.
> - **S6:** in the two "completed" modes, once "completed" (or "passed") has been reported, a later fail must not write "incomplete" or "failed"; score updates may still be sent. Test each of the five modes, in-session and after resume.
> - **S8, S9:** the web shell code is compiled only for wasm, so native clippy/tests don't see it. Check it yourself with `cargo check -p coursecraft --target wasm32-unknown-unknown` (and `cargo clippy` for that target if it runs) before finishing; the driver's full gate also builds wasm.

> **Gate cadence:** Run `cargo test -p <crate> -j 4` on every step. Run `cargo xtask build-player` + `cargo xtask ci` after **S4**, **S7**, and **S9**.

### Must Fix (`S1`–`S7`)

- **S1 — Add `Ctrl+S` / `Ctrl+Shift+S` / `Ctrl+O` / `Ctrl+N` Shortcuts, Menu Hints & Shortcuts Dialog**
  - **Files:** `crates/ui-egui/src/lib.rs` (`:4224-4565`), `crates/ui-egui/src/menu.rs` (`:31-159, 571-643`), `crates/ui-egui/src/shortcuts.rs` (`:245-354`), `apps/coursecraft/src/main.rs` (`:1106-1211`), `crates/ui-egui/src/tests.rs`, `apps/coursecraft/src/u16_shell_tests.rs`.
  - **Exact changes:**
    1. Hoist `let mut requests = Vec::new();` above the shortcut block in `Editor::show` (`lib.rs`). Consume `Ctrl+Shift+S` before `Ctrl+S`, then `Ctrl+O`, then `Ctrl+N` under the non-text/non-modal guard (`RISK-02`, `RISK-03`), gating `Save`/`SaveAs` on `self.file_ops_available && !self.ui.show_welcome` and `Open` on `self.file_ops_available`.
    2. In `apps/coursecraft/src/main.rs`, ignore incoming `New`/`Open`/`Save`/`SaveAs` `EditorRequest`s while `pending_action`, `conflict_prompt`, `restore_prompt`, or `error_modal` is active.
    3. Add `.shortcut_text(shortcut_label(ui.ctx(), ...))` to File menu items `"New"`, `"Open…"`, `"Save"`, `"Save As…"` in `menu.rs`, and extend `format_shortcut` to format `" / "`-joined compound shortcuts.
    4. Add `"File"` group (`"New / Open"`, `"Save / Save As"`, `"Publish course"`) to `build_shortcut_groups()` in `shortcuts.rs` and remove duplicate `"Publish course"` from `General` (`RISK-01`).
  - **Tests:** `M31-01`, `M31-02`, `M31-03`, `M31-04`.
  - **Done when:** `cargo test -p coursecraft-ui-egui -j 4 shortcut` and `cargo test -p coursecraft -j 4` pass, including `shortcut_dialog_shows_every_row_without_scrolling_at_1440x900`.

- **S2 — Preserve Media in MCP `--course` Startup & All Headless/`--attach` Open/Save Paths**
  - **Files:** `crates/control/src/host.rs` (`:204-506`), `apps/coursecraft-mcp/src/lib.rs` (`:57-72, 248-258`), `apps/coursecraft-mcp/src/tests.rs`, `crates/control/src/tests.rs`, `apps/coursecraft/src/tests.rs`.
  - **Exact changes:**
    1. Add `HeadlessHost::open_path(path: &Path) -> Result<Self, HostError>` delegating to `ControlHost::open(&mut host, path, true)`.
    2. In `HeadlessHost::save` (`host.rs:406-468`), collect in-memory media plus any referenced course media resolved from `self.media_source.get(&r.src)` (preserving sibling `media/` when converting `course.json` → `.ccraft`/folder), and update `self.media_source.set_path(Some(target.clone()))`.
    3. Replace `load_course` in `apps/coursecraft-mcp/src/lib.rs` with `open_headless_host(course_path)` calling `HeadlessHost::open_path`.
  - **Tests:** `M31-05`, `M31-06`, `M31-07`.
  - **Done when:** `cargo test -p coursecraft-control -j 4`, `cargo test -p coursecraft-mcp -j 4`, and `cargo test -p coursecraft -j 4` pass with `.ccraft`, `.ccraft-folder`, `course.json`+`media/`, and `--attach` media round-trip coverage.

- **S3 — Fix Editor Panics on Tiny Slides (`< 20 px`) & Tall Scroll Panels (`> 20,000 px`) + Insert Tests**
  - **Files:** `crates/ui-egui/src/input_insert.rs` (`:22-32`), `crates/ui-egui/src/interaction_insert.rs` (`:8-18`), `crates/ui-egui/src/properties.rs` (`:7154-7203`), `crates/ui-egui/src/bughunt_insert.rs`, `crates/engine/src/tests.rs` (`:2805-2841`).
  - **Exact changes:**
    1. In `input_insert.rs::centred_frame` and `interaction_insert.rs::centred_frame`, clamp width/height safely via `coursecraft_course::safe_clamp(width, 20.0_f32.min(size.w as f32).max(1.0), (size.w as f32).max(1.0))` (same for `height`).
    2. In `properties.rs::show_scroll_panel_controls`, bound `DragValue::range` lower limit to `frame_height.max(0.0).min(coursecraft_course::MAX_SCROLL_CONTENT_HEIGHT)` and replace `fit_height.clamp(...)` with `coursecraft_course::safe_clamp(fit_height, frame_height.max(0.0), coursecraft_course::MAX_SCROLL_CONTENT_HEIGHT)`.
    3. Add tests for every UI insert helper on `1×1`, `10×10`, and `8192×8192` slides, `"Fit to content"` on a `25_000 px` scroll panel, and extend engine `panic_hunt` with tiny/huge slide sizes across all insert commands.
  - **Tests:** `M31-08`, `M31-09`, `M31-10`.
  - **Done when:** `cargo test -p coursecraft-ui-egui -j 4 bughunt_insert` and `cargo test -p coursecraft-engine -j 4 panic_hunt` pass with zero panics.

- **S4 — Workspace-Wide `safe_clamp` Hardening Sweep**
  - **Files:** `crates/course/src/model.rs` (`:1371-1377`), `crates/course/src/shapes.rs` (`:1761`), `crates/render/src/access.rs` (`:763`), `crates/render/src/shape_fill.rs` (`:1056`), `crates/render/src/player_frame.rs` (`:1300, 2142, 2144, 2486, 2501, 3046`), `crates/render/src/focus.rs` (`:2414, 2419`), `crates/render/src/shape_geometry.rs` (`:973, 1134, 2870-2882`), `crates/render/src/compose.rs` (`:3106, 3108`), `crates/render/src/paint.rs` (`:3715, 3728, 3740`), `crates/render/src/view.rs` (`:697, 780-781`), `crates/ui-egui/src/preview.rs` (`:841-842, 1065, 1419-1420, 1433-1434`), `crates/ui-egui/src/translate_ai.rs` (`:641`), `crates/ui-egui/src/coach.rs` (`:133`).
  - **Exact changes:**
    1. Replace every risky `.clamp(...)` listed in the Item 3 audit table with `coursecraft_course::safe_clamp(...)` (and make `model.rs::finite_clamp` delegate to `safe_clamp` after checking `value.is_nan()`).
    2. Add unit tests passing inverted rects (`Rect::NOTHING`), negative `w`/`h`, and `NaN` bounds into `shape_geometry`, `shape_fill`, `focus::clamp_point`, `player_frame::slider_value`, `preview::clamp_preview_size`, and `course::shapes` without panicking.
  - **Tests:** `M31-11`.
  - **Gate #1:** `cargo xtask build-player` + `cargo xtask ci`.
  - **Done when:** All risky `.clamp(` sites use `safe_clamp` and Gate #1 is green.

- **S5 — Allow Publishing Empty `state.src: ""` & Warn in Check Course**
  - **Files:** `crates/publish/src/lib.rs` (`:167-173`), `crates/publish/src/tests.rs`, `crates/course/src/validate.rs` (`:2194-2220`), `crates/course/src/m29_state_validation_tests.rs`.
  - **Exact changes:**
    1. In `crates/publish/src/lib.rs:168`, change `if let Some(src) = &state.src` to `if let Some(src) = &state.src && !src.is_empty()`.
    2. In `crates/course/src/validate.rs:2194-2220`, add an `else` branch for `src.trim().is_empty()` emitting `Severity::Warning` at `format!("{p}/states/{i}/src")` explaining that the state's image path is empty and the base image will be used (`RISK-06`).
  - **Tests:** `M31-12`, `M31-13`.
  - **Done when:** `cargo test -p coursecraft-publish -j 4` and `cargo test -p coursecraft-course -j 4` pass.

- **S6 — Prevent SCORM 1.2 LMS Pass/Completion Downgrade Across All `LmsStatus` Modes**
  - **Files:** `crates/lms/src/bridge.rs` (`:190-195, 540-548, 720-794, 846-873`), `crates/lms/tests/u21_lms_status_tests.rs`, `docs/spec/triggers.md` (`:778`).
  - **Exact changes:**
    1. Add `is_protected_status(written: LessonStatus, lms_status: LmsStatus) -> bool` and update `CmiMapping::is_passed_for(&self, statuses, lms_status)` so `CompletedIncomplete` and `CompletedFailed` recognize `"completed"` on resume alongside `"passed"`.
    2. In `LmsBridge::handle_score` and `handle_completion`, reject status downgrades once `self.passed` is true (using the mapped status for SCORM 1.2), and set `self.passed = true` whenever a protected SCORM 1.2 status is written (`LessonStatus::Passed` for `Auto`/`PassedIncomplete`/`PassedFailed`; `LessonStatus::Passed | LessonStatus::Completed` for `CompletedIncomplete`/`CompletedFailed`), while preserving score upgrades when `best_passed_score` is `None` or higher (`RISK-05`).
    3. Update `docs/spec/triggers.md:778` to document the non-downgrade rule for all 5 SCORM 1.2 `lms_status` modes.
  - **Tests:** `M31-14`, `M31-15`, `M31-16`.
  - **Done when:** `cargo test -p coursecraft-lms -j 4` passes with 0 strict-LMS violations across all modes and standards.

- **S7 — Fix `SlideTarget::LastVisited` History Pop & Loop Bug**
  - **Files:** `crates/logic/src/run.rs` (`:805-825`), `crates/logic/src/tests.rs` (`:10415-10496`), `apps/coursecraft-cli/src/tests.rs`, `docs/spec/triggers.md` (`:380`).
  - **Exact changes:**
    1. Replace the `find_map` + single `pop()` block in `Player::jump` (`run.rs:807-824`) with a `while let Some(id) = self.st.visit_history.pop()` loop that skips missing IDs and `idx == cur`, falling back to `cur.checked_sub(1)` only after the stack is exhausted.
    2. Update `docs/spec/triggers.md:380` with the exact popping/skipping semantics.
    3. Add unit tests (`M31-17`) and a CLI `simulate` test (`M31-18`) covering bank redraw + `LastVisited`, consecutive duplicates, and fallback on slide 0.
  - **Tests:** `M31-17`, `M31-18`.
  - **Gate #2:** `cargo xtask build-player` + `cargo xtask ci`.
  - **Done when:** All logic, CLI simulate, and full CI checks pass.

### Should Fix (`S8`–`S9`)

- **S8 — Add Unsaved-Changes Prompt to Web Editor (`New`, `Open`, `Example`, `Template`, `PPTX`)**
  - **Files:** `apps/coursecraft/src/web_state.rs` (new, pure cross-platform state machine), `apps/coursecraft/src/main.rs` (`:217-236, 3075-3384, 3578-3635`), `apps/coursecraft/src/tests.rs`.
  - **Exact changes:**
    1. Implement `WebUnsavedState` + `WebPendingAction` (`New`, `Open`, `OpenExample`, `OpenTemplate`, `ImportPptx`) in `web_state.rs` with full native unit tests (`M31-19`).
    2. Wire `web_unsaved: WebUnsavedState` into wasm `EditorApp`, sync `self.editor.set_title_info(name, is_dirty)`, guard `New`/`Open`/`OpenExample`/`OpenTemplate`/`PickPptx`, and show the `"Save changes to {name}?"` modal (`Save`, `Don't Save`, `Cancel` / Esc) preserving user-gesture click invocation for `AsyncFileDialog` (`RISK-04`).
  - **Tests:** `M31-19`, plus `cargo xtask wasm` check.
  - **Done when:** Native unit tests for `WebUnsavedState` and `cargo xtask wasm` pass.

- **S9 — Chunked Frame-Yielding Web Publish (`PackageZipper`) & Final 0.4.2 Release Gate**
  - **Files:** `crates/publish/src/lib.rs` (`:621-658`), `crates/publish/src/tests.rs`, `crates/ui-egui/src/publish.rs` (`:138-184, 349-354`), `apps/coursecraft/src/main.rs` (`:217-236, 3385-3412, 3578-3590`), `docs/testing/m31-report.md` (new report at end of milestone).
  - **Exact changes:**
    1. Add `PackageZipper` and `ZipStep` in `crates/publish/src/lib.rs` writing `max_bytes` per `step()`, byte-identical to `zip_package`.
    2. Add `progress: Option<f32>` + `Editor::set_publish_progress` in `crates/ui-egui/src/publish.rs` so `PublishPhase::Busy` displays `"Publishing… {pct}%"` when present.
    3. Drive `WebPublishJob` (`Preparing` → `Zipping` at `256 KiB`/frame → `web_download_bytes`) in wasm `EditorApp`, requesting repaint each frame so the spinner renders and the browser tab stays responsive; leave desktop `start_publish` untouched.
    4. Write concise `docs/testing/m31-report.md` summarizing test IDs `M31-01`–`M31-21` and gate results.
  - **Tests:** `M31-20`, `M31-21`.
  - **Final Release Gate:** `cargo xtask build-player`, `cargo xtask ci`, `cargo xtask perf`.
  - **Done when:** All 21 tests (`M31-01`–`M31-21`) and the full release gate are green.

---

## 4. Release Gate for `0.4.2`

### 4.1 Automated Gate Commands (run in order)
1. `cargo xtask build-player` (rebuilds embedded WASM web player after `S4`, `S6`, `S7`, `S9`)
2. `cargo xtask ci` (`fmt --check`, `clippy --workspace --all-targets -D warnings`, `test --workspace`, `layers`, `wasm`)
3. `cargo xtask perf` (confirm 300-slide timings remain within baselines)

### 4.2 Owner Manual Verification Checklist (Desktop + Browser — ~5 min)
1. **File shortcuts (Desktop & Web):** Open a course, edit a slide, press `Cmd+S`/`Ctrl+S` (untitled → Save As dialog; saved file → saves cleanly, dirty dot clears), `Cmd+Shift+S`/`Ctrl+Shift+S` (Save As), `Cmd+N`/`Ctrl+N` and `Cmd+O`/`Ctrl+O` (on dirty course → unsaved-changes prompt). Confirm shortcuts appear in **File** menu and **Help ▸ Keyboard shortcuts** (`?`). Click into a text box and type — confirm `Cmd+N`/`O`/`S` do not fire while editing text.
2. **Publish with empty state image:** On an Image object with a state whose image is empty (`src: ""`), check **File ▸ Check Course** shows a warning (not an error) and **File ▸ Publish…** succeeds.
3. **Web editor unsaved changes & responsive publish:** Open the web editor, edit a slide, click **File ▸ New** (or press `Cmd+N`/`Ctrl+N`) and **File ▸ Open…** → confirm the `"Save changes?"` prompt appears and `Cancel` keeps edits. Open **File ▸ Publish…** and click **Publish** → confirm the `"Publishing…"` spinner and percentage animate smoothly while the zip builds without freezing the tab, then the `.zip` downloads.
