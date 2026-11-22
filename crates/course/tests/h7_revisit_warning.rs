use coursecraft_course::{Course, RevisitMode, Severity};

fn course_with_revisit(revisit: RevisitMode, triggers_json: &str) -> Course {
    let revisit_str = match revisit {
        RevisitMode::Auto => "auto",
        RevisitMode::Reset => "reset",
        RevisitMode::Resume => "resume",
    };
    let json = format!(
        r##"{{
          "schema_version": 1,
          "id": "revisit-warn",
          "title": "Revisit Warn",
          "scenes": [{{
            "id": "sc1",
            "title": "Scene 1",
            "slides": [{{
              "id": "s1",
              "title": "Video Slide",
              "duration_ms": 5000,
              "revisit": "{revisit_str}",
              "objects": [
                {{
                  "id": "vid",
                  "kind": {{
                    "type": "video",
                    "src": "clip.mp4",
                    "cues": [{{ "id": "c1", "at_ms": 500, "name": "Cue 1" }}]
                  }},
                  "frame": {{ "x": 40, "y": 40, "w": 400, "h": 225 }}
                }},
                {{
                  "id": "row1",
                  "kind": {{ "type": "shape", "shape": "rectangle", "fill": "#FFFFFF" }},
                  "frame": {{ "x": 500, "y": 40, "w": 200, "h": 60 }},
                  "states": [{{ "name": "done", "fill": "#00FF00" }}]
                }}
              ],
              "triggers": {triggers_json}
            }}]
          }}]
        }}"##
    );
    Course::from_json(&json).expect("valid course json")
}

#[test]
fn auto_revisit_with_timeline_or_cue_state_change_warns() {
    let course = course_with_revisit(
        RevisitMode::Auto,
        r#"[{
            "when": { "event": "cue_reached", "object": "vid", "cue": "c1" },
            "action": { "action": "change_state", "object": "row1", "state": "done" }
        }]"#,
    );
    let issues = course.validate();
    let warn = issues
        .iter()
        .find(|i| i.severity == Severity::Warning && i.path == "/scenes/0/slides/0/revisit")
        .expect("expected revisit warning");
    assert!(
        warn.message.contains("Video Slide")
            && warn
                .message
                .contains("changes object states from its timeline or media"),
        "unexpected message: {}",
        warn.message
    );
}

#[test]
fn reset_or_resume_or_click_only_does_not_warn() {
    let reset = course_with_revisit(
        RevisitMode::Reset,
        r#"[{
            "when": { "event": "cue_reached", "object": "vid", "cue": "c1" },
            "action": { "action": "change_state", "object": "row1", "state": "done" }
        }]"#,
    );
    assert!(
        !reset
            .validate()
            .iter()
            .any(|i| i.path.ends_with("/revisit")),
        "reset must not warn"
    );

    let resume = course_with_revisit(
        RevisitMode::Resume,
        r#"[{
            "when": { "event": "timeline_reaches", "ms": 500 },
            "action": { "action": "change_state", "object": "row1", "state": "done" }
        }]"#,
    );
    assert!(
        !resume
            .validate()
            .iter()
            .any(|i| i.path.ends_with("/revisit")),
        "resume must not warn"
    );

    let click_only = course_with_revisit(
        RevisitMode::Auto,
        r#"[{
            "when": { "event": "click", "object": "row1" },
            "action": { "action": "change_state", "object": "row1", "state": "done" }
        }]"#,
    );
    assert!(
        !click_only
            .validate()
            .iter()
            .any(|i| i.path.ends_with("/revisit")),
        "click-only state change must not warn"
    );
}
