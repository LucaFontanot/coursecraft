//! Interface translations: the language setting, `tl!`, and the pseudo locale
//! that finds strings which skipped the lookup (docs/ui/localization.md › Testing).

use crate::EditorRequest;
use crate::i18n::{self, AUTO, I18N, Lang};
use crate::test_harness::TestHarness;
use coursecraft_engine::Session;

fn harness() -> TestHarness {
    TestHarness::new_with_size(Session::blank("Course").course, (1280.0, 800.0))
}

#[test]
fn settings_resolve_to_a_language() {
    assert_eq!(i18n::resolve("en"), Lang::EN);
    assert_eq!(i18n::resolve("EN"), Lang::EN);
    // An unknown code (a build with fewer languages) shows English.
    assert_eq!(i18n::resolve("xx-unknown"), Lang::EN);
    assert_eq!(i18n::resolve(AUTO), I18N.system_lang());
    assert_eq!(i18n::resolve(""), I18N.system_lang());
    // Debug builds can draw the pseudo locale.
    assert_eq!(i18n::resolve("pseudo"), Lang::PSEUDO);
}

#[test]
fn settings_are_saved_normalized_and_unknown_codes_survive() {
    assert_eq!(i18n::normalize(""), AUTO);
    assert_eq!(i18n::normalize("  "), AUTO);
    assert_eq!(i18n::normalize("PT-BR"), "pt-br");
    assert_eq!(i18n::normalize("xx-unknown"), "xx-unknown");
}

#[test]
fn tl_looks_up_in_the_current_language() {
    i18n::set_current(Lang::EN);
    assert_eq!(tl!("Preferences"), "Preferences");
    i18n::set_current(Lang::PSEUDO);
    assert_eq!(tl!("Preferences"), "⟦Preferences⟧");
    i18n::set_current(Lang::EN);
}

#[test]
fn english_is_always_offered_first() {
    let languages = I18N.languages();
    assert_eq!(languages.first(), Some(&Lang::EN));
    assert!(!languages.contains(&Lang::PSEUDO));
}

#[test]
fn embedded_catalogs_have_no_malformed_rows() {
    // Empty unless CRAFT_I18N_DIR was set for this build.
    assert_eq!(I18N.check_all(), Vec::<String>::new());
}

#[test]
fn preferences_window_draws_every_string_through_tl() {
    let mut h = harness();
    h.editor.set_language("pseudo");
    h.editor.open_preferences();
    h.idle_frame();
    for text in [
        "Preferences",
        "Autosave every",
        "Writing",
        "Check spelling as you type",
        "Appearance",
        "Language",
        "Default slide size for new courses",
    ] {
        assert!(
            h.find_text(&format!("⟦{text}⟧")).is_some(),
            "{text:?} is not drawn through tl!"
        );
        assert!(h.find_text(text).is_none(), "{text:?} is drawn without tl!");
    }
}

#[test]
fn choosing_a_language_saves_it_and_applies_without_restart() {
    let mut h = harness();
    h.editor.open_preferences();
    h.idle_frame();
    let combo = h
        .find_text_last("Automatic (system)")
        .expect("language picker");
    h.click_at(combo);
    h.idle_frame();
    let english = h.find_text_last("English").expect("English in the picker");
    let requests = h.click_at_requests(english);
    assert_eq!(h.editor.language, "en");
    assert!(
        requests
            .iter()
            .any(|r| matches!(r, EditorRequest::SetLanguage(code) if code == "en")),
        "the choice goes to the app to save"
    );
    // The next frame draws in the new language; no restart.
    h.editor.set_language("pseudo");
    assert!(h.find_text("⟦Language⟧").is_some());
}

#[test]
fn tr_16_tracking_ui_draws_every_string_through_tl() {
    let mut h = TestHarness::new_with_size(Session::blank("Course").course, (1280.0, 1200.0));
    h.editor.run(
        "course.set_tracking",
        serde_json::json!({
            "slides_viewed": { "mode": "at_least", "count": 3 },
            "quiz": {}
        }),
    );
    h.editor.set_language("pseudo");

    // 1. Course Settings + Course Inspector
    h.editor.ui.inspector_tab = crate::InspectorTab::Course;
    h.editor.ui.show_course_settings_window = true;
    for _ in 0..3 {
        h.idle_frame();
    }
    if let Some(pos) = h.find_text_last("⟦Tracking⟧") {
        h.scroll_over(pos, -250.0);
        h.idle_frame();
    }

    for text in [
        "Tracking",
        "Report completion to the LMS when any selected option is met.",
        "Slides viewed",
        "All slides (1)",
        "At least",
        "slides",
        "Exceeds course slide count (1); completes when all slides are viewed.",
        "Quiz result",
        "Results slide",
        "Course quiz (all graded questions)",
        "No passing score is set in Quiz settings.",
        "This course has no graded questions or results slide.",
        "Completion triggers are always active",
    ] {
        assert!(
            h.find_text(&format!("⟦{text}⟧")).is_some(),
            "{text:?} is not drawn through tl! in Course Settings / Inspector"
        );
        assert!(
            h.find_text(text).is_none(),
            "{text:?} is drawn without tl! in Course Settings / Inspector"
        );
    }
    assert!(
        h.find_text("⟦Slides viewed (3)⟧, ⟦Quiz result⟧, ⟦Triggers⟧")
            .is_some(),
        "tracking_summary is not drawn through tl! in Course Inspector"
    );
    assert!(
        h.find_text("Slides viewed (3), Quiz result, Triggers")
            .is_none(),
        "tracking_summary is drawn without tl! in Course Inspector"
    );

    // 2. Publish dialog (LMS standard)
    h.editor.ui.show_course_settings_window = false;
    h.editor.open_publish_dialog();
    for _ in 0..3 {
        h.idle_frame();
    }

    for text in [
        "Tracking",
        "Slides viewed",
        "Quiz result",
        "Completion triggers are always active",
    ] {
        assert!(
            h.find_text(&format!("⟦{text}⟧")).is_some(),
            "{text:?} is not drawn through tl! in Publish dialog"
        );
        assert!(
            h.find_text(text).is_none(),
            "{text:?} is drawn without tl! in Publish dialog"
        );
    }

    // 3. Publish dialog (Web standard)
    h.editor.ui.publish.standard = crate::publish::PublishStandard::Web;
    h.idle_frame();
    let web_note = "Web packages do not report completion to an LMS.";
    assert!(
        h.find_text(&format!("⟦{web_note}⟧")).is_some(),
        "{web_note:?} is not drawn through tl! in Publish Web"
    );
    assert!(
        h.find_text(web_note).is_none(),
        "{web_note:?} is drawn without tl! in Publish Web"
    );

    // 4. Passing score note, missing results slide label/warning, and remaining summary variants
    h.editor.ui.publish.open = false;
    h.editor.run(
        "course.set_quiz",
        serde_json::json!({ "passing_percent": 80 }),
    );
    h.editor.run(
        "course.set_tracking",
        serde_json::json!({
            "slides_viewed": { "mode": "all" },
            "quiz": { "slide": "gone" }
        }),
    );
    h.editor.ui.show_course_settings_window = true;
    for _ in 0..3 {
        h.idle_frame();
    }
    if let Some(pos) = h.find_text_last("⟦Tracking⟧") {
        h.scroll_over(pos, -250.0);
        h.idle_frame();
    }
    for text in [
        "Missing slide (gone)",
        "Passing score is 80% in Quiz settings.",
        "Selected results slide no longer exists.",
    ] {
        assert!(
            h.find_text(&format!("⟦{text}⟧")).is_some(),
            "{text:?} is not drawn through tl!"
        );
        assert!(h.find_text(text).is_none(), "{text:?} is drawn without tl!");
    }
    assert!(
        h.find_text("⟦Slides viewed (all)⟧, ⟦Quiz result⟧, ⟦Triggers⟧")
            .is_some(),
        "all-slides summary is not drawn through tl!"
    );

    h.editor.run(
        "course.set_tracking",
        serde_json::json!({ "slides_viewed": null, "quiz": null }),
    );
    h.idle_frame();
    assert!(
        h.find_text("⟦Triggers only⟧").is_some(),
        "Triggers only summary is not drawn through tl!"
    );
    assert!(
        h.find_text("Triggers only").is_none(),
        "Triggers only summary is drawn without tl!"
    );
}
