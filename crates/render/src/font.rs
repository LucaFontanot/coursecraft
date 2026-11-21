//! Bundled font loading, custom course fonts, installed system fonts, and fallback resolution.

use std::collections::HashSet;
use std::sync::Arc;

use coursecraft_course::{Course, FontStyle};
use egui::{FontData, FontDefinitions, FontFamily, FontId};

use crate::media::{MediaFetch, MediaSource};

/// Loaded bytes and collection face index for an installed system font face.
#[derive(Clone, Debug)]
pub struct SystemFontData {
    pub bytes: Arc<[u8]>,
    pub face_index: u32,
}

/// Optional provider of installed system fonts (desktop only; absent on wasm).
pub trait SystemFontProvider: Send + Sync {
    /// Load the font face bytes for `(family, style)` if installed, falling
    /// back to `FontStyle::Regular` when the exact style is unavailable.
    fn load_face(&self, family: &str, style: FontStyle) -> Option<(FontStyle, SystemFontData)>;
}

#[cfg(not(target_arch = "wasm32"))]
fn to_media_style(style: FontStyle) -> coursecraft_media::FontFileStyle {
    match style {
        FontStyle::Regular => coursecraft_media::FontFileStyle::Regular,
        FontStyle::Bold => coursecraft_media::FontFileStyle::Bold,
        FontStyle::Italic => coursecraft_media::FontFileStyle::Italic,
        FontStyle::BoldItalic => coursecraft_media::FontFileStyle::BoldItalic,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn from_media_style(style: coursecraft_media::FontFileStyle) -> FontStyle {
    match style {
        coursecraft_media::FontFileStyle::Regular => FontStyle::Regular,
        coursecraft_media::FontFileStyle::Bold => FontStyle::Bold,
        coursecraft_media::FontFileStyle::Italic => FontStyle::Italic,
        coursecraft_media::FontFileStyle::BoldItalic => FontStyle::BoldItalic,
    }
}

/// [`SystemFontProvider`] backed by [`coursecraft_media::system_fonts::SystemFontIndex`].
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub struct IndexFontProvider(pub Arc<coursecraft_media::system_fonts::SystemFontIndex>);

#[cfg(not(target_arch = "wasm32"))]
impl SystemFontProvider for IndexFontProvider {
    fn load_face(&self, family: &str, style: FontStyle) -> Option<(FontStyle, SystemFontData)> {
        let (_, face) = self.0.resolve_face(family, to_media_style(style))?;
        let bytes = self.0.load_face_bytes(face)?;
        Some((
            from_media_style(face.style),
            SystemFontData {
                bytes,
                face_index: face.face_index,
            },
        ))
    }
}

const INTER_REGULAR: &[u8] = include_bytes!("../fonts/Inter-Regular.ttf");
const INTER_MEDIUM: &[u8] = include_bytes!("../fonts/Inter-Medium.ttf");
const INTER_SEMIBOLD: &[u8] = include_bytes!("../fonts/Inter-SemiBold.ttf");
const INTER_BOLD: &[u8] = include_bytes!("../fonts/Inter-Bold.ttf");
const INTER_ITALIC: &[u8] = include_bytes!("../fonts/Inter-Italic.ttf");
const INTER_BOLD_ITALIC: &[u8] = include_bytes!("../fonts/Inter-BoldItalic.ttf");
const PHOSPHOR_REGULAR: &[u8] = include_bytes!("../fonts/Phosphor.ttf");
const PHOSPHOR_FILL: &[u8] = include_bytes!("../fonts/Phosphor-Fill.ttf");

/// `ctx.data` key holding the fingerprint of the font set currently installed.
const FINGERPRINT_ID: &str = "coursecraft-installed-fonts";
/// `ctx.data` key holding the optional `Arc<dyn SystemFontProvider>`.
const SYSTEM_PROVIDER_ID: &str = "coursecraft-system-font-provider";
/// `ctx.data` key holding `(family_lower, FontStyle)` pairs already looked up
/// via the system font provider, so missing or already-loaded faces are never
/// re-queried every frame.
const TRIED_SYSTEM_FACES_ID: &str = "coursecraft-tried-system-faces";
/// `ctx.data` key holding `(cumulative_pass_nr, FontDefinitions)` queued via
/// `ctx.set_fonts` during the current pass, because egui only applies
/// `set_fonts` at the start of the next pass. Without tracking it here, a
/// second incremental font install in the same frame would clone stale
/// `ctx.fonts()` definitions and clobber the first font.
const PENDING_DEFS_ID: &str = "coursecraft-pending-font-defs";

/// Which file is backing a registered family. `Ready(len)` means the bytes
/// are available; `Missing` and `Pending` both omit the face until it arrives.
#[derive(Clone, Debug, PartialEq, Eq)]
struct FaceFingerprint {
    family: String,
    style: FontStyle,
    src: String,
    status: u8,
    len: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct FontFingerprint {
    faces: Vec<FaceFingerprint>,
}

fn base_definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert(
        "inter_regular".to_owned(),
        Arc::new(FontData::from_static(INTER_REGULAR)),
    );
    fonts.font_data.insert(
        "inter_medium".to_owned(),
        Arc::new(FontData::from_static(INTER_MEDIUM)),
    );
    fonts.font_data.insert(
        "inter_semibold".to_owned(),
        Arc::new(FontData::from_static(INTER_SEMIBOLD)),
    );
    fonts.font_data.insert(
        "inter_bold".to_owned(),
        Arc::new(FontData::from_static(INTER_BOLD)),
    );
    fonts.font_data.insert(
        "inter_italic".to_owned(),
        Arc::new(FontData::from_static(INTER_ITALIC)),
    );
    fonts.font_data.insert(
        "inter_bold_italic".to_owned(),
        Arc::new(FontData::from_static(INTER_BOLD_ITALIC)),
    );
    fonts.font_data.insert(
        "phosphor_regular".to_owned(),
        Arc::new(FontData::from_static(PHOSPHOR_REGULAR)),
    );
    fonts.font_data.insert(
        "phosphor_fill".to_owned(),
        Arc::new(FontData::from_static(PHOSPHOR_FILL)),
    );

    let mut default_proportional = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    // Phosphor icon font is the last fallback of Proportional, so an icon codepoint
    // inside a normal label string will resolve cleanly without manual layout splitting.
    default_proportional.push("phosphor_regular".to_owned());

    let make_family = |primary: &str| -> Vec<String> {
        let mut list = Vec::with_capacity(1 + default_proportional.len());
        list.push(primary.to_owned());
        list.extend(default_proportional.clone());
        list
    };

    fonts.families.insert(
        FontFamily::Name("inter".into()),
        make_family("inter_regular"),
    );
    fonts.families.insert(
        FontFamily::Name("inter-medium".into()),
        make_family("inter_medium"),
    );
    fonts.families.insert(
        FontFamily::Name("inter-semibold".into()),
        make_family("inter_semibold"),
    );
    fonts.families.insert(
        FontFamily::Name("inter-bold".into()),
        make_family("inter_bold"),
    );
    fonts.families.insert(
        FontFamily::Name("inter-italic".into()),
        make_family("inter_italic"),
    );
    fonts.families.insert(
        FontFamily::Name("inter-bold-italic".into()),
        make_family("inter_bold_italic"),
    );
    fonts.families.insert(
        FontFamily::Name("phosphor".into()),
        vec!["phosphor_regular".to_owned()],
    );
    fonts.families.insert(
        FontFamily::Name("phosphor-fill".into()),
        vec!["phosphor_fill".to_owned()],
    );
    // Inter is also the default proportional face, so text set with plain
    // `FontId::proportional` matches the rest of the UI.
    fonts
        .families
        .insert(FontFamily::Proportional, make_family("inter_regular"));

    fonts
}

fn installed_fingerprint(ctx: &egui::Context) -> Option<FontFingerprint> {
    ctx.data(|data| data.get_temp(egui::Id::unique(FINGERPRINT_ID)))
}

fn store_fingerprint(ctx: &egui::Context, fingerprint: FontFingerprint) {
    ctx.data_mut(|data| data.insert_temp(egui::Id::unique(FINGERPRINT_ID), fingerprint));
}

fn pending_defs_for_pass(ctx: &egui::Context) -> Option<FontDefinitions> {
    let pass = ctx.cumulative_pass_nr();
    ctx.data(|data| {
        data.get_temp::<(u64, FontDefinitions)>(egui::Id::unique(PENDING_DEFS_ID))
            .and_then(|(saved_pass, defs)| (saved_pass == pass).then_some(defs))
    })
}

fn set_fonts_and_track(ctx: &egui::Context, defs: FontDefinitions) {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|data| {
        data.insert_temp(egui::Id::unique(PENDING_DEFS_ID), (pass, defs.clone()));
    });
    ctx.set_fonts(defs);
}

fn install_base(ctx: &egui::Context) {
    let fingerprint = FontFingerprint::default();
    if installed_fingerprint(ctx).as_ref() == Some(&fingerprint) {
        return;
    }
    set_fonts_and_track(ctx, base_definitions());
    clear_tried_system_faces(ctx);
    store_fingerprint(ctx, fingerprint);
}

/// Register (or clear) the optional system font provider on `ctx`.
///
/// Only desktop builds supply a provider; the web player leaves it unset.
pub fn set_system_font_provider(
    ctx: &egui::Context,
    provider: Option<Arc<dyn SystemFontProvider>>,
) {
    let id = egui::Id::unique(SYSTEM_PROVIDER_ID);
    let unchanged = ctx.data(|data| {
        let existing = data.get_temp::<Option<Arc<dyn SystemFontProvider>>>(id);
        match (
            existing.as_ref().and_then(|o| o.as_ref()),
            provider.as_ref(),
        ) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => existing.is_some(),
            _ => false,
        }
    });
    if unchanged {
        return;
    }
    ctx.data_mut(|data| data.insert_temp(id, provider));
    clear_tried_system_faces(ctx);
}

fn system_font_provider(ctx: &egui::Context) -> Option<Arc<dyn SystemFontProvider>> {
    ctx.data(|data| {
        data.get_temp::<Option<Arc<dyn SystemFontProvider>>>(egui::Id::unique(SYSTEM_PROVIDER_ID))
            .flatten()
    })
}

fn clear_tried_system_faces(ctx: &egui::Context) {
    ctx.data_mut(|data| {
        data.remove::<HashSet<(String, &'static str)>>(egui::Id::unique(TRIED_SYSTEM_FACES_ID));
    });
}

fn was_system_face_tried(ctx: &egui::Context, key: &(String, &'static str)) -> bool {
    ctx.data(|data| {
        data.get_temp::<HashSet<(String, &'static str)>>(egui::Id::unique(TRIED_SYSTEM_FACES_ID))
            .is_some_and(|set| set.contains(key))
    })
}

fn mark_system_face_tried(ctx: &egui::Context, key: (String, &'static str)) {
    ctx.data_mut(|data| {
        let id = egui::Id::unique(TRIED_SYSTEM_FACES_ID);
        let mut set = data
            .get_temp::<HashSet<(String, &'static str)>>(id)
            .unwrap_or_default();
        set.insert(key);
        data.insert_temp(id, set);
    });
}

/// Install the bundled Inter and Phosphor font families into an [`egui::Context`].
///
/// Calling this again never resets the font atlas, and it leaves any custom
/// course faces installed by [`install_course_fonts`] in place.
pub fn install_fonts(ctx: &egui::Context) {
    if installed_fingerprint(ctx).is_some() {
        return;
    }
    install_base(ctx);
}

/// A stable egui family name for one course face.
#[must_use]
pub fn system_face_family(family: &str, style: FontStyle) -> FontFamily {
    FontFamily::Name(
        format!(
            "coursecraft-sysfont:{}:{}",
            family.trim().to_ascii_lowercase(),
            style.as_str()
        )
        .into(),
    )
}
#[must_use]
pub fn course_face_family(family: &str, style: FontStyle) -> FontFamily {
    FontFamily::Name(
        format!(
            "coursecraft-font:{}:{}",
            family.trim().to_ascii_lowercase(),
            style.as_str()
        )
        .into(),
    )
}

fn fingerprint_for(course: &Course, media: &dyn MediaSource) -> FontFingerprint {
    let mut faces = Vec::with_capacity(course.fonts.len());
    for face in &course.fonts {
        let (status, len) = match media.get(&face.src) {
            MediaFetch::Ready(bytes) => (1u8, bytes.len()),
            MediaFetch::Pending => (2, 0),
            MediaFetch::Missing => (3, 0),
        };
        faces.push(FaceFingerprint {
            family: face.family.clone(),
            style: face.style,
            src: face.src.clone(),
            status,
            len,
        });
    }
    FontFingerprint { faces }
}

/// Register every course font face and one egui family per (family, style),
/// with Inter after it as the glyph fallback.
///
/// Safe to call every frame: the font atlas is only reset when the set of
/// faces, their sources or their loaded sizes change.
pub fn install_course_fonts(ctx: &egui::Context, course: &Course, media: &dyn MediaSource) {
    if course.fonts.is_empty() {
        install_base(ctx);
        return;
    }

    let fingerprint = fingerprint_for(course, media);
    if installed_fingerprint(ctx).as_ref() == Some(&fingerprint) {
        return;
    }

    let mut fonts = base_definitions();
    let inter_fallback = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_else(|| vec!["inter_regular".to_owned()]);

    for (index, face) in course.fonts.iter().enumerate() {
        let MediaFetch::Ready(bytes) = media.get(&face.src) else {
            continue;
        };
        // Treat a hostile or corrupt file as missing; never feed egui bytes
        // that do not parse as a font.
        let Ok(faces) = coursecraft_media::font_collection_info(&bytes) else {
            continue;
        };
        let face_index = faces
            .iter()
            .find(|(_, info, aliases)| {
                (info.family.eq_ignore_ascii_case(&face.family)
                    || aliases.iter().any(|a| a.eq_ignore_ascii_case(&face.family)))
                    && to_course_style(info.style) == face.style
            })
            .or_else(|| {
                faces.iter().find(|(_, info, aliases)| {
                    info.family.eq_ignore_ascii_case(&face.family)
                        || aliases.iter().any(|a| a.eq_ignore_ascii_case(&face.family))
                })
            })
            .map_or(0, |(idx, _, _)| *idx);
        let data_key = format!("course-font-{index}");
        let mut font_data = FontData::from_owned(bytes.to_vec());
        font_data.index = face_index;
        fonts
            .font_data
            .insert(data_key.clone(), Arc::new(font_data));
        let mut list = Vec::with_capacity(1 + inter_fallback.len());
        list.push(data_key);
        // Inter follows the course face so a missing glyph never turns into a
        // box while the bundled font can draw it.
        list.extend(inter_fallback.iter().cloned());
        fonts
            .families
            .insert(course_face_family(&face.family, face.style), list);
    }

    set_fonts_and_track(ctx, fonts);
    clear_tried_system_faces(ctx);
    store_fingerprint(ctx, fingerprint);
}

fn to_course_style(style: coursecraft_media::FontFileStyle) -> FontStyle {
    match style {
        coursecraft_media::FontFileStyle::Regular => FontStyle::Regular,
        coursecraft_media::FontFileStyle::Bold => FontStyle::Bold,
        coursecraft_media::FontFileStyle::Italic => FontStyle::Italic,
        coursecraft_media::FontFileStyle::BoldItalic => FontStyle::BoldItalic,
    }
}

fn inter_family(bold: bool, italic: bool) -> &'static str {
    match (bold, italic) {
        (false, false) => "inter",
        (true, false) => "inter-bold",
        (false, true) => "inter-italic",
        (true, true) => "inter-bold-italic",
    }
}

fn style_for(bold: bool, italic: bool) -> FontStyle {
    match (bold, italic) {
        (false, false) => FontStyle::Regular,
        (true, false) => FontStyle::Bold,
        (false, true) => FontStyle::Italic,
        (true, true) => FontStyle::BoldItalic,
    }
}

fn family_exists(ctx: &egui::Context, family: &FontFamily) -> bool {
    pending_defs_for_pass(ctx).is_some_and(|defs| defs.families.contains_key(family))
        || ctx.fonts(|f| f.definitions().families.contains_key(family))
}

fn install_system_face_incrementally(
    ctx: &egui::Context,
    family_name: &str,
    style: FontStyle,
    data: SystemFontData,
) -> FontFamily {
    let target = system_face_family(family_name, style);
    if family_exists(ctx, &target) {
        return target;
    }
    let mut defs =
        pending_defs_for_pass(ctx).unwrap_or_else(|| ctx.fonts(|f| f.definitions().clone()));
    let data_key = format!(
        "sys-font:{}:{}",
        family_name.trim().to_ascii_lowercase(),
        style.as_str()
    );
    let mut font_data = FontData::from_owned(data.bytes.to_vec());
    font_data.index = data.face_index;
    defs.font_data.insert(data_key.clone(), Arc::new(font_data));
    let inter_fallback = defs
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_else(|| vec!["inter_regular".to_owned()]);
    let mut list = Vec::with_capacity(1 + inter_fallback.len());
    list.push(data_key);
    list.extend(inter_fallback);
    defs.families.insert(target.clone(), list);
    set_fonts_and_track(ctx, defs);
    ctx.request_repaint();
    target
}

/// Choose a slide text font, resolving `family` to:
/// 1. course-embedded face (exact style, then regular)
/// 2. installed system face (exact style, then regular)
/// 3. Inter (exact style, then proportional).
#[must_use]
pub fn slide_font_named(
    ctx: &egui::Context,
    family: Option<&str>,
    bold: bool,
    italic: bool,
    size: f32,
) -> FontId {
    let name = family.map(str::trim).filter(|name| !name.is_empty());

    if let Some(name) = name
        && !name.eq_ignore_ascii_case("Inter")
        && !name.eq_ignore_ascii_case("Open Sans")
    {
        let requested_style = style_for(bold, italic);
        let exact = course_face_family(name, requested_style);
        if family_exists(ctx, &exact) {
            return FontId::new(size, exact);
        }
        let regular = course_face_family(name, FontStyle::Regular);
        if family_exists(ctx, &regular) {
            return FontId::new(size, regular);
        }

        // Installed system font (desktop only, when a provider is registered).
        let sys_exact = system_face_family(name, requested_style);
        if family_exists(ctx, &sys_exact) {
            return FontId::new(size, sys_exact);
        }
        let key = (name.to_ascii_lowercase(), requested_style.as_str());
        if !was_system_face_tried(ctx, &key) {
            mark_system_face_tried(ctx, key);
            if let Some(provider) = system_font_provider(ctx)
                && let Some((actual_style, data)) = provider.load_face(name, requested_style)
            {
                let installed = install_system_face_incrementally(ctx, name, actual_style, data);
                return FontId::new(size, installed);
            }
        }
        let sys_regular = system_face_family(name, FontStyle::Regular);
        if family_exists(ctx, &sys_regular) {
            return FontId::new(size, sys_regular);
        }
    }

    let target = FontFamily::Name(inter_family(bold, italic).into());
    let font_family = if family_exists(ctx, &target) {
        target
    } else {
        FontFamily::Proportional
    };
    FontId::new(size, font_family)
}

/// Choose the appropriate slide text font, falling back to [`FontFamily::Proportional`]
/// if [`install_fonts`] has not been called on `ctx`.
#[must_use]
pub fn slide_font(ctx: &egui::Context, bold: bool, italic: bool, size: f32) -> FontId {
    slide_font_named(ctx, None, bold, italic, size)
}
