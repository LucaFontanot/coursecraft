//! Preferences storage and recent files tracking.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Maximum number of recent files retained.
pub const MAX_RECENT: usize = 8;

fn default_true() -> bool {
    true
}

/// User theme preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemePref {
    /// Always use the dark theme (default).
    #[default]
    Dark,
    /// Always use the light theme.
    Light,
    /// Follow the operating system theme (defaults to dark if undetermined).
    System,
}

impl From<coursecraft_ui_egui::coursecraft_render::ThemePref> for ThemePref {
    fn from(p: coursecraft_ui_egui::coursecraft_render::ThemePref) -> Self {
        match p {
            coursecraft_ui_egui::coursecraft_render::ThemePref::Dark => Self::Dark,
            coursecraft_ui_egui::coursecraft_render::ThemePref::Light => Self::Light,
            coursecraft_ui_egui::coursecraft_render::ThemePref::System => Self::System,
        }
    }
}

impl From<ThemePref> for coursecraft_ui_egui::coursecraft_render::ThemePref {
    fn from(p: ThemePref) -> Self {
        match p {
            ThemePref::Dark => coursecraft_ui_egui::coursecraft_render::ThemePref::Dark,
            ThemePref::Light => coursecraft_ui_egui::coursecraft_render::ThemePref::Light,
            ThemePref::System => coursecraft_ui_egui::coursecraft_render::ThemePref::System,
        }
    }
}

impl ThemePref {
    /// Resolve preference to a concrete theme mode given the egui context.
    #[must_use]
    pub fn resolve(
        self,
        ctx: &egui::Context,
    ) -> coursecraft_ui_egui::coursecraft_render::ThemeMode {
        coursecraft_ui_egui::coursecraft_render::ThemePref::from(self).resolve(ctx)
    }
}

/// Optional AI endpoint settings, stored only on the desktop.
///
/// The key is never printed: [`Debug`] shows `***` instead.
#[derive(Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AiPrefs {
    /// What powers the built-in AI features: an endpoint or an installed agent.
    #[serde(default)]
    pub source: coursecraft_ai::AiSource,
    /// Base URL of the OpenAI-compatible endpoint.
    pub base_url: String,
    /// Model name.
    pub model: String,
    /// API key; empty when the server needs none.
    pub api_key: String,
    /// Speech-to-text model, omitted when unset.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub speech_to_text_model: String,
    /// Text-to-speech model, omitted when unset.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text_to_speech_model: String,
    /// Text-to-speech voice, omitted when unset.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub voice: String,
}

impl std::fmt::Debug for AiPrefs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AiPrefs")
            .field("source", &self.source)
            .field("base_url", &self.base_url)
            .field("model", &self.model)
            .field("api_key", &"***")
            .finish()
    }
}

impl AiPrefs {
    /// Read the config used by the AI client.
    #[must_use]
    pub fn to_config(&self) -> coursecraft_ai::AiConfig {
        coursecraft_ai::AiConfig {
            base_url: self.base_url.clone(),
            model: self.model.clone(),
            api_key: self.api_key.clone(),
            speech_to_text_model: self.speech_to_text_model.clone(),
            text_to_speech_model: self.text_to_speech_model.clone(),
            voice: self.voice.clone(),
        }
    }

    /// Store a config in prefs.
    #[must_use]
    pub fn from_config(config: &coursecraft_ai::AiConfig) -> Self {
        Self {
            source: coursecraft_ai::AiSource::Endpoint,
            base_url: config.base_url.clone(),
            model: config.model.clone(),
            api_key: config.api_key.clone(),
            speech_to_text_model: config.speech_to_text_model.clone(),
            text_to_speech_model: config.text_to_speech_model.clone(),
            voice: config.voice.clone(),
        }
    }
}

/// Optional ElevenLabs narration settings, stored only on the desktop.
///
/// The key is never printed: [`Debug`] shows `***` instead.
#[derive(Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ElevenLabsPrefs {
    /// API key.
    pub api_key: String,
    /// Voice id used in the speech URL path.
    pub voice_id: String,
    /// Human name of the voice.
    pub voice_name: String,
    /// Model id; empty means the default.
    pub model_id: String,
}

impl std::fmt::Debug for ElevenLabsPrefs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ElevenLabsPrefs")
            .field("api_key", &"***")
            .field("voice_id", &self.voice_id)
            .field("voice_name", &self.voice_name)
            .field("model_id", &self.model_id)
            .finish()
    }
}

impl ElevenLabsPrefs {
    /// Read the config used by the ElevenLabs client.
    #[must_use]
    pub fn to_config(&self) -> coursecraft_ai::ElevenLabsConfig {
        coursecraft_ai::ElevenLabsConfig {
            api_key: self.api_key.clone(),
            voice_id: self.voice_id.clone(),
            voice_name: self.voice_name.clone(),
            model_id: self.model_id.clone(),
        }
    }

    /// Store a config in prefs.
    #[must_use]
    pub fn from_config(config: &coursecraft_ai::ElevenLabsConfig) -> Self {
        Self {
            api_key: config.api_key.clone(),
            voice_id: config.voice_id.clone(),
            voice_name: config.voice_name.clone(),
            model_id: config.model_id.clone(),
        }
    }
}

/// Which service the Narration from Text window used last.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SpeechSourcePref {
    /// The OpenAI-compatible endpoint in AI Settings.
    #[default]
    AiProvider,
    /// The author's own ElevenLabs account.
    ElevenLabs,
}

impl From<SpeechSourcePref> for coursecraft_ui_egui::SpeechSource {
    fn from(source: SpeechSourcePref) -> Self {
        match source {
            SpeechSourcePref::AiProvider => Self::AiProvider,
            SpeechSourcePref::ElevenLabs => Self::ElevenLabs,
        }
    }
}

impl From<coursecraft_ui_egui::SpeechSource> for SpeechSourcePref {
    fn from(source: coursecraft_ui_egui::SpeechSource) -> Self {
        match source {
            coursecraft_ui_egui::SpeechSource::AiProvider => Self::AiProvider,
            coursecraft_ui_egui::SpeechSource::ElevenLabs => Self::ElevenLabs,
        }
    }
}

/// User editor performance mode stored in `prefs.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PerformanceModePref {
    /// Automatically pick Lite on constrained devices or large courses.
    #[default]
    Auto,
    /// Standard cache ceilings and full editor preview detail.
    Standard,
    /// Reduced editor cache ceilings and preview detail.
    Lite,
}

impl From<coursecraft_ui_egui::coursecraft_render::PerformanceMode> for PerformanceModePref {
    fn from(mode: coursecraft_ui_egui::coursecraft_render::PerformanceMode) -> Self {
        match mode {
            coursecraft_ui_egui::coursecraft_render::PerformanceMode::Auto => Self::Auto,
            coursecraft_ui_egui::coursecraft_render::PerformanceMode::Standard => Self::Standard,
            coursecraft_ui_egui::coursecraft_render::PerformanceMode::Lite => Self::Lite,
        }
    }
}

impl From<PerformanceModePref> for coursecraft_ui_egui::coursecraft_render::PerformanceMode {
    fn from(mode: PerformanceModePref) -> Self {
        match mode {
            PerformanceModePref::Auto => Self::Auto,
            PerformanceModePref::Standard => Self::Standard,
            PerformanceModePref::Lite => Self::Lite,
        }
    }
}

/// Persisted user preferences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Prefs {
    #[serde(default)]
    pub recent: Vec<String>,
    #[serde(default = "default_true")]
    pub automation_enabled: bool,
    #[serde(default)]
    pub theme: ThemePref,
    #[serde(default)]
    pub coach_marks_done: bool,
    /// Optional AI endpoint, omitted entirely when there is none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai: Option<AiPrefs>,
    /// Optional ElevenLabs narration settings, omitted when there are none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elevenlabs: Option<ElevenLabsPrefs>,
    /// Which service the Narration from Text window used last.
    #[serde(default)]
    pub speech_source: SpeechSourcePref,
    /// Minutes between autosaves. 0 turns autosave off.
    #[serde(default = "default_autosave_minutes")]
    pub autosave_minutes: u32,
    /// How many earlier saves of each project to keep. 0 keeps none.
    #[serde(default = "default_versions_to_keep")]
    pub versions_to_keep: u32,
    /// Slide size of a new course.
    #[serde(default)]
    pub default_slide: DefaultSlideSize,
    /// Underline misspellings while editing text.
    #[serde(default = "default_true")]
    pub spell_check_as_you_type: bool,
    /// Words added to the personal dictionary.
    #[serde(default)]
    pub spell_words: Vec<String>,
    /// Editor performance mode (`auto` / `standard` / `lite`).
    #[serde(default)]
    pub performance_mode: PerformanceModePref,
    /// Check GitHub at most once a day on startup for a newer desktop version.
    #[serde(default = "default_true")]
    pub auto_check_updates: bool,
    /// Unix timestamp (seconds) of the last completed or rate-limited update check.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_update_check_unix: Option<u64>,
    /// Semver version string the user chose to skip (e.g. `"0.4.4"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skipped_update_version: Option<String>,
    /// Interface language: `"auto"` follows the system, else a language code.
    #[serde(default = "default_language")]
    pub language: String,
    /// The ribbon replaces the flat toolbar.
    #[serde(default = "default_true")]
    pub ribbon: bool,
    /// The panels are docked, so they can be moved, tabbed, floated and closed.
    #[serde(default = "default_true")]
    pub docked_panels: bool,
    /// The docked panel arrangement (`craft_ui::docking::Workspace`); the editor
    /// checks it on load and falls back to the default arrangement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub panel_layout: Option<serde_json::Value>,
}

fn default_language() -> String {
    coursecraft_ui_egui::i18n::AUTO.to_owned()
}

/// Largest autosave interval, in minutes.
pub const MAX_AUTOSAVE_MINUTES: u32 = 60;

fn default_autosave_minutes() -> u32 {
    1
}

/// Most file versions a project can keep.
pub const MAX_VERSIONS_TO_KEEP: u32 = 100;

fn default_versions_to_keep() -> u32 {
    10
}

/// The slide sizes a new course can start with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SlidePreset {
    #[default]
    Wide16x9,
    Wide16x10,
    Standard4x3,
    Square,
}

impl SlidePreset {
    /// Width and height in pixels.
    #[must_use]
    pub fn size(self) -> (u32, u32) {
        match self {
            Self::Wide16x9 => (1280, 720),
            Self::Wide16x10 => (1280, 800),
            Self::Standard4x3 => (960, 720),
            Self::Square => (1080, 1080),
        }
    }

    /// The preset with this size, if it is one.
    #[must_use]
    pub fn from_size(size: (u32, u32)) -> Option<Self> {
        [
            Self::Wide16x9,
            Self::Wide16x10,
            Self::Standard4x3,
            Self::Square,
        ]
        .into_iter()
        .find(|p| p.size() == size)
    }
}

/// Default slide size for new courses (16:9 HD).
pub const DEFAULT_SLIDE_SIZE: (u32, u32) = (1280, 720);

/// Allowed slide width range in pixels.
pub const SLIDE_W_RANGE: std::ops::RangeInclusive<u32> = 100..=7680;

/// Allowed slide height range in pixels.
pub const SLIDE_H_RANGE: std::ops::RangeInclusive<u32> = 100..=4320;

/// Backward-compatible default slide size stored in `prefs.json`.
///
/// Accepts both legacy [`SlidePreset`] strings (`"wide16x9"`, `"wide16x10"`,
/// `"standard4x3"`, `"square"`) and `{ "w": u32, "h": u32 }` custom sizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefaultSlideSize {
    pub w: u32,
    pub h: u32,
}

impl Default for DefaultSlideSize {
    fn default() -> Self {
        Self {
            w: DEFAULT_SLIDE_SIZE.0,
            h: DEFAULT_SLIDE_SIZE.1,
        }
    }
}

impl From<SlidePreset> for DefaultSlideSize {
    fn from(preset: SlidePreset) -> Self {
        let (w, h) = preset.size();
        Self { w, h }
    }
}

impl DefaultSlideSize {
    /// Construct a [`DefaultSlideSize`] if `(w, h)` is within `100..=7680` × `100..=4320`.
    #[must_use]
    pub fn from_valid_size((w, h): (u32, u32)) -> Option<Self> {
        if SLIDE_W_RANGE.contains(&w) && SLIDE_H_RANGE.contains(&h) {
            Some(Self { w, h })
        } else {
            None
        }
    }

    /// Width and height in pixels.
    #[must_use]
    pub fn size(self) -> (u32, u32) {
        (self.w, self.h)
    }
}

impl Serialize for DefaultSlideSize {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if let Some(preset) = SlidePreset::from_size(self.size()) {
            preset.serialize(serializer)
        } else {
            #[derive(Serialize)]
            struct RawSize {
                w: u32,
                h: u32,
            }
            RawSize {
                w: self.w,
                h: self.h,
            }
            .serialize(serializer)
        }
    }
}

impl<'de> Deserialize<'de> for DefaultSlideSize {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let Ok(val) = serde_json::Value::deserialize(deserializer) else {
            return Ok(Self::default());
        };
        if let Some(s) = val.as_str() {
            if let Ok(preset) =
                serde_json::from_value::<SlidePreset>(serde_json::Value::String(s.to_owned()))
            {
                return Ok(Self::from(preset));
            }
            return Ok(Self::default());
        }
        if let Some(obj) = val.as_object() {
            let w = obj
                .get("w")
                .and_then(serde_json::Value::as_u64)
                .and_then(|v| u32::try_from(v).ok());
            let h = obj
                .get("h")
                .and_then(serde_json::Value::as_u64)
                .and_then(|v| u32::try_from(v).ok());
            if let (Some(w), Some(h)) = (w, h)
                && let Some(valid) = Self::from_valid_size((w, h))
            {
                return Ok(valid);
            }
        }
        Ok(Self::default())
    }
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            recent: Vec::new(),
            automation_enabled: true,
            theme: ThemePref::default(),
            coach_marks_done: false,
            ai: None,
            elevenlabs: None,
            speech_source: SpeechSourcePref::default(),
            autosave_minutes: default_autosave_minutes(),
            versions_to_keep: default_versions_to_keep(),
            default_slide: DefaultSlideSize::default(),
            spell_check_as_you_type: true,
            spell_words: Vec::new(),
            language: default_language(),
            ribbon: true,
            docked_panels: true,
            panel_layout: None,
            performance_mode: PerformanceModePref::default(),
            auto_check_updates: true,
            last_update_check_unix: None,
            skipped_update_version: None,
        }
    }
}

impl Prefs {
    /// The ribbon, docking and panel settings for the editor.
    #[must_use]
    pub fn layout(&self) -> coursecraft_ui_egui::dock::LayoutPrefs {
        coursecraft_ui_egui::dock::LayoutPrefs {
            ribbon: self.ribbon,
            docked: self.docked_panels,
            workspace: self.panel_layout.clone(),
        }
    }

    /// Store the editor's ribbon, docking and panel settings.
    pub fn set_layout(&mut self, layout: coursecraft_ui_egui::dock::LayoutPrefs) {
        self.ribbon = layout.ribbon;
        self.docked_panels = layout.docked;
        self.panel_layout = layout.workspace;
    }
}

impl Prefs {
    /// The autosave interval, repaired into range.
    #[must_use]
    pub fn autosave_minutes_clamped(&self) -> u32 {
        self.autosave_minutes.min(MAX_AUTOSAVE_MINUTES)
    }

    /// The number of versions to keep, repaired into range.
    #[must_use]
    pub fn versions_to_keep_clamped(&self) -> u32 {
        self.versions_to_keep.min(MAX_VERSIONS_TO_KEEP)
    }

    /// A new blank course at the default slide size.
    #[must_use]
    pub fn blank_course(&self) -> coursecraft_course::Course {
        let mut course = coursecraft_engine::Session::blank("Untitled course").course;
        let (w, h) = self.default_slide.size();
        course.slide_size = coursecraft_course::Size { w, h };
        course
    }

    /// Add a path to the recent files list, deduplicating and keeping at most [`MAX_RECENT`].
    pub fn add_recent(&mut self, path: &Path) {
        let path_str = path.to_string_lossy().to_string();
        self.recent.retain(|p| p != &path_str);
        self.recent.insert(0, path_str);
        self.recent.truncate(MAX_RECENT);
    }

    /// Remove a path from the recent files list.
    pub fn remove_recent(&mut self, path: &Path) {
        let path_str = path.to_string_lossy().to_string();
        self.recent.retain(|p| p != &path_str);
    }
}

/// Determine the default application configuration directory from environment variables.
#[must_use]
pub fn config_dir_from_env() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join("Library/Application Support/CourseCraft"))
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("CourseCraft"))
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(|xdg| PathBuf::from(xdg).join("coursecraft"))
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config/coursecraft"))
            })
    }
}

/// Load preferences from `prefs.json` inside the specified directory.
///
/// Missing or corrupt files safely return default preferences without crashing.
#[must_use]
pub fn load_prefs(config_dir: &Path) -> Prefs {
    let prefs_file = config_dir.join("prefs.json");
    let Ok(data) = std::fs::read(&prefs_file) else {
        return Prefs::default();
    };
    serde_json::from_slice(&data).unwrap_or_default()
}

/// Save preferences to `prefs.json` inside the specified directory.
///
/// # Errors
/// Returns an error if the directory cannot be created or the file cannot be written.
pub fn save_prefs(config_dir: &Path, prefs: &Prefs) -> Result<(), String> {
    if let Err(e) = std::fs::create_dir_all(config_dir) {
        return Err(format!("Failed to create config directory: {e}"));
    }
    let prefs_file = config_dir.join("prefs.json");
    let json = serde_json::to_string_pretty(prefs).map_err(|e| e.to_string())?;
    crate::files::write_atomic(&prefs_file, json.as_bytes())
        .map_err(|e| format!("Failed to write preferences: {e}"))?;

    #[cfg(unix)]
    if prefs.ai.as_ref().is_some_and(|ai| !ai.api_key.is_empty())
        || prefs
            .elevenlabs
            .as_ref()
            .is_some_and(|elevenlabs| !elevenlabs.api_key.is_empty())
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&prefs_file, permissions)
            .map_err(|e| format!("Failed to secure preferences: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{PerformanceModePref, Prefs, load_prefs, save_prefs};

    #[test]
    fn performance_mode_defaults_to_auto_on_older_json_and_round_trips() {
        let dir = std::env::temp_dir().join(format!(
            "coursecraft-prefs-perf-mode-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");

        // Older prefs.json without `performance_mode` must default to Auto.
        std::fs::write(dir.join("prefs.json"), r#"{"recent": []}"#).expect("write old prefs");
        let old = load_prefs(&dir);
        assert_eq!(old.performance_mode, PerformanceModePref::Auto);

        for mode in [
            PerformanceModePref::Standard,
            PerformanceModePref::Lite,
            PerformanceModePref::Auto,
        ] {
            let prefs = Prefs {
                performance_mode: mode,
                ..Prefs::default()
            };
            save_prefs(&dir, &prefs).expect("save prefs");
            let loaded = load_prefs(&dir);
            assert_eq!(loaded.performance_mode, mode);
            let render_mode: coursecraft_ui_egui::coursecraft_render::PerformanceMode = mode.into();
            assert_eq!(PerformanceModePref::from(render_mode), mode);
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn up09_update_prefs_backward_compat_and_round_trip() {
        let dir =
            std::env::temp_dir().join(format!("coursecraft-prefs-up09-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");

        // Legacy prefs.json lacking the 3 update fields must load with defaults.
        std::fs::write(dir.join("prefs.json"), r#"{"recent": []}"#).expect("write legacy prefs");
        let legacy = load_prefs(&dir);
        assert!(legacy.auto_check_updates);
        assert_eq!(legacy.last_update_check_unix, None);
        assert_eq!(legacy.skipped_update_version, None);

        // Saving and reloading with all 3 fields populated must preserve them exactly.
        let populated = Prefs {
            auto_check_updates: false,
            last_update_check_unix: Some(1_700_000_000),
            skipped_update_version: Some("0.4.4".to_owned()),
            ..Prefs::default()
        };
        save_prefs(&dir, &populated).expect("save prefs");
        let reloaded = load_prefs(&dir);
        assert!(!reloaded.auto_check_updates);
        assert_eq!(reloaded.last_update_check_unix, Some(1_700_000_000));
        assert_eq!(reloaded.skipped_update_version.as_deref(), Some("0.4.4"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
