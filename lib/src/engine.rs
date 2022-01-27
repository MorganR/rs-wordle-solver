use std::collections::HashMap;
use std::collections::HashSet;
use super::data::*;

/// The result of a given letter at a specific location.
#[derive(Debug)]
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

/// Defines a Wordle game.
pub struct Game<'a> {
    bank: &'a WordBank,
    restrictions: WordRestrictions,
}

impl<'a> Game<'a> {
    /// Constructs a new game using the given `WordBank`.
    pub fn new(bank: &'a WordBank) -> Self {
        Self {
            bank: bank,
            restrictions: WordRestrictions::new(),
        }
    }

    /// Calculates the best guess based on all the guesses so far in this game.
    pub fn calculate_best_guess(&self) -> Option<&str> {
        let possible_words = self.bank.get_possible_words(&self.restrictions);

        let mut count_per_letter: HashMap<char, u32> = HashMap::new();
        possible_words.iter().for_each(|word| {
            let unique_chars: HashSet<char> = word.chars().collect();
            unique_chars.iter().for_each(|letter| {
                let count = count_per_letter.entry(*letter).or_insert(0);
                *count += 1;
            });
        });

        possible_words.iter().max_by_key(|word| {
            let unique_chars: HashSet<char> = word.chars().collect();
            unique_chars.iter().fold(0, |sum, letter| sum + count_per_letter.get(letter).unwrap())
        }).map(|word| *word)
    }

    /// Updates the game state based on the given information about a guess.
    pub fn update_guess_result(&mut self, result: &GuessResult) {
        for (index, lr) in result.letters.iter().enumerate() {
            match lr {
                LetterResult::Correct(letter) => self
                    .restrictions
                    .must_contain_here
                    .push(LocatedLetter::new(*letter, index as u8)),
                LetterResult::PresentNotHere(letter) => self
                    .restrictions
                    .must_contain_but_not_here
                    .push(LocatedLetter::new(*letter, index as u8)),
                LetterResult::NotPresent(letter) => {
                    self.restrictions.must_not_contain.push(*letter)
                }
            }
        }
    }
}
