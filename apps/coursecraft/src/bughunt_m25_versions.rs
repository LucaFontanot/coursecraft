#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use crate::{files, versions};
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
fn json_project_version_snapshot_is_readable_and_openable() {
    let dir = scratch_dir("bughunt-m25-json-versions");
    let config_dir = dir.join("config");
    let json_path = dir.join("course.json");

    let course = Session::blank("Plain Json Course").course;
    std::fs::write(&json_path, format!("{}\n", course.to_json().unwrap())).unwrap();

    let id = versions::snapshot_before_save(&config_dir, &json_path, 5, &course.title)
        .unwrap()
        .expect("snapshot should be created");

    let listed = versions::list_versions(&config_dir, &json_path);
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, id);
    assert!(
        listed[0].slides.is_some(),
        "version history should be able to read slide count for a plain JSON project snapshot"
    );

    let opened = files::open_from(&listed[0].path);
    assert!(
        opened.is_ok(),
        "Open Copy from Version History should open a plain JSON project snapshot, got: {opened:?}"
    );
}
