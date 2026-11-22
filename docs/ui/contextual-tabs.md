# Ribbon: contextual tabs and modes

> Part of [the ribbon plan](README.md). Kinds, actions and status marks as in [ribbon.md](ribbon.md).
> Reference: [storyline-observed.md](storyline-observed.md) §3.

## When each one shows

Contextual tabs sit right of Help under a coloured caption. Selecting the object does **not**
switch to its tab (Storyline stays on the current tab; the tutorial clicks Format itself), except
when the object is just inserted.

| Caption | Tab | Shown when the selection contains… |
|---|---|---|
| Drawing Tools | **Format** | a shape, text box, hotspot, marker, slider, dial, input, or text being edited |
| Button Tools | **Format** | a button (instead of Drawing Tools) |
| Picture Tools | **Format** | an image (also for a state image being edited) |
| Video Tools | **Options** | a video |
| Audio Tools | **Options** | an audio object |
| Table Tools | **Design**, **Layout** | a table or a table cell |
| Question Tools | **Design** | the slide is a question slide (any selection) |

Several captions can show at once (Question Tools + Drawing Tools). If the current tab disappears
because the selection changed, the ribbon returns to Home.

Modes replace the tab strip:

| Mode | Tab strip |
|---|---|
| Slide master | File, **Slide Master**, Home, Insert, Transitions, Animations, View, Help |
| Feedback master | File, **Feedback Master**, Home, Insert, View, Help |
| Question form view | File, **Question**, Slides, Help |
| Preview | File, **Preview** |
| Edit state (canvas state editing, M29) | the normal strip plus an **Editing State** caption with a **State** tab (Done, state picker, Reset property) |

## Drawing Tools › Format

| Group | Control | Kind | Action | Status | Notes |
|---|---|---|---|---|---|
| Insert Shapes | shape gallery | G | `object.add` `{shape}` | ✅ | `shape_gallery.rs` |
| | Change Shape | SS | `object.update` `{shape}` | ⛔ | keep size and format, change geometry |
| | Edit Points | S | `ui:editPoints` | ✅ | today Edit › Edit points |
| | Text Box | S | `object.add` `{text}` | ✅ | |
| Shape Styles ↘ | style gallery | G | `object.apply_style` | ✅ | |
| | Shape Fill | SS | `object.set_fill_style` | ✅ | solid, gradient, picture; eyedropper |
| | Shape Outline | SS | `object.update` `{outline}`, `object.set_outline_join` | ✅ | |
| | Shape Effect | SS | `object.set_shadow`, `object.set_glow` | 🟡 | ▸ Shadow ✅, Glow ✅, Reflection ⛔, Soft Edges ⛔ |
| Arrange and Align | Arrange | SS | `object.reorder` | ✅ | Bring to Front/Forward, Send Backward/to Back |
| | Group | SS | `object.group`, `object.ungroup` | ✅ | |
| | Rotate | SS | `object.set_rotation`, `object.flip` | ✅ | |
| | align grid (9) | S | `object.align`, `object.distribute`, `object.match_size` | ✅ | Align to Slide toggle |
| Size and Position ↘ | X, Y, Width, Height | N | `object.update` | ✅ | ↘ opens Size and Position dialog ⛔ (inspector today) |
| Publish | Preview | LS | | ✅ | |

## Button Tools › Format

| Group | Control | Kind | Action | Status | Notes |
|---|---|---|---|---|---|
| Action | When Clicked Action | L | `trigger.add` (on click) | 🟡 | opens the trigger editor pre-filled with "when the user clicks" this button |
| Button Icons | icon gallery | G | `object.update` `{icon}` | ⛔ | built-in arrow / check / play icons inside the button |
| | Align Icon | SS | `object.update` `{icon_align}` | ⛔ | left, right, top, bottom, centre |
| | Icon Color | SS | `object.update` `{icon_color}` | ⛔ | |
| | Remove Icon | S | `object.update` `{icon: null}` | ⛔ | |
| Button Styles ↘ | style gallery | G | `object.apply_style` | ✅ | |
| | Button Fill / Border / Effects | SS | as Shape Fill / Outline / Effect | ✅ / 🟡 | |
| Arrange and Align | as Drawing Tools | | | ✅ | |
| Size and Position ↘ | X, Y, Width, Height | N | `object.update` | ✅ | |
| Publish | Preview | LS | | ✅ | |

A new button gets five built-in states (Normal, Hover, Down, Visited, Disabled); CourseCraft
already creates them (M29), check the set matches.

## Picture Tools › Format

| Group | Control | Kind | Action | Status | Notes |
|---|---|---|---|---|---|
| Adjust | Brightness | SS | — | ⛔ | |
| | Contrast | SS | — | ⛔ | (`contrast` in code is the accessibility checker) |
| | Recolor | SS | `course.recolor` | 🟡 | recolours SVG/icons; photos ⛔ |
| | Replace Picture | SS | `object.update` `{src}` | ✅ | ▾ From File…, From Media Library… |
| | Reset Picture | SS | `object.set_crop` (clear) + reset size | 🟡 | |
| Picture Styles ↘ | style gallery | G | `object.apply_style` | ⛔ | frames, bevels: needs picture style presets |
| | Picture Shape | SS | `object.set_crop` `{mask}` | ⛔ | crop to shape |
| | Picture Border | SS | `object.update` `{outline}` | ✅ | |
| | Picture Effects | SS | `object.set_shadow`, `object.set_glow` | 🟡 | as Shape Effect |
| Arrange and Align | as Drawing Tools | | | ✅ | |
| Size and Position ↘ | Zoom Picture | L | `object.update` `{zoom_on_click}` | ⛔ | learner clicks to enlarge |
| | Crop | L | `object.set_crop` | ✅ | `crop.rs` |
| | X, Y, Width, Height | N | `object.update` | ✅ | |
| Publish | Preview | LS | | ✅ | |

## Video Tools › Options

Not shown in the tutorial; mirrors Audio Tools plus video specifics.

| Group | Control | Kind | Action | Status |
|---|---|---|---|---|
| Play | Preview | L | `ui:media.preview` | ✅ |
| Video Options ↘ | Volume | LS | `object.set_media` `{volume}` | ✅ |
| | Video Editor (trim) | L | `object.set_media` `{trim}` | 🟡 |
| | Replace Video | SS | `object.update` `{src}` | ✅ |
| | Export Video | S | `ui:media.export` | ⛔ |
| | Show Video: On Slide / In New Window | C | `object.set_media` | ⛔ |
| | Play Video: Automatically / When Clicked / From Trigger | C | `object.set_media` `{autoplay}` | ✅ |
| | Video Controls: Show / Hide | C | `object.set_media` `{controls}` | ✅ |
| | Compression | C | — | ⛔ |
| Closed Captions | Add / Import / Export / Delete | | `object.set_captions` | 🟡 import ✅ |
| Cue points | Add Cue / Question at time | | `video.add_cue`, `video.add_question_cue` | ✅ (CourseCraft extra) |
| Publish | Preview | LS | | ✅ |

## Audio Tools › Options

| Group | Control | Kind | Action | Status |
|---|---|---|---|---|
| Play | Preview | L | `ui:media.preview` | ✅ |
| Audio Options ↘ | Audio Volume | LS | `object.set_media` `{volume}` | ✅ |
| | Audio Editor | L | `ui:audioEditor` | 🟡 trim ✅, edit waveform ⛔ |
| | Replace Audio | SS | `object.update` `{src}` | ✅ ▾ From File, Record Mic, Text-to-Speech |
| | Export Audio | S | `ui:media.export` | ⛔ |
| | Compression | C | — | ⛔ |
| Closed Captions | Add Captions | L | `object.set_captions` | 🟡 |
| | Import / Export / Delete | S | `object.set_captions` | 🟡 |
| Publish | Preview | LS | | ✅ |

## Table Tools › Design and Layout

Not shown in the tutorial. CourseCraft already has the commands.

| Tab | Group | Controls | Action | Status |
|---|---|---|---|---|
| Design | Table Style Options | Header Row (K) | `table.set_header_row` | ✅ |
| | Table Styles | gallery, Shading, Borders | `table.set_cell`, `table.set_borders` | 🟡 |
| Layout | Rows & Columns | Insert Above/Below/Left/Right, Delete | `table.insert_row`, `table.insert_column`, `table.delete_row`, `table.delete_column` | ✅ |
| | Merge | Merge Cells, Split Cells | `table.merge`, `table.split` | ✅ |
| | Cell Size | row height, column width | `table.resize` | ✅ |
| | Alignment | cell text alignment | `table.set_cell` | 🟡 |

## Question Tools › Design

| Group | Control | Kind | Action | Status | Notes |
|---|---|---|---|---|---|
| Feedback | Feedback Master | LS | — | ⛔ | |
| Display | Feedback: By Question / By Choice / None | C | `question.set` `{feedback}` | 🟡 | per-choice feedback to verify |
| | Shuffle: Answers / None | C | `question.set` `{shuffle}` | ✅ | |
| | Results Slide | C | `question.set` `{results}` | ✅ | |
| Scoring | Score: By Question / By Choice | C | `question.set` `{scoring}` | 🟡 | |
| | Attempts | C | `question.set` `{attempts}` | ✅ | |
| AI | Edit Question / Insert Question | L | — | ⛔ | |
| Publish | Player / Preview / Publish | | | ✅ | |

The right panel gains a **Question** pane with a **Form View | Slide View** toggle (today
`question.rs` shows the question editor in the inspector).

## Question form view (Question tab)

| Group | Controls | Status |
|---|---|---|
| Clipboard | Cut, Copy, Paste | ✅ |
| Text | Bold, Italic, Underline, Spelling | ✅ |
| Feedback | Feedback Master | ⛔ |
| Display, Scoring | as Question Tools › Design | as there |
| AI | Edit Question, Insert Question | ⛔ |
| Insert | Media ▾, Audio ▾ (per question/feedback) | ⛔ |
| Publish | Player, Preview, Publish | ✅ |

Body layout (storyline-observed.md §3): question box, choices grid with a Correct column (rows A–J),
feedback-and-branching grid (Correct, Incorrect, Post-quiz review; Feedback, More…, Points). Today
the same data is edited in `question.rs`; form view is a full-canvas layout of it. ⛔ as a view, ✅
as data.

## Slide Master tab

| Group | Control | Kind | Action | Status |
|---|---|---|---|---|
| Edit Master | Insert Slide Master | L | `master.add` | ✅ |
| | Insert Layout | L | `master.add` `{layout}` | ✅ |
| | Delete | S | `master.delete` | ✅ |
| | Rename | S | `ui:rename` | ✅ |
| | Preserve | T | — | ⛔ |
| Master Layout | Insert Placeholder | LS | — | ⛔ ▾ Content, Text, Picture, Media… |
| | Title | K | — | ⛔ |
| Edit Theme | Themes | LS | `course.set_theme` | ✅ |
| | Colors / Fonts | SS | as Design | as Design |
| Background ↘ | Background Styles, Hide Background Graphics | | as Design | as Design |
| Close | Close Master View | L | `ui:view.slide` | ✅ |

## Feedback Master tab

⛔ as a whole. Same groups as Slide Master; edits the layouts of Correct / Incorrect / Try Again
feedback layers (today generated by `question.add_feedback_layers`).

## Preview tab (preview window)

| Group | Control | Kind | Action | Status |
|---|---|---|---|---|
| Close | Close Preview | L | `ui:preview.close` | ✅ |
| Preview | Select | LS | `ui:preview.scope` | ✅ This Slide / This Scene / Entire Course |
| | Replay | LS | `ui:preview.replay` | 🟡 ▾ Replay Slide, Replay Scene, Replay Course |
| | Edit Slide | L | `ui:preview.edit` | ⛔ close preview on the current slide |
| | Inspect | L | `ui:preview.inspect` | ⛔ variables and trigger log (the simulator has the data) |
| (right) | Desktop / Tablet landscape / Tablet portrait / Phone landscape / Phone portrait | T×5 | `ui:preview.device` | ⛔ |
| | Settings ⚙ | S | `ui:preview.settings` | ⛔ |

## 360° image editor

🚫 Not planned. Not drawn: there is no 360° object to select, so its tabs never appear. If 360°
images are planned later, the observed layout is in storyline-observed.md §3.
