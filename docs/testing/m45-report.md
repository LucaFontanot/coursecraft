# M45 Report — Quick Wins & Follow-Up Fixes

**Milestone:** M45 (Quick Wins & Follow-Up Fixes, pending release)  
**Plan:** [`docs/testing/m45-plan.md`](m45-plan.md)  
**Scope:** Six one-step quick wins from the plan (`S1`–`S6`) plus two owner-reported follow-up bug fixes (`S7`–`S8`): wiring **Insert › Media › Caption** to the Callouts shape category (`S1`), adding a pure HTML/CSS web player loading indicator with `prefers-reduced-motion` and startup-error handling (`S2`), scoping spell check to English courses with a localized note for non-English `course.language` tags (`S3`), registering `.ccraft` file associations on Windows and Linux plus launch-argument hardening (`S4`), adding the Intel Mac (`x86_64-apple-darwin`) `.dmg` build and `-size 200m` image sizing fix to the release workflow (`S5`), syncing `docs/ui/feature-parity.md` status rows and summary totals with the codebase (`S6`), remembering and restoring each object's/state's previous fill colour and fill style when toggling Fill (`S7`), and cropping audio/video playback at a shortened timeline bar's visible window end while keeping timeline audio waveforms at real time scale (`S8`).

## What Shipped Per Step (`S1`–`S8`)

| Step | Summary | Review Verdict |
|---|---|---|
| **S1** | Added `only_category: Option<ShapeCategory>` to `show_shape_gallery_inner` and exposed `show_shape_gallery_category` in `crates/ui-egui/src/shape_gallery.rs` (skipping Draw tools and non-matching categories when filtered, and clearing `editor.ui.shape_gallery_category` on close/pick/reset, including Insert › Shape in `menu.rs`); added `UiAction::Caption` and `UiMenu::Callouts` in `crates/ui-egui/src/ribbon/spec.rs` and wired `insert.caption` as a split control through `crates/ui-egui/src/ribbon/mod.rs` (`ui_available`, `run_ui`, `ui_selected`, `menu_only`, `shapes_menu_category`) with `tl!`-localized labels and help text; updated `docs/ui/ribbon.md`, `docs/ui/feature-parity.md`, and `CHANGELOG.md`. | `OK` |
| **S2** | Added a pure HTML/CSS centred loading overlay (`#coursecraft_loading`, `role="status"`, `aria-live="polite"`, 32 px spinner, `"Loading course…"` text, and `@media (prefers-reduced-motion: reduce)` hiding the spinner, no `<script>`) to `apps/coursecraft-player/index.html`; added `LOADING_*` constants, `remove_loading_overlay()`, and `show_loading_error()` (handling missing `#coursecraft_canvas` and `WebRunner::start` failures) in `apps/coursecraft-player/src/main.rs`; removed the overlay once on the first wasm `eframe::App::ui` frame in `apps/coursecraft-player/src/app.rs` (published courses copy `index.html` from `player-dist` via `crates/publish`, with no duplicate template in `crates/export`); updated `CHANGELOG.md`. | `OK` |
| **S3** | Added `unsupported_course_language(Option<&str>) -> Option<&str>` in `crates/ui-egui/src/spell_check.rs` (trims, splits on `'-'`/`'_'`, and accepts `None`, empty/whitespace, and `en` primary subtags case-insensitively); gated `Editor::spell_words` before cache lookup (covering canvas inline edit and properties inspector) and `collect_issues` so non-English courses skip as-you-type squiggles and issue collection; rendered the localized note (`"Spelling is checked in English only. This course's language is {lang}."` via `crate::tl!` and `craft_i18n::fmt`) in `window_body` before the empty-issues branch; updated `CHANGELOG.md`. | `OK` |
| **S4** | Added `ChangesAssociations=yes` and `[Registry]` under `Root: HKA` (`Software\Classes\.ccraft`, `OpenWithProgids`, `CourseCraft.Project`, `DefaultIcon`, `shell\open\command` with Inno-escaped `"""{app}\coursecraft.exe"" ""%1"""`, and `uninsdeletekey`/`uninsdeletevalue`) in `packaging/windows/coursecraft.iss`; created `packaging/linux/coursecraft-mime.xml` (`application/x-coursecraft-project`, `*.ccraft`), added `MimeType=application/x-coursecraft-project;` to `packaging/linux/coursecraft.desktop`, updated `packaging/linux/README.md`, and updated `.github/workflows/release.yml` Linux `.tar.gz` and AppImage steps to install the desktop entry and MIME XML; extracted `parse_launch_args` using `OsString` / `std::env::args_os()` in `apps/coursecraft/src/main.rs` to preserve `.ccraft` paths with spaces or non-UTF-8 bytes; updated `CHANGELOG.md`. | `OK` |
| **S5** | Added optional 4th `arch` argument (`arch=${4:-$(uname -m)}`) and `-size 200m` on `hdiutil create` (preventing undersized-image failures) in `scripts/package-macos.sh`; updated `.github/workflows/release.yml` to add `x86_64-apple-darwin` to `dtolnay/rust-toolchain@1.99` on macOS, build `coursecraft` for `x86_64-apple-darwin` with `MACOSX_DEPLOYMENT_TARGET: "11.0"`, and package both Apple Silicon and Intel `.dmg` files into `dist` (covered by `SHA256SUMS.txt`); updated `README.md`, `docs/maintaining.md`, and `CHANGELOG.md` (verified `docs/release-checklist.md` had no manual Intel step and left it untouched). | `OK` |
| **S6** | Docs-only sync of `docs/ui/feature-parity.md` against the codebase: updated **Graded question kinds** to `🟡` (`M41`, word bank and numeric not yet), **Theme fonts (heading + body)** to `🟡` (`M39`, single `Theme.font` picker), clarified **Spelling** (`✅`, English dictionary; skips non-English courses with a note), verified **Caption / callout shapes** (`✅`), and recounted all 12 tables and `## Summary` (`Text`: `12 \| 4 \| 8 \| 0`, `Shapes, pictures and buttons`: `16 \| 4 \| 9 \| 0`, `Questions and quizzes`: `8 \| 4 \| 3 \| 0`, **Total**: `110 \| 39 \| 58 \| 12`). | `OK` |
| **S7** | Added `RememberedFill`, `PropertiesState::remembered_fills` (keyed by `(slide_id, object_id, Option<state_name>)`, preserved across `sync`, cleared by `Editor::replace_course`), and `remember_fill` / `restore_fill` helpers in `crates/ui-egui/src/properties.rs`; updated all three Fill checkboxes (`shape_fill_rows` shared by Format inspector and Shape Style panel, Button Fill, and multi-selection Fill) to remember non-transparent colour and `fill_style` on untick and restore them per-object/per-state on re-tick (falling back to `#D0D7DE` / `#2F6FEB` when no prior fill exists), dispatching exclusively through `object.update` / `object.patch_state`; updated `CHANGELOG.md`. | `OK` |
| **S8** | Added `object_visible_end_ms` in `crates/logic/src/run.rs`, gated `autoplay` and `advance` start on `object_in_timing_window`, stopped playing `Audio`/`Video` objects when crossing their visible window end (clearing `media_clocks` and cue state and emitting `Effect::Media { op: MediaOp::Stop }` without firing `media_ends`), and documented the rule in `docs/spec/triggers.md`; added `trim` support, half-open `playback_includes`, `track_file_offset_ms`, and `effective_media_end_ms` in `crates/ui-egui/src/timeline_playback.rs`; cached `Editor::audio_durations` alongside `audio_waveforms` in `crates/ui-egui/src/lib.rs` and added `audio_waveform_window` / `audio_waveform_for_object` in `crates/ui-egui/src/timeline.rs` to draw waveforms at real time scale; updated `CHANGELOG.md`. Review fixed an unclamped `offset` in `flush_seeks` when `playing == true` for trimmed videos seeking past `trim.end_ms` and added a regression test. | `FIXED` |

## Test Coverage (`M45-01`–`M45-21`, 21 New Passing Tests)

| ID | Step | Crate / File | Coverage |
|---|---|---|---|
| `M45-01` | S1 | `coursecraft-ui-egui` (`ribbon/spec_tests.rs`) | `insert_caption_spec_item_is_enabled`: verifies `insert.caption` is enabled in the ribbon spec and no longer `NotYet` |
| `M45-02` | S1 | `coursecraft-ui-egui` (`ribbon/spec_tests.rs`) | `insert_caption_opens_callout_shapes_and_arms_placement`: clicks Insert › Media › Caption, verifies the Callouts category menu opens without Draw tools or non-callout shapes, and picking a callout arms placement |
| `M45-03` | S2 | `coursecraft-player` (`tests.rs`) | `web_player_index_html_has_loading_indicator_and_reduced_motion`: verifies `#coursecraft_loading`, `#coursecraft_loading_spinner`, `#coursecraft_loading_text`, `"Loading course…"`, `prefers-reduced-motion`, and zero `<script>` tags in `index.html` |
| `M45-04` | S3 | `coursecraft-ui-egui` (`spell_check_tests.rs`) | `unsupported_course_language_accepts_none_and_english_tags`: unit-tests `unsupported_course_language` across `None`, blank strings, `en`, `en-GB`, `EN_us`, and non-English tags |
| `M45-05` | S3 | `coursecraft-ui-egui` (`spell_check_tests.rs`) | `spell_check_checks_when_course_has_no_language_or_en_gb`: confirms as-you-type words and Check Spelling issues continue working when `course.language` is `None` or `en-GB` |
| `M45-06` | S3 | `coursecraft-ui-egui` (`spell_check_tests.rs`) | `spell_check_skips_squiggles_and_shows_note_for_fr_ca`: confirms `fr-CA` suppresses squiggles/issues and renders the localized note in the Check Spelling window (including `Lang::PSEUDO` verification) |
| `M45-07` | S4 | `coursecraft` (`tests.rs`) | `parse_launch_args_handles_ccraft_path_and_spaces`: verifies `parse_launch_args` skips argv[0]/empty args, handles `--blank`, and preserves `.ccraft` paths with spaces |
| `M45-08` | S4 | `coursecraft` (`tests.rs`) | `startup_opens_ccraft_project_with_spaces_in_path`: opens a real `.ccraft` project whose path contains spaces through the startup path |
| `M45-09` | S4 | `coursecraft-cli` (`tests/bughunt_m26_cli_and_packaging.rs`) | `ccraft_file_association_configured_for_windows_and_linux`: static verification of `coursecraft.iss`, `coursecraft-mime.xml`, `coursecraft.desktop`, and `.github/workflows/release.yml` Linux packaging steps |
| `M45-10` | S5 | `coursecraft-cli` (`tests/bughunt_m26_cli_and_packaging.rs`) | `release_workflow_builds_intel_macos_dmg_and_sizes_image`: static verification of `.github/workflows/release.yml` (`x86_64-apple-darwin`, `MACOSX_DEPLOYMENT_TARGET: "11.0"`, both `.dmg` artifacts) and `scripts/package-macos.sh` (4th `arch` argument and `-size 200m`) |
| `M45-11` | S7 | `coursecraft-ui-egui` (`f22_fill_outline_dash_tests.rs`) | `shape_fill_toggle_restores_previous_colour_and_gradient`: unticking and re-ticking Fill on a shape restores its previous colour and `FillStyle::Linear` gradient and stays clean across undo/redo |
| `M45-12` | S7 | `coursecraft-ui-egui` (`f22_fill_outline_dash_tests.rs`) | `button_fill_toggle_restores_previous_colour`: unticking and re-ticking Fill on a button restores its previous colour instead of `#2F6FEB` |
| `M45-13` | S7 | `coursecraft-ui-egui` (`f22_fill_outline_dash_tests.rs`) | `multi_select_fill_toggle_restores_each_objects_previous_colour`: multi-select Fill untick/re-tick restores each selected shape's own remembered colour |
| `M45-14` | S7 | `coursecraft-ui-egui` (`f22_fill_outline_dash_tests.rs`) | `state_override_fill_toggle_restores_previous_colour`: unticking and re-ticking Fill while editing a custom state restores that state's previous override colour |
| `M45-15` | S7 | `coursecraft-ui-egui` (`f22_fill_outline_dash_tests.rs`) | `shape_without_prior_fill_uses_default_when_ticked`: ticking Fill on a shape created with `#00000000` falls back to `#D0D7DE` |
| `M45-16` | S8 | `coursecraft-logic` (`tests.rs`) | `audio_stops_when_visible_window_ends`: verifies audio stops at `timing.end_ms` without firing `media_ends`, exit-animated audio stops at `duration_ms`, and untrimmed audio without exit keeps playing |
| `M45-17` | S8 | `coursecraft-logic` (`tests.rs`) | `audio_mid_bar_seek_uses_trim_start_offset_and_after_bar_plays_nothing`: verifies mid-bar seeks map to `trim.start_ms + (pos - start)` and seeks at/after `end_ms` stop playing audio and start nothing |
| `M45-18` | S8 | `coursecraft-ui-egui` (`timeline_playback.rs`) | `audio_bar_cropped_to_5s_stops_at_bar_end_and_ignores_play_after_end`: verifies editor timeline playback pauses cropped audio at `5000 ms`, never restarts past bar end, and ignores Play starting at/after `5000 ms` |
| `M45-19` | S8 | `coursecraft-ui-egui` (`timeline_playback.rs`) | `trimmed_audio_mid_bar_play_seeks_from_trim_start`: verifies mid-bar Play seeks to `trim.start_ms + offset` and maps reported file `position_ms` back to timeline playhead |
| `M45-20` | S8 | `coursecraft-ui-egui` (`timeline_playback.rs`) | `seeking_while_playing_past_video_trim_end_clamps_last_frame_seek`: review regression test confirming playing seeks past a trimmed video's `trim.end_ms` clamp to `trim.end_ms` |
| `M45-21` | S8 | `coursecraft-ui-egui` (`timeline.rs`) | `audio_waveform_window_keeps_real_time_scale_for_shorter_trimmed_and_longer_bars`: verifies real-time-scale peak slicing and bar fill fractions for shorter, trimmed, longer, and empty/past-end bars |

*(Step `S6` is docs-only; `cargo test -p coursecraft-course` passed with `320` unit + `2` integration tests and all 12 tables plus summary counts in `docs/ui/feature-parity.md` were recounted programmatically.)*

## Driver Gates

- **`cargo xtask ci`:**
  - `g1` (after `S3` — `g1-1.log`): **passed on attempt 1**:
    - `cargo fmt --all --check`: passed.
    - `cargo clippy --workspace --all-targets -- -D warnings`: passed (0 warnings, `1m 30s`).
    - `cargo test --workspace`: **4,158 passed; 0 failed; 17 ignored** (`2m 06s`).
    - `cargo xtask layers`: `layers: ok`.
    - `cargo check --target wasm32-unknown-unknown` across all 17 WASM crates: passed (`47.37s`).
  - `g2` (after `S6` — `g2-1.log`): **passed on attempt 1**:
    - `cargo fmt --all --check`: passed.
    - `cargo clippy --workspace --all-targets -- -D warnings`: passed (0 warnings, `30.02s`).
    - `cargo test --workspace`: **4,162 passed; 0 failed; 17 ignored** (`34.27s`).
    - `cargo xtask layers`: `layers: ok`.
    - `cargo check --target wasm32-unknown-unknown` across all 17 WASM crates: passed (`3.37s`).
  - `g3` (final, after `S8` + review fix — `g3-1.log`): **passed on attempt 1**:
    - `cargo fmt --all --check`: passed.
    - `cargo clippy --workspace --all-targets -- -D warnings`: passed (0 warnings, `40.37s`).
    - `cargo test --workspace`: **4,173 passed; 0 failed; 17 ignored** (`58.34s`), including `coursecraft-ui-egui` `1,473` (+`11` ignored, +`1` ignored doc-test), `coursecraft-logic` `272`+`11`, `coursecraft-player` `58`, `coursecraft` `189`, `coursecraft-cli` `59`+`66` (+`1` ignored), `coursecraft-course` `320`+`2`, `coursecraft-engine` `397`+`1` (+`3` ignored), `coursecraft-render` `488`+`13` (+`1` ignored), `coursecraft-lms` `96`+`27`.
    - `cargo xtask layers`: `layers: ok`.
    - `cargo check --target wasm32-unknown-unknown` across all 17 WASM crates: passed (`18.43s`).
- **`cargo xtask build-player`:**
  - `p1` (after `S2` — `p1-player.log`): passed (`wasm-release` finished in `1m 14s`, written to `target/wt-m45/target/player-dist`).
  - `p2` (after `g2` — `p2-player.log`): passed (`wasm-release` finished in `0.30s`, written to `target/wt-m45/target/player-dist`).
  - `p3` (final, after `g3` — `p3-player.log`): passed (`wasm-release` finished in `1m 09s`, written to `target/wt-m45/target/player-dist`).
- **Release editor perf suite (`perf.log` — `coursecraft-ui-egui::perf_tests`, `--release --ignored`):** passed (`9 passed; 0 failed; 0 ignored; 1440 filtered out` in `2.46s`; run after `S6`/`p2`, prior to follow-up steps `S7`–`S8`).

## Measured Performance (`perf.log`) vs. Baseline

`docs/testing/m45-plan.md` does not define a separate performance baseline table. Below are the exact measured numbers from `/Users/aditya/Documents/Claude Projects/ScormCraft/target/m45/perf.log` (recorded after `S6`/`p2`; not re-run by the driver after `S7`–`S8`) compared against the budgets and recorded baseline in [`docs/performance.md`](../performance.md) (with M44's measured figure from [`docs/testing/m44-report.md`](m44-report.md) shown for `low-end lite 300 slides`, which post-dates the M21 baseline table):

| Stage (300-slide course, 1280×900) | Budget (`docs/performance.md`) | Baseline (`docs/performance.md`, median / p95) | Measured (`perf.log`, median / p95) | Notes |
|---|---:|---:|---:|---|
| `idle` | 16 ms | 2.1 / 2.2 ms | **0.40 ms / 0.42 ms** | Well under 16 ms budget |
| `select slide 150` | 33 ms | 2.1 / 2.1 ms | **0.64 ms / 0.79 ms** | Well under 33 ms budget |
| `scroll slide list` | 33 ms | 1.9 / 2.1 ms | **0.36 ms / 0.38 ms** | Well under 33 ms budget |
| `typing in a text field` | 33 ms | 2.0 / 2.0 ms | **0.54 ms / 0.55 ms** | Well under 33 ms budget |
| `type on canvas with spell check` | 33 ms | 0.42 / 0.44 ms | **0.80 ms / 0.83 ms** | Well under 33 ms budget |
| `check spelling, open` | 250 ms | 31 ms | **38.23 ms** | Well under 250 ms budget |
| `check spelling, ignore step` (20 issues) | 33 ms | 9.5 / 10.0 ms | **0.45 ms / 0.58 ms** | Well under 33 ms budget |
| `insert interaction + undo` (8 kinds) | 100 ms | 2.0 / 2.2 ms | **1.15 ms / 1.60 ms** | Well under 100 ms budget |
| `undo / redo move` (30 cycles) | 33 ms | 8.2 / 8.7 ms (M33 target ≤ 6.5 / 7.5 ms) | **2.58 ms / 2.91 ms** | Well under budget and M33 target |
| `first frame` | 500 ms | 13 ms | **8.51 ms** | Well under 500 ms budget |
| `visit every slide` | 33 ms | 1.9 / 1.9 ms | **0.43 ms / 0.72 ms** | `rss 265 -> 266 MB` |
| `visit every slide, real pictures` | 33 ms | 4.2 / 7.0 ms | **0.44 ms / 0.72 ms** | `rss 270 -> 273 MB, media 0.2 MiB, 1 decoded pictures` |
| `low-end lite 300 slides` | 33 ms | 15.31 / 35.65 ms (`m44-report.md`) | **14.84 ms / 35.72 ms** | `media 4.4 MiB, undo 0.0 KiB` (test passed; median under 33 ms, p95 unchanged vs. M44) |

## Skipped Steps & Open Concerns

- **Skipped steps:** None — `/Users/aditya/Documents/Claude Projects/ScormCraft/target/m45/SKIPPED` does not exist. All 8 steps (`s1`–`s8`) completed, committed, and passed review.
- **Open concerns from `/Users/aditya/Documents/Claude Projects/ScormCraft/target/m45/REVIEW.md`:**
  - **None unresolved.** Steps `s1`–`s7` received `Verdict: OK` with no changes required. Step `s8` received `Verdict: FIXED` after Neon's review caught and fixed an unclamped `offset` in `crates/ui-egui/src/timeline_playback.rs` (`flush_seeks`) when `playing == true` for trimmed videos seeking past `trim.end_ms`, adding regression test `seeking_while_playing_past_video_trim_end_clamps_last_frame_seek` and re-verifying `fmt`, `clippy`, `wasm32`, and tests (followed by full gate `g3` and `p3`).
- **Known limits & pre-release operational notes (from the plan and step logs):**
  1. **macOS `.ccraft` file association is out of scope:** Per `docs/testing/m45-plan.md` step 4, double-clicking a `.ccraft` file on macOS delivers an OS open-file event rather than a CLI argument, and CourseCraft does not handle that event yet. Only Windows and Linux file associations shipped in M45.
  2. **`.github/workflows/release.yml` hand-commit check:** Steps `S4` and `S5` edited `.github/workflows/release.yml`. Per the plan's reviewer notes, verify/commit `.github` workflow changes explicitly in the owner's release orchestration if the driver omits `.github` when merging.
  3. **Packaging tools not executed locally:** Inno Setup (`iscc`), Linux `desktop-file-validate` / `appimagetool`, and the GitHub Actions macOS Intel (`x86_64-apple-darwin`) cross-build and `hdiutil create -size 200m` packaging run were checked via static assertions (`M45-09`, `M45-10`) and `sh -n scripts/package-macos.sh`; they are verified end-to-end on the release workflow run and target OSes.
  4. **Perf suite timing relative to `S7`–`S8`:** The driver ran `coursecraft-ui-egui::perf_tests` (`perf.log`) after `S6`/`p2` and did not re-run it after follow-up steps `S7`–`S8` (`steps2.sh` ran `g3` and `p3`). Re-run `cargo xtask perf` before release if fresh post-`S8` timings are desired.
  5. **Pre-existing `coursecraft-playback` WASM warnings:** `cargo xtask build-player` (`p1`–`p3`) succeeded with 16 pre-existing `dead_code` warnings in `crates/playback/src/lib.rs` (unrelated to M45).

## Owner Manual Verification Checklist

*(Derived from steps 1–6 and the reviewer notes in `docs/testing/m45-plan.md`, which did not include a separate checklist section, plus follow-up steps `S7`–`S8`.)*

- [ ] **Insert › Media › Caption (`S1`):** In the desktop editor, click **Insert › Media › Caption** (both the button and split dropdown), confirm the picker shows only **Callouts** (no Draw tools or other shape categories), pick a callout, drag it onto the slide, and verify **Insert › Shape** still opens the full gallery afterwards.
- [ ] **Web player loading indicator (`S2`):** Publish a course to Web (or SCORM) and open `index.html` in a browser with network throttling enabled. Confirm the centred spinner and `"Loading course…"` appear immediately and vanish on the first rendered frame; toggle **Reduce motion** in OS/browser settings to confirm the spinner is hidden and only text is shown; simulate a startup failure (e.g., rename the `.wasm` file) to confirm an error message replaces the spinner.
- [ ] **Spell check course language (`S3`):** Open a course with a misspelling and leave **Course language** unset, then set it to `en-GB` — confirm squiggles and Check Spelling issues appear as before. Change **Course language** to `fr-CA` (or `de`) — confirm as-you-type squiggles disappear and **Check Spelling** shows `"Spelling is checked in English only. This course's language is fr-CA."`.
- [ ] **`.ccraft` file association on Windows & Linux (`S4`):**
  - **Windows:** Install from the Inno Setup `-setup.exe`, confirm `.ccraft` files get the CourseCraft icon and double-clicking a `.ccraft` file (including one in a path containing spaces) opens it in CourseCraft; uninstall and confirm registry keys clean up.
  - **Linux:** Verify the `.tar.gz` and `.AppImage` layouts contain `coursecraft.desktop` (with `MimeType=application/x-coursecraft-project;`) and `share/mime/packages/coursecraft.xml`, and double-clicking a `.ccraft` file after desktop/MIME install opens CourseCraft.
- [ ] **Intel Mac `.dmg` in release CI (`S5`):** Verify `.github/workflows/release.yml` changes are included before tagging, run the release workflow, and confirm both `arm64` and `x86_64` `.dmg` artifacts build cleanly without `hdiutil` out-of-space errors, appear in `SHA256SUMS.txt`, and launch on an Intel Mac (or under Rosetta 2) without missing `libswiftCoreMedia.dylib`.
- [ ] **Feature parity doc review (`S6`):** Skim `docs/ui/feature-parity.md` summary counts (`110 | 39 | 58 | 12`) and updated rows (**Graded question kinds**, **Theme fonts**, **Spelling**, **Caption / callout shapes**) before tagging the release.
- [ ] **Fill toggle restores previous colour (`S7`):** Fill a shape (solid and linear gradient), a button, a multi-selection of differently coloured shapes, and a custom state override; untick **Fill** and re-tick **Fill** on each and confirm the prior colour/gradient returns (and `Ctrl+Z` / `⌘Z` undo/redo works cleanly). Confirm a shape created with no fill still picks up the default fill when ticked.
- [ ] **Audio timeline bar cropping & waveform scale (`S8`):** Insert a ~10 s audio clip, drag the end handle of its timeline bar back to ~5 s, and verify: (a) the waveform stays at real time scale and is cut off at 5 s rather than squeezed; (b) pressing Play on the timeline stops audio at 5 s, starting Play mid-bar plays from the matching offset, and starting Play after 5 s plays nothing; (c) editor **Preview** and a **Published** web course likewise stop the audio at 5 s without looping or continuing past the bar end.
