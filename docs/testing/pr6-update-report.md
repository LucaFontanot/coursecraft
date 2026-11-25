# PR #6 ("UI parity") Update Report — Merge `main` (0.4.6 / M37)

**Branch:** `pr6-update` (based on PR #6 head `cedcea3`)  
**Merged:** `main` (`ebfa16e`, v0.4.6 — M37 point text from issue #7 and interactive button groups from issue #8 part B)  
**Merge strategy:** `git merge main` (non-rebase; all contributor commits preserved verbatim)

---

## 1. Merge Conflicts and Resolution

`git merge main` produced three content conflicts and one semantic compile mismatch:

1. **`crates/engine/src/tests.rs`**
   - **Registry command count & golden lock (`m32_sentence_case_command_labels`, `r5_01_registry_golden_lock`):** PR #6 added `slide.set_transition_all` (186 → 187 commands) while `main` added `group.set_interactive` and `group.update` (186 → 188 commands). Resolved to **189 commands**, keeping both sides' `EXPECTED` entries (auto-merged cleanly) and updating the golden table length (`24_336` bytes) and FNV-1a hash (`0xdc61_5cf1_56dd_4f3d`).
   - **Appended test functions:** PR #6 added `grid_spacing_is_a_view_setting_that_undoes_and_rejects_nonsense` and `transition_apply_to_all_sets_every_slide_in_one_undo_step` at the end of the file where `main` appended `pt_03_headless_point_text_add_and_refit` through `gb_15_*`. Resolved by keeping **both** sides' tests intact with clean function boundaries.
2. **`crates/ui-egui/src/uiux_sweep_tests.rs`**
   - PR #6 intentionally deleted `ui07_update_status_bar_hint_handles_narrow_width` in commit `9a99068` when replacing the hand-written `editor.ui.update` dialog / status-bar hint with `craft-update`. `main` appended `m37_sweep_course` and `gb_14_m37_point_text_and_interactive_group_uiux_sweep` right below that deleted test.
   - Resolved by omitting the removed `ui07_update_status_bar_hint_handles_narrow_width` (which referenced removed `editor.ui.update` fields) and keeping all of `main`'s M37 UI/UX sweep code and tests (`m37_sweep_course` + `gb_14_m37_point_text_and_interactive_group_uiux_sweep`).
3. **`docs/roadmap.md`**
   - Both branches added an `## M37 ...` section after `## M36 Refactor A ✅`. Resolved in the merge commit by keeping `main`'s `## M37 Point text and grouped buttons ✅` followed by PR #6's ribbon roadmap sections, then renumbered PR #6's sections to `M38`–`M43` in a dedicated commit (Section 3 below).
4. **Post-merge compile & clippy fixes:**
   - `show_text_controls` in `crates/ui-egui/src/properties.rs` gained a `point_text: bool` parameter in `main`; updated `show_text_panel`'s `TextTarget` and call site to thread `point_text` through.
   - Merging PR #6's `ribbon.rect` check with `main`'s live point-text measurement pushed `draw_inline_edit` in `crates/ui-egui/src/canvas.rs` over clippy's cognitive-complexity limit (`28/25`). Extracted `update_point_text_live_bounds` and `dispatch_inline_edit_format_shortcuts` helpers to bring it back under 25.

---

## 2. Where M37 Features Live in the New Ribbon & Dock-Panel UI

### Interactive button groups (`group.set_interactive`, `group.update`)
- **Contextual ribbon (`Drawing Tools › Format`, `Button Tools › Format`, `Picture Tools › Format` › `Arrange and Align`):** Added a direct small control (`ctx.group_button` in `crates/ui-egui/src/ribbon/spec.rs`) wired to `Action::Command("group.set_interactive")` via `menu::adapter` / `menu::adapter_reason`.
  - Shows `"Make button from group"` when a regular group is selected, flips to `"Remove button behaviour"` (with active/selected styling) when an interactive group is selected, and draws disabled with tooltip `"Select a group first"` (or master/state-edit lock reason) otherwise.
- **`Home › Drawing › Arrange ▾` & contextual `Arrange ▾`:** Dispatches through `UiMenu::Arrange` → `menu::arrange_menu_items` → `render_arrange_extras`, which displays `"Make button from group"` / `"Remove button behaviour"` for a selected group and runs `group.set_interactive`.
- **Canvas & timeline group-row context menus:** Preserved in `menu::render_group_ungroup_items` (called by both canvas and timeline object/group-row menus), localized with `tl!`.
- **Docked `Format` inspector (`Panel::Format`):** Continues to draw the full M37 Group section via `properties::show_multi_selection`: `"Act as one button"` checkbox + tooltip, `Name`, `Initial state`, `Accessible description`, `Focus order` + `Clear`, and `Add trigger…` (which opens the modal trigger form pre-targeted at the group). All M37 group inspector strings now go through `tl!`.
- **Docked `Timeline` panel (`Panel::Timeline`):** Continues to render the `"Button"` pill on interactive group folder rows (`timeline.rs`), now localized through `tl!("Button")`.

### Point text (issue #7) & `"Wrap text"`
- **Text Box tool (`Insert › Text › Text Box`, `Drawing Tools › Format › Insert Shapes › Text Box`):** Arms `PlacementTool::TextBox`; clicking the canvas places fitted point text (`point_text: true`, `style.wrap: false`) and enters inline edit (which auto-opens `Panel::Text`), while dragging places fixed-width area text (`point_text: false`, `style.wrap: true`). Updated the ribbon tooltip to describe both click and drag placement.
- **Ribbon `Home › Paragraph › Wrap Text`:** Added `TextOp::WrapText` / `text_format::Change::Wrap` (`paragraph.wrap_text` in `ribbon/spec.rs`), dispatching `text.set_paragraph` `{ wrap }` (the exact same engine command path that converts point text to area text) and committing any uncommitted inline-edited buffer first in the same single undo step.
- **`Text` settings dock panel (`Panel::Text` — Font / Paragraph / Text Box sections):** Extracted shared `show_autofit_row` / `show_text_box_rows` / `show_text_box_controls` helpers in `crates/ui-egui/src/properties.rs` (zero duplicated controls; Format inspector layout preserved verbatim). `Panel::Text › Text Box` now displays `Autofit`, `VAlign`, `Wrap text` (with the point-text hover tooltip), `Direction`, `Insets`, and `Columns` (disabled). Toggling `"Wrap text"` from `Panel::Text` runs `text.set_paragraph` `{ wrap }`, commits uncommitted inline-edited text in the same undo step when editing on canvas, and converts point text to area text with 1-step undo.
- **Localization (`tl!`):** All new and M37 user-facing labels/tooltips touched in `ribbon/spec.rs`, `ribbon/mod.rs`, `menu.rs`, `properties.rs`, and `timeline.rs` go through `tl!` and are verified in `Lang::PSEUDO` tests.

### New UI Tests Added
- `ribbon::contextual_tests::ribbon_group_button_toggles_interactive_group_and_uses_tl` (tests contextual ribbon toggle, label flip, docked `Panel::Format` `"Act as one button"`, docked `Panel::Timeline` `"Button"` pill, and `Lang::PSEUDO` localization).
- `ribbon::contextual_tests::home_arrange_menu_toggles_interactive_group` (tests `Home › Drawing › Arrange ▾` dispatching `group.set_interactive`).
- `panels_tests::point_text_wrap_from_text_panel_and_ribbon_converts_to_area_text` (tests `Panel::Text › Text Box` `"Wrap text"`, `Home › Paragraph › Wrap Text`, 1-step undo back to point text, and `Lang::PSEUDO` localization).

---

## 3. Milestone Renaming (Docs Only)

Commit history was left untouched. Across all documentation, PR #6's milestones were shifted by +1 so `main`'s `M32`–`M37` entries remain intact and collision-free:

| Old PR #6 Label | New Label | Scope |
|---|---|---|
| M37 | **M38** | Ribbon UI parity |
| M38 | **M39** | Text and formatting gaps |
| M39 | **M40** | Pictures, buttons and media polish |
| M40 | **M41** | Structure, masters and questions |
| M41 | **M42** | Animation and timeline depth |
| M42 | **M43** | Preview and AI ribbon |

Updated files: `docs/roadmap.md`, `docs/release-checklist.md`, `docs/ui/README.md`, `docs/ui/ribbon.md`, `docs/ui/contextual-tabs.md`, `docs/ui/panels.md`, `docs/ui/feature-parity.md`, `docs/ui/implementation.md`, and `docs/ui/localization.md`.

---

## 4. Update Checks & Release Packaging Audit (`craft-update` vs. Issue #5)

Audited `.github/workflows/release.yml`, `scripts/package-macos.sh`, `apps/coursecraft/src/updates.rs`, `apps/coursecraft/src/main.rs`, `README.md`, and the pinned `craft-update` crate (`storytold/craft-libs` rev `63196e66`):

1. **`SHA256SUMS.txt` published on release — YES (guarded by test):**
   - `.github/workflows/release.yml` lines 114–116 run `sha256sum coursecraft-* > SHA256SUMS.txt` in `dist/` and upload `dist/*` to the draft GitHub release.
   - Added an explicit assertion for `"sha256sum coursecraft-* > SHA256SUMS.txt"` inside `updates::tests::the_release_file_names_are_the_ones_the_update_picks`.
2. **SHA-256 verification before installing — YES (fail-closed):**
   - `craft_update::install::download` (`crates/update/src/install.rs:416-449`) requires a 64-hex-char SHA-256 digest (from GitHub API `asset.digest` or fetched from the release's `SHA256SUMS.txt`). If no checksum is published or `SHA256SUMS.txt` cannot be fetched, it refuses to download/install. `Http::download` verifies the downloaded bytes against the checksum before proceeding (retrying once on mismatch, then aborting).
3. **No silent install or silent restart (issue #5 promise) — YES for checks, with important behavioural differences from M35 to note for review:**
   - Startup checks (`check_at_start`) and manual checks (`check_now`) **never** download, install, or restart on their own; they only open `craft-update`'s prompt dialog. Added an assertion in `an_offered_release_opens_the_prompt` confirming status stays `Status::Idle` when offered.
   - Fixed a stale contradiction in `README.md` line 118 (which still claimed `"Help > Check for Updates only opens the releases page. CourseCraft makes no network calls to look for updates."` while line 40 described automatic update checks).
   - **Behavioural shift from M35 to be aware of before merging:**
     - **Self-update vs. download-and-reveal:** M35 downloaded installers into `~/Downloads` and only offered **Show in folder** (never replacing the binary or restarting). `craft-update` instead offers **Update and Restart** on self-updatable copies (macOS `.app`, Linux `.tar.gz` folder, Windows portable folder) and opens the GitHub Releases page in the browser when manual download is chosen or self-update is unsupported.
     - **Explicit confirmation + dirty-state guard:** Installation only starts when the user clicks **Update and Restart** and `!self.project.is_dirty(...)` (`clean == true`). If there is unsaved work, `craft-update` refuses to start and tells the user to save or close work first.
     - **Edge case in `craft-update` (`Later` during download):** Once the user clicks **Update and Restart**, dismissing the prompt with **Later** while the download is still in flight does not cancel the background install; when it completes, if `clean` is still `true`, `Updater::show_if` returns `Action::Close` (which sends `ViewportCommand::Close`) and `on_exit` invokes `start_pending_update()`. On macOS/Linux `prepare_update` also swaps the app bundle/binary on disk before exit.
     - **Unsigned Windows builds:** `craft_update::self_update::windows_precheck` verifies Authenticode signatures (`verifysign::verify_file`). Because CourseCraft's Windows releases are currently unsigned, Windows portable self-update will refuse in-place replacement and direct users to download manually (non-fatal, safe fallback, but worth noting alongside `README.md`'s description of portable self-update).

---

## 5. Gate Results

Both required gates passed cleanly on the final tree:

- **`cargo xtask ci`: PASS**
  - `cargo fmt --all --check`: PASS
  - `cargo clippy --workspace --all-targets -- -D warnings`: PASS (0 warnings)
  - `cargo test --workspace`: **4,095 passed; 0 failed; 17 ignored**
  - `cargo xtask layers`: `layers: ok`
  - `cargo check --target wasm32-unknown-unknown` (all 17 WASM crates: L0–L6, `coursecraft-player`, `coursecraft`): PASS
- **`cargo xtask build-player`: PASS**
  - `trunk build --release` (`wasm-release` profile) succeeded in `1m 16s`; output written to `target/player-dist`.

---

## 6. What Could Not Be Done / Left for Hand Verification

- **Interactive desktop & browser hand checks:** No GUI window or live browser session was launched; all UI, dock-panel, ribbon, and pseudo-locale checks were verified via headless `TestHarness` tests.
- **Live end-to-end `craft-update` self-update test:** Testing a real GitHub release download + `SHA256SUMS.txt` fetch + binary replacement on macOS/Windows/Linux requires a tagged release with the new asset naming scheme (`coursecraft-<ver>-<os>-<arch>.*`).
- **Pre-existing non-M37 doc counts:** `docs/ui/feature-parity.md`'s summary table totals were left aligned with PR #6's existing counts (M37's point text and interactive button groups were folded into existing ✅ rows rather than adding new rows).

---

## 7. Clean-Room Concerns Flagged in PR #6 (Reported Only, Not Changed)

Per the task instructions, the items below are **flagged for review without modification**:

1. **Direct structural & positional mirroring of Storyline 360's UI ("frame by frame"):**
   - `docs/ui/README.md` (lines 3–11, 50–51) states the ribbon plan was written from Storyline 360 beginner tutorials *"watched frame by frame"* and sets the goal as *"the same tabs, the same groups and the same buttons in the same order"*.
   - `docs/ui/storyline-observed.md` (lines 11–369) is a full screen-by-screen transcription of Storyline 360's window chrome, tab strip, group order, control order, right-click menus, and modal dialogs — including exact product strings (`Articulate Storyline x64`, `Name.story*`, `Content Library 360`, `Review 360`, `Reach 360`, `Trigger Wizard`, `Freeform`, `Moderate Frame`, `Bevel Matte`, `Clean`, exact tooltip wording on lines 44–46, and exact dialog field layouts on lines 348–359).
   - `crates/ui-egui/src/ribbon/spec.rs:620` contains the comment `// Three rows of five, in Storyline's reading order.`
   - Note that `docs/contributing.md` › Clean-room rules states: *"Study other authoring tools for **behaviour only**: what happens when you click, the order triggers fire in. Design our own look and layout from `DESIGN.md`."* Replicating exact tab/group/button ordering and shipping disabled placeholders for features CourseCraft explicitly marks **Not planned** (`Record Screen`, `Characters`, `Photos`, `Illustrations`, `Videos`, `360° Image` — `spec.rs:972, 1152-1208`) solely to match Storyline's ribbon layout goes beyond behaviour-only parity and warrants an explicit owner call on whether to keep or trim those disabled slots and `storyline-observed.md`.
2. **Storyline-specific / distinctive control labels still present in `crates/ui-egui/src/ribbon/spec.rs` and `docs/ui/`:**
   - Even after PR #6's clean-room renaming table (`docs/ui/README.md:58-68`), several labels in `ribbon/spec.rs` and `docs/ui/` read as Storyline-specific rather than generic eLearning terminology:
     - **`When Clicked Action`** (`spec.rs:2522`, `contextual-tabs.md:72`) — Storyline's specific Button Tools phrasing (generic equivalent: *On click* / *Add click trigger*).
     - **`Feedback Master`** (`spec.rs:1538, 1584, 3061`, `ribbon.md`, `contextual-tabs.md`) — Storyline's coined companion to Slide Master (generic equivalent: *Feedback layouts*).
     - **`Zoom Region`** (`spec.rs:1138`, `ribbon.md:197`) — Storyline's feature name for camera zoom (generic equivalent: *Camera zoom* / *Zoom area*).
     - **`Focus Order`** (`spec.rs:591`, `ribbon.md:157`) — Storyline's label for tab/reading order (generic equivalent: *Tab order* / *Reading order*).
     - **`Starting Scene`** & **`Index`** (`spec.rs:919, 926`, `ribbon.md:120-121`) — including `Index` whose behaviour is explicitly marked *"Storyline's scene index menu; behaviour to confirm"* (drawn as a placeholder copied from the tutorial without a defined generic need).
     - **`Button Set`** (`docs/ui/shell-panels-dialogs.md:103`, `feature-parity.md:131`) — Storyline's name for mutually exclusive selectable objects (generic equivalent: *Selection group* / *Exclusive group*).
     - **`Scrolling Panel`** (`spec.rs:1381`, `ribbon.md:224`) — already used in CourseCraft M12, though closely matches Storyline's label.
     - **`Preserve`** (`spec.rs:3180`, Slide Master tab) — PowerPoint/Storyline master-retention label.
     - **Disabled `NotPlanned` buttons on the live ribbon (`spec.rs:972, 1152-1208`):** `Record Screen` (on the patent watch list in `docs/contributing.md`), `Characters`, `Photos`, `Illustrations`, `Videos`, and `360° Image` are rendered as disabled buttons in CourseCraft's UI even though CourseCraft does not plan to build them — their presence is explained only by matching Storyline's ribbon geometry.
3. **Competitor-referencing prose in docs & release checklist:**
   - `docs/release-checklist.md:187` references *"the two tutorial courses' first slides"* (referring to the two Storyline 360 YouTube tutorials cited in `docs/ui/README.md`).
   - `docs/contributing.md:26` was edited by PR #6 to say *"in the place the tutorials show (`docs/ui/storyline-observed.md`)"*, which sits in tension with the clean-room rule on line 41 of the same file (*"Design our own look and layout from `DESIGN.md`"*).

---

## 8. Release Packaging for 0.5.0

### What Changed
- **`.github/workflows/release.yml`:**
  - **macOS:** unchanged (`scripts/package-macos.sh` → `coursecraft-<ver>-macos-<arch>.dmg`).
  - **Windows:** removed the `Package for Windows (portable)` step (`-windows-x64-portable.zip` and `portable.txt`); only `Build Windows installer` remains, producing `coursecraft-<ver>-windows-x64-setup.exe` via Inno Setup (`packaging/windows/coursecraft.iss`).
  - **Linux:** kept `Package for Linux` (`coursecraft-<ver>-linux-x86_64.tar.gz`) and added `Package for Linux (AppImage)` producing `coursecraft-<ver>-linux-x86_64.AppImage`. Installed `desktop-file-utils` in `Install Linux libraries` so `desktop-file-validate` checks the `.desktop` entry during the build.
  - **Checksums:** `sha256sum coursecraft-* > SHA256SUMS.txt` in the `release` job covers all four release artifacts (`.dmg`, `-setup.exe`, `.tar.gz`, and `.AppImage`).
- **Linux desktop metadata (`packaging/linux/coursecraft.desktop`, `packaging/linux/README.md`):** added a freedesktop `.desktop` entry (`Type=Application`, `Name=CourseCraft`, `Comment=eLearning authoring`, `Exec=coursecraft %F`, `Icon=coursecraft`, `Terminal=false`, `Categories=Education;Office;`, `StartupWMClass=coursecraft`).
- **In-app updates (`apps/coursecraft/src/updates.rs`):** audited `craft-update` (`storytold/craft-libs` rev `63196e66`) and added a minimal Windows wrapper in CourseCraft (no edits to `craft-libs`) plus Linux/Windows asset-selection tests.
- **Docs & Changelog:**
  - `README.md`: updated **Get CourseCraft**, **Linux** first-launch steps, the in-app update summary, and **Install on Windows** to remove portable Windows `.zip` instructions and document `.AppImage`.
  - `docs/maintaining.md`: updated the **Releasing a version** artifacts list (`.dmg`, `-setup.exe`, `.AppImage`, `.tar.gz`, `SHA256SUMS.txt`) and in-app update description.
  - `CHANGELOG.md`: added `## 0.5.0 (unreleased)` noting that Windows portable builds stop and Linux gains an `.AppImage` (verified with `./scripts/release-notes.sh 0.5.0`).
- **Packaging tests (`apps/coursecraft-cli/tests/bughunt_m26_cli_and_packaging.rs`, `apps/coursecraft/src/updates.rs`):** updated existing tests to remove portable assertions and added `release_workflow_builds_dmg_setup_tarball_appimage_and_no_portable` asserting `.dmg`, `-setup.exe`, `.tar.gz`, `.AppImage`, pinned/checksummed `appimagetool`, `.desktop` + 1024 px icon + licences, `SHA256SUMS.txt`, and no portable Windows package.

### How the Linux AppImage Is Built
- **Tool:** `AppImage/appimagetool` pinned to immutable release **`1.9.1`** (`appimagetool-x86_64.AppImage`, published 2025-11-18).
- **Download URL:** `https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-x86_64.AppImage`
- **SHA-256 checksum (verified via `sha256sum -c -` before executing):**
  `ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0`
- **AppDir layout (`stage/CourseCraft.AppDir`):**
  - `AppRun` (mode `0755`, POSIX `/bin/sh` wrapper exec'ing `$HERE/usr/bin/coursecraft "$@"`)
  - `usr/bin/coursecraft` (release binary, mode `0755`)
  - `coursecraft.desktop` and `usr/share/applications/coursecraft.desktop` (from `packaging/linux/coursecraft.desktop`, validated with `desktop-file-validate`)
  - `coursecraft.png`, `.DirIcon` (symlink to `coursecraft.png`), and `usr/share/icons/hicolor/1024x1024/apps/coursecraft.png` (from `apps/coursecraft/icons/coursecraft-1024.png`)
  - `usr/share/doc/coursecraft/{LICENSE-MIT,LICENSE-APACHE,ATTRIBUTION.md}`
- **Execution in CI:** `ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 "$tool" "$appdir" "$out"` so `appimagetool` runs cleanly on `ubuntu-24.04` runners without requiring FUSE2. Output is written to `dist/coursecraft-$ver-linux-x86_64.AppImage` (mode `0755`).

### Update-Selection Behaviour by OS
- **macOS (`InstallMode::CraftUpdate`):** delegated directly to `craft-update`. Picks `coursecraft-<ver>-macos-arm64.dmg` (or `x86_64`), verifies SHA-256 against `SHA256SUMS.txt` / GitHub digest, and offers **Update and Restart** when the `.app` bundle is writable (otherwise **Download** opens the release page). Never installs or restarts silently.
- **Linux (`InstallMode::CraftUpdate`):** audited `craft-update/src/self_update.rs` (`detect` lines 594–601, `Install::kind` lines 64–75, `prepare_update` lines 823–904):
  - When `$APPIMAGE` is set, `craft-update` detects `Install::AppImage(path)` → `PackageKind::AppImage` → selects `coursecraft-<ver>-linux-x86_64.AppImage`, verifies SHA-256, atomically swaps the AppImage with mode `0755`, and relaunches after exit.
  - When `$APPIMAGE` is unset (e.g. unpacked `.tar.gz`), `craft-update` detects `Install::Standalone(exe)` → `PackageKind::TarGz` → selects `coursecraft-<ver>-linux-x86_64.tar.gz`, verifies SHA-256, unpacks and atomically replaces `bin/coursecraft`, and relaunches after exit.
  - No edits to `craft-libs` were needed; CourseCraft adds unit tests verifying asset selection for `$APPIMAGE` set, unset, empty, and architecture mismatch.
- **Windows (`InstallMode::WindowsSetup` / `InstallMode::WindowsLegacyPortable`):**
  - `craft-update` has no `PackageKind` for Inno Setup (`-setup.exe`) and maps both `Install::Portable` and Windows `Install::Standalone` to `PackageKind::PortableZip`, which would fail with `"has no Portable (.zip) for Windows x64"` if self-update were ever enabled (e.g. signed binaries or `CRAFT_UPDATE_DEV=1`) and would only open the generic release HTML page otherwise.
  - CourseCraft's minimal wrapper in `apps/coursecraft/src/updates.rs` detects Windows copies (marking copies with a sibling `portable.txt` as `WindowsLegacyPortable`, others as `WindowsSetup`), intercepts discovered releases from `Updater`'s background check thread while preserving `Status::Checking` / `UpToDate` / `Failed` / `Managed` handling, and opens `craft-update`'s `StartupPrompt` with `is_self_updating = false` (manual **Download** / **Later** only; never returns `Action::Close`, never installs or restarts silently).
  - Clicking **Download** opens the verified direct URL to `coursecraft-<ver>-windows-x64-setup.exe` (bound to the release tag and repository via `expected_download_url` + `safe_web_link`, falling back cleanly to `release.html_url` / `releases_url()` if absent or invalid; never selecting `-portable.zip`).
  - On `WindowsLegacyPortable`, the prompt displays a translated notice: *"The portable Windows version is no longer published. Download the Windows installer (-setup.exe) to update."*

### Validation & Test Results
- **Workflow YAML validation:** `actionlint` is not installed in this environment; `.github/workflows/release.yml` was parsed with Ruby's `YAML.load_file` (valid YAML, 2 jobs: `build` with 12 steps and `release` with 5 steps) and carefully reviewed line by line (indentation, shell quoting, heredoc indentation, matrix `if:` conditions, artifact paths, and `SHA256SUMS.txt` glob).
- **Changelog extraction:** `./scripts/release-notes.sh 0.5.0` exited 0 and printed the `0.5.0 (unreleased)` notes cleanly.
- **`cargo xtask ci`: PASS**
  - `cargo fmt --all --check`: PASS
  - `cargo clippy --workspace --all-targets -- -D warnings`: PASS (0 warnings)
  - `cargo test --workspace`: **4,101 passed; 0 failed; 17 ignored** (including 10 `updates::tests::*` tests and 12 `bughunt_m26_cli_and_packaging` tests)
  - `cargo xtask layers`: `layers: ok`
  - `cargo check --target wasm32-unknown-unknown` (all 17 WASM crates): PASS
- **`cargo xtask build-player`: PASS** (`trunk build --release` succeeded; written to `target/player-dist`).

### What Can Only Be Verified by a Real Tagged Release (or `workflow_dispatch` Run)
- **End-to-end GitHub Actions execution on `ubuntu-24.04`, `windows-2022`, and `macos-15`:** confirming `appimagetool 1.9.1` downloads cleanly, passes the pinned SHA-256 check, validates `coursecraft.desktop`, and builds a working squashfs AppImage under `APPIMAGE_EXTRACT_AND_RUN=1`; confirming the Windows job produces only `coursecraft-<ver>-windows-x64-setup.exe` and no portable zip; confirming `SHA256SUMS.txt` lists all four `coursecraft-*` files.
- **Launching the built `.AppImage` on real Linux desktops** (Ubuntu/Fedora/Wayland/X11) to verify GTK 3 runtime linking, desktop icon/window-class matching, and in-place self-update from an older `.AppImage` (`$APPIMAGE` set) and from an unpacked `.tar.gz` (`$APPIMAGE` unset) against a live GitHub release.
- **Live Windows update check on an old portable copy (`portable.txt` beside `coursecraft.exe`)** against a tagged `v0.5.0` release to verify the prompt notice and browser download of `coursecraft-0.5.0-windows-x64-setup.exe`.


---

## 9. Clean-room cleanup

Addressed all owner decisions from Section 7 plus the in-flight update cancellation fix, in five logical commits (`9cbe6b4`, `cc6eb5a`, `4b0c8c1`, `d177b97`, `dc9ca94`).

### Files Changed (Sections 1–5)

1. **Section 1 — Remove Storyline references (`9cbe6b4`):**
   - Deleted: `docs/ui/storyline-observed.md`
   - Modified: `AGENTS.md`, `README.md`, `docs/contributing.md`, `docs/release-checklist.md`, `docs/roadmap.md`, `docs/ui/README.md`, `docs/ui/contextual-tabs.md`, `docs/ui/feature-parity.md`, `docs/ui/implementation.md`, `docs/ui/panels.md`, `docs/ui/ribbon.md`, `docs/ui/shell-panels-dialogs.md`, `crates/ui-egui/src/doc_tabs.rs`, `crates/ui-egui/src/dock.rs`, `crates/ui-egui/src/properties.rs`, `crates/ui-egui/src/ribbon/spec.rs`
2. **Section 2 — Rename coined labels and remove undefined `Index` placeholder (`cc6eb5a`):**
   - Renamed `"When Clicked Action"` → `"On Click"`, `"Feedback Master"` → `"Feedback Layouts"`, `"Zoom Region"` → `"Camera Zoom"`, `"Button Set"` → `"Selection Group"`, `"Starting Scene"` → `"Start Section"`; removed `"Index"` (`map.index`); kept `Focus Order`, `Scrolling Panel`, `Preserve`.
   - Modified: `crates/ui-egui/src/ribbon/spec.rs`, `crates/ui-egui/src/ribbon/spec_tests.rs`, `docs/roadmap.md`, `docs/ui/README.md`, `docs/ui/contextual-tabs.md`, `docs/ui/feature-parity.md`, `docs/ui/implementation.md`, `docs/ui/ribbon.md`, `docs/ui/shell-panels-dialogs.md`
3. **Section 3 — Hide `NotPlanned` placeholder buttons while keeping spec entries (`4b0c8c1`):**
   - Filtered `Action::NotPlanned` (`map.record_screen`, `insert.characters`, `insert.photos`, `insert.illustrations`, `insert.videos`, `insert.image_360`) out of ribbon groups, shrunk groups, launchers, menus, tooltips, and the accessibility tree with zero empty gaps; kept entries in `spec.rs` with owner-decision comments. Added `ribbon::spec_tests::not_planned_controls_stay_in_spec_but_are_never_rendered_or_focusable`.
   - Modified: `crates/ui-egui/src/ribbon/mod.rs`, `crates/ui-egui/src/ribbon/spec.rs`, `crates/ui-egui/src/ribbon/spec_tests.rs`, `docs/roadmap.md`, `docs/ui/implementation.md`
4. **Section 4 — Licence verification and attribution for `craft-libs` & Craft forks (`d177b97`):**
   - Modified: `ATTRIBUTION.md`, `CHANGELOG.md`
5. **Section 5 — Cancel in-flight Update and Restart on prompt dismissal (`dc9ca94`):**
   - CourseCraft's wrapper (`apps/coursecraft/src/updates.rs`) now manages `StartupPrompt` and the self-update worker with a `craft_update::Stop` token passed to `prepare_update` (both for download and its pre-swap `last` check). Clicking **Later** / **Dismiss** or closing the prompt (`Esc` / `×`) cancels the in-flight update immediately so the app does not close, swap binaries/bundles, or install on exit, while allowing a later explicit **Update and Restart** to start and complete cleanly. Added `updates::tests::closing_prompt_while_downloading_cancels_update_and_allows_retry`. Updated `CHANGELOG.md` 0.5.0 entry.
   - Modified: `apps/coursecraft/src/updates.rs`, `CHANGELOG.md`

### Verified `craft-libs` and Fork Licences

Checked directly in `~/.cargo/git/checkouts/`:
- **`storytold/craft-libs`** (`rev 63196e66f3033a6623eb629af95a104d26986725` — `craft-ui`, `craft-i18n`, `craft-update`): **`MIT OR Apache-2.0`** (`LICENSE-MIT`, `LICENSE-APACHE`; `Copyright (c) 2026 ArtCraft Team and the craft-libs contributors`). Compatible with CourseCraft's `MIT OR Apache-2.0`.
- **`storytold/egui`** (`rev 6d518fc799681571963842f95b55a6b68431b15a`): **`MIT OR Apache-2.0`** (`LICENSE-MIT`, `LICENSE-APACHE`).
- **`storytold/winit`** (`rev a517b3846ee8457d84ee4975110bfda6797c44dc`): **`Apache-2.0`** (`LICENSE`).
- **DeckCraft** ribbon widget patterns (`crates/ui-egui/src/ribbon/widgets.rs`, commit `7fbdc085`): **`MIT OR Apache-2.0`**.

### Final `git grep -niE "storyline|articulate"` Output (Excluding This Report)

Zero new matches relative to `main` (every line below already existed on `main`):

```text
AGENTS.md:3:CourseCraft is an open-source, native, Storyline-comparable eLearning authoring tool written in **Rust only**. The product name is always **CourseCraft** in user-facing text. Machine names are lowercase (`coursecraft-*`).
PRODUCT.md:14:  - professional instructional designers who know Storyline/Captivate and build courses daily: they want density, speed, keyboard shortcuts and precise control;
PRODUCT.md:21:CourseCraft is an open-source, native, Storyline-comparable eLearning authoring tool written in Rust only. Authors build interactive slide-based courses (layers, states, triggers, variables, media, animations, eight question types) and publish them as SCORM packages that run in any LMS. Success: an author can build and publish a good-looking, working course without fighting the tool, and the learner experience looks modern by default.
PRODUCT.md:44:- Clean-room: match Storyline's capabilities, not its look or assets.
README.md:5:An open-source, clean-room alternative to Articulate Storyline 360, written in pure Rust. You build interactive eLearning with slides, layers, states, triggers, variables and quizzes, and publish it to any LMS.
README.md:154:MIT OR Apache-2.0. CourseCraft is not affiliated with Articulate or Adobe. "Storyline" and "Articulate" are trademarks of Articulate Global; "Captivate" and "Adobe" are trademarks of Adobe. They are used only to describe comparable features.
crates/spell/dictionaries/en_US.dic:12612:articulate/YGNPDSX
crates/spell/dictionaries/en_US.dic:12613:articulateness/IM
crates/spell/dictionaries/en_US.dic:27979:inarticulate/Y
crates/spell/dictionaries/en_US.dic:35568:particulate/SM
docs/contributing.md:43:- Never copy another product's code, player JS/CSS, icons, templates, fonts, characters or other assets from closed-source or commercial tools (Articulate, Adobe or any other). Never decompile or unpack their players.
docs/contributing.md:46:- In docs, competitor names may describe comparable features ("what a Storyline author expects"), never "copied from" or "in the style of". The README carries the trademark notice.
docs/plans/feedback-3.md:3:The limits round 2 left open: Storyline's layer options, and shapes that can carry an outline and text, be adjusted, and be drawn freely. Each step ends with `cargo xtask ci` passing.
docs/plans/feedback-3.md:15:- **Layer options are play-time behaviour,** so they live in `coursecraft-logic` and `docs/spec/triggers.md` changes with them. Storyline specifics we can't check are marked `[verify]`.
docs/plans/m11-basics.md:3:Goal: what a Storyline or PowerPoint author expects in the first hour. Every step keeps old `course.json` files loading unchanged (new fields default and are omitted when default), goes through engine commands with undo, has an inspector control, and is drawn the same way on the canvas, in preview, in the published player and in the offscreen renderer (one code path in `coursecraft-render`). PowerPoint import picks up the same property where the file has it. Spell check is left for later.
docs/plans/m12-interactions.md:3:Goal: the interactive building blocks Storyline authors reach for, without writing code. Every runtime change lives in `coursecraft-logic` and is written into `docs/spec/triggers.md` in the same change. Every new object works on the canvas, in preview, in the published player and in offscreen renders, is reachable by keyboard and announced on the web, goes through engine commands with undo, and is driveable from the CLI and MCP.
docs/plans/m13-quizzes.md:3:Goal: quizzes that match what Storyline authors expect: flexible scoring, surveys, question banks, results slides, retry, review and certificates. Every runtime change lives in `coursecraft-logic` and is written into `docs/spec/triggers.md` in the same change.
docs/plans/m8-authoring.md:3:Goal: the everyday speed of Storyline 360. An author can select many things, copy, paste, line them up, reuse layouts and fix text across a course without touching slides one at a time.
docs/roadmap.md:80:Priority (the user's): people can use CourseCraft themselves, in the visual editor, without spending any money. AI is optional help on top and is never required, because requiring it would mean paying for an AI subscription. The first release comes after M9, and later milestones aim to bring it as close to Storyline 360 as possible.
docs/roadmap.md:128:## Toward Storyline 360 and Captivate
docs/roadmap.md:210:Goal: things Storyline and Captivate can't do at all, built on the command engine and the open `course.json`.
docs/roadmap.md:480:- Phone-friendly, scrolling lessons, once CourseCraft can compete with Storyline 360. This is where it starts to stand apart from the competition.
docs/testing/m28-noe-states-analysis.md:187:| **(c) Both: Widened `StateDef` overrides + constrained visual-only `extras` (RECOMMENDED)** | Combine (a) for modifying the host object itself with (b) for adding visual-only child decorations (`extras: Vec<StateExtra>`) positioned relative to the host's `(frame.x, frame.y)`. | Full Storyline-class authoring capability while preserving single-source-of-truth inheritance for the host object AND keeping `coursecraft-logic` completely untouched. | More surface area to implement; cleanly mitigated by shipping in two incremental stages (**c1**: widened `StateDef` + canvas state-edit mode; **c2**: `extras`). |
docs/testing/m28-noe-states-analysis.md:279:Delivers the full Storyline-class, clean-room Object States experience in three shippable slices:
```

### Gate Results

- **`cargo xtask ci`: PASS**
  - `cargo fmt --all --check`: PASS
  - `cargo clippy --workspace --all-targets -- -D warnings`: PASS (0 warnings)
  - `cargo test --workspace`: **4,104 passed; 0 failed; 17 ignored**
  - `cargo xtask layers`: `layers: ok`
  - `cargo check --target wasm32-unknown-unknown`: PASS
- **`cargo xtask build-player`: PASS** (`trunk build --release` succeeded; written to `target/player-dist`)
