# Feature parity with Storyline 360

> Part of [the ribbon plan](README.md). **Written:** 2026-11-23. One row per **feature** a
> Storyline author reaches through the UI seen in the tutorials (README › Sources), whatever tab
> or dialog it sits in. The control-by-control view is in [ribbon.md](ribbon.md) and
> [contextual-tabs.md](contextual-tabs.md); this file is the summary that drives the roadmap.
>
> Status marks as in README (✅ built, 🟡 partly, ⛔ not yet, 🚫 not planned, — left out).
> **Milestone** is where the work is planned in `docs/roadmap.md`. A row changes in the same commit
> as the code that changes it.

## Summary

| Area | ✅ | 🟡 | ⛔ | 🚫 / — |
|---|---|---|---|---|
| Project and file | 7 | 2 | 1 | 2 |
| Structure: scenes, slides, masters | 8 | 3 | 7 | 0 |
| Text | 12 | 3 | 9 | 0 |
| Shapes, pictures and buttons | 13 | 5 | 11 | 0 |
| Media | 10 | 3 | 5 | 1 |
| Interactivity | 12 | 3 | 2 | 0 |
| Animation, transitions, timeline | 10 | 3 | 9 | 0 |
| Questions and quizzes | 9 | 3 | 3 | 0 |
| Player, preview, publish | 10 | 4 | 7 | 2 |
| Editor shell and panels | 12 | 8 | 2 | 0 |
| AI | 3 | 1 | 6 | 0 |
| Assets and services | 0 | 0 | 1 | 7 |
| **Total** | **106** | **38** | **63** | **12** |

Counted from the tables below; recount when a row changes.

## Project and file

| Feature | CourseCraft | Status | Milestone |
|---|---|---|---|
| New, Open, Save, Save As | File menu | ✅ | — |
| Recent projects | File › Open Recent (names) | 🟡 thumbnails missing | M30 |
| Start screen / File page with start actions and Recent | File page from the File tab (`file_page.rs`), welcome screen on launch; one page with thumbnails and news to come | 🟡 layout | M30 |
| Import PowerPoint | File › Import | ✅ | — |
| Import questions (CSV / spreadsheet) | File › Import | ✅ | — |
| Translation export / import (XLIFF) | File › Translation | ✅ | — |
| Version history, autosave and recovery | `version_history.rs` | ✅ | — |
| Close project (without quitting) | — | ⛔ | M30 |
| Slide size and default size | Slide Size dialog | ✅ | — |
| Project info (title, author, description) | Course settings | ✅ | — |
| Team Slides (shared cloud library) | — | — | — |
| Record Screen project | — | 🚫 patent watch list | — |

## Structure: scenes, slides, masters

| Feature | CourseCraft | Status | Milestone |
|---|---|---|---|
| Scenes; new scene; rename | `scene.*` | ✅ | — |
| Starting scene (red flag) | — | ⛔ needs model field + runtime rule | M33 |
| Story view (Course Map): cards, zoom, fit | `course_map.rs` | ✅ | — |
| Course Map: collapse scene cards, branch lines between scenes | — | ⛔ | M34 |
| New slide from layout, Basic Layouts gallery | `slide.add_from_layout` | ✅ | — |
| My Templates | `template.*` | ✅ | — |
| Duplicate, delete, move slides | `slide.*` | ✅ | — |
| Apply layout; reset slide to layout | `slide.set_master` | 🟡 reset ⛔ | M33 |
| Slide masters and layouts (insert, rename, delete) | `master.*` | ✅ | — |
| Master: Preserve, Title checkbox, Insert Placeholder | — | ⛔ | M33 |
| Hide background graphics | — | ⛔ | M33 |
| Feedback master | — | ⛔ | M33 |
| Slide layers (new, duplicate, delete, dim, properties) | `layer.*` | ✅ | — |
| Slide properties: advance, revisit, prev/next/submit | `slide.update`, `slide.set_player_overrides` | ✅ | — |
| Swipe gestures (prev/next) | — | ⛔ | M33 |
| Scene index menu | — | ⛔ behaviour to confirm | M33 |
| Focus order | audit only | 🟡 dialog missing | M31 |
| Themes: colours, fonts, gallery | `course.set_theme`, `theme.*` | 🟡 theme-font dialog | M31 |

## Text

| Feature | CourseCraft | Status | Milestone |
|---|---|---|---|
| Text boxes, placeholders, autofit | `object.add`, `object.set_autofit` | ✅ | — |
| Font family and size, grow/shrink | `text.style_range` | ✅ | — |
| Bold, italic, underline, strikethrough | `text.style_range` | ✅ | — |
| Superscript, subscript | `text.style_range` | ✅ | — |
| Character spacing | `letter_spacing` | ✅ | — |
| Font colour with theme colours and eyedropper | `eyedropper.rs` | ✅ | — |
| Text styles | `text_style.*` | ✅ | — |
| Bullets, numbering, indent, line spacing, alignment | `text.set_paragraph` | 🟡 numbering styles | M31 |
| Find and replace | `course.find`, `course.replace` | ✅ | — |
| Spelling | `spell_check.rs` | ✅ | — |
| Hyperlinks | `text.set_link` | ✅ | — |
| Format painter | `format_painter.rs` | ✅ | — |
| Theme fonts (heading + body) | Theme dialog | ✅ | — |
| Paste options (destination theme, keep source, text only) | `object.paste` | 🟡 one mode | M31 |
| Vertical text alignment | inspector | 🟡 | M31 |
| Clear formatting | — | ⛔ | M31 |
| Text shadow | — | ⛔ | M31 |
| Change case | — | ⛔ | M31 |
| Highlight colour | — | ⛔ needs model field | M31 |
| Text direction (rotate, stacked) | — | ⛔ | M31 |
| Font and Paragraph dialogs | — | ⛔ | M31 |
| Slide number field | — | ⛔ | M31 |
| Symbol picker | — | ⛔ | M31 |
| Insert variable reference | — | ⛔ variables exist | M31 |

## Shapes, pictures and buttons

| Feature | CourseCraft | Status | Milestone |
|---|---|---|---|
| Shape gallery, edit points | `shape_gallery.rs` | ✅ | — |
| Shape fill (solid, gradient, picture) | `object.set_fill_style` | ✅ | — |
| Outline, joins | `object.update` | ✅ | — |
| Quick styles / shape styles | `object.apply_style` | ✅ | — |
| Shadow, glow | `object.set_shadow`, `object.set_glow` | ✅ | — |
| Arrange, group, align, distribute, match size | `object.*` | ✅ | — |
| Rotate, flip | `object.set_rotation`, `object.flip` | ✅ | — |
| X, Y, width, height | `object.update` | ✅ | — |
| Smart guides, grid, guides, rulers | `snap.rs`, `rulers.rs` | ✅ | — |
| Lock and hide in editor | `object.lock`, `object.hide_in_editor` | ✅ | — |
| Pictures: insert, replace, crop | `object.set_crop`, `crop.rs` | ✅ | — |
| Picture border | `object.update` | ✅ | — |
| States (built-in and custom, items inside states) | M29 | ✅ | — |
| Recolor | `course.recolor` | 🟡 SVG only | M32 |
| Reset picture | | 🟡 | M32 |
| Picture effects | | 🟡 as shape effects | M32 |
| Button styles, fill, border, effects | | 🟡 | M32 |
| When Clicked Action on a button | trigger editor | 🟡 not pre-filled | M32 |
| Reflection, soft edges | — | ⛔ | M32 |
| Change shape (keep format) | — | ⛔ | M32 |
| Size and Position dialog | inspector | ⛔ dialog | M31 |
| Brightness, contrast | — | ⛔ | M32 |
| Picture styles (frames, bevels) | — | ⛔ | M32 |
| Crop to shape | — | ⛔ | M32 |
| Zoom picture (learner enlarges) | — | ⛔ | M32 |
| Button icons (gallery, align, colour, remove) | — | ⛔ | M32 |
| Button sets | — | ⛔ | M32 |
| Caption / callout shapes | — | ⛔ check gallery | M32 |
| Export picture | — | ⛔ | M32 |

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
| Audio / video editor | trim only | 🟡 | M32 |
| Closed captions (add, import, export, delete) | `object.set_captions` | 🟡 | M32 |
| AI audio (sound effects, music) | — | ⛔ | M35 |
| Audio options dialog | — | 🟡 | M32 |
| Export audio / video | — | ⛔ | M32 |
| Compression setting | — | ⛔ | M32 |
| Show video in a new window | — | ⛔ | M32 |
| Record webcam | — | ⛔ | M32 |
| 360° images | — | 🚫 | — |

## Interactivity

| Feature | CourseCraft | Status | Milestone |
|---|---|---|---|
| Triggers: actions, events, conditions | `trigger.*` | ✅ | — |
| Object / slide / layer / player trigger sections | `triggers.rs` | ✅ | — |
| Variables | `variable.*` | ✅ | — |
| Buttons, hotspots, markers | `object.add`, `object.set_marker`; no hotspot object yet (hotspot questions and the Hotspot reveal interaction only) | 🟡 | M30 |
| Sliders, dials | `Slider`, `Dial` | ✅ | — |
| Text entry, checkbox, radio buttons | `object.set_input` | ✅ | — |
| Drag and drop | `object.set_drag` | ✅ | — |
| Scrolling panel | `object.set_scroll_panel` | ✅ | — |
| Tables | `table.*` | ✅ | — |
| Interactions gallery (tabs, process, flip cards…) | `interaction.insert` | ✅ (CourseCraft extra) | — |
| Timer, meter, motion path | `object.set_timer`, `object.set_meter`, `object.add_motion_path` | ✅ | — |
| Glossary, resources | `glossary.*`, `resource.*` | ✅ | — |
| Copy and paste triggers | `triggers.rs` | ✅ | — |
| Trigger dialog (Action / When / Conditions form) | inline editor | 🟡 | M30 |
| Built-in states for every input | M29 | 🟡 check the 5 button states | M32 |
| Custom mouse cursor | — | ⛔ | M33 |
| Convert slide to custom question | — | ⛔ | M33 |

## Animation, transitions, timeline

| Feature | CourseCraft | Status | Milestone |
|---|---|---|---|
| Entrance and exit animations | `object.set_animation` | ✅ | — |
| Animation duration and direction | `object.set_animation` | ✅ | — |
| Motion paths (add, duration, name, options) | `object.*motion_path` | ✅ | — |
| Slide transitions and duration | `slide.set_transition` | ✅ | — |
| Timeline: bars, drag timing, rename, restack | `timeline.rs` | ✅ | — |
| Show until end / show always | `timeline.rs` | ✅ | — |
| Cue points, snapping | `slide.add_cue` | ✅ | — |
| Slide duration | `slide.set_duration` | ✅ | — |
| Play / stop / zoom in timeline | `timeline_playback.rs` | ✅ | — |
| Timeline shows current layer; group rows expand | `timeline.rs` | ✅ | — |
| Full entrance gallery (16 effects) | | 🟡 subset | M34 |
| Transition effect options | | 🟡 | M34 |
| Path options | | 🟡 | M34 |
| Emphasis animations | — | ⛔ | M34 |
| Animation painter | — | ⛔ | M34 |
| Sequence by paragraph (build steps) | — | ⛔ | M34 |
| Transition: apply to all | — | ⛔ | M34 |
| Timeline: fit to longest, reset, align to playhead/cue | — | ⛔ | M34 |
| Timing dialog | — | ⛔ | M34 |
| Zoom region (camera zoom) | — | ⛔ | M34 |
| C key adds a cue point while playing | | ⛔ verify | M34 |
| Timeline row context menu complete | | ⛔ | M34 |

## Questions and quizzes

| Feature | CourseCraft | Status | Milestone |
|---|---|---|---|
| Graded question kinds (MC, MR, T/F, fill-in, word bank, matching, sequence, numeric, hotspot) | `question.*` | ✅ | — |
| Survey questions | `question.insert` | ✅ | — |
| Custom (freeform) questions | `question.set_kind` | ✅ | — |
| Feedback layers (correct, incorrect, try again) | `question.add_feedback_layers` | ✅ | — |
| Shuffle, attempts | `question.set` | ✅ | — |
| Points | `question.set` | ✅ | — |
| Question banks, draw from bank | `bank.*` | ✅ | — |
| Results slide | `results.insert` | ✅ | — |
| Certificate slide | `certificate.insert` | ✅ (CourseCraft extra) | — |
| Feedback by choice | | 🟡 verify | M33 |
| Scoring by choice | | 🟡 | M33 |
| Insert Question dialog with filters and preview | new-slide popup | 🟡 | M30 |
| Question form view (grid editor) | inspector | ⛔ | M33 |
| Feedback master | — | ⛔ | M33 |
| Media in question form (per choice / feedback) | — | ⛔ | M33 |

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
| Tracking by slides viewed / quiz | `course.set_publish_settings` | ✅ | — |
| Publish video (MP4), Word, PDF | Export | ✅ | — |
| Notes as learner transcript | player | ✅ | — |
| Text labels (edit the player's own strings) | — | ⛔ verify | M35 |
| Player Properties dialog with its own ribbon and live preview | Course settings | 🟡 | M30 |
| Publish dialog: list on the left, property links | `publish.rs` | 🟡 | M30 |
| Preview: Replay slide / scene / course | | 🟡 | M35 |
| Save player / import player | | 🟡 verify | M35 |
| Preview device sizes (desktop, tablet, phone; both orientations) | — | ⛔ | M35 |
| Preview: Edit Slide | — | ⛔ | M35 |
| Preview: Inspect (variables, trigger log) | — | ⛔ | M35 |
| Swipe in player | — | ⛔ | M33 |
| Publish quality setting | — | ⛔ | M35 |
| Report to external LRS alongside LMS | | ⛔ verify | M35 |
| Share for review with comments | — | 🚫 | — |
| Hosted delivery (Reach 360) | — | — | — |

## Editor shell and panels

| Feature | CourseCraft | Status | Milestone |
|---|---|---|---|
| Undo / redo with history | `edit.history` | ✅ | — |
| Keyboard shortcuts sheet | `shortcuts.rs` | ✅ | — |
| Light / dark editor | View › Theme | ✅ | — |
| Scenes panel with scene combo | `slides.rs` | ✅ | — |
| Triggers panel | `triggers.rs` | ✅ | — |
| Slide layers panel | `layers.rs` | ✅ | — |
| Timeline, States, Notes tabs | `timeline.rs`, `states.rs` | ✅ | — |
| Canvas zoom and fit | | ✅ | — |
| Tooltips with name, shortcut and help | | 🟡 | M30 |
| Status bar: slide n of m, size, theme, zoom | `status.rs` | 🟡 | M30 |
| Scene combo: mouse wheel switches scenes | | 🟡 | M30 |
| Right-click menus complete (canvas, timeline, scenes) | | 🟡 | M30 |
| Collapse panels | | 🟡 | M30 |
| Slide Properties in the Course Map | | 🟡 | M30 |
| **Tabbed ribbon** with contextual tabs | on by default (`ribbon/`): every main tab and the File page, replacing the menu bar; the galleries (themes, transitions, animations) and the contextual tabs to come | 🟡 | M30 |
| Quick access toolbar | Save, Undo, Redo, Undo History left of the tab strip; Customize to come | 🟡 | M30 |
| Collapse ribbon | click or double-click the open tab, Ctrl+F1 | ✅ | — |
| View tabs for open slides | `doc_tabs.rs`: a Course Map tab, then a tab per opened slide (× closes it), under the ribbon | ✅ | — |
| Undock / float / redock panels | docked by default (`dock.rs`, craft-ui docking): move, tab, float, close and reopen (View › Panels), the four inspectors included; the arrangement is saved in the preferences; Preferences › Interface turns it off | ✅ | — |
| Reset panel layout | View › Panels › Reset Panel Layout | ✅ | — |
| Slide-flow arrows in the Scenes panel | — | ⛔ | M30 |
| Comments tab | — | ⛔ needs review workflow | — |

## AI

| Feature | CourseCraft | Status | Milestone |
|---|---|---|---|
| Narration from text | `audio_ai.rs` | ✅ | — |
| Alt text suggestions | `alt_text.rs` | ✅ | — |
| Translate course | `translate_ai.rs` | ✅ | — |
| Course from document | File › Import | 🟡 outline, not a quiz | M35 |
| Generate quiz from content | — | ⛔ | M35 |
| Generate one question | — | ⛔ | M35 |
| Summary slide | — | ⛔ | M35 |
| Insert / edit text with AI | — | ⛔ | M35 |
| Generate image | — | ⛔ | M35 |
| AI panel (chat with the connected agent) | — | ⛔ | M35 |

## Assets and services

| Feature | CourseCraft | Status | Milestone |
|---|---|---|---|
| Icon set | — | ⛔ an open-licence bundled set | M32 |
| Characters | — | 🚫 | — |
| Stock photos, illustrations, videos | — | 🚫 | — |
| Content library slide templates | — | 🚫 | — |
| 360° images and editor | — | 🚫 | — |
| Screen recording, software simulation | — | 🚫 patent watch list | — |
| Review 360 / comments | — | 🚫 | — |
| Team slides, Reach 360 | — | — | — |
