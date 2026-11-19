# M21 Optimization pass (release 0.2.x)

Goal: CourseCraft stays quick and cool on a big course, and stays that way. Measure first, fix only what the numbers show, and leave tests behind so it can't slowly get worse again. No new features, no change to the course format or the runtime.

## What we already know (2026-11-19, installed 0.2.0 release build, Apple Silicon)

- Idle with a course open (hello-course, showcase): 0% CPU, about 130 MB memory. Good.
- Welcome screen, idle: short CPU bursts up to about 9% every few seconds, and memory creeping up slowly. Worth finding.
- Existing timing tests, ignored by default because timings only mean something in release:
  - `crates/ui-egui/src/perf_tests.rs`: editor frame times on a 300-slide course (idle, select slide, scroll the slide list, typing in Properties). Budgets: idle frame 16 ms, interaction 33 ms.
  - `apps/coursecraft-cli/tests/big_course.rs`: open, save, unchanged save, publish and memory on a big course with real pictures.
- Not measured yet: the M20 features on a big course (typing with spell check, Check Spelling over the whole course, inserting an interaction and undoing it), start-up time, and memory after visiting every slide (decoded pictures).

## Steps

Each step is one commit. Hy 4 Flash Free implements and Opus reviews, adding any tests Hy4 missed. If Hy4 fails review twice on a step, the step goes to Sonnet 5.5 (medium effort) or Gemini 4 Hydrogen. Quick crate checks after each step, and the full gate after steps 2 and 5. Timing runs happen in release mode, so they go to a manual GitHub workflow where possible, to keep the Mac cool.

### Step 1: one command for the numbers, and a baseline

- `cargo xtask perf` runs every ignored timing test in release and writes `target/perf/report.md`, a table of stage, median, worst (p95) and budget, with over-budget rows marked.
- Add the missing stages to the existing tests:
  - Typing on the canvas in-place editor with spell check on (a 2,000-character paragraph, one letter a frame).
  - Check Spelling: open it and walk the whole 300-slide course.
  - Insert each M20 interaction on slide 150, then undo.
  - Undo and redo a move on slide 150.
  - Visit all 300 slides one after another, recording memory before and after.
  - Time to the first editor frame for the big course.
- A manual GitHub workflow, `perf.yml` (workflow_dispatch only), runs `cargo xtask perf` on a macOS runner and uploads the report.
- `docs/performance.md`: what each stage means, its budget, how to run it, and the baseline table from this step.

### Step 2: nothing moves, nothing runs

- A normal (not ignored) test: after a few settling frames with no input, no playback, no drag and no animation, the editor asks for no repaint. Check the same for the welcome screen, a dialog left open and the learner player. egui's `Context` reports whether a repaint is pending, so this needs no timing.
- Find and fix the welcome-screen bursts and memory creep. Waiting work (autosave, the font scan, background loads) uses `request_repaint_after` with a delay instead of repainting every frame.
- Anything that animates (playback, the loading bar, a hover ring) keeps repainting only while it is visible and moving.

### Step 3: fix the slow stages

The brief is written after step 1's baseline. It covers every stage over budget, worst first. Typical fixes: cache work that is repeated each frame on unchanged data, avoid copying the whole course for a small read, lay out text once and not twice, and limit work to what is on screen. Each fix states its before and after numbers.

### Step 4: start-up and memory

- Start-up: work not needed for the first frame is done later or in the background (scans, catalogues, large tables).
- Memory: decoded pictures and video frames for slides that aren't shown are freed after a while and decoded again when needed, if step 1 shows memory growing with every slide visited. Set a ceiling, and test it.
- The brief is finalized after step 1's numbers.

### Step 5: guard rails in the normal test run

Timing tests are too noisy for CI, so these use counts instead of clocks:
- An idle frame copies the course 0 times.
- Spell check runs 0 times on unchanged text.
- Thumbnails aren't redrawn when nothing changed.
- A slide switch decodes only that slide's pictures.

Add a CHANGELOG line in plain words (for example, "The editor uses less power when idle and stays quick on big courses"), with real numbers if they are worth quoting.

## Out of scope

New features, the course format, runtime behaviour (so no `docs/spec/triggers.md` change), and video export speed.

Clean-room: this is all internal engineering on our own code. Nothing is copied from other tools.
