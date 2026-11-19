# M22 Editor upgrades

Goal: make everyday editing clearer and harder to get wrong. Five upgrades to things CourseCraft already has. Nothing here changes play-time behaviour, so `docs/spec/triggers.md` stays as it is.

Already built, so not in this milestone: the Format Painter (`format_painter.rs`), Distribute (`properties.rs`, `menu.rs`), the plain-words trigger sentences (`triggers.rs` `describe*`).

Clean-room: our own layout, wording and icons (Phosphor, already bundled). Check the patent watch list in `docs/contributing.md` before step 3; equal-spacing guides are common in design tools, but keep to our own drawing and wording.

## Step 1: Undo steps have names (engine)

Today `Session` keeps `undo: Vec<Course>` and `redo: Vec<Course>` with no names, so the editor can only say "Undo".

- Keep a name next to every snapshot: `undo: Vec<(Course, String)>`, and the same for `redo` (or two parallel `Vec<String>`; one vector of pairs is preferred so they can't drift).
- `execute` names the step after the command's `label` (for example "Set fill"). `execute_batch` uses the label of the first step that changed something. Add `execute_batch_named(&mut self, name: &str, steps)` for callers that know a better name; `execute_batch` calls it.
- `undo` and `redo` move the name with the snapshot. `MAX_HISTORY` trimming drops the oldest name too.
- New read-only API: `undo_names(&self) -> Vec<&str>` (newest first), `redo_names(&self) -> Vec<&str>` (next redo first), and `undo_steps(&mut self, n: usize) -> Result<()>` / `redo_steps(&mut self, n)` that undo or redo `n` steps and bump `revision` once per step (as today).
- Files: `crates/engine/src/lib.rs` (struct `Session` about lines 68–80, `execute` about 160–192, `execute_batch` 203–262, `undo`/`redo` 265–277). Tests go in `crates/engine/src/tests.rs`.
- Tests: names follow commands; a batch takes its first changing step's name; a no-change command adds no name; undo/redo move names; trimming at `MAX_HISTORY` keeps names and snapshots in step; `undo_steps(3)` equals three `undo()` calls; `undo_steps` past the end returns the error and leaves the course as it was before the call.
- The Edit menu says "Undo Set fill" / "Redo Set fill" (`crates/ui-egui/src/menu.rs` around line 339 and 871, where `edit.undo` and `edit.redo` are drawn).

## Step 2: History panel (editor)

- Edit ▸ History… opens a small window listing the steps: redo steps greyed at the top, then "Now" marker, then undo steps newest first, and "Opened" at the bottom (the course as it was loaded, or as far back as history goes).
- Clicking a row moves there in one go (`undo_steps` / `redo_steps`). The selection is repaired as after a normal undo (reuse what `edit.undo` does in `menu.rs`).
- The window follows the theme palette (`coursecraft_render::palette`), stays inside the editor window, and scrolls when long.
- Files: new `crates/ui-egui/src/history.rs` (window), `crates/ui-egui/src/lib.rs` (a `show_history: bool` in the UI state and a call next to `fonts::show_fonts_window` near the end of `Editor::show`), `crates/ui-egui/src/menu.rs` (menu item). Use a command id `edit.history` registered in `crates/engine/src/commands.rs` as a non-mutating command with `menu: "Edit/History…"`, like other window-opening commands there.
- Tests (`crates/ui-egui/src/history_tests.rs`, use `TestHarness`): make three edits, open History, see three named rows; click the oldest, the course equals the original and the redo rows show; click a redo row, the course comes back; the window passes `layout_report` with no "clipped" or "overlap" findings at 1100×700.

## Step 3: Equal-spacing guides (editor)

Today's smart guides (`crates/ui-egui/src/snap.rs`) snap edges and centres. Add equal gaps.

- While moving one object (or a selection) between or beside others in the same row or column, snap when the gap to its neighbour equals an existing gap between two other objects on that axis, or equals the gap on its other side.
- Draw each matched gap as a short line with end ticks, plus a small label with the gap in slide pixels (for example "24"), in the accent colour. Labels never cover the moving object's handles.
- Keep it pure: a new function in `snap.rs` takes the moving rect, the other rects and the threshold, and returns an offset plus `SpacingGuide { vertical, from, to, across }` values. `canvas.rs` draws them next to `draw_snap_guides` (about line 1054) and stores them like `editor.ui.snap_guides` (`lib.rs` line ~771).
- Edge/centre snapping still wins when both apply within the threshold; ties go to the smaller movement.
- Files: `crates/ui-egui/src/snap.rs`, `crates/ui-egui/src/canvas.rs` (`snap_targets` about line 1004, the move code that calls the snap functions, `draw_snap_guides`), `crates/ui-egui/src/lib.rs` (new field).
- Tests in `snap.rs`: three boxes in a row with equal gaps snap; unequal gaps do not; vertical columns work; a non-finite rect is ignored; edge snap beats spacing snap. One harness test drags a box between two others and checks the final x.

## Step 4: Problems shown on triggers (editor)

`crates/course/src/validate.rs` already finds broken triggers (paths like `/scenes/0/slides/2/triggers/1` and `/scenes/0/slides/2/layers/0/triggers/3`), but they appear only in the course check list.

- The Triggers tab marks each trigger that has an issue with a warning icon (errors in the danger colour, warnings in the warning colour) and a tooltip with the issue's message in plain words. The trigger's sentence stays readable.
- A one-line note above the list: "2 triggers on this slide need attention" with a button that selects the first one and opens its editor.
- Run `course.validate()` only when the course revision changes, and cache the issues by path (no validation per frame; add a count-based guard test like the ones in `guard_tests.rs`).
- Files: `crates/ui-egui/src/triggers.rs` (`show_triggers_tab` about line 1386 and the row drawing below it), `crates/ui-egui/src/lib.rs` (cached issues next to `validation_issues`, about line 607), `crates/course/src/validate.rs` only if an issue lacks a usable path (read-only otherwise).
- Tests: a trigger pointing at a deleted object shows the icon and tooltip; fixing it removes the icon; a course with no problems shows no note; validation runs once per revision (guard test).

## Step 5: Live variables in Preview (editor)

While previewing, authors can't see why a branch went one way. Show the course variables as they change.

- A "Variables" toggle in the preview window (next to the effect-log toggle; F3 as its shortcut, like F2 for the log) opens a side list: each course variable's name, its live value from `PlayerState.variables`, and its type. A value that changed in the last second is highlighted, then fades.
- Read-only: the panel never writes to the player state, so the runtime and `docs/spec/triggers.md` stay unchanged.
- Variables with `%var%` text references are listed first; long text values are cut with an ellipsis and shown in full in the tooltip.
- Files: `crates/ui-egui/src/preview.rs` (`Preview` struct about line 108, its constructors around lines 277, 317 and 334 where `show_log` is set, the drawing code around line 470–520 where the log toggle and F2 are handled). Read values from the resumed `Player` (`Player::variable`, `crates/logic/src/lib.rs` about line 1201) or from `state.variables`; do not change `coursecraft-logic`.
- Tests (in `crates/ui-egui/src/tests.rs` or a new `preview_variables_tests.rs`): preview a course with a number variable and a button that adds 1; open the panel, click the button, see the new value; the panel stays inside the preview window (`layout_report`).

## Order and checks

Steps run one at a time in this order (2 needs 1; 3, 4 and 5 are independent but share `lib.rs`, so they also run one after another). After each step: `cargo fmt --all`, then clippy `-D warnings` and tests for the crates it touched; commit when those pass. Full `cargo xtask ci` after step 2 and at the end.
