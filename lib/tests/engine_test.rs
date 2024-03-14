#[macro_use]
extern crate assert_matches;

use rs_wordle_solver::scorers::*;
use rs_wordle_solver::*;

use std::result::Result;
use std::sync::Arc;

#[test]
fn random_guesser_select_next_guess_no_words() -> Result<(), WordleError> {
    let bank = WordBank::from_iterator(&Vec::<Arc<str>>::new())?;
    let mut guesser = RandomGuesser::new(bank);

    assert_eq!(guesser.select_next_guess(), None);
    Ok(())
}

#[test]
fn random_guesser_select_next_guess_chooses_best_word() -> Result<(), WordleError> {
    let bank = WordBank::from_iterator(vec!["abc", "def", "ghi"])?;
    let mut guesser = RandomGuesser::new(bank.clone());

    let guess = guesser.select_next_guess();
    assert!(guess.is_some());
    assert!(&bank.contains(&guess.unwrap()));
    Ok(())
}

#[test]
fn random_guesser_update_guess_result_modifies_next_guess() -> Result<(), WordleError> {
    let bank = WordBank::from_iterator(vec!["abc", "bcd", "cde"])?;
    let mut guesser = RandomGuesser::new(bank);

    guesser.update(&GuessResult {
        guess: "bcd",
        results: vec![
            LetterResult::PresentNotHere,
            LetterResult::PresentNotHere,
            LetterResult::NotPresent,
        ],
    })?;

    assert_eq!(guesser.select_next_guess(), Some(Arc::from("abc")));
    Ok(())
}

#[test]
fn random_guesser_invalid_update_fails() -> Result<(), WordleError> {
    let bank = WordBank::from_iterator(vec!["abc", "bcd", "cde"])?;
    let mut guesser = RandomGuesser::new(bank);

    guesser.update(&GuessResult {
        guess: "abc",
        results: vec![
            LetterResult::NotPresent,
            LetterResult::NotPresent,
            LetterResult::PresentNotHere,
        ],
    })?;
    guesser.update(&GuessResult {
        guess: "bcd",
        results: vec![
            LetterResult::NotPresent,
            LetterResult::PresentNotHere,
            LetterResult::NotPresent,
        ],
    })?;

    // This makes the 'c' required but impossible.
    assert_matches!(
        guesser.update(&GuessResult {
            guess: "cde",
            results: vec![
                LetterResult::PresentNotHere,
                LetterResult::NotPresent,
                LetterResult::NotPresent,
            ],
        }),
        Err(WordleError::InvalidResults)
    );
    Ok(())
}

#[test]
fn max_score_guesser_select_next_guess_no_words() -> Result<(), WordleError> {
    let bank = WordBank::from_iterator(&Vec::<Arc<str>>::new())?;
    let scorer = MaxUniqueLetterFrequencyScorer::new(&bank);
    let mut guesser = MaxScoreGuesser::new(GuessFrom::PossibleWords, bank, scorer);

    assert_eq!(guesser.select_next_guess(), None);
    Ok(())
}

#[test]
fn max_score_guesser_select_next_guess_chooses_best_word() -> Result<(), WordleError> {
    let bank = WordBank::from_iterator(vec!["abcz", "wxyz", "defy", "ghix"])?;
    let scorer = MaxUniqueLetterFrequencyScorer::new(&bank);
    let mut guesser = MaxScoreGuesser::new(GuessFrom::PossibleWords, bank, scorer);

    assert_eq!(guesser.select_next_guess(), Some(Arc::from("wxyz")));
    Ok(())
}

#[test]
fn max_score_guesser_update_guess_result_modifies_next_guess() -> Result<(), WordleError> {
    let bank = WordBank::from_iterator(vec!["abcz", "weyz", "defy", "ghix"])?;
    let scorer = MaxUniqueLetterFrequencyScorer::new(&bank);
    let mut guesser = MaxScoreGuesser::new(GuessFrom::PossibleWords, bank, scorer);

    guesser.update(&GuessResult {
        guess: "weyz",
        results: vec![
            LetterResult::NotPresent,
            LetterResult::Correct,
            LetterResult::PresentNotHere,
            LetterResult::NotPresent,
        ],
    })?;

    assert_eq!(guesser.select_next_guess(), Some(Arc::from("defy")));
    Ok(())
}

#[test]
fn max_score_guesser_select_top_n_guesses_no_words() -> Result<(), WordleError> {
    let bank = WordBank::from_iterator(&Vec::<Arc<str>>::new())?;
    let scorer = MaxUniqueLetterFrequencyScorer::new(&bank);
    let mut guesser = MaxScoreGuesser::new(GuessFrom::PossibleWords, bank, scorer);

    assert_eq!(guesser.select_top_n_guesses(5), vec![]);
    Ok(())
}

#[test]
fn max_score_guesser_select_top_n_guesses() -> Result<(), WordleError> {
    let bank = WordBank::from_iterator(vec!["xxxx", "aaaa", "baac", "xabc"])?;
    let scorer = MaxUniqueLetterFrequencyScorer::new(&bank);
    let mut guesser = MaxScoreGuesser::new(GuessFrom::PossibleWords, bank, scorer);

    assert_eq!(
        guesser.select_top_n_guesses(3),
        vec![
            ScoredGuess {
                score: 9,
                guess: Arc::from("xabc")
            },
            ScoredGuess {
                score: 7,
                guess: Arc::from("baac")
            },
            ScoredGuess {
                score: 3,
                guess: Arc::from("aaaa")
            }
        ]
    );
    Ok(())
}

#[test]
fn max_score_guesser_invalid_update_fails() -> Result<(), WordleError> {
    let bank = WordBank::from_iterator(vec!["abc", "bcd", "cde"])?;
    let scorer = MaxUniqueLetterFrequencyScorer::new(&bank);
    let mut guesser = MaxScoreGuesser::new(GuessFrom::PossibleWords, bank, scorer);

    guesser.update(&GuessResult {
        guess: "abc",
        results: vec![
            LetterResult::NotPresent,
            LetterResult::NotPresent,
            LetterResult::NotPresent,
        ],
    })?;
    assert_matches!(
        guesser.update(&GuessResult {
            guess: "bcd",
            results: vec![
                LetterResult::NotPresent,
                // "c" can't be present, because we just said it's not present.
                LetterResult::PresentNotHere,
                LetterResult::PresentNotHere,
            ],
        }),
        Err(WordleError::InvalidResults)
    );
    Ok(())
}

#[test]
fn play_game_with_unknown_word_random() -> Result<(), WordleError> {
    let bank = WordBank::from_iterator(vec!["abcz", "weyz", "defy", "ghix"])?;
    let guesser = RandomGuesser::new(bank);

    assert_eq!(
        play_game_with_guesser("nope", 10, guesser),
        GameResult::UnknownWord
    );
    Ok(())
}

#[test]
fn play_game_with_known_word_random() -> Result<(), WordleError> {
    let bank = WordBank::from_iterator(vec!["abcz", "weyz", "defy", "ghix"])?;
    let guesser = RandomGuesser::new(bank);

    if let GameResult::Success(data) = play_game_with_guesser("abcz", 10, guesser) {
        assert!(data.turns.len() < 10);
        assert_eq!(
            data.turns.iter().map(|turn| &turn.guess).last(),
            Some(&Box::from("abcz"))
        );
    } else {
        assert!(false);
    }
    Ok(())
}

#[test]
fn play_game_with_unknown_word_max_eliminations() -> Result<(), WordleError> {
    let bank = WordBank::from_iterator(vec!["abcz", "weyz", "defy", "ghix"])?;
    let scorer = MaxEliminationsScorer::new(bank.clone()).unwrap();
    let guesser = MaxScoreGuesser::new(GuessFrom::PossibleWords, bank, scorer);

    assert_eq!(
        play_game_with_guesser("nope", 10, guesser),
        GameResult::UnknownWord
    );
    Ok(())
}

#[test]
fn play_game_with_known_word_max_eliminations() -> Result<(), WordleError> {
    let bank = WordBank::from_iterator(vec!["abcz", "weyz", "defy", "ghix"])?;
    let scorer = MaxEliminationsScorer::new(bank.clone()).unwrap();
    let guesser = MaxScoreGuesser::new(GuessFrom::PossibleWords, bank, scorer);

    if let GameResult::Success(data) = play_game_with_guesser("abcz", 10, guesser) {
        assert!(data.turns.len() < 10);
        assert_eq!(
            data.turns.iter().map(|turn| &turn.guess).last(),
            Some(&Box::from("abcz"))
        );
    } else {
        assert!(false);
    }
    Ok(())
}

#[test]
fn play_game_takes_too_many_guesses() -> Result<(), WordleError> {
    let bank = WordBank::from_iterator(vec!["abcz", "weyz", "defy", "ghix"])?;
    let scorer = MaxUniqueLetterFrequencyScorer::new(&bank);
    let guesser = MaxScoreGuesser::new(GuessFrom::PossibleWords, bank, scorer);

    if let GameResult::Failure(data) = play_game_with_guesser("abcz", 1, guesser) {
        assert_eq!(data.turns.len(), 1);
        assert!(!data
            .turns
            .iter()
            .map(|turn| &turn.guess)
            .any(|guess| guess == &Box::from("abcz")));
    } else {
        assert!(false);
    }
    Ok(())
}
