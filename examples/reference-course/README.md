# CourseCraft Reference Course

A comprehensive, self-contained reference course exercising every runtime feature, object kind, trigger event, action, condition, comparison operator, question kind, and question bank draw in `coursecraft-course` and `coursecraft-logic`.

## Feature Index

- **Course Root**: `theme` (`palette`), `text_styles`, `quiz`, `background_audio`, `resources` (`url` & `file`), `glossary`, `masters`, `templates` (stripped on publish), `player` (`navigation: restricted`, `logo`, `show_transcript`), `strings`.
- **Scene 1 (`sc-core`)**:
  - `s01-intro`: all 12 static built-ins, `%%` escape, `timeline_starts`, `timeline_reaches` (ms & `SlideCue`), `timeline_ends`, `click`, `hover_start`, `hover_end`, `state_changes` (specific & any), `variable_changes`, `change_state`, `set_variable` (all 6 `VarOp`s + var-to-var), `open_url`, `run_javascript`, `disabled: true` trigger, `all`/`any`/`not`/`object_state`/all 9 `CmpOp`s, `next_requires: timeline_ends`.
  - `s02-layers`: `l-modal`, `l-overlay`, `l-pause` exercising all 7 layer options, `show_layer`, `hide_layer` (self & named), `pause_timeline`, `resume_timeline`, `next_requires: variable`.
  - `s03-inputs-widgets`: `text` (with `Link::Url` & `Link::Slide`), `shape` (gradient + `ShapeText`), `image` (crop + state `src`), `button`, `path` (picture fill), `table` (merged cells + `header_row`), `web` (`WebSource::Folder`), `text_entry`, `slider`, `dial`, `checkbox`, `radio_group`, `marker`, `scroll_panel`, `progress_meter`.
  - `s04-motion-drag-timer`: all 5 entrance effects, exit effect, relative & absolute `motion_paths`, `move_on_path`, `animation_completes`, `drag` (`dropped_on`, `objects_intersect`, `objects_stop_intersecting`), `key_pressed` (global & focused), `timer` (`start_timer`, `pause_timer`, `reset_timer`, `timer_ends`, `%Timer.t_main.Remaining%`).
  - `s05-media-cues`: `audio` (trimmed + VTT), `video` (poster + VTT + pausing & non-pausing cues), `lottie` (non-looping), `Course.BackgroundAudio`, `play_media`, `pause_media`, `stop_media`, `cue_reached`, `media_ends`.
  - `s06-branch-lightbox`, `s06b-branch-alt`, `s06c-lightbox`: `jump_to_slide` (`next`, `previous`, `last_visited`, `id`, `scene`), `lightbox_slide`, `close_lightbox`.
- **Scene 2 (`sc-quiz`)**:
  - `s07-q-mc` (`MultipleChoice`, `PerAnswer`, `try_again_layer`), `s08-q-mr` (`MultipleResponse`, `Partial`, `Submit` event), `s09-q-tf` (`TrueFalse`, `AllOrNothing`, unlimited attempts), `s10-q-fill` (`FillIn`, `case_sensitive`), `s11-q-seq` (`Sequence`), `s12-q-match` (`Matching`), `s13-q-hotspot` (`Hotspot`, 2 markers), `s14-q-drag` (`DragDrop`, distractor), `s15-q-freeform` (`Freeform`, `revisit: reset`), `s16a-likert`, `s16b-short`, `s16c-essay`, `s17-draw-1` (2 from `bank-pool`), `s18-draw-2` (1 from `bank-pool`).
- **Scene 3 (`sc-end`)**:
  - `s19-results`: `submit_results`, `retry_quiz`, `review_quiz`, conditional `jump_to_slide`, `exit_course`.
  - `s20-cert`: `print_slide`, `complete_course`, `exit_course`.

## Commands

```sh
cargo run -p coursecraft-cli -- validate examples/reference-course/course.json
cargo run -p coursecraft-cli -- simulate examples/reference-course/course.json examples/reference-course/pass.inputs.json --strict
cargo run -p coursecraft-cli -- simulate examples/reference-course/course.json examples/reference-course/fail.inputs.json --strict
cargo run -p coursecraft-cli -- simulate examples/reference-course/completion-only.json examples/reference-course/completion-only.inputs.json --strict
```
