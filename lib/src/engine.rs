use crate::data::*;
use crate::results::*;
use crate::trie::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;

/// Attempts to guess the given word within the maximum number of guesses, using words from the
/// word bank.
pub fn play_game(word_to_guess: &str, max_num_guesses: u32, word_bank: &WordBank) -> GameResult {
    let all_words = word_bank.all_words();
    let scorer = MaxUniqueUnguessedLetterFrequencyScorer::new(&all_words);
    play_game_with_scorer(word_to_guess, max_num_guesses, word_bank, scorer)
}

/// Attempts to guess the given word within the maximum number of guesses, using words from the
/// word bank and using the given word scorer.
pub fn play_game_with_scorer<S: WordScorer>(
    word_to_guess: &str,
    max_num_guesses: u32,
    word_bank: &WordBank,
    word_scorer: S,
) -> GameResult {
    let mut guesser = MaxScoreGuesser::new(
        GuessFrom::AllUnguessedWords,
        word_bank.all_words(),
        word_scorer,
    );
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
        guesser.update(&result);
    }
    return GameResult::Failure(guesses);
}

/// Determines the result of the given `guess` when applied to the given `objective`.
pub fn get_result_for_guess<'a>(objective: &str, guess: &'a str) -> GuessResult<'a> {
    if objective.len() != guess.len() {
        panic!(
            "Objective ({}) must have the same length as the guess ({})",
            objective, guess
        );
    }
    GuessResult {
        guess: guess,
        results: guess
            .char_indices()
            .map(|(index, letter)| {
                if objective.chars().nth(index).unwrap() == letter {
                    return LetterResult::Correct;
                }
                if objective.contains(letter) {
                    return LetterResult::PresentNotHere;
                }
                LetterResult::NotPresent
            })
            .collect(),
    }
}

/// Guesses words in order to solve a single Wordle.
pub trait Guesser {
    /// Updates this guesser with information about a word.
    fn update<'a, 'b>(&mut self, result: &'b GuessResult);

    /// Selects a new guess for the Wordle, if any words are possible.
    fn select_next_guess(&mut self) -> Option<Rc<str>>;
}

/// Guesses at random from the possible words that meet the restrictions.
pub struct RandomGuesser {
    possible_words: Vec<Rc<str>>,
    restrictions: WordRestrictions,
}

impl RandomGuesser {
    pub fn new(bank: &WordBank) -> RandomGuesser {
        RandomGuesser {
            possible_words: bank.all_words(),
            restrictions: WordRestrictions::new(),
        }
    }
}

impl Guesser for RandomGuesser {
    fn update<'a, 'b>(&mut self, result: &'b GuessResult) {
        self.restrictions.update(result);
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
    /// Updates the scorer with the restrictions introduced by the most-recent guess, and with the
    /// updated list of possible words.
    fn update(
        &mut self,
        latest_guess: &str,
        latest_restrictions: &WordRestrictions,
        possible_words: &Vec<Rc<str>>,
    );
    /// Determines a score for the given word.
    fn score_word(&mut self, word: &Rc<str>) -> i64;
}

/// Indicates which set of words to guess from.
#[derive(Debug, PartialEq, Eq)]
pub enum GuessFrom {
    /// Choose the next guess from any unguessed word in the whole word list.
    AllUnguessedWords,
    /// Choose the next guess from any possible word based on the current restrictions.
    PossibleWords,
}

/// Selects the next guess that maximizes the score according to the given scorer.
pub struct MaxScoreGuesser<T>
where
    T: WordScorer,
{
    guess_mode: GuessFrom,
    all_unguessed_words: Vec<Rc<str>>,
    possible_words: Vec<Rc<str>>,
    scorer: T,
}

impl<T> MaxScoreGuesser<T>
where
    T: WordScorer,
{
    pub fn new(guess_mode: GuessFrom, all_words: Vec<Rc<str>>, scorer: T) -> MaxScoreGuesser<T> {
        MaxScoreGuesser {
            guess_mode: guess_mode,
            all_unguessed_words: all_words.clone(),
            possible_words: all_words,
            scorer: scorer,
        }
    }
}

impl<T> Guesser for MaxScoreGuesser<T>
where
    T: WordScorer,
{
    fn update<'b>(&mut self, result: &'b GuessResult) {
        if let Some(position) = self
            .all_unguessed_words
            .iter()
            .position(|word| word.as_ref() == result.guess)
        {
            self.all_unguessed_words.swap_remove(position);
        }
        // We only need to filter by the current guess's restrictions, since all previous
        // restrictions have already been accounted for.
        // Unless the current guess was the winning guess, this also filters out the current guess.
        let restriction = WordRestrictions::from_result(result);
        self.possible_words
            .retain(|word| restriction.is_satisfied_by(word.as_ref()));
        self.scorer
            .update(result.guess, &restriction, &self.possible_words);
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
        _latest_restrictions: &WordRestrictions,
        possible_words: &Vec<Rc<str>>,
    ) {
        self.num_words_per_letter = compute_num_words_per_letter(possible_words);
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
        _latest_restrictions: &WordRestrictions,
        possible_words: &Vec<Rc<str>>,
    ) {
        self.guessed_letters.extend(latest_guess.chars());
        self.num_words_per_letter = compute_num_words_per_letter(possible_words);
        for (letter, count) in self.num_words_per_letter.iter_mut() {
            if self.guessed_letters.contains(letter) {
                *count = 0;
            }
        }
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

/// Scores words by the number of unique words that have the same letter (in any location), summed
/// across each unique and not-yet guessed letter in the word.
pub struct MaxExpectedEliminationsScorer<'a> {
    restrictions: WordRestrictions,
    possible_words: WordTracker<'a>,
    memoized_possibilities: Trie<'a, PossibleOutcomes<'a>>,
}

impl<'a> MaxExpectedEliminationsScorer<'a> {
    pub fn new(possible_words_tracker: WordTracker<'a>) -> MaxExpectedEliminationsScorer<'a> {
        MaxExpectedEliminationsScorer {
            restrictions: WordRestrictions::new(),
            possible_words: possible_words_tracker,
            memoized_possibilities: Trie::new(),
        }
    }

    pub fn from_precomputed(
        possible_words_tracker: WordTracker<'a>,
        first_possibilities_trie: Trie<'a, PossibleOutcomes<'a>>,
    ) -> MaxExpectedEliminationsScorer<'a> {
        MaxExpectedEliminationsScorer {
            restrictions: WordRestrictions::new(),
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
            if self.restrictions.must_not_contain.contains(&letter) {
                continue;
            }
            if self
                .restrictions
                .must_contain_here
                .contains(&LocatedLetter::new(letter, index as u8))
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
        latest_restrictions: &WordRestrictions,
        possible_words: &Vec<Rc<str>>,
    ) {
        self.restrictions.union(latest_restrictions);
        self.possible_words = WordTracker::new(possible_words);
        self.memoized_possibilities = Trie::new();
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

/// Selects the word with a maximum score, computed as follows:
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
///         * 2 points for every possible word with this letter in the same place.
///         * 1 point for every possible word with this letter in another place.
///     
///      * Else:
///
///         * 1 point for every possible word with this letter in the same place.
pub struct ScoreLocatedLettersGuesser {
    all_unguessed_words: Vec<Rc<str>>,
    possible_words: Vec<Rc<str>>,
    counter: WordCounter,
    restrictions: WordRestrictions,
}

impl ScoreLocatedLettersGuesser {
    pub fn new(bank: &WordBank, counter: WordCounter) -> ScoreLocatedLettersGuesser {
        ScoreLocatedLettersGuesser {
            all_unguessed_words: bank.all_words(),
            possible_words: bank.all_words(),
            restrictions: WordRestrictions::new(),
            counter: counter,
        }
    }

    fn score_word(&self, word: &str) -> u32 {
        let mut sum = 0;
        for (index, letter) in word.char_indices() {
            if self
                .restrictions
                .must_contain_here
                .iter()
                .any(|ll| ll.letter == letter && ll.location == index as u8)
            {
                sum += 1;
                continue;
            }
            if self
                // If the letter should be in the word, and this is a new location for it.
                // We don't need to confirm that the locations differ, because those words should
                // have already been removed from the counter.
                .restrictions
                .must_contain_but_not_here
                .iter()
                .chain(self.restrictions.must_contain_here.iter())
                .any(|ll| ll.letter == letter)
                // Or if this letter has already been scored once
                || (index > 0
                && word
                    .chars()
                    .take(index)
                    .any(|other_letter| other_letter == letter))
            {
                sum += self
                    .counter
                    .num_words_with_located_letter(&LocatedLetter::new(letter, index as u8));
                continue;
            }
            // We don't need to check for letters that must not be in the word, because they should
            // have already been removed from the counter.

            // This is a letter that we know nothing about:
            sum += self
                .counter
                .num_words_with_located_letter(&LocatedLetter::new(letter, index as u8))
                + self.counter.num_words_with_letter(letter);
        }
        sum
    }
}

impl Guesser for ScoreLocatedLettersGuesser {
    fn update<'a>(&mut self, result: &'a GuessResult) {
        self.restrictions.update(result);
        if let Some(position) = self
            .all_unguessed_words
            .iter()
            .position(|word| word.as_ref() == result.guess)
        {
            self.all_unguessed_words.swap_remove(position);
        }
        // We only need to filter by the current guess's restrictions, since all previous
        // restrictions have already been accounted for.
        // Unless the current guess was the winning guess, this also filters out the current guess.
        let restriction = WordRestrictions::from_result(result);
        self.possible_words = self
            .possible_words
            .iter()
            .filter_map(|word| {
                if restriction.is_satisfied_by(word) {
                    return Some(Rc::clone(word));
                }
                self.counter.remove(word.as_ref());
                None
            })
            .collect();
    }

    fn select_next_guess(&mut self) -> Option<Rc<str>> {
        if self.possible_words.len() > 2 {
            return self
                .all_unguessed_words
                .iter()
                .max_by_key(|word| self.score_word(word.as_ref()))
                .map(|word| Rc::clone(word));
        }
        return self
            .possible_words
            .iter()
            .max_by_key(|word| self.score_word(word.as_ref()))
            .map(|word| Rc::clone(word));
    }
}

/// Chooses the word that is expected to eliminate the most other words.
pub struct MostEliminationsGuesser {
    all_unguessed_words: Vec<Rc<str>>,
    possible_words: Vec<Rc<str>>,
    counter: WordCounter,
}

impl MostEliminationsGuesser {
    pub fn new(bank: &WordBank, counter: WordCounter) -> MostEliminationsGuesser {
        MostEliminationsGuesser {
            all_unguessed_words: bank.all_words(),
            possible_words: bank.all_words(),
            counter: counter,
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
        let num_if_present_not_here =
            self.counter.num_words_with_letter(located_letter.letter) as f64 - num_if_correct;
        let total = self.possible_words.len() as f64;
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
        let num_if_not_present = total - num_if_correct - num_if_present_not_here;
        let eliminations_if_not_present = total - num_if_not_present;
        return expected_eliminations_for_present_somewhere
            + eliminations_if_not_present * num_if_not_present / total;
    }
}

impl Guesser for MostEliminationsGuesser {
    fn update<'a>(&mut self, result: &'a GuessResult) {
        if let Some(position) = self
            .all_unguessed_words
            .iter()
            .position(|word| word.as_ref() == result.guess)
        {
            self.all_unguessed_words.swap_remove(position);
        }
        // We only need to filter by the current guess's restrictions, since all previous
        // restrictions have already been accounted for.
        // Unless the current guess was the winning guess, this also filters out the current guess.
        let restriction = WordRestrictions::from_result(result);
        self.possible_words = self
            .possible_words
            .iter()
            .filter_map(|word| {
                if restriction.is_satisfied_by(word.as_ref()) {
                    return Some(Rc::clone(word));
                }
                self.counter.remove(word.as_ref());
                None
            })
            .collect();
    }

    fn select_next_guess(&mut self) -> Option<Rc<str>> {
        if self.possible_words.len() > 2 {
            return self
                .all_unguessed_words
                .iter()
                .max_by_key(|word| {
                    // f64 doesn't implement `Ord`, so multiple by 100 to keep some precision then
                    // convert to i64.
                    (self.compute_expected_eliminations(word.as_ref()) * 100.0) as i64
                })
                .map(|word| Rc::clone(word));
        }
        return self
            .possible_words
            .iter()
            .max_by_key(|word| (self.compute_expected_eliminations(word.as_ref()) * 100.0) as i64)
            .map(|word| Rc::clone(word));
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn to_string_vec(words: Vec<&str>) -> Vec<String> {
        words.iter().map(|word| word.to_string()).collect()
    }

    #[test]
    fn score_located_letters_guesser_score_word() {
        let bank = WordBank::from_vec(to_string_vec(vec![
            "alpha", "allot", "begot", "below", "endow", "ingot",
        ]));
        let scorer = ScoreLocatedLettersGuesser::new(&bank, WordCounter::new(&bank.all_words()));

        assert_eq!(scorer.score_word("alpha"), 4 + 5 + 2 + 2 + 1);
        assert_eq!(scorer.score_word("allot"), 4 + 5 + 2 + 10 + 6);
        assert_eq!(scorer.score_word("begot"), 4 + 5 + 4 + 10 + 6);
        assert_eq!(scorer.score_word("below"), 4 + 5 + 5 + 10 + 4);
        assert_eq!(scorer.score_word("endow"), 4 + 4 + 2 + 10 + 4);
        assert_eq!(scorer.score_word("ingot"), 2 + 4 + 4 + 10 + 6);
        assert_eq!(scorer.score_word("other"), 5 + 3 + 1 + 3 + 0);
    }

    #[test]
    fn score_located_letters_guesser_select_next_guess() {
        let bank = WordBank::from_vec(to_string_vec(vec![
            "alpha", "allot", "begot", "below", "endow", "ingot",
        ]));
        let mut scorer =
            ScoreLocatedLettersGuesser::new(&bank, WordCounter::new(&bank.all_words()));

        assert_eq!(scorer.select_next_guess().unwrap().as_ref(), "begot");
    }

    #[test]
    fn score_located_letters_guesser_update() {
        let bank = WordBank::from_vec(to_string_vec(vec![
            "alpha", "allot", "begot", "below", "endow", "ingot",
        ]));
        let mut scorer =
            ScoreLocatedLettersGuesser::new(&bank, WordCounter::new(&bank.all_words()));

        scorer.update(&GuessResult {
            guess: "begot",
            results: vec![
                LetterResult::NotPresent,
                LetterResult::PresentNotHere,
                LetterResult::NotPresent,
                LetterResult::Correct,
                LetterResult::NotPresent,
            ],
        });
        // Remaining possible words: 'endow'

        assert_eq!(scorer.score_word("alpha"), 0 + 0 + 0 + 0 + 0);
        assert_eq!(scorer.score_word("below"), 0 + 0 + 0 + 1 + 2);
        assert_eq!(scorer.score_word("endow"), 1 + 2 + 2 + 1 + 2);
        assert_eq!(scorer.score_word("other"), 0 + 0 + 0 + 0 + 0);
        assert_eq!(scorer.select_next_guess().unwrap().as_ref(), "endow");
    }

    #[test]
    fn score_located_letters_guesser_update_with_unknown_word() {
        let bank = WordBank::from_vec(to_string_vec(vec![
            "alpha", "allot", "begot", "below", "endow", "ingot",
        ]));
        let mut scorer =
            ScoreLocatedLettersGuesser::new(&bank, WordCounter::new(&bank.all_words()));

        scorer.update(&GuessResult {
            guess: "other",
            results: vec![
                LetterResult::PresentNotHere,
                LetterResult::NotPresent,
                LetterResult::NotPresent,
                LetterResult::PresentNotHere,
                LetterResult::NotPresent,
            ],
        });
        // Remaining possible words: 'below', 'endow'

        assert_eq!(scorer.score_word("alpha"), 0 + 1 + 0 + 0 + 0);
        assert_eq!(scorer.score_word("below"), 2 + 1 + 2 + 2 + 4);
        assert_eq!(scorer.score_word("endow"), 1 + 2 + 2 + 2 + 4);
        assert_eq!(scorer.score_word("other"), 0 + 0 + 0 + 0 + 0);

        let next_guess = scorer.select_next_guess().unwrap();
        assert!(next_guess.as_ref() == "below" || next_guess.as_ref() == "endow");
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
        });
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
