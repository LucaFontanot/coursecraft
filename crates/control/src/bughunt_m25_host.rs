#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use crate::{ControlHost, HeadlessHost};
use coursecraft_engine::Session;

fn scratch_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/agy-scratch")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn headless_host_save_to_folder_preserves_media() {
    let dir = scratch_dir("bughunt-m25-control-folder-save");
    let ccraft_path = dir.join("source.ccraft");
    let folder_path = dir.join("saved.ccraft-folder");

    let mut session = Session::blank("Media Course");
    let slide_id = session.course.scenes[0].slides[0].id.clone();
    session
        .execute(
            "object.add",
            &serde_json::json!({
                "slide": slide_id,
                "kind": { "type": "image", "src": "hero.png" },
                "description": "Hero",
                "frame": { "x": 10.0, "y": 10.0, "w": 100.0, "h": 100.0 }
            }),
        )
        .unwrap();

    let media = vec![("hero.png".to_owned(), vec![0x89, b'P', b'N', b'G'])];
    let bytes = coursecraft_format::save_project(&session.course, &media).unwrap();
    std::fs::write(&ccraft_path, &bytes).unwrap();

    let mut host = HeadlessHost::new(Session::blank("Empty"), None);
    host.open(&ccraft_path, true).unwrap();
    host.save(Some(&folder_path)).unwrap();

    let saved_media = folder_path.join("media").join("hero.png");
    assert!(
        saved_media.is_file(),
        "saving a course with media to a .ccraft-folder via HeadlessHost::save should write media/hero.png"
    );
}

fn course_with_image(title: &str) -> Session {
    let mut session = Session::blank(title);
    let slide_id = session.course.scenes[0].slides[0].id.clone();
    session
        .execute(
            "object.add",
            &serde_json::json!({
                "slide": slide_id,
                "kind": { "type": "image", "src": "hero.png" },
                "description": "Hero",
                "frame": { "x": 10.0, "y": 10.0, "w": 100.0, "h": 100.0 }
            }),
        )
        .unwrap();
    session
}

fn write_ccraft_with_hero(dir: &std::path::Path, bytes: &[u8]) -> PathBuf {
    let ccraft = dir.join("source.ccraft");
    let session = course_with_image("Media Course");
    let media = vec![("hero.png".to_owned(), bytes.to_vec())];
    std::fs::write(
        &ccraft,
        coursecraft_format::save_project(&session.course, &media).unwrap(),
    )
    .unwrap();
    ccraft
}

#[test]
fn a_failed_json_open_keeps_the_session_media() {
    let dir = scratch_dir("fix-e-r1-failed-open");
    let ccraft = write_ccraft_with_hero(&dir, &[1, 2, 3, 4]);
    let bad = dir.join("bad.json");
    std::fs::write(&bad, "{ not json").unwrap();

    let mut host = HeadlessHost::new(Session::blank("Empty"), None);
    host.open(&ccraft, true).unwrap();
    assert!(host.open(&bad, true).is_err());
    assert!(host.open(&dir.join("missing.json"), true).is_err());
    assert!(
        matches!(
            host.media().get("hero.png"),
            coursecraft_render::MediaFetch::Ready(_)
        ),
        "the open course keeps its media after a failed open"
    );
}

#[test]
fn a_json_save_writes_the_media_beside_it() {
    let dir = scratch_dir("fix-e-l1-json-media");
    let ccraft = write_ccraft_with_hero(&dir, &[9, 8, 7]);
    let mut host = HeadlessHost::new(Session::blank("Empty"), None);
    host.open(&ccraft, true).unwrap();
    let json = dir.join("out").join("course.json");
    host.save(Some(&json)).unwrap();
    assert_eq!(
        std::fs::read(dir.join("out").join("media").join("hero.png")).unwrap(),
        vec![9u8, 8, 7]
    );
    let mut again = HeadlessHost::new(Session::blank("Empty"), None);
    again.open(&json, true).unwrap();
    assert!(matches!(
        again.media().get("hero.png"),
        coursecraft_render::MediaFetch::Ready(_)
    ));
}

#[test]
fn a_json_save_that_cannot_write_media_fails_and_stays_dirty() {
    let dir = scratch_dir("fix-e-l1-json-media-fail");
    let ccraft = write_ccraft_with_hero(&dir, &[9, 8, 7]);
    let mut host = HeadlessHost::new(Session::blank("Empty"), None);
    host.open(&ccraft, true).unwrap();
    // A file named `media` blocks the media folder.
    let out = dir.join("out");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("media"), b"in the way").unwrap();
    let json = out.join("course.json");
    host.session.revision += 1;
    assert!(host.save(Some(&json)).is_err());
    assert!(host.info().dirty, "a failed save leaves the course unsaved");
    assert!(!json.exists());
}
