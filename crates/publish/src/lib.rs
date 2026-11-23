//! CourseCraft publishing (L4).
//!
//! Builds a SCORM 1.2 package in memory from a course, the compiled web player
//! and the course's media, and zips it. No filesystem access, so it also runs
//! in the browser; the CLI does the reading and writing.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]

use std::collections::HashSet;
use std::io::Write as _;

use coursecraft_course::{
    BACKGROUND_AUDIO_ID, Course, FontFace, FontStyle, Object, ObjectKind, TextRun, TextStyle,
};
use coursecraft_scorm::Standard;
use coursecraft_xapi::{Flavor, PackageSpec, cmi5_xml, tincan_xml};

#[cfg(test)]
mod tests;

/// Root marker naming the package standard. Missing means SCORM 1.2.
pub const PACKAGE_MARKER: &str = "coursecraft-package.json";

/// A package standard: SCORM, one of the xAPI flavors, LTI, or standalone Web.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageStandard {
    Scorm(Standard),
    Xapi(Flavor),
    /// An LTI 1.3 package for the CourseCraft LTI server: the SCORM 1.2 file
    /// set without a manifest, plus a `{"standard":"lti"}` marker.
    Lti,
    /// A standalone web package for any static host: the SCORM 1.2 file set
    /// without a manifest, plus a `{"standard":"web"}` marker.
    Web,
}

/// A single file in a package, holding its relative package path and contents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageFile {
    pub path: String,
    pub bytes: Vec<u8>,
}

/// In-memory inputs needed to build a SCORM 1.2 package.
pub struct PackageInput<'a> {
    pub course: &'a Course,
    pub course_json: &'a str,
    pub player_files: &'a [PackageFile],
    pub media: &'a dyn Fn(&str) -> Option<Vec<u8>>,
}

/// Publishing errors. Every invalid input is an error, never a panic.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PublishError {
    #[error("player files must contain index.html")]
    MissingIndexHtml,
    #[error("unsafe path for object {object}: {src}")]
    UnsafePath { object: String, src: String },
    #[error("missing media file for object {object}: {src}")]
    MissingMedia { object: String, src: String },
    #[error("duplicate package file: {0}")]
    DuplicateFile(String),
    #[error("scorm manifest error: {0}")]
    Manifest(#[from] coursecraft_scorm::ScormError),
    #[error("zip error: {0}")]
    Zip(String),
}

impl From<zip::result::ZipError> for PublishError {
    fn from(err: zip::result::ZipError) -> Self {
        PublishError::Zip(err.to_string())
    }
}

impl From<std::io::Error> for PublishError {
    fn from(err: std::io::Error) -> Self {
        PublishError::Zip(err.to_string())
    }
}

pub use coursecraft_format::is_safe_relative_path;

struct MediaRef<'a> {
    object_id: &'a str,
    src: &'a str,
}

fn collect_object_media<'a>(obj: &'a Object, out: &mut Vec<MediaRef<'a>>) {
    match &obj.kind {
        ObjectKind::Image { src, .. } | ObjectKind::Lottie { src, .. } => {
            out.push(MediaRef {
                object_id: &obj.id,
                src,
            });
        }
        ObjectKind::Audio { src, captions, .. } => {
            out.push(MediaRef {
                object_id: &obj.id,
                src,
            });
            if let Some(captions) = captions {
                out.push(MediaRef {
                    object_id: &obj.id,
                    src: captions,
                });
            }
        }
        ObjectKind::Video {
            src,
            captions,
            poster,
            ..
        } => {
            out.push(MediaRef {
                object_id: &obj.id,
                src,
            });
            if let Some(captions) = captions {
                out.push(MediaRef {
                    object_id: &obj.id,
                    src: captions,
                });
            }
            if let Some(poster) = poster
                && !poster.is_empty()
            {
                out.push(MediaRef {
                    object_id: &obj.id,
                    src: poster,
                });
            }
        }
        ObjectKind::Shape { fill_style, .. } | ObjectKind::Button { fill_style, .. } => {
            if let Some(src) = fill_style
                .as_ref()
                .and_then(coursecraft_course::FillStyle::picture_src)
                && !src.is_empty()
            {
                out.push(MediaRef {
                    object_id: &obj.id,
                    src,
                });
            }
        }
        ObjectKind::Path { path } => {
            if let Some(src) = path
                .fill_style
                .as_ref()
                .and_then(coursecraft_course::FillStyle::picture_src)
                && !src.is_empty()
            {
                out.push(MediaRef {
                    object_id: &obj.id,
                    src,
                });
            }
        }
        _ => {}
    }
    for state in &obj.states {
        if let Some(src) = &state.src
            && !src.is_empty()
        {
            out.push(MediaRef {
                object_id: &obj.id,
                src,
            });
        }
        if let Some(src) = state
            .fill_style
            .as_ref()
            .and_then(coursecraft_course::FillStyle::picture_src)
            && !src.is_empty()
        {
            out.push(MediaRef {
                object_id: &obj.id,
                src,
            });
        }
        for extra in &state.extras {
            let extra_src = match &extra.kind {
                coursecraft_course::StateExtraKind::Image { src, .. } => Some(src.as_str()),
                coursecraft_course::StateExtraKind::Shape { fill_style, .. } => fill_style
                    .as_ref()
                    .and_then(coursecraft_course::FillStyle::picture_src),
                coursecraft_course::StateExtraKind::Path { path } => path
                    .fill_style
                    .as_ref()
                    .and_then(coursecraft_course::FillStyle::picture_src),
                coursecraft_course::StateExtraKind::Text { .. } => None,
            };
            if let Some(src) = extra_src
                && !src.is_empty()
            {
                out.push(MediaRef {
                    object_id: &obj.id,
                    src,
                });
            }
        }
    }
}

fn collect_resource_media<'a>(course: &'a Course, out: &mut Vec<MediaRef<'a>>) {
    if let Some(logo) = &course.player.logo {
        out.push(MediaRef {
            object_id: "player-logo",
            src: logo,
        });
    }
    for resource in &course.resources {
        if let coursecraft_course::ResourceTarget::File { src } = &resource.target {
            out.push(MediaRef {
                object_id: &resource.id,
                src,
            });
        }
    }
}

fn add_web_folders(
    course: &Course,
    files: &mut Vec<PackageFile>,
    web_folder: &WebFolderFiles<'_>,
) -> Result<(), PublishError> {
    let mut web_folders = HashSet::new();
    for slide in course.slides() {
        for obj in &slide.objects {
            collect_web_folders(obj, &mut web_folders);
        }
        for layer in &slide.layers {
            for obj in &layer.objects {
                collect_web_folders(obj, &mut web_folders);
            }
        }
    }
    for master in &course.masters {
        for obj in &master.objects {
            collect_web_folders(obj, &mut web_folders);
        }
    }
    for bank in &course.banks {
        for slide in &bank.slides {
            for obj in &slide.objects {
                collect_web_folders(obj, &mut web_folders);
            }
            for layer in &slide.layers {
                for obj in &layer.objects {
                    collect_web_folders(obj, &mut web_folders);
                }
            }
        }
    }
    let mut folders: Vec<String> = web_folders.into_iter().collect();
    folders.sort();
    for folder in folders {
        if !is_safe_relative_path(&folder) || !folder.starts_with("web/") {
            return Err(PublishError::UnsafePath {
                object: "web".to_string(),
                src: folder,
            });
        }
        let entry = web_folder_entry(course, &folder);
        let folder_files = web_folder(&folder).ok_or_else(|| PublishError::MissingMedia {
            object: "web".to_string(),
            src: entry.clone().unwrap_or_else(|| folder.clone()),
        })?;
        let prefix = format!("{folder}/");
        let mut found_entry = entry.is_none();
        for (relative, bytes) in folder_files {
            if !is_safe_relative_path(&relative) || !relative.starts_with(&prefix) {
                return Err(PublishError::UnsafePath {
                    object: "web".to_string(),
                    src: relative,
                });
            }
            if entry.as_deref() == Some(relative.as_str()) {
                found_entry = true;
            }
            files.push(PackageFile {
                path: format!("media/{relative}"),
                bytes,
            });
        }
        if !found_entry {
            return Err(PublishError::MissingMedia {
                object: "web".to_string(),
                src: entry.unwrap_or(folder),
            });
        }
    }
    Ok(())
}

fn collect_web_folders(obj: &Object, out: &mut HashSet<String>) {
    if let ObjectKind::Web {
        source: coursecraft_course::WebSource::Folder { path, .. },
    } = &obj.kind
    {
        out.insert(path.clone());
    }
}

/// The media-relative entry file of the first web object using `folder`.
fn web_folder_entry(course: &Course, folder: &str) -> Option<String> {
    let mut objects = course
        .slides()
        .flat_map(|slide| {
            slide
                .objects
                .iter()
                .chain(slide.layers.iter().flat_map(|layer| layer.objects.iter()))
        })
        .chain(
            course
                .masters
                .iter()
                .flat_map(|master| master.objects.iter()),
        )
        .chain(
            course
                .banks
                .iter()
                .flat_map(|bank| bank.slides.iter())
                .flat_map(|slide| {
                    slide
                        .objects
                        .iter()
                        .chain(slide.layers.iter().flat_map(|layer| layer.objects.iter()))
                }),
        );
    objects.find_map(|object| match &object.kind {
        ObjectKind::Web {
            source: coursecraft_course::WebSource::Folder { path, entry },
        } if path == folder => Some(format!("{path}/{entry}")),
        _ => None,
    })
}

/// Build the full SCORM 1.2 file set in memory.
///
/// This keeps its original signature and behaviour; it calls
/// [`build_package_for`] with [`Standard::Scorm12`].
///
/// # Errors
/// Returns [`PublishError`] if `index.html` is missing, any path is unsafe,
/// a media file cannot be resolved, or manifest generation fails.
pub fn build_package(input: &PackageInput<'_>) -> Result<Vec<PackageFile>, PublishError> {
    build_package_for(input, Standard::Scorm12)
}

/// Build a package for a SCORM standard in memory.
///
/// This delegates to [`build_package_standard`] with
/// [`PackageStandard::Scorm`], so existing SCORM output is unchanged.
///
/// # Errors
/// Returns [`PublishError`] if `index.html` is missing, any path is unsafe,
/// a media file cannot be resolved, or manifest generation fails.
pub fn build_package_for(
    input: &PackageInput<'_>,
    standard: Standard,
) -> Result<Vec<PackageFile>, PublishError> {
    build_package_standard(input, PackageStandard::Scorm(standard))
}

/// Build a package for any supported standard in memory.
///
/// Validates player files and course media paths, replaces any player
/// `course.json` with the current course, and returns the files sorted by
/// path with no duplicates.
///
/// - SCORM packages get `imsmanifest.xml`; 2004 also gets a standard marker.
/// - cmi5 gets `cmi5.xml` and a `{"standard":"cmi5"}` marker.
/// - xAPI gets `tincan.xml` and a `{"standard":"xapi"}` marker.
/// - LTI gets the SCORM 1.2 file set without a manifest and a
///   `{"standard":"lti"}` marker.
///
/// xAPI and LTI packages have no `imsmanifest.xml`.
///
/// # Errors
/// Returns [`PublishError`] if `index.html` is missing, any path is unsafe,
/// a media file cannot be resolved, or manifest generation fails.
pub fn build_package_standard(
    input: &PackageInput<'_>,
    standard: PackageStandard,
) -> Result<Vec<PackageFile>, PublishError> {
    build_package_standard_with_folders(input, standard, &|_| None)
}

/// Build a package and include every file in each web object's folder.
///
/// `web_folder` receives a folder path relative to `media/` (for example
/// `web/site`) and returns its files keyed the same way (`web/site/index.html`).
/// Returning `None` is reported as missing media. Use this from hosts that can
/// list a folder; the plain [`build_package_standard`] cannot and rejects a
/// course that uses a folder source.
///
/// # Errors
/// Returns [`PublishError`] for the same reasons as [`build_package_standard`],
/// plus unsafe or missing web-folder files.
/// Lists a web object's folder files by media-relative path.
pub type WebFolderFiles<'a> = dyn Fn(&str) -> Option<Vec<(String, Vec<u8>)>> + 'a;

pub fn build_package_standard_with_folders(
    input: &PackageInput<'_>,
    standard: PackageStandard,
    web_folder: &WebFolderFiles<'_>,
) -> Result<Vec<PackageFile>, PublishError> {
    // 1. Validate player files and check for index.html.
    let mut has_index = false;
    let mut files = Vec::new();

    for player_file in input.player_files {
        if !is_safe_relative_path(&player_file.path) {
            return Err(PublishError::UnsafePath {
                object: "player".to_string(),
                src: player_file.path.clone(),
            });
        }
        if player_file.path == "index.html" {
            has_index = true;
        }
        if player_file.path == "course.json" || player_file.path == PACKAGE_MARKER {
            // Ours replace any course.json or standard marker in the player folder.
            continue;
        }
        files.push(player_file.clone());
    }

    if !has_index {
        return Err(PublishError::MissingIndexHtml);
    }

    // 2. Add course.json. Templates are authoring-only and never published.
    let course_json = if input.course.templates.is_empty() {
        input.course_json.to_owned()
    } else {
        let mut published = input.course.clone();
        published.templates.clear();
        published
            .to_json()
            .unwrap_or_else(|_| input.course_json.to_owned())
    };
    files.push(PackageFile {
        path: "course.json".to_string(),
        bytes: course_json.as_bytes().to_vec(),
    });

    // 3. Collect media references from base objects and layer objects, and from
    // masters, which draw under every slide that names them. Template media is
    // not collected: a template is not played.
    let mut media_refs = Vec::new();
    for slide in input.course.slides() {
        for obj in &slide.objects {
            collect_object_media(obj, &mut media_refs);
        }
        for layer in &slide.layers {
            for obj in &layer.objects {
                collect_object_media(obj, &mut media_refs);
            }
        }
    }
    for master in &input.course.masters {
        for obj in &master.objects {
            collect_object_media(obj, &mut media_refs);
        }
    }
    for bank in &input.course.banks {
        for slide in &bank.slides {
            for obj in &slide.objects {
                collect_object_media(obj, &mut media_refs);
            }
            for layer in &slide.layers {
                for obj in &layer.objects {
                    collect_object_media(obj, &mut media_refs);
                }
            }
        }
    }

    // A course-wide background track is media too.
    media_refs.extend(input.course.background_audio.iter().map(|audio| MediaRef {
        object_id: BACKGROUND_AUDIO_ID,
        src: &audio.src,
    }));

    // Course fonts are media too: every face ships under media/fonts/.
    for face in &input.course.fonts {
        media_refs.push(MediaRef {
            object_id: "font",
            src: &face.src,
        });
    }

    // Resource files ship like any other asset, under media/.
    collect_resource_media(input.course, &mut media_refs);

    let mut seen_sources = HashSet::new();
    for m in media_refs {
        let src = m.src;
        // Skip remote http:// and https:// sources.
        if src.starts_with("http://") || src.starts_with("https://") {
            continue;
        }

        if !is_safe_relative_path(src) {
            return Err(PublishError::UnsafePath {
                object: m.object_id.to_string(),
                src: src.to_string(),
            });
        }

        if !seen_sources.insert(src) {
            continue;
        }

        let bytes = (input.media)(src).ok_or_else(|| PublishError::MissingMedia {
            object: m.object_id.to_string(),
            src: src.to_string(),
        })?;

        // Package layout rule: always "media/" + src (the player loads `media/{src}`).
        let path = format!("media/{src}");

        files.push(PackageFile { path, bytes });
    }

    // 3b. Web object folders: every file ships under media/<path>/ and keeps
    // its relative path, so the iframe can load `media/web/<name>/<entry>`.
    add_web_folders(input.course, &mut files, web_folder)?;

    // 4. State the standard for the player. SCORM 1.2 stays marker-free so its
    // packages are unchanged; a missing file means 1.2.
    match standard {
        PackageStandard::Scorm(Standard::Scorm12) => {}
        PackageStandard::Scorm(Standard::Scorm2004) => {
            files.push(PackageFile {
                path: PACKAGE_MARKER.to_string(),
                bytes: b"{\"standard\":\"scorm2004\"}".to_vec(),
            });
        }
        PackageStandard::Xapi(flavor) => {
            let marker = format!("{{\"standard\":\"{}\"}}", flavor.as_id());
            files.push(PackageFile {
                path: PACKAGE_MARKER.to_string(),
                bytes: marker.into_bytes(),
            });
            let spec = PackageSpec {
                course_id: input.course.id.clone(),
                title: input.course.title.clone(),
                mastery_score: input.course.quiz.as_ref().map(|q| q.passing_percent),
            };
            let (path, xml) = match flavor {
                Flavor::Cmi5 => ("cmi5.xml", cmi5_xml(&spec)),
                Flavor::Xapi => ("tincan.xml", tincan_xml(&spec)),
            };
            files.push(PackageFile {
                path: path.to_string(),
                bytes: xml.into_bytes(),
            });
        }
        PackageStandard::Lti => {
            files.push(PackageFile {
                path: PACKAGE_MARKER.to_string(),
                bytes: b"{\"standard\":\"lti\"}".to_vec(),
            });
        }
        PackageStandard::Web => {
            files.push(PackageFile {
                path: PACKAGE_MARKER.to_string(),
                bytes: b"{\"standard\":\"web\"}".to_vec(),
            });
        }
    }

    // 5. Generate imsmanifest.xml with the full file list, sorted. xAPI
    // packages carry no imsmanifest.
    if let PackageStandard::Scorm(scorm_standard) = standard {
        let mut manifest_files: Vec<String> = files.iter().map(|f| f.path.clone()).collect();
        manifest_files.sort();
        manifest_files.dedup();

        let spec = coursecraft_scorm::ManifestSpec {
            identifier: format!("coursecraft.{}", input.course.id),
            title: input.course.title.clone(),
            launch: "index.html".to_string(),
            files: manifest_files,
            mastery_score: input.course.quiz.as_ref().map(|q| q.passing_percent),
        };

        let manifest_xml = match scorm_standard {
            Standard::Scorm12 => coursecraft_scorm::manifest_scorm12(&spec)?,
            Standard::Scorm2004 => coursecraft_scorm::manifest_scorm2004(&spec)?,
        };
        files.push(PackageFile {
            path: "imsmanifest.xml".to_string(),
            bytes: manifest_xml.into_bytes(),
        });
    }

    // 6. Return the files sorted by path with no duplicates.
    files.sort_by(|a, b| a.path.cmp(&b.path));

    for window in files.windows(2) {
        if let (Some(first), Some(second)) = (window.first(), window.get(1))
            && first.path == second.path
        {
            return Err(PublishError::DuplicateFile(first.path.clone()));
        }
    }

    Ok(files)
}

/// Pack package files into a deterministic SCORM 1.2 zip archive.
///
/// Text is deflated; pictures, video, audio and fonts, which are already
/// compressed, are stored (see [`coursecraft_format::is_precompressed`]).
/// Uses a fixed timestamp of 1980-01-01 00:00:00 for reproducible,
/// byte-identical output.
///
/// # Errors
/// Returns [`PublishError`] if writing or compressing any file fails.
pub fn zip_package(files: &[PackageFile]) -> Result<Vec<u8>, PublishError> {
    let capacity = files
        .iter()
        .map(|f| f.bytes.len().saturating_add(128))
        .fold(1 << 16, usize::saturating_add);
    let mut out = Vec::with_capacity(capacity);
    zip_package_to(files, std::io::Cursor::new(&mut out))?;
    Ok(out)
}

/// Pack package files into a zip written straight to `out` (a file, say), so
/// a large package is never held in memory a second time. Same bytes as
/// [`zip_package`].
///
/// # Errors
/// Returns [`PublishError`] if writing or compressing any file fails.
pub fn zip_package_to<W: std::io::Write + std::io::Seek>(
    files: &[PackageFile],
    out: W,
) -> Result<(), PublishError> {
    let mut writer = zip::ZipWriter::new(out);
    for file in files {
        writer.start_file(&file.path, coursecraft_format::zip_options_for(&file.path))?;
        writer.write_all(&file.bytes)?;
    }
    writer.finish()?;
    Ok(())
}

/// Progress or completion from [`PackageZipper::step`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZipStep {
    /// More bytes remain to be written.
    InProgress { bytes_done: u64, bytes_total: u64 },
    /// Archive finished; contains the complete zip bytes.
    Done(Vec<u8>),
}

/// Incremental zip writer that writes a bounded number of input bytes per
/// [`step`](Self::step) call so callers on single-threaded targets (such as the
/// web editor) can yield between animation frames.
///
/// Produces bytes identical to [`zip_package`].
pub struct PackageZipper {
    files: Vec<PackageFile>,
    writer: Option<zip::ZipWriter<std::io::Cursor<Vec<u8>>>>,
    file_index: usize,
    file_offset: usize,
    bytes_done: u64,
    bytes_total: u64,
}

impl PackageZipper {
    /// Start zipping `files` incrementally.
    #[must_use]
    pub fn new(files: Vec<PackageFile>) -> Self {
        let capacity = files
            .iter()
            .map(|f| f.bytes.len().saturating_add(128))
            .fold(1 << 16, usize::saturating_add);
        let bytes_total = files.iter().map(|f| f.bytes.len() as u64).sum();
        let writer = Some(zip::ZipWriter::new(std::io::Cursor::new(
            Vec::with_capacity(capacity),
        )));
        Self {
            files,
            writer,
            file_index: 0,
            file_offset: 0,
            bytes_done: 0,
            bytes_total,
        }
    }

    /// Write up to `max_bytes` of input file data (at least 1 byte) and return
    /// either [`ZipStep::InProgress`] or [`ZipStep::Done`].
    ///
    /// # Errors
    /// Returns [`PublishError::Zip`] if writing, compressing, or finishing fails,
    /// or if called again after [`ZipStep::Done`].
    pub fn step(&mut self, max_bytes: usize) -> Result<ZipStep, PublishError> {
        let mut budget = max_bytes.max(1);
        let Some(writer) = self.writer.as_mut() else {
            return Err(PublishError::Zip("zipper already finished".to_string()));
        };

        while self.file_index < self.files.len() && budget > 0 {
            let Some(file) = self.files.get(self.file_index) else {
                break;
            };
            if self.file_offset == 0 {
                writer.start_file(&file.path, coursecraft_format::zip_options_for(&file.path))?;
            }
            if file.bytes.is_empty() {
                self.file_index += 1;
                self.file_offset = 0;
                continue;
            }
            if !coursecraft_format::is_precompressed(&file.path) {
                // DeflateEncoder's block boundaries depend on input slice size,
                // so deflated entries are written whole to stay byte-identical
                // to `zip_package`.
                writer.write_all(&file.bytes)?;
                self.bytes_done = self.bytes_done.saturating_add(file.bytes.len() as u64);
                budget = budget.saturating_sub(file.bytes.len());
                self.file_index += 1;
                self.file_offset = 0;
                continue;
            }
            let remaining = file.bytes.len().saturating_sub(self.file_offset);
            let take = remaining.min(budget);
            let end = self.file_offset.saturating_add(take);
            if let Some(chunk) = file.bytes.get(self.file_offset..end) {
                writer.write_all(chunk)?;
            }
            self.file_offset = end;
            self.bytes_done = self.bytes_done.saturating_add(take as u64);
            budget = budget.saturating_sub(take);
            if self.file_offset >= file.bytes.len() {
                self.file_index += 1;
                self.file_offset = 0;
            }
        }

        if self.file_index >= self.files.len() {
            let Some(writer) = self.writer.take() else {
                return Err(PublishError::Zip("zipper already finished".to_string()));
            };
            let cursor = writer.finish()?;
            return Ok(ZipStep::Done(cursor.into_inner()));
        }

        Ok(ZipStep::InProgress {
            bytes_done: self.bytes_done,
            bytes_total: self.bytes_total,
        })
    }
}

/// Outcome of looking up an installed font face for auto-embedding at publish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishFontLookup {
    /// Embeddable font bytes (already validated).
    Found {
        style: FontStyle,
        bytes: Vec<u8>,
        extension: &'static str,
    },
    /// Installed on this computer, but its OS/2 `fsType` restricts embedding.
    Restricted,
    /// Installed on this computer, but could not be read.
    Unreadable,
    /// Not installed on this computer.
    Missing,
}

/// Optional provider of installed fonts for auto-embedding at publish.
pub trait PublishFontProvider {
    fn lookup_font(&self, family: &str, style: FontStyle) -> PublishFontLookup;
}

/// Result of auto-embedding installed fonts used by a course before packaging.
#[derive(Debug, Clone, PartialEq)]
pub struct AutoEmbedResult {
    pub course: Course,
    pub embedded_files: Vec<(FontFace, Vec<u8>)>,
    pub embedded_families: Vec<String>,
    pub warnings: Vec<String>,
}

fn font_style_for(bold: bool, italic: bool) -> FontStyle {
    match (bold, italic) {
        (false, false) => FontStyle::Regular,
        (true, false) => FontStyle::Bold,
        (false, true) => FontStyle::Italic,
        (true, true) => FontStyle::BoldItalic,
    }
}

fn is_bundled_font(name: &str) -> bool {
    let trimmed = name.trim();
    trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("Inter")
        || trimmed.eq_ignore_ascii_case("Open Sans")
}

fn push_used_face(out: &mut Vec<(String, FontStyle)>, family: &str, style: FontStyle) {
    let trimmed = family.trim();
    if is_bundled_font(trimmed) {
        return;
    }
    if !out
        .iter()
        .any(|(f, s)| f.eq_ignore_ascii_case(trimmed) && *s == style)
    {
        out.push((trimmed.to_owned(), style));
    }
}

fn collect_runs_faces(out: &mut Vec<(String, FontStyle)>, style: &TextStyle, runs: &[TextRun]) {
    push_used_face(out, &style.font, font_style_for(style.bold, style.italic));
    for run in runs {
        let fam = run.font.as_deref().unwrap_or(&style.font);
        let bold = run.bold.unwrap_or(style.bold);
        let italic = run.italic.unwrap_or(style.italic);
        push_used_face(out, fam, font_style_for(bold, italic));
    }
}

fn collect_object_faces(out: &mut Vec<(String, FontStyle)>, obj: &Object) {
    match &obj.kind {
        ObjectKind::Text { style, runs, .. } => collect_runs_faces(out, style, runs),
        ObjectKind::Shape {
            text: Some(text), ..
        } => collect_runs_faces(out, &text.style, &text.runs),
        ObjectKind::Table { rows, .. } => {
            for row in rows {
                for cell in &row.cells {
                    collect_runs_faces(out, &cell.style, &cell.runs);
                }
            }
        }
        ObjectKind::Marker { runs, .. } => {
            for run in runs {
                if let Some(font) = &run.font {
                    let bold = run.bold.unwrap_or(false);
                    let italic = run.italic.unwrap_or(false);
                    push_used_face(out, font, font_style_for(bold, italic));
                }
            }
        }
        ObjectKind::Timer { style, .. } => {
            push_used_face(out, &style.font, font_style_for(style.bold, style.italic));
        }
        _ => {}
    }
    let base_style = match &obj.kind {
        ObjectKind::Text { style, .. } => Some(style),
        ObjectKind::Shape { text: Some(t), .. } => Some(&t.style),
        _ => None,
    };
    for state in &obj.states {
        match (&state.text_style, &state.runs) {
            (Some(ts), Some(runs)) => collect_runs_faces(out, ts, runs),
            (Some(ts), None) => {
                push_used_face(out, &ts.font, font_style_for(ts.bold, ts.italic));
            }
            (None, Some(runs)) => {
                if let Some(bs) = base_style {
                    collect_runs_faces(out, bs, runs);
                } else {
                    for run in runs {
                        if let Some(font) = &run.font {
                            let bold = run.bold.unwrap_or(false);
                            let italic = run.italic.unwrap_or(false);
                            push_used_face(out, font, font_style_for(bold, italic));
                        }
                    }
                }
            }
            (None, None) => {}
        }
        for extra in &state.extras {
            match &extra.kind {
                coursecraft_course::StateExtraKind::Text { style, runs, .. } => {
                    collect_runs_faces(out, style, runs);
                }
                coursecraft_course::StateExtraKind::Shape {
                    text: Some(text), ..
                } => {
                    collect_runs_faces(out, &text.style, &text.runs);
                }
                coursecraft_course::StateExtraKind::Shape { text: None, .. }
                | coursecraft_course::StateExtraKind::Image { .. }
                | coursecraft_course::StateExtraKind::Path { .. } => {}
            }
        }
    }
}

/// Collect every non-Inter `(family, style)` face used by text in `course`
/// (excluding authoring-only templates, which are not published).
#[must_use]
pub fn collect_used_font_faces(course: &Course) -> Vec<(String, FontStyle)> {
    let mut out = Vec::new();
    push_used_face(&mut out, &course.theme.font, FontStyle::Regular);
    for ts in &course.text_styles {
        push_used_face(
            &mut out,
            &ts.style.font,
            font_style_for(ts.style.bold, ts.style.italic),
        );
    }
    for slide in course.slides() {
        for obj in &slide.objects {
            collect_object_faces(&mut out, obj);
        }
        for layer in &slide.layers {
            for obj in &layer.objects {
                collect_object_faces(&mut out, obj);
            }
        }
    }
    for master in &course.masters {
        for obj in &master.objects {
            collect_object_faces(&mut out, obj);
        }
    }
    for bank in &course.banks {
        for slide in &bank.slides {
            for obj in &slide.objects {
                collect_object_faces(&mut out, obj);
            }
            for layer in &slide.layers {
                for obj in &layer.objects {
                    collect_object_faces(&mut out, obj);
                }
            }
        }
    }
    out
}

fn sanitize_font_slug(family: &str) -> String {
    let mut slug = String::new();
    for ch in family.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
        } else if (ch == ' ' || ch == '-' || ch == '_') && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let trimmed = slug.trim_matches('-').to_owned();
    if trimmed.is_empty() {
        "font".to_owned()
    } else {
        trimmed
    }
}

/// Auto-embed installed font faces used by `course` that are not yet in
/// `course.fonts`, respecting embedding permissions.
///
/// For any font that could not be embedded (restricted, missing, or unreadable),
/// adds `"Learners without <font> will see Inter."` to `warnings`, followed by
/// `"Font licences are your responsibility."` when any font was embedded or warned.
#[must_use]
pub fn auto_embed_fonts(
    course: &Course,
    provider: Option<&dyn PublishFontProvider>,
) -> AutoEmbedResult {
    let mut out_course = course.clone();
    let mut embedded_files = Vec::new();
    let mut embedded_families: Vec<String> = Vec::new();
    let mut unembedded_families: Vec<String> = Vec::new();

    let mut used_paths: HashSet<String> = out_course
        .fonts
        .iter()
        .map(|f| f.src.to_ascii_lowercase())
        .collect();

    let push_unique = |list: &mut Vec<String>, name: &str| {
        if !list
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(name))
        {
            list.push(name.to_owned());
        }
    };

    for (family, style) in collect_used_font_faces(course) {
        let already_in_course = out_course
            .fonts
            .iter()
            .any(|f| f.family.eq_ignore_ascii_case(&family) && f.style == style);
        if already_in_course {
            continue;
        }
        // If Regular is already embedded and the requested style isn't, only
        // try to embed the exact style when available; otherwise Regular covers it.
        let regular_in_course = out_course
            .fonts
            .iter()
            .any(|f| f.family.eq_ignore_ascii_case(&family) && f.style == FontStyle::Regular);

        let lookup = provider
            .map(|p| p.lookup_font(&family, style))
            .unwrap_or(PublishFontLookup::Missing);

        match lookup {
            PublishFontLookup::Found {
                style: actual_style,
                bytes,
                extension,
            } => {
                let dup = out_course
                    .fonts
                    .iter()
                    .any(|f| f.family.eq_ignore_ascii_case(&family) && f.style == actual_style);
                if dup {
                    continue;
                }
                let slug = sanitize_font_slug(&family);
                let mut candidate = format!("fonts/{slug}-{}.{extension}", actual_style.as_str());
                let mut n = 2usize;
                while used_paths.contains(&candidate.to_ascii_lowercase()) {
                    candidate = format!("fonts/{slug}-{}-{n}.{extension}", actual_style.as_str());
                    n += 1;
                }
                used_paths.insert(candidate.to_ascii_lowercase());
                let face = FontFace {
                    family: family.clone(),
                    style: actual_style,
                    src: candidate,
                };
                out_course.fonts.push(face.clone());
                embedded_files.push((face, bytes));
                push_unique(&mut embedded_families, &family);
            }
            PublishFontLookup::Restricted
            | PublishFontLookup::Unreadable
            | PublishFontLookup::Missing => {
                if !regular_in_course {
                    push_unique(&mut unembedded_families, &family);
                }
            }
        }
    }

    let mut warnings: Vec<String> = unembedded_families
        .into_iter()
        .map(|family| format!("Learners without {family} will see Inter."))
        .collect();
    if !embedded_families.is_empty() || !warnings.is_empty() {
        warnings.push("Font licences are your responsibility.".to_owned());
    }

    AutoEmbedResult {
        course: out_course,
        embedded_files,
        embedded_families,
        warnings,
    }
}
