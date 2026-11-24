use super::*;
use coursecraft_course::{
    BACKGROUND_AUDIO_ID, BackgroundAudio, Object, ObjectKind, PlayerSettings, Rect, Scene, Size,
    Slide, StateDef, Timing,
};
use std::io::Read as _;

const HELLO_COURSE_JSON: &str = include_str!("../../../examples/hello-course/course.json");

fn minimal_course(title: &str) -> Course {
    Course {
        schema_version: 1,
        id: "test-course".into(),
        title: title.into(),
        slide_size: Size::default(),
        theme: coursecraft_course::Theme::default(),
        variables: Vec::new(),
        scenes: vec![Scene {
            id: "scene1".into(),
            title: "Scene 1".into(),
            slides: vec![Slide {
                id: "slide1".into(),
                title: "Slide 1".into(),
                duration_ms: 5000,
                background: None,
                objects: Vec::new(),
                layers: Vec::new(),
                triggers: Vec::new(),
                question: None,
                draw: None,
                transition: None,
                notes: String::new(),
                master: None,
                revisit: coursecraft_course::RevisitMode::Auto,
                player: None,
                cue_points: Vec::new(),
            }],
        }],
        masters: Vec::new(),
        templates: Vec::new(),
        banks: Vec::new(),
        start_slide: None,
        quiz: None,
        player: PlayerSettings::default(),
        language: None,
        strings: std::collections::BTreeMap::new(),
        fonts: Vec::new(),
        background_audio: None,
        glossary: Vec::new(),
        editor: Default::default(),
        resources: Vec::new(),
        text_styles: Vec::new(),
    }
}

fn fake_player() -> Vec<PackageFile> {
    vec![
        PackageFile {
            path: "index.html".into(),
            bytes: b"<!doctype html><html><body>fake player</body></html>".to_vec(),
        },
        PackageFile {
            path: "app.js".into(),
            bytes: b"console.log('app');".to_vec(),
        },
        PackageFile {
            path: "app_bg.wasm".into(),
            bytes: b"\0asm fake wasm binary".to_vec(),
        },
        PackageFile {
            path: "course.json".into(),
            bytes: b"{\"stale\": true}".to_vec(),
        },
    ]
}

fn web_object(id: &str, path: &str, entry: &str) -> Object {
    Object {
        id: id.into(),
        name: "Site".into(),
        kind: ObjectKind::Web {
            source: coursecraft_course::WebSource::Folder {
                path: path.into(),
                entry: entry.into(),
            },
        },
        frame: Rect {
            x: 0.0,
            y: 0.0,
            w: 640.0,
            h: 360.0,
        },
        timing: Timing::default(),
        entrance: None,
        exit: None,
        motion_paths: Vec::new(),
        states: Vec::<StateDef>::new(),
        initial_state: "normal".into(),
        focus_order: None,
        decorative: false,
        description: None,
        group: None,
        panel: None,
        rotation: 0.0,
        flip_h: false,
        flip_v: false,
        opacity: 1.0,
        shadow: None,
        glow: None,
        drag: None,
        locked: false,
        editor_hidden: false,
    }
}

#[test]
fn background_audio_is_packaged() {
    let mut course = minimal_course("Background audio");
    course.background_audio = Some(BackgroundAudio {
        src: "music.mp3".into(),
        volume: 0.5,
        looping: true,
    });
    let player_files = fake_player();
    let course_json = course.to_json().expect("course json");
    let media =
        |src: &str| -> Option<Vec<u8>> { (src == "music.mp3").then(|| b"audio bytes".to_vec()) };
    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &player_files,
        media: &media,
    };
    let files = build_package(&input).expect("package builds");
    let paths: Vec<&str> = files.iter().map(|file| file.path.as_str()).collect();
    assert!(
        paths.contains(&"media/music.mp3"),
        "background track missing from {paths:?}"
    );
}

#[test]
fn missing_background_audio_is_reported() {
    let mut course = minimal_course("Background audio");
    course.background_audio = Some(BackgroundAudio {
        src: "music.mp3".into(),
        volume: 0.5,
        looping: true,
    });
    let player_files = fake_player();
    let course_json = course.to_json().expect("course json");
    let media = |_src: &str| -> Option<Vec<u8>> { None };
    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &player_files,
        media: &media,
    };
    let error = build_package(&input).expect_err("missing background audio must fail");
    assert_eq!(
        error,
        PublishError::MissingMedia {
            object: BACKGROUND_AUDIO_ID.into(),
            src: "music.mp3".into(),
        }
    );
}

#[test]
fn web_folder_files_keep_their_relative_paths_in_the_package() {
    let mut course = minimal_course("Web folder");
    course.scenes[0].slides[0]
        .objects
        .push(web_object("web1", "web/site", "index.html"));
    let player_files = fake_player();
    let course_json = course.to_json().expect("course json");
    let media = |src: &str| -> Option<Vec<u8>> {
        (src == "web/site/index.html").then(|| b"<html>site</html>".to_vec())
    };
    let folder = |path: &str| -> Option<Vec<(String, Vec<u8>)>> {
        (path == "web/site").then(|| {
            vec![
                ("web/site/index.html".into(), b"<html>site</html>".to_vec()),
                ("web/site/style.css".into(), b"body{}".to_vec()),
                ("web/site/assets/app.js".into(), b"run()".to_vec()),
            ]
        })
    };
    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &player_files,
        media: &media,
    };
    let files = build_package_standard_with_folders(
        &input,
        PackageStandard::Scorm(Standard::Scorm12),
        &folder,
    )
    .expect("package builds with folder");
    let paths: Vec<&str> = files.iter().map(|file| file.path.as_str()).collect();
    for path in [
        "media/web/site/index.html",
        "media/web/site/style.css",
        "media/web/site/assets/app.js",
    ] {
        assert!(paths.contains(&path), "{path} missing from {paths:?}");
    }
    let manifest = files
        .iter()
        .find(|file| file.path == "imsmanifest.xml")
        .expect("manifest");
    let manifest = std::str::from_utf8(&manifest.bytes).expect("manifest utf8");
    assert!(manifest.contains("media/web/site/assets/app.js"));
}

#[test]
fn missing_web_folder_entry_is_reported() {
    let mut course = minimal_course("Web folder");
    course.scenes[0].slides[0]
        .objects
        .push(web_object("web1", "web/site", "index.html"));
    let player_files = fake_player();
    let course_json = course.to_json().expect("course json");
    let media = |_src: &str| -> Option<Vec<u8>> { None };
    let folder = |path: &str| -> Option<Vec<(String, Vec<u8>)>> {
        (path == "web/site").then(|| vec![("web/site/other.html".into(), b"x".to_vec())])
    };
    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &player_files,
        media: &media,
    };
    let error = build_package_standard_with_folders(
        &input,
        PackageStandard::Scorm(Standard::Scorm12),
        &folder,
    )
    .expect_err("missing entry must fail");
    assert!(
        matches!(error, PublishError::MissingMedia { ref src, .. } if src == "web/site/index.html"),
        "{error:?}"
    );
}

#[test]
fn hello_course_packaged_with_fake_player() {
    let course = Course::from_json(HELLO_COURSE_JSON).expect("valid course");
    let no_media = |_src: &str| -> Option<Vec<u8>> { None };

    let input = PackageInput {
        course: &course,
        course_json: HELLO_COURSE_JSON,
        player_files: &fake_player(),
        media: &no_media,
    };

    let files = build_package(&input).expect("package builds");

    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        paths,
        vec![
            "app.js",
            "app_bg.wasm",
            "course.json",
            "imsmanifest.xml",
            "index.html"
        ]
    );

    // course.json is ours, replacing the stale one
    let course_file = files
        .iter()
        .find(|f| f.path == "course.json")
        .expect("course.json present");
    assert_eq!(course_file.bytes, HELLO_COURSE_JSON.as_bytes());

    // imsmanifest.xml lists all files and has mastery score 80
    let manifest_file = files
        .iter()
        .find(|f| f.path == "imsmanifest.xml")
        .expect("imsmanifest.xml present");
    let xml = std::str::from_utf8(&manifest_file.bytes).expect("valid utf-8 manifest");

    assert!(xml.contains("<adlcp:masteryscore>80</adlcp:masteryscore>"));
    assert!(xml.contains("<file href=\"app.js\"/>"));
    assert!(xml.contains("<file href=\"app_bg.wasm\"/>"));
    assert!(xml.contains("<file href=\"course.json\"/>"));
    assert!(!xml.contains("<file href=\"imsmanifest.xml\"/>"));
    assert!(xml.contains("<file href=\"index.html\"/>"));
}

#[test]
fn course_with_image_packaged_under_media() {
    let mut course = minimal_course("Image Course");
    course.scenes[0].slides[0].objects.push(Object {
        id: "img1".into(),
        name: "Picture".into(),
        kind: ObjectKind::Image {
            crop: None,
            src: "media/pic.png".into(),
            alt: "A picture".into(),
            outline: None,
            shape: None,
        },
        frame: Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        },
        timing: Timing::default(),
        states: Vec::new(),
        initial_state: "normal".into(),
        entrance: None,
        exit: None,
        motion_paths: Vec::new(),
        focus_order: None,
        decorative: false,
        description: None,
        group: None,
        panel: None,
        rotation: 0.0,
        flip_h: false,
        flip_v: false,
        opacity: 1.0,
        shadow: None,
        glow: None,
        drag: None,
        locked: false,
        editor_hidden: false,
    });

    let fake_png = vec![0x89, 0x50, 0x4e, 0x47];
    let media = |src: &str| -> Option<Vec<u8>> {
        if src == "media/pic.png" {
            Some(fake_png.clone())
        } else {
            None
        }
    };

    let course_json = course.to_json().expect("serialize");
    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &fake_player(),
        media: &media,
    };

    let files = build_package(&input).expect("package builds");
    let media_file = files
        .iter()
        .find(|f| f.path == "media/media/pic.png")
        .expect("media file packaged");
    assert_eq!(media_file.bytes, fake_png);

    let manifest_file = files
        .iter()
        .find(|f| f.path == "imsmanifest.xml")
        .expect("manifest present");
    let xml = std::str::from_utf8(&manifest_file.bytes).expect("utf8");
    assert!(xml.contains("<file href=\"media/media/pic.png\"/>"));
}

#[test]
fn course_with_svg_image_packaged_under_media() {
    let mut course = minimal_course("SVG Course");
    course.scenes[0].slides[0].objects.push(Object {
        id: "svg1".into(),
        name: "Vector".into(),
        kind: ObjectKind::Image {
            crop: None,
            src: "media/logo.svg".into(),
            alt: "Logo".into(),
            outline: None,
            shape: None,
        },
        frame: Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        },
        timing: Timing::default(),
        states: Vec::new(),
        initial_state: "normal".into(),
        entrance: None,
        exit: None,
        motion_paths: Vec::new(),
        focus_order: None,
        decorative: false,
        description: None,
        group: None,
        panel: None,
        rotation: 0.0,
        flip_h: false,
        flip_v: false,
        opacity: 1.0,
        shadow: None,
        glow: None,
        drag: None,
        locked: false,
        editor_hidden: false,
    });

    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="16" height="16" fill="#123456"/></svg>"##.to_vec();
    let media = |src: &str| -> Option<Vec<u8>> {
        if src == "media/logo.svg" {
            Some(svg.clone())
        } else {
            None
        }
    };

    let course_json = course.to_json().expect("serialize");
    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &fake_player(),
        media: &media,
    };
    let files = build_package(&input).expect("package builds");
    let media_file = files
        .iter()
        .find(|f| f.path == "media/media/logo.svg")
        .expect("SVG packaged under media/");
    assert_eq!(media_file.bytes, svg);

    let manifest = files
        .iter()
        .find(|f| f.path == "imsmanifest.xml")
        .expect("manifest present");
    let xml = std::str::from_utf8(&manifest.bytes).expect("utf8");
    assert!(xml.contains("<file href=\"media/media/logo.svg\"/>"));
}

#[test]
fn course_fonts_ship_as_media() {
    let mut course = minimal_course("Fonts");
    course.fonts.push(coursecraft_course::FontFace {
        family: "Lato".into(),
        style: coursecraft_course::FontStyle::Regular,
        src: "fonts/lato.ttf".into(),
    });
    let font_bytes = b"font-bytes".to_vec();
    let media =
        |src: &str| -> Option<Vec<u8>> { (src == "fonts/lato.ttf").then(|| font_bytes.clone()) };
    let course_json = course.to_json().expect("serialize");
    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &fake_player(),
        media: &media,
    };
    let files = build_package(&input).expect("package builds");
    let packaged = files
        .iter()
        .find(|f| f.path == "media/fonts/lato.ttf")
        .expect("font packaged under media/");
    assert_eq!(packaged.bytes, b"font-bytes");
    let manifest = files
        .iter()
        .find(|f| f.path == "imsmanifest.xml")
        .expect("manifest present");
    assert!(
        std::str::from_utf8(&manifest.bytes)
            .expect("utf8")
            .contains("media/fonts/lato.ttf")
    );
}

#[test]
fn missing_media_errors() {
    let mut course = minimal_course("Missing Media");
    course.scenes[0].slides[0].objects.push(Object {
        id: "img-missing".into(),
        name: "Missing Pic".into(),
        kind: ObjectKind::Image {
            crop: None,
            src: "media/pic.png".into(),
            alt: "".into(),
            outline: None,
            shape: None,
        },
        frame: Rect {
            x: 0.0,
            y: 0.0,
            w: 10.0,
            h: 10.0,
        },
        timing: Timing::default(),
        states: Vec::new(),
        initial_state: "normal".into(),
        entrance: None,
        exit: None,
        motion_paths: Vec::new(),
        focus_order: None,
        decorative: false,
        description: None,
        group: None,
        panel: None,
        rotation: 0.0,
        flip_h: false,
        flip_v: false,
        opacity: 1.0,
        shadow: None,
        glow: None,
        drag: None,
        locked: false,
        editor_hidden: false,
    });

    let no_media = |_src: &str| -> Option<Vec<u8>> { None };
    let course_json = course.to_json().expect("serialize");
    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &fake_player(),
        media: &no_media,
    };

    let err = build_package(&input).expect_err("must error on missing media");
    assert_eq!(
        err,
        PublishError::MissingMedia {
            object: "img-missing".into(),
            src: "media/pic.png".into(),
        }
    );
}

#[test]
fn each_unsafe_path_case_errors() {
    let unsafe_cases = [
        "",
        "/abs/file.png",
        "foo\\bar.png",
        "C:file.png",
        "ftp://host/file.png",
        "javascript:alert(1)",
        "foo//bar.png",
        "foo/bar/",
        "foo/./bar.png",
        ".",
        "foo/../bar.png",
        "..",
        "foo\0bar.png",
        "foo\nbar.png",
    ];

    for unsafe_src in unsafe_cases {
        let mut course = minimal_course("Unsafe Path");
        course.scenes[0].slides[0].objects.push(Object {
            id: "bad-obj".into(),
            name: "Bad".into(),
            kind: ObjectKind::Image {
                crop: None,
                src: unsafe_src.into(),
                alt: "".into(),
                outline: None,
                shape: None,
            },
            frame: Rect {
                x: 0.0,
                y: 0.0,
                w: 10.0,
                h: 10.0,
            },
            timing: Timing::default(),
            states: Vec::new(),
            initial_state: "normal".into(),
            entrance: None,
            exit: None,
            motion_paths: Vec::new(),
            focus_order: None,
            decorative: false,
            description: None,
            group: None,
            panel: None,
            rotation: 0.0,
            flip_h: false,
            flip_v: false,
            opacity: 1.0,
            shadow: None,
            glow: None,
            drag: None,
            locked: false,
            editor_hidden: false,
        });

        let dummy_media = |_src: &str| -> Option<Vec<u8>> { Some(vec![1, 2, 3]) };
        let course_json = course.to_json().expect("serialize");
        let input = PackageInput {
            course: &course,
            course_json: &course_json,
            player_files: &fake_player(),
            media: &dummy_media,
        };

        let err = build_package(&input).expect_err("must reject unsafe path");
        assert_eq!(
            err,
            PublishError::UnsafePath {
                object: "bad-obj".into(),
                src: unsafe_src.into(),
            },
            "failed on case: {unsafe_src:?}"
        );
    }

    // Also test unsafe player file path
    let course = minimal_course("Player Unsafe");
    let course_json = course.to_json().expect("serialize");
    let no_media = |_src: &str| -> Option<Vec<u8>> { None };

    let bad_player_files = vec![
        PackageFile {
            path: "index.html".into(),
            bytes: b"ok".to_vec(),
        },
        PackageFile {
            path: "../escape.js".into(),
            bytes: b"bad".to_vec(),
        },
    ];

    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &bad_player_files,
        media: &no_media,
    };

    let err = build_package(&input).expect_err("must reject unsafe player file");
    assert_eq!(
        err,
        PublishError::UnsafePath {
            object: "player".into(),
            src: "../escape.js".into(),
        }
    );
}

#[test]
fn missing_index_html_errors() {
    let course = minimal_course("No Index");
    let course_json = course.to_json().expect("serialize");
    let no_media = |_src: &str| -> Option<Vec<u8>> { None };

    let player_without_index = vec![PackageFile {
        path: "app.js".into(),
        bytes: b"console.log('app');".to_vec(),
    }];

    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &player_without_index,
        media: &no_media,
    };

    let err = build_package(&input).expect_err("missing index.html");
    assert_eq!(err, PublishError::MissingIndexHtml);
}

#[test]
fn read_zip_back_with_zip_archive() {
    let course = Course::from_json(HELLO_COURSE_JSON).expect("valid course");
    let no_media = |_src: &str| -> Option<Vec<u8>> { None };

    let input = PackageInput {
        course: &course,
        course_json: HELLO_COURSE_JSON,
        player_files: &fake_player(),
        media: &no_media,
    };

    let files = build_package(&input).expect("package builds");
    let zip_bytes = zip_package(&files).expect("zip builds");

    let cursor = std::io::Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(cursor).expect("valid zip archive");

    assert_eq!(archive.len(), files.len());
    for file in &files {
        let mut entry = archive.by_name(&file.path).expect("file in zip");
        let mut extracted = Vec::new();
        entry.read_to_end(&mut extracted).expect("read entry");
        assert_eq!(extracted, file.bytes);
    }
}

#[test]
fn deterministic_zip() {
    let course = Course::from_json(HELLO_COURSE_JSON).expect("valid course");
    let no_media = |_src: &str| -> Option<Vec<u8>> { None };

    let input = PackageInput {
        course: &course,
        course_json: HELLO_COURSE_JSON,
        player_files: &fake_player(),
        media: &no_media,
    };

    let files = build_package(&input).expect("package builds");
    let zip1 = zip_package(&files).expect("zip1");
    let zip2 = zip_package(&files).expect("zip2");

    assert_eq!(zip1, zip2, "zips must be byte-identical");
}

#[test]
fn dedupes_identical_media_sources() {
    let mut course = minimal_course("Dedupe");
    course.scenes[0].slides[0].objects.push(Object {
        id: "img1".into(),
        name: "Pic 1".into(),
        kind: ObjectKind::Image {
            crop: None,
            src: "media/logo.png".into(),
            alt: "".into(),
            outline: None,
            shape: None,
        },
        frame: Rect {
            x: 0.0,
            y: 0.0,
            w: 10.0,
            h: 10.0,
        },
        timing: Timing::default(),
        states: vec![StateDef {
            name: "hover".into(),
            fill: None,
            text: None,
            src: Some("media/logo.png".into()),
            ..Default::default()
        }],
        initial_state: "normal".into(),
        entrance: None,
        exit: None,
        motion_paths: Vec::new(),
        focus_order: None,
        decorative: false,
        description: None,
        group: None,
        panel: None,
        rotation: 0.0,
        flip_h: false,
        flip_v: false,
        opacity: 1.0,
        shadow: None,
        glow: None,
        drag: None,
        locked: false,
        editor_hidden: false,
    });
    course.scenes[0].slides[0].objects.push(Object {
        id: "img2".into(),
        name: "Pic 2".into(),
        kind: ObjectKind::Image {
            crop: None,
            src: "media/logo.png".into(),
            alt: "".into(),
            outline: None,
            shape: None,
        },
        frame: Rect {
            x: 10.0,
            y: 0.0,
            w: 10.0,
            h: 10.0,
        },
        timing: Timing::default(),
        states: Vec::new(),
        initial_state: "normal".into(),
        entrance: None,
        exit: None,
        motion_paths: Vec::new(),
        focus_order: None,
        decorative: false,
        description: None,
        group: None,
        panel: None,
        rotation: 0.0,
        flip_h: false,
        flip_v: false,
        opacity: 1.0,
        shadow: None,
        glow: None,
        drag: None,
        locked: false,
        editor_hidden: false,
    });

    let resolve_count = std::sync::atomic::AtomicUsize::new(0);
    let media = |src: &str| -> Option<Vec<u8>> {
        if src == "media/logo.png" {
            resolve_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Some(vec![9, 9, 9])
        } else {
            None
        }
    };

    let course_json = course.to_json().expect("serialize");
    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &fake_player(),
        media: &media,
    };

    let files = build_package(&input).expect("package builds");
    assert_eq!(
        resolve_count.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "identical media source resolved only once"
    );

    let logo_count = files
        .iter()
        .filter(|f| f.path == "media/media/logo.png")
        .count();
    assert_eq!(logo_count, 1, "packaged exactly once");
}

#[test]
fn skips_remote_http_and_https_sources() {
    let mut course = minimal_course("Remote Media");
    course.scenes[0].slides[0].objects.push(Object {
        id: "audio1".into(),
        name: "Remote Audio".into(),
        kind: ObjectKind::Audio {
            src: "https://example.com/stream.mp3".into(),
            autoplay: false,
            captions: None,
            volume: 1.0,
            trim: None,
        },
        frame: Rect {
            x: 0.0,
            y: 0.0,
            w: 10.0,
            h: 10.0,
        },
        timing: Timing::default(),
        states: Vec::new(),
        initial_state: "normal".into(),
        entrance: None,
        exit: None,
        motion_paths: Vec::new(),
        focus_order: None,
        decorative: false,
        description: None,
        group: None,
        panel: None,
        rotation: 0.0,
        flip_h: false,
        flip_v: false,
        opacity: 1.0,
        shadow: None,
        glow: None,
        drag: None,
        locked: false,
        editor_hidden: false,
    });
    course.scenes[0].slides[0].objects.push(Object {
        id: "video1".into(),
        name: "Remote Video".into(),
        kind: ObjectKind::Video {
            src: "http://example.com/video.mp4".into(),
            autoplay: false,
            captions: None,
            volume: 1.0,
            trim: None,
            cues: Vec::new(),
            poster: None,
        },
        frame: Rect {
            x: 0.0,
            y: 0.0,
            w: 10.0,
            h: 10.0,
        },
        timing: Timing::default(),
        states: Vec::new(),
        initial_state: "normal".into(),
        entrance: None,
        exit: None,
        motion_paths: Vec::new(),
        focus_order: None,
        decorative: false,
        description: None,
        group: None,
        panel: None,
        rotation: 0.0,
        flip_h: false,
        flip_v: false,
        opacity: 1.0,
        shadow: None,
        glow: None,
        drag: None,
        locked: false,
        editor_hidden: false,
    });

    let no_media = |_src: &str| -> Option<Vec<u8>> { None };
    let course_json = course.to_json().expect("serialize");
    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &fake_player(),
        media: &no_media,
    };

    let files = build_package(&input).expect("package builds");
    assert!(!files.iter().any(|f| f.path.contains("example.com")));
}

#[test]
fn detects_duplicate_files_in_player() {
    let course = minimal_course("Duplicates");
    let course_json = course.to_json().expect("serialize");
    let no_media = |_src: &str| -> Option<Vec<u8>> { None };

    let player_files = vec![
        PackageFile {
            path: "index.html".into(),
            bytes: b"1".to_vec(),
        },
        PackageFile {
            path: "app.js".into(),
            bytes: b"2".to_vec(),
        },
        PackageFile {
            path: "app.js".into(),
            bytes: b"3".to_vec(),
        },
    ];

    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &player_files,
        media: &no_media,
    };

    let err = build_package(&input).expect_err("duplicate in player files");
    assert_eq!(err, PublishError::DuplicateFile("app.js".into()));
}

#[test]
fn distinct_sources_never_collide_in_package() {
    let mut course = minimal_course("No Collide");
    for (id, src) in [("i1", "a.png"), ("i2", "media/a.png")] {
        course.scenes[0].slides[0].objects.push(Object {
            id: id.into(),
            name: id.into(),
            kind: ObjectKind::Image {
                crop: None,
                src: src.into(),
                alt: "".into(),
                outline: None,
                shape: None,
            },
            frame: Rect {
                x: 0.0,
                y: 0.0,
                w: 10.0,
                h: 10.0,
            },
            timing: Timing::default(),
            states: Vec::new(),
            initial_state: "normal".into(),
            entrance: None,
            exit: None,
            motion_paths: Vec::new(),
            focus_order: None,
            decorative: false,
            description: None,
            group: None,
            panel: None,
            rotation: 0.0,
            flip_h: false,
            flip_v: false,
            opacity: 1.0,
            shadow: None,
            glow: None,
            drag: None,
            locked: false,
            editor_hidden: false,
        });
    }
    let media = |src: &str| -> Option<Vec<u8>> {
        match src {
            "a.png" => Some(vec![1]),
            "media/a.png" => Some(vec![2]),
            _ => None,
        }
    };
    let course_json = course.to_json().expect("serialize");
    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &fake_player(),
        media: &media,
    };
    let files = build_package(&input).expect("package builds");
    let find = |p: &str| files.iter().find(|f| f.path == p).map(|f| f.bytes.clone());
    assert_eq!(find("media/a.png"), Some(vec![1]));
    assert_eq!(find("media/media/a.png"), Some(vec![2]));
}

#[test]
fn editor_saved_project_roundtrips_to_publish_package() {
    let mut course = minimal_course("Editor Saved");
    course.scenes[0].slides[0].objects.push(Object {
        id: "img1".into(),
        name: "img1".into(),
        kind: ObjectKind::Image {
            crop: None,
            src: "photo.png".into(),
            alt: "A scenic photo".into(),
            outline: None,
            shape: None,
        },
        frame: Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        },
        timing: Timing::default(),
        states: Vec::new(),
        initial_state: "normal".into(),
        entrance: None,
        exit: None,
        motion_paths: Vec::new(),
        focus_order: None,
        decorative: false,
        description: None,
        group: None,
        panel: None,
        rotation: 0.0,
        flip_h: false,
        flip_v: false,
        opacity: 1.0,
        shadow: None,
        glow: None,
        drag: None,
        locked: false,
        editor_hidden: false,
    });

    let media_bytes = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    let editor_media = vec![("photo.png".to_string(), media_bytes.clone())];

    let archive_bytes =
        coursecraft_format::save_project(&course, &editor_media).expect("save project succeeds");

    let opened = coursecraft_format::open_project(&archive_bytes).expect("open project succeeds");
    assert_eq!(opened.course, course);

    let media_lookup = |src: &str| -> Option<Vec<u8>> {
        opened
            .media
            .iter()
            .find(|(s, _)| s == src)
            .map(|(_, b)| b.clone())
    };
    let course_json = opened.course.to_json().expect("serialize course");
    let input = PackageInput {
        course: &opened.course,
        course_json: &course_json,
        player_files: &fake_player(),
        media: &media_lookup,
    };
    let files = build_package(&input).expect("package builds");

    let media_file = files
        .iter()
        .find(|f| f.path == "media/photo.png")
        .expect("media/photo.png packaged");
    assert_eq!(media_file.bytes, media_bytes);
}

#[test]
fn course_with_audio_and_video_packaged_under_media() {
    let mut course = minimal_course("Audio and Video Course");
    course.scenes[0].slides[0].objects.push(Object {
        id: "aud1".into(),
        name: "Audio".into(),
        kind: ObjectKind::Audio {
            src: "sound.wav".into(),
            autoplay: true,
            captions: None,
            volume: 1.0,
            trim: None,
        },
        frame: Rect {
            x: 0.0,
            y: 0.0,
            w: 48.0,
            h: 48.0,
        },
        timing: Timing::default(),
        states: Vec::new(),
        initial_state: "normal".into(),
        entrance: None,
        exit: None,
        motion_paths: Vec::new(),
        focus_order: None,
        decorative: false,
        description: None,
        group: None,
        panel: None,
        rotation: 0.0,
        flip_h: false,
        flip_v: false,
        opacity: 1.0,
        shadow: None,
        glow: None,
        drag: None,
        locked: false,
        editor_hidden: false,
    });
    course.scenes[0].slides[0].objects.push(Object {
        id: "vid1".into(),
        name: "Video".into(),
        kind: ObjectKind::Video {
            src: "clip.mp4".into(),
            autoplay: false,
            captions: None,
            volume: 1.0,
            trim: None,
            cues: Vec::new(),
            poster: None,
        },
        frame: Rect {
            x: 10.0,
            y: 10.0,
            w: 640.0,
            h: 360.0,
        },
        timing: Timing::default(),
        states: Vec::new(),
        initial_state: "normal".into(),
        entrance: None,
        exit: None,
        motion_paths: Vec::new(),
        focus_order: None,
        decorative: false,
        description: None,
        group: None,
        panel: None,
        rotation: 0.0,
        flip_h: false,
        flip_v: false,
        opacity: 1.0,
        shadow: None,
        glow: None,
        drag: None,
        locked: false,
        editor_hidden: false,
    });

    let wav_bytes = vec![b'R', b'I', b'F', b'F', 0, 0, 0, 0];
    let mp4_bytes = vec![0, 0, 0, 0x18, b'f', b't', b'y', b'p'];

    let media = |src: &str| -> Option<Vec<u8>> {
        match src {
            "sound.wav" => Some(wav_bytes.clone()),
            "clip.mp4" => Some(mp4_bytes.clone()),
            _ => None,
        }
    };

    let course_json = course.to_json().expect("serialize course");
    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &fake_player(),
        media: &media,
    };

    let files = build_package(&input).expect("package builds");

    let aud_file = files
        .iter()
        .find(|f| f.path == "media/sound.wav")
        .expect("media/sound.wav packaged");
    assert_eq!(aud_file.bytes, wav_bytes);

    let vid_file = files
        .iter()
        .find(|f| f.path == "media/clip.mp4")
        .expect("media/clip.mp4 packaged");
    assert_eq!(vid_file.bytes, mp4_bytes);

    let manifest = files
        .iter()
        .find(|f| f.path == "imsmanifest.xml")
        .expect("manifest present");
    let xml = std::str::from_utf8(&manifest.bytes).expect("utf-8 manifest");
    assert!(xml.contains("<file href=\"media/sound.wav\"/>"));
    assert!(xml.contains("<file href=\"media/clip.mp4\"/>"));
}

#[test]
fn course_with_a_lottie_animation_packages_its_file() {
    let mut course = minimal_course("Lottie Course");
    course.scenes[0].slides[0].objects.push(Object {
        id: "lot1".into(),
        name: "Animation".into(),
        kind: ObjectKind::Lottie {
            src: "motion.lottie".into(),
            loop_playback: false,
            autoplay: true,
            speed: 1.0,
        },
        frame: Rect {
            x: 10.0,
            y: 10.0,
            w: 320.0,
            h: 180.0,
        },
        timing: Timing::default(),
        states: Vec::new(),
        initial_state: "normal".into(),
        entrance: None,
        exit: None,
        motion_paths: Vec::new(),
        focus_order: None,
        decorative: false,
        description: None,
        group: None,
        panel: None,
        rotation: 0.0,
        flip_h: false,
        flip_v: false,
        opacity: 1.0,
        shadow: None,
        glow: None,
        drag: None,
        locked: false,
        editor_hidden: false,
    });
    let lottie_bytes = b"PK\x03\x04fake lottie".to_vec();
    let media =
        |src: &str| -> Option<Vec<u8>> { (src == "motion.lottie").then(|| lottie_bytes.clone()) };
    let course_json = course.to_json().expect("serialize course");
    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &fake_player(),
        media: &media,
    };
    let files = build_package(&input).expect("package builds");
    let packaged = files
        .iter()
        .find(|f| f.path == "media/motion.lottie")
        .expect("Lottie file packaged");
    assert_eq!(packaged.bytes, lottie_bytes);
    let manifest = files
        .iter()
        .find(|f| f.path == "imsmanifest.xml")
        .expect("manifest present");
    let xml = std::str::from_utf8(&manifest.bytes).expect("utf-8 manifest");
    assert!(xml.contains("<file href=\"media/motion.lottie\"/>"));
}

#[test]
fn scorm12_package_is_unchanged_and_has_no_standard_marker() {
    let course = Course::from_json(HELLO_COURSE_JSON).expect("valid course");
    let no_media = |_src: &str| -> Option<Vec<u8>> { None };
    let input = PackageInput {
        course: &course,
        course_json: HELLO_COURSE_JSON,
        player_files: &fake_player(),
        media: &no_media,
    };

    let legacy = build_package(&input).expect("package builds");
    let explicit = build_package_for(&input, Standard::Scorm12).expect("scorm12 package builds");
    assert_eq!(legacy, explicit, "SCORM 1.2 output must stay unchanged");
    assert!(
        !legacy.iter().any(|f| f.path == PACKAGE_MARKER),
        "1.2 packages must not carry the standard marker"
    );

    let manifest = legacy
        .iter()
        .find(|f| f.path == "imsmanifest.xml")
        .expect("manifest present");
    let xml = std::str::from_utf8(&manifest.bytes).expect("utf-8 manifest");
    assert!(xml.contains("<schemaversion>1.2</schemaversion>"));
    assert!(xml.contains("adlcp:scormtype=\"sco\""));
    assert!(!xml.contains("coursecraft-package.json"));
}

#[test]
fn cmi5_package_has_course_structure_and_marker() {
    let course = Course::from_json(HELLO_COURSE_JSON).expect("valid course");
    let no_media = |_src: &str| -> Option<Vec<u8>> { None };
    let input = PackageInput {
        course: &course,
        course_json: HELLO_COURSE_JSON,
        player_files: &fake_player(),
        media: &no_media,
    };

    let files =
        build_package_standard(&input, PackageStandard::Xapi(Flavor::Cmi5)).expect("cmi5 package");
    assert!(
        !files.iter().any(|f| f.path == "imsmanifest.xml"),
        "cmi5 packages have no imsmanifest"
    );
    let marker = files
        .iter()
        .find(|f| f.path == PACKAGE_MARKER)
        .expect("marker");
    assert_eq!(marker.bytes, b"{\"standard\":\"cmi5\"}".as_slice());

    let cmi5 = files
        .iter()
        .find(|f| f.path == "cmi5.xml")
        .expect("cmi5.xml");
    let xml = std::str::from_utf8(&cmi5.bytes).expect("utf-8");
    assert!(xml.contains("https://w3id.org/xapi/profiles/cmi5/v1/CourseStructure.xsd"));
    assert!(xml.contains("urn:coursecraft:course:hello-course"));
    assert!(xml.contains("urn:coursecraft:au:hello-course"));
    assert!(xml.contains("moveOn=\"Passed\""));
    assert!(xml.contains("masteryScore=\"0.8\""));
    assert!(xml.contains("Workplace Safety Basics"));
}

#[test]
fn xapi_package_has_tincan_xml_and_marker() {
    let course = Course::from_json(HELLO_COURSE_JSON).expect("valid course");
    let no_media = |_src: &str| -> Option<Vec<u8>> { None };
    let input = PackageInput {
        course: &course,
        course_json: HELLO_COURSE_JSON,
        player_files: &fake_player(),
        media: &no_media,
    };

    let files =
        build_package_standard(&input, PackageStandard::Xapi(Flavor::Xapi)).expect("xapi package");
    assert!(
        !files.iter().any(|f| f.path == "imsmanifest.xml"),
        "xAPI packages have no imsmanifest"
    );
    let marker = files
        .iter()
        .find(|f| f.path == PACKAGE_MARKER)
        .expect("marker");
    assert_eq!(marker.bytes, b"{\"standard\":\"xapi\"}".as_slice());

    let tincan = files
        .iter()
        .find(|f| f.path == "tincan.xml")
        .expect("tincan.xml");
    let xml = std::str::from_utf8(&tincan.bytes).expect("utf-8");
    assert!(xml.contains("<tincan xmlns=\"http://projecttincan.com/tincan.xsd\">"));
    assert!(xml.contains("urn:coursecraft:activity:hello-course"));
    assert!(xml.contains("Workplace Safety Basics"));
    assert!(xml.contains("<launch lang=\"en-US\">index.html</launch>"));
}

#[test]
fn lti_package_has_the_marker_no_manifest_and_the_same_media_as_scorm12() {
    let mut course = minimal_course("LTI Course");
    course.scenes[0].slides[0].objects.push(media_object(
        "media1",
        ObjectKind::Audio {
            src: "sound.wav".into(),
            autoplay: false,
            captions: None,
            volume: 1.0,
            trim: None,
        },
    ));
    let media =
        |src: &str| -> Option<Vec<u8>> { (src == "sound.wav").then(|| b"audio bytes".to_vec()) };
    let course_json = course.to_json().expect("serialize");
    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &fake_player(),
        media: &media,
    };

    let lti = build_package_standard(&input, PackageStandard::Lti).expect("lti package");
    let scorm = build_package_standard(&input, PackageStandard::Scorm(Standard::Scorm12))
        .expect("scorm package");

    let lti_paths: Vec<&str> = lti.iter().map(|file| file.path.as_str()).collect();
    let scorm_paths: Vec<&str> = scorm.iter().map(|file| file.path.as_str()).collect();

    let marker = lti
        .iter()
        .find(|file| file.path == PACKAGE_MARKER)
        .expect("LTI marker");
    assert_eq!(marker.bytes, b"{\"standard\":\"lti\"}".as_slice());
    assert!(
        !lti_paths.contains(&"imsmanifest.xml"),
        "LTI packages have no imsmanifest: {lti_paths:?}"
    );

    // The same files as SCORM 1.2, plus the marker and minus the manifest.
    let mut expected: Vec<&str> = scorm_paths
        .into_iter()
        .filter(|path| *path != "imsmanifest.xml")
        .collect();
    expected.push(PACKAGE_MARKER);
    expected.sort_unstable();
    assert_eq!(lti_paths, expected);

    let packaged_media = lti
        .iter()
        .find(|file| file.path == "media/sound.wav")
        .expect("media packaged");
    assert_eq!(packaged_media.bytes, b"audio bytes".as_slice());
}

#[test]
fn web_package_has_marker_index_and_no_manifest() {
    let mut course = minimal_course("Web Course");
    course.scenes[0].slides[0].objects.push(media_object(
        "media1",
        ObjectKind::Audio {
            src: "sound.wav".into(),
            autoplay: false,
            captions: None,
            volume: 1.0,
            trim: None,
        },
    ));
    let media =
        |src: &str| -> Option<Vec<u8>> { (src == "sound.wav").then(|| b"audio bytes".to_vec()) };
    let course_json = course.to_json().expect("serialize");
    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &fake_player(),
        media: &media,
    };

    let web = build_package_standard(&input, PackageStandard::Web).expect("web package");
    let scorm = build_package_standard(&input, PackageStandard::Scorm(Standard::Scorm12))
        .expect("scorm package");

    let web_paths: Vec<&str> = web.iter().map(|file| file.path.as_str()).collect();
    let scorm_paths: Vec<&str> = scorm.iter().map(|file| file.path.as_str()).collect();

    assert!(
        web_paths.contains(&"index.html"),
        "web packages must include index.html: {web_paths:?}"
    );
    let marker = web
        .iter()
        .find(|file| file.path == PACKAGE_MARKER)
        .expect("web marker");
    assert_eq!(marker.bytes, b"{\"standard\":\"web\"}".as_slice());
    assert!(
        !web_paths.contains(&"imsmanifest.xml"),
        "web packages have no imsmanifest: {web_paths:?}"
    );

    // The same files as SCORM 1.2, plus the marker and minus the manifest.
    let mut expected: Vec<&str> = scorm_paths
        .into_iter()
        .filter(|path| *path != "imsmanifest.xml")
        .collect();
    expected.push(PACKAGE_MARKER);
    expected.sort_unstable();
    assert_eq!(web_paths, expected);
}

#[test]
fn scorm12_via_the_standard_entry_point_is_byte_identical() {
    let course = Course::from_json(HELLO_COURSE_JSON).expect("valid course");
    let no_media = |_src: &str| -> Option<Vec<u8>> { None };
    let input = PackageInput {
        course: &course,
        course_json: HELLO_COURSE_JSON,
        player_files: &fake_player(),
        media: &no_media,
    };
    let legacy = build_package(&input).expect("legacy package");
    let explicit = build_package_standard(&input, PackageStandard::Scorm(Standard::Scorm12))
        .expect("standard package");
    assert_eq!(legacy, explicit);
}

#[test]
fn scorm2004_package_adds_marker_and_2004_manifest() {
    let course = Course::from_json(HELLO_COURSE_JSON).expect("valid course");
    let no_media = |_src: &str| -> Option<Vec<u8>> { None };
    let input = PackageInput {
        course: &course,
        course_json: HELLO_COURSE_JSON,
        player_files: &fake_player(),
        media: &no_media,
    };

    let files = build_package_for(&input, Standard::Scorm2004).expect("package builds");
    let marker = files
        .iter()
        .find(|f| f.path == PACKAGE_MARKER)
        .expect("standard marker present");
    assert_eq!(marker.bytes, b"{\"standard\":\"scorm2004\"}".as_slice());

    let manifest = files
        .iter()
        .find(|f| f.path == "imsmanifest.xml")
        .expect("manifest present");
    let xml = std::str::from_utf8(&manifest.bytes).expect("utf-8 manifest");
    assert!(xml.contains("<schema>ADL SCORM</schema>"));
    assert!(xml.contains("<schemaversion>2004 4th Edition</schemaversion>"));
    assert!(xml.contains("adlcp:scormType=\"sco\""));
    assert!(xml.contains("<file href=\"coursecraft-package.json\"/>"));
    assert!(!xml.contains("masteryscore"));
    assert!(!xml.contains("sequencing"));
    assert!(!xml.contains("adlcp:scormtype"));
}

fn media_object(id: &str, kind: ObjectKind) -> Object {
    Object {
        id: id.into(),
        name: id.into(),
        kind,
        frame: Rect {
            x: 0.0,
            y: 0.0,
            w: 48.0,
            h: 48.0,
        },
        timing: Timing::default(),
        states: Vec::new(),
        initial_state: "normal".into(),
        entrance: None,
        exit: None,
        motion_paths: Vec::new(),
        focus_order: None,
        decorative: false,
        description: None,
        group: None,
        panel: None,
        rotation: 0.0,
        flip_h: false,
        flip_v: false,
        opacity: 1.0,
        shadow: None,
        glow: None,
        drag: None,
        locked: false,
        editor_hidden: false,
    }
}

#[test]
fn missing_caption_file_fails_publish_for_audio_and_video() {
    for kind in [
        ObjectKind::Audio {
            src: "sound.wav".into(),
            autoplay: false,
            captions: Some("sound.vtt".into()),
            volume: 1.0,
            trim: None,
        },
        ObjectKind::Video {
            src: "clip.mp4".into(),
            autoplay: false,
            captions: Some("clip.vtt".into()),
            volume: 1.0,
            trim: None,
            cues: Vec::new(),
            poster: None,
        },
    ] {
        let mut course = minimal_course("Missing captions");
        course.scenes[0].slides[0]
            .objects
            .push(media_object("media1", kind));
        let media = |src: &str| -> Option<Vec<u8>> {
            if src.ends_with(".vtt") {
                None
            } else {
                Some(b"media".to_vec())
            }
        };
        let input = PackageInput {
            course: &course,
            course_json: "{}",
            player_files: &fake_player(),
            media: &media,
        };
        let err = build_package(&input).expect_err("missing vtt must fail");
        assert!(matches!(
            err,
            PublishError::MissingMedia { ref src, .. } if src.ends_with(".vtt")
        ));
    }
}

#[test]
fn caption_files_are_packaged_under_media_for_audio_and_video() {
    for (kind, vtt) in [
        (
            ObjectKind::Audio {
                src: "sound.wav".into(),
                autoplay: false,
                captions: Some("sound.vtt".into()),
                volume: 1.0,
                trim: None,
            },
            "sound.vtt",
        ),
        (
            ObjectKind::Video {
                src: "clip.mp4".into(),
                autoplay: false,
                captions: Some("clip.vtt".into()),
                volume: 1.0,
                trim: None,
                cues: Vec::new(),
                poster: None,
            },
            "clip.vtt",
        ),
    ] {
        let mut course = minimal_course("Captions");
        course.scenes[0].slides[0]
            .objects
            .push(media_object("media1", kind));
        let media = |src: &str| -> Option<Vec<u8>> { Some(format!("bytes:{src}").into_bytes()) };
        let input = PackageInput {
            course: &course,
            course_json: "{}",
            player_files: &fake_player(),
            media: &media,
        };
        let files = build_package(&input).expect("package builds");
        let caption = files
            .iter()
            .find(|f| f.path == format!("media/{vtt}"))
            .expect("caption file packaged");
        assert_eq!(caption.bytes, format!("bytes:{vtt}").into_bytes());
    }
}

#[test]
fn master_media_is_packaged_templates_are_stripped_and_their_media_is_not() {
    let mut course = minimal_course("Masters");
    let image = |id: &str, src: &str| Object {
        id: id.to_owned(),
        name: String::new(),
        kind: ObjectKind::Image {
            crop: None,
            src: src.to_owned(),
            alt: String::new(),
            outline: None,
            shape: None,
        },
        frame: Rect {
            x: 0.0,
            y: 0.0,
            w: 10.0,
            h: 10.0,
        },
        timing: Timing::default(),
        states: Vec::new(),
        initial_state: "normal".into(),
        entrance: None,
        exit: None,
        motion_paths: Vec::new(),
        focus_order: None,
        decorative: false,
        description: None,
        group: None,
        panel: None,
        rotation: 0.0,
        flip_h: false,
        flip_v: false,
        opacity: 1.0,
        shadow: None,
        glow: None,
        drag: None,
        locked: false,
        editor_hidden: false,
    };

    let mut master = course.scenes[0].slides[0].clone();
    master.id = "master1".into();
    master.title = "Master".into();
    master.objects = vec![image("mi", "master.png")];
    course.masters.push(master);
    course.scenes[0].slides[0].master = Some("master1".into());

    let mut template = course.scenes[0].slides[0].clone();
    template.id = "template1".into();
    template.title = "Template".into();
    template.objects = vec![image("ti", "template.png")];
    course.templates.push(template);

    let course_json = course.to_json().expect("serialise");
    assert!(course_json.contains("\"templates\""));
    let media = |src: &str| -> Option<Vec<u8>> {
        match src {
            "master.png" | "template.png" => Some(src.as_bytes().to_vec()),
            _ => None,
        }
    };
    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &fake_player(),
        media: &media,
    };
    let files = build_package(&input).expect("package builds");
    let paths: Vec<&str> = files.iter().map(|file| file.path.as_str()).collect();
    assert!(paths.contains(&"media/master.png"), "{paths:?}");
    assert!(!paths.contains(&"media/template.png"), "{paths:?}");

    let published = files
        .iter()
        .find(|file| file.path == "course.json")
        .expect("course.json");
    let published: Course =
        Course::from_json(std::str::from_utf8(&published.bytes).expect("utf8")).expect("parses");
    assert!(published.templates.is_empty());
    assert_eq!(published.masters.len(), 1);
}

#[test]
fn a_course_without_templates_publishes_the_same_course_json_bytes() {
    let course = Course::from_json(HELLO_COURSE_JSON).expect("valid course");
    let no_media = |_src: &str| -> Option<Vec<u8>> { None };
    let input = PackageInput {
        course: &course,
        course_json: HELLO_COURSE_JSON,
        player_files: &fake_player(),
        media: &no_media,
    };
    let files = build_package(&input).expect("package builds");
    let published = files
        .iter()
        .find(|file| file.path == "course.json")
        .expect("course.json");
    assert_eq!(published.bytes, HELLO_COURSE_JSON.as_bytes());
}

#[test]
fn a_picture_shape_fill_is_packaged_as_media() {
    let mut course = minimal_course("Picture Fill Course");
    course.scenes[0].slides[0].objects.push(Object {
        id: "shape1".into(),
        name: "Photo shape".into(),
        kind: ObjectKind::Shape {
            shape: coursecraft_course::ShapeKind::Ellipse,
            fill: coursecraft_course::Color("#D9D9D9".into()),
            fill_style: Some(coursecraft_course::FillStyle::Picture {
                src: "texture.png".into(),
                mode: coursecraft_course::PictureFillMode::Stretch,
                tile_scale: [1.0, 1.0],
            }),
            outline: None,
            text: None,
            adjust: Vec::new(),
        },
        frame: Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        },
        timing: Timing::default(),
        states: Vec::new(),
        initial_state: "normal".into(),
        entrance: None,
        exit: None,
        motion_paths: Vec::new(),
        focus_order: None,
        decorative: false,
        description: None,
        group: None,
        panel: None,
        rotation: 0.0,
        flip_h: false,
        flip_v: false,
        opacity: 1.0,
        shadow: None,
        glow: None,
        drag: None,
        locked: false,
        editor_hidden: false,
    });

    let media = |src: &str| -> Option<Vec<u8>> {
        if src == "texture.png" {
            Some(vec![1, 2, 3])
        } else {
            None
        }
    };
    let course_json = course.to_json().expect("serialize");
    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &fake_player(),
        media: &media,
    };
    let files = build_package(&input).expect("picture fill packages");
    assert!(
        files.iter().any(|file| file.path == "media/texture.png"),
        "picture fill media is in the package"
    );
}

#[test]
fn media_used_only_on_a_bank_slide_is_packaged() {
    let mut course = minimal_course("Bank media");
    let mut bank_slide = course.scenes[0].slides[0].clone();
    bank_slide.id = "bank-slide".into();
    bank_slide.objects.push(Object {
        id: "bank-img".into(),
        name: "Bank image".into(),
        kind: ObjectKind::Image {
            src: "bank-only.png".into(),
            alt: "Only on a bank slide".into(),
            crop: None,
            outline: None,
            shape: None,
        },
        frame: Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        },
        timing: Timing::default(),
        entrance: None,
        exit: None,
        motion_paths: Vec::new(),
        states: Vec::<StateDef>::new(),
        initial_state: "normal".into(),
        focus_order: None,
        decorative: false,
        description: None,
        group: None,
        panel: None,
        rotation: 0.0,
        flip_h: false,
        flip_v: false,
        opacity: 1.0,
        shadow: None,
        glow: None,
        drag: None,
        locked: false,
        editor_hidden: false,
    });
    course.banks.push(coursecraft_course::QuestionBank {
        id: "bank1".into(),
        title: "Bank One".into(),
        slides: vec![bank_slide],
    });
    let media =
        |src: &str| -> Option<Vec<u8>> { (src == "bank-only.png").then_some(vec![9, 8, 7]) };
    let course_json = course.to_json().expect("serialize");
    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &fake_player(),
        media: &media,
    };
    let files = build_package(&input).expect("bank media packages");
    assert!(
        files.iter().any(|file| file.path == "media/bank-only.png"),
        "media used only on a bank slide is in the package: {:?}",
        files.iter().map(|f| &f.path).collect::<Vec<_>>()
    );
}

#[test]
fn resource_files_are_packaged_and_links_are_not() {
    let mut course = minimal_course("Resources");
    course.resources = vec![
        coursecraft_course::Resource {
            id: "guide".into(),
            title: "Guide".into(),
            target: coursecraft_course::ResourceTarget::File {
                src: "guide.pdf".into(),
            },
        },
        coursecraft_course::Resource {
            id: "site".into(),
            title: "Site".into(),
            target: coursecraft_course::ResourceTarget::Url {
                url: "https://example.com".into(),
            },
        },
    ];
    let player_files = fake_player();
    let course_json = course.to_json().expect("course json");
    let media =
        |src: &str| -> Option<Vec<u8>> { (src == "guide.pdf").then(|| b"%PDF-1.7".to_vec()) };
    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &player_files,
        media: &media,
    };
    let files = build_package(&input).expect("package builds");
    let paths: Vec<&str> = files.iter().map(|file| file.path.as_str()).collect();
    assert!(paths.contains(&"media/guide.pdf"), "{paths:?}");

    let missing = |_src: &str| -> Option<Vec<u8>> { None };
    let input = PackageInput {
        media: &missing,
        ..input
    };
    assert_eq!(
        build_package(&input).expect_err("missing resource file must fail"),
        PublishError::MissingMedia {
            object: "guide".into(),
            src: "guide.pdf".into(),
        }
    );
}

#[test]
fn zip_package_stores_compressed_media_and_deflates_text() {
    let files = vec![
        PackageFile {
            path: "index.html".into(),
            bytes: vec![b'a'; 4096],
        },
        PackageFile {
            path: "media/pic.png".into(),
            bytes: vec![1u8; 4096],
        },
        PackageFile {
            path: "media/clip.mp4".into(),
            bytes: vec![2u8; 4096],
        },
    ];
    let bytes = zip_package(&files).expect("zip");
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes.as_slice())).expect("open");
    for (name, method) in [
        ("index.html", zip::CompressionMethod::Deflated),
        ("media/pic.png", zip::CompressionMethod::Stored),
        ("media/clip.mp4", zip::CompressionMethod::Stored),
    ] {
        assert_eq!(
            archive.by_name(name).expect("entry").compression(),
            method,
            "{name}"
        );
    }
}

#[test]
fn zip_package_to_writes_the_same_bytes_as_zip_package() {
    let files = vec![
        PackageFile {
            path: "index.html".into(),
            bytes: b"<html></html>".to_vec(),
        },
        PackageFile {
            path: "media/pic.png".into(),
            bytes: vec![5u8; 777],
        },
    ];
    let mut out = std::io::Cursor::new(Vec::new());
    zip_package_to(&files, &mut out).expect("zip to writer");
    assert_eq!(out.into_inner(), zip_package(&files).expect("zip"));
}

#[test]
fn auto_embed_fonts_embeds_allowed_skips_restricted_and_warns_for_unembedded() {
    use coursecraft_course::FontStyle;

    struct FakeProvider;
    impl crate::PublishFontProvider for FakeProvider {
        fn lookup_font(&self, family: &str, style: FontStyle) -> crate::PublishFontLookup {
            match family {
                "OpenDisplay" => crate::PublishFontLookup::Found {
                    style,
                    bytes: b"TTF-BYTES".to_vec(),
                    extension: "ttf",
                },
                "RestrictedFont" => crate::PublishFontLookup::Restricted,
                _ => crate::PublishFontLookup::Missing,
            }
        }
    }

    let mut course = minimal_course("Font test");
    let obj: coursecraft_course::Object = serde_json::from_value(serde_json::json!({
        "id": "t1",
        "name": "T1",
        "kind": {
            "type": "text",
            "text": "Hello",
            "style": { "font": "OpenDisplay", "size": 18, "color": "#000000" },
            "runs": [
                { "text": "A", "font": "RestrictedFont" },
                { "text": "B", "font": "AbsentFont" }
            ]
        },
        "frame": { "x": 0.0, "y": 0.0, "w": 100.0, "h": 40.0 }
    }))
    .expect("valid object");
    course.scenes[0].slides[0].objects.push(obj);

    let res = crate::auto_embed_fonts(&course, Some(&FakeProvider));
    assert_eq!(res.embedded_families, vec!["OpenDisplay".to_string()]);
    assert_eq!(res.course.fonts.len(), 1);
    assert_eq!(res.course.fonts[0].family, "OpenDisplay");
    assert_eq!(res.embedded_files.len(), 1);
    assert!(
        res.warnings
            .contains(&"Learners without RestrictedFont will see Inter.".to_string())
    );
    assert!(
        res.warnings
            .contains(&"Learners without AbsentFont will see Inter.".to_string())
    );
    assert!(
        res.warnings
            .contains(&"Font licences are your responsibility.".to_string())
    );
}

#[test]
fn published_packages_pass_strict_manifest_validation_and_include_poster_and_path_fill() {
    use coursecraft_strict_lms::{PackageStandardKind, validate_package};

    let mut course = minimal_course("Strict Publish Test");
    let video_obj: Object = serde_json::from_value(serde_json::json!({
        "id": "v1",
        "name": "Video",
        "kind": {
            "type": "video",
            "src": "media/clip.mp4",
            "poster": "media/poster.png"
        },
        "frame": { "x": 0.0, "y": 0.0, "w": 100.0, "h": 60.0 }
    }))
    .expect("video obj");
    let path_obj: Object = serde_json::from_value(serde_json::json!({
        "id": "p1",
        "name": "Path",
        "kind": {
            "type": "path",
            "points": [
                { "x": 0.0, "y": 0.0 },
                { "x": 50.0, "y": 0.0 },
                { "x": 50.0, "y": 50.0 }
            ],
            "closed": true,
            "outline": { "color": "#000000", "width": 1.0 },
            "fill_style": { "type": "picture", "src": "media/tile.png" }
        },
        "frame": { "x": 0.0, "y": 0.0, "w": 50.0, "h": 50.0 }
    }))
    .expect("path obj");
    course.scenes[0].slides[0].objects.push(video_obj);
    course.scenes[0].slides[0].objects.push(path_obj);

    let media = |src: &str| -> Option<Vec<u8>> {
        match src {
            "media/clip.mp4" => Some(b"MP4".to_vec()),
            "media/poster.png" => Some(b"PNG-POSTER".to_vec()),
            "media/tile.png" => Some(b"PNG-TILE".to_vec()),
            _ => None,
        }
    };
    let course_json = serde_json::to_string(&course).expect("serialize course");
    let player_files = fake_player();
    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &player_files,
        media: &media,
    };

    let standards = [
        (
            PackageStandard::Scorm(Standard::Scorm12),
            PackageStandardKind::Scorm12,
        ),
        (
            PackageStandard::Scorm(Standard::Scorm2004),
            PackageStandardKind::Scorm2004,
        ),
        (
            PackageStandard::Xapi(Flavor::Cmi5),
            PackageStandardKind::Cmi5,
        ),
        (
            PackageStandard::Xapi(Flavor::Xapi),
            PackageStandardKind::Xapi,
        ),
        (PackageStandard::Lti, PackageStandardKind::Lti),
        (PackageStandard::Web, PackageStandardKind::Web),
    ];

    for (std, kind) in standards {
        let files = build_package_standard(&input, std).expect("package builds");
        assert!(
            files.iter().any(|f| f.path.ends_with("poster.png")),
            "{std:?} missing poster.png"
        );
        assert!(
            files.iter().any(|f| f.path.ends_with("tile.png")),
            "{std:?} missing tile.png"
        );
        let refs: Vec<(&str, &[u8])> = files
            .iter()
            .map(|f| (f.path.as_str(), f.bytes.as_slice()))
            .collect();
        let violations = validate_package(kind, &refs);
        assert!(
            violations.is_empty(),
            "{std:?} manifest violations: {violations:?}"
        );
    }
}

#[test]
fn publish_bundles_state_src_and_state_fill_style_picture_media() {
    let mut course = Course::from_json(include_str!("../../../examples/hello-course/course.json"))
        .expect("valid course");
    let img_obj: coursecraft_course::Object = serde_json::from_value(serde_json::json!({
        "id": "img-state",
        "name": "Image State",
        "kind": { "type": "image", "src": "base.png", "alt": "Base" },
        "frame": { "x": 0.0, "y": 0.0, "w": 50.0, "h": 50.0 },
        "states": [{
            "name": "hover",
            "src": "hover-only.png"
        }]
    }))
    .expect("img obj");
    let shp_obj: coursecraft_course::Object = serde_json::from_value(serde_json::json!({
        "id": "shp-state",
        "name": "Shape State",
        "kind": { "type": "shape", "shape": "rectangle", "fill": "#112233" },
        "frame": { "x": 60.0, "y": 0.0, "w": 50.0, "h": 50.0 },
        "states": [{
            "name": "hover",
            "fill_style": { "type": "picture", "src": "state-fill-only.png" }
        }]
    }))
    .expect("shp obj");
    course.scenes[0].slides[0].objects.push(img_obj);
    course.scenes[0].slides[0].objects.push(shp_obj);

    let media = |src: &str| -> Option<Vec<u8>> {
        match src {
            "base.png" => Some(b"BASE".to_vec()),
            "hover-only.png" => Some(b"HOVER".to_vec()),
            "state-fill-only.png" => Some(b"FILL".to_vec()),
            _ => None,
        }
    };
    let course_json = serde_json::to_string(&course).expect("serialize");
    let player_files = fake_player();
    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &player_files,
        media: &media,
    };
    let files = build_package_standard(&input, PackageStandard::Web).expect("publish");
    assert!(files.iter().any(|f| f.path == "media/hover-only.png"));
    assert!(files.iter().any(|f| f.path == "media/state-fill-only.png"));
    let refs: Vec<(&str, &[u8])> = files
        .iter()
        .map(|f| (f.path.as_str(), f.bytes.as_slice()))
        .collect();
    let violations = coursecraft_strict_lms::validate_package(
        coursecraft_strict_lms::PackageStandardKind::Web,
        &refs,
    );
    assert!(violations.is_empty(), "violations: {violations:?}");
}

#[test]
fn publish_rejects_missing_or_unsafe_state_fill_style_media() {
    let mut course = Course::from_json(include_str!("../../../examples/hello-course/course.json"))
        .expect("valid course");
    let shp_obj: coursecraft_course::Object = serde_json::from_value(serde_json::json!({
        "id": "shp-missing",
        "name": "Shape Missing",
        "kind": { "type": "shape", "shape": "rectangle", "fill": "#112233" },
        "frame": { "x": 60.0, "y": 0.0, "w": 50.0, "h": 50.0 },
        "states": [{
            "name": "hover",
            "fill_style": { "type": "picture", "src": "missing-state.png" }
        }]
    }))
    .expect("shp obj");
    course.scenes[0].slides[0].objects.push(shp_obj);

    let media = |_: &str| -> Option<Vec<u8>> { None };
    let course_json = serde_json::to_string(&course).expect("serialize");
    let player_files = fake_player();
    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &player_files,
        media: &media,
    };
    let err = build_package_standard(&input, PackageStandard::Web).expect_err("missing media");
    assert!(
        matches!(
            &err,
            PublishError::MissingMedia { object, src }
                if object == "shp-missing" && src == "missing-state.png"
        ),
        "expected MissingMedia, got {err:?}"
    );

    // Unsafe path
    course.scenes[0].slides[0]
        .objects
        .last_mut()
        .expect("obj")
        .states[0]
        .fill_style = Some(coursecraft_course::FillStyle::Picture {
        src: "../secret.png".into(),
        mode: Default::default(),
        tile_scale: [1.0, 1.0],
    });
    let course_json = serde_json::to_string(&course).expect("serialize");
    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &player_files,
        media: &media,
    };
    let err = build_package_standard(&input, PackageStandard::Web).expect_err("unsafe path");
    assert!(
        matches!(err, PublishError::UnsafePath { .. }),
        "expected UnsafePath, got {err:?}"
    );
}

#[test]
fn collect_used_font_faces_includes_fonts_used_only_in_state_text_style_and_runs() {
    let mut course = Course::from_json(include_str!("../../../examples/hello-course/course.json"))
        .expect("valid course");
    let obj: coursecraft_course::Object = serde_json::from_value(serde_json::json!({
        "id": "btn-font",
        "name": "Button Font",
        "kind": { "type": "button", "label": "Next" },
        "frame": { "x": 0.0, "y": 0.0, "w": 120.0, "h": 40.0 },
        "states": [{
            "name": "hover",
            "text_style": { "font": "Lora", "size": 16.0, "color": "#111111", "bold": true },
            "runs": [
                { "text": "A", "font": "JetBrains Mono", "italic": true }
            ]
        }]
    }))
    .expect("btn obj");
    course.scenes[0].slides[0].objects.push(obj);

    let faces = collect_used_font_faces(&course);
    assert!(
        faces.contains(&("Lora".to_string(), coursecraft_course::FontStyle::Bold)),
        "expected Lora Bold in {faces:?}"
    );
    assert!(
        faces.contains(&(
            "JetBrains Mono".to_string(),
            coursecraft_course::FontStyle::BoldItalic
        )),
        "expected JetBrains Mono BoldItalic in {faces:?}"
    );
}

#[test]
fn publish_bundles_state_extra_image_shape_and_path_media() {
    let mut course = Course::from_json(include_str!("../../../examples/hello-course/course.json"))
        .expect("valid course");
    let obj: coursecraft_course::Object = serde_json::from_value(serde_json::json!({
        "id": "btn-extra-media",
        "name": "Button Extra Media",
        "kind": { "type": "button", "label": "Next" },
        "frame": { "x": 0.0, "y": 0.0, "w": 120.0, "h": 40.0 },
        "states": [{
            "name": "hover",
            "extras": [
                {
                    "id": "ex-img",
                    "kind": { "type": "image", "src": "ex-img.png" },
                    "frame": { "x": 0.0, "y": 0.0, "w": 20.0, "h": 20.0 }
                },
                {
                    "id": "ex-shp",
                    "kind": {
                        "type": "shape",
                        "shape": "rectangle",
                        "fill": "#112233",
                        "fill_style": { "type": "picture", "src": "ex-shp.png" }
                    },
                    "frame": { "x": 0.0, "y": 0.0, "w": 20.0, "h": 20.0 }
                },
                {
                    "id": "ex-path",
                    "kind": {
                        "type": "path",
                        "points": [{ "x": 0.0, "y": 0.0 }, { "x": 1.0, "y": 0.0 }, { "x": 1.0, "y": 1.0 }],
                        "closed": true,
                        "outline": { "color": "#000000", "width": 1.0 },
                        "fill_style": { "type": "picture", "src": "ex-path.png" }
                    },
                    "frame": { "x": 0.0, "y": 0.0, "w": 20.0, "h": 20.0 }
                }
            ]
        }]
    }))
    .expect("btn obj");
    course.scenes[0].slides[0].objects.push(obj);

    let media = |src: &str| -> Option<Vec<u8>> {
        if matches!(src, "ex-img.png" | "ex-shp.png" | "ex-path.png") {
            Some(vec![1, 2, 3, 4])
        } else {
            None
        }
    };
    let course_json = serde_json::to_string(&course).expect("serialize");
    let player_files = fake_player();
    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &player_files,
        media: &media,
    };
    let files = build_package_standard(&input, PackageStandard::Web).expect("build package");
    for expected in ["media/ex-img.png", "media/ex-shp.png", "media/ex-path.png"] {
        assert!(
            files.iter().any(|f| f.path == expected),
            "missing {expected} in published package"
        );
    }
    let refs: Vec<(&str, &[u8])> = files
        .iter()
        .map(|f| (f.path.as_str(), f.bytes.as_slice()))
        .collect();
    let violations = coursecraft_strict_lms::validate_package(
        coursecraft_strict_lms::PackageStandardKind::Web,
        &refs,
    );
    assert!(violations.is_empty(), "violations: {violations:?}");
}

#[test]
fn publish_rejects_missing_or_unsafe_state_extra_media() {
    let mut course = Course::from_json(include_str!("../../../examples/hello-course/course.json"))
        .expect("valid course");
    let obj: coursecraft_course::Object = serde_json::from_value(serde_json::json!({
        "id": "btn-missing-extra",
        "name": "Button Missing Extra",
        "kind": { "type": "button", "label": "Next" },
        "frame": { "x": 0.0, "y": 0.0, "w": 120.0, "h": 40.0 },
        "states": [{
            "name": "hover",
            "extras": [{
                "id": "ex-img",
                "kind": { "type": "image", "src": "missing-extra.png" },
                "frame": { "x": 0.0, "y": 0.0, "w": 20.0, "h": 20.0 }
            }]
        }]
    }))
    .expect("btn obj");
    course.scenes[0].slides[0].objects.push(obj);

    let media = |_: &str| -> Option<Vec<u8>> { None };
    let course_json = serde_json::to_string(&course).expect("serialize");
    let player_files = fake_player();
    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &player_files,
        media: &media,
    };
    let err = build_package_standard(&input, PackageStandard::Web).expect_err("missing media");
    assert!(
        matches!(
            &err,
            PublishError::MissingMedia { object, src }
                if object == "btn-missing-extra" && src == "missing-extra.png"
        ),
        "expected MissingMedia, got {err:?}"
    );

    course.scenes[0].slides[0]
        .objects
        .last_mut()
        .expect("obj")
        .states[0]
        .extras[0]
        .kind = coursecraft_course::StateExtraKind::Image {
        src: "../secret.png".into(),
        alt: String::new(),
        crop: None,
        outline: None,
        shape: None,
    };
    let course_json = serde_json::to_string(&course).expect("serialize");
    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &player_files,
        media: &media,
    };
    let err = build_package_standard(&input, PackageStandard::Web).expect_err("unsafe path");
    assert!(
        matches!(err, PublishError::UnsafePath { .. }),
        "expected UnsafePath, got {err:?}"
    );
}

#[test]
fn collect_used_font_faces_includes_fonts_used_only_in_state_extra_text() {
    let mut course = Course::from_json(include_str!("../../../examples/hello-course/course.json"))
        .expect("valid course");
    let obj: coursecraft_course::Object = serde_json::from_value(serde_json::json!({
        "id": "btn-extra-font",
        "name": "Button Extra Font",
        "kind": { "type": "button", "label": "Next" },
        "frame": { "x": 0.0, "y": 0.0, "w": 120.0, "h": 40.0 },
        "states": [{
            "name": "hover",
            "extras": [
                {
                    "id": "ex-txt",
                    "kind": {
                        "type": "text",
                        "text": "Hi",
                        "style": { "font": "Merriweather", "size": 14.0, "color": "#111111", "bold": true },
                        "runs": [{ "text": "Hi", "font": "IBM Plex Mono", "italic": true }]
                    },
                    "frame": { "x": 0.0, "y": 0.0, "w": 40.0, "h": 20.0 }
                },
                {
                    "id": "ex-shp",
                    "kind": {
                        "type": "shape",
                        "shape": "rectangle",
                        "fill": "#112233",
                        "text": {
                            "text": "Yo",
                            "style": { "font": "Rubik", "size": 14.0, "color": "#111111" }
                        }
                    },
                    "frame": { "x": 0.0, "y": 24.0, "w": 40.0, "h": 20.0 }
                }
            ]
        }]
    }))
    .expect("btn obj");
    course.scenes[0].slides[0].objects.push(obj);

    let faces = collect_used_font_faces(&course);
    assert!(
        faces.contains(&(
            "Merriweather".to_string(),
            coursecraft_course::FontStyle::Bold
        )),
        "expected Merriweather Bold in {faces:?}"
    );
    assert!(
        faces.contains(&(
            "IBM Plex Mono".to_string(),
            coursecraft_course::FontStyle::BoldItalic
        )),
        "expected IBM Plex Mono BoldItalic in {faces:?}"
    );
    assert!(
        faces.contains(&("Rubik".to_string(), coursecraft_course::FontStyle::Regular)),
        "expected Rubik Regular in {faces:?}"
    );
}

#[test]
fn m31_12_empty_state_src_builds_and_zips_with_base_image() {
    let mut course = minimal_course("Empty state src");
    course.scenes[0].slides[0].objects.push(Object {
        id: "img1".into(),
        name: "Photo".into(),
        kind: ObjectKind::Image {
            crop: None,
            src: "photo.png".into(),
            alt: "Photo".into(),
            outline: None,
            shape: None,
        },
        frame: Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        },
        timing: Timing::default(),
        states: vec![StateDef {
            name: "hover".into(),
            fill: None,
            text: None,
            src: Some(String::new()),
            ..Default::default()
        }],
        initial_state: "normal".into(),
        entrance: None,
        exit: None,
        motion_paths: Vec::new(),
        focus_order: None,
        decorative: false,
        description: None,
        group: None,
        panel: None,
        rotation: 0.0,
        flip_h: false,
        flip_v: false,
        opacity: 1.0,
        shadow: None,
        glow: None,
        drag: None,
        locked: false,
        editor_hidden: false,
    });
    assert!(
        course.is_playable(),
        "course with empty state.src must be playable"
    );

    let player_files = fake_player();
    let course_json = course.to_json().expect("course json");
    let media =
        |src: &str| -> Option<Vec<u8>> { (src == "photo.png").then(|| b"png bytes".to_vec()) };
    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &player_files,
        media: &media,
    };
    let files = build_package(&input).expect("package builds with empty state.src");
    let paths: Vec<&str> = files.iter().map(|file| file.path.as_str()).collect();
    assert!(
        paths.contains(&"media/photo.png"),
        "base image missing from {paths:?}"
    );
    let zip_bytes = zip_package(&files).expect("zip builds cleanly");
    assert!(!zip_bytes.is_empty(), "zip output must be non-empty");
}

#[test]
fn m31_20_package_zipper_matches_zip_package_across_chunk_sizes() {
    // Mix of deflated text/wasm, precompressed media, and empty files, > 1 MiB total.
    let mut big_wasm = Vec::with_capacity(600 * 1024);
    for i in 0..(600 * 1024 / 4) {
        big_wasm.extend_from_slice(&(i as u32).to_le_bytes());
    }
    let mut big_png = Vec::with_capacity(700 * 1024);
    for i in 0..(700 * 1024) {
        big_png.push((i % 251) as u8);
    }
    let files = vec![
        PackageFile {
            path: "course.json".to_string(),
            bytes: br#"{"title":"Chunked"}"#.repeat(200),
        },
        PackageFile {
            path: "coursecraft_player_bg.wasm".to_string(),
            bytes: big_wasm,
        },
        PackageFile {
            path: "empty.txt".to_string(),
            bytes: Vec::new(),
        },
        PackageFile {
            path: "index.html".to_string(),
            bytes: b"<!doctype html><html></html>".to_vec(),
        },
        PackageFile {
            path: "media/photo.png".to_string(),
            bytes: big_png,
        },
    ];
    let expected = zip_package(&files).expect("reference zip_package");
    let expected_total: u64 = files.iter().map(|f| f.bytes.len() as u64).sum();

    for chunk_size in [1024_usize, 64 * 1024, 256 * 1024] {
        let mut zipper = PackageZipper::new(files.clone());
        let mut steps = 0usize;
        let mut prev_done = 0u64;
        let result = loop {
            steps += 1;
            match zipper.step(chunk_size).expect("step") {
                ZipStep::InProgress {
                    bytes_done,
                    bytes_total,
                } => {
                    assert_eq!(bytes_total, expected_total);
                    assert!(bytes_done >= prev_done, "progress must be monotonic");
                    assert!(bytes_done <= bytes_total);
                    prev_done = bytes_done;
                }
                ZipStep::Done(bytes) => break bytes,
            }
        };
        assert!(
            steps > 1,
            "multi-MB payload at chunk_size={chunk_size} must take > 1 step"
        );
        assert_eq!(
            result, expected,
            "PackageZipper output must be byte-identical to zip_package at chunk_size={chunk_size}"
        );
    }

    // Empty package edge case.
    let empty_expected = zip_package(&[]).expect("empty zip");
    let mut empty_zipper = PackageZipper::new(Vec::new());
    match empty_zipper.step(256 * 1024).expect("empty step") {
        ZipStep::Done(bytes) => assert_eq!(bytes, empty_expected),
        ZipStep::InProgress { .. } => panic!("empty package must finish in one step"),
    }
}

/// G-04: `build_package_shared` shares `Arc<[u8]>` media buffers with zero
/// copies (`Arc::ptr_eq`), emits compact `course.json` without pretty-print
/// newlines (including when stripping templates), and zips identically to
/// `zip_package`.
#[test]
fn g04_build_package_shared_zero_copy_and_compact_course_json() {
    let mut course = minimal_course("Zero Copy Shared");
    course.scenes[0].slides[0].objects.push(Object {
        id: "img1".into(),
        name: "Hero".into(),
        kind: ObjectKind::Image {
            crop: None,
            src: "hero.png".into(),
            alt: "Hero image".into(),
            outline: None,
            shape: None,
        },
        frame: Rect {
            x: 0.0,
            y: 0.0,
            w: 200.0,
            h: 120.0,
        },
        timing: Timing::default(),
        states: Vec::new(),
        initial_state: "normal".into(),
        entrance: None,
        exit: None,
        motion_paths: Vec::new(),
        focus_order: None,
        decorative: false,
        description: None,
        group: None,
        panel: None,
        rotation: 0.0,
        flip_h: false,
        flip_v: false,
        opacity: 1.0,
        shadow: None,
        glow: None,
        drag: None,
        locked: false,
        editor_hidden: false,
    });
    course.scenes[0].slides[0]
        .objects
        .push(web_object("web1", "web/widget", "index.html"));

    // Add a template so template-stripping also exercises compact re-serialization.
    let mut template = course.scenes[0].slides[0].clone();
    template.id = "tpl1".into();
    template.title = "Template".into();
    template.objects.clear();
    course.templates.push(template);

    let hero_arc: Arc<[u8]> = Arc::from(vec![0x89, b'P', b'N', b'G', 1, 2, 3, 4]);
    let web_index_arc: Arc<[u8]> = Arc::from(b"<!doctype html><html>widget</html>".to_vec());

    let compact_json = course.to_json_compact().expect("compact json");
    assert!(
        !compact_json.contains('\n'),
        "Course::to_json_compact must not emit newlines"
    );

    let player_files = fake_player();
    let resolve_media =
        |src: &str| -> Option<Arc<[u8]>> { (src == "hero.png").then(|| Arc::clone(&hero_arc)) };
    let resolve_folder = |folder: &str| -> Option<Vec<(String, Arc<[u8]>)>> {
        (folder == "web/widget")
            .then(|| vec![("web/widget/index.html".into(), Arc::clone(&web_index_arc))])
    };

    let shared_input = SharedPackageInput {
        course: &course,
        course_json: &compact_json,
        player_files: &player_files,
        media: &resolve_media,
    };
    let shared_files = build_package_shared(
        &shared_input,
        PackageStandard::Scorm(Standard::Scorm12),
        &resolve_folder,
    )
    .expect("build_package_shared succeeds");

    // Zero-copy guarantee: output Arcs point to the exact same allocation as input Arcs.
    let packaged_hero = shared_files
        .iter()
        .find(|f| f.path == "media/hero.png")
        .expect("media/hero.png present");
    assert!(
        Arc::ptr_eq(&packaged_hero.bytes, &hero_arc),
        "media/hero.png must share the input Arc without copying"
    );

    let packaged_web = shared_files
        .iter()
        .find(|f| f.path == "media/web/widget/index.html")
        .expect("web folder entry present");
    assert!(
        Arc::ptr_eq(&packaged_web.bytes, &web_index_arc),
        "web folder file must share the input Arc without copying"
    );

    // Published course.json is compact (no pretty-print newlines) and round-trips.
    let packaged_course = shared_files
        .iter()
        .find(|f| f.path == "course.json")
        .expect("course.json present");
    let published_json = std::str::from_utf8(&packaged_course.bytes).expect("utf8 course.json");
    assert!(
        !published_json.contains('\n'),
        "published course.json must be compact without pretty-print newlines"
    );
    let round_tripped = Course::from_json(published_json).expect("round-trip course.json");
    let mut expected_course = course.clone();
    expected_course.templates.clear();
    assert_eq!(round_tripped, expected_course);

    // zip_shared_package and zip_shared_package_to match zip_package byte-for-byte.
    let legacy_files: Vec<PackageFile> = shared_files
        .iter()
        .map(|f| PackageFile {
            path: f.path.clone(),
            bytes: f.bytes.to_vec(),
        })
        .collect();
    let expected_zip = zip_package(&legacy_files).expect("zip_package");
    let shared_zip = zip_shared_package(&shared_files).expect("zip_shared_package");
    assert_eq!(shared_zip, expected_zip);

    let mut streamed = std::io::Cursor::new(Vec::new());
    zip_shared_package_to(&shared_files, &mut streamed).expect("zip_shared_package_to");
    assert_eq!(streamed.into_inner(), expected_zip);
}

#[test]
fn r1_02_publish_media_walker_golden_and_errors() {
    let json = r##"{
        "schema_version": 1,
        "id": "r1-02-publish",
        "title": "R1-02 Publish Media",
        "slide_size": { "w": 1280, "h": 720 },
        "fonts": [
            { "family": "Lato", "style": "regular", "src": "fonts/lato.ttf" }
        ],
        "background_audio": { "src": "bgm.mp3" },
        "player": { "logo": "logo.png" },
        "resources": [
            { "id": "res-file", "title": "Guide", "target": { "kind": "file", "src": "handout.pdf" } },
            { "id": "res-url", "title": "Site", "target": { "kind": "url", "url": "https://example.com" } }
        ],
        "scenes": [{
            "id": "sc1",
            "title": "Scene 1",
            "slides": [{
                "id": "s1",
                "title": "Slide 1",
                "duration_ms": 5000,
                "objects": [
                    {
                        "id": "obj-img",
                        "name": "Hero",
                        "kind": { "type": "image", "src": "hero.png", "alt": "" },
                        "frame": { "x": 0.0, "y": 0.0, "w": 100.0, "h": 100.0 }
                    },
                    {
                        "id": "obj-lot",
                        "name": "Anim",
                        "kind": { "type": "lottie", "src": "anim.json" },
                        "frame": { "x": 0.0, "y": 0.0, "w": 100.0, "h": 100.0 }
                    },
                    {
                        "id": "obj-aud",
                        "name": "Voice",
                        "kind": { "type": "audio", "src": "voice.mp3", "captions": "voice.vtt" },
                        "frame": { "x": 0.0, "y": 0.0, "w": 48.0, "h": 48.0 }
                    },
                    {
                        "id": "obj-vid",
                        "name": "Clip",
                        "kind": {
                            "type": "video",
                            "src": "clip.mp4",
                            "captions": "clip.vtt",
                            "poster": "clip-poster.png"
                        },
                        "frame": { "x": 0.0, "y": 0.0, "w": 320.0, "h": 180.0 }
                    },
                    {
                        "id": "obj-vid-empty-poster",
                        "name": "ClipEmptyPoster",
                        "kind": {
                            "type": "video",
                            "src": "clip2.mp4",
                            "poster": ""
                        },
                        "frame": { "x": 0.0, "y": 0.0, "w": 320.0, "h": 180.0 }
                    },
                    {
                        "id": "obj-shp",
                        "name": "ShapePic",
                        "kind": {
                            "type": "shape",
                            "shape": "rectangle",
                            "fill": "#ffffff",
                            "fill_style": { "type": "picture", "src": "shape-fill.png" }
                        },
                        "frame": { "x": 0.0, "y": 0.0, "w": 100.0, "h": 100.0 }
                    },
                    {
                        "id": "obj-shp-empty",
                        "name": "ShapeEmptyPic",
                        "kind": {
                            "type": "shape",
                            "shape": "rectangle",
                            "fill": "#ffffff",
                            "fill_style": { "type": "picture", "src": "" }
                        },
                        "frame": { "x": 0.0, "y": 0.0, "w": 100.0, "h": 100.0 }
                    },
                    {
                        "id": "obj-btn",
                        "name": "BtnPic",
                        "kind": {
                            "type": "button",
                            "label": "Go",
                            "fill_style": { "type": "picture", "src": "btn-fill.png" }
                        },
                        "frame": { "x": 0.0, "y": 0.0, "w": 120.0, "h": 44.0 }
                    },
                    {
                        "id": "obj-path",
                        "name": "PathPic",
                        "kind": {
                            "type": "path",
                            "points": [
                                { "x": 0.0, "y": 0.0 },
                                { "x": 1.0, "y": 0.0 },
                                { "x": 0.5, "y": 1.0 }
                            ],
                            "closed": true,
                            "fill": "#ffffff",
                            "fill_style": { "type": "picture", "src": "path-fill.png" },
                            "outline": { "color": "#000000", "width": 1.0 }
                        },
                        "frame": { "x": 0.0, "y": 0.0, "w": 100.0, "h": 100.0 }
                    },
                    {
                        "id": "obj-web",
                        "name": "DemoWeb",
                        "kind": {
                            "type": "web",
                            "source": { "type": "folder", "path": "web/demo", "entry": "index.html" }
                        },
                        "frame": { "x": 0.0, "y": 0.0, "w": 400.0, "h": 300.0 }
                    },
                    {
                        "id": "obj-st",
                        "name": "Stateful",
                        "kind": { "type": "image", "src": "base-state.png", "alt": "" },
                        "frame": { "x": 0.0, "y": 0.0, "w": 100.0, "h": 100.0 },
                        "states": [
                            {
                                "name": "hover",
                                "src": "state-img.png",
                                "fill_style": { "type": "picture", "src": "state-fill.png" },
                                "extras": [
                                    {
                                        "id": "ex-img",
                                        "kind": { "type": "image", "src": "extra-img.png" },
                                        "frame": { "x": 0.0, "y": 0.0, "w": 24.0, "h": 24.0 }
                                    },
                                    {
                                        "id": "ex-shp",
                                        "kind": {
                                            "type": "shape",
                                            "shape": "ellipse",
                                            "fill": "#ffffff",
                                            "fill_style": { "type": "picture", "src": "extra-shape-fill.png" }
                                        },
                                        "frame": { "x": 0.0, "y": 0.0, "w": 24.0, "h": 24.0 }
                                    },
                                    {
                                        "id": "ex-path",
                                        "kind": {
                                            "type": "path",
                                            "points": [
                                                { "x": 0.0, "y": 0.0 },
                                                { "x": 1.0, "y": 0.0 },
                                                { "x": 0.5, "y": 1.0 }
                                            ],
                                            "closed": true,
                                            "fill": "#ffffff",
                                            "fill_style": { "type": "picture", "src": "extra-path-fill.png" },
                                            "outline": { "color": "#000000", "width": 1.0 }
                                        },
                                        "frame": { "x": 0.0, "y": 0.0, "w": 24.0, "h": 24.0 }
                                    },
                                    {
                                        "id": "ex-txt",
                                        "kind": { "type": "text", "text": "Plain" },
                                        "frame": { "x": 0.0, "y": 0.0, "w": 40.0, "h": 20.0 }
                                    }
                                ]
                            },
                            {
                                "name": "empty_optional",
                                "src": "",
                                "fill_style": { "type": "picture", "src": "" },
                                "extras": [{
                                    "id": "ex-empty-img",
                                    "kind": { "type": "image", "src": "" },
                                    "frame": { "x": 0.0, "y": 0.0, "w": 24.0, "h": 24.0 }
                                }]
                            }
                        ]
                    }
                ],
                "layers": [{
                    "id": "l1",
                    "name": "Layer 1",
                    "objects": [{
                        "id": "l-img",
                        "name": "LayerPic",
                        "kind": { "type": "image", "src": "layer-pic.png", "alt": "" },
                        "frame": { "x": 0.0, "y": 0.0, "w": 60.0, "h": 60.0 }
                    }]
                }]
            }]
        }],
        "masters": [{
            "id": "m1",
            "title": "Master 1",
            "duration_ms": 5000,
            "objects": [{
                "id": "m-img",
                "name": "MasterBg",
                "kind": { "type": "image", "src": "master-base.png", "alt": "" },
                "frame": { "x": 0.0, "y": 0.0, "w": 100.0, "h": 100.0 }
            }],
            "layers": [{
                "id": "ml1",
                "name": "Master Layer",
                "objects": [{
                    "id": "ml-img",
                    "name": "MasterLayerIgnored",
                    "kind": { "type": "image", "src": "master-layer-ignored.png", "alt": "" },
                    "frame": { "x": 0.0, "y": 0.0, "w": 40.0, "h": 40.0 }
                }]
            }]
        }],
        "templates": [{
            "id": "t1",
            "title": "Template 1",
            "duration_ms": 5000,
            "objects": [{
                "id": "t-img",
                "name": "TemplateIgnored",
                "kind": { "type": "image", "src": "tpl-missing.png", "alt": "" },
                "frame": { "x": 0.0, "y": 0.0, "w": 100.0, "h": 100.0 }
            }]
        }],
        "banks": [{
            "id": "b1",
            "title": "Bank 1",
            "slides": [{
                "id": "bs1",
                "title": "Bank Slide 1",
                "duration_ms": 5000,
                "objects": [{
                    "id": "b-img",
                    "name": "BankPic",
                    "kind": { "type": "image", "src": "bank-base.png", "alt": "" },
                    "frame": { "x": 0.0, "y": 0.0, "w": 100.0, "h": 100.0 }
                }],
                "layers": [{
                    "id": "bl1",
                    "name": "Bank Layer",
                    "objects": [{
                        "id": "bl-img",
                        "name": "BankLayerPic",
                        "kind": { "type": "image", "src": "bank-layer.png", "alt": "" },
                        "frame": { "x": 0.0, "y": 0.0, "w": 50.0, "h": 50.0 }
                    }]
                }]
            }]
        }]
    }"##;
    let course = Course::from_json(json).expect("valid R1-02 course");
    let course_json = course.to_json_compact().expect("compact json");
    let player_files = fake_player();

    let all_media: std::collections::HashMap<&str, Vec<u8>> = [
        ("fonts/lato.ttf", b"font-bytes".to_vec()),
        ("bgm.mp3", b"bgm-bytes".to_vec()),
        ("logo.png", b"logo-bytes".to_vec()),
        ("handout.pdf", b"pdf-bytes".to_vec()),
        ("hero.png", b"hero-bytes".to_vec()),
        ("anim.json", b"{}".to_vec()),
        ("voice.mp3", b"voice-bytes".to_vec()),
        ("voice.vtt", b"WEBVTT".to_vec()),
        ("clip.mp4", b"clip-bytes".to_vec()),
        ("clip.vtt", b"WEBVTT clip".to_vec()),
        ("clip-poster.png", b"poster-bytes".to_vec()),
        ("clip2.mp4", b"clip2-bytes".to_vec()),
        ("shape-fill.png", b"shape-bytes".to_vec()),
        ("btn-fill.png", b"btn-bytes".to_vec()),
        ("path-fill.png", b"path-bytes".to_vec()),
        ("base-state.png", b"base-state-bytes".to_vec()),
        ("state-img.png", b"state-img-bytes".to_vec()),
        ("state-fill.png", b"state-fill-bytes".to_vec()),
        ("extra-img.png", b"extra-img-bytes".to_vec()),
        ("extra-shape-fill.png", b"extra-shape-bytes".to_vec()),
        ("extra-path-fill.png", b"extra-path-bytes".to_vec()),
        ("layer-pic.png", b"layer-bytes".to_vec()),
        ("master-base.png", b"master-bytes".to_vec()),
        ("bank-base.png", b"bank-base-bytes".to_vec()),
        ("bank-layer.png", b"bank-layer-bytes".to_vec()),
    ]
    .into_iter()
    .collect();

    let resolve_media = |src: &str| -> Option<Vec<u8>> { all_media.get(src).cloned() };
    let resolve_folder = |folder: &str| -> Option<Vec<(String, Vec<u8>)>> {
        (folder == "web/demo").then(|| {
            vec![
                ("web/demo/index.html".into(), b"<h1>Demo</h1>".to_vec()),
                ("web/demo/app.js".into(), b"console.log(1);".to_vec()),
            ]
        })
    };

    let input = PackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &player_files,
        media: &resolve_media,
    };
    let files1 = build_package_standard_with_folders(
        &input,
        PackageStandard::Scorm(Standard::Scorm12),
        &resolve_folder,
    )
    .expect("publish succeeds with all media present");
    let files2 = build_package_standard_with_folders(
        &input,
        PackageStandard::Scorm(Standard::Scorm12),
        &resolve_folder,
    )
    .expect("second publish succeeds");

    let zip1 = zip_package(&files1).expect("zip1");
    let zip2 = zip_package(&files2).expect("zip2");
    assert_eq!(zip1, zip2, "published zip must be deterministic");

    let media_paths: Vec<&str> = files1
        .iter()
        .map(|f| f.path.as_str())
        .filter(|p| p.starts_with("media/"))
        .collect();
    assert_eq!(
        media_paths,
        vec![
            "media/anim.json",
            "media/bank-base.png",
            "media/bank-layer.png",
            "media/base-state.png",
            "media/bgm.mp3",
            "media/btn-fill.png",
            "media/clip-poster.png",
            "media/clip.mp4",
            "media/clip.vtt",
            "media/clip2.mp4",
            "media/extra-img.png",
            "media/extra-path-fill.png",
            "media/extra-shape-fill.png",
            "media/fonts/lato.ttf",
            "media/handout.pdf",
            "media/hero.png",
            "media/layer-pic.png",
            "media/logo.png",
            "media/master-base.png",
            "media/path-fill.png",
            "media/shape-fill.png",
            "media/state-fill.png",
            "media/state-img.png",
            "media/voice.mp3",
            "media/voice.vtt",
            "media/web/demo/app.js",
            "media/web/demo/index.html",
        ]
    );

    // Error-selection parity: empty primary src on every required slot triggers UnsafePath
    // with the exact expected object id.
    let run_expect_unsafe = |mutated: &Course, expected_object: &str| {
        let json_str = mutated.to_json_compact().expect("json");
        let inp = PackageInput {
            course: mutated,
            course_json: &json_str,
            player_files: &player_files,
            media: &|_| Some(vec![1]),
        };
        let err = build_package(&inp).expect_err("empty primary src must fail");
        assert_eq!(
            err,
            PublishError::UnsafePath {
                object: expected_object.to_string(),
                src: String::new(),
            },
            "mismatch for {expected_object}"
        );
    };

    let mut base_empty = minimal_course("Empty checks");
    let make_obj = |id: &str, kind: ObjectKind| Object {
        id: id.into(),
        name: id.into(),
        kind,
        frame: Rect {
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
        },
        timing: Timing::default(),
        entrance: None,
        exit: None,
        motion_paths: Vec::new(),
        states: Vec::new(),
        initial_state: "normal".into(),
        focus_order: None,
        decorative: false,
        description: None,
        group: None,
        panel: None,
        rotation: 0.0,
        flip_h: false,
        flip_v: false,
        opacity: 1.0,
        shadow: None,
        glow: None,
        drag: None,
        locked: false,
        editor_hidden: false,
    };

    {
        let mut c = base_empty.clone();
        c.scenes[0].slides[0].objects.push(make_obj(
            "img-empty",
            ObjectKind::Image {
                src: String::new(),
                alt: String::new(),
                crop: None,
                outline: None,
                shape: None,
            },
        ));
        run_expect_unsafe(&c, "img-empty");
    }
    {
        let mut c = base_empty.clone();
        c.scenes[0].slides[0].objects.push(make_obj(
            "lot-empty",
            ObjectKind::Lottie {
                src: String::new(),
                loop_playback: true,
                autoplay: true,
                speed: 1.0,
            },
        ));
        run_expect_unsafe(&c, "lot-empty");
    }
    {
        let mut c = base_empty.clone();
        c.scenes[0].slides[0].objects.push(make_obj(
            "aud-empty",
            ObjectKind::Audio {
                src: String::new(),
                autoplay: false,
                captions: None,
                volume: 1.0,
                trim: None,
            },
        ));
        run_expect_unsafe(&c, "aud-empty");
    }
    {
        let mut c = base_empty.clone();
        c.scenes[0].slides[0].objects.push(make_obj(
            "aud-cap-empty",
            ObjectKind::Audio {
                src: "ok.mp3".into(),
                autoplay: false,
                captions: Some(String::new()),
                volume: 1.0,
                trim: None,
            },
        ));
        run_expect_unsafe(&c, "aud-cap-empty");
    }
    {
        let mut c = base_empty.clone();
        c.scenes[0].slides[0].objects.push(make_obj(
            "vid-empty",
            ObjectKind::Video {
                src: String::new(),
                autoplay: false,
                captions: None,
                volume: 1.0,
                trim: None,
                cues: Vec::new(),
                poster: None,
            },
        ));
        run_expect_unsafe(&c, "vid-empty");
    }
    {
        let mut c = base_empty.clone();
        c.scenes[0].slides[0].objects.push(make_obj(
            "vid-cap-empty",
            ObjectKind::Video {
                src: "ok.mp4".into(),
                autoplay: false,
                captions: Some(String::new()),
                volume: 1.0,
                trim: None,
                cues: Vec::new(),
                poster: None,
            },
        ));
        run_expect_unsafe(&c, "vid-cap-empty");
    }
    {
        let mut c = base_empty.clone();
        c.background_audio = Some(BackgroundAudio {
            src: String::new(),
            volume: 0.5,
            looping: true,
        });
        run_expect_unsafe(&c, BACKGROUND_AUDIO_ID);
    }
    {
        let mut c = base_empty.clone();
        c.fonts.push(FontFace {
            family: "Lato".into(),
            style: FontStyle::Regular,
            src: String::new(),
        });
        run_expect_unsafe(&c, "font");
    }
    {
        let mut c = base_empty.clone();
        c.player.logo = Some(String::new());
        run_expect_unsafe(&c, "player-logo");
    }
    {
        let mut c = base_empty.clone();
        c.resources.push(coursecraft_course::Resource {
            id: "res-empty".into(),
            title: "Empty".into(),
            target: coursecraft_course::ResourceTarget::File { src: String::new() },
        });
        run_expect_unsafe(&c, "res-empty");
    }

    // Missing-media error ordering: slide object reported before background_audio,
    // and background_audio reported before font.
    base_empty.background_audio = Some(BackgroundAudio {
        src: "missing-bgm.mp3".into(),
        volume: 0.5,
        looping: true,
    });
    base_empty.fonts.push(FontFace {
        family: "Lato".into(),
        style: FontStyle::Regular,
        src: "fonts/missing.ttf".into(),
    });
    {
        let mut c = base_empty.clone();
        c.scenes[0].slides[0].objects.push(make_obj(
            "obj-missing",
            ObjectKind::Image {
                src: "missing-slide.png".into(),
                alt: String::new(),
                crop: None,
                outline: None,
                shape: None,
            },
        ));
        let json_str = c.to_json_compact().expect("json");
        let inp = PackageInput {
            course: &c,
            course_json: &json_str,
            player_files: &player_files,
            media: &|_| None,
        };
        assert_eq!(
            build_package(&inp),
            Err(PublishError::MissingMedia {
                object: "obj-missing".into(),
                src: "missing-slide.png".into(),
            })
        );
    }
    {
        let json_str = base_empty.to_json_compact().expect("json");
        let inp = PackageInput {
            course: &base_empty,
            course_json: &json_str,
            player_files: &player_files,
            media: &|_| None,
        };
        assert_eq!(
            build_package(&inp),
            Err(PublishError::MissingMedia {
                object: BACKGROUND_AUDIO_ID.into(),
                src: "missing-bgm.mp3".into(),
            })
        );
    }
}

#[test]
fn r2_01_shared_package_zipper_matches_zip_package_across_chunk_sizes() {
    let mut course = minimal_course("Shared Zipper Parity");
    course.scenes[0].slides[0].objects.push(Object {
        id: "hero-img".into(),
        name: "Hero".into(),
        kind: ObjectKind::Image {
            crop: None,
            src: "hero.png".into(),
            alt: "Hero image".into(),
            outline: None,
            shape: None,
        },
        frame: Rect {
            x: 0.0,
            y: 0.0,
            w: 200.0,
            h: 120.0,
        },
        timing: Timing::default(),
        states: Vec::new(),
        initial_state: "normal".into(),
        entrance: None,
        exit: None,
        motion_paths: Vec::new(),
        focus_order: None,
        decorative: false,
        description: None,
        group: None,
        panel: None,
        rotation: 0.0,
        flip_h: false,
        flip_v: false,
        opacity: 1.0,
        shadow: None,
        glow: None,
        drag: None,
        locked: false,
        editor_hidden: false,
    });
    course.background_audio = Some(BackgroundAudio {
        src: "audio.mp3".into(),
        volume: 0.8,
        looping: true,
    });
    course.resources.push(coursecraft_course::Resource {
        id: "empty-res".into(),
        title: "Empty".into(),
        target: coursecraft_course::ResourceTarget::File {
            src: "empty.txt".into(),
        },
    });

    let hero_png: Arc<[u8]> = Arc::from(vec![0x89u8; 4096]);
    let audio_mp3: Arc<[u8]> = Arc::from(
        (0..70_000usize)
            .map(|i| (i % 251) as u8)
            .collect::<Vec<_>>(),
    );
    let empty_bytes: Arc<[u8]> = Arc::from(Vec::<u8>::new());

    let course_json = course.to_json_compact().expect("course json");
    let player_files = fake_player();
    let resolve = |src: &str| -> Option<Arc<[u8]>> {
        match src {
            "hero.png" => Some(Arc::clone(&hero_png)),
            "audio.mp3" => Some(Arc::clone(&audio_mp3)),
            "empty.txt" => Some(Arc::clone(&empty_bytes)),
            _ => None,
        }
    };
    let input = SharedPackageInput {
        course: &course,
        course_json: &course_json,
        player_files: &player_files,
        media: &resolve,
    };

    let shared_files =
        build_package_shared(&input, PackageStandard::Scorm(Standard::Scorm12), &|_| None)
            .expect("build shared package");

    let paths: Vec<&str> = shared_files.iter().map(|f| f.path.as_str()).collect();
    for expected_path in [
        "index.html",
        "course.json",
        "imsmanifest.xml",
        "media/hero.png",
        "media/audio.mp3",
        "media/empty.txt",
    ] {
        assert!(
            paths.contains(&expected_path),
            "{expected_path} missing from {paths:?}"
        );
    }

    let expected_shared = zip_shared_package(&shared_files).expect("zip_shared_package");
    let owned_files: Vec<PackageFile> = shared_files
        .iter()
        .map(|f| PackageFile {
            path: f.path.clone(),
            bytes: f.bytes.to_vec(),
        })
        .collect();
    let expected_owned = zip_package(&owned_files).expect("zip_package");
    assert_eq!(expected_shared, expected_owned);

    for chunk_size in [1, 17, 1024, 65_536, 262_144] {
        let mut zipper = SharedPackageZipper::new(shared_files.clone());
        let zipped = loop {
            match zipper.step(chunk_size).expect("zipper step") {
                ZipStep::InProgress {
                    bytes_done,
                    bytes_total,
                } => {
                    assert!(bytes_done <= bytes_total);
                }
                ZipStep::Done(bytes) => break bytes,
            }
        };
        assert_eq!(
            zipped, expected_shared,
            "SharedPackageZipper mismatch for chunk_size={chunk_size}"
        );
        assert!(
            matches!(zipper.step(chunk_size), Err(PublishError::Zip(_))),
            "calling step after Done must return Err(PublishError::Zip(_))"
        );
    }

    let expected_empty = zip_shared_package(&[]).expect("zip_shared_package empty");
    assert_eq!(expected_empty, zip_package(&[]).expect("zip_package empty"));
    let mut empty_zipper = SharedPackageZipper::new(Vec::new());
    let done = empty_zipper.step(1024).expect("empty zipper step");
    assert_eq!(done, ZipStep::Done(expected_empty));
    assert!(matches!(empty_zipper.step(1024), Err(PublishError::Zip(_))));
}
