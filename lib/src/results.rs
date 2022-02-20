use std::collections::HashMap;
use std::collections::HashSet;

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
    /// Indicates that one or more given characters are not in the supported set.
    UnsupportedCharacter,
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
    let mut objective_letter_indices: HashMap<char, HashSet<usize>> = HashMap::new();
    for (index, letter) in objective.char_indices() {
        objective_letter_indices
            .entry(letter)
            .or_insert_with(HashSet::new)
            .insert(index);
    }
    let mut guess_letter_indices: HashMap<char, HashSet<usize>> = HashMap::new();
    for (index, letter) in guess.char_indices() {
        guess_letter_indices
            .entry(letter)
            .or_insert_with(HashSet::new)
            .insert(index);
    }
    GuessResult {
        guess,
        results: guess
            .char_indices()
            .map(|(index, letter)| {
                if let Some(indices) = objective_letter_indices.get(&letter) {
                    if indices.contains(&index) {
                        return LetterResult::Correct;
                    }
                    let mut num_in_place = 0;
                    let mut num_ahead_not_in_place = 0;
                    let guess_indices = guess_letter_indices.get(&letter).unwrap();
                    for guess_index in guess_indices.iter() {
                        if indices.contains(guess_index) {
                            num_in_place += 1;
                        } else if *guess_index < index {
                            num_ahead_not_in_place += 1;
                        }
                    }
                    if indices.len() - num_in_place > num_ahead_not_in_place {
                        LetterResult::PresentNotHere
                    } else {
                        LetterResult::NotPresent
                    }
                } else {
                    LetterResult::NotPresent
                }
            })
            .collect(),
    }
}
