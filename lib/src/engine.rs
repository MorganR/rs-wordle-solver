use crate::data::*;
use crate::results::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;

/// Attempts to guess the given word within the maximum number of guesses, using words from the
/// word bank.
pub fn play_game(word_to_guess: &str, max_num_guesses: u32, word_bank: &WordBank) -> GameResult {
    let all_words = word_bank.all_words();
    let scorer = MaxUniqueUnguessedLetterFrequencyScorer::new(&all_words);
    let mut guesser = MaxScoreGuesser::new(GuessFrom::AllUnguessedWords, all_words, scorer);
    let mut guesses: Vec<Box<str>> = Vec::new();
    for _ in 1..=max_num_guesses {
        let maybe_guess = guesser.select_next_guess();
        if maybe_guess.is_none() {
            return GameResult::UnknownWord;
        }
        let guess = maybe_guess.unwrap();
        guesses.push(Box::from(guess.as_ref()));
        let result = get_result_for_guess(word_to_guess, guess.as_ref());

        if result.letters.iter().all(|lr| match lr {
            LetterResult::Correct(_) => true,
            _ => false,
        }) {
            return GameResult::Success(guesses);
        }
        guesser.update(guess.as_ref(), &result);
    }
    return GameResult::Failure(guesses);
}

/// Determines the result of the given `guess` when applied to the given `objective`.
pub fn get_result_for_guess(objective: &str, guess: &str) -> GuessResult {
    if objective.len() != guess.len() {
        panic!(
            "Objective ({}) must have the same length as the guess ({})",
            objective, guess
        );
    }
    GuessResult {
        letters: guess
            .char_indices()
            .map(|(index, letter)| {
                if objective.chars().nth(index).unwrap() == letter {
                    return LetterResult::Correct(letter);
                }
                if objective.contains(letter) {
                    return LetterResult::PresentNotHere(letter);
                }
                LetterResult::NotPresent(letter)
            })
            .collect(),
    }
}

/// Guesses words in order to solve a single Wordle.
pub trait Guesser {
    /// Updates this guesser with information about a word.
    fn update<'a, 'b>(&mut self, guess: &'a str, result: &'b GuessResult);

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
    fn update<'a, 'b>(&mut self, _guess: &'a str, result: &'b GuessResult) {
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
    fn score_word(&mut self, word: &str) -> i64;
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
    fn update<'a, 'b>(&mut self, guess: &'a str, result: &'b GuessResult) {
        if let Some(position) = self
            .all_unguessed_words
            .iter()
            .position(|word| word.as_ref() == guess)
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
                None
            })
            .collect();
        self.scorer
            .update(guess, &restriction, &self.possible_words);
    }

    fn select_next_guess(&mut self) -> Option<Rc<str>> {
        if self.guess_mode == GuessFrom::AllUnguessedWords {
            if self.possible_words.len() > 2 {
                let mut best_word = self.all_unguessed_words.get(0);
                let mut best_score =
                    best_word.map_or(0, |word| self.scorer.score_word(word.as_ref()));
                let mut scores_all_same = true;
                for word in self.all_unguessed_words.iter() {
                    let score = self.scorer.score_word(word.as_ref());
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
            .max_by_key(|word| self.scorer.score_word(word.as_ref()))
            .map(Rc::clone);
    }
}

/// Scores words by the number of unique words that have the same letter (in any location), summed
/// across each unique letter in the word.
pub struct MaxUniqueLetterFrequencyScorer {
    num_words_per_letter: HashMap<char, u32>,
}

fn compute_num_words_per_letter(words: &Vec<Rc<str>>) -> HashMap<char, u32> {
    let mut num_words_per_letter: HashMap<char, u32> = HashMap::new();
    words.iter().for_each(|word| {
        let unique_chars: HashSet<char> = word.chars().collect();
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

    fn score_word(&mut self, word: &str) -> i64 {
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
    pub fn new(possible_words: &Vec<Rc<str>>) -> MaxUniqueUnguessedLetterFrequencyScorer {
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

    fn score_word(&mut self, word: &str) -> i64 {
        let unique_chars: HashSet<char> = word.chars().collect();
        return unique_chars.iter().fold(0 as i64, |sum, letter| {
            sum + *self.num_words_per_letter.get(letter).unwrap_or(&0) as i64
        });
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
    fn update<'a, 'b>(&mut self, guess: &'a str, result: &'b GuessResult) {
        self.restrictions.update(result);
        if let Some(position) = self
            .all_unguessed_words
            .iter()
            .position(|word| word.as_ref() == guess)
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
    fn update<'a, 'b>(&mut self, guess: &'a str, result: &'b GuessResult) {
        if let Some(position) = self
            .all_unguessed_words
            .iter()
            .position(|word| word.as_ref() == guess)
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

        scorer.update(
            "begot",
            &GuessResult {
                letters: vec![
                    LetterResult::NotPresent('b'),
                    LetterResult::PresentNotHere('e'),
                    LetterResult::NotPresent('g'),
                    LetterResult::Correct('o'),
                    LetterResult::NotPresent('t'),
                ],
            },
        );
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

        scorer.update(
            "other",
            &GuessResult {
                letters: vec![
                    LetterResult::PresentNotHere('o'),
                    LetterResult::NotPresent('t'),
                    LetterResult::NotPresent('h'),
                    LetterResult::PresentNotHere('e'),
                    LetterResult::NotPresent('r'),
                ],
            },
        );
        // Remaining possible words: 'below', 'endow'

        assert_eq!(scorer.score_word("alpha"), 0 + 1 + 0 + 0 + 0);
        assert_eq!(scorer.score_word("below"), 2 + 1 + 2 + 2 + 4);
        assert_eq!(scorer.score_word("endow"), 1 + 2 + 2 + 2 + 4);
        assert_eq!(scorer.score_word("other"), 0 + 0 + 0 + 0 + 0);

        let next_guess = scorer.select_next_guess().unwrap();
        assert!(next_guess.as_ref() == "below" || next_guess.as_ref() == "endow");
    }
}
