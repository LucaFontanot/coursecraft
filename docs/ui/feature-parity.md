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
| Text | 13 | 3 | 8 | 0 |
| Shapes, pictures and buttons | 14 | 5 | 10 | 0 |
| Media | 10 | 3 | 5 | 1 |
| Interactivity | 12 | 3 | 2 | 0 |
| Animation, transitions, timeline | 10 | 3 | 9 | 0 |
| Questions and quizzes | 9 | 3 | 3 | 0 |
| Player, preview, publish | 10 | 4 | 7 | 2 |
| Editor shell and panels | 12 | 9 | 2 | 0 |
| AI | 3 | 1 | 6 | 0 |
| Assets and services | 0 | 0 | 1 | 7 |
| **Total** | **108** | **39** | **61** | **12** |

Counted from the tables below; recount when a row changes.

## Project and file

| Feature | CourseCraft | Status | Milestone |
|---|---|---|---|
| New, Open, Save, Save As | File menu | ✅ | — |
| Recent projects | File › Open Recent (names) | 🟡 thumbnails missing | M37 |
| Start screen / File page with start actions and Recent | File page from the File tab and on launch (`file_page.rs`, its Start section is the welcome screen); thumbnails in Recent and news to come | 🟡 thumbnails | M37 |
| Import PowerPoint | File › Import | ✅ | — |
| Import questions (CSV / spreadsheet) | File › Import | ✅ | — |
| Translation export / import (XLIFF) | File › Translation | ✅ | — |
| Version history, autosave and recovery | `version_history.rs` | ✅ | — |
| Close project (without quitting) | — | ⛔ | M37 |
| Slide size and default size | Slide Size dialog | ✅ | — |
| Project info (title, author, description) | Course settings | ✅ | — |
| Team Slides (shared cloud library) | — | — | — |
| Record Screen project | — | 🚫 patent watch list | — |

## Structure: scenes, slides, masters

| Feature | CourseCraft | Status | Milestone |
|---|---|---|---|
| Scenes; new scene; rename | `scene.*` | ✅ | — |
| Starting scene (red flag) | — | ⛔ needs model field + runtime rule | M40 |
| Story view (Course Map): cards, zoom, fit | `course_map.rs` | ✅ | — |
| Course Map: collapse scene cards, branch lines between scenes | — | ⛔ | M41 |
| New slide from layout, Basic Layouts gallery | `slide.add_from_layout` | ✅ | — |
| My Templates | `template.*` | ✅ | — |
| Duplicate, delete, move slides | `slide.*` | ✅ | — |
| Apply layout; reset slide to layout | `slide.set_master` | 🟡 reset ⛔ | M40 |
| Slide masters and layouts (insert, rename, delete) | `master.*` | ✅ | — |
| Master: Preserve, Title checkbox, Insert Placeholder | — | ⛔ | M40 |
| Hide background graphics | — | ⛔ | M40 |
| Feedback master | — | ⛔ | M40 |
| Slide layers (new, duplicate, delete, dim, properties) | `layer.*` | ✅ | — |
| Slide properties: advance, revisit, prev/next/submit | `slide.update`, `slide.set_player_overrides` | ✅ | — |
| Swipe gestures (prev/next) | — | ⛔ | M40 |
| Scene index menu | — | ⛔ behaviour to confirm | M40 |
| Focus order | audit only | 🟡 dialog missing | M38 |
| Themes: colours, fonts, gallery | `course.set_theme`, `theme.*` | 🟡 theme-font dialog | M38 |

## Text

| Feature | CourseCraft | Status | Milestone |
|---|---|---|---|
| Text boxes, placeholders, autofit | `object.add`, `object.set_autofit` | ✅ | — |
| Font family and size, grow/shrink | `text.style_range`; Home › Font through `text_format.rs` | ✅ | — |
| Bold, italic, underline, strikethrough | `text.style_range` | ✅ | — |
| Superscript, subscript | `text.style_range` | ✅ | — |
| Character spacing | `letter_spacing` | ✅ | — |
| Font colour with theme colours and eyedropper | `eyedropper.rs` | ✅ | — |
| Text styles | `text_style.*` | ✅ | — |
| Bullets, numbering, indent, line spacing, alignment | `text.set_paragraph(s)`; Home › Paragraph through `text_format.rs` | 🟡 numbering styles | M38 |
| Find and replace | `course.find`, `course.replace` | ✅ | — |
| Spelling | `spell_check.rs` | ✅ | — |
| Hyperlinks | `text.set_link` | ✅ | — |
| Format painter | `format_painter.rs` | ✅ | — |
| Theme fonts (heading + body) | Theme dialog | ✅ | — |
| Paste options (destination theme, keep source, text only) | `object.paste` | 🟡 one mode | M38 |
| Vertical text alignment | `text.set_paragraph` `{valign}`: Home › Paragraph › Align Text, inspector | ✅ | — |
| Clear formatting | — | ⛔ | M38 |
| Text shadow | — | ⛔ | M38 |
| Change case | — | ⛔ | M38 |
| Highlight colour | — | ⛔ needs model field | M38 |
| Text direction (rotate, stacked) | — | ⛔ | M38 |
| Font and Paragraph settings (Storyline's dialogs) | Text settings panel (`panels.rs`, from Font ↘ and Paragraph ↘): every text control, space before and after; caps, highlight, underline styles, tab stops and columns drawn disabled | 🟡 | M38 |
| Slide number field | — | ⛔ | M38 |
| Symbol picker | — | ⛔ | M38 |
| Insert variable reference | — | ⛔ variables exist | M38 |

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
| Smart guides, grid, guides, rulers | `snap.rs`, `rulers.rs`; the Grid and Guides panel (`grid_panel.rs`) sets the grid spacing and lists the guides | ✅ | — |
| Lock and hide in editor | `object.lock`, `object.hide_in_editor` | ✅ | — |
| Pictures: insert, replace, crop | `object.set_crop`, `crop.rs` | ✅ | — |
| Picture border | `object.update` | ✅ | — |
| States (built-in and custom, items inside states) | M29 | ✅ | — |
| Recolor | `course.recolor` | 🟡 SVG only | M39 |
| Reset picture | | 🟡 | M39 |
| Picture effects | | 🟡 as shape effects | M39 |
| Button styles, fill, border, effects | | 🟡 | M39 |
| When Clicked Action on a button | trigger editor | 🟡 not pre-filled | M39 |
| Reflection, soft edges | — | ⛔ | M39 |
| Change shape (keep format) | — | ⛔ | M39 |
| Size and Position dialog | the Size and Position panel (`size_panel.rs`): width and height with an aspect lock, rotation, flips, position from the top left or the centre, lock | ✅ | — |
| Brightness, contrast | — | ⛔ | M39 |
| Picture styles (frames, bevels) | — | ⛔ | M39 |
| Crop to shape | — | ⛔ | M39 |
| Zoom picture (learner enlarges) | — | ⛔ | M39 |
| Button icons (gallery, align, colour, remove) | — | ⛔ | M39 |
| Button sets | — | ⛔ | M39 |
| Caption / callout shapes | — | ⛔ check gallery | M39 |
| Export picture | — | ⛔ | M39 |

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
| Audio / video editor | trim only | 🟡 | M39 |
| Closed captions (add, import, export, delete) | `object.set_captions` | 🟡 | M39 |
| AI audio (sound effects, music) | — | ⛔ | M42 |
| Audio options dialog | — | 🟡 | M39 |
| Export audio / video | — | ⛔ | M39 |
| Compression setting | — | ⛔ | M39 |
| Show video in a new window | — | ⛔ | M39 |
| Record webcam | — | ⛔ | M39 |
| 360° images | — | 🚫 | — |

## Interactivity

| Feature | CourseCraft | Status | Milestone |
|---|---|---|---|
| Triggers: actions, events, conditions | `trigger.*` | ✅ | — |
| Object / slide / layer / player trigger sections | `triggers.rs` | ✅ | — |
| Variables | `variable.*` | ✅ | — |
| Buttons, hotspots, markers | `object.add`, `object.set_marker`; no hotspot object yet (hotspot questions and the Hotspot reveal interaction only) | 🟡 | M37 |
| Sliders, dials | `Slider`, `Dial` | ✅ | — |
| Text entry, checkbox, radio buttons | `object.set_input` | ✅ | — |
| Drag and drop | `object.set_drag` | ✅ | — |
| Scrolling panel | `object.set_scroll_panel` | ✅ | — |
| Tables | `table.*` | ✅ | — |
| Interactions gallery (tabs, process, flip cards…) | `interaction.insert` | ✅ (CourseCraft extra) | — |
| Timer, meter, motion path | `object.set_timer`, `object.set_meter`, `object.add_motion_path` | ✅ | — |
| Glossary, resources | `glossary.*`, `resource.*` | ✅ | — |
| Copy and paste triggers | `triggers.rs` | ✅ | — |
| Trigger dialog (Action / When / Conditions form) | inline editor | 🟡 | M37 |
| Built-in states for every input | M29 | 🟡 check the 5 button states | M39 |
| Custom mouse cursor | — | ⛔ | M40 |
| Convert slide to custom question | — | ⛔ | M40 |

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
| Full entrance gallery (16 effects) | | 🟡 subset | M41 |
| Transition effect options | | 🟡 | M41 |
| Path options | | 🟡 | M41 |
| Emphasis animations | — | ⛔ | M41 |
| Animation painter | — | ⛔ | M41 |
| Sequence by paragraph (build steps) | — | ⛔ | M41 |
| Transition: apply to all | — | ⛔ | M41 |
| Timeline: fit to longest, reset, align to playhead/cue | — | ⛔ | M41 |
| Timing dialog | — | ⛔ | M41 |
| Zoom region (camera zoom) | — | ⛔ | M41 |
| C key adds a cue point while playing | | ⛔ verify | M41 |
| Timeline row context menu complete | | ⛔ | M41 |

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
| Feedback by choice | | 🟡 verify | M40 |
| Scoring by choice | | 🟡 | M40 |
| Insert Question dialog with filters and preview | new-slide popup | 🟡 | M37 |
| Question form view (grid editor) | inspector | ⛔ | M40 |
| Feedback master | — | ⛔ | M40 |
| Media in question form (per choice / feedback) | — | ⛔ | M40 |

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
| Text labels (edit the player's own strings) | — | ⛔ verify | M42 |
| Player Properties dialog with its own ribbon and live preview | Course settings | 🟡 | M37 |
| Publish dialog: list on the left, property links | `publish.rs` | 🟡 | M37 |
| Preview: Replay slide / scene / course | | 🟡 | M42 |
| Save player / import player | | 🟡 verify | M42 |
| Preview device sizes (desktop, tablet, phone; both orientations) | — | ⛔ | M42 |
| Preview: Edit Slide | — | ⛔ | M42 |
| Preview: Inspect (variables, trigger log) | — | ⛔ | M42 |
| Swipe in player | — | ⛔ | M40 |
| Publish quality setting | — | ⛔ | M42 |
| Report to external LRS alongside LMS | | ⛔ verify | M42 |
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
| Slide layers panel | `layers.rs`; docked, a panel of its own to the right of the Timeline's group | ✅ | — |
| Timeline, States, Notes tabs | `timeline.rs`, `states.rs`; docked, Timeline, States, Triggers, Variables, Question and Notes are dock panels of their own, tabbed together by default (`dock.rs`); with the Notes panel open the Slide inspector leaves the notes to it | ✅ | — |
| Canvas zoom and fit | | ✅ | — |
| Tooltips with name, shortcut and help | | 🟡 | M37 |
| Status bar: slide n of m, size, theme, zoom | `status.rs` | 🟡 | M37 |
| Scene combo: mouse wheel switches scenes | | 🟡 | M37 |
| Right-click menus complete (canvas, timeline, scenes) | | 🟡 | M37 |
| Collapse panels | | 🟡 | M37 |
| Slide Properties in the Course Map | | 🟡 | M37 |
| **Tabbed ribbon** with contextual tabs | on by default (`ribbon/`): every main tab and the File page, replacing the menu bar, and the contextual tabs (Drawing, Button, Picture, Video, Audio, Table, Question Tools; `ribbon/contextual.rs`); the galleries (themes, transitions, animations) and the modes' tab strips to come | 🟡 | M37 |
| Settings panels from the ↘ launchers, opening by themselves for the work in hand | `panels.rs` (`docs/ui/panels.md`): Text; the other categories to come | 🟡 | M37–M42 |
| Quick access toolbar | Save, Undo, Redo, Undo History left of the tab strip; Customize to come | 🟡 | M37 |
| Collapse ribbon | click or double-click the open tab, Ctrl+F1 | ✅ | — |
| View tabs for open slides | `doc_tabs.rs`: a Course Map tab, then a tab per opened slide (× closes it), under the ribbon | ✅ | — |
| Undock / float / redock panels | docked by default (`dock.rs`, craft-ui docking): move, tab, float, close and reopen (View › Panels), the four inspectors included; a floating panel can leave the app window as its own OS window (desktop, not Wayland); the arrangement is saved in the preferences; Preferences › Interface turns it off | ✅ | — |
| Reset panel layout | View › Panels › Reset Panel Layout | ✅ | — |
| Slide-flow arrows in the Scenes panel | — | ⛔ | M37 |
| Comments tab | — | ⛔ needs review workflow | — |

## AI

| Feature | CourseCraft | Status | Milestone |
|---|---|---|---|
| Narration from text | `audio_ai.rs` | ✅ | — |
| Alt text suggestions | `alt_text.rs` | ✅ | — |
| Translate course | `translate_ai.rs` | ✅ | — |
| Course from document | File › Import | 🟡 outline, not a quiz | M42 |
| Generate quiz from content | — | ⛔ | M42 |
| Generate one question | — | ⛔ | M42 |
| Summary slide | — | ⛔ | M42 |
| Insert / edit text with AI | — | ⛔ | M42 |
| Generate image | — | ⛔ | M42 |
| AI panel (chat with the connected agent) | — | ⛔ | M42 |

## Assets and services

| Feature | CourseCraft | Status | Milestone |
|---|---|---|---|
| Icon set | — | ⛔ an open-licence bundled set | M39 |
| Characters | — | 🚫 | — |
| Stock photos, illustrations, videos | — | 🚫 | — |
| Content library slide templates | — | 🚫 | — |
| 360° images and editor | — | 🚫 | — |
| Screen recording, software simulation | — | 🚫 patent watch list | — |
| Review 360 / comments | — | 🚫 | — |
| Team slides, Reach 360 | — | — | — |
