//! Test-only work counters for the M21 guard tests (see `guard_tests.rs`).
//!
//! Each counter is a `Cell<u64>` in a thread-local, so this module is compiled
//! only into test builds: no release frame pays for a single increment. The
//! editor is single-threaded and each test gets its own thread, so a test sees
//! only the frames it ran itself.
//!
//! `THUMBNAIL_REDRAWS` counts thumbnails that actually need redrawing, not every
//! paint call. egui repaints the whole panel on every forced harness frame, but
//! a row only needs its content recomposed when it has just come into view or
//! its slide changed. Tracking the last slide and frame per row keeps the idle
//! count at zero, makes a one-object move cost exactly one redraw, and counts
//! the rows that come back into view after a scroll. This is the work step 4's
//! off-screen paint skip is meant to protect.
// Only the debug-only guard tests read these counters.
#![cfg_attr(not(debug_assertions), allow(dead_code))]

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::thread::LocalKey;

use coursecraft_course::{Object, Slide};

thread_local! {
    static COURSE_COPIES: Cell<u64> = const { Cell::new(0) };
    static SPELLER_RUNS: Cell<u64> = const { Cell::new(0) };
    static SUGGESTION_LOOKUPS: Cell<u64> = const { Cell::new(0) };
    static THUMBNAIL_REDRAWS: Cell<u64> = const { Cell::new(0) };
    static STATE_THUMBNAIL_REDRAWS: Cell<u64> = const { Cell::new(0) };
    static VALIDATION_RUNS: Cell<u64> = const { Cell::new(0) };
    static AUDIT_RUNS: Cell<u64> = const { Cell::new(0) };
    static SLIDE_CLONES: Cell<u64> = const { Cell::new(0) };
    static TIMELINE_LIST_REBUILDS: Cell<u64> = const { Cell::new(0) };
    static FRAME: Cell<u64> = const { Cell::new(0) };
    /// Last slide drawn in each row, and the frame it was drawn on.
    static LAST_THUMB_SLIDES: RefCell<HashMap<String, (Slide, u64)>> =
        RefCell::new(HashMap::new());
    /// Last object drawn in each state thumbnail card, and the frame it was drawn on.
    static LAST_STATE_THUMBS: RefCell<HashMap<(String, String), (Object, u64)>> =
        RefCell::new(HashMap::new());
}

fn bump(counter: &'static LocalKey<Cell<u64>>) {
    counter.set(counter.get().saturating_add(1));
}

/// Call once per editor frame, before any thumbnail is painted.
pub(crate) fn begin_frame() {
    FRAME.set(FRAME.get().saturating_add(1));
}

/// Zero the counts before the measured part of a test. The per-row signatures
/// stay, so a slide already drawn while settling is still considered unchanged.
pub(crate) fn reset() {
    COURSE_COPIES.set(0);
    SPELLER_RUNS.set(0);
    SUGGESTION_LOOKUPS.set(0);
    THUMBNAIL_REDRAWS.set(0);
    STATE_THUMBNAIL_REDRAWS.set(0);
    VALIDATION_RUNS.set(0);
    AUDIT_RUNS.set(0);
    SLIDE_CLONES.set(0);
    TIMELINE_LIST_REBUILDS.set(0);
}

/// Forget which rows were painted where, so the next frame counts every row
/// it draws. Used by the scroll guard to measure one viewport in isolation.
pub(crate) fn reset_thumb_signatures() {
    LAST_THUMB_SLIDES.with(|slot| slot.borrow_mut().clear());
    LAST_STATE_THUMBS.with(|slot| slot.borrow_mut().clear());
}

pub(crate) fn note_course_copy() {
    bump(&COURSE_COPIES);
}

pub(crate) fn note_speller_run() {
    bump(&SPELLER_RUNS);
}

pub(crate) fn note_suggestion_lookup() {
    bump(&SUGGESTION_LOOKUPS);
}

pub(crate) fn note_thumbnail_paint(slide_id: &str, slide: &Slide) {
    let frame = FRAME.get();
    LAST_THUMB_SLIDES.with(|slot| {
        let mut last = slot.borrow_mut();
        let redraw = last
            .get(slide_id)
            .is_none_or(|(old, drawn_on)| old != slide || drawn_on.saturating_add(1) != frame);
        // Remember every frame the row is painted, even when it needed no work:
        // otherwise the next frame would look like a fresh arrival again.
        last.insert(slide_id.to_owned(), (slide.clone(), frame));
        if redraw {
            bump(&THUMBNAIL_REDRAWS);
        }
    });
}

pub(crate) fn note_validation_run() {
    bump(&VALIDATION_RUNS);
}

pub(crate) fn note_audit_run() {
    bump(&AUDIT_RUNS);
}

pub(crate) fn note_slide_clone() {
    bump(&SLIDE_CLONES);
}

pub(crate) fn note_timeline_list_rebuild() {
    bump(&TIMELINE_LIST_REBUILDS);
}

pub(crate) fn course_copies() -> u64 {
    COURSE_COPIES.get()
}

pub(crate) fn speller_runs() -> u64 {
    SPELLER_RUNS.get()
}

pub(crate) fn suggestion_lookups() -> u64 {
    SUGGESTION_LOOKUPS.get()
}

pub(crate) fn note_state_thumbnail_paint(obj_id: &str, state_name: &str, obj: &Object) {
    let frame = FRAME.get();
    LAST_STATE_THUMBS.with(|slot| {
        let mut last = slot.borrow_mut();
        let key = (obj_id.to_owned(), state_name.to_owned());
        let redraw = last
            .get(&key)
            .is_none_or(|(old, drawn_on)| old != obj || drawn_on.saturating_add(1) != frame);
        last.insert(key, (obj.clone(), frame));
        if redraw {
            bump(&STATE_THUMBNAIL_REDRAWS);
        }
    });
}

pub(crate) fn state_thumbnail_redraws() -> u64 {
    STATE_THUMBNAIL_REDRAWS.get()
}

pub(crate) fn thumbnail_redraws() -> u64 {
    THUMBNAIL_REDRAWS.get()
}

pub(crate) fn validation_runs() -> u64 {
    VALIDATION_RUNS.get()
}

pub(crate) fn audit_runs() -> u64 {
    AUDIT_RUNS.get()
}

pub(crate) fn slide_clones() -> u64 {
    SLIDE_CLONES.get()
}

pub(crate) fn timeline_list_rebuilds() -> u64 {
    TIMELINE_LIST_REBUILDS.get()
}
