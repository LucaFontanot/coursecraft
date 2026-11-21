//! M26 Step f19: imported per-run fonts and WYSIWYG inline text editing.

use std::sync::Arc;

use coursecraft_course::{
    Align, Baseline, Color, Course, FontFace, FontStyle, Object, ObjectKind, Paragraph, Rect,
    TextInsets, TextRun, TextStyle, VAlign,
};
use coursecraft_logic::Player;
use coursecraft_render::{MediaFetch, MediaSource, PlayerFrame};
use egui::epaint::{ClippedShape, Shape};
use egui::{Key, Modifiers, pos2};

use crate::test_harness::TestHarness;

fn hello() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    Course::from_json(&json).expect("valid course")
}

fn base_object(id: &str, frame: Rect, kind: ObjectKind) -> Object {
    let mut o = hello().scenes[0].slides[0].objects[0].clone();
    o.id = id.into();
    o.frame = frame;
    o.kind = kind;
    o.states.clear();
    o
}

fn find_galley_containing_in<'a>(
    shapes: &'a [ClippedShape],
    region: egui::Rect,
    needle: &str,
) -> Option<&'a egui::epaint::TextShape> {
    fn walk<'a>(
        shape: &'a Shape,
        region: egui::Rect,
        needle: &str,
    ) -> Option<&'a egui::epaint::TextShape> {
        match shape {
            Shape::Text(t)
                if t.galley.text().contains(needle)
                    && region.contains(t.galley.rect.translate(t.pos.to_vec2()).center()) =>
            {
                Some(t)
            }
            Shape::Vec(v) => v.iter().find_map(|s| walk(s, region, needle)),
            _ => None,
        }
    }
    shapes.iter().find_map(|cs| walk(&cs.shape, region, needle))
}

fn run(text: &str, font: Option<&str>) -> TextRun {
    TextRun {
        text: text.into(),
        font: font.map(str::to_owned),
        bold: None,
        italic: None,
        underline: None,
        strikethrough: None,
        baseline: None,
        letter_spacing: None,
        size: None,
        color: None,
        link: None,
    }
}

fn install_temp_font_provider(
    tag: &str,
) -> (
    std::path::PathBuf,
    Arc<dyn coursecraft_render::SystemFontProvider>,
) {
    let dir = std::env::temp_dir().join(format!("coursecraft-f19-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create temp font dir");

    let dm_serif =
        include_bytes!("../../../examples/healthy-habits/media/fonts/dm-serif-display-regular.ttf");
    let phosphor = include_bytes!("../../render/fonts/Phosphor.ttf");
    std::fs::write(dir.join("dm-serif.ttf"), dm_serif).expect("write dm serif");
    std::fs::write(dir.join("phosphor.ttf"), phosphor).expect("write phosphor");

    let index = Arc::new(
        crate::coursecraft_media::system_fonts::SystemFontIndex::from_folders(
            std::slice::from_ref(&dir),
        ),
    );
    let provider: Arc<dyn coursecraft_render::SystemFontProvider> =
        Arc::new(coursecraft_render::IndexFontProvider(index));
    (dir, provider)
}

#[test]
fn f19_1_multi_run_installed_fonts_with_delayed_provider() {
    let (dir, provider) = install_temp_font_provider("delayed");

    let mut course = hello();
    let slide = &mut course.scenes[0].slides[0];
    slide.objects.clear();
    slide.triggers.clear();
    slide.objects.push(base_object(
        "txt-multi",
        Rect {
            x: 60.0,
            y: 60.0,
            w: 700.0,
            h: 140.0,
        },
        ObjectKind::Text {
            text: "ALPHA BETA GAMMA".into(),
            style: TextStyle {
                font: "Inter".into(),
                size: 28.0,
                color: Color("#000000".into()),
                ..Default::default()
            },
            runs: vec![
                run("ALPHA ", Some("DM Serif Display")),
                run("BETA ", Some("Phosphor")),
                run("GAMMA", Some("Inter")),
            ],
            paragraphs: Vec::new(),
            autofit: Default::default(),
            text_style: Default::default(),
            outline: None,
        },
    ));

    let mut harness = TestHarness::new(course);
    // Simulate real app: first frames run BEFORE background font scan completes.
    harness.idle_frame();
    harness.idle_frame();

    harness.editor.set_system_font_provider(Some(provider));

    harness.idle_frame();
    let shapes = harness.frame_shapes(vec![]);

    let central = harness.central_panel_rect();
    let text_shape =
        find_galley_containing_in(&shapes, central, "ALPHA BETA GAMMA").expect("text shape");
    let first_section_family = &text_shape.galley.job.sections[0].format.font_id.family;
    let expected = coursecraft_render::system_face_family("DM Serif Display", FontStyle::Regular);
    assert_eq!(
        first_section_family, &expected,
        "run 0 should use DM Serif Display system face, got {first_section_family:?}"
    );
    let defs = harness.ctx.fonts(|f| f.definitions().clone());
    assert!(
        defs.families.contains_key(&expected),
        "DM Serif Display family must actually be installed in egui FontDefinitions, not dropped"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn f19_1b_provider_before_reopen_case_whitespace_bold_italic_thumbnail_preview() {
    let (dir, provider) = install_temp_font_provider("variants");

    let mut course = hello();
    let slide = &mut course.scenes[0].slides[0];
    slide.objects.clear();
    slide.triggers.clear();
    let mut bold_run = run("BOLD ", Some("  dm serif display  "));
    bold_run.bold = Some(true);
    let mut italic_run = run("ITALIC", Some("DM SERIF DISPLAY"));
    italic_run.italic = Some(true);
    slide.objects.push(base_object(
        "txt-variants",
        Rect {
            x: 60.0,
            y: 60.0,
            w: 700.0,
            h: 140.0,
        },
        ObjectKind::Text {
            text: "BOLD ITALIC".into(),
            style: TextStyle {
                font: "Inter".into(),
                size: 28.0,
                color: Color("#000000".into()),
                ..Default::default()
            },
            runs: vec![bold_run, italic_run],
            paragraphs: Vec::new(),
            autofit: Default::default(),
            text_style: Default::default(),
            outline: None,
        },
    ));

    // Provider set BEFORE first layout.
    let mut harness = TestHarness::new(course.clone());
    harness.editor.set_system_font_provider(Some(provider));
    harness.idle_frame();
    let shapes = harness.frame_shapes(vec![]);

    let expected = coursecraft_render::system_face_family("DM Serif Display", FontStyle::Regular);

    // Canvas
    let central = harness.central_panel_rect();
    let canvas_shape =
        find_galley_containing_in(&shapes, central, "BOLD ITALIC").expect("canvas shape");
    assert_eq!(
        &canvas_shape.galley.job.sections[0].format.font_id.family,
        &expected
    );
    assert_eq!(
        canvas_shape
            .galley
            .job
            .sections
            .last()
            .unwrap()
            .format
            .font_id
            .family,
        expected
    );

    // Slide thumbnail (left panel)
    let left_panel = harness.left_panel_rect();
    let thumb_shape =
        find_galley_containing_in(&shapes, left_panel, "BOLD ITALIC").expect("thumbnail shape");
    assert_eq!(
        &thumb_shape.galley.job.sections[0].format.font_id.family,
        &expected
    );

    // Course reopened
    harness.editor.replace_course(course);
    harness.idle_frame();
    let reopened_shapes = harness.frame_shapes(vec![]);
    let reopened_shape = find_galley_containing_in(&reopened_shapes, central, "BOLD ITALIC")
        .expect("reopened canvas shape");
    assert_eq!(
        &reopened_shape.galley.job.sections[0].format.font_id.family,
        &expected
    );

    // Preview window
    harness.editor.open_preview();
    for _ in 0..4 {
        harness.idle_frame();
    }
    let preview_rect = harness
        .editor
        .preview
        .as_ref()
        .expect("preview open")
        .window_rect
        .expect("preview window rect");
    let preview_shapes = harness.frame_shapes(vec![]);
    let preview_shape = preview_shapes
        .iter()
        .rev()
        .find_map(|cs| {
            fn walk<'a>(
                shape: &'a Shape,
                region: egui::Rect,
                needle: &str,
            ) -> Option<&'a egui::epaint::TextShape> {
                match shape {
                    Shape::Text(t)
                        if t.galley.text().contains(needle)
                            && region
                                .contains(t.galley.rect.translate(t.pos.to_vec2()).center()) =>
                    {
                        Some(t)
                    }
                    Shape::Vec(v) => v.iter().rev().find_map(|s| walk(s, region, needle)),
                    _ => None,
                }
            }
            walk(&cs.shape, preview_rect, "BOLD ITALIC")
        })
        .expect("preview shape");
    assert_eq!(
        &preview_shape.galley.job.sections[0].format.font_id.family,
        &expected
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn f19_1c_published_player_path_with_embedded_fonts() {
    struct FontMedia(Arc<[u8]>);
    impl MediaSource for FontMedia {
        fn get(&self, src: &str) -> MediaFetch {
            if src == "fonts/dm.ttf" {
                MediaFetch::Ready(Arc::clone(&self.0))
            } else {
                MediaFetch::Missing
            }
        }
    }

    let dm_serif: Arc<[u8]> = Arc::from(
        &include_bytes!(
            "../../../examples/healthy-habits/media/fonts/dm-serif-display-regular.ttf"
        )[..],
    );
    let media = FontMedia(dm_serif);

    let mut course = hello();
    course.fonts.push(FontFace {
        family: "DM Serif Display".into(),
        style: FontStyle::Regular,
        src: "fonts/dm.ttf".into(),
    });
    let slide = &mut course.scenes[0].slides[0];
    slide.objects.clear();
    slide.triggers.clear();
    slide.objects.push(base_object(
        "txt-player",
        Rect {
            x: 80.0,
            y: 80.0,
            w: 600.0,
            h: 120.0,
        },
        ObjectKind::Text {
            text: "PLAYER EMBEDDED".into(),
            style: TextStyle {
                font: "Inter".into(),
                size: 32.0,
                color: Color("#000000".into()),
                ..Default::default()
            },
            runs: vec![
                run("PLAYER ", Some("DM Serif Display")),
                run("EMBEDDED", Some("Inter")),
            ],
            paragraphs: Vec::new(),
            autofit: Default::default(),
            text_style: Default::default(),
            outline: None,
        },
    ));

    let ctx = egui::Context::default();
    coursecraft_render::install_course_fonts(&ctx, &course, &media);
    let player = Player::new(&course).expect("player");
    let mut frame = PlayerFrame::new();
    let mut view = coursecraft_render::SlideView::new();
    let mut cache = coursecraft_render::MediaCache::new();
    let screen = egui::Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(1280.0, 720.0));
    let mut last_shapes = Vec::new();
    for _ in 0..3 {
        let out = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(screen),
                ..Default::default()
            },
            |ui| {
                frame.show(ui, &player, &mut view, Some((&mut cache, &media)));
            },
        );
        last_shapes = out.shapes.clone();
        out.drop_without_applying_deltas();
    }

    let shape = find_galley_containing_in(&last_shapes, screen, "PLAYER EMBEDDED")
        .expect("player text shape");
    let expected = coursecraft_render::course_face_family("DM Serif Display", FontStyle::Regular);
    assert_eq!(
        &shape.galley.job.sections[0].format.font_id.family, &expected,
        "published player must render run 0 in embedded DM Serif Display"
    );
}

#[test]
fn f19_2_inline_edit_preserves_formatting_while_open() {
    let mut course = hello();
    let slide = &mut course.scenes[0].slides[0];
    slide.objects.clear();
    slide.triggers.clear();
    slide.objects.push(base_object(
        "txt-rich",
        Rect {
            x: 100.0,
            y: 100.0,
            w: 500.0,
            h: 150.0,
        },
        ObjectKind::Text {
            text: "SmallBigRed".into(),
            style: TextStyle {
                font: "Inter".into(),
                size: 20.0,
                color: Color("#111111".into()),
                align: Align::Left,
                valign: VAlign::Top,
                ..Default::default()
            },
            runs: vec![
                TextRun {
                    text: "Small".into(),
                    font: None,
                    bold: Some(true),
                    italic: None,
                    underline: Some(true),
                    strikethrough: None,
                    baseline: None,
                    letter_spacing: None,
                    size: Some(14.0),
                    color: Some(Color("#112233".into())),
                    link: None,
                },
                TextRun {
                    text: "Big".into(),
                    font: None,
                    bold: None,
                    italic: Some(true),
                    underline: None,
                    strikethrough: Some(true),
                    baseline: None,
                    letter_spacing: None,
                    size: Some(36.0),
                    color: Some(Color("#cc2200".into())),
                    link: None,
                },
                TextRun {
                    text: "Red".into(),
                    font: None,
                    bold: None,
                    italic: None,
                    underline: None,
                    strikethrough: None,
                    baseline: Some(Baseline::Superscript),
                    letter_spacing: None,
                    size: Some(20.0),
                    color: Some(Color("#0055cc".into())),
                    link: None,
                },
            ],
            paragraphs: Vec::new(),
            autofit: Default::default(),
            text_style: Default::default(),
            outline: None,
        },
    ));

    let mut harness = TestHarness::new(course);
    harness.idle_frame();
    let slide_id = harness.editor.selection.slide.clone();
    let central = harness.central_panel_rect();
    let center = crate::canvas::object_screen_rect(
        &harness.editor.session.course,
        &slide_id,
        "txt-rich",
        central,
    )
    .expect("txt-rich rect")
    .center();
    // Double-click to enter inline edit.
    harness.click_at(center);
    harness.click_at(center);
    harness.idle_frame();
    assert!(
        harness.editor.ui.inline_edit.is_some(),
        "double-click should enter inline edit"
    );

    let shapes = harness.frame_shapes(vec![]);
    let editing_shape = find_galley_containing_in(&shapes, central, "SmallBigRed")
        .expect("inline editor galley should be painted");
    assert!(
        editing_shape.galley.job.sections.len() >= 3,
        "inline editor must preserve per-run sections while open, got {} section(s)",
        editing_shape.galley.job.sections.len()
    );
    let s0 = &editing_shape.galley.job.sections[0].format;
    let s1 = &editing_shape.galley.job.sections[1].format;
    let s2 = &editing_shape.galley.job.sections[2].format;
    assert_ne!(s0.font_id.size, s1.font_id.size, "run sizes preserved");
    assert_ne!(s0.color, s1.color, "run colours preserved");
    assert_ne!(
        s0.underline,
        egui::Stroke::NONE,
        "run 0 underline preserved"
    );
    assert_ne!(
        s1.strikethrough,
        egui::Stroke::NONE,
        "run 1 strikethrough preserved"
    );
    assert_eq!(s2.valign, egui::Align::TOP, "run 2 superscript preserved");
}

#[test]
fn f19_2b_inline_edit_typing_boundary_backspace_undo_redo_and_insets() {
    let mut course = hello();
    let slide = &mut course.scenes[0].slides[0];
    slide.objects.clear();
    slide.triggers.clear();
    slide.objects.push(base_object(
        "txt-edit",
        Rect {
            x: 100.0,
            y: 100.0,
            w: 500.0,
            h: 160.0,
        },
        ObjectKind::Text {
            text: "AlphaBeta".into(),
            style: TextStyle {
                font: "Inter".into(),
                size: 24.0,
                color: Color("#111111".into()),
                align: Align::Left,
                valign: VAlign::Top,
                insets: Some(TextInsets {
                    left: 30.0,
                    right: 10.0,
                    top: 15.0,
                    bottom: 10.0,
                }),
                ..Default::default()
            },
            runs: vec![
                TextRun {
                    text: "Alpha".into(),
                    font: None,
                    bold: Some(true),
                    italic: None,
                    underline: Some(true),
                    strikethrough: None,
                    baseline: None,
                    letter_spacing: None,
                    size: Some(16.0),
                    color: Some(Color("#112233".into())),
                    link: None,
                },
                TextRun {
                    text: "Beta".into(),
                    font: None,
                    bold: None,
                    italic: Some(true),
                    underline: None,
                    strikethrough: Some(true),
                    baseline: None,
                    letter_spacing: None,
                    size: Some(32.0),
                    color: Some(Color("#cc2200".into())),
                    link: None,
                },
            ],
            paragraphs: vec![Paragraph {
                line_spacing: 1.5,
                ..Default::default()
            }],
            autofit: Default::default(),
            text_style: Default::default(),
            outline: None,
        },
    ));

    let mut harness = TestHarness::new(course);
    harness.idle_frame();
    let slide_id = harness.editor.selection.slide.clone();
    let central = harness.central_panel_rect();
    let obj_rect = crate::canvas::object_screen_rect(
        &harness.editor.session.course,
        &slide_id,
        "txt-edit",
        central,
    )
    .expect("txt-edit rect");

    // Double-click to enter inline edit.
    harness.click_at(obj_rect.center());
    harness.click_at(obj_rect.center());
    harness.idle_frame();
    assert!(harness.editor.ui.inline_edit.is_some());

    // Check insets: galley position should be inset from obj_rect.min.
    let shapes = harness.frame_shapes(vec![]);
    let editing = find_galley_containing_in(&shapes, central, "AlphaBeta").expect("galley");
    assert!(
        editing.pos.x > obj_rect.min.x + 10.0,
        "left inset should shift inline editor text right (pos.x={}, rect.min.x={})",
        editing.pos.x,
        obj_rect.min.x
    );

    // Move to start, collapse selection, move to index 5 (boundary after "Alpha"), type "!"
    harness.press(Key::Home, Modifiers::NONE);
    for _ in 0..5 {
        harness.press(Key::ArrowRight, Modifiers::NONE);
    }
    harness.type_text("!");
    harness.idle_frame();

    // Verify "!" took "Alpha"'s formatting (run before boundary, like PowerPoint).
    let live_runs = &harness
        .editor
        .ui
        .inline_edit
        .as_ref()
        .expect("still editing")
        .live_runs;
    assert_eq!(live_runs.len(), 2);
    assert_eq!(live_runs[0].text, "Alpha!");
    assert_eq!(live_runs[1].text, "Beta");

    // Select across the boundary ("!B") and press Backspace.
    harness.press(Key::ArrowLeft, Modifiers::SHIFT);
    harness.press(Key::ArrowRight, Modifiers::NONE);
    // Caret is after "!", select 1 char left and 1 char right? Simpler: move to after "B" (right 1),
    // select 2 chars left ("!B"), press Backspace.
    harness.press(Key::ArrowRight, Modifiers::NONE);
    harness.press(Key::ArrowLeft, Modifiers::SHIFT);
    harness.press(Key::ArrowLeft, Modifiers::SHIFT);
    harness.press(Key::Backspace, Modifiers::NONE);
    harness.idle_frame();

    // Commit with Command+Enter.
    harness.press(Key::Enter, Modifiers::COMMAND);
    harness.idle_frame();
    assert!(harness.editor.ui.inline_edit.is_none());

    let saved_runs = match &harness
        .editor
        .session
        .course
        .slide(&slide_id)
        .unwrap()
        .object("txt-edit")
        .unwrap()
        .0
        .kind
    {
        ObjectKind::Text { runs, .. } => runs.clone(),
        other => panic!("expected text, got {other:?}"),
    };
    assert_eq!(saved_runs.len(), 2, "runs must not be flattened on commit");
    assert_eq!(saved_runs[0].text, "Alpha");
    assert_eq!(saved_runs[0].bold, Some(true));
    assert_eq!(saved_runs[0].size, Some(16.0));
    assert_eq!(saved_runs[1].text, "eta");
    assert_eq!(saved_runs[1].italic, Some(true));
    assert_eq!(saved_runs[1].size, Some(32.0));

    // Undo and redo keep runs.
    harness.editor.session.undo().expect("undo");
    harness.idle_frame();
    let undone_runs = match &harness
        .editor
        .session
        .course
        .slide(&slide_id)
        .unwrap()
        .object("txt-edit")
        .unwrap()
        .0
        .kind
    {
        ObjectKind::Text { runs, .. } => runs.clone(),
        other => panic!("expected text, got {other:?}"),
    };
    assert_eq!(undone_runs.len(), 2);
    assert_eq!(undone_runs[0].text, "Alpha");
    assert_eq!(undone_runs[1].text, "Beta");

    harness.editor.session.redo().expect("redo");
    harness.idle_frame();
    let redone_runs = match &harness
        .editor
        .session
        .course
        .slide(&slide_id)
        .unwrap()
        .object("txt-edit")
        .unwrap()
        .0
        .kind
    {
        ObjectKind::Text { runs, .. } => runs.clone(),
        other => panic!("expected text, got {other:?}"),
    };
    assert_eq!(redone_runs.len(), 2);
    assert_eq!(redone_runs[1].text, "eta");
}
