//! Publishing from the editor: no egui, so the package logic stays testable.

use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
use std::sync::Arc;

use coursecraft_course::Course;
#[cfg(not(target_arch = "wasm32"))]
use coursecraft_publish::zip_package_to;
use coursecraft_publish::{
    PackageFile, PackageInput, PackageStandard, PublishError, build_package_standard_with_folders,
    zip_package,
};
use coursecraft_scorm::Standard;
use coursecraft_ui_egui::PublishStandard;
use coursecraft_xapi::Flavor;

/// Everything a background publish needs. It owns the course and media, so a
/// publish never borrows the live session while the window keeps drawing.
#[cfg(not(target_arch = "wasm32"))]
pub struct PublishJob {
    pub course: Course,
    pub media: Vec<(String, Arc<[u8]>)>,
    pub standard: PackageStandard,
    pub player: Vec<PackageFile>,
    pub out: PathBuf,
}

/// The written file and its size in bytes.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishOutcome {
    pub path: PathBuf,
    pub bytes: u64,
}

/// Map the UI's small standard enum onto the publishing crate's type.
#[must_use]
pub fn package_standard(standard: PublishStandard) -> PackageStandard {
    match standard {
        PublishStandard::Scorm12 => PackageStandard::Scorm(Standard::Scorm12),
        PublishStandard::Scorm2004 => PackageStandard::Scorm(Standard::Scorm2004),
        PublishStandard::Cmi5 => PackageStandard::Xapi(Flavor::Cmi5),
        PublishStandard::Xapi => PackageStandard::Xapi(Flavor::Xapi),
        PublishStandard::Lti => PackageStandard::Lti,
        PublishStandard::Web => PackageStandard::Web,
    }
}

/// Build the package files for a course and its in-memory media, resolving
/// media from the in-memory items exactly like the CLI's `.ccraft` path.
///
/// # Errors
/// Returns a plain sentence naming the object and file when media is missing,
/// or describing another package problem.
pub(crate) fn package_files_from(
    course: &Course,
    media: &[(String, Arc<[u8]>)],
    standard: PackageStandard,
    player: &[PackageFile],
) -> Result<Vec<PackageFile>, String> {
    let course_json = course
        .to_json()
        .map_err(|e| format!("Couldn't prepare the course for publishing: {e}"))?;
    let mut course_json = course_json;
    // Save writes a trailing newline; publish stays byte-identical to that form.
    course_json.push('\n');

    let media_map: HashMap<&str, &[u8]> = media
        .iter()
        .map(|(name, bytes)| (name.as_str(), bytes.as_ref()))
        .collect();
    let resolve = |src: &str| media_map.get(src).map(|bytes| bytes.to_vec());
    let folder = |path: &str| {
        let prefix = format!("{path}/");
        let files: Vec<(String, Vec<u8>)> = media
            .iter()
            .filter(|(name, _)| name.starts_with(&prefix))
            .map(|(name, bytes)| (name.clone(), bytes.to_vec()))
            .collect();
        (!files.is_empty()).then_some(files)
    };

    let input = PackageInput {
        course,
        course_json: &course_json,
        player_files: player,
        media: &resolve,
    };
    build_package_standard_with_folders(&input, standard, &folder).map_err(readable_error)
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn package_files(job: &PublishJob) -> Result<Vec<PackageFile>, String> {
    package_files_from(&job.course, &job.media, job.standard, &job.player)
}

/// Build a published package zip in memory (used by the web editor).
///
/// # Errors
/// Returns a plain sentence an author can act on.
pub fn run_publish_bytes(
    course: &Course,
    media: &[(String, Arc<[u8]>)],
    standard: PackageStandard,
    player: &[PackageFile],
) -> Result<Vec<u8>, String> {
    let files = package_files_from(course, media, standard, player)?;
    zip_package(&files).map_err(|e| format!("Couldn't create the zip file: {e}"))
}

/// Build the package and write it to the job's output path atomically.
///
/// # Errors
/// Returns a plain sentence an author can act on.
#[cfg(not(target_arch = "wasm32"))]
pub fn run_publish_job(job: PublishJob) -> Result<PublishOutcome, String> {
    let files = package_files(&job)?;

    if let Some(parent) = job.out.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Couldn't create {}: {e}", parent.display()))?;
    }
    // Stream the zip into a temporary file next to the target and rename it
    // into place, so a big package is never held in memory a second time.
    let file_name = job
        .out
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("package.zip");
    let tmp = job
        .out
        .with_file_name(format!("{file_name}.tmp-{}", std::process::id()));
    let written = std::fs::File::create(&tmp)
        .map_err(|e| format!("Couldn't write {}: {e}", job.out.display()))
        .and_then(|file| {
            let mut file = std::io::BufWriter::new(file);
            zip_package_to(&files, &mut file)
                .map_err(|e| format!("Couldn't create the zip file: {e}"))?;
            std::io::Write::flush(&mut file)
                .map_err(|e| format!("Couldn't write {}: {e}", job.out.display()))
        })
        .and_then(|()| {
            std::fs::rename(&tmp, &job.out)
                .map_err(|e| format!("Couldn't write {}: {e}", job.out.display()))
        });
    if let Err(e) = written {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    let bytes = std::fs::metadata(&job.out).map_or(0, |m| m.len());

    Ok(PublishOutcome {
        path: job.out,
        bytes,
    })
}

fn readable_error(error: PublishError) -> String {
    match error {
        PublishError::MissingMedia { object, src } => format!(
            "Couldn't find the media file '{src}' used by object {object}. It may have been moved or deleted."
        ),
        PublishError::MissingIndexHtml => {
            "The bundled player is missing its index.html. Run `cargo xtask build-player` and rebuild the editor.".to_string()
        }
        PublishError::UnsafePath { object, src } => format!(
            "The path '{src}' used by object {object} can't be included in a published package."
        ),
        other => format!("Couldn't build the package: {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coursecraft_engine::Session;

    #[test]
    fn run_publish_bytes_builds_valid_scorm12_zip_in_memory() {
        let course = Session::blank("Web Publish Test").course;
        let player = vec![PackageFile {
            path: "index.html".to_string(),
            bytes: b"<!doctype html><html></html>".to_vec(),
        }];
        let files = package_files_from(
            &course,
            &[],
            PackageStandard::Scorm(Standard::Scorm12),
            &player,
        )
        .expect("package files succeed");
        let names: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        assert!(names.contains(&"imsmanifest.xml"));
        assert!(names.contains(&"course.json"));
        assert!(names.contains(&"index.html"));

        let zip_bytes = run_publish_bytes(
            &course,
            &[],
            PackageStandard::Scorm(Standard::Scorm12),
            &player,
        )
        .expect("in-memory publish succeeds");
        assert!(zip_bytes.starts_with(b"PK\x03\x04"));
        assert!(!zip_bytes.is_empty());
    }
}
