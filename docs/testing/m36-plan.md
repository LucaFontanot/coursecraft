# M36 Refactor A — Architecture, Migration & Test Plan

**Role:** Refactor Architecture, Mechanical Migration & Verification Plan (planning only — zero source files edited)  
**Target milestone:** M36 (Pure Refactor — **Refactor A** from [`docs/testing/refactor-proposal.md`](refactor-proposal.md))  
**Implementer:** Gemini 4 Neon (high), one step per run, with a reviewer after each step.

**Approved Scope (strict):**
1. **R1** — One canonical media walker in `coursecraft-course` (`visit_course_media` / `visit_slide_media` / `visit_object_media`) replacing all 4 duplicated walkers, plus an always-indexed `MediaStore` in `coursecraft-ui-egui` replacing `Editor.media: Vec<(String, Arc<[u8]>)>` and M34's shadow-index workaround.
2. **R2 (remaining)** — Finish the zero-copy, streaming Open and Publish APIs left over after M34 (`SharedPackageZipper` / zero-copy web publish, zero-copy `player_files`, remaining `std::fs::read` + `open_project` disk callers including `control`, `template_library`, and `big_course.rs`, and consolidating `coursecraft-publish` from two parallel APIs to one zero-copy API).
3. **R5** — Split `crates/engine/src/commands.rs` (13,703 lines, 186 commands) into domain submodules under `crates/engine/src/commands/`, fold `UndoScope` (and `can_use_slide_validation`) directly into `Command` / `REGISTRY` so every command is declared in one place, and split the 1,228-line `interaction_insert` handler into per-kind builders.
4. **R8** — Split `apps/coursecraft/src/main.rs` (4,315 lines) into `main.rs`, `app.rs`, `desktop.rs`, `web.rs`, and `control_adapter.rs`, and deduplicate the repeated web `AsyncFileDialog` pickers.

**Explicitly Out of Scope:** **R3** (already shipped in M34 S3), **R4** (`properties.rs`), **R6** (`UiState` / `Editor` sub-structs), **R7** (`canvas.rs` / `paint.rs`), **R9** (`course/{model,validate}.rs`, `pptx/src/lib.rs`, monolithic `tests.rs`), and any new features or runtime/UI behaviour changes.

> **Implementer Guardrails (`AGENTS.md`, `docs/contributing.md`)**
> 1. **Zero behaviour change:** `course.json`, saved `.ccraft` / `.ccraft-folder` projects, published SCORM/xAPI/cmi5/LTI/Web packages, command ids, labels, menu paths, shortcuts, param docs, MCP tools, CLI output, and [`docs/cli.md`](../cli.md) MUST stay **byte-identical**.
> 2. **Mechanical move first, shape change second:** Cut/paste code into new modules with `pub(crate)` / `pub(super)` re-exports in one step; change data shapes or deduplicate logic in a separate follow-up step. Every single step leaves the workspace compiling and green.
> 3. **Golden rules hold on every step:**
>    - Never crash: zero `unwrap`/`expect`/`panic!`/`unreachable!`/`todo!` in non-test code; `unsafe_code = "forbid"`.
>    - Everything is a command: all mutations still route through `coursecraft-engine`.
>    - One runtime: zero play-time behaviour added outside `coursecraft-logic`.
>    - Layering: `cargo xtask layers` stays green; zero new workspace crate dependencies.
>    - WASM: all 17 WASM crates (including `coursecraft-player` and `coursecraft`) keep building cleanly for `wasm32-unknown-unknown` with `-D warnings`.
>    - Performance budgets: release `coursecraft-ui-egui::perf_tests` and `coursecraft-engine::undo_perf_tests` stay within budget.
> 4. **Public APIs used by other crates:** Keep old names as thin wrappers until all callers are migrated, then remove redundant wrappers only in the designated cleanup step.

---


> **Reviewer notes (Claude) — these override the plan below.**
> 1. Approved by the owner as Refactor A (R1, rest of R2, R5, R8). Nothing outside this scope; no behaviour changes, no new features.
> 2. Golden and byte-compare tests land BEFORE any code moves in each group (S1, S6) and must pass unchanged after every later step. Never edit a golden file to make a step pass; if one changes, the step is wrong.
> 3. For pure moves of large blocks the implementer may copy line ranges with shell tools (for example `sed -n 'A,Bp' old.rs > new.rs`) and then delete the moved range, as long as the result compiles and `cargo fmt` is run; no reformatting or rewording during moves.
> 4. Release perf tests and `undo_perf` must stay within budget; S14 records before/after numbers.

## 1. Goals

1. **Eliminate media-walker drift permanently (R1):** Maintain one authoritative media-reference traversal in L1 (`coursecraft-course`) so future media-bearing fields are wired once, and guarantee O(1) indexed media lookups and O(1) raw-media byte tracking by construction via `MediaStore`.
2. **Finish zero-copy, streaming Open/Publish everywhere (R2 remaining):** Eliminate the remaining media `.to_vec()` duplication in web publish, the ~5 MiB `player_files` clone on publish, and the remaining `std::fs::read` + `open_project` disk callers (`control`, `template_library`, `big_course.rs`), leaving `coursecraft-publish` with one clean zero-copy API.
3. **Eliminate the #1 merge-conflict hotspot & command-metadata drift (R5):** Break the 13,703-line `commands.rs` monolith into focused domain modules (~100–2,400 lines each), declare every command's metadata + `UndoScope` + `slide_validation` flag in a single `REGISTRY` row, and break the 1,228-line `interaction_insert` function into 8 per-kind helpers.
4. **Separate desktop and web platform shells cleanly (R8):** Isolate desktop-only, web-only, and `ControlHost` adapter code out of the 4,315-line `apps/coursecraft/src/main.rs` so desktop edits stop triggering `wasm32` `#[cfg]` warnings and vice versa.

---

## 2. Current State (Measured on Current Checkout Post-M34 & M35)

M34 and M35 modified every area targeted by Refactor A after `docs/testing/refactor-proposal.md` was written. All numbers and line references below were re-measured directly against the current checkout.

### 2.1 File Size Snapshot

| File | Proposal Size | Current Size | Notes (Post-M34/M35) |
|---|---:|---:|---|
| `crates/course/src/lib.rs` | ~600 | **616** | Holds walker #1 (`media_references`, lines `126–316`, 191 lines) |
| `crates/publish/src/lib.rs` | ~900 | **1,180** | Holds walker #2 (`collect_object_media` + `add_web_folders` + `web_folder_entry`, lines `112–360, 535–580`) + parallel `Package*` and `SharedPackage*` APIs + `Vec<u8>` `PackageZipper` |
| `crates/format/src/lib.rs` | ~450 | **473** | M34 S8 already added `open_project_from{,_with_limits}` (`376–473`) |
| `crates/ui-egui/src/lib.rs` | 5,527 | **5,919** | Holds walker #3 (`referenced_media_srcs`, `web_folder_prefixes`, `collect_object_media_srcs`, lines `5403–5579` + inline loop in `slide_media:2289–2311`) + `pub media: Vec<...>` + M34's 4 shadow-index fields (`1306–1310`) and 6 sync/sig helpers (`2105–2158, 5294–5319`) |
| `apps/coursecraft-player/src/media.rs` | ~470 | **485** | Holds walker #4 (`slide_image_srcs`, `slide_media_srcs`, `collect_slide_srcs`, `collect_object_srcs`, lines `78–220`, 143 lines) |
| `crates/engine/src/commands.rs` | 13,580 | **13,703** | **186** commands; `undo_scope` (`42–192`, 151 lines) + `can_use_slide_validation` (`203–290`, 88 lines) + `REGISTRY` (`292–1970`, 1,679 lines) + handlers (`2092–13703`, 11,612 lines); `interaction_insert` is **1,228 lines** (`11115–12343`). Existing submodules: `commands/styles.rs` (390), `commands/csv_import.rs` (598) |
| `crates/engine/src/lib.rs` | ~850 | **931** | `Command` struct (`171–190`) delegates `scope()` to `commands::undo_scope(self.id)`; line `404` calls `commands::can_use_slide_validation(cmd_id)` |
| `apps/coursecraft/src/main.rs` | 3,928 | **4,315** | Grew by +387 lines in M34/M35 (autosave `save_epoch`, `UpdateClient`, perf hooks); desktop `impl EditorApp` (`430–2585`), shared `impl EditorApp` (`2587–2846`), `EditorHostAdapter` (`2886–3177`), interleaved `show_app` (`3186–3821`, **636 lines**), web `impl EditorApp` (`4061–4308`) |
| `apps/coursecraft/src/publish.rs` | ~280 | **310** | Still keeps `#[cfg(any(test, target_arch = "wasm32"))] package_files_from` (`104–118`) calling `.to_vec()` solely for `PackageZipper` |
| `apps/coursecraft-cli/tests/big_course.rs` | ~478 | **478** | Still calls legacy `PackageInput` + `.to_vec()` + `build_package_standard_with_folders` + `zip_package_to` (`396–406`) and `std::fs::read` + `open_project_with_limits` (`432–433`) |

### 2.2 Exact Audit of the 4 Duplicated Media Walkers (R1)

All 4 walkers traverse the exact same `ObjectKind` + `StateDef` + `StateExtraKind` media fields, with subtle per-caller rules that **must be preserved identically**:

| # | Location | Scope | Web Folder Representation | Empty `src` Rule | Ordering / Special Rules |
|---|---|---|---|---|---|
| 1 | `crates/course/src/lib.rs:126–316` (`media_references`) | Course-level (`fonts`, `background_audio`, `player.logo`, `resources`) + `scenes`, `masters`, `templates`, `banks` | Emits `"{path}/"` | Skips `src.is_empty()` everywhere | Course-level first, then scenes (1-based slide #), masters, templates, banks; walks `master.layers` |
| 2 | `crates/publish/src/lib.rs:112–360, 535–580` (`collect_object_media`, `collect_resource_media`, `add_web_folders`, `web_folder_entry`) | `scenes`, `masters` (objects only), `banks`, then `background_audio`, `fonts`, `player.logo`, `resources` (**no `templates`**) | Handled separately in `add_web_folders` (`path` + `entry`) | **Does NOT skip** empty `src` on primary `Image`, `Lottie`, `Audio.src/captions`, `Video.src/captions`, `background_audio`, `fonts`, `player.logo`, `resources` (triggers `PublishError::UnsafePath`); **skips** empty `src` on `Video.poster`, `FillStyle::Picture`, `state.src`, `state.fill_style`, `state.extras` | Slides/masters/banks checked before course-level (fixes first-seen `object_id` in error messages); final file list is sorted by path at line `679` before zipping |
| 3 | `crates/ui-egui/src/lib.rs:5403–5579, 2289–2311` (`referenced_media_srcs`, `web_folder_prefixes`, `collect_object_media_srcs`, `Editor::slide_media`) | Course-level + `scenes`, `masters`, `templates`, `banks` (and single `Slide` in `slide_media`) | `referenced_media_srcs` emits `"{path}/{entry}"`; `web_folder_prefixes` emits `"{path}/"` | Skips `src.is_empty()` everywhere | Returns `HashSet<String>` / sorted `Vec<String>` |
| 4 | `apps/coursecraft-player/src/media.rs:78–220` (`slide_image_srcs`, `slide_media_srcs`, `collect_object_srcs`) | Single `Slide` + its optional `master` | Ignored (browser loads iframe directly) | Skips `src.is_empty()` everywhere | `slide_image_srcs`: `Image`, picture fills, `Video.poster`, `state.src`, `state.fill_style` picture, `state.extras` image/picture fills; `slide_media_srcs`: adds `Lottie`, `Audio.captions`, `Video.captions`; excludes `Audio.src` & `Video.src` |

*(Note: `rewrite_slide_media_srcs` / `rewrite_object_media_srcs` at `crates/ui-egui/src/lib.rs:5581–5715` mutates `&mut String` paths in place for template collision renames and stays untouched.)*

### 2.3 Exact Audit of `Editor.media` & M34 Shadow Index (R1)

- `Editor` (`crates/ui-egui/src/lib.rs:1306–1310`) still exposes `pub media: Vec<(String, Arc<[u8]>)>`, which forced M34 S11 to maintain 4 private shadow fields (`media_index`, `media_index_len`, `media_index_sig`, `media_total_bytes`) and 6 runtime staleness-check helpers (`media_names_sig`, `media_index_matches_ends`, `rebuild_media_index`, `sync_media_index`, `media_index_quick_valid`, `media_index_if_synced`).
- Direct mutations of `editor.media` outside `crates/ui-egui/src/lib.rs` occur in only **8 files** (~15 lines total):
  - Production (2 lines): `apps/coursecraft/src/main.rs:1224` (`pick_web_folder`), `crates/ui-egui/src/publish.rs:561` (`pending_save_fonts`), `crates/ui-egui/src/snapshot.rs:310`.
  - Tests (5 files): `crates/ui-egui/src/tests.rs:14929`, `u12_media_fonts_styles_galleries_tests.rs:73,123,197,207`, `m27_layers_questions_tests.rs:1243`, `m27_timeline_media_tests.rs:1342,1346`, `guard_tests.rs:1363,1379`.
- `MemoryMediaSource::from_parts` is constructed in **10 places**, all internal to `crates/ui-egui/src/` (`lib.rs:4586,5233`, `canvas.rs:7501`, `slides.rs:692,1785,2198`, `course_map.rs:354`, `media_library.rs:616`, `states.rs:597,965`).
- Three linear scans still remain in `crates/ui-egui/src/media_library.rs:196,419,444` (`editor.media.iter().any(...)`).

### 2.4 Exact Audit of What Remains for R2 (Post-M34)

**Already shipped in M34 S8 (do NOT redo):**
- `crates/format/src/lib.rs`: `open_project_from<R: Read + Seek>` and `open_project_from_with_limits` are already the primary implementation; `open_project` / `open_project_with_limits` are already 1-line `Cursor::new` wrappers.
- `apps/coursecraft/src/files.rs:500` and `apps/coursecraft-cli/src/lib.rs:439` (also used by `batch.rs:219`) already stream `.ccraft` files through 256 KiB `BufReader<File>` + `open_project_from`.
- `crates/publish/src/lib.rs`: `SharedPackageFile`, `SharedPackageInput`, `build_package_shared`, `zip_shared_package`, and `zip_shared_package_to` already exist and are wired into desktop `apps/coursecraft/src/publish.rs`, `apps/coursecraft-cli/src/lib.rs`, and `apps/coursecraft-cli/src/batch.rs`.

**What is STILL left for R2:**
1. **Web incremental publish copies 100% of media bytes (`crates/publish/src/lib.rs:779–873`, `apps/coursecraft/src/publish.rs:104–118`, `apps/coursecraft/src/main.rs:4072–4083`):** `PackageZipper` still owns `Vec<PackageFile>` (`Vec<u8>`), forcing `apps/coursecraft/src/publish.rs::package_files_from` to call `f.bytes.to_vec()` on every asset before handing files to `PackageZipper::new` on `wasm32`.
2. **Player files are cloned on every publish (`crates/publish/src/lib.rs:72, 509`, `apps/coursecraft/src/player_bundle.rs:18`):** `SharedPackageInput::player_files` is still `&'a [PackageFile]` (`Vec<u8>`), causing `build_package_entries` to clone ~5 MiB of player `.wasm` bytes on every publish.
3. **3 remaining disk `.ccraft` readers still load the entire zip into RAM via `std::fs::read` + `open_project`:**
   - `crates/control/src/host.rs:513–514` (`HeadlessHost::open`)
   - `apps/coursecraft/src/template_library.rs:191–192` (`load_template`)
   - `apps/coursecraft-cli/tests/big_course.rs:432–433` (`open_stage` — M34 Open Concern #3)
4. **`apps/coursecraft-cli/tests/big_course.rs:396–406` still benchmarks legacy copying publish (`PackageInput` + `.to_vec()` + `build_package_standard_with_folders` + `zip_package_to`)** instead of zero-copy `Arc<[u8]>` + buffered streaming zip.
5. **Parallel duplicate APIs in `crates/publish/src/lib.rs`:** `PackageFile` + `SharedPackageFile`, `PackageInput` + `SharedPackageInput`, `build_package_standard_with_folders` + `build_package_shared`, `zip_package{,_to}` + `zip_shared_package{,_to}`. Zero non-test callers outside web `PackageZipper` still use the old `Vec<u8>` APIs; once web `PackageZipper` and `big_course.rs` move off them, `coursecraft-publish` can consolidate onto one zero-copy API.

---

## 3. Target Shape Per Refactor

### 3.1 R1 — Canonical Media Visitor + `MediaStore`

1. **`crates/course/src/media.rs`** (re-exported from `crates/course/src/lib.rs`):
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub enum MediaScope { Publishable, AllIncludingTemplates }

   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub enum MediaRole {
       Image, PictureFill, StateImage, StateExtraImage, VideoPoster,
       Lottie, Audio, Video, Caption, WebFolder,
       Font, BackgroundAudio, PlayerLogo, ResourceFile,
   }

   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub enum SlideContainer<'a> {
       SceneSlide { number: usize }, // 1-based play order
       Master,
       Template,
       Bank { bank: &'a QuestionBank },
       Detached,
   }

   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub enum MediaOwner<'a> {
       Font { family: &'a str },
       BackgroundAudio,
       PlayerLogo,
       Resource { id: &'a str, title: &'a str },
       Object {
           container: SlideContainer<'a>,
           slide: &'a Slide,
           layer: Option<&'a Layer>,
           object: &'a Object,
           state_name: Option<&'a str>,
       },
   }

   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub struct MediaRef<'a> {
       pub src: &'a str,
       pub role: MediaRole,
       pub owner: MediaOwner<'a>,
       /// Present only when `role == MediaRole::WebFolder`.
       pub web_entry: Option<&'a str>,
   }

   pub fn visit_course_media<'a>(course: &'a Course, scope: MediaScope, visit: &mut dyn FnMut(MediaRef<'a>));
   pub fn visit_slide_media<'a>(slide: &'a Slide, visit: &mut dyn FnMut(MediaRef<'a>));
   pub fn visit_object_media<'a>(object: &'a Object, visit: &mut dyn FnMut(MediaRef<'a>));
   ```
   - Helper methods on `MediaRole`: `is_player_image(self) -> bool` (`Image | PictureFill | StateImage | StateExtraImage | VideoPoster`) and `is_player_fetched(self) -> bool` (`is_player_image() || matches!(self, Lottie | Caption)`).
   - Canonical traversal order: `fonts` → `background_audio` → `player.logo` → `resources` → `scenes` (with 1-based `number`) → `masters` → `templates` *(only for `AllIncludingTemplates`)* → `banks`.
   - Emits `MediaRef` for every populated model slot (including empty strings on non-optional fields so `publish` can still emit `PublishError::UnsafePath` identically; callers that ignore empty strings filter `!r.src.is_empty()`).
   - All 4 existing public/private walkers (`course::media_references`, `publish` media + web-folder collection, `ui-egui::{referenced_media_srcs, web_folder_prefixes, Editor::slide_media}`, `player::{slide_image_srcs, slide_media_srcs}`) become thin adapters over `visit_*_media`, keeping their exact existing signatures and outputs.

2. **`crates/ui-egui/src/media_store.rs`** (re-exported as `pub use media_store::MediaStore;` from `crates/ui-egui/src/lib.rs`):
   ```rust
   #[derive(Clone, Debug, Default)]
   pub struct MediaStore {
       entries: Vec<(String, Arc<[u8]>)>,
       index: HashMap<String, usize>, // first-seen index per name
       total_bytes: u64,
       generation: u64,
   }
   ```
   - Implements `Deref<Target = [(String, Arc<[u8]>)]>` so all existing slice reads (`.iter()`, `.len()`, `.is_empty()`, `&editor.media`) stay zero-churn.
   - Public methods: `new()`, `from_vec(Vec<(String, Arc<[u8]>)>)`, `as_slice()`, `get(&str) -> Option<&Arc<[u8]>>`, `contains(&str) -> bool`, `total_bytes() -> u64`, `generation() -> u64`, `push(String, Arc<[u8]>)`, `replace(&str, Arc<[u8]>) -> bool`, `set_all(Vec<(String, Arc<[u8]>)>)`, `clear()`, `truncate(usize)`, `retain(impl FnMut(&(String, Arc<[u8]>)) -> bool)`, plus `pub(crate) fn index(&self) -> &HashMap<String, usize>`.
   - `Editor.media: MediaStore` replaces `pub media: Vec<...>` and deletes `media_index`, `media_index_len`, `media_index_sig`, `media_total_bytes`, and all 6 staleness helpers.
   - `Editor::media(&self) -> &[(String, Arc<[u8]>)]` and `Editor::set_media(&mut self, Vec<(String, Arc<[u8]>)>)` keep their exact existing signatures; `Editor::add_raw_media(&mut self, name: String, bytes: Arc<[u8]>)` provides clean external insertion.
   - `MemoryMediaSource::from_store(store: &'a MediaStore)` replaces `MemoryMediaSource::from_parts(...)`.

### 3.2 R2 (Remaining) — One Zero-Copy Streaming Open/Publish Pipeline

- **`crates/format/src/lib.rs`:** Keep `open_project_from{,_with_limits}` as the primary streaming path and keep `open_project{,_with_limits}` as 1-line `Cursor::new` wrappers (required by web open and in-memory unit tests).
- **Streaming disk callers:** `crates/control/src/host.rs:513`, `apps/coursecraft/src/template_library.rs:191`, and `apps/coursecraft-cli/tests/big_course.rs:432` open `.ccraft` files with `BufReader::with_capacity(256 * 1024, File::open(path)?)` + `open_project_from{,_with_limits}`.
- **`crates/publish/src/lib.rs` (two-step migration):**
  - *Step S4 (additive, zero breakage):* Add `SharedPackageZipper` (`Vec<SharedPackageFile>`), migrate `apps/coursecraft` web publish (`publish.rs` + `main.rs`) and `big_course.rs` to zero-copy `SharedPackage*` + `BufWriter`.
  - *Step S5 (consolidate & remove duplicate wrappers):* Upgrade `PackageFile::bytes` to `Arc<[u8]>`, `PackageInput::media` to `&'a dyn Fn(&str) -> Option<Arc<[u8]>>`, `build_package_standard_with_folders` web folders to `Arc<[u8]>`, and `PackageZipper` to zero-copy `Vec<PackageFile>`. Update the 6 workspace call sites + `publish/src/tests.rs` and remove the redundant `SharedPackageFile`, `SharedPackageInput`, `build_package_shared`, `zip_shared_package{,_to}`, `SharedPackageZipper`, and `apps/coursecraft/src/publish.rs::package_files_from`.

### 3.3 R5 — Domain-Split `crates/engine/src/commands/` + Single-Row `REGISTRY`

```text
crates/engine/src/commands/
  mod.rs           // registry(), single-row REGISTRY (id, label, menu, shortcut, params, mutates, scope, slide_validation, run), shared param/validation helpers (~1,850 lines)
  course.rs        // course.*, edit.*, view.*, resource.*, glossary.*, variable.* (~1,150 lines)
  slide.rs         // scene.*, slide.*, master.*, template.*, bank.*, draw.*, results.*, certificate.*, course.import_outline, copy_slide & remap helpers (~2,400 lines)
  object.rs        // object.* geometry, layout, grouping, duplicate/paste, style, fill/shadow/glow, specialized kinds, crop/captions/media (~2,350 lines)
  object_states.rs // object.* states, state extras, object.delete, animations & motion paths (~1,050 lines)
  text.rs          // text.*, object.set_autofit (~450 lines)
  table.rs         // table.* (~400 lines)
  layer.rs         // layer.* (~280 lines)
  trigger.rs       // trigger.* (~90 lines)
  video_cues.rs    // video.* cues + slide.* cues (~480 lines)
  question.rs      // question.* + feedback/submit helpers (~1,350 lines)
  interaction.rs   // interaction.insert dispatcher (~80 lines) + 8 per-kind builder fns (~1,550 lines total)
  styles.rs        // existing (390 lines: theme.*, text_style.*, course.recolor)
  csv_import.rs    // existing (598 lines: bank.import_csv)
```

- In `crates/engine/src/lib.rs`, `Command` gains two `pub(crate)` fields:
  ```rust
  pub(crate) scope: UndoScope,
  pub(crate) slide_validation: bool,
  ```
  - `Command::scope(&self) -> UndoScope` returns `self.scope`.
  - `Command::can_use_slide_validation(&self) -> bool` returns `self.slide_validation`.
  - Both separate match tables (`commands::undo_scope` and `commands::can_use_slide_validation`) are deleted.
  - External `Command` public fields and `pub fn registry() -> &'static [Command]` remain 100% unchanged.

### 3.4 R8 — Platform-Split `apps/coursecraft/src/`

```text
apps/coursecraft/src/
  main.rs             // Module declarations, pub(crate) re-exports for tests, CLI/MCP dispatch, native & wasm main() (~230 lines)
  app.rs              // EditorApp struct, eframe::App impl, thin show_app() dispatcher, shared AI/PPTX/document methods, initial_course*, into_arc_media (~550 lines)
  desktop.rs          // #[cfg(not(target_arch = "wasm32"))] desktop types (PendingAction, RestorePrompt, ConflictPrompt, AutosaveOutcome, SaveKind), desktop EditorApp methods + show_desktop(), reveal_in_folder, save_kind, control_save_is_folder (~2,450 lines)
  web.rs              // #[cfg(target_arch = "wasm32")] WebPublishJob, web EditorApp methods + show_web(), shared pick_web_file/pick_web_files helpers, web_download_bytes, sanitize_filename_stem, install_web_file_shortcut_guard; #[cfg(any(target_arch = "wasm32", test))] resolve_web_theme & should_prevent_browser_file_shortcut (~430 lines)
  control_adapter.rs  // #[cfg(not(target_arch = "wasm32"))] EditorHostAdapter + impl ControlHost + control_waker (~320 lines)
```

- `main.rs` re-exports all `pub(crate)` / test-referenced items (`EditorApp`, `PendingAction`, `RestorePrompt`, `ConflictPrompt`, `SaveKind`, `save_kind`, `control_save_is_folder`, `should_intercept_close`, `window_title`, `title_changed`, `editor_viewport_builder`, `resolve_web_theme`, `initial_course`, `should_prevent_browser_file_shortcut`), so all 8 existing test files in `apps/coursecraft/src/` compile untouched.

---

## 4. Test Strategy (Proving Zero Behaviour Change)

Every golden/parity test below is added **BEFORE** moving or rewriting the code it guards, so any unintended drift fails immediately.

| ID | Refactor | Crate / File | What It Proves (Byte-Identical / Exact Parity) |
|---|---|---|---|
| **R1-01** | R1 | `crates/course/src/tests.rs` | Golden snapshot of `media_references()` (exact order, `src`, `place`, `slide`, `object`) on a comprehensive course covering fonts, background audio, player logo, file resources, scenes/masters/templates/banks, base+layer objects, all media kinds, picture fills, web folders, states (`src`, `fill_style`), and all `state.extras` variants; plus direct unit tests for `visit_course_media` (`Publishable` vs `AllIncludingTemplates`), `visit_slide_media`, and `visit_object_media`. |
| **R1-02** | R1 | `crates/publish/src/tests.rs` | Golden byte-identical zip comparison (`zip1 == zip2`) + error-selection parity test covering every publishable media slot, web folders, template exclusion, and empty-`src` `UnsafePath` vs empty-optional-field skipping (poster, picture fill, state `src`, state extras). |
| **R1-03** | R1 | `crates/ui-egui/src/tests.rs` | Golden parity test for `referenced_media_srcs`, `web_folder_prefixes`, `Editor::used_media`, `Editor::slide_media`, and `Editor::missing_media` across all object/state/course media kinds and web folders. |
| **R1-04** | R1 | `apps/coursecraft-player/src/tests.rs` | Golden parity test for `slide_image_srcs` and `slide_media_srcs` across images, picture fills, posters, Lottie, audio/video src & captions, states, state extras, layers, and masters (confirming exact inclusion/exclusion and ordering). |
| **R1-05** | R1 | `crates/ui-egui/src/media_store.rs` (`#[cfg(test)]`) | Unit tests for `MediaStore`: deterministic insertion order, first-match duplicate-key semantics, O(1) `get`/`contains`, `total_bytes` saturating accounting, `generation` bumps on every mutation, `replace` (hit & miss), `truncate`, `retain`, `set_all`, `clear`. |
| **R1-06** | R1 | `crates/ui-egui/src/guard_tests.rs` | Replaces M34's `s11_media_index_detects_same_length_replacement`: verifies `Editor.media` (`MediaStore`) keeps index + `total_bytes` + `MemoryMediaSource::from_store` exact across `push`, `replace`, `set_all`, `retain`, and `truncate`, with zero stale lookups. |
| **R2-01** | R2 | `crates/publish/src/tests.rs` | Byte-identical ZIP proof: `SharedPackageZipper` (then unified `PackageZipper`) across chunk sizes `1, 17, 1024, 65536, 262144` and empty input matches `zip_shared_package` / `zip_package` byte-for-byte. |
| **R2-02** | R2 | `crates/control/src/tests.rs`, `apps/coursecraft/src/template_library.rs` | Streaming `open_project_from` parity tests for `HeadlessHost::open` (`.ccraft` with media, missing file, corrupt zip) and `template_library::load_template` (media round-trip, corrupt archive error). |
| **R2-03** | R2 | `crates/publish/src/tests.rs` | Updated `g04` zero-copy proof: `Arc::ptr_eq` holds for course media, web-folder files, **and** `player_files` (zero byte copies anywhere in packaging), while compact `course.json` and zip bytes remain byte-identical. |
| **R5-01** | R5 | `crates/engine/src/tests.rs` | Golden registry lock test: asserts `registry().len() == 186` and checks the exact ordered table of `(id, label, menu, shortcut, params, mutates, scope, can_use_slide_validation)` against a golden checksum/list so zero metadata, order, or scope drift can occur across S6–S10. |
| **R5-02** | R5 | `crates/engine/src/tests.rs` | Golden JSON snapshot test for all 8 `interaction.insert` kinds (`tabs`, `accordion`, `flip_cards`, `hotspot_reveal`, `process_steps`, `timeline`, `reveal_grid`, `scenario_choice`): captures exact post-command `Course::to_json()` + return JSON before S9/S11 and proves byte-identical output after splitting `interaction_insert`. |
| **R5-03** | R5 | `crates/engine/src/m27_all_commands_undo_tests.rs` | Existing full-command proof (`d5_1`, `df_01`, `sc_01`, `u05`) + `panic_hunt` + invariant assertion that every `!cmd.mutates` command has `UndoScope::FullCourse && !cmd.can_use_slide_validation()`. |
| **R5-04** | R5 | `apps/coursecraft-cli/src/tests.rs`, `apps/coursecraft-mcp/src/tests.rs` | Existing CLI `docs/cli.md` sync test (`docs_cli_md_lists_all_commands`), CLI `commands` output tests, and MCP tool-list tests proving zero drift in CLI/MCP surfaces or `docs/cli.md`. |
| **R8-01** | R8 | `apps/coursecraft/src/u16_shell_tests.rs`, `tests.rs`, `m27_files_ui_tests.rs`, `updates_ui_tests.rs`, `files_tests.rs` | All existing 195 desktop shell, file modal, control-adapter, autosave, web-state, and update-UI tests pass untouched, plus `R8-01` checking desktop `show_app` + `EditorHostAdapter` + `resolve_web_theme` + `should_prevent_browser_file_shortcut` + clean `wasm32-unknown-unknown` build with `-D warnings`. |

**Existing Perf & Guard Suites (Regression Protection):**
- `crates/ui-egui/src/guard_tests.rs` (`G-02`–`G-07`, `GD-01`)
- `crates/engine/src/undo_perf_tests.rs` (`PF-01`–`PF-03`, release `--ignored`)
- `crates/ui-egui/src/perf_tests.rs` (all 9 release `--ignored` benchmarks including `PF-05`)
- `apps/coursecraft-cli/tests/big_course.rs` (release `--ignored` Open & Publish verification)

---

## 5. Numbered Implementer Steps (14 Steps, Bottom-Up Ordering)

> **Sequencing rationale:** Execute **R1 → R2 → R5 → R8** (bottom-up by layer: L1/L4/L6 → L3/L4/apps → L5 → app binary). R1 cleans up `publish` media walking before R2 touches `publish`; R1 & R2 land their small `apps/coursecraft/src/main.rs` edits before R8 splits `main.rs`; R5 is completely contained inside `crates/engine` and split into small mechanical chunks before changing `Command` shape.
>
> **CI Gate cadence:** Run targeted `cargo test -p <affected>` + `cargo check --target wasm32-unknown-unknown -p <affected>` on every step. Run full `cargo xtask ci` at **S2, S5, S9, S11, and S14**.

### Phase 1: R1 — Shared Media Walker & Indexed `MediaStore` (S1–S3)

- **S1 — Add R1 Golden Parity Tests & Canonical Media Visitor (`crates/course/src/media.rs`)**
  - **Files:** `crates/course/src/media.rs` (new), `crates/course/src/lib.rs`, `crates/course/src/tests.rs`, `crates/publish/src/tests.rs`, `crates/ui-egui/src/tests.rs`, `apps/coursecraft-player/src/tests.rs`.
  - **Exact moves/changes:**
    1. Add golden parity tests **before** modifying any walker: `R1-01` (`crates/course/src/tests.rs`), `R1-02` (`crates/publish/src/tests.rs`), `R1-03` (`crates/ui-egui/src/tests.rs`), and `R1-04` (`apps/coursecraft-player/src/tests.rs`). Verify all 4 pass against the existing walkers.
    2. Create `crates/course/src/media.rs` implementing `MediaScope`, `MediaRole` (with `is_player_image` and `is_player_fetched`), `SlideContainer`, `MediaOwner`, `MediaRef`, `visit_course_media`, `visit_slide_media`, and `visit_object_media` per §3.1.
    3. Export `mod media; pub use media::{...};` in `crates/course/src/lib.rs`.
    4. Replace `walk_slide_media_references` and `walk_object_media_references` in `crates/course/src/lib.rs` by rewriting `media_references(course: &Course) -> Vec<MediaReference>` as a thin adapter over `visit_course_media(course, MediaScope::AllIncludingTemplates, ...)` (skipping `r.src.is_empty()` and formatting `WebFolder` as `format!("{}/", r.src)`).
  - **Tests:** `R1-01`–`R1-04`, `cargo test -p coursecraft-course -p coursecraft-publish -p coursecraft-player -j 4`, `cargo test -p coursecraft-ui-egui -j 4 referenced_media`.
  - **Done when:** All 4 golden parity tests exist and pass, `crates/course::media_references` runs on `visit_course_media` with identical output, and zero warnings occur.

- **S2 — Migrate Remaining 3 Media Walkers (`publish`, `ui-egui`, `player`) to `coursecraft_course::visit_*_media`**
  - **Files:** `crates/publish/src/lib.rs`, `crates/ui-egui/src/lib.rs`, `apps/coursecraft-player/src/media.rs`.
  - **Exact moves/changes:**
    1. **`crates/publish/src/lib.rs`:** Replace `collect_object_media`, `collect_resource_media`, `collect_web_folders`, and the hand-rolled loop in `web_folder_entry` with thin adapters over `coursecraft_course::visit_course_media(course, MediaScope::Publishable, ...)`:
       - Skip `MediaOwner::Object { container: SlideContainer::Master, layer: Some(_), .. }` (preserving master-objects-only parity even on unvalidated courses).
       - Skip empty `src` ONLY for `MediaRole::VideoPoster | MediaRole::PictureFill | MediaRole::StateImage | MediaRole::StateExtraImage`; keep empty `src` on all other roles so `UnsafePath` triggers identically.
       - Collect slide/master/bank refs first and course-level refs (`BackgroundAudio`, `Font`, `PlayerLogo`, `ResourceFile`) in their exact existing order (`BACKGROUND_AUDIO_ID`, `"font"`, `"player-logo"`, `&resource.id`) so error messages and zip bytes stay identical.
       - Collect web folders (`MediaRole::WebFolder`) and `web_folder_entry` from the same `visit_course_media` call.
    2. **`crates/ui-egui/src/lib.rs`:** Rewrite `referenced_media_srcs`, `web_folder_prefixes`, and `Editor::slide_media` as thin adapters over `visit_course_media(..., MediaScope::AllIncludingTemplates, ...)` and `visit_slide_media(slide, ...)`. Delete `collect_object_media_srcs` (~100 lines).
    3. **`apps/coursecraft-player/src/media.rs`:** Rewrite `slide_image_srcs` and `slide_media_srcs` using `visit_slide_media` (for `slide` and optional `master`) filtering `!r.src.is_empty()` and `r.role.is_player_image()` / `r.role.is_player_fetched()`. Delete `collect_slide_srcs` and `collect_object_srcs` (~110 lines).
  - **Tests:** `R1-01`–`R1-04` + **CI Gate #1: `cargo xtask ci`**.
  - **Done when:** Zero hand-rolled read-only media walkers remain outside `crates/course/src/media.rs`, all golden parity tests pass, and `cargo xtask ci` (including `layers` and `wasm`) is green.

- **S3 — Introduce Indexed `MediaStore` (`crates/ui-egui/src/media_store.rs`) & Replace `Editor.media` + M34 Shadow Index**
  - **Files:** `crates/ui-egui/src/media_store.rs` (new), `crates/ui-egui/src/lib.rs`, `crates/ui-egui/src/canvas.rs`, `crates/ui-egui/src/slides.rs`, `crates/ui-egui/src/course_map.rs`, `crates/ui-egui/src/media_library.rs`, `crates/ui-egui/src/states.rs`, `crates/ui-egui/src/publish.rs`, `crates/ui-egui/src/snapshot.rs`, `crates/ui-egui/src/guard_tests.rs`, `crates/ui-egui/src/tests.rs`, `crates/ui-egui/src/u12_media_fonts_styles_galleries_tests.rs`, `crates/ui-egui/src/m27_layers_questions_tests.rs`, `crates/ui-egui/src/m27_timeline_media_tests.rs`, `apps/coursecraft/src/main.rs`.
  - **Exact moves/changes:**
    1. Create `crates/ui-egui/src/media_store.rs` implementing `MediaStore` per §3.1 (`Deref<Target = [(String, Arc<[u8]>)]>`, O(1) `get`/`contains`/`total_bytes`/`generation`, insertion-order-preserving `push`/`replace`/`set_all`/`clear`/`truncate`/`retain`) and unit tests `R1-05`.
    2. Re-export `pub use media_store::MediaStore;` from `crates/ui-egui/src/lib.rs`.
    3. Change `Editor.media` to `pub media: MediaStore`, delete `media_index`, `media_index_len`, `media_index_sig`, `media_total_bytes`, and delete `media_names_sig`, `media_index_matches_ends`, `rebuild_media_index`, `sync_media_index`, `media_index_quick_valid`, `media_index_if_synced`. Keep `pub fn media(&self) -> &[(String, Arc<[u8]>)]` and `pub fn set_media(&mut self, Vec<(String, Arc<[u8]>)>)` unchanged, and add `pub fn add_raw_media(&mut self, name: String, bytes: Arc<[u8]>)`.
    4. Add `MemoryMediaSource::from_store(store: &'a MediaStore)` (retiring `from_parts`) and update all 10 call sites in `lib.rs`, `canvas.rs`, `slides.rs`, `course_map.rs`, `media_library.rs`, and `states.rs`, plus `TimelinePlayback::sync_slide_indexed` call in `lib.rs:1871` and `missing_images_as_placeholders` call in `canvas.rs:7224`.
    5. Replace linear `editor.media.iter().any(|(name, _)| name == ...)` in `media_library.rs:196,419,444` with O(1) `editor.media.contains(...)`.
    6. Update the 8 direct-mutation call sites (`apps/coursecraft/src/main.rs:1224` → `add_raw_media`, `publish.rs:561` → `media.push`, `snapshot.rs:310` → `media.set_all`, and the 5 test files) and update `guard_tests.rs:1345` to `R1-06`.
  - **Tests:** `R1-05`, `R1-06`, `cargo test -p coursecraft-ui-egui -p coursecraft -j 4`, `cargo check --target wasm32-unknown-unknown -p coursecraft-ui-egui -p coursecraft`.
  - **Done when:** `Editor.media` is `MediaStore`, zero shadow-index staleness helpers remain, `guard_tests` + `media_store` tests pass, and `wasm32` checks cleanly.

---

### Phase 2: R2 (Remaining) — Zero-Copy Streaming Open & Publish APIs (S4–S5)

- **S4 — Zero-Copy Web `SharedPackageZipper` & Remaining Streaming `.ccraft` Disk Openers**
  - **Files:** `crates/publish/src/lib.rs`, `crates/publish/src/tests.rs`, `apps/coursecraft/src/publish.rs`, `apps/coursecraft/src/main.rs`, `crates/control/src/host.rs`, `crates/control/src/tests.rs`, `apps/coursecraft/src/template_library.rs`, `apps/coursecraft-cli/tests/big_course.rs`.
  - **Exact moves/changes:**
    1. Add golden tests `R2-01` (in `crates/publish/src/tests.rs`, proving byte-identical zip output between `SharedPackageZipper`, `zip_shared_package`, and `zip_package`) and `R2-02` (in `crates/control/src/tests.rs` and `apps/coursecraft/src/template_library.rs`).
    2. In `crates/publish/src/lib.rs`, add `SharedPackageZipper` operating over `Vec<SharedPackageFile>` (sharing the chunking logic with `PackageZipper` via a private generic helper `ZipperCore<B: AsRef<[u8]>>`, keeping existing `PackageZipper` intact as a thin wrapper).
    3. In `apps/coursecraft/src/main.rs` (`WebPublishJob::Zipping` + `step_web_publish:4072–4083`), switch web publish from `publish::package_files_from` + `PackageZipper` to `publish::package_files_shared_from` + `SharedPackageZipper` (zero `.to_vec()` copies in WASM!).
    4. Migrate remaining disk `.ccraft` openers from `std::fs::read` + `open_project` to `BufReader::with_capacity(256 * 1024, File::open(path)?)` + `open_project_from{,_with_limits}`:
       - `crates/control/src/host.rs:513–514`
       - `apps/coursecraft/src/template_library.rs:191–192`
       - `apps/coursecraft-cli/tests/big_course.rs:432–433`
    5. Migrate `apps/coursecraft-cli/tests/big_course.rs:392–406` from `PackageInput` + `.to_vec()` + `build_package_standard_with_folders` + `zip_package_to` to `SharedPackageInput` + `Arc<[u8]>` + `build_package_shared` + `zip_shared_package_to` with 256 KiB `BufWriter` (closing M34 Open Concern #3).
  - **Tests:** `R2-01`, `R2-02`, `cargo test -p coursecraft-publish -p coursecraft-control -p coursecraft -p coursecraft-cli -j 4`, `cargo check --target wasm32-unknown-unknown -p coursecraft-publish -p coursecraft`.
  - **Done when:** Zero non-test callers in the workspace still call `std::fs::read` + `open_project` on disk `.ccraft` files or copy media bytes with `.to_vec()` during publish (desktop, CLI, batch, or web).

- **S5 — Consolidate `coursecraft-publish` to One Zero-Copy API & Remove Legacy Wrappers**
  - **Files:** `crates/publish/src/lib.rs`, `crates/publish/src/tests.rs`, `apps/coursecraft/src/player_bundle.rs`, `apps/coursecraft/src/publish.rs`, `apps/coursecraft/src/main.rs`, `apps/coursecraft-cli/src/lib.rs`, `apps/coursecraft-cli/src/batch.rs`, `apps/coursecraft-cli/tests/big_course.rs`.
  - **Exact moves/changes:**
    1. In `crates/publish/src/lib.rs`, consolidate onto one zero-copy API:
       - Change `PackageFile::bytes` to `Arc<[u8]>` (so `player_files` are zero-copy too — `build_package_entries` uses `Arc::clone(&player_file.bytes)` instead of `.clone()` on a ~5 MiB `Vec<u8>`).
       - Change `PackageInput::media` to `&'a dyn Fn(&str) -> Option<Arc<[u8]>>` and `build_package_standard_with_folders`'s web-folder callback to `&dyn Fn(&str) -> Option<Vec<(String, Arc<[u8]>)>>`.
       - Make `PackageZipper` operate directly on `Vec<PackageFile>` (now zero-copy `Arc<[u8]>`).
       - Remove the duplicate `SharedPackageFile`, `SharedPackageInput`, `build_package_shared`, `zip_shared_package`, `zip_shared_package_to`, and `SharedPackageZipper`.
    2. Update the 6 workspace call sites (`player_bundle.rs` → `Arc::from(entry.1)`, `apps/coursecraft/src/publish.rs` removing `package_files_from`, `apps/coursecraft/src/main.rs`, `apps/coursecraft-cli/src/lib.rs`, `apps/coursecraft-cli/src/batch.rs`, `apps/coursecraft-cli/tests/big_course.rs`) and `crates/publish/src/tests.rs` (including strengthening `g04` → `R2-03` to assert `Arc::ptr_eq` on `player_files` as well as media and web folders).
    3. Keep `coursecraft_format::open_project{,_with_limits}` as 1-line `Cursor::new` wrappers (used by web open and in-memory tests).
  - **Tests:** `R2-01`–`R2-03` + **CI Gate #2: `cargo xtask ci`**.
  - **Done when:** `coursecraft-publish` exposes one clean zero-copy API with zero `.to_vec()` media copies and zero `player_files` byte clones, all zip bytes remain byte-identical, and `cargo xtask ci` is green.

---

### Phase 3: R5 — Split `crates/engine/src/commands.rs` & Fold `UndoScope` into `REGISTRY` (S6–S11)

- **S6 — Add R5 Golden Tests (`R5-01`, `R5-02`), Convert `commands.rs` → `commands/mod.rs`, & Mechanically Extract `course.rs`, `layer.rs`, `trigger.rs`, `text.rs`, `table.rs`**
  - **Files:** `crates/engine/src/tests.rs`, `crates/engine/src/commands.rs` (moved to `crates/engine/src/commands/mod.rs`), `crates/engine/src/commands/{course,layer,trigger,text,table}.rs` (new).
  - **Exact moves/changes:**
    1. Add `R5-01` (golden snapshot of all 186 `REGISTRY` entries: exact order, `id`, `label`, `menu`, `shortcut`, `params`, `mutates`, `scope()`, `can_use_slide_validation()`) and `R5-02` (golden JSON snapshot of all 8 `interaction.insert` kinds) in `crates/engine/src/tests.rs` and verify both pass before moving code.
    2. Mechanically move `crates/engine/src/commands.rs` to `crates/engine/src/commands/mod.rs` (keeping `mod csv_import;` and `mod styles;` intact; mark shared helpers `pub(super)`).
    3. Mechanically cut/paste into 5 new submodules with `pub(super)` handlers and zero logic changes:
       - `commands/course.rs` (`course.*` except `import_outline`, `edit.*`, `view.*`, `resource.*`, `glossary.*`, `variable.*`)
       - `commands/layer.rs` (`layer.*`)
       - `commands/trigger.rs` (`trigger.*`)
       - `commands/text.rs` (`text.*`, `object.set_autofit`, `shape_text_default_style`, `text_parts*`)
       - `commands/table.rs` (`table.*`)
    4. Wire `mod course; mod layer; mod table; mod text; mod trigger;` in `commands/mod.rs` and point `REGISTRY` entries at `course::...`, `layer::...`, etc.
  - **Tests:** `R5-01`, `R5-02`, `cargo test -p coursecraft-engine -j 4`.
  - **Done when:** All 5 submodules compile with zero warnings, `R5-01` and `R5-02` pass, and all engine tests pass.

- **S7 — Mechanically Extract `slide.rs` and `video_cues.rs` from `commands/mod.rs`**
  - **Files:** `crates/engine/src/commands/mod.rs`, `crates/engine/src/commands/slide.rs` (new), `crates/engine/src/commands/video_cues.rs` (new), `crates/engine/src/commands/csv_import.rs`.
  - **Exact moves/changes:**
    1. Mechanically cut/paste `scene.*`, `slide.*` (non-cue), `master.*`, `template.*`, `bank.*`, `draw.*`, `results.*`, `certificate.*`, `course.import_outline`, and the `copy_slide` / `remap_*` / `insert_slide_after` helpers into `crates/engine/src/commands/slide.rs`.
    2. Re-export `pub(super) use slide::{bank_create, new_bank_slide_with_id, next_fresh_id};` in `commands/mod.rs` (or import via `super::slide::` in `csv_import.rs`).
    3. Mechanically cut/paste `video.add_cue`, `video.set_cue`, `video.remove_cue`, `video.add_question_cue`, `slide.add_cue`, `slide.set_cue`, `slide.remove_cue`, and cue helpers into `crates/engine/src/commands/video_cues.rs`.
    4. Update `REGISTRY` handler paths in `commands/mod.rs`. Zero logic changes.
  - **Tests:** `R5-01`, `cargo test -p coursecraft-engine -j 4`.
  - **Done when:** `slide.rs` and `video_cues.rs` are extracted mechanically and all engine tests pass.

- **S8 — Mechanically Extract `object.rs` and `object_states.rs` from `commands/mod.rs`**
  - **Files:** `crates/engine/src/commands/mod.rs`, `crates/engine/src/commands/object.rs` (new), `crates/engine/src/commands/object_states.rs` (new), `crates/engine/src/commands/styles.rs`.
  - **Exact moves/changes:**
    1. Mechanically cut/paste core `object.*` commands (`add`, `update`, `reorder`, `align`, `distribute`, `match_size`, `group`, `ungroup`, `duplicate`, `paste`, `set_frame`, `set_rotation`, `flip`, `lock`, `hide_in_editor`, `apply_style`, `set_opacity`, `set_outline_join`, `set_fill_style`, `set_shadow`, `set_glow`, `set_drag`, `set_lottie`, `set_web_source`, `set_input`, `set_marker`, `set_panel`, `set_scroll_panel`, `set_timer`, `set_meter`, `set_path`, `set_crop`, `set_captions`, `set_media`) and shared helpers (`find_object_mut`, `objects_param`, `objects_vec_mut`, `clamp_frame`, etc.) into `crates/engine/src/commands/object.rs`. Keep `pub(super) use object::{find_object_mut, objects_param};` in `commands/mod.rs` so `styles.rs` needs minimal/no churn.
    2. Mechanically cut/paste state, delete, and animation/motion-path commands (`object.add_state`, `update_state`, `patch_state`, `delete_state`, `duplicate_state`, `rename_state`, `reorder_state`, `state_*_extra`, `object.delete`, `object.set_animation`, `object.*_motion_path`, plus `remove_dangling_triggers` and state helpers) into `crates/engine/src/commands/object_states.rs`.
    3. Update `REGISTRY` handler paths in `commands/mod.rs`. Zero logic changes.
  - **Tests:** `R5-01`, `cargo test -p coursecraft-engine -j 4`.
  - **Done when:** `object.rs` and `object_states.rs` are extracted mechanically and all engine tests pass.

- **S9 — Mechanically Extract `question.rs` and `interaction.rs` from `commands/mod.rs`**
  - **Files:** `crates/engine/src/commands/mod.rs`, `crates/engine/src/commands/question.rs` (new), `crates/engine/src/commands/interaction.rs` (new).
  - **Exact moves/changes:**
    1. Mechanically cut/paste all `question.*` commands and question support helpers (`default_multiple_choice_question`, `ensure_question_support`, `add_submit_button`, `add_feedback_layers`, `add_try_again_layer`, `apply_question_kind`, `question_delete`, `is_untouched_feedback_layer`, etc.) into `crates/engine/src/commands/question.rs`.
    2. Mechanically cut/paste `interaction_insert` and its 9 `interaction_*` / `tint` helpers verbatim into `crates/engine/src/commands/interaction.rs` (do NOT refactor `interaction_insert` internals in this step — mechanical move only).
    3. Update `REGISTRY` handler paths in `commands/mod.rs` (leaving `commands/mod.rs` at ~1,950 lines: `REGISTRY`, `registry()`, `undo_scope`, `can_use_slide_validation`, and ~120 lines of shared helpers).
  - **Tests:** `R5-01`, `R5-02`, `R5-03` + **CI Gate #3: `cargo xtask ci`**.
  - **Done when:** Every command handler lives in its domain module, `commands/mod.rs` holds only the registry and shared helpers, and `cargo xtask ci` is green.

- **S10 — Fold `UndoScope` & `slide_validation` into `Command` / `REGISTRY` (One Declaration Per Command)**
  - **Files:** `crates/engine/src/lib.rs`, `crates/engine/src/commands/mod.rs`, `crates/engine/src/m27_all_commands_undo_tests.rs`, `docs/contributing.md` *(update §"Add a command" path note only; keep `docs/cli.md` untouched and byte-identical)*.
  - **Exact moves/changes:**
    1. Add `pub(crate) scope: UndoScope` and `pub(crate) slide_validation: bool` to `pub struct Command` in `crates/engine/src/lib.rs`.
    2. Update `Command::scope(&self) -> UndoScope` to return `self.scope`, and add `pub(crate) fn can_use_slide_validation(&self) -> bool { self.slide_validation }`.
    3. Populate `scope` and `slide_validation` directly on every one of the 186 `Command` rows in `REGISTRY` inside `crates/engine/src/commands/mod.rs`.
    4. Delete `pub(crate) fn undo_scope(id: &str)` (151 lines) and `pub(crate) fn can_use_slide_validation(id: &str)` (88 lines) from `crates/engine/src/commands/mod.rs`.
    5. Update `crates/engine/src/lib.rs:404` (`cmd.can_use_slide_validation().then_some(loc)`) and `crates/engine/src/m27_all_commands_undo_tests.rs:1934` (`cmd.can_use_slide_validation()`), and assert the invariant that every `!cmd.mutates` command has `scope == UndoScope::FullCourse && !cmd.can_use_slide_validation()`.
    6. Update `docs/contributing.md` §"Add a command" to reference `crates/engine/src/commands/` and the single `REGISTRY` row (do NOT touch `docs/cli.md`).
  - **Tests:** `R5-01`, `R5-03`, `R5-04`, `cargo test -p coursecraft-engine -p coursecraft-cli -p coursecraft-mcp -j 4`.
  - **Done when:** Every command's metadata, `UndoScope`, and `slide_validation` flag live in one `REGISTRY` row, `R5-01` matches 100% identically, and `docs/cli.md` is unchanged and passing.

- **S11 — Split `interaction_insert` (1,228 Lines) into 8 Per-Kind Builders in `commands/interaction.rs`**
  - **Files:** `crates/engine/src/commands/interaction.rs`.
  - **Exact moves/changes:**
    1. Extract each of the 8 `match kind` branches inside `interaction_insert` into a dedicated private helper function: `insert_tabs`, `insert_accordion`, `insert_flip_cards`, `insert_hotspot_reveal`, `insert_process_steps`, `insert_timeline`, `insert_reveal_grid`, and `insert_scenario_choice` (passing shared context struct `InteractionCtx<'a>` for `size`, `theme`, `slide_id`, `used_ids`, etc.).
    2. Keep `interaction_insert` as a concise ~80-line param parser + dispatcher + `keep_valid` commit.
  - **Tests:** `R5-02` (byte-identical JSON across all 8 kinds), `R5-03` + **CI Gate #4: `cargo xtask ci`**.
  - **Done when:** No function in `commands/interaction.rs` exceeds ~260 lines, `R5-02` matches byte-for-byte on all 8 interaction kinds, and `cargo xtask ci` is green.

---

### Phase 4: R8 — Split `apps/coursecraft/src/main.rs` into Platform Shells (S12–S14)

- **S12 — Mechanically Extract `control_adapter.rs` and `web.rs` from `apps/coursecraft/src/main.rs`**
  - **Files:** `apps/coursecraft/src/main.rs`, `apps/coursecraft/src/control_adapter.rs` (new), `apps/coursecraft/src/web.rs` (new).
  - **Exact moves/changes:**
    1. Move `EditorHostAdapter`, `impl ControlHost for EditorHostAdapter`, and `control_waker` verbatim into `apps/coursecraft/src/control_adapter.rs` (gated `#[cfg(not(target_arch = "wasm32"))]`).
    2. Move `WebPublishJob`, the `#[cfg(target_arch = "wasm32")] impl EditorApp` block (`step_web_publish`, `guard_web_unsaved`, `execute_web_pending_action`, `perform_web_save`, `open_web_project_bytes`), `sanitize_filename_stem`, `web_download_bytes`, `install_web_file_shortcut_guard`, `web_main()`, and the `#[cfg(any(target_arch = "wasm32", test))]` helpers (`resolve_web_theme`, `should_prevent_browser_file_shortcut`) into `apps/coursecraft/src/web.rs`.
    3. Extract the `#[cfg(target_arch = "wasm32")]` body of `EditorApp::show_app` verbatim into `EditorApp::show_web(&mut self, ui: &mut egui::Ui)` inside `web.rs` (do NOT deduplicate the file pickers yet — mechanical move first).
    4. Declare `#[cfg(not(target_arch = "wasm32"))] mod control_adapter;` and `#[cfg(any(target_arch = "wasm32", test))] mod web;` in `main.rs`, with `pub(crate) use` re-exports so all 8 test files compile untouched.
  - **Tests:** `cargo test -p coursecraft -j 4`, `cargo clippy --target wasm32-unknown-unknown -p coursecraft -- -D warnings`.
  - **Done when:** `control_adapter.rs` and `web.rs` build cleanly with zero warnings on both native and `wasm32`, and all 195 `coursecraft` tests pass.

- **S13 — Mechanically Extract `desktop.rs` and `app.rs` from `apps/coursecraft/src/main.rs`**
  - **Files:** `apps/coursecraft/src/main.rs`, `apps/coursecraft/src/app.rs` (new), `apps/coursecraft/src/desktop.rs` (new).
  - **Exact moves/changes:**
    1. Create `apps/coursecraft/src/app.rs` containing `EditorApp` (with `pub(crate)` fields for `desktop.rs`/`web.rs`), `impl eframe::App for EditorApp`, a thin `EditorApp::show_app` calling `self.poll_ai(ui.ctx())` then delegating to `self.show_desktop(ui)` (`not(wasm32)`) or `self.show_web(ui)` (`wasm32`), the shared `impl EditorApp` (AI, PPTX, document import), `initial_course`, `initial_course_and_media`, `document_text_from_bytes`, and `into_arc_media`.
    2. Create `apps/coursecraft/src/desktop.rs` (`#[cfg(not(target_arch = "wasm32"))]`) containing `PendingAction`, `RestorePrompt`, `ConflictPrompt`, `AutosaveOutcome`, `SaveKind`, `CONFLICT_FILES_SHOWN`, desktop helpers (`should_intercept_close`, `window_title`, `title_changed`, `changed_files_message`, `restore_prompt_for_path`, `untitled_restore_prompt`, `autosave_time_str`, `now_unix_secs`, `resolved_to_update_info`, `has_extension_ignore_ascii_case`, `save_kind`, `control_save_is_folder`, `reveal_in_folder`), the desktop `impl EditorApp` block, and `EditorApp::show_desktop(&mut self, ui: &mut egui::Ui)`.
    3. Keep `apps/coursecraft/src/main.rs` (~230 lines) as the module root, test-import `pub(crate) use` re-exports, and native/wasm entry points (`cli_with_player`, `editor_viewport_builder`, `run_editor_window`, native `main()`, wasm `main()`).
  - **Tests:** `cargo test -p coursecraft -j 4`, `cargo clippy --target wasm32-unknown-unknown -p coursecraft -- -D warnings`.
  - **Done when:** `main.rs` is ~230 lines, desktop and web shells are cleanly separated with minimal `#[cfg]` noise, and all 195 `coursecraft` tests + `wasm32` clippy pass.

- **S14 — Deduplicate Web File Pickers in `web.rs`, Add `R8-01`, & Final M36 Release Gate**
  - **Files:** `apps/coursecraft/src/web.rs`, `apps/coursecraft/src/u16_shell_tests.rs`.
  - **Exact moves/changes:**
    1. In `apps/coursecraft/src/web.rs`, add two small `#[cfg(target_arch = "wasm32")]` helpers:
       - `fn pick_web_file(label: &'static str, exts: &'static [&'static str], tx: mpsc::Sender<(String, Vec<u8>)>, ctx: egui::Context)`
       - `fn pick_web_files(label: &'static str, exts: &'static [&'static str], tx: mpsc::Sender<(String, Vec<u8>)>, ctx: egui::Context)`
       Replace all 8 repeated single-file `wasm_bindgen_futures::spawn_local` + `rfd::AsyncFileDialog` blocks (`PickImage`, `PickAudio`, `PickBackgroundAudio`, `PickVideo`, `PickLottie`, `PickDocument`, `Open`, `ImportPptx`) and the multi-file `PickFonts` block with 1-line calls, preserving exact filter labels and extension lists.
    2. Add `R8-01` in `apps/coursecraft/src/u16_shell_tests.rs` verifying `EditorApp::show_app` dispatch, `EditorHostAdapter` round-trip, `resolve_web_theme`, and `should_prevent_browser_file_shortcut` through the split modules.
    3. Run the complete §7 Final Release Gate (CI, `build-player`, release `undo_perf_tests`, release `perf_tests`, and release `big_course`).
  - **Tests:** `R8-01` + **Final Release Gate (§7)**.
  - **Done when:** All §7 automated release gates pass with zero regressions and every budget is met.

---

## 6. Watchlist of Risky Spots (`RISK-01`–`RISK-10`)

| ID | Area / Location | Risk & Concrete Mitigation |
|---|---|---|
| **RISK-01** | `crates/publish/src/lib.rs` (empty `src` vs `PublishError::UnsafePath`) | `course::media_references` skips empty `src`, while `publish` intentionally feeds empty primary `src` (`Image`, `Lottie`, `Audio.src/captions`, `Video.src/captions`, `Font`, `BackgroundAudio`, `PlayerLogo`, `ResourceFile`) into `is_safe_relative_path("")` so it returns `PublishError::UnsafePath`, yet skips empty `Video.poster`, picture `fill_style`, `state.src`, `state.fill_style`, and `state.extras`. **Mitigation:** Emit empty primary `src` from `visit_*_media` and filter by `MediaRole` in `publish`; prove with `R1-02`. |
| **RISK-02** | `crates/publish/src/lib.rs` (error-reporting order & master layers) | `publish` currently visits slides/masters/banks before course-level media, and skips `master.layers`. Changing traversal order could change which `object_id` is reported first on a course with two missing files. **Mitigation:** Partition slide/master/bank refs before course-level refs in the `publish` adapter and ignore `Master` layers; prove with `R1-02`. |
| **RISK-03** | `crates/ui-egui/src/canvas.rs:7160` (`canvas_media_fingerprint`) & `Editor::replace_course` | `CanvasComposeCache` keys on `canvas_media_fingerprint(&editor.media)` and `Editor::set_media` resets `media_cache`. With `MediaStore`, `Deref<Target = [(String, Arc<[u8]>)]>` keeps `canvas_media_fingerprint` working identically with zero risk of stale canvas composition. |
| **RISK-04** | `crates/ui-egui/src/media_store.rs` (duplicate media names) | Today's `Vec` linear search and M34's `index.entry(name).or_insert(idx)` both return the **first** entry when duplicate names exist, while `replace` updates the first match and `retain` removes all matching names. **Mitigation:** Preserve exact first-match semantics in `MediaStore::push`/`replace`/`truncate`/`retain` and assert in `R1-05`. |
| **RISK-05** | `apps/coursecraft/src/player_bundle.rs` (`Arc<[u8]>` in S5) | `PLAYER_FILES` holds `&'static [u8]` slices embedded by `build.rs`. Converting `entry.1` via `Arc::from(entry.1)` creates one `Arc<[u8]>` per player file, then `build_package_entries` clones only the `Arc` pointer (0 byte copies). Verify `build.rs` output is untouched and `cargo xtask build-player` embeds cleanly. |
| **RISK-06** | `crates/publish/src/lib.rs` (`PackageZipper` deflate block boundaries) | Deflate output depends on write chunk size; `PackageZipper::step` writes non-precompressed entries whole (`!is_precompressed(&file.path)`) so chunked zips are byte-identical to `zip_package`. **Mitigation:** Keep that exact branch inside `ZipperCore` / `PackageZipper` and verify across 5 chunk sizes in `R2-01`. |
| **RISK-07** | `crates/engine/src/commands/mod.rs` (`REGISTRY` order & `docs/cli.md`) | `coursecraft-cli commands`, editor menus, and `docs/cli.md` depend on `REGISTRY` order, ids, labels, menus, shortcuts, and param strings. **Mitigation:** `R5-01` locks down all 186 rows in exact order before S6 starts; never touch `docs/cli.md`. |
| **RISK-08** | `crates/engine/src/commands/{styles,csv_import}.rs` (`super::` imports) | Both existing submodules import private helpers from `super::` (`bad`, `opt`, `req`, `keep_valid`, `find_object_mut`, `objects_param`, `bank_create`, `new_bank_slide_with_id`, `next_fresh_id`, `validate_id`). **Mitigation:** Keep `pub(super) use` re-exports in `commands/mod.rs` during S6–S8 so `styles.rs` and `csv_import.rs` stay green on every step. |
| **RISK-09** | `crates/engine/src/commands/interaction.rs` (ID generation order in `interaction_insert`) | Splitting `interaction_insert` into 8 builders could accidentally change `fresh_id` allocation order or layer/object ordering, altering generated IDs or JSON. **Mitigation:** `R5-02` compares full `Course::to_json()` byte-for-byte before and after S11 across all 8 kinds. |
| **RISK-10** | `apps/coursecraft/src/{app,desktop,web,control_adapter}.rs` (`wasm32` `-D warnings` & test imports) | Moving `EditorApp` methods across desktop and web modules easily creates dead-code or unused-import warnings on only one target (`wasm32` vs native), or breaks `use crate::{...}` in the 8 test files. **Mitigation:** Run BOTH `cargo test -p coursecraft` AND `cargo clippy --target wasm32-unknown-unknown -p coursecraft -- -D warnings` on S12, S13, and S14, and keep crate-root `pub(crate) use` re-exports in `main.rs`. |

---

## 7. Release Gate for M36

Run all 5 commands in order after S14 before merging/releasing:

1. **`cargo xtask ci`** — verifies `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, full workspace `cargo test` (~3,932+ existing tests + all new `R1`/`R2`/`R5`/`R8` tests), `cargo xtask layers` (`layers: ok`), and `cargo xtask wasm` (all 17 crates on `wasm32-unknown-unknown`).
2. **`cargo xtask build-player`** — verifies the embedded release WASM web player builds cleanly with Trunk + `wasm-opt`.
3. **`cargo test --release -p coursecraft-engine undo_perf_tests -- --ignored --nocapture --test-threads=1`** — verifies `PF-01`–`PF-03` (`perf300` & `rich300` undo/redo memory and timing budgets) remain within budget after R5.
4. **`cargo test --release -p coursecraft-ui-egui perf_tests -- --ignored --nocapture --test-threads=1`** — verifies all 9 release editor perf benchmarks (including `PF-05` Lite 300-slide `<= 64 MiB`) remain within budget after R1.
5. **`cargo test --release -p coursecraft-cli --test big_course -- --ignored --nocapture`** — verifies R2's streaming `open_project_from_with_limits` and zero-copy `PackageInput` / `zip_package_to` on the large-course benchmark.

---

## 8. Owner Verification Checklist (~5 Minutes)

Because M36 is a pure refactor with zero intended user-visible changes, manual verification is a quick smoke check:

- [ ] **Open, edit, undo, save & reopen (Desktop):** Open `examples/showcase/` in the desktop app; insert one interaction (e.g. Tabs), edit an object, press `⌘Z` / `Ctrl+Z` and redo, inspect **Media Library** (check usage counts and missing-media badge), save as `.ccraft`, and reopen — confirm identical look and behaviour.
- [ ] **Publish (Desktop):** Publish `examples/showcase/` to SCORM 1.2 and Web (`File ▸ Publish`), confirm the zip builds cleanly and opens in the browser with all images, fonts, and audio intact.
- [ ] **Web editor smoke check:** Run `trunk serve` in `apps/coursecraft`, open a `.ccraft` file, import an image, and publish to Web — confirm the file pickers work and the chunked publish progress bar completes cleanly with no browser console errors.
- [ ] **CLI & docs parity:** Confirm `docs/cli.md` has zero diff and `coursecraft-cli commands` output is unchanged.
