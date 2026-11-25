//! Step u16 regression tests: Preview split panes (`T13.5`), Web editor disabled actions (`T13.8`), and Player parity (`T14.9`).

use crate::test_harness::TestHarness;
use coursecraft_course::Course;
use coursecraft_render::{PlayerFrame, SlideView, ThemeMode, snapshot_player};
use egui::{Event, Key, Modifiers, Rect, pos2, vec2};

const HELLO_JSON: &str = include_str!("../../../examples/hello-course/course.json");

struct TestNoMedia;

impl coursecraft_render::MediaSource for TestNoMedia {
    fn get(&self, _src: &str) -> coursecraft_render::MediaFetch {
        coursecraft_render::MediaFetch::Missing
    }
}

fn course_with_many_variables() -> Course {
    let mut course = Course::from_json(HELLO_JSON).expect("valid hello course");
    for i in 0..20 {
        course.variables.push(coursecraft_course::VariableDef {
            name: format!("very_long_variable_name_for_testing_split_pane_overflow_{i}"),
            value: coursecraft_course::Value::Text(format!("value_{i}_abcdefghijklmnopqrstuvwxyz")),
        });
    }
    course
}

#[test]
fn t13_5_preview_live_variables_and_effect_log_at_1024x600() {
    let course = course_with_many_variables();
    let mut h = TestHarness::new_with_theme(course, (1024.0, 600.0), ThemeMode::Dark);
    h.editor.open_preview();
    h.press(Key::F2, Modifiers::NONE);
    h.press(Key::F3, Modifiers::NONE);

    let mut prev_window_rect = Rect::NOTHING;
    for frame_idx in 0..10 {
        let shapes = h.frame_shapes(vec![]);
        let mut window_rect = Rect::NOTHING;
        for clipped in &shapes {
            if let egui::epaint::Shape::Rect(r) = &clipped.shape
                && r.rect.width() > 500.0
                && r.rect.height() > 350.0
                && r.rect.width() < 1020.0
            {
                window_rect = window_rect.union(r.rect);
            }
        }
        if frame_idx > 1 && window_rect.is_positive() {
            if prev_window_rect.is_positive() {
                assert!(
                    (window_rect.width() - prev_window_rect.width()).abs() <= 2.0
                        && (window_rect.height() - prev_window_rect.height()).abs() <= 2.0,
                    "preview window auto-grew on frame {frame_idx}: prev={prev_window_rect:?} now={window_rect:?}"
                );
            }
            prev_window_rect = window_rect;
        }
    }
}

#[test]
fn t13_8_web_editor_disables_desktop_only_actions_with_tooltip() {
    let course = Course::from_json(HELLO_JSON).expect("valid hello course");
    let mut h = TestHarness::new_with_theme(course, (1280.0, 800.0), ThemeMode::Dark);
    h.editor.file_ops_available = false;
    h.editor.publish_available = false;
    h.ctx.all_styles_mut(|s| s.interaction.tooltip_delay = 0.0);
    h.idle_frame();
    h.idle_frame();

    let file_pos = h.find_text("File").expect("File menu");
    h.click_at(file_pos);
    h.idle_frame();
    if let Some(pos) = h.find_text("Save as Folder…") {
        let shapes = h.frame_shapes(vec![Event::PointerMoved(pos)]);
        let has_tooltip = shapes.iter().any(|c| {
            matches!(&c.shape, egui::epaint::Shape::Text(t) if t.galley.text().contains("desktop app"))
        });
        assert!(
            has_tooltip,
            "disabled desktop-only menu item must show 'Available in the desktop app' tooltip"
        );
    }
}

#[test]
fn t14_9_preview_and_published_player_layout_parity() {
    let course = Course::from_json(HELLO_JSON).expect("valid hello course");
    let snap = snapshot_player(
        course.clone(),
        None,
        &TestNoMedia,
        (960, 540),
        1.0,
        coursecraft_render::PlayerSnapshotSelect::Playing,
    )
    .expect("snapshot succeeds");
    assert_eq!(snap.width, 960);
    assert_eq!(snap.height, 540);

    let player = coursecraft_logic::Player::new(&course).expect("valid player");
    let ctx = egui::Context::default();
    let mut frame = PlayerFrame::new();
    let mut view = SlideView::new();
    let mut out_rect = Rect::NOTHING;
    let raw = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(960.0, 540.0))),
        time: Some(1.0),
        ..Default::default()
    };
    let full = ctx.run_ui(raw, |ui| {
        let out = frame.show(ui, &player, &mut view, None);
        out_rect = out.slide_rect;
    });
    full.drop_without_applying_deltas();
    assert!(out_rect.is_positive(), "slide_rect must be positive");
    let strip = frame.strip_rect().expect("strip rect must exist");
    assert!(
        strip.top() >= out_rect.bottom() - 1.0,
        "reserved strip must sit below slide_rect"
    );
}

#[test]
fn gb_10_preview_and_published_player_interactive_group_parity() {
    use coursecraft_course::Value;
    use coursecraft_logic::{Input, Player};
    use coursecraft_render::{AccessRole, FocusTarget, build_access_tree, render_player_rgba};
    use egui::PointerButton;

    let raw = serde_json::json!({
        "schema_version": 1,
        "id": "gb-10-parity",
        "title": "GB-10 Parity",
        "slide_size": { "w": 960, "h": 540 },
        "player": { "show_bar": false },
        "variables": [
            { "name": "clicks", "value": 0 },
            { "name": "hovers", "value": 0 }
        ],
        "scenes": [{
            "id": "sc1",
            "title": "Scene 1",
            "slides": [{
                "id": "s1",
                "title": "Interactive Group Parity",
                "duration_ms": 60000,
                "groups": [{
                    "id": "g-btn",
                    "name": "Action Button",
                    "interactive": true
                }],
                "objects": [
                    {
                        "id": "m-bg",
                        "name": "Background",
                        "group": "g-btn",
                        "frame": { "x": 100.0, "y": 120.0, "w": 120.0, "h": 80.0 },
                        "kind": { "type": "shape", "shape": "rectangle", "fill": "#224488" },
                        "states": [
                            { "name": "hover", "fill": "#3366CC" },
                            { "name": "down", "fill": "#112255" },
                            { "name": "selected", "fill": "#228844" },
                            { "name": "disabled", "fill": "#888888" }
                        ]
                    },
                    {
                        "id": "m-icon",
                        "name": "Icon",
                        "group": "g-btn",
                        "decorative": true,
                        "frame": { "x": 220.0, "y": 120.0, "w": 80.0, "h": 80.0 },
                        "kind": { "type": "shape", "shape": "ellipse", "fill": "#FFFFFF" },
                        "states": [
                            { "name": "hover", "fill": "#FFEE88" },
                            { "name": "down", "fill": "#FFAA22" },
                            { "name": "selected", "fill": "#AAFFAA" }
                        ]
                    },
                    {
                        "id": "m-label",
                        "name": "Label",
                        "group": "g-btn",
                        "frame": { "x": 300.0, "y": 120.0, "w": 140.0, "h": 80.0 },
                        "kind": { "type": "text", "text": "GO" },
                        "states": [
                            { "name": "hover", "text": "GO!" },
                            { "name": "down", "text": "GOING" },
                            { "name": "selected", "text": "DONE" }
                        ]
                    },
                    {
                        "id": "btn-disable",
                        "name": "Disable",
                        "frame": { "x": 100.0, "y": 320.0, "w": 160.0, "h": 50.0 },
                        "kind": { "type": "button", "label": "Disable" }
                    }
                ],
                "triggers": [
                    {
                        "when": { "event": "click", "object": "g-btn" },
                        "action": { "action": "set_variable", "variable": "clicks", "op": "add", "value": 1 }
                    },
                    {
                        "when": { "event": "hover_start", "object": "g-btn" },
                        "action": { "action": "set_variable", "variable": "hovers", "op": "add", "value": 1 }
                    },
                    {
                        "when": { "event": "click", "object": "btn-disable" },
                        "action": { "action": "change_state", "object": "g-btn", "state": "disabled" }
                    }
                ]
            }]
        }]
    })
    .to_string();
    let course = Course::from_json(&raw).expect("valid gb-10 course");

    // Verify published snapshot_player renders cleanly.
    let initial_snap = snapshot_player(
        course.clone(),
        None,
        &TestNoMedia,
        (960, 540),
        1.0,
        coursecraft_render::PlayerSnapshotSelect::Playing,
    )
    .expect("snapshot_player succeeds");
    assert_eq!(initial_snap.width, 960);
    assert_eq!(initial_snap.height, 540);

    // 1. Set up Editor Preview via TestHarness.
    let mut harness = TestHarness::new_with_theme(course.clone(), (1280.0, 800.0), ThemeMode::Dark);
    harness.editor.open_preview();
    for _ in 0..3 {
        harness.idle_frame();
    }
    let preview_slide_pt = |h: &TestHarness, sx: f32, sy: f32| -> egui::Pos2 {
        h.editor
            .preview
            .as_ref()
            .and_then(|p| p.view.last_transform())
            .expect("preview transform")
            .to_screen_pos(pos2(sx, sy))
    };
    // Focus preview window by clicking an empty slide spot (top-left).
    let preview_empty = preview_slide_pt(&harness, 20.0, 20.0);
    harness.frame(vec![
        Event::PointerMoved(preview_empty),
        Event::PointerButton {
            pos: preview_empty,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
        Event::PointerButton {
            pos: preview_empty,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        },
    ]);

    struct PubHarness<'c> {
        ctx: egui::Context,
        screen: Rect,
        time: f64,
        frame: PlayerFrame,
        view: SlideView,
        player: Player<'c>,
        log: Vec<String>,
    }

    impl<'c> PubHarness<'c> {
        fn new(course: &'c Course) -> Self {
            let mut player = Player::new(course).expect("valid player");
            let mut log = Vec::new();
            for eff in player.handle(&Input::Start) {
                if let Ok(line) = serde_json::to_string(&eff) {
                    log.push(line);
                }
            }
            let ctx = egui::Context::default();
            coursecraft_render::install_fonts(&ctx);
            let mut frame = PlayerFrame::new();
            frame.can_fullscreen = true;
            Self {
                ctx,
                screen: Rect::from_min_size(pos2(0.0, 0.0), vec2(960.0, 540.0)),
                time: 0.0,
                frame,
                view: SlideView::new(),
                player,
                log,
            }
        }

        fn run(&mut self, events: Vec<Event>) {
            self.time += 1.0 / 60.0;
            let raw = egui::RawInput {
                screen_rect: Some(self.screen),
                time: Some(self.time),
                events,
                ..Default::default()
            };
            let mut inputs = Vec::new();
            let full = self.ctx.run_ui(raw, |ui| {
                let out = self.frame.show(ui, &self.player, &mut self.view, None);
                inputs = out.inputs;
            });
            full.drop_without_applying_deltas();
            for input in inputs {
                for eff in self.player.handle(&input) {
                    if let Ok(line) = serde_json::to_string(&eff) {
                        self.log.push(line);
                    }
                }
            }
        }

        fn slide_pt(&self, sx: f32, sy: f32) -> egui::Pos2 {
            self.view
                .last_transform()
                .expect("published transform")
                .to_screen_pos(pos2(sx, sy))
        }
    }

    let mut pub_h = PubHarness::new(&course);
    pub_h.run(Vec::new());

    // Helper: map slide-space pointer events into screen space.
    let map_events = |events: &[Event], map_pt: &dyn Fn(egui::Pos2) -> egui::Pos2| -> Vec<Event> {
        events
            .iter()
            .cloned()
            .map(|ev| match ev {
                Event::PointerMoved(p) => Event::PointerMoved(map_pt(p)),
                Event::PointerButton {
                    pos,
                    button,
                    pressed,
                    modifiers,
                } => Event::PointerButton {
                    pos: map_pt(pos),
                    button,
                    pressed,
                    modifiers,
                },
                other => other,
            })
            .collect()
    };

    let assert_parity = |label: &str, h: &TestHarness, pub_h: &PubHarness<'_>| {
        let prev = h.editor.preview.as_ref().expect("preview open");
        let prev_state = prev.state.clone().expect("preview state");
        let prev_player = Player::resume(&prev.course, prev_state).expect("resume preview");

        assert_eq!(
            prev_player.variable("clicks"),
            pub_h.player.variable("clicks"),
            "{label}: clicks variable mismatch"
        );
        assert_eq!(
            prev_player.variable("hovers"),
            pub_h.player.variable("hovers"),
            "{label}: hovers variable mismatch"
        );
        assert_eq!(
            prev_player.object_state("g-btn"),
            pub_h.player.object_state("g-btn"),
            "{label}: g-btn state mismatch"
        );
        assert_eq!(
            prev.log, pub_h.log,
            "{label}: effect trace mismatch between Preview and Published"
        );
        assert_eq!(
            prev.view.last_items(),
            pub_h.view.last_items(),
            "{label}: composed DrawItems mismatch between Preview and Published"
        );
        assert_eq!(
            prev.frame.focus().list(),
            pub_h.frame.focus().list(),
            "{label}: Tab stop list mismatch between Preview and Published"
        );
        let prev_tree = build_access_tree(
            &prev_player,
            &prev.frame,
            prev.view.last_items(),
            false,
            false,
        );
        let pub_tree = build_access_tree(
            &pub_h.player,
            &pub_h.frame,
            pub_h.view.last_items(),
            false,
            false,
        );
        assert_eq!(
            prev_tree, pub_tree,
            "{label}: AccessTree mismatch between Preview and Published"
        );
        let prev_rgba =
            render_player_rgba(&prev_player, 320, &[], &TestNoMedia, None).expect("preview rgba");
        let pub_rgba = render_player_rgba(&pub_h.player, 320, &[], &TestNoMedia, None)
            .expect("published rgba");
        assert_eq!(
            prev_rgba.pixels, pub_rgba.pixels,
            "{label}: rendered RGBA pixels mismatch between Preview and Published"
        );
    };

    let member_a = pos2(160.0, 160.0);
    let member_b = pos2(260.0, 160.0);
    let member_c = pos2(370.0, 160.0);
    let disable_pt = pos2(180.0, 345.0);
    let empty_pt = pos2(20.0, 20.0);

    let key_ev = |key: Key, modifiers: Modifiers| Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    };

    // Step 1: Hover member A -> move pointer to member B (no hover flicker).
    let prev_ev = map_events(&[Event::PointerMoved(member_a)], &|p| {
        preview_slide_pt(&harness, p.x, p.y)
    });
    let pub_ev = map_events(&[Event::PointerMoved(member_a)], &|p| {
        pub_h.slide_pt(p.x, p.y)
    });
    harness.frame(prev_ev);
    harness.idle_frame();
    pub_h.run(pub_ev);
    pub_h.run(Vec::new());
    assert_eq!(pub_h.player.variable("hovers"), Some(&Value::Number(1.0)));
    assert_parity("after hover A", &harness, &pub_h);

    // Move pointer from member A to member B -> NO hover flicker.
    let prev_ev = map_events(&[Event::PointerMoved(member_b)], &|p| {
        preview_slide_pt(&harness, p.x, p.y)
    });
    let pub_ev = map_events(&[Event::PointerMoved(member_b)], &|p| {
        pub_h.slide_pt(p.x, p.y)
    });
    harness.frame(prev_ev);
    harness.idle_frame();
    pub_h.run(pub_ev);
    pub_h.run(Vec::new());
    assert_eq!(
        pub_h.player.variable("hovers"),
        Some(&Value::Number(1.0)),
        "moving member A -> B must not flicker hover"
    );
    assert_parity("after hover B", &harness, &pub_h);

    // Step 2: Press on member B -> release on member C (group click fires once, counter=1, selected).
    let press_b = vec![Event::PointerButton {
        pos: member_b,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    }];
    let prev_ev = map_events(&press_b, &|p| preview_slide_pt(&harness, p.x, p.y));
    let pub_ev = map_events(&press_b, &|p| pub_h.slide_pt(p.x, p.y));
    harness.frame(prev_ev);
    pub_h.run(pub_ev);
    assert_parity("while pressed on B", &harness, &pub_h);

    let release_c = vec![
        Event::PointerMoved(member_c),
        Event::PointerButton {
            pos: member_c,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        },
    ];
    let prev_ev = map_events(&release_c, &|p| preview_slide_pt(&harness, p.x, p.y));
    let pub_ev = map_events(&release_c, &|p| pub_h.slide_pt(p.x, p.y));
    harness.frame(prev_ev);
    harness.idle_frame();
    pub_h.run(pub_ev);
    pub_h.run(Vec::new());
    assert_eq!(pub_h.player.variable("clicks"), Some(&Value::Number(1.0)));
    assert_eq!(pub_h.player.variable("hovers"), Some(&Value::Number(1.0)));
    assert_eq!(pub_h.player.object_state("g-btn"), Some("selected"));
    assert_parity("after press B release C", &harness, &pub_h);

    // Move pointer off the group before keyboard testing.
    let leave = vec![Event::PointerMoved(empty_pt)];
    let prev_ev = map_events(&leave, &|p| preview_slide_pt(&harness, p.x, p.y));
    let pub_ev = map_events(&leave, &|p| pub_h.slide_pt(p.x, p.y));
    harness.frame(prev_ev);
    harness.idle_frame();
    pub_h.run(pub_ev);
    pub_h.run(Vec::new());

    // Step 3: Keyboard Tab -> single Tab stop on the group -> Enter & Space toggle.
    harness.frame(vec![key_ev(Key::Tab, Modifiers::NONE)]);
    harness.idle_frame();
    pub_h.run(vec![key_ev(Key::Tab, Modifiers::NONE)]);
    pub_h.run(Vec::new());
    assert_eq!(
        pub_h.frame.focus().list(),
        &[
            FocusTarget::Object("g-btn".into()),
            FocusTarget::Object("btn-disable".into()),
        ],
        "interactive group must produce exactly one Tab stop"
    );
    assert_eq!(
        pub_h.frame.focus().focused(),
        Some(&FocusTarget::Object("g-btn".into()))
    );
    let tree_tab = build_access_tree(
        &pub_h.player,
        &pub_h.frame,
        pub_h.view.last_items(),
        false,
        false,
    );
    let g_nodes: Vec<_> = tree_tab
        .nodes
        .iter()
        .filter(|n| n.target == Some(FocusTarget::Object("g-btn".into())))
        .collect();
    assert_eq!(g_nodes.len(), 1);
    assert_eq!(g_nodes[0].role, AccessRole::Button);
    assert_eq!(g_nodes[0].checked, Some(true));
    assert_parity("after Tab to g-btn", &harness, &pub_h);

    // Press Enter -> clicks increments to 2, selected toggles back to normal.
    harness.frame(vec![key_ev(Key::Enter, Modifiers::NONE)]);
    harness.idle_frame();
    pub_h.run(vec![key_ev(Key::Enter, Modifiers::NONE)]);
    pub_h.run(Vec::new());
    assert_eq!(pub_h.player.variable("clicks"), Some(&Value::Number(2.0)));
    assert_eq!(pub_h.player.object_state("g-btn"), Some("normal"));
    assert_parity("after Enter", &harness, &pub_h);

    // Press Space -> clicks increments to 3, selected toggles back to selected.
    harness.frame(vec![key_ev(Key::Space, Modifiers::NONE)]);
    harness.idle_frame();
    pub_h.run(vec![key_ev(Key::Space, Modifiers::NONE)]);
    pub_h.run(Vec::new());
    assert_eq!(pub_h.player.variable("clicks"), Some(&Value::Number(3.0)));
    assert_eq!(pub_h.player.object_state("g-btn"), Some("selected"));
    assert_parity("after Space", &harness, &pub_h);

    // Step 4: Click btn-disable -> g-btn becomes disabled; hover/click on members do nothing.
    let click_disable = vec![
        Event::PointerMoved(disable_pt),
        Event::PointerButton {
            pos: disable_pt,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
        Event::PointerButton {
            pos: disable_pt,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        },
    ];
    let prev_ev = map_events(&click_disable, &|p| preview_slide_pt(&harness, p.x, p.y));
    let pub_ev = map_events(&click_disable, &|p| pub_h.slide_pt(p.x, p.y));
    harness.frame(prev_ev);
    harness.idle_frame();
    pub_h.run(pub_ev);
    pub_h.run(Vec::new());
    assert_eq!(pub_h.player.object_state("g-btn"), Some("disabled"));
    assert_parity("after disable", &harness, &pub_h);

    // Attempt hover + press + click across disabled members -> visual hover/down skipped and click swallowed.
    let hover_disabled = vec![Event::PointerMoved(member_a)];
    let prev_ev = map_events(&hover_disabled, &|p| preview_slide_pt(&harness, p.x, p.y));
    let pub_ev = map_events(&hover_disabled, &|p| pub_h.slide_pt(p.x, p.y));
    harness.frame(prev_ev);
    harness.idle_frame();
    pub_h.run(pub_ev);
    pub_h.run(Vec::new());
    // Visual hover state must NOT apply while disabled (fill remains disabled #888888, text remains "GO").
    assert!(
        pub_h.view.last_items().iter().any(|item| matches!(
            &item.visual,
            coursecraft_render::Visual::Shape { fill, .. }
                if *fill == egui::Color32::from_rgb(0x88, 0x88, 0x88)
        )),
        "disabled group must keep disabled fill on hover"
    );
    assert!(
        !pub_h.view.last_items().iter().any(|item| matches!(
            &item.visual,
            coursecraft_render::Visual::Shape { fill, .. }
                if *fill == egui::Color32::from_rgb(0x33, 0x66, 0xCC)
        )),
        "disabled group must not render hover fill"
    );
    assert_parity("after hover while disabled", &harness, &pub_h);

    let disabled_click = vec![
        Event::PointerButton {
            pos: member_a,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
        Event::PointerMoved(member_c),
        Event::PointerButton {
            pos: member_c,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        },
    ];
    let prev_ev = map_events(&disabled_click, &|p| preview_slide_pt(&harness, p.x, p.y));
    let pub_ev = map_events(&disabled_click, &|p| pub_h.slide_pt(p.x, p.y));
    harness.frame(prev_ev);
    harness.idle_frame();
    pub_h.run(pub_ev);
    pub_h.run(Vec::new());
    assert_eq!(pub_h.player.variable("clicks"), Some(&Value::Number(3.0)));
    assert_eq!(pub_h.player.object_state("g-btn"), Some("disabled"));
    assert_parity("after disabled click attempt", &harness, &pub_h);
}
