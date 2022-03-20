use crate::data::*;
use crate::restrictions::LetterRestriction;
use crate::restrictions::WordRestrictions;
use crate::results::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;
use std::result::Result;

/// Guesses words in order to solve a single Wordle.
pub trait Guesser {
    /// Updates this guesser with information about a word.
    fn update<'a>(&mut self, result: &'a GuessResult) -> Result<(), WordleError>;

    /// Selects a new guess for the Wordle.
    ///
    /// Returns `None` if no known words are possible given the known restrictions imposed by
    /// previous calls to [`Self::update()`].
    fn select_next_guess(&mut self) -> Option<Rc<str>>;

    /// Provides read access to the remaining set of possible words in this guesser.
    fn possible_words(&self) -> &[Rc<str>];
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
#[derive(Clone)]
pub struct RandomGuesser {
    possible_words: Vec<Rc<str>>,
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

    fn select_next_guess(&mut self) -> Option<Rc<str>> {
        if self.possible_words.is_empty() {
            return None;
        }
        let random: usize = rand::random();
        self.possible_words
            .get(random % self.possible_words.len())
            .map(Rc::clone)
    }

    fn possible_words(&self) -> &[Rc<str>] {
        &self.possible_words
    }
}

/// Gives words a score, where the maximum score indicates the best guess.
///
/// Each implementation has different pros and cons, but generally you want to use either the
/// [`MaxEliminationsScorer`] to guess the word in the fewest number of guesses if you can afford
/// the computation/time cost, or the [`MaxApproximateEliminationsScorer`] for decent guessing
/// performance at considerably lower computational cost.
///
/// For comparison, each scorer was benchmarked guessing each word in the `data/improved-words.txt`
/// file. This table shows the average number of guesses needed to find the objective word. There
/// are more details in the docs for each scorer.
///
/// | Scorer                             |[`GuessFrom::PossibleWords`]|[`GuessFrom::AllUnguessedWords`]|
/// |------------------------------------|----------------------------|--------------------------------|
/// |[`MaxEliminationsScorer`]           |               3.95 +/- 1.10|                   3.78 +/- 0.65|
/// |[`MaxApproximateEliminationsScorer`]|               4.02 +/- 1.16|                   3.85 +/- 0.72|
/// |[`LocatedLettersScorer`]            |               4.00 +/- 1.15|                   3.90 +/- 0.99|
/// |[`MaxUniqueLetterFrequencyScorer`]  |               4.16 +/- 1.22|                   4.12 +/- 0.87|
pub trait WordScorer {
    /// Updates the scorer with the latest guess, the updated set of restrictions, and the updated
    /// list of possible words.
    fn update(
        &mut self,
        latest_guess: &str,
        restrictions: &WordRestrictions,
        possible_words: &[Rc<str>],
    ) -> Result<(), WordleError>;
    /// Determines a score for the given word. The higher the score, the better the guess.
    fn score_word(&mut self, word: &Rc<str>) -> i64;
}

/// Indicates which set of words to guess from. See [`MaxScoreGuesser::new()`].
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum GuessFrom {
    /// Choose the next guess from any unguessed word in the whole word list.
    AllUnguessedWords,
    /// Choose the next guess from any possible word based on the current restrictions.
    PossibleWords,
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
    all_unguessed_words: Vec<Rc<str>>,
    possible_words: Vec<Rc<str>>,
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
    /// use std::rc::Rc;
    /// use rs_wordle_solver::GuessFrom;
    /// use rs_wordle_solver::Guesser;
    /// use rs_wordle_solver::MaxEliminationsScorer;
    /// use rs_wordle_solver::MaxScoreGuesser;
    /// use rs_wordle_solver::WordBank;
    ///
    /// let bank = WordBank::from_iterator(&["azz", "bzz", "czz", "abc"]).unwrap();
    /// let scorer = MaxEliminationsScorer::new(&bank).unwrap();
    /// let mut guesser = MaxScoreGuesser::new(GuessFrom::AllUnguessedWords, &bank, scorer);
    ///
    /// assert_eq!(guesser.select_next_guess(), Some(Rc::from("abc")));
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

    fn select_next_guess(&mut self) -> Option<Rc<str>> {
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
                return best_word.map(Rc::clone);
            } else {
                return self.possible_words.get(0).map(Rc::clone);
            }
        }

        return self
            .possible_words
            .iter()
            .max_by_key(|word| self.scorer.score_word(word))
            .map(Rc::clone);
    }

    fn possible_words(&self) -> &[Rc<str>] {
        &self.possible_words
    }
}

/// Scores words by the number of unique words that have the same letter (in any location), summed
/// across each unique and not-yet guessed letter in the word.
///
/// When benchmarked against the 4602 words in `data/improved-words.txt`, this has the following
/// results:
///
/// |Num guesses|Num games (Guess from: `PossibleWords`)|Num games (Guess from: `AllUnguessedWords`)|
/// |-----------|---------|---------------|
/// |1|1|1|
/// |2|137|32|
/// |3|1264|1019|
/// |4|1831|2227|
/// |5|829|1054|
/// |6|321|228|
/// |7|129|36
/// |8|57|5|
/// |9|26|0|
/// |10|6|0|
/// |11|1|0|
///
/// **Average guesses:**
///
/// Guess from `PossibleWords`: 4.16 +/- 1.22
///
/// Guess from `AllUnguessedWords`: 4.12 +/- 0.87
#[derive(Clone)]
pub struct MaxUniqueLetterFrequencyScorer {
    guessed_letters: HashSet<char>,
    word_counter: WordCounter,
}

impl MaxUniqueLetterFrequencyScorer {
    /// Constructs a `MaxUniqueLetterFrequencyScorer` using the given [`WordCounter`]. The word
    /// counter should be constructed from the same word bank that the guesser will use.
    ///
    /// ```
    /// use rs_wordle_solver::GuessFrom;
    /// use rs_wordle_solver::Guesser;
    /// use rs_wordle_solver::MaxUniqueLetterFrequencyScorer;
    /// use rs_wordle_solver::MaxScoreGuesser;
    /// use rs_wordle_solver::WordBank;
    /// use rs_wordle_solver::WordCounter;
    ///
    /// let bank = WordBank::from_iterator(&["abc", "def", "ghi"]).unwrap();
    /// let mut guesser = MaxScoreGuesser::new(
    ///     GuessFrom::AllUnguessedWords,
    ///     &bank,
    ///     MaxUniqueLetterFrequencyScorer::new(WordCounter::from_iter(&*bank)));
    ///
    /// assert!(guesser.select_next_guess().is_some());
    /// ```
    pub fn new(word_counter: WordCounter) -> MaxUniqueLetterFrequencyScorer {
        MaxUniqueLetterFrequencyScorer {
            guessed_letters: HashSet::new(),
            word_counter,
        }
    }
}

impl WordScorer for MaxUniqueLetterFrequencyScorer {
    fn update(
        &mut self,
        latest_guess: &str,
        _restrictions: &WordRestrictions,
        possible_words: &[Rc<str>],
    ) -> Result<(), WordleError> {
        self.guessed_letters.extend(latest_guess.chars());
        self.word_counter = WordCounter::from_iter(possible_words);
        Ok(())
    }

    fn score_word(&mut self, word: &Rc<str>) -> i64 {
        let mut sum = 0;
        for (index, letter) in word.char_indices() {
            if (index > 0
                && word
                    .chars()
                    .take(index)
                    .any(|other_letter| other_letter == letter))
                || self.guessed_letters.contains(&letter)
            {
                continue;
            }
            sum += self.word_counter.num_words_with_letter(letter) as i64;
        }
        sum
    }
}

/// This selects the word that maximizes a score, based on both the presence and the the location of
/// that letter in the possible words. The score is computed for each letter and then summed. Each
/// letter is scored as follows.
///
/// * For each letter, score:
///
///   * 1 point if the letter must be in this location.
///   * 1 point for every word with this letter in this place if the letter's location is not yet
///     known, and this is a new location for the letter.
///   * If this letter is completely new:
///
///      * If this letter has not yet been scored in this word:
///
///         * 1 point for every possible word with this letter in the same place.
///         * 1 point for every possible word with this letter in another place.
///
///      * Else:
///
///         * 1 point for every possible word with this letter in the same place.
///
/// When benchmarked against the 4602 words in `data/improved-words.txt`, this has the following
/// results:
///
/// |Num guesses|Num games (Guess from: `PossibleWords`)|Num games (Guess from: `AllUnguessedWords`)|
/// |-----------|---------|---------------|
/// |1|1|1|
/// |2|180|114|
/// |3|1442|1558|
/// |4|1838|2023|
/// |5|722|633|
/// |6|259|180|
/// |7|101|62|
/// |8|41|22|
/// |9|13|7|
/// |10|3|2|
/// |11|1|0|
/// |12|1|0|
///
/// **Average guesses:**
///
/// Guess from `PossibleWords`: 4.00 +/- 1.15
///
/// Guess from `AllUnguessedWords`: 3.90 +/- 0.99
#[derive(Clone)]
pub struct LocatedLettersScorer {
    counter: WordCounter,
    restrictions: WordRestrictions,
}

impl LocatedLettersScorer {
    /// Constructs a `LocatedLettersScorer` based on the given [`WordBank`] and [`WordCounter`].
    /// The counter should be constructed from the same bank.
    ///
    /// ```
    /// use rs_wordle_solver::GuessFrom;
    /// use rs_wordle_solver::Guesser;
    /// use rs_wordle_solver::LocatedLettersScorer;
    /// use rs_wordle_solver::MaxScoreGuesser;
    /// use rs_wordle_solver::WordBank;
    /// use rs_wordle_solver::WordCounter;
    ///
    /// let bank = WordBank::from_iterator(&["abc", "def", "ghi"]).unwrap();
    /// let scorer = LocatedLettersScorer::new(&bank, WordCounter::from_iter(&*bank));
    /// let mut guesser = MaxScoreGuesser::new(GuessFrom::AllUnguessedWords, &bank, scorer.clone());
    ///
    /// assert!(guesser.select_next_guess().is_some());
    /// ```
    pub fn new(bank: &WordBank, counter: WordCounter) -> LocatedLettersScorer {
        LocatedLettersScorer {
            restrictions: WordRestrictions::new(bank.word_length() as u8),
            counter,
        }
    }
}

impl WordScorer for LocatedLettersScorer {
    fn update<'a>(
        &mut self,
        _last_guess: &str,
        restrictions: &WordRestrictions,
        possible_words: &[Rc<str>],
    ) -> Result<(), WordleError> {
        self.restrictions = restrictions.clone();
        self.counter = WordCounter::from_iter(possible_words);
        Ok(())
    }

    fn score_word(&mut self, word: &Rc<str>) -> i64 {
        let mut sum = 0;
        for (index, letter) in word.char_indices() {
            let located_letter = LocatedLetter::new(letter, index as u8);
            if let Some(known_state) = self.restrictions.state(&located_letter) {
                match known_state {
                    LetterRestriction::Here => {
                        sum += 1;
                        continue;
                    }
                    LetterRestriction::PresentMaybeHere => {
                        sum += self.counter.num_words_with_located_letter(&located_letter) as i64;
                        continue;
                    }
                    _ => {
                        // Letter is not here or is not in the word.
                        continue;
                    }
                }
            }
            // Nothing is known about the letter.
            let letter_already_scored = word
                .chars()
                .take(index)
                .any(|other_letter| other_letter == letter);
            if !letter_already_scored {
                sum += self.counter.num_words_with_letter(letter) as i64;
            }
            sum += self.counter.num_words_with_located_letter(&located_letter) as i64;
        }
        sum
    }
}

/// This selects the word that is expected to eliminate approximately the most other words.
/// For each letter, the expected number of eliminations is computed for each possible state:
///
/// * *{expected number of eliminated words if in state}* * *{fraction of possible words matching*
///   *this state}*
///
/// So for example, with the words `["could", "match", "coast"]`, these would be computed as follows
/// for the letter `c` in `could`:
///
/// * if correct: `match` is removed, so: 1 * (2/3)
/// * if present not here: `could` and `coast` are removed, so: 2 * (1/3)
/// * if not present: all are removed, so: 3 * (0/3) *(note: this expectation is skipped if this letter*
///   *has already been checked at another location)*.
///
/// These per-letter expectations are then summed together to get the expectation value for the word.
/// Approximating the expected eliminations in this way is cheap to compute, but slightly less accurate,
/// and therefore less effective, than using the precise counts computed by [`MaxEliminationsScorer`].
///
/// When benchmarked against the 4602 words in `data/improved-words.txt`, this has the following
/// results:
///
/// |Num guesses|Num games (Guess from: `PossibleWords`)|Num games (Guess from: `AllUnguessedWords`)|
/// |-----------|---------|---------------|
/// |1|1|1|
/// |2|180|72|
/// |3|1415|1303|
/// |4|1843|2507|
/// |5|734|664|
/// |6|262|52|
/// |7|104|3|
/// |8|41|0|
/// |9|14|0|
/// |10|6|0|
/// |11|1|0|
/// |12|1|0|
///
/// **Average guesses:**
///
/// Guess from `PossibleWords`: 4.02 +/- 1.16
///
/// Guess from `AllUnguessedWords`: 3.85 +/- 0.72
#[derive(Clone)]
pub struct MaxApproximateEliminationsScorer {
    counter: WordCounter,
}

impl MaxApproximateEliminationsScorer {
    /// Constructs a `MaxApproximateEliminationsScorer` based on the given [`WordCounter`].
    /// The counter should be constructed from the same bank as the associated [`Guesser`].
    ////Unmi
    /// ```
    /// use rs_wordle_solver::GuessFrom;
    /// use rs_wordle_solver::Guesser;
    /// use rs_wordle_solver::MaxApproximateEliminationsScorer;
    /// use rs_wordle_solver::MaxScoreGuesser;
    /// use rs_wordle_solver::WordBank;
    /// use rs_wordle_solver::WordCounter;
    ///
    /// let bank = WordBank::from_iterator(&["abc", "def", "ghi"]).unwrap();
    /// let scorer = MaxApproximateEliminationsScorer::new(WordCounter::from_iter(&*bank));
    /// let mut guesser = MaxScoreGuesser::new(GuessFrom::AllUnguessedWords, &bank, scorer);
    ///
    /// assert!(guesser.select_next_guess().is_some());
    /// ```
    pub fn new(counter: WordCounter) -> MaxApproximateEliminationsScorer {
        MaxApproximateEliminationsScorer { counter }
    }

    fn compute_expected_eliminations(&self, word: &str) -> f64 {
        let mut sum = 0.0;
        for (index, letter) in word.char_indices() {
            sum += self.compute_expected_eliminations_for_letter(
                LocatedLetter::new(letter, index as u8),
                index == 0
                    || word
                        .chars()
                        .take(index)
                        .all(|other_letter| other_letter != letter),
            );
        }
        sum
    }

    fn compute_expected_eliminations_for_letter(
        &self,
        located_letter: LocatedLetter,
        is_new_letter: bool,
    ) -> f64 {
        let num_if_correct = self.counter.num_words_with_located_letter(&located_letter) as f64;
        let num_if_present = self.counter.num_words_with_letter(located_letter.letter) as f64;
        let num_if_present_not_here = num_if_present - num_if_correct;
        let total = self.counter.num_words() as f64;
        let eliminations_if_correct = total - num_if_correct;
        let eliminations_if_present_not_here = total - num_if_present_not_here;
        let expected_eliminations_for_present_somewhere = eliminations_if_correct * num_if_correct
            / total
            + eliminations_if_present_not_here * num_if_present_not_here / total;
        if !is_new_letter {
            // Only expect the eliminations tied to location, since we've already included the
            // expected eliminations for if the letter is not present at all.
            return expected_eliminations_for_present_somewhere;
        }
        let num_if_not_present = total - num_if_present;
        let eliminations_if_not_present = num_if_present;
        expected_eliminations_for_present_somewhere
            + eliminations_if_not_present * num_if_not_present / total
    }
}

impl WordScorer for MaxApproximateEliminationsScorer {
    fn update<'a>(
        &mut self,
        _last_guess: &str,
        _restrictions: &WordRestrictions,
        possible_words: &[Rc<str>],
    ) -> Result<(), WordleError> {
        self.counter = WordCounter::from_iter(possible_words);
        Ok(())
    }

    fn score_word(&mut self, word: &Rc<str>) -> i64 {
        (self.compute_expected_eliminations(word.as_ref()) * 1000.0) as i64
    }
}

/// This probabilistically calculates the expectation value for how many words will be eliminated by
/// each guess, and chooses the word that eliminates the most other guesses.
///
/// This is a highly effective scoring strategy, but also quite expensive to compute. On my
/// machine, constructing the scorer for about 4600 words takes about 9 seconds, but each
/// subsequent game can be played in about 650ms if the scorer is then cloned before each game.
///
/// When benchmarked against the 4602 words in `data/improved-words.txt`, this has the following
/// results:
///
/// |Num guesses|Num games (Guess from: `PossibleWords`)|Num games (Guess from: `AllUnguessedWords`)|
/// |-----------|---------|---------------|
/// |1|1|1|
/// |2|180|53|
/// |3|1452|1426|
/// |4|1942|2635|
/// |5|666|468|
/// |6|220|19|
/// |7|93|0|
/// |8|33|0|
/// |9|10|0|
/// |10|4|0|
/// |11|1|0|
///
/// **Average guesses:**
///
/// Guess from `PossibleWords`: 3.95 +/- 1.10
///
/// Guess from `AllUnguessedWords`: 3.78 +/- 0.65
#[derive(Clone)]
pub struct MaxEliminationsScorer {
    possible_words: Vec<Rc<str>>,
    previous_expected_eliminations_per_word: HashMap<Rc<str>, f64>,
    max_expected_eliminations: f64,
}

impl MaxEliminationsScorer {
    /// Constructs a `MaxEliminationsScorer`. **Be careful, this is expensive to compute!**
    ///
    /// Once constructed for a given set of words, this precomputation can be reused by simply
    /// cloning a new version of the scorer for each game.
    ///
    /// The cost of this function scales in approximately *O*(*n*<sup>2</sup>), where *n* is the
    /// number of words.
    ///
    /// ```
    /// use rs_wordle_solver::GuessFrom;
    /// use rs_wordle_solver::Guesser;
    /// use rs_wordle_solver::MaxEliminationsScorer;
    /// use rs_wordle_solver::MaxScoreGuesser;
    /// use rs_wordle_solver::WordBank;
    ///
    /// let bank = WordBank::from_iterator(&["abc", "def", "ghi"]).unwrap();
    /// let scorer = MaxEliminationsScorer::new(&bank).unwrap();
    /// let mut guesser = MaxScoreGuesser::new(GuessFrom::AllUnguessedWords, &bank, scorer);
    ///
    /// assert!(guesser.select_next_guess().is_some());
    /// ```
    pub fn new(all_words: &[Rc<str>]) -> Result<MaxEliminationsScorer, WordleError> {
        let mut expected_eliminations_per_word: HashMap<Rc<str>, f64> = HashMap::new();
        let mut max_eliminations = 0.0;
        for word in all_words {
            let expected_elimations = compute_expected_eliminations(word, all_words);
            if expected_elimations > max_eliminations {
                max_eliminations = expected_elimations;
            }
            expected_eliminations_per_word.insert(Rc::clone(word), expected_elimations);
        }
        Ok(MaxEliminationsScorer {
            possible_words: all_words.to_vec(),
            previous_expected_eliminations_per_word: expected_eliminations_per_word,
            max_expected_eliminations: max_eliminations,
        })
    }

    fn can_skip(&self, word: &Rc<str>) -> bool {
        self.previous_expected_eliminations_per_word
            .get(word)
            .map_or(false, |previous| *previous < self.max_expected_eliminations)
    }

    fn compute_expected_eliminations(&mut self, word: &Rc<str>) -> f64 {
        compute_expected_eliminations(word, &self.possible_words)
    }
}

fn compute_expected_eliminations(word: &Rc<str>, possible_words: &[Rc<str>]) -> f64 {
    let mut matching_results: HashMap<CompressedGuessResult, usize> = HashMap::new();
    for possible_word in possible_words {
        let guess_result = CompressedGuessResult::from_results(
            &get_result_for_guess(possible_word.as_ref(), word.as_ref())
                .unwrap()
                .results,
        )
        .unwrap();
        *matching_results.entry(guess_result).or_insert(0) += 1;
    }
    let num_possible_words = possible_words.len();
    matching_results.into_values().fold(0, |acc, num_matched| {
        let num_eliminated = num_possible_words - num_matched;
        acc + num_eliminated * num_matched
    }) as f64
        / num_possible_words as f64
}

impl WordScorer for MaxEliminationsScorer {
    fn update(
        &mut self,
        _latest_guess: &str,
        _restrictions: &WordRestrictions,
        possible_words: &[Rc<str>],
    ) -> Result<(), WordleError> {
        self.possible_words = possible_words.to_vec();
        self.max_expected_eliminations = 0.0;
        Ok(())
    }

    fn score_word(&mut self, word: &Rc<str>) -> i64 {
        if self.can_skip(word) {
            return 0;
        }
        let expected_elimations = self.compute_expected_eliminations(word);
        if expected_elimations > self.max_expected_eliminations {
            self.max_expected_eliminations = expected_elimations;
        }
        self.previous_expected_eliminations_per_word
            .insert(Rc::clone(word), expected_elimations);
        (expected_elimations * 1000.0) as i64
    }
}

/// This probabilistically calculates the expectation value for how many words will be eliminated by
/// the next two guesses, and chooses the word that maximizes that.
#[derive(Clone)]
pub struct MaxComboEliminationsScorer {
    all_words: Vec<Rc<str>>,
    possible_words: Vec<Rc<str>>,
    previous_expected_eliminations_per_word: HashMap<Rc<str>, f64>,
    max_expected_eliminations: f64,
}

impl MaxComboEliminationsScorer {
    /// Constructs a `MaxComboEliminationsScorer`. **Be careful, this is expensive to compute!**
    ///
    /// Once constructed for a given set of words, this precomputation can be reused by simply
    /// cloning a new version of the scorer for each game.
    ///
    /// The cost of this function scales in approximately *O*(*n*<sup>3</sup>), where *n* is the
    /// number of words.
    ///
    /// ```
    /// use rs_wordle_solver::GuessFrom;
    /// use rs_wordle_solver::Guesser;
    /// use rs_wordle_solver::MaxComboEliminationsScorer;
    /// use rs_wordle_solver::MaxScoreGuesser;
    /// use rs_wordle_solver::WordBank;
    ///
    /// let bank = WordBank::from_iterator(&["abc", "def", "ghi"]).unwrap();
    /// let scorer = MaxComboEliminationsScorer::new(&bank).unwrap();
    /// let mut guesser = MaxScoreGuesser::new(GuessFrom::AllUnguessedWords, &bank, scorer);
    ///
    /// assert!(guesser.select_next_guess().is_some());
    /// ```
    pub fn new(all_words: &[Rc<str>]) -> Result<MaxComboEliminationsScorer, WordleError> {
        let mut scorer = MaxComboEliminationsScorer {
            all_words: all_words.iter().map(Rc::clone).collect(),
            possible_words: all_words.iter().map(Rc::clone).collect(),
            previous_expected_eliminations_per_word: HashMap::new(),
            max_expected_eliminations: 0.0,
        };
        for word in all_words {
            let expected_elimations = scorer.compute_expected_eliminations(word);
            if expected_elimations > scorer.max_expected_eliminations {
                scorer.max_expected_eliminations = expected_elimations;
            }
            scorer
                .previous_expected_eliminations_per_word
                .insert(Rc::clone(word), expected_elimations);
        }
        Ok(scorer)
    }

    fn can_skip(&self, word: &Rc<str>) -> bool {
        self.previous_expected_eliminations_per_word
            .get(word)
            .map_or(false, |previous| *previous < self.max_expected_eliminations)
    }

    fn compute_expected_eliminations(&self, word: &Rc<str>) -> f64 {
        if self.possible_words.len() > 100 {
            self.compute_expected_combo_eliminations(word)
        } else {
            compute_expected_eliminations(word, &self.possible_words)
        }
    }

    fn compute_expected_combo_eliminations(&self, word: &Rc<str>) -> f64 {
        let mut best_expected_eliminations = 0.0;
        let num_possible_words = self.possible_words.len();
        let mut first_guess_result_per_objective: HashMap<Rc<str>, CompressedGuessResult> =
            HashMap::with_capacity(num_possible_words);
        for possible_objective in &self.possible_words {
            let first_guess_result =
                get_result_for_guess(possible_objective.as_ref(), word.as_ref()).unwrap();
            let compressed_result =
                CompressedGuessResult::from_results(&first_guess_result.results).unwrap();
            first_guess_result_per_objective
                .insert(Rc::clone(possible_objective), compressed_result);
        }
        for second_guess in &self.all_words {
            let mut matching_results: HashMap<
                (CompressedGuessResult, CompressedGuessResult),
                usize,
            > = HashMap::new();
            for possible_objective in &self.possible_words {
                let second_guess_result =
                    get_result_for_guess(possible_objective.as_ref(), second_guess.as_ref())
                        .unwrap();
                let compressed_first_result = unsafe {
                    first_guess_result_per_objective
                        .get(possible_objective)
                        .unwrap_unchecked()
                };
                let compressed_second_result =
                    CompressedGuessResult::from_results(&second_guess_result.results).unwrap();
                *matching_results
                    .entry((*compressed_first_result, compressed_second_result))
                    .or_insert(0) += 1;
            }
            let expected_eliminations =
                matching_results.into_values().fold(0, |acc, num_matched| {
                    let num_eliminated = num_possible_words - num_matched;
                    acc + num_eliminated * num_matched
                }) as f64
                    / num_possible_words as f64;
            if best_expected_eliminations < expected_eliminations {
                best_expected_eliminations = expected_eliminations;
            }
        }
        best_expected_eliminations
    }
}

impl WordScorer for MaxComboEliminationsScorer {
    fn update(
        &mut self,
        _latest_guess: &str,
        _restrictions: &WordRestrictions,
        possible_words: &[Rc<str>],
    ) -> Result<(), WordleError> {
        self.possible_words = possible_words.to_vec();
        self.max_expected_eliminations = 0.0;
        Ok(())
    }

    fn score_word(&mut self, word: &Rc<str>) -> i64 {
        if self.can_skip(word) {
            return 0;
        }
        let expected_elimations = self.compute_expected_eliminations(word);
        if expected_elimations > self.max_expected_eliminations {
            self.max_expected_eliminations = expected_elimations;
        }
        self.previous_expected_eliminations_per_word
            .insert(Rc::clone(word), expected_elimations);
        (expected_elimations * 1000.0) as i64
    }
}
