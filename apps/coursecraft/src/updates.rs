//! Desktop-only update check policy, version helpers, and download limits.
//!
//! All networking, version parsing, and Downloads I/O for CourseCraft updates
//! live in this module behind `#[cfg(not(target_arch = "wasm32"))]`.

/// Minimum elapsed seconds between automatic startup update checks (24 hours).
pub const CHECK_INTERVAL_SECS: u64 = 86_400;

/// Hard size cap for a GitHub Releases API JSON response (1 MiB).
pub const MAX_API_RESPONSE_BYTES: usize = 1024 * 1024;

/// Hard size cap for a `SHA256SUMS.txt` checksum manifest (64 KiB).
pub const MAX_SUMS_BYTES: usize = 64 * 1024;

/// Hard size cap for a downloaded installer package (512 MiB).
pub const MAX_DOWNLOAD_BYTES: u64 = 512 * 1024 * 1024;

/// Maximum number of HTTPS redirect hops allowed per request.
pub const MAX_REDIRECTS: usize = 5;

/// Decide whether an automatic startup update check should run now.
///
/// Returns `false` when automatic checks are disabled or when the system clock
/// moved backwards relative to `last_unix`. Returns `true` when no check has
/// been recorded yet, or at least [`CHECK_INTERVAL_SECS`] seconds have elapsed.
#[must_use]
pub fn should_auto_check(enabled: bool, last_unix: Option<u64>, now_unix: u64) -> bool {
    if !enabled {
        return false;
    }
    let Some(last) = last_unix else {
        return true;
    };
    if now_unix < last {
        return false;
    }
    now_unix.saturating_sub(last) >= CHECK_INTERVAL_SECS
}

fn env_flag_truthy(val: Option<&str>) -> bool {
    let Some(raw) = val else {
        return false;
    };
    let trimmed = raw.trim();
    !trimmed.is_empty() && trimmed != "0" && !trimmed.eq_ignore_ascii_case("false")
}

/// Return `true` when environment variables suppress automatic update checks
/// (for example in CI or headless runs).
#[must_use]
pub fn auto_check_suppressed_by_env(ci_env: Option<&str>, disable_env: Option<&str>) -> bool {
    env_flag_truthy(ci_env) || env_flag_truthy(disable_env)
}

/// Read `CI` and `COURSECRAFT_DISABLE_UPDATES` from the process environment
/// and report whether automatic update checks are suppressed.
#[must_use]
pub fn is_ci_or_headless_env() -> bool {
    let ci = std::env::var("CI").ok();
    let disable = std::env::var("COURSECRAFT_DISABLE_UPDATES").ok();
    auto_check_suppressed_by_env(ci.as_deref(), disable.as_deref())
}

/// Build the minimal privacy-preserving `User-Agent` header value.
///
/// Sends only `CourseCraft/<version>` with no OS, CPU, hostname, locale, or IDs.
#[must_use]
pub fn user_agent(version: &str) -> String {
    format!("CourseCraft/{version}")
}

/// Parse the running CourseCraft crate version as SemVer.
#[must_use]
pub fn current_version() -> Option<semver::Version> {
    semver::Version::parse(env!("CARGO_PKG_VERSION")).ok()
}

/// Errors that can occur while checking for or downloading a CourseCraft update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateError {
    /// Network-level failure (offline, DNS, connection refused, TLS).
    Network(String),
    /// Request or download stream timed out.
    Timeout,
    /// GitHub API rate limit reached (HTTP 403 with remaining 0, or HTTP 429).
    RateLimited,
    /// Unexpected non-success HTTP status code.
    HttpStatus(u16),
    /// Empty, malformed, non-array, or oversized API response.
    InvalidResponse,
    /// Target or redirect URL violated the HTTPS / GitHub host allowlist.
    DisallowedUrl,
    /// Redirect chain exceeded [`MAX_REDIRECTS`].
    TooManyRedirects,
    /// Download exceeded [`MAX_DOWNLOAD_BYTES`] or declared asset size.
    DownloadTooLarge,
    /// Download ended with fewer bytes than the declared asset size.
    DownloadTruncated,
    /// Computed SHA-256 digest did not match the expected checksum.
    ChecksumMismatch,
    /// User cancelled an in-progress download.
    Cancelled,
    /// Local filesystem error while writing or moving the downloaded file.
    Io(String),
}

impl UpdateError {
    /// Return a calm, plain-language explanation suitable for display in the UI.
    #[must_use]
    pub fn user_message(&self) -> String {
        match self {
            Self::Network(_) => {
                "Couldn't check for updates. Check your internet connection and try again."
                    .to_string()
            }
            Self::Timeout => "The update server took too long to respond.".to_string(),
            Self::RateLimited => {
                "GitHub's update service is temporarily busy. Try again later or open the releases page in your browser."
                    .to_string()
            }
            Self::HttpStatus(code) => format!(
                "GitHub returned an unexpected status (HTTP {code}). Try again later or open the releases page in your browser."
            ),
            Self::InvalidResponse => "Received an unexpected response from GitHub.".to_string(),
            Self::DisallowedUrl => {
                "The download link pointed to an unexpected address and was blocked.".to_string()
            }
            Self::TooManyRedirects => {
                "The download link redirected too many times and was stopped.".to_string()
            }
            Self::DownloadTooLarge => {
                "The update file is larger than expected and was not saved.".to_string()
            }
            Self::DownloadTruncated => {
                "The download was incomplete and was removed. Try again or open the releases page in your browser."
                    .to_string()
            }
            Self::ChecksumMismatch => {
                "The downloaded file didn't pass verification and was removed.".to_string()
            }
            Self::Cancelled => "The download was cancelled.".to_string(),
            Self::Io(_) => {
                "Couldn't save the update to your Downloads folder. Check available disk space and permissions, or open the releases page in your browser."
                    .to_string()
            }
        }
    }
}

impl std::fmt::Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.user_message())
    }
}

impl std::error::Error for UpdateError {}

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + serde::Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Ok(<Option<T> as serde::Deserialize<'de>>::deserialize(deserializer)?.unwrap_or_default())
}

/// Relevant subset of a GitHub Release object from `/repos/.../releases`.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
pub struct GitHubRelease {
    /// Git tag name (for example `"v0.4.3"`).
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub tag_name: String,
    /// Human-readable release title.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub name: String,
    /// Markdown release notes body.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub body: String,
    /// Browser URL for the GitHub release page.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub html_url: String,
    /// ISO-8601 publication timestamp, if provided.
    #[serde(default)]
    pub published_at: Option<String>,
    /// Whether the release is an unpublished draft.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub draft: bool,
    /// Whether GitHub marks the release as a pre-release.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub prerelease: bool,
    /// Attached downloadable files.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub assets: Vec<GitHubAsset>,
}

/// Relevant subset of a GitHub Release Asset object.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
pub struct GitHubAsset {
    /// Asset filename (for example `"CourseCraft-0.4.3-macos.dmg"`).
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub name: String,
    /// Direct download URL on GitHub.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub browser_download_url: String,
    /// Declared size of the asset in bytes.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub size: u64,
    /// Optional content digest from GitHub (for example `"sha256:<64-hex>"`).
    #[serde(default)]
    pub digest: Option<String>,
}

/// Parse a GitHub Releases API JSON array response while enforcing [`MAX_API_RESPONSE_BYTES`].
///
/// # Errors
/// Returns [`UpdateError::InvalidResponse`] if the payload is empty, exceeds
/// [`MAX_API_RESPONSE_BYTES`], or is not a valid JSON array of releases.
pub fn parse_releases_json(bytes: &[u8]) -> Result<Vec<GitHubRelease>, UpdateError> {
    if bytes.is_empty() || bytes.len() > MAX_API_RESPONSE_BYTES {
        return Err(UpdateError::InvalidResponse);
    }
    serde_json::from_slice(bytes).map_err(|_| UpdateError::InvalidResponse)
}

/// Parse a Git release tag as SemVer, stripping a single optional leading `'v'` or `'V'`.
#[must_use]
pub fn parse_tag_version(tag: &str) -> Option<semver::Version> {
    let trimmed = tag.trim();
    let stripped = trimmed
        .strip_prefix('v')
        .or_else(|| trimmed.strip_prefix('V'))
        .unwrap_or(trimmed);
    semver::Version::parse(stripped).ok()
}

/// Decide whether a parsed candidate release is eligible for the running version.
///
/// - Draft releases are never eligible.
/// - Clean `0.x` builds (`current.major == 0 && current.pre.is_empty()`) accept
///   clean SemVer releases (`candidate.pre.is_empty()`) even when GitHub marks
///   them `prerelease: true`, while skipping SemVer RCs/alphas/betas.
/// - Pre-release builds (`!current.pre.is_empty()`) accept newer pre-releases and
///   final releases alike.
/// - Stable `1.x+` builds (`current.major >= 1 && current.pre.is_empty()`) accept
///   only releases where `!release.prerelease && candidate.pre.is_empty()`.
#[must_use]
pub fn is_release_eligible(
    release: &GitHubRelease,
    candidate: &semver::Version,
    current: &semver::Version,
) -> bool {
    if release.draft {
        return false;
    }
    if !current.pre.is_empty() {
        return true;
    }
    if current.major == 0 {
        return candidate.pre.is_empty();
    }
    !release.prerelease && candidate.pre.is_empty()
}

/// Select the highest eligible release whose SemVer version is strictly greater than `current`.
#[must_use]
pub fn select_latest_update(
    releases: Vec<GitHubRelease>,
    current: &semver::Version,
) -> Option<(semver::Version, GitHubRelease)> {
    let mut best: Option<(semver::Version, GitHubRelease)> = None;
    for release in releases {
        let Some(version) = parse_tag_version(&release.tag_name) else {
            continue;
        };
        if !is_release_eligible(&release, &version, current) || version <= *current {
            continue;
        }
        let is_newer = best
            .as_ref()
            .is_none_or(|(best_version, _)| version > *best_version);
        if is_newer {
            best = Some((version, release));
        }
    }
    best
}

/// Find a `SHA256SUMS.txt` or `SHA256SUMS` checksum manifest asset (case-insensitive).
#[must_use]
pub fn find_checksums_asset(assets: &[GitHubAsset]) -> Option<&GitHubAsset> {
    assets.iter().find(|asset| {
        asset.name.eq_ignore_ascii_case("sha256sums.txt")
            || asset.name.eq_ignore_ascii_case("sha256sums")
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AssetArch {
    Arm64,
    X64,
    Universal,
    Unspecified,
}

fn is_non_installer_asset(asset: &GitHubAsset) -> bool {
    if asset.name.trim().is_empty() || asset.browser_download_url.trim().is_empty() {
        return true;
    }
    let lower = asset.name.to_ascii_lowercase();
    lower == "sha256sums"
        || lower == "sha256sums.txt"
        || lower.ends_with(".txt")
        || lower.ends_with(".sha256")
        || lower.ends_with(".sig")
        || lower.ends_with(".asc")
        || lower.ends_with(".json")
        || lower.ends_with(".md")
}

fn asset_arch_tag(lower_name: &str) -> AssetArch {
    let mut has_arm64 = false;
    let mut has_x64 = lower_name.contains("x86_64") || lower_name.contains("x86-64");
    let mut has_universal = false;

    for token in lower_name.split(['-', '_', '.', ' ']) {
        match token {
            "arm64" | "aarch64" => has_arm64 = true,
            "amd64" | "x64" | "intel" => has_x64 = true,
            "universal" => has_universal = true,
            _ => {}
        }
    }

    if has_universal || (has_arm64 && has_x64) {
        AssetArch::Universal
    } else if has_arm64 {
        AssetArch::Arm64
    } else if has_x64 {
        AssetArch::X64
    } else {
        AssetArch::Unspecified
    }
}

fn normalize_target_arch(arch: &str) -> Option<AssetArch> {
    match arch.trim().to_ascii_lowercase().as_str() {
        "aarch64" | "arm64" => Some(AssetArch::Arm64),
        "x86_64" | "amd64" | "x64" => Some(AssetArch::X64),
        _ => None,
    }
}

fn arch_compatibility_rank(asset_arch: AssetArch, target_arch: &str) -> Option<u8> {
    let target = normalize_target_arch(target_arch);
    match asset_arch {
        AssetArch::Arm64 | AssetArch::X64 => (target == Some(asset_arch)).then_some(0),
        AssetArch::Universal => target.map(|_| 1),
        AssetArch::Unspecified => Some(2),
    }
}

fn rank_macos_asset(lower_name: &str, arch_rank: u8) -> Option<(u8, u8)> {
    let format_rank = if lower_name.ends_with(".dmg") {
        0
    } else if lower_name.ends_with(".pkg") {
        1
    } else if lower_name.contains("macos")
        && (lower_name.ends_with(".zip") || lower_name.ends_with(".tar.gz"))
    {
        2
    } else {
        return None;
    };
    Some((format_rank, arch_rank))
}

fn rank_windows_asset(lower_name: &str, arch_rank: u8, prefer_portable: bool) -> Option<(u8, u8)> {
    let format_rank = if lower_name.ends_with(".exe") {
        if lower_name.contains("portable") {
            if prefer_portable { 0 } else { 2 }
        } else if lower_name.contains("setup") || lower_name.contains("installer") {
            if prefer_portable { 1 } else { 0 }
        } else {
            3
        }
    } else if lower_name.ends_with(".msi") {
        if prefer_portable { 2 } else { 1 }
    } else {
        return None;
    };
    Some((format_rank, arch_rank))
}

fn rank_linux_asset(lower_name: &str, arch_rank: u8) -> Option<(u8, u8)> {
    let is_appimage = lower_name.ends_with(".appimage");
    if !is_appimage && !lower_name.contains("linux") {
        return None;
    }
    let format_rank = if lower_name.ends_with(".tar.gz") {
        0
    } else if is_appimage {
        1
    } else if lower_name.ends_with(".deb") {
        2
    } else if lower_name.ends_with(".rpm") {
        3
    } else {
        return None;
    };
    Some((format_rank, arch_rank))
}

/// Pick the best release installer asset for the given OS, CPU architecture, and
/// Windows portable preference.
#[must_use]
pub fn pick_asset<'a>(
    assets: &'a [GitHubAsset],
    os: &str,
    arch: &str,
    prefer_portable: bool,
) -> Option<&'a GitHubAsset> {
    let os_lower = os.trim().to_ascii_lowercase();
    if !matches!(os_lower.as_str(), "macos" | "windows" | "linux") {
        return None;
    }

    let mut best: Option<((u8, u8), &'a GitHubAsset)> = None;
    for asset in assets {
        if is_non_installer_asset(asset) {
            continue;
        }
        let lower_name = asset.name.to_ascii_lowercase();
        let Some(arch_rank) = arch_compatibility_rank(asset_arch_tag(&lower_name), arch) else {
            continue;
        };
        let rank = match os_lower.as_str() {
            "macos" => rank_macos_asset(&lower_name, arch_rank),
            "windows" => rank_windows_asset(&lower_name, arch_rank, prefer_portable),
            "linux" => rank_linux_asset(&lower_name, arch_rank),
            _ => None,
        };
        let Some(rank) = rank else {
            continue;
        };
        if best.as_ref().is_none_or(|(best_rank, _)| rank < *best_rank) {
            best = Some((rank, asset));
        }
    }
    best.map(|(_, asset)| asset)
}

fn is_valid_sha256_hex(hex: &str) -> bool {
    hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Parse a GitHub asset digest of the form `"sha256:<64-hex>"` into lowercase 64-char hex.
#[must_use]
pub fn parse_sha256_digest(digest: &str) -> Option<String> {
    let (prefix, hex) = digest.trim().split_once(':')?;
    if !prefix.eq_ignore_ascii_case("sha256") || !is_valid_sha256_hex(hex) {
        return None;
    }
    Some(hex.to_ascii_lowercase())
}

/// Extract the lowercase 64-char SHA-256 hex digest for `filename` from a
/// `SHA256SUMS.txt` manifest.
#[must_use]
pub fn parse_sha256sums_line(text: &str, filename: &str) -> Option<String> {
    if filename.is_empty() {
        return None;
    }
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some(hash) = line.get(..64) else {
            continue;
        };
        if !is_valid_sha256_hex(hash) {
            continue;
        }
        let Some(rest) = line.get(64..) else {
            continue;
        };
        if !rest.starts_with([' ', '\t']) {
            continue;
        }
        let after_ws = rest.trim_start_matches([' ', '\t']);
        let after_mode = after_ws.strip_prefix('*').unwrap_or(after_ws);
        let entry_name = after_mode.strip_prefix("./").unwrap_or(after_mode);
        if entry_name == filename {
            return Some(hash.to_ascii_lowercase());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        CHECK_INTERVAL_SECS, GitHubAsset, GitHubRelease, MAX_API_RESPONSE_BYTES, UpdateError,
        auto_check_suppressed_by_env, current_version, find_checksums_asset, is_release_eligible,
        parse_releases_json, parse_sha256_digest, parse_sha256sums_line, parse_tag_version,
        pick_asset, select_latest_update, should_auto_check, user_agent,
    };

    const V043_RELEASES_JSON: &str = r#"[
      {
        "id": 214567890,
        "tag_name": "v0.4.3",
        "name": "CourseCraft 0.4.3",
        "body": "**Faster, lighter editor**\n\n- Improved timeline scrolling\n- Fixed SCORM export edge cases\n",
        "html_url": "https://github.com/ad1-kamath/coursecraft/releases/tag/v0.4.3",
        "published_at": "2026-10-12T18:30:00Z",
        "draft": false,
        "prerelease": true,
        "assets": [
          {
            "id": 1001,
            "name": "CourseCraft-0.4.3-macos.dmg",
            "browser_download_url": "https://github.com/ad1-kamath/coursecraft/releases/download/v0.4.3/CourseCraft-0.4.3-macos.dmg",
            "size": 28677774,
            "digest": "sha256:f6caeb20ad874ade46c8bf992cb70e777d30ae273877667de6671b4acae051e3"
          },
          {
            "id": 1002,
            "name": "CourseCraft-v0.4.3-setup.exe",
            "browser_download_url": "https://github.com/ad1-kamath/coursecraft/releases/download/v0.4.3/CourseCraft-v0.4.3-setup.exe",
            "size": 20809277,
            "digest": "sha256:1111111111111111111111111111111111111111111111111111111111111111"
          },
          {
            "id": 1003,
            "name": "CourseCraft-v0.4.3-portable.exe",
            "browser_download_url": "https://github.com/ad1-kamath/coursecraft/releases/download/v0.4.3/CourseCraft-v0.4.3-portable.exe",
            "size": 65619456,
            "digest": "sha256:2222222222222222222222222222222222222222222222222222222222222222"
          },
          {
            "id": 1004,
            "name": "CourseCraft-v0.4.3-linux-x86_64.tar.gz",
            "browser_download_url": "https://github.com/ad1-kamath/coursecraft/releases/download/v0.4.3/CourseCraft-v0.4.3-linux-x86_64.tar.gz",
            "size": 29924711,
            "digest": "sha256:3333333333333333333333333333333333333333333333333333333333333333"
          }
        ]
      }
    ]"#;

    fn make_release(tag: &str, draft: bool, prerelease: bool) -> GitHubRelease {
        GitHubRelease {
            tag_name: tag.to_string(),
            name: format!("CourseCraft {tag}"),
            html_url: format!("https://github.com/ad1-kamath/coursecraft/releases/tag/{tag}"),
            draft,
            prerelease,
            ..GitHubRelease::default()
        }
    }

    fn make_asset(name: &str) -> GitHubAsset {
        GitHubAsset {
            name: name.to_string(),
            browser_download_url: format!(
                "https://github.com/ad1-kamath/coursecraft/releases/download/v0.4.4/{name}"
            ),
            size: 1024,
            digest: None,
        }
    }

    #[test]
    fn up01_parse_v043_fixture() {
        let releases = parse_releases_json(V043_RELEASES_JSON.as_bytes()).expect("valid fixture");
        assert_eq!(releases.len(), 1);
        let rel = &releases[0];
        assert_eq!(rel.tag_name, "v0.4.3");
        assert_eq!(rel.name, "CourseCraft 0.4.3");
        assert!(rel.body.contains("Faster, lighter editor"));
        assert_eq!(
            rel.html_url,
            "https://github.com/ad1-kamath/coursecraft/releases/tag/v0.4.3"
        );
        assert_eq!(rel.published_at.as_deref(), Some("2026-10-12T18:30:00Z"));
        assert!(!rel.draft);
        assert!(rel.prerelease);
        assert_eq!(rel.assets.len(), 4);

        assert_eq!(rel.assets[0].name, "CourseCraft-0.4.3-macos.dmg");
        assert_eq!(rel.assets[0].size, 28_677_774);
        assert_eq!(
            rel.assets[0].digest.as_deref(),
            Some("sha256:f6caeb20ad874ade46c8bf992cb70e777d30ae273877667de6671b4acae051e3")
        );
        assert_eq!(rel.assets[1].name, "CourseCraft-v0.4.3-setup.exe");
        assert_eq!(rel.assets[1].size, 20_809_277);
        assert_eq!(rel.assets[2].name, "CourseCraft-v0.4.3-portable.exe");
        assert_eq!(rel.assets[2].size, 65_619_456);
        assert_eq!(rel.assets[3].name, "CourseCraft-v0.4.3-linux-x86_64.tar.gz");
        assert_eq!(rel.assets[3].size, 29_924_711);
    }

    #[test]
    fn up02_semver_and_prerelease_policy() {
        let v043 = semver::Version::parse("0.4.3").expect("valid semver");
        let v050_rc1 = semver::Version::parse("0.5.0-rc.1").expect("valid semver");
        let v100 = semver::Version::parse("1.0.0").expect("valid semver");

        // Tag parsing strips one optional leading v/V and trims whitespace.
        assert_eq!(
            parse_tag_version("v0.4.4"),
            Some(semver::Version::new(0, 4, 4))
        );
        assert_eq!(
            parse_tag_version(" V0.4.10 "),
            Some(semver::Version::new(0, 4, 10))
        );
        assert_eq!(parse_tag_version("0.4.3"), Some(v043.clone()));
        assert_eq!(parse_tag_version("vv0.4.3"), None);
        assert_eq!(parse_tag_version("not-a-version"), None);

        // 0.4.3 -> v0.4.4 (GitHub prerelease=true) = update.
        let picked = select_latest_update(vec![make_release("v0.4.4", false, true)], &v043);
        assert_eq!(picked.map(|(v, _)| v), Some(semver::Version::new(0, 4, 4)));

        // 0.4.3 -> v0.4.10 > v0.4.9 (SemVer, not lexicographic).
        let picked = select_latest_update(
            vec![
                make_release("v0.4.9", false, true),
                make_release("v0.4.10", false, true),
            ],
            &v043,
        );
        assert_eq!(picked.map(|(v, _)| v), Some(semver::Version::new(0, 4, 10)));

        // 0.4.3 -> v0.5.0-rc.1 = ignored.
        let picked = select_latest_update(vec![make_release("v0.5.0-rc.1", false, true)], &v043);
        assert!(picked.is_none());

        // 0.5.0-rc.1 -> v0.5.0-rc.2 and v0.5.0 = update.
        let picked_rc2 =
            select_latest_update(vec![make_release("v0.5.0-rc.2", false, true)], &v050_rc1);
        assert_eq!(
            picked_rc2.map(|(v, _)| v.to_string()),
            Some("0.5.0-rc.2".to_string())
        );
        let picked_final = select_latest_update(
            vec![
                make_release("v0.5.0-rc.2", false, true),
                make_release("v0.5.0", false, false),
            ],
            &v050_rc1,
        );
        assert_eq!(
            picked_final.map(|(v, _)| v),
            Some(semver::Version::new(0, 5, 0))
        );

        // 1.0.0 -> v1.1.0-beta.1 or GitHub prerelease=true v1.1.0 = ignored; stable v1.0.1 accepted.
        assert!(
            select_latest_update(
                vec![
                    make_release("v1.1.0-beta.1", false, true),
                    make_release("v1.1.0", false, true),
                ],
                &v100,
            )
            .is_none()
        );
        let picked_stable = select_latest_update(vec![make_release("v1.0.1", false, false)], &v100);
        assert_eq!(
            picked_stable.map(|(v, _)| v),
            Some(semver::Version::new(1, 0, 1))
        );

        // Drafts are always ignored even for pre-release users.
        let draft = make_release("v9.9.9", true, false);
        assert!(!is_release_eligible(
            &draft,
            &semver::Version::new(9, 9, 9),
            &v050_rc1
        ));
        assert!(select_latest_update(vec![draft], &v043).is_none());

        // Out-of-order releases pick maximum SemVer.
        let out_of_order = select_latest_update(
            vec![
                make_release("v0.3.9", false, true),
                make_release("v0.4.6", false, true),
                make_release("v0.4.4", false, true),
                make_release("v0.4.5", false, false),
            ],
            &v043,
        );
        assert_eq!(
            out_of_order.map(|(v, _)| v),
            Some(semver::Version::new(0, 4, 6))
        );

        // Same or older version returns None.
        assert!(
            select_latest_update(
                vec![
                    make_release("v0.4.3", false, true),
                    make_release("v0.4.2", false, true),
                ],
                &v043,
            )
            .is_none()
        );
    }

    #[test]
    fn up03_hostile_and_oversized_json() {
        // Empty payload.
        assert_eq!(parse_releases_json(b""), Err(UpdateError::InvalidResponse));

        // Invalid JSON syntax.
        assert_eq!(
            parse_releases_json(b"[{not json"),
            Err(UpdateError::InvalidResponse)
        );

        // JSON object instead of array (for example GitHub error body).
        assert_eq!(
            parse_releases_json(br#"{"message":"Not Found"}"#),
            Err(UpdateError::InvalidResponse)
        );

        // Oversized payload (MAX_API_RESPONSE_BYTES + 1).
        let mut oversized = vec![b' '; MAX_API_RESPONSE_BYTES + 1];
        oversized[0] = b'[';
        oversized[MAX_API_RESPONSE_BYTES] = b']';
        assert_eq!(
            parse_releases_json(&oversized),
            Err(UpdateError::InvalidResponse)
        );

        // Missing fields, null nullable fields, and invalid tag strings do not panic.
        let sparse = br#"[
          {},
          {"tag_name": null, "name": null, "body": null, "assets": null},
          {"tag_name": "nightly-2026-10-12"},
          {"tag_name": "v0.4.5"}
        ]"#;
        let parsed = parse_releases_json(sparse).expect("sparse array parses");
        assert_eq!(parsed.len(), 4);
        let picked = select_latest_update(parsed, &semver::Version::new(0, 4, 3));
        assert_eq!(picked.map(|(v, _)| v), Some(semver::Version::new(0, 4, 5)));

        // Verify plain-language user messages never leak raw inner payloads.
        let net_err = UpdateError::Network("raw dns stacktrace 127.0.0.1".to_string());
        assert!(!net_err.user_message().contains("stacktrace"));
        assert_eq!(net_err.to_string(), net_err.user_message());
        let io_err = UpdateError::Io("/secret/path errno 28".to_string());
        assert!(!io_err.user_message().contains("/secret/path"));
        assert!(UpdateError::HttpStatus(503).user_message().contains("503"));
        assert!(!UpdateError::Timeout.user_message().is_empty());
        assert!(!UpdateError::RateLimited.user_message().is_empty());
        assert!(!UpdateError::DisallowedUrl.user_message().is_empty());
        assert!(!UpdateError::TooManyRedirects.user_message().is_empty());
        assert!(!UpdateError::DownloadTooLarge.user_message().is_empty());
        assert!(!UpdateError::DownloadTruncated.user_message().is_empty());
        assert!(!UpdateError::ChecksumMismatch.user_message().is_empty());
        assert!(!UpdateError::Cancelled.user_message().is_empty());
    }

    #[test]
    fn up04_pick_asset_platforms_and_arches() {
        let mut releases = parse_releases_json(V043_RELEASES_JSON.as_bytes()).expect("valid json");
        let mut assets = releases.remove(0).assets;
        assets.push(make_asset("SHA256SUMS.txt"));
        assets.push(make_asset("CourseCraft-0.4.3-macos.dmg.sig"));
        assets.push(make_asset("CourseCraft-0.4.3-macos.dmg.sha256"));
        assets.push(GitHubAsset {
            name: "CourseCraft-0.4.3-macos-arm64.dmg".to_string(),
            browser_download_url: String::new(),
            size: 100,
            digest: None,
        });

        assert_eq!(
            find_checksums_asset(&assets).map(|a| a.name.as_str()),
            Some("SHA256SUMS.txt")
        );

        // Real v0.4.3 assets across all platforms.
        assert_eq!(
            pick_asset(&assets, "macos", "aarch64", false).map(|a| a.name.as_str()),
            Some("CourseCraft-0.4.3-macos.dmg")
        );
        assert_eq!(
            pick_asset(&assets, "windows", "x86_64", false).map(|a| a.name.as_str()),
            Some("CourseCraft-v0.4.3-setup.exe")
        );
        assert_eq!(
            pick_asset(&assets, "windows", "x86_64", true).map(|a| a.name.as_str()),
            Some("CourseCraft-v0.4.3-portable.exe")
        );
        assert_eq!(
            pick_asset(&assets, "linux", "x86_64", false).map(|a| a.name.as_str()),
            Some("CourseCraft-v0.4.3-linux-x86_64.tar.gz")
        );
        assert_eq!(pick_asset(&assets, "linux", "aarch64", false), None);
        assert_eq!(pick_asset(&assets, "freebsd", "x86_64", false), None);

        // Explicit arch beats universal which beats unqualified; wrong arch is rejected.
        let mac_assets = vec![
            make_asset("CourseCraft-0.4.4-macos.dmg"),
            make_asset("CourseCraft-0.4.4-macos-universal.dmg"),
            make_asset("CourseCraft-0.4.4-macos-x86_64.dmg"),
            make_asset("CourseCraft-0.4.4-macos-arm64.dmg"),
        ];
        assert_eq!(
            pick_asset(&mac_assets, "macos", "aarch64", false).map(|a| a.name.as_str()),
            Some("CourseCraft-0.4.4-macos-arm64.dmg")
        );
        assert_eq!(
            pick_asset(&mac_assets, "macos", "x86_64", false).map(|a| a.name.as_str()),
            Some("CourseCraft-0.4.4-macos-x86_64.dmg")
        );

        let mac_universal_vs_plain = vec![
            make_asset("CourseCraft-0.4.4-macos.dmg"),
            make_asset("CourseCraft-0.4.4-macos-universal.dmg"),
        ];
        assert_eq!(
            pick_asset(&mac_universal_vs_plain, "macos", "aarch64", false).map(|a| a.name.as_str()),
            Some("CourseCraft-0.4.4-macos-universal.dmg")
        );

        let mac_intel_only = vec![make_asset("CourseCraft-0.4.4-macos-intel.dmg")];
        assert_eq!(pick_asset(&mac_intel_only, "macos", "aarch64", false), None);
        assert_eq!(
            pick_asset(&mac_intel_only, "macos", "x86_64", false).map(|a| a.name.as_str()),
            Some("CourseCraft-0.4.4-macos-intel.dmg")
        );

        // Windows MSI ranking between setup.exe and portable.exe.
        let win_msi_assets = vec![
            make_asset("CourseCraft-v0.4.4-portable.exe"),
            make_asset("CourseCraft-v0.4.4.msi"),
        ];
        assert_eq!(
            pick_asset(&win_msi_assets, "windows", "x86_64", false).map(|a| a.name.as_str()),
            Some("CourseCraft-v0.4.4.msi")
        );
        assert_eq!(
            pick_asset(&win_msi_assets, "windows", "x86_64", true).map(|a| a.name.as_str()),
            Some("CourseCraft-v0.4.4-portable.exe")
        );
    }

    #[test]
    fn up08_sha256_digest_and_sums_parsing() {
        let lower_hex = "f6caeb20ad874ade46c8bf992cb70e777d30ae273877667de6671b4acae051e3";
        let upper_hex = "F6CAEB20AD874ADE46C8BF992CB70E777D30AE273877667DE6671B4ACAE051E3";

        // Valid sha256: digests (case-insensitive prefix and hex).
        assert_eq!(
            parse_sha256_digest(&format!("sha256:{lower_hex}")),
            Some(lower_hex.to_string())
        );
        assert_eq!(
            parse_sha256_digest(&format!("  SHA256:{upper_hex}  ")),
            Some(lower_hex.to_string())
        );

        // Invalid digests.
        assert_eq!(parse_sha256_digest(""), None);
        assert_eq!(parse_sha256_digest(&format!("sha512:{lower_hex}")), None);
        assert_eq!(parse_sha256_digest("sha256:abcd"), None);
        assert_eq!(parse_sha256_digest(&format!("sha256:{lower_hex}a")), None);
        assert_eq!(
            parse_sha256_digest(
                "sha256:zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"
            ),
            None
        );

        // SHA256SUMS.txt parsing: standard two-space, binary `*`, leading `./`, comments, bad lines.
        let sums = format!(
            "# CourseCraft release checksums\n\
             \n\
             not-a-valid-hash  CourseCraft-0.4.3-macos.dmg\n\
             {lower_hex}a  CourseCraft-0.4.3-macos.dmg\n\
             {upper_hex}  CourseCraft-0.4.3-macos.dmg\n\
             1111111111111111111111111111111111111111111111111111111111111111 *CourseCraft-v0.4.3-setup.exe\n\
             2222222222222222222222222222222222222222222222222222222222222222  ./CourseCraft-v0.4.3-linux-x86_64.tar.gz\n\
             3333333333333333333333333333333333333333333333333333333333333333 *./CourseCraft-v0.4.3-portable.exe\n"
        );

        assert_eq!(
            parse_sha256sums_line(&sums, "CourseCraft-0.4.3-macos.dmg"),
            Some(lower_hex.to_string())
        );
        assert_eq!(
            parse_sha256sums_line(&sums, "CourseCraft-v0.4.3-setup.exe"),
            Some("1111111111111111111111111111111111111111111111111111111111111111".to_string())
        );
        assert_eq!(
            parse_sha256sums_line(&sums, "CourseCraft-v0.4.3-linux-x86_64.tar.gz"),
            Some("2222222222222222222222222222222222222222222222222222222222222222".to_string())
        );
        assert_eq!(
            parse_sha256sums_line(&sums, "CourseCraft-v0.4.3-portable.exe"),
            Some("3333333333333333333333333333333333333333333333333333333333333333".to_string())
        );
        assert_eq!(parse_sha256sums_line(&sums, "missing.dmg"), None);
        assert_eq!(parse_sha256sums_line(&sums, ""), None);
    }

    #[test]
    fn up05_should_auto_check_daily_throttle() {
        let base: u64 = 1_700_000_000;

        // Disabled always returns false.
        assert!(!should_auto_check(false, None, base));
        assert!(!should_auto_check(
            false,
            Some(base),
            base + CHECK_INTERVAL_SECS
        ));

        // First run (no recorded check yet) checks immediately.
        assert!(should_auto_check(true, None, base));

        // 1 second shy of 24 hours stays throttled; exactly 24 hours allows a check.
        assert!(!should_auto_check(
            true,
            Some(base),
            base + CHECK_INTERVAL_SECS - 1
        ));
        assert!(should_auto_check(
            true,
            Some(base),
            base + CHECK_INTERVAL_SECS
        ));
        assert!(should_auto_check(
            true,
            Some(base),
            base + CHECK_INTERVAL_SECS + 60
        ));

        // System clock moving backwards never triggers an unexpected check.
        assert!(!should_auto_check(true, Some(base), base - 1));
    }

    #[test]
    fn up05_env_suppression_user_agent_and_current_version() {
        assert!(!auto_check_suppressed_by_env(None, None));
        assert!(!auto_check_suppressed_by_env(Some(""), Some("")));
        assert!(!auto_check_suppressed_by_env(Some("0"), Some("0")));
        assert!(!auto_check_suppressed_by_env(Some("false"), Some("FALSE")));
        assert!(!auto_check_suppressed_by_env(Some("False"), Some("  ")));

        assert!(auto_check_suppressed_by_env(Some("true"), None));
        assert!(auto_check_suppressed_by_env(Some("1"), None));
        assert!(auto_check_suppressed_by_env(None, Some("1")));
        assert!(auto_check_suppressed_by_env(None, Some("true")));
        assert!(auto_check_suppressed_by_env(Some("0"), Some("yes")));

        assert_eq!(user_agent("0.4.3"), "CourseCraft/0.4.3");
        assert!(current_version().is_some());
    }
}
