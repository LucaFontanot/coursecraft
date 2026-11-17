# M20 Everyday authoring (release 0.2)

Four features authors expect from a desktop authoring tool: spell check, any installed font, pasting images and an eyedropper, and ready-made interactions. None changes play-time behaviour: interactions are built from objects, states, layers and triggers the runtime already has, so `docs/spec/triggers.md` stays as it is.

Clean-room: match what authors can do, never another tool's look, wording or files. The interactions are our own designs, drawn from the theme. Nothing here is on the patent watch list in `docs/contributing.md`.

Implementer: Hy 4 Flash Free through OpenCode, one step per run, with the file map in each brief. Opus plans and reviews.

## Steps

### Step 1. Any installed font (S)

- Desktop: list the fonts installed on the computer with `fontdb` (in `apps/coursecraft`, not in `ui-egui`, so the web build is unaffected). Only `.ttf` and `.otf` files are offered; collections (`.ttc`, `.otc`) and variable fonts are skipped in this step. Group faces by family.
- The UI asks for the list with a new `EditorRequest::ListSystemFonts` and gets it back as a field on the editor (`editor.ui.system_fonts: Option<Vec<SystemFontFamily>>`, each with a name and the paths of its regular, bold, italic and bold-italic faces). The web build answers with an empty list.
- The font picker in `crates/ui-egui/src/properties.rs` (~line 2755) shows the course fonts first, then an "On this computer" section with a search box. Picking a family that isn't in the course yet adds every face of it with the existing `Editor::import_font` (`crates/ui-egui/src/lib.rs` ~2044), then applies the font.
- Embedding permission: read the OS/2 `fsType` field. If the maker restricts embedding (`fsType & 0x0002`), still add the font, and show a one-line note in the status bar: "This font's maker restricts embedding. Check your licence allows using it in a published course." The Fonts window (`crates/ui-egui/src/fonts.rs`) shows the licence text from the name table (ID 13, first 200 characters) under each family, and the same note where it applies. Put the `fsType` and licence readers in `coursecraft-media` next to `font_info`, with tests.
- Done when: an installed family can be chosen from the picker, it is copied into the course and published, restricted fonts show the note, and the web build still compiles.

### Step 2. Paste images (S)

- Desktop: Cmd/Ctrl+V with an image on the system clipboard inserts it as a picture, centred on the slide and scaled down to fit inside it if it's bigger, then selected. Read the clipboard with `arboard` (desktop only, in `apps/coursecraft`), encode it as PNG and pass it to the existing `Editor::import_image` (`lib.rs` ~1798) as `pasted-image.png`.
- Order: if the clipboard has our own object JSON, paste objects as now (`crates/ui-egui/src/clipboard.rs`); else if it has an image, paste the image; else paste text as now.
- Also make sure copying and pasting text and objects reaches the system clipboard on the desktop: `eframe` is built without default features, so check that egui-winit's `clipboard` feature is on, and turn it on if it isn't.
- Web: unchanged (browsers don't hand images to the canvas this way). The Edit menu's Paste works as before.
- Done when: a screenshot copied to the clipboard pastes onto the slide as a picture on macOS and Windows, with an undo step, and object copy and paste still works.

### Step 3. Eyedropper (S)

- Every colour picker (the shared one used by colour fields) gets an eyedropper button. Clicking it enters pick mode: the cursor becomes a crosshair, a small swatch follows it showing the colour under it, a click anywhere in the editor window applies that colour, and Esc cancels.
- Read the pixel from an egui screenshot (`ViewportCommand::Screenshot`, then the `Event::Screenshot` image), which works on the desktop and the web. Convert from screen pixels to the image's pixels using `pixels_per_point`.
- The picked colour goes through the same command as choosing a swatch, so it is one undo step, and it is added to the recent colours.
- Done when: picking a colour from a picture on the canvas sets a shape's fill to that colour, and Esc leaves pick mode without changing anything.

### Step 4. Spell check engine (M)

- A new crate `crates/spell` (`coursecraft-spell`), layer 0, standalone, registered in `xtask/src/main.rs`. It wraps `spellbook` with an English (US) Hunspell dictionary bundled in the crate (`dictionaries/en_US.aff` and `.dic`, with their licence file and a row in `ATTRIBUTION.md`).
- API: `Speller::english()`; `misspellings(&self, text: &str) -> Vec<Misspelling { start, end, word }>` (byte ranges); `suggestions(&self, word: &str) -> Vec<String>` (at most 5); and a personal word list (`add_word`, `remove_word`, `words`) that `misspellings` respects.
- The word splitter skips: words with digits, all-caps words of 2+ letters (acronyms), URLs and email addresses, `%name%` variable references, and single letters. Apostrophes inside words are part of the word (`don't`). Case: "Hello" at the start of a sentence is fine when "hello" is in the dictionary.
- Works in wasm: no file reading, dictionary loaded from the bundled bytes once.
- Done when: unit tests cover the splitter rules, suggestions for common typos (`teh`, `recieve`), the personal list, and a 5,000-word text checks in well under 50 ms in release.

### Step 5. Spell check in the editor (M)

- As you type: in the canvas text editor (`crates/ui-egui/src/canvas.rs` ~4240) and the inspector's text box (`properties.rs` ~3065), misspelled words get a red wavy underline, drawn from the text edit's galley. Right-clicking one shows up to 5 suggestions, "Add to Dictionary" and "Ignore". A suggestion replaces the word in the buffer being edited.
- Check Spelling (Review menu, F7): a window that walks every author-visible string with `coursecraft_course::course_strings` (`crates/course/src/strings.rs`), lists each misspelling with its slide and field, and offers Change (from the suggestions or a typed word), Change All, Ignore, Ignore All and Add to Dictionary. Changes go through `course.replace` with `only` set to that location (whole word, case-sensitive), so each is one undo step.
- The personal dictionary is saved in the desktop preferences (`apps/coursecraft/src/prefs.rs`) and in local storage on the web. A Preferences checkbox turns "Check spelling as you type" on and off (default on).
- Done when: typos are underlined while editing on the canvas and in the inspector, suggestions replace the word, the course-wide check walks every slide, and added words stay after a restart.

### Step 6. Ready-made interactions, part 1 (M)

- Extend `interaction.insert` (`crates/engine/src/commands.rs` ~8904, beside `tabs` and `accordion`) with three new kinds, each sized from the slide and coloured from the theme, every part a normal editable object:
  - `flip_cards`: three cards, each with a front and back state (title on the front, detail text on the back). Clicking a card toggles its state. A "Flip all back" isn't included.
  - `hotspot_reveal`: a picture placeholder with three numbered hotspot markers; clicking a marker shows its callout layer (one at a time), with a close button on each callout.
  - `process_steps`: four numbered steps in a row with connecting lines; clicking a step highlights it (selected state) and shows its detail layer; Next and Back buttons move between steps using a number variable created for the interaction (`<interaction>_step`).
- An Insert ▸ Interaction gallery window lists every interaction (the existing tabs, accordion, markers and scrolling panel plus the new ones) as cards with a small drawn preview and a one-line description. It replaces the separate menu items; the old commands keep working.
- Tests: for each new kind, insert it, check the objects, layers, states and triggers it made, and play it through `coursecraft-logic` (click a card, a marker, a step) to show it behaves as described. The course stays valid after each insert, and undo removes everything in one step.

### Step 7. Ready-made interactions, part 2 (M)

- Three more kinds:
  - `timeline`: five dated points on a line; clicking a point shows its detail panel (layer) and marks the point visited (a visited state).
  - `reveal_grid`: a 2×3 grid of tiles; clicking a tile reveals its text (a state) and it stays revealed; a counter text shows "3 of 6 explored" using a number variable and `%name%` text.
  - `scenario_choice`: a short situation, three choices; each choice shows its own consequence layer with feedback text, and a "Try again" button on the wrong ones hides the layer. The best choice is marked by the layer's text only (no scoring), so it is not a graded question.
- Same tests as step 6, plus the gallery entries.

## Order and checks

Steps 1–3 first (small, independent), then 4–5, then 6–7. Quick crate checks after each step; the full `cargo xtask ci` after steps 3, 5 and 7 (the local gate heats the Mac). One commit per step. A CHANGELOG entry under "Unreleased" with each step, in plain words.

## Not in this milestone

Other languages for spell check (the crate takes any Hunspell dictionary, so this is mostly packaging), grammar checking, variable fonts and font collections, image paste in the browser, an eyedropper that reads outside the editor window, and graded versions of the interactions.
