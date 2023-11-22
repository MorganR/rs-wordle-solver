#[macro_use]
extern crate assert_matches;

use rs_wordle_solver::details::*;
use rs_wordle_solver::*;

use std::io::Cursor;
use std::result::Result;
use std::sync::Arc;

macro_rules! assert_arc_eq {
    ($arc_vec:expr, $non_arc_vec:expr) => {
        assert_eq!(
            $arc_vec as &[Arc<str>],
            $non_arc_vec
                .iter()
                .map(|thing| Arc::from(*thing))
                .collect::<Vec<Arc<_>>>()
        );
    };
}

#[test]
fn word_bank_from_reader_succeeds() -> Result<(), WordleError> {
    let mut cursor = Cursor::new(String::from("\n\nworda\n wordb\n"));

    let word_bank = WordBank::from_reader(&mut cursor)?;

    assert_eq!(word_bank.len(), 2);
    assert_arc_eq!(&word_bank, &["worda", "wordb"]);
    assert_eq!(word_bank.word_length(), 5);
    Ok(())
}

#[test]
fn word_bank_from_iterator_succeeds() -> Result<(), WordleError> {
    let word_bank = WordBank::from_iterator(vec!["", "worda", "Wordb "])?;

    assert_eq!(word_bank.len(), 2);
    assert_arc_eq!(&word_bank, &["worda", "wordb"]);
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
    assert_arc_eq!(&word_bank, &["worda", "wordb"]);
    assert_eq!(word_bank.word_length(), 5);
    Ok(())
}

#[test]
fn word_bank_from_reader_mismatched_word_length_fails() {
    let mut cursor = Cursor::new(String::from("\nlongword\n   short\n"));

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
fn compressed_guess_result_too_long() {
    assert_matches!(
        CompressedGuessResult::from_results(
            &[LetterResult::Correct; MAX_LETTERS_IN_COMPRESSED_GUESS_RESULT]
        ),
        Ok(_)
    );
    assert_matches!(
        CompressedGuessResult::from_results(
            &[LetterResult::Correct; MAX_LETTERS_IN_COMPRESSED_GUESS_RESULT + 1]
        ),
        Err(WordleError::WordLength(
            MAX_LETTERS_IN_COMPRESSED_GUESS_RESULT
        ))
    );
}
