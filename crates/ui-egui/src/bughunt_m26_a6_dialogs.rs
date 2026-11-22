//! M26 bug-hunt tests for A6: Dialog reopening at full size.
//!
//! Ordered P1 -> P2 -> P3.

use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use egui::{Id, Key, Modifiers, Rect, pos2, vec2};

fn hello() -> Course {
    let json = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/hello-course/course.json"
    ))
    .expect("hello course");
    Course::from_json(&json).expect("valid course")
}

fn open_insert_interaction(h: &mut TestHarness) {
    let insert = h.find_text("Insert").expect("Insert menu");
    h.click_at(insert);
    h.idle_frame();
    let item = h.find_text("Interaction…").expect("Interaction… menu item");
    h.click_at(item);
    h.idle_frame();
    h.idle_frame();
}

fn dialog_rect(h: &mut TestHarness, title: &str) -> Rect {
    let id = Id::unique(("coursecraft_dialog", title));
    if let Some(r) = h.ctx.memory(|m| m.area_rect(id)) {
        return r;
    }
    let rep = h.layout_report(1);
    rep.windows
        .into_iter()
        .map(|(_, r)| r)
        .max_by(|a, b| (a.width() * a.height()).total_cmp(&(b.width() * b.height())))
        .unwrap_or_else(|| panic!("dialog '{title}' area_rect missing"))
}

// ============================================================================
// P1
// ============================================================================

#[test]
fn a6_1_every_interaction_card_reopens_full_size() {
    let cards = [
        "Tabs",
        "Accordion",
        "Marker",
        "Scrolling panel",
        "Timer",
        "Progress meter",
        "Flip cards",
        "Hotspot reveal",
        "Process steps",
        "Timeline",
        "Reveal grid",
        "Scenario choice",
    ];
    for card in cards {
        let mut h = TestHarness::new(hello());
        h.idle_frame();
        open_insert_interaction(&mut h);
        let r1 = dialog_rect(&mut h, "Insert interaction");
        assert!(
            r1.width() >= 700.0 && r1.height() >= 300.0,
            "{card} 1st open too small: {r1:?}"
        );

        let pos = h
            .find_text_in(card, r1)
            .unwrap_or_else(|| panic!("card '{card}' not found in {r1:?}"));
        h.click_at(pos);
        h.idle_frame();
        assert!(
            !h.editor.ui.show_interaction_gallery,
            "{card} should close gallery"
        );

        open_insert_interaction(&mut h);
        let r2 = dialog_rect(&mut h, "Insert interaction");
        assert!(
            r2.width() >= 700.0 && r2.height() >= 300.0,
            "{card} 2nd open collapsed: {r2:?}"
        );
        let report = h.layout_report(2);
        let bad: Vec<_> = report
            .findings
            .iter()
            .filter(|f| !f.contains("window-off-screen") && !f.contains("backg"))
            .collect();
        assert!(bad.is_empty(), "{card} layout findings: {bad:?}");
    }
}

#[test]
fn a6_2_close_by_x_escape_and_card_reopens_full_size() {
    let mut h = TestHarness::new(hello());
    h.idle_frame();
    open_insert_interaction(&mut h);
    let r1 = dialog_rect(&mut h, "Insert interaction");
    assert!(r1.width() >= 700.0 && r1.height() >= 300.0);

    // Close via title-bar ×
    let x_pos = h
        .find_text_in("×", r1)
        .unwrap_or_else(|| pos2(r1.max.x - 16.0, r1.min.y + 14.0));
    h.click_at(x_pos);
    h.idle_frame();
    assert!(!h.editor.ui.show_interaction_gallery);

    open_insert_interaction(&mut h);
    let r2 = dialog_rect(&mut h, "Insert interaction");
    assert!(r2.width() >= 700.0 && r2.height() >= 300.0);

    // Close via Escape
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    assert!(!h.editor.ui.show_interaction_gallery);

    open_insert_interaction(&mut h);
    let r3 = dialog_rect(&mut h, "Insert interaction");
    assert!(r3.width() >= 700.0 && r3.height() >= 300.0);

    // Close via card click
    let acc = h.find_text_in("Accordion", r3).expect("Accordion");
    h.click_at(acc);
    h.idle_frame();
    assert!(!h.editor.ui.show_interaction_gallery);

    open_insert_interaction(&mut h);
    let r4 = dialog_rect(&mut h, "Insert interaction");
    assert!(r4.width() >= 700.0 && r4.height() >= 300.0);
}

#[test]
fn a6_3_shapes_gallery_filtered_close_reopens_full_size() {
    let mut h = TestHarness::new(hello());
    h.idle_frame();
    h.editor.ui.show_shape_gallery_window = true;
    h.idle_frame();
    h.idle_frame();
    let r1 = dialog_rect(&mut h, "Shapes");
    assert!(r1.width() >= 300.0 && r1.height() >= 200.0, "r1={r1:?}");

    let search = h.editor.ui.shape_gallery_search_rect.expect("shape search");
    h.set_field(search.center(), "Heart");
    h.idle_frame();
    let heart_rect = h
        .editor
        .ui
        .shape_gallery_rects
        .iter()
        .find_map(|(k, r)| (*k == coursecraft_course::ShapeKind::Heart).then_some(*r))
        .expect("Heart icon");
    h.click_at(heart_rect.center());
    h.idle_frame();

    // Reopen Shapes dialog
    h.editor.ui.show_shape_gallery_window = true;
    h.idle_frame();
    h.idle_frame();

    let r2 = dialog_rect(&mut h, "Shapes");
    assert!(
        r2.width() >= 300.0 && r2.height() >= 200.0,
        "Shapes reopened collapsed after filter: {r2:?}"
    );
    let report = h.layout_report(2);
    assert!(
        report.findings.is_empty(),
        "Shapes layout findings: {:?}",
        report.findings
    );
}

#[test]
fn a6_4_sweep_other_dialogs_reopen_full_size() {
    let cases: [(&str, &str, &str); 6] = [
        ("Insert", "Web object…", "Insert web object"),
        ("File", "Slide Size…", "Slide Size"),
        ("Project", "Theme…", "Theme"),
        ("Project", "Course Settings", "Course Settings"),
        ("File", "Publish…", "Publish course"),
        ("Edit", "Find and Replace…", "Find and Replace"),
    ];
    for (menu, item, title) in cases {
        let mut h = TestHarness::new(hello());
        h.editor.file_ops_available = true;
        h.idle_frame();
        let m = h.find_text(menu).unwrap_or_else(|| panic!("menu {menu}"));
        h.click_at(m);
        h.idle_frame();
        let it = h.find_text(item).unwrap_or_else(|| panic!("item {item}"));
        h.click_at(it);
        h.idle_frame();
        h.idle_frame();
        let r1 = dialog_rect(&mut h, title);
        assert!(
            r1.width() >= 220.0 && r1.height() >= 100.0,
            "{title} 1st open too small: {r1:?}"
        );

        h.press(Key::Escape, Modifiers::NONE);
        h.idle_frame();

        let m2 = h.find_text(menu).unwrap();
        h.click_at(m2);
        h.idle_frame();
        let it2 = h.find_text(item).unwrap();
        h.click_at(it2);
        h.idle_frame();
        h.idle_frame();
        let r2 = dialog_rect(&mut h, title);
        assert!(
            r2.width() >= r1.width() * 0.9
                && r2.height() >= r1.height() * 0.9
                && r2.height() >= 100.0,
            "{title} collapsed on 2nd open: r1={r1:?}, r2={r2:?}"
        );
    }
}

#[test]
fn a6_9_fixed_footer_dialogs_on_short_screen_reopen_unclipped() {
    let cases: [(&str, &str, &str); 2] = [
        ("Project", "Theme…", "Theme"),
        ("Project", "Course Settings", "Course Settings"),
    ];
    for (menu, item, title) in cases {
        let mut h = TestHarness::new_with_size(hello(), (1024.0, 620.0));
        h.editor.file_ops_available = true;
        h.idle_frame();
        let m = h.find_text(menu).unwrap_or_else(|| panic!("menu {menu}"));
        h.click_at(m);
        h.idle_frame();
        let it = h.find_text(item).unwrap_or_else(|| panic!("item {item}"));
        h.click_at(it);
        h.idle_frame();
        h.idle_frame();
        let r1 = dialog_rect(&mut h, title);
        assert!(r1.height() >= 100.0, "{title} 1st open small: {r1:?}");
        let rep1 = h.layout_report(3);
        assert!(
            rep1.findings.is_empty(),
            "{title} 1st open findings: {:?}",
            rep1.findings
        );

        h.press(Key::Escape, Modifiers::NONE);
        h.idle_frame();

        let m2 = h.find_text(menu).unwrap_or_else(|| panic!("menu2 {menu}"));
        h.click_at(m2);
        h.idle_frame();
        let it2 = h.find_text(item).unwrap_or_else(|| panic!("item2 {item}"));
        h.click_at(it2);
        h.idle_frame();
        h.idle_frame();
        let rep2 = h.layout_report(3);
        assert!(
            rep2.findings.is_empty(),
            "{title} 2nd open findings: {:?}",
            rep2.findings
        );
    }

    // Publish course on 1024x620
    let mut h = TestHarness::new_with_size(hello(), (1024.0, 620.0));
    h.editor.file_ops_available = true;
    h.idle_frame();
    h.editor.open_publish_dialog();
    h.idle_frame();
    h.idle_frame();
    assert!(dialog_rect(&mut h, "Publish course").height() >= 100.0);
    let rep1 = h.layout_report(3);
    assert!(
        rep1.findings.is_empty(),
        "Publish 1st open findings: {:?}",
        rep1.findings
    );
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();
    h.editor.open_publish_dialog();
    h.idle_frame();
    h.idle_frame();
    let rep2 = h.layout_report(3);
    assert!(
        rep2.findings.is_empty(),
        "Publish 2nd open findings: {:?}",
        rep2.findings
    );
}

// ============================================================================
// P2
// ============================================================================

#[test]
fn a6_5_dialog_dragged_by_title_bar_reopens_full_size_on_screen() {
    let mut h = TestHarness::new(hello());
    h.idle_frame();
    open_insert_interaction(&mut h);
    let r1 = dialog_rect(&mut h, "Insert interaction");
    h.drag_by(
        pos2(r1.center().x, r1.min.y + 12.0),
        vec2(120.0, 60.0),
        6,
        Modifiers::NONE,
    );
    h.idle_frame();
    let marker = h.find_text("Marker").expect("Marker");
    h.click_at(marker);
    h.idle_frame();

    open_insert_interaction(&mut h);
    let r2 = dialog_rect(&mut h, "Insert interaction");
    assert!(r2.width() >= 700.0 && r2.height() >= 300.0);
    let report = h.layout_report(2);
    assert!(
        report.findings.is_empty(),
        "findings: {:?}",
        report.findings
    );
}

#[test]
fn a6_6_window_shrink_and_enlarge_between_dialog_opens() {
    let mut h = TestHarness::new_with_size(hello(), (1280.0, 900.0));
    h.idle_frame();
    open_insert_interaction(&mut h);
    assert!(dialog_rect(&mut h, "Insert interaction").width() >= 700.0);
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();

    h.screen_rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(900.0, 600.0));
    h.idle_frame();
    open_insert_interaction(&mut h);
    let rep_small = h.layout_report(2);
    assert!(
        rep_small.findings.is_empty(),
        "small findings: {:?}",
        rep_small.findings
    );
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();

    h.screen_rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(1280.0, 900.0));
    h.idle_frame();
    open_insert_interaction(&mut h);
    let r_big = dialog_rect(&mut h, "Insert interaction");
    assert!(r_big.width() >= 700.0 && r_big.height() >= 300.0);
}

#[test]
fn a6_8_alternating_between_two_dialogs_no_bleed() {
    let mut h = TestHarness::new(hello());
    h.idle_frame();
    open_insert_interaction(&mut h);
    let marker = h.find_text("Marker").expect("Marker");
    h.click_at(marker);
    h.idle_frame();

    h.editor.ui.show_shape_gallery_window = true;
    h.idle_frame();
    h.idle_frame();
    assert!(dialog_rect(&mut h, "Shapes").width() >= 300.0);
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();

    let slide_menu = h.find_text("Project").expect("Project");
    h.click_at(slide_menu);
    h.idle_frame();
    let theme = h.find_text("Theme…").expect("Theme…");
    h.click_at(theme);
    h.idle_frame();
    h.idle_frame();
    assert!(dialog_rect(&mut h, "Theme").height() >= 100.0);
    h.press(Key::Escape, Modifiers::NONE);
    h.idle_frame();

    open_insert_interaction(&mut h);
    let r = dialog_rect(&mut h, "Insert interaction");
    assert!(r.width() >= 700.0 && r.height() >= 300.0);
}

#[test]
fn a6_10_undo_after_interaction_choice_then_reopen() {
    let mut h = TestHarness::new(hello());
    h.idle_frame();
    open_insert_interaction(&mut h);
    let tabs = h.find_text("Tabs").expect("Tabs");
    h.click_at(tabs);
    h.idle_frame();

    assert!(h.editor.session.undo().is_ok());
    h.idle_frame();

    open_insert_interaction(&mut h);
    let r2 = dialog_rect(&mut h, "Insert interaction");
    assert!(r2.width() >= 700.0 && r2.height() >= 300.0);
    let acc = h.find_text("Accordion").expect("Accordion");
    h.click_at(acc);
    h.idle_frame();
    assert!(h.editor.session.course.validate().is_empty());
}

// ============================================================================
// P3
// ============================================================================

#[test]
fn a6_7_zero_idle_frames_between_close_and_reopen() {
    let mut h = TestHarness::new(hello());
    h.idle_frame();
    open_insert_interaction(&mut h);
    let tabs = h.find_text("Tabs").expect("Tabs");
    h.click_at(tabs);
    // Immediately reopen without extra idle frames
    h.editor.ui.show_interaction_gallery = true;
    h.idle_frame();
    h.idle_frame();
    let r = dialog_rect(&mut h, "Insert interaction");
    assert!(
        r.width() >= 700.0 && r.height() >= 300.0,
        "immediate reopen: {r:?}"
    );
}
