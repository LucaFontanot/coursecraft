//! The update check of the Craft suite (`craft_update`, as DeckCraft uses it): one quiet check
//! at start-up and one on demand from Help, the prompt that offers the update, and the window
//! that reports a requested check. The flow itself is `craft_update::updater::Updater`.
//!
//! Desktop only; the web build has no updates.

use craft_update::updater::{Action, Status, Updater};
use craft_update::{CheckResultDialog, CheckResultResponse};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// CourseCraft as craft-update knows it: its releases live in the upstream repository, and the
/// prompt shows its icon and teal accent.
pub const UPDATE_APP: craft_update::App = craft_update::App {
    icon_png: Some(include_bytes!("../icons/window-256.png")),
    accent: Some([0x0B, 0x7F, 0x72]),
    ..craft_update::App::new(
        "coursecraft",
        "CourseCraft",
        "eLearning authoring",
        "ad1-kamath/coursecraft",
    )
};

/// The updater and whether this run checks at start.
pub struct Runtime {
    updater: Updater,
    /// The desktop host turns the check at start on (preference on, not CI); tests leave it off.
    enabled: bool,
    check_requested: bool,
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            updater: Updater::new(&UPDATE_APP, VERSION).with_translator(translate),
            enabled: false,
            check_requested: false,
        }
    }
}

/// The prompt's words in the interface language, through the editor's catalogs.
fn translate(text: &str, args: &[(&str, &str)]) -> String {
    coursecraft_ui_egui::i18n::translate_with(text, args)
}

impl Runtime {
    /// Turns the check at start on or off.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Help ▸ Check for Updates: runs a check and reports the outcome either way.
    pub fn request_check(&mut self) {
        if !matches!(self.updater.status(), Status::Checking) {
            self.check_requested = true;
        }
    }

    /// Starts the helper of an update that is ready, so that it installs once the process has
    /// exited. Later calls do nothing.
    pub fn start_pending_update(&mut self) {
        self.updater.start_pending_update();
    }

    /// Runs the checks and the installation worker; call once a frame.
    pub fn tick(&mut self, ctx: &egui::Context) {
        if std::mem::take(&mut self.check_requested) {
            self.updater.check_now(ctx);
        } else if self.enabled {
            self.updater.check_at_start(ctx);
        }
        self.updater.tick(ctx);
    }

    /// Draws the prompt and the result window. `clean`: nothing is unsaved, so an installation
    /// may start and a ready update may ask the app to close.
    pub fn show(&mut self, ctx: &egui::Context, clean: bool) {
        match self.updater.show_if(ctx, clean) {
            Action::OpenUrl(url) => ctx.open_url(egui::OpenUrl::new_tab(url)),
            // The app's close path asks about unsaved work; `on_exit` starts the helper.
            Action::Close => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            Action::None => {}
        }
        if !matches!(self.updater.status(), Status::Idle) {
            let response = CheckResultDialog::new(&UPDATE_APP, VERSION)
                .with_translator(translate)
                .show(ctx, self.updater.status());
            self.answer(response);
        }
    }

    /// Acts on the result window's answer.
    fn answer(&mut self, response: CheckResultResponse) {
        match response {
            CheckResultResponse::Close => self.updater.dismiss_status(),
            CheckResultResponse::CheckAgain => self.check_requested = true,
            // The next frame's `show_if` asks the window to close once nothing is unsaved.
            CheckResultResponse::RestartToUpdate => self.updater.restart_to_update(),
            CheckResultResponse::None => {}
        }
    }
}

/// Whether `CI` or `COURSECRAFT_DISABLE_UPDATES` turn the check at start off.
#[must_use]
pub fn suppressed_by_env() -> bool {
    let set = |name: &str| {
        std::env::var(name).is_ok_and(|v| {
            let v = v.trim();
            !v.is_empty() && v != "0" && !v.eq_ignore_ascii_case("false")
        })
    };
    set("CI") || set("COURSECRAFT_DISABLE_UPDATES")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A runtime whose updater keeps its records under `root`, reads no launcher choice and asks
    /// no server, with the check at start on.
    fn runtime_with_records(root: &std::path::Path) -> Runtime {
        Runtime {
            updater: Updater::new(&UPDATE_APP, VERSION)
                .with_paths(craft_update::Paths::under(root))
                .with_pins_path(root.join("pins.json"))
                .with_source(std::sync::Arc::new(|| Ok(None))),
            enabled: true,
            check_requested: false,
        }
    }

    fn records_dir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("coursecraft-updates-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn frame(rt: &mut Runtime, ctx: &egui::Context) {
        ctx.run_ui(egui::RawInput::default(), |ui| {
            rt.tick(ui.ctx());
            rt.show(ui.ctx(), true);
        })
        .textures_delta
        .clear();
    }

    #[test]
    fn the_app_is_described_by_its_upstream_repository() {
        assert_eq!(UPDATE_APP.repo, "ad1-kamath/coursecraft");
        assert_eq!(
            UPDATE_APP.releases_url(),
            "https://github.com/ad1-kamath/coursecraft/releases"
        );
    }

    #[test]
    fn an_offered_release_opens_the_prompt() {
        let mut rt = Runtime::default();
        let ctx = egui::Context::default();
        assert!(!rt.updater.prompt_open());
        rt.updater.offer(craft_update::Release {
            tag: "v99.0.0".into(),
            ..Default::default()
        });
        ctx.run_ui(egui::RawInput::default(), |ui| rt.show(ui.ctx(), true))
            .textures_delta
            .clear();
        assert!(rt.updater.prompt_open());
    }

    #[test]
    fn the_start_after_an_update_reports_it_until_dismissed() {
        let root = records_dir("installed");
        std::fs::create_dir_all(root.join("data")).unwrap();
        std::fs::write(root.join("data").join("self-update-done.txt"), VERSION).unwrap();
        let mut rt = runtime_with_records(&root);
        let ctx = egui::Context::default();
        frame(&mut rt, &ctx);
        assert!(
            matches!(rt.updater.status(), Status::Updated(_)),
            "{:?}",
            rt.updater.status()
        );
        rt.answer(CheckResultResponse::Close);
        assert_eq!(rt.updater.status(), &Status::Idle);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_requested_check_with_nothing_newer_says_so() {
        let root = records_dir("uptodate");
        let mut rt = runtime_with_records(&root);
        rt.enabled = false;
        let ctx = egui::Context::default();
        rt.request_check();
        for _ in 0..200 {
            frame(&mut rt, &ctx);
            if !matches!(rt.updater.status(), Status::Checking) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(rt.updater.status(), &Status::UpToDate);
        rt.answer(CheckResultResponse::CheckAgain);
        assert!(rt.check_requested);
        let _ = std::fs::remove_dir_all(&root);
    }
}
