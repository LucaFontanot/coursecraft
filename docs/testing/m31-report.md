# M31 (`0.4.2`) Urgent Bug-Fix Report

**Milestone:** M31 (`0.4.2` urgent fixes)
**Scope:** 6 Must-fix items (data loss, crashes, LMS status downgrade, navigation loop) + 2 Should-fix items (web editor unsaved-changes guard & responsive publish), across steps `S1`–`S9`.

## Test Coverage (`M31-01`–`M31-21`)

| ID | Step | Crate / File | Coverage |
|---|---|---|---|
| `M31-01` | S1 | `coursecraft-ui-egui` (`tests.rs`) | `Ctrl/Cmd+S`, `Ctrl/Cmd+Shift+S`, `Ctrl/Cmd+O`, `Ctrl/Cmd+N` emit exact `EditorRequest`s; `Shift+S` checked before `S` |
| `M31-02` | S1 | `coursecraft-ui-egui` (`tests.rs`) | File shortcuts suppressed in `TextEdit`, inline text edit, preview focus, and welcome screen (`Save`/`SaveAs`) |
| `M31-03` | S1 | `coursecraft` (`u16_shell_tests.rs`) | Desktop `EditorApp` routes `Ctrl+S` on untitled → Save As, `Ctrl+N`/`O` on dirty → unsaved prompt, blocked during modal |
| `M31-04` | S1 | `coursecraft-ui-egui` (`tests.rs`) | File menu shortcut hints, compound `" / "` formatting, and `1440×900` shortcuts dialog height check |
| `M31-05` | S2 | `coursecraft-mcp` (`tests.rs`) | MCP startup `--course x.ccraft` preserves media across edit and `file_save` |
| `M31-06` | S2 | `coursecraft-mcp`, `coursecraft-control` | MCP startup `.ccraft-folder` and `course.json` + sibling `media/` preserve media on save |
| `M31-07` | S2 | `coursecraft` (`files_tests.rs`) | `--attach` (`EditorHostAdapter`) `.ccraft` round-trip preserves media |
| `M31-08` | S3 | `coursecraft-ui-egui` (`bughunt_insert.rs`) | All UI insert helpers on `1×1`, `10×10`, and `8192×8192` slides without panics |
| `M31-09` | S3 | `coursecraft-ui-egui` (`bughunt_insert.rs`) | `ScrollPanel` inspector & `"Fit to content"` at `frame.h = 25_000.0` |
| `M31-10` | S3 | `coursecraft-engine` (`tests.rs`) | `panic_hunt` extended with `1×1`, `10×10`, `8192×8192` slides across all insert commands |
| `M31-11` | S4 | `coursecraft-course`, `coursecraft-render`, `coursecraft-ui-egui` | Hostile-bounds `safe_clamp` unit tests (`Rect::NOTHING`, negative dimensions, `NaN` bounds) |
| `M31-12` | S5 | `coursecraft-publish` (`tests.rs`) | Course with `state.src: ""` builds & zips cleanly with base image |
| `M31-13` | S5 | `coursecraft-course` (`m29_state_validation_tests.rs`) | `validate()` emits `Severity::Warning` (not `Error`) for `state.src: ""`; `is_playable()` stays `true` |
| `M31-14` | S6 | `coursecraft-lms` (`u21_lms_status_tests.rs`) | `StrictScorm12` in-session non-downgrade across `PassedIncomplete`, `PassedFailed`, `CompletedIncomplete`, `CompletedFailed`, and `Auto` |
| `M31-15` | S6 | `coursecraft-lms` (`u21_lms_status_tests.rs`) | SCORM 1.2 resume protects reported pass/completion across all five `LmsStatus` modes |
| `M31-16` | S6 | `coursecraft-lms` (`u21_lms_status_tests.rs`) | `Completed` → `Score(90, Passed)` under `PassedIncomplete` records `90`; SCORM 2004 regression check |
| `M31-17` | S7 | `coursecraft-logic` (`tests.rs`) | `LastVisited` after bank redraw drains stale entries and never loops onto the current slide |
| `M31-18` | S7 | `coursecraft-cli` (`tests.rs`) | `coursecraft-cli simulate` verifies `LastVisited` after bank redraw does not loop |
| `M31-19` | S8 | `coursecraft` (`tests.rs`) | `WebUnsavedState` clean vs dirty guards (`New`, `Open`, `Example`, `Template`, `PPTX`), `Save`/`Don't Save`/`Cancel` |
| `M31-20` | S9 | `coursecraft-publish` (`tests.rs`) | `PackageZipper` chunked at `1 KiB`, `64 KiB`, `256 KiB` is byte-identical to `zip_package`; empty package handled |
| `M31-21` | S9 | `coursecraft-ui-egui` (`u21_publish_status_tests.rs`) | `PublishPhase::Busy` renders `"Publishing… {pct}%"` when `progress` is `Some` and `"Publishing…"` when `None` |

## Release Gate & Performance (Driver)

- `cargo xtask build-player`: _pending driver run_
- `cargo xtask ci`: _pending driver run_
- `cargo xtask perf`: _pending driver run_


## Driver gates and perf

- `cargo xtask build-player`: passed after S4, S7 and S9.
- `cargo xtask ci`: passed at S4, S7 and S9, each on the first attempt.
- `cargo xtask perf`: all 8 editor timing tests and the 300-slide big-course test passed within budget.
