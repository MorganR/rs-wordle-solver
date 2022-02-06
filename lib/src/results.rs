/// The result of a given letter at a specific location.
#[derive(Debug, Eq, PartialEq)]
pub enum LetterResult {
    Correct,
    PresentNotHere,
    NotPresent,
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
