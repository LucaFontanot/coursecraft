//! Find the fonts installed on this computer (desktop only).
//!
//! Font folders are checked in a fixed order, files are read with `skrifa`
//! (supporting `.ttf`, `.otf`, `.ttc`, and `.otc`), and anything unreadable or
//! oversized is skipped. Font bytes are loaded lazily and cached on first use.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::font::{FontFileStyle, MAX_FONT_BYTES, font_collection_info};

/// How deep below a font folder we walk before stopping.
const MAX_DEPTH: u32 = 6;
/// Maximum number of directory entries examined across all folders.
const MAX_ENTRIES: usize = 10_000;

/// One installed font face on disk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemFontFace {
    pub style: FontFileStyle,
    pub path: PathBuf,
    pub face_index: u32,
    pub embedding_restricted: bool,
}

/// One installed font family and the faces found for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemFontFamily {
    pub name: String,
    /// Localized or alternate family names (e.g. East-Asian names) from the `name` table.
    pub aliases: Vec<String>,
    pub faces: Vec<SystemFontFace>,
    /// True when at least one face restricts font embedding.
    pub embedding_restricted: bool,
}

/// Cached index of installed system fonts with lazy byte loading.
#[derive(Debug, Default)]
pub struct SystemFontIndex {
    families: Vec<SystemFontFamily>,
    /// Lower-cased family name / alias -> index into `families`.
    lookup: HashMap<String, usize>,
    /// Cached file bytes keyed by path, loaded on first use.
    bytes_cache: Mutex<HashMap<PathBuf, Arc<[u8]>>>,
}

/// The font folders to search on this platform, in priority order.
///
/// A folder whose environment variable is not set is simply left out.
#[must_use]
pub fn default_font_folders() -> Vec<PathBuf> {
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

/// True when the path ends in `.ttf`, `.otf`, `.ttc`, or `.otc`, in any case.
fn is_font_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".ttf")
        || lower.ends_with(".otf")
        || lower.ends_with(".ttc")
        || lower.ends_with(".otc")
}

fn common_alias(lower: &str) -> Option<&'static str> {
    match lower {
        "times new roman" => Some("times"),
        "times" => Some("times new roman"),
        "courier new" => Some("courier"),
        "courier" => Some("courier new"),
        _ => None,
    }
}

impl SystemFontIndex {
    /// Scan `folders` and build a cached font name index.
    ///
    /// Unreadable folders and corrupt or oversized files are skipped.
    #[must_use]
    pub fn from_folders(folders: &[PathBuf]) -> Self {
        let mut files = Vec::new();
        let mut entries = 0usize;
        for folder in folders {
            if entries >= MAX_ENTRIES {
                break;
            }
            collect_files(folder, 0, &mut files, &mut entries);
        }

        let mut families: Vec<SystemFontFamily> = Vec::new();
        let mut lookup: HashMap<String, usize> = HashMap::new();

        for path in files {
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            let Ok(faces) = font_collection_info(&bytes) else {
                continue;
            };
            for (face_index, info, aliases) in faces {
                let face = SystemFontFace {
                    style: info.style,
                    path: path.clone(),
                    face_index,
                    embedding_restricted: info.embedding_restricted,
                };
                let key = info.family.to_lowercase();
                let idx = if let Some(&existing_idx) = lookup.get(&key) {
                    let family = &mut families[existing_idx];
                    if !family.faces.iter().any(|f| f.style == info.style) {
                        family.faces.push(face);
                    }
                    family.embedding_restricted |= info.embedding_restricted;
                    for alias in &aliases {
                        if !family.aliases.iter().any(|a| a.eq_ignore_ascii_case(alias)) {
                            family.aliases.push(alias.clone());
                        }
                    }
                    existing_idx
                } else {
                    let existing_idx = families.len();
                    families.push(SystemFontFamily {
                        name: info.family,
                        aliases: aliases.clone(),
                        faces: vec![face],
                        embedding_restricted: info.embedding_restricted,
                    });
                    lookup.insert(key, existing_idx);
                    existing_idx
                };
                for alias in aliases {
                    lookup.entry(alias.to_lowercase()).or_insert(idx);
                }
            }
        }

        families.sort_by_key(|family| family.name.to_lowercase());
        // Rebuild lookup after sorting so indices match `families`.
        lookup.clear();
        for (idx, family) in families.iter().enumerate() {
            lookup.entry(family.name.to_lowercase()).or_insert(idx);
            for alias in &family.aliases {
                lookup.entry(alias.to_lowercase()).or_insert(idx);
            }
        }

        Self {
            families,
            lookup,
            bytes_cache: Mutex::new(HashMap::new()),
        }
    }

    /// Scan the default platform font folders.
    #[must_use]
    pub fn scan_default() -> Self {
        Self::from_folders(&default_font_folders())
    }

    /// All discovered families, sorted by name.
    #[must_use]
    pub fn families(&self) -> &[SystemFontFamily] {
        &self.families
    }

    /// Look up a family by name (trimmed, case-insensitive, checking localized
    /// aliases and common aliases like Times New Roman <-> Times).
    #[must_use]
    pub fn find_family(&self, family: &str) -> Option<&SystemFontFamily> {
        let trimmed = family.trim();
        if trimmed.is_empty() {
            return None;
        }
        let lower = trimmed.to_lowercase();
        if let Some(&idx) = self.lookup.get(&lower) {
            return self.families.get(idx);
        }
        if let Some(alias) = common_alias(&lower)
            && let Some(&idx) = self.lookup.get(alias)
        {
            return self.families.get(idx);
        }
        None
    }

    /// Return true if `family` (or a common alias) is installed.
    #[must_use]
    pub fn has_family(&self, family: &str) -> bool {
        self.find_family(family).is_some()
    }

    /// Resolve a family and style to a face, falling back to `Regular` if the
    /// exact style is not installed.
    #[must_use]
    pub fn resolve_face(
        &self,
        family: &str,
        style: FontFileStyle,
    ) -> Option<(&SystemFontFamily, &SystemFontFace)> {
        let fam = self.find_family(family)?;
        let face = fam
            .faces
            .iter()
            .find(|f| f.style == style)
            .or_else(|| fam.faces.iter().find(|f| f.style == FontFileStyle::Regular))?;
        Some((fam, face))
    }

    /// Load and cache the bytes for `face`. Returns `None` if the file was
    /// removed or exceeds [`MAX_FONT_BYTES`].
    #[must_use]
    pub fn load_face_bytes(&self, face: &SystemFontFace) -> Option<Arc<[u8]>> {
        if let Ok(cache) = self.bytes_cache.lock()
            && let Some(bytes) = cache.get(&face.path)
        {
            return Some(Arc::clone(bytes));
        }
        let meta = std::fs::metadata(&face.path).ok()?;
        if meta.len() > MAX_FONT_BYTES as u64 {
            return None;
        }
        let bytes: Arc<[u8]> = std::fs::read(&face.path).ok()?.into();
        if let Ok(mut cache) = self.bytes_cache.lock() {
            cache.insert(face.path.clone(), Arc::clone(&bytes));
        }
        Some(bytes)
    }
}
