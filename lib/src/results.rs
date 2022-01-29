/// The result of a given letter at a specific location.
#[derive(Debug, Eq, PartialEq)]
pub enum LetterResult {
    Correct(char),
    PresentNotHere(char),
    NotPresent(char),
}

/// The result of a single word guess.
#[derive(Debug)]
pub struct GuessResult {
    /// The result of each letter, provided in the same leter order as in the guess.
    pub letters: Vec<LetterResult>,
}

/// Whether the game was won or lost by the guesser.
#[derive(Debug, Eq, PartialEq)]
pub enum GameResult {
    /// Indicates that the guesser won the game, and provides the guesses that were given.
    Success(Vec<String>),
    /// Indicates that the guesser failed to guess the word, and provides the guesses that were given.
    Failure(Vec<String>),
    /// Indicates that the given word was not in the word bank.
    UnknownWord,
}