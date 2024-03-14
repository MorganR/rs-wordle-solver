use rayon::prelude::*;

use crate::data::*;
use crate::restrictions::WordRestrictions;
use crate::results::*;
use crate::scorers::WordScorer;
use std::num::NonZeroUsize;
use std::result::Result;
use std::sync::Arc;

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
/// let mut guesser = RandomGuesser::new(bank);
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// let guesser = RandomGuesser::new(bank);
    /// ```
    pub fn new(bank: WordBank) -> RandomGuesser {
        let word_length = bank.word_length();
        RandomGuesser {
            possible_words: bank.all_words,
            restrictions: WordRestrictions::new(word_length as u8),
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MaxScoreGuesser<T>
where
    T: WordScorer + Clone + Sync,
{
    guess_mode: GuessFrom,
    grouped_words: GroupedWords,
    restrictions: WordRestrictions,
    scorer: T,
    parallelisation_limit: usize,
    word_scores: Option<Vec<i64>>,
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
    /// let scorer = MaxEliminationsScorer::new(bank.clone());
    /// let mut guesser = MaxScoreGuesser::new(GuessFrom::AllUnguessedWords, bank, scorer);
    ///
    /// assert_eq!(guesser.select_next_guess(), Some(Arc::from("abc")));
    /// ```
    pub fn new(guess_mode: GuessFrom, word_bank: WordBank, scorer: T) -> MaxScoreGuesser<T> {
        Self::with_parallelisation_limit(
            guess_mode,
            word_bank,
            scorer,
            std::thread::available_parallelism()
                .map(NonZeroUsize::get)
                .unwrap_or(1),
        )
    }

    /// Constructs a new `MaxScoreGuesser` like [`Self::new()`], but with a custom
    /// `parallelisation_limit`. Various internal operations may be parallelised if operating on
    /// lists greater than this limit. The default setting is the result of
    /// `std::thread::available_parallelism`.
    pub fn with_parallelisation_limit(
        guess_mode: GuessFrom,
        word_bank: WordBank,
        scorer: T,
        parallelisation_limit: usize,
    ) -> MaxScoreGuesser<T> {
        let word_length = word_bank.word_length();
        MaxScoreGuesser {
            guess_mode,
            grouped_words: GroupedWords::new(word_bank),
            restrictions: WordRestrictions::new(word_length as u8),
            scorer,
            parallelisation_limit,
            word_scores: None,
        }
    }

    /// Returns up-to the top `n` guesses for the wordle, based on the current state.
    ///
    /// Returns an empty vector if no known words are possible given the known restrictions imposed
    /// by previous calls to [`Self::update()`].
    pub fn select_top_n_guesses(&mut self, n: usize) -> Vec<ScoredGuess> {
        self.compute_scores_if_unknown();
        let word_scores: &Vec<i64> = self.word_scores.as_ref().unwrap();
        let mut scored_words: Vec<(&Arc<str>, i64)> = word_scores
            .iter()
            .zip(self.words_to_score().iter())
            .map(|(score, word)| (word, *score))
            .collect();

        if scored_words.len() >= self.parallelisation_limit {
            // Use a stable sort, because possible words come before impossible words, and we want
            // to prioritise possible words.
            scored_words.par_sort_by_key(|(_, score)| -score);
        } else {
            // Use a stable sort, because possible words come before impossible words, and we want
            // to prioritise possible words.
            scored_words.sort_by_key(|(_, score)| -score);
        };
        scored_words
            .iter()
            .take(n)
            .map(|(word, score)| ScoredGuess {
                score: *score,
                guess: Arc::clone(*word),
            })
            .collect()
    }

    /// Computes the word scores if they are not known. The result is cached into
    /// [`Self::word_scores`] until the scorer's state changes.
    ///
    /// This can be useful to precompute the scores for the first guess in a base guesser, and then
    /// clone that guesser for each use.
    ///
    /// ```
    /// # use std::sync::Arc;
    /// # use rs_wordle_solver::GuessFrom;
    /// # use rs_wordle_solver::Guesser;
    /// # use rs_wordle_solver::MaxScoreGuesser;
    /// # use rs_wordle_solver::WordBank;
    /// # use rs_wordle_solver::scorers::MaxEliminationsScorer;
    ///
    /// let bank = WordBank::from_iterator(&["azz", "bzz", "czz", "abc"]).unwrap();
    /// let scorer = MaxEliminationsScorer::new(bank.clone());
    /// let mut base_guesser = MaxScoreGuesser::new(GuessFrom::AllUnguessedWords, bank, scorer);
    ///
    /// // Precompute the first scores.
    /// base_guesser.compute_word_scores_if_unknown();
    ///
    /// // Clone a new guesser for each use.
    /// let guesser = base_guesser.clone();
    /// ```
    ///
    /// You can also use the `serde` feature to serialise an instance after doing this
    /// precomputation.
    pub fn compute_scores_if_unknown(&mut self) {
        if self.word_scores.is_none() {
            let words_to_score = self.words_to_score();
            let word_scores = if words_to_score.len() >= self.parallelisation_limit {
                words_to_score
                    .par_iter()
                    .map(|word| self.scorer.score_word(word))
                    .collect()
            } else {
                words_to_score
                    .iter()
                    .map(|word| self.scorer.score_word(word))
                    .collect()
            };
            self.word_scores = Some(word_scores);
        }
    }

    /// Retrieves the words that need scoring, in the same order as the
    fn words_to_score(&self) -> &[Arc<str>] {
        match self.guess_mode {
            // Only score possible words if we're down to the last two guesses.
            _ if self.grouped_words.num_possible_words() <= 2 => {
                self.grouped_words.possible_words()
            }
            GuessFrom::AllUnguessedWords => self.grouped_words.unguessed_words(),
            GuessFrom::PossibleWords => self.grouped_words.possible_words(),
        }
    }
}

impl<T> Guesser for MaxScoreGuesser<T>
where
    T: WordScorer + Clone + Sync,
{
    fn update(&mut self, result: &GuessResult) -> Result<(), WordleError> {
        self.word_scores = None;
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
        self.compute_scores_if_unknown();
        let word_scores = self.word_scores.as_ref().unwrap();
        let words_to_score = self.words_to_score();
        let (best_index, _) = if words_to_score.len() > self.parallelisation_limit {
            word_scores.par_iter().enumerate().reduce(
                || (usize::MAX, &i64::MIN),
                |(best_index, best_score), (index, score)| {
                    if *score > *best_score {
                        return (index, score);
                    }
                    // Use the lower index, because it is more likely to be a possible word.
                    if *score == *best_score && index < best_index {
                        return (index, score);
                    }
                    (best_index, best_score)
                },
            )
        } else {
            let mut best_score = &i64::MIN;
            let mut best_index = usize::MAX;
            word_scores.iter().enumerate().for_each(|(i, score)| {
                if *score > *best_score {
                    best_score = score;
                    best_index = i;
                }
            });
            (best_index, best_score)
        };
        if best_index > words_to_score.len() {
            None
        } else {
            Some(Arc::clone(&words_to_score[best_index]))
        }
    }

    fn possible_words(&self) -> &[Arc<str>] {
        self.grouped_words.possible_words()
    }
}
