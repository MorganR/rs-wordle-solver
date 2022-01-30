use crate::data::*;
use crate::results::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::rc::Rc;

pub trait Guesser {
    fn update<'a, 'b>(&mut self, guess: &'a str, result: &'b GuessResult);

    fn select_next_guess(&self) -> Option<Rc<String>>;
}

/// Attempts to guess the given word within the maximum number of guesses, using words from the
/// word bank.
pub fn play_game(word_to_guess: &str, max_num_guesses: u32, word_bank: &WordBank) -> GameResult {
    let mut guesser = RandomGuesser::new(word_bank);
    let mut guesses: Vec<String> = Vec::new();
    for _ in 1..=max_num_guesses {
        let maybe_guess = guesser.select_next_guess();
        if maybe_guess.is_none() {
            return GameResult::UnknownWord;
        }
        let guess = maybe_guess.unwrap();
        guesses.push((*guess).clone());
        let result = get_result_for_guess(word_to_guess, guess.as_str());

        if result.letters.iter().all(|lr| match lr {
            LetterResult::Correct(_) => true,
            _ => false,
        }) {
            return GameResult::Success(guesses);
        }
        guesser.update(guess.as_str(), &result);
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

/// Guesses at random from the possible words that meet the restrictions.
pub struct RandomGuesser {
    possible_words: Vec<Rc<String>>,
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

    fn select_next_guess(&self) -> Option<Rc<String>> {
        if self.possible_words.is_empty() {
            return None;
        }
        let random: usize = rand::random();
        self.possible_words
            .get(random % self.possible_words.len())
            .map(Rc::clone)
    }
}

/// Selects the word that maximizes the sum of the frequency of unique letters.
pub struct MaxUniqueLetterFrequencyGuesser<'a> {
    bank: &'a WordBank,
    restrictions: WordRestrictions,
    guessed: Vec<String>,
}

impl<'a> MaxUniqueLetterFrequencyGuesser<'a> {
    pub fn new(bank: &'a WordBank) -> MaxUniqueLetterFrequencyGuesser<'a> {
        MaxUniqueLetterFrequencyGuesser {
            bank: bank,
            restrictions: WordRestrictions::new(),
            guessed: Vec::new(),
        }
    }
}

impl<'a> Guesser for MaxUniqueLetterFrequencyGuesser<'a> {
    fn update<'b, 'c>(&mut self, guess: &'b str, result: &'c GuessResult) {
        self.guessed.push(guess.to_string());
        self.restrictions.update(result);
    }

    fn select_next_guess(&self) -> Option<Rc<String>> {
        let possible_words = get_possible_words(&self.restrictions, self.bank);

        let count_per_letter = compute_count_per_unique_letter(&possible_words);

        retrieve_word_with_max_letter_frequency(&count_per_letter, &self.guessed, &possible_words)
    }
}

fn compute_count_per_unique_letter(words: &Vec<Rc<String>>) -> HashMap<char, u32> {
    let mut count_per_letter: HashMap<char, u32> = HashMap::new();
    words.iter().for_each(|word| {
        let unique_chars: HashSet<char> = word.chars().collect();
        unique_chars.iter().for_each(|letter| {
            let count = count_per_letter.entry(*letter).or_insert(0);
            *count += 1;
        });
    });
    count_per_letter
}

fn retrieve_word_with_max_letter_frequency<'a, 'b, 'c, 'd>(
    count_per_letter: &'a HashMap<char, u32>,
    words_to_avoid: &'b Vec<String>,
    words: &Vec<Rc<String>>,
) -> Option<Rc<String>> {
    words
        .iter()
        .filter_map(|word| {
            if words_to_avoid
                .iter()
                .any(|other_word| **word == *other_word)
            {
                return None;
            }
            Some(Rc::clone(word))
        })
        .max_by_key(|word| {
            let unique_chars: HashSet<char> = word.chars().collect();
            unique_chars.iter().fold(0, |sum, letter| {
                sum + *count_per_letter.get(letter).unwrap_or(&0)
            })
        })
}

/// Selects the word that maximizes the sum of the frequency of unique letters that have not yet
/// been guessed.
///
/// This can select words from the whole word bank, not just from the remaining possible words.
pub struct MaxUnguessedUniqueLetterFrequencyGuesser<'a> {
    bank: &'a WordBank,
    restrictions: WordRestrictions,
    guessed: Vec<String>,
}

impl<'a> MaxUnguessedUniqueLetterFrequencyGuesser<'a> {
    pub fn new(bank: &'a WordBank) -> MaxUnguessedUniqueLetterFrequencyGuesser<'a> {
        MaxUnguessedUniqueLetterFrequencyGuesser {
            bank: bank,
            restrictions: WordRestrictions::new(),
            guessed: Vec::new(),
        }
    }
}

impl<'a> Guesser for MaxUnguessedUniqueLetterFrequencyGuesser<'a> {
    fn update<'b, 'c>(&mut self, guess: &'b str, result: &'c GuessResult) {
        self.guessed.push(guess.to_string());
        self.restrictions.update(result);
    }

    fn select_next_guess(&self) -> Option<Rc<String>> {
        let possible_words = get_possible_words(&self.restrictions, self.bank);

        let mut count_per_letter = compute_count_per_unique_letter(&possible_words);
        for word in &self.guessed {
            for letter in word.chars() {
                count_per_letter
                    .entry(letter)
                    .and_modify(|count| *count = 0);
            }
        }
        // Check if there is at least one new letter in the possible words list.
        let may_still_be_unknown_letters = count_per_letter.values().any(|count| *count > 0);

        // If there are still many possible words, choose the next best guess from anywhere,
        // including words that can't be the correct answer. This maximizes the information
        // per guess.
        if possible_words.len() > 2 && may_still_be_unknown_letters {
            return retrieve_word_with_max_letter_frequency(
                &count_per_letter,
                &self.guessed,
                &self.bank.all_words(),
            );
        }
        retrieve_word_with_max_letter_frequency(&count_per_letter, &self.guessed, &possible_words)
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
    all_ungessed_words: Vec<Rc<String>>,
    possible_words: Vec<Rc<String>>,
    counter: WordCounter,
    restrictions: WordRestrictions,
}

impl ScoreLocatedLettersGuesser {
    pub fn new(bank: &WordBank, counter: WordCounter) -> ScoreLocatedLettersGuesser {
        ScoreLocatedLettersGuesser {
            all_ungessed_words: bank.all_words(),
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
            .all_ungessed_words
            .iter()
            .position(|word| word.as_str() == guess)
        {
            self.all_ungessed_words.swap_remove(position);
        }
        // We only need to filter by the current guess's restrictions, since all previous
        // restrictions have already been accounted for.
        // Unless the current guess was the winning guess, this should also filter out the current
        // guess.
        let restriction = WordRestrictions::from_result(result);
        self.possible_words = self
            .possible_words
            .iter()
            .filter_map(|word| {
                if restriction.is_satisfied_by(word.as_str()) {
                    return Some(Rc::clone(word));
                }
                self.counter.remove(word.as_str());
                None
            })
            .collect();
    }

    fn select_next_guess(&self) -> Option<Rc<String>> {
        if self.possible_words.len() > 2 {
            return self
                .all_ungessed_words
                .iter()
                .max_by_key(|word| self.score_word(word.as_str()))
                .map(|word| Rc::clone(word));
        }
        return self
            .possible_words
            .iter()
            .max_by_key(|word| self.score_word(word.as_str()))
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
        let scorer = ScoreLocatedLettersGuesser::new(&bank, WordCounter::new(&bank.all_words()));

        assert_eq!(scorer.select_next_guess().unwrap().as_str(), "begot");
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
        assert_eq!(scorer.select_next_guess().unwrap().as_str(), "endow");
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
        assert!(next_guess.as_str() == "below" || next_guess.as_str() == "endow");
    }
}
