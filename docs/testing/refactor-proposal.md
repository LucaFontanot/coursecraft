# CourseCraft Codebase Refactor Proposal

> **STATUS: PROPOSAL ONLY.** No code has been changed, and **no agent or contributor may act on these proposals without explicit owner approval.**

**Context:** While auditing the entire workspace (~221,000 lines of production Rust + ~163,000 lines of tests across 20 library crates and 6 app crates) for the M34 performance plan (`docs/testing/m34-plan.md`), we systematically catalogued where the code structure itself causes bugs, slows down both human and AI implementers, blocks zero-copy performance optimizations, or makes unit testing unnecessarily heavy.

**Overall verdict:** CourseCraft's **crate layering (L0–L6)** and **architectural invariants** (one runtime in `coursecraft-logic`, command-only mutations in `coursecraft-engine`, zero panics, WASM parity) are **exceptionally clean and do NOT need a rewrite**. However, **8 specific files and 2 data-access seams have outgrown single-file scale** (spanning 3,900–20,100 lines each, with single structs holding 145–209 fields and single functions spanning 700–2,029 lines). Targeted, incremental module splits — mostly **after** M34, with 3 small high-leverage API/seam refactors done **with** M34 — will dramatically speed up future milestones and prevent recurring classes of bugs.

---

## 1. Structural Evidence Snapshot (Measured on Current Checkout)

### 1.1 Largest Production Files (Non-Test)

| File | Lines | Primary Pain Point |
|---|---:|---|
| `crates/engine/src/commands.rs` | **13,580** | All 186 commands in one file (`fn interaction_insert`: **1,225 lines**); high merge conflict rate, slow incremental edits |
| `crates/ui-egui/src/properties.rs` | **10,889** | 209-field `PropertiesState` god struct; `fn show_object_properties`: **2,029 lines**; `show_multi_properties`: **721 lines**; `show_text_controls`: **673 lines** |
| `crates/ui-egui/src/canvas.rs` | **7,834** | Mixes compose, drag/resize/rotate (`process_active_drag`: **775 lines**), inline text edit, crop, draw tool, snap guides, overlays (`show_canvas_and_layers`: **555 lines**) |
| `crates/pptx/src/lib.rs` | **6,019** | ZIP unpack + XML parsing + slide/shape/table/text/image mapping in one file (`process_shape`: **521 lines**, `import_pptx_with_limits`: **471 lines**) |
| `crates/ui-egui/src/lib.rs` | **5,527** | 170+ `mod` declarations, 145-field `UiState` + 30-field `Editor` god structs, `Editor::show` (**705 lines**), media import helpers, duplicated media walkers |
| `crates/logic/src/run.rs` | **4,563** | All trigger/event/action/question/timeline execution in one file (cleanly cohesive, lower urgency than UI/engine) |
| `crates/course/src/model.rs` | **4,557** | Every serde model struct/enum for Course, Slide, Object, Style, Question, Trigger, Player, Theme in one file |
| `crates/render/src/paint.rs` | **4,088** | `fn paint_items`: **1,177 lines** handling all 26 `Visual` variants + effects + transforms inline |
| `apps/coursecraft/src/main.rs` | **3,928** | Desktop shell + Web shell interleaved with `#[cfg(wasm32)]`, `show_app`: **632 lines**, `EditorHostAdapter` embedded inline |
| `crates/course/src/validate.rs` | **3,569** | Course, slide, object, trigger, question, master, bank validation in one file |

### 1.2 Largest Monolithic Test Files

| File | Lines | Pain Point |
|---|---:|---|
| `crates/ui-egui/src/tests.rs` | **20,159** | Single file holding hundreds of unrelated editor tests; slows edits, search, and cognitive navigation |
| `crates/engine/src/tests.rs` | **12,189** | Monolithic engine unit test file |
| `crates/logic/src/tests.rs` | **11,406** | Monolithic runtime test file |
| `crates/render/src/tests.rs` | **8,714** | Monolithic compositor/painter test file |

---

## 2. Ranked Refactor Proposals (`R1`–`R9`)

Ranked by **return on investment** (bugs prevented + performance unlocked + developer/AI speed gained per unit of effort and risk).

---

### R1 — Unify Media Reference Walking & Replace Raw `Vec` Media Store with Indexed `MediaStore`

- **Timing vs M34:** **With M34** *(direct enabler for P2, P5, P9)*
- **Rough size:** **2 steps** (small)
- **The problem (concrete evidence):**
  1. **4 separate copies of the exact same media-reference walker** exist across 4 crates:
     - `crates/course/src/lib.rs:216` (`walk_object_media_references`)
     - `crates/publish/src/lib.rs:95` (`collect_object_media`)
     - `crates/ui-egui/src/lib.rs:5075` (`collect_object_media_srcs` / `referenced_media_srcs`)
     - `apps/coursecraft-player/src/media.rs:128` (`collect_object_srcs`)
     Every time a new media-bearing field is added (as happened in M29 for `state.fill_style` and `state.extras`, and in M27 for question-bank web folders), at least one of the 4 walkers silently misses it, causing missing-media publish errors or lost assets on Save!
  2. **O(M) linear scans everywhere:** `Editor.media` (`crates/ui-egui/src/lib.rs:1144`) is a bare `pub media: Vec<(String, Arc<[u8]>)>`. `MemoryMediaSource::get` (`:4951`), `Editor::get` (`:4962`), `refresh_validate_only` (`:4142`, O(R × M)!), `missing_images_as_placeholders` (`canvas.rs:5325`), and `timeline_playback::sync_slide` (`:305`) all scan the `Vec` linearly.
- **Proposed shape:**
  1. In `crates/course/src/lib.rs` (L1), add one canonical visitor:
     ```rust
     pub enum MediaScope { Publishable, AllIncludingTemplates }
     pub fn visit_course_media<'a>(
         course: &'a Course,
         scope: MediaScope,
         visit: &mut dyn FnMut(MediaRef<'a>),
     );
     pub fn visit_slide_media<'a>(slide: &'a Slide, visit: &mut dyn FnMut(MediaRef<'a>));
     ```
     Replace all 4 hand-rolled walkers (`course::media_references`, `publish::collect_object_media`, `ui-egui::referenced_media_srcs`, `player::slide_media_srcs`) with thin 5-line adapters over `visit_course_media` / `visit_slide_media`.
  2. In `crates/ui-egui/src/media_store.rs`, wrap `Editor.media` in a small `MediaStore` preserving deterministic insertion order (`entries: Vec<(String, Arc<[u8]>)>`) + `index: HashMap<String, usize>` + `total_bytes: usize` (with `as_slice()`, `get(&str)`, `contains(&str)`, `insert()`, `replace()`, `retain()`, `total_bytes()`).
- **Benefit:** Permanently eliminates an entire recurring class of lost/missing-media bugs, unlocks O(1) media lookups and O(1) raw-media byte tracking for M34, and deletes ~300 lines of duplicated match arms.
- **Risk:** Low — `Editor.media` is `pub` and touched directly by ~40 tests; either keep thin transition methods or update test call sites (`editor.media.push` → `editor.add_raw_media` / `Deref` for reads) in one clean mechanical pass.

---

### R2 — Zero-Copy Shared-Bytes APIs in `coursecraft-format` and `coursecraft-publish`

- **Timing vs M34:** **With M34** *(direct enabler for P5 — −1.35 GB peak RAM)*
- **Rough size:** **2–3 steps** (small)
- **The problem (concrete evidence):**
  - `crates/publish/src/lib.rs:56`: `PackageInput::media: &'a dyn Fn(&str) -> Option<Vec<u8>>` and `PackageFile { pub bytes: Vec<u8> }` force `apps/coursecraft/src/publish.rs:79` to call `bytes.to_vec()` on every asset, duplicating up to **1.36 GB** of videos/images in RAM during Publish (explicitly flagged as unfinished debt in `docs/performance.md:85`!).
  - `crates/format/src/lib.rs:368`: `open_project(bytes: &[u8])` forces `apps/coursecraft/src/files.rs:469` (`std::fs::read(path)`) to load the entire `.ccraft` file into RAM before unzipping into a second copy (**2,735 MB** peak RSS for a 1,362 MB project!).
- **Proposed shape:**
  - `crates/format`: Add `open_project_from<R: Read + Seek>(reader: R)` and `open_project_from_with_limits` as the primary implementation; make `open_project(bytes: &[u8])` a 1-line wrapper using `Cursor::new(bytes)`.
  - `crates/publish`: Change `PackageFile::bytes` to `Arc<[u8]>` (or introduce `SharedPackageFile` / `SharedPackageInput` alongside a thin compatibility wrapper) and add `zip_package_streaming` / `zip_shared_package_to`.
- **Benefit:** Cuts Open and Publish peak memory in **half** (**−1.35 GB** on big courses), making 1 GB+ projects openable and publishable on 4 GB RAM laptops.
- **Risk:** Low — clean additive API if `SharedPackageInput` is added first, or small call-site updates across `publish`, `apps/coursecraft`, and `apps/coursecraft-cli`.

---

### R3 — Decouple `coursecraft-player` from `coursecraft-format` & Workspace `default_fonts`

- **Timing vs M34:** **With M34** *(direct enabler for P3 & P8 — ~1.8 MB WASM savings)*
- **Rough size:** **1 step** (tiny)
- **The problem (concrete evidence):**
  - `apps/coursecraft-player/Cargo.toml:16` pulls in the entire `coursecraft-format` crate (`zip`, `quick-xml`, 2,052-line `xliff.rs`) solely to call `coursecraft_format::is_safe_relative_path` in 3 lines of `apps/coursecraft-player/src/media.rs:38,276,287` — while `coursecraft-course` already owns `is_safe_media_path`.
  - Root `Cargo.toml:45-46` enables `default_fonts` workspace-wide on `egui` and `eframe`, forcing `epaint_default_fonts` (~1.5–1.7 MB of Ubuntu-Light, Hack, NotoEmoji TTFs) into every published learner WASM binary even though `coursecraft-render` bundles Inter + Phosphor.
- **Proposed shape:**
  - Unify path validation in `coursecraft-course::is_safe_media_path`, re-export from `coursecraft-format`, remove `coursecraft-format` from `apps/coursecraft-player/Cargo.toml`.
  - Remove `default_fonts` from root `Cargo.toml` `egui`/`eframe` dependencies; keep `coursecraft-render/src/font.rs` as the single owner of bundled font definitions.
- **Benefit:** Immediate ~1.6–2.0 MB WASM reduction, cleaner dependency graph, faster player compile times.
- **Risk:** Low — verify bare `egui::Context::default()` tests install fonts and verify UI glyph coverage in Inter + Phosphor.

---

### R4 — Split `crates/ui-egui/src/properties.rs` (10,889 Lines, 209-Field `PropertiesState`)

- **Timing vs M34:** **After M34**
- **Rough size:** **5 steps** (medium)
- **The problem (concrete evidence):**
  - At **10,889 lines**, `properties.rs` is the largest UI file in the workspace.
  - `PropertiesState` (`properties.rs:58-420`) has **209 flat fields** spanning Format, Animate, Slide, Course, Multi-selection, Text, Shape, Path, Image, Media, Lottie, Web, Input, Timer, Meter, Marker, ScrollPanel, and Table buffers — plus test-hook `Option<Rect>` fields interleaved directly with edit state!
  - `show_object_properties` is **2,029 lines** (`:7471`), `show_multi_properties` is **721 lines** (`:3217`), `show_text_controls` is **673 lines** (`:4360`), and `sync` is **443 lines** (`:614`).
  - Finding a single inspector bug requires scanning thousands of lines, and unit-testing one control group (e.g., shadow/glow or color picker) requires compiling and navigating the entire 11k-line monolith.
- **Proposed shape:** Convert `crates/ui-egui/src/properties.rs` into a directory module with **zero public API breakage** (`pub use` re-exports from `properties/mod.rs`):
  ```text
  crates/ui-egui/src/properties/
    mod.rs           // InspectorTab, show_properties_panel (~250 lines)
    state.rs         // PropertiesState grouped into sub-structs (TextProps, ShapeProps, MediaProps, MultiProps, TestRects)
    sync.rs          // PropertiesState::sync + revision/selection diffing
    color_picker.rs  // publish_theme_colors, swatches, recent colors, hex/alpha widgets
    format.rs        // Format tab dispatch + Frame/Transform/Arrange/States/Effects sections
    text.rs          // Text content, rich-text toolbar, font picker wiring, links, paragraphs
    media_inputs.rs  // Image, Audio, Video, Lottie, Web, Input, Timer, Meter, Marker, Table sections
    multi.rs         // Multi-selection mixed-value inspector
    animate.rs       // Animate tab (entrance, exit, motion paths, timing)
    slide.rs         // Slide & master inspector tab
    course.rs        // Course inspector tab
  ```
- **Benefit:** Cuts largest function from 2,029 lines → ~250 lines; makes each inspector tab independently readable, testable, and parallelizable across agents/contributors with near-zero merge conflicts.
- **Risk:** Low–Medium — purely mechanical move if done first without renaming public fields on `PropertiesState`, followed by optional sub-struct grouping once tests are green.

---

### R5 — Split `crates/engine/src/commands.rs` (13,580 Lines, 186 Commands) by Domain

- **Timing vs M34:** **After M34**
- **Rough size:** **4 steps** (medium)
- **The problem (concrete evidence):**
  - `crates/engine/src/commands.rs` is **13,580 lines** — the single largest production file in the entire repository.
  - `fn interaction_insert` alone is **1,225 lines** (`:10992`) constructing 8 multi-object/multi-layer interaction templates inline.
  - Every milestone that adds or edits a command touches the same giant file, causing merge conflicts and forcing AI agents to read multi-thousand-line offsets just to inspect one command.
  - Command metadata (`REGISTRY` entry, `undo_scope(id)` match arm in `:42-192`, and `fn ...` handler) live in **3 separate places thousands of lines apart**, making it easy to forget `undo_scope` or desync param docs.
- **Proposed shape:**
  ```text
  crates/engine/src/commands/
    mod.rs           // registry(), CommandSpec table, shared req/opt/keep_valid helpers
    course.rs        // course.*, theme.*, view.*, resource.*, glossary.*, variable.*
    slide.rs         // slide.*, scene.*, master.*, template.*, bank.*, draw.*, results.*, certificate.*
    object.rs        // object.* (geometry, style, fill, shadow, glow, states, extras, arrange, group)
    text.rs          // text.*, text_style.*
    table.rs         // table.*
    layer.rs         // layer.*
    trigger.rs       // trigger.*
    question.rs      // question.*
    interaction.rs   // interaction.insert (split per interaction kind!)
    media_video.rs   // video.*, captions, audio/media helpers
    styles.rs        // existing
    csv_import.rs    // existing
  ```
  Additionally, attach `scope: UndoScope` directly as a field on `Command` in `REGISTRY` so command id, label, menu, shortcut, params, mutates, handler, and `UndoScope` live in **one row** (eliminating the separate 150-line `undo_scope(id)` match table!).
- **Benefit:** Eliminates the #1 merge-conflict hotspot in the codebase, prevents `UndoScope` drift on future commands, and cuts per-file navigation size by ~10×.
- **Risk:** Very low — purely internal to `crates/engine` (`pub fn registry()` and `Session::execute` signatures stay 100% unchanged).

---

### R6 — Group `Editor` / `UiState` (145 Fields in `crates/ui-egui/src/lib.rs`, 5,527 Lines) into Cohesive Sub-Structs

- **Timing vs M34:** **After M34**
- **Rough size:** **4 steps** (medium)
- **The problem (concrete evidence):**
  - `UiState` (`crates/ui-egui/src/lib.rs:845-1034`) has **145 fields**, and `Editor` (`:1133-1176`) adds **30 more fields** (175 total).
  - `Editor::new` (`:1321-1515`) is a 195-line manual struct initializer, and `Editor::replace_course` (`:1665-1780`) must manually remember to clear/reset ~50 transient fields and caches. Forgetting to clear ONE field on `replace_course` (like a cache or pending modal) is a proven source of cross-course state leaks!
  - `Editor::show` (`:4228`) is **705 lines**, mixing shortcuts, clipboard events, panel layout, window dispatch, and preview modal orchestration.
- **Proposed shape:**
  - Extract a `CourseSessionCaches` struct (holding `thumbnail_cache`, `navigator_index`, `canvas_compose_cache`, `properties_lists`, `media_refs_cache`, `audio_waveforms`, `font_facts`, `spell_cache`) with a single `caches.clear()` called in `Editor::replace_course` (compile-time guarantee that new caches reset automatically!).
  - Group modal/dialog states into `EditorDialogs` (`Default`-derivable, reset in one `self.ui.dialogs = Default::default()` on `replace_course`!), and extract `media_import.rs` + `shortcuts_dispatch.rs` out of `lib.rs`.
- **Benefit:** Shrinks `lib.rs` from 5,527 → ~1,800 lines, eliminates forgotten-reset bugs when opening/switching courses, and makes `Editor::show` readable at a glance.
- **Risk:** Medium — `UiState` fields are `pub` and referenced directly by ~90 test files in `crates/ui-egui/src/*_tests.rs`. Best done as a dedicated mechanical step using `cargo check --tests` to catch every field path cleanly.

---

### R7 — Split `crates/ui-egui/src/canvas.rs` (7,834 Lines) & `crates/render/src/paint.rs` (4,088 Lines)

- **Timing vs M34:** **After M34**
- **Rough size:** **4 steps** (medium)
- **The problem (concrete evidence):**
  - `crates/ui-egui/src/canvas.rs` (**7,834 lines**) combines 7 distinct state machines: main canvas orchestration (`show_canvas_and_layers`: 555 lines), object drag/resize/rotate (`process_active_drag`: 775 lines, `start_drag_if_pressed`: 519 lines), inline rich-text editing, image crop mode, freeform/connector path drawing, smart-guide snapping, and canvas overlays/zoom HUD.
  - `crates/render/src/paint.rs` (**4,088 lines**) has a single **1,177-line** `paint_items` match function (`:1676`) where shapes, shadows, glows, text, tables, images, video, Lottie, markers, inputs, and question chrome are all intertwined.
- **Proposed shape:**
  - `crates/ui-egui/src/canvas/{mod.rs, drag.rs, hit_test.rs, inline_edit.rs, crop_ui.rs, draw_tool.rs, overlays.rs}`
  - `crates/render/src/paint/{mod.rs, shapes.rs, text.rs, media.rs, tables.rs, inputs.rs, effects.rs}` (dispatching each `Visual` variant to a focused helper ≤ 120 lines).
- **Benefit:** Makes canvas gestures and render caching drastically easier to reason about, review, and test in isolation.
- **Risk:** Low — pure intra-crate module splits with `pub use` re-exports preserving all external signatures.

---

### R8 — Separate Desktop Shell, Web Shell & Control Adapter out of `apps/coursecraft/src/main.rs` (3,928 Lines)

- **Timing vs M34:** **After M34**
- **Rough size:** **3 steps** (small)
- **The problem (concrete evidence):**
  - `apps/coursecraft/src/main.rs` (**3,928 lines**) interleaves `#[cfg(not(target_arch = "wasm32"))]` and `#[cfg(target_arch = "wasm32")]` blocks across `EditorApp`, `show_app` (**632 lines**), `EditorHostAdapter`, file dialogs, modals, and `main()`.
  - Editing desktop features routinely triggers unused-variable or cfg-mismatch warnings on `wasm32-unknown-unknown` (and vice versa).
  - Web file-picker handlers (`main.rs:3198-3326`) repeat the exact same 18-line `wasm_bindgen_futures::spawn_local` + `AsyncFileDialog` boilerplate **7 times**.
- **Proposed shape:**
  ```text
  apps/coursecraft/src/
    main.rs             // CLI/MCP dispatch + native/wasm entry points (~180 lines)
    app.rs              // Shared EditorApp struct + common AI/PPTX/document dispatch
    desktop.rs          // #[cfg(not(target_arch = "wasm32"))] file ops, autosave, publish, export, modals
    web.rs              // #[cfg(target_arch = "wasm32")] web save/open/publish, shared pick_web_file helper
    control_adapter.rs  // #[cfg(not(target_arch = "wasm32"))] EditorHostAdapter
  ```
- **Benefit:** Clean platform boundaries, ~75% less `#[cfg]` noise, fewer accidental WASM breakages.
- **Risk:** Low — binary-only crate, zero downstream consumers.

---

### R9 — Modularize `crates/course/src/{model,validate}.rs`, `crates/pptx/src/lib.rs`, and Monolithic `tests.rs` Files

- **Timing vs M34:** **After M34** (opportunistic / low priority)
- **Rough size:** **4–6 steps** (medium, can be done one crate at a time)
- **The problem (concrete evidence):**
  - `crates/course/src/model.rs` (4,557 lines) + `validate.rs` (3,569 lines) = **8,126 lines** across 2 files.
  - `crates/pptx/src/lib.rs` is **6,019 lines** (`process_shape`: 521 lines, `import_pptx_with_limits`: 471 lines).
  - Monolithic test files — `crates/ui-egui/src/tests.rs` (**20,159 lines**), `crates/engine/src/tests.rs` (**12,189 lines**), `crates/logic/src/tests.rs` (**11,406 lines**), `crates/render/src/tests.rs` (**8,714 lines**) — make locating relevant tests slow and cause unnecessary recompilation of huge test modules.
- **Proposed shape:**
  - Split `crates/pptx/src/lib.rs` into `archive.rs`, `slide.rs`, `shape.rs`, `text.rs`, `table.rs`, `media.rs`.
  - Split `crates/course/src/model.rs` and `validate.rs` into domain submodules (`course`, `slide`, `object`, `style`, `trigger`, `question`) with identical public re-exports from `crates/course/src/lib.rs`.
  - Incrementally extract feature-grouped test modules out of the 10k–20k line `tests.rs` files whenever touching an area.
- **Benefit:** Faster search, cleaner code reviews, faster targeted test iteration, and much lower context window overhead for AI-assisted work.
- **Risk:** Very low — purely internal file organization with zero API changes.

---

## 3. Recommended Sequencing Summary

| Phase | Refactors | Why |
|---|---|---|
| **With M34** | **R1** (Unified media walker + `MediaStore`), **R2** (Zero-copy Open/Publish APIs), **R3** (Drop `coursecraft-format` & `default_fonts` from Player) | Small (1–3 steps each), low risk, and **directly unlock M34's biggest RAM, WASM, and O(1) lookup wins**. |
| **Shortly After M34** (M35 hygiene pass, pending owner approval) | **R4** (`properties.rs` split), **R5** (`commands.rs` split + inline `UndoScope`), **R6** (`EditorCaches` / `UiState` grouping) | Highest day-to-day velocity & bug-prevention payoff across the 3 biggest hotspots (`30,000` lines combined). |
| **Later / Opportunistic** | **R7** (`canvas.rs` + `paint.rs`), **R8** (`apps/coursecraft/src/main.rs`), **R9** (`course`, `pptx`, monolithic `tests.rs`) | Clean, independent, zero urgency; can be picked up one crate at a time between feature milestones. |
