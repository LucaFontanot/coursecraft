//! M39 Package F: theme heading font, the Theme Fonts pickers, Design › Fonts,
//! and Scale in the Size and Position panel.

use crate::ribbon::spec::{self, Menu, UiMenu};
use crate::size_panel::{original_size, reset_frame, scale_percent, scaled_frame};
use crate::test_harness::TestHarness;
use crate::theme_dialog::{builtin_font_pairs, font_pair_params, open_theme_fonts_dialog};
use coursecraft_course::{Course, CropRect, Rect};

fn course() -> Course {
    Course::from_json(include_str!("../../../examples/hello-course/course.json"))
        .expect("hello course")
}

#[test]
fn theme_font_rows_list_heading_then_body_with_roles() {
    let rows = crate::font_picker::theme_font_rows("Lora", "Inter");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0], ("Lora (Headings)".to_owned(), "Lora".to_owned()));
    assert_eq!(rows[1], ("Inter (Body)".to_owned(), "Inter".to_owned()));
    assert!(crate::font_picker::theme_font_rows(" ", "").is_empty());
}

#[test]
fn builtin_pairs_start_with_inter_and_use_course_fonts() {
    let none = builtin_font_pairs(&[]);
    assert_eq!(none, vec![("Inter".to_owned(), "Inter".to_owned())]);
    let pairs = builtin_font_pairs(&["Inter", "Lora"]);
    assert_eq!(pairs.len(), 3);
    assert!(pairs.contains(&("Lora".to_owned(), "Inter".to_owned())));
    assert!(pairs.contains(&("Inter".to_owned(), "Lora".to_owned())));
}

#[test]
fn pair_params_clear_the_heading_when_it_equals_the_body() {
    let same = font_pair_params("inter", "Inter");
    assert_eq!(same["heading_font"], serde_json::Value::Null);
    assert_eq!(same["font"], "Inter");
    let diff = font_pair_params("Lora", "Inter");
    assert_eq!(diff["heading_font"], "Lora");
}

#[test]
fn saving_a_pair_is_one_undo_step() {
    let mut h = TestHarness::new(course());
    let before = h.editor.session.undo_len();
    h.editor
        .run("course.set_theme", font_pair_params("Lora", "Roboto"))
        .expect("set theme");
    assert_eq!(h.editor.session.undo_len(), before + 1);
    let theme = &h.editor.session.course.theme;
    assert_eq!(theme.heading_font.as_deref(), Some("Lora"));
    assert_eq!(theme.font, "Roboto");
}

#[test]
fn the_dialog_opens_with_the_current_theme_fonts() {
    let mut h = TestHarness::new(course());
    h.editor
        .run(
            "course.set_theme",
            serde_json::json!({ "heading_font": "Lora" }),
        )
        .expect("set");
    open_theme_fonts_dialog(&mut h.editor);
    let d = &h.editor.ui.theme_dialog;
    assert!(d.fonts_open);
    assert_eq!(d.fonts_heading, "Lora");
    assert_eq!(d.fonts_body, h.editor.session.course.theme.font);
    h.frame(Vec::new());
    assert!(h.editor.ui.theme_dialog.fonts_open, "the dialog draws");
}

#[test]
fn the_heading_font_counts_as_in_use() {
    let mut h = TestHarness::new(course());
    assert!(!h.editor.font_family_in_use("Lora"));
    h.editor
        .run(
            "course.set_theme",
            serde_json::json!({ "heading_font": "Lora" }),
        )
        .expect("set");
    assert!(h.editor.font_family_in_use("Lora"));
    assert!(h.editor.font_family_in_use("lora"));
}

#[test]
fn design_fonts_is_a_split_with_the_theme_fonts_menu() {
    let item = spec::all_items()
        .find(|i| i.id == "design.fonts")
        .expect("design.fonts");
    assert!(item.kind.is_split());
    assert!(matches!(item.menu, Menu::Ui(UiMenu::ThemeFonts)));
}

#[test]
fn scale_percent_is_relative_to_the_base() {
    assert!((scale_percent(150.0, 100.0) - 150.0).abs() < 1e-4);
    assert!((scale_percent(150.0, 0.0) - 100.0).abs() < 1e-4);
}

#[test]
fn scale_sets_one_side_or_both_when_the_aspect_is_locked() {
    let frame = Rect {
        x: 10.0,
        y: 20.0,
        w: 200.0,
        h: 100.0,
    };
    let base = (200.0, 100.0);
    let free = scaled_frame(frame, base, 50.0, true, false);
    assert_eq!((free.w, free.h, free.x, free.y), (100.0, 100.0, 10.0, 20.0));
    let locked = scaled_frame(frame, base, 50.0, true, true);
    assert_eq!((locked.w, locked.h), (100.0, 50.0));
    let by_height = scaled_frame(frame, base, 200.0, false, true);
    assert_eq!((by_height.w, by_height.h), (400.0, 200.0));
    let tiny = scaled_frame(frame, base, 0.0, true, false);
    assert!(tiny.w >= 1.0, "a zero percent never gives an empty frame");
}

#[test]
fn original_size_applies_the_crop_and_rejects_empty_sizes() {
    assert_eq!(original_size((800.0, 600.0), None), Some((800.0, 600.0)));
    let crop = CropRect {
        x: 0.0,
        y: 0.0,
        w: 0.5,
        h: 0.5,
    };
    assert_eq!(
        original_size((800.0, 600.0), Some(crop)),
        Some((400.0, 300.0))
    );
    assert_eq!(original_size((0.0, 600.0), None), None);
    assert_eq!(original_size((f32::NAN, 600.0), None), None);
}

#[test]
fn reset_to_original_size_is_clamped_to_the_slide() {
    let frame = Rect {
        x: 600.0,
        y: 300.0,
        w: 100.0,
        h: 100.0,
    };
    let slide = (1280.0, 720.0);
    let small = reset_frame(frame, (200.0, 100.0), slide);
    assert_eq!((small.w, small.h), (200.0, 100.0));
    assert!(small.x >= 0.0 && small.x + small.w <= slide.0);
    let huge = reset_frame(frame, (4000.0, 2000.0), slide);
    assert!(huge.w <= slide.0 && huge.h <= slide.1, "{huge:?}");
    assert!((huge.w / huge.h - 2.0).abs() < 1e-3, "aspect is kept");
    assert!(huge.x >= 0.0 && huge.y >= 0.0);
    assert!(huge.x + huge.w <= slide.0 + 0.01 && huge.y + huge.h <= slide.1 + 0.01);
}
