//! The update check of the Craft suite (`craft_update`, as DeckCraft uses it): one quiet check
//! at start-up and one on demand from Help, the prompt that offers the update, and the window
//! that reports a requested check. The flow itself is `craft_update::updater::Updater`.
//!
//! Platform update behaviour from 0.5.0:
//! - macOS: `craft-update` selects the `.dmg` and offers self-update when writable.
//! - Linux: `craft-update` detects `$APPIMAGE` (`Install::AppImage` -> `PackageKind::AppImage`,
//!   picking `coursecraft-<ver>-linux-x86_64.AppImage`) or falls back to `Install::Standalone`
//!   (`PackageKind::TarGz`, picking `coursecraft-<ver>-linux-x86_64.tar.gz`).
//! - Windows: releases ship only the Inno Setup `-setup.exe` installer (no portable `.zip`).
//!   Because `craft-update` maps Windows standalone/portable copies to `PackageKind::PortableZip`,
//!   CourseCraft intercepts discovered releases on Windows and shows `craft-update`'s prompt in
//!   manual-download mode pointing at `-setup.exe` — never attempting a portable self-update,
//!   never erroring, and never installing or restarting silently. Legacy portable copies
//!   (`portable.txt` beside the executable) also show a short notice that the portable build is
//!   no longer published.
//!
//! Desktop only; the web build has no updates.

use std::ffi::OsStr;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use craft_update::release::{expected_download_url, safe_web_link, same_download_url};
use craft_update::translate::Note;
use craft_update::updater::{Action, CheckSource, Status, Updater};
use craft_update::{
    Arch, Asset, CheckResultDialog, CheckResultResponse, Os, PackageKind, Platform, Release,
    StartupPrompt, StartupPromptResponse, pick_asset,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Notice shown in the update prompt when running an older Windows portable build (`portable.txt`
/// beside the executable), since portable builds stopped in 0.5.0.
pub const PORTABLE_DISCONTINUED_NOTE: &str = "The portable Windows version is no longer published. Download the Windows installer (-setup.exe) to update.";

/// CourseCraft as craft-update knows it: its releases live in the upstream repository, and the
/// prompt shows its icon and teal accent.
pub const UPDATE_APP: craft_update::App = craft_update::App {
    icon_png: Some(include_bytes!("../icons/window-256.png")),
    accent: Some([0x0B, 0x7F, 0x72]),
    // The macOS bundle's identifier (scripts/package-macos.sh), kept for existing installs.
    bundle_id_override: Some("org.coursecraft.CourseCraft"),
    ..craft_update::App::new(
        "coursecraft",
        "CourseCraft",
        "eLearning authoring",
        "ad1-kamath/coursecraft",
    )
};

/// How this running copy handles discovered updates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) enum InstallMode {
    /// macOS and Linux: delegated directly to `craft_update::Updater` (`.dmg`, `.AppImage` when
    /// `$APPIMAGE` is set, `.tar.gz` otherwise).
    CraftUpdate,
    /// Windows Inno Setup installation: manual download of `-setup.exe`.
    WindowsSetup,
    /// Legacy Windows portable copy (`portable.txt` beside the executable): manual download of
    /// `-setup.exe` with [`PORTABLE_DISCONTINUED_NOTE`].
    WindowsLegacyPortable,
}

/// Whether `exe` belongs to an older Windows portable build (marked by `portable.txt` beside it).
#[must_use]
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn is_legacy_portable_at(exe: &Path) -> bool {
    exe.parent()
        .is_some_and(|dir| dir.join("portable.txt").is_file())
}

/// Detects the update handling mode for the current OS and executable.
fn detect_install_mode() -> InstallMode {
    #[cfg(windows)]
    {
        if std::env::current_exe()
            .ok()
            .is_some_and(|exe| is_legacy_portable_at(&exe))
        {
            InstallMode::WindowsLegacyPortable
        } else {
            InstallMode::WindowsSetup
        }
    }
    #[cfg(not(windows))]
    {
        InstallMode::CraftUpdate
    }
}

/// The Linux package format matching the running process: `.AppImage` when `$APPIMAGE` is set
/// and non-empty, `.tar.gz` otherwise (matching `craft_update::self_update`'s Linux detection).
#[must_use]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn linux_package_kind(appimage_env: Option<&OsStr>) -> PackageKind {
    if appimage_env.is_some_and(|value| !value.is_empty()) {
        PackageKind::AppImage
    } else {
        PackageKind::TarGz
    }
}

/// Picks the matching Linux release asset (`.AppImage` when `$APPIMAGE` is set, `.tar.gz`
/// otherwise) using `craft-update`'s asset matcher.
#[must_use]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn select_linux_asset<'a>(
    release: &'a Release,
    arch: Arch,
    appimage_env: Option<&OsStr>,
) -> Option<(&'a Asset, PackageKind)> {
    let stems: Vec<&str> = UPDATE_APP.stems().collect();
    let kind = linux_package_kind(appimage_env);
    pick_asset(
        release,
        &stems,
        Platform {
            os: Os::Linux,
            arch,
        },
        &[kind],
    )
}

/// Preferred Windows asset platform tokens for `arch` (native first, then emulated).
fn windows_tokens(arch: Arch) -> &'static [&'static str] {
    match arch {
        Arch::X64 => &["windows-x64", "windows-x86"],
        Arch::Arm64 => &["windows-arm64", "windows-x64", "windows-x86"],
        Arch::X86 => &["windows-x86"],
        Arch::Riscv64 => &[],
    }
}

/// Parses a Windows Inno Setup installer asset name (`<stem>-<version>-windows-<arch>-setup.exe`),
/// returning its `<os>-<arch>` token when it matches one of `stems` and a valid version.
fn parse_windows_setup_token(name: &str, stems: &[&str]) -> Option<String> {
    let base = name.strip_suffix("-setup.exe")?;
    for stem in stems {
        let Some(rest) = base.strip_prefix(stem).and_then(|r| r.strip_prefix('-')) else {
            continue;
        };
        let mut parts = rest.rsplitn(3, '-');
        let (Some(arch), Some(os), Some(version)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        if os != "windows"
            || craft_update::parse_version(version).is_none()
            || !version.starts_with(|c: char| c.is_ascii_digit())
        {
            continue;
        }
        return Some(format!("{os}-{arch}"));
    }
    None
}

/// Picks the Windows `-setup.exe` installer asset from `release` for `arch`. Never selects a
/// portable zip or a non-Windows file.
#[must_use]
pub(crate) fn select_windows_setup_asset<'a>(
    release: &'a Release,
    stems: &[&str],
    arch: Arch,
) -> Option<&'a Asset> {
    let parsed: Vec<(&Asset, String)> = release
        .assets
        .iter()
        .filter_map(|asset| {
            parse_windows_setup_token(&asset.name, stems).map(|token| (asset, token))
        })
        .collect();
    for token in windows_tokens(arch) {
        if let Some((asset, _)) = parsed.iter().find(|(_, t)| t == token) {
            return Some(*asset);
        }
    }
    None
}

/// The safe download link for a Windows update: the release's `-setup.exe` asset when present and
/// bound to `app`'s repository/tag, otherwise the release page or the app's releases page.
#[must_use]
pub(crate) fn windows_download_url(
    app: &craft_update::App,
    release: &Release,
    arch: Arch,
) -> Option<String> {
    let stems: Vec<&str> = app.stems().collect();
    let repo = if release.repo.is_empty() {
        app.repo
    } else {
        release.repo.as_str()
    };
    if let Some(asset) = select_windows_setup_asset(release, &stems, arch)
        && safe_web_link(&asset.url)
        && expected_download_url(repo, &release.tag, &asset.name)
            .is_some_and(|expected| same_download_url(&asset.url, &expected))
    {
        return Some(asset.url.clone());
    }
    if safe_web_link(&release.html_url) {
        return Some(release.html_url.clone());
    }
    let releases = app.releases_url();
    safe_web_link(&releases).then_some(releases)
}

fn current_windows_arch() -> Arch {
    Platform::current().map_or(Arch::X64, |p| p.arch)
}

fn running_version() -> String {
    craft_update::self_update::current_version(&UPDATE_APP, VERSION)
        .map_or_else(|| VERSION.to_owned(), |v| v.to_string())
}

fn build_windows_prompt(release: Release, mode: InstallMode) -> StartupPrompt {
    // Manual download only: never attempt PortableZip self-update or silent install/restart.
    let mut prompt =
        StartupPrompt::with_self_update(&UPDATE_APP, release, running_version(), false)
            .with_translator(translate);
    if mode == InstallMode::WindowsLegacyPortable {
        prompt.set_note(Note::new(PORTABLE_DISCONTINUED_NOTE, &[]));
    }
    prompt
}

/// The updater and whether this run checks at start.
pub struct Runtime {
    updater: Updater,
    mode: InstallMode,
    /// Release intercepted from the background check thread in Windows modes so CourseCraft can
    /// show the manual `-setup.exe` prompt instead of `craft-update`'s `PortableZip` flow.
    intercepted: Arc<Mutex<Option<Release>>>,
    windows_prompt: Option<StartupPrompt>,
    /// The desktop host turns the check at start on (preference on, not CI); tests leave it off.
    enabled: bool,
    check_requested: bool,
}

impl Default for Runtime {
    fn default() -> Self {
        Self::with_mode(detect_install_mode(), None)
    }
}

/// The prompt's words in the interface language, through the editor's catalogs.
fn translate(text: &str, args: &[(&str, &str)]) -> String {
    coursecraft_ui_egui::i18n::translate_with(text, args)
}

impl Runtime {
    /// Builds a runtime for `mode`, optionally substituting `source` for GitHub API calls (tests).
    fn with_mode(mode: InstallMode, source: Option<CheckSource>) -> Self {
        let intercepted = Arc::new(Mutex::new(None));
        let mut updater = Updater::new(&UPDATE_APP, VERSION).with_translator(translate);
        match mode {
            InstallMode::CraftUpdate => {
                if let Some(src) = source {
                    updater = updater.with_source(src);
                }
            }
            InstallMode::WindowsSetup | InstallMode::WindowsLegacyPortable => {
                let slot = Arc::clone(&intercepted);
                let wrapped: CheckSource = Arc::new(move || {
                    let result = match &source {
                        Some(ask) => ask(),
                        None => craft_update::Http::new(&format!("{}/Update", UPDATE_APP.name))
                            .and_then(|http| {
                                craft_update::check_for_update(&UPDATE_APP, &http, VERSION)
                            }),
                    };
                    match result {
                        Ok(Some(release)) => {
                            *slot.lock().unwrap_or_else(PoisonError::into_inner) = Some(release);
                            Ok(None)
                        }
                        other => other,
                    }
                });
                updater = updater.with_source(wrapped);
            }
        }
        Self {
            updater,
            mode,
            intercepted,
            windows_prompt: None,
            enabled: false,
            check_requested: false,
        }
    }

    /// Turns the check at start on or off.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Help ▸ Check for Updates: runs a check and reports the outcome either way.
    pub fn request_check(&mut self) {
        if self.windows_prompt.is_none() && !matches!(self.updater.status(), Status::Checking) {
            self.check_requested = true;
        }
    }

    /// Starts the helper of an update that is ready, so that it installs once the process has
    /// exited. Later calls do nothing.
    pub fn start_pending_update(&mut self) {
        self.updater.start_pending_update();
    }

    /// Runs the checks and the installation worker; call once a frame.
    pub fn tick(&mut self, ctx: &egui::Context) {
        if std::mem::take(&mut self.check_requested) {
            self.updater.check_now(ctx);
        } else if self.enabled {
            self.updater.check_at_start(ctx);
        }
        self.updater.tick(ctx);
        if self.mode != InstallMode::CraftUpdate {
            if self.updater.managed().is_some() {
                self.windows_prompt = None;
                self.intercepted
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .take();
            } else if let Some(release) = self
                .intercepted
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .take()
            {
                self.updater.dismiss_status();
                self.windows_prompt = Some(build_windows_prompt(release, self.mode));
            }
        }
    }

    /// Draws the prompt and the result window. `clean`: nothing is unsaved, so an installation
    /// may start and a ready update may ask the app to close.
    pub fn show(&mut self, ctx: &egui::Context, clean: bool) {
        match self.show_action(ctx, clean) {
            Action::OpenUrl(url) => ctx.open_url(egui::OpenUrl::new_tab(url)),
            // The app's close path asks about unsaved work; `on_exit` starts the helper.
            Action::Close => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            Action::None => {}
        }
    }

    /// Draws the prompt and returns the resulting action without executing OS side effects.
    fn show_action(&mut self, ctx: &egui::Context, clean: bool) -> Action {
        if let Some(prompt) = &mut self.windows_prompt {
            return match prompt.show(ctx) {
                StartupPromptResponse::Pending | StartupPromptResponse::UpdateAndRestart => {
                    Action::None
                }
                StartupPromptResponse::Download => {
                    let url =
                        windows_download_url(&UPDATE_APP, &prompt.release, current_windows_arch());
                    self.windows_prompt = None;
                    url.map_or(Action::None, Action::OpenUrl)
                }
                StartupPromptResponse::Later => {
                    self.windows_prompt = None;
                    Action::None
                }
            };
        }
        let action = self.updater.show_if(ctx, clean);
        if !matches!(self.updater.status(), Status::Idle) {
            let response = CheckResultDialog::new(&UPDATE_APP, VERSION)
                .with_translator(translate)
                .show(ctx, self.updater.status());
            self.answer(response);
        }
        action
    }

    /// Acts on the result window's answer.
    fn answer(&mut self, response: CheckResultResponse) {
        match response {
            CheckResultResponse::Close => self.updater.dismiss_status(),
            CheckResultResponse::CheckAgain => self.check_requested = true,
            // The next frame's `show_if` asks the window to close once nothing is unsaved.
            CheckResultResponse::RestartToUpdate => self.updater.restart_to_update(),
            CheckResultResponse::None => {}
        }
    }
}

/// Whether `CI` or `COURSECRAFT_DISABLE_UPDATES` turn the check at start off.
#[must_use]
pub fn suppressed_by_env() -> bool {
    let set = |name: &str| {
        std::env::var(name).is_ok_and(|v| {
            let v = v.trim();
            !v.is_empty() && v != "0" && !v.eq_ignore_ascii_case("false")
        })
    };
    set("CI") || set("COURSECRAFT_DISABLE_UPDATES")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A runtime whose updater keeps its records under `root`, reads no launcher choice and asks
    /// no server, with the check at start on.
    fn runtime_with_records(root: &std::path::Path) -> Runtime {
        let mut rt = Runtime::with_mode(
            InstallMode::CraftUpdate,
            Some(std::sync::Arc::new(|| Ok(None))),
        );
        rt.updater = rt
            .updater
            .with_paths(craft_update::Paths::under(root))
            .with_pins_path(root.join("pins.json"));
        rt.enabled = true;
        rt
    }

    fn windows_runtime(
        root: &std::path::Path,
        mode: InstallMode,
        release: Option<Release>,
    ) -> Runtime {
        let mut rt = Runtime::with_mode(mode, Some(Arc::new(move || Ok(release.clone()))));
        rt.updater = rt
            .updater
            .with_paths(craft_update::Paths::under(root))
            .with_pins_path(root.join("pins.json"));
        rt
    }

    fn records_dir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("coursecraft-updates-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn frame(rt: &mut Runtime, ctx: &egui::Context) -> Action {
        let mut action = Action::None;
        ctx.run_ui(egui::RawInput::default(), |ui| {
            rt.tick(ui.ctx());
            action = rt.show_action(ui.ctx(), true);
        })
        .textures_delta
        .clear();
        action
    }

    fn release_0_5_0() -> Release {
        let tag = "v0.5.0";
        let repo = UPDATE_APP.repo;
        let names = [
            "coursecraft-0.5.0-macos-arm64.dmg",
            "coursecraft-0.5.0-windows-x64-setup.exe",
            "coursecraft-0.5.0-linux-x86_64.tar.gz",
            "coursecraft-0.5.0-linux-x86_64.AppImage",
            "SHA256SUMS.txt",
        ];
        Release {
            tag: tag.into(),
            version: craft_update::parse_version("0.5.0"),
            name: "CourseCraft v0.5.0".into(),
            html_url: format!("https://github.com/{repo}/releases/tag/{tag}"),
            repo: repo.into(),
            assets: names
                .iter()
                .map(|n| Asset {
                    name: (*n).into(),
                    size: 1,
                    url: format!("https://github.com/{repo}/releases/download/{tag}/{n}"),
                    sha256: None,
                })
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn the_app_is_described_by_its_upstream_repository() {
        assert_eq!(UPDATE_APP.repo, "ad1-kamath/coursecraft");
        assert_eq!(
            UPDATE_APP.releases_url(),
            "https://github.com/ad1-kamath/coursecraft/releases"
        );
    }

    /// The file names release.yml and package-macos.sh make are the ones craft-update and
    /// CourseCraft's Windows selector pick, with no portable Windows package.
    #[test]
    fn the_release_file_names_are_the_ones_the_update_picks() {
        let release = release_0_5_0();
        let stems: Vec<&str> = UPDATE_APP.stems().collect();
        for (os, arch, kind, expected) in [
            (
                Os::MacOs,
                Arch::Arm64,
                PackageKind::Dmg,
                "coursecraft-0.5.0-macos-arm64.dmg",
            ),
            (
                Os::Linux,
                Arch::X64,
                PackageKind::TarGz,
                "coursecraft-0.5.0-linux-x86_64.tar.gz",
            ),
            (
                Os::Linux,
                Arch::X64,
                PackageKind::AppImage,
                "coursecraft-0.5.0-linux-x86_64.AppImage",
            ),
        ] {
            let picked = pick_asset(&release, &stems, Platform { os, arch }, &[kind]);
            assert_eq!(picked.map(|(a, _)| a.name.as_str()), Some(expected));
        }
        assert_eq!(
            select_windows_setup_asset(&release, &stems, Arch::X64).map(|a| a.name.as_str()),
            Some("coursecraft-0.5.0-windows-x64-setup.exe")
        );

        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let yml = std::fs::read_to_string(root.join(".github/workflows/release.yml")).unwrap();
        for part in [
            "coursecraft-$ver-windows-x64-setup",
            "coursecraft-${GITHUB_REF_NAME#v}-linux-x86_64",
            "coursecraft-$ver-linux-x86_64.AppImage",
            "$root/bin/coursecraft",
            "packaging/linux/coursecraft.desktop",
            "apps/coursecraft/icons/coursecraft-1024.png",
            "sha256sum coursecraft-* > SHA256SUMS.txt",
        ] {
            assert!(yml.contains(part), "release.yml lacks {part}");
        }
        for forbidden in ["portable.txt", "-windows-x64-portable", "-portable.zip"] {
            assert!(
                !yml.contains(forbidden),
                "release.yml must not reference {forbidden}"
            );
        }
        let mac = std::fs::read_to_string(root.join("scripts/package-macos.sh")).unwrap();
        assert!(mac.contains("coursecraft-$version-macos-$arch.dmg"));
    }

    #[test]
    fn linux_asset_selection_uses_appimage_when_env_set_and_tarball_otherwise() {
        let release = release_0_5_0();

        let (asset, kind) = select_linux_asset(
            &release,
            Arch::X64,
            Some(OsStr::new(
                "/home/user/Applications/coursecraft-0.4.6-linux-x86_64.AppImage",
            )),
        )
        .expect("AppImage selected when APPIMAGE is set");
        assert_eq!(kind, PackageKind::AppImage);
        assert_eq!(asset.name, "coursecraft-0.5.0-linux-x86_64.AppImage");

        let (asset, kind) =
            select_linux_asset(&release, Arch::X64, None).expect("tarball when APPIMAGE unset");
        assert_eq!(kind, PackageKind::TarGz);
        assert_eq!(asset.name, "coursecraft-0.5.0-linux-x86_64.tar.gz");

        // An empty APPIMAGE variable does not count as running from an AppImage.
        let (asset, kind) = select_linux_asset(&release, Arch::X64, Some(OsStr::new("")))
            .expect("tarball when APPIMAGE empty");
        assert_eq!(kind, PackageKind::TarGz);
        assert_eq!(asset.name, "coursecraft-0.5.0-linux-x86_64.tar.gz");

        // Wrong architecture never falls back to x86_64 on Linux.
        assert_eq!(select_linux_asset(&release, Arch::Arm64, None), None);
        assert_eq!(
            select_linux_asset(&release, Arch::Arm64, Some(OsStr::new("/tmp/a.AppImage"))),
            None
        );
    }

    #[test]
    fn windows_setup_selection_never_picks_portable_and_validates_urls() {
        let mut release = release_0_5_0();
        // Even if an older release lists a portable zip first, Windows selection picks -setup.exe.
        release.assets.insert(
            0,
            Asset {
                name: "coursecraft-0.5.0-windows-x64-portable.zip".into(),
                size: 1,
                url: "https://github.com/ad1-kamath/coursecraft/releases/download/v0.5.0/coursecraft-0.5.0-windows-x64-portable.zip".into(),
                sha256: None,
            },
        );
        let url = windows_download_url(&UPDATE_APP, &release, Arch::X64).unwrap();
        assert_eq!(
            url,
            "https://github.com/ad1-kamath/coursecraft/releases/download/v0.5.0/coursecraft-0.5.0-windows-x64-setup.exe"
        );
        // Windows ARM64 falls back to x64 installer cleanly.
        assert_eq!(
            windows_download_url(&UPDATE_APP, &release, Arch::Arm64).as_deref(),
            Some(url.as_str())
        );

        // If a release has a forged asset URL, refuse it and fall back to the release HTML page.
        let mut forged = release_0_5_0();
        for asset in &mut forged.assets {
            if asset.name.ends_with("-setup.exe") {
                asset.url = "https://evil.example/coursecraft-0.5.0-windows-x64-setup.exe".into();
            }
        }
        assert_eq!(
            windows_download_url(&UPDATE_APP, &forged, Arch::X64).as_deref(),
            Some("https://github.com/ad1-kamath/coursecraft/releases/tag/v0.5.0")
        );

        // If no -setup.exe asset exists at all, fall back to the release page without error.
        let mut no_setup = release_0_5_0();
        no_setup.assets.retain(|a| !a.name.ends_with("-setup.exe"));
        assert_eq!(
            windows_download_url(&UPDATE_APP, &no_setup, Arch::X64).as_deref(),
            Some("https://github.com/ad1-kamath/coursecraft/releases/tag/v0.5.0")
        );
    }

    #[test]
    fn legacy_portable_marker_detection_checks_sibling_portable_txt() {
        let dir = records_dir("portable-marker");
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("coursecraft.exe");
        std::fs::write(&exe, b"exe").unwrap();
        assert!(!is_legacy_portable_at(&exe));
        std::fs::write(dir.join("portable.txt"), b"").unwrap();
        assert!(is_legacy_portable_at(&exe));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn windows_legacy_portable_offers_setup_exe_with_notice_and_no_error() {
        let root = records_dir("win-portable");
        let mut rt = windows_runtime(
            &root,
            InstallMode::WindowsLegacyPortable,
            Some(release_0_5_0()),
        );
        let ctx = egui::Context::default();
        rt.request_check();
        for _ in 0..200 {
            let action = frame(&mut rt, &ctx);
            assert_eq!(action, Action::None, "must never silently install or close");
            if rt.windows_prompt.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        let prompt = rt
            .windows_prompt
            .as_ref()
            .expect("Windows legacy portable copy should open the update prompt");
        assert!(
            !prompt.is_self_updating,
            "must not attempt PortableZip self-update"
        );
        assert_eq!(rt.updater.status(), &Status::Idle, "must not show an error");
        let msg = prompt
            .message()
            .expect("legacy portable prompt must explain portable builds stopped");
        assert!(msg.contains("portable"), "{msg}");
        assert!(msg.contains("no longer published"), "{msg}");
        assert!(msg.contains("-setup.exe"), "{msg}");

        // Choosing Download points straight to the -setup.exe installer.
        let url = windows_download_url(&UPDATE_APP, &prompt.release, Arch::X64).unwrap();
        assert!(
            url.ends_with("coursecraft-0.5.0-windows-x64-setup.exe"),
            "{url}"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn windows_setup_offers_setup_exe_without_portable_notice_or_silent_restart() {
        let root = records_dir("win-setup");
        let mut rt = windows_runtime(&root, InstallMode::WindowsSetup, Some(release_0_5_0()));
        let ctx = egui::Context::default();
        rt.request_check();
        for _ in 0..200 {
            let action = frame(&mut rt, &ctx);
            assert_eq!(action, Action::None, "must never silently install or close");
            if rt.windows_prompt.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        let prompt = rt
            .windows_prompt
            .as_ref()
            .expect("Windows installer copy should open the update prompt");
        assert!(!prompt.is_self_updating);
        assert_eq!(rt.updater.status(), &Status::Idle);
        assert_eq!(prompt.message(), None);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_offered_release_opens_the_prompt() {
        let mut rt = Runtime::with_mode(InstallMode::CraftUpdate, None);
        let ctx = egui::Context::default();
        assert!(!rt.updater.prompt_open());
        rt.updater.offer(craft_update::Release {
            tag: "v99.0.0".into(),
            ..Default::default()
        });
        ctx.run_ui(egui::RawInput::default(), |ui| rt.show(ui.ctx(), true))
            .textures_delta
            .clear();
        assert!(rt.updater.prompt_open());
        // Offering an update never silently installs or closes the app.
        assert_eq!(rt.updater.status(), &Status::Idle);
    }

    #[test]
    fn the_start_after_an_update_reports_it_until_dismissed() {
        let root = records_dir("installed");
        std::fs::create_dir_all(root.join("data")).unwrap();
        std::fs::write(root.join("data").join("self-update-done.txt"), VERSION).unwrap();
        let mut rt = runtime_with_records(&root);
        let ctx = egui::Context::default();
        frame(&mut rt, &ctx);
        assert!(
            matches!(rt.updater.status(), Status::Updated(_)),
            "{:?}",
            rt.updater.status()
        );
        rt.answer(CheckResultResponse::Close);
        assert_eq!(rt.updater.status(), &Status::Idle);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_requested_check_with_nothing_newer_says_so() {
        let root = records_dir("uptodate");
        let mut rt = runtime_with_records(&root);
        rt.enabled = false;
        let ctx = egui::Context::default();
        rt.request_check();
        for _ in 0..200 {
            frame(&mut rt, &ctx);
            if !matches!(rt.updater.status(), Status::Checking) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(rt.updater.status(), &Status::UpToDate);
        rt.answer(CheckResultResponse::CheckAgain);
        assert!(rt.check_requested);
        let _ = std::fs::remove_dir_all(&root);
    }
}
