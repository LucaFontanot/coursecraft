#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::strings::{StringTarget, replace_in_target, variable_reference_spans};

#[test]
fn builtin_variable_reference_is_protected_in_find_and_replace() {
    let mut text = "You scored %Results.ScorePercent%! Awarded to %Course.LearnerName%.".to_owned();
    let spans = variable_reference_spans(&text);
    assert_eq!(
        spans.len(),
        2,
        "built-in variables with dots should be recognized as protected spans, got {spans:?}"
    );

    let outcome = replace_in_target(
        StringTarget::Plain(&mut text),
        "Score",
        "Grade",
        false,
        false,
        1024,
    );
    assert_eq!(
        text, "You Graded %Results.ScorePercent%! Awarded to %Course.LearnerName%.",
        "Find & Replace must not mutate inside %Results.ScorePercent%"
    );
    assert_eq!(outcome.replaced, 1);
    assert_eq!(outcome.skipped, 1);
}
