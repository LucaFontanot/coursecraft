//! Engine undo/redo performance benchmarks (`PF-01`, `PF-02`, `PF-03`) and
//! debug full-course-clone counter checks.
//!
//! Benchmarks are `#[ignore]`d by default because timings only have meaning in
//! release builds. Run them with:
//! `cargo test --release -p coursecraft-engine undo_perf_tests -- --ignored --nocapture --test-threads=1`

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::{Duration, Instant};

use coursecraft_course::Course;
use serde_json::json;

use crate::{MAX_HISTORY, Session};

const SLIDES: usize = 300;
const SCENES: usize = 10;
const SLIDES_PER_SCENE: usize = SLIDES / SCENES;

/// Deterministic 300-slide course (10 scenes × 30 slides, 6 objects/slide:
/// 3 text boxes, 1 image, 1 shape, 1 button), matching `crates/ui-egui/src/perf_tests.rs::big_course()`.
pub(crate) fn perf300() -> Course {
    let mut scenes = Vec::with_capacity(SCENES);
    for scene in 0..SCENES {
        let mut slides = Vec::with_capacity(SLIDES_PER_SCENE);
        for n in scene * SLIDES_PER_SCENE..(scene + 1) * SLIDES_PER_SCENE {
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
        r#"{{"schema_version":1,"id":"perf300","title":"Perf 300","slide_size":{{"w":1280,"h":720}},"scenes":[{}]}}"#,
        scenes.join(",")
    );
    Course::from_json(&json).expect("generated perf300 course parses")
}

/// Deterministic rich 300-slide course (10 scenes × 30 slides, 15 stateful
/// shape/text objects per slide, each with `hover` and `visited` states).
fn rich300() -> Course {
    let mut scenes = Vec::with_capacity(SCENES);
    for scene in 0..SCENES {
        let mut slides = Vec::with_capacity(SLIDES_PER_SCENE);
        for n in scene * SLIDES_PER_SCENE..(scene + 1) * SLIDES_PER_SCENE {
            let mut objects = Vec::with_capacity(15);
            for k in 0..15 {
                let x = 40 + (k % 5) * 240;
                let y = 40 + (k / 5) * 210;
                let kind = if k % 2 == 0 {
                    r##"{"type":"shape","shape":"rounded_rectangle","fill":"#2E8B57"}"##.to_string()
                } else {
                    format!(
                        r#"{{"type":"text","text":"Slide {n} card {k}: rich authoring content with states.","style":{{"size":18}}}}"#
                    )
                };
                objects.push(format!(
                    r##"{{"id":"o{n}_{k}","kind":{kind},"frame":{{"x":{x},"y":{y},"w":210,"h":180}},"states":[{{"name":"hover","fill":"#3B9E68","text":"Hover {k}"}},{{"name":"visited","fill":"#1D5C3A","text":"Visited {k}"}}]}}"##
                ));
            }
            slides.push(format!(
                r#"{{"id":"s-{n}","title":"Slide {n}","duration_ms":3000,"objects":[{}]}}"#,
                objects.join(",")
            ));
        }
        scenes.push(format!(
            r#"{{"id":"sc{scene}","title":"Scene {scene}","slides":[{}]}}"#,
            slides.join(",")
        ));
    }
    let json = format!(
        r#"{{"schema_version":1,"id":"rich300","title":"Rich 300","slide_size":{{"w":1280,"h":720}},"scenes":[{}]}}"#,
        scenes.join(",")
    );
    Course::from_json(&json).expect("generated rich300 course parses")
}

/// Interpolated q-th percentile of a sorted slice.
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

/// Current resident memory in MB (`0.0` off unix where `ps` is unavailable).
fn rss_mb() -> f64 {
    let pid = std::process::id().to_string();
    std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &pid])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<f64>().ok())
        .map_or(0.0, |kb| kb / 1024.0)
}

fn report_stage(label: &str, samples: &[Duration], rss_before_mb: f64) {
    let mut ms: Vec<f64> = samples.iter().map(|d| d.as_secs_f64() * 1000.0).collect();
    ms.sort_by(f64::total_cmp);
    let median = percentile(&ms, 0.5);
    let p95 = percentile(&ms, 0.95);
    let total_ms: f64 = ms.iter().sum();
    let rss_now = rss_mb();
    let rss_delta = rss_now - rss_before_mb;
    println!(
        "| {label:<34} | total {total_ms:>7.2} ms | median {median:>7.4} ms | p95 {p95:>7.4} ms | rss {rss_now:>6.1} MB ({rss_delta:+6.1} MB)"
    );
}

/// PF-01: 200 edits, 200 undos, 200 redos, and 100 post-cap evictions on `perf300`.
#[test]
#[ignore = "release-only undo performance benchmark (PF-01)"]
fn pf_01_perf300_200_edits_undo_redo_and_eviction() {
    crate::with_scope_verifier_disabled(|| {
        #[cfg(debug_assertions)]
        crate::test_counts::reset();

        let mut session = Session::new(perf300());
        let initial_json = session.course.to_json().expect("initial perf300 JSON");
        let rss_start = rss_mb();

        let mut edit_times = Vec::with_capacity(MAX_HISTORY);
        for i in 0..MAX_HISTORY {
            let params = json!({
                "slide": "s-150",
                "object": "h150",
                "frame": { "x": 60.0 + (i + 1) as f64, "y": 520.0, "w": 200.0, "h": 80.0 }
            });
            let start = Instant::now();
            session
                .execute("object.set_frame", &params)
                .expect("set_frame edit succeeds");
            edit_times.push(start.elapsed());
        }
        report_stage("PF-01 perf300 200 edits", &edit_times, rss_start);
        assert_eq!(session.undo_len(), MAX_HISTORY);
        let est_bytes = session.undo_estimated_bytes();
        println!(
            "| {:<34} | {} bytes ({:.1} KB)",
            "PF-01 perf300 undo_estimated_bytes",
            est_bytes,
            est_bytes as f64 / 1024.0
        );
        assert!(
            est_bytes <= 512 * 1024,
            "PF-01 undo_estimated_bytes should be <= 512 KiB, got {est_bytes} bytes"
        );
        let after_200_json = session.course.to_json().expect("after-200 perf300 JSON");

        let mut undo_times = Vec::with_capacity(MAX_HISTORY);
        for _ in 0..MAX_HISTORY {
            let start = Instant::now();
            session.undo().expect("undo succeeds");
            undo_times.push(start.elapsed());
        }
        report_stage("PF-01 perf300 200 undos", &undo_times, rss_start);
        assert_eq!(
            session.course.to_json().expect("undone perf300 JSON"),
            initial_json,
            "200 undos must restore initial perf300 JSON byte-identically"
        );

        let mut redo_times = Vec::with_capacity(MAX_HISTORY);
        for _ in 0..MAX_HISTORY {
            let start = Instant::now();
            session.redo().expect("redo succeeds");
            redo_times.push(start.elapsed());
        }
        report_stage("PF-01 perf300 200 redos", &redo_times, rss_start);
        assert_eq!(
            session.course.to_json().expect("redone perf300 JSON"),
            after_200_json,
            "200 redos must restore post-edit perf300 JSON byte-identically"
        );

        let mut evict_times = Vec::with_capacity(100);
        for i in 0..100 {
            let params = json!({
                "slide": "s-150",
                "object": "h150",
                "frame": { "x": 300.0 + (i + 1) as f64, "y": 520.0, "w": 200.0, "h": 80.0 }
            });
            let start = Instant::now();
            session
                .execute("object.set_frame", &params)
                .expect("eviction set_frame edit succeeds");
            evict_times.push(start.elapsed());
        }
        report_stage("PF-01 perf300 100 evictions", &evict_times, rss_start);
        assert!(session.history_truncated());
        assert_eq!(session.undo_len(), MAX_HISTORY);
        let est_bytes_after_evict = session.undo_estimated_bytes();
        assert!(
            est_bytes_after_evict <= 512 * 1024,
            "PF-01 undo_estimated_bytes after eviction should be <= 512 KiB, got {est_bytes_after_evict} bytes"
        );
        #[cfg(debug_assertions)]
        assert_eq!(
            crate::test_counts::full_course_clones(),
            0,
            "PF-01 single-slide edits must perform zero full-course clones"
        );
    });
}

/// PF-02: 200 edits, 200 undos, and 200 redos on `rich300`.
#[test]
#[ignore = "release-only undo performance benchmark (PF-02)"]
fn pf_02_rich300_200_edits_undo_redo() {
    crate::with_scope_verifier_disabled(|| {
        #[cfg(debug_assertions)]
        crate::test_counts::reset();

        let mut session = Session::new(rich300());
        let initial_json = session.course.to_json().expect("initial rich300 JSON");
        let rss_start = rss_mb();

        let mut edit_times = Vec::with_capacity(MAX_HISTORY);
        for i in 0..MAX_HISTORY {
            let params = json!({
                "slide": "s-150",
                "object": "o150_0",
                "frame": { "x": 40.0 + (i + 1) as f64, "y": 40.0, "w": 210.0, "h": 180.0 }
            });
            let start = Instant::now();
            session
                .execute("object.set_frame", &params)
                .expect("rich300 set_frame edit succeeds");
            edit_times.push(start.elapsed());
        }
        report_stage("PF-02 rich300 200 edits", &edit_times, rss_start);
        assert_eq!(session.undo_len(), MAX_HISTORY);
        let est_bytes = session.undo_estimated_bytes();
        println!(
            "| {:<34} | {} bytes ({:.1} KB)",
            "PF-02 rich300 undo_estimated_bytes",
            est_bytes,
            est_bytes as f64 / 1024.0
        );
        assert!(
            est_bytes <= 2 * 1024 * 1024,
            "PF-02 undo_estimated_bytes should be <= 2 MiB, got {est_bytes} bytes"
        );
        let after_200_json = session.course.to_json().expect("after-200 rich300 JSON");

        let mut undo_times = Vec::with_capacity(MAX_HISTORY);
        for _ in 0..MAX_HISTORY {
            let start = Instant::now();
            session.undo().expect("undo succeeds");
            undo_times.push(start.elapsed());
        }
        report_stage("PF-02 rich300 200 undos", &undo_times, rss_start);
        assert_eq!(
            session.course.to_json().expect("undone rich300 JSON"),
            initial_json,
            "200 undos must restore initial rich300 JSON byte-identically"
        );

        let mut redo_times = Vec::with_capacity(MAX_HISTORY);
        for _ in 0..MAX_HISTORY {
            let start = Instant::now();
            session.redo().expect("redo succeeds");
            redo_times.push(start.elapsed());
        }
        report_stage("PF-02 rich300 200 redos", &redo_times, rss_start);
        assert_eq!(
            session.course.to_json().expect("redone rich300 JSON"),
            after_200_json,
            "200 redos must restore post-edit rich300 JSON byte-identically"
        );
        #[cfg(debug_assertions)]
        assert_eq!(
            crate::test_counts::full_course_clones(),
            0,
            "PF-02 single-slide edits must perform zero full-course clones"
        );
    });
}

/// PF-03: 200 identical no-op `object.set_frame` edits on `perf300`.
#[test]
#[ignore = "release-only undo performance benchmark (PF-03)"]
fn pf_03_perf300_200_noop_edits() {
    crate::with_scope_verifier_disabled(|| {
        #[cfg(debug_assertions)]
        crate::test_counts::reset();

        let mut session = Session::new(perf300());
        let rss_start = rss_mb();
        let params = json!({
            "slide": "s-150",
            "object": "h150",
            "frame": { "x": 60.0, "y": 520.0, "w": 200.0, "h": 80.0 }
        });

        let mut noop_times = Vec::with_capacity(MAX_HISTORY);
        for _ in 0..MAX_HISTORY {
            let start = Instant::now();
            session
                .execute("object.set_frame", &params)
                .expect("no-op set_frame succeeds");
            noop_times.push(start.elapsed());
        }
        report_stage("PF-03 perf300 200 no-ops", &noop_times, rss_start);
        let est_bytes = session.undo_estimated_bytes();
        println!(
            "| {:<34} | {} bytes ({:.1} KB)",
            "PF-03 perf300 undo_estimated_bytes",
            est_bytes,
            est_bytes as f64 / 1024.0
        );
        assert_eq!(session.undo_len(), 0);
        assert_eq!(est_bytes, 0);
        #[cfg(debug_assertions)]
        assert_eq!(
            crate::test_counts::full_course_clones(),
            0,
            "PF-03 no-op single-slide edits must perform zero full-course clones"
        );
    });
}

#[cfg(debug_assertions)]
#[test]
fn test_counts_tracks_full_course_clones() {
    crate::test_counts::reset();
    let mut session = Session::blank("Counter Test");

    session
        .execute("course.info", &json!({}))
        .expect("course.info succeeds");
    assert_eq!(crate::test_counts::full_course_clones(), 0);

    session
        .execute("course.set_title", &json!({ "title": "Renamed" }))
        .expect("course.set_title succeeds");
    assert_eq!(crate::test_counts::full_course_clones(), 0);

    session
        .execute("slide.add", &json!({ "title": "Slide 2" }))
        .expect("slide.add succeeds");
    assert_eq!(crate::test_counts::full_course_clones(), 1);

    crate::test_counts::reset();
    assert_eq!(crate::test_counts::full_course_clones(), 0);
}
