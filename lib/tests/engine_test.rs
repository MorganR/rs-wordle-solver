use std::rc::Rc;
use wordle_solver::*;

#[test]
fn calculate_best_guess_no_words() {
    let bank = create_word_bank(vec![]);
    let all_words = bank.all_words();
    let scorer = MaxUniqueLetterFrequencyScorer::new(&all_words);
    let mut guesser = MaxScoreGuesser::new(GuessFrom::PossibleWords, all_words, scorer);

    assert_eq!(guesser.select_next_guess(), None);
}

#[test]
fn calculate_best_guess_chooses_best_word() {
    let bank = create_word_bank(vec!["abcz", "wxyz", "defy", "ghix"]);
    let all_words = bank.all_words();
    let scorer = MaxUniqueLetterFrequencyScorer::new(&all_words);
    let mut guesser = MaxScoreGuesser::new(GuessFrom::PossibleWords, all_words, scorer);

    assert_eq!(guesser.select_next_guess(), Some(Rc::from("wxyz")));
}

#[test]
fn update_guess_result_modifies_next_guess() {
    let bank = create_word_bank(vec!["abcz", "weyz", "defy", "ghix"]);
    let all_words = bank.all_words();
    let scorer = MaxUniqueLetterFrequencyScorer::new(&all_words);
    let mut guesser = MaxScoreGuesser::new(GuessFrom::PossibleWords, all_words, scorer);

    guesser.update(&GuessResult {
        guess: "weyz",
        results: vec![
            LetterResult::NotPresent,
            LetterResult::Correct,
            LetterResult::PresentNotHere,
            LetterResult::NotPresent,
        ],
    });

    assert_eq!(guesser.select_next_guess(), Some(Rc::from("defy")));
}

#[test]
fn play_game_with_unknown_word() {
    let bank = create_word_bank(vec!["abcz", "weyz", "defy", "ghix"]);

    assert_eq!(play_game("nope", 10, &bank), GameResult::UnknownWord);
}

#[test]
fn play_game_with_known_word() {
    let bank = create_word_bank(vec!["abcz", "weyz", "defy", "ghix"]);

    if let GameResult::Success(guesses) = play_game("abcz", 10, &bank) {
        assert!(guesses.len() < 10);
        assert_eq!(guesses.iter().last(), Some(&Box::from("abcz")));
    } else {
        assert!(false);
    }
}

#[test]
fn play_game_with_scorer_with_unknown_word() {
    let bank = create_word_bank(vec!["abcz", "weyz", "defy", "ghix"]);
    let tracker = WordTracker::new(&bank.all_words());
    let precomputed_possibilities =
        MaxExpectedEliminationsScorer::precompute_possibilities(tracker.clone());
    let scorer =
        MaxExpectedEliminationsScorer::from_precomputed(tracker, precomputed_possibilities);

    assert_eq!(
        play_game_with_scorer("nope", 10, &bank, scorer),
        GameResult::UnknownWord
    );
}

#[test]
fn play_game_with_scorer_with_known_word() {
    let bank = create_word_bank(vec!["abcz", "weyz", "defy", "ghix"]);
    let tracker = WordTracker::new(&bank.all_words());
    let precomputed_possibilities =
        MaxExpectedEliminationsScorer::precompute_possibilities(tracker.clone());
    let scorer =
        MaxExpectedEliminationsScorer::from_precomputed(tracker, precomputed_possibilities);

    if let GameResult::Success(guesses) = play_game_with_scorer("abcz", 10, &bank, scorer) {
        assert!(guesses.len() < 10);
        assert_eq!(guesses.iter().last(), Some(&Box::from("abcz")));
    } else {
        assert!(false);
    }
}

#[test]
fn play_game_takes_too_many_guesses() {
    let bank = create_word_bank(vec!["abcz", "weyz", "defy", "ghix"]);

    if let GameResult::Failure(guesses) = play_game("abcz", 1, &bank) {
        assert_eq!(guesses.len(), 1);
        assert!(!guesses.contains(&Box::from("abcz")));
    } else {
        assert!(false);
    }
}

#[test]
fn get_result_for_guess_success() {
    let result = get_result_for_guess("piano", "amino");

    assert_eq!(
        result,
        GuessResult {
            guess: "amino",
            results: vec![
                LetterResult::PresentNotHere,
                LetterResult::NotPresent,
                LetterResult::PresentNotHere,
                LetterResult::Correct,
                LetterResult::Correct,
            ]
        }
    )
}

fn create_word_bank(words: Vec<&str>) -> WordBank {
    WordBank::from_vec(words.iter().map(|word| word.to_string()).collect())
}
