/// The result of a given letter at a specific location. There is some complexity here when a
/// letter appears in a word more than once. See [`GuessResult`] for more details.
#[derive(Debug, Eq, PartialEq, Clone, Copy, Hash)]
pub enum LetterResult {
    /// This letter goes exactly here in the objective word.
    Correct,
    /// This letter is in the objective word, but not here.
    PresentNotHere,
    /// This letter is not in the objective word, or is only in the word as many times as it was
    /// marked either `PresentNotHere` or `Correct`.
    NotPresent,
}

/// Indicates that an error occurred while trying to guess the objective word.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum WordleError {
    /// Indicates that the objective word must not be in the word bank.
    NotFound,
    /// Indicates that the given `GuessResult`s are impossible due to some inconsistency.
    InvalidResults,
    /// Indicates that a word had the wrong length (all words in a Wordle game must have the same
    /// length).
    WordWrongLength,
    /// Indicates that some kind of internal error has occurred.
    Internal,
}

/// The result of a single word guess.
///
/// There is some complexity here when the guess has duplicate letters. Duplicate letters are
/// matched to [`LetterResult`]s as follows:
///
/// 1. All letters in the correct location are marked `Correct`.
/// 2. For any remaining letters, if the objective word has more letters than were marked correct,
///    then these letters are marked as `PresentNotHere` starting from the beginning of the word,
///    until all letters have been accounted for.
/// 3. Any remaining letters are marked as `NotPresent`.
///
/// For example, if the guess was "sassy" for the objective word "mesas", then the results would
/// be: `[PresentNotHere, PresentNotHere, Correct, NotPresent, NotPresent]`.
#[derive(Debug, PartialEq)]
pub struct GuessResult<'a> {
    /// The guess that was made.
    pub guess: &'a str,
    /// The result of each letter, provided in the same leter order as in the guess.
    pub results: Vec<LetterResult>,
}

/// Whether the game was won or lost by the guesser.
#[derive(Debug, Eq, PartialEq)]
pub enum GameResult {
    /// Indicates that the guesser won the game, and provides the guesses that were given.
    Success(Vec<Box<str>>),
    /// Indicates that the guesser failed to guess the word, and provides the guesses that were given.
    Failure(Vec<Box<str>>),
    /// Indicates that the given word was not in the guesser's word bank.
    UnknownWord,
}

/// Determines the result of the given `guess` when applied to the given `objective`.
pub fn get_result_for_guess<'a>(objective: &str, guess: &'a str) -> GuessResult<'a> {
    if objective.len() != guess.len() {
        panic!(
            "Objective ({}) must have the same length as the guess ({})",
            objective, guess
        );
    }
    let mut results = Vec::with_capacity(guess.len());
    results.resize(guess.len(), LetterResult::NotPresent);
    for (objective_index, objective_letter) in objective.char_indices() {
        let mut set_index = None;
        for (guess_index, guess_letter) in guess.char_indices() {
            // Break if we're done and there is no chance of being correct.
            if set_index.is_some() && guess_index > objective_index {
                break;
            }
            // Continue if this letter doesn't match.
            if guess_letter != objective_letter {
                continue;
            }
            let existing_result = results[guess_index];
            // This letter is correct.
            if guess_index == objective_index {
                results[guess_index] = LetterResult::Correct;
                if set_index.is_some() {
                    // Undo the previous set.
                    results[set_index.unwrap()] = LetterResult::NotPresent;
                    set_index = None;
                }
                // This result was previously unset and we're done with this letter.
                if existing_result == LetterResult::NotPresent {
                    break;
                }
                // This result was previously set to "LetterResult::PresentNotHere", so we need to
                // forward that to the next matching letter.
                continue;
            }
            // This result is already set to something, so skip.
            if existing_result != LetterResult::NotPresent || set_index.is_some() {
                continue;
            }
            // This result was previously unset and matches this letter.
            results[guess_index] = LetterResult::PresentNotHere;
            set_index = Some(guess_index);
        }
    }
    GuessResult { guess, results }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_result_for_guess_correct() {
        assert_eq!(
            get_result_for_guess("abcb", "abcb"),
            GuessResult {
                guess: "abcb",
                results: vec![LetterResult::Correct; 4]
            }
        );
    }

    #[test]
    fn get_result_for_guess_partial() {
        assert_eq!(
            get_result_for_guess("mesas", "sassy"),
            GuessResult {
                guess: "sassy",
                results: vec![
                    LetterResult::PresentNotHere,
                    LetterResult::PresentNotHere,
                    LetterResult::Correct,
                    LetterResult::NotPresent,
                    LetterResult::NotPresent
                ]
            }
        );
    }

    #[test]
    fn get_result_for_guess_none() {
        assert_eq!(
            get_result_for_guess("abcb", "defg"),
            GuessResult {
                guess: "defg",
                results: vec![LetterResult::NotPresent; 4],
            }
        );
    }
}

#[cfg(all(feature = "unstable", test))]
mod benches {
    extern crate test;

    use super::*;
    use test::Bencher;

    #[bench]
    fn bench_get_result_for_guess_correct(b: &mut Bencher) {
        b.iter(|| get_result_for_guess("abcbd", "abcbd"))
    }

    #[bench]
    fn bench_get_result_for_guess_partial(b: &mut Bencher) {
        b.iter(|| get_result_for_guess("mesas", "sassy"))
    }
}
