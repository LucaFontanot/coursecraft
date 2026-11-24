# M35 Check for Updates Inside the App — Architecture, Security & Test Plan

**Role:** Product Architecture, Security & Test Plan (planning only — zero source files edited)  
**Target release:** 0.4.5 / M35 (lands after M34 performance; designed to keep diff overlap with M34 minimal)  
**Source request:** GitHub issue #5 (tester request):
> *"I want to be able to see the latest version and upgrade within the app. For non technical users it would help them stay on top of the rapid develop cycle and be able to see what new features are added. Today I have to go find the latest version on GitHub and install it."*

**Scope:**  
Add a calm, desktop-only **Check for updates + direct download** workflow to CourseCraft:
1. Quietly checks GitHub's public releases API for `ad1-kamath/coursecraft` in the background on startup (at most once every 24 hours, respectful of offline use and a Preferences toggle) and on demand via **Help → Check for updates…**.
2. Shows a calm, non-nagging **Software update** dialog with the new version, safely rendered Markdown release notes, **Download update**, **Later**, **Skip this version**, and an **Open releases page** fallback.
3. Picks the exact release installer for the user's OS and CPU, streams it to their **Downloads** folder with live progress and cancellation, verifies byte size and SHA-256 checksum (via GitHub's asset `digest` field and a new `SHA256SUMS.txt` published by `.github/workflows/release.yml`), and reveals (or on macOS `.dmg`, opens) the verified file.
4. Shows a quiet **Update available** hint in the status bar and **Help** menu when a newer, non-skipped release is known — nothing nags.

**No self-replacing binary updater:** CourseCraft deliberately downloads the verified platform package (`.dmg`, `-setup.exe`/`-portable.exe`, `.tar.gz`) rather than replacing its own executable in place. That avoids code-signing/notarisation pitfalls, Windows file-lock elevation hacks, and partial self-update corruption while giving non-technical users a one-click path to the right installer.

**Clean-room commitment:** Pure Rust update client, asset selector, checksum verifier, and minimal Markdown renderer written from scratch following `DESIGN.md` ("The Quiet Studio") and `docs/contributing.md:21-40`. Standard generic desktop wording ("Check for updates…", "Software update", "Update available", "Skip this version", "Show in folder"); zero competitor code, assets, or coined terms.

> **Implementer Guardrails (`AGENTS.md`, `docs/contributing.md`, `docs/testing/m28-uiux-plan.md`)**
> 1. **Never crash:** Zero `unwrap`/`expect`/`panic!`/`unreachable!`/`todo!` in non-test code; `unsafe_code = "forbid"` everywhere. Every HTTP response, JSON field, redirect URL, file size, filename, and Markdown input is treated as hostile.
> 2. **No `coursecraft-engine` command needed:** Checking for updates, saving update preferences, and downloading installers are host/app-shell actions — they never mutate `Course` or undo history (just like `EditorRequest::SetTheme`, `SetAutomation`, `SetPreferences`, `ShowPublished`). Do **not** touch `coursecraft-engine`, `coursecraft-course`, `coursecraft-logic`, or `docs/cli.md`.
> 3. **Desktop only — never break WASM:** All networking (`ureq`), version parsing (`semver`), hashing (`ring`), threads, and Downloads I/O live behind `#[cfg(not(target_arch = "wasm32"))]` in `apps/coursecraft`. The web editor and published learner player (`apps/coursecraft-player`) never check for updates, never compile `ureq`/`semver`/`ring`, and `cargo xtask wasm` stays green on every step.
> 4. **Zero new external crates to fetch:** `ureq 3.5.0` (with `rustls` + `webpki-roots`), `ring 0.17.14`, and `semver 1.0.28` are **already in `Cargo.lock`** (details in §2.1). Implementers work fully offline.
> 5. **Strict M28 UI/UX compliance:** Sentence case everywhere, positive toggle labels, no trailing colons, `Dialog::show_with_footer` with a single vertical scroll area (`UX-02`), palette tokens only (`UX-06`, `UX-07`), ≥ 24×24 pt compact / ≥ 32 pt primary hit targets (`UX-05`), plain-language errors (`UX-18`), and registered in `uiux_sweep_tests.rs`.
> 6. **No live network calls in `cargo test`:** All automated tests run against JSON fixtures and an in-memory fake HTTP transport (`UpdateHttp`). Automatic startup checks never hit the real network during `#[cfg(test)]`.
> 7. **Minimize overlap with M34 (performance):** M34 lands first and targets rendering/canvas/slides/timeline/properties hot paths. Keep M35 code in two brand-new modules (`apps/coursecraft/src/updates.rs` and `crates/ui-egui/src/update_dialog.rs`) with only tiny, additive hooks in existing files.

---


> **Reviewer notes (Claude) — these override the plan below.**
> 1. M35 starts after M34 is merged into this branch; re-check every file and line reference against the current code before editing.
> 2. Adding `semver = "1"` to `[workspace.dependencies]` and the desktop-only target dependencies is allowed (all crates are already in `Cargo.lock`, so offline builds work). No other new crates.
> 3. The automatic check must be off in tests, CI and headless runs (CLI, MCP, snapshot tools): no test may touch the network.
> 4. Never run or open a downloaded installer automatically; only reveal it in the file manager.

## 1. Goals & Exact Current Baseline

### 1.1 Goals
1. Non-technical desktop users can see when a newer CourseCraft version exists and read what changed inside the app without hunting on GitHub.
2. Downloading picks the right installer for macOS, Windows, or Linux (and CPU architecture), writes it safely to the user's **Downloads** folder with live progress and cancel, verifies size and SHA-256, and reveals it in one click.
3. Startup checks are silent, background-threaded, throttled to at most once every 24 hours, respect a Preferences opt-out, and degrade silently when offline or rate-limited.
4. Zero personal data, telemetry, or tracking leaves the machine — only a plain `GET` with `User-Agent: CourseCraft/<version>`.
5. Web editor and published courses remain completely free of update checks and desktop networking dependencies.

### 1.2 Verified Codebase & Live Release Baseline
Inspected `Cargo.toml`, `Cargo.lock`, `apps/coursecraft/Cargo.toml`, `apps/coursecraft/src/{main,prefs,files,export,preview_server}.rs`, `crates/ui-egui/src/{lib,menu,status,preferences,dialog,report,project_links,snapshot}.rs`, `.github/workflows/release.yml`, `scripts/package-macos.sh`, `packaging/windows/coursecraft.iss`, and the live `v0.4.3` GitHub release (`gh release view v0.4.3` and `gh api repos/ad1-kamath/coursecraft/releases?per_page=2`):

- **Existing Help menu stub (`crates/ui-egui/src/menu.rs:1202-1205`):**
  `show_project_help_items` already has a `"Check for updates"` button that simply opens `project_links::releases_url()` (`https://github.com/ad1-kamath/coursecraft/releases`) in the browser. M35 upgrades this on desktop to `"Check for updates…"` opening the in-app update dialog.
- **Existing reveal helper (`apps/coursecraft/src/main.rs:2196-2224`):**
  `reveal_in_folder(path: &Path) -> Result<(), String>` already exists and opens Finder (`open -R`), Explorer (`explorer /select,<path>`), or `xdg-open <parent>` without a shell.
- **Existing prefs persistence (`apps/coursecraft/src/prefs.rs:205-238, 477-511`):**
  `Prefs` is stored atomically at `<config_dir>/prefs.json` via `files::write_atomic`, using `#[serde(default)]` so new fields stay 100% backward-compatible with older `prefs.json` files.
- **Existing desktop vs web capability split (`crates/ui-egui/src/lib.rs:1141, 1154`, `apps/coursecraft/src/main.rs:390, 3627`):**
  Note that `file_ops_available` is `true` on **both** desktop and web (since M25 added browser open/save/publish), whereas desktop-only capabilities use dedicated flags like `automation_available` (defaults `false` in `Editor::new`, set `true` only in `EditorApp::new_desktop`). M35 follows this exact pattern with `updates_available: bool`.
- **Real `v0.4.3` GitHub Release JSON & Asset Names (verified live):**
  - `tag_name`: `"v0.4.3"`, `draft`: `false`, **`prerelease`: `true`** (all `0.x` releases are currently marked pre-release on GitHub!).
  - Real asset names (note inconsistent `v` prefix between macOS and Windows/Linux!):
    - `CourseCraft-0.4.3-macos.dmg` (`28,677,774` bytes, built on `macos-15` ARM64 runner)
    - `CourseCraft-v0.4.3-setup.exe` (`20,809,277` bytes, Inno Setup x64 per-user installer)
    - `CourseCraft-v0.4.3-portable.exe` (`65,619,456` bytes, standalone x64 exe)
    - `CourseCraft-v0.4.3-linux-x86_64.tar.gz` (`29,924,711` bytes, x86_64 tarball)
  - **GitHub API `digest` field:** Every asset in GitHub's REST API JSON already includes `"digest": "sha256:<64-lowercase-hex>"` (e.g. `"sha256:f6caeb20ad874ade46c8bf992cb70e777d30ae273877667de6671b4acae051e3"`).
  - **Missing `SHA256SUMS.txt`:** `.github/workflows/release.yml:86-103` does not yet publish a `SHA256SUMS.txt` checksum file. M35 adds `SHA256SUMS.txt` generation in `release.yml` AND verifies against `asset.digest` first with `SHA256SUMS.txt` as fallback.
  - **Real release body format (`scripts/release-notes.sh` from `CHANGELOG.md`):** Uses plain paragraphs, bold headings/lead-ins (`**Faster, lighter editor**`), `- ` bullet lists with `**bold**` and `` `code` `` spans, and occasional `[label](https://...)` links and `##`/`###` headings.

---

## 2. Design & Architecture

### 2.1 Dependencies (Zero Network Needed for Implementers)
All three crates are already locked in `Cargo.lock` with compatible features:

| Crate | Version in `Cargo.lock` | Workspace `Cargo.toml` change | `apps/coursecraft/Cargo.toml` change | Why |
|---|---|---|---|---|
| `ureq` | `3.5.0` (includes `rustls` + `webpki-roots` + `flate2`) | Already present (`Cargo.toml:50`) | Add `ureq.workspace = true` under `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]` | Blocking HTTPS client on background worker thread; supports streaming `body_mut().as_reader()` and explicit timeouts. |
| `ring` | `0.17.14` | Already present (`Cargo.toml:42`) | Add `ring.workspace = true` under `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]` | Streaming SHA-256 via `ring::digest::Context::new(&ring::digest::SHA256)`. |
| `semver` | `1.0.28` (default `std` feature, zero sub-deps) | **Add `semver = "1"` under `[workspace.dependencies]`** | Add `semver.workspace = true` under `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]` | Exact SemVer parsing & comparison (`0.4.10 > 0.4.9`, pre-release ordering). |

> **Owner note:** Because `semver 1.0.28`, `ureq 3.5.0`, and `ring 0.17.14` are already present in `Cargo.lock` (verified via `Cargo.lock` inspection), adding `semver = "1"` to `[workspace.dependencies]` resolves completely offline with zero crates.io fetches. `crates/ui-egui/Cargo.toml` adds **zero** dependencies.

### 2.2 Module Boundaries & Layering
No new workspace crates are added (`xtask/src/main.rs` `LAYERS` stays unchanged; `cargo xtask layers` stays green):

```text
crates/ui-egui (L6, desktop + wasm, ZERO networking / ZERO filesystem)
├── src/update_dialog.rs (NEW)
│   ├── UpdateState, UpdateInfo, UpdateAssetInfo, UpdateDownloadState, UpdateCheckStatus
│   ├── Safe Markdown parser & renderer: MdBlock, MdSpan, parse_release_notes(), show_release_notes()
│   ├── show_update_dialog(editor, ctx, requests)      // calm Dialog::show_with_footer
│   └── show_status_bar_hint(editor, ui)               // quiet "Update available" pill
├── src/lib.rs (minimal additive hooks)
│   ├── Editor::updates_available (default false), set_updates_available(bool)
│   ├── UiState::update: update_dialog::UpdateState
│   ├── EditorRequest::{CheckForUpdates, DownloadUpdate, CancelUpdateDownload,
│   │     SkipUpdateVersion(String), RevealDownloadedUpdate, OpenDownloadedUpdate,
│   │     SetAutoCheckUpdates(bool)}
│   └── Calls update_dialog::show_update_dialog in both welcome and normal editor branches
├── src/menu.rs        -> Help -> "Check for updates…" (desktop only) + quiet "Update available" hint
├── src/status.rs      -> Calls update_dialog::show_status_bar_hint(editor, ui)
├── src/preferences.rs -> "Updates" section with "Check for updates automatically" (desktop only)
└── src/project_links.rs -> RELEASES_API_URL constant alongside REPO_URL / releases_url()

apps/coursecraft (binary, desktop-only module behind #[cfg(not(target_arch = "wasm32"))])
├── src/updates.rs (NEW, #[cfg(not(target_arch = "wasm32"))])
│   ├── Pure logic (100% offline-testable):
│   │   ├── should_auto_check(enabled, last_unix, now_unix) -> bool
│   │   ├── parse_releases_json(bytes: &[u8]) -> Result<Vec<GitHubRelease>, UpdateError>
│   │   ├── select_latest_update(releases, current: &Version) -> Option<GitHubRelease>
│   │   ├── pick_asset(assets, os, arch, prefer_portable) -> Option<&GitHubAsset>
│   │   ├── parse_sha256_digest(digest: &str) -> Option<String>
│   │   ├── parse_sha256sums_line(text: &str, filename: &str) -> Option<String>
│   │   ├── is_allowed_update_url(url: &str) -> bool
│   │   ├── safe_asset_filename(name: &str) -> Option<String>
│   │   └── unique_download_path(dir: &Path, filename: &str) -> PathBuf
│   ├── Testable HTTP abstraction:
│   │   ├── trait UpdateHttp: Send + Sync { fn get(&self, url: &str, accept: &str, user_agent: &str, kind: RequestKind) -> Result<HttpResponse, UpdateError>; }
│   │   └── UreqHttp (real ureq 3.5 implementation with manual redirect hop validation)
│   ├── Streaming downloader:
│   │   └── download_asset_verified(http, url, expected_size, expected_sha256, dest_dir, filename, cancel, on_progress) -> Result<DownloadResult, UpdateError>
│   ├── Background worker coordinator:
│   │   └── UpdateClient (spawns worker thread, receives UpdateEvent via mpsc::Receiver, wakes egui via ctx.request_repaint())
│   └── Platform helpers:
│       ├── downloads_dir() -> Option<PathBuf>
│       └── open_downloaded_file(path: &Path) -> Result<(), String>  (macOS .dmg only via `open`)
├── src/prefs.rs       -> Prefs::{auto_check_updates, last_update_check_unix, skipped_update_version}
└── src/main.rs        -> Wires UpdateClient on desktop EditorApp, triggers silent startup check, polls events, handles EditorRequests
```

### 2.3 Threads & Data Flow

```text
[Startup (desktop only, !cfg!(test))]
  if should_auto_check(prefs.auto_check_updates, prefs.last_update_check_unix, now_unix)
      ──▶ UpdateClient::start_check(manual = false, ctx)
            ──▶ std::thread::spawn (1 background thread, never blocks UI)
                  ──▶ GET https://api.github.com/repos/ad1-kamath/coursecraft/releases?per_page=20
                  ──▶ parse_releases_json -> select_latest_update -> pick_asset
                  ──▶ sends UpdateEvent::CheckFinished { manual, outcome } via mpsc::Sender
                  ──▶ ctx.request_repaint()

[UI Thread (EditorApp::show_app)]
  poll_updates(ctx): drains update_rx.try_recv() (non-blocking)
    ├── CheckFinished (manual=false, Error / UpToDate / Skipped):
    │     stays completely silent; persists last_update_check_unix on Ok or RateLimited
    ├── CheckFinished (manual=false, Available(info) not skipped):
    │     updates editor.ui.update.available = Some(info)
    │     persists last_update_check_unix
    │     opens calm dialog ONCE only if no crash/restore modal is open AND session.revision == 0;
    │     otherwise shows only the quiet status bar & Help menu hint (never interrupts active editing!)
    ├── CheckFinished (manual=true, any outcome):
    │     updates dialog state (Available / UpToDate / Error) and keeps dialog open
    ├── DownloadProgress { downloaded_bytes, total_bytes }:
    │     throttled (~100 ms / 128 KiB intervals) -> updates progress bar + requests repaint
    └── DownloadFinished(Ok(result) | Err(msg)):
          updates dialog to Verified (with "Show in folder" / macOS "Open disk image") or Error

[User clicks "Download update"]
  Editor emits EditorRequest::DownloadUpdate
    ──▶ EditorApp::start_update_download(ctx)
          ──▶ UpdateClient::start_download(asset, checksums_url, downloads_dir, ctx)
                ──▶ std::thread::spawn (1 background thread with Arc<AtomicBool> cancel flag)
                      ──▶ resolves expected SHA-256 (asset.digest first, else fetches SHA256SUMS.txt <= 64 KiB)
                      ──▶ follows up to 5 HTTPS redirects, validating EVERY hop host
                      ──▶ streams 64 KiB chunks into <dest>.part while feeding ring::digest::SHA256
                      ──▶ checks cancel flag, MAX_DOWNLOAD_BYTES (512 MiB), exact size, and SHA-256
                      ──▶ on success: atomically renames <dest>.part -> <dest>
                      ──▶ on cancel/error: deletes <dest>.part immediately
                      ──▶ sends UpdateEvent::DownloadFinished + ctx.request_repaint()
```

### 2.4 Version Comparison & Pre-Release Policy (Decision & Justification)
GitHub currently marks all CourseCraft `0.x` releases (`v0.4.2`, `v0.4.3`, …) with `"prerelease": true` even though their tags are clean semver versions (`0.4.3`, empty pre-release identifier). Because GitHub's `/releases/latest` endpoint skips all `prerelease: true` releases and returns **HTTP 404**, CourseCraft **must** query `GET /repos/ad1-kamath/coursecraft/releases?per_page=20` instead.

**Eligibility rule for candidate release `R` with parsed semver `V` against running version `C` (`env!("CARGO_PKG_VERSION")`):**
1. **Always skip** `R.draft == true` (draft releases are unpublished).
2. **Always skip** tags that do not parse as valid SemVer after stripping an optional leading `v` or `V`.
3. **Pre-release gate:**
   - If `C.major == 0 && C.pre.is_empty()` (today's `0.4.3` tester builds): accept releases with `V.pre.is_empty()` regardless of GitHub's `R.prerelease` flag. Reject semver pre-releases like `0.5.0-rc.1` / `0.5.0-alpha.1` so testers on `0.4.3` aren't pushed unfinished RCs.
   - If `!C.pre.is_empty()` (someone explicitly installed an alpha/beta/rc build like `1.0.0-rc.1`): accept both `R.prerelease == true` and `false`, and both pre-release and stable semver versions (so `1.0.0-rc.1` upgrades to `1.0.0-rc.2` and then `1.0.0`).
   - If `C.major >= 1 && C.pre.is_empty()` (future post-1.0 stable builds): accept **only** `!R.prerelease && V.pre.is_empty()`.
4. **Winner selection:** Among all eligible releases in the returned list, pick the one with the **maximum `semver::Version`**. Report an update only if `max_version > current_version`.

**Why this policy:**
- Works immediately during the `0.x` rapid-development cycle with zero extra settings for non-technical testers.
- Transitions cleanly and automatically to stable-only updates once CourseCraft ships `1.0.0`.
- Immune to out-of-order release publication (e.g. a `v0.3.7` hotfix published after `v0.4.4`) and immune to lexicographic version bugs (`0.4.10 > 0.4.9`).

### 2.5 Platform & CPU Asset Picker (`pick_asset`)
Uses the real release naming conventions verified from `v0.4.3` and `.github/workflows/release.yml`, while remaining forward-compatible with future arch-qualified macOS/Windows/Linux builds:

1. **Always ignore non-installer files:** skip names ending in `.txt`, `.sha256`, `.sig`, `.asc`, `.json`, `.md`, or equal to `SHA256SUMS` / `SHA256SUMS.txt` (case-insensitive).
2. **Never assume a `v` prefix** (macOS uses `CourseCraft-0.4.3-macos.dmg`; Windows and Linux use `CourseCraft-v0.4.3-…`).
3. **Architecture compatibility helper:**
   - An asset name containing `arm64` or `aarch64` is tagged `Arm64`.
   - An asset name containing `x86_64`, `amd64`, `x64`, or `intel` is tagged `X64`.
   - An asset name containing `universal` is tagged `Universal`.
   - Otherwise `Unspecified`.
   - Never select an asset explicitly tagged for a *different* architecture than `arch`.
4. **Per-OS ranking:**
   - **`macos`:** Match `.dmg` files first (fallback: `.pkg` or `macos` `.zip`/`.tar.gz`). Prefer exact arch (`arm64`/`aarch64` on `aarch64`; `x86_64`/`x64`/`intel` on `x86_64`) → `universal` → `Unspecified` (matches today's `CourseCraft-0.4.3-macos.dmg`).
   - **`windows`:** Match `.exe` / `.msi` files. If `prefer_portable` is `true` (detected when `current_exe` file stem contains `"portable"` case-insensitively), rank `portable.exe` first then `setup.exe`/`.msi`; otherwise (default for non-technical users) rank `setup.exe` → `.msi` → `portable.exe`.
   - **`linux`:** Match `.tar.gz`, `.AppImage`, `.deb`, `.rpm` containing `linux` (or `.AppImage`). Require exact arch match when arch is tagged (matches `CourseCraft-v0.4.3-linux-x86_64.tar.gz` on `x86_64`; returns `None` on `aarch64` so ARM Linux users get the browser fallback rather than an unrunnable x86_64 binary).
   - **No matching asset:** Still show the update dialog with release notes and replace **Download update** with **Open releases page** (primary button) + a calm note: *"No direct download is packaged for this platform. Open the releases page in your browser to see available files."*

### 2.6 Settings Storage (`apps/coursecraft/src/prefs.rs`)
Add three backward-compatible fields to `Prefs`:
```rust
/// Check GitHub at most once a day on startup for a newer desktop version.
#[serde(default = "default_true")]
pub auto_check_updates: bool,
/// Unix timestamp (seconds) of the last completed or rate-limited update check.
#[serde(default, skip_serializing_if = "Option::is_none")]
pub last_update_check_unix: Option<u64>,
/// Semver version string the user chose to skip (e.g. "0.4.4").
#[serde(default, skip_serializing_if = "Option::is_none")]
pub skipped_update_version: Option<String>,
```
- **Preferences UI (`crates/ui-egui/src/preferences.rs`):** When `editor.updates_available` is `true`, add an `"Updates"` section at the bottom of **Preferences** with a positive-worded checkbox: `"Check for updates automatically"` and helper caption `"Checks GitHub once a day on startup. No personal data is sent."`. Toggling emits `EditorRequest::SetAutoCheckUpdates(bool)` (modeled on `SetAutomation` / `SetTheme`, avoiding churn to existing `SetPreferences` callers in `slide_size_dialog.rs` and `spell_check.rs`).
- **Skipping a version:** Clicking **Skip this version** emits `EditorRequest::SkipUpdateVersion(version)` → saves `prefs.skipped_update_version = Some(version)`, closes the dialog, and clears the status-bar/Help hint for that version. If a still-newer version (`0.4.5 > 0.4.4`) later ships, it is NOT skipped and notifies normally. Manually clicking **Help → Check for updates…** always shows the latest newer version even if previously skipped.

### 2.7 Failure Handling Matrix

| Failure Mode | Startup Background Check (`manual = false`) | Manual Check / Download (`manual = true`) |
|---|---|---|
| Offline / DNS / connection refused / TLS error | Completely silent; does not advance `last_update_check_unix` (retries on next launch). | Plain message: *"Couldn't check for updates. Check your internet connection and try again."* + **Try again** + **Open releases page**. |
| Timeout (15 s metadata; 30 s per-read on download) | Completely silent. | Plain message: *"The update server took too long to respond."* + **Try again** + **Open releases page**. |
| Rate limited (HTTP 403 w/ `X-RateLimit-Remaining: 0` or HTTP 429) | Completely silent; records `last_update_check_unix` so we back off for 24 h. | Plain message: *"GitHub's update service is temporarily busy. Try again later or open the releases page in your browser."* + **Open releases page**. |
| Other HTTP 4xx / 5xx | Completely silent. | Plain message naming only status code (no raw HTML/debug dumps) + **Try again** + **Open releases page**. |
| Bad / truncated / > 1 MiB JSON | Completely silent. | Plain message: *"Received an unexpected response from GitHub."* + **Open releases page**. |
| Redirect loop (> 5 hops) or disallowed redirect host/scheme | Silent (check) / aborted (download). | Download aborted, `.part` deleted, plain message + **Open releases page**. |
| Asset > 512 MiB or truncated download (`bytes != asset.size`) | N/A | Download aborted immediately, `.part` deleted, plain message explaining incomplete/oversized file + **Try again** + **Open releases page**. |
| SHA-256 mismatch | N/A | `.part` deleted immediately (never renamed to final file), plain message: *"The downloaded file didn't pass verification and was removed."* + **Try again** + **Open releases page**. |
| User clicks **Cancel** mid-download | N/A | Worker checks `AtomicBool` every 64 KiB chunk, closes and removes `.part`, returns dialog cleanly to update-available state. |
| Downloads folder missing / unwritable / disk full | N/A | `.part` cleaned up if created, plain message + **Open releases page**. |
| Running version >= latest GitHub release | Completely silent; records `last_update_check_unix`. | Dialog shows: *"You're up to date. CourseCraft <version> is the latest version."* + **Close**. |

### 2.8 UI/UX & Copy Specification (M28 Rules `UX-01`–`UX-22`)
- **Menu (`crates/ui-egui/src/menu.rs`):**
  - Under **Help** (only when `editor.updates_available` is `true`; omitted on web editor):
    - Item label: `"Check for updates…"` (sentence case, ellipsis because it opens a dialog).
    - When a non-skipped update is known (`editor.ui.update.has_available_hint()`), show a quiet right-aligned `"Update available"` hint in `p.accent_text` on the menu row (keeps one consistent action name per M32!).
- **Status bar (`crates/ui-egui/src/status.rs`):**
  - When `editor.updates_available && editor.ui.update.has_available_hint()`, render a quiet pill `"Update available"` with a small `p.accent` dot to the left of the slide-size pill in the right-hand layout (hidden automatically if available width is too narrow, matching the slide-size pill's `RISK-05` guard).
  - Height `24.0 pt` (`UX-05`), hover tooltip `"CourseCraft <version> is available — click to view"`, clicking opens the **Software update** dialog.
- **Software update dialog (`crates/ui-egui/src/update_dialog.rs`):**
  - Built with `Dialog::new("Software update").open(&mut open).resizable(true).default_size([520.0, 420.0]).show_with_footer(ctx, ...)`.
  - **Single vertical scroll area (`UX-02`):** `DialogUi::body` holds the header summary + rendered Markdown release notes + download status; `DialogUi::footer` holds the action buttons pinned at the bottom. Never wrap an inner `ScrollArea` inside `DialogUi::body`.
  - **Header:** `"CourseCraft <new_ver> is available"` (`heading` / semibold) and secondary caption `"You're currently running <current_ver>."` (plus asset name & formatted size like `"CourseCraft-0.4.4-macos.dmg · 27.4 MB"` when matched).
  - **Release notes body:** Rendered by `show_release_notes(ui, &blocks)`:
    - Headings (`#`/`##`/`###`), bulleted and numbered lists with hanging indent, paragraphs, horizontal rules, inline **bold**, *italic*, `monospace code`, and safe `https://` links (clickable via `ctx.open_url`, showing full URL on hover).
    - Empty release notes fallback: secondary text `"No release notes were included for this version."` (`UX-17`).
  - **Footer buttons by state:**
    - **Available (asset matched):** Primary (`primary_button`, ≥ 32 pt tall): `"Download update"` | Secondary: `"Later"`, `"Skip this version"`, quiet link `"Open releases page"`.
    - **Available (no matching asset for OS/arch):** Primary: `"Open releases page"` | Secondary: `"Later"`, `"Skip this version"`.
    - **Downloading:** Progress bar (`egui::ProgressBar::new(frac)` with `"12.4 MB of 28.7 MB"` or `"12.4 MB downloaded"` when size unknown) + `"Cancel"` button + `"Open releases page"`.
    - **Downloaded & verified:** Success line `"Saved to Downloads (<filename>)."` + checksum badge (`"SHA-256 verified"` when verified, or `"Size verified"` if no digest published) + calm instruction `"Save your work and close CourseCraft before installing."` | Footer: Primary `"Show in folder"`, optional `"Open disk image"` (macOS `.dmg` only), `"Close"`.
    - **Checking:** `"Checking for updates…"` + `"Close"`.
    - **Up to date:** `"You're up to date."` + secondary `"CourseCraft <current> is the latest version."` + `"Close"`.
    - **Error:** Plain sentence (`UX-18`) + `"Try again"`, `"Open releases page"`, `"Close"`.
  - **Welcome screen support:** `update_dialog::show_update_dialog` is invoked in **both** the `if self.ui.show_welcome` branch (`crates/ui-egui/src/lib.rs:4657`) and the normal editor branch (`crates/ui-egui/src/lib.rs:4844`), so Help → Check for updates… and quiet startup notices work before a course is opened.

---

## 3. Security & Privacy Notes

| Threat | Mitigation Enforced in Code |
|---|---|
| **Tracking / personal data leakage** | Only plain `GET` requests to `api.github.com` / `github.com` / `*.githubusercontent.com`. Headers limited to `User-Agent: CourseCraft/<version>` (no OS, CPU, hostname, locale, or UUID), `Accept`, and `X-GitHub-Api-Version`. No cookies, no tokens, no course data. |
| **MITM / HTTP downgrade / malicious redirect** | HTTPS (`https://`) strictly required on the API URL, every asset URL, and **every redirect hop** (manual redirect loop up to 5 hops with `ureq` `max_redirects(0)`). Only allowlisted hosts (`api.github.com`, `github.com`, `*.githubusercontent.com`) are accepted; private IPs, `localhost`, `http://`, and third-party domains are rejected. |
| **Tampered or corrupted download** | Streamed SHA-256 computed via `ring::digest::SHA256` during download and verified against GitHub's `asset.digest` (`sha256:...`) or `SHA256SUMS.txt`. Byte count verified against `asset.size`. `.part` file is deleted on any mismatch and never renamed to the final filename. |
| **Path traversal in `asset.name`** | `safe_asset_filename` rejects any `/`, `\`, `:`, `..`, leading `.`, NUL, control characters, or names > 200 chars before joining onto `downloads_dir`. Never overwrites existing files (`unique_download_path` appends ` (1)`, ` (2)`, …). |
| **Memory / disk exhaustion (zip-bomb / infinite stream)** | API JSON response capped at `1 MiB`; `SHA256SUMS.txt` capped at `64 KiB`; installer download hard-capped at `512 MiB` (`MAX_DOWNLOAD_BYTES`) and bounded by `asset.size`. Download streams in 64 KiB chunks directly to disk (O(1) RAM). |
| **Hostile Markdown / XSS / remote image tracking / shell execution** | Release notes body capped at `32 KiB` / `300 lines`; control chars and Unicode bidi overrides stripped; raw HTML rendered as inert text; Markdown images (`![alt](url)`) rendered as plain text with zero network fetches; links clickable **only** when URL strictly starts with `https://` without whitespace/control chars (`javascript:`, `file:`, `data:`, `http:` rendered as plain text). |
| **Auto-executing untrusted binaries** | CourseCraft **never** auto-executes a downloaded `.exe` or script. On completion, the user clicks **Show in folder** (`reveal_in_folder` — no shell) or, on macOS `.dmg` only, **Open disk image** (`open <dmg>`, which mounts the DMG in Finder without executing code). |
| **WASM / published player leakage** | `updates.rs`, `ureq`, `semver`, and `ring` are strictly `#[cfg(not(target_arch = "wasm32"))]` in `apps/coursecraft`. `Editor::updates_available` defaults to `false`; web editor and published SCORM/xAPI/cmi5/LTI/Web courses never make any update request. |

---

## 4. Watchlist of Risky Spots (`RISK-01`–`RISK-09`)

| ID | Area / File | Risk & Concrete Mitigation |
|---|---|---|
| **RISK-01** | GitHub `/releases/latest` returning 404 | All current `0.x` releases have `prerelease: true`, so `/releases/latest` returns 404. **Mitigation:** Always query `/repos/ad1-kamath/coursecraft/releases?per_page=20` and apply §2.4's semver + pre-release filter. |
| **RISK-02** | Inconsistent `v` prefix in asset names | Live `v0.4.3` uses `CourseCraft-0.4.3-macos.dmg` (no `v`) alongside `CourseCraft-v0.4.3-setup.exe` and `CourseCraft-v0.4.3-linux-x86_64.tar.gz` (with `v`). **Mitigation:** Never match the exact tag string in `pick_asset`; match case-insensitively on OS/arch tokens and extensions, tested with the real `v0.4.3` JSON fixture. |
| **RISK-03** | macOS Intel (`x86_64`) vs Apple Silicon (`aarch64`) | `.github/workflows/release.yml` currently builds one `macos-15` (ARM64) binary named `CourseCraft-<ver>-macos.dmg` with no arch suffix. **Mitigation:** `pick_asset` supports arch-tagged names (`arm64`/`aarch64`/`x86_64`/`universal`) first and falls back to unqualified `macos.dmg`; document in owner checklist to verify macOS arch naming if Intel builds are later added. |
| **RISK-04** | `ureq` redirect header & host handling | GitHub asset URLs (`github.com/.../releases/download/...`) always 302-redirect to `objects.githubusercontent.com` or `release-assets.githubusercontent.com`. **Mitigation:** Set `max_redirects(0)` on `ureq` and follow up to 5 `301/302/303/307/308` hops manually, validating `is_allowed_update_url` on every hop and resolving relative `Location` headers safely. |
| **RISK-05** | Tests accidentally making live network calls | `EditorApp::new_desktop` / `startup` runs in `apps/coursecraft/src/tests.rs` and `m27_files_ui_tests.rs`. **Mitigation:** Gate automatic startup network check behind `!cfg!(test)` (and inject fake `UpdateHttp` in tests), so `cargo test` performs zero real network I/O. |
| **RISK-06** | `EditorRequest::SetPreferences` churn | `SetPreferences` is constructed in 3 files (`preferences.rs`, `slide_size_dialog.rs`, `spell_check.rs`) and matched in 3 tests. **Mitigation:** Use dedicated `EditorRequest::SetAutoCheckUpdates(bool)` (like `SetAutomation` and `SetTheme`) so unrelated files stay untouched. |
| **RISK-07** | M28 UI/UX rules (`UX-02`, `UX-06`, `UX-07`, `UX-11`) | New dialogs often trip nested-scroll (`UX-02`), hardcoded `Color32` (`UX-07`), or trailing-colon / Title-Case (`UX-11`) checks. **Mitigation:** Use `Dialog::show_with_footer` with no inner `ScrollArea`, palette tokens only, sentence case everywhere, and add `"update"` to `snapshot.rs` + `uiux_sweep_tests.rs` in S8. |
| **RISK-08** | Status bar overflow on narrow windows (`1024×600`, scale `1.5`) | Adding `"Update available"` pill could crowd or clip the slide-size pill at narrow widths. **Mitigation:** Hide the update pill cleanly when `ui.available_width()` is below the combined threshold of slide-size pill + update pill (leaving Help menu always reachable); verify in `uiux_sweep_tests.rs`. |
| **RISK-09** | Merge conflicts with M34 (performance) | M34 modifies hot editor/render files before M35 merges. **Mitigation:** Put ~90% of M35 code in two new files (`apps/coursecraft/src/updates.rs`, `crates/ui-egui/src/update_dialog.rs`), and keep edits in `lib.rs` and `main.rs` small and strictly additive. |

---

## 5. Concrete Test Plan

### 5.1 Parsing, SemVer, Pre-Release & Asset Selection Unit Tests (`UP-01`–`UP-09`)

| ID | File | Behaviour Tested | Expected Result |
|---|---|---|---|
| **UP-01** | `apps/coursecraft/src/updates.rs` | Real `v0.4.3` GitHub JSON fixture parsing | Parses tag, name, body, `html_url`, `prerelease`, `draft`, and all 4 assets including `size` and `digest: "sha256:..."`. |
| **UP-02** | `apps/coursecraft/src/updates.rs` | SemVer comparison & pre-release policy (§2.4) | `0.4.3` → `v0.4.4` (`prerelease: true`) = update; `0.4.3` → `v0.4.10` = update; `0.4.3` → `v0.5.0-rc.1` = ignored; `0.5.0-rc.1` → `v0.5.0-rc.2` / `v0.5.0` = update; `1.0.0` → `v1.1.0-beta.1` (`prerelease: true`) = ignored; `draft: true` always ignored; picks max semver when releases are out of order; `0.4.3` vs `v0.4.3` or `v0.4.2` = up to date. |
| **UP-03** | `apps/coursecraft/src/updates.rs` | Hostile / malformed / oversized JSON | Empty body, invalid JSON, non-array JSON, missing fields, invalid tag strings, and > 1 MiB payload all return `Err` with 0 panics. |
| **UP-04** | `apps/coursecraft/src/updates.rs` | Platform & CPU asset picker (`pick_asset`) | On real `v0.4.3` asset list: `("macos", "aarch64")` → `CourseCraft-0.4.3-macos.dmg`; `("windows", "x86_64", false)` → `CourseCraft-v0.4.3-setup.exe`; `("windows", "x86_64", true)` → `CourseCraft-v0.4.3-portable.exe`; `("linux", "x86_64")` → `CourseCraft-v0.4.3-linux-x86_64.tar.gz`; `("linux", "aarch64")` → `None`; ignores `SHA256SUMS.txt` and `.sig`; prefers explicit arch match over unqualified when both exist. |
| **UP-05** | `apps/coursecraft/src/updates.rs` | Daily throttle (`should_auto_check`) | `false` when disabled; `true` when `last == None`; `false` at `86_399 s`; `true` at `86_400 s`; `false` when clock moved backwards (`now < last`). |
| **UP-06** | `apps/coursecraft/src/updates.rs` | URL & redirect host allowlist (`is_allowed_update_url`) | Allows `https://api.github.com/...`, `https://github.com/...`, `https://objects.githubusercontent.com/...`, `https://release-assets.githubusercontent.com/...`; rejects `http://`, `file://`, `https://evil.com`, `https://github.com.evil.com`, `https://127.0.0.1`, URLs with whitespace/control chars. |
| **UP-07** | `apps/coursecraft/src/updates.rs` | Safe filename & collision-free download path | Rejects `../evil`, `a/b`, `a\b`, `.hidden`, `""`, NUL, > 200 chars; accepts `CourseCraft-0.4.4-macos.dmg`; `unique_download_path` creates `foo (1).dmg`, `foo (2).dmg` without overwriting existing files. |
| **UP-08** | `apps/coursecraft/src/updates.rs` | `SHA256SUMS.txt` & `asset.digest` parsing | Parses `sha256:<64-hex>` (case-insensitive) and rejects non-sha256 or short/non-hex strings; parses standard `sha256sum` lines (`<hex>  <name>` and `<hex> *<name>`), ignoring comments and unrelated files. |
| **UP-09** | `apps/coursecraft/src/prefs.rs` | Prefs backward compatibility & round-trip | Old `prefs.json` lacking the 3 update fields loads with `auto_check_updates: true`, `last_update_check_unix: None`, `skipped_update_version: None`; saving & reloading preserves all 3 fields. |

### 5.2 Fake-HTTP Download, Verification & Worker Tests (`DL-01`–`DL-08`)

| ID | File | Behaviour Tested | Expected Result |
|---|---|---|---|
| **DL-01** | `apps/coursecraft/src/updates.rs` | Happy-path check + privacy header assertion | Fake `UpdateHttp` records exact request URL & headers: verifies `User-Agent == "CourseCraft/<version>"` (no OS/arch/user data), no `Cookie`/`Authorization`, returns parsed `UpdateInfo`. |
| **DL-02** | `apps/coursecraft/src/updates.rs` | Rate limits (HTTP 403 w/ `X-RateLimit-Remaining: 0`, HTTP 429) & offline error | Maps cleanly to `UpdateError::RateLimited` / `UpdateError::Network` with non-technical user-facing sentences (zero raw debug dumps). |
| **DL-03** | `apps/coursecraft/src/updates.rs` | Redirect following & hostile redirect blocking | Follows 2 valid HTTPS redirects (`github.com` → `objects.githubusercontent.com`) and downloads successfully; rejects `http://` downgrade, disallowed host, and > 5 redirect hops. |
| **DL-04** | `apps/coursecraft/src/updates.rs` | Streaming download + progress + SHA-256 verification via `asset.digest` | Writes to temp `.part`, emits progress callbacks, verifies size & `ring` SHA-256, renames to final path, leaves zero `.part` files behind. |
| **DL-05** | `apps/coursecraft/src/updates.rs` | Fallback SHA-256 verification via `SHA256SUMS.txt` | When `asset.digest` is `None` and `SHA256SUMS.txt` asset is present, fetches sums file (capped at 64 KiB) and verifies SHA-256. |
| **DL-06** | `apps/coursecraft/src/updates.rs` | SHA-256 mismatch, truncated download, and oversized download | Each fails with `Err`, deletes `.part` immediately, and leaves zero file at final destination path. |
| **DL-07** | `apps/coursecraft/src/updates.rs` | Mid-download cancellation (`AtomicBool`) | Setting cancel flag aborts stream promptly, deletes `.part` file, returns `UpdateError::Cancelled`. |
| **DL-08** | `apps/coursecraft/src/updates.rs` | `UpdateClient` background thread & deduplication | Spawns worker with fake `UpdateHttp`, delivers `UpdateEvent` over channel without blocking caller, ignores duplicate concurrent check requests. |

### 5.3 Safe Markdown, Dialog UI & App-Shell Integration Tests (`UI-01`–`UI-08`)

| ID | File | Behaviour Tested | Expected Result |
|---|---|---|---|
| **UI-01** | `crates/ui-egui/src/update_dialog.rs` | Markdown parser & safety caps | Parses headings, bold, italic, code, bullets, numbered lists, rules, and `https://` links; neutralizes `javascript:`, `file:`, `http:` links to plain text; strips raw HTML/bidi/control chars; renders `![img](url)` as text; truncates > 32 KiB / > 300 lines cleanly with 0 panics. |
| **UI-02** | `crates/ui-egui/src/update_dialog.rs` | Update dialog states & button requests | Headless `TestHarness` verifies Available (Download / Later / Skip / Open releases), No-Asset fallback, Checking, Up-to-date, Downloading (progress + Cancel), Verified (Show in folder), and Error (Try again) emit exact `EditorRequest`s. |
| **UI-03** | `crates/ui-egui/src/update_dialog.rs` | Help menu, status bar hint & Welcome screen visibility | When `updates_available == true`: Help shows `"Check for updates…"`, non-skipped update shows `"Update available"` pill in status bar & hint in Help; clicking pill opens dialog; dialog renders cleanly even when `show_welcome == true`. |
| **UI-04** | `crates/ui-egui/src/update_dialog.rs` | Web editor isolation (`updates_available == false`) | Help omits `"Check for updates…"`, Preferences omits `"Check for updates automatically"`, status bar never shows `"Update available"`. |
| **UI-05** | `crates/ui-egui/src/preferences.rs` | Automatic check toggle in Preferences | Toggling `"Check for updates automatically"` emits `EditorRequest::SetAutoCheckUpdates(false)` / `(true)`. |
| **UI-06** | `apps/coursecraft/src/updates_ui_tests.rs` | Shell wiring: silent startup vs manual check vs skip | Injecting fake `UpdateHttp` into `EditorApp`: silent startup error stays silent; silent startup newer version shows hint (and dialog if idle at rev 0); **Skip this version** persists in `prefs.json` and suppresses future silent startup notice for that version; manual check still shows skipped version; `auto_check_updates = false` skips startup check entirely. |
| **UI-07** | `crates/ui-egui/src/uiux_sweep_tests.rs` | Full M28 UI/UX sweep on `"update"` dialog + status bar hint | 0 MUST violations across `1024×600`, `1280×800`, `1920×1080` × `Dark`/`Light` × scale `1.0`/`1.5` (`UX-02` nested scroll, `UX-04` clip/overlap, `UX-05` hit targets, `UX-06` contrast, `UX-07` palette tokens, `UX-11` copy, `UX-14` bounds). |
| **UI-08** | Workspace gate | WASM build & layering verification | `cargo xtask ci` (includes `layers` and `wasm` across `coursecraft`, `coursecraft-ui-egui`, `coursecraft-player`) and `cargo xtask build-player` pass cleanly. |

---

## 6. Numbered Implementer Steps (8 Steps, ~30–45 min each)

> **Gate cadence:** Run targeted `cargo test -p <crate> -j 4 <filter>` after every step. Run full `cargo xtask ci` after **S4, S6, and S8** (plus `cargo xtask build-player` at **S8**).

- **S1 — Dependencies, Update Preferences & Daily Check Policy**
  - **Files:** `Cargo.toml`, `apps/coursecraft/Cargo.toml`, `apps/coursecraft/src/prefs.rs`, `apps/coursecraft/src/updates.rs` (new, `#[cfg(not(target_arch = "wasm32"))]`), `apps/coursecraft/src/main.rs` (`#[cfg(not(target_arch = "wasm32"))] pub mod updates;`).
  - **Exact changes:**
    1. In `Cargo.toml`, add `semver = "1"` under `[workspace.dependencies]` (resolves offline to `semver 1.0.28` already in `Cargo.lock`).
    2. In `apps/coursecraft/Cargo.toml`, under `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]`, add `ureq.workspace = true`, `semver.workspace = true`, and `ring.workspace = true`.
    3. In `apps/coursecraft/src/prefs.rs`, add `auto_check_updates: bool` (`#[serde(default = "default_true")]`), `last_update_check_unix: Option<u64>`, and `skipped_update_version: Option<String>` to `Prefs` and `Prefs::default()`.
    4. Create `apps/coursecraft/src/updates.rs` with constants (`CHECK_INTERVAL_SECS = 86_400`, `MAX_API_RESPONSE_BYTES = 1024 * 1024`, `MAX_SUMS_BYTES = 64 * 1024`, `MAX_DOWNLOAD_BYTES = 512 * 1024 * 1024`, `MAX_REDIRECTS = 5`), `should_auto_check`, `user_agent(version: &str) -> String` (`format!("CourseCraft/{version}")`), and `current_version() -> Option<semver::Version>`.
    5. Wire `#[cfg(not(target_arch = "wasm32"))] pub mod updates;` in `apps/coursecraft/src/main.rs`.
  - **Tests:** `UP-05`, `UP-09`; run `cargo test -p coursecraft -j 4 prefs` and `cargo test -p coursecraft -j 4 updates`.
  - **Done when:** Offline build succeeds with zero crates downloaded, `Prefs` backward-compatibility tests pass, and daily throttle tests pass.

- **S2 — GitHub Release Parser, SemVer Pre-Release Policy & Platform/CPU Asset Picker**
  - **Files:** `apps/coursecraft/src/updates.rs`.
  - **Exact changes:**
    1. Define serde structs `GitHubRelease` (`tag_name`, `name`, `body`, `html_url`, `draft`, `prerelease`, `assets`) and `GitHubAsset` (`name`, `browser_download_url`, `size`, `digest: Option<String>`) with `#[serde(default)]` on optional fields so extra/missing GitHub fields never fail deserialization.
    2. Implement `parse_releases_json(bytes: &[u8]) -> Result<Vec<GitHubRelease>, UpdateError>` enforcing `MAX_API_RESPONSE_BYTES`.
    3. Implement `parse_tag_version(tag: &str) -> Option<semver::Version>` (strips optional leading `v`/`V`), `is_release_eligible(release, candidate, current) -> bool` (exact §2.4 rule), and `select_latest_update(releases, current) -> Option<(semver::Version, GitHubRelease)>` (returns highest eligible semver strictly `> current`).
    4. Implement `pick_asset(assets: &[GitHubAsset], os: &str, arch: &str, prefer_portable: bool) -> Option<&GitHubAsset>` and `find_checksums_asset(assets: &[GitHubAsset]) -> Option<&GitHubAsset>` per §2.5.
    5. Implement `parse_sha256_digest(digest: &str) -> Option<String>` and `parse_sha256sums_line(text: &str, filename: &str) -> Option<String>`.
    6. Add realistic JSON fixture copied from the verified `v0.4.3` GitHub API shape (in `#[cfg(test)]`).
  - **Tests:** `UP-01`, `UP-02`, `UP-03`, `UP-04`, `UP-08`; run `cargo test -p coursecraft -j 4 updates`.
  - **Done when:** All release-parsing, semver/pre-release matrix, hostile JSON, asset-picking, and checksum-parsing tests pass with zero panics.

- **S3 — Safe Download Engine: URL/Redirect Validation, Streaming Progress, Size Cap & SHA-256 Verification**
  - **Files:** `apps/coursecraft/src/updates.rs`.
  - **Exact changes:**
    1. Implement `is_allowed_update_url(url: &str) -> bool` (HTTPS-only, allowlist `api.github.com`, `github.com`, `*.githubusercontent.com`, no whitespace/control chars) and `resolve_redirect_url(base: &str, location: &str) -> Option<String>`.
    2. Implement `safe_asset_filename(name: &str) -> Option<String>` and `unique_download_path(dir: &Path, filename: &str) -> PathBuf`.
    3. Define `UpdateError` enum (`Offline`/`Network`, `Timeout`, `RateLimited`, `HttpStatus(u16)`, `InvalidResponse`, `DisallowedUrl`, `TooManyRedirects`, `DownloadTooLarge`, `DownloadTruncated`, `ChecksumMismatch`, `Cancelled`, `Io`) with `user_message(&self) -> String` producing calm, non-technical sentences (`UX-18`).
    4. Define `trait UpdateHttp: Send + Sync` returning a status code, relevant headers (`location`, `content-length`, `x-ratelimit-remaining`), and `Box<dyn Read + Send>` body reader, plus `follow_get_redirects(http, url, accept, ua, kind)` enforcing `MAX_REDIRECTS` and `is_allowed_update_url` on every hop.
    5. Implement `check_for_update_with(http, api_url, current, os, arch, prefer_portable)` and `download_asset_verified(...)` streaming 64 KiB chunks to `<dest>.part`, hashing with `ring::digest::Context::new(&ring::digest::SHA256)`, enforcing `MAX_DOWNLOAD_BYTES` and `expected_size`, verifying SHA-256 (`asset.digest` first, else fetching `SHA256SUMS.txt` capped at 64 KiB), deleting `.part` on any error/cancel, and renaming `<dest>.part` → `<dest>` on success.
  - **Tests:** `UP-06`, `UP-07`, `DL-01`, `DL-02`, `DL-03`, `DL-04`, `DL-05`, `DL-06`, `DL-07`; run `cargo test -p coursecraft -j 4 updates`.
  - **Done when:** Full check & download lifecycle (happy path, redirects, rate limits, bad checksum, truncated/huge stream, cancel, safe paths) is proven offline against fake `UpdateHttp`.

- **S4 — Real `ureq` Transport, `UpdateClient` Background Worker & Platform Downloads/Open Helpers**
  - **Files:** `apps/coursecraft/src/updates.rs`.
  - **Exact changes:**
    1. Implement `UreqHttp` implementing `UpdateHttp` using `ureq::Agent` with `max_redirects(0)`, `http_status_as_error(false)`, connect timeout `10 s`, metadata global timeout `15 s`, download recv timeout `30 s`, sending only `User-Agent: CourseCraft/<version>`, `Accept`, and (for API calls) `X-GitHub-Api-Version: 2022-11-28`.
    2. Implement `downloads_dir() -> Option<PathBuf>` (macOS `$HOME/Downloads`, Windows `%USERPROFILE%\Downloads`, Linux `user-dirs.dirs` `XDG_DOWNLOAD_DIR` fallback `$HOME/Downloads`) and `open_downloaded_file(path: &Path) -> Result<(), String>` (macOS `.dmg` via `Command::new("open").arg(path)`, no shell).
    3. Implement `UpdateClient` holding `http: Arc<dyn UpdateHttp>`, `rx: Option<mpsc::Receiver<UpdateEvent>>`, `busy: UpdateBusy` (`Idle` | `Checking` | `Downloading`), and `cancel: Arc<AtomicBool>`:
       - `start_check(manual: bool, ctx: &egui::Context)` (ignores duplicate if already checking/downloading).
       - `start_download(asset, checksums_url, downloads_dir, ctx)` (throttles `DownloadProgress` sends to every ~100 ms / 128 KiB and calls `ctx.request_repaint()`).
       - `cancel_download()` and `poll() -> Vec<UpdateEvent>`.
    4. Implement `is_portable_install() -> bool` checking whether `std::env::current_exe()` file stem contains `"portable"` (case-insensitive, Windows-relevant, safe fallback `false`).
  - **Tests:** `DL-08`; run `cargo xtask ci` (Gate #1 — verify desktop + wasm builds & clippy are clean before UI wiring).
  - **Done when:** Background worker passes concurrency/dedup tests and `cargo xtask ci` is green.

- **S5 — Safe Markdown Release-Notes Renderer & `Software update` Dialog (`crates/ui-egui`)**
  - **Files:** `crates/ui-egui/src/update_dialog.rs` (new), `crates/ui-egui/src/project_links.rs`.
  - **Exact changes:**
    1. In `crates/ui-egui/src/project_links.rs`, add `pub const RELEASES_API_URL: &str = "https://api.github.com/repos/ad1-kamath/coursecraft/releases?per_page=20";`.
    2. Create `crates/ui-egui/src/update_dialog.rs` (pure egui, zero networking/FS, WASM-safe):
       - Data structs: `UpdateInfo` (`version`, `current_version`, `release_url`, `notes_markdown`, `asset: Option<UpdateAssetInfo>`, `published_at: Option<String>`), `UpdateAssetInfo` (`name`, `size`, `can_open_directly: bool` — `true` for macOS `.dmg`), `UpdateDownloadState` (`Idle`, `Downloading { downloaded: u64, total: Option<u64> }`, `Verified { filename: String, folder_display: String, sha256_verified: bool, can_open_directly: bool }`, `Failed(String)`), `UpdateCheckStatus` (`Idle`, `Checking`, `UpToDate`, `Available(UpdateInfo)`, `Error(String)`), and `UpdateState` (`open: bool`, `status: UpdateCheckStatus`, `download: UpdateDownloadState`, `dismissed_version: Option<String>`).
       - Safe Markdown parser `parse_release_notes(markdown: &str) -> Vec<MdBlock>` and `show_release_notes(ui: &mut Ui, blocks: &[MdBlock])` per §2.8 & §3 (caps at 32 KiB / 300 lines, strips control/bidi chars, renders raw HTML & images as inert text, allows only `https://` links, uses `LayoutJob` + palette tokens only).
       - `show_update_dialog(editor: &mut Editor, ctx: &egui::Context, requests: &mut Vec<EditorRequest>)` using `Dialog::new("Software update").open(&mut open).resizable(true).default_size([520.0, 420.0]).show_with_footer(...)` (zero nested scroll areas, `UX-02`–`UX-18` compliant).
       - `show_status_bar_hint(editor: &mut Editor, ui: &mut Ui)` rendering the quiet `"Update available"` pill when `editor.updates_available && editor.ui.update.has_available_hint()`.
  - **Tests:** `UI-01`, `UI-02`; run `cargo test -p coursecraft-ui-egui -j 4 update_dialog`.
  - **Done when:** Markdown safety/parsing tests and headless dialog state-matrix tests pass in both Dark and Light themes.

- **S6 — Editor Wiring: `EditorRequest`s, Help Menu, Status Bar & Preferences Toggle**
  - **Files:** `crates/ui-egui/src/lib.rs`, `crates/ui-egui/src/menu.rs`, `crates/ui-egui/src/status.rs`, `crates/ui-egui/src/preferences.rs`.
  - **Exact changes:**
    1. In `crates/ui-egui/src/lib.rs`:
       - Declare `pub mod update_dialog;`.
       - Add `pub updates_available: bool` (defaults `false`) + `pub fn set_updates_available(&mut self, available: bool)` on `Editor`.
       - Add `pub update: update_dialog::UpdateState` on `UiState` (plus `pub auto_check_updates: bool` on `PrefsState`, defaulting `true`, with `Editor::set_auto_check_updates(&mut self, enabled: bool)`).
       - Add 7 variants to `EditorRequest`: `CheckForUpdates`, `DownloadUpdate`, `CancelUpdateDownload`, `SkipUpdateVersion(String)`, `RevealDownloadedUpdate`, `OpenDownloadedUpdate`, `SetAutoCheckUpdates(bool)`.
       - Call `update_dialog::show_update_dialog(self, ui.ctx(), &mut requests);` in BOTH the `show_welcome` branch (`lib.rs:4657`) and the normal editor branch (`lib.rs:4844`).
    2. In `crates/ui-egui/src/menu.rs`: pass `requests: &mut Vec<EditorRequest>` to `show_project_help_items`; when `editor.updates_available` is `true`, show `"Check for updates…"` (with quiet right-aligned `"Update available"` hint when `editor.ui.update.has_available_hint()`) which opens the dialog and pushes `EditorRequest::CheckForUpdates` (if not already downloading/showing an update). When `editor.updates_available` is `false` (web editor), omit the update check item.
    3. In `crates/ui-egui/src/status.rs`: inside the right-to-left layout next to the slide-size pill, call `crate::update_dialog::show_status_bar_hint(editor, ui)` only when enough width remains (`RISK-08`).
    4. In `crates/ui-egui/src/preferences.rs`: when `editor.updates_available`, render the `"Updates"` section with `"Check for updates automatically"` checkbox emitting `EditorRequest::SetAutoCheckUpdates(enabled)`.
  - **Tests:** `UI-03`, `UI-04`, `UI-05`; run `cargo xtask ci` (Gate #2 — confirms web editor `wasm32` builds cleanly with zero unused/missing matches).
  - **Done when:** Help menu, status bar pill, Preferences toggle, Welcome screen dialog, and web-editor isolation all pass unit/harness tests and `cargo xtask ci` is green.

- **S7 — Desktop App Shell Integration (`apps/coursecraft/src/main.rs`) & End-to-End Fake-HTTP Shell Tests**
  - **Files:** `apps/coursecraft/src/main.rs`, `apps/coursecraft/src/updates_ui_tests.rs` (new, `#[cfg(all(test, not(target_arch = "wasm32")))]`).
  - **Exact changes:**
    1. On desktop `EditorApp` (`#[cfg(not(target_arch = "wasm32"))]`), add `updates: updates::UpdateClient`, `available_update: Option<updates::ResolvedUpdate>`, and `downloaded_update_path: Option<PathBuf>`.
    2. In `EditorApp::new_desktop`: call `editor.set_updates_available(true)` and `editor.set_auto_check_updates(prefs.auto_check_updates)`.
    3. In `EditorApp::startup`: if `!cfg!(test)` and `updates::should_auto_check(self.prefs.auto_check_updates, self.prefs.last_update_check_unix, now_unix)`, trigger silent `self.updates.start_check(false, &ctx)` (passing `ctx` via `repaint_ctx`).
    4. In `EditorApp::show_app` (desktop block): call `self.poll_updates(ui.ctx())` to process `UpdateEvent`s:
       - Persist `prefs.last_update_check_unix` on completed check or `RateLimited`.
       - On silent startup `Available(resolved)`: if `prefs.skipped_update_version.as_deref() == Some(&resolved.version)`, stay silent; otherwise set `editor.ui.update` to `Available`, and open the dialog automatically **only** if `self.restore_prompt.is_none() && !self.editor.ui.report.crash_prompt && self.editor.session.revision == 0`.
       - On manual check: always populate dialog (`Available` / `UpToDate` / `Error`).
       - On download progress/finish: sync `editor.ui.update.download` and save `self.downloaded_update_path`.
    5. In `EditorApp::handle_request`: handle all 7 new `EditorRequest` variants (`CheckForUpdates`, `DownloadUpdate`, `CancelUpdateDownload`, `SkipUpdateVersion`, `RevealDownloadedUpdate` calling `reveal_in_folder`, `OpenDownloadedUpdate` calling `updates::open_downloaded_file`, `SetAutoCheckUpdates` saving `prefs.json`).
    6. Verify web `EditorApp` (`#[cfg(target_arch = "wasm32")]`) leaves `updates_available == false` and ignores update requests in `_ => {}` with zero network calls.
    7. Add `updates_ui_tests.rs` driving `EditorApp` with an injected fake `UpdateHttp` and temp `config_dir` + temp `downloads_dir` (zero live network, zero writes to user's real Downloads).
  - **Tests:** `UI-06`; run `cargo test -p coursecraft -j 4 updates`.
  - **Done when:** Silent startup check, daily throttle persistence, skip-version persistence, manual re-check of skipped version, opt-out preference, and download-to-temp-dir all pass end-to-end inside `EditorApp`.

- **S8 — Release Workflow `SHA256SUMS.txt`, M28 UI/UX Sweep, Docs & Final Release Gate**
  - **Files:** `.github/workflows/release.yml`, `crates/ui-egui/src/snapshot.rs`, `crates/ui-egui/src/uiux_sweep_tests.rs`, `docs/testing/m35-report.md` (new implementer report).
  - **Exact changes:**
    1. In `.github/workflows/release.yml` inside the `release` job (between `actions/download-artifact@v8` and `softprops/action-gh-release@v3`), add:
       ```yaml
       - name: Generate SHA-256 checksums
         working-directory: dist
         run: sha256sum CourseCraft-* > SHA256SUMS.txt
       ```
    2. In `crates/ui-egui/src/snapshot.rs`, add `window!("update", "Software update", ...)` populating `editor.updates_available = true` and a realistic multi-section Markdown `UpdateInfo` (plus status-bar hint).
    3. In `crates/ui-egui/src/uiux_sweep_tests.rs`, include `"update"` in the full surface sweep matrix so all M28 rules (`UX-01`–`UX-19`) run automatically across sizes, themes, and scales (`1.0`, `1.5`, `2.0`). Fix any layout/contrast/copy findings surfaced by the sweep.
    4. Write concise `docs/testing/m35-report.md` summarizing implemented behaviour, test counts, and any follow-ups.
  - **Tests:** `UI-07`, `UI-08`; run `cargo test -p coursecraft-ui-egui -j 4 uiux_sweep`.
  - **Final Release Gate:** `cargo xtask ci` and `cargo xtask build-player`.
  - **Done when:** `SHA256SUMS.txt` is wired in `release.yml`, M28 UI/UX sweep passes with 0 MUST violations on the update dialog and status-bar hint, and the full release gate passes.

---

## 7. Release Gate for M35

### 7.1 Automated Release Gate Commands
Run in order before cutting the release:
1. `cargo xtask ci` (`fmt --check`, `clippy --workspace --all-targets -D warnings`, `test --workspace`, `layers`, `wasm`)
2. `cargo xtask build-player` (confirm embedded WASM web player builds cleanly with zero update code)
3. `cargo test -p coursecraft -j 4 updates` (all `UP-*`, `DL-*`, and `UI-06` fake-HTTP shell tests)
4. `cargo test -p coursecraft-ui-egui -j 4 update_dialog` (all `UI-01`–`UI-04` Markdown & dialog tests)
5. `cargo test -p coursecraft-ui-egui -j 4 uiux_sweep` (confirms 0 MUST M28 UI/UX violations across all sizes, themes, and scales)

### 7.2 Owner Manual Verification Checklist (Including Real GitHub Test — 10 Minutes)
Because implementers run offline and never hit live GitHub, the owner verifies these real-world checks before tagging:

1. **Real GitHub check (up-to-date case):** Launch the built desktop app (at current version `0.4.3` or new version). Click **Help → Check for updates…**. Confirm the dialog responds in ~1 s with *"You're up to date."* and no UI freeze.
2. **Real GitHub check + real download (simulated older version):** Temporarily test a local build with an older version (or inspect against a published newer tag / test release, e.g. temporarily building with version `0.4.2` against live `v0.4.3` on GitHub):
   - Confirm quiet startup check finds `v0.4.3`, opens the calm **Software update** dialog once, and renders the real `v0.4.3` Markdown release notes cleanly (bold lead-ins, bullet lists, no clipped lines, readable in both Dark and Light themes).
   - Click **Later** → confirm the dialog closes and the quiet **Update available** pill appears in the status bar and **Help** menu without nagging.
   - Reopen via the status-bar pill and click **Download update** → confirm live progress bar updates smoothly while dragging/scrolling the editor (UI never freezes), file lands in `~/Downloads/CourseCraft-0.4.3-macos.dmg` (or `-setup.exe` on Windows / `-linux-x86_64.tar.gz` on Linux), reports **SHA-256 verified**, and **Show in folder** / **Open disk image** reveals/mounts it cleanly.
   - Verify no leftover `.part` file exists in `~/Downloads`.
3. **Skip this version & daily throttle:** Click **Skip this version**, quit, and relaunch — confirm no startup dialog and no status-bar pill appear. Open **Help → Check for updates…** manually — confirm it still shows `v0.4.3` on demand.
4. **Preferences opt-out:** Open **Preferences**, uncheck **Check for updates automatically**, remove `last_update_check_unix` from `prefs.json`, relaunch — confirm zero network requests occur on startup.
5. **Offline silence:** Turn Wi-Fi off, clear `last_update_check_unix`, relaunch — confirm zero popups or error banners on startup. Then click **Help → Check for updates…** — confirm a calm offline message with **Try again** and **Open releases page**.
6. **Web editor & published course check:** Open the web editor (`trunk serve` in `apps/coursecraft`) and a published web course — confirm **Help** has no **Check for updates…** item, **Preferences** has no update toggle, and the browser Network tab shows zero requests to `api.github.com`.
7. **Release workflow check (on next tag):** After pushing the next `v*` tag, inspect the draft GitHub release and verify `SHA256SUMS.txt` is attached and its SHA-256 hashes match the uploaded `.dmg`, `.exe`, and `.tar.gz` files.

---

## 8. Summary

M35 delivers a calm, safe, desktop-only in-app update checker and direct installer downloader for CourseCraft with **zero new external crates** (`ureq`, `semver`, and `ring` are already in `Cargo.lock`), **zero changes to `coursecraft-engine`**, **zero network calls in the web editor or published player**, and **minimal overlap with M34**. All release parsing, semver pre-release handling (supporting today's `0.x` GitHub pre-releases while transitioning automatically to stable-only at `1.0.0`), OS/CPU asset picking (matching real `v0.4.3` filenames), redirect host allowlisting, streaming size + SHA-256 verification, safe Markdown rendering, and `EditorApp` wiring are split into **8 small, offline-testable steps (`S1`–`S8`)** gated by `cargo xtask ci`, `cargo xtask build-player`, the M28 UI/UX sweep, and a short real-GitHub owner checklist.
