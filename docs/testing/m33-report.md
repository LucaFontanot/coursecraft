# M33 Delta Undo Engine Report

**Milestone:** M33 (Undo that stores changes, not copies)  
**Plan:** [`docs/testing/m33-plan.md`](m33-plan.md)  
**Scope:** Replace `coursecraft_engine::Session`'s full-course-copy undo/redo history (`Vec<(Course, String)>`) with a reversible structural delta engine (`CourseDelta` in `VecDeque<(CourseDelta, String)>`) and scoped pre-edit snapshotting (`UndoScope`), preserving byte-identical `Course::to_json()` restoration, atomic batches and error rollback, `skip_undo`/no-op parity, hostile-input safety, and WASM compatibility, across driver steps `S1`–`S9` (implementing plan steps `S1`–`S8`).

## What Shipped Per Step (`S1`–`S9`)

| Step | Plan Step | Summary | Review Verdict |
|---|---|---|---|
| **S1** | S1 | Added `#[cfg(debug_assertions)] pub mod test_counts` (`crates/engine/src/test_counts.rs`, thread-local `Cell<u64>`, WASM-safe) wired via `clone_course_counted` at all 3 `Session` full-course clone sites in `crates/engine/src/lib.rs`; added deterministic `perf300()` / `rich300()` generators and `#[ignore]` release benchmarks (`PF-01`–`PF-03`) in `crates/engine/src/undo_perf_tests.rs`. | `OK` |
| **S2** | S2 (part 1) | Implemented slide-level structural delta and course-header snapshot/patch in `crates/engine/src/delta.rs` (`SlideLoc`, `SlideFieldPatch`, `SlideDelta`, `CourseHeaderSnapshot`, `CourseHeaderPatch`) with compile-time exhaustive destructuring (no `..`) over all 21 `Course`, 15 `Slide`, and 13 `Layer` fields, zero-clone symmetric in-place swaps, bounds-checked `Err(EngineError::Invalid(_))` handling, and `estimated_bytes()`. | `OK` |
| **S3** | S2 (part 2) | Implemented course-level structural delta in `crates/engine/src/delta.rs` (`ListOp<T>`, `StructuralOp`, `CoursePatch`, `CourseDelta`) with exhaustive destructuring of `Course`, `Scene`, and `QuestionBank`, `> 50%` multi-slide wholesale `FullCourse` fallback (preserving `SlideChange` on 1-slide courses), atomic rollback unwinding on partial `CoursePatch`/`Sequence` failures, and `estimated_bytes()`. | `OK` |
| **S4** | S3 | Added `#[cfg(test)] assert_delta_roundtrip` after `reroute_connectors` in `Session::execute` and `Session::execute_batch_with_name` (keeping primary `Vec<(Course, String)>` unchanged) and added `DF-01`–`DF-04` differential tests across all 177 mutating commands in the 186-command `REGISTRY`. Review fix rewrote `Df02LegacyOracle` in `crates/engine/src/tests.rs` to invoke `(cmd.run)` directly with independent legacy snapshot logic and tightened `DF-02` `Ok`/`Err` assertions. | `FIXED` |
| **S5** | S4 | Switched `Session::undo` / `Session::redo` to `VecDeque<(CourseDelta, String)>` with O(1) `pop_front()` eviction at `MAX_HISTORY = 200`, atomic unwind on failed `delta.apply` and mid-loop `undo_steps`/`redo_steps` errors, `undo_estimated_bytes()`, and preserved `skip_undo`, no-op redo preservation, `valid_at_revision`, and `#[cfg(test)]` shadow oracle (`shadow_undo`/`shadow_redo`). Review fix tightened `undo_estimated_bytes_tracks_undo_and_redo_deltas` to verify exact byte tracking across both `undo` and `redo`. | `FIXED` |
| **S6** | S5 | Extracted `reroute_connectors_in_slide` (`crates/engine/src/connectors.rs`), added `UndoScope` (`FullCourse`, `SingleSlide`, `CourseHeader`) with pilot annotations (`object.set_frame`, `object.update`, `text.set`, `course.set_title`), split `Session::execute` into scoped execution paths, added the same-slide batch fast-path (removing the redundant `step_snapshot` full-course clone), and added the `#[cfg(test)]` scope-isolation verifier (`VERIFY_SCOPE`). Review fix added `SlideLoc::find_editable` matching `Course::slide_mut` precedence (`scenes -> banks -> masters`) so duplicate-ID malformed courses cannot snapshot the wrong slide, plus a regression test. | `FIXED` |
| **S7** | S6 | Populated `undo_scope` in `crates/engine/src/commands.rs` with 140 verified mutating commands (`25` `CourseHeader` + `115` `SingleSlide`), leaving all `37` cross-slide, structural, and course+slide mutating commands on `_ => UndoScope::FullCourse`; added `sc_01_all_annotated_commands_isolated_and_zero_full_clones` verifying `0` full-course clones, scope isolation, and byte-identical undo/redo JSON across all 177 mutating commands. | `OK` |
| **S8** | S7 | Implemented `CourseDelta::merge` (O(1) collapse for repeated same-target `ObjectModified`, same-slide `FullSlide`, and `FullCourse`; flat `Sequence` composition otherwise) and `Session::merge_last_two_undos`, added `U-07` tests, and removed the temporary `#[cfg(test)]` `shadow_undo`/`shadow_redo` oracle from `Session` while keeping `VERIFY_SCOPE` and standalone `DF-01`/`DF-02` differential tests intact. | `OK` |
| **S9** | S8 | Added `GD-02` (`two_hundred_edits_on_300_slide_course_zero_full_clones_and_small_deltas`), gated `#[cfg(test)]` `verify_before` clones on `VERIFY_SCOPE.get()`, removed synthetic `note_course_copy()` calls in `crates/ui-egui/src/lib.rs`, wired `guard_counters::course_copies()` / `reset()` to real `coursecraft_engine::test_counts::full_course_clones()`, tightened `GD-01` to assert `gc::course_copies() == 0`, added byte-budget and zero-clone assertions to `PF-01`–`PF-03`, and updated `docs/performance.md`. | `OK` |

## Test Coverage (`U-01`–`U-07`, `DF-01`–`DF-04`, `SC-01`–`SC-03`, `GD-01`–`GD-03`, `PF-01`–`PF-04`)

| ID | Step | Crate / File | Coverage |
|---|---|---|---|
| `U-01` | S2 | `coursecraft-engine` (`delta.rs`) | `u_01_slide_delta_single_object_ops_base_and_layer`: `ObjectModified`, `ObjectInserted`, `ObjectRemoved`, and `ObjectReordered` on base and slide layers round-trip byte-identically |
| `U-02` | S2 | `coursecraft-engine` (`delta.rs`) | `u_02_slide_delta_all_15_fields`: mutating each of the 15 `Slide` fields individually produces `SlideDelta::Fields` (or object/layer variant) and round-trips byte-identically |
| `U-03` | S2 | `coursecraft-engine` (`delta.rs`) | `u_03_course_header_patch_all_17_fields`: mutating each of the 17 `Course` header fields individually produces `CourseHeaderPatch` and round-trips byte-identically |
| `U-04` | S3 | `coursecraft-engine` (`delta.rs`) | `u_04_course_delta_structural_ops_are_compact_and_roundtrip`: slide/scene/master/template/bank add, delete, move, rename, trigger cleanup, and cross-collection moves produce compact deltas (never `FullCourse`) and round-trip byte-identically |
| `U-05` | S3 | `coursecraft-engine` (`delta.rs`) | `u_05_course_delta_wholesale_fallback_over_half_slides`: 5/10 slides → `Patch`, 6/10 slides → `FullCourse`, 1/1 slide → `SlideChange`, all round-tripping byte-identically |
| `U-06` | S2, S3 | `coursecraft-engine` (`delta.rs`) | `u_06_slide_delta_hostile_out_of_bounds_no_panic` & `u_06_course_delta_hostile_out_of_bounds_no_panic`: out-of-bounds `SlideLoc`, layer/object indices, `StructuralOp`, and partial-failure `Patch`/`Sequence` return `Err(EngineError::Invalid(_))` with 0 panics and leave course and delta untouched |
| `U-07` | S8 | `coursecraft-engine` (`delta.rs`, `tests.rs`) | `u_07_delta_merge_collapses_same_object_and_sequences_mixed` & `u_07_session_merge_last_two_undos_roundtrip`: constant-size collapse for `ObjectModified`/`FullSlide`/`FullCourse`, flat `Sequence` for mixed deltas, `< 2` guard, label handling, `revision`/`redo` preservation, byte-identical round-trips |
| `DF-01` | S4 | `coursecraft-engine` (`m27_all_commands_undo_tests.rs`) | `df_01_all_mutating_commands_byte_identical_delta_oracle`: all 177 mutating commands in `REGISTRY` checked for byte-identical `to_json()`, `undo_names()`, `redo_names()`, and `revision` across execute → undo → redo → undo |
| `DF-02` | S4 | `coursecraft-engine` (`tests.rs`) | `df_02_differential_250_step_mixed_sequence_with_eviction`: 250-step mixed edits, batches, `skip_undo`/no-ops, failing commands/batches, `undo_steps`/`redo_steps`, and `MAX_HISTORY` eviction verified against independent `Df02LegacyOracle` |
| `DF-03` | S4 | `coursecraft-engine` (`connector_tests.rs`) | `df_03_glued_connector_reroute_undo_redo_byte_identical`: moving a shape with a glued connector captures rerouting in the same undo step and restores shape + points byte-identically |
| `DF-04` | S4 | `coursecraft-engine` (`bughunt_undo_tests.rs`) | `df_04_failed_and_hostile_commands_byte_identical_parity`: empty history, failed commands, and hostile huge/negative frames match legacy rollback and history metadata byte-for-byte |
| `SC-01` | S6, S7 | `coursecraft-engine` (`tests.rs`, `m27_all_commands_undo_tests.rs`) | `sc_01_pilot_scoped_commands_zero_full_clones_and_isolated` & `sc_01_all_annotated_commands_isolated_and_zero_full_clones`: verifies 140 annotated commands (`>= 130`), `mutates == true`, `full_course_clones() == 0`, scope-isolation verifier, and byte-identical undo/redo JSON |
| `SC-02` | S6 | `coursecraft-engine` (`tests.rs`) | `sc_02_same_slide_batch_zero_full_clones_and_atomic_rollback`: 5-step same-slide batch takes 0 full-course clones and rolls back atomically when a later step fails |
| `SC-03` | S6 | `coursecraft-engine` (`tests.rs`) | `sc_03_single_slide_missing_or_unknown_slide_falls_back_safely` & `sc_03_single_slide_matches_course_slide_mut_precedence_with_duplicate_ids`: missing/unknown slide fallback and duplicate-ID precedence parity with `Course::slide_mut` |
| `GD-01` | S9 | `coursecraft-ui-egui` (`guard_tests.rs`) | `moving_an_object_redraws_only_that_slide_thumbnail`: wired to real `coursecraft_engine::test_counts::full_course_clones()`, asserting `gc::course_copies() == 0` on object nudge |
| `GD-02` | S9 | `coursecraft-engine` (`tests.rs`) | `two_hundred_edits_on_300_slide_course_zero_full_clones_and_small_deltas`: 200 `object.set_frame` edits on `perf300` take 0 full-course clones, `undo_len() == 200`, `undo_estimated_bytes() < 512 KiB`, and byte-identical `undo_steps(200)` / `redo_steps(200)` |
| `GD-03` | S7, S9 | `coursecraft-engine` (`tests.rs`) | `panic_hunt` passes across all commands with 0 panics and 0 scope violations |
| `PF-01`–`PF-03` | S1, S9 | `coursecraft-engine` (`undo_perf_tests.rs`, `#[ignore]`) | `pf_01_perf300_200_edits_undo_redo_and_eviction`, `pf_02_rich300_200_edits_undo_redo`, `pf_03_perf300_200_noop_edits`: assert `undo_estimated_bytes()` budgets (`<= 512 KiB`, `<= 2 MiB`, `0`), byte-identical `to_json()`, and `full_course_clones() == 0` (debug run passed in S9 review; release run pending manual execution) |
| `PF-04` | Driver | `coursecraft-ui-egui` (`perf_tests.rs`, `--release --ignored`) | All 8 release editor perf tests passed within budget (see measurements below) |
| Extra | S1, S2, S5 | `coursecraft-engine` (`undo_perf_tests.rs`, `delta.rs`, `tests.rs`) | `test_counts_tracks_full_course_clones`, `slide_loc_find_and_lookup_all_collections`, `undo_estimated_bytes_tracks_undo_and_redo_deltas`, `undo_steps_and_redo_steps_unwind_atomically_on_mid_loop_delta_error` |

## Driver Gates

- **`cargo xtask ci`:**
  - `g1` (after S4): passed on attempt 1.
  - `g2` (after S7): passed on attempt 1.
  - `g3` (after S8): passed on attempt 1.
  - `g4` (after S9): passed on attempt 1 — `coursecraft-engine` (`372 passed; 0 failed; 3 ignored` unit tests + `1 passed` integration test), `coursecraft-ui-egui` (`1295 passed; 0 failed; 10 ignored`, including `17` `guard_tests`), `layers: ok`, and `cargo check --target wasm32-unknown-unknown` across all 17 WASM crates.
- **`cargo xtask build-player` (`p1`, after `g4`):** passed (`trunk build --release`, finished `wasm-release` profile in `0.23s`).
- **Release editor perf suite (`perf.log` — `cargo test --release -p coursecraft-ui-egui perf_tests -- --ignored --nocapture --test-threads=1`):** passed (`8 passed; 0 failed; 0 ignored` in `2.12s`).

## Measured Performance vs. Plan Baseline

### Editor Release Perf (`perf.log`)

Source: `/Users/aditya/Documents/Claude Projects/ScormCraft/target/m33/perf.log` (Apple Silicon macOS, `--release`, `--test-threads=1`) compared against the baseline and targets recorded in `docs/testing/m33-plan.md` §2.2 & §5:

| Stage | Budget | Plan Baseline (median / p95) | M33 Target (median / p95) | Measured After M33 (median / p95) | Notes |
|---|---:|---:|---:|---:|---|
| `undo / redo move` (30 cycles) | 33 ms | 9.52 ms / 9.85 ms | ≤ 6.5 ms / ≤ 7.5 ms | **8.73 ms / 9.03 ms** | Below 33 ms budget and faster than 9.52 / 9.85 ms baseline; above 6.5 / 7.5 ms stretch target (see Open Concerns) |
| `insert interaction + undo` | 100 ms | 2.90 ms / 3.62 ms | — | **3.32 ms / 4.77 ms** | 8 kinds; well under 100 ms budget |
| `idle` | 16 ms | — | — | **0.48 ms / 0.49 ms** | — |
| `select slide 150` | 33 ms | — | — | **0.71 ms / 0.86 ms** | — |
| `scroll slide list` | 33 ms | — | — | **0.45 ms / 0.47 ms** | — |
| `typing in a text field` | 33 ms | — | — | **0.61 ms / 0.62 ms** | — |
| `type on canvas with spell check` | 33 ms | — | — | **0.76 ms / 0.78 ms** | — |
| `check spelling, open` | 250 ms | — | — | **48.74 ms** | Single-shot open |
| `check spelling, ignore step` | 33 ms | — | — | **10.39 ms / 10.55 ms** | 20 issues |
| `first frame` | 500 ms | — | — | **15.58 ms** | — |
| `visit every slide` | 33 ms | — | — | **0.50 ms / 0.77 ms** | rss `94 -> 95 MB` |
| `visit every slide, real pictures` | 33 ms | — | — | **3.30 ms / 6.11 ms** | rss `112 -> 136 MB`, 81 decoded pictures |

### Engine Undo Benchmarks (`PF-01`–`PF-03` & CLI `/usr/bin/time -l`)

The driver's `do_perf` only executes `coursecraft-ui-egui` `perf_tests`, so release wall-time and RSS measurements for `undo_perf_tests` (`PF-01`–`PF-03`), `coursecraft-cli run` under `/usr/bin/time -l` (plan §2.1 & §9 reviewer note), and the full `cargo xtask perf` big-course suite were **not** captured in `perf.log`.

Deterministic delta payload sizes and clone counts verified during `s9` review (`cargo test -p coursecraft-engine -j 4 undo_perf_tests -- --ignored --nocapture --test-threads=1`, debug build; `REVIEW.md`):

| Benchmark | Plan Baseline (200 steps) | M33 Budget | Measured Delta Size / Full Clones (S9 debug run) | Release Wall Time / RSS |
|---|---|---|---|---|
| `PF-01` (`perf300`, 200 edits + undo/redo) | +391.89 MB footprint (`2,006 KB/step`), 396.5 ms | ≤ 2.0 MB heap (`undo_estimated_bytes <= 512 KiB`), ≤ 120 ms | **181.6 KB** estimated delta, **0** full-course clones | _pending release run_ |
| `PF-02` (`rich300`, 200 edits + undo/redo) | +1,900.05 MB footprint (`9,728 KB/step`), 2,119.2 ms | ≤ 4.0 MB heap (`undo_estimated_bytes <= 2 MiB`), ≤ 400 ms | **387.5 KB** estimated delta, **0** full-course clones | _pending release run_ |
| `PF-03` (`perf300`, 200 no-op edits) | 353.8 ms (`1.70 ms/no-op`) | 0 B delta, ≤ 100 ms | **0 B** estimated delta, **0** full-course clones | _pending release run_ |

## Skipped Steps & Open Concerns

- **Skipped steps:** None (`SKIPPED` file does not exist; all 9 steps `S1`–`S9` completed, reviewed, and committed).
- **Open concerns from `REVIEW.md` and driver logs:**
  - **Shared `CARGO_TARGET_DIR` worktree fingerprinting (`REVIEW.md` S1; `p1-player.log`):** Because `wt-m33` lives inside the shared `CARGO_TARGET_DIR` (`.../target`), plain `cargo test -p coursecraft-engine` initially reused a stale `wt-m32` fingerprint (`32ef7b05c73202d1`) until cleaned/recompiled with a distinct fingerprint, and `p1-player.log` wrote `player-dist` under `.../target/wt-m32/target/player-dist`. Isolate `CARGO_TARGET_DIR` per worktree for future runs and rebuild the player from the release checkout before packaging.
  - **Pending release engine undo & CLI measurements (plan §9 reviewer note & §10.1):** Run the following release benchmarks before cutting a release to record final release wall-time and RSS numbers against plan §2.1:
    - `cargo test --release -p coursecraft-engine undo_perf_tests -- --ignored --nocapture --test-threads=1`
    - `coursecraft-cli run` under `/usr/bin/time -l` on `perf300` / `rich300`
    - `cargo xtask perf` (full editor + big-course suite)
  - **`undo / redo move` stretch target (`perf.log`):** Measured `8.73 ms` median / `9.03 ms` p95 — comfortably within the `33 ms` budget and faster than the plan's `9.52 ms / 9.85 ms` baseline, but above the plan's `≤ 6.5 ms / ≤ 7.5 ms` stretch target (likely host load variance during `perf.log` and UI layout/tessellation dominating the 30-cycle frame loop rather than engine undo).
  - **Resolved review note (`REVIEW.md` S2 → fixed in S6):** The duplicate-ID precedence difference between `SlideLoc::find` and `Course::slide_mut` noted in S2 was resolved in S6 via `SlideLoc::find_editable` and regression test `sc_03_single_slide_matches_course_slide_mut_precedence_with_duplicate_ids`.

## Owner Manual Verification Checklist (Desktop App Only — ~8 Minutes)

From `docs/testing/m33-plan.md` §10.2:

- [ ] **Everyday editing + Undo/Redo:** Open `examples/showcase/` (or a large course). Move an object with arrow keys, edit text inline, change a shape's fill and shadow, add a state item, delete an object, duplicate a slide, and resize the slide (`Slide size…`). Press `⌘Z` / `Ctrl+Z` repeatedly back to the start and `⇧⌘Z` / `Ctrl+Y` forward — verify every single step restores cleanly with zero glitches.
- [ ] **History window jump:** Open **Edit ▸ History…**, make 10 edits across slides and course settings (including a trigger and a glossary term), click back 6 steps in one click, then click forward 4 steps — verify canvas, inspector, and slide list stay perfectly in sync.
- [ ] **Glued connectors:** Draw two shapes and connect them with a glued connector. Drag one shape so the connector reroutes. Press `⌘Z` / `Ctrl+Z` — verify both the shape AND the connector snap back together in one undo step; press Redo — both move and reroute together.
- [ ] **Error rollback & no-ops:** Try a no-op (e.g. set an object's opacity to its current value, or sort an already-sorted glossary) — verify **Edit ▸ Undo** does NOT add a useless step and does not clear an existing Redo stack.
- [ ] **Large-course memory check:** Open a large multi-slide course (100+ slides) while watching Activity Monitor / Task Manager memory, nudge objects ~50 times, and verify app RAM stays virtually flat (climbing by < 1–2 MB rather than hundreds of MB).
