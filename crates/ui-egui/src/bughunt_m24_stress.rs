//! Bug hunt (2026-11-20, M24): a second seeded random walk over healthy-habits,
//! with the M24 keys (F2, Space, Escape, Cmd+Enter, Cmd+B, Delete, arrows,
//! Cmd+Z, Shift+Cmd+Z) and Alt/Shift drags. Fails on a panic, on new validation
//! errors, or when undoing everything does not return the original course.
//!
//! Ignored by default. Run with
//! `cargo test -p coursecraft-ui-egui --lib bughunt_m24_stress -j 4 -- --include-ignored --nocapture`.

use crate::test_harness::TestHarness;
use coursecraft_course::{Course, Severity};
use egui::{Event, Key, Modifiers, Pos2, vec2};
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
#[ignore = "slow: several seconds; run with --include-ignored before a release"]
fn m24_random_walk_with_keys_and_alt_drags() {
    const STEPS: usize = 2000;
    const W: f32 = 1280.0;
    const H: f32 = 800.0;
    let start = healthy();
    let baseline = error_set(&start);
    let mut h = TestHarness::new_with_size(start.clone(), (W, H));
    h.idle_frame();
    let mut rng = Lcg(0x0B16_4D24_1120);
    let keys = [
        Key::F2,
        Key::Space,
        Key::Escape,
        Key::Enter,
        Key::Delete,
        Key::ArrowLeft,
        Key::ArrowRight,
        Key::ArrowUp,
        Key::ArrowDown,
        Key::B,
        Key::I,
        Key::U,
        Key::Z,
        Key::A,
    ];
    let mut undo_steps = 0usize;
    let started = std::time::Instant::now();

    for step in 0..STEPS {
        let roll = rng.below(100);
        let p = Pos2::new(rng.below(W as u64) as f32, rng.below(H as u64) as f32);
        match roll {
            0..=29 => h.click_at(p),
            30..=39 => h.click_at_with(p, Modifiers::SHIFT),
            40..=49 => h.click_at_with(p, Modifiers::COMMAND),
            50..=61 => {
                let d = vec2(rng.below(120) as f32 - 60.0, rng.below(80) as f32 - 40.0);
                let m = match rng.below(4) {
                    0 => Modifiers::ALT,
                    1 => Modifiers::SHIFT,
                    _ => Modifiers::NONE,
                };
                h.drag_by(p, d, 3, m);
            }
            62..=69 => h.right_click_at(p),
            70..=79 => {
                h.frame(vec![Event::PointerMoved(p)]);
                h.click_at(p);
                h.click_at(p);
            }
            80..=91 => {
                let k = keys[rng.below(keys.len() as u64) as usize];
                let m = match rng.below(4) {
                    0 => Modifiers::COMMAND,
                    1 => Modifiers::SHIFT | Modifiers::COMMAND,
                    _ => Modifiers::NONE,
                };
                h.press(k, m);
            }
            92..=95 => {
                let before = h.editor.session.undo_len();
                h.editor.run("edit.undo", serde_json::json!({}));
                if h.editor.session.undo_len() < before {
                    undo_steps += 1;
                }
            }
            _ => h.type_text("x"),
        }
        if step % 50 == 0 {
            h.idle_frame();
        }
        // Errors must never gain new entries: the editor may refuse an edit,
        // but it must not store a course that fails validation.
        let now = error_set(&h.editor.session.course);
        let new: Vec<_> = now.difference(&baseline).collect();
        assert!(new.is_empty(), "step {step}: new validation errors {new:?}");
    }

    let elapsed = started.elapsed();
    eprintln!("m24 stress: {STEPS} steps in {elapsed:?}, {undo_steps} undos applied");

    // Undo everything: the course must return to where the walk started.
    let mut guard = 0;
    while h.editor.session.undo_len() > 0 && guard < 100_000 {
        h.editor.run("edit.undo", serde_json::json!({}));
        guard += 1;
    }
    assert_eq!(
        h.editor.session.course.to_json().expect("json"),
        start.to_json().expect("json"),
        "undoing every step must return to the original course"
    );
}
