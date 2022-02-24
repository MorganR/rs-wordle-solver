#[macro_use]
extern crate assert_matches;

use wordle_solver::details::*;
use wordle_solver::*;

use std::io::Cursor;
use std::rc::Rc;
use std::result::Result;

macro_rules! assert_rc_eq {
    ($rc_vec:expr, $non_rc_vec:expr) => {
        assert_eq!(
            $rc_vec as &[Rc<str>],
            $non_rc_vec
                .iter()
                .map(|thing| Rc::from(*thing))
                .collect::<Vec<Rc<_>>>()
        );
    };
}

#[test]
fn word_bank_from_reader_succeeds() -> Result<(), WordleError> {
    let mut cursor = Cursor::new(String::from("\n\nworda\nwordb\n"));

    let word_bank = WordBank::from_reader(&mut cursor)?;

    assert_eq!(word_bank.len(), 2);
    assert_rc_eq!(&word_bank, &["worda", "wordb"]);
    assert_eq!(word_bank.word_length(), 5);
    Ok(())
}

#[test]
fn word_bank_from_iterator_succeeds() -> Result<(), WordleError> {
    let word_bank = WordBank::from_iterator(vec!["", "worda", "Wordb "])?;

    assert_eq!(word_bank.len(), 2);
    assert_rc_eq!(&word_bank, &["worda", "wordb"]);
    assert_eq!(word_bank.word_length(), 5);
    Ok(())
}

#[test]
fn word_bank_from_string_iterator_succeeds() -> Result<(), WordleError> {
    let word_bank = WordBank::from_iterator(vec![
        "".to_string(),
        "worda".to_string(),
        "Wordb ".to_string(),
    ])?;

    assert_eq!(word_bank.len(), 2);
    assert_rc_eq!(&word_bank, &["worda", "wordb"]);
    assert_eq!(word_bank.word_length(), 5);
    Ok(())
}

#[test]
fn word_bank_from_reader_mismatched_word_length_fails() {
    let mut cursor = Cursor::new(String::from("\nlongword\nshort\n"));

    assert_matches!(
        WordBank::from_reader(&mut cursor),
        Err(WordleError::WordLength(8))
    );
}

#[test]
fn compressed_guess_result_equality() -> Result<(), WordleError> {
    let result_correct = CompressedGuessResult::from_results(&[LetterResult::Correct; 4])?;
    let result_not_here = CompressedGuessResult::from_results(&[LetterResult::PresentNotHere; 4])?;
    let result_not_present = CompressedGuessResult::from_results(&[LetterResult::NotPresent; 4])?;

    assert_eq!(result_correct, result_correct);
    assert_eq!(result_not_here, result_not_here);
    assert_eq!(result_not_present, result_not_present);
    assert!(result_correct != result_not_here);
    assert!(result_correct != result_not_present);
    assert!(result_not_here != result_not_present);
    Ok(())
}

#[test]
fn guess_results_computes_for_all_words() {
    let all_words = vec![Rc::from("hello"), Rc::from("hallo"), Rc::from("worda")];
    let results = PrecomputedGuessResults::compute(&all_words).unwrap();

    assert_eq!(
        results.get_result(&all_words[0], &all_words[0]),
        Some(CompressedGuessResult::from_results(&[LetterResult::Correct; 5]).unwrap())
    );
    assert_eq!(
        results.get_result(&all_words[0], &all_words[1]),
        Some(
            CompressedGuessResult::from_results(&[
                LetterResult::Correct,
                LetterResult::NotPresent,
                LetterResult::Correct,
                LetterResult::Correct,
                LetterResult::Correct,
            ])
            .unwrap()
        )
    );
    assert_eq!(
        results.get_result(&all_words[0], &all_words[2]),
        Some(
            CompressedGuessResult::from_results(&[
                LetterResult::NotPresent,
                LetterResult::PresentNotHere,
                LetterResult::NotPresent,
                LetterResult::NotPresent,
                LetterResult::NotPresent,
            ])
            .unwrap()
        )
    );
    assert_eq!(
        results.get_result(&all_words[1], &all_words[1]),
        Some(CompressedGuessResult::from_results(&[LetterResult::Correct; 5]).unwrap())
    );
    assert_eq!(
        results.get_result(&all_words[1], &all_words[0]),
        Some(
            CompressedGuessResult::from_results(&[
                LetterResult::Correct,
                LetterResult::NotPresent,
                LetterResult::Correct,
                LetterResult::Correct,
                LetterResult::Correct,
            ])
            .unwrap()
        )
    );
    assert_eq!(
        results.get_result(&all_words[1], &all_words[2]),
        Some(
            CompressedGuessResult::from_results(&[
                LetterResult::NotPresent,
                LetterResult::PresentNotHere,
                LetterResult::NotPresent,
                LetterResult::NotPresent,
                LetterResult::PresentNotHere,
            ])
            .unwrap()
        )
    );
    assert_eq!(
        results.get_result(&all_words[2], &all_words[2]),
        Some(CompressedGuessResult::from_results(&[LetterResult::Correct; 5]).unwrap())
    );
    assert_eq!(
        results.get_result(&all_words[2], &all_words[0]),
        Some(
            CompressedGuessResult::from_results(&[
                LetterResult::NotPresent,
                LetterResult::NotPresent,
                LetterResult::NotPresent,
                LetterResult::NotPresent,
                LetterResult::PresentNotHere,
            ])
            .unwrap()
        )
    );
    assert_eq!(
        results.get_result(&all_words[2], &all_words[1]),
        Some(
            CompressedGuessResult::from_results(&[
                LetterResult::NotPresent,
                LetterResult::PresentNotHere,
                LetterResult::NotPresent,
                LetterResult::NotPresent,
                LetterResult::PresentNotHere,
            ])
            .unwrap()
        )
    );
}
