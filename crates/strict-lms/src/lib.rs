//! Strict LMS conformance engines for CourseCraft testing.
//!
//! Single source of truth for SCORM 1.2 and SCORM 2004 (3rd & 4th Edition)
//! Run-Time Environment validation, shared across native tests and the browser
//! mock LMS. Pure L1 crate: zero filesystem, zero sockets, zero `std::time`,
//! zero `unsafe`.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]

pub mod manifest;
pub mod scorm12;
pub mod scorm2004;
pub mod xapi_lrs;

pub use manifest::{
    PackageStandardKind, validate_cmi5_xml, validate_package, validate_scorm12_manifest,
    validate_scorm2004_manifest, validate_tincan_xml,
};
pub use scorm12::{Scorm12Config, StrictScorm12};
pub use scorm2004::{Scorm2004Config, Scorm2004Edition, StrictScorm2004};
pub use xapi_lrs::{
    Flavor, LaunchSpec, LrsResponse, StateKeyView, StrictLrs, build_cmi5_launch_data, canonical_ifi,
};

use serde::Serialize;

/// A strict specification violation or advisory warning recorded by an engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Violation {
    /// Standard identifier: `"scorm12"`, `"scorm2004"`, `"cmi5"`, `"xapi"`, `"lti13"`, or `"manifest"`.
    pub standard: &'static str,
    /// Stable rule identifier, e.g. `"SCORM12-405-TIMESPAN"`.
    pub rule: &'static str,
    /// 1-based API call or HTTP request index.
    pub call_index: usize,
    /// CMI key, xAPI path, header, or XML XPath.
    pub element: String,
    /// Concrete actual vs expected description.
    pub message: String,
    /// Exact SCORM error code (`"405"`, `"408"`, ...) or HTTP status if applicable.
    pub error_code: Option<String>,
}

/// Recorded API call made against a strict LMS engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CallRecord {
    /// 1-based call index.
    pub index: usize,
    /// API method name (e.g. `"LMSSetValue"` or `"SetValue"`).
    pub method: String,
    /// Arguments passed to the API method.
    pub args: Vec<String>,
    /// Return value of the API method.
    pub result: String,
    /// Resulting error code (`"0"` on success).
    pub error_code: String,
}

/// Parse a plain finite decimal (no scientific notation, no whitespace, no NaN/Inf).
pub(crate) fn parse_plain_decimal(s: &str) -> Option<f64> {
    if s.is_empty() {
        return None;
    }
    let bytes = s.as_bytes();
    let start = usize::from(bytes[0] == b'-');
    let rest = &s[start..];
    if rest.is_empty() {
        return None;
    }
    let mut dot_count = 0u8;
    let mut has_digit = false;
    for &b in rest.as_bytes() {
        if b.is_ascii_digit() {
            has_digit = true;
        } else if b == b'.' {
            dot_count += 1;
            if dot_count > 1 {
                return None;
            }
        } else {
            return None;
        }
    }
    if !has_digit {
        return None;
    }
    let val = s.parse::<f64>().ok()?;
    val.is_finite().then_some(val)
}

/// Validate a printable-ASCII identifier without whitespace.
pub(crate) fn is_valid_scorm_identifier(s: &str, max_chars: usize, ascii_only: bool) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut count = 0usize;
    for ch in s.chars() {
        count += 1;
        if count > max_chars || ch.is_whitespace() || ch.is_control() {
            return false;
        }
        if ascii_only && !ch.is_ascii_graphic() {
            return false;
        }
    }
    true
}

/// Parse `cmi.<prefix>.<index>.<rest>` where `<index>` is a non-negative integer.
pub(crate) fn parse_indexed_key<'a>(key: &'a str, prefix: &str) -> Option<(usize, &'a str)> {
    let rest = key.strip_prefix(prefix)?;
    let (idx_str, tail) = rest.split_once('.')?;
    if idx_str.is_empty() || !idx_str.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let idx = idx_str.parse::<usize>().ok()?;
    Some((idx, tail))
}

/// Parse a SCORM 1.2 `CMITimespan` (`HHHH:MM:SS(.SS)?`) into centiseconds.
pub(crate) fn parse_cmi_timespan_cs(s: &str) -> Option<u64> {
    let (hms, frac) = match s.split_once('.') {
        Some((l, r)) => {
            if r.is_empty() || r.len() > 2 || !r.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            let cs = if r.len() == 1 {
                r.parse::<u64>().ok()? * 10
            } else {
                r.parse::<u64>().ok()?
            };
            (l, cs)
        }
        None => (s, 0),
    };
    let mut parts = hms.split(':');
    let h_str = parts.next()?;
    let m_str = parts.next()?;
    let s_str = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    if !(2..=4).contains(&h_str.len()) || m_str.len() != 2 || s_str.len() != 2 {
        return None;
    }
    if !h_str.bytes().all(|b| b.is_ascii_digit())
        || !m_str.bytes().all(|b| b.is_ascii_digit())
        || !s_str.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let h: u64 = h_str.parse().ok()?;
    let m: u64 = m_str.parse().ok()?;
    let sec: u64 = s_str.parse().ok()?;
    if m > 59 || sec > 59 {
        return None;
    }
    Some(((h * 3600) + (m * 60) + sec) * 100 + frac)
}

/// Format centiseconds into SCORM 1.2 `CMITimespan` (`HHHH:MM:SS.SS`), saturating at `9999:59:59.99`.
pub(crate) fn format_cmi_timespan_cs(total_cs: u64) -> String {
    let max_cs = ((9999 * 3600) + (59 * 60) + 59) * 100 + 99;
    let clamped = total_cs.min(max_cs);
    let cs = clamped % 100;
    let total_secs = clamped / 100;
    let sec = total_secs % 60;
    let total_mins = total_secs / 60;
    let min = total_mins % 60;
    let hours = total_mins / 60;
    format!("{hours:04}:{min:02}:{sec:02}.{cs:02}")
}

/// Validate SCORM 1.2 `CMITime` (`HH:MM:SS(.SS)?`).
pub(crate) fn is_valid_cmi_time(s: &str) -> bool {
    let (hms, frac_ok) = match s.split_once('.') {
        Some((l, r)) => (
            l,
            !r.is_empty() && r.len() <= 2 && r.bytes().all(|b| b.is_ascii_digit()),
        ),
        None => (s, true),
    };
    if !frac_ok {
        return false;
    }
    let mut parts = hms.split(':');
    let (Some(h_str), Some(m_str), Some(s_str)) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    if parts.next().is_some() || h_str.len() != 2 || m_str.len() != 2 || s_str.len() != 2 {
        return false;
    }
    let Ok(h) = h_str.parse::<u8>() else {
        return false;
    };
    let Ok(m) = m_str.parse::<u8>() else {
        return false;
    };
    let Ok(sec) = s_str.parse::<u8>() else {
        return false;
    };
    h <= 23 && m <= 59 && sec <= 59
}

/// Validate ISO 8601 duration (`timeinterval(second,10,2)`):
/// `^P(?:(\d+)Y)?(?:(\d+)M)?(?:(\d+)D)?(?:T(?:(\d+)H)?(?:(\d+)M)?(?:(\d+(?:\.\d{1,2})?)S)?)?$`
/// Must have at least one component and never be bare `"P"` or `"PT"`.
pub(crate) fn is_valid_iso8601_duration(s: &str) -> bool {
    let Some(rest) = s.strip_prefix('P') else {
        return false;
    };
    if rest.is_empty() {
        return false;
    }
    let (date_part, time_part) = match rest.split_once('T') {
        Some((d, t)) => {
            if t.is_empty() {
                return false;
            }
            (d, Some(t))
        }
        None => (rest, None),
    };
    let mut has_component = false;
    if !date_part.is_empty() {
        if !parse_duration_components(date_part, &['Y', 'M', 'D'], false) {
            return false;
        }
        has_component = true;
    }
    if let Some(tp) = time_part {
        if !parse_duration_components(tp, &['H', 'M', 'S'], true) {
            return false;
        }
        has_component = true;
    }
    has_component
}

fn parse_duration_components(mut s: &str, order: &[char], allow_frac_on_last: bool) -> bool {
    let mut min_idx = 0usize;
    while !s.is_empty() {
        let Some(pos) = s.find(|c: char| order.contains(&c)) else {
            return false;
        };
        let num_str = &s[..pos];
        let unit = s[pos..].chars().next();
        let Some(unit_ch) = unit else {
            return false;
        };
        let Some(unit_idx) = order.iter().position(|&c| c == unit_ch) else {
            return false;
        };
        if unit_idx < min_idx {
            return false;
        }
        min_idx = unit_idx + 1;
        let is_last = unit_idx + 1 == order.len();
        if !is_valid_duration_number(num_str, allow_frac_on_last && is_last) {
            return false;
        }
        s = &s[pos + unit_ch.len_utf8()..];
    }
    true
}

fn is_valid_duration_number(s: &str, allow_frac: bool) -> bool {
    if s.is_empty() {
        return false;
    }
    if allow_frac && let Some((whole, frac)) = s.split_once('.') {
        return !whole.is_empty()
            && whole.bytes().all(|b| b.is_ascii_digit())
            && (1..=2).contains(&frac.len())
            && frac.bytes().all(|b| b.is_ascii_digit());
    }
    s.bytes().all(|b| b.is_ascii_digit())
}
