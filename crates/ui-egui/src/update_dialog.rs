//! Calm desktop update dialog, safe Markdown release-notes renderer, and status-bar hint.
//!
//! Pure egui, zero networking, zero filesystem I/O, and 100% WASM-safe.
//! All network checks and verified downloads live in `apps/coursecraft/src/updates.rs`;
//! this module only renders state and emits [`EditorRequest`]s.

use crate::dialog::Dialog;
use crate::{Editor, EditorRequest, QuietTone};
use coursecraft_render::Palette;
use egui::{CornerRadius, FontFamily, FontId, Label, RichText, Sense, Ui};

/// Maximum release-notes payload parsed, in UTF-8 bytes (32 KiB).
pub const MAX_NOTES_BYTES: usize = 32 * 1024;

/// Maximum release-notes lines parsed.
pub const MAX_NOTES_LINES: usize = 300;

/// Label shown on the quiet status-bar pill when an update is available.
const STATUS_PILL_LABEL: &str = "Update available";

/// Metadata for the platform installer matched for the user's OS and CPU.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateAssetInfo {
    pub name: String,
    pub size: u64,
}

/// Information about a newer release available on GitHub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateInfo {
    pub version: String,
    pub current_version: String,
    pub release_url: String,
    pub notes_markdown: String,
    pub asset: Option<UpdateAssetInfo>,
    pub published_at: Option<String>,
}

/// State of an in-progress or completed installer download.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum UpdateDownloadState {
    #[default]
    Idle,
    Downloading {
        downloaded: u64,
        total: Option<u64>,
    },
    Verified {
        filename: String,
        folder_display: String,
        sha256_verified: bool,
    },
    Failed(String),
}

/// Outcome of the most recent update check.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum UpdateCheckStatus {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available(UpdateInfo),
    Error(String),
}

/// UI state for the Software update dialog and status-bar/menu hints.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UpdateState {
    pub open: bool,
    pub status: UpdateCheckStatus,
    pub download: UpdateDownloadState,
    pub dismissed_version: Option<String>,
}

impl UpdateState {
    /// True when a newer release is known and the user has not skipped it.
    #[must_use]
    pub fn has_available_hint(&self) -> bool {
        matches!(
            &self.status,
            UpdateCheckStatus::Available(info)
                if self.dismissed_version.as_deref() != Some(info.version.as_str())
        )
    }

    /// Return the available release metadata, if an update is known.
    #[must_use]
    pub fn available_info(&self) -> Option<&UpdateInfo> {
        match &self.status {
            UpdateCheckStatus::Available(info) => Some(info),
            _ => None,
        }
    }
}

/// A parsed block of release-notes Markdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MdBlock {
    Heading { level: u8, spans: Vec<MdSpan> },
    Paragraph(Vec<MdSpan>),
    BulletItem(Vec<MdSpan>),
    NumberedItem { number: String, spans: Vec<MdSpan> },
    Rule,
}

/// A parsed inline span inside a release-notes block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MdSpan {
    Text(String),
    Bold(String),
    Italic(String),
    Code(String),
    Link { label: String, url: String },
}

/// Parse GitHub release notes Markdown into safe, bounded UI blocks.
///
/// Enforces [`MAX_NOTES_BYTES`] and [`MAX_NOTES_LINES`], strips control and Unicode
/// bidirectional override/isolate characters, leaves raw HTML and Markdown images
/// `![alt](url)` as inert plain text, and only emits clickable [`MdSpan::Link`]s for
/// strict `https://` URLs with no whitespace or control characters.
#[must_use]
pub fn parse_release_notes(markdown: &str) -> Vec<MdBlock> {
    let bounded = truncate_utf8_boundary(markdown, MAX_NOTES_BYTES);
    let sanitized = sanitize_markdown(bounded);
    let mut blocks = Vec::new();
    let mut paragraph_lines: Vec<&str> = Vec::new();
    let mut in_code_fence = false;

    for raw_line in sanitized.split('\n').take(MAX_NOTES_LINES) {
        let trimmed = raw_line.trim();
        if trimmed.starts_with("```") {
            flush_paragraph(&mut paragraph_lines, &mut blocks);
            in_code_fence = !in_code_fence;
            continue;
        }
        if in_code_fence {
            if !trimmed.is_empty() {
                blocks.push(MdBlock::Paragraph(vec![MdSpan::Code(raw_line.to_owned())]));
            }
            continue;
        }
        if trimmed.is_empty() {
            flush_paragraph(&mut paragraph_lines, &mut blocks);
            continue;
        }
        if let Some(block) = parse_single_line_block(trimmed) {
            flush_paragraph(&mut paragraph_lines, &mut blocks);
            blocks.push(block);
        } else {
            paragraph_lines.push(trimmed);
        }
    }
    flush_paragraph(&mut paragraph_lines, &mut blocks);
    blocks
}

fn truncate_utf8_boundary(input: &str, max_bytes: usize) -> &str {
    if input.len() <= max_bytes {
        return input;
    }
    let mut end = max_bytes;
    while end > 0 && !input.is_char_boundary(end) {
        end -= 1;
    }
    input.get(..end).unwrap_or_default()
}

fn is_forbidden_char(ch: char) -> bool {
    if ch == '\n' || ch == '\t' {
        return false;
    }
    ch.is_control()
        || matches!(
            ch,
            '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}'
        )
}

fn sanitize_markdown(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\r' {
            if chars.peek() != Some(&'\n') {
                out.push('\n');
            }
            continue;
        }
        if !is_forbidden_char(ch) {
            out.push(ch);
        }
    }
    out
}

fn flush_paragraph(lines: &mut Vec<&str>, blocks: &mut Vec<MdBlock>) {
    if lines.is_empty() {
        return;
    }
    let joined = lines.join(" ");
    lines.clear();
    let spans = parse_inline_spans(&joined);
    if !spans.is_empty() {
        blocks.push(MdBlock::Paragraph(spans));
    }
}

fn parse_single_line_block(trimmed: &str) -> Option<MdBlock> {
    if is_horizontal_rule(trimmed) {
        return Some(MdBlock::Rule);
    }
    if let Some((level, rest)) = parse_heading_prefix(trimmed) {
        return Some(MdBlock::Heading {
            level,
            spans: parse_inline_spans(rest),
        });
    }
    if let Some(rest) = parse_bullet_prefix(trimmed) {
        return Some(MdBlock::BulletItem(parse_inline_spans(rest)));
    }
    if let Some((number, rest)) = parse_numbered_prefix(trimmed) {
        return Some(MdBlock::NumberedItem {
            number,
            spans: parse_inline_spans(rest),
        });
    }
    None
}

fn is_horizontal_rule(trimmed: &str) -> bool {
    let mut marker: Option<char> = None;
    let mut count = 0usize;
    for ch in trimmed.chars() {
        if ch == ' ' || ch == '\t' {
            continue;
        }
        if !matches!(ch, '-' | '*' | '_') {
            return false;
        }
        match marker {
            None => marker = Some(ch),
            Some(m) if m != ch => return false,
            Some(_) => {}
        }
        count += 1;
    }
    count >= 3
}

fn parse_heading_prefix(trimmed: &str) -> Option<(u8, &str)> {
    let hashes = trimmed.bytes().take_while(|&b| b == b'#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let after = trimmed.get(hashes..)?;
    let rest = after.strip_prefix(' ')?.trim();
    if rest.is_empty() {
        return None;
    }
    let level = hashes.min(3) as u8;
    Some((level, rest))
}

fn parse_bullet_prefix(trimmed: &str) -> Option<&str> {
    for prefix in ["- ", "• ", "+ ", "* "] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            let content = rest.trim();
            if !content.is_empty() {
                return Some(content);
            }
        }
    }
    None
}

fn parse_numbered_prefix(trimmed: &str) -> Option<(String, &str)> {
    let digits_len = trimmed.bytes().take_while(|b| b.is_ascii_digit()).count();
    if digits_len == 0 || digits_len > 6 {
        return None;
    }
    let number = trimmed.get(..digits_len)?;
    let rest = trimmed.get(digits_len..)?.strip_prefix(". ")?.trim();
    if rest.is_empty() {
        return None;
    }
    Some((number.to_owned(), rest))
}

fn is_safe_https_url(url: &str) -> bool {
    url.starts_with("https://")
        && url.len() > "https://".len()
        && !url
            .chars()
            .any(|c| c.is_whitespace() || is_forbidden_char(c))
}

#[derive(Default)]
struct InlineScanState {
    no_close_bracket: bool,
    no_close_paren: bool,
}

fn parse_inline_spans(text: &str) -> Vec<MdSpan> {
    let mut spans = Vec::new();
    let mut plain = String::new();
    let mut rest = text;
    let mut scan = InlineScanState::default();

    while !rest.is_empty() {
        if let Some((literal, next)) = consume_image_syntax(rest, &mut scan) {
            plain.push_str(literal);
            rest = next;
            continue;
        }
        if let Some((span, next)) = consume_inline_token(rest, &mut scan) {
            match span {
                MdSpan::Text(t) => plain.push_str(&t),
                other => {
                    flush_plain(&mut plain, &mut spans);
                    spans.push(other);
                }
            }
            rest = next;
            continue;
        }
        if let Some(ch) = rest.chars().next() {
            plain.push(ch);
            rest = rest.get(ch.len_utf8()..).unwrap_or_default();
        } else {
            break;
        }
    }
    flush_plain(&mut plain, &mut spans);
    spans
}

fn flush_plain(plain: &mut String, spans: &mut Vec<MdSpan>) {
    if !plain.is_empty() {
        spans.push(MdSpan::Text(std::mem::take(plain)));
    }
}

/// Match `![alt](url)` and return it verbatim so images stay inert text and never turn into links.
fn consume_image_syntax<'a>(
    rest: &'a str,
    scan: &mut InlineScanState,
) -> Option<(&'a str, &'a str)> {
    if scan.no_close_bracket || scan.no_close_paren {
        return None;
    }
    let after_bang = rest.strip_prefix("![")?;
    let Some(close_bracket) = after_bang.find(['[', ']']) else {
        scan.no_close_bracket = true;
        return None;
    };
    if after_bang.as_bytes().get(close_bracket) != Some(&b']') {
        return None;
    }
    let after_bracket = after_bang.get(close_bracket + 1..)?;
    let after_paren = after_bracket.strip_prefix('(')?;
    let Some(close_paren) = after_paren.find(')') else {
        scan.no_close_paren = true;
        return None;
    };
    let total_len = 2 + close_bracket + 2 + close_paren + 1;
    let literal = rest.get(..total_len)?;
    let next = rest.get(total_len..)?;
    Some((literal, next))
}

fn consume_inline_token<'a>(
    rest: &'a str,
    scan: &mut InlineScanState,
) -> Option<(MdSpan, &'a str)> {
    if let Some((code, next)) = consume_delimited(rest, "`") {
        return Some((MdSpan::Code(code.to_owned()), next));
    }
    if let Some((bold, next)) = consume_delimited(rest, "**") {
        return Some((MdSpan::Bold(bold.to_owned()), next));
    }
    if !rest.starts_with("**")
        && let Some((italic, next)) = consume_italic(rest)
    {
        return Some((MdSpan::Italic(italic.to_owned()), next));
    }
    consume_link(rest, scan)
}

fn consume_delimited<'a>(rest: &'a str, delim: &str) -> Option<(&'a str, &'a str)> {
    let after_open = rest.strip_prefix(delim)?;
    let end = after_open.find(delim)?;
    let inner = after_open.get(..end)?;
    if inner.is_empty() {
        return None;
    }
    let next = after_open.get(end + delim.len()..)?;
    Some((inner, next))
}

fn consume_italic(rest: &str) -> Option<(&str, &str)> {
    let after_open = rest.strip_prefix('*')?;
    if after_open.starts_with(char::is_whitespace) || after_open.starts_with('*') {
        return None;
    }
    let end = after_open.find('*')?;
    let inner = after_open.get(..end)?;
    if inner.is_empty() || inner.ends_with(char::is_whitespace) {
        return None;
    }
    let next = after_open.get(end + 1..)?;
    Some((inner, next))
}

fn consume_link<'a>(rest: &'a str, scan: &mut InlineScanState) -> Option<(MdSpan, &'a str)> {
    if scan.no_close_bracket || scan.no_close_paren {
        return None;
    }
    let after_open = rest.strip_prefix('[')?;
    let Some(close_bracket) = after_open.find(['[', ']']) else {
        scan.no_close_bracket = true;
        return None;
    };
    if after_open.as_bytes().get(close_bracket) != Some(&b']') {
        return None;
    }
    let label = after_open.get(..close_bracket)?;
    let after_bracket = after_open.get(close_bracket + 1..)?;
    let after_paren = after_bracket.strip_prefix('(')?;
    let Some(close_paren) = after_paren.find(')') else {
        scan.no_close_paren = true;
        return None;
    };
    let url = after_paren.get(..close_paren)?;
    let next = after_paren.get(close_paren + 1..)?;

    if is_safe_https_url(url) {
        let display = if label.is_empty() { url } else { label };
        Some((
            MdSpan::Link {
                label: display.to_owned(),
                url: url.to_owned(),
            },
            next,
        ))
    } else {
        let fallback = if label.is_empty() { url } else { label };
        Some((MdSpan::Text(fallback.to_owned()), next))
    }
}

/// Render parsed release-notes blocks using only CourseCraft palette tokens.
pub fn show_release_notes(ui: &mut Ui, blocks: &[MdBlock]) {
    let p = coursecraft_render::palette(ui.ctx());
    if blocks.is_empty() {
        ui.add(
            Label::new(
                coursecraft_render::body("No release notes were included for this version.")
                    .color(p.text_secondary),
            )
            .wrap(),
        );
        return;
    }

    for (idx, block) in blocks.iter().enumerate() {
        match block {
            MdBlock::Heading { level, spans } => {
                if idx > 0 {
                    ui.add_space(coursecraft_render::SPACE_MD);
                }
                let size = match level {
                    1 | 2 => coursecraft_render::SIZE_TITLE,
                    _ => coursecraft_render::SIZE_BODY_STRONG,
                };
                show_spans(ui, spans, size, true, &p);
                ui.add_space(coursecraft_render::SPACE_SM);
            }
            MdBlock::Paragraph(spans) => {
                show_spans(ui, spans, coursecraft_render::SIZE_BODY, false, &p);
                ui.add_space(coursecraft_render::SPACE_SM);
            }
            MdBlock::BulletItem(spans) => {
                ui.horizontal_top(|ui| {
                    ui.add_space(coursecraft_render::SPACE_SM);
                    ui.label(RichText::new("•").color(p.text_secondary));
                    ui.vertical(|ui| {
                        show_spans(ui, spans, coursecraft_render::SIZE_BODY, false, &p);
                    });
                });
                ui.add_space(coursecraft_render::SPACE_XS);
            }
            MdBlock::NumberedItem { number, spans } => {
                let marker = if number.ends_with('.') {
                    number.clone()
                } else {
                    format!("{number}.")
                };
                ui.horizontal_top(|ui| {
                    ui.add_space(coursecraft_render::SPACE_SM);
                    ui.label(RichText::new(marker).color(p.text_secondary));
                    ui.vertical(|ui| {
                        show_spans(ui, spans, coursecraft_render::SIZE_BODY, false, &p);
                    });
                });
                ui.add_space(coursecraft_render::SPACE_XS);
            }
            MdBlock::Rule => {
                ui.add_space(coursecraft_render::SPACE_SM);
                ui.separator();
                ui.add_space(coursecraft_render::SPACE_SM);
            }
        }
    }
}

fn show_spans(ui: &mut Ui, spans: &[MdSpan], font_size: f32, default_bold: bool, p: &Palette) {
    let has_link = spans.iter().any(|s| matches!(s, MdSpan::Link { .. }));
    if !has_link {
        let mut job = egui::text::LayoutJob::default();
        job.wrap.max_width = ui.available_width().max(80.0);
        for span in spans {
            append_span_to_job(&mut job, span, font_size, default_bold, p);
        }
        ui.add(Label::new(job).wrap());
        return;
    }

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for span in spans {
            show_single_span(ui, span, font_size, default_bold, p);
        }
    });
}

fn semibold_font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("inter-semibold".into()))
}

fn append_span_to_job(
    job: &mut egui::text::LayoutJob,
    span: &MdSpan,
    font_size: f32,
    default_bold: bool,
    p: &Palette,
) {
    match span {
        MdSpan::Text(text) => {
            let font_id = if default_bold {
                semibold_font(font_size)
            } else {
                FontId::proportional(font_size)
            };
            let fmt = egui::TextFormat::simple(font_id, p.text);
            job.append(text, 0.0, fmt);
        }
        MdSpan::Bold(text) => {
            let fmt = egui::TextFormat::simple(semibold_font(font_size), p.text);
            job.append(text, 0.0, fmt);
        }
        MdSpan::Italic(text) => {
            let mut fmt = egui::TextFormat::simple(FontId::proportional(font_size), p.text);
            fmt.italics = true;
            job.append(text, 0.0, fmt);
        }
        MdSpan::Code(text) => {
            let mut fmt =
                egui::TextFormat::simple(FontId::monospace(coursecraft_render::SIZE_MONO), p.text);
            fmt.background = p.input;
            job.append(text, 0.0, fmt);
        }
        MdSpan::Link { label, .. } => {
            let mut fmt = egui::TextFormat::simple(FontId::proportional(font_size), p.accent_text);
            fmt.underline = egui::Stroke::new(1.0, p.accent_text);
            job.append(label, 0.0, fmt);
        }
    }
}

fn show_single_span(ui: &mut Ui, span: &MdSpan, font_size: f32, default_bold: bool, p: &Palette) {
    match span {
        MdSpan::Text(text) => {
            let rt = if default_bold {
                RichText::new(text)
                    .font(semibold_font(font_size))
                    .color(p.text)
            } else {
                RichText::new(text).size(font_size).color(p.text)
            };
            ui.add(Label::new(rt).wrap());
        }
        MdSpan::Bold(text) => {
            ui.add(
                Label::new(
                    RichText::new(text)
                        .font(semibold_font(font_size))
                        .color(p.text),
                )
                .wrap(),
            );
        }
        MdSpan::Italic(text) => {
            ui.add(Label::new(RichText::new(text).size(font_size).italics().color(p.text)).wrap());
        }
        MdSpan::Code(text) => {
            ui.add(
                Label::new(
                    RichText::new(text)
                        .monospace()
                        .size(coursecraft_render::SIZE_MONO)
                        .color(p.text)
                        .background_color(p.input),
                )
                .wrap(),
            );
        }
        MdSpan::Link { label, url } => {
            let resp = ui
                .add(
                    Label::new(
                        RichText::new(label)
                            .size(font_size)
                            .color(p.accent_text)
                            .underline(),
                    )
                    .wrap()
                    .sense(Sense::click()),
                )
                .on_hover_text(url)
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if resp.clicked() {
                crate::report::open_link(ui.ctx(), url);
            }
        }
    }
}

/// Format a byte count for human display (e.g. `"27.4 MB"`).
#[must_use]
pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    let b = bytes as f64;
    if bytes < 1024 {
        format!("{bytes} B")
    } else if b < MB {
        format!("{:.1} KB", b / KB)
    } else if b < GB {
        format!("{:.1} MB", b / MB)
    } else {
        format!("{:.2} GB", b / GB)
    }
}

enum DialogAction {
    None,
    Close,
    CheckForUpdates,
    DownloadUpdate,
    CancelDownload,
    SkipVersion(String),
    RevealDownloaded,
    OpenUrl(String),
}

/// Show the calm Software update dialog when `editor.ui.update.open` is true.
pub fn show_update_dialog(
    editor: &mut Editor,
    ctx: &egui::Context,
    requests: &mut Vec<EditorRequest>,
) {
    if !editor.ui.update.open {
        return;
    }

    let status = editor.ui.update.status.clone();
    let download = editor.ui.update.download.clone();
    let mut open = true;
    let mut action = DialogAction::None;

    Dialog::new("Software update")
        .open(&mut open)
        .resizable(true)
        .default_size([520.0, 420.0])
        .show_with_footer(ctx, |dialog| {
            dialog.body(|ui| {
                show_dialog_body(ui, &status, &download);
            });
            action = dialog.footer(|ui| {
                ui.add_space(coursecraft_render::SPACE_SM);
                show_dialog_footer(ui, &status, &download)
            });
        });

    if !open {
        editor.ui.update.open = false;
    }
    apply_dialog_action(editor, ctx, requests, action);
}

fn apply_dialog_action(
    editor: &mut Editor,
    ctx: &egui::Context,
    requests: &mut Vec<EditorRequest>,
    action: DialogAction,
) {
    match action {
        DialogAction::None => {}
        DialogAction::Close => {
            editor.ui.update.open = false;
        }
        DialogAction::CheckForUpdates => {
            requests.push(EditorRequest::CheckForUpdates);
        }
        DialogAction::DownloadUpdate => {
            requests.push(EditorRequest::DownloadUpdate);
        }
        DialogAction::CancelDownload => {
            requests.push(EditorRequest::CancelUpdateDownload);
        }
        DialogAction::SkipVersion(version) => {
            editor.ui.update.dismissed_version = Some(version.clone());
            editor.ui.update.open = false;
            requests.push(EditorRequest::SkipUpdateVersion(version));
        }
        DialogAction::RevealDownloaded => {
            requests.push(EditorRequest::RevealDownloadedUpdate);
        }
        DialogAction::OpenUrl(url) => {
            crate::report::open_link(ctx, &url);
        }
    }
}

fn show_dialog_body(ui: &mut Ui, status: &UpdateCheckStatus, download: &UpdateDownloadState) {
    let p = coursecraft_render::palette(ui.ctx());
    match status {
        UpdateCheckStatus::Idle | UpdateCheckStatus::Checking => {
            ui.add(
                Label::new(coursecraft_render::body("Checking for updates…").color(p.text)).wrap(),
            );
        }
        UpdateCheckStatus::UpToDate => {
            ui.add(
                Label::new(coursecraft_render::body_strong("You're up to date.").color(p.text))
                    .wrap(),
            );
            ui.add_space(coursecraft_render::SPACE_SM);
            ui.add(
                Label::new(
                    coursecraft_render::body(format!(
                        "CourseCraft {} is the latest version.",
                        env!("CARGO_PKG_VERSION")
                    ))
                    .color(p.text_secondary),
                )
                .wrap(),
            );
        }
        UpdateCheckStatus::Error(msg) => {
            ui.add(Label::new(coursecraft_render::body(msg).color(p.danger)).wrap());
        }
        UpdateCheckStatus::Available(info) => {
            show_available_body(ui, info, download, &p);
        }
    }
}

fn show_available_body(
    ui: &mut Ui,
    info: &UpdateInfo,
    download: &UpdateDownloadState,
    p: &Palette,
) {
    ui.add(
        Label::new(
            coursecraft_render::title(format!("CourseCraft {} is available", info.version))
                .color(p.text),
        )
        .wrap(),
    );
    ui.add_space(coursecraft_render::SPACE_XS);
    ui.add(
        Label::new(
            coursecraft_render::body(format!(
                "You're currently running {}.",
                info.current_version
            ))
            .color(p.text_secondary),
        )
        .wrap(),
    );
    ui.add_space(coursecraft_render::SPACE_XS);
    if let Some(asset) = &info.asset {
        ui.add(
            Label::new(
                coursecraft_render::small(format!("{} · {}", asset.name, format_bytes(asset.size)))
                    .color(p.text_secondary),
            )
            .wrap(),
        );
    } else {
        ui.add(
            Label::new(
                coursecraft_render::body(
                    "No direct download is packaged for this platform. Open the releases page in your browser to see available files.",
                )
                .color(p.text_secondary),
            )
            .wrap(),
        );
    }

    ui.add_space(coursecraft_render::SPACE_MD);
    ui.separator();
    ui.add_space(coursecraft_render::SPACE_MD);

    let blocks = parse_release_notes(&info.notes_markdown);
    show_release_notes(ui, &blocks);

    show_download_section(ui, download, p);
}

fn show_download_section(ui: &mut Ui, download: &UpdateDownloadState, p: &Palette) {
    match download {
        UpdateDownloadState::Idle => {}
        UpdateDownloadState::Downloading { downloaded, total } => {
            ui.add_space(coursecraft_render::SPACE_MD);
            ui.separator();
            ui.add_space(coursecraft_render::SPACE_SM);
            let (fraction, progress_text) = match total {
                Some(tot) if *tot > 0 => (
                    (*downloaded as f32 / *tot as f32).clamp(0.0, 1.0),
                    format!("{} of {}", format_bytes(*downloaded), format_bytes(*tot)),
                ),
                _ => (0.0, format!("{} downloaded", format_bytes(*downloaded))),
            };
            ui.add(egui::ProgressBar::new(fraction).text(progress_text));
        }
        UpdateDownloadState::Verified {
            filename,
            folder_display,
            sha256_verified,
        } => {
            ui.add_space(coursecraft_render::SPACE_MD);
            ui.separator();
            ui.add_space(coursecraft_render::SPACE_SM);
            ui.add(
                Label::new(
                    coursecraft_render::body_strong(format!(
                        "Saved to {folder_display} ({filename})."
                    ))
                    .color(p.text),
                )
                .wrap(),
            );
            ui.add_space(coursecraft_render::SPACE_XS);
            let badge = if *sha256_verified {
                "SHA-256 verified"
            } else {
                "Size verified"
            };
            ui.horizontal(|ui| {
                let (dot_rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 14.0), Sense::hover());
                ui.painter()
                    .circle_filled(dot_rect.center(), 3.5, p.success);
                ui.label(coursecraft_render::small(badge).color(p.text));
            });
            ui.add_space(coursecraft_render::SPACE_XS);
            ui.add(
                Label::new(
                    coursecraft_render::body(
                        "Save your work and close CourseCraft before installing.",
                    )
                    .color(p.text_secondary),
                )
                .wrap(),
            );
        }
        UpdateDownloadState::Failed(err) => {
            ui.add_space(coursecraft_render::SPACE_MD);
            ui.separator();
            ui.add_space(coursecraft_render::SPACE_SM);
            ui.add(Label::new(coursecraft_render::body(err).color(p.danger)).wrap());
        }
    }
}

fn safe_release_url(candidate: &str) -> String {
    if is_safe_https_url(candidate) {
        candidate.to_owned()
    } else {
        crate::project_links::releases_url()
    }
}

fn secondary_button(ui: &mut Ui, label: &str) -> egui::Response {
    ui.add(egui::Button::new(label).min_size(egui::vec2(0.0, coursecraft_render::CONTROL_COMPACT)))
}

fn show_dialog_footer(
    ui: &mut Ui,
    status: &UpdateCheckStatus,
    download: &UpdateDownloadState,
) -> DialogAction {
    let mut action = DialogAction::None;
    ui.horizontal_wrapped(|ui| match status {
        UpdateCheckStatus::Idle | UpdateCheckStatus::Checking | UpdateCheckStatus::UpToDate => {
            if secondary_button(ui, "Close").clicked() {
                action = DialogAction::Close;
            }
        }
        UpdateCheckStatus::Error(_) => {
            if crate::primary_button(ui, "Try again").clicked() {
                action = DialogAction::CheckForUpdates;
            }
            if crate::quiet_button(ui, "Open releases page", QuietTone::Secondary, true).clicked() {
                action = DialogAction::OpenUrl(crate::project_links::releases_url());
            }
            if secondary_button(ui, "Close").clicked() {
                action = DialogAction::Close;
            }
        }
        UpdateCheckStatus::Available(info) => {
            action = show_available_footer(ui, info, download);
        }
    });
    action
}

fn show_available_footer(
    ui: &mut Ui,
    info: &UpdateInfo,
    download: &UpdateDownloadState,
) -> DialogAction {
    let url = safe_release_url(&info.release_url);
    match download {
        UpdateDownloadState::Downloading { .. } => {
            if secondary_button(ui, "Cancel").clicked() {
                return DialogAction::CancelDownload;
            }
            if crate::quiet_button(ui, "Open releases page", QuietTone::Secondary, true).clicked() {
                return DialogAction::OpenUrl(url);
            }
        }
        UpdateDownloadState::Verified { .. } => {
            if crate::primary_button(ui, "Show in folder").clicked() {
                return DialogAction::RevealDownloaded;
            }
            if secondary_button(ui, "Close").clicked() {
                return DialogAction::Close;
            }
        }
        UpdateDownloadState::Failed(_) => {
            if crate::primary_button(ui, "Try again").clicked() {
                return DialogAction::DownloadUpdate;
            }
            if crate::quiet_button(ui, "Open releases page", QuietTone::Secondary, true).clicked() {
                return DialogAction::OpenUrl(url);
            }
            if secondary_button(ui, "Close").clicked() {
                return DialogAction::Close;
            }
        }
        UpdateDownloadState::Idle => {
            if info.asset.is_some() {
                if crate::primary_button(ui, "Download update").clicked() {
                    return DialogAction::DownloadUpdate;
                }
                if secondary_button(ui, "Later").clicked() {
                    return DialogAction::Close;
                }
                if secondary_button(ui, "Skip this version").clicked() {
                    return DialogAction::SkipVersion(info.version.clone());
                }
                if crate::quiet_button(ui, "Open releases page", QuietTone::Secondary, true)
                    .clicked()
                {
                    return DialogAction::OpenUrl(url);
                }
            } else {
                if crate::primary_button(ui, "Open releases page").clicked() {
                    return DialogAction::OpenUrl(url);
                }
                if secondary_button(ui, "Later").clicked() {
                    return DialogAction::Close;
                }
                if secondary_button(ui, "Skip this version").clicked() {
                    return DialogAction::SkipVersion(info.version.clone());
                }
            }
        }
    }
    DialogAction::None
}

/// Measure the rendered width of the status-bar `"Update available"` pill so `status.rs`
/// can hide it cleanly on narrow windows (`RISK-08`).
#[must_use]
pub fn status_bar_hint_width(ui: &Ui) -> f32 {
    let p = coursecraft_render::palette(ui.ctx());
    let font_id = egui::TextStyle::Small.resolve(ui.style());
    let galley = ui
        .painter()
        .layout_no_wrap(STATUS_PILL_LABEL.to_owned(), font_id, p.text);
    8.0 + 6.0 + 6.0 + galley.size().x + 8.0
}

/// Render the quiet `"Update available"` pill in the status bar when an update is known.
pub fn show_status_bar_hint(editor: &mut Editor, ui: &mut Ui) {
    if !editor.updates_available || !editor.ui.update.has_available_hint() {
        return;
    }
    let Some(version) = editor
        .ui
        .update
        .available_info()
        .map(|info| info.version.clone())
    else {
        return;
    };

    let p = coursecraft_render::palette(ui.ctx());
    let font_id = egui::TextStyle::Small.resolve(ui.style());
    let galley = ui
        .painter()
        .layout_no_wrap(STATUS_PILL_LABEL.to_owned(), font_id, p.text);
    let desired_width = 8.0 + 6.0 + 6.0 + galley.size().x + 8.0;
    let desired_height = 24.0;

    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(desired_width, desired_height), Sense::click());
    resp.widget_info(|| {
        egui::WidgetInfo::labeled(egui::Role::Button, true, STATUS_PILL_LABEL.to_owned())
    });

    if ui.is_rect_visible(rect) {
        let bg = if resp.is_pointer_button_down_on() {
            p.pressed
        } else if resp.hovered() {
            p.hover
        } else {
            p.input
        };
        ui.painter().rect_filled(rect, CornerRadius::same(12), bg);
        let dot_center = egui::pos2(rect.min.x + 8.0 + 3.0, rect.center().y);
        ui.painter().circle_filled(dot_center, 3.0, p.accent);
        let text_pos = egui::pos2(
            rect.min.x + 8.0 + 6.0 + 6.0,
            rect.center().y - galley.size().y / 2.0,
        );
        ui.painter().galley(text_pos, galley, p.text);
    }

    if resp
        .on_hover_text(format!(
            "CourseCraft {version} is available — click to view"
        ))
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
    {
        editor.ui.update.open = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_harness::TestHarness;
    use coursecraft_course::Course;
    use coursecraft_render::ThemeMode;

    fn sample_course() -> Course {
        let json = include_str!("../../../examples/hello-course/course.json");
        Course::from_json(json).expect("hello-course fixture")
    }

    fn sample_update_info(with_asset: bool) -> UpdateInfo {
        UpdateInfo {
            version: "0.4.5".to_owned(),
            current_version: "0.4.3".to_owned(),
            release_url: "https://github.com/ad1-kamath/coursecraft/releases/tag/v0.4.5".to_owned(),
            notes_markdown: "## What's new\n\n- **Faster startup** and `low-latency` timeline.\n- See [changelog](https://github.com/ad1-kamath/coursecraft/releases).\n".to_owned(),
            asset: with_asset.then(|| UpdateAssetInfo {
                name: "CourseCraft-0.4.5-macos.dmg".to_owned(),
                size: 28_677_774,
            }),
            published_at: Some("2026-11-24T00:00:00Z".to_owned()),
        }
    }

    fn collect_links(blocks: &[MdBlock]) -> Vec<(String, String)> {
        let mut links = Vec::new();
        for block in blocks {
            let spans = match block {
                MdBlock::Heading { spans, .. }
                | MdBlock::Paragraph(spans)
                | MdBlock::BulletItem(spans)
                | MdBlock::NumberedItem { spans, .. } => spans.as_slice(),
                MdBlock::Rule => &[],
            };
            for span in spans {
                if let MdSpan::Link { label, url } = span {
                    links.push((label.clone(), url.clone()));
                }
            }
        }
        links
    }

    fn collect_plain_text(blocks: &[MdBlock]) -> String {
        let mut out = String::new();
        for block in blocks {
            let spans = match block {
                MdBlock::Heading { spans, .. }
                | MdBlock::Paragraph(spans)
                | MdBlock::BulletItem(spans)
                | MdBlock::NumberedItem { spans, .. } => spans.as_slice(),
                MdBlock::Rule => &[],
            };
            for span in spans {
                match span {
                    MdSpan::Text(s) | MdSpan::Bold(s) | MdSpan::Italic(s) | MdSpan::Code(s) => {
                        out.push_str(s);
                    }
                    MdSpan::Link { label, .. } => out.push_str(label),
                }
            }
        }
        out
    }

    /// UI-01: Safe Markdown parser features, hostile-input neutralization, and size/line caps.
    #[test]
    fn ui01_markdown_parser_and_safety_caps() {
        let md = "\
# Release 0.4.5
## Highlights
### Details

Paragraph with **bold**, *italic*, `code`, and [GitHub](https://github.com/ad1-kamath/coursecraft).

- First bullet
• Second bullet
+ Third bullet

1. Step one
2. Step two

---
***

Bad links: [js](javascript:alert(1)) [file](file:///etc/passwd) [data](data:text/html,hi) [http](http://example.com) [space](https://example.com/a b)
Image: ![tracker](https://evil.example/pixel.png)
Raw HTML: <script>alert('xss')</script>
Bidi & ctrl: a\u{202E}b\u{2066}c\u{200F}d\u{0007}e\u{0000}f
";
        let blocks = parse_release_notes(md);

        assert!(
            blocks
                .iter()
                .any(|b| matches!(b, MdBlock::Heading { level: 1, .. }))
        );
        assert!(
            blocks
                .iter()
                .any(|b| matches!(b, MdBlock::Heading { level: 2, .. }))
        );
        assert!(
            blocks
                .iter()
                .any(|b| matches!(b, MdBlock::Heading { level: 3, .. }))
        );
        assert_eq!(
            blocks
                .iter()
                .filter(|b| matches!(b, MdBlock::BulletItem(_)))
                .count(),
            3
        );
        assert_eq!(
            blocks
                .iter()
                .filter(|b| matches!(b, MdBlock::NumberedItem { .. }))
                .count(),
            2
        );
        assert_eq!(
            blocks.iter().filter(|b| matches!(b, MdBlock::Rule)).count(),
            2
        );

        let has_bold = blocks.iter().any(|b| match b {
            MdBlock::Paragraph(s) => s.contains(&MdSpan::Bold("bold".to_owned())),
            _ => false,
        });
        let has_italic = blocks.iter().any(|b| match b {
            MdBlock::Paragraph(s) => s.contains(&MdSpan::Italic("italic".to_owned())),
            _ => false,
        });
        let has_code = blocks.iter().any(|b| match b {
            MdBlock::Paragraph(s) => s.contains(&MdSpan::Code("code".to_owned())),
            _ => false,
        });
        assert!(has_bold && has_italic && has_code);

        // Only the single valid https:// link is emitted as MdSpan::Link.
        let links = collect_links(&blocks);
        assert_eq!(
            links,
            vec![(
                "GitHub".to_owned(),
                "https://github.com/ad1-kamath/coursecraft".to_owned()
            )]
        );

        // Raw HTML, image syntax, and stripped bidi/control chars stay inert.
        let text = collect_plain_text(&blocks);
        assert!(text.contains("<script>alert('xss')</script>"));
        assert!(text.contains("![tracker](https://evil.example/pixel.png)"));
        assert!(text.contains("abcdef"));
        assert!(!text.contains('\u{202E}'));
        assert!(!text.contains('\u{2066}'));
        assert!(!text.contains('\u{200F}'));
        assert!(!text.contains('\u{0007}'));
        assert!(!text.contains('\u{0000}'));

        // Byte cap (> 32 KiB with multibyte UTF-8 characters) truncates cleanly on a char boundary.
        let huge_utf8 = "é".repeat(MAX_NOTES_BYTES);
        let huge_blocks = parse_release_notes(&huge_utf8);
        assert_eq!(huge_blocks.len(), 1);
        assert!(collect_plain_text(&huge_blocks).len() <= MAX_NOTES_BYTES);

        // Line cap (> 300 lines) caps cleanly with zero panics.
        let many_lines = (0..500)
            .map(|i| format!("- Item {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let capped_blocks = parse_release_notes(&many_lines);
        assert_eq!(capped_blocks.len(), MAX_NOTES_LINES);

        // Pathological unmatched brackets/parens stay linear time (no O(n^2) freeze).
        for hostile in [
            "[".repeat(MAX_NOTES_BYTES),
            format!("{}]", "[".repeat(MAX_NOTES_BYTES - 1)),
            "[a](b".repeat(5_000),
            format!("{}] [ok](https://example.com)", "![a".repeat(5_000)),
        ] {
            let blocks = parse_release_notes(&hostile);
            assert_eq!(blocks.len(), 1);
        }

        // Direct release_url guard rejects bidi overrides and non-https schemes.
        assert_eq!(
            safe_release_url("https://github.com/ad1-kamath/\u{202E}evil"),
            crate::project_links::releases_url()
        );
        assert_eq!(
            safe_release_url("javascript:alert(1)"),
            crate::project_links::releases_url()
        );
    }

    /// UI-02: Update dialog renders every state in Dark and Light themes and emits exact requests.
    #[test]
    fn ui02_update_dialog_states_and_requests() {
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            // 1. Available (asset matched): Download update, Later, Skip this version, Open releases page.
            {
                let mut h = TestHarness::new_with_theme(sample_course(), (1280.0, 800.0), mode);
                h.editor.set_updates_available(true);
                h.editor.ui.update.open = true;
                h.editor.ui.update.status = UpdateCheckStatus::Available(sample_update_info(true));
                h.idle_frame();
                h.idle_frame();

                assert!(h.find_text("CourseCraft 0.4.5 is available").is_some());
                assert!(h.find_text("Open releases page").is_some());

                let dl_pos = h
                    .find_text_last("Download update")
                    .expect("Download update button");
                let reqs = h.click_at_requests(dl_pos);
                assert!(
                    reqs.contains(&EditorRequest::DownloadUpdate),
                    "expected DownloadUpdate in {mode:?}, got {reqs:?}"
                );

                // Clicking Later closes the dialog without emitting SkipUpdateVersion.
                let later_pos = h.find_text_last("Later").expect("Later button");
                let reqs = h.click_at_requests(later_pos);
                assert!(!h.editor.ui.update.open);
                assert!(reqs.is_empty());

                // Reopen and click Skip this version.
                h.editor.ui.update.open = true;
                h.idle_frame();
                let skip_pos = h
                    .find_text_last("Skip this version")
                    .expect("Skip this version button");
                let reqs = h.click_at_requests(skip_pos);
                assert!(!h.editor.ui.update.open);
                assert!(reqs.contains(&EditorRequest::SkipUpdateVersion("0.4.5".to_owned())));
                assert!(!h.editor.ui.update.has_available_hint());
            }

            // 2. Available (no matching asset): fallback note + Open releases page, no Download update.
            {
                let mut h = TestHarness::new_with_theme(sample_course(), (1280.0, 800.0), mode);
                h.editor.set_updates_available(true);
                h.editor.ui.update.open = true;
                h.editor.ui.update.status = UpdateCheckStatus::Available(sample_update_info(false));
                h.idle_frame();
                h.idle_frame();

                assert!(h.find_text("Download update").is_none());
                assert!(h.find_text_last("Open releases page").is_some());
                assert!(
                    h.find_text_containing("No direct download is packaged")
                        .is_some()
                );
            }

            // 3. Checking state.
            {
                let mut h = TestHarness::new_with_theme(sample_course(), (1280.0, 800.0), mode);
                h.editor.ui.update.open = true;
                h.editor.ui.update.status = UpdateCheckStatus::Checking;
                h.idle_frame();
                h.idle_frame();

                assert!(h.find_text("Checking for updates…").is_some());
                let close_pos = h.find_text_last("Close").expect("Close button");
                h.click_at(close_pos);
                assert!(!h.editor.ui.update.open);
            }

            // 4. UpToDate state.
            {
                let mut h = TestHarness::new_with_theme(sample_course(), (1280.0, 800.0), mode);
                h.editor.ui.update.open = true;
                h.editor.ui.update.status = UpdateCheckStatus::UpToDate;
                h.idle_frame();
                h.idle_frame();

                assert!(h.find_text("You're up to date.").is_some());
                assert!(h.find_text_last("Close").is_some());
            }

            // 5. Downloading state: progress + Cancel emits CancelUpdateDownload.
            {
                let mut h = TestHarness::new_with_theme(sample_course(), (1280.0, 800.0), mode);
                h.editor.ui.update.open = true;
                h.editor.ui.update.status = UpdateCheckStatus::Available(sample_update_info(true));
                h.editor.ui.update.download = UpdateDownloadState::Downloading {
                    downloaded: 10 * 1024 * 1024,
                    total: Some(20 * 1024 * 1024),
                };
                h.idle_frame();
                h.idle_frame();

                assert!(h.find_text_containing("10.0 MB of 20.0 MB").is_some());
                let cancel_pos = h.find_text_last("Cancel").expect("Cancel button");
                let reqs = h.click_at_requests(cancel_pos);
                assert!(reqs.contains(&EditorRequest::CancelUpdateDownload));
            }

            // 6. Verified state: Show in folder emits RevealDownloadedUpdate; NO "Open disk image".
            {
                let mut h = TestHarness::new_with_theme(sample_course(), (1280.0, 800.0), mode);
                h.editor.ui.update.open = true;
                h.editor.ui.update.status = UpdateCheckStatus::Available(sample_update_info(true));
                h.editor.ui.update.download = UpdateDownloadState::Verified {
                    filename: "CourseCraft-0.4.5-macos.dmg".to_owned(),
                    folder_display: "Downloads".to_owned(),
                    sha256_verified: true,
                };
                h.idle_frame();
                h.idle_frame();

                assert!(h.find_text("SHA-256 verified").is_some());
                assert!(h.find_text("Open disk image").is_none());
                let reveal_pos = h
                    .find_text_last("Show in folder")
                    .expect("Show in folder button");
                let reqs = h.click_at_requests(reveal_pos);
                assert!(reqs.contains(&EditorRequest::RevealDownloadedUpdate));
            }

            // 7. Error state: Try again emits CheckForUpdates.
            {
                let mut h = TestHarness::new_with_theme(sample_course(), (1280.0, 800.0), mode);
                h.editor.ui.update.open = true;
                h.editor.ui.update.status = UpdateCheckStatus::Error(
                    "Couldn't check for updates. Check your internet connection and try again."
                        .to_owned(),
                );
                h.idle_frame();
                h.idle_frame();

                let retry_pos = h.find_text_last("Try again").expect("Try again button");
                let reqs = h.click_at_requests(retry_pos);
                assert!(reqs.contains(&EditorRequest::CheckForUpdates));
                assert!(h.find_text_last("Open releases page").is_some());
            }

            assert_download_failed_and_empty_notes(mode);
        }
    }

    fn assert_download_failed_and_empty_notes(mode: ThemeMode) {
        let mut h = TestHarness::new_with_theme(sample_course(), (1280.0, 800.0), mode);
        let mut info = sample_update_info(true);
        info.notes_markdown.clear();
        h.editor.ui.update.open = true;
        h.editor.ui.update.status = UpdateCheckStatus::Available(info);
        h.editor.ui.update.download = UpdateDownloadState::Failed(
            "The downloaded file didn't pass verification and was removed.".to_owned(),
        );
        h.idle_frame();
        h.idle_frame();

        assert!(
            h.find_text_containing("No release notes were included")
                .is_some()
        );
        let retry_pos = h.find_text_last("Try again").expect("Try again button");
        let reqs = h.click_at_requests(retry_pos);
        assert!(reqs.contains(&EditorRequest::DownloadUpdate));
    }

    /// UI-03: Help menu, status bar hint, and Welcome screen visibility when updates are available.
    #[test]
    fn ui03_help_menu_status_bar_and_welcome_visibility() {
        let mut h = TestHarness::new(sample_course());
        h.editor.set_updates_available(true);
        h.idle_frame();

        // 1. Help menu contains "Check for updates…" and emits CheckForUpdates on click.
        let help_pos = h.find_text("Help").expect("Help menu button");
        h.click_at(help_pos);
        h.idle_frame();
        let check_pos = h
            .find_text_last("Check for updates…")
            .expect("Check for updates… menu item");
        let reqs = h.click_at_requests(check_pos);
        assert!(h.editor.ui.update.open);
        assert_eq!(h.editor.ui.update.status, UpdateCheckStatus::Checking);
        assert!(
            reqs.contains(&EditorRequest::CheckForUpdates),
            "expected CheckForUpdates, got {reqs:?}"
        );

        // 2. With a non-skipped UpdateInfo, status bar paints "Update available" and clicking opens dialog.
        h.editor.ui.update.open = false;
        h.editor.ui.update.status = UpdateCheckStatus::Available(sample_update_info(true));
        h.idle_frame();
        h.idle_frame();

        let pill_pos = h
            .find_text("Update available")
            .expect("status bar Update available pill");
        h.click_at(pill_pos);
        h.idle_frame();
        assert!(h.editor.ui.update.open);
        assert!(h.find_text("Software update").is_some());
        assert!(h.find_text("CourseCraft 0.4.5 is available").is_some());

        // 3. Help menu also shows the quiet "Update available" hint and opens without dropping state.
        h.editor.ui.update.open = false;
        h.idle_frame();
        let help_pos = h.find_text("Help").expect("Help menu button");
        h.click_at(help_pos);
        h.idle_frame();
        assert!(h.find_text_last("Check for updates…").is_some());
        assert!(
            h.find_texts("Update available").len() >= 2,
            "expected Update available in both status bar and Help menu"
        );
        let check_pos = h
            .find_text_last("Check for updates…")
            .expect("Check for updates… menu item");
        let reqs = h.click_at_requests(check_pos);
        assert!(h.editor.ui.update.open);
        assert!(matches!(
            h.editor.ui.update.status,
            UpdateCheckStatus::Available(_)
        ));
        assert!(
            !reqs.contains(&EditorRequest::CheckForUpdates),
            "clicking Check for updates… with a known non-skipped update should show it immediately"
        );

        // 4. Dialog renders cleanly over the welcome screen when show_welcome == true.
        h.editor.ui.show_welcome = true;
        h.editor.ui.update.open = true;
        h.idle_frame();
        h.idle_frame();
        assert!(h.find_text("Software update").is_some());
        assert!(h.find_text("CourseCraft 0.4.5 is available").is_some());
    }

    /// UI-04: Web editor isolation (`updates_available == false` by default).
    #[test]
    fn ui04_web_editor_isolation() {
        let mut h = TestHarness::new(sample_course());
        assert!(!h.editor.updates_available);
        h.editor.ui.update.status = UpdateCheckStatus::Available(sample_update_info(true));
        h.idle_frame();
        h.idle_frame();

        // Status bar never shows "Update available".
        assert!(h.find_text("Update available").is_none());

        // Help menu omits "Check for updates…" (and old "Check for updates").
        let help_pos = h.find_text("Help").expect("Help menu button");
        h.click_at(help_pos);
        h.idle_frame();
        assert!(
            h.find_text_last("Report a problem…").is_some(),
            "Help menu should be open"
        );
        assert!(h.find_text("Check for updates…").is_none());
        assert!(h.find_text("Check for updates").is_none());
        assert!(h.find_text("Update available").is_none());

        // Preferences omits "Check for updates automatically".
        h.editor.open_preferences();
        h.idle_frame();
        h.idle_frame();
        assert!(
            h.find_text("Check spelling as you type").is_some(),
            "Preferences dialog should be open"
        );
        assert!(h.find_text("Check for updates automatically").is_none());
        assert!(h.find_text("Updates").is_none());
    }
}
