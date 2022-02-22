use std::error::Error;
use std::fmt;

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
#[derive(Debug)]
pub enum WordleError {
    /// Indicates that the word lengths differed, or words were too long for the chosen
    /// implementation. The expected word length or max possible word length is provided.
    WordLength(usize),
    /// Indicates that the given `GuessResult`s are impossible due to some inconsistency.
    InvalidResults,
    /// An IO error occurred.
    IoError(std::io::Error),
}

impl fmt::Display for WordleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WordleError::WordLength(expected_length) => write!(f, "{:?}: All words and guesses in a Wordle game must have the same length, and must be less than the max word length. Max/expected word length: {}", self, expected_length),
            WordleError::InvalidResults => write!(f, "{:?}: Provided GuessResults led to an impossible set of WordRestrictions.", self),
            WordleError::IoError(io_err) => write!(f, "{:?}: IO error: {}", self, io_err),
        }
    }
}

impl Error for WordleError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            WordleError::IoError(io_err) => io_err.source(),
            _ => None,
        }
    }
}

impl From<std::io::Error> for WordleError {
    fn from(io_err: std::io::Error) -> Self {
        WordleError::IoError(io_err)
    }
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
    /// Indicates that the guesser failed to guess the word under the guess limit, and provides the
    /// guesses that were given.
    Failure(Vec<Box<str>>),
    /// Indicates that the given word was not in the guesser's word bank.
    UnknownWord,
}

/// Determines the result of the given `guess` when applied to the given `objective`.
///
/// ```
/// use wordle_solver::get_result_for_guess;
/// use wordle_solver::GuessResult;
/// use wordle_solver::LetterResult;
///
/// let result = get_result_for_guess("mesas", "sassy");
/// assert!(
///     matches!(
///         result,
///         Ok(GuessResult {
///             guess: "sassy",
///             results: _
///         })
///     )
/// );
/// assert_eq!(
///     result.unwrap().results,
///     vec![
///         LetterResult::PresentNotHere,
///         LetterResult::PresentNotHere,
///         LetterResult::Correct,
///         LetterResult::NotPresent,
///         LetterResult::NotPresent
///     ]
/// );
/// ```
pub fn get_result_for_guess<'a>(
    objective: &str,
    guess: &'a str,
) -> Result<GuessResult<'a>, WordleError> {
    if objective.len() != guess.len() {
        return Err(WordleError::WordLength(objective.len()));
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
    Ok(GuessResult { guess, results })
}
