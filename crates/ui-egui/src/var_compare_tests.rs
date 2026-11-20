//! Headless UI test for building a variable-to-variable condition in the Trigger Editor.

use crate::test_harness::TestHarness;
use crate::triggers::{ConditionNode, TriggerFormState, describe};
use coursecraft_course::{
    CmpOp, Condition, Course, Event as CourseEvent, Trigger, Value, ValueSource, VariableDef,
};

fn hello_with_scores() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    let mut course = Course::from_json(&json).expect("valid course");
    course.variables.push(VariableDef {
        name: "Score".into(),
        value: Value::Number(0.0),
    });
    course.variables.push(VariableDef {
        name: "Best Score".into(),
        value: Value::Number(10.0),
    });
    course
}

#[test]
fn trigger_editor_builds_variable_to_variable_condition_and_describes_naturally() {
    let course = hello_with_scores();
    let mut harness = TestHarness::new(course);
    harness.editor.selection.slide = "s-intro".into();
    harness.editor.selection.layer = None;
    harness.idle_frame();

    let slide = harness
        .editor
        .session
        .course
        .slide("s-intro")
        .expect("slide")
        .clone();
    let mut form = TriggerFormState::new_for_owner(&slide, None, &harness.editor.session.course);
    form.condition_enabled = true;
    form.condition_root = ConditionNode::Compare {
        variable: "Score".into(),
        op: CmpOp::Eq,
        value: ValueSource::Constant(Value::Number(0.0)),
    };

    harness.editor.ui.trigger_form = Some(form);
    harness.idle_frame();

    // Switch the comparison source combo from "A value" to "Another variable".
    harness.pick_combo("A value", "Another variable");
    harness.idle_frame();

    // Pick "Best Score" in the comparison variable combo (which starts on "tabs_seen").
    harness.pick_combo("tabs_seen", "Best Score");
    harness.idle_frame();

    let editor_form = harness
        .editor
        .ui
        .trigger_form
        .as_ref()
        .expect("trigger editor open");

    let built: Trigger = editor_form
        .build_trigger(&harness.editor.session.course)
        .expect("build");
    assert_eq!(
        built.condition,
        Some(Condition::Compare {
            variable: "Score".into(),
            op: CmpOp::Eq,
            value: ValueSource::Variable {
                variable: "Best Score".into(),
            },
        })
    );

    let sentence = describe(&built, &slide);
    assert!(
        sentence.contains("Score equals Best Score"),
        "expected natural sentence, got: {sentence}"
    );
    // Also check plain-words when Score equals Best Score
    let _ = CourseEvent::TimelineStarts { layer: None };
}
