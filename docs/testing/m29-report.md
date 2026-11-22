# M29 Rich Object States — Release & UI/UX Gate Report

**Date:** 2026-11-23  
**Plan:** [`docs/testing/m29-plan.md`](m29-plan.md)  
**Detailed Bug-Hunt Log:** [`docs/bug-hunt/2026-11-22-m29.md`](../bug-hunt/2026-11-22-m29.md)  

---

## 1. Executive Summary

M29 delivered **Rich Object States** across three independently shippable slices (Steps **s1–s12**):

1. **Slice 1 (Steps s1–s2b):** Bottom `States` tab with side-by-side live thumbnail cards (`compose_object_with_value`, viewport-culled), `object.duplicate_state` / `object.rename_state` (atomic reference rewrite across `initial_state`, base/layer triggers, and `question.correct_when`) / `object.reorder_state`, compact Format panel `States` section, right-click `"Edit states…"`, keyboard/AccessKit navigation, and validation hardening (`MAX_STATES_PER_OBJECT = 32`, `MAX_STATE_NAME_CHARS = 64`, master-slide warnings/paste stripping).
2. **Slice 2 (Steps s3a–s7):** Widened `StateDef` overrides (14 fields + 6 `clear_*` flags) with backward-compatible serialization, `object.patch_state`, layered state resolver (`normal -> runtime -> hover/down`, `disabled`/`hidden` precedence), Canvas Edit-State Mode (non-blocking accent banner, dimming, parent frame/object-level locking, `Esc` precedence stack), Format inspector override dots & per-property reset, inline state text editing, `Find`/`Check Course`/`Media Library` deep navigation, and `uiux_sweep_all_surfaces` Slice 2 coverage.
3. **Slice 3 (Steps s8a–s12):** Parent-relative `StateExtra`s (`Shape`, `Image`, `Text`, `Path`, `MAX_STATE_EXTRAS = 16`, 4 `object.state_*_extra` commands), media/font/recolor/strings/XLIFF walkers, runtime & pointer extra composition/transforms/hit-testing/a11y (`ST-13`), canvas insert/select/move/resize/rotate/nudge/delete + `"Items in this state"` list + extra inline text edit/context menu/thumbnails, clean-room `examples/rich-states/` verified across all 6 publish targets (`strict-lms` / `StrictLrs` / `StrictLtiPlatform`), and full `3 × 2 × 2` + web UI/UX matrix sweep.

- **Full UI/UX Matrix Sweep (`uiux_sweep_all_surfaces`):** **68 surfaces**, **952 audited frames**, **0 unopened**, **0 MUST violations** (`target/uiux-sweep-report.json`), with `7` allowlisted `TextEdit::singleline` long-name entries in `crates/ui-egui/src/uiux_allowlist.rs` (unchanged from M28, budget `<= 20`).
- **Watchlist Disposition (`SUS-01`–`SUS-16`):** All **16** suspects in `docs/testing/m29-plan.md` were reproduced and fixed across Steps **s1–s10b** (see §2).
- **Driver-Deferred Gates:** Per the Step s12 run rules, `cargo xtask build-player`, `cargo xtask ci`, and `cargo xtask perf` (`PERF-04`) were **not** run in this session and are left to the driver.

---

## 2. Complete Disposition of Suspects (`SUS-01`–`SUS-16`)

| ID | Area / File | Disposition | Step | Summary of Fix |
|---|---|---|---|---|
| **SUS-01** | `crates/render/src/access.rs:1788-1842` | Reproduced & Fixed | s9b | `object_item` and `hotspot_item` now skip `item.is_state_extra`; added `append_extra_labels` (`ST-13`) without adding focus stops. |
| **SUS-02** | `crates/render/src/compose.rs:2503-2510`, `view.rs` | Reproduced & Fixed | s9a | `is_target_label`, `mark_drag_items`, and drag/click helpers skip `item.is_state_extra`; `lift_dragged_item` lifts parent + extras together. |
| **SUS-03** | `crates/ui-egui/src/canvas.rs:4642-4688, 6322-6339` | Reproduced & Fixed | s2a, s9b | Replaced single-item preview with `apply_canvas_state_preview` (`compose_object_items` + `Some(&course.theme)` + `missing_images_as_placeholders`). |
| **SUS-04** | `crates/ui-egui/src/lib.rs:4817-5045`, `course/src/lib.rs` | Reproduced & Fixed | s3b, s8b | Extended `collect_object_media_srcs`, `rewrite_object_media_srcs`, and `walk_object_media_references` across `state.fill_style`, `Path.fill_style`, `Video.poster`, and `state.extras`. |
| **SUS-05** | `crates/publish/src/lib.rs:167-210, 697-795` | Reproduced & Fixed | s3b, s8b | Extended `collect_object_media` and `collect_object_faces` to walk `state.fill_style`, `state.text_style`/`runs`, and `state.extras`. |
| **SUS-06** | `apps/coursecraft-player/src/media.rs:133-218` | Reproduced & Fixed | s3b, s9b | Extended `collect_object_srcs` across `state.fill_style`, `Path.fill_style`, `Video.poster`, and `state.extras` for prefetch & LRU protection. |
| **SUS-07** | `crates/ui-egui/src/lib.rs:5015-5045` | Reproduced & Fixed | s3b, s8b | Extended `object_uses_font` to check `state.text_style`, `state.runs`, and `state.extras` (`Text`, `Shape.text`). |
| **SUS-08** | `crates/course/src/validate.rs:796-805` | Reproduced & Fixed | s1 | `check_master` emits `Severity::Warning` for states on master objects (preserving existing courses); engine commands refuse master state mutations. |
| **SUS-09** | `crates/ui-egui/src/states.rs:199-231` | Reproduced & Fixed | s2b | `state_delete_blocker` checks `slide.question.correct_when` recursively (`condition_references_state`). |
| **SUS-10** | `crates/course/src/validate.rs:893-912` | Reproduced & Fixed | s8b | `slide_has_unsupported_script` checks `state.runs` and `state.extras` (`Text`, `Shape.text`, `Image.alt`). |
| **SUS-11** | `crates/course/src/strings.rs`, `crates/format/src/xliff.rs` | Reproduced & Fixed | s3b, s8b | Extended string walkers and XLIFF export/import (`<g>` run tags) to keep `state.text`, `state.runs`, `state.paragraphs`, and `state.extras` synced. |
| **SUS-12** | `crates/ui-egui/src/canvas.rs:221-337, 685-880` | Reproduced & Fixed | s6, s10b | Added `InlineEdit.state_name` and `InlineEdit.extra_id` so inline text edits commit to `object.patch_state` or `object.state_update_extra`. |
| **SUS-13** | `crates/ui-egui/src/canvas.rs:6025-6062`, `states.rs` | Reproduced & Fixed | s5, s10a | Strict `Esc` precedence stack (`popup → inline edit/crop/tool → selected_extra → editing_state → selection`). |
| **SUS-14** | `crates/ui-egui/src/properties.rs`, `states.rs` | Reproduced & Fixed | s2a, s6 | Replaced duplicate `"State"` / `"States"` sections with single compact `show_compact_states_section` (`Initial` + `"Open States tab (N)"`). |
| **SUS-15** | `crates/engine/src/commands.rs:5448-5451` | Reproduced & Fixed | s1 | `object_paste` onto a master slide clears `object.states` and resets `object.initial_state = "normal"`. |
| **SUS-16** | `crates/ui-egui/src/find.rs`, `status.rs`, `media_library.rs` | Reproduced & Fixed | s7 | Added `Editor::preview_navigated_state` so Find, Check Course `/states/...`, and Media Library places open `BottomTab::States` and preview the matched state. |

---

## 3. UI/UX Matrix Sweep & Step s12 Fixes

### 3.1 Step s12 Additions (`U-20`)
- Added **`bottom-states-long`** (64-char state name) and **`edit-state-mode-extra`** (Edit-State Mode with a `StateExtra` selected) to `all_surfaces()` in `crates/ui-egui/src/uiux_sweep_tests.rs`.
- Added **`bottom-states-1`**, **`edit-state-mode`**, and **`edit-state-mode-extra`** to `core_labels` in `uiux_sweep_scale_2_and_fixtures` (scale `2.0` + `empty_course` + `long_names_course`).

### 3.2 Violations Found & Fixed in Step s12
1. **States tab header overflow on long object names (`crates/ui-egui/src/states.rs:1373, 1393-1400`):**
   - *Before:* `ui.add(egui::Label::new(...).truncate())` inside `ui.horizontal` consumed the full available width on long object names, pushing `"2 states"` and `"Initial"` past the panel clip rect (`UX-04`).
   - *Fix:* Bounded the object-name label scope with `ui.set_max_width(140.0)` and raised `wide_header` threshold from `780.0` to `900.0`.
2. **`"Items in this state"` delete button contrast (`crates/ui-egui/src/properties.rs:2607-2609`):**
   - *Before:* `RichText::new("×").color(p.danger)` painted `#FF6B63` on `#3A3A3F` (`4.06:1`) in Dark and `#D70015` on `#DEDEE2` (`4.01:1`) in Light, failing `UX-06` (`>= 4.5:1`).
   - *Fix:* Switched to `egui::Button::new("×").min_size(egui::vec2(24.0, 24.0))` using `p.text` (`> 10:1` contrast) while preserving the `"Delete extra"` tooltip and `24×24 pt` hit target.

### 3.3 Observed Test Results (Step s12)
- `cargo test -p coursecraft-ui-egui -j 4 uiux_sweep`: **3 passed, 0 failed** (`uiux_sweep_all_surfaces`, `uiux_sweep_scale_2_and_fixtures`, `uiux_sweep_modals_and_tooltips`).
  - `target/uiux-sweep-report.json`: `68` surfaces, `952` audited frames, `0` unopened, `0` violations.
- `cargo test -p coursecraft-render -j 4 uiux_player`: **2 passed, 0 failed** (`uiux_player_sweep`, `uiux_player_preview_vs_published_pixel_parity`).
- `cargo clippy -p coursecraft-ui-egui --all-targets -j 4 -- -D warnings`: **0 warnings / 0 errors** (verified after final edit).
- `cargo test -p coursecraft-ui-egui -j 4`: **full crate test suite** (verified after final edit; see Step s12 log).

---

## 4. Test Coverage Matrix (`E-01`–`E-18`, `R-01`–`R-16`, `P-01`–`P-08`, `U-01`–`U-20`, `PUB-01`–`PUB-08`, `E2E-01`–`E2E-05`, `PERF-01`–`PERF-04`, `FZ-01`–`FZ-06`)

All test IDs from `docs/testing/m29-plan.md` are implemented and tracked in [`docs/bug-hunt/2026-11-22-m29.md`](../bug-hunt/2026-11-22-m29.md):
- **Engine & Model (`E-01`–`E-18`, `FZ-01`–`FZ-04`, `FZ-06`):** Covered in `crates/course/src/m29_state_validation_tests.rs`, `crates/engine/src/m29_state_commands_tests.rs`, `crates/engine/src/m27_all_commands_undo_tests.rs`, `crates/engine/src/tests.rs`, `crates/format/src/tests.rs`, `crates/format/tests/xliff_parity.rs`, and `crates/ui-egui/src/m29_slice2_interactions_tests.rs`.
- **Render & Runtime (`R-01`–`R-16`, `P-01`–`P-08`, `FZ-05`):** Covered in `crates/render/src/m29_state_resolver_tests.rs`, `crates/render/src/m29_state_extras_tests.rs`, and `apps/coursecraft-cli/tests/m29_rich_states.rs`.
- **UI/UX (`U-01`–`U-20`):** Covered in `crates/ui-egui/src/m29_states_tab_tests.rs`, `crates/ui-egui/src/m29_edit_state_mode_tests.rs`, `crates/ui-egui/src/m29_format_state_routing_tests.rs`, `crates/ui-egui/src/m29_slice2_interactions_tests.rs`, `crates/ui-egui/src/m29_state_extras_ui_tests.rs`, and `crates/ui-egui/src/uiux_sweep_tests.rs`.
- **Publish & E2E (`PUB-01`–`PUB-08`, `E2E-01`–`E2E-05`):** Covered in `crates/publish/src/tests.rs`, `apps/coursecraft-player/src/tests.rs`, and `apps/coursecraft-cli/tests/m29_rich_states.rs` (`examples/rich-states/`).
- **Performance Guards (`PERF-01`–`PERF-04`):** `PERF-01` (`m29_states_tab_tests.rs`), `PERF-02` (`guard_tests.rs`), `PERF-03` (`m29_format_state_routing_tests.rs`) pass in-crate; `PERF-04` (`cargo xtask perf`) and `cargo xtask build-player` / `cargo xtask ci` are deferred to the driver.
