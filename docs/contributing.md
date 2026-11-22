# Contributing

## Gate

`cargo xtask ci` must pass: fmt, clippy with `-D warnings`, tests, layers.

## Add a command

1. Write `fn my_cmd(s: &mut Session, p: &Json) -> Result<Json>` in `crates/engine/src/commands.rs`. Parse params with `req`/`opt`. Return `Err`, never panic.
2. If it mutates, set `mutates: true` and call `keep_valid(s, was_valid)` before returning.
3. Add a row to `REGISTRY` (id `area.verb`, label, menu path, shortcut, params doc).
4. Add a test. `panic_hunt` covers hostile params automatically.
5. Add its line to the command list in `docs/cli.md`, from `coursecraft-cli commands`. A test holds this.

## Add a trigger action or event

1. Add the variant in `crates/course/src/model.rs`, its validation in `validate.rs`, and its behaviour in `crates/logic/src/run.rs`.
2. Document it in `docs/spec/triggers.md` (mark **[verify]** until checked by observation).
3. Add a runtime test that drives it through `Player::handle`.

## Add or wire a UI control

The editor's layout is specified in `docs/ui/` (start at `docs/ui/README.md`).

1. Find the control in `docs/ui/ribbon.md` or `docs/ui/contextual-tabs.md`. If it is not there,
   add it first, in the place the tutorials show (`docs/ui/storyline-observed.md`).
2. If it edits the course, add the engine command first (above). UIs only dispatch.
3. In the ribbon spec, change its action from `NotYet(..)` to the command or UI action. A control
   whose feature is missing stays in place, drawn disabled with its reason.
4. Update its status in the doc table and the feature's row and summary in
   `docs/ui/feature-parity.md`.
5. Labels follow the naming table in `docs/ui/README.md` › Clean-room.
6. Every user-facing string is English in the code and goes through `tl!` (or `tid`, `trn`,
   `fmt`); never concatenate sentences. Translations go to the craft-i18n data repository, not
   here (`docs/ui/localization.md`).
7. Use `craft-ui` components (panes, docking, tabs, buttons, combos, number fields) before writing
   a new widget; generic additions go upstream to craft-libs.

## Clean-room rules

- Study other authoring tools for **behaviour only**: what happens when you click, the order triggers fire in. Design our own look and layout from `DESIGN.md`.
- Never copy another product's code, player JS/CSS, icons, templates, fonts, characters or other assets (Articulate, Adobe or anyone else). Never decompile or unpack their players.
- Use our own names in the UI. Generic terms (layer, trigger, state, lightbox, seekbar) are fine; a competitor's coined terms are not (we renamed "Story view" to "Course map" and "Freeform" to "Custom question").
- In docs, competitor names may describe comparable features ("what a Storyline author expects"), never "copied from" or "in the style of". The README carries the trademark notice.
- Build parity test cases from courses **you made yourself**. Never use client or employer courses.
- Standards come from public specs: ADL SCORM 1.2/2004, xAPI, cmi5, WCAG.
- `.story` import touches a proprietary format and stays on hold until a legal check.
- Third-party assets must be permissively licensed, keep their licence file next to them, and get a row in `ATTRIBUTION.md`.

### Patent watch list

A patent check on 2026-11-11 found nothing that CourseCraft as built practises. Before building any of these, read the claims of the listed patents and design around them:

- **SCORM data:** use only data-model elements the SCORM, xAPI and cmi5 specs define. KnowBe4 patents content that asks the LMS for undefined elements (US10949760, US11599810, US11847579).
- **Automatic course from documents:** an AI pipeline that splits uploaded documents into blocks, summarises them, scores keywords and assembles a course is close to Docebo US11720617B2. Drafting from the author's outline, or the author's own agent reading a file, is fine.
- **Screen recording and software simulation (M20):** Assima patents copying a running app's screens for training (US9570050B2, US12360646B2). Adobe's screenshot-and-mouse-path patent (US7086032B2) expired in 2024.
- **Learner classification and adaptive content:** machine learning that classifies learners and reorders content is Adobe US10911553B2 (to 2038); knowledge tracing with hints is Adobe US10943497B2.

## Style

- Never panic on input. Use `get()` instead of indexing for input-derived indices, use checked/saturating arithmetic, and bound recursion.
- Write tests first for formats and runtime rules. One behaviour per test, named after the behaviour.
