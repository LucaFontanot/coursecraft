# M45: Quick wins for 0.5.2

Goal: ship the remaining one-step quick wins from the everyday-user research
(`docs/testing/next-research.md` §7, copied below where needed) as release 0.5.2.
Already done on main and NOT part of this milestone: media read errors, bare
`course.json` sibling media, the Theme dialog font picker, the On Click tooltip.
Check Course already exists (File › Check Course), so it is dropped.

Rules for every step: AGENTS.md golden rules (never crash, everything is a command,
one runtime, layering, wasm stays working, clean-room wording, `tl!` for every
user-facing string). Editor UI changes update `docs/ui/` status tables
(`ribbon.md` / `contextual-tabs.md` and `feature-parity.md` with its summary counts)
in the same step. Add a `CHANGELOG.md` line under the existing `## Unreleased` heading
for each user-visible change (plain words, no internal ids).

## Steps

1. **Insert › Media › Caption opens the callout shapes.**
   `crates/ui-egui/src/ribbon/spec.rs` has `insert.caption` as `NotYet("callout shapes")`.
   Callout shapes already exist (`ShapeCategory::Callouts`, `crates/course/src/shapes.rs`).
   Make the button open the shapes picker filtered to (or scrolled to) the Callouts
   category, then drag on the slide like any shape. Reuse the existing shapes menu
   code; do not add a new picker. Tests: the spec item is enabled; the action
   opens the shapes menu with the callouts category. Docs: ribbon.md row ✅,
   feature-parity.md row and counts.

2. **Web player loading indicator.**
   `apps/coursecraft-player/index.html` shows a blank dark page until the wasm starts.
   Add a pure HTML/CSS centred spinner and "Loading course…" text, removed (or hidden)
   once the player starts: the player's start-up code removes the element by id when
   the first frame is ready (look in `apps/coursecraft-player/src/` for where the
   eframe web runner starts). Respect `prefers-reduced-motion` (no spin, text only).
   If start fails, replace the text with a short error message instead of spinning
   forever. Must also apply to published courses (check how export copies or
   generates `index.html` in `crates/export`; change that template too if it is a
   separate copy). Then run `player_build`.

3. **Spell check follows the course language.**
   `crates/ui-egui/src/spell_check.rs` always checks English (`crates/spell/dictionaries`
   has only en_US). When `course.language` (`crates/course/src/model.rs`, BCP 47 tag)
   is set and does not start with `en`, skip the as-you-type squiggles and show a
   quiet note in the Check Spelling dialog: "Spelling is checked in English only. This
   course's language is <tag>." (through `tl!`). No language or `en*` keeps today's
   behaviour. Tests for: no language, `en-GB`, `fr-CA`.

4. **`.ccraft` file association on Windows and Linux.**
   - `packaging/windows/coursecraft.iss`: register `.ccraft` for the current user
     (`HKCU` / `HKA` with `PrivilegesRequired=lowest`), "CourseCraft project" type,
     icon from the exe, open command `"{app}\coursecraft.exe" "%1"`, uninsdeletekey,
     and `ChangesAssociations=yes`.
   - Linux: add a shared-mime-info XML `packaging/linux/coursecraft-mime.xml`
     (`application/x-coursecraft-project`, glob `*.ccraft`), add `MimeType=` to
     `packaging/linux/coursecraft.desktop`, and install the XML in the AppImage /
     tar.gz layout (`.github/workflows/release.yml` Linux steps; mention in
     `packaging/linux/README.md`).
   - The app already opens the first non-flag argument (`apps/coursecraft/src/main.rs`
     `run_editor_window`). Check it handles a `.ccraft` path and a path with spaces.
   - macOS is out of scope: a double-click there sends an open-file event, not an
     argument, and the app does not handle that event yet. Note this in the report.

5. **Intel Mac dmg in the release workflow.**
   `.github/workflows/release.yml` builds only the Apple Silicon dmg; 0.5.1's Intel dmg
   was added by hand. In the macOS job, also build
   `cargo build --release -p coursecraft --target x86_64-apple-darwin` with
   `MACOSX_DEPLOYMENT_TARGET=11.0` (needed: Rust's default 10.12 leaves the app
   unable to find `libswiftCoreMedia.dylib`), add the target to the toolchain step,
   and package it as `coursecraft-<ver>-macos-x86_64.dmg`. Give
   `scripts/package-macos.sh` an optional 4th argument for the arch (default
   `uname -m`). Also fix the flaky dmg: `hdiutil create` must pass `-size 200m`
   (without it the image can be undersized, "No space left on device", hidden by
   `-quiet`). Update `README.md` download notes if they say Intel is missing, and
   `docs/release-checklist.md` if it lists the manual Intel step.

6. **Sync `docs/ui/feature-parity.md` with the code.**
   Check these rows against the code and correct status and wording: word bank and
   numeric questions (`crates/course/src/model.rs`), theme heading and body fonts,
   Caption/callouts (after step 1), and any other row this milestone changed.
   Recount the summary numbers. Docs only.

Then a full gate, a player build, and the report `docs/testing/m45-report.md`.

## Reviewer notes (binding)

- Steps 4 and 5 edit `.github/workflows/release.yml`; the driver does not commit
  `.github`, the owner's orchestrator commits it by hand. Keep those edits minimal.
- No new dependencies. No changes to the course model or runtime.
- Do not touch the version number; the release commit sets 0.5.2.
