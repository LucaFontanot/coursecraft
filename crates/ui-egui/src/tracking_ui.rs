//! Shared LMS completion tracking controls for Course Settings, Publish, and Course Inspector.

use coursecraft_course::{Course, QuizTracking, SlidesViewedTracking};
use coursecraft_render::{SIZE_SMALL, SPACE_SM, palette};
use egui::RichText;

use crate::Editor;

/// Draw the LMS completion tracking controls (Slides viewed, Quiz result, and active triggers note).
///
/// Dispatches `course.set_tracking` immediately on any change so each edit is a single undo step.
pub fn show_tracking_controls(
    editor: &mut Editor,
    ui: &mut egui::Ui,
    enabled: bool,
    id_prefix: &str,
) {
    let tracking = editor.session.course.tracking.clone();
    let total = editor.session.course.play_slide_count();
    let results_slides: Vec<(String, String)> = editor
        .session
        .course
        .results_slides()
        .map(|s| (s.id.clone(), s.title.clone()))
        .collect();
    let has_graded = editor.session.course.has_graded_questions();
    let passing = editor.session.course.quiz.map(|q| q.passing_percent);

    let mut next_tracking = tracking.clone();
    let p = palette(ui.ctx());

    let resp = ui
        .add_enabled_ui(enabled, |ui| {
            ui.label(
                RichText::new(tl!(
                    "Report completion to the LMS when any selected option is met."
                ))
                .size(SIZE_SMALL)
                .color(p.text_secondary),
            );
            ui.add_space(SPACE_SM);

            // Slides viewed
            let mut slides_enabled = next_tracking.slides_viewed.is_some();
            if ui
                .checkbox(&mut slides_enabled, tl!("Slides viewed"))
                .changed()
            {
                next_tracking.slides_viewed = if slides_enabled {
                    Some(SlidesViewedTracking::All)
                } else {
                    None
                };
            }
            if let Some(sv) = next_tracking.slides_viewed {
                ui.indent((id_prefix, "slides_viewed"), |ui| {
                    let is_all = matches!(sv, SlidesViewedTracking::All);
                    let all_label = craft_i18n::fmt(
                        tl!("All slides ({total})"),
                        &[("total", &total.to_string())],
                    );
                    if ui.radio(is_all, all_label).clicked() {
                        next_tracking.slides_viewed = Some(SlidesViewedTracking::All);
                    }

                    let total_u32 = u32::try_from(total).unwrap_or(u32::MAX);
                    let default_count = total_u32.max(1);
                    let mut count = match sv {
                        SlidesViewedTracking::AtLeast { count } => count.max(1),
                        SlidesViewedTracking::All => default_count,
                    };
                    let is_at_least = matches!(sv, SlidesViewedTracking::AtLeast { .. });
                    ui.horizontal_wrapped(|ui| {
                        if ui.radio(is_at_least, tl!("At least")).clicked() {
                            next_tracking.slides_viewed =
                                Some(SlidesViewedTracking::AtLeast { count });
                        }
                        let max_val = total_u32.max(count).max(1);
                        let dv = ui.add(
                            egui::DragValue::new(&mut count)
                                .range(1..=max_val)
                                .speed(0.2),
                        );
                        if dv.changed() {
                            next_tracking.slides_viewed = Some(SlidesViewedTracking::AtLeast {
                                count: count.max(1),
                            });
                        }
                        ui.label(tl!("slides"));
                    });

                    if let Some(SlidesViewedTracking::AtLeast { count }) =
                        next_tracking.slides_viewed
                        && count > total_u32
                        && total > 0
                    {
                        let msg = craft_i18n::fmt(
                            tl!("Exceeds course slide count ({total}); completes when all slides are viewed."),
                            &[("total", &total.to_string())],
                        );
                        ui.label(RichText::new(msg).size(SIZE_SMALL).color(p.warning));
                    }
                });
            }

            // Quiz result
            let mut quiz_enabled = next_tracking.quiz.is_some();
            if ui.checkbox(&mut quiz_enabled, tl!("Quiz result")).changed() {
                next_tracking.quiz = if quiz_enabled {
                    let first_slide = results_slides.first().map(|(id, _)| id.clone());
                    Some(QuizTracking { slide: first_slide })
                } else {
                    None
                };
            }
            if let Some(ref mut qt) = next_tracking.quiz {
                ui.indent((id_prefix, "quiz"), |ui| {
                    let current_slide = qt.slide.clone();
                    let current_play_slide = current_slide
                        .as_deref()
                        .and_then(|id| editor.session.course.slides().find(|s| s.id == id));
                    let slide_in_play_order =
                        current_slide.is_none() || current_play_slide.is_some();
                    let selected_label = match &current_slide {
                        None => tl!("Course quiz (all graded questions)").to_owned(),
                        Some(id) => current_play_slide
                            .map(|slide| {
                                if slide.title.trim().is_empty() {
                                    id.clone()
                                } else {
                                    slide.title.clone()
                                }
                            })
                            .unwrap_or_else(|| {
                                craft_i18n::fmt(tl!("Missing slide ({id})"), &[("id", id)])
                            }),
                    };

                    ui.horizontal_wrapped(|ui| {
                        ui.label(tl!("Results slide"));
                        crate::widgets::styled_combo(
                            ui,
                            (id_prefix, "quiz_slide"),
                            crate::widgets::COMBO_MIN_WIDTH,
                        )
                        .selected_text(selected_label)
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(
                                    current_slide.is_none(),
                                    tl!("Course quiz (all graded questions)"),
                                )
                                .clicked()
                            {
                                qt.slide = None;
                            }
                            for (sid, title) in &results_slides {
                                let label = if title.trim().is_empty() {
                                    sid.as_str()
                                } else {
                                    title.as_str()
                                };
                                if ui
                                    .selectable_label(
                                        current_slide.as_deref() == Some(sid.as_str()),
                                        label,
                                    )
                                    .clicked()
                                {
                                    qt.slide = Some(sid.clone());
                                }
                            }
                        });
                    });

                    if let Some(pct) = passing {
                        let msg = craft_i18n::fmt(
                            tl!("Passing score is {percent}% in Quiz settings."),
                            &[("percent", &pct.to_string())],
                        );
                        ui.label(RichText::new(msg).size(SIZE_SMALL).color(p.text_secondary));
                    } else {
                        ui.label(
                            RichText::new(tl!("No passing score is set in Quiz settings."))
                                .size(SIZE_SMALL)
                                .color(p.warning),
                        );
                    }

                    if qt.slide.is_none() && !has_graded && results_slides.is_empty() {
                        ui.label(
                            RichText::new(tl!(
                                "This course has no graded questions or results slide."
                            ))
                            .size(SIZE_SMALL)
                            .color(p.warning),
                        );
                    }

                    if !slide_in_play_order {
                        ui.label(
                            RichText::new(tl!("Selected results slide no longer exists."))
                                .size(SIZE_SMALL)
                                .color(p.warning),
                        );
                    }
                });
            }

            // Completion triggers (always active)
            ui.add_space(SPACE_SM);
            ui.horizontal(|ui| {
                ui.label(RichText::new("✓").size(SIZE_SMALL).color(p.text_secondary));
                ui.label(
                    RichText::new(tl!("Completion triggers are always active"))
                        .size(SIZE_SMALL)
                        .color(p.text_secondary),
                )
                .on_hover_text(tl!(
                    "Complete course and Submit results triggers always report to the LMS."
                ));
            });
        })
        .response;

    if !enabled {
        resp.on_disabled_hover_text(tl!("Wait for publishing to finish"));
    }

    if next_tracking != tracking {
        editor.run(
            "course.set_tracking",
            serde_json::json!({
                "slides_viewed": next_tracking.slides_viewed,
                "quiz": next_tracking.quiz,
            }),
        );
    }
}

/// Concise localized summary of the course's active LMS tracking options.
#[must_use]
pub fn tracking_summary(course: &Course) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(sv) = course.tracking.slides_viewed {
        match sv {
            SlidesViewedTracking::All => {
                parts.push(tl!("Slides viewed (all)").to_owned());
            }
            SlidesViewedTracking::AtLeast { count } => {
                parts.push(craft_i18n::fmt(
                    tl!("Slides viewed ({count})"),
                    &[("count", &count.to_string())],
                ));
            }
        }
    }
    if course.tracking.quiz.is_some() {
        parts.push(tl!("Quiz result").to_owned());
    }
    if parts.is_empty() {
        tl!("Triggers only").to_owned()
    } else {
        parts.push(tl!("Triggers").to_owned());
        parts.join(", ")
    }
}
