//! Debug-only counter for full [`Course`](coursecraft_course::Course) clones inside
//! [`Session`](crate::Session).
//!
//! Gated on `debug_assertions` rather than `cfg(test)` so downstream guard tests
//! (such as `coursecraft-ui-egui`) can observe clone counts while release builds
//! compile the counter out completely.

use std::cell::Cell;

thread_local! {
    static FULL_COURSE_CLONES: Cell<u64> = const { Cell::new(0) };
}

/// Record one full-course clone on the current thread.
pub fn note_full_course_clone() {
    FULL_COURSE_CLONES.set(FULL_COURSE_CLONES.get().saturating_add(1));
}

/// Number of full-course clones recorded on the current thread since the last [`reset`].
#[must_use]
pub fn full_course_clones() -> u64 {
    FULL_COURSE_CLONES.get()
}

/// Reset the current thread's full-course clone counter to zero.
pub fn reset() {
    FULL_COURSE_CLONES.set(0);
}
