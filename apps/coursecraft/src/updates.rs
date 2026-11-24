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

#[cfg(test)]
mod tests {
    use super::{
        CHECK_INTERVAL_SECS, auto_check_suppressed_by_env, current_version, should_auto_check,
        user_agent,
    };

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
