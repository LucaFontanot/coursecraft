# Ribbon: main tabs

> Part of [the ribbon plan](README.md). Status marks are explained there. Reference layout:
> [storyline-observed.md](storyline-observed.md) §2.
>
> **Kinds:** `L` large button · `LS` large split button (▾ menu) · `S` small button · `SS` small
> split button · `C` combo box · `G` gallery · `N` number spinner · `K` checkbox · `T` toggle ·
> `↘` dialog launcher in the group caption.
>
> **Action:** an engine command id (`slide.add`), a UI action (`ui:preview.slide`, defined in
> `coursecraft-ui-egui`), or `—` when the control is disabled. Ids marked `(new)` do not exist yet.
> "Existing UI" names where the feature lives today, so the wiring reuses it instead of writing it
> again.

## Tab strip

| Order | Tab | Shown when |
|---|---|---|
| 0 | **File** | always; opens the file menu (below), drawn in the accent colour |
| 1 | **Home** | always; Course Map variant or Slide variant |
| 2 | **Insert** | slide open |
| 3 | **Slides** | always (hidden in master mode) |
| 4 | **Design** | always (hidden in master mode) |
| 5 | **Transitions** | slide open |
| 6 | **Animations** | slide open |
| 7 | **View** | always |
| 8 | **AI** | always; CourseCraft's own tab (Storyline keeps its AI Assistant on Home) |
| 9 | **Help** | always |
| 10+ | contextual | see [contextual-tabs.md](contextual-tabs.md) |

In the Course Map, Insert, Transitions and Animations stay in the strip but every control in them is
disabled with the tooltip `Open a slide first`. (This keeps the strip from jumping; Storyline hides
nothing either.)

Every main tab but AI and Help ends with a **Publish** group (Preview at least). Its controls are the same
everywhere and are specified once, under Home.

## File

Not a ribbon tab: a full-window **File page**, the same page CourseCraft shows on launch
(today's welcome screen, `crates/ui-egui/src/welcome.rs`). Layout per storyline-observed.md ›
FILE: an accent-coloured left column with the start actions, Recent and Browse for more…, and a
right area with CourseCraft news (release notes from the app, no network call) and links. Esc or
the Back entry returns to the editor. It replaces today's File menu (`menu.rs` `show_menu_bar`)
and keeps all of it:

| Start action (left column, top) | Action | Status | Existing UI |
|---|---|---|---|
| New Project | `ui:file.new` | ✅ | welcome screen, File › New |
| Record Screen | — | 🚫 | Not planned (patent watch list) |
| Import ▾ (PowerPoint…, Questions from CSV…, Course from Document…) | `ui:file.import.*` | ✅ | File › Import |
| Team Slides | — | — | shared cloud library; not drawn |
| Recent (thumbnail + name) | `ui:file.openRecent` | 🟡 | names only today; add thumbnails |
| Browse for more… | `ui:file.open` | ✅ | File › Open… |

Project commands (below the start actions once a course is open):

| Item | Action | Status | Existing UI |
|---|---|---|---|
| New | `ui:file.new` | ✅ | File › New |
| Open… | `ui:file.open` | ✅ | File › Open… |
| Open Folder… | `ui:file.openFolder` | ✅ | File › Open Folder… (CourseCraft extra, keep) |
| Open Recent ▸ | `ui:file.openRecent` | ✅ | File › Open Recent |
| Save | `ui:file.save` | ✅ | File › Save |
| Save As… | `ui:file.saveAs` | ✅ | File › Save As… |
| Import ▸ PowerPoint…, Questions from CSV…, Course from Document…, Outline | `ui:file.import.*` | ✅ | File › Import |
| Translation ▸ Export…, Import…, Translate with AI… | `course.export_xliff`, `course.import_xliff`, `ui:translate.ai` | ✅ | File › Export/Import Translation |
| Publish… | `ui:publish` | ✅ | toolbar Publish |
| Export ▸ Video (MP4)…, Word for review…, PDF for review… | `ui:export.*` | ✅ | File › Export |
| Course Info… | `course.info` | 🟡 | Course settings |
| Version History… | `ui:versionHistory` | ✅ | `version_history.rs` |
| Preferences… | `ui:preferences` | ✅ | File › Preferences… |
| Close | `ui:file.close` | ⛔ | today New replaces the course; add a real Close |
| Quit | `ui:quit` | ✅ | File › Quit |

## Home (Course Map variant)

Shown when the Course Map view tab is active.

> **In the code (M30 steps 1–2):** every main tab (Home in both variants, Insert, Slides, Design,
> Transitions, Animations, View, Help) is drawn from
> `crates/ui-egui/src/ribbon/spec.rs`; the ribbon is on by default (Preferences › Interface) and
> replaces the flat toolbar, with Save, Undo and Redo in a quick access toolbar left of the tab
> strip. The Status column is the feature's status. Built features that the ribbon doesn't reach
> yet (text formatting, Hyperlink, Apply Layout, Question Banks, Quick Styles, Shape
> Fill/Outline/Effect) are drawn disabled with "Not available yet: ribbon wiring". Groups that
> don't fit the window shrink to one button, rightmost first, which opens the group in a popup.

| Group | Control | Kind | Action | Status | Notes |
|---|---|---|---|---|---|
| Clipboard | Paste | LS | `slide.paste` | ✅ | pastes slides in the map |
| | Cut | S | `ui:edit.cut` | ✅ | |
| | Copy | S | `ui:edit.copy` | ✅ | |
| Scenes | New Scene | L | `scene.add` | ✅ | `toolbar::add_new_scene` |
| | Starting Scene | T | `course.update` `{start_scene}` (new field) | ⛔ | the course model has no start-scene field yet; the course starts at the first slide. Needs the field, a flag in the map and a runtime rule in `triggers.md` |
| | Index | LS | — | ⛔ | Storyline's scene index menu; behaviour to confirm |
| | Question Banks | LS | `bank.create`, `ui:banks.manage` | ✅ | ▾ New Question Bank, Manage Banks…, Import Questions from CSV… |
| Slide | New Slide | LS | `slide.add`, `slide.add_from_layout`, `template.add_slide` | 🟡 | ▾ menu below |
| | Apply Layout | SS | `slide.set_master` | ✅ | |
| | Reset | S | `slide.set_master` (reapply) | ⛔ | reset placeholders to the layout |
| | Duplicate | S | `slide.duplicate` | ✅ | |
| | Record Screen | LS | — | 🚫 | Not planned (patent watch list: Assima) |
| Publish | Player | L | `ui:player.properties` | ✅ | Course settings › Player |
| | Preview | LS | `ui:preview.*` | ✅ | ▾ This Slide, This Scene, Selected Slides, Entire Course, Publish… (`toolbar` preview popup) |
| | Publish | L | `ui:publish` | ✅ | |

**New Slide ▾** (shared by every tab that has New Slide):

| Item | Action | Status |
|---|---|---|
| Library… | — | 🚫 Not planned: content library |
| My Templates… | `template.add_slide` | ✅ (`slides::show_new_slide_popup`) |
| Team Slides… | — | — not drawn |
| Basic Layouts ▸ (gallery of the course's slide layouts, headed by the theme name) | `slide.add_from_layout` | ✅ |
| Import ▸ PowerPoint…, Questions from CSV… | `ui:file.import.*` | ✅ |
| Graded Question… / Survey Question… / Custom Question… | `question.insert` | ✅ |
| Interaction… | `interaction.insert` | ✅ (CourseCraft extra) |
| Results… | `results.insert` | ✅ |
| Record Screen… | — | 🚫 |

## Home (Slide variant)

| Group | Control | Kind | Action | Status | Notes |
|---|---|---|---|---|---|
| Clipboard | Paste | LS | `object.paste` | 🟡 | ▾ Use Destination Theme ✅, Keep Source Formatting ⛔, Keep Text Only ⛔, Paste Special… ⛔ |
| | Cut | S | `ui:edit.cut` | ✅ | |
| | Copy | SS | `ui:edit.copy` | ✅ | ▾ Copy, Duplicate (`object.duplicate`) |
| | Format Painter | S | `ui:formatPainter` | ✅ | double-click keeps it on |
| Slide | New Slide | LS | as Course Map | ✅ | |
| | Apply Layout | SS | `slide.set_master` | ✅ | |
| | Focus Order | S | `ui:focusOrder` | 🟡 | focus order exists (`focus_order`); needs its own dialog |
| | Duplicate | S | `slide.duplicate` | ✅ | |
| Font ↘ | Text Styles | LS | `text_style.apply` | ✅ | `text_styles_ui.rs`; ▾ lists course text styles |
| | Font family | C | `text.style_range` `{font}` | ✅ | `font_picker.rs` |
| | Font size | C | `text.style_range` `{size}` | ✅ | |
| | Grow Font / Shrink Font | S | `text.style_range` | ✅ | |
| | Clear Formatting | S | `text.style_range` (reset) | ⛔ | |
| | Bold / Italic / Underline | T | `text.style_range` | ✅ | |
| | Shadow (text) | T | — | ⛔ | |
| | Strikethrough | T | `text.style_range` | ✅ | |
| | Character Spacing | SS | `text.style_range` `{letter_spacing}` | ✅ | |
| | Change Case | SS | — | ⛔ | Sentence, lower, UPPER, Capitalize, tOGGLE |
| | Highlight | SS | `text.style_range` `{highlight}` (new field) | ⛔ | text runs have no highlight colour yet |
| | Font Color | SS | `text.style_range` `{color}` | ✅ | theme colours + eyedropper (`eyedropper.rs`) |
| | Spelling | S | `ui:spellCheck` | ✅ | `spell_check.rs` |
| | ↘ Font dialog | — | `ui:dialog.font` (new) | ⛔ | |
| Paragraph ↘ | Bullets | SS | `text.set_paragraph` `{bullet}` | ✅ | ▾ bullet symbols |
| | Numbering | SS | `text.set_paragraph` `{numbering}` | 🟡 | verify numbering styles |
| | Decrease / Increase Indent | S | `text.set_paragraph` `{indent}` | ✅ | |
| | Line Spacing | SS | `text.set_paragraph` `{line_spacing}` | ✅ | |
| | Align Left / Center / Right / Justify | T | `text.set_paragraph` `{align}` | ✅ | |
| | Text Direction | SS | — | ⛔ | horizontal, rotate 90°/270°, stacked |
| | Align Text | SS | `object.update` `{v_align}` | 🟡 | top / middle / bottom; verify field |
| | Find / Replace | SS | `course.find`, `course.replace` | ✅ | `find.rs` |
| | ↘ Paragraph dialog | — | `ui:dialog.paragraph` (new) | ⛔ | |
| Drawing ↘ | Arrange | LS | `object.reorder`, `object.align`, `object.distribute`, `object.group` | ✅ | ▾ Order, Group, Align, Rotate (as Edit › Order/Align) |
| | Quick Styles | LS | `object.apply_style` | ✅ | |
| | Shape Fill | SS | `object.set_fill_style` | ✅ | |
| | Shape Outline | SS | `object.update` `{outline}` | ✅ | |
| | Shape Effect | SS | `object.set_shadow`, `object.set_glow` | 🟡 | Shadow ✅, Glow ✅, Reflection ⛔, Soft Edges ⛔ |
| | ↘ Format Shape | — | `ui:inspector.format` | ✅ | opens the Format inspector |
| Publish | Player / Preview / Publish | | as Course Map | ✅ | |

## Insert

| Group | Control | Kind | Action | Status | Notes |
|---|---|---|---|---|---|
| Slide | Slide Layer | L | `layer.add` | ✅ | `toolbar::add_new_layer`. **Insert is built in `spec.rs`**; the menu bar's Insert menu is hidden while the ribbon is on |
| | Convert to Custom Question | L | `question.set_kind` | ⛔ | turn the slide's objects into a custom question |
| | Zoom Region | L | — | ⛔ | camera zoom on part of the slide |
| AI | Insert Text, Edit Text, Insert Image, Insert Audio, Quiz, Question, Summary | | on the AI tab | | |
| Library | Characters | L | — | 🚫 | Not planned: characters |
| | Photos | L | — | 🚫 | Not planned: content library |
| | Illustrations | L | — | 🚫 | |
| | Icons | L | `ui:insert.icon` | ⛔ | CourseCraft ships no icon set yet; a bundled open icon set would make this ✅ |
| | Videos | L | — | 🚫 | |
| Media | Picture | LS | `object.add` `{image}` | ✅ | ▾ Picture from File… ✅, Media Library… ✅ (`media_library.rs`), Screenshot ⛔ |
| | 360° Image | LS | — | 🚫 | Not planned: 360° images |
| | Shape | LS | `object.add` `{shape}` | ✅ | `shape_gallery.rs` |
| | Caption | LS | `object.add` `{shape: callout}` | ⛔ | callout shapes; check the shape gallery |
| | Video | LS | `object.add` `{video}` | 🟡 | ▾ Video from File… ✅, From Website (embed) ✅ (`web_insert.rs`), Record Webcam ⛔, Media Library… ✅ |
| | Audio | LS | `object.add` `{audio}` | 🟡 | ▾ Audio from File… ✅, Media Library… ✅, Record Mic… ✅ (`record_narration.rs`), Text-to-Speech… ✅ (`audio_ai.rs`), AI Audio ▸ ⛔, Background Audio ▸ ✅ (`course.set_background_audio`), Options… ⛔ |
| | Web Object | L | `object.add` `{web}` | ✅ | `web_insert.rs` |
| | Animation (Lottie) | L | `object.set_lottie` | ✅ | CourseCraft extra; place after Web Object |
| Text | Text Box | LS | `object.add` `{text}` | ✅ | |
| | Table | LS | `object.add` `{table}` | ✅ | grid picker on ▾ |
| | Slide Number | LS | — | ⛔ | slide number / total field |
| | Symbol | S | — | ⛔ | symbol picker |
| | Reference | S | — | ⛔ | variable reference `%var%` insert; variables exist |
| | Hyperlink | S | `text.set_link` | ✅ | |
| Interactive Objects | Button | LS | `object.add` `{button}` | ✅ | ▾ button styles |
| | Slider | LS | `object.add` `{slider}` | ✅ | `input_insert.rs` |
| | Dial | LS | `object.add` `{dial}` | ✅ | |
| | Hotspot | LS | `object.add` `{hotspot}` | ⛔ | no hotspot object yet: hotspot questions have their own regions and the Hotspot reveal interaction covers clickable areas; ▾ rectangle, oval, freeform to come |
| | Input | LS | `object.add` `{text_entry}` / `object.set_input` | ✅ | ▾ Text Entry, Checkbox, Radio Buttons |
| | Marker | LS | `object.set_marker` | ✅ | |
| | Trigger | S | `trigger.add` | ✅ | |
| | Scrolling Panel | S | `object.set_scroll_panel` | ✅ | |
| | Mouse | SS | `slide.update` `{cursor}` | ⛔ | custom cursor per slide |
| | Interaction | L | `interaction.insert` | ✅ | CourseCraft extra (`interaction_gallery.rs`) |
| Publish | Preview | LS | | ✅ | |

## Slides

Not shown in the tutorial (see storyline-observed.md). The list below is the proposal; confirm the
order against the app before building.

| Group | Control | Kind | Action | Status |
|---|---|---|---|---|
| Slide | New Slide | LS | `slide.add` | ✅ (built in `spec.rs`, as is the rest of this tab) |
| | Import | LS | `ui:file.import.*` | ✅ ▾ PowerPoint…, Questions from CSV…, Course from Document…, Slides from Outline… |
| | Duplicate | S | `slide.duplicate` | ✅ |
| Scene | New Scene | L | `scene.add` | ✅ |
| Questions | Graded / Survey / Custom Question | LS | `question.insert` | ✅ (one Question ▾ listing the kinds) |
| | Question Banks | LS | `bank.*` | ✅ (drawn disabled: ribbon wiring) |
| | Results Slide | L | `results.insert` | ✅ |
| | Certificate | L | `certificate.insert` | ✅ (CourseCraft extra) |
| Masters | Slide Master | L | `ui:view.master` | ✅ |
| | Feedback Master | L | — | ⛔ |
| Publish | Preview | LS | | ✅ |

## Design

| Group | Control | Kind | Action | Status | Notes |
|---|---|---|---|---|---|
| Setup | Slide Size | L | `course.set_slide_size` | ✅ | `slide_size_dialog.rs`. In the code: Themes, Colors and Fonts open the theme window; Background Styles waits for ribbon wiring (↘ opens the Slide inspector) |
| Themes | theme gallery | G | `course.set_theme` | ✅ | `theme_dialog.rs`; gallery of built-in themes |
| Theme options | Colors | SS | `theme.set_palette` | ✅ | ▾ palettes, Create New Theme Colors… (`theme.set_palette_color`) |
| | Fonts | SS | `course.update` `{fonts}` | 🟡 | ▾ font pairs, Create New Theme Fonts… |
| Background ↘ | Background Styles | SS | `slide.set_background` | ✅ | |
| | Hide Background Graphics | K | `slide.update` `{hide_master_graphics}` | ⛔ | |
| Publish | Preview | LS | | ✅ | |

## Transitions

Not shown in the tutorial; standard layout.

| Group | Control | Kind | Action | Status |
|---|---|---|---|---|
| Transition to This Slide | transition gallery | G | `slide.set_transition` | ✅ (in the code: a Transition button that opens the Slide inspector until the gallery is wired) |
| | Effect Options | LS | `slide.set_transition` `{direction}` | 🟡 |
| Timing | Duration | N | `slide.set_transition` `{duration}` | ✅ |
| | Apply To All | S | `slide.set_transition` (all) | ⛔ |
| Publish | Preview | LS | | ✅ |

## Animations

| Group | Control | Kind | Action | Status | Notes |
|---|---|---|---|---|---|
| Advanced | Animation Painter | L | `ui:animationPainter` | ⛔ | Format Painter for animations |
| Entrance Animations | effect gallery | LS | `object.set_animation` `{entrance}` | ✅ | in the code: Entrance and Exit buttons open the Animate inspector until the galleries are wired; Add Motion Path runs `object.add_motion_path`; None, Fade, Grow, Fly In, Float In, Split, Wipe, Shape, Wheel, Random Bars, Spin, Spin & Grow, Grow & Spin, Zoom, Swivel, Bounce; missing effects disabled in the gallery |
| | Duration | N | `object.set_animation` `{duration}` | ✅ | |
| | Effect Options | SS | `object.set_animation` `{direction, sequence}` | 🟡 | direction ✅; Sequence By Paragraph ⛔ |
| Exit Animations | (same three) | | `object.set_animation` `{exit}` | ✅ | |
| Emphasis Animations | Add Emphasis Animation | LS | — | ⛔ | pulse, spin, teeter… at a time |
| | emphasis list | G | — | ⛔ | |
| | Duration / Effect Options / Delete | | — | ⛔ | |
| Motion Paths | Add Motion Path | LS | `object.add_motion_path` | ✅ | |
| | Duration, Name | N, C | `object.update_motion_path` | ✅ | |
| | Path Options | LS | `object.update_motion_path` | 🟡 | |
| Size and Position ↘ | X, Y, Width, Height | N×4 | `object.update` | ✅ | |
| Publish | Preview | LS | | ✅ | |

## View

Not shown except Slide Master; proposal.

| Group | Control | Kind | Action | Status |
|---|---|---|---|---|
| Views | Course Map | L | `ui:view.map` | ✅ (built in `spec.rs`, as is the rest of this tab) |
| | Slide | L | `ui:view.slide` | ✅ |
| | Slide Master | L | `ui:view.master` | ✅ |
| | Feedback Master | L | — | ⛔ |
| Zoom | Zoom | L | `ui:zoom.dialog` | 🟡 (Zoom In, Zoom Out, 100% for now) |
| | Fit in Window | L | `ui:zoom.fit` | ✅ |
| Show | Rulers | K | `view.set` `{rulers}` | ✅ |
| | Grid | K | `view.set` `{grid}` | ✅ |
| | Guides | K | `view.set` `{guides}` | ✅ |
| | Snap to Grid / Snap to Guides | K | `view.set` | ✅ |
| Panels | Timeline, States, Notes, Triggers, Layers, Scenes | K | `ui:panel.toggle` | 🟡 (Slides, Timeline, and the Format, Animate, Slide and Course inspectors, each its own panel; the others are tabs inside the timeline panel) |
| | Reset Panels | S | `ui:panel.reset` | ✅ |
| Theme | Light / Dark | T | `ui:theme` | ✅ (Light, Dark, Match System) |
| Publish | Preview | LS | | ✅ |

## AI

CourseCraft's own tab: every AI control in one place, so Home keeps only editing. Storyline puts
its AI Assistant group on Home; the controls and their status are the same.

| Group | Control | Kind | Action | Status | Notes |
|---|---|---|---|---|---|
| Write | Insert Text | L | — | ⛔ | draft text on the slide |
| | Edit Text | L | — | ⛔ | rewrite selected text |
| | Summary | L | `ui:ai.summary` (new) | ⛔ | |
| Media | Insert Image | L | — | ⛔ | |
| | Insert Audio | LS | `ui:narration.fromText` | 🟡 | ▾ Narration from Text… ✅ (`audio_ai.rs`), Sound Effect ⛔, Music ⛔ |
| Assess | Quiz | L | `ui:ai.quiz` (new) | ⛔ | draft a quiz from the course text through the user's agent |
| | Question | L | `ui:ai.question` (new) | ⛔ | |
| Agents | Connect an AI Agent…, AI Settings…, Allow automation | S×3 | `ui:ai.*` | ✅ | moved from Help |

## Help

| Group | Control | Action | Status |
|---|---|---|---|
| Help | Help / Guide | `ui:help.guide` | ✅ (opens the docs on GitHub) |
| | Keyboard Shortcuts | `ui:help.shortcuts` | ✅ |
| | Welcome Screen | `ui:welcome` | ✅ |
| Community | CourseCraft on GitHub, Report a Problem…, Suggest a Feature… | `ui:help.*` | ✅ |
| Updates | Check for Updates | `ui:updates` | ✅ |
| About | About CourseCraft | `ui:about` | ⛔ (no About window yet; drawn disabled) |

CourseCraft's **Project** menu (Course settings, Variables, Glossary, Resources, Audit,
Accessibility check, Suggest Alt Text…) has no Storyline twin on a tab. Course settings and Player go
under Home › Publish › Player; Variables, Glossary and Resources move into Player Properties and
the Triggers panel (as in Storyline); Audit and Accessibility go under Review controls in View until
a Review tab is decided.
