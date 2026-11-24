# M37 Report — Point Text (Issue #7) & Interactive Button Groups (Issue #8 Part B)

**Milestone:** M37 (Issue #7: point text vs area text; Issue #8 Part B: one shared state for a grouped button)  
**Plan:** [`docs/testing/m37-plan.md`](m37-plan.md)  
**Scope:** Backward-compatibility golden tests (`S1`), point text model/metrics/engine auto-fit/canvas click-vs-drag & live inline growth (`S2`–`S5`), interactive `Group` model/validation/runtime/renderer/accessibility/preview-published parity/engine commands/editor UI (`S6`–`S12`), and M28 UI/UX sweep (`S13`). The plan's 10 logical steps were split into 13 focused execution steps (`S1`–`S13`) incorporating all five reviewer overrides in `docs/testing/m37-plan.md` (pre-model golden tests, `skrifa`-vs-`egui` layout parity, no silent member-trigger migration and blocking opt-out while group triggers exist, same-step spec/CLI updates, and Preview-vs-Published parity).

## What Shipped Per Step (`S1`–`S13`)

| Step | Summary | Review Verdict |
|---|---|---|
| **S1** | Added 4 pre-model backward-compatibility golden tests (`m37_golden_course_json_roundtrip`, `m37_golden_ccraft_roundtrip`, `m37_golden_published_course_json`, `m37_golden_suspend_data_resume`) locking pre-M37 FNV-1a hashes, byte lengths, `cc1:` `suspend_data` literal, and absence of `"point_text"`/`"groups"`. Zero production changes. | `OK` |
| **S2** | Added `#[serde(default, skip_serializing_if = "is_false")] pub point_text: bool` on `ObjectKind::Text` and `StateExtraKind::Text` (`crates/course/src/model.rs`), updated exhaustive literals/patterns across workspace crates, documented `point_text` in `docs/spec/course-model.md`, and added `PT-01`. | `OK` |
| **S3** | Moved the single copy of `Inter-{Regular,Bold,Italic,BoldItalic}.ttf` + `OFL.txt` into `crates/media/fonts/`, added WASM-safe headless `skrifa` measurement in `crates/media/src/text_metrics.rs`, pointed `crates/render/src/font.rs` at `coursecraft_media::text_metrics::INTER_*`, and added `PT-02` plus 6 `text_metrics` unit tests. Review fixed `MAX_CHARS` enforcement across `\n`/`\r`/remaining runs and added `hostile_newline_and_cr_flood_respects_char_cap`. | `FIXED` |
| **S4** | Added `coursecraft-media` dependency to `crates/engine` (L5 → L2), implemented `parse_measured_size` and `fit_point_text_frame` (`crates/engine/src/commands/text.rs`) with alignment/valign anchoring and `1.0..=10_000.0` clamping, wired auto-fit and 1-step area-text conversion across `object.add`, `object.update`, `object.set_frame`, `object.apply_style`, `object.state_add_extra`, `object.state_update_extra`, `text.set`, `text.style_range`, `text.set_paragraph`, `text.set_paragraphs`, and `object.set_autofit` while preserving `point_text` on move/align/distribute, and added `PT-03`–`PT-07`. | `OK` |
| **S5** | Added `inline_edit_layout_job_with_wrap` (`crates/render/src/text_layout.rs`) setting `max_width = f32::INFINITY` when `!wrap`, wired click vs drag placement in `crates/ui-egui/src/canvas.rs` for both stage (`object.add`) and State Editor extras (`object.state_add_extra`), added live keystroke growth/shrink in `draw_inline_edit` with validated `measured_size` on commit, added the `"Wrap text"` point-text tooltip in `crates/ui-egui/src/properties.rs`, and added `PT-08`–`PT-10`. | `OK` |
| **S6** | Defined `Group` (`#[serde(deny_unknown_fields)]`) and `Slide.groups` / `Layer.groups` (`crates/course/src/model.rs`, re-exported from `crates/course/src/lib.rs`), wired `groups` into `SlideFieldPatch`, `SlideDelta::diff`/`apply`, `layer_metadata_eq`, and saturating byte estimators (`crates/engine/src/delta.rs`), and added `GB-01`. | `OK` |
| **S7** | Added `Slide` group helpers (`group`, `interactive_group`, `interactive_group_of_object`, `group_members`, `group_has_state`, `is_disallowed_interactive_group_member`, and group IDs in `local_ids`) in `crates/course/src/lib.rs`, added group and interactive-group trigger validation in `crates/course/src/validate.rs`, documented `Slide.groups`/`Layer.groups` and `Group` in `docs/spec/course-model.md`, and added `GB-02`. | `OK` |
| **S8** | Added the normative `## Interactive groups` section in `docs/spec/triggers.md`, implemented shared group state, liveness, single-fire triggers (preserving member triggers), built-in `selected`/`visited`, `disabled`/`hidden`, hover, keyboard focus/activation, and `suspend_data` encoding (at `all_objects.len()..`, byte-identical when 0 interactive groups) in `crates/logic/src/{lib.rs,run.rs,suspend.rs}`, and added `GB-03`–`GB-05`. | `OK` |
| **S9** | Updated `crates/render/src/{compose.rs,paint.rs,view.rs,focus.rs,offscreen.rs}` for shared hover/down pointer state (blocked when `disabled`), post-panel `remap_interactive_group_hits` (`RISK-06`), clickable cursor, `PrintSlide` exclusion, single Tab stop + Enter/Space activation, and primary-member union focus ring (ignoring oversized state extras), and added `GB-06`–`GB-08`. | `OK` |
| **S10** | Updated `crates/render/src/access.rs` to emit one `AccessRole::Button` node (`group:<slide>:<group>`) per interactive group with 4-tier `%var%`-substituted label, `checked` state, primary-only union rect, and member `content_node` suppression, keeping `targets(&tree) == focus.list()`, and added `GB-09`–`GB-11` (including Editor Preview vs Published parity and web-player integration). | `OK` |
| **S11** | Added `group.set_interactive` (no silent member-trigger migration; blocks opt-out without mutating while group triggers exist; collision-safe `grp-N` remapping; disallowed-member rejection) and `group.update`, updated lifecycle cleanup in `object.group`/`ungroup`/`delete`/`duplicate`/`paste`, updated `REGISTRY` + `r5_01_registry_golden_lock` (`186` → `188`) and `docs/cli.md`, and added `GB-12`. Review fixed no-op opt-out for plain groups whose legacy ID collides with an object ID and capped/validated `object.paste` `groups` (`<= 500`, `MAX_TEXT`), adding `gb_12_opt_out_plain_colliding_group_and_hostile_paste_groups`. | `FIXED` |
| **S12** | Added `"Make button from group"` / `"Remove button behaviour"` in Arrange and canvas/timeline context menus (`menu.rs`), `"Act as one button"` + Name / Initial state / Accessible description / Focus order / `"Add trigger…"` in the Group Inspector (`properties.rs`), group name + palette-only `"Button"` pill on timeline group folder rows (`timeline.rs`), and interactive-group support across all 7 trigger pickers, labels, defaults, and validation badges (`triggers.rs`, `toolbar.rs`), plus `GB-13`. | `OK` |
| **S13** | Added `gb_14_m37_point_text_and_interactive_group_uiux_sweep` in `crates/ui-egui/src/uiux_sweep_tests.rs` covering both point text and interactive groups across the full M28 matrix (`1024×600`/`1280×800`/`1920×1080` × Dark/Light × scale `1.0`/`1.5`, web mode, scale `2.0`, trailing colons, tooltips, popups, modals, and menus) with `0` MUST violations and zero new allowlist exemptions. | `OK` |

## Test Coverage (46 New Passing Tests: `3,950` → `3,996` in `cargo xtask ci`)

| ID | Step | Crate / File | Coverage |
|---|---|---|---|
| `M37-G1` | S1 | `coursecraft-course` (`tests.rs`) | `m37_golden_course_json_roundtrip`: locks pre-M37 FNV-1a hashes, byte lengths, and absence of `"point_text"`/`"groups"` on `examples/hello-course` and `examples/showcase` |
| `M37-G2` | S1 | `coursecraft-format` (`tests.rs`) | `m37_golden_ccraft_roundtrip`: locks `.ccraft` archive round-trip byte identity |
| `M37-G3` | S1 | `coursecraft-publish` (`tests.rs`) | `m37_golden_published_course_json`: locks published `course.json` hash and byte identity |
| `M37-G4` | S1 | `coursecraft-logic` (`tests.rs`) | `m37_golden_suspend_data_resume`: locks pre-M37 `cc1:` `suspend_data` literal and full resume parity (slide, visited slides, object states, variables, quiz answers) |
| `PT-01` | S2 | `coursecraft-course` (`tests.rs`) | `pt_01_old_course_json_defaults_point_text_false_and_roundtrips`, `pt_01_point_text_true_roundtrips`: default `false`, JSON omission on area text, preservation on `point_text: true`, byte-identical pre-M37 round-trips |
| `PT-02` | S3 | `coursecraft-media` (`text_metrics.rs`), `coursecraft-render` (`tests.rs`) | 7 `text_metrics` unit tests (`empty_string_has_zero_width_and_positive_line_height`, `single_line_scales_with_length_and_size`, `multiline_takes_max_width_and_summed_height`, `bold_is_wider_than_regular`, `hostile_nan_inf_and_huge_inputs_never_panic_and_stay_clamped`, `paragraph_metrics_apply_spacing_and_list_indent`, `hostile_newline_and_cr_flood_respects_char_cap`) + `pt_02_skrifa_matches_egui_unwrapped_layout` across strings, emoji/symbols, 4 styles, 4 sizes, letter spacing, and super/subscript |
| `PT-03` | S4 | `coursecraft-engine` (`tests.rs`) | `pt_03_headless_point_text_add_and_refit`, `pt_03_paragraphs_apply_style_and_hostile_measured_size`: headless auto-fit on `object.add`, `text.set`, `text.style_range`, `text.set_paragraphs`, `object.apply_style`, `Align::Center` anchoring, and hostile `measured_size` sanitization |
| `PT-04` | S4 | `coursecraft-engine` (`tests.rs`) | `pt_04_point_text_converts_to_area_text`: conversion to area text on `text.set_paragraph { wrap: true }`, `object.set_frame` width change, and `object.set_autofit { autofit: "shrink" }` |
| `PT-05` | S4 | `coursecraft-engine` (`tests.rs`) | `pt_05_move_align_distribute_preserve_point_text`: pure translation, `object.align`, and `object.distribute` preserve `point_text: true` and `wrap: false` |
| `PT-06` | S4 | `coursecraft-engine` (`tests.rs`) | `pt_06_state_extra_point_text_fits_and_converts`: `object.state_add_extra` and `object.state_update_extra` auto-fit and convert on wrap/width resize |
| `PT-07` | S4 | `coursecraft-engine` (`m27_all_commands_undo_tests.rs`) | `pt_07_point_text_single_step_undo`: 1-step undo/redo for insert, `text.set` + refit, and width-resize conversion |
| `PT-08` | S5 | `coursecraft-render` (`tests.rs`), `coursecraft-ui-egui` (`m37_point_text_ui_tests.rs`) | `pt_08_inline_edit_layout_job_respects_wrap_false`, `pt_08_canvas_click_places_point_text_and_drag_places_area_text`: unwrapped inline layout job, live keystroke growth/shrink, click point text vs drag area text |
| `PT-09` | S5 | `coursecraft-ui-egui` (`m37_point_text_ui_tests.rs`) | `pt_09_state_editor_click_places_point_text_extra_and_drag_places_area_extra`: State Editor click places fitted point-text extra; drag places fixed-width wrapping extra |
| `PT-10` | S5 | `coursecraft-ui-egui` (`m37_point_text_ui_tests.rs`) | `pt_10_inspector_wrap_and_width_handle_convert_point_text`: `"Wrap text"` tooltip & checkbox and East resize handle convert to area text in 1 undo step |
| `GB-01` | S6 | `coursecraft-course` (`tests.rs`), `coursecraft-engine` (`delta.rs`) | `gb_01_courses_without_groups_serialize_byte_identically`, `gb_01_plain_group_and_interactive_group_serde`, `gb_01_slide_delta_groups_roundtrip`: byte-identical serialization without groups, `Group` serde defaults, and `SlideDelta` undo/redo for `slide.groups` and `layer.groups` |
| `GB-02` | S7 | `coursecraft-course` (`tests.rs`) | `gb_02_validation_rejects_invalid_groups_and_plain_group_triggers`, `gb_02_validation_accepts_valid_interactive_group_and_triggers`: ID collisions, 0-member groups, master/disallowed members, plain-group trigger rejection, and valid interactive-group triggers |
| `GB-03` | S8 | `coursecraft-logic` (`tests.rs`) | `gb_03_interactive_group_states_triggers_and_member_triggers`: shared state, single-fire group triggers, preserved member triggers, built-in `selected`/`visited`, `disabled`/`hidden`, and hover |
| `GB-04` | S8 | `coursecraft-logic` (`tests.rs`) | `gb_04_interactive_group_keyboard_objects_and_focus`: single `keyboard_objects()` entry, member exclusion, scroll-panel viewport, `prevent_base_clicks`/`hide_base_objects`, and focused `KeyPressed` |
| `GB-05` | S8 | `coursecraft-logic` (`tests.rs`) | `gb_05_suspend_data_interactive_groups_and_golden_compat`: golden compatibility for 0 interactive groups and `encode`/`decode`/`resume` persistence of interactive-group states |
| `GB-06` | S9 | `coursecraft-render` (`u19_state_hover_tests.rs`) | `gb_06_interactive_group_shared_hover_down_and_disabled`: offscreen pixel verification of shared hover/down across members (fallback to `normal`), disabled suppression, and `PrintSlide` exclusion |
| `GB-07` | S9 | `coursecraft-render` (`view.rs`) | `gb_07_interactive_group_hit_hover_stability_and_cursor`: zero hover flicker moving across members, cross-member press-release click, and enabled/disabled cursor icons |
| `GB-08` | S9 | `coursecraft-render` (`focus/tests.rs`) | `gb_08_interactive_group_tab_activate_and_union_focus_ring`: single Tab stop, Enter/Space activation, and primary-member union focus ring (ignoring oversized state extras) |
| `GB-09` | S10 | `coursecraft-render` (`access/tests.rs`) | `gb_09_interactive_group_single_button_access_node`: single `AccessRole::Button` node, 4-tier `%var%` label, `checked`, union rect excluding state extras, layer groups, and zero duplicate member `content_node`s |
| `GB-10` | S10 | `coursecraft-ui-egui` (`u16_preview_tests.rs`) | `gb_10_preview_and_published_player_interactive_group_parity`: Editor Preview vs Published player parity on variables, effect traces, composed `DrawItem`s, focus list, `AccessTree`, and offscreen RGBA pixels |
| `GB-11` | S10 | `coursecraft-player` (`tests.rs`) | `gb_11_web_player_interactive_group_pointer_keyboard_and_a11y`: web-player `Playing` pointer, keyboard, and accessibility integration |
| `GB-12` | S11 | `coursecraft-engine` (`tests.rs`, `m27_all_commands_undo_tests.rs`) | `gb_12_group_set_interactive_no_silent_migration_and_opt_out_guard`, `gb_12_group_set_interactive_collision_safe_id_and_rejections`, `gb_12_group_update_and_lifecycle_cleanup`, `gb_12_opt_out_plain_colliding_group_and_hostile_paste_groups`, `gb_12_interactive_group_commands_single_step_undo`, plus updated `r5_01_registry_golden_lock` (`188` commands) and CLI `docs/cli.md` sync |
| `GB-13` | S12 | `coursecraft-ui-egui` (`tests.rs`) | `gb_13_interactive_group_authoring_ui_end_to_end`, `gb_13_layer_group_labels_and_toolbar_insert_trigger`: menus, inspector, timeline folder row + `"Button"` pill, trigger editor/list, layer group labels, and 1-step undo chains |
| `GB-14` | S13 | `coursecraft-ui-egui` (`uiux_sweep_tests.rs`) | `gb_14_m37_point_text_and_interactive_group_uiux_sweep`: full M28 UI/UX matrix across point text and interactive groups (`0` MUST violations) |

## Driver Gates

- **`cargo xtask ci`:**
  - `g1` (after S3): passed on attempt 1 (`3,964 passed; 0 failed; 16 ignored`, `layers: ok`, all 17 WASM crates checked).
  - `g2` (after S6): passed on attempt 1 (`3,977 passed; 0 failed; 16 ignored`, `layers: ok`, all 17 WASM crates checked).
  - `g3` (after S9): passed on attempt 1 (`3,985 passed; 0 failed; 16 ignored`, `layers: ok`, all 17 WASM crates checked).
  - `g4` (after S12): passed on attempt 1 (`3,995 passed; 0 failed; 16 ignored`, `layers: ok`, all 17 WASM crates checked).
  - `g5` (final, after S13): passed on attempt 1 (`3,996 passed; 0 failed; 16 ignored`, `fmt --all --check`, `clippy --workspace --all-targets -- -D warnings`, `layers: ok`, all 17 WASM crates checked — including `coursecraft-course` `316`+`2`, `coursecraft-media` `57`, `coursecraft-logic` `265`+`11`, `coursecraft-render` `488`+`13` (+`1` ignored), `coursecraft-format` `90`+`6`, `coursecraft-publish` `50`+`1`, `coursecraft-engine` `388`+`1` (+`3` ignored), `coursecraft-ui-egui` `1,329` (+`11` ignored), `coursecraft-player` `56`, `coursecraft` `196`, `coursecraft-cli` `58`+`63` (+`1` ignored), `coursecraft-mcp` `10`).
- **`cargo xtask build-player`:**
  - `p1` (after `g1`): passed (`wasm-release` finished in `1m 13s`).
  - `p2` (after `g3`): passed (`wasm-release` finished in `1m 14s`).
  - `p3` (after S10): passed (`wasm-release` finished in `1m 07s`).
  - `p4` (final, after `g5`): passed (`wasm-release` finished in `0.24s`).
- **Release editor perf suite (`perf.log` — `coursecraft-ui-egui::perf_tests`, `--release --ignored`):** passed (`9 passed; 0 failed; 0 ignored; 1305 filtered out` in `2.40s`).

## Measured Performance (`perf.log`) vs. Baseline

`docs/testing/m37-plan.md` §8.1 verifies zero editor hot-path regression against the budgets in [`docs/performance.md`](../performance.md). Below are the exact measured numbers from `/Users/aditya/Documents/Claude Projects/ScormCraft/target/m37/perf.log` compared against those budgets and the preceding M36 baseline in [`docs/testing/m36-report.md`](m36-report.md):

| Stage (300-slide course, 1280×900) | Budget (`docs/performance.md`) | Pre-M37 (`m36-report.md`, median / p95) | Measured After M37 (`perf.log`, median / p95) | Notes |
|---|---:|---:|---:|---|
| `idle` | 16 ms | 0.40 ms / 0.41 ms | **0.40 ms / 0.41 ms** | Flat; well under budget |
| `select slide 150` | 33 ms | 0.63 ms / 0.78 ms | **0.63 ms / 0.78 ms** | Flat; well under budget |
| `scroll slide list` | 33 ms | 0.36 ms / 0.38 ms | **0.36 ms / 0.38 ms** | Flat; well under budget |
| `typing in a text field` | 33 ms | 0.53 ms / 0.54 ms | **0.53 ms / 0.55 ms** | Flat; well under budget |
| `type on canvas with spell check` | 33 ms | 0.71 ms / 0.72 ms | **0.71 ms / 0.72 ms** | Flat; well under budget |
| `check spelling, open` | 250 ms | 30.09 ms | **29.47 ms** | Well under 250 ms budget |
| `check spelling, ignore step` (20 issues) | 33 ms | 0.43 ms / 0.57 ms | **0.44 ms / 0.61 ms** | Flat; well under budget |
| `insert interaction + undo` (8 kinds) | 100 ms | 1.17 ms / 1.65 ms | **1.16 ms / 1.64 ms** | Well under 100 ms budget |
| `undo / redo move` (30 cycles) | 33 ms | 2.54 ms / 2.86 ms | **2.58 ms / 2.90 ms** | Flat; well under budget |
| `first frame` | 500 ms | 8.32 ms | **8.82 ms** | Well under 500 ms budget |
| `visit every slide` | 33 ms | 0.43 ms / 0.69 ms | **0.43 ms / 0.70 ms** | `rss 263 -> 264 MB` |
| `visit every slide, real pictures` | 33 ms | 0.43 ms / 0.70 ms | **0.44 ms / 0.71 ms** | `rss 269 -> 271 MB, media 0.2 MiB, 1 decoded pictures` |
| `low-end lite 300 slides` | 33 ms | 14.23 ms / 34.56 ms | **14.18 ms / 34.81 ms** | `media 4.4 MiB, undo 0.0 KiB` (median well under 33 ms; p95 flat vs. pre-M37) |

No editor hot-path regression was introduced by M37.

### Plan §8.1 Benchmark Not Executed by Driver

The driver's `do_perf` step ran only `coursecraft-ui-egui::perf_tests` (`perf.log`). One additional release benchmark listed in `docs/testing/m37-plan.md` §8.1 remains unrun by the driver:

- `cargo test --release -p coursecraft-engine undo_perf_tests -- --ignored --nocapture --test-threads=1` (verifies `SlideDelta` with `Slide.groups` keeps undo memory and timing within budget)

## Skipped Steps & Open Concerns

- **Skipped steps:** None — `/Users/aditya/Documents/Claude Projects/ScormCraft/target/m37/SKIPPED` does not exist. All 13 steps (`S1`–`S13`) completed, passed review (`11` `OK`, `2` `FIXED` in `S3` and `S11` with regression tests committed), and passed every gate on attempt 1.
- **Open concerns from `/Users/aditya/Documents/Claude Projects/ScormCraft/target/m37/REVIEW.md`:**
  1. **Named text styles on point text (S4 note, outside `s4` scope):** `text_style.apply` and `text_style.update` (`crates/engine/src/commands/styles.rs` → `apply_named_style`) overwrite `style.wrap` without refitting point text; consider handling if named text styles are applied to point-text labels.
  2. **Duplicating a slide with interactive-group triggers (S11 note, outside `s11` scope):** `copy_slide` (`crates/engine/src/commands/slide.rs:1833-1906`) remaps triggers using `object_map` without including `slide.groups` / `layer.groups` IDs, so `slide.duplicate` currently skips triggers targeting interactive groups.
  3. **Minor test-scoping note (S13):** `report.painted("Button")` / `report.painted("Back button")` in `gb_14_m37_point_text_and_interactive_group_uiux_sweep` check whole-frame text rather than scoping strictly to `timeline.group_row_rects` (timeline row painting is independently covered by `gb_13`, and the full-frame layout/contrast/hit-target checks still audit the painted row).
- **Pending release benchmark:** Run `cargo test --release -p coursecraft-engine undo_perf_tests -- --ignored --nocapture --test-threads=1` from `docs/testing/m37-plan.md` §8.1 before tagging if full §8.1 sign-off is desired.

## Owner Manual Verification Checklist (`docs/testing/m37-plan.md` §8.2)

### Issue #7 — Point text vs area text

- [ ] **Click places fitted point text:** Select the Text box tool, click once on the stage, and type `"Next"`. Confirm the box is one line and only as wide as the text plus padding (no 300×60 excess box).
- [ ] **Center-align without manual resize:** Place two clicked labels (`"Next"` and `"Back"`) and center-align them onto buttons. Confirm they center accurately without resizing either box first.
- [ ] **Drag places fixed-width wrapping area text:** Drag with the Text box tool to draw a box and type a long sentence. Confirm the box keeps the dragged width and wraps cleanly.
- [ ] **Convert point text to area text:** On a clicked point-text box, turn **Wrap text** on (and in a second box, drag the side width handle). Confirm both convert to fixed-width wrapping area text.
- [ ] **State Editor placement:** Enter **Edit states**, click with the Text box tool to add a state-extra label. Confirm it places fitted point text; drag places wrapping area text.
- [ ] **Undo parity:** Undo typing (`Ctrl+Z` / `⌘Z`) restores `"Text"` and its original fitted width in one step; a second Undo deletes the text box in one step.
- [ ] **Existing courses unchanged:** Open `examples/showcase/course.json`, save without editing (or inspect diff), and confirm existing text boxes keep exact frames, wrapping, and byte-identical JSON.

### Issue #8 Part B — One state for a grouped button

- [ ] **Make a button from a group:** Group a rectangle, an icon, and a `"BACK"` text box (`Ctrl+G` / `⌘G`), give each member a `Hover` and `Down` look (e.g. dark fill + white icon + white text), and turn on **Act as one button** (or **Arrange ▸ Make button from group**).
- [ ] **Shared Hover & Down across all members:** Open **Preview**. Move the pointer over the rectangle, then onto the icon, then onto `"BACK"`. Confirm **all** members stay in `Hover` together without flicker, and pressing on any member puts **all** members in `Down`. Confirm a member without a `Hover` state stays `Normal`.
- [ ] **Shared Selected, Visited & Disabled:** Add a `Selected` (or `Visited`) state to members and test clicking, plus a trigger setting the group to `Disabled`. Confirm all members change together and clicking a disabled group does nothing.
- [ ] **Click trigger fires once:** Attach one click trigger to the group (e.g. increment a counter variable or jump slide). Click the rectangle, icon, or text — confirm the trigger fires **once**, not three times.
- [ ] **Keyboard focus & focus ring:** Press `Tab` in Preview. Confirm focus reaches the grouped button **once** (never individual members), the focus ring surrounds the whole group bounds, and `Enter` / `Space` activates it.
- [ ] **Published SCORM/Web parity:** Publish to Web or SCORM, open in a browser, and verify hover/down/click/Tab/Enter/Space and screen-reader button name behave identically to Preview.
- [ ] **Spec & CI:** Confirm `docs/spec/triggers.md` documents the interactive-group state rule, `docs/spec/course-model.md` documents `point_text` and `Group`, and `cargo xtask ci` passes.
