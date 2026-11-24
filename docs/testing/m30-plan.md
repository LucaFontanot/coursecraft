# M30 Slide Size, 0.4.0 Bug Hunt & Everyday Polish — Product, Architecture & Test Plan

**Role:** Product Specification, Bug-Hunt Audit & Implementation Plan (planning only — zero source files edited)  
**Target release:** 0.4.1 / M30  
**Scope:**
- **Part A:** Slide size done properly (tester request from PhilOB) — redesigned dialog with presets, aspect lock, default-for-new-courses persistence, live placement preview, `"fill"` mode, complete content scaling (including M29 states & state items), multi-surface discoverability, and Check Course off-slide warnings.
- **Part B:** Targeted bug hunt on 0.4.0 — 8 concrete, code-grounded bugs across M29 rich object states, the contributor's seekbar behaviour (`PlayerSettings.seekbar`), the web Play start gate (`PlayerFrame.awaiting_start`), and slide-size interactions.
- **Part C:** Three small, high-value everyday authoring features — glossary Find/Replace/Spell/XLIFF coverage, one-click alphabetical glossary sort + Resources/Glossary polish, and Timeline `"Show all"` / `"Unlock all"` quick actions.

**Clean-room commitment:** Capability match only (`docs/contributing.md:21-40`). Visual layout, palette tokens, typography, icons (Phosphor MIT), and copy belong strictly to CourseCraft (`DESIGN.md` and the 22 UI/UX rules `UX-01`–`UX-22` in `docs/testing/m28-uiux-plan.md`).

> **Implementer Guardrails (`AGENTS.md`, `docs/contributing.md`)**
> 1. **Never crash:** Zero `unwrap`/`expect`/`panic!`/`unreachable!`/`todo!` in non-test code; `unsafe_code = "forbid"` everywhere except `coursecraft-video-native`. Input-derived numbers are hostile.
> 2. **Everything is a command:** Every course mutation dispatches an atomic, single-undo command through `coursecraft-engine` (`keep_valid`), is registered in `REGISTRY`, documented in `docs/cli.md` (enforced by `apps/coursecraft-cli/src/tests.rs`), and covered in `crates/engine/src/m27_all_commands_undo_tests.rs`.
> 3. **One runtime:** Play-time behaviour lives strictly in `coursecraft-logic`. Preview and published player share `coursecraft-render`.
> 4. **Spec first:** Runtime/schema changes update `docs/spec/course-model.md`, `docs/spec/triggers.md`, and `docs/cli.md` in the same step.
> 5. **Layering & WASM:** `cargo xtask layers` and `cargo xtask wasm` (L0–L5) stay green; run `cargo xtask build-player` after render/player changes (`S2`, `S6`, `S8`, `S12`).
> 6. **Test first (red → green):** Each step writes its assigned failing tests first, confirms they fail on the bug/gap, then implements the minimal fix to turn them green.

---

## 1. Goals & Clean-Room / Patent Check

### 1.1 Goals
1. **Slide size feels complete and trustworthy:** Authors can resize any course to Full HD (`1920 × 1080`), HD (`1280 × 720`), 4:3, 16:10, square, portrait (`9:16`), or custom dimensions, lock aspect ratio while editing, preview how slides will land before committing, save any size as their default for new courses, find the control effortlessly, and trust that every visual property (including M29 states and state items) scales cleanly and undoes byte-for-byte.
2. **Harden 0.4.0 before building more:** Fix every confirmed data-loss, web-player-blocking, stale-inspector, transform, and layered-state rendering bug shipped in 0.4.0.
3. **High-impact authoring polish:** Close three recurring everyday friction points (glossary Find/Replace/Spell/XLIFF, glossary A–Z sort, and Timeline Show All / Unlock All) with minimal code surface.

### 1.2 Clean-Room & Patent Check (`docs/contributing.md:21-40`)
- **SCORM data (KnowBe4 US10949760 / US11599810 / US11847579):** Not touched. Zero new LMS data-model elements; `cc1:` `suspend_data` format is unchanged.
- **Automatic course from documents (Docebo US11720617B2):** Not touched.
- **Screen recording / software simulation (Assima US9570050B2 / US12360646B2):** Not touched.
- **Learner classification / adaptive content (Adobe US10911553B2 / US10943497B2):** Not touched.
- **Clean-room UI wording:** Sentence case everywhere (`"Slide size"`, `"Lock aspect ratio"`, `"Use as default for new courses"`, `"Scale to fit"`, `"Fill slide"`, `"Keep as is"`, `"Sort A–Z"`, `"Show all"`, `"Unlock all"`). Zero competitor chrome, icons, or coined labels.

---

## 2. Part A — Slide Size, Done Properly

### 2.1 Audit of What Exists Today & What `slide_size.rs` Misses

**Current state read from the codebase:**
- `crates/engine/src/commands.rs:85, 1882-1901` (`course.set_slide_size`): accepts `{"w": u32, "h": u32, "mode": "scale"|"keep"}`, bounds `100..=7680` × `100..=4320`.
- `crates/engine/src/slide_size.rs:1-219`: scales uniform `factor = (w / old_w).min(h / old_h)`, centred by `(dx, dy)`. Scales scenes, masters, templates, banks, `editor.guides`, `course.text_styles`, and `question.regions`.
- `crates/ui-egui/src/slide_size_dialog.rs:1-142`: basic 4-radio list (`1280×720`, `1280×800`, `960×720`, `1080×1080`), **no `1920×1080`**, **no `9:16` portrait**, no aspect-ratio lock, no "use as default" button, no preview, reachable only from `File ▸ Slide Size…` (`menu.rs:671`).
- `crates/ui-egui/src/preferences.rs:10, 45-52, 119-123` & `apps/coursecraft/src/prefs.rs:231, 255-289` & `apps/coursecraft/src/main.rs:1354-1356`: `Prefs.default_slide` is a closed 4-variant `SlidePreset` enum (`Wide16x9`, `Wide16x10`, `Standard4x3`, `Square`); custom sizes and `1920×1080` are silently dropped.

**Complete audit of what `crates/engine/src/slide_size.rs:50-148` misses on resize today:**

| Model area | File & field | What happens today (`scale`) | Required M30 behaviour |
|---|---|---|---|
| **M29 `Object.states` (`StateDef`)** | `model.rs:3932-3990` (`outline`, `text_style`, `runs`, `paragraphs`, `shadow`, `glow`) | **Completely unscaled!** State outlines stay thick/thin, state fonts don't scale, state shadows/glows stay at old pixel sizes. | Scale every `StateDef`'s `outline`, `text_style`, `runs`, `paragraphs`, `shadow`, `glow` with `m.len(...)` and clamp to valid bounds. |
| **M29 `StateDef.extras` (`StateExtra`)** | `model.rs:3995-4070` (`frame`, `shadow`, `glow`, `Shape`/`Image`/`Text`/`Path` kinds) | **Completely unscaled! (Confirmed live via CLI — `BUG-03`).** Parent scales 1.5× while parent-relative `extra.frame` stays at old px, pulling corner badges into the middle of the object! | Scale `extra.frame.x = m.len(x).round()`, `y = m.len(y).round()` (preserve negative overhang — **no** `+dx/+dy` because parent-relative!), `w/h = round_len(m.len(...), 1.0)`. Scale `extra.shadow`, `extra.glow`, and kind-specific `outline`, `text`/`style`/`runs`/`paragraphs`, `path.outline`/`path.text`. |
| **`ObjectKind::Text`** | `model.rs:2135-2155` (`outline`, `style.letter_spacing`, `style.insets`, `runs[].letter_spacing`, `paragraphs[].bullet_size`) | `style.size`, `runs[].size`, `space_before/after` scale; **`outline`, `letter_spacing`, `insets`, and `bullet_size` are skipped!** | Scale `outline.width` (`0..=50`), `style.letter_spacing` & `run.letter_spacing` (`MIN_LETTER_SPACING..=MAX_LETTER_SPACING`), `style.insets.{left,top,right,bottom}` (`>= 0`, finite), `paragraph.bullet_size` (`1..=400`). |
| **`ObjectKind::Image`** | `model.rs:2169-2183` (`outline`, `crop`) | `outline` is **skipped**; `crop` is normalized `0..=1` of source image (stays valid & unchanged under uniform scaling). | Scale `Image.outline`; leave `crop` untouched (document invariant). |
| **`ObjectKind::Path`** | `model.rs:2359-2362, 2994-3025` (`path.points`, `path.subpaths`, `path.outline`, `path.text`) | `points`/`subpaths` are normalized `0..=1` of `o.frame` (scale automatically); `outline` scales; **`path.text` (`ShapeText`) is skipped!** Glued connectors stay aligned because all frames transform by the same affine `Map`. | Scale `path.text` (`style`, `runs`, `paragraphs`). |
| **`ObjectKind::Timer`** | `model.rs:2305-2319` (`style`) | **`style` is skipped!** Timer digits shrink/grow out of sync with the slide. | Scale `Timer.style` (`size`, `letter_spacing`, `insets`). |
| **`ObjectKind::Marker`** | `model.rs:2279-2294` (`runs`, `paragraphs`) | **`runs` and `paragraphs` are skipped!** | Scale `Marker.runs` and `Marker.paragraphs`. |
| **`ObjectKind::Shape` & `Table` & `NamedTextStyle`** | `model.rs:2156, 2365, 3834` | `style.size`/`runs.size`/`space_before/after` scale; **`letter_spacing`, `insets`, `bullet_size` are skipped.** | Route all text styling through one shared `scale_text_style` / `scale_runs` / `scale_paragraphs` / `scale_shape_text` / `scale_shadow` / `scale_glow` / `scale_outline` helper set. |
| **`ScrollPanel` & children** | `model.rs:1069, 2296` | Children store slide-space `o.frame` (`compose.rs:2960-2970`), so scaling both panel and children by `Map` keeps alignment, and `content_height` scales with `m.len`. | Keep existing `content_height` scaling + verify child/panel alignment in tests. |
| **`MotionPath` & `HotspotRegion` & `Guide`** | `slide_size.rs:92-102, 159-166, 202-208` | Already scaled (relative motion paths by `m.len`, absolute by `m.x`/`m.y`; regions by `m.x`/`m.y`/`m.len`; guides by `m.x`/`m.y`). | Clamp scaled `MotionPath` points and `Guide.at` to finite values within `validate.rs` bounds (`-100_000..=100_000`) so extreme `"fill"` resizes never fail `keep_valid`. |
| **Question layouts, player bar, export & published output** | `compose.rs:2122-2250`, `player_frame.rs`, `export/src/{mp4,pages}.rs` | `SlideTransform::fit` letters/pillars any aspect ratio cleanly; player strip reserves its own strip (`m28` h6) and never covers the slide; MP4 (`mp4.rs:160-174`) and PDF/Word (`pages.rs:154-162`) derive height from `course.slide_size`. In `compose.rs:2250`, `row_w = 900.0.min(prompt_rect.width())` stays readable at 1280×720, 1920×1080, 1080×1080, and 1080×1920, though `question_prompt_rect` (`x=80, w=slide_w-160`) collapses to `w=0` when `slide_w <= 160`! | Clamp question margin proportionally when `slide_w < 320.0` (`margin = (slide_w * 0.1).min(80.0)`) while keeping `slide_w >= 800.0` byte-for-byte identical to today (`margin = 80.0`) so existing parity tests never shift! Test all 7 presets through `compose`, `PlayerFrame`, `render_slide_rgba`, and `export`. |

### 2.2 Engine Command: `course.set_slide_size` (Backward Compatible + `"fill"` Mode)

- **Command ID:** `"course.set_slide_size"` (`crates/engine/src/commands.rs:85, 1882-1901`).
- **Params:** `{"w": number, "h": number, "mode": "scale"|"fill"|"keep"}`.
  - `"scale"` (existing, unchanged semantics): `factor = (w / old_w).min(h / old_h)`, centred (`dx = (w - old_w * factor) / 2`, `dy = (h - old_h * factor) / 2`). Entire old canvas fits inside the new slide (adds letterbox/pillarbox margins when aspect ratio changes).
  - `"fill"` (**new**): `factor = (w / old_w).max(h / old_h)`, centred (`dx = (w - old_w * factor) / 2`, `dy = (h - old_h * factor) / 2`). Old canvas fills the new slide edge-to-edge with zero blank margins; symmetric strips along the longer axis extend past the new edges (objects pushed completely outside are flagged by Check Course!).
  - `"keep"` (existing, unchanged semantics): updates `course.slide_size` only; no objects, guides, or styles move or scale.
- **Safety:** Every scaled object frame clamps `w/h` to `1.0..=MAX_OBJECT_FRAME_SIZE` and coordinates to finite values inside `validate.rs` bounds; `keep_valid(s, was_valid)?` runs before returning; single atomic undo step restoring the exact prior `Course` JSON; unchanged `(w, h)` sets `s.skip_undo = true`.
- **Docs updated in `S1`:** `docs/cli.md` (params line for `course.set_slide_size`) and `docs/spec/course-model.md` (document `slide_size` bounds and resize modes).

### 2.3 Shared Presets, Aspect-Ratio Formatting & Default-Size Persistence

Define one single source of truth in `crates/ui-egui/src/slide_size_dialog.rs` (reused by `preferences.rs`):

```rust
pub struct SlideSizePreset {
    pub label: &'static str,
    pub w: u32,
    pub h: u32,
}

pub const PRESETS: [SlideSizePreset; 7] = [
    SlideSizePreset { label: "16:9 Full HD (1920 × 1080)", w: 1920, h: 1080 },
    SlideSizePreset { label: "16:9 HD (1280 × 720)",       w: 1280, h: 720  },
    SlideSizePreset { label: "16:10 Widescreen (1280 × 800)", w: 1280, h: 800 },
    SlideSizePreset { label: "4:3 Standard (960 × 720)",   w: 960,  h: 720  },
    SlideSizePreset { label: "4:3 Classic (1024 × 768)",   w: 1024, h: 768  },
    SlideSizePreset { label: "1:1 Square (1080 × 1080)",   w: 1080, h: 1080 },
    SlideSizePreset { label: "9:16 Portrait (1080 × 1920)", w: 1080, h: 1920 },
];
```

- **Aspect-ratio display helper (`format_aspect_ratio(w: u32, h: u32) -> String`):**
  - Reduces `w / gcd(w, h) : h / gcd(w, h)`.
  - If both reduced terms are `<= 32` (covers `16:9`, `16:10`, `4:3`, `1:1`, `9:16`, `21:9`, `3:2`, `5:4`), returns `"{rw}:{rh}"`.
  - Otherwise formats decimal ratio `"{:.2}:1"` (when `w >= h`) or `"1:{:.2}"` (when `h > w`).
  - Handles `w == 0 || h == 0` safely returning `"—"`.
- **Default slide size persistence (`apps/coursecraft/src/prefs.rs`):**
  - Replace the closed 4-variant `SlidePreset` field type on `Prefs.default_slide` with a backward-compatible `DefaultSlideSize` wrapper (keeping `SlidePreset` enum or serde compatibility for legacy strings `"wide16x9"`, `"wide16x10"`, `"standard4x3"`, `"square"`):
    - Deserializes either a legacy preset string (`"wide16x9"` → `(1280, 720)`, etc.), a new preset string if present, OR `{ "w": u32, "h": u32 }` (clamping/validating against `100..=7680` × `100..=4320`, falling back to `(1280, 720)` on invalid).
    - Serializes legacy presets as their existing string (or `{ "w": w, "h": h }` for new/custom sizes — clean, human-readable, and old `prefs.json` files upgrade seamlessly).
  - `Editor::set_preferences` (`crates/ui-egui/src/preferences.rs:45-52`) accepts any in-bounds `(w, h)` instead of resetting non-4-preset sizes back to `1280×720`.
  - `apps/coursecraft/src/main.rs:1354` stores any valid `(w, h)` into `self.prefs.default_slide` and saves `prefs.json`.
  - `crates/ui-egui/src/preferences.rs:118-123` replaces the 4-radio list with a `styled_combo` preset drop-down + `"Custom"` width/height `DragValue`s (`" px"` suffix), staying compact and consistent with the Slide Size dialog.

### 2.4 Redesigned Slide Size Dialog (`crates/ui-egui/src/slide_size_dialog.rs`)

Built with `Dialog::new("Slide size").open(&mut open).width(500.0).show_with_footer(...)`, `Form`, `styled_combo`, and `palette(ctx)` — strictly compliant with `UX-01`–`UX-22`:

1. **Preset row (`Form`):** Label `"Preset"` → `styled_combo` (`width >= 240 pt`) listing all 7 `PRESETS` plus `"Custom"`. Selecting a preset sets `(w, h)` and updates `locked_ratio = (w as f64) / (h as f64)`.
2. **Dimensions row (`Form`):** Label `"Dimensions"` → `W` `DragValue` (`" px"`, `100..=7680` clamped on display, accepts typing/dragging per `UX-20`), `"×"` muted label, `H` `DragValue` (`" px"`, `100..=4320`).
   - Inline validation message in `p.danger` if manually pushed out of `100..=7680` × `100..=4320`.
3. **Aspect ratio lock row (`Form`):** Checkbox `"Lock aspect ratio"` (checked by default when opening) followed by a subtle pill/caption showing the live ratio (e.g. `"16:9"`).
   - When locked and `W` changes, `H = ((W as f64) / locked_ratio).round().clamp(100.0, 4320.0) as u32` (and reciprocally when `H` changes, `W = ((H as f64) * locked_ratio).round().clamp(100.0, 7680.0) as u32`).
   - Toggling lock ON captures the current `w / h` as `locked_ratio`.
4. **Default-for-new-courses row:** Secondary button `"Use as default for new courses"` (disabled when `!state.valid()` or `(state.w, state.h) == editor.ui.prefs.default_slide`, with explicit `.on_disabled_hover_text(...)` per `UX-08`: `"Already the default for new courses"` or range error). Clicking updates `editor.ui.prefs.default_slide = (state.w, state.h)` and emits `EditorRequest::SetPreferences { ... }` immediately (showing a subtle inline `"Saved as default"` confirmation in `p.success`).
5. **Content behaviour section:** Heading `"Existing content"` + 3 radio options (sentence case, no trailing colons, `UX-11`):
   - **Scale to fit** (`"scale"`) — `"Fits every slide inside the new size, adding margins if the aspect ratio changes."`
   - **Fill slide** (`"fill"`) — `"Fills the new size edge to edge; edges may extend past the slide if the aspect ratio changes."`
   - **Keep as is** (`"keep"`) — `"Changes only the slide size; objects stay at their current positions and sizes."`
6. **Live placement preview (`220 × 132 pt` card):**
   - Painted directly with `ui.painter_at(preview_rect)` using `palette(ctx)` (works identically in Dark & Light modes, `UX-07`):
     - Neutral canvas backdrop (`p.app` / `p.input` with `1 pt` `p.border`).
     - Drawn at uniform fit scale covering the union of the new slide rectangle and the projected old slide rectangle:
       - New slide boundary filled with the current slide's effective background (`course.slide_background(slide)`) and stroked with `1.5 pt` `p.accent`.
       - Projected old slide area shown with subtle dashed outline (clearly revealing letterbox/pillarbox bands in `"scale"` mode or overflow bands in `"fill"` mode).
       - Current slide's visual objects projected into preview coordinates (simple tinted rounded rectangles: `p.accent.gamma_multiply(0.35)` when inside new slide, `p.warning.gamma_multiply(0.55)` when partially clipped, `p.danger.gamma_multiply(0.65)` when completely outside!).
   - Beside/below the preview box, a live impact summary computed across **the whole course** (cached on `(revision, w, h, mode)` so idle frames do 0 recomputation!):
     - Same dimensions: `"Current size (no change)."`
     - Same aspect ratio: `"Scales cleanly at {pct}% with no cropping or margins."` (for `"scale"`/`"fill"`).
     - Aspect ratio changes: states `"Adds top/bottom margins"` / `"Adds left/right margins"` (`"scale"`), or `"Crops left/right edges"` / `"Crops top/bottom edges"` (`"fill"`), plus if any visual objects across the course will land **outside** or **clipped**, displays a `p.warning` line: e.g. `"{N} object(s) will sit outside the slide"` / `"{M} object(s) will extend past an edge"`.
7. **Footer (`DialogUi::footer`):** Right-aligned `"Cancel"` and primary accent `"Apply"` (disabled when `!state.valid()` or `(w, h) == current_size`, with `.on_disabled_hover_text(...)` per `UX-08`). Pressing `Enter` when valid applies; `Esc` closes only open combo first, then dialog (`UX-13`).

### 2.5 Discoverability (Five Entry Points, CourseCraft's Own Style)

| # | Surface | File & Location | Exact UI |
|---|---|---|---|
| 1 | **File menu** | `crates/ui-egui/src/menu.rs:671` | `"Slide size…"` (sentence case fix from `"Slide Size…"` per `UX-11`). |
| 2 | **Slide menu** | `crates/ui-egui/src/menu.rs` (`render_extra_category_items` for `"Slide"`) | Separator + `"Slide size…"` button (natural home next to Slide Master / layout items). |
| 3 | **Course Inspector tab** | `crates/ui-egui/src/properties.rs:4258-4301` (`show_course_tab`) | `inspector_row(ui, "Slide size", ...)` displaying `"{w} × {h} px ({ratio})"` + compact `"Change…"` button right above the `"Course settings…"` / `"Theme…"` row. |
| 4 | **Course Settings dialog** | `crates/ui-egui/src/course_settings.rs:272-350` (`General` section) | `form.row(ui, "Slide size", ...)` displaying `"{w} × {h} px ({ratio})"` + `"Change…"` button that opens Slide Size dialog. |
| 5 | **Status bar** | `crates/ui-egui/src/status.rs:230-350` | Quiet right-aligned pill/button showing `"{w} × {h} px"` (`>= 24 pt` tall, `UX-05`) with hover tooltip `"Slide size ({ratio}) — click to change"`; clicking calls `editor.open_slide_size_dialog()`. |

### 2.6 Check Course: Off-Slide Object Warnings (`crates/course/src/validate.rs`)

- **Rule:** In `check_slide` (`validate.rs:685-713`) and `check_master` (`validate.rs:828-830`), check every visual object (`!matches!(o.kind, ObjectKind::Audio { .. })` and `o.panel.is_none()`):
  - Compute the object's rotated axis-aligned bounding box `[min_x, max_x] × [min_y, max_y]` around its frame centre using `o.rotation`.
  - If the bounding box has **zero overlap** with `[0, slide_w] × [0, slide_h]` (`max_x <= 0.0 || min_x >= slide_w || max_y <= 0.0 || min_y >= slide_h`):
    - Emit `Severity::Warning` at path `"{obj_p}/frame"` (e.g. `/scenes/0/slides/0/objects/2/frame`, `/scenes/0/slides/0/layers/0/objects/1/frame`, or `/masters/0/objects/0/frame`) with message:
      `format!("object {:?} lies outside the {}×{} slide", label, sw, sh)`
- **Why completely-outside (not partial overhang):** Audited all 5 bundled examples (`hello-course`, `showcase`, `reference-course`, `rich-states`, `healthy-habits`): `examples/healthy-habits/course.json` has 5 intentionally overlapping/bleeding objects (`partial=5`, `outside=0`), and all 5 examples have `outside=0`. Warning on zero-overlap catches every object stranded off-stage after a `"keep"` or `"fill"` resize with zero false positives on full-bleed backgrounds or PPTX imports.
- **Navigation:** `crates/ui-egui/src/status.rs:12-154` (`navigate_to_issue_path`) already parses `/scenes/{si}/slides/{li}/objects/{oi}/...`, `/layers/{lyi}/objects/{oi}/...`, `/banks/{bi}/slides/{si}/...`, and `/masters/{mi}/objects/{oi}/...`, so clicking the warning in **Check Course** immediately jumps to and selects the stranded object!

---

## 3. Part B — Bug Hunt on 0.4.0

Focused audit of 0.4.0's newest code: M29 rich object states (canvas editing, state items, States tab, layered resolver, engine commands), seekbar behaviour (`PlayerSettings.seekbar`), web Play start gate (`PlayerFrame.awaiting_start`, `draw_start_gate`), and slide-size interactions. **8 real bugs found — zero padding.**

| ID | Severity | Status | Location | Summary | Fix Step |
|---|---|---|---|---|---|
| **BUG-01** | **High (Data Loss)** | **Confirmed live** (`coursecraft-cli run`) | `crates/engine/src/commands.rs:7689-7694` | `object.patch_state` with `reset_all: true` ("Reset overrides" banner button) **deletes all `StateExtra` items** in that state! | `S6` |
| **BUG-02** | **High (Web Blocker)** | **Confirmed by code audit** (WASM DOM unrun in headless CLI) | `apps/coursecraft-player/src/web_object.rs:44-55, 152`, `app.rs:583, 1258`, `crates/render/src/media_driver.rs:546-554` | HTML `<iframe>` Web objects (`z-index: 10`) render **on top of the Web Start Gate**, blocking clicks on the Play button and loading third-party pages before Start; also cover the Ended card and open drawer. | `S8` |
| **BUG-03** | **High (Visual Corruption)** | **Confirmed live** (`coursecraft-cli run`) | `crates/engine/src/slide_size.rs:50-148` | `course.set_slide_size` (`mode: "scale"`) leaves all M29 `StateDef` overrides (`outline`, `text_style`, `runs`, `paragraphs`, `shadow`, `glow`) and `StateExtra` frames/effects/text unscaled — plus misses base `Text.outline`, `Image.outline`, `Path.text`, `Timer.style`, `Marker.runs/paragraphs`, `insets`, `letter_spacing`, `bullet_size`. | `S1` |
| **BUG-04** | **Medium (Stale UI / Silent Overwrite)** | **Confirmed by code audit** | `crates/ui-egui/src/properties.rs:62, 343, 2708-2726` | `show_state_extra_properties` caches edit buffers on `(revision, extra.id)` without `state_name`. After `object.duplicate_state` (which clones `extra_1` with the same ID), switching states at the same revision displays the previous state's extra fields and can overwrite the new state's extra! | `S7` |
| **BUG-05** | **Medium (Broken Workflow)** | **Confirmed by code audit** | `crates/ui-egui/src/states.rs:1108-1120` + `crates/ui-egui/src/lib.rs:4059-4080` | Renaming the active state in the `States` tab while in Canvas Edit-State Mode updates `preview_state` only, not `editing_state.state_name`, causing the next frame's validity check (`lib.rs:4065-4079`) to kick the author out of Edit-State Mode and clear `preview_state`. | `S7` |
| **BUG-06** | **Medium (Wrong Rendering)** | **Confirmed by code audit** | `crates/render/src/paint.rs:1842-1844, 1953-1955` + `crates/render/src/view.rs:312, 355, 586-591` | During a pointer press (`is_pressed && is_hovered`), `paint.rs`'s `solid_override` checks `(is_pressed && down_fill.is_some()) || (is_hovered && hover_fill.is_some())`. When `hover` has solid `fill` and `down` has a gradient/picture/pattern `fill_style`, pressing wipes out `down`'s `fill_style`! | `S6` |
| **BUG-07** | **Medium (Incomplete M29)** | **Confirmed by code audit** | (a) `crates/render/src/compose.rs:1769-1797`<br>(b) `crates/ui-egui/src/canvas.rs:2402-2405, 2892-2897, 4360-4365, 5855-5870` | (a) `ObjectKind::Timer` ignores `state_overrides.text_style` completely.<br>(b) Inserting (`place_object`, draw path) or inline-editing (`inline_edit_slide_rect`) a `StateExtra` on a rotated/flipped parent ignores parent rotation/flip; live parent rotate drag (`canvas.rs:4360-4364`) overwrites extra `item.rotation` directly. | `S6` (a), `S7` (b) |
| **BUG-08** | **Medium (Runtime / Chrome)** | **Confirmed by code audit** | (a) `crates/logic/src/run.rs:3078-3114`<br>(b) `crates/render/src/player_frame.rs:567, 944-992, 2674` | (a) `seek_base` never calls `self.note_timeline_completed()` when seeking forward to `duration_ms`; under `seekbar: free` + `next_requires: timeline_ends`, scrubbing to the end then back re-locks Next.<br>(b) `handle_keyboard` runs even while the Start Gate is active (pressing `C` toggles `captions_on` behind the gate), and `draw_start_gate` uses `layout_no_wrap` (overflows on narrow mobile widths with long localized `startCourse`). | `S9` (a), `S8` (b) |

### 3.1 Detailed Evidence & Repro Sketches

#### `BUG-01` — `object.patch_state` with `reset_all: true` deletes all `StateExtra`s (`crates/engine/src/commands.rs:7689-7694`)
- **Code:**
  ```rust
  if reset_all {
      *target = StateDef {
          name: target.name.clone(),
          ..Default::default()
      };
  }
  ```
- **Why it is wrong:** `StateDef::default()` sets `extras: Vec::new()`. The M29 spec (`docs/testing/m29-plan.md:189`) explicitly requires `reset_all: true` to preserve `name` and `extras`, and the Edit-State banner button (`canvas.rs:6927`) is labelled `"Reset overrides"`. Existing test `E-10` (`crates/engine/src/m29_state_commands_tests.rs:508-547`) was written in Slice 2 before `extras` existed, so it never tested a state with extras.
- **Confirmed live:** Ran `coursecraft-cli run examples/rich-states/course.json` with `[{"command":"object.patch_state","params":{"slide":"s01-character","object":"obj-character","name":"hover","reset_all":true}}]` → `hover.extras` dropped from **`2`** to **`0`**.
- **Failing-test sketch (`B-01`):** Add an extra to `hover` alongside `fill`/`glow` overrides; run `object.patch_state { reset_all: true }`; assert `fill`/`glow` are `None` AND `extras.len() == 1` (fails today with `extras.len() == 0`).

#### `BUG-02` — HTML `<iframe>` Web objects cover the Web Start Gate (`apps/coursecraft-player/src/web_object.rs:44-55, 152` & `app.rs:1258`)
- **Code:** `Playing::show_ui` (`app.rs:1258`) calls `playing.web_objects.layout(&playing.player, &playing.view)` unconditionally every frame. `WebObjects::layout` (`web_object.rs:47`) checks only `if player.lightbox_open()`, then creates an `<iframe>` at `z-index: 10` (`web_object.rs:152`) and positions it at `item.rect` (`web_object.rs:120-131`).
- **Why it is wrong:** While `awaiting_start == true`, `SlideView` still paints the slide's `Visual::Web` placeholder under the dimmed Start Gate (`z-index < 5`), so `view.last_items()` contains `Visual::Web`. `WebObjects::layout` creates and displays the real `<iframe>` at `z-index: 10` (above both the canvas and `coursecraft_a11y`'s `z-index: 5`), covering the Start Gate Play button and executing third-party iframe content before the learner clicks Play. The same leak occurs when `playing.ended == true` (covering the "Course complete" card) or `playing.frame.drawer_open == true` (covering the outline drawer). Likewise, `MediaDriver::layout_videos` (`media_driver.rs:546-554`) only checks `player.lightbox_open()`, leaving active `<video>` elements visible above the open drawer or Ended card.
- **Repro / failing-test sketch (`B-07`, `B-08`):** Unit test `media_driver.layout_videos_hidden(...)` with `hidden = true` (simulating `awaiting_start || ended || drawer_open`) asserts `backend.layout_video(id, None)` is called; pure helper `should_hide_dom_overlays(player, frame, ended)` tested in `apps/coursecraft-player` asserts `true` when `awaiting_start`, `ended`, `drawer_open`, or `lightbox_open`.

#### `BUG-03` — `course.set_slide_size` ignores M29 `StateDef`/`StateExtra` and several base visual fields (`crates/engine/src/slide_size.rs:50-148`)
- **Confirmed live:** Ran `coursecraft-cli run examples/rich-states/course.json` with `course.set_slide_size { w: 1920, h: 1080, mode: "scale" }` → `obj-character` frame scaled `1.5×` (`480,180,320,360 → 720,270,480,540`), while `hover.extras[0].frame` remained `{x:250, y:-40, w:180, h:72}` and `hover.glow.size` remained `14.0`.
- **Failing-test sketch (`A-02`, `A-03`):** Scale a course containing `StateDef` (`outline`, `text_style`, `runs`, `paragraphs`, `shadow`, `glow`), `StateExtra` (`Shape`, `Image`, `Text`, `Path` with negative overhanging `x/y`), `Text.outline`, `Image.outline`, `Path.text`, `Timer.style`, `Marker.runs/paragraphs`, `insets`, `letter_spacing`, and `bullet_size` by `1.5×` → assert every field scales by `1.5×` and `undo()` restores exact JSON.

#### `BUG-04` — Stale `extra_cache_key` in Format Inspector (`crates/ui-egui/src/properties.rs:62, 343, 2708-2726`)
- **Code:** `if props.extra_cache_key.0 != rev || props.extra_cache_key.1 != extra.id` caches only `(revision, extra_id)`.
- **Why it is wrong:** `object.duplicate_state` (`commands.rs:7822`) clones `extras` verbatim with identical IDs (e.g. `"extra_1"`). Switching from `"hover"`'s `"extra_1"` to `"visited"`'s `"extra_1"` without editing the course keeps `revision` unchanged, so `props.extra_cache_key` matches and `show_state_extra_properties` displays `"hover"`'s `name`/`x`/`y`/`w`/`h`/`rotation`/`opacity`/`text`/`alt` for `"visited"`'s extra. Any focus loss or commit writes `"hover"`'s stale values into `"visited"`.
- **Failing-test sketch (`B-04`):** Create object with `hover` (`extra_1`, `x: 10`, `text: "Hover"`) and `visited` (`extra_1`, `x: 90`, `text: "Visited"`). Enter Edit-State Mode on `hover`, select `extra_1`, run frame (`props.extra_x == 10.0`). Switch `editing_state.state_name = "visited"`, select `extra_1`, run frame → assert `props.extra_x == 90.0` and `props.extra_text == "Visited"` (fails today: stays `10.0` and `"Hover"`).

#### `BUG-05` — Renaming active state in Edit-State Mode exits Edit-State Mode (`crates/ui-egui/src/states.rs:1108-1120`)
- **Code:** `states.rs:1117-1119` updates `editor.ui.preview_state = Some(trimmed)`, but never updates `editor.ui.editing_state.state_name`. On the next frame `lib.rs:4065-4079` sees the old `state_name` missing, sets `self.ui.preview_state = None`, and calls `self.exit_edit_state()`.
- **Failing-test sketch (`B-05`):** Enter Edit-State Mode on `"active"`, perform inline rename to `"chosen"` in `BottomTab::States`, run 2 frames → assert `editor.ui.editing_state.as_ref().map(|s| s.state_name.as_str()) == Some("chosen")` and `editor.ui.preview_state.as_deref() == Some("chosen")` (fails today: both become `None`).

#### `BUG-06` — Solid `hover` fill kills gradient/picture `down` `fill_style` on press (`crates/render/src/paint.rs:1842-1844, 1953-1955`)
- **Code:** `SlideView` (`view.rs:312, 355, 586-591`) sets BOTH `highlight.hovered = Some(Hit::Object(id))` AND `highlight.pressed = Some(Hit::Object(id))` while pressed. `paint.rs` computes:
  ```rust
  let solid_override = !disabled
      && ((is_pressed && down_fill.is_some())
          || (is_hovered && hover_fill.is_some()));
  ```
- **Why it is wrong:** When `hover` has solid `fill: Some("#00FF00")` (`hover_fill = Some(...)`) and `down` has gradient `fill_style: Some(Linear { .. })` (`down_fill = None`), `compose.rs` resolves `down`'s gradient into `Visual::Shape.fill_style`, then `paint.rs` sets `solid_override = true` because of `is_hovered && hover_fill.is_some()` — skipping `paint_gradient_fill` and painting a flat colour instead!
- **Failing-test sketch (`B-03`):** Offscreen-render a `Shape` (and `Button`) with `hover: { fill: "#00FF00" }` and `down: { fill_style: Linear(#FF0000 → #0000FF) }` under `Highlight { hovered: Some(obj), pressed: Some(obj) }` and `PointerState::Pressed` → assert left and right sides of the shape have distinct red/blue gradient pixels (fails today: flat solid base fill!).

#### `BUG-07` — `Timer` ignores `state.text_style`, and `StateExtra` placement/inline-edit/rotate ignores parent transform
- **(7a) `crates/render/src/compose.rs:1769-1797`:** `ObjectKind::Timer` passes base `style` instead of `state_overrides.text_style.unwrap_or(style)` into `resolve_text_runs` and `Visual::Timer { align, valign }`.
- **(7b) `crates/ui-egui/src/canvas.rs:2402-2405, 2892-2897, 4360-4365, 5855-5870`:**
  - `place_object` (`2402-2405`) and draw-path placement (`2892-2897`) compute `(frame.min.x - parent_frame.x)` instead of calling `world_to_extra_frame(&parent, &session.state_name, frame)`, so adding an extra on a rotated/flipped parent drops it at the wrong slide coordinates.
  - `inline_edit_slide_rect` (`5855-5870`) computes `object.frame.x + extra.frame.x` instead of `state_extra_world_geometry(object, state_name, extra)`, misplacing the inline text editor on rotated/flipped parents.
  - Live parent rotate drag (`4360-4364`) sets `item.rotation = rotation` on every `DrawItem` matching `Hit::Object(id)`, including `is_state_extra` items, detaching extras visually during drag.
- **Failing-test sketches (`B-02`, `B-06`):** Compose a `Timer` with a `"warning"` state overriding `text_style` (`size: 36, bold: true, color: "#FF0000"`) → assert resolved run has size `36` and red bold. Place a `Shape` extra on a `90°`-rotated parent at slide pos `P` → assert composed `state_extra_world_geometry` centre matches `P`.

#### `BUG-08` — `seek_base` skips `note_timeline_completed()`, and Start Gate leaks `C` key & overflows narrow labels
- **(8a) `crates/logic/src/run.rs:3078-3114`:** Seeking forward to `duration_ms` fires `timeline_ends`, but `seek_base` never calls `self.note_timeline_completed()` (unlike `tick` at `:2740`). Under `seekbar: free` + `next_requires: timeline_ends`, seeking to the end then scrubbing back to `500 ms` re-locks `Next`! *(Note: `timeline_completed` across suspend/resume is explicitly specified in `docs/spec/triggers.md:326, 580` as runtime-only with `visited - {current}` fallback, so we do NOT break the `cc1:` wire format; we only fix the `seek_base` omission and document it in `triggers.md:580`.)*
- **(8b) `crates/render/src/player_frame.rs:567, 944-992, 2674`:** `PlayerFrame::show` calls `self.handle_keyboard(...)` even when `gated == true`, letting `C` toggle `self.captions_on` before Start. And `draw_start_gate` uses `painter.layout_no_wrap(...)` instead of `painter.layout(..., (rect.width() - 32.0).max(60.0))`, overflowing on narrow phones with long localized `startCourse`.
- **Failing-test sketches (`B-09`, `B-10`, `B-11`):** With `seekbar: free` and `next_requires: timeline_ends`, send `PlayerSeek { ms: 2000 }` then `PlayerSeek { ms: 500 }` → assert `player.can_next() == true` (fails today: `false`). In `PlayerFrame` with `awaiting_start = true`, press `Key::C` → assert `frame.captions_on` stays `false` (fails today: flips to `true`). Render start gate at `320×568` with a 40-char `startCourse` → assert label rect stays inside `slide_rect`.

---

## 4. Part C — Three Small, High-Value Authoring Features

Every item below is chosen from `docs/roadmap.md` or direct authoring gaps observed in the codebase, stays strictly outside "Not planned for now", and fits in 1 small step each.

### 4.1 Feature C1 — Find, Replace, Spell Check & XLIFF Translation in Glossary Entries (`S10`)
- **Source:** `docs/roadmap.md:327` ("Find and Replace in glossary entries").
- **Why authors notice:** Glossaries hold domain terms, product names, and acronyms. Today `course_strings` (`crates/course/src/strings.rs`) and `xliff.rs` (`crates/format/src/xliff.rs`) walk slides, layers, objects, states, state extras, questions, and masters — but completely skip `course.glossary`. Rebranding a product name or exporting XLIFF for translation misses the entire glossary!
- **Design:**
  - `crates/course/src/strings.rs`: emit `glossary/{id}/term` (label `"Glossary ▸ {term} ▸ Term"`) and `glossary/{id}/definition` (label `"Glossary ▸ {term} ▸ Definition"`) in `walk_course` (`slide: None, object: None, layer: None`) and resolve both in `string_mut` (`StringTarget::Plain`). Note: `course.replace` rejects an empty replacement on `term` if it would break `validate.rs:2933` (`keep_valid` already protects against emptying a glossary term!).
  - **Free bonus:** Because `crates/ui-egui/src/spell_check.rs` walks `course_strings`, **Check Spelling (`F7`)** automatically checks glossary terms and definitions too!
  - `crates/format/src/xliff.rs`: export & import `glossary/{id}/term` and `glossary/{id}/definition` units so `crates/format/tests/xliff_parity.rs` (`walker_and_xliff_agree`) stays 100% in sync.
  - `crates/ui-egui/src/find.rs:405-452` (`go_to`): when `hit.field.starts_with("glossary/")`, open Course Settings (`editor.ui.show_course_settings_window = true`) and set a one-shot `editor.ui.course_settings.open_resources = true` flag so `"Resources & glossary"` expands immediately (`resources_ui.rs:82-86` uses `.open(Some(true))` on that frame).

### 4.2 Feature C2 — Glossary Alphabetical Sort (`glossary.sort`) & Resources/Glossary Polish (`S11`)
- **Why authors notice:** Adding 20+ glossary terms over the life of a project leaves them in insertion order; today an author must click `↑` dozens of times in Course Settings to alphabetize them, and `resources_ui.rs` still uses raw `egui::ComboBox` and tiny `< 24 pt` order buttons missing disabled tooltips.
- **Design:**
  - `crates/engine/src/commands.rs`: add `glossary.sort` (`mutates: true`, label `"Sort glossary"`, params `{}`) that stably sorts `s.course.glossary` by `term.to_lowercase()` (tie-breaking on `term`, then `id`); sets `s.skip_undo = true` if order didn't change; single undo step; registered in `REGISTRY`, `docs/cli.md`, `m27_all_commands_undo_tests.rs`.
  - `crates/ui-egui/src/resources_ui.rs`: add `"Sort A–Z"` button beside `"Add term"` (disabled with `.on_disabled_hover_text(...)` when `< 2` terms or already sorted); migrate `resource_kind` combo to `crate::widgets::styled_combo`; set `min_size(vec2(24.0, 24.0))` and `.on_disabled_hover_text(...)` on `↑`, `↓`, and `✕` buttons (`UX-05`, `UX-08`).

### 4.3 Feature C3 — Timeline `"Show all"` & `"Unlock all"` Quick Actions (`S12`)
- **Why authors notice:** When editing dense layered slides, authors routinely hide or lock 5–10 background/overlay objects in the Timeline. Today revealing or unlocking them requires clicking each eye/lock icon one by one (producing 5–10 separate undo steps), even though `object.set_editor_hidden` and `object.set_locked` (`crates/engine/src/commands.rs:5650-5689`) already support `"objects": id[]` in one atomic undo step!
- **Design:**
  - `crates/ui-egui/src/timeline.rs`: in the Timeline header bar (right of the playhead readout), when the current slide/layer has `hidden_count > 0` or `locked_count > 0`, show compact quiet buttons `"Show all ({hidden_count})"` (icon `Icon::Eye`) and `"Unlock all ({locked_count})"` (icon `Icon::LockOpen`), each dispatching one atomic `object.set_editor_hidden { slide, objects, hidden: false }` or `object.set_locked { slide, objects, locked: false }`.
  - `crates/ui-egui/src/menu.rs` (`View` menu): add `"Show all hidden objects"` and `"Unlock all objects"` (enabled when count > 0, disabled with `"No hidden objects on this slide"` / `"No locked objects on this slide"` tooltips per `UX-08`).

---

## 5. Normative Rules (`M30-01`–`M30-16`)

| Rule ID | Category | Normative Requirement |
|---|---|---|
| **M30-01** | CLI/MCP Compatibility | `course.set_slide_size` accepts `"scale"`, `"fill"`, and `"keep"`; existing `"scale"` and `"keep"` callers remain 100% compatible; bounds stay `100..=7680` × `100..=4320`; unchanged size is a no-op (`skip_undo = true`). |
| **M30-02** | Complete Content Scaling | `"scale"` and `"fill"` scale every spatial/typographic property in the model: frames, shadows, glows, motion paths, outlines (`Text`/`Shape`/`Image`/`Path`/`Table`), text sizes, letter spacing, insets, paragraph spacing, bullet sizes, `Timer.style`, `Marker.runs/paragraphs`, `Path.text`, `ScrollPanel.content_height`, `HotspotRegion`, `Guide`, `NamedTextStyle`, and **all M29 `StateDef` overrides & parent-relative `StateExtra` frames/effects/kinds**. |
| **M30-03** | Exact Resize Undo | Every `course.set_slide_size` (`scale`, `fill`, `keep`) is one atomic undo step; `undo()` restores the exact byte-for-byte prior `Course` JSON (including all states and extras). |
| **M30-04** | Off-Slide Warnings | `validate()` emits `Severity::Warning` (never Error) when a non-Audio, non-ScrollPanel-child visual object's rotated bounding box has zero overlap with `[0, slide_w] × [0, slide_h]`. All 5 bundled example courses emit 0 off-slide warnings. |
| **M30-05** | Default Slide Size Persistence | `prefs.json` loads legacy `SlidePreset` strings (`wide16x9`, `wide16x10`, `standard4x3`, `square`) AND arbitrary valid `{w, h}` custom sizes; invalid/out-of-range sizes fall back cleanly to `(1280, 720)`. |
| **M30-06** | Aspect Lock & Live Preview | Slide Size dialog computes exact GCD/decimal aspect ratios, maintains locked ratio while editing `W` or `H`, and renders an accurate live preview + course-wide off-slide/clip count without per-frame course cloning. |
| **M30-07** | Multi-Surface Discoverability | Slide Size dialog is reachable from File menu, Slide menu, Course Inspector tab, Course Settings dialog, and Status bar (`"{w} × {h} px"` pill). |
| **M30-08** | `reset_all` Preserves Extras | `object.patch_state` with `reset_all: true` resets every visual override and `clear_*` flag while preserving `name` and all `StateExtra` items intact. |
| **M30-09** | Web Start Gate Isolation | While `awaiting_start == true` (or `ended` / `drawer_open` / `lightbox_open`), DOM `<iframe>` web objects and `<video>` overlays stay hidden; keyboard shortcuts (`C`, `M`, arrows) do not mutate player state behind the Start Gate; localized `startCourse` wraps cleanly at `320×568`. |
| **M30-10** | Layered Hover/Down Integrity | Pressing an object whose `hover` has solid `fill` and whose `down` has a `fill_style` (gradient/picture/pattern) renders `down`'s `fill_style`, never letting `hover_fill` suppress it. |
| **M30-11** | Edit-State Mode Integrity | `show_state_extra_properties` never serves stale buffers across states sharing an extra ID; renaming the active state keeps Edit-State Mode open; `StateExtra` placement, inline editing, and parent rotation respect parent transforms; `Timer` honours `state.text_style`. |
| **M30-12** | Seekbar Completion Consistency | Forward `PlayerSeek` reaching `slide.duration_ms` records `timeline_completed` identically to `Tick`, so scrubbing back never re-locks `Next` under `next_requires: timeline_ends`. |
| **M30-13** | Glossary Full-Pipeline Parity | `course_strings`, `string_mut`, `export_xliff`, `import_xliff`, `xliff_parity.rs`, Find & Replace, and Check Spelling (`F7`) all cover `glossary/{id}/term` and `glossary/{id}/definition`. |
| **M30-14** | Atomic Glossary Sort & Timeline Bulk Actions | `glossary.sort`, Timeline `"Show all"`, and Timeline `"Unlock all"` each execute as a single atomic undoable engine command and pass `m27_all_commands_undo_tests.rs` and `panic_hunt`. |
| **M30-15** | Zero Logic Regression | `coursecraft-cli simulate` output on `examples/hello-course`, `examples/showcase`, `examples/reference-course`, and `examples/rich-states` is unchanged. |
| **M30-16** | UI/UX Sweep Compliance | Redesigned Slide Size dialog, updated Preferences, Course Inspector, Course Settings, Status Bar, Resources/Glossary, and Timeline header pass `uiux_sweep_all_surfaces` and `uiux_player_sweep` with **0 MUST violations**. |

---

## 6. Watchlist of Risky Spots (`RISK-01`–`RISK-10`)

| ID | File & Lines | Risk & Mitigation |
|---|---|---|
| **RISK-01** | `crates/ui-egui/src/tests.rs:17141, 17146, 17230`, `bughunt_m26_a6_dialogs.rs:182` | Existing tests click `"Slide Size…"` (Title Case) and radio labels `"4:3 (960 x 720)"` / `"1:1 (1080 x 1080)"`. When migrating to sentence case `"Slide size…"` and `styled_combo` presets, update those tests in the same step (`S3`/`S4`/`S5`) so CI never breaks. |
| **RISK-02** | `crates/engine/src/slide_size.rs:181-188` | In `"fill"` mode on extreme aspect changes (e.g. `1280×720 → 100×4320`, factor `6.0`), an object's `w` or `h` could exceed `MAX_OBJECT_FRAME_SIZE` or `coord` could exceed `±100_000`. Always clamp scaled `w/h` to `1.0..=MAX_OBJECT_FRAME_SIZE` and coordinates/motion points/guides to `±100_000.0` so `keep_valid` never rejects a valid resize! |
| **RISK-03** | `crates/course/src/validate.rs` (off-slide warning) | Existing tests or `examples/healthy-habits/course.json` have objects **partially** overlapping the edge (`x = -64, w = 281.6`). Must warn **only** when the rotated AABB has **zero intersection** with `[0, sw] × [0, sh]`, skipping `Audio` and `panel.is_some()`. |
| **RISK-04** | `apps/coursecraft/src/prefs.rs:231, 255-289` | Existing user `prefs.json` files on disk store `"default_slide": "wide16x9"` (or `"standard4x3"`, `"wide16x10"`, `"square"`). Custom deserializer MUST keep accepting all 4 legacy strings AND `{ "w": u32, "h": u32 }`, and existing `apps/coursecraft/src/tests.rs:1376-1458` must stay green. |
| **RISK-05** | `crates/ui-egui/src/status.rs:230-350` | Status bar at narrow `1024×600` (and scale `1.5` / `2.0` in `uiux_sweep_scale_2_and_fixtures`) has tight horizontal width. Put the `"{w} × {h} px"` pill on the right with `ui.with_layout(Layout::right_to_left(...))` only when `ui.available_width() >= 96.0` (or shrink `max_title_w` proportionally) so zero clipping occurs at `1024×600 × 1.5`. |
| **RISK-06** | `crates/render/src/compose.rs:2228-2250` | Changing `question_prompt_rect` or `row_w` for `1280×720` would break pixel-parity tests (`crates/render/tests/parity.rs`). Keep `slide_w >= 800.0` geometry identical; only protect narrow slides (`slide_w < 320.0`) from collapsing to `w == 0`. |
| **RISK-07** | `crates/ui-egui/src/canvas.rs:2362-2424, 2858-2910` | When converting a dragged placement rect into a `StateExtra` on a rotated/flipped parent via `world_to_extra_frame`, ensure `extra.rotation` is initialized to `world_to_extra_rotation(parent, state_name, 0.0)` (or keep axis-aligned in parent space) and verify `pick_state_extra` hits it cleanly right after placement! |
| **RISK-08** | `crates/format/tests/xliff_parity.rs` | `walker_and_xliff_agree` fails immediately if `strings.rs` walks glossary entries without `xliff.rs` exporting them (or vice versa). Land both in the same step (`S10`)! |
| **RISK-09** | `crates/course/src/strings.rs:506-525` + `course.replace` | Replacing a glossary `term` with `""` would violate `validate.rs:2933` (`glossary term is empty`). Verify `course.replace` rejects an empty term via `keep_valid` and restores the course cleanly without mutating. |
| **RISK-10** | `apps/coursecraft-player/src/app.rs` & `web_object.rs` | WASM-only `#[cfg(target_arch = "wasm32")]` code is not executed by native `cargo test`, only compiled by `cargo xtask wasm` / `cargo xtask build-player`. Extract pure decision helper (`should_hide_dom_overlays`) tested on native AND always run `cargo xtask build-player` + `cargo xtask wasm` after `S8`. |

---

## 7. Concrete Test Plan

### 7.1 Part A Tests (`A-01`–`A-18`)

| ID | Crate / File | Behaviour Tested | Expected Result |
|---|---|---|---|
| **A-01** | `crates/engine` (`tests.rs`) | `course.set_slide_size` `"fill"` mode math & centering | `1280×720 → 1000×1000` with `"fill"` uses `factor = max(1000/1280, 1000/720) = 1.3889`, `dx = (1000 - 1280*f)/2 < 0`, `dy = 0`; object at `(0,0,1280,720)` covers full height and overhangs symmetrically left/right; `undo()` restores exact prior JSON. |
| **A-02** | `crates/engine` (`tests.rs`) | `"scale"` & `"fill"` scale M29 `StateDef` overrides & `StateExtra` items (`BUG-03`) | Resize `examples/rich-states/course.json` (and synthetic fixture) `1280×720 → 1920×1080` (`1.5×`): `StateDef.{outline, text_style, runs, paragraphs, shadow, glow}` and `StateExtra.{frame (incl. negative overhanging x/y without +dx/+dy!), shadow, glow, Shape/Image/Text/Path}` all scale by `1.5×`; `undo()` restores exact JSON. |
| **A-03** | `crates/engine` (`tests.rs`) | `"scale"` scales previously missed base object fields (`BUG-03`) | `Text.outline`, `Image.outline`, `Path.text`, `Timer.style`, `Marker.runs/paragraphs`, `TextStyle.{letter_spacing, insets}`, `TextRun.letter_spacing`, `Paragraph.bullet_size`, and `NamedTextStyle.{letter_spacing, insets, bullet_size}` all scale by `factor`, clamped to valid ranges; `Image.crop` remains unchanged. |
| **A-04** | `crates/engine` (`tests.rs`) | Extreme `"fill"` aspect resize stays valid (`RISK-02`) | `1280×720 → 100×4320` and `1280×720 → 7680×100` with `"scale"`, `"fill"`, and `"keep"` all pass `validate()` with 0 errors and round-trip `undo()`. |
| **A-05** | `crates/engine` (`m27_all_commands_undo_tests.rs`) | `course.set_slide_size` (`"fill"`) & `docs/cli.md` sync | `course.set_slide_size` listed in `docs/cli.md` with `"scale"|"fill"|"keep"`; undo/redo exact equality passes. |
| **A-06** | `crates/course` (`validate.rs` tests) | Off-slide object warning after `"keep"` / `"fill"` resize (`M30-04`) | Visual object completely outside `[0, sw] × [0, sh]` (base, layer, master) emits `Severity::Warning` at `.../frame`; partially overlapping object emits **0** warnings; `Audio` and `ScrollPanel` children emit **0** warnings; rotated object whose AABB still touches slide emits **0** warnings. |
| **A-07** | `crates/course` (`validate.rs` tests) | Zero false positives on all 5 bundled examples (`RISK-03`) | Validating `hello-course`, `showcase`, `reference-course`, `rich-states`, and `healthy-habits` emits 0 `"lies outside"` warnings. |
| **A-08** | `crates/render` & `crates/export` | Question layouts, `PlayerFrame`, `render_slide_rgba`, MP4/PDF/Word at all 7 presets + `100×100` | `compose_question`, `PlayerFrame`, and offscreen render succeed with finite non-zero rects and 0 panics at `1920×1080`, `1280×720`, `1280×800`, `960×720`, `1024×768`, `1080×1080`, `1080×1920`, and `100×100`. |
| **A-09** | `apps/coursecraft` (`tests.rs`) | Backward-compatible `Prefs.default_slide` persistence (`M30-05`) | Legacy `prefs.json` (`"wide16x9"`, `"standard4x3"`, `"wide16x10"`, `"square"`) loads accurately; Full HD `(1920, 1080)`, Portrait `(1080, 1920)`, and Custom `(1600, 900)` save & reload; invalid `(50, 99999)` falls back to `(1280, 720)`; `Prefs::blank_course()` uses stored custom size. |
| **A-10** | `crates/ui-egui` | `format_aspect_ratio` unit matrix | `(1920, 1080) → "16:9"`, `(1280, 720) → "16:9"`, `(1280, 800) → "16:10"`, `(960, 720) → "4:3"`, `(1080, 1080) → "1:1"`, `(1080, 1920) → "9:16"`, `(1500, 900) → "5:3"`, `(1001, 700) → "1.43:1"`, `(0, 720) → "—"`. |
| **A-11** | `crates/ui-egui` | Redesigned Slide Size dialog preset combo & Custom switching | Open dialog, select `"16:9 Full HD (1920 × 1080)"` → `(w, h) == (1920, 1080)`, ratio `"16:9"`; select `"9:16 Portrait (1080 × 1920)"` → `(1080, 1920)`; typing custom width switches combo label to `"Custom"`. |
| **A-12** | `crates/ui-egui` | Aspect-ratio lock behaviour (`M30-06`) | With lock ON at `1280×720`, changing `w` to `1920` updates `h` to `1080`; changing `h` to `900` updates `w` to `1600`; unchecking lock lets `w` change independently; re-checking locks new ratio. |
| **A-13** | `crates/ui-egui` | `"Use as default for new courses"` button | Clicking emits `EditorRequest::SetPreferences { default_slide: (1920, 1080), .. }`, updates `editor.ui.prefs.default_slide`, shows `"Saved as default"`, and disables button with explanatory tooltip when already default. |
| **A-14** | `crates/ui-egui` | Live placement preview & course-wide impact counts | Preview reports `"0 outside"` on `1280×720 → 1920×1080 (scale)`, reports margin hint on `1280×720 → 1080×1080 (scale)`, and reports exact off-slide object count on `1280×720 → 640×360 (keep)` — with 0 course copies on idle frames. |
| **A-15** | `crates/ui-egui` | Content mode radios (`Scale to fit`, `Fill slide`, `Keep as is`) dispatch & undo | Selecting each mode and clicking Apply dispatches `"scale"`, `"fill"`, `"keep"` respectively, closes dialog, and `Ctrl+Z` restores prior course size & frames. |
| **A-16** | `crates/ui-egui` | Preferences dialog supports all 7 presets + Custom default size | Selecting `"16:9 Full HD (1920 × 1080)"` or typing custom `(1440, 900)` emits `EditorRequest::SetPreferences` with exact tuple. |
| **A-17** | `crates/ui-egui` | Multi-surface discoverability (`M30-07`) | Opening Slide Size works from: (1) `File ▸ Slide size…`, (2) `Slide ▸ Slide size…`, (3) Course Inspector `"Change…"`, (4) Course Settings `"Change…"`, and (5) Status bar `"1280 × 720 px"` pill. |
| **A-18** | `crates/ui-egui` | Check Course click-to-navigate for off-slide object warning | Resize with `"keep"` to strand an object outside; open Check Course; click the warning row → selects the exact slide, layer, and stranded object. |

### 7.2 Part B Bug-Fix Tests (`B-01`–`B-12`)

| ID | Crate / File | Bug Verified | Expected Result |
|---|---|---|---|
| **B-01** | `crates/engine` (`m29_state_commands_tests.rs`) | `BUG-01` (`reset_all` preserves `extras`) | State with `fill`, `glow`, `clear_shadow: true`, and 2 `StateExtra`s → `object.patch_state { reset_all: true }` clears overrides/flags while keeping both `extras` untouched; `undo()` restores overrides + extras identically. |
| **B-02** | `crates/render` (`m29_state_resolver_tests.rs`) | `BUG-07a` (`Timer` `state.text_style`) | `Timer` with `"warning"` state setting `text_style { size: 36, bold: true, color: "#FF0000", align: Right }` composes `Visual::Timer` with bold red 36 px run and `Align::Right`. |
| **B-03** | `crates/render` (`m29_state_resolver_tests.rs`) | `BUG-06` (`hover` solid fill vs `down` `fill_style`) | `Shape` and `Button` with `hover: { fill: "#00FF00" }` and `down: { fill_style: Linear(#FF0000 → #0000FF) }` rendered with `Highlight { hovered: Some(id), pressed: Some(id) }` + `PointerState::Pressed` draws the red→blue gradient (distinct left/right pixels, not flat colour!). |
| **B-04** | `crates/ui-egui` (`m29_state_extras_ui_tests.rs`) | `BUG-04` (stale `extra_cache_key`) | Duplicate `hover` (with `extra_1` at `x: 10`, `"Hover"`) to `visited`, edit `visited`'s `extra_1` to `x: 90`, `"Visited"`; view `hover/extra_1` then switch to `visited/extra_1` at same revision → Format inspector shows `90` and `"Visited"`, never `"Hover"`. |
| **B-05** | `crates/ui-egui` (`m29_edit_state_mode_tests.rs`) | `BUG-05` (rename while in Edit-State Mode) | Enter Edit-State Mode on `"active"`, rename to `"chosen"` in States tab, advance 2 frames → stays in Edit-State Mode with `editing_state.state_name == "chosen"` and `preview_state == Some("chosen")`. |
| **B-06** | `crates/ui-egui` (`m29_state_extras_ui_tests.rs`) | `BUG-07b` (`StateExtra` on rotated/flipped parent) | On a `90°`-rotated (and `flip_h`) parent, place a `Shape` extra and a `Path` extra at slide point `P` → composed `state_extra_world_geometry` centre matches `P`; double-clicking a text extra opens `inline_edit_slide_rect` at the rotated world rect; live parent rotate drag leaves extra items intact. |
| **B-07** | `crates/render` (`media_driver.rs` tests) | `BUG-02` (`MediaDriver::layout_videos` overlay hiding) | Calling `layout_videos_with_visibility(player, view, true)` (overlays hidden) calls `backend.layout_video(id, None)` for every laid-out video. |
| **B-08** | `apps/coursecraft-player` (`tests.rs`) | `BUG-02` (Start Gate / Drawer / Ended overlay helper) | `should_hide_dom_overlays` returns `true` whenever `awaiting_start`, `ended`, `drawer_open`, or `lightbox_open` is `true`, and `false` during normal play. |
| **B-09** | `crates/render` (`player_frame/tests.rs`) | `BUG-08b` (Start Gate keyboard isolation) | With `awaiting_start = true`, pressing `Key::C`, `Key::M`, `ArrowRight`, `ArrowLeft` leaves `frame.captions_on == false`, `frame.drawer_open == false`, and `output.inputs.is_empty()`. |
| **B-10** | `crates/render` (`player_frame/tests.rs`) | `BUG-08b` (Start Gate long label wrapping) | At `320×568` with 40-char `startCourse`, `start_button_rect()` stays completely within `slide_rect` (`UX-04`/`UX-16`). |
| **B-11** | `crates/logic` (`tests.rs`) | `BUG-08a` (`seek_base` records `timeline_completed`) | With `seekbar: free` and `next_requires: timeline_ends`, `PlayerSeek { ms: 2000 }` then `PlayerSeek { ms: 500 }` keeps `can_next() == true` and `Input::Next` advances without warning. |
| **B-12** | `crates/ui-egui` (`course_settings.rs` tests) | Seekbar dropdown gating (`UX-08`) | When `show_bar == false` or `show_seekbar == false`, the `Seekbar` combo in Course Settings is disabled and hovering shows an explanatory tooltip. |

### 7.3 Part C Tests (`C-01`–`C-09`)

| ID | Crate / File | Behaviour Tested | Expected Result |
|---|---|---|---|
| **C-01** | `crates/course` (`strings.rs` tests) | Glossary string walk & `string_mut` | `course_strings` emits `glossary/{id}/term` and `glossary/{id}/definition`; `find_in_course` matches both; `string_mut` mutates both and protects `%var%` references. |
| **C-02** | `crates/engine` (`tests.rs`) | `course.find` & `course.replace` on glossary | Replaces matches in glossary terms/definitions in one undo step; refuses a replacement that would empty a glossary term (`RISK-09`) leaving course untouched. |
| **C-03** | `crates/format` (`xliff_parity.rs` & `xliff.rs` tests) | XLIFF export/import & parity for glossary | `export_xliff` emits `glossary/{id}/term` and `glossary/{id}/definition`; `import_xliff` updates both; `walker_and_xliff_agree` passes on a course with glossary entries. |
| **C-04** | `crates/ui-egui` (`find.rs` & `spell_check.rs` tests) | Find/Replace click-to-open & Spell Check in glossary | Clicking a glossary hit in Find & Replace opens Course Settings with `"Resources & glossary"` expanded; `spell_check` flags misspelled words in glossary definitions. |
| **C-05** | `crates/engine` (`tests.rs`, `m27_all_commands_undo_tests.rs`) | `glossary.sort` command + undo + `docs/cli.md` | Sorts `["Zebra", "alpha", "Beta"]` → `["alpha", "Beta", "Zebra"]` in one undo step; already-sorted list sets `skip_undo = true`; registered in `docs/cli.md` and `panic_hunt`. |
| **C-06** | `crates/ui-egui` | `"Sort A–Z"` button & Resources/Glossary UI rules | Clicking `"Sort A–Z"` dispatches `glossary.sort`; disabled with tooltip when `< 2` terms or already sorted; `↑`/`↓`/`✕` buttons are `>= 24×24 pt` with disabled tooltips. |
| **C-07** | `crates/ui-egui` | Timeline `"Show all"` & `"Unlock all"` quick actions | Hide 3 objects → `"Show all (3)"` appears → clicking unhides all 3 in **1** undo step and hides the button; lock 2 objects → `"Unlock all (2)"` appears → clicking unlocks both in **1** undo step; `Ctrl+Z` restores all 3 hidden / 2 locked. |
| **C-08** | `crates/ui-egui` | `View` menu `"Show all hidden objects"` / `"Unlock all objects"` | Disabled with explanatory tooltips when none exist; enabled and executes 1-undo batch when hidden/locked objects exist. |
| **C-09** | `crates/ui-egui` (`uiux_sweep_tests.rs`) | Full UI/UX matrix sweep (`M30-16`) | `uiux_sweep_all_surfaces`, `uiux_sweep_scale_2_and_fixtures`, `uiux_sweep_modals_and_tooltips`, and `uiux_player_sweep` pass with **0 MUST violations** across all updated surfaces (`slide-size`, `preferences`, `inspector-course`, `course-settings`, status bar, timeline). |

### 7.4 Hostile-Input & Regression Tests (`FZ-01`–`FZ-03`)
- **FZ-01:** `panic_hunt` covers updated `course.set_slide_size` (`"fill"`, huge/negative/wrong-type params) and new `glossary.sort` with 0 panics.
- **FZ-02:** `coursecraft-cli simulate` on `examples/hello-course`, `examples/showcase`, `examples/reference-course`, and `examples/rich-states` produces zero regressions (`M30-15`).
- **FZ-03:** Hostile `prefs.json` values for `default_slide` (negative, 0, `999999`, wrong JSON type, unknown string) never panic and fall back cleanly to `(1280, 720)`.

---

## 8. Numbered Implementer Steps (12 Steps, ~30–40 min each)

> **Reviewer notes (Claude, binding for implementers)**
> - The step list below runs as **14 steps**: S1 splits into **S1a** (shared scale helpers, every missed base field, `StateDef` overrides and `StateExtra` scaling, clamps; modes stay "scale"/"keep") and **S1b** (`ResizeMode` with the new `"fill"` mode, command params, `docs/cli.md`, `docs/spec/course-model.md`). S4 splits into **S4a** (dialog controls: preset combo, W/H, aspect lock and ratio badge, use-as-default, the three content options, footer, Enter/Esc) and **S4b** (the live placement preview and the cached course-wide impact summary).
> - New courses keep **1280 × 720** as the default unless the author picks another default; the new presets only add choices. Existing tests that use the old four sizes must keep passing.
> - **BUG-06 fix, corrected:** while pressed, the `down` state wins only if it defines a fill of its own (solid `fill` *or* `fill_style`). If `down` defines no fill, the hover solid fill keeps showing as today. Test both cases.
> - **BUG-08a:** recording completion on a forward seek to the end only affects `seekbar: free`; `locked` and `until_timeline_ends` already refuse that seek. Add a test that proves it for both.
> - **S12:** if `object.set_editor_hidden` / `object.set_locked` take one object only, do "Show all" / "Unlock all" as one undo step through the engine (a batch command or a list param added to those commands, documented in `docs/cli.md`). Never one undo step per object.
> - Wording in every new string is CourseCraft's own (no text from the tester's screenshot), sentence case.

> **Gate cadence:** Run targeted `cargo test -p <crate> -j 4` on every step. Run full `cargo xtask ci` after **S2, S5, S8, S10, and S12**. Run `cargo xtask build-player` after render/player changes (**S2, S6, S8, and S12**).

### Part A — Slide Size, Done Properly (Steps S1–S5)

- **S1 — Complete `slide_size.rs` Scaling & Add `"fill"` Mode**
  - **Files:** `crates/engine/src/slide_size.rs`, `crates/engine/src/commands.rs` (`:85, 1882-1901`), `crates/engine/src/tests.rs` (`:8180-8335`), `crates/engine/src/m27_all_commands_undo_tests.rs`, `docs/cli.md`, `docs/spec/course-model.md`.
  - **Exact changes:**
    1. Replace `scale: bool` in `slide_size.rs` with `pub(crate) enum ResizeMode { Scale, Fill, Keep }`. Compute `factor = min(...)` for `Scale`, `max(...)` for `Fill`, centred `(dx, dy)` for both.
    2. Extract shared helpers in `slide_size.rs`: `scale_text_style` (scales `size`, `letter_spacing`, `insets.{left,top,right,bottom}`), `scale_runs` (`size`, `letter_spacing`), `scale_paragraphs` (`space_before`, `space_after`, `bullet_size`), `scale_shape_text`, `scale_outline`, `scale_shadow`, `scale_glow`.
    3. Update `scale_object` (`slide_size.rs:77-148`) to scale:
       - `Text.outline`, `Image.outline`, `Path.text`, `Timer.style`, `Marker.{runs, paragraphs}`, plus all fields handled by the shared helpers above;
       - Every `StateDef` in `o.states`: `outline`, `text_style`, `runs`, `paragraphs`, `shadow`, `glow`, and every `StateExtra` in `state.extras` (`frame.x/y` via `m.len(...).round()`, `frame.w/h` via `round_len(m.len(...), 1.0)`, `shadow`, `glow`, and all 4 `StateExtraKind` variants);
       - Clamp scaled frame widths/heights to `1.0..=MAX_OBJECT_FRAME_SIZE` and coordinates/motion points/guides to `-100_000.0..=100_000.0` (`RISK-02`).
    4. Accept `"mode": "fill"` in `course_set_slide_size` (`commands.rs:1888-1892`) alongside `"scale"` and `"keep"`, and update `REGISTRY` params string (`:88`), `docs/cli.md:328`, and `docs/spec/course-model.md`.
  - **Tests:** `A-01`, `A-02`, `A-03`, `A-04`, `A-05`, `FZ-01`.
  - **Done when:** All new & existing `coursecraft-engine` slide-size and `m27_all_commands_undo_tests` pass, and `apps/coursecraft-cli` command-doc sync test passes.

- **S2 — Check Course Off-Slide Object Warnings & Multi-Aspect Render/Export Verification**
  - **Files:** `crates/course/src/validate.rs` (`:644-732, 826-846, 1923-1958`), `crates/render/src/compose.rs` (`:2228-2250`), `crates/render/src/tests.rs`, `crates/export/src/tests.rs`.
  - **Exact changes:**
    1. Add helper `object_completely_outside_slide(frame: Rect, rotation: f32, slide: Size) -> bool` in `validate.rs` computing the rotated 4-corner AABB and checking `max_x <= 0.0 || min_x >= sw || max_y <= 0.0 || min_y >= sh`.
    2. In `check_object` (or `check_slide` / `check_master`), emit `Severity::Warning` at `format!("{p}/frame")` whenever `!matches!(o.kind, ObjectKind::Audio { .. }) && o.panel.is_none()` and `object_completely_outside_slide(o.frame, o.rotation, cx.course.slide_size)` is true.
    3. In `crates/render/src/compose.rs:2228-2232` (`question_prompt_rect`), guard tiny slide widths (`let margin = if slide_w < 320.0 { (slide_w * 0.1).min(80.0) } else { 80.0 };`) while keeping `slide_w >= 320.0` byte-for-byte unchanged (`RISK-06`).
    4. Add multi-aspect tests verifying `compose_question`, `PlayerFrame`, `render_slide_rgba`, and `export` across all 7 presets + `100×100`.
  - **Tests:** `A-06`, `A-07`, `A-08`.
  - **Gate #1:** `cargo xtask build-player` + `cargo xtask ci`.
  - **Done when:** All 5 bundled examples emit 0 off-slide warnings, `"keep"` resize stranding an object emits a navigable warning, and CI is green.

- **S3 — Shared 7-Preset Table, Aspect-Ratio Helper & Custom Default Slide Size Preferences**
  - **Files:** `crates/ui-egui/src/slide_size_dialog.rs` (constants & helpers), `crates/ui-egui/src/preferences.rs`, `apps/coursecraft/src/prefs.rs`, `apps/coursecraft/src/main.rs` (`:380, 1343-1361`), `apps/coursecraft/src/tests.rs`, `crates/ui-egui/src/tests.rs` (`:17220-17248`).
  - **Exact changes:**
    1. Define `SlideSizePreset`, the 7-entry `PRESETS` array (including `1920×1080` and `1080×1920`), and `format_aspect_ratio(w, h)` in `slide_size_dialog.rs`.
    2. Upgrade `Prefs.default_slide` in `apps/coursecraft/src/prefs.rs` so it deserializes legacy kebab-case `SlidePreset` strings (`wide16x9`, `wide16x10`, `standard4x3`, `square`) AND `{ "w": u32, "h": u32 }` custom tuples, clamps/validates to `100..=7680` × `100..=4320` (fallback `(1280, 720)`), and exposes `.size() -> (u32, u32)`.
    3. Update `apps/coursecraft/src/main.rs:1354-1356` to persist any valid `default_slide` tuple.
    4. Update `crates/ui-egui/src/preferences.rs:45-52, 118-123` to accept any valid `(w, h)` and replace the 4 radios with a `styled_combo` preset drop-down + Custom `W` × `H` `DragValue`s (`" px"` suffix). Update `preferences_open_from_the_menu_and_changes_are_requested` (`tests.rs:17220`) to match the combo (`RISK-01`).
  - **Tests:** `A-09`, `A-10`, `A-16`, `FZ-03`.
  - **Done when:** Old `prefs.json` files load identically, custom & Full HD defaults persist and apply to `blank_course()`, and Preferences UI passes tests.

- **S4 — Redesigned Slide Size Dialog (Presets, Aspect Lock, Use as Default & Live Preview)**
  - **Files:** `crates/ui-egui/src/slide_size_dialog.rs`, `crates/ui-egui/src/lib.rs` (`:4774`), `crates/ui-egui/src/tests.rs` (`:17136-17175`), `crates/ui-egui/src/u13_core_authoring_dialogs_tests.rs`.
  - **Exact changes:**
    1. Extend `SlideSizeState` with `mode: ResizeModeChoice` (`Scale` / `Fill` / `Keep`), `lock_aspect: bool` (default `true`), `locked_ratio: f64`, `saved_default_notice: bool`, and cached course-impact summary keyed on `(revision, w, h, mode)`.
    2. Pass `requests: &mut Vec<EditorRequest>` into `show_slide_size_dialog(editor, ctx, requests)` from `lib.rs:4774`.
    3. Build the redesigned dialog using `Dialog::new("Slide size").open(&mut open).width(500.0).show_with_footer(...)`, `Form`, `styled_combo` (7 presets + `"Custom"`), `W`/`H` `DragValue`s (`" px"`), `"Lock aspect ratio"` checkbox + ratio badge, `"Use as default for new courses"` button, `"Scale to fit"` / `"Fill slide"` / `"Keep as is"` radios with helper text, live painted `220×132 pt` preview + course impact summary, and `Cancel` / primary `Apply` footer (`Enter` commits, `Esc` follows popup-first rule).
    4. Update existing `slide_size_dialog_*` tests for the combo/footer controls (`RISK-01`).
  - **Tests:** `A-11`, `A-12`, `A-13`, `A-14`, `A-15`.
  - **Done when:** All Slide Size dialog interactions, aspect locking, default saving, live preview counts, and apply/undo paths pass unit & harness tests.

- **S5 — Slide Size Discoverability Across Editor & UI/UX Sweep**
  - **Files:** `crates/ui-egui/src/menu.rs` (`:671` + Slide menu), `crates/ui-egui/src/properties.rs` (`:4258-4301`), `crates/ui-egui/src/course_settings.rs` (`:272-350`), `crates/ui-egui/src/status.rs` (`:230-350`), `crates/ui-egui/src/bughunt_m26_a6_dialogs.rs` (`:182`), `crates/ui-egui/src/uiux_sweep_tests.rs`.
  - **Exact changes:**
    1. Rename `File ▸ Slide Size…` → `"Slide size…"` (`menu.rs:671`, `UX-11`) and add `"Slide size…"` to the `Slide` menu.
    2. Add `"Slide size"` row (`"{w} × {h} px ({ratio})"` + `"Change…"`) to `show_course_tab` (`properties.rs:4290`) and `show_course_settings_window` General section (`course_settings.rs:345`, adding `"Slide size"` to `Form` labels).
    3. Add right-aligned `"{w} × {h} px"` clickable pill to `show_status_bar` (`status.rs:349`, respecting narrow width `RISK-05`, `>= 24 pt` tall, tooltip `"Slide size ({ratio}) — click to change"`).
    4. Verify `slide-size` and `preferences` in `uiux_sweep_all_surfaces` and `uiux_sweep_scale_2_and_fixtures` (0 MUST violations).
  - **Tests:** `A-17`, `A-18`, `C-09` (Part A surfaces).
  - **Gate #2:** `cargo xtask ci`.
  - **Done when:** All 5 entry points open Slide Size cleanly, clicking an off-slide warning selects the object, and full CI + UI/UX sweep pass with 0 violations.

### Part B — Bug Hunt Fixes on 0.4.0 (Steps S6–S9)

- **S6 — Fix `reset_all` Data Loss (`BUG-01`), `Timer` State Text Style (`BUG-07a`) & Hover/Down Fill Override (`BUG-06`)**
  - **Files:** `crates/engine/src/commands.rs` (`:7689-7694`), `crates/render/src/compose.rs` (`:1769-1797`), `crates/render/src/paint.rs` (`:1842-1844, 1953-1955`), `crates/engine/src/m29_state_commands_tests.rs`, `crates/render/src/m29_state_resolver_tests.rs`.
  - **Exact changes:**
    1. In `object_patch_state` (`commands.rs:7689-7694`), preserve extras:
       ```rust
       if reset_all {
           let extras = std::mem::take(&mut target.extras);
           *target = StateDef {
               name: target.name.clone(),
               extras,
               ..Default::default()
           };
       }
       ```
    2. In `compose_object_opts` (`compose.rs:1769-1797`), resolve `let resolved_style = state_overrides.text_style.unwrap_or(style);` for `ObjectKind::Timer`, apply legacy `state_overrides.fill` only when `state_overrides.text_style.is_none()`, and pass `resolved_style` to `resolve_text_runs`, `align`, and `valign`.
    3. In `paint.rs:1842-1844` (`Visual::Button`) and `1953-1955` (`Visual::Shape`), change `solid_override` to:
       ```rust
       let solid_override = !disabled
           && if is_pressed {
               down_fill.is_some()
           } else {
               is_hovered && hover_fill.is_some()
           };
       ```
  - **Tests:** `B-01`, `B-02`, `B-03`.
  - **Rebuild player:** `cargo xtask build-player`.
  - **Done when:** `B-01`, `B-02`, `B-03` pass and all M29 engine/render tests stay green.

- **S7 — Fix Edit-State Mode Editor Bugs (`BUG-04`, `BUG-05`, `BUG-07b`)**
  - **Files:** `crates/ui-egui/src/properties.rs` (`:62, 343, 2708-2726, 2782-2817`), `crates/ui-egui/src/states.rs` (`:1108-1120`), `crates/ui-egui/src/canvas.rs` (`:2402-2405, 2892-2897, 4360-4365, 5855-5870`), `crates/ui-egui/src/m29_state_extras_ui_tests.rs`, `crates/ui-egui/src/m29_edit_state_mode_tests.rs`.
  - **Exact changes:**
    1. Widen `PropertiesState::extra_cache_key` (`properties.rs:62, 343, 2708`) to `(u64, String, String, String, String)` (`(rev, slide_id, object_id, state_name, extra.id)`) and add `.suffix(" px")` to `extra_x/y/w/h` `DragValue`s (`UX-11`).
    2. In `states.rs:1117-1120`, update `editor.ui.editing_state` (`if let Some(session) = editor.ui.editing_state.as_mut() && session.state_name == state_name { session.state_name = trimmed.clone(); }`) alongside `preview_state`.
    3. In `canvas.rs`, use `world_to_extra_frame` (and `world_to_extra_rotation` where appropriate, `RISK-07`) in `place_object` (`:2402`) and draw-path placement (`:2892`); use `state_extra_world_geometry` in `inline_edit_slide_rect` (`:5855`); skip `item.is_state_extra` in `DragOp::Rotate` live preview (`:4360-4364`).
  - **Tests:** `B-04`, `B-05`, `B-06`.
  - **Done when:** All Edit-State Mode & StateExtra UI tests pass.

- **S8 — Fix Web Start Gate Overlay Blocking (`BUG-02`) & Keyboard/Wrap Issues (`BUG-08b`)**
  - **Files:** `crates/render/src/media_driver.rs` (`:546-579`), `crates/render/src/player_frame.rs` (`:567, 2674`), `apps/coursecraft-player/src/web_object.rs` (`:44-55`), `apps/coursecraft-player/src/app.rs` (`:583, 1249, 1258`), `crates/render/src/player_frame/tests.rs`, `apps/coursecraft-player/src/tests.rs`.
  - **Exact changes:**
    1. Add pure helper `should_hide_dom_overlays(player: &Player<'_>, frame: &PlayerFrame, ended: bool) -> bool` returning `frame.awaiting_start || ended || frame.drawer_open || player.lightbox_open()`.
    2. Pass `hide_overlays: bool` to `WebObjects::layout` (`web_object.rs:44`) — hiding (and skipping creation of) iframes while `hide_overlays` is true — and add `MediaDriver::layout_videos_with_visibility(player, view, hide_overlays)` (`media_driver.rs:546`, keeping `layout_videos` delegating with `false` for backward compatibility) called from `app.rs:583` with `should_hide_dom_overlays(...)`.
    3. In `player_frame.rs:567`, gate `handle_keyboard`: `let mut inputs = if gated { Vec::new() } else { self.handle_keyboard(ui, player, show_menu, show_nav) };`.
    4. In `draw_start_gate` (`player_frame.rs:2674`), replace `layout_no_wrap` with `painter.layout(..., (rect.width() - 32.0).max(60.0))` so long localized `startCourse` labels wrap inside `slide_rect`.
  - **Tests:** `B-07`, `B-08`, `B-09`, `B-10`.
  - **Gate #3:** `cargo xtask build-player` + `cargo xtask ci` (verifies WASM compilation of `web_object.rs` and `app.rs`!).
  - **Done when:** Start Gate tests and full CI + WASM build pass.

- **S9 — Fix Seekbar `seek_base` Timeline Completion (`BUG-08a`) & Course Settings Seekbar Gating**
  - **Files:** `crates/logic/src/run.rs` (`:3078-3114`), `docs/spec/triggers.md` (`:580`), `crates/ui-egui/src/course_settings.rs` (`:471-495`), `crates/logic/src/tests.rs`, `crates/ui-egui/src/tests.rs`.
  - **Exact changes:**
    1. In `crates/logic/src/run.rs:3102` (end of forward branch in `seek_base`, before early return check or right after `self.st.base` update when `self.st.epoch == epoch`), call `self.note_timeline_completed();`.
    2. Update `docs/spec/triggers.md:580` to note completion is also recorded when a forward seek reaches `duration_ms`.
    3. In `crates/ui-egui/src/course_settings.rs:471-495`, wrap the `Seekbar` row's combo in `ui.add_enabled_ui(show_bar && editor.ui.course_settings.show_seekbar, ...)` with `.on_disabled_hover_text(...)` explaining whether the player bar or seekbar is turned off (`UX-08`).
  - **Tests:** `B-11`, `B-12`, `FZ-02` (`simulate` parity).
  - **Done when:** Seekbar completion tests, Course Settings disabled-tooltip test, and all `coursecraft-logic` tests pass.

### Part C — Three Small, High-Value Features (Steps S10–S12)

- **S10 — Find, Replace, Spell Check & XLIFF Translation for Glossary Entries**
  - **Files:** `crates/course/src/strings.rs` (`:178-215, 506-525`), `crates/format/src/xliff.rs`, `crates/format/tests/xliff_parity.rs`, `crates/ui-egui/src/find.rs` (`:405-452`), `crates/ui-egui/src/course_settings.rs`, `crates/ui-egui/src/resources_ui.rs` (`:79-87`), `crates/engine/src/tests.rs`.
  - **Exact changes:**
    1. In `strings.rs`: emit `glossary/{id}/term` and `glossary/{id}/definition` in `walk_course` (after course title / scenes or at end of course) and resolve them in `string_mut`.
    2. In `xliff.rs`: export and import `glossary/{id}/term` and `glossary/{id}/definition` units cleanly, keeping `xliff_parity.rs` green (`RISK-08`).
    3. In `find.rs::go_to`: if `hit.field.starts_with("glossary/")`, set `editor.ui.show_course_settings_window = true` and `editor.ui.course_settings.open_resources = true`; consume `open_resources` in `resources_ui.rs` to open the `"Resources & glossary"` `CollapsingHeader`.
  - **Tests:** `C-01`, `C-02`, `C-03`, `C-04`.
  - **Gate #4:** `cargo xtask ci`.
  - **Done when:** Glossary matches appear in Find & Replace, Spell Check, and XLIFF export/import, clicking a hit opens Course Settings expanded to Glossary, and `xliff_parity` + CI are green.

- **S11 — Glossary Alphabetical Sort (`glossary.sort`) & Resources/Glossary Polish**
  - **Files:** `crates/engine/src/commands.rs` (`:1390-1400, 9235-9255`), `crates/engine/src/m27_all_commands_undo_tests.rs`, `docs/cli.md`, `crates/ui-egui/src/resources_ui.rs`.
  - **Exact changes:**
    1. Add `glossary.sort` command (`mutates: true`, label `"Sort glossary"`, params `{}`): sorts `s.course.glossary` by `(entry.term.to_lowercase(), entry.term.clone(), entry.id.clone())`; sets `s.skip_undo = true` if order is unchanged; register in `REGISTRY`, `docs/cli.md`, and `m27_all_commands_undo_tests.rs`.
    2. In `resources_ui.rs`: add `"Sort A–Z"` button next to `"Add term"` (disabled when `< 2` terms or already sorted, with explanatory tooltip); switch `resource_kind` to `styled_combo`; set `min_size(vec2(24.0, 24.0))` and `.on_disabled_hover_text(...)` on `↑`/`↓`/`✕` buttons (`UX-05`, `UX-08`).
  - **Tests:** `C-05`, `C-06`, `FZ-01`.
  - **Done when:** `glossary.sort` passes undo/redo, `panic_hunt`, CLI docs sync, and UI tests.

- **S12 — Timeline `"Show all"` / `"Unlock all"` Quick Actions & Final M30 Release Gate**
  - **Files:** `crates/ui-egui/src/timeline.rs` (`:490-530`), `crates/ui-egui/src/menu.rs` (`View` menu), `crates/ui-egui/src/uiux_sweep_tests.rs`, `docs/testing/m30-report.md` (new report at end of milestone).
  - **Exact changes:**
    1. In `timeline.rs`: compute `hidden_ids` and `locked_ids` for the active slide/layer; when non-empty, render compact quiet buttons `"Show all ({N})"` and `"Unlock all ({N})"` (`>= 24 pt` tall) dispatching single-undo `object.set_editor_hidden` / `object.set_locked` with `"objects": ids`.
    2. In `menu.rs` (`View` menu): add `"Show all hidden objects"` and `"Unlock all objects"` items (with disabled hover tooltips when count == 0).
    3. Write concise `docs/testing/m30-report.md` summarizing all test IDs, bug dispositions (`BUG-01`–`BUG-08`), sweep results, and perf results.
  - **Tests:** `C-07`, `C-08`, `C-09`, `FZ-02`.
  - **Final Release Gate:** `cargo xtask build-player`, `cargo xtask ci`, `cargo xtask perf`.
  - **Done when:** All test IDs (`A-01`–`A-18`, `B-01`–`B-12`, `C-01`–`C-09`, `FZ-01`–`FZ-03`) pass, `uiux_sweep_all_surfaces` has 0 MUST violations, and the full release gate is green.

---

## 9. Release Gate for M30

### 9.1 Automated Release Gate Commands
Run in order before cutting the release:
1. `cargo xtask build-player` (rebuilds embedded WASM web player after `S2`, `S6`, `S8`, `S12`)
2. `cargo xtask ci` (`fmt --check`, `clippy --workspace --all-targets -D warnings`, `test --workspace`, `layers`, `wasm`)
3. `cargo test -p coursecraft-ui-egui -j 4 uiux_sweep` (0 MUST violations across all surfaces including redesigned Slide Size, Preferences, Course Inspector, Course Settings, Status Bar, and Timeline)
4. `cargo test -p coursecraft-render -j 4 uiux_player` (0 player chrome / Start Gate violations across all 6 viewports)
5. `cargo xtask perf` (confirm 300-slide big-course timings stay within M21 baselines)

### 9.2 Owner Manual Verification Checklist (Desktop App + Browser — App Only, No CLI)
A 10-minute by-hand walkthrough before shipping:
1. **Slide size discoverability & redesigned dialog:** Open `examples/rich-states/` (or any course).
   - *Verify entry points:* Open **Slide size** from **File ▸ Slide size…**, **Slide ▸ Slide size…**, **Course** inspector → **Change…**, **Course settings** → **Change…**, and by clicking **`1280 × 720 px`** in the bottom status bar.
   - *Verify controls & live preview:* Pick **`16:9 Full HD (1920 × 1080)`** — ratio shows `16:9`, preview shows clean 150% scale with 0 objects outside. With **Lock aspect ratio** checked, type `1600` in Width → Height updates to `900`. Switch to **`1:1 Square (1080 × 1080)`** and toggle between **Scale to fit** (shows top/bottom margins), **Fill slide** (shows cropped left/right edges), and **Keep as is** (shows off-slide object warning count).
   - *Verify default:* Pick **`16:9 Full HD (1920 × 1080)`**, click **Use as default for new courses** → button confirms `"Saved as default"`; open **File ▸ Preferences…** to confirm it matches; click **File ▸ New** → new course starts at `1920 × 1080 px`.
2. **State & extra scaling + Undo + Check Course:** On `examples/rich-states/`, resize `1280 × 720 → 1920 × 1080` with **Scale to fit** → open the **States** tab and preview `hover` on the character → speech bubble and glow scale cleanly with the character (not floating over the chest). Press `Cmd+Z` / `Ctrl+Z` → returns cleanly to `1280 × 720`. Now resize to `640 × 360` with **Keep as is** → click the warnings chip in the status bar (**Check Course**) → click an off-slide warning → selects the stranded object.
3. **0.4.0 bug fixes in the editor:**
   - Open `examples/rich-states/`, enter **Edit state** on `hover` (which has 2 items), click **Reset overrides** → confirm glow resets while both speech-bubble items **remain intact**! Press `Cmd+Z` to restore.
   - While still in **Edit state**, rename the state in the **States** tab card → confirm you stay in Edit-State Mode without getting kicked out.
   - Duplicate a state with an extra, edit the extra's position in the duplicate, then switch back and forth between the two states' extras → confirm the Format inspector always shows the selected state's own coordinates.
4. **Published Web / SCORM Start Gate & Seekbar:** On a course with a Web object on Slide 1, use **File ▸ Publish…** to publish to **Web** and click **Open in browser**:
   - *See:* The dimmed Play start gate sits cleanly in front with **no** Web `<iframe>` covering the Play button; pressing `C` before clicking Play does nothing; clicking **Play** dismisses the gate and reveals the Web object.
5. **Part C everyday polish:**
   - Open **Course settings** → expand **Resources & glossary** → add 3 terms out of order → click **Sort A–Z** → terms alphabetize in one undo step.
   - Press `Cmd+F` / `Ctrl+F` (Find & Replace), search for a word in a glossary definition, click the result → jumps straight to Course Settings with **Resources & glossary** expanded; run **Replace** and verify it updates cleanly.
   - On a slide with 3+ objects, hide 2 and lock 2 in the **Timeline** → click **Show all (2)** and **Unlock all (2)** in the Timeline header → each restores all objects in one undo step.
