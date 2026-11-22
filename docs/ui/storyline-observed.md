# Storyline 360 UI as observed

> Reference for [the ribbon plan](README.md). Source: a public YouTube beginner course,
> "Articulate Storyline 360 Tutorial 2026 Full Course for Beginners" (78 min, 1920×1080, Windows,
> Storyline 360 x64, free trial, English UI). Timestamps are `mm:ss` into that video. Behaviour and
> layout only; no assets were copied (see README › Clean-room).
>
> **Seen** = visible in the video. **Not shown** = the tutorial never opens it; the entry comes from
> the tab strip only and must be checked against the app before anyone builds it.

## 1. Window at a glance (07:55, 08:20)

```
┌───────────────────────────────────────────────────────────────────────────────────────────────┐
│ [app] [Save][Undo][Redo][▾]          Name.story* - Articulate Storyline x64          [_][□][x]│ title bar + QAT
│ FILE  HOME  INSERT  SLIDES  DESIGN  TRANSITIONS  ANIMATIONS  VIEW  HELP  [TOOLS: FORMAT]   (?) │ tab strip
│ ┌Clipboard┐┌Slide┐┌AI Assistant New┐┌Font──────┐┌Paragraph──┐┌Drawing──┐┌Publish┐               │ ribbon (~95 px)
├───────────────────────────────────────────────────────────────────────────────────────────────┤
│ [STORY VIEW] [1.2 Introduction ×]                               [desk][tabL][tabP][phL][phP][⚙]│ view tabs row
├──────────┬──────────────────────────────────────────────────────────┬─────────────────────────┤
│ Scenes   │                                                          │ Slide | Comments | AI   │
│ [1 Basics▾]                    slide canvas                        │ Triggers   [toolbar]    │
│ 1.1 ▢    │                                                          │  Object / Slide /       │
│   ↓      │                                                          │  Player Triggers        │
│ 1.2 ▢    ├──────────────────────────────────────────────────────────┤                         │
│   ↓      │ Timeline | States | Notes                     [⊕][▾][⧉]  ├─────────────────────────┤
│ 1.3 ▢    │ [👁][🔒] rows …  ruler 00:01 … End                         │ Slide Layers            │
│          │ [▶][■] [zoom −○+] 00:05.00                                │  Base Layer  [+][⧉][🗑] Dim [⚙]│
├──────────┴──────────────────────────────────────────────────────────┴─────────────────────────┤
│ Slide 1 of 5   960 × 540   "Clean"                         [views] 96% [− ─○─ +] [fit]        │ status bar
└───────────────────────────────────────────────────────────────────────────────────────────────┘
```

- Tab strip order is fixed: FILE (blue, opens the file menu), HOME, INSERT, SLIDES, DESIGN,
  TRANSITIONS, ANIMATIONS, VIEW, HELP. Contextual tabs appear at the right end under a coloured
  caption ("DRAWING TOOLS" › FORMAT, "PICTURE TOOLS" › FORMAT, "AUDIO TOOLS" › OPTIONS,
  "QUESTION TOOLS" › DESIGN). Two captions can show at once (QUESTION TOOLS + DRAWING TOOLS, 71:30).
- Each ribbon group has a caption under it; some have a dialog launcher (↘) at the bottom right
  (Font, Paragraph, Drawing, Background, Size and Position, Audio Options, Picture Styles).
- Controls: large buttons (icon over a one- or two-line label), large split buttons (lower half
  with ▾ opens a menu), small buttons in columns of three (icon + label), small split buttons,
  combo boxes (font, size), galleries with scroll arrows and a "more" chevron, spinners
  (Duration, X, Y, Width, Height) and checkboxes (Title, Hide Background Graphics).
- Hovering a split button's top half explains it (09:05: Preview's top half previews the entire
  project; ▾ offers smaller scopes).
- Every panel can be collapsed (▾), undocked (⧉) into a floating window (36:30) and docked back.
  The Timeline panel collapses to a one-line bar "Timeline, States, Notes" at the bottom (19:10).
- The ribbon shows a **Story view** variant of HOME (07:55) and a **Slide view** variant (08:20):
  same tab, different groups.

## 2. Main tabs

### FILE (not shown)

A blue button at the left of the tab strip. The tutorial saves with the QAT Save button (13:20)
and never opens FILE. Expected entries to confirm: New, Open, Recent, Save, Save As, Import,
Translation, Publish, Project Info, Options, Help, Close, Exit.

### HOME, Story view (07:55, 14:45)

| Group | Controls (left to right; a column is listed top to bottom) |
|---|---|
| Clipboard | **Paste** (large split ▾) · column: Cut, Copy |
| Scenes | **New Scene** · **Starting Scene** (toggle; the start scene gets a red flag) · **Index** ▾ · **Question Banks** ▾ |
| AI Assistant `New` | **Quiz** · **Question** · **Summary** |
| Slide | **New Slide** ▾ · column: Apply Layout ▾, Reset, Duplicate · **Record Screen** ▾ |
| Publish | **Player** · **Preview** ▾ · **Publish** |

### HOME, Slide view (08:20, 24:35, 34:15)

| Group | Controls |
|---|---|
| Clipboard | **Paste** ▾ (paste options: Use Destination Theme, Keep Source Formatting, Keep Text Only, Paste Special…; 24:30) · column: Cut, Copy ▾, Format Painter |
| Slide | **New Slide** ▾ · column: Apply Layout ▾, Focus Order, Duplicate |
| AI Assistant `New` | **Insert Text** · **Edit Text** · **Insert Image** · **Insert Audio** ▾ · column: Quiz, Question, Summary |
| Font (↘) | **Text Styles** ▾ (Normal Text, headings…) · font family combo, size combo, Grow Font, Shrink Font, Clear Formatting · Bold, Italic, Underline, Shadow, Strikethrough, Character Spacing ▾, Change Case ▾, Highlight ▾, Font Color ▾ (theme colours, eyedropper; 25:20), Spelling |
| Paragraph (↘) | Bullets ▾ (symbol choice; 26:00), Numbering ▾, Decrease Indent, Increase Indent, Line Spacing ▾ · Align Left, Center, Align Right, Justify · column: Text Direction ▾, Align Text ▾, Find / Replace ▾ |
| Drawing (↘) | **Arrange** ▾ · **Quick Styles** ▾ · column: Shape Fill ▾, Shape Outline ▾, Shape Effect ▾ |
| Publish | **Player** · **Preview** ▾ · **Publish** |

Preview ▾ (09:40): This Slide, This Scene, Selected Slides, Entire Project, Publish….

### INSERT (08:45)

| Group | Controls |
|---|---|
| Slide | **Slide Layer** · **Convert to Freeform** · **Zoom Region** |
| AI Assistant `New` | **Insert Text** · **Edit Text** · **Insert Image** · **Insert Audio** ▾ · column: Quiz, Question, Summary |
| Content Library 360 | **Characters** · **Photos** · **Illustrations** · **Icons** · **Videos** |
| Media | **Picture** ▾ · **360° Image** ▾ · **Shape** ▾ · **Caption** ▾ · **Video** ▾ · **Audio** ▾ · **Web Object** |
| Text | **Text Box** ▾ · **Table** ▾ · **Slide Number** ▾ · column: Symbol, Reference, Hyperlink |
| Interactive Objects | **Button** ▾ · **Slider** ▾ · **Dial** ▾ · **Hotspot** ▾ · **Input** ▾ · **Marker** ▾ · column: Trigger, Scrolling Panel, Mouse ▾ |
| Publish | **Preview** ▾ |

Audio ▾ (09:15): Audio from File…, Media Library…, Record Mic…, Text-to-Speech…, AI Audio ▸,
Background Audio ▸, Options….

Marker gallery picks up theme accent 1 automatically (44:05). Characters opens a dialog (§5).
Photos opens "Search Photos" (27:35).

### SLIDES (not shown)

Present in the strip from the start; never opened. To confirm: New Slide, Import (PowerPoint,
Quizmaker, Storyline, Rise), Duplicate, New Scene, Question Banks, Feedback Master, Slide Master.

### DESIGN (12:35, 20:20, 41:30)

| Group | Controls |
|---|---|
| Setup | **Slide Size** (opens Slide Size dialog) |
| Themes | gallery of ~21 theme tiles "Aa" with a colour strip; scroll ▲▼ and "more" |
| (no caption, Theme options) | column: Colors ▾ (… Create New Theme Colors…), Fonts ▾ (… Create New Theme Fonts…) |
| Background (↘) | Background Styles ▾ · Hide Background Graphics (checkbox) |
| Publish | **Preview** ▾ |

### TRANSITIONS (not shown)

To confirm: transition gallery, Effect Options ▾, Duration, Preview.

### ANIMATIONS (35:05, 35:30)

| Group | Controls |
|---|---|
| Advanced | **Animation Painter** |
| Entrance Animations | gallery button showing current (None / Wipe…) ▾ · column: Duration spinner, Effect Options ▾ |
| Exit Animations | same as Entrance |
| Emphasis Animations | **Add Emphasis Animation** ▾ · list box of emphasis effects · column: Duration, Effect Options ▾, Delete |
| Motion Paths | **Add Motion Path** ▾ · column: Duration, Name · **Path Options** ▾ |
| Size and Position (↘) | X, Y, Width, Height spinners |
| Publish | **Preview** ▾ |

Entrance gallery (35:10): None, Fade, Grow, Fly In, Float In, Split, Wipe, Shape, Wheel,
Random Bars, Spin, Spin & Grow, Grow & Spin, Zoom, Swivel, Bounce. Effect Options holds the
direction (From Left…) and **Sequence** (As One Object / By Paragraph); By Paragraph adds a ▸ on the
timeline row that expands one bar per paragraph (35:30).

### VIEW (not shown except Slide Master)

The tutorial opens View › Slide Master (14:45 → master view at 15:00). To confirm: Story View,
Normal View, Slide Master, Feedback Master, Zoom, Fit in Window, Ruler, Gridlines, Guides,
Grid and Guides…, panel toggles, Reset panel layout.

### HELP (not shown)

To confirm: Help, Tutorials, Community, Check for Updates, About.

## 3. Contextual tabs

### DRAWING TOOLS › FORMAT (54:30)

| Group | Controls |
|---|---|
| Insert Shapes | shape gallery (3 rows, scroll) · column: Change Shape ▾, Text Box |
| Shape Styles (↘) | style gallery "Abc" tiles · column: Shape Fill ▾, Shape Outline ▾, Shape Effect ▾ (Shadow ▸, Reflection ▸, Glow ▸, Soft Edges ▸) |
| Arrange and Align | column: Arrange ▾, Group ▾, Rotate ▾ · 3×3 icon grid: Align Left/Center/Right, Top/Middle/Bottom, Distribute Horizontally/Vertically, (align to slide) |
| Size and Position (↘) | X, Y, Width, Height |
| Publish | **Preview** ▾ |

### PICTURE TOOLS › FORMAT (29:15, 44:05)

| Group | Controls |
|---|---|
| Adjust | column: Brightness ▾, Contrast ▾, Recolor ▾ · column: Replace Picture ▾, Reset Picture ▾ |
| Picture Styles (↘) | style gallery (Moderate Frame, Bevel Matte…) · column: Picture Shape ▾, Picture Border ▾, Picture Effects ▾ |
| Arrange and Align | Arrange ▾, Group ▾, Rotate ▾ · align grid |
| Size and Position (↘) | **Zoom Picture** · **Crop** (handles, live px read-out; 29:30) · X, Y, Width, Height |
| Publish | **Preview** ▾ |

### AUDIO TOOLS › OPTIONS (33:55)

| Group | Controls |
|---|---|
| Play | **Preview** |
| Audio Options (↘) | **Audio Volume** ▾ · **Audio Editor** · column: Replace Audio ▾, Export Audio, Compression combo (Automatic) |
| Closed Captions | **Add Captions** · column: Import, Export, Delete |
| Publish | **Preview** ▾ |

### QUESTION TOOLS › DESIGN (71:30)

| Group | Controls |
|---|---|
| (Feedback) | **Feedback Master** ▾ |
| Display | Feedback: [By Question ▾] · Shuffle: [Answers ▾] · Results Slide: [None ▾] |
| Scoring | Score: [By Question ▾] · Attempts: [1 ▾] |
| AI Assistant `New` | **Edit Question** · **Insert Question** |
| Publish | **Player** · **Preview** ▾ · **Publish** |

### Slide master mode: SLIDE MASTER tab (15:00, 16:35)

Strip becomes FILE, **SLIDE MASTER**, HOME, INSERT, TRANSITIONS, ANIMATIONS, VIEW, HELP
(SLIDES and DESIGN leave).

| Group | Controls |
|---|---|
| Edit Master | **Insert Slide Master** · **Insert Layout** · column: Delete, Rename, Preserve |
| Master Layout | **Insert Placeholder** ▾ · Title (checkbox) |
| Edit Theme | **Themes** ▾ · column: Colors ▾, Fonts ▾ |
| Background (↘) | Background Styles ▾ · Hide Background Graphics |
| Close | **Close Master View** |

Left panel: master thumbnail with its layouts indented beneath, joined by a dotted line.

### Question form view (69:15, 69:55)

Strip: FILE, **QUESTION**, SLIDES, HELP.

| Group | Controls |
|---|---|
| Clipboard | column: Cut, Copy, Paste |
| Text | Bold, Italic, Underline, (format), Spelling |
| (Feedback) | **Feedback Master** ▾ |
| Display | Feedback ▾ · Shuffle ▾ · Results Slide ▾ |
| Scoring | Score ▾ · Attempts ▾ |
| AI Assistant `New` | **Edit Question** · **Insert Question** |
| Insert | **Media** ▾ · **Audio** ▾ |
| Publish | **Player** · **Preview** ▾ · **Publish** |

Body: "Enter the question" box; "Enter the choices" grid (rows A–J, columns Correct | Choice;
click "Click to enter a choice"); "Set feedback and branching" grid (Correct / Incorrect /
Post-quiz review; Feedback text; MORE… button; Points 10 / 0). Right panel "Question" with a
**Form View | Slide View** toggle and a live thumbnail.

### Preview mode: PREVIEW tab (47:25)

Strip: FILE, **PREVIEW**. Groups: Close (**Close Preview**), Preview (**Select** ▾, **Replay** ▾,
**Edit Slide**, **Inspect**). Right side: device buttons Desktop, Tablet landscape, Tablet portrait,
Phone landscape, Phone portrait, and a settings gear. The player shows MENU sidebar, title,
RESOURCES, the slide, and the control bar. "The player frame is responsive; the slide is not"
(47:40).

### 360° image editor (50:20, 52:00) — 🚫 not planned in CourseCraft

Strip: FILE, MARKER, LABEL, INTERACTION. LABEL: Insert (Picture, Video, Audio, Button),
Play (Preview), Edit (Volume, Crop, Accent), Accessibility (Alt Text, Add Captions, Import, Export,
Delete), Font, Preview, Close (**Save & Close** ▾). Left panel "Markers & Hotspots": Free
navigation / Guided tour, Show progress indicator, marker list. A breadcrumb tab
"1.4 Untitled Slide › 360° Image 1" sits in the view tabs row.

## 4. Panels

### Scenes panel (left, slide view; 08:20)

Title "Scenes" with undock. Scene combo ("1 Basics ▾"); the mouse wheel over it switches scenes
(10:40). Thumbnails numbered "1.1 Course Title", arrows between them show the navigation flow,
a link icon after the last. Right-click: New Slide ▸ Basic Layouts ▸ (Title Only, Title and
Content, Blank…), Apply Layout ▸, Delete… (23:30, 71:30).

### Story view (07:55, 66:30)

Each scene is a card: "1 Basics ▾" header (red flag on the starting scene), thumbnails with
titles, arrows. Ctrl+wheel zooms. Double-click a slide opens it in a new view tab. Double-click a
scene or slide title renames it; slide titles feed the player menu (67:30).

### Timeline / States / Notes (bottom; 34:15, 36:30, 37:40)

- Tabs: Timeline, States, Notes. Header icons right: cue/adjust (⊕), menu ▾, undock (⧉).
- Columns: eye (show/hide in editor) and lock, then the object name (double-click to rename;
  the tutor names `txt_title`, `pic_outsiders`), then the bar with its start, a fade in/out wedge
  for entrance/exit animations, and the "End" marker of the slide.
- Ruler `00:01…`; playhead (blue triangle); **cue points** added with **C** while audio plays,
  objects snap to them (37:40).
- Footer: Play, Stop, zoom out / slider / zoom in, slide duration `00:05.00`.
- Default slide duration 5 s; adding a longer audio stretches the slide and objects that have
  "Show Until End" (33:30).

Row right-click (34:10): Cut, Copy, Paste, Paste Special…, Rename "<name>", Show in Media
Library…, Bring to Front ▸, Send to Back ▸, Create Cue Point at Playhead, Fit Timeline to Longest
Object, Reset Timeline to Default, Alignment ▸, ✓ Show Until End, Show Always, Timing…,
Accessibility….

States tab (64:40): **EDIT STATES** button, "Initial state: [Normal ▾]", caption "States for
<object>", one thumbnail per state (Normal with a flag = initial; Hover, Visited with a lightning
mark = built-in). Built-in states work automatically; custom states need triggers (65:10).

### Right panel (08:20, 58:45)

- Tabs: **Slide**, **Comments**, **AI Assistant** `New`.
- **Triggers**: toolbar New Trigger, Edit, Copy, Paste, Delete, Move Up/Down; Group checkbox;
  Manage project variables. Sections **Object Triggers**, **Slide Triggers**, **Player Triggers**
  (collapsible); each trigger reads as a sentence: "Show layer **Intro** / When the timeline
  starts on **this slide**", links in blue to edit inline.
- **Slide Properties** (Story view; 07:55): Properties for "1.1 Course Title"; Slide advances
  [By user ▾]; When revisiting [Automatically decide ▾]; Slide navigation and gestures: Buttons
  ☑Prev ☑Next ☐Submit; Swipe ☑Prev ☑Next; Player features [Player defaults ▾].
- **Slide Layers**: list with the base layer last "(Base Layer)" and an eye per layer; footer New
  Layer, Duplicate, Delete, ☑ Dim, Layer properties ⚙. Double-click renames (53:20).

### Status bar (08:20)

Left: "Slide 1 of 5" (Scene 1 of 1 in Story view, "Slide Master" in master mode), "960 × 540",
theme name "Clean". Right: three view buttons, zoom %, − slider +, fit to window.

## 5. Context menus and dialogs

**Object right-click** (16:35): Cut, Copy, Paste Options (three icons), Paste Special…, Group ▸,
Bring to Front ▸, Send to Back ▸, Button Set ▸, Drag and Drop ▸, Add new horizontal guide line,
Add new vertical guide line, Replace Picture ▸, Export Picture ▸, Show in Media Library…,
Rename "Picture 1", Size and Position…, Accessibility…, Format Picture….

| Dialog | Seen at | Contents |
|---|---|---|
| Slide Size | 12:35 | Preset ▾ (4:3 720×540, 16:9 …), Width, Height (px spinners), ☑ Lock aspect ratio, CURRENT DEFAULT SIZE, ☑ Scale existing content to fit, Learn more, OK, Cancel |
| Create New Theme Fonts | 20:20 | Heading font, Body font (font lists show each face in its own font), sample, Name, Save, Cancel |
| Create New Theme Colors | 41:30 | Name; swatches Text/Background Light 1, Dark 1, Light 2, Dark 2, Accent 1–6, Hyperlink, Control 1, Quiz Correct, Quiz Incorrect, Custom 1, Custom 2 (each ▾ with hex input); light and dark preview; OK, Cancel |
| Trigger Wizard | 58:45 | Action [verb ▾] + target (Layer [▾]); When [event] + Object [▾]; Conditions (optional) "+ if"; Learn more; OK, Cancel. Clicking an object first pre-fills "When the user clicks Button 1" |
| Insert Slide | 60:30, 68:50 | Title per category (Content Library 360, Graded Questions…); search; Sort by [Newest ▾]; filters (In Use, Type: Openings, Titles and Sections, Content, Charts and Graphs, Scenarios, Interactions, Closings, Graded, Survey, Freeform, Blank; Theme: Light, Dark); tile grid with zoom slider; preview pane with description; INSERT SLIDE. Built-in graded questions: True/False, Multiple Choice, Multiple Response, Fill-in-the-Blank, Word Bank, Matching Drag-and-Drop, Matching Drop-down, Sequence Drag-and-Drop, Sequence Drop-down, Numeric, Hotspot. Ctrl/Shift-click inserts several (61:10) |
| Search Photos | 27:35 | search box, type ▾, result grid with source on hover, Insert |
| Characters | 72:10 | tabs Character / Expression / Pose; filters Emotion (Positive, Neutral, Negative), Style (Neutral, Talking, Listening, Reacting, Thinking, Gesturing, Presenting, Holding Item, Holding Sign); Sort by; preview; INSERT |
| Player Properties | 74:35 | its own ribbon PROPERTIES: Layout (Features), Data (Menu, Resources, Glossary), Appearance (Player Style [Modern ▾], Menus & Controls [On ▾], Colors & Effects), Custom (Text Labels, Other), Player (Current Player ▾). Body: Player Tabs (Sidebar: Menu, Glossary, Notes; Topbar: Resources…) with add/edit/delete/up/down; Player Features (Title, Sidebar [On Left ▾], Sidebar starts collapsed, Logo, Cover Photo, Fill player frame); Player Controls (Search, Seekbar with Allow drag / Read-only / Drag after completion, Play/Pause, Volume, Captions, Playback speed, Accessibility controls…); live preview; Learn more; OK, Cancel. Colors & Effects offers a light theme and takes Accent 1 (75:30) |
| Publish | 76:15 | left list: Review 360, Web, Video, Word, LMS / LRS, Reach 360. LMS page: Title (…), Description, Folder (…); Properties: Player, Quality, Publish (Entire Project), Tracking (Slides viewed 6 of 6) as blue links; Reporting and Tracking: ☑ Report to an LMS [SCORM 1.2 ▾], ☐ Report to an external LRS, REPORTING AND TRACKING…; Learn more about publishing; PUBLISH, CANCEL |
| Save As / Image from file | 13:20, 75:00 | system file dialogs |

## 6. Interaction details worth keeping

- Selecting a text box frame (solid border) applies formatting to all its text (16:50).
- Pink smart guides while dragging (17:00); Shift keeps proportions (56:10).
- Shift+wheel zooms the canvas (45:30); the slide can be zoomed out to work off-slide.
- Empty text boxes delete themselves on deselect (21:10).
- Locking an object on the timeline prevents selecting it on the canvas (44:30).
- New objects of a type take the theme colour (markers, buttons; 44:05).
- The ribbon's Font combo accepts typing a letter to jump (17:40).
