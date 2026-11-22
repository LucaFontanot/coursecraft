# Tester Feedback Analysis: Object States (Noe, V03)

**Type:** Read-only engineering & UX analysis (zero source edits)  
**Source feedback:** `docs/user-feedback/V03/Feedback_Noe.md` + screen recordings (`a.png`, `a_end.png`, `b2.png`)  
**Scope:** Silent hover-image failure on the silhouette, plus Suggestions 1–3 (discoverability/layout, file picker for `StateDef.src`, direct canvas state editing & mixed elements).

---

## 1. Root-Cause Analysis: Why the Silhouette Hover Image Never Showed

In `a.png` / `a_end.png`, the tester adds `hover` and `selected` states to an image, sees only an empty `Src` text box, opens Windows Explorer, runs **Copy as path** on `Picture48.png` (copying `"C:\Users\...\Picture48.png"` with quotes and backslashes), and pastes it into `Src`. In `b2.png` (Preview), the gray text box swaps to `"Tadah!"` and stays `"Tadah!"` after the pointer moves away until clicked again (so it is driven by a click / `selected` state or trigger), while hovering the black silhouette does nothing — it never swaps images and never even displays a broken-image placeholder.

Three separate defects stack here:

1. **Defect 1A (Primary play-time bug): Built-in `hover` and `down` states never swap image `src` (or text, or path fills).**
   - `docs/spec/course-model.md:587` and `docs/spec/triggers.md:87` state that `hover` and `down` are presentational states applied by the renderer without storing them in runtime state.
   - `crates/render/src/compose.rs:1200-1213` extracts **only** `hover_fill` and `down_fill` from the `"hover"` / `"down"` `StateDef`s and attaches them **only** to `Visual::Button` (`:240-241, 1272-1273`) and `Visual::Shape` (`:251-252, 1351-1352`).
   - `Visual::Image` (`compose.rs:255-263, 1379-1395`) carries **no** hover/down `src` (and no `disabled` flag). `crates/render/src/paint.rs:2222-2333` completely ignores `is_hovered` and `is_pressed` for `Visual::Image`. The same gap affects `Visual::Text` (`:225-235, 1215-1250`) and `Visual::Path` (`:1356-1365`).
   - `SlideView::show_covered` (`crates/render/src/view.rs:1083`) calls `compose(player)` using only `player.object_state(&obj.id)` (`compose.rs:2124`), which never becomes `"hover"` unless the author wires manual `hover_start` / `hover_end` → `change_state` triggers.
   - **Evidence from `b2.png`:** Because hovering the silhouette kept showing the normal black silhouette rather than switching to `"image failed to load"` (`paint.rs:2293-2314`), the renderer never even attempted to load the hover state's `src`.

2. **Defect 1B (Media-lookup & publish failure): Pasting an OS path into `StateDef.src` can never resolve in Preview, Canvas, or Published output.**
   - `crates/ui-egui/src/states.rs:536-570` exposes a raw single-line `TextEdit` for `state.src` and commits whatever string the user pastes via `object.update_state` (`crates/engine/src/commands.rs:7305-7327`).
   - `MemoryMediaSource::get` (`crates/ui-egui/src/lib.rs:4398-4407`) matches strictly against keys already imported into `editor.media` (created via `sanitize_media_src`). An absolute Windows path (especially wrapped in quotes from **Copy as path**) is never in `editor.media` and is never read from disk.
   - If the state *is* activated (via eye preview, `selected`, or a trigger), `MediaCache::texture_for` returns `MediaView::Failed` and `paint.rs:2293-2314` draws `"image failed to load"`. (Minor secondary bug: `crates/ui-egui/src/canvas.rs:5965` runs `missing_images_as_placeholders` *before* `preview_state` replaces the item at `:5984-5997`, so eye-previewing a broken state `src` renders `"image failed to load"` instead of the neutral `"No image yet"` placeholder.)
   - At publish time, `crates/publish/src/lib.rs:167-174` collects `state.src` into `MediaRef`, causing `PublishError::UnsafePath` or `PublishError::MissingMedia` (`:64-67`) only *after* the author tries to publish.

3. **Defect 1C (Zero validation or Check Course feedback today):**
   - **Does `Course::validate()` warn or error today? No.** `crates/course/src/validate.rs:2057-2078` validates `o.states` for `name != "normal"`, duplicate names, and `s.fill.is_valid()`. It never checks `is_safe_relative_path` on `s.src` (unlike fonts `:288`, background audio `:320`, web folders `:1780`, captions `:1805`, and resources `:2598`). Note that `ObjectKind::Image.src` (`:1968-1977`) also only checks `!src.trim().is_empty()`, not `is_safe_relative_path`.
   - **Does Check Course warn about missing media bytes today? No.** `Editor::refresh_validate_only` (`crates/ui-egui/src/lib.rs:3667-3677`) runs `course.validate` (L0 `coursecraft-course` has no access to `editor.media` bytes), and `Check Course` (`crates/ui-egui/src/status.rs:317-418`) displays only `validation_issues` + `audit_findings` (WCAG checks). Even though `coursecraft_course::media_references(&course)` (`crates/course/src/lib.rs:269-280`) already walks `state.src`, nothing cross-checks those references against `editor.media` in Check Course.

---

## 2. Issue-by-Issue Breakdown

### Issue 0: Silent-Failure Bug (Built-In `hover`/`down` on All Visuals + `state.src` Validation)

#### Today
- `compose.rs:1200-1213, 1272-1273, 1351-1352` and `paint.rs:1825-1844, 1933-1955`: only `Button` and `Shape` apply built-in `hover`/`down` fills. `Image`, `Text`, and `Path` ignore `hover`/`down` completely.
- `validate.rs:2057-2078`: accepts any string in `StateDef.src`.
- `status.rs:317-418`: no missing-media check against `editor.media`.

#### Smallest Fix
1. **Renderer presentational states (`hover`/`down`):** Introduce `compose_with_pointer(player, hovered: Option<&str>, pressed: Option<&str>)` (or pass `PresentationalOverlay { hovered, pressed }` into `compose_object_opts` when composing in `SlideView::show_covered` at `crates/render/src/view.rs:1083`). Resolve effective state overrides in layers: `normal → runtime_state → hover/down` (skipping `hover`/`down` when `runtime_state == "disabled"` or `"hidden"`). This immediately makes `hover`/`down` work for `Image.src`, `Text.text` + `Text.fill`, `Shape.text` + `Shape.fill`, `Button.label` + `Button.fill`, and `Path.fill` — without bloating every `Visual` enum variant with `hover_*` / `down_*` fields.
2. **Path validation in `coursecraft-course`:** In `crates/course/src/validate.rs:2057-2078`, if `s.src` is `Some(src)`:
   - error if `!src.is_empty() && !is_safe_relative_path(src)` (`"state src {src:?} must be a safe relative path under media/"`), and
   - warn if `s.src.is_some()` on a non-image/non-video object.
3. **Missing-media warning in Check Course:** In `Editor::refresh_validate_only` (`crates/ui-egui/src/lib.rs:3667-3677`), append a `"warning"` issue for any `MediaReference` from `coursecraft_course::media_references(&self.session.course)` whose `src` is not present in `self.media` (and not a web folder prefix). Run `missing_images_as_placeholders` *after* `preview_state` substitution in `canvas.rs:5984-5997`.

#### Ambitious Version (Clean-Room)
- Unified 2-layer state resolver in `coursecraft-render` (`resolve_state_visual(object, runtime_state, pointer_state, theme)`) that layers arbitrary widened `StateDef` properties and state `extras` (see §3), plus inline broken-media badge + one-click **"Relink…"** action directly inside Check Course and on state cards.

#### Impact Matrix
- **Schema:** None (no new fields).
- **Engine commands:** None (`keep_valid` automatically rejects unsafe `state.src` once `validate.rs` enforces `is_safe_relative_path`).
- **Runtime (`coursecraft-logic`) & spec:** Zero changes to `coursecraft-logic`. Update `docs/spec/triggers.md:87` to specify exact `hover`/`down` layering precedence (`disabled`/`hidden` suppress `hover`/`down`; `down` overrides `hover`; `hover`/`down` layer their defined overrides on top of the current runtime state such as `selected`/`visited`).
- **Player / Web (WASM):** Shared via `coursecraft-render` (`SlideView` / `PlayerFrame`). `apps/coursecraft-player/src/media.rs:139-145` already prefetches `state.src` for `ObjectKind::Image`, so web player hover swaps are instant once prefetched.
- **Risk & Effort:** **Low risk.** **1.5–2 implementer steps** (1 step for `compose_with_pointer` + `triggers.md` + render tests; 0.5–1 step for `validate.rs` + missing-media Check Course warning + canvas `preview_state` placeholder order).

#### Headless Test Cases
- `coursecraft-render`: hovering/pressing an `Image` with a `"hover"`/`"down"` `StateDef { src: Some("alt.png") }` switches `Visual::Image.src` to `"alt.png"`; leaving restores base `src`; when runtime state is `"disabled"`, hovering keeps base `src`; when runtime state is `"visited"` (with overridden text) and `"hover"` overrides only `fill`, both `visited` text and `hover` fill apply; hovering a `Text` object with `"hover"` `text` swaps its displayed runs.
- `coursecraft-course`: `validate()` returns error on `StateDef { src: Some("C:\\Users\\a.png") }`, `Some("\"C:\\a.png\"")`, `Some("../escape.png")`, and passes on `Some("picture48.png")`.
- `coursecraft-engine`: `object.update_state` rejects an unsafe `src` path via `keep_valid` and preserves prior valid course.
- `coursecraft-ui-egui` harness: missing media in `state.src` surfaces a warning in `editor.ui.validation_issues` / Check Course; eye preview on missing `state.src` shows neutral placeholder.

---

### Suggestion 2: Replace the Raw `Src` Text Box With a File Picker

#### Today
- `crates/ui-egui/src/states.rs:536-570`: renders `ui.label("Src")` + `TextEdit::singleline(&mut buf.src_override)` (`120 pt` wide) for `ObjectKind::Image`.
- By contrast, the base image inspector (`crates/ui-egui/src/properties.rs:8698-8705, 8793-8797`) shows `"Replace image…"` which sets `editor.pending_replace_object` and emits `EditorRequest::PickImage`. Both native (`apps/coursecraft/src/main.rs:1211-1227`) and WASM (`main.rs:3083-3101`) open `rfd` and feed `(name, bytes)` into `Editor::import_image` (`crates/ui-egui/src/lib.rs:2423-2476`), which calls `sanitize_media_src`, stores bytes in `editor.media`, and dispatches an undoable command.
- Note an exact existing precedent: `editor.pending_picture_fill: Option<(String, String)>` (`crates/ui-egui/src/lib.rs:1110, 2429-2437, 2480-2510`) routes `PickImage` into a shape's picture fill instead of inserting a new canvas object.
- **Important distinction:** Do **not** use `EditorRequest::ReplaceMediaFile { src }` (`apps/coursecraft/src/main.rs:590-611`). That overwrites an existing media file's bytes in-place across every object referencing it and is non-undoable. State image picking must go through `PickImage` → `import_image` → `object.update_state` so `normal` is untouched and the change is undoable.

#### Smallest Fix
1. Unify `pending_replace_object` (`lib.rs:1107`) and `pending_picture_fill` (`lib.rs:1110`) into one enum on `Editor`:
   ```rust
   pub enum PendingImagePick {
       ReplaceObject(String),
       PictureFill { slide: String, object: String },
       StateImage { slide: String, object: String, state: String },
   }
   ```
   (Eliminates manually clearing multiple `Option` fields across `lib.rs`, `toolbar.rs:205`, `question.rs:1334,1360`, `properties.rs:8794`, `main.rs:1215`.)
2. In `Editor::import_image` (`crates/ui-egui/src/lib.rs:2429-2462`), handle `PendingImagePick::StateImage { slide, object, state }` by looking up the current `StateDef`, cloning with `updated.src = Some(src)`, and dispatching `object.update_state` (verifying slide/object/state still exist, matching `pending_picture_fill`'s guard).
3. In `crates/ui-egui/src/states.rs:536-570`, replace the raw `TextEdit` with:
   - a small `28×28 pt` preview swatch (using `editor.media_cache`) + truncated filename (or muted `"Same as Normal"` when `None`, with full filename tooltip),
   - a `"Choose image…"` (or `"Replace…"` when set) button that sets `PendingImagePick::StateImage` and emits `EditorRequest::PickImage`,
   - a `"Clear"` icon/quiet button (enabled when `state.src.is_some()`) that resets `state.src = None` via `object.update_state`,
   - optionally a compact `"From project…"` menu listing existing image assets in `editor.media` so authors can reuse an already-imported image without duplicating bytes.
4. Keep `states.rs` free of any raw path `TextEdit`. (CLI / JSON scripts can still set `state.src` directly.)

#### Ambitious Version (Clean-Room)
- A reusable **Image Source Popover** shared by base Image `"Replace image…"`, Shape Picture Fill, and State Image:
  - **From computer…** (`PickImage`),
  - **From Media Library** (searchable live thumbnail grid of `editor.media` images),
  - **Paste image from clipboard** (desktop),
  - **Clear override** (for states),
  - plus drag-and-drop an image file directly onto a state thumbnail card in the States panel/tab to import and assign in one drop.

#### Impact Matrix
- **Schema:** None (`StateDef.src` already exists).
- **Engine commands:** None (`object.update_state` already exists and is atomic/undoable; media stays in `editor.media` and is garbage-collected on save via `referenced_media_srcs` at `crates/ui-egui/src/lib.rs:4562-4568`).
- **Runtime (`coursecraft-logic`) & spec:** None.
- **Player / Web (WASM):** Works automatically on WASM because `EditorRequest::PickImage` is already wired to `rfd::AsyncFileDialog` at `apps/coursecraft/src/main.rs:3083-3101` (verify WASM cancel clears `pending_image_pick`, or rely on the slide/object/state match guard + clearing when another pick starts).
- **Risk & Effort:** **Very low risk.** **1 implementer step** for the smallest fix (including `"From project…"` menu + cleanup of `PendingImagePick`); **+1.5 steps** for the shared Media Library popover + drag-and-drop onto state cards.

#### Headless Test Cases
- `coursecraft-ui-egui` harness:
  - Select an `Image` object with a `"hover"` state → assert no raw `Src` text input is shown, `"Choose image…"` button is present (`>= 24 pt` tall, UX-05).
  - Click `"Choose image…"` → assert `EditorRequest::PickImage` emitted and `pending_image_pick == Some(StateImage { .. })`.
  - Call `editor.import_image("alt.png", &png_bytes)` → assert `obj.states[hover].src == Some("alt.png")`, `normal` image `src` is unchanged, bytes exist in `editor.media`, and `Ctrl+Z` reverts `state.src` back to `None` (`UX-21`).
  - Click `"Clear"` → reverts `state.src` to `None` in one undo step.
  - Switch selection or delete the state before `import_image` returns → falls back safely without mutating the wrong object.
  - Pick an existing project image via `"From project…"` → sets `state.src` without duplicating media bytes.

---

### Suggestion 1: States Discoverability & Layout

#### Today
- `crates/ui-egui/src/properties.rs:2402-2418` places `States` near the very bottom of the Format tab, **below** ID, Name, Kind controls, Frame, Timing, Rotate/Flip, Opacity, Effects, Draggable, Scroll Panel, and Question.
- Worse (visible directly in `a_end.png`): `properties.rs:8044-8074` renders a separate `"State"` section containing only `Initial [normal v]` immediately above `"States"` (`properties.rs:2414`, `states.rs:216-580`), which *also* has `normal (initial)` and `Set Initial` buttons — two adjacent sections doing overlapping jobs at the bottom of a long scroll.
- Object right-click menu (`crates/ui-egui/src/menu.rs:1105-1217`) has no `"Edit states…"` action.
- Bottom panel (`crates/ui-egui/src/timeline.rs:28-46, 429-441, 2327-2341`) has 4 tabs: `Timeline`, `Triggers`, `Variables`, `Question` (hidden in Master view, where objects have no states per `docs/spec/triggers.md:17`).

#### Comparison: Option (a) vs Option (b)

| Criterion | (a) Pin States near top of Format + Right-click `"Edit states…"` | (b) New Bottom `BottomTab::States` tab with live side-by-side thumbnails |
|---|---|---|
| **Discoverability** | High when an object is selected; still hidden when Inspector is on Slide/Animate/Course tab. | Always visible in the bottom tab strip next to `Timeline` / `Triggers` / `Variables` / `Question` — matches where interactive authoring already lives. |
| **Side-by-side visual comparison** | **No.** Right inspector is ~260–280 pt wide; cards stack vertically and still require toggling eye icons one by one. | **Yes.** Horizontal card strip (`~128×104 pt` cards) shows `Normal`, `Hover`, `Selected`, `Visited`, `Disabled`, custom states simultaneously. |
| **Vertical space / `1024×600` scale `1.5`** | Putting full state cards above Frame/Text/Fill pushes core formatting below the fold unless collapsed to a summary header (`States (N)`). | Uses the existing bottom panel (already resizable and scrollable horizontally); keeps the Format panel focused on formatting. |
| **Future fit with Suggestion 3 (Canvas State Edit)** | Awkward — editing a state on canvas requires using the Format panel *at the same time*, so Format cannot simultaneously hold the state switcher and the format controls without scrolling. | **Natural fit.** Bottom `States` tab holds the state cards + `"Edit state"` / `"+ New state"` / `"Delete"` controls while the **right Format panel stays 100% free** to edit the active state's properties! |
| **Cost** | Tiny (`0.5–1` step). | Moderate (`2` steps). |

#### Live Per-State Thumbnail Feasibility (Verified in `coursecraft-render`)
**Yes — completely feasible with zero new rasterizer or offscreen PNG pipeline.**
- The slide navigator (`crates/ui-egui/src/slides.rs:1345-1375`) already paints live vector/texture thumbnails directly onto an egui `Painter` using `SlideTransform::fit` + `coursecraft_render::paint(...)` backed by `&mut editor.media_cache` and `MemoryMediaSource { media: &editor.media }`.
- For each state card (e.g., `120×76 pt` preview area inside a `132×108 pt` card):
  1. Compose the single object in that state via `compose_object_with_value(obj, state_name, Some(&course.theme), course_input_value(course, obj))` (`crates/render/src/compose.rs:1113`). *(Note: `canvas.rs:5987` currently calls `compose_object` with `theme = None`; switching both to `compose_object_with_value` fixes theme-accent button/text fidelity too.)*
  2. Compute the bounding rect of the composed item(s) (parent `item.rect` unioned with any state `extras` in Phase 3), add a small margin (e.g., `10%` or shadow/glow extent), translate `item.rect` to origin `(margin, margin)`, and build `SlideTransform::fit(thumb_rect, SlideSize { w, h })`.
  3. Paint a subtle checkerboard/slide-background rect, then call `coursecraft_render::paint(&ui.painter_at(thumb_rect), &t, &items, Highlight::default(), Some((&mut editor.media_cache, &source)), Some(&course.theme), None)`.
  4. Gate painting on `thumb_rect.intersects(ui.clip_rect())` (exact pattern from `slides.rs:1356`) so scrolled-off cards never pin textures in `MediaCache`.
- **Performance:** Objects typically have 1–8 states; painting 4–6 single-item thumbnails is cheaper than 1 slide thumbnail and reuses the exact GPU textures already decoded for the canvas.
- **Edge cases:** `"hidden"` state (if shown as a built-in info chip or initial-state option) renders dimmed at 25% opacity with a crossed-eye icon; non-visual objects (`Audio`) show a clean audio-waveform icon; `Editor::selection.object == None` or multi-select shows a clear empty state (`UX-17`: *"Select one object on the slide to view and edit its states"*).

#### Recommendation: Smallest Fix vs Ambitious Version
- **Smallest fix (immediate quick win):**
  1. Merge the duplicate `"State"` (`properties.rs:8044-8074`) and `"States"` (`properties.rs:2414`, `states.rs:216`) sections into a single collapsible `"States"` section placed near the top of the Format tab (right below ID/Name or right below Kind-specific controls, collapsed by default when an object has 0 custom states and showing `"States (N)"` + `"Initial: normal"` in its header; auto-expanded when the user clicks `"Edit states…"`).
  2. Add `"Edit states…"` to the object right-click context menu (`crates/ui-egui/src/menu.rs:1105-1217`, disabled in Master view with tooltip `"Master objects don't have states"`).
  3. Remove `"hidden"` from `SUGGESTED_STATE_NAMES` (`states.rs:14`) since `course-model.md:584` explicitly states `hidden` needs no definition (`initial_state` combo already lists `"hidden"`).
- **Ambitious version (Clean-Room):**
  - Add `BottomTab::States` right next to `Timeline` / `Triggers` / `Variables` / `Question` (`crates/ui-egui/src/timeline.rs:28-46, 429-441, 2327-2341`, hidden in Master view).
  - Clean-room Quiet Studio layout (`DESIGN.md`): top strip inside the tab with selected object name, `Initial state` combo, `"+ New state"` (with quick-add chips for `Hover`, `Down`, `Selected`, `Visited`, `Disabled` + custom name), and primary `"Edit state"` button; below it, a single horizontal `ScrollArea` (never nested vertical-in-vertical, `UX-02`) of live thumbnail cards showing `Normal` + every defined state side by side, each with name, `(initial)` badge, hover/eye preview click, duplicate/delete actions, and inline quick overrides (fill / text / `"Choose image…"`).
  - Right-click `"Edit states…"` and a compact `"States (N) — Open States tab"` row in the Format inspector both jump directly to `BottomTab::States`.

#### Impact Matrix
- **Schema / Engine / Runtime (`coursecraft-logic`) / Player (WASM):** None. Pure `coursecraft-ui-egui` + existing `coursecraft-render::paint` APIs.
- **Risk & Effort:**
  - Option (a) quick win: **Very low risk**, **0.5–1 implementer step**.
  - Option (b) `BottomTab::States` with live thumbnails: **Low risk**, **2 implementer steps** (1 step for `BottomTab::States` + thumbnail painter + card strip + right-click wiring; 1 step for responsive/theme/scale/long-name polish + `uiux_sweep_tests` integration across all 48+1 surfaces).

#### Headless Test Cases
- `coursecraft-ui-egui` harness:
  - Right-click object → `"Edit states…"` switches to `BottomTab::States` (or expands/focuses States section in quick-win phase); disabled in Master View with tooltip (`UX-08`).
  - `BottomTab::States` empty state when no object or multi-object is selected (`UX-17`).
  - Adding `hover` and `selected` shows 3 live thumbnail cards (`Normal`, `hover`, `selected`) side-by-side without clipping/overlap at `1024×600`, `1280×800`, `1920×1080` × Light/Dark × scale `1.0`/`1.5` (`UX-04`, `UX-05`, `UX-06`).
  - Clicking a state card previews it on the canvas (`editor.ui.preview_state == Some(name)`); selecting a different object updates the cards and clears stale `preview_state`.
  - No duplicate `"State"` / `"States"` sections in Format tab.

---

### Suggestion 3: Direct Canvas Editing & Mixing Elements Inside States

#### Today
- `StateDef` (`crates/course/src/model.rs:3848-3858`) holds only 3 sparse fields (`fill: Option<Color>`, `text: Option<String>`, `src: Option<String>`), edited strictly inside sidebar cards (`crates/ui-egui/src/states.rs:461-570`).
- `editor.ui.preview_state: Option<String>` (`crates/ui-egui/src/lib.rs:907`, `canvas.rs:5984-5997`) is **read-only preview**: editing any property in Format or dragging/typing on the canvas still mutates the base (`normal`) object, NOT the previewed state! (In fact, `canvas.rs:3060, 5691` merely disables shape adjust handles during `preview_state`.)

#### Data Model Evaluation: (a) vs (b) vs (c)

| Option | Model | Strengths | Fatal Limitation / Trade-Off |
|---|---|---|---|
| **(a) Widen `StateDef` to visual property overrides + Canvas State Edit Mode** | Add optional overrides to `StateDef`: `fill_style`, `outline`, `text_style`/`runs`/`paragraphs`, `crop`, `image_shape`, `opacity`, `shadow`, `glow`, `rotation`, `flip_h`, `flip_v` (using tri-state `StateOverride<T>` where "remove normal's outline/shadow/glow" must be distinct from "inherit normal"). | Clean inheritance from `normal` (moving/renaming/resizing the base object updates every state). Zero changes to `coursecraft-logic`. Solves glowing highlights on hover (Noe's clinic graphic), rich-text/color changes, dimmed disabled states, image swaps with outline/crop. | **Cannot** add a checkmark icon onto a `visited` button or a speech-bubble badge onto a character state — the exact real-world workflow Noe asked for. |
| **(b) States own extra child objects only (`extras`)** | Keep `StateDef` narrow and let each state carry a `Vec` of extra child objects shown only in that state. | Solves badges, checkmarks, icons, callouts, and composite overlays. | **Cannot** cleanly edit the base object's own fill/outline/text/image/opacity on canvas without awkwardly hiding the base object and duplicating it as a child (breaking inheritance, accessibility alt/text, and XLIFF). |
| **(c) Both: Widened `StateDef` overrides + constrained visual-only `extras` (RECOMMENDED)** | Combine (a) for modifying the host object itself with (b) for adding visual-only child decorations (`extras: Vec<StateExtra>`) positioned relative to the host's `(frame.x, frame.y)`. | Full Storyline-class authoring capability while preserving single-source-of-truth inheritance for the host object AND keeping `coursecraft-logic` completely untouched. | More surface area to implement; cleanly mitigated by shipping in two incremental stages (**c1**: widened `StateDef` + canvas state-edit mode; **c2**: `extras`). |

#### Recommended Architecture: Option (c) in Two Stages (`c1` → `c2`)

1. **Exact Schema Design (`crates/course/src/model.rs`, stays on `schema_version = 1`):**
   - `StateDef` keeps `#[serde(deny_unknown_fields)]` and all existing fields (`name`, `fill`, `text`, `src`) unchanged for 100% backwards compatibility.
   - **Important serde detail:** For nullable object properties (`outline`, `shadow`, `glow`, `fill_style`, `crop`), an author may want to *remove* Normal's shadow in `disabled`, not just replace it. Plain `Option<T>` cannot distinguish "inherit from Normal" (`None`) from "explicitly removed in this state". Use a small `StateOverride<T>` enum (`Inherit` [default, skipped on serialize], `Clear` → `"none"`, `Set(T)`), OR keep `Option<T>` for set-overrides + `clear_*: bool` (`skip_serializing_if = "is_false"`). A compact `#[serde(untagged)]` or explicit `clear_*` bool keeps JSON clean and `deny_unknown_fields`-safe (note: avoid `#[serde(untagged)]` with `deny_unknown_fields` if ambiguous — explicit `outline: Option<Outline>` + `clear_outline: bool` or a tagged enum `{ "override": "clear" } | { "override": "set", "value": ... }` is unambiguous and round-trips cleanly).
   - **Stage c1 new fields on `StateDef`** (all `#[serde(default, skip_serializing_if = ...)]`):
     - `fill_style: Option<FillStyle>` (+ `clear_fill_style: bool`)
     - `outline: Option<Outline>` (+ `clear_outline: bool`)
     - `text_style: Option<TextStyle>`, `runs: Option<Vec<TextRun>>`, `paragraphs: Option<Vec<Paragraph>>` (when `text` is also set or omitted; existing plain `text: Option<String>` keeps working identically)
     - `crop: Option<CropRect>` (+ `clear_crop: bool`), `image_shape: Option<ShapeKind>` (+ `clear_image_shape: bool`)
     - `opacity: Option<f32>` (clamped `0..=1` via `clamp_opacity`)
     - `shadow: Option<Shadow>` (+ `clear_shadow: bool`), `glow: Option<Glow>` (+ `clear_glow: bool`)
     - `rotation: Option<f32>`, `flip_h: Option<bool>`, `flip_v: Option<bool>`
     - **Deliberately omitted:** parent `frame` (`x/y/w/h`) and `visible`! Keeping parent `frame` single-source-of-truth avoids desynchronising `coursecraft-logic`'s `objects_intersect` (`triggers.md:217`), `drag` hit targets (`triggers.md:205`), and `scroll_panel` clipping. Visibility is already owned by the built-in `hidden` state.
   - **Stage c2 new field on `StateDef`:**
     - `extras: Vec<StateExtra>` (`#[serde(default, skip_serializing_if = "Vec::is_empty")]`, capped at e.g. `MAX_STATE_EXTRAS = 16` in `validate.rs`).
     - `StateExtra`: `{ id: String (unique within this state), kind: StateExtraKind, frame: Rect (relative to parent top-left `(0, 0)`), rotation, flip_h, flip_v, opacity, shadow, glow }`.
     - `StateExtraKind` is **strictly visual-only**: `Shape`, `Image`, `Text`, `Path`. No nested `states`, no `triggers`, no `timing`, no `drag`, no `focus_order`, no inputs/media/questions/scroll panels.

2. **Migration Story:**
   - **Backwards compatibility:** Every new field has `#[serde(default)]` and `skip_serializing_if`. Every existing v1 `course.json` deserializes identically and re-serializes byte-for-byte unchanged unless the author uses the new features.
   - **Precedence with legacy fields:**
     - If `state.fill_style` is `Some(_)`, it overrides solid `state.fill` (matching `ObjectKind::Shape`/`Button`); if `state.fill` is `Some(_)` and `state.fill_style` is `None`, solid `state.fill` suppresses Normal's `fill_style` (matching existing `compose.rs:1267, 1341`).
     - If `state.text` is `Some(_)` without `runs`, it behaves identically to today (`resolve_text_runs` at `compose.rs:1229, 1296`).
   - **Forward compatibility:** `#[serde(deny_unknown_fields)]` means older binaries will refuse files using new fields with a clear parse error — identical to every prior v1 schema extension (`fill_style`, `shadow`, `glow`, `shape.text`, `crop`, `motion_paths`).

3. **Interactions with Built-In States & Triggers:**
   - **`hover` and `down` (renderer-owned, `triggers.md:87`):** Resolved via the 2-layer compositor (`normal → runtime_state → pointer_state`), where `down` beats `hover`, and both are suppressed when `runtime_state` is `disabled` or `hidden`. If `visited` adds a checkmark `extra` and `hover` adds a `glow`, hovering a `visited` button displays **both** the checkmark and the glow!
   - **`selected` and `visited` (`coursecraft-logic/src/run.rs:2671-2686`):** Unchanged. Runtime tracks state name strings (`"selected"`, `"visited"`), and `compose` renders their overrides + `extras`.
   - **`disabled` (`triggers.md:74`):** Swallows clicks in `coursecraft-logic` and suppresses `hover`/`down` in `coursecraft-render`; authors can now customize `disabled` with lowered `opacity` (e.g. `0.45`), desaturated fill, or muted outline directly on canvas.
   - **`hidden` (`course-model.md:584`):** Built-in, never drawn (`compose.rs:2125`). `validate.rs` should warn (or reject new additions in UI) if someone defines a `StateDef` named `"hidden"`, because its overrides are never painted.
   - **Triggers (`change_state`, `state_changes`, `Condition::ObjectState`):** **Zero changes.** Triggers address states by `(object_id, state_name)`. `extras` live inside the parent object and never enter the slide's object ID namespace, so hit-testing (`Hit::Object(parent_id)`), keyboard focus, drag-and-drop, and `state_delete_blocker` (`states.rs:191-213`) continue to treat the parent object as a single logical unit.
   - **Downstream walkers to extend in c1/c2:**
     - `media_references` (`crates/course/src/lib.rs:269`), `referenced_media_srcs` (`crates/ui-egui/src/lib.rs:4562`), `coursecraft-publish` (`crates/publish/src/lib.rs:167`), and `apps/coursecraft-player/src/media.rs:139`: also walk `state.fill_style` (picture fill) and `state.extras` (`Image.src`, `Shape`/`Path` picture fill).
     - `coursecraft-course/src/strings.rs` (XLIFF export/import): walk `state.extras` text & `Image.alt` alongside `state.text`.
     - `coursecraft-render/src/access.rs`: accessible name remains on the parent object (incorporating `state.text` when overridden); decorative `extras` don't pollute tab order.

4. **Canvas "State Edit Mode" UX (Clean-Room):**
   - State on `EditorUi`: `pub editing_state: Option<StateEditSession>` (`slide_id`, `object_id`, `state_name`, `selected_extra: Option<String>`), replacing ad-hoc read-only `preview_state` while editing.
   - **Visual chrome:** Non-blocking accent-bordered strip across the top of the canvas: `"Editing state “hover” of Button 1"` + `"Reset overrides"` + primary `"Done editing"` button (`Esc` exits: first deselects any `selected_extra`, second exits state edit mode — `UX-13`). Non-target slide objects render dimmed (`dim_base` style) and ignore canvas clicks so the author cannot accidentally mutate another object.
   - **Command routing:**
     - Format panel & inline text edit read resolved state values and show a subtle overridden dot + reset button per overridden group.
     - Edits dispatch `object.patch_state` (new atomic, undoable engine command accepting partial override fields so drag-scrubbing opacity/glow/color stays clean and single-undo).
     - In Stage c2, toolbar Insert (`Shape`, `Image`, `Text`, `Path`) and canvas drag/resize/delete while `editing_state.is_some()` dispatch `object.state_add_extra`, `object.state_update_extra`, `object.state_delete_extra` (all atomic, undoable commands in `coursecraft-engine`). Non-visual insert tools (Audio, Video, Question, Input, Scroll Panel, Web) are disabled with tooltip `"Not available inside a state"` (`UX-08`).

#### Smallest Fix vs Ambitious Version
- **Smallest fix (Stage c1):** Widen `StateDef` with visual property overrides (`fill_style`, `outline`, `opacity`, `shadow`, `glow`, `text_style`/`runs`, `crop`, `rotation`, `flip_h`, `flip_v`) + `object.patch_state` + Canvas State Edit Mode banner routing Format panel, inline text edit, and `"Choose image…"` into the active state.
- **Ambitious version (Stage c2):** Add `StateExtra` (`Shape`, `Image`, `Text`, `Path`) with full canvas insert/select/drag/resize/delete inside state edit mode, duplicate-state command (`object.duplicate_state`), live thumbnail updates in `BottomTab::States`, plus XLIFF and Media Library integration.

#### Impact Matrix
- **Schema:** Additive optional fields on `StateDef` (v1 backwards-compatible; `#[serde(default, skip_serializing_if)]`).
- **Engine commands:**
  - Stage c1: `object.patch_state`, `object.duplicate_state` (optional), `object.rename_state` (optional, updates triggers + `initial_state` atomically).
  - Stage c2: `object.state_add_extra`, `object.state_update_extra`, `object.state_delete_extra`, `object.state_reorder_extra`.
- **Runtime (`coursecraft-logic`) & spec:** Zero logic changes! Update `docs/spec/course-model.md:578-590` (document widened `StateDef` and `extras`) and `docs/spec/triggers.md:87` (document 2-layer `hover`/`down` composition).
- **Player / Web (WASM):** Automatic via `coursecraft-render` (`compose` emits extra `DrawItem`s with `hit = Hit::Object(parent_id)` right after the parent item, respecting parent's animation/drag/motion-path `offset`, `scale`, `opacity`, `clip` and scroll-panel clipping). Extend `apps/coursecraft-player/src/media.rs:139` to prefetch `state.fill_style` and `state.extras` images.
- **Risk & Effort:**
  - Stage c1 (widened `StateDef` + canvas state-edit mode routing): **Medium risk** (Format panel has many controls that must cleanly branch on `editing_state`). **4–5 implementer steps** (1: model + validate + engine `patch_state`; 1: `compose` 2-layer resolver + render tests; 1.5: Canvas state-edit mode banner/dimming + Format panel routing; 1: inline text + image pick + undo/harness tests).
  - Stage c2 (`StateExtra` child decorations): **Medium–high risk** (canvas selection/drag/resize sub-mode + XLIFF/media/publish walkers + animation/transform inheritance). **4–5 implementer steps** (1: model + validate + media/publish/XLIFF walkers + engine commands; 1: `compose` + `paint` parent-relative transform/animation inheritance; 2: canvas insert/select/drag/resize/delete of extras + inspector routing; 1: end-to-end harness, undo, and preview/player parity tests).

#### Headless Test Cases
- `coursecraft-course`: serde roundtrip of legacy `StateDef` (zero diff), widened `StateDef` (including `clear_outline`/`clear_shadow`), and `StateExtra`s; validation rejects unsafe `src`, invalid colors, `> MAX_STATE_EXTRAS`, non-finite frames, duplicate extra IDs.
- `coursecraft-engine`: `object.patch_state`, `object.state_add_extra`, `object.state_update_extra`, `object.state_delete_extra` roundtrip `undo`/`redo` cleanly and reject invalid mutations via `keep_valid`.
- `coursecraft-render`: 2-layer composition (`normal → visited (checkmark extra) → hover (glow)`), disabled suppresses `hover`/`down`, parent motion-path/drag/entrance-animation offset & opacity apply to `extras`, hit-test on an `extra` returns `Hit::Object(parent_id)`.
- `coursecraft-publish` & `apps/coursecraft-player`: package bundles images referenced in `state.fill_style` and `state.extras`; web prefetcher queues them.
- `coursecraft-ui-egui` harness: enter state edit mode on `"hover"` → change fill/glow/image in Format panel → assert `normal` object unchanged and `hover` state updated → `Ctrl+Z` restores prior state (`UX-21`) → `Esc` exits state edit mode (`UX-13`); in c2, insert checkmark shape inside `"visited"`, drag it, exit state edit mode, verify it appears only when previewing `"visited"` or after clicking in Preview.

---

## 3. Recommended Phasing

### Phase A — Fits in Current M28 Run (Immediately After Step u18): **3.5–4 steps**

Fixes the tester's broken workflow end-to-end with zero schema changes and low regression risk while keeping `cargo xtask ci` and the M28 UI/UX sweep 100% green:

1. **Step u19 — Fix built-in `hover`/`down` rendering for Image/Text/Path + `state.src` validation (1.5 steps)**
   - Add `compose_with_pointer` 2-layer presentational resolution in `crates/render/src/compose.rs` & `view.rs` so hovering/pressing an `Image` swaps `src`, `Text` swaps `text`/`fill`, `Path` swaps `fill`, and `disabled` suppresses both.
   - Update `docs/spec/triggers.md:87`.
   - Enforce `is_safe_relative_path` on `StateDef.src` in `crates/course/src/validate.rs:2057-2078`; add missing-media warnings in `Editor::refresh_validate_only` (`Check Course`); fix `canvas.rs:5965` vs `:5984` placeholder order during `preview_state`.
2. **Step u20 — Replace raw `Src` text box with Image Picker in States (1 step)**
   - Unify `PendingImagePick` enum on `Editor` (`crates/ui-egui/src/lib.rs`, `apps/coursecraft/src/main.rs`).
   - Replace `TextEdit` in `crates/ui-egui/src/states.rs:536-570` with thumbnail + filename + `"Choose image…"` + `"From project…"` + `"Clear"` (works on desktop and WASM; atomic & undoable via `object.update_state`).
3. **Step u21 — States quick-win discoverability & deduplication (1 step)**
   - Merge duplicate `"State"` (`properties.rs:8044`) and `"States"` (`properties.rs:2414`) sections into one collapsible `"States"` section near the top of Format (with state count badge and initial-state picker).
   - Add `"Edit states…"` to the object right-click context menu (`crates/ui-egui/src/menu.rs:1105-1217`).
   - Remove redundant `"hidden"` from `SUGGESTED_STATE_NAMES` (`states.rs:14`).
   - Re-run full M28 UI/UX sweep (`uiux_sweep_tests`) + `cargo xtask ci`.

### Phase B — New Milestone: M29 "Rich Object States" (**10–12 steps total**)

Delivers the full Storyline-class, clean-room Object States experience in three shippable slices:

1. **Slice M29.1 — Bottom `States` Tab With Live Side-by-Side Thumbnails (2 steps)**
   - Add `BottomTab::States` (`crates/ui-egui/src/timeline.rs`), horizontal scrollable live thumbnail cards rendered via `compose_object_with_value` + `SlideTransform::fit` + `coursecraft_render::paint`, right-click `"Edit states…"` switches straight to it, Format panel shrinks to a compact `"Initial state"` + `"Open States tab (N)"` row.
   - Add `object.duplicate_state` and `object.rename_state` engine commands (atomic, undoable, rewriting trigger refs).
2. **Slice M29.2 — Widened `StateDef` Overrides & Canvas State Edit Mode (`Option c1`) (4–5 steps)**
   - Widen `StateDef` (`fill_style`, `outline`, `text_style`/`runs`/`paragraphs`, `crop`, `image_shape`, `opacity`, `shadow`, `glow`, `rotation`, `flip_h`, `flip_v` + explicit clear flags), add `object.patch_state`, update `compose` 2-layer resolver, media/publish/player walkers, and `docs/spec/course-model.md`.
   - Build Canvas State Edit Mode banner, dimmed non-target objects, `Esc` exit, and Format panel / inline-text / image-picker routing with per-property override indicators and reset affordances.
3. **Slice M29.3 — Visual Child Decorations Inside States (`Option c2`) (4–5 steps)**
   - Add `StateExtra` (`Shape`, `Image`, `Text`, `Path` in parent-relative coordinates) to `StateDef` + validation + `object.state_{add,update,delete,reorder}_extra` commands + media/publish/player/XLIFF walkers.
   - Emit parent-relative `DrawItem`s in `compose` inheriting parent animation/motion-path/drag/scroll transforms with `Hit::Object(parent_id)`.
   - Wire canvas insert/select/drag/resize/delete of `extras` inside State Edit Mode + full harness, undo, and preview-vs-player parity tests.
