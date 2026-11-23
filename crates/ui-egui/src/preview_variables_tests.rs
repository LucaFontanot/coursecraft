//! Headless tests for the Preview window's live variables panel (F3).

use crate::test_harness::TestHarness;
use coursecraft_course::Course;

fn counter_course() -> Course {
    let json = r#"{
        "schema_version": 1,
        "id": "preview-vars-test",
        "title": "Preview Variables Test",
        "slide_size": { "w": 1280, "h": 720 },
        "variables": [
            { "name": "unreferenced_flag", "value": false },
            { "name": "long_note", "value": "This is a very long text value that should be truncated with an ellipsis" },
            { "name": "clicks", "value": 0 }
        ],
        "scenes": [
            {
                "id": "sc-1",
                "title": "Main",
                "slides": [
                    {
                        "id": "s-1",
                        "title": "Counter",
                        "duration_ms": 5000,
                        "objects": [
                            {
                                "id": "lbl-clicks",
                                "kind": { "type": "text", "text": "Clicks: %clicks%" },
                                "frame": { "x": 80, "y": 80, "w": 400, "h": 60 }
                            },
                            {
                                "id": "btn-add",
                                "kind": { "type": "button", "label": "Add One" },
                                "frame": { "x": 80, "y": 200, "w": 220, "h": 70 }
                            }
                        ],
                        "triggers": [
                            {
                                "when": { "event": "click", "object": "btn-add" },
                                "action": {
                                    "action": "set_variable",
                                    "variable": "clicks",
                                    "op": "add",
                                    "value": 1
                                }
                            }
                        ]
                    }
                ]
            }
        ]
    }"#;
    Course::from_json(json).expect("valid course")
}

#[test]
fn preview_variables_panel_updates_on_click_and_stays_inside_window() {
    let mut harness = TestHarness::new_with_size(counter_course(), (1280.0, 800.0));
    harness.editor.open_preview();
    for _ in 0..3 {
        harness.idle_frame();
    }

    // Toggle the Variables panel with F3.
    harness.press(egui::Key::F3, egui::Modifiers::NONE);
    for _ in 0..3 {
        harness.idle_frame();
    }
    assert!(
        harness
            .editor
            .preview
            .as_ref()
            .is_some_and(|p| p.show_variables),
        "F3 should open the live variables panel"
    );

    let report = harness.layout_report(2);
    assert!(
        report.findings.is_empty(),
        "preview with variables panel had layout findings: {:?}",
        report.findings
    );
    assert!(report.painted("Variables"), "header should be painted");
    assert!(report.painted("clicks"), "variable name should be painted");
    assert!(report.painted("Number"), "variable type should be painted");
    assert!(report.painted("0"), "initial value 0 should be painted");

    // Verify referenced variable (%clicks%) is listed before unreferenced ones.
    let clicks_pos = harness
        .find_text("clicks")
        .expect("clicks label in variables panel");
    let flag_pos = harness
        .find_text("unreferenced_flag")
        .expect("unreferenced_flag label in variables panel");
    assert!(
        clicks_pos.y < flag_pos.y,
        "referenced variable %clicks% (y={}) should appear above unreferenced_flag (y={})",
        clicks_pos.y,
        flag_pos.y
    );

    // Verify long text value is truncated with an ellipsis.
    assert!(
        report.texts.iter().any(|t| t.ends_with('…')),
        "long text value should be truncated with an ellipsis; texts={:?}",
        report.texts
    );
    assert!(
        !report.painted("should be truncated with an ellipsis"),
        "full long text should not be painted inline"
    );

    // Verify the panel's texts are all inside the preview window rect.
    let preview_rect = harness
        .editor
        .preview
        .as_ref()
        .and_then(|p| p.window_rect)
        .expect("preview window rect");
    assert!(
        preview_rect.contains(clicks_pos) && preview_rect.contains(flag_pos),
        "variables panel entries must stay inside preview window {preview_rect:?}"
    );

    // Click the "Add One" button on the slide and check the live value updates to 1.
    let btn_pos = harness
        .find_text_in("Add One", preview_rect)
        .expect("Add One button");
    assert!(
        btn_pos.x < clicks_pos.x,
        "slide button should be to the left of the side variables panel"
    );
    harness.click_at(btn_pos);
    for _ in 0..3 {
        harness.idle_frame();
    }

    let report_after = harness.layout_report(1);
    assert!(
        report_after.painted("1"),
        "live value should update to 1 after clicking Add One"
    );

    // Also verify opening both Variables and Effect Log together keeps layout clean.
    harness.press(egui::Key::F2, egui::Modifiers::NONE);
    let report_both = harness.layout_report(2);
    assert!(
        report_both.findings.is_empty(),
        "preview with both variables and effect log had layout findings: {:?}",
        report_both.findings
    );
    assert!(report_both.painted("Variables"));
    assert!(report_both.painted("Effect log"));
}
