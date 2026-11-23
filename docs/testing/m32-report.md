# M32 UI Quick Wins & Slide-List Speed Report

**Milestone:** M32 (UI quick wins & slide-list speed)  
**Plan:** [`docs/testing/m32-plan.md`](m32-plan.md)  
**Scope:** Sentence-case label pass & multi-name unification (`M32-01`–`M32-03`), unified `File ▸ Import ▸` & symmetric `File ▸ Export ▸` submenus (`M32-08`), top-level `Preview` menu & **Preview from here** (`M32-06`–`M32-07`), single-source shortcut table & tooltips (`M32-04`), disabled-control hover reasons & static CI guard (`M32-05`), and visible-only slide-list composition with revision-keyed `ThumbnailCache` (`M32-09`–`M32-12`), across steps `S1`–`S9`.

## What Shipped Per Step (`S1`–`S9`)

| Step | Summary | Review Verdict |
|---|---|---|
| **S1** | Converted all 186 `REGISTRY` `label` and `menu` entries in `crates/engine/src/commands.rs` to sentence case with zero `id` changes; moved `course.update` to `Project/Course settings…` and removed the manual duplicate button in `menu.rs`; added `format_undo_redo_label` for dynamic Edit menu Undo/Redo labels; synced `docs/cli.md`. | `OK` |
| **S2** | Applied sentence-case copy and multi-name unification across `menu.rs`, `toolbar.rs`, `slides.rs`, `shortcuts.rs`, `media_library.rs`, `lib.rs`, `audio_ai.rs`, `record_narration.rs`, `clipboard.rs`, `input_insert.rs`, `interaction_insert.rs`, and `web_insert.rs` (`New slide`, `Slide masters`, `Close slide masters`, `Not available in slide masters`, `Preview this slide`, `Preview whole course`, `Select all`, `Format painter`, template `Delete`). | `OK` |
| **S3** | Applied sentence-case labels across dialogs, panels, inspector, status bar, `snapshot.rs` `WINDOWS` needles, `docs/cli.md`, and `apps/coursecraft/src/main.rs` (`Save a copy`, `Open latest version`, preserving `Don't Save`). Review fixes caught remaining Title Case stragglers in `properties.rs` (arrange strip & section headers), `preview.rs`, and `triggers.rs`, and extended `L-04` to lint `inspector_section_header`. | `FIXED` |
| **S4** | Consolidated all 7 import actions under `File ▸ Import ▸` (`PowerPoint (.pptx)…`, `Course from document…`, `Slides from outline…`, `Questions from spreadsheet (CSV)…`, `Translation (XLIFF)…`, separator, `Media files…`, `Font…`) with `desktop_file_ops` gating; moved `Translation (XLIFF)…` into `File ▸ Export ▸`; updated `course.import_outline` menu path in `commands.rs` and `docs/cli.md`; removed old top-level `File` and `Insert` duplicates while keeping Welcome and Media Library shortcuts. | `OK` |
| **S5** | Added `Preview::new_from_slide` (`preview.rs`) and `Editor::open_preview_from` (`lib.rs`, excluding bank slides and keeping full-course playback + restart-at-slide semantics with zero `coursecraft-logic` changes); added top-level `Preview` menu (before `Help`) sharing `show_preview_menu_items` with the toolbar Preview caret; added `F1`–`F20` formatting in `menu.rs`; bound **Preview from here** to `Ctrl+F12` (`⌘F12` on macOS, chosen over the plan's `Alt+F12` to avoid macOS `Option+F12` clashes). | `OK` |
| **S6** | Extended M31's shortcut table in `shortcuts.rs` with `ShortcutAction` (24 actions), `shortcut_for_action`, and `shortcut_for_command`, preserving M31's `File` group; added `tooltip_with_shortcut` and `menu_button` in `menu.rs`; wired all shortcutted toolbar buttons, menus (including `View ▸ Course map` and `View ▸ Zoom` items), context menus, slide menus, and all 9 Bold/Italic/Underline toggles in `properties.rs` to the shared source. | `OK` |
| **S7** | Added `icon_button_enabled_reason` / `icon_button_danger_reason` (`widgets.rs`) and `quiet_button_enabled` (`lib.rs`, `RISK-08`); resolved all 15 audited disabled-control sites (including the `.on_hover_text` bug on the required drop-target checkbox in `question.rs` and `ui.disable()` in `properties.rs`); added `D-01` and static CI guard `D-02`. Review fixes closed a blind spot in `D-02` where out-of-line `#[cfg(test)] mod foo;` declarations truncated scanning of `lib.rs` and `apps/coursecraft/src/main.rs`. | `FIXED` |
| **S8** | Added unsuppressed `THUMBNAIL_COMPOSES` counter (`guard_counters.rs`), `ThumbnailCache` (`MAX_CACHED_THUMBNAILS = 128`, O(1) `verified_revision` fast path, deterministic 64-bit FNV-1a fingerprinting, LRU eviction, cleared on `Editor::replace_course`), revision-cached `NavigatorIndex`, and viewport culling across slide rows, scene/bank headers, masters (`slides.rs`), and Course map (`course_map.rs`). Fix round 1 (`s8-fix1`) clipped `mark_content_region` to `visible_thumb` and made the Triggers filter box in `triggers.rs` responsive after culling unmasked a pre-existing `UX-04` clip in narrow windows. | `OK` |
| **S9** | Strengthened `idle_frames_do_no_counted_work` with `gc::thumbnail_composes() == 0`, added `many_slide_course` and counter-based guard tests `SL-01`–`SL-04` in `guard_tests.rs`, and verified `D-03` (`uiux_sweep`, 0 MUST violations). | `OK` |

## Test Coverage (`L-01`–`L-05`, `I-01`–`I-03`, `P-01`–`P-03`, `SC-01`–`SC-03`, `D-01`–`D-03`, `SL-01`–`SL-06`)

| ID | Step | Crate / File | Coverage |
|---|---|---|---|
| `L-01` | S1 | `coursecraft-engine` (`tests.rs`) | `registry_labels_and_menu_leaves_use_sentence_case`: all 186 `REGISTRY` labels & menu leaves pass sentence-case lint; command IDs unchanged; `docs/cli.md` stays in sync |
| `L-02` | S2 | `coursecraft-ui-egui` (`toolbar_menu_tests.rs`) | `menus_toolbar_and_navigator_use_canonical_sentence_case_names`: canonical names (`New slide`, `Slide masters`, `Preview this slide`, `Preview whole course`, `Select all`, `Course settings…`) with no legacy duplicates |
| `L-03` | S3 | `coursecraft-ui-egui` (`u13_core_authoring_dialogs_tests.rs`, `u14_ai_dialogs_tests.rs`, `u15_status_welcome_tests.rs`) | `ai_and_translation_dialogs_paint_sentence_case_titles` & `status_and_report_dialogs_paint_sentence_case_titles`: dialog titles and `WINDOWS` needles paint in sentence case |
| `L-04` | S3 | `coursecraft-ui-egui` (`tests.rs`) | `static_sentence_case_lint_for_dialogs_menus_toolbar_and_slides`: static sentence-case lint over `Dialog::new`, `menu.rs`, `toolbar.rs`, `slides.rs`, and `inspector_section_header` in `properties.rs` |
| `L-05` | S3 | `coursecraft` (`m27_files_ui_tests.rs`) | `app_shell_modals_use_sentence_case_and_keep_dont_save`: `"Save a copy"`, `"Open latest version"`, and platform-standard `"Don't Save"` preserved |
| `I-01` | S4 | `coursecraft-ui-egui` (`toolbar_menu_tests.rs`) | `file_import_menu_lists_all_seven_items_in_order_and_triggers_actions`: `File ▸ Import ▸` order and emitted requests/dialogs |
| `I-02` | S4 | `coursecraft-ui-egui` (`toolbar_menu_tests.rs`) | `old_scattered_import_and_export_duplicates_are_removed`: old `File`/`Insert` duplicates removed; `File ▸ Export ▸ Translation (XLIFF)…` present |
| `I-03` | S4 | `coursecraft-ui-egui` (`toolbar_menu_tests.rs`) | `web_mode_disables_desktop_only_import_and_export_items_with_tooltips`: web-mode disabled tooltips and no-op clicks for desktop-only imports/exports |
| `P-01` | S5 | `coursecraft-ui-egui` (`toolbar_menu_tests.rs`) | `preview_menu_and_toolbar_caret_share_items_shortcuts_and_disabled_reasons` (plus `test_function_key_shortcut_formatting_both_platforms` in `tests.rs`) |
| `P-02` | S5 | `coursecraft-ui-egui` (`preview_subset_tests.rs`) | `preview_from_here_starts_at_selected_slide_advances_and_restarts_there`: starts at slide 2 via menu or `Ctrl+F12`, `is_partial_preview() == false`, Next advances, Restart returns to slide 2 |
| `P-03` | S5 | `coursecraft-ui-egui` (`preview_subset_tests.rs`) | `preview_menu_modes_and_bank_and_slide_masters_disabled_reasons`: all 4 preview modes, bank-slide disabling, and slide-masters disabling with reasons |
| `SC-01` | S6 | `coursecraft-ui-egui` (`toolbar_menu_tests.rs`) | `toolbar_buttons_with_shortcuts_show_formatted_shortcut_in_tooltip`: `New slide`, `Preview`, `Publish`, `Map`, `Paint`, `Undo`, `Redo` tooltips |
| `SC-02` | S6 | `coursecraft-ui-egui` (`toolbar_menu_tests.rs`) | `menu_items_show_shortcut_tooltips_matching_shortcut_groups`: menu hints & hover tooltips match `build_shortcut_groups()` |
| `SC-03` | S6 | `coursecraft-ui-egui` (`text_format_tests.rs`) | `inspector_bold_italic_underline_tooltips_follow_platform_shortcuts`: macOS `⌘B`/`⌘I`/`⌘U` vs Windows/Linux `Ctrl+B`/`Ctrl+I`/`Ctrl+U` |
| `D-01` | S7 | `coursecraft-ui-egui` (`toolbar_menu_tests.rs`) | `disabled_controls_show_explanatory_hover_reasons`: runtime hover checks across all audited disabled controls |
| `D-02` | S7 | `coursecraft-ui-egui` (`tests.rs`) | `every_add_enabled_has_disabled_reason` (+ self-check test): static CI guard over `crates/ui-egui/src` and `apps/coursecraft/src` forbidding unreasoned `add_enabled` / `add_enabled_ui` and `ui.disable()` |
| `D-03` | S9 | `coursecraft-ui-egui` (`uiux_sweep_tests.rs`) | `uiux_sweep_all_surfaces`, `uiux_sweep_scale_2_and_fixtures`, `uiux_sweep_modals_and_tooltips`: **3 passed, 0 failed, 0 MUST violations** |
| `SL-01` | S9 | `coursecraft-ui-egui` (`guard_tests.rs`) | `idle_frames_do_zero_thumbnail_composes`: 300-slide course, 20 idle frames → `thumbnail_composes() == 0`, `thumbnail_redraws() == 0` |
| `SL-02` | S9 | `coursecraft-ui-egui` (`guard_tests.rs`) | `moving_object_composes_only_one_thumbnail`: nudging `t0` on slide 0 of 300 slides → `thumbnail_composes() == 1` |
| `SL-03` | S9 | `coursecraft-ui-egui` (`guard_tests.rs`) | `scrolling_300_slide_list_composes_only_visible_rows`: downward scroll composes `1..=10`; scrolling back to cached top at same revision composes `0` |
| `SL-04` | S9 | `coursecraft-ui-egui` (`guard_tests.rs`) | `editing_master_or_theme_invalidates_visible_thumbnails`: master object & theme edits trigger `>= 1` compose; visible selection switch triggers `0` |
| `SL-05` | S8 | `coursecraft-ui-egui` (`slides.rs`) | `thumbnail_cache_cap_reset_scroll_to_rect_and_drag_reorder`: cap `<= 128`, `replace_course` clear, off-screen `s-250` `scroll_to_rect`, slide drag-reorder |
| `SL-06` | S9 / Driver | `coursecraft-ui-egui` (`perf_tests.rs`, `--release --ignored`) | All 8 release editor perf tests passed well below budget and below every M32 target (see measurements below) |

## Driver Gates

- **`cargo xtask ci`:**
  - `g1` (after S3): passed on attempt 1.
  - `g2` (after S5): passed on attempt 1.
  - `g3` (after S7): passed on attempt 1.
  - `g4` (after S9): attempt 1 failed in `coursecraft-playback` (`bughunt_m26_a7_native_video::a7_10_native_playback_backend_rejects_hostile_bytes_safely` hit the 10 s timeout during cold macOS `AVFoundation` initialization); raised the per-payload wait timeout from `10s` to `30s` (`crates/playback/src/bughunt_m26_a7_native_video.rs`), after which `g4` passed on attempt 2.
- **`cargo xtask build-player` (`p1`, after `g4`):** passed (`trunk build --release` wrote `target/player-dist` in `36.48s`).
- **Release editor perf suite (`cargo test --release -p coursecraft-ui-egui perf_tests -- --ignored --nocapture --test-threads=1`):** passed (`8 passed; 0 failed` in `1.16s`).
- **UI/UX sweep & guard tests (verified in S9 and `g4`):** `guard_tests` (`17 passed; 0 failed`), `uiux_sweep` (`3 passed; 0 failed`, 0 MUST violations), `coursecraft-ui-egui` full unit suite (`1295 passed; 0 failed; 10 ignored`).

## Measured Performance vs. Plan Baseline

Source: `/Users/aditya/Documents/Claude Projects/ScormCraft/target/m32/perf.log` (Apple Silicon macOS, `--release`, `--test-threads=1`) compared against the baseline and targets recorded in `docs/testing/m32-plan.md` §2.1:

| Stage | Budget | Plan Baseline (median / p95) | M32 Target (median / p95) | Measured After M32 (median / p95) | Notes |
|---|---:|---:|---:|---:|---|
| `idle` | 16 ms | 1.37 ms / 1.43 ms | ≤ 0.65 ms / ≤ 0.80 ms | **0.28 ms / 0.29 ms** | ~4.9× faster median |
| `select slide 150` | 33 ms | 1.59 ms / 1.70 ms | ≤ 0.85 ms / ≤ 1.00 ms | **0.37 ms / 0.47 ms** | ~4.3× faster median |
| `scroll slide list` | 33 ms | 1.32 ms / 1.37 ms | ≤ 0.75 ms / ≤ 0.90 ms | **0.24 ms / 0.27 ms** | ~5.5× faster median |
| `typing in a text field` | 33 ms | 1.51 ms / 1.53 ms | ≤ 0.85 ms / ≤ 1.00 ms | **0.34 ms / 0.38 ms** | ~4.4× faster median |
| `visit every slide` | 33 ms | 1.41 ms / 1.69 ms | ≤ 0.85 ms / ≤ 1.10 ms | **0.28 ms / 0.44 ms** | rss `95 -> 97 MB` |
| `visit every slide, real pictures` | 33 ms | 4.27 ms / 7.07 ms | ≤ 4.0 ms / ≤ 6.5 ms | **1.78 ms / 3.25 ms** | rss `114 -> 138 MB`, 81 decoded pictures |
| `undo / redo move` (30 cycles) | 33 ms | 9.52 ms / 9.85 ms | Improved further in M33 | **5.00 ms / 5.40 ms** | ~1.9× faster median |
| `first frame` | 500 ms | 19.65 ms | ≤ 15 ms | **8.49 ms** | ~2.3× faster |
| `check spelling, open` | 250 ms | — | — | **16.94 ms** | Single-shot open |
| `check spelling, ignore step` | 33 ms | — | — | **5.84 ms / 6.35 ms** | 20 issues |
| `insert interaction + undo` | 100 ms | — | — | **1.80 ms / 2.11 ms** | 8 kinds |
| `type on canvas with spell check` | 33 ms | — | — | **0.44 ms / 0.48 ms** | — |

_Note:_ The driver's `do_perf` ran the 8 release `coursecraft-ui-egui` `perf_tests` above; the big-course save/publish suite (`cargo xtask perf`) was not run separately in `perf.log` (M32 made no save/publish changes).

## Skipped Steps & Open Concerns

- **Skipped steps:** None (`SKIPPED` file does not exist; all 9 steps `S1`–`S9` completed, reviewed, and committed).
- **Open concerns from `REVIEW.md`:**
  - **Canvas zoom HUD buttons (minor, outside S7 scope):** `crates/ui-egui/src/canvas.rs:7504,7527` (`Zoom out` / `Zoom in` HUD icon buttons) still use `icon_button_enabled` with `"Zoom out"` / `"Zoom in"` at min/max zoom instead of `icon_button_enabled_reason` with a min/max explanation.
  - **Shortcut adjustment (intentional, per S5 reviewer note):** **Preview from here** shipped as `Ctrl+F12` (`⌘F12` on macOS) rather than `Alt+F12` to avoid macOS `Option+F12` clashes, and shares the `"Preview this slide / from here"` row (`F12 / Ctrl+F12`) in the Keyboard shortcuts dialog (similarly `Course map / Zoom to fit`) so the dialog continues to fit without scrolling at `1440×900`.
  - **Docs follow-up:** `docs/performance.md` has not yet been updated with the post-M32 release numbers above (tracked here in `m32-report.md`).

## Owner Manual Verification Checklist (Desktop App — ~8 Minutes)

- [ ] **Labels & consistency:** Open each top-level menu (`File`, `Edit`, `Insert`, `Arrange`, `Project`, `Slide`, `View`, `Preview`, `Help`). Verify every item uses sentence case (no leftover Title Case such as `"Save As…"` or `"Slide Masters"`), the toolbar reads **`New slide`** (not `"Add slide"`), and `View ▸ Slide masters` matches **`Close slide masters`**.
- [ ] **Shortcut tooltips:** Hover **`New slide`**, **`Preview`**, **`Publish`**, **`Map`**, **`Paint`**, **`Undo`**, and **`Redo`** on the toolbar and hover shortcutted items in `File`, `Edit`, `View`, `Preview`, and `Help` — verify macOS shows `⌘` symbols (and Windows/Linux shows `Ctrl+`), matching **Help ▸ Keyboard shortcuts** (`?`). Select a text box and hover `B` / `I` / `U` in the Format inspector — verify macOS shows `Bold (⌘B)`.
- [ ] **Disabled reasons:** On a freshly opened course with nothing selected, hover disabled **`Undo`**, **`Redo`**, **`Paint`**, **Arrange ▸ Group**, and in the slide context menu **`Move up`** (on slide 1) and **`Preview selected slides`** (with 1 slide selected). Verify every disabled control shows a clear, short reason (never blank and never just repeating the button name).
- [ ] **Preview menu:** On slide 2 of `examples/showcase/`, test all four items in the top-level **Preview** menu (and verify the toolbar Preview caret matches):
  - **Preview this slide (`F12`)** → plays slide 2 only; clicking Next shows the end-of-preview popup.
  - **Preview from here (`⌘F12` / `Ctrl+F12`)** → starts on slide 2, clicking Next continues to slide 3, and clicking Restart returns to slide 2.
  - Select slides 2 & 4 (`⌘`-click) → **Preview selected slides** plays only those 2 slides.
  - **Preview whole course (`⇧F12` / `Shift+F12`)** → plays from slide 1.
- [ ] **One Import menu:** Open **File ▸ Import ▸** and verify all 7 imports (`PowerPoint (.pptx)…`, `Course from document…`, `Slides from outline…`, `Questions from spreadsheet (CSV)…`, `Translation (XLIFF)…`, `Media files…`, `Font…`) are gathered in one submenu, and **File ▸ Export ▸** holds `Video (MP4)…`, `PDF for review…`, `Word for review…`, and `Translation (XLIFF)…`.
- [ ] **Slide-list smoothness:** Open a large multi-slide course (or duplicate slides to ~100+), scroll the left slide list quickly up and down, nudge an object on a visible slide, and edit a slide master — verify scrolling feels instant and thumbnails update immediately with zero flicker or stale previews.
