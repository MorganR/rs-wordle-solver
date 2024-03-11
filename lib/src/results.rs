use std::error::Error;
use std::fmt;

/// A compressed form of [LetterResult]s. Can only store vectors of up to 10 results.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct CompressedGuessResult {
    data: u32,
}

const NUM_BITS_PER_LETTER_RESULT: usize = 2;
pub const MAX_LETTERS_IN_COMPRESSED_GUESS_RESULT: usize =
    std::mem::size_of::<u32>() * 8 / NUM_BITS_PER_LETTER_RESULT;

impl CompressedGuessResult {
    /// Creates a compressed form of the given letter results.
    ///
    /// Returns a [`WordleError::WordLength`] error if `letter_results` has more than
    /// [`MAX_LETTERS_IN_COMPRESSED_GUESS_RESULT`] values.
    pub fn from_results(
        letter_results: &[LetterResult],
    ) -> std::result::Result<CompressedGuessResult, WordleError> {
        if letter_results.len() > MAX_LETTERS_IN_COMPRESSED_GUESS_RESULT {
            return Err(WordleError::WordLength(
                MAX_LETTERS_IN_COMPRESSED_GUESS_RESULT,
            ));
        }
        let mut data = 0;
        let mut index = 0;
        for letter in letter_results {
            data |= (*letter as u32) << index;
            index += NUM_BITS_PER_LETTER_RESULT;
        }
        Ok(Self { data })
    }
}

/// The result of a given letter at a specific location. There is some complexity here when a
/// letter appears in a word more than once. See [`GuessResult`] for more details.
#[derive(Debug, Eq, PartialEq, Clone, Copy, Hash)]
pub enum LetterResult {
    /// This letter goes exactly here in the objective word.
    Correct = 0b01,
    /// This letter is in the objective word, but not here.
    PresentNotHere = 0b10,
    /// This letter is not in the objective word, or is only in the word as many times as it was
    /// marked either `PresentNotHere` or `Correct`.
    NotPresent = 0b11,
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
            WordleError::WordLength(expected_length) => write!(f, "{:?}: all words and guesses in a Wordle game must have the same length, and must be less than or equal to the max word length: {}", self, expected_length),
            WordleError::InvalidResults => write!(f, "{:?}: provided GuessResults led to an impossible set of WordRestrictions", self),
            WordleError::IoError(io_err) => write!(f, "{:?}: {}", self, io_err),
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
#[derive(Clone, Debug, PartialEq, Hash)]
pub struct GuessResult<'a> {
    /// The guess that was made.
    pub guess: &'a str,
    /// The result of each letter, provided in the same leter order as in the guess.
    pub results: Vec<LetterResult>,
}

/// Data about a single turn of a Wordle game.
#[derive(Clone, Debug, PartialEq, Hash)]
pub struct TurnData {
    /// The guess that was made this turn.
    pub guess: Box<str>,
    /// The number of possible words that remained at the start of this turn.
    pub num_possible_words_before_guess: usize,
}

/// The data from a game that was played.
#[derive(Clone, Debug, PartialEq)]
pub struct GameData {
    /// Data for each turn that was played.
    pub turns: Vec<TurnData>,
}

/// Whether the game was won or lost by the guesser.
#[derive(Clone, Debug, PartialEq)]
pub enum GameResult {
    /// Indicates that the guesser won the game, and provides the guesses that were given.
    Success(GameData),
    /// Indicates that the guesser failed to guess the word under the guess limit, and provides the
    /// guesses that were given.
    Failure(GameData),
    /// Indicates that the given word was not in the guesser's word bank.
    UnknownWord,
}

/// Determines the result of the given `guess` when applied to the given `objective`.
///
/// ```
/// use rs_wordle_solver::get_result_for_guess;
/// use rs_wordle_solver::GuessResult;
/// use rs_wordle_solver::LetterResult;
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
