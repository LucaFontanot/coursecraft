# M29 Rich Object States — Product, Architecture & Test Plan

**Role:** Product Specification, Architecture & QA Plan (planning only — zero source files edited)  
**Scope:** Owner-approved M29 three-slice scope from `docs/roadmap.md` ("M29 Rich object states (planned)"), building on `docs/testing/m28-noe-states-analysis.md` (Suggestions 1 & 3, Phase B). Where the roadmap and earlier analysis differ, **the roadmap wins**: the Format-panel quick fix was dropped from M28 and is superseded by Slice 1.  
**Preconditions assumed in tree:** M28 steps **u19** (2-layer `hover`/`down` presentational resolver across all visuals, `StateDef.src` path validation, Check Course missing-media warnings, `canvas.rs` placeholder order) and **u20** (`PendingImagePick::StateImage` file picker + `"From project…"` + `"Clear"`).  
**Clean-room commitment:** Capability match only. Visual language, layout, typography, icons (Phosphor MIT), and wording follow `DESIGN.md` ("The Quiet Studio") and `docs/contributing.md:21-40`.

> **Reviewer notes (Opus, 2026-11-22). These override the plan below.**
> 1. **Steps are split smaller.** Steps 2, 3, 8, 9 and 10 are each too big for one 30–40 minute step. Use this list (18 steps), keeping each step's tests and docs:
>    - S1 = Step 1.
>    - S2a: States tab, live thumbnails, empty and multi-select states, compact Format row, right-click Edit states…, preview_state theme and placeholder fix (U-01, U-02, U-03, U-06, PERF-01, SUS-03, SUS-14).
>    - S2b: card actions, inline rename, keyboard and accesskit, delete blocker (U-04, U-05, U-07, SUS-09).
>    - S3a: widened `StateDef`, validation, `object.patch_state` (E-01–E-04, E-10, E-14, FZ-01, FZ-02).
>    - S3b: media, font, string and XLIFF walkers for the widened fields (E-16, E-17, PUB-01–PUB-04, FZ-04, SUS-04–07, SUS-11).
>    - S4 to S7 as written.
>    - S8a: `StateExtra` schema, validation, the 4 commands (E-04 extras, E-11–E-14, FZ-03, FZ-06).
>    - S8b: extras in the walkers, copy/template, recolor, strings, XLIFF (E-15–E-18, PUB-01–PUB-03, SUS-10).
>    - S9a: extras composition, transforms, hit-testing, `is_target_label` (R-09–R-16, SUS-02).
>    - S9b: a11y label, multi-item canvas preview, web player prefetch, parity (P-02, P-05, P-06, PUB-04, FZ-05, SUS-01, SUS-03, SUS-06).
>    - S10a: insert, select, drag, resize, rotate, nudge, delete of extras (U-14, U-15).
>    - S10b: "Items in this state" list, inspector routing, inline text, context menu, thumbnails, undo safety, media save (U-16–U-19 for extras).
>    - S11 and S12 as written.
>    - Full gate after S2b, S3b, S5, S7, S8b, S9b, S10b and S12. Web player build after S4, S9b and S12.
> 2. **Existing files must keep opening.** Any new check that an older course could already break gives a **warning**, not an error: a state on a master object, a `hidden` StateDef, and more than 32 states or names over 64 characters. Engine commands still refuse to create these. Hard errors are only for the new fields (for example `Some` together with `clear_*`).
> 3. **No wall-clock budgets in tests.** PERF-02 asserts bounded work through the guard counters (copies, composes, repaints) in debug only. Timing numbers go in the S12 report and are not asserted.
> 4. **The owner's manual checklist uses only the app**, never cargo. Publish to Web and SCORM 1.2 from the Publish dialog and open the result in the built-in preview or browser. Use the real Preview shortcut from the menu, not "F12".
> 5. **Hover over an overhanging extra.** When a `hover` extra pokes outside the object, moving the pointer onto it must not flicker the hover off and on (P-08 must cover a pointer moving from the object onto its own overhanging hover extra over 10 frames).
> 6. Clean-room wording stays as listed in §2. No step adds dependencies.

> **Implementer Guardrails (`AGENTS.md`, `docs/contributing.md`)**
> 1. **Never crash:** Zero `unwrap`/`expect`/`panic!`/`unreachable!`/`todo!` in non-test code; `unsafe_code = "forbid"` everywhere except `coursecraft-video-native`. Input-derived numbers are hostile.
> 2. **Everything is a command:** Every state mutation dispatches an atomic, single-undo command through `coursecraft-engine` (`keep_valid`), is registered in `REGISTRY`, documented in `docs/cli.md` (enforced by `apps/coursecraft-cli/src/tests.rs:1209`), and covered in `crates/engine/src/m27_all_commands_undo_tests.rs:1453`.
> 3. **One runtime:** `coursecraft-logic` stays unchanged unless a runtime rule genuinely changes (none is needed for M29). Preview and published player share `coursecraft-render`.
> 4. **Spec first:** Any schema or rendering/precedence rule updates `docs/spec/course-model.md` and `docs/spec/triggers.md` in the same step.
> 5. **Layering & WASM:** `cargo xtask layers` and `cargo xtask wasm` (L0–L5) stay green; run `cargo xtask build-player` after every render/player change.
> 6. **Test first (red → green):** Each step writes its assigned failing tests first, then implements the minimal code to turn them green.

---

## 1. Product Definition by Slice

Each slice ends independently shippable with zero regressions to existing authoring, preview, or publish workflows.

### 1.1 Slice 1 — Bottom `States` Tab & State Management (Shippable)

#### Layout & Chrome
- Add `BottomTab::States` to `crates/ui-egui/src/timeline.rs:28-46, 429-441, 2351-2365` in the bottom panel tab strip: `Timeline | States | Triggers | Variables | Question`.
- **Master view:** `BottomTab::States` is omitted from the tab strip when `editor.ui.master_view == true` (matching `Triggers`, `Variables`, and `Question`, because `docs/spec/triggers.md:17` states master objects have no states). Entering Master view while `bottom_tab == BottomTab::States` resets `bottom_tab = BottomTab::Timeline`.
- **Tab header strip (`32 pt` height, `PANEL_MARGIN` horizontal padding):**
  - Left: selected object display name (truncated with full-name hover tooltip, `UX-19`), subtle state count caption (`"{N} states"` including `Normal`), and `Initial` combo (`normal`, `hidden`, plus every custom state in `obj.states`).
  - Middle: quick-add chips (`+ Hover`, `+ Down`, `+ Selected`, `+ Visited`, `+ Disabled`, each disabled with `"Already added"` tooltip once present, `>= 24 pt` tall per `UX-05`) and a `+ New state` button opening a compact popover/inline input (`state name` + `Add` button, `Enter` commits, `Esc` closes).
  - **Removed:** `"hidden"` is removed from `SUGGESTED_STATE_NAMES` (`crates/ui-egui/src/states.rs:14`) and is never offered as a quick-add chip (`docs/spec/course-model.md:584`: `hidden` is built-in and needs no definition). `"hidden"` remains selectable in the `Initial` combo and trigger editors.
  - Right: primary `"Edit state"` button (`>= 32 pt` tall, disabled when `Normal` is active or no single object is selected, with tooltip explaining why per `UX-08`). In Slice 1 this focuses the inline override strip for the selected state; in Slice 2 it enters Canvas Edit-State Mode.
- **Thumbnail card strip:**
  - Single horizontal `ScrollArea::horizontal()` (zero nested vertical-in-vertical scroll areas, `UX-02`) displaying live side-by-side cards: `Normal` pinned first, followed by `obj.states` in authored order.
  - Card geometry: `140×128 pt` outer card (`RADIUS_MD`, `p.surface` fill, `1 pt` `p.border` stroke; active/previewed card uses `2 pt` `p.accent` border and `p.selected` tint).
  - Live preview viewport inside each card: `120×72 pt` (`RADIUS_SM`).
  - Footer row inside each card: state name (single-line truncated with tooltip, double-click starts inline rename for custom states), small `"Initial"` pill badge if `obj.initial_state == name`, and a `24×24 pt` `⋯` menu icon (`UX-05`, `UX-10`, tooltip `"State actions"`) containing:
    - For `Normal`: `Set as initial` (disabled if already initial).
    - For custom states: `Edit state`, `Duplicate`, `Rename…`, `Set as initial`, `Move left` / `Move right` (disabled at ends), separator, `Delete` (disabled with blocker reason if initial or trigger/question-referenced).
- **Slice 1 inline override bar (below or beside the card strip when a custom state is selected):**
  - Keeps full parity with existing `states.rs` editing until Slice 2 lands: shows `Fill` (for Button/Shape/closed Path), `Text` (for Text/Button/text-bearing Shape), and the u20 image picker (`28×28` swatch + filename + `"Choose image…"` + `"From project…"` + `"Clear"`, reusing `PendingImagePick::StateImage`) for Image objects. Removed/replaced by full inspector routing once Slice 2 enters Edit-State Mode (quick preview clicks stay read-only unless `"Edit state"` is entered).

#### Live Thumbnail Rendering Contract
Every card paints live vectors and textures using the shared `coursecraft-render` pipeline (same pattern as slide thumbnails in `crates/ui-egui/src/slides.rs:1360-1396`):
1. **Viewport culling:** Paint only when `thumb_rect.intersects(ui.clip_rect())` so off-screen cards never pin textures in `editor.media_cache` or waste frame work.
2. **State composed:** `Normal` composes `"normal"`; each custom card composes `&state.name`. Compose with `compose_object_with_value(obj, state_name, Some(&course.theme), course_input_value(course, obj))` (or multi-item `compose_object_items` in Slice 3 so `extras` appear in the thumbnail too).
3. **Theme, background & fonts:**
   - Paint the current slide's effective background colour (`course.slide_background(slide)`) inside `thumb_rect`; if the object fill has alpha `< 255` or is an image/transparent shape, paint a subtle `6 pt` neutral checkerboard behind transparent areas.
   - Uses `Some(&course.theme)` so theme accent, default text colour, and theme font match canvas and player.
   - Fonts come directly from `ui.ctx()` (which already holds bundled Inter + loaded course custom fonts).
4. **Media:** Pass `Some((&mut editor.media_cache, &MemoryMediaSource { media: &editor.media }))` and run `missing_images_as_placeholders` on the composed item(s) *after* state composition so broken/unimported state `src` renders the neutral `"No image yet"` placeholder.
5. **Fitting & framing:** Compute the union bounding `Rect` of the composed item(s) (parent `item.rect` unioned with any Slice 3 `extras` rects, padded by `10%` of `max(w, h)` plus shadow/glow blur margin, clamped to finite positive `>= 1.0`). Translate all item rects by `-bounds.min`, build `SlideTransform::fit(thumb_rect, SlideSize { w: bounds.width(), h: bounds.height() })`, and call `coursecraft_render::paint(&ui.painter_at(thumb_rect), &t, &items, Highlight::default(), media, Some(&course.theme), None)`.
6. **Non-visual & special objects:**
   - `Audio`: paint a centred Phosphor waveform icon (`Icon::SpeakerHigh`) + truncated filename caption.
   - `initial_state == "hidden"`: `Normal` card still shows the object's normal look; the `Initial` combo displays `hidden`, and a muted `"Starts hidden"` indicator pill sits beside the `Initial` combo.

#### Selection Sync, Empty States, Multi-Select, Keyboard & Entry Points
- **Selection sync:**
  - Selecting an object on the canvas or timeline updates `BottomTab::States` immediately and resets `editor.ui.preview_state = None`.
  - Clicking a custom state card sets `editor.ui.preview_state = Some(state_name)` (canvas previews that state using `compose_object_with_value(..., Some(&course.theme), ...)`). Clicking `Normal` sets `editor.ui.preview_state = Some("normal")` when `obj.initial_state != "normal"` or `None` when `obj.initial_state == "normal"`.
  - Clicking the already-previewed custom card keeps it selected (toggling eye button or clicking `Normal` returns to normal).
  - Switching slide, switching layer, changing selection, entering Master view, or deleting the previewed state clears `editor.ui.preview_state`.
- **Empty states (`UX-17`):**
  - No object selected: centred secondary label `"Select an object on the slide to view and edit its states."` (no primary button).
  - Multi-select (`editor.selection.is_multi()`): centred secondary label `"Select a single object to edit its states ({N} selected)."` with all add/edit actions hidden/disabled (`UX-08`).
  - Object with only `Normal` (`obj.states.is_empty()`): shows the `Normal` live thumbnail card next to a dashed `"+ Add state"` prompt card + quick-add chips (`+ Hover`, `+ Down`, `+ Selected`, `+ Visited`, `+ Disabled`) so the next step is one click away.
- **State operations (all atomic, single-undo engine commands):**
  - **Add:** `object.add_state`. Name trimmed, non-empty, `!= "normal"`, `!= "hidden"` (UI rejects `"hidden"` with inline hint `"Hidden is built in — choose it in Initial or a trigger"`), `<= 64` chars, unique on object, `obj.states.len() < MAX_STATES_PER_OBJECT` (32). Newly added state becomes `preview_state`.
  - **Duplicate:** `object.duplicate_state { slide, object, name, new_name? }` → clones full `StateDef` (including Slice 2 overrides and Slice 3 `extras`), assigns `"<name> copy"`, `"<name> copy 2"`, … if `new_name` omitted, inserts immediately after the source state, returns `{"name": created_name}`.
  - **Rename:** `object.rename_state { slide, object, from, to }` → validates `to` (trimmed, non-empty, `!= "normal"`, `!= "hidden"`, `<= 64` chars, unique), renames `StateDef.name`, AND atomically rewrites every reference on the slide:
    1. `obj.initial_state` if `== from`,
    2. `slide.triggers` and every `layer.triggers`: `Event::StateChanges { object, to: Some(s) }`, `Action::ChangeState { object, state }`, and recursive `Condition::ObjectState { object, state, .. }`,
    3. `slide.question.correct_when` recursive `Condition::ObjectState { object, state, .. }`.
    Updates `preview_state` / `editing_state` to `to` if currently active.
  - **Delete:** `object.delete_state { slide, object, name }`. Blocked in UI (disabled button + tooltip + inline caption) AND rejected by `keep_valid` if `initial_state == name` or referenced by any trigger or `question.correct_when`.
  - **Set initial:** `object.update { slide, object, initial_state }` from header combo or card `⋯` menu.
  - **Reorder:** `object.reorder_state { slide, object, name, index }` moves the state in `obj.states` to `index` (`0..obj.states.len()`). `Normal` is always first and non-movable. Pure authoring order (zero play-time effect).
- **Keyboard & accessibility (`UX-05`, `UX-10`, `UX-13`):**
  - When focus is inside the card strip: `ArrowLeft`/`ArrowRight` move active card selection, `Enter` enters edit mode (Slice 2) / focuses overrides (Slice 1), `F2` starts inline rename on a custom state, `Delete`/`Backspace` deletes an unblocked custom state, `Esc` cancels inline rename or clears `preview_state`.
  - Every card exposes an `accesskit` button/tab node labelled `"State {name}"` (appending `", initial"` and `", active"` as applicable), with `>= 2 pt` focus ring (`>= 3.0:1` contrast).
- **Format panel compact replacement (`crates/ui-egui/src/properties.rs`):**
  - Remove BOTH the old `"State"` section (`properties.rs:8044-8074`) and the full `"States"` section (`properties.rs:2413-2415`).
  - Replace with a single compact `"States"` section (above Accessibility) containing two rows:
    1. `Initial`: the initial-state `styled_combo` (`normal`, `hidden`, + custom states).
    2. `States`: quiet link/button `"Open States tab ({N})"` (or `"Open States tab"` when `N == 0`), clicking switches `editor.ui.bottom_tab = BottomTab::States`.
  - In Master view: omitted (or disabled with `"Master objects don't have states"`).
- **Right-click context menu (`crates/ui-egui/src/menu.rs:1105-1217`):**
  - Add `"Edit states…"` in `show_object_row_menu` (automatically shared by canvas right-click and timeline row right-click).
  - Enabled iff single non-master object selected; disabled in Master view (`"Master objects don't have states"`) or multi-select (`"Select a single object"`).
  - Clicking switches `editor.ui.bottom_tab = BottomTab::States` (and in Slice 2, if a custom state is already previewed, opens it in edit-state mode; otherwise focuses the States tab).

---

### 1.2 Slice 2 — Widened `StateDef` Overrides & Canvas Edit-State Mode (Shippable)

#### Data Model & Exact Overridable Properties (`crates/course/src/model.rs`)
`Course.schema_version` stays `1`. `StateDef` retains `#[serde(deny_unknown_fields)]` and all 4 existing fields unchanged.

| JSON field on `StateDef` | Rust type | Applies to | Nullable tri-state clear flag | Default / serialize skip |
|---|---|---|---|---|
| `name` | `String` | All | — | Required |
| `fill` | `Option<Color>` | Button, Shape, closed Path, Text (colour) | — | `None` / omitted |
| `text` | `Option<String>` | Text, Button, text-bearing Shape | — | `None` / omitted |
| `src` | `Option<String>` | Image, Video | — | `None` / omitted |
| `fill_style` | `Option<FillStyle>` | Button, Shape, closed Path | `clear_fill_style: bool` | `None` / `false` omitted |
| `outline` | `Option<Outline>` | Text, Shape, Image, Path | `clear_outline: bool` | `None` / `false` omitted |
| `text_style` | `Option<TextStyle>` | Text, text-bearing Shape, Timer | — | `None` / omitted |
| `runs` | `Option<Vec<TextRun>>` | Text, text-bearing Shape | — | `None` / omitted |
| `paragraphs` | `Option<Vec<Paragraph>>` | Text, text-bearing Shape | — | `None` / omitted |
| `autofit` | `Option<Autofit>` | Text, text-bearing Shape | — | `None` / omitted |
| `opacity` | `Option<f32>` | All visual objects | — | `None` / omitted (clamped `0..=1` on deser) |
| `shadow` | `Option<Shadow>` | All visual objects | `clear_shadow: bool` | `None` / `false` omitted |
| `glow` | `Option<Glow>` | All visual objects | `clear_glow: bool` | `None` / `false` omitted |
| `crop` | `Option<CropRect>` | Image | `clear_crop: bool` | `None` / `false` omitted |
| `image_shape` | `Option<ShapeKind>` | Image | `clear_image_shape: bool` | `None` / `false` omitted |
| `rotation` | `Option<f32>` | All visual objects | — | `None` / omitted (normalised `0..360` on deser) |
| `flip_h` | `Option<bool>` | All visual objects | — | `None` / omitted |
| `flip_v` | `Option<bool>` | All visual objects | — | `None` / omitted |

#### Tri-State Semantics, Backwards & Loud Forward Compatibility
- **Not overridden (Inherit):** `field == None` (and `clear_field == false` for nullable properties). Omitted from JSON. Follows whatever Normal changes to later.
- **Overridden to same value:** `field == Some(v)` (even if `v` equals Normal's current value). Serialized in JSON. Stays pinned to `v` if Normal changes later.
- **Explicitly cleared (for nullable properties `fill_style`, `outline`, `shadow`, `glow`, `crop`, `image_shape`):** `field == None && clear_field == true`. Serialized as `"clear_<field>": true`. Explicitly removes Normal's property in this state (e.g., removing Normal's drop shadow in `disabled`).
- **Mutual-exclusion invariant:** `field.is_some() && clear_field == true` is a hard validation error (`validate.rs`) and impossible to create through `object.patch_state`.
- **Old 0.3.x files opened in M29:** All new fields have `#[serde(default)]` and `skip_serializing_if`, so every pre-M29 `course.json` loads identically and re-serializes **byte-for-byte unchanged** unless new state properties are set.
- **M29 files opened in 0.3.x:** Because 0.3.x `StateDef` carries `#[serde(deny_unknown_fields)]`, any file using any new M29 state field (e.g., `"glow"`, `"clear_shadow"`, `"extras"`) **fails loudly** in 0.3.x with a serde unknown-field error instead of silently dropping state appearance. Courses saved by M29 that do *not* use any new state fields omit them and remain readable in 0.3.x.

#### Layering Rule (`object → runtime_state → pointer_state`)
Resolved independently per property in `coursecraft-render` (`compose.rs`):
1. **Layer 0 (Base):** The `Object` itself (`normal`).
2. **Layer 1 (Runtime state):** `object.states.find(|s| s.name == runtime_state)` when `runtime_state != "normal"` and `runtime_state != "hidden"`.
3. **Layer 2 (Pointer presentational state):** `"down"` if pressed, else `"hover"` if hovered — **suppressed completely** when `runtime_state` is `"disabled"` or `"hidden"`, or when the object is non-interactive (master slide, hidden base under `prevent_base_clicks`).
4. **Resolution rule per property:**
   - For scalar/value fields (`opacity`, `rotation`, `flip_h`, `flip_v`, `text_style`, `autofit`, `src`): Layer 2 `Some` → else Layer 1 `Some` → else Layer 0.
   - For nullable fields (`outline`, `shadow`, `glow`, `crop`, `image_shape`): Layer 2 (`Some(v)` → `Some(v)`, `clear == true` → `None`) → else Layer 1 (`Some(v)` → `Some(v)`, `clear == true` → `None`) → else Layer 0.
   - For `fill` / `fill_style` interaction (preserving existing `compose.rs:1267, 1341` compatibility):
     - Within a layer, `fill_style: Some(fs)` wins over `fill`.
     - If a higher layer sets `fill: Some(c)` (with `fill_style: None` and `!clear_fill_style`), it sets solid fill `c` and suppresses any lower layer's `fill_style` (so a legacy `"hover": { "fill": "#FF0000" }` on a gradient button still turns solid red on hover!).
     - If a higher layer sets `clear_fill_style: true`, it clears any lower `fill_style` and uses that layer's `fill` (or lower layer's solid `fill`).
   - For text (`text`, `runs`, `paragraphs`, `text_style`):
     - Effective `text_style` resolves top-down (Layer 2 → Layer 1 → Layer 0). Note: legacy `Text` object `state.fill` still overrides `style.color` when `state.text_style` is `None`.
     - Effective text content (`text`, `runs`, `paragraphs`): the highest layer that sets `text.is_some() || runs.is_some()` supplies the content (`runs` if `Some`, else single-run plain `text` styled with effective `text_style`; `paragraphs` from highest layer setting `paragraphs.is_some()`, else Layer 0). If no layer overrides `text`/`runs`, Layer 0's `text`/`runs` render using the resolved `text_style` (so a hover state can change font weight/colour/size without duplicating text!).
   - Document this exact precedence table in `docs/spec/triggers.md` ("Built-in states after a click") and `docs/spec/course-model.md` ("States") in the same step.

#### Engine Command: `object.patch_state`
- **ID:** `"object.patch_state"`, `mutates: true`, label `"Edit state"`.
- **Params:**
  ```json
  {
    "slide": "s1",
    "object": "btn1",
    "name": "hover",
    "reset_all": false,
    "patch": {
      "fill": "#1E40AF",
      "outline": null,
      "clear_shadow": true,
      "glow": { "color": "#2F6FEB", "size": 12.0 },
      "opacity": 0.9
    }
  }
  ```
- **Patch semantics (strict, no unknown keys inside `patch`):**
  - Key **absent** from `patch`: untouched.
  - Key set to **`null`**: resets that property to **Inherit** (`field = None`, and for nullable properties also sets `clear_field = false`).
  - Key set to a concrete JSON value: sets `field = Some(val)` (and for nullable properties automatically sets `clear_field = false`).
  - `"clear_<field>": true` (for `fill_style`, `outline`, `shadow`, `glow`, `crop`, `image_shape`): sets `field = None, clear_field = true`. `"clear_<field>": false` clears the flag without setting a value. Providing BOTH `<field>: <non-null>` and `clear_<field>: true` in the same patch returns an `EngineError::Invalid` error.
  - `"reset_all": true` (optional bool, default `false`): resets every visual override and `clear_*` flag on the state back to Inherit (preserves `name` and Slice 3 `extras`) before applying any fields in `patch`.
- **Validation & undo:** Runs `keep_valid(s, was_valid)?` before returning; one atomic undo step; History panel displays `"Edit state \"<name>\""` (or `"Reset state \"<name>\""` when `reset_all: true`). `object.update_state` remains unchanged for full-struct replacement.

#### Canvas Edit-State Mode UX & Command Routing
- **State on `EditorUi`:**
  ```rust
  pub struct StateEditSession {
      pub slide_id: String,
      pub object_id: String,
      pub state_name: String,
      pub selected_extra: Option<String>, // used in Slice 3
  }
  pub editing_state: Option<StateEditSession>,
  ```
- **Entering:**
  - Click `"Edit state"` in `BottomTab::States` or double-click any non-Normal state thumbnail card (or right-click `"Edit states…"` when a custom state is already previewed).
  - Clicking another custom state card while already in `editing_state` seamlessly switches `editing_state.state_name` (and clears `selected_extra`). Clicking `Normal` exits `editing_state`.
  - Sets `editor.ui.inspector_tab = InspectorTab::Format`.
- **Canvas visual chrome:**
  - Non-blocking `36 pt` accent-tinted top bar across the canvas panel: Phosphor `Icon::SlidersHorizontal` + `"Editing state “{state_name}” of {object_label}"` + quiet `"Reset overrides"` button (disabled when state has no overrides, with tooltip `"No overrides to reset"`) + primary `"Done editing"` button (`>= 32 pt` tall, `UX-05`).
  - Non-target slide objects render dimmed (`dimmed = true`) and ignore pointer clicks, drags, hovers, and marquee selection.
  - Target object renders in its resolved state (`compose_object_with_value(obj, state_name, Some(&course.theme), ...)`), framed by a `1.5 pt` dashed accent border.
- **Exiting (`Esc` stack per `UX-13`):**
  - Click `"Done editing"`, switch slide/layer, enter Master view / Course map / Preview, or delete the edited state/object.
  - `Esc` closes strictly the topmost layer: open popup/combo/colour picker → inline text edit / crop / draw tool → `selected_extra` (Slice 3) → `editing_state` → object selection.
- **What routes to the state vs stays object-level (Decision & Justification):**
  - **Routed to `object.patch_state` (state-level):**
    - `fill`, `fill_style` / `clear_fill_style`, `outline` / `clear_outline`
    - `text`, `runs`, `paragraphs`, `text_style`, `autofit` (Format panel AND canvas inline text edit)
    - `src` (via u20's `"Choose image…"` / `"From project…"` / `"Clear"` with `PendingImagePick::StateImage`), `crop` / `clear_crop` (including canvas Crop mode!), `image_shape` / `clear_image_shape`
    - `opacity`, `shadow` / `clear_shadow`, `glow` / `clear_glow`
    - `rotation` (including canvas rotate handle!), `flip_h`, `flip_v`
  - **Locked object-level (disabled with explicit tooltip `"Shared across all states — exit state editing to change"` per `UX-08`):**
    - **Parent `frame` (`X`, `Y`, `W`, `H`) and canvas move/resize handles:** *Justification:* `coursecraft-logic` reads `obj.frame` for `objects_intersect`, drag-drop hit tests (`triggers.md:205, 217`), and scroll-panel bounds. Keeping parent `frame` single-source-of-truth prevents logic/render divergence and ensures moving/resizing an object updates all states together. (Visual decorations at different positions belong in Slice 3 `extras`.)
    - **Object `id`, `name`, `timing`, `initial_state`, `entrance`/`exit`/`motion_paths`, `drag`, `panel`, `group`, `locked`, `editor_hidden`, `focus_order`, `decorative`, `description`, `question`, `Animate` tab, `Arrange`/`Align`/`Group`, Format Painter, parent Copy/Cut/Delete:** Disabled or hidden during edit-state mode with clear tooltips so the author can never accidentally mutate the base object or slide while believing they are editing a state.
- **Per-property override indicators & reset:**
  - In Format inspector during edit-state mode, every overridable property group (Fill, Outline, Text style, Opacity, Shadow, Glow, Rotate/Flip, Image source, Crop, Image shape) reads the resolved value (`state` over `normal`).
  - When overridden in the active state, paints a `6 pt` `p.accent` dot beside the label (tooltip `"Overridden in “{state_name}”"`) and a `24×24 pt` reset icon button (`Icon::ArrowCounterClockwise`, tooltip `"Reset {property} to Normal"`, `UX-05`, `UX-10`) that dispatches `object.patch_state` with `null` for that property.
  - When clearing a nullable property (e.g. turning off the `Outline`, `Shadow`, or `Glow` toggle while Normal has one), dispatches `object.patch_state { "clear_outline": true }` (and marks the row overridden with a dot + reset button to restore Normal's outline!).
- **Text editing inside a state:**
  - Double-clicking or pressing `Enter` on a text-bearing target object (`Text`, `Button`, text-bearing `Shape`) while in `editing_state` opens `InlineEdit` with `state_name: Some(state_name)` prefilled from resolved state text/runs/paragraphs and styled with resolved `text_style`.
  - `Cmd+B/I/U` updates `live_runs` identically to normal inline edit.
  - `commit_inline_edit` (`canvas.rs:642`) checks `edit.state_name`: when `Some(state)`, dispatches `object.patch_state { text, runs, paragraphs }` instead of `text.set` or `object.update`.
  - `Esc` cancels inline edit without exiting `editing_state`.

---

### 1.3 Slice 3 — Visual Items Inside a State (`StateExtra`) (Shippable)

#### Data Model (`crates/course/src/model.rs`)
Add `extras` to `StateDef`:
```rust
#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub extras: Vec<StateExtra>,
```
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateExtra {
    pub id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    pub kind: StateExtraKind,
    /// Relative to parent object's unrotated top-left `(0, 0)` in slide pixels.
    /// Negative x/y or extents overhanging parent's w/h are allowed (e.g. corner badges).
    pub frame: Rect,
    #[serde(default, deserialize_with = "deserialize_rotation", skip_serializing_if = "is_zero_rotation")]
    pub rotation: f32,
    #[serde(default, skip_serializing_if = "is_false")]
    pub flip_h: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub flip_v: bool,
    #[serde(default = "default_opacity", deserialize_with = "deserialize_opacity", skip_serializing_if = "is_default_opacity")]
    pub opacity: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow: Option<Shadow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glow: Option<Glow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum StateExtraKind {
    Shape {
        shape: ShapeKind,
        fill: Color,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        fill_style: Option<FillStyle>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        outline: Option<Outline>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        text: Option<ShapeText>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        adjust: Vec<f32>,
    },
    Image {
        src: String,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        alt: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        crop: Option<CropRect>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        outline: Option<Outline>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        shape: Option<ShapeKind>,
    },
    Text {
        text: String,
        #[serde(default)]
        style: TextStyle,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        runs: Vec<TextRun>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        paragraphs: Vec<Paragraph>,
        #[serde(default, skip_serializing_if = "is_default_autofit")]
        autofit: Autofit,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        outline: Option<Outline>,
    },
    Path {
        #[serde(flatten)]
        path: Path,
    },
}
```
- **Strictly visual-only:** No nested `states`, no `triggers`, no `timing`, no `drag`, no `focus_order`, no `connector`/`start_glue`/`end_glue` on `Path`, no Audio/Video/Web/Inputs/Tables/Questions/ScrollPanels.

#### Coordinates, Drawing, Transforms, Hit-Testing & Animation
- **Parent-relative coordinates:** An extra at `extra.frame = { x, y, w, h }` sits at `(parent.frame.x + x, parent.frame.y + y, w, h)` in unrotated slide space. Overhanging badges (e.g., `x = parent.w - 16, y = -8`) are first-class.
- **Z-order & 2-layer extras:** Extras draw in list order (`0..n`) immediately **after** (on top of) the parent's `DrawItem`, keeping the whole composite at the parent's exact z-index relative to sibling objects and layers. When both runtime state (e.g., `visited` checkmark) and pointer state (e.g., `hover` glow/sparkle) define `extras`, runtime state's `extras` draw first, then pointer state's `extras` draw above them. `disabled` and `hidden` suppress `hover`/`down` extras completely.
- **Transform & animation inheritance:**
  - Parent `rotation`, `flip_h`, `flip_v` (using the parent's resolved state transform): rotate/flip each extra's centre around the parent's centre, compose `world_rotation = normalize_rotation(parent_rot + flip_sign * extra.rotation)`, `world_flip_h = parent_flip_h ^ extra.flip_h`, `world_flip_v = parent_flip_v ^ extra.flip_v`.
  - Parent animation sample (`sample_object`): translate extra by `sample.offset`, scale extra position & size around parent centre by `sample.scale`, multiply `item.opacity = (parent_effective_opacity * extra.opacity * sample.opacity).clamp(0.0, 1.0)`, inherit `sample.clip` (wipe) and parent scroll-panel `clip_rect` + scroll offset.
  - Parent drag-and-drop: extras move with the parent during drag and on drop (`DragDrop` / freeform drag).
  - `%var%` interpolation runs on `Text` and `Shape.text` extras during `compose`.
- **Hit-testing & `is_state_extra` tag:**
  - Add `pub is_state_extra: bool` (default `false`) to `DrawItem` (`crates/render/src/compose.rs`).
  - Every extra `DrawItem` gets `hit = Hit::Object(parent_id.clone())` (retargeted to `Hit::DragItem(parent_id)` if parent is a drag item) and `is_state_extra = true`.
  - Clicking, hovering, pressing, or dragging an overhanging extra routes `Input::Click` / `HoverStart` / `HoverEnd` / drag directly to the **parent object**.
  - Parent-lookup helpers (`access.rs:1783` `object_item`, `compose.rs:2249` `is_target_label`, `view.rs:384, 402`) filter `!item.is_state_extra` so extras never hijack the parent's primary item, focus rect, or drag-target label pill.

#### Accessibility / Screen Readers (Decision)
- **No extra focus stops:** Extras NEVER enter `focus_order` or create standalone `AccessNode`s. Keyboard Tab order and arrow navigation treat the parent + its state extras as one logical widget.
- **Screen-reader announcement (`crates/render/src/access.rs:1150-1168`):**
  - Unless the parent object has `decorative: true`, `access.rs` collects non-empty visible text from active state `extras` (`Text` runs, `Shape.text` runs, and `Image.alt` — after `%var%` display substitution) in list order and appends them to the parent's accessible `label` separated by `", "` (skipping duplicates equal to the parent label).
  - *Rationale:* An extra `Image` with `alt: ""` (e.g., a purely decorative sparkle) stays silent; an extra `Image` with `alt: "Completed"` or a `Text` badge `"Visited"` is announced as part of the button (`"Module 1, Completed"`), matching what sighted learners see without fragmenting keyboard focus.
  - `audit.rs` (WCAG checker): does NOT emit missing-alt errors on state-extra images (empty `alt` means decorative), DOES check text contrast for `Text`/`Shape.text` extras when background is determinable, and `validate.rs:884` (`slide_has_unsupported_script`) checks extra text.

#### Media, Publish, Web Player, Fonts & XLIFF
- **Media walkers (all 5 kept in sync):**
  1. `coursecraft_course::media_references` (`crates/course/src/lib.rs:269`): walk `state.fill_style` (Picture) and `state.extras` (`Image.src`, `Shape.fill_style`, `Path.fill_style`), formatting place as `"{object_place} ▸ State \"{state}\" ▸ {extra_name}"`.
  2. `coursecraft_publish::collect_object_media` (`crates/publish/src/lib.rs:167`): walk the exact same refs so every publish target bundles them into `media/` and fails loudly if any is missing or unsafe.
  3. `apps/coursecraft-player/src/media.rs:128` (`collect_object_srcs`): prefetch all state `fill_style` and `extras` images for current & next slide and protect them from LRU eviction.
  4. `crates/ui-egui/src/lib.rs:4547` (`collect_object_media_srcs`) & `:4609` (`rewrite_object_media_srcs`): keep state/extra media on `.ccraft`/folder save and rewrite paths on Media Library Replace/Rename. (Also fix latent omission of `Path.fill_style` and `Video.poster` here.)
- **Font walkers (both kept in sync):**
  1. `crates/publish/src/lib.rs:817` (`collect_used_font_faces`): walk `state.text_style`, `state.runs`, and `state.extras` (`Text`/`Shape.text` styles & runs) so fonts used only in states/extras are auto-embedded.
  2. `crates/ui-egui/src/lib.rs:4646` (`object_uses_font`): walk the same fields so Font Manager won't silently delete a font used only inside a state.
- **Strings & XLIFF (`crates/course/src/strings.rs:408, 506`, `crates/format/src/xliff.rs:778, 1351, 1689`):**
  - Existing: `state/{state_name}/text` (updated to keep `state.runs` synchronized when `state.runs` is present, matching `ObjectKind::Text`).
  - New: `state/{state_name}/extra/{extra_id}/text` (for `Text` and `Shape { text: Some(_) }` extras, with full rich-text `<g>` run support) and `state/{state_name}/extra/{extra_id}/alt` (for `Image` extras with non-empty `alt`).
  - Parity guaranteed by `crates/format/tests/xliff_parity.rs` (extended with a rich-states fixture).

#### Limits & Validation (`crates/course/src/validate.rs`)
- `pub const MAX_STATES_PER_OBJECT: usize = 32;` (error if exceeded).
- `pub const MAX_STATE_NAME_CHARS: usize = 64;` (error if empty, `> 64` chars, or `"normal"`; **warning** if `"hidden"`).
- `pub const MAX_STATE_EXTRAS: usize = 16;` per state (error if exceeded).
- Master objects: error if `!o.states.is_empty()` on a master slide (`check_master`, enforcing `triggers.md:17`).
- Mutual-exclusion errors: `clear_<field> && <field>.is_some()` for all 6 nullable fields.
- `StateDef` field checks: `fill`/`fill_style`/`outline`/`text_style`/`runs`/`paragraphs`/`shadow`/`glow`/`crop`/`opacity`/`rotation` validated using the exact same helpers as `Object`; `src` checked with `is_safe_relative_path`.
- `StateExtra` checks: valid `id` (`1..=128` `[A-Za-z0-9_-]`), unique `id` within `state.extras`, `coord_ok(frame.x, frame.y)`, `extent_ok(frame.w, frame.h)`, `opacity` in `0..=1`, finite `rotation`, valid `shadow`/`glow`, kind-specific checks (`Image.src` safe relative path + non-empty, `FillStyle::Picture.src` safe relative path, `Path` has no `connector`/`start_glue`/`end_glue` and respects `PATH_MIN_POINTS..=PATH_MAX_POINTS`).

#### Slice 3 Engine Commands
1. `object.state_add_extra`: `{"slide", "object", "state", "extra": StateExtra}` (if `extra.id` is omitted or empty, generates a fresh `"extra_1"`, `"extra_2"`, … unique within that state). Returns `{"id": extra_id}`.
2. `object.state_update_extra`: `{"slide", "object", "state", "extra": id, "patch": { name?, frame?, kind?, rotation?, flip_h?, flip_v?, opacity?, shadow?, glow? }}` (`shadow: null` / `glow: null` clears them; unknown patch keys error).
3. `object.state_delete_extra`: `{"slide", "object", "state", "extra": id}`.
4. `object.state_reorder_extra`: `{"slide", "object", "state", "extra": id, "index": usize}`.

#### Canvas Editing of `extras` in Edit-State Mode
- **Toolbar Insert while `editing_state.is_some()`:**
  - `Text`, `Shape` (+ Shape gallery), `Image`, `Path`/Draw tools: enabled; place/insert into the active state via `object.state_add_extra` with `frame` converted to parent-relative coordinates (`slide_pos - parent.frame.min`). `Image` sets `PendingImagePick::StateExtraImage { slide, object, state }` and imports via `PickImage`!
  - Non-visual tools (`Audio`, `Video`, `Button`, `Question`, `Web`, `Input` controls, `Marker`, `ScrollPanel`, `Timer`, `ProgressMeter`, `Lottie`, `Table`, `Interaction`): disabled with tooltip `"Not available inside a state"` (`UX-08`).
- **Canvas interaction:**
  - Clicking an extra selects it (`editing_state.selected_extra = Some(extra_id)`), shows its selection box + 8 resize handles + rotate handle + yellow shape adjust handle, supports drag-move, resize, rotate, arrow-key nudge (`1 px` / `Shift` `10 px`), `Delete`/`Backspace`, double-click inline text edit (`InlineEdit` with `extra_id: Some(extra_id)`), and right-click (`Duplicate extra`, `Bring forward`, `Send backward`, `Delete extra`).
  - Clicking the parent object outside extras clears `selected_extra = None` and shows the parent's state overrides in the Format inspector.
  - Selecting an extra routes the Format inspector to edit that extra's `frame` (relative `X`/`Y`/`W`/`H`), fill, outline, text style, image source/crop/shape, opacity, shadow, glow, rotation, and flip via `object.state_update_extra`.
  - **Bottom panel / Inspector item list:** Inside the Format panel when `editing_state.is_some()`, a compact `"Items in this state"` list (shown when `!state.extras.is_empty()`) lets authors select, reorder (`↑`/`↓`), or delete extras even if overlapped or tiny.

---

### 1.4 Cross-Feature Interaction Matrix

| Feature | Exact M29 Behaviour |
|---|---|
| **Built-in states (`normal`, `hover`, `down`, `selected`, `visited`, `disabled`, `hidden`)** | `normal` = base object (never in `obj.states`). `hidden` = built-in, not drawn, not clickable, never needs a `StateDef` (warning if defined). `disabled` = drawn, swallows clicks, suppresses `hover`/`down` overrides & extras. `selected`/`visited` = toggled/set by `coursecraft-logic` after clicks (`triggers.md:78-85`). `hover`/`down` = renderer presentational layer (`down` > `hover`) layered over active runtime state (`normal → runtime → pointer`). |
| **Triggers & Custom Question `correct_when`** | Zero changes to `coursecraft-logic`. `object.rename_state` atomically rewrites `initial_state`, `slide.triggers`, `layer.triggers`, and `slide.question.correct_when`. `object.delete_state` is blocked whenever referenced by any of them. |
| **Revisit modes & Suspend/Resume** | `PlayerState` and `cc1:` `suspend_data` store state name strings only — zero wire-format changes. `KeepSaved` restores state names and renders their widened overrides & extras; `Reset` resets back to `initial_state`. Existing `validate.rs:715` revisit warning unchanged. |
| **Master slides** | Master objects have no states (`triggers.md:17`). `BottomTab::States` hidden in Master view; right-click `"Edit states…"` disabled; `check_master` errors on `!o.states.is_empty()`. Pasting an object with states onto a master strips `states` and resets `initial_state = "normal"` with a neutral status notice `"States were removed on the slide master"`. |
| **Groups (`obj.group`)** | Groups are authoring-only (`model.rs:1004`). Selecting a group selects multiple objects → `BottomTab::States` shows multi-select message. Double-clicking into a group selects one member → full state editing works on that member. |
| **Copy/Paste & Duplicate** | `object.duplicate`, `object.paste`, `slide.duplicate`, `layer.duplicate`, and clipboard deep-clone full `StateDef` (all widened fields + `extras`). |
| **Slide Templates** | `template.save` and `template.add_slide` preserve all widened `states` and `extras`; `referenced_media_srcs` preserves template state/extra media on save. |
| **Find & Replace** | Searches and replaces `state/{name}/text`, `state/{name}/extra/{id}/text`, and `state/{name}/extra/{id}/alt` (preserving `%var%` refs and rich runs). Clicking a state hit in Find (`find.rs:383`) selects the slide/object, opens `BottomTab::States`, and sets `preview_state = Some(state_name)`. |
| **Check Course (`status.rs`)** | Reports validation issues on `/states/...` and missing media bytes from `state.src`, `state.fill_style`, and `state.extras`. Clicking a `/states/...` issue selects the slide & object and switches `bottom_tab = BottomTab::States`. |
| **Undo / Redo & History** | Every new command is single-step undoable with a clear History label (`Duplicate state`, `Rename state`, `Reorder state`, `Edit state`, `Add state item`, `Edit state item`, `Delete state item`, `Reorder state item`). Undoing while in `editing_state` safely exits `editing_state` if the edited state/object disappears (and clears `selected_extra` if that extra disappears). |

---

## 2. Clean-Room & Patent Check

Checked against `docs/contributing.md:21-40` (Patent watch list as of 2026-11-11):
- **SCORM data (KnowBe4 US10949760 / US11599810 / US11847579):** Not touched. Zero new LMS/SCORM data-model elements; `suspend_data` format is unchanged.
- **Automatic course from documents (Docebo US11720617B2):** Not touched. Zero AI pipeline changes.
- **Screen recording / software simulation (Assima US9570050B2 / US12360646B2):** Not touched.
- **Learner classification / adaptive content (Adobe US10911553B2 / US10943497B2):** Not touched.

**Clean-room UI & copy rules for M29:**
- Visual design follows `DESIGN.md` tokens (`coursecraft-render::design`) and Phosphor icons exclusively.
- Use CourseCraft's own plain, sentence-case wording: **`States`** tab, **`Edit state`** / **`Done editing`**, **`Reset overrides`**, **`Items in this state`**, **`New state`**, **`Initial`**. Never copy competitor dialog chrome, icons, or coined labels.
- All example courses and test assets (`examples/rich-states/`) are newly authored clean-room fixtures.

---

## 3. Rules (`ST-01`–`ST-20`)

| Rule ID | Category | Normative Requirement |
|---|---|---|
| **ST-01** | Schema Compatibility | Every pre-M29 v1 `course.json` loads identically and re-serializes byte-for-byte unchanged unless M29 state fields are edited. |
| **ST-02** | Loud Forward Refusal | Any M29 `course.json` using new `StateDef` fields or `extras` fails loudly with a serde `unknown field` error on a 0.3.x `StateDef` (`#[serde(deny_unknown_fields)]`), never silently dropping data. |
| **ST-03** | Tri-State Integrity | Nullable state overrides (`fill_style`, `outline`, `shadow`, `glow`, `crop`, `image_shape`) distinguish Inherit (`None, false`), Set (`Some(v), false`), and Clear (`None, true`); `Some(_) && clear == true` is rejected by both `validate.rs` and `object.patch_state`. |
| **ST-04** | Two-Layer Precedence | Renderer resolves `normal → runtime_state → pointer_state` (`down` > `hover`) independently per property; `disabled` and `hidden` suppress `hover`/`down` overrides AND `hover`/`down` extras completely. |
| **ST-05** | Legacy Fill Precedence | A state overriding solid `fill` with `fill_style: None && !clear_fill_style` suppresses lower layers' `fill_style`, preserving pre-M29 behaviour. |
| **ST-06** | Atomic State Rename | `object.rename_state` updates `StateDef.name`, `obj.initial_state`, `slide.triggers`, `layer.triggers`, and `slide.question.correct_when` in one atomic undo step; zero dangling references remain. |
| **ST-07** | Hostile Input Safety | Bounds enforced in `validate.rs` and engine: `MAX_STATES_PER_OBJECT = 32`, `MAX_STATE_NAME_CHARS = 64`, `MAX_STATE_EXTRAS = 16`, `is_safe_relative_path` on all media paths, finite clamped floats, zero panics. |
| **ST-08** | No Logic Bloat | `coursecraft-logic` remains untouched; `simulate` output on `hello-course`, `showcase`, and `reference-course` is byte-for-byte identical. |
| **ST-09** | Parent-Single-Frame | Parent `frame` (`x/y/w/h`) is never overridable per state and is locked during edit-state mode; moving/resizing the parent moves all states and parent-relative `extras` together. |
| **ST-10** | Edit-State Routing | While `editing_state.is_some()`, every enabled Format control, inline text edit, crop, rotate/flip, and image pick mutates ONLY the active state (or `selected_extra`), never the base `normal` object. |
| **ST-11** | Extra Isolation & Hit Parity | `StateExtra` is visual-only (`Shape`, `Image`, `Text`, `Path` without connectors/glue); all extra `DrawItem`s carry `Hit::Object(parent_id)` (or `Hit::DragItem`) + `is_state_extra: true`, so clicks/hovers/drags on extras drive the parent without hijacking `access.rs` or drag-target label pills. |
| **ST-12** | Transform & Animation Inheritance | State `extras` inherit parent `rotation`, `flip_h`, `flip_v`, entrance/exit/motion-path `offset`/`scale`/`opacity`/`clip`, drag offset, and scroll-panel `clip_rect` + scroll offset. |
| **ST-13** | Accessibility Without Tab Bloat | State `extras` never create standalone `AccessNode`s or focus stops; non-empty extra text and image `alt` append to the parent's accessible label (unless parent is `decorative`). |
| **ST-14** | Universal Media & Font Parity | `media_references`, `publish::collect_object_media`, `player::media::collect_object_srcs`, `ui-egui::collect_object_media_srcs`, and `rewrite_object_media_srcs` all walk `state.src`, `state.fill_style`, and `state.extras`; `collect_used_font_faces` and `object_uses_font` walk state/extra fonts. |
| **ST-15** | Strings & XLIFF Parity | `course_strings`, `string_mut`, `export_xliff`, and `import_xliff` agree 100% on `state/{name}/text`, `state/{name}/extra/{id}/text`, and `state/{name}/extra/{id}/alt` (verified by `xliff_parity.rs`). |
| **ST-16** | Editor/Preview/Player Parity | Canvas state preview, States tab thumbnail, in-editor `Preview`, and published `PlayerFrame` (desktop & WASM) compose through the exact same `coursecraft-render` functions with `Some(&course.theme)`. |
| **ST-17** | Master Slide Purity | Master objects have no states (`triggers.md:17`): `BottomTab::States` hidden in Master view, `"Edit states…"` disabled, `check_master` rejects `!o.states.is_empty()`, paste to master strips states cleanly. |
| **ST-18** | Universal Undo & History | Every new mutating command is single-step undoable/redoable, listed in `docs/cli.md`, tested in `m27_all_commands_undo_tests.rs`, and clears stale `preview_state`/`editing_state`/`selected_extra` on undo. |
| **ST-19** | UI/UX Sweep Compliance | `BottomTab::States` and Canvas Edit-State Mode pass every M28 rule `UX-01`–`UX-22` across `1024×600`, `1280×800`, `1920×1080` × `Dark`/`Light` × scale `1.0`/`1.5`, plus long-name and empty fixtures. |
| **ST-20** | Performance Budget | Off-screen state thumbnails skip paint; idle editor triggers 0 course copies and 0 redundant state repaints; 10-state object and 50-object stateful slide stay within frame budget. |

---

## 4. Suspects Watchlist (`SUS-01`–`SUS-16`)

Existing code locations verified before implementation that will break or silently drop data unless updated:

| ID | File & Lines | Failure Mode / Latent Bug | Fixed In |
|---|---|---|---|
| **SUS-01** | `crates/render/src/access.rs:1783-1788` | `object_item` uses `items.iter().rev().find(...)` on `Hit::Object(id)`. Once Slice 3 emits extra `DrawItem`s *after* the parent with `Hit::Object(parent_id)`, `.rev().find()` grabs the last **extra** instead of the parent, breaking screen-reader label and focus rect! Must skip `item.is_state_extra`. | Step 9 |
| **SUS-02** | `crates/render/src/compose.rs:2249-2256` | `is_target_label` matches any `Hit::Object(_)` + `Visual::Text` overlapping a drag target. A `Text` extra inside a state on a drag target or drag item would be misclassified as a target-label pill and moved outside! Must require `!item.is_state_extra`. | Step 9 |
| **SUS-03** | `crates/ui-egui/src/canvas.rs:5989-6002` | `preview_state` calls `compose_object(obj, preview_state)` with `theme = None` (losing theme accent/text/font) AND replaces only a single `items[pos]`, which drops all Slice 3 `extras` AND runs *after* `missing_images_as_placeholders` (`:5970`). | Step 2 & Step 9 |
| **SUS-04** | `crates/ui-egui/src/lib.rs:4547-4641` | `collect_object_media_srcs` and `rewrite_object_media_srcs` walk only `state.src` (and already miss `Path.fill_style` and `Video.poster`). Without walking `state.fill_style` and `state.extras`, saving a `.ccraft` deletes state/extra images and Media Library Replace breaks paths! | Step 3 & Step 8 |
| **SUS-05** | `crates/publish/src/lib.rs:167-174, 817` | `collect_object_media` walks only `state.src`, and `collect_used_font_faces` ignores `obj.states` completely. Published packages will miss images in `state.fill_style`/`state.extras` and fail to auto-embed fonts used only in states/extras. | Step 3 & Step 8 |
| **SUS-06** | `apps/coursecraft-player/src/media.rs:128-170` | `collect_object_srcs` only checks `state.src` inside `ObjectKind::Image` and ignores `state.fill_style` and `state.extras`. Web player will fail to prefetch or protect state/extra images from LRU eviction. | Step 3 & Step 9 |
| **SUS-07** | `crates/ui-egui/src/lib.rs:4646-4661` | `object_uses_font` checks only base `Text`/`Shape` and ignores `obj.states` (`text_style`/`runs`/`extras`), letting Font Manager delete fonts still used in states. | Step 3 & Step 8 |
| **SUS-08** | `crates/course/src/validate.rs:755-825` | `check_master` delegates to `check_object`, which never rejects `!o.states.is_empty()` on master slides, violating `docs/spec/triggers.md:17`. | Step 1 |
| **SUS-09** | `crates/ui-egui/src/states.rs:191-213` | `state_delete_blocker` checks `slide.triggers` and `layer.triggers`, but **misses** `slide.question.correct_when` (`Condition::ObjectState` on Custom questions), letting the UI enable Delete only for `keep_valid` to reject it. | Step 1 |
| **SUS-10** | `crates/course/src/validate.rs:884-888` | `slide_has_unsupported_script` checks `state.text` only; misses `state.extras` text (`Text` and `Shape.text`). | Step 8 |
| **SUS-11** | `crates/course/src/strings.rs:408-416, 506-513` & `crates/format/src/xliff.rs:778-788, 1351-1359, 1689-1698` | Both treat `state.text` as `StringTarget::Optional` (plain only) and ignore `state.runs` and `state.extras`. Importing XLIFF or running Replace on a state with `runs` would desync `state.text` and `state.runs`! | Step 3 & Step 8 |
| **SUS-12** | `crates/ui-egui/src/canvas.rs:642-760` | `commit_inline_edit` unconditionally dispatches `text.set` / `object.update` onto the base object even while previewing/editing a state. | Step 6 & Step 10 |
| **SUS-13** | `crates/ui-egui/src/canvas.rs:6011-6045` | Canvas `Escape` handler does not know about `editing_state` or `selected_extra`, so `Esc` would clear object selection before exiting edit-state mode (violating `UX-13`). | Step 5 & Step 10 |
| **SUS-14** | `crates/ui-egui/src/properties.rs:2414, 8044` | Duplicate `"State"` and `"States"` sections in Format tab, both reading/writing base `PropertiesState` without checking `editing_state`. | Step 2 & Step 6 |
| **SUS-15** | `crates/engine/src/commands.rs:5321-5334` | `object_paste` onto a master slide checks Audio/Video but never strips or checks `object.states`, allowing states onto masters via paste. | Step 1 |
| **SUS-16** | `crates/ui-egui/src/find.rs:383-415` & `crates/ui-egui/src/status.rs:420-435` | `Find::go_to` and Check Course issue navigation select the object but never open `BottomTab::States` or set `preview_state` when the hit/issue path is under `state/{name}/...` or `/states/{i}/...`. | Step 7 |

---

## 5. Concrete Test Cases

Every test case specifies one single behaviour with a concrete expected result.

### 5.1 Unit & Engine Tests (`crates/course`, `crates/engine`, `crates/format`)

| ID | Behaviour Tested | Expected Result |
|---|---|---|
| **E-01** | Legacy 0.3.x `course.json` round-trip (`ST-01`) | Deserialize existing `examples/showcase/course.json` and `examples/reference-course/course.json`; serialize back; zero diff on `states` arrays. |
| **E-02** | Loud forward refusal by 0.3.x schema (`ST-02`) | Define a `#[serde(deny_unknown_fields)] struct LegacyStateDef { name, fill, text, src }` in tests; serialize an M29 `StateDef` with `glow`, `clear_shadow`, or `extras`; assert `LegacyStateDef` fails with `unknown field` error; assert an M29 `StateDef` using only legacy fields still deserializes into `LegacyStateDef`. |
| **E-03** | Tri-state JSON round-trip (`ST-03`) | Round-trip all 14 new `StateDef` fields + all 6 `clear_*` flags (`fill_style`, `outline`, `shadow`, `glow`, `crop`, `image_shape`); Inherit omits both keys; Set writes field only; Clear writes `clear_<field>: true` only; overridden-to-same-as-Normal serializes `Some(v)` and survives round-trip. |
| **E-04** | Validation: mutual exclusion & bounds (`ST-03`, `ST-07`) | `validate()` errors on `outline: Some(_) && clear_outline: true` (all 6 nullable pairs), `> 32` states, `> 64`-char state name, empty name, `"normal"`, duplicate state name, `> 16` extras, duplicate extra ID, unsafe `src` (`../a.png`, `C:\a.png`), non-finite/negative extra frame, `Path` extra with `connector`/`start_glue`; warns on `"hidden"` `StateDef`. |
| **E-05** | Validation: master slides reject states (`ST-17`, **SUS-08**) | Object on a master slide with `states: [hover]` produces validation error naming the master. |
| **E-06** | `object.duplicate_state` + undo/redo | Duplicates a state containing widened overrides + `extras` to `"hover copy"` (and `"hover copy 2"` on second call), inserts after source, preserves all overrides/extras; `undo` restores exact prior `Course`; `redo` restores duplicate. |
| **E-07** | `object.rename_state` rewrites all refs + undo/redo (`ST-06`) | Renames `"active"` → `"chosen"` on an object whose `initial_state == "active"` and is referenced in base trigger `Event::StateChanges`, layer trigger `Action::ChangeState`, nested `Condition::All([Not(ObjectState)])`, and `slide.question.correct_when`; all 4 rewrite cleanly, `validate()` has 0 errors, `undo` restores every ref verbatim. |
| **E-08** | `object.rename_state` refusals | Rejects renaming to `"normal"`, `"hidden"`, `""`, `> 64` chars, or an existing state name; leaves course unchanged. |
| **E-09** | `object.reorder_state` + undo/redo | Moves state from index `2` to `0`; rejects out-of-range index or unknown state name; `undo`/`redo` restores exact order. |
| **E-10** | `object.patch_state` partial set, clear, inherit-reset, `reset_all` | Patches `fill_style` + `glow` without touching existing `text`; sets `clear_outline: true` (clears `outline`); sets `glow: null` (resets to Inherit); sets `reset_all: true` (resets overrides while keeping `extras`); rejects unknown patch key and rejects `outline: Some && clear_outline: true`; single undo step each. |
| **E-11** | `object.state_add_extra` + auto-ID + undo/redo | Adds `Shape`, `Image`, `Text`, and `Path` extras; auto-generates unique ID when omitted; rejects 17th extra (`> MAX_STATE_EXTRAS`) and unsafe `Image.src` via `keep_valid`; `undo`/`redo` round-trips. |
| **E-12** | `object.state_update_extra` + undo/redo | Patches `frame`, `rotation`, `opacity`, `kind`, `shadow: null`; rejects unknown patch key or non-finite frame via `keep_valid`; `undo`/`redo` round-trips. |
| **E-13** | `object.state_delete_extra` & `object.state_reorder_extra` + undo/redo | Deletes extra by ID (errors on missing ID); reorders extra index `0 ↔ 2` (errors out of bounds); `undo`/`redo` round-trips. |
| **E-14** | `m27_all_commands_undo_tests.rs` & `docs/cli.md` completeness (`ST-18`) | All 8 new mutating commands exist in `REGISTRY`, `command_cases()`, `docs/cli.md`, and pass exact JSON undo/redo equality. |
| **E-15** | Copy/paste/duplicate/template preservation & master paste strip (**SUS-15**) | `object.duplicate`, `object.paste`, `slide.duplicate`, `layer.duplicate`, `template.save`/`add_slide` preserve widened `states` and `extras`; `object.paste` onto a master strips `states` and resets `initial_state = "normal"`. |
| **E-16** | `recolor_course` walks widened states & extras | `recolor_course` replaces matching RGB hex inside `StateDef.fill`, `fill_style` gradient stops, `outline`, `shadow`, `glow`, `text_style`, `runs`, AND `StateExtra` fills/outlines/text/shadows/glows while preserving alpha. |
| **E-17** | `course_strings` + `string_mut` + `course.replace` on states & extras (**SUS-11**) | Finds and replaces text in `state/{name}/text` (keeping `state.runs` synced!), `state/{name}/extra/{id}/text` (with runs), and `state/{name}/extra/{id}/alt`, protecting `%var%` refs. |
| **E-18** | XLIFF export/import + `xliff_parity.rs` (`ST-15`) | `export_xliff` emits units for `state/{name}/text`, `state/{name}/extra/{id}/text` (preserving `<g>` run tags), and `state/{name}/extra/{id}/alt`; `import_xliff` applies translations cleanly; `walker_and_xliff_agree` passes on a rich-states course. |

### 5.2 Render Tests (`crates/render`)

| ID | Behaviour Tested | Expected Result |
|---|---|---|
| **R-01** | `fill_style` override & `clear_fill_style` | State with linear/radial/picture/pattern `fill_style` replaces solid base fill; `clear_fill_style: true` strips base gradient back to solid fill; legacy `state.fill: Some(c)` without `fill_style` strips base gradient (`ST-05`). |
| **R-02** | `outline` override & `clear_outline` | State sets dashed/coloured `outline` on `Text`, `Shape`, `Image`, `Path`; `clear_outline: true` removes base outline (`Visual.*.outline == None`). |
| **R-03** | `text_style`, `runs`, `paragraphs`, `autofit` overrides | State overriding only `text_style` (bold + accent colour + size) re-styles base text without copying `text`; state overriding `runs` + `paragraphs` renders rich runs; state overriding `autofit` applies shrink-to-fit. |
| **R-04** | `opacity`, `shadow`, `glow` overrides & `clear_shadow`/`clear_glow` | State sets `opacity: 0.45`, adds `glow`, or sets `clear_shadow: true` to remove base shadow; final `DrawItem` carries resolved opacity (multiplied with animation sample), shadow, glow. |
| **R-05** | `crop` & `image_shape` overrides + clear flags | `Image` state changes `crop` rect and `image_shape: Some(Ellipse)`; `clear_crop: true` / `clear_image_shape: true` restores uncropped rectangle. |
| **R-06** | `rotation`, `flip_h`, `flip_v` overrides | State sets `rotation: 15.0`, `flip_h: true`, `flip_v: true`; `DrawItem` receives resolved transform (not additive with base, override replaces base value). |
| **R-07** | Two-layer precedence `normal → runtime → hover/down` (`ST-04`) | Button in `visited` (custom text + outline + checkmark extra) hovered with `hover` (glow + fill) renders ALL FOUR simultaneously; pressing applies `down` over `hover`; leaving restores `visited` only. |
| **R-08** | `disabled` and `hidden` suppress `hover`/`down` (`ST-04`) | Object in `disabled` hovered/pressed keeps `disabled` visuals & extras with zero `hover`/`down` overrides or extras; `hidden` emits 0 `DrawItem`s. |
| **R-09** | `StateExtra` parent-relative placement & overhang | Extra at `{ x: 144, y: -8, w: 24, h: 24 }` on parent `{ x: 100, y: 200, w: 160, h: 48 }` composes at slide rect `{ x: 244, y: 192, w: 24, h: 24 }` immediately after parent in `DrawItem` list with `is_state_extra == true`. |
| **R-10** | `StateExtra` hit-testing routes to parent (`ST-11`) | Pointer click/hover on overhanging corner of an extra (outside parent `frame`) hits `Hit::Object(parent_id)` and emits `Input::Click { object: parent_id }` / `HoverStart` / `HoverEnd`. |
| **R-11** | `StateExtra` parent rotation & flip inheritance (`ST-12`) | Parent rotated `90°` or `flip_h: true` transforms extra's centre around parent centre and composes rotation/flip cleanly; pixel check confirms badge rotates with button. |
| **R-12** | `StateExtra` parent animation & motion-path inheritance (`ST-12`) | During fade-in (`opacity 0.5`), zoom (`scale 0.5`), wipe (`clip`), or motion path (`offset (40, 20)`), extra inherits exact offset, scale-around-parent-centre, multiplied opacity, and wipe clip. |
| **R-13** | `StateExtra` inside `ScrollPanel` & `DragDrop` (**SUS-02**) | Extra on a scroll-panel child shifts by `-panel_offset` and clips to `panel_rect`; extra on a drag item gets `Hit::DragItem(id)` and moves while dragged; `Text` extra on a drag target is NOT misclassified as a moved target-label pill. |
| **R-14** | `StateExtra` `%var%` interpolation | `Text` and `Shape.text` extras containing `"%score% pts"` display substituted runtime variable value. |
| **R-15** | `StateExtra` two-layer stacking (`visited` + `hover` extras) | Both `visited` and `hover` define extras → `visited` extras draw above parent, `hover` extras draw above `visited` extras; leaving hover removes only `hover` extras. |
| **R-16** | Offscreen pixel render of all 4 `StateExtraKind`s | `render_slide_rgba` renders `Shape`, `Image`, `Text`, and `Path` extras without panics or NaN meshes, producing distinct non-background pixels at expected slide coordinates. |

### 5.3 Logic, Runtime & Cross-Host Parity Tests (`crates/logic`, `crates/render`, `crates/ui-egui`)

| ID | Behaviour Tested | Expected Result |
|---|---|---|
| **P-01** | `coursecraft-logic` zero regression (`ST-08`) | `simulate` output on `examples/hello-course`, `examples/showcase`, and `examples/reference-course` is unchanged. |
| **P-02** | Editor canvas preview vs Preview vs PlayerFrame pixel parity (`ST-16`, `UX-22`) | Render same stateful slide (with theme accent, widened overrides, and extras) via canvas `compose_object_items`, in-editor `Preview`, and `snapshot_player` at matching size; pixel diff inside slide rect `<= 1%`. |
| **P-03** | Revisit `KeepSaved` vs `Reset` with rich states | Click object → `visited` (override + checkmark extra); jump away and back under `KeepSaved` → `visited` override + extra remain; repeat under `Reset` → returns to `normal` (extra gone). |
| **P-04** | Suspend/resume preserves rich states | Encode `cc1:` suspend data while object is in `selected` (with overrides + extras), resume fresh `Player`, compose slide → renders `selected` overrides & extras identically. |
| **P-05** | `access.rs` parent item & screen-reader label (`ST-13`, **SUS-01**) | Interactive button `"Topic 1"` in `visited` state with checkmark `Image { alt: "Completed" }` + decorative `Shape` (no text) produces **1** `AccessNode` (parent focus rect, NOT extra rect!) with label `"Topic 1, Completed"` and zero extra focus targets. |
| **P-06** | `access.rs` decorative parent & empty-alt extra | Button with `decorative: true` emits 0 nodes even with text extras; non-decorative button with `Image { alt: "" }` extra keeps label `"Topic 1"` unchanged. |
| **P-07** | Custom question (`freeform`) `correct_when` with renamed/rich states | Answer card toggled to `selected` (rich overrides + extra) satisfies `Condition::ObjectState { state: "selected", is: true }`; grades Correct in both `simulate` and `PlayerFrame`. |
| **P-08** | Hover/down pointer parity in `SlideView` (`view.rs`) | Real pointer move/press/release events in `SlideView` trigger `hover` and `down` overrides & extras (including when pointer enters over an overhanging extra) and fire `Input::Click` on release. |

### 5.4 Editor UI-egui Harness Tests (`crates/ui-egui`)

| ID | Behaviour Tested | Expected Result |
|---|---|---|
| **U-01** | `BottomTab::States` visible & Master view hiding (`ST-17`) | Bottom tab strip shows `States` on normal slides; switching to Master view hides `States` and switches `bottom_tab` back to `Timeline`. |
| **U-02** | Empty state & multi-select state (`UX-17`, `UX-08`) | No object selected → explanatory empty text; 2 objects (or a group) selected → multi-select message + disabled add/edit controls with tooltips; double-clicking into group member shows member's states. |
| **U-03** | Live side-by-side thumbnails & culling (`ST-16`, `ST-20`) | Object with `Normal`, `hover`, `selected`, `visited` renders 4 side-by-side cards with live painted visuals (using course theme and imported images); off-screen cards in horizontal scroll skip paint. |
| **U-04** | Add state via quick-add chips & custom name; no `"Hidden"` chip | Clicking `+ Hover` adds `hover` and disables chip (`"Already added"`); no `+ Hidden` chip exists; typing `"custom_1"` + `Enter` adds it; typing `"normal"` or `"hidden"` shows inline validation hint and disables Add. |
| **U-05** | Duplicate, inline Rename, Reorder, Set Initial, Delete via UI | Each action works from card UI/menu, updates canvas/triggers immediately, and round-trips single-step `Ctrl+Z` / `Ctrl+Shift+Z` (`UX-21`). Delete disabled with blocker tooltip when state is `initial_state`, trigger-referenced, or `correct_when`-referenced (**SUS-09**). |
| **U-06** | Compact Format panel row & right-click `"Edit states…"` (**SUS-14**) | Format tab has 1 compact `States` section (`Initial` combo + `"Open States tab (N)"`, never duplicate `"State"` + `"States"`); clicking `"Open States tab"` or right-clicking object → `"Edit states…"` switches `bottom_tab = BottomTab::States`. Right-click `"Edit states…"` disabled in Master view and multi-select with tooltips (`UX-08`). |
| **U-07** | Keyboard navigation & accesskit in States tab (`UX-10`, `UX-13`) | `ArrowRight`/`ArrowLeft` move card focus, `F2` starts rename, `Esc` cancels rename, `Delete` removes unblocked state; every card & icon button has non-empty `accesskit` label, tooltip, and `>= 24×24 pt` hit target (`UX-05`). |
| **U-08** | Enter & exit Canvas Edit-State Mode + `Esc` stack (**SUS-13**, `UX-13`) | Clicking `"Edit state"` (or double-clicking custom state card) opens accent top banner, dims non-target objects, ignores clicks on non-target objects; `Esc` closes popup → inline edit → `selected_extra` → `editing_state` in strict order; `"Done editing"` exits cleanly. |
| **U-09** | Format inspector routing in Edit-State Mode (`ST-10`) | While editing `"hover"`, changing Fill, Fill style (gradient), Outline, Opacity, Shadow, Glow, Rotation, Flip, Crop, and Image shape mutates ONLY `hover` via `object.patch_state`; `normal` object is byte-for-byte untouched; each change is 1-step undoable (`UX-21`). |
| **U-10** | Override dots, per-property reset & clear toggles (`ST-03`) | Overridden properties show accent dot + reset icon; clicking reset restores Inherit; toggling OFF Outline/Shadow/Glow when Normal has one sets `clear_* = true` and shows override dot; `"Reset overrides"` resets all overrides in 1 undo step. |
| **U-11** | Parent frame & object-level controls locked in Edit-State Mode (`ST-09`, `UX-08`) | Parent `X/Y/W/H`, `Timing`, `Animate`, `Arrange`, `Group`, `Name` are disabled with explanatory hover tooltips; dragging parent on canvas does NOT move `obj.frame`. |
| **U-12** | Inline text editing inside a state (**SUS-12**) | Double-click / `Enter` on text/shape/button while editing `"selected"` → type `"Tadah!"` + `Cmd+B` → commit → updates `selected.text` & `selected.runs`, leaves `normal` text untouched; `Ctrl+Z` reverts. |
| **U-13** | Image picker reuse in Edit-State Mode (u20 integration) | Editing `"hover"` of an `Image`, click `"Choose image…"` → `PendingImagePick::StateImage` → `import_image` sets `hover.src` without touching `normal.src`; `"Clear"` resets to `None`; `"From project…"` reuses existing asset. |
| **U-14** | Canvas insert of `StateExtra`s & disabled non-visual tools (`UX-08`) | In Edit-State Mode, inserting `Shape`, `Text`, `Image` (`PendingImagePick::StateExtraImage`), and `Path` creates parent-relative `StateExtra`s; `Audio`/`Video`/`Question`/`Input`/`Web`/`Table` toolbar buttons are disabled with `"Not available inside a state"`. |
| **U-15** | Select, drag, resize, rotate, inline-edit, reorder & delete `StateExtra` | Click extra selects it (`selected_extra`), drag moves `extra.frame` relative to parent, resize updates `extra.frame.w/h`, Format edits route to `object.state_update_extra`, double-click edits extra text, `Delete` removes extra, `Ctrl+Z` restores it; clicking parent deselects extra. |
| **U-16** | Live state thumbnail reflects widened overrides & `extras` | Adding a checkmark extra or glow to `"visited"` immediately updates the `"visited"` thumbnail card in `BottomTab::States` while leaving `"Normal"` thumbnail unchanged. |
| **U-17** | Find/Replace & Check Course navigation to state (**SUS-16**) | Clicking a Find hit in `state/visited/extra/e1/text` or a Check Course issue on `/states/0/src` selects slide & object, opens `BottomTab::States`, and previews `"visited"`. |
| **U-18** | Undo/delete safety while editing a state | Undo past `object.add_state` (or delete edited state) cleanly exits `editing_state`; undo past `object.state_add_extra` clears `selected_extra` without panic or stale ID. |
| **U-19** | Save/load & Media Library preserve state/extra media (**SUS-04**) | Save `.ccraft` + reopen keeps images referenced only by `state.fill_style` and `state.extras`; Media Library shows them in `"Places using file"` and `"Replace"` rewrites their paths. |
| **U-20** | Full M28 UI/UX matrix sweep (`ST-19`, `UX-01`–`UX-22`) | Add `BottomTab::States` (empty, multi-select, 1-state, 8-state, long-names) AND Canvas Edit-State Mode (parent selected + extra selected) to `uiux_sweep_tests.rs`; **0 MUST violations** across `1024×600`, `1280×800`, `1920×1080` × `Dark`/`Light` × scale `1.0`/`1.5`. |

### 5.5 Publish & Strict LMS Tests (`crates/publish`, `crates/strict-lms`, `apps/coursecraft-player`)

| ID | Behaviour Tested | Expected Result |
|---|---|---|
| **PUB-01** | Bundle all state & extra media (**SUS-05**) | Publish course with images used ONLY in `state.src`, `state.fill_style` (Picture), `state.extras` (`Image.src`, `Shape.fill_style`, `Path.fill_style`); zip contains every file under `media/` and passes strict manifest check. |
| **PUB-02** | Missing or unsafe state/extra media blocks publish | Missing bytes for a `state.fill_style` or `state.extras` image returns `PublishError::MissingMedia` naming the object and path; `../` returns `PublishError::UnsafePath`. |
| **PUB-03** | Auto-embed fonts used only in states/extras (**SUS-05**) | Custom font family referenced ONLY by `StateDef.text_style` or `StateExtraKind::Text` is collected by `collect_used_font_faces` and embedded in the published package. |
| **PUB-04** | Web player media prefetcher & LRU protection (**SUS-06**) | `slide_image_srcs` and `slide_media_srcs` (`apps/coursecraft-player/src/media.rs`) return all images from `state.src`, `state.fill_style`, and `state.extras` on base objects and layers. |
| **PUB-05** | SCORM 1.2 + SCORM 2004 3rd/4th Ed. via `strict-lms` | Publish `examples/rich-states/` to SCORM 1.2, 2004 3rd, and 2004 4th; run scripted learner (pass, fail, mid-course suspend/resume preserving rich states) through `StrictScorm12` / `StrictScorm2004` → **0 violations**. |
| **PUB-06** | cmi5 & xAPI via `strict-lms` | Publish `examples/rich-states/` to cmi5 and xAPI; run scripted learner + resume through `StrictLrs` → **0 violations**. |
| **PUB-07** | LTI 1.3 via `StrictLtiPlatform` | Publish `examples/rich-states/` to LTI 1.3; launch + complete through `StrictLtiPlatform` → **0 violations**, score posted accurately. |
| **PUB-08** | Web target + Web resume | Publish `examples/rich-states/` to Web; run scripted learner + suspend/resume via `WebResume` → resumes on exact slide with rich states intact. |

### 5.6 End-to-End Scenarios (`examples/rich-states/`)

Build a dedicated clean-room example course **`examples/rich-states/`** (`course.json`, `media/` clean-room PNG/SVG assets, `pass.inputs.json`, `resume-1.inputs.json`, `resume-2.inputs.json`, `README.md`), tested in `apps/coursecraft-cli/tests/m29_rich_states.rs` like `examples/reference-course/`:

| ID | Scenario in `examples/rich-states/` | Verification |
|---|---|---|
| **E2E-01** | **Slide 1 — Expressive character:** Character image whose `hover` state swaps `src` + adds a speech-bubble `Shape`+`Text` extra (`"Click me!"`) + `glow`, `down` tilts `rotation: 3.0`, and `selected` swaps to a celebratory pose `src` with an overhanging star `Path` extra. | Scripted pointer hover → press → click → leave in `SlideView` + offscreen render checks confirm every visual swap, extra appearance, and hit on overhanging speech bubble; `simulate` records `selected` state change. |
| **E2E-02** | **Slide 2 — Visited checklist buttons & disabled unlock:** Three topic buttons (`normal` → `visited` adds a green overhanging checkmark `Shape`+`Path` extra + outline change; `hover` adds outer glow layered *over* `visited`; "Continue" button starts `disabled` at `opacity: 0.45`, ignores hover/click, unlocks to `normal` via trigger once all 3 are `visited`). | Scripted learner hovers disabled Continue (no hover visual, click ignored), clicks all 3 buttons (each gains checkmark extra, hovering visited button shows BOTH checkmark AND glow), Continue unlocks and advances slide. |
| **E2E-03** | **Slide 3 — Interactive flip cards & revisit modes:** Two cards that swap linear gradient `fill_style`, dashed `outline`, `shadow`, bold accent `text_style` + rich `runs`, and an info badge `Image` extra on `selected`; one slide uses `KeepSaved`, a sibling uses `Reset`. | Scripted learner selects cards, jumps away & returns, plus suspends & resumes via `cc1:` → `KeepSaved` restores rich `selected` look, `Reset` restores `normal`. |
| **E2E-04** | **Slide 4 — Custom question (`freeform`) + XLIFF round-trip:** Graded by `Condition::ObjectState` on two selected cards; full course exported to XLIFF, translated (including state text, extra text, extra alt), and re-imported cleanly. | Passes grading in `simulate` and `PlayerFrame`; translated course renders localized text in normal, state overrides, and state extras with 0 XLIFF warnings. |
| **E2E-05** | **All 6 publish targets end-to-end:** `examples/rich-states/` published to SCORM 1.2, SCORM 2004 (3rd & 4th), cmi5, xAPI, LTI 1.3, and Web, driven by `pass.inputs.json` and `resume-1/2.inputs.json` through `crates/strict-lms` + `StrictLtiPlatform`. | 0 violations across all 6 targets, 100% media present, `coursecraft-cli validate` 0 errors, `coursecraft-cli render` succeeds on every slide. |

### 5.7 Performance Tests (`crates/ui-egui/src/guard_tests.rs`, `crates/render`)

| ID | Workload | Budget / Assertion |
|---|---|---|
| **PERF-01** | 1 object with 10 rich states (each with overrides + 4 extras) open in `BottomTab::States` | Scrolling horizontally paints ONLY visible cards (`thumb_rect.intersects(clip_rect)`); 5 idle frames record **0** `course_copies` and **0** redundant state-thumbnail repaints (`guard_counters`). |
| **PERF-02** | Slide with 50 objects × 6 states × 4 extras (all visible) | `compose` + `paint` on `SlideView` completes with zero allocations/panics and average frame compose time `< 4 ms` (release) / `< 16 ms` (debug guard), and **0** `course_copies` on idle editor frames. |
| **PERF-03** | Live drag-scrubbing opacity/glow in Edit-State Mode | Dragging a `DragValue` across 20 frames in Edit-State Mode does not run full validation/audit per drag frame (`audit_dirty` deferred) and produces bounded frame work. |
| **PERF-04** | `cargo xtask perf` regression check | Existing 300-slide big-course save/open/publish/render timings stay within M21 recorded baseline budgets. |

### 5.8 Fuzz & Hostile Input Tests (`crates/course`, `crates/engine`, `crates/render`)

| ID | Hostile Input | Expected Result |
|---|---|---|
| **FZ-01** | Non-finite / huge floats (`NaN`, `Infinity`, `-1e30`, `1e30`) in `StateDef` (`opacity`, `rotation`, `shadow`, `glow`, `crop`, `outline.width`) and `StateExtra.frame` | Deserialization clamps safely or `validate()` returns clean error; `compose` & `paint` never panic or emit NaN vertices. |
| **FZ-02** | Unknown keys in `StateDef`, `StateExtra`, `StateExtraKind`, and `object.patch_state.patch` | Rejected cleanly with serde / `EngineError::Invalid`, zero panics. |
| **FZ-03** | Excessive counts: `1,000` states on an object; `500` extras in one state; `5,000`-point `Path` extra | `validate()` returns clean bounded-count error; renderer handles even unvalidated structs without OOM or hang. |
| **FZ-04** | Path traversal & OS paths (`"../secret.png"`, `"C:\\Users\\a.png"`, `"\"a.png\""`, `"\0bad.png"`, `"https://evil/a.png"`) in `state.src`, `state.fill_style`, and `extra` `Image.src`/`fill_style` | Rejected by `validate()`, `keep_valid`, `publish`, `DesktopMediaSource`, and `WebMediaSource`. |
| **FZ-05** | Missing media bytes for `state.src`, `state.fill_style`, and `state.extras` images | Canvas & state thumbnails show neutral placeholder (no crash); player shows graceful fallback (no crash); Check Course lists warning; Publish refuses with `MissingMedia`. |
| **FZ-06** | `panic_hunt` across all 8 new engine commands | Existing `crates/engine/src/tests.rs:2784` `panic_hunt` throws empty/null/wrong-type/huge/unicode params at all 8 new commands with 0 panics. |

---

## 6. Release Gate for M29

### 6.1 Slice "Done" Criteria (Each Slice Ships Cleanly)
- **Slice 1 Done (after Step 2):** `BottomTab::States` live thumbnails, add/duplicate/rename/delete/set-initial/reorder, compact Format panel row, right-click `"Edit states…"`, `"Hidden"` chip removed; **E-05–E-09, E-14 (Slice 1 cmds), U-01–U-07, PERF-01** pass; `cargo xtask ci` green.
- **Slice 2 Done (after Step 7):** All 14+ widened `StateDef` overrides + 6 `clear_*` flags, `object.patch_state`, 2-layer resolver, Canvas Edit-State Mode, per-property override dots & reset, inline text edit inside state, u20 image picker reuse, Find/Check Course navigation; **E-01–E-04, E-10, E-16, R-01–R-08, P-01–P-04, U-08–U-13, U-17–U-18, PUB-01–PUB-04 (Slice 2), FZ-01–FZ-06** pass; `cargo xtask build-player` + `cargo xtask ci` green.
- **Slice 3 Done (after Step 12):** `StateExtra` model/commands/renderer/transforms/hit-testing/a11y/XLIFF/media/publish/canvas editing, `examples/rich-states/` end-to-end across all 6 LMS/web targets, full 3×2×2 UI/UX matrix sweep; **ALL test IDs (`E-01`–`E-18`, `R-01`–`R-16`, `P-01`–`P-08`, `U-01`–`U-20`, `PUB-01`–`PUB-08`, `E2E-01`–`E2E-05`, `PERF-01`–`PERF-04`, `FZ-01`–`FZ-06`)** pass; `cargo xtask build-player` + `cargo xtask ci` + `cargo xtask perf` green.

### 6.2 Automated Release Gate Commands
Run in order before cutting a release:
1. `cargo xtask build-player` (rebuilds embedded WASM web player after all render/player changes)
2. `cargo xtask ci` (`fmt --check`, `clippy --workspace --all-targets -D warnings`, `test --workspace`, `layers`, `wasm`)
3. `cargo test -p coursecraft-cli --test m29_rich_states` (end-to-end `examples/rich-states/` + all 6 publish targets via `strict-lms`)
4. `cargo test -p coursecraft-ui-egui uiux_sweep_all_surfaces` (0 MUST violations across all surfaces including `States` tab and Edit-State Mode)
5. `cargo xtask perf` (confirm big-course timings stay within M21 baselines)

### 6.3 Owner Manual Verification Checklist (Desktop + Published Browser Course)
Before releasing M29, click through this 10-minute check by hand:
1. **States tab & thumbnails:** Open `examples/rich-states/` (or a new course). Select the character on Slide 1. Open the **`States`** tab at the bottom.
   - *See:* `Normal`, `hover`, `down`, `selected` live thumbnails side-by-side; no duplicate `"State"`/`"States"` sections in Format (only the compact `Initial` + `"Open States tab"` row); no `+ Hidden` chip.
   - Right-click an object → click **`Edit states…`** → jumps straight to the **`States`** tab.
   - Switch to **View ▸ Slide master** → `States` tab disappears; right-click **`Edit states…`** is disabled with tooltip.
2. **Duplicate, rename & delete safety:** Duplicate `hover` → rename it to `hover_alt` → confirm a trigger targeting that state updates automatically → `Cmd+Z` reverts both rename and trigger. Confirm Delete is disabled with a tooltip on a trigger-referenced state.
3. **Edit-State Mode & overrides (Noe's workflow):** Select an Image object → add `Hover` → click **`Edit state`**.
   - *See:* Accent `"Editing state “hover” of …"` banner at top of canvas; other slide objects dimmed and unclickable; parent `X/Y/W/H` locked with tooltip.
   - Click **`Choose image…`** and pick a PNG from disk → image swaps immediately in `hover` without changing `Normal`; accent override dot + reset icon appear; add an outer **Glow** and **Outline**; click **`Done editing`** (or press `Esc`).
   - *See:* `Normal` thumbnail unchanged; `hover` thumbnail shows the new image, glow, and outline.
4. **Text editing & items inside a state:** Select a button → add `Visited` → click **`Edit state`** → double-click button to edit text to `"Done"` → insert a small green circle **Shape** and drag it to overhang the top-right corner → click **`Done editing`**.
   - *See:* `Visited` thumbnail shows `"Done"` + corner badge; `Normal` shows original label and no badge. Move the base button on the canvas → badge stays pinned to the button's corner when previewing `Visited`.
5. **Preview & Published Web/SCORM Player:** Press `F12` (Preview), then **File ▸ Publish…** to **Web** (Open in browser) and **SCORM 1.2** (check in `cargo xtask package-example` / mock LMS):
   - *See:* Hovering the silhouette/character swaps its image, glow, and speech bubble; hovering the overhanging badge keeps hover active; clicking a visited button adds its checkmark badge and hovering it shows BOTH checkmark and glow; disabled button stays dimmed and ignores hover/click; Dark & Light editor themes both look clean and readable.

---

## 7. Numbered Implementer Steps (12 Steps, ~30–40 min each)

> **Cadence:** Full CI gate (`cargo xtask ci`) after Steps **3, 6, 9, and 12** (plus slice-shippable check at Slice 1 end Step 2 and Slice 2 end Step 7). Web player rebuild (`cargo xtask build-player`) after every render/player change (Steps **4, 9, and 12**).

### Slice 1 — Bottom `States` Tab (Steps 1–2)

- **Step 1 — State Management Engine Commands & Validation Hardening**
  - **Goal:** Add `object.duplicate_state`, `object.rename_state` (rewriting `initial_state`, base/layer triggers, and `question.correct_when`), and `object.reorder_state`; enforce `MAX_STATES_PER_OBJECT = 32`, `MAX_STATE_NAME_CHARS = 64`, `"hidden"` `StateDef` warning, master-slide state rejection (**SUS-08**), and master-paste state stripping (**SUS-15**).
  - **Crates:** `crates/course`, `crates/engine`.
  - **Tests first:** `E-05`, `E-06`, `E-07`, `E-08`, `E-09`, `E-14` (for new Slice 1 commands), `E-15` (master paste strip), `FZ-06` (new commands in `panic_hunt`).
  - **Docs:** `docs/cli.md` (add `object.duplicate_state`, `object.rename_state`, `object.reorder_state`), `docs/spec/course-model.md` (document state limits and `"hidden"` warning).

- **Step 2 — Bottom `States` Tab With Live Thumbnails & Compact Format Row** *(Slice 1 shippable)*
  - **Goal:** Add `BottomTab::States` (`timeline.rs`, `states.rs`) with live horizontal thumbnail cards (`compose_object_with_value` + `SlideTransform::fit` + `paint`, culled on `clip_rect`), header controls (`Initial`, quick-add chips without `"Hidden"`, `+ New state`, `"Edit state"`), card actions (add/dup/inline-rename/delete/set-initial/reorder), keyboard/accesskit, empty & multi-select states; fix `state_delete_blocker` for `correct_when` (**SUS-09**); fix `canvas.rs` `preview_state` theme + placeholder order (**SUS-03**); shrink Format panel to compact `Initial` + `"Open States tab (N)"` row (**SUS-14**); add right-click `"Edit states…"` (`menu.rs`).
  - **Crates:** `crates/ui-egui`.
  - **Tests first:** `U-01`, `U-02`, `U-03`, `U-04`, `U-05`, `U-06`, `U-07`, `PERF-01`.
  - **Docs:** None.
  - **Slice 1 shippable check:** `cargo xtask ci`.

### Slice 2 — Widened `StateDef` Overrides & Canvas Edit-State Mode (Steps 3–7)

- **Step 3 — Widened `StateDef` Schema, Validation, `object.patch_state` & Media/Font/String Walkers**
  - **Goal:** Add all 14 widened override fields + 6 `clear_*` flags to `StateDef` (`model.rs`, `serde(default, skip_serializing_if)`, `deny_unknown_fields`); validate mutual exclusion, colours, fills, outlines, text, crops, opacity, rotation (`validate.rs`); add `object.patch_state` (`commands.rs`, History label, `m27_all_commands_undo_tests.rs`); extend media walkers (`course/lib.rs`, `publish/lib.rs`, `ui-egui/lib.rs`, `player/media.rs`, fixing **SUS-04/05/06**), font walkers (**SUS-05/07**), and `strings.rs`/`xliff.rs` `state.runs` sync (**SUS-11**).
  - **Crates:** `crates/course`, `crates/engine`, `crates/format`, `crates/publish`, `crates/ui-egui`, `apps/coursecraft-player`.
  - **Tests first:** `E-01`, `E-02`, `E-03`, `E-04` (Slice 2 fields), `E-10`, `E-14` (`object.patch_state`), `E-16` (Slice 2 fields), `E-17` (`state.runs`), `PUB-01`–`PUB-04` (Slice 2 `state.fill_style`/fonts), `FZ-01`, `FZ-02`, `FZ-04`.
  - **Docs:** `docs/spec/course-model.md` (document widened `StateDef` & tri-state clear flags), `docs/cli.md` (add `object.patch_state`).
  - **Full Gate #1:** `cargo xtask ci`.

- **Step 4 — Renderer Two-Layer Override Resolver (All Widened Properties)**
  - **Goal:** Upgrade `coursecraft-render` (`compose.rs`, `paint.rs`, `view.rs`) to resolve `normal → runtime_state → pointer_state` (`down` > `hover`, suppressed by `disabled`/`hidden`) across every widened property and `clear_*` flag, keeping legacy solid `fill` over `fill_style` precedence (`ST-05`).
  - **Crates:** `crates/render`.
  - **Tests first:** `R-01`, `R-02`, `R-03`, `R-04`, `R-05`, `R-06`, `R-07`, `R-08`, `P-01`, `P-02` (Slice 2), `P-03`, `P-04`, `P-08`.
  - **Docs:** `docs/spec/triggers.md` (update "Built-in states after a click" with full two-layer property precedence table).
  - **Web player build:** `cargo xtask build-player`.

- **Step 5 — Canvas Edit-State Mode Chrome, Dimming, Locking & `Esc` Stack**
  - **Goal:** Add `StateEditSession` (`editor.ui.editing_state`), non-blocking accent top banner (`"Editing state “…” of …"`, `"Reset overrides"`, `"Done editing"`), dimming & click-blocking of non-target slide objects, target dashed outline, `Esc` precedence stack (**SUS-13**), parent `frame`/move/resize/object-level locking (`ST-09`), and routing of canvas rotate handle & Crop mode into `object.patch_state`.
  - **Crates:** `crates/ui-egui`.
  - **Tests first:** `U-08`, `U-11`, `U-18` (edit-state exit on undo/delete).
  - **Docs:** None.

- **Step 6 — Format Inspector Routing, Override Dots/Reset, Inline Text Edit & Image Picker**
  - **Goal:** Route all overridable Format controls (Fill, Fill style, Outline, Text style, Opacity, Shadow, Glow, Rotate/Flip, Image source via u20 `PendingImagePick::StateImage`, Crop, Image shape) into `object.patch_state` when `editing_state.is_some()`; add accent override dot + per-property reset button + clear-toggle semantics; route `canvas.rs` `InlineEdit` (`commit_inline_edit`, **SUS-12**) into `object.patch_state { text, runs, paragraphs }`.
  - **Crates:** `crates/ui-egui`.
  - **Tests first:** `U-09`, `U-10`, `U-12`, `U-13`, `PERF-03`.
  - **Docs:** None.
  - **Full Gate #2:** `cargo xtask ci`.

- **Step 7 — Slice 2 Interactions & UI/UX Sweep (Find, Check Course, Copy/Paste)** *(Slice 2 shippable)*
  - **Goal:** Wire `Find::go_to` and Check Course `/states/...` navigation to open `BottomTab::States` and preview the matched state (**SUS-16**); verify copy/paste/duplicate/templates/Media Library (`U-19` for `state.fill_style`); extend `uiux_sweep_tests.rs` with `BottomTab::States` and Edit-State Mode surfaces and resolve any `UX-01`–`UX-22` findings.
  - **Crates:** `crates/ui-egui`.
  - **Tests first:** `U-17` (Slice 2), `U-19` (Slice 2), `U-20` (Slice 2 surfaces), `E-15` (widened states).
  - **Docs:** None.
  - **Slice 2 shippable check:** `cargo xtask ci`.

### Slice 3 — Visual Items Inside a State (`StateExtra`) (Steps 8–12)

- **Step 8 — `StateExtra` Schema, Validation, Engine Commands & Walkers (Media, Fonts, Strings, XLIFF)**
  - **Goal:** Add `StateExtra` & `StateExtraKind` (`Shape`, `Image`, `Text`, `Path`) to `StateDef.extras` (`model.rs`), `MAX_STATE_EXTRAS = 16` + full validation (`validate.rs`, **SUS-10**), 4 engine commands (`object.state_add_extra`, `object.state_update_extra`, `object.state_delete_extra`, `object.state_reorder_extra`), and extend media/font/strings/XLIFF walkers (`course/lib.rs`, `strings.rs`, `format/xliff.rs`, `publish/lib.rs`, `ui-egui/lib.rs`, fixing **SUS-04/05/07/11**).
  - **Crates:** `crates/course`, `crates/engine`, `crates/format`, `crates/publish`, `crates/ui-egui`.
  - **Tests first:** `E-04` (extras limits/paths/no-connector), `E-11`, `E-12`, `E-13`, `E-14` (4 extra commands), `E-15` (extras copy/template), `E-16` (recolor extras), `E-17` (strings/replace extras), `E-18` (XLIFF parity with extras), `PUB-01`–`PUB-03` (extras media & fonts), `FZ-03`, `FZ-04`, `FZ-06`.
  - **Docs:** `docs/spec/course-model.md` (document `StateExtra` / `StateExtraKind` & limits), `docs/cli.md` (add the 4 `object.state_*_extra` commands).

- **Step 9 — Renderer `StateExtra` Composition, Transforms, Hit-Testing, A11y & Web Prefetch**
  - **Goal:** Emit parent-relative extra `DrawItem`s with `is_state_extra: true` and `Hit::Object(parent_id)` (or `Hit::DragItem`), inheriting parent rotation/flip/animation/motion-path/drag/scroll-panel transforms & `%var%` substitution; layer runtime + pointer extras; fix `access.rs:1783` (**SUS-01**) and append non-empty extra text/alt to parent accessible label without adding tab stops (`ST-13`); fix `compose.rs:2249` `is_target_label` (**SUS-02**); update `canvas.rs` multi-item state preview (**SUS-03**) and `apps/coursecraft-player/src/media.rs` prefetch (**SUS-06**).
  - **Crates:** `crates/render`, `apps/coursecraft-player`, `crates/ui-egui` (multi-item preview helper).
  - **Tests first:** `R-09`, `R-10`, `R-11`, `R-12`, `R-13`, `R-14`, `R-15`, `R-16`, `P-02` (with extras), `P-05`, `P-06`, `PUB-04` (extras prefetch), `FZ-05`.
  - **Docs:** `docs/spec/triggers.md` (note hit-testing & a11y behaviour of state extras).
  - **Web player build:** `cargo xtask build-player`.
  - **Full Gate #3:** `cargo xtask ci`.

- **Step 10 — Canvas Editing of `StateExtra`s Inside Edit-State Mode**
  - **Goal:** Enable visual-only toolbar Insert (`Text`, `Shape`, `Image` via `PendingImagePick::StateExtraImage`, `Path`) and disable non-visual Insert buttons (`UX-08`) while `editing_state.is_some()`; wire canvas select (`selected_extra`), drag, 8-handle resize, rotate, shape adjust, arrow nudge, double-click inline text edit, context menu (duplicate/reorder/delete), `"Items in this state"` inspector list, Format inspector routing to `object.state_update_extra`, and live thumbnail composition of `extras`.
  - **Crates:** `crates/ui-egui`, `apps/coursecraft` (if `PendingImagePick::StateExtraImage` touches shell pick routing).
  - **Tests first:** `U-14`, `U-15`, `U-16`, `U-17` (extra hit jump), `U-18` (extra undo safety), `U-19` (extra media save/replace).
  - **Docs:** None.

- **Step 11 — End-to-End `examples/rich-states/` Course, Strict-LMS Publish & Perf Guards**
  - **Goal:** Author clean-room `examples/rich-states/` covering character expression hover/down/selected, visited checkmark badge + disabled unlock, flip cards + revisit modes, custom question + XLIFF round-trip; add `apps/coursecraft-cli/tests/m29_rich_states.rs` running scripted learners through all 6 publish targets via `crates/strict-lms` + `StrictLtiPlatform`; add `PERF-02` (50-object stateful slide) guard test.
  - **Crates:** `examples/rich-states/`, `apps/coursecraft-cli`, `crates/ui-egui` (`guard_tests.rs`), `crates/render`.
  - **Tests first:** `E2E-01`, `E2E-02`, `E2E-03`, `E2E-04`, `E2E-05`, `PUB-05`, `PUB-06`, `PUB-07`, `PUB-08`, `P-07`, `PERF-02`.
  - **Docs:** `examples/rich-states/README.md`.

- **Step 12 — Full UI/UX Matrix Sweep, Web Player Build & Release Gate Sign-Off** *(Slice 3 shippable)*
  - **Goal:** Run full `uiux_sweep_all_surfaces` (all surfaces including `States` tab, Edit-State Mode parent + extra selected, empty & long-name fixtures across `3 × 2 × 2` matrix), fix any remaining visual/layout/contrast/tooltip violations, rebuild embedded web player, run full release gate (`cargo xtask build-player`, `cargo xtask ci`, `cargo xtask perf`), and record results in `docs/roadmap.md` / `docs/testing/m29-report.md`.
  - **Crates:** `crates/ui-egui`, `crates/render`, `apps/coursecraft-player`.
  - **Tests first:** `U-20` (full matrix green), `PERF-04`.
  - **Docs:** `docs/roadmap.md` (mark M29 status), `docs/testing/m29-report.md` (final gate log & suspect dispositions).
  - **Web player build:** `cargo xtask build-player`.
  - **Full Gate #4:** `cargo xtask ci` + `cargo xtask perf`.
