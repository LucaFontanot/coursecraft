//! Font file inspection: read a family name and style from a TrueType or
//! OpenType file, rejecting hostile input instead of panicking.

use skrifa::MetadataProvider as _;
use skrifa::attribute::Style;
use skrifa::raw::TableProvider as _;
use skrifa::string::StringId;

use crate::MediaError;

/// Maximum permitted size of a course font file (15 MiB).
pub const MAX_FONT_BYTES: usize = 15 * 1024 * 1024;

/// The four faces a course can register per family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontFileStyle {
    Regular,
    Bold,
    Italic,
    BoldItalic,
}

impl FontFileStyle {
    /// A stable lower-case name, used by the editor and tests.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Regular => "regular",
            Self::Bold => "bold",
            Self::Italic => "italic",
            Self::BoldItalic => "bold_italic",
        }
    }
}

/// Family and style read from a font file's `name` table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontInfo {
    /// Family name, from nameID 16 (typographic family) or 1.
    pub family: String,
    /// Subfamily name, from nameID 17 (typographic subfamily) or 2.
    pub subfamily: String,
    /// The face style inferred from the subfamily and the OS/2 attributes.
    pub style: FontFileStyle,
    /// True when the OS/2 table's `fsType` has bit 0x0002 set (restricted
    /// licence embedding). False when there is no OS/2 table.
    pub embedding_restricted: bool,
    /// The licence text from name ID 13, cleaned and capped at 200 characters.
    /// `None` when missing or empty.
    pub licence: Option<String>,
}

/// Maximum number of faces inspected in a single `.ttc`/`.otc` collection.
const MAX_COLLECTION_FACES: u32 = 64;

/// Inspect a TrueType or OpenType font at `face_index`.
///
/// Reads nameID 16/17 when present, falling back to 1/2. The input must be at
/// most [`MAX_FONT_BYTES`].
///
/// # Errors
/// Returns [`MediaError::FontTooLarge`] when the input exceeds the cap and
/// [`MediaError::InvalidFont`] when it does not parse or has no family name.
pub fn font_info_at(bytes: &[u8], face_index: u32) -> Result<FontInfo, MediaError> {
    if bytes.len() > MAX_FONT_BYTES {
        return Err(MediaError::FontTooLarge {
            bytes: bytes.len(),
            max: MAX_FONT_BYTES,
        });
    }
    if bytes.is_empty() {
        return Err(MediaError::InvalidFont("font file is empty".to_string()));
    }

    let font = skrifa::FontRef::from_index(bytes, face_index)
        .map_err(|error| MediaError::InvalidFont(error.to_string()))?;
    extract_font_info(&font)
}

/// Inspect a TrueType or OpenType font (face index 0).
///
/// Reads nameID 16/17 when present, falling back to 1/2. The input must be at
/// most [`MAX_FONT_BYTES`].
///
/// # Errors
/// Returns [`MediaError::FontTooLarge`] when the input exceeds the cap and
/// [`MediaError::InvalidFont`] when it does not parse or has no family name.
pub fn font_info(bytes: &[u8]) -> Result<FontInfo, MediaError> {
    font_info_at(bytes, 0)
}

/// Inspect every face in a single font or `.ttc`/`.otc` collection.
///
/// Returns `(face_index, info, all_family_names)` for each valid face (up to
/// 64 faces). `all_family_names` includes localized family names (e.g.
/// East-Asian names) in addition to `info.family`.
///
/// # Errors
/// Returns [`MediaError::FontTooLarge`] when the input exceeds the cap and
/// [`MediaError::InvalidFont`] when the file does not parse or yields no
/// valid faces.
pub fn font_collection_info(bytes: &[u8]) -> Result<Vec<(u32, FontInfo, Vec<String>)>, MediaError> {
    if bytes.len() > MAX_FONT_BYTES {
        return Err(MediaError::FontTooLarge {
            bytes: bytes.len(),
            max: MAX_FONT_BYTES,
        });
    }
    if bytes.is_empty() {
        return Err(MediaError::InvalidFont("font file is empty".to_string()));
    }

    let file_ref = skrifa::raw::FileRef::new(bytes)
        .map_err(|error| MediaError::InvalidFont(error.to_string()))?;
    let mut out = Vec::new();
    match file_ref {
        skrifa::raw::FileRef::Font(font) => {
            let info = extract_font_info(&font)?;
            let aliases = collect_family_aliases(&font, &info.family);
            out.push((0, info, aliases));
        }
        skrifa::raw::FileRef::Collection(coll) => {
            let count = coll.len().min(MAX_COLLECTION_FACES);
            for idx in 0..count {
                let Ok(font) = coll.get(idx) else {
                    continue;
                };
                if let Ok(info) = extract_font_info(&font) {
                    let aliases = collect_family_aliases(&font, &info.family);
                    out.push((idx, info, aliases));
                }
            }
        }
    }
    if out.is_empty() {
        return Err(MediaError::InvalidFont(
            "font collection has no readable faces".to_string(),
        ));
    }
    Ok(out)
}

fn collect_family_aliases(font: &skrifa::FontRef<'_>, primary: &str) -> Vec<String> {
    let mut aliases = Vec::new();
    for id in [16u16, 1u16] {
        for loc in font.localized_strings(StringId::new(id)) {
            let cleaned = clean_name(&loc.to_string());
            if !cleaned.is_empty()
                && !cleaned.eq_ignore_ascii_case(primary)
                && !aliases
                    .iter()
                    .any(|existing: &String| existing.eq_ignore_ascii_case(&cleaned))
            {
                aliases.push(cleaned);
            }
        }
    }
    aliases
}

fn extract_font_info(font: &skrifa::FontRef<'_>) -> Result<FontInfo, MediaError> {
    // MetadataProvider's string helper prefers English strings and falls back
    // to the first record, which is what a file browser shows.
    let read = |id: u16| -> Option<String> {
        font.localized_strings(StringId::new(id))
            .english_or_first()
            .map(|value| value.to_string())
    };

    let family = read(16)
        .or_else(|| read(1))
        .map(|value| clean_name(&value))
        .unwrap_or_default();
    if family.is_empty() {
        return Err(MediaError::InvalidFont(
            "font has no family name".to_string(),
        ));
    }
    let subfamily = read(17)
        .or_else(|| read(2))
        .map(|value| clean_name(&value))
        .unwrap_or_default();

    // fsType bit 0x0002 is "restricted licence embedding". When another
    // permission bit (0x0004 preview & print, 0x0008 editable) is also set, the
    // most permissive level applies. A font without an OS/2 table is unrestricted.
    let embedding_restricted = font
        .os2()
        .map(|os2| {
            let fs = os2.fs_type();
            (fs & 0x0002 != 0) && (fs & 0x000C == 0)
        })
        .unwrap_or(false);

    let licence = read(13).and_then(|value| {
        let cleaned = clean_text(&value, 200);
        if cleaned.is_empty() {
            None
        } else {
            Some(cleaned)
        }
    });

    let attributes = font.attributes();
    let lower = subfamily.to_ascii_lowercase();
    let bold = lower.contains("bold")
        || lower.contains("black")
        || lower.contains("heavy")
        || lower.contains("semibold")
        || attributes.weight.value() >= 600.0;
    let italic = lower.contains("italic")
        || lower.contains("oblique")
        || !matches!(attributes.style, Style::Normal);

    let style = match (bold, italic) {
        (false, false) => FontFileStyle::Regular,
        (true, false) => FontFileStyle::Bold,
        (false, true) => FontFileStyle::Italic,
        (true, true) => FontFileStyle::BoldItalic,
    };

    Ok(FontInfo {
        family,
        subfamily,
        style,
        embedding_restricted,
        licence,
    })
}

/// Trim, drop control characters, and cap the name to 64 characters.
fn clean_name(value: &str) -> String {
    clean_text(value, 64)
}

/// Drop control characters, trim, and cap to `max_chars` characters.
fn clean_text(value: &str, max_chars: usize) -> String {
    value
        .chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .trim()
        .chars()
        .take(max_chars)
        .collect::<String>()
        .trim()
        .to_string()
}
