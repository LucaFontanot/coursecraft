//! Find the fonts installed on this computer (desktop only).
//!
//! Font folders are checked in a fixed order, files are read with the same
//! `skrifa`-based inspector the course fonts use, and anything unreadable is
//! skipped. This module is never compiled for wasm; the web build answers the
//! font list request with an empty list instead.

use std::path::{Path, PathBuf};

use coursecraft_ui_egui::coursecraft_media::{FontFileStyle, MAX_FONT_BYTES, font_info};

/// How deep below a font folder we walk before stopping.
const MAX_DEPTH: u32 = 5;
/// Maximum number of directory entries examined across all folders.
const MAX_ENTRIES: usize = 5_000;

/// One installed font family and the face files we found for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemFontFamily {
    pub name: String,
    pub faces: Vec<(FontFileStyle, PathBuf)>,
    /// True when at least one face restricts font embedding.
    pub embedding_restricted: bool,
}

/// The font folders to search on this platform, in priority order.
///
/// A folder whose environment variable is not set is simply left out.
#[must_use]
pub fn font_folders() -> Vec<PathBuf> {
    let mut folders = Vec::new();

    if cfg!(target_os = "macos") {
        folders.push(PathBuf::from("/System/Library/Fonts"));
        folders.push(PathBuf::from("/System/Library/Fonts/Supplemental"));
        folders.push(PathBuf::from("/Library/Fonts"));
        if let Some(home) = std::env::var_os("HOME") {
            folders.push(PathBuf::from(home).join("Library/Fonts"));
        }
    } else if cfg!(target_os = "windows") {
        let windir = std::env::var_os("WINDIR").unwrap_or_else(|| "C:\\Windows".into());
        folders.push(PathBuf::from(windir).join("Fonts"));
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            folders.push(PathBuf::from(local).join("Microsoft/Windows/Fonts"));
        }
    } else {
        folders.push(PathBuf::from("/usr/share/fonts"));
        folders.push(PathBuf::from("/usr/local/share/fonts"));
        if let Some(home) = std::env::var_os("HOME") {
            folders.push(PathBuf::from(&home).join(".local/share/fonts"));
            folders.push(PathBuf::from(&home).join(".fonts"));
        }
    }

    folders
}

/// Recursively collect candidate font files, never following symlinks.
fn collect_files(dir: &Path, depth: u32, files: &mut Vec<PathBuf>, entries: &mut usize) {
    if depth > MAX_DEPTH || *entries >= MAX_ENTRIES {
        return;
    }
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return;
    };
    let mut children: Vec<PathBuf> = read_dir
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .collect();
    children.sort();
    for path in children {
        if *entries >= MAX_ENTRIES {
            return;
        }
        *entries += 1;
        let Ok(kind) = std::fs::symlink_metadata(&path).map(|meta| meta.file_type()) else {
            continue;
        };
        if kind.is_dir() {
            collect_files(&path, depth.saturating_add(1), files, entries);
        } else if kind.is_file() && is_font_file(&path) {
            let within_cap = std::fs::metadata(&path)
                .map(|meta| meta.len() <= MAX_FONT_BYTES as u64)
                .unwrap_or(false);
            if within_cap {
                files.push(path);
            }
        }
    }
}

/// True when the path ends in `.ttf` or `.otf`, in any case.
fn is_font_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".ttf") || lower.ends_with(".otf")
}

/// Scan `folders`, group the fonts found by family and sort by family name.
///
/// Unreadable folders and files are skipped. For each family at most one path
/// per face style is kept (the first found).
#[must_use]
pub fn scan(folders: &[PathBuf]) -> Vec<SystemFontFamily> {
    let mut files = Vec::new();
    let mut entries = 0usize;
    for folder in folders {
        if entries >= MAX_ENTRIES {
            break;
        }
        collect_files(folder, 0, &mut files, &mut entries);
    }

    let mut families: Vec<SystemFontFamily> = Vec::new();
    for path in files {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(info) = font_info(&bytes) else {
            continue;
        };
        if let Some(family) = families
            .iter_mut()
            .find(|family| family.name.eq_ignore_ascii_case(&info.family))
        {
            if !family.faces.iter().any(|(style, _)| *style == info.style) {
                family.faces.push((info.style, path));
            }
            family.embedding_restricted |= info.embedding_restricted;
        } else {
            families.push(SystemFontFamily {
                name: info.family,
                faces: vec![(info.style, path)],
                embedding_restricted: info.embedding_restricted,
            });
        }
    }

    families.sort_by_key(|family| family.name.to_lowercase());
    families
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    const INTER_REGULAR: &[u8] = include_bytes!("../../../crates/render/fonts/Inter-Regular.ttf");

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("coursecraft-system-fonts-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create scratch folder");
        dir
    }

    #[test]
    fn scan_finds_one_family_and_skips_junk() {
        let dir = scratch("one-family");
        std::fs::write(dir.join("Inter.ttf"), INTER_REGULAR).expect("write test font");
        std::fs::write(dir.join("junk.ttf"), b"not a font").expect("write junk font");

        let families = scan(std::slice::from_ref(&dir));
        assert_eq!(families.len(), 1, "{families:?}");
        assert_eq!(families[0].name, "Inter");
        assert_eq!(families[0].faces.len(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn scan_of_missing_folder_is_empty() {
        let dir = scratch("missing").join("does-not-exist");
        assert!(scan(&[dir]).is_empty());
    }
}
