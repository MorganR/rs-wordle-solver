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
    let word_counter = WordCounter::new(&word_bank.all_words());
    let mut guesser = ScoreLocatedLettersGuesser::new(word_bank, word_counter);
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
        panic!("Objective ({}) must have the same length as the guess ({})", objective, guess); 
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
///   * 1 point if the letter's location is already known.
///   * 1 point for every word with this letter in this place if the letter's location is not yet
///     known, and this is a new location for the letter.
///   * If this letter is completely new:
///
///      * 2 points for every possible word with this letter in the same place.
///      * 1 point for every possible word with this letter in another place.
pub struct ScoreLocatedLettersGuesser {
    possible_words: Vec<Rc<String>>,
    counter: WordCounter,
    restrictions: WordRestrictions,
}

impl ScoreLocatedLettersGuesser {
    pub fn new(bank: &WordBank, counter: WordCounter) -> ScoreLocatedLettersGuesser {
        ScoreLocatedLettersGuesser {
            possible_words: bank.all_words(),
            restrictions: WordRestrictions::new(),
            counter: counter,
        }
    }

    fn score_word(&self, word: &str) -> u32 {
        let mut sum = 0;
        for (index, letter) in word.char_indices() {
            if index > 0 && word.chars().take(index).any(|other_letter| other_letter == letter) {
                continue;
            }
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
                .restrictions
                .must_contain_but_not_here
                .iter()
                .chain(self.restrictions.must_contain_here.iter())
                .any(|ll| ll.letter == letter)
                // We don't need to confirm that the locations differ, because those words should
                // have already been removed from the possible words.
            {
                sum += self
                    .counter
                    .num_words_with_located_letter(&LocatedLetter::new(letter, index as u8));
                continue;
            }
            // We don't need to check for letters that must not be in the word, because they should
            // have already been removed from the possible words.

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
    fn update<'a, 'b>(&mut self, _guess: &'a str, result: &'b GuessResult) {
        self.restrictions.update(result);
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
        self.possible_words
            .iter()
            .max_by_key(|word| self.score_word(word.as_str()))
            .map(|word| Rc::clone(word))
    }
}
