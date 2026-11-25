//! The Publish dialog: standard picker, pre-flight checks and publish results.

use crate::dialog::Dialog;
use coursecraft_course::LmsStatus;
use coursecraft_render::{
    CONTROL_PRIMARY, RADIUS_MD, SIZE_BODY, SPACE_MD, SPACE_SM, palette, small,
};
use egui::{Color32, RichText};

use crate::{Editor, EditorRequest};

/// The package standards an author can pick, without pulling the
/// publishing crate into the UI layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PublishStandard {
    /// SCORM 1.2, the safe default.
    #[default]
    Scorm12,
    /// SCORM 2004 4th edition.
    Scorm2004,
    /// cmi5, which uses xAPI.
    Cmi5,
    /// Plain xAPI (Tin Can).
    Xapi,
    /// LTI 1.3, for the CourseCraft LTI server.
    Lti,
    /// Standalone web hosting with no LMS.
    Web,
}

impl PublishStandard {
    /// Every standard, in the order shown in the dialog.
    pub const ALL: [Self; 6] = [
        Self::Scorm12,
        Self::Scorm2004,
        Self::Cmi5,
        Self::Xapi,
        Self::Lti,
        Self::Web,
    ];

    /// Learner-facing name for the standard picker.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Scorm12 => "SCORM 1.2",
            Self::Scorm2004 => "SCORM 2004 (4th edition)",
            Self::Cmi5 => "cmi5",
            Self::Xapi => "xAPI (Tin Can)",
            Self::Lti => "LTI 1.3 (for the CourseCraft LTI server)",
            Self::Web => "Web (any website, no LMS)",
        }
    }

    /// One line of plain help beneath the name.
    #[must_use]
    pub fn help(self) -> &'static str {
        match self {
            Self::Scorm12 => "Works in almost every LMS. Choose this if you're not sure.",
            Self::Scorm2004 => "For an LMS that asks for SCORM 2004.",
            Self::Cmi5 => "A modern standard using xAPI. Your LMS must support cmi5.",
            Self::Xapi => "Sends results to a Learning Record Store.",
            Self::Lti => "Launch from an LTI 1.3 platform through your own CourseCraft LTI server.",
            Self::Web => "Upload the folder to any web server. Results are not reported to an LMS.",
        }
    }

    /// Short, filename-safe standard name used in the default zip name.
    #[must_use]
    pub fn file_stem(self) -> &'static str {
        match self {
            Self::Scorm12 => "scorm12",
            Self::Scorm2004 => "scorm2004",
            Self::Cmi5 => "cmi5",
            Self::Xapi => "xapi",
            Self::Lti => "lti",
            Self::Web => "web",
        }
    }

    /// Parse a standard from its [`Self::file_stem`].
    #[must_use]
    pub fn from_stem(stem: &str) -> Option<Self> {
        match stem {
            "scorm12" => Some(Self::Scorm12),
            "scorm2004" => Some(Self::Scorm2004),
            "cmi5" => Some(Self::Cmi5),
            "xapi" => Some(Self::Xapi),
            "lti" => Some(Self::Lti),
            "web" => Some(Self::Web),
            _ => None,
        }
    }

    /// Help shown under a successful publish.
    #[must_use]
    pub fn upload_help(self) -> &'static str {
        match self {
            Self::Xapi => {
                "Upload this zip to your LMS, or host it and launch it with your LRS details."
            }
            Self::Lti => {
                "Publish this folder to the CourseCraft LTI server, then launch it from your platform."
            }
            Self::Web => {
                "Unzip and upload the folder to any web server. Results are not reported to an LMS."
            }
            Self::Scorm12 | Self::Scorm2004 | Self::Cmi5 => {
                "Upload this zip to your LMS as a new course."
            }
        }
    }
}

/// What the Publish dialog is showing.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PublishPhase {
    /// Ready for the author to choose a standard and publish.
    #[default]
    Idle,
    /// A background publish is running; the dialog cannot be closed.
    Busy,
    /// The package was written.
    Success {
        /// File name shown to the author.
        file_name: String,
        /// Size in bytes.
        bytes: u64,
    },
    /// The publish failed and the author can try again.
    Failed {
        /// Plain sentence explaining the problem.
        message: String,
    },
}

/// Transient state for the Publish dialog, kept in `editor.ui` like other dialogs.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PublishState {
    /// Whether the dialog is open.
    pub open: bool,
    /// Chosen standard; kept for the rest of the session.
    pub standard: PublishStandard,
    /// What the dialog is showing.
    pub phase: PublishPhase,
    /// Optional 0.0..=1.0 progress shown alongside `"Publishing…"` when busy.
    pub progress: Option<f32>,
    /// A quiet status line, e.g. when "Show in folder" fails.
    pub status: Option<String>,
    /// Font families auto-embedded into the published package.
    pub embedded_fonts: Vec<String>,
    /// Font warnings from publishing (missing, restricted, licence notice).
    pub font_warnings: Vec<String>,
    /// Auto-embedded font faces offered to be saved into the project (undoable).
    pub pending_save_fonts: Vec<(coursecraft_course::FontFace, std::sync::Arc<[u8]>)>,
}

impl Editor {
    /// Open the Publish dialog and refresh its pre-flight checks.
    pub fn open_publish_dialog(&mut self) {
        if !self.file_ops_available {
            return;
        }
        self.ui.publish.open = true;
        self.ui.publish.status = None;
        if !matches!(self.ui.publish.phase, PublishPhase::Busy) {
            self.ui.publish.phase = PublishPhase::Idle;
            self.ui.publish.progress = None;
            self.ui.publish.standard = self
                .session
                .course
                .editor
                .last_publish
                .as_ref()
                .and_then(|last| PublishStandard::from_stem(&last.standard))
                .unwrap_or_default();
        }
        self.refresh_validation();
    }

    /// Show that a publish is running.
    pub fn set_publish_busy(&mut self) {
        self.ui.publish.open = true;
        self.ui.publish.phase = PublishPhase::Busy;
        self.ui.publish.progress = None;
        self.ui.publish.status = None;
    }

    /// Update the optional 0.0..=1.0 progress shown while publishing.
    pub fn set_publish_progress(&mut self, progress: Option<f32>) {
        self.ui.publish.progress = progress.map(|p| coursecraft_course::safe_clamp(p, 0.0, 1.0));
    }

    /// Show a successful publish.
    pub fn set_publish_success(&mut self, file_name: impl Into<String>, bytes: u64) {
        self.set_publish_success_with_fonts(file_name, bytes, Vec::new(), Vec::new(), Vec::new());
    }

    /// Show a successful publish with auto-embedded font summary and warnings.
    pub fn set_publish_success_with_fonts(
        &mut self,
        file_name: impl Into<String>,
        bytes: u64,
        embedded_fonts: Vec<String>,
        font_warnings: Vec<String>,
        pending_save_fonts: Vec<(coursecraft_course::FontFace, std::sync::Arc<[u8]>)>,
    ) {
        self.ui.publish.open = true;
        self.ui.publish.phase = PublishPhase::Success {
            file_name: file_name.into(),
            bytes,
        };
        self.ui.publish.progress = None;
        self.ui.publish.status = None;
        self.ui.publish.embedded_fonts = embedded_fonts;
        self.ui.publish.font_warnings = font_warnings;
        self.ui.publish.pending_save_fonts = pending_save_fonts;
    }

    /// Show a failed publish and keep the dialog open for a retry.
    pub fn set_publish_failure(&mut self, message: impl Into<String>) {
        self.ui.publish.open = true;
        self.ui.publish.phase = PublishPhase::Failed {
            message: message.into(),
        };
        self.ui.publish.progress = None;
        self.ui.publish.status = None;
    }

    /// Set or clear the quiet status line under the result.
    pub fn set_publish_status(&mut self, message: Option<String>) {
        self.ui.publish.status = message;
    }

    /// Configure whether this build can publish (it has a bundled player).
    pub fn set_publish_available(&mut self, available: bool) {
        self.publish_available = available;
    }
}

/// Draw the Publish dialog when it is open.
pub fn show_publish_dialog(
    editor: &mut Editor,
    ctx: &egui::Context,
    requests: &mut Vec<EditorRequest>,
) {
    if !editor.ui.publish.open {
        return;
    }

    let busy = matches!(editor.ui.publish.phase, PublishPhase::Busy);
    let success = matches!(editor.ui.publish.phase, PublishPhase::Success { .. });
    let mut open = true;
    let mut close_after = false;

    let mut dialog = Dialog::new("Publish course").width(430.0).resizable(true);
    if !busy {
        dialog = dialog.open(&mut open);
    }
    dialog.show_with_footer(ctx, |dialog| {
        let p = palette(ctx);
        let validation_errors = editor.ui.validation_errors;
        let available = editor.publish_available;
        let missing_media = editor.missing_media();
        dialog.body(|ui| {
            ui.set_max_width(390.0);
            ui.label(
                small(editor.session.course.title.clone())
                    .size(SIZE_BODY)
                    .color(p.text_secondary),
            );
            ui.add_space(SPACE_MD);

            if success {
                show_success_details(editor, ui);
                return;
            }

            show_standards(editor, ui, busy);

            ui.separator();
            ui.add_space(SPACE_SM);
            if editor.ui.publish.standard == PublishStandard::Web {
                ui.label(
                    small(tl!("Web packages do not report completion to an LMS."))
                        .color(p.text_secondary),
                );
            } else {
                ui.label(
                    RichText::new(tl!("Tracking"))
                        .size(SIZE_BODY)
                        .strong()
                        .color(p.text),
                );
                crate::tracking_ui::show_tracking_controls(editor, ui, !busy, "publish_tracking");
            }

            let warnings = editor.ui.validation_warnings;
            let audit_errors = editor
                .ui
                .audit_findings
                .iter()
                .filter(|f| f.severity == "error")
                .count();

            if validation_errors > 0 {
                ui.add_space(SPACE_SM);
                ui.label(
                    RichText::new(format!(
                        "This course has {validation_errors} problem(s) to fix before publishing."
                    ))
                    .color(p.danger),
                );
                if ui.button("Show problems").clicked() {
                    editor.ui.show_validation_window = true;
                }
            } else if warnings > 0 {
                ui.add_space(SPACE_SM);
                ui.label(
                    small(format!("{warnings} warning(s). You can still publish."))
                        .color(p.text_secondary),
                );
            }

            if !missing_media.is_empty() {
                ui.add_space(SPACE_SM);
                for src in &missing_media {
                    ui.label(
                        RichText::new(format!("Missing media: {src}"))
                            .color(p.danger)
                            .size(SIZE_BODY),
                    );
                }
            }

            if audit_errors > 0 {
                ui.add_space(SPACE_SM);
                ui.horizontal(|ui| {
                    ui.label(
                        small(format!(
                            "{audit_errors} accessibility findings. You can still publish."
                        ))
                        .color(p.text_secondary),
                    );
                    if ui.button("Show findings").clicked() {
                        editor.ui.show_validation_window = true;
                    }
                });
            }

            if !available {
                ui.add_space(SPACE_SM);
                ui.label(
                    small(
                        "This build doesn't include the player. Run cargo xtask build-player and rebuild.",
                    )
                    .color(p.danger),
                );
            }

            if let PublishPhase::Failed { ref message } = editor.ui.publish.phase {
                ui.add_space(SPACE_SM);
                ui.label(RichText::new(message.clone()).color(p.danger));
            }

            if let Some(ref status) = editor.ui.publish.status {
                ui.add_space(SPACE_SM);
                ui.label(small(status.clone()).color(p.danger));
            }
        });

        // The action buttons stay put below the scrolling body.
        dialog.footer(|ui| {
            ui.add_space(SPACE_SM);
            if success {
                show_success_buttons(editor, ui, requests, &mut close_after);
            } else if busy {
                ui.horizontal(|ui| {
                    ui.add(egui::Spinner::new());
                    let label = match editor.ui.publish.progress {
                        Some(progress) => {
                            let pct = (coursecraft_course::safe_clamp(progress, 0.0, 1.0) * 100.0)
                                .round() as u8;
                            format!("Publishing… {pct}%")
                        }
                        None => "Publishing…".to_string(),
                    };
                    ui.label(label);
                });
            } else {
                ui.horizontal(|ui| {
                    let blocked = validation_errors > 0 || !available || !missing_media.is_empty();
                    if blocked {
                        let reason = if validation_errors > 0 {
                            "Fix validation problems before publishing"
                        } else if !missing_media.is_empty() {
                            "Restore missing media files before publishing"
                        } else {
                            "Player bundle is not available in this build"
                        };
                        ui.add_enabled(false, egui::Button::new("Publish"))
                            .on_disabled_hover_text(reason);
                    } else if publish_button(ui, "Publish").clicked() {
                        editor.ui.publish.phase = PublishPhase::Idle;
                        editor.ui.publish.status = None;
                        let standard = editor.ui.publish.standard;
                        let lms_status = editor
                            .session
                            .course
                            .editor
                            .last_publish
                            .as_ref()
                            .map_or(LmsStatus::Auto, |lp| lp.lms_status);
                        editor.run(
                            "view.set",
                            serde_json::json!({
                                "last_publish": {
                                    "standard": standard.file_stem(),
                                    "lms_status": lms_status,
                                }
                            }),
                        );
                        requests.push(EditorRequest::Publish { standard });
                    }
                    if ui.button("Cancel").clicked() {
                        close_after = true;
                    }
                });
            }
        });
    });

    if close_after || (!busy && !open) {
        editor.ui.publish.open = false;
    }
}

const LMS_STATUS_TOOLTIP: &str =
    "What CourseCraft sends to the LMS when the learner finishes. Match what your LMS expects.";

const LMS_STATUS_OPTIONS: [LmsStatus; 6] = [
    LmsStatus::Auto,
    LmsStatus::AutoKeepCompleted,
    LmsStatus::PassedIncomplete,
    LmsStatus::PassedFailed,
    LmsStatus::CompletedIncomplete,
    LmsStatus::CompletedFailed,
];

fn lms_status_label(status: LmsStatus) -> String {
    match status {
        LmsStatus::Auto => tl!("Automatic (quiz result wins)"),
        LmsStatus::AutoKeepCompleted => tl!("Automatic (completion stays)"),
        LmsStatus::PassedIncomplete => tl!("Passed / Incomplete"),
        LmsStatus::PassedFailed => tl!("Passed / Failed"),
        LmsStatus::CompletedIncomplete => tl!("Completed / Incomplete"),
        LmsStatus::CompletedFailed => tl!("Completed / Failed"),
    }
    .to_owned()
}

fn lms_status_hint(status: LmsStatus) -> Option<String> {
    match status {
        LmsStatus::Auto => Some(
            tl!("A failed quiz shows as failed, even after the course was completed.").to_owned(),
        ),
        LmsStatus::AutoKeepCompleted => Some(
            tl!(
                "Once completed, the course stays completed; a failed quiz still records its score."
            )
            .to_owned(),
        ),
        _ => None,
    }
}

fn show_standards(editor: &mut Editor, ui: &mut egui::Ui, busy: bool) {
    let p = palette(ui.ctx());
    for standard in PublishStandard::ALL {
        let selected = editor.ui.publish.standard == standard;
        if ui
            .add_enabled(!busy, egui::RadioButton::new(selected, standard.label()))
            .on_disabled_hover_text("Wait for publishing to finish")
            .clicked()
        {
            editor.ui.publish.standard = standard;
        }
        ui.add_space(2.0);
        ui.label(small(standard.help()).color(p.text_secondary));
        ui.add_space(SPACE_SM);
    }
    if editor.ui.publish.standard == PublishStandard::Scorm12 {
        let current = editor
            .session
            .course
            .editor
            .last_publish
            .as_ref()
            .map_or(LmsStatus::Auto, |lp| lp.lms_status);
        let mut selected = current;
        ui.add_enabled_ui(!busy, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    small("Report status as")
                        .size(SIZE_BODY)
                        .color(p.text_secondary),
                )
                .on_hover_text(LMS_STATUS_TOOLTIP);
                crate::widgets::styled_combo(
                    ui,
                    "publish_scorm12_lms_status",
                    crate::widgets::COMBO_MIN_WIDTH,
                )
                .selected_text(lms_status_label(selected))
                .show_ui(ui, |ui| {
                    for option in LMS_STATUS_OPTIONS {
                        ui.selectable_value(&mut selected, option, lms_status_label(option));
                    }
                })
                .response
                .on_hover_text(LMS_STATUS_TOOLTIP)
                .on_disabled_hover_text("Wait for publishing to finish");
            });
            if let Some(hint) = lms_status_hint(selected) {
                ui.label(small(&hint).color(p.text_secondary));
            }
        })
        .response
        .on_disabled_hover_text("Wait for publishing to finish");
        if selected != current {
            editor.run(
                "course.set_publish_settings",
                serde_json::json!({ "lms_status": selected }),
            );
        }
        ui.add_space(SPACE_SM);
    }
    ui.add_space(SPACE_SM);
}

fn show_success_details(editor: &mut Editor, ui: &mut egui::Ui) {
    let p = palette(ui.ctx());
    let PublishPhase::Success {
        ref file_name,
        bytes,
    } = editor.ui.publish.phase
    else {
        return;
    };

    ui.label(
        RichText::new(format!("Published to {file_name}"))
            .size(SIZE_BODY)
            .strong()
            .color(p.text),
    );
    ui.add_space(SPACE_SM);
    let megabytes = bytes as f64 / (1024.0 * 1024.0);
    ui.label(small(format!("{megabytes:.1} MB")).color(p.text_secondary));

    if let Some(ref status) = editor.ui.publish.status {
        ui.add_space(SPACE_SM);
        ui.label(small(status.clone()).color(p.danger));
    }

    if !editor.ui.publish.embedded_fonts.is_empty() {
        ui.add_space(SPACE_SM);
        ui.label(
            small(format!(
                "Embedded fonts: {}.",
                editor.ui.publish.embedded_fonts.join(", ")
            ))
            .color(p.text_secondary),
        );
    }

    if !editor.ui.publish.font_warnings.is_empty() {
        ui.add_space(SPACE_SM);
        for warning in &editor.ui.publish.font_warnings {
            let color = if warning.contains("responsibility") {
                p.text_secondary
            } else {
                p.warning
            };
            ui.label(small(warning.clone()).color(color));
        }
    }

    if !editor.ui.publish.pending_save_fonts.is_empty() {
        ui.add_space(SPACE_SM);
        if ui.button("Save embedded fonts to project").clicked() {
            let to_save = std::mem::take(&mut editor.ui.publish.pending_save_fonts);
            for (face, bytes) in to_save {
                if !editor
                    .media
                    .iter()
                    .any(|(name, _)| name.eq_ignore_ascii_case(&face.src))
                {
                    editor.media.push(face.src.clone(), bytes);
                }
                let _ = editor.run(
                    "course.add_font",
                    serde_json::json!({
                        "family": face.family,
                        "style": face.style,
                        "src": face.src,
                    }),
                );
            }
        }
    }

    ui.add_space(SPACE_SM);
    ui.label(small(editor.ui.publish.standard.upload_help()).color(p.text_secondary));
}

fn show_success_buttons(
    editor: &mut Editor,
    ui: &mut egui::Ui,
    requests: &mut Vec<EditorRequest>,
    close_after: &mut bool,
) {
    ui.horizontal(|ui| {
        if !cfg!(target_arch = "wasm32") {
            let open_label = match editor.ui.publish.standard {
                PublishStandard::Web => "Open in browser",
                PublishStandard::Scorm12
                | PublishStandard::Scorm2004
                | PublishStandard::Cmi5
                | PublishStandard::Xapi
                | PublishStandard::Lti => "Open in browser (preview, no LMS)",
            };
            if ui.button(open_label).clicked() {
                requests.push(EditorRequest::OpenPublishedInBrowser);
            }
            if ui.button("Show in folder").clicked() {
                requests.push(EditorRequest::ShowPublished);
            }
        }
        if publish_button(ui, "Done").clicked() {
            *close_after = true;
        }
    });
}

fn publish_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let p = palette(ui.ctx());
    let button = egui::Button::new(RichText::new(label).color(Color32::WHITE))
        .fill(p.accent)
        .corner_radius(RADIUS_MD as u8)
        .min_size(egui::vec2(0.0, CONTROL_PRIMARY));
    ui.add(button)
}
