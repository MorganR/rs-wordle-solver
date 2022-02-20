use crate::data::*;
use crate::restrictions::LetterRestriction;
use crate::restrictions::WordRestrictions;
use crate::results::*;
use crate::trie::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;
use std::result::Result;

/// Attempts to guess the given word within the maximum number of guesses, using words from the
/// word bank.
pub fn play_game(word_to_guess: &str, max_num_guesses: u32, word_bank: &WordBank) -> GameResult {
    let guesser = RandomGuesser::new(&word_bank);
    play_game_with_guesser(word_to_guess, max_num_guesses, guesser)
}

/// Attempts to guess the given word within the maximum number of guesses, using words from the
/// word bank and using the given word scorer.
pub fn play_game_with_guesser<G: Guesser>(
    word_to_guess: &str,
    max_num_guesses: u32,
    mut guesser: G,
) -> GameResult {
    let mut guesses: Vec<Box<str>> = Vec::new();
    for _ in 1..=max_num_guesses {
        let maybe_guess = guesser.select_next_guess();
        if maybe_guess.is_none() {
            return GameResult::UnknownWord;
        }
        let guess = maybe_guess.unwrap();
        guesses.push(Box::from(guess.as_ref()));
        let result = get_result_for_guess(word_to_guess, guess.as_ref());

        if result.results.iter().all(|lr| match lr {
            LetterResult::Correct => true,
            _ => false,
        }) {
            return GameResult::Success(guesses);
        }
        guesser.update(&result).unwrap();
    }
    return GameResult::Failure(guesses);
}

/// Guesses words in order to solve a single Wordle.
pub trait Guesser {
    /// Updates this guesser with information about a word.
    fn update<'a>(&mut self, result: &'a GuessResult) -> Result<(), WordleError>;

    /// Selects a new guess for the Wordle, if any words are possible.
    fn select_next_guess(&mut self) -> Option<Rc<str>>;
}

/// Guesses at random from the possible words that meet the restrictions.
#[derive(Clone)]
pub struct RandomGuesser {
    possible_words: Vec<Rc<str>>,
    restrictions: WordRestrictions,
}

impl RandomGuesser {
    pub fn new(bank: &WordBank) -> RandomGuesser {
        RandomGuesser {
            possible_words: bank.all_words(),
            restrictions: WordRestrictions::new(bank.max_word_len() as u8),
        }
    }
}

impl Guesser for RandomGuesser {
    fn update<'a>(&mut self, result: &'a GuessResult) -> Result<(), WordleError> {
        self.restrictions.update(result)?;
        self.possible_words = self
            .possible_words
            .iter()
            .filter_map(|word| {
                if self.restrictions.is_satisfied_by(word) {
                    return Some(Rc::clone(word));
                }
                None
            })
            .collect();
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
}

/// Gives words a score, where the maximum score indicates the best guess.
pub trait WordScorer {
    /// Updates the scorer with the latest guess, the updated set of restrictions, and the updated
    /// list of possible words.
    fn update(
        &mut self,
        latest_guess: &str,
        restrictions: &WordRestrictions,
        possible_words: &Vec<Rc<str>>,
    ) -> Result<(), WordleError>;
    /// Determines a score for the given word.
    fn score_word(&mut self, word: &Rc<str>) -> i64;
}

/// Indicates which set of words to guess from.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum GuessFrom {
    /// Choose the next guess from any unguessed word in the whole word list.
    AllUnguessedWords,
    /// Choose the next guess from any possible word based on the current restrictions.
    PossibleWords,
}

/// Selects the next guess that maximizes the score according to the given scorer.
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
    pub fn new(guess_mode: GuessFrom, word_bank: &WordBank, scorer: T) -> MaxScoreGuesser<T> {
        MaxScoreGuesser {
            guess_mode: guess_mode,
            all_unguessed_words: word_bank.all_words(),
            possible_words: word_bank.all_words(),
            restrictions: WordRestrictions::new(word_bank.max_word_len() as u8),
            scorer: scorer,
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
        // Unless the current guess was the winning guess, this also filters out the current guess.
        self.restrictions.update(result)?;
        self.possible_words
            .retain(|word| self.restrictions.is_satisfied_by(word.as_ref()));
        self.scorer
            .update(result.guess, &self.restrictions, &self.possible_words)?;
        Ok(())
    }

    fn select_next_guess(&mut self) -> Option<Rc<str>> {
        if self.guess_mode == GuessFrom::AllUnguessedWords {
            if self.possible_words.len() > 2 {
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
        }

        return self
            .possible_words
            .iter()
            .max_by_key(|word| self.scorer.score_word(word))
            .map(Rc::clone);
    }
}

/// Scores words by the number of unique words that have the same letter (in any location), summed
/// across each unique letter in the word.
#[derive(Clone)]
pub struct MaxUniqueLetterFrequencyScorer {
    num_words_per_letter: HashMap<char, u32>,
}

fn compute_num_words_per_letter<'a, I, T>(words: I) -> HashMap<char, u32>
where
    I: IntoIterator<Item = &'a T>,
    T: AsRef<str> + 'a,
{
    let mut num_words_per_letter: HashMap<char, u32> = HashMap::new();
    words.into_iter().for_each(|word| {
        let unique_chars: HashSet<char> = word.as_ref().chars().collect();
        unique_chars.iter().for_each(|letter| {
            *num_words_per_letter.entry(*letter).or_insert(0) += 1;
        });
    });
    num_words_per_letter
}

impl MaxUniqueLetterFrequencyScorer {
    pub fn new(possible_words: &Vec<Rc<str>>) -> MaxUniqueLetterFrequencyScorer {
        MaxUniqueLetterFrequencyScorer {
            num_words_per_letter: compute_num_words_per_letter(possible_words),
        }
    }
}

impl WordScorer for MaxUniqueLetterFrequencyScorer {
    fn update(
        &mut self,
        _latest_guess: &str,
        _restrictions: &WordRestrictions,
        possible_words: &Vec<Rc<str>>,
    ) -> Result<(), WordleError> {
        self.num_words_per_letter = compute_num_words_per_letter(possible_words);
        Ok(())
    }

    fn score_word(&mut self, word: &Rc<str>) -> i64 {
        let unique_chars: HashSet<char> = word.chars().collect();
        unique_chars.iter().fold(0 as i64, |sum, letter| {
            sum + *self.num_words_per_letter.get(letter).unwrap_or(&0) as i64
        })
    }
}

/// Scores words by the number of unique words that have the same letter (in any location), summed
/// across each unique and not-yet guessed letter in the word.
#[derive(Clone)]
pub struct MaxUniqueUnguessedLetterFrequencyScorer {
    guessed_letters: HashSet<char>,
    num_words_per_letter: HashMap<char, u32>,
}

impl MaxUniqueUnguessedLetterFrequencyScorer {
    pub fn new<'a, I, T>(possible_words: I) -> MaxUniqueUnguessedLetterFrequencyScorer
    where
        I: IntoIterator<Item = &'a T>,
        T: AsRef<str> + 'a,
    {
        MaxUniqueUnguessedLetterFrequencyScorer {
            guessed_letters: HashSet::new(),
            num_words_per_letter: compute_num_words_per_letter(possible_words),
        }
    }
}

impl WordScorer for MaxUniqueUnguessedLetterFrequencyScorer {
    fn update(
        &mut self,
        latest_guess: &str,
        _restrictions: &WordRestrictions,
        possible_words: &Vec<Rc<str>>,
    ) -> Result<(), WordleError> {
        self.guessed_letters.extend(latest_guess.chars());
        self.num_words_per_letter = compute_num_words_per_letter(possible_words);
        for (letter, count) in self.num_words_per_letter.iter_mut() {
            if self.guessed_letters.contains(letter) {
                *count = 0;
            }
        }
        Ok(())
    }

    fn score_word(&mut self, word: &Rc<str>) -> i64 {
        let unique_chars: HashSet<char> = word.chars().collect();
        return unique_chars.iter().fold(0 as i64, |sum, letter| {
            sum + *self.num_words_per_letter.get(letter).unwrap_or(&0) as i64
        });
    }
}

#[derive(Clone)]
pub struct PossibleOutcome<'a> {
    /// The probability that this result will happen.
    pub probability: f64,
    /// The words that will still be possible with this result.
    pub still_possible_words: WordTracker<'a>,
}

impl<'a> PossibleOutcome<'a> {
    fn expected_num_eliminations(&self, total_num_possible_words: usize) -> f64 {
        self.probability
            * (total_num_possible_words - self.still_possible_words.all_words().len()) as f64
    }
}

#[derive(Clone)]
pub struct PossibleOutcomes<'a> {
    pub(crate) outcomes: Vec<PossibleOutcome<'a>>,
}

impl<'a> PossibleOutcomes<'a> {
    pub(crate) fn expected_num_eliminations(&self, total_num_possible_words: usize) -> f64 {
        self.outcomes.iter().fold(0.0, |acc, outcome| {
            acc + outcome.expected_num_eliminations(total_num_possible_words)
        })
    }
}

#[derive(Clone)]
pub struct MaxEliminationsScorer<'a> {
    guess_results: GuessResults,
    restrictions: WordRestrictions,
    possible_words: WordTracker<'a>,
}

impl<'a> MaxEliminationsScorer<'a> {
    pub fn new(all_words_tracker: WordTracker<'a>) -> MaxEliminationsScorer {
        let guess_results = GuessResults::compute(all_words_tracker.all_words());
        MaxEliminationsScorer {
            restrictions: WordRestrictions::new(all_words_tracker.max_word_length()),
            possible_words: all_words_tracker,
            guess_results: guess_results,
        }
    }

    fn can_skip(&self, word: &str) -> bool {
        let mut can_skip = true;
        for (index, letter) in word.char_indices() {
            if self
                .restrictions
                .is_state_known(LocatedLetter::new(letter, index as u8))
            {
                continue;
            }
            if !self.possible_words.has_letter(letter) {
                continue;
            }
            can_skip = false;
            break;
        }
        can_skip
    }

    fn compute_expected_eliminations(&mut self, word: &Rc<str>) -> f64 {
        let mut matching_results: HashMap<CompressedGuessResult, usize> = HashMap::new();
        for possible_word in self.possible_words.all_words() {
            let guess_result = self
                .guess_results
                .get_result(possible_word, word)
                .unwrap_or_else(|| {
                    CompressedGuessResult::from_result(
                        &get_result_for_guess(possible_word.as_ref(), word.as_ref()).results,
                    )
                });
            *matching_results.entry(guess_result).or_insert(0) += 1;
        }
        let num_possible_words = self.possible_words.all_words().len();
        matching_results.into_values().fold(0, |acc, num_matched| {
            let num_eliminated = num_possible_words - num_matched;
            return acc + num_eliminated * num_matched;
        }) as f64
            / num_possible_words as f64
    }
}

impl<'a> WordScorer for MaxEliminationsScorer<'a> {
    fn update(
        &mut self,
        _latest_guess: &str,
        restrictions: &WordRestrictions,
        possible_words: &Vec<Rc<str>>,
    ) -> Result<(), WordleError> {
        self.possible_words = WordTracker::new(possible_words);
        self.restrictions = restrictions.clone();
        Ok(())
    }

    fn score_word(&mut self, word: &Rc<str>) -> i64 {
        if self.can_skip(word.as_ref()) {
            return 0;
        }
        (self.compute_expected_eliminations(word) * 1000.0) as i64
    }
}

/// Scores words by the number of unique words that have the same letter (in any location), summed
/// across each unique and not-yet guessed letter in the word.
#[derive(Clone)]
pub struct MaxExpectedEliminationsScorer<'a> {
    restrictions: WordRestrictions,
    possible_words: WordTracker<'a>,
    memoized_possibilities: Trie<'a, PossibleOutcomes<'a>>,
}

impl<'a> MaxExpectedEliminationsScorer<'a> {
    pub fn new(possible_words_tracker: WordTracker<'a>) -> MaxExpectedEliminationsScorer<'a> {
        MaxExpectedEliminationsScorer {
            restrictions: WordRestrictions::new(possible_words_tracker.max_word_length()),
            possible_words: possible_words_tracker,
            memoized_possibilities: Trie::new(),
        }
    }

    pub fn from_precomputed(
        possible_words_tracker: WordTracker<'a>,
        first_possibilities_trie: Trie<'a, PossibleOutcomes<'a>>,
    ) -> MaxExpectedEliminationsScorer<'a> {
        MaxExpectedEliminationsScorer {
            restrictions: WordRestrictions::new(possible_words_tracker.max_word_length()),
            possible_words: possible_words_tracker,
            memoized_possibilities: first_possibilities_trie,
        }
    }

    pub fn precompute_possibilities<'b>(
        possible_words_tracker: WordTracker<'b>,
    ) -> Trie<'b, PossibleOutcomes> {
        let all_words: Vec<Rc<str>> = possible_words_tracker
            .all_words()
            .iter()
            .map(Rc::clone)
            .collect();
        let mut scorer = MaxExpectedEliminationsScorer::new(possible_words_tracker);

        for word in all_words {
            scorer.score_word(&word);
        }
        return scorer.memoized_possibilities;
    }

    fn can_skip(&self, word: &str) -> bool {
        let mut can_skip = true;
        for (index, letter) in word.char_indices() {
            if self
                .restrictions
                .is_state_known(LocatedLetter::new(letter, index as u8))
            {
                continue;
            }
            if !self.possible_words.has_letter(letter) {
                continue;
            }
            can_skip = false;
            break;
        }
        can_skip
    }

    fn compute_possible_outcome<'b, 'c>(
        possible_words_after: impl Iterator<Item = &'b Rc<str>>,
        num_possible_words_before: usize,
        probability_so_far: f64,
    ) -> Option<PossibleOutcome<'c>> {
        let mut peekable_words = possible_words_after.peekable();
        if peekable_words.peek() == None {
            return None;
        }
        let possible_words_tracker = WordTracker::new(peekable_words);
        let num_matching_words = possible_words_tracker.all_words().len();

        let outcome = PossibleOutcome {
            probability: (num_matching_words as f64 / num_possible_words_before as f64)
                * probability_so_far,
            still_possible_words: possible_words_tracker,
        };
        Some(outcome)
    }
}

impl<'a> WordScorer for MaxExpectedEliminationsScorer<'a> {
    fn update(
        &mut self,
        _latest_guess: &str,
        restrictions: &WordRestrictions,
        possible_words: &Vec<Rc<str>>,
    ) -> Result<(), WordleError> {
        self.restrictions = restrictions.clone();
        self.possible_words = WordTracker::new(possible_words);
        self.memoized_possibilities = Trie::new();
        Ok(())
    }

    fn score_word(&mut self, word: &Rc<str>) -> i64 {
        if self.can_skip(word) {
            return 0;
        }
        let starting_index;
        let mut possible_outcomes;
        let base_outcomes;
        if let Some(subtree) = self.memoized_possibilities.get_ancestor(word) {
            starting_index = subtree.key.len();
            possible_outcomes = subtree.value;
        } else {
            starting_index = 0;
            base_outcomes = PossibleOutcomes {
                outcomes: vec![PossibleOutcome {
                    probability: 1.0,
                    still_possible_words: self.possible_words.clone(),
                }],
            };
            possible_outcomes = &base_outcomes;
        }
        for (index, letter) in word.char_indices().skip(starting_index) {
            let mut all_outcomes_here: Vec<PossibleOutcome> = Vec::new();
            for outcome in possible_outcomes.outcomes.iter() {
                let num_possible_words = outcome.still_possible_words.all_words().len();
                // Determine possible words for each outcome.
                let located_letter = LocatedLetter::new(letter, index as u8);
                let words_if_correct = outcome
                    .still_possible_words
                    .words_with_located_letter(&located_letter);
                let words_if_present_not_here = outcome
                    .still_possible_words
                    .words_with_letter_not_here(&located_letter);
                let words_if_not_present =
                    outcome.still_possible_words.words_without_letter(&letter);

                all_outcomes_here.extend(
                    [
                        MaxExpectedEliminationsScorer::compute_possible_outcome(
                            words_if_correct,
                            num_possible_words,
                            outcome.probability,
                        ),
                        MaxExpectedEliminationsScorer::compute_possible_outcome(
                            words_if_present_not_here,
                            num_possible_words,
                            outcome.probability,
                        ),
                        MaxExpectedEliminationsScorer::compute_possible_outcome(
                            words_if_not_present,
                            num_possible_words,
                            outcome.probability,
                        ),
                    ]
                    .into_iter()
                    .flat_map(|next_outcome| next_outcome),
                );
            }
            let prefix = unsafe { word.get_unchecked(0..(index + letter.len_utf8())) };
            self.memoized_possibilities.insert(
                prefix,
                PossibleOutcomes {
                    outcomes: all_outcomes_here,
                },
            );
            possible_outcomes = self
                .memoized_possibilities
                .get_ancestor(prefix)
                .unwrap()
                .value;
        }
        let num_all_possible_words = self.possible_words.all_words().len();
        let score =
            (possible_outcomes.expected_num_eliminations(num_all_possible_words) * 1000.0) as i64;
        score
    }
}

/// Maximizes a score as follows:
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
#[derive(Clone)]
pub struct LocatedLettersScorer {
    counter: WordCounter,
    restrictions: WordRestrictions,
}

impl LocatedLettersScorer {
    pub fn new(bank: &WordBank, counter: WordCounter) -> LocatedLettersScorer {
        LocatedLettersScorer {
            restrictions: WordRestrictions::new(bank.max_word_len() as u8),
            counter: counter,
        }
    }
}

impl WordScorer for LocatedLettersScorer {
    fn update<'a>(
        &mut self,
        _last_guess: &str,
        restrictions: &WordRestrictions,
        possible_words: &Vec<Rc<str>>,
    ) -> Result<(), WordleError> {
        self.restrictions = restrictions.clone();
        self.counter = WordCounter::new(possible_words);
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

/// Chooses the word that is expected to eliminate approximately the most other words.
#[derive(Clone)]
pub struct MaxApproximateEliminationsScorer {
    counter: WordCounter,
}

impl MaxApproximateEliminationsScorer {
    pub fn new(counter: WordCounter) -> MaxApproximateEliminationsScorer {
        MaxApproximateEliminationsScorer { counter: counter }
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
        return expected_eliminations_for_present_somewhere
            + eliminations_if_not_present * num_if_not_present / total;
    }
}

impl WordScorer for MaxApproximateEliminationsScorer {
    fn update<'a>(
        &mut self,
        _last_guess: &str,
        _restrictions: &WordRestrictions,
        possible_words: &Vec<Rc<str>>,
    ) -> Result<(), WordleError> {
        self.counter = WordCounter::new(possible_words);
        Ok(())
    }

    fn score_word(&mut self, word: &Rc<str>) -> i64 {
        (self.compute_expected_eliminations(word.as_ref()) * 1000.0) as i64
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_result_for_guess() {
        assert_eq!(
            get_result_for_guess("abba", "back"),
            GuessResult {
                guess: "back",
                results: vec![
                    LetterResult::PresentNotHere,
                    LetterResult::PresentNotHere,
                    LetterResult::NotPresent,
                    LetterResult::NotPresent
                ],
            }
        );
        assert_eq!(
            get_result_for_guess("abba", "babb"),
            GuessResult {
                guess: "babb",
                results: vec![
                    LetterResult::PresentNotHere,
                    LetterResult::PresentNotHere,
                    LetterResult::Correct,
                    LetterResult::NotPresent
                ],
            }
        );
        assert_eq!(
            get_result_for_guess("abcd", "bcce"),
            GuessResult {
                guess: "bcce",
                results: vec![
                    LetterResult::PresentNotHere,
                    LetterResult::NotPresent,
                    LetterResult::Correct,
                    LetterResult::NotPresent
                ],
            }
        );
    }

    fn to_string_vec(words: Vec<&str>) -> Vec<String> {
        words.iter().map(|word| word.to_string()).collect()
    }

    #[test]
    fn score_located_letters_guesser_score_word() {
        let bank = WordBank::from_vec(to_string_vec(vec![
            "alpha", "allot", "begot", "below", "endow", "ingot",
        ]));
        let mut scorer = LocatedLettersScorer::new(&bank, WordCounter::new(&bank.all_words()));

        assert_eq!(scorer.score_word(&Rc::from("alpha")), 4 + 5 + 2 + 2 + 1);
        assert_eq!(scorer.score_word(&Rc::from("allot")), 4 + 5 + 2 + 10 + 6);
        assert_eq!(scorer.score_word(&Rc::from("begot")), 4 + 5 + 4 + 10 + 6);
        assert_eq!(scorer.score_word(&Rc::from("below")), 4 + 5 + 5 + 10 + 4);
        assert_eq!(scorer.score_word(&Rc::from("endow")), 4 + 4 + 2 + 10 + 4);
        assert_eq!(scorer.score_word(&Rc::from("ingot")), 2 + 4 + 4 + 10 + 6);
        assert_eq!(scorer.score_word(&Rc::from("other")), 5 + 3 + 1 + 3 + 0);
    }

    #[test]
    fn score_located_letters_guesser_update() -> Result<(), WordleError> {
        let bank = WordBank::from_vec(to_string_vec(vec![
            "alpha", "allot", "begot", "below", "endow", "ingot",
        ]));
        let mut scorer = LocatedLettersScorer::new(&bank, WordCounter::new(&bank.all_words()));

        let restrictions = WordRestrictions::from_result(&GuessResult {
            guess: "begot",
            results: vec![
                LetterResult::NotPresent,
                LetterResult::PresentNotHere,
                LetterResult::NotPresent,
                LetterResult::Correct,
                LetterResult::NotPresent,
            ],
        })?;
        scorer.update("begot", &restrictions, &vec![Rc::from("endow")])?;
        // Remaining possible words: 'endow'

        assert_eq!(scorer.score_word(&Rc::from("alpha")), 0 + 0 + 0 + 0 + 0);
        assert_eq!(scorer.score_word(&Rc::from("below")), 0 + 0 + 0 + 1 + 2);
        assert_eq!(scorer.score_word(&Rc::from("endow")), 1 + 2 + 2 + 1 + 2);
        assert_eq!(scorer.score_word(&Rc::from("other")), 0 + 0 + 0 + 0 + 0);
        Ok(())
    }

    #[test]
    fn score_located_letters_guesser_update_with_unknown_word() -> Result<(), WordleError> {
        let bank = WordBank::from_vec(to_string_vec(vec![
            "alpha", "allot", "begot", "below", "endow", "ingot",
        ]));
        let mut scorer = LocatedLettersScorer::new(&bank, WordCounter::new(&bank.all_words()));

        let restrictions = WordRestrictions::from_result(&GuessResult {
            guess: "other",
            results: vec![
                LetterResult::PresentNotHere,
                LetterResult::NotPresent,
                LetterResult::NotPresent,
                LetterResult::PresentNotHere,
                LetterResult::NotPresent,
            ],
        })?;
        scorer.update(
            "other",
            &restrictions,
            &vec![Rc::from("below"), Rc::from("endow")],
        )?;
        // Remaining possible words: 'below', 'endow'

        assert_eq!(scorer.score_word(&Rc::from("alpha")), 0 + 1 + 0 + 0 + 0);
        assert_eq!(scorer.score_word(&Rc::from("below")), 2 + 1 + 2 + 2 + 4);
        assert_eq!(scorer.score_word(&Rc::from("endow")), 1 + 2 + 2 + 2 + 4);
        assert_eq!(scorer.score_word(&Rc::from("other")), 0 + 0 + 0 + 0 + 0);
        Ok(())
    }

    #[test]
    fn max_elminations_scorer_score_word() {
        let possible_words: Vec<Rc<str>> = vec![Rc::from("cod"), Rc::from("wod"), Rc::from("mod")];
        let word_tracker = WordTracker::new(&possible_words);
        let mut scorer = MaxExpectedEliminationsScorer::new(word_tracker);

        assert_eq!(scorer.score_word(&possible_words[0]), 1333);
        assert_eq!(scorer.score_word(&Rc::from("mwc")), 2000);
        assert_eq!(scorer.score_word(&Rc::from("zzz")), 0);
    }

    #[test]
    fn max_elminations_scorer_score_word_after_update() {
        let possible_words: Vec<Rc<str>> = vec![
            Rc::from("abb"),
            Rc::from("abc"),
            Rc::from("bad"),
            Rc::from("zza"),
            Rc::from("zzz"),
        ];
        let word_tracker = WordTracker::new(&possible_words);
        let mut scorer = MaxExpectedEliminationsScorer::new(word_tracker);

        let restrictions = WordRestrictions::from_result(&GuessResult {
            guess: "zza",
            results: vec![
                LetterResult::NotPresent,
                LetterResult::NotPresent,
                LetterResult::PresentNotHere,
            ],
        })
        .unwrap();
        scorer.update("zza", &restrictions, &Vec::from(&possible_words[0..3]));
        // Still possible: abb, abc, bad

        // Eliminates 2 in all cases.
        assert_eq!(scorer.score_word(&possible_words[0]), 2000);
        // Eliminates 2 in all cases.
        assert_eq!(scorer.score_word(&possible_words[1]), 2000);
        // Could be true in one case (elimnate 2), or false in 2 cases (eliminate 1)
        assert_eq!(scorer.score_word(&possible_words[2]), 1333);
        assert_eq!(scorer.score_word(&Rc::from("zzz")), 0);
    }

    #[test]
    fn max_eliminations_scorer_score_word() {
        let possible_words: Vec<Rc<str>> = vec![Rc::from("cod"), Rc::from("wod"), Rc::from("mod")];
        let tracker = WordTracker::new(&possible_words);
        let mut scorer = MaxEliminationsScorer::new(tracker);

        assert_eq!(scorer.score_word(&possible_words[0]), 1333);
        assert_eq!(scorer.score_word(&Rc::from("mwc")), 2000);
        assert_eq!(scorer.score_word(&Rc::from("zzz")), 0);
    }

    #[test]
    fn max_eliminations_scorer_score_word_after_update() {
        let possible_words: Vec<Rc<str>> = vec![
            Rc::from("abb"),
            Rc::from("abc"),
            Rc::from("bad"),
            Rc::from("zza"),
            Rc::from("zzz"),
        ];
        let tracker = WordTracker::new(&possible_words);
        let mut scorer = MaxEliminationsScorer::new(tracker);

        let restrictions = WordRestrictions::from_result(&GuessResult {
            guess: "zza",
            results: vec![
                LetterResult::NotPresent,
                LetterResult::NotPresent,
                LetterResult::PresentNotHere,
            ],
        })
        .unwrap();
        scorer.update("zza", &restrictions, &Vec::from(&possible_words[0..3]));
        // Still possible: abb, abc, bad

        // Eliminates 2 in all cases.
        assert_eq!(scorer.score_word(&possible_words[0]), 2000);
        // Eliminates 2 in all cases.
        assert_eq!(scorer.score_word(&possible_words[1]), 2000);
        // Could be true in one case (elimnate 2), or false in 2 cases (eliminate 1)
        assert_eq!(scorer.score_word(&possible_words[2]), 1333);
        assert_eq!(scorer.score_word(&Rc::from("zzz")), 0);
    }
}
