use crate::data::*;
use crate::restrictions::LetterRestriction;
use crate::restrictions::WordRestrictions;
use crate::results::get_result_for_guess;
use crate::results::WordleError;
use rayon::prelude::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::result::Result;
use std::sync::Arc;

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
/// | Scorer                             |[`GuessFrom::PossibleWords`](crate::engine::GuessFrom::PossibleWords)|[`GuessFrom::AllUnguessedWords`](crate::engine::GuessFrom::PossibleWords)|
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
        possible_words: &[Arc<str>],
    ) -> Result<(), WordleError>;
    /// Determines a score for the given word. The higher the score, the better the guess.
    fn score_word(&self, word: &Arc<str>) -> i64;
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
    /// use rs_wordle_solver::MaxScoreGuesser;
    /// use rs_wordle_solver::WordBank;
    /// use rs_wordle_solver::scorers::MaxUniqueLetterFrequencyScorer;
    ///
    /// let bank = WordBank::from_iterator(&["abc", "def", "ghi"]).unwrap();
    /// let mut guesser = MaxScoreGuesser::new(
    ///     GuessFrom::AllUnguessedWords,
    ///     &bank,
    ///     MaxUniqueLetterFrequencyScorer::new(&*bank));
    ///
    /// assert!(guesser.select_next_guess().is_some());
    /// ```
    pub fn new<S>(all_words: &[S]) -> MaxUniqueLetterFrequencyScorer
    where
        S: AsRef<str>,
    {
        MaxUniqueLetterFrequencyScorer {
            guessed_letters: HashSet::new(),
            word_counter: WordCounter::new(all_words),
        }
    }
}

impl WordScorer for MaxUniqueLetterFrequencyScorer {
    fn update(
        &mut self,
        latest_guess: &str,
        _restrictions: &WordRestrictions,
        possible_words: &[Arc<str>],
    ) -> Result<(), WordleError> {
        self.guessed_letters.extend(latest_guess.chars());
        self.word_counter = WordCounter::from_iter(possible_words);
        Ok(())
    }

    fn score_word(&self, word: &Arc<str>) -> i64 {
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
    /// use rs_wordle_solver::MaxScoreGuesser;
    /// use rs_wordle_solver::WordBank;
    /// use rs_wordle_solver::scorers::LocatedLettersScorer;
    ///
    /// let bank = WordBank::from_iterator(&["abc", "def", "ghi"]).unwrap();
    /// let scorer = LocatedLettersScorer::new(&bank);
    /// let mut guesser = MaxScoreGuesser::new(GuessFrom::AllUnguessedWords, &bank, scorer.clone());
    ///
    /// assert!(guesser.select_next_guess().is_some());
    /// ```
    pub fn new(bank: &WordBank) -> LocatedLettersScorer {
        LocatedLettersScorer {
            restrictions: WordRestrictions::new(bank.word_length() as u8),
            counter: WordCounter::from_iter(&**bank),
        }
    }
}

impl WordScorer for LocatedLettersScorer {
    fn update<'a>(
        &mut self,
        _last_guess: &str,
        restrictions: &WordRestrictions,
        possible_words: &[Arc<str>],
    ) -> Result<(), WordleError> {
        self.restrictions = restrictions.clone();
        self.counter = WordCounter::from_iter(possible_words);
        Ok(())
    }

    fn score_word(&self, word: &Arc<str>) -> i64 {
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
/// These per-letter expectations are then summed together to get the expectation value for the
/// word. Approximating the expected eliminations in this way is cheap to compute, but slightly less
/// accurate, and therefore less effective, than using the precise counts computed by
/// [`MaxEliminationsScorer`]. Ignoring `MaxEliminationsScorer`'s precomputation on construction,
/// this approximate scorer is still about 10x faster.
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
    /// Constructs a `MaxApproximateEliminationsScorer` based on the given [`WordBank`].
    /// The counter should be constructed from the same bank as the associated\
    /// [`Guesser`](crate::engine::Guesser).
    ///
    /// ```
    /// use rs_wordle_solver::GuessFrom;
    /// use rs_wordle_solver::Guesser;
    /// use rs_wordle_solver::MaxScoreGuesser;
    /// use rs_wordle_solver::WordBank;
    /// use rs_wordle_solver::scorers::MaxApproximateEliminationsScorer;
    ///
    /// let bank = WordBank::from_iterator(&["abc", "def", "ghi"]).unwrap();
    /// let scorer = MaxApproximateEliminationsScorer::new(&bank);
    /// let mut guesser = MaxScoreGuesser::new(GuessFrom::AllUnguessedWords, &bank, scorer);
    ///
    /// assert!(guesser.select_next_guess().is_some());
    /// ```
    pub fn new<S>(all_words: &[S]) -> MaxApproximateEliminationsScorer
    where
        S: AsRef<str>,
    {
        MaxApproximateEliminationsScorer {
            counter: WordCounter::new(all_words),
        }
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
        possible_words: &[Arc<str>],
    ) -> Result<(), WordleError> {
        self.counter = WordCounter::from_iter(possible_words);
        Ok(())
    }

    fn score_word(&self, word: &Arc<str>) -> i64 {
        (self.compute_expected_eliminations(word.as_ref()) * 1000.0) as i64
    }
}

/// This probabilistically calculates the expectation value for how many words will be eliminated by
/// each guess, and chooses the word that eliminates the most other guesses.
///
/// This is a highly effective scoring strategy, but also quite expensive to compute. On my
/// machine, constructing the scorer for about 4600 words takes about 0.5 seconds, but each
/// subsequent game can be played in about 27ms if the scorer is then cloned before each game.
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
    possible_words: Vec<Arc<str>>,
    first_expected_eliminations_per_word: HashMap<Arc<str>, f64>,
    is_first_round: bool,
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
    /// use rs_wordle_solver::MaxScoreGuesser;
    /// use rs_wordle_solver::WordBank;
    /// use rs_wordle_solver::scorers::MaxEliminationsScorer;
    ///
    /// let bank = WordBank::from_iterator(&["abc", "def", "ghi"]).unwrap();
    /// let scorer = MaxEliminationsScorer::new(&bank).unwrap();
    /// let mut guesser = MaxScoreGuesser::new(GuessFrom::AllUnguessedWords, &bank, scorer);
    ///
    /// assert!(guesser.select_next_guess().is_some());
    /// ```
    pub fn new(all_words: &[Arc<str>]) -> Result<MaxEliminationsScorer, WordleError> {
        let expected_eliminations_per_word: HashMap<Arc<str>, f64> = all_words
            .par_iter()
            .map(|word| {
                (
                    Arc::clone(word),
                    compute_expected_eliminations(word, all_words),
                )
            })
            .collect();
        Ok(MaxEliminationsScorer {
            possible_words: all_words.to_vec(),
            first_expected_eliminations_per_word: expected_eliminations_per_word,
            is_first_round: true,
        })
    }

    fn compute_expected_eliminations(&self, word: &Arc<str>) -> f64 {
        compute_expected_eliminations(word, &self.possible_words)
    }
}

fn compute_expected_eliminations(word: &Arc<str>, possible_words: &[Arc<str>]) -> f64 {
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
        possible_words: &[Arc<str>],
    ) -> Result<(), WordleError> {
        self.possible_words = possible_words.to_vec();
        self.is_first_round = false;
        Ok(())
    }

    fn score_word(&self, word: &Arc<str>) -> i64 {
        if self.is_first_round {
            if let Some(expected_elimations) = self.first_expected_eliminations_per_word.get(word) {
                return (expected_elimations * 1000.0) as i64;
            }
        }
        let expected_elimations = self.compute_expected_eliminations(word);
        (expected_elimations * 1000.0) as i64
    }
}

/// This probabilistically calculates the expectation value for how many words will be eliminated by
/// the next two guesses, and chooses the word that maximizes that.
///
/// This is very expensive to run, and seems to perform worse than [`MaxEliminationsScorer`], so you
/// should probably use that instead. Constructing this solver with 4602 words takes almost 6 hours.
#[derive(Clone)]
pub struct MaxComboEliminationsScorer {
    all_words: Vec<Arc<str>>,
    possible_words: Vec<Arc<str>>,
    first_expected_eliminations_per_word: HashMap<Arc<str>, f64>,
    is_first_round: bool,
    min_possible_words_for_combo: usize,
}

impl MaxComboEliminationsScorer {
    /// Constructs a `MaxComboEliminationsScorer`. **Be careful, this is expensive to compute!**
    ///
    /// Once constructed for a given set of words, this precomputation can be reused by simply
    /// cloning a new version of the scorer for each game.
    ///
    /// The cost of this function scales in approximately *O*(*n*<sup>3</sup>), where *n* is the
    /// number of words in `all_words`. `min_possible_words_for_combo` indicates the threshold at
    /// which this scorer will only score words for the max eliminations on a single guess (i.e.
    /// [`MaxEliminationsScorer`] behavior) instead of calculating the expected eliminations in
    /// combination with a subsequent guess.
    ///
    /// ```
    /// use rs_wordle_solver::GuessFrom;
    /// use rs_wordle_solver::Guesser;
    /// use rs_wordle_solver::MaxScoreGuesser;
    /// use rs_wordle_solver::WordBank;
    /// use rs_wordle_solver::scorers::MaxComboEliminationsScorer;
    ///
    /// let bank = WordBank::from_iterator(&["abc", "def", "ghi"]).unwrap();
    /// let scorer = MaxComboEliminationsScorer::new(&bank, 1000).unwrap();
    /// let mut guesser = MaxScoreGuesser::new(GuessFrom::AllUnguessedWords, &bank, scorer);
    ///
    /// assert!(guesser.select_next_guess().is_some());
    /// ```
    pub fn new(
        all_words: &[Arc<str>],
        min_possible_words_for_combo: usize,
    ) -> Result<MaxComboEliminationsScorer, WordleError> {
        let mut scorer = MaxComboEliminationsScorer {
            all_words: all_words.iter().map(Arc::clone).collect(),
            possible_words: all_words.iter().map(Arc::clone).collect(),
            first_expected_eliminations_per_word: HashMap::new(),
            is_first_round: true,
            min_possible_words_for_combo,
        };
        scorer.first_expected_eliminations_per_word = all_words.par_iter()
            .map(|word| (Arc::clone(word), scorer.compute_expected_combo_eliminations(word)))
            .collect();
        Ok(scorer)
    }

    fn compute_expected_eliminations(&self, word: &Arc<str>) -> f64 {
        if self.possible_words.len() > self.min_possible_words_for_combo {
            self.compute_expected_combo_eliminations(word)
        } else {
            compute_expected_eliminations(word, &self.possible_words)
        }
    }

    fn compute_expected_combo_eliminations(&self, word: &Arc<str>) -> f64 {
        let mut best_expected_eliminations = 0.0;
        let num_possible_words = self.possible_words.len();
        let mut first_guess_result_per_objective: HashMap<Arc<str>, CompressedGuessResult> =
            HashMap::with_capacity(num_possible_words);
        for possible_objective in &self.possible_words {
            let first_guess_result =
                get_result_for_guess(possible_objective.as_ref(), word.as_ref()).unwrap();
            let compressed_result =
                CompressedGuessResult::from_results(&first_guess_result.results).unwrap();
            first_guess_result_per_objective
                .insert(Arc::clone(possible_objective), compressed_result);
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
        possible_words: &[Arc<str>],
    ) -> Result<(), WordleError> {
        self.possible_words = possible_words.to_vec();
        self.is_first_round = false;
        Ok(())
    }

    fn score_word(&self, word: &Arc<str>) -> i64 {
        if self.is_first_round {
            if let Some(expected_elimations) = self.first_expected_eliminations_per_word.get(word) {
                return (expected_elimations * 1000.0) as i64;
            }
        }
        let expected_elimations = self.compute_expected_eliminations(word);
        (expected_elimations * 1000.0) as i64
    }
}
