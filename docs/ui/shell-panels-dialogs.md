# Shell, panels, context menus and dialogs

> Part of [the ribbon plan](README.md). Reference: [storyline-observed.md](storyline-observed.md)
> §1, §4, §5. Status marks as in README.

## Window layout

Top to bottom:

1. **Title bar** with the quick access toolbar (QAT) on the left and the document title centred:
   `<course name>* - CourseCraft`. The asterisk marks unsaved changes.
2. **Tab strip** (File + tabs + contextual tabs; help `?` at the far right).
3. **Ribbon** (one row of groups, about 95 px; can be collapsed to the tab strip).
4. **View tabs row**: Course Map tab, then one tab per open slide (closable ×), and breadcrumb
   tabs for sub-editors. Right side: preview device toggles and a settings gear.
5. **Work area**: Scenes panel (left), canvas (centre), Timeline/States/Notes (bottom of centre),
   right panel (Triggers above, Slide Layers below).
6. **Status bar**.

| Element | Storyline | CourseCraft today | Status | Work |
|---|---|---|---|---|
| QAT: Save, Undo, Redo, Customize ▾ | yes | Save, Undo, Redo, Undo History left of the ribbon tab strip (`ribbon/mod.rs`) | 🟡 | Customize: right-click a ribbon button → Add to QAT (as DeckCraft); move into the title bar |
| Title `name* - App` | yes | window title | 🟡 | add the `*` |
| Menu bar | none | drawn only when Preferences › Interface › Ribbon is off; with the ribbon, File is the File page and every other item has a ribbon home (ribbon.md › Where the menu bar went) | — | delete the code with the flat toolbar (step 4) |
| Flat toolbar | none | `toolbar.rs`, drawn only when Preferences › Interface › Ribbon is off | — | its buttons are on Home, Insert, Slides and View; Format, Animate, Slide and Course are dock panels (a tab row in the fixed Inspector when docking is off); delete the code with the menu bar (step 4) |
| Collapse ribbon (Ctrl+F1, double-click a tab) | yes | click or double-click the open tab, Ctrl+F1 | ✅ | |
| File page = start screen (left action column, Recent with thumbnails, news) | yes | `file_page.rs` from the File tab (accent column, Recent, Import, Export, Translation, Fonts); `welcome.rs` on launch | 🟡 | merge the welcome screen into it; thumbnails and news |
| Tooltips: name, shortcut, one sentence | yes | partial | 🟡 | every ribbon control, from the registry |
| View tabs row with open slides | yes | `doc_tabs.rs`: Course Map tab, then each opened slide (× closes; closing the shown one shows its neighbour, the last one the map) | ✅ | breadcrumb tabs for sub-editors |
| Device toggles in view row | yes | — | ⛔ | they set the preview device |
| Panels undock / float / dock back | yes | docked by default (`dock.rs`); View › Panels closes and reopens; saved in prefs; Timeline, States, Triggers, Variables, Question and Notes are dock panels of their own, and Slide Layers sits beside them; a floating panel can be dragged out of the app window into its own OS window (desktop; off on Wayland, which reports no window positions), and keys typed there reach the editor's shortcuts | ✅ | — |
| Status bar: Slide n of m, size, theme | yes | `status.rs` | 🟡 | add size and theme name |
| Status bar: view buttons, zoom %, slider, fit | yes | zoom in View | 🟡 | |

## Panels

### Scenes panel (left)

| Feature | Status | Notes |
|---|---|---|
| Scene combo; mouse wheel switches scenes | 🟡 | `slides.rs`; add wheel |
| Thumbnails numbered `scene.slide Title` | ✅ | |
| Flow arrows between slides; link icon at the end | ⛔ | from slide navigation triggers |
| Right-click: New Slide ▸ layouts, Apply Layout ▸, Duplicate, Delete, Rename, Preview | 🟡 | |
| Undock | ⛔ | |

### Course Map (Story view)

✅ exists (`course_map.rs`). Parity items: scene cards with a header menu ▾ ⛔, start-scene flag ⛔,
Ctrl+wheel zoom 🟡, double-click to rename scene and slide titles 🟡, double-click a slide opens a
slide tab ⛔ (needs view tabs). In the Course Map the right panel shows **Slide Properties**:

| Field | Values | Action | Status |
|---|---|---|---|
| Slide advances | By user / Automatically | `slide.update` `{advance}` | ✅ |
| When revisiting | Automatically decide / Resume saved state / Reset to initial state | `slide.update` `{revisit}` | ✅ |
| Buttons: Prev, Next, Submit | checkboxes | `slide.set_player_overrides` | ✅ |
| Swipe: Prev, Next | checkboxes | `slide.set_player_overrides` | ⛔ |
| Player features | Player defaults / Custom | `slide.set_player_overrides` | ✅ |

### Timeline / States / Notes (bottom)

CourseCraft has Timeline (`timeline.rs`), States (`states.rs`, M29) and Notes. Parity items:

| Feature | Status |
|---|---|
| Tabs Timeline, States, Notes; collapse to a one-line bar; undock | 🟡 (docked: Timeline, States, Triggers, Variables, Question and Notes undock as dock panels, and Slide Layers is one beside them; collapse to come) |
| Eye and lock columns | ✅ (`object.hide_in_editor`, `object.lock`) |
| Double-click a row to rename | ✅ |
| Bars with entrance/exit wedges; drag start, end, ends | ✅ |
| Expand ▸ for paragraph build steps | ⛔ (needs Sequence By Paragraph) |
| Cue points: **C** while playing, snap, Create Cue Point at Playhead | ✅ (`slide.add_cue`) verify the C key |
| Footer: Play, Stop, zoom, duration | ✅ |
| Row right-click menu (below) | 🟡 |

Notes tab: per-slide text, background follows the player colours, optional learner-facing tab
(Player Properties › Notes) 🟡 — notes exist; check the player tab and the background.
Rows restack by drag and groups expand with ▸ ✅ (verify group rows).

States tab: Edit States button, Initial state combo, one thumbnail per state, built-in markers ✅
(M29).

### Right panel

| Feature | Status | Notes |
|---|---|---|
| Panel tabs: Slide, Comments, AI | 🟡 | Slide ✅ (inspector), Comments 🚫 (review workflow not planned), AI ⛔ (chat with the connected agent) |
| Triggers: toolbar New, Edit, Copy, Paste, Delete, Up, Down; Group; Variables | ✅ | `triggers.rs`, `variables.rs`; check Copy/Paste |
| Sections Object / Slide / Layer / Player triggers, sentence rows with inline links | ✅ | |
| Slide Layers: base last, eye, New, Duplicate, Delete, Dim, properties ⚙ | ✅ | `layers.rs` |
| Format / Animate / Slide / Course inspectors | CourseCraft extra | dock panels of their own (✅); every ↘ opens a settings panel instead of a dialog, see [panels.md](panels.md) |

## Context menus

### Object on the canvas

| Item | Action | Status |
|---|---|---|
| Cut, Copy | `ui:edit.*` | ✅ |
| Paste Options (3), Paste Special… | `object.paste` | 🟡 |
| Group ▸ Group / Ungroup | `object.group`, `object.ungroup` | ✅ |
| Bring to Front ▸ / Send to Back ▸ | `object.reorder` | ✅ |
| Button Set ▸ | — | ⛔ |
| Drag and Drop ▸ | `object.set_drag` | ✅ |
| Add new horizontal / vertical guide line | `view.guides.add` | ✅ |
| Replace Picture ▸ / Export Picture ▸ | `object.update` / `ui:media.export` | 🟡 |
| Show in Media Library… | `ui:mediaLibrary.reveal` | ⛔ |
| Rename "<name>" | `ui:rename` | ✅ |
| Size and Position… | `UiAction::PanelAt(SizePosition)` | ✅ (a settings panel, [panels.md](panels.md) step 3) |
| Accessibility… | `ui:altText` | ✅ (`alt_text.rs`) |
| Edit States… | `ui:states` | ✅ (M29) |
| Format Shape / Picture… | `ui:inspector.format` | ✅ |

### Timeline row

Cut, Copy, Paste, Paste Special… 🟡 · Rename ✅ · Show in Media Library… ⛔ · Bring to Front ▸,
Send to Back ▸ ✅ · Create Cue Point at Playhead ✅ · Fit Timeline to Longest Object ⛔ · Reset
Timeline to Default ⛔ · Alignment ▸ (to playhead, to cue point) ⛔ · Show Until End / Show Always
✅ · Timing… ⛔ (dialog) · Accessibility… ✅.

### Scenes panel / Course Map

New Slide ▸, New Scene, Apply Layout ▸, Duplicate, Delete, Rename, Move to Scene ▸, Preview ▸
🟡.

## Dialogs

| Dialog | Opened from | CourseCraft | Status |
|---|---|---|---|
| Slide Size | Design › Slide Size | `slide_size_dialog.rs` | ✅ add Lock aspect ratio, Current Default Size, Scale existing content ⛔ |
| Create New Theme Colors | Design › Colors ▾ | `theme_dialog.rs` | 🟡 match the slot list: Light 1, Dark 1, Light 2, Dark 2, Accent 1–6, Hyperlink, Control, Quiz Correct, Quiz Incorrect, Custom 1–2 |
| Create New Theme Fonts | Design › Fonts ▾ | `font_picker.rs` | ⛔ heading + body + sample + name |
| Trigger | Triggers › New | `triggers.rs` (inline editor) | 🟡 add the dialog form: Action, target, When, Object, Conditions |
| Insert Slide | New Slide ▾ | `slides::show_new_slide_popup`, `interaction_gallery.rs` | 🟡 one dialog with filters (Type, In Use), search, sort, preview pane, multi-select |
| Insert Question | New Slide › Graded/Survey/Custom | `toolbar::insert_question` | 🟡 same dialog, question tiles |
| Player Properties | Home › Player | `course_settings.rs` | 🟡 own ribbon (Features, Menu, Resources, Glossary, Colors & Effects, Text Labels, Other) and live preview |
| Publish | Home › Publish | `publish.rs` | 🟡 left list: Share for Review 🚫, Web ✅, Video ✅, Word ✅, LMS (SCORM 1.2, SCORM 2004, cmi5, xAPI) ✅; Properties links (Player, Quality, Publish scope, Tracking) |
| Font, Paragraph | ↘ launchers | Text settings panel (`panels.rs`) | 🟡 caps, highlight, underline styles, tab stops, columns to come ([panels.md](panels.md)) |
| Size and Position | ↘ launcher, context menu | the Size and Position panel | 🟡 built and opened from Animations; the context menu entry is still to do |
| Find and Replace | Home › Find / Replace | `find.rs` | ✅ |
| Characters, Photos, Illustrations, Videos | Insert › Library | — | 🚫 |
| Media Library | Insert › Picture ▾ | `media_library.rs` | ✅ |
| Spelling | Home › Spelling | `spell_check.rs` | ✅ |
| Accessibility (Alt text) | context menu | `alt_text.rs` | ✅ |

## Keyboard

Keep the existing shortcuts (`shortcuts.rs`). Add: Ctrl+F1 collapse ribbon ⛔, Alt key tips on
Windows ⛔, **C** add cue point while the timeline plays (verify), F12 preview slide ✅,
Ctrl+F12 preview course (verify), Ctrl+wheel zoom in Course Map and canvas, Shift+wheel zoom
canvas (Storyline) ⛔.
