#[cfg(test)]
mod tests {

    use std::error::Error;
    use std::fs::File;
    use std::io;

    use ron;
    use rs_wordle_solver::scorers::{MaxEliminationsScorer, WordScorer};
    use rs_wordle_solver::*;

    #[test]
    fn max_eliminations_scorer_serde() {
        let word_bank = WordBank::from_iterator(vec!["worda", "wordb"]).unwrap();
        let scorer = MaxEliminationsScorer::new(word_bank.clone());
        let score = scorer.score_word(&(&word_bank)[0]);

        let ser = ron::to_string(&scorer);
        assert!(ser.is_ok());

        let deser = ron::from_str::<MaxEliminationsScorer>(&ser.unwrap());
        assert!(deser.is_ok());
        let deser_score = scorer.score_word(&word_bank[0]);
        assert_eq!(deser_score, score);
    }

    #[test]
    fn max_score_guesser_serde() -> Result<(), Box<dyn Error>> {
        let all_words = io::BufReader::new(File::open("../data/1000-improved-words-shuffled.txt")?);

        let bank = WordBank::from_reader(all_words)?;
        let scorer = MaxEliminationsScorer::new(bank.clone());
        let mut guesser = MaxScoreGuesser::new(GuessFrom::AllUnguessedWords, bank, scorer);
        // Assume the word is "groan".
        guesser.update(&GuessResult {
            guess: "align",
            results: vec![
                LetterResult::PresentNotHere,
                LetterResult::NotPresent,
                LetterResult::NotPresent,
                LetterResult::NotPresent,
                LetterResult::Correct,
            ],
        })?;
        let top_guesses = guesser.select_top_n_guesses(5);

        let ser = ron::to_string(&guesser);
        assert!(ser.is_ok());

        let deser = ron::from_str::<MaxScoreGuesser<MaxEliminationsScorer>>(&ser.unwrap());
        assert!(deser.is_ok());
        let deser_top_guesses = deser.unwrap().select_top_n_guesses(5);
        assert_eq!(deser_top_guesses, top_guesses);
        Ok(())
    }
}
