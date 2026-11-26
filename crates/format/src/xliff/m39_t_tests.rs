//! M39 package T: run highlight and shadow survive an XLIFF round trip.

use coursecraft_course::{Course, ObjectKind};

use super::*;

#[test]
fn xliff_round_trip_keeps_run_highlight_and_shadow() {
    let json = serde_json::json!({
        "schema_version": 1,
        "id": "hl",
        "title": "Highlight",
        "scenes": [{
            "id": "sc",
            "title": "Scene",
            "slides": [{
                "id": "s1",
                "title": "Slide",
                "objects": [{
                    "id": "t1",
                    "kind": {
                        "type": "text",
                        "text": "Hello world",
                        "runs": [
                            { "text": "Hello ", "highlight": "#FFFF00" },
                            { "text": "world", "shadow": true }
                        ]
                    },
                    "frame": { "x": 0.0, "y": 0.0, "w": 200.0, "h": 50.0 }
                }]
            }]
        }]
    });
    let course = Course::from_json(&json.to_string()).expect("valid course");
    let targets: std::collections::BTreeMap<String, String> = translation_units(&course, "fr")
        .into_iter()
        .map(|unit| (unit.id, format!("[fr] {}", unit.source)))
        .collect();
    let xml = fill_xliff(&course, "fr", &targets).expect("fill");
    let doc = parse_xliff(xml.as_bytes()).expect("parse");
    let (translated, report) = apply_xliff(&course, &doc);
    assert!(report.placeholder_mismatch.is_empty());
    let object = translated
        .slide("s1")
        .and_then(|slide| slide.objects.first())
        .expect("object");
    let ObjectKind::Text { runs, .. } = &object.kind else {
        panic!("expected text object");
    };
    assert_eq!(
        runs.iter()
            .find_map(|run| run.highlight.as_ref())
            .map(|c| c.0.as_str()),
        Some("#FFFF00")
    );
    assert!(runs.iter().any(|run| run.shadow == Some(true)));
}
