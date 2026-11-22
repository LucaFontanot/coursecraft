use coursecraft_course::{Course, LmsStatus, PublishSettings};
use coursecraft_publish::{PackageFile, PackageInput, PackageStandard, build_package_standard};
use coursecraft_scorm::Standard;

fn hello_course() -> Course {
    let json = include_str!("../../../examples/hello-course/course.json");
    Course::from_json(json).expect("hello course")
}

#[test]
fn published_scorm12_package_preserves_lms_status_in_course_json() {
    let mut course = hello_course();
    course.editor.last_publish = Some(PublishSettings {
        standard: "scorm12".to_string(),
        lms_status: LmsStatus::CompletedIncomplete,
    });
    let course_json = course.to_json().expect("serialize");
    let player_files = vec![PackageFile {
        path: "index.html".to_string(),
        bytes: b"<!doctype html><html></html>".to_vec(),
    }];
    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &player_files,
        media: &|_| None,
    };

    let files = build_package_standard(&input, PackageStandard::Scorm(Standard::Scorm12))
        .expect("build scorm12 package");
    let packaged_course = files
        .iter()
        .find(|f| f.path == "course.json")
        .expect("course.json in package");
    let reparsed =
        Course::from_json(std::str::from_utf8(&packaged_course.bytes).expect("utf8 course.json"))
            .expect("valid packaged course.json");

    assert_eq!(
        reparsed.editor.last_publish.as_ref().map(|p| p.lms_status),
        Some(LmsStatus::CompletedIncomplete)
    );
}
