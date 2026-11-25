# M46 Animation system (draft plan)

Status: **draft for the owner's review** (2026-11-25). Nothing here is built yet.

This plan replaces the animation half of M42 ("Full entrance/exit gallery, emphasis animations,
animation painter, sequence by paragraph"). M42 keeps the timeline items (fit to longest, Timing
dialog, cue-point key, camera zoom, Course Map).

## Why

- Today there are five effects (`Fade`, `FlyIn`, `Zoom`, `Wipe`, `Float`), four directions, four
  easings, and no emphasis.
- Effects are a closed enum in `coursecraft-course` (`AnimEffect`). Each one is matched by hand
  in `coursecraft-logic::animation`, four times in `ui-egui/src/properties.rs`, and twice in
  `ui-egui/src/ribbon/gallery.rs` (index numbers 1–5). Adding one effect means editing about
  eight places, and missing one fails silently.
- `ObjectSample` can only move, scale evenly, fade and clip to a rectangle. Shape, wheel, bars,
  spin and swivel can't be drawn.
- Effects have no icons, options are plain text in combo boxes, and you can only see an effect by
  previewing the whole slide.

## Reference material

- **Tutorial video** (Storyline 360 webinar, 42 min). Capabilities only: we match what it can do and
  never copy its look (clean-room, `docs/contributing.md`).
  - Entrance gallery of 16: None, Fade, Grow, Fly In, Float In, Split, Wipe, Shape, Wheel, Random
    Bars, Spin, Spin & Grow, Grow & Spin, Zoom, Swivel, Bounce. Exit mirrors it.
  - Effect Options is one menu with sections (Direction, then Sequence, plus Shape, Spokes and
    Origin where the effect has them). Each choice has an icon. Direction has 8 ways.
  - Duration sits next to every slot. The animation also shows as a bar on the timeline row.
  - Bullets build by paragraph, synced to cue points. Motion paths have relative start point,
    orient shape to path, and an "animation completes" trigger.
- **DeckCraft** `crates/anim` (MIT OR Apache-2.0, so we can reuse it with notice and credit):
  - About 30 entrance/exit effects (`effects.rs`) and a set of emphasis effects (pulse, teeter,
    spin, grow/shrink, darken, lighten, desaturate, transparency, wave, bold flash).
  - An easing with acceleration, deceleration and bounce (`easing.rs`).
  - About 45 slide transitions (`transition_fx.rs`). `mesh.rs` builds triangle meshes with a
    projective grid for 3-D cards.
  - `AnimState` already has what `ObjectSample` lacks: `scale_x`, `scale_y`, `rotate` and `tint`.

## Architecture: one effect registry

Every effect is **one entry in one table**. The runtime, renderer, editor gallery, Animate panel,
validator, CLI `commands` and MCP docs all read that table. Adding an effect means adding one
entry and one sample function. A test fails if any part is missing.

```
coursecraft-course (L1)        coursecraft-logic (L2)          coursecraft-render (L3)      ui-egui (L6)
EffectDef  (data only)  ──id──▶ EffectImpl { sample fn }  ──▶   ObjectSample → paint     ◀── gallery, panel,
 id, kind, category,            sample(def, opts, phase,         (transform, tint,            option pickers,
 options schema,                 p, ctx) -> ObjectSample          mask meshes)                 icon renderer,
 defaults, label key                                                                           canvas preview
```

### 1. Model (`coursecraft-course`)

`AnimEffect` stays as `{ "type": ..., ...options }` in JSON, so **existing `course.json` files
open unchanged** (round-trip test on every example). Inside the code it becomes:

```rust
pub struct EffectRef {
    #[serde(rename = "type")] pub id: EffectId,   // validated against the registry
    #[serde(flatten)] pub opts: EffectOptions,    // typed, all optional, deny_unknown_fields
}
pub struct EffectOptions {
    pub from: Option<Direction8>,       // left … bottom_right; today's 4 values still parse
    pub origin: Option<Origin>,         // object centre | slide centre | slide bottom
    pub shape: Option<MaskShape>,       // circle | box | diamond | plus
    pub inout: Option<InOut>,           // in | out
    pub spokes: Option<u8>,             // 1,2,3,4,8 (clamped)
    pub orientation: Option<Orient>,    // horizontal | vertical
    pub amount: Option<f32>,            // degrees / percent; clamped per effect
    pub intensity: Option<Intensity>,   // subtle | normal | strong (emphasis)
    pub sequence: Option<Sequence>,     // as_one | by_paragraph | by_letter
}
```

- `EffectId` is a string newtype, not an enum. If a newer file has an effect this build doesn't
  know, it still opens: the validator gives a warning and the runtime plays it as Fade. It never
  crashes, and it doesn't refuse to open newer files.
- `effects.rs` holds `pub static EFFECTS: &[EffectDef]`. Each entry has the id, kind (entrance,
  exit, emphasis), category (Basic, Subtle, Moderate, Exciting), allowed options with defaults,
  default duration, whether it can build by paragraph, and its `tl!` label key. The validator
  checks every option against the entry. Only options the effect lists are allowed, and numbers
  are clamped (input-derived numbers are hostile).
- `Easing` gains `back` (overshoot), `bounce` and `elastic`, plus `accel` and `decel` numbers
  (DeckCraft's model), clamped to 0..1.
- `Object.emphasis: Vec<Emphasis { effect, at_ms, duration_ms, repeat, easing }>`. It is omitted
  when empty, so old files are unchanged.

### 2. Runtime (`coursecraft-logic`)

- `EffectImpl { id, sample: fn(&EffectOptions, Phase, f32, &Ctx) -> ObjectSample }`, one per
  registry id. A test checks that every `EffectDef` has an impl, and the other way round.
- Exit effects use the same sample function with phase `Exiting`. We don't keep a second list of
  "fly_out"-style ids. The gallery shows exit names ("Fly Out") taken from the def.
- `ObjectSample` gains `scale: (f32, f32)`, `rotation_deg`, `tint: Option<Tint>` and
  `mask: Mask`. The mask is `None`, `Rect(ClipFrac)` or `Polys(SmallVec<ConvexPoly>)`. Every
  reveal shape we need reduces to a list of convex polygons in object-local 0..1 space: circle,
  diamond and plus are polygons, wheel is wedges, bars, blinds, checker and strips are rectangles,
  and dissolve is a seeded set of cells.
- Emphasis: the sampler layers emphasis on top of entrance/exit. Grow/shrink, spin and colour keep
  their end state when they finish. `animation_completes` also fires for emphasis. Spec text goes
  in `docs/spec/triggers.md`.
- Sequence by paragraph/letter: the sampler takes `(index, count)` and staggers progress. The
  renderer passes paragraph or glyph indices.

### 3. Renderer (`coursecraft-render`)

- Apply rotation and non-uniform scale around the object centre (paint already rotates objects),
  then tint (colour multiply/lerp on the draw items).
- **Masks by mesh clipping**: tessellate the object's items as now, then clip the triangles
  against each convex polygon (Sutherland–Hodgman, interpolating position, UV and colour). This
  works the same on native, wasm and the offscreen rasteriser. It needs no render-to-texture and
  covers text glyphs too. Cost is bounded: at most 64 polygons per mask, and over that it falls
  back to opacity.
- Parity tests: `render_slide_rgba` snapshots of every effect at p = 0, 0.5 and 1.

### 4. Editor (`coursecraft-ui-egui`)

- **Galleries and panel are generated from the registry.** The hand-written matches in
  `properties.rs` and `gallery.rs` go away.
- **Effect icons are drawn, not shipped as images.** Each effect's icon is a small glyph (a rounded
  card) run through the *same* sample function at a few progress values. This draws a ghost trail
  for motion effects and the mask shape for reveal effects. Colours come from `DESIGN.md` tokens
  by kind: entrance green, emphasis amber, exit red, motion path blue. On hover the icon plays in
  a loop.
  - Every new effect gets a correct icon for free.
  - Icons follow the dark/light theme and stay sharp at any DPI.
  - The icons always match what the effect does.
  - Nothing to license.
- **Option icons** use Phosphor, which is already bundled under MIT: the eight arrows,
  circle/square/diamond/plus, in/out (`ArrowsIn`/`ArrowsOut`), and sequence (`Paragraph`,
  `TextAa`). Spokes and bars use small drawn glyphs.
- **Animations dock panel**, rebuilt as four cards: Entrance, Emphasis, Exit, Motion Path. Each
  card has:
  - an effect tile (icon + name) that opens a popover gallery with category headers, a search
    field, and Recent;
  - an option row made of icon buttons generated from the schema. Direction is a 3×3 compass
    picker, Shape a segmented row, Spokes a stepper;
  - Duration (slider + field), Delay and Easing (picker drawn as curve icons);
  - ▶ Preview, ⟲ Remove, and "Apply to all selected".

  Animation Painter copies and pastes the whole set (`object.copy_animation` /
  `object.paste_animation`).
- **Live preview on the canvas.**
  - The canvas compositor takes an optional `PreviewOverride { object, slot, effect, started_at }`
    and plays the selected object's animation in place, through the same
    `coursecraft-logic` sampler as the player. The other objects stay still.
  - It plays when you hover a gallery tile (looping), when you change an option (once), or when
    you press ▶.
  - It stops when you click, change selection or edit. An "Auto-preview" toggle in the panel can
    turn it off, and it respects reduced motion.

### 5. Commands, CLI and MCP

- `object.set_animation` keeps its params. `effect` now accepts every registry id and its options.
- New: `animation.effects` (lists the registry as JSON: ids, kinds, options, defaults, so agents
  discover effects instead of guessing), `object.add_emphasis`, `object.set_emphasis`,
  `object.remove_emphasis`, `object.copy_animation`, `object.paste_animation`, and
  `slide.set_transition` with the new transition ids.

## Effect catalogue (target)

Entrance (exit mirrors each one):

| Category | Effects | Options |
|---|---|---|
| Basic | Appear, Fade, Fly In, Float In, Wipe, Split, Zoom, Grow | direction ×8, origin, in/out, orientation |
| Subtle | Rise Up, Peek In, Expand, Shape, Dissolve, Blinds | shape (circle/box/diamond/plus), in/out, orientation |
| Moderate | Wheel, Random Bars, Checkerboard, Strips, Spin, Swivel | spokes, orientation, amount |
| Exciting | Bounce, Spin & Grow, Grow & Spin, Pop (back easing), Drop (elastic), Flip | direction |
| Text (CourseCraft extra) | Typewriter, Word by word | sequence |

Emphasis: Pulse, Teeter, Spin, Grow/Shrink, Transparency, Darken, Lighten, Desaturate, Colour
pulse, Bold flash, Wave (text). Three of these are CourseCraft extras for eLearning feedback:
**Shake** (incorrect answer), **Heartbeat** (call to action) and **Glow pulse** (uses the
existing object glow).

Transitions: today Fade, Push and Cover. Added: Fade through black, Uncover, Wipe, Split,
Reveal, Zoom, Shape (circle/diamond/plus), Clock, Dissolve, Random Bars, Blinds, Checker. Later,
DeckCraft's mesh-based 3-D ones (Cube, Flip, Gallery, Doors). They use the same registry pattern
(`TransitionDef` / `TransitionImpl`) and the same mask meshes.

## Phases (each one a commit series that passes `cargo xtask ci`)

1. **Registry, no visible change.** `EffectId`/`EffectOptions`/`EFFECTS` and the impl table. The
   five current effects move into it. `ObjectSample` gains the new fields. Galleries and panel
   read the registry. Round-trip tests on every example file. `animation.effects` command.
2. **Renderer.** Rotation, x/y scale, tint, convex-mask mesh clipping (DeckCraft `mesh.rs`
   credited), and snapshot tests.
3. **Effect pack 1.** The 16 + text entrance/exit effects, 8 directions, all options. Spec and
   parity tables updated.
4. **Icons.** The procedural effect-icon renderer and Phosphor option icons in ribbon galleries
   and Effect Options menus.
5. **Animations panel and canvas live preview.**
6. **Emphasis.** Model, runtime, timeline bars (amber), trigger spec, Animation Painter.
7. **Sequence by paragraph/letter** and the new easings.
8. **Transitions pack** with option icons, and Apply to All.

Each phase updates `docs/spec/triggers.md`, `docs/ui/ribbon.md` and `docs/ui/feature-parity.md`
(with counts) where it changes behaviour or UI, as CLAUDE.md requires.

## Notes from DeckCraft

- DeckCraft's galleries draw one star per class in code: green for entrance, gold for emphasis,
  red for exit, and a dotted path for motion paths. We take the colour-by-kind idea, but each of our
  icons also shows its own effect's motion, because the same star on every effect doesn't help
  tell them apart.
- `deckcraft/crates/model/src/anim.rs` lists a full entrance, emphasis and exit catalogue with each
  effect's option values. Use it as a checklist for phase 3 (credit it if any code is taken).

## Phase 2 as built

- `ObjectSample` has `scale: (f32, f32)`, `rotation_deg`, `tint: Option<Tint>` and
  `mask: Option<Mask>`. The rectangular `clip` stays as a fast path, and `Mask` is an
  `Arc<[ConvexPoly]>`.
- `compose::apply_sample_to_items` (shared by the player and the canvas timeline preview) moves,
  scales and turns the rect, and stores scale, tint and the placed mask in `DrawItem::anim`.
- `paint_items` lays content out at `rect / scale`. `mesh_post::MeshPost` then scales, rotates,
  flips, clips to the mask and tints each tessellated mesh. It is our own code; nothing came from
  DeckCraft's `mesh.rs`.

## Phase 4 as built

- `ribbon/effect_art.rs` draws an effect icon on a 120×90 stage with a 48×36 card. The still
  icon is a faint ghost at 25 % (only when the effect moves, scales or turns) and the card at
  60 %, drawn through its mask and clip. On hover it plays from 0 to 1 over a second, then holds.
  Opaque mask pieces get a hairline edge in their own colour to hide seams.
- Option icons use the bundled Phosphor subset. Diagonal arrows, the diamond and vertical bars
  are the same glyphs turned (`TextShape::with_angle`), so the font did not need re-subsetting.
  Spokes are a drawn wheel.
- The Effect Options menu (`UiMenu::EffectOptions`) and `gallery::set_option` set a value only on
  selected objects whose effect takes it, checked with `EffectDef::option_problems`.

## On stock icons from Magnific / Freepik

Searching the stock library for animation icons returned a mixed, mostly premium set ("Sputnik",
"Golf ball", "Tool"…) with no common style. Freepik's licence also doesn't allow redistributing the
icons as part of an MIT/Apache app, where anyone can take the files out. So we **don't ship stock
icons**. Magnific is still useful for drawing a style reference while we tune the procedural
glyph, or for marketing art. Anything shipped is drawn in code or comes from Phosphor (MIT).

## Open questions for the owner

1. Milestone: is this M46 (taking the animation items out of M42), or do we fold it into M42?
2. Proceed with procedural effect icons instead of stock icons?
3. Should phase 1 start now, with the phases following in order?
