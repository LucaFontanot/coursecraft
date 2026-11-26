//! Tests for opening courses and safe saving: crash safety, verification, locked
//! files, files that changed on disk, damaged files and leftover temp files.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::files::{
    Hooks, LOCKED_MESSAGE, Project, open_from, remove_stale_temps, save_to, write_atomic_with,
};
use coursecraft_course::Course;

const HELLO_JSON: &str = include_str!("../../../examples/hello-course/course.json");

fn scratch_dir(sub: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/agy-scratch")
        .join(sub);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn course_titled(title: &str) -> Course {
    let mut course = Course::from_json(HELLO_JSON).expect("valid course");
    course.title = title.to_string();
    course
}

fn names_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// The child half of the kill test. Does nothing in a normal run; when the
/// parent sets `KILL_CHILD_DIR` it saves again and again until it is killed.
#[test]
fn kill_child_writer() {
    let Some(dir) = std::env::var_os("KILL_CHILD_DIR") else {
        return;
    };
    let target = PathBuf::from(dir).join("p.ccraft");
    let mut n: u32 = 1;
    loop {
        let media = vec![("a.png".to_string(), vec![n as u8; 3000])];
        let _ = save_to(&target, &course_titled(&format!("v{n}")), &media);
        n = n.wrapping_add(1);
    }
}

/// Open the project, trying again for a moment when a read lands on the instant
/// the writer swaps the file in. A damaged file fails every time, so this never
/// hides real damage.
fn open_retrying(target: &Path) -> Result<crate::files::LoadedCourse, String> {
    let mut last = String::new();
    for _ in 0..40 {
        match open_from(target) {
            Ok(loaded) => return Ok(loaded),
            Err(e) => last = e,
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    Err(last)
}

#[test]
fn a_killed_save_never_damages_the_project() {
    let dir = scratch_dir("test-kill-save");
    let target = dir.join("p.ccraft");
    save_to(
        &target,
        &course_titled("v0"),
        &[("a.png".to_string(), vec![0u8; 3000])],
    )
    .unwrap();

    let exe = std::env::current_exe().unwrap();
    let mut seed: u64 = 0x9E37_79B9;
    let mut newest: u32 = 0;
    for round in 0..20 {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let delay = 40 + (seed >> 33) % 90;
        let mut child = std::process::Command::new(&exe)
            .args(["--exact", "files_tests::kill_child_writer", "--nocapture"])
            .env("KILL_CHILD_DIR", &dir)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        // Wait until this child has really replaced the file at least once, so a
        // slow start under a loaded machine cannot end the round before any save.
        let started = std::time::Instant::now();
        let before = open_retrying(&target).map(|(c, _)| c.title);
        loop {
            let now = open_retrying(&target).map(|(c, _)| c.title);
            if now.is_ok() && now != before {
                break;
            }
            assert!(
                started.elapsed() < Duration::from_secs(60),
                "round {round}: the child never completed a save"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        std::thread::sleep(Duration::from_millis(delay));
        let _ = child.kill();
        let _ = child.wait();

        let (course, media) =
            open_retrying(&target).unwrap_or_else(|e| panic!("round {round}: target damaged: {e}"));
        let n: u32 = course.title.strip_prefix('v').unwrap().parse().unwrap();
        newest = newest.max(n);
        assert_eq!(media.len(), 1, "round {round}");
        assert_eq!(
            media[0].1,
            vec![n as u8; 3000],
            "round {round}: media matches {n}"
        );
    }

    assert!(
        newest > 0,
        "the child process must have completed some saves"
    );

    // The next successful save clears whatever temp files the kills left.
    save_to(&target, &course_titled("final"), &[]).unwrap();
    assert_eq!(names_in(&dir), vec!["p.ccraft".to_string()]);
}

#[test]
fn a_broken_temp_file_never_replaces_the_old_file() {
    let dir = scratch_dir("test-verify-refuses");
    let target = dir.join("p.ccraft");
    save_to(
        &target,
        &course_titled("old"),
        &[("a.png".to_string(), vec![1u8; 500])],
    )
    .unwrap();
    let before = std::fs::read(&target).unwrap();

    // Save new bytes, but flip one byte inside the stored picture on disk
    // after the write, as a failing disk or a sync client might.
    let media = vec![("a.png".to_string(), vec![7u8; 5000])];
    let bytes = coursecraft_format::save_project(&course_titled("new"), &media).unwrap();
    let corrupt = |tmp: &Path| -> std::io::Result<()> {
        let mut data = std::fs::read(tmp)?;
        let run = vec![7u8; 100];
        let at = data
            .windows(run.len())
            .position(|w| w == run.as_slice())
            .expect("stored picture bytes are in the file");
        data[at + 50] ^= 0xFF;
        std::fs::write(tmp, data)
    };
    let hooks = Hooks {
        after_write: &corrupt,
        ..Hooks::default()
    };
    let err = write_atomic_with(
        &target,
        &bytes,
        Some(&crate::files::verify_ccraft_file),
        &hooks,
    )
    .expect_err("the check must refuse a damaged copy");
    assert!(err.to_string().contains("existing file was kept"), "{err}");
    assert_eq!(std::fs::read(&target).unwrap(), before);
    assert_eq!(names_in(&dir), vec!["p.ccraft".to_string()]);
}

#[test]
fn a_locked_file_is_retried_until_the_lock_clears() {
    let dir = scratch_dir("test-lock-clears");
    let target = dir.join("p.ccraft");
    std::fs::write(&target, b"old").unwrap();
    let calls = Cell::new(0u32);
    let rename = |from: &Path, to: &Path| -> std::io::Result<()> {
        calls.set(calls.get() + 1);
        if calls.get() <= 3 {
            Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
        } else {
            std::fs::rename(from, to)
        }
    };
    let hooks = Hooks {
        rename: &rename,
        retry_delay: Duration::ZERO,
        ..Hooks::default()
    };
    write_atomic_with(&target, b"new", None, &hooks).unwrap();
    assert_eq!(calls.get(), 4);
    assert_eq!(std::fs::read(&target).unwrap(), b"new");
}

#[test]
fn a_file_that_stays_locked_gives_a_plain_message_and_keeps_the_old_file() {
    let dir = scratch_dir("test-lock-stays");
    let target = dir.join("p.ccraft");
    std::fs::write(&target, b"old").unwrap();
    let calls = Cell::new(0u32);
    let rename = |_: &Path, _: &Path| -> std::io::Result<()> {
        calls.set(calls.get() + 1);
        Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
    };
    let hooks = Hooks {
        rename: &rename,
        retry_delay: Duration::ZERO,
        ..Hooks::default()
    };
    let err = write_atomic_with(&target, b"new", None, &hooks).unwrap_err();
    assert_eq!(err.to_string(), LOCKED_MESSAGE);
    assert_eq!(calls.get(), 5, "five tries in all");
    assert_eq!(std::fs::read(&target).unwrap(), b"old");
    assert_eq!(names_in(&dir), vec!["p.ccraft".to_string()]);
}

#[test]
fn a_change_made_on_disk_is_noticed() {
    let dir = scratch_dir("test-changed-on-disk");
    let path = dir.join("p.ccraft");
    save_to(&path, &course_titled("mine"), &[]).unwrap();

    let mut project = Project::new(0);
    assert!(!project.changed_on_disk(), "untitled is never in conflict");
    project.path = Some(path.clone());
    assert!(!project.changed_on_disk(), "no stamp yet");
    project.record_disk_stamp();
    assert!(!project.changed_on_disk());

    // Another machine's edit arrives through the sync service.
    save_to(&path, &course_titled("theirs, with more text"), &[]).unwrap();
    assert!(project.changed_on_disk());

    // Our own save records the new state and clears the conflict.
    project.record_disk_stamp();
    assert!(!project.changed_on_disk());

    // A deleted file is not a conflict: saving just recreates it.
    std::fs::remove_file(&path).unwrap();
    assert!(!project.changed_on_disk());
}

#[test]
fn save_asks_before_overwriting_a_changed_file_and_overwrite_saves() {
    let dir = scratch_dir("test-conflict-flow");
    let path = dir.join("p.ccraft");
    save_to(&path, &course_titled("first"), &[]).unwrap();

    let mut app = crate::EditorApp::new_desktop(course_titled("first"), None);
    app.project.path = Some(path.clone());
    app.project.record_disk_stamp();
    app.editor.session.course.title = "edited here".into();

    save_to(&path, &course_titled("edited elsewhere"), &[]).unwrap();
    let theirs = std::fs::read(&path).unwrap();

    assert!(!app.perform_save(), "save is held back");
    assert!(app.conflict_prompt.is_some());
    assert_eq!(std::fs::read(&path).unwrap(), theirs, "file untouched");

    assert!(app.save_project_file(true), "Overwrite saves");
    assert_eq!(open_from(&path).unwrap().0.title, "edited here");
    assert!(!app.project.changed_on_disk());
}

#[test]
fn autosave_never_prompts_and_never_touches_the_project_file() {
    let dir = scratch_dir("test-autosave-safe");
    let path = dir.join("p.ccraft");
    save_to(&path, &course_titled("mine"), &[]).unwrap();
    let mut project = Project::new(0);
    project.path = Some(path.clone());
    project.record_disk_stamp();
    let before = std::fs::read(&path).unwrap();

    let auto = crate::autosave::autosave_path_for_project(Some(&path), Path::new(""));
    crate::autosave::write_autosave(&course_titled("unsaved"), &[], &auto).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert!(!project.changed_on_disk());
    assert_eq!(open_from(&auto).unwrap().0.title, "unsaved");
}

#[test]
fn damaged_project_files_give_a_plain_error_and_stay_untouched() {
    let dir = scratch_dir("test-damaged");
    let good = dir.join("good.ccraft");
    save_to(
        &good,
        &course_titled("ok"),
        &[("a.png".to_string(), vec![5u8; 4000])],
    )
    .unwrap();
    let full = std::fs::read(&good).unwrap();

    let mut random = vec![0u8; 3000];
    let mut x: u64 = 0xDEAD_BEEF;
    for b in &mut random {
        x = x.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        *b = (x >> 40) as u8;
    }
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("empty.ccraft", Vec::new()),
        ("truncated.ccraft", full[..full.len() / 2].to_vec()),
        ("tiny.ccraft", full[..10].to_vec()),
        ("random.ccraft", random),
        ("bad.json", b"{ not json".to_vec()),
    ];
    for (name, bytes) in cases {
        let path = dir.join(name);
        std::fs::write(&path, &bytes).unwrap();
        let err = open_from(&path).expect_err(name);
        assert!(err.contains("can't be opened"), "{name}: {err}");
        assert_eq!(std::fs::read(&path).unwrap(), bytes, "{name} untouched");
    }
}

struct LooseCourseDir(PathBuf);

impl LooseCourseDir {
    fn new(name: &str) -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "coursecraft-json-{name}-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&dir).unwrap();
        Self(dir)
    }

    fn write_course(&self, sources: &[(&str, &str)]) -> PathBuf {
        let mut course = course_titled("Loose course");
        let objects = &mut course.scenes[0].slides[0].objects;
        let template = objects[0].clone();
        for (index, (kind, src)) in sources.iter().enumerate() {
            let mut object = template.clone();
            object.id = format!("media-{index}");
            object.kind = serde_json::from_value(serde_json::json!({
                "type": kind,
                "src": src,
            }))
            .unwrap();
            objects.push(object);
        }
        let path = self.0.join("course.json");
        std::fs::write(&path, course.to_json().unwrap()).unwrap();
        path
    }
}

impl Drop for LooseCourseDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn bare_json_loads_only_referenced_sibling_media_once_and_keeps_it_on_save() {
    let dir = LooseCourseDir::new("sibling-media");
    let assets = dir.0.join("media");
    std::fs::create_dir_all(assets.join("pics")).unwrap();
    std::fs::write(assets.join("pics/logo.png"), b"picture").unwrap();
    std::fs::write(assets.join("voice.mp3"), b"audio").unwrap();
    std::fs::write(assets.join("clip.mp4"), b"video").unwrap();
    std::fs::write(assets.join("unused.png"), b"unreferenced").unwrap();
    let path = dir.write_course(&[
        ("image", "pics/logo.png"),
        ("audio", "voice.mp3"),
        ("video", "clip.mp4"),
        ("image", "pics/logo.png"),
    ]);

    let (course, media) = open_from(&path).unwrap();
    assert_eq!(
        media,
        vec![
            ("clip.mp4".to_owned(), b"video".to_vec()),
            ("pics/logo.png".to_owned(), b"picture".to_vec()),
            ("voice.mp3".to_owned(), b"audio".to_vec()),
        ]
    );
    let archive = dir.0.join("converted.ccraft");
    save_to(&archive, &course, &media).unwrap();
    assert_eq!(open_from(&archive).unwrap().1, media);
}

#[test]
fn bare_json_skips_missing_and_unreadable_media() {
    let dir = LooseCourseDir::new("missing-media");
    let assets = dir.0.join("media");
    std::fs::create_dir_all(assets.join("directory.png")).unwrap();
    std::fs::write(assets.join("present.png"), b"present").unwrap();
    let path = dir.write_course(&[
        ("image", "missing.png"),
        ("image", "directory.png"),
        ("image", "present.png"),
    ]);

    let (_, media) = open_from(&path).unwrap();
    assert_eq!(media, vec![("present.png".to_owned(), b"present".to_vec())]);
}

#[test]
fn bare_json_skips_unsafe_media_paths() {
    let dir = LooseCourseDir::new("unsafe-media");
    std::fs::create_dir(dir.0.join("media")).unwrap();
    std::fs::write(dir.0.join("secret.png"), b"outside media").unwrap();
    let path = dir.write_course(&[("image", "../secret.png")]);

    let (_, media) = open_from(&path).unwrap();
    assert!(media.is_empty());
}

#[test]
fn bare_json_opens_without_a_media_folder() {
    let dir = LooseCourseDir::new("no-media");
    let path = dir.write_course(&[("image", "logo.png")]);

    let (course, media) = open_from(&path).unwrap();
    assert_eq!(course.title, "Loose course");
    assert!(media.is_empty());
}

#[test]
fn a_dead_process_temp_file_is_removed_at_the_next_save_and_others_stay() {
    let dir = scratch_dir("test-stale-temps");
    let path = dir.join("p.ccraft");
    save_to(&path, &course_titled("a"), &[]).unwrap();

    // A process id far above any real one: nobody is running it.
    std::fs::write(dir.join("p.ccraft.tmp-999999999"), b"half").unwrap();
    // Not a pid suffix, another project's temp, and one of our own: all stay.
    std::fs::write(dir.join("p.ccraft.tmp-notes"), b"keep").unwrap();
    std::fs::write(dir.join("other.ccraft.tmp-999999999"), b"keep").unwrap();
    let own = format!("p.ccraft.tmp-{}", std::process::id());
    std::fs::write(dir.join(&own), b"live").unwrap();

    // Opening ignores them.
    assert_eq!(open_from(&path).unwrap().0.title, "a");
    assert!(dir.join("p.ccraft.tmp-999999999").exists());

    remove_stale_temps(&path);
    assert!(!dir.join("p.ccraft.tmp-999999999").exists());
    assert!(dir.join("p.ccraft.tmp-notes").exists());
    assert!(dir.join("other.ccraft.tmp-999999999").exists());
    assert!(dir.join(&own).exists());
}

// ---------------------------------------------------------------- folders

fn media_items() -> Vec<(String, Vec<u8>)> {
    vec![
        ("logo.png".to_string(), vec![1u8; 40]),
        ("fonts/f.ttf".to_string(), vec![2u8; 30]),
    ]
}

#[test]
fn a_folder_project_saves_opens_and_notices_outside_changes_by_name() {
    use crate::files::{open_folder_from, save_folder_to};
    use coursecraft_format::FolderRecord;

    let dir = scratch_dir("test-folder-project");
    let path = dir.join("p.ccraft-folder");
    let mut record = FolderRecord::new();
    save_folder_to(&path, &course_titled("mine"), &media_items(), &mut record).unwrap();

    let ((course, media), opened_record) = open_folder_from(&path).unwrap();
    assert_eq!(course.title, "mine");
    assert_eq!(media.len(), 2);
    let mut project = Project::new(0);
    project.path = Some(path.clone());
    project.folder = Some(opened_record);
    assert!(project.is_folder());
    assert!(!project.changed_on_disk());

    // A teammate replaces media/logo.png through git.
    std::fs::write(path.join("media/logo.png"), b"their new logo, longer").unwrap();
    assert!(project.changed_on_disk());
    assert_eq!(project.changed_files(), vec!["media/logo.png".to_string()]);

    // open_from reads a folder too, for places that need no record.
    assert_eq!(open_from(&path).unwrap().0.title, "mine");
}

#[test]
fn saving_a_folder_project_asks_first_and_keeps_no_versions() {
    use crate::files::save_folder_to;
    use coursecraft_format::FolderRecord;

    let dir = scratch_dir("test-folder-conflict-flow");
    let path = dir.join("p.ccraft-folder");
    let mut record = FolderRecord::new();
    save_folder_to(&path, &course_titled("first"), &media_items(), &mut record).unwrap();

    let mut app = crate::EditorApp::new_desktop(course_titled("first"), None);
    let config = dir.join("config");
    std::fs::create_dir_all(&config).unwrap();
    app.config_dir = Some(config.clone());
    app.finish_open(&path, Some(path.clone()), false);
    assert!(app.project.is_folder());
    app.editor.session.course.title = "edited here".into();

    std::fs::write(path.join("media/logo.png"), b"teammate's logo, new size").unwrap();
    let theirs = std::fs::read(path.join("media/logo.png")).unwrap();

    assert!(!app.perform_save(), "save is held back");
    let prompt = app.conflict_prompt.clone().expect("prompt");
    assert_eq!(prompt.files, vec!["media/logo.png".to_string()]);
    let sentence = crate::changed_files_message("p.ccraft-folder", &prompt.files);
    assert!(sentence.contains("media/logo.png"), "{sentence}");
    assert_eq!(std::fs::read(path.join("media/logo.png")).unwrap(), theirs);
    assert!(!open_from(&path).unwrap().0.title.contains("edited"));

    assert!(app.save_project_file(true), "Overwrite saves");
    assert_eq!(open_from(&path).unwrap().0.title, "edited here");
    assert!(!app.project.changed_on_disk());
    assert!(!config.join("versions").exists(), "git is the history");
    assert!(app.project.is_folder());
}

#[test]
fn many_changed_files_are_listed_with_a_count() {
    let files: Vec<String> = (0..12).map(|n| format!("media/{n}.png")).collect();
    let text = crate::changed_files_message("p", &files);
    assert!(text.contains("media/0.png") && text.contains("media/7.png"));
    assert!(!text.contains("media/8.png"));
    assert!(text.contains("and 4 more"), "{text}");
}

#[test]
fn a_too_big_folder_gets_the_same_plain_sentence_as_a_zip() {
    let zip = crate::files::plain_error(&coursecraft_format::FormatError::TooLarge);
    let folder = crate::files::plain_error(&coursecraft_format::FormatError::EntryTooLarge(
        "media/x.mp4".into(),
    ));
    assert!(zip.starts_with("this project is too big to open again"));
    assert!(folder.starts_with("this project is too big to open again"));
    assert!(folder.contains("up to 4 GB"));
}

#[test]
fn saving_a_bare_json_project_writes_pretty_json_not_zip() {
    let dir = scratch_dir("test-save-bare-json");
    let path = dir.join("course.JSON");
    let initial = course_titled("before");
    let json = initial.to_json().unwrap();
    std::fs::write(&path, format!("{json}\n")).unwrap();

    let mut app = crate::EditorApp::new_desktop(course_titled("before"), None);
    app.finish_open(&path, Some(path.clone()), false);
    app.editor.session.course.title = "after".into();

    assert!(app.save_project_file(true));
    let raw = std::fs::read_to_string(&path).expect("saved file must be UTF-8 JSON");
    assert!(!raw.starts_with("PK"), "must not write zip bytes");
    let parsed = coursecraft_course::Course::from_json(&raw).expect("valid course JSON");
    assert_eq!(parsed.title, "after");
}

#[test]
fn control_save_on_open_folder_without_extension_keeps_folder_record() {
    use coursecraft_control::ControlHost;
    use coursecraft_format::FolderRecord;

    let dir = scratch_dir("test-control-save-plain-folder");
    let folder = dir.join("hello-course");
    let mut initial = course_titled("first");
    let mut img = initial.scenes[0].slides[0].objects[0].clone();
    img.id = "logo".to_owned();
    img.kind = coursecraft_course::ObjectKind::Image {
        src: "logo.png".to_owned(),
        alt: "Logo".to_owned(),
        crop: None,
        outline: None,
        shape: None,
    };
    initial.scenes[0].slides[0].objects.push(img);
    let mut record = FolderRecord::new();
    crate::files::save_folder_to(&folder, &initial, &media_items(), &mut record).unwrap();

    let mut app = crate::EditorApp::new_desktop(course_titled("first"), None);
    app.finish_open(&folder, Some(folder.clone()), false);
    assert!(app.project.is_folder());
    app.editor.session.course.title = "updated".into();

    let mut adapter = crate::EditorHostAdapter {
        editor: &mut app.editor,
        project: &mut app.project,
        config_dir: app.config_dir.as_deref(),
        prefs: &mut app.prefs,
        save_epoch: &app.save_epoch,
    };
    let saved_path = ControlHost::save(&mut adapter, None).expect("control save succeeds");
    assert_eq!(saved_path, folder);
    assert!(
        app.project.is_folder(),
        "project.folder must not be cleared"
    );
    assert_eq!(open_from(&folder).unwrap().0.title, "updated");
    assert!(folder.join("media/logo.png").exists());
}

#[test]
fn save_kind_chooses_json_for_json_path_without_media() {
    use crate::{SaveKind, save_kind};
    assert_eq!(
        save_kind(Path::new("course.json"), false, false),
        SaveKind::Json
    );
    assert_eq!(
        save_kind(Path::new("course.JSON"), false, false),
        SaveKind::Json
    );
}

#[test]
fn save_kind_requests_save_as_for_json_path_with_media() {
    use crate::{SaveKind, save_kind};
    assert_eq!(
        save_kind(Path::new("course.json"), false, true),
        SaveKind::JsonNeedsSaveAs
    );
}

#[test]
fn save_kind_chooses_ccraft_for_ccraft_path_even_with_media() {
    use crate::{SaveKind, save_kind};
    assert_eq!(
        save_kind(Path::new("deck.ccraft"), false, true),
        SaveKind::Ccraft
    );
}

#[test]
fn save_kind_chooses_folder_when_open_project_is_folder() {
    use crate::{SaveKind, save_kind};
    assert_eq!(
        save_kind(Path::new("examples/hello-course"), true, true),
        SaveKind::Folder
    );
}

#[test]
fn control_save_is_folder_matches_open_folder_without_extension() {
    use crate::control_save_is_folder;
    use coursecraft_format::FolderRecord;

    let mut project = Project::new(0);
    let dir = PathBuf::from("/tmp/hello-course");
    project.path = Some(dir.clone());
    project.folder = Some(FolderRecord::new());

    assert!(control_save_is_folder(&dir, &project));
    assert!(!control_save_is_folder(
        Path::new("/tmp/other.json"),
        &project
    ));
    assert!(control_save_is_folder(
        Path::new("/tmp/new.ccraft-folder"),
        &project
    ));
}

#[test]
fn m31_07_attach_editor_host_adapter_ccraft_round_trip_keeps_media() {
    use coursecraft_control::ControlHost;

    let dir = scratch_dir("m31-07-attach-ccraft-media");
    let ccraft_path = dir.join("attach-deck.ccraft");

    let mut initial = course_titled("Attach Initial");
    let mut img = initial.scenes[0].slides[0].objects[0].clone();
    img.id = "logo".to_owned();
    img.kind = coursecraft_course::ObjectKind::Image {
        src: "logo.png".to_owned(),
        alt: "Logo".to_owned(),
        crop: None,
        outline: None,
        shape: None,
    };
    initial.scenes[0].slides[0].objects.push(img);

    let media = vec![("logo.png".to_owned(), vec![1_u8; 40])];
    crate::files::save_to(&ccraft_path, &initial, &media).expect("save initial ccraft");

    let mut app = crate::EditorApp::new_desktop(course_titled("Blank"), None);
    let mut adapter = crate::EditorHostAdapter {
        editor: &mut app.editor,
        project: &mut app.project,
        config_dir: app.config_dir.as_deref(),
        prefs: &mut app.prefs,
        save_epoch: &app.save_epoch,
    };

    let opened_title =
        ControlHost::open(&mut adapter, &ccraft_path, true).expect("attach open ccraft");
    assert_eq!(opened_title, "Attach Initial");

    ControlHost::run(
        &mut adapter,
        "slide.add",
        &serde_json::json!({ "title": "Attach Edited Slide" }),
    )
    .expect("attach edit");

    let saved_path = ControlHost::save(&mut adapter, None).expect("attach save");
    assert_eq!(saved_path, ccraft_path);

    let (reopened_course, reopened_media) = open_from(&ccraft_path).expect("reopen ccraft");
    assert!(
        reopened_course.scenes[0]
            .slides
            .iter()
            .any(|s| s.title == "Attach Edited Slide")
    );
    assert_eq!(reopened_media, media);
}

// ---------------------------------------------------------------- U-07 background autosave

/// U-07: guarded autosave writes a valid, verifiable `.ccraft` when `save_epoch` matches
/// and returns `Ok(false)` without touching disk when `save_epoch` is already stale.
#[test]
fn u07_guarded_autosave_writes_valid_verifiable_ccraft() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};

    let dir = scratch_dir("u07-guarded-autosave-valid");
    let auto_path = dir.join("deck.autosave.ccraft");
    let course = course_titled("Guarded Autosave");
    let media: Vec<(String, Arc<[u8]>)> =
        vec![("logo.png".to_owned(), Arc::from(b"png-bytes".as_slice()))];
    let save_epoch = Arc::new(AtomicU64::new(7));

    let committed =
        crate::autosave::write_autosave_arc_guarded(&course, &media, &auto_path, &save_epoch, 7)
            .expect("guarded autosave succeeds");
    assert!(committed, "matching epoch must commit");
    assert!(auto_path.exists());

    let file = std::fs::File::open(&auto_path).expect("open autosave");
    coursecraft_format::verify_project(std::io::BufReader::new(file))
        .expect("autosave passes verify_project");

    let (loaded_course, loaded_media) = open_from(&auto_path).expect("reopen autosave");
    assert_eq!(loaded_course.title, "Guarded Autosave");
    assert_eq!(
        loaded_media,
        vec![("logo.png".to_owned(), b"png-bytes".to_vec())]
    );
    assert_eq!(names_in(&dir), vec!["deck.autosave.ccraft".to_string()]);

    // Pre-write stale epoch check skips writing entirely.
    save_epoch.fetch_add(1, Ordering::AcqRel);
    let stale_path = dir.join("stale.autosave.ccraft");
    let committed_stale =
        crate::autosave::write_autosave_arc_guarded(&course, &media, &stale_path, &save_epoch, 7)
            .expect("stale guarded autosave returns cleanly");
    assert!(!committed_stale, "stale epoch must abort");
    assert!(!stale_path.exists());
}

/// U-07 / RISK-07: advancing `save_epoch` mid-write aborts before rename, leaves no
/// autosave file or temp file behind, and never replaces an existing autosave.
#[test]
fn u07_guarded_autosave_aborts_when_save_epoch_advances_mid_write() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};

    // Case 1: no existing autosave file — must neither create target nor leave temp.
    let dir = scratch_dir("u07-guarded-autosave-abort-new");
    let auto_path = dir.join("deck.autosave.ccraft");
    let course = course_titled("Mid-Write Race");
    let media: Vec<(String, Arc<[u8]>)> =
        vec![("a.png".to_owned(), Arc::from(vec![42u8; 4096].as_slice()))];
    let save_epoch = Arc::new(AtomicU64::new(3));
    let epoch_in_hook = Arc::clone(&save_epoch);
    let after_write = move |tmp: &Path| -> std::io::Result<()> {
        assert!(
            tmp.exists(),
            "temp file exists mid-write before commit check"
        );
        epoch_in_hook.fetch_add(1, Ordering::AcqRel);
        Ok(())
    };

    let committed = crate::autosave::write_autosave_slices_guarded_with_after_write(
        &course,
        &media,
        &auto_path,
        &save_epoch,
        3,
        &after_write,
    )
    .expect("aborted write returns Ok(false)");
    assert!(!committed, "mid-write epoch bump must abort commit");
    assert!(!auto_path.exists(), "autosave file must not be created");
    assert!(names_in(&dir).is_empty(), "no temp file may remain");

    // Case 2: existing autosave file — must not be replaced when epoch advances mid-write.
    let dir2 = scratch_dir("u07-guarded-autosave-abort-existing");
    let auto_path2 = dir2.join("deck.autosave.ccraft");
    let save_epoch2 = Arc::new(AtomicU64::new(5));
    assert!(
        crate::autosave::write_autosave_arc_guarded(
            &course_titled("Before"),
            &media,
            &auto_path2,
            &save_epoch2,
            5,
        )
        .expect("initial autosave")
    );
    let before_bytes = std::fs::read(&auto_path2).expect("read existing autosave");

    let epoch2_in_hook = Arc::clone(&save_epoch2);
    let bump_mid_write = move |_tmp: &Path| -> std::io::Result<()> {
        epoch2_in_hook.fetch_add(1, Ordering::AcqRel);
        Ok(())
    };
    let committed2 = crate::autosave::write_autosave_slices_guarded_with_after_write(
        &course_titled("Stale After"),
        &media,
        &auto_path2,
        &save_epoch2,
        5,
        &bump_mid_write,
    )
    .expect("aborted replacement returns Ok(false)");
    assert!(!committed2);
    assert_eq!(
        std::fs::read(&auto_path2).expect("existing autosave still readable"),
        before_bytes,
        "existing autosave must not be replaced"
    );
    assert_eq!(
        names_in(&dir2),
        vec!["deck.autosave.ccraft".to_string()],
        "no temp file may remain"
    );
}

/// U-07: overlapping autosave triggers while a background write is in flight are
/// coalesced without spawning a second write, and completion updates
/// `last_autosaved_revision` cleanly (while ignoring stale epochs after a manual save).
#[test]
fn u07_background_autosave_coalesces_overlapping_triggers() {
    use std::sync::atomic::Ordering;

    let dir = scratch_dir("u07-bg-autosave-coalesce");
    let project_path = dir.join("deck.ccraft");
    let mut app = crate::EditorApp::new_desktop(course_titled("Initial"), Some(dir.clone()));
    app.project.path = Some(project_path.clone());
    app.project.saved_revision = 1;
    app.editor.session.revision = 5;

    let ctx = egui::Context::default();
    let step_autosave = |app: &mut crate::EditorApp, time: f64| {
        ctx.begin_pass(egui::RawInput {
            time: Some(time),
            ..Default::default()
        });
        app.update_autosave(&ctx);
        ctx.end_pass().drop_without_applying_deltas();
    };

    // Part A: simulate an in-flight background autosave; repeated due ticks must coalesce.
    let (tx, rx) = std::sync::mpsc::channel();
    let epoch = app.save_epoch.load(Ordering::Acquire);
    app.autosave_rx = Some(rx);

    step_autosave(&mut app, 120.0);
    step_autosave(&mut app, 240.0);
    assert!(app.autosave_rx.is_some(), "in-flight receiver stays active");
    assert_eq!(
        app.autosaver.last_autosave_time, 0.0,
        "overlapping ticks while in flight must not consume take_due"
    );
    assert_eq!(app.autosaver.last_autosaved_revision, None);

    // Finish the in-flight autosave and poll once.
    tx.send(crate::AutosaveOutcome {
        revision: 5,
        epoch,
        result: Ok(true),
    })
    .expect("send outcome");
    step_autosave(&mut app, 120.0);
    assert!(app.autosave_rx.is_none(), "completed receiver is cleared");
    assert_eq!(app.autosaver.last_autosaved_revision, Some(5));

    // Part B: a new revision past the interval spawns a real background autosave worker
    // and writes a verifiable `.autosave.ccraft` without blocking the main thread.
    app.editor.session.revision = 6;
    app.editor.session.course.title = "Edited Rev 6".to_owned();
    step_autosave(&mut app, 250.0);
    assert!(
        app.autosave_rx.is_some(),
        "background autosave thread spawned"
    );

    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while app.autosave_rx.is_some() && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
        step_autosave(&mut app, 251.0);
    }
    assert!(
        app.autosave_rx.is_none(),
        "background autosave finished within timeout"
    );
    assert_eq!(app.autosaver.last_autosaved_revision, Some(6));

    let auto_path = crate::autosave::autosave_path_for_project(Some(&project_path), &dir);
    assert!(auto_path.exists());
    let file = std::fs::File::open(&auto_path).expect("open autosave");
    coursecraft_format::verify_project(std::io::BufReader::new(file))
        .expect("background autosave passes verify_project");

    // Part C: manual save bumps save_epoch, deletes autosave, and causes any stale
    // completed outcome to be ignored.
    let epoch_before_save = app.save_epoch.load(Ordering::Acquire);
    let (stale_tx, stale_rx) = std::sync::mpsc::channel();
    app.autosave_rx = Some(stale_rx);
    assert!(app.perform_save());
    assert!(
        app.save_epoch.load(Ordering::Acquire) > epoch_before_save,
        "manual save must advance save_epoch"
    );
    assert!(!auto_path.exists(), "manual save removes autosave file");

    stale_tx
        .send(crate::AutosaveOutcome {
            revision: 99,
            epoch: epoch_before_save,
            result: Ok(true),
        })
        .expect("send stale outcome");
    step_autosave(&mut app, 260.0);
    assert!(app.autosave_rx.is_none());
    assert_ne!(
        app.autosaver.last_autosaved_revision,
        Some(99),
        "stale epoch outcome must not overwrite last_autosaved_revision"
    );
}
