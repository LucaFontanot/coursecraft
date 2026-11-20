# M23 Week-one authoring and web sharing

Goal: remove the first-week roadblocks testers hit. Start from a template, share a course on any website, give learners a second try, branch on variables, and time slides to narration. Based on Gemini 4 Neon's plan (`target/m23/neon-plan.md`), trimmed in review.

Steps 3, 4 and 5 change play-time behaviour, so each updates `docs/spec/triggers.md` in the same step (golden rule 5). Every new course field has a serde default and is skipped when default, so old courses load and play exactly as before, and `schema_version` stays as it is.

Clean-room: our own wording, layout and icons (Phosphor). Nothing here is near the patent watch list in `docs/contributing.md`.

Trimmed from Neon's plan, for later: "Publish and open in browser" (a wasm player can't fetch files from `file://`, so it needs a local server), `.cctemplate` slide templates, a random-number source for variables (needs a seed saved in suspend data), and the bare `course.json` media fix (not specific enough yet).

## Step 1: Publish to Web (no LMS)

- New `PackageStandard::Web` in `crates/publish/src/lib.rs`: the same file set as SCORM 1.2 but no `imsmanifest.xml` and no SCORM API wrapper need, plus `coursecraft-package.json` with `{"standard":"web"}` (like `Lti` does around line 493).
- Player: `PlayerStandard::Web` in `apps/coursecraft-player/src/load.rs` (`parse_package_standard` accepts `"web"`). It runs with no LMS reporting and shows no "not connected / reporting lost" warning (find where that warning is drawn: `grep -rn` in `apps/coursecraft-player/src` and `crates/render/src/player_frame.rs`). Resume still works through the browser's local storage if the player already supports that for no-LMS runs; if it doesn't, leave resume out and say so.
- Editor: `PublishStandard::Web` in `crates/ui-egui/src/publish.rs`, labelled "Web (any website, no LMS)", with a one-line hint under it: "Upload the folder to any web server. Results are not reported to an LMS."
- CLI: `--standard web` on `coursecraft-cli publish` (find the parser in `apps/coursecraft-cli/src`). Update `docs/cli.md`.
- Tests: publish tests check the file list (no manifest, marker present, `index.html` present); player test for `parse_package_standard(Some(r#"{"standard":"web"}"#))`; a ui-egui test picks Web in the publish dialog.

## Step 2: Lock and hide are saved, plus starter templates

- Today lock, hide, rulers, grid and guides are editor session state (`crates/ui-egui/src/lib.rs` `hidden_objects` / `locked_objects` about line 683, cleared about line 1301; `crates/ui-egui/src/rulers.rs` `guides`). Save lock and hide in the course: `Object.locked: bool` and `Object.editor_hidden: bool` (serde default false, skipped when false) in `crates/course/src/model.rs`. Guides and the grid/ruler toggles go in a small `Course.editor: EditorSettings` struct (default, skipped when default).
- Changes go through engine commands (`object.lock`, `object.hide_in_editor`, `view.guides.*` or whatever ids fit the existing naming in `crates/engine/src/commands.rs`), so they undo like other edits. The editor reads the course fields instead of the old sets; remove the old sets.
- The player and publish ignore `locked` and `editor_hidden` completely: an object hidden in the editor still shows to learners. Add a logic or render test that proves it.
- Templates: four starter courses in `examples/templates/` (Blank, Quick Knowledge Check, Linear Lesson, Branching Scenario), our own content, embedded with `include_str!`. The welcome screen (`crates/ui-egui/src/welcome.rs`) shows them as "Start from a template" cards. Each must pass `validate` with no errors, and the Blank one with no warnings either.
- Tests: lock/hide survive save and reload (serialize round trip) and undo; a hidden object is drawn in the player; each template loads and validates; the welcome screen shows the four cards and clicking one opens that course.

## Step 3: Try Again layer and safe resume (runtime)

- `Question.try_again_layer: Option<String>` (serde default, skipped when None) next to `correct_layer` / `incorrect_layer` (`crates/course/src/model.rs` about line 3740).
- Logic (`crates/logic/src/run.rs`, grep `incorrect_layer`): on a wrong answer while attempts remain and `try_again_layer` is set, show the try-again layer instead of the incorrect layer and don't lock the question. On the last attempt, the incorrect layer shows as today. A new action or the existing hide-layer action on the layer's button closes it and clears the learner's choice so they can answer again (pick whichever fits the current model; describe it in the spec).
- Safe resume: re-entering a question slide that is already answered and locked (after resume, or by going back) shows its outcome layer again (correct or incorrect, by the saved result), so its Continue button is there. This replaces the `[verify]` note in `docs/spec/triggers.md` around line 540 with the decided rule.
- Editor: the question panel (`crates/ui-egui/src/question.rs`) gets an "Add Try Again layer" button when attempts are not 1, which runs one engine command that creates the layer with a message and a "Try Again" button wired to close it. Validation (`crates/course/src/validate.rs`) warns when `try_again_layer` names a missing layer.
- Spec: describe both rules in `docs/spec/triggers.md`.
- Tests (logic): wrong then right with 2 attempts shows try-again, then correct; wrong twice shows try-again then incorrect; with no try-again layer, behaviour is unchanged (an existing test must still pass untouched); resume on an answered question shows its outcome layer. Engine test for the new command, ui-egui test for the button.

## Step 4: Variables compared to variables, text checks that ignore case, and smarter jumps (runtime)

- `SetVariable` and `Condition::Compare` (`crates/course/src/model.rs` about lines 3594 and 3699) can take another variable as the value: add a value-source enum or field in a way that keeps today's JSON loading as a constant (serde untagged or a defaulted field; add a test that an old course loads unchanged).
- `CmpOp`: add `EqIgnoreCase` and `ContainsIgnoreCase` for text.
- `SlideTarget`: add `LastVisited` (the slide the learner came from; keep a short history in `PlayerState`, at most 50 entries; after a resume the history starts empty and `LastVisited` acts like Previous) and `Scene(id)` (first slide of that scene).
- Logic in `crates/logic/src/run.rs`; validation for unknown variables/scenes and type mismatches (comparing a number to a text variable is a warning); the trigger editor (`crates/ui-egui/src/triggers.rs`) offers "a value / another variable", the new operators and the new jump targets; the plain-words trigger sentences (`describe*`) read naturally ("when Score equals Best Score").
- Suspend data must stay compatible: do not add the history to suspend data.
- Spec: `docs/spec/triggers.md` gets each new rule.
- Tests: logic tests for each new option, plus the old-JSON round trip; ui-egui test that builds one variable-to-variable condition in the trigger editor.

## Step 5: Audio waveforms and named cue points on the timeline

- Waveform: compute a small peak envelope (at most 2,000 peaks per clip) when an audio clip loads, cached by media id, off the per-frame path; draw it inside the audio bars in `crates/ui-egui/src/timeline.rs`. Find where audio is decoded (`grep -rn "decode" crates/media/src crates/playback/src`). If no decoder is reachable from ui-egui without a new dependency, draw the waveform only for WAV/PCM and leave a plain bar otherwise; say so.
- Cue points: `Slide.cue_points: Vec<SlideCue { id, name, at_ms }>` (and the same on layers if simple; otherwise slides only), serde default, skipped when empty. Press `C` with the timeline focused to add one at the playhead; drag to move; double-click to rename; Delete removes. All through engine commands.
- Objects' start and end snap to cue points when dragged on the timeline (reuse the timeline's snapping code, grep `snapped_playhead_ms`).
- `Event::TimelineReaches` can name a cue (`cue: Option<String>` next to `ms`, serde default); logic fires it at the cue's time; moving the cue moves the trigger with it. Spec update in `docs/spec/triggers.md`.
- Tests: peak envelope of a synthetic sine has the right length and range; add/move/rename/delete cue undo; a trigger on a cue fires at the cue's time in logic, and after moving the cue fires at the new time.

## Order and checks

One step at a time, in order. After each step: `cargo fmt --all`, clippy `-D warnings` and tests for the touched crates, then commit. Full `cargo xtask ci` after step 2 and step 5.
