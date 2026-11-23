# M32 UI Quick Wins & Slide-List Speed — Product, Architecture & Test Plan

**Role:** Product Specification, UI Audit & Implementation Plan (planning only — zero source files edited)  
**Target release:** 0.4.3 / M32 (starts after M31 is merged)  
**Scope:**
1. **Sentence-case label pass & multi-name unification:** Inventory every user-facing label across the command registry, menus, toolbar, dialogs, inspector, status bar, app shell, and player chrome; enforce CourseCraft's sentence-case rule (`UX-11`) and give every multi-named action one canonical name.
2. **Shortcut tooltips from one shared source:** Reuse M31's shortcut table so every toolbar button, menu item, context menu item, and inspector formatting toggle with a keyboard shortcut displays its platform-formatted shortcut in its tooltip (while menu items keep their right-aligned hint), with zero drift.
3. **A reason on every disabled control (`UX-08`):** Audit and fix every `add_enabled` / `add_enabled_ui` / `icon_button_enabled` / `quiet_button` / `ui.disable()` site missing an explanatory disabled hover tooltip, plus add a static CI guard.
4. **Preview in a menu:** Put all four Preview actions (**Preview this slide**, **Preview from here**, **Preview selected slides**, **Preview whole course**) in a dedicated top-level **Preview** menu sharing the exact same helper as the toolbar dropdown.
5. **One Import menu:** Consolidate every import entry point (PowerPoint, document to course, outline to slides, questions from spreadsheet, XLIFF translation, media files, font) under **File ▸ Import ▸** with consistent sentence-case names (and keep **File ▸ Export ▸** symmetric).
6. **Slide-list speed (visible-only composition + revision-keyed cache):** Eliminate unconditional off-screen `compose_authoring` calls (currently 300 calls/frame on the 300-slide perf course!), add a bounded revision/fingerprint-keyed `ThumbnailCache`, and lock the win with counter-based guard tests and release perf measurements.

**Builds on M31 (do not redo M31):** M31 already ships Save / Save As / Open / New shortcuts with File menu hints and a `File` group in the Keyboard shortcuts dialog, MCP media fixes, clamp crash fixes, empty-state-image publish fixes, SCORM 1.2 pass downgrade fixes, the last-visited loop fix, the web editor unsaved-changes prompt, and chunked web publish. M32 builds directly on M31's shortcut table and File menu hints without duplicating any M31 work.

**Clean-room commitment:** Capability match only (`docs/contributing.md:21-40`). Visual layout, palette tokens, typography, icons (Phosphor MIT), and terminology belong strictly to CourseCraft (`DESIGN.md` and `UX-01`–`UX-22` in `docs/testing/m28-uiux-plan.md`). Zero competitor coined terms (we use **Course map**, **Slide masters**, and **Custom question**).

> **Implementer Guardrails (`AGENTS.md`, `docs/contributing.md`)**
> 1. **Never crash:** Zero `unwrap`/`expect`/`panic!`/`unreachable!`/`todo!` in non-test code; `unsafe_code = "forbid"` everywhere except `coursecraft-video-native`.
> 2. **Everything is a command:** Never rename a shipped command `id` in `REGISTRY`; only `label` and `menu` display text change. Keep `docs/cli.md` in sync (checked by `apps/coursecraft-cli/src/tests.rs`).
> 3. **One runtime:** Do not add play-time logic outside `coursecraft-logic`. `"Preview from here"` sets `start_slide` on `Preview`'s cloned `Course` without altering `coursecraft-logic` runtime semantics.
> 4. **Layering & WASM:** `cargo xtask ci` (`fmt`, `clippy -D warnings`, `test --workspace`, `layers`, `wasm`) must stay green. Never break `wasm32-unknown-unknown` for L0–L5.
> 5. **Tests updated in the same step:** Whenever a label changes, update every test asserting on that label in the same step so CI is never red between steps.

---

## 1. Goals & Clean-Room / Patent Check

### 1.1 Goals
1. **Consistent, quiet, sentence-case UI copy:** Every menu, submenu, toolbar button, tooltip, dialog title, panel header, status chip, undo/redo name, and shell modal follows one sentence-case rule with one canonical name per action.
2. **Discoverable shortcuts without drift:** Authors see the platform-appropriate shortcut (`⌘` on macOS, `Ctrl+` elsewhere) on hover for every toolbar button and menu item that has one, sourced from a single table shared with M31 and the Keyboard shortcuts dialog.
3. **Zero mystery-disabled controls:** Hovering any disabled button, menu item, checkbox, radio, or icon button tells the author in one short sentence why it is unavailable and how to enable it.
4. **Menus match the toolbar:** Both **Preview** (all four modes, including the new **Preview from here**) and **Import** (every file/content import in one place) are first-class menu paths.
5. **Fast 300-slide navigator:** Composing thumbnails happens only for rows visible in the scroll viewport and only when a slide (or its master / course theme) actually changed — dropping idle `compose_authoring` calls on the 300-slide perf course from **300/frame to 0/frame**.

### 1.2 Clean-Room & Patent Check (`docs/contributing.md:21-40`)
- **SCORM data (KnowBe4 US10949760 / US11599810 / US11847579):** Untouched.
- **Automatic course from documents (Docebo US11720617B2):** Untouched; only menu placement and casing of the existing user-driven `"Course from document…"` dialog change.
- **Screen recording / software simulation (Assima US9570050B2 / US12360646B2):** Untouched.
- **Learner classification / adaptive content (Adobe US10911553B2 / US10943497B2):** Untouched.
- **Clean-room UI wording:** Sentence case everywhere; zero competitor terms or assets.

---

## 2. Baseline Measurements (Measured Before Planning)

### 2.1 Release Frame-Time Baseline (`perf_tests`)
Command run on Apple Silicon (macOS), release build:

```sh
cargo test --release -p coursecraft-ui-egui perf_tests -- --ignored --nocapture --test-threads=1
```

Measured baseline on the 300-slide perf course (`SLIDES = 300`, 10 scenes, 6 objects/slide, 1280×900 headless frames):

| Stage | Budget | Measured Baseline (median / p95) | Target After M32 (median / p95) |
|---|---|---|---|
| `idle` | 16 ms | **1.37 ms / 1.43 ms** | **≤ 0.65 ms / ≤ 0.80 ms** |
| `select slide 150` | 33 ms | **1.59 ms / 1.70 ms** | **≤ 0.85 ms / ≤ 1.00 ms** |
| `scroll slide list` | 33 ms | **1.32 ms / 1.37 ms** | **≤ 0.75 ms / ≤ 0.90 ms** |
| `typing in a text field` | 33 ms | **1.51 ms / 1.53 ms** | **≤ 0.85 ms / ≤ 1.00 ms** |
| `visit every slide` | 33 ms | **1.41 ms / 1.69 ms** | **≤ 0.85 ms / ≤ 1.10 ms** |
| `visit every slide, real pictures` | 33 ms | **4.27 ms / 7.07 ms** (81 decoded pictures) | **≤ 4.0 ms / ≤ 6.5 ms** (≤ 85 decoded) |
| `undo / redo move` (30 cycles) | 33 ms | **9.52 ms / 9.85 ms** | Improved further in M33 |
| `first frame` | 500 ms | **19.65 ms** | **≤ 15 ms** |

### 2.2 Structural Root Cause & Compose Calls Per Frame (Measured by Code Audit)
Audited `crates/ui-egui/src/slides.rs` and `crates/ui-egui/src/course_map.rs`:
1. **`slides.rs:1359-1399` (`show_slides_content`, main slide loop):** Calls `compose_authoring(&editor.session.course, slide_ref, None)` at line **1366** **before** checking `if thumb_rect.intersects(ui.clip_rect())` at line **1380**. In a 1280×900 window (`row_height = 135 pt`, panel height `~760 pt`), only **6–7 rows** are visible, yet every single frame runs:
   - **300** `compose_authoring` calls (1,800 objects composed and allocated into fresh `Vec<DrawItem>`s),
   - **300** gutter text paints, `rect_filled` backgrounds, `rect_stroke` borders, `ui.interact` allocations, and `fonts.layout_job` title layouts,
   - **300** `SlideSummary` `id`/`title` `String` clones (`slides.rs:1206-1224`).
2. **`slides.rs:1773-1801` (expanded question bank slides):** Same bug — `compose_authoring` at line **1773** runs unconditionally before `if thumb_rect.intersects(ui.clip_rect())` at line **1785**.
3. **`slides.rs:309-334` (`show_masters_panel`):** Calls `compose_authoring` at line **309** with **no viewport clip check at all** and no caching.
4. **`course_map.rs:337` (`show_map_slide_card`):** Calls `compose_authoring` unconditionally every frame with no viewport clip check and no caching (despite its comment `"The existing thumbnail rendering and cache, as in the navigator"`).
5. **Why existing M21 `guard_tests.rs` missed it:** `guard_counters::note_thumbnail_paint` (`slides.rs:1385`) sits *inside* `if thumb_rect.intersects(ui.clip_rect())` and suppresses counts for unchanged slides, so `thumbnail_redraws()` reported `0` even while `compose_authoring` ran **300 times per frame (6,000 calls over 20 idle frames)**!

| Metric (300-slide perf course, 1280×900) | Before M32 | Target After M32 |
|---|---:|---:|
| `compose_authoring` calls per idle frame | **300** | **0** |
| `compose_authoring` calls over 20 idle frames | **6,000** | **0** |
| `compose_authoring` calls on 1 object nudge (visible slide) | **300** | **1** |
| `compose_authoring` calls on 1 scroll wheel step | **300** | **≤ 8** (newly entering visible rows only) |
| `compose_authoring` calls on slide selection change (no scroll) | **300** | **0** |

---

## 3. Detailed Design & Complete Inventories

### 3.1 Label Pass — Sentence-Case Rule & Complete Inventory

#### Casing Rule (`UX-11`, `docs/testing/m28-uiux-plan.md`)
- **Sentence case** on every menu item, submenu, toolbar label, tooltip, dialog title, section heading, form label, button, status chip, undo/redo command label, and shell modal button.
- **Capitalize only:**
  1. The first word of a label or sentence.
  2. Proper nouns, product/brand names, file formats, and acronyms: `CourseCraft`, `PowerPoint`, `PPTX`, `Word`, `PDF`, `MP4`, `CSV`, `XLIFF`, `SCORM`, `LTI`, `xAPI`, `cmi5`, `HTML5`, `AI`, `Lottie`, `GitHub`, `Inter`, `OK`, `ID`, `A–Z`, `Web`, `git` (lowercase as in `"(for git)"`).
  3. Platform-standard confirmation button `"Don't Save"` (explicitly preserved by M28 Reviewer note #4).
  4. Keyboard key names (`Esc`, `Enter`, `Delete`, `Ctrl`, `Cmd`, `Shift`, `Alt`, `F2`, `F7`, `F11`, `F12`).

#### Multi-Name Action Unification Table
Every action that currently has two or more different names across the UI gets **one canonical name**:

| Action | Current Names (Where Found) | Canonical Name |
|---|---|---|
| Insert a new slide | `"Add slide"` (toolbar button), `"Add slide to course"` (toolbar tooltip), `"New Slide"` (Insert menu & registry), `"New slide"` (slides footer, scene header menu, empty navigator menu) | **`"New slide"`** *(toolbar label `"New slide"`, tooltip `"New slide (⌘M)"`, menu `"New slide"`)* |
| Open course settings | `"Course Properties"` (`File/Course Properties` in registry), `"Course settings"` (`Project` menu & dialog title) | **`"Course settings…"`** *(remove duplicate from `File` or unify both to `"Course settings…"` — keep one canonical `"Course settings…"` under `Project` and update `course.update` menu to `""` since `Project` already has `"Course settings…"` — wait: check `menu_routes_cover_every_menu_command` and `menu_routes_open_the_variables_tab_and_course_settings`, keeping `course.update` at `"Project/Course settings…"` eliminates the duplicate!)* |
| Toggle slide masters mode | `"Slide Masters"` (`View` menu checkbox), `"Master View"` (`slides.rs:239` `"Close Master View"`, disabled tooltips `"Not available in Master View"` in `menu.rs`, `toolbar.rs`, `media_library.rs`) | **`"Slide masters"`** *(toggle `"Slide masters"`, close button `"Close slide masters"`, disabled text `"Not available in slide masters"`)* |
| Preview currently selected slide only | `"Preview current slide"` (toolbar tooltip), `"Preview this slide"` (toolbar dropdown & shortcuts dialog) | **`"Preview this slide"`** |
| Preview entire course from start | `"Preview from the start"` (toolbar dropdown & shortcuts dialog) | **`"Preview whole course"`** *(contrasts clearly with `"Preview this slide"`, `"Preview from here"`, and `"Preview selected slides"`)* |
| Select all objects | `"Select All"` (`Edit` menu), `"Select all"` (context menus & shortcuts dialog) | **`"Select all"`** |
| Effect log in Preview | `"Effect Log"` (`preview.rs:1198, 1220` panel header), `"Effect log (F2)"` (`preview.rs:997` toggle tooltip) | **`"Effect log"`** |
| Format painter | `"Format Painter"` (`toolbar.rs:543` tooltip), `"Paint"` (`toolbar.rs:548` compact toolbar label) | **`"Format painter"`** *(compact toolbar label stays `"Paint"` like `"Map"` for `"Course map"`, tooltip becomes `"Format painter (⌥⌘C)"`)* |
| Delete template | `"Remove"` (`slides.rs:766` course templates) vs `"Delete"` (`slides.rs:837` library templates) | **`"Delete"`** *(consistent across both template lists)* |

#### Complete Label Change Inventory

1. **Command registry (`crates/engine/src/commands.rs` — 186 commands):**
   - Convert every multi-word `label` in `REGISTRY` from Title Case to sentence case (about 175 commands; e.g. `"Course Info"` → `"Course info"`, `"Check Course"` → `"Check course"`, `"Course Properties"` → `"Course settings"`, `"Slide Size"` → `"Slide size"`, `"Course Title"` → `"Course title"`, `"Player Settings"` → `"Player settings"`, `"Quiz Settings"` → `"Quiz settings"`, `"Publish Settings"` → `"Publish settings"`, `"Set Background Audio"` → `"Set background audio"`, `"New Scene"` → `"New scene"`, `"New Slide"` → `"New slide"`, `"Delete Slide"` → `"Delete slide"`, `"Move/Resize Object"` → `"Move/resize object"`, `"Edit Path Points"` → `"Edit path points"`, `"Add State Item"` → `"Add state item"`, `"Find and Replace"` → `"Find and replace"`, `"Import Questions from CSV"` → `"Import questions from CSV"`, `"Set Lottie Playback"` → `"Set Lottie playback"`, etc.).
   - Convert every `menu` path in `REGISTRY` to sentence case:
     - `"File/Check Course"` → `"File/Check course"`
     - `"File/Course Properties"` → `""` (deduplicated; `Project/Course settings…` handles it, or move `"course.update"` to `"Project/Course settings…"` and remove the manual duplicate `"Course settings"` button in `render_extra_category_items`)
     - `"Project/Player Settings"` → `"Project/Player settings…"`
     - `"Project/Quiz Settings"` → `"Project/Quiz settings…"`
     - `"Insert/New Scene"` → `"Insert/New scene"`
     - `"Edit/Delete Scene"` → `"Edit/Delete scene"`
     - `"Insert/New Slide"` → `"Insert/New slide"`
     - `"Edit/Delete Slide"` → `"Edit/Delete slide"`
     - `"Insert/Slides from Outline…"` → `"File/Import/Slides from outline…"` (routed into the unified Import menu in `S4`)
     - `"Insert/New Layer"` → `"Insert/New layer"`
     - `"Edit/Delete Layer"` → `"Edit/Delete layer"`
     - `"Arrange/Bring Forward"` → `"Arrange/Bring forward"`
     - `"Insert/Results Slide"` → `"Insert/Results slide"`
     - `"Insert/Certificate Slide"` → `"Insert/Certificate slide"`
     - `"Edit/Delete Question"` → `"Edit/Delete question"`
     - `"Edit/Find and Replace…"` → `"Edit/Find and replace…"`
   - Also check `Edit` menu dynamic undo/redo formatting (`menu.rs:948, 956`): `"Undo {name}"` / `"Redo {name}"`. With sentence-case command labels (`"New slide"` → `"Undo new slide"`, `"Edit state"` → `"Undo edit state"`), lowercase the first ASCII character of `name` when it does not start with an acronym (`AI`, `CSV`, `XLIFF`, `Lottie`, `PowerPoint`, `PDF`, `MP4`, `Word`, `SCORM`, `LTI`) so the Edit menu reads `"Undo new slide"` and `"Undo import questions from CSV"`, while `History` rows keep capitalized first letter `"New slide"`.

2. **Menu bar & context menus (`crates/ui-egui/src/menu.rs`):**
   - `File`: `"Open Folder…"` → `"Open folder…"`, `"Open Recent"` → `"Open recent"`, `"Save As…"` → `"Save as…"`, `"Save As Project Folder (for git)…"` → `"Save as project folder (for git)…"`, `"Version History…"` → `"Version history…"`, `"Export Translation…"` → moved to `Export ▸ Translation (XLIFF)…`, `"Import PowerPoint…"` / `"Import Translation…"` / `"Add font…"` → moved to `Import ▸` (`S4`).
   - `Edit`: `"Select All"` → `"Select all"`, `"Check Spelling…"` → `"Check spelling…"`.
   - `View`: `"Slide Masters"` → `"Slide masters"`, `"Snap to Guides"` → `"Snap to guides"`, `"Snap to Grid"` → `"Snap to grid"`.
   - `Arrange` & object context menu: `"Align Left"` → `"Align left"`, `"Align Center"` → `"Align center"`, `"Align Right"` → `"Align right"`, `"Align Top"` → `"Align top"`, `"Align Middle"` → `"Align middle"`, `"Align Bottom"` → `"Align bottom"`, `"Distribute Horizontally"` → `"Distribute horizontally"`, `"Distribute Vertically"` → `"Distribute vertically"`, `"Match Width"` → `"Match width"`, `"Match Height"` → `"Match height"`, `"Align to Slide"` → `"Align to slide"`.
   - `Project`: `"AI Settings…"` → `"AI settings…"`, `"Suggest Alt Text…"` → `"Suggest alt text…"`, `"Save as Template"` → `"Save as template"`, `"Course settings"` → `"Course settings…"` (uses ellipsis because it opens a dialog).
   - `Insert`: `"Text Box"` → `"Text box"`, `"Record Narration…"` → `"Record narration…"`, `"Narration from Text…"` → `"Narration from text…"`, `"Course from Document…"` → moved to `File ▸ Import ▸ Course from document…` (`S4`).
   - `Help`: `"Report a Problem…"` → `"Report a problem…"`, `"Suggest a Feature…"` → `"Suggest a feature…"`, `"Check for Updates"` → `"Check for updates"`.
   - Disabled messages: `"Not available in Master View"` → `"Not available in slide masters"` (all 8 occurrences in `menu.rs`).

3. **Toolbar (`crates/ui-egui/src/toolbar.rs`):**
   - `"Add slide"` / `"Add slide to course"` → `"New slide"` (tooltip `"New slide ({sc})"`).
   - `"Preview current slide (F12)"` → `"Preview this slide ({sc})"`.
   - `"Preview from the start"` → `"Preview whole course"`.
   - `"Format Painter"` → `"Format painter"` (tooltip `"Format painter ({sc})"`).
   - `"Course Inspector"` / `"Slide Inspector"` / `"Animate Inspector"` / `"Format Inspector"` → `"Course inspector"`, `"Slide inspector"`, `"Animate inspector"`, `"Format inspector"`.
   - `"Insert Text Box"` → `"Insert text box"`, `"Insert Shape"` → `"Insert shape"`, `"Insert Image"` → `"Insert image"`, `"Insert Media"` → `"Insert media"`, `"Insert Button"` → `"Insert button"`, `"Insert Question"` → `"Insert question"`, `"Insert Trigger"` → `"Insert trigger"`.
   - `"Record Narration…"` → `"Record narration…"`.
   - `"Not available in Master View"` → `"Not available in slide masters"`.

4. **Dialogs, panels, inspector, status bar, app shell, and player chrome:**
   - `slides.rs`: `"Close Master View"` → `"Close slide masters"`, `"Save as Template"` → `"Save as template"`, `"Remove"` (course templates, `:766`) → `"Delete"`.
   - `media_library.rs`: `"Not available in Master View"` → `"Not available in slide masters"`.
   - `ai_settings.rs:430`: `"AI Settings"` → `"AI settings"`.
   - `alt_text.rs:369`: `"Suggest Alt Text"` → `"Suggest alt text"`.
   - `audio_ai.rs:409, 648`: `"Generate Captions"` → `"Generate captions"`, `"Narration from Text"` → `"Narration from text"`.
   - `document.rs:295`: `"Course from Document"` → `"Course from document"`.
   - `outline.rs:128`: `"Slides from Outline"` → `"Slides from outline"`.
   - `record_narration.rs:150`: `"Record Narration"` → `"Record narration"`.
   - `report.rs:388, 443`: `"Report a Problem"` → `"Report a problem"`, `"Copy to Clipboard"` → `"Copy to clipboard"`.
   - `spell_check.rs:248`: `"Add to Dictionary"` → `"Add to dictionary"`.
   - `status.rs:402, 537, 556`: `"Check Course"` → `"Check course"`, `"Error History"` → `"Error history"`, `"PowerPoint Import Report"` → `"PowerPoint import report"`.
   - `translation.rs:21, 94`: `"Export Translation"` → `"Export translation"`, `"Translation Import Report"` → `"Translation import report"`.
   - `preview.rs:1198, 1220, 1288, 1291`: `"Effect Log"` → `"Effect log"`, `"Replay Slide(s)"` → `"Replay slide(s)"`, `"Close Preview"` → `"Close preview"`.
   - `properties.rs:4039, 4052, 4914`: `"Entrance Animation"` → `"Entrance animation"`, `"Exit Animation"` → `"Exit animation"`, `"Text Content"` → `"Text content"`.
   - `question.rs:16-18`: `"Multiple Choice"` → `"Multiple choice"`, `"Multiple Response"` → `"Multiple response"`, `"True / False"` → `"True / false"`.
   - `states.rs:366, 437, 638`: `"Set Initial"` → `"Set initial"`, `"Same as Normal"` → `"Same as normal"`.
   - `triggers.rs`: `"New Trigger"` → `"New trigger"`, `"Edit Trigger"` → `"Edit trigger"`, `"Paste Triggers"` → `"Paste triggers"`, `"Slide Base Timeline"` → `"Slide base timeline"`.
   - `variables.rs:112`: `"Initial: True/False"` → `"Initial: true/false"`.
   - `shortcuts.rs:343`: `"View & Canvas"` → `"View & canvas"`.
   - `apps/coursecraft/src/main.rs:2974, 3052`: `"Save a Copy"` → `"Save a copy"`, `"Open Latest Version"` → `"Open latest version"` (keep `"Don't Save"` at `:2925` per M28 Reviewer note #4).
   - `crates/ui-egui/src/snapshot.rs:159-227`: update `WINDOWS` `shows` needles to match the sentence-case titles.
   - **Player chrome audit (`crates/course/src/model.rs:15-91`, `crates/render/src/player_frame.rs`, `apps/coursecraft-player/src/app.rs`):** Verified all 76 `CHROME_STRINGS` defaults and player shell strings (`"Unable to load course"`, etc.) — **100% are already sentence case.** No `CHROME_STRINGS` edits required (preserves XLIFF `chrome/*` stability).

---

### 3.2 Shortcut Tooltips from One Shared Source

Build directly on M31's shortcut table (do not create a second table):

1. **Single source of truth (`crates/ui-egui/src/shortcuts.rs`):**
   - Extend M31's shortcut lookup table so every editor action with a shortcut has a stable `ShortcutAction` enum (or `&'static str` action ID) mapping to its canonical raw shortcut string (e.g. `"Ctrl+M"`, `"Ctrl+Shift+P"`, `"F12"`, `"Alt+F12"`, `"Shift+F12"`, `"Ctrl+Shift+M"`, `"Ctrl+Alt+C"`, `"Ctrl+,"`, `"F7"`, `"Ctrl+B"`, `"Ctrl+I"`, `"Ctrl+U"`, `"?"`, plus M31's File shortcuts `"Ctrl+N"`, `"Ctrl+O"`, `"Ctrl+S"`, `"Ctrl+Shift+S"`).
   - For registry commands (`edit.undo`, `edit.redo`, `slide.add`, `object.duplicate`, `object.delete`, `object.group`, `object.ungroup`, `course.find`), look up `cmd.shortcut` directly from `coursecraft_engine::registry()` via `shortcut_for_command(id: &str) -> Option<&'static str>`.
   - `build_shortcut_groups()` in `shortcuts.rs` reads from this exact table + `registry()`, so the Keyboard shortcuts dialog, menus, and tooltips can never diverge.

2. **Two shared UI helpers (`crates/ui-egui/src/menu.rs` or `widgets.rs`):**
   ```rust
   /// Format `"Label (⌘M)"` (or plain `"Label"` when `shortcut` is `None`).
   pub fn tooltip_with_shortcut(ctx: &egui::Context, base: &str, shortcut: Option<&str>) -> String;

   /// Render a menu button that shows a right-aligned shortcut hint AND a hover
   /// tooltip containing the same formatted shortcut, plus a disabled reason when off.
   pub fn menu_button(
       ui: &mut Ui,
       label: &str,
       shortcut: Option<&str>,
       enabled: bool,
       disabled_reason: Option<&str>,
   ) -> egui::Response;
   ```
   - When `enabled == true` and `shortcut.is_some()`: sets `.shortcut_text(formatted)` AND `.on_hover_text(tooltip_with_shortcut(ui.ctx(), label, shortcut))`.
   - When `enabled == false`: sets `.shortcut_text(formatted)` (if any) AND `.on_disabled_hover_text(reason)`.

3. **Every control wired to the shared source:**
   - **Toolbar buttons (`toolbar.rs`):**
     - `New slide` → `shortcut_for_command("slide.add")` (`Ctrl+M` → `"New slide (⌘M)"` / `"New slide (Ctrl+M)"`)
     - `Preview` → `ShortcutAction::PreviewThisSlide` (`F12` → `"Preview this slide (F12)"`)
     - `Publish` → `ShortcutAction::Publish` (`Ctrl+Shift+P` → `"Publish course (⇧⌘P)"` / `"Publish course (Ctrl+Shift+P)"`)
     - `Map` → `ShortcutAction::CourseMap` (`Ctrl+Shift+M` → `"Course map (⇧⌘M)"`)
     - `Paint` → `ShortcutAction::FormatPainter` (`Ctrl+Alt+C` → `"Format painter (⌥⌘C)"`)
     - `Undo` → `shortcut_for_command("edit.undo")` (`Ctrl+Z` → `"Undo (⌘Z)"` when enabled; `"Nothing to undo"` when disabled)
     - `Redo` → `shortcut_for_command("edit.redo")` (`Ctrl+Y` → `"Redo (⌘Y)"` when enabled; `"Nothing to redo"` when disabled)
   - **Menu items (`menu.rs`, `slides.rs`, `toolbar.rs` popups):**
     - All registry menu items (`render_registry_menu_item`, `render_open_window_menu_item`, `render_delete_slide_menu_item`) go through `menu_button(...)`.
     - All non-registry menu items with shortcuts (`File`: M31's New/Open/Save/Save as, Preferences `Ctrl+,`, Publish `Ctrl+Shift+P`; `Edit`: Cut/Copy/Paste/Select all/Check spelling `F7`; `View`: Course map `Ctrl+Shift+M`, Zoom to fit `Ctrl+0`, Zoom in `Ctrl+=`, Zoom out `Ctrl+-`; `Preview`: This slide `F12`, From here `Alt+F12`, Whole course `Shift+F12`; `Help`: Keyboard shortcuts `?`; slide & canvas context menus) go through `menu_button(...)`.
   - **Inspector formatting toggles (`properties.rs:3445, 3473, 3502, 4582, 4605, 4629, 6921, 6932, 6943`):**
     - Replace hardcoded `"Bold (Ctrl+B)"`, `"Italic (Ctrl+I)"`, `"Underline (Ctrl+U)"` with `tooltip_with_shortcut(ui.ctx(), "Bold", Some(shortcut(ShortcutAction::Bold)))` (shows `Bold (⌘B)` on macOS!).

---

### 3.3 Disabled Controls Audit — Every Site Missing a Reason

Complete audit of `crates/ui-egui/src` and `apps/coursecraft/src` (non-test code):

| # | File & Line(s) | Control | Current Behaviour | M32 Fix |
|---|---|---|---|---|
| 1 | `menu.rs:603` | `"(No recent files)"` in Open recent | Disabled button, no tooltip | Replace with muted non-interactive `ui.label(small("No recent files").color(p.text_disabled))` (not an actionable button). |
| 2 | `toolbar.rs:417` | `"Not available in Master View"` in Add slide caret popup | Disabled button, no tooltip | Replace with muted `ui.label(small("Not available in slide masters").color(p.text_disabled))`. |
| 3 | `toolbar.rs:428` | `"New layer"` in Add slide caret popup | Disabled when no slide selected, no tooltip | Add `.on_disabled_hover_text("Select a slide first")`. |
| 4 | `toolbar.rs:456` | `"Not available in Master View"` in Preview caret popup | Disabled button, no tooltip | Handled by shared `show_preview_menu_items` (disables each item with `"Not available in slide masters"`). |
| 5 | `toolbar.rs:473` | `"Preview selected slides"` in Preview caret popup | Disabled when `< 2` slides selected, no tooltip | Add `.on_disabled_hover_text("Select two or more slides to preview them")`. |
| 6 | `toolbar.rs:746` | Disabled `media_tooltip` button in Media popup | Disabled button, no tooltip | Replace with muted `ui.label(small(media_tooltip).color(p.text_disabled))`. |
| 7 | `toolbar.rs:642, 653` | Toolbar `Undo` / `Redo` icon buttons | Shows `"Undo (⌘Z)"` when disabled instead of why | Add `widgets::icon_button_enabled_reason(ui, icon, enabled, hover_tip, disabled_reason)` and pass `"Nothing to undo"` / `"Nothing to redo"`. |
| 8 | `slides.rs:929, 944` | `"Move up"` / `"Move down"` in slide context menu | Disabled at top/bottom, no tooltip | Add `.on_disabled_hover_text("Slide is already at the top of this scene")` / `"Slide is already at the bottom of this scene"`. |
| 9 | `slides.rs:980` | `"Preview selected slides"` in slide context menu | Disabled when `< 2` slides selected, no tooltip | Add `.on_disabled_hover_text("Select two or more slides to preview them")`. |
| 10 | `question.rs:1876-1879` | Required drag-and-drop drop-target checkbox | **Bug:** calls `.on_hover_text(...)` on a disabled checkbox (egui never renders `.on_hover_text` when disabled!) | Change to `.on_disabled_hover_text("Required because a drag item targets this object")`. |
| 11 | `question.rs:904, 907, 993, 1082, 1085, 1088, 1140, 1143, 1146, 1242, 1245, 1248, 1607, 1680, 1845` | `icon_button_enabled` / `icon_button_danger` for Move up/down and Remove answer/choice/pair/region/item | Shows action name (`"Move up"`, `"Remove choice"`) when disabled | Switch to `icon_button_enabled_reason` / `icon_button_danger_reason` with exact reasons (`"Already first"`, `"Already last"`, `"At least one answer is required"`, `"At least two choices are required"`, `"At least two pairs are required"`, `"At least one region is required"`, `"At least one drag item is required"`). |
| 12 | `triggers.rs:1959` | Delete trigger icon button (`!form_open`) | Shows `"Delete trigger"` when disabled | Switch to `icon_button_danger_reason(..., "Delete trigger", "Close the trigger editor first")`. |
| 13 | `publish.rs:428, 446` | Standard radio buttons & SCORM 1.2 status combo while `busy` | Disabled during publish, no tooltip | Add `.on_disabled_hover_text("Wait for publishing to finish")`. |
| 14 | `ai_settings.rs:906, 913, 923, 930` | Agent row `quiet_button` (`Connect` / `Disconnect` / `Test` / `Try again`) when `!can_act` | Disabled while checking, no tooltip | Add `.on_disabled_hover_text("Wait for the current agent check to finish")` (ensure `quiet_button` wraps disabled state with `add_enabled_ui(enabled, ...)` so `.on_disabled_hover_text` fires reliably in egui). |
| 15 | `properties.rs:4030` | `show_animate_tab` `ui.disable()` in Edit-State Mode | Banner shown above, individual controls have no hover tooltip | Wrap body in `ui.add_enabled_ui(!state_edit_locked, ...)` with `.response.on_disabled_hover_text(crate::STATE_LOCK_TOOLTIP)` (matching Format tab pattern at `properties.rs:7522`). |

**Static guard test (`every_add_enabled_has_disabled_reason`):** Scans non-test production source files in `crates/ui-egui/src` and `apps/coursecraft/src` (before `#[cfg(test)]`) and fails CI if any `add_enabled(` or `add_enabled_ui(` call lacks `.on_disabled_hover_text(` on its widget/block (excluding `add_enabled(true, ...)` which is never disabled — also clean up `slides.rs:2198` `add_enabled(true, Button::new("Delete scene"))` to plain `ui.button("Delete scene")`).

---

### 3.4 Preview Menu & `"Preview from here"`

1. **Menu placement:** Add a top-level **`Preview`** menu in `menu.rs` right before **`Help`** (9 top-level menus total: `File`, `Edit`, `Insert`, `Arrange`, `Project`, `Slide`, `View`, `Preview`, `Help`; total width `~420 pt`, fits comfortably inside `1024×600` even at scale `2.0`).
2. **One shared menu builder (`show_preview_menu_items(editor, ui, requests)`):** Used by **both** the top-level **Preview** menu AND the toolbar's Preview caret dropdown so labels, shortcuts, enabling rules, and disabled reasons are guaranteed identical:
   - **Preview this slide** (`F12`): previews only the currently selected slide (`open_preview_at(Some(&slide_id))` → 1-slide partial preview). Disabled in slide masters (`"Not available in slide masters"`) or on a bank slide (`"Question bank slides can't be previewed directly"`).
   - **Preview from here** (`Alt+F12`): previews the **whole course starting at the currently selected slide** (`open_preview_from(&slide_id)` — no partial-preview popup, Next continues through subsequent slides). Disabled in slide masters or on a bank slide with the same reasons.
   - **Preview selected slides**: previews the selected subset (`open_preview_selected()`). Disabled in slide masters (`"Not available in slide masters"`) or when `< 2` non-bank slides are selected (`"Select two or more slides to preview them"`).
   - **Preview whole course** (`Shift+F12`): previews the entire course from its start slide (`open_preview_at(None)`). Disabled in slide masters (`"Not available in slide masters"`).
3. **`Preview::new_from_slide(course, slide_id)` (`crates/ui-egui/src/preview.rs` & `lib.rs`):**
   - Clones `course`, sets `course_clone.start_slide = Some(slide_id.to_owned())`, stores `course_start = Some(slide_id.to_owned())` (so clicking **Restart** inside the preview restarts from that slide rather than jumping back to slide 1!), leaves `preview_subset = None`, and constructs `Player::with_draw_seed(&course_clone, clock_seed())`.
   - Zero changes to `coursecraft-logic` (`Player` already honours `course.start_slide`!).
   - Wire `Alt+F12` in `crates/ui-egui/src/lib.rs:4555` right alongside `Shift+F12` and `F12` (consuming `Modifiers::ALT, Key::F12` before plain `Key::F12`).

---

### 3.5 Unified Import Menu (`File ▸ Import ▸`) & Symmetric Export Menu

Gather every import scattered across `File`, `Insert`, and `Media library` into **one** `File ▸ Import ▸` submenu (`crates/ui-egui/src/menu.rs`):

| Order | Item in `File ▸ Import ▸` | Action / Request | Availability |
|---:|---|---|---|
| 1 | `PowerPoint (.pptx)…` | `EditorRequest::PickPptx` | Always |
| 2 | `Course from document…` | `editor.open_document_dialog()` | Always |
| 3 | `Slides from outline…` | `course.import_outline` → `editor.open_outline_dialog()` | Always |
| 4 | `Questions from spreadsheet (CSV)…` | `EditorRequest::ImportQuestionsCsv` | Always |
| 5 | `Translation (XLIFF)…` | `EditorRequest::ImportTranslation` | `desktop_file_ops` (else `FILE_OPS_DISABLED_MSG`) |
| — | *separator* | | |
| 6 | `Media files…` | `EditorRequest::PickMediaFiles` | `desktop_file_ops` (else `FILE_OPS_DISABLED_MSG`) |
| 7 | `Font…` | `EditorRequest::PickFonts` | Always |

- **Remove old scattered duplicates:**
  - Remove top-level `File ▸ Import PowerPoint…`, `File ▸ Import Translation…`, and `File ▸ Add font…` (keep `File ▸ Manage fonts…`).
  - Remove `Insert ▸ Slides from Outline…` (change `course.import_outline` `menu` in `REGISTRY` to `"File/Import/Slides from outline…"`) and `Insert ▸ Course from Document…`.
- **Keep `File ▸ Export ▸` symmetric:** Move `File ▸ Export Translation…` into `File ▸ Export ▸ Translation (XLIFF)…` (gated on `desktop_file_ops`) alongside `Video (MP4)…`, `PDF for review…`, and `Word for review…`.
- **Keep Welcome screen & Media Library buttons working:** Welcome screen's `"Import PowerPoint…"` tile and Media Library's `"Import…"` button remain as quick shortcuts to the exact same `EditorRequest`s.

---

### 3.6 Slide-List Speed: Visible-Only Composition & Revision-Keyed Cache

#### Architecture (`crates/ui-egui/src/slides.rs`, `lib.rs`, `course_map.rs`)

1. **Row-level viewport culling (before any compose, paint, text layout, or `ui.interact`):**
   - In `show_slides_content` (`slides.rs:1320`), compute `thumb_rect` and `row_rect` (gutter + thumbnail + title) up front.
   - Always update `selected_rect` (so `scroll_to_rect` works when jumping to an off-screen slide!) and record lightweight drag-target geometry when `editor.ui.slide_drag.is_some()` (don't even allocate `slide_rows` when not dragging!).
   - Check `let visible = row_rect.intersects(ui.clip_rect());` (expanded vertically by `0.0` or `4.0 pt` for selection ring stroke).
   - If `!visible` AND not currently renaming this slide: **`current_y += row_height; continue;`**!
     - Skips: `compose_authoring`, `parse_color`, `rect_filled`, `paint`, gutter number text, border strokes, `⋯` button, title `LayoutJob` (`fonts.layout_job`), `ui.interact`, `context_menu`, and `mark_content_region`.
   - Apply the exact same viewport culling to **scene headers** (except when renaming), **question bank slide rows** (`slides.rs:1760`), **master rows** (`slides.rs:305`), and **Course map cards** (`course_map.rs:320`).
   - Eliminate per-frame `Vec<SceneSummary>` 300-slide string cloning (`slides.rs:1206-1224`): cache a lightweight `NavigatorIndex` keyed on `SyncRevision` (rebuilt only when `editor.session.revision` changes — 0 allocations on idle and scroll frames!).

2. **Per-slide revision-keyed `ThumbnailCache` (`crates/ui-egui/src/slides.rs`, stored on `UiState`):**
   ```rust
   const MAX_CACHED_THUMBNAILS: usize = 128;

   pub struct ThumbnailCache {
       /// Course visual fingerprint (slide_size, theme, text_styles) + revision it was checked at.
       visual_revision: u64,
       course_visual_fp: u64,
       /// Monotonic touch clock for O(n <= 128) LRU eviction.
       tick: u64,
       entries: HashMap<String, CachedThumbnail>,
   }

   struct CachedThumbnail {
       /// Last `session.revision` at which this entry was verified fresh (O(1) fast path).
       verified_revision: u64,
       /// Deterministic 64-bit content fingerprint of `(slide, master, course_visual_fp)`.
       fingerprint: u64,
       /// Background fill color parsed once.
       bg: Color32,
       /// Composed authoring items, shared by Arc so painting never clones `Vec<DrawItem>`.
       items: Arc<[DrawItem]>,
       last_used: u64,
   }
   ```
   - **Lookup algorithm (`ThumbnailCache::get_or_compose(&mut self, revision, course, slide) -> (Color32, Arc<[DrawItem]>)`):**
     1. **Fast path (idle frames, hover, selection change, scroll revisit at same revision):** If `entries.get(&slide.id)` has `verified_revision == revision`, bump `last_used` and return `(entry.bg, Arc::clone(&entry.items))` in **O(1)** — **0 hashing, 0 `compose_authoring` calls!**
     2. **Revision-changed path (only called for the ~6 visible rows!):**
        - Compute `course_visual_fp` once per `revision` (hash `slide_size`, `theme`, `text_styles`).
        - Compute `fp = thumbnail_fingerprint(course, slide, course_visual_fp)`: deterministic FNV-1a 64-bit hash over `slide` + its referenced `master` (if any) + `course_visual_fp` (normalizing `f32` via `if x == 0.0 { 0u32 } else { x.to_bits() }` — zero allocation, ~2–5 µs per visible slide).
        - If `entries.get_mut(&slide.id)` has `entry.fingerprint == fp`: update `entry.verified_revision = revision` and `entry.last_used = tick`, and return `(entry.bg, Arc::clone(&entry.items))` — **still 0 `compose_authoring` calls for untouched visible slides!**
        - Otherwise (cache miss or this slide/master/theme actually changed): call `compose_authoring(course, slide, None)` (incrementing `guard_counters::note_thumbnail_compose()` in debug/test builds!), parse `bg`, store in `entries`, and if `entries.len() > MAX_CACHED_THUMBNAILS`, evict the oldest `last_used` entry.
     3. **Clear on course replacement:** Clear `self.ui.thumbnail_cache.clear()` inside `Editor::set_course` (and whenever loading a new/opened/example course).
   - **Why both `verified_revision` AND `fingerprint`?**
     - `verified_revision` guarantees **O(1) zero-work** on every idle, hover, and scroll-cache-hit frame.
     - `fingerprint` guarantees that editing slide 0 recomposes **only slide 0** (1 compose call, NOT 6!), AND editing a master or course theme automatically invalidates every visible slide that depends on it!

3. **Counter-based guard instrumentation (`crates/ui-egui/src/guard_counters.rs` & `guard_tests.rs`):**
   - Add `THUMBNAIL_COMPOSES: Cell<u64>` (`note_thumbnail_compose()`, `thumbnail_composes()`), incremented **every single time** `compose_authoring` is called in `slides.rs` (normal slides, bank slides, and masters).
   - Unlike `THUMBNAIL_REDRAWS`, `THUMBNAIL_COMPOSES` has zero suppression logic — it counts literal `compose_authoring` invocations so any future regression that bypasses the cache or viewport check immediately fails CI!

---

## 4. Normative Rules (`M32-01`–`M32-12`)

| Rule ID | Category | Normative Requirement |
|---|---|---|
| **M32-01** | Sentence Case | Every menu item, toolbar label/tooltip, dialog title, panel header, status chip, undo/redo command label, and shell button uses sentence case (preserving only proper nouns, acronyms, key names, and `"Don't Save"`). |
| **M32-02** | Single Canonical Name | Each action in the multi-name table (§3.1) uses its one canonical name everywhere (`"New slide"`, `"Course settings…"`, `"Slide masters"`, `"Preview this slide"`, `"Preview whole course"`, `"Select all"`, `"Effect log"`, `"Format painter"`). |
| **M32-03** | Command ID Stability | Zero shipped command IDs in `REGISTRY` are renamed; `docs/cli.md` stays in sync. |
| **M32-04** | Shared Shortcut Source | Toolbar tooltips, menu `.shortcut_text` hints, menu hover tooltips, inspector Bold/Italic/Underline tooltips, and the Keyboard shortcuts dialog all format shortcuts from one shared table (extending M31's table) via `shortcut_label`. |
| **M32-05** | Disabled Reasons (`UX-08`) | Every disabled button, menu item, checkbox, radio, and icon button explains why it is disabled on hover; zero `add_enabled` / `add_enabled_ui` controls omit `.on_disabled_hover_text`. |
| **M32-06** | Preview Menu & Parity | Top-level **Preview** menu and toolbar Preview dropdown share `show_preview_menu_items` with all 4 modes: **Preview this slide** (`F12`), **Preview from here** (`Alt+F12`), **Preview selected slides**, and **Preview whole course** (`Shift+F12`). |
| **M32-07** | Preview From Here | `"Preview from here"` starts playback at the selected slide without partial-preview restriction (`is_partial_preview() == false`), allows advancing through subsequent slides, and Restart returns to that starting slide. |
| **M32-08** | Unified Import Menu | All 7 import actions live under **File ▸ Import ▸** with parallel sentence-case names; old duplicates in top-level `File` and `Insert` are removed; `Export ▸ Translation (XLIFF)…` joins `File ▸ Export ▸`. |
| **M32-09** | Zero Off-Screen Composes | Off-screen slide, bank, master, and course-map thumbnails never invoke `compose_authoring`; `thumbnail_composes()` is **0** over 20 idle frames on a 300-slide course. |
| **M32-10** | Revision-Keyed Cache Precision | Nudging 1 object on a visible slide of a 300-slide course triggers **exactly 1** `compose_authoring` call; scrolling triggers at most **≤ 10** calls (newly visible rows only); editing a master or theme invalidates dependent visible thumbnails. |
| **M32-11** | Bounded Cache Memory | `ThumbnailCache` holds at most `MAX_CACHED_THUMBNAILS` (128) entries and clears on `Editor::set_course`. |
| **M32-12** | UI/UX Sweep & Perf Gate | `uiux_sweep_all_surfaces`, `uiux_sweep_scale_2_and_fixtures`, `uiux_sweep_modals_and_tooltips` pass with **0 MUST violations**, and release `perf_tests` idle/scroll/select frame times stay well below budgets (and improve measurably vs baseline). |

---

## 5. Watchlist of Risky Spots (`RISK-01`–`RISK-10`)

| ID | File & Lines | Risk & Mitigation |
|---|---|---|
| **RISK-01** | `crates/engine/src/tests.rs:456-496`, `crates/ui-egui/src/{history_tests,m27_undo_clipboard_tests,toolbar_menu_tests,tests,walkthrough_tests,u14_ai_dialogs_tests,u15_status_welcome_tests,u9_navigator_canvas_tests,bughunt_m26_a6_dialogs,bughunt_m26_a10_question_panel,f24_exit_animation_labels_tests}.rs`, `apps/coursecraft/src/m27_files_ui_tests.rs` | Dozens of existing tests click or assert on old Title Case labels (`"Add slide"`, `"New Slide"`, `"Slide Masters"`, `"Multiple Choice"`, `"Check Course"`, `"Open Folder…"`, `"Save As…"`, `"Import PowerPoint…"`, `"Entrance Animation"`, `"Course Title"`). Update affected test assertions in the **exact same step** as each label change; run full `cargo test` for the affected crate after every step. |
| **RISK-02** | `crates/ui-egui/src/menu.rs:279-297` & `tests.rs` (`menu_routes_cover_every_menu_command`, `menu_routes_open_the_variables_tab_and_course_settings`) | Changing `course.import_outline`'s `menu` path to `"File/Import/Slides from outline…"` or changing `course.update`'s `menu` path must keep `menu_route(id)` and the top-level menu category collector (`menu.rs:560-567`) consistent so `"File/Import/..."` does not duplicate a button at the bottom of `File`. Filter `cmd.menu.strip_prefix("File/")` to skip `"Import/"` (rendered inside the Import submenu). |
| **RISK-03** | `crates/ui-egui/src/lib.rs:4555` (`Alt+F12` vs `Shift+F12` vs `F12`) | In egui, `consume_key(Modifiers::NONE, Key::F12)` can ignore extra modifiers if checked first; always check `SHIFT` and `ALT` **before** `Modifiers::NONE`, matching the existing `Shift+F12` ordering. |
| **RISK-04** | `crates/ui-egui/src/preview.rs:710-720` (`Preview::restart`) | `Preview::new_from_slide` sets `course_clone.start_slide = Some(slide_id)` AND must set `self.course_start = Some(slide_id)` so `restart()` returns to the starting slide instead of snapping back to slide 1. |
| **RISK-05** | `crates/ui-egui/src/slides.rs:1402, 1864` (drag-and-drop & `scroll_to_rect`) | Culling off-screen rows must STILL compute `selected_rect` for `editor.selection.slide` (so `scroll_to_rect` works when navigating via Find/Check Course/Course Map to slide 250!) and must still populate `slide_rows` when `editor.ui.slide_drag.is_some()` so dragging past the viewport edge works identically. |
| **RISK-06** | `crates/ui-egui/src/slides.rs` (`ThumbnailCache` invalidation) | A slide thumbnail depends on `slide`, `course.master(slide.master)`, `course.theme`, `course.slide_size`, `course.text_styles`, and `course.variables` (for `%var%` default preview text if shown). Include all of these in `course_visual_fp` / `thumbnail_fingerprint` and test master/theme edits explicitly (`SL-04`). |
| **RISK-07** | `crates/ui-egui/src/guard_tests.rs:214` (`scrolling_the_list_only_touches_visible_thumbnails`) | `TestHarness` renders at `1280×800` or `1280×900` (~6–8 visible thumbnails). Ensure `VISIBLE_LIMIT` accounts for partially clipped top/bottom edge rows (`<= 10` on a single scroll frame). |
| **RISK-08** | `crates/ui-egui/src/lib.rs:1188` (`quiet_button`) | `quiet_button` currently sets `Sense::hover()` without `add_enabled_ui`, so calling `.on_disabled_hover_text(...)` on its return `Response` will NOT fire because egui considers the response "enabled"! Wrap `quiet_button` in `ui.add_enabled_ui(enabled, ...)` (like `icon_button_impl`) so `.on_disabled_hover_text(...)` works reliably. |
| **RISK-09** | `crates/ui-egui/src/snapshot.rs:133-236` (`WINDOWS`) | `uiux_sweep_all_surfaces` checks that every `WindowSpec.shows` string is painted! Updating dialog titles (`"AI Settings"` → `"AI settings"`, etc.) requires updating `WINDOWS` in the exact same step (`S3`). |
| **RISK-10** | M31 merge compatibility | M31 adds `File` shortcuts (`Ctrl+N`, `Ctrl+O`, `Ctrl+S`, `Ctrl+Shift+S`) and a `File` group in `shortcuts.rs`. Implementers of `S6` must read the post-M31 `shortcuts.rs` and `menu.rs` first and extend M31's table in place — never overwrite or duplicate M31's File shortcuts. |

---

## 6. Concrete Test Plan

### 6.1 Label & Copy Consistency Tests (`L-01`–`L-05`)

| ID | Crate / File | Behaviour Tested | Expected Result |
|---|---|---|---|
| **L-01** | `crates/engine` (`tests.rs`) | Registry labels & menu paths follow sentence case (`M32-01`, `M32-03`) | Every `cmd.label` and `cmd.menu` leaf in `REGISTRY` passes sentence-case linter; all 186 command IDs unchanged; `docs/cli.md` sync passes. |
| **L-02** | `crates/ui-egui` (`toolbar_menu_tests.rs`) | Multi-name unification (`M32-02`) | Toolbar shows `"New slide"` (never `"Add slide"`), View menu shows `"Slide masters"` (never `"Slide Masters"` / `"Master View"`), Preview uses `"Preview this slide"` and `"Preview whole course"`, Edit uses `"Select all"` and `"Check spelling…"`, Project has `"Course settings…"` (no duplicate `"Course Properties"` in File). |
| **L-03** | `crates/ui-egui` (`u13_core_authoring_dialogs_tests.rs`, `u14_ai_dialogs_tests.rs`, `u15_status_welcome_tests.rs`) | Dialog titles & panel labels sentence case | Every dialog in `WINDOWS` paints its sentence-case title (`"AI settings"`, `"Check course"`, `"Error history"`, `"Suggest alt text"`, `"Course from document"`, `"Slides from outline"`, `"Record narration"`, `"Report a problem"`, `"Export translation"`, etc.) and passes `uiux_sweep_all_surfaces`. |
| **L-04** | `crates/ui-egui` (`tests.rs`) | Static sentence-case lint over `menu.rs`, `toolbar.rs`, `slides.rs`, and dialog titles | Zero Title Case regressions in menu/toolbar/dialog definitions (allowing explicit proper-noun/acronym set and `"Don't Save"`). |
| **L-05** | `apps/coursecraft` (`m27_files_ui_tests.rs`) | App shell modal buttons | Conflict modal shows `"Save a copy"`, error modal shows `"Open latest version"`, unsaved-changes modal keeps platform-standard `"Don't Save"`. |

### 6.2 Unified Import Menu & Preview Menu Tests (`I-01`–`I-03`, `P-01`–`P-05`)

| ID | Crate / File | Behaviour Tested | Expected Result |
|---|---|---|---|
| **I-01** | `crates/ui-egui` (`toolbar_menu_tests.rs`) | `File ▸ Import ▸` contains all 7 import actions (`M32-08`) | Opening `File ▸ Import` paints `"PowerPoint (.pptx)…"`, `"Course from document…"`, `"Slides from outline…"`, `"Questions from spreadsheet (CSV)…"`, `"Translation (XLIFF)…"`, `"Media files…"`, `"Font…"`; clicking each emits the exact expected `EditorRequest` or opens the dialog. |
| **I-02** | `crates/ui-egui` (`toolbar_menu_tests.rs`) | Old duplicate import items removed | Top-level `File` has no `"Import PowerPoint…"` / `"Import Translation…"` / `"Add font…"`; `Insert` has no `"Slides from Outline…"` / `"Course from Document…"`; `File ▸ Export ▸` contains `"Translation (XLIFF)…"`. |
| **I-03** | `crates/ui-egui` (`toolbar_menu_tests.rs`) | Web mode (`file_ops_available = false`) gating in Import & Export | `"Translation (XLIFF)…"` and `"Media files…"` are disabled with desktop-only reason tooltips; browser-safe imports stay enabled. |
| **P-01** | `crates/ui-egui` (`toolbar_menu_tests.rs`) | Top-level `Preview` menu & toolbar dropdown parity (`M32-06`) | Both paint `"Preview this slide"`, `"Preview from here"`, `"Preview selected slides"`, and `"Preview whole course"` with matching shortcuts and disabled reasons. |
| **P-02** | `crates/ui-egui` (`preview_subset_tests.rs`) | `"Preview from here"` starts at selected slide & continues (`M32-07`) | Select slide 2 of a 4-slide course, trigger **Preview ▸ Preview from here** (or press `Alt+F12`) → preview opens on slide 2, `is_partial_preview() == false`, clicking Next advances to slide 3 (no end-of-preview popup!), and clicking Restart returns to slide 2. |
| **P-03** | `crates/ui-egui` (`preview_subset_tests.rs`) | `"Preview this slide"` (`F12`), `"Preview selected slides"`, `"Preview whole course"` (`Shift+F12`) from Preview menu | Each opens the exact expected preview scope; bank slide disables `"Preview this slide"` and `"Preview from here"` with explanatory tooltips; slide masters mode disables all 4 items with `"Not available in slide masters"`. |

### 6.3 Shared Shortcut Tooltips & Disabled Reason Tests (`SC-01`–`SC-03`, `D-01`–`D-03`)

| ID | Crate / File | Behaviour Tested | Expected Result |
|---|---|---|---|
| **SC-01** | `crates/ui-egui` (`toolbar_menu_tests.rs`) | Toolbar buttons with shortcuts show formatted shortcut in tooltip (`M32-04`) | Hovering `New slide`, `Preview`, `Publish`, `Map`, `Paint`, `Undo`, `Redo` paints tooltips containing `shortcut_label(ctx, sc)` on both macOS and non-macOS formatting. |
| **SC-02** | `crates/ui-egui` (`toolbar_menu_tests.rs`) | Menu items keep right-aligned hint AND show shortcut in tooltip | Opening `File`, `Edit`, `Insert`, `Arrange`, `View`, `Preview`, `Help` and hovering shortcutted items paints tooltips containing the exact same formatted shortcut string shown in the menu hint and in `build_shortcut_groups()`. |
| **SC-03** | `crates/ui-egui` (`text_format_tests.rs`) | Inspector Bold/Italic/Underline tooltips use platform shortcut formatting | On macOS context (`is_mac = true`), Bold/Italic/Underline tooltips show `⌘B` / `⌘I` / `⌘U`; on Windows/Linux, `Ctrl+B` / `Ctrl+I` / `Ctrl+U`. |
| **D-01** | `crates/ui-egui` (`toolbar_menu_tests.rs`) | Every audited disabled control shows a clear reason (`M32-05`) | Hovering disabled Undo/Redo, New layer, Preview selected slides, Move up/down, required drop-target checkbox (`question.rs:1876`), question reorder/remove buttons, trigger delete while form open, busy publish controls, and Edit-State Animate tab each paints a non-empty reason tooltip (never just the action name!). |
| **D-02** | `crates/ui-egui` (`tests.rs`) | Static CI guard `every_add_enabled_has_disabled_reason` | Scans production `.rs` files in `crates/ui-egui/src` and `apps/coursecraft/src` and asserts every `add_enabled` / `add_enabled_ui` has `.on_disabled_hover_text`. |
| **D-03** | `crates/ui-egui` (`uiux_sweep_tests.rs`) | Full UI/UX sweep (`UX-08`, `UX-10`, `UX-11`, `UX-04`) | `uiux_sweep_all_surfaces`, `uiux_sweep_scale_2_and_fixtures`, and `uiux_sweep_modals_and_tooltips` pass with **0 MUST violations**. |

### 6.4 Slide-List Speed Guard & Perf Tests (`SL-01`–`SL-06`)

| ID | Crate / File | Behaviour Tested | Expected Result |
|---|---|---|---|
| **SL-01** | `crates/ui-egui` (`guard_tests.rs`) | `idle_frames_do_zero_thumbnail_composes` (`M32-09`) | 300-slide perf course settled (4 frames), reset counts, run 20 idle frames → `gc::thumbnail_composes() == 0`, `gc::thumbnail_redraws() == 0`. |
| **SL-02** | `crates/ui-egui` (`guard_tests.rs`) | `moving_object_composes_only_one_thumbnail` (`M32-10`) | 300-slide perf course settled, select object on slide 0, reset counts, press `ArrowRight` → `gc::thumbnail_composes() == 1` (not 6, not 300!). |
| **SL-03** | `crates/ui-egui` (`guard_tests.rs`) | `scrolling_300_slide_list_composes_only_visible_rows` (`M32-09`, `M32-10`) | 300-slide perf course settled, reset counts, send 1 wheel event scrolling down → `1 <= gc::thumbnail_composes() <= 10`; scrolling back up to already-cached rows at same revision → `gc::thumbnail_composes() == 0`! |
| **SL-04** | `crates/ui-egui` (`guard_tests.rs`) | `editing_master_or_theme_invalidates_visible_thumbnails` (`RISK-06`) | Slide 0 uses master `m1`; settle; edit object on `m1` (or change course theme background) → visible slide 0 recomposes (`thumbnail_composes() >= 1`) and reflects the new colour/object. |
| **SL-05** | `crates/ui-egui` (`slides.rs` tests) | Cache cap (`<= 128`), `set_course` clear, off-screen `scroll_to_rect`, and drag-reorder (`M32-11`, `RISK-05`) | Scrolling across all 300 slides keeps `thumbnail_cache.len() <= 128`; `set_course` empties cache; selecting `s-250` scrolls `s-250` into view; dragging slide 0 to index 2 still reorders properly. |
| **SL-06** | `crates/ui-egui` (`perf_tests.rs`, release `--ignored`) | Release 300-slide frame timings (`M32-12`) | `idle`, `select slide 150`, `scroll slide list`, `visit every slide` all stay well under budget and improve cleanly vs the `1.37 / 1.59 / 1.32 / 1.41 ms` baseline. |

---

## 7. Numbered Implementer Steps (9 Steps, ~30–40 min each)

> **Reviewer notes (Claude, binding for implementers)**
> - This plan was written against main before M31 merged. M31 adds Save/Save As/Open/New shortcuts, File menu hints and a File group in the shortcuts dialog; line numbers will have moved. Build on M31's shortcut table in S6, don't duplicate it.
> - The label pass changes **labels only**: command ids, MCP tool names, CLI flags, `course.json` fields and learner-facing player string **keys** stay exactly as they are. Update `docs/cli.md` wherever a changed label appears there.
> - S5: before adding `Alt+F12` (or any new shortcut), check it clashes with nothing in the shortcut table and isn't a common macOS or browser system key; pick another if it is.
> - S8: measure with the plan's counters before and after; if the frame-time targets aren't met, report the real numbers rather than loosening a test.

> **Gate cadence:** Run targeted `cargo test -p <crate> -j 4` on every step. Run full `cargo xtask ci` after **S3, S5, S7, and S9**. Run release `perf_tests` and `cargo xtask perf` at **S9**.

- **S1 — Sentence-Case Pass on Command Registry (`coursecraft-engine`)**
  - **Files:** `crates/engine/src/commands.rs` (`REGISTRY` `label` & `menu` fields only — zero `id` changes), `crates/engine/src/tests.rs` (`undo_names`/`redo_names` assertions + new `L-01` lint test), `crates/ui-egui/src/{history_tests,m27_undo_clipboard_tests,tests}.rs` (update any test matching exact command labels/menu names).
  - **Exact changes:**
    1. Convert all Title Case `label` and `menu` leaf strings in `REGISTRY` to sentence case per §3.1.
    2. Change `course.update`'s `menu` from `"File/Course Properties"` to `"Project/Course settings…"` (eliminating the duplicate between File and Project; remove the manual `"Course settings"` button in `menu.rs:1841` in this step or `S2`). Keep `course.import_outline` at `"Insert/Slides from outline…"` until `S4` moves it into `File/Import/`.
    3. Add helper in `menu.rs:946-963` so `"Undo {name}"` / `"Redo {name}"` lowercases the leading word unless it begins with a proper noun/acronym (`AI`, `CSV`, `XLIFF`, `Lottie`, `PowerPoint`, `PDF`, `MP4`, `Word`).
    4. Update tests matching old Title Case undo/menu names and add `L-01`.
  - **Tests:** `L-01`, `cargo test -p coursecraft-engine -j 4`, `cargo test -p coursecraft-cli -j 4`, `cargo test -p coursecraft-ui-egui -j 4 history`.
  - **Done when:** Every registry label and menu path is sentence case, zero command IDs changed, and all engine/CLI/history tests pass.

- **S2 — Sentence-Case Pass & Multi-Name Unification on Menus, Toolbar & Navigator**
  - **Files:** `crates/ui-egui/src/{menu,toolbar,slides,media_library,shortcuts}.rs`, `crates/ui-egui/src/{toolbar_menu_tests,tests,walkthrough_tests,slide_delete_tests,u9_navigator_canvas_tests,bughunt_menus}.rs`.
  - **Exact changes:**
    1. Apply every sentence-case & multi-name unification change from §3.1 in `menu.rs`, `toolbar.rs`, `slides.rs`, `media_library.rs`, and `shortcuts.rs` (`"New slide"`, `"Slide masters"`, `"Close slide masters"`, `"Not available in slide masters"`, `"Preview this slide"`, `"Preview whole course"`, `"Select all"`, `"Check spelling…"`, `"Open folder…"`, `"Open recent"`, `"Save as…"`, `"Save as project folder (for git)…"`, `"Version history…"`, `"Snap to guides"`, `"Snap to grid"`, `"Align left"`…`"Align to slide"`, `"AI settings…"`, `"Suggest alt text…"`, `"Save as template"`, `"Text box"`, `"Record narration…"`, `"Narration from text…"`, `"Report a problem…"`, `"Suggest a feature…"`, `"Check for updates"`, `"Format painter"`, `"Course/Slide/Animate/Format inspector"`, `"Insert text box"`…`"Insert trigger"`, `"View & canvas"`).
    2. Unify template delete button in `slides.rs:766` (`"Remove"` → `"Delete"`).
    3. Update matching test assertions in the same step and add `L-02`.
  - **Tests:** `L-02`, `cargo test -p coursecraft-ui-egui -j 4 toolbar_menu`, `cargo test -p coursecraft-ui-egui -j 4 bughunt_menus`.
  - **Done when:** Menus, toolbar, and navigator have 100% sentence case and one name per action, with all tests green.

- **S3 — Sentence-Case Pass on Dialogs, Panels, Inspector, Status Bar & App Shell**
  - **Files:** `crates/ui-egui/src/{ai_settings,alt_text,audio_ai,document,outline,record_narration,report,spell_check,status,translation,preview,properties,question,states,triggers,variables,snapshot}.rs`, `apps/coursecraft/src/main.rs` (`:2974, 3052`), affected test files (`u13_*`, `u14_*`, `u15_*`, `bughunt_m26_*`, `f24_*`, `m27_files_ui_tests.rs`, `tests.rs`).
  - **Exact changes:**
    1. Apply every dialog/panel/inspector/shell label change listed in §3.1 item 4 (keep `"Don't Save"` in `apps/coursecraft/src/main.rs:2925`).
    2. Update `WINDOWS` `shows` needles in `crates/ui-egui/src/snapshot.rs` (`RISK-09`).
    3. Update matching test assertions in the same step and add `L-03`, `L-04`, `L-05`.
  - **Tests:** `L-03`, `L-04`, `L-05`.
  - **Gate #1:** `cargo xtask ci` (confirms all label updates and `uiux_sweep` pass cleanly).
  - **Done when:** Zero Title Case drift remains across editor and shell, `L-01`–`L-05` pass, and CI is green.

- **S4 — Unified `File ▸ Import ▸` Menu & Symmetric `File ▸ Export ▸` Menu**
  - **Files:** `crates/engine/src/commands.rs` (`course.import_outline` `menu: "File/Import/Slides from outline…"`), `crates/ui-egui/src/menu.rs`, `crates/ui-egui/src/{toolbar_menu_tests,tests,outline_tests,document_tests,m27_find_a11y_tests}.rs`.
  - **Exact changes:**
    1. Build the unified `File ▸ Import ▸` submenu in `menu.rs` with all 7 items in §3.5 order (`PowerPoint (.pptx)…`, `Course from document…`, `Slides from outline…`, `Questions from spreadsheet (CSV)…`, `Translation (XLIFF)…`, separator, `Media files…`, `Font…`), respecting `desktop_file_ops` with `FILE_OPS_DISABLED_MSG`.
    2. Move `Export translation…` into `File ▸ Export ▸ Translation (XLIFF)…`.
    3. Remove old top-level `File` duplicates (`Import PowerPoint…`, `Import Translation…`, `Add font…`) and old `Insert` duplicates (`Slides from outline…`, `Course from document…`).
    4. Skip `"Import/"` in the `File/` registry loop (`RISK-02`) so `course.import_outline` renders only inside `File ▸ Import ▸`, keeping `menu_routes_cover_every_menu_command` green.
    5. Update tests that opened Outline/Document/PPTX/Translation from old menu paths and add `I-01`, `I-02`, `I-03`.
  - **Tests:** `I-01`, `I-02`, `I-03`, `cargo test -p coursecraft-ui-egui -j 4`.
  - **Done when:** Every import lives under `File ▸ Import ▸` with zero scattered duplicates and all tests pass.

- **S5 — Top-Level `Preview` Menu & `"Preview from here"` (`Alt+F12`)**
  - **Files:** `crates/ui-egui/src/{preview,lib,menu,toolbar,shortcuts}.rs`, `crates/ui-egui/src/{preview_subset_tests,toolbar_menu_tests}.rs`.
  - **Exact changes:**
    1. Add `Preview::new_from_slide(course: &Course, slide_id: &str) -> Self` in `preview.rs` (sets `course_clone.start_slide = Some(slide_id)`, `course_start = Some(slide_id)`, `preview_subset = None`, `Player::with_draw_seed`) and `Editor::open_preview_from(&mut self, slide_id: &str)` in `lib.rs` (falls back to `open_preview_at(None)` if bank slide/empty).
    2. Extract `pub(crate) fn show_preview_menu_items(editor: &mut Editor, ui: &mut Ui, requests: &mut Vec<EditorRequest>)` implementing all 4 items (**Preview this slide**, **Preview from here**, **Preview selected slides**, **Preview whole course**) with master-view, bank-slide, and selection-count disabled reasons.
    3. Add top-level **`Preview`** menu in `menu.rs` (before `Help`) calling `show_preview_menu_items`, and update the toolbar Preview caret popup (`toolbar.rs:454`) to call the exact same `show_preview_menu_items`.
    4. Bind `Alt+F12` in `lib.rs:4555` (before plain `F12`, `RISK-03`) to `open_preview_from` and add `"Preview from here"` (`Alt+F12`) to `shortcuts.rs`.
    5. Add `P-01`, `P-02`, `P-03`.
  - **Tests:** `P-01`, `P-02`, `P-03`.
  - **Gate #2:** `cargo xtask ci`.
  - **Done when:** All 4 Preview actions work identically from the Preview menu and toolbar dropdown, `"Preview from here"` plays from the selected slide through the rest of the course, and CI is green.

- **S6 — Single-Source Shortcut Table & Tooltips Across Toolbar, Menus & Inspector**
  - **Files:** `crates/ui-egui/src/{shortcuts,menu,toolbar,slides,properties}.rs`, `crates/ui-egui/src/{toolbar_menu_tests,text_format_tests}.rs`.
  - **Exact changes:**
    1. Read post-M31 `shortcuts.rs` & `menu.rs` (`RISK-10`). Extend M31's shortcut table with `ShortcutAction` + `shortcut_for_action(ShortcutAction)` + `shortcut_for_command(id)` so every registry and editor/shell shortcut has one definition.
    2. Add `tooltip_with_shortcut(ctx, base, shortcut)` and `menu_button(ui, label, shortcut, enabled, disabled_reason)` in `menu.rs` (setting both `.shortcut_text` and `.on_hover_text` when enabled, `.on_disabled_hover_text` when disabled).
    3. Wire every toolbar button with a shortcut (`New slide`, `Preview`, `Publish`, `Map`, `Paint`, `Undo`, `Redo`), every shortcutted menu/context-menu item (including `Preferences…` `Ctrl+,`, `Keyboard shortcuts` `?`, `View ▸ Course map` `Ctrl+Shift+M`, `View ▸ Zoom` items, and M31's `File` items), and inspector `Bold`/`Italic`/`Underline` toggles (`properties.rs`) through the shared helpers.
    4. Add `SC-01`, `SC-02`, `SC-03`.
  - **Tests:** `SC-01`, `SC-02`, `SC-03`, `cargo test -p coursecraft-ui-egui -j 4 toolbar_menu`.
  - **Done when:** Every toolbar button and menu item with a shortcut displays its platform-formatted shortcut in its tooltip from the single shared table, and drift tests pass.

- **S7 — Disabled-Control Reason Sweep & Static Guard Test**
  - **Files:** `crates/ui-egui/src/{widgets,lib,menu,toolbar,slides,question,triggers,publish,ai_settings,properties}.rs`, `crates/ui-egui/src/{toolbar_menu_tests,tests}.rs`.
  - **Exact changes:**
    1. Add `icon_button_enabled_reason` and `icon_button_danger_reason` in `widgets.rs` (taking separate `hover_tooltip` and `disabled_reason`), keeping existing `icon_button_enabled`/`icon_button_danger` delegating for any always-enabled or already-reasoned callers.
    2. Wrap `quiet_button` (`lib.rs:1188`) in `ui.add_enabled_ui(enabled, ...)` (`RISK-08`) so `.on_disabled_hover_text(...)` fires on disabled quiet buttons.
    3. Fix every single site in the §3.3 table (#1–#15), including fixing the `.on_hover_text` bug on the disabled checkbox in `question.rs:1876`, removing `add_enabled(true, ...)` at `slides.rs:2198`, and giving Undo/Redo, New layer, Move up/down, Preview selected slides, question controls, trigger delete, busy publish controls, agent quiet buttons, and Animate tab clear disabled reasons.
    4. Add `D-01` (runtime hover test across disabled controls) and `D-02` (static `every_add_enabled_has_disabled_reason` guard test).
  - **Tests:** `D-01`, `D-02`, `D-03`.
  - **Gate #3:** `cargo xtask ci`.
  - **Done when:** Zero disabled controls lack an explanatory hover reason, static guard passes with 0 exceptions, and CI is green.

- **S8 — Visible-Only Slide-List Composition & Revision-Keyed `ThumbnailCache`**
  - **Files:** `crates/ui-egui/src/{slides,lib,course_map,guard_counters}.rs`.
  - **Exact changes:**
    1. Add `THUMBNAIL_COMPOSES` (`note_thumbnail_compose`, `thumbnail_composes`) to `guard_counters.rs` (reset in `guard_counters::reset()`).
    2. Implement `ThumbnailCache` (`MAX_CACHED_THUMBNAILS = 128`, `(verified_revision, fingerprint, bg, Arc<[DrawItem]>, last_used)`) and deterministic `thumbnail_fingerprint(course, slide, course_visual_fp)` in `slides.rs`, stored on `UiState` and cleared in `Editor::set_course`.
    3. Add revision-cached `NavigatorIndex` on `UiState` so `show_slides_content` does not clone 300 `SlideSummary` `String`s every frame.
    4. Move viewport culling (`row_rect.intersects(ui.clip_rect())`) **before** `ThumbnailCache::get_or_compose`, background/border/gutter/title paint, and `ui.interact` in `show_slides_content` (normal slides + bank slides) and `show_masters_panel`, while preserving `selected_rect` and `slide_drag` geometry (`RISK-05`).
    5. Reuse `editor.ui.thumbnail_cache` + viewport culling (`thumb_rect.intersects(ui.clip_rect())`) in `course_map.rs:337`.
    6. Call `#[cfg(test)] crate::guard_counters::note_thumbnail_compose();` right beside `compose_authoring` on cache misses.
  - **Tests:** `SL-05`, `cargo test -p coursecraft-ui-egui -j 4 guard_tests`, `cargo test -p coursecraft-ui-egui -j 4 u9_navigator`.
  - **Done when:** Off-screen thumbnails do zero work, visible thumbnails reuse cached `Arc<[DrawItem]>`, master/theme/slide edits invalidate accurately, and existing navigator/drag/scroll tests stay green.

- **S9 — Slide-List Guard Tests, Release Perf Measurement, Docs & Final M32 Gate**
  - **Files:** `crates/ui-egui/src/guard_tests.rs`, `docs/performance.md`, `docs/testing/m32-report.md` (new report).
  - **Exact changes:**
    1. Add `SL-01` (`idle_frames_do_zero_thumbnail_composes`), `SL-02` (`moving_object_composes_only_one_thumbnail`), `SL-03` (`scrolling_300_slide_list_composes_only_visible_rows`), and `SL-04` (`editing_master_or_theme_invalidates_visible_thumbnails`) to `crates/ui-egui/src/guard_tests.rs` (plus assert `gc::thumbnail_composes() == 0` inside existing `idle_frames_do_no_counted_work`!).
    2. Run release `perf_tests` (`cargo test --release -p coursecraft-ui-egui perf_tests -- --ignored --nocapture --test-threads=1`) and record actual before/after numbers in `docs/performance.md` and `docs/testing/m32-report.md`.
    3. Write concise `docs/testing/m32-report.md` summarizing all test IDs (`L-01`–`L-05`, `I-01`–`I-03`, `P-01`–`P-03`, `SC-01`–`SC-03`, `D-01`–`D-03`, `SL-01`–`SL-06`), UI/UX sweep status, and before/after measurements.
  - **Tests:** `SL-01`–`SL-06`, `D-03`.
  - **Final Release Gate:** `cargo xtask ci`, `cargo xtask build-player`, `cargo xtask perf`.
  - **Done when:** All M32 tests, guard tests, UI/UX sweeps, and release perf benchmarks pass and report is written.

---

## 8. Release Gate for M32

### 8.1 Automated Release Gate Commands
Run in order before cutting the release:
1. `cargo xtask ci` (`fmt --check`, `clippy --workspace --all-targets -D warnings`, `test --workspace`, `layers`, `wasm`)
2. `cargo xtask build-player` (confirm embedded web player builds cleanly)
3. `cargo test -p coursecraft-ui-egui -j 4 uiux_sweep` (0 MUST violations across all surfaces, themes, sizes, and scales)
4. `cargo test -p coursecraft-ui-egui -j 4 guard_tests` (confirms 0 idle thumbnail composes, 1 compose on single-slide edit, ≤ 10 on scroll)
5. `cargo xtask perf` (records post-M32 release timings on the 300-slide course and big-course suite)

### 8.2 Owner Manual Verification Checklist (Desktop App Only — 8 Minutes)
1. **Labels & consistency:** Open each top-level menu (`File`, `Edit`, `Insert`, `Arrange`, `Project`, `Slide`, `View`, `Preview`, `Help`). Verify every item uses sentence case (no leftover Title Case like `"Save As…"` or `"Slide Masters"`), toolbar reads **`New slide`** (not `"Add slide"`), and `View ▸ Slide masters` matches **`Close slide masters`**.
2. **Shortcut tooltips:** Hover **`New slide`**, **`Preview`**, **`Publish`**, **`Map`**, **`Paint`**, **`Undo`**, **`Redo`** on the toolbar and hover shortcutted items in `File`, `Edit`, `View`, `Preview`, and `Help` menus — verify macOS shows `⌘` symbols (and Windows/Linux shows `Ctrl+`), matching **Help ▸ Keyboard shortcuts** (`?`). Select a text box and hover `B` / `I` / `U` in Format inspector — verify macOS shows `Bold (⌘B)`.
3. **Disabled reasons:** On a freshly opened course with nothing selected, hover disabled **`Undo`**, **`Redo`**, **`Paint`**, **Arrange ▸ Group**, slide context menu **`Move up`** (on slide 1) and **`Preview selected slides`** (with 1 slide selected). Verify every disabled control shows a clear, short reason (never blank and never just repeating the button name).
4. **Preview menu:** On slide 2 of `examples/showcase/`, test all four items in the top-level **Preview** menu:
   - **Preview this slide (`F12`)** → plays slide 2 only; clicking Next shows the end-of-preview popup.
   - **Preview from here (`⌥F12` / `Alt+F12`)** → starts on slide 2 and clicking Next continues to slide 3; clicking Restart returns to slide 2.
   - Select slides 2 & 4 (`⌘`-click) → **Preview selected slides** plays only those 2 slides.
   - **Preview whole course (`⇧F12` / `Shift+F12`)** → plays from slide 1.
5. **One Import menu:** Open **File ▸ Import ▸** and verify all 7 imports (`PowerPoint (.pptx)…`, `Course from document…`, `Slides from outline…`, `Questions from spreadsheet (CSV)…`, `Translation (XLIFF)…`, `Media files…`, `Font…`) are gathered in one submenu, and **File ▸ Export ▸** holds `Video (MP4)…`, `PDF for review…`, `Word for review…`, and `Translation (XLIFF)…`.
6. **Slide-list smoothness:** Open a large multi-slide course (or duplicate slides to ~100+), scroll the left slide list quickly up and down, nudge an object on a visible slide, and edit a slide master — verify scrolling feels instant and thumbnails update immediately with zero flicker or stale previews.
