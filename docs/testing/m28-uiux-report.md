# M28 UI/UX Audit Report — Before & After

**Date:** 2026-11-22  
**Plan:** [`docs/testing/m28-uiux-plan.md`](m28-uiux-plan.md)  
**Detailed Bug-Hunt Log:** [`docs/bug-hunt/2026-11-22-m28.md`](../bug-hunt/2026-11-22-m28.md)

---

## 1. Executive Summary

Across Steps **h1–h7** (owner feedback) and **u1–u17** (automated harness, shared primitives, surface-by-surface fixes, and full matrix sweep), CourseCraft moved from ad-hoc manual UI checks to automated `accesskit` + painted-shape enforcement across **61 editor surfaces** (`854` audited frames) and **36 learner-player viewport/theme states** (`6` browser viewports × `3` themes × `2` states).

- **Final automated sweep result:** **0 MUST violations**, **0 unopened surfaces**, **7 allowlisted `TextEdit::singleline` long-name exceptions** (budget: `<= 20`, zero blanket entries).
- **Watchlist disposition:** All **42** confirmed watchlist suspects (`SUS-01`–`SUS-42`) were audited; **39** were fully reproduced and fixed, **1** (`SUS-37`) was intentionally kept per Reviewer Note #4 (platform-standard `"Don't Save"` / `"Don't ask again"`), and **2** have minor non-blocking residuals documented in §3 (`SUS-06` in `resources_ui.rs:175` and `text_styles_ui.rs:109`; `SUS-13` in `shape_gallery.rs:322`).

---

## 2. Baseline vs. Final Counts

> **Note on baseline methodology:** The automated sweep harness (`audit_surface_report`, `uiux_sweep_all_surfaces`, `uiux_player_sweep`) was built during Steps **u1–u2** and **u17**, so no pre-u1 runtime sweep JSON exists. Baseline counts below reflect the **static-audit confirmed suspects** (`SUS-01`–`SUS-42` plus owner-feedback items `h1`–`h7`) mapped to `UX-01`–`UX-22`; final counts are **observed** from the passing Step u17/u18 test runs.

### 2.1 By Rule (`UX-01`–`UX-22`)

| Rule | Category | Baseline Confirmed Suspects / Gaps | Final Sweep Violations | Allowlisted Entries | Status |
|---|---|---:|---:|---:|---|
| **UX-01** | Dropdowns & Popups | `SUS-01`–`SUS-06` (41 combos across inspector & dialogs) | 0 | 0 | Pass (2 minor `SUS-06` combos noted in §3) |
| **UX-02** | Scrolling (no nested vertical `ScrollArea`) | `SUS-07`–`SUS-10` (12 dialogs) | 0 | 0 | Pass |
| **UX-03** | Scroll Stability | `SUS-03`, `font_picker` / `Dialog` scroll | 0 | 0 | Pass |
| **UX-04** | Clipping & Overlap | `SUS-19`, `SUS-20`, `SUS-24`, strip title, rulers, course map | 0 | 4 (`long_names_course` `TextEdit::singleline`) | Pass |
| **UX-05** | Hit Target Size | `SUS-32`, `SUS-34`, `widgets::icon_button`, `timeline`, `properties`, `slides` | 0 | 0 | Pass |
| **UX-06** | Colour Contrast | `SUS-26`, `SUS-30`, Light/Dark palette tokens, `record_narration` | 0 | 0 | Pass |
| **UX-07** | Theme Fidelity | `SUS-29` (`SQUIGGLE_COLOR`, WASM `ThemeMode::Dark`) | 0 | 0 | Pass |
| **UX-08** | Disabled Controls | `SUS-25`, `SUS-33` (31+ `add_enabled` sites + Format Painter) | 0 | 0 | Pass |
| **UX-09** | Toggle & Mixed State Fidelity | `h5`, `SUS-21`, `SUS-22` (run-boundary & mixed `B/I/U`) | 0 | 0 | Pass |
| **UX-10** | Icon Tooltips & Accessible Names | `SUS-31`, `SUS-32`, `widgets::icon_button`, `slides`, `layers`, `canvas` | 0 | 0 | Pass |
| **UX-11** | Copy, Casing, Positive Wording & Units | `SUS-35`–`SUS-39` | 0 | 0 | Pass (`SUS-37` kept per Reviewer Note #4) |
| **UX-12** | Spacing & Column Alignment | `preferences.rs`, `layers.rs`, `course_settings.rs` | 0 | 0 | Pass |
| **UX-13** | Keyboard Navigation & Focus | `SUS-23`, `SUS-41`, `SUS-42`, `font_picker` `Esc` | 0 | 0 | Pass |
| **UX-14** | Dialog Bounds, Resizing & Memory | `SUS-11`–`SUS-15`, `h3` | 0 | 0 | Pass (`shape_gallery.rs` noted in §3) |
| **UX-15** | Modal Backdrop Dimming | `h3`, `SUS-27`, `SUS-28` | 0 | 0 | Pass |
| **UX-16** | Z-Order & Clamping | `h1`, `SUS-16`–`SUS-18`, `eyedropper`, `speed_popup`, `zoom_hud` | 0 | 3 (`long_names_course` `TextEdit::singleline`) | Pass |
| **UX-17** | Empty States | `media_library`, `question`, `triggers`, `alt_text` | 0 | 0 | Pass |
| **UX-18** | Error Messages | `record_narration` 0-device, `State::Failed` player screen | 0 | 0 | Pass |
| **UX-19** | Long-Name Stress Resilience | `SUS-24`, `slides`, `course_map`, `variables`, `status`, `media_library` | 0 | 0 (covered via 7 `TextEdit` entries above) | Pass |
| **UX-20** | Numeric Input Consistency | `SUS-39` (`multi_text_size` `TextEdit` → `DragValue`) | 0 | 0 | Pass |
| **UX-21** | Universal Undo | Verified across `u8`–`u13` area tests | 0 | 0 | Pass |
| **UX-22** | Preview-Equals-Published Parity | `h4`, `h6`, `h7`, `uiux_player_preview_vs_published_pixel_parity` | 0 | 0 | Pass |

### 2.2 By Surface Group

| Surface Group | Automated Coverage | Final Sweep Violations |
|---|---|---:|
| Menu bar, context menus & top toolbar | Covered in `u7` (`toolbar_menu_tests.rs`) + every editor sweep frame | 0 |
| Inspector (`Format`, `Animate`, `Slide`, `Course`) & States | `inspector-*` × 14 matrix configs + `u4`/`u5`/`u8` unit suites | 0 |
| Slide Navigator, Master View, Course Map & Canvas/Rulers | `course-map`, `master-view`, `canvas-rulers` × 14 configs + `u9` suite | 0 |
| Bottom Panel (`Timeline`, `Triggers`, `Variables`, `Question` × 9 types + Results) | `bottom-*`, `question-*` × 14 configs + `u10`/`u11` suites | 0 |
| Dialogs & Galleries (`WINDOWS` + `window-history`, 22 windows) | `window-*` × 14 configs + `u12`–`u15` suites | 0 |
| In-Editor Preview & Desktop App Shell | `window-preview` × 14 configs + `h1`/`h3`/`h4`/`u6`/`u16` suites | 0 |
| Learner Player Chrome (`PlayerFrame`) | `uiux_player_sweep` (36 states) + `uiux_player_preview_vs_published_pixel_parity` | 0 |

---

## 3. Complete Disposition of Watchlist Suspects (`SUS-01`–`SUS-42`)

| ID | Area / File | Disposition | Step | Summary of Fix / Note |
|---|---|---|---|---|
| **SUS-01** | `properties.rs` (`prop_multi_font`, `prop_timer_font`) | Reproduced & Fixed | u4 | Migrated both to `font_picker::show_font_picker`. |
| **SUS-02** | `layers.rs` (`layer_allow_seek`, `layer_revisit`) | Reproduced & Fixed | u10 | Migrated to `widgets::styled_combo` respecting form width. |
| **SUS-03** | `triggers.rs` (`tall_combo`), `variables.rs` (`var_new_type_combo`) | Reproduced & Fixed | u11 | Migrated to `widgets::styled_combo(ui, id, >= 120.0)`. |
| **SUS-04** | `properties.rs` (5 inspector/link combos) | Reproduced & Fixed | u8 | Migrated to `widgets::styled_combo`. |
| **SUS-05** | 9 dialog/panel combos (`ai_settings`, `audio_ai`, `course_settings`, `export_dialog`, `media_library`, `record_narration`, `translate_ai`, `variables`) | Reproduced & Fixed | u11–u14 | Migrated all 9 to `widgets::styled_combo`. |
| **SUS-06** | 25 combos missing `.height(...)` (`properties.rs`, `course_settings.rs`, `states.rs`, `resources_ui.rs`, `text_styles_ui.rs`) | **Partial** (23 fixed, 2 residual) | u8, u13 | `properties.rs`, `course_settings.rs`, and `states.rs` migrated to `styled_combo`. Residual: `resources_ui.rs:175` (`resource_kind`, 2 items, `.width(60.0)`) and `text_styles_ui.rs:109` (`text_style_pick`, `.width(110.0)`) still use raw `egui::ComboBox` without `.height`. |
| **SUS-07** | `status.rs` (`Check Course`, `Error History`) | Reproduced & Fixed | u15 | Removed nested `ScrollArea`s; `Dialog::show` provides single vertical scroll. |
| **SUS-08** | `media_library.rs` (`Media Library`) | Reproduced & Fixed | u12 | Switched to `DialogUi::body_unscrolled` + dynamic `table_height`. |
| **SUS-09** | `version_history.rs`, `find.rs`, `text_styles_ui.rs`, `translation.rs` | Reproduced & Fixed | u12–u14 | Removed inner `ScrollArea`s (`find.rs` uses `body_unscrolled` + single results scroll). |
| **SUS-10** | `document.rs`, `outline.rs`, `translate_ai.rs`, `shape_gallery.rs`, `triggers.rs` (JS) | Reproduced & Fixed | u11–u14 | Switched dialogs to `body_unscrolled`; removed inner `ScrollArea::both()` in `triggers.rs`. |
| **SUS-11** | `preview.rs` (960×640 cap, no resize/fullscreen) | Reproduced & Fixed | h3, u6 | Added 8 resize grips, session size memory, `can_fullscreen = true`, `F11`/`Cmd+Ctrl+F`. |
| **SUS-12** | `apps/coursecraft/src/main.rs` (`ViewportBuilder`) | Reproduced & Fixed | u16 | Added `.with_min_inner_size([1024.0, 600.0])` in `editor_viewport_builder()`. |
| **SUS-13** | 7 non-resizable dialogs (`course_settings`, `find`, `ai_settings`, `publish`, `text_styles_ui`, `interaction_gallery`, `shape_gallery`) | **Partial** (6 fixed, 1 residual) | u12–u14 | 6 dialogs made `.resizable(true)`. Residual: `shape_gallery.rs:322` (`Dialog::new("Shapes")`) does not set `.resizable(true)`. |
| **SUS-14** | `shortcuts.rs` (raw `Area::new` bypassing `dialog_area`) | Reproduced & Fixed | u15 | Constrained `shortcuts_dialog_area` to `dialog::dialog_area(ctx)`. |
| **SUS-15** | `dialog.rs`, `find.rs` (session size memory) | Reproduced & Fixed | u13 | Switched `find.rs` to persistent `Id::unique("find_and_replace_window")` (session memory per Reviewer Note #9). |
| **SUS-16** | `player_frame.rs` (`draw_drawer` clip override) | Reproduced & Fixed | h1, u6 | Clipped drawer background & child `Ui` to `ui.clip_rect().intersect(full).intersect(rect)`. |
| **SUS-17** | `player_frame.rs` (`draw_speed_popup` clamping) | Reproduced & Fixed | u6 | Clamped speed popup horizontally and vertically inside `full`. |
| **SUS-18** | `coach.rs` (hardcoded coords, no `Esc`) | Reproduced & Fixed | u15 | Anchored to live `coach::set_chrome_rects`, added `.constrain_to(screen)` and `Esc`. |
| **SUS-19** | `welcome.rs` (card height math) | Reproduced & Fixed | u15 | Fixed to `(ground_rect.max.y - card_y - 16.0).max(120.0)` and bounded recent paths. |
| **SUS-20** | `toolbar.rs` (desktop overflow at 1100–1220 px) | Reproduced & Fixed | u7 | Raised label threshold to `1220.0 pt` when `file_ops_available == true`. |
| **SUS-21** | `properties.rs` (run-boundary `<=` & multi-run mixed) | Reproduced & Fixed | h5, u4 | Fixed half-open `[start, end)` query (`h5`) and `Option<bool>` + `mixed_toggle` (`u4`). |
| **SUS-22** | `properties.rs` (multi-object `B/I/U` mixed state) | Reproduced & Fixed | u4 | Migrated to `widgets::mixed_toggle` (`accesskit::Toggled::Mixed`). |
| **SUS-23** | `player_frame.rs` (`draw_speed_popup` `Esc` check) | Reproduced & Fixed | u6 | Separated `Key::Escape` consumption from pointer-outside check. |
| **SUS-24** | `player_frame.rs` (drawer long-name overflow) | Reproduced & Fixed | u16 | Added `paint_truncated_text` with hover tooltips for titles, tabs, resources, glossary. |
| **SUS-25** | `toolbar.rs` (Course map toggle in Master View) | Reproduced & Fixed | u7 | Switched to `toolbar_button_enabled` with `"Not available in Master View"`. |
| **SUS-26** | `font_picker.rs` (Light-mode contrast failure) | Reproduced & Fixed | u5 | Changed highlighted row fill from `p.accent_hover` to `p.accent_subtle`. |
| **SUS-27** | `preview.rs`, `apps/coursecraft/src/main.rs` (weak backdrops) | Reproduced & Fixed | h3, u16 | Preview uses `0xCC` (80% black); 4 shell modals use `modal_backdrop_color(mode)`. |
| **SUS-28** | `shortcuts.rs` (transparent backdrop) | Reproduced & Fixed | u15 | Painted `modal_backdrop_color(mode)` on `shortcuts_backdrop`. |
| **SUS-29** | `spell_check.rs` (`SQUIGGLE_COLOR`), `main.rs` (WASM theme) | Reproduced & Fixed | u13, u16 | `spell_check::squiggle_color(ctx)` uses `palette(ctx).danger`; WASM uses `resolve_web_theme`. |
| **SUS-30** | `player_frame.rs` (drawer/tab/dim contrast) | Reproduced & Fixed | u16 | Added `contrasting_fg(bg)` and `dim_text_color(text, bar)`. |
| **SUS-31** | `properties.rs` (`B/I/U` tooltips & accessible names) | Reproduced & Fixed | u4 | Provided via `widgets::mixed_toggle` (`"Bold (Ctrl+B)"`, etc.). |
| **SUS-32** | `shortcuts.rs` (raw close `Button`) | Reproduced & Fixed | u15 | Replaced with `widgets::icon_button(ui, Icon::X, "Close (Esc)")`. |
| **SUS-33** | `toolbar.rs` + 31 `add_enabled` sites | Reproduced & Fixed | u7–u16 | Added `.on_disabled_hover_text(...)` across toolbar, inspector, and all dialogs. |
| **SUS-34** | `widgets.rs` caret, `status.rs` chip, `coach.rs` btn, `shortcuts.rs` close | Reproduced & Fixed | u7, u15 | Caret `16→20 pt`, validation chip `20→24 pt`, coach `24→26 pt`, shortcuts `icon_button`. |
| **SUS-35** | `toolbar.rs` (`"Story"` label) | Reproduced & Fixed | u7 | Renamed to `"Map"` with `"Course map"` tooltip (per Reviewer Note #4). |
| **SUS-36** | `layers.rs`, `ai_settings.rs`, `translate_ai.rs`, `triggers.rs` | Reproduced & Fixed | u10–u14 | Removed trailing colons; reworded `"Stop clicks..."` to `"Block clicks..."`. |
| **SUS-37** | `report.rs` (`"Don't ask again"`), `main.rs` (`"Don't Save"`) | Kept intentionally | u15, u16 | Platform-standard dialog buttons retained per Reviewer Note #4. |
| **SUS-38** | 23 dialogs & labels mixing Title/Sentence case | Reproduced & Fixed | u11–u13 | Standardized dialog titles and field labels to sentence case. |
| **SUS-39** | 27 `DragValue` units + `multi_text_size` `TextEdit` | Reproduced & Fixed | u4, u8, u11, u13 | Added `px`/`pt`/`ms`/`pts`/`×` suffixes; migrated `multi_text_size` to `DragValue`. |
| **SUS-40** | `player_frame.rs` (fullscreen tooltip when active) | Reproduced & Fixed | u6 | Shows `"Exit full screen"` when `fullscreen_on` is true. |
| **SUS-41** | `dialog.rs` + 18+ dialogs `Esc` & 5 popup-stacking bugs | Reproduced & Fixed | u3, u9–u15 | Centralized `Dialog::open` `Esc` check with `!egui::Popup::is_any_open(ctx)`. |
| **SUS-42** | `apps/coursecraft/src/main.rs` (4 shell `egui::Modal`s) | Reproduced & Fixed | u16 | Handled `modal.should_close()`, `Esc` consumption, and `set_max_width(480.0)`. |

---

## 4. Sweep Coverage Summary

### 4.1 Editor Sweep (`coursecraft-ui-egui`)

Command: `cargo test -p coursecraft-ui-egui -j 4 uiux_ -- --nocapture`  
Result: **4 passed, 0 failed** in **33.83s** (headless in-memory; no PNGs written to `target/uiux-snapshots/`).

- **`uiux_sweep_all_surfaces`**:
  - **61 surfaces** (`4` inspector tabs, `4` bottom tabs, `9` question types + results, `course-map`, `master-view`, `canvas-rulers`, `window-history`, `22` `WINDOWS` entries).
  - **Matrix:** `3` window sizes (`1024×600` [or `1536×900` at `1.5×`], `1280×800`, `1920×1080`) × `2` themes (`Dark`, `Light`) × `2` UI scales (`1.0`, `1.5`) in desktop mode (`file_ops_available = true`), plus `2` themes at `1280×800` scale `1.0` in web mode (`file_ops_available = false`).
  - **Frames audited:** `854` (`61 × 14`), **Unopened:** `0`, **Violations:** `0` (verified in `target/uiux-sweep-report.json`).
- **`uiux_sweep_scale_2_and_fixtures`**: `12` core panels/dialogs at scale `2.0` (`2048×1200`, `1920×1080`) plus `empty_course` and `long_names_course` (120-char Unicode/accented/CJK names + 32 long variables) at `1024×600` and `1280×800` (`0` unallowlisted violations).
- **`uiux_sweep_modals_and_tooltips`**: Preview (`0xCC`) & Shortcuts (`0x99`) modal backdrop checks in Dark & Light, plus `check_tooltips_and_disabled()` across the default editor (`0` violations).
- **`allowlist_stays_within_budget_and_has_no_blanket_entries`**: Verified `7` specific `TextEdit::singleline` entries (`<= 20` budget) in `crates/ui-egui/src/uiux_allowlist.rs`.

### 4.2 Learner Player Sweep (`coursecraft-render`)

Command: `cargo test -p coursecraft-render -j 4 uiux_ -- --nocapture`  
Result: **2 passed, 0 failed** in **0.95s**.

- **`uiux_player_sweep`**: `6` browser viewports (`320×568`, `390×844`, `844×390`, `768×1024`, `1280×720`, `1920×1080`) × `3` course themes (`default-dark`, `bright-light`, `saturated-warm`) × `2` states (`default`, `drawer` open) using `long_names_player_course()` = `36` audited states (`0` violations).
- **`uiux_player_preview_vs_published_pixel_parity`**: Verified byte-for-byte deterministic `snapshot_player` rendering at `960×540` (`0` diffs).
- **Manual step remaining (Reviewer Note #8):** Live browser DOM/canvas verification in Chromium (offline CI has no Playwright/Chromium).

---

## 5. Key Before/After Visual Comparison Notes

1. **Font picker (Light & Dark):** Before: highlighted/selected row painted `p.accent_hover` (`#1A75E6`) behind dark `p.text` and `"restricted"` badge (`~1.6:1`–`3.2:1` in Light mode), showed 11 rows, and `Esc` cleared canvas selection. After: uses `p.accent_subtle` (`>= 4.5:1`), `24.0 pt` rows (`>= 12` visible), hover tooltip on elided names, and `Esc` closes only the picker.
2. **Desktop toolbar at 1100 px:** Before: with `Publish` visible (`file_ops_available = true`), the left group overflowed into the centred Insert group, `"Story"` violated clean-room naming, and carets were `16 pt` wide. After: collapses cleanly to icons below `1220.0 pt`, reads `"Map"` (`"Course map"` tooltip), widens carets to `20.0 pt`, and adds disabled tooltips for Master View & Format Painter.
3. **Preview modal backdrop + full-screen:** Before: faint default `egui::Modal` backdrop, fixed `960×640` cap, no fullscreen sync. After: `PREVIEW_BACKDROP_COLOR` (`0xCC` / ~80% black), 8 perimeter resize grips (`480×320` min) with session memory, `can_fullscreen = true` + `F11`/`Cmd+Ctrl+F` + strip sync, and partial-preview subset popup.
4. **Player drawer edge clip:** Before: `draw_drawer` overrode parent clip rects, sliding in from outside the preview modal over editor chrome, and long titles overflowed. After: clipped to `ui.clip_rect().intersect(full).intersect(rect)` with `paint_truncated_text` + hover tooltips and `contrasting_fg` for custom course accents.
5. **Layer Settings:** Before: raw `ComboBox`es clipped `"Resume where it left off"`, labels had trailing colons and `"Stop clicks..."`, `OK` lacked a disabled tooltip, and `Esc` closed the dialog while a combo was open. After: `styled_combo`, colon-free labels, `"Block clicks on the base layer"`, disabled `OK` tooltip, and centralized `Dialog` `Esc`.
6. **Media Library:** Before: 3 nested `ScrollArea`s with a fixed `260 pt` table that never grew on resize, narrow `Kind` combo, and missing disabled tooltips. After: `DialogUi::body_unscrolled` + dynamic `table_height`, `styled_combo`, truncated filenames with tooltips, disabled tooltips, and `"Media library"` sentence case.

---

## 6. Clean-Room Sign-Off

Static grep across `crates/ui-egui/src/` and `crates/render/src/` confirms:
- **Zero** occurrences of `"Story"` or `"Story view"` in user-facing UI strings (only the negative assertion in `toolbar_menu_tests.rs:172`).
- **Zero** occurrences of `"Freeform"` for question types (`QuestionKind::Freeform` renders `"Custom question (graded by a condition)"`, verified by `u11_triggers_vars_question_tests.rs:538`; the only `"Freeform"` string in `crates/ui-egui/src/lib.rs:702` is the vector shape-drawing tool alongside `"Line"`, `"Arrow"`, `"Curve"`, and `"Scribble"`).
