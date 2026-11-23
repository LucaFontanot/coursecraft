# M33 Undo That Stores Changes, Not Copies — Architecture, Migration & Test Plan

**Role:** Systems Architecture, Performance Audit & Migration Plan (planning only — zero source files edited)  
**Target release:** 0.4.4 / M33 (starts after M31 is merged; independent of M32 except shared `guard_tests.rs` hygiene)  
**Scope:**
Replace `coursecraft_engine::Session`'s full-course-copy undo stack (`Vec<(Course, String)>`) with a **reversible structural delta engine (`CourseDelta`) plus scoped pre-edit snapshotting (`UndoScope`)**, cutting undo history memory on 300-slide courses by **> 200×–1,000×** and eliminating full-course cloning on everyday slide/object edits — while keeping **byte-identical `course.json` restoration**, atomic multi-command batches, error rollback, `skip_undo`, hostile-input safety, and WASM compatibility 100% intact.

**Builds on M31 (do not redo M31):** M31 covers urgent external-review bug fixes (File shortcuts, MCP media, clamp crashes, empty state image publish, SCORM 1.2 pass downgrade, last-visited loop, web unsaved prompt, chunked web publish). M33 focuses strictly on engine undo storage, snapshotting, and performance.

**Clean-room commitment:** Pure Rust data structures and diff/patch engine written from scratch inside `coursecraft-engine` (`docs/contributing.md:21-40`). Zero external undo/diff libraries required; zero competitor code or concepts.

> **Implementer Guardrails (`AGENTS.md`, `docs/contributing.md`)**
> 1. **Never crash:** Zero `unwrap`/`expect`/`panic!`/`unreachable!`/`todo!` in non-test code; `unsafe_code = "forbid"`. Every slice/index lookup in `CourseDelta::apply` uses `.get()` / `.get_mut()` and returns `Err(EngineError::Invalid(...))` if an invariant is ever violated.
> 2. **Everything is a command:** Every mutating command in `REGISTRY` remains automatically atomic and undoable with zero per-command inverse boilerplate required for safety (unknown/unannotated commands default to `UndoScope::FullCourse`).
> 3. **Byte-identical restoration:** After any command or batch, `undo()` must restore **byte-identical `Course::to_json()`** (and `PartialEq` equality), and `redo()` must restore **byte-identical post-command JSON**.
> 4. **Zero breaking changes outside `coursecraft-engine`:** `coursecraft-course` public structs (`Course`, `Scene`, `Slide`, `Object`) stay plain ergonomic Rust types.
> 5. **Layering & WASM:** `coursecraft-engine` (L5) depends only on `coursecraft-course`, `coursecraft-format`, `serde`, `serde_json`, `thiserror`. `cargo xtask layers` and `cargo xtask wasm` stay green on every step.
> 6. **Safe migration (dual-path → differential proof → switch → cleanup):** Never delete the old snapshot path until a differential oracle proves byte-for-byte parity across every command in `m27_all_commands_undo_tests.rs`, `bughunt_undo_tests.rs`, `tests.rs`, and `panic_hunt`.

---

## 1. Goals & Where Full Copies Live Today

### 1.1 Exact Audit of Current Undo Implementation
Inspected `crates/engine/src/lib.rs`, `crates/engine/src/commands.rs`, `crates/engine/src/connectors.rs`, and `crates/ui-egui/src/lib.rs`:

- **Storage (`crates/engine/src/lib.rs:69-72`):**
  ```rust
  pub struct Session {
      pub course: Course,
      undo: Vec<(Course, String)>,   // <-- full deep clone of entire Course per step!
      redo: Vec<(Course, String)>,   // <-- full deep clone of entire Course per step!
      pub revision: u64,
      skip_undo: bool,
      history_truncated: bool,
      ...
  }
  ```
- **Per-command copy (`crates/engine/src/lib.rs:227-255`):** Every mutating `Session::execute` calls `let snapshot = self.course.clone();` **up front** before running `(cmd.run)(self, params)`, compares `self.course == snapshot` across all 300 slides, runs `reroute_connectors(&mut self.course)` across all 300 slides, pushes the full `snapshot` onto `self.undo`, and evicts via `self.undo.remove(0)` (O(N) shift + drops a multi-MB `Course` heap tree) once `undo.len() > MAX_HISTORY` (`200`).
- **Batch copy (`crates/engine/src/lib.rs:303-370`):** `execute_batch_with_name` clones the entire `Course` at line `303`, AND when `name.is_none()`, clones the entire `Course` **a second time** at line `326` (`step_snapshot`) on every mutating step until `first_label` is found!
- **No-op waste:** Even when a command sets `s.skip_undo = true` (26 places in `commands.rs` + `commands/styles.rs`) or leaves `self.course == snapshot`, `Session::execute` already paid the full `Course::clone()` + `Course::eq()` + `Drop` cost!
- **Undo step merging:** Currently `Session` has **no** step-merging API; interactive drags (`canvas.rs`, colour picker in `properties.rs`) coalesce by holding transient UI drag state and committing one command on pointer release, while multi-step actions use `execute_batch` / `execute_batch_named`. M33's delta design must preserve batches and cleanly support optional coalescing/merging (`CourseDelta::compose` / `merge_last_undo`).
- **Media bytes verified NOT duplicated:** Audited `crates/ui-egui/src/lib.rs:1140` (`pub media: Vec<(String, Arc<[u8]>)>`) and `crates/course/src/model.rs:328-379`. `Course` stores only media path strings (`src: String`); raw media bytes live outside `Course` and `Session`, shared via `Arc<[u8]>`. Our live `.ccraft` measurement below confirms 0 bytes of media duplication in undo.

---

## 2. Baseline Measurements (Measured Before Planning)

Measured on Apple Silicon (macOS), release build (`target/release/coursecraft-cli run` under `/usr/bin/time -l`, median of 3 runs), executing `object.set_frame` edits, `edit.undo`, and `edit.redo` on three courses:
1. **`perf300.json`**: Exact shape of `crates/ui-egui/src/perf_tests.rs::big_course()` — 300 slides in 10 scenes, 6 objects/slide (`313,294 bytes` JSON).
2. **`rich300.json`**: Realistic rich authoring course — 300 slides in 10 scenes, 15 stateful shape/text objects/slide (`hover` + `visited` states each, `1,666,598 bytes` JSON).
3. **`perf300_100mb.ccraft`**: `perf300.json` packaged with `10 × 10 MB` stored media blobs (`100 MB` media) to verify media isolation.

### 2.1 Measured CLI Engine Baseline

| Course & Workload | Wall Time (median) | Max RSS | Peak Memory Footprint | Footprint Delta vs 0 Ops | Per-Step Memory |
|---|---:|---:|---:|---:|---:|
| **`perf300.json`** — 0 ops (parse + save baseline) | 14.7 ms | 13.42 MB | 6.83 MB | — | — |
| **`perf300.json`** — 50 `object.set_frame` edits | 116.1 ms | 112.62 MB | 105.19 MB | **+98.36 MB** | **2,014 KB/step** |
| **`perf300.json`** — 200 edits (`MAX_HISTORY`) | 396.5 ms | 406.02 MB | **398.72 MB** | **+391.89 MB** | **2,006 KB/step** |
| **`perf300.json`** — 200 edits + 200 `edit.undo` | 418.8 ms | 406.19 MB | 398.88 MB | +392.05 MB | — |
| **`perf300.json`** — 200 edits + 200 undo + 200 redo | 392.6 ms | 406.33 MB | 399.00 MB | +392.17 MB | — |
| **`perf300.json`** — 300 edits (100 evictions past cap) | 545.8 ms | — | 400.86 MB | +394.03 MB | — |
| **`perf300.json`** — 200 **no-op** `object.set_frame` edits | 353.8 ms | — | 9.64 MB | +2.81 MB | **1.70 ms/no-op!** |
| **`rich300.json`** — 0 ops (parse + save baseline) | 29.8 ms | 30.89 MB | 24.33 MB | — | — |
| **`rich300.json`** — 200 edits (`MAX_HISTORY`) | **2,119.2 ms** | **1,930.80 MB** | **1,924.38 MB** | **+1,900.05 MB** | **9,728 KB/step** |
| **`rich300.json`** — 200 edits + 200 undo + 200 redo | 2,160.5 ms | 1,931.11 MB | 1,924.69 MB | +1,900.36 MB | — |
| **`perf300_100mb.ccraft`** — 0 ops (100 MB media) | 71.5 ms | 235.05 MB | 228.30 MB | — | — |
| **`perf300_100mb.ccraft`** — 200 edits (100 MB media) | 458.1 ms | 627.30 MB | 619.99 MB | **+391.69 MB** | **2,005 KB/step** *(media untouched)* |

### 2.2 Derived Per-Operation Baseline & Editor Perf Baseline
- **`perf300.json`:** **1.91 ms/edit** (1.70 ms even on a no-op!), **1.49 ms/edit** steady-state with eviction, **~0.05–0.11 ms/undo**, **~392 MB RAM** for 200 steps (~6.4× the 313 KB JSON size per copy!).
- **`rich300.json`:** **10.45 ms/edit**, **~0.10 ms/undo**, **~1,900 MB (~1.9 GB!) RAM** for 200 steps — enough to **OOM a WASM browser tab** on a single rich course!
- **Editor `perf_tests` (`cargo test --release -p coursecraft-ui-egui perf_tests -- --ignored --nocapture --test-threads=1`):**
  - `undo / redo move` (30 cycles of nudge + undo + redo on `perf300`): **9.52 ms median / 9.85 ms p95** (budget 33 ms).
  - `insert interaction + undo` (8 interaction kinds on `perf300`): **2.90 ms median / 3.62 ms p95** (budget 100 ms).

---

## 3. Architecture Comparison & Recommendation

We evaluated **four** concrete architectures against CourseCraft's rules (every command undoable, byte-identical JSON, atomic batches, error rollback, `skip_undo`, step merging, zero panics, WASM, clean layering):

| Criterion | 1. JSON Patches (RFC 6902) | 2. Per-Command Inverse Ops | 3. Structural Sharing (`Arc<Slide>` in `coursecraft-course`) | 4. **Recommended:** Engine-Only Structural Delta (`CourseDelta`) + Scoped Pre-Edit Snapshot (`UndoScope`) |
|---|---|---|---|---|
| **200-step memory (`perf300` / `rich300`)** | ~0.2 MB / ~0.4 MB *(+15–25 MB shadow `Value`)* | ~0.1 MB / ~0.2 MB | ~2.0 MB / ~7.0 MB *(50-obj slide still copies ~500 KB/edit)* | **~0.2 MB / ~0.5 MB** *(1 `Box<Object>` per single-object edit!)* |
| **Edit time (`rich300`, baseline 10.45 ms)** | **~25–40 ms** *(worse! full JSON serialize + tree diff)* | < 0.5 ms | ~0.8–1.2 ms | **~0.8–1.5 ms** *(clones 1 slide ~15 µs, not 300 slides!)* |
| **Undo / Redo time (`rich300`)** | **~15–30 ms** *(worse! patch + deserialize whole `Course`)* | < 0.05 ms | **< 0.01 ms** *(pointer swap)* | **< 0.02 ms** *(in-place field/Box swap, 0 allocations!)* |
| **Byte-identical JSON guarantee** | Risky (`f32`↔`f64` JSON rounding, `-0.0`) | High human-error risk across 186 commands | Guaranteed | **Guaranteed** *(exact Rust values moved/swapped directly; zero serde in undo/redo!)* |
| **Automatic coverage of all 186 + future commands** | Yes | **No** (186 hand-rolled inverses + cross-slide trigger cleanup + `reroute_connectors`!) | Yes | **Yes** *(defaults to `FullCourse` scope; exhaustive destructuring catches new `Course`/`Slide` fields at compile time!)* |
| **Blast radius across workspace** | L5 only | L5 (`commands.rs` 453 KB rewrite) | **Very high** (`Vec<Slide>` → `Vec<Arc<Slide>>` breaks ~150 test files & mutable borrows in 10 crates; needs `serde/rc`) | **L5 only (`crates/engine`)** *(zero breaking changes to `coursecraft-course` or any other crate!)* |
| **Error rollback atomicity** | Slow (deserialize whole `Course`) | Fragile (partial mutations hard to unwind) | O(1) swap | **O(1) scoped restore** (`*slide = before_slide` or `*course = before_course`) |
| **Step merging / coalescing** | Easy | Hard | Easy | **Easy** *(O(1) merge for same-object edits; `Sequence` composition for arbitrary deltas)* |
| **WASM & Layering** | Extra deps/allocations | Clean | Needs `serde/rc` | **Clean** (zero new deps, pure `std` + L1 types) |

### 3.1 Why We Recommend Option 4 (Engine-Only Structural Delta + Scoped Snapshot)
1. **Fixes BOTH memory AND speed:** Storing `CourseDelta` cuts 200-step memory on `rich300` from **1,900 MB to < 1 MB** (**~2,000× reduction**), while `UndoScope::SingleSlide` / `CourseHeader` eliminates the 10 ms pre-command full-course clone on the ~145 hot editing commands!
2. **Zero blast radius outside `crates/engine`:** Unlike `Arc<Slide>` in `coursecraft-course`, Option 4 touches zero public model fields, zero `serde` features, and zero downstream crates (`coursecraft-logic`, `coursecraft-render`, `coursecraft-format`, `coursecraft-pptx`, `coursecraft-publish`, `coursecraft-export`, `apps/*`).
3. **Compile-time completeness & zero drift:** Exhaustive struct destructuring of `Course` (21 fields) and `Slide` (15 fields) inside `crates/engine/src/delta.rs` makes the Rust compiler reject any future model field addition until `delta.rs` handles it. And a `#[cfg(test)]` **scope-isolation verifier** proves no `SingleSlide` or `CourseHeader` command ever mutates anything outside its declared scope.
4. **Symmetric in-place swap (`apply`):** Calling `delta.apply(&mut self.course)` transforms `after → before` while simultaneously turning `delta` into its own exact inverse (`before → after`) in place — **zero heap allocations and zero clones on undo or redo!**

---

## 4. Detailed Design of `crates/engine/src/delta.rs` & `Session`

### 4.1 Core Data Types (`crates/engine/src/delta.rs`)

```rust
/// Identifies one slide anywhere inside a [`Course`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SlideLoc {
    SceneSlide { scene: usize, slide: usize },
    Master(usize),
    Template(usize),
    BankSlide { bank: usize, slide: usize },
}

/// Reversible change inside a single [`Slide`].
/// `apply(&mut self, slide)` swaps old/new in place so `self` becomes its own inverse.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SlideDelta {
    /// One object modified in place (`layer: None` = base layer, `Some(li)` = `slide.layers[li]`).
    ObjectModified {
        layer: Option<usize>,
        index: usize,
        object: Box<Object>,
    },
    /// One object inserted at `index` (applying removes it -> becomes `ObjectRemoved`).
    ObjectInserted { layer: Option<usize>, index: usize },
    /// One object removed from `index` (applying inserts it -> becomes `ObjectInserted`).
    ObjectRemoved {
        layer: Option<usize>,
        index: usize,
        object: Box<Object>,
    },
    /// One object moved from `from` to `to` (applying moves `to -> from` and swaps `from`/`to`).
    ObjectReordered {
        layer: Option<usize>,
        from: usize,
        to: usize,
    },
    /// One or more slide metadata fields changed (title, duration, background, transition, notes,
    /// master, revisit, player, question, draw, cue_points, triggers) without replacing objects.
    Fields(Box<SlideFieldPatch>),
    /// Fallback when multiple objects or layers changed in one step (e.g. align, paste, interaction.insert).
    FullSlide(Box<Slide>),
}

/// Reversible change to a scene list or slide list inside a scene/master/template/bank.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ListOp<T> {
    Insert { index: usize },
    Remove { index: usize, item: Box<T> },
    Move { from: usize, to: usize },
}

/// Reversible top-level change to a [`Course`].
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CourseDelta {
    /// Single slide modified in place (the ~85% common case).
    SlideChange { loc: SlideLoc, change: SlideDelta },
    /// Course-header fields only (zero slides touched: title, theme, player, quiz, glossary, etc.).
    Header(Box<CourseHeaderPatch>),
    /// Mixed / structural change: optional header patch + optional structural list ops + changed slides.
    Patch(Box<CoursePatch>),
    /// Sequence of sub-deltas (applied in reverse order, reversing the Vec in place — enables clean merging!).
    Sequence(Vec<CourseDelta>),
    /// Ultimate exact fallback for wholesale course rewrites (e.g. `course.set_slide_size` resizing 300 slides).
    FullCourse(Box<Course>),
}
```

#### Compile-Time Exhaustiveness Guard
Both `CourseHeaderPatch::diff` / `apply` and `SlideFieldPatch::diff` / `apply` destructure `Course` and `Slide` **without `..`**:
- `Course` has 21 fields: 17 header fields (`schema_version`, `id`, `title`, `slide_size`, `variables`, `start_slide`, `quiz`, `player`, `theme`, `language`, `strings`, `fonts`, `background_audio`, `text_styles`, `resources`, `glossary`, `editor`) + 4 slide collections (`scenes`, `masters`, `templates`, `banks`).
- `Slide` has 15 fields: `id`, `title`, `duration_ms`, `background`, `objects`, `layers`, `triggers`, `cue_points`, `question`, `draw`, `transition`, `notes`, `master`, `revisit`, `player`.
- If a future milestone adds a 22nd field to `Course` or 16th field to `Slide`, compilation fails immediately until `delta.rs` handles it.

#### Diff Granularity Rules (`SlideDelta::diff` & `CourseDelta::diff`)
1. **`SlideDelta::diff(before: Slide, after: &Slide) -> Option<SlideDelta>`:**
   - If `before == *after`: return `None` (no-op!).
   - Compare all non-`objects`, non-`layers` fields first. If `before.objects == after.objects && before.layers == after.layers`: build `SlideDelta::Fields(Box::new(...))` containing only the changed fields (moved out of `before` — zero extra clones!).
   - If all metadata + `layers` match and only `objects` differs:
     - Same len, 1 index differs → `SlideDelta::ObjectModified { layer: None, index, object: Box::new(before_obj) }` (**~0.5–2 KB!**).
     - `after.len() == before.len() + 1` and removing 1 element at `index` makes them equal → `SlideDelta::ObjectInserted { layer: None, index }` (**24 bytes!**).
     - `after.len() + 1 == before.len()` and removing 1 element from `before` at `index` makes them equal → `SlideDelta::ObjectRemoved { layer: None, index, object: Box::new(removed) }`.
     - Same len, single move `from -> to` → `SlideDelta::ObjectReordered { layer: None, from, to }` (**24 bytes!**).
   - Same single-object detection for a single changed layer `layers[li]` when all layer metadata and other layers match!
   - Otherwise → `SlideDelta::FullSlide(Box::new(before))` (moves `before` straight into the `Box`, zero extra cloning!).
2. **`CourseDelta::diff(before: Course, after: &Course) -> Option<CourseDelta>`:**
   - If `before == *after`: return `None`.
   - Diff the 17 header fields into `Option<CourseHeaderPatch>`.
   - If all 4 slide collections (`scenes`, `masters`, `templates`, `banks`) match: return `CourseDelta::Header(...)`.
   - Check if `scenes` structure matches (same scene count, same `id`/`title` per scene, same slide count per scene):
     - Collect `(SlideLoc, SlideDelta)` for every slide where `before_slide != after_slide`.
     - If > 50% of all slides changed (e.g. `course.set_slide_size` scaling all 300 slides) → move `before` straight into `CourseDelta::FullCourse(Box::new(before))` (faster & simpler than 300 `FullSlide` entries!).
     - If exactly 1 slide changed and no header/master/template/bank changes → `CourseDelta::SlideChange { loc, change }`.
     - Otherwise → `CourseDelta::Patch(...)` storing only the changed slides & header fields!
   - If a single slide was inserted, removed, or moved within/between scenes (e.g., `slide.add`, `slide.delete`, `slide.move`, `slide.duplicate`, `slide.paste`, `results.insert`, `certificate.insert`), capture the minimal `SceneSlideOp` (`InsertSlide`, `RemoveSlide`, `MoveSlide`, plus any other slides whose triggers changed, e.g. `slide.delete` pruning jump triggers on 2 other slides!).
   - Otherwise (multi-scene rewrite, e.g. `course.import_outline`) → `CourseDelta::FullCourse(Box::new(before))` (or `CoursePatch` with `SceneOp`s).
3. **Hostile-input safety in `apply`:**
   - Every index in `apply` uses `.get_mut()` / bounds checks (`index <= vec.len()` before `insert`, `index < vec.len()` before `remove`). On any impossible out-of-bounds state, return `Err(EngineError::Invalid("undo delta out of range".into()))` — **zero panics, zero `unwrap`**.

### 4.2 Scoped Pre-Edit Snapshot (`UndoScope`)
Add `pub(crate) scope: UndoScope` to `Command` in `crates/engine/src/commands.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UndoScope {
    /// Default & safest: snapshots full `Course` temporarily, diffs into `CourseDelta`,
    /// drops the temporary clone immediately. Used for structural/multi-slide commands.
    FullCourse,
    /// Command mutates at most the single slide named by `params["slide"]` (in scenes,
    /// masters, templates, or banks). Clones ONLY that 1 `Slide` before running, rolls
    /// back ONLY that 1 `Slide` on `Err`, reroutes connectors ONLY in that 1 `Slide`,
    /// and diffs ONLY that 1 `Slide` into `CourseDelta::SlideChange`.
    /// If `params["slide"]` is missing or not found, safely falls back to `FullCourse`.
    SingleSlide,
    /// Command mutates only the 17 course-header fields (zero slides, scenes, masters,
    /// templates, or banks touched). Snapshots ONLY `CourseHeaderSnapshot` (zero slides cloned!).
    CourseHeader,
}
```

- **Command classification across all 186 commands:**
  - `UndoScope::SingleSlide` (~125 commands): `object.*` (all object property, frame, rotation, flip, lock, hide, opacity, style, shadow, glow, fill_style, animation, motion_path, state, state_extra, add, delete, duplicate, paste, reorder, align, distribute, match_size, group, ungroup, drag, lottie, web, input, marker, panel, scroll_panel, timer, meter, path, crop, captions, media), `text.*`, `table.*`, `layer.*`, `trigger.*`, `question.*`, `video.*`, `slide.update`, `slide.set_duration`, `slide.set_background`, `slide.set_transition`, `slide.set_player_overrides`, `slide.set_master`, `slide.add_cue`, `slide.set_cue`, `slide.remove_cue`, `interaction.insert`, `draw.update`.
    - *Careful exclusions that MUST stay `FullCourse`:* `object.delete`? Check if `object.delete` prunes triggers on *other* slides or only the same slide! (Audit `object_delete` in `commands.rs` before annotating — `object.delete` only prunes triggers on the same slide, whereas `slide.delete` prunes triggers across ALL slides so `slide.delete` MUST stay `FullCourse`! The `#[cfg(test)]` scope-isolation verifier catches any mis-annotation automatically!)
    - *Check `layer.delete`, `question.delete`, `slide.set_master`, `course.add_finish_button`:* `course.add_finish_button` adds to a slide; verify each with the scope-isolation verifier!
  - `UndoScope::CourseHeader` (~22 commands): `course.set_title`, `course.update`, `course.set_player`, `course.set_quiz`, `course.set_publish_settings`, `course.set_background_audio`, `course.set_theme`, `theme.set_palette`, `theme.set_palette_color`, `course.add_font`, `course.remove_font`, `text_style.create`, `text_style.update`, `text_style.delete`, `view.set`, `view.guides.add/move/remove`, `resource.add/update/remove/move`, `glossary.add/update/remove/move/sort`, `variable.add`, `variable.set_value`. *(Note: `variable.delete` may touch triggers across slides — check and keep `FullCourse` if so! `text_style.apply` modifies objects on slides — keep `FullCourse` or `SingleSlide`!)*
  - `UndoScope::FullCourse` (~35 structural / course-wide commands): `slide.add`, `slide.add_from_layout`, `slide.delete`, `slide.move`, `slide.duplicate`, `slide.paste`, `scene.*`, `master.*`, `bank.*`, `template.*`, `draw.insert`, `results.insert`, `certificate.insert`, `course.set_slide_size`, `course.recolor`, `course.replace`, `course.import_xliff`, `course.import_outline`, `variable.delete`, `text_style.apply`, etc.
- **Per-slide connector rerouting (`crates/engine/src/connectors.rs`):**
  - `connectors.rs:27-46` already runs strictly per-slide (`reroute_owner(&mut slide.objects)` + each layer). Extract `pub(crate) fn reroute_connectors_in_slide(slide: &mut Slide)` and call it for `SingleSlide` edits instead of scanning all 300 slides!
- **Batch fast-path (`execute_batch_with_name`):**
  - Before cloning, check if every mutating step in `steps` has `UndoScope::SingleSlide` targeting the **exact same `slide` ID** (and that `SlideLoc` exists). If so, clone ONLY that 1 slide once for the entire batch!
  - Also eliminate the redundant second `Course::clone()` (`step_snapshot` at `crates/engine/src/lib.rs:326`) used only to find `first_label` — inside the single-slide batch fast-path it compares only 1 slide, and in the full-course batch path compare only if `first_label.is_none()` using the first mutating command whose `skip_undo` was not set (or lightweight slide check)!
- **Queue eviction (`VecDeque`):**
  - Replace `undo: Vec<...>` with `undo: VecDeque<(CourseDelta, String)>` so capping at `MAX_HISTORY = 200` uses O(1) `pop_front()` instead of O(N) `remove(0)`.
- **Undo step merging (`CourseDelta::merge_with_next` / `Session::merge_last_undo`):**
  - Provide clean `CourseDelta::merge(older: CourseDelta, newer: CourseDelta) -> CourseDelta`:
    - Fast O(1) collapse when both are `SlideChange { loc, change: SlideDelta::ObjectModified { layer, index, .. } }` targeting the same `(loc, layer, index)`: keep `older` (the original pre-edit `Object`) and drop `newer`!
    - Fast O(1) collapse when both are `SlideChange { loc, change: SlideDelta::FullSlide(_) }` on the same `loc`: keep `older`!
    - General fallback: `CourseDelta::Sequence(vec![older, newer])` (when undoing, apply `newer` then `older`, reversing the vec into `[older_inv, newer_inv]` so redo applies `older_inv` then `newer_inv`!).
  - Expose `pub fn merge_last_two_undos(&mut self, label: Option<&str>) -> bool` (or `execute_merged` for future drag/typing coalescing), tested with byte-identical undo/redo roundtrips.

---

## 5. Performance & Memory Budgets

| Metric | Measured Before M33 | Target Budget After M33 | Verification |
|---|---:|---:|---|
| **`perf300` — 200 single-object edits heap delta** | **+391.89 MB** (`2,006 KB/step`) | **≤ 2.0 MB** (`≤ 10 KB/step`, ~200×–800× lower) | `undo_perf_tests` + `GD-02` |
| **`rich300` — 200 single-object edits heap delta** | **+1,900.05 MB** (`9,728 KB/step`) | **≤ 4.0 MB** (`≤ 20 KB/step`, ~500×–2,000× lower) | `undo_perf_tests` + `GD-02` |
| **`perf300` — 200 edits + 200 undo + 200 redo wall time** | **396 ms** (`1.91 ms/edit`) | **≤ 120 ms** (`≤ 0.60 ms/edit`, ≥ 3× faster) | `undo_perf_tests` (`PF-01`) |
| **`rich300` — 200 edits wall time** | **2,119 ms** (`10.45 ms/edit`) | **≤ 400 ms** (`≤ 2.0 ms/edit`, ≥ 5× faster) | `undo_perf_tests` (`PF-02`) |
| **`perf300` — single `undo()` or `redo()` latency** | ~0.05–0.11 ms | **≤ 0.05 ms** (median) | `undo_perf_tests` (`PF-01`) |
| **`perf300` — 200 no-op mutating commands wall time** | **353.8 ms** (`1.70 ms/no-op`) | **≤ 100 ms** (`≤ 0.50 ms/no-op`) | `undo_perf_tests` (`PF-03`) |
| **Full-course clones on object nudge (`guard_tests.rs`)** | **1** per nudge | **0** per nudge | `guard_tests.rs` (`GD-01`) |
| **Editor `perf_tests` `undo / redo move` (30 cycles)** | **9.52 ms / 9.85 ms** | **≤ 6.5 ms / ≤ 7.5 ms** (budget 33 ms) | `perf_tests.rs` (`PF-04`) |
| **Media bytes duplicated by undo (`100 MB .ccraft`)** | **0 MB** | **0 MB** | Verified in design + `PF-01` |

---

## 6. Normative Rules (`M33-01`–`M33-12`)

| Rule ID | Category | Normative Requirement |
|---|---|---|
| **M33-01** | Byte-Identical Undo/Redo | For every mutating command in `REGISTRY` (all 186 commands) and every batch, `undo()` restores byte-identical `Course::to_json()` and `redo()` restores byte-identical post-command `Course::to_json()`. |
| **M33-02** | Differential Equivalence | Across the entire `m27_all_commands_undo_tests.rs`, `bughunt_undo_tests.rs`, and `tests.rs` suites, delta-based `Session` matches legacy full-snapshot `Session` byte-for-byte after every command, undo, redo, multi-step `undo_steps`/`redo_steps`, and rollback. |
| **M33-03** | Scope Isolation | In test builds, every `UndoScope::SingleSlide` and `UndoScope::CourseHeader` command is verified to mutate zero fields outside its declared scope; any command that can touch multiple slides stays `UndoScope::FullCourse`. |
| **M33-04** | Atomic Error Rollback | A failing mutating command or failing multi-step batch leaves `session.course`, `undo_names()`, `redo_names()`, and `revision` byte-for-byte unchanged. |
| **M33-05** | `skip_undo` & No-Op Parity | A mutating command setting `s.skip_undo = true` or producing zero change records no undo step, clears no redo stack, and does not bump `revision`. |
| **M33-06** | History Cap & Truncation | `MAX_HISTORY = 200` is enforced with O(1) `VecDeque::pop_front()`, `history_truncated()` flips to `true` on first eviction, and `undo_steps(n)` / `redo_steps(n)` remain atomic (rejecting `n > len` with `EngineError::NothingTo` without mutating). |
| **M33-07** | Step Merging | Merging two consecutive undo steps yields 1 undo step whose `undo()` restores the pre-first-edit JSON byte-for-byte and whose `redo()` restores the post-second-edit JSON byte-for-byte. |
| **M33-08** | Zero Full-Course Clones on Hot Edits | Single-slide edits (`object.set_frame`, `text.set`, `object.update`, etc.), course-header edits, and same-slide batches perform **0** full-course clones (`coursecraft_engine::test_counts::full_course_clones() == 0`). |
| **M33-09** | Bounded Delta Memory | 200 single-object edits on `perf300` keep total estimated undo payload well under `2 MB` (and `<= 4 MB` on `rich300`), vs `392 MB` / `1,900 MB` before M33. |
| **M33-10** | Hostile Input Safety | `panic_hunt` and malformed/extreme delta tests produce **0 panics**; zero `unwrap`/`expect`/`panic!`/`unreachable!`/`todo!` in `crates/engine/src`. |
| **M33-11** | Compile-Time Field Coverage | `Course` and `Slide` are destructured exhaustively (no `..`) in `delta.rs`, so adding any future model field forces delta support at compile time. |
| **M33-12** | WASM & Layering Parity | `crates/engine` adds zero new crate dependencies; `cargo xtask layers` and `cargo xtask wasm` stay green. |

---

## 7. Watchlist of Risky Spots (`RISK-01`–`RISK-09`)

| ID | File & Lines | Risk & Mitigation |
|---|---|---|
| **RISK-01** | `crates/engine/src/commands.rs` (mis-annotating a cross-slide command as `SingleSlide`) | Some commands take a `"slide"` param but ALSO mutate other slides or course fields (e.g., `slide.delete` removes triggers across all slides; `question.insert` / `results.insert` / `certificate.insert` may set `course.quiz` or add slides; `variable.delete` removes trigger references across slides; `interaction.insert` might add a course variable!). **Mitigation:** Run the `#[cfg(test)]` scope-isolation verifier (`S5`/`S6`) across ALL 186 commands in `m27_all_commands_undo_tests.rs` + `tests.rs` — any command that touches anything outside its declared scope immediately panics in tests and stays `UndoScope::FullCourse`! |
| **RISK-02** | `crates/course/src/model.rs` (`f32` equality on `NaN` / `-0.0`) | In Rust, `f32::NaN != f32::NaN` and `-0.0 == 0.0` (while `serde_json` serializes `-0.0` as `"-0.0"` and `0.0` as `"0.0"`!). If a field changes from `0.0` to `-0.0`, `==` says equal so `diff` might skip it, breaking byte-identical JSON. **Mitigation:** Validated courses never contain `NaN`, and in `SlideDelta::diff` / `CourseDelta::diff` use `Course::to_json()` ONLY in `#[cfg(test)]` differential assertions, and ensure `f32` fields in `SlideFieldPatch` / `CourseHeaderPatch` compare cleanly (or normalize `-0.0` if ever encountered). |
| **RISK-03** | `crates/engine/src/lib.rs:238, 358` (`reroute_connectors`) | `reroute_connectors` mutates glued connectors AFTER `(cmd.run)` succeeds and BEFORE recording the undo step. In `SingleSlide` mode, `reroute_connectors_in_slide` MUST run BEFORE computing `SlideDelta::diff(before_slide, after_slide)` so connector rerouting is captured inside the same undo delta (matching today's exact behaviour!). |
| **RISK-04** | `crates/engine/src/lib.rs:234` (`skip_undo` vs `reroute_connectors`) | Today, when `unchanged` is true (`skip_undo || self.course == snapshot`), `reroute_connectors` is NOT called and `redo` is NOT cleared. Preserve this exact ordering in both `SingleSlide` and `FullCourse` paths! |
| **RISK-05** | `crates/engine/src/lib.rs:246-248` (`valid_at_revision` cache) | `valid_at_revision` caches `is_playable()` for `self.revision`. On `undo()` / `redo()`, `valid_at_revision` must still be cleared to `None`, and on a successful command where `validated_in_cmd == true`, updated to `Some((self.revision, true))`. |
| **RISK-06** | `crates/ui-egui/src/lib.rs:3838-3845` & `guard_tests.rs:204-208` | `Editor::run` currently fakes `guard_counters::note_course_copy()` for every `cmd.mutates` command regardless of whether `Session` actually cloned `Course`! In `S8`, replace that synthetic counter with the real `coursecraft_engine::test_counts::full_course_clones()` counter and update `moving_an_object_redraws_only_that_slide_thumbnail` to assert **`0`** full-course clones. |
| **RISK-07** | `crates/engine/src/lib.rs:395-415` (`undo_steps` / `redo_steps` atomicity) | If `undo_steps(n)` fails mid-loop (impossible under valid invariants, but defensively checked), rollback must not leave a half-undone course. Validate `n <= self.undo.len()` up front and verify `apply` is infallible on internally produced deltas. |
| **RISK-08** | `crates/engine/src/lib.rs:71` (`redo` stack cleared on new edit) | Confirm a no-op / `skip_undo` command does **not** clear `self.redo`, while any actual mutating edit clears `self.redo` (preserving `m27_undo_clipboard_tests.rs`). |
| **RISK-09** | `crates/engine/Cargo.toml` (`debug_assertions` test counter) | Remember `docs/performance.md:222`: when `coursecraft-ui-egui` runs `cargo test`, its dependency `coursecraft-engine` is compiled with `debug_assertions`, NOT `cfg(test)`. Gate `coursecraft_engine::test_counts` on `#[cfg(debug_assertions)]` (just like `coursecraft_render::media::test_counts`), so release builds compile it out completely while `crates/ui-egui/src/guard_tests.rs` can read it! |

---

## 8. Concrete Test Plan

### 8.1 Delta Unit & Round-Trip Tests (`U-01`–`U-07`)

| ID | Crate / File | Behaviour Tested | Expected Result |
|---|---|---|---|
| **U-01** | `crates/engine` (`delta.rs` tests) | Single-object modify / insert / remove / reorder on base layer & slide layers | `SlideDelta::diff` produces `ObjectModified` / `ObjectInserted` / `ObjectRemoved` / `ObjectReordered`; `apply` restores exact `before` JSON; second `apply` restores exact `after` JSON. |
| **U-02** | `crates/engine` (`delta.rs` tests) | Every single one of the 15 `Slide` fields | Mutating each `Slide` field individually produces `SlideDelta::Fields` (or object/layer variant) and roundtrips `undo → redo` byte-identically. |
| **U-03** | `crates/engine` (`delta.rs` tests) | Every single one of the 17 `Course` header fields | Mutating each `Course` header field individually produces `CourseDelta::Header` and roundtrips `undo → redo` byte-identically. |
| **U-04** | `crates/engine` (`delta.rs` tests) | Structural scene/slide/master/template/bank ops | Slide insert, slide delete (with cross-slide trigger cleanup), slide move across scenes, scene add/delete, master add/delete, template add/delete, bank slide move all produce compact `CourseDelta::Patch` (not `FullCourse`!) and roundtrip byte-identically. |
| **U-05** | `crates/engine` (`delta.rs` tests) | Wholesale fallback (`FullCourse`) | Mutating > 50% of slides (e.g. `course.set_slide_size` on 10-slide course) falls back cleanly to `CourseDelta::FullCourse` and roundtrips byte-identically. |
| **U-06** | `crates/engine` (`delta.rs` tests) | Bounds-checked hostile `CourseDelta::apply` (`M33-10`) | Applying a synthetically constructed out-of-bounds `SlideLoc`, `layer`, `index`, or `from/to` returns `Err(EngineError::Invalid(_))` with **0 panics** and leaves `course` untouched. |
| **U-07** | `crates/engine` (`delta.rs` tests) | Step merging (`M33-07`) | Merging 5 consecutive `ObjectModified` deltas on the same object collapses to 1 `ObjectModified` (constant size!); merging mixed deltas (`ObjectModified` + `Header`) forms `Sequence` that undoes to pre-first JSON and redoes to post-last JSON byte-identically. |

### 8.2 Differential Oracle & Scope-Isolation Tests (`DF-01`–`DF-04`, `SC-01`–`SC-03`)

| ID | Crate / File | Behaviour Tested | Expected Result |
|---|---|---|---|
| **DF-01** | `crates/engine` (`m27_all_commands_undo_tests.rs`) | Differential test across ALL 186 `REGISTRY` commands (`M33-01`, `M33-02`) | Run every mutating command on both delta `Session` and legacy snapshot oracle: after execute, after `undo()`, and after `redo()`, `Course::to_json()` is byte-for-byte identical, `undo_names()` / `redo_names()` match, and `revision` matches. |
| **DF-02** | `crates/engine` (`tests.rs`) | Differential test on randomized 250-step mixed edit/batch/undo/redo/undo_steps/redo_steps/eviction sequence | Covers `MAX_HISTORY = 200` eviction, `history_truncated()`, `undo_steps(n)` / `redo_steps(n)`, `skip_undo`, failed commands, and failed batches — byte-identical JSON at every single step! |
| **DF-03** | `crates/engine` (`connector_tests.rs`) | Glued connector rerouting inside undo/redo (`RISK-03`) | Moving a shape with a glued connector (`SingleSlide` scope) reroutes the connector in the same undo step; `undo()` restores both shape and original connector points byte-identically; `redo()` restores both moved shape and rerouted points. |
| **DF-04** | `crates/engine` (`bughunt_undo_tests.rs`) | Failed commands, empty history, and hostile huge/negative frames | Exact parity with legacy behaviour; zero undo steps added on failure; course untouched on rollback. |
| **SC-01** | `crates/engine` (`m27_all_commands_undo_tests.rs`) | Scope-isolation verifier (`M33-03`, `RISK-01`) | Every command annotated `SingleSlide` or `CourseHeader` in `REGISTRY` is executed with the `#[cfg(test)]` scope verifier enabled — asserts zero bytes changed outside the declared scope (both on success and on error rollback!). |
| **SC-02** | `crates/engine` (`tests.rs`) | Same-slide batch fast-path (`M33-08`) | `execute_batch` of 5 `SingleSlide` commands on the same slide takes **0** full-course clones and undoes/redoes atomically in 1 step; a failing 4th step rolls back steps 1–3 byte-identically. |
| **SC-03** | `crates/engine` (`tests.rs`) | Missing/invalid `"slide"` param on `SingleSlide` command | Safely falls back without panic, returns `Err(NotFound)` / `Err(BadParams)`, leaves course untouched. |

### 8.3 Guard & Release Performance Tests (`GD-01`–`GD-03`, `PF-01`–`PF-04`)

| ID | Crate / File | Behaviour Tested | Expected Result |
|---|---|---|---|
| **GD-01** | `crates/ui-egui` (`guard_tests.rs`) | `moving_an_object_redraws_only_that_slide_thumbnail` (`M33-08`, `RISK-06`) | Uses real `coursecraft_engine::test_counts::full_course_clones()`: arrow-key nudge on a slide triggers **0** full-course clones (down from 1!). |
| **GD-02** | `crates/engine` (`tests.rs`) | `two_hundred_edits_on_300_slide_course_zero_full_clones_and_small_deltas` (`M33-08`, `M33-09`) | 200 `object.set_frame` edits on `perf300`: `full_course_clones() == 0`, `session.undo_len() == 200`, `session.undo_estimated_bytes() < 512 * 1024` (< 512 KB for all 200 steps!), and `undo_steps(200)` restores original `perf300` JSON byte-for-byte! |
| **GD-03** | `crates/engine` (`tests.rs`) | `panic_hunt` + hostile `CourseDelta` fuzz (`M33-10`) | Existing `panic_hunt` passes across all 186 commands with zero panics and zero scope violations. |
| **PF-01** | `crates/engine` (`undo_perf_tests.rs`, ignored release) | `perf300` 200 edits + 200 undo + 200 redo + eviction (`§5`) | Edit median `<= 0.60 ms`, undo/redo median `<= 0.05 ms`, RSS/heap growth `<= 2 MB` (down from `392 MB`), `undo_estimated_bytes() <= 512 KB`. |
| **PF-02** | `crates/engine` (`undo_perf_tests.rs`, ignored release) | `rich300` 200 edits + 200 undo + 200 redo (`§5`) | Edit median `<= 2.0 ms` (down from `10.45 ms`), undo/redo median `<= 0.05 ms`, heap growth `<= 4 MB` (down from `1,900 MB`!). |
| **PF-03** | `crates/engine` (`undo_perf_tests.rs`, ignored release) | `perf300` 200 no-op edits (`§5`) | No-op median `<= 0.50 ms` (down from `1.70 ms`), 0 full-course clones. |
| **PF-04** | `crates/ui-egui` (`perf_tests.rs`, ignored release) | `undo / redo move` & `insert interaction + undo` | Both stay comfortably below budget and `undo / redo move` improves cleanly vs `9.52 ms / 9.85 ms` baseline. |

---

## 9. Numbered Implementer Steps (8 Steps, ~30–45 min each)

> **Reviewer notes (Claude, binding for implementers)**
> - This plan was written against main before M31 and M32 merged; line numbers will have moved. Re-read the code first.
> - Correctness beats speed: when a command's scope is in any doubt (it touches another slide, the course header, masters, banks, variables or connectors on other slides), leave it `FullCourse`. The scope-isolation verifier must run over every annotated command.
> - Keep the shadow differential oracle running in tests through S6; remove it only in S7 after the full workspace passed with zero mismatches.
> - S8 re-measures memory and time with the same method used for the baseline (`coursecraft-cli run` + `/usr/bin/time -l`) and records before/after numbers in the report.

> **Gate cadence:** Run targeted `cargo test -p coursecraft-engine -j 4` on every step. Run full `cargo xtask ci` after **S3, S5, S7, and S8**. Run release `undo_perf_tests` and `cargo xtask perf` at **S1** (confirm baseline) and **S8** (prove improvement).

- **S1 — Engine Full-Course-Clone Counter & Ignored Release Undo Benchmark**
  - **Files:** `crates/engine/src/test_counts.rs` (new, `#[cfg(debug_assertions)]`), `crates/engine/src/lib.rs`, `crates/engine/src/undo_perf_tests.rs` (new, `#[cfg(test)]`).
  - **Exact changes:**
    1. Add `#[cfg(debug_assertions)] pub mod test_counts` with thread-local `Cell<u64>` counter `FULL_COURSE_CLONES` (`note_full_course_clone()`, `full_course_clones()`, `reset()`), following `coursecraft_render::media::test_counts` (`RISK-09`).
    2. Call `test_counts::note_full_course_clone()` at the existing `self.course.clone()` sites in `Session::execute` and `Session::execute_batch_with_name`.
    3. Add ignored release benchmark `undo_perf_tests.rs` building `perf300` and `rich300`, timing 200 edits, 200 undos, 200 redos, 100 post-cap evictions, and 200 no-ops, and printing RSS delta and per-op median/p95 timings.
    4. Run `cargo test --release -p coursecraft-engine undo_perf_tests -- --ignored --nocapture --test-threads=1` to confirm baseline numbers match §2.
  - **Tests:** `PF-01`, `PF-02`, `PF-03` (baseline capture).
  - **Done when:** Counter compiles cleanly in debug & compiles out in release, and baseline release benchmark runs green.

- **S2 — Implement Reversible Structural Delta Module (`crates/engine/src/delta.rs`)**
  - **Files:** `crates/engine/src/delta.rs` (new), `crates/engine/src/lib.rs` (`mod delta;`).
  - **Exact changes:**
    1. Implement `SlideLoc`, `SlideFieldPatch`, `SlideDelta`, `ListOp`, `SceneSlideOp`, `CourseHeaderSnapshot`, `CourseHeaderPatch`, `CoursePatch`, and `CourseDelta` per §4.1.
    2. Implement `SlideLoc::find(course, slide_id)`, `slide_at(course, loc)`, `slide_at_mut(course, loc)`.
    3. Implement exhaustive `SlideDelta::diff(before: Slide, after: &Slide) -> Option<SlideDelta>` and `CourseDelta::diff(before: Course, after: &Course) -> Option<CourseDelta>` (destructuring `Course` and `Slide` without `..` for compile-time coverage, `M33-11`).
    4. Implement symmetric in-place `CourseDelta::apply(&mut self, course: &mut Course) -> Result<()>` using only `.get()` / `.get_mut()` bounds checks (zero panics, `M33-10`).
    5. Implement `CourseDelta::estimated_bytes(&self) -> usize` (for `GD-02` memory-budget assertions).
    6. Write unit tests `U-01`–`U-06`.
  - **Tests:** `U-01`–`U-06`, `cargo test -p coursecraft-engine -j 4 delta`.
  - **Done when:** All delta unit tests pass with byte-identical JSON roundtrips and zero panics on hostile inputs.

- **S3 — Shadow Differential Oracle Across All 186 Commands (Old Path Still Primary)**
  - **Files:** `crates/engine/src/lib.rs`, `crates/engine/src/m27_all_commands_undo_tests.rs`, `crates/engine/src/tests.rs`.
  - **Exact changes:**
    1. Keep `Session.undo: Vec<(Course, String)>` as the primary path unchanged for release builds.
    2. Under `#[cfg(test)]`, compute `CourseDelta::diff(snapshot.clone(), &self.course)` on every committed single command and batch, apply it to a copy of `self.course` to verify `delta_undone.to_json() == snapshot.to_json()` AND re-apply to verify `delta_redone.to_json() == self.course.to_json()`!
    3. Add `DF-01` (explicit differential runner over `m27_all_commands_undo_tests.rs`) and `DF-02` (250-step mixed sequence with eviction, `undo_steps`, `redo_steps`, `skip_undo`, and rollbacks).
  - **Tests:** `DF-01`, `DF-02`, `DF-03`, `DF-04`.
  - **Gate #1:** `cargo xtask ci` (every existing test across the entire workspace now proves `CourseDelta` matches snapshot undo byte-for-byte!).
  - **Done when:** 100% of commands, batches, and workspace tests pass the shadow differential oracle with zero mismatches.

- **S4 — Switch `Session` Primary Storage to `VecDeque<(CourseDelta, String)>`**
  - **Files:** `crates/engine/src/lib.rs`.
  - **Exact changes:**
    1. Replace `undo: Vec<(Course, String)>` and `redo: Vec<(Course, String)>` with `undo: VecDeque<(CourseDelta, String)>` and `redo: VecDeque<(CourseDelta, String)>`.
    2. On commit, push `(delta, label)` to `self.undo` and evict with O(1) `self.undo.pop_front()` when `self.undo.len() > MAX_HISTORY`.
    3. Update `Session::undo` and `Session::redo` to pop `(mut delta, name)`, call `delta.apply(&mut self.course)?`, and push `(delta, name)` onto the opposite stack (zero clones!).
    4. Expose `pub fn undo_estimated_bytes(&self) -> usize` (sum of `delta.estimated_bytes()` across undo + redo) for memory assertions.
    5. Keep a `#[cfg(test)]` shadow check only in the dedicated differential test helper (`DF-01`/`DF-02`), so normal `Session` runs lean.
  - **Tests:** `DF-01`–`DF-04`, `cargo test -p coursecraft-engine -j 4`, `cargo test -p coursecraft-ui-egui -j 4 undo`.
  - **Done when:** Primary undo/redo storage uses `VecDeque<(CourseDelta, String)>` everywhere and all engine & UI undo tests pass.

- **S5 — Scoped Pre-Edit Snapshot (`UndoScope`), Per-Slide Connector Rerouting & Scope-Isolation Verifier**
  - **Files:** `crates/engine/src/connectors.rs`, `crates/engine/src/commands.rs`, `crates/engine/src/lib.rs`, `crates/engine/src/tests.rs`.
  - **Exact changes:**
    1. Extract `pub(crate) fn reroute_connectors_in_slide(slide: &mut Slide)` in `connectors.rs` (`RISK-03`), called by `reroute_connectors` for each slide and by `Session` for `SingleSlide` commands.
    2. Add `pub(crate) scope: UndoScope` to `Command` in `commands.rs`, initially setting `UndoScope::FullCourse` on all commands except a small pilot set (`object.set_frame`, `object.update`, `text.set`, `course.set_title`).
    3. Implement the three execution paths in `Session::execute`:
       - `SingleSlide`: look up `SlideLoc` from `params["slide"]` (falling back to `FullCourse` if missing/not found, `SC-03`); clone only 1 `Slide`; run command; on `Err`, restore 1 `Slide`; on `Ok`, check `skip_undo || after == before`; run `reroute_connectors_in_slide`; diff 1 `Slide` into `CourseDelta::SlideChange`.
       - `CourseHeader`: snapshot `CourseHeaderSnapshot` (zero slides cloned!); run command; on `Err`, restore header; on `Ok`, diff header into `CourseDelta::Header`.
       - `FullCourse`: existing temporary-clone → `CourseDelta::diff` → drop clone path.
    4. Implement the same-slide batch fast-path in `execute_batch_with_name` (when every mutating step is `SingleSlide` on the same `slide` ID) and remove the redundant second full-course `step_snapshot` clone (`crates/engine/src/lib.rs:326`).
    5. Add `#[cfg(test)]` **scope-isolation verifier**: in `Session::execute`, when `scope != FullCourse`, capture a verification snapshot in `#[cfg(test)]` (only when a thread-local `VERIFY_SCOPE` flag is active in `SC-01`, or always in `coursecraft-engine` unit tests — except in `GD-02` where we test real clone count!) to assert zero bytes outside the declared scope changed!
    6. Add `SC-01`, `SC-02`, `SC-03`.
  - **Tests:** `SC-01`, `SC-02`, `SC-03`, `DF-03`.
  - **Gate #2:** `cargo xtask ci`.
  - **Done when:** Pilot `SingleSlide` / `CourseHeader` commands and same-slide batches take 0 full-course clones, connector rerouting stays atomic, and CI is green.

- **S6 — Annotate All Safe `SingleSlide` & `CourseHeader` Commands in `REGISTRY`**
  - **Files:** `crates/engine/src/commands.rs`, `crates/engine/src/m27_all_commands_undo_tests.rs`.
  - **Exact changes:**
    1. Annotate all ~125 single-slide commands (`object.*`, `text.*`, `table.*`, `layer.*`, `trigger.*`, `question.*`, `video.*`, `slide.update/set_*/cue`, `draw.update`) with `UndoScope::SingleSlide`, and all ~22 header-only commands (`course.set_title/update/set_player/set_quiz/set_publish_settings/set_background_audio/set_theme`, `theme.*`, `course.add_font/remove_font`, `text_style.create/update/delete`, `view.*`, `resource.*`, `glossary.*`, `variable.add/set_value`) with `UndoScope::CourseHeader`.
    2. Keep every cross-slide or structural command (`slide.add/delete/move/duplicate/paste/add_from_layout`, `scene.*`, `master.*`, `bank.*`, `template.*`, `draw.insert`, `results.insert`, `certificate.insert`, `course.set_slide_size/recolor/replace/import_xliff/import_outline`, `variable.delete`, `text_style.apply`, `interaction.insert` if it touches variables) as `UndoScope::FullCourse`.
    3. Run `m27_all_commands_undo_tests.rs` + `panic_hunt` + `tests.rs` with the scope-isolation verifier active (`SC-01`, `RISK-01`); any command that touches out-of-scope state is immediately caught and kept `FullCourse`.
  - **Tests:** `SC-01`, `DF-01`, `GD-03`, `cargo test -p coursecraft-engine -j 4`.
  - **Done when:** All ~145 hot commands run with 0 full-course clones and 100% pass the scope-isolation verifier and `m27_all_commands_undo_tests`.

- **S7 — Undo Step Merging & Legacy Shadow Cleanup**
  - **Files:** `crates/engine/src/delta.rs`, `crates/engine/src/lib.rs`, `crates/engine/src/tests.rs`.
  - **Exact changes:**
    1. Implement `CourseDelta::merge(older: CourseDelta, newer: CourseDelta) -> CourseDelta` (O(1) collapse for repeated `ObjectModified` / `FullSlide` on the same target; `Sequence` composition otherwise) and `Session::merge_last_two_undos(&mut self, label: Option<&str>) -> bool`.
    2. Add `U-07` verifying byte-identical undo/redo after merging repeated edits and mixed edits.
    3. Remove any leftover temporary legacy shadow fields from `Session` (keep only the clean test-only differential helper in `m27_all_commands_undo_tests.rs` / `tests.rs`).
  - **Tests:** `U-07`, `DF-01`, `DF-02`.
  - **Gate #3:** `cargo xtask ci`.
  - **Done when:** Step merging works and is tested, no legacy full-snapshot history code remains in `Session`, and CI is green.

- **S8 — Real Engine Guard Tests, Release Perf Proof, Docs & Final M33 Gate**
  - **Files:** `crates/engine/src/tests.rs`, `crates/engine/src/undo_perf_tests.rs`, `crates/ui-egui/src/{lib,guard_counters,guard_tests}.rs`, `docs/performance.md`, `docs/testing/m33-report.md` (new report).
  - **Exact changes:**
    1. Add `GD-02` (`two_hundred_edits_on_300_slide_course_zero_full_clones_and_small_deltas`) in `crates/engine/src/tests.rs`.
    2. In `crates/ui-egui/src/lib.rs:3838-3845, 3881-3884`, remove the synthetic `note_course_copy()` calls and wire `guard_counters::course_copies()` (and `reset()`) directly to `coursecraft_engine::test_counts::full_course_clones()` (`RISK-06`).
    3. Update `moving_an_object_redraws_only_that_slide_thumbnail` (`guard_tests.rs:204-208`) to assert **`gc::course_copies() == 0`** (`GD-01`)!
    4. Run release benchmarks (`cargo test --release -p coursecraft-engine undo_perf_tests -- --ignored --nocapture --test-threads=1` and `cargo test --release -p coursecraft-ui-egui perf_tests -- --ignored --nocapture --test-threads=1`) and verify every §5 budget is met.
    5. Update `docs/performance.md` (guard test table + new undo memory/time section) and write concise `docs/testing/m33-report.md` with exact before/after numbers.
  - **Tests:** `GD-01`, `GD-02`, `GD-03`, `PF-01`–`PF-04`.
  - **Final Release Gate:** `cargo xtask ci`, `cargo xtask build-player`, `cargo xtask perf`.
  - **Done when:** All M33 budgets, differential tests, guard tests, WASM builds, and release perf suites pass.

---

## 10. Release Gate for M33

### 10.1 Automated Release Gate Commands
Run in order before cutting the release:
1. `cargo xtask ci` (`fmt --check`, `clippy --workspace --all-targets -D warnings`, `test --workspace`, `layers`, `wasm`)
2. `cargo xtask build-player` (confirm embedded WASM web player builds cleanly)
3. `cargo test -p coursecraft-engine -j 4 m27_all_commands_undo` (all 186 commands pass delta undo/redo + scope isolation)
4. `cargo test -p coursecraft-ui-egui -j 4 guard_tests` (confirms **0** full-course copies on object nudge)
5. `cargo test --release -p coursecraft-engine undo_perf_tests -- --ignored --nocapture --test-threads=1` (confirms §5 memory and time budgets on `perf300` and `rich300`)
6. `cargo xtask perf` (confirms full 300-slide editor + big-course suite stays green and `undo / redo move` improves)

### 10.2 Owner Manual Verification Checklist (Desktop App Only — 8 Minutes)
1. **Everyday editing + Undo/Redo:** Open `examples/showcase/` (or a large course). Move an object with arrow keys, edit text inline, change a shape's fill and shadow, add a state item, delete an object, duplicate a slide, and resize the slide (`Slide size…`). Press `⌘Z` / `Ctrl+Z` repeatedly back to the start and `⇧⌘Z` / `Ctrl+Y` forward — verify every single step restores cleanly with zero glitches.
2. **History window jump:** Open **Edit ▸ History…**, make 10 edits across slides and course settings (including a trigger and a glossary term), click back 6 steps in one click, then click forward 4 steps — verify canvas, inspector, and slide list stay perfectly in sync.
3. **Glued connectors:** Draw two shapes and connect them with a glued connector. Drag one shape so the connector reroutes. Press `⌘Z` / `Ctrl+Z` — verify both the shape AND the connector snap back together in one undo step; press Redo — both move and reroute together.
4. **Error rollback & no-ops:** Try a no-op (e.g. set an object's opacity to its current value, or sort an already-sorted glossary) — verify **Edit ▸ Undo** does NOT add a useless step and does not clear an existing Redo stack.
5. **Large-course memory check:** Open a large multi-slide course (100+ slides) while watching Activity Monitor / Task Manager memory, nudge objects ~50 times, and verify app RAM stays virtually flat (climbing by < 1–2 MB rather than hundreds of MB).
