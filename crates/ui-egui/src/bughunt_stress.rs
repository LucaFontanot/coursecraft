//! Bug hunt (2026-11-20): a seeded random walk of clicks, drags, keys and undos
//! over the healthy-habits editor. Fails on a panic, on new validation errors, or
//! when undoing everything does not return the original course.
//!
//! Ignored by default: it runs thousands of editor frames. Run it with
//! `cargo test -p coursecraft-ui-egui --lib bughunt_stress -j 4 -- --include-ignored`.

use crate::test_harness::TestHarness;
use coursecraft_course::{Course, Severity};
use egui::{Key, Modifiers, Pos2, Vec2};
use std::collections::BTreeSet;

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
    fn point(&mut self, w: f32, h: f32) -> Pos2 {
        Pos2::new(self.below(w as u64) as f32, self.below(h as u64) as f32)
    }
}

fn healthy() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/healthy-habits/course.json"
    ))
    .expect("healthy habits");
    Course::from_json(&json).expect("valid course")
}

fn error_set(course: &Course) -> BTreeSet<String> {
    course
        .validate()
        .into_iter()
        .filter(|i| i.severity == Severity::Error)
        .map(|i| format!("{} {}", i.path, i.message))
        .collect()
}

#[test]
#[ignore = "slow (about 75 s): run with --ignored before a release"]
fn random_walk_over_healthy_habits_keeps_the_editor_sane() {
    const STEPS: usize = 2000;
    const W: f32 = 1280.0;
    const H: f32 = 800.0;
    let start = healthy();
    let baseline_errors = error_set(&start);
    let mut h = TestHarness::new_with_size(start.clone(), (W, H));
    h.idle_frame();
    let mut rng = Lcg(0x5EED_2026_1120);
    let mut mutations = 0usize;
    let mut panics = 0usize;
    let mut first_panic: Option<String> = None;
    let mut max_history = 0usize;
    let slide_ids: Vec<String> = start.slides().map(|s| s.id.clone()).collect();

    for step in 0..STEPS {
        let before_len = h.editor.undo_len_for_test();
        let action = rng.below(20);
        if std::env::var("BUGHUNT_TRACE").is_ok() {
            eprintln!(
                "TRACE step={step} action={action} slide={} object={:?} tab={:?}",
                h.editor.selection.slide, h.editor.selection.object, h.editor.ui.inspector_tab
            );
        }
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            match action {
                0..=3 => {
                    let p = rng.point(W, H);
                    h.click_at(p);
                }
                4 => {
                    let p = rng.point(W, H);
                    h.right_click_at(p);
                }
                5..=8 => {
                    let a = rng.point(W, H);
                    let d = Vec2::new(rng.below(200) as f32 - 100.0, rng.below(120) as f32 - 60.0);
                    h.drag_by(a, d, 3, Modifiers::NONE);
                }
                9..=11 => {
                    let keys = [
                        Key::Delete,
                        Key::Escape,
                        Key::ArrowLeft,
                        Key::ArrowRight,
                        Key::ArrowUp,
                        Key::ArrowDown,
                    ];
                    let k = keys[rng.below(keys.len() as u64) as usize];
                    h.press(k, Modifiers::NONE);
                }
                12..=15 => {
                    let keys = [Key::D, Key::C, Key::V, Key::G, Key::A];
                    let k = keys[rng.below(keys.len() as u64) as usize];
                    h.press(k, Modifiers::COMMAND);
                }
                16 => {
                    h.press(Key::Z, Modifiers::COMMAND);
                }
                17 => {
                    h.press(Key::Z, Modifiers::COMMAND | Modifiers::SHIFT);
                }
                18 => {
                    let slide = &slide_ids[rng.below(slide_ids.len() as u64) as usize];
                    h.editor.selection.slide = slide.clone();
                    h.editor.selection.clear_objects();
                    h.idle_frame();
                }
                _ => {
                    let p = rng.point(W, H);
                    h.hover_at(p);
                }
            }
            h.idle_frame();
        }));
        if let Err(payload) = outcome {
            panics += 1;
            let text = payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()))
                .unwrap_or_default();
            first_panic.get_or_insert_with(|| format!("step {step} (action {action}): {text}"));
            // egui's context is unusable after a panic in the middle of a frame, so the walk stops here.
            break;
        }
        let after_len = h.editor.undo_len_for_test();
        if after_len > before_len {
            mutations += 1;
        }
        max_history = max_history.max(after_len);
        if step % 250 == 249 {
            let new_errors: Vec<String> = error_set(&h.editor.session.course)
                .difference(&baseline_errors)
                .cloned()
                .collect();
            assert!(
                new_errors.is_empty(),
                "after {} steps the course has new validation errors: {new_errors:?}",
                step + 1
            );
        }
    }

    assert!(
        panics == 0,
        "the walk panicked (and stopped there): {}",
        first_panic.clone().unwrap_or_default()
    );
    let new_errors: Vec<String> = error_set(&h.editor.session.course)
        .difference(&baseline_errors)
        .cloned()
        .collect();
    assert!(
        new_errors.is_empty(),
        "new validation errors at the end: {new_errors:?}"
    );

    eprintln!(
        "STRESS steps={STEPS} recorded_steps={mutations} max_history={max_history} panics={panics}"
    );

    // Undo everything. If history was trimmed by the cap, the original cannot be
    // reached, and that is reported as its own failure.
    let mut guard = 0;
    while h.editor.undo_len_for_test() > 0 && guard < 5000 {
        h.press(Key::Z, Modifiers::COMMAND);
        guard += 1;
    }
    assert!(
        max_history < coursecraft_engine::MAX_HISTORY,
        "history reached the {} step cap after {mutations} recorded steps, so undo-all cannot return to the original",
        coursecraft_engine::MAX_HISTORY
    );
    assert_eq!(
        h.editor.session.course, start,
        "undo-all did not return to the original course ({mutations} steps recorded)"
    );
}
