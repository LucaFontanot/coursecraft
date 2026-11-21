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
