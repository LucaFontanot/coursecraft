# M30 Milestone Report

**Date:** 2026-11-23  
**Scope:** Part A (Slide Size, presets, default persistence, off-slide warnings, 5-surface discoverability), Part B (`BUG-01`–`BUG-08` fixes), Part C (Glossary Find/Replace/Spell Check/XLIFF, `glossary.sort`, Timeline & View menu bulk Show all / Unlock all).

---

## 1. Bug Dispositions (`BUG-01`–`BUG-08`)

| ID | Severity | Status | Fix Summary | Verified By |
|---|---|---|---|---|
| **BUG-01** | High (Data Loss) | Fixed (S6) | `object.patch_state` (`reset_all: true`) preserves `StateDef.extras` via `std::mem::take` while resetting overrides and `clear_*` flags (`crates/engine/src/commands.rs`). | `B-01` (`b01_patch_state_reset_all_preserves_state_extras_and_undoes_cleanly`) |
| **BUG-02** | High (Web Blocker) | Fixed (S8) | Added `should_hide_dom_overlays` in `apps/coursecraft-player/src/app.rs` and `hide_overlays` gating in `WebObjects::layout` (`web_object.rs`) and `MediaDriver::layout_videos_with_visibility` (`crates/render/src/media_driver.rs`). | `B-07`, `B-08` |
| **BUG-03** | High (Visual Corruption) | Fixed (S1a/S1b) | `crates/engine/src/slide_size.rs` scales all `StateDef` overrides, parent-relative `StateExtra` frames/effects/kinds, and previously missed base fields (`Text.outline`, `Image.outline`, `Path.text`, `Timer.style`, `Marker.runs/paragraphs`, `insets`, `letter_spacing`, `bullet_size`, `NamedTextStyle`) with `RISK-02` clamps. | `A-01`–`A-05`, `FZ-01` |
| **BUG-04** | Medium (Stale UI) | Fixed (S7) | Widened `PropertiesState::extra_cache_key` in `crates/ui-egui/src/properties.rs` to `(rev, slide_id, object_id, state_name, extra.id)`. | `B-04` (`b04_extra_inspector_cache_key_includes_state_name`) |
| **BUG-05** | Medium (Broken Workflow) | Fixed (S7) | Inline state rename in `crates/ui-egui/src/states.rs` updates `editor.ui.editing_state.state_name` alongside `preview_state`. | `B-05` (`b05_renaming_active_state_in_states_tab_keeps_edit_state_mode`) |
| **BUG-06** | Medium (Wrong Rendering) | Fixed (S6) | `crates/render/src/compose.rs` & `paint.rs` (`resolve_pointer_fill`): while pressed, `down` wins when it defines a fill (`down_fill` or `down_has_fill_style`); otherwise `hover` solid fill keeps showing. | `B-03` (`b03_down_fill_style_wins_over_hover_solid_and_no_down_fill_keeps_hover_solid`) |
| **BUG-07a** | Medium (Incomplete M29) | Fixed (S6) | `ObjectKind::Timer` in `crates/render/src/compose.rs` resolves `state_overrides.text_style.unwrap_or(style)`. | `B-02` (`b02_timer_honours_state_text_style`) |
| **BUG-07b** | Medium (Incomplete M29) | Fixed (S7) | `crates/ui-egui/src/canvas.rs` uses `world_to_extra_frame` / `world_to_extra_rotation` on placement, `state_extra_world_geometry` for `inline_edit_slide_rect`, and skips `is_state_extra` in `apply_rotate_preview`. | `B-06` (`b06_state_extra_placement_inline_edit_and_rotate_preview_respect_parent_transform`) |
| **BUG-08a** | Medium (Runtime) | Fixed (S9) | `seek_base` in `crates/logic/src/run.rs` calls `self.note_timeline_completed()` when forward seek reaches `duration_ms` under `seekbar: free`; documented in `docs/spec/triggers.md`; Course Settings Seekbar combo gated (`UX-08`). | `B-11`, `B-12`, `FZ-02` |
| **BUG-08b** | Medium (Chrome) | Fixed (S8) | `PlayerFrame::show` (`crates/render/src/player_frame.rs`) gates `handle_keyboard` on `!gated` and wraps `startCourse` with `painter.layout` in `draw_start_gate`. | `B-09`, `B-10` |

---

## 2. Test IDs Implemented (`A-01`–`A-18`, `B-01`–`B-12`, `C-01`–`C-09`, `FZ-01`–`FZ-03`)

- **Part A (`A-01`–`A-18`):**
  - `A-01`–`A-04`: `crates/engine/src/tests.rs` (`set_slide_size_fill_uses_max_factor_and_centres_overhang`, `set_slide_size_scale_scales_states_and_state_extras`, `set_slide_size_scale_scales_previously_missed_base_fields`, `set_slide_size_extreme_aspects_and_clamps_stay_valid`).
  - `A-05`: `crates/engine/src/m27_all_commands_undo_tests.rs` (`set_slide_size_fill_undo_redo_and_cli_docs_sync`).
  - `A-06`, `A-07`: `crates/course/src/tests.rs` (`off_slide_object_warnings_cover_base_layer_master_and_exclusions`, `all_bundled_examples_emit_zero_off_slide_warnings`).
  - `A-08`: `crates/render/src/tests.rs` (`question_and_player_and_offscreen_succeed_across_all_aspect_presets_and_tiny`), `crates/export/src/tests.rs` (`pdf_and_docx_export_succeed_at_all_aspect_presets_and_tiny`, `mp4_export_succeeds_at_all_aspect_presets_and_tiny`).
  - `A-09`: `apps/coursecraft/src/tests.rs` (`preferences_default_slide_backward_compatible_and_custom_sizes`).
  - `A-10`–`A-18`: `crates/ui-egui/src/tests.rs` (`format_aspect_ratio_unit_matrix`, `slide_size_dialog_preset_combo_and_custom_switching`, `slide_size_dialog_aspect_lock_keeps_and_updates_ratio`, `slide_size_dialog_use_as_default_emits_request_and_disables_when_already_default`, `slide_size_dialog_live_preview_and_course_wide_impact_counts`, `slide_size_dialog_content_modes_dispatch_and_undo`, `preferences_open_from_the_menu_and_changes_are_requested`, `slide_size_dialog_opens_from_all_five_entry_points`, `check_course_off_slide_warning_navigates_to_stranded_object`).
- **Part B (`B-01`–`B-12`):**
  - `B-01`: `crates/engine/src/m29_state_commands_tests.rs`.
  - `B-02`, `B-03`: `crates/render/src/m29_state_resolver_tests.rs`.
  - `B-04`, `B-06`: `crates/ui-egui/src/m29_state_extras_ui_tests.rs`.
  - `B-05`: `crates/ui-egui/src/m29_edit_state_mode_tests.rs`.
  - `B-07`: `crates/render/src/media_driver.rs`.
  - `B-08`: `apps/coursecraft-player/src/tests.rs`.
  - `B-09`, `B-10`: `crates/render/src/player_frame/tests.rs`.
  - `B-11`: `crates/logic/src/tests.rs`.
  - `B-12`: `crates/ui-egui/src/tests.rs`.
- **Part C (`C-01`–`C-09`):**
  - `C-01`: `crates/course/src/strings.rs`.
  - `C-02`, `C-05`: `crates/engine/src/tests.rs` & `crates/engine/src/m27_all_commands_undo_tests.rs`.
  - `C-03`: `crates/format/src/xliff/tests.rs` & `crates/format/tests/xliff_parity.rs`.
  - `C-04`: `crates/ui-egui/src/tests.rs` & `crates/ui-egui/src/spell_check_tests.rs`.
  - `C-06`, `C-07`, `C-08`: `crates/ui-egui/src/tests.rs`.
  - `C-09`: `crates/ui-egui/src/uiux_sweep_tests.rs`.
- **Hostile-Input & Regression (`FZ-01`–`FZ-03`):**
  - `FZ-01`: `crates/engine/src/tests.rs` (`panic_hunt`).
  - `FZ-02`: `coursecraft-cli simulate` on `examples/hello-course`, `examples/showcase`, `examples/reference-course`, and `examples/rich-states` (all exited 0).
  - `FZ-03`: `apps/coursecraft/src/tests.rs` (`preferences_default_slide_hostile_inputs_never_panic`).

---

## 3. UI/UX Sweep Results

- `cargo test -p coursecraft-ui-egui -j 4 uiux_sweep`: **3 passed; 0 failed** (`uiux_sweep_modals_and_tooltips`, `uiux_sweep_scale_2_and_fixtures`, `uiux_sweep_all_surfaces`).

---

## 4. Release Gate & Performance (Driver-Run)

- `cargo xtask build-player`: *Pending driver run.*
- `cargo xtask ci`: *Pending driver run.*
- `cargo xtask perf`: *Pending driver run.*
