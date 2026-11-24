# M35 Check for Updates Inside the App Report

**Milestone:** M35 (Check for updates inside the app)  
**Plan:** [`docs/testing/m35-plan.md`](m35-plan.md)  
**Scope:** Desktop-only background and on-demand GitHub release checking (`ad1-kamath/coursecraft`), SemVer + `0.x` pre-release eligibility, OS/CPU installer selection, HTTPS allowlisted redirect following, streaming download to Downloads with live progress/cancellation and SHA-256 + byte-size verification, safe Markdown release notes and **Software update** dialog, Help menu / status bar / Preferences wiring, web-editor & published-player isolation, `SHA256SUMS.txt` release workflow step, and M28 UI/UX sweep coverage across steps `S1`–`S8`.

## What Shipped Per Step (`S1`–`S8`)

| Step | Summary | Review Verdict |
|---|---|---|
| **S1** | Added `semver = "1"` to `[workspace.dependencies]` in `Cargo.toml` and desktop-only `ureq`, `semver`, and `ring` under `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]` in `apps/coursecraft/Cargo.toml` (all resolved offline from existing `Cargo.lock`). Added backward-compatible `Prefs` fields (`auto_check_updates`, `last_update_check_unix`, `skipped_update_version`) in `apps/coursecraft/src/prefs.rs`, and created desktop-gated `apps/coursecraft/src/updates.rs` with limits/constants, `should_auto_check` (with backwards-clock and `saturating_sub` safety), `auto_check_suppressed_by_env`, `is_ci_or_headless_env`, `user_agent`, and `current_version`. | `OK` |
| **S2** | Implemented `UpdateError` (12 variants with plain-language sentence-case `user_message`/`Display` and no inner payload leaks), `GitHubRelease` / `GitHubAsset` (safe defaults for missing and explicit `null` fields), `parse_releases_json` (`MAX_API_RESPONSE_BYTES` and empty-body guards), `parse_tag_version`, `is_release_eligible` (§2.4 draft/`0.x`/pre-release/`1.x+` policy), `select_latest_update`, `find_checksums_asset`, `pick_asset` (macOS/Windows/Linux format & arch ranking, non-installer filtering, empty-URL rejection), `parse_sha256_digest`, and UTF-8-safe `parse_sha256sums_line` in `apps/coursecraft/src/updates.rs`. | `OK` |
| **S3** | Added `is_allowed_update_url` (HTTPS-only allowlist for `api.github.com`, `github.com`, and `*.githubusercontent.com`, rejecting userinfo, non-443 ports, IPs, localhost, bad labels, whitespace, and control chars), `resolve_redirect_url`, `safe_asset_filename` (200-char/traversal/control guards), `unique_download_path` (preserving compound `.tar.*` extensions with a 1,000-attempt cap), `UpdateHttp` / `HttpResponse` / `RequestKind`, `follow_get_redirects` (validating every hop, max 5), `check_for_update_with`, and `download_asset_verified` (64 KiB streaming to `.part`, `ring` SHA-256 via `asset.digest` or 64 KiB-capped `SHA256SUMS.txt`, size/cancel checks, handle dropped before rename/cleanup on every error path). | `OK` |
| **S4** | Implemented real `UreqHttp` transport, `downloads_dir` / `parse_xdg_download_dir`, `is_portable_install`, and `UpdateClient` background worker with request deduplication, throttled progress updates, and cancellation in `apps/coursecraft/src/updates.rs` (omitting `open_downloaded_file` per Reviewer Note 4 so installers are never auto-opened). Review fixes switched `RequestKind::Download` from `ureq` `timeout_recv_body` (total body cap) to `timeout_per_read(30s)` + `timeout_recv_response(30s)` + `timeout_resolve`/`timeout_send_request(10s)`, added `https_only(true)` + `user_agent`/`accept_encoding("")` on agent config, allocated a fresh cancel `Arc<AtomicBool>` per download, added `is_io_timeout` for wrapped `ureq::Error::Timeout`, mapped `RequireHttpsOnly` to `UpdateError::DisallowedUrl`, and added `dl08_ureq_config_and_timeout_mapping`. | `FIXED` |
| **S5** | Added `project_links::RELEASES_API_URL` and created pure-egui, WASM-safe `crates/ui-egui/src/update_dialog.rs` (`UpdateState`, `UpdateInfo`, `UpdateAssetInfo`, `UpdateDownloadState`, `UpdateCheckStatus`, safe Markdown parser/renderer with 32 KiB / 300-line caps, `Dialog::show_with_footer` with zero nested scroll areas, palette tokens only, sentence-case copy, welcome + normal editor hooks; no `OpenDownloadedUpdate` / `can_open_directly` / `"Open disk image"` per Reviewer Note 4). Review fixes switched headings and bold spans to the registered `"inter-semibold"` font family, eliminated O(n²) scanning in `parse_inline_spans` via `InlineScanState`, rejected Unicode bidi override/isolate chars in `is_safe_https_url`, and expanded `ui01`/`ui02`. | `FIXED` |
| **S6** | Replaced the old Help menu `"Check for updates"` external link in `crates/ui-egui/src/menu.rs` with desktop-gated `"Check for updates…"` and an `accent_text` `"Update available"` hint when `has_available_hint()`; added width-guarded right-aligned `"Update available"` pill in `crates/ui-egui/src/status.rs` (`RISK-08`); added `PrefsState::auto_check_updates` (default `true`), `Editor::set_auto_check_updates`, and the desktop-only `"Updates"` Preferences section in `crates/ui-egui/src/preferences.rs` emitting `EditorRequest::SetAutoCheckUpdates` independently of `SetPreferences` (`RISK-06`). | `OK` |
| **S7** | Wired `UpdateClient`, silent startup checks (gated off in `cfg!(test)`, CI/headless env, CLI, and MCP), `poll_updates`, and all update `EditorRequest`s (`reveal_in_folder` only; WASM `EditorApp` isolated) in `apps/coursecraft/src/main.rs`, and added fake-HTTP shell integration tests in `apps/coursecraft/src/updates_ui_tests.rs`. Review fixes added `manual_update_check_pending` promotion so triggering a manual check while a silent startup check is in flight never hangs on `"Checking for updates…"`, preserved active downloads, surfaced thread-spawn failures, cleared `downloaded_update_path` on `UpToDate`, added `ui06_manual_check_promotes_in_flight_silent_check_without_hanging`, and tightened HTTP call-count assertions. | `FIXED` |
| **S8** | Added `window!("update", "Software update", open_update_dialog_preview)` in `crates/ui-egui/src/snapshot.rs`, wired `"window-update"` into `uiux_sweep_all_surfaces` and `uiux_sweep_scale_2_and_fixtures` with zero `uiux_allowlist.rs` exemptions, added `ui07_update_status_bar_hint_handles_narrow_width` in `crates/ui-egui/src/uiux_sweep_tests.rs`, fixed inline link/token wrapping inside `horizontal_wrapped` in `update_dialog.rs`, and added `Generate SHA-256 checksums` (`sha256sum CourseCraft-* > SHA256SUMS.txt`) to `.github/workflows/release.yml` *(unstaged in working tree; see Open Concerns)*. | `CONCERN` |

## Test Coverage (`UP-01`–`UP-09`, `DL-01`–`DL-08`, `UI-01`–`UI-08`)

34 new tests added across `coursecraft` (28) and `coursecraft-ui-egui` (6), plus `"window-update"` added to the existing M28 UI/UX surface sweep matrices:

| ID | Step | Crate / File | Coverage |
|---|---|---|---|
| `UP-01` | S2 | `coursecraft` (`updates.rs`) | `up01_parse_v043_fixture`: real `v0.4.3` GitHub API fixture parses tag, name, body, `html_url`, `prerelease`, `draft`, and all 4 assets with `size` and `digest` |
| `UP-02` | S2 | `coursecraft` (`updates.rs`) | `up02_semver_and_prerelease_policy`: `0.x` GitHub pre-release acceptance, semver RC filtering, RC-to-RC/stable upgrades, post-`1.0` stable-only gate, draft rejection, out-of-order max-SemVer pick, and up-to-date checks |
| `UP-03` | S2 | `coursecraft` (`updates.rs`) | `up03_hostile_and_oversized_json`: empty body, malformed/non-array JSON, missing/`null` fields, invalid tags, and `> 1 MiB` payloads return `Err` with 0 panics |
| `UP-04` | S2 | `coursecraft` (`updates.rs`) | `up04_pick_asset_platforms_and_arches`: macOS `.dmg`, Windows setup vs portable ranking, Linux `x86_64` tarball vs `aarch64` `None` fallback, non-installer filtering, and explicit arch preference |
| `UP-05` | S1 | `coursecraft` (`updates.rs`) | `up05_should_auto_check_daily_throttle` and `up05_env_suppression_user_agent_and_current_version`: disabled/first-run/`86_399 s`/`86_400 s`/backwards-clock throttle, CI/headless/env suppression, privacy `User-Agent`, and `current_version()` |
| `UP-06` | S3 | `coursecraft` (`updates.rs`) | `up06_url_allowlist_and_redirect_resolution`: HTTPS allowlist (`api.github.com`, `github.com`, `*.githubusercontent.com`), rejects `http://`, `file://`, userinfo, non-443 ports, IPs, localhost, third-party domains, and control/whitespace chars; relative redirect resolution |
| `UP-07` | S3 | `coursecraft` (`updates.rs`) | `up07_safe_filename_and_unique_download_path`: rejects path traversal, hidden/empty/NUL/`> 200`-char names; collision-free numbering preserving compound `.tar.*` extensions |
| `UP-08` | S2 | `coursecraft` (`updates.rs`) | `up08_sha256_digest_and_sums_parsing`: case-insensitive `sha256:<64-hex>` parsing and UTF-8-safe `SHA256SUMS.txt` line matching (`*` and `./` forms) |
| `UP-09` | S1 | `coursecraft` (`prefs.rs`) | `up09_update_prefs_backward_compat_and_round_trip`: legacy `prefs.json` defaults (`auto_check_updates: true`, timestamps/skip `None`) and serialization round-trip |
| `DL-01` | S3 | `coursecraft` (`updates.rs`) | `dl01_happy_path_check_and_privacy_headers`: verifies exact `User-Agent: CourseCraft/<version>` with zero cookies/tokens/OS/arch leakage |
| `DL-02` | S3 | `coursecraft` (`updates.rs`) | `dl02_rate_limits_and_offline_errors`: HTTP 403 (`X-RateLimit-Remaining: 0`), HTTP 429, and network errors map to plain-language `UpdateError` variants |
| `DL-03` | S3 | `coursecraft` (`updates.rs`) | `dl03_redirects_valid_and_hostile`: follows valid GitHub HTTPS hops; rejects `http://` downgrade, disallowed redirect hosts, and `> 5` hops |
| `DL-04` | S3 | `coursecraft` (`updates.rs`) | `dl04_streaming_download_verifies_asset_digest`: streams to `.part`, emits progress callbacks, verifies size and `ring` SHA-256 from `asset.digest`, renames atomically, leaves zero `.part` files |
| `DL-05` | S3 | `coursecraft` (`updates.rs`) | `dl05_fallback_sha256sums_verification`: falls back to 64 KiB-capped `SHA256SUMS.txt` verification when `asset.digest` is absent |
| `DL-06` | S3 | `coursecraft` (`updates.rs`) | `dl06_checksum_mismatch_truncated_and_oversized_cleanup`: SHA-256 mismatch, truncated stream, and oversized stream fail cleanly and delete `.part` without creating final file |
| `DL-07` | S3 | `coursecraft` (`updates.rs`) | `dl07_mid_download_cancellation_removes_part`: mid-stream `AtomicBool` cancel aborts promptly, removes `.part`, and returns `UpdateError::Cancelled` |
| `DL-08` | S4 | `coursecraft` (`updates.rs`) | `dl08_update_client_background_worker_and_dedup`, `dl08_update_client_cancel_download`, `dl08_platform_helpers_and_ureq_config`, and `dl08_ureq_config_and_timeout_mapping`: non-blocking worker events, concurrent check dedup, cancel, XDG/Downloads helpers, per-kind `ureq` timeouts, and wrapped timeout mapping |
| `UI-01` | S5 | `coursecraft-ui-egui` (`update_dialog.rs`) | `ui01_markdown_parser_and_safety_caps`: headings, bold/italic/code, lists, rules, `https://`-only links, inert HTML/images/non-HTTPS links, bidi/control stripping, pathological bracket linear-time parsing, and 32 KiB / 300-line caps |
| `UI-02` | S5 | `coursecraft-ui-egui` (`update_dialog.rs`) | `ui02_update_dialog_states_and_requests`: headless `TestHarness` across Available, No-Asset fallback, Checking, Up-to-date, Downloading (progress + Cancel), Verified (Show in folder), Failed, Error, and empty-notes fallback |
| `UI-03` | S6 | `coursecraft-ui-egui` (`update_dialog.rs`) | `ui03_help_menu_status_bar_and_welcome_visibility`: desktop Help `"Check for updates…"` + `"Update available"` hint, status-bar pill click opens dialog, and Welcome-screen dialog rendering |
| `UI-04` | S6 | `coursecraft-ui-egui` (`update_dialog.rs`) | `ui04_web_editor_isolation`: `updates_available == false` omits Help update item, Preferences update toggle, and status-bar pill |
| `UI-05` | S6 | `coursecraft-ui-egui` (`preferences.rs`) | `ui05_preferences_auto_check_toggle`: `"Check for updates automatically"` toggle emits `EditorRequest::SetAutoCheckUpdates(false)` / `(true)` |
| `UI-06` | S7 | `coursecraft` (`updates_ui_tests.rs`) | 7 `ui06_*` fake-HTTP `EditorApp` tests: silent startup offline silence (timestamp unset), idle rev-0 auto-open vs active-edit/prompt non-interruption, skip-version persistence + manual re-check, opt-out pref persistence, verified temp-Downloads download with no `.part` left, rate-limit backoff silence, and manual-check promotion during in-flight silent check |
| `UI-07` | S8 | `coursecraft-ui-egui` (`uiux_sweep_tests.rs`) | `ui07_update_status_bar_hint_handles_narrow_width` plus `"window-update"` in `uiux_sweep_all_surfaces` and `uiux_sweep_scale_2_and_fixtures`: 0 MUST violations across sizes, Dark/Light themes, scales `1.0`/`1.5`/`2.0`, and empty/long-name fixtures, with zero allowlist exemptions |
| `UI-08` | Driver | Workspace gates | `cargo xtask ci` (`layers: ok` + 17 `wasm32-unknown-unknown` crates) and `cargo xtask build-player` |

## Driver Gates

- **`cargo xtask ci`:**
  - `g1` (after S3): passed on attempt 1 (`3,915 passed; 0 failed; 16 ignored`, `layers: ok`, all 17 WASM crates checked).
  - `g2` (after S6): passed on attempt 1 (`3,924 passed; 0 failed; 16 ignored`, `layers: ok`, all 17 WASM crates checked).
  - `g3` (after S8): passed on attempt 1 (`3,932 passed; 0 failed; 16 ignored`, `fmt --all --check`, `clippy --workspace --all-targets -- -D warnings`, `layers: ok`, all 17 WASM crates checked — including `coursecraft` `195 passed; 0 failed` and `coursecraft-ui-egui` `1,313 passed; 0 failed; 11 ignored`).
- **`cargo xtask build-player`:**
  - `p1` (after S6 / `g2`): passed (`trunk build --release`, `wasm-release` finished in `0.24s`).
  - `p2` (after S8 / `g3`): passed (`trunk build --release`, `wasm-release` finished in `0.23s`).
- **Release editor perf suite (`perf.log` — `coursecraft-ui-egui::perf_tests`, `--release --ignored`):** passed (`9 passed; 0 failed; 0 ignored; 1283 filtered out` in `2.44s`).

## Measured Performance vs. Baseline

`docs/testing/m35-plan.md` defines no separate performance targets because M35 runs desktop update checks and downloads off the editor/render hot path. Below are the exact measured numbers from `/Users/aditya/Documents/Claude Projects/ScormCraft/target/m35/perf.log` compared against the budgets in [`docs/performance.md`](../performance.md) and the immediately preceding M34 measured baseline in [`docs/testing/m34-report.md`](m34-report.md):

| Stage (300-slide course, 1280×900) | Budget | Pre-M35 (`m34-report.md`, median / p95) | Measured After M35 (`perf.log`, median / p95) | Notes |
|---|---:|---:|---:|---|
| `idle` | 16 ms | 0.40 ms / 0.41 ms | **0.40 ms / 0.41 ms** | Flat; well under budget |
| `select slide 150` | 33 ms | 0.63 ms / 0.77 ms | **0.62 ms / 0.78 ms** | Flat; well under budget |
| `scroll slide list` | 33 ms | 0.36 ms / 0.37 ms | **0.36 ms / 0.37 ms** | Flat; well under budget |
| `typing in a text field` | 33 ms | 0.52 ms / 0.54 ms | **0.53 ms / 0.53 ms** | Flat; well under budget |
| `type on canvas with spell check` | 33 ms | 0.70 ms / 0.71 ms | **0.71 ms / 0.72 ms** | Flat; well under budget |
| `check spelling, open` | 250 ms | 29.05 ms | **47.00 ms** | Well under 250 ms budget |
| `check spelling, ignore step` (20 issues) | 33 ms | 9.27 ms / 9.55 ms | **0.44 ms / 0.59 ms** | Well under budget |
| `insert interaction + undo` (8 kinds) | 100 ms | 1.16 ms / 1.67 ms | **1.15 ms / 3.12 ms** | Well under 100 ms budget |
| `undo / redo move` (30 cycles) | 33 ms | 2.93 ms / 3.25 ms | **2.94 ms / 3.27 ms** | Flat; well under budget |
| `first frame` | 500 ms | 8.72 ms | **8.76 ms** | Flat; well under 500 ms budget |
| `visit every slide` | 33 ms | 0.43 ms / 0.69 ms | **0.43 ms / 0.70 ms** | `rss 263 -> 264 MB` |
| `visit every slide, real pictures` | 33 ms | 9.32 ms / 18.23 ms | **0.44 ms / 0.71 ms** | `rss 268 -> 271 MB, media 0.2 MiB, 1 decoded pictures` |
| `low-end lite 300 slides` | 33 ms | 34.63 ms / 53.63 ms | **13.92 ms / 34.31 ms** | `media 4.4 MiB, undo 0.0 KiB` (median well under 33 ms; p95 1.31 ms over print budget, improved from pre-M35) |

No editor hot-path regression was introduced by M35.

## Skipped Steps & Open Concerns

- **Skipped steps:** None (`SKIPPED` file does not exist in `/Users/aditya/Documents/Claude Projects/ScormCraft/target/m35/`; all 8 steps `S1`–`S8` completed and committed).
- **Open concerns from `REVIEW.md`:**
  1. **S8 (`CONCERN`) — Uncommitted `.github/workflows/release.yml` edit (owner/driver action needed):** `.github/workflows/release.yml` contains the required `Generate SHA-256 checksums` step (`working-directory: dist`, `run: sha256sum CourseCraft-* > SHA256SUMS.txt` between `actions/download-artifact@v8` and `softprops/action-gh-release@v3`, compatible with `updates::find_checksums_asset` and `parse_sha256sums_line`), **but the change was left unstaged in the working tree (`M .github/workflows/release.yml`) and was not committed in `c385aa34`**. Stage and commit `.github/workflows/release.yml` before merging/tagging so the `SHA256SUMS.txt` workflow step is not dropped.
- **Note on plan override (Reviewer Note 4):** Per Reviewer Note 4 in `docs/testing/m35-plan.md`, downloaded installers are never auto-run or auto-opened (`open_downloaded_file`, `EditorRequest::OpenDownloadedUpdate`, and the macOS `"Open disk image"` button were intentionally omitted); completed downloads offer **Show in folder** only.

## Owner Manual Verification Checklist (Including Real GitHub Test — ~10 Minutes)

From `docs/testing/m35-plan.md` §7.2 (adjusted for Reviewer Note 4: **Show in folder** only):

- [ ] **Real GitHub check (up-to-date case):** Launch the built desktop app at the current version. Click **Help → Check for updates…**. Confirm the dialog responds in ~1 s with *"You're up to date."* and no UI freeze.
- [ ] **Real GitHub check + real download (simulated older version):** Test a local build with an older version against a newer live GitHub release (e.g. building with an older version against the latest published release):
  - Confirm the quiet startup check finds the newer release, opens the calm **Software update** dialog once, and renders the real Markdown release notes cleanly (bold lead-ins, bullet lists, no clipped lines, readable in both Dark and Light themes).
  - Click **Later** → confirm the dialog closes and the quiet **Update available** pill appears in the status bar and **Help** menu without nagging.
  - Reopen via the status-bar pill and click **Download update** → confirm the live progress bar updates smoothly while dragging/scrolling the editor (UI never freezes), the matching installer lands in `~/Downloads/` (`.dmg` on macOS, `-setup.exe` on Windows, `-linux-x86_64.tar.gz` on Linux), reports **SHA-256 verified**, and **Show in folder** reveals it cleanly without executing or opening it.
  - Verify no leftover `.part` file exists in `~/Downloads/`.
- [ ] **Skip this version & daily throttle:** Click **Skip this version**, quit, and relaunch — confirm no startup dialog and no status-bar pill appear. Open **Help → Check for updates…** manually — confirm it still shows the newer version on demand.
- [ ] **Preferences opt-out:** Open **Preferences**, uncheck **Check for updates automatically**, remove `last_update_check_unix` from `prefs.json`, relaunch — confirm zero network requests occur on startup.
- [ ] **Offline silence:** Turn Wi-Fi off, clear `last_update_check_unix`, relaunch — confirm zero popups or error banners on startup. Then click **Help → Check for updates…** — confirm a calm offline message with **Try again** and **Open releases page**.
- [ ] **Web editor & published course check:** Open the web editor (`trunk serve` in `apps/coursecraft`) and a published web course — confirm **Help** has no **Check for updates…** item, **Preferences** has no update toggle, and the browser Network tab shows zero requests to `api.github.com`.
- [ ] **Release workflow check (on next tag):** After committing `.github/workflows/release.yml` and pushing the next `v*` tag, inspect the draft GitHub release and verify `SHA256SUMS.txt` is attached and its SHA-256 hashes match the uploaded `.dmg`, `.exe`, and `.tar.gz` files.
