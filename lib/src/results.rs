/// The result of a given letter at a specific location.
#[derive(Debug, Eq, PartialEq)]
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
