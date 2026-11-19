//! Spell checking in the editor: wavy underlines, the word menu on a squiggle
//! and the course-wide Check Spelling window.
//!
//! One [`Speller`] is owned by the [`Editor`] and created the first time it is
//! needed. Misspellings (and their suggestions) are cached on the text's hash,
//! so an unchanged string is not re-checked on every frame. Words the author
//! adds live in the desktop preferences; words they ignore last for this
//! session only.

use std::collections::{BTreeMap, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};

use coursecraft_course::{Hit, course_strings};
use coursecraft_spell::{Misspelling, Speller};
use egui::text::{CCursor, LayoutJob, TextFormat};
use egui::{Color32, Galley, Painter, Pos2, Rect, RichText, Stroke, Ui, pos2};
use serde_json::{Value as Json, json};

use crate::dialog::{Dialog, Form};
use crate::{Editor, EditorRequest};

/// Colour of the wavy line drawn under a misspelled word.
pub const SQUIGGLE_COLOR: Color32 = Color32::from_rgb(224, 74, 74);

/// One misspelling with the screen rects of its underline. A word that wraps
/// onto two rows has one rect per row.
#[derive(Debug, Clone, PartialEq)]
pub struct WordRects {
    /// Byte range of the word in the text being edited.
    pub start: usize,
    pub end: usize,
    pub word: String,
    pub rects: Vec<Rect>,
}

/// A cached misspelling with its (at most five) suggestions.
#[derive(Debug, Clone, PartialEq)]
pub struct SpelledWord {
    pub misspelling: Misspelling,
    pub suggestions: Vec<String>,
}

/// Misspellings keyed by the hash of the text that produced them.
#[derive(Debug, Clone, Default)]
pub struct SpellCache {
    entries: Vec<(u64, Vec<SpelledWord>)>,
    /// Suggestions per word. Working them out is the slow part of spell
    /// checking, so it happens only when a menu or the window needs them.
    suggestions: Vec<(String, Vec<String>)>,
}

impl SpellCache {
    /// Number of different texts kept. Oldest first, so a long editing session
    /// cannot grow this without bound.
    const CAPACITY: usize = 12;

    pub fn get(&self, hash: u64) -> Option<&Vec<SpelledWord>> {
        self.entries
            .iter()
            .find(|(key, _)| *key == hash)
            .map(|(_, words)| words)
    }

    pub fn insert(&mut self, hash: u64, words: Vec<SpelledWord>) {
        if self.entries.len() >= Self::CAPACITY {
            self.entries.remove(0);
        }
        self.entries.push((hash, words));
    }

    /// Number of words whose suggestions are kept.
    const SUGGESTION_CAPACITY: usize = 64;

    pub fn suggestions(&self, word: &str) -> Option<&Vec<String>> {
        self.suggestions
            .iter()
            .find(|(key, _)| key == word)
            .map(|(_, list)| list)
    }

    pub fn insert_suggestions(&mut self, word: &str, list: Vec<String>) {
        if self.suggestions.len() >= Self::SUGGESTION_CAPACITY {
            self.suggestions.remove(0);
        }
        self.suggestions.push((word.to_owned(), list));
    }

    /// Forget everything: personal and ignored words change what is correct.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.suggestions.clear();
    }
}

fn hash_text(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

// ---------------------------------------------------------------- rects

/// Compute the underline rects for `misspellings` from a text edit's galley.
///
/// Byte ranges from [`Speller::misspellings`] are converted to character
/// indices, which is what egui's cursor API uses. For a word split by wrapping,
/// the intersection with each row is underlined separately.
#[must_use]
pub fn underline_rects(
    galley: &Galley,
    galley_pos: Pos2,
    text: &str,
    misspellings: &[Misspelling],
) -> Vec<WordRects> {
    let mut out = Vec::new();
    for miss in misspellings {
        if miss.start > miss.end
            || !text.is_char_boundary(miss.start)
            || !text.is_char_boundary(miss.end)
        {
            continue;
        }
        let Some(head) = text.get(..miss.start) else {
            continue;
        };
        let Some(word) = text.get(miss.start..miss.end) else {
            continue;
        };
        let first = head.chars().count();
        let last = first.saturating_add(word.chars().count());
        if last == first {
            continue;
        }

        let mut row_start = 0usize;
        let mut rects = Vec::new();
        for row in &galley.rows {
            let with_newline = row.char_count_including_newline().0;
            let without_newline = row.char_count_excluding_newline().0.min(with_newline);
            let segment_start = first.max(row_start);
            let segment_end = last.min(row_start.saturating_add(without_newline));
            if segment_start < segment_end {
                let left = galley.pos_from_cursor(CCursor::new(segment_start)).left();
                let right = galley.pos_from_cursor(CCursor::new(segment_end)).right();
                if right > left {
                    let rect =
                        Rect::from_min_max(pos2(left, row.min_y()), pos2(right, row.max_y()))
                            .translate(galley_pos.to_vec2());
                    if rect.is_finite() {
                        rects.push(rect);
                    }
                }
            }
            row_start = row_start.saturating_add(with_newline);
        }
        if !rects.is_empty() {
            out.push(WordRects {
                start: miss.start,
                end: miss.end,
                word: miss.word.clone(),
                rects,
            });
        }
    }
    out
}

/// Draw a red wavy line under each misspelled word in a text edit.
pub fn spell_underlines(
    painter: &Painter,
    galley: &Galley,
    galley_pos: Pos2,
    text: &str,
    misspellings: &[Misspelling],
    color: Color32,
) {
    for word in underline_rects(galley, galley_pos, text, misspellings) {
        for rect in word.rects {
            paint_wavy(painter, rect, color);
        }
    }
}

fn paint_wavy(painter: &Painter, rect: Rect, color: Color32) {
    if rect.width() < 1.0 || !rect.is_finite() {
        return;
    }
    let baseline = rect.bottom() - 1.5;
    let amplitude = 1.4;
    let step = 3.0;
    let mut points = Vec::new();
    let mut x = rect.left();
    let mut up = false;
    while x < rect.right() - step {
        points.push(pos2(x, baseline + if up { -amplitude } else { amplitude }));
        x += step;
        up = !up;
    }
    points.push(pos2(rect.right(), baseline));
    if points.len() >= 2 {
        painter.add(egui::Shape::line(points, Stroke::new(1.0, color)));
    }
}

/// The squiggled word under a pointer position, if any.
#[must_use]
pub fn word_at(words: &[WordRects], pos: Pos2) -> Option<&WordRects> {
    words
        .iter()
        .find(|word| word.rects.iter().any(|rect| rect.contains(pos)))
}

/// What the author chose in a squiggled word's context menu. Byte ranges refer
/// to the text being edited.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WordMenuOutcome {
    pub replace: Option<(usize, usize, String)>,
    pub add: Option<String>,
    pub ignore: Option<String>,
}

/// Fill an open text-edit context menu with spelling actions for the word under
/// `click_pos`. Called from a `Response::context_menu` closure; if the click
/// was not on a squiggle it adds nothing and egui closes the empty menu.
pub fn show_word_menu(
    ui: &mut Ui,
    click_pos: Pos2,
    words: &[WordRects],
    suggestions: &BTreeMap<String, Vec<String>>,
) -> WordMenuOutcome {
    let mut outcome = WordMenuOutcome::default();
    let Some(word) = word_at(words, click_pos) else {
        return outcome;
    };
    let options = suggestions.get(&word.word).cloned().unwrap_or_default();
    for suggestion in options {
        if ui.button(&suggestion).clicked() {
            outcome.replace = Some((word.start, word.end, suggestion));
            ui.close();
            break;
        }
    }
    if !outcome.replace.is_some() {
        ui.separator();
        if ui.button("Add to Dictionary").clicked() {
            outcome.add = Some(word.word.clone());
            ui.close();
        }
        if ui.button("Ignore").clicked() {
            outcome.ignore = Some(word.word.clone());
            ui.close();
        }
    }
    outcome
}

/// Apply a choice from a text box's word menu to its buffer. The caller owns
/// the buffer, so this only reports what changed.
pub fn apply_word_menu(
    word: WordMenuOutcome,
    buffer: &mut String,
) -> (bool, Vec<String>, Vec<String>) {
    let mut changed = false;
    if let Some((start, end, replacement)) = word.replace
        && buffer.is_char_boundary(start)
        && buffer.is_char_boundary(end)
        && start <= end
    {
        buffer.replace_range(start..end, &replacement);
        changed = true;
    }
    let added = word.add.into_iter().collect();
    let ignored = word.ignore.into_iter().collect();
    (changed, added, ignored)
}

// ---------------------------------------------------------------- editor

impl Editor {
    /// The one speller for this session, built on first use. The bundled
    /// dictionary takes a moment to parse, so it is never rebuilt per frame.
    pub fn speller(&mut self) -> Option<&mut Speller> {
        if self.spell.is_none() && !self.spell_tried {
            self.spell_tried = true;
            match Speller::english() {
                Ok(mut speller) => {
                    for word in &self.ui.spell_words {
                        speller.add_word(word);
                    }
                    self.spell = Some(speller);
                }
                Err(error) => self.set_status(format!("Spell check unavailable: {error}")),
            }
        }
        self.spell.as_mut()
    }

    /// Suggestions for one word, best first, worked out once and cached.
    pub fn spell_suggestions(&mut self, word: &str) -> Vec<String> {
        if let Some(list) = self.spell_cache.suggestions(word) {
            return list.clone();
        }
        let list = self
            .speller()
            .map(|speller| speller.suggestions(word))
            .unwrap_or_default();
        self.spell_cache.insert_suggestions(word, list.clone());
        list
    }

    /// Suggestions for every word in `words`, keyed by the word.
    pub fn spell_suggestion_map(
        &mut self,
        words: &[SpelledWord],
    ) -> std::collections::BTreeMap<String, Vec<String>> {
        words
            .iter()
            .map(|word| {
                let text = word.misspelling.word.clone();
                let list = self.spell_suggestions(&text);
                (text, list)
            })
            .collect()
    }

    /// Misspellings for `text`, or none when as-you-type checking is off.
    /// Ignored words are skipped for this session. `suggestions` is left
    /// empty: they are slow to work out, so ask [`Editor::spell_suggestions`]
    /// only when a menu needs them.
    #[must_use]
    pub fn spell_words(&mut self, text: &str) -> Vec<SpelledWord> {
        if !self.ui.prefs.spell_check_as_you_type {
            return Vec::new();
        }
        let hash = hash_text(text);
        if let Some(words) = self.spell_cache.get(hash) {
            return words.clone();
        }
        let ignored = self.ui.spell_ignored.clone();
        let words: Vec<SpelledWord> = match self.speller() {
            Some(speller) => speller
                .misspellings(text)
                .into_iter()
                .filter(|miss| !is_ignored(&ignored, &miss.word))
                .map(|miss| SpelledWord {
                    suggestions: Vec::new(),
                    misspelling: miss,
                })
                .collect(),
            None => Vec::new(),
        };
        self.spell_cache.insert(hash, words.clone());
        words
    }

    /// Add a word to the personal dictionary, in memory and in the preferences
    /// the app shell saves. One request marks prefs dirty, as other prefs do.
    pub fn add_spell_word(&mut self, word: &str) {
        let word = word.trim();
        if word.is_empty() {
            return;
        }
        if let Some(speller) = self.speller() {
            speller.add_word(word);
        }
        if !self
            .ui
            .spell_words
            .iter()
            .any(|known| known.eq_ignore_ascii_case(word))
        {
            self.ui.spell_words.push(word.to_owned());
        }
        self.spell_changed();
        self.push_spell_prefs_request();
    }

    /// Ignore a word for this session only; nothing is written to prefs.
    pub fn ignore_spell_word(&mut self, word: &str) {
        if word.is_empty() {
            return;
        }
        if !self
            .ui
            .spell_ignored
            .iter()
            .any(|known| known.eq_ignore_ascii_case(word))
        {
            self.ui.spell_ignored.push(word.to_owned());
        }
        self.spell_changed();
    }

    /// Ignore one occurrence in one field, for this session only.
    pub fn ignore_spell_at(&mut self, location: &coursecraft_course::Location, start: usize) {
        let entry = (location.clone(), start);
        if !self.ui.spell_ignored_at.contains(&entry) {
            self.ui.spell_ignored_at.push(entry);
        }
        self.spell_changed();
    }

    /// Load the personal dictionary and the as-you-type preference at start-up.
    pub fn set_spell_preferences(&mut self, check_as_you_type: bool, words: Vec<String>) {
        self.ui.prefs.spell_check_as_you_type = check_as_you_type;
        self.ui.spell_words = words
            .into_iter()
            .map(|word| word.trim().to_owned())
            .filter(|word| !word.is_empty())
            .collect();
        if let Some(speller) = self.spell.as_mut() {
            for word in &self.ui.spell_words {
                speller.add_word(word);
            }
        }
        self.spell_changed();
    }

    fn spell_changed(&mut self) {
        self.spell_cache.clear();
        self.spell_version = self.spell_version.wrapping_add(1);
        self.ui.spell_check.key = None;
    }

    fn push_spell_prefs_request(&mut self) {
        let prefs = &self.ui.prefs;
        self.pending_requests.push(EditorRequest::SetPreferences {
            autosave_minutes: prefs.autosave_minutes,
            versions_to_keep: prefs.versions_to_keep,
            default_slide: prefs.default_slide,
            spell_check_as_you_type: prefs.spell_check_as_you_type,
            spell_words: self.ui.spell_words.clone(),
        });
    }
}

fn is_ignored(ignored: &[String], word: &str) -> bool {
    ignored.iter().any(|known| known.eq_ignore_ascii_case(word))
}

// ---------------------------------------------------------------- window

/// One misspelling found while walking the course, in walk order.
#[derive(Debug, Clone, PartialEq)]
pub struct SpellingIssue {
    pub item: coursecraft_course::StringItem,
    pub misspelling: Misspelling,
}

/// State of the Check Spelling window.
#[derive(Debug, Clone, Default)]
pub struct SpellingState {
    pub open: bool,
    pub index: usize,
    pub change_to: String,
    pub issues: Vec<SpellingIssue>,
    pub message: Option<String>,
    pub(crate) key: Option<(u64, u64)>,
}

/// Open the window, starting at the first issue.
pub fn open(editor: &mut Editor) {
    if !editor.ui.spell_check.open {
        editor.ui.spell_check.index = 0;
        editor.ui.spell_check.change_to.clear();
        editor.ui.spell_check.message = None;
    }
    editor.ui.spell_check.open = true;
}

/// Escape closes the window before the canvas can act on it.
pub fn consume_escape(editor: &mut Editor, ctx: &egui::Context) {
    if editor.ui.spell_check.open
        && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        editor.ui.spell_check.open = false;
    }
}

/// Walk every author-visible string, skipping ignored words and occurrences.
/// The Check Spelling window uses this; it does not depend on the as-you-type
/// preference.
#[must_use]
pub fn collect_issues(editor: &mut Editor) -> Vec<SpellingIssue> {
    let strings = course_strings(&editor.session.course);
    let ignored_words = editor.ui.spell_ignored.clone();
    let ignored_at = editor.ui.spell_ignored_at.clone();
    let Some(speller) = editor.speller() else {
        return Vec::new();
    };
    let mut issues = Vec::new();
    for item in strings {
        for miss in speller.misspellings(&item.text) {
            if is_ignored(&ignored_words, &miss.word)
                || ignored_at
                    .iter()
                    .any(|(location, start)| location == &item.location && *start == miss.start)
            {
                continue;
            }
            issues.push(SpellingIssue {
                item: item.clone(),
                misspelling: miss,
            });
        }
    }
    issues
}

fn refresh(editor: &mut Editor) {
    let key = (editor.session.revision, editor.spell_version);
    if editor.ui.spell_check.key == Some(key) {
        return;
    }
    let issues = collect_issues(editor);
    let state = &mut editor.ui.spell_check;
    state.issues = issues;
    state.key = Some(key);
    state.index = state.index.min(state.issues.len().saturating_sub(1));
    sync_change_to(state);
}

fn current_word(state: &SpellingState) -> String {
    state
        .issues
        .get(state.index)
        .map_or_else(String::new, |issue| issue.misspelling.word.clone())
}

fn sync_change_to(state: &mut SpellingState) {
    state.index = state.index.min(state.issues.len().saturating_sub(1));
    state.change_to = current_word(state);
}

/// Replace the word at `index` in its one location, as one undo step.
pub fn change_one(editor: &mut Editor, index: usize, replacement: &str) -> bool {
    let Some(issue) = editor.ui.spell_check.issues.get(index).cloned() else {
        return false;
    };
    run_change(editor, &issue, replacement, true)
}

/// Replace the word in every location, as one undo step.
pub fn change_all(editor: &mut Editor, index: usize, replacement: &str) -> bool {
    let Some(issue) = editor.ui.spell_check.issues.get(index).cloned() else {
        return false;
    };
    run_change(editor, &issue, replacement, false)
}

fn run_change(editor: &mut Editor, issue: &SpellingIssue, replacement: &str, one: bool) -> bool {
    let mut params = json!({
        "query": issue.misspelling.word,
        "replacement": replacement,
        "case": true,
        "whole_word": true,
    });
    if one {
        let only = serde_json::to_value(&issue.item.location).unwrap_or(Json::Null);
        params["only"] = json!([only]);
    }
    let replaced = editor
        .run("course.replace", params)
        .and_then(|reply| reply.get("replaced").and_then(Json::as_u64))
        .unwrap_or(0);
    if replaced > 0 {
        refresh(editor);
    }
    replaced > 0
}

/// Ignore one occurrence, or every occurrence of its word, for this session.
pub fn ignore_issue(editor: &mut Editor, index: usize, all: bool) {
    let Some(issue) = editor.ui.spell_check.issues.get(index).cloned() else {
        return;
    };
    if all {
        editor.ignore_spell_word(&issue.misspelling.word);
    } else {
        editor.ignore_spell_at(&issue.item.location, issue.misspelling.start);
    }
    refresh(editor);
    sync_change_to(&mut editor.ui.spell_check);
}

/// Add the word at `index` to the personal dictionary.
pub fn add_issue_word(editor: &mut Editor, index: usize) {
    let Some(word) = editor
        .ui
        .spell_check
        .issues
        .get(index)
        .map(|issue| issue.misspelling.word.clone())
    else {
        return;
    };
    editor.add_spell_word(&word);
    refresh(editor);
    sync_change_to(&mut editor.ui.spell_check);
}

/// Draw the Check Spelling window when it is open.
pub fn show_spelling_window(editor: &mut Editor, ctx: &egui::Context) {
    if !editor.ui.spell_check.open {
        return;
    }
    refresh(editor);

    let index = editor.ui.spell_check.index;
    let word = editor
        .ui
        .spell_check
        .issues
        .get(index)
        .map(|issue| issue.misspelling.word.clone());
    let suggestions = word
        .as_deref()
        .map(|word| editor.spell_suggestions(word))
        .unwrap_or_default();

    let mut open = true;
    let mut action = WindowAction::None;
    let mut change_to = editor.ui.spell_check.change_to.clone();
    Dialog::new("Check Spelling")
        .width(520.0)
        .open(&mut open)
        .show(ctx, |ui| {
            window_body(
                &editor.ui.spell_check,
                &suggestions,
                ui,
                &mut change_to,
                &mut action,
            );
        });
    if !open {
        editor.ui.spell_check.open = false;
    }
    editor.ui.spell_check.change_to = change_to;
    apply_action(editor, action);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowAction {
    None,
    Change,
    ChangeAll,
    Ignore,
    IgnoreAll,
    Add,
    Close,
    Goto(usize),
}

fn window_body(
    state: &SpellingState,
    suggestions: &[String],
    ui: &mut Ui,
    change_to: &mut String,
    action: &mut WindowAction,
) {
    let p = coursecraft_render::palette(ui.ctx());
    if state.issues.is_empty() {
        ui.label(RichText::new("Spelling check complete.").color(p.text));
        return;
    }
    let index = state.index.min(state.issues.len().saturating_sub(1));
    let issue = &state.issues[index];
    let done = index.saturating_add(1);
    ui.label(
        RichText::new(format!("{} of {}", done, state.issues.len()))
            .color(p.text_secondary)
            .small(),
    );
    ui.horizontal(|ui| {
        ui.label(RichText::new(&issue.item.label).strong());
        if ui.link("Go to").clicked() {
            *action = WindowAction::Goto(index);
        }
    });

    let (snippet, highlight) = coursecraft_course::snippet(
        &issue.item.text,
        issue.misspelling.start..issue.misspelling.end,
    );
    let mut job = LayoutJob::default();
    job.wrap.max_width = ui.available_width();
    let font = egui::FontId::proportional(coursecraft_render::SIZE_BODY);
    job.append(
        snippet.get(..highlight.start).unwrap_or(""),
        0.0,
        TextFormat::simple(font.clone(), p.text),
    );
    job.append(
        snippet.get(highlight.clone()).unwrap_or(""),
        0.0,
        TextFormat::simple(font.clone(), SQUIGGLE_COLOR),
    );
    job.append(
        snippet.get(highlight.end..).unwrap_or(""),
        0.0,
        TextFormat::simple(font, p.text),
    );
    ui.add_space(6.0);
    ui.label(job);

    let form = Form::new(ui, &["Word", "Change to"]);
    form.row(ui, "Word", |ui, _| {
        ui.label(RichText::new(&issue.misspelling.word).strong());
    });
    form.row(ui, "Change to", |ui, width| {
        ui.add(
            egui::TextEdit::singleline(change_to)
                .hint_text("Type a replacement")
                .desired_width(width),
        );
    });

    ui.add_space(6.0);
    ui.label(RichText::new("Suggestions").color(p.text_secondary).small());
    for suggestion in suggestions {
        if ui.selectable_label(false, suggestion).clicked() {
            *change_to = suggestion.clone();
        }
    }

    ui.separator();
    ui.horizontal_wrapped(|ui| {
        if ui.button("Change").clicked() {
            *action = WindowAction::Change;
        }
        if ui.button("Change All").clicked() {
            *action = WindowAction::ChangeAll;
        }
        if ui.button("Ignore").clicked() {
            *action = WindowAction::Ignore;
        }
        if ui.button("Ignore All").clicked() {
            *action = WindowAction::IgnoreAll;
        }
        if ui.button("Add to Dictionary").clicked() {
            *action = WindowAction::Add;
        }
        if ui.button("Close").clicked() {
            *action = WindowAction::Close;
        }
    });
}

fn apply_action(editor: &mut Editor, action: WindowAction) {
    match action {
        WindowAction::None => {}
        WindowAction::Close => editor.ui.spell_check.open = false,
        WindowAction::Change => {
            let replacement = editor.ui.spell_check.change_to.clone();
            if change_one(editor, editor.ui.spell_check.index, &replacement) {
                advance(editor);
            }
        }
        WindowAction::ChangeAll => {
            let replacement = editor.ui.spell_check.change_to.clone();
            if change_all(editor, editor.ui.spell_check.index, &replacement) {
                advance(editor);
            }
        }
        WindowAction::Ignore => {
            ignore_issue(editor, editor.ui.spell_check.index, false);
        }
        WindowAction::IgnoreAll => {
            ignore_issue(editor, editor.ui.spell_check.index, true);
        }
        WindowAction::Add => {
            add_issue_word(editor, editor.ui.spell_check.index);
        }
        WindowAction::Goto(index) => {
            if let Some(issue) = editor.ui.spell_check.issues.get(index).cloned() {
                go_to(editor, &issue);
            }
        }
    }
}

fn advance(editor: &mut Editor) {
    sync_change_to(&mut editor.ui.spell_check);
    editor.ui.spell_check.message = None;
}

fn go_to(editor: &mut Editor, issue: &SpellingIssue) {
    let location = &issue.item.location;
    let hit = Hit {
        slide: location.slide.clone(),
        object: location.object.clone(),
        layer: location.layer.clone(),
        field: location.field.clone(),
        label: issue.item.label.clone(),
        snippet: String::new(),
        snippet_match: 0..0,
        start: issue.misspelling.start,
        end: issue.misspelling.end,
    };
    crate::find::go_to(editor, &hit);
}
