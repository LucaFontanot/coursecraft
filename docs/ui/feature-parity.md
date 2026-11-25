# Authoring feature status

> Part of [the ribbon plan](README.md). **Written:** 2026-11-23. One row per authoring **feature**
> in CourseCraft, whatever tab or dialog it sits in. The control-by-control view is in
> [ribbon.md](ribbon.md) and [contextual-tabs.md](contextual-tabs.md); this file is the summary
> that drives the roadmap.
>
> Status marks as in README (✅ built, 🟡 partly, ⛔ not yet, 🚫 not planned, — left out).
> **Milestone** is where the work is planned in `docs/roadmap.md`. A row changes in the same commit
> as the code that changes it.

## Summary

| Area | ✅ | 🟡 | ⛔ | 🚫 / — |
|---|---|---|---|---|
| Project and file | 7 | 2 | 1 | 2 |
| Structure: scenes, slides, masters | 8 | 3 | 6 | 0 |
| Text | 12 | 4 | 8 | 0 |
| Shapes, pictures and buttons | 16 | 4 | 9 | 0 |
| Media | 10 | 3 | 5 | 1 |
| Interactivity | 12 | 3 | 2 | 0 |
| Animation, transitions, timeline | 13 | 1 | 8 | 0 |
| Questions and quizzes | 8 | 4 | 3 | 0 |
| Player, preview, publish | 10 | 4 | 7 | 2 |
| Editor shell and panels | 12 | 9 | 2 | 0 |
| AI | 3 | 1 | 6 | 0 |
| Assets and services | 0 | 0 | 1 | 7 |
| **Total** | **111** | **38** | **58** | **12** |

Counted from the tables below; recount when a row changes.

## Project and file

| Feature | CourseCraft | Status | Milestone |
|---|---|---|---|
| New, Open, Save, Save As | File menu | ✅ | — |
| Recent projects | File › Open Recent (names) | 🟡 thumbnails missing | M38 |
| Start screen / File page with start actions and Recent | File page from the File tab and on launch (`file_page.rs`, its Start section is the welcome screen), Recent with first-slide thumbnails; news to come | 🟡 news | M38 |
| Import PowerPoint | File › Import | ✅ | — |
| Import questions (CSV / spreadsheet) | File › Import | ✅ | — |
| Translation export / import (XLIFF) | File › Translation | ✅ | — |
| Version history, autosave and recovery | `version_history.rs` | ✅ | — |
| Close project (without quitting) | — | ⛔ | M38 |
| Slide size and default size | Slide Size dialog | ✅ | — |
| Project info (title, author, description) | Course settings | ✅ | — |
| Team Slides (shared cloud library) | — | — | — |
| Record Screen project | — | 🚫 patent watch list | — |

## Structure: scenes, slides, masters

| Feature | CourseCraft | Status | Milestone |
|---|---|---|---|
| Scenes; new scene; rename | `scene.*` | ✅ | — |
| Start section (start flag) | — | ⛔ needs model field + runtime rule | M41 |
| Course Map: cards, zoom, fit | `course_map.rs` | ✅ | — |
| Course Map: collapse scene cards, branch lines between scenes | — | ⛔ | M42 |
| New slide from layout, Basic Layouts gallery | `slide.add_from_layout` | ✅ | — |
| My Templates | `template.*` | ✅ | — |
| Duplicate, delete, move slides | `slide.*` | ✅ | — |
| Apply layout; reset slide to layout | `slide.set_master` | 🟡 reset ⛔ | M41 |
| Slide masters and layouts (insert, rename, delete) | `master.*` | ✅ | — |
| Master: Preserve, Title checkbox, Insert Placeholder | — | ⛔ | M41 |
| Hide background graphics | — | ⛔ | M41 |
| Feedback layouts | — | ⛔ | M41 |
| Slide layers (new, duplicate, delete, dim, properties) | `layer.*` | ✅ | — |
| Slide properties: advance, revisit, prev/next/submit | `slide.update`, `slide.set_player_overrides` | ✅ | — |
| Swipe gestures (prev/next) | — | ⛔ | M41 |
| Focus order | audit only | 🟡 dialog missing | M39 |
| Themes: colours, fonts, gallery | `course.set_theme`, `theme.*`; Design › Themes gallery of built-in themes; Theme dialog uses the shared font picker for the course theme font | 🟡 custom font-pair creation dialog still planned | M39 |

## Text

| Feature | CourseCraft | Status | Milestone |
|---|---|---|---|
| Text boxes, point text, placeholders, autofit | `object.add`, `object.set_autofit`, `text.set_paragraph` `{wrap}` (click places point text; Wrap Text in Home › Paragraph and Text panel › Text Box converts to area text) | ✅ | — |
| Font family and size, grow/shrink | `text.style_range`; Home › Font through `text_format.rs` | ✅ | — |
| Bold, italic, underline, strikethrough | `text.style_range` | ✅ | — |
| Superscript, subscript | `text.style_range` | ✅ | — |
| Character spacing | `letter_spacing` | ✅ | — |
| Font colour with theme colours and eyedropper | `eyedropper.rs` | ✅ | — |
| Text styles | `text_style.*` | ✅ | — |
| Bullets, numbering, indent, line spacing, alignment | `text.set_paragraph(s)`; Home › Paragraph through `text_format.rs` | 🟡 numbering styles | M39 |
| Find and replace | `course.find`, `course.replace` | ✅ | — |
| Spelling | `spell_check.rs` (English dictionary; skips non-English course languages with a note) | ✅ | — |
| Hyperlinks | `text.set_link` | ✅ | — |
| Format painter | `format_painter.rs` | ✅ | — |
| Theme fonts (heading + body) | Theme dialog (`course.set_theme` sets one course theme font; separate heading and body font pairs not yet) | 🟡 | M39 |
| Paste options (destination theme, keep source, text only) | `object.paste` | 🟡 one mode | M39 |
| Vertical text alignment | `text.set_paragraph` `{valign}`: Home › Paragraph › Align Text, inspector | ✅ | — |
| Clear formatting | — | ⛔ | M39 |
| Text shadow | — | ⛔ | M39 |
| Change case | — | ⛔ | M39 |
| Highlight colour | — | ⛔ needs model field | M39 |
| Text direction (rotate, stacked) | — | ⛔ | M39 |
| Font and Paragraph settings | Text settings panel (`panels.rs`, from Font ↘ and Paragraph ↘): every text control, space before and after; caps, highlight, underline styles, tab stops and columns drawn disabled | 🟡 | M39 |
| Slide number field | — | ⛔ | M39 |
| Symbol picker | — | ⛔ | M39 |
| Insert variable reference | — | ⛔ variables exist | M39 |

## Shapes, pictures and buttons

| Feature | CourseCraft | Status | Milestone |
|---|---|---|---|
| Shape gallery, edit points | `shape_gallery.rs` | ✅ | — |
| Shape fill (solid, gradient, picture) | `object.set_fill_style` | ✅ | — |
| Outline, joins | `object.update` | ✅ | — |
| Quick styles / shape styles | `object.apply_style` | ✅ | — |
| Shadow, glow | `object.set_shadow`, `object.set_glow` | ✅ | — |
| Arrange, group, interactive button groups, align, distribute, match size | `object.*`, `group.set_interactive`, `group.update` (Home › Arrange, contextual Arrange and Align, Format inspector, Timeline Button pill) | ✅ | — |
| Rotate, flip | `object.set_rotation`, `object.flip` | ✅ | — |
| X, Y, width, height | `object.update` | ✅ | — |
| Smart guides, grid, guides, rulers | `snap.rs`, `rulers.rs`; the Grid and Guides panel (`grid_panel.rs`) sets the grid spacing and lists the guides | ✅ | — |
| Lock and hide in editor | `object.lock`, `object.hide_in_editor` | ✅ | — |
| Pictures: insert, replace, crop | `object.set_crop`, `crop.rs` | ✅ | — |
| Picture border | `object.update` | ✅ | — |
| States (built-in and custom, items inside states) | M29 | ✅ | — |
| Recolor | `course.recolor` | 🟡 SVG only | M40 |
| Reset picture | | 🟡 | M40 |
| Picture effects | | 🟡 as shape effects | M40 |
| Button styles, fill, border, effects | | 🟡 | M40 |
| On Click on a button | trigger editor with the click event and selected button pre-filled | ✅ | — |
| Reflection, soft edges | — | ⛔ | M40 |
| Change shape (keep format) | — | ⛔ | M40 |
| Size and Position dialog | the Size and Position panel (`size_panel.rs`): width and height with an aspect lock, rotation, flips, position from the top left or the centre, lock | ✅ | — |
| Brightness, contrast | — | ⛔ | M40 |
| Picture styles (frames, bevels) | — | ⛔ | M40 |
| Crop to shape | — | ⛔ | M40 |
| Zoom picture (learner enlarges) | — | ⛔ | M40 |
| Button icons (gallery, align, colour, remove) | — | ⛔ | M40 |
| Selection groups | — | ⛔ | M40 |
| Caption / callout shapes | Insert › Media › Caption (Callouts category in `shape_gallery.rs`) | ✅ | — |
| Export picture | — | ⛔ | M40 |

## Media

| Feature | CourseCraft | Status | Milestone |
|---|---|---|---|
| Audio from file | `object.add` | ✅ | — |
| Record microphone | `record_narration.rs` | ✅ | — |
| Text-to-speech narration | `audio_ai.rs` | ✅ | — |
| Background audio | `course.set_background_audio` | ✅ | — |
| Video from file, from website | `object.add`, `web_insert.rs` | ✅ | — |
| Video play options, controls, volume | `object.set_media` | ✅ | — |
| Video cue points and in-video questions | `video.*` | ✅ | — |
| Web object | `object.add` | ✅ | — |
| Media library (list, replace, unused) | `media_library.rs` | ✅ | — |
| Lottie animation | `object.set_lottie` | ✅ (CourseCraft extra) | — |
| Audio / video editor | trim only | 🟡 | M40 |
| Closed captions (add, import, export, delete) | `object.set_captions` | 🟡 | M40 |
| AI audio (sound effects, music) | — | ⛔ | M43 |
| Audio options dialog | — | 🟡 | M40 |
| Export audio / video | — | ⛔ | M40 |
| Compression setting | — | ⛔ | M40 |
| Show video in a new window | — | ⛔ | M40 |
| Record webcam | — | ⛔ | M40 |
| 360° images | — | 🚫 | — |

## Interactivity

| Feature | CourseCraft | Status | Milestone |
|---|---|---|---|
| Triggers: actions, events, conditions | `trigger.*` | ✅ | — |
| Object / slide / layer / player trigger sections | `triggers.rs` | ✅ | — |
| Variables | `variable.*` | ✅ | — |
| Buttons, hotspots, markers | `object.add`, `object.set_marker`; no hotspot object yet (hotspot questions and the Hotspot reveal interaction only) | 🟡 | M38 |
| Sliders, dials | `Slider`, `Dial` | ✅ | — |
| Text entry, checkbox, radio buttons | `object.set_input` | ✅ | — |
| Drag and drop | `object.set_drag` | ✅ | — |
| Scrolling panel | `object.set_scroll_panel` | ✅ | — |
| Tables | `table.*` | ✅ | — |
| Interactions gallery (tabs, process, flip cards…) | `interaction.insert` | ✅ (CourseCraft extra) | — |
| Timer, meter, motion path | `object.set_timer`, `object.set_meter`, `object.add_motion_path` | ✅ | — |
| Glossary, resources | `glossary.*`, `resource.*` | ✅ | — |
| Copy and paste triggers | `triggers.rs` | ✅ | — |
| Trigger dialog (Action / When / Conditions form) | inline editor | 🟡 | M38 |
| Built-in states for every input | M29 | 🟡 check the 5 button states | M40 |
| Custom mouse cursor | — | ⛔ | M41 |
| Convert slide to custom question | — | ⛔ | M41 |

## Animation, transitions, timeline

| Feature | CourseCraft | Status | Milestone |
|---|---|---|---|
| Entrance and exit animations | `object.set_animation` | ✅ | — |
| Animation duration and effect options (8 directions, shape, in/out, spokes, orientation) | `object.set_animation`; Animate inspector lists each option the effect takes | ✅ | — |
| Motion paths (add, duration, name, options) | `object.*motion_path` | ✅ | — |
| Slide transitions and duration | `slide.set_transition` | ✅ | — |
| Timeline: bars, drag timing, rename, restack | `timeline.rs` | ✅ | — |
| Show until end / show always | `timeline.rs` | ✅ | — |
| Cue points, snapping | `slide.add_cue` | ✅ | — |
| Slide duration | `slide.set_duration` | ✅ | — |
| Play / stop / zoom in timeline | `timeline_playback.rs` | ✅ | — |
| Timeline shows current layer; group rows expand | `timeline.rs` | ✅ | — |
| Full entrance gallery (16 effects) | Animations › Entrance / Exit galleries (`ribbon/gallery.rs`): 25 effects from the registry, each also as an exit | ✅ | M46 |
| Transition effect options | Transitions › Effect Options ▾ (from left, right, top, bottom) | ✅ | — |
| Path options | | 🟡 | M42 |
| Emphasis animations | — | ⛔ | M42 |
| Animation painter | — | ⛔ | M42 |
| Sequence by paragraph (build steps) | — | ⛔ | M42 |
| Transition: apply to all | `slide.set_transition_all`, Transitions › Apply To All | ✅ | — |
| Timeline: fit to longest, reset, align to playhead/cue | — | ⛔ | M42 |
| Timing dialog | — | ⛔ | M42 |
| Camera zoom | — | ⛔ | M42 |
| C key adds a cue point while playing | | ⛔ verify | M42 |
| Timeline row context menu complete | | ⛔ | M42 |

## Questions and quizzes

| Feature | CourseCraft | Status | Milestone |
|---|---|---|---|
| Graded question kinds (MC, MR, T/F, fill-in, word bank, matching, sequence, numeric, hotspot, drag and drop) | `question.*` (MC, MR, T/F, fill-in, matching, sequence, hotspot, drag and drop built; word bank and numeric not yet) | 🟡 | M41 |
| Survey questions | `question.insert` | ✅ | — |
| Custom questions | `question.set_kind` | ✅ | — |
| Feedback layers (correct, incorrect, try again) | `question.add_feedback_layers` | ✅ | — |
| Shuffle, attempts | `question.set` | ✅ | — |
| Points | `question.set` | ✅ | — |
| Question banks, draw from bank | `bank.*` | ✅ | — |
| Results slide | `results.insert` | ✅ | — |
| Certificate slide | `certificate.insert` | ✅ (CourseCraft extra) | — |
| Feedback by choice | | 🟡 verify | M41 |
| Scoring by choice | | 🟡 | M41 |
| Insert Question dialog with filters and preview | new-slide popup | 🟡 | M38 |
| Question form view (grid editor) | inspector | ⛔ | M41 |
| Feedback layouts | — | ⛔ | M41 |
| Media in question form (per choice / feedback) | — | ⛔ | M41 |

## Player, preview, publish

| Feature | CourseCraft | Status | Milestone |
|---|---|---|---|
| Player: menu, resources, glossary, notes tabs | Course settings | ✅ | — |
| Player: title, logo, sidebar, seekbar modes, controls | Course settings | ✅ | — |
| Player colours light/dark, accent from theme | Course settings | ✅ | — |
| Navigation modes (free, restricted, locked) | `course.set_player` | ✅ | — |
| Preview slide, scene, selected slides, course | preview popup | ✅ | — |
| Publish web | `publish.rs` | ✅ | — |
| Publish LMS: SCORM 1.2, SCORM 2004, cmi5, xAPI, LTI | `publish.rs` | ✅ | — |
| Tracking by slides viewed / quiz | `course.set_tracking` (Course settings & Publish dialog); SCORM 1.2 Report status as offers Automatic (quiz result wins) and Automatic (completion stays) | ✅ | — |
| Publish video (MP4), Word, PDF | Export | ✅ | — |
| Notes as learner transcript | player | ✅ | — |
| Text labels (edit the player's own strings) | — | ⛔ verify | M43 |
| Player Properties dialog with its own ribbon and live preview | Course settings | 🟡 | M38 |
| Publish dialog: list on the left, property links | `publish.rs` | 🟡 | M38 |
| Preview: Replay slide / scene / course | | 🟡 | M43 |
| Save player / import player | | 🟡 verify | M43 |
| Preview device sizes (desktop, tablet, phone; both orientations) | — | ⛔ | M43 |
| Preview: Edit Slide | — | ⛔ | M43 |
| Preview: Inspect (variables, trigger log) | — | ⛔ | M43 |
| Swipe in player | — | ⛔ | M41 |
| Publish quality setting | — | ⛔ | M43 |
| Report to external LRS alongside LMS | | ⛔ verify | M43 |
| Share for review with comments | — | 🚫 | — |
| Hosted cloud delivery | — | — | — |

## Editor shell and panels

| Feature | CourseCraft | Status | Milestone |
|---|---|---|---|
| Undo / redo with history | `edit.history` | ✅ | — |
| Keyboard shortcuts sheet | `shortcuts.rs` | ✅ | — |
| Light / dark editor | View › Theme | ✅ | — |
| Scenes panel with scene combo | `slides.rs` | ✅ | — |
| Triggers panel | `triggers.rs` | ✅ | — |
| Slide layers panel | `layers.rs`; docked, a panel of its own to the right of the Timeline's group | ✅ | — |
| Timeline, States, Notes tabs | `timeline.rs`, `states.rs`; docked, Timeline, States, Triggers, Variables, Question and Notes are dock panels of their own, tabbed together by default (`dock.rs`); with the Notes panel open the Slide inspector leaves the notes to it | ✅ | — |
| Canvas zoom and fit | | ✅ | — |
| Tooltips with name, shortcut and help | | 🟡 | M38 |
| Status bar: slide n of m, size, theme, zoom | `status.rs` | 🟡 | M38 |
| Scene combo: mouse wheel switches scenes | | 🟡 | M38 |
| Right-click menus complete (canvas, timeline, scenes) | | 🟡 | M38 |
| Collapse panels | | 🟡 | M38 |
| Slide Properties in the Course Map | | 🟡 | M38 |
| **Tabbed ribbon** with contextual tabs | on by default (`ribbon/`): every main tab and the File page, replacing the menu bar, and the contextual tabs (Drawing, Button, Picture, Video, Audio, Table, Question Tools; `ribbon/contextual.rs`) and the theme, transition and animation galleries (`ribbon/gallery.rs`), and the Slide Master tab in Master View; the other modes' tab strips to come | 🟡 | M38 |
| Settings panels from the ↘ launchers, opening by themselves for the work in hand | `panels.rs` (`docs/ui/panels.md`): Text; the other categories to come | 🟡 | M38–M43 |
| Quick access toolbar | Save, Undo, Redo, Undo History left of the tab strip; Customize to come | 🟡 | M38 |
| Collapse ribbon | click or double-click the open tab, Ctrl+F1 | ✅ | — |
| View tabs for open slides | `doc_tabs.rs`: a Course Map tab, then a tab per opened slide (× closes it), under the ribbon | ✅ | — |
| Undock / float / redock panels | docked by default (`dock.rs`, craft-ui docking): move, tab, float, close and reopen (View › Panels), the four inspectors included; a floating panel can leave the app window as its own OS window (desktop, not Wayland); a panel dropped on the window's outer edge takes the whole width (bottom, top) or height (left, right); the arrangement is saved in the preferences; Preferences › Interface turns it off | ✅ | — |
| Reset panel layout | View › Panels › Reset Panel Layout | ✅ | — |
| Slide-flow arrows in the Scenes panel | — | ⛔ | M38 |
| Comments tab | — | ⛔ needs review workflow | — |

## AI

| Feature | CourseCraft | Status | Milestone |
|---|---|---|---|
| Narration from text | `audio_ai.rs` | ✅ | — |
| Alt text suggestions | `alt_text.rs` | ✅ | — |
| Translate course | `translate_ai.rs` | ✅ | — |
| Course from document | File › Import | 🟡 outline, not a quiz | M43 |
| Generate quiz from content | — | ⛔ | M43 |
| Generate one question | — | ⛔ | M43 |
| Summary slide | — | ⛔ | M43 |
| Insert / edit text with AI | — | ⛔ | M43 |
| Generate image | — | ⛔ | M43 |
| AI panel (chat with the connected agent) | — | ⛔ | M43 |

## Assets and services

| Feature | CourseCraft | Status | Milestone |
|---|---|---|---|
| Icon set | — | ⛔ an open-licence bundled set | M40 |
| Characters | — | 🚫 | — |
| Stock photos, illustrations, videos | — | 🚫 | — |
| Stock slide template library | — | 🚫 | — |
| 360° images and editor | — | 🚫 | — |
| Screen recording, software simulation | — | 🚫 patent watch list | — |
| Hosted review / comments | — | 🚫 | — |
| Shared cloud slides / hosted delivery | — | — | — |
