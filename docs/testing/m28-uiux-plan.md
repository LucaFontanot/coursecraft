# M28 UI/UX Refinement Pass — Test & Fix Plan

**Role:** Product Design & QA Specification (planning only — zero source code edited here)  
**Target release bar:** Zero cramped, unreadable, clipped, overlapping, inconsistent, theme-broken, or awkward controls anywhere in CourseCraft — across the desktop editor, web editor (WASM), in-editor preview, and published learner player (SCORM 1.2/2004, xAPI, cmi5, LTI 1.3, Web).  
**Design reference:** `DESIGN.md` ("The Quiet Studio") and `crates/render/src/design.rs` (normative tokens).  
**Clean-room commitment:** CourseCraft matches the authoring and learner capabilities expected of modern slide-based eLearning tools, but its visual language, layout, typography, icons (Phosphor MIT), component architecture, and terminology are strictly original (`docs/contributing.md:21-30`). We never copy another product's look, CSS/JS, icons, or coined terminology (for example, we use **Course map** rather than "Story view" and **Custom question** rather than "Freeform").

> **Implementer Guardrails (`AGENTS.md`, `docs/contributing.md`)**
> 1. **Never crash:** Zero `unwrap`/`expect`/`panic!`/`unreachable!`/`todo!` in non-test code; `unsafe_code = "forbid"` everywhere except `coursecraft-video-native`.
> 2. **Everything is a command:** Every edit triggered from any panel, toolbar, dialog, or menu dispatches an atomic, undoable command through `coursecraft-engine`.
> 3. **One runtime:** Never add play-time logic outside `coursecraft-logic`. Preview (`crates/ui-egui/src/preview.rs`) and published player (`apps/coursecraft-player`) share `coursecraft_render::PlayerFrame`.
> 4. **Layering & WASM:** `cargo xtask ci` (`fmt`, `clippy -D warnings`, `test --workspace`, `layers`, `wasm`) must stay green after every step. Never break `wasm32-unknown-unknown` for L0–L5 or the web editor/player.
> 5. **Test first (red → green):** Every fix step writes automated failing tests first, confirms they catch the defect, then implements the minimal, systematic fix.

> **Reviewer notes (Claude, read first; they override the plan where they differ)**
> 1. **Already being fixed before this plan starts.** Owner-feedback steps h1–h6 run first on the same branch: h1 outline drawer clipped to the player, h2 even shape outlines, h3 preview backdrop ~80% black + resizable + full screen, h4 preview only chosen slides with an end-of-preview popup, h5 style buttons show the selection's real style (with a mixed state), h6 the control bar gets its own strip and never covers the slide. So SUS-11, SUS-16, SUS-21, SUS-22, SUS-27 (preview part) and parts of Steps 4 and 6 may already be done: **read `docs/bug-hunt/2026-11-22-m28.md` and the code first, keep what works, add only the missing tests and fixes. Never undo or redo an h-step.** T14.7 "captions sit above the strip" now means inside the slide area, which sits above the bar's own strip.
> 2. **Suspects are suspicions.** Reproduce each with a failing test before changing code; if it doesn't reproduce, say so in the report and move on. Line numbers drift: grep for the function.
> 3. **Backdrop thresholds (UX-15):** the in-editor Preview is ~80% black (`alpha >= 0xCC`) in both themes, as the owner asked. Other modals: `alpha >= 0x99` in both themes.
> 4. **Wording (UX-11, SUS-37):** the positive-wording rule is for toggles and checkbox labels. Keep platform-standard dialog buttons such as "Don't Save" and "Don't ask again" (that's what macOS and Windows users expect). Do fix trailing colons, Title Case vs sentence case, and the "Story" toolbar label (the clean-room rule in `docs/contributing.md` renamed "Story view" to "Course map": use "Map" with the tooltip "Course map").
> 5. **Nested scrolls (UX-02):** a horizontal scroll inside a vertical one (timeline tracks), and independent side-by-side panes, are allowed. Only vertical-in-vertical is a violation.
> 6. **The sweep's goal (Step 17):** 0 MUST violations. A finding that is truly fine may go on an allowlist (`crates/ui-egui/src/uiux_allowlist.rs`) with one line per entry giving the reason. No blanket entries (whole rule or whole surface), at most ~20 entries in total, each reviewed in the report.
> 7. **`accesskit`:** in egui 0.37 accesskit is always compiled in (no Cargo feature; `egui = { features = ["accesskit"] }` fails). Do not touch `Cargo.toml` for it: use the re-exported `egui::accesskit` types (no new dependency), call `ctx.enable_accesskit()` in the harness and read `platform_output.accesskit_update`. If the tree turns out unusable in tests, fall back to the painted-shape walker and say so; don't block on it.
> 8. **Web player in a real browser (§3.3.2):** no Playwright or Chromium download, and no network. Use the native `PlayerFrame` sweep. The browser check stays a documented manual step for the reviewer.
> 9. **Dialog size memory (SUS-15):** remembering it within the session is enough. Don't add cross-session persistence.
> 10. **Out of room?** Finish in step order. Each step must leave `cargo xtask ci` green and the bug-hunt log updated with what is left undone.

---

## Executive Summary

| Section | Contents | Count |
|---|---|---:|
| **1. UI Surface Inventory** | Every panel, tab, menu, popup, dialog, modal, app shell, web editor, and player chrome surface mapped to files & test reachability | **48 surfaces** |
| **2. Automated UI/UX Rule Book** | Measurable thresholds covering geometry, scroll stability, contrast, hit targets, state fidelity, copy, keyboard, dialogs, modals, undo, and parity | **22 rules (`UX-01`–`UX-22`)** |
| **3. Automated Test Harness & Sweep Architecture** | `TestHarness` extensions (`accesskit` + painted-shape walker + contrast + nested-scroll + tooltip + scale), 3×2×2 sweep matrix, pixel snapshots, and web player responsive checks | **7 harness subsystems** |
| **4. Known Suspects Watchlist** | Confirmed defects and high-risk UI code found by static code audit before writing a single line of code | **42 concrete suspects** |
| **5. Concrete Test Cases by Area** | 14 surface areas × 8–10 concrete test cases each, with exact file:line citations and expected results | **120 concrete test cases** |
| **6. Ordered Fix Steps** | Small, single-session steps (red → green) culminating in a green full sweep and `docs/testing/m28-uiux-report.md` | **18 sessions** |

---

## 1. Complete UI Surface Inventory

Every user-facing UI surface read directly from the codebase, its source file(s), primary controls, and how an automated test reaches it.

| # | Surface | File(s) | Main Controls | How a Test Reaches It |
|---|---|---|---|---|
| 1 | **Menu bar & submenus** | `crates/ui-egui/src/menu.rs:556-1095`, `lib.rs:4101-4108` | File, Edit, Insert, Slide, Arrange, View, Tools, Help menus; shortcut labels; disabled reasons (`adapter_reason`); theme & zoom items | Visible on every editor frame; `h.click_at(h.find_text("File"))` (or `Edit`/`Insert`/`Slide`/`Arrange`/`View`/`Tools`/`Help`) |
| 2 | **Context menus** | `crates/ui-egui/src/menu.rs:1105-1350`, `slides.rs:148`, `canvas.rs`, `timeline.rs` | Object right-click menu, canvas background right-click menu, slide thumbnail `⋯`/right-click menu, timeline row menu | `h.right_click_at(obj_center)` / `h.right_click_at(canvas_empty)` / `h.right_click_at(thumb_center)` |
| 3 | **Top toolbar & popups** | `crates/ui-egui/src/toolbar.rs:346-795`, `widgets.rs:200-458` | Left: Add slide (+caret), Preview (+caret), Publish (`file_ops_available`), Course map toggle, Format Painter; Centre: Text, Shape (+gallery), Image, Media (+menu), Button, Question (+menu), Trigger; Right: Undo, Redo, Format/Animate/Slide/Course switches | Always visible when `!show_welcome`; set `editor.file_ops_available = true` to include Publish; click carets/dropdowns via `find_icon` / `find_text` |
| 4 | **Slide navigator & Master view** | `crates/ui-egui/src/slides.rs:107-311`, `lib.rs:4176-4185` | Scene headers (inline rename), numbered 16:9 thumbnails, `⋯` action menu, drag-reorder, `+ New slide` / `+ New master` footer, Master view banner & exit | Always visible on left; enter Master view via `editor.ui.master_view = true` or `View ▸ Slide master` |
| 5 | **Canvas, rulers, zoom pill & inline editors** | `crates/ui-egui/src/canvas.rs:1-1358`, `rulers.rs:1-361`, `crop.rs:1-488`, `snap.rs:1-527` | Slide stage, selection handles, yellow adjust diamonds, equal-spacing guides, inline rich-text editor, image crop handles, Bezier path editor, bottom-right zoom pill (`Fit`, `50%`–`200%`), top/left rulers & draggable guides | Default central panel (`story_view == false`); double-click text for `inline_edit`; toggle rulers via `editor.ui.show_rulers = true`; crop via `editor.ui.crop_edit` |
| 6 | **Course map (formerly Story view)** | `crates/ui-egui/src/course_map.rs:1-874`, `lib.rs:4208-4210` | Zoomable/pannable 2D scene/slide cards (`ScrollArea::both`), branching arrows, multi-select, scene add/rename/delete, keyboard navigation | `editor.ui.story_view = true` or click toolbar Course map button / `Ctrl+Shift+M` |
| 7 | **Inspector — Format tab** | `crates/ui-egui/src/properties.rs:1880-2947, 3250-7580` | Arrange icon strip; Position/Size (`X/Y/W/H`, rotation, opacity); Text (`FontPicker`, Autofit, `B/I/U/S/x²/x₂`, size, colour, align, spacing, Link, Text Style); Shape Fill (Solid/Gradient/Picture/None), Outline & Dash, Effects (Shadow/Glow); Table rows/cols/cells; Input/Slider/Dial/Meter/Timer/ScrollPanel/Media/Lottie/Video/Marker/Web controls; Multi-select bulk panel; Accessibility (`Alt text`, `Focus order`) | `SnapshotSelect { inspector_tab: Some(InspectorTab::Format), object: Some(id) }` across each `ObjectKind` + multi-select (`selection.set_many(...)`) |
| 8 | **Inspector — States section** | `crates/ui-egui/src/states.rs:1-572` | State list (`Normal`, `Hover`, `Down`, `Visited`, `Disabled`, `Selected`, custom), state preview toggle, `+ Add`, quick-add suggestion combo & pills, rename/delete | Format tab with any object selected; scroll to States section or select state via `editor.ui.preview_state` |
| 9 | **Inspector — Animate tab** | `crates/ui-egui/src/properties.rs:2948-3038, 8350-8520` | Entrance & Exit animation combos (effect, direction, easing, duration, delay), Motion path list (add/edit/easing/orient/reverse/delete) | `SnapshotSelect { inspector_tab: Some(InspectorTab::Animate), object: Some(id) }` |
| 10 | **Inspector — Slide tab** | `crates/ui-egui/src/properties.rs:3039-3197, 7650-8020` | Slide title, Slide Master combo, Draw-from-Bank controls, Background fill/picture, Slide transition (type, direction, duration), Navigation buttons & Next-unlocks overrides, When revisiting, Duration, Speaker notes + "Narrate notes…" | `SnapshotSelect { inspector_tab: Some(InspectorTab::Slide) }` (normal slide, bank draw slide, and master slide) |
| 11 | **Inspector — Course tab** | `crates/ui-egui/src/properties.rs:3199-3242` | Course title, slide count, pass mark summary, quick links to `Course settings…` and `Theme…` | `SnapshotSelect { inspector_tab: Some(InspectorTab::Course) }` |
| 12 | **Font picker popup** | `crates/ui-egui/src/font_picker.rs:1-619` | Sticky search box, single virtualised list (`In this course`, `Recent`, `On this computer` A–Z), `restricted` badge, keyboard `↑↓/Enter/Esc`, pinned `Add font…` footer | Format tab on text/shape/button/table/timer → click Font combo button |
| 13 | **Colour picker popup & Eyedropper** | `crates/ui-egui/src/properties.rs:1310-1430`, `eyedropper.rs:1-58` | 18×18 swatch + hex field, Eyedropper button & full-window sampling loupe, Palette/Theme/Recent swatch rows, egui HSV/RGBA picker | Format/Slide/Theme panel → click any colour swatch button |
| 14 | **Shape gallery (popup & window)** | `crates/ui-egui/src/shape_gallery.rs:1-336` | Search box, category headers, wrapped grid of 32 px shape & draw-tool icon buttons with tooltips | Toolbar `Shape` popup, Format `Change shape` popup, or `SnapshotSelect { window: Some("shape-gallery") }` |
| 15 | **Bottom panel — Timeline tab** | `crates/ui-egui/src/timeline.rs:1-345+`, `timeline_playback.rs:1-55+` | Tab strip (Timeline/Triggers/Variables/Question), play/pause scrub bar, time ruler, cue points (add/rename/delete), object track rows (eye/lock/expand/drag-trim timing bars), zoom slider | `SnapshotSelect { bottom_tab: Some(BottomTab::Timeline) }` |
| 16 | **Bottom panel — Slide Layers pane** | `crates/ui-egui/src/layers.rs:1-1079` | Front-to-back layer list, Base Layer pinned below hairline, eye visibility toggle, gear settings icon, `+ New layer`, duplicate & delete icons, `Dim base` checkbox | Visible on right side of bottom panel when `!master_view` |
| 17 | **Bottom panel — Triggers tab** | `crates/ui-egui/src/triggers.rs:1600-2199` | Filter/search, trigger rows (object/slide/layer grouped), enable/disable, reorder up/down, duplicate/copy/paste/delete, `+ Add trigger`, broken-reference warning badges | `SnapshotSelect { bottom_tab: Some(BottomTab::Triggers) }` |
| 18 | **Bottom panel — Variables tab** | `crates/ui-egui/src/variables.rs:1-297` | Add-variable bar (name, type combo `Text`/`Number`/`True/False`, initial value, `+ Add`), scrollable variable list with inline default editor, usage count, delete | `SnapshotSelect { bottom_tab: Some(BottomTab::Variables) }` |
| 19 | **Bottom panel — Question tab (12 kinds + Results)** | `crates/ui-egui/src/question.rs:1-166+` | Prompt, choices/correct toggles, Drag-and-Drop pairs & options, Hotspot regions, Matching/Sequence/Fill-in/Numeric/Short-answer/Essay/Likert/Custom editors, attempts, points, feedback layers, Results slide passing score & included questions | `SnapshotSelect { bottom_tab: Some(BottomTab::Question), slide: Some("s-q-mc" \| ... \| "s-results") }` |
| 20 | **Trigger editor dialog** | `crates/ui-egui/src/triggers.rs:2200-3597` | Action combo & dynamic target fields, When event combo & fields, optional recursive Condition tree (`and`/`or`), JavaScript code box, OK/Cancel footer | `SnapshotSelect { window: Some("trigger-editor") }` or double-click a trigger row |
| 21 | **Layer settings dialog** | `crates/ui-egui/src/layers.rs:1080-1278` | Name, Duration (`ms`), visibility/base-layer checkboxes, Allow seeking combo, When revisiting combo, Cancel/OK footer | `SnapshotSelect { window: Some("layer-settings") }` or click layer gear icon |
| 22 | **Course settings dialog (incl. Resources & Glossary)** | `crates/ui-egui/src/course_settings.rs:1-652`, `resources_ui.rs:1-261` | General (Title, ID, Language BCP-47), Player chrome flags, Logo picker, Navigation mode, Next unlocks, Quiz passing score, Background audio & volume, Resources & Glossary editors | `SnapshotSelect { window: Some("course-settings") }` |
| 23 | **Theme & Palette dialog** | `crates/ui-egui/src/theme_dialog.rs:1-199` | Preset themes, Accent/Background/Text/Surface/Player bar swatches, Heading/Body fonts, custom palette colours | `SnapshotSelect { window: Some("theme") }` |
| 24 | **Text styles manager dialog** | `crates/ui-egui/src/text_styles_ui.rs:1-329` | Named course text styles list, preview swatches, add/edit/apply/delete styles | `SnapshotSelect { window: Some("text-styles") }` |
| 25 | **Slide size dialog** | `crates/ui-egui/src/slide_size_dialog.rs:1-132` | Aspect presets (16:9, 4:3, 16:10, Portrait), custom W × H `DragValue`s, scale-objects option, Apply/Cancel | `SnapshotSelect { window: Some("slide-size") }` |
| 26 | **Publish course dialog** | `crates/ui-egui/src/publish.rs:1-512` | Publish target radios (SCORM 1.2, SCORM 2004 3rd/4th, cmi5, xAPI, LTI 1.3, Web), standard-specific options, pre-publish validation status, progress/done feedback | `SnapshotSelect { window: Some("publish") }` |
| 27 | **Export Video & Export for Review dialogs** | `crates/ui-egui/src/export_dialog.rs:1-284` | Video resolution/FPS combo/progress; PDF & Word review options (notes, layers, slide numbers); status bar job progress | `SnapshotSelect { window: Some("export-video") \| Some("export-review") }` |
| 28 | **Preferences dialog** | `crates/ui-egui/src/preferences.rs:1-139` | Autosave interval (`min`), Check spelling as you type, Versions to keep, Theme (`System`/`Light`/`Dark`), Default slide size radios | `SnapshotSelect { window: Some("preferences") }` |
| 29 | **Version history dialog** | `crates/ui-egui/src/version_history.rs:1-136` | Kept version table (timestamp, size, slide count / `Damaged`), `Open as Copy`, `Delete`, folder-project git explanation | `SnapshotSelect { window: Some("version-history") }` |
| 30 | **Find and Replace dialog** | `crates/ui-egui/src/find.rs:1-452` | Find & Replace inputs, Match case / Whole word checkboxes, match count summary, scrollable result list with highlights, Replace / Replace All | `SnapshotSelect { window: Some("find") }` or `Ctrl+F` |
| 31 | **Check Spelling dialog** | `crates/ui-egui/src/spell_check.rs:1-303+` | Current misspelling context, suggestions list, Change / Change all / Ignore / Add to dictionary | `Tools ▸ Check spelling…` or `F7` |
| 32 | **AI Settings & Agents dialog** | `crates/ui-egui/src/ai_settings.rs:1-1026`, `connect_agent.rs:1-65` | Tabs (AI provider / Agents / Voice), provider combo, base URL, API key, model, Test connection, local agent detection & MCP config copy | `SnapshotSelect { window: Some("ai-settings") \| Some("agents") }` |
| 33 | **AI Authoring dialogs (Outline, Document, Alt Text, Captions, Narration, Translate)** | `outline.rs:1-259`, `document.rs:1-494`, `alt_text.rs:1-943`, `audio_ai.rs:1-1264`, `translate_ai.rs:1-969`, `elevenlabs.rs:1-369` | Multi-line inputs, file picker, model/voice/language combos, progress/cancel, per-item accept/edit checkboxes, Apply footer | `SnapshotSelect { window: Some("outline" \| "document" \| "alt-text" \| "speech" \| "translate") }` + `editor.open_captions_dialog(...)` |
| 34 | **Record Narration dialog** | `crates/ui-egui/src/record_narration.rs:1-426` | Input device combo, live level meter, Record/Stop/Play/Insert buttons, timer | `SnapshotSelect { window: Some("record-narration") }` |
| 35 | **Translation Export & Import Report dialogs** | `crates/ui-egui/src/translation.rs:1-108` | Target language code input, Export XLIFF button, scrollable import report | `SnapshotSelect { window: Some("export-translation") \| Some("translation-report") }` |
| 36 | **Media Library & Delete confirmation** | `crates/ui-egui/src/media_library.rs:1-645` | Toolbar (Import, Insert on slide, Replace, Delete, Delete unused), Filter & Kind combo, thumbnail table, Places-using-file list, total size footer | `SnapshotSelect { window: Some("media-library") }` |
| 37 | **Fonts Manager & Remove confirmation** | `crates/ui-egui/src/fonts.rs:1-154` | Embedded course font list, Add font…, Remove + confirmation dialog | `SnapshotSelect { window: Some("fonts") }` |
| 38 | **Insert Interaction & Web Object dialogs** | `crates/ui-egui/src/interaction_gallery.rs:1-407`, `interaction_insert.rs:1-188`, `web_insert.rs:1-119` | Pre-built interaction template cards (Tabs, Accordion, Flip cards, Process, Scenario, etc.) + Web URL/folder insert form | `SnapshotSelect { window: Some("interaction-gallery") \| Some("web") }` |
| 39 | **History (Undo stack) dialog** | `crates/ui-egui/src/history.rs:1-120` | Clickable undo/redo command history list | `Edit ▸ History…` (`editor.ui.show_history = true`) |
| 40 | **Link dialog & Delete confirmations** | `crates/ui-egui/src/properties.rs:3956-4030`, `slides.rs:629, 2133`, `timeline.rs:2013, 2066` | Text hyperlink URL/Slide picker, Delete scene/master confirmations, Rename cue dialog | Select text → Link button; right-click scene/master → Delete; double-click cue |
| 41 | **Status bar, Check Course (Validation + Accessibility), Error History & PPTX Report** | `crates/ui-egui/src/status.rs:1-487` | Placement hint, file/dirty title, validation pill, transient error/status/format-painter notices, automation badge; Check Course (validation + WCAG audit findings + manual checklist); Error History; PPTX Import Report | Status bar always visible; `SnapshotSelect { window: Some("validation" \| "error-history" \| "pptx-report") }` |
| 42 | **Report a Problem & Crash Recovery dialogs** | `crates/ui-egui/src/report.rs:1-448` | Crash notice (`Report` / `Don't ask again` / `Close`), problem description, scrubbed technical details toggle, Copy / Open GitHub | `SnapshotSelect { window: Some("report") \| Some("crash") }` |
| 43 | **Welcome screen** | `crates/ui-egui/src/welcome.rs:1-746` | Centered 720 px card, wordmark, `Back to course`, 2×2 action tiles (New + template menu, Open, Import PPTX, Explore examples), Recent courses list | `SnapshotSelect { window: Some("welcome") }` |
| 44 | **Coach marks & Keyboard shortcuts cheat sheet** | `crates/ui-egui/src/coach.rs:1-204`, `shortcuts.rs:1-356` | 3-step anchored onboarding popovers (`1 of 3`, Skip/Next/Done); modal keyboard shortcuts cheat sheet with platform keycap chips | `SnapshotSelect { window: Some("coach-1" \| "coach-2" \| "coach-3" \| "shortcuts") }` |
| 45 | **In-Editor Preview modal** | `crates/ui-egui/src/preview.rs:1-1048` | Modal overlay, Restart / Effect Log (`F2`) / Live Variables (`F3`) / Resize / Full-screen / Close controls, embedded `PlayerFrame`, side-by-side Live Variables & Effect Log | `SnapshotSelect { window: Some("preview") }` or `F12` / toolbar Preview |
| 46 | **Desktop App Shell modals & drop overlay** | `apps/coursecraft/src/main.rs:2854-3022, 3428-3470` | Native window (`ViewportBuilder`), `Drop to import` overlay, `Save changes to …?` modal, `This file/folder changed…` conflict modal, `Restore unsaved changes?` modal, `Error` (+ Open Latest Version) modal | Unit/integration tests in `apps/coursecraft/src/m27_files_ui_tests.rs` & new M28 shell harness driving `EditorApp::show_app` |
| 47 | **Web Editor shell (WASM)** | `apps/coursecraft/src/main.rs:3024-3120, 3472-3530`, `apps/coursecraft/index.html`, `Trunk.toml` | Full-viewport `<canvas>`, async browser file pickers/downloads, disabled desktop-only actions with tooltips, browser theme detection | `cargo xtask wasm` + headless browser viewport tests (or native `file_ops_available = false` simulation) |
| 48 | **Learner Player chrome & Web Player shell** | `crates/render/src/{chrome,player_frame,drawer,focus,access}.rs`, `apps/coursecraft-player/src/{app,a11y,web_object}.rs`, `apps/coursecraft-player/index.html` | Letterboxed slide on `#0E0E10`, floating transport strip (logo, play/pause, replay, seekbar, tabular time, mute+volume, captions, speed popup, outline button, title, `n / N`, prev/next, fullscreen), sliding 4-tab drawer (Menu, Resources, Glossary, Transcript + Start over), lightbox close `×`, review & notice banners, audio captions, ended card, loading/error screens, `F2` log panel, DOM a11y mirror | `coursecraft_render::snapshot::snapshot_player` + `PlayerFrame` headless harness across 6 viewport sizes + headless Chromium check on published web package |

---

## 2. Automated UI/UX Rule Book

Every rule below has a stable ID, a **measurable pass/fail threshold**, and an automated enforcement method. Rules marked **MUST** fail CI; rules marked **SHOULD** emit warnings unless elevated per surface.

| Rule ID | Category | Rule | Measurable Threshold | Enforcement |
|---|---|---|---|---|
| **UX-01** | Dropdowns & Popups | Popups & `ComboBox` menus must be wide enough for their longest item (or truncate cleanly with full-text tooltip) and tall enough for long lists. | Popup width `>= min(longest_item_width + 36 pt, screen.width() - 32 pt)` (min `180 pt` for named-item combos); popup height `>= min(12 * row_height, screen.height() * 0.75)` (min `300 pt` when ≥ 12 items exist). Zero right-edge clipping of item text without tooltip. | Harness opens combo/popup, measures popup rect, item text rects, and visible item count. |
| **UX-02** | Scrolling | Exactly **one** scroll area per scrollable list or dialog body — **no nested vertical scroll areas**. | For any two active `ScrollArea` clip rects `A` and `B` on the same `LayerId` (excluding independent side-by-side split panes where `A.intersect(B)` is empty), `!(A.contains_rect(B) \|\| B.contains_rect(A))`. | Harness inspects `ScrollArea` registry / nested clip rects per layer in every frame. |
| **UX-03** | Scroll Stability | Scrollbars and popup/panel widths must stay stable while scrolling regardless of item label lengths. | Scrolling a list or popup from top (`offset = 0`) to bottom in 5 steps must change container width and scrollbar track `x` by `<= 1.0 pt`. | Harness scrolls every open list/popup and asserts `abs(w_i - w_0) <= 1.0`. |
| **UX-04** | Clipping & Overlap | Zero clipped text, zero clipped framed controls, and zero overlapping chrome labels across all supported sizes, themes, and scales. | Text horizontal clip overhang `<= 1.0 pt`; widget right/top/bottom clip overhang `<= 1.5 pt`; inter-label overlap `<= 2.0 pt` in both axes. Tested at `1024×600`, `1280×800`, `1920×1080` × `Dark`/`Light` × scale `1.0`/`1.5` (plus `2.0` stress pass). | Extended `check_layout` in `test_harness.rs` across full sweep matrix. |
| **UX-05** | Hit Target Size | Every interactive button, toggle, tab, chip, and close icon must meet minimum clickable area. | Compact editor controls: `>= 24×24 pt` (standard icon buttons `>= 26×26 pt` per `DESIGN.md`); primary buttons `>= 32 pt` tall; toolbar carets `>= 20×44 pt` (up from 16 pt); player strip buttons `>= 36×36 pt`; mobile web player touch targets `>= 40×40 pt` (effective hit rect). | Harness walks every interactive widget rect (`Role::Button`, `CheckBox`, `RadioButton`, `Tab`, `ComboBox`) and checks `width >= min_w && height >= min_h`. |
| **UX-06** | Colour Contrast | All visible chrome text and icons must pass WCAG 2.1 AA contrast in **both** Light and Dark themes (excluding intentionally disabled controls and slide-content regions). | Normal text (`< 18 pt` regular / `< 14 pt` bold): `contrast_ratio(fg, bg) >= 4.5`. Large text (`>= 18 pt` or `>= 14 pt` bold) and UI icons/focus rings/borders of interactive inputs: `contrast_ratio(fg, bg) >= 3.0`. | Harness composites painted background `Shape::Rect` behind each `TextShape` and calls `coursecraft_render::contrast_ratio(fg, bg)`. |
| **UX-07** | Theme Fidelity | Chrome must never hard-code raw RGB colours that break Dark/Light switching or bleed course colours into editor chrome. | Zero `Color32::from_rgb`/`WHITE`/`BLACK` literals in `crates/ui-egui/src` outside approved tokens (`WHITE` on `p.accent`/`p.danger` primary fill, slide canvas defaults, and `palette(ctx)`). Dark-to-Light switch changes all panel/raised/text fills to `Palette::light()`. | Static grep check + dual-theme pixel/shape check in sweep. |
| **UX-08** | Disabled Controls | Every disabled control must look disabled (`text_disabled`, non-interactive) AND explain why on hover. | When `enabled == false`: painted colour matches `p.text_disabled` (contrast `< 4.5` vs enabled `p.text`), clicking produces zero commands/requests, and hovering for `1.0 s` paints a non-empty `Order::Tooltip` explaining how to enable it. | Harness detects disabled nodes via `accesskit`/widget probe, hovers center, asserts tooltip painted. |
| **UX-09** | Toggle & Mixed State Fidelity | Every toggle/checkbox/segmented button must reflect the exact model state of the current selection — never stale, never wrong at run boundaries, and **explicitly indeterminate/mixed** when a selection mixes values. | Single-run/cursor: bold/italic/underline/strikethrough/baseline/font at boundary `i` matches run starting at `i` (half-open `[start, end)`), never the previous run. Multi-run or multi-object mixed state: shows distinct mixed indicator (neither plain-off nor active-on), and clicking toggles uniformly to ON then OFF. | Unit + harness state-matrix tests across single-run, boundary, multi-run, and multi-object selections. |
| **UX-10** | Icon Tooltips & Accessible Names | Every icon-only button and 1–2 character symbol button (`B`, `I`, `U`, `S`, `×`, `⋯`, `+`, `-`, `↑`, `↓`, carets) must have a human-readable tooltip AND accessible name. | For every interactive widget whose painted label is empty, a Phosphor PUA glyph (`U+E000..=U+F8FF`), or `<= 2` chars (excluding `"OK"`, `"No"`, `"On"`): `accesskit` label is non-empty AND hovering paints a tooltip `>= 3` chars (with shortcut when one exists). | Harness probes every icon/short-label widget rect in sweep. |
| **UX-11** | Copy, Casing, Positive Wording & Units | Consistent `DESIGN.md` typography & UX writing: sentence case, no trailing colons on labels/headers, positive labels (no double negatives), explicit units on numeric inputs, and clean-room terminology. | Zero trailing `:` on form labels/section headers; zero `"Don't ..."` / double-negative toggles; zero competitor coined terms (`"Story"`, `"Story view"`, `"Freeform"`) in UI strings; every dimension/time/size/angle/percent `DragValue` displays its unit (`px`, `pt`, `ms`, `s`, `%`, `°`, `×`, `min`). | Automated string-audit test over painted labels + static lint test over `crates/ui-egui/src`. |
| **UX-12** | Spacing & Column Alignment | Consistent 4 pt spacing rhythm (`design.rs`) and aligned label columns in inspector and dialogs. | Every multi-field dialog uses `dialog::Form` so all field left edges match within `<= 1.0 pt`. Inspector label column fits all standard labels without wrapping/eliding. Panel inner padding `>= 12 pt` (`PANEL_MARGIN`). | Harness checks field `min.x` alignment in forms + `inspector_label_column_shows_the_long_labels`. |
| **UX-13** | Keyboard Navigation & Focus | Predictable Tab order, visible focus ring, `Enter` confirms primary action, and `Esc` closes the **topmost** transient layer only (popup/combo → inline edit/tool → modal/dialog → selection). | Pressing `Esc` while a `ComboBox`/`Popup` is open inside a dialog closes ONLY the popup (dialog stays open); second `Esc` closes the dialog. Focused controls show a `>= 2.0 pt` accent ring with `>= 3.0:1` contrast. | Harness keyboard-stack tests on every dialog with combos + player `FocusManager` tests. |
| **UX-14** | Dialog Bounds, Resizing & Memory | Dialogs open fully on-screen below menu bar & toolbar; dialogs with growing lists/tables/logs/editors are resizable and remember their resized dimensions across reopen within the session. | `window.min.y >= top_inset`, `screen.contains_rect(window)` within `1.0 pt`; every list/table/tree/log dialog has `.resizable(true)`; dragging resize corner changes persisted size and survives close+reopen. | `check_windows` + resize-and-reopen harness test across all dialogs. |
| **UX-15** | Modal Backdrop Dimming | Modal overlays (Preview, Shortcuts, Unsaved Changes, Conflict, Restore Autosave, Error) must dim the editor enough that background chrome does not distract or read as interactive. | Backdrop `Shape::Rect` covering `screen_rect` has alpha `>= 0x99` (~60% black) in Dark mode and `>= 0x73` (~45% black) in Light mode; clicking editor controls behind backdrop has zero effect. | Harness inspects backdrop `RectShape` alpha + offscreen pixel brightness check + click-through test. |
| **UX-16** | Z-Order & Clamping | Nothing opens off-screen, behind another window, or leaks outside its clipping container (e.g., player outline drawer inside preview modal, coach marks, speed popup). | Every `Area`/`Popup`/`Window` rect satisfies `screen.contains_rect(rect)` (within `1.0 pt`). `PlayerFrame` drawer and scrim paint strictly within `full` (`drawer_clip == drawer.intersect(full)`). Layer order strictly `Background < Middle < Foreground < Tooltip`. | Harness checks layer ordering, rect clamping, and offscreen pixel bleed outside preview modal. |
| **UX-17** | Empty States | Empty panels, lists, and dialogs must state what belongs there and offer one clear next action — never a blank rectangle or duplicate primary buttons. | Empty course/slide/triggers/variables/media/version-history/find/transcript views paint explanatory secondary text (`>= 12` chars) and at most 1 primary button per view. | Harness empty-course fixture sweep asserting non-blank guidance text on every empty surface. |
| **UX-18** | Error Messages | User-facing error banners and modals must say **what happened** and **what to do next**, with zero raw Rust debug formatting (`{:?}`, `Err(`, backtraces) or unbounded overflow. | Error text matches non-technical prose (no `Err(`, `panicked`, `0x` pointer dumps), wraps within modal width (`<= 520 pt`), and offers an actionable button or fix hint. | Harness error-state tests across preview errors, file errors, AI errors, publish errors, and player `State::Failed`. |
| **UX-19** | Long-Name Stress Resilience | 120-character names (course, slide, scene, layer, object, variable, font, media file), plus CJK/emoji-free Unicode/accented strings, must never break layout, clip adjacent controls, or expand panels off-screen. | With a hostile long-name stress course loaded, full sweep produces **0** `clipped`, `overlap`, `off-window`, or `clipped-widget` findings; truncated text exposes full string on hover. | Dedicated `long_names_stress_course()` fixture run through the full sweep. |
| **UX-20** | Numeric Input Consistency | All numeric fields must support both **direct typing** (click → type → Enter/blur commits; Esc reverts) and **horizontal dragging**, with safe clamping for hostile input (`NaN`, negative, huge). | Every numeric property in Format/Animate/Slide/Layer/Dialog accepts `set_field` typing AND `drag_by` dragging, updating the exact same model field with valid bounds. | Harness numeric-input matrix test across inspector & dialogs. |
| **UX-21** | Universal Undo | Every model mutation initiated from any panel, toolbar, popup, or dialog must push exactly one atomic undo step (`Ctrl+Z` restores exact prior `Course` JSON). | After performing action from UI, `course_after != course_before`, single `Ctrl+Z` yields `course_undone == course_before` (and `Ctrl+Shift+Z` restores `course_after`). | Harness UI-edit → undo → redo roundtrip across all authoring panels. |
| **UX-22** | Preview-Equals-Published Parity | What a control previews in the editor (canvas + in-editor `Preview`) must match what the published web player (`PlayerFrame`) renders and executes. | Identical slide layout, text wrapping, font metrics, theme accent, seekbar, drawer bounds, and control visibility between `Preview` and `PlayerFrame` at matching viewport sizes (pixel diff `<= 1%` excluding preview chrome bar). | Headless `snapshot_player` vs `Preview` frame comparison + `coursecraft-cli simulate` parity. |

---

## 3. Automated Testing Architecture

### 3.1 Extending `crates/ui-egui/src/test_harness.rs`

`crates/ui-egui/src/test_harness.rs` already captures per-layer `ClippedShape` lists (`capture_layers`, `:577-591`), `content_regions` (`:550`), `window_rects` (`:700-713`), and checks `clipped`, `overlap`, `off-window`, `clipped-widget`, `window-off-screen`, and `window-over-menu-bar` (`:716-896`). It currently has **five major blind spots** that let this week's bugs slip through:
1. It has **no widget/role/accessibility tree** (cannot check hit target sizes, disabled states, toggle states, or missing tooltips/accessible names).
2. It has **no contrast checker** (cannot catch `font_picker.rs:519` dark text on bright blue fill in Light mode).
3. It has **no nested-`ScrollArea` detector** (missed 12 dialogs with nested scroll areas inside `DialogUi::body`).
4. It has **no UI scale (`zoom_factor`) or `file_ops_available = true` (desktop Publish button) coverage** (missed toolbar overflow at 1100–1200 px and scale 1.5/2.0).
5. It does **not open popups/combos/menus** in the sweep and does **not inspect modal backdrops** (missed light preview backdrop, narrow combos, and drawer bleed).

#### Proposed Harness Architecture (`crates/ui-egui/src/test_harness.rs`)

1. **Enable `accesskit` in `crates/ui-egui` dev-dependencies:**
   - `Cargo.toml:45` uses `egui = "0.37"` and `Cargo.lock` already includes `accesskit 0.25.2` (enabled on `eframe`).
   - Add `egui = { workspace = true, features = ["accesskit"] }` under `[dev-dependencies]` in `crates/ui-egui/Cargo.toml` (zero impact on production WASM or release binaries; `xtask layers` allows dev-deps).
   - In `TestHarness::new_with_config(course, size, mode, zoom_factor)`, call `ctx.enable_accesskit()` and `ctx.set_zoom_factor(zoom_factor)` (note: `ctx.set_zoom_factor` scales logical coordinates inside the fixed physical viewport, matching real OS/user UI scaling at `1.0`, `1.5`, and `2.0`).
   - On every audited frame, capture `output.platform_output.accesskit_update` and walk all `TreeUpdate.nodes` into a flat `Vec<WidgetNode>`:
     ```rust
     pub(crate) struct WidgetNode {
         pub id: accesskit::NodeId,
         pub role: accesskit::Role,
         pub label: Option<String>,
         pub bounds: Rect,        // converted from accesskit::Rect (physical -> logical via 1.0 / ppp)
         pub disabled: bool,
         pub toggled: Option<accesskit::Toggled>, // False / True / Mixed!
         pub layer: Option<LayerId>,
     }
     ```
   - **Fallback if a custom painted widget omits `widget_info`:** pair `accesskit` nodes with `ctx.memory(|m| ...)` + `capture_layers(ctx)` painted `RectShape`/`TextShape` entries. Any clickable area that painted a button rect without an `accesskit` node is itself flagged as an **accessibility gap** (`missing-widget-info`).

2. **Painted-Shape Contrast Auditor (`check_contrast`):**
   - For each non-content `TextShape` in `layers`:
     - Extract foreground `Color32` (`override_text_color` or section `format.color`), effective font size `font_id.size`, bold/semibold weight, and whether `fg == p.text_disabled` (skip contrast error for disabled text, but verify `disabled == true` on the owning `WidgetNode`!).
     - Find the painted background colour at `item.visible.center()` by walking all earlier `Shape::Rect` / `Shape::Mesh` fills in the same layer and lower layers that contain `center`, alpha-compositing bottom-to-top onto `p.app` (or `PLAYER_GROUND` in player tests).
     - Compute `cr = coursecraft_render::contrast_ratio(fg, bg)`.
     - Assert `cr >= 4.5` for normal text (`< 18.0 pt` regular / `< 14.0 pt` bold) and `cr >= 3.0` for large text and Phosphor icons (`U+E000..=U+F8FF`).

3. **Nested `ScrollArea` Detector (`check_nested_scrolls`):**
   - Add a tiny `#[cfg(test)]` (or `pub(crate)` debug) hook, or inspect egui's `ScrollArea` clip-rect hierarchy: in egui 0.37, every `ScrollArea` stores `egui::scroll_area::State` in `ctx.memory` and registers an inner clip rect (`ui.clip_rect()`) smaller than or distinct from the outer layer clip rect. Even more reliably, introduce a 15-line `pub(crate) mod scroll_audit` (zero cost when `not(test)`): wrap or record every `ScrollArea` outer rect per `LayerId` in `ctx.data_mut` (plus `DialogUi::body` records its scroll viewport rect), and in `check_nested_scrolls` report any two scroll viewports `(A, B)` on the same `LayerId` where `A` strictly contains `B` (or overlaps `B` by `> 80%` of `B`'s area).
   - Complement with a static test (`no_scroll_area_inside_dialog_show`) that verifies the rule structurally across every `Dialog::new` caller.

4. **Scrollbar & Popup Stability Probe (`check_popup_stability`):**
   - Helper `harness.audit_combo(combo_label)` and `harness.audit_popup(open_fn)`:
     1. Clicks/opens the combo or popup and runs 2 warm-up frames.
     2. Locates the `Order::Foreground` popup layer and its bounding `Rect`.
     3. Asserts `screen.contains_rect(popup_rect)` (rule **UX-16**).
     4. Asserts popup width fits the widest visible item (or truncated items have hover tooltips) and popup height fits `>= min(12, item_count)` items when viewport allows (**UX-01**).
     5. Scrolls down in 5 increments via `scroll_over(popup_rect.center(), -180.0)` and records `popup_rect.width()` and scrollbar rect `min.x` on each step; asserts max drift `<= 1.0 pt` (**UX-03**).
     6. Presses `Escape` once and asserts the popup closed while any parent `Dialog` remained open (**UX-13**).

5. **Icon Tooltip & Disabled Explanation Probe (`check_tooltips_and_disabled`):**
   - Walks all `WidgetNode`s with interactive roles (`Button`, `ToggleButton`, `CheckBox`, `ComboBox`, `MenuItem`):
     - If `bounds.width() < min_w || bounds.height() < min_h` → `hit-target-too-small` (**UX-05**).
     - If icon-only (empty text or Phosphor PUA glyph) or 1–2 char symbol (`"B"`, `"I"`, `"U"`, `"S"`, `"×"`, `"+"`, `"-"`, `"⋯"`):
       - Assert `node.label.as_deref().is_some_and(|s| s.len() >= 3)` (**UX-10**).
       - Move pointer to `bounds.center()`, advance virtual clock by `1.0 s`, run frame, assert `Order::Tooltip` painted non-empty text (**UX-10**).
     - If `node.disabled`:
       - Hover at `bounds.center()`, advance `1.0 s`, assert `Order::Tooltip` painted an explanation (**UX-08**).

6. **Modal Backdrop & Z-Order Auditor (`check_modals_and_backdrops`):**
   - Whenever a modal (`preview`, `shortcuts`, `unsaved_changes`, `changed_on_disk`, `restore_autosave`, `error_modal`) is active:
     - Find full-screen backdrop `Shape::Rect` covering `>= 95%` of `screen_rect`.
     - Assert `fill.a() >= 0x99` (Dark) or `fill.a() >= 0x73` (Light) (**UX-15**).
     - Click a known background toolbar button outside the modal and assert zero editor commands fire (**UX-15**).

7. **Full Sweep Matrix Test (`crates/ui-egui/src/uiux_sweep_tests.rs`):**
   - Replaces and extends `layout_tests.rs`'s limited sweep with a comprehensive matrix:
     - **Window sizes (3):** `(1024.0, 600.0)` minimum, `(1280.0, 800.0)` laptop, `(1920.0, 1080.0)` desktop.
     - **Themes (2):** `ThemeMode::Dark`, `ThemeMode::Light`.
     - **UI scales (2 in full matrix + 1 targeted pass):** `1.0` and `1.5` across all surfaces (`3 × 2 × 2 = 12` configs per surface), plus a `2.0` scale pass at `(1920.0, 1080.0)` and `(1440.0, 900.0)` (effective logical `960×540` / `720×450`) for core panels & dialogs.
     - **Host modes:** Both desktop (`file_ops_available = true`, `automation_available = true`) and web (`file_ops_available = false`).
     - **Course fixtures:** `examples/showcase/course.json`, an `empty_course()` fixture (empty states), and a `long_names_course()` fixture (120-char names, 30+ variables, 30+ media files, 20+ slides, 50+ system fonts).
     - **Readable report output:** Groups findings by `[UX-XX]` rule ID and surface, prints summary table + first 50 detailed lines on failure, and writes a machine-readable JSON summary to `target/uiux-sweep-report.json` (in `target/`, never tracked).

### 3.2 Snapshot & Pixel Verification (`crates/render/src/snapshot.rs` & `offscreen.rs`)

Use existing CPU offscreen rasterisation (`coursecraft_ui_egui::snapshot::snapshot_editor` and `coursecraft_render::snapshot::snapshot_player`, both powered by `crates/render/src/offscreen.rs:render_ui` with zero GPU/display required):
- **Preview backdrop dimming pixel test:** Render `snapshot_editor(..., SnapshotSelect { preview: true, .. })` in Dark and Light modes; sample a pixel over a bright white slide area outside the preview modal and assert luminance drops by `>= 60%` (Dark) / `>= 45%` (Light).
- **Drawer edge-clip pixel test:** Render `Preview` with outline drawer mid-animation (`drawer_progress = 0.5`) and fully open (`1.0`); sample pixels `4 px` to the left of the preview modal's left edge and assert they equal the dimmed backdrop colour (zero drawer pixels leaking outside the player edge).
- **Before/after visual diff archive:** Capture PNGs for key surfaces (toolbar at 1024/1100/1440, Format inspector with font picker open, Preview modal, Player drawer at phone/tablet/desktop) into `target/uiux-snapshots/{before,after}/` for human visual sign-off in `docs/testing/m28-uiux-report.md`.

### 3.3 Learner Player & Web Player Testing at Multiple Browser Sizes

1. **Native headless `PlayerFrame` layout & pixel sweep (`crates/render/src/player_frame/tests.rs` & `crates/render/src/uiux_player_tests.rs`):**
   - Run `PlayerFrame` with `check_layout` + `check_contrast` + hit-target checks across **6 viewport sizes**:
     - `320×568` (small phone portrait)
     - `390×844` (modern phone portrait)
     - `844×390` (phone landscape)
     - `768×1024` (tablet portrait)
     - `1280×720` (laptop 16:9)
     - `1920×1080` (desktop 16:9)
   - Exercise every player state: `Playing`, `Paused`, `Hidden`, `Drawer` (all 4 tabs: `Menu`, `Resources`, `Glossary`, `Transcript`, plus mid-slide animation `progress = 0.5`), `SpeedPopupOpen`, `LightboxOpen`, `NoticeBanner`, `ReviewBanner`, `CaptionsOn`, `Ended`, `Loading`, `Failed`.
   - Test with both default dark course bar (`#1B1B1B`) and custom light/saturated course themes (`bar = #FFFFFF`, `bar_text = #1D1D1F`, high/low contrast accents) and 120-char course/slide/resource/glossary titles.
2. **Headless Web Player verification (WASM + DOM a11y layer):**
   - Local loopback HTTP server serving a freshly published web package (reusing `apps/coursecraft/src/preview_server.rs` pattern on `127.0.0.1:0`, zero internet).
   - Optional local Playwright / headless Chrome verification script (kept under `xtask` or documented as a manual/local check if Chromium is installed, skipped cleanly in offline CI without network): loads the published player at `390×844`, `768×1024`, and `1440×900`, verifies zero JS console errors, `<canvas>` fills viewport without scrollbars, DOM `a11y.rs` button rects match visual controls within `2 px`, keyboard `Tab`/`Enter`/`Escape`/`M`/`C` work, and `Full screen` API toggles cleanly.

---

## 4. Known Suspects to Check First (Confirmed in Source Code)

Every item below was verified directly in the current codebase. The implementer should write failing tests reproducing each suspect before fixing it.

### 4.1 Narrow / Short `ComboBox`es (Missing `.width(...)` and/or `.height(...)`)

| ID | File & Lines | Confirmed Defect |
|---|---|---|
| **SUS-01** | `crates/ui-egui/src/properties.rs:2346` (`prop_multi_font`) & `:5505` (`prop_timer_font`) | Still uses raw `egui::ComboBox` with **no `.width` and no `.height`** instead of `font_picker::show_font_picker` — misses search, system fonts, recent fonts, and stable width. |
| **SUS-02** | `crates/ui-egui/src/layers.rs:1145` (`layer_allow_seek`) & `:1166` (`layer_revisit`) | Neither sets `.width(...)` (ignores `form.row`'s `width` argument with `|ui, _|`!) or `.height(...)`; `"Resume where it left off"` clips/truncates inside the 360 px dialog. |
| **SUS-03** | `crates/ui-egui/src/triggers.rs:53` (`tall_combo`) | Sets `.height(combo_popup_height(...))` but **never sets `.width(...)`**; long object/slide/variable names cause narrow closed combos and width-jumping popups. |
| **SUS-04** | `crates/ui-egui/src/properties.rs:3078, 3140, 3355, 3883, 3969` | Five inspector/link combos (`prop_master`, `prop_draw_bank`, `autofit`, `properties.rs:3883`, `link_slide_pick`) have **no `.width` and no `.height`**. `"Shrink text on overflow"` clips in narrow inspector. |
| **SUS-05** | `crates/ui-egui/src/ai_settings.rs:608`, `audio_ai.rs:989, 1021`, `course_settings.rs:409`, `export_dialog.rs:138`, `media_library.rs:222`, `record_narration.rs:206`, `translate_ai.rs:513`, `variables.rs:98` | Nine dialog/panel `ComboBox`es have **no `.width` and no `.height`** — notably `elevenlabs_voice`, `record_narration_device`, and `translate_common_language` which hold 20–100+ items and open in egui's tiny default 200 px popup! |
| **SUS-06** | `crates/ui-egui/src/properties.rs:1658, 1699, 1725, 4092, 4210, 4234, 5165, 5633, 5820, 6712, 6746, 7545, 7710, 7850, 7883, 7956, 8077, 8431, 8778`, `course_settings.rs:50, 90, 115`, `resources_ui.rs:175`, `states.rs:304`, `text_styles_ui.rs:109` | Twenty-five combos set `.width(...)` but **no `.height(...)`** — default egui `200 pt` popup height shows only ~6–7 items instead of ≥ 12 (hurts `anim_effect`, `prop_picture_src`, `prop_input_variable`, `slide_transition_combo`, `text_style_pick`). |

### 4.2 Nested `ScrollArea`s (Inside `DialogUi::body`'s Outer `ScrollArea`)

`crates/ui-egui/src/dialog.rs:217-228` wraps **every** `Dialog::show` / `DialogUi::body` in `ScrollArea::vertical()`. Twelve dialogs create a **second (or third!)** `ScrollArea` inside that body:

| ID | File & Lines | Confirmed Defect |
|---|---|---|
| **SUS-07** | `crates/ui-egui/src/status.rs:329-406` (`Check Course`) & `:451-465` (`Error History`) | `Check Course` has **THREE** scroll areas (outer `Dialog` + `validation-issues` max 240 px + `accessibility-findings` max 240 px). `Error History` nests a second `ScrollArea::vertical()` directly inside `Dialog::show`. |
| **SUS-08** | `crates/ui-egui/src/media_library.rs:109-321` (`Media Library`) | **THREE** scroll areas (outer `Dialog` + `media_library_table` capped at `max_height(260.0)` + `media_library_places` capped at `120.0`). Resizing the dialog taller never expands the table! |
| **SUS-09** | `crates/ui-egui/src/version_history.rs:72-131`, `find.rs:108-241`, `text_styles_ui.rs:194-225`, `translation.rs:92-104` | Each nests `ScrollArea::vertical()` inside `Dialog::show`; `version_history.rs:93` hard-caps `max_height(320.0)` even though the dialog is `.resizable(true)`. |
| **SUS-10** | `crates/ui-egui/src/document.rs:292-330`, `outline.rs:127-165`, `translate_ai.rs:476-640`, `shape_gallery.rs:75, 309`, `triggers.rs:2215, 2747` | Five more `Dialog`s nesting `ScrollArea::vertical()` (or `ScrollArea::both()` for JS editor at `triggers.rs:2747`) inside `DialogUi::body`'s scroll area. Note: `Dialog` should support a `.scrollable_body(false)` / `body_unscrolled` mode for dialogs that manage their own split/virtualised list! |

### 4.3 Fixed-Size / Non-Resizable Windows & Missing Minimum Window Size

| ID | File & Lines | Confirmed Defect |
|---|---|---|
| **SUS-11** | `crates/ui-egui/src/preview.rs:711-716` | Preview modal clamps size to `(960.0, 640.0)` max with **no resize handles, no maximize/full-screen button**, and never sets `frame.can_fullscreen = true`. On a 1920×1080 screen, the preview sits tiny in the middle! |
| **SUS-12** | `apps/coursecraft/src/main.rs:3441-3443` | `ViewportBuilder` sets `.with_inner_size([1440.0, 900.0])` with **no `.with_min_inner_size([1024.0, 600.0])`** — user can shrink the desktop window until panels collapse completely. |
| **SUS-13** | `crates/ui-egui/src/course_settings.rs:243`, `find.rs:108`, `ai_settings.rs:430`, `publish.rs:244`, `text_styles_ui.rs:194`, `interaction_gallery.rs:393`, `shape_gallery.rs:309` | Seven dialogs with growing lists/results/tabs are **non-resizable** (`resizable = false` default in `Dialog::new`). |
| **SUS-14** | `crates/ui-egui/src/shortcuts.rs:85-101` | Uses raw `Area::new` with `.fixed_pos` and fixed height/width instead of `Dialog`, bypassing `dialog_area` top-inset protection. |
| **SUS-15** | `crates/ui-egui/src/dialog.rs:154-177` | `Dialog` remembers size only while egui's `Memory` keeps the same ID (never persisted across sessions, and `find.rs:105` uses a fresh `window_id` on reopen, forgetting size/position!). |

### 4.4 Popups, Drawers & Overlays Positioned Without Clamping / Clipping

| ID | File & Lines | Confirmed Defect |
|---|---|---|
| **SUS-16** | `crates/render/src/player_frame.rs:583, 2555-2597` | **Owner-reported bug:** `drawer_rect(full, progress)` puts `x = full.left() - width + width * eased` (negative relative to `full.left()` during animation), and `draw_drawer` calls `ui.painter().with_clip_rect(rect)` and `child.set_clip_rect(rect)` — **overriding** `full`'s clip instead of `rect.intersect(full)`! In Editor Preview, the drawer visibly slides in from outside the modal over the editor chrome! |
| **SUS-17** | `crates/render/src/player_frame.rs:1967-1973` | Speed popup (`draw_speed_popup`) computes `x = anchor.center().x - width * 0.5` with **no horizontal clamping** to `full` / `screen_rect`, clipping off the right edge on narrow viewports when speed is near the strip's right end. |
| **SUS-18** | `crates/ui-egui/src/coach.rs:43-90` | Coach marks hardcode fixed pixel coordinates (`236.0`, `screen.max.x - 300.0`, `y = 88.0`) ignoring actual resized panel widths (`180–320` left, `260–400` right), have **no `.constrain_to(screen)`**, and ignore `Escape`. |
| **SUS-19** | `crates/ui-egui/src/welcome.rs:348-356` | Computes child height as `ground_rect.height() - card_y - 16.0` using screen-space `card_y` (which includes the 26 px menu bar offset `ground_rect.min.y`), shrinking the card by 26 px and causing premature clipping/scrolling at `1024×600` and scale `1.5×`. |
| **SUS-20** | `crates/ui-egui/src/toolbar.rs:348-368, 462-502` | When `editor.file_ops_available == true` (desktop!), the left toolbar group (`Add slide` + caret + `Preview` + caret + `Publish` + `Story` + `Paint`) totals ~340 px, **overflowing into the centred Insert group** between `1100 px` and `~1220 px` width (or at `1440 px` with `1.5×` scale)! Hidden from current `layout_sweep` because `TestHarness::new` leaves `file_ops_available = false`. |

### 4.5 Style / Selection State & Interactive Bugs

| ID | File & Lines | Confirmed Defect |
|---|---|---|
| **SUS-21** | `crates/ui-egui/src/properties.rs:1815-1851` (`style_at_cursor`) & `:1795-1813` (`effects_at_cursor`) & `:3269-3288` | **Owner-reported bug:** Uses inclusive upper bound `cursor_char <= current_offset + len` instead of half-open `< current_offset + len` (unlike `font_at_cursor` at `:1858`). When run 0 is italic (`0..2`) and run 1 is normal (`2..8`), placing the cursor at `2` or selecting run 1 (`2..8`, `sorted.start = 2`) matches run 0 and lights up **Italic active** even though the selection is NOT italic! Furthermore, `show_text_controls` never computes mixed state across a multi-run `char_range`. |
| **SUS-22** | `crates/ui-egui/src/properties.rs:2385-2459` | Multi-object text `Style` toggles (`B`, `I`, `U`) have `Option<bool>` (`None` = mixed), but `.selectable_label(italic == Some(true), ...)` renders **mixed as OFF** instead of an explicit indeterminate/mixed state. |
| **SUS-23** | `crates/render/src/player_frame.rs:2014-2020` | Speed popup's `Escape` close check is gated by `let Some(pos) = pointer.interact_pos() && !popup.contains(pos)` — pressing `Escape` while hovering the popup (or using keyboard-only with no pointer position) **never closes it**! |
| **SUS-24** | `crates/render/src/player_frame.rs:2806, 2906, 2983, 3034` | Drawer slide titles, tab labels, resource titles, and glossary terms are painted with raw `painter.text(...)` without `truncate_at_width` — long titles overflow the 320 px drawer and overlap the external-link icon at `:2990`. |
| **SUS-25** | `crates/ui-egui/src/toolbar.rs:479-490` | Course map toggle (`"Story"`) uses `toolbar_toggle` instead of `toolbar_button_enabled`, so in Master View it looks enabled, highlights on hover, and silently ignores clicks with no disabled tooltip. |

### 4.6 Hard-Coded Colours, Contrast Failures & Weak Modal Backdrops

| ID | File & Lines | Confirmed Defect |
|---|---|---|
| **SUS-26** | `crates/ui-egui/src/font_picker.rs:515-545, 560-573` | Highlighted/current font row paints `p.accent_hover` (`#1A75E6` in Light mode) behind `p.text` (`#1D1D1F` near-black) and `p.text_secondary` (`#5E5E66` for `"restricted"`) — **catastrophic WCAG AA contrast failure (~3.2:1 and ~1.6:1) in Light mode**! Should use `p.accent_subtle` (Selection Wash) or white text on `p.accent`. |
| **SUS-27** | `crates/ui-egui/src/preview.rs:714` & `apps/coursecraft/src/main.rs:2896, 2924, 2969, 2997` | **Owner-reported bug:** All 5 `egui::Modal::new(...)` calls use egui's default backdrop alpha (too light — editor shows right through Preview and app modals). Need explicit `.backdrop_color(...)` (e.g. `Color32::from_black_alpha(0xB8)` dark / `0x8C` light). |
| **SUS-28** | `crates/ui-egui/src/shortcuts.rs:68-76` | Shortcuts modal backdrop paints **nothing** (completely transparent), so editor chrome behind it looks active and distracting. |
| **SUS-29** | `crates/ui-egui/src/spell_check.rs:23`, `apps/coursecraft/src/main.rs:3513` | `SQUIGGLE_COLOR` hard-codes `Color32::from_rgb(224, 74, 74)` instead of `palette(ctx).danger`; WASM editor (`main.rs:3513`) hard-codes `ThemeMode::Dark`, ignoring browser `prefers-color-scheme`. |
| **SUS-30** | `crates/render/src/player_frame.rs:62-67, 1735-1736, 2896-2915` | Drawer colours (`DRAWER_BG/TEXT/SECONDARY/TERTIARY`) and tab selected text (`Color32::WHITE` on arbitrary course `accent`) do not verify contrast when a course sets a bright yellow/pastel `theme.accent`! Strip `dim` alpha (`0x55`) drops below 3:1 on dark bars. |

### 4.7 Missing Tooltips, Disabled Explanations & Hit-Target Violations

| ID | File & Lines | Confirmed Defect |
|---|---|---|
| **SUS-31** | `crates/ui-egui/src/properties.rs:2387, 2412, 2437, 3389, 3404, 3419, 5520, 5527, 5534` | `B`, `I`, `U` style toggles have **no `.on_hover_text(...)` tooltips** and no accessible names (`"Bold (Ctrl+B)"`, `"Italic (Ctrl+I)"`, `"Underline (Ctrl+U)"`), inconsistent with `S` (`:3436`), `x²`, and `x₂`. |
| **SUS-32** | `crates/ui-egui/src/shortcuts.rs:112-126` | Shortcuts close `×` uses raw `egui::Button` instead of `widgets::icon_button`: no tooltip, no accessible `widget_info`, non-standard hit target. |
| **SUS-33** | `crates/ui-egui/src/toolbar.rs:495-502` & 31 `add_enabled` sites (`ai_settings.rs:585`, `alt_text.rs:446, 457`, `audio_ai.rs:821`, `course_settings.rs:130`, `export_dialog.rs:177, 249`, `find.rs:253`, `layers.rs:807, 822, 1191`, `media_library.rs:190, 207`, `properties.rs:2927, 4782, 4830, 5967`, `publish.rs:357`, `slide_size_dialog.rs:110`, `states.rs:273, 295`, `translate_ai.rs:597, 706`, `translation.rs:36`, `triggers.rs:1888`, `variables.rs:137`) | 31+ disabled buttons lack `.on_disabled_hover_text(...)` explaining why they are disabled; `Format Painter` (`toolbar.rs:501`) keeps its enabled tooltip even when disabled. |
| **SUS-34** | `crates/ui-egui/src/widgets.rs:321` (`toolbar_caret_button`), `status.rs:248` (validation pill), `coach.rs:130` (`Next`/`Done`), `shortcuts.rs:39` | Caret button is `16×44 pt` (narrower than 20–24 pt comfortable target); status bar validation pill is `20 pt` tall; coach button is `24 pt` tall (`CONTROL_COMPACT - 2.0`). |

### 4.8 Copy, Clean-Room Naming, Negative Labels, Trailing Colons & Missing Units

| ID | File & Lines | Confirmed Defect |
|---|---|---|
| **SUS-35** | `crates/ui-egui/src/toolbar.rs:482` | **Clean-room violation:** Toolbar label still reads `"Story"` despite `docs/contributing.md:25` renaming "Story view" to **Course map** (tooltip already says `"Course map"`, label should say `"Map"` or `"Course map"`). |
| **SUS-36** | `crates/ui-egui/src/layers.rs:1111, 1115, 1119, 1124, 1130, 1134, 1140, 1161`, `ai_settings.rs:527`, `translate_ai.rs:507`, `triggers.rs:3474-3488` | Trailing colons on labels/headers (`"While this layer shows:"`, `"Use:"`, `"Translate to:"`, `"the following:"`) violate `DESIGN.md` "The No Colon Rule"; `layers.rs` checkboxes mix negative wording (`"Hide other layers"`, `"Stop clicks on the base layer"`). |
| **SUS-37** | `crates/ui-egui/src/report.rs:369`, `apps/coursecraft/src/main.rs:2909` | Negative/awkward button copy (`"Don't ask again"`, `"Don't Save"`) — should be positive/direct (`"Silence crash prompts"` or `"Dismiss"`, `"Discard changes"`). |
| **SUS-38** | 23 dialogs (`ai_settings.rs:430`, `course_settings.rs:243`, `find.rs:108`, `media_library.rs:109`, `version_history.rs:72`, `status.rs:329, 451, 472`, `export_dialog.rs:115, 205`, `slide_size_dialog.rs:73`, etc.) & `course_settings.rs:252-253`, `version_history.rs:109`, `find.rs:253` | Title Case (`"Course Settings"`, `"Course Title"`, `"Replace All"`, `"Open as Copy"`) mixed inconsistently with Sentence case (`"Publish course"`, `"Text styles"`, `"Keyboard shortcuts"`), violating `DESIGN.md` "The Sentence Case Rule". |
| **SUS-39** | 27 `DragValue` sites (`properties.rs:1558, 1581, 3496, 4528, 5542, 5746, 7162, 7419, 7440`, `slide_size_dialog.rs:91, 93`, `triggers.rs:2932`, etc.) & `properties.rs:2468` | Missing unit suffixes (`pt`, `px`, `ms`, `×`) on `DragValue`s; multi-select font size (`properties.rs:2468`) uses `TextEdit` instead of `DragValue`, breaking drag-to-scrub consistency (**UX-20**). |
| **SUS-40** | `crates/render/src/player_frame.rs:1930-1934` | Full-screen toggle shows `"Full screen"` tooltip even when already in full-screen mode (should show `"Exit full screen"`). |

### 4.9 Keyboard / Escape Stacking Bugs

| ID | File & Lines | Confirmed Defect |
|---|---|---|
| **SUS-41** | `crates/ui-egui/src/dialog.rs`, `course_settings.rs`, `preferences.rs`, `publish.rs`, `theme_dialog.rs`, `version_history.rs`, `media_library.rs`, `status.rs`, `fonts.rs`, `history.rs`, `slide_size_dialog.rs`, `web_insert.rs`, `export_dialog.rs`, `translation.rs`, `text_styles_ui.rs`, `coach.rs` | 18+ dialogs never close on `Escape` because `Dialog` does not centralize `Esc` handling; meanwhile `ai_settings.rs:474`, `find.rs:89`, `alt_text.rs:289`, `translate_ai.rs:248`, `layers.rs:1205` consume `Esc` **without** checking `!egui::Popup::is_any_open(ctx)`, closing the whole dialog instead of the open dropdown! |
| **SUS-42** | `apps/coursecraft/src/main.rs:2896, 2924, 2969, 2997` | All 4 desktop shell `egui::Modal`s ignore `modal.should_close()`, so `Escape` and outside-click do not cancel dismissible modals (`unsaved_changes`, `changed_on_disk`, `error_modal`); none bound `max_width`, so long paths stretch past the window. |

---

## 5. Concrete Test Cases by Area (120 Cases)

Every area below defines **8–10 concrete, automated test cases** with exact file/line citations and expected results.

### Area 1: Menu Bar, Context Menus & Top Toolbar
*Files:* `crates/ui-egui/src/menu.rs:556-1350`, `crates/ui-egui/src/toolbar.rs:346-795`, `crates/ui-egui/src/widgets.rs:83-458`

| Case | Scenario | Steps / Inputs | Expected Result |
|---|---|---|---|
| **T1.1** | Desktop toolbar fit with Publish visible (**SUS-20**) | Create `TestHarness` at `(1100, 700)` and `(1280, 800)` with `editor.file_ops_available = true`, both themes. | Zero overlap between left group (`Add slide`..`Format painter`), centre group (`Text`..`Trigger`), and right group (`Undo`..`Course`); `check_layout` returns 0 findings. |
| **T1.2** | Toolbar responsive icon-only mode at minimum width | Run at `(1024, 600)` at scale `1.0` and at `(1440, 900)` at scale `1.5` (`960×600` logical). | Labels collapse cleanly to icons; every toolbar button stays inside `toolbar_panel` with `>= 36×44 pt` hit target and displays full label + shortcut tooltip on hover (**UX-05**, **UX-10**). |
| **T1.3** | Clean-room toolbar label (**SUS-35**) | Inspect all painted texts in `toolbar.rs:479-485`. | Button label reads `"Map"` (or `"Course map"`), never `"Story"`; tooltip reads `"Course map (Ctrl+Shift+M)"` / `"Course map (⌘⇧M)"`. |
| **T1.4** | Master View toolbar disabled states & tooltips (**SUS-25**, **SUS-33**) | Set `editor.ui.master_view = true`; hover `Add slide`, `Preview`, `Course map`, `Format painter`, `Media`, `Question`. | Every unavailable button renders in `p.text_disabled`, ignores clicks, and shows `"Not available in Master View"` tooltip on hover (**UX-08**). |
| **T1.5** | Format Painter disabled tooltip when nothing selected (**SUS-33**) | Clear object selection on normal slide; hover Format Painter button (`toolbar.rs:495-502`). | Button is disabled and hover tooltip explains `"Select an object to copy its format"`. |
| **T1.6** | Toolbar caret hit targets & popup clamping (**SUS-34**) | Measure `toolbar_caret_button` (`widgets.rs:319-357`) and open `Add slide`, `Preview`, `Shape`, `Media`, `Question` popups at `(1024, 600)`. | Caret width `>= 20 pt`; every popup fits strictly inside `screen_rect`; `Esc` closes popup without affecting selection (**UX-05**, **UX-13**, **UX-16**). |
| **T1.7** | Menu bar disabled items explain why (`menu.rs:468-555`) | With no selection, open `Edit` and `Arrange` menus; hover disabled `Group`, `Ungroup`, `Align left`, `Bring forward`. | Each disabled item renders visually muted and paints a non-empty `on_disabled_hover_text` reason from `adapter_reason` (**UX-08**). |
| **T1.8** | Context menus near bottom-right window edge | Right-click an object positioned near `(max_x - 20, max_y - 20)` on canvas (`menu.rs:1105-1350`). | Context menu flips/clamps completely inside `screen_rect` with 0 `off-window` findings (**UX-16**). |
| **T1.9** | Menu shortcut formatting cross-platform (`menu.rs:18-164`) | Inspect all menu items in both dark/light themes; verify zero missing-glyph boxes (`□`) and valid contrast on shortcut text (`p.text_secondary` on `p.raised`). | All shortcut labels render with available Inter glyphs and `contrast_ratio >= 4.5` (**UX-06**). |

### Area 2: Right Panel — Inspector (Format, Animate, Slide, Course & States)
*Files:* `crates/ui-egui/src/properties.rs:1880-8797`, `crates/ui-egui/src/states.rs:1-572`

| Case | Scenario | Steps / Inputs | Expected Result |
|---|---|---|---|
| **T2.1** | Run-boundary Italic/Bold/Underline bug (**SUS-21**) | Create text object with Run 0 `"Hi"` (`italic: true`) and Run 1 `" there"` (`italic: false`). Place cursor at char `2` (and separately select `2..8`). | Format panel `I` toggle is **inactive** (`false`), matching Run 1 (`[2, 8)`), NOT Run 0 (**UX-09**). Repeat for `B`, `U`, `S`, and `baseline`. |
| **T2.2** | Multi-run mixed selection state (**SUS-21**) | Select range `0..8` spanning Run 0 (`bold: true, italic: false`) and Run 1 (`bold: false, italic: true`). | Both `B` and `I` show explicit **mixed/indeterminate** state (`Toggled::Mixed` + distinct visual indicator); clicking `B` sets the whole range bold in one undoable command (**UX-09**, **UX-21**). |
| **T2.3** | Multi-object mixed selection state (**SUS-22**) | Multi-select two text boxes (one italic, one normal; sizes `18` and `24`; fonts `Inter` and `Lora`). | `I` toggle renders mixed state (not plain OFF); Font shows `"Mixed"` using `show_font_picker` (**SUS-01**); Size supports both typing and dragging (`DragValue` with `" pt"` or mixed placeholder) (**UX-09**, **UX-20**). |
| **T2.4** | `B`/`I`/`U`/`S`/`x²`/`x₂` tooltips & accessible names (**SUS-31**) | Hover each text style button in single-select (`:3389-3465`), multi-select (`:2387-2459`), and Timer (`:5520-5534`). | All 6 buttons have tooltips (`"Bold (Ctrl+B)"`, `"Italic (Ctrl+I)"`, `"Underline (Ctrl+U)"`, `"Strikethrough"`, `"Superscript"`, `"Subscript"`) and non-empty `accesskit` labels (**UX-10**). |
| **T2.5** | Narrowest inspector width (`260 px`) across all `ObjectKind`s | Resize right panel to `260 px` at `(1024, 600)` and select Text, Shape (Gradient/Picture fill), Image, Button, Checkbox, RadioGroup, Slider, Dial, Input, Meter, Timer, ScrollPanel, Table, Marker, Video, Audio, Lottie, WebObject. | Zero `clipped`, `overlap`, or `clipped-widget` findings across all 18 object kinds in Dark and Light themes (**UX-04**). |
| **T2.6** | All inspector `ComboBox`es have sufficient width & height (**SUS-04**, **SUS-06**) | Open `Autofit`, `Fill type`, `Dash`, `Slide Master`, `Transition`, `Animation effect`, `Easing`, `Variable`, `Initial state`, `When revisiting`. | Every closed combo fits its selected label without clipping (or truncates with tooltip); every open popup is wide enough for its longest item and tall enough for `>= 12` items when present (**UX-01**). |
| **T2.7** | Numeric `DragValue` units, typing, dragging & undo (**SUS-39**) | For `X`, `Y`, `W`, `H`, `Rotation`, `Opacity`, `Font size`, `Glow size`, `Content height`, `Start/End ms`, `Lottie speed`: verify unit suffix, type new value (`set_field`), verify undo, then drag (`drag_by`) and verify undo. | Every control shows its unit (`px`, `°`, `%`, `pt`, `ms`, `×`), updates identically via typing and dragging, and reverts cleanly with one `Ctrl+Z` (**UX-11**, **UX-20**, **UX-21**). |
| **T2.8** | States section disabled buttons & delete hints (`states.rs:273-344`) | Hover disabled `+ Add` (empty name), already-added suggestion pill (`+ Hover`), and attempt to delete an in-use state. | Each disabled control explains why on hover (**UX-08**); States combo has proper width/height; deleting a referenced state displays a clear actionable message (**UX-18**). |
| **T2.9** | Animate & Motion Path panel empty & populated states (`properties.rs:2948-3038`) | Open Animate tab with no object selected, with object without animations, and with entrance + exit + 3 motion paths. | Empty state says what to select (**UX-17**); animation combos show full labels and ≥ 12 items; deleting/editing a motion path is 1-step undoable (**UX-21**). |
| **T2.10** | Slide & Course tabs at scale `1.5×` (`properties.rs:3039-3242`) | Switch to Slide tab (normal, draw slide, master) and Course tab at `1280×800`, scale `1.5`, Light & Dark. | All labels fit `LABEL_COL_WIDTH`; long course/slide titles truncate cleanly; `Course settings…` and `Theme…` buttons stay unclipped (**UX-04**, **UX-19**). |

### Area 3: Font Picker & Colour Picker / Eyedropper
*Files:* `crates/ui-egui/src/font_picker.rs:1-619`, `crates/ui-egui/src/properties.rs:1310-1430`, `crates/ui-egui/src/eyedropper.rs:1-58`

| Case | Scenario | Steps / Inputs | Expected Result |
|---|---|---|---|
| **T3.1** | Font picker Light & Dark contrast on selected/highlighted row (**SUS-26**) | Open `show_font_picker` in both `ThemeMode::Light` and `ThemeMode::Dark`; inspect current/highlighted row and `"restricted"` badge. | Row text and badge achieve `contrast_ratio >= 4.5` in both Light and Dark themes (Selection Wash `p.accent_subtle` + `p.text`, or `Color32::WHITE` on `p.accent`) (**UX-06**). |
| **T3.2** | Single scroll area & scrollbar stability with 100+ long font names | Populate `system_fonts` with 120 fonts mixing 4-char (`"Lato"`) and 80-char (`"Noto Sans Traditional Chinese ExtraCondensed SemiBold Italic"`) names; scroll top to bottom in 8 steps. | Exactly **1** `ScrollArea` inside popup (**UX-02**); popup width stays `380 pt` (`±1 pt`), scrollbar `x` never jumps (**UX-03**); truncated long names show full name on hover (**UX-19**). |
| **T3.3** | Font picker shows ≥ 12 rows & clamps to small viewports | Open font picker at `(1440, 900)` and `(1024, 600)` at scale `1.0` and `1.5`. | At `1440×900`, `>= 12` font rows visible simultaneously (**UX-01**); at `1024×600` scale `1.5`, popup clamps completely inside `screen_rect` with sticky search and pinned `Add font…` footer both visible (**UX-16**). |
| **T3.4** | Font picker keyboard navigation (`↑`, `↓`, `Enter`, `Esc`, search) | Open picker, type `"geo"`, press `ArrowDown`, `Enter`; reopen and press `Esc`. | Search auto-focuses; `↑`/`↓` skip section headers and scroll highlight into view; `Enter` applies font in one undoable step; `Esc` closes only the picker without clearing canvas selection (**UX-13**, **UX-21**). |
| **T3.5** | Multi-select & Timer use unified `FontPicker` (**SUS-01**) | Multi-select two text objects; also select a Timer object; click Font control in each. | Both open `show_font_picker` (not old raw `ComboBox`), with search, system fonts, and stable scroll (**UX-01**, **UX-02**). |
| **T3.6** | Colour picker popup clamping near bottom-right of inspector (`properties.rs:1343`) | Scroll inspector so a colour swatch sits near `(screen.max.x - 40, screen.max.y - 40)` and click it. | Colour picker popup opens above/left of swatch, 100% inside `screen_rect` with Eyedropper, Palette, Theme, Recent, and HSV square all visible (**UX-16**). |
| **T3.7** | Colour swatch hex validation & single-step undo (`properties.rs:1338-1410`) | Drag HSV picker continuously for 10 frames then release; also type invalid hex `"#zzz"` then valid `"#2288CC"`. | Dragging commits **1** undo step on pointer release (not 10); invalid hex never crashes and gives clear feedback; `Ctrl+Z` restores previous colour (**UX-21**). |
| **T3.8** | Eyedropper mode prompt, preview & `Esc` cancel (`eyedropper.rs:1-58`) | Click `Eyedropper` inside colour picker; move pointer over slide; press `Esc`. | Colour picker closes immediately; sampling loupe stays inside screen bounds; `Esc` cancels cleanly without modifying colour or clearing selection (**UX-13**, **UX-16**). |

### Area 4: Left Panel — Slide Navigator, Master View & Course Map
*Files:* `crates/ui-egui/src/slides.rs:1-311+`, `crates/ui-egui/src/course_map.rs:1-874`

| Case | Scenario | Steps / Inputs | Expected Result |
|---|---|---|---|
| **T4.1** | Navigator resize range (`180–320 px`) & long slide/scene titles | Load 120-char scene and slide titles; test navigator at `180 px`, `220 px`, and `320 px` in Dark and Light themes. | Titles truncate with `…` and show full title tooltip on hover; thumbnail number gutter (`22 px`) never overlaps thumbnail or title; 0 layout findings (**UX-04**, **UX-19**). |
| **T4.2** | Thumbnail `⋯` action button hit target, tooltip & popup | Hover a slide thumbnail; inspect `⋯` button; click it. | Hit target `>= 24×24 pt`, tooltip `"Slide options"` present, popup opens on-screen with Duplicate/Delete/Rename/Add to bank, `Esc` closes popup (**UX-05**, **UX-10**, **UX-13**). |
| **T4.3** | Inline scene rename commit, empty-name rejection & `Esc` revert | Double-click scene header, type `"New Scene"` + `Enter` (verify undo); repeat with `""` and with `Esc`. | `Enter` commits in 1 undo step; `Esc` restores old title; blank name is rejected cleanly without losing scene (**UX-13**, **UX-21**). |
| **T4.4** | Master View visual clarity & exit affordance | Enter `View ▸ Slide master` at `(1024, 600)` and `(1440, 900)`, Dark & Light. | Clear Master View banner/header with high-contrast `"Exit Master View"` button (`contrast >= 4.5`), bottom panel hidden cleanly, returning restores prior slide selection (**UX-06**, **UX-09**). |
| **T4.5** | Delete scene & Delete master confirmation dialogs (`slides.rs:629, 2133`) | Trigger delete on a scene (and a master); press `Esc`, reopen and confirm. | Dialog uses sentence case, sits inside `dialog_area`, `Esc` cancels cleanly, Confirm deletes in 1 undoable step (**UX-11**, **UX-13**, **UX-21**). |
| **T4.6** | Course map single `ScrollArea::both` & long-title cards (`course_map.rs:140`) | Open Course map with 5 scenes × 8 slides with 100-char titles; pan and zoom in/out. | Zero nested scroll areas; card titles truncate cleanly; branch arrows never obscure card text; 0 layout findings (**UX-02**, **UX-04**, **UX-19**). |
| **T4.7** | Course map keyboard guards when modal/preview is open | Open Course map, then open Preferences or Preview; press `Delete` / `Arrow` / `Enter`. | Modal/Preview owns keys — Course map never deletes or navigates slides behind an open dialog (**UX-13**). |
| **T4.8** | Single-slide course disabled Delete affordance | Load 1-slide course; open thumbnail `⋯` menu and right-click menu. | `Delete slide` is visibly disabled and explains `"A course needs at least one slide"` on hover (**UX-08**). |

### Area 5: Central Canvas, Rulers, Zoom & Direct Manipulation
*Files:* `crates/ui-egui/src/canvas.rs:1-1358`, `crates/ui-egui/src/rulers.rs:1-361`, `crates/ui-egui/src/crop.rs:1-488`

| Case | Scenario | Steps / Inputs | Expected Result |
|---|---|---|---|
| **T5.1** | Zoom pill position, hit targets & contrast at all window sizes | Test `(1024, 600)`, `(1280, 800)`, `(1920, 1080)` × Light/Dark × scale `1.0`/`1.5`. | Floating zoom pill stays inside canvas bottom-right, never overlaps rulers or bottom panel, `-/Fit/+` buttons `>= 26×26 pt` with tooltips and `>= 4.5:1` text contrast (**UX-05**, **UX-06**, **UX-10**). |
| **T5.2** | Rulers & guide lines contrast in Light and Dark themes (`rulers.rs`) | Enable `show_rulers = true` in both Dark and Light themes; drag a guide from top and left rulers. | Ruler tick labels achieve `>= 4.5:1` contrast on ruler background; guide drag preview never bleeds over toolbar or panels (**UX-06**, **UX-16**). |
| **T5.3** | Layer pills strip above canvas with 10+ long-named layers | Add 12 layers with 40-char names; test at `(1024, 600)` and `(1440, 900)`. | Pills truncate/scroll cleanly without clipping canvas or overflowing into inspector; active layer pill uses Selection Wash (`p.accent_subtle` + `p.accent_text`) (**UX-04**, **UX-09**, **UX-19**). |
| **T5.4** | Inline text edit `Esc` hierarchy & inspector sync | Double-click text box to start `inline_edit`; select a word; verify inspector `B/I/U` reflect exact selection; press `Esc`. | First `Esc` exits `inline_edit` (keeps object selected); second `Esc` clears object selection (**UX-09**, **UX-13**). |
| **T5.5** | Image crop mode affordance, `Enter` commit & `Esc` cancel (`crop.rs`) | Select image, enter crop mode, drag crop handle, press `Esc` (verify unchanged); repeat and press `Enter` (verify 1 undo step). | Crop handles visible with high contrast; `Esc` cancels cleanly; `Enter` commits one undoable command (**UX-13**, **UX-21**). |
| **T5.6** | Path point editor `Esc` & delete point (`canvas.rs:447-580`) | Enter path edit on a Bezier path; select point; press `Delete`; press `Esc`. | Point deletes in 1 undoable step (does not delete entire object); `Esc` exits path edit (**UX-13**, **UX-21**). |
| **T5.7** | Click-through guard when a menu/popup is open (`widgets.rs:34-81`) | Open `Insert` menu or `Shape` gallery over canvas; click a menu item or click outside to dismiss. | Underlying canvas object is NOT selected or dragged by the dismissing click (**UX-16**). |
| **T5.8** | Placement tool status hint & `Esc` cancel | Arm Shape/Text placement tool; check status bar; press `Esc`. | Status bar displays clear placement prompt (`"Click for the default size, or drag to draw …"`); `Esc` disarms tool without clearing selection (**UX-13**, **UX-17**). |

### Area 6: Bottom Panel — Timeline & Slide Layers Pane
*Files:* `crates/ui-egui/src/timeline.rs:1-345+`, `crates/ui-egui/src/layers.rs:1-1278`

| Case | Scenario | Steps / Inputs | Expected Result |
|---|---|---|---|
| **T6.1** | Bottom panel resize bounds & 200 px canvas guarantee (`lib.rs:4133-4168`) | Drag timeline top edge to minimum (`140 px`) and maximum at `(1024, 600)` and `(1440, 900)`. | Canvas always retains `>= 200 px` height; resize grab zone (`8 px`) never overlaps tab buttons; 0 clipped controls (**UX-04**, **UX-05**). |
| **T6.2** | Timeline icon buttons (Play, Eye, Lock, Cue, Zoom) tooltips & hit targets | Inspect all icon controls in `timeline.rs` and `layers.rs`. | Every icon button is `>= 24×24 pt` (`26×26 pt` standard) with descriptive tooltip and `accesskit` label (**UX-05**, **UX-10**). |
| **T6.3** | Timeline long object names & horizontal time ruler alignment | Load 100-char object names and 60 s slide duration; scroll vertically and horizontally. | Object names truncate in left `TRACK_HEADER_WIDTH` column with full name + timing tooltip on hover; bars never draw over track header (**UX-04**, **UX-19**). |
| **T6.4** | Timeline trim drag commits single atomic undo step | Drag right trim handle of an object bar by `+1000 ms` across 6 pointer frames; press `Ctrl+Z`. | One single undo step restores exact prior `end_ms` (**UX-21**). |
| **T6.5** | Slide Layers pane narrow width (`200 px` when bottom panel `< 900 px`) | Test `(1024, 600)` (bottom panel triggers 200 px layers pane) with long layer names, Dark & Light. | Eye icon, truncated layer name, gear icon, `+ New layer`, duplicate, delete, and `Dim` checkbox all fit without clipping or overlap (**UX-04**, **UX-19**). |
| **T6.6** | Slide Layers disabled Duplicate/Delete tooltips (`layers.rs:807-825`) | Select Base Layer (cannot delete/duplicate) and hover Duplicate/Delete buttons; also hover disabled Bring forward/Send backward. | Disabled buttons show clear reason on hover (**UX-08**). |
| **T6.7** | Layer Settings dialog copy, combos, disabled OK & `Esc` (**SUS-02**, **SUS-33**, **SUS-36**, **SUS-41**) | Open Layer Settings (`layers.rs:1084-1215`): inspect headers, checkboxes, `Allow seeking` & `When revisiting` combos, clear Name field, press `Esc` with combo open. | Zero trailing colons on section headers; positive/clear checkbox wording; combos fill `form.row` width so `"Resume where it left off"` never clips; disabled `OK` explains `"Enter a layer name"` on hover; first `Esc` closes combo only, second `Esc` closes dialog (**UX-01**, **UX-08**, **UX-11**, **UX-13**). |
| **T6.8** | Rename Cue dialog (`timeline.rs:2013, 2066`) | Double-click cue marker; verify `Dialog` bounds, `Enter` commit, `Esc` cancel, and undo. | Dialog sits inside `dialog_area`; `Enter` renames in 1 undo step; `Esc` cancels (**UX-13**, **UX-14**, **UX-21**). |

### Area 7: Bottom Panel — Triggers, Variables & Trigger Editor
*Files:* `crates/ui-egui/src/triggers.rs:1-3597`, `crates/ui-egui/src/variables.rs:1-297`

| Case | Scenario | Steps / Inputs | Expected Result |
|---|---|---|---|
| **T7.1** | Triggers empty state & broken-reference readability | Open Triggers tab on slide with 0 triggers; then on slide with 15 triggers including a broken target warning. | Empty state explains triggers and shows one `+ Add trigger` quiet button (**UX-17**); warning badges pass contrast in both themes (**UX-06**). |
| **T7.2** | Trigger Editor `tall_combo` width & 12+ item height (**SUS-03**) | Open Trigger Editor (`triggers.rs:2215`); open Action combo (20+ actions), Event combo, and Object target combo with 60-char object names. | Every closed combo is wide enough (not cramped); every open popup fits longest item width without horizontal clipping and displays `>= 12` items without scrollbar jitter (**UX-01**, **UX-03**). |
| **T7.3** | Trigger Editor JavaScript action — zero nested scroll (**SUS-10**) | Select `Run JavaScript` action in Trigger Editor (`triggers.rs:2747`); inspect active `ScrollArea`s. | Zero nested scroll areas (or cleanly decoupled single-scroll layout); code editor remains resizable and usable at `1024×600` (**UX-02**, **UX-14**). |
| **T7.4** | Trigger Editor `Esc` stacking with open combo (`triggers.rs:2213, 2287`) | Open Action combo inside Trigger Editor; press `Esc` once; press `Esc` a second time. | First `Esc` closes ONLY the Action combo (Trigger Editor stays open); second `Esc` closes Trigger Editor (**UX-13**). |
| **T7.5** | Trigger Editor numeric inputs & colon-free condition labels (**SUS-36**, **SUS-39**) | Select `Timeline reaches` event (`event_ms` at `:2932`) and compound `All of` / `Any of` conditions (`:3474-3488`). | `event_ms` `DragValue` displays `" ms"` unit; condition headers have no trailing colons; `Cmd/Ctrl+Enter` commits in 1 undo step (**UX-11**, **UX-13**, **UX-21**). |
| **T7.6** | Trigger row menu disabled states (`triggers.rs:1886-1915`) | Open row `⋯` menu while Trigger Editor is open; hover `Duplicate` / `Up` / `Down`. | Disabled actions show hover explanation; Move Up on first row and Move Down on last row are disabled with tooltips (**UX-08**). |
| **T7.7** | Variables tab type combo, disabled `+ Add` tooltip & empty state (`variables.rs:98-164`) | Open Variables tab on empty course; inspect type combo (`:98`), hover disabled `+ Add` (`:137`) with empty/duplicate name; add 25 long-named variables. | Empty state guides user (**UX-17**); type combo has explicit width; disabled `+ Add` explains `"Enter a unique variable name"` (**UX-08**); list scrolls smoothly with 1 scroll area (**UX-02**). |
| **T7.8** | Variable default value editing & undo (`variables.rs:243`) | Edit Text, Number (`DragValue`), and Bool variable defaults in Variables tab; press `Ctrl+Z`. | Number accepts typing & dragging; each edit is atomic and undoable (**UX-20**, **UX-21**). |

### Area 8: Bottom Panel — Question Editors (All 12 Kinds + Results Slide)
*Files:* `crates/ui-egui/src/question.rs:1-166+`

| Case | Scenario | Steps / Inputs | Expected Result |
|---|---|---|---|
| **T8.1** | All 12 `QuestionKind`s + Results slide at minimum window (`1024×600`) | Open Question tab for `MultipleChoice`, `MultipleResponse`, `TrueFalse`, `FillInBlank`, `Numeric`, `ShortAnswer`, `Essay`, `Likert`, `Matching`, `Sequence`, `Hotspot`, `DragDrop`, `Custom` (freeform), and `Results` in Dark & Light. | Zero `clipped`, `overlap`, or `clipped-widget` findings across all 13 views (**UX-04**). |
| **T8.2** | Clean-room terminology across question types | Inspect every label painted in Question tab and toolbar Question menu. | Uses `"Custom question"` (never `"Freeform"`); sentence case on all headers and buttons (**UX-11**). |
| **T8.3** | Non-question slide empty state (`question.rs`) | Open Question tab on a normal content slide (`question == None`). | Shows clear explanation + single quiet action to convert/insert a question, never a blank pane (**UX-17**). |
| **T8.4** | Choice/Pair add & delete buttons tooltips & minimum-choice guard | On a 2-choice Multiple Choice question, hover Delete choice button; add a choice; delete it. | Delete is disabled at minimum allowed count with hover tooltip explaining minimum choices; icon buttons `>= 26×26 pt` with tooltips (**UX-05**, **UX-08**, **UX-10**). |
| **T8.5** | Drag-and-Drop & Matching combos with long object/item names | Populate 15 drag items and 15 drop targets with 60-char names; open mapping combos. | Combos fit labels or truncate with full-name tooltip; popup shows `>= 12` items without scrollbar jump (**UX-01**, **UX-03**, **UX-19**). |
| **T8.6** | Points, Attempts & Passing Score numeric inputs | Inspect `DragValue`s for Points, Attempts, Numeric tolerance, and Results Passing Score. | All display units (`pts`, `%`, etc.), accept typing and dragging, clamp hostile negatives, and undo cleanly (**UX-11**, **UX-20**, **UX-21**). |
| **T8.7** | Question feedback & Results included-questions list scrolling | Add 20 question slides and open Results slide in Question tab at `1024×600`. | Single scroll area (no nested scroll), all checkboxes readable with `>= 4.5:1` contrast in both themes (**UX-02**, **UX-06**). |
| **T8.8** | Question edits undo atomically | Toggle correct choice, edit prompt, change attempts, toggle shuffle; press `Ctrl+Z` once per edit. | Every edit reverts cleanly in 1 undo step (**UX-21**). |

### Area 9: Media Library, Fonts Manager, Text Styles & Galleries
*Files:* `crates/ui-egui/src/media_library.rs:1-645`, `fonts.rs:1-154`, `text_styles_ui.rs:1-329`, `shape_gallery.rs:1-336`, `interaction_gallery.rs:1-407`, `web_insert.rs:1-119`

| Case | Scenario | Steps / Inputs | Expected Result |
|---|---|---|---|
| **T9.1** | Media Library — eliminate triple nested scroll (**SUS-08**) | Open Media Library with 30 media files and 10 usage places; resize window from `720×480` to `900×700`. | At most 1 scroll area per distinct split region (never a scroll area inside `DialogUi::body`'s scroll area); resizing taller expands the media table proportionally (**UX-02**, **UX-14**). |
| **T9.2** | Media Library Kind combo, long filenames & disabled tooltips (**SUS-05**, **SUS-33**) | Open `Kind` combo (`:222`); load 100-char filename; hover disabled `Delete`, `Delete unused`, `Import…` (in web mode), `Insert on slide` (in Master View). | `Kind` combo has explicit width & height; long filename truncates with tooltip; every disabled button explains why on hover (**UX-01**, **UX-08**, **UX-19**). |
| **T9.3** | Media Library empty state & Delete confirmation | Open Media Library on a course with 0 media; also test Delete confirmation dialog (`:401`). | Empty state explains how to import media (**UX-17**); confirmation dialog is centred, sentence-case, and closes on `Esc` (**UX-11**, **UX-13**). |
| **T9.4** | Fonts Manager single scroll, tooltip & confirmation (`fonts.rs:42-154`) | Open Fonts dialog with 15 embedded fonts; check `Remove` button (`:75`) and `"Remove font?"` confirmation (`:122`). | No nested scroll; `Remove` has descriptive tooltip; confirmation dialog closes on `Esc` and removes font in 1 undo step (**UX-02**, **UX-10**, **UX-13**, **UX-21**). |
| **T9.5** | Text Styles dialog resizable & single scroll (**SUS-09**, **SUS-13**) | Open Text Styles (`text_styles_ui.rs:194-225`) with 15 styles; resize dialog. | Resizable, single scroll area (no nested scroll inside `Dialog::show`), `Esc` closes (**UX-02**, **UX-13**, **UX-14**). |
| **T9.6** | Shape Gallery window — zero nested scroll & search focus (**SUS-10**) | Open `Insert ▸ Shapes…` (`shape_gallery.rs:309`) and toolbar `Shape` popup; type `"arrow"`, press `Esc`. | Zero nested scroll areas when shown as `Dialog`; search auto-focuses; every 32 px shape icon has tooltip; `Esc` closes cleanly (**UX-02**, **UX-10**, **UX-13**). |
| **T9.7** | Insert Interaction gallery resizable & card contrast (`interaction_gallery.rs:393`) | Open Interaction gallery at `(1024, 600)` and `(1440, 900)` in Dark & Light themes. | Resizable or cleanly scrolled with 1 scroll area, all card descriptions `>= 4.5:1` contrast, `Esc` closes (**UX-02**, **UX-06**, **UX-13**, **UX-14**). |
| **T9.8** | Insert Web Object dialog validation & `Esc` (`web_insert.rs:62`) | Open Insert Web Object; enter invalid URL `"ftp://bad"` and empty URL; hover disabled Insert; press `Esc`. | Actionable validation message explains valid `https://` URL or folder (**UX-18**); disabled button explains why (**UX-08**); `Esc` closes (**UX-13**). |

### Area 10: Authoring Dialogs (Course Settings, Theme, Publish, Export, Preferences, Version History, Find, Spelling, Slide Size, History)
*Files:* `course_settings.rs`, `resources_ui.rs`, `theme_dialog.rs`, `publish.rs`, `export_dialog.rs`, `preferences.rs`, `version_history.rs`, `find.rs`, `spell_check.rs`, `slide_size_dialog.rs`, `history.rs`

| Case | Scenario | Steps / Inputs | Expected Result |
|---|---|---|---|
| **T10.1** | Uniform sentence-case dialog titles & `Form` alignment (**SUS-38**) | Sweep all 37 `Dialog::new(...)` titles and form rows across every authoring dialog. | Every dialog title uses sentence case (`"Course settings"`, `"Find and replace"`, `"Version history"`, `"Slide size"`, etc., keeping proper nouns `"AI"`, `"PowerPoint"`); all form inputs align to `Form::label_width()` within `1 pt` (**UX-11**, **UX-12**). |
| **T10.2** | Centralized `Dialog` `Escape` & popup-first rule (**SUS-41**) | Open each dialog (`Course settings`, `Preferences`, `Publish`, `Theme`, `Version history`, `Find`, `Slide size`, `Export`, `History`); open a combo inside it (where present); press `Esc` once, then twice. | First `Esc` closes ONLY the open combo/popup; next `Esc` closes the dialog (**UX-13**). |
| **T10.3** | Course Settings resizable, combos & Resources/Glossary growth (**SUS-05**, **SUS-13**) | Add 10 Resources and 15 Glossary entries; open Course Settings at `(1024, 600)` and `(1440, 900)`. | Dialog is resizable, Navigation & Logo combos have explicit width/height, `Clear` disabled button has tooltip, sentence-case `"Course title"` / `"Course ID"` labels (**UX-01**, **UX-08**, **UX-11**, **UX-14**). |
| **T10.4** | Publish dialog validation feedback, disabled button & standard options (`publish.rs:244-512`) | Open Publish on course with a validation error, on web (`!file_ops_available`), and on valid desktop course; test all 7 targets. | Disabled `Publish` button explains why on hover (**UX-08**); validation errors explain how to fix (**UX-18**); dialog fits `(1024, 600)` scale `1.5` with fixed footer visible (**UX-04**, **UX-14**). |
| **T10.5** | Version History — single scroll, resizable growth & sentence case (**SUS-09**, **SUS-38**) | Open Version History with 25 saved versions (including 1 `Damaged`); resize window taller; also test empty & folder-project states. | Single scroll area grows with window height (no `320 px` inner cap); `"Open as copy"` sentence case; empty & git-folder states give clear guidance (**UX-02**, **UX-11**, **UX-14**, **UX-17**). |
| **T10.6** | Find and Replace — single scroll, resizable, remembered size & disabled tooltips (**SUS-09**, **SUS-13**, **SUS-15**, **SUS-33**) | Open Find & Replace, search `"a"` matching 50 hits, resize dialog, close and reopen; hover disabled `Replace` / `Replace all`. | Single scroll area; resizable; remembers size across reopen; `"Replace all"` sentence case with disabled tooltip when no matches (**UX-02**, **UX-08**, **UX-11**, **UX-14**). |
| **T10.7** | Preferences dialog `Form` alignment, explicit width & immediate theme apply (`preferences.rs:77-139`) | Open Preferences in Dark & Light at `(1024, 600)` scale `1.5`; switch theme radio to Light, edit Autosave & Versions `DragValue`s. | Aligned label column with comfortable width (`>= 400 px`); theme switches immediately with zero stale dark tokens; `Esc` closes (**UX-07**, **UX-12**, **UX-13**). |
| **T10.8** | Slide Size & Export dialogs units & disabled states (`slide_size_dialog.rs:91-110`, `export_dialog.rs:138-249`) | Open Slide Size (set `w = 0`), Export Video (`fps` combo), Export for Review. | Slide Size `W`/`H` show `" px"` unit and disabled `Apply` explains valid range (**UX-08**, **UX-11**); Export Video `FPS` combo has width/height; disabled `Export…` explains why (**UX-01**, **UX-08**). |
| **T10.9** | Check Spelling & Theme dialogs contrast & undo (`spell_check.rs`, `theme_dialog.rs`) | Open Spelling on course with misspelling (verify squiggle uses `p.danger` in both themes, **SUS-29**); open Theme dialog and change accent colour, verify undo. | Theme-aware squiggle colour (`>= 3.0:1`), sentence case, `Esc` closes, changes undo cleanly (**UX-06**, **UX-07**, **UX-13**, **UX-21**). |

### Area 11: AI Settings, Agents & AI Authoring Dialogs
*Files:* `ai_settings.rs`, `connect_agent.rs`, `outline.rs`, `document.rs`, `alt_text.rs`, `audio_ai.rs`, `record_narration.rs`, `translate_ai.rs`, `translation.rs`

| Case | Scenario | Steps / Inputs | Expected Result |
|---|---|---|---|
| **T11.1** | AI Settings tabs, provider combo, `Esc` stacking & disabled `Test connection` (**SUS-05**, **SUS-13**, **SUS-33**, **SUS-36**, **SUS-41**) | Open AI Settings (`AI` / `Agents` / `Voice` tabs); open `Provider` combo; press `Esc`; hover disabled `Test connection`. | Dialog resizable; `Provider` combo has width/height; `"Use"` has no trailing colon; first `Esc` closes combo only; disabled `Test connection` explains `"Enter an API URL and model"` (**UX-01**, **UX-08**, **UX-11**, **UX-13**, **UX-14**). |
| **T11.2** | Narration from Text (ElevenLabs) model & voice combos (**SUS-05**) | Open `speech` dialog with 30 voices loaded; open `Model` and `Voice` combos; hover disabled `Generate`. | Both combos show wide, stable, `>= 12`-item popups; disabled `Generate` explains why (**UX-01**, **UX-03**, **UX-08**). |
| **T11.3** | Translate with AI — single scroll, language combo & colon-free label (**SUS-05**, **SUS-10**, **SUS-33**, **SUS-36**) | Open `translate` dialog with 20 translation rows; open language combo; hover disabled `Translate` / `Apply`. | Single scroll area; language combo wide & tall (`>= 12` items); `"Translate to"` without colon; disabled buttons explain why; `Esc` closes combo before dialog (**UX-01**, **UX-02**, **UX-08**, **UX-11**, **UX-13**). |
| **T11.4** | Slides from Outline & Course from Document — zero nested scroll (**SUS-10**) | Open `outline` and `document` dialogs at `(1024, 600)` and `(1440, 900)` with 60-line outline text. | Single scroll area per pane; fixed footer with primary CTA always visible; `Esc` closes cleanly (**UX-02**, **UX-13**, **UX-14**). |
| **T11.5** | Suggest Alt Text disabled `Suggest` / `Apply` tooltips & `Esc` (`alt_text.rs:289-520`) | Open `alt-text` with no images, with unticked images, and during busy state. | Empty state says no images need alt text (**UX-17**); disabled `Suggest` and `Apply` explain why (**UX-08**); `Esc` closes popups first (**UX-13**). |
| **T11.6** | Record Narration device combo & error states (`record_narration.rs:149-270`) | Open `record-narration` with 5 long audio device names and with 0 devices. | Device combo has explicit width/height (**UX-01**); 0-device state gives clear actionable message (**UX-18**); `Record` white-on-danger passes `>= 4.5:1` (**UX-06**). |
| **T11.7** | Export Translation & Import Report — single scroll & disabled Export (**SUS-09**, **SUS-33**) | Open `export-translation` with empty language tag; open `translation-report` with 40 lines. | Disabled `Export…` explains `"Enter a valid BCP-47 language tag (for example, fr or es-MX)"`; Import Report has single scroll area and closes on `Esc` (**UX-02**, **UX-08**, **UX-13**). |
| **T11.8** | Agents tab (`connect_agent.rs`) copy feedback & contrast at `1024×600` | Open `agents` window in Light & Dark at `(1024, 600)` scale `1.5`. | Long executable paths wrap/truncate without pushing `Copy` off-screen; `Copy` shows confirmation feedback; 0 layout findings (**UX-04**, **UX-19**). |

### Area 12: Status Bar, Check Course (Validation + Accessibility), Welcome, Coach Marks, Shortcuts & Report
*Files:* `status.rs`, `welcome.rs`, `coach.rs`, `shortcuts.rs`, `report.rs`

| Case | Scenario | Steps / Inputs | Expected Result |
|---|---|---|---|
| **T12.1** | Status bar chip hit target & long title truncation (`status.rs:202-315`, **SUS-34**) | Load 120-char filename + transient error + status message + automation badge at `(1024, 600)` scale `1.5`. | Validation pill height `>= 24 pt`; long title truncates so validation chip and error button never clip off the right edge (**UX-04**, **UX-05**, **UX-19**). |
| **T12.2** | Check Course & Error History — eliminate triple/nested scroll (**SUS-07**) | Open `Check Course` with 15 validation issues + 15 accessibility findings; open `Error History` with 20 errors. | Single vertical scroll area in each dialog (zero nested `ScrollArea`s); long issue descriptions wrap cleanly inside dialog width; `Esc` closes (**UX-02**, **UX-04**, **UX-13**). |
| **T12.3** | Welcome screen geometry fix at `1024×600` & scale `1.5×` (**SUS-19**) | Open Welcome screen with 10 recent courses (long paths) at `(1024, 600)`, `(1280, 800)`, `(1920, 1080)` × Light/Dark × scale `1.0`/`1.5`. | Card height uses `ground_rect.max.y - card_y - 16.0` (not `ground_rect.height() - card_y`); zero clipped tiles or recent-file rows; `Back to course` aligned cleanly (**UX-04**, **UX-19**). |
| **T12.4** | Welcome screen `New` template popup clamping & contrast (`welcome.rs:493`) | Click `New` tile / template menu at `(1024, 600)` in Light & Dark. | Popup fits inside screen, all 4 template descriptions pass `>= 4.5:1` contrast, `Esc` closes popup (**UX-06**, **UX-13**, **UX-16**). |
| **T12.5** | Coach marks anchor to live panel widths, clamp & handle `Esc` (**SUS-18**) | Resize left navigator to `320 px` and right inspector to `400 px`; step through `coach-1`, `coach-2`, `coach-3` at `(1024, 600)` scale `1.5`. | Carets point accurately to actual panel/toolbar edges; popovers never clip off-screen or overlap their target; `Next`/`Done` height `>= 26 pt`; `Esc` skips/dismisses (**UX-05**, **UX-13**, **UX-16**). |
| **T12.6** | Keyboard Shortcuts modal backdrop, close tooltip & bounds (**SUS-14**, **SUS-28**, **SUS-32**) | Open Shortcuts (`?`) at `(1024, 600)` and `(1440, 900)` in Dark & Light. | Backdrop dims editor (`alpha >= 0x99` dark / `0x73` light); close `×` uses `widgets::icon_button` (`26×26`, tooltip `"Close (Esc)"`); dialog stays below menu bar and inside screen (**UX-05**, **UX-10**, **UX-14**, **UX-15**). |
| **T12.7** | Crash recovery & Report a Problem wording & `Esc` (**SUS-37**) | Open `crash` and `report` dialogs in Light & Dark. | Positive button copy (no `"Don't ask again"`); scrubbed details toggle disables preview cleanly with explanation; `Esc` closes (**UX-08**, **UX-11**, **UX-13**). |
| **T12.8** | PPTX Import Report empty/long report scroll (`status.rs:469-486`) | Open `pptx-report` with 80 lines of import notes at `(1024, 600)`. | Single scroll area (`DialogUi::body`), `Dismiss` button pinned in footer so it is always reachable without scrolling (**UX-02**, **UX-14**). |

### Area 13: In-Editor Preview Modal & Desktop/Web App Shell
*Files:* `crates/ui-egui/src/preview.rs:1-1048`, `apps/coursecraft/src/main.rs:2850-3530`, `apps/coursecraft/index.html`

| Case | Scenario | Steps / Inputs | Expected Result |
|---|---|---|---|
| **T13.1** | Preview backdrop dims editor strongly (**SUS-27**) | Open `Preview` in Dark and Light modes; inspect backdrop `RectShape` alpha AND render `snapshot_editor(..., preview: true)` to check pixel luminance over white slide behind modal. | Backdrop alpha `>= 0xB8` (Dark) / `>= 0x8C` (Light); background editor chrome is clearly obscured and cannot receive clicks (**UX-15**). |
| **T13.2** | Preview is resizable AND supports Full-screen / Maximize (**SUS-11**) | Open `Preview` at `(1440, 900)` and `(1920, 1080)`; drag resize handle; click Full-screen/Maximize toggle; press `F11` or click toggle again to restore. | Preview is no longer capped at `960×640`; resizing grows the player frame smoothly and remembers size; Full-screen expands preview to fill editor viewport (or native fullscreen) and restores cleanly (**UX-14**). |
| **T13.3** | Preview outline drawer slides from **player's** left edge, never screen edge (**SUS-16**) | Open `Preview`, click Outline button, capture mid-animation (`drawer_progress = 0.5`) and open (`1.0`) both in `TestHarness` and `snapshot_editor`. | Drawer `clip_rect` equals `drawer.intersect(full)`; zero drawer pixels appear to the left of the preview player's left edge (**UX-16**). |
| **T13.4** | Preview `Esc` hierarchy (speed popup → drawer → lightbox → text input → preview modal) | Inside Preview: (a) open drawer → `Esc`; (b) open lightbox → `Esc`; (c) focus text input → `Esc`; (d) press `Esc` on clean slide. | `Esc` closes drawer/lightbox/blur FIRST without closing Preview; pressing `Esc` again closes Preview cleanly and stops media (**UX-13**). |
| **T13.5** | Preview Live Variables (`F3`) + Effect Log (`F2`) split pane at narrow size | Open Preview at `(1024, 600)`, toggle both `F2` and `F3`, load 20 long-named variables & 100 long effect log lines. | Slide retains usable width, side panes scroll independently with zero overlap or auto-grow drift across 10 frames (**UX-04**, **UX-19**). |
| **T13.6** | Desktop minimum window size enforced (`main.rs:3441-3443`, **SUS-12**) | Inspect `ViewportBuilder` configuration (unit-testable helper `editor_viewport_builder()`). | `.min_inner_size` is explicitly `[1024.0, 600.0]`, matching the layout sweep floor (**UX-04**, **UX-14**). |
| **T13.7** | Desktop app shell modals — backdrop, width clamp, copy & `Esc` (**SUS-27**, **SUS-37**, **SUS-42**) | Drive `unsaved_changes_modal`, `changed_on_disk_modal`, `restore_autosave_modal`, `error_modal` with 150-char filenames/errors at `(1024, 600)`. | All 4 use shared backdrop dimming (`alpha >= 0xB8`/`0x8C`), `max_width <= 480 pt` with wrapped text, primary button styling, positive wording (`"Discard changes"` not `"Don't Save"`), and `Esc` cancels dismissible modals (**UX-11**, **UX-13**, **UX-15**, **UX-18**, **UX-19**). |
| **T13.8** | Web Editor (`wasm32`) theme preference & disabled desktop actions (**SUS-29**) | Check WASM init path (`main.rs:3472-3530`) and run editor with `file_ops_available = false`. | Web editor respects system/stored theme instead of forcing `ThemeMode::Dark`; every desktop-only action (native save-as-folder, import local font/media if unavailable) is visibly disabled with `"Available in the desktop app"` tooltip (**UX-07**, **UX-08**). |

### Area 14: Learner Player Chrome & Published Web Player
*Files:* `crates/render/src/{chrome,player_frame,drawer,focus,access}.rs`, `apps/coursecraft-player/src/{app,a11y}.rs`, `apps/coursecraft-player/index.html`

| Case | Scenario | Steps / Inputs | Expected Result |
|---|---|---|---|
| **T14.1** | Responsive player strip across all 6 browser sizes | Run `PlayerFrame` sweep at `320×568`, `390×844`, `844×390`, `768×1024`, `1280×720`, `1920×1080` with logo, captions, speed, fullscreen, menu, nav, seekbar all enabled. | Zero clipped or overlapping strip controls at any size; progressive drop order (`title → counter → speed → fullscreen → volume slider → time → replay`) keeps Play, Seek, and Next always usable with `>= 36×36 pt` hit targets (**UX-04**, **UX-05**). |
| **T14.2** | Outline drawer clipped to player edge & smooth animation (**SUS-16**) | Run `PlayerFrame` inside an inset rect (`min = (200, 80)`, `size = (800, 500)`) with `drawer_progress` at `0.0, 0.25, 0.5, 0.75, 1.0`. | Every painted drawer shape and scrim shape is clipped to `full`; `x < 200.0` never receives a single drawer pixel (**UX-16**). |
| **T14.3** | Drawer long titles, 4-tab header & Resources/Glossary truncation (**SUS-24**) | Open drawer at `320 px` width with all 4 tabs (`Menu`, `Resources`, `Glossary`, `Transcript`) and 100-char course, scene, slide, resource, and glossary titles. | All 4 tab labels fit without colliding; slide/resource/glossary titles truncate cleanly before icons (`ArrowSquareOut`) with full-text tooltips on hover (**UX-04**, **UX-19**). |
| **T14.4** | Speed popup clamping & `Esc` close (**SUS-17**, **SUS-23**) | Open speed popup at `390×844` and `1280×720`; hover over popup and press `Esc`; also press `Esc` with `pointer = None` (keyboard-only). | Popup stays 100% inside `full`; `Esc` ALWAYS closes speed popup regardless of pointer position (**UX-13**, **UX-16**). |
| **T14.5** | Full-screen button tooltip reflects real state (**SUS-40**) | Inspect fullscreen button tooltip when `frame.fullscreen == false` vs `frame.fullscreen == true`. | Tooltip reads `"Full screen"` when windowed and `"Exit full screen"` when full-screen (**UX-09**, **UX-10**). |
| **T14.6** | Player chrome contrast under bright/pastel & dark course themes (**SUS-30**) | Test `PlayerFrame` with (a) default `#1B1B1B` bar + `#0A6CFF` accent, (b) light `#FFFFFF` bar + `#1D1D1F` text, and (c) bright yellow `#FFD60A` course accent. | Selected drawer tab, active speed row, Next button, and disabled seekbar time label compute contrasting foreground (`WHITE` vs `#1D1D1F`) so every control meets `>= 4.5:1` text / `>= 3.0:1` icon contrast (**UX-06**). |
| **T14.7** | Notice banner, Review banner, Audio captions & Lightbox close never collide | Activate Notice banner + Review banner + Audio captions + open Lightbox simultaneously at `390×844` and `1280×720`. | Banners stack cleanly without overlapping each other; captions sit above the visible strip; lightbox `×` button (`>= 32×32 pt`) is clickable and unobstructed (**UX-04**, **UX-05**, **UX-16**). |
| **T14.8** | Web player `State::Failed` error screen & `State::Loading` (`apps/coursecraft-player/src/app.rs:1000-1023`) | Render `State::Failed` with a 300-char error message at `390×844` and `1280×720`. | Error screen uses player ground + raised card (not raw unstyled panel), wraps within `<= 480 pt`, gives human-readable fix guidance, and passes `>= 4.5:1` contrast (**UX-06**, **UX-18**). |
| **T14.9** | Preview-equals-published pixel parity (**UX-22**) | Render `showcase` slide `"s-media"` at `960×540` via `snapshot_player` and inside editor `Preview` (cropping player rect). | Slide rect, strip layout, button order, and rendered content match within `<= 1%` pixel diff (**UX-22**). |

---

## 6. Ordered Fix Steps (18 Single-Session Steps)

Each step is scoped to **one implementer session**, starts by writing failing tests (red), then implements the fix (green), and finishes with `cargo xtask ci` passing.

### Phase 1: Automated UI/UX Verification Infrastructure (Sessions 1–3)

- **Step 1 — Harness `accesskit` widget walker, UI scale & desktop-mode support**
  - *Crates / Files:* `crates/ui-egui/Cargo.toml` (`[dev-dependencies]` add `accesskit` feature on `egui`), `crates/ui-egui/src/test_harness.rs`.
  - *Tests first:* Unit tests on synthetic egui scenes proving the harness catches (a) a `< 24 pt` button (`UX-05`), (b) an icon button missing tooltip or accessible name (`UX-10`), (c) a disabled button missing disabled tooltip (`UX-08`), and (d) a clipped control at scale `1.5`.
  - *Implementation:* Add `TestHarness::new_with_config(course, size, mode, zoom_factor, file_ops)`, `WidgetNode` extraction from `accesskit_update`, `check_hit_targets`, `check_tooltips_and_disabled`.

- **Step 2 — Painted-shape contrast auditor, nested-`ScrollArea` detector & modal backdrop checker**
  - *Crates / Files:* `crates/ui-egui/src/test_harness.rs`, `crates/ui-egui/src/dialog.rs` (lightweight test-only scroll-rect tracker or clip-hierarchy inspector).
  - *Tests first:* Synthetic tests proving `check_contrast` catches dark-on-blue (`~3.2:1`, **SUS-26**), `check_nested_scrolls` catches a `ScrollArea` inside `DialogUi::body` (**SUS-07**), `check_popup_stability` catches width-jumping scrollbars (**UX-03**), and `check_modals_and_backdrops` catches light/transparent backdrops (**SUS-27**, **SUS-28**).
  - *Implementation:* Implement `check_contrast`, `check_nested_scrolls`, `audit_popup`, and `check_modals_and_backdrops` in `test_harness.rs`.

- **Step 3 — Shared UI primitives (`Dialog` Esc/resize/unscrolled body, `ComboBox` helper, `Modal` backdrop helper)**
  - *Crates / Files:* `crates/ui-egui/src/dialog.rs`, `crates/ui-egui/src/widgets.rs`, `crates/ui-egui/src/lib.rs`.
  - *Tests first:* Unit tests asserting (a) `Dialog` handles `Esc` ONLY when `!Popup::is_any_open(ctx)`, (b) `Dialog::body_unscrolled` emits zero outer `ScrollArea`, (c) new `widgets::styled_combo` enforces min width + `>= 12`-item popup height, (d) `widgets::modal_backdrop_color(mode)` meets `UX-15` alpha thresholds, and (e) `widgets::mixed_toggle` supports `Option<bool>` (Off / On / Mixed) with tooltip + `Toggled::Mixed` accesskit state.
  - *Implementation:* Add these 5 reusable primitives so every subsequent surface step deletes ad-hoc code rather than duplicating fixes.

### Phase 2: Highest-Impact Owner-Reported Fixes (Sessions 4–6)

- **Step 4 — Text formatting style-state accuracy & mixed-state toggles (`Area 2`)**
  - *Crates / Files:* `crates/ui-egui/src/properties.rs`.
  - *Tests first:* `T2.1`, `T2.2`, `T2.3`, `T2.4` (reproducing **SUS-21**, **SUS-22**, **SUS-31** — Italic falsely active at run boundary, missing mixed state, missing `B`/`I`/`U` tooltips).
  - *Implementation:* Fix `style_at_cursor` & `effects_at_cursor` to use half-open `[offset, offset + len)` range matching `font_at_cursor`; add `style_for_range(runs, style, range) -> TriState` returning `Mixed` when runs disagree; migrate single-select, multi-select, and Timer `B/I/U` to `widgets::mixed_toggle` with tooltips & accessible names; switch multi-select (`:2346`) and Timer (`:5505`) font pickers to `font_picker::show_font_picker` (**SUS-01**); switch multi-select Size (`:2468`) to `DragValue`.

- **Step 5 — Font picker contrast & Colour picker / Eyedropper polish (`Area 3`)**
  - *Crates / Files:* `crates/ui-egui/src/font_picker.rs`, `crates/ui-egui/src/properties.rs`, `crates/ui-egui/src/eyedropper.rs`.
  - *Tests first:* `T3.1`–`T3.8` (reproducing **SUS-26** Light-mode contrast failure on highlighted font row, long-name tooltip, clamping at bottom-right).
  - *Implementation:* Use `p.accent_subtle` + `p.text` (or `p.accent` + `Color32::WHITE`) for current/highlighted font rows and contrasting badge colour; add full-name hover tooltip when a font name is truncated; clamp colour picker popup inside `screen_rect`.

- **Step 6 — In-Editor Preview backdrop, resize, full-screen & PlayerFrame drawer clip (`Area 13` & `Area 14` criticals)**
  - *Crates / Files:* `crates/ui-egui/src/preview.rs`, `crates/render/src/player_frame.rs`.
  - *Tests first:* `T13.1`, `T13.2`, `T13.3`, `T13.4`, `T14.2`, `T14.4`, `T14.5` (reproducing **SUS-11**, **SUS-16**, **SUS-17**, **SUS-23**, **SUS-27**, **SUS-40**).
  - *Implementation:*
    - `player_frame.rs:2583, 2597`: clip drawer & scrim to `rect.intersect(full)` so drawer slides strictly from the player's left edge; clamp speed popup to `full`; fix speed popup `Esc` handling so `Esc` closes regardless of pointer position; fix fullscreen tooltip to `"Exit full screen"` when active.
    - `preview.rs:711-805`: apply dark opaque-enough backdrop (`widgets::modal_backdrop_color`); convert preview to a resizable, screen-clamped modal/window with remembered size + Maximize/Full-screen toggle button (and wire `frame.can_fullscreen = true`); close speed popup/drawer/lightbox before closing preview on `Esc`.

### Phase 3: Editor Chrome, Toolbar, Navigator, Canvas & Bottom Panels (Sessions 7–11)

- **Step 7 — Toolbar overflow fix, clean-room label & Menu/Context-menu polish (`Area 1`)**
  - *Crates / Files:* `crates/ui-egui/src/toolbar.rs`, `crates/ui-egui/src/widgets.rs`, `crates/ui-egui/src/menu.rs`.
  - *Tests first:* `T1.1`–`T1.9` (reproducing **SUS-20**, **SUS-25**, **SUS-33**, **SUS-34**, **SUS-35**).
  - *Implementation:* Raise `show_label` width threshold dynamically when `file_ops_available == true` (or compute exact left/centre/right bounds so groups never collide); rename `"Story"` → `"Map"` (**clean-room**); disable Course map toggle properly in Master View with tooltip; add disabled tooltip for Format Painter; widen `toolbar_caret_button` from `16` to `20 pt`.

- **Step 8 — Inspector combos, numeric units, States & Animate/Slide/Course tabs (`Area 2` remainder)**
  - *Crates / Files:* `crates/ui-egui/src/properties.rs`, `crates/ui-egui/src/states.rs`.
  - *Tests first:* `T2.5`–`T2.10` (reproducing **SUS-04**, **SUS-06**, **SUS-33**, **SUS-39**).
  - *Implementation:* Migrate all inspector `ComboBox`es to `widgets::styled_combo` (explicit width + tall popup); add unit suffixes (`pt`, `px`, `ms`, `×`) to all unitless `DragValue`s; add disabled hover tooltips to `Clear`, `Delete row`, `Delete column`, and States `+ Add`/pills.

- **Step 9 — Slide Navigator, Master View, Course Map & Canvas/Rulers (`Area 4` & `Area 5`)**
  - *Crates / Files:* `crates/ui-egui/src/slides.rs`, `crates/ui-egui/src/course_map.rs`, `crates/ui-egui/src/canvas.rs`, `crates/ui-egui/src/rulers.rs`, `crates/ui-egui/src/crop.rs`.
  - *Tests first:* `T4.1`–`T4.8`, `T5.1`–`T5.8`.
  - *Implementation:* Ensure long slide/scene/layer titles truncate with hover tooltips; enforce `>= 24×24 pt` on thumbnail `⋯` and zoom buttons; guard Course map keys against open dialogs/preview; add `Esc` support to Delete scene/master dialogs; check ruler contrast in Light mode.

- **Step 10 — Timeline, Slide Layers pane & Layer Settings dialog (`Area 6`)**
  - *Crates / Files:* `crates/ui-egui/src/timeline.rs`, `crates/ui-egui/src/layers.rs`.
  - *Tests first:* `T6.1`–`T6.8` (reproducing **SUS-02**, **SUS-33**, **SUS-36**, **SUS-41**).
  - *Implementation:* Remove trailing colons from `layers.rs:1111-1161`; clarify layer checkboxes; wire `form.row` width + tall popup onto `layer_allow_seek` and `layer_revisit`; add disabled tooltips for `OK`, `Bring forward`, `Send backward`, and Base Layer actions; use centralized `Dialog` `Esc` handling.

- **Step 11 — Triggers, Trigger Editor, Variables & Question Editors (`Area 7` & `Area 8`)**
  - *Crates / Files:* `crates/ui-egui/src/triggers.rs`, `crates/ui-egui/src/variables.rs`, `crates/ui-egui/src/question.rs`.
  - *Tests first:* `T7.1`–`T7.8`, `T8.1`–`T8.8` (reproducing **SUS-03**, **SUS-10**, **SUS-33**, **SUS-36**, **SUS-39**).
  - *Implementation:* Give `tall_combo` explicit min/available width; remove nested `ScrollArea::both` inside Trigger Editor (`:2747`); remove trailing colons (`:3474-3488`); add `" ms"` unit to `event_ms`; add width/height to `var_new_type_combo` and disabled tooltip to `+ Add` & `Duplicate`; verify all 12 `QuestionKind`s + Results pass at `1024×600` scale `1.5`.

### Phase 4: Dialogs, Welcome, Onboarding, App Shell & Web Player (Sessions 12–16)

- **Step 12 — Media Library, Fonts Manager, Text Styles & Shape/Interaction/Web Galleries (`Area 9`)**
  - *Crates / Files:* `crates/ui-egui/src/media_library.rs`, `fonts.rs`, `text_styles_ui.rs`, `shape_gallery.rs`, `interaction_gallery.rs`, `web_insert.rs`.
  - *Tests first:* `T9.1`–`T9.8` (reproducing **SUS-05**, **SUS-08**, **SUS-09**, **SUS-10**, **SUS-13**, **SUS-33**).
  - *Implementation:* Switch `Media Library`, `Text styles`, and `Shape gallery` window to `Dialog::body_unscrolled` (eliminating nested scroll areas so resizing grows the list); size `media_library_kind` combo; add missing disabled/icon tooltips; make `Text styles` & `Interaction gallery` resizable; unify sentence-case titles & `Esc` close.

- **Step 13 — Core Authoring Dialogs: Course Settings, Theme, Publish, Export, Preferences, Version History, Find, Spelling, Slide Size (`Area 10`)**
  - *Crates / Files:* `course_settings.rs`, `resources_ui.rs`, `theme_dialog.rs`, `publish.rs`, `export_dialog.rs`, `preferences.rs`, `version_history.rs`, `find.rs`, `spell_check.rs`, `slide_size_dialog.rs`, `history.rs`.
  - *Tests first:* `T10.1`–`T10.9` (reproducing **SUS-05**, **SUS-09**, **SUS-13**, **SUS-15**, **SUS-29**, **SUS-33**, **SUS-38**, **SUS-39**, **SUS-41**).
  - *Implementation:* Remove nested `ScrollArea`s in `Version History` and `Find and Replace`; make `Course Settings`, `Find and Replace`, and `Publish` resizable with persistent window IDs; add `Form` layout to `Preferences`; add `" px"` units to `Slide Size`; switch `SQUIGGLE_COLOR` to `palette(ctx).danger`; add disabled tooltips; standardize all dialog titles & button labels to sentence case; centralize `Esc` close.

- **Step 14 — AI Settings, Agents & AI Authoring Dialogs (`Area 11`)**
  - *Crates / Files:* `ai_settings.rs`, `connect_agent.rs`, `outline.rs`, `document.rs`, `alt_text.rs`, `audio_ai.rs`, `record_narration.rs`, `translate_ai.rs`, `translation.rs`.
  - *Tests first:* `T11.1`–`T11.8` (reproducing **SUS-05**, **SUS-09**, **SUS-10**, **SUS-13**, **SUS-33**, **SUS-36**, **SUS-41**).
  - *Implementation:* Remove nested `ScrollArea`s in `outline`, `document`, `translate_ai`, `translation`; migrate `ai_provider`, `elevenlabs_model`, `elevenlabs_voice`, `record_narration_device`, `translate_common_language` to `styled_combo`; remove trailing colons; add disabled tooltips to `Test connection`, `Suggest`, `Apply`, `Generate`, `Translate`, `Export…`; guard `Esc` against open popups; make `AI Settings` resizable.

- **Step 15 — Status Bar, Check Course, Welcome, Coach Marks, Shortcuts & Report (`Area 12`)**
  - *Crates / Files:* `crates/ui-egui/src/status.rs`, `welcome.rs`, `coach.rs`, `shortcuts.rs`, `report.rs`.
  - *Tests first:* `T12.1`–`T12.8` (reproducing **SUS-07**, **SUS-14**, **SUS-18**, **SUS-19**, **SUS-28**, **SUS-32**, **SUS-34**, **SUS-37**).
  - *Implementation:* Flatten `Check Course` & `Error History` to 1 scroll area; raise status bar validation chip to `24 pt` and truncate long title; fix `welcome.rs:355` height math; anchor coach marks to live panel rects with screen clamping, `26 pt` buttons, and `Esc`; add dimmed backdrop + `widgets::icon_button` close + `dialog_area` clamping to `shortcuts.rs`; replace negative `"Don't ask again"` in `report.rs`.

- **Step 16 — Desktop App Shell, Web Editor & Learner Player Chrome (`Area 13` remainder & `Area 14`)**
  - *Crates / Files:* `apps/coursecraft/src/main.rs`, `crates/render/src/player_frame.rs`, `apps/coursecraft-player/src/app.rs`.
  - *Tests first:* `T13.5`–`T13.8`, `T14.1`, `T14.3`, `T14.6`–`T14.9` (reproducing **SUS-12**, **SUS-24**, **SUS-27**, **SUS-29**, **SUS-30**, **SUS-37**, **SUS-42**).
  - *Implementation:*
    - `apps/coursecraft/src/main.rs`: add `.with_min_inner_size([1024.0, 600.0])`; give all 4 shell `Modal`s dimmed backdrop, `max_width(480.0)`, `modal.should_close()` `Esc` handling, primary button styling, and `"Discard changes"`; resolve system theme on WASM init instead of hardcoding `ThemeMode::Dark`.
    - `crates/render/src/player_frame.rs`: truncate drawer slide/resource/glossary/tab text with hover tooltips; compute adaptive contrasting foreground (`WHITE` vs `#1D1D1F`) on course `accent` buttons/tabs; raise disabled strip time contrast to `>= 3.0:1`.
    - `apps/coursecraft-player/src/app.rs`: style `State::Failed` on `PLAYER_GROUND` with raised card, wrapped message, and clear fix guidance.

### Phase 5: Full Matrix Sweep & Before/After Report (Sessions 17–18)

- **Step 17 — Full UI/UX Sweep Green Across All Surfaces, Sizes, Themes, Scales & Fixtures**
  - *Crates / Files:* `crates/ui-egui/src/uiux_sweep_tests.rs`, `crates/render/src/uiux_player_tests.rs`, `crates/ui-egui/src/layout_tests.rs`.
  - *Verification:*
    1. Run `uiux_sweep_all_surfaces` across all 48 surfaces × 3 sizes (`1024×600`, `1280×800`, `1920×1080`) × 2 themes (`Dark`, `Light`) × 2 scales (`1.0`, `1.5`) + `2.0` stress pass + `long_names_course` + `empty_course` + desktop (`file_ops_available = true`) and web (`false`) modes.
    2. Run `uiux_player_sweep` across all 6 browser viewport sizes (`320×568` to `1920×1080`) × 3 course themes × all player states + preview-vs-published pixel parity test.
    3. Fix any residual findings discovered by the full sweep until **0 violations** remain and `cargo xtask ci` is 100% green.

- **Step 18 — Before/After UI/UX Audit Report (`docs/testing/m28-uiux-report.md`)**
  - *Files:* `docs/testing/m28-uiux-report.md` (and local review PNGs in `target/uiux-snapshots/`).
  - *Contents:*
    - Baseline vs final violation counts grouped by rule (`UX-01`–`UX-22`) and by surface.
    - Complete disposition table for all 42 watchlist suspects (`SUS-01`–`SUS-42`), noting reproduced/fixed vs non-reproducing.
    - Sweep coverage summary (matrix dimensions, total frames audited, total widgets probed, execution time).
    - Key before/after visual comparison notes (font picker Light/Dark, desktop toolbar at 1100 px, Preview modal backdrop + full-screen, Player drawer edge clip, Layer Settings, Media Library).
    - Clean-room sign-off confirming zero competitor terms or copied UI patterns remain.

---

## Summary

This plan moves CourseCraft from ad-hoc manual UI bug discovery to **continuous, automated UI/UX enforcement** across all 48 editor, app-shell, web-editor, and learner-player surfaces. By combining egui 0.37's `accesskit` tree with painted-shape contrast, clipping, overlap, nested-scroll, scrollbar-stability, tooltip, and backdrop checks across a `3 window sizes × 2 themes × 2 UI scales` sweep matrix (plus a 6-viewport learner player sweep), the 18 ordered sessions will systematically eliminate all 42 confirmed source-code suspects — including narrow/short combos, 12 dialogs with nested scroll areas, the run-boundary Italic/style toggle bug, Light-mode font-picker contrast failure, desktop toolbar overflow, weak modal backdrops, non-resizable Preview, and the player outline drawer clipping bug — and lock them out permanently via CI.
