//! M35 Step S7 (**UI-06**): End-to-end desktop app shell tests for the update
//! checker and verified downloader, driving the real `EditorApp` headlessly
//! against an in-memory fake [`UpdateHttp`] with temp `config_dir` and
//! `downloads_dir` (zero live network calls, zero writes outside temp dirs).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use coursecraft_course::Course;
use coursecraft_ui_egui::EditorRequest;
use coursecraft_ui_egui::update_dialog::{UpdateCheckStatus, UpdateDownloadState};
use egui::{Context, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, pos2, vec2};

use crate::prefs::{load_prefs, save_prefs};
use crate::updates::{
    HttpResponse, RequestKind, UpdateBusy, UpdateClient, UpdateError, UpdateHttp,
};
use crate::{EditorApp, RestorePrompt};

const HELLO_JSON: &str = include_str!("../../../examples/hello-course/course.json");
const ASSET_URL: &str = "https://github.com/ad1-kamath/coursecraft/releases/download/v0.4.4/CourseCraft-0.4.4-macos.dmg";
const ASSET_CDN_URL: &str = "https://objects.githubusercontent.com/github-production-release-asset/CourseCraft-0.4.4-macos.dmg";

struct TempDirs {
    root: PathBuf,
    config: PathBuf,
    downloads: PathBuf,
}

impl TempDirs {
    fn new(name: &str) -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/agy-scratch")
            .join(format!("m35-ui06-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let config = root.join("config");
        let downloads = root.join("downloads");
        std::fs::create_dir_all(&config).expect("create temp config dir");
        std::fs::create_dir_all(&downloads).expect("create temp downloads dir");
        Self {
            root,
            config,
            downloads,
        }
    }
}

impl Drop for TempDirs {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = ring::digest::digest(&ring::digest::SHA256, bytes);
    let mut out = String::with_capacity(digest.as_ref().len() * 2);
    for &b in digest.as_ref() {
        out.push(char::from(HEX[usize::from(b >> 4)]));
        out.push(char::from(HEX[usize::from(b & 0x0f)]));
    }
    out
}

fn release_json(version: &str, payload: &[u8]) -> Vec<u8> {
    let digest = sha256_hex(payload);
    format!(
        r#"[
          {{
            "id": 220001,
            "tag_name": "v{version}",
            "name": "CourseCraft {version}",
            "body": "**What's new in {version}**\n\n- Smoother canvas zooming\n- Faster SCORM packaging\n",
            "html_url": "https://github.com/ad1-kamath/coursecraft/releases/tag/v{version}",
            "published_at": "2026-11-20T12:00:00Z",
            "draft": false,
            "prerelease": true,
            "assets": [
              {{
                "id": 9001,
                "name": "CourseCraft-{version}-macos.dmg",
                "browser_download_url": "https://github.com/ad1-kamath/coursecraft/releases/download/v{version}/CourseCraft-{version}-macos.dmg",
                "size": {size},
                "digest": "sha256:{digest}"
              }}
            ]
          }}
        ]"#,
        size = payload.len()
    )
    .into_bytes()
}

type Handler = Box<dyn Fn(&str, RequestKind) -> Result<HttpResponse, UpdateError> + Send + Sync>;

struct FakeHttp {
    calls: AtomicUsize,
    urls: Mutex<Vec<String>>,
    handler: Handler,
}

impl FakeHttp {
    fn new<F>(handler: F) -> Arc<Self>
    where
        F: Fn(&str, RequestKind) -> Result<HttpResponse, UpdateError> + Send + Sync + 'static,
    {
        Arc::new(Self {
            calls: AtomicUsize::new(0),
            urls: Mutex::new(Vec::new()),
            handler: Box::new(handler),
        })
    }

    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    fn recorded_urls(&self) -> Vec<String> {
        self.urls.lock().expect("lock urls").clone()
    }
}

impl UpdateHttp for FakeHttp {
    fn get(
        &self,
        url: &str,
        _accept: &str,
        _user_agent: &str,
        kind: RequestKind,
    ) -> Result<HttpResponse, UpdateError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.urls.lock().expect("lock urls").push(url.to_owned());
        (self.handler)(url, kind)
    }
}

fn ok_response(body: Vec<u8>) -> HttpResponse {
    let len = body.len() as u64;
    HttpResponse {
        status: 200,
        location: None,
        content_length: Some(len),
        rate_limit_remaining: Some(59),
        body: Box::new(Cursor::new(body)),
    }
}

fn redirect_response(location: &str) -> HttpResponse {
    HttpResponse {
        status: 302,
        location: Some(location.to_owned()),
        content_length: None,
        rate_limit_remaining: None,
        body: Box::new(Cursor::new(Vec::new())),
    }
}

struct AppHarness {
    ctx: Context,
    app: EditorApp,
    screen_rect: Rect,
    time: f64,
}

impl AppHarness {
    fn new(config_dir: PathBuf, downloads_dir: PathBuf, http: Arc<dyn UpdateHttp>) -> Self {
        let ctx = Context::default();
        coursecraft_ui_egui::coursecraft_render::apply_style(
            &ctx,
            coursecraft_ui_egui::coursecraft_render::ThemeMode::Dark,
        );
        let mut style = (*ctx.global_style()).clone();
        style.animation_time = 0.0;
        ctx.set_global_style(style);

        let course = Course::from_json(HELLO_JSON).expect("parse hello-course");
        let mut app = EditorApp::new_desktop(course, Some(config_dir));
        app.editor.ui.show_welcome = false;
        app.editor.ui.has_loaded_course = true;
        app.updates = UpdateClient::with_http_and_env(
            http,
            semver::Version::new(0, 4, 3),
            "macos",
            "aarch64",
            false,
        );
        app.test_downloads_dir = Some(downloads_dir);
        if let Ok(mut slot) = app.repaint_ctx.lock() {
            *slot = Some(ctx.clone());
        }

        Self {
            ctx,
            app,
            screen_rect: Rect::from_min_size(pos2(0.0, 0.0), vec2(1280.0, 900.0)),
            time: 1.0,
        }
    }

    fn next_raw_input(&mut self, events: Vec<Event>) -> RawInput {
        let ctx_time = self.ctx.input(|i| i.time);
        if ctx_time > self.time {
            self.time = ctx_time + 1.0 / 60.0;
        }
        let time = self.time;
        self.time += 1.0 / 60.0;
        RawInput {
            screen_rect: Some(self.screen_rect),
            time: Some(time),
            events,
            ..Default::default()
        }
    }

    fn frame(&mut self, events: Vec<Event>) -> egui::FullOutput {
        let raw_input = self.next_raw_input(events);
        self.ctx.run_ui(raw_input, |ui| {
            self.app.show_app(ui);
        })
    }

    fn idle_frame(&mut self) {
        let out = self.frame(Vec::new());
        out.drop_without_applying_deltas();
    }

    fn wait_for_updates_idle(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while self.app.updates.busy() != UpdateBusy::Idle && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
            self.idle_frame();
        }
        self.idle_frame();
        assert_eq!(
            self.app.updates.busy(),
            UpdateBusy::Idle,
            "background update worker did not finish within timeout"
        );
    }

    fn find_texts(&mut self, label: &str) -> Vec<Pos2> {
        fn walk(shape: &egui::epaint::Shape, label: &str, out: &mut Vec<Pos2>) {
            match shape {
                egui::epaint::Shape::Text(t) if t.galley.text() == label => {
                    let r = t.visual_bounding_rect();
                    let center = if r.is_positive()
                        && r.center().x.is_finite()
                        && r.center().y.is_finite()
                    {
                        r.center()
                    } else {
                        t.galley.rect.translate(t.pos.to_vec2()).center()
                    };
                    out.push(center);
                }
                egui::epaint::Shape::Vec(v) => {
                    for s in v {
                        walk(s, label, out);
                    }
                }
                _ => {}
            }
        }
        let out = self.frame(Vec::new());
        let mut found = Vec::new();
        for clipped in &out.shapes {
            walk(&clipped.shape, label, &mut found);
        }
        out.drop_without_applying_deltas();
        found
    }

    fn click_at(&mut self, pos: Pos2) {
        let out1 = self.frame(vec![
            Event::PointerMoved(pos),
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ]);
        out1.drop_without_applying_deltas();
        let out2 = self.frame(vec![
            Event::PointerMoved(pos),
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::NONE,
            },
        ]);
        out2.drop_without_applying_deltas();
        self.idle_frame();
    }

    fn click_button(&mut self, label: &str) {
        self.idle_frame();
        let pos = self
            .find_texts(label)
            .into_iter()
            .last()
            .unwrap_or_else(|| panic!("button/text '{label}' not found in UI"));
        self.click_at(pos);
    }
}

fn dir_entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("read dir")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn ui06_silent_startup_offline_stays_silent_and_keeps_timestamp_unset() {
    let dirs = TempDirs::new("offline");
    let http = FakeHttp::new(|_, _| Err(UpdateError::Network("offline".to_owned())));
    let mut h = AppHarness::new(dirs.config.clone(), dirs.downloads.clone(), http.clone());

    // Desktop capability is enabled, and cfg!(test) keeps startup() from touching the network.
    assert!(h.app.editor.updates_available);
    h.app.startup(None, true);
    h.idle_frame();
    assert_eq!(
        http.call_count(),
        0,
        "startup() must never auto-check in cfg!(test)"
    );

    // Explicitly invoke the silent startup helper as production startup would.
    let ctx = h.ctx.clone();
    h.app.maybe_start_auto_update_check(&ctx, 1_700_000_000);
    h.wait_for_updates_idle();

    assert_eq!(http.call_count(), 1);
    assert!(!h.app.editor.ui.update.open);
    assert_eq!(h.app.editor.ui.update.status, UpdateCheckStatus::Idle);
    assert!(!h.app.editor.ui.update.has_available_hint());
    assert_eq!(h.app.prefs.last_update_check_unix, None);
    assert_eq!(load_prefs(&dirs.config).last_update_check_unix, None);
}

#[test]
fn ui06_silent_startup_opens_dialog_only_when_idle_rev0_and_no_prompts() {
    let payload = b"fake-dmg-bytes-for-0.4.4";
    let body = release_json("0.4.4", payload);

    // Case 1: Idle at revision 0 with no crash/restore prompts -> dialog opens automatically.
    {
        let dirs = TempDirs::new("idle-rev0");
        let body_clone = body.clone();
        let http = FakeHttp::new(move |_, _| Ok(ok_response(body_clone.clone())));
        let mut h = AppHarness::new(dirs.config.clone(), dirs.downloads.clone(), http);
        assert_eq!(h.app.editor.session.revision, 0);

        let ctx = h.ctx.clone();
        h.app.maybe_start_auto_update_check(&ctx, 1_700_000_000);
        h.wait_for_updates_idle();

        assert!(h.app.editor.ui.update.open);
        assert!(h.app.editor.ui.update.has_available_hint());
        assert_eq!(
            h.app
                .editor
                .ui
                .update
                .available_info()
                .map(|i| i.version.as_str()),
            Some("0.4.4")
        );
        assert!(h.app.prefs.last_update_check_unix.is_some());
        assert!(load_prefs(&dirs.config).last_update_check_unix.is_some());
    }

    // Case 2: Active editing (revision > 0) -> dialog stays closed, hint only.
    {
        let dirs = TempDirs::new("editing-rev1");
        let body_clone = body.clone();
        let http = FakeHttp::new(move |_, _| Ok(ok_response(body_clone.clone())));
        let mut h = AppHarness::new(dirs.config.clone(), dirs.downloads.clone(), http);
        let slide_id = h.app.editor.session.course.scenes[0].slides[0].id.clone();
        assert!(
            h.app
                .editor
                .run(
                    "object.add",
                    serde_json::json!({
                        "slide": slide_id,
                        "id": "m35_edit_obj",
                        "kind": { "type": "text", "text": "Editing" },
                        "frame": { "x": 10.0, "y": 10.0, "w": 120.0, "h": 40.0 }
                    }),
                )
                .is_some()
        );
        assert!(h.app.editor.session.revision > 0);

        let ctx = h.ctx.clone();
        h.app.maybe_start_auto_update_check(&ctx, 1_700_000_000);
        h.wait_for_updates_idle();

        assert!(!h.app.editor.ui.update.open);
        assert!(h.app.editor.ui.update.has_available_hint());
    }

    // Case 3: Crash prompt active -> dialog stays closed, hint only.
    {
        let dirs = TempDirs::new("crash-prompt");
        let body_clone = body.clone();
        let http = FakeHttp::new(move |_, _| Ok(ok_response(body_clone.clone())));
        let mut h = AppHarness::new(dirs.config.clone(), dirs.downloads.clone(), http);
        h.app.editor.ui.report.crash_prompt = true;

        let ctx = h.ctx.clone();
        h.app.maybe_start_auto_update_check(&ctx, 1_700_000_000);
        h.wait_for_updates_idle();

        assert!(!h.app.editor.ui.update.open);
        assert!(h.app.editor.ui.update.has_available_hint());
    }

    // Case 4: Restore prompt active -> dialog stays closed, hint only.
    {
        let dirs = TempDirs::new("restore-prompt");
        let body_clone = body;
        let http = FakeHttp::new(move |_, _| Ok(ok_response(body_clone.clone())));
        let mut h = AppHarness::new(dirs.config.clone(), dirs.downloads.clone(), http);
        h.app.restore_prompt = Some(RestorePrompt {
            file_path: None,
            autosave_path: dirs.config.join("untitled.autosave.ccraft"),
            time_str: "just now".to_owned(),
        });

        let ctx = h.ctx.clone();
        h.app.maybe_start_auto_update_check(&ctx, 1_700_000_000);
        h.wait_for_updates_idle();

        assert!(!h.app.editor.ui.update.open);
        assert!(h.app.editor.ui.update.has_available_hint());
    }
}

#[test]
fn ui06_skip_version_persists_suppresses_silent_check_but_manual_shows_it() {
    let dirs = TempDirs::new("skip-version");
    let body = release_json("0.4.4", b"dmg-payload");
    let body_clone = body.clone();
    let http = FakeHttp::new(move |_, _| Ok(ok_response(body_clone.clone())));

    // First session: silent check finds 0.4.4, user clicks "Skip this version".
    {
        let mut h = AppHarness::new(dirs.config.clone(), dirs.downloads.clone(), http.clone());
        let ctx = h.ctx.clone();
        h.app.maybe_start_auto_update_check(&ctx, 1_700_000_000);
        h.wait_for_updates_idle();
        assert!(h.app.editor.ui.update.open);

        h.click_button("Skip this version");
        assert!(!h.app.editor.ui.update.open);
        assert!(!h.app.editor.ui.update.has_available_hint());
        assert_eq!(h.app.prefs.skipped_update_version.as_deref(), Some("0.4.4"));
        assert_eq!(
            load_prefs(&dirs.config).skipped_update_version.as_deref(),
            Some("0.4.4")
        );
    }

    // Second session (next day): silent check is completely suppressed for 0.4.4,
    // while a manual CheckForUpdates still displays 0.4.4.
    {
        let mut h = AppHarness::new(dirs.config.clone(), dirs.downloads.clone(), http.clone());
        let ctx = h.ctx.clone();
        let next_day = h
            .app
            .prefs
            .last_update_check_unix
            .unwrap_or(0)
            .saturating_add(90_000);
        h.app.maybe_start_auto_update_check(&ctx, next_day);
        h.wait_for_updates_idle();

        assert_eq!(http.call_count(), 2);
        assert!(!h.app.editor.ui.update.open);
        assert!(!h.app.editor.ui.update.has_available_hint());
        assert!(h.app.available_update.is_none());

        // Manual check must still show the skipped version.
        h.app.handle_request(EditorRequest::CheckForUpdates, &ctx);
        h.wait_for_updates_idle();

        assert_eq!(http.call_count(), 3);
        assert!(h.app.editor.ui.update.open);
        assert_eq!(
            h.app
                .editor
                .ui
                .update
                .available_info()
                .map(|i| i.version.as_str()),
            Some("0.4.4")
        );
        assert_eq!(h.app.editor.ui.update.dismissed_version, None);
    }
}

#[test]
fn ui06_auto_check_opt_out_skips_startup_and_persists_pref() {
    let dirs = TempDirs::new("opt-out");
    let body = release_json("0.4.4", b"dmg-payload");
    let http = FakeHttp::new(move |_, _| Ok(ok_response(body.clone())));

    // First session: user turns off automatic checks via Preferences request.
    {
        let mut h = AppHarness::new(dirs.config.clone(), dirs.downloads.clone(), http.clone());
        assert!(h.app.prefs.auto_check_updates);
        assert!(h.app.editor.ui.prefs.auto_check_updates);

        let ctx = h.ctx.clone();
        h.app
            .handle_request(EditorRequest::SetAutoCheckUpdates(false), &ctx);
        assert!(!h.app.prefs.auto_check_updates);
        assert!(!h.app.editor.ui.prefs.auto_check_updates);
        assert!(!load_prefs(&dirs.config).auto_check_updates);
    }

    // Fresh session loads persisted opt-out and performs zero requests on startup check.
    {
        let mut persisted = load_prefs(&dirs.config);
        persisted.last_update_check_unix = None;
        save_prefs(&dirs.config, &persisted).expect("save prefs");

        let mut h = AppHarness::new(dirs.config.clone(), dirs.downloads.clone(), http.clone());
        assert!(!h.app.prefs.auto_check_updates);
        assert!(!h.app.editor.ui.prefs.auto_check_updates);

        let ctx = h.ctx.clone();
        h.app.maybe_start_auto_update_check(&ctx, 1_700_000_000);
        h.wait_for_updates_idle();

        assert_eq!(http.call_count(), 0);
        assert!(!h.app.editor.ui.update.open);
        assert_eq!(h.app.editor.ui.update.status, UpdateCheckStatus::Idle);
    }
}

#[test]
fn ui06_download_verify_writes_to_temp_downloads_with_no_part_left() {
    let dirs = TempDirs::new("download-verify");
    let payload: Vec<u8> = (0..180_000_u32).map(|i| (i % 251) as u8).collect();
    let api_body = release_json("0.4.4", &payload);
    let download_payload = payload.clone();

    let http = FakeHttp::new(move |url, kind| match kind {
        RequestKind::Metadata => Ok(ok_response(api_body.clone())),
        RequestKind::Download if url == ASSET_URL => Ok(redirect_response(ASSET_CDN_URL)),
        RequestKind::Download if url == ASSET_CDN_URL => Ok(ok_response(download_payload.clone())),
        _ => Err(UpdateError::HttpStatus(404)),
    });

    let mut h = AppHarness::new(dirs.config.clone(), dirs.downloads.clone(), http.clone());
    let ctx = h.ctx.clone();
    h.app.handle_request(EditorRequest::CheckForUpdates, &ctx);
    h.wait_for_updates_idle();
    assert!(h.app.editor.ui.update.open);

    // Click "Download update" in the live Software update dialog.
    h.click_button("Download update");
    h.wait_for_updates_idle();

    assert_eq!(
        h.app.editor.ui.update.download,
        UpdateDownloadState::Verified {
            filename: "CourseCraft-0.4.4-macos.dmg".to_owned(),
            folder_display: "Downloads".to_owned(),
            sha256_verified: true,
        }
    );

    let expected_path = dirs.downloads.join("CourseCraft-0.4.4-macos.dmg");
    assert_eq!(
        h.app.downloaded_update_path.as_deref(),
        Some(expected_path.as_path())
    );
    let written = std::fs::read(&expected_path).expect("read verified installer");
    assert_eq!(written, payload);

    let entries = dir_entries(&dirs.downloads);
    assert_eq!(entries, vec!["CourseCraft-0.4.4-macos.dmg".to_owned()]);
    assert!(
        !entries.iter().any(|name| name.ends_with(".part")),
        "no .part temp file may remain after verified download: {entries:?}"
    );

    // Verify redirect chain was exercised and dialog only offers "Show in folder" (never auto-runs).
    let urls = http.recorded_urls();
    assert!(urls.iter().any(|u| u == ASSET_URL));
    assert!(urls.iter().any(|u| u == ASSET_CDN_URL));
    assert!(
        !h.find_texts("Show in folder").is_empty(),
        "verified state should offer Show in folder"
    );
    assert!(
        h.find_texts("Open disk image").is_empty(),
        "must never offer an auto-open installer button"
    );
}

#[test]
fn ui06_rate_limited_silent_startup_backs_off_without_showing_error() {
    let dirs = TempDirs::new("rate-limit");
    let http = FakeHttp::new(|_, _| {
        Ok(HttpResponse {
            status: 403,
            location: None,
            content_length: None,
            rate_limit_remaining: Some(0),
            body: Box::new(Cursor::new(Vec::new())),
        })
    });
    let mut h = AppHarness::new(dirs.config.clone(), dirs.downloads.clone(), http);
    let ctx = h.ctx.clone();
    h.app.maybe_start_auto_update_check(&ctx, 1_700_000_000);
    h.wait_for_updates_idle();

    assert!(!h.app.editor.ui.update.open);
    assert_eq!(h.app.editor.ui.update.status, UpdateCheckStatus::Idle);
    assert!(h.app.prefs.last_update_check_unix.is_some());
    assert!(load_prefs(&dirs.config).last_update_check_unix.is_some());
}

#[test]
fn ui06_manual_check_promotes_in_flight_silent_check_without_hanging() {
    // Case 1: In-flight silent startup check finishes UpToDate after manual check opened the dialog.
    {
        let dirs = TempDirs::new("promote-up-to-date");
        let (entered_tx, entered_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let release_rx = Mutex::new(release_rx);
        let body = release_json("0.4.3", b"current");
        let http = FakeHttp::new(move |_, _| {
            let _ = entered_tx.send(());
            let _ = release_rx
                .lock()
                .expect("lock release_rx")
                .recv_timeout(Duration::from_secs(5));
            Ok(ok_response(body.clone()))
        });
        let mut h = AppHarness::new(dirs.config.clone(), dirs.downloads.clone(), http.clone());
        let ctx = h.ctx.clone();

        h.app.maybe_start_auto_update_check(&ctx, 1_700_000_000);
        entered_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("silent check should start");
        assert_eq!(h.app.updates.busy(), UpdateBusy::Checking);

        // User manually clicks Help -> Check for updates… while the silent check is still running.
        h.app.handle_request(EditorRequest::CheckForUpdates, &ctx);
        assert!(h.app.editor.ui.update.open);
        assert_eq!(h.app.editor.ui.update.status, UpdateCheckStatus::Checking);

        release_tx.send(()).expect("release silent check");
        h.wait_for_updates_idle();

        assert_eq!(http.call_count(), 1);
        assert!(h.app.editor.ui.update.open);
        assert_eq!(h.app.editor.ui.update.status, UpdateCheckStatus::UpToDate);
    }

    // Case 2: In-flight silent startup check fails offline after manual check opened the dialog.
    {
        let dirs = TempDirs::new("promote-error");
        let (entered_tx, entered_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let release_rx = Mutex::new(release_rx);
        let http = FakeHttp::new(move |_, _| {
            let _ = entered_tx.send(());
            let _ = release_rx
                .lock()
                .expect("lock release_rx")
                .recv_timeout(Duration::from_secs(5));
            Err(UpdateError::Network("offline".to_owned()))
        });
        let mut h = AppHarness::new(dirs.config.clone(), dirs.downloads.clone(), http.clone());
        let ctx = h.ctx.clone();

        h.app.maybe_start_auto_update_check(&ctx, 1_700_000_000);
        entered_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("silent check should start");

        h.app.handle_request(EditorRequest::CheckForUpdates, &ctx);
        assert!(h.app.editor.ui.update.open);
        assert_eq!(h.app.editor.ui.update.status, UpdateCheckStatus::Checking);

        release_tx.send(()).expect("release silent check");
        h.wait_for_updates_idle();

        assert_eq!(http.call_count(), 1);
        assert!(h.app.editor.ui.update.open);
        assert!(matches!(
            h.app.editor.ui.update.status,
            UpdateCheckStatus::Error(_)
        ));
    }
}
