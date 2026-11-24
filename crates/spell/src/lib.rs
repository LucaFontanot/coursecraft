//! Spell checking with a bundled English (US) dictionary.
//!
//! CourseCraft wraps the `spellbook` crate with a Hunspell `en_US` dictionary
//! that is compiled into the binary with `include_str!`. Parsing the dictionary
//! takes a little while, so create one [`Speller`] and keep it: don't rebuild it
//! in a render loop.
//!
//! There is no file access, thread or platform API here, so this crate also
//! builds for `wasm32-unknown-unknown`.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[cfg(test)]
mod tests;

/// One misspelled word found in a text.
///
/// `start` and `end` are byte offsets into the original text. Both are `char`
/// boundaries, so the range can be sliced directly; `word` is that slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Misspelling {
    pub start: usize,
    pub end: usize,
    pub word: String,
}

/// An error while building a [`Speller`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellError {
    Dictionary(String),
}

impl fmt::Display for SpellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpellError::Dictionary(message) => write!(f, "spell dictionary error: {message}"),
        }
    }
}

impl std::error::Error for SpellError {}

/// A spell checker with a bundled English (US) dictionary and a personal word
/// list.
///
/// Building the dictionary is relatively slow: create one `Speller` (for
/// example with [`Speller::english`]) and keep it for the session rather than
/// rebuilding it for each check.
pub struct Speller {
    dictionary: spellbook::Dictionary,
    /// Lower-cased word to the spelling first added by the author. Matching on
    /// the key makes the personal list case-insensitive and also accepts the
    /// capitalised form of every added word.
    personal: BTreeMap<String, String>,
}

impl Speller {
    /// The bundled English (US) dictionary, parsed from `include_str!` of the
    /// two dictionary files. Nothing is read from disk at run time.
    pub fn english() -> Result<Speller, SpellError> {
        let aff = include_str!("../dictionaries/en_US.aff");
        let dic = include_str!("../dictionaries/en_US.dic");
        let dictionary = spellbook::Dictionary::new(aff, dic)
            .map_err(|error| SpellError::Dictionary(error.to_string()))?;
        Ok(Speller {
            dictionary,
            personal: BTreeMap::new(),
        })
    }

    /// Find the misspelled words in `text`, in reading order.
    pub fn misspellings(&self, text: &str) -> Vec<Misspelling> {
        let mut found = Vec::new();
        for (start, run) in non_whitespace_runs(text) {
            if is_url(run) || is_email(run) {
                continue;
            }
            self.check_run(run, start, &mut found);
        }
        found
    }

    /// Check a single word with the same rules [`Speller::misspellings`] uses.
    pub fn is_correct(&self, word: &str) -> bool {
        self.word_is_correct(word)
    }

    /// Up to five suggestions for `word`, best first. The word itself and
    /// duplicates are never returned.
    pub fn suggestions(&self, word: &str) -> Vec<String> {
        let mut candidates = Vec::new();
        self.dictionary.suggest(word, &mut candidates);
        let mut seen = BTreeSet::new();
        let mut out = Vec::new();
        for candidate in candidates {
            if out.len() >= 5 {
                break;
            }
            if candidate == word || !seen.insert(candidate.clone()) {
                continue;
            }
            out.push(candidate);
        }
        out
    }

    /// Add a word to the personal list. Matching is case-insensitive, so this
    /// also makes the word's capitalised form correct.
    pub fn add_word(&mut self, word: &str) {
        let key = word.to_lowercase();
        if !key.is_empty() {
            self.personal.entry(key).or_insert_with(|| word.to_string());
        }
    }

    /// Remove a word from the personal list (case-insensitively), making it
    /// misspelled again unless the bundled dictionary knows it.
    pub fn remove_word(&mut self, word: &str) {
        self.personal.remove(&word.to_lowercase());
    }

    /// The personal word list, sorted, in the spelling each word was added
    /// with.
    pub fn words(&self) -> Vec<String> {
        self.personal.values().cloned().collect()
    }

    fn check_run(&self, run: &str, run_start: usize, found: &mut Vec<Misspelling>) {
        let mut pos = 0usize;
        let mut prev_ch: Option<char> = None;
        while let Some(ch) = run.get(pos..).and_then(|s| s.chars().next()) {
            if ch == '%' {
                let after_open = pos + ch.len_utf8();
                pos = match run.get(after_open..).and_then(|rest| rest.find('%')) {
                    Some(rel) => after_open + rel + 1,
                    None => after_open,
                };
                prev_ch = Some('%');
                continue;
            }
            if !ch.is_alphabetic() {
                pos += ch.len_utf8();
                prev_ch = Some(ch);
                continue;
            }

            let word_start = pos;
            let prev_is_digit = prev_ch.is_some_and(char::is_numeric);
            let mut end = pos + ch.len_utf8();
            let mut previous_is_letter = true;
            let mut last_word_ch = ch;
            let mut next_ch: Option<char> = None;
            let mut rest = run.get(end..).unwrap_or("").chars();
            while let Some(c) = rest.next() {
                if c.is_alphabetic() {
                    end += c.len_utf8();
                    previous_is_letter = true;
                    last_word_ch = c;
                } else if is_apostrophe(c)
                    && previous_is_letter
                    && rest.clone().next().is_some_and(char::is_alphabetic)
                {
                    end += c.len_utf8();
                    previous_is_letter = false;
                    last_word_ch = c;
                } else {
                    next_ch = Some(c);
                    break;
                }
            }
            let touches_digit = prev_is_digit || next_ch.is_some_and(char::is_numeric);
            if let Some(word) = run.get(word_start..end)
                && !should_skip(word, touches_digit)
                && !self.word_is_correct(word)
            {
                found.push(Misspelling {
                    start: run_start + word_start,
                    end: run_start + end,
                    word: word.to_string(),
                });
            }
            pos = end;
            prev_ch = Some(last_word_ch);
        }
    }

    fn word_is_correct(&self, word: &str) -> bool {
        if self.personal.is_empty() {
            if self.dictionary.check(word) {
                return true;
            }
            if word.chars().any(char::is_uppercase) && (is_capitalised(word) || is_all_upper(word))
            {
                let lower = word.to_lowercase();
                return lower != word && self.dictionary.check(&lower);
            }
            return false;
        }
        let has_upper = word.chars().any(char::is_uppercase);
        let lower = has_upper.then(|| word.to_lowercase());
        let lower_ref = lower.as_deref().unwrap_or(word);
        if self.personal.contains_key(lower_ref) {
            return true;
        }
        if self.dictionary.check(word) {
            return true;
        }
        if let Some(ref lower) = lower
            && lower != word
            && (is_capitalised(word) || is_all_upper(word))
        {
            return self.dictionary.check(lower);
        }
        false
    }
}

/// Maximal runs of non-whitespace characters, with their byte offset in `text`.
fn non_whitespace_runs(text: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut iter = text.char_indices();
    std::iter::from_fn(move || {
        let (start, _) = iter.find(|&(_, ch)| !ch.is_whitespace())?;
        let end = iter
            .find(|&(_, ch)| ch.is_whitespace())
            .map_or(text.len(), |(byte, _)| byte);
        text.get(start..end).map(|run| (start, run))
    })
}

fn is_url(run: &str) -> bool {
    run.get(..7)
        .is_some_and(|s| s.eq_ignore_ascii_case("http://"))
        || run
            .get(..8)
            .is_some_and(|s| s.eq_ignore_ascii_case("https://"))
        || run.get(..4).is_some_and(|s| s.eq_ignore_ascii_case("www."))
}

fn is_email(run: &str) -> bool {
    run.contains('@')
}

fn is_apostrophe(ch: char) -> bool {
    ch == '\'' || ch == '\u{2019}'
}

fn should_skip(word: &str, touches_digit: bool) -> bool {
    if touches_digit {
        return true;
    }
    let mut letter_count = 0usize;
    let mut all_upper = true;
    for ch in word.chars() {
        if ch.is_alphabetic() {
            letter_count += 1;
            if !ch.is_uppercase() {
                all_upper = false;
            }
        }
    }
    letter_count < 2 || all_upper
}

/// First letter upper, every other letter lower.
fn is_capitalised(word: &str) -> bool {
    let mut letters = word.chars().filter(|ch| ch.is_alphabetic());
    match letters.next() {
        Some(first) if first.is_uppercase() => letters.all(|ch| ch.is_lowercase()),
        _ => false,
    }
}

fn is_all_upper(word: &str) -> bool {
    let mut letters = word.chars().filter(|ch| ch.is_alphabetic());
    match letters.next() {
        Some(_) => letters.all(|ch| ch.is_uppercase()),
        None => false,
    }
}
