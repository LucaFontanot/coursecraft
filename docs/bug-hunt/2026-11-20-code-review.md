# Code review, 2026-11-20 (after M24, commit 7c39e0f)

Scope: a read-through of every crate for correctness, security and speed, plus one full
`cargo test` run on Windows 11 (Rust 1.99.1). No source file was changed.

**How to read the Status column**

- **Confirmed**: the code at that location was read a second time and the defect is there.
- **Reported**: one reviewer traced it through the code and its callers; not checked a second time.
- Nothing here was reproduced in the running app, except the test failure in section 1.
  Write a failing test first for each fix (project rule: one behaviour per test).

**Rules that apply to every fix** (from `AGENTS.md` and `CLAUDE.md`): `cargo xtask ci` must pass;
a runtime behaviour change updates `docs/spec/triggers.md` in the same commit; no `unwrap`,
`expect` or `panic!` in non-test code; one commit per working step.

---

## 1. Test failure on Windows

`cargo test` passes for every crate except `coursecraft-video-native`: 3 of 6 tests fail.

```
windows_tests::open_info
windows_tests::frame_at_is_within_one_frame
windows_tests::frame_at_known_time_is_not_blank
  VideoError("frame size: The requested attribute was not found. (0xC00D36E6)")
```

- **Where:** `crates/video-native/src/lib.rs:1321` (`probe_size`), also `:1238-1248` (reader setup)
  and `:1376` (`probe_has_audio`).
- **Cause (probable):** the code uses stream index `0` as the video stream and index `1` as audio.
  Media Foundation does not promise container order. `examples/showcase/media/intro.mp4` has
  video as track 0 in the file, but stream 0 of the source reader has no `MF_MT_FRAME_SIZE`, so it
  is not the video stream there. `decode_audio` (`:1563`) searches for its stream by major type and
  its test passes.
- **Effect:** video does not open in the Windows editor or preview for such files.
- **Fix:** use `MF_SOURCE_READER_FIRST_VIDEO_STREAM` (and `..._FIRST_AUDIO_STREAM`), or search the
  streams by `MF_MT_MAJOR_TYPE` as `decode_audio` does. Deselect all streams, then select the one found.

---

## 2. Data loss and crashes

### 2.1 MCP `file.save` on a `.ccraft` writes no media — Confirmed
- **Where:** `crates/control/src/host.rs:397` (`HeadlessHost::save`).
- **Defect:** calls `save_project(&self.session.course, &[])`. The media list is always empty. The
  write is also `std::fs::write`, not atomic and not verified.
- **Trigger:** `coursecraft-mcp --course x.ccraft` (or `file.open`), then `file.save` with no path.
  `x.ccraft` is replaced by a zip with only `course.json`.
- **Fix:** keep the opened media (it is already in `HeadlessMediaSource.in_memory`) and pass it.
  Write to a temp file, `verify_project`, then rename, as `apps/coursecraft/src/files.rs::save_to` does.

### 2.2 Save on a bare `course.json` writes a zip into the `.json` file — Confirmed
- **Where:** `apps/coursecraft/src/main.rs:590-612` (`save_project_file`); open side in
  `apps/coursecraft/src/files.rs:423-443`.
- **Defect:** when the project is not a folder, save always uses the zip writer. It never looks at
  the extension.
- **Trigger:** open `examples/hello-course/course.json` in the app, edit, press Ctrl+S. The file now
  holds zip bytes and will not open again ("It may be damaged").
- **Fix:** for a `.json` path, write JSON (as the control `save` at `main.rs:2409-2429` does), or
  send the user to Save As.

### 2.3 Panic: ring progress meter in a very small rect — Confirmed
- **Where:** `crates/render/src/paint.rs:2683`.
- **Defect:** `(side * 0.18).clamp(2.0 * scale, side * 0.5)` panics when `side * 0.5 < 2.0 * scale`.
- **Trigger:** a ring-style progress meter with a Zoom entrance or exit animation (the rect is scaled
  from near 0), or a meter frame under 4 px. Player, preview and editor timeline playback all panic.
- **Fix:** `let hi = (side * 0.5).max(0.0); (side * 0.18).clamp((2.0 * scale).min(hi), hi)`; return
  early when `side` is not finite or not positive.

### 2.4 Panic: canvas-menu Paste of a copy wider or taller than the slide — Confirmed
- **Where:** `crates/ui-egui/src/clipboard.rs:470-479` (`placement_delta`).
- **Defect:** the guard is `half_w <= slide_w` but the clamp is `clamp(half_w, slide_w - half_w)`.
  Between 1x and 2x the slide width, min > max.
- **Trigger:** copy a full-bleed 1400 px picture on a 1280 px slide, right-click the canvas, Paste.
- **Fix:** guards become `half_w * 2.0 <= slide_w` and `half_h * 2.0 <= slide_h`.

### 2.5 Panic: timeline end-handle on a bar that starts near the end — Confirmed
- **Where:** `crates/ui-egui/src/timeline.rs:646-662` (`ResizeEnd`).
- **Defect:** `raw_end.clamp(min_end, orig_duration_ms)` with `min_end = start + snap_step`. When
  `start + 100 > duration`, min > max. It runs on the first frame after the press.
- **Trigger:** an object that starts less than 100 ms before the timeline end; press its right handle.
- **Fix:** `let max_end = drag.orig_duration_ms.max(min_end)`; use `saturating_add` for `min_end`.

### 2.6 Panic: `object.set_scroll_panel` with a tall frame — Confirmed
- **Where:** `crates/engine/src/commands.rs:6216`.
- **Defect:** `value.clamp(object.frame.h.max(0.0), MAX_SCROLL_CONTENT_HEIGHT)` panics when
  `frame.h > 20000`. Validation has no upper limit on frame size.
- **Trigger:** scroll panel with `frame.h = 20000.4`, then `object.set_scroll_panel {patch: {content_height: 500}}`.
- **Fix:** clamp the lower bound to the maximum first, or return `Err` when `frame.h > MAX`.

### 2.7 Hang: timeline ruler loop never ends for a huge duration — Confirmed
- **Where:** `crates/ui-egui/src/timeline.rs:538-578` (`render_time_ruler`).
- **Defect:** `while ms_tick <= duration_ms` with `saturating_add`. At `duration_ms == u32::MAX` the
  tick stays at the maximum. Each pass also adds two shapes, so memory grows without limit. A legal
  24 h duration still makes 172,800 passes per repaint.
- **Trigger:** a course file with `"duration_ms": 4294967295`, Timeline tab visible.
- **Fix:** iterate only the visible ms range; `break` when the add does not increase the tick. Cap
  slide and layer duration in validation.

### 2.8 Abort: hostile `.ccraft` through pre-allocation of the declared size — Confirmed
- **Where:** `crates/format/src/lib.rs:417-426` (`open_project_with_limits`).
- **Defect:** `Vec::with_capacity(file.size())` trusts the declared size. The running total counts
  only bytes read.
- **Trigger:** a small zip with 200 deflated entries that each declare 256 MiB. About 50 GB of commit
  is requested on Windows; the allocator fails and the process aborts, with no crash note.
- **Fix:** cap the reserve (for example `min(size, 1 MiB)`), or add the declared size to the total
  before the allocation.

### 2.9 Folder projects lose media when two names differ only by case — Reported
- **Where:** `crates/format/src/folder.rs:384-392`, `460-504` (`put_all`), `551-576`
  (`delete_unused`); name source `crates/ui-egui/src/lib.rs:4207-4252` (`sanitize_media_src`).
- **Defect:** every check is case-sensitive; NTFS and APFS are not.
- **Trigger:** delete `Logo.png`, import `logo.png`, save the folder project. The new file is written,
  then `delete_unused` removes the recorded `Logo.png`, which is the same file on disk.
- **Fix:** reject or rename case-insensitive duplicates; skip a delete whose path equals a wanted
  path ignoring case; compare ignoring case in `sanitize_media_src`.

### 2.10 Control `file.save` on an open folder project writes only `course.json` — Reported
- **Where:** `apps/coursecraft/src/main.rs:2375-2434`.
- **Defect:** the folder writer is chosen from the `.ccraft-folder` extension, not from
  `self.project.folder`. Open accepts any folder with a `course.json`.
- **Trigger:** open the folder `examples/hello-course`; an attached agent adds media and calls
  `file.save`. Only `course.json` is written, `project.folder` is cleared, and the next Ctrl+S tries to
  rename a zip onto a directory (on Windows: access denied, a 2 s freeze, and the "sync service" message).
- **Fix:** when the target is the open folder project, call `save_folder_to` with the existing record.

---

## 3. Wrong behaviour

### 3.1 PowerPoint and Word import drops `&`, `<`, `>` and character references — Confirmed
- **Where:** `crates/pptx/src/lib.rs:549-571` (`parse_xml_tree`), shared with `crates/pptx/src/docx.rs`.
- **Defect:** quick-xml 0.41 reports `&amp;`, `&lt;`, `&#8217;` as `Event::GeneralRef`. The loop handles
  only `Event::Text`; everything else falls into `_ => {}`. `Event::CData` is dropped too.
- **Trigger:** `<a:t>Research &amp; Development</a:t>` imports as "Research  Development".
- **Fix:** add a `GeneralRef` arm (see `crates/format/src/xliff.rs:1017-1024`) and a `CData` arm.

### 3.2 Find and Replace deletes some run formatting — Confirmed
- **Where:** `crates/course/src/strings.rs:951-960` (`all_plain`).
- **Defect:** checks 6 of the 10 `TextRun` override fields. Runs that only use `strikethrough`,
  `baseline`, `letter_spacing` or `font` count as plain and are replaced by `Vec::new()`.
- **Trigger:** "H" + subscript "2" + "O"; replace "H" with "D". The subscript is lost.
- **Fix:** add the four fields, or share one helper with `crates/course/src/text.rs:365-376`.

### 3.3 Replace edits a real `%variable%` when a `%` comes before it — Confirmed
- **Where:** `crates/course/src/strings.rs:741-770` (`variable_reference_spans`).
- **Defect:** it pairs any two `%` signs. In `Save 50% today, %UserName%!` the protected span becomes
  `% today, %` and the real reference is left open.
- **Trigger:** find `User`, replace with `Member`: the text becomes `%MemberName%`, which no longer resolves.
- **Fix:** accept a span only when the name is made of variable-name characters; otherwise move one
  byte forward. Apply the same rule to the XLIFF placeholder check that uses this function.

### 3.4 `diff` ignores named text styles — Confirmed
- **Where:** `crates/course/src/diff.rs:72-110` and `settings_json` at `:892-910`.
- **Defect:** `course.text_styles` is never compared.
- **Fix:** add `d.keyed("text style", &a.text_styles, &b.text_styles, ...)`.

### 3.5 Retry Quiz, Review Quiz and Exit Review do not close an open lightbox — Confirmed
- **Where:** `crates/logic/src/run.rs:3848-3919`. Only `jump` (`:782`) closes the lightbox.
- **Trigger:** a results slide shown as a lightbox with a Retry button.
- **Effect:** the question draws inside the box, Next and Previous are refused, closing the box
  returns to the old base slide, and suspend data saves the wrong slide. With banks the stored
  indices are also stale.
- **Fix:** call `self.close_lightbox(false, out)` at the start of the three functions. Update
  `docs/spec/triggers.md`.

### 3.6 `Player::resume` leaves `entered = false` — Confirmed
- **Where:** `crates/logic/src/lib.rs:859-897`, with `crates/logic/src/run.rs:1425-1428`.
- **Defect:** a player rebuilt from `PlayerState` never saves the slide it leaves. The editor preview
  rebuilds the player for each input (`crates/ui-egui/src/preview.rs:617, 710, 955`); the published
  player keeps one. So `revisit: resume` works when published and not in preview.
- **Fix:** in `resume`, set `p.entered = p.st.started` after `p.st = state`.

### 3.7 Objects and layers on question-bank slides cannot be deleted — Confirmed
- **Where:** `crates/engine/src/commands.rs:7187-7195` (`remove_dangling_triggers`).
- **Defect:** the slide path is built only from `course.scenes`. For a bank slide the validator path is
  `/banks/{bi}/slides/{si}`, so no trigger is removed and `keep_valid` refuses the delete.
- **Trigger:** `bank.add_question`, then `object.delete` on its Submit button.
- **Fix:** also search `course.banks` and use the `/banks/...` prefix.

### 3.8 Control server rejects large requests with non-ASCII text — Confirmed
- **Where:** `crates/control/src/server.rs:295-310` (`read_line_capped`).
- **Defect:** each `fill_buf()` chunk is decoded with `from_utf8` on its own. A multi-byte character
  split across the 8 KiB buffer gives `InvalidData`, and the reply says "line exceeded 16 MiB limit".
- **Fix:** collect bytes in a `Vec<u8>` and decode one time when the whole line is read.

### 3.9 Shortcuts with a modifier fire while typing — Reported
- **Where:** `crates/ui-egui/src/lib.rs:3461-3535`.
- **Defect:** the loop skips a shortcut only when `conflicts_with_text_editing()` is true, which is
  false for any shortcut with a modifier. `text_focus` and `inline_edit` (computed at `:3367`) are not used.
- **Trigger:** double-click a text box, type, press Ctrl+Z: the last course edit is undone, not the
  typing. Ctrl+M adds a slide and the typed text is lost. Ctrl+D duplicates the object being edited.
- **Fix:** skip the registry loop and the redo block when `text_focus` is true; keep an explicit
  allow-list (Ctrl+S and similar).

### 3.10 Edit Trigger window keeps a stale index — Reported
- **Where:** `crates/ui-egui/src/triggers.rs:780`, `1458-1471` (`commit_ok`), list buttons `1880-1955`.
- **Trigger:** triggers A, B, C. Edit B, delete A in the list with the window open, press OK.
  `trigger.update` at index 1 overwrites C with the edited B.
- **Fix:** close or re-map the form when the trigger list changes, or disable the row buttons while
  the form is open.

### 3.11 Canvas drag breaks over the zoom HUD, a ruler or a guide — Reported
- **Where:** `crates/ui-egui/src/canvas.rs:5427-5437` and `5466-5503`.
- **Defect:** an active drag is not processed while `pointer_in_hud` is true (HUD, ruler strips,
  within 4 px of a guide). The object jumps back for those frames. A release there is ignored, and
  the move commits later at the wrong position.
- **Fix:** test `pointer_in_hud` only at the start of a gesture.

### 3.12 A click on a path point moves it and adds an undo step — Reported
- **Where:** `crates/ui-egui/src/canvas.rs:3198-3306` (`PathPoint`), `3308-3369` (`Adjust`),
  `3371-3408` (`MotionPathPoint`).
- **Defect:** these branches return before the 3 px `is_move` test and always commit at the pointer
  position, not the point plus the drag delta.
- **Fix:** store the press offset from the point centre; skip the commit under 3 px.

### 3.13 Duplicate and paste do not remap `Object.panel` and `drag.targets` — Reported
- **Where:** `crates/engine/src/commands.rs:4164` (`copy_slide`), `:3792` (`layer_duplicate`),
  `:5047` (`object_duplicate`), `:5124` (`object_paste`).
- **Effect:** a slide with a scroll panel that has children, or a draggable with targets, cannot be
  duplicated (the copy fails validation). A duplicated panel's children stay attached to the original.
- **Fix:** remap both through `object_map` after the id maps are built; clear a value whose target was
  not copied (the rule `remap_glue` uses).

### 3.14 `object.remove_motion_path` makes a valid course invalid — Reported
- **Where:** `crates/engine/src/commands.rs:12288-12322`.
- **Defect:** removes the path, lists the triggers that refer to it, and returns `Ok` without
  `keep_valid`. Later commands then skip `keep_valid` because the course is no longer valid.
- **Note:** `crates/engine/src/tests.rs:3467-3479` expects the current behaviour.
- **Fix:** remove the referring triggers in the same undo step (as `video.remove_cue` does at `:6699`).

### 3.15 `edit.undo` or `edit.redo` inside a failed batch damages the history — Reported
- **Where:** `crates/engine/src/lib.rs:246-322` (`execute_batch_with_name`).
- **Defect:** the batch snapshot holds only the course, not the undo and redo stacks.
- **Fix:** refuse `edit.undo` and `edit.redo` in a batch, or snapshot the stacks and the revision.

### 3.16 A timer stops for good when its timeline ends — Reported (intent unclear)
- **Where:** `crates/logic/src/run.rs:2673-2752` (`advance_timers`), with `:2836`.
- **Effect:** with the defaults (slide 5 s, timer 60 s) the timer stops at 55 s and `timer_ends`
  never fires. The spec sentence "while its owning timeline runs" may mean this is intended.
- **Fix:** decide the rule. Either keep counting after the timeline ends, or add a validation warning
  when the timer outlasts its timeline. Update `docs/spec/triggers.md`.

### 3.17 A deck with a link to a hidden slide fails the whole PowerPoint import — Reported
- **Where:** `crates/pptx/src/lib.rs:1152-1162` (`slide_parts`), `1193-1200`, `3711-3724`.
- **Defect:** a slide hidden with `<p:sld show="0">` is skipped later, but links to it are kept, so
  validation fails and `import_pptx` returns an error.
- **Fix:** after the slide loop, drop `Link::Slide` values whose target was not imported and add a
  report line.

### 3.18 Slide-size scaling misses some pixel values — Reported
- **Where:** `crates/engine/src/slide_size.rs:65-146` and `:156-185`.
- **Defect:** `course.editor.guides[].at`, `Paragraph.space_before` / `space_after`, and the sizes in
  `course.text_styles` are not scaled.

### 3.19 `go_to` skips the navigation lock while a lightbox is open — Reported (low confidence)
- **Where:** `crates/logic/src/run.rs:344`.

### 3.20 `retry_quiz` with banks uses the old slide index against the new order — Reported (low confidence)
- **Where:** `crates/logic/src/run.rs:3875-3884`. Run the leave steps before `self.slides` is replaced.

### 3.21 Cmd+B with a caret and no selection bolds the whole box — open from the M24 hunt
- See `docs/bug-hunt/2026-11-20-m24b.md`, bug 1. Still open.

### Smaller items
- `crates/engine/src/commands/csv_import.rs:84`: a row of only commas (`,,,,,`) is not treated as
  blank, so the whole import is refused.
- `crates/engine/src/commands.rs:2727-2733`: `slide.set_master` accepts a bank slide id, changes
  nothing and returns `Ok`.
- `crates/control/src/server.rs:331-349`: on Windows the control token is a `DefaultHasher` of the
  pid, the time and a stack address, so it is not random.
- `apps/coursecraft/src/agents.rs:305-316` (not verified): with a `.cmd` shim, `child.kill()` stops
  only `cmd.exe`, so Cancel and the timeout may not return.
- `apps/coursecraft/src/ai.rs:74-78` (not verified): the 16 MiB answer cap runs after the download;
  an error body excerpt is shown without removing the key.

---

## 4. LMS and security

### 4.1 `/lti/score` accepts cross-site requests — Confirmed
- **Where:** `apps/coursecraft-lti/src/server.rs:376-437`; cookie at `apps/coursecraft-lti/src/http.rs:247`
  (`SameSite=None`).
- **Defect:** the handler trusts the session cookie alone. There is no `Origin`, `Referer` or
  `Content-Type` check. A `text/plain` POST needs no preflight.
- **Trigger:** a learner with a live launch session opens another web page, which posts
  `{"percent":0,"status":"failed"}`. The server sends a score of 0 to the LMS.
- **Fix:** require `Content-Type: application/json` and an `Origin` equal to the origin of `cfg.tool_url`.

### 4.2 cmi5/xAPI: a pass after a fail in the same session is never reported — Confirmed in code; a spec decision
- **Where:** `crates/lms/src/lrs.rs:766-768` (`enqueue_passed` returns when `failed_this_session`).
- **Effect:** fail, Retry, pass in one session: no `passed` statement, so the LMS never satisfies the AU.
  The SCORM bridge reports the pass in the same case.
- **Note:** `docs/spec/triggers.md` ("Repeat rules") asks for this. Check cmi5 9.3.4 and 9.3.5 before
  changing it; change the spec in the same commit.

### 4.3 SCORM and LTI overwrite a pass with a later fail — Reported
- **Where:** `crates/lms/src/bridge.rs:676-693` (`handle_score`); `crates/lms/src/lti.rs:63-106`.
- **Fix:** remember a pass in `LmsBridge` and `LtiReporter`; do not write a later fail or lower score.

### 4.4 LTI launch redirect drops the tool's URL prefix — Reported
- **Where:** `apps/coursecraft-lti/src/server.rs:311`, `316-325` (`launch_location`).
- **Fix:** build the location from the path part of `cfg.tool_url`, as `login_redirect` does.

### 4.5 LTI platform key rotation breaks launches for up to one hour — Reported
- **Where:** `apps/coursecraft-lti/src/server.rs:340-354`; `crates/lti/src/lib.rs:275-288`.
- **Fix:** on a `kid` miss, refetch the JWKS one time (rate-limited) and validate again.

### 4.6 LTI `state` is not bound to the browser (login CSRF) — Reported
- **Where:** `apps/coursecraft-lti/src/server.rs:202-272`.
- **Fix:** set a short-lived cookie with a hash of `state` in `login`; require it in `launch`.
- **Related:** `crates/lti/src/lib.rs:311-350` does not check `azp` when `aud` has several values.

### 4.7 Invalid LTI logins fill the table and evict valid ones — Reported
- **Where:** `apps/coursecraft-lti/src/server.rs:233-254`.
- **Fix:** validate first, insert only on success, evict by oldest `expires`.

### 4.8 The problem report can leak file names that contain spaces — Reported
- **Where:** `crates/ui-egui/src/report.rs:208-274`.
- **Trigger:** "Replaced logo.png with Jane Doe Passport.png." scrubs to "Replaced <file> with Jane Doe <file>."
- **Fix:** keep raw paths out of recorded status messages, or quote every path and name.

### 4.9 SCORM 1.2 interaction responses use long identifiers — Reported
- **Where:** `crates/lms/src/bridge.rs:331-353`.
- **Defect:** CMIFeedback wants single characters `[0-9a-z]`; the bridge writes full choice ids. A
  strict LMS returns error 405 and the interaction has no response data. The project spec documents
  the current format, so this is a standards decision.

---

## 5. Optimizations

**Engine**
- `crates/engine/src/lib.rs:194-200`: each mutating command does two full validations, one full course
  copy and one full comparison. Keep the last "is valid" result per revision.
- `crates/engine/src/commands.rs:7201`: `remove_dangling_triggers` validates the whole course once per
  removed trigger. Collect all matches from one pass.
- `crates/engine/src/commands.rs:2828` (from `csv_import.rs:552`): `fresh_id` rebuilds the id set per
  imported row, O(n²) for 2000 rows.
- `crates/engine/src/commands.rs:2188`: `course.replace` clones every string only to get locations.
- `crates/control/src/dispatch.rs:117`: `course.get` with a `pointer` serializes the whole course to
  read one node.

**Runtime**
- `crates/logic/src/run.rs:1609`: `check_intersections` compares all object pairs on every tick, also
  when no trigger listens for intersections.
- `crates/logic/src/lib.rs:805-814`: `Player::resume` runs full `validate()` and `play_order`; the
  preview calls it for each frame and input.
- `crates/logic/src/run.rs:3192`: `reach_points` collects, sorts and allocates per timeline per tick.
- `crates/logic/src/run.rs:857, 897`: the cue `Vec` is cloned for each video position report.
- `crates/logic/src/run.rs:2528`: `has_pointer_trigger` scans all triggers per object per frame.

**Editor and render**
- `crates/ui-egui/src/triggers.rs:1495`, `crates/ui-egui/src/question.rs:337`: `slide.clone()` on every
  repaint while the tab is visible.
- `crates/ui-egui/src/timeline.rs:2198-2241`: rebuilds object lists and clones cue points each repaint.
  Cache by `session.revision`.
- `crates/ui-egui/src/timeline.rs:539-578`: the ruler draws ticks outside the visible range.
- `crates/render/src/media.rs:557-594`: a rejected SVG is fetched and parsed again on every paint.
- `crates/render/src/media.rs:709-785`: decoded GIF/WebP animations are outside the picture budget and
  stay in memory after the preview closes.

**Save and open**
- `crates/ui-egui/src/lib.rs:1691`, `apps/coursecraft/src/autosave.rs:116`: each save and autosave
  copies all media; autosave builds the whole zip in memory.
- `crates/format/src/folder.rs:603`: each folder save hashes all media with a byte-at-a-time FNV.
- `apps/coursecraft/src/main.rs:779-782`, `2463-2466`: open copies each media `Vec<u8>` into a new
  `Arc<[u8]>`, so peak memory is twice the project.
- `apps/coursecraft/src/versions.rs:151-173`: `list_versions` parses `course.json` of every version on
  every refresh.
- `crates/pptx/src/lib.rs:1014-1057`: import reads every zip entry, including parts it never uses.

**LTI server**
- `apps/coursecraft-lti/src/server.rs:369-371`: each course file is read fully per request, with no
  `Cache-Control`, `ETag` or `Range` (Safari needs `Range` for video).
- `apps/coursecraft-lti/src/server.rs:466-508`: the AGS token is cached per learner; cache one per server.
- `apps/coursecraft-lti/src/server.rs:195-200`, `crates/lti/src/jwt.rs:55-56`: the PKCS#8 key is parsed
  again for every JWKS request and signature.

---

## 6. Updates to existing features

- **Validation limits.** Add maximum values for slide and layer duration and for frame size. Bugs
  2.3, 2.6 and 2.7 all start from values with no upper limit.
- **A safe clamp.** Four panics (2.3 to 2.6) come from `clamp` with computed bounds. Add one helper
  that orders its bounds, use it everywhere, and deny `f32::clamp` / `Ord::clamp` with non-constant
  bounds in review.
- **Bold with a caret.** Keep a typing style in the inline editor instead of styling the whole box (3.21).
- **Web editor.** It cannot publish and file operations are disabled.
- **Web runs.** Publish to Web has no resume; browser storage can supply it.
- **Desktop player.** Playback speed is offered only in the web player.
- **Documents for AI.** A document over 60,000 characters is cut; process it in parts.
- **Templates.** A template lives in one course; add a shared template library.
- **Editor settings.** Check that lock, hide, rulers, grid and guides save and reload correctly after
  M23; `docs/release-checklist.md` still says they last for the session only.

---

## Suggested order

1. Section 2 (data loss and crashes), one commit per bug, each with a failing test first.
2. Section 1 (Windows video).
3. 4.1 (LTI score CSRF).
4. Section 3, the Confirmed items first.
5. The rest of section 4, then sections 5 and 6.
