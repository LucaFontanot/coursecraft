use std::collections::BTreeSet;
use std::time::Instant;

use crate::{Misspelling, Speller};

fn speller() -> Speller {
    Speller::english().expect("the bundled dictionary parses")
}

#[test]
fn correct_sentences_return_nothing() {
    let speller = speller();
    assert!(
        speller
            .misspellings("This is a short sentence with only correctly spelled words.")
            .is_empty()
    );
    assert!(speller.misspellings("").is_empty());
    assert!(speller.misspellings("   \n\t ").is_empty());
}

#[test]
fn common_typos_have_byte_ranges_and_suggestions() {
    let speller = speller();
    let text = "The teh recieve seperate answer.";
    let found = speller.misspellings(text);
    assert_eq!(
        found,
        vec![
            Misspelling {
                start: 4,
                end: 7,
                word: "teh".to_string(),
            },
            Misspelling {
                start: 8,
                end: 15,
                word: "recieve".to_string(),
            },
            Misspelling {
                start: 16,
                end: 24,
                word: "seperate".to_string(),
            },
        ]
    );
    assert!(speller.suggestions("teh").contains(&"the".to_string()));
    assert!(
        speller
            .suggestions("recieve")
            .contains(&"receive".to_string())
    );
    assert!(
        speller
            .suggestions("seperate")
            .contains(&"separate".to_string())
    );
}

#[test]
fn digit_touching_words_are_skipped() {
    let speller = speller();
    assert!(
        speller
            .misspellings("Play the mp3 file in the 2nd lesson.")
            .is_empty()
    );
    assert!(speller.misspellings("teh2 3teh mp3 2nd").is_empty());
}

#[test]
fn all_caps_and_single_letters_are_skipped() {
    let speller = speller();
    assert!(
        speller
            .misspellings("SCORM and LMS and API are acronyms.")
            .is_empty()
    );
    assert!(speller.misspellings("I am A, and I see a B.").is_empty());
}

#[test]
fn urls_and_emails_skip_the_whole_run() {
    let speller = speller();
    assert!(
        speller
            .misspellings("See http://example.com/teh and https://example.com/teh now")
            .is_empty()
    );
    assert!(
        speller
            .misspellings("Visit www.example.com/teh today")
            .is_empty()
    );
    assert!(
        speller
            .misspellings("Mail teh@example.com/teh now")
            .is_empty()
    );
    let found = speller.misspellings("teh http://example.com/teh");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].word, "teh");
}

#[test]
fn variable_references_are_skipped() {
    let speller = speller();
    assert!(
        speller
            .misspellings("Use %user_name% and %teh% here")
            .is_empty()
    );
    assert!(speller.misspellings("Hello%name%world").is_empty());
    let found = speller.misspellings("teh %name% teh");
    assert_eq!(found.len(), 2);
    assert!(found.iter().all(|item| item.word == "teh"));
}

#[test]
fn apostrophes_are_part_of_words_only_between_letters() {
    let speller = speller();
    assert!(
        speller
            .misspellings("Don't worry about the author’s notes.")
            .is_empty()
    );
    let found = speller.misspellings("'teh' and dogs' and 'hello");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].word, "teh");
    assert_eq!((found[0].start, found[0].end), (1, 4));
}

#[test]
fn hyphenated_words_are_checked_as_parts() {
    let speller = speller();
    assert!(
        speller
            .misspellings("A light-weight approach works.")
            .is_empty()
    );
    let found = speller.misspellings("light-teh");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].word, "teh");
}

#[test]
fn capitalisation_follows_sentence_case_and_all_caps() {
    let speller = speller();
    assert!(speller.misspellings("Hello world. HELLO WORLD.").is_empty());
    assert!(speller.is_correct("hello"));
    assert!(speller.is_correct("Hello"));
    assert!(speller.is_correct("HELLO"));
    assert!(!speller.is_correct("teh"));
    // Mixed case must be checked as written, not folded to lower case.
    let found = speller.misspellings("hEllo");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].word, "hEllo");
    assert!(!speller.is_correct("hEllo"));
}

#[test]
fn byte_ranges_are_char_boundaries_with_accented_text_and_emoji() {
    let speller = speller();
    let text = "😀 teh café naïve";
    let found = speller.misspellings(text);
    for item in &found {
        assert!(text.is_char_boundary(item.start));
        assert!(text.is_char_boundary(item.end));
        assert_eq!(&text[item.start..item.end], item.word);
    }
    let teh = found
        .iter()
        .find(|item| item.word == "teh")
        .expect("teh found");
    assert_eq!((teh.start, teh.end), (5, 8));
}

#[test]
fn personal_words_are_case_insensitive_and_can_be_removed() {
    let mut speller = speller();
    assert_eq!(speller.misspellings("CourseCraft is great.").len(), 1);
    speller.add_word("CourseCraft");
    assert!(speller.misspellings("CourseCraft is great.").is_empty());
    assert!(speller.misspellings("coursecraft").is_empty());
    assert!(speller.is_correct("COURSECRAFT"));
    assert!(speller.words().contains(&"CourseCraft".to_string()));
    speller.remove_word("courseCraft");
    assert_eq!(speller.misspellings("CourseCraft").len(), 1);
    assert!(!speller.is_correct("coursecraft"));
}

#[test]
fn personal_words_are_sorted() {
    let mut speller = speller();
    speller.add_word("zebra");
    speller.add_word("apple");
    speller.add_word("mango");
    assert_eq!(speller.words(), vec!["apple", "mango", "zebra"]);
}

#[test]
fn suggestions_are_limited_unique_and_never_the_word_itself() {
    let speller = speller();
    let suggestions = speller.suggestions("teh");
    assert!(suggestions.len() <= 5);
    assert!(suggestions.iter().all(|item| item != "teh"));
    let mut seen = BTreeSet::new();
    assert!(suggestions.iter().all(|item| seen.insert(item)));
    assert!(speller.suggestions("").is_empty());
}

#[test]
fn five_thousand_words_check_quickly() {
    let speller = speller();
    let paragraph = "This is a short paragraph with several common words that are spelled correctly and one teh mistake here and there. ";
    let text = paragraph.repeat(250);
    assert!(text.split_whitespace().count() >= 5_000);

    let started = Instant::now();
    let found = speller.misspellings(&text);
    let elapsed = started.elapsed();
    assert_eq!(found.len(), 250);
    assert!(
        elapsed.as_millis() < 200,
        "spell check took {} ms",
        elapsed.as_millis()
    );
}
