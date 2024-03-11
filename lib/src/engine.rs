use rayon::prelude::*;

use crate::data::*;
use crate::restrictions::WordRestrictions;
use crate::results::*;
use crate::scorers::WordScorer;
use std::result::Result;
use std::sync::Arc;

const MIN_PARALLELIZATION_LIMIT: usize = 1000;

/// Guesses words in order to solve a single Wordle.
pub trait Guesser {
    /// Updates this guesser with information about a word.
    fn update(&mut self, result: &GuessResult) -> Result<(), WordleError>;

    /// Selects a new guess for the Wordle.
    ///
    /// Returns `None` if no known words are possible given the known restrictions imposed by
    /// previous calls to [`Self::update()`].
    fn select_next_guess(&mut self) -> Option<Arc<str>>;

    /// Provides read access to the remaining set of possible words in this guesser.
    fn possible_words(&self) -> &[Arc<str>];
}

/// Attempts to guess the given word within the maximum number of guesses, using the given word
/// guesser.
///
/// ```
/// use rs_wordle_solver::GameResult;
/// use rs_wordle_solver::RandomGuesser;
/// use rs_wordle_solver::WordBank;
/// use rs_wordle_solver::play_game_with_guesser;
///
/// let bank = WordBank::from_iterator(&["abc", "def", "ghi"]).unwrap();
/// let mut guesser = RandomGuesser::new(&bank);
/// let result = play_game_with_guesser("def", 4, guesser.clone());
///
/// assert!(matches!(result, GameResult::Success(_guesses)));
///
/// let result = play_game_with_guesser("zzz", 4, guesser.clone());
///
/// assert!(matches!(result, GameResult::UnknownWord));
///
/// let result = play_game_with_guesser("other", 4, guesser);
///
/// assert!(matches!(result, GameResult::UnknownWord));
/// ```
pub fn play_game_with_guesser<G: Guesser>(
    word_to_guess: &str,
    max_num_guesses: u32,
    mut guesser: G,
) -> GameResult {
    let mut turns: Vec<TurnData> = Vec::new();
    for _ in 1..=max_num_guesses {
        let maybe_guess = guesser.select_next_guess();
        if maybe_guess.is_none() {
            return GameResult::UnknownWord;
        }
        let guess = maybe_guess.unwrap();
        let num_possible_words_before_guess = guesser.possible_words().len();
        let result = get_result_for_guess(word_to_guess, guess.as_ref());
        if result.is_err() {
            return GameResult::UnknownWord;
        }
        let result = result.unwrap();
        turns.push(TurnData {
            num_possible_words_before_guess,
            guess: Box::from(guess.as_ref()),
        });
        if result.results.iter().all(|lr| *lr == LetterResult::Correct) {
            return GameResult::Success(GameData { turns });
        }
        guesser.update(&result).unwrap();
    }
    GameResult::Failure(GameData { turns })
}

/// Guesses at random from the possible words that meet the restrictions.
///
/// A sample benchmark against the `data/improved-words.txt` list performed as follows:
///
/// |Num guesses to win|Num games|
/// |------------------|---------|
/// |1|1|
/// |2|106|
/// |3|816|
/// |4|1628|
/// |5|1248|
/// |6|518|
/// |7|180|
/// |8|67|
/// |9|28|
/// |10|7|
/// |11|2|
/// |12|1|
///
/// **Average number of guesses:** 4.49 +/- 1.26
#[derive(Debug, Clone)]
pub struct RandomGuesser {
    possible_words: Vec<Arc<str>>,
    restrictions: WordRestrictions,
}

impl RandomGuesser {
    /// Constructs a new `RandomGuesser` using the given word bank.
    ///
    /// ```
    /// use rs_wordle_solver::RandomGuesser;
    /// use rs_wordle_solver::WordBank;
    ///
    /// let bank = WordBank::from_iterator(&["abc", "def", "ghi"]).unwrap();
    /// let guesser = RandomGuesser::new(&bank);
    /// ```
    pub fn new(bank: &WordBank) -> RandomGuesser {
        RandomGuesser {
            possible_words: bank.to_vec(),
            restrictions: WordRestrictions::new(bank.word_length() as u8),
        }
    }
}

impl Guesser for RandomGuesser {
    fn update(&mut self, result: &GuessResult) -> Result<(), WordleError> {
        self.restrictions.update(result)?;
        self.possible_words
            .retain(|word| self.restrictions.is_satisfied_by(word));
        Ok(())
    }

    fn select_next_guess(&mut self) -> Option<Arc<str>> {
        if self.possible_words.is_empty() {
            return None;
        }
        let random: usize = rand::random();
        self.possible_words
            .get(random % self.possible_words.len())
            .map(Arc::clone)
    }

    fn possible_words(&self) -> &[Arc<str>] {
        &self.possible_words
    }
}

/// Indicates which set of words to guess from. See [`MaxScoreGuesser::new()`].
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum GuessFrom {
    /// Choose the next guess from any unguessed word in the whole word list.
    AllUnguessedWords,
    /// Choose the next guess from any possible word based on the current restrictions.
    PossibleWords,
}

/// Represents a guess with a 'score' estimating how useful the guess is. Higher scores are better.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScoredGuess {
    pub score: i64,
    pub guess: Arc<str>,
}

/// Selects the next guess that maximizes the score according to the owned scorer.
///
/// See [`WordScorer`] for more information about possible scoring algorithms.
#[derive(Debug, Clone)]
pub struct MaxScoreGuesser<T>
where
    T: WordScorer + Clone + Sync,
{
    guess_mode: GuessFrom,
    grouped_words: GroupedWords,
    restrictions: WordRestrictions,
    scorer: T,
}

impl<T> MaxScoreGuesser<T>
where
    T: WordScorer + Clone + Sync,
{
    /// Constructs a new `MaxScoreGuesser` that will guess the word with the maximum score
    /// according to the given [`WordScorer`]. This will only score and guess from the words in the
    /// `word_bank` according to the given `guess_from` strategy.
    ///
    /// If in doubt, you probably want to use `GuessFrom::AllUnguessedWords` for better performance.
    ///
    /// See [`WordScorer`] for more information about possible scoring algorithms.
    ///
    /// ```
    /// use std::sync::Arc;
    /// use rs_wordle_solver::GuessFrom;
    /// use rs_wordle_solver::Guesser;
    /// use rs_wordle_solver::MaxScoreGuesser;
    /// use rs_wordle_solver::WordBank;
    /// use rs_wordle_solver::scorers::MaxEliminationsScorer;
    ///
    /// let bank = WordBank::from_iterator(&["azz", "bzz", "czz", "abc"]).unwrap();
    /// let scorer = MaxEliminationsScorer::new(&bank).unwrap();
    /// let mut guesser = MaxScoreGuesser::new(GuessFrom::AllUnguessedWords, &bank, scorer);
    ///
    /// assert_eq!(guesser.select_next_guess(), Some(Arc::from("abc")));
    /// ```
    pub fn new(guess_mode: GuessFrom, word_bank: &WordBank, scorer: T) -> MaxScoreGuesser<T> {
        MaxScoreGuesser {
            guess_mode,
            grouped_words: GroupedWords::new(word_bank),
            restrictions: WordRestrictions::new(word_bank.word_length() as u8),
            scorer,
        }
    }

    /// Returns up-to the top `n` guesses for the wordle, based on the current state.
    ///
    /// Returns an empty vector if no known words are possible given the known restrictions imposed
    /// by previous calls to [`Self::update()`].
    pub fn select_top_n_guesses(&self, n: usize) -> Vec<ScoredGuess> {
        let words_to_score = match self.guess_mode {
            // Only score possible words if we're down to the last two guesses.
            _ if self.grouped_words.num_possible_words() <= 2 => {
                self.grouped_words.possible_words()
            }
            GuessFrom::AllUnguessedWords => self.grouped_words.unguessed_words(),
            GuessFrom::PossibleWords => self.grouped_words.possible_words(),
        };

        let scored_words = if words_to_score.len() > MIN_PARALLELIZATION_LIMIT {
            let mut scored_words: Vec<(&Arc<str>, i64)> = words_to_score
                .par_iter()
                .map(|word| (word, self.scorer.score_word(word)))
                .collect();
            scored_words.par_sort_by_key(|(_, score)| -score);
            scored_words
        } else {
            let mut scored_words: Vec<(&Arc<str>, i64)> = words_to_score
                .iter()
                .map(|word| (word, self.scorer.score_word(word)))
                .collect();
            scored_words.sort_by_key(|(_, score)| -score);
            scored_words
        };
        scored_words
            .into_iter()
            .take(n)
            .map(|(word, score)| ScoredGuess {
                guess: Arc::clone(word),
                score,
            })
            .collect()
    }
}

impl<T> Guesser for MaxScoreGuesser<T>
where
    T: WordScorer + Clone + Sync,
{
    fn update(&mut self, result: &GuessResult) -> Result<(), WordleError> {
        self.grouped_words.remove_guess_if_present(result.guess);
        self.restrictions.update(result)?;
        self.grouped_words
            .filter_possible_words(|word| self.restrictions.is_satisfied_by(word));
        self.scorer.update(
            result.guess,
            &self.restrictions,
            self.grouped_words.possible_words(),
        )?;
        Ok(())
    }

    fn select_next_guess(&mut self) -> Option<Arc<str>> {
        if self.guess_mode == GuessFrom::AllUnguessedWords
            && self.grouped_words.num_possible_words() > 2
        {
            let unguessed_words = self.grouped_words.unguessed_words();
            let (_, best_index) = unguessed_words
                .par_iter()
                .enumerate()
                .map(|(i, word)| (self.scorer.score_word(word), i))
                .reduce(
                    || (i64::MIN, usize::MAX),
                    |(best_score, best_index), (score, index)| {
                        if score > best_score {
                            return (score, index);
                        }
                        // Use the lower index, because it is more likely to be a possible word.
                        if score == best_score && index < best_index {
                            return (score, index);
                        }
                        (best_score, best_index)
                    },
                );
            if best_index > self.grouped_words.num_unguessed_words() {
                return None;
            } else {
                return Some(Arc::clone(&unguessed_words[best_index]));
            }
        }

        if self.grouped_words.num_possible_words() > MIN_PARALLELIZATION_LIMIT {
            return self
                .grouped_words
                .possible_words()
                .par_iter()
                .max_by_key(|word| self.scorer.score_word(word))
                .map(Arc::clone);
        }

        return self
            .grouped_words
            .possible_words()
            .iter()
            .max_by_key(|word| self.scorer.score_word(word))
            .map(Arc::clone);
    }

    fn possible_words(&self) -> &[Arc<str>] {
        self.grouped_words.possible_words()
    }
}
