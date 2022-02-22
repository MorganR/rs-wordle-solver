#[macro_use]
extern crate assert_matches;

use wordle_solver::*;

#[test]
fn get_result_for_guess_correct() {
    let result = get_result_for_guess("abcb", "abcb");

    assert_matches!(
        get_result_for_guess("abcb", "abcb"),
        Ok(GuessResult {
            guess: "abcb",
            results: _,
        })
    );
    assert_eq!(result.unwrap().results, vec![LetterResult::Correct; 4]);
}

#[test]
fn get_result_for_guess_partial() {
    let result = get_result_for_guess("mesas", "sassy");
    assert_matches!(
        result,
        Ok(GuessResult {
            guess: "sassy",
            results: _
        })
    );
    assert_eq!(
        result.unwrap().results,
        vec![
            LetterResult::PresentNotHere,
            LetterResult::PresentNotHere,
            LetterResult::Correct,
            LetterResult::NotPresent,
            LetterResult::NotPresent
        ]
    );

    let result = get_result_for_guess("abba", "babb");
    assert_matches!(
        result,
        Ok(GuessResult {
            guess: "babb",
            results: _,
        })
    );
    assert_eq!(
        result.unwrap().results,
        vec![
            LetterResult::PresentNotHere,
            LetterResult::PresentNotHere,
            LetterResult::Correct,
            LetterResult::NotPresent
        ]
    );

    let result = get_result_for_guess("abcb", "bcce");
    assert_matches!(
        result,
        Ok(GuessResult {
            guess: "bcce",
            results: _,
        })
    );
    assert_eq!(
        result.unwrap().results,
        vec![
            LetterResult::PresentNotHere,
            LetterResult::NotPresent,
            LetterResult::Correct,
            LetterResult::NotPresent
        ]
    );
}

#[test]
fn get_result_for_guess_none_match() {
    let result = get_result_for_guess("abcb", "defg");
    assert_matches!(
        result,
        Ok(GuessResult {
            guess: "defg",
            results: _,
        })
    );
    assert_eq!(result.unwrap().results, vec![LetterResult::NotPresent; 4]);
}

#[test]
fn get_result_for_guess_invalid_guess() {
    assert_matches!(
        get_result_for_guess("goal", "guess"),
        Err(WordleError::WordLength(4))
    );
}
