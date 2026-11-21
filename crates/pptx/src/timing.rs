//! OOXML `<p:timing>` parser and timeline scheduler for PowerPoint animations.
//!
//! Translates PowerPoint entrance, exit, and emphasis timing trees into
//! CourseCraft [`Animation`]s, [`ObjectTiming`] bounds, and `SkipKind::Animation`
//! diagnostics. Motion paths (`p:animMotion`) and interactive sequences
//! (`interactiveSeq`) are wired in Step C-3.

use std::collections::{HashMap, HashSet};

use coursecraft_course::{AnimEffect, Animation, Direction, Easing, Object};

use crate::{SlideTracker, XmlNode};

/// Maximum number of timing nodes visited per slide to bound hostile inputs.
const MAX_ANIM_NODES: usize = 1000;

/// Maximum `<p:par>` nesting depth when walking `<p:timing>`.
const MAX_TIMING_DEPTH: usize = 64;

/// Stagger gap (ms) inserted between on-click animation steps.
const CLICK_STEP_GAP_MS: u32 = 500;

/// Default animation duration (ms) when omitted or unparseable.
const DEFAULT_DURATION_MS: u32 = 500;

/// Maximum valid animation duration (ms) in `coursecraft-course`.
const MAX_DURATION_MS: u32 = 60_000;

/// Maximum valid delay (ms) clamped for hostile inputs.
const MAX_DELAY_MS: u32 = 86_400_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StepKind {
    OnClick,
    WithPrevious,
    AfterPrevious,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PresetClass {
    Entrance,
    Exit,
    Emphasis,
}

#[derive(Debug, Clone)]
struct ParsedEffect<'a> {
    preset_class: PresetClass,
    preset_id: u32,
    preset_subtype: u32,
    step_kind: StepKind,
    delay_ms: u32,
    duration_ms: u32,
    easing: Easing,
    spid: Option<String>,
    paragraph_level: bool,
    ctn_node: &'a XmlNode,
}

#[derive(Debug, Default)]
struct TimingWalker {
    visited: usize,
    node_limit_reported: bool,
    depth_limit_reported: bool,
}

impl TimingWalker {
    fn visit_node(&mut self, tracker: &mut SlideTracker) -> bool {
        if self.visited >= MAX_ANIM_NODES {
            if !self.node_limit_reported {
                self.node_limit_reported = true;
                tracker.report_animation(format!(
                    "Animation timing node count exceeded {MAX_ANIM_NODES}; remaining animations skipped"
                ));
            }
            return false;
        }
        self.visited = self.visited.saturating_add(1);
        true
    }

    fn check_depth(&mut self, depth: usize, tracker: &mut SlideTracker) -> bool {
        if depth > MAX_TIMING_DEPTH {
            if !self.depth_limit_reported {
                self.depth_limit_reported = true;
                tracker.report_animation(format!(
                    "Animation timing nesting exceeded depth {MAX_TIMING_DEPTH}; branch skipped"
                ));
            }
            return false;
        }
        true
    }
}

/// Parse `<p:timing>` on a slide and apply entrance/exit timings and emphasis
/// diagnostics to `objects`. Returns the maximum effect end timestamp (ms)
/// observed across all scheduled effects so the caller can extend slide duration.
pub(crate) fn apply_slide_timing(
    timing_node: &XmlNode,
    objects: &mut [Object],
    tracker: &mut SlideTracker,
) -> u32 {
    let Some(tn_lst) = timing_node.child("tnLst") else {
        return 0;
    };

    let mut walker = TimingWalker::default();
    let mut main_effects: Vec<ParsedEffect<'_>> = Vec::new();

    for root_par in tn_lst.children_named("par") {
        if !walker.visit_node(tracker) {
            break;
        }
        let Some(root_ctn) = root_par.child("cTn") else {
            continue;
        };
        let Some(root_children) = root_ctn.child("childTnLst") else {
            continue;
        };
        for seq in root_children.children_named("seq") {
            if !walker.visit_node(tracker) {
                break;
            }
            let Some(seq_ctn) = seq.child("cTn") else {
                continue;
            };
            let node_type = seq_ctn.attr("nodeType").unwrap_or("");
            if node_type == "mainSeq"
                && let Some(seq_children) = seq_ctn.child("childTnLst")
            {
                for step_par in seq_children.children_named("par") {
                    collect_effects_from_par(
                        step_par,
                        0,
                        0,
                        None,
                        &mut walker,
                        tracker,
                        &mut main_effects,
                    );
                }
            }
        }
    }

    schedule_main_sequence(&main_effects, objects, tracker)
}

fn collect_effects_from_par<'a>(
    par: &'a XmlNode,
    depth: usize,
    inherited_delay_ms: u32,
    inherited_step: Option<StepKind>,
    walker: &mut TimingWalker,
    tracker: &mut SlideTracker,
    out: &mut Vec<ParsedEffect<'a>>,
) {
    if !walker.check_depth(depth, tracker) || !walker.visit_node(tracker) {
        return;
    }

    let Some(ctn) = par.child("cTn") else {
        return;
    };

    let own_delay = parse_ctn_delay_ms(ctn);
    let total_delay = inherited_delay_ms
        .saturating_add(own_delay)
        .min(MAX_DELAY_MS);
    let own_step = parse_step_kind(ctn.attr("nodeType")).or(inherited_step);

    if let Some(preset_class) = ctn.attr("presetClass").and_then(parse_preset_class) {
        let preset_id = ctn
            .attr("presetID")
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(0);
        let preset_subtype = ctn
            .attr("presetSubtype")
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(0);
        let step_kind = own_step.unwrap_or(StepKind::WithPrevious);
        let duration_ms = parse_effect_duration_ms(ctn);
        let easing = parse_effect_easing(ctn);
        let (spid, paragraph_level) = find_effect_target(ctn);

        out.push(ParsedEffect {
            preset_class,
            preset_id,
            preset_subtype,
            step_kind,
            delay_ms: total_delay,
            duration_ms,
            easing,
            spid,
            paragraph_level,
            ctn_node: ctn,
        });
        return;
    }

    if let Some(children) = ctn.child("childTnLst") {
        for child_par in children.children_named("par") {
            collect_effects_from_par(
                child_par,
                depth.saturating_add(1),
                total_delay,
                own_step,
                walker,
                tracker,
                out,
            );
        }
    }
}

fn schedule_main_sequence(
    effects: &[ParsedEffect<'_>],
    objects: &mut [Object],
    tracker: &mut SlideTracker,
) -> u32 {
    let mut id_to_index: HashMap<String, usize> = HashMap::with_capacity(objects.len());
    for (idx, obj) in objects.iter().enumerate() {
        id_to_index.insert(obj.id.clone(), idx);
    }

    let mut had_entrance: HashSet<usize> = HashSet::new();
    let mut had_exit: HashSet<usize> = HashSet::new();

    let mut last_group_start_ms: u32 = 0;
    let mut last_end_ms: u32 = 0;
    let mut max_effect_end_ms: u32 = 0;
    let mut emitted_click_note = false;

    for eff in effects {
        // Keep `ctn_node` referenced cleanly for Step C-3 (`repeatCount` / `animMotion`).
        let _ = eff.ctn_node;

        let start_ms = match eff.step_kind {
            StepKind::WithPrevious => last_group_start_ms.saturating_add(eff.delay_ms),
            StepKind::AfterPrevious => {
                let s = last_end_ms.saturating_add(eff.delay_ms);
                last_group_start_ms = s;
                s
            }
            StepKind::OnClick => {
                if !emitted_click_note {
                    emitted_click_note = true;
                    tracker.report_animation(
                        "On-click animation steps converted to staggered timeline timings (500 ms step)",
                    );
                }
                let s = last_end_ms
                    .saturating_add(CLICK_STEP_GAP_MS)
                    .saturating_add(eff.delay_ms);
                last_group_start_ms = s;
                s
            }
        };

        let end_ms = start_ms.saturating_add(eff.duration_ms);
        last_end_ms = match eff.step_kind {
            StepKind::WithPrevious => last_end_ms.max(end_ms),
            StepKind::AfterPrevious | StepKind::OnClick => end_ms,
        };
        max_effect_end_ms = max_effect_end_ms.max(end_ms);

        let Some(spid) = eff.spid.as_deref() else {
            tracker.report_animation("Animation missing target spid; skipped");
            continue;
        };
        let Some(target_obj_ids) = tracker.pptx_ids.get(spid).cloned() else {
            tracker.report_animation(format!("Animation target spid {spid} not found on slide"));
            continue;
        };

        for obj_id in target_obj_ids {
            let Some(&obj_idx) = id_to_index.get(&obj_id) else {
                continue;
            };
            let obj_name = objects[obj_idx].name.clone();

            if eff.paragraph_level {
                tracker.report_animation(format!(
                    "Paragraph-level animation applied to whole text box on {obj_name}"
                ));
            }

            match eff.preset_class {
                PresetClass::Emphasis => {
                    let preset_name = emphasis_preset_name(eff.preset_id);
                    tracker.report_animation(format!(
                        "Emphasis animation ({preset_name}, presetID={}) skipped on {obj_name}",
                        eff.preset_id
                    ));
                }
                PresetClass::Entrance => {
                    if !had_entrance.insert(obj_idx) {
                        tracker.report_animation(format!(
                            "Duplicate entrance animation skipped on {obj_name}"
                        ));
                        continue;
                    }
                    let mapped = map_entrance_exit_preset(
                        eff.preset_id,
                        eff.preset_subtype,
                        &obj_name,
                        tracker,
                    );
                    let obj = &mut objects[obj_idx];
                    obj.timing.start_ms = start_ms;
                    obj.entrance = mapped.map(|effect| Animation {
                        effect,
                        duration_ms: eff.duration_ms,
                        easing: eff.easing,
                    });
                }
                PresetClass::Exit => {
                    if !had_exit.insert(obj_idx) {
                        tracker.report_animation(format!(
                            "Earlier exit animation replaced on {obj_name}"
                        ));
                    }
                    let mapped = map_entrance_exit_preset(
                        eff.preset_id,
                        eff.preset_subtype,
                        &obj_name,
                        tracker,
                    );
                    let obj = &mut objects[obj_idx];
                    let safe_end = if end_ms <= obj.timing.start_ms {
                        obj.timing.start_ms.saturating_add(eff.duration_ms)
                    } else {
                        end_ms
                    };
                    max_effect_end_ms = max_effect_end_ms.max(safe_end);
                    obj.timing.end_ms = Some(safe_end);
                    obj.exit = mapped.map(|effect| Animation {
                        effect,
                        duration_ms: eff.duration_ms,
                        easing: eff.easing,
                    });
                }
            }
        }
    }

    max_effect_end_ms
}

fn parse_preset_class(raw: &str) -> Option<PresetClass> {
    match raw {
        "entr" => Some(PresetClass::Entrance),
        "exit" => Some(PresetClass::Exit),
        "emph" => Some(PresetClass::Emphasis),
        _ => None,
    }
}

fn parse_step_kind(raw: Option<&str>) -> Option<StepKind> {
    match raw? {
        "clickEffect" => Some(StepKind::OnClick),
        "withEffect" => Some(StepKind::WithPrevious),
        "afterEffect" => Some(StepKind::AfterPrevious),
        _ => None,
    }
}

fn parse_ctn_delay_ms(ctn: &XmlNode) -> u32 {
    let Some(st_cond) = ctn.child("stCondLst") else {
        return 0;
    };
    let mut sum: u32 = 0;
    for cond in st_cond.children_named("cond") {
        if let Some(delay_str) = cond.attr("delay") {
            let trimmed = delay_str.trim();
            if trimmed.eq_ignore_ascii_case("indefinite") {
                continue;
            }
            if let Ok(val) = trimmed.parse::<i64>()
                && val > 0
            {
                let clamped =
                    u32::try_from(val.min(i64::from(MAX_DELAY_MS))).unwrap_or(MAX_DELAY_MS);
                sum = sum.saturating_add(clamped).min(MAX_DELAY_MS);
            }
        }
    }
    sum
}

fn parse_ms_attr(raw: Option<&str>) -> Option<u32> {
    let trimmed = raw?.trim();
    if trimmed.eq_ignore_ascii_case("indefinite") {
        return None;
    }
    let val = trimmed.parse::<i64>().ok()?;
    if val <= 0 {
        return None;
    }
    Some(u32::try_from(val.min(i64::from(MAX_DURATION_MS))).unwrap_or(MAX_DURATION_MS))
}

fn parse_effect_duration_ms(ctn: &XmlNode) -> u32 {
    if let Some(dur) = parse_ms_attr(ctn.attr("dur"))
        && dur > 1
    {
        return dur.clamp(1, MAX_DURATION_MS);
    }

    let mut max_child_dur: Option<u32> = None;
    if let Some(children) = ctn.child("childTnLst") {
        for behavior in &children.children {
            let is_visibility_set = behavior.name == "set";
            if let Some(cbhvr) = behavior.child("cBhvr")
                && let Some(b_ctn) = cbhvr.child("cTn")
                && let Some(dur) = parse_ms_attr(b_ctn.attr("dur"))
            {
                if is_visibility_set && dur <= 1 {
                    continue;
                }
                max_child_dur = Some(max_child_dur.map_or(dur, |prev| prev.max(dur)));
            }
        }
    }

    max_child_dur
        .unwrap_or(DEFAULT_DURATION_MS)
        .clamp(1, MAX_DURATION_MS)
}

fn parse_effect_easing(ctn: &XmlNode) -> Easing {
    let mut saw_attr = false;
    let mut has_accel = false;
    let mut has_decel = false;

    if let Some(children) = ctn.child("childTnLst") {
        for behavior in &children.children {
            if let Some(cbhvr) = behavior.child("cBhvr")
                && let Some(b_ctn) = cbhvr.child("cTn")
            {
                if let Some(accel_str) = b_ctn.attr("accel")
                    && let Ok(v) = accel_str.trim().parse::<u32>()
                {
                    saw_attr = true;
                    if v > 0 {
                        has_accel = true;
                    }
                }
                if let Some(decel_str) = b_ctn.attr("decel")
                    && let Ok(v) = decel_str.trim().parse::<u32>()
                {
                    saw_attr = true;
                    if v > 0 {
                        has_decel = true;
                    }
                }
            }
        }
    }

    if !saw_attr {
        return Easing::EaseOut;
    }
    match (has_accel, has_decel) {
        (true, true) => Easing::EaseInOut,
        (true, false) => Easing::EaseIn,
        (false, true) => Easing::EaseOut,
        (false, false) => Easing::Linear,
    }
}

fn find_effect_target(ctn: &XmlNode) -> (Option<String>, bool) {
    let Some(children) = ctn.child("childTnLst") else {
        return (None, false);
    };
    for behavior in &children.children {
        if let Some(cbhvr) = behavior.child("cBhvr")
            && let Some(tgt_el) = cbhvr.child("tgtEl")
            && let Some(sp_tgt) = tgt_el.child("spTgt")
            && let Some(spid) = sp_tgt.attr("spid")
        {
            let paragraph_level = sp_tgt
                .child("txEl")
                .is_some_and(|tx| tx.child("pRg").is_some());
            return (Some(spid.to_owned()), paragraph_level);
        }
    }
    (None, false)
}

fn map_direction(subtype: u32, obj_name: &str, tracker: &mut SlideTracker) -> Direction {
    match subtype {
        1 => Direction::Top,
        2 => Direction::Right,
        4 | 0 => Direction::Bottom,
        8 => Direction::Left,
        3 | 9 => {
            tracker.report_animation(format!(
                "Diagonal animation direction mapped to cardinal on {obj_name}"
            ));
            Direction::Top
        }
        6 | 12 => {
            tracker.report_animation(format!(
                "Diagonal animation direction mapped to cardinal on {obj_name}"
            ));
            Direction::Bottom
        }
        _ => Direction::Bottom,
    }
}

fn map_entrance_exit_preset(
    preset_id: u32,
    preset_subtype: u32,
    obj_name: &str,
    tracker: &mut SlideTracker,
) -> Option<AnimEffect> {
    match preset_id {
        1 => None,
        2 => {
            let from = map_direction(preset_subtype, obj_name, tracker);
            Some(AnimEffect::FlyIn { from })
        }
        10 => Some(AnimEffect::Fade),
        22 => {
            let from = map_direction(preset_subtype, obj_name, tracker);
            Some(AnimEffect::Wipe { from })
        }
        23 => Some(AnimEffect::Zoom),
        42 => {
            let from = match preset_subtype {
                1 | 2 | 4 | 8 => map_direction(preset_subtype, obj_name, tracker),
                _ => Direction::Bottom,
            };
            Some(AnimEffect::Float { from })
        }
        47 => Some(AnimEffect::Float {
            from: Direction::Top,
        }),
        12 => {
            let from = map_direction(preset_subtype, obj_name, tracker);
            tracker.report_animation(format!(
                "Peek preset (id=12) approximated as Wipe on {obj_name}"
            ));
            Some(AnimEffect::Wipe { from })
        }
        53 => {
            tracker.report_animation(format!(
                "Grow & Turn preset (id=53) approximated as Zoom on {obj_name}"
            ));
            Some(AnimEffect::Zoom)
        }
        16 => {
            tracker.report_animation(format!(
                "Animation preset Split (id=16) approximated as Fade on {obj_name}"
            ));
            Some(AnimEffect::Fade)
        }
        other => {
            tracker.report_animation(format!(
                "Unsupported animation preset (id={other}) approximated as Fade on {obj_name}"
            ));
            Some(AnimEffect::Fade)
        }
    }
}

fn emphasis_preset_name(preset_id: u32) -> &'static str {
    match preset_id {
        1 => "Fill Color",
        2 => "Font",
        3 => "Font Color",
        4 => "Font Size",
        5 => "Font Style",
        6 => "Grow/Shrink",
        7 => "Line Color",
        8 => "Spin",
        9 => "Transparency",
        10 => "Bold Flash",
        14 => "Pulse",
        15 => "Color Pulse",
        26 => "Teeter",
        27 => "Flicker",
        32 => "Wave",
        33 => "Blink",
        _ => "Emphasis",
    }
}

#[cfg(test)]
mod tests {
    use coursecraft_course::{Color, Object, ObjectKind, Rect, ShapeKind, Timing};

    use super::*;
    use crate::{SkipKind, parse_xml_tree};

    fn dummy_object(id: &str, name: &str) -> Object {
        Object {
            id: id.to_owned(),
            name: name.to_owned(),
            kind: ObjectKind::Shape {
                shape: ShapeKind::Rectangle,
                fill: Color("#112233".to_owned()),
                fill_style: None,
                outline: None,
                text: None,
                adjust: Vec::new(),
            },
            frame: Rect {
                x: 100.0,
                y: 100.0,
                w: 200.0,
                h: 100.0,
            },
            timing: Timing::default(),
            states: Vec::new(),
            initial_state: "normal".to_owned(),
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
    fn maps_entrance_exit_emphasis_and_staggered_timeline() {
        let xml = r#"
        <p:timing xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
          <p:tnLst>
            <p:par><p:cTn id="1" nodeType="tmRoot"><p:childTnLst>
              <p:seq><p:cTn id="2" nodeType="mainSeq"><p:childTnLst>
                <p:par><p:cTn id="3"><p:childTnLst>
                  <p:par><p:cTn id="4" presetID="10" presetClass="entr" presetSubtype="0" nodeType="withEffect" dur="500">
                    <p:stCondLst><p:cond delay="0"/></p:stCondLst>
                    <p:childTnLst><p:set><p:cBhvr><p:cTn id="5" dur="1"/><p:tgtEl><p:spTgt spid="2"/></p:tgtEl></p:cBhvr></p:set></p:childTnLst>
                  </p:cTn></p:par>
                  <p:par><p:cTn id="6" presetID="2" presetClass="entr" presetSubtype="8" nodeType="withEffect" dur="400">
                    <p:stCondLst><p:cond delay="200"/></p:stCondLst>
                    <p:childTnLst><p:animEffect><p:cBhvr><p:cTn id="7" dur="400" accel="50000" decel="0"/><p:tgtEl><p:spTgt spid="3"/></p:tgtEl></p:cBhvr></p:animEffect></p:childTnLst>
                  </p:cTn></p:par>
                  <p:par><p:cTn id="8" presetID="8" presetClass="emph" presetSubtype="0" nodeType="afterEffect" dur="600">
                    <p:stCondLst><p:cond delay="100"/></p:stCondLst>
                    <p:childTnLst><p:animRot><p:cBhvr><p:cTn id="9" dur="600"/><p:tgtEl><p:spTgt spid="2"/></p:tgtEl></p:cBhvr></p:animRot></p:childTnLst>
                  </p:cTn></p:par>
                  <p:par><p:cTn id="10" presetID="22" presetClass="exit" presetSubtype="2" nodeType="clickEffect" dur="500">
                    <p:stCondLst><p:cond delay="0"/></p:stCondLst>
                    <p:childTnLst><p:animEffect><p:cBhvr><p:cTn id="11" dur="500" accel="50000" decel="50000"/><p:tgtEl><p:spTgt spid="2"/></p:tgtEl></p:cBhvr></p:animEffect></p:childTnLst>
                  </p:cTn></p:par>
                </p:childTnLst></p:cTn></p:par>
              </p:childTnLst></p:cTn></p:seq>
            </p:childTnLst></p:cTn></p:par>
          </p:tnLst>
        </p:timing>"#;

        let timing_tree = parse_xml_tree(xml, "slide1.xml", 64).expect("valid xml");
        let mut objects = vec![
            dummy_object("obj_a", "Box A"),
            dummy_object("obj_b", "Box B"),
        ];
        let mut tracker = SlideTracker::new(1);
        tracker
            .pptx_ids
            .insert("2".to_owned(), vec!["obj_a".to_owned()]);
        tracker
            .pptx_ids
            .insert("3".to_owned(), vec!["obj_b".to_owned()]);

        let max_end = apply_slide_timing(&timing_tree, &mut objects, &mut tracker);

        assert_eq!(objects[0].timing.start_ms, 0);
        assert_eq!(
            objects[0].entrance,
            Some(Animation {
                effect: AnimEffect::Fade,
                duration_ms: 500,
                easing: Easing::EaseOut,
            })
        );
        assert_eq!(objects[1].timing.start_ms, 200);
        assert_eq!(
            objects[1].entrance,
            Some(Animation {
                effect: AnimEffect::FlyIn {
                    from: Direction::Left
                },
                duration_ms: 400,
                easing: Easing::EaseIn,
            })
        );
        // Emphasis at 600 + 100 = 700..1300; clickEffect at 1300 + 500 = 1800..2300.
        assert_eq!(objects[0].timing.end_ms, Some(2300));
        assert_eq!(
            objects[0].exit,
            Some(Animation {
                effect: AnimEffect::Wipe {
                    from: Direction::Right
                },
                duration_ms: 500,
                easing: Easing::EaseInOut,
            })
        );
        assert_eq!(max_end, 2300);

        let anim_notes: Vec<&str> = tracker
            .items
            .iter()
            .filter(|i| i.kind == SkipKind::Animation)
            .map(|i| i.note.as_str())
            .collect();
        assert!(anim_notes.iter().any(|n| n.contains("Spin")));
        assert!(anim_notes.iter().any(|n| n.contains("On-click")));
    }

    #[test]
    fn group_spid_applies_animation_to_all_flattened_children() {
        let xml = r#"
        <p:timing xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
          <p:tnLst>
            <p:par><p:cTn id="1" nodeType="tmRoot"><p:childTnLst>
              <p:seq><p:cTn id="2" nodeType="mainSeq"><p:childTnLst>
                <p:par><p:cTn id="3" presetID="10" presetClass="entr" nodeType="withEffect" dur="500">
                  <p:childTnLst><p:set><p:cBhvr><p:tgtEl><p:spTgt spid="100"/></p:tgtEl></p:cBhvr></p:set></p:childTnLst>
                </p:cTn></p:par>
              </p:childTnLst></p:cTn></p:seq>
            </p:childTnLst></p:cTn></p:par>
          </p:tnLst>
        </p:timing>"#;

        let timing_tree = parse_xml_tree(xml, "slide1.xml", 64).expect("valid xml");
        let mut objects = vec![
            dummy_object("c1", "Child 1"),
            dummy_object("c2", "Child 2"),
            dummy_object("c3", "Child 3"),
        ];
        let mut tracker = SlideTracker::new(1);
        tracker.pptx_ids.insert(
            "100".to_owned(),
            vec!["c1".to_owned(), "c2".to_owned(), "c3".to_owned()],
        );

        apply_slide_timing(&timing_tree, &mut objects, &mut tracker);
        for obj in &objects {
            assert_eq!(
                obj.entrance,
                Some(Animation {
                    effect: AnimEffect::Fade,
                    duration_ms: 500,
                    easing: Easing::EaseOut,
                })
            );
        }
    }

    #[test]
    fn appear_disappear_and_exit_before_start_clamping() {
        let xml = r#"
        <p:timing xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
          <p:tnLst>
            <p:par><p:cTn id="1" nodeType="tmRoot"><p:childTnLst>
              <p:seq><p:cTn id="2" nodeType="mainSeq"><p:childTnLst>
                <p:par><p:cTn id="3"><p:childTnLst>
                  <p:par><p:cTn id="4" presetID="1" presetClass="entr" nodeType="withEffect" dur="1">
                    <p:stCondLst><p:cond delay="1000"/></p:stCondLst>
                    <p:childTnLst><p:set><p:cBhvr><p:tgtEl><p:spTgt spid="2"/></p:tgtEl></p:cBhvr></p:set></p:childTnLst>
                  </p:cTn></p:par>
                  <p:par><p:cTn id="5" presetID="1" presetClass="exit" nodeType="withEffect" dur="200">
                    <p:stCondLst><p:cond delay="100"/></p:stCondLst>
                    <p:childTnLst><p:set><p:cBhvr><p:tgtEl><p:spTgt spid="2"/></p:tgtEl></p:cBhvr></p:set></p:childTnLst>
                  </p:cTn></p:par>
                  <p:par><p:cTn id="6" presetID="26" presetClass="entr" presetSubtype="6" nodeType="withEffect" dur="999999999">
                    <p:stCondLst><p:cond delay="-500"/></p:stCondLst>
                    <p:childTnLst><p:animEffect><p:cBhvr><p:tgtEl><p:spTgt spid="3"/></p:tgtEl></p:cBhvr></p:animEffect></p:childTnLst>
                  </p:cTn></p:par>
                </p:childTnLst></p:cTn></p:par>
              </p:childTnLst></p:cTn></p:seq>
            </p:childTnLst></p:cTn></p:par>
          </p:tnLst>
        </p:timing>"#;

        let timing_tree = parse_xml_tree(xml, "slide1.xml", 64).expect("valid xml");
        let mut objects = vec![dummy_object("o1", "One"), dummy_object("o2", "Two")];
        let mut tracker = SlideTracker::new(1);
        tracker
            .pptx_ids
            .insert("2".to_owned(), vec!["o1".to_owned()]);
        tracker
            .pptx_ids
            .insert("3".to_owned(), vec!["o2".to_owned()]);

        apply_slide_timing(&timing_tree, &mut objects, &mut tracker);

        assert_eq!(objects[0].timing.start_ms, 1000);
        assert_eq!(objects[0].entrance, None);
        assert_eq!(objects[0].exit, None);
        // Exit delay 100 + dur 500 = 600 <= start_ms 1000, so clamped > start_ms.
        assert!(objects[0].timing.end_ms.is_some_and(|e| e > 1000));

        assert_eq!(objects[1].timing.start_ms, 0);
        assert_eq!(
            objects[1].entrance,
            Some(Animation {
                effect: AnimEffect::Fade,
                duration_ms: 60_000,
                easing: Easing::EaseOut,
            })
        );
    }

    #[test]
    fn pptx_ids_records_group_children_and_connectors() {
        let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
               xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
          <p:cSld>
            <p:spTree>
              <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
              <p:grpSpPr/>
              <p:grpSp>
                <p:nvGrpSpPr><p:cNvPr id="10" name="Group 1"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
                <p:grpSpPr/>
                <p:sp>
                  <p:nvSpPr><p:cNvPr id="11" name="S1"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
                  <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="914400"/></a:xfrm><a:prstGeom prst="rect"/></p:spPr>
                </p:sp>
                <p:sp>
                  <p:nvSpPr><p:cNvPr id="12" name="S2"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
                  <p:spPr><a:xfrm><a:off x="914400" y="0"/><a:ext cx="914400" cy="914400"/></a:xfrm><a:prstGeom prst="rect"/></p:spPr>
                </p:sp>
              </p:grpSp>
              <p:cxnSp>
                <p:nvCxnSpPr><p:cNvPr id="20" name="Conn 1"/><p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr>
                <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="0"/></a:xfrm><a:prstGeom prst="line"/></p:spPr>
              </p:cxnSp>
            </p:spTree>
          </p:cSld>
        </p:sld>"#;

        let tree = parse_xml_tree(slide_xml, "slide1.xml", 64).expect("valid xml");
        let sp_tree = tree.find_descendant("spTree").expect("spTree");
        let mut objects = Vec::new();
        let mut tracker = SlideTracker::new(1);
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let writer = zip::ZipWriter::new(&mut buf);
            writer.finish().expect("finish empty zip");
        }
        let zip_bytes = buf.into_inner();
        let archive = zip::ZipArchive::new(std::io::Cursor::new(zip_bytes.as_slice()))
            .expect("open empty zip");
        let limits = crate::Limits::default();
        let entries = crate::ZipEntries::new(archive, HashSet::new(), limits);
        let layout = crate::SlideLayout::from_emu(12_192_000.0, 6_858_000.0);
        let rels = HashMap::new();
        let mut slide_title = None;
        let mut imported_media = Vec::new();
        let mut existing_media_names = HashSet::new();
        let mut resolver = crate::PlaceholderResolver::new(limits.max_xml_depth);
        let slide_parts = HashMap::new();
        let theme = crate::ThemePalette::default();
        let table_styles = HashMap::new();
        let mut cx = crate::SlideContext {
            layout: &layout,
            slide_rels: &rels,
            entries: &entries,
            objects: &mut objects,
            slide_title: &mut slide_title,
            imported_media: &mut imported_media,
            existing_media_names: &mut existing_media_names,
            tracker: &mut tracker,
            limits: &limits,
            resolver: &mut resolver,
            layout_path: None,
            slide_parts: &slide_parts,
            theme: &theme,
            table_styles: &table_styles,
        };
        crate::process_group_children(sp_tree, crate::GroupTransform::default(), 0, &mut cx)
            .expect("group children processed");

        assert_eq!(objects.len(), 3);
        let grp_ids = tracker.pptx_ids.get("10").expect("group id mapped");
        assert_eq!(grp_ids.len(), 2);
        assert_eq!(tracker.pptx_ids.get("11").map(Vec::len), Some(1));
        assert_eq!(tracker.pptx_ids.get("12").map(Vec::len), Some(1));
        assert_eq!(tracker.pptx_ids.get("20").map(Vec::len), Some(1));
    }
}
