//! Embed the craft-i18n interface catalogs named by `CRAFT_I18N_DIR`.
//!
//! The catalogs are an optional build input, never a Cargo dependency
//! (docs/ui/localization.md). Unset, the generated list is empty and the editor is
//! English only; that is how CI and contributors build. Releases also set
//! `CRAFT_I18N_REQUIRED=1`, so a missing checkout fails the build instead of
//! shipping English only.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo
)]

use std::env;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// This app's catalog name: `locales/<locale>/coursecraft.tsv`.
const APP: &str = "coursecraft";

fn main() {
    println!("cargo::rerun-if-env-changed=CRAFT_I18N_DIR");
    println!("cargo::rerun-if-env-changed=CRAFT_I18N_REQUIRED");
    let required = env::var("CRAFT_I18N_REQUIRED").is_ok_and(|v| v.trim() == "1");
    let entries = match env::var_os("CRAFT_I18N_DIR").filter(|d| !d.is_empty()) {
        None => {
            if required {
                println!("cargo::error=CRAFT_I18N_REQUIRED=1 but CRAFT_I18N_DIR is not set");
            }
            Vec::new()
        }
        Some(dir) => match catalogs(&PathBuf::from(dir)) {
            Ok(entries) => entries,
            Err(problem) => {
                let level = if required { "error" } else { "warning" };
                println!("cargo::{level}=craft-i18n: {problem}; the editor is English only");
                Vec::new()
            }
        },
    };
    write_source(&entries);
}

/// One locale's catalogs: (locale, shared catalog path, app catalog path).
type Entry = (String, Option<PathBuf>, PathBuf);

/// The locales of a craft-i18n checkout that have CourseCraft's own catalog.
fn catalogs(dir: &Path) -> Result<Vec<Entry>, String> {
    if !dir.is_absolute() {
        return Err(format!(
            "CRAFT_I18N_DIR must be an absolute path, got {}",
            dir.display()
        ));
    }
    let locales = dir.join("locales");
    let read = std::fs::read_dir(&locales)
        .map_err(|e| format!("{} is not a craft-i18n checkout: {e}", locales.display()))?;
    println!("cargo::rerun-if-changed={}", locales.display());
    let mut entries = Vec::new();
    for item in read.flatten() {
        let path = item.path();
        let app = path.join(format!("{APP}.tsv"));
        if !app.is_file() {
            // A locale with only shared strings is left out: never offer a language
            // the editor has barely been translated into.
            continue;
        }
        let Some(code) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let shared = path.join("common.tsv");
        println!("cargo::rerun-if-changed={}", app.display());
        let shared = shared.is_file().then(|| {
            println!("cargo::rerun-if-changed={}", shared.display());
            shared
        });
        entries.push((code.to_owned(), shared, app));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(entries)
}

/// Write `craft_i18n.rs` into `OUT_DIR`: always, so the crate needs no `cfg`.
fn write_source(entries: &[Entry]) {
    let mut src = String::from(
        "/// (locale, shared catalog, CourseCraft's catalog), one entry per locale; \"\" for a missing file.\n\
         pub static CRAFT_I18N: &craft_i18n::Embedded = &[\n",
    );
    for (code, shared, app) in entries {
        // `{:?}` escapes Windows backslashes; the paths stay as given (no canonicalize).
        let shared = shared
            .as_ref()
            .map_or_else(|| "\"\"".to_owned(), |p| format!("include_str!({p:?})"));
        let _ = writeln!(src, "    ({code:?}, {shared}, include_str!({app:?})),");
    }
    src.push_str("];\n");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap_or_default()).join("craft_i18n.rs");
    if let Err(err) = std::fs::write(&out, src) {
        println!("cargo::error=cannot write {}: {err}", out.display());
    }
}
