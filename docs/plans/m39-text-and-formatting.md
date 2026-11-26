# M39 plan: text and formatting gaps

Base: `main` at b6be80e (Release 0.6.0). Roadmap entry: `docs/roadmap.md` § M39.
Status source: `docs/ui/feature-parity.md` rows marked M39.
Also in this milestone: GitHub issues #11 and #12 (Package L).

Checked against 0.6.0 (M46 animation system): the M39 ribbon items, `TextRun`, `text.rs`,
`text_format.rs`, `clipboard.rs` and `layers.rs` are unchanged, so the packages below stand.
New things to respect: `Animation.sequence` (M46 phase 7) splits text into paragraph or
letter parts for entrances and exits, and the renderer mesh pass (phase 2) scales, turns and
clips text. Highlight and shadow must draw correctly inside both.

## What already exists (no model work needed)

| Item | Already there | Missing |
|---|---|---|
| Numbering styles | `Paragraph.number_scheme` (7 schemes), `start_at`, render support | a ▾ menu on Home › Paragraph › Numbering |
| Text direction | `TextStyle.direction` (`horizontal`, `vertical`, `vertical270`), `text.set_paragraph {direction}`, render, Text panel row | the ribbon split button menu. Stacked text is not in the model: out of scope (stays a known gap) |
| Slide number field | built-in variables `Course.SlideNumber`, `Course.TotalSlides`, `%var%` substitution everywhere | the Insert button that inserts the reference |
| Variable reference | `%Name%` substitution, `variable_reference_spans` | a picker that inserts `%Name%` |
| Focus order | `Object.focus_order`, `Group.focus_order`, `object.set`/`group.set` take `focus_order`, `render/src/access.rs` computes the order | the dialog |
| Object shadow | `Object.shadow` | (not reused: text shadow is per run, see below) |
| Theme font | `Theme.font`, `course.set_theme {font}` | a heading font, the dialog, the font list entries |

## Packages (one agent each, in parallel, separate worktrees: T, I, P, F, L)

Every package: no `unwrap`/`expect`/`panic!`; every user-facing string through `tl!`; each user action is one undo step; tests for every item; `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, and the tests of the crates touched must pass. Do **not** edit `docs/ui/feature-parity.md`, `docs/ui/ribbon.md`, `docs/roadmap.md` or `CHANGELOG.md`: the reviewer updates them once, from the package reports. Do edit `docs/spec/course-model.md` (and `docs/spec/triggers.md` if render or runtime behaviour changes) for model or command changes.

### Package T: Font and Paragraph groups (model, engine, render, ribbon)

Files: `crates/course` (model.rs `TextRun`, text.rs, named_styles.rs, strings.rs, diff), `crates/engine/src/commands/text.rs` + `mod.rs` registry, `crates/render` (compose.rs, text_layout.rs, paint.rs), `crates/format/src/xliff.rs`, `crates/pptx` (import mapping), `crates/ui-egui/src/text_format.rs`, `ribbon/spec.rs` (Font and Paragraph groups only), `ribbon/mod.rs`, `properties.rs` (`show_text_panel`).

1. **Model:** `TextRun.highlight: Option<Color>` (background behind the glyphs) and `TextRun.shadow: Option<bool>`. Both `skip_serializing_if = "Option::is_none"`. Update `is_plain`, `normalize_runs`, `TextRunPatch` (`highlight: Option<Option<Color>>` where `Some(None)` clears), the range-styling helpers, Find and Replace run preservation, course diff, XLIFF run round trip, validation (highlight colour valid). Old files load unchanged.
2. **Engine:** `text.style_range` accepts `highlight: Color|null` and `shadow: bool`. New `text.clear_formatting {slide, object, start, end, …}`: removes every run override in the range except `link`; one undo step; `start == end == 0` means the whole text (same rule as `text.style_range`). New `text.change_case {slide, object, start, end, case: "sentence"|"lower"|"upper"|"title"|"toggle"}`: changes the characters run by run, keeps run formatting and paragraph breaks; a character whose case mapping changes the length (for example `ß` → `SS`) is mapped per run so the runs stay aligned; refits point text and Grow boxes like `text.style_range`. Register both in the command registry with params docs; add them to the all-commands undo test.
3. **Render:** highlight draws a filled rectangle behind the run's glyphs (egui `TextFormat.background` or an explicit rect, whichever survives letter spacing and wrapping); shadow draws the run's glyphs again behind, offset (≈ 1.5% of the font size, at least 1 px) in black at 45% alpha. Both go in the text layout cache key. Works in the editor canvas, preview, the web player (wasm) and offscreen renders, since they share `coursecraft-render`.
4. **PPTX import:** map `a:highlight` to `highlight` and an outer shadow on run properties (`a:effectLst/a:outerShdw` in `a:rPr`) to `shadow: true`.
5. **Ribbon, Font group:** wire `font.clear` (Clear Formatting), `font.shadow` (toggle, shows on/mixed/off like Bold), `font.case` (▾ menu: Sentence case, lowercase, UPPERCASE, Capitalize Each Word, tOGGLE cASE), `font.highlight` (colour menu with the theme palette, a few standard highlight colours and No Color). They act on the selected text while editing inline, else on the whole text of every selected text object (the same targets as Bold, through `text_format::targets`).
6. **Ribbon, Paragraph group:** `paragraph.numbering` becomes a split button: the main part toggles numbering as today; the ▾ lists the 7 schemes with samples and "Start at…" (a small number field). `paragraph.direction` gets its menu: Horizontal, Rotate all text 90°, Rotate all text 270°. Stacked is not offered.
7. **Format Painter** (`format_painter.rs`) copies highlight and shadow with the other run formatting; the named text styles keep ignoring them unless `named_styles.rs` already stores run-level fields of that kind.
8. **Animation:** text with highlight or shadow plays correctly with `Animation.sequence` by paragraph and by letter and through the mesh pass (scale, rotation, clip): add a render test for each.
9. **Text panel:** add Highlight and Shadow rows to the Font section (switch the disabled highlight row on). Caps, underline styles, tab stops and columns stay disabled.

### Package I: Insert tab text items

Files: `ribbon/spec.rs` (Insert › Text group items only), `ribbon/mod.rs` (dispatch), a new `crates/ui-egui/src/symbol_picker.rs`, a new `crates/ui-egui/src/reference_picker.rs` (or one `insert_text.rs`), the inline text editor (to insert at the caret), `dialog.rs` (`any_open`).

1. **Shared helper:** "insert this text" — when a text object is being edited inline, insert at the caret (replacing the selection) as one edit; otherwise create a new text box at the slide centre holding the text, selected, with the theme body font. One undo step either way.
2. **Slide Number** (`insert.slide_number`): split button. Main part inserts `%Course.SlideNumber%`. ▾: "Slide number", "Slide number of total" (`%Course.SlideNumber% / %Course.TotalSlides%`). The editor canvas shows the reference text; preview and the player show the number (existing substitution).
3. **Symbol** (`insert.symbol`): a window with a grid of characters grouped in tabs or a combo (Common: © ® ™ ° ± × ÷ € £ ¥ § ¶ • … — – « » ¿ ¡; Arrows; Maths; Greek; Shapes and check marks ✓ ✗ ★ ☆ ● ○ ■ □ ▲ ▼), a search field by name, a "Recently used" row (session only), Insert and Close. Only characters the bundled fonts draw (check the font has the glyph, or limit the set to what Inter and the bundled fonts cover).
4. **Reference** (`insert.reference`): a window listing the course variables (name, kind) and the built-in variables (from the spec list), with a filter; Insert puts `%Name%`. Invalid names never appear.
5. Escape closes the windows; they count in `dialog::any_open` so global keys stay blocked.

### Package P: Paste options and the Focus Order dialog

Files: `crates/ui-egui/src/clipboard.rs`, `ribbon/spec.rs` (`PASTE_MENU` and `slide.focus_order` only), `ribbon/mod.rs`, `menu.rs` (Edit › Paste Special if a menu exists), a new `crates/ui-egui/src/focus_order.rs`, `dialog.rs`.

1. **Clipboard payload:** add an optional source theme (font, heading font if present, accent, text and background colours) to the object payload. Old payloads without it still paste.
2. **Use Destination Theme** (the default for Ctrl+V and the main Paste button): text fonts equal to the source theme font become the destination theme font; colours equal to the source theme's accent, text or background colour become the destination's. When the source theme equals the destination theme, nothing changes.
3. **Keep Source Formatting:** paste exactly as copied (today's behaviour).
4. **Keep Text Only:** paste the text of the copied objects (in order, one paragraph each) as one new plain text box in the theme body font; from plain OS clipboard text, the same as today's plain-text paste.
5. **Paste Special…:** a window listing the formats on the clipboard (CourseCraft objects, CourseCraft slides, plain text) with the three options above for objects; OK pastes. Disabled entries when the clipboard has nothing.
6. **Focus Order** (`slide.focus_order`): a window for the current slide listing every object and group that can take keyboard focus, in the current order (from the same logic as `render/src/access.rs`, factored into a public function there if needed: keep the layering rules). Move Up, Move Down, drag to reorder, "Reset to default order" (clears every `focus_order` on the slide). OK writes `focus_order` 1..n for every listed item through `object.set`/`group.set` in one `execute_batch` (one undo step). Layers: list the base layer and each layer in its own section if the access order treats them separately.

### Package F: Theme fonts and the Size and Position panel

Files: `crates/course/src/model.rs` (`Theme` only), validate.rs, `crates/engine/src/commands/course.rs` (`course.set_theme`), `crates/engine/src/commands/slide.rs` (layout title placeholders), `crates/publish/src/lib.rs` (font embedding), `crates/ui-egui/src/theme_dialog.rs`, the font picker, `crates/ui-egui/src/size_panel.rs`.

1. **Model:** `Theme.heading_font: Option<String>`, omitted when `None` (headings use `font`). Validation checks it like `font`. Publish embeds it. `Editor::font_family_in_use` counts it.
2. **Engine:** `course.set_theme {heading_font: string|null}`. Layout title placeholders and new title text use the heading font; body placeholders and new text boxes keep `font`.
3. **Create New Theme Fonts** dialog (from Design › Fonts ▾ and the Theme dialog): Heading font and Body font pickers, a live sample ("Heading" / "Body text sample"), a name is not needed (one pair per course). Save sets both through one `course.set_theme`. Design › Fonts becomes a split: the ▾ lists a few built-in pairs from the bundled fonts plus "Create New Theme Fonts…".
4. **Font list:** the ribbon Font combo and the Text panel font picker show a "Theme Fonts" section at the top: "<Heading font> (Headings)" and "<Body font> (Body)". Picking one sets that concrete font.
5. **Size and Position panel:** add Scale (width % and height %, linked by the aspect lock) and, for pictures, "Reset to original size" (natural pixel size from the media, clamped to the slide). If the natural size is not available without a decode, leave the row disabled with its reason and say so in the report.

### Package L: GitHub issues #11 and #12 (triggers and layers in the editor)

Files: `crates/ui-egui/src/triggers.rs` (list drawing only), `crates/ui-egui/src/layers.rs`, the canvas code that calls `extra_layer_ids`, `docs/ui/shell-panels-dialogs.md`. Editor only: no model or runtime change.

1. **#11, selected object highlights its triggers.** In the Triggers panel, rows whose event object (`event_object_id(&trigger.when)`) is the selected object or interactive group (`default_trigger_target`) get a clear highlight: an accent-tinted background and a 3 px accent bar on the left, distinct from the hover and active-row colours. Rows whose actions or conditions name the selected object (change state of it, show it, move it, a condition on its state) get a lighter tint with no bar. Both work in the flat list (which already sorts the selected object's rows first) and in Group by object (the selected object's group header is highlighted too). When the selection changes and the Triggers panel is already visible (its dock tab is the active tab, or its floating window is open), scroll the first highlighted row into view once. Never open, reveal or switch to the Triggers panel for this: if the Timeline or another tab is in front, nothing moves (owner decision). Nothing is highlighted when no object is selected. Reuse the existing reference walkers (`clipboard.rs` `event_object_ref` / `action_refs_only_copied` style helpers, or `course` reference helpers) instead of a new walker if one fits.
2. **#12, selecting a layer shows only that layer.** Today, when a layer is focused, every other layer whose eye is on draws too, faded (`LayersState::extra_layer_ids`), so a lower layer looks merged with the layers above it. New rule (owner decision): selecting a layer shows that layer and the base slide (dimmed as now by Dim) only; every other layer is fully hidden. The eye is manual: an eye is off by default, and the author turns a layer's eye on to see it beside the focused layer (drawn with the Dim fade, as extra layers are now). Store the opt-in set (`shown_layers`) instead of `hidden_layers`, cleared when the slide changes, as now. The focused layer's eye is always on. Base view (no layer focused): unchanged, layers never show there. Update the existing tests that expect other layers to show by default.
3. Tests for both: a trigger row on the selected object is highlighted and one on another object is not; with the Timeline tab in front, a selection change does not switch to or scroll the Triggers panel; a layer focus draws only that layer; turning another layer's eye on draws it; switching slides resets the eyes.

## Merge order and review

1. Merge T, I, P, F, L into one branch. Expected conflicts: `ribbon/spec.rs`, `ribbon/mod.rs`, `lib.rs` (test module list), `dialog.rs`, `docs/spec/course-model.md`. Keep every side.
2. Reviewer reads each diff against this plan, fixes problems, runs the full gate.
3. Reviewer updates `docs/ui/feature-parity.md` (rows and summary counts), `docs/ui/ribbon.md`, `docs/ui/panels.md`, `docs/roadmap.md` (M39 status) and writes `docs/testing/m39-report.md`. The handoff `FIXES.md` names #11 and #12 so the PR closes them.

## Out of scope (stays disabled)

Stacked text direction; All Caps and Small Caps; underline styles; tab stops; text columns; Paste Special for pictures from the OS clipboard; named theme font sets saved outside the course.
