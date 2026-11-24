# M36 Refactor A Report

**Milestone:** M36 (Pure Refactor — Refactor A: R1, remaining R2, R5; R8 deferred)  
**Plan:** [`docs/testing/m36-plan.md`](m36-plan.md)  
**Scope:** Canonical media visitor in `coursecraft-course` and indexed `MediaStore` in `coursecraft-ui-egui` (`R1`, steps `S1`–`S3`), zero-copy streaming Open/Publish consolidation in `coursecraft-publish`, `coursecraft-control`, `coursecraft`, and `coursecraft-cli` (`R2`, steps `S4`–`S5`), and splitting `crates/engine/src/commands.rs` into domain submodules with single-row `Command` / `REGISTRY` metadata and 8 per-kind `interaction_insert` builders (`R5`, steps `S6`–`S11`). Phase 4 (`R8`, steps `S12`–`S14`: splitting `apps/coursecraft/src/main.rs`) was dropped by the owner mid-run to redo after the UI overhaul PR.

## What Shipped Per Step (`S1`–`S11`; `S12`–`S14` Deferred)

| Step | Summary | Review Verdict |
|---|---|---|
| **S1** | Added all 4 R1 golden parity tests (`R1-01`–`R1-04`) before migrating any walker, created the canonical media visitor in `crates/course/src/media.rs` (`MediaScope`, `MediaRole` with `is_player_image`/`is_player_fetched`, `SlideContainer`, `MediaOwner`, `MediaRef`, `visit_course_media`, `visit_slide_media`, `visit_object_media`), re-exported it from `crates/course/src/lib.rs`, and rewrote `media_references` as a thin adapter over `visit_course_media` (removing `walk_*_media_references`). | `OK` |
| **S2** | Migrated the remaining 3 read-only media walkers (`crates/publish/src/lib.rs`, `crates/ui-egui/src/lib.rs`, and `apps/coursecraft-player/src/media.rs`) to thin adapters over `coursecraft_course::visit_*_media` (−565/+109 lines), preserving master-layer skipping, `PublishError::UnsafePath` empty-`src` rules, 5-bucket error ordering, and first-seen web-folder entries, with zero golden test edits. | `OK` |
| **S3** | Added indexed `MediaStore` (`crates/ui-egui/src/media_store.rs`, read-only `Deref<Target = [(String, Arc<[u8]>)]>`, first-seen index map, saturating `total_bytes`, wrapping `generation`), replaced `Editor.media` and deleted all 4 M34 shadow-index fields and 6 staleness helpers, added `Editor::add_raw_media`, migrated all 10 `MemoryMediaSource::from_parts` sites to `from_store`, replaced the 3 linear scans in `media_library.rs` with `contains`, and added `R1-05` and `R1-06`. | `OK` |
| **S4** | Added `ZipperCore<F>` + `SharedPackageZipper` in `crates/publish/src/lib.rs` (preserving the exact `!is_precompressed` whole-write deflate boundary), switched `apps/coursecraft/src/main.rs` web publish (`WebPublishJob::Zipping`, `step_web_publish`) to `package_files_shared_from` + `SharedPackageZipper` with zero `.to_vec()` copies, migrated the 3 remaining disk `.ccraft` openers (`crates/control/src/host.rs`, `apps/coursecraft/src/template_library.rs`, `apps/coursecraft-cli/tests/big_course.rs`) to 256 KiB `BufReader` + `open_project_from{,_with_limits}`, updated `big_course.rs` publish to `SharedPackageInput` + `Arc<[u8]>` + 256 KiB `BufWriter`, and added `R2-01` and `R2-02`. | `OK` |
| **S5** | Consolidated `coursecraft-publish` onto one zero-copy API: upgraded `PackageFile::bytes`, `PackageInput::media`, `WebFolderFiles`, and `PackageZipper` to `Arc<[u8]>`, made `build_package_entries` non-generic with `Arc::clone(&player_file.bytes)` (eliminating `player_files` byte clones), removed all duplicate `SharedPackage*`/`SharedWebFolderFiles`/`ZipperCore`/`build_package_shared`/`zip_shared_package*` symbols and `apps/coursecraft/src/publish.rs::package_files_from`, updated all workspace call sites, and strengthened `R2-03` (`Arc::ptr_eq` across media, web folders, and all `player_files`). | `OK` |
| **S6** | Added `R5-01` (186-row `REGISTRY` order/metadata/hash golden lock) and `R5-02` (8-kind `interaction.insert` JSON/return golden lock) in `crates/engine/src/tests.rs`, converted `crates/engine/src/commands.rs` to `crates/engine/src/commands/mod.rs`, and mechanically extracted 5 domain submodules (`course.rs`, `layer.rs`, `text.rs`, `table.rs`, `trigger.rs`) verbatim. | `OK` |
| **S7** | Mechanically extracted `crates/engine/src/commands/slide.rs` (scene, slide, master, bank, draw, layout, outline, template, copy/remap, results, certificate) and `crates/engine/src/commands/video_cues.rs` (7 video/slide cue commands + helpers) verbatim from `commands/mod.rs`, updating 39 `REGISTRY` `run:` pointers. | `OK` |
| **S8** | Mechanically extracted `crates/engine/src/commands/object.rs` (52 items, 34 commands) and `crates/engine/src/commands/object_states.rs` (27 items, 16 commands) verbatim from `commands/mod.rs`, updating 50 `REGISTRY` `run:` pointers while keeping sibling `super::` imports intact. | `OK` |
| **S9** | Mechanically extracted `crates/engine/src/commands/question.rs` (54 items, 26 commands) and `crates/engine/src/commands/interaction.rs` (10 items, `interaction.insert` verbatim) from `commands/mod.rs`, completing the handler split so all 186 `REGISTRY` `run:` pointers target domain modules. | `OK` |
| **S10** | Folded `pub(crate) scope: UndoScope` and `pub(crate) slide_validation: bool` directly into `Command` (`crates/engine/src/lib.rs`) and all 186 `REGISTRY` rows (`crates/engine/src/commands/mod.rs`: 25 `CourseHeader`, 115 `SingleSlide`, 46 `FullCourse`; 83 `slide_validation: true`, 103 `false`), deleted the separate `undo_scope` and `can_use_slide_validation` match tables, tightened `R5-03` with the `!cmd.mutates` invariant, and updated `docs/contributing.md` §"Add a command" while leaving `docs/cli.md` untouched. | `OK` |
| **S11** | Split `interaction_insert` in `crates/engine/src/commands/interaction.rs` into a 127-line dispatcher with `InteractionCtx<'a>` and 8 per-kind builders (`insert_tabs` 107, `insert_accordion` 133, `insert_flip_cards` 91, `insert_hotspot_reveal` 127, `insert_process_steps` 251, `insert_timeline` 153, `insert_reveal_grid` 154, `insert_scenario_choice` 226 lines — all ≤ 260 lines), removing `#[allow(clippy::cognitive_complexity)]` and preserving `R5-02` byte-for-byte. | `OK` |
| **S12–S14** | _Deferred (R8):_ Splitting `apps/coursecraft/src/main.rs` (`control_adapter.rs`, `web.rs`, `desktop.rs`, `app.rs`), deduplicating web `AsyncFileDialog` pickers, and adding `R8-01` were dropped by the owner during the run (`progress.txt`) to redo after the UI overhaul PR. | _Dropped by owner_ |

## Test Coverage (`R1-01`–`R1-06`, `R2-01`–`R2-03`, `R5-01`–`R5-04`)

11 net new passing tests added across the workspace (`3,932` → `3,943` passing tests in `cargo xtask ci`; 12 new test functions added, 1 shadow-index test replaced by `R1-06`, 1 zero-copy test upgraded to `R2-03`):

| ID | Step | Crate / File | Coverage |
|---|---|---|---|
| `R1-01` | S1 | `coursecraft-course` (`tests.rs`) | `r1_01_media_references_golden_and_visitors` and `r1_01_visit_media_scopes_and_roles`: locks exact `media_references()` ordering, `place` strings, `WebFolder` trailing `/`, empty-`src` filtering, `Publishable` vs `AllIncludingTemplates` scopes, `visit_slide_media`/`visit_object_media`, and `MediaRole` helpers |
| `R1-02` | S1 | `coursecraft-publish` (`tests.rs`) | `r1_02_publish_media_walker_golden_and_errors`: golden byte-identical zip comparison and error-selection parity across all publishable media slots, web folders, template exclusion, master-layer skipping, and empty-`src` `UnsafePath` vs optional-field skipping |
| `R1-03` | S1 | `coursecraft-ui-egui` (`tests.rs`) | `r1_03_ui_media_walkers_golden`: golden parity for `referenced_media_srcs`, `web_folder_prefixes`, `Editor::used_media`, `Editor::slide_media`, and `Editor::missing_media` across all media kinds and web folders |
| `R1-04` | S1 | `coursecraft-player` (`tests.rs`) | `r1_04_player_slide_media_walkers_golden`: golden parity for `slide_image_srcs` and `slide_media_srcs` across images, picture fills, posters, Lottie, audio/video src & captions, states, state extras, layers, and masters |
| `R1-05` | S3 | `coursecraft-ui-egui` (`media_store.rs`) | `r1_05_media_store_invariants`: deterministic order, first-seen duplicate-key semantics, O(1) `get`/`contains`, saturating `total_bytes`, wrapping `generation`, `push`/`replace`/`truncate`/`retain`/`set_all`/`clear` |
| `R1-06` | S3 | `coursecraft-ui-egui` (`guard_tests.rs`) | `r1_06_media_store_stays_exact_across_all_mutations` (replaced M34's `s11_media_index_detects_same_length_replacement`): verifies `Editor.media` (`MediaStore`), `total_bytes`, and `MemoryMediaSource::from_store` stay exact across all mutations |
| `R2-01` | S4, S5 | `coursecraft-publish` (`tests.rs`) | `r2_01_shared_package_zipper_matches_zip_package_across_chunk_sizes`: proves unified `PackageZipper` matches `zip_package` byte-for-byte across chunk sizes `1, 17, 1024, 65536, 262144`, empty input, and post-`Done` error handling |
| `R2-02` | S4 | `coursecraft-control` (`tests.rs`), `coursecraft` (`template_library.rs`) | `r2_02_headless_open_streaming_ccraft_and_errors` and `r2_02_load_template_streaming_round_trip_and_corrupt`: verifies streaming `BufReader` + `open_project_from` media round-trip and missing/corrupt archive errors |
| `R2-03` | S5 | `coursecraft-publish` (`tests.rs`) | `r2_03_zero_copy_publish_shares_media_web_and_player_arcs` (upgraded `g04`): proves `Arc::ptr_eq` across course media, web-folder files, and all `player_files` (`index.html`, `app.js`, `app_bg.wasm`), compact `course.json` round-trip, and byte-identical `zip_package`/`zip_package_to` |
| `R5-01` | S6, S10 | `coursecraft-engine` (`tests.rs`) | `r5_01_registry_golden_lock`: locks all 186 `REGISTRY` rows in exact order for `(id, mutates, scope, can_use_slide_validation)` plus byte length (`23_881`) and FNV-1a 64-bit hash over the full metadata table |
| `R5-02` | S6 | `coursecraft-engine` (`tests.rs`) | `r5_02_interaction_insert_golden_all_eight_kinds`: locks exact return JSON, `Course::to_json()` lengths/hashes, object/layer/variable/trigger counts, and first/last IDs for all 8 `interaction.insert` kinds (passed untouched after S9 and S11) |
| `R5-03` | S10 | `coursecraft-engine` (`m27_all_commands_undo_tests.rs`) | `u05_scoped_validation_matches_full_validation_across_all_commands` updated to `cmd.can_use_slide_validation()` and enforcing `UndoScope::FullCourse && !cmd.can_use_slide_validation()` for every `!cmd.mutates` command |
| `R5-04` | S10 | `coursecraft-cli` (`tests.rs`), `coursecraft-mcp` (`tests.rs`) | Existing CLI `docs/cli.md` sync and `commands` tests (`58` unit + `63` integration) and MCP tool-list tests (`10`) verified green with zero `docs/cli.md` drift |
| `R8-01` | S14 | `coursecraft` (`u16_shell_tests.rs`) | _Deferred with R8 (`S12`–`S14` dropped by owner)_; all `196` existing `coursecraft` tests pass |

## Driver Gates

- **`cargo xtask ci`:**
  - `g1` (after S2): passed on attempt 1 (`3,937 passed; 0 failed; 16 ignored`, `layers: ok`, all 17 WASM crates checked).
  - `g2` (after S5): passed on attempt 1 (`3,941 passed; 0 failed; 16 ignored`, `layers: ok`, all 17 WASM crates checked).
  - `g3` (after S9): passed on attempt 1 (`3,943 passed; 0 failed; 16 ignored`, `layers: ok`, all 17 WASM crates checked).
  - `g4` (after S11): passed on attempt 1 (`3,943 passed; 0 failed; 16 ignored`, `layers: ok`, all 17 WASM crates checked).
  - `g5` (final): passed on attempt 1 (`3,943 passed; 0 failed; 16 ignored`, `fmt --all --check`, `clippy --workspace --all-targets -- -D warnings`, `layers: ok`, all 17 WASM crates checked — including `coursecraft-course` `309`+`2`, `coursecraft-publish` `49`+`1`, `coursecraft-control` `17`, `coursecraft-engine` `376`+`1` (+`3` ignored), `coursecraft-ui-egui` `1,315` (+`11` ignored), `coursecraft-player` `55`, `coursecraft` `196`, `coursecraft-cli` `58`+`63` (+`1` ignored), `coursecraft-mcp` `10`).
- **`cargo xtask build-player`:**
  - `p1` (after S2 / `g1`): passed (`trunk build --release`, `wasm-release` finished in `1m 13s`).
  - `p2` (after `g5`): passed (`trunk build --release`, `wasm-release` finished in `0.23s`).
- **Release editor perf suite (`perf.log` — `coursecraft-ui-egui::perf_tests`, `--release --ignored`):** passed (`9 passed; 0 failed; 0 ignored; 1291 filtered out` in `2.38s`).

## Measured Performance & File Sizes vs. Baseline

### File Size Snapshot vs. Plan Baseline (`docs/testing/m36-plan.md` §2.1)

| File | Plan Baseline (lines) | Measured After M36 (lines) | Notes |
|---|---:|---:|---|
| `crates/course/src/lib.rs` | 616 | **507** | −109 lines; new canonical `crates/course/src/media.rs` (`368` lines) |
| `crates/publish/src/lib.rs` | 1,180 | **900** | −280 lines (shared visitor + single zero-copy API) |
| `crates/ui-egui/src/lib.rs` | 5,919 | **5,669** | −250 lines (shared visitor + shadow index removed); new `media_store.rs` (`275` lines) |
| `apps/coursecraft-player/src/media.rs` | 485 | **389** | −96 lines (shared visitor) |
| `apps/coursecraft/src/publish.rs` | 310 | **290** | −20 lines (`package_files_from` removed) |
| `crates/engine/src/commands.rs` | 13,703 | **Split into 14 files** | `mod.rs` (`2,225`), `slide.rs` (`2,487`), `object.rs` (`2,266`), `question.rs` (`1,799`), `interaction.rs` (`1,721`, 8 builders all ≤ 260 lines), `object_states.rs` (`1,036`), `course.rs` (`915`), `csv_import.rs` (`598`), `video_cues.rs` (`484`), `text.rs` (`434`), `table.rs` (`408`), `styles.rs` (`390`), `layer.rs` (`288`), `trigger.rs` (`83`) |
| `crates/engine/src/lib.rs` | 931 | **938** | +7 lines (`Command.scope` and `Command.slide_validation` fields) |
| `apps/coursecraft/src/main.rs` | 4,315 | **4,315** | Unchanged (`R8` deferred) |

### Release Editor Perf (`perf.log`) vs. Baseline

`docs/testing/m36-plan.md` requires no regressions against existing release budgets in [`docs/performance.md`](../performance.md). Below are the exact measured numbers from `/Users/aditya/Documents/Claude Projects/ScormCraft/target/m36/perf.log` compared against those budgets and the immediately preceding M35 measured baseline in [`docs/testing/m35-report.md`](m35-report.md):

| Stage (300-slide course, 1280×900) | Budget | Pre-M36 (`m35-report.md`, median / p95) | Measured After M36 (`perf.log`, median / p95) | Notes |
|---|---:|---:|---:|---|
| `idle` | 16 ms | 0.40 ms / 0.41 ms | **0.40 ms / 0.41 ms** | Flat; well under budget |
| `select slide 150` | 33 ms | 0.62 ms / 0.78 ms | **0.63 ms / 0.78 ms** | Flat; well under budget |
| `scroll slide list` | 33 ms | 0.36 ms / 0.37 ms | **0.36 ms / 0.38 ms** | Flat; well under budget |
| `typing in a text field` | 33 ms | 0.53 ms / 0.53 ms | **0.53 ms / 0.54 ms** | Flat; well under budget |
| `type on canvas with spell check` | 33 ms | 0.71 ms / 0.72 ms | **0.71 ms / 0.72 ms** | Flat; well under budget |
| `check spelling, open` | 250 ms | 47.00 ms | **30.09 ms** | Well under 250 ms budget |
| `check spelling, ignore step` (20 issues) | 33 ms | 0.44 ms / 0.59 ms | **0.43 ms / 0.57 ms** | Flat; well under budget |
| `insert interaction + undo` (8 kinds) | 100 ms | 1.15 ms / 3.12 ms | **1.17 ms / 1.65 ms** | Well under 100 ms budget |
| `undo / redo move` (30 cycles) | 33 ms | 2.94 ms / 3.27 ms | **2.54 ms / 2.86 ms** | Faster than pre-M36; well under budget |
| `first frame` | 500 ms | 8.76 ms | **8.32 ms** | Well under 500 ms budget |
| `visit every slide` | 33 ms | 0.43 ms / 0.70 ms | **0.43 ms / 0.69 ms** | `rss 263 -> 265 MB` |
| `visit every slide, real pictures` | 33 ms | 0.44 ms / 0.71 ms | **0.43 ms / 0.70 ms** | `rss 269 -> 272 MB, media 0.2 MiB, 1 decoded pictures` |
| `low-end lite 300 slides` | 33 ms | 13.92 ms / 34.31 ms | **14.23 ms / 34.56 ms** | `media 4.4 MiB, undo 0.0 KiB` (median well under 33 ms; p95 1.56 ms over print budget, flat vs. pre-M36) |

No editor hot-path regression was introduced by M36.

### Plan §7 Benchmarks Not Executed by Driver

The driver's `do_perf` step ran only `coursecraft-ui-egui::perf_tests` (`perf.log`). Two additional release benchmarks listed in `docs/testing/m36-plan.md` §7 remain unrun by the driver:

- `cargo test --release -p coursecraft-engine undo_perf_tests -- --ignored --nocapture --test-threads=1` (`PF-01`–`PF-03`, verifying undo/redo memory and timing budgets after R5)
- `cargo test --release -p coursecraft-cli --test big_course -- --ignored --nocapture` (verifying R2's streaming `open_project_from_with_limits` and zero-copy `PackageInput` / `zip_package_to` on the 300-slide / 1.3 GB course)

## Skipped Steps & Open Concerns

- **Skipped / deferred steps:** No `SKIPPED` file exists in `/Users/aditya/Documents/Claude Projects/ScormCraft/target/m36/`. All 11 executed steps (`S1`–`S11`) completed, passed review on the first pass (`OK`), and committed cleanly. Steps **`S12`–`S14` (R8 — splitting `apps/coursecraft/src/main.rs` into `main.rs`/`app.rs`/`desktop.rs`/`web.rs`/`control_adapter.rs`, deduplicating web file pickers, and adding `R8-01`)** were intentionally dropped by the owner during the run (`progress.txt`: *"18:36:59 owner: S12-S14 (R8) dropped for now, redo after the UI overhaul PR"*) and remain to be redone later.
- **Open concerns from `REVIEW.md`:** None — all 11 reviewed steps (`S1`–`S11`) received `Verdict: OK` with zero blocking or non-blocking follow-up fixes required. Two minor implementation notes were recorded as explicitly harmless:
  - **S1:** `SlideContainer`, `MediaOwner`, and `MediaRef` derive `PartialEq` (not `Eq`) because borrowed model types hold `f32`/`f64`.
  - **S2:** `Video` emits `Caption` before `VideoPoster` in `apps/coursecraft-player/src/media.rs::slide_media_srcs` (harmless because callers insert results into a `HashSet`).
- **Pending release checks:** Run the two non-UI release benchmarks from `docs/testing/m36-plan.md` §7 (`coursecraft-engine::undo_perf_tests` and `coursecraft-cli --test big_course`) before tagging if full §7 sign-off is desired.

## Owner Verification Checklist (~5 Minutes)

From `docs/testing/m36-plan.md` §8 (pure refactor smoke check; note R8's web file-picker deduplication was deferred, while R2's zero-copy chunked web publish shipped in `S4`–`S5`):

- [ ] **Open, edit, undo, save & reopen (Desktop):** Open `examples/showcase/` in the desktop app; insert one interaction (e.g. Tabs), edit an object, press `⌘Z` / `Ctrl+Z` and redo, inspect **Media Library** (check usage counts and missing-media badge), save as `.ccraft`, and reopen — confirm identical look and behaviour.
- [ ] **Publish (Desktop):** Publish `examples/showcase/` to SCORM 1.2 and Web (`File ▸ Publish`), confirm the zip builds cleanly and opens in the browser with all images, fonts, and audio intact.
- [ ] **Web editor smoke check:** Run `trunk serve` in `apps/coursecraft`, open a `.ccraft` file, import an image, and publish to Web — confirm the file pickers work and the chunked zero-copy publish progress bar completes cleanly with no browser console errors.
- [ ] **CLI & docs parity:** Confirm `docs/cli.md` has zero diff and `coursecraft-cli commands` output is unchanged.
