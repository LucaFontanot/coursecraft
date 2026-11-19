//! Editor frame-time measurements on a 300-slide course. Ignored by default:
//! timings only mean something in a release build. Run with
//! `cargo test --release -p coursecraft-ui-egui perf_tests -- --ignored --nocapture`.
//! Budgets: idle frame at most 16 ms, interaction frame at most 33 ms.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::canvas;
use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use egui::{Event, Key, Modifiers, RawInput, pos2};

const SLIDES: usize = 300;
const SUITE: &str = "editor";

/// Where the timing suites append their machine-readable results. Never panics:
/// if the target folder cannot be found or written, the run just has no TSV row.
fn results_path() -> Option<PathBuf> {
    let target = if let Ok(dir) = std::env::var("CARGO_TARGET_DIR") {
        if dir.is_empty() {
            None
        } else {
            Some(PathBuf::from(dir))
        }
    } else {
        None
    }
    .or_else(|| {
        let mut here = Path::new(env!("CARGO_MANIFEST_DIR"));
        loop {
            if here.join("Cargo.toml").is_file() && here.join("target").is_dir() {
                return Some(here.join("target"));
            }
            here = here.parent()?;
        }
    })?;
    let dir = target.join("perf");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("results.tsv"))
}

fn append_tsv(stage: &str, median_ms: f64, p95_ms: f64, budget_ms: f64, note: &str) {
    use std::io::Write;
    let Some(path) = results_path() else {
        return;
    };
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    else {
        return;
    };
    let _ = writeln!(
        file,
        "{SUITE}\t{stage}\t{median_ms:.3}\t{p95_ms:.3}\t{budget_ms}\t{note}"
    );
}

/// Interpolated q-th value of an already sorted slice.
fn percentile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let index = q * (sorted.len() - 1) as f64;
    let lo = index.floor() as usize;
    let hi = index.ceil() as usize;
    if lo == hi {
        sorted[lo]
    } else {
        sorted[lo] + (sorted[hi] - sorted[lo]) * (index - lo as f64)
    }
}

/// Current resident memory in MB (0 off unix, where `ps` is unavailable).
fn rss_mb() -> f64 {
    let pid = std::process::id().to_string();
    std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &pid])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<f64>().ok())
        .map_or(0.0, |k| k / 1024.0)
}

/// The same shape as the big-course speed test (300 slides in 10 scenes). The
/// pictures name files the editor does not have, which stubs the media out.
fn big_course() -> Course {
    let mut scenes = Vec::new();
    for scene in 0..10 {
        let mut slides = Vec::new();
        for n in scene * 30..(scene + 1) * 30 {
            let target = if n + 1 < SLIDES {
                "\"next\"".to_string()
            } else {
                "{\"id\":\"s-0\"}".to_string()
            };
            let extra = if n % 10 == 0 {
                format!(
                    r#","layers":[{{"id":"lay{n}","name":"Hint","objects":[{{"id":"lt{n}","kind":{{"type":"text","text":"A hint, teh."}},"frame":{{"x":300,"y":200,"w":500,"h":100}}}}]}}],
                    "question":{{"kind":"multiple_choice","prompt":"Pick","choices":[{{"id":"a","text":"Wrong"}},{{"id":"b","text":"Right","correct":true}}],"points":10,"attempts":2}}"#
                )
            } else {
                String::new()
            };
            slides.push(format!(
                r##"{{"id":"s-{n}","title":"Slide {n}","duration_ms":3000,"objects":[
                {{"id":"t{n}","kind":{{"type":"text","text":"Slide {n} title","style":{{"size":44,"bold":true}}}},"frame":{{"x":60,"y":40,"w":1100,"h":80}}}},
                {{"id":"a{n}","kind":{{"type":"text","text":"First paragraph of body text."}},"frame":{{"x":60,"y":140,"w":500,"h":160}}}},
                {{"id":"b{n}","kind":{{"type":"text","text":"Second paragraph of body text."}},"frame":{{"x":60,"y":320,"w":500,"h":160}}}},
                {{"id":"p{n}","kind":{{"type":"image","src":"img/p{n}.png","alt":"Test picture"}},"frame":{{"x":600,"y":140,"w":560,"h":340}}}},
                {{"id":"h{n}","kind":{{"type":"shape","shape":"rounded_rectangle","fill":"#2E8B57"}},"frame":{{"x":60,"y":520,"w":200,"h":80}}}},
                {{"id":"btn{n}","kind":{{"type":"button","label":"Next"}},"frame":{{"x":1000,"y":600,"w":200,"h":60}}}}],
                "triggers":[{{"action":{{"action":"jump_to_slide","target":{target}}},"when":{{"event":"click","object":"btn{n}"}}}}]{extra}}}"##
            ));
        }
        scenes.push(format!(
            r#"{{"id":"sc{scene}","title":"Scene {scene}","slides":[{}]}}"#,
            slides.join(",")
        ));
    }
    let json = format!(
        r#"{{"schema_version":1,"id":"big","title":"Big","slide_size":{{"w":1280,"h":720}},"scenes":[{}]}}"#,
        scenes.join(",")
    );
    Course::from_json(&json).expect("the generated big course parses")
}

/// A ~2,000-character paragraph with a few misspellings for the typing stage.
fn paragraph_with_typos() -> String {
    let sentence = "This paragraf describes how the teh team recieves an adress and then reviews the adress again before the recieve deadline, while the quick brown fox jumps over the lazy dog and the editors check every detail for the next release. ";
    let mut text = String::new();
    while text.len() < 2_000 {
        text.push_str(sentence);
    }
    text.truncate(2_000);
    text
}

/// A one-slide course holding the long paragraph on a text object.
fn typing_course(text: &str) -> Course {
    let json = format!(
        r##"{{"schema_version":1,"id":"typing","title":"Typing","slide_size":{{"w":1280,"h":720}},"scenes":[{{"id":"sc0","title":"Scene 0","slides":[{{"id":"s-type","title":"Type","objects":[{{"id":"txt","kind":{{"type":"text","text":"{text}","style":{{"size":18}}}},"frame":{{"x":60,"y":40,"w":1100,"h":600}}}}]}}]}}]}}"##
    );
    Course::from_json(&json).expect("the generated typing course parses")
}

/// One frame with events, timed through layout and tessellation.
fn timed(h: &mut TestHarness, events: Vec<Event>) -> Duration {
    let raw = RawInput {
        screen_rect: Some(h.screen_rect),
        events,
        ..Default::default()
    };
    let t = Instant::now();
    let output = h.ctx.run_ui(raw, |ui| {
        let _ = h.editor.show(ui);
    });
    let _meshes = h
        .ctx
        .tessellate(output.shapes.clone(), output.pixels_per_point);
    let took = t.elapsed();
    output.drop_without_applying_deltas();
    took
}

fn report_note(name: &str, budget_ms: f64, samples: &[Duration], note: &str) -> f64 {
    let mut ms: Vec<f64> = samples.iter().map(|d| d.as_secs_f64() * 1000.0).collect();
    ms.sort_by(f64::total_cmp);
    let median = percentile(&ms, 0.5);
    let p95 = percentile(&ms, 0.95);
    println!(
        "| {name:<32} | median {median:>7.2} ms | p95 {p95:>7.2} ms | budget {budget_ms} ms | {note}"
    );
    append_tsv(name, median, p95, budget_ms, note);
    p95
}

fn report(name: &str, budget_ms: f64, samples: &[Duration]) -> f64 {
    report_note(name, budget_ms, samples, "")
}

/// A stage with a single measurement (open, save, first frame): median and p95
/// are the same number.
fn report_one(name: &str, budget_ms: f64, took: Duration, note: &str) {
    let ms = took.as_secs_f64() * 1000.0;
    println!("| {name:<32} | {ms:>9.2} ms | budget {budget_ms} ms | {note}");
    append_tsv(name, ms, ms, budget_ms, note);
}

#[test]
#[ignore = "timing test: run in release with --ignored --nocapture"]
fn editor_frame_times_on_a_300_slide_course() {
    let mut h = TestHarness::new(big_course());
    h.editor.selection.slide = "s-0".into();
    for _ in 0..5 {
        h.idle_frame();
    }

    let idle: Vec<_> = (0..60).map(|_| timed(&mut h, Vec::new())).collect();
    report("idle", 16.0, &idle);

    let select: Vec<_> = (0..60)
        .map(|i| {
            h.editor.selection.slide = format!("s-{}", 150 + i % 2);
            timed(&mut h, Vec::new())
        })
        .collect();
    report("select slide 150", 33.0, &select);

    let list_pos = h
        .find_text_containing("Slide 1")
        .unwrap_or(pos2(90.0, 400.0));
    h.frame(vec![Event::PointerMoved(list_pos)]);
    let scroll: Vec<_> = (0..60)
        .map(|i| {
            let dy = if i % 2 == 0 { -240.0 } else { 240.0 };
            timed(
                &mut h,
                vec![Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, dy),
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Wheel,
                    modifiers: Modifiers::NONE,
                }],
            )
        })
        .collect();
    report("scroll slide list", 33.0, &scroll);

    // Typing: focus the button label field in Properties, then type a letter a frame.
    h.editor.selection.slide = "s-150".into();
    h.editor.selection.object = Some("btn150".into());
    for _ in 0..3 {
        h.idle_frame();
    }
    let rect = h.editor.ui.properties.btn_label_rect;
    assert!(rect.is_some(), "the button label field should be laid out");
    if let Some(rect) = rect {
        h.click_at(rect.center());
    }
    let typing: Vec<_> = (0..60)
        .map(|_| {
            timed(
                &mut h,
                vec![
                    Event::Text("x".into()),
                    Event::Key {
                        key: Key::X,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: Modifiers::NONE,
                    },
                ],
            )
        })
        .collect();
    report("typing in a text field", 33.0, &typing);
}

#[test]
#[ignore = "timing test: run in release with --ignored --nocapture"]
fn type_on_canvas_with_spell_check() {
    let paragraph = paragraph_with_typos();
    let mut h = TestHarness::new(typing_course(&paragraph));
    h.editor.set_spell_preferences(true, Vec::new());
    h.editor.selection.slide = "s-type".into();
    h.editor.ui.inline_edit = Some(canvas::InlineEdit {
        slide: "s-type".into(),
        object: "txt".into(),
        buffer: paragraph.clone(),
        original: paragraph.clone(),
        is_button: false,
        is_shape: false,
        cell: None,
        just_entered: true,
    });
    // First frame selects all and focuses, the second clears `just_entered`,
    // then one arrow key collapses the selection to the end of the paragraph.
    h.idle_frame();
    h.idle_frame();
    h.press(Key::ArrowRight, Modifiers::NONE);
    let samples: Vec<_> = (0..60)
        .map(|_| timed(&mut h, vec![Event::Text("x".into())]))
        .collect();
    assert!(
        h.editor
            .ui
            .inline_edit
            .as_ref()
            .is_some_and(|edit| edit.buffer.len() > paragraph.len()),
        "typing one letter per frame should grow the in-place buffer"
    );
    report("type on canvas with spell check", 33.0, &samples);
}

#[test]
#[ignore = "timing test: run in release with --ignored --nocapture"]
fn check_spelling_whole_course() {
    let mut h = TestHarness::new(big_course());
    h.editor.selection.slide = "s-0".into();
    let t = Instant::now();
    crate::spell_check::open(&mut h.editor);
    h.idle_frame();
    report_one("check spelling, open", 250.0, t.elapsed(), "");

    let mut steps = Vec::new();
    for _ in 0..20 {
        let index = h.editor.ui.spell_check.index;
        let t = Instant::now();
        crate::spell_check::ignore_issue(&mut h.editor, index, false);
        h.idle_frame();
        steps.push(t.elapsed());
    }
    report_note("check spelling, ignore step", 33.0, &steps, "20 issues");
}

const INTERACTION_KINDS: &[&str] = &[
    "tabs",
    "accordion",
    "flip_cards",
    "hotspot_reveal",
    "process_steps",
    "timeline",
    "reveal_grid",
    "scenario_choice",
];

#[test]
#[ignore = "timing test: run in release with --ignored --nocapture"]
fn insert_interaction_and_undo() {
    let mut h = TestHarness::new(big_course());
    h.editor.selection.slide = "s-150".into();
    h.idle_frame();
    let mut samples = Vec::new();
    for kind in INTERACTION_KINDS {
        let t = Instant::now();
        let inserted = h.editor.run(
            "interaction.insert",
            serde_json::json!({ "slide": "s-150", "kind": kind }),
        );
        assert!(inserted.is_some(), "insert {kind} should succeed");
        let _ = h.editor.run("edit.undo", serde_json::json!({}));
        samples.push(t.elapsed());
    }
    report_note("insert interaction + undo", 100.0, &samples, "8 kinds");
}

#[test]
#[ignore = "timing test: run in release with --ignored --nocapture"]
fn undo_redo_move() {
    let mut h = TestHarness::new(big_course());
    h.editor.selection.slide = "s-150".into();
    h.editor.selection.object = Some("t150".into());
    h.idle_frame();
    let key = |pressed| Event::Key {
        key: Key::ArrowRight,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers: Modifiers::NONE,
    };
    let mut samples = Vec::new();
    for _ in 0..30 {
        let t = Instant::now();
        let _ = timed(&mut h, vec![key(true)]);
        let _ = timed(&mut h, vec![key(false)]);
        let _ = h.editor.run("edit.undo", serde_json::json!({}));
        let _ = h.editor.run("edit.redo", serde_json::json!({}));
        samples.push(t.elapsed());
    }
    let x = h
        .editor
        .session
        .course
        .slide("s-150")
        .and_then(|slide| slide.object("t150"))
        .map(|(object, _)| object.frame.x);
    assert!(x.is_some_and(|x| x > 60.0), "the redone move should stick");
    report_note("undo / redo move", 33.0, &samples, "30 cycles");
}

#[test]
#[ignore = "timing test: run in release with --ignored --nocapture"]
fn visit_every_slide() {
    let mut h = TestHarness::new(big_course());
    let rss_before = rss_mb();
    let mut samples = Vec::new();
    for n in 0..SLIDES {
        h.editor.selection.slide = format!("s-{n}");
        samples.push(timed(&mut h, Vec::new()));
    }
    let rss_after = rss_mb();
    report_note(
        "visit every slide",
        33.0,
        &samples,
        &format!("rss {rss_before:.0} -> {rss_after:.0} MB"),
    );
}

#[test]
#[ignore = "timing test: run in release with --ignored --nocapture"]
fn first_frame() {
    let t = Instant::now();
    let mut h = TestHarness::new(big_course());
    let _ = h.frame(Vec::new());
    report_one("first frame", 500.0, t.elapsed(), "");
}
