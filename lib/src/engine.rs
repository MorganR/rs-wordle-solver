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
    let mut guesser = MaxUnguessedUniqueLetterFrequencyGuesser::new(word_bank);
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
