# UI parity: the ribbon editor

> **Status:** planned (M30). **Written:** 2026-11-23. **Source:** a public 78-minute beginner
> tutorial for Storyline 360 (2026 build), watched frame by frame. Observations are of layout and
> behaviour only (see [Clean-room](#clean-room)).

CourseCraft's editor today is a menu bar (File, Edit, Insert, Arrange, Project, View, Help) over a
flat toolbar (Add slide, Preview, Text, Shape, Image, Media, Button, Question, Trigger, Publish) and
an inspector on the right. A Storyline author expects a **tabbed ribbon** instead. This folder is the
plan for that ribbon: the same tabs, the same groups and the same buttons in the same order, so a
Storyline author finds every command where they look for it.

**The rule: build every menu now, wire what exists, disable the rest.** A button whose feature is not
built yet is still drawn, in its place, greyed out, with a tooltip that says why
("Not available yet: zoom regions"). It never disappears and it never fakes a result. When the
feature lands, the button is switched on; nothing moves.

## Files

| File | What it holds |
|---|---|
| [storyline-observed.md](storyline-observed.md) | What the tutorial shows, tab by tab and panel by panel, with timestamps. The reference. |
| [ribbon.md](ribbon.md) | CourseCraft's main tabs: every group and control, its CourseCraft label, kind, command id and status. |
| [contextual-tabs.md](contextual-tabs.md) | Tabs that appear for a selection or a mode: Drawing Tools, Picture Tools, Audio/Video Tools, Question Tools, Table Tools, Slide Master, Feedback Master, Preview, question form view. |
| [shell-panels-dialogs.md](shell-panels-dialogs.md) | Window layout around the ribbon: title bar, quick access toolbar, view tabs, panels (Scenes, Timeline/States/Notes, Triggers, Slide Layers, Slide Properties), status bar, context menus and dialogs. |
| [implementation.md](implementation.md) | How to build it in `coursecraft-ui-egui`: a declarative ribbon spec, the widgets to port from DeckCraft, enable rules, tests and the order of work. |

## Status legend

Used in every table in this folder.

| Mark | Meaning | Drawn as |
|---|---|---|
| ✅ | Built: dispatches an existing engine command or UI action | enabled (subject to the selection) |
| 🟡 | Partly built: the button works, some options of its menu are disabled | enabled; missing menu items disabled |
| ⛔ | Not built yet, but planned or plausible | **disabled**, tooltip `Not available yet: <feature>` |
| 🚫 | Not planned (see `docs/roadmap.md` › Not planned for now) | **disabled**, tooltip `Not planned: <feature>` |
| — | Storyline has it, CourseCraft deliberately leaves it out (cloud services, licensing) | not drawn; listed so nobody adds it by mistake |

A status is a claim about the code on the date above. Whoever wires a control checks it against the
code first and updates the table in the same commit.

## Clean-room

Read `docs/contributing.md` › Clean-room rules first. For this work it means:

- **Same structure, our own look.** Tab names, group names, button order and what each button does
  follow the tutorial. Icons, colours, spacing and fonts come from `DESIGN.md`. No icon, screenshot
  or asset from Storyline goes into the repository, and no frame of the tutorial is committed.
  Timestamps in `storyline-observed.md` point at the public video instead.
- **Generic labels stay; coined names change.** Home, Insert, Design, Animations, Trigger, Layer,
  State, Question Bank, Slide Master, Text Styles, Format Painter are generic and stay. Product names
  and coined terms get CourseCraft names:

| Storyline label | CourseCraft label | Why |
|---|---|---|
| Story View | **Course Map** | coined; CourseCraft already uses Course Map |
| Convert to Freeform | **Convert to Custom Question** | coined; CourseCraft already uses Custom question |
| Freeform (question type) | **Custom question** | as above |
| Content Library 360 (group) | **Library** | product name |
| Review 360 | **Share for Review** | product name |
| Reach 360 | — (not drawn) | hosted service |
| Trigger Wizard (dialog title) | **Trigger** | coined dialog name |
| AI Assistant (group) | **AI** | product feature name; CourseCraft's AI runs through the user's own agent or key |
| Articulate 360 / Storyline in titles and Help | **CourseCraft** | product name |

  If the owner wants an exact label where this table differs, that is a decision for
  `docs/contributing.md`, not for a single commit.

## How to add or wire a button

1. Find the control in [ribbon.md](ribbon.md) or [contextual-tabs.md](contextual-tabs.md).
2. If the behaviour is new, add the engine command first (`docs/contributing.md` › add-a-command
   checklist). UIs only dispatch (AGENTS.md rule 2).
3. In the ribbon spec (see [implementation.md](implementation.md)), change the control's action from
   `NotYet(..)` to `Command("<id>")` or `Ui(<action>)`.
4. Change its status in the table to ✅ or 🟡.
5. If the control changes play-time behaviour, update `docs/spec/triggers.md` in the same commit.
6. Run `cargo xtask ci`.
