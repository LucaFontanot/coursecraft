# Healthy Habits at Work

The sample course CourseCraft opens on a fresh launch. It is a short wellbeing module, and every slide shows off a different part of the app, so a new author can open it, press F12 and see what CourseCraft can do.

36 slides in 7 scenes: Welcome, Move, Breathe, Connect, Check your knowledge, Reflect and finish, and Extras.

## What it shows

| Area | Where to look |
|---|---|
| Custom font (DM Serif Display from Google Fonts, SIL OFL) | Every heading. Course Settings ▸ Fonts; the file is `media/fonts/` with its licence |
| Background music across the whole course | Course Settings ▸ Background audio (`music.mp3`, 12% volume, loops) |
| Theme, palette, named text styles, slide master | Every slide. Course Settings ▸ Theme; the Heading, Eyebrow and Body styles; the `Content` master's accent strip |
| Entrance and exit animations (fade, fly in, zoom, wipe, float) | Welcome, Three small habits, Three quick resets, Well done; the desk hint leaves on its own (`timing.end_ms` plus an exit animation) |
| Objects timed to narration (staggered timeline) | Welcome, Three small habits and each habit's layer, Set up your desk, Rest your eyes, A healthy day, Your results. Open the Timeline panel to see each object start on its spoken words |
| Slide transitions (fade, push, cover) | Every slide |
| Hover states and custom states | Every button; the habit cards; the stretch checklist; the sort cards |
| Layers (modal, toast, feedback) | Three small habits, Three desk stretches, Rest your eyes, every question |
| Variables, `%var%` text, conditions | Your name everywhere; habits explored; tips opened; cards sorted; stress and sleep check-in |
| Text entry, slider, dial, checkbox, radio group | Nice to meet you, How are you today?, Make your plan, Question 9 |
| Progress meter (ring and bar) | Three small habits, How are you today? |
| Markers | Set up your desk |
| Video with cue points and captions | Three desk stretches |
| Audio narration with captions | 12 short clips: Welcome, Three small habits (and audio inside each habit layer), desk, eyes, day, breathing, results (passed and failed layers), Well done |
| Lottie animation | Breathe with the circle |
| Motion paths and `animation_completes` | Breathe with the circle (the floating dot) |
| `timeline_reaches`, `timeline_ends`, pause and resume timeline | Breathe with the circle |
| Layer that pauses the base timeline, layer that hides when its timeline ends | Habit layers; the Rest your eyes toast |
| Timer with start, pause and reset | Rest your eyes |
| Lightbox | Rest your eyes ▸ Why 20-20-20? |
| Draggable objects, `dropped_on`, object intersections | Helps or hurts? |
| Key presses | Enter on the Welcome slide, Escape on the habit cards |
| Ready-made interactions | Flip cards, process steps, tabs, scenario, timeline, accordion, reveal grid, hotspot reveal |
| Table | Make your plan |
| Scrolling panel, web object, Open URL, slide link | Read more |
| Picture fill on a shape, image crops, opacity, shadows, glow, rotation, groups, paths | Explore a healthy office, Welcome, Rest your eyes, Well done |
| All 12 question types | Questions 1 to 9 (graded) and Reflections 1 to 3 (not scored): multiple choice, multiple response, true/false, fill-in, sequence, matching, hotspot, drag and drop, freeform, Likert, short answer, essay |
| Question bank and random draw | Quick checks bank (3 questions); one is drawn after Question 9 |
| Results, retry, review and certificate | Your results, Your certificate (with print) |
| Per-slide player rules | No seekbar on graded questions; Next waits for the welcome narration |
| Slide template | Lesson page (New slide ▸ Your templates) |
| Keyboard focus order, text that shrinks to fit | Welcome; the flip cards |
| Glossary, resources, transcript (slide notes), captions | The player menu |

## Checks

```sh
cargo run -p coursecraft-cli -- validate examples/healthy-habits/course.json
cargo run -p coursecraft-cli -- a11y-check examples/healthy-habits/course.json
cargo run -p coursecraft-cli -- simulate examples/healthy-habits/course.json examples/healthy-habits/pass.inputs.json
```

`pass.inputs.json` plays the whole course and scores 100%.

## Media

The background music was made with CassetteAI on fal.ai. The pictures were made with an AI image generator, the narration with ElevenLabs (twelve clips of 2 to 7 seconds), and `stretch.mp4` with ffmpeg from three of the pictures (Google Flow prompts for real clips are in `flow/prompts.md`). `breathe-circle.json` is a hand-written Lottie file. Captions are WebVTT files next to their media.

## Rebuilding

`build.py` builds `course.json` from engine commands plus direct JSON edits. Run it from the repository root after `cargo build -p coursecraft-cli`:

```sh
python3 examples/healthy-habits/build.py
```
