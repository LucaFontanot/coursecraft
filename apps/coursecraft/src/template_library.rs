//! Shared template library stored under `<config_dir>/templates/` as one
//! `.ccraft`-style zip per template.

use std::fs::File;
use std::path::{Path, PathBuf};

use coursecraft_course::{
    Course, EditorSettings, PlayerSettings, SCHEMA_VERSION, Size, Slide, Theme,
};

/// One saved template in the user's shared library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryTemplate {
    /// Safe file stem (without `.ccraft`).
    pub id: String,
    /// Display title of the template.
    pub title: String,
}

/// Directory holding shared library templates under `config_dir`.
#[must_use]
pub fn templates_dir(config_dir: &Path) -> PathBuf {
    config_dir.join("templates")
}

fn is_safe_id(id: &str) -> bool {
    !id.is_empty()
        && !id.contains('/')
        && !id.contains('\\')
        && !id.contains("..")
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn slug_from_title(title: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash && !slug.is_empty() {
            slug.push('-');
            last_dash = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        "template".to_owned()
    } else {
        slug.chars().take(48).collect()
    }
}

fn unique_template_id(dir: &Path, title: &str) -> String {
    let base = slug_from_title(title);
    if !dir.join(format!("{base}.ccraft")).exists() {
        return base;
    }
    for n in 2..10_000 {
        let candidate = format!("{base}-{n}");
        if !dir.join(format!("{candidate}.ccraft")).exists() {
            return candidate;
        }
    }
    format!("{base}-copy")
}

fn template_file(config_dir: &Path, id: &str) -> PathBuf {
    templates_dir(config_dir).join(format!("{id}.ccraft"))
}

/// Save a template slide and its referenced media into `<config_dir>/templates/`.
///
/// # Errors
/// Returns an error if the directory cannot be created, serialization fails,
/// or the file cannot be written.
pub fn save_template(
    config_dir: &Path,
    slide: &Slide,
    slide_size: Size,
    media: &[(String, Vec<u8>)],
) -> Result<LibraryTemplate, String> {
    let dir = templates_dir(config_dir);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create templates directory: {e}"))?;

    let id = unique_template_id(&dir, &slide.title);
    let title = if slide.title.trim().is_empty() {
        id.clone()
    } else {
        slide.title.clone()
    };

    let course = Course {
        schema_version: SCHEMA_VERSION,
        id: id.clone(),
        title: title.clone(),
        slide_size,
        variables: Vec::new(),
        scenes: Vec::new(),
        masters: Vec::new(),
        templates: vec![slide.clone()],
        banks: Vec::new(),
        start_slide: None,
        quiz: None,
        tracking: Default::default(),
        player: PlayerSettings::default(),
        theme: Theme::default(),
        language: None,
        strings: std::collections::BTreeMap::new(),
        fonts: Vec::new(),
        background_audio: None,
        text_styles: Vec::new(),
        resources: Vec::new(),
        glossary: Vec::new(),
        editor: EditorSettings::default(),
    };

    let bytes = coursecraft_format::save_project(&course, media)
        .map_err(|e| format!("Failed to package template: {e}"))?;
    let path = template_file(config_dir, &id);
    crate::files::write_atomic(&path, &bytes)
        .map_err(|e| format!("Failed to save template: {e}"))?;

    Ok(LibraryTemplate { id, title })
}

/// List all valid templates in `<config_dir>/templates/`, sorted by title.
#[must_use]
pub fn list_templates(config_dir: &Path) -> Vec<LibraryTemplate> {
    let dir = templates_dir(config_dir);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("ccraft") || !path.is_file() {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if !is_safe_id(stem) {
            continue;
        }
        let Ok(file) = File::open(&path) else {
            continue;
        };
        let Ok(course) = coursecraft_format::read_course(file) else {
            continue;
        };
        let Some(template) = course.templates.first() else {
            continue;
        };
        let title = if template.title.trim().is_empty() {
            stem.to_owned()
        } else {
            template.title.clone()
        };
        out.push(LibraryTemplate {
            id: stem.to_owned(),
            title,
        });
    }
    out.sort_by(|a, b| {
        a.title
            .to_ascii_lowercase()
            .cmp(&b.title.to_ascii_lowercase())
            .then_with(|| a.id.cmp(&b.id))
    });
    out
}

/// A loaded template slide and its `(path, bytes)` media entries.
pub type LoadedTemplate = (Slide, Vec<(String, Vec<u8>)>);

/// Load a saved template slide and its media from `<config_dir>/templates/`.
///
/// # Errors
/// Returns an error if the id is invalid, the file cannot be read, or the
/// archive is missing a template slide.
pub fn load_template(config_dir: &Path, id: &str) -> Result<LoadedTemplate, String> {
    if !is_safe_id(id) {
        return Err(format!("Invalid template id {id:?}"));
    }
    let path = template_file(config_dir, id);
    let file = File::open(&path).map_err(|e| format!("Failed to read template {id:?}: {e}"))?;
    let reader = std::io::BufReader::with_capacity(256 * 1024, file);
    let project = coursecraft_format::open_project_from(reader)
        .map_err(|e| format!("Failed to open template {id:?}: {e}"))?;
    let slide = project
        .course
        .templates
        .into_iter()
        .next()
        .ok_or_else(|| format!("Template {id:?} has no slide"))?;
    Ok((slide, project.media))
}

/// Delete a template from `<config_dir>/templates/`.
///
/// # Errors
/// Returns an error if the id is invalid or the file cannot be removed.
pub fn delete_template(config_dir: &Path, id: &str) -> Result<(), String> {
    if !is_safe_id(id) {
        return Err(format!("Invalid template id {id:?}"));
    }
    let path = template_file(config_dir, id);
    std::fs::remove_file(&path).map_err(|e| format!("Failed to delete template {id:?}: {e}"))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use coursecraft_course::{Course, Object};
    use coursecraft_ui_egui::Editor;

    const HELLO_JSON: &str = include_str!("../../../examples/hello-course/course.json");

    fn scratch_dir(sub: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/agy-scratch")
            .join(sub);
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    fn image_object(id: &str, name: &str, src: &str) -> Object {
        serde_json::from_value(serde_json::json!({
            "id": id,
            "name": name,
            "kind": { "type": "image", "src": src },
            "description": "Hero image",
            "frame": { "x": 10.0, "y": 20.0, "w": 100.0, "h": 80.0 }
        }))
        .unwrap()
    }

    #[test]
    fn save_list_insert_into_second_course_and_delete() {
        let dir = scratch_dir("template-library-roundtrip");

        let course1 = Course::from_json(HELLO_JSON).unwrap();
        let mut editor1 = Editor::new(course1);
        let img_bytes = b"fake-png-bytes".to_vec();
        editor1
            .media
            .push("hero.png".to_owned(), img_bytes.clone().into());

        let mut slide = editor1.session.course.scenes[0].slides[0].clone();
        slide.id = "tpl-welcome".to_owned();
        slide.title = "Welcome Card".to_owned();
        slide.objects.push(image_object("img1", "Hero", "hero.png"));

        let media = editor1.slide_media(&slide);
        assert_eq!(media.len(), 1);
        assert_eq!(media[0].0, "hero.png");

        let saved = save_template(&dir, &slide, editor1.session.course.slide_size, &media)
            .expect("save template");
        assert_eq!(saved.title, "Welcome Card");
        assert!(
            templates_dir(&dir)
                .join(format!("{}.ccraft", saved.id))
                .exists()
        );

        let listed = list_templates(&dir);
        assert_eq!(listed, vec![saved.clone()]);

        // Insert into a second, separate course.
        let course2 = Course::from_json(HELLO_JSON).unwrap();
        let initial_slides = course2.slides().count();
        let mut editor2 = Editor::new(course2);
        let (loaded_slide, loaded_media) = load_template(&dir, &saved.id).expect("load template");
        editor2.insert_library_template(loaded_slide, loaded_media, None, None);

        assert_eq!(editor2.session.course.slides().count(), initial_slides + 1);
        assert!(
            editor2
                .media
                .iter()
                .any(|(src, bytes)| src == "hero.png" && bytes.as_ref() == img_bytes.as_slice()),
            "second course should receive template media"
        );

        delete_template(&dir, &saved.id).expect("delete template");
        assert!(list_templates(&dir).is_empty());
        assert!(
            !templates_dir(&dir)
                .join(format!("{}.ccraft", saved.id))
                .exists()
        );
    }

    #[test]
    fn insert_library_template_renames_colliding_media_with_different_bytes() {
        let dir = scratch_dir("template-library-media-collision");

        let course1 = Course::from_json(HELLO_JSON).unwrap();
        let mut slide = course1.scenes[0].slides[0].clone();
        slide.id = "tpl-img".to_owned();
        slide.title = "Image Slide".to_owned();
        slide
            .objects
            .push(image_object("img1", "Banner", "banner.png"));

        let saved = save_template(
            &dir,
            &slide,
            course1.slide_size,
            &[("banner.png".to_owned(), b"template-banner".to_vec())],
        )
        .unwrap();

        let course2 = Course::from_json(HELLO_JSON).unwrap();
        let mut editor2 = Editor::new(course2);
        editor2
            .media
            .push("banner.png".to_owned(), b"existing-banner".to_vec().into());

        let (loaded_slide, loaded_media) = load_template(&dir, &saved.id).unwrap();
        editor2.insert_library_template(loaded_slide, loaded_media, None, None);

        assert!(
            editor2
                .media
                .iter()
                .any(|(src, bytes)| src == "banner.png" && bytes.as_ref() == b"existing-banner"),
            "existing course media must not be overwritten"
        );
        assert!(
            editor2
                .media
                .iter()
                .any(|(src, bytes)| src != "banner.png" && bytes.as_ref() == b"template-banner"),
            "colliding template media should be stored under a fresh key"
        );
    }

    #[test]
    fn invalid_template_id_is_rejected() {
        let dir = scratch_dir("template-library-invalid-id");
        assert!(load_template(&dir, "../secret").is_err());
        assert!(delete_template(&dir, "a/b").is_err());
        assert!(delete_template(&dir, "").is_err());
    }

    #[test]
    fn r2_02_load_template_streaming_round_trip_and_corrupt() {
        let dir = scratch_dir("r2-02-template-streaming");

        let course = Course::from_json(HELLO_JSON).unwrap();
        let mut slide = course.scenes[0].slides[0].clone();
        slide.id = "tpl-stream".to_owned();
        slide.title = "Streaming Template".to_owned();
        slide
            .objects
            .push(image_object("img-stream", "Stream Hero", "stream-hero.png"));

        let media_bytes = b"\x89PNG\r\n\x1a\nstream-template-bytes".to_vec();
        let saved = save_template(
            &dir,
            &slide,
            course.slide_size,
            &[("stream-hero.png".to_owned(), media_bytes.clone())],
        )
        .expect("save template");

        let (loaded_slide, loaded_media) = load_template(&dir, &saved.id).expect("load template");
        assert_eq!(loaded_slide, slide);
        assert_eq!(
            loaded_media,
            vec![("stream-hero.png".to_owned(), media_bytes)]
        );

        let corrupt_path = templates_dir(&dir).join("corrupt.ccraft");
        std::fs::write(&corrupt_path, b"not-a-valid-zip").expect("write corrupt template");
        let err = load_template(&dir, "corrupt").expect_err("corrupt template must fail");
        assert!(
            err.contains("Failed to open template"),
            "unexpected corrupt error: {err}"
        );

        let missing_err = load_template(&dir, "missing").expect_err("missing template must fail");
        assert!(
            missing_err.contains("Failed to read template"),
            "unexpected missing error: {missing_err}"
        );
    }
}
