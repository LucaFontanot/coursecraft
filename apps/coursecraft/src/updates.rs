//! Desktop-only update check policy, version helpers, and download limits.
//!
//! All networking, version parsing, and Downloads I/O for CourseCraft updates
//! live in this module behind `#[cfg(not(target_arch = "wasm32"))]`.

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

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

const FALLBACK_RELEASES_URL: &str = "https://github.com/ad1-kamath/coursecraft/releases";
const MAX_ASSET_FILENAME_CHARS: usize = 200;
const MAX_UNIQUE_DOWNLOAD_ATTEMPTS: u32 = 1000;
const DOWNLOAD_CHUNK_BYTES: usize = 64 * 1024;

fn is_valid_subdomain_prefix(sub: &str) -> bool {
    if sub.is_empty() {
        return false;
    }
    sub.split('.').all(|label| {
        !label.is_empty()
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    })
}

fn extract_https_host(url: &str) -> Option<&str> {
    if url.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return None;
    }
    let rest = url.strip_prefix("https://")?;
    let authority = rest.split(['/', '?', '#']).next()?;
    if authority.is_empty() || authority.contains('@') {
        return None;
    }
    match authority.split_once(':') {
        Some((host, "443")) => Some(host),
        Some(_) => None,
        None => Some(authority),
    }
}

/// Validate that an update or redirect URL uses `https://` and targets an
/// allowlisted GitHub host (`api.github.com`, `github.com`, or
/// `*.githubusercontent.com`).
#[must_use]
pub fn is_allowed_update_url(url: &str) -> bool {
    let Some(host) = extract_https_host(url) else {
        return false;
    };
    let lower = host.to_ascii_lowercase();
    if lower == "api.github.com" || lower == "github.com" {
        return true;
    }
    lower
        .strip_suffix(".githubusercontent.com")
        .is_some_and(is_valid_subdomain_prefix)
}

/// Resolve an HTTP `Location` header relative to `base`, rejecting empty values,
/// whitespace/control characters, and non-HTTPS schemes.
#[must_use]
pub fn resolve_redirect_url(base: &str, location: &str) -> Option<String> {
    if location.is_empty()
        || location
            .chars()
            .any(|c| c.is_control() || c.is_whitespace())
    {
        return None;
    }
    if location.starts_with("https://") {
        return Some(location.to_owned());
    }
    if let Some(rest) = location.strip_prefix("//") {
        if rest.is_empty() || rest.starts_with('/') {
            return None;
        }
        return Some(format!("https:{location}"));
    }
    if location.starts_with('/') {
        if base.chars().any(|c| c.is_control() || c.is_whitespace()) {
            return None;
        }
        let rest = base.strip_prefix("https://")?;
        let authority = rest.split(['/', '?', '#']).next()?;
        if authority.is_empty() {
            return None;
        }
        return Some(format!("https://{authority}{location}"));
    }
    None
}

/// Validate a release asset filename before joining it onto a local directory.
#[must_use]
pub fn safe_asset_filename(name: &str) -> Option<String> {
    if name.is_empty()
        || name.trim().is_empty()
        || name.chars().count() > MAX_ASSET_FILENAME_CHARS
        || name.starts_with('.')
        || name.contains("..")
    {
        return None;
    }
    if name
        .chars()
        .any(|c| matches!(c, '/' | '\\' | ':' | '\0') || c.is_control())
    {
        return None;
    }
    Some(name.to_owned())
}

fn path_is_occupied(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok() || path.exists()
}

fn split_filename_stem_ext(filename: &str) -> (&str, &str) {
    let lower = filename.to_ascii_lowercase();
    for compound in [".tar.gz", ".tar.xz", ".tar.bz2", ".tar.zst"] {
        if lower.ends_with(compound) && filename.len() > compound.len() {
            let split_at = filename.len() - compound.len();
            if let (Some(stem), Some(ext)) = (filename.get(..split_at), filename.get(split_at..)) {
                return (stem, ext);
            }
        }
    }
    if let Some(dot) = filename.rfind('.')
        && dot > 0
        && let (Some(stem), Some(ext)) = (filename.get(..dot), filename.get(dot..))
    {
        return (stem, ext);
    }
    (filename, "")
}

/// Pick a collision-free destination path under `dir` without overwriting any
/// existing file. Preserves compound extensions such as `.tar.gz`.
#[must_use]
pub fn unique_download_path(dir: &Path, filename: &str) -> PathBuf {
    let initial = dir.join(filename);
    if !path_is_occupied(&initial) {
        return initial;
    }
    let (stem, ext) = split_filename_stem_ext(filename);
    for n in 1..=MAX_UNIQUE_DOWNLOAD_ATTEMPTS {
        let candidate = dir.join(format!("{stem} ({n}){ext}"));
        if !path_is_occupied(&candidate) {
            return candidate;
        }
    }
    dir.join(format!(
        "{stem} ({}){ext}",
        MAX_UNIQUE_DOWNLOAD_ATTEMPTS + 1
    ))
}

fn part_path_for(dest: &Path) -> PathBuf {
    let mut os = dest.as_os_str().to_os_string();
    os.push(".part");
    PathBuf::from(os)
}

/// Kind of HTTP request made by the update subsystem (selects timeouts/headers).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestKind {
    /// GitHub Releases metadata JSON query.
    Metadata,
    /// `SHA256SUMS.txt` checksum manifest fetch.
    Checksums,
    /// Installer package binary stream.
    Download,
}

/// Minimal HTTP response surface returned by [`UpdateHttp`].
pub struct HttpResponse {
    /// Numeric HTTP status code (e.g. `200`, `302`, `429`).
    pub status: u16,
    /// Value of the `Location` header, if present.
    pub location: Option<String>,
    /// Parsed `Content-Length` header value, if present.
    pub content_length: Option<u64>,
    /// Parsed `X-RateLimit-Remaining` header value, if present.
    pub rate_limit_remaining: Option<u64>,
    /// Streaming response body reader.
    pub body: Box<dyn Read + Send>,
}

/// Testable HTTP transport seam for update checks and downloads.
pub trait UpdateHttp: Send + Sync {
    /// Perform a single non-redirecting HTTPS `GET` request.
    ///
    /// # Errors
    /// Returns [`UpdateError`] on transport failures or timeouts.
    fn get(
        &self,
        url: &str,
        accept: &str,
        user_agent: &str,
        kind: RequestKind,
    ) -> Result<HttpResponse, UpdateError>;
}

/// Execute an HTTPS `GET` while manually following up to [`MAX_REDIRECTS`]
/// redirects and validating every hop with [`is_allowed_update_url`].
///
/// # Errors
/// Returns [`UpdateError::DisallowedUrl`] if any hop or `Location` header is
/// disallowed, [`UpdateError::TooManyRedirects`] if redirects exceed
/// [`MAX_REDIRECTS`], or any transport error from `http`.
pub fn follow_get_redirects(
    http: &dyn UpdateHttp,
    url: &str,
    accept: &str,
    user_agent: &str,
    kind: RequestKind,
) -> Result<HttpResponse, UpdateError> {
    let mut current_url = url.to_owned();
    for hop in 0..=MAX_REDIRECTS {
        if !is_allowed_update_url(&current_url) {
            return Err(UpdateError::DisallowedUrl);
        }
        let resp = http.get(&current_url, accept, user_agent, kind)?;
        if !matches!(resp.status, 301 | 302 | 303 | 307 | 308) {
            return Ok(resp);
        }
        if hop == MAX_REDIRECTS {
            return Err(UpdateError::TooManyRedirects);
        }
        let Some(location) = resp.location.as_deref() else {
            return Err(UpdateError::DisallowedUrl);
        };
        let Some(next_url) = resolve_redirect_url(&current_url, location) else {
            return Err(UpdateError::DisallowedUrl);
        };
        current_url = next_url;
    }
    Err(UpdateError::TooManyRedirects)
}

fn check_http_status(status: u16, rate_limit_remaining: Option<u64>) -> Result<(), UpdateError> {
    if (200..300).contains(&status) {
        return Ok(());
    }
    if status == 429 || (status == 403 && rate_limit_remaining == Some(0)) {
        return Err(UpdateError::RateLimited);
    }
    Err(UpdateError::HttpStatus(status))
}

fn read_capped_bytes(reader: &mut dyn Read, max_bytes: usize) -> Result<Vec<u8>, UpdateError> {
    let limit = (max_bytes as u64).saturating_add(1);
    let mut buf = Vec::new();
    let mut limited = reader.take(limit);
    limited.read_to_end(&mut buf).map_err(|err| {
        if err.kind() == io::ErrorKind::TimedOut {
            UpdateError::Timeout
        } else {
            UpdateError::Network(err.to_string())
        }
    })?;
    if buf.len() > max_bytes {
        return Err(UpdateError::InvalidResponse);
    }
    Ok(buf)
}

/// Matched installer asset resolved from a GitHub release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAsset {
    /// Release asset filename.
    pub name: String,
    /// Direct download URL for the asset.
    pub download_url: String,
    /// Declared byte size from GitHub.
    pub size: u64,
    /// Normalized lowercase 64-char SHA-256 hex digest from `asset.digest`, if valid.
    pub sha256_digest: Option<String>,
}

/// Resolved update metadata ready for display or download.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedUpdate {
    /// New SemVer version string (without leading `v`).
    pub version: String,
    /// Currently running SemVer version string.
    pub current_version: String,
    /// Validated HTTPS release page URL on GitHub.
    pub release_url: String,
    /// Raw Markdown release notes body.
    pub notes_markdown: String,
    /// ISO-8601 publication timestamp, if provided.
    pub published_at: Option<String>,
    /// Platform-matched installer asset, if one exists for the target OS/CPU.
    pub asset: Option<ResolvedAsset>,
    /// Download URL for `SHA256SUMS.txt` if present on the release.
    pub checksums_url: Option<String>,
}

/// Outcome of checking GitHub Releases for a newer CourseCraft version.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)]
pub enum CheckOutcome {
    /// Running version is already at or ahead of the latest eligible release.
    UpToDate,
    /// A newer eligible release is available.
    Available(ResolvedUpdate),
}

/// Query GitHub Releases via `http` and resolve whether an update is available
/// for the given current version and target platform.
///
/// # Errors
/// Returns [`UpdateError`] on disallowed URLs, rate limits, HTTP/network errors,
/// or malformed/oversized responses.
pub fn check_for_update_with(
    http: &dyn UpdateHttp,
    api_url: &str,
    current: &semver::Version,
    os: &str,
    arch: &str,
    prefer_portable: bool,
) -> Result<CheckOutcome, UpdateError> {
    let current_str = current.to_string();
    let ua = user_agent(&current_str);
    let mut resp = follow_get_redirects(
        http,
        api_url,
        "application/vnd.github+json",
        &ua,
        RequestKind::Metadata,
    )?;
    check_http_status(resp.status, resp.rate_limit_remaining)?;
    let bytes = read_capped_bytes(&mut *resp.body, MAX_API_RESPONSE_BYTES)?;
    let releases = parse_releases_json(&bytes)?;

    let Some((version, release)) = select_latest_update(releases, current) else {
        return Ok(CheckOutcome::UpToDate);
    };

    let asset =
        pick_asset(&release.assets, os, arch, prefer_portable).map(|picked| ResolvedAsset {
            name: picked.name.clone(),
            download_url: picked.browser_download_url.clone(),
            size: picked.size,
            sha256_digest: picked.digest.as_deref().and_then(parse_sha256_digest),
        });
    let checksums_url = find_checksums_asset(&release.assets)
        .map(|a| a.browser_download_url.clone())
        .filter(|u| !u.trim().is_empty());
    let release_url = if is_allowed_update_url(&release.html_url) {
        release.html_url
    } else {
        FALLBACK_RELEASES_URL.to_string()
    };

    Ok(CheckOutcome::Available(ResolvedUpdate {
        version: version.to_string(),
        current_version: current_str,
        release_url,
        notes_markdown: release.body,
        published_at: release.published_at,
        asset,
        checksums_url,
    }))
}

/// Parameters for a streaming, verified installer download.
pub struct DownloadSpec<'a> {
    /// Asset download URL (must satisfy [`is_allowed_update_url`]).
    pub url: &'a str,
    /// Declared byte size from GitHub, if known.
    pub expected_size: Option<u64>,
    /// Expected SHA-256 digest (either 64-char hex or `"sha256:<64-hex>"`), if known.
    pub expected_sha256: Option<&'a str>,
    /// Optional fallback URL to a `SHA256SUMS.txt` manifest.
    pub checksums_url: Option<&'a str>,
    /// Local destination directory (typically the user's Downloads folder).
    pub dest_dir: &'a Path,
    /// Asset filename (validated via [`safe_asset_filename`]).
    pub filename: &'a str,
    /// Privacy-preserving `User-Agent` header value.
    pub user_agent: &'a str,
}

/// Result of a completed and verified installer download.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadResult {
    /// Final saved file path in `dest_dir`.
    pub path: PathBuf,
    /// Exact number of bytes written.
    pub bytes: u64,
    /// Whether the file was verified against a SHA-256 digest.
    pub sha256_verified: bool,
}

fn encode_hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(char::from(HEX[usize::from(b >> 4)]));
        out.push(char::from(HEX[usize::from(b & 0x0f)]));
    }
    out
}

fn normalize_expected_sha256(raw: &str) -> Option<String> {
    if let Some(parsed) = parse_sha256_digest(raw) {
        return Some(parsed);
    }
    let trimmed = raw.trim();
    if is_valid_sha256_hex(trimmed) {
        Some(trimmed.to_ascii_lowercase())
    } else {
        None
    }
}

fn resolve_expected_sha256(
    http: &dyn UpdateHttp,
    spec: &DownloadSpec<'_>,
    safe_name: &str,
) -> Result<Option<String>, UpdateError> {
    if let Some(expected) = spec.expected_sha256 {
        let normalized = normalize_expected_sha256(expected).ok_or(UpdateError::InvalidResponse)?;
        return Ok(Some(normalized));
    }
    let Some(sums_url) = spec.checksums_url else {
        return Ok(None);
    };
    let mut resp = follow_get_redirects(
        http,
        sums_url,
        "text/plain, */*",
        spec.user_agent,
        RequestKind::Checksums,
    )?;
    check_http_status(resp.status, resp.rate_limit_remaining)?;
    let bytes = read_capped_bytes(&mut *resp.body, MAX_SUMS_BYTES)?;
    let text = std::str::from_utf8(&bytes).map_err(|_| UpdateError::InvalidResponse)?;
    Ok(parse_sha256sums_line(text, safe_name))
}

struct StreamTarget<'a> {
    part_path: &'a Path,
    expected_size: Option<u64>,
    content_length: Option<u64>,
    expected_sha256: Option<&'a str>,
}

fn verify_stream_completion(
    downloaded: u64,
    target: &StreamTarget<'_>,
    actual_hex: &str,
) -> Result<bool, UpdateError> {
    if let Some(expected) = target.expected_size {
        if downloaded < expected {
            return Err(UpdateError::DownloadTruncated);
        }
    } else if let Some(cl) = target.content_length
        && downloaded < cl
    {
        return Err(UpdateError::DownloadTruncated);
    }

    if let Some(expected_hex) = target.expected_sha256 {
        if actual_hex != expected_hex {
            return Err(UpdateError::ChecksumMismatch);
        }
        return Ok(true);
    }
    Ok(false)
}

fn stream_to_part_file(
    reader: &mut dyn Read,
    target: &StreamTarget<'_>,
    cancel: &AtomicBool,
    on_progress: &mut impl FnMut(u64, Option<u64>),
) -> Result<(u64, bool), UpdateError> {
    let mut file = File::create(target.part_path).map_err(|e| UpdateError::Io(e.to_string()))?;
    let mut buf = vec![0_u8; DOWNLOAD_CHUNK_BYTES];
    let mut hasher = ring::digest::Context::new(&ring::digest::SHA256);
    let mut downloaded: u64 = 0;
    let total = target.expected_size.or(target.content_length);

    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(UpdateError::Cancelled);
        }
        let n = match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(err) if err.kind() == io::ErrorKind::TimedOut => return Err(UpdateError::Timeout),
            Err(err) => return Err(UpdateError::Network(err.to_string())),
        };
        downloaded = downloaded.saturating_add(n as u64);
        if downloaded > MAX_DOWNLOAD_BYTES
            || target.expected_size.is_some_and(|exp| downloaded > exp)
        {
            return Err(UpdateError::DownloadTooLarge);
        }
        let Some(chunk) = buf.get(..n) else {
            return Err(UpdateError::InvalidResponse);
        };
        file.write_all(chunk)
            .map_err(|e| UpdateError::Io(e.to_string()))?;
        hasher.update(chunk);
        on_progress(downloaded, total);
    }

    if cancel.load(Ordering::Relaxed) {
        return Err(UpdateError::Cancelled);
    }
    file.flush().map_err(|e| UpdateError::Io(e.to_string()))?;
    let actual_hex = encode_hex_lower(hasher.finish().as_ref());
    let sha256_verified = verify_stream_completion(downloaded, target, &actual_hex)?;
    Ok((downloaded, sha256_verified))
}

/// Stream a release installer to a `.part` file under `spec.dest_dir`, verify
/// size limits and SHA-256 (from `expected_sha256` or `SHA256SUMS.txt`), and
/// atomically rename to a collision-free final path. Removes `.part` on any
/// error or cancellation.
///
/// # Errors
/// Returns [`UpdateError`] on invalid filename/URL, cancellation, size limit or
/// truncation failure, SHA-256 mismatch, HTTP/network failure, or local I/O error.
pub fn download_asset_verified(
    http: &dyn UpdateHttp,
    spec: &DownloadSpec<'_>,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(u64, Option<u64>),
) -> Result<DownloadResult, UpdateError> {
    if cancel.load(Ordering::Relaxed) {
        return Err(UpdateError::Cancelled);
    }
    let safe_name = safe_asset_filename(spec.filename).ok_or(UpdateError::InvalidResponse)?;
    if !is_allowed_update_url(spec.url) {
        return Err(UpdateError::DisallowedUrl);
    }
    if spec.expected_size.is_some_and(|s| s > MAX_DOWNLOAD_BYTES) {
        return Err(UpdateError::DownloadTooLarge);
    }

    let expected_sha256 = resolve_expected_sha256(http, spec, &safe_name)?;
    if cancel.load(Ordering::Relaxed) {
        return Err(UpdateError::Cancelled);
    }

    let mut resp = follow_get_redirects(
        http,
        spec.url,
        "application/octet-stream",
        spec.user_agent,
        RequestKind::Download,
    )?;
    check_http_status(resp.status, resp.rate_limit_remaining)?;
    if resp
        .content_length
        .is_some_and(|cl| cl > MAX_DOWNLOAD_BYTES)
        || spec
            .expected_size
            .is_some_and(|exp| resp.content_length.is_some_and(|cl| cl > exp))
    {
        return Err(UpdateError::DownloadTooLarge);
    }

    let dest = unique_download_path(spec.dest_dir, &safe_name);
    let part = part_path_for(&dest);
    let target = StreamTarget {
        part_path: &part,
        expected_size: spec.expected_size,
        content_length: resp.content_length,
        expected_sha256: expected_sha256.as_deref(),
    };

    let (bytes, sha256_verified) =
        match stream_to_part_file(&mut *resp.body, &target, cancel, &mut on_progress) {
            Ok(res) => res,
            Err(err) => {
                let _ = std::fs::remove_file(&part);
                return Err(err);
            }
        };

    if let Err(err) = std::fs::rename(&part, &dest) {
        let _ = std::fs::remove_file(&part);
        return Err(UpdateError::Io(err.to_string()));
    }

    Ok(DownloadResult {
        path: dest,
        bytes,
        sha256_verified,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::io::{self, Cursor, Read};
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    use super::{
        CHECK_INTERVAL_SECS, CheckOutcome, DownloadSpec, GitHubAsset, GitHubRelease, HttpResponse,
        MAX_API_RESPONSE_BYTES, MAX_DOWNLOAD_BYTES, MAX_SUMS_BYTES, RequestKind, UpdateError,
        UpdateHttp, auto_check_suppressed_by_env, check_for_update_with, current_version,
        download_asset_verified, encode_hex_lower, find_checksums_asset, follow_get_redirects,
        is_allowed_update_url, is_release_eligible, parse_releases_json, parse_sha256_digest,
        parse_sha256sums_line, parse_tag_version, pick_asset, resolve_redirect_url,
        safe_asset_filename, select_latest_update, should_auto_check, unique_download_path,
        user_agent,
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

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct RecordedRequest {
        url: String,
        accept: String,
        user_agent: String,
        kind: RequestKind,
    }

    struct FakeHttp {
        requests: Mutex<Vec<RecordedRequest>>,
        responses: Mutex<VecDeque<Result<HttpResponse, UpdateError>>>,
    }

    impl FakeHttp {
        fn new(responses: Vec<Result<HttpResponse, UpdateError>>) -> Self {
            Self {
                requests: Mutex::new(Vec::new()),
                responses: Mutex::new(VecDeque::from(responses)),
            }
        }

        fn recorded(&self) -> Vec<RecordedRequest> {
            self.requests.lock().expect("requests lock").clone()
        }
    }

    impl UpdateHttp for FakeHttp {
        fn get(
            &self,
            url: &str,
            accept: &str,
            user_agent: &str,
            kind: RequestKind,
        ) -> Result<HttpResponse, UpdateError> {
            self.requests
                .lock()
                .expect("requests lock")
                .push(RecordedRequest {
                    url: url.to_string(),
                    accept: accept.to_string(),
                    user_agent: user_agent.to_string(),
                    kind,
                });
            self.responses
                .lock()
                .expect("responses lock")
                .pop_front()
                .expect("unexpected extra HTTP request in FakeHttp")
        }
    }

    fn ok_response(body: Vec<u8>) -> HttpResponse {
        let len = body.len() as u64;
        HttpResponse {
            status: 200,
            location: None,
            content_length: Some(len),
            rate_limit_remaining: Some(59),
            body: Box::new(Cursor::new(body)),
        }
    }

    fn redirect_response(status: u16, location: Option<&str>) -> HttpResponse {
        HttpResponse {
            status,
            location: location.map(str::to_owned),
            content_length: Some(0),
            rate_limit_remaining: Some(59),
            body: Box::new(Cursor::new(Vec::new())),
        }
    }

    fn status_response(status: u16, rate_limit_remaining: Option<u64>) -> HttpResponse {
        HttpResponse {
            status,
            location: None,
            content_length: Some(0),
            rate_limit_remaining,
            body: Box::new(Cursor::new(Vec::new())),
        }
    }

    struct TempTestDir {
        path: PathBuf,
    }

    impl TempTestDir {
        fn new(label: &str) -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "coursecraft-m35-{label}-{}-{n}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).expect("create temp test dir");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }

        fn entries(&self) -> Vec<String> {
            std::fs::read_dir(&self.path)
                .expect("read temp dir")
                .map(|e| {
                    e.expect("dir entry")
                        .file_name()
                        .to_string_lossy()
                        .into_owned()
                })
                .collect()
        }
    }

    impl Drop for TempTestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    fn sha256_hex(data: &[u8]) -> String {
        let digest = ring::digest::digest(&ring::digest::SHA256, data);
        encode_hex_lower(digest.as_ref())
    }

    struct ChunkedReader {
        data: Vec<u8>,
        pos: usize,
        max_chunk: usize,
    }

    impl Read for ChunkedReader {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let remaining = self.data.len().saturating_sub(self.pos);
            if remaining == 0 {
                return Ok(0);
            }
            let take = remaining.min(buf.len()).min(self.max_chunk);
            buf[..take].copy_from_slice(&self.data[self.pos..self.pos + take]);
            self.pos += take;
            Ok(take)
        }
    }

    #[test]
    fn up06_url_allowlist_and_redirect_resolution() {
        let allowed = [
            "https://api.github.com/repos/ad1-kamath/coursecraft/releases?per_page=20",
            "https://github.com/ad1-kamath/coursecraft/releases/download/v0.4.3/CourseCraft-0.4.3-macos.dmg",
            "https://objects.githubusercontent.com/github-production-release-asset-123/file.dmg?sig=abc",
            "https://release-assets.githubusercontent.com/asset/file.dmg",
            "https://a.b.githubusercontent.com/asset",
            "https://API.GITHUB.COM/repos/foo",
            "https://github.com:443/foo",
        ];
        for url in allowed {
            assert!(is_allowed_update_url(url), "should allow {url}");
        }

        let rejected = [
            "http://github.com/ad1-kamath/coursecraft/releases",
            "file:///etc/passwd",
            "https://evil.com/file.dmg",
            "https://github.com.evil.com/file.dmg",
            "https://127.0.0.1/file.dmg",
            "https://localhost/file.dmg",
            "https://[::1]/file.dmg",
            "https://githubusercontent.com/file.dmg",
            "https://.githubusercontent.com/file.dmg",
            "https://a..b.githubusercontent.com/file.dmg",
            "https://-bad.githubusercontent.com/file.dmg",
            "https://user@github.com/file.dmg",
            "https://github.com:8443/file.dmg",
            "https://github.com:/file.dmg",
            "https:///file.dmg",
            "https://github.com/a b",
            "https://github.com/a\nb",
            "https://github.com/a\0b",
        ];
        for url in rejected {
            assert!(!is_allowed_update_url(url), "should reject {url}");
        }

        let base = "https://github.com/ad1-kamath/coursecraft/releases/download/v0.4.3/a.dmg";
        let resolved_cases = [
            (
                "https://objects.githubusercontent.com/blob",
                Some("https://objects.githubusercontent.com/blob"),
            ),
            (
                "//objects.githubusercontent.com/blob?x=1",
                Some("https://objects.githubusercontent.com/blob?x=1"),
            ),
            (
                "/releases/step2?token=abc",
                Some("https://github.com/releases/step2?token=abc"),
            ),
            ("http://objects.githubusercontent.com/blob", None),
            ("file:///tmp/x", None),
            ("", None),
            ("///bad", None),
            ("relative/path", None),
            ("https://github.com/has space", None),
            ("https://github.com/has\r\ncrlf", None),
        ];
        for (location, expected) in resolved_cases {
            assert_eq!(
                resolve_redirect_url(base, location).as_deref(),
                expected,
                "redirect location {location:?}"
            );
        }
    }

    #[test]
    fn up07_safe_filename_and_unique_download_path() {
        // Rejected filenames.
        assert_eq!(safe_asset_filename(""), None);
        assert_eq!(safe_asset_filename("   "), None);
        assert_eq!(safe_asset_filename("../evil.dmg"), None);
        assert_eq!(safe_asset_filename("foo..bar.dmg"), None);
        assert_eq!(safe_asset_filename("a/b.dmg"), None);
        assert_eq!(safe_asset_filename("a\\b.dmg"), None);
        assert_eq!(safe_asset_filename("C:evil.exe"), None);
        assert_eq!(safe_asset_filename(".hidden"), None);
        assert_eq!(safe_asset_filename("bad\0name.dmg"), None);
        assert_eq!(safe_asset_filename("bad\nname.dmg"), None);
        let too_long = format!("{}.dmg", "a".repeat(197));
        assert_eq!(too_long.chars().count(), 201);
        assert_eq!(safe_asset_filename(&too_long), None);

        // Accepted filenames.
        assert_eq!(
            safe_asset_filename("CourseCraft-0.4.4-macos.dmg"),
            Some("CourseCraft-0.4.4-macos.dmg".to_string())
        );
        let exact_max = format!("{}.dmg", "a".repeat(196));
        assert_eq!(exact_max.chars().count(), 200);
        assert_eq!(safe_asset_filename(&exact_max), Some(exact_max));

        // Unique download path avoids overwriting and preserves `.tar.gz`.
        let tmp = TempTestDir::new("up07");
        let p0 = unique_download_path(tmp.path(), "foo.dmg");
        assert_eq!(p0, tmp.path().join("foo.dmg"));
        std::fs::write(&p0, b"first").expect("write p0");

        let p1 = unique_download_path(tmp.path(), "foo.dmg");
        assert_eq!(p1, tmp.path().join("foo (1).dmg"));
        std::fs::write(&p1, b"second").expect("write p1");

        let p2 = unique_download_path(tmp.path(), "foo.dmg");
        assert_eq!(p2, tmp.path().join("foo (2).dmg"));
        assert_eq!(std::fs::read(&p0).expect("read p0"), b"first");
        assert_eq!(std::fs::read(&p1).expect("read p1"), b"second");

        let t0 = unique_download_path(tmp.path(), "app.tar.gz");
        assert_eq!(t0, tmp.path().join("app.tar.gz"));
        std::fs::write(&t0, b"tar0").expect("write t0");

        let t1 = unique_download_path(tmp.path(), "app.tar.gz");
        assert_eq!(t1, tmp.path().join("app (1).tar.gz"));
        std::fs::write(&t1, b"tar1").expect("write t1");

        let t2 = unique_download_path(tmp.path(), "app.tar.gz");
        assert_eq!(t2, tmp.path().join("app (2).tar.gz"));
    }

    #[test]
    fn dl01_happy_path_check_and_privacy_headers() {
        let api_url = "https://api.github.com/repos/ad1-kamath/coursecraft/releases?per_page=20";
        let current = semver::Version::new(0, 4, 2);
        let http = FakeHttp::new(vec![Ok(ok_response(
            V043_RELEASES_JSON.as_bytes().to_vec(),
        ))]);

        let outcome = check_for_update_with(&http, api_url, &current, "macos", "aarch64", false)
            .expect("check succeeds");

        let recorded = http.recorded();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].url, api_url);
        assert_eq!(recorded[0].accept, "application/vnd.github+json");
        assert_eq!(recorded[0].user_agent, "CourseCraft/0.4.2");
        assert_eq!(recorded[0].kind, RequestKind::Metadata);
        assert!(!recorded[0].user_agent.contains("macos"));
        assert!(!recorded[0].user_agent.contains("aarch64"));

        let CheckOutcome::Available(update) = outcome else {
            panic!("expected CheckOutcome::Available");
        };
        assert_eq!(update.version, "0.4.3");
        assert_eq!(update.current_version, "0.4.2");
        assert_eq!(
            update.release_url,
            "https://github.com/ad1-kamath/coursecraft/releases/tag/v0.4.3"
        );
        assert!(update.notes_markdown.contains("Faster, lighter editor"));
        assert_eq!(update.published_at.as_deref(), Some("2026-10-12T18:30:00Z"));
        let asset = update.asset.expect("macOS asset matched");
        assert_eq!(asset.name, "CourseCraft-0.4.3-macos.dmg");
        assert_eq!(asset.size, 28_677_774);
        assert_eq!(
            asset.sha256_digest.as_deref(),
            Some("f6caeb20ad874ade46c8bf992cb70e777d30ae273877667de6671b4acae051e3")
        );

        // Already at 0.4.3 -> UpToDate.
        let http_up_to_date = FakeHttp::new(vec![Ok(ok_response(
            V043_RELEASES_JSON.as_bytes().to_vec(),
        ))]);
        let same = check_for_update_with(
            &http_up_to_date,
            api_url,
            &semver::Version::new(0, 4, 3),
            "macos",
            "aarch64",
            false,
        )
        .expect("up to date check succeeds");
        assert_eq!(same, CheckOutcome::UpToDate);

        // Malformed html_url falls back to the canonical releases page.
        let bad_url_json = br#"[{"tag_name":"v0.4.4","html_url":"http://evil.com/phish"}]"#;
        let http_bad_url = FakeHttp::new(vec![Ok(ok_response(bad_url_json.to_vec()))]);
        let fallback =
            check_for_update_with(&http_bad_url, api_url, &current, "macos", "aarch64", false)
                .expect("fallback check succeeds");
        let CheckOutcome::Available(fallback_update) = fallback else {
            panic!("expected Available with fallback URL");
        };
        assert_eq!(
            fallback_update.release_url,
            "https://github.com/ad1-kamath/coursecraft/releases"
        );
    }

    #[test]
    fn dl02_rate_limits_and_offline_errors() {
        let api_url = "https://api.github.com/repos/ad1-kamath/coursecraft/releases?per_page=20";
        let current = semver::Version::new(0, 4, 2);

        // HTTP 403 with X-RateLimit-Remaining: 0 -> RateLimited.
        let http_403_rl = FakeHttp::new(vec![Ok(status_response(403, Some(0)))]);
        let err = check_for_update_with(&http_403_rl, api_url, &current, "macos", "aarch64", false)
            .expect_err("403 rate limited");
        assert_eq!(err, UpdateError::RateLimited);
        assert!(!err.user_message().is_empty());

        // HTTP 403 with remaining > 0 -> HttpStatus(403).
        let http_403_plain = FakeHttp::new(vec![Ok(status_response(403, Some(10)))]);
        let err = check_for_update_with(
            &http_403_plain,
            api_url,
            &current,
            "macos",
            "aarch64",
            false,
        )
        .expect_err("403 non-rate-limit");
        assert_eq!(err, UpdateError::HttpStatus(403));
        assert!(err.user_message().contains("403"));

        // HTTP 429 -> RateLimited.
        let http_429 = FakeHttp::new(vec![Ok(status_response(429, None))]);
        let err = check_for_update_with(&http_429, api_url, &current, "macos", "aarch64", false)
            .expect_err("429 rate limited");
        assert_eq!(err, UpdateError::RateLimited);
        assert!(!err.user_message().is_empty());

        // Network failure -> UpdateError::Network with calm message.
        let http_net = FakeHttp::new(vec![Err(UpdateError::Network(
            "dns lookup failed: 127.0.0.1".to_string(),
        ))]);
        let err = check_for_update_with(&http_net, api_url, &current, "macos", "aarch64", false)
            .expect_err("network error");
        assert!(matches!(err, UpdateError::Network(_)));
        assert!(!err.user_message().is_empty());
        assert!(!err.user_message().contains("127.0.0.1"));

        // Timeout -> UpdateError::Timeout with calm message.
        let http_timeout = FakeHttp::new(vec![Err(UpdateError::Timeout)]);
        let err =
            check_for_update_with(&http_timeout, api_url, &current, "macos", "aarch64", false)
                .expect_err("timeout error");
        assert_eq!(err, UpdateError::Timeout);
        assert!(!err.user_message().is_empty());

        // Oversized API response (> MAX_API_RESPONSE_BYTES) -> InvalidResponse.
        let mut oversized = vec![b' '; MAX_API_RESPONSE_BYTES + 1];
        oversized[0] = b'[';
        oversized[MAX_API_RESPONSE_BYTES] = b']';
        let http_big = FakeHttp::new(vec![Ok(ok_response(oversized))]);
        let err = check_for_update_with(&http_big, api_url, &current, "macos", "aarch64", false)
            .expect_err("oversized api body");
        assert_eq!(err, UpdateError::InvalidResponse);
    }

    #[test]
    fn dl03_redirects_valid_and_hostile() {
        let start_url =
            "https://github.com/ad1-kamath/coursecraft/releases/download/v0.4.3/CourseCraft.dmg";
        let payload = b"redirected-installer-bytes".to_vec();
        let digest = sha256_hex(&payload);
        let tmp = TempTestDir::new("dl03");

        // 2 valid HTTPS redirects (origin-relative then scheme-relative to objects.githubusercontent.com).
        let http_valid = FakeHttp::new(vec![
            Ok(redirect_response(302, Some("/download/hop1"))),
            Ok(redirect_response(
                307,
                Some("//objects.githubusercontent.com/assets/CourseCraft.dmg"),
            )),
            Ok(ok_response(payload.clone())),
        ]);
        let spec = DownloadSpec {
            url: start_url,
            expected_size: Some(payload.len() as u64),
            expected_sha256: Some(&digest),
            checksums_url: None,
            dest_dir: tmp.path(),
            filename: "CourseCraft.dmg",
            user_agent: "CourseCraft/0.4.2",
        };
        let cancel = AtomicBool::new(false);
        let result = download_asset_verified(&http_valid, &spec, &cancel, |_, _| {})
            .expect("2 valid redirects succeed");
        assert_eq!(result.bytes, payload.len() as u64);
        assert!(result.sha256_verified);
        assert_eq!(std::fs::read(&result.path).expect("read result"), payload);
        let hops: Vec<String> = http_valid.recorded().into_iter().map(|r| r.url).collect();
        assert_eq!(
            hops,
            vec![
                start_url.to_string(),
                "https://github.com/download/hop1".to_string(),
                "https://objects.githubusercontent.com/assets/CourseCraft.dmg".to_string(),
            ]
        );

        // HTTP downgrade rejected.
        let http_downgrade = FakeHttp::new(vec![Ok(redirect_response(
            302,
            Some("http://objects.githubusercontent.com/assets/CourseCraft.dmg"),
        ))]);
        let err = follow_get_redirects(
            &http_downgrade,
            start_url,
            "application/octet-stream",
            "CourseCraft/0.4.2",
            RequestKind::Download,
        )
        .err()
        .expect("http downgrade rejected");
        assert_eq!(err, UpdateError::DisallowedUrl);

        // Disallowed host rejected without contacting evil.com.
        let http_evil = FakeHttp::new(vec![Ok(redirect_response(
            302,
            Some("https://evil.com/CourseCraft.dmg"),
        ))]);
        let err = follow_get_redirects(
            &http_evil,
            start_url,
            "application/octet-stream",
            "CourseCraft/0.4.2",
            RequestKind::Download,
        )
        .err()
        .expect("disallowed redirect host rejected");
        assert_eq!(err, UpdateError::DisallowedUrl);
        assert_eq!(http_evil.recorded().len(), 1);

        // Missing Location header rejected.
        let http_missing_loc = FakeHttp::new(vec![Ok(redirect_response(302, None))]);
        let err = follow_get_redirects(
            &http_missing_loc,
            start_url,
            "application/octet-stream",
            "CourseCraft/0.4.2",
            RequestKind::Download,
        )
        .err()
        .expect("missing location rejected");
        assert_eq!(err, UpdateError::DisallowedUrl);

        // > 5 hops (6 redirects) rejected with TooManyRedirects.
        let http_loop = FakeHttp::new(vec![
            Ok(redirect_response(302, Some("https://github.com/hop1"))),
            Ok(redirect_response(302, Some("https://github.com/hop2"))),
            Ok(redirect_response(302, Some("https://github.com/hop3"))),
            Ok(redirect_response(302, Some("https://github.com/hop4"))),
            Ok(redirect_response(302, Some("https://github.com/hop5"))),
            Ok(redirect_response(302, Some("https://github.com/hop6"))),
        ]);
        let err = follow_get_redirects(
            &http_loop,
            start_url,
            "application/octet-stream",
            "CourseCraft/0.4.2",
            RequestKind::Download,
        )
        .err()
        .expect("too many redirects");
        assert_eq!(err, UpdateError::TooManyRedirects);
    }

    #[test]
    fn dl04_streaming_download_verifies_asset_digest() {
        let tmp = TempTestDir::new("dl04");
        let payload: Vec<u8> = (0..150_000_usize).map(|i| (i % 251) as u8).collect();
        let digest = format!("sha256:{}", sha256_hex(&payload));
        let http = FakeHttp::new(vec![Ok(ok_response(payload.clone()))]);
        let spec = DownloadSpec {
            url: "https://github.com/ad1-kamath/coursecraft/releases/download/v0.4.3/CourseCraft-0.4.3-macos.dmg",
            expected_size: Some(payload.len() as u64),
            expected_sha256: Some(&digest),
            checksums_url: None,
            dest_dir: tmp.path(),
            filename: "CourseCraft-0.4.3-macos.dmg",
            user_agent: "CourseCraft/0.4.2",
        };
        let cancel = AtomicBool::new(false);
        let mut progress_events = Vec::new();

        let result = download_asset_verified(&http, &spec, &cancel, |done, total| {
            progress_events.push((done, total));
        })
        .expect("verified download succeeds");

        assert_eq!(result.path, tmp.path().join("CourseCraft-0.4.3-macos.dmg"));
        assert_eq!(result.bytes, payload.len() as u64);
        assert!(result.sha256_verified);
        assert_eq!(std::fs::read(&result.path).expect("read file"), payload);
        assert!(
            progress_events.len() >= 2,
            "expected multiple 64 KiB chunks"
        );
        assert_eq!(
            progress_events.last().copied(),
            Some((payload.len() as u64, Some(payload.len() as u64)))
        );
        let entries = tmp.entries();
        assert_eq!(entries, vec!["CourseCraft-0.4.3-macos.dmg".to_string()]);
    }

    #[test]
    fn dl05_fallback_sha256sums_verification() {
        let tmp = TempTestDir::new("dl05");
        let payload = b"installer-verified-via-sha256sums".to_vec();
        let digest = sha256_hex(&payload);
        let sums_body = format!(
            "# CourseCraft checksums\n\
             0000000000000000000000000000000000000000000000000000000000000000  other.exe\n\
             {digest}  CourseCraft-v0.4.3-setup.exe\n"
        );
        let sums_url =
            "https://github.com/ad1-kamath/coursecraft/releases/download/v0.4.3/SHA256SUMS.txt";
        let asset_url = "https://github.com/ad1-kamath/coursecraft/releases/download/v0.4.3/CourseCraft-v0.4.3-setup.exe";

        let http = FakeHttp::new(vec![
            Ok(ok_response(sums_body.into_bytes())),
            Ok(ok_response(payload.clone())),
        ]);
        let spec = DownloadSpec {
            url: asset_url,
            expected_size: Some(payload.len() as u64),
            expected_sha256: None,
            checksums_url: Some(sums_url),
            dest_dir: tmp.path(),
            filename: "CourseCraft-v0.4.3-setup.exe",
            user_agent: "CourseCraft/0.4.2",
        };
        let cancel = AtomicBool::new(false);
        let result = download_asset_verified(&http, &spec, &cancel, |_, _| {})
            .expect("fallback SHA256SUMS download succeeds");
        assert!(result.sha256_verified);
        assert_eq!(result.bytes, payload.len() as u64);
        assert_eq!(std::fs::read(&result.path).expect("read setup"), payload);

        let recorded = http.recorded();
        assert_eq!(recorded.len(), 2);
        assert_eq!(recorded[0].kind, RequestKind::Checksums);
        assert_eq!(recorded[1].kind, RequestKind::Download);

        // Oversized SHA256SUMS.txt (> MAX_SUMS_BYTES) is rejected safely before downloading asset.
        let tmp_big = TempTestDir::new("dl05-big");
        let huge_sums = vec![b'a'; MAX_SUMS_BYTES + 1];
        let http_big = FakeHttp::new(vec![Ok(ok_response(huge_sums))]);
        let spec_big = DownloadSpec {
            dest_dir: tmp_big.path(),
            ..spec
        };
        let err = download_asset_verified(&http_big, &spec_big, &cancel, |_, _| {})
            .expect_err("oversized SHA256SUMS rejected");
        assert_eq!(err, UpdateError::InvalidResponse);
        assert_eq!(http_big.recorded().len(), 1);
        assert!(tmp_big.entries().is_empty());
    }

    #[test]
    fn dl06_checksum_mismatch_truncated_and_oversized_cleanup() {
        let asset_url =
            "https://github.com/ad1-kamath/coursecraft/releases/download/v0.4.3/CourseCraft.dmg";
        let cancel = AtomicBool::new(false);

        // 1. Checksum mismatch removes `.part` and leaves no final file.
        let tmp_bad_hash = TempTestDir::new("dl06-hash");
        let payload = b"tampered-installer-payload".to_vec();
        let wrong_hash = "0000000000000000000000000000000000000000000000000000000000000000";
        let http_hash = FakeHttp::new(vec![Ok(ok_response(payload.clone()))]);
        let spec_hash = DownloadSpec {
            url: asset_url,
            expected_size: Some(payload.len() as u64),
            expected_sha256: Some(wrong_hash),
            checksums_url: None,
            dest_dir: tmp_bad_hash.path(),
            filename: "CourseCraft.dmg",
            user_agent: "CourseCraft/0.4.2",
        };
        let err = download_asset_verified(&http_hash, &spec_hash, &cancel, |_, _| {})
            .expect_err("checksum mismatch fails");
        assert_eq!(err, UpdateError::ChecksumMismatch);
        assert!(tmp_bad_hash.entries().is_empty());

        // 2. Truncated stream (`downloaded < expected_size`) fails and removes `.part`.
        let tmp_trunc = TempTestDir::new("dl06-trunc");
        let short_body = b"only-half".to_vec();
        let http_trunc = FakeHttp::new(vec![Ok(ok_response(short_body.clone()))]);
        let spec_trunc = DownloadSpec {
            url: asset_url,
            expected_size: Some(100),
            expected_sha256: None,
            checksums_url: None,
            dest_dir: tmp_trunc.path(),
            filename: "CourseCraft.dmg",
            user_agent: "CourseCraft/0.4.2",
        };
        let err = download_asset_verified(&http_trunc, &spec_trunc, &cancel, |_, _| {})
            .expect_err("truncated download fails");
        assert_eq!(err, UpdateError::DownloadTruncated);
        assert!(tmp_trunc.entries().is_empty());

        // 3. Oversized stream (`downloaded > expected_size`) fails and removes `.part`.
        let tmp_over = TempTestDir::new("dl06-over");
        let extra_body = vec![0xAB_u8; 200];
        let http_over = FakeHttp::new(vec![Ok(HttpResponse {
            status: 200,
            location: None,
            content_length: None,
            rate_limit_remaining: Some(59),
            body: Box::new(Cursor::new(extra_body)),
        })]);
        let spec_over = DownloadSpec {
            url: asset_url,
            expected_size: Some(100),
            expected_sha256: None,
            checksums_url: None,
            dest_dir: tmp_over.path(),
            filename: "CourseCraft.dmg",
            user_agent: "CourseCraft/0.4.2",
        };
        let err = download_asset_verified(&http_over, &spec_over, &cancel, |_, _| {})
            .expect_err("oversized stream fails");
        assert_eq!(err, UpdateError::DownloadTooLarge);
        assert!(tmp_over.entries().is_empty());

        // 4. Expected size exceeding MAX_DOWNLOAD_BYTES fails immediately.
        let tmp_huge = TempTestDir::new("dl06-huge");
        let http_huge = FakeHttp::new(vec![]);
        let spec_huge = DownloadSpec {
            url: asset_url,
            expected_size: Some(MAX_DOWNLOAD_BYTES + 1),
            expected_sha256: None,
            checksums_url: None,
            dest_dir: tmp_huge.path(),
            filename: "CourseCraft.dmg",
            user_agent: "CourseCraft/0.4.2",
        };
        let err = download_asset_verified(&http_huge, &spec_huge, &cancel, |_, _| {})
            .expect_err("over MAX_DOWNLOAD_BYTES fails");
        assert_eq!(err, UpdateError::DownloadTooLarge);
        assert!(http_huge.recorded().is_empty());
        assert!(tmp_huge.entries().is_empty());
    }

    #[test]
    fn dl07_mid_download_cancellation_removes_part() {
        let tmp = TempTestDir::new("dl07");
        let payload = vec![0x5A_u8; 256 * 1024];
        let total = payload.len() as u64;
        let reader = ChunkedReader {
            data: payload,
            pos: 0,
            max_chunk: 64 * 1024,
        };
        let http = FakeHttp::new(vec![Ok(HttpResponse {
            status: 200,
            location: None,
            content_length: Some(total),
            rate_limit_remaining: Some(59),
            body: Box::new(reader),
        })]);
        let spec = DownloadSpec {
            url: "https://github.com/ad1-kamath/coursecraft/releases/download/v0.4.3/CourseCraft.dmg",
            expected_size: Some(total),
            expected_sha256: None,
            checksums_url: None,
            dest_dir: tmp.path(),
            filename: "CourseCraft.dmg",
            user_agent: "CourseCraft/0.4.2",
        };
        let cancel = AtomicBool::new(false);
        let mut chunks_seen = 0_usize;

        let err = download_asset_verified(&http, &spec, &cancel, |_done, _total| {
            chunks_seen += 1;
            if chunks_seen == 1 {
                cancel.store(true, Ordering::Relaxed);
            }
        })
        .expect_err("mid-download cancel returns Err");

        assert_eq!(err, UpdateError::Cancelled);
        assert_eq!(chunks_seen, 1);
        assert!(
            tmp.entries().is_empty(),
            "no .part or final file should remain after cancellation"
        );
    }
}
