#[macro_use]
extern crate assert_matches;

use rs_wordle_solver::details::*;
use rs_wordle_solver::scorers::*;
use rs_wordle_solver::*;

use std::rc::Rc;
use std::result::Result;

macro_rules! test_scorer {
    ($construct_scorer_from_bank_fn:ident) => {
        #[test]
        fn solve_wordle() -> Result<(), WordleError> {
            let bank = WordBank::from_iterator(vec![
                "alpha", "allot", "begot", "below", "endow", "ingot",
            ])?;
            let scorer = $construct_scorer_from_bank_fn(&bank);
            let guesser = MaxScoreGuesser::new(GuessFrom::AllUnguessedWords, &bank, scorer);

            let result = play_game_with_guesser("alpha", bank.len() as u32, guesser);

            assert_matches!(result, GameResult::Success(_guesses));
            Ok(())
        }

        #[test]
        fn try_solve_unknown_word() -> Result<(), WordleError> {
            let bank = WordBank::from_iterator(vec![
                "alpha", "allot", "begot", "below", "endow", "ingot",
            ])?;
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
        MaxUniqueLetterFrequencyScorer::new(bank)
    }

    test_scorer!(create_scorer);
}

mod score_located_letters {

    use super::*;

    fn create_scorer(bank: &WordBank) -> LocatedLettersScorer {
        LocatedLettersScorer::new(bank)
    }

    test_scorer!(create_scorer);

    #[test]
    fn score_word() -> Result<(), WordleError> {
        let bank =
            WordBank::from_iterator(vec!["alpha", "allot", "begot", "below", "endow", "ingot"])?;
        let mut scorer = LocatedLettersScorer::new(&bank);

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
        let bank =
            WordBank::from_iterator(vec!["alpha", "allot", "begot", "below", "endow", "ingot"])?;
        let mut scorer = LocatedLettersScorer::new(&bank);

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
        let bank =
            WordBank::from_iterator(vec!["alpha", "allot", "begot", "below", "endow", "ingot"])?;
        let mut scorer = LocatedLettersScorer::new(&bank);

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
        MaxApproximateEliminationsScorer::new(bank)
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
