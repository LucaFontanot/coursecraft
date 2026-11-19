# Performance

Measured on the maintainer's Mac (Apple M5, 10 cores, 24 GB, macOS), release build, 2026-11-11.

## The big-course test

`apps/coursecraft-cli/tests/big_course.rs` (ignored; writes about 5 GB under `target/big-course/`, which a guard deletes at the end). It lives in `coursecraft-cli` because the editor app has no library target and this crate already links format, publish, logic and course.

The course: 300 slides in 10 scenes; each slide has a title, 2 text boxes, a picture, a shape and a button with a jump trigger; every 10th slide also has a graded question and a layer. Media: 300 distinct valid PNGs (about 1 MB each, random pixels so nothing compresses away) and 10 distinct 100 MB video blobs: 1362 MB in all.

```sh
cargo test --release -p coursecraft-cli --test big_course -- --ignored --nocapture
# per-stage peak memory (macOS keeps freed pages in the resident count, so run stages apart):
BIG_STAGE=save BIG_KEEP=1 /usr/bin/time -l cargo test --release -p coursecraft-cli --test big_course -- --ignored --nocapture
BIG_STAGE=publish BIG_KEEP=1 /usr/bin/time -l cargo test ... (same)
BIG_STAGE=open /usr/bin/time -l cargo test ... (same; run after `save`)
```

There is no separate HTML5 target; the second publish row is the manifest-free LTI zip, which is the same file set as SCORM 1.2 minus the manifest.

| Stage | Budget | Before | After |
|---|---|---|---|
| validate | 200 ms | 2 ms | 2 ms |
| save `.ccraft` (1362 MB written) | 5 s | 33.8 s | 0.35 s |
| publish SCORM 1.2 | 60 s | 34.9 s | 0.5 s |
| publish LTI zip | 60 s | 34.3 s | 0.45 s |
| open `.ccraft` | 5 s | failed: "archive too large" | 0.5 s |
| simulate all 300 slides | none | 4 ms | 4 ms |
| peak memory, whole run | 3224 MB (2 x media + 500 MB) | 5472 MB | see stages |
| peak memory, save | 3224 MB | - | 1390 MB |
| peak memory, publish | 3224 MB | - | 2756 MB |
| peak memory, open | 3224 MB | - | 2735 MB |

The "after" whole-run figure is not comparable, so it is not given: when the stages share a process, macOS reports pages the program has already freed as still resident. The three stage figures each run alone.

### After M18 (with verify and versions)

A desktop save now also checks the file it wrote and keeps a copy of the file it replaced. Measured on the same course (2026-11-11), save stage run alone:

| Stage | Budget | After M18 |
|---|---|---|
| save `.ccraft` (1362 MB) | 5 s | 0.40 s |
| verify the saved file (streams every entry, CRCs checked) | part of the save | 0.38 s |
| keep a version (copy of the replaced file) | part of the save | under 1 ms here |
| the whole save | 5 s | about 0.8 s |
| peak memory, save | 3224 MB | 1393 MB (the desktop app builds the archive in memory first, about 2.7 GB for this course) |

The version copy is instant on this Mac because the file system (APFS) clones files; on a file system that cannot, it costs a plain disk copy of the project (about as long as writing it). `big_course.rs` prints the verify and copy rows with the save stage. Verification reads the file back, so on a slow or network disk it roughly doubles the time spent reading and writing.

### Project folder (for git)

The same course saved as a folder (`big.ccraft-folder`: `course.json` plus 310 media files), measured with the big-course test on 2026-11-11, release build. The first save writes every file, several at a time, each through a temporary file, a flush and a rename. The second save finds from its record that nothing changed and writes nothing (a hash of each file in memory, no read from disk). Open reads every file and records its size, hash and time.

| Stage | Budget | Folder |
|---|---|---|
| save folder, first (1362 MB, 311 files written) | 5 s | 2.8 s |
| save folder, unchanged (0 files written) | 5 s | 0.27 s |
| open folder | 5 s | 1.4 s |

The first save is slower than the `.ccraft` (0.5 s) because it flushes each of 311 files to disk before renaming it, so a crash leaves a whole file or the old one. Before the files were written in parallel, the first save took 5.3 s and the unchanged save 1.3 s. The check that runs before an app save (is any file different on disk from the record?) compares size and modified time only and reads nothing.

### Editor frame time (300 slides, pictures stubbed)

`crates/ui-egui/src/perf_tests.rs` (ignored), 60 frames each, layout plus tessellation:

```sh
cargo test --release -p coursecraft-ui-egui perf_tests -- --ignored --nocapture
```

| Case | Budget | Average | Worst |
|---|---|---|---|
| idle | 16 ms | 3.07 ms | 3.31 ms |
| select slide 150 | 33 ms | 3.14 ms | 3.43 ms |
| scroll the slide list | 33 ms | 3.07 ms | 3.11 ms |
| type in a text field | 33 ms | 3.18 ms | 3.22 ms |

All within budget with room to spare; nothing needed fixing. Frame times use stubbed media, so real thumbnail decoding cost is not in these numbers.

## What was slow, and the fixes

1. **Deflating already-compressed media.** `save_project` and `zip_package` deflated every file. PNG, JPEG, MP4 and similar do not shrink, so the time was wasted (about 34 s for 1.3 GB). Now pictures, video, audio, fonts and archives are stored (`coursecraft_format::is_precompressed`); text (`course.json`, HTML, JS, captions) is still deflated. Old files, which are all deflated, still open.
2. **Opening a big project failed.** The 512 MiB total and 256 MiB per-entry limits made any course over 512 MB unopenable. Stored entries cannot expand, so they now use the larger limits (4 GiB total, 1 GiB per entry); deflated entries keep the old 512 MiB / 256 MiB caps, so the zip-bomb guard is as strict as before.
3. **Memory copies.** The save and zip buffers are sized once instead of doubling; entries are read into an exactly-sized buffer; `save_project_to` and `zip_package_to` write straight into a file, and the editor's publish now streams the zip to a temporary file and renames it. That removes one whole-package copy from publish (it was about 4.1 GB at peak, over budget).

Still copied once: publish resolves each media file with a copy (`PackageInput::media` returns an owned `Vec`), so publish holds the media twice (2756 MB here, within budget). Removing that needs a borrowed-bytes API and was left alone.

## `cargo xtask perf` (M21)

CourseCraft keeps a set of ignored timing tests on a 300-slide course. They only
mean something in a release build, so they are not part of the normal test run.
`cargo xtask perf` runs all of them and writes two files under `target/perf/`:

- `results.tsv` — one row per stage: `suite`, `stage`, `median_ms`, `p95_ms`,
  `budget_ms`, `note`. One-shot stages use the same number for median and p95;
  memory stages put the resident memory in the note.
- `report.md` — a header (date, OS, CPU count) and a table of suite, stage,
  median, p95, budget and status. A row is marked **over** when its p95 is above
  its budget. The command only fails when a test fails, not when a stage is over.

Timings differ between machines, so compare runs on the **same** machine. The
budgets are guardrails for regressions, not a promise about every laptop.

### What each stage measures

Editor suite (`crates/ui-egui/src/perf_tests.rs`, 1280×900 headless frames):

| Stage | What it measures | Budget |
|---|---|---|
| idle | A frame with no input after settling | 16 ms |
| select slide 150 | Switching between two slides | 33 ms |
| scroll slide list | A wheel event over the slide list | 33 ms |
| typing in a text field | A keystroke in the button-label field in Properties | 33 ms |
| type on canvas with spell check | A keystroke in the in-place editor of a 2,000-character paragraph with misspellings, as-you-type checking on | 33 ms |
| check spelling, open | Opening Check Spelling on the 300-slide course (a misspelling on every 10th slide): collect every issue and draw the window | 250 ms |
| check spelling, ignore step | Ignoring one issue (re-scanning the course) and drawing the next, 20 times | 33 ms |
| insert interaction + undo | For each of the 8 M20 interactions, insert it on slide 150 then undo, timed together | 100 ms |
| undo / redo move | Arrow-key nudge of an object on slide 150, undo and redo, 30 cycles | 33 ms |
| visit every slide | Selecting s-0 … s-299, one frame each; the note records resident memory before and after | 33 ms |
| visit every slide, real pictures | The same 300 slides, but each picture is backed by a real 1280×720 PNG (distinct cache keys, shared source bytes); the note records resident memory before and after and the number of decoded pictures left in the cache | 33 ms |
| first frame | From `TestHarness::new(big_course())` through the first editor frame | 500 ms |

Big-course suite (`apps/coursecraft-cli/tests/big_course.rs`, a real 300-slide
project with pictures and video, about 5 GB of scratch data):

| Stage | What it measures | Budget |
|---|---|---|
| generate | Building the course and its media (not budgeted) | — |
| validate | Validating the course | 200 ms |
| save .ccraft | Writing the project file | 5 s |
| verify saved file | Streaming CRC check of the file | — |
| keep a version (copy) | Copying the file into version history | — |
| save folder, first | First project-folder save | 5 s |
| save folder, unchanged | Re-save with nothing changed | 5 s |
| publish SCORM 1.2 | Building and zipping the package | 60 s |
| publish LTI zip | Building and zipping the package | 60 s |
| open .ccraft | Reading and parsing the project file | 5 s |
| open folder | Reopening the project folder | 5 s |
| simulate all slides | Playing through every slide (not budgeted) | — |

### Running it

Locally, from the repository root:

```sh
cargo xtask perf
```

The big-course test needs at least 15 GB free and takes minutes; scratch files
under `target/big-course/` are deleted afterwards. The editor tests are filtered
with `perf_tests` and the CLI test with `big_course`; both run with `--ignored
--nocapture --test-threads=1`.

On GitHub: **Actions → Perf → Run workflow**. The workflow runs `cargo xtask
perf` on a macOS runner and uploads `target/perf/` as the `perf` artifact.

### Baseline

Editor suite, release build, the maintainer's Mac (Apple M5, 10 cores), 2026-11-19. Times are per frame (median / p95, ms) unless the budget says otherwise. "Before" is 0.2.0; "after" is the M21 step 3 fix (suggestions worked out only when a word menu or the Check Spelling window needs them, then cached).

| Stage | Budget | Before | After |
|---|---|---|---|
| idle | 16 | 3.5 / 3.6 | 2.1 / 2.2 |
| select slide 150 | 33 | 3.6 / 3.6 | 2.1 / 2.1 |
| scroll slide list | 33 | 3.5 / 3.6 | 1.9 / 2.1 |
| typing in a text field | 33 | 3.7 / 3.8 | 2.0 / 2.0 |
| type on canvas with spell check | 33 | **777 / 807** | 0.42 / 0.44 |
| check spelling, open | 250 | 41 | 31 |
| check spelling, ignore step | 33 | 14.7 / 15.7 | 9.5 / 10.0 |
| insert interaction + undo | 100 | 3.6 / 4.1 | 2.0 / 2.2 |
| undo / redo move | 33 | 15.6 / 15.9 | 8.2 / 8.7 |
| visit every slide | 33 | 3.6 / 3.8 (85 → 86 MB) | 1.9 / 1.9 (85 → 86 MB) |
| first frame | 500 | 26 | 13 |

Only the canvas typing change comes from code; the general drop in the other rows is the machine being less busy on the second run. Compare rows within one run.

### Decoded-picture ceiling (M21 step 4)

`coursecraft_render::MediaCache` now limits decoded *picture* textures to 512 MB
on the desktop and 256 MB on wasm (`DEFAULT_PICTURE_BUDGET`). A picture costs
width × height × 4 bytes. When a hand-out would push the total over the ceiling,
pictures last used before the current frame are dropped least-recently-used
first and decoded again when needed. Pictures handed out in the current frame
are never dropped, so a frame may briefly sit over the ceiling; the editor and
the learner player call `MediaCache::begin_frame()` once per frame. SVG rasters,
GIF/WebP frames, Lottie compositions, video textures and failed or pending
lookups have their own lifetimes and are neither counted nor evicted by this
rule. `set_picture_budget(bytes)` overrides the ceiling for tests.

Slide thumbnails outside the scroll viewport are no longer painted, so their
pictures are not decoded just because the rows are laid out. This is invisible
on screen and is what lets the ceiling apply to off-screen slides at all:
without it, every thumbnail is "used this frame" on every frame.

The `visit every slide, real pictures` stage measures the result. Its source
bytes are shared between slides so the resident-memory delta is decoded pixels,
not PNG bytes. Release build, the maintainer's Mac, 2026-11-19:

| Stage | Budget | Before (0.2.0) | After |
|---|---|---|---|
| visit every slide, real pictures | 33 ms | every one of the 300 pictures stays decoded (about 1.1 GB counted as width × height × 4) | 4.2 / 7.0 ms a frame; 81 pictures decoded at the end (about 300 MB), under the 512 MB ceiling |

The headless test harness never uploads textures to a GPU, so its resident memory (98 → 123 MB) understates what a real window holds; the decoded-picture count is the number to watch. The same run's idle frame fell from 2.1 to 1.3 ms, because off-screen thumbnails are no longer painted.

### Start-up (M21 step 4)

Nothing was moved. `Editor::new` builds its state, then `check_revision` runs
`course.validate` and `course.audit` through the engine on the first call; both
walk the whole course. Their counts and findings are drawn on the first frame
(status bar and canvas markers), so deferring them is not clearly safe. On the
desktop, `EditorApp::new_desktop` also reads the preferences file (needed for
the theme and other first-frame settings) and starts the automation control
server (a listener and a thread, whose port is shown in the UI). Both are small
here, and moving the listener could change when automation connects, so they
were left alone. The font scan and the speller are already lazy or background
work and were not touched.
