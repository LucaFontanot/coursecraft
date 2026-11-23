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
