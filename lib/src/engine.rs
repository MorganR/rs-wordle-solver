use super::data::*;
use std::collections::HashMap;
use std::collections::HashSet;

/// The result of a given letter at a specific location.
#[derive(Debug, Eq, PartialEq)]
pub enum LetterResult {
    Correct(char),
    PresentNotHere(char),
    NotPresent(char),
}

/// The result of a single word guess.
#[derive(Debug)]
pub struct GuessResult {
    /// The result of each letter, provided in the same leter order as in the guess.
    pub letters: Vec<LetterResult>,
}

/// Whether the game was won or lost by the guesser.
#[derive(Debug, Eq, PartialEq)]
pub enum GameResult {
    /// Indicates that the guesser won the game, and provides the guesses that were given.
    Success(Vec<String>),
    /// Indicates that the guesser failed to guess the word, and provides the guesses that were given.
    Failure(Vec<String>),
    /// Indicates that the given word was not in the word bank.
    UnknownWord,
}

trait GuessSelector {
    fn select_guess<'a, 'b, 'c>(
        &self,
        restrictions: &'a WordRestrictions,
        already_guessed: &'b Vec<String>,
        bank: &'c WordBank,
    ) -> Option<&'c str>;
}

/// Selects the word that maximizes the sum of the frequency of unique letters.
#[derive(Clone, Copy)]
struct MaxUniqueLetterFrequencySelector();

impl GuessSelector for MaxUniqueLetterFrequencySelector {
    fn select_guess<'a, 'b, 'c>(
        &self,
        restrictions: &'a WordRestrictions,
        already_guessed: &'b Vec<String>,
        bank: &'c WordBank,
    ) -> Option<&'c str> {
        let possible_words = get_possible_words(restrictions, bank);

        let count_per_letter = compute_count_per_unique_letter(&possible_words);

        retrieve_word_with_max_letter_frequency(&count_per_letter, already_guessed, &possible_words)
    }
}

fn compute_count_per_unique_letter(words: &Vec<&str>) -> HashMap<char, u32> {
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
    words: &'c Vec<&'d str>,
) -> Option<&'d str> {
    words
        .iter()
        .filter_map(|word| {
            if words_to_avoid.iter().any(|other_word| *word == other_word) {
                return None;
            }
            Some(*word)
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
#[derive(Clone, Copy)]
struct MaxUnguessedUniqueLetterFrequencySelector();

impl GuessSelector for MaxUnguessedUniqueLetterFrequencySelector {
    fn select_guess<'a, 'b, 'c>(
        &self,
        restrictions: &'a WordRestrictions,
        already_guessed: &'b Vec<String>,
        bank: &'c WordBank,
    ) -> Option<&'c str> {
        let possible_words = get_possible_words(restrictions, bank);

        let mut count_per_letter = compute_count_per_unique_letter(&possible_words);
        for word in already_guessed {
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
                already_guessed,
                &bank.all_words(),
            );
        }
        retrieve_word_with_max_letter_frequency(&count_per_letter, already_guessed, &possible_words)
    }
}

/// Defines a Wordle game.
pub struct Game<'a> {
    bank: &'a WordBank,
    restrictions: WordRestrictions,
    guesses: Vec<String>,
}

impl<'a> Game<'a> {
    /// Constructs a new game using the given `WordBank`.
    pub fn new(bank: &'a WordBank) -> Self {
        Self {
            bank: bank,
            restrictions: WordRestrictions::new(),
            guesses: Vec::new(),
        }
    }

    /// Calculates the best guess based on all the guesses so far in this game.
    pub fn calculate_best_guess(&self) -> Option<&str> {
        MaxUniqueLetterFrequencySelector().select_guess(
            &self.restrictions,
            &self.guesses,
            &self.bank,
        )
    }

    /// Updates the game state based on the given information about a guess.
    pub fn update_guess_result(&mut self, result: &GuessResult) {
        let mut guess = String::with_capacity(result.letters.len());
        for (index, lr) in result.letters.iter().enumerate() {
            match lr {
                LetterResult::Correct(letter) => {
                    self.restrictions
                        .must_contain_here
                        .insert(LocatedLetter::new(*letter, index as u8));
                    guess.push(*letter);
                }
                LetterResult::PresentNotHere(letter) => {
                    self.restrictions
                        .must_contain_but_not_here
                        .insert(LocatedLetter::new(*letter, index as u8));
                    guess.push(*letter);
                }
                LetterResult::NotPresent(letter) => {
                    self.restrictions.must_not_contain.insert(*letter);
                    guess.push(*letter);
                }
            }
        }
        self.guesses.push(guess);
    }
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

/// Attempts to guess the given word within the maximum number of guesses, using words from the
/// word bank.
pub fn play_game(word_to_guess: &str, max_num_guesses: u32, word_bank: &WordBank) -> GameResult {
    let mut game = Game::new(word_bank);
    let mut guesses: Vec<String> = Vec::new();
    for _ in 1..=max_num_guesses {
        let maybe_guess = game.calculate_best_guess();
        if maybe_guess.is_none() {
            return GameResult::UnknownWord;
        }
        let guess = maybe_guess.unwrap();
        guesses.push(String::from(guess));
        let result = get_result_for_guess(word_to_guess, guess);

        if result.letters.iter().all(|lr| match lr {
            LetterResult::Correct(_) => true,
            _ => false,
        }) {
            return GameResult::Success(guesses);
        }
        game.update_guess_result(&result);
    }
    return GameResult::Failure(guesses);
}
