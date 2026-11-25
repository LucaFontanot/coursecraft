# Live Text Box Resizing While Typing (0.5.1)

## Bug Report (0.5.0)
> "Since the default text box size change (point text, M37), when we edit the text and type or delete any text, the box doesn't auto-resize horizontally or vertically. It stays the same until we click outside and select the text box again."

## Root Causes Found

Reproducing the exact owner flows in headless egui UI tests (`crates/ui-egui/src/text_resize_tests.rs`) — canvas click with `PlacementTool::TextBox`, ribbon **Insert › Text Box** in the 0.5.0 ribbon + docked-panel workspace, and double-clicking an existing text box with a border outline — uncovered five distinct bugs:

1. **Resize handles and rotate handle ignored `inline_edit_live_rect` (`crates/ui-egui/src/canvas.rs` `draw_handles_and_outline`):**
   - The selection-outline loop substituted `editor.ui.inline_edit_live_rect` into `active_item.rect`, **but** the handle lookup (`single_item`) queried `items` directly (the stored pre-edit `item.rect`). As a result, all 8 white square resize handles, the rotate stem/handle, and handle hover cursors remained frozen at the pre-edit stored frame until commit.
   - Before fix (reproduction test output):
     - Canvas click: `handles vs selection outline after typing: rects disagree (tol=2.5): [[389.3 249.2] - [415.2 264.0]] vs [[389.3 249.2] - [515.5 264.0]]` (handles stayed at 25.9 px width while outline grew to 126.2 px).
     - Ribbon Insert › Text Box (docked layout): `ribbon-placed handles vs outline: rects disagree (tol=2.5): [[555.9 282.2] - [583.9 298.2]] vs [[555.9 282.2] - [688.3 298.2]]`.
2. **Painted text box outline/border ignored `inline_edit_live_rect` (`crates/ui-egui/src/canvas.rs` `show_canvas_and_layers`):**
   - `paint(..., view_items, ...)` painted `cached_items` directly (`hide_edited_text` cleared `runs` while keeping `Visual::Text { outline }`), so any border/outline on the edited text box remained frozen at the stored frame until commit.
3. **1-frame lag and stale `TextEdit` rect on every keystroke (`draw_inline_edit` & `show_canvas_and_layers`):**
   - `paint` and `draw_handles_and_outline` ran *before* `draw_inline_edit`, and `draw_inline_edit` sized the dashed border and `egui::TextEdit` from the pre-keystroke `edit.buffer`, only updating `inline_edit_live_rect` and calling `ctx.request_repaint()` *after* `TextEdit::show`. On every input frame (and on initial entry), the outline, handles, dashed border, and `TextEdit` painted at the previous size.
4. **Zoomed-out `TextEdit` minimum height mismatch (`draw_inline_edit`):**
   - `egui::TextEdit` enforces `ui.spacing().interact_size.y` (26 px) as a minimum height. On a zoomed-out slide (e.g. ~55% scale where a 20 pt line is ~14.8 screen px), the inline `TextEdit` widget rect extended ~11 px below the selection outline and handles (`[[389.3 249.2] - [515.5 275.2]]` vs `[[389.3 249.2] - [515.5 264.0]]`).
5. **Fixed-width wrapping area text had no live-grow mode (`Autofit` only had `None` and `Shrink`) and empty trailing lines used `FALLBACK_SIZE`:**
   - Dragging a Text Box (or editing a wrapping area text box) created `point_text: false, autofit: Autofit::None`, for which `draw_inline_edit` cleared `inline_edit_live_rect = None`. Typing or pressing Enter never resized the box vertically.
   - `split_paragraphs` in `crates/render/src/text_layout.rs` populated empty paragraphs (such as a trailing `\n` after pressing Enter) with hardcoded `FALLBACK_SIZE` (20 pt) instead of inheriting the active run's font/size.
   - Editing `Text content` in the Text panel / Format inspector while inline edit was yielded to the panel did not sync the active `inline_edit` buffer/live rect.

## Fix Summary

### 1. Live point-text & Grow-text canvas rendering (`crates/ui-egui/src/canvas.rs`, `crates/render/src/text_layout.rs`, `crates/ui-egui/src/properties.rs`)
- Added `sync_inline_edit_live_rect_before_paint` and `apply_inline_edit_live_rect` in `canvas.rs` so `view_items` carries the live rect *before* both `paint(...)` (painted border/outline) and `draw_handles_and_outline(...)` (2 px selection outline, all 8 resize handles, rotate handle + stem, hover cursors).
- Updated `single_item` in `draw_handles_and_outline` (and its hit-test helper) to apply `inline_edit_live_rect` directly as well.
- Updated `update_inline_edit_live_bounds` to call `ctx.request_discard("inline text box live resize")` (plus `ctx.request_repaint()`) whenever a keystroke changes the live rect, eliminating the 1-frame lag so every element — painted border, selection outline, all 8 resize handles, dashed edit border, and `TextEdit` rect — matches on the very frame input arrives.
- Cleared `child.spacing_mut().interact_size = egui::Vec2::ZERO` inside `draw_inline_edit` for live-resizing boxes so `TextEdit`'s widget rect matches the scaled text box height at every zoom level.
- Cleared `edit.just_entered` immediately when a keystroke modifies the buffer so typing on an entry pass cannot re-select newly typed text.
- Updated `split_paragraphs` in `crates/render/src/text_layout.rs` so empty paragraphs inherit the preceding run's font, size, bold, italic, baseline, and letter spacing.
- Synced `editor.ui.inline_edit` when `text.set` runs from the Text panel / Format inspector (`properties.rs` `finish`).

### 2. New `Autofit::Grow` ("Resize box to fit text") mode
- **Model (`crates/course/src/model.rs`, `docs/spec/course-model.md`):** Added `Autofit::Grow` (`"grow"` in JSON). `Autofit::None` remains `#[default]` and omitted during serialization so all existing courses serialize byte-identically.
- **Headless wrapped measurement (`crates/media/src/text_metrics.rs`):** Added pure-Rust, WASM-safe, hostile-input-clamped `measure_wrapped_text_height` and `grow_text_box_height` using `skrifa` against the bundled Inter faces (width fixed, wraps on word boundaries and breaks overlong words by character, never smaller than one line).
- **Engine (`crates/engine/src/commands/{text.rs, object.rs, styles.rs, object_states.rs, mod.rs}`):** Added `fit_grow_text_frame` (width and `x` unchanged, `y` anchored by `style.valign`, `h` clamped to `1..=10000`). Wired into `object.add`, `text.set`, `text.style_range`, `text.set_paragraph`, `text.set_paragraphs`, `object.set_autofit`, `object.update`, `object.set_frame` (width-only resize), `object.apply_style`, `text_style.apply`/`update`, and state extras (`object.state_add_extra`, `object.state_update_extra`). All operations remain single-step undoable.
- **Editor UI (`crates/ui-egui/src/canvas.rs`, `crates/ui-egui/src/properties.rs`):**
  - New dragged area text boxes (`PlacementTool::TextBox` drag, including via ribbon **Insert › Text Box** and State Editor drag) default to `autofit: "grow"` (passing initial dragged `measured_size` so dragged initial height is preserved until edited).
  - While editing a `Grow` box, height follows wrapped text and newlines live every frame (growing and shrinking, never smaller than one line, width fixed); committing sends `measured_size` in one undo step so the stored frame matches the last live frame.
  - Added `tl!("Resize box to fit text")` to the `Autofit` combo in the Format inspector and Text panel alongside `Off` and `Shrink text on overflow`. `None` and `Shrink` keep their existing behaviour; play-time rendering is unchanged (the player draws the committed frame).

## Tests Added & Verification

### Before vs. After
- **Before fix:** All 3 reproduction tests in `crates/ui-egui/src/text_resize_tests.rs` failed:
  - `repro_point_text_canvas_click_live_resize_outline_handles_and_edit_rect`: failed because the 8 resize handles stayed frozen at `[389.3..415.2]` while the selection outline grew to `[389.3..515.5]`, and because `TextEdit` height was forced to 26 px instead of 14.8 px.
  - `repro_ribbon_insert_text_box_in_docked_layout_resizes_live`: failed with handles frozen at `[555.9..583.9]` vs outline `[555.9..688.3]`.
  - `repro_double_click_existing_point_text_resizes_drawn_border_handles_and_anchors_center`: failed because the painted `#E11D48` box border and resize handles stayed at the stored frame.
- **After fix:** All 5 tests in `text_resize_tests` pass:
  - `repro_point_text_canvas_click_live_resize_outline_handles_and_edit_rect` (checks immediate input frame & settled frame, Enter vertical growth, Backspace horizontal+vertical shrink, handles, selection outline, and AccessKit `TextEdit` rect).
  - `repro_ribbon_insert_text_box_in_docked_layout_resizes_live` (real 0.5.0 ribbon + docked-panel layout).
  - `repro_double_click_existing_point_text_resizes_drawn_border_handles_and_anchors_center` (verifies painted outline mesh, center anchoring, handles, and 1-step commit matching last live rect).
  - `area_text_drag_defaults_to_grow_and_resizes_vertically_live` (ribbon drag -> `Autofit::Grow`, live vertical growth & shrink with fixed width on immediate input frame, 1-step commit).
  - `inspector_autofit_combo_offers_grow_and_refits_existing_area_text` (inspector combo offers Off / Shrink / Resize box to fit text; switching refits height in 1 undo step).

### Additional Tests
- **Backward-compatibility golden (`crates/course/src/tests.rs`):** `autofit_grow_roundtrip_and_existing_examples_byte_identical` verifies `hello-course`, `showcase`, `healthy-habits`, `reference-course`, and `rich-states` round-trip byte-identically without emitting `"grow"`, plus `Autofit::Grow` serde round-trip.
- **Headless wrapped measurement (`crates/media/src/text_metrics.rs`):** `wrapped_height_grows_as_width_narrows_and_never_below_one_line` and `wrapped_height_hostile_inputs_clamped_without_panic`.
- **Engine Grow suite (`crates/engine/src/tests.rs`):** `autofit_grow_add_text_set_paragraphs_and_undo` and `autofit_grow_apply_style_set_autofit_and_hostile` cover `object.add`, `text.set`, `text.set_paragraphs`, `object.apply_style`, `object.set_autofit`, fixed width, 1-step undo, empty-text one-line floor, and hostile input clamping.
- **Existing suites:** All `pt_*`, `question_panel_tests`, `uiux_sweep_tests`, `cargo xtask ci`, and `cargo xtask build-player` pass.
