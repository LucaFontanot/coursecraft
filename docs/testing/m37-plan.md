# M37 Plan — Issue #7 (Point Text) and Issue #8 Part B (One State for a Grouped Button)

**Role:** Architecture, Runtime Semantics & Test Plan (planning only — zero source files edited)  
**Target release:** 0.4.6 / M37 (lands after M36 Refactor A and 0.4.5's Issue #8 Part A timeline group folders)  
**Source requests:**
- **GitHub issue #7:** *Text box placed with a click is a fixed 300×60 wrapping box; short labels need a manual resize*
- **GitHub issue #8 (Part B only):** *Groups: share hover/pressed states across a grouped button* (Part A — collapsible group folder rows on the timeline — already shipped in 0.4.5 and is out of scope)

**Clean-room commitment:** Both features match capabilities authors expect from established slide-based authoring tools (point text vs area text, and interactive button groups with one shared state), implemented from scratch in pure Rust with CourseCraft's own schema, runtime contract, UI tokens (`DESIGN.md` / `crates/render/src/design.rs`), and generic clean-room wording (`"Wrap text"`, `"Make button from group"`, `"Act as one button"`, `"Button group"`). Zero competitor code, assets, or coined terms (`docs/contributing.md:21-40`).

> **Implementer Guardrails (`AGENTS.md`, `docs/contributing.md`, `docs/testing/m28-uiux-plan.md`)**
> 1. **Never crash:** Zero `unwrap`/`expect`/`panic!`/`unreachable!`/`todo!` in non-test code; `unsafe_code = "forbid"` everywhere except `coursecraft-video-native`. Every numeric input, text length, group id, and frame dimension is hostile.
> 2. **Everything is a command:** All authoring mutations live in `crates/engine/src/commands/` with a single-row `REGISTRY` entry declaring `scope` and `slide_validation` (`docs/testing/m36-report.md`). UIs only dispatch.
> 3. **One runtime:** Every play-time rule for interactive groups lives **only** in `coursecraft-logic`. Preview (`crates/ui-egui/src/preview.rs`) and published web/SCORM packages (`apps/coursecraft-player`) run the exact same `Player`, `PlayerFrame`, `SlideView`, `FocusManager`, and `AccessTree`.
> 4. **Spec first (same step):** `docs/spec/triggers.md` MUST be updated with the interactive-group state rule in the **same step** (`S5`) that implements `coursecraft-logic`. `docs/spec/course-model.md` MUST be updated in the same step (`S1`, `S4`) that adds model fields.
> 5. **Layering & WASM:** Never break `cargo xtask layers` or `wasm32-unknown-unknown` for L0–L5, the web editor, or the web player. Below L6, only `coursecraft-render` may depend on `egui` (`docs/architecture.md:31`).
> 6. **Backward compatibility:** Old courses lacking the new fields MUST load unchanged, behave identically, and serialize **byte-identically**.
> 7. **Registry & CLI lock:** Any new command or `REGISTRY` `params` change MUST update `r5_01_registry_golden_lock` (`crates/engine/src/tests.rs:12246`) and `docs/cli.md` (verified by `coursecraft-cli` tests).
> 8. **M28 UI/UX compliance:** Sentence case, positive toggle labels, no trailing colons, palette tokens only (`UX-06`, `UX-07`), ≥ 24×24 pt compact hit targets (`UX-05`), single scroll area (`UX-02`), and green `uiux_sweep_tests.rs`.

---


> **Reviewer notes (Claude) — these override the plan below.**
> 1. **Measurement must match what the editor draws.** The engine's `skrifa` fit and the egui text layout must agree: add a test that lays out the same point text both ways (several strings, sizes, bold/italic, letter spacing, emoji/symbols) and asserts the drawn text fits inside the fitted box with no wrap and at most a few px of slack. If they can't agree, the UI passes its measured size to the command and headless uses `skrifa`.
> 2. **No silent trigger migration.** Making a group interactive must not move or delete existing member triggers. Member triggers keep working exactly as today; triggers attached to the group are new, and the group's Click fires once. If an easy "move click triggers to the group" helper is offered, it is an explicit, undoable command the author chooses, never automatic. Turning interactive off keeps group triggers in the file but inactive only if the spec says so clearly; otherwise ask the author before removing them (prefer: block the opt-out with a clear message while group triggers exist).
> 3. **Backward compatibility is a hard gate:** golden byte-compare tests for existing course JSON, `.ccraft` files and published `course.json` land before model changes and must pass unchanged. Older `suspend_data` resumes correctly.
> 4. `docs/spec/triggers.md` changes in the same commit as the runtime change (S5), and `docs/cli.md` in the same commit as new commands.
> 5. Preview and the published web/SCORM player must behave identically: add a parity test that drives both through the same pointer and keyboard sequence on an interactive group.

## 1. Goals

### 1.1 Issue #7 — Point Text (Click) vs Area Text (Drag)
1. Clicking on the stage (or inside the State Editor container) with the **Text box** tool creates **point text** (`wrap: false`, auto-sized to content): one line wide enough for its text plus padding, growing and shrinking live as the author types.
2. Dragging with the **Text box** tool keeps today's **area text** behaviour: fixed width at the dragged size with `wrap: true`.
3. Turning **Wrap text** on or dragging a width resize handle converts point text into fixed-width wrapping area text in one undo step.
4. Short labels such as `"Next"` and `"Back"` align and centre on buttons without manual resizing.
5. Fitting runs in `coursecraft-engine` using headless, WASM-safe font measurement (`coursecraft-media` via `skrifa`), so CLI, MCP, and canvas click inserts all produce identical fitted boxes.
6. Existing courses load, render, and re-serialize byte-identically.

### 1.2 Issue #8 Part B — One State for a Grouped Button
1. Authors can opt an existing group into an **interactive group** (*"Make button from group"* / *"Act as one button"*), turning a multi-object button (e.g., rectangle + icon + text) into one interactive control.
2. All members share one runtime and pointer state: hovering any member puts every member into `hover`; pressing any member puts every member into `down`; `selected`, `visited`, `disabled`, `hidden`, and custom states apply to the whole group.
3. Each member renders its own `StateDef` look for the active state, or stays `normal` if it has no override for that state.
4. A hit on any member (or overhanging state extra) counts as a hit on the group; moving the pointer between members inside the same group emits no spurious `hover_end`/`hover_start`.
5. Triggers attached to the group fire **once** (not once per member), and built-in `selected`/`visited` transitions apply to the whole group once.
6. Keyboard focus (`Tab`) reaches the group **once** (skipping individual members), `Enter`/`Space` activates it, the focus ring surrounds the union group bounds, and assistive technology sees one named `button` control.
7. Editor **Preview** and published **Web/SCORM/xAPI/cmi5/LTI** output behave identically, including SCORM `suspend_data` persistence across resume.
8. Groups without the interactive flag behave identically to today, and existing courses serialize byte-identically.

---

## 2. Verified Current State

Every file and symbol below was verified directly against the current repository after M36 Refactor A and 0.4.5:

### 2.1 Issue #7 Baseline
- **Model (`crates/course/src/model.rs`):**
  - `ObjectKind::Text` (`:2140-2160`) carries `text`, `style: TextStyle`, `runs`, `paragraphs`, `autofit: Autofit`, `text_style`, `outline`.
  - `StateExtraKind::Text` (`:4058-4070`) carries `text`, `style`, `runs`, `paragraphs`, `autofit`, `outline`.
  - `TextStyle` (`:3621-3658`) already has `pub wrap: bool` (`#[serde(default = "yes", skip_serializing_if = "is_true")]`, default `true`), `insets: Option<TextInsets>`, `direction: TextDirection`, `align: Align`, `valign: VAlign`.
  - `Autofit` (`:2785-2789`) has only `None` (default) and `Shrink`. It is shared by `ObjectKind::Text`, `ShapeText` (`:3144`), `StateDef.autofit` (`:3960`), and `StateExtraKind::Text` (`:4067`).
- **Text Measurement & Fonts:**
  - `coursecraft-media` (`crates/media/Cargo.toml`, Layer 2, standalone, WASM-safe, zero `egui`) already depends on `skrifa = "0.44"` and reads TrueType/OpenType tables in `crates/media/src/font.rs:64-229`. Its tests (`crates/media/src/tests.rs:14-16`) already reference `Inter-Regular.ttf`, `Inter-Bold.ttf`, and `Inter-Italic.ttf`.
  - `crates/render/src/font.rs:65-72` bundles `Inter-{Regular,Medium,SemiBold,Bold,Italic,BoldItalic}.ttf` (~57 KB each) and `Phosphor*.ttf`.
  - `crates/render/src/text_layout.rs:703` (`layout_text_with_wrap`) supports `wrap: bool`, and `crates/render/src/paint.rs:1609` (`paint_text_with_options`) already passes `style.wrap`.
  - **Existing bug spotted:** `crates/render/src/text_layout.rs:1799-1831` (`inline_edit_layout_job`) sets `TextWrapping.max_width` using `if multiline { wrap_width.max(1.0) } else { f32::INFINITY }`, ignoring `style.wrap == false` during inline editing!
- **Engine (`crates/engine`):**
  - `crates/engine/Cargo.toml` (Layer 5) depends on `coursecraft-course` and `coursecraft-format` (adding `coursecraft-media` is L5 → L2, legal under `xtask/src/main.rs:28-50` and `WASM_CRATES:132-150`, with zero `egui`).
  - Commands live under `crates/engine/src/commands/`: `object.rs` (`object_add:18`, `object_update:143`, `object_set_frame:963`, `object_apply_style:1240`), `object_states.rs` (`object_state_add_extra:525`, `object_state_update_extra:560`), `text.rs` (`text_set:113`, `text_style_range:155`, `text_set_paragraph:229`, `text_set_paragraphs:297`, `object_set_autofit:416`).
  - `REGISTRY` in `crates/engine/src/commands/mod.rs` has 186 rows locked by `r5_01_registry_golden_lock` (`crates/engine/src/tests.rs:12246`).
- **Canvas & Inspector (`crates/ui-egui`):**
  - `crates/ui-egui/src/canvas.rs:1947-1958` (`placement_kind_and_size`) returns hardcoded `(300.0, 60.0)` and omits `wrap` (defaulting `true`) for `PlacementTool::TextBox`.
  - `crates/ui-egui/src/canvas.rs:3335-3347` already distinguishes `is_click` vs drag (`moved < PLACEMENT_CLICK_SLOP`), yet calls `place_object(editor, &tool, frame)` identically in both branches.
  - `crates/ui-egui/src/canvas.rs:2362-2426` (`place_object` inside `editor.ui.editing_state` — State Editor container placement at `:2379`) constructs `StateExtraKind::Text` with default `wrap: true` and does not auto-fit on click.
  - `crates/ui-egui/src/canvas.rs:4728-4755` commits resize drags via `object.set_frame`.
  - `crates/ui-egui/src/canvas.rs:775-1095` (`commit_inline_edit`) commits plain/rich text edits in one `text.set` (or `object.state_update_extra`) step; `draw_inline_edit` (`:5927-6250`) renders the inline `TextEdit` at fixed `inline_edit_slide_rect`.
  - `crates/ui-egui/src/properties.rs:4884-4892` already exposes `"Wrap text"` dispatching `text.set_paragraph { wrap }`, and `:4547-4585` exposes `Autofit` (`Off` / `Shrink text on overflow`).

### 2.2 Issue #8 Part B Baseline
- **Model & Validation (`crates/course`):**
  - `Object.group: Option<String>` (`crates/course/src/model.rs:1066`) is currently authoring-only; `Slide` (`:890`) and `Layer` (`:961`) have no `groups` collection.
  - `Slide::object` (`crates/course/src/lib.rs:461`) and `Slide::local_ids` (`:474`) know only objects and layers.
  - `crates/course/src/validate.rs:1073,1149,1404` (`check_group_members`) only warns when a group has 1 member; `check_trigger` (`:3315-3403`), `check_action` (`:3457`), and `Condition::ObjectState` (`:3876`) validate `object` strictly against `slide.object(id)`.
- **Undo Delta (`crates/engine/src/delta.rs`):**
  - `SlideFieldPatch` and `SlideDelta::diff`/`apply` (`:175-393`) destructure every field of `Slide` exhaustively; adding `Slide.groups` must be wired into `SlideFieldPatch` and `SlideDelta`.
- **Runtime (`crates/logic`):**
  - `PlayerState.object_states: BTreeMap<String, BTreeMap<String, String>>` (`crates/logic/src/lib.rs:665`) stores per-slide runtime states keyed by id.
  - `Player::object_state` (`:1609`), `set_state` (`run.rs:1911`), `object_is_live` (`:2439`), `click_check` (`:2489`), `keyboard_objects` (`:2565`), `click` (`:2609`), `Input::HoverStart`/`HoverEnd` (`:263`), `Input::FocusChanged` (`:315`), and `Condition::ObjectState` (`:639`) all look up `slide.object(id)` individually.
  - `crates/logic/src/suspend.rs:372-398,513-545` serializes modified `object_states` by enumerating `slide_objects(slide)` indices (`0..all_objects.len()`).
- **Renderer & Web Player (`crates/render`, `apps/coursecraft-player`):**
  - `compose_with_pointer` (`crates/render/src/compose.rs:2769`) composes base and layer objects using `player.object_state(&obj.id)` and `pointer_state_for(hovered, pressed, &obj.id)` (`:1155`), sets `hit: Hit::Object(object.id.clone())` (`:1872`), and post-processes scroll panels (`:2995,3021`).
  - `paint_items` (`crates/render/src/paint.rs:2442-2443`) computes `is_pressed`/`is_hovered` by comparing `highlight.pressed`/`highlight.hovered` to `Some(&item.hit)` and feeds them into `resolve_pointer_fill` (`:1401`, called at `:2625` for buttons and `:2735` for shapes).
  - `SlideView` (`crates/render/src/view.rs:293-564,1086-1146,1437-1514`) tracks `hovered`/`pressed` by `Hit::Object(id)`, emits `Input::HoverStart`/`HoverEnd`/`Click`, and chooses cursor icon via `is_object_clickable`.
  - `FocusManager` (`crates/render/src/focus.rs:398-422,586-598,1081,1220,1885-1923,2198-2227,2321`) builds the Tab list from `player.keyboard_objects()`, activates `FocusTarget::Object(id)` with `Input::Click`, and paints a focus ring around the single topmost matching `DrawItem`.
  - `build_access_tree` (`crates/render/src/access.rs:98-339,1124-1220`) builds `AccessNode`s for every interactive target and non-interactive `content_node`; `apps/coursecraft-player/src/a11y.rs:340-405` mirrors `AccessTree` into DOM elements purely by `AccessRole` and `FocusTarget` (zero `slide.object(` lookups).
  - Editor Preview (`crates/ui-egui/src/preview.rs:167-169`) and Published Web Player (`apps/coursecraft-player/src/app.rs:72-74`) both render through the shared `PlayerFrame` + `SlideView`.
- **Editor UI (`crates/ui-egui`):**
  - `Selection::is_one_group` (`crates/ui-egui/src/selection.rs:233`) already detects when the selection is exactly one complete group.
  - `crates/ui-egui/src/properties.rs:3241-3282` renders the Group header and `Group`/`Ungroup` button.
  - `crates/ui-egui/src/menu.rs:1662-1698` renders `Group`/`Ungroup` in Arrange and context menus.
  - `crates/ui-egui/src/timeline.rs:2233-2399` renders Part A's collapsible group folder rows.
  - `crates/ui-egui/src/triggers.rs:13-80,948` resolves `object_name`/`object_label` and initializes new triggers from the selected object.

---

## 3. Design — Issue #7: Point Text vs Area Text

### 3.1 Model Representation & Backward Compatibility
Add one optional boolean field to `ObjectKind::Text` (`crates/course/src/model.rs:2140`) and `StateExtraKind::Text` (`crates/course/src/model.rs:4058`):

```rust
/// When true, the box acts as point text (`style.wrap == false`, `autofit == None`):
/// its width and height fit its content automatically as text or style changes.
/// Omitted when false so existing courses stay byte-identical.
#[serde(default, skip_serializing_if = "is_false")]
pub point_text: bool,
```

**Why `point_text: bool` rather than inferring from `wrap: false` or adding `Autofit::Grow`:**
1. **No breakage on existing `wrap: false` boxes:** Existing courses and PPTX imports can carry `wrap: false` on a fixed-width box; an explicit opt-in flag guarantees zero unintended frame changes on load or edit of old files.
2. **Byte-identical JSON:** Default is `false` and `skip_serializing_if = "is_false"` omits the key completely on all area text boxes and every existing course. `schema_version` stays `1` (same additive-optional policy as `outline`, `insets`, `direction`, `text_style`).
3. **Clean separation from `Autofit`:** `Autofit` is shared with `ShapeText` and `StateDef` (neither of which can auto-resize an object's `frame`). Keeping `Autofit` focused on `None | Shrink` avoids exposing invalid point-text options on shapes and state overrides.
4. **Interaction with `Autofit` (and decision on `Grow`):**
   - Point text enforces `style.wrap = false` and `autofit = Autofit::None`.
   - Switching `autofit` to `Autofit::Shrink` (via `object.set_autofit`) clears `point_text = false` so the fixed box shrinks overflowing text as requested.
   - **`Autofit::Grow` (fixed-width wrapping area text whose height grows) is explicitly deferred** to a follow-up milestone: Issue #7 lists it only under "Notes" as related work, not in Proposed behaviour or Acceptance Criteria, and it affects `ShapeText`, `StateDef`, and PPTX import. The headless `coursecraft-media::text_metrics` engine built here provides the exact measurement foundation a future `Autofit::Grow` can reuse.

### 3.2 Where Fitting Happens — Headless & WASM Measurement Architecture
**Decision:** Frame fitting runs in **`coursecraft-engine`**, powered by pure-Rust font measurement in **`coursecraft-media`** (`skrifa`), with optional exact galley hints from the editor for custom/system fonts.

```text
coursecraft-media (L2, standalone, WASM-safe, zero egui)
  ├── fonts/Inter-{Regular,Bold,Italic,BoldItalic}.ttf  (single copy; re-exported to render!)
  └── src/text_metrics.rs
        ├── pub const INTER_REGULAR / INTER_BOLD / INTER_ITALIC / INTER_BOLD_ITALIC
        ├── pub const POINT_TEXT_PAD_X: f32 = 4.0;   // per side (8.0 total)
        ├── pub const POINT_TEXT_PAD_Y: f32 = 2.0;   // per side (4.0 total)
        ├── pub const POINT_TEXT_MIN_W: f32 = 20.0;
        └── pub fn measure_unwrapped_text(...) -> (f32, f32)   // skrifa FontRef + Metrics + GlyphMetrics

coursecraft-render (L3)
  ├── src/font.rs        -> uses coursecraft_media::text_metrics::INTER_* (zero duplicated bytes)
  └── src/text_layout.rs -> inline_edit_layout_job respects `wrap: bool` (max_width = INFINITY when !wrap)

coursecraft-engine (L5, adds coursecraft-media.workspace = true; L5 -> L2 legal, WASM-safe, zero egui)
  └── src/commands/text.rs::fit_point_text_frame(...)
        ├── Uses `measured_size` hint if provided & finite (from canvas with custom/system font)
        └── Else computes exact size via `coursecraft_media::text_metrics::measure_unwrapped_text`
            (CLI, MCP, headless scripts, and default Inter font in editor all get identical fit!)
```

**Exact measurement & anchoring rules (`measure_unwrapped_text` + `fit_point_text_frame`):**
1. Split resolved runs on `\n` into paragraphs (supporting single-line labels and explicit Shift+Enter/Enter line breaks).
2. For each run segment, resolve the `skrifa::FontRef` for `(bold, italic)` (from bundled Inter; `FontRef::new` is zero-copy, no heap allocation). Scale by `size.clamp(1.0, 400.0)` (times `0.65` for `Superscript`/`Subscript`, matching `text_layout.rs:919`).
3. Sum per-char `glyph_metrics.advance_width(gid)` + `clamp_letter_spacing(letter_spacing)` across the paragraph; add list marker indent if `paragraph.list != ListKind::None`.
4. Paragraph height = `(metrics.ascent - metrics.descent + metrics.leading).max(size) * line_spacing.clamp(0.8, 3.0)` + `space_before` + `space_after`. Total height is the sum over paragraphs; total width is the max paragraph width.
5. Add padding: `w = (content_w + insets.left + insets.right + 2.0 * POINT_TEXT_PAD_X).ceil().max(POINT_TEXT_MIN_W)` and `h = (content_h + insets.top + insets.bottom + 2.0 * POINT_TEXT_PAD_Y).ceil().max(min_line_h)`. Clamp both into `1.0..=10000.0`.
6. If `style.direction` is `Vertical` or `Vertical270`, swap `(w, h)`.
7. **Alignment-preserving anchor:**
   - `Align::Left | Align::Justify`: keep `frame.x`; `Align::Center`: shift `frame.x += (old_w - new_w) * 0.5`; `Align::Right`: shift `frame.x += old_w - new_w`.
   - `VAlign::Top`: keep `frame.y`; `VAlign::Middle`: shift `frame.y += (old_h - new_h) * 0.5`; `VAlign::Bottom`: shift `frame.y += old_h - new_h`.
   - Round `x, y, w, h` to whole slide pixels.

### 3.3 Engine Command Invariants & Undo Scope
Every mutation remains **one atomic undo step** (`UndoScope::SingleSlide`):

| Command | Behaviour on `point_text == true` | Undo steps |
|---|---|---:|
| `object.add` | Forces `style.wrap = false`, `autofit = Autofit::None`; fits `frame.w`/`frame.h` from initial text (`"Text"` or CLI-provided text), keeping `(frame.x, frame.y)` as anchor (accepts optional `measured_size`). | 1 |
| `object.state_add_extra` | Same normalization & fit when `extra.kind` is `StateExtraKind::Text { point_text: true, .. }` (State Editor container placement, `canvas.rs:2379`). | 1 |
| `text.set` | Updates `text`/`runs`/`paragraphs` AND refits `obj.frame` in the same mutation (accepts optional `measured_size: {w, h}` for custom fonts). | 1 |
| `object.state_update_extra` | If `StateExtraKind::Text { point_text: true, .. }`, refits `extra.frame` (and clears `point_text` if `wrap` turned `true` or width explicitly changed). | 1 |
| `text.style_range` / `text.set_paragraphs` / `object.apply_style` / `object.update` | If `point_text` remains `true` and `style.wrap == false`, refits `obj.frame` when font/size/bold/italic/spacing/text changes. If `object.update` sets `wrap: true`, clears `point_text = false`. | 1 |
| `text.set_paragraph` | If `wrap == Some(true)` → clears `point_text = false` (converts to area text at current width). Otherwise refits if `insets`/`direction` changed. | 1 |
| `object.set_autofit` | If `autofit == Autofit::Shrink` → clears `point_text = false`. | 1 |
| `object.set_frame` | If `(frame.w - obj.frame.w).abs() > 0.5` → converts to area text (`point_text = false`, `style.wrap = true`) at the new frame! Pure translation `(x, y)` (move, align, distribute, nudge) keeps `point_text = true` and `style.wrap = false`. | 1 |

### 3.4 Canvas Placement, Container Placement & Live Inline Editing
1. **Click vs Drag (`crates/ui-egui/src/canvas.rs:3335-3347`, `:2362-2480`):**
   - Change `place_object(editor, &tool, frame)` to `place_object(editor, &tool, frame, is_click: bool)`.
   - On `PlacementTool::TextBox` with `is_click == true`:
     - Build `"kind": { "type": "text", "text": "Text", "point_text": true, "style": { "color": theme.text, "wrap": false } }`.
     - Anchor top-left near click point (`x = start_slide.x`, `y = start_slide.y - initial_h * 0.5`, clamped inside slide) instead of centering a 300×60 box.
     - Applies identically in **both** normal slide placement (`object.add`) AND **State Editor container placement** (`canvas.rs:2379` → `object.state_add_extra`)!
   - On `PlacementTool::TextBox` with `is_click == false` (drag):
     - Exact current behaviour: `"point_text"` omitted (`false`), `"wrap"` omitted (`true`), uses the exact dragged `frame`.
2. **Live Box Growth While Typing (`canvas.rs:5927-6250`, `text_layout.rs:1799`):**
   - Pass `wrap: bool` to `inline_edit_layout_job` (or add `inline_edit_layout_job_with_wrap`) so `!wrap` sets `TextWrapping.max_width = f32::INFINITY` while preserving `break_on_newline = multiline`.
   - In `draw_inline_edit`, when the target is a `point_text == true` object (or state extra), compute the live fitted `slide_rect` from the current `buffer` using `coursecraft_render::layout_text_with_wrap(..., wrap = false)` + `POINT_TEXT_PAD_X`/`Y` + insets, anchored by `style.align`/`style.valign`. The dashed accent frame and `TextEdit` width grow/shrink smoothly on every keystroke without wrapping.
   - In `commit_inline_edit`, pass `"measured_size": { "w": fitted_w, "h": fitted_h }` to `text.set` (or `object.state_update_extra`) so custom/system fonts match the rendered galley 1:1.
   - Undo of insert (`Ctrl+Z` after commit, then `Ctrl+Z` again): first undo reverts text AND refitted frame to `"Text"` in one step; second undo deletes the inserted text box in one step — identical to today.
3. **Inspector (`crates/ui-egui/src/properties.rs:4884`):**
   - Existing `"Wrap text"` checkbox reflects `style.wrap` (unchecked for point text). Checking it dispatches `text.set_paragraph { wrap: true }`, converting point text to area text. Add a calm tooltip on the row when `point_text == true`: `"Fits width to text. Turn on wrap or drag a side handle for fixed width."`

---

## 4. Design — Issue #8 Part B: One State for a Grouped Button

### 4.1 Model Shape — Explicit `Group` Record on `Slide` and `Layer`
**Decision:** Add a `Group` metadata record stored in `Slide.groups` (for base-layer groups) and `Layer.groups` (for layer groups), rather than duplicating boolean flags across every member `Object`.

```rust
/// Optional metadata record for a group of objects on a slide or layer.
/// Plain authoring groups omit this record; interactive button groups store
/// their shared opt-in flag, accessible name, initial state, and focus order here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    /// Matches `Object.group` on the group's members on this slide/layer.
    /// Must be unique across objects, layers, and groups on the slide.
    pub id: String,
    /// Display and accessible name (e.g. "Back button"). Omitted when empty.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// When true, the group behaves as one interactive button at play time.
    /// Omitted when false.
    #[serde(default, skip_serializing_if = "is_false")]
    pub interactive: bool,
    /// Initial runtime state for an interactive group (`"normal"` default, omitted).
    #[serde(default = "normal_state", skip_serializing_if = "is_normal_state")]
    pub initial_state: String,
    /// Optional author-set keyboard focus position. Omitted when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus_order: Option<u32>,
    /// Accessible description override. Omitted when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
```

Add to both `Slide` (`crates/course/src/model.rs:890`) and `Layer` (`crates/course/src/model.rs:961`):
```rust
/// Group records carrying group-level behaviour (such as interactive button groups).
/// Omitted when empty so existing courses stay byte-identical.
#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub groups: Vec<Group>,
```

**Why `Group` records on `Slide`/`Layer` instead of a flag on `Object`:**
1. **Single source of truth:** `interactive`, `name`, `initial_state`, `focus_order`, and `description` live in one place — impossible for members to disagree on whether the group is interactive or what its initial state/name is.
2. **Zero new trigger variants:** Because an interactive `Group.id` is unique and disjoint from all object/layer IDs on the slide, triggers targeting the group simply store `group.id` in the existing `Event::Click { object }`, `Event::HoverStart { object }`, `Event::HoverEnd { object }`, `Event::StateChanges { object, to }`, `Event::KeyPressed { key, object }`, `Action::ChangeState { object, state }`, and `Condition::ObjectState { object, state, is }` fields!
3. **100% backward compatibility:** Existing courses have `groups: []` (`skip_serializing_if = "Vec::is_empty"`), and plain authoring groups created by `Ctrl+G` (`object.group`) do not create a `Group` record — so existing courses and plain groups stay **byte-identical** and behave identically to today.

**Validation rules (`crates/course/src/validate.rs`):**
- Every `Group` in `slide.groups` / `layer.groups`:
  - `id` must be a valid non-empty ID, unique across all groups on the slide, and disjoint from all `slide.local_ids()` (objects and layers).
  - Must have at least 1 member on its own owner (base or that layer) referencing `object.group == Some(g.id)` (error if 0 members; existing `check_group_members` warns if 1 member).
  - If `g.interactive == true`:
    - Forbidden on master slides (masters are non-interactive, `docs/spec/triggers.md:13-19`).
    - Members cannot be nested interactive widgets (`obj.kind.input_kind().is_some()`, `ScrollPanel`, `Web`, `Audio`, `Video`, `Marker`, `Table`) or question controls (hotspot image, drag items) — prevents broken nested focus/widget conflicts. Visual objects (`Shape`, `Text`, `Image`, `Path`, `Button`, `Lottie`) are all allowed.
    - `g.initial_state` must satisfy `slide.group_has_state(&g.id, &g.initial_state)` (`"normal"`, `"hidden"`, `"disabled"`, or any state defined in `states` of at least one member).
- **Trigger validation (`check_trigger`, `check_action`, `check_condition`):**
  - `Event::Click`, `Event::HoverStart`, `Event::HoverEnd`, `Event::KeyPressed { object: Some(..) }`, `Event::StateChanges`, `Action::ChangeState`, and `Condition::ObjectState` accept **either** a slide object id **or** an interactive group id (`slide.interactive_group(id).is_some()`).
  - Plain (non-interactive) group IDs are rejected by trigger validation.
  - State names on interactive group targets are validated via `slide.group_has_state(group_id, state)` (plus `BUILTIN_STATES` for `Event::StateChanges`).

### 4.2 Runtime Contract (`coursecraft-logic` + `docs/spec/triggers.md`)
All play-time semantics live **exclusively** in `coursecraft-logic` and are documented in `docs/spec/triggers.md` (`## Interactive groups`) in the same step (`S5`):

1. **Canonical target resolution:**
   - `Player::interactive_group_of(&self, object_id: &str) -> Option<&'c str>` returns `Some(&g.id)` when `object_id` belongs to an interactive group on the current slide, else `None`.
   - `Player::is_interactive_group(&self, id: &str) -> bool` returns `true` when `id` names an interactive group on the current slide.
   - `Player::group_members(&self, group_id: &str) -> Vec<&'c Object>` returns the group's members on its owning layer.
2. **State storage & resolution:**
   - On slide entry / reset / layer reset / lightbox entry / `suspend::rebuild_initial_states`, every interactive group `g` initializes `object_states[slide.id][g.id] = g.initial_state.clone()`.
   - `Player::object_state(id)`: if `id` is an interactive group `g` **or** a member of an interactive group `g`, returns `object_states[slide.id].get(&g.id)` (fallback `&g.initial_state`).
   - `set_state(target, state, q, out)`: resolves `target` through canonical target (`group_id` if interactive group or member of one); checks `slide.group_has_state(&g.id, state)`; updates `object_states[slide.id][g.id]`, emits **one** `Effect::StateChanged { object: g.id, state }` and queues **one** `Event::StateChanges { object: g.id, to: Some(state) }`.
   - `Condition::ObjectState { object, state, is }` calls `self.object_state(object)`, working uniformly for both group IDs and member IDs.
3. **Liveness & clickability:**
   - An interactive group `g` is live when its owning layer is visible (or base and `!base_objects_hidden()`), its state is not `"hidden"`, and at least one member is inside its timing window (`start_ms <= t < end_ms` and not exited).
   - `click_check(id)` resolves `id` to `g.id`: returns `NotLive` if not live, `Disabled` if group state is `"disabled"` (swallows click silently, no warning, no triggers), `Prevented` if base layer under a visible `prevent_base_clicks` layer, else `ReachesGroup { group, layer }`.
4. **Clicks & built-in states (`Input::Click { object }`):**
   - Whether `object` is `g.id` or any member id of `g`, `click` resolves to `g.id` and queues **one** `Event::Click { object: g.id }`.
   - Built-in post-click states (if click triggers stayed on slide and left `object_state(g.id)` unchanged):
     - If **any** member of `g` defines a `"selected"` `StateDef` → toggles group state `"normal"` ⇄ `"selected"`.
     - Else if **any** member of `g` defines a `"visited"` `StateDef` and current state != `"visited"` → sets group state to `"visited"`.
     - Applied via `set_state(&g.id, next, ...)` so all members transition together and `state_changes` fires once.
5. **Hover (`Input::HoverStart { object }` / `Input::HoverEnd { object }`):**
   - Resolves `object` to `g.id`, checks group liveness, and queues **one** `Event::HoverStart { object: g.id }` or `Event::HoverEnd { object: g.id }`.
6. **Keyboard focus order (`Player::keyboard_objects()`):**
   - Every live, non-disabled, non-prevented interactive group (with at least one member inside its scroll-panel viewport) is emitted **once** as `g.id` (at the position of its first member in layer order, with `g.focus_order` honoured by `FocusManager`).
   - Individual member objects of an interactive group are **excluded** from `keyboard_objects()`.
   - `Input::FocusChanged { object }` accepts interactive group IDs so `Event::KeyPressed { key, object: Some(g.id) }` fires while the group has focus.
7. **SCORM `suspend_data` (`crates/logic/src/suspend.rs`):**
   - In section 3 (`modified_object_states`), objects occupy indices `0..all_objects.len()`, and interactive groups (base then layers in authored order) occupy indices `all_objects.len()..all_objects.len() + interactive_groups.len()`, using a deterministic sorted state list per group (`"normal"`, `"hidden"`, `"disabled"`, plus unique `StateDef.name`s across members).
   - For any course with zero interactive groups, `suspend_data` encoding and decoding are **100% byte-identical** to today.

### 4.3 Rendering, Hit Testing, Hover/Down & Focus Ring (`coursecraft-render`)
1. **Composition (`crates/render/src/compose.rs`):**
   - When composing each base or layer object `obj`:
     - `current_state = player.object_state(&obj.id)` already returns the interactive group's shared state (`normal`, `selected`, `visited`, `disabled`, `hidden`, or custom). If `"hidden"`, every member is skipped automatically.
     - `let pointer_id = player.interactive_group_of(&obj.id).unwrap_or(&obj.id);`
     - `pointer: pointer_state_for(hovered, pressed, pointer_id)` puts **every** member of the group into `PointerState::Hovered` or `PointerState::Pressed` whenever `hovered == Some(g.id)` or `pressed == Some(g.id)`!
     - Each member resolves its own `StateDef` for the runtime state and pointer state (`hover`/`down`), falling back to `normal` if that member lacks that state (already how `resolve_state_overrides` works!).
   - After scroll-panel clipping/offsetting and drag-drop question passes finish (which read each member's own `obj.id`), remap `item.hit` for every member (and its `is_state_extra` items) of an interactive group `g` from `Hit::Object(member_id)` to **`Hit::Object(g.id.clone())`**!
2. **Painting & `resolve_pointer_fill` (`crates/render/src/paint.rs:1401,2442-2443,2625,2735`):**
   - Because every member carries `item.hit == Hit::Object(g.id)` and `highlight.hovered`/`highlight.pressed` carry `Some(Hit::Object(g.id))`, `is_hovered` and `is_pressed` evaluate to `true` simultaneously on **every member** in `paint_items`, feeding `resolve_pointer_fill` for every shape/button member with zero per-member hacks!
3. **Hit Testing, Hover Stability & Cursor (`crates/render/src/view.rs`):**
   - `hit_test(items, slide_pos)` hitting any member (or overhanging state extra) returns `Some(Hit::Object(g.id))`.
   - Moving the pointer from member A (rectangle) to member B (icon) to member C (text) keeps `hit == Some(Hit::Object(g.id)) == self.hovered` → **zero spurious `HoverEnd`/`HoverStart`**!
   - Pressing on member A and releasing on member C keeps `hit == Some(pressed_hit)` → emits **one** `Input::Click { object: g.id }`!
   - `is_object_clickable(player, id)` (`view.rs:1437`) returns `true` when `slide.interactive_group(id)` is present (and not base-prevented), giving the pointing-hand cursor over every member when enabled and default cursor when `disabled`.
4. **Keyboard Focus & Group Focus Ring (`crates/render/src/focus.rs`):**
   - `build_list` (`:398`) and `first_target_on_layer` (`:586`) recognize `slide.interactive_group(&id)` alongside `slide.object(&id)`, using `group.focus_order`.
   - Scroll-panel auto-scroll (`:1081`) checks the first member's `panel` when focused on a group.
   - Keyboard hover (`:1220`) applies to interactive groups just like buttons, so tabbing onto the group puts all members into Hover state.
   - `activate` (`:1885`) already falls through to `inputs.push(Input::Click { object: id })`, activating the group on `Enter`/`Space`.
   - **Focus ring (`:2198-2227`, `target_rect:2321`):** When `player.is_interactive_group(id)`, compute the **union bounding rect** (`Rect::union`) of `crate::compose::item_bounds(item)` across all primary (`!item.is_state_extra`) `DrawItem`s with `item.hit == Hit::Object(id)` and draw `draw_ring` around the whole group bounds!
5. **Offscreen Print Exclusion (`crates/render/src/offscreen.rs:745`):**
   - Ensure `Action::PrintSlide` excluding the clicked `group.id` excludes all items with `item.hit == Hit::Object(group_id)`.

### 4.4 Accessibility & Preview-vs-Published Parity
- **`crates/render/src/access.rs`:**
  - `build_access_tree` emits **one** `AccessNode` per interactive group:
    - `key`: `format!("group:{}:{}", slide.id, g.id)`
    - `role`: `AccessRole::Button`
    - `label`: first non-empty of: (1) `g.description` (with `%var%` substitution), (2) `g.name` (with `%var%`), (3) joined non-empty text/alt of non-decorative members in reading order (with `%var%`, e.g. `"BACK"`), (4) `g.id`.
    - `checked`: `Some(player.object_state(&g.id) == Some("selected"))` when any member defines `"selected"`, else `None`.
    - `target`: `Some(FocusTarget::Object(g.id.clone()))`.
    - `rect`: union bounding rect of the group's visible primary `DrawItem`s.
  - Suppresses standalone `content_node`s for individual members of an interactive group (preventing screen readers from reading duplicate stray text/icon nodes outside the button).
- **Published Web/SCORM Player (`apps/coursecraft-player/src/a11y.rs` & `app.rs`):**
  - `a11y.rs` renders every `AccessRole::Button` node as an HTML `<button>` overlay at `node.rect` and dispatches `ExternalFocus::Focused`/`Activated(target)` directly into `FocusManager` — zero changes needed to DOM event routing, and 100% behavioral parity with Editor Preview!
  - Editor Preview (`crates/ui-egui/src/preview.rs`) and Web Player (`apps/coursecraft-player/src/app.rs`) both host `PlayerFrame` + `SlideView` + `Player` (zero duplicated runtime logic).

### 4.5 Editor Authoring, Engine Commands & Trigger UI
1. **New Engine Commands (`crates/engine/src/commands/object.rs` & `commands/mod.rs`):**
   - **`group.set_interactive`** (`UndoScope::SingleSlide`, `slide_validation: true`, `mutates: true`):
     - Params: `{"slide": id, "group": id, "interactive": bool, "name"?: string, "migrate_triggers"?: bool}`
     - Turning **on** (`interactive: true`):
       - Verifies the group has `>= 2` members on one layer and contains no disallowed interactive widgets or question controls.
       - If the legacy group id collides with an object/layer id on the slide, mints a fresh non-colliding group id and rewrites member `object.group` fields.
       - Inserts/updates `Group { id, name, interactive: true, .. }` in `slide.groups` / `layer.groups`, deriving default `name` from the first text/button member (or `"Button group"`) when omitted.
       - If `migrate_triggers` (default `true`): rewrites `Click` / `HoverStart` / `HoverEnd` triggers targeting any member of this group to target `group_id` (deduplicating identical triggers) in the **same undo step**.
       - Returns `{"group": group_id, "migrated_triggers": n}`.
     - Turning **off** (`interactive: false`):
       - Removes any triggers targeting `group_id` via `remove_dangling_triggers` (same pattern as `object.delete`) and removes the `Group` record so the slide returns to a byte-identical plain group.
       - Returns `{"group": group_id, "removed_triggers": n}`.
   - **`group.update`** (`UndoScope::SingleSlide`, `slide_validation: true`, `mutates: true`):
     - Params: `{"slide": id, "group": id, "name"?: string, "initial_state"?: string, "focus_order"?: u32|null, "description"?: string|null}`
     - Updates metadata on an existing `Group` record in one undo step.
2. **Existing Command Lifecycle Safety:**
   - `object.group`: generates `new_group` avoiding both `all_group_ids(slide)` AND `slide.local_ids()`; if merging an existing interactive group into a larger group, cleans up the superseded `Group` record and dangling triggers in the same undo step.
   - `object.ungroup`: removes matching `Group` records from `slide.groups` / `layer.groups` and removes dangling triggers targeting ungrouped interactive group IDs in the same undo step.
   - `object.delete`: if deleting an object leaves a `Group` record with `0` members, removes the `Group` record and its dangling triggers in the same undo step.
   - `object.duplicate` / `object.paste`: when a complete interactive group is duplicated/pasted with a remapped `new_group` id, clones its `Group` record under `new_group`.
3. **Editor UI (`crates/ui-egui`):**
   - **Arrange menu & Canvas context menu (`menu.rs`):** When `selection.is_one_group(&course)`, show `"Make button from group"` (or `"Remove button behaviour"` when already interactive), dispatching `group.set_interactive`.
   - **Group Inspector (`properties.rs:3241-3282`):** When `is_one_group`:
     - Show checkbox `"Act as one button"` (positive wording, `UX-11`) with tooltip `"Share hover, down, selected, visited, and disabled states across all objects in this group"`.
     - When `interactive == true`, show **Name**, **Initial state** combo (Normal / Hidden / Disabled + union of member states), **Accessible description**, **Focus order**, and an **Add trigger…** button targeting the group id.
   - **Timeline folder row (`timeline.rs:2233`):** Display `group.name` (when non-empty) and a quiet `"Button"` pill / icon indicator on interactive group folder rows.
   - **Triggers panel & Trigger Editor (`triggers.rs`):**
     - `object_name` and `object_label` check `slide.interactive_group(id)` and format `"<Name> (button group)"` (with `(layer name)` on layers).
     - Object target pickers for `Click`, `HoverStart`, `HoverEnd`, `StateChanges`, `KeyPressed`, `ChangeState`, and `ObjectState` include all interactive groups on the slide.
     - Opening **Add trigger** while an interactive group is selected defaults `with_selected_object` to the group's `id`.
     - Trigger validation/problem badges treat interactive group IDs as valid targets.

---

## 5. Watchlist of Risky Spots (`RISK-01`–`RISK-10`)

| ID | Area / File | Risk & Concrete Mitigation |
|---|---|---|
| **RISK-01** | `crates/engine/src/tests.rs:12246` (`r5_01_registry_golden_lock`) & `docs/cli.md` | Adding `group.set_interactive` / `group.update` (or changing any `REGISTRY` `params` string) breaks `r5_01_registry_golden_lock` and the CLI `docs/cli.md` sync test. **Mitigation:** Update both in the exact step (`S2`/`S8`) that modifies `REGISTRY` and verify with `cargo test -p coursecraft-engine r5_01` and `cargo test -p coursecraft-cli`. |
| **RISK-02** | `crates/engine/src/delta.rs:181-393` | `SlideDelta` destructures `Slide` exhaustively. Forgetting `groups` causes compile errors or broken undo for `group.set_interactive` / `group.update`. **Mitigation:** Wire `groups` into `SlideFieldPatch`, `SlideDelta::diff`, `apply`, `estimated_bytes`, and `FullSlide` in `S4`, tested by `m27_all_commands_undo_tests`. |
| **RISK-03** | Font byte duplication & WASM bloat (`crates/media` vs `crates/render`) | Including `Inter-*.ttf` twice would add ~230 KB to desktop & WASM builds. **Mitigation:** Keep a single copy in `coursecraft-media` (`text_metrics::INTER_*`) and import it from `crates/render/src/font.rs`. Confirm `cargo xtask layers` and `cargo xtask wasm` stay green. |
| **RISK-04** | `skrifa` vs `egui` text width mismatch on point text | If `skrifa` under-measures by even 1 px vs `egui`, a label could clip or wrap if later converted. **Mitigation:** Add explicit `POINT_TEXT_PAD_X = 4.0` per side (`8.0` total), write cross-crate parity test `PT-02` asserting `skrifa` box >= `egui` galley across strings/styles/sizes, and pass optional `measured_size` from canvas inline edit for custom/system fonts. |
| **RISK-05** | `inline_edit_layout_job` wrapping `wrap: false` text (`crates/render/src/text_layout.rs:1821`) | Today `inline_edit_layout_job` always wraps multiline `TextEdit` at `wrap_width`, ignoring `style.wrap == false`. **Mitigation:** Fix in `S3` by plumbing `wrap: bool` so `!wrap` sets `max_width = f32::INFINITY`. |
| **RISK-06** | Scroll-panel clipping broken if `item.hit` is remapped too early (`crates/render/src/compose.rs:2995`) | `compose_with_pointer`'s scroll-panel pass looks up `slide.object(id)` via `item.hit`. Remapping `item.hit` to `group_id` before that pass would break scrolling interactive groups. **Mitigation:** Remap `Hit::Object(member_id)` → `Hit::Object(group_id)` at the **very end** of `compose_with_pointer`, after scroll-panel and drag-drop passes. |
| **RISK-07** | `slide.object(id)` assumptions in `focus.rs`, `access.rs`, `view.rs` | `focus.rs:399,589,1081,1220,2200`, `access.rs:180,1125`, and `view.rs:1445` assume every focusable/clickable `id` is a `Slide::object`. Missing even one silently drops the group from Tab order, focus ring, cursor, or screen reader. **Mitigation:** Audit every `slide.object(` call listed in `S6`–`S7` and lock with unit tests `GB-06`–`GB-11`. |
| **RISK-08** | `suspend_data` backward compatibility (`crates/logic/src/suspend.rs:372-545`) | Changing section 3 encoding could break resume of existing LMS learners. **Mitigation:** Append interactive group state indices strictly *after* `all_objects.len()`, so courses with 0 interactive groups emit byte-identical `suspend_data`, proven by golden test `GB-05`. |
| **RISK-09** | `object.set_frame` accidentally converting point text on move/align/distribute | Moving a point-text box calls `object.set_frame` with unchanged `w`/`h`. **Mitigation:** Only clear `point_text` (and set `wrap = true`) when `(frame.w - obj.frame.w).abs() > 0.5`; pure translation preserves `point_text = true` (`PT-05`). |
| **RISK-10** | M28 UI/UX rules on new Group inspector & menu controls | Hardcoded colours, trailing colons, negative wording, or cramped inspector rows fail `uiux_sweep_tests.rs`. **Mitigation:** Use `inspector_row`, palette tokens only, positive sentence-case labels (`"Act as one button"`, `"Make button from group"`), and sweep in `S10`. |

---

## 6. Concrete Test Plan

### 6.1 Issue #7 — Point Text Tests (`PT-01`–`PT-10`)

| ID | Crate / File | Category | Behaviour Tested & Expected Result |
|---|---|---|---|
| **PT-01** | `coursecraft-course` (`tests.rs`) | Backward compat & golden | Old course JSON without `point_text` loads with `point_text == false` and re-serializes **byte-identically** (golden hash match on `examples/hello-course/course.json` and `examples/showcase/course.json`). A `Text` object with `point_text: true` round-trips cleanly. |
| **PT-02** | `coursecraft-render` (`tests.rs`) | Measurement parity | Cross-crate parity: `coursecraft_media::text_metrics::measure_unwrapped_text` matches `coursecraft_render::layout_text_with_wrap(..., wrap = false)` across `"Text"`, `"Next"`, `"Back"`, `"Submit Answer"`, bold/italic, sizes `12/20/36/72`, letter spacing, superscript, and multi-line (`\n`) — measured box always fits galley without clipping and stays tight (`<= galley + 16 px`). |
| **PT-03** | `coursecraft-engine` (`tests.rs`) | Engine headless / CLI / MCP | `object.add` with `point_text: true` (no `measured_size`) forces `wrap: false`, `autofit: None`, and fits `frame.w`/`frame.h` to `"Next"` (much smaller than `300×60`); `text.set` from `"Next"` → `"Continue to Module 2"` widens `frame.w` and preserves `frame.h` (single line); shrinking back to `"Go"` narrows `frame.w`; `Align::Center` keeps centre X fixed; `text.style_range` increasing font size grows `frame`. |
| **PT-04** | `coursecraft-engine` (`tests.rs`) | Conversion rules | On a `point_text: true` object: (a) `text.set_paragraph { wrap: true }` sets `point_text = false, wrap = true` keeping current width; (b) `object.set_frame` changing `w` sets `point_text = false, wrap = true`; (c) `object.set_autofit { autofit: "shrink" }` sets `point_text = false`. |
| **PT-05** | `coursecraft-engine` (`tests.rs`) | Move / Align preservation | Moving a point-text box (`object.set_frame` changing only `x`/`y`), `object.align` (centring two point-text labels `"Next"` and `"Back"` on a button), and `object.distribute` all keep `point_text == true` and `wrap == false`, and centre accurately without manual resize. |
| **PT-06** | `coursecraft-engine` (`tests.rs`) | Container / State extra | `object.state_add_extra` and `object.state_update_extra` with `StateExtraKind::Text { point_text: true, .. }` auto-fit `extra.frame` on insert and text update, and convert to area text on wrap-on or width resize. |
| **PT-07** | `coursecraft-engine` (`m27_all_commands_undo_tests.rs`) | Undo | Insert point text = 1 undo step; `text.set` (updating text + refitted frame) = 1 undo step (undo restores both old text AND old frame); width resize converting to area text = 1 undo step (undo restores `point_text = true, wrap = false` and old width). |
| **PT-08** | `coursecraft-ui-egui` (`canvas.rs` tests) | Click vs Drag UX | Canvas click with `PlacementTool::TextBox` creates `point_text: true, wrap: false` with fitted box and opens inline edit; typing `"Next"` grows/shrinks live inline rect on one line and commits fitted frame (`w < 120`, `h < 45`); canvas drag with `PlacementTool::TextBox` creates `point_text: false, wrap: true` keeping exact dragged width and wrapping. |
| **PT-09** | `coursecraft-ui-egui` (`canvas.rs` tests) | State Editor container click | Inside `editing_state`, clicking with `PlacementTool::TextBox` places a fitted `point_text: true` state extra; dragging places a fixed-width wrapping state extra. |
| **PT-10** | `coursecraft-ui-egui` (`properties.rs` / `canvas.rs`) | Inspector Wrap & Width handle | Checking `"Wrap text"` in inspector or dragging the East resize handle on a point-text object converts it to area text (`point_text == false, wrap == true`) in 1 undo step; `Ctrl+Z` restores point text. |

### 6.2 Issue #8 Part B — Interactive Group Tests (`GB-01`–`GB-14`)

| ID | Crate / File | Category | Behaviour Tested & Expected Result |
|---|---|---|---|
| **GB-01** | `coursecraft-course` (`tests.rs`) | Backward compat & golden | Courses without `groups` load identically and re-serialize **byte-identically** (`examples/hello-course`, `examples/showcase`). Plain groups (`Object.group` without `Group` record) serialize byte-identically and behave unchanged. |
| **GB-02** | `coursecraft-course` (`tests.rs`) | Validation | Rejects: colliding group/object/layer IDs; 0-member `Group`; interactive group on master slide; interactive group containing `TextEntry`/`Slider`/`ScrollPanel`/`Web`/drag item; trigger targeting a *plain* (non-interactive) group ID; invalid `initial_state` or unknown state in `ChangeState` on interactive group. Accepts valid triggers (`Click`, `HoverStart`, `HoverEnd`, `StateChanges`, `KeyPressed`, `ChangeState`, `ObjectState`) targeting an interactive group ID. |
| **GB-03** | `coursecraft-logic` (`tests.rs`) | Runtime states & triggers | Interactive group of 3 objects (rect + icon + text): `Input::Click` on group id OR any member id fires `Event::Click { object: group_id }` **exactly once**; built-in `selected` toggles normal ⇄ selected across all members (if any member defines `selected`); built-in `visited` sets all members to `visited`; `Action::ChangeState { object: group_id, state: "disabled" }` disables all members and swallows subsequent clicks silently; `hidden` hides all members; `Condition::ObjectState` matches for group id and member ids; `HoverStart`/`HoverEnd` fire once for the group. |
| **GB-04** | `coursecraft-logic` (`tests.rs`) | Keyboard order & focus | `Player::keyboard_objects()` returns `group_id` **once** and omits all individual member IDs; respects `disabled`, `hidden`, `prevent_base_clicks`, `hide_base_objects`, and `scroll_panel` viewport; `Input::FocusChanged { object: Some(group_id) }` enables `Event::KeyPressed { object: Some(group_id) }`. |
| **GB-05** | `coursecraft-logic` (`suspend.rs` tests) | SCORM `suspend_data` parity | Courses with 0 interactive groups encode **byte-identical** `suspend_data` to pre-M37 golden bytes. Courses with interactive groups persist modified group states (`selected`, `visited`, `disabled`, custom) through `encode` → `decode` → `Player::resume` and restore all members to that state. |
| **GB-06** | `coursecraft-render` (`tests.rs` / `u19_state_hover_tests.rs`) | Hover/Down rendering & `resolve_pointer_fill` | Grouped button (rect with `hover`/`down` fill, icon with `hover` fill, text with `hover` colour, plus optional 4th member with no `hover` state): hovering **any** member renders `hover` on all members that define it (4th member stays `normal`); pressing **any** member renders `down` via `resolve_pointer_fill` on all members; pixel-verified via offscreen render! Disabled group ignores hover/down on every member. |
| **GB-07** | `coursecraft-render` (`view.rs` tests) | Hit testing & cursor | Pointer moving from member A → member B → member C inside the same interactive group emits exactly ONE `HoverStart { group_id }` and ZERO intermediate `HoverEnd`/`HoverStart`; pressing on member A and releasing on member C emits ONE `Click { group_id }`; `hover_cursor_icon` returns `PointingHand` over every member (and `None` when disabled). |
| **GB-08** | `coursecraft-render` (`focus/tests.rs`) | Keyboard Tab, Enter/Space & Group Focus Ring | `Tab` visits the interactive group **once** (never lands on individual members); `Enter` and `Space` each emit `Input::Click { object: group_id }` once; `FocusManager::paint` draws `last_ring` surrounding the **union bounding rect** of all group members (verified against individual member bounds). |
| **GB-09** | `coursecraft-render` (`access/tests.rs`) | Accessibility tree | `build_access_tree` emits **one** `AccessNode` (`AccessRole::Button`, `label` resolved from description → name → joined member text, `checked` when `selected` state exists, `rect` = union group bounds) and emits **zero** duplicate `content_node`s for the group's members. |
| **GB-10** | `coursecraft-render` + `coursecraft-ui-egui` | Preview vs Published parity | Run the exact same interactive-group course through Editor `Preview` (`crates/ui-egui/src/preview.rs`) and `PlayerFrame` / `snapshot_player` (published player path): verify identical hover/down/selected/disabled pixels, identical effect traces (`simulate` parity), identical Tab stop count, and identical `AccessTree` nodes. |
| **GB-11** | `coursecraft-player` (`tests.rs`) | Web player integration | `coursecraft-player` `App` + `PlayerFrame` test confirms hover/press/click/Tab/Enter/Space and `build_access_tree` on a course with an interactive group produce identical effects and state transitions to Preview (plus `cargo xtask build-player` WASM check). |
| **GB-12** | `coursecraft-engine` (`tests.rs` & `m27_all_commands_undo_tests.rs`) | Engine commands & undo | `group.set_interactive` (on/off, trigger migration, collision-safe id minting, rejecting disallowed member kinds), `group.update` (name, description, initial_state, focus_order), `object.ungroup` cleanup, `object.delete` cleanup, `object.duplicate`/`paste` cloning `Group`, all single-step undo/redo verified + `panic_hunt` hostile params + `r5_01_registry_golden_lock` + `coursecraft-cli` `docs/cli.md` sync. |
| **GB-13** | `coursecraft-ui-egui` (`tests.rs`) | Authoring UI end-to-end | Headless `TestHarness`: group 3 objects → click `"Act as one button"` (or Arrange → `"Make button from group"`) → edit group name & description → timeline folder row shows group name & button indicator → open Trigger Editor (pre-selected to group id) → add click trigger → trigger list shows group name cleanly without warnings → undo reverses each edit in 1 step. |
| **GB-14** | `coursecraft-ui-egui` (`uiux_sweep_tests.rs`) | M28 UI/UX sweep | Full sweep across `1024×600`, `1280×800`, `1920×1080` × Dark/Light × scale `1.0`/`1.5` with an interactive group selected in Format inspector and a point-text box selected: **0 MUST violations** (`UX-01`–`UX-19`). |

---

## 7. Numbered Implementer Steps (10 Steps, ~30–45 min each)

> **Gate cadence:** Run targeted `cargo test -p <crate> -j 4 <filter>` after every step. Run full `cargo xtask ci` after **S3, S5, S7, S9, and S10** (plus `cargo xtask build-player` and release perf tests at **S10**).

- **S1 — Issue #7: Model `point_text`, Spec Update & Headless/WASM `skrifa` Text Metrics**
  - **Files:** `crates/course/src/model.rs`, `crates/course/src/tests.rs`, `docs/spec/course-model.md`, `crates/media/fonts/` (move/share `Inter-{Regular,Bold,Italic,BoldItalic}.ttf` from `crates/render/fonts/`), `crates/media/src/text_metrics.rs` (new), `crates/media/src/lib.rs`, `crates/render/src/font.rs`, `crates/render/src/tests.rs`, plus any exhaustive `ObjectKind::Text` / `StateExtraKind::Text` pattern matches (`crates/engine/src/delta.rs`, etc.).
  - **Exact changes:**
    1. Add `#[serde(default, skip_serializing_if = "is_false")] pub point_text: bool` to `ObjectKind::Text` (`crates/course/src/model.rs:2140`) and `StateExtraKind::Text` (`:4058`). Update all struct-literal constructions to set `point_text: false`.
    2. Update `docs/spec/course-model.md` under `## Objects` (`text` row + short point-text note) documenting `point_text?` (default `false`, omitted when `false`), `wrap: false`, and conversion to area text on wrap-on or width resize.
    3. Create `crates/media/src/text_metrics.rs` exporting `INTER_REGULAR`, `INTER_BOLD`, `INTER_ITALIC`, `INTER_BOLD_ITALIC`, `POINT_TEXT_PAD_X`, `POINT_TEXT_PAD_Y`, `POINT_TEXT_MIN_W`, `MeasuredRun`, `ParagraphMetrics`, and `measure_unwrapped_text(...) -> (f32, f32)` using `skrifa::FontRef` + `Metrics` + `GlyphMetrics` + `charmap` (zero `unwrap`/`panic!`, finite/clamped outputs, WASM-safe).
    4. Point `crates/render/src/font.rs:65-70` (and `crates/media/src/tests.rs:14-16`) at `coursecraft_media::text_metrics::INTER_*` so zero font bytes are duplicated.
  - **Tests:** `PT-01`, `PT-02`; run `cargo test -p coursecraft-course -j 4`, `cargo test -p coursecraft-media -j 4 text_metrics`, and `cargo test -p coursecraft-render -j 4 pt_02`.
  - **Done when:** Existing courses deserialize and re-serialize byte-identically, `skrifa` measurement passes cross-crate parity with `egui` `layout_text_with_wrap`, and zero font bytes are duplicated.

- **S2 — Issue #7: Engine Auto-Fit & Area-Text Conversion Commands**
  - **Files:** `crates/engine/Cargo.toml`, `crates/engine/src/commands/text.rs`, `crates/engine/src/commands/object.rs`, `crates/engine/src/commands/object_states.rs`, `crates/engine/src/commands/mod.rs` (only if `params` docs change), `crates/engine/src/tests.rs`, `crates/engine/src/m27_all_commands_undo_tests.rs`, `docs/cli.md` (only if `REGISTRY` changed).
  - **Exact changes:**
    1. Add `coursecraft-media.workspace = true` to `crates/engine/Cargo.toml` (L5 → L2, no `egui`, WASM-safe).
    2. Implement `pub(super) fn fit_point_text_frame(frame: &mut Rect, text: &str, runs: &[TextRun], style: &TextStyle, paragraphs: &[Paragraph], measured_override: Option<(f32, f32)>)` in `crates/engine/src/commands/text.rs` with alignment-preserving anchoring and vertical direction swap.
    3. Wire `fit_point_text_frame` and point-text invariants into:
       - `object_add` (`object.rs:18`): normalize `style.wrap = false`, `autofit = Autofit::None` when `point_text == true`, fit `frame.w`/`frame.h` (accepting optional `measured_size: {w, h}`).
       - `object_state_add_extra` & `object_state_update_extra` (`object_states.rs:525,560`): same for `StateExtraKind::Text`.
       - `text_set` (`text.rs:113`): refit `obj.frame` when `point_text == true` (accepting optional `measured_size`).
       - `text_style_range` (`text.rs:155`), `text_set_paragraphs` (`:297`), `object_update` (`object.rs:143`), `object_apply_style` (`:1240`): refit when `point_text == true`; if `object_update` sets `wrap: true`, clear `point_text = false`.
       - `text_set_paragraph` (`text.rs:229`): if `wrap == Some(true)` clear `point_text = false`; else refit if `point_text == true`.
       - `object_set_autofit` (`text.rs:416`): if `autofit == Autofit::Shrink` clear `point_text = false`.
       - `object_set_frame` (`object.rs:963`): if `point_text == true` and `(frame.w - obj.frame.w).abs() > 0.5`, set `point_text = false` and `style.wrap = true`; keep `point_text = true` on pure `(x, y)` moves.
  - **Tests:** `PT-03`, `PT-04`, `PT-05`, `PT-06`, `PT-07`; run `cargo test -p coursecraft-engine -j 4 pt_0` and `cargo test -p coursecraft-engine -j 4 m27_all_commands_undo`.
  - **Done when:** Headless CLI/MCP inserts and edits auto-fit point text, conversions on wrap/width/shrink work in 1 undo step, and move/align/distribute keep `point_text = true`.

- **S3 — Issue #7: Canvas Click vs Drag Placement, State Extra Placement & Live Inline Growth**
  - **Files:** `crates/render/src/text_layout.rs`, `crates/ui-egui/src/canvas.rs`, `crates/ui-egui/src/properties.rs`.
  - **Exact changes:**
    1. In `crates/render/src/text_layout.rs:1799`, add `wrap: bool` support (`inline_edit_layout_job_with_wrap` or update `inline_edit_layout_job` callers) so `!wrap` sets `TextWrapping.max_width = f32::INFINITY` while keeping `break_on_newline = multiline`.
    2. In `crates/ui-egui/src/canvas.rs`:
       - Pass `is_click: bool` to `place_object(editor, &tool, frame, is_click)` from `:3342` (`true`) and `:3344` (`false`).
       - On `PlacementTool::TextBox` + `is_click`: emit `"point_text": true` and `"wrap": false` for both slide placement (`object.add`) and State Editor container placement (`:2379` → `object.state_add_extra`), anchoring the click origin cleanly inside the slide.
       - On `PlacementTool::TextBox` + `!is_click` (drag): keep existing fixed-width wrapping area text unchanged.
       - In `draw_inline_edit` (`:5927-6250`): for `point_text == true` objects/extras, compute live fitted `slide_rect` from `edit.buffer` via `layout_text_with_wrap(..., wrap = false)` + padding + insets so the box grows/shrinks live as the author types, and pass `"measured_size"` on `commit_inline_edit`.
    3. In `crates/ui-egui/src/properties.rs:4884`, add the calm tooltip on the `"Wrap"` row when `point_text == true`.
  - **Tests:** `PT-08`, `PT-09`, `PT-10`; run `cargo test -p coursecraft-ui-egui -j 4 pt_0` and **Gate #1:** `cargo xtask ci`.
  - **Done when:** Clicking places live-growing one-line point text (on stage and in State Editor), dragging places wrapping area text, wrap-on / width-handle drag converts to area text, undo works in 1 step each, and `cargo xtask ci` is green.

- **S4 — Issue #8 Part B: `Group` Model, Validation, Spec & Undo Delta**
  - **Files:** `crates/course/src/model.rs`, `crates/course/src/lib.rs`, `crates/course/src/validate.rs`, `crates/course/src/tests.rs`, `crates/engine/src/delta.rs`, `docs/spec/course-model.md`, plus exhaustive `Slide`/`Layer` literals in tests/importers.
  - **Exact changes:**
    1. Define `pub struct Group { id, name, interactive, initial_state, focus_order, description }` in `crates/course/src/model.rs` and add `#[serde(default, skip_serializing_if = "Vec::is_empty")] pub groups: Vec<Group>` to `Slide` and `Layer` (re-export `Group` from `crates/course/src/lib.rs`).
    2. Add helpers on `Slide` in `crates/course/src/lib.rs`: `group(&self, id)`, `interactive_group(&self, id)`, `interactive_group_of_object(&self, obj_id)`, `group_members(&self, group_id)`, `group_has_state(&self, group_id, state)`.
    3. Update `crates/course/src/validate.rs`: validate `slide.groups` / `layer.groups` (ID uniqueness, disjoint from `local_ids()`, member presence, no interactive groups on masters, disallowed nested widget kinds, `initial_state`), and allow interactive group IDs in `Event::{Click, HoverStart, HoverEnd, StateChanges, KeyPressed}`, `Action::ChangeState`, and `Condition::ObjectState`.
    4. Wire `groups` into `SlideFieldPatch` and `SlideDelta` in `crates/engine/src/delta.rs:175-393`.
    5. Document `Slide.groups` / `Layer.groups` (`Group`) in `docs/spec/course-model.md`.
  - **Tests:** `GB-01`, `GB-02`; run `cargo test -p coursecraft-course -j 4` and `cargo test -p coursecraft-engine -j 4 delta`.
  - **Done when:** Backward-compatibility golden tests are byte-identical, all group & trigger validation rules pass, and `SlideDelta` round-trips `groups` cleanly.

- **S5 — Issue #8 Part B: `coursecraft-logic` Runtime Semantics + `docs/spec/triggers.md` (Same Step!)**
  - **Files:** `docs/spec/triggers.md`, `crates/logic/src/lib.rs`, `crates/logic/src/run.rs`, `crates/logic/src/suspend.rs`, `crates/logic/src/tests.rs`.
  - **Exact changes:**
    1. Add normative `## Interactive groups` section to `docs/spec/triggers.md` covering state sharing, pointer hover/down, member look fallback to `normal`, single-fire triggers, built-in `selected`/`visited` transitions, `disabled`/`hidden`, keyboard focus order, `suspend_data`, and backward compatibility (project rule: spec in same change!).
    2. In `crates/logic/src/lib.rs` and `run.rs`:
       - Expose `Player::interactive_group_of`, `Player::is_interactive_group`, `Player::group_members`.
       - Seed interactive group `initial_state` on slide/layer/lightbox entry and revisit reset.
       - Update `object_state` and `set_state` to resolve interactive group IDs and member IDs to the shared group state, emitting one `Effect::StateChanged` and one `Event::StateChanges`.
       - Update `click_check`, `click`, `Input::HoverStart`/`HoverEnd`, and `Input::FocusChanged` to resolve group and member IDs to the canonical group ID, firing triggers once and running group-level built-in `selected`/`visited` transitions.
       - Update `Player::keyboard_objects()` to emit each reachable interactive group ID once (at its first member's position) and omit all member IDs.
    3. In `crates/logic/src/suspend.rs`: seed interactive group initial states in `rebuild_initial_states`, and encode/decode modified interactive group states at indices `all_objects.len()..` so courses without interactive groups stay byte-identical.
  - **Tests:** `GB-03`, `GB-04`, `GB-05`; run `cargo test -p coursecraft-logic -j 4` and **Gate #2:** `cargo xtask ci`.
  - **Done when:** All runtime group state, trigger, focus-list, and `suspend_data` tests pass, `docs/spec/triggers.md` matches the implementation line-for-line, and `cargo xtask ci` is green.

- **S6 — Issue #8 Part B: Renderer Composition, Pointer Hover/Down, Cursor & Group Focus Ring (`coursecraft-render`)**
  - **Files:** `crates/render/src/compose.rs`, `crates/render/src/paint.rs` (verify + tests), `crates/render/src/view.rs`, `crates/render/src/focus.rs`, `crates/render/src/offscreen.rs`, `crates/render/src/u19_state_hover_tests.rs`, `crates/render/src/focus/tests.rs`.
  - **Exact changes:**
    1. In `crates/render/src/compose.rs`:
       - Resolve `pointer_id = player.interactive_group_of(&obj.id).unwrap_or(&obj.id)` when calling `pointer_state_for(hovered, pressed, pointer_id)` on base and layer objects.
       - At the end of `compose_with_pointer` (after scroll-panel and drag-drop passes, `RISK-06`), remap `Hit::Object(member_id)` → `Hit::Object(group_id)` for all items belonging to an interactive group.
    2. Verify `crates/render/src/paint.rs:2442-2443` + `resolve_pointer_fill` (`:1401`) applies `is_hovered` and `is_pressed` across every member item simultaneously (and skips when `disabled`).
    3. In `crates/render/src/view.rs:1437`, update `is_object_clickable` to return `true` for interactive groups (respecting `prevent_base_clicks`), so every member shows `CursorIcon::PointingHand` when enabled.
    4. In `crates/render/src/focus.rs`: update `build_list` (`:398`), `first_target_on_layer` (`:586`), scroll-into-view (`:1081`), keyboard hover (`:1220`), `paint` (`:2198`), and `target_rect` (`:2321`) for interactive groups — drawing the focus ring around the **union bounding rect** of all primary member items.
    5. In `crates/render/src/offscreen.rs:745`, support excluding interactive group items on `PrintSlide`.
  - **Tests:** `GB-06`, `GB-07`, `GB-08`; run `cargo test -p coursecraft-render -j 4`.
  - **Done when:** Hovering/pressing any member visually updates all members (verified via offscreen pixels), moving between members emits no duplicate hover events, Tab reaches the group once with a union-bounds focus ring, and Enter/Space activates it.

- **S7 — Issue #8 Part B: Accessibility Tree & Preview-vs-Published Web/SCORM Parity**
  - **Files:** `crates/render/src/access.rs`, `crates/render/src/access/tests.rs`, `apps/coursecraft-player/src/tests.rs`, `crates/ui-egui/src/u16_preview_tests.rs`.
  - **Exact changes:**
    1. In `crates/render/src/access.rs`:
       - Emit **one** `AccessNode` (`AccessRole::Button`, resolved accessible name `description → name → joined member text/alt → id`, `checked` if any member defines `"selected"`, `target: Some(FocusTarget::Object(g.id))`, `rect` = union group bounds) for each interactive group in `build_access_tree` (`:176-339`) and `target_node` (`:1124`).
       - Skip emitting standalone `content_node`s for individual members of an interactive group.
       - Update `object_label` / `object_rect` helpers in `access.rs` so group IDs resolve cleanly.
    2. Add Preview-vs-Published parity test (`GB-10`) comparing `Preview` (`crates/ui-egui/src/preview.rs`), `PlayerFrame` / `snapshot_player`, `AccessTree`, and `Player::handle` effects on an interactive-group course.
    3. Add web-player integration test (`GB-11`) in `apps/coursecraft-player/src/tests.rs`.
  - **Tests:** `GB-09`, `GB-10`, `GB-11`; run `cargo test -p coursecraft-render -j 4 access`, `cargo test -p coursecraft-player -j 4`, and **Gate #3:** `cargo xtask ci`.
  - **Done when:** Assistive technology sees one named button per interactive group with zero duplicate member nodes, Preview matches Published pixel-for-pixel and event-for-event, and `cargo xtask ci` is green.

- **S8 — Issue #8 Part B: Engine Commands (`group.set_interactive`, `group.update`) & Group Lifecycle**
  - **Files:** `crates/engine/src/commands/object.rs`, `crates/engine/src/commands/object_states.rs`, `crates/engine/src/commands/mod.rs`, `crates/engine/src/tests.rs`, `crates/engine/src/m27_all_commands_undo_tests.rs`, `docs/cli.md`.
  - **Exact changes:**
    1. Implement `group_set_interactive` and `group_update` in `crates/engine/src/commands/object.rs` per §4.5 (including member trigger migration on opt-in, collision-safe ID minting, disallowed-widget rejection, and `remove_dangling_triggers` cleanup on opt-out).
    2. Register both commands in `REGISTRY` (`crates/engine/src/commands/mod.rs`) with `UndoScope::SingleSlide`, `slide_validation: true`, `mutates: true`.
    3. Update `object_group` (avoid colliding with `slide.local_ids()`; clean superseded `Group` records on merge), `object_ungroup` (drop `Group` records & dangling triggers), `object_delete` (drop empty `Group` records & dangling triggers), and `object_duplicate`/`object_paste` (clone `Group` records under remapped group IDs).
    4. Update `r5_01_registry_golden_lock` in `crates/engine/src/tests.rs:12246` and append the 2 new command rows in `docs/cli.md`.
  - **Tests:** `GB-12`; run `cargo test -p coursecraft-engine -j 4`, `cargo test -p coursecraft-cli -j 4`, and `cargo test -p coursecraft-mcp -j 4`.
  - **Done when:** All new commands, lifecycle cleanups, single-step undo/redo, `panic_hunt`, `r5_01_registry_golden_lock`, and `docs/cli.md` sync pass.

- **S9 — Issue #8 Part B: Editor UI — Menus, Group Inspector, Timeline Badge & Trigger Editor**
  - **Files:** `crates/ui-egui/src/menu.rs`, `crates/ui-egui/src/properties.rs`, `crates/ui-egui/src/timeline.rs`, `crates/ui-egui/src/triggers.rs`, `crates/ui-egui/src/tests.rs`.
  - **Exact changes:**
    1. In `crates/ui-egui/src/menu.rs`: add `"Make button from group"` / `"Remove button behaviour"` in Arrange and canvas context menus when `selection.is_one_group(&course)`.
    2. In `crates/ui-egui/src/properties.rs:3241-3282`: add `"Act as one button"` checkbox (`group.set_interactive`); when interactive, show **Name**, **Initial state** combo, **Accessible description**, **Focus order** (`group.update`), and **Add trigger…** button.
    3. In `crates/ui-egui/src/timeline.rs:2233`: show `group.name` (when set) and a subtle button indicator on interactive group folder rows.
    4. In `crates/ui-egui/src/triggers.rs`: update `object_name`, `object_label`, target object pickers (for `Click`, `HoverStart`, `HoverEnd`, `StateChanges`, `KeyPressed`, `ChangeState`, `ObjectState`), `with_selected_object` (default to interactive group ID when one interactive group is selected), and broken-reference checks so group triggers display and validate cleanly.
  - **Tests:** `GB-13`; run `cargo test -p coursecraft-ui-egui -j 4 gb_13` and **Gate #4:** `cargo xtask ci`.
  - **Done when:** Authors can convert a group to a button, name/configure it, attach triggers to it, preview it, and undo every step cleanly, with `cargo xtask ci` green.

- **S10 — M28 UI/UX Sweep, Performance Verification, Report & Final Release Gate**
  - **Files:** `crates/ui-egui/src/snapshot.rs`, `crates/ui-egui/src/uiux_sweep_tests.rs`, `docs/testing/m37-report.md` (new implementer report).
  - **Exact changes:**
    1. Add an interactive-group button and a point-text object to the UI/UX sweep fixture (or dedicated snapshot case in `snapshot.rs` / `uiux_sweep_tests.rs`) and verify 0 MUST violations across `1024×600`, `1280×800`, `1920×1080` × Dark/Light × scale `1.0`/`1.5`. Fix any layout/contrast/copy findings surfaced.
    2. Run full automated release gate (§8.1), including `cargo xtask ci`, `cargo xtask build-player`, and release perf benchmarks.
    3. Write concise `docs/testing/m37-report.md` summarizing shipped behaviour, test counts, perf numbers, and any follow-ups.
  - **Tests:** `GB-14` + full release gate (§8.1).
  - **Done when:** All automated gates, M28 UI/UX sweep, WASM web player build, and release perf budgets pass with zero regressions.

---

## 8. Release Gate for M37

### 8.1 Automated Release Gate Commands
Run in order before tagging:
1. `cargo xtask ci` (`fmt --all --check`, `clippy --workspace --all-targets -- -D warnings`, `test --workspace`, `layers`, `wasm` across all 17 WASM crates)
2. `cargo xtask build-player` (verifies release WASM web player compiles cleanly with shared interactive-group runtime and accessibility tree)
3. `cargo test -p coursecraft-ui-egui -j 4 uiux_sweep` (0 MUST violations under M28 `UX-01`–`UX-22`)
4. `cargo test --release -p coursecraft-ui-egui perf_tests -- --ignored --nocapture` (verifies zero editor hot-path regression against `docs/performance.md` budgets)
5. `cargo test --release -p coursecraft-engine undo_perf_tests -- --ignored --nocapture --test-threads=1` (verifies `SlideDelta` with `Slide.groups` keeps undo memory and timing within budget)

### 8.2 Owner Verification Checklist (Mirrors Issue #7 & Issue #8 Part B Acceptance Criteria)

**Issue #7 — Point text vs area text:**
- [ ] **Click places fitted point text:** Select the Text box tool, click once on the stage, and type `"Next"`. Confirm the box is one line and only as wide as the text plus padding (no 300×60 excess box).
- [ ] **Center-align without manual resize:** Place two clicked labels (`"Next"` and `"Back"`) and center-align them onto buttons. Confirm they center accurately without resizing either box first.
- [ ] **Drag places fixed-width wrapping area text:** Drag with the Text box tool to draw a box and type a long sentence. Confirm the box keeps the dragged width and wraps cleanly.
- [ ] **Convert point text to area text:** On a clicked point-text box, turn **Wrap text** on (and in a second box, drag the side width handle). Confirm both convert to fixed-width wrapping area text.
- [ ] **State Editor placement:** Enter **Edit states**, click with the Text box tool to add a state-extra label. Confirm it places fitted point text; drag places wrapping area text.
- [ ] **Undo parity:** Undo typing (`Ctrl+Z` / `⌘Z`) restores `"Text"` and its original fitted width in one step; a second Undo deletes the text box in one step.
- [ ] **Existing courses unchanged:** Open `examples/showcase/course.json`, save without editing (or inspect diff), and confirm existing text boxes keep exact frames, wrapping, and byte-identical JSON.

**Issue #8 Part B — One state for a grouped button:**
- [ ] **Make a button from a group:** Group a rectangle, an icon, and a `"BACK"` text box (`Ctrl+G` / `⌘G`), give each member a `Hover` and `Down` look (e.g. dark fill + white icon + white text), and turn on **Act as one button** (or **Arrange ▸ Make button from group**).
- [ ] **Shared Hover & Down across all members:** Open **Preview**. Move the pointer over the rectangle, then onto the icon, then onto `"BACK"`. Confirm **all** members stay in `Hover` together without flicker, and pressing on any member puts **all** members in `Down`. Confirm a member without a `Hover` state stays `Normal`.
- [ ] **Shared Selected, Visited & Disabled:** Add a `Selected` (or `Visited`) state to members and test clicking, plus a trigger setting the group to `Disabled`. Confirm all members change together and clicking a disabled group does nothing.
- [ ] **Click trigger fires once:** Attach one click trigger to the group (e.g. increment a counter variable or jump slide). Click the rectangle, icon, or text — confirm the trigger fires **once**, not three times.
- [ ] **Keyboard focus & focus ring:** Press `Tab` in Preview. Confirm focus reaches the grouped button **once** (never individual members), the focus ring surrounds the whole group bounds, and `Enter` / `Space` activates it.
- [ ] **Published SCORM/Web parity:** Publish to Web or SCORM, open in a browser, and verify hover/down/click/Tab/Enter/Space and screen-reader button name behave identically to Preview.
- [ ] **Spec & CI:** Confirm `docs/spec/triggers.md` documents the interactive-group state rule, `docs/spec/course-model.md` documents `point_text` and `Group`, and `cargo xtask ci` passes.

---

## 9. Summary

M37 delivers both tester-requested authoring improvements cleanly across **10 focused, single-run steps (`S1`–`S10`)**:
- **Issue #7 (`S1`–`S3`):** Adds backward-compatible `point_text: bool` (`skip_serializing_if = "is_false"`) on `ObjectKind::Text` and `StateExtraKind::Text`, shares a single copy of Inter TTF metrics in `coursecraft-media::text_metrics` (`skrifa`, headless & WASM-safe, zero `egui`), auto-fits frames inside `coursecraft-engine` commands (preserving 1-step undo and CLI/MCP parity), fixes `inline_edit_layout_job` for `wrap: false`, and wires click vs drag placement (both stage and State Editor) with live growth while typing and clean conversion to area text on wrap-on or width resize.
- **Issue #8 Part B (`S4`–`S9`):** Adds backward-compatible `Group` records on `Slide`/`Layer` (`skip_serializing_if = "Vec::is_empty"`), puts 100% of play-time shared-state/trigger/focus/suspend semantics in `coursecraft-logic` alongside the normative `docs/spec/triggers.md` contract (`S5`), unifies rendering/hit-testing/`resolve_pointer_fill`/cursor/union-bounds focus ring/single-control accessibility across Preview and Published Web/SCORM via `coursecraft-render` (`S6`–`S7`), adds `group.set_interactive` and `group.update` commands with full lifecycle cleanup (`S8`), and exposes clean-room authoring controls across Menus, Group Inspector, Timeline, and Trigger Editor (`S9`).
- **Verification (`S10`):** Closes with M28 UI/UX sweep (`UX-01`–`UX-22`), `cargo xtask ci`, `cargo xtask build-player`, release editor & undo perf tests, and the owner acceptance checklist.

**Total steps:** 10 (`S1`–`S10`).
