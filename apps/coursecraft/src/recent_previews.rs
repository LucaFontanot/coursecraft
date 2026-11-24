//! Thumbnails for the start page's recent courses: each recent file's
//! `course.json` is read once, off the UI thread, and handed to the editor.
//! Media is not loaded, so pictures show as their placeholder.

use std::collections::HashSet;
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender, channel};

use coursecraft_course::Course;
use coursecraft_ui_egui::Editor;

/// Largest `course.json` read for a thumbnail.
const MAX_JSON_BYTES: u64 = 32 * 1024 * 1024;

pub struct RecentPreviews {
    /// Paths already asked for (read or being read).
    asked: HashSet<String>,
    tx: Sender<(String, Course)>,
    rx: Receiver<(String, Course)>,
}

impl Default for RecentPreviews {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self {
            asked: HashSet::new(),
            tx,
            rx,
        }
    }
}

impl RecentPreviews {
    /// Read the recent courses not asked for yet, and hand finished ones to
    /// the editor. Does nothing while the start page is hidden.
    pub fn update(&mut self, editor: &mut Editor, ctx: &egui::Context) {
        while let Ok((path, course)) = self.rx.try_recv() {
            if editor.recent_files.contains(&path) {
                editor.set_recent_preview(path, &course);
            }
        }
        if !editor.start_page_shown() {
            return;
        }
        let new: Vec<String> = editor
            .recent_files
            .iter()
            .filter(|p| !self.asked.contains(*p))
            .cloned()
            .collect();
        if new.is_empty() {
            return;
        }
        self.asked.extend(new.iter().cloned());
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        let spawned = std::thread::Builder::new()
            .name("recent-previews".into())
            .spawn(move || {
                for path in new {
                    if let Some(course) = read_course(Path::new(&path))
                        && tx.send((path, course)).is_err()
                    {
                        return;
                    }
                    ctx.request_repaint();
                }
            });
        if spawned.is_err() {
            // No thread: the rows keep their blank card.
        }
    }
}

/// The course of a project file, a project folder or a `.json` file.
pub fn read_course(path: &Path) -> Option<Course> {
    if path.is_dir() {
        return read_json(&path.join("course.json"));
    }
    if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
    {
        return read_json(path);
    }
    let file = std::fs::File::open(path).ok()?;
    let reader = std::io::BufReader::with_capacity(64 * 1024, file);
    coursecraft_format::read_course(reader).ok()
}

fn read_json(path: &Path) -> Option<Course> {
    let len = std::fs::metadata(path).ok()?.len();
    if len > MAX_JSON_BYTES {
        return None;
    }
    let text = std::fs::read_to_string(path).ok()?;
    Course::from_json(&text).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const HELLO: &str = include_str!("../../../examples/hello-course/course.json");

    #[test]
    fn reads_json_folders_and_projects_and_skips_the_rest() {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/agy-scratch/recent-previews");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let json = dir.join("a.json");
        std::fs::write(&json, HELLO).expect("write json");
        assert!(read_course(&json).is_some());
        let folder = dir.join("proj");
        std::fs::create_dir(&folder).expect("folder");
        std::fs::write(folder.join("course.json"), HELLO).expect("write");
        assert!(read_course(&folder).is_some());
        let junk = dir.join("junk.ccraft");
        std::fs::write(&junk, b"not a zip").expect("write junk");
        assert!(read_course(&junk).is_none());
        assert!(read_course(&dir.join("missing.ccraft")).is_none());
    }
}
