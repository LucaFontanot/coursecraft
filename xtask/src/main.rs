//! `cargo xtask <task>`. Tasks: `layers` (enforce the crate layering), `wasm`
//! (L0–L5 and the player build for `wasm32-unknown-unknown`), `ci`
//! (fmt + clippy + test + layers + wasm), `dep-src` (copy the sources of key
//! dependencies into `target/dep-src/` for reading, e.g. by coding agents that
//! may only read inside the workspace), `build-player` (build the release web
//! player into `target/player-dist/` for the desktop editor to embed),
//! `package-example` (build the web player with trunk, publish
//! `examples/hello-course` into the mock LMS at `target/mock-lms/` and to
//! `target/hello-course.zip`; also publishes the showcase as SCORM 2004 to
//! `target/mock-lms/scorm2004/` and `target/showcase-2004.zip`), `perf` (run
//! the ignored timing tests in release and write `target/perf/report.md`) and
//! `mock-ai` (a local OpenAI-compatible endpoint for trying AI features offline).

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod mock_ai;
mod mock_lrs;

use std::fmt::Write as _;
use std::path::Path;
use std::process::{Command, ExitCode};

use serde_json::Value;

/// Layer of every workspace library crate (name without `coursecraft-`).
/// A crate may only depend on crates in strictly lower layers; a standalone
/// crate may depend on no workspace crate at all.
const LAYERS: &[(&str, u8, bool)] = &[
    // (name, layer, standalone)
    ("scorm", 0, true),
    ("spell", 0, true),
    ("xapi", 0, true),
    ("course", 1, false),
    ("strict-lms", 1, false),
    ("video-native", 1, true),
    ("logic", 2, false),
    ("media", 2, true),
    ("playback", 2, false),
    ("render", 3, false),
    ("ai", 3, false),
    ("format", 3, false),
    ("pptx", 3, false),
    ("lms", 4, false),
    ("lti", 4, true),
    ("publish", 4, false),
    ("export", 4, false),
    ("engine", 5, false),
    ("ui-egui", 6, false),
    ("control", 6, false),
];

/// Binaries and tooling are exempt.
const EXEMPT: &[&str] = &[
    "coursecraft",
    "coursecraft-cli",
    "coursecraft-lti-server",
    "coursecraft-mcp",
    "coursecraft-player",
    "coursecraft-strict-lms-wasm",
    "xtask",
];

fn layer_of(pkg: &str) -> Option<(u8, bool)> {
    let short = pkg.strip_prefix("coursecraft-")?;
    LAYERS
        .iter()
        .find(|(n, ..)| *n == short)
        .map(|(_, l, s)| (*l, *s))
}

fn check_layers() -> Result<(), Vec<String>> {
    let out = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
        .map_err(|e| vec![format!("cargo metadata: {e}")])?;
    let meta: Value = serde_json::from_slice(&out.stdout)
        .map_err(|e| vec![format!("cargo metadata json: {e}")])?;
    let Some(packages) = meta["packages"].as_array() else {
        return Err(vec!["cargo metadata: no packages".into()]);
    };
    let mut problems = Vec::new();
    for pkg in packages {
        let name = pkg["name"].as_str().unwrap_or_default();
        if EXEMPT.contains(&name) {
            continue;
        }
        let Some((layer, standalone)) = layer_of(name) else {
            problems.push(format!(
                "{name}: not registered in xtask/src/main.rs LAYERS"
            ));
            continue;
        };
        for dep in pkg["dependencies"].as_array().into_iter().flatten() {
            let dep_name = dep["name"].as_str().unwrap_or_default();
            if dep["kind"].as_str() == Some("dev") || !dep_name.starts_with("coursecraft-") {
                continue;
            }
            if standalone {
                problems.push(format!(
                    "{name} (standalone) depends on workspace crate {dep_name}"
                ));
                continue;
            }
            match layer_of(dep_name) {
                Some((dl, _)) if dl < layer => {}
                Some((dl, _)) => problems.push(format!(
                    "{name} (L{layer}) depends on {dep_name} (L{dl}): only lower layers allowed"
                )),
                None => problems.push(format!(
                    "{name} depends on unregistered or exempt crate {dep_name}"
                )),
            }
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems)
    }
}

fn cargo(args: &[&str]) -> bool {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    println!("$ cargo {}", args.join(" "));
    Command::new(cargo)
        .args(args)
        .status()
        .is_ok_and(|s| s.success())
}

/// Crates that must keep building for the browser.
const WASM_CRATES: &[&str] = &[
    "coursecraft-scorm",
    "coursecraft-strict-lms",
    "coursecraft-course",
    "coursecraft-logic",
    "coursecraft-media",
    "coursecraft-playback",
    "coursecraft-render",
    "coursecraft-ai",
    "coursecraft-format",
    "coursecraft-pptx",
    "coursecraft-lms",
    "coursecraft-publish",
    "coursecraft-export",
    "coursecraft-engine",
    "coursecraft-ui-egui",
    "coursecraft-player",
    "coursecraft",
];

fn check_wasm() -> bool {
    let mut args = vec!["check", "--target", "wasm32-unknown-unknown"];
    for c in WASM_CRATES {
        args.extend(["-p", c]);
    }
    cargo(&args)
}

/// Dependencies copied whole by `dep-src`.
const DEP_SRC_CRATES: &[&str] = &[
    "image",
    "rodio",
    "cpal",
    "egui",
    "epaint",
    "eframe",
    "emath",
    "ecolor",
    "egui-wgpu",
    "egui-winit",
    "zip",
    "quick-xml",
    "js-sys",
    "wasm-bindgen",
    "wasm-bindgen-futures",
    "ehttp",
    "rfd",
    "objc2",
    "objc2-foundation",
    "objc2-core-foundation",
    "objc2-core-media",
    "objc2-core-video",
    "objc2-av-foundation",
    "windows",
    "skrifa",
    "read-fonts",
    "resvg",
    "usvg",
    "tiny-skia",
    "velato",
    "kurbo",
    "peniko",
];

/// Single `web-sys` binding files copied by `dep-src` (the crate is huge).
const DEP_SRC_WEB_SYS: &[&str] = &[
    "gen_Window.rs",
    "gen_Document.rs",
    "gen_Performance.rs",
    "gen_EventTarget.rs",
    "gen_PageTransitionEvent.rs",
    "gen_console.rs",
    "gen_Location.rs",
    "gen_Element.rs",
    "gen_HtmlElement.rs",
    "gen_HtmlInputElement.rs",
    "gen_HtmlSelectElement.rs",
    "gen_HtmlOptionElement.rs",
    "gen_HtmlButtonElement.rs",
    "gen_HtmlImageElement.rs",
    "gen_Node.rs",
    "gen_Event.rs",
    "gen_FocusEvent.rs",
    "gen_KeyboardEvent.rs",
    "gen_CssStyleDeclaration.rs",
];

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn dep_src() -> Result<(), String> {
    let out = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .args(["metadata", "--format-version", "1"])
        .output()
        .map_err(|e| format!("cargo metadata: {e}"))?;
    let meta: Value =
        serde_json::from_slice(&out.stdout).map_err(|e| format!("cargo metadata json: {e}"))?;
    let target_dir = meta["target_directory"]
        .as_str()
        .ok_or("cargo metadata: no target_directory")?;
    let dest = Path::new(target_dir).join("dep-src");
    for pkg in meta["packages"].as_array().into_iter().flatten() {
        let (Some(name), Some(version), Some(manifest)) = (
            pkg["name"].as_str(),
            pkg["version"].as_str(),
            pkg["manifest_path"].as_str(),
        ) else {
            continue;
        };
        let Some(root) = Path::new(manifest).parent() else {
            continue;
        };
        let to = dest.join(format!("{name}-{version}"));
        if DEP_SRC_CRATES.contains(&name) {
            copy_dir(root, &to).map_err(|e| format!("{name}: {e}"))?;
            println!("dep-src: {name}-{version}");
        } else if name == "web-sys" {
            let features = to.join("src/features");
            std::fs::create_dir_all(&features).map_err(|e| format!("web-sys: {e}"))?;
            for file in DEP_SRC_WEB_SYS {
                std::fs::copy(root.join("src/features").join(file), features.join(file))
                    .map_err(|e| format!("web-sys {file}: {e}"))?;
            }
            println!("dep-src: {name}-{version} (selected bindings)");
        }
    }
    Ok(())
}

/// Publish one course for one standard into `out_dir`, optionally zipping it.
fn publish_course(
    dist: &Path,
    course: &str,
    standard: &str,
    out_dir: &Path,
    zip: Option<&Path>,
) -> Result<(), String> {
    if out_dir.exists() {
        std::fs::remove_dir_all(out_dir).map_err(|e| format!("{}: {e}", out_dir.display()))?;
    }
    std::fs::create_dir_all(out_dir).map_err(|e| format!("{}: {e}", out_dir.display()))?;
    let dist_arg = dist.to_string_lossy().into_owned();
    let out_arg = out_dir.to_string_lossy().into_owned();
    let mut args: Vec<String> = vec![
        "run".into(),
        "-q".into(),
        "-p".into(),
        "coursecraft-cli".into(),
        "--".into(),
        "publish".into(),
        "--standard".into(),
        standard.into(),
        course.into(),
        "--player".into(),
        dist_arg,
        "--out-dir".into(),
        out_arg,
    ];
    if let Some(zip) = zip {
        args.push("-o".into());
        args.push(zip.to_string_lossy().into_owned());
    }
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    if !cargo(&refs) {
        return Err(format!(
            "coursecraft-cli publish failed for {course} ({standard})"
        ));
    }
    Ok(())
}

/// Delete the demo `course.json` copied by `index.html` for local `trunk serve`
/// so it is not embedded in desktop builds or published zips.
fn remove_bundled_demo_course(dist: &Path) -> Result<(), String> {
    let demo_course = dist.join("course.json");
    match std::fs::remove_file(&demo_course) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("{}: {err}", demo_course.display())),
    }
}

/// Build the release web player into `target/player-dist` for the desktop editor to embed.
fn build_player() -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("xtask has no parent directory")?;
    let player_dir = root.join("apps/coursecraft-player");
    let dist = root.join("target/player-dist");
    if dist.exists() {
        std::fs::remove_dir_all(&dist).map_err(|e| format!("{}: {e}", dist.display()))?;
    }
    println!("$ trunk build --release --dist {}", dist.display());
    let built = Command::new("trunk")
        .args(["build", "--release", "--dist"])
        .arg(&dist)
        .current_dir(&player_dir)
        .status()
        .map_err(|e| format!("trunk: {e} (install with `cargo install trunk`)"))?;
    if !built.success() {
        return Err("trunk build failed".into());
    }
    remove_bundled_demo_course(&dist)?;
    println!("build-player: player written to {}", dist.display());
    Ok(())
}

/// Build `xtask/strict-lms-wasm` into `target/mock-lms/strict-lms-wasm/` using `wasm-bindgen`.
fn build_strict_lms_wasm(root: &Path, mock: &Path) -> Result<(), String> {
    if !cargo(&[
        "build",
        "--release",
        "-p",
        "coursecraft-strict-lms-wasm",
        "--target",
        "wasm32-unknown-unknown",
    ]) {
        return Err("cargo build failed for coursecraft-strict-lms-wasm".into());
    }
    let wasm_in =
        root.join("target/wasm32-unknown-unknown/release/coursecraft_strict_lms_wasm.wasm");
    let out_dir = mock.join("strict-lms-wasm");
    if out_dir.exists() {
        std::fs::remove_dir_all(&out_dir).map_err(|e| format!("{}: {e}", out_dir.display()))?;
    }
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("{}: {e}", out_dir.display()))?;
    let out_arg = out_dir.to_string_lossy().into_owned();
    let wasm_arg = wasm_in.to_string_lossy().into_owned();
    println!("$ wasm-bindgen --target web --no-typescript --out-dir {out_arg} {wasm_arg}");
    let bound = Command::new("wasm-bindgen")
        .args([
            "--target",
            "web",
            "--no-typescript",
            "--out-dir",
            &out_arg,
            &wasm_arg,
        ])
        .status()
        .map_err(|e| format!("wasm-bindgen: {e}"))?;
    if !bound.success() {
        return Err("wasm-bindgen failed for coursecraft-strict-lms-wasm".into());
    }
    Ok(())
}

/// Build the release web player, publish hello-course and showcase into the mock LMS.
fn package_example() -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("xtask has no parent directory")?;
    let player_dir = root.join("apps/coursecraft-player");
    println!("$ trunk build --release (in apps/coursecraft-player)");
    let built = Command::new("trunk")
        .args(["build", "--release"])
        .current_dir(&player_dir)
        .status()
        .map_err(|e| format!("trunk: {e} (install with `cargo install trunk`)"))?;
    if !built.success() {
        return Err("trunk build failed".into());
    }
    let mock = root.join("target/mock-lms");
    let package = mock.join("package");
    if package.exists() {
        std::fs::remove_dir_all(&package).map_err(|e| format!("{}: {e}", package.display()))?;
    }
    std::fs::create_dir_all(&mock).map_err(|e| format!("{}: {e}", mock.display()))?;
    build_strict_lms_wasm(root, &mock)?;
    let dist = player_dir.join("dist");
    let package_arg = package.to_string_lossy().into_owned();
    let dist_arg = dist.to_string_lossy().into_owned();
    let zip_arg = root
        .join("target/hello-course.zip")
        .to_string_lossy()
        .into_owned();
    let published = cargo(&[
        "run",
        "-q",
        "-p",
        "coursecraft-cli",
        "--",
        "publish",
        "examples/hello-course/course.json",
        "--player",
        &dist_arg,
        "--out-dir",
        &package_arg,
        "-o",
        &zip_arg,
    ]);
    if !published {
        return Err("coursecraft-cli publish failed for hello-course".into());
    }
    std::fs::copy(
        root.join("xtask/mock-lms/index.html"),
        mock.join("index.html"),
    )
    .map_err(|e| format!("mock LMS page: {e}"))?;

    let showcase_mock = mock.join("showcase");
    let showcase_package = showcase_mock.join("package");
    if showcase_package.exists() {
        std::fs::remove_dir_all(&showcase_package)
            .map_err(|e| format!("{}: {e}", showcase_package.display()))?;
    }
    std::fs::create_dir_all(&showcase_mock)
        .map_err(|e| format!("{}: {e}", showcase_mock.display()))?;
    let showcase_package_arg = showcase_package.to_string_lossy().into_owned();
    let showcase_zip_arg = root
        .join("target/showcase.zip")
        .to_string_lossy()
        .into_owned();
    let published_showcase = cargo(&[
        "run",
        "-q",
        "-p",
        "coursecraft-cli",
        "--",
        "publish",
        "examples/showcase/course.json",
        "--player",
        &dist_arg,
        "--out-dir",
        &showcase_package_arg,
        "-o",
        &showcase_zip_arg,
    ]);
    if !published_showcase {
        return Err("coursecraft-cli publish failed for showcase".into());
    }
    std::fs::copy(
        root.join("xtask/mock-lms/index.html"),
        showcase_mock.join("index.html"),
    )
    .map_err(|e| format!("mock LMS showcase page: {e}"))?;

    let scorm2004_mock = mock.join("scorm2004");
    let scorm2004_package = scorm2004_mock.join("package");
    if scorm2004_package.exists() {
        std::fs::remove_dir_all(&scorm2004_package)
            .map_err(|e| format!("{}: {e}", scorm2004_package.display()))?;
    }
    std::fs::create_dir_all(&scorm2004_mock)
        .map_err(|e| format!("{}: {e}", scorm2004_mock.display()))?;
    let scorm2004_package_arg = scorm2004_package.to_string_lossy().into_owned();
    let scorm2004_zip_arg = root
        .join("target/showcase-2004.zip")
        .to_string_lossy()
        .into_owned();
    let published_2004 = cargo(&[
        "run",
        "-q",
        "-p",
        "coursecraft-cli",
        "--",
        "publish",
        "--standard",
        "scorm2004",
        "examples/showcase/course.json",
        "--player",
        &dist_arg,
        "--out-dir",
        &scorm2004_package_arg,
        "-o",
        &scorm2004_zip_arg,
    ]);
    if !published_2004 {
        return Err("coursecraft-cli publish failed for SCORM 2004 showcase".into());
    }
    std::fs::copy(
        root.join("xtask/mock-lms/index.html"),
        scorm2004_mock.join("index.html"),
    )
    .map_err(|e| format!("mock LMS SCORM 2004 page: {e}"))?;

    let mock_lrs = root.join("target/mock-lrs");
    std::fs::create_dir_all(&mock_lrs).map_err(|e| format!("{}: {e}", mock_lrs.display()))?;
    std::fs::copy(
        root.join("xtask/mock-lrs/index.html"),
        mock_lrs.join("index.html"),
    )
    .map_err(|e| format!("mock LRS page: {e}"))?;

    for flavor in ["cmi5", "xapi"] {
        let showcase = mock_lrs.join(flavor).join("showcase");
        let zip = root.join(format!("target/showcase-{flavor}.zip"));
        publish_course(
            &dist,
            "examples/showcase/course.json",
            flavor,
            &showcase,
            Some(&zip),
        )?;
        let hello = mock_lrs.join(flavor).join("hello");
        publish_course(
            &dist,
            "examples/hello-course/course.json",
            flavor,
            &hello,
            None,
        )?;
    }

    println!(
        "package-example: open target/mock-lms/index.html, /showcase/index.html or /scorm2004/index.html over http (launch config `mock-lms`); xAPI packages are under target/mock-lrs (launch config `mock-lrs`)"
    );
    Ok(())
}

fn parse_port(args: impl Iterator<Item = String>) -> Result<u16, String> {
    parse_port_with_default(args, 8091)
}

fn parse_port_with_default(
    args: impl Iterator<Item = String>,
    default: u16,
) -> Result<u16, String> {
    let mut port = default;
    let mut args = args;
    while let Some(arg) = args.next() {
        if arg == "--port" {
            let Some(value) = args.next() else {
                return Err("--port needs a value".into());
            };
            port = value
                .parse::<u16>()
                .map_err(|_| format!("invalid port {value:?}"))?;
        } else {
            return Err(format!("unknown argument {arg:?}"));
        }
    }
    Ok(port)
}

/// Turn TSV rows from the timing suites into the markdown report. Pure so it
/// can be unit tested without a release run; the caller supplies the date, OS
/// and CPU count for the header. A zero or negative budget means "not
/// budgeted" and is never marked over.
fn results_to_markdown(tsv: &str, date: &str, os: &str, cpus: usize) -> String {
    let mut out = String::new();
    out.push_str("# CourseCraft performance report\n\n");
    let _ = writeln!(out, "Date: {date}");
    let _ = writeln!(out, "OS: {os}");
    let _ = writeln!(out, "CPUs: {cpus}");
    out.push_str("\n| Suite | Stage | Median | p95 | Budget | Status |\n");
    out.push_str("|---|---|---|---|---|---|\n");
    for line in tsv.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let mut fields = line.splitn(6, '\t');
        let (Some(suite), Some(stage), Some(median), Some(p95), Some(budget)) = (
            fields.next(),
            fields.next(),
            fields.next(),
            fields.next(),
            fields.next(),
        ) else {
            continue;
        };
        let note = fields.next().unwrap_or("");
        let median = median.parse::<f64>().unwrap_or(0.0);
        let p95 = p95.parse::<f64>().unwrap_or(0.0);
        let budget = budget.parse::<f64>().unwrap_or(0.0);
        let status = if budget > 0.0 && p95 > budget {
            "over"
        } else {
            "ok"
        };
        let stage_cell = if note.is_empty() {
            stage.to_owned()
        } else {
            format!("{stage} ({note})")
        };
        let _ = writeln!(
            out,
            "| {suite} | {stage_cell} | {median:.1} ms | {p95:.1} ms | {budget} ms | {status} |"
        );
    }
    out
}

/// Run every ignored timing test in release, then summarise their TSV rows.
/// Exits non-zero only when a test fails, never because a stage is over budget.
fn perf() -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("xtask has no parent directory")?;
    let dir = root.join("target/perf");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let tsv = dir.join("results.tsv");
    let report = dir.join("report.md");
    let _ = std::fs::remove_file(&tsv);

    let ran_ui = cargo(&[
        "test",
        "--release",
        "-p",
        "coursecraft-ui-egui",
        "perf_tests",
        "--",
        "--ignored",
        "--nocapture",
        "--test-threads=1",
    ]);
    let ran_cli = cargo(&[
        "test",
        "--release",
        "-p",
        "coursecraft-cli",
        "--test",
        "big_course",
        "--",
        "--ignored",
        "--nocapture",
        "--test-threads=1",
    ]);
    if !ran_ui || !ran_cli {
        return Err("a timing test failed".into());
    }

    let raw = std::fs::read_to_string(&tsv).map_err(|e| {
        format!(
            "{}: {e} (did the timing tests write any results?)",
            tsv.display()
        )
    })?;
    let date = std::process::Command::new("date")
        .args(["-u", "+%Y-%m-%d"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".into());
    let os = format!("{} {}", std::env::consts::OS, std::env::consts::ARCH);
    let cpus = std::thread::available_parallelism().map_or(0, |n| n.get());
    let mut markdown = results_to_markdown(&raw, &date, &os, cpus);
    markdown.push_str(&format_wasm_size_section(&root.join("target/player-dist")));
    std::fs::write(&report, markdown).map_err(|e| format!("{}: {e}", report.display()))?;
    println!("perf: report written to {}", report.display());
    Ok(())
}

fn measure_wasm_size(dist: &Path) -> Option<(String, u64, Option<u64>)> {
    let entries = std::fs::read_dir(dist).ok()?;
    let mut wasm_path = None;
    for entry in entries.flatten() {
        let path = entry.path();
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with("_bg.wasm"))
        {
            wasm_path = Some(path);
            break;
        }
    }
    let path = wasm_path?;
    let name = path.file_name()?.to_string_lossy().into_owned();
    let raw_bytes = std::fs::metadata(&path).ok()?.len();
    let gzip_bytes = Command::new("gzip")
        .arg("-c")
        .arg(&path)
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| out.stdout.len() as u64);
    Some((name, raw_bytes, gzip_bytes))
}

fn format_wasm_size_section(dist: &Path) -> String {
    let mut out = String::from("\n## Web player wasm size\n\n");
    let Some((name, raw_bytes, gzip_bytes)) = measure_wasm_size(dist) else {
        out.push_str("Not built (run `cargo xtask build-player`).\n");
        return out;
    };
    let raw_mib = raw_bytes as f64 / (1024.0 * 1024.0);
    let gzip_cell = match gzip_bytes {
        Some(bytes) => format!("{bytes} B ({:.2} MiB)", bytes as f64 / (1024.0 * 1024.0)),
        None => "n/a".to_owned(),
    };
    let _ = writeln!(out, "- File: `{name}`");
    let _ = writeln!(out, "- Raw: {raw_bytes} B ({raw_mib:.2} MiB)");
    let _ = writeln!(out, "- Gzip: {gzip_cell}");
    out
}

fn main() -> ExitCode {
    let task = std::env::args().nth(1).unwrap_or_default();
    let layers = || match check_layers() {
        Ok(()) => {
            println!("layers: ok");
            true
        }
        Err(problems) => {
            for p in problems {
                eprintln!("layers: {p}");
            }
            false
        }
    };
    let ok = match task.as_str() {
        "layers" => layers(),
        "wasm" => check_wasm(),
        "dep-src" => dep_src().map_err(|e| eprintln!("dep-src: {e}")).is_ok(),
        "build-player" => build_player()
            .map_err(|e| eprintln!("build-player: {e}"))
            .is_ok(),
        "package-example" => package_example()
            .map_err(|e| eprintln!("package-example: {e}"))
            .is_ok(),
        "perf" => perf().map_err(|e| eprintln!("perf: {e}")).is_ok(),
        "mock-lrs" => match parse_port(std::env::args().skip(2)) {
            Ok(port) => mock_lrs::run(port)
                .map_err(|e| eprintln!("mock-lrs: {e}"))
                .is_ok(),
            Err(err) => {
                eprintln!("mock-lrs: {err}");
                false
            }
        },
        "mock-ai" => match parse_port_with_default(std::env::args().skip(2), 8099) {
            Ok(port) => mock_ai::run(port)
                .map_err(|e| eprintln!("mock-ai: {e}"))
                .is_ok(),
            Err(err) => {
                eprintln!("mock-ai: {err}");
                false
            }
        },
        "ci" => {
            cargo(&["fmt", "--all", "--check"])
                && cargo(&[
                    "clippy",
                    "--workspace",
                    "--all-targets",
                    "--",
                    "-D",
                    "warnings",
                ])
                && cargo(&["test", "--workspace"])
                && layers()
                && check_wasm()
        }
        _ => {
            eprintln!(
                "usage: cargo xtask <layers|wasm|ci|dep-src|build-player|package-example|perf|mock-lrs|mock-ai>"
            );
            false
        }
    };
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod perf_tests {
    use super::results_to_markdown;

    #[test]
    fn tsv_becomes_a_report_with_over_budget_rows_marked() {
        let tsv = "editor\tidle\t12.3\t13.1\t16\t\n\
                   editor\tslow\t40.0\t55.0\t33\t\n\
                   big-course\topen\t1200.0\t1200.0\t5000\trss 130 MB\n";
        let md = results_to_markdown(tsv, "2026-11-19", "macos x86_64", 8);
        assert!(md.contains("# CourseCraft performance report"));
        assert!(md.contains("Date: 2026-11-19"));
        assert!(md.contains("OS: macos x86_64"));
        assert!(md.contains("CPUs: 8"));
        assert!(md.contains("| editor | idle | 12.3 ms | 13.1 ms | 16 ms | ok |"));
        assert!(md.contains("| editor | slow | 40.0 ms | 55.0 ms | 33 ms | over |"));
        assert!(md.contains("open (rss 130 MB)"));
    }

    #[test]
    fn a_zero_budget_is_never_over() {
        let md = results_to_markdown(
            "big-course\tgenerate\t100.0\t150.0\t0\tmedia 500 MB\n",
            "d",
            "os",
            1,
        );
        assert!(md.contains("generate (media 500 MB) | 100.0 ms | 150.0 ms | 0 ms | ok |"));
    }

    #[test]
    fn dist_cleanup_removes_course_json_and_nothing_else() {
        let dir = std::env::temp_dir().join(format!(
            "coursecraft-xtask-dist-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        std::fs::write(dir.join("index.html"), "<html></html>").expect("write index.html");
        std::fs::write(dir.join("player.js"), "console.log(1);").expect("write player.js");
        std::fs::write(dir.join("player_bg.wasm"), b"wasm").expect("write wasm");
        std::fs::write(dir.join("course.json"), "{}").expect("write course.json");

        super::remove_bundled_demo_course(&dir).expect("cleanup succeeds");
        assert!(!dir.join("course.json").exists());
        assert!(dir.join("index.html").exists());
        assert!(dir.join("player.js").exists());
        assert!(dir.join("player_bg.wasm").exists());

        // Idempotent when course.json is already absent.
        super::remove_bundled_demo_course(&dir).expect("second cleanup succeeds");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn wasm_size_section_reports_raw_and_gzip_bytes() {
        let dir = std::env::temp_dir().join(format!(
            "coursecraft-xtask-wasm-size-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let wasm_bytes = vec![0x41u8; 4096];
        std::fs::write(dir.join("coursecraft-player_bg.wasm"), &wasm_bytes).expect("write wasm");

        let section = super::format_wasm_size_section(&dir);
        assert!(section.contains("## Web player wasm size"));
        assert!(section.contains("coursecraft-player_bg.wasm"));
        assert!(section.contains("Raw: 4096 B"));
        assert!(section.contains("Gzip:"));

        let _ = std::fs::remove_dir_all(&dir);
        let missing = super::format_wasm_size_section(&dir);
        assert!(missing.contains("Not built"));
    }
}
