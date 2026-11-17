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
        let chars: Vec<(usize, char)> = run.char_indices().collect();
        let count = chars.len();
        let mut i = 0;
        while i < count {
            let ch = chars[i].1;
            if ch == '%' {
                i = skip_variable(&chars, i);
                continue;
            }
            if !ch.is_alphabetic() {
                i += 1;
                continue;
            }

            let (end, next) = word_end(&chars, i);
            let touches_digit = (i > 0 && chars[i - 1].1.is_numeric())
                || (next < count && chars[next].1.is_numeric());
            let word = &run[chars[i].0..end];
            if !should_skip(word, touches_digit) && !self.word_is_correct(word) {
                found.push(Misspelling {
                    start: run_start + chars[i].0,
                    end: run_start + end,
                    word: word.to_string(),
                });
            }
            i = next;
        }
    }

    fn word_is_correct(&self, word: &str) -> bool {
        let lower = word.to_lowercase();
        if self.personal.contains_key(&lower) {
            return true;
        }
        if self.dictionary.check(word) {
            return true;
        }
        if lower != word && (is_capitalised(word) || is_all_upper(word)) {
            return self.dictionary.check(&lower);
        }
        false
    }
}

/// Maximal runs of non-whitespace characters, with their byte offset in `text`.
fn non_whitespace_runs(text: &str) -> Vec<(usize, &str)> {
    let mut runs = Vec::new();
    let mut start = None;
    for (byte, ch) in text.char_indices() {
        if ch.is_whitespace() {
            if let Some(from) = start.take() {
                runs.push((from, &text[from..byte]));
            }
        } else if start.is_none() {
            start = Some(byte);
        }
    }
    if let Some(from) = start {
        runs.push((from, &text[from..]));
    }
    runs
}

fn is_url(run: &str) -> bool {
    let lower = run.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("www.")
}

fn is_email(run: &str) -> bool {
    run.contains('@')
}

fn is_apostrophe(ch: char) -> bool {
    ch == '\'' || ch == '\u{2019}'
}

/// Skip from a `%` to the next `%` in the same run (a `%name%` reference).
fn skip_variable(chars: &[(usize, char)], at: usize) -> usize {
    (at + 1..chars.len())
        .find(|&i| chars[i].1 == '%')
        .map_or(at + 1, |close| close + 1)
}

/// The end byte offset (relative to the run) of the word starting at `at`, and
/// the index of the first character after it. Apostrophes count only between
/// letters, so `don't` is one word while `'hello` and `dogs'` are not.
fn word_end(chars: &[(usize, char)], at: usize) -> (usize, usize) {
    let mut end = chars[at].0 + chars[at].1.len_utf8();
    let mut previous_is_letter = true;
    let mut i = at + 1;
    while i < chars.len() {
        let ch = chars[i].1;
        if ch.is_alphabetic() {
            end += ch.len_utf8();
            previous_is_letter = true;
            i += 1;
        } else if is_apostrophe(ch)
            && previous_is_letter
            && i + 1 < chars.len()
            && chars[i + 1].1.is_alphabetic()
        {
            end += ch.len_utf8();
            previous_is_letter = false;
            i += 1;
        } else {
            break;
        }
    }
    (end, i)
}

fn should_skip(word: &str, touches_digit: bool) -> bool {
    if touches_digit {
        return true;
    }
    let letters: Vec<char> = word.chars().filter(|ch| ch.is_alphabetic()).collect();
    letters.len() < 2 || letters.iter().all(|ch| ch.is_uppercase())
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
