use crate::data::*;
use crate::restrictions::WordRestrictions;
use crate::results::*;
use crate::scorers::WordScorer;
use std::result::Result;
use std::sync::Arc;

/// Guesses words in order to solve a single Wordle.
pub trait Guesser {
    /// Updates this guesser with information about a word.
    fn update<'a>(&mut self, result: &'a GuessResult) -> Result<(), WordleError>;

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
#[derive(Clone)]
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
    fn update<'a>(&mut self, result: &'a GuessResult) -> Result<(), WordleError> {
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
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum GuessFrom {
    /// Choose the next guess from any unguessed word in the whole word list.
    AllUnguessedWords,
    /// Choose the next guess from any possible word based on the current restrictions.
    PossibleWords,
}

/// Represents a guess with a 'score' estimating how useful the guess is. Higher scores are better.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScoredGuess {
    pub score: i64,
    pub guess: Arc<str>,
}

/// Selects the next guess that maximizes the score according to the owned scorer.
///
/// See [`WordScorer`] for more information about possible scoring algorithms.
#[derive(Clone)]
pub struct MaxScoreGuesser<T>
where
    T: WordScorer + Clone,
{
    guess_mode: GuessFrom,
    all_unguessed_words: Vec<Arc<str>>,
    possible_words: Vec<Arc<str>>,
    restrictions: WordRestrictions,
    scorer: T,
}

impl<T> MaxScoreGuesser<T>
where
    T: WordScorer + Clone,
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
            all_unguessed_words: word_bank.to_vec(),
            possible_words: word_bank.to_vec(),
            restrictions: WordRestrictions::new(word_bank.word_length() as u8),
            scorer,
        }
    }

    /// Returns up-to the top `n` guesses for the wordle, based on the current state.
    ///
    /// Returns an empty vector if no known words are possible given the known restrictions imposed
    /// by previous calls to [`Self::update()`].
    pub fn select_top_n_guesses(&mut self, n: usize) -> Vec<ScoredGuess> {
        let words_to_score = match self.guess_mode {
            // Only score possible words if we're down to the last two guesses.
            _ if self.possible_words.len() <= 2 => self.possible_words.iter(),
            GuessFrom::AllUnguessedWords => self.all_unguessed_words.iter(),
            GuessFrom::PossibleWords => self.possible_words.iter(),
        };

        let mut scored_words: Vec<_> = words_to_score
            .map(|word| ScoredGuess {
                guess: Arc::clone(word),
                score: self.scorer.score_word(word),
            })
            .collect();
        scored_words.sort_unstable();
        return scored_words.into_iter().rev().take(n).collect();
    }
}

impl<T> Guesser for MaxScoreGuesser<T>
where
    T: WordScorer + Clone,
{
    fn update<'a>(&mut self, result: &'a GuessResult) -> Result<(), WordleError> {
        if let Some(position) = self
            .all_unguessed_words
            .iter()
            .position(|word| word.as_ref() == result.guess)
        {
            self.all_unguessed_words.swap_remove(position);
        }
        self.restrictions.update(result)?;
        self.possible_words
            .retain(|word| self.restrictions.is_satisfied_by(word.as_ref()));
        self.scorer
            .update(result.guess, &self.restrictions, &self.possible_words)?;
        Ok(())
    }

    fn select_next_guess(&mut self) -> Option<Arc<str>> {
        if self.guess_mode == GuessFrom::AllUnguessedWords && self.possible_words.len() > 2 {
            let mut best_word = self.all_unguessed_words.get(0);
            let mut best_score = best_word.map_or(0, |word| self.scorer.score_word(word));
            let mut scores_all_same = true;
            for word in self.all_unguessed_words.iter() {
                let score = self.scorer.score_word(word);
                if best_score != score {
                    scores_all_same = false;
                    if best_score < score {
                        best_score = score;
                        best_word = Some(word);
                    }
                }
            }
            if !scores_all_same {
                return best_word.map(Arc::clone);
            } else {
                return self.possible_words.get(0).map(Arc::clone);
            }
        }

        return self
            .possible_words
            .iter()
            .max_by_key(|word| self.scorer.score_word(word))
            .map(Arc::clone);
    }

    fn possible_words(&self) -> &[Arc<str>] {
        &self.possible_words
    }
}
