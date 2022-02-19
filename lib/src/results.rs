use std::collections::HashMap;
use std::collections::HashSet;

/// The result of a given letter at a specific location.
#[derive(Debug, Eq, PartialEq, Clone, Copy, Hash)]
pub enum LetterResult {
    Correct,
    PresentNotHere,
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
}

/// The result of a single word guess.
#[derive(Debug, PartialEq)]
pub struct GuessResult<'a> {
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
    /// Indicates that the given word was not in the word bank.
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
            .or_insert(HashSet::new())
            .insert(index);
    }
    let mut guess_letter_indices: HashMap<char, HashSet<usize>> = HashMap::new();
    for (index, letter) in guess.char_indices() {
        guess_letter_indices
            .entry(letter)
            .or_insert(HashSet::new())
            .insert(index);
    }
    GuessResult {
        guess: guess,
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
                        return LetterResult::PresentNotHere;
                    } else {
                        return LetterResult::NotPresent;
                    }
                } else {
                    return LetterResult::NotPresent;
                }
            })
            .collect(),
    }
}
