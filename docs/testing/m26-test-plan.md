# M26 Test Plan: Aurelie's Feedback (A1–A13) & Cross-Cutting Regression Suite

This plan specifies concrete, automated Rust and CLI tests for every item in [`docs/user-feedback/aurelie-v02-issues.md`](../user-feedback/aurelie-v02-issues.md) (**A1–A13**), plus **Cross-Cutting Checks** spanning undo/redo, persistence, published web player parity, multi-select, layers/states, window sizes, and focus/keyboard routing.

Several items in round 2 had been marked fixed once before and still failed for a real user. Every planned test therefore drives the **real user path** (`TestHarness` pointer/keyboard events in `coursecraft-ui-egui`, `Player` in `coursecraft-logic`, `render_slide_rgba` / `render_player_rgba` / `compose` in `coursecraft-render`, `import_pptx` in `coursecraft-pptx`, and `coursecraft-cli`), and tests **the same class of bug from multiple other angles** rather than repeating only the reported reproduction.

> **Conventions for the implementer**
> - **Never violate Golden Rules**: no `unwrap`/`expect`/`panic!` in non-test code (test modules may use `expect`), keep `cargo xtask layers` and `cargo xtask wasm` green, and write all CLI / rendered test outputs under `target/` (never into the repo tree).
> - **Priority scale**:
>   - **P1** — Normal everyday authoring or learner use; must pass before shipping.
>   - **P2** — Common workflow or frequent edge case.
>   - **P3** — Rare / extreme / hostile input or boundary condition.
> - **Recommended test file locations**:
>   - UI harness tests: `crates/ui-egui/src/` (new modules such as `m26_qa_a1_a2_tests.rs`, `m26_qa_a4_a5_a6_tests.rs`, `m26_qa_a7_a9_a10_tests.rs`, `m26_qa_a11_a12_tests.rs`, `m26_qa_crosscut_tests.rs`, registered under `#[cfg(test)]` in `crates/ui-egui/src/lib.rs`, or extending the existing per-area test modules).
>   - Runtime tests: `crates/logic/src/tests.rs` (or `crates/logic/tests/m26_exit_transitions.rs`).
>   - Render & offscreen parity tests: `crates/render/src/tests.rs` (or `crates/render/tests/m26_render_qa.rs`).
>   - PPTX importer unit tests: `crates/pptx/src/m26_fidelity_tests.rs`.
>   - PPTX visual-fidelity + CLI + Windows packaging integration tests: `apps/coursecraft-cli/tests/m26_pptx_fidelity.rs`, `apps/coursecraft-cli/tests/m26_cli_and_packaging.rs` (`coursecraft-cli` is exempt from `xtask layers` and already depends on `coursecraft-pptx`, `coursecraft-render`, `coursecraft-publish`, `coursecraft-format`, `coursecraft-logic`, `coursecraft-engine`, and `coursecraft-ui-egui`; add `coursecraft-media` under `[dev-dependencies]` if decoding reference PNGs directly).
>   - Native video hostile-input tests: `crates/playback/src/tests.rs`.

---

## Summary Index

| Area | Planned Cases | P1 | P2 | P3 | Primary Test Surface |
|---|---:|---:|---:|---:|---|
| [A1 — Text format panel](#a1-text-format-panel) | 10 | 5 | 4 | 1 | `TestHarness` (`properties.rs`), `render_slide_rgba` |
| [A2 — Gradients](#a2-gradients) | 11 | 6 | 4 | 1 | `TestHarness`, `render_slide_rgba`, `Player`, CLI `publish` |
| [A3 — Exit animations & transitions](#a3-exit-animations--slide-transitions) | 10 | 5 | 4 | 1 | `Player` (`logic`), `compose` / `render_slide_rgba`, CLI `simulate` |
| [A4 — Timeline scrubbing & preview](#a4-timeline-scrubbing--canvas-preview) | 10 | 5 | 4 | 1 | `TestHarness` (`timeline.rs`, `canvas.rs`) |
| [A5 — Shape drawing drag preview](#a5-shape-drawing-drag-preview) | 9 | 4 | 4 | 1 | `TestHarness` (`canvas.rs`, `shape_gallery.rs`) |
| [A6 — Dialog reopening at full size](#a6-dialog-reopening-at-full-size) | 10 | 5 | 4 | 1 | `TestHarness` (`dialog.rs`, all `Dialog::new` dialogs) |
| [A7 — Video selection & decode safety](#a7-video-selection--decode-safety) | 10 | 5 | 3 | 2 | `TestHarness` (`timeline_playback.rs`), `coursecraft-playback` |
| [A8 — Windows installer & portable packaging](#a8-windows-installer--portable-packaging) | 9 | 4 | 3 | 2 | Static/structural Rust tests, `coursecraft.iss`, `release.yml`, Windows `iscc` |
| [A9 — Slide layers Base slide view](#a9-slide-layers-base-slide-view-show--dim--hide) | 10 | 5 | 4 | 1 | `TestHarness` (`canvas.rs`, `layers.rs`), `Player`, `render_slide_rgba` |
| [A10 — Question editor in right panel](#a10-question-editor-in-right-panel) | 10 | 5 | 4 | 1 | `TestHarness` (`question.rs`, `properties.rs`) |
| [A11 — Format painter (Paint)](#a11-format-painter-paint) | 10 | 5 | 4 | 1 | `TestHarness` (`format_painter.rs`, `canvas.rs`), `engine` |
| [A12 — Preview modal closing](#a12-preview-window-closing) | 10 | 5 | 4 | 1 | `TestHarness` (`preview.rs`) |
| [A13 — PowerPoint import & fidelity suite](#a13-powerpoint-import--fidelity-suite) | 10 | 5 | 3 | 2 | `apps/coursecraft-cli/tests/m26_pptx_fidelity.rs`, `crates/pptx`, CLI |
| [Cross-cutting checks](#cross-cutting-checks) | 8 | 5 | 3 | 0 | Multi-crate harness, `Player`, `format`, CLI `publish`/`diff` |
| **Total** | **137** | **69** | **52** | **16** | |

---

## A1. Text format panel

**Bug class:** Numeric inspector controls (`DragValue` / `TextEdit`) resetting typed values or ignoring drags because per-frame sync overwrites local edits, or missing `Enter` / `lost_focus` / `drag_stopped` commit paths across different text-bearing object kinds and editing modes.

### A1.1 — Timer object `Size` field via typing + Enter, click-away, and drag
- **What it checks:** Editing the numeric `Size` `DragValue` on a `Timer` object (`crates/ui-egui/src/properties.rs`, around lines 5461–5469) commits on Enter, on click-away, and on drag.
- **Why it might break:** `properties.rs:5462–5469` binds `DragValue::new(&mut props.text_size)` and only checks `if r.drag_stopped() || r.lost_focus()`—it omits `enter` (`r.has_focus() && ui.input(|i| i.key_pressed(Key::Enter))`) and lacks a `text_size_seen` guard, which is the exact same pattern that broke A1 on text boxes.
- **Where / how:** `crates/ui-egui/src/text_format_tests.rs` using `TestHarness::new`.
- **Setup & steps:**
  1. Build a course with one `ObjectKind::Timer` (`id: "t1"`, `style.size: 24.0`).
  2. Select `"t1"`, set `InspectorTab::Format`, run two `idle_frame()`s.
  3. Find the `"Size"` label in `right_panel_rect()`, click the `DragValue` to its right, type `"40"` and press `Enter` (`set_field`). Assert `timer.style.size ≈ 40.0`.
  4. Click the field, type `"52"`, click away to panel header. Assert `timer.style.size ≈ 52.0`.
  5. `drag_by` the field by `vec2(40.0, 0.0)` over 6 steps. Assert `timer.style.size` changed by `> 1.0`.
- **Expected result:** All three interactions update `Timer.style.size` and push one undo step each.
- **Priority:** P1

### A1.2 — Table cell font size, letter spacing, and line spacing
- **What it checks:** Selecting a table cell and changing `Font size`, `Spacing` (letter spacing), and paragraph `Spacing` (line spacing) via typing + Enter, click-away, and drag.
- **Why it might break:** Table cells sync `props.text_size` separately (`properties.rs:4646`) and dispatch `text.style_range` / `text.set_paragraphs` with `row` and `column` parameters; cell-level sync or commit wiring can silently reset the typed value or apply to the wrong cell.
- **Where / how:** `crates/ui-egui/src/text_format_tests.rs` using `TestHarness::new_with_size(course, (1280.0, 1200.0))`.
- **Setup & steps:**
  1. Build a course with a 2×2 `ObjectKind::Table` (`id: "tbl1"`, cell `(0,1)` text `"Cell B"`).
  2. Select `"tbl1"`, enter cell `(0,1)` (set `editor.selection.table_cell = Some((0, 1))` or click cell `(0,1)` on canvas), open `InspectorTab::Format`.
  3. Use `set_field` on `"Font size"` to `"28"`, `"Spacing"` (letter) to `"2.5"`, then scroll down to paragraph `"Spacing"` combo and drag custom line spacing.
  4. Inspect cell `(0,1)` and cell `(0,0)` in `session.course`.
- **Expected result:** Cell `(0,1)` has size `28.0`, letter spacing `2.5`, and updated `line_spacing`; cell `(0,0)` is unchanged; undo reverts each change.
- **Priority:** P1

### A1.3 — Shape with text: typing + Enter, click-away, and line spacing
- **What it checks:** All numeric text fields on `ObjectKind::Shape { text: Some(_), .. }` accept typed + Enter, typed + blur, and line-spacing edits (existing test only drags size/spacing).
- **Why it might break:** Shape text shares `show_text_format` but embeds `ShapeText` inside `ObjectKind::Shape`; typing without dragging or editing line spacing could drop `paragraphs` or fail to sync `eff_size` when `runs` is empty vs non-empty.
- **Where / how:** `crates/ui-egui/src/text_format_tests.rs` using `TestHarness::new_with_size(..., (1280.0, 1400.0))`.
- **Setup & steps:**
  1. Create a rectangle shape `"badge"` with `ShapeText { text: "Line 1\nLine 2", style.size: 18.0 }`.
  2. Select `"badge"`, `InspectorTab::Format`.
  3. Type `"34"` + Enter in `"Font size"`; type `"2.0"` + click away in `"Spacing"`; open line spacing combo (`"1.00x"`) and pick `"1.5x"`, then drag `"custom"`.
- **Expected result:** `badge`'s `ShapeText` reflects size `34.0`, letter spacing `2.0`, and custom line spacing `!= 1.5`.
- **Priority:** P1

### A1.4 — Multi-select mixed objects (two Text boxes with different sizes, and Text box + Shape with text)
- **What it checks:** Multi-select `Size` field (`prop_multi_text_size`, `properties.rs:2363–2403`) displays `"Mixed"` hint when sizes differ, updates all selected `Text` boxes on Enter and on click-away, rejects `0`/`-5`/`abc`, and tests whether a `Shape` with text selected alongside a `Text` box is updated or handled consistently.
- **Why it might break:** `properties.rs:2383–2388` only matches `ObjectKind::Text`, so selecting a `Text` box + a `Shape` with text either hides the text section or silently skips the shape. Also, if one text box already has per-run `size` overrides, `multi_text_size` must update runs too.
- **Where / how:** `crates/ui-egui/src/text_format_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Add `"t1"` (size 48 with a run at size 20), `"t2"` (size 24), and `"s1"` (Shape with `ShapeText` size 18).
  2. `selection.set_many(["t1", "t2"])`; verify `"Mixed"` hint is shown in `Size`; type `"36"` and click away (not Enter). Assert both `"t1"` (all runs) and `"t2"` become `36.0` in a single revision or clean undoable batch.
  3. Type `"-10"` and `"abc"` + Enter — assert sizes remain `36.0`.
  4. `selection.set_many(["t1", "s1"])` — check whether multi-select text controls appear and verify no crash or partial silent corruption.
- **Expected result:** Both `"t1"` (including its run overrides) and `"t2"` update to `36.0` on blur and Enter; invalid strings are ignored; `"t1"+"s1"` behaves cleanly without crashing or dropping data.
- **Priority:** P1

### A1.5 — Inline editing with a partial substring selection
- **What it checks:** Changing `Font size` and `Spacing` while inline editing has only a middle substring selected (e.g. chars `6..11` of `"Hello World"`) changes only that range.
- **Why it might break:** Clicking the inspector `Font size` field takes keyboard focus from the canvas inline edit; if focus loss commits/closes `inline_edit` before `char_range` is read, the entire text box gets resized instead of the selection (or the edit is discarded).
- **Where / how:** `crates/ui-egui/src/text_format_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Select `"title"` (`"Hello World"`, size 24), double-click on canvas to open `inline_edit`, set `inline_edit` selection range to `(6, 11)` (or `Shift+Left` 5 times from end).
  2. Click `"Font size"` field, type `"40"` + Enter.
  3. Click `"Spacing"` field, type `"3.0"` + Enter.
  4. Commit inline edit (`Escape` or click canvas away).
- **Expected result:** `"Hello "` keeps size `24.0` and spacing `0.0`; `"World"` has `size: Some(40.0)` and `letter_spacing: Some(3.0)`.
- **Priority:** P2

### A1.6 — Line spacing custom field via typing + Enter and typing + click-away, plus presets
- **What it checks:** Inside the line-spacing `ComboBox` (`properties.rs:3822–3861`), typing `"1.75"` + Enter into the `custom ` `DragValue`, typing `"2.25"` + clicking away, and picking presets (`1.0x`, `1.15x`, `1.5x`, `2.0x`).
- **Why it might break:** The custom `DragValue` lives inside a `ComboBox` popup; pressing Enter or clicking away can close the popup on the same frame `lost_focus()` fires, dropping `chosen = Some(...)` before `text.set_paragraphs` is queued.
- **Where / how:** `crates/ui-egui/src/text_format_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Select multi-line text object `"body"` (`"Para 1\nPara 2"`).
  2. Scroll right panel to paragraph `"Spacing"` combo (`"1.00x"`), open popup, click `"1.5x"` → assert all paragraphs have `line_spacing == 1.5`.
  3. Reopen combo, `set_field` on `"custom"` to `"1.85"` → assert `line_spacing ≈ 1.85`.
  4. Reopen combo, click `"custom"`, select all, type `"2.2"`, click outside popup → assert `line_spacing ≈ 2.2`.
- **Expected result:** Presets, typed+Enter, and typed+blur all update `paragraphs[*].line_spacing` and persist across idle frames.
- **Priority:** P1

### A1.7 — Extreme, boundary, and hostile numeric inputs
- **What it checks:** Typing boundary and invalid values into `Font size` (`0`, `-20`, `1`, `200`, `9999`, `"abc"`, `""`), `Spacing` (`MIN_LETTER_SPACING`, `MAX_LETTER_SPACING`, `-999`, `999`), and `custom` line spacing (`0.1`, `0.8`, `3.0`, `99.0`).
- **Why it might break:** Clamping mismatch between UI `DragValue::range(...)` and engine command validation (`text.style_range` / `text.set_paragraphs`) can produce rejected commands that leave `props.text_size` out of sync with the document.
- **Where / how:** `crates/ui-egui/src/text_format_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Select `"title"` (size 48).
  2. Sequentially `set_field` on `"Font size"` with `"1"`, `"200"`, `"0"`, `"-5"`, `"9999"`, `"xyz"`.
  3. Sequentially `set_field` on `"Spacing"` with `"-10"`, `"50"`, `"-999"`, `"999"`.
  4. Run `course.validate()` after all edits.
- **Expected result:** Valid min/max values commit (`1.0`, `200.0`, clamped letter spacing); out-of-range values clamp cleanly to the allowed range; `"xyz"` leaves the previous value intact; `course.validate()` has zero errors and `props.text_size` matches the stored value on the next frame.
- **Priority:** P3

### A1.8 — Text box on a Slide Layer and on a Master Slide
- **What it checks:** Typing and dragging `Font size`, `Spacing`, and `Line spacing` on an object inside a slide layer (`about-text` on layer `about`) and on a master slide object.
- **Why it might break:** Commands (`text.style_range`, `text.set_paragraphs`) take `"slide": slide_id` and look up objects across base+layers or masters; layer/master selection sync can accidentally reset `text_size_seen` or fail to locate the object.
- **Where / how:** `crates/ui-egui/src/text_format_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Open `hello-course`, select slide `"s-intro"`, layer `Some("about")`, object `Some("about-text")`, `InspectorTab::Format`.
  2. `set_field` `"Font size"` to `"26"`, `set_field` `"Spacing"` to `"1.5"`. Assert `about-text` updated.
  3. Add a master slide via `master.add`, add a text object on it, select it, repeat `Font size` = `"30"` and `Spacing` = `"2.0"`.
- **Expected result:** Both layer text object and master slide text object update and survive undo/redo.
- **Priority:** P2

### A1.9 — Rapid selection switch while a numeric field has focus (cross-object bleed check)
- **What it checks:** Focusing `"Font size"` on text box A (size 48), typing `"72"` WITHOUT pressing Enter, then immediately clicking text box B (size 18) on the canvas.
- **Why it might break:** On the click frame, `r_size.lost_focus()` fires; if `selection.object` already changed to B before `show_text_format` runs, the blur commit could apply `"72"` to B instead of A, or leave `props.text_size = 72` stuck on B!
- **Where / how:** `crates/ui-egui/src/text_format_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Course with two non-overlapping text boxes `"t1"` (size 48) and `"t2"` (size 18).
  2. Select `"t1"`, click `"Font size"`, `Cmd+A`, `type_text("72")` (no Enter).
  3. Click `"t2"` directly on the canvas; run 2 `idle_frame()`s.
- **Expected result:** `"t2"` MUST still have size `18.0` (never overwritten with `72.0`), and the inspector `"Font size"` field displays `18` when `"t2"` is selected.
- **Priority:** P2

### A1.10 — End-to-end rendering, save/reopen, and CLI `render` parity for numeric text properties
- **What it checks:** Edited `Font size`, `letter_spacing`, `line_spacing`, and M26 `justify`/`wrap`/`direction`/`insets` survive `to_json`/`from_json`, affect rendered pixels in `render_slide_rgba` (`Authoring` and `Player` modes), and pass `coursecraft-cli validate`.
- **Why it might break:** Editor canvas and offscreen/player text layout could diverge on `letter_spacing` or `line_spacing` if only one path reads `run.letter_spacing`.
- **Where / how:** `crates/render/src/tests.rs` + `apps/coursecraft-cli/tests/m26_cli_and_packaging.rs`.
- **Setup & steps:**
  1. Render a slide with text at `(size=20, letter_spacing=0, line_spacing=1.0)` → image A.
  2. Update to `(size=40, letter_spacing=6.0, line_spacing=2.0)`, save/reopen JSON, render again → image B (both `RenderMode::Authoring` and `RenderMode::Player`).
  3. Assert pixel buffers A and B differ significantly, and `Authoring` vs `Player` images match within tiny tolerance.
- **Expected result:** Numeric text changes visibly alter rendered output identically across editor, preview, and player.
- **Priority:** P2

---

## A2. Gradients

**Bug class:** Multi-stop (>2) linear and radial gradients truncated to 2 stops, stops misordered or losing colours when edited/added/removed/reordered, or failing on non-shape objects, states, format painter, thumbnails, or published web player.

### A2.1 — 4-stop, 8-stop, and maximum 10-stop linear & radial gradients
- **What it checks:** Adding stops up to `MAX_GRADIENT_STOPS` (10) in both Linear and Radial gradients: every stop colour appears in rendered pixels, and `"Add colour"` button hides at 10 stops.
- **Why it might break:** `shape_fill.rs` has a vertex cap (`GRADIENT_MAX_EDGE` / mesh subdivision cap) that could coarsen triangles on many stops, or `resolve_gradient_stops` / UI loop could truncate or clip rows at 8–10 stops.
- **Where / how:** `crates/ui-egui/src/fill_style_tests.rs` using `TestHarness` + `coursecraft_render::render_slide_rgba`.
- **Setup & steps:**
  1. Select a large shape (`w=800, h=400`) on `s-intro`, choose `"Linear gradient"`.
  2. Click `"Add colour"` 2 times (total 4 stops: `#FF0000`, `#00FF00`, `#0000FF`, `#FFFF00` at 0/33/66/100%). Render and verify all 4 distinct RGB regions appear along the horizontal centreline.
  3. Click `"Add colour"` 4 more times (8 stops), then 2 more times (10 stops = `MAX_GRADIENT_STOPS`). Assert `"Add colour"` button is no longer shown at 10 stops and `stops.len() == 10`.
  4. Repeat 4-stop pixel check for `"Radial gradient"` (`center: [0.5, 0.5]`) sampling along a radius from centre to corner.
  5. Run `layout_report(2)` with 4 stops and 10 stops (scrolled) — zero `clipped-widget` or `overlap` findings.
- **Expected result:** 4, 8, and 10 stops all persist, fit cleanly in the right panel, and render all stops smoothly.
- **Priority:** P1

### A2.2 — Removing a middle stop and enforcing the 2-stop minimum
- **What it checks:** Deleting stop 2 of a 4-stop gradient leaves stops 1, 3, 4 intact; deleting down to 2 stops removes the `"×"` button so a gradient can never become a 1-stop invalid gradient.
- **Why it might break:** Off-by-one index removal or failing to hide `"×"` at `len == 2` could leave invalid 1-stop gradients that fail `course.validate()` or fall back to solid fill.
- **Where / how:** `crates/ui-egui/src/fill_style_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Create a 4-stop linear gradient (`#FF0000` @ 0%, `#00FF00` @ 33%, `#0000FF` @ 66%, `#FFFF00` @ 100%).
  2. Click `"×"` on `"Colour 2"` (`#00FF00`). Assert remaining colours are `["#FF0000", "#0000FF", "#FFFF00"]` and green no longer appears in `render_slide_rgba`.
  3. Click `"×"` on `"Colour 2"` again (`#0000FF`). Assert remaining colours are `["#FF0000", "#FFFF00"]` and `find_text_in("×", right_panel)` is `None`.
  4. Undo twice — assert 4 stops return in original order.
- **Expected result:** Middle removal, 2-stop floor, render update, and undo all work cleanly.
- **Priority:** P1

### A2.3 — Adding a stop + immediately editing hex colour after `stops.sort_by` re-indexing
- **What it checks:** Clicking `"Add colour"` (which computes `next_gradient_stop_position` and then calls `stops.sort_by` in `properties.rs:4193`), then immediately typing a new hex colour into the newly inserted row (`"Colour 2"`) without selecting away first.
- **Why it might break:** `show_gradient_stops` pushes `ui.push_id(("fill_stop", i), ...)`. Because `stops.sort_by` inserts the new stop in the middle (`i = 1`), the old stop at `i = 1` shifts to `i = 2`; egui's `TextEdit` state cached under `("fill_stop", i)` can hold stale text from the previous stop that sat at index `i`, causing the wrong colour to display or overwrite!
- **Where / how:** `crates/ui-egui/src/fill_style_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Start with 2-stop linear gradient: Colour 1 `#FF0000` (0%), Colour 2 `#0000FF` (100%).
  2. Click `"Add colour"` (inserts stop at 50% with `#0000FF`, sorted to index 1).
  3. Verify the rendered hex fields on rows 1, 2, 3 display `#FF0000`, `#0000FF`, `#0000FF` respectively.
  4. Edit Colour 2's hex field to `#00FF00`. Assert stored stops are `[#FF0000 @ 0.0, #00FF00 @ 0.5, #0000FF @ 1.0]`.
  5. Edit Colour 2's position to `"95"` then `"5"`, verifying Colour 2's hex field updates cleanly when sorting moves stops across indices.
- **Expected result:** Displayed hex fields always match each row's actual stored stop colour after additions and position-driven re-sorts.
- **Priority:** P1

### A2.4 — Two stops at the exact same position (hard step edge) & reordering them
- **What it checks:** Two stops both at `position = 0.5` (e.g. `#FF0000 @ 0.0`, `#FF0000 @ 0.5`, `#0000FF @ 0.5`, `#0000FF @ 1.0`), plus clicking `↑`/`↓` to swap them.
- **Why it might break:** `sample_gradient` (`crates/render/src/paint.rs:193`) checks `if span.abs() <= 1e-6`; division by zero or unstable `sort_by` when positions tie could flip stops unexpectedly or produce NaN pixels.
- **Where / how:** `crates/ui-egui/src/fill_style_tests.rs` using `TestHarness` + `render_slide_rgba`.
- **Setup & steps:**
  1. Configure 4 stops on a wide rectangle (`x=100..900, angle=0°`): Red @ 0%, Red @ 50%, Blue @ 50%, Blue @ 100%.
  2. Render via `render_slide_rgba`: assert pixel at `x=490` (48.75%) is pure red (`r>220, b<30`) and pixel at `x=510` (51.25%) is pure blue (`b>220, r<30`).
  3. Click `↓` on Colour 2 (or `↑` on Colour 3) in the Format panel. Assert stops swap cleanly without crashing and undo restores the sharp edge.
- **Expected result:** Crisp hard-edge colour transition at 50% with zero NaN or crash; reordering tied stops works and undoes cleanly.
- **Priority:** P2

### A2.5 — Semi-transparent (alpha) gradient stops over a contrasting background
- **What it checks:** A 3-stop gradient with alpha channels (`#FF0000FF` @ 0%, `#FF000080` @ 50%, `#FF000000` @ 100%) drawn over a dark blue (`#000044`) slide background.
- **Why it might break:** Premultiplied vs straight alpha blending bugs (seen in `target/fidelity/report.md` Slide 6: "alpha-gradient fades towards white/grey instead of transparent") cause bright grey haloes at mid-alpha stops.
- **Where / how:** `crates/render/src/tests.rs` using `render_slide_rgba`.
- **Setup & steps:**
  1. Create a slide with `background: Some(Color("#000044"))` and a rectangle filled with linear 3-stop gradient `#FF0000FF` (0.0) → `#FF000080` (0.5) → `#FF000000` (1.0), `angle: 0.0`.
  2. Render via `render_slide_rgba`.
  3. Sample left edge (0%), midpoint (50%), and right edge (99%):
     - Left: red (`r > 230, g < 15, b < 15`).
     - Midpoint: blended red+dark-blue (`r ≈ 128 ± 25, g < 20, b ≈ 34 ± 20` — NOT bright grey `g > 60`!).
     - Right: matches background `#000044` (`r < 15, g < 15, b in 55..80`).
- **Expected result:** Smooth alpha fade into the underlying background with no grey/white halo.
- **Priority:** P2

### A2.6 — Gradients across all fillable object kinds & fill-type round-trip
- **What it checks:** 3-stop linear & radial gradients on: (a) non-rectangular shapes (`Ellipse`, `Heart`, `Star5`, `RoundedRectangle`), (b) `Button`, (c) closed `ObjectKind::Path` (custom geometry), and (d) switching a shape through `Solid` → `Linear (3 stops)` → `Picture` → `Radial (3 stops)` → `Solid` → Undo × 4.
- **Why it might break:** Concave shapes (`Heart`, `Star5`) go through ear-clipping + shaded mesh triangulation (`shape_fill.rs`); closed `Path` uses `paint_gradient_contours` separately (`paint.rs:1270`); switching from `Picture` to `Radial` could carry stale `pending_picture_fill`.
- **Where / how:** `crates/render/src/tests.rs` + `crates/ui-egui/src/fill_style_tests.rs`.
- **Setup & steps:**
  1. Render `Ellipse`, `Heart`, `Star5`, `Button`, and a closed triangle `ObjectKind::Path` each with a 3-stop `#0000FF`/`#FF0000`/`#FFFF00` linear gradient; verify all 3 colours appear inside each object's interior pixels.
  2. In `TestHarness`, switch `btn-start` from `Solid` → `Linear gradient` (add 3rd stop) → `Picture` (with an imported PNG) → `Radial gradient` → `Solid`, then `undo()` back through every state.
- **Expected result:** Every fillable kind renders all 3 stops; fill-type transitions and undos are lossless.
- **Priority:** P1

### A2.7 — Multi-stop gradient combined with interactive States (Hover / Selected / Disabled)
- **What it checks:** A button with a 3-stop linear gradient and two states: `Hover` with `fill: Some("#11AA11")` (solid override) and `Selected` with `fill: None, text: Some("Chosen")` (text-only override).
- **Why it might break:** `crates/render/src/compose.rs:1239–1243, 1313–1317` sets `fill_style: None` whenever `state_fill.is_some()`. If state transitions in `Player` don't clear hover cleanly, or if `Selected` (with `fill: None`) accidentally strips `fill_style`, the gradient disappears permanently!
- **Where / how:** `crates/render/src/tests.rs` driving real `Player` + `render_player_rgba`.
- **Setup & steps:**
  1. Create a slide with button `"b1"` having 3-stop gradient (`#0000FF`/`#FF0000`/`#FFFF00`), state `Hover { fill: Some("#11AA11") }`, state `Selected { fill: None, text: Some("Chosen") }`, and a click trigger setting state to `Selected`.
  2. Start `Player`: render Normal → assert blue, red, yellow pixels present.
  3. Send `Input::HoverStart { object: "b1" }` → render → assert solid green `#11AA11`, no yellow/blue.
  4. Send `Input::HoverEnd { object: "b1" }` → render → assert blue, red, yellow gradient restored.
  5. Send `Input::Click { object: "b1" }` (activates `Selected`) → render → assert 3-stop gradient is STILL rendered.
- **Expected result:** Solid state override masks gradient only while active; text-only state keeps the 3-stop gradient; returning to Normal restores the 3-stop gradient.
- **Priority:** P2

### A2.8 — Duplicate, Copy/Paste, and Format Painter with 3-stop gradients
- **What it checks:** Duplicating (`Ctrl+D`), copying/pasting (`Ctrl+C`/`Ctrl+V`), and format-painting (`Paint` / `object.apply_style`) an object with a 4-stop linear or radial gradient preserves all stops, positions, and angle/centre.
- **Why it might break:** Shallow cloning or a custom style-copy routine (`look_of`/`apply_look` in `commands.rs:5624–5720`) could drop `fill_style` on some target kinds (e.g. `Path`), or copy `fill` without `fill_style`.
- **Where / how:** `crates/ui-egui/src/fill_style_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Give shape `"s1"` a 4-stop linear gradient (`angle: 135.0`).
  2. `Ctrl+D` (duplicate) → inspect new shape's `fill_style` (all 4 stops + angle match).
  3. `Ctrl+C`, switch to Slide 2, `Ctrl+V` → inspect pasted shape's `fill_style` (all 4 stops + angle match).
  4. Arm `Paint` on `"s1"`, click button `"b1"` → assert `"b1"` receives the 4-stop `FillStyle::Linear`.
  5. Arm `Paint` on `"s1"`, click a closed `ObjectKind::Path` `"p1"` → check whether `p1.path.fill_style` receives the gradient (flags gap if `apply_look` skips `ObjectKind::Path`!).
- **Expected result:** Duplicate, cross-slide copy/paste, and Paint onto shapes/buttons preserve all 4 stops verbatim; closed `Path` behaviour is explicitly verified.
- **Priority:** P1

### A2.9 — Multi-select with gradient objects
- **What it checks:** Selecting two shapes where one has a 3-stop gradient and the other has a solid fill (or both have 3-stop gradients), editing properties in multi-select, and verifying neither object's gradient stops are wiped unexpectedly.
- **Why it might break:** Multi-select `Fill` editor (`show_multi_properties`) edits solid `fill` — changing solid fill or opacity/position in multi-select must not accidentally clear `fill_style`, or if multi-select offers `fill_style`, it must apply to all selected objects.
- **Where / how:** `crates/ui-egui/src/fill_style_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Slide with `"s1"` (3-stop gradient) and `"s2"` (solid `#112233`).
  2. `selection.set_many(["s1", "s2"])`, open `InspectorTab::Format`, change Opacity to `80%` and run `layout_report(2)`.
  3. Inspect `"s1"` and `"s2"` in `session.course`.
- **Expected result:** `"s1"` retains all 3 gradient stops intact while both objects get opacity `0.8`; zero layout findings.
- **Priority:** P2

### A2.10 — Extreme angles, off-centre radial origins, and tiny vs full-slide shapes
- **What it checks:** Linear angles `0°`, `45°`, `90°`, `180°`, `270°`, `360°`; Radial centre `[0.0, 0.0]` and `[1.0, 1.0]`; shape dimensions `8×8` (tiny) and `1280×720` (full slide).
- **Why it might break:** `extent` calculation at `90°`/`270°` or `max_distance` when `center` is at a corner `[0.0, 0.0]` could invert or clip stops; tiny `8×8` shapes or huge `1280×720` shapes stress `GRADIENT_MAX_EDGE` subdivision limits.
- **Where / how:** `crates/render/src/tests.rs` using `render_slide_rgba`.
- **Setup & steps:**
  1. Render a 3-stop linear gradient (`#0000FF`, `#FF0000`, `#FFFF00`) at `0°` (blue left, yellow right), `90°` (blue top, yellow bottom), `180°` (yellow left, blue right), `270°` (yellow top, blue bottom) — assert directional pixel checks for all four.
  2. Render radial with `center: [0.0, 0.0]` — assert top-left corner is blue and bottom-right corner is yellow.
  3. Render on `8×8` and `1280×720` shapes — assert no panic, finite pixels, all 3 colours present on `1280×720`.
- **Expected result:** All angles, corner radial centres, and extreme sizes render accurately without artifacts or crashes.
- **Priority:** P3

### A2.11 — Editor canvas, thumbnails, Preview, and Published Web Player parity
- **What it checks:** A 3-stop linear gradient and 3-stop radial gradient render identically in: (1) editor slide thumbnail, (2) editor canvas, (3) Preview modal (`render_player_rgba`), and (4) a published web package (`coursecraft-cli publish --standard web -o target/m26-grad-web.zip`).
- **Why it might break:** Thumbnails use small scale factors (`SlideTransform::fit` on ~140px width) where gradient subdivision could collapse to 2 stops; published package serialization could strip or alter `fill_style`.
- **Where / how:** `apps/coursecraft-cli/tests/m26_cli_and_packaging.rs` + `crates/render/src/tests.rs`.
- **Setup & steps:**
  1. Build a course with 3-stop linear & radial gradient shapes, save to `target/m26-grad.ccraft`.
  2. Render at thumbnail width (`160px`) and full width (`1280px`) via `render_slide_rgba` (`Authoring` and `Player`) — assert blue, red, and yellow all appear even at `160px` width.
  3. Run `coursecraft-cli publish target/m26-grad.ccraft -o target/m26-grad-web.zip --standard web` (if `target/player-dist` exists, else test `coursecraft_publish::publish_course` with a stub player dir); unzip `course.json` from the zip, parse with `Course::from_json`, and assert all 3 stops are identical.
- **Expected result:** Thumbnails, full renders, player renders, and published `course.json` all preserve and render all 3 stops.
- **Priority:** P1

---

## A3. Exit animations & slide transitions

**Bug class:** Adding a slide transition (`Fade`, `Push`, `Cover`) interferes with object exit animations (`FlyIn`/Fly out, `Fade`/Disappear, `Zoom`, `Wipe`, `Float`), especially when actually transitioning from a previous slide, when clocks/transitions overlap, on layers, or after seek/replay.

### A3.1 — Real multi-slide transition (Slide 1 → Slide 2) × all transition types × all exit effects
- **What it checks:** Navigating from Slide 1 to Slide 2 (where Slide 2 has `Fade`, `Push` 4 dirs, or `Cover` 4 dirs) so `active_transition` genuinely runs on Slide 2, then advancing into Slide 2's exit window for every exit effect (`Fade`, `FlyIn` 4 dirs, `Zoom`, `Wipe` 4 dirs, `Float` 4 dirs).
- **Why it might break:** The existing A3 test (`crates/logic/src/tests.rs:10868`) sets `transition` on the *first* slide (`s-intro`) and calls `Input::Start` — on Slide 1 there is no previous slide so `active_transition` may never even be populated! Navigating Slide 1 → Slide 2 exercises the real transition state machine (`active_transition` active during `0..trans.duration_ms` and completing before/during exit).
- **Where / how:** `crates/logic/src/tests.rs` driving `Player`.
- **Setup & steps:**
  1. Two-slide course: Slide 1 (`s1`, 2000ms) → Slide 2 (`s2`, 5000ms, `transition: Some(trans)`).
  2. Place two objects on `s2`: `"obj-exit"` with each `AnimEffect` exit (`duration_ms: 1000`, `end_ms: None`) and `"obj-fade"` with `Fade` exit (`duration_ms: 1000`, `end_ms: None`).
  3. `Player::new` → `Start` → `GoTo { slide: "s2" }`. Assert `player.active_transition().is_some()`.
  4. Tick through transition (`500ms`), verify `active_transition().is_none()`, tick to `4500ms` (mid-exit), assert BOTH `"obj-exit"` and `"obj-fade"` have non-identity exit samples (exact offset/opacity/scale/clip checks).
  5. Tick to `5000ms` — assert both return `None` from `sample_object`.
- **Expected result:** Every exit effect animates accurately on Slide 2 after a real slide transition from Slide 1.
- **Priority:** P1

### A3.2 — Transition duration overlapping the exit animation window
- **What it checks:** A short slide (`duration_ms: 1500`) with a `1000ms` `Push` transition and an object whose `FlyIn` exit runs `700..1500ms` (so transition and exit overlap during `700..1000ms`).
- **Why it might break:** If transition playback pauses/freezes object animations or overrides object transforms while `active_transition.is_some()`, the exit won't start at `700ms` or will jump discontinuously at `1000ms`.
- **Where / how:** `crates/logic/src/tests.rs` + `crates/render/src/tests.rs` (`compose(&player)`).
- **Setup & steps:**
  1. Slide 1 → `GoTo` Slide 2 (`duration_ms: 1500`, `transition: Push(Right, 1000ms)`, object `exit: FlyIn(Left, 800ms)`).
  2. Tick `850ms` (transition at 85%, exit at 18.75%): assert `sample_object` already has non-zero negative X offset AND `compose(&player)` applies both transition offset and object exit offset.
  3. Tick `250ms` (t=1100ms, transition done, exit at 50%): assert exact 50% Fly out offset.
  4. Tick `400ms` (t=1500ms): assert `sample_object` is `None`.
- **Expected result:** Overlapping transition and exit both progress smoothly with zero stall or jump.
- **Priority:** P2

### A3.3 — Explicit `timing.end_ms < slide.duration_ms` vs slide-end exit (`end_ms: None`) with transition
- **What it checks:** Two objects on a 5000ms slide with a 500ms Fade transition: `"early"` (`end_ms: Some(3000)`, Fly out 1000ms → exits 2000..3000ms) and `"late"` (`end_ms: None`, Fly out 1000ms → exits 4000..5000ms).
- **Why it might break:** Exit start computation (`end_ms.unwrap_or(duration_ms) - exit.duration_ms`) could behave differently for explicit `end_ms` vs `None`, or confuse slide-end clamping.
- **Where / how:** `crates/logic/src/tests.rs` driving `Player`.
- **Setup & steps:**
  1. Navigate Slide 1 → Slide 2.
  2. Tick to `2500ms`: `"early"` is at 50% Fly out, `"late"` is `IDENTITY`.
  3. Tick to `3000ms`: `"early"` is `None`, `"late"` is `IDENTITY`.
  4. Tick to `4500ms`: `"early"` is `None`, `"late"` is at 50% Fly out.
  5. Tick to `5000ms`: both are `None`.
- **Expected result:** Both early-ending and slide-ending exits animate in their exact windows and disappear at their respective ends.
- **Priority:** P1

### A3.4 — Exit animations on a Slide Layer while base slide has a transition
- **What it checks:** Base slide has a `Cover` transition; a trigger opens Layer `"L1"` (`duration_ms: 3000`, tested with both `pause_base_timeline: true` and `false`) containing a shape with a `FlyIn` exit (`1000ms`).
- **Why it might break:** `sample_object` uses `layer_clocks` and `layer.duration_ms`; pausing the base timeline or having a base transition could interfere with layer clock advancement or layer exit sampling.
- **Where / how:** `crates/logic/src/tests.rs` driving `Player`.
- **Setup & steps:**
  1. Slide 1 → `GoTo` Slide 2 (transition 500ms) → tick `1000ms` → click button to show layer `"L1"`.
  2. Tick `1500ms` (layer clock 1500ms, before exit 2000..3000ms): layer object is `IDENTITY`.
  3. Tick `1000ms` (layer clock 2500ms, mid-exit): layer object has exact 50% Fly out offset.
  4. Tick `500ms` (layer clock 3000ms): layer object is `None`.
  5. Repeat with `pause_base_timeline: true` and verify base objects freeze while layer exit plays.
- **Expected result:** Layer exit animations run accurately on layer time regardless of base slide transition or `pause_base_timeline`.
- **Priority:** P1

### A3.5 — Rendered visual check (`compose` + `render_slide_rgba` in `RenderMode::Player`) during mid-exit with transition
- **What it checks:** Not just `Player::sample_object`, but the actual rendered pixels and `DrawItem` rects produced by `coursecraft_render::compose` and `render_slide_rgba`!
- **Why it might break:** A bug in `compose.rs` or `paint.rs` (e.g., ignoring `sample.offset` or `sample.clip` when `slide.transition.is_some()`, or clipping an exiting object incorrectly) passes `logic` unit tests while staying broken visually for users!
- **Where / how:** `crates/render/src/tests.rs` using `Player`, `compose(&player)`, and `render_player_rgba`.
- **Setup & steps:**
  1. Slide 1 → `GoTo` Slide 2 (5000ms, `transition: Push(Left, 500ms)`), bright red rectangle `"obj-fly"` at `x=400..600, y=200..300` with `FlyIn(Right, 1000ms)` exit, bright green rectangle `"obj-wipe"` with `Wipe(Left, 1000ms)` exit.
  2. Render at `t=2000ms` (rest): red pixels present at `x=500, y=250`; green pixels present across `obj-wipe`'s full width.
  3. Tick to `t=4500ms` (mid-exit): assert `obj-fly`'s `DrawItem.rect.min.x > 700.0`, pixel at original `x=500, y=250` is no longer red, pixel at shifted rect centre IS red, and `obj-wipe`'s right half is clipped away.
  4. Tick to `t=5000ms`: neither red nor green appears anywhere on the slide.
- **Expected result:** Rendered pixels confirm Fly out physically moves off-slide and Wipe clips away even with a slide transition configured.
- **Priority:** P1

### A3.6 — Seek, Pause, Resume, and Playback Speed (`0.5x` / `2.0x`) inside the exit window
- **What it checks:** Seeking directly into the exit window (`Input::PlayerSeek { ms: 4500 }`), seeking back before exit (`2000ms`), pausing mid-exit (`PlayerPause`), resuming (`PlayerResume`), and playing at `2.0x` speed on a slide with a transition.
- **Why it might break:** Seeking past the transition or backward from `5000ms` (already exited) to `4500ms` might fail to re-materialize an exited object or mis-scale exit progress under `speed != 1.0`.
- **Where / how:** `crates/logic/src/tests.rs` driving `Player`.
- **Setup & steps:**
  1. Navigate Slide 1 → Slide 2 (5000ms, Fade transition, Fly out 4000..5000ms).
  2. Tick to `5000ms` (`sample_object` is `None`).
  3. `PlayerSeek { ms: 4500 }` → assert `sample_object` returns 50% Fly out offset!
  4. `PlayerPause` → `Tick { ms: 1000 }` → assert offset unchanged (still 50%).
  5. `PlayerResume` + `SetSpeed { speed: 2.0 }` → `Tick { ms: 125 }` (250ms effective → 4750ms) → assert 75% Fly out offset.
  6. `PlayerSeek { ms: 2000 }` → assert `ObjectSample::IDENTITY`.
- **Expected result:** Seeking backward/forward, pausing, resuming, and speed scaling all compute exact exit states.
- **Priority:** P2

### A3.7 — Replay and Revisit (`revisit: reset` vs `resume`) after exit finishes
- **What it checks:** Pressing Replay (`PlayerSeek { ms: 0 }`) at slide end, or navigating away to Slide 3 and returning to Slide 2 with `revisit: Revisit::Reset` vs `Revisit::Resume`.
- **Why it might break:** On revisit, Slide 2's transition plays again; stale exited state from the first visit could prevent the object from re-entering or re-exiting cleanly.
- **Where / how:** `crates/logic/src/tests.rs` driving `Player`.
- **Setup & steps:**
  1. Navigate `s1` → `s2` (transition + Fly out) → tick to `5000ms` (exited).
  2. Navigate `s2` → `s3` → `s2` with `revisit: Reset`: assert `s2` starts at 0ms, object is visible at `2000ms`, animates Fly out at `4500ms`, and exits at `5000ms`.
  3. Repeat with `revisit: Resume`: returning to `s2` keeps it at `5000ms` (exited), then Replay (`PlayerSeek { ms: 0 }`) replays from 0 and plays Fly out at `4500ms`.
- **Expected result:** Both reset-revisit and resume+replay play the exit animation cleanly.
- **Priority:** P2

### A3.8 — Exit animation combined with Motion Path on a transitioned slide
- **What it checks:** An object moves `(+200, 0)` along a motion path at `t=1000..2000ms`, then plays a `FlyIn(Left)` exit at `4000..5000ms` on a slide with a `Fade` transition.
- **Why it might break:** `sample_object` sums `anim_sample.offset + motion_offset + drag_pos` (`crates/logic/src/lib.rs:1700–1703`); if the motion path resets or overrides exit offset, the object snaps back instead of flying out from its resting position.
- **Where / how:** `crates/logic/src/tests.rs` driving `Player`.
- **Setup & steps:**
  1. Configure `s2` with transition, motion path `(0,0) -> (200,0)` triggered on `TimelineStarts`, and `FlyIn(Left, 1000ms)` exit.
  2. Navigate `s1` → `s2`, tick `3000ms`: assert `offset == (200.0, 0.0)`.
  3. Tick `1500ms` (`t=4500ms`, mid-exit): assert `offset.0 ≈ 200.0 - 0.5 * (frame.x + frame.w)`.
- **Expected result:** Exit animation composes additively with the motion path's final resting offset.
- **Priority:** P3

### A3.9 — `AnimationCompletes { which: Exit }` trigger & click hit-testing during/after exit
- **What it checks:** A trigger firing on `Event::AnimationCompletes { object: "obj-fly", which: Exit }` sets variable `exited = true`, and clicking `"obj-fly"` after exit produces a warning (not a click).
- **Why it might break:** If a slide transition delays or drops exit completion detection in `run.rs`, `AnimationCompletes` never fires (or fires twice).
- **Where / how:** `crates/logic/src/tests.rs` driving `Player`.
- **Setup & steps:**
  1. `s1` → `GoTo("s2")` with transition, `"obj-fly"` (`end_ms: Some(4000)`, Fly out `1000ms`), trigger `AnimationCompletes(Exit)` → increment counter `n` by 1.
  2. Tick `3500ms`: `n == 0`, clicking `"obj-fly"` succeeds.
  3. Tick `500ms` (`t=4000ms`): `n == 1`.
  4. Tick `500ms` (`t=4500ms`): `n` is still `1` (fired exactly once), clicking `"obj-fly"` emits warning.
- **Expected result:** `AnimationCompletes` fires exactly once at `4000ms`; post-exit clicks are rejected.
- **Priority:** P2

### A3.10 — CLI `simulate` and `render --mode player --at-ms` end-to-end check
- **What it checks:** Driving the 2-slide transition + Fly out / Disappear course through `coursecraft-cli simulate` and `coursecraft-cli render --mode player --at-ms 4500`.
- **Why it might break:** Verifies CLI / headless publish pipeline uses the exact same `Player` and renderer behaviour without silent divergence.
- **Where / how:** `apps/coursecraft-cli/tests/m26_cli_and_packaging.rs`.
- **Setup & steps:**
  1. Write the 2-slide course to `target/m26-a3.json` and an input script `[start, goto s2, tick 4500, tick 500]` to `target/m26-a3.inputs.json`.
  2. Run `coursecraft-cli simulate target/m26-a3.json target/m26-a3.inputs.json --strict` — assert exit 0 and `AnimationCompletes` effect in JSONL output.
  3. Run `coursecraft-cli render target/m26-a3.json --slide s2 --mode player --at-ms 2000 -o target/m26-a3-rest.png` and `--at-ms 4500 -o target/m26-a3-exit.png` — assert the two PNGs differ and exit 0.
- **Expected result:** CLI simulate and player-mode renders match `Player` expectations cleanly.
- **Priority:** P1

---

## A4. Timeline scrubbing & canvas preview

**Bug class:** Scrubbing the playhead updates the time readout but not the canvas, or breaks object visibility, non-fade animations, layers, selection, inline editing, or Play/Pause transitions.

### A4.1 — All entrance & exit animation types while scrubbing (`FlyIn`, `Zoom`, `Wipe`, `Float`)
- **What it checks:** Scrubbing to entrance midpoint and exit midpoint moves `FlyIn` shapes, scales `Zoom` shapes, clips `Wipe` shapes, and shifts+fades `Float` shapes on the editor canvas.
- **Why it might break:** Existing `timeline_scrub_tests.rs` only tested `Fade` opacity! `timeline_animated_items` (`crates/ui-egui/src/canvas.rs:6126–6171`) applies `offset`, `scale`, and `clip` separately; a bug in `scale` centering or `clip` painting in authoring mode would go unnoticed.
- **Where / how:** `crates/ui-egui/src/timeline_scrub_tests.rs` using `TestHarness` + `frame_shapes`.
- **Setup & steps:**
  1. Course with 4 objects on `s1` (`duration_ms: 5000`), each having `0..1000ms` entrance and `4000..5000ms` exit for `FlyIn(Left)`, `Zoom`, `Wipe(Left)`, and `Float(Bottom)`.
  2. Capture text/rect screen positions at `playhead = 2500ms` (rest).
  3. Scrub to `500ms` (entrance mid): assert `FlyIn` painted centre is shifted left, `Zoom` shape bounding rect width is ~50% of resting width, `Wipe` clip rect width is ~50%, `Float` is shifted down + half alpha.
  4. Scrub to `4500ms` (exit mid): assert all four animate toward their exited states.
- **Expected result:** All 4 non-Fade animation types visibly transform on the canvas when scrubbed.
- **Priority:** P1

### A4.2 — Scrubbing while editing a Slide Layer with a different `layer.duration_ms`
- **What it checks:** Base slide has `duration_ms: 5000`, Layer `"L1"` has `duration_ms: 2000` with object `"l-delayed"` (`start_ms: 1000`) and `"l-exit"` (`exit: Fade 500ms`, `end_ms: None` → exits `1500..2000ms`).
- **Why it might break:** `timeline_animated_items` looks up `layer.duration_ms`, but if the timeline ruler still scrubs against `slide.duration_ms` (or doesn't pass the layer's duration to the ruler), scrubbing to 50% hits `2500ms` instead of `1000ms` and hides all layer objects!
- **Where / how:** `crates/ui-egui/src/timeline_scrub_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Switch active layer to `"L1"` via its layer pill on the canvas.
  2. Scrub to `500ms`: `"l-delayed"` is hidden, `"l-exit"` is full alpha.
  3. Scrub to `1250ms`: `"l-delayed"` is visible, `"l-exit"` is full alpha.
  4. Scrub to `1750ms`: `"l-exit"` is ~half alpha (`100..=160`).
- **Expected result:** Layer scrubbing follows `layer.duration_ms` and layer object timings accurately.
- **Priority:** P1

### A4.3 — Exact boundary timestamps (`0ms`, `start_ms - 1`, `start_ms`, `end_ms`, `duration_ms`)
- **What it checks:**
  - At `0ms` paused: static design view shows delayed objects (`start_ms: 2000`).
  - At `0ms` playing: time-based view hides delayed objects.
  - At `1999ms`: delayed object hidden; at `2000ms`: delayed object visible.
  - At `end_ms` (`4000ms`): object with `end_ms: 4000` is hidden.
  - At `duration_ms` (`5000ms`): objects with `end_ms: None` and no exit stay visible; objects with exit animation are hidden.
- **Why it might break:** Off-by-one `<` vs `<=` at `start_ms`/`end_ms` or `playhead_ms == 0` guard (`time_view = is_playing() || playhead_ms > 0`).
- **Where / how:** `crates/ui-egui/src/timeline_scrub_tests.rs` using `TestHarness` (direct ruler clicks + `playhead_ms` checks).
- **Setup & steps:** Test each exact boundary timestamp on `scrub_course()`, verifying `find_text_in(..., canvas)`.
- **Expected result:** Every boundary matches `sample_timed_animation` semantics; paused at `0ms` always returns to full static design view.
- **Priority:** P1

### A4.4 — Continuous drag scrubbing forward, backward, and past ruler edges
- **What it checks:** Dragging the playhead continuously (`drag_by`) from `0s` → `4.5s` → `1.0s` → past left edge (`-40px`) → past right edge (`+100px`).
- **Why it might break:** Dragging past the left or right ruler edge could produce negative/overflow `playhead_ms` or leave `playhead_ms = 12ms` instead of `0ms`, trapping the user out of static design view.
- **Where / how:** `crates/ui-egui/src/timeline_scrub_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. `drag_by` on ruler from `0s` to `4.5s` (10 steps) — verify `Exiting Text` disappears.
  2. `drag_by` back past `0s` by `-40px` — verify `playhead_ms == 0` and `Delayed Hello` reappears.
  3. `drag_by` past `5s` by `+80px` — verify `playhead_ms == 5000` (clamped, never > `duration_ms`).
- **Expected result:** Smooth drag scrubbing clamps cleanly to `[0, duration_ms]` and restores static view at 0.
- **Priority:** P2

### A4.5 — Selected object becomes hidden after scrubbing outside its timing
- **What it checks:** Select `"delayed"` (`start_ms: 2000`) at `3s`, then scrub to `1s` where `"delayed"` is hidden; attempt to drag at its old canvas position.
- **Why it might break:** `draw_handles_and_outline` receives `view_items`, so handles disappear, but if `start_drag_if_pressed` or marquee selection still tests `items` instead of `input_animated`, dragging empty space could move the invisible object!
- **Where / how:** `crates/ui-egui/src/timeline_scrub_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Scrub to `3000ms`, click `"delayed"` to select it, record `orig_frame`.
  2. Scrub to `1000ms` (`"delayed"` no longer drawn).
  3. Drag across `orig_frame`'s screen centre by `vec2(80.0, 0.0)`.
  4. Scrub back to `3000ms` and check `"delayed"`'s `frame.x`.
- **Expected result:** `"delayed"` did NOT move while hidden at `1000ms`.
- **Priority:** P2

### A4.6 — Inline text editing open when scrubbing ruler or clicking timeline Play
- **What it checks:** Double-clicking `"Delayed Hello"` to open inline text edit, typing `" Edited"`, then scrubbing the timeline ruler to `1000ms` (where that object is hidden) or clicking timeline Play.
- **Why it might break:** `draw_inline_edit` (`canvas.rs:5823`) paints an overlay `TextEdit` at the object's un-animated rect; if scrubbing doesn't commit & close `inline_edit`, the text box floats on screen even when the object is hidden or moved!
- **Where / how:** `crates/ui-egui/src/timeline_scrub_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. At `playhead = 0`, double-click `"Delayed Hello"`, type `"!"`. Assert `editor.ui.inline_edit.is_some()`.
  2. Scrub ruler to `1000ms`.
  3. Assert `editor.ui.inline_edit.is_none()`, the edited text `"Delayed Hello!"` was committed to `session.course`, and it is not drawn on canvas at `1000ms`.
- **Expected result:** Scrubbing or playing commits and closes inline edit cleanly; no floating editor over hidden text.
- **Priority:** P2

### A4.7 — Timeline Play advancing through time (`frame_at_time`), pausing, resuming, and auto-stopping at end
- **What it checks:** Clicking timeline Play advances `playhead_ms` across simulated real time (`frame_at_time`), reveals `"Delayed Hello"` after `2.0s`, fades `"Exiting Text"` at `3.5s`, stops at `5.0s`, and restarts from `0` when Play is clicked again at the end.
- **Why it might break:** Existing test only checks Play at `t=0`; advancing real time exercises `TimelinePlayback::tick`, clock anchors, pause/resume, and end-of-slide stop.
- **Where / how:** `crates/ui-egui/src/timeline_scrub_tests.rs` using `TestHarness::frame_at_time`.
- **Setup & steps:**
  1. Click Play button at `time = 1.0`. Assert `is_playing()`, `"Delayed Hello"` hidden.
  2. Advance `frame_at_time(3.5)` (2.5s elapsed): assert `playhead_ms` ≈ 2500ms and `"Delayed Hello"` is now visible on canvas.
  3. Click Pause button at `time = 3.5`, then `frame_at_time(5.0)`: assert `!is_playing()` and `playhead_ms` stayed ~2500ms.
  4. Click Play again at `time = 5.0`, advance to `time = 8.0` (past 5000ms end): assert `!is_playing()` and `playhead_ms == 5000`.
  5. Click Play again at `time = 8.5`: assert `is_playing()` and `playhead_ms == 0` (restarted from start).
- **Expected result:** Playhead and canvas advance in sync, Pause holds position, end stops playback, and Play-from-end restarts at 0.
- **Priority:** P1

### A4.8 — Switching slides or shortening `slide.duration_ms` while scrubbed or playing
- **What it checks:** Scrubbing Slide 1 (5000ms) to `4200ms` (or playing), then (a) switching to Slide 2 (2000ms) or (b) reducing Slide 1's duration to `2000ms` via inspector/command.
- **Why it might break:** Leaving `playhead_ms = 4200` on a 2000ms slide places the playhead off the right edge of the ruler and hides every object on Slide 2!
- **Where / how:** `crates/ui-egui/src/timeline_scrub_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Two-slide course: Slide 1 (`5000ms`), Slide 2 (`2000ms`, object visible `0..2000ms`).
  2. Scrub Slide 1 to `4200ms`, start Play, then click Slide 2 in the slides panel.
  3. Assert `!timeline_playback.is_playing()`, `playhead_ms <= 2000` (ideally `0`), and Slide 2's object is visible.
  4. Return to Slide 1, scrub to `4200ms`, run `slide.set_duration` to `2000ms`, idle frame — assert `playhead_ms <= 2000`.
- **Expected result:** Slide switch stops playback and keeps playhead in range; shortening duration clamps playhead cleanly.
- **Priority:** P1

### A4.9 — Master slide objects and `editor_hidden` objects while scrubbed
- **What it checks:** Master slide footer text stays visible when scrubbed to `3000ms`; an object marked `editor_hidden: true` stays hidden even when scrubbed into its `[start_ms, end_ms]` window.
- **Why it might break:** `timeline_animated_items` keeps non-slide items (masters) only if `slide.object(id)` returns `None`, and `editor_hidden` filtering must happen before time-view animation.
- **Where / how:** `crates/ui-egui/src/timeline_scrub_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Attach a master with text `"Master Logo"` to `s1`; set `editor_hidden: true` on a slide text object `"Hidden Note"` (`0..5000ms`).
  2. Scrub to `2500ms`.
  3. Assert `"Master Logo"` IS visible on canvas and `"Hidden Note"` is NOT visible.
- **Expected result:** Master objects persist across scrub times; `editor_hidden` objects stay hidden.
- **Priority:** P2

### A4.10 — Spacebar Play/Pause shortcut vs focused text inputs
- **What it checks:** Pressing `Space` with canvas focus toggles timeline Play/Pause; pressing `Space` while typing inside an inline text edit or an inspector `TextEdit` inserts a space character and does NOT start timeline playback.
- **Why it might break:** Global Spacebar handling in `shortcuts.rs` / `timeline.rs` could fire even when a text field has focus, causing objects to disappear mid-typing!
- **Where / how:** `crates/ui-egui/src/timeline_scrub_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Idle on canvas: `press(Key::Space, Modifiers::NONE)` → assert `timeline_playback.is_playing()`; press `Space` again → assert `!is_playing()`.
  2. Double-click `"Delayed Hello"` to open inline edit, press `Space` + `type_text(" ")` → assert `!timeline_playback.is_playing()`.
- **Expected result:** Spacebar toggles timeline playback only when no text input owns keyboard focus.
- **Priority:** P3

---

## A5. Shape drawing drag preview

**Bug class:** Dragging to draw a shape renders a plain rectangle instead of the chosen shape outline, or breaks on specific shape categories, negative/degenerate drags, zoom/pan, layers, snapping, or cancellation.

### A5.1 — Broad `ShapeKind` sweep across every gallery category
- **What it checks:** Picking shapes from every category — `Diamond`, `Pentagon`, `Hexagon`, `Star8`, `ArrowLeft`, `ArrowLeftRight`, `Chevron`, `CalloutRect`, `CalloutOval`, `Cloud`, `Lightning`, `Moon`, `Cross`, `Donut`, `BlockArc` — shows the actual shape outline mesh (not a plain `Shape::Rect`) during drag.
- **Why it might break:** Multi-contour shapes (`Donut` has an inner hole!) or complex curves (`Cloud`, `Moon`, `CalloutOval`) exercise `paint_shape` + `paint_outline` differently from basic polygons; any fallback to `Shape::Rect` or empty mesh would hide the preview.
- **Where / how:** `crates/ui-egui/src/shape_preview_tests.rs` using `TestHarness` + `drag_preview_shapes`.
- **Setup & steps:**
  1. Loop over all 15 `ShapeKind`s above.
  2. Pick each from the shape gallery, drag `(360, 200)` by `(180, 140)`.
  3. Assert: (a) no plain `Shape::Rect` preview at `preview_rect`, (b) at least one `Shape::Mesh` in `p.accent` whose `calc_bounds()` matches `preview_rect` within `6.0pt`, and (c) `Donut` has sufficient vertices for both outer + inner rings (`vertices.len() >= 24`).
- **Expected result:** Every shape category draws its real outline mesh during drag.
- **Priority:** P1

### A5.2 — All four drag quadrants with and without `Shift` (aspect lock)
- **What it checks:** Dragging `(+180, +120)`, `(-180, +120)`, `(+180, -120)`, and `(-180, -120)` with `ShapeKind::Heart` and `ShapeKind::ArrowRight`, both freehand and with `Modifiers::SHIFT`.
- **Why it might break:** Negative X or Y combined with `Shift` aspect-ratio snapping can flip the anchor corner or produce negative width/height rects that `paint_shape` skips.
- **Where / how:** `crates/ui-egui/src/shape_preview_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. For each of the 4 quadrant deltas and both `Modifiers::NONE` and `Modifiers::SHIFT`:
     - Pick `ShapeKind::Heart`, drag from `(640, 360)`.
     - Assert preview mesh exists, `preview_rect.is_positive()`, and with `SHIFT` `|width - height| < 2.0`.
     - Release pointer and assert placed `Object.frame` has `w > 0, h > 0` and matches `drag.frame` within `1.0`.
- **Expected result:** All 4 drag directions (with and without Shift) preview and place positive, accurately positioned shapes.
- **Priority:** P1

### A5.3 — Degenerate drags (zero-width, zero-height) and single-click placement
- **What it checks:** Pure vertical drag `(0, 140)`, pure horizontal drag `(140, 0)`, tiny 2px drag `(2, 2)`, and simple click (0px drag) with `ShapeKind::Star5`.
- **Why it might break:** `canvas.rs:4834` checks `if drag.frame.width() < 0.5 && drag.frame.height() < 0.5 { return; }` (logical AND!). A pure vertical drag has `width == 0.0, height == 140.0`, which passes that guard and calls `paint_shape` / `paint_outline` with a zero-width rect — risking NaN normals or degenerate triangles!
- **Where / how:** `crates/ui-egui/src/shape_preview_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Pick `ShapeKind::Star5`, drag `(400, 200)` by `(0, 140)` → inspect painted shapes (assert every mesh vertex position is finite, zero NaN) → release → assert placed shape has sensible minimum `w >= 1.0, h >= 1.0` (or default size).
  2. Repeat with `(140, 0)`.
  3. Pick `ShapeKind::Star5`, single-click `(500, 300)` without dragging → assert default-sized `Star5` is placed on the slide (`w > 40, h > 40`).
- **Expected result:** Zero NaNs or panics on zero-width/zero-height drags; single-click places a default-sized `Star5`.
- **Priority:** P1

### A5.4 — Cancel mid-drag with Escape
- **What it checks:** Starting a `ShapeKind::Heart` drag on canvas, moving to `(580, 360)` so preview is visible, then pressing `Escape` before releasing the mouse button.
- **Why it might break:** If `Escape` cancels `placement_tool` without clearing `placement_drag` (or vice versa), releasing the mouse button still inserts a shape or leaves a ghost preview stuck on canvas.
- **Where / how:** `crates/ui-egui/src/shape_preview_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Record `objects_before`. Pick `ShapeKind::Heart`, pointer down at `(400, 220)`, move to `(580, 360)` (assert `placement_drag.is_some()`).
  2. Press `Key::Escape`, then release mouse button at `(580, 360)`.
  3. Assert `placement_drag.is_none()`, `placement_tool.is_none()`, and `slide.objects.len() == objects_before`.
- **Expected result:** Escape aborts the drag cleanly with no ghost preview and no object created.
- **Priority:** P1

### A5.5 — Drawing a shape while zoomed in (200%), zoomed out (50%), and panned
- **What it checks:** Changing `editor.ui.zoom` to `2.0` (with non-zero `editor.ui.pan`) and `0.5`, then dragging to draw `ShapeKind::Ellipse`.
- **Why it might break:** `draw_placement_preview` converts `drag.frame` via `t.to_screen_rect`; mismatch between `SlideTransform` used for input vs preview painting would offset the preview outline from the pointer and the final placed shape.
- **Where / how:** `crates/ui-egui/src/shape_preview_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Set `editor.ui.zoom = 2.0`, `editor.ui.pan = vec2(80.0, -40.0)`, `idle_frame()`.
  2. Pick `ShapeKind::Ellipse`, drag between two screen points inside `central_panel_rect()`.
  3. Verify preview mesh bounds match screen drag start/end within `6pt`; release and verify placed shape's screen rect matches the preview rect.
  4. Repeat at `zoom = 0.5`.
- **Expected result:** Preview outline aligns with pointer and final placed shape at all zoom/pan states.
- **Priority:** P2

### A5.6 — Drawing a shape on a Slide Layer and on a Master Slide
- **What it checks:** Selecting layer `"about"` (and separately a master slide), picking `ShapeKind::Cloud`, dragging to preview and releasing.
- **Why it might break:** Placement preview renders in canvas space, and release dispatches `object.add` — verify the preview is visible above dimmed/hidden base objects and release adds the shape to the active layer/master, not base.
- **Where / how:** `crates/ui-egui/src/shape_preview_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Select layer `"about"` on `s-intro`, pick `ShapeKind::Cloud`, drag `(300, 200)` by `(160, 120)`.
  2. Assert outline mesh painted during drag; release; assert new `Cloud` object is in `slide.layer("about").objects` and NOT in `slide.objects`.
  3. Repeat on a master slide created with `master.add`.
- **Expected result:** Shape drag preview works on layers and masters, and places into the active container.
- **Priority:** P2

### A5.7 — Snap to grid and snap to guides active during shape drag preview
- **What it checks:** With `show_grid = true`, `snap_to_grid = true`, `snap_to_guides = true`, dragging `ShapeKind::Triangle` snaps the preview outline and dimension badge `"W × H"` to the same frame that gets placed.
- **Why it might break:** If preview draws the un-snapped raw pointer rect while mouse-up snaps to grid, the outline jumps on release.
- **Where / how:** `crates/ui-egui/src/shape_preview_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Enable `snap_to_grid = true` (e.g. via `view.set`), pick `ShapeKind::Triangle`, drag `(403, 217)` by `(173, 129)`.
  2. Record `preview_frame = editor.ui.placement_drag.as_ref().unwrap().frame` and assert dimension label `format!("{:.0} × {:.0}", ...)` is painted.
  3. Release mouse and compare `placed.frame` with `preview_frame`.
- **Expected result:** `placed.frame` equals `preview_frame` within `0.5px` and the `"W × H"` badge is visible.
- **Priority:** P2

### A5.8 — Dark vs Light theme and non-shape placement tools (`Text`, `Button`)
- **What it checks:** Shape drag preview renders with Light theme's `p.accent` as well as Dark theme's, and non-shape placement tools (`PlacementTool::Text`, `PlacementTool::Button`) still show rectangular placement preview without regressing.
- **Why it might break:** Hardcoded colours or overly broad `if let` branches in `draw_placement_preview` could break Light theme contrast or break Text/Button placement previews.
- **Where / how:** `crates/ui-egui/src/shape_preview_tests.rs` using `TestHarness::new_with_theme`.
- **Setup & steps:**
  1. Create `TestHarness::new_with_theme(hello(), (1280.0, 900.0), ThemeMode::Light)`, pick `ShapeKind::Heart`, drag → verify mesh uses Light `p.accent`.
  2. Pick toolbar `"Text"` tool (`PlacementTool::Text`), drag → verify `Shape::Rect` preview with `p.accent` stroke is painted and releases a `Text` object.
- **Expected result:** Both themes render crisp previews; Text/Button tools keep rectangular preview.
- **Priority:** P2

### A5.9 — Freehand/Line/Connector draw tools (`DrawTool`) preview parity
- **What it checks:** Selecting `Line`, `Arrow`, `CurvedConnector`, and `Curve` from the shape/draw toolbar and dragging on canvas still paints `draw_draw_preview` (`canvas.rs:4873–4920`) and places an `ObjectKind::Path`.
- **Why it might break:** Shared toolbar/gallery plumbing between `PlacementTool::Shape` and `DrawTool` could regress line/connector drag previews.
- **Where / how:** `crates/ui-egui/src/shape_preview_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Set `DrawTool::Line` (or click Line in Shapes gallery), drag `(200, 200)` to `(500, 350)`.
  2. Assert `Shape::Path` / line segment in `p.accent` is painted during drag; release and assert `ObjectKind::Path` is added.
  3. Repeat for `DrawTool::CurvedConnector`.
- **Expected result:** Line and curved connector previews render accurately and create valid `Path` objects.
- **Priority:** P3

---

## A6. Dialog reopening at full size

**Bug class:** An egui `Window` built via `Dialog::new` remembers a collapsed/zeroed size in `ctx.memory` after closing (especially when closing itself inside a button callback or via Escape/X), reopening on the 2nd visit as a tiny title-bar-only strip.

### A6.1 — Every card in `Insert Interaction` gallery closes and reopens at full size
- **What it checks:** Picking each of the 12 cards in `interaction_gallery.rs` (`Tabs`, `Accordion`, `Marker`, `Scrolling panel`, `Timer`, `Progress meter`, `Flip cards`, `Hotspot reveal`, `Process steps`, `Timeline`, `Reveal grid`, `Scenario choice`), then reopening `Insert ▸ Interaction…`.
- **Why it might break:** Some cards run `insert_interaction` (multi-layer batch), others run `insert_marker` / `insert_timer` / `insert_scroll_panel` / `insert_progress_meter` (which also creates a variable); side effects on editor state or revision during the close frame could differ by card.
- **Where / how:** `crates/ui-egui/src/dialog_reopen_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. For each card name in `CARDS`:
     - Fresh `TestHarness::new(hello())`, open `Insert ▸ Interaction…`, capture `first_rect = dialog_rect(&h, "Insert Interaction")`.
     - Click the card name, `idle_frame()`, assert `!h.editor.ui.show_interaction_gallery`.
     - Reopen `Insert ▸ Interaction…`, capture `second_rect` and run `layout_report(2)`.
     - Assert `second_rect.width() >= 700.0 && second_rect.height() >= 300.0` and `report.findings.is_empty()`.
- **Expected result:** All 12 interaction cards leave the gallery reopening at full size with zero layout issues.
- **Priority:** P1

### A6.2 — `Insert Interaction` closed by X button, by Escape, and by card pick in sequence
- **What it checks:** Closing via title-bar `×`, reopening, closing via `Escape`, reopening, closing via card click, reopening — checking exact rect stability every time.
- **Why it might break:** Closing via `window.open(&mut open)` vs `Escape` vs setting `show_interaction_gallery = false` executes on different parts of the frame lifecycle in egui.
- **Where / how:** `crates/ui-egui/src/dialog_reopen_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Open `Insert Interaction`, record `r1`. Close by clicking title-bar close button (top-right of `r1`).
  2. Reopen, record `r2`. Press `Key::Escape`.
  3. Reopen, record `r3`. Click `"Accordion"`.
  4. Reopen, record `r4`.
  5. Assert all `r1..r4` have `width >= 700.0`, `height >= 300.0`, and stay inside `dialog_area`.
- **Expected result:** All three close mechanisms reopen at full size.
- **Priority:** P1

### A6.3 — `Shapes` gallery across both Insert menu and Toolbar button, plus search filter state
- **What it checks:** Opening `Shapes` via Toolbar icon, filtering `"Heart"`, picking `Heart`, reopening via `Insert ▸ Shape`, verifying full size and usable gallery content.
- **Why it might break:** Filtering reduces gallery height to 1 row; if that shrunk height is persisted in egui area memory when closed, reopening with cleared search could stay stuck at 1-row height!
- **Where / how:** `crates/ui-egui/src/dialog_reopen_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Open `Shapes` via Toolbar `Icon::Shapes` button → record `r1` (`height >= 200`).
  2. Type `"Heart"` in search box (shrinks matches to 1 item), click `Heart` to close.
  3. Reopen `Shapes` via `Insert ▸ Shape` → record `r2`, run `layout_report(2)`.
  4. Assert `r2.width() >= 300.0 && r2.height() >= 200.0` and multiple shapes are visible.
- **Expected result:** Filtering before close never traps the reopened `Shapes` gallery at shrunk height.
- **Priority:** P1

### A6.4 — Comprehensive sweep of all other `Dialog::new` dialogs (open → close → reopen twice)
- **What it checks:** Every dialog using `Dialog::new` that can close itself via action button, X, or Escape:
  1. `Insert Web Object` (`web_insert.rs:62`) — close via Cancel and via Insert with `https://example.com`
  2. `Slide Size` (`slide_size_dialog.rs:73`) — close via Apply/OK and Cancel
  3. `Link` (`properties.rs:3886`) — close via Apply (`https://example.com`) and Cancel
  4. `Text styles` (`text_styles_ui.rs:194`)
  5. `Theme` (`theme_dialog.rs:52`) — close via Apply/Close
  6. `Slides from Outline` (`outline.rs:127`)
  7. `Course Settings` (`course_settings.rs:243`)
  8. `Publish course` (`publish.rs:223`)
  9. `Export for Review` (`export_dialog.rs:205`)
  10. `Find and Replace` (`find.rs:108`, `pinned_top()`)
  11. `Preferences` (`preferences.rs:77`)
  12. `AI Settings` (`ai_settings.rs:430`)
  13. `Trigger` editor dialog (`triggers.rs:2177`) — save trigger → reopen to add another trigger
  14. `Layer Properties` (`layers.rs:1080`)
  15. `Delete scene` (`slides.rs:2132`) / `Rename timeline cue` (`timeline.rs:2066`)
- **Why it might break:** Aurelie's spec explicitly says *"Check every other dialog that closes itself after a choice."* Any dialog without `.width(...)` or `set_min_width(...)` inside `ScrollArea::auto_shrink([true, true])` (`dialog.rs:219`) is vulnerable to collapsing on 2nd open!
- **Where / how:** `crates/ui-egui/src/dialog_reopen_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. For each dialog, open via its real menu/button trigger, warm up 2 frames, record `r1 = area_rect(id)` and assert `r1.width() >= 220.0 && r1.height() >= 100.0`.
  2. Close via its primary action/cancel/Escape, `idle_frame()`.
  3. Reopen via the same trigger, warm up 2 frames, record `r2`, run `layout_report(1)`.
  4. Assert `r2.width() >= r1.width() * 0.9`, `r2.height() >= r1.height() * 0.9`, `r2.height() >= 100.0`, and `report.findings.is_empty()`.
- **Expected result:** Zero dialogs collapse to title-bar-only or clip widgets on reopen.
- **Priority:** P1

### A6.5 — Dialog dragged by title bar, closed, and reopened
- **What it checks:** Opening `Insert Interaction` (and `Theme`), dragging its title bar by `vec2(120.0, 60.0)`, closing via choice/button, and reopening.
- **Why it might break:** Dragging writes a persisted position/size into egui `AreaState`; reopening could snap partially off-screen, cover the menu bar, or lose its width.
- **Where / how:** `crates/ui-egui/src/dialog_reopen_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Open `Insert Interaction`, get `r1`. Drag title bar (`pos2(r1.center().x, r1.min.y + 12.0)`) by `vec2(120.0, 60.0)`.
  2. Click `"Marker"` to close. Reopen `Insert Interaction`, get `r2`.
  3. Run `layout_report(2)` (which runs `check_windows` against screen & menu bar).
- **Expected result:** `r2` is full size (`>= 700×300`), inside `screen_rect`, and does not overlap the menu bar (`report.findings.is_empty()`).
- **Priority:** P2

### A6.6 — Main window shrinks between 1st and 2nd dialog open, then enlarges again
- **What it checks:** Open `Insert Interaction` at `1280×900` → close → shrink `harness.screen_rect` to `900×600` → reopen → close → enlarge back to `1280×900` → reopen.
- **Why it might break:** Shrinking constrains `Window` via `.max_size(area.size())`; enlarging again could leave the dialog permanently stuck at the shrunk `900×600` size (or clip off-screen at `900×600`).
- **Where / how:** `crates/ui-egui/src/dialog_reopen_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Perform the 3-step open/close cycle across `1280×900` → `900×600` → `1280×900`.
  2. At each open, assert `layout_report(2).findings.is_empty()` and check dialog rect fits inside `screen_rect` at `900×600` and is full width/height at `1280×900`.
- **Expected result:** Dialog adapts cleanly to viewport shrink and restores full size when enlarged.
- **Priority:** P2

### A6.7 — Zero idle frames between closing and reopening a dialog
- **What it checks:** Picking `"Tabs"` in `Insert Interaction` and reopening `Insert Interaction` on the very next frame (no extra `idle_frame()`s between close and reopen).
- **Why it might break:** If egui's window `open` transition needs a frame where the `Window` is absent to reset internal state, reopening on the immediate next frame can skip initialization and collapse.
- **Where / how:** `crates/ui-egui/src/dialog_reopen_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Open `Insert Interaction`, click `"Tabs"`.
  2. Immediately set `editor.ui.show_interaction_gallery = true` (or click Insert → Interaction on the very next frame without extra idle frames), then run 2 frames.
  3. Assert `dialog_rect` is `>= 700×300`.
- **Expected result:** Immediate reopen still opens at full size.
- **Priority:** P3

### A6.8 — Alternating between two dialogs (`Insert Interaction` ↔ `Shapes` ↔ `Theme`)
- **What it checks:** Opening `Insert Interaction` → picking `Marker` → opening `Shapes` → picking `Heart` → opening `Theme` → closing → reopening `Insert Interaction`.
- **Why it might break:** Shared temp state in `ctx.data` (`dialog_footer_height`, `top_inset_id`) or `Id` collisions could cause one dialog's dimensions to bleed into another.
- **Where / how:** `crates/ui-egui/src/dialog_reopen_tests.rs` using `TestHarness`.
- **Setup & steps:** Run the alternating sequence above, asserting each dialog's expected width/height and zero `layout_report` findings on every open.
- **Expected result:** Each dialog retains its own exact geometry with zero cross-contamination.
- **Priority:** P2

### A6.9 — Fixed-footer dialogs (`show_with_footer`) on a short screen (`1024×620`) opened twice
- **What it checks:** `Theme`, `Publish course`, `Course Settings`, and `Export for Review` opened, closed, and reopened on a short `1024×620` viewport.
- **Why it might break:** `DialogUi::footer` stores `footer_id(window_id)` in `ctx.data` (`dialog.rs:240–243`) and `body` subtracts it from `max_body_height`. On 2nd open, stale or zeroed `footer_h` could clip the footer buttons at the bottom of the window.
- **Where / how:** `crates/ui-egui/src/dialog_reopen_tests.rs` using `TestHarness::new_with_size(hello(), (1024.0, 620.0))`.
- **Setup & steps:** For each of the 4 footer dialogs: open → `layout_report(3)` (assert zero findings) → close → reopen → `layout_report(3)` (assert zero findings and footer primary button visible inside dialog rect).
- **Expected result:** Footers remain unclipped and clickable on both first and second open on short screens.
- **Priority:** P1

### A6.10 — Undo after `Insert Interaction` choice, then reopen gallery
- **What it checks:** Open `Insert Interaction` → pick `"Tabs"` → Undo (`Ctrl+Z`) → reopen `Insert Interaction` → pick `"Accordion"`.
- **Why it might break:** Undo resets `session.revision` and layer selection; ensure the reopened dialog stays full size and `"Accordion"` inserts cleanly onto the reverted slide.
- **Where / how:** `crates/ui-egui/src/dialog_reopen_tests.rs` using `TestHarness`.
- **Setup & steps:** Perform sequence above; assert gallery rect `>= 700×300` on 2nd open and `course.validate()` has zero errors after `"Accordion"` is inserted.
- **Expected result:** Full-size reopen and clean course model after Undo + second interaction insert.
- **Priority:** P2

---

## A7. Video selection & decode safety

**Bug class:** Clicking/selecting a video (especially short, corrupt, missing, or slow-to-decode videos on Windows/desktop) freezes the UI thread, retries decoding in an infinite repaint loop, or panics/crashes instead of showing a plain placeholder message.

### A7.1 — Corrupt/garbage `.mp4` imported via `import_video` and selected on canvas
- **What it checks:** Importing invalid bytes (`b"not a real mp4 file"`) as `"corrupt.mp4"`, placing it on a slide, deselecting, clicking it to select, and running 15 frames with the real default backend (or failing backend).
- **Why it might break:** `import_video` succeeds for arbitrary `.mp4` bytes, then clicking/selecting arms the native video decoder; if the native backend panics, blocks synchronously, or fails to report `ended`/`video_error`, the editor freezes or spins forever.
- **Where / how:** `crates/ui-egui/src/video_selection_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. `harness.editor.import_video("corrupt.mp4", b"not a real mp4")`, click canvas to place, deselect, click video to select.
  2. Advance 15 frames with `frame_at_time` spanning `0.0..7.0` seconds; measure wall-clock elapsed time (`< 5s`).
  3. Assert selection is the video object, `!timeline_playback.needs_repaint(0)`, and either `"Could not decode video"` or `"Video preview timed out"` (or backend error) is displayed on canvas.
- **Expected result:** Editor stays responsive, shows a plain-language placeholder, and stops repainting.
- **Priority:** P1

### A7.2 — Empty (0-byte) video file and missing media (`src` absent from `editor.media`)
- **What it checks:** A course containing two video objects: `"v-empty"` (0-byte `"empty.mp4"` in `editor.media`) and `"v-missing"` (`"missing.mp4"` not in `editor.media`). Selecting, moving, resizing, and toggling `Autoplay`/`Controls`/`Loop` in the Format panel.
- **Why it might break:** Passing empty `Arc<[u8]>` to the native decoder or `sync_tracks` missing media bytes can trigger divide-by-zero/EOF loops or panic on empty slices.
- **Where / how:** `crates/ui-egui/src/video_selection_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Load course with `"v-empty"` and `"v-missing"`, insert `("empty.mp4", Arc::from(&[][..]))` into `editor.media`.
  2. Click `"v-empty"` → run 6 frames → toggle `"Autoplay"` checkbox in right panel.
  3. Click `"v-missing"` → run 6 frames → drag it by `vec2(40.0, 20.0)` on canvas.
- **Expected result:** No panic, no infinite retries, both videos remain selectable/editable and `course.validate()` reports only the expected missing-media issue for `"v-missing"`.
- **Priority:** P1

### A7.3 — Explicit backend error string surfaces on canvas (`TimelineMedia::video_error`)
- **What it checks:** When `TimelineMedia::video_error` returns `Some("Unsupported codec: hevc")`, the canvas displays that exact plain error message instead of generic text or blank black box, and stops retrying.
- **Why it might break:** `drain_ended` (`timeline_playback.rs:402`) and `update_video_decode_status` (`timeline_playback.rs:432`) both read `video_error`; if `ended` arrives before `video_error` is checked (or `video_error` returns without `ended`), the message could be lost or retried.
- **Where / how:** `crates/ui-egui/src/video_selection_tests.rs` extending `StubTimelineMedia` with an `error: Option<String>` field.
- **Setup & steps:**
  1. Test both cases: (a) backend pushes `ended` AND returns `video_error() = Some("Unsupported codec")`, (b) backend does NOT push `ended` yet, only returns `video_error() = Some("Unsupported codec")`.
  2. Click video, run 5 frames.
  3. Assert `play_calls == 1`, `!needs_repaint(0)`, and `find_text("Unsupported codec").is_some()`.
- **Expected result:** Specific backend error message renders on canvas and stops further decode attempts in both paths.
- **Priority:** P1

### A7.4 — Two videos on one slide (one working, one failing/hanging) + rapid clicking between them
- **What it checks:** Slide with `"v-good"` (returns a valid `VideoFrame`) and `"v-bad"` (fails/hangs); clicking `"v-bad"` → `"v-good"` → `"v-bad"` → multi-selecting both.
- **Why it might break:** A shared failure flag or `stop_all()` on error could kill `"v-good"`'s decoded frame when `"v-bad"` fails or times out!
- **Where / how:** `crates/ui-egui/src/video_selection_tests.rs` with a per-object `StubTimelineMedia`.
- **Setup & steps:**
  1. Configure `"v-good"` to return a 4×4 green `VideoFrame` at `pts_ms = 0`, and `"v-bad"` to hang past 5s timeout.
  2. Advance simulated time `0.0..6.5s`, clicking alternately between `"v-bad"` and `"v-good"`.
  3. Assert `"v-bad"` shows `"Video preview timed out"` while `"v-good"` still returns `Some(frame)` and has `video_error_for("v-good") == None`.
- **Expected result:** Failure/timeout is strictly isolated per video object; healthy videos continue rendering frames normally.
- **Priority:** P1

### A7.5 — Delayed video (`start_ms > 0`) scrubbed onto, off, and back onto
- **What it checks:** A failing video with `timing: { start_ms: 2000, end_ms: 4000 }`: at `t=0` it is not armed; scrub to `2500ms` (arms once and fails); scrub back to `500ms`; scrub again to `3000ms`.
- **Why it might break:** `sync_tracks` / `arm_videos_at` could clear `track.failed` when scrubbing outside `[start_ms, end_ms]`, causing repeated failed decode loops every time the author scrubs past the video.
- **Where / how:** `crates/ui-egui/src/video_selection_tests.rs` using `TestHarness` + `StubTimelineMedia::failing()`.
- **Setup & steps:**
  1. At `playhead = 0`: assert `play_calls == 0` (outside timing window).
  2. Scrub to `2500ms`, run 4 frames: assert `play_calls == 1` and `"Could not decode video"` visible.
  3. Scrub to `500ms`, run 2 frames; scrub back to `3000ms`, run 4 frames.
  4. Assert `play_calls == 1` (still cached as failed, not re-armed) and error message still shown.
- **Expected result:** Delayed video only arms when playhead enters its window and caches its failure across scrubs.
- **Priority:** P2

### A7.6 — Video placed on a Slide Layer
- **What it checks:** Placing a failing video (and a real `stretch.mp4` video) on layer `"L1"`, switching Base ↔ `"L1"`, and clicking the video on `"L1"`.
- **Why it might break:** `timeline_track_specs` (`timeline_playback.rs:159–163`) collects tracks from all layers, so a layer video could be armed while on Base or lose its error placeholder when switching layers.
- **Where / how:** `crates/ui-egui/src/video_selection_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Course with failing video `"vid-layer"` on layer `"L1"`.
  2. Switch to `"L1"`, click `"vid-layer"`, run 6 frames.
  3. Assert `selection.object == Some("vid-layer")`, `play_calls == 1`, `"Could not decode video"` visible, switch back to Base and back to `"L1"` without crashing or re-arming.
- **Expected result:** Layer videos select, display placeholders, and avoid retry loops identically to base videos.
- **Priority:** P2

### A7.7 — Dragging, resizing, duplicating, deleting, and undoing delete of a video
- **What it checks:** With a failing video (and with `stretch.mp4`): drag it on canvas, `Ctrl+D` duplicate it, `Delete` it, `Ctrl+Z` undo delete, and `Ctrl+Y` redo delete.
- **Why it might break:** `sync_tracks` compares `object_id/src/start_ms/end_ms` and preserves `prev_state` (`timeline_playback.rs:362–389`); deleting and undoing re-adds the same `object_id` and must not hang or panic when re-arming.
- **Where / how:** `crates/ui-egui/src/video_selection_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Select `"vid1"` (with `StubTimelineMedia::failing()`), drag `vec2(50.0, 30.0)`, `press(Key::D, Modifiers::COMMAND)`, assert 2 video objects exist.
  2. Press `Key::Delete`, `idle_frame()`, assert 1 video object remains.
  3. `session.undo()`, run 4 frames, assert 2 video objects exist again and neither loops repaints.
- **Expected result:** Move, duplicate, delete, and undo/redo on video objects keep playback state clean and responsive.
- **Priority:** P2

### A7.8 — Preview modal with both a broken video and real `stretch.mp4`
- **What it checks:** Opening Preview (`editor.open_preview()`) on a slide with a broken/missing video and on a slide with `examples/healthy-habits/media/stretch.mp4`, ticking 10 frames, and closing Preview.
- **Why it might break:** Preview uses `MediaDriver` + `Player` (separate from editor `TimelinePlayback`); a broken video could crash/hang Preview or leak a running worker thread after Preview closes.
- **Where / how:** `crates/ui-egui/src/video_selection_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Course with `broken.mp4` (`b"garbage"`) → `open_preview()` → run 10 `frame_at_time` frames → close via `Escape` → assert `editor.preview.is_none()` and editor responds to subsequent clicks.
  2. Repeat with real `examples/healthy-habits/media/stretch.mp4` → `open_preview()` → run 5 frames → close via `Escape`.
- **Expected result:** Preview never freezes or crashes on broken or real video and stops all media cleanly on close.
- **Priority:** P1

### A7.9 — Video with trim (`object.set_media`) and cue points (`video.add_cue`)
- **What it checks:** Adding a pausing cue point at `1500ms` and setting trim `start_ms: 500, end_ms: 2500` on a video object, selecting it, scrubbing across the trim range, and validating.
- **Why it might break:** Trim offsets combined with `playhead_ms - timing.start_ms` seek offsets can seek past EOF of a short (e.g. 2-second) video — matching Aurelie's "few-second video" report!
- **Where / how:** `crates/ui-egui/src/video_selection_tests.rs` using `TestHarness` + real `stretch.mp4`.
- **Setup & steps:**
  1. Import and place `stretch.mp4` on a 10,000ms slide (longer than the short video!).
  2. Run `object.set_media` with `trim: { start_ms: 500, end_ms: 2000 }` and `video.add_cue` at `1200ms`.
  3. Scrub ruler to `0ms`, `1500ms`, `5000ms` (past video EOF!), and `9000ms`; click video to select at each point.
- **Expected result:** Scrubbing past a short video's actual duration never freezes, loops, or crashes; `course.validate()` passes.
- **Priority:** P3

### A7.10 — Native playback backend (`coursecraft-playback`) hostile-bytes stress test
- **What it checks:** Feeding 0-byte, random garbage, truncated MP4 `ftyp` header, and oversized fake `moov` atom bytes directly into `coursecraft_playback::create_default_backend()` (`play` → `pause` → `seek(0)` → `seek(999_999)` → `stop_all`).
- **Why it might break:** `coursecraft-video-native` is the ONE crate allowed `unsafe` (OS AVFoundation / Media Foundation calls). Hostile bytes or out-of-range seeks must return an error cleanly within bounded time without crashing or leaking threads.
- **Where / how:** `crates/playback/src/tests.rs`.
- **Setup & steps:**
  1. For each hostile payload (`&[]`, `b"not-a-video"`, `b"\x00\x00\x00\x18ftypisom\x00\x00\x02\x00"`, truncated valid `stretch.mp4` first 128 bytes):
     - Call `backend.play("v", "bad.mp4", MediaKind::Video, bytes)`, `backend.pause("v")`, `backend.seek("v", 0)`, `backend.seek("v", 5000)`.
     - Poll `video_error("v")` / `take_ended()` for up to 3 seconds wall-clock, then `backend.stop_all()`.
     - Assert clean completion (no panic, no hang > 3s, error or ended reported).
- **Expected result:** Native decoder rejects every malformed payload safely and promptly.
- **Priority:** P3

---

## A8. Windows installer & portable packaging

**Bug class:** Broken Inno Setup script, missing bundled files/icons, mismatch between `.github/workflows/release.yml` and `packaging/windows/coursecraft.iss`, silent CI failures (`continue-on-error: true`), or confusing/stale portable-vs-installer documentation.

### A8.1 — `packaging/windows/coursecraft.iss` required installer directives
- **What it checks:** Parses `packaging/windows/coursecraft.iss` and verifies every user-facing Windows installer requirement:
  - Fixed `AppId=` GUID (`{{7B6E3A4C-5F1A-4D8E-9C2B-8A1F3E5D7C90}`) so upgrades replace cleanly
  - `PrivilegesRequired=lowest` (no admin UAC required)
  - `DefaultDirName={localappdata}\Programs\CourseCraft`
  - `UninstallDisplayName=CourseCraft` and `UninstallDisplayIcon={app}\coursecraft.exe`
  - Start Menu shortcut under `[Icons]`: `{autoprograms}\CourseCraft` → `{app}\coursecraft.exe`
  - Optional Desktop shortcut task (`desktopicon`, `Flags: unchecked`) + `[Icons]` `{autodesktop}\CourseCraft`
  - 64-bit directives (`ArchitecturesAllowed=x64compatible`, `ArchitecturesInstallIn64BitMode=x64compatible`)
  - `[Run]` postinstall launch with `nowait postinstall skipifsilent`
- **Why it might break:** Accidental edits to `.iss` (dropping `PrivilegesRequired=lowest`, breaking `{autoprograms}`, or changing `AppId`) silently break Windows installation or create duplicate entries in Settings → Apps.
- **Where / how:** `apps/coursecraft-cli/tests/m26_cli_and_packaging.rs` (runs on all OSes!).
- **Setup & steps:** Read `packaging/windows/coursecraft.iss` relative to `CARGO_MANIFEST_DIR/../../` and assert every required section and key/value line is present.
- **Expected result:** All installer contract directives are present and exact.
- **Priority:** P1

### A8.2 — Every asset referenced by `coursecraft.iss` exists and is valid
- **What it checks:** Every `{#RepoRoot}\...` file referenced in `coursecraft.iss` (`apps/coursecraft/icons/coursecraft.ico`, `LICENSE-MIT`, `LICENSE-APACHE`, `ATTRIBUTION.md`) exists on disk, is non-empty, and `coursecraft.ico` starts with the valid ICO header bytes `[0x00, 0x00, 0x01, 0x00]` with image count >= 1.
- **Why it might break:** Renaming or moving an icon or license file breaks the Inno Setup build only when a release tag is pushed!
- **Where / how:** `apps/coursecraft-cli/tests/m26_cli_and_packaging.rs`.
- **Setup & steps:** Parse all `SetupIconFile=` and `Source:` lines referencing `{#RepoRoot}` in `coursecraft.iss`, resolve them against repo root, assert each exists and is non-empty, and inspect `coursecraft.ico` header bytes.
- **Expected result:** Every bundled file exists and `coursecraft.ico` is a valid ICO file.
- **Priority:** P1

### A8.3 — `.github/workflows/release.yml` ↔ `coursecraft.iss` defines and artifact names parity
- **What it checks:** Every `/D<Key>=...` passed to `$iscc` in `.github/workflows/release.yml` (`AppVersion`, `OutputName`, `OutputDir`, `RepoRoot`) matches a `#ifndef <Key>` macro in `coursecraft.iss`, and the produced filenames (`CourseCraft-<tag>-setup.exe` and `CourseCraft-<tag>-portable.exe`) match `README.md`.
- **Why it might break:** A typo in a `/D` macro name (e.g. `/DOutName` vs `OutputName`) silently falls back to the default `CourseCraft-setup.exe` without version tag, breaking release links.
- **Where / how:** `apps/coursecraft-cli/tests/m26_cli_and_packaging.rs`.
- **Setup & steps:** Read `.github/workflows/release.yml` and `packaging/windows/coursecraft.iss`; extract all `/D([A-Za-z0-9_]+)=` tokens from `release.yml` and assert each appears in `#ifndef <Name>` in `coursecraft.iss`; assert `release.yml` builds both `CourseCraft-$($env:GITHUB_REF_NAME)-portable.exe` and `OutputName=CourseCraft-$tag-setup`.
- **Expected result:** CI workflow defines and `.iss` macros match 1:1.
- **Priority:** P1

### A8.4 — Release workflow installer failure gate audit (`continue-on-error`)
- **What it checks:** Checks `.github/workflows/release.yml` for `continue-on-error: true` on the `Build Windows installer` step (`release.yml:53`).
- **Why it might break:** With `continue-on-error: true`, if `choco install innosetup` or `ISCC.exe` fails during a release build, GitHub Actions still marks the Windows job GREEN and publishes a release missing `-setup.exe` — reproducing Aurelie's exact complaint!
- **Where / how:** `apps/coursecraft-cli/tests/m26_cli_and_packaging.rs`.
- **Setup & steps:** Inspect the `Build Windows installer` step in `.github/workflows/release.yml`; assert `continue-on-error: true` is NOT set on the installer step (if currently present, this test is expected to catch and flag it for review!).
- **Expected result:** Release workflow fails loudly if the Windows installer fails to build instead of silently shipping only the portable `.exe`.
- **Priority:** P1

### A8.5 — Documentation clearly explains Windows Installer vs Portable `.exe`
- **What it checks:** `README.md`, `docs/cli.md`, and `packaging/windows/README.md` document both `-setup.exe` (Start menu, Settings → Apps uninstall, `%LOCALAPPDATA%\Programs\CourseCraft`) and `-portable.exe` (standalone, no installation step).
- **Why it might break:** Users downloading `-portable.exe` won't know why it isn't in the Start menu unless README/release notes clearly contrast the two downloads.
- **Where / how:** `apps/coursecraft-cli/tests/m26_cli_and_packaging.rs`.
- **Setup & steps:** Read `README.md` and assert it mentions `-setup.exe`, `-portable.exe`, `Start menu`, `uninstall` / `Settings → Apps`, and `%LOCALAPPDATA%\Programs\CourseCraft`; read `packaging/windows/README.md` and assert it references `coursecraft.iss` and `release.yml`.
- **Expected result:** Documentation accurately guides Windows users to the installer and clearly labels the portable `.exe`.
- **Priority:** P2

### A8.6 — Version string handling for semver & pre-release tags in `coursecraft.iss`
- **What it checks:** Verifies `coursecraft.iss` does not set numeric-only `VersionInfoVersion={#AppVersion}` (which fails in Inno Setup on tags like `0.2.5-rc.1`), while `AppVersion={#AppVersion}` and `AppVerName=CourseCraft {#AppVersion}` accept arbitrary semver strings, and `release.yml` strips leading `v` (`TrimStart('v')`).
- **Why it might break:** Adding `VersionInfoVersion={#AppVersion}` later would break pre-release tags like `v0.3.0-beta.1` at ISCC compile time.
- **Where / how:** `apps/coursecraft-cli/tests/m26_cli_and_packaging.rs`.
- **Setup & steps:** Check `coursecraft.iss` lines for `AppVersion`, `AppVerName`, and absence of raw `VersionInfoVersion={#AppVersion}`; check `release.yml` uses `TrimStart('v')`.
- **Expected result:** Both stable and pre-release tags compile cleanly with ISCC.
- **Priority:** P2

### A8.7 — Portable vs installed app does not write config beside the executable
- **What it checks:** Inspects `apps/coursecraft` / `crates/control` config path logic to verify app config/preferences/control files resolve under the OS user config directory (not relative to `current_exe()` or current working directory), so both installed (`%LOCALAPPDATA%\Programs\CourseCraft`) and portable runs work reliably.
- **Why it might break:** Writing next to the exe breaks when installed in read-only directories or leaves stray files in `Downloads` when run portably.
- **Where / how:** Unit test in `crates/control` or `apps/coursecraft-cli/tests/m26_cli_and_packaging.rs`.
- **Setup & steps:** Call the control-file path helper (if public) and assert it does not point inside the test binary's directory and is an absolute path under user config/data dir.
- **Expected result:** Runtime state lives in the OS user config directory regardless of install mode.
- **Priority:** P2

### A8.8 — Inno Setup `ISCC.exe` compile smoke test (Windows / when `iscc` is installed)
- **What it checks:** Actually compiles `packaging/windows/coursecraft.iss` using `ISCC.exe` against a dummy executable in `target/installer-test/coursecraft.exe` and verifies `target/installer-test/CourseCraft-v0.0.0-test-setup.exe` is created.
- **Why it might break:** Static checks cannot catch Inno Setup syntax typos, invalid `[Icons]` flags, or bad relative paths on Windows.
- **Where / how:** `apps/coursecraft-cli/tests/m26_cli_and_packaging.rs` (`#[cfg(target_os = "windows")]`, skipping gracefully if `ISCC.exe` is not installed unless `COURSECRAFT_REQUIRE_ISCC=1`).
- **Setup & steps:** Create dummy `target/installer-test/coursecraft.exe`, invoke `ISCC.exe` with the exact `/D` flags from `release.yml` plus `/DSourceExe=...`, assert exit code 0 and output `.exe` exists and is `> 100 KB`.
- **Expected result:** Inno Setup compiles `coursecraft.iss` with zero errors.
- **Priority:** P3

### A8.9 — Silent Windows install + uninstall verification (`#[ignore]`, Windows VM/CI)
- **What it checks:** Runs the compiled installer with `/VERYSILENT /SUPPRESSMSGBOXES /DIR=<temp>`, asserts `coursecraft.exe`, license files, and `unins000.exe` are installed, then runs `unins000.exe /VERYSILENT` and asserts clean directory removal.
- **Why it might break:** Verifies end-to-end Start menu / uninstaller registration and clean uninstall on real Windows.
- **Where / how:** `apps/coursecraft-cli/tests/m26_cli_and_packaging.rs` (`#[cfg(target_os = "windows")] #[ignore]`).
- **Setup & steps:** Build installer per A8.8, run silent install to `target/installer-test/installed`, verify files exist, run `unins000.exe /VERYSILENT`, wait for exit, verify `coursecraft.exe` is removed.
- **Expected result:** Installs and uninstalls cleanly with zero leftover executable.
- **Priority:** P3

---

## A9. Slide layers: Base slide view (Show / Dim / Hide)

**Bug class:** While editing a layer tab, the base slide bleeds through unreadably, or the `Base slide: Show | Dim | Hide` toggle fails on certain object types, conflicts with other layer toggles, leaks into runtime playback, or breaks selection/layout.

### A9.1 — Interaction between `BaseSlideView::Dim` and `editor.ui.layers.dim_other_layers`
- **What it checks:** Turning off `editor.ui.layers.dim_other_layers` (in the Slide Layers panel) while editing a layer, then clicking `"Dim"` on the canvas `"Base slide: Show | Dim | Hide"` pill.
- **Why it might break:** Look at `crates/ui-egui/src/canvas.rs:5463–5466`:
  ```rust
  let dim_base = editor.selection.layer.is_some()
      && editor.session.course.editor.base_slide_view == BaseSlideView::Dim
      && editor.ui.layers.dim_other_layers;
  ```
  If the user unchecked "Dim other layers" in the Layers panel, clicking `"Dim"` in the `"Base slide:"` bar appears selected (`BaseSlideView::Dim`), yet `dim_base` evaluates to `false` and the base slide stays at full alpha!
- **Where / how:** `crates/ui-egui/src/base_slide_view_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Switch to `"About"` layer on `s-intro`, uncheck `dim_other_layers` (via UI or `editor.ui.layers.dim_other_layers = false`).
  2. Click `"Show"` then click `"Dim"` in the `"Base slide:"` pill bar.
  3. Capture `frame_shapes` and check `"Start"` text alpha on canvas vs full alpha on Base.
- **Expected result:** Explicitly selecting `"Dim"` in the `"Base slide:"` toggle MUST dim the base slide (`dim_color.a() < full_color.a()`) even if `dim_other_layers` was previously toggled off!
- **Priority:** P1

### A9.2 — Editor `BaseSlideView::Show` vs runtime `layer.hide_base_objects = true`
- **What it checks:** Setting `layer.hide_base_objects = true` in Layer Properties on `"about"`, then toggling editor `Base slide` between `Show`, `Dim`, and `Hide`, and finally running `Player` + `compose(&player)`.
- **Why it might break:** Confusing authoring `base_slide_view` with runtime `layer.hide_base_objects` could either break the author's ability to preview/align against base objects or cause `BaseSlideView::Hide` to leak into runtime playback.
- **Where / how:** `crates/ui-egui/src/base_slide_view_tests.rs` using `TestHarness` + `Player`.
- **Setup & steps:**
  1. Run `layer.update` setting `"hide_base_objects": true` on `"about"`.
  2. In editor on layer `"about"`, test `Show` / `Dim` / `Hide` and verify layer objects stay editable.
  3. In `Player`, click `"btn-about"` to show layer `"about"`, call `compose(&player)` — assert base objects ARE hidden at runtime (`hide_base_objects: true`) even when editor `base_slide_view == BaseSlideView::Show`.
  4. Flip `hide_base_objects: false` and `base_slide_view == BaseSlideView::Hide` — assert base objects ARE shown at runtime in `Player`.
- **Expected result:** Runtime strictly obeys `layer.hide_base_objects`; editor `base_slide_view` never leaks into `Player`.
- **Priority:** P1

### A9.3 — Master slide objects under `Show`, `Dim`, and `Hide`
- **What it checks:** Slide using a master with a footer text `"Master Footer"` and shape: editing layer `"about"` under `Show`, `Dim`, and `Hide`.
- **Why it might break:** `is_base_slide_item` (`canvas.rs:5478`) checks whether an item belongs to the base slide; master items (`Hit::None` or master hit) might remain bright/unhidden in `Hide` mode, still cluttering the layer view!
- **Where / how:** `crates/ui-egui/src/base_slide_view_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Create a master with `"Master Footer"` text, assign it to `s-intro` (`slide.set_master`).
  2. Switch to layer `"about"`, click `"Hide"`, inspect `frame_shapes` for `"Master Footer"`.
  3. Click `"Dim"` and `"Show"`, inspect alpha of `"Master Footer"`.
- **Expected result:** `"Master Footer"` behaves consistently with the base slide (hidden in `Hide`, dimmed in `Dim`, full alpha in `Show`, never selectable on a layer).
- **Priority:** P2

### A9.4 — Dimming & hiding ALL base object kinds (shapes with gradients, images, tables, paths, hotspot overlays)
- **What it checks:** A slide whose base contains a 3-stop gradient shape, an image, a table, a path, and a Hotspot question region (`canvas.rs:5804–5812`): switching to a layer and toggling `Show` / `Dim` / `Hide`.
- **Why it might break:** `base_slide_view_tests.rs` only checked text alpha! Gradient meshes (`paint_gradient_contours`), picture fills, and hotspot overlays each take separate rendering branches that can ignore `dimmed` or `base_hidden`.
- **Where / how:** `crates/ui-egui/src/base_slide_view_tests.rs` using `TestHarness` + `frame_shapes`.
- **Setup & steps:**
  1. Build a slide with a 3-stop gradient rectangle, a table (`"TblCell"`), an image, and a Hotspot question with one region (`"Hotspot 1"`), plus layer `"L1"` with text `"Layer Note"`.
  2. On `"L1"` with `Hide`: assert `"TblCell"` is not painted, no gradient mesh at base rect is painted, and hotspot region overlay is not painted; `"Layer Note"` IS painted at full alpha.
  3. On `"L1"` with `Dim`: assert base gradient mesh vertex alpha is reduced (`< 200`) and `"TblCell"` alpha is reduced (`< 200`).
- **Expected result:** Every base object kind and hotspot overlay respects `Show`, `Dim`, and `Hide`.
- **Priority:** P1

### A9.5 — Multiple layers with another layer pinned visible (`extra_layer_ids`)
- **What it checks:** Slide with Base + Layer A + Layer B where Layer B's eye icon (visibility) is pinned on while editing Layer A, and `Base slide` is set to `Hide`.
- **Why it might break:** If `Hide` filters out everything except the active layer instead of only base items, it would wrongly hide pinned Layer B!
- **Where / how:** `crates/ui-egui/src/base_slide_view_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Add Layer B (`"layer-b"`, text `"Pinned B"`) alongside `"about"` (`"Ten minutes"`).
  2. Select `"about"`, pin `"layer-b"` visible in `editor.ui.layers` (or click its visibility eye in the Layers panel), click `"Hide"` on `"Base slide:"`.
  3. Inspect `frame_shapes`: assert base `"Start"` is hidden, `"Ten minutes"` (Layer A) is full alpha, and `"Pinned B"` (pinned Layer B) is STILL painted.
- **Expected result:** `Hide` hides only the base slide, preserving both the active layer and explicitly pinned layers.
- **Priority:** P2

### A9.6 — Switching slides and layers preserves `base_slide_view` and hides toggle on Base
- **What it checks:** Set `Hide` on Slide 1 Layer A → switch to Slide 1 Layer B → switch to Slide 2 (Base) → switch to Slide 2 Layer C → back to Slide 1 Base.
- **Why it might break:** `base_slide_view` is course-wide (`course.editor.base_slide_view`); switching slides must neither reset it nor show the `"Base slide:"` toggle when on Base.
- **Where / how:** `crates/ui-egui/src/base_slide_view_tests.rs` using `TestHarness`.
- **Setup & steps:** Perform the slide/layer switching sequence above, asserting toggle visibility (`None` on Base, `Some` on any Layer) and `course.editor.base_slide_view == BaseSlideView::Hide` throughout.
- **Expected result:** Setting persists cleanly across all slides/layers and toggle only appears on layers.
- **Priority:** P1

### A9.7 — Click and marquee selection on a layer never picks base objects in `Show`, `Dim`, or `Hide`
- **What it checks:** Dragging a marquee box covering both a base button and a layer text object while on layer `"about"` in each of `Show`, `Dim`, and `Hide`.
- **Why it might break:** Click hit-testing was checked, but marquee selection (`finish_marquee`) might iterate `items` without excluding base items when `Show` is active!
- **Where / how:** `crates/ui-egui/src/base_slide_view_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. On layer `"about"`, for each mode (`Show`, `Dim`, `Hide`):
     - Clear selection, drag marquee across the entire slide canvas.
     - Assert `editor.selection.ids()` contains ONLY objects from layer `"about"` and ZERO base object IDs (`"btn-start"`, `"title"`, etc.).
- **Expected result:** Marquee selection on a layer never selects base objects in any `BaseSlideView` mode.
- **Priority:** P1

### A9.8 — Timeline scrubbing while editing a layer under `Dim` and `Hide`
- **What it checks:** On layer `"about"` with `Base slide: Hide`, scrub timeline to `1500ms` (activating `time_view` in `canvas.rs:5756`).
- **Why it might break:** `timeline_animated_items` runs after `items.retain(...)`, or could re-compose base items if `time_view` bypasses `BaseSlideView::Hide`.
- **Where / how:** `crates/ui-egui/src/base_slide_view_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Switch to `"about"`, click `"Hide"`, scrub timeline ruler to `1500ms`.
  2. Assert base `"Start"` is still NOT painted and `"about"` layer objects render at `1500ms`.
- **Expected result:** `Hide` and `Dim` stay respected during timeline scrubbing and playback.
- **Priority:** P2

### A9.9 — Toggle bar layout on narrow window (`1024×680`) and slide with 6 layers
- **What it checks:** Adding 6 layers with realistic names (`"Correct"`, `"Incorrect"`, `"Try Again"`, `"Hint 1"`, `"Hint 2"`, `"Summary"`) on a `1024×680` window and selecting a layer.
- **Why it might break:** The `"Base slide: Show | Dim | Hide"` pills share the horizontal strip above the slide with the layer pills (`canvas.rs:1507`); on narrow screens with many layers, the toggle could clip off the right edge or overlap layer pills!
- **Where / how:** `crates/ui-egui/src/base_slide_view_tests.rs` using `TestHarness::new_with_size(..., (1024.0, 680.0))`.
- **Setup & steps:**
  1. Add 6 layers to `s-intro`, select the 6th layer, run `layout_report(3)`.
  2. Assert `"Show"`, `"Dim"`, `"Hide"` are all visible inside `central_panel_rect()` and `report.findings.is_empty()`.
- **Expected result:** Toggle pills remain visible and unclipped even with many layers on a narrow window.
- **Priority:** P2

### A9.10 — `.ccraft` pack/unpack, `view.set` CLI command, and `validate` round-trip
- **What it checks:** Running `view.set {"base_slide_view": "hide"}` via `coursecraft-cli run`, packing to `.ccraft`, unpacking, validating, and verifying `render --mode player` still shows base objects.
- **Why it might break:** Serialization/deserialization of `editor.base_slide_view` (note `skip_serializing_if = "is_default_base_slide_view"`) must round-trip all 3 enum values (`show`, `dim`, `hide`) cleanly.
- **Where / how:** `apps/coursecraft-cli/tests/m26_cli_and_packaging.rs`.
- **Setup & steps:**
  1. Run `coursecraft-cli run` with `view.set` for `"hide"`, `"show"`, and `"dim"`; `pack` + `unpack`; verify `course.json` values (`"hide"`, `"show"`, omitted for default `"dim"`).
  2. Run `coursecraft-cli validate` (exit 0) and `render --mode player` (base objects rendered).
- **Expected result:** Full round-trip fidelity for all 3 `BaseSlideView` values with zero runtime impact.
- **Priority:** P3

---

## A10. Question editor in right panel

**Bug class:** Question editing controls missing from the right side panel for some question kinds/objects, clipping/overlapping inside the narrower right panel width, desynchronizing from the bottom Question tab, or holding stale state when switching slides.

### A10.1 — Every `QuestionKind` renders cleanly in the right panel with zero layout clipping/overlap
- **What it checks:** All 8 `QuestionKind` variants (`MultipleChoice`, `MultipleResponse`, `TrueFalse`, `FillIn`, `Matching`, `Sequence`, `Hotspot`, `DragDrop`) displayed in the right panel (`Slide` tab, scrolled top to bottom) at `1280×900` and `1024×720`.
- **Why it might break:** `show_question_editor_body` was originally designed for the wide bottom panel (~900px); rendering it inside the narrow right panel (`PROPERTIES_PANEL_WIDTH`, ~280px) is very likely to clip buttons, combos, or choice rows!
- **Where / how:** `crates/ui-egui/src/question_panel_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. For each of the 8 `QuestionKind` variants, create a course via `question.insert {"slide": "s1", "kind": ...}` (using a valid image for `Hotspot` and shapes for `DragDrop`).
  2. Select slide `"s1"` (`selection.object = None`), scroll right panel in 8 steps of `-200.0`, running `layout_report(2)` at every step.
  3. Collect all `clipped-widget` and `overlap` findings.
- **Expected result:** Every question kind renders inside the right panel with ZERO clipped widgets and ZERO overlapping text.
- **Priority:** P1

### A10.2 — Editing choices, correct answer, adding/removing choices, points, attempts, and shuffle from the right panel
- **What it checks:** Existing `question_panel_tests.rs` only edits the Prompt! This test exercises the full question authoring workflow inside the right panel: edit Choice A text, toggle Choice B as correct, click Add choice, remove a choice, change Points, change Attempts, and toggle Shuffle.
- **Why it might break:** Choice text edits, radio/checkbox toggles, and add/remove buttons inside the right panel could collide with widget IDs or fail to dispatch `question.set`.
- **Where / how:** `crates/ui-egui/src/question_panel_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Open `hello-course`, select `"s-quiz"`, `selection.object = None`.
  2. In `right_panel_rect()`, edit a choice's text, toggle another choice as correct, click `"Add choice"` (scrolling as needed), verify `q.choices.len()` increased by 1, then remove the added choice.
  3. Toggle `"Shuffle"` (if present) and change `"Points"` — assert `slide.question` updated after each edit and `undo()` reverts each step.
- **Expected result:** Full question editing works from the right panel identically to the bottom tab.
- **Priority:** P1

### A10.3 — Changing `QuestionKind` from the right panel
- **What it checks:** Opening the question type `ComboBox` in the right panel on `"s-quiz"` and switching `Multiple choice` → `True / False` → `Fill in the blank` → `Matching`, then undoing back.
- **Why it might break:** `question.set_kind` rebuilds question fields and slide objects; if the right panel caches old choice buffers, switching kind can panic or display stale controls.
- **Where / how:** `crates/ui-egui/src/question_panel_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Select `"s-quiz"` (`selection.object = None`).
  2. Use `pick_combo` in right panel to switch `"Multiple choice"` → `"True / False"` → verify `q.kind == QuestionKind::TrueFalse` and right panel updates.
  3. Switch to `"Fill in the blank"` → verify accepted-answer controls appear in right panel; `undo()` × 2 → verify `MultipleChoice` returns.
- **Expected result:** Kind switching from the right panel updates both model and UI cleanly and is undoable.
- **Priority:** P1

### A10.4 — Selecting EVERY kind of question object (`Format` tab)
- **What it checks:** Selecting each object on a question slide shows (or intentionally hides) the Question section in the right panel:
  - Submit button (`Action::SubmitQuestion` trigger) → shows Question section
  - Hotspot image (`q.hotspot_image`) → shows Question section
  - Drag item & Drop target (`q.drag_items`, `q.drop_targets`) → shows Question section
  - **Choice button / prompt text on an MC question slide** (triggers `SelectChoice` or `SetState`, NOT `SubmitQuestion`!) — checks `is_question_object` (`crates/ui-egui/src/question.rs:331–353`)!
- **Why it might break:** `is_question_object` currently checks ONLY `Hotspot` image, `DragDrop` items/targets, and `Action::SubmitQuestion` triggers — meaning clicking a Multiple Choice answer button or the question prompt on canvas does NOT show the Question section in the right panel, which is a huge surprise for authors like Aurelie!
- **Where / how:** `crates/ui-egui/src/question_panel_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. On a Hotspot slide, select the hotspot image → assert `"Question"` section appears in right panel (after scrolling).
  2. On a DragDrop slide, select a drag item AND separately select a drop target → assert `"Question"` section appears for both.
  3. On `s-quiz` (Multiple Choice), select a choice object (e.g. a choice button/row if present on the slide) or submit button → assert expected visibility (and explicitly flag if choice objects on MC slides don't expose the Question section!).
- **Expected result:** Hotspot image, drag items, drop targets, submit controls (and choice objects per product spec) surface the Question editor in the right panel.
- **Priority:** P1

### A10.5 — Question slide while editing a feedback layer (`Correct` / `Incorrect` / `Try Again`)
- **What it checks:** Selecting the `"Correct"` or `"Incorrect"` feedback layer on `"s-quiz"`, with and without a layer object selected.
- **Why it might break:** When editing a feedback layer, the inspector shows Layer properties (or layer object properties); verify the Question section doesn't clobber layer properties or crash if edited while a layer is active.
- **Where / how:** `crates/ui-egui/src/question_panel_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. On `"s-quiz"`, click the `"Correct"` layer pill (`selection.layer = Some("correct")`).
  2. Check right panel with no object selected, and with a feedback layer text object selected; run `layout_report(2)`.
  3. Return to `"Base"` pill — verify Question section is immediately visible and editable again.
- **Expected result:** Feedback layer editing and Base question editing coexist with zero crashes or layout issues.
- **Priority:** P2

### A10.6 — Question Bank slide vs Bank Draw slide in the right panel
- **What it checks:**
  1. Selecting a question slide inside a Question Bank (`bank.create` + `bank.add_question`) shows the Question section in the right panel and edits its prompt/choices.
  2. Selecting a Draw slide (`draw.insert`) shows the `"Draw"` section (bank + count) and does NOT show a broken `"Question"` section.
- **Why it might break:** Bank slides aren't in `course.scenes`; `course.slide(&selection.slide)` must find bank slides and `sync_question_slide` must skip draw slides (`slide.draw.is_some()`).
- **Where / how:** `crates/ui-egui/src/question_panel_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Run `bank.create`, `bank.add_question`, `draw.insert`.
  2. Select the bank's question slide → assert `"Question"` section appears in right panel, edit prompt, assert updated.
  3. Select the draw slide → assert `"Draw"` section appears and `"Question"` section does NOT appear.
- **Expected result:** Bank question slides are fully editable from the right panel; draw slides cleanly show Draw settings only.
- **Priority:** P2

### A10.7 — Both Bottom Question tab & Right Panel open simultaneously (choice/checkbox/combo sync & no ID collision)
- **What it checks:** With `BottomTab::Question` open AND right panel showing Question section simultaneously, edit choices, toggle correct answer, and change `QuestionKind` in one panel and verify the other panel updates immediately with zero focus/ID collision or double-command execution.
- **Why it might break:** Any widget inside `show_question_editor_body` that forgets to salt its ID with `id_salt` (`"right_question_panel"` vs `"bottom_question_panel"`) will collide when both panels are open at the same time — causing typing or combo clicks in one panel to jump or fail!
- **Where / how:** `crates/ui-egui/src/question_panel_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Open `s-quiz` with `BottomTab::Question` active and `selection.object = None`.
  2. Record `rev = session.revision`. Edit prompt in right panel → assert `session.revision == rev + 1` (single command, not double!).
  3. Open the `QuestionKind` combo in the right panel and pick `"True / False"` → assert bottom panel immediately displays True/False and `session.revision` incremented by 1.
  4. Edit in bottom panel → assert right panel immediately reflects it.
- **Expected result:** Zero egui ID collisions, single-command revisions, and instant two-way sync.
- **Priority:** P1

### A10.8 — Deleting a question (`question.delete`) and inserting a new question (`question.insert`) while viewing right panel
- **What it checks:** Starting on non-question slide `s-intro` → run `Insert ▸ Question` → right panel immediately displays `"Question"` section; then `Edit ▸ Delete Question` → right panel hides `"Question"` section; then `Undo` → right panel restores `"Question"` section.
- **Why it might break:** Stale `editor.ui.question.cached_slide` when `slide.question` toggles between `None` and `Some` could leave ghost controls or fail to appear until switching slides.
- **Where / how:** `crates/ui-egui/src/question_panel_tests.rs` using `TestHarness`.
- **Setup & steps:** Execute insert → assert visible → delete → assert hidden → undo → assert visible on `s-intro`.
- **Expected result:** Right panel reacts immediately to question creation, deletion, and undo.
- **Priority:** P2

### A10.9 — Multi-select containing a question object
- **What it checks:** Selecting `btn-submit` together with another object on `s-quiz` (`selection.set_many(["btn-submit", "q-title"])`).
- **Why it might break:** `properties.rs:2026–2029` returns early into `show_multi_properties` before checking `selected_object_is_question_related`; verify multi-select stays clean with no crash, and collapsing back to single-select `btn-submit` restores the Question section.
- **Where / how:** `crates/ui-egui/src/question_panel_tests.rs` using `TestHarness`.
- **Setup & steps:** Multi-select `btn-submit` + another object → `layout_report(2)` → single-select `btn-submit` → verify `"Question"` section reappears.
- **Expected result:** Clean transition between multi-select and single question-object selection.
- **Priority:** P3

### A10.10 — Switching directly between two different question slides (stale buffer check)
- **What it checks:** Course with Question Slide 1 (`"Question One?"`, MultipleChoice) and Question Slide 2 (`"Question Two?"`, FillIn): edit prompt on Slide 1, switch to Slide 2, verify right panel immediately shows `"Question Two?"` and FillIn controls without bleeding Slide 1's buffer.
- **Why it might break:** `editor.ui.question` stores editable text buffers (`prompt_buf`, etc.) synced by `(slide_id, revision)`. If switching slides while a field is focused triggers blur-commit onto Slide 2, Slide 1's prompt could overwrite Slide 2!
- **Where / how:** `crates/ui-egui/src/question_panel_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. On Slide 1, click prompt field in right panel, type `"Draft 1"` (do NOT click away).
  2. Click Slide 2 in the left slides panel, run 2 idle frames.
  3. Assert Slide 2's prompt in `session.course` is STILL `"Question Two?"` (not `"Draft 1"`) and right panel displays `"Question Two?"`.
- **Expected result:** Switching slides never leaks draft edits from Question Slide 1 onto Question Slide 2.
- **Priority:** P2

---

## A11. Format painter (Paint)

**Bug class:** Clicking `Paint` does nothing, fails to apply styles across object types/slides/layers, loses state on jittered clicks or inline edits, doesn't show status hint/cursor, or mishandles Esc/sticky/disabled states.

### A11.1 — Cross-slide format painting (Slide 1 → Slide 2)
- **What it checks:** Select styled shape `"a"` on Slide 1 → click toolbar `"Paint"` → click Slide 2 in slides panel → click target shape `"d"` on Slide 2.
- **Why it might break:** Switching slides changes `selection.slide` and clears `selection.object`; if slide switch disarms `format_painter`, cross-slide painting (a primary authoring workflow!) breaks completely.
- **Where / how:** `crates/ui-egui/src/format_painter.rs` (tests module) using `TestHarness`.
- **Setup & steps:**
  1. Two-slide course: Slide 1 has `"a"` (`fill: "#112233"`, `outline: 3px #FFAA00`, `opacity: 0.7`, shadow); Slide 2 has `"d"` (`fill: "#FFFFFF"`, no outline).
  2. Click `"a"`, click `"Paint"`, assert `format_painter.is_some()`.
  3. Click Slide 2 in left slides panel — assert `format_painter.is_some()` (still armed with `source_slide == "slide1"`!).
  4. Click `"d"` on Slide 2 canvas — assert `"d"` now has `fill: "#112233"`, `outline: 3px #FFAA00`, `opacity: 0.7`, shadow, and `format_painter.is_none()`.
  5. `session.undo()` — assert `"d"` reverts to `"#FFFFFF"` in one step.
- **Expected result:** Format painter works seamlessly across slides and undoes in one step.
- **Priority:** P1

### A11.2 — Cross-object-kind matrix (Shape ↔ Button ↔ Text ↔ Image ↔ Closed Path)
- **What it checks:**
  1. Shape (3-stop gradient + outline + shadow + glow + opacity + shape text) → Shape (keeps target `ShapeKind` and target text content!)
  2. Shape (3-stop gradient + shadow) → Button (copies `fill`, `fill_style`, `shadow`, `opacity`; keeps button `label` and triggers!)
  3. Button (gradient + glow) → Shape (copies `fill`, `fill_style`, `glow`, `opacity`; keeps shape kind!)
  4. Text box (bold, 30pt, `#AA2244`, letter spacing 2.0, align Center) → Shape with text (copies text style onto `ShapeText.style` without wiping shape's fill!) and Shape with text → Text box
  5. Shape (shadow + opacity 0.6) → Image (copies shadow & opacity; preserves image `src` and `crop`!)
  6. Shape (gradient + outline) → Closed `ObjectKind::Path` (checks whether `apply_look` copies fill/fill_style/outline onto closed `Path`!)
- **Why it might break:** `look_of` / `apply_look` (`crates/engine/src/commands.rs:5624–5720`) explicitly enumerates `Text`, `Shape`, `Button` — easy to miss `Path` or accidentally clobber target content/triggers/crop.
- **Where / how:** `crates/engine/src/tests.rs` + `crates/ui-egui/src/format_painter.rs` (`TestHarness`).
- **Setup & steps:** Exercise all 6 pairs through toolbar `Paint` click + target canvas click, inspecting full `Object` before/after.
- **Expected result:** Compatible visual attributes copy accurately without mutating target identity, content, geometry, crop, or triggers.
- **Priority:** P1

### A11.3 — Painting onto a Text box with per-run overrides and hyperlinks
- **What it checks:** Target text box `"t2"` has `"Visit Docs"` with run `"Docs"` styled bold/red AND carrying a `Link::Url { url: "https://example.com" }`. Paint from plain 28pt `#0044AA` text box `"t1"`.
- **Why it might break:** `clear_run_look` (`commands.rs:5659–5671`) clears run visual overrides (`bold`, `color`, `size`, `letter_spacing`, `strikethrough`, `baseline`, `font`) and MUST preserve `run.link`!
- **Where / how:** `crates/ui-egui/src/format_painter.rs` using `TestHarness`.
- **Setup & steps:**
  1. Arm Paint on `"t1"`, click `"t2"`.
  2. Inspect `"t2"`'s `style` and `runs`: assert `style.size == 28.0`, `style.color == "#0044AA"`, all run visual overrides are `None`, AND the hyperlink on `"Docs"` is STILL `Some(Link::Url { url: "https://example.com" })`.
- **Expected result:** Visual run overrides clear so target looks uniform like source, while text content and hyperlinks are preserved.
- **Priority:** P2

### A11.4 — Keyboard shortcuts `Cmd/Ctrl+Alt+C` (arm) and `Cmd/Ctrl+Alt+V` (apply to multi-select)
- **What it checks:** Select `"a"` → press `Cmd/Ctrl+Alt+C` → select `"b"` and `"c"` → press `Cmd/Ctrl+Alt+V` → verify both `"b"` and `"c"` update in ONE undo step and painter disarms.
- **Why it might break:** Shortcut consumption in `handle_format_painter_shortcuts` could conflict with other shortcuts or fail to disarm after applying via keyboard.
- **Where / how:** `crates/ui-egui/src/format_painter.rs` using `TestHarness::press`.
- **Setup & steps:**
  1. Select `"a"`, `press(Key::C, Modifiers::COMMAND | Modifiers::ALT)` → assert `format_painter.is_some()`.
  2. `selection.set_many(["b", "c"])`, record `rev = session.revision`.
  3. `press(Key::V, Modifiers::COMMAND | Modifiers::ALT)` → assert both `"b"` and `"c"` match `"a"`'s fill, `session.revision == rev + 1`, and `format_painter.is_none()`.
  4. `session.undo()` → assert both `"b"` and `"c"` revert simultaneously.
- **Expected result:** Keyboard arm & apply work for multi-select as a single atomic undo step.
- **Priority:** P1

### A11.5 — Clicking source itself, clicking empty canvas, and right-clicking while armed
- **What it checks:**
  1. Non-sticky Paint armed on `"a"` → click `"a"` itself: no-op (`revision` unchanged, `skip_undo = true`) and disarms.
  2. Non-sticky Paint armed on `"a"` → click empty canvas space: disarms cleanly (no error, no unintended selection side effect).
  3. Sticky Paint armed on `"a"` → click empty canvas: stays armed (or cancels predictably) without crashing.
  4. Armed on `"a"` → right-click `"b"`: does not crash or corrupt paint mode.
- **Why it might break:** Clicking self creates a useless empty undo step if `skip_undo` isn't respected; clicking empty canvas could leave painter stuck or fail to disarm.
- **Where / how:** `crates/ui-egui/src/format_painter.rs` using `TestHarness`.
- **Setup & steps:** Run each of the 4 cases via `click_at` / `right_click_at` and assert `revision`, `format_painter`, and object fills.
- **Expected result:** Self-click is a zero-undo no-op; empty click and right-click behave cleanly without crashes.
- **Priority:** P2

### A11.6 — Arming Paint while inline text editing is open
- **What it checks:** Double-click text box `"t1"`, type `" Updated"`, then click toolbar `"Paint"` button, then click `"t2"`.
- **Why it might break:** `arm_format_painter` calls `commit_inline_edit(self)` (`format_painter.rs:43`); if toolbar click clears selection or loses inline edit text before committing, `"t1"` loses `" Updated"` or Paint fails to arm.
- **Where / how:** `crates/ui-egui/src/format_painter.rs` using `TestHarness`.
- **Setup & steps:**
  1. Double-click `"t1"` (`size: 32, bold: true`), type `" Updated"`. Assert `inline_edit.is_some()`.
  2. Click toolbar `"Paint"` button. Assert `inline_edit.is_none()`, `"t1"` text contains `" Updated"`, and `format_painter.is_some()`.
  3. Click `"t2"` — assert `"t2"` gets `size: 32, bold: true` while keeping its own text.
- **Expected result:** Inline edit commits cleanly, Paint arms, and target receives source formatting.
- **Priority:** P2

### A11.7 — Painting across Base slide and Slide Layer (Base → Layer and Layer → Base)
- **What it checks:** Arm Paint on a styled shape on Base slide → switch to Layer `"about"` → click shape `"about-panel"` on the layer; and reverse (Layer → Base).
- **Why it might break:** `object.apply_style` looks up `source` and `objects` via `slide.object(...)` / `find_object_mut(slide, id)`; ensure layer objects are found as both source and target and canvas click targets layer objects (not dimmed base objects).
- **Where / how:** `crates/ui-egui/src/format_painter.rs` using `TestHarness`.
- **Setup & steps:**
  1. On `hello-course` `s-intro`, give `btn-start` a distinct fill `"#123456"`, click `"Paint"`.
  2. Click `"About"` layer pill, click `"about-panel"` on canvas → assert `"about-panel"` fill becomes `"#123456"`.
  3. Reverse: style `"about-panel"` to `"#654321"`, click `"Paint"`, click `"Base"` pill, click `"btn-start"` → assert `"btn-start"` fill becomes `"#654321"`.
- **Expected result:** Format painting works seamlessly in both directions between Base and Layers.
- **Priority:** P1

### A11.8 — Source object deleted while Sticky Paint is armed
- **What it checks:** Arm sticky Paint on `"a"` → delete `"a"` via engine command (`object.delete`) or undo its creation → click `"b"` on canvas.
- **Why it might break:** `format_painter.source` now points to a deleted object ID; `object.apply_style` returns `EngineError::NotFound`. Must not panic, must not corrupt `"b"`, and should surface a clean status message or disarm gracefully.
- **Where / how:** `crates/ui-egui/src/format_painter.rs` using `TestHarness`.
- **Setup & steps:**
  1. Select `"a"`, arm sticky Paint.
  2. Run `object.delete` on `"a"`, `idle_frame()`.
  3. Click `"b"` on canvas. Assert no panic and `"b"` retains its original fill `"#FF0000"`.
- **Expected result:** Graceful handling with zero crash or corruption.
- **Priority:** P3

### A11.9 — Disabled state, status hints, and single-click vs double-click toggle on toolbar `Paint` button
- **What it checks:**
  1. 0 objects selected: clicking `"Paint"` does nothing (`format_painter.is_none()`).
  2. 2 objects selected: clicking `"Paint"` does nothing.
  3. 1 object selected: clicking `"Paint"` once arms non-sticky mode and shows `"Click an object to copy the format onto it (Esc to cancel)"`; clicking `"Paint"` again (slowly, not a double-click) disarms it!
  4. Double-clicking `"Paint"` arms sticky mode (does NOT immediately disarm on the 2nd click's release thanks to `just_double_clicked`), shows status hint, and clicking `"Paint"` a third time (slowly) disarms it.
- **Why it might break:** `just_double_clicked` flag (`format_painter.rs:19`) is subtle; slow second click vs fast double-click timing can easily get inverted so clicking Paint toggles off immediately or never toggles off.
- **Where / how:** `crates/ui-egui/src/format_painter.rs` using `TestHarness` with explicit timestamps (`time = 1.0`, `1.1`, `3.0`).
- **Setup & steps:** Drive single-click arm → slow click disarm (at `t=3.0`); then fast double-click (`t=5.0`, `5.1`) → sticky armed → slow click (`t=8.0`) → disarmed.
- **Expected result:** Slow toggle disarms; fast double-click enters sticky mode cleanly; status bar updates accurately.
- **Priority:** P1

### A11.10 — Paint onto rotated, overlapping, or locked target objects without moving them
- **What it checks:** Clicking a rotated target (`rotation: 45.0`) with 5px mouse jitter while Paint is armed applies style to the rotated target and does NOT drag/move it; clicking overlapping objects paints the topmost one; clicking a `locked: true` object behaves cleanly (does not move or crash).
- **Why it might break:** `canvas.rs:5655–5665` handles Paint clicks on `response.clicked() || response.drag_stopped()`; verify `start_drag_if_pressed` is completely bypassed when `format_painter.is_some()` so jitter never shifts `frame.x/y`.
- **Where / how:** `crates/ui-egui/src/format_painter.rs` using `TestHarness`.
- **Setup & steps:**
  1. Place `"b"` rotated `45°` overlapping `"c"` (`"b"` on top). Record `"b"`'s exact `frame`.
  2. Arm Paint on `"a"`, press on `"b"`, move `(+5, +3)`, release.
  3. Assert `"b"` received `"a"`'s fill, `"c"` did NOT, and `"b".frame` is completely unchanged.
- **Expected result:** Topmost rotated object is styled with zero unintended movement.
- **Priority:** P2

---

## A12. Preview window closing

**Bug class:** Clicking outside the Preview modal fails to close it, leaks clicks to editor controls underneath, accidentally closes on drags/drawers/text inputs, or fails on error screens / small windows / reopened previews.

### A12.1 — Click outside on all four margins across multiple viewport sizes
- **What it checks:** Clicking top backdrop margin, bottom backdrop margin, left backdrop margin, and right backdrop margin at `1024×680`, `1280×800`, and `1920×1080`.
- **Why it might break:** If the preview modal fills or nearly fills one axis on a compact `1024×680` window, one of the four margins might vanish or sit behind the top menu bar.
- **Where / how:** `crates/ui-egui/src/preview_close_tests.rs` using `TestHarness::new_with_size`.
- **Setup & steps:**
  1. For each size `(1024.0, 680.0)`, `(1280.0, 800.0)`, `(1920.0, 1080.0)`:
     - Test 4 points outside `window_rect`: `(rect.center().x, 4.0)`, `(rect.center().x, h - 4.0)`, `(4.0, rect.center().y)`, `(w - 4.0, rect.center().y)`.
     - Open preview, assert point is `!rect.contains(pt)`, `click_at(pt)`, `idle_frame()`, assert `editor.preview.is_none()`.
- **Expected result:** Every margin closes the preview reliably at every window size.
- **Priority:** P1

### A12.2 — Backdrop click does NOT click through to editor panels/objects underneath
- **What it checks:** Clicking outside the preview directly over (a) Slide 2 in the left slides panel, (b) the `"Insert"` menu in the top menu bar, and (c) an inspector tab in the right panel closes Preview ONLY without switching slides, opening menus, or changing tabs.
- **Why it might break:** If the backdrop doesn't consume pointer events before underlying editor panels run, dismissing preview by clicking outside could accidentally delete/select/open whatever sat underneath!
- **Where / how:** `crates/ui-egui/src/preview_close_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Two-slide course on Slide 1 (`s-1`). Find exact screen positions of `"Slide 2"` (left panel) and `"Insert"` (menu bar) before opening Preview.
  2. Open Preview, verify those positions are outside `window_rect` (use `(1440.0, 900.0)` so outer panels peek out behind backdrop).
  3. Click `"Slide 2"`'s screen position → assert `editor.preview.is_none()` AND `editor.selection.slide == "s-1"` (did NOT switch to Slide 2!).
  4. Reopen Preview, click `"Insert"`'s screen position → assert `editor.preview.is_none()` AND no Insert menu popup opened.
- **Expected result:** Backdrop consumes the dismiss click completely; zero click-through to the editor.
- **Priority:** P1

### A12.3 — Outline/Menu drawer open inside Preview: `Esc` closes drawer first, 2nd `Esc` closes Preview
- **What it checks:** Opening the learner Outline/Menu drawer inside Preview (`player_frame`), then pressing `Esc` once (drawer closes, Preview stays open via `drawer_was_open` at `preview.rs:987`), then pressing `Esc` a second time (Preview closes).
- **Why it might break:** If `drawer_was_open` is sampled after the drawer already closed in the same frame, a single `Esc` closes both drawer and Preview abruptly!
- **Where / how:** `crates/ui-egui/src/preview_close_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Course with `player.show_menu: true` and 2 slides. Open Preview.
  2. Click the Menu/Outline toggle button inside Preview; assert `editor.preview.as_ref().unwrap().frame.drawer_open()` (or drawer text visible).
  3. Press `Key::Escape` → assert `editor.preview.is_some()` AND drawer is now closed!
  4. Press `Key::Escape` again → assert `editor.preview.is_none()`.
- **Expected result:** First `Esc` dismisses the drawer; second `Esc` closes Preview.
- **Priority:** P1

### A12.4 — Effect Log (`F2`) and Live Variables (`F3`) panels inside Preview
- **What it checks:** Toggling Effect Log and Variables panels open inside Preview (`preview.rs:780–797`), clicking inside both panels (must NOT close Preview), then clicking outside the expanded modal (MUST close Preview).
- **Why it might break:** Opening side panels changes layout width; if `window_rect` or modal rect lags, clicking inside the new side panel could register as a backdrop click and close Preview!
- **Where / how:** `crates/ui-egui/src/preview_close_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Open Preview, click Effect Log toggle (`Icon::Notepad`) and Variables toggle (`Icon::BracketsCurly`), `idle_frame()`.
  2. Click `"Effect Log"` heading and inside the variables pane → assert `editor.preview.is_some()`.
  3. Click `(4.0, 4.0)` outside `window_rect` → assert `editor.preview.is_none()`.
- **Expected result:** Side panels stay open when clicked; outside backdrop still closes Preview.
- **Priority:** P2

### A12.5 — Dragging a slider/seekbar inside Preview and releasing pointer OUTSIDE the modal
- **What it checks:** Starting a pointer drag on a `Slider` object (or the player seekbar) inside Preview, moving the pointer outside `window_rect`, and releasing the mouse button on the backdrop.
- **Why it might break:** `modal.backdrop_response.clicked()` in egui could fire on pointer release over the backdrop (or a subtle drag-release event), unexpectedly closing Preview mid-drag!
- **Where / how:** `crates/ui-egui/src/preview_close_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Course with a `Slider` input object (`variable: "v"`, `0..100`) on `s-1`. Open Preview.
  2. Find the slider inside `window_rect`, `drag_by` from slider centre to `pos2(5.0, 5.0)` (outside `window_rect`!).
  3. `idle_frame()`, assert `editor.preview.is_some()` (Preview did NOT close on drag release outside!).
  4. Now perform a clean click at `pos2(5.0, 5.0)` → assert `editor.preview.is_none()`.
- **Expected result:** Dragging from inside Preview and releasing outside does NOT close Preview; a fresh click outside DOES close it.
- **Priority:** P1

### A12.6 — Validation-error Preview screen closes on backdrop click and on Escape
- **What it checks:** Opening Preview on a course with a validation error (e.g. a trigger jumping to a nonexistent slide ID), so Preview renders its error screen (`self.errors` non-empty, `self.state.is_none()`), then closing via backdrop click and via `Esc`.
- **Why it might break:** Preview may have a separate code branch when `self.state.is_none()` (validation errors); if backdrop/Esc handling only runs in the playable branch, the error preview can only be closed via `X`!
- **Where / how:** `crates/ui-egui/src/preview_close_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Build a course with a broken `jump_to_slide` target `"does-not-exist"`.
  2. `editor.open_preview()`, run 3 frames, assert `!editor.preview.as_ref().unwrap().errors.is_empty()`.
  3. Click `(5.0, 5.0)` outside `window_rect` → assert `editor.preview.is_none()`.
  4. Reopen Preview, press `Key::Escape` → assert `editor.preview.is_none()`.
- **Expected result:** Error-mode Preview closes cleanly via both backdrop click and Escape.
- **Priority:** P2

### A12.7 — Lightbox open inside Preview + Esc and backdrop click
- **What it checks:** Clicking a button inside Preview that opens a `lightbox_slide`, then testing `Esc` and outside backdrop click.
- **Why it might break:** Lightbox paints an inner overlay inside `PlayerFrame`; pressing `Esc` or clicking outside must close cleanly without leaving the modal stuck or unclosable.
- **Where / how:** `crates/ui-egui/src/preview_close_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Two-slide course where Slide 1 button triggers `lightbox_slide` for Slide 2.
  2. Open Preview, click button so lightbox opens (`player.lightbox().is_some()`).
  3. Click `(5.0, 5.0)` outside `window_rect` → assert `editor.preview.is_none()`.
  4. Reopen, open lightbox again, press `Esc` (and a 2nd `Esc` if first closes lightbox) → assert `editor.preview.is_none()`.
- **Expected result:** Lightbox inside Preview never blocks closing Preview via backdrop click or Esc.
- **Priority:** P2

### A12.8 — Media playback stops on backdrop click and Esc close
- **What it checks:** Opening Preview on a slide with auto-playing audio/video, then closing via backdrop click (and separately via `Esc`), verifying `Preview` drops and calls `media.stop_all()` (`preview.rs:992–993, 171–175`).
- **Why it might break:** If backdrop close short-circuits before `if !*open { self.media.stop_all(); }`, audio keeps playing after the Preview window disappears!
- **Where / how:** `crates/ui-egui/src/preview_close_tests.rs` (or `crates/ui-egui/src/preview.rs` unit test with a recording `MediaBackend`).
- **Setup & steps:**
  1. Open Preview with a stub `MediaBackend` recording `stop_all_calls`.
  2. Close Preview via backdrop click → assert `stop_all_calls >= 1` and `editor.preview.is_none()`.
  3. Reopen and close via `Esc` → assert `stop_all_calls >= 1`.
- **Expected result:** Closing Preview by any method immediately stops all Preview media.
- **Priority:** P1

### A12.9 — Editor shortcuts (`Delete`, `Ctrl+Z`, `Ctrl+M`) do not mutate course while Preview is open
- **What it checks:** Selecting an object in the editor, opening Preview, pressing `Delete`, `Ctrl+M`, and `Ctrl+Z`, then closing Preview with `Esc`.
- **Why it might break:** If Preview modal doesn't fence editor keyboard shortcuts, pressing keys while previewing could silently delete slides/objects in the background!
- **Where / how:** `crates/ui-egui/src/preview_close_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Select `"txt-1"`, record `rev = session.revision` and `slides_count`, `objects_count`.
  2. Open Preview, press `Key::Delete`, `Key::M` + `COMMAND`, `Key::Z` + `COMMAND`.
  3. Press `Key::Escape` to close Preview.
  4. Assert `session.revision == rev`, slide count and object count unchanged, and `"txt-1"` still exists.
- **Expected result:** Preview completely isolates the underlying editor document from keyboard shortcuts.
- **Priority:** P2

### A12.10 — Close via `X`, reopen, click Restart, close via backdrop, reopen, close via Esc
- **What it checks:** Repeated open/close lifecycle across all 3 close paths (`X` button, backdrop click, `Esc`) plus `Restart`, checking `window_rect` size stability and `layout_report` cleanliness.
- **Why it might break:** Stale `had_text_focus` or `window_rect` across reopens could break Esc or backdrop detection on the 2nd or 3rd open.
- **Where / how:** `crates/ui-egui/src/preview_close_tests.rs` using `TestHarness`.
- **Setup & steps:**
  1. Open Preview → click `Icon::X` close button → assert closed.
  2. Open Preview → click Restart → click outside backdrop → assert closed.
  3. Open Preview → check `layout_report(2).findings.is_empty()` → press `Esc` → assert closed.
- **Expected result:** Every reopen starts cleanly at full size and closes cleanly via X, backdrop, and Esc.
- **Priority:** P3

---

## A13. PowerPoint import & fidelity suite

**Bug class:** Missing backgrounds, broken theme/scheme colours, dropped videos/pictures, broken custom geometry/subpaths, lost layout/master placeholder styling, broken tables/bullets/text layout, stray lines in scaled/rotated/flipped groups, or whole-deck aborts on one bad shape.

### A13.1 — Automated Visual Fidelity Regression Test (`fidelity.pptx` vs PowerPoint PNGs)
- **What it checks:** Imports `target/fidelity/fidelity.pptx`, renders all 18 imported slides via `coursecraft_render::render_slide_rgba`, compares each slide pixel-by-pixel against PowerPoint's own reference capture in `target/fidelity/ppt/slide01.png..slide18.png`, and enforces **per-slide minimum similarity thresholds** so import quality can never silently get worse.
- **Why it might break:** Any regression in `coursecraft-pptx` (theme resolution, backgrounds, placeholders, groups, custom geometry, fills, text layout, bullets, tables, video posters) or `coursecraft-render` immediately drops the similarity score on the affected slide(s).
- **Where / how:** New integration test `apps/coursecraft-cli/tests/m26_pptx_fidelity.rs` (add `coursecraft-media.workspace = true` to `[dev-dependencies]` of `apps/coursecraft-cli/Cargo.toml` to decode reference PNGs via `coursecraft_media::decode_image`).
- **Fixture path resolution:** Look for the fidelity directory in order:
  1. `std::env::var("COURSECRAFT_FIDELITY_DIR")`
  2. `<workspace_root>/target/fidelity`
  3. `<workspace_root>/../../target/fidelity` (supports git worktrees located under `<main_repo>/target/<wt>/`, as in `target/wt-m26`!)
  If absent locally and `COURSECRAFT_REQUIRE_FIDELITY` is unset, skip with an explicit `eprintln!` notice; if `COURSECRAFT_REQUIRE_FIDELITY=1`, fail.
- **Setup & steps:**
  1. Read `<fidelity_dir>/fidelity.pptx` and call `coursecraft_pptx::import_pptx(&bytes, "Fidelity")`.
  2. Assert `imported.course.scenes[0].slides.len() == 18` (Slide 19 is hidden and skipped).
  3. Build an in-memory `MediaSource` backed by `imported.media` (`HashMap<String, Arc<[u8]>>`).
  4. For each `slide_num` in `1..=18`:
     - Render `slide = &imported.course.scenes[0].slides[slide_num - 1]` with `render_slide_rgba(&imported.course, &slide.id, &RenderOptions { width: 640, mode: RenderMode::Authoring, ..Default::default() }, &media_source)`.
     - Decode `<fidelity_dir>/ppt/slide{slide_num:02}.png` using `coursecraft_media::decode_image` and downsample (area/box average) to `(img.width, img.height)` (`640×360`).
     - Compute `mad = mean(|r_cc - r_ppt| + |g_cc - g_ppt| + |b_cc - b_ppt|) / 3.0` and `sim = 1.0 - mad / 255.0` (matching the metric in `target/fidelity/report.md`!).
     - Assert `sim >= THRESHOLDS[slide_num - 1]`, using per-slide floors that account for PowerPoint's screen colour-profile offset (~0.02–0.04) while strictly guarding M26 fixes:
       - `S01` (Basic shapes + solid bg): `>= 0.91`
       - `S02` (More auto shapes): `>= 0.90`
       - `S03` (Rotated & flipped): `>= 0.92`
       - `S04` (Custom geometry): `>= 0.92`
       - `S05` (Lines & connectors): `>= 0.93`
       - `S06` (Fills: gradients, patterns, tiled pic): `>= 0.88`
       - `S07` (Outlines & shadows): `>= 0.93`
       - `S08` (Mixed text formatting): `>= 0.93`
       - `S09` (Text layout: justify, wrap, vert270, insets): `>= 0.89`
       - `S10` (Bullets & numbering): `>= 0.90`
       - `S11` (Theme scheme colours): `>= 0.90` *(pre-fix was 0.882 with grey #D9D9D9)*
       - `S12` (Pictures & crop & ellipse/outline): `>= 0.90`
       - `S13` (Group transforms): `>= 0.91`
       - `S14` (Tables): `>= 0.90`
       - `S15` (Video + dark bg + poster): `>= 0.80` *(pre-fix was 0.265!)*
       - `S16` (Gradient slide background): `>= 0.85` *(pre-fix was 0.510!)*
       - `S17` (Picture slide background): `>= 0.85` *(pre-fix was 0.596!)*
       - `S18` (Layout placeholders): `>= 0.90`
     - On any failure, write the failing slide's rendered PNG to `target/fidelity-test-out/slide{slide_num:02}.png` and print the full 18-slide score table `(slide, actual_sim, min_sim, mad)` so regressions are instantly visible.
- **Expected result:** All 18 slides meet or exceed their similarity thresholds; any regression fails loudly with exact slide numbers and PNG artifacts.
- **Priority:** P1

### A13.2 — Structural Assertions on `fidelity.pptx` (all 18 slides + hidden Slide 19)
- **What it checks:** Verifies exact imported `Course` AST structure on `fidelity.pptx` (so visual similarity can't mask structural regressions like turning a video into a static image or flattening a table):
  - **Deck level**: 18 slides; hidden Slide 19 reported as skipped; `imported.report.items` contains ZERO `SkipKind::SchemeColor`.
  - **S01**: `slide.background.is_some()` (cream solid bg); shapes include `Rectangle`, `RoundedRectangle`, `Ellipse`, `Triangle`, `Diamond`, `Pentagon`, `Hexagon`, `Star5`, `Star8`, `Heart`; at least 4 shapes have non-empty `adjust` vectors.
  - **S02**: includes arrows (`ArrowRight`/`Chevron`/`BlockArc`), callouts, `Cloud`, `Lightning`, `Moon`, `Cross`, `Donut`.
  - **S03**: shapes with `rotation` matching 30/60/90/180/270/45 and shapes with `flip_h == true` / `flip_v == true`.
  - **S04**: `ObjectKind::Path` objects include at least one with `!path.subpaths.is_empty()` (ring with hole), one with `path.fill_style.is_some()` (gradient freeform), and one with `path.text.is_some()` (text inside freeform).
  - **S05 & S07**: paths with `DashStyle::DashDot`, arrowheads (`start_end != LineEnd::None` or `end_end != LineEnd::None`), elbow/curved connectors, and objects with `shadow.is_some()`.
  - **S06**: shapes with `FillStyle::Linear` having `stops.len() == 2`, `3`, and `5`; `FillStyle::Radial`; `FillStyle::Picture { mode: Stretch, .. }` and `FillStyle::Picture { mode: Tile, tile_scale, .. }` with `tile_scale[0] < 0.6`; pattern-filled shapes do NOT have fallback `#D9D9D9`.
  - **S08**: text runs include `bold: Some(true)`, `italic: Some(true)`, `underline: Some(true)`, `strikethrough: Some(true)`, `baseline: Some(Baseline::Superscript)`, `baseline: Some(Baseline::Subscript)`, and >= 3 distinct `font` families.
  - **S09**: text objects include `Align::Justify`, `wrap == false`, `TextDirection::Vertical270`, and `insets.is_some_and(|i| i.left > 5.0)`.
  - **S10**: paragraphs include `level >= 2`, custom `bullet` chars with `bullet_color.is_some()` and `bullet_size.is_some()`, multiple `NumberScheme` variants, and `start_at == Some(5)`.
  - **S11**: shapes have exact resolved theme fills `#E91E63`, `#00BCD4`, `#FFC107`, `#673AB7`, `#4CAF50`, `#FF5722`, plus tinted fills distinct from both base and `#D9D9D9`.
  - **S12**: `ObjectKind::Image` objects include cropped images (`crop.is_some()`), an image with `outline.is_some()`, and an image with `shape == Some(ShapeKind::Ellipse)`.
  - **S13**: grouped/nested/scaled/flipped children all have finite frames inside the slide bounds (`-50..=1330`, `-50..=770`, no stray lines shooting off-canvas!).
  - **S14**: `ObjectKind::Table` with `header_row == true`, 5 rows × 4 cols, non-empty header fills, at least one merged cell (`col_span > 1 || row_span > 1`), and borders present.
  - **S15**: `ObjectKind::Video` with `src` ending `.mp4` and `poster.is_some()`, plus dark `slide.background.is_some()`, and both video + poster exist in `imported.media`.
  - **S16**: full-slide background shape with `FillStyle::Linear` having 3 stops at ~60°.
  - **S17**: full-slide background shape with `FillStyle::Picture { mode: Stretch, .. }`.
  - **S18**: inherited title has `font == Some("Georgia")`, `bold == Some(true)`, red color, and inherited body has blue level-1 custom bullet and green italic level-2 bullet.
- **Why it might break:** Pixel similarity alone might still pass 0.90 if, say, a video silently degrades to a poster `Image` or a 5-stop gradient drops a middle stop.
- **Where / how:** `apps/coursecraft-cli/tests/m26_pptx_fidelity.rs`.
- **Expected result:** Every structural invariant passes on `fidelity.pptx`.
- **Priority:** P1

### A13.3 — Imported `fidelity.pptx` passes `validate`, `Player::new`, `simulate`, and `publish --standard web`
- **What it checks:** The imported `Course` from `fidelity.pptx` is immediately valid, playable, and publishable without manual repair.
- **Why it might break:** Importer could emit duplicate IDs, out-of-range font sizes/insets/frame sizes, invalid `col_span`, or missing media references that fail `course.validate()` or `Player::new`.
- **Where / how:** `apps/coursecraft-cli/tests/m26_pptx_fidelity.rs`.
- **Setup & steps:**
  1. Import `fidelity.pptx`.
  2. Assert `imported.course.validate()` has zero `Severity::Error` issues.
  3. Construct `Player::new(&imported.course)` → `Start` → step `Next` through all 18 slides with `Tick { ms: 500 }` on each — assert zero `Effect::Warning`.
  4. Pack to `target/m26-fidelity-pub.ccraft` via `coursecraft-format`, reopen, and run `coursecraft_publish::publish_course` (web standard) to `target/m26-fidelity-web.zip` — assert succeeds.
- **Expected result:** Imported course validates cleanly, plays all 18 slides without warnings, and publishes cleanly.
- **Priority:** P1

### A13.4 — Canva-style PPTX export constructs (nested scaled groups, multi-path `custGeom`, shape `blipFill`, `bgRef`)
- **What it checks:** Synthetic PPTX mimicking real Canva exports (Aurelie's exact deck source):
  - Nested `p:grpSp` inside `p:grpSp` with non-zero `chOff`, scaled `chExt != ext`, `flipH="1"`, and `rot="1800000"` containing both `p:sp` and `p:cxnSp` lines
  - `p:sp` with `a:blipFill` (picture fill inside a rounded-rect/ellipse shape, not `p:pic`)
  - `a:custGeom` with cubic beziers (`cubicBezTo`) + `fill="none"` open stroke path
  - Slide background via `<p:bg><p:bgRef idx="1001"><a:schemeClr val="accent1"/></p:bgRef></p:bg>`
- **Why it might break:** Canva heavily uses `<p:bgRef>`, picture-filled shapes, and nested group transforms; missing `<p:bgRef>` leaves slides blank, and broken group math creates Aurelie's "stray lines".
- **Where / how:** `crates/pptx/src/m26_fidelity_tests.rs` using `build_pptx` + `import_pptx`.
- **Setup & steps:** Build synthetic PPTX containing all 4 Canva constructs, import, and assert exact background colour, shape `FillStyle::Picture`, nested child frame coordinates within `±2px`, and `Path` geometry.
- **Expected result:** All Canva-style constructs import accurately without stray lines or missing pictures/backgrounds.
- **Priority:** P1

### A13.5 — Master & layout inheritance cascade (background, `txStyles`, decorative master shapes)
- **What it checks:**
  1. Slide has no `<p:bg>`, layout has no `<p:bg>`, master has `<p:bg>` solid `#223344` → slide inherits `#223344`.
  2. Layout placeholder omits font size/colour, master `<p:txStyles><p:titleStyle>` defines 40pt `#FFEEDD` → slide title inherits master `titleStyle`.
  3. Master `<p:spTree>` has a decorative logo rectangle (non-placeholder) → appears behind slide content (or on the master).
- **Why it might break:** Layout-only inheritance works on S18, while many real decks put backgrounds and typography only on `slideMaster1.xml`!
- **Where / how:** `crates/pptx/src/m26_fidelity_tests.rs` using `build_pptx` + `import_pptx`.
- **Setup & steps:** Build slide → layout → master chain, import, and assert inherited background, title run color/size, and decorative master object presence.
- **Expected result:** Full slide → layout → master fallback chain works for backgrounds, typography, and master graphics.
- **Priority:** P2

### A13.6 — Color map overrides (`p:clrMapOvr`) on dark-themed slides
- **What it checks:** A slide overriding `<p:clrMapOvr><a:overrideClrMapping bg1="dk1" tx1="lt1" .../></p:clrMapOvr>` so `schemeClr val="bg1"` resolves dark (`#111111`) and `schemeClr val="tx1"` resolves white (`#FFFFFF`).
- **Why it might break:** Ignoring `p:clrMapOvr` renders dark-section divider slides as black text on black background (or white on white — "blank slide"!).
- **Where / how:** `crates/pptx/src/m26_fidelity_tests.rs` using `build_pptx` + `import_pptx`.
- **Setup & steps:** Build a 2-slide deck sharing one theme/master where Slide 2 uses `overrideClrMapping` swapping `dk1`/`lt1`, import, and assert Slide 2's background is `#111111` and `tx1` text run colour is `#FFFFFF`.
- **Expected result:** Per-slide `clrMapOvr` remaps scheme colours accurately.
- **Priority:** P2

### A13.7 — Non-16:9 slide sizes (4:3 standard & 9:16 portrait)
- **What it checks:** Importing a 4:3 deck (`cx=9144000, cy=6858000`) and a vertical 9:16 deck (`cx=5143500, cy=9144000`).
- **Why it might break:** Hardcoding 16:9 (`1280×720`) EMU conversion or font scale factor distorts aspect ratios or clips objects off-slide on 4:3 or portrait decks.
- **Where / how:** `crates/pptx/src/m26_fidelity_tests.rs` using `import_pptx`.
- **Setup & steps:** Build 4:3 and 9:16 decks with a centered square shape and 24pt text; import; assert `course.slide_size` preserves exact aspect ratio (`w/h ≈ 4/3` and `9/16`), shape remains centered and square (`|w - h| < 1.0`), and `course.validate()` passes.
- **Expected result:** Non-16:9 decks scale proportionally with zero distortion.
- **Priority:** P2

### A13.8 — Hostile / malformed PPTX inputs (never crash, never hang, never traverse paths)
- **What it checks:**
  1. Group with `chExt cx="0" cy="0"` (division by zero in scale transform!)
  2. Negative `ext` / `off` and `u64::MAX` / NaN-like huge EMU values (must clamp or skip single shape, NOT abort whole deck!)
  3. Missing relationship target, external `http://evil.invalid/x.png` image target, and path-traversal `Target="../../etc/passwd"`
  4. Corrupt XML inside `slide2.xml` alongside valid `slide1.xml`
  5. Truncated/non-zip bytes and 0-slide presentation (`<p:sldIdLst/>`)
  6. Deck where ALL slides are hidden (`show="0"`)
- **Why it might break:** Golden Rule #1 ("Never crash; input-derived numbers are hostile"). `report.md` already noted one bad freeform frame used to abort the whole deck!
- **Where / how:** `crates/pptx/src/m26_fidelity_tests.rs`.
- **Setup & steps:** Run `import_pptx` on each hostile payload; assert no panic, no network/filesystem reads, graceful `Err(...)` or valid `Course` with skip warnings, and `course.validate()` passes on every `Ok` result.
- **Expected result:** Zero panics, zero path traversal, and single bad shapes do not abort otherwise valid slides.
- **Priority:** P3

### A13.9 — Edit imported `fidelity.pptx` in Editor UI (`TestHarness`) + Undo/Redo + `.ccraft` round-trip
- **What it checks:** Opening the imported `fidelity.pptx` course directly in `TestHarness`, then performing M26 edits on imported objects: edit text `Font size` (A1) on S08, reorder gradient stops (A2) on S06, format-paint (A11) between two shapes on S01, undo all 3, redo all 3, pack to `target/m26-fid-edited.ccraft`, reopen, and assert identical.
- **Why it might break:** Imported objects can carry unusual combinations (`runs` with explicit font/baseline, `Path` with `subpaths` and `fill_style`, `Image` with `crop` + `shape` + `outline`) that hand-authored courses don't have, exposing crashes in inspector sync or format painter!
- **Where / how:** `apps/coursecraft-cli/tests/m26_pptx_fidelity.rs` (or `crates/ui-egui` test).
- **Setup & steps:** Load `fidelity.pptx` into `TestHarness`, run the A1/A2/A11 edits on S08/S06/S01, run `layout_report(2)` on S01/S06/S08/S14, undo/redo, pack & unpack `.ccraft`, verify `course.validate()`.
- **Expected result:** Imported PPTX courses are fully editable in the UI with zero panics or layout clipping.
- **Priority:** P1

### A13.10 — CLI `import-pptx` → `validate` → `render` → `export-pdf` pipeline
- **What it checks:** Running the real CLI commands end-to-end on `fidelity.pptx`:
  - `coursecraft-cli import-pptx <fidelity.pptx> -o target/m26-cli-fid.ccraft`
  - `coursecraft-cli validate target/m26-cli-fid.ccraft`
  - `coursecraft-cli render target/m26-cli-fid.ccraft --slide slide16 -o target/m26-cli-s16.png`
  - `coursecraft-cli export-pdf target/m26-cli-fid.ccraft -o target/m26-cli-fid.pdf`
- **Why it might break:** CLI `import-pptx` media packing or PDF exporter could fail on imported video posters, custom paths, or gradient backgrounds even when in-memory `import_pptx` succeeds.
- **Where / how:** `apps/coursecraft-cli/tests/m26_pptx_fidelity.rs` invoking `coursecraft_cli::run` or the compiled `coursecraft-cli` binary.
- **Setup & steps:** Run the 4 CLI commands sequentially writing under `target/`, assert exit code 0 for all 4, assert `m26-cli-s16.png` is non-white (gradient background rendered!) and `m26-cli-fid.pdf` starts with `%PDF-`.
- **Expected result:** Full CLI import → validate → render → export-pdf pipeline succeeds cleanly.
- **Priority:** P3

---

## Cross-cutting checks

Tests that span multiple A1–A13 items together to catch interaction bugs between features.

### CC.1 — Multi-feature Undo/Redo stack atomicity & inspector/canvas sync
- **What it checks:** In one continuous `TestHarness` session, perform 6 sequential M26 operations: (1) change text `Font size` & `Spacing` (A1), (2) add & reorder 3rd gradient stop (A2), (3) draw a `Heart` shape (A5), (4) switch `Base slide` to `Hide` on a layer (A9), (5) edit Question prompt in right panel (A10), (6) Format Paint from one shape to another (A11). Then undo all 6 steps one by one, checking course model + inspector displayed values + canvas after EVERY undo, then redo all 6 steps.
- **Why it might break:** Cached inspector buffers (`text_size_seen`, `line_spacing_seen`, `fill_style`, `question.prompt_buf`) frequently drift out of sync across multi-step undo/redo.
- **Where / how:** `crates/ui-egui/src/m26_qa_crosscut_tests.rs` using `TestHarness`.
- **Expected result:** Every single undo and redo step restores exact model, inspector, and canvas state; initial state after 6 undos equals original course.
- **Priority:** P1

### CC.2 — Save, `.ccraft` pack/unpack, reopen & `coursecraft-cli diff` lossless round-trip
- **What it checks:** Construct a rich course combining A1 (letter spacing, line spacing, justify, vertical text, insets), A2 (4-stop linear + 3-stop radial gradients), A3 (slide transition + Fly out & Fade exits), A7 (video with poster & trim), A9 (`base_slide_view: Hide`), A10 (question slide), and A13 (imported custom path with subpaths + gradient). Save to `target/m26-cc2.ccraft` via `coursecraft-format`, unpack with `coursecraft-cli unpack`, re-pack with `coursecraft-cli pack`, run `coursecraft-cli diff` between original and round-tripped `.ccraft` (expect exit code 0 = identical!), and run `coursecraft-cli validate`.
- **Why it might break:** Missing serde fields, `skip_serializing_if` bugs, or media manifest omissions silently lose M26 data on save/reopen.
- **Where / how:** `apps/coursecraft-cli/tests/m26_cli_and_packaging.rs`.
- **Expected result:** `pack` → `unpack` → `pack` produces zero `diff` lines and zero validation errors.
- **Priority:** P1

### CC.3 — Published Web Player parity (Editor Preview vs `render_player_rgba` vs published `web` zip)
- **What it checks:** Take the rich course from CC.2 (3-stop gradients, slide transition + Fly out exit, `base_slide_view: Hide`, question slide, imported PPTX slide), publish it with `coursecraft-cli publish --standard web -o target/m26-web.zip` (using `target/player-dist` when present), unzip the published `course.json` + media, load it in `Player` + `render_player_rgba`, and compare against Editor Preview's `render_player_rgba` at key timestamps (t=0, t=2000ms, t=4500ms mid-exit, and after answering the question).
- **Why it might break:** Golden Rule #3 ("One runtime"): published `course.json` transformation, media path rewriting, or `base_slide_view` leakage could cause published courses to diverge from Editor Preview.
- **Where / how:** `apps/coursecraft-cli/tests/m26_cli_and_packaging.rs`.
- **Expected result:** Published package `course.json` renders pixel-identically to Editor Preview at every sampled timestamp, and all media referenced by gradients/posters/images exist inside the zip.
- **Priority:** P1

### CC.4 — Multi-select matrix across M26 features
- **What it checks:** Select multiple heterogeneous objects (2 text boxes, 2 gradient shapes, 1 video + 1 shape, 1 question submit button + 1 text box): verify A1 multi-select `Size`, A2 non-destructive gradient preservation, A7 no video decode hang, A10 clean inspector transition, A11 `Paint` disabled on multi-select + `Cmd/Ctrl+Alt+V` applying style to multi-select.
- **Why it might break:** Multi-select takes a completely separate inspector code path (`show_multi_properties`) and toolbar gating (`can_arm_format_painter`).
- **Where / how:** `crates/ui-egui/src/m26_qa_crosscut_tests.rs` using `TestHarness`.
- **Expected result:** All heterogeneous multi-selections work cleanly with zero layout errors or data loss.
- **Priority:** P2

### CC.5 — Layers & States matrix across M26 features
- **What it checks:** On a single multi-layer slide with stateful objects: edit text size/spacing on a layer (A1), render 3-stop gradient on a layer and on Hover/Selected states (A2), run exit animation on a layer while base has a transition (A3), scrub layer timeline (A4), draw a `Heart` onto a layer (A5), toggle `Show`/`Dim`/`Hide` (A9), and Format Paint Base ↔ Layer (A11).
- **Why it might break:** Many editor paths assume objects live on `slide.objects` (base) rather than `slide.layers[i].objects`, or assume `Normal` state.
- **Where / how:** `crates/ui-egui/src/m26_qa_crosscut_tests.rs` using `TestHarness` + `Player` + `render_player_rgba`.
- **Expected result:** Every M26 feature works seamlessly on slide layers and stateful objects.
- **Priority:** P1

### CC.6 — Window size & theme sweep (`layout_report` zero-clipping audit)
- **What it checks:** Run `TestHarness::layout_report(3)` across 3 window sizes (`1024×680`, `1280×900`, `1920×1080`) × 2 themes (`ThemeMode::Dark`, `ThemeMode::Light`) while inspecting:
  - Text box Format panel (A1)
  - 4-stop linear gradient Format panel (A2)
  - `Insert Interaction` & `Shapes` dialogs on 2nd open (A6)
  - Active Layer with 4 layers + `Base slide: Show | Dim | Hide` pills (A9)
  - Question slide right panel (A10)
  - Preview modal with Effect Log + Variables open (A12)
- **Why it might break:** Compact windows (`1024×680`) and Light theme frequently expose clipped widgets, overlapping labels, or dialogs crossing the menu bar.
- **Where / how:** `crates/ui-egui/src/m26_qa_crosscut_tests.rs` using `TestHarness::new_with_theme`.
- **Expected result:** Zero `clipped-widget`, `overlap`, `window-off-screen`, or `window-over-menu-bar` findings across all 6 `(size, theme)` combinations.
- **Priority:** P1

### CC.7 — Escape & keyboard focus precedence chain
- **What it checks:** Verifies deterministic `Escape` priority without unintended cascade:
  1. Text input focused inside Preview → `Esc` blurs input (Preview stays open) → 2nd `Esc` closes Preview.
  2. Outline drawer open inside Preview → `Esc` closes drawer (Preview stays open) → 2nd `Esc` closes Preview.
  3. Format Painter armed on canvas → `Esc` disarms Paint (selection stays intact).
  4. Shape placement drag active → `Esc` cancels drag & tool (no object placed).
  5. Inline text edit open → `Esc` closes inline edit (selection stays intact).
  6. Dialog open (`Insert Interaction` / `Shapes`) → `Esc` closes dialog.
- **Why it might break:** Multiple subsystems call `ctx.input(|i| i.key_pressed(Key::Escape))` instead of `consume_key`, causing one `Esc` press to close two layers of UI simultaneously!
- **Where / how:** `crates/ui-egui/src/m26_qa_crosscut_tests.rs` using `TestHarness`.
- **Expected result:** Each `Esc` press dismisses exactly the single topmost active mode/focus level.
- **Priority:** P2

### CC.8 — Workspace architecture & quality gates (`layers`, `wasm`, `a11y-check`)
- **What it checks:** After implementing all planned tests:
  - `cargo xtask layers` exits 0 (no illegal cross-layer dependencies introduced).
  - `cargo xtask wasm` exits 0 (L0–L5 and `coursecraft-player` still compile for `wasm32-unknown-unknown`).
  - `coursecraft-cli validate` and `coursecraft-cli a11y-check` pass on the generated test fixtures (including `fidelity.pptx` import).
- **Why it might break:** Adding a test dependency carelessly in `[dependencies]` instead of `[dev-dependencies]`, or adding desktop-only code in L0–L5, breaks layering or wasm.
- **Where / how:** Shell gate commands run by the test implementer after writing the suite.
- **Expected result:** All workspace gates (`layers`, `wasm`, `validate`) pass cleanly.
- **Priority:** P2
