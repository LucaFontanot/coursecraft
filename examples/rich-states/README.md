# Rich Object States Example (`examples/rich-states`)

Clean-room showcase course exercising M29 Rich Object States across authoring, runtime, rendering, XLIFF localization, and all 6 LMS/Web publish targets.

## Slides

1. **`s01-character` (Expressive Character):** `Image` object with `hover` (`src` swap + `glow` + overhanging `Shape` & `Text` speech-bubble `extras`), `down` (`rotation: 3.0`), and `selected` (`src` swap + overhanging star `Path` extra).
2. **`s02-checklist` (Visited Checklist & Disabled Unlock):** Three topic buttons with `visited` outline + overhanging checkmark `Shape`+`Path` extras and `hover` `glow` layered over `visited`. `Continue` starts `disabled` (`opacity: 0.45`, ignores hover/click) and unlocks to `normal` once all three are `visited`.
3. **`s03-cards-keep` & `s03b-cards-reset` (Interactive Flip Cards & Revisit Modes):** Cards swapping linear gradient `fill_style`, dashed `outline`, `shadow`, bold `text_style`, rich `runs`, and an `Image` info badge extra (`badge-info.svg`) on `selected`, contrasting `revisit: "keep_saved"` vs `revisit: "reset"`.
4. **`s04-question` (Custom Freeform Question):** Graded by `Condition::ObjectState` on `s04-ans-1` and `s04-ans-2` (`selected`) with state extra text (`"Chosen"`) covered by XLIFF export/import.
5. **`s05-done` (Completion):** Submits results on `timeline_starts`.

## Commands

```sh
cargo run -p coursecraft-cli -- validate examples/rich-states/course.json
cargo run -p coursecraft-cli -- simulate examples/rich-states/course.json examples/rich-states/pass.inputs.json --strict
cargo test -p coursecraft-cli --test m29_rich_states
```
