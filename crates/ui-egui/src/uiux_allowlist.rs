//! Allowlist for justified UI/UX sweep exceptions (Reviewer note #6).
//!
//! Every entry must be specific (never a whole rule or whole surface) and carry a concrete
//! one-line reason. At most 20 entries total.

/// One justified exception to the automated UI/UX sweep.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AllowlistEntry {
    pub rule: &'static str,
    pub surface: &'static str,
    pub needle: &'static str,
    pub reason: &'static str,
}

/// Maximum number of allowlist entries permitted by the M28 UI/UX plan.
pub(crate) const MAX_ALLOWLIST_ENTRIES: usize = 20;

/// Specific, reviewed exceptions. Keep this list as close to empty as possible.
pub(crate) const ALLOWLIST: &[AllowlistEntry] = &[
    AllowlistEntry {
        rule: "UX-04",
        surface: "long_names_course",
        needle: "Object 00 OOOOO",
        reason: "TextEdit::singleline horizontally clips 120-char object name inside its input box.",
    },
    AllowlistEntry {
        rule: "UX-16",
        surface: "long_names_course",
        needle: "Object 00 OOOOO",
        reason: "TextEdit::singleline text galley extends past viewport before clip_rect applies.",
    },
    AllowlistEntry {
        rule: "UX-04",
        surface: "long_names_course",
        needle: "Slide 02 LLLLL",
        reason: "TextEdit::singleline horizontally clips 120-char slide title inside its input box.",
    },
    AllowlistEntry {
        rule: "UX-16",
        surface: "long_names_course",
        needle: "Slide 02 LLLLL",
        reason: "TextEdit::singleline text galley extends past viewport before clip_rect applies.",
    },
    AllowlistEntry {
        rule: "UX-04",
        surface: "long_names_course",
        needle: "AAAAAAAAAAAA",
        reason: "TextEdit::singleline horizontally clips 120-char course title inside its input box.",
    },
    AllowlistEntry {
        rule: "UX-16",
        surface: "long_names_course",
        needle: "AAAAAAAAAAAA",
        reason: "TextEdit::singleline text galley extends past viewport before clip_rect applies.",
    },
    AllowlistEntry {
        rule: "UX-04",
        surface: "long_names_course",
        needle: "Initial value 0",
        reason: "TextEdit::singleline horizontally clips long variable initial value inside its input box.",
    },
];

/// Returns `Some(reason)` if a finding is explicitly allowlisted.
pub(crate) fn match_allowlist(rule: &str, surface: &str, detail: &str) -> Option<&'static str> {
    ALLOWLIST.iter().find_map(|entry| {
        if entry.rule == rule && entry.surface == surface && detail.contains(entry.needle) {
            Some(entry.reason)
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_stays_within_budget_and_has_no_blanket_entries() {
        assert!(
            ALLOWLIST.len() <= MAX_ALLOWLIST_ENTRIES,
            "allowlist has {} entries, max is {MAX_ALLOWLIST_ENTRIES}",
            ALLOWLIST.len()
        );
        for entry in ALLOWLIST {
            assert!(!entry.rule.is_empty(), "blanket rule in allowlist");
            assert!(!entry.surface.is_empty(), "blanket surface in allowlist");
            assert!(!entry.needle.is_empty(), "blanket needle in allowlist");
            assert!(!entry.reason.is_empty(), "missing reason in allowlist");
        }
    }
}
