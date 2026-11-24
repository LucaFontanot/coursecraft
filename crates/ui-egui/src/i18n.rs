//! Interface translations with craft-i18n (docs/ui/localization.md).
//!
//! The code holds the English; translations come only from the craft-i18n
//! catalogs embedded at build time (`build.rs`, `CRAFT_I18N_DIR`). Every
//! user-facing string goes through [`tl!`](crate::tl) or one of the [`I18N`]
//! lookups. A missing translation shows the English; nothing fails.

use std::cell::Cell;
use std::sync::LazyLock;

pub use craft_i18n::Lang;

include!(concat!(env!("OUT_DIR"), "/craft_i18n.rs"));

/// The languages CourseCraft offers besides English, once their catalogs exist.
/// Latin script, left to right, covered by the bundled Inter font. Right-to-left
/// and other scripts stay fallback-only until the editor supports them.
pub const OFFERED: &[&str] = &["de", "es", "fr", "it", "nl", "pl", "pt-br"];

/// The saved language setting that follows the system language.
pub const AUTO: &str = "auto";

/// The embedded catalogs, built once.
pub static I18N: LazyLock<craft_i18n::I18n> = LazyLock::new(|| {
    let (i18n, problems) = craft_i18n::I18n::new(CRAFT_I18N, OFFERED);
    for problem in &problems {
        log::warn!("translation catalog: {problem}");
    }
    i18n
});

thread_local! {
    /// The language the editor draws in on this thread. The editor sets it at
    /// the start of every frame from the saved setting.
    static CURRENT: Cell<Lang> = const { Cell::new(Lang::EN) };
}

/// The language strings are looked up in on this thread.
#[must_use]
pub fn current() -> Lang {
    CURRENT.with(Cell::get)
}

/// Draw in `lang` on this thread from now on.
pub fn set_current(lang: Lang) {
    CURRENT.with(|c| c.set(lang));
}

/// `template` (English, the catalog key) in the current language, its `{name}` placeholders
/// filled from `args`. For libraries that take a translate function, such as craft-update's
/// prompts, whose English stays when the catalogs have no row for it.
#[must_use]
pub fn translate_with(template: &str, args: &[(&str, &str)]) -> String {
    craft_i18n::fmt(I18N.t(current(), template), args)
}

/// The language for a saved setting: [`AUTO`] (or empty) follows the system,
/// a known code picks that language, anything else is English.
#[must_use]
pub fn resolve(setting: &str) -> Lang {
    let setting = setting.trim();
    if setting.is_empty() || setting.eq_ignore_ascii_case(AUTO) {
        I18N.system_lang()
    } else {
        I18N.from_code(setting).unwrap_or(Lang::EN)
    }
}

/// The saved form of a setting: [`AUTO`] or a lowercase code the build offers.
/// Unknown codes are kept, so a setting made by a build with more languages
/// survives a build with fewer.
#[must_use]
pub fn normalize(setting: &str) -> String {
    let setting = setting.trim();
    if setting.is_empty() {
        AUTO.to_owned()
    } else {
        setting.to_ascii_lowercase()
    }
}

/// `source` in the current language.
///
/// ```ignore
/// ui.label(tl!("Preferences"));
/// ```
#[macro_export]
macro_rules! tl {
    ($source:expr) => {
        $crate::i18n::I18N.t($crate::i18n::current(), $source)
    };
}
