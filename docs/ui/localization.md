# Localizing the editor interface

> Part of [the ribbon plan](README.md). **Decision (2026-11-23, owner):** the editor's own
> interface is localized with **`craft-i18n`** (runtime in `storytold/craft-libs`, catalogs in
> the `craft-i18n` data repository), as the other Craft apps are. Built together with the UI
> rewrite (M30 step 0) so every new string goes through it from the start.

## What this covers, and what it doesn't

| Text | Localized by |
|---|---|
| Editor interface: ribbon, menus, panels, dialogs, tooltips, messages, Preferences | **craft-i18n** (this file) |
| Engine command labels shown in the UI, the palette and tooltips | **craft-i18n**, keyed by command id (`tid`) |
| Numbers in the interface (X, Y, sizes, durations, file sizes, counts) | **craft-i18n** `Format` via `craft-ui::localized` |
| The author's course content (slide text, questions, feedback) | XLIFF export/import and Translate with AI, as today. Not craft-i18n |
| The learner player's built-in labels (Next, Prev, Submit, Menu…) | the course's text labels, chosen by course language. Not craft-i18n for now |
| CLI and MCP output, logs, error codes | English (machine-facing) |

## How it works

- The catalogs are tab-separated files in the craft-i18n data repository:
  `locales/<locale>/common.tsv` (strings shared by all Craft apps) and
  `locales/<locale>/coursecraft.tsv` (CourseCraft's own). 49 locales exist for other apps;
  CourseCraft has none yet.
- They are an **optional build input**, never a Cargo dependency. `apps/coursecraft/build.rs`
  (or the UI crate's) reads `CRAFT_I18N_DIR` and writes one static into `OUT_DIR`:
  `pub static CRAFT_I18N: &craft_i18n::Embedded`, one entry per locale that has a
  `coursecraft.tsv`. Unset, it is empty and the app is English only; that is how CI and
  contributors build. Releases set `CRAFT_I18N_DIR` and `CRAFT_I18N_REQUIRED=1` so a release
  never ships English-only by accident. Recipe: craft-i18n `docs/integration.md` (follow it
  exactly: absolute paths, `{:?}` quoting, rerun-if lines; wasm embeds the same locales).
- At start-up the UI builds one `I18n` from `CRAFT_I18N` and **CourseCraft's own list of offered
  languages**. Only offered languages appear in the picker; an embedded locale that is not offered
  still serves as a fallback (`pt` for `pt-br`).
- Lookup falls back `pt-br` app → `pt-br` shared → `pt` app → `pt` shared → English. A missing
  translation shows the English; nothing fails.

```rust
// crates/ui-egui/src/i18n.rs
include!(concat!(env!("OUT_DIR"), "/craft_i18n.rs")); // pub static CRAFT_I18N

pub static I18N: LazyLock<craft_i18n::I18n> = LazyLock::new(|| {
    let (i18n, problems) = craft_i18n::I18n::new(CRAFT_I18N, OFFERED);
    problems.iter().for_each(|p| log::warn!("translation catalog: {p}"));
    i18n
});

thread_local! { static CURRENT: Cell<craft_i18n::Lang> = const { Cell::new(craft_i18n::Lang::EN) }; }

macro_rules! tl { ($s:expr) => { $crate::i18n::I18N.t($crate::i18n::CURRENT.get(), $s) }; }
```

## Rules for code

1. **English in the code, translations only in craft-i18n.** A PR that adds or changes
   interface text adds the English only; it never blocks on translations.
2. **Every user-facing string goes through a lookup:** `tl!("Save")` (plain), `tc` with a context
   when one English word has two meanings (`"Open"` the verb vs. a state), `tid(lang, id, label)`
   for engine command labels, `trn` for counts ("1 slide" / "{n} slides"), `fmt` for `{name}`
   placeholders. Never build a sentence by concatenating pieces; one message, one lookup.
3. **Placeholders** are `{name}` and must survive translation; wrap file names, paths and codes in
   `ltr()` before passing them so right-to-left text can't reorder them.
4. **Numbers** shown or edited in the UI use the region `Format` (`craft-ui::localized`
   `NumberField` / `LocalizedDragValue`). Numbers written into the course file stay plain (`trn_plain`).
5. **No layout by English width.** Ribbon groups and buttons size from the measured text (German
   is about 30% longer); never fixed pixel widths for labels. The ribbon's shrink rules
   (implementation.md › Narrow windows) handle the rest.
6. **Keyboard shortcuts stay** on the same physical keys in every language; menu hints use
   localized key names (`get(lang, "key", …)`).
7. **Product and feature names:** "CourseCraft" is never translated. Coined CourseCraft names
   (Course Map, Custom Question) are translated as ordinary words, consistently.
8. **Clean-room for translations:** entries are written from the plain meaning of the English in
   ordinary computing vocabulary; no other product's localisation or string table is consulted
   (as the craft-i18n catalog headers require).

## Languages and scripts

- **Offered list** (`OFFERED` in `i18n.rs`): start with the locales that get a `coursecraft.tsv`
  first; suggested order de, fr, es, it, pt-br, nl, pl (Latin script, left to right, covered by
  the bundled Inter font).
- **Right-to-left** (ar, he, fa, ur…): not offered until the editor mirrors its layout; they stay
  fallback-only. `I18n::rtl(lang)` is the switch to check.
- **Other scripts** (hi, zh, ja, ko, th…): not offered until the UI font stack covers them.
  `craft-ui` `fonts` with the `system_fonts` feature can add installed fonts for missing
  characters; this ties in with roadmap › After the first release › More languages.

## Language setting

Preferences › General › **Language**: "Automatic (system)" (default, `I18N.system_lang()`) or one
offered language (`I18N.languages()`, names from `I18N.name(lang)` in their own language). Saved
as a code (`"auto"`, `"fr"`). Changing it redraws the UI at once; no restart. The web editor
follows the browser language when set to Automatic.

## Testing

- **Pseudo locale:** `Lang::PSEUDO` marks every looked-up string (debug builds). A test draws every
  ribbon tab, contextual tab, panel and dialog in the pseudo locale and fails if any visible text
  is unmarked, which finds strings that skipped `tl!`.
- **`I18N.looked_up()`** lists the English source texts the app used; with `CRAFT_I18N_DIR` set,
  the build prints which have no translation and which translations are no longer used. The report
  never fails the build.
- **`I18N.check_all()`** in a test with `CRAFT_I18N_DIR` set: no malformed rows in
  `coursecraft.tsv`.
- Hand checks: release checklist § 9 › Localization.

## Getting translations

1. The English source texts come from the code (the `looked_up()` report); there is no English
   catalog file.
2. Open a PR in the craft-i18n data repository adding `locales/<locale>/coursecraft.tsv` for each
   language (partial catalogs are welcome), following its `docs/catalog-format.md`. Strings
   already in `common.tsv` (Save, Cancel, Undo…) need no app row.
3. Release builds point `CRAFT_I18N_DIR` at a pinned craft-i18n commit (recorded in
   `docs/maintaining.md` › Releasing); move that pin to a commit that has the new rows. Until
   then new strings show in English.
