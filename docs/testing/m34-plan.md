# M34 Everyday Performance & Low-End Hardware Speed — Engineering, Budget & Test Plan

**Role:** Systems Performance Audit, Low-End Hardware Budget Spec & Implementation Plan (planning only — zero source files edited)  
**Target release:** 0.4.5 / M34 (builds directly on M32 cached visible-only thumbnails and M33 structural delta undo)  
**North Star:** Software that is **free AND fast**. CourseCraft must feel effortless every day on low-end hardware — a **4 GB RAM dual-core laptop with integrated graphics**, an **aging Intel Mac**, a **cheap Chromebook running the web editor**, and a **learner's budget smartphone running a published course** over a slow connection. Startup time matters least; what matters is milliseconds saved on every click/nudge/keystroke/scroll, dramatic cuts in RAM and GPU/texture memory, **0% CPU when idle** (in both editor AND learner player), non-blocking saves and autosaves, smaller and faster opens/publishes, and a much leaner published WASM player.

**Builds on M32 & M33 (do not redo them):**
- M32 already ships visible-only slide-list composition with revision/fingerprint-keyed `ThumbnailCache` (`MAX_CACHED_THUMBNAILS = 128`) and `NavigatorIndex`, cutting idle 300-slide editor frames from `1.37 ms` to `0.48 ms`.
- M33 already ships `CourseDelta` + `UndoScope` in `coursecraft-engine`, cutting 200-step undo history on 300-slide courses from `392 MB` / `1,900 MB` down to `181.6 KB` / `387.5 KB` and undo/redo latency to `0.0001 ms`.

**Clean-room commitment:** Pure Rust engineering across existing CourseCraft crates (`docs/contributing.md:21-40`). Zero competitor code, assets, or terminology. Zero new patent exposure.

> **Implementer Guardrails (`AGENTS.md`, `docs/contributing.md`)**
> 1. **Never crash:** Zero `unwrap`/`expect`/`panic!`/`unreachable!`/`todo!` in non-test code; `unsafe_code = "forbid"` everywhere except `coursecraft-video-native`. No `unsafe` global allocators in workspace crates; all memory/work telemetry uses 100% safe Rust counters and tracked byte budgets.
> 2. **Everything is a command:** All document mutations still go through `coursecraft_engine::Session::execute`; never mutate `Course` directly from UI caches or helpers.
> 3. **One runtime:** Any play-time wake/idle scheduling logic (`Player::next_wake_ms`) lives inside `coursecraft-logic` so editor preview, CLI `simulate`, and published player stay identical.
> 4. **Spec first:** If any runtime timing or animation-wake rule is documented in `docs/spec/triggers.md`, update the spec in the same step (M34 changes *when* hosts repaint, never *what* triggers/timers fire).
> 5. **Layering & WASM:** `cargo xtask layers` and `cargo xtask wasm` (all 17 WASM crates) stay green on every step.
> 6. **Preserve determinism & fidelity:** Saved `.ccraft` archives, `.ccraft-folder` diffs, and undo/redo `to_json()` remain byte-identical where required; published `course.json` switches to compact JSON only for published learner packages (still valid `schema_version = 1` parsed identically by `Course::from_json`).

---


> **Reviewer notes (Claude, owner's reviewer) — these override the plan below.**
> 1. **No refactors in M34.** The owner has not approved any item in `refactor-proposal.md` (R1, R2, R4, R9, ...). Where a step cites a refactor, do the smallest change that gets the speed win without reshaping APIs: S10 uses a private `media_index: HashMap<String, usize>` next to the existing `media` Vec (no `MediaStore` type, no shared media visitor); S9 streams Open through a `Read + Seek` path and avoids `to_vec()` copies inside publish, but keeps the public `PackageInput`/`PackageFile` signatures unless a change is unavoidable and local; S4/S5 remove dependencies and fonts only where listed, with no module reshuffles.
> 2. **Fonts:** remove `epaint_default_fonts` only from the published player and web builds where CourseCraft's own bundled fonts cover every glyph the player chrome draws; keep the desktop editor's fallback fonts. Add a test that the player renders its chrome strings (including symbols such as ▶, ✓, ×) without missing glyphs.
> 3. **Lite mode** must never change what a course looks like when published; it may only lower editor-side cache sizes and preview quality.
> 4. **Measure with the same method before and after**, report real numbers in the M34 report, and keep every existing perf budget passing.
> 5. Event-driven repaint (P1) must still wake for timers, media, animations, input and timeline cues: add tests for each wake source.

## 1. Goals & Clean-Room / Patent Check

### 1.1 Goals
1. **Zero idle CPU burn everywhere:** Eliminate the learner player's and editor preview's unconditional 60 fps `request_repaint()` loop so a learner reading a static slide or quiz on a cheap phone uses **~0% CPU/GPU** instead of draining battery at 60 fps.
2. **Low-end RAM & GPU/VRAM survival (4 GB laptop & cheap phone):**
   - Decode raster images at display-tier pixel size (thumbnails never decode 4K photos into 33 MB GPU textures!), deduplicate identical media bytes into one GPU texture, and enforce a unified memory ceiling across pictures, SVG rasters, and decoded GIF/WebP CPU frames.
   - Stream `.ccraft` Open directly from disk and use zero-copy `Arc<[u8]>` in Publish, cutting peak RAM on big courses in **half** (`~2.75 GB → ~1.40 GB`, saving **~1.35 GB** on both Open and Publish).
3. **Sub-millisecond editing on 300-slide courses:** Eliminate the hidden `Course::validate()` bottleneck uncovered in M33's release benchmark (`PF-01`/`PF-02`/`PF-03` still spend `1.15–4.60 ms` per edit solely inside `keep_valid`!) and remove double validation + JSON round-tripping in `Editor::check_revision`.
4. **Zero-stutter everyday authoring:** Move autosave off the UI thread (ending 1–10 s freezes every 60 s on large projects), fix the `FontDefinitions` per-text-run full-clone bug, cache laid-out text (`Autofit::Shrink` currently runs 22× layout per frame!), cache triangulated/gradient shape meshes, cache main-canvas composition, and make Check Spelling `Ignore` O(1) instead of re-scanning all 300 slides (`9.55 ms → ≤ 0.2 ms`).
5. **Lean mobile web player & Chromebook web editor:** Strip egui's ~1.6 MB `epaint_default_fonts`, drop `coursecraft-format` (`quick-xml`/`xliff`/`zip`) from `coursecraft-player`, emit compact `course.json` in published packages (**38–62% smaller** raw JSON), and fix the web editor's missing `wasm-release` Trunk profile.
6. **Explicit low-end hardware budgets, Lite mode & deterministic CI guards:** Add a `PerformanceMode` (`Standard` / `Lite` / `Auto`) for constrained devices, low-end perf budgets for 4 hardware tiers, and counter-based CI guard tests that lock every single win against regression.

### 1.2 Clean-Room & Patent Check (`docs/contributing.md:21-40`)
- **SCORM data (KnowBe4 US10949760 / US11599810 / US11847579):** Untouched.
- **Automatic course from documents (Docebo US11720617B2):** Untouched.
- **Screen recording / software simulation (Assima US9570050B2 / US12360646B2):** Untouched.
- **Learner classification / adaptive content (Adobe US10911553B2 / US10943497B2):** Untouched.
- **Third-party assets:** Removing `epaint_default_fonts` reduces third-party font footprint; any compact fallback monospace font (if added) must be permissively licensed (OFL/MIT/Apache-2.0) with its licence file and `ATTRIBUTION.md` entry, or use bundled `Inter` directly (zero new assets).

---

## 2. Baseline Measurements (Measured Before Planning)

All measurements below were run directly on this checkout (Apple Silicon macOS, `--release`, read-only) prior to planning.

### 2.1 Release Editor Frame-Time Baseline (`coursecraft-ui-egui::perf_tests`)
Command:
```sh
cargo test --release -p coursecraft-ui-egui perf_tests -- --ignored --nocapture --test-threads=1
```

| Stage (300-slide course, 1280×900) | Existing Budget | Measured Baseline (median / p95) | Target After M34 (median / p95) | Primary Win Driving Improvement |
|---|---:|---:|---:|---|
| `idle` | 16 ms | **0.48 ms / 0.50 ms** | **≤ 0.25 ms / ≤ 0.30 ms** | P3 (font clone fix), P7 (canvas/text/mesh cache), P9 (no per-frame allocs) |
| `select slide 150` | 33 ms | **0.69 ms / 0.84 ms** | **≤ 0.40 ms / ≤ 0.50 ms** | P3, P7, P9 |
| `scroll slide list` | 33 ms | **0.43 ms / 0.46 ms** | **≤ 0.25 ms / ≤ 0.30 ms** | P3, P7, P9 |
| `typing in a text field` | 33 ms | **0.58 ms / 0.61 ms** | **≤ 0.30 ms / ≤ 0.38 ms** | P3, P4, P7 |
| `type on canvas with spell check` | 33 ms | **0.75 ms / 0.78 ms** | **≤ 0.45 ms / ≤ 0.52 ms** | P3, P7, P9 (`coursecraft-spell` alloc removal) |
| `check spelling, open` | 250 ms | **29.04 ms** | **≤ 15.0 ms** | P9 (`coursecraft-spell` zero-alloc word scan) |
| `check spelling, ignore step` (20 issues) | 33 ms | **9.55 ms / 11.55 ms** | **≤ 0.40 ms / ≤ 0.60 ms** (~25× faster) | P9 (in-place issue filtering instead of 300-slide rescan) |
| `insert interaction + undo` (8 kinds) | 100 ms | **3.27 ms / 4.11 ms** | **≤ 1.50 ms / ≤ 2.00 ms** | P4 (fast validation + cached engine validation) |
| `undo / redo move` (30 cycles) | 33 ms | **7.03 ms / 8.55 ms** | **≤ 2.50 ms / ≤ 3.20 ms** (~2.8× faster) | P4 (scoped validation + eliminate double validate), P7 |
| `visit every slide` | 33 ms | **0.61 ms / 1.07 ms** | **≤ 0.35 ms / ≤ 0.55 ms** | P3, P7 |
| `visit every slide, real pictures` | 33 ms | **3.26 ms / 6.08 ms** (81 decoded, ~300 MB) | **≤ 1.50 ms / ≤ 3.00 ms** (≤ 128 MB, dedup/downscaled) | P2 (display-size decode + content-hash dedup + tighter budget) |
| `first frame` | 500 ms | **15.76 ms** | **≤ 12.0 ms** | P3, P4 |

*(Observation: `perf_tests.rs::rss_mb()` printed `rss 0 -> 0 MB` because spawning `ps` fails in sandboxed/restricted runners; M34 fixes this with deterministic tracked cache bytes plus a non-spawning OS RSS probe.)*

### 2.2 Release Engine Undo & Edit Baseline (`coursecraft-engine::undo_perf_tests`)
Command:
```sh
cargo test --release -p coursecraft-engine undo_perf -- --ignored --nocapture --test-threads=1
```

| Benchmark | Measured Baseline (total / median / p95) | M33 Budget | Target After M34 | Root Cause Discovered |
|---|---:|---:|---:|---|
| `PF-01` `perf300` 200 edits | **232.87 ms** / **1.146 ms** / 1.188 ms | ≤ 120 ms / 0.60 ms *(over!)* | **≤ 20 ms / ≤ 0.08 ms** (~14× faster) | `keep_valid` runs full 300-slide `Course::validate()` on every single edit |
| `PF-01` `perf300` 200 undos | **0.06 ms** / **0.0001 ms** / 0.0003 ms | ≤ 0.05 ms | ≤ 0.05 ms (keep) | Already O(1) in-place swap |
| `PF-01` `perf300` 200 redos | **0.01 ms** / **0.0001 ms** / 0.0001 ms | ≤ 0.05 ms | ≤ 0.05 ms (keep) | Already O(1) in-place swap |
| `PF-01` `perf300` 100 evictions | **117.76 ms** / **1.165 ms** / 1.172 ms | — | **≤ 12 ms / ≤ 0.10 ms** | Same `keep_valid` bottleneck |
| `PF-02` `rich300` 200 edits | **933.20 ms** / **4.601 ms** / 4.683 ms | ≤ 400 ms / 2.00 ms *(over!)* | **≤ 60 ms / ≤ 0.25 ms** (~18× faster) | `Course::validate()` allocates ~30,000+ path `String`s across 4,500 stateful objects every edit |
| `PF-03` `perf300` 200 no-ops | **234.31 ms** / **1.156 ms** / 1.200 ms | ≤ 100 ms / 0.50 ms *(over!)* | **≤ 5 ms / ≤ 0.02 ms** (~58× faster) | `object_set_frame` runs `keep_valid` even when `frame` is unchanged |

### 2.3 Release CLI Baseline (Example Courses, median of 5 runs)
Binary: `target/release/coursecraft-cli`

| Command & Input | Measured Baseline (median / min) | Target After M34 | Root Cause Discovered |
|---|---:|---:|---|
| `validate examples/hello-course/course.json` (7.3 KB) | **100.51 ms** / 99.40 ms | **≤ 10 ms** (~10× faster) | Unconditionally calls `SystemFontIndex::scan_default()` (~93 ms disk scan!) even when course only uses Inter |
| `validate examples/reference-course/course.json` (59 KB) | **100.30 ms** / 98.07 ms | **≤ 12 ms** | Same unconditional OS font scan |
| `validate examples/healthy-habits/course.json` (442 KB) | **102.98 ms** / 101.99 ms | **≤ 15 ms** | Same unconditional OS font scan |
| `a11y-check examples/healthy-habits/course.json` (442 KB) | **9.07 ms** / 8.52 ms | **≤ 8 ms** | Baseline reference |
| `simulate examples/hello-course/course.json` | **7.06 ms** / 6.82 ms | **≤ 7 ms** | Baseline reference |
| `manifest examples/reference-course/course.json` | **7.37 ms** / 7.00 ms | **≤ 7 ms** | Baseline reference |

### 2.4 Published `course.json` Size: Pretty vs Compact (Measured on Example Courses)

| Course | Pretty JSON | Compact JSON | Raw Savings | Gzip Pretty → Compact | Gzip Savings |
|---|---:|---:|---:|---:|---:|
| `examples/showcase/course.json` | 74,664 B | 28,762 B | **−61.5%** | 5,496 B → 4,514 B | **−17.9%** |
| `examples/rich-states/course.json` | 24,581 B | 12,643 B | **−48.6%** | 2,831 B → 2,409 B | **−14.9%** |
| `examples/healthy-habits/course.json` | 452,866 B | 240,449 B | **−46.9%** | 32,402 B → 25,414 B | **−21.6%** |
| `examples/reference-course/course.json` | 60,375 B | 37,745 B | **−37.5%** | 7,563 B → 6,613 B | **−12.6%** |
| `examples/hello-course/course.json` | 7,478 B | 5,283 B | **−29.4%** | 1,548 B → 1,385 B | **−10.5%** |

### 2.5 Historical Big-Course I/O & Memory Baseline (`docs/performance.md`)
From `docs/performance.md` (300 slides, 1,362 MB media — 300 × 1 MB PNG + 10 × 100 MB video):
- **Open `.ccraft` peak RSS:** **2,735 MB** (2.0× media because `apps/coursecraft/src/files.rs:469` reads the entire `.ccraft` into RAM via `std::fs::read` before `open_project` extracts a second copy!).
- **Publish SCORM 1.2 peak RSS:** **2,756 MB** (2.0× media because `PackageInput::media` returns owned `Vec<u8>` via `bytes.to_vec()` at `apps/coursecraft/src/publish.rs:79`).
- **Autosave UI freeze:** `apps/coursecraft/src/autosave.rs:127-152` runs `save_project_to` + `file.sync_all()` + `verify_ccraft_file` **synchronously on the UI thread** every 60 s (~800 ms on M5 NVMe, **5–30 s freeze** on a low-end laptop HDD/eMMC).

---

## 3. Ranked Performance Candidate Catalog (`P1`–`P11`)

Ranked strictly by **everyday impact on low-end hardware** (4 GB dual-core laptop, old Intel Mac, Chromebook web editor, cheap learner phone).

---

### P1 — Event-Driven Repaint Scheduling in Learner Player & Editor Preview (Stop 60 fps Idle Burn)

- **Where:**
  - `apps/coursecraft-player/src/app.rs:1083` (`Playing::show_ui`: unconditional `ui.ctx().request_repaint();`)
  - `crates/ui-egui/src/preview.rs:840` (`Preview::show`: unconditional `ctx.request_repaint();`)
  - `crates/logic/src/lib.rs` (new pure runtime method `Player::next_wake_ms(&self) -> Option<u32>`)
  - `crates/render/src/paint.rs:3642` (`paint_marker` pulse calls `request_repaint()` — cap to ~30 fps or `reduced_motion`/Lite-mode off)
- **What it costs today:** Every published course running in a browser tab (including on a learner's cheap phone!) and every open Editor Preview repaints at **60 fps continuously forever** — even while sitting on the Start gate (`awaiting_start`), after `ended == true`, while paused (`PlayerPause`), or after a slide's timeline has finished while the learner reads text or thinks about a quiz question. On a low-end phone this holds the CPU + WebGL/WebGPU GPU active continuously (**~25–60% CPU, rapid thermal throttling, 3–5× battery drain**). Measure on desktop/phone via browser Performance / DevTools frame count over 10 s of idle reading (currently **~600 frames / 10 s** instead of **0 frames / 10 s**).
- **The fix:**
  1. Add `pub fn next_wake_ms(&self) -> Option<u32>` on `coursecraft_logic::Player` (L2 — one runtime!):
     - Returns `None` when `!state.started`, when `ended`, or when nothing time-dependent is active (base clock is paused or ended, all visible layer clocks are paused or ended, no running countdown timers, no active motion paths, no playing media/Lottie).
     - Returns `Some(0)` (immediate / vsync 60 fps) ONLY when visual smooth motion is actively occurring: an entrance/exit animation is in progress, a motion path is moving, a slide transition or lightbox fade is active, a playing video or Lottie is advancing, or a seekbar is visibly moving (or throttle seekbar-only slides with no animations to `Some(100)` — 10 fps instead of 60 fps!).
     - Returns `Some(ms)` (exact delay to next event, e.g., next object `start_ms`, next `cue_point`, next `timeline_reaches` trigger, next timer tick, or slide `duration_ms`!) when a timeline is ticking quietly with nothing visually moving between events!
  2. In `Playing::show_ui` (`apps/coursecraft-player/src/app.rs:1083`) and `Preview::show` (`crates/ui-egui/src/preview.rs:840`), remove unconditional `request_repaint()` and instead call `request_repaint()` / `request_repaint_after(Duration::from_millis(ms))` based on `player.next_wake_ms()`, `view.transition_active()`, active GIF/WebP/video/Lottie/DOM-loading state, or `var_changed_at` highlight fade.
  3. Also in `Preview::show` (`crates/ui-egui/src/preview.rs:845`), stop cloning `PlayerState` twice per frame on idle frames (pass `&mut Option<PlayerState>` via `Option::take()` and move it back instead of `.clone()`!).
- **Expected gain:** **−95% to −100% idle CPU/GPU frames** on static/reading/quiz/paused/start/ended slides (from **60 fps → 0 fps**); **−80% CPU/GPU** during quiet timeline segments without visual motion (60 fps → wake at exact event or 10 fps seekbar). Massive battery & thermal win on cheap phones and fanless Chromebooks/MacBooks.
- **Risk:** Medium — missing a wake source could freeze a timer, animation, cue, or seekbar. Mitigate by putting `next_wake_ms` inside `coursecraft-logic` with exhaustive unit tests covering every time-driven feature (base clock, layer clock, entrance/exit, motion path, cue points, `timeline_reaches`, countdown timer, Lottie, video, GIF, seekbar, transition, lightbox fade).
- **Proving tests:**
  - Logic unit tests `W-01` (`crates/logic/src/tests.rs`): `next_wake_ms` returns `None` on start/paused/ended/finished slide, `Some(0)` during entrance/exit/motion-path/Lottie, exact `Some(delta)` to next cue/trigger/object start.
  - Guard tests `G-01` (`crates/ui-egui/src/idle_tests.rs` + `apps/coursecraft-player/src/tests.rs`): `preview_idle_after_timeline_ends_does_not_repaint`, `player_start_gate_and_finished_slide_do_not_repaint` asserting `repaint_delay >= 1s` or `Duration::MAX`.

---

### P2 — Display-Size Raster Decoding, Content-Hash Deduplication & Unified Low-End `MediaCache`

- **Where:**
  - `crates/render/src/media.rs:195-221` (`DEFAULT_PICTURE_BUDGET`, `MediaCache`), `:321-445` (`picture_bytes`, `note_picture_use`, `evict_over_budget`), `:574-607` (`texture_for`), `:917-978` (`texture_with_key`)
  - `crates/media/src/lib.rs:135-201` (`decode_image`, add `decode_image_max_dim`)
  - `crates/render/src/paint.rs:455-457` (pass `target_px` to picture fills too)
- **What it costs today:**
  1. **Thumbnails decode full-resolution photos:** `MediaCache::texture_for(..., target_px)` ignores `target_px` for PNG/JPEG/WebP (`media.rs:596`). A 4000×3000 photo painted only in a `186×105` slide thumbnail or `40×40` Media Library preview (`media_library.rs:622`) decodes into a **48 MB GPU texture** instead of **0.16 MB** (**300× wasted VRAM/RAM!**).
  2. **Duplicate bytes decoded N times:** Courses with reused/duplicated image paths decode and upload separate textures for identical bytes.
  3. **O(N) scan + 2 heap String allocs on EVERY picture lookup EVERY frame:** `texture_for` allocates `src.to_string()` (`media.rs:596`), `note_picture_use` allocates `key.to_owned()` (`media.rs:353`), and `evict_over_budget()` calls `self.picture_bytes()` (`media.rs:371`) which walks the entire `entries` + `animations` maps on EVERY single image paint on EVERY frame!
  4. **Double peak allocation on decode:** `ColorImage::from_rgba_unmultiplied` (`media.rs:937`) copies the entire `img.pixels: Vec<u8>` into a new `Vec<Color32>`, temporarily holding **2× decoded pixels** (e.g., 96 MB for one 4K photo) during upload!
  5. **Unbounded SVG + Animation CPU memory:** `DEFAULT_PICTURE_BUDGET` is **512 MB desktop / 256 MB wasm** — on a 4 GB laptop (shared iGPU RAM) or cheap 2 GB phone, 256–512 MB of textures + 256 MB of `Animation.frames` CPU pixel buffers causes OS swapping or browser tab OOM!
- **The fix:**
  1. Add `decode_image_fitted(bytes: &[u8], max_dim: Option<(u32, u32)>, limits: &Limits) -> Result<Rgba8Image, MediaError>` in `crates/media/src/lib.rs` (returns `(Rgba8Image, [u32; 2])` preserving original natural `[w, h]` for aspect-fit & crop!). When `width > max_w * 2 || height > max_h * 2`, downscale via fast box/triangle filter (`image::imageops::FilterType::Triangle`) before returning — quantized to power-of-two / tier buckets (`Thumbnail <= 384px`, `Medium <= 1024px`, `Canvas <= 2048px`, `Full` only when zoomed > 200% or offscreen export).
  2. **Content-hash texture sharing:** Compute fast 64-bit FNV-1a hash + byte length of `Arc<[u8]>` (cached per `src` via `Arc::as_ptr` + len or in `MediaStore`), so two `src` keys with identical bytes at the same tier share ONE `TextureHandle` (**0 extra VRAM**!).
  3. **Zero-copy `Vec<u8>` → `Vec<Color32>` conversion** (safe chunked conversion or pre-sized single pass dropping `img.pixels` promptly) and **O(1) running `picture_bytes` + `svg_bytes` + `animation_cpu_bytes`** counters on `MediaCache`, eliminating `src.to_string()` / `key.to_owned()` on cache hits (`entries.get(key)` before allocating!) and eliminating `picture_bytes()` map scans on every hit!
  4. **Lower default low-end-safe ceilings + unified accounting:** Count SVG rasters and decoded animation CPU frames in a unified budget; lower defaults to **256 MB desktop / 96 MB wasm** (`64 MB` in Lite mode), and evict cold `AnimationEntry` CPU frames (`frames` + `animation`) once not used for > 120 frames.
- **Expected gain:**
  - **−70% to −95% GPU/texture RAM** on thumbnail-heavy and photo-heavy courses (e.g., 300 slides with 1280×720 PNGs: thumbnail tier drops per-thumbnail texture from **3.69 MB → 0.16 MB (23× smaller)**; 81 cached thumbnails drop from **~300 MB → ~13 MB**!).
  - **−30% to −60% decode time** on `visit every slide, real pictures` (`3.26 ms → ≤ 1.50 ms`), plus **0 per-frame String allocations** on cache hits.
- **Risk:** Medium — must preserve exact `natural_size` (`size: [u32; 2]` in `MediaView::Ready`) so `image_dest_rect` and crop coordinates never shift, and must upgrade a thumbnail-tier texture to canvas-tier when the slide is selected on the main canvas (without thrashing when both thumbnail and canvas paint the same image on the same frame: canvas tier satisfies thumbnail requests!).
- **Proving tests:**
  - Unit tests `U-01` (`crates/media/src/tests.rs`): `decode_image_fitted_downscales_and_preserves_natural_dims`.
  - Unit tests `U-02` (`crates/render/src/media.rs`): thumbnail-only paint stays in thumbnail tier (`<= 384px`), canvas upgrades cleanly, duplicate-bytes images share one `TextureId`, unified budget bounds SVG + animation + picture bytes.
  - Guard test `G-02` (`crates/ui-egui/src/guard_tests.rs`): scrolling 100-slide picture list keeps `media_cache.total_texture_bytes() <= 8 MiB` even with 1280×720 source PNGs!

---

### P3 — Fix `FontDefinitions` Per-Text-Run Full Clone Bug & Strip `epaint_default_fonts` (~1.5 MB WASM Win)

- **Where:**
  - `crates/render/src/font.rs:206-220` (`pending_defs_for_pass`, `set_fonts_and_track`), `:348-357` (`install_course_fonts`), `:437-440` (`family_exists`), `:105-196` (`base_definitions`)
  - `Cargo.toml:45-46` (`egui` / `eframe` `"default_fonts"` feature)
  - `crates/render/src/design.rs:477` & `crates/render/src/player_frame.rs:1565,1962` (`FontFamily::Monospace` usage)
- **What it costs today:**
  1. **Hidden per-frame clone bug (`font.rs:206-220,438`):** `install_base` inserts `(pass, FontDefinitions)` into `ctx.data` under `PENDING_DEFS_ID` on frame 1 and **never removes it**. Thereafter, EVERY call to `slide_font_named` → `family_exists` → `pending_defs_for_pass` calls `data.get_temp::<(u64, FontDefinitions)>`, which **clones the entire `FontDefinitions` struct** (all BTreeMaps, 12+ family `Vec<String>`s, and `Arc<FontData>`s!) on EVERY SINGLE TEXT OBJECT on EVERY SINGLE FRAME — just to discover `saved_pass != pass` and immediately drop the clone! On a slide with 20 text boxes + 7 visible thumbnails (~45 text calls/frame), that's **45–100+ full `FontDefinitions` deep clones per frame**!
  2. **Per-frame `install_course_fonts` allocations (`font.rs:348-357`):** When a course has custom fonts, `Editor::show` (`lib.rs:4240`) and `PlayerApp::ui` (`app.rs:1042`) call `install_course_fonts` EVERY frame, which allocates a `Vec<FaceFingerprint>` + clones strings + clones the previous `FontFingerprint` out of `ctx.data` every frame.
  3. **~1.5–1.7 MB bloated `epaint_default_fonts` in every binary & published WASM:** `Cargo.toml:45-46` enables `default_fonts` on `egui` and `eframe`, embedding `Ubuntu-Light.ttf` (~400 KB), `Hack-Regular.ttf` (~340 KB), `NotoEmoji-Regular.ttf` (~900 KB), and `emoji-icon-font.ttf` (~40 KB) into BOTH the editor and every published SCORM/LTI/Web course — even though CourseCraft already bundles 6 weights of `Inter` (~342 KB) + `Phosphor` (~56 KB) and overrides `FontFamily::Proportional` with Inter!
- **The fix:**
  1. **Zero-clone `family_exists` (`font.rs`):** Store a lightweight `InstalledFamilySet` (`Arc<HashSet<FontFamily>>` + `u64` pass/epoch) in `ctx.data` (updated ONLY inside `set_fonts_and_track`), and remove `PENDING_DEFS_ID` after the pass advances (or store `Arc<FontDefinitions>` so reading never deep-clones BTreeMaps!). Even better: `family_exists` checks the `Arc<HashSet<FontFamily>>` in **O(1) with 0 clones and 0 allocations**!
  2. **Revision-gated `install_course_fonts`:** Cache `(course_revision, media_count)` (or fast hash without `String` cloning) before building `FontFingerprint`.
  3. **Remove `default_fonts` from `Cargo.toml` (`egui` and `eframe`):** Start `base_definitions()` from `FontDefinitions::empty()`, map `FontFamily::Proportional` to `inter_regular` + `phosphor_regular`, and map `FontFamily::Monospace` to `inter_regular` in the player (or a tiny subsetted ASCII-only monospace TTF ~25–35 KB in `crates/render/fonts/` if strict monospace alignment is desired for editor hex/ID/code fields — with licence + `ATTRIBUTION.md`). *(Check every `FontFamily::Monospace` site: 14 sites total; timecodes like `00:12 / 01:30` look crisp in Inter tabular digits or a 30 KB ASCII mono subset!)*
- **Expected gain:**
  - **−1.4 to −1.6 MB uncompressed (≈ −700 KB to −900 KB gzipped)** from **EVERY published web player `.wasm`** and web editor `.wasm`!
  - **−20% to −35% idle/scroll frame CPU time** in editor & player from eliminating 50–100 `FontDefinitions` clones/frame!
  - **Smaller GPU font atlas & lower startup RAM** (zero Ubuntu/Hack/NotoEmoji glyph tables parsed by `ab_glyph`).
- **Risk:** Low–Medium — check whether any test relies on an emoji glyph from `NotoEmoji-Regular` or expects `FontDefinitions::default()` without calling `install_fonts(ctx)`. Mitigate by checking `crates/render/src/tests.rs` and `crates/ui-egui/src/font_harness_tests.rs` and ensuring `TestHarness::new` + `PlayerApp` always call `install_fonts(ctx)` (and keeping Phosphor icons for all UI symbols).
- **Proving tests:**
  - Guard test `G-03` (`crates/render/src/font.rs` + `guard_tests.rs`): `slide_font_named` performs **0** `FontDefinitions` clones across 100 calls (`test_counts::font_def_clones() == 0`).
  - Unit test `U-03` (`crates/render/src/font.rs`): `base_definitions()` contains zero `Ubuntu-Light` / `Hack` / `NotoEmoji` entries, resolves both `Proportional` and `Monospace` cleanly, and renders ASCII + Latin + Phosphor glyphs without panic.
  - Build/size check `W-02`: `Cargo.lock` / `cargo tree -p coursecraft-player` has zero `epaint_default_fonts` dependency; measure player `.wasm` before/after.

---

### P4 — Zero-Allocation `Course::validate()`, Scoped Validation & Eliminate Double-Validation on Every Edit

- **Where:**
  - `crates/course/src/validate.rs:199,387,438,686,692,695,706,711` (`Course::validate`, `check_slide`, `check_object`, `check_masters_and_templates`, `check_text_styles`)
  - `crates/engine/src/commands.rs:1956-1972` (`keep_valid`), `:5726-5734` (`object_set_frame`), and no-op checks
  - `crates/engine/src/lib.rs:346-448` (`Session::is_playable`, `execute_single_slide`, `last_validation` cache)
  - `crates/ui-egui/src/lib.rs:4133-4156` (`Editor::refresh_validate_only`)
- **What it costs today (measured!):**
  - `PF-01` (`perf300` 200 edits): **1.15 ms/edit** (232.9 ms total) — **98% of that time is `keep_valid` calling `Course::validate()` over all 300 slides!**
  - `PF-02` (`rich300` 200 edits): **4.60 ms/edit** (933.2 ms total) — **99% of that time is `Course::validate()`!**
  - `PF-03` (`perf300` 200 no-ops): **1.16 ms/no-op** (234.3 ms total) — runs full `Course::validate()` even when nothing changed!
  - Editor `undo / redo move` (30 cycles): **7.03 ms/cycle** — runs `Course::validate()` **4 times per cycle** (`keep_valid` + `refresh_validate_only` via JSON round-trip on nudge, undo, and redo)!
  - On a dual-core 4 GB laptop (~4–5× slower single-core than M5), a single edit on `rich300` currently stalls **~20–35 ms** just in validation!
- **The fix:**
  1. **Zero-allocation path stack in `crates/course/src/validate.rs`:** Replace eager `format!("/scenes/{si}/slides/{li}/objects/{i}/states/{k}")` passed into every recursive `check_*` call with a lightweight borrowed path stack (`PathSeg` enum on `Ctx`, pushed/popped in O(1) with zero heap allocs; materialized into a `String` **only** inside `cx.err(...)` / `cx.warn(...)` when an issue actually occurs!). Also remove `cx.course.text_styles.clone()` (`validate.rs:387`) and `cx.course.masters.get(mi).cloned()` (`validate.rs:438`) by separating `&Course` from `&mut Vec<Issue>` (no borrow conflict!).
  2. **Early-exit `is_playable()` & single-slide validation (`Course::validate_single_slide`):**
     - Add `Course::first_error(&self) -> Option<Issue>` that skips warning-only checks (like `slide_has_unsupported_script` scanning every character of every string on all 300 slides!) and stops immediately on the first `Severity::Error`!
     - For `UndoScope::SingleSlide` commands in `Session` that only touch in-slide geometry/styling/text (and cannot invalidate cross-slide invariants like slide IDs, bank draws, or `start_slide`), validate ONLY the modified slide (using pre-built slide/variable ID sets or `validate_slide_in_context(course, loc)`), falling back to `first_error()` for commands that change triggers/questions/IDs.
     - Short-circuit no-ops before `keep_valid` (e.g., in `object_set_frame`: if `obj.frame == frame { s.skip_undo = true; return Ok(Json::Null); }` and in `keep_valid`: `if s.skip_undo { return Ok(()); }`!).
  3. **Cache validation issues on `Session` & eliminate editor double-validate + JSON round-trip:**
     - Add `pub fn validation_issues(&mut self) -> &[Issue]` on `Session` keyed by `self.revision`.
     - In `Editor::refresh_validate_only` (`crates/ui-egui/src/lib.rs:4133`), read `self.session.validation_issues()` directly (still 100% engine-owned, zero direct `Course::validate()` calls outside engine!) instead of serializing/deserializing `course.validate` through `serde_json::Value` every edit!
     - Only rebuild `self.ui.last_known_slides` (`lib.rs:4006`) when slide count/IDs changed (not on every object nudge!).
- **Expected gain:**
  - `Course::validate()` itself: **3–5× faster** across CLI, MCP, Player startup, and Editor (`rich300`: `4.5 ms → ≤ 1.0 ms`).
  - `PF-01` (`perf300` 200 edits): **1.15 ms → ≤ 0.08 ms/edit** (**~14× faster!**).
  - `PF-02` (`rich300` 200 edits): **4.60 ms → ≤ 0.25 ms/edit** (**~18× faster!**).
  - `PF-03` (`perf300` 200 no-ops): **1.16 ms → ≤ 0.02 ms/no-op** (**~58× faster!**).
  - Editor `undo / redo move`: **7.03 ms → ≤ 2.50 ms** (**~2.8× faster!**).
- **Risk:** Medium — scoped validation MUST never allow an invalid course through. Mitigate with a `#[cfg(test)]` oracle test verifying `validate_slide_only` agrees 100% with full `Course::validate()` across all 177 mutating commands in `m27_all_commands_undo_tests.rs` and `panic_hunt`!
- **Proving tests:**
  - Differential test `U-04` (`crates/course/src/tests.rs`): `first_error` matches `validate().into_iter().find(Error)` identically on every valid and invalid test course.
  - Differential test `U-05` (`crates/engine/src/m27_all_commands_undo_tests.rs`): scoped validation matches full validation across all 186 commands + `panic_hunt`.
  - Release benchmarks `PF-01`, `PF-02`, `PF-03` (`crates/engine/src/undo_perf_tests.rs`) and `PF-04` (`undo / redo move` in `perf_tests.rs`).

---

### P5 — Zero-Copy Streaming `.ccraft` Open & Publish (−50% Peak RAM / −1.35 GB!) + Compact Published `course.json`

- **Where:**
  - `crates/format/src/lib.rs:368-459` (`open_project` / `open_project_with_limits` → add `open_project_from<R: Read + Seek>`)
  - `apps/coursecraft/src/files.rs:465-486` (`open_from` currently calls `std::fs::read(path)` on the whole archive!)
  - `crates/publish/src/lib.rs:45-58,412-659` (`PackageFile`, `PackageInput::media`, `build_package_standard_with_folders`, `zip_package_to`)
  - `apps/coursecraft/src/publish.rs:62-97` (`package_files_from` currently calls `bytes.to_vec()` on every media asset!)
  - `crates/course/src/lib.rs:351-357` (add `Course::to_json_compact()`)
  - `apps/coursecraft-cli/src/lib.rs` (CLI open & publish paths)
- **What it costs today (measured in `docs/performance.md` & §2.4):**
  - Opening a 1,362 MB `.ccraft` peaks at **2,735 MB RSS** (`std::fs::read` loads 1,362 MB into a `Vec<u8>`, then `open_project` allocates a second 1,362 MB for `Project.media`). On a **4 GB RAM laptop**, opening a 1 GB course causes immediate heavy disk thrashing or OOM!
  - Publishing a 1,362 MB course peaks at **2,756 MB RSS** because `PackageInput::media` returns owned `Vec<u8>`, copying every video/image a second time!
  - Published `course.json` is pretty-printed, inflating size by **38–62%** (`healthy-habits`: 453 KB → 240 KB; `showcase`: 75 KB → 29 KB).
- **The fix:**
  1. **Streaming `open_project_from<R: Read + Seek>` in `crates/format/src/lib.rs`:** Refactor `open_project_with_limits` to delegate to `open_project_from_with_limits(std::io::Cursor::new(bytes), limits)`. In `apps/coursecraft/src/files.rs:483` and `apps/coursecraft-cli`, open a `BufReader::new(File::open(path)?)` and call `open_project_from` directly — **zero whole-archive RAM copy!**
  2. **Shared-bytes `Arc<[u8]>` in `coursecraft-publish`:** Add `SharedPackageFile { pub path: String, pub bytes: Arc<[u8]> }` + `SharedPackageInput` (or `PackageBytes(Arc<[u8]>)` / `build_package_shared` + `zip_shared_package_to`, keeping the existing `PackageInput`/`PackageFile` API intact for backwards compatibility or delegating through `Arc<[u8]>`). Desktop & Web editor publish pass `Arc::clone(bytes)` — **0 media byte copies!**
  3. **Compact published `course.json`:** Add `Course::to_json_compact(&self) -> Result<String, serde_json::Error>` (`serde_json::to_string(self)`). Use `to_json_compact()` in `apps/coursecraft/src/publish.rs`, `apps/coursecraft-cli` `publish`, and `build_package_standard_with_folders` (when templates are stripped). Keep `Course::to_json()` (pretty) for `.ccraft`, `.ccraft-folder`, and undo tests!
  4. **Use a 256 KiB `BufWriter` / `BufReader`** (`BufWriter::with_capacity(256 * 1024, file)`) in `files.rs` and `publish.rs` (instead of Rust's tiny 8 KiB default) to cut write/verify syscalls by **32×** on large projects.
- **Expected gain:**
  - **Open `.ccraft` peak RAM:** `2,735 MB → ~1,390 MB` (**−49% / −1.35 GB saved!**).
  - **Publish SCORM/LTI peak RAM:** `2,756 MB → ~1,410 MB` (**−49% / −1.35 GB saved!**).
  - **Publish time on 1.3 GB course:** **−15% to −25%** (skips 1.36 GB `memcpy` + 32× fewer write syscalls).
  - **Published `course.json` size:** **−38% to −62% raw**, **−13% to −22% gzipped**, and **~20–30% faster JSON parse** on learner phones.
- **Risk:** Low — `open_project_from` enforces the exact same `Limits` / zip-bomb caps; check if any existing publish test asserts exact whitespace/newlines in published `course.json` and update those assertions explicitly.
- **Proving tests:**
  - Unit tests `U-06` (`crates/format/src/tests.rs`): `open_project_from` produces identical `Project` and rejects identical hostile zips as `open_project`.
  - Guard test `G-04` (`crates/publish/src/tests.rs`): `build_package_shared` shares `Arc::ptr_eq` with input media buffers (**0 media byte copies!**) and emits valid compact `course.json` that parses identically via `Course::from_json`.
  - Release big-course benchmark (`apps/coursecraft-cli/tests/big_course.rs` stages `open` and `publish` under `/usr/bin/time -l`): peak RSS drops below **1.7 GB** (down from 2.75 GB!).

---

### P6 — Non-Blocking Background Autosave (End 60-Second UI Freezes)

- **Where:**
  - `apps/coursecraft/src/autosave.rs:127-152,254-281` (`write_autosave_slices`, `Autosaver::maybe_autosave_slices`)
  - `apps/coursecraft/src/main.rs:1049-1071` (`EditorApp::update_autosave`)
- **What it costs today:** Every 60 seconds when dirty, `update_autosave` runs **synchronously on the main UI thread**: serializes `course.json`, streams ALL media (up to 1.3+ GB!) to `.autosave.ccraft.tmp`, calls `file.sync_all()`, and then reads the ENTIRE file back from disk in `verify_ccraft_file`! On a low-end laptop with HDD or slow eMMC, this locks up the editor completely for **5–30 seconds once a minute**.
- **The fix:**
  1. Run `write_autosave_arc` on a single background worker thread (managed via `autosave_rx: Option<Receiver<Result<u64, String>>>` on `EditorApp`, matching the existing `publish_rx` pattern!).
  2. Snapshot `(self.editor.session.course.clone(), self.editor.used_media())` (which clones only `Arc<[u8]>` handles — **0 media byte copies!**) and spawn the write+verify off the UI thread.
  3. While a background autosave is in flight, skip launching a second one; when it completes, record `last_autosaved_revision = Some(rev)` without waking the UI until needed (just one repaint on completion).
  4. **Bonus I/O cut (safe recovery fast-path):** For projects that are already saved as a `.ccraft-folder`, autosave only `course.json` when media hashes haven't changed (or always stream off-thread with a 256 KiB `BufWriter` so UI thread time is **< 2 ms** regardless!).
  5. On graceful quit/save, wait briefly or cancel/clean stale `.autosave.ccraft` cleanly so a finished background autosave cannot recreate an autosave file *after* manual Save deleted it (guard with a monotonic `save_generation: u64`!).
- **Expected gain:** UI-thread autosave pause drops from **800 ms – 30,000 ms → < 2 ms** (**> 400× reduction in UI hitching**).
- **Risk:** Medium — race between background autosave finishing right after a manual `Save` (could leave an orphaned `.autosave.ccraft` newer than the manual save!). Mitigate strictly with a `save_epoch: Arc<AtomicU64>` checked before the atomic rename in `write_and_swap` (if epoch bumped, discard temp file instead of renaming!).
- **Proving tests:**
  - Unit tests `U-07` (`apps/coursecraft/src/autosave.rs` + `files_tests.rs`): background autosave writes valid verifiable `.ccraft`, coalesces concurrent triggers, and aborts rename when `save_epoch` advances mid-write (never leaves a newer autosave after manual Save).

---

### P7 — Text Layout Cache (`LaidText` + `Autofit::Shrink`), Shape/Gradient Mesh Cache & Main-Canvas Compose Cache

- **Where:**
  - `crates/render/src/text_layout.rs:107-167` (`layout_text_with_wrap` — especially `Autofit::Shrink`'s 20-step binary search!)
  - `crates/render/src/paint.rs:222,583,601,929` (`paint_gradient_fill`, `paint_shape`, `paint_outline`, `paint_text_with_options`)
  - `crates/render/src/shape_fill.rs:179,449` (`triangulate_polygon`, `fill_contours`, `fill_contours_linear_shaded`)
  - `crates/ui-egui/src/canvas.rs:7113` (`show_canvas_and_layers` calling `compose_authoring_layers_opts` every frame!)
- **What it costs today:**
  1. **Text layout recomputed every frame:** Every visible text box (canvas + ~6–8 thumbnails = ~25–45 text boxes/frame) runs `split_paragraphs`, clones `runs` into `dimmed_runs` even when `!dimmed && opacity == 1.0` (`paint.rs:929`), and shapes every paragraph from scratch. If a box has `Autofit::Shrink` and overflows, `layout_text_with_wrap` runs **22 full text layouts per box per frame** (`text_layout.rs:157-166`)!
  2. **O(N³) ear clipping & gradient subdivision every frame:** Every non-rect shape (arrows, stars, callouts, speech bubbles, rounded shapes) and every gradient fill runs `shape_contours_adjusted` → `clean_contours` → `triangulate_polygon` (O(N³)) → triangle subdivision (`GRADIENT_MAX_EDGE = 12.0` pt) from scratch on EVERY frame!
  3. **Main canvas recomposes every frame:** `canvas.rs:7113` calls `compose_authoring_layers_opts` unconditionally every frame — even on idle, hover, and scroll frames — allocating a brand-new `Vec<DrawItem>` and cloning every string/run/path/table on the current slide!
- **The fix:**
  1. **Bounded `TextLayoutCache` (256 entries, LRU, stored in `ctx.data` keyed by font epoch):** Keyed on 64-bit content hash of `(runs, paragraphs, quantized rect_w/h, quantized scale, align, valign, autofit, wrap, font_epoch)`. Returns `Arc<LaidText>` (positions offset by `rect.min` relative to `(0,0)` so moving a slide/thumbnail in scroll doesn't even miss the cache!). Also skip `dimmed_runs` clone in `paint.rs:929` when `!dimmed && opacity >= 1.0`.
  2. **Bounded `ShapeMeshCache` (256 entries, LRU, stored in `ctx.data`):** Keyed on `(ShapeKind, quantized_w_px, quantized_h_px, adjust_bits, fill/outline hash, ppp_bits)` storing local-origin `Arc<Mesh>`. Painting translates vertices by `rect.min` in a single tight loop (0 triangulation, 0 subdivision on hits!). Also add a fast-path for simple convex polygons (skip O(N³) ear clipping via simple fan triangulation when `is_convex(&points)` — O(N) instead of O(N³)!).
  3. **Main-canvas `CanvasComposeCache` on `UiState`:** Keyed on `(revision, slide_id, layer, extra_layers, base_slide_view, selected_marker_id, preview_state, editing_state, inline_edit_target, media_len)` storing `Arc<[DrawItem]>`. Cleared in `Editor::replace_course`. Idle, hover, and slide-list scroll frames do **0 main-canvas composes**!
- **Expected gain:**
  - **−40% to −60% frame CPU time** on complex slides (50 objects, shapes with gradients, `Autofit::Shrink` text boxes) in both editor AND learner player!
  - **20× faster** rendering of `Autofit::Shrink` text boxes on steady frames.
  - **0 canvas composes** on idle/scroll frames.
- **Risk:** Medium — stale cache entries if font definitions change or zoom/DPI changes. Mitigate by including a monotonic `font_epoch: u64` (bumped inside `set_fonts_and_track` in `font.rs`) and `pixels_per_point.to_bits()` in cache keys, and bounding both caches to 256 entries (~2–4 MB max).
- **Proving tests:**
  - Counter-based guard tests `G-05` (`crates/ui-egui/src/guard_tests.rs`): over 20 idle frames on `perf02_fifty_object_stateful_slide` and a gradient+autofit slide: `canvas_composes() == 0`, `text_layout_misses() == 0`, `shape_mesh_misses() == 0`.
  - Pixel/visual equivalence test `U-08` (`crates/render/src/tests.rs`): cached vs uncached `render_slide_rgba` produces **byte-identical RGBA pixels** on shapes, gradients, and `Autofit::Shrink` text.

---

### P8 — Web Player & Web Editor WASM Bundle Slimming (Fast Load on Slow Phones & Chromebooks)

- **Where:**
  - `apps/coursecraft-player/Cargo.toml:16` (remove `coursecraft-format` dependency!)
  - `apps/coursecraft-player/src/media.rs:38,276,287` (switch `coursecraft_format::is_safe_relative_path` → `coursecraft_course::is_safe_media_path`)
  - `apps/coursecraft/index.html:8` (add `data-cargo-profile="wasm-release"` — currently missing!)
  - `Cargo.toml:83-87` (`[profile.wasm-release]`: add `panic = "abort"`, `strip = "symbols"`)
  - `apps/coursecraft-player/index.html` & `apps/coursecraft/index.html` (add `data-wasm-opt="z"` when `wasm-opt` is available or document in `xtask`)
  - `apps/coursecraft-player/src/media.rs:72` (`MAX_WEB_CACHE_BYTES`: adaptive 48 MiB on mobile/narrow viewports vs 96 MiB desktop, down from 128 MiB)
- **What it costs today:**
  1. `coursecraft-player` links the ENTIRE `coursecraft-format` crate (`zip`, `quick-xml`, `xliff.rs` 2,052 lines) just to call one 15-line path sanitizer that ALREADY exists in `coursecraft-course`!
  2. `apps/coursecraft/index.html:8` omits `data-cargo-profile="wasm-release"`, so the web editor compiles with `release` (`lto = "thin"`, `opt-level = 3`) instead of `wasm-release` (`opt-level = "z"`, `lto = "fat"`, `codegen-units = 1`) — inflating web editor `.wasm` by **~25–40%**!
  3. `[profile.wasm-release]` omits `panic = "abort"` and `strip = "symbols"`, retaining unwinding tables and debug symbol names in shipped WASM.
  4. Combined with P3 (`epaint_default_fonts` ~1.5 MB), every published course currently forces a learner on a slow 3G/4G phone to download **~2+ MB of unnecessary WASM payload** before the first slide can even appear!
- **The fix:**
  1. Replace the 3 calls to `coursecraft_format::is_safe_relative_path` in `apps/coursecraft-player/src/media.rs` with `coursecraft_course::is_safe_media_path` (verify 100% behavioral parity between the two functions first — unify them in `coursecraft-course` and re-export from `coursecraft-format`!). Remove `coursecraft-format` from `apps/coursecraft-player/Cargo.toml`.
  2. Add `panic = "abort"` and `strip = "symbols"` to `[profile.wasm-release]` in root `Cargo.toml` (only affects `wasm-release`, never tests!).
  3. Add `data-cargo-profile="wasm-release"` to `apps/coursecraft/index.html:8`.
  4. Lower `MAX_WEB_CACHE_BYTES` in `apps/coursecraft-player/src/media.rs` to `64 * 1024 * 1024` (64 MiB default, `32 MiB` on mobile/Lite mode) so mobile Safari/Chrome tabs never get killed by iOS/Android jetsam on media-heavy courses.
  5. *(Optional stretch / future-safe)* Add `data-wasm-opt="z"` in `apps/coursecraft-player/index.html` and `apps/coursecraft/index.html` (Trunk downloads/runs `wasm-opt -Oz` automatically if configured, saving another **~10–18%** on `.wasm` size; verify offline/CI friendliness before enabling!).
- **Expected gain:**
  - **Published web player `.wasm`:** **−1.6 to −2.5 MB uncompressed** (**−30% to −45% smaller!**), **−0.8 to −1.3 MB gzipped** — cuts cold load time on a 3 Mbps mobile connection by **~2.5–4 seconds**!
  - **Web editor `.wasm`:** **−30% to −45% smaller** (Chromebook startup & memory win).
  - **Mobile player RAM:** **−64 to −96 MB** lower peak browser tab memory.
- **Risk:** Low — verify `is_safe_media_path` matches `is_safe_relative_path` edge-for-edge (especially `"` and control chars — `crates/format/src/lib.rs:176` rejects `"`!). Make `coursecraft-course::is_safe_media_path` the single canonical implementation and re-export it from `coursecraft-format`.
- **Proving tests:**
  - Unit test `U-09` (`crates/format/src/tests.rs`): `is_safe_relative_path` == `coursecraft_course::is_safe_media_path` across all hostile path vectors.
  - Dependency/size gate `W-02`: `coursecraft-player` has zero `coursecraft-format` / `epaint_default_fonts` dependencies; `target/player-dist/*.wasm` meets the M34 WASM size budget (§4).

---

### P9 — Eliminate O(R × M) Media Scans, Per-Frame Editor Allocations & Check Spelling 300-Slide Rescan

- **Where:**
  - `crates/ui-egui/src/lib.rs:1490,4138-4150,4946-4970` (`Editor.media`, `refresh_validate_only`, `MemoryMediaSource::get`), `crates/ui-egui/src/canvas.rs:5325` (`missing_images_as_placeholders`)
  - `crates/ui-egui/src/properties.rs:1676-1694` (`publish_theme_colors` allocates 2 `Vec`s + 10+ `String`s every frame!)
  - `crates/ui-egui/src/timeline_playback.rs:292-367` (`sync_slide` allocates `Vec<TrackSpec>` + `Vec<TimelineTrack>` every single frame!)
  - `crates/ui-egui/src/spell_check.rs:588-615` (`ignore_issue` / `add_issue_word` rescans all 300 slides!)
  - `crates/spell/src/lib.rs:140,170,185,255` (4 heap allocations per word in `Speller::misspellings`!)
  - `crates/ui-egui/src/slides.rs:60-102` (`thumbnail_fingerprint` runs `serde_json::to_writer` on every visible slide whenever revision bumps)
- **What it costs today:**
  - `check spelling, ignore step`: **9.55 ms / 11.55 ms** per click (measured!) because ignoring 1 word clears `spell_check.key` and rescans all 300 slides through Hunspell!
  - `check spelling, open`: **29.04 ms** (measured!) due to 4 per-word heap allocations in `coursecraft-spell`.
  - `refresh_validate_only`: O(R × M) quadratic scan (90,000 string comparisons on a 300-slide / 300-media course) on every single edit.
  - `publish_theme_colors` + `sync_slide`: ~15–20 unnecessary heap allocations on EVERY idle frame.
- **The fix:**
  1. **Indexed media lookup on `Editor`:** Keep a lightweight `media_index: HashMap<String, usize>` alongside `media: Vec<(String, Arc<[u8]>)>` (or wrap in `MediaStore` — see Refactor Proposal `R1`!), rebuilt/updated on `set_media`/`push`/`retain`. Turns `MemoryMediaSource::get`, `missing_images_as_placeholders`, `refresh_validate_only`, and `timeline_playback::sync_slide` into **O(1)** lookups!
  2. **Revision-gate `publish_theme_colors` & `timeline_playback::sync_slide`:** Skip both completely when `(revision, slide_id, media_len)` is unchanged (0 allocations on idle frames!).
  3. **In-place Check Spelling filtering (`spell_check.rs:588-615`):**
     - `ignore_issue(editor, index, false)` removes only the ignored `(location, start)` entry from `state.issues` in-place and updates `state.key = Some((rev, spell_version))` — **0 speller runs!**
     - `ignore_issue(editor, index, true)` and `add_issue_word(editor, index)` retain only issues whose `word` does not `eq_ignore_ascii_case(&word)` and update `state.key` — **0 speller runs!**
  4. **Zero-alloc word scanner in `crates/spell/src/lib.rs`:** Iterate `non_whitespace_runs` as an iterator (no `Vec`), scan chars directly with byte indices (no `Vec<(usize, char)>`), check `should_skip` with a counter/iterator (no `Vec<char>`), and only call `word.to_lowercase()` when `personal` is non-empty OR `word` contains an uppercase char!
  5. **Faster `thumbnail_fingerprint` (`slides.rs`):** Pass `Session::last_changed_slide() -> Option<&str>` (already known from M33's `CourseDelta::SlideChange`!) so when a single slide changes, `ThumbnailCache` bumps `verified_revision` on untouched visible slides in O(1) without even hashing them!
- **Expected gain:**
  - `check spelling, ignore step`: **9.55 ms → ≤ 0.25 ms** (**~38× faster!**).
  - `check spelling, open`: **29.04 ms → ≤ 14.0 ms** (**~2× faster!**).
  - `refresh_validate_only` missing-media check on 300-slide course: **O(R × M) → O(R)** (~300× fewer comparisons).
  - Idle frame allocations: **−15 to −25 allocations/frame**.
- **Risk:** Very low — purely local optimizations with exact behavioral parity.
- **Proving tests:**
  - Guard test `G-06` (`crates/ui-egui/src/guard_tests.rs`): `check_spelling_ignore_runs_zero_speller_checks` (`gc::speller_runs() == 0` on Ignore, Ignore All, and Add to Dictionary!).
  - Unit tests in `crates/spell/src/tests.rs` (all existing spell tests pass identically) + `PF-05` (`check spelling, open` & `ignore step` in `perf_tests.rs`).

---

### P10 — CLI `validate` Lazy System-Font Scan (13× Faster CLI!) & Batch Reuse

- **Where:**
  - `apps/coursecraft-cli/src/lib.rs:1914-1918` (`Cmd::Validate`)
  - `apps/coursecraft-cli/src/batch.rs` (batch processing across many courses)
- **What it costs today (measured!):** `coursecraft-cli validate examples/hello-course/course.json` takes **100.51 ms** (vs `7.06 ms` for `simulate`!) because it unconditionally scans the entire OS font directory (`SystemFontIndex::scan_default()` ≈ 93 ms) even when the course only uses bundled `Inter`!
- **The fix:**
  1. Call `coursecraft_publish::collect_used_font_faces(&c)` first (~0.02 ms), filter out faces already present in `c.fonts`, and **only** call `SystemFontIndex::scan_default()` when at least one unembedded non-Inter font face is actually referenced!
  2. In `batch.rs`, initialize `player_files` once outside the per-course loop (if not already) and share a lazy `OnceCell<SystemFontIndex>` across all courses in the batch.
- **Expected gain:** `coursecraft-cli validate` drops from **100.5 ms → ~7.5 ms** (**~13× faster!**) for every Inter/self-contained course (huge speedup for CI scripts, AI agents, and `batch` runs!).
- **Risk:** Negligible — exact same warnings emitted whenever a course references a system font.
- **Proving tests:**
  - Unit test `U-10` (`apps/coursecraft-cli/src/tests.rs`): `validate` on Inter-only course skips font scan; `validate` on course naming an unknown system font still reports the exact same font warning.

---

### P11 — Low-End "Lite / Performance" Mode & Hard Memory Ceilings for Huge Courses

- **Where:**
  - `crates/render/src/media.rs`, `crates/render/src/paint.rs` (`MAX_EFFECT_LAYERS`, `GRADIENT_MAX_EDGE`), `crates/render/src/design.rs`
  - `crates/ui-egui/src/preferences.rs`, `crates/ui-egui/src/lib.rs`, `apps/coursecraft/src/prefs.rs`
  - `apps/coursecraft-player/src/load.rs` (`?lite=1` query flag + automatic mobile heuristic), `apps/coursecraft-player/src/app.rs`
- **What it costs today:** Low-end machines (4 GB RAM dual-core laptop with integrated graphics, old Intel Mac, cheap Android phone) run the exact same texture budgets (512 MB / 256 MB), 6-pass shadow/glow blur approximations (`MAX_EFFECT_LAYERS = 6`), dense 12 pt gradient meshes, 2× HiDPI thumbnail rasterisation, and continuous marker pulses as a 24 GB Apple M5 desktop.
- **The fix:**
  1. Introduce `PerformanceMode { Auto, Standard, Lite }` in `coursecraft-render` (L3, pure enum + `PerfProfile` struct of concrete limits) so BOTH editor and learner player share one clean definition:
     | Setting | `Standard` (Desktop) | `Standard` (WASM) | `Lite` (Desktop / WASM / Mobile) |
     |---|---:|---:|---:|
     | Unified `MediaCache` budget | 256 MB | 96 MB | **64 MB** |
     | Raster decode tier cap | 2048 px (canvas) / 384 px (thumb) | 1536 px | **1024 px (canvas) / 256 px (thumb)** |
     | `ThumbnailCache` entries | 128 | 64 | **32** |
     | `TextLayoutCache` / `ShapeMeshCache` | 256 / 256 | 128 / 128 | **96 / 96** |
     | Shadow/glow `MAX_EFFECT_LAYERS` | 6 | 4 | **2** |
     | Gradient `GRADIENT_MAX_EDGE` | 12.0 pt | 16.0 pt | **24.0 pt** (~4× fewer triangles!) |
     | Marker pulse animations | On (unless reduced motion) | On (unless reduced motion) | **Off** (saves continuous repaints!) |
     | `WebMediaSource` raw cache cap | — | 64 MiB | **32 MiB** |
  2. **`Auto` detection:**
     - Web player: enters `Lite` automatically when `?lite=1`, when `prefers-reduced-motion: reduce`, or when `navigator.deviceMemory <= 4` (or viewport width `<= 640px` touch phone).
     - Desktop/Web editor: selectable in **Preferences** (`Performance: Automatic / Standard / Lite`); `Auto` activates `Lite` automatically on WASM (`deviceMemory <= 4`) or when an open course exceeds **150 slides** or **256 MB raw media**.
  3. **Memory pressure status warning:** When raw `Editor.media` total bytes exceed `512 MiB` (desktop) or `128 MiB` (web editor), show a one-time non-blocking status hint suggesting `.ccraft-folder` or Lite mode (never blocks editing or crashes).
- **Expected gain:** On a cheap phone or 4 GB laptop with a heavy course, **−60% to −75% GPU/RAM footprint**, **~3–4× fewer gradient/effect triangles**, and smooth 60 fps scrolling/interactions without thermal throttling.
- **Risk:** Low — visual fidelity in `Standard` mode is 100% unchanged; `Lite` mode is strictly opt-in or auto-triggered on constrained devices/huge courses and keeps full functional/trigger parity (`coursecraft-logic` untouched!).
- **Proving tests:**
  - Unit tests `U-11` (`crates/render/src/tests.rs` & `apps/coursecraft-player/src/tests.rs`): `PerfProfile::for_mode(Lite)` enforces every cap, `?lite=1` activates Lite mode, and `paint` produces valid output with reduced triangle count on gradient/shadow shapes.

---

## 4. Low-End Hardware Performance Budgets & Deterministic Simulation

### 4.1 Target Hardware Tiers & Budgets

| Metric | Reference Dev Machine (Apple M5, release headless) | Low-End Desktop Budget (4 GB RAM, dual-core i5/Celeron, iGPU) | Chromebook Web Editor Budget (4 GB RAM, WASM) | Cheap Phone Web Player Budget (2 GB RAM, 3G/4G) |
|---|---:|---:|---:|---:|
| **Idle CPU (editor, settled)** | 0 repaints, ≤ 0.25 ms if triggered | **0% CPU** (0 repaints) | **0% CPU** (0 repaints) | — |
| **Idle CPU (player, static/paused/start/ended)** | 0 repaints | **0% CPU** | **0% CPU** | **0% CPU (0 fps!)** |
| **Editor select / scroll / type (300 slides)** | ≤ 0.40 ms median / ≤ 0.55 ms p95 | ≤ 4 ms median / ≤ 8 ms p95 | ≤ 6 ms median / ≤ 12 ms p95 | — |
| **Single-slide edit (`perf300` / `rich300`)** | ≤ 0.08 ms / ≤ 0.25 ms | ≤ 0.5 ms / ≤ 1.5 ms | ≤ 1.0 ms / ≤ 3.0 ms | — |
| **Undo / redo move (30-cycle frame loop)** | ≤ 2.5 ms median / ≤ 3.2 ms p95 | ≤ 16 ms | ≤ 20 ms | — |
| **Check Spelling ignore step (300 slides)** | ≤ 0.40 ms | ≤ 2 ms | ≤ 4 ms | — |
| **Autosave UI-thread stall (1.3 GB project)** | ≤ 2 ms | **≤ 5 ms** (background thread) | — | — |
| **MediaCache total texture + anim CPU bytes** | ≤ 256 MB (Std) / ≤ 64 MB (Lite) | ≤ 256 MB (Std) / **≤ 64 MB (Lite)** | ≤ 96 MB (Std) / **≤ 64 MB (Lite)** | **≤ 64 MB (Lite)** |
| **Peak RAM on Open `.ccraft` (1.36 GB media)** | ≤ 1,650 MB (~1.2× media) | ≤ 1.25× media (fits 4 GB for ≤ 1.5 GB proj) | — | — |
| **Peak RAM on Publish (1.36 GB media)** | ≤ 1,650 MB (~1.2× media) | ≤ 1.25× media | ≤ 2.0× small/med project | — |
| **Published Web Player `.wasm` (gzip / raw)** | — | — | — | **≤ 1.5 MB gz / ≤ 4.5 MB raw** *(verify baseline in S1 and tighten if already smaller!)* |
| **Published `course.json` overhead vs compact** | 0% (compact) | — | — | **0% pretty-print bloat** |
| **CLI `validate` on Inter-only course** | ≤ 10 ms | ≤ 25 ms | — | — |

### 4.2 How We Simulate Low-End Hardware Deterministically in Tests (100% Safe Rust)
Because CI runners are fast multi-core machines and `unsafe_code = "forbid"` prohibits `unsafe impl GlobalAlloc`, we simulate low-end constraints **deterministically without flakiness**:
1. **Tracked byte budgets on every cache (`MediaCache::total_bytes()`, `ThumbnailCache::estimated_bytes()`, `TextLayoutCache::estimated_bytes()`, `ShapeMeshCache::estimated_bytes()`, `Session::undo_estimated_bytes()`):** Low-end tests instantiate `Editor` and `Playing` with `PerformanceMode::Lite` (64 MB texture cap, 32-entry thumbnail cap, 32 MiB web cache cap) and assert exact hard ceilings after walking 300 slides with real 1280×720 PNGs, SVGs, and animated GIFs.
2. **Work counters (`guard_counters` + `coursecraft_render::test_counts` + `coursecraft_engine::test_counts`):** Compiled under `#[cfg(debug_assertions)]` (zero release cost!). Count exact operations that kill dual-core CPUs:
   - `font_def_clones()` (must be **0**)
   - `canvas_composes()` (must be **0** on idle/scroll)
   - `text_layout_misses()` (must be **0** on idle)
   - `shape_mesh_misses()` (must be **0** on idle)
   - `picture_decodes()` & `full_res_thumbnail_decodes()` (must be **0** when thumbnails decode at thumbnail tier!)
   - `full_validations()` (must be **0** on `SingleSlide` geometry/style edits & no-ops!)
   - `speller_runs()` (must be **0** on Check Spelling Ignore / Ignore All / Add!)
3. **4×–6× CPU headroom multiplier on release benchmarks (`perf_tests.rs` & `undo_perf_tests.rs`):** Every M34 release budget on Apple M5 is set **4×–6× tighter** than a 60 fps (16 ms) / 30 fps (33 ms) low-end target so passing on M5 mathematically guarantees smooth performance on a 4×–6× slower dual-core laptop or mobile CPU.
4. **Reliable OS RSS reporting in `perf_tests.rs`:** Replace the sandbox-failing `ps` subprocess with reading `/proc/self/statm` on Linux and reporting deterministic `media_cache.total_bytes()` + `thumbnail_cache.estimated_bytes()` + `undo_estimated_bytes()` on all platforms (so macOS/Windows/Linux/CI all emit real byte numbers even when `ps` is blocked!).

---

## 5. Normative Rules (`M34-01`–`M34-14`)

| Rule ID | Category | Normative Requirement |
|---|---|---|
| **M34-01** | Player & Preview Idle CPU | When a course is on the start gate, paused, ended, or past its slide/layer timelines with no active timers/animations/media, `Player::next_wake_ms()` returns `None` and neither `Playing` nor `Preview` requests an immediate repaint. |
| **M34-02** | Runtime Wake Exactness | Whenever a timeline, entrance/exit animation, motion path, cue point, `timeline_reaches` trigger, countdown timer, Lottie, or video is active, `Player::next_wake_ms()` returns `Some(delay)` so every trigger/animation fires on time without drift. |
| **M34-03** | Display-Tier Raster Decode | Painting a raster image solely inside a slide thumbnail (`<= 384 px`) or media-library preview decodes at thumbnail tier (`<= 384 px` longest side) while preserving exact natural `[w, h]` for aspect-fit and crop; selecting the slide on the canvas upgrades seamlessly without thrashing. |
| **M34-04** | Texture Deduplication & Unified Budget | Identical image bytes under distinct `src` keys share one GPU texture per tier; `MediaCache` accounts for pictures, SVG rasters, and animation CPU frames under one O(1)-tracked byte ceiling (`256 MB` desktop / `96 MB` wasm / `64 MB` Lite). |
| **M34-05** | Zero `FontDefinitions` Clones | `slide_font_named` / `family_exists` performs **0** `FontDefinitions` clones on steady frames; `install_course_fonts` skips work when `(revision, media_len)` is unchanged. |
| **M34-06** | Lean Font Bundle | Workspace `egui`/`eframe` drop `epaint_default_fonts` (`default_fonts` disabled); `coursecraft-player` carries zero Ubuntu/Hack/NotoEmoji bytes while rendering Proportional, Bold, Italic, Monospace, and Phosphor icons cleanly. |
| **M34-07** | Fast Scoped Validation | `Course::validate()` allocates **0** path `String`s on valid elements; `SingleSlide` geometry/style edits and no-ops run **0** full-course validations; `Editor::check_revision` reuses `Session::validation_issues()` with **0** redundant `validate()` calls and **0** JSON round-trips. |
| **M34-08** | Zero-Copy Open & Publish | Desktop `.ccraft` Open streams from `BufReader<File>` via `open_project_from` (never `std::fs::read` whole archive); Publish shares `Arc<[u8]>` media bytes with **0** per-asset `.to_vec()` copies and emits compact `course.json`. |
| **M34-09** | Non-Blocking Autosave | Desktop autosave writes and verifies on a background thread with **0** media byte copies and an atomic `save_epoch` guard so a completed autosave never overwrites or outlives a newer manual Save. |
| **M34-10** | Render & Canvas Caching | Bounded LRU `TextLayoutCache`, `ShapeMeshCache`, and `CanvasComposeCache` deliver **0** canvas composes, **0** text-layout misses, and **0** shape-mesh misses on idle frames, with byte-identical pixel output. |
| **M34-11** | Lean WASM Bundles | `coursecraft-player` drops `coursecraft-format` (using canonical `coursecraft_course::is_safe_media_path`); `apps/coursecraft/index.html` uses `data-cargo-profile="wasm-release"`; `[profile.wasm-release]` sets `panic = "abort"` and `strip = "symbols"`. |
| **M34-12** | O(1) Media Index & Fast Spelling | `Editor` resolves media by O(1) index (no O(R × M) scans); Check Spelling `Ignore` / `Ignore all` / `Add` performs **0** speller runs; `coursecraft-spell` eliminates per-word `Vec`/`String` allocations. |
| **M34-13** | Fast CLI `validate` | `coursecraft-cli validate` scans OS fonts only when the course references an unembedded non-Inter font (`≤ 10 ms` on Inter-only example courses). |
| **M34-14** | Low-End Lite Mode | `PerformanceMode` (`Auto` / `Standard` / `Lite`) enforces Lite budgets (64 MB texture cap, 32 thumbs, 2 effect layers, 24 pt gradient edge, no pulse) cleanly across desktop editor, web editor, and web player (`?lite=1`). |

---

## 6. Watchlist of Risky Spots (`RISK-01`–`RISK-10`)

| ID | File & Lines | Risk & Mitigation |
|---|---|---|
| **RISK-01** | `crates/logic/src/lib.rs`, `apps/coursecraft-player/src/app.rs:1083`, `crates/ui-egui/src/preview.rs:840` | Missing a wake condition in `Player::next_wake_ms()` could stall a timer, animated GIF, Lottie, video caption, hover transition, or seekbar. **Mitigation:** Cover every time-dependent state (`base_clock`, visible `layer_clocks`, `active_timers`, `active_motion_paths`, active entrance/exit windows, playing `media_states`, `View::transition_active`, `lightbox_opened_at`, `var_changed_at`, pending web fetches) with dedicated unit tests, and always wake immediately on any pointer/keyboard/DOM event (egui already repaints on input!). |
| **RISK-02** | `crates/render/src/media.rs:574-607` & `crates/render/src/paint.rs:45` | Returning downscaled texture pixel dimensions as `MediaView::Ready { size }` would break `image_dest_rect` aspect ratio and crop math if rounding slightly alters `w/h`! **Mitigation:** Always return the **original natural `[natural_w, natural_h]`** in `MediaView::Ready { size }`, and track `uploaded_bytes = decoded_w * decoded_h * 4` separately for the byte budget. |
| **RISK-03** | `crates/render/src/media.rs` (thumbnail vs canvas tier thrashing) | If slide 0 is visible in BOTH the slide list (`186×105`) AND the main canvas (`800×450`) on the same frame, a naive tier cache could decode twice or thrash. **Mitigation:** A higher-resolution cached tier (`Canvas`/`Full`) ALWAYS satisfies lower-resolution (`Thumbnail`) requests in O(1) without decoding a second copy! Only decode a thumbnail tier when no larger tier is cached. |
| **RISK-04** | `Cargo.toml:45-46` (removing `default_fonts`) | Tests that construct a raw `egui::Context::default()` without calling `coursecraft_render::install_fonts(&ctx)` will have empty fonts and panic/render blank inside egui layout! **Mitigation:** Audit every `egui::Context::default()` in tests across `render`, `ui-egui`, `player`, and `export` (or provide a `render` test helper / keep `install_fonts` idempotent inside `paint` / `TestHarness::new`). Also check if any test renders Unicode symbols outside Inter + Phosphor (e.g., `→`, `←`, `…`, `•`, `▪`, `▸`, `⌘`, `⇧`, `⌥`, `✓` — verify Inter covers all UI glyphs used in `menu.rs`, `shortcuts.rs`, `player_frame.rs`!). |
| **RISK-05** | `crates/engine/src/commands.rs` (scoped validation soundness) | A `SingleSlide` command could theoretically break a course-wide invariant (e.g., removing the last `complete_course` trigger changes a warning, or setting an invalid jump target `SlideTarget::Id("missing")`). **Mitigation:** Only use single-slide validation for commands that pass `is_local_slide_edit` (or validate slide + course-level ID lookups against `&Course` — `Course::validate_slide_errors(loc)` checks jump targets against `course.play_slide()` in O(1)/O(slides) without re-validating the other 299 slides!). Verify 100% parity with full `Course::validate()` in `U-05`! |
| **RISK-06** | `crates/publish/src/lib.rs` (compact `course.json` in published packages) | Existing tests in `crates/publish/src/tests.rs` or `apps/coursecraft-cli/tests/` may assert exact byte equality between `course.to_json()` and published `course.json`. **Mitigation:** Audit `crates/publish/src/tests.rs`, `apps/coursecraft/src/publish.rs`, and `apps/coursecraft-cli/tests/` in S7 and update assertions to expect `course.to_json_compact()` (still parsing to an identical `Course`). |
| **RISK-07** | `apps/coursecraft/src/autosave.rs` (background autosave race with manual Save or Quit) | If background autosave finishes 50 ms *after* manual `Save` deletes `.autosave.ccraft`, it would leave an autosave newer than the saved file and falsely trigger "Restore unsaved changes?" on next open! **Mitigation:** Pass `save_epoch: Arc<AtomicU64>` into the background job; bump it on Save/Open/New; re-check epoch inside `write_and_swap` right before `rename(tmp, autosave_path)` — if stale, delete `tmp` and exit cleanly. |
| **RISK-08** | `crates/render/src/text_layout.rs` & `paint.rs` (cache key collisions or missed invalidations) | Hashing floating-point rects or colors poorly could cause stale text/shapes on zoom, theme change, or custom font load. **Mitigation:** Normalize `-0.0` → `0.0` before `to_bits()`, include `font_epoch` (from `font.rs`), `pixels_per_point.to_bits()`, and full run/paragraph/fill values (using 128-bit hash or exact key struct equality on hit!), and bound cache size to 256 entries. |
| **RISK-09** | `crates/course/src/model.rs` (`is_safe_media_path` vs `coursecraft_format::is_safe_relative_path`) | `format::is_safe_relative_path` rejects `"` (`crates/format/src/lib.rs:176`) and control chars; verify `course::is_safe_media_path` has identical rules before removing `coursecraft-format` from `coursecraft-player`. **Mitigation:** Make `course::is_safe_media_path` the single strict implementation and have `format::is_safe_relative_path` re-export it directly. |
| **RISK-10** | `crates/render/src/shape_fill.rs` (convex fast-path on degenerate polygons) | A nearly-collinear or self-intersecting hostile polygon might falsely look convex under loose float tolerance. **Mitigation:** Require strict consistent cross-product sign (`> EPS` or `< -EPS`) across ALL vertices AND total turning angle `≈ 2π` before taking the O(N) fan path; otherwise fall back to existing ear clipping. |

---

## 7. Concrete Test Plan

### 7.1 Counter-Based Guard Tests (`G-01`–`G-07`) — Run in Normal `cargo test` (Zero Timing Flakes)

| ID | Crate / File | Behaviour Proved | Exact Assertion |
|---|---|---|---|
| **G-01** | `crates/ui-egui/src/idle_tests.rs` & `apps/coursecraft-player/src/tests.rs` | Idle repaint silence in Preview & Player (`M34-01`) | After timeline ends (or when paused / on start gate / ended), 10 settling frames + 1 idle frame produce `requested_repaint_last_pass() == false` and `repaint_delay >= 1s` (`Duration::MAX`). |
| **G-02** | `crates/ui-egui/src/guard_tests.rs` | Thumbnail-tier raster decoding & low-end texture ceiling (`M34-03`, `M34-04`) | Scrolling a 100-slide course with real `1280×720` PNGs keeps `media_cache.total_bytes() <= 8 MiB` and `full_res_thumbnail_decodes() == 0`; switching to slide `s-50` upgrades only `s-50`'s image to canvas tier. |
| **G-03** | `crates/ui-egui/src/guard_tests.rs` | Zero `FontDefinitions` clones on steady frames (`M34-05`) | 20 idle frames + 20 scroll frames on `hello-course` and a custom-font course produce `font_def_clones() == 0`. |
| **G-04** | `crates/publish/src/tests.rs` | Zero-copy shared media in Publish + compact `course.json` (`M34-08`) | `build_package_shared` returns `Arc::ptr_eq` buffers matching input media (`0` copies), and published `course.json` has zero pretty-print indentation newlines and round-trips to an identical `Course`. |
| **G-05** | `crates/ui-egui/src/guard_tests.rs` | Canvas compose, text layout & shape mesh cache hit rates (`M34-10`) | 20 idle frames on a 50-object slide with `Autofit::Shrink` text and gradient shapes produce `canvas_composes() == 0`, `text_layout_misses() == 0`, `shape_mesh_misses() == 0`. |
| **G-06** | `crates/ui-egui/src/guard_tests.rs` | Check Spelling Ignore/Add does zero rescan (`M34-12`) | Clicking Ignore, Ignore all, and Add to dictionary in Check Spelling on a 100-slide course produces `gc::speller_runs() == 0`. |
| **G-07** | `crates/ui-egui/src/guard_tests.rs` & `crates/engine/src/tests.rs` | Zero full-course validations on single-slide nudge & no-op (`M34-07`) | Arrow nudge and no-op `object.set_frame` on `perf300` produce `full_course_validations() == 0` and `gc::validation_runs() == 1` (using cached engine issues, no double validate). |

### 7.2 Unit, Correctness & Differential Tests (`U-01`–`U-11`, `W-01`–`W-02`)

| ID | Crate / File | Behaviour Proved |
|---|---|---|
| **W-01** | `crates/logic/src/tests.rs` | `Player::next_wake_ms()` exactness across start gate, paused, ended, finished slide, entrance/exit window, motion path, cue point, `timeline_reaches`, countdown timer, layer clock, Lottie, and playing video. |
| **U-01** | `crates/media/src/tests.rs` | `decode_image_fitted` downscales oversized PNG/JPEG/WebP cleanly, preserves exact natural `[w, h]`, and rejects hostile inputs with 0 panics. |
| **U-02** | `crates/render/src/media.rs` | Content-hash dedup shares one `TextureId`, canvas tier satisfies thumbnail requests (no thrash), unified ceiling evicts cold pictures, SVG rasters, and animation CPU frames cleanly. |
| **U-03** | `crates/render/src/font.rs` | `base_definitions()` without `epaint_default_fonts` renders Proportional, Bold, Italic, Monospace, and all UI glyphs (`→`, `←`, `…`, `•`, `▸`, `⌘`, `⇧`, `⌥`, Phosphor icons) with 0 missing-glyph boxes. |
| **U-04** | `crates/course/src/tests.rs` | Lazy-path `Course::validate()` emits byte-for-byte identical `Vec<Issue>` paths/messages as before; `Course::first_error()` matches `validate().into_iter().find(Error)`. |
| **U-05** | `crates/engine/src/m27_all_commands_undo_tests.rs` | Scoped validation matches full `Course::validate()` across all 186 commands + `panic_hunt` + hostile invalid jump targets. |
| **U-06** | `crates/format/src/tests.rs` | `open_project_from<R: Read + Seek>` matches `open_project` identically and enforces all zip-bomb / path / duplicate limits. |
| **U-07** | `apps/coursecraft/src/autosave.rs` | Background autosave writes verifiable `.ccraft`, coalesces overlapping requests, and aborts rename when `save_epoch` advances. |
| **U-08** | `crates/render/src/tests.rs` | Cached `TextLayoutCache` and `ShapeMeshCache` (plus convex fast-path) produce byte-identical RGBA renders vs uncached across zoom/DPI/font changes. |
| **U-09** | `crates/format/src/tests.rs` | Unified `is_safe_relative_path` == `coursecraft_course::is_safe_media_path` across all hostile path test cases. |
| **U-10** | `apps/coursecraft-cli/src/tests.rs` | CLI `validate` skips OS font scan for Inter-only courses and preserves warnings for unembedded system fonts. |
| **U-11** | `crates/render/src/tests.rs` & `apps/coursecraft-player/src/tests.rs` | `PerformanceMode::Lite` enforces 64 MB cap, 2 effect layers, 24 pt gradient edge, no pulse; `?lite=1` activates Lite mode. |
| **W-02** | Build/CI check (`cargo tree` + `target/player-dist`) | `coursecraft-player` has zero `coursecraft-format` and zero `epaint_default_fonts`; player `.wasm` meets size budget (`≤ 1.5 MB` gz / `≤ 4.5 MB` raw). |

### 7.3 Release Performance & Memory Tests (`PF-01`–`PF-06`)

| ID | Suite / Command | Budget (Apple M5 Release) |
|---|---|---|
| **PF-01** | `coursecraft-engine::undo_perf_tests::pf_01` (`perf300` 200 edits) | Total `≤ 20 ms`, edit median `≤ 0.08 ms`, undo/redo `≤ 0.05 ms`, `undo_estimated_bytes <= 512 KiB` |
| **PF-02** | `coursecraft-engine::undo_perf_tests::pf_02` (`rich300` 200 edits) | Total `≤ 60 ms`, edit median `≤ 0.25 ms`, `undo_estimated_bytes <= 2 MiB` |
| **PF-03** | `coursecraft-engine::undo_perf_tests::pf_03` (`perf300` 200 no-ops) | Total `≤ 5 ms`, no-op median `≤ 0.02 ms`, `0` bytes |
| **PF-04** | `coursecraft-ui-egui::perf_tests` (`idle`, `select`, `scroll`, `typing`, `undo/redo move`, `check spelling ignore`) | `idle ≤ 0.25 ms`, `scroll ≤ 0.25 ms`, `undo/redo move ≤ 2.5 ms`, `check spelling ignore ≤ 0.40 ms`, `visit every slide real pictures ≤ 1.5 ms` & `total_bytes <= 128 MB` |
| **PF-05** | `coursecraft-ui-egui::perf_tests::low_end_lite_mode_300_slides` (new!) | 300 slides with real 1280×720 PNGs + gradient shapes + `Autofit::Shrink` in `PerformanceMode::Lite`: `media_cache.total_bytes() <= 64 MiB`, median frame `≤ 1.5 ms` |
| **PF-06** | `apps/coursecraft-cli/tests/big_course.rs` (`BIG_STAGE=open` / `publish`) | Open peak RSS `≤ 1,700 MB` (down from 2,735 MB), Publish peak RSS `≤ 1,700 MB` (down from 2,756 MB) |

---

## 8. Numbered Implementer Steps (`S1`–`S12`, Small & Self-Contained)

> **Execution guidance:** Every step is scoped so a single implementer pass can complete, test, and verify it cleanly in one go. Run targeted `cargo test -p <crate> -j 4` after every step. Run full `cargo xtask ci` after **S3, S6, S9, and S12**. Run `cargo xtask build-player` + release perf tests at **S1** (confirm baseline) and **S12** (prove all gains).

- **S1 — Telemetry Counters, Reliable Cache Byte Reporting & Low-End Perf Benchmark**
  - **Title:** Add debug-only M34 guard counters, cache byte inspection APIs, and low-end release benchmark.
  - **Files:** `crates/render/src/media.rs`, `crates/render/src/font.rs`, `crates/ui-egui/src/guard_counters.rs`, `crates/ui-egui/src/perf_tests.rs`, `crates/engine/src/test_counts.rs`.
  - **Exact changes:**
    1. Add `#[cfg(debug_assertions)]` counters: `font_def_clones()`, `text_layout_misses()`, `shape_mesh_misses()`, `full_res_thumbnail_decodes()` in `coursecraft-render`; `full_course_validations()` in `coursecraft-engine`; `canvas_composes()` in `crates/ui-egui/src/guard_counters.rs`.
    2. Expose `pub fn total_bytes(&self) -> u64` on `MediaCache` (summing still pictures, uploaded animation frames, SVG rasters, and CPU animation buffers).
    3. Fix `perf_tests.rs` reporting to print `media_cache.total_bytes()` alongside OS RSS (adding `/proc/self/statm` on Linux so sandboxed runs never print `0 -> 0 MB` without real memory metrics) and add ignored release benchmark `low_end_lite_mode_300_slides` (`PF-05`) with real 1280×720 PNGs, gradient shapes, and `Autofit::Shrink` text.
  - **Tests:** `cargo test -p coursecraft-ui-egui -j 4 guard_tests` and baseline capture of `PF-04` / `PF-05`.
  - **Done when:** All new counters compile out in release builds, existing guard tests pass, and baseline numbers are recorded.

- **S2 — Fix `FontDefinitions` Per-Frame Clone Bug & Revision-Gate `install_course_fonts`**
  - **Title:** Eliminate per-text-run `FontDefinitions` cloning (`P3` part 1).
  - **Files:** `crates/render/src/font.rs`, `crates/ui-egui/src/guard_tests.rs`.
  - **Exact changes:**
    1. Track an `Arc<HashSet<FontFamily>>` + monotonic `font_epoch: u64` in `ctx.data` updated inside `set_fonts_and_track`; store `PENDING_DEFS_ID` as `(u64, Arc<FontDefinitions>)` and clear stale pass entries when `saved_pass != pass`.
    2. Rewrite `family_exists(ctx, family)` to check the `Arc<HashSet<FontFamily>>` in O(1) with **0** `FontDefinitions` clones.
    3. Add a fast revision/len guard to `install_course_fonts` so unchanged courses skip building `FontFingerprint`.
    4. Add guard test `G-03`.
  - **Tests:** `G-03`, `cargo test -p coursecraft-render -j 4 font`, `cargo test -p coursecraft-ui-egui -j 4 guard_tests`.
  - **Done when:** `G-03` proves `font_def_clones() == 0` across 20 idle and scroll frames.

- **S3 — Strip `epaint_default_fonts` & Unify Safe Path Helper to Drop `coursecraft-format` from Player**
  - **Title:** Cut ~1.6–2.0 MB from WASM player & web editor (`P3` part 2 + `P8` + Refactor `R9`).
  - **Files:** `Cargo.toml`, `crates/course/src/model.rs`, `crates/format/src/lib.rs`, `crates/render/src/font.rs`, `apps/coursecraft-player/Cargo.toml`, `apps/coursecraft-player/src/media.rs`, `apps/coursecraft/index.html`, `crates/format/src/tests.rs`, `crates/render/src/tests.rs`.
  - **Exact changes:**
    1. Make `coursecraft_course::is_safe_media_path` match `coursecraft_format::is_safe_relative_path` identically (including `"` and control chars) and re-export it from `coursecraft-format`; switch the 3 calls in `apps/coursecraft-player/src/media.rs` to `coursecraft_course::is_safe_media_path` and remove `coursecraft-format` from `apps/coursecraft-player/Cargo.toml`.
    2. Disable `default_fonts` on workspace `egui` and `eframe` in `Cargo.toml`; start `base_definitions()` in `crates/render/src/font.rs` from `FontDefinitions::empty()`, mapping `Proportional` → Inter + Phosphor and `Monospace` → Inter (or a tiny OFL ASCII monospace subset in `crates/render/fonts/` with licence + `ATTRIBUTION.md`); ensure any test using a bare `egui::Context::default()` installs fonts.
    3. Add `panic = "abort"` and `strip = "symbols"` to `[profile.wasm-release]` in `Cargo.toml`, and add `data-cargo-profile="wasm-release"` to `apps/coursecraft/index.html`.
    4. Lower `MAX_WEB_CACHE_BYTES` in `apps/coursecraft-player/src/media.rs` to `64 * 1024 * 1024` (with `32 MiB` in Lite mode).
    5. Add `U-03`, `U-09`, verify `W-02`.
  - **Tests:** `U-03`, `U-09`, `W-02`.
  - **Gate #1:** `cargo xtask ci` (all workspace tests, clippy, layers, wasm).
  - **Done when:** Zero `epaint_default_fonts` and zero `coursecraft-format` in `coursecraft-player`, all UI/render/font tests green.

- **S4 — Runtime `Player::next_wake_ms()` & Event-Driven Repaints in Player and Preview**
  - **Title:** Stop 60 fps idle CPU/battery burn in learner player and editor preview (`P1`).
  - **Files:** `crates/logic/src/lib.rs`, `crates/logic/src/run.rs`, `apps/coursecraft-player/src/app.rs`, `crates/ui-egui/src/preview.rs`, `crates/logic/src/tests.rs`, `crates/ui-egui/src/idle_tests.rs`, `apps/coursecraft-player/src/tests.rs`.
  - **Exact changes:**
    1. Implement `pub fn next_wake_ms(&self) -> Option<u32>` on `Player` in `crates/logic/src/lib.rs` (handling start/ended/paused, base & visible layer clocks, entrance/exit windows, motion paths, cue points, `timeline_reaches`, countdown timers, playing media/Lottie, and seekbar cadence).
    2. Replace unconditional `ui.ctx().request_repaint()` in `Playing::show_ui` (`apps/coursecraft-player/src/app.rs:1083`) and `Preview::show` (`crates/ui-egui/src/preview.rs:840`) with wake scheduling driven by `player.next_wake_ms()`, active view transitions/fades, pending web fetches, and active variable highlights.
    3. In `Preview::show`, replace per-frame `self.state.clone()` with `self.state.take()` moved back at frame end.
    4. Add `W-01` and `G-01`.
  - **Tests:** `W-01`, `G-01`, `cargo test -p coursecraft-logic -j 4`, `cargo test -p coursecraft-player -j 4`, `cargo test -p coursecraft-ui-egui -j 4 idle_tests preview`.
  - **Done when:** Both Preview and Player schedule **0** repaints when static/paused/ended/start-gate while all playback/animation/timer tests pass identically.

- **S5 — Display-Tier Raster Decoding, Content-Hash Dedup & O(1) Unified `MediaCache`**
  - **Title:** Slash GPU/VRAM usage 10–30× on thumbnails and multi-slide courses (`P2`).
  - **Files:** `crates/media/src/lib.rs`, `crates/media/src/tests.rs`, `crates/render/src/media.rs`, `crates/render/src/paint.rs`, `crates/ui-egui/src/guard_tests.rs`.
  - **Exact changes:**
    1. Add `decode_image_fitted(bytes, max_dim: Option<(u32, u32)>, limits) -> Result<(Rgba8Image, [u32; 2]), MediaError>` in `crates/media/src/lib.rs` using fast triangle downscaling only when source exceeds target tier (`Thumbnail <= 384`, `Medium <= 1024`, `Canvas <= 2048`).
    2. Upgrade `MediaCache` (`crates/render/src/media.rs`):
       - Quantize `target_px` into tiers; allow any cached larger tier to satisfy a smaller request (`RISK-03`).
       - Preserve natural `[w, h]` in `MediaView::Ready` (`RISK-02`).
       - Deduplicate identical bytes by `(len, fnv1a64(bytes))` into a shared `TextureHandle` table.
       - Maintain O(1) running `picture_bytes`, `svg_bytes`, and `animation_cpu_bytes` counters; eliminate `src.to_string()` / `key.to_owned()` on cache hits; lower default unified ceiling to `256 MB` desktop / `96 MB` wasm; evict cold `AnimationEntry` CPU pixel buffers when over budget.
    3. Pass `target_px` for non-tiled picture fills in `crates/render/src/paint.rs:457`.
    4. Add `U-01`, `U-02`, `G-02`.
  - **Tests:** `U-01`, `U-02`, `G-02`, `cargo test -p coursecraft-media -j 4`, `cargo test -p coursecraft-render -j 4 media`.
  - **Done when:** `G-02` proves 100-slide picture scroll stays `<= 8 MiB` with `full_res_thumbnail_decodes() == 0`.

- **S6 — Zero-Allocation `Course::validate()`, Scoped Validation & Engine-Cached Editor Validation**
  - **Title:** Make 300-slide edits 14–18× faster and no-ops 58× faster (`P4`).
  - **Files:** `crates/course/src/validate.rs`, `crates/course/src/tests.rs`, `crates/engine/src/lib.rs`, `crates/engine/src/commands.rs`, `crates/engine/src/m27_all_commands_undo_tests.rs`, `crates/ui-egui/src/lib.rs`, `crates/ui-egui/src/guard_tests.rs`.
  - **Exact changes:**
    1. Refactor `Ctx` in `crates/course/src/validate.rs` to hold a borrowed path segment stack (0 `String` allocations on valid elements!), remove `text_styles.clone()` (`:387`) and `master.clone()` (`:438`), and add `Course::first_error(&self)` + `Course::validate_slide_errors(&self, slide: &Slide, path_prefix: &str)`.
    2. In `crates/engine/src/commands.rs` and `lib.rs`: short-circuit `keep_valid` when `s.skip_undo` is set, short-circuit unchanged `object_set_frame` before `keep_valid`, use scoped slide validation for local `SingleSlide` commands (`RISK-05`), and add `Session::validation_issues(&mut self) -> &[Issue]` cached by `self.revision`.
    3. In `crates/ui-egui/src/lib.rs:4133` (`refresh_validate_only`), read `self.session.validation_issues()` directly (no redundant `course.validate` JSON round-trip) and only rebuild `last_known_slides` when slide IDs/order changed.
    4. Add `U-04`, `U-05`, `G-07`.
  - **Tests:** `U-04`, `U-05`, `G-07`, `PF-01`–`PF-03`.
  - **Gate #2:** `cargo xtask ci`.
  - **Done when:** `PF-01` `<= 20 ms`, `PF-02` `<= 60 ms`, `PF-03` `<= 5 ms`, and `U-05` proves 100% parity across all 186 commands.

- **S7 — Streaming `.ccraft` Open, Zero-Copy `Arc<[u8]>` Publish & Compact Published `course.json`**
  - **Title:** Cut Open and Publish peak RAM by 50% (−1.35 GB) and shrink published `course.json` by 38–62% (`P5` + Refactor `R4`).
  - **Files:** `crates/course/src/lib.rs`, `crates/format/src/lib.rs`, `crates/format/src/tests.rs`, `crates/publish/src/lib.rs`, `crates/publish/src/tests.rs`, `apps/coursecraft/src/files.rs`, `apps/coursecraft/src/publish.rs`, `apps/coursecraft-cli/src/lib.rs`.
  - **Exact changes:**
    1. Add `Course::to_json_compact(&self)` (`serde_json::to_string(self)`).
    2. Add `open_project_from<R: Read + Seek>` / `open_project_from_with_limits` in `crates/format/src/lib.rs` and wire `apps/coursecraft/src/files.rs:483` and `apps/coursecraft-cli` to stream from `BufReader::with_capacity(256 * 1024, File::open(path)?)`.
    3. Add `SharedPackageFile` / `SharedPackageInput` / `build_package_shared` / `zip_shared_package_to` in `crates/publish/src/lib.rs` using `Arc<[u8]>` (0 media byte copies!) and compact `course.json` for published packages (`RISK-06`); switch `apps/coursecraft/src/publish.rs` and `apps/coursecraft-cli` publish to use it with 256 KiB `BufWriter`.
    4. Add `U-06`, `G-04`.
  - **Tests:** `U-06`, `G-04`, `cargo test -p coursecraft-format -p coursecraft-publish -p coursecraft-cli -j 4`.
  - **Done when:** Open streams without `std::fs::read`, Publish shares `Arc` pointers (`0` media copies), and published `course.json` is compact.

- **S8 — Background Non-Blocking Autosave with `save_epoch` Guard**
  - **Title:** Eliminate 60-second UI freezes on large projects (`P6`).
  - **Files:** `apps/coursecraft/src/autosave.rs`, `apps/coursecraft/src/files.rs`, `apps/coursecraft/src/main.rs`, `apps/coursecraft/src/files_tests.rs`.
  - **Exact changes:**
    1. Add an optional `should_commit: &dyn Fn() -> bool` check inside `write_and_swap` (checked right before `rename(tmp, path)`).
    2. Add background autosave worker state (`autosave_rx`, `save_epoch: Arc<AtomicU64>`) on `EditorApp` (`apps/coursecraft/src/main.rs`), spawning `write_autosave_arc` on a worker thread with cloned `Course` + `used_media()` (`Arc` handles, 0 byte copies) and a 256 KiB `BufWriter`.
    3. Bump `save_epoch` on manual Save, Save As, Open, and New (`RISK-07`), and poll completion cheaply in `show_app`.
    4. Add `U-07`.
  - **Tests:** `U-07`, `cargo test -p coursecraft -j 4 autosave files`.
  - **Done when:** Autosave never blocks the UI thread and never creates a stale autosave after a manual Save.

- **S9 — `TextLayoutCache`, `ShapeMeshCache` (with Convex Fast-Path) & Main-Canvas Compose Cache**
  - **Title:** Eliminate per-frame text re-shaping, 22× `Autofit::Shrink`, O(N³) ear clipping, and idle canvas composes (`P7`).
  - **Files:** `crates/render/src/text_layout.rs`, `crates/render/src/shape_fill.rs`, `crates/render/src/paint.rs`, `crates/ui-egui/src/canvas.rs`, `crates/ui-egui/src/lib.rs`, `crates/render/src/tests.rs`, `crates/ui-egui/src/guard_tests.rs`.
  - **Exact changes:**
    1. Add bounded LRU `TextLayoutCache` (256 entries, keyed with `font_epoch` and quantized dimensions) in `crates/render/src/text_layout.rs`; skip `dimmed_runs` clone when `!dimmed && opacity >= 1.0` in `paint.rs:929`.
    2. Add strict convex fast-path (`RISK-10`) in `shape_fill.rs:179` and bounded LRU `ShapeMeshCache` (256 entries, local-space `Arc<Mesh>` translated to `rect.min` on hit) in `crates/render/src/paint.rs`.
    3. Add `CanvasComposeCache` on `UiState` used by `show_canvas_and_layers` (`canvas.rs:7113`), cleared in `Editor::replace_course`.
    4. Add `U-08` and `G-05`.
  - **Tests:** `U-08`, `G-05`.
  - **Gate #3:** `cargo xtask ci`.
  - **Done when:** `G-05` proves 0 canvas composes, 0 text-layout misses, and 0 shape-mesh misses on idle frames, and `U-08` proves pixel-identical rendering.

- **S10 — Unified Media Index (`MediaStore` / `R1`), Revision-Gated Editor Hooks, Fast Check Spelling & Fast CLI `validate`**
  - **Title:** Eliminate O(R × M) media scans, per-frame theme/timeline allocs, 300-slide spelling rescans, and 93 ms CLI font scans (`P9` + `P10` + Refactor `R1`).
  - **Files:** `crates/course/src/lib.rs`, `crates/ui-egui/src/lib.rs`, `crates/ui-egui/src/canvas.rs`, `crates/ui-egui/src/properties.rs`, `crates/ui-egui/src/timeline_playback.rs`, `crates/ui-egui/src/slides.rs`, `crates/ui-egui/src/spell_check.rs`, `crates/spell/src/lib.rs`, `apps/coursecraft-cli/src/lib.rs`, `apps/coursecraft-cli/src/batch.rs`.
  - **Exact changes:**
    1. Maintain an O(1) `HashMap<String, usize>` media index on `Editor` (kept in sync via helper methods; used by `MemoryMediaSource::get`, `missing_images_as_placeholders`, `refresh_validate_only`, and `timeline_playback::sync_slide`).
    2. Revision-gate `publish_theme_colors` (`properties.rs:1676`) and `timeline_playback::sync_slide` (`timeline_playback.rs:292`) so idle frames allocate 0 Vecs/Strings.
    3. In `spell_check.rs:588-615`, filter `state.issues` in-place on `Ignore`, `Ignore all`, and `Add to dictionary` without rescanning the course; remove the 4 per-word allocations in `crates/spell/src/lib.rs`.
    4. In `crates/ui-egui/src/slides.rs`, use engine's `last_changed_slide()` (when available) to skip fingerprinting untouched visible thumbnails on single-slide edits.
    5. In `apps/coursecraft-cli/src/lib.rs:1914`, check `collect_used_font_faces(&c)` before calling `SystemFontIndex::scan_default()`.
    6. Add `G-06`, `U-10`.
  - **Tests:** `G-06`, `U-10`, `cargo test -p coursecraft-spell -p coursecraft-cli -p coursecraft-ui-egui -j 4`.
  - **Done when:** `G-06` proves `speller_runs() == 0` on Ignore/Add, and CLI `validate` drops below `10 ms` on Inter-only courses.

- **S11 — Low-End `PerformanceMode` (`Auto` / `Standard` / `Lite`) & Memory-Pressure Guidance**
  - **Title:** Ship explicit Lite mode for 4 GB laptops, Chromebooks, and cheap phones (`P11`).
  - **Files:** `crates/render/src/lib.rs`, `crates/render/src/media.rs`, `crates/render/src/paint.rs`, `crates/ui-egui/src/preferences.rs`, `crates/ui-egui/src/lib.rs`, `apps/coursecraft/src/prefs.rs`, `apps/coursecraft-player/src/load.rs`, `apps/coursecraft-player/src/app.rs`.
  - **Exact changes:**
    1. Add `PerformanceMode` (`Auto`, `Standard`, `Lite`) + `PerfProfile` constants in `coursecraft-render` and thread active profile into `MediaCache`, `paint` (`MAX_EFFECT_LAYERS`, `GRADIENT_MAX_EDGE`, marker pulse), `TextLayoutCache`, `ShapeMeshCache`, and `ThumbnailCache`.
    2. Add **Performance** selector (`Automatic / Standard / Lite`) in Editor Preferences (persisted in `prefs.rs`) and `?lite=1` + automatic constrained-device detection in `coursecraft-player`.
    3. Emit a one-time non-blocking status notice when raw `Editor.media` exceeds `512 MiB` (desktop) or `128 MiB` (web).
    4. Add `U-11`.
  - **Tests:** `U-11`, `PF-05`.
  - **Done when:** Lite mode enforces every §3 P11 ceiling cleanly in both editor and web player.

- **S12 — Full Release Verification, Before/After Perf Report & `docs/performance.md` Update**
  - **Title:** Run complete release benchmarks, WASM size checks, and update documentation.
  - **Files:** `docs/performance.md`, `docs/testing/m34-report.md` (new, created by implementer at end of M34).
  - **Exact changes:**
    1. Run all release benchmarks (`perf_tests`, `undo_perf_tests`, CLI `validate`/`simulate`, `big_course` Open/Publish stages under `/usr/bin/time -l`, and `cargo xtask build-player` WASM size check).
    2. Record every before/after metric against §2 and §4 in `docs/performance.md` and `docs/testing/m34-report.md`.
  - **Gate #4 (Release Gate):** See §9.
  - **Done when:** All gates pass and every target budget in §4 is met or explicitly explained.

---

## 9. Release Gate

Before M34 is merged, ALL of the following must pass:
1. **`cargo xtask ci`** — `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo xtask layers`, and `cargo xtask wasm` (all 17 WASM crates).
2. **`cargo xtask build-player`** — builds `target/player-dist` cleanly and verifies `.wasm` size meets the §4 budget (record exact raw + gzipped bytes!).
3. **Release editor perf suite:**
   ```sh
   cargo test --release -p coursecraft-ui-egui perf_tests -- --ignored --nocapture --test-threads=1
   ```
4. **Release engine undo/edit perf suite:**
   ```sh
   cargo test --release -p coursecraft-engine undo_perf -- --ignored --nocapture --test-threads=1
   ```
5. **Release big-course memory check (`open` and `publish` stages):**
   ```sh
   BIG_STAGE=save BIG_KEEP=1 /usr/bin/time -l cargo test --release -p coursecraft-cli --test big_course -- --ignored --nocapture
   BIG_STAGE=open BIG_KEEP=1 /usr/bin/time -l cargo test --release -p coursecraft-cli --test big_course -- --ignored --nocapture
   BIG_STAGE=publish /usr/bin/time -l cargo test --release -p coursecraft-cli --test big_course -- --ignored --nocapture
   ```
6. **Guard & idle suites:** `cargo test -p coursecraft-ui-egui guard_tests idle_tests` and `cargo test -p coursecraft-player`.

---

## 10. Owner Manual Verification Checklist (~10 Minutes)

- [ ] **Idle CPU check (Editor & Preview):** Open `examples/showcase/` in the desktop app, wait 2 seconds, and watch Activity Monitor / Task Manager CPU — verify the idle editor sits at **~0% CPU**. Open **Preview whole course**, let slide 1 finish its timeline (or pause it), and verify CPU drops back to **~0%** (no continuous 60 fps spin).
- [ ] **Published Web Player on phone / browser DevTools:** Publish `examples/showcase/` to Web (`File ▸ Publish`), open in Chrome/Safari DevTools → Performance & Network tabs:
  - Verify `.wasm` transfer size is markedly smaller and `course.json` is compact.
  - Sit on the Start screen and on a completed quiz slide for 5 seconds — verify **0 animation frames painted** while idle, and clicking Next / playing an animation / running a timer still responds immediately and smoothly.
- [ ] **Large-course responsiveness & RAM:** Open a 100+ slide course with photos (or duplicate slides in `examples/showcase/`), scroll the slide list rapidly, nudge objects, undo/redo rapidly, and run **Edit ▸ Check spelling…** clicking **Ignore** repeatedly — verify zero stutters and low RAM growth.
- [ ] **Background autosave:** Edit a large `.ccraft` project and wait past the autosave interval (set to 1 min in Preferences) while continuously dragging/nudging — verify **zero UI freeze** while autosave writes, then press `⌘S` / `Ctrl+S` and reopen to confirm no false "Restore unsaved changes?" prompt appears.
- [ ] **Lite mode:** Switch **Preferences ▸ Performance** to **Lite** (and open a published course with `?lite=1`) — verify everything renders cleanly and interactions/triggers work identically.
