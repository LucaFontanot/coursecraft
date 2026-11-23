//! Pure, cross-platform unsaved-changes state machine for the web editor.

use coursecraft_ui_egui::{ExampleId, TemplateId};

/// Course-replacing actions in the web editor that require confirmation when dirty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebPendingAction {
    /// Start a new blank course.
    New,
    /// Open a `.ccraft` or `course.json` file from disk.
    Open,
    /// Open one of the built-in example courses.
    OpenExample(ExampleId),
    /// Open one of the built-in starter templates.
    OpenTemplate(TemplateId),
    /// Import a PowerPoint (`.pptx`) file.
    ImportPptx,
}

/// Tracks unsaved-changes revision and any pending course-replacing action on the web.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebUnsavedState {
    /// Session revision at the last save, load, or reset.
    pub saved_revision: u64,
    /// Display name of the last saved or opened project file, if any.
    pub file_name: Option<String>,
    /// Action waiting for user confirmation in the `"Save changes?"` modal.
    pub pending: Option<WebPendingAction>,
}

impl WebUnsavedState {
    /// Create a clean state at `revision` with no file name or pending action.
    #[must_use]
    pub fn new(revision: u64) -> Self {
        Self {
            saved_revision: revision,
            file_name: None,
            pending: None,
        }
    }

    /// Whether the current session `revision` differs from the last saved/loaded revision.
    #[must_use]
    pub fn is_dirty(&self, revision: u64) -> bool {
        revision != self.saved_revision
    }

    /// Display name shown in the status bar and unsaved-changes prompt.
    #[must_use]
    pub fn file_name_display(&self) -> &str {
        self.file_name.as_deref().unwrap_or("Untitled")
    }

    /// Record `revision` as saved (keeping any existing file name).
    pub fn mark_saved(&mut self, revision: u64) {
        self.saved_revision = revision;
    }

    /// Record `revision` as saved under `file_name`.
    pub fn mark_saved_as(&mut self, revision: u64, file_name: impl Into<String>) {
        self.saved_revision = revision;
        self.file_name = Some(file_name.into());
    }

    /// Reset to a clean, untitled state at `revision` (e.g. after New, Example, or Template).
    pub fn reset_for_new(&mut self, revision: u64) {
        self.saved_revision = revision;
        self.file_name = None;
    }

    /// Mark the course dirty after importing a PowerPoint deck (mirrors desktop).
    pub fn mark_dirty_after_import(&mut self) {
        self.saved_revision = u64::MAX;
        self.file_name = None;
    }

    /// Return `Some(action)` to run immediately when clean, or store `pending` and return
    /// `None` when dirty so the confirmation modal can ask first.
    pub fn request_or_run(
        &mut self,
        action: WebPendingAction,
        revision: u64,
    ) -> Option<WebPendingAction> {
        if self.is_dirty(revision) {
            self.pending = Some(action);
            None
        } else {
            Some(action)
        }
    }

    /// Take and clear any pending action (e.g. after Save or Don't Save).
    pub fn take_pending(&mut self) -> Option<WebPendingAction> {
        self.pending.take()
    }

    /// Clear any pending action without running it (Cancel / Escape).
    pub fn cancel(&mut self) {
        self.pending = None;
    }
}
