//! M27 Step D-4 (D5.1): Exhaustive undo/redo round-trip sweep across every
//! `mutates == true` command in `REGISTRY`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashSet;

use coursecraft_course::{
    Action, BackgroundAudio, BankDraw, Choice, Color, Course, Cue, DragItem, Event, FontFace,
    FontStyle, GlossaryEntry, Guide, HotspotRegion, HotspotShape, Layer, MatchPair, MotionPath,
    NamedTextStyle, Object, ObjectKind, Outline, PaletteColor, Path, PathPoint, Point, Question,
    QuestionBank, QuestionKind, Rect, Resource, ResourceTarget, Scene, Scoring, Severity, Slide,
    SlideCue, StateDef, TableBorders, TableCell, TableRow, TextStyle, Timing, Trigger, Value,
    VariableDef,
};
use serde_json::{Value as Json, json};

use crate::{Session, registry};

const TRANSLATION_XLIFF: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<xliff version="1.2" xmlns="urn:oasis:names:tc:xliff:document:1.2">
  <file original="course.json" datatype="plaintext" source-language="en" target-language="fr">
    <body>
      <trans-unit id="course/title" xml:space="preserve"><source>M27 Sweep</source><target>Balayage M27</target></trans-unit>
      <trans-unit id="slide/slide1/title" xml:space="preserve"><source>Slide 1</source><target>Diapositive 1</target></trans-unit>
    </body>
  </file>
</xliff>"#;

const CSV_QUESTIONS: &str = include_str!("../../../examples/questions.csv");

fn base_object(id: &str, kind: ObjectKind, frame: Rect) -> Object {
    Object {
        id: id.into(),
        name: id.into(),
        kind,
        frame,
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

fn make_text_obj(id: &str, text: &str, x: f32, y: f32) -> Object {
    base_object(
        id,
        ObjectKind::Text {
            text: text.into(),
            style: TextStyle::default(),
            runs: Vec::new(),
            paragraphs: Vec::new(),
            autofit: Default::default(),
            text_style: None,
            outline: None,
        },
        Rect {
            x,
            y,
            w: 200.0,
            h: 50.0,
        },
    )
}

fn make_shape_obj(id: &str, x: f32, y: f32, w: f32, h: f32, fill: &str) -> Object {
    base_object(
        id,
        ObjectKind::Shape {
            shape: coursecraft_course::ShapeKind::Rectangle,
            fill: Color(fill.into()),
            fill_style: None,
            outline: None,
            text: None,
            adjust: Vec::new(),
        },
        Rect { x, y, w, h },
    )
}

fn make_empty_slide(id: &str, title: &str) -> Slide {
    Slide {
        id: id.into(),
        title: title.into(),
        duration_ms: 5000,
        background: None,
        objects: Vec::new(),
        layers: Vec::new(),
        triggers: Vec::new(),
        cue_points: Vec::new(),
        transition: None,
        question: None,
        draw: None,
        notes: String::new(),
        master: None,
        revisit: coursecraft_course::RevisitMode::Auto,
        player: None,
    }
}

fn make_empty_question(kind: QuestionKind, prompt: &str) -> Question {
    Question {
        kind,
        prompt: prompt.into(),
        choices: Vec::new(),
        points: 10,
        scoring: Scoring::default(),
        attempts: 1,
        shuffle: false,
        correct_layer: None,
        incorrect_layer: None,
        try_again_layer: None,
        accepted: Vec::new(),
        case_sensitive: false,
        pairs: Vec::new(),
        hotspot_image: None,
        regions: Vec::new(),
        markers: 1,
        drag_items: Vec::new(),
        drop_targets: Vec::new(),
        snap_to_center: true,
        return_if_wrong: false,
        one_per_target: true,
        correct_when: None,
    }
}

fn make_table_obj(id: &str) -> Object {
    let cell = |t: &str| TableCell {
        text: t.into(),
        style: TextStyle::default(),
        runs: Vec::new(),
        paragraphs: Vec::new(),
        fill: None,
        valign: Default::default(),
        col_span: 1,
        row_span: 1,
    };
    let mut r0c0 = cell("H0");
    r0c0.col_span = 2;
    let r0c1 = cell("");
    base_object(
        id,
        ObjectKind::Table {
            columns: vec![1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0],
            rows: vec![
                TableRow {
                    height: 1.0 / 3.0,
                    cells: vec![r0c0, r0c1, cell("H2")],
                },
                TableRow {
                    height: 1.0 / 3.0,
                    cells: vec![cell("A1"), cell("B1"), cell("C1")],
                },
                TableRow {
                    height: 1.0 / 3.0,
                    cells: vec![cell("A2"), cell("B2"), cell("C2")],
                },
            ],
            borders: TableBorders::default(),
            header_row: false,
        },
        Rect {
            x: 50.0,
            y: 300.0,
            w: 300.0,
            h: 120.0,
        },
    )
}

/// Builds a valid fixture course containing every object kind, layer, master,
/// template, bank, draw, question kind, table, path, video cue, variable,
/// resource, glossary, font, text style, palette, and guide.
fn fixture_session() -> Session {
    let mut s = Session::blank("M27 Sweep");
    let c = &mut s.course;

    c.language = Some("en".into());
    c.background_audio = Some(BackgroundAudio {
        src: "audio/bg.mp3".into(),
        volume: 0.5,
        looping: true,
    });
    c.theme.palette = vec![
        PaletteColor {
            id: "pal1".into(),
            name: "Primary".into(),
            color: Color("#112233".into()),
        },
        PaletteColor {
            id: "pal2".into(),
            name: "Secondary".into(),
            color: Color("#445566".into()),
        },
    ];
    c.text_styles = vec![NamedTextStyle {
        id: "tstyle1".into(),
        name: "Heading".into(),
        style: TextStyle {
            size: 24.0,
            bold: true,
            color: Color("#112233".into()),
            ..TextStyle::default()
        },
        paragraphs: None,
    }];
    c.fonts = vec![FontFace {
        src: "fonts/custom.ttf".into(),
        family: "CustomFont".into(),
        style: FontStyle::Regular,
    }];
    c.variables = vec![
        VariableDef {
            name: "var1".into(),
            value: Value::Number(1.0),
        },
        VariableDef {
            name: "var_text".into(),
            value: Value::Text(String::new()),
        },
        VariableDef {
            name: "var_unused".into(),
            value: Value::Bool(false),
        },
    ];
    c.resources = vec![
        Resource {
            id: "res1".into(),
            title: "Guide".into(),
            target: ResourceTarget::Url {
                url: "https://example.com/1".into(),
            },
        },
        Resource {
            id: "res2".into(),
            title: "Ref".into(),
            target: ResourceTarget::Url {
                url: "https://example.com/2".into(),
            },
        },
    ];
    c.glossary = vec![
        GlossaryEntry {
            id: "gloss1".into(),
            term: "SCORM".into(),
            definition: "Standard".into(),
        },
        GlossaryEntry {
            id: "gloss2".into(),
            term: "xAPI".into(),
            definition: "Experience API".into(),
        },
    ];
    c.editor.guides = vec![
        Guide {
            vertical: true,
            at: 100.0,
        },
        Guide {
            vertical: false,
            at: 200.0,
        },
    ];
    c.masters = vec![make_empty_slide("master1", "Master 1")];
    c.templates = vec![make_empty_slide("tpl1", "Template 1")];

    let mut bank_q = make_empty_slide("bank_q1", "Bank Question 1");
    let mut bq = make_empty_question(QuestionKind::TrueFalse, "Is Rust safe?");
    bq.choices = vec![
        Choice {
            id: "t".into(),
            text: "True".into(),
            correct: true,
            points: 0,
        },
        Choice {
            id: "f".into(),
            text: "False".into(),
            correct: false,
            points: 0,
        },
    ];
    bank_q.question = Some(bq);
    let mut bank_q2 = make_empty_slide("bank_q2", "Bank Question 2");
    bank_q2.question = bank_q.question.clone();
    let mut bank_u1 = make_empty_slide("bank_u1", "Unused Bank Question");
    bank_u1.question = bank_q.question.clone();
    c.banks = vec![
        QuestionBank {
            id: "bank1".into(),
            title: "Bank 1".into(),
            slides: vec![bank_q, bank_q2],
        },
        QuestionBank {
            id: "bank_unused".into(),
            title: "Unused Bank".into(),
            slides: vec![bank_u1],
        },
    ];

    // Populate slide1 with all object kinds, layers, cues, triggers, states, motion paths.
    let mut obj_shape = make_shape_obj("obj_shape", 40.0, 40.0, 120.0, 80.0, "#112233");
    obj_shape.states = vec![StateDef {
        name: "custom1".into(),
        fill: Some(Color("#223344".into())),
        text: None,
        src: None,
    }];
    obj_shape.motion_paths = vec![MotionPath {
        id: "mpath1".into(),
        points: vec![Point { x: 0.0, y: 0.0 }, Point { x: 100.0, y: 0.0 }],
        duration_ms: 1000,
        easing: Default::default(),
        relative: true,
    }];

    let obj_shape2 = make_shape_obj("obj_shape2", 200.0, 40.0, 100.0, 60.0, "#445566");
    let obj_shape3 = make_shape_obj("obj_shape3", 360.0, 40.0, 100.0, 60.0, "#445566");

    let mut obj_g1 = make_shape_obj("obj_g1", 500.0, 40.0, 60.0, 60.0, "#AABBCC");
    obj_g1.group = Some("grp1".into());
    let mut obj_g2 = make_shape_obj("obj_g2", 580.0, 40.0, 60.0, 60.0, "#AABBCC");
    obj_g2.group = Some("grp1".into());

    let obj_text = make_text_obj("obj_text", "Hello world", 40.0, 140.0);

    let obj_pic = base_object(
        "obj_pic",
        ObjectKind::Image {
            src: "media/pic.png".into(),
            alt: "Pic".into(),
            crop: None,
            outline: None,
            shape: None,
        },
        Rect {
            x: 40.0,
            y: 210.0,
            w: 120.0,
            h: 80.0,
        },
    );

    let obj_video = base_object(
        "obj_video",
        ObjectKind::Video {
            src: "media/clip.mp4".into(),
            autoplay: false,
            captions: None,
            volume: 1.0,
            trim: None,
            cues: vec![Cue {
                id: "vcue1".into(),
                at_ms: 1000.0,
                name: "Cue 1".into(),
                pause: false,
            }],
            poster: None,
        },
        Rect {
            x: 200.0,
            y: 210.0,
            w: 160.0,
            h: 90.0,
        },
    );

    let obj_lottie = base_object(
        "obj_lottie",
        ObjectKind::Lottie {
            src: "media/anim.json".into(),
            loop_playback: true,
            autoplay: true,
            speed: 1.0,
        },
        Rect {
            x: 400.0,
            y: 210.0,
            w: 100.0,
            h: 100.0,
        },
    );

    let obj_web = base_object(
        "obj_web",
        ObjectKind::Web {
            source: coursecraft_course::WebSource::Url {
                url: "https://example.com".into(),
            },
        },
        Rect {
            x: 520.0,
            y: 210.0,
            w: 120.0,
            h: 90.0,
        },
    );

    let obj_input = base_object(
        "obj_input",
        ObjectKind::TextEntry {
            variable: "var_text".into(),
            placeholder: "Type".into(),
            multiline: false,
            max_chars: 0,
        },
        Rect {
            x: 660.0,
            y: 210.0,
            w: 140.0,
            h: 40.0,
        },
    );

    let obj_marker = base_object(
        "obj_marker",
        ObjectKind::Marker {
            title: "Info".into(),
            body: "Body".into(),
            runs: Vec::new(),
            paragraphs: Vec::new(),
            icon: Default::default(),
            color: Some(Color("#112233".into())),
        },
        Rect {
            x: 660.0,
            y: 270.0,
            w: 32.0,
            h: 32.0,
        },
    );

    let obj_scroll = base_object(
        "obj_scroll",
        ObjectKind::ScrollPanel {
            content_height: 400.0,
            bar: Default::default(),
        },
        Rect {
            x: 720.0,
            y: 270.0,
            w: 150.0,
            h: 150.0,
        },
    );

    let obj_timer = base_object(
        "obj_timer",
        ObjectKind::Timer {
            duration_ms: 10_000,
            format: Default::default(),
            autostart: false,
            style: TextStyle::default(),
        },
        Rect {
            x: 40.0,
            y: 440.0,
            w: 100.0,
            h: 40.0,
        },
    );

    let obj_meter = base_object(
        "obj_meter",
        ObjectKind::ProgressMeter {
            variable: "var1".into(),
            min: 0.0,
            max: 100.0,
            style: Default::default(),
            show_value: true,
        },
        Rect {
            x: 160.0,
            y: 440.0,
            w: 160.0,
            h: 24.0,
        },
    );

    let obj_path = base_object(
        "obj_path",
        ObjectKind::Path {
            path: Path {
                points: vec![PathPoint { x: 0.0, y: 0.0 }, PathPoint { x: 1.0, y: 1.0 }],
                style: Default::default(),
                closed: false,
                subpaths: Vec::new(),
                fill: None,
                fill_style: None,
                outline: Outline {
                    color: Color("#112233".into()),
                    width: 2.0,
                    dash: Default::default(),
                    dash_pattern: None,
                    join: Default::default(),
                    miter_limit: None,
                },
                start_end: Default::default(),
                end_end: Default::default(),
                connector: None,
                start_glue: None,
                end_glue: None,
                text: None,
            },
        },
        Rect {
            x: 340.0,
            y: 440.0,
            w: 100.0,
            h: 50.0,
        },
    );

    let obj_table = make_table_obj("obj_table");

    let s1 = &mut c.scenes[0].slides[0];
    s1.objects = vec![
        obj_shape, obj_shape2, obj_shape3, obj_g1, obj_g2, obj_text, obj_pic, obj_video,
        obj_lottie, obj_web, obj_input, obj_marker, obj_scroll, obj_timer, obj_meter, obj_path,
        obj_table,
    ];
    s1.layers = vec![
        Layer {
            id: "layer1".into(),
            name: "Layer 1".into(),
            duration_ms: 3000,
            hide_other_layers: true,
            pause_base_timeline: false,
            prevent_base_clicks: false,
            hide_base_objects: false,
            hide_when_timeline_ends: false,
            allow_seek: Default::default(),
            revisit: Default::default(),
            objects: Vec::new(),
            triggers: Vec::new(),
            cue_points: Vec::new(),
        },
        Layer {
            id: "layer2".into(),
            name: "Layer 2".into(),
            duration_ms: 3000,
            hide_other_layers: true,
            pause_base_timeline: false,
            prevent_base_clicks: false,
            hide_base_objects: false,
            hide_when_timeline_ends: false,
            allow_seek: Default::default(),
            revisit: Default::default(),
            objects: Vec::new(),
            triggers: Vec::new(),
            cue_points: Vec::new(),
        },
    ];
    s1.cue_points = vec![SlideCue {
        id: "scue1".into(),
        name: "Slide Cue 1".into(),
        at_ms: 1500,
    }];
    s1.triggers = vec![
        Trigger {
            when: Event::Click {
                object: "obj_shape".into(),
            },
            condition: None,
            action: Action::ShowLayer {
                layer: "layer1".into(),
            },
            disabled: false,
        },
        Trigger {
            when: Event::Click {
                object: "obj_shape2".into(),
            },
            condition: None,
            action: Action::ShowLayer {
                layer: "layer2".into(),
            },
            disabled: false,
        },
    ];

    // Additional slides in scene1 for slide.*, question.*, draw.*
    let slide2 = make_empty_slide("slide2", "Slide 2");

    let mut slide_q_mc = make_empty_slide("slide_q_mc", "MC Question");
    let mut q_mc = make_empty_question(QuestionKind::MultipleChoice, "Pick A");
    q_mc.choices = vec![
        Choice {
            id: "c1".into(),
            text: "A".into(),
            correct: true,
            points: 0,
        },
        Choice {
            id: "c2".into(),
            text: "B".into(),
            correct: false,
            points: 0,
        },
    ];
    slide_q_mc.question = Some(q_mc);

    let mut slide_q_fib = make_empty_slide("slide_q_fib", "Fill In Blank");
    let mut q_fib = make_empty_question(QuestionKind::FillIn, "Language?");
    q_fib.accepted = vec!["rust".into(), "Rust".into()];
    slide_q_fib.question = Some(q_fib);

    let mut slide_q_match = make_empty_slide("slide_q_match", "Matching");
    let mut q_match = make_empty_question(QuestionKind::Matching, "Match");
    q_match.pairs = vec![
        MatchPair {
            id: "p1".into(),
            left: "L1".into(),
            right: "R1".into(),
        },
        MatchPair {
            id: "p2".into(),
            left: "L2".into(),
            right: "R2".into(),
        },
        MatchPair {
            id: "p3".into(),
            left: "L3".into(),
            right: "R3".into(),
        },
    ];
    slide_q_match.question = Some(q_match);

    let mut slide_q_seq = make_empty_slide("slide_q_seq", "Sequence");
    let mut q_seq = make_empty_question(QuestionKind::Sequence, "Order");
    q_seq.choices = vec![
        Choice {
            id: "sq1".into(),
            text: "First".into(),
            correct: true,
            points: 0,
        },
        Choice {
            id: "sq2".into(),
            text: "Second".into(),
            correct: true,
            points: 0,
        },
        Choice {
            id: "sq3".into(),
            text: "Third".into(),
            correct: true,
            points: 0,
        },
    ];
    slide_q_seq.question = Some(q_seq);

    let mut slide_q_hotspot = make_empty_slide("slide_q_hotspot", "Hotspot");
    slide_q_hotspot.objects = vec![
        base_object(
            "hs_img",
            ObjectKind::Image {
                src: "media/hotspot.png".into(),
                alt: "Hotspot".into(),
                crop: None,
                outline: None,
                shape: None,
            },
            Rect {
                x: 0.0,
                y: 0.0,
                w: 400.0,
                h: 300.0,
            },
        ),
        base_object(
            "hs_img2",
            ObjectKind::Image {
                src: "media/hotspot2.png".into(),
                alt: "Hotspot 2".into(),
                crop: None,
                outline: None,
                shape: None,
            },
            Rect {
                x: 0.0,
                y: 0.0,
                w: 400.0,
                h: 300.0,
            },
        ),
    ];
    let mut q_hotspot = make_empty_question(QuestionKind::Hotspot, "Click target");
    q_hotspot.hotspot_image = Some("hs_img".into());
    q_hotspot.regions = vec![
        HotspotRegion {
            id: "hr1".into(),
            shape: HotspotShape::Rect,
            x: 10.0,
            y: 10.0,
            w: 100.0,
            h: 100.0,
            correct: true,
        },
        HotspotRegion {
            id: "hr2".into(),
            shape: HotspotShape::Ellipse,
            x: 200.0,
            y: 10.0,
            w: 100.0,
            h: 100.0,
            correct: false,
        },
    ];
    slide_q_hotspot.question = Some(q_hotspot);

    let mut slide_q_dnd = make_empty_slide("slide_q_dnd", "Drag and Drop");
    slide_q_dnd.objects = vec![
        make_shape_obj("dnd_item1", 40.0, 40.0, 80.0, 40.0, "#112233"),
        make_shape_obj("dnd_item2", 40.0, 100.0, 80.0, 40.0, "#112233"),
        make_shape_obj("dnd_item3", 40.0, 160.0, 80.0, 40.0, "#112233"),
        make_shape_obj("dnd_target1", 300.0, 40.0, 120.0, 60.0, "#445566"),
        make_shape_obj("dnd_target2", 300.0, 120.0, 120.0, 60.0, "#445566"),
        make_shape_obj("dnd_target3", 300.0, 200.0, 120.0, 60.0, "#445566"),
    ];
    let mut q_dnd = make_empty_question(QuestionKind::DragDrop, "Drag");
    q_dnd.drag_items = vec![
        DragItem {
            item: "dnd_item1".into(),
            target: "dnd_target1".into(),
        },
        DragItem {
            item: "dnd_item2".into(),
            target: "dnd_target2".into(),
        },
    ];
    q_dnd.drop_targets = vec!["dnd_target1".into(), "dnd_target2".into()];
    slide_q_dnd.question = Some(q_dnd);

    let mut slide_draw = make_empty_slide("slide_draw", "Bank Draw");
    slide_draw.draw = Some(BankDraw {
        bank: "bank1".into(),
        count: 1,
    });

    c.scenes[0].slides.extend([
        slide2,
        slide_q_mc,
        slide_q_fib,
        slide_q_match,
        slide_q_seq,
        slide_q_hotspot,
        slide_q_dnd,
        slide_draw,
    ]);

    c.scenes.push(Scene {
        id: "scene2".into(),
        title: "Scene 2".into(),
        slides: vec![make_empty_slide("scene2_s1", "Scene 2 Slide 1")],
    });

    s
}

fn validation_errors(course: &Course) -> Vec<String> {
    course
        .validate()
        .into_iter()
        .filter(|i| i.severity == Severity::Error)
        .map(|i| format!("{}: {}", i.path, i.message))
        .collect()
}

fn command_cases() -> Vec<(&'static str, Json)> {
    let pasted_obj = serde_json::to_value(make_shape_obj(
        "pasted_src",
        20.0,
        20.0,
        80.0,
        40.0,
        "#112233",
    ))
    .unwrap();
    let pasted_slide = serde_json::to_value(make_empty_slide("pasted_slide", "Pasted")).unwrap();

    vec![
        ("course.update", json!({ "title": "Updated Title" })),
        (
            "course.set_slide_size",
            json!({ "w": 1920, "h": 1080, "mode": "scale" }),
        ),
        ("course.set_title", json!({ "title": "New Title" })),
        (
            "course.set_player",
            json!({ "show_nav": false, "navigation": "restricted" }),
        ),
        ("course.set_quiz", json!({ "passing_percent": 75 })),
        (
            "course.set_background_audio",
            json!({ "src": "audio/new.mp3", "volume": 0.25, "loop": false }),
        ),
        (
            "theme.set_palette",
            json!({ "colors": [{ "id": "pal1", "name": "Brand", "color": "#ABCDEF" }] }),
        ),
        (
            "theme.set_palette_color",
            json!({ "id": "pal1", "color": "#FF00AA", "recolor": true }),
        ),
        (
            "course.recolor",
            json!({ "from": "#112233", "to": "#998877" }),
        ),
        (
            "text_style.create",
            json!({ "name": "Body Style", "style": { "size": 16.0 } }),
        ),
        (
            "text_style.update",
            json!({ "id": "tstyle1", "name": "Subheading", "style": { "size": 20.0 } }),
        ),
        ("text_style.delete", json!({ "id": "tstyle1" })),
        (
            "text_style.apply",
            json!({ "id": "tstyle1", "slide": "slide1", "objects": ["obj_text"] }),
        ),
        ("course.add_finish_button", json!({})),
        (
            "course.set_theme",
            json!({ "accent": "#FF5500", "font": "Georgia" }),
        ),
        (
            "course.add_font",
            json!({ "src": "fonts/bold.ttf", "family": "CustomFont", "style": "bold" }),
        ),
        (
            "course.remove_font",
            json!({ "family": "CustomFont", "style": "regular" }),
        ),
        ("course.import_xliff", json!({ "xliff": TRANSLATION_XLIFF })),
        ("scene.add", json!({ "title": "New Scene" })),
        (
            "scene.update",
            json!({ "scene": "scene2", "title": "Renamed Scene 2" }),
        ),
        ("scene.delete", json!({ "scene": "scene2" })),
        (
            "slide.add",
            json!({ "scene": "scene1", "after": "slide1", "title": "Inserted Slide" }),
        ),
        (
            "slide.update",
            json!({ "slide": "slide1", "title": "Updated Slide 1", "notes": "Note" }),
        ),
        ("slide.delete", json!({ "slide": "slide2" })),
        (
            "slide.move",
            json!({ "slide": "slide2", "scene": "scene2", "index": 0 }),
        ),
        (
            "slide.duplicate",
            json!({ "slide": "slide1", "after": "slide1" }),
        ),
        (
            "slide.paste",
            json!({ "scene": "scene1", "after": "slide1", "slide": pasted_slide }),
        ),
        (
            "slide.set_duration",
            json!({ "slide": "slide1", "ms": 8000 }),
        ),
        (
            "slide.set_background",
            json!({ "slide": "slide1", "background": "#EFEFEF" }),
        ),
        (
            "slide.set_transition",
            json!({ "slide": "slide1", "transition": { "type": "fade", "duration_ms": 400 } }),
        ),
        (
            "slide.set_player_overrides",
            json!({ "slide": "slide1", "show_nav": false }),
        ),
        ("master.add", json!({ "title": "Master 2" })),
        ("master.delete", json!({ "master": "master1" })),
        (
            "slide.set_master",
            json!({ "slides": ["slide1"], "master": "master1" }),
        ),
        ("bank.create", json!({ "title": "Bank 2" })),
        (
            "bank.update",
            json!({ "bank": "bank1", "title": "Renamed Bank" }),
        ),
        ("bank.delete", json!({ "bank": "bank_unused" })),
        (
            "bank.add_question",
            json!({ "bank": "bank1", "title": "Extra Q" }),
        ),
        (
            "bank.import_csv",
            json!({ "csv": CSV_QUESTIONS, "bank": "CSV Bank" }),
        ),
        (
            "bank.move_slide",
            json!({ "slide": "slide_q_mc", "bank": "bank1", "index": 0 }),
        ),
        ("bank.remove_slide", json!({ "slide": "bank_q2" })),
        (
            "draw.insert",
            json!({ "bank": "bank1", "count": 1, "scene": "scene1" }),
        ),
        ("draw.update", json!({ "slide": "slide_draw", "count": 2 })),
        (
            "slide.add_from_layout",
            json!({ "layout": "title_content", "after": "slide1", "master": "master1" }),
        ),
        (
            "course.import_outline",
            json!({
                "outline": {
                    "scenes": [{
                        "title": "Outline Scene",
                        "slides": [{ "title": "Outline Slide", "body": ["Point A"] }]
                    }]
                }
            }),
        ),
        (
            "template.save",
            json!({ "slide": "slide1", "title": "Saved Template" }),
        ),
        (
            "template.add_slide",
            json!({ "template": "tpl1", "after": "slide1" }),
        ),
        ("template.delete", json!({ "template": "tpl1" })),
        ("layer.add", json!({ "slide": "slide1", "name": "Layer 3" })),
        (
            "layer.update",
            json!({ "slide": "slide1", "layer": "layer1", "name": "Feedback", "duration_ms": 4000 }),
        ),
        (
            "layer.delete",
            json!({ "slide": "slide1", "layer": "layer2" }),
        ),
        (
            "layer.move",
            json!({ "slide": "slide1", "layer": "layer2", "to": 0 }),
        ),
        (
            "layer.duplicate",
            json!({ "slide": "slide1", "layer": "layer1" }),
        ),
        (
            "object.add",
            json!({
                "slide": "slide1",
                "kind": { "type": "shape", "shape": "ellipse", "fill": "#123456" },
                "frame": { "x": 10.0, "y": 10.0, "w": 80.0, "h": 80.0 }
            }),
        ),
        (
            "object.update",
            json!({ "slide": "slide1", "object": "obj_shape", "name": "Renamed Shape" }),
        ),
        (
            "object.reorder",
            json!({ "slide": "slide1", "object": "obj_shape", "index": 2 }),
        ),
        (
            "object.align",
            json!({ "slide": "slide1", "objects": ["obj_shape", "obj_shape2"], "edge": "bottom" }),
        ),
        (
            "object.distribute",
            json!({ "slide": "slide1", "objects": ["obj_shape", "obj_shape2", "obj_shape3"], "axis": "horizontal" }),
        ),
        (
            "object.match_size",
            json!({ "slide": "slide1", "objects": ["obj_shape", "obj_shape2"], "dimension": "both" }),
        ),
        (
            "object.group",
            json!({ "slide": "slide1", "objects": ["obj_shape", "obj_shape2"] }),
        ),
        (
            "object.ungroup",
            json!({ "slide": "slide1", "objects": ["obj_g1", "obj_g2"] }),
        ),
        (
            "object.duplicate",
            json!({ "slide": "slide1", "object": "obj_shape" }),
        ),
        (
            "object.paste",
            json!({ "slide": "slide1", "objects": [pasted_obj] }),
        ),
        (
            "object.set_frame",
            json!({ "slide": "slide1", "object": "obj_shape", "frame": { "x": 99.0, "y": 88.0, "w": 140.0, "h": 90.0 } }),
        ),
        (
            "object.set_rotation",
            json!({ "slide": "slide1", "object": "obj_shape", "degrees": 45.0 }),
        ),
        (
            "object.flip",
            json!({ "slide": "slide1", "object": "obj_shape", "axis": "horizontal" }),
        ),
        (
            "object.lock",
            json!({ "slide": "slide1", "object": "obj_shape", "locked": true }),
        ),
        (
            "object.hide_in_editor",
            json!({ "slide": "slide1", "object": "obj_shape", "hidden": true }),
        ),
        (
            "view.set",
            json!({ "show_grid": true, "snap_to_grid": true }),
        ),
        ("view.guides.add", json!({ "vertical": true, "at": 320.0 })),
        ("view.guides.move", json!({ "index": 0, "at": 150.0 })),
        ("view.guides.remove", json!({ "index": 0 })),
        (
            "object.set_opacity",
            json!({ "slide": "slide1", "object": "obj_shape", "opacity": 0.5 }),
        ),
        (
            "object.set_fill_style",
            json!({ "slide": "slide1", "object": "obj_shape", "fill_style": { "type": "linear", "angle": 90.0, "stops": [{ "position": 0.0, "color": "#FF0000" }, { "position": 1.0, "color": "#0000FF" }] } }),
        ),
        (
            "object.set_shadow",
            json!({ "slide": "slide1", "object": "obj_shape", "shadow": { "offset_x": 4.0, "offset_y": 4.0, "blur": 8.0, "color": "#00000080" } }),
        ),
        (
            "object.set_glow",
            json!({ "slide": "slide1", "object": "obj_shape", "glow": { "size": 6.0, "color": "#FFFF0080" } }),
        ),
        (
            "object.apply_style",
            json!({ "slide": "slide1", "source_slide": "slide1", "source": "obj_shape", "objects": ["obj_shape2"] }),
        ),
        (
            "object.set_drag",
            json!({ "slide": "slide1", "object": "obj_shape", "drag": { "targets": ["obj_shape2"], "snap": true, "return_on_miss": true } }),
        ),
        (
            "object.set_lottie",
            json!({ "slide": "slide1", "object": "obj_lottie", "loop": false, "speed": 1.5 }),
        ),
        (
            "object.set_web_source",
            json!({ "slide": "slide1", "object": "obj_web", "source": { "type": "url", "url": "https://example.org" } }),
        ),
        (
            "object.set_input",
            json!({ "slide": "slide1", "object": "obj_input", "patch": { "placeholder": "Enter value" } }),
        ),
        (
            "object.set_marker",
            json!({ "slide": "slide1", "object": "obj_marker", "patch": { "title": "Updated Marker" } }),
        ),
        (
            "object.set_panel",
            json!({ "slide": "slide1", "object": "obj_shape", "panel": "obj_scroll" }),
        ),
        (
            "object.set_scroll_panel",
            json!({ "slide": "slide1", "object": "obj_scroll", "patch": { "content_height": 600.0 } }),
        ),
        (
            "object.set_timer",
            json!({ "slide": "slide1", "object": "obj_timer", "patch": { "duration_ms": 20000, "autostart": true } }),
        ),
        (
            "object.set_meter",
            json!({ "slide": "slide1", "object": "obj_meter", "patch": { "max": 200.0 } }),
        ),
        (
            "object.set_path",
            json!({ "slide": "slide1", "object": "obj_path", "points": [{ "x": 0.0, "y": 0.0 }, { "x": 0.5, "y": 1.0 }] }),
        ),
        (
            "object.set_crop",
            json!({ "slide": "slide1", "object": "obj_pic", "crop": { "x": 0.1, "y": 0.1, "w": 0.8, "h": 0.8 } }),
        ),
        (
            "object.set_captions",
            json!({ "slide": "slide1", "object": "obj_video", "captions": "media/clip.vtt" }),
        ),
        (
            "object.set_media",
            json!({ "slide": "slide1", "object": "obj_video", "volume": 0.6 }),
        ),
        (
            "video.add_cue",
            json!({ "slide": "slide1", "object": "obj_video", "at_ms": 2500, "name": "Cue 2" }),
        ),
        (
            "video.set_cue",
            json!({ "slide": "slide1", "object": "obj_video", "cue": "vcue1", "patch": { "name": "Renamed Cue" } }),
        ),
        (
            "video.remove_cue",
            json!({ "slide": "slide1", "object": "obj_video", "cue": "vcue1" }),
        ),
        (
            "video.add_question_cue",
            json!({ "slide": "slide1", "object": "obj_video", "at_ms": 2000, "name": "Q Cue" }),
        ),
        (
            "slide.add_cue",
            json!({ "slide": "slide1", "at_ms": 2500, "name": "Slide Cue 2" }),
        ),
        (
            "slide.set_cue",
            json!({ "slide": "slide1", "cue": "scue1", "patch": { "name": "Renamed Slide Cue" } }),
        ),
        (
            "slide.remove_cue",
            json!({ "slide": "slide1", "cue": "scue1" }),
        ),
        (
            "object.add_state",
            json!({ "slide": "slide1", "object": "obj_shape", "state": { "name": "hover", "fill": "#FFAA00" } }),
        ),
        (
            "object.update_state",
            json!({ "slide": "slide1", "object": "obj_shape", "state": { "name": "custom1", "fill": "#00AAFF" } }),
        ),
        (
            "object.delete_state",
            json!({ "slide": "slide1", "object": "obj_shape", "name": "custom1" }),
        ),
        (
            "object.delete",
            json!({ "slide": "slide1", "object": "obj_shape3" }),
        ),
        (
            "object.set_animation",
            json!({ "slide": "slide1", "object": "obj_shape", "which": "entrance", "animation": { "effect": { "type": "fade" }, "duration_ms": 500 } }),
        ),
        (
            "object.add_motion_path",
            json!({ "slide": "slide1", "object": "obj_shape2", "duration_ms": 1200 }),
        ),
        (
            "object.update_motion_path",
            json!({ "slide": "slide1", "object": "obj_shape", "path": "mpath1", "duration_ms": 1500 }),
        ),
        (
            "object.remove_motion_path",
            json!({ "slide": "slide1", "object": "obj_shape", "path": "mpath1" }),
        ),
        (
            "text.set",
            json!({ "slide": "slide1", "object": "obj_text", "text": "Updated text" }),
        ),
        (
            "text.style_range",
            json!({ "slide": "slide1", "object": "obj_text", "start": 0, "end": 5, "bold": true }),
        ),
        (
            "text.set_paragraph",
            json!({ "slide": "slide1", "object": "obj_text", "align": "center" }),
        ),
        (
            "text.set_paragraphs",
            json!({ "slide": "slide1", "object": "obj_text", "list": "bullet" }),
        ),
        (
            "text.set_link",
            json!({ "slide": "slide1", "object": "obj_text", "link": { "type": "url", "url": "https://example.com" } }),
        ),
        (
            "object.set_autofit",
            json!({ "slide": "slide1", "object": "obj_text", "autofit": "shrink" }),
        ),
        (
            "table.set_cell",
            json!({ "slide": "slide1", "object": "obj_table", "row": 1, "column": 1, "text": "Edited" }),
        ),
        (
            "table.insert_row",
            json!({ "slide": "slide1", "object": "obj_table", "index": 2 }),
        ),
        (
            "table.delete_row",
            json!({ "slide": "slide1", "object": "obj_table", "index": 2 }),
        ),
        (
            "table.insert_column",
            json!({ "slide": "slide1", "object": "obj_table", "index": 2 }),
        ),
        (
            "table.delete_column",
            json!({ "slide": "slide1", "object": "obj_table", "index": 2 }),
        ),
        (
            "table.resize",
            json!({ "slide": "slide1", "object": "obj_table", "columns": [0.4, 0.3, 0.3] }),
        ),
        (
            "table.merge",
            json!({ "slide": "slide1", "object": "obj_table", "row": 1, "column": 0, "rows": 2, "columns": 2 }),
        ),
        (
            "table.split",
            json!({ "slide": "slide1", "object": "obj_table", "row": 0, "column": 0 }),
        ),
        (
            "table.set_borders",
            json!({ "slide": "slide1", "object": "obj_table", "outer": { "color": "#000000", "width": 2.0 } }),
        ),
        (
            "table.set_header_row",
            json!({ "slide": "slide1", "object": "obj_table", "header_row": true }),
        ),
        (
            "trigger.add",
            json!({
                "slide": "slide1",
                "trigger": serde_json::to_value(Trigger {
                    when: Event::TimelineStarts { layer: None },
                    condition: None,
                    action: Action::ShowLayer { layer: "layer1".into() },
                    disabled: false,
                }).unwrap()
            }),
        ),
        (
            "trigger.update",
            json!({
                "slide": "slide1",
                "index": 0,
                "trigger": serde_json::to_value(Trigger {
                    when: Event::Click { object: "obj_shape".into() },
                    condition: None,
                    action: Action::HideLayer { layer: Some("layer1".into()) },
                    disabled: false,
                }).unwrap()
            }),
        ),
        (
            "trigger.move",
            json!({ "slide": "slide1", "from": 0, "to": 1 }),
        ),
        ("trigger.delete", json!({ "slide": "slide1", "index": 0 })),
        (
            "resource.add",
            json!({ "title": "Doc", "target": { "kind": "url", "url": "https://example.com/doc" } }),
        ),
        (
            "resource.update",
            json!({ "id": "res1", "title": "Updated Guide" }),
        ),
        ("resource.remove", json!({ "id": "res1" })),
        ("resource.move", json!({ "id": "res2", "to": 0 })),
        (
            "glossary.add",
            json!({ "term": "LTI", "definition": "Learning Tools Interoperability" }),
        ),
        (
            "glossary.update",
            json!({ "id": "gloss1", "definition": "Sharable Content Object Reference Model" }),
        ),
        ("glossary.remove", json!({ "id": "gloss1" })),
        ("glossary.move", json!({ "id": "gloss2", "to": 0 })),
        (
            "variable.add",
            json!({ "name": "score_flag", "value": false }),
        ),
        ("variable.set_value", json!({ "name": "var1", "value": 42 })),
        ("variable.delete", json!({ "name": "var_unused" })),
        (
            "interaction.insert",
            json!({ "slide": "slide2", "kind": "tabs" }),
        ),
        ("question.insert", json!({ "slide": "slide2" })),
        ("results.insert", json!({ "after": "slide1" })),
        ("certificate.insert", json!({ "after": "slide1" })),
        (
            "question.add_submit_button",
            json!({ "slide": "slide_q_mc" }),
        ),
        (
            "question.add_feedback_layers",
            json!({ "slide": "slide_q_mc" }),
        ),
        (
            "question.add_try_again_layer",
            json!({ "slide": "slide_q_mc" }),
        ),
        (
            "question.set",
            json!({
                "slide": "slide2",
                "question": {
                    "kind": "true_false",
                    "prompt": "True or false?",
                    "choices": [
                        { "id": "t", "text": "True", "correct": true },
                        { "id": "f", "text": "False", "correct": false }
                    ],
                    "points": 5
                }
            }),
        ),
        ("question.delete", json!({ "slide": "slide_q_mc" })),
        (
            "question.set_kind",
            json!({ "slide": "slide_q_mc", "kind": "multiple_response" }),
        ),
        (
            "question.add_accepted",
            json!({ "slide": "slide_q_fib", "text": "RUST" }),
        ),
        (
            "question.remove_accepted",
            json!({ "slide": "slide_q_fib", "index": 1 }),
        ),
        (
            "question.update_accepted",
            json!({ "slide": "slide_q_fib", "index": 0, "text": "cargo" }),
        ),
        (
            "question.set_case_sensitive",
            json!({ "slide": "slide_q_fib", "case_sensitive": true }),
        ),
        (
            "question.add_pair",
            json!({ "slide": "slide_q_match", "left": "L3", "right": "R3" }),
        ),
        (
            "question.remove_pair",
            json!({ "slide": "slide_q_match", "id": "p2" }),
        ),
        (
            "question.update_pair",
            json!({ "slide": "slide_q_match", "id": "p1", "left": "Left 1" }),
        ),
        (
            "question.reorder_pair",
            json!({ "slide": "slide_q_match", "id": "p1", "to": 1 }),
        ),
        (
            "question.reorder_sequence",
            json!({ "slide": "slide_q_seq", "id": "sq1", "to": 2 }),
        ),
        (
            "question.set_hotspot_image",
            json!({ "slide": "slide_q_hotspot", "image": "hs_img2" }),
        ),
        (
            "question.add_region",
            json!({ "slide": "slide_q_hotspot", "x": 50.0, "y": 50.0, "w": 60.0, "h": 60.0, "correct": false }),
        ),
        (
            "question.update_region",
            json!({ "slide": "slide_q_hotspot", "id": "hr1", "w": 150.0 }),
        ),
        (
            "question.remove_region",
            json!({ "slide": "slide_q_hotspot", "id": "hr2" }),
        ),
        (
            "question.set_markers",
            json!({ "slide": "slide_q_hotspot", "markers": 3 }),
        ),
        (
            "question.add_drag_item",
            json!({ "slide": "slide_q_dnd", "item": "dnd_item3", "target": "dnd_target1" }),
        ),
        (
            "question.update_drag_item",
            json!({ "slide": "slide_q_dnd", "item": "dnd_item1", "target": "dnd_target2" }),
        ),
        (
            "question.remove_drag_item",
            json!({ "slide": "slide_q_dnd", "item": "dnd_item2" }),
        ),
        (
            "question.set_drop_targets",
            json!({ "slide": "slide_q_dnd", "targets": ["dnd_target1", "dnd_target2", "dnd_target3"] }),
        ),
        (
            "question.set_drag_options",
            json!({ "slide": "slide_q_dnd", "snap_to_center": false }),
        ),
        (
            "course.replace",
            json!({ "query": "Hello", "replacement": "Hi" }),
        ),
    ]
}

#[test]
fn d5_1_fixture_course_is_valid() {
    let s = fixture_session();
    let errs = validation_errors(&s.course);
    assert!(errs.is_empty(), "fixture course invalid: {errs:#?}");
}

#[test]
fn d5_1_all_mutating_commands_undo_redo_round_trip() {
    let cases = command_cases();

    let registry_mutating: Vec<&'static str> = registry()
        .iter()
        .filter(|c| c.mutates)
        .map(|c| c.id)
        .collect();

    let mut seen = HashSet::new();
    for &(id, _) in &cases {
        assert!(
            seen.insert(id),
            "duplicate command entry in command_cases(): {id}"
        );
        assert!(
            registry_mutating.contains(&id),
            "command_cases() entry {id} is not a mutates==true command in REGISTRY"
        );
    }

    let missing: Vec<&'static str> = registry_mutating
        .iter()
        .copied()
        .filter(|id| !seen.contains(id))
        .collect();
    assert!(
        missing.is_empty(),
        "REGISTRY mutating commands missing from D5.1 test table ({}): {missing:?}",
        missing.len()
    );

    let mut failures = Vec::new();
    for (id, params) in &cases {
        let mut s = fixture_session();
        let before = s.course.clone();

        let res = s.execute(id, params);
        if let Err(e) = res {
            failures.push(format!("{id}: failed to execute: {e:?}"));
            continue;
        }

        let after = s.course.clone();
        if after == before {
            failures.push(format!("{id}: did not mutate the course"));
            continue;
        }

        let errs = validation_errors(&s.course);
        if !errs.is_empty() {
            failures.push(format!("{id}: validation errors: {errs:?}"));
            continue;
        }

        if let Err(e) = s.execute("edit.undo", &json!({})) {
            failures.push(format!("{id}: first edit.undo failed: {e:?}"));
            continue;
        }
        if s.course != before {
            failures.push(format!("{id}: first edit.undo did not restore before"));
            continue;
        }

        if let Err(e) = s.execute("edit.redo", &json!({})) {
            failures.push(format!("{id}: edit.redo failed: {e:?}"));
            continue;
        }
        if s.course != after {
            failures.push(format!("{id}: edit.redo did not restore after"));
            continue;
        }

        if let Err(e) = s.execute("edit.undo", &json!({})) {
            failures.push(format!("{id}: second edit.undo failed: {e:?}"));
            continue;
        }
        if s.course != before {
            failures.push(format!("{id}: second edit.undo did not restore before"));
        }
    }

    assert!(
        failures.is_empty(),
        "D5.1 failures ({}):\n{}",
        failures.len(),
        failures.join("\n")
    );
}
