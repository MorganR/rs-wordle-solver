#[macro_use]
extern crate assert_matches;

use rs_wordle_solver::details::*;
use rs_wordle_solver::*;

use std::rc::Rc;
use std::result::Result;

#[test]
fn random_guesser_select_next_guess_no_words() -> Result<(), WordleError> {
    let bank = create_word_bank(vec![])?;
    let mut guesser = RandomGuesser::new(&bank);

    assert_eq!(guesser.select_next_guess(), None);
    Ok(())
}

#[test]
fn random_guesser_select_next_guess_chooses_best_word() -> Result<(), WordleError> {
    let bank = create_word_bank(vec!["abc", "def", "ghi"])?;
    let mut guesser = RandomGuesser::new(&bank);

    let guess = guesser.select_next_guess();
    assert!(guess.is_some());
    assert!(&bank.contains(&guess.unwrap()));
    Ok(())
}

#[test]
fn random_guesser_update_guess_result_modifies_next_guess() -> Result<(), WordleError> {
    let bank = create_word_bank(vec!["abc", "bcd", "cde"])?;
    let mut guesser = RandomGuesser::new(&bank);

    guesser.update(&GuessResult {
        guess: "bcd",
        results: vec![
            LetterResult::PresentNotHere,
            LetterResult::PresentNotHere,
            LetterResult::NotPresent,
        ],
    })?;

    assert_eq!(guesser.select_next_guess(), Some(Rc::from("abc")));
    Ok(())
}

#[test]
fn random_guesser_invalid_update_fails() -> Result<(), WordleError> {
    let bank = create_word_bank(vec!["abc", "bcd", "cde"])?;
    let mut guesser = RandomGuesser::new(&bank);

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
    let bank = create_word_bank(vec![])?;
    let scorer = MaxUniqueLetterFrequencyScorer::new(WordCounter::new(&bank));
    let mut guesser = MaxScoreGuesser::new(GuessFrom::PossibleWords, &bank, scorer);

    assert_eq!(guesser.select_next_guess(), None);
    Ok(())
}

#[test]
fn max_score_guesser_select_next_guess_chooses_best_word() -> Result<(), WordleError> {
    let bank = create_word_bank(vec!["abcz", "wxyz", "defy", "ghix"])?;
    let scorer = MaxUniqueLetterFrequencyScorer::new(WordCounter::new(&bank));
    let mut guesser = MaxScoreGuesser::new(GuessFrom::PossibleWords, &bank, scorer);

    assert_eq!(guesser.select_next_guess(), Some(Rc::from("wxyz")));
    Ok(())
}

#[test]
fn max_score_guesser_update_guess_result_modifies_next_guess() -> Result<(), WordleError> {
    let bank = create_word_bank(vec!["abcz", "weyz", "defy", "ghix"])?;
    let scorer = MaxUniqueLetterFrequencyScorer::new(WordCounter::new(&bank));
    let mut guesser = MaxScoreGuesser::new(GuessFrom::PossibleWords, &bank, scorer);

    guesser.update(&GuessResult {
        guess: "weyz",
        results: vec![
            LetterResult::NotPresent,
            LetterResult::Correct,
            LetterResult::PresentNotHere,
            LetterResult::NotPresent,
        ],
    })?;

    assert_eq!(guesser.select_next_guess(), Some(Rc::from("defy")));
    Ok(())
}

#[test]
fn play_game_with_unknown_word_random() -> Result<(), WordleError> {
    let bank = create_word_bank(vec!["abcz", "weyz", "defy", "ghix"])?;
    let guesser = RandomGuesser::new(&bank);

    assert_eq!(
        play_game_with_guesser("nope", 10, guesser),
        GameResult::UnknownWord
    );
    Ok(())
}

#[test]
fn play_game_with_known_word_random() -> Result<(), WordleError> {
    let bank = create_word_bank(vec!["abcz", "weyz", "defy", "ghix"])?;
    let guesser = RandomGuesser::new(&bank);

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
    let bank = create_word_bank(vec!["abcz", "weyz", "defy", "ghix"])?;
    let scorer = MaxEliminationsScorer::new(&bank).unwrap();
    let guesser = MaxScoreGuesser::new(GuessFrom::PossibleWords, &bank, scorer);

    assert_eq!(
        play_game_with_guesser("nope", 10, guesser),
        GameResult::UnknownWord
    );
    Ok(())
}

#[test]
fn play_game_with_known_word_max_eliminations() -> Result<(), WordleError> {
    let bank = create_word_bank(vec!["abcz", "weyz", "defy", "ghix"])?;
    let scorer = MaxEliminationsScorer::new(&bank).unwrap();
    let guesser = MaxScoreGuesser::new(GuessFrom::PossibleWords, &bank, scorer);

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
    let bank = create_word_bank(vec!["abcz", "weyz", "defy", "ghix"])?;
    let scorer = MaxUniqueLetterFrequencyScorer::new(WordCounter::new(&bank));
    let guesser = MaxScoreGuesser::new(GuessFrom::PossibleWords, &bank, scorer);

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

macro_rules! test_scorer {
    ($construct_scorer_from_bank_fn:ident) => {
        #[test]
        fn solve_wordle() -> Result<(), WordleError> {
            let bank =
                create_word_bank(vec!["alpha", "allot", "begot", "below", "endow", "ingot"])?;
            let scorer = $construct_scorer_from_bank_fn(&bank);
            let guesser = MaxScoreGuesser::new(GuessFrom::AllUnguessedWords, &bank, scorer);

            let result = play_game_with_guesser("alpha", bank.len() as u32, guesser);

            assert_matches!(result, GameResult::Success(_guesses));
            Ok(())
        }

        #[test]
        fn try_solve_unknown_word() -> Result<(), WordleError> {
            let bank =
                create_word_bank(vec!["alpha", "allot", "begot", "below", "endow", "ingot"])?;
            let scorer = $construct_scorer_from_bank_fn(&bank);
            let guesser = MaxScoreGuesser::new(GuessFrom::AllUnguessedWords, &bank, scorer);

            let result = play_game_with_guesser("other", bank.len() as u32, guesser);

            assert_matches!(result, GameResult::UnknownWord);
            Ok(())
        }
    };
}

mod max_unique_letters_scorer {

    use super::*;

    fn create_scorer(bank: &WordBank) -> MaxUniqueLetterFrequencyScorer {
        MaxUniqueLetterFrequencyScorer::new(WordCounter::new(bank))
    }

    test_scorer!(create_scorer);
}

mod score_located_letters {

    use super::*;

    fn create_scorer(bank: &WordBank) -> LocatedLettersScorer {
        LocatedLettersScorer::new(bank, WordCounter::new(bank))
    }

    test_scorer!(create_scorer);

    #[test]
    fn score_word() -> Result<(), WordleError> {
        let bank = create_word_bank(vec!["alpha", "allot", "begot", "below", "endow", "ingot"])?;
        let mut scorer = LocatedLettersScorer::new(&bank, WordCounter::new(bank.as_ref()));

        assert_eq!(scorer.score_word(&Rc::from("alpha")), 4 + 5 + 2 + 2 + 1);
        assert_eq!(scorer.score_word(&Rc::from("allot")), 4 + 5 + 2 + 10 + 6);
        assert_eq!(scorer.score_word(&Rc::from("begot")), 4 + 5 + 4 + 10 + 6);
        assert_eq!(scorer.score_word(&Rc::from("below")), 4 + 5 + 5 + 10 + 4);
        assert_eq!(scorer.score_word(&Rc::from("endow")), 4 + 4 + 2 + 10 + 4);
        assert_eq!(scorer.score_word(&Rc::from("ingot")), 2 + 4 + 4 + 10 + 6);
        assert_eq!(scorer.score_word(&Rc::from("other")), 5 + 3 + 1 + 3 + 0);
        Ok(())
    }

    #[test]
    fn score_word_after_update() -> Result<(), WordleError> {
        let bank = create_word_bank(vec!["alpha", "allot", "begot", "below", "endow", "ingot"])?;
        let mut scorer = LocatedLettersScorer::new(&bank, WordCounter::new(bank.as_ref()));

        let restrictions = WordRestrictions::from_result(&GuessResult {
            guess: "begot",
            results: vec![
                LetterResult::NotPresent,
                LetterResult::PresentNotHere,
                LetterResult::NotPresent,
                LetterResult::Correct,
                LetterResult::NotPresent,
            ],
        });
        scorer.update("begot", &restrictions, &vec![Rc::from("endow")])?;
        // Remaining possible words: 'endow'

        assert_eq!(scorer.score_word(&Rc::from("alpha")), 0 + 0 + 0 + 0 + 0);
        assert_eq!(scorer.score_word(&Rc::from("below")), 0 + 0 + 0 + 1 + 2);
        assert_eq!(scorer.score_word(&Rc::from("endow")), 1 + 2 + 2 + 1 + 2);
        assert_eq!(scorer.score_word(&Rc::from("other")), 0 + 0 + 0 + 0 + 0);
        Ok(())
    }

    #[test]
    fn update_with_uknown_word() -> Result<(), WordleError> {
        let bank = create_word_bank(vec!["alpha", "allot", "begot", "below", "endow", "ingot"])?;
        let mut scorer = LocatedLettersScorer::new(&bank, WordCounter::new(&bank));

        let restrictions = WordRestrictions::from_result(&GuessResult {
            guess: "other",
            results: vec![
                LetterResult::PresentNotHere,
                LetterResult::NotPresent,
                LetterResult::NotPresent,
                LetterResult::PresentNotHere,
                LetterResult::NotPresent,
            ],
        });
        scorer.update(
            "other",
            &restrictions,
            &vec![Rc::from("below"), Rc::from("endow")],
        )?;
        // Remaining possible words: 'below', 'endow'

        assert_eq!(scorer.score_word(&Rc::from("alpha")), 0 + 1 + 0 + 0 + 0);
        assert_eq!(scorer.score_word(&Rc::from("below")), 2 + 1 + 2 + 2 + 4);
        assert_eq!(scorer.score_word(&Rc::from("endow")), 1 + 2 + 2 + 2 + 4);
        assert_eq!(scorer.score_word(&Rc::from("other")), 0 + 0 + 0 + 0 + 0);
        Ok(())
    }
}

mod max_approximate_eliminations_scorer {

    use super::*;

    fn create_scorer(bank: &WordBank) -> MaxApproximateEliminationsScorer {
        MaxApproximateEliminationsScorer::new(WordCounter::new(bank))
    }

    test_scorer!(create_scorer);
}

mod max_eliminations_scorer {

    use super::*;

    fn create_scorer(bank: &WordBank) -> MaxEliminationsScorer {
        MaxEliminationsScorer::new(bank).unwrap()
    }

    test_scorer!(create_scorer);

    #[test]
    fn score_word() {
        let possible_words: Vec<Rc<str>> = vec![Rc::from("cod"), Rc::from("wod"), Rc::from("mod")];
        let mut scorer = MaxEliminationsScorer::new(&possible_words).unwrap();

        assert_eq!(scorer.score_word(&possible_words[0]), 1333);
        assert_eq!(scorer.score_word(&Rc::from("mwc")), 2000);
        assert_eq!(scorer.score_word(&Rc::from("zzz")), 0);
    }

    #[test]
    fn score_word_after_update() -> Result<(), WordleError> {
        let possible_words: Vec<Rc<str>> = vec![
            Rc::from("abb"),
            Rc::from("abc"),
            Rc::from("bad"),
            Rc::from("zza"),
            Rc::from("zzz"),
        ];
        let mut scorer = MaxEliminationsScorer::new(&possible_words).unwrap();

        let restrictions = WordRestrictions::from_result(&GuessResult {
            guess: "zza",
            results: vec![
                LetterResult::NotPresent,
                LetterResult::NotPresent,
                LetterResult::PresentNotHere,
            ],
        });
        scorer.update("zza", &restrictions, &Vec::from(&possible_words[0..3]))?;
        // Still possible: abb, abc, bad

        // Eliminates 2 in all cases.
        assert_eq!(scorer.score_word(&possible_words[0]), 2000);
        // Eliminates 2 in all cases.
        assert_eq!(scorer.score_word(&possible_words[1]), 2000);
        // Could be true in one case (elimnate 2), or false in 2 cases (eliminate 1)
        assert_eq!(scorer.score_word(&possible_words[2]), 1333);
        assert_eq!(scorer.score_word(&Rc::from("zzz")), 0);
        Ok(())
    }
}

fn create_word_bank(words: Vec<&str>) -> Result<WordBank, WordleError> {
    WordBank::from_iterator(words)
}
