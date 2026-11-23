# Building the new UI

> Part of [the ribbon plan](README.md). How to turn [ribbon.md](ribbon.md),
> [contextual-tabs.md](contextual-tabs.md) and [shell-panels-dialogs.md](shell-panels-dialogs.md)
> into code in `coursecraft-ui-egui` (layer L6). Nothing here changes play-time behaviour, so
> `docs/spec/triggers.md` is untouched unless a newly wired button adds a runtime feature.
>
> **Decision (2026-11-23, owner):** the UI rewrite is built on **`craft-ui`**, the shared egui
> component library of the Craft apps, and the interface is localized with **`craft-i18n`** (see
> [localization.md](localization.md)). Both come from `storytold/craft-libs`.

## 0. Foundation: craft-libs

`craft-ui` is the base of every panel, tab, pane, button, combo and number field. CourseCraft code
adds only what `craft-ui` does not have: the ribbon (spec, groups, large and split buttons), the
CourseCraft theme values and the document views. When something generic is missing from
`craft-ui` (a ribbon group frame, say), prefer adding it to craft-libs upstream over a local copy.

| Crate (craft-libs) | Used for | Where |
|---|---|---|
| `craft-ui` `docking` | the work area: Scenes, canvas, Timeline/States/Notes, Triggers, Slide Layers as dockable panels; float, redock, Reset Panels; layout saved with the `serde` feature | `ui-egui/src/workspace.rs` (new) |
| `craft-ui` `panes` | task panes with title, ▾ menu and ×, resizable edge, floating frame | each panel's frame |
| `craft-ui` `tabs` | view tabs row (Course Map + open slides), Timeline/States/Notes tabs, right-panel tabs | `ui-egui/src/view_tabs.rs` (new) |
| `craft-ui` `layout` | split rects and resize handles (canvas / timeline splitter), toolbar layout | |
| `craft-ui` `buttons`, `icons` | icon buttons; SVG icon sets prepared once (needs `egui_extras` SVG loader) | ribbon buttons, panel toolbars |
| `craft-ui` `combo`, `localized` | `ComboBox`; number fields in the user's region (X, Y, Width, Height, Duration spinners) | ribbon combos and spinners |
| `craft-ui` `focus`, `ime`, `text`, `theme`, `fonts` | keyboard focus, input methods, text helpers, theme mechanisms, font install | everywhere |
| `craft-i18n` | every interface string, plurals, numbers, language names, system locale | [localization.md](localization.md) |
| later: `craft-menu`, `craft-keymap` | native macOS menu, customisable shortcuts | M30 step 8 or after |
| later: `craft-window`, `craft-dialogs`, `craft-settings` | app-drawn title bar with the QAT, non-blocking file dialogs, settings storage | after M30 |

**Pins.** Like DeckCraft, add each crate as a git dependency pinned to one craft-libs `rev` in
`[workspace.dependencies]`, all on the same rev, and bump them together:

```toml
craft-ui = { git = "https://github.com/storytold/craft-libs", rev = "<rev>", version = "=0.1.0" }
craft-i18n = { git = "https://github.com/storytold/craft-libs", rev = "<rev>", version = "=0.1.0" }
```

Today's rev is `63196e66f3033a6623eb629af95a104d26986725`, the one DeckCraft pins.

**egui fork.** craft-libs is built and tested against storytold's egui fork. CourseCraft adopts
DeckCraft's `[patch.crates-io]` block on the same rev DeckCraft uses, limited to the crates in
our graph (egui, eframe, egui-winit, egui-wgpu, epaint, emath, ecolor, epaint_default_fonts,
winit; add egui_extras or egui_kittest when we start using them),
so the editor runs on what craft-ui was tested with. This changes the player and render crates'
egui too: run the full gate and `cargo xtask wasm`, and compare the player in a browser before and
after.

**Toolchain.** craft-libs needs Rust 1.95 or newer; CourseCraft already requires 1.99.

**Layering.** craft-* crates are external dependencies of L6 (`coursecraft-ui-egui`) and the apps
only. Nothing below L6 depends on them; `cargo xtask layers` keeps it so. (A later choice may let
the player use `craft-i18n` for its own built-in labels; that is a separate decision.)

**Licences.** craft-libs is MIT OR Apache-2.0. Add rows to `ATTRIBUTION.md` for any assets they
embed (icon sets, fonts, CLDR data in `craft-i18n`).

## 1. One declarative spec

The ribbon is data, not hand-written layout per button. A single table in
`crates/ui-egui/src/ribbon/spec.rs` lists every tab, group and control in order, and the drawing
code walks it. This keeps the docs and the UI in step and makes "disabled, not missing" automatic.

```rust
pub struct Tab { pub id: &'static str, pub label: &'static str, pub when: TabWhen, pub groups: &'static [Group] }
pub struct Group { pub label: &'static str, pub launcher: Option<Action>, pub items: &'static [Item] }
pub struct Item { pub id: &'static str, pub label: &'static str, pub icon: Icon, pub kind: Kind, pub action: Action, pub menu: &'static [Item] }

pub enum Kind { Large, LargeSplit, Small, SmallSplit, Toggle, Check, Combo { width: f32 }, Spinner, Gallery { rows: u8 }, Column(&'static [Item]) }

pub enum Action {
    Command(&'static str),          // engine command id; enabled = registry `enabled` check
    Ui(UiAction),                    // editor-side action (open dialog, switch view, preview)
    NotYet(&'static str),            // drawn disabled: "Not available yet: {0}"
    NotPlanned(&'static str),        // drawn disabled: "Not planned: {0}"
}

pub enum TabWhen { Always, SlideOpen, MasterMode, Contextual(Ctx) }
```

- `Command` items take their label hint and shortcut from the engine registry, so the ribbon,
  the command palette and MCP agree.
- Parameters that depend on the selection (align edge, reorder delta) go through the existing
  adapters in `menu.rs` (`adapter`, `align_params`, `distribute_params`, `match_size_params`);
  move them to `ribbon/adapters.rs` when the menu bar goes.
- `NotYet` / `NotPlanned` reasons are short nouns ("zoom regions"); the tooltip text is built in
  one place.
- Every `label`, group caption, tooltip and reason is **English source text**, looked up through
  `tl!` when drawn, never stored translated. `Command` items look their label up with
  `I18N.tid(lang, id, label)` so translators can key it by command id (localization.md).
- A split button's `menu` items follow the same rules: one disabled entry never hides the button.

## 2. Ribbon widgets to port from DeckCraft

`craft-ui` has no ribbon. DeckCraft (`../deckcraft/crates/ui-egui`, same licence MIT OR
Apache-2.0, same egui fork) already draws one. Port only the ribbon-specific widgets, rebuilt on
`craft-ui` buttons and combos where those exist; do not depend on `deckcraft-ui-egui` (it pulls
in DeckCraft's model and engine). Copy with a header comment naming the source file and commit.
Generic pieces that turn out useful to other Craft apps (a ribbon group, a split button) are
candidates for craft-libs.

| DeckCraft | What it gives | Adapt |
|---|---|---|
| `widgets.rs` `big_button`, `big_button_ex` | large button and large split button with hover halves | colours from CourseCraft `DESIGN.md` tokens |
| `widgets.rs` `small_button`, `small_button_ex`, `small_button_named` | small (split) buttons in columns | |
| `widgets.rs` `group`, `separator`, `rows` | group frame with caption, column of three | add the ↘ launcher |
| `widgets.rs` `dropdown`, `spinner`, `gallery`, `color_split`, `color_grid`, `check_item`, `icon_toggle` | combos, spinners, galleries, colour menus | colour grid uses CourseCraft palette slots |
| `widgets.rs` `split_popup`, `split_color_popup` | menus under split buttons | |
| `ribbon.rs` `show`, `contextual`, `tab_label` | tab strip, contextual captions, collapse | drive from `spec.rs` instead of a match per tab |
| `ribbon/qat.rs` | quick access toolbar with right-click Add / Remove | |
| `ribbon/title_bar.rs` | title bar on macOS / Windows | |
| `icons.rs` | vector icon enum drawn with the painter | not ported: CourseCraft keeps its Phosphor icons (re-subset the font for the ribbon's glyphs) or moves to an SVG set through `craft-ui::icons`; one style only |

DeckCraft's tab-specific modules (`home.rs`, `insert.rs`…) are PowerPoint tabs; use them only as
examples of wiring, not as content.

## 3. Enable rules

A control is enabled when **all** hold:

1. its action is `Command` or `Ui`;
2. the tab's `when` holds (e.g. a slide is open);
3. for `Command`: the registry's `enabled(session)` returns ok (same check the menu uses today,
   `menu::adapter_reason` gives the disabled reason);
4. for `Ui`: the action's own check (e.g. preview needs at least one slide);
5. not blocked by a mode (state editing, master view) — reuse the reasons already in
   `toolbar.rs` ("Not available in Master View", "Not available inside a state").

Disabled controls always show a tooltip with the reason. Hover on enabled controls shows the
label, the shortcut and one sentence of help.

## 4. Narrow windows

Storyline-style ribbons shrink groups from the right: large buttons become small, then a group
collapses into one button with its caption that opens the group as a popup. Implement in that order
with a measured width per group; never wrap to a second row and never clip. Minimum supported
window width stays as today (`toolbar.rs` thresholds 1100/1220 px show labels).

## 5. Tests

In `crates/ui-egui/src/ribbon/spec_tests.rs` (debug test builds only, like the other guard tests):

- every `Command(id)` exists in `coursecraft_engine::registry()`;
- every `NotYet`/`NotPlanned` has a non-empty reason;
- item ids are unique; no tab is empty; Publish is the last group of every main tab;
- every command the old menu bar and toolbar reached is reachable from the ribbon or File menu
  (guards the removal of the menu bar);
- tab order matches the documented strip (`ribbon.md` › Tab strip);
- every ribbon string is looked up through `tl!`/`tid` (the pseudo-locale test in
  localization.md marks them all).

UI tests with `egui_kittest` (as the existing `*_tests.rs`): a disabled control shows its tooltip and
does not dispatch; a contextual tab appears for a picture and goes when it is deselected; the ribbon
returns to Home when the current contextual tab goes.

A small `cargo xtask ui-spec` (optional) can print the spec as Markdown tables so the docs can be
diffed against the code.

## 6. Order of work (M30)

Each step is one commit, `cargo xtask ci` green, docs status updated in the same commit.

0. Foundation: add `craft-ui` and `craft-i18n` (pinned rev), the egui fork patch, the
   `build.rs` that embeds `CRAFT_I18N`, the `tl!` macro and the Preferences › Language setting.
   Gate and web player checked before and after the fork. **Built**, except the web player check
   in a browser.
1. Workspace on `craft-ui` docking and panes with today's panels inside; then port the ribbon
   widgets and draw the tab strip + Home from `spec.rs`, side by side with the old toolbar behind
   a View › Ribbon toggle. **Ribbon built first** (it doesn't depend on docking): the tab strip,
   Home (Slide and Course Map variants) and View › Ribbon, off by default. Controls whose
   feature exists but needs the text-selection or inspector plumbing are drawn disabled with
   "ribbon wiring (use the Format inspector)" until step 2. Icons use today's Phosphor subset;
   Paste and Cut need better glyphs (clipboard, scissors) in the next re-subset.
   **Docking built** next (`dock.rs`, View › Docked Panels, off by default): slides and
   Inspector in start and end docks, the protected canvas over the timeline between them; bodies
   are the same `Editor::*_body` functions the fixed panels call, so both paths stay in step.
   DeckCraft uses only `craft_ui::ComboBox`; CourseCraft is the first Craft app on
   `craft_ui::docking`. It was chosen over `craft_ui::panes` (the suite's pick for Office-style
   apps) because Storyline stacks panels (Triggers over Slide Layers) and tabs them (Timeline,
   States, Notes), which panes cannot. Still open: saving the workspace in prefs (`serde`
   feature), View toggles that close and reopen panels, the Triggers, Slide Layers, States and
   Notes panels, and switching the default once the fixed-panel tests move over.
2. Fill every main tab from ribbon.md with the right `Action`s (most ✅ wire to existing commands;
   the rest are `NotYet`).
3. Contextual tabs: Drawing Tools, Picture Tools, Audio/Video Tools, Table Tools, Question Tools;
   modes: Slide Master, Preview.
4. File menu and Help tab take over the menu bar items; delete the menu bar and the flat toolbar;
   QAT.
5. View tabs row (open slides as tabs), status bar parity.
6. Panels: right panel Triggers above Slide Layers; Slide Properties in the Course Map; undock.
7. Dialogs that are only missing a front-end over existing commands (Trigger dialog form, Insert
   Slide dialog, Player Properties ribbon, Publish list).
8. Then switch ⛔ items on one feature at a time, in the order users ask for them.

## 7. Rules that still apply

- Never crash: no `unwrap`/`expect` in the ribbon code; widths and counts from the spec are
  trusted, everything from the course is not.
- Never break wasm: the ribbon draws in the web editor too; file actions keep their
  desktop-only reasons (`FILE_OPS_DISABLED_MSG` in `menu.rs`).
- Everything is a command: a button that edits the course dispatches an engine command; if none
  exists, add it to `coursecraft-engine` first.
- Clean-room: own icons and colours; labels per README › Clean-room.
- Every new user-facing string goes through `tl!` (or `tid`, `trn`, `fmt`); English only in the
  code, translations only in craft-i18n (localization.md).
