use super::*;
use coursecraft_course::{
    Course, Object, ObjectKind, PlayerSettings, Rect, Scene, Size, Slide, Timing,
};

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
                groups: Vec::new(),
            }],
        }],
        masters: Vec::new(),
        templates: Vec::new(),
        banks: Vec::new(),
        start_slide: None,
        quiz: None,
        tracking: Default::default(),
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

#[test]
fn ccraft_round_trip_keeps_fonts_and_their_media() {
    let mut course = minimal_course("Fonts");
    course.fonts.push(coursecraft_course::FontFace {
        family: "Lato".into(),
        style: coursecraft_course::FontStyle::Bold,
        src: "fonts/lato-bold.ttf".into(),
    });
    let media = vec![("fonts/lato-bold.ttf".to_string(), b"font".to_vec())];
    let saved = save_project(&course, &media).expect("save");
    let opened = open_project(&saved).expect("open");
    assert_eq!(opened.course.fonts, course.fonts);
    assert_eq!(opened.media, media);
}

// ---------------------------------------------------------------------------
// Moved tests for is_safe_relative_path
// ---------------------------------------------------------------------------

#[test]
fn safe_relative_path_accepts_plain_paths() {
    for p in [
        "index.html",
        "app.js",
        "media/pic.png",
        "assets/icons/star.svg",
    ] {
        assert!(is_safe_relative_path(p), "should accept {p:?}");
    }
}

#[test]
fn safe_relative_path_rejects_separators_and_schemes() {
    for p in [
        "",
        "/index.html",
        "/media/pic.png",
        "media\\pic.png",
        "a\\b\\c",
        "C:file.png",
        "C:/file.png",
        "file://foo.png",
        "http://foo.com/pic.png",
        "javascript:alert(1)",
    ] {
        assert!(!is_safe_relative_path(p), "should reject {p:?}");
    }
}

#[test]
fn safe_relative_path_rejects_bad_segments() {
    for p in [
        "foo//bar",
        "foo/bar/",
        "/foo/bar",
        ".",
        "foo/./bar",
        "./bar",
        "bar/.",
        "..",
        "foo/../bar",
        "../bar",
        "bar/..",
    ] {
        assert!(!is_safe_relative_path(p), "should reject {p:?}");
    }
}

#[test]
fn safe_relative_path_rejects_control_chars() {
    for p in [
        "foo\0bar",
        "foo\nbar",
        "foo\rbar",
        "foo\tbar",
        "foo\x1fbar",
        "foo\x7fbar",
    ] {
        assert!(!is_safe_relative_path(p), "should reject {p:?}");
    }
}

// ---------------------------------------------------------------------------
// Format tests
// ---------------------------------------------------------------------------

#[test]
fn round_trip_hello_course_and_media() {
    let course = Course::from_json(HELLO_COURSE_JSON).expect("valid hello-course");
    let media = vec![
        (
            "images/badge.png".to_string(),
            vec![0x89, b'P', b'N', b'G', 1, 2, 3],
        ),
        (
            "audio/chime.wav".to_string(),
            vec![b'R', b'I', b'F', b'F', 4, 5, 6],
        ),
    ];

    let bytes = save_project(&course, &media).expect("save_project succeeds");
    let project = open_project(&bytes).expect("open_project succeeds");

    assert_eq!(project.course, course);
    assert_eq!(project.media, media);
}

#[test]
fn deterministic_bytes() {
    let course = Course::from_json(HELLO_COURSE_JSON).expect("valid hello-course");
    let media = vec![
        ("img1.png".to_string(), vec![1, 2, 3, 4]),
        ("snd1.mp3".to_string(), vec![5, 6, 7, 8]),
    ];

    let bytes1 = save_project(&course, &media).expect("save first time");
    let bytes2 = save_project(&course, &media).expect("save second time");

    assert_eq!(bytes1, bytes2, "output bytes must be deterministic");
}

#[test]
fn missing_course_json() {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    writer
        .start_file("media/icon.png", options)
        .expect("start file");
    writer.write_all(b"fake png").expect("write file");
    let bytes = writer.finish().expect("finish").into_inner();

    let err = open_project(&bytes).expect_err("should fail without course.json");
    assert_eq!(err, FormatError::MissingCourseJson);
}

#[test]
fn bad_json() {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    writer
        .start_file("course.json", options)
        .expect("start file");
    writer
        .write_all(b"{ not valid json: true }")
        .expect("write file");
    let bytes = writer.finish().expect("finish").into_inner();

    let err = open_project(&bytes).expect_err("should fail on invalid JSON");
    assert!(
        matches!(err, FormatError::CourseJson(_)),
        "expected CourseJson error, got {err:?}"
    );
}

#[test]
fn unsafe_entry_name() {
    // 1. Handcrafted zip with "../evil"
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    let course_json = Course::to_json(&minimal_course("test")).expect("to_json");
    writer
        .start_file("course.json", options)
        .expect("start course.json");
    writer
        .write_all(course_json.as_bytes())
        .expect("write course.json");
    writer
        .start_file("../evil", options)
        .expect("start ../evil");
    writer.write_all(b"payload").expect("write evil");
    let bytes = writer.finish().expect("finish").into_inner();

    let err = open_project(&bytes).expect_err("should reject ../evil");
    assert_eq!(err, FormatError::UnsafePath("../evil".to_string()));

    // 2. Handcrafted zip with "/abs"
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    writer
        .start_file("course.json", options)
        .expect("start course.json");
    writer
        .write_all(course_json.as_bytes())
        .expect("write course.json");
    writer.start_file("/abs", options).expect("start /abs");
    writer.write_all(b"payload").expect("write abs");
    let bytes = writer.finish().expect("finish").into_inner();

    let err = open_project(&bytes).expect_err("should reject /abs");
    assert_eq!(err, FormatError::UnsafePath("/abs".to_string()));

    // 3. save_project rejects unsafe media paths
    let course = minimal_course("test");
    let bad_media = vec![("../evil.png".to_string(), vec![1, 2, 3])];
    let err = save_project(&course, &bad_media).expect_err("save rejects unsafe path");
    assert_eq!(err, FormatError::UnsafePath("../evil.png".to_string()));
}

#[test]
fn duplicate_entry() {
    // 1. save_project rejects duplicate media paths
    let course = minimal_course("test");
    let dup_media = vec![
        ("badge.png".to_string(), vec![1, 2]),
        ("badge.png".to_string(), vec![3, 4]),
    ];
    let err = save_project(&course, &dup_media).expect_err("save rejects duplicates");
    assert_eq!(err, FormatError::DuplicateEntry("badge.png".to_string()));

    // 2. zip 9 ZipWriter prevents writing duplicate filenames at creation time.
    // As noted in task requirements ("a duplicate entry, if zip 9 lets you write one
    // (otherwise skip and say so)"): zip 9's ZipWriter returns an error if start_file
    // is called with a name that is already in the archive.
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    writer.start_file("file.txt", options).expect("first file");
    let dup_res = writer.start_file("file.txt", options);
    assert!(
        dup_res.is_err(),
        "zip 9 does not allow writing duplicate filenames via ZipWriter"
    );
}

#[test]
fn too_many_entries() {
    let course = minimal_course("test");
    let media = vec![
        ("m1.png".to_string(), vec![1]),
        ("m2.png".to_string(), vec![2]),
        ("m3.png".to_string(), vec![3]),
    ];
    let bytes = save_project(&course, &media).expect("save succeeds");

    // 4 total entries: course.json + 3 media files. Limit to 3.
    let limits = Limits {
        max_entries: 3,
        ..Limits::default()
    };
    let err = open_project_with_limits(&bytes, limits).expect_err("should reject too many entries");
    assert_eq!(err, FormatError::TooManyEntries);
}

#[test]
fn entry_over_small_limit() {
    let course = minimal_course("test");
    let media = vec![("large.png".to_string(), vec![0u8; 5000])];
    let bytes = save_project(&course, &media).expect("save succeeds");

    // course.json is ~500 bytes. Cap single entry bytes at 2000 bytes, so
    // course.json fits but media/large.png exceeds the limit.
    let limits = Limits {
        max_entry_bytes: 2000,
        ..Limits::default()
    };
    let err =
        open_project_with_limits(&bytes, limits).expect_err("should reject oversized single entry");
    assert_eq!(
        err,
        FormatError::EntryTooLarge("media/large.png".to_string())
    );
}

#[test]
fn total_archive_over_small_limit() {
    let course = minimal_course("test");
    let media = vec![
        ("m1.png".to_string(), vec![0u8; 60]),
        ("m2.png".to_string(), vec![0u8; 60]),
    ];
    let bytes = save_project(&course, &media).expect("save succeeds");

    // Total bytes will exceed 120 bytes. Set max_total_bytes to 80.
    let limits = Limits {
        max_entry_bytes: 1000,
        max_total_bytes: 80,
        ..Limits::default()
    };
    let err = open_project_with_limits(&bytes, limits).expect_err("should reject oversized total");
    assert_eq!(err, FormatError::TooLarge);
}

#[test]
fn unknown_non_media_entries_ignored() {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    let course_json = Course::to_json(&minimal_course("test")).expect("to_json");

    writer
        .start_file("course.json", options)
        .expect("start course.json");
    writer
        .write_all(course_json.as_bytes())
        .expect("write course.json");

    writer
        .start_file("media/icon.png", options)
        .expect("start media file");
    writer.write_all(b"icon bytes").expect("write media file");

    writer
        .start_file("README.md", options)
        .expect("start readme");
    writer.write_all(b"# Hello").expect("write readme");

    writer
        .start_file("metadata.xml", options)
        .expect("start metadata");
    writer.write_all(b"<xml/>").expect("write metadata");

    let bytes = writer.finish().expect("finish").into_inner();
    let project = open_project(&bytes).expect("open succeeds ignoring unknown entries");

    assert_eq!(project.media.len(), 1);
    assert_eq!(project.media[0].0, "icon.png");
    assert_eq!(project.media[0].1, b"icon bytes");
}

#[test]
fn directory_entries_skipped() {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    let course_json = Course::to_json(&minimal_course("test")).expect("to_json");

    writer
        .start_file("course.json", options)
        .expect("start course.json");
    writer
        .write_all(course_json.as_bytes())
        .expect("write course.json");

    writer
        .add_directory("media", options)
        .expect("add media dir");

    writer
        .start_file("media/photo.jpg", options)
        .expect("start photo");
    writer.write_all(b"photo bytes").expect("write photo");

    let bytes = writer.finish().expect("finish").into_inner();
    let project = open_project(&bytes).expect("open succeeds skipping directories");

    assert_eq!(project.media.len(), 1);
    assert_eq!(project.media[0].0, "photo.jpg");
    assert_eq!(project.media[0].1, b"photo bytes");
}

#[test]
fn unsafe_directory_entry_rejected() {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    let course_json = Course::to_json(&minimal_course("test")).expect("to_json");

    writer
        .start_file("course.json", options)
        .expect("start course.json");
    writer
        .write_all(course_json.as_bytes())
        .expect("write course.json");

    writer
        .add_directory("../evil_dir", options)
        .expect("add evil dir");

    let bytes = writer.finish().expect("finish").into_inner();
    let err = open_project(&bytes).expect_err("should reject unsafe directory");
    assert_eq!(err, FormatError::UnsafePath("../evil_dir/".to_string()));
}

#[test]
fn empty_zip() {
    let writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let bytes = writer.finish().expect("finish").into_inner();

    let err = open_project(&bytes).expect_err("should fail on empty zip");
    assert_eq!(err, FormatError::MissingCourseJson);
}

// Deterministic Linear Congruential Generator for non-panicking pseudo-random test inputs.
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.0 >> 32) as u32
    }

    fn next_bytes(&mut self, len: usize) -> Vec<u8> {
        let mut buf = vec![0u8; len];
        for b in &mut buf {
            *b = self.next_u32() as u8;
        }
        buf
    }
}

#[test]
fn random_garbage_bytes_never_panic() {
    let mut rng = Lcg::new(0x1234_5678_9ABC_DEF0);
    for _ in 0..500 {
        let len = (rng.next_u32() % 2048) as usize;
        let garbage = rng.next_bytes(len);
        let _ = open_project(&garbage);
    }
}

#[test]
fn lottie_animation_survives_a_ccraft_round_trip() {
    let mut course = minimal_course("Lottie");
    course.scenes[0].slides[0].objects.push(Object {
        id: "lot1".into(),
        name: "Animation".into(),
        kind: ObjectKind::Lottie {
            src: "motion.lottie".into(),
            loop_playback: false,
            autoplay: true,
            speed: 2.5,
        },
        frame: Rect {
            x: 10.0,
            y: 20.0,
            w: 320.0,
            h: 180.0,
        },
        timing: Timing::default(),
        states: Vec::new(),
        initial_state: "normal".into(),
        entrance: None,
        exit: None,
        motion_paths: Vec::new(),
        emphasis: Vec::new(),
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
    let media = vec![("motion.lottie".to_string(), b"PK\x03\x04".to_vec())];
    let bytes = save_project(&course, &media).expect("save");
    let project = open_project(&bytes).expect("open");
    assert_eq!(project.course, course);
    assert_eq!(project.media, media);
    assert!(matches!(
        project.course.scenes[0].slides[0].objects[0].kind,
        ObjectKind::Lottie {
            loop_playback: false,
            autoplay: true,
            speed,
            ..
        } if (speed - 2.5).abs() < f32::EPSILON
    ));
}

#[test]
fn caption_files_survive_a_ccraft_round_trip_for_audio_and_video() {
    for (id, kind, vtt) in [
        (
            "audio1",
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
            "video1",
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
        course.scenes[0].slides[0].objects.push(Object {
            id: id.into(),
            name: id.into(),
            kind,
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
            emphasis: Vec::new(),
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
        let media = vec![(vtt.to_string(), b"WEBVTT".to_vec())];
        let bytes = save_project(&course, &media).expect("save");
        let project = open_project(&bytes).expect("open");
        assert_eq!(project.course, course);
        assert_eq!(project.media, media);
        let object = &project.course.scenes[0].slides[0].objects[0];
        let captions = match &object.kind {
            ObjectKind::Audio { captions, .. } | ObjectKind::Video { captions, .. } => {
                captions.as_deref()
            }
            _ => None,
        };
        assert_eq!(captions, Some(vtt));
    }
}

fn method_of(bytes: &[u8], name: &str) -> zip::CompressionMethod {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("zip opens");
    let file = archive.by_name(name).expect("entry exists");
    file.compression()
}

#[test]
fn already_compressed_media_is_stored_and_text_is_deflated() {
    let media = vec![
        ("a.png".to_string(), vec![7u8; 4096]),
        ("b.MP4".to_string(), vec![8u8; 4096]),
        ("notes.txt".to_string(), vec![9u8; 4096]),
    ];
    let bytes = save_project(&minimal_course("t"), &media).expect("save");
    assert_eq!(
        method_of(&bytes, "media/a.png"),
        zip::CompressionMethod::Stored
    );
    assert_eq!(
        method_of(&bytes, "media/b.MP4"),
        zip::CompressionMethod::Stored
    );
    assert_eq!(
        method_of(&bytes, "media/notes.txt"),
        zip::CompressionMethod::Deflated
    );
    assert_eq!(
        method_of(&bytes, "course.json"),
        zip::CompressionMethod::Deflated
    );
    let project = open_project(&bytes).expect("open");
    assert_eq!(project.media.len(), 3);
    assert!(project.media.iter().all(|(_, b)| b.len() == 4096));
}

#[test]
fn files_saved_with_everything_deflated_still_open() {
    let course_json = minimal_course("old").to_json().expect("json");
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    writer.start_file("course.json", options).expect("start");
    writer.write_all(course_json.as_bytes()).expect("write");
    writer.start_file("media/pic.png", options).expect("start");
    writer.write_all(&[3u8; 10_000]).expect("write");
    let bytes = writer.finish().expect("finish").into_inner();
    let project = open_project(&bytes).expect("an older, fully deflated file opens");
    assert_eq!(project.media.len(), 1);
}

#[test]
fn save_project_to_writes_the_same_bytes_as_save_project() {
    let media = vec![("a.png".to_string(), vec![1u8; 300])];
    let course = minimal_course("same");
    let expected = save_project(&course, &media).expect("save");
    let mut out = std::io::Cursor::new(Vec::new());
    save_project_to(&course, &media, &mut out).expect("save to writer");
    assert_eq!(out.into_inner(), expected);
}

#[test]
fn save_project_to_reopens_equal_to_original() {
    let media = vec![
        ("img/logo.png".to_string(), vec![42u8; 256]),
        (
            "captions/intro.vtt".to_string(),
            b"WEBVTT\n\n00:00.000 --> 00:01.000\nHi\n".to_vec(),
        ),
    ];
    let course = minimal_course("streamed");
    let mut out = std::io::Cursor::new(Vec::new());
    save_project_to(&course, &media, &mut out).expect("save_project_to");
    let reopened = open_project(&out.into_inner()).expect("reopen streamed project");
    assert_eq!(reopened.course, course);
    assert_eq!(reopened.media, media);
}

#[test]
fn precompressed_check_ignores_case_and_folders() {
    assert!(is_precompressed("img/Photo.JPG"));
    assert!(is_precompressed("video/clip.webm"));
    assert!(!is_precompressed("captions/a.vtt"));
    assert!(!is_precompressed("noextension"));
    assert!(!is_precompressed("dir.png/readme"));
}

#[test]
fn long_narration_and_documents_are_stored() {
    assert!(is_precompressed("narration/take1.WAV"));
    assert!(is_precompressed("a.aiff"));
    assert!(is_precompressed("handout.pdf"));
}

#[test]
fn verify_refuses_a_project_that_could_not_be_opened_again() {
    let media = vec![
        ("a.png".to_string(), vec![7u8; 5000]),
        ("b.png".to_string(), vec![8u8; 5000]),
    ];
    let bytes = save_project(&minimal_course("v"), &media).expect("save");
    let verify = |limits| verify_project_with_limits(std::io::Cursor::new(&bytes), limits);

    let one_entry_over = Limits {
        max_entry_bytes: 4000,
        ..Limits::default()
    };
    assert_eq!(
        verify(one_entry_over),
        Err(FormatError::EntryTooLarge("media/a.png".to_string()))
    );
    let total_over = Limits {
        max_total_bytes: 9000,
        ..Limits::default()
    };
    assert_eq!(verify(total_over), Err(FormatError::TooLarge));
    let too_many = Limits {
        max_entries: 2,
        ..Limits::default()
    };
    assert_eq!(verify(too_many), Err(FormatError::TooManyEntries));

    // Verify and open agree on the same limits.
    for limits in [one_entry_over, total_over, too_many] {
        assert!(open_project_with_limits(&bytes, limits).is_err());
    }
    assert_eq!(verify(Limits::default()), Ok(()));
}

#[test]
fn verify_project_accepts_a_whole_save_and_rejects_damage() {
    let media = vec![("a.png".to_string(), vec![7u8; 5000])];
    let bytes = save_project(&minimal_course("v"), &media).expect("save");
    verify_project(std::io::Cursor::new(&bytes)).expect("a whole save verifies");

    let mut damaged = bytes.clone();
    let at = damaged
        .windows(100)
        .position(|w| w.iter().all(|b| *b == 7))
        .expect("stored bytes present");
    damaged[at + 50] ^= 0xFF;
    assert!(verify_project(std::io::Cursor::new(&damaged)).is_err());
    assert!(verify_project(std::io::Cursor::new(&bytes[..bytes.len() / 2])).is_err());
    assert!(verify_project(std::io::Cursor::new(Vec::<u8>::new())).is_err());
}

#[test]
fn open_project_with_limits_rejects_inflated_declared_sizes_before_reading() {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        writer
            .start_file("course.json", opts)
            .expect("start course");
        let course_json = minimal_course("hostile").to_json().expect("json");
        std::io::Write::write_all(&mut writer, course_json.as_bytes()).expect("write course");
        writer.start_file("media/a.txt", opts).expect("start media");
        std::io::Write::write_all(&mut writer, b"tiny").expect("write media");
        writer.finish().expect("finish");
    }
    let mut bytes = buf.into_inner();
    let inflated: u32 = 150 * 1024 * 1024;
    let inflated_le = inflated.to_le_bytes();

    // Patch local file header for media/a.txt (PK\x03\x04, uncompressed size at +22..+26).
    let mut patched_local = 0usize;
    for i in 0..bytes.len().saturating_sub(30) {
        if &bytes[i..i + 4] == b"PK\x03\x04" {
            let name_len = u16::from_le_bytes([bytes[i + 26], bytes[i + 27]]) as usize;
            if bytes.get(i + 30..i + 30 + name_len) == Some(b"media/a.txt") {
                bytes[i + 22..i + 26].copy_from_slice(&inflated_le);
                patched_local += 1;
            }
        }
    }
    // Patch central directory header for media/a.txt (PK\x01\x02, uncompressed size at +24..+28).
    let mut patched_cd = 0usize;
    for i in 0..bytes.len().saturating_sub(46) {
        if &bytes[i..i + 4] == b"PK\x01\x02" {
            let name_len = u16::from_le_bytes([bytes[i + 28], bytes[i + 29]]) as usize;
            if bytes.get(i + 46..i + 46 + name_len) == Some(b"media/a.txt") {
                bytes[i + 24..i + 28].copy_from_slice(&inflated_le);
                patched_cd += 1;
            }
        }
    }
    assert_eq!(patched_local, 1);
    assert_eq!(patched_cd, 1);

    let limits = Limits {
        max_entries: 10,
        max_entry_bytes: 256 * 1024 * 1024,
        max_total_bytes: 100 * 1024 * 1024,
    };
    let err = open_project_with_limits(&bytes, limits).expect_err("inflated zip should fail");
    assert_eq!(err, FormatError::TooLarge);
}

#[test]
fn safe_relative_path_rejects_quotes_and_traversal() {
    for p in [
        "../secret.png",
        "C:\\Users\\a.png",
        "\"a.png\"",
        "\0bad.png",
        "https://evil/a.png",
    ] {
        assert!(!is_safe_relative_path(p), "should reject {p:?}");
    }
}

#[test]
fn xliff_round_trip_keeps_state_text_and_runs_synced() {
    let course: Course = serde_json::from_value(serde_json::json!({
        "schema_version": 1,
        "id": "xliff-state-runs",
        "title": "XLIFF State Runs",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "scene-1",
            "title": "Scene 1",
            "slides": [{
                "id": "s1",
                "title": "Slide 1",
                "objects": [{
                    "id": "btn-1",
                    "name": "Button 1",
                    "frame": { "x": 100, "y": 100, "w": 200, "h": 60 },
                    "kind": { "type": "button", "label": "Next" },
                    "states": [{
                        "name": "hover",
                        "text": "Go forward",
                        "runs": [
                            { "text": "Go ", "bold": true },
                            { "text": "forward", "italic": true }
                        ]
                    }]
                }]
            }]
        }]
    }))
    .expect("valid course");

    let doc = crate::xliff::export_xliff(&course, "fr").expect("export xliff");
    assert!(
        doc.contains("<g id=\"r0\">Go </g>") && doc.contains("<g id=\"r1\">forward</g>"),
        "expected <g> run tags for state.runs in xliff:\n{doc}"
    );

    let translated = doc.replace(
        "<g id=\"r0\">Go </g>\n          <g id=\"r1\">forward</g>\n        </source>",
        "<g id=\"r0\">Go </g>\n          <g id=\"r1\">forward</g>\n        </source><target><g id=\"r0\">Aller </g><g id=\"r1\">avant</g></target>",
    );
    let parsed = crate::xliff::parse_xliff(translated.as_bytes()).expect("parse xliff");
    let (course, report) = crate::xliff::apply_xliff(&course, &parsed);
    assert!(report.applied >= 1, "expected applied unit: {report:?}");

    let st = &course.scenes[0].slides[0].objects[0].states[0];
    assert_eq!(st.text.as_deref(), Some("Aller avant"));
    let runs = st.runs.as_ref().expect("runs synced");
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0].text, "Aller ");
    assert_eq!(runs[0].bold, Some(true));
    assert_eq!(runs[1].text, "avant");
    assert_eq!(runs[1].italic, Some(true));
}

#[test]
fn xliff_round_trip_covers_state_extra_text_runs_and_alt() {
    let course: Course = serde_json::from_value(serde_json::json!({
        "schema_version": 1,
        "id": "xliff-state-extras",
        "title": "XLIFF State Extras",
        "slide_size": { "w": 1280, "h": 720 },
        "scenes": [{
            "id": "scene-1",
            "title": "Scene 1",
            "slides": [{
                "id": "s1",
                "title": "Slide 1",
                "objects": [{
                    "id": "btn-1",
                    "name": "Button 1",
                    "frame": { "x": 100, "y": 100, "w": 200, "h": 60 },
                    "kind": { "type": "button", "label": "Next" },
                    "states": [{
                        "name": "hover",
                        "extras": [
                            {
                                "id": "ex-txt",
                                "kind": {
                                    "type": "text",
                                    "text": "Go forward",
                                    "runs": [
                                        { "text": "Go ", "bold": true },
                                        { "text": "forward", "italic": true }
                                    ]
                                },
                                "frame": { "x": 0.0, "y": 0.0, "w": 80.0, "h": 24.0 }
                            },
                            {
                                "id": "ex-shp",
                                "kind": {
                                    "type": "shape",
                                    "shape": "rectangle",
                                    "fill": "#112233",
                                    "text": { "text": "Badge" }
                                },
                                "frame": { "x": 0.0, "y": 28.0, "w": 80.0, "h": 24.0 }
                            },
                            {
                                "id": "ex-img",
                                "kind": { "type": "image", "src": "icon.png", "alt": "Arrow" },
                                "frame": { "x": 84.0, "y": 0.0, "w": 24.0, "h": 24.0 }
                            }
                        ]
                    }]
                }]
            }]
        }]
    }))
    .expect("valid course");

    let doc = crate::xliff::export_xliff(&course, "fr").expect("export xliff");
    assert!(
        doc.contains("state/hover/extra/ex-txt/text")
            && doc.contains("state/hover/extra/ex-shp/text")
            && doc.contains("state/hover/extra/ex-img/alt"),
        "expected state extra units in xliff:\n{doc}"
    );
    assert!(
        doc.contains("<g id=\"r0\">Go </g>") && doc.contains("<g id=\"r1\">forward</g>"),
        "expected <g> run tags for state extra text in xliff:\n{doc}"
    );

    let translated = doc
        .replace(
            "<g id=\"r0\">Go </g>\n          <g id=\"r1\">forward</g>\n        </source>",
            "<g id=\"r0\">Go </g>\n          <g id=\"r1\">forward</g>\n        </source><target><g id=\"r0\">Aller </g><g id=\"r1\">avant</g></target>",
        )
        .replace(
            "<source>Badge</source>",
            "<source>Badge</source><target>Insigne</target>",
        )
        .replace(
            "<source>Arrow</source>",
            "<source>Arrow</source><target>Flèche</target>",
        );

    let parsed = crate::xliff::parse_xliff(translated.as_bytes()).expect("parse xliff");
    let (course, report) = crate::xliff::apply_xliff(&course, &parsed);
    assert_eq!(report.applied, 3, "expected 3 applied units: {report:?}");

    let extras = &course.scenes[0].slides[0].objects[0].states[0].extras;
    let coursecraft_course::StateExtraKind::Text { text, runs, .. } = &extras[0].kind else {
        panic!("expected text extra");
    };
    assert_eq!(text, "Aller avant");
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0].text, "Aller ");
    assert_eq!(runs[0].bold, Some(true));
    assert_eq!(runs[1].text, "avant");
    assert_eq!(runs[1].italic, Some(true));

    let coursecraft_course::StateExtraKind::Shape {
        text: Some(shp_text),
        ..
    } = &extras[1].kind
    else {
        panic!("expected shape extra");
    };
    assert_eq!(shp_text.text, "Insigne");

    let coursecraft_course::StateExtraKind::Image { alt, .. } = &extras[2].kind else {
        panic!("expected image extra");
    };
    assert_eq!(alt, "Flèche");
}

/// U-09: `coursecraft_format::is_safe_relative_path` agrees with
/// `coursecraft_course::is_safe_media_path` on every safe and hostile path.
#[test]
fn u09_is_safe_relative_path_matches_course_is_safe_media_path() {
    let cases = [
        ("", false),
        ("/leading/slash.png", false),
        ("/", false),
        ("..", false),
        ("../escape.png", false),
        ("a/../b.png", false),
        ("a/..", false),
        (".", false),
        ("./a.png", false),
        ("a/./b.png", false),
        ("a/.", false),
        ("a\\b.png", false),
        ("C:\\Users\\a.png", false),
        ("C:file.png", false),
        ("https://example.com/a.png", false),
        ("\"quoted.png\"", false),
        ("a\"b.png", false),
        ("a\0b.png", false),
        ("a\nb.png", false),
        ("a\tb.png", false),
        ("a\rb.png", false),
        ("a\x1fb.png", false),
        ("a\x7fb.png", false),
        ("a//b.png", false),
        ("a/b/", false),
        ("pic.png", true),
        ("media/pic.png", true),
        ("assets/icons/star.svg", true),
        ("nested/dir/clip-01_final.mp4", true),
    ];

    for (path, expected) in cases {
        let fmt_ok = is_safe_relative_path(path);
        let course_ok = coursecraft_course::is_safe_media_path(path);
        assert_eq!(
            fmt_ok, course_ok,
            "is_safe_relative_path and is_safe_media_path disagreed on {path:?}"
        );
        assert_eq!(fmt_ok, expected, "unexpected result for {path:?}");
    }
}

/// U-06: streaming `open_project_from` / `open_project_from_with_limits` matches
/// `open_project` / `open_project_with_limits` on valid archives and rejects
/// oversized, zip-bomb, duplicate, and unsafe-path archives identically.
#[test]
fn u06_open_project_from_matches_open_project_and_rejects_hostile_zips() {
    // 1. Valid archive matches byte-for-byte via BufReader over Cursor.
    let course = Course::from_json(HELLO_COURSE_JSON).expect("valid hello-course");
    let media = vec![
        (
            "images/badge.png".to_string(),
            vec![0x89, b'P', b'N', b'G', 1, 2, 3],
        ),
        (
            "captions/intro.vtt".to_string(),
            b"WEBVTT\n\n00:00.000 --> 00:01.000\nHi\n".to_vec(),
        ),
    ];
    let saved = save_project(&course, &media).expect("save_project");
    let from_slice = open_project(&saved).expect("open_project");
    let from_reader = open_project_from(std::io::BufReader::with_capacity(
        256 * 1024,
        std::io::Cursor::new(&saved),
    ))
    .expect("open_project_from");
    assert_eq!(from_reader, from_slice);

    // 2. Oversized single entry, oversized total archive, and too many entries.
    let big_media = vec![
        ("m1.png".to_string(), vec![1u8; 4000]),
        ("m2.png".to_string(), vec![2u8; 4000]),
        ("m3.png".to_string(), vec![3u8; 4000]),
    ];
    let big_zip = save_project(&minimal_course("limits"), &big_media).expect("save big");
    for limits in [
        Limits {
            max_entry_bytes: 2000,
            ..Limits::default()
        },
        Limits {
            max_entry_bytes: 5000,
            max_total_bytes: 5000,
            ..Limits::default()
        },
        Limits {
            max_entries: 2,
            ..Limits::default()
        },
    ] {
        let err_slice = open_project_with_limits(&big_zip, limits).expect_err("slice rejects");
        let err_reader = open_project_from_with_limits(
            std::io::BufReader::new(std::io::Cursor::new(&big_zip)),
            limits,
        )
        .expect_err("reader rejects");
        assert_eq!(err_reader, err_slice);
    }

    // 3. Zip-bomb (inflated declared size).
    let mut bomb_buf = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut bomb_buf);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        let course_json = minimal_course("bomb").to_json().expect("json");
        writer
            .start_file("course.json", opts)
            .expect("start course");
        writer
            .write_all(course_json.as_bytes())
            .expect("write course");
        writer.start_file("media/a.txt", opts).expect("start media");
        writer.write_all(b"tiny").expect("write media");
        writer.finish().expect("finish");
    }
    let mut bomb = bomb_buf.into_inner();
    let inflated_le = (150u32 * 1024 * 1024).to_le_bytes();
    for i in 0..bomb.len().saturating_sub(30) {
        if &bomb[i..i + 4] == b"PK\x03\x04" {
            let name_len = u16::from_le_bytes([bomb[i + 26], bomb[i + 27]]) as usize;
            if bomb.get(i + 30..i + 30 + name_len) == Some(b"media/a.txt") {
                bomb[i + 22..i + 26].copy_from_slice(&inflated_le);
            }
        }
    }
    for i in 0..bomb.len().saturating_sub(46) {
        if &bomb[i..i + 4] == b"PK\x01\x02" {
            let name_len = u16::from_le_bytes([bomb[i + 28], bomb[i + 29]]) as usize;
            if bomb.get(i + 46..i + 46 + name_len) == Some(b"media/a.txt") {
                bomb[i + 24..i + 28].copy_from_slice(&inflated_le);
            }
        }
    }
    let bomb_limits = Limits {
        max_entries: 10,
        max_entry_bytes: 256 * 1024 * 1024,
        max_total_bytes: 100 * 1024 * 1024,
    };
    assert_eq!(
        open_project_from_with_limits(std::io::Cursor::new(&bomb), bomb_limits),
        open_project_with_limits(&bomb, bomb_limits)
    );
    assert_eq!(
        open_project_from_with_limits(std::io::Cursor::new(&bomb), bomb_limits),
        Err(FormatError::TooLarge)
    );

    // 4. Duplicate entry (patched central directory + local header).
    let mut dup_buf = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut dup_buf);
        let opts = zip::write::SimpleFileOptions::default();
        let course_json = minimal_course("dup").to_json().expect("json");
        writer
            .start_file("course.json", opts)
            .expect("start course");
        writer
            .write_all(course_json.as_bytes())
            .expect("write course");
        writer.start_file("media/a.png", opts).expect("start a");
        writer.write_all(b"one").expect("write a");
        writer.start_file("media/b.png", opts).expect("start b");
        writer.write_all(b"two").expect("write b");
        writer.finish().expect("finish");
    }
    let mut dup = dup_buf.into_inner();
    for i in 0..dup.len().saturating_sub(11) {
        if &dup[i..i + 11] == b"media/b.png" {
            dup[i..i + 11].copy_from_slice(b"media/a.png");
        }
    }
    let dup_slice = open_project(&dup).expect_err("slice rejects duplicate");
    let dup_reader = open_project_from(std::io::BufReader::new(std::io::Cursor::new(&dup)))
        .expect_err("reader rejects duplicate");
    assert_eq!(dup_reader, dup_slice);
    assert!(
        matches!(dup_reader, FormatError::DuplicateEntry(_)),
        "expected DuplicateEntry, got {dup_reader:?}"
    );

    // 5. Unsafe path entry.
    let mut evil_buf = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut evil_buf);
        let opts = zip::write::SimpleFileOptions::default();
        let course_json = minimal_course("evil").to_json().expect("json");
        writer
            .start_file("course.json", opts)
            .expect("start course");
        writer
            .write_all(course_json.as_bytes())
            .expect("write course");
        writer
            .start_file("../escape.png", opts)
            .expect("start evil");
        writer.write_all(b"bad").expect("write evil");
        writer.finish().expect("finish");
    }
    let evil = evil_buf.into_inner();
    assert_eq!(
        open_project_from(std::io::BufReader::new(std::io::Cursor::new(&evil))),
        open_project(&evil)
    );
    assert_eq!(
        open_project_from(std::io::Cursor::new(&evil)),
        Err(FormatError::UnsafePath("../escape.png".to_string()))
    );
}

#[test]
fn m37_golden_ccraft_roundtrip() {
    fn fnv1a64(bytes: &[u8]) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for &b in bytes {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        h
    }

    fn extract_course_json(archive_bytes: &[u8]) -> Vec<u8> {
        let mut archive =
            zip::ZipArchive::new(std::io::Cursor::new(archive_bytes)).expect("open zip");
        let mut entry = archive.by_name("course.json").expect("course.json entry");
        let mut out = Vec::new();
        entry.read_to_end(&mut out).expect("read course.json");
        out
    }

    const SHOWCASE_COURSE_JSON: &str = include_str!("../../../examples/showcase/course.json");
    let expected: [(&str, usize, u64); 2] = [
        ("hello-course", 19_661, 0x0376_c459_8af0_a02c),
        ("showcase", 92_523, 0xf72d_eff4_ae12_2639),
    ];

    let mut actual = Vec::new();
    for (name, raw) in [
        ("hello-course", HELLO_COURSE_JSON),
        ("showcase", SHOWCASE_COURSE_JSON),
    ] {
        let course = Course::from_json(raw).expect("parse example course");
        let empty_media: Vec<(String, Vec<u8>)> = Vec::new();
        let archive1 = save_project(&course, &empty_media).expect("save first ccraft");
        let opened = open_project(&archive1).expect("open first ccraft");
        assert_eq!(opened.course, course, "{name}: reopened course must match");

        let archive2 = save_project(&opened.course, &opened.media).expect("save second ccraft");
        let json1 = extract_course_json(&archive1);
        let json2 = extract_course_json(&archive2);
        assert_eq!(
            json1, json2,
            "{name}: course.json inside .ccraft must be byte-identical after reload"
        );
        assert_eq!(
            archive1, archive2,
            "{name}: full .ccraft archive must be byte-identical after reload"
        );

        let text = std::str::from_utf8(&json1).expect("utf8 course.json");
        assert!(
            !text.contains("\"point_text\""),
            "{name}: .ccraft course.json must not emit point_text"
        );
        assert!(
            !text.contains("\"groups\""),
            "{name}: .ccraft course.json must not emit groups"
        );

        actual.push((name, json1.len(), fnv1a64(&json1)));
    }

    assert_eq!(
        actual.as_slice(),
        &expected,
        ".ccraft course.json changed: {actual:#x?}"
    );
}
