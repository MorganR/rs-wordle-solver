use crate::results::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::io::BufRead;
use std::io::Result;
use std::rc::Rc;

/// A letter along with its location in the word.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LocatedLetter {
    pub letter: char,
    /// The zero-based location (i.e. index) for this letter in a word.
    pub location: u8,
}

impl LocatedLetter {
    pub fn new(letter: char, location: u8) -> LocatedLetter {
        LocatedLetter { letter, location }
    }
}

/// Defines letter restrictions that a word must adhere to.
pub struct WordRestrictions {
    /// Letters that must occur in specific locations in the word.
    pub must_contain_here: HashSet<LocatedLetter>,
    /// Letters that must be present, but must be somewhere else in the word.
    pub must_contain_but_not_here: HashSet<LocatedLetter>,
    /// Letters that must not be in the word.
    pub must_not_contain: HashSet<char>,
}

impl WordRestrictions {
    /// Creates a new empty WordRestrictions struct.
    pub fn new() -> WordRestrictions {
        WordRestrictions {
            must_contain_here: HashSet::new(),
            must_contain_but_not_here: HashSet::new(),
            must_not_contain: HashSet::new(),
        }
    }

    /// Returns the restrictions imposed by the given result.
    pub fn from_result(result: &GuessResult) -> WordRestrictions {
        let mut restrictions = WordRestrictions::new();
        restrictions.update(result);
        restrictions
    }

    /// Adds restrictions arising from the given guess result.
    pub fn update(&mut self, guess_result: &GuessResult) {
        for (index, lr) in guess_result.letters.iter().enumerate() {
            match lr {
                LetterResult::Correct(letter) => {
                    self.must_contain_here
                        .insert(LocatedLetter::new(*letter, index as u8));
                }
                LetterResult::PresentNotHere(letter) => {
                    self.must_contain_but_not_here
                        .insert(LocatedLetter::new(*letter, index as u8));
                }
                LetterResult::NotPresent(letter) => {
                    self.must_not_contain.insert(*letter);
                }
            }
        }
    }

    /// Returns `true` iff the given word satisfies these restrictions.
    pub fn is_satisfied_by(&self, word: &str) -> bool {
        self.must_contain_here
            .iter()
            .all(|ll| word.chars().nth(ll.location as usize) == Some(ll.letter))
            && self.must_contain_but_not_here.iter().all(|ll| {
                word.chars().nth(ll.location as usize) != Some(ll.letter)
                    && word.contains(ll.letter)
            })
            && !self
                .must_not_contain
                .iter()
                .any(|letter| word.contains(*letter))
    }
}

/// Contains all the possible words for this Wordle game.
pub struct WordBank {
    all_words: Vec<Rc<String>>,
    max_word_length: usize,
}

impl WordBank {
    /// Constructs a new `WordBank` struct by reading words from the given reader.
    ///
    /// The reader should provide one word per line. Each word will be converted to lower case.
    pub fn from_reader<R: BufRead>(word_reader: &mut R) -> Result<Self> {
        let mut max_word_length = 0;
        Ok(WordBank {
            all_words: word_reader
                .lines()
                .map(|maybe_word| {
                    maybe_word.map(|word| {
                        let word_length = word.len();
                        if max_word_length < word_length {
                            max_word_length = word_length;
                        }
                        Rc::new(word.to_lowercase())
                    })
                })
                .filter(|maybe_word| maybe_word.as_ref().map_or(true, |word| word.len() > 0))
                .collect::<Result<Vec<Rc<String>>>>()?,
            max_word_length: max_word_length,
        })
    }

    /// Constructs a new `WordBank` struct using the words from the given vector.
    ///
    /// Each word will be converted to lower case.
    pub fn from_vec(words: Vec<String>) -> Self {
        let mut max_word_length = 0;
        WordBank {
            all_words: words
                .iter()
                .filter_map(|word| {
                    let word_length = word.len();
                    if word_length == 0 {
                        return None;
                    }
                    if max_word_length < word_length {
                        max_word_length = word_length;
                    }
                    Some(Rc::new(word.to_lowercase()))
                })
                .collect(),
            max_word_length: max_word_length,
        }
    }

    /// Retrieves the full list of available words.
    pub fn all_words(&self) -> Vec<Rc<String>> {
        self.all_words.iter().map(|word| Rc::clone(word)).collect()
    }

    /// Returns the number of possible words.
    pub fn len(&self) -> usize {
        self.all_words.len()
    }

    /// Returns the length of the longest word in the bank.
    pub fn max_word_len(&self) -> usize {
        self.max_word_length
    }
}

/// Counts the number of words that have letters in certain locations.
#[derive(Clone)]
pub struct WordCounter {
    num_words_by_ll: HashMap<LocatedLetter, u32>,
    num_words_by_letter: HashMap<char, u32>,
}

impl WordCounter {
    /// Creates a new word counter based on the given word list.
    pub fn new(words: &Vec<Rc<String>>) -> WordCounter {
        let mut num_words_by_ll: HashMap<LocatedLetter, u32> = HashMap::new();
        let mut num_words_by_letter: HashMap<char, u32> = HashMap::new();
        for word in words {
            for (index, letter) in word.char_indices() {
                *num_words_by_ll
                    .entry(LocatedLetter::new(letter, index as u8))
                    .or_insert(0) += 1;
                if index == 0
                    || word
                        .chars()
                        .take(index)
                        .all(|other_letter| other_letter != letter)
                {
                    *num_words_by_letter.entry(letter).or_insert(0) += 1;
                }
            }
        }
        WordCounter {
            num_words_by_ll: num_words_by_ll,
            num_words_by_letter: num_words_by_letter,
        }
    }

    /// Removes the given word from the counter.
    pub fn remove(&mut self, word: &str) {
        for (index, letter) in word.char_indices() {
            self.num_words_by_ll
                .entry(LocatedLetter::new(letter, index as u8))
                .and_modify(|num_words| *num_words -= 1);
            if index == 0
                || word
                    .chars()
                    .take(index)
                    .all(|other_letter| other_letter != letter)
            {
                self.num_words_by_letter
                    .entry(letter)
                    .and_modify(|num_words| *num_words -= 1);
            }
        }
    }

    /// Retrieves the count of words with the given letter at the given location.
    pub fn num_words_with_located_letter(&self, ll: &LocatedLetter) -> u32 {
        *self.num_words_by_ll.get(ll).unwrap_or(&0)
    }

    /// Retrieves the count of words that contain the given letter.
    pub fn num_words_with_letter(&self, letter: char) -> u32 {
        *self.num_words_by_letter.get(&letter).unwrap_or(&0)
    }
}

/// Gets the list of possible words in the word bank that meet the given restrictions.
pub fn get_possible_words(restrictions: &WordRestrictions, bank: &WordBank) -> Vec<Rc<String>> {
    bank.all_words
        .iter()
        .filter_map(|word| {
            if restrictions.is_satisfied_by(word) {
                return Some(Rc::clone(word));
            }
            None
        })
        .collect()
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::io::Cursor;

    macro_rules! assert_rc_eq {
        ($rc_vec:expr, $non_rc_vec:expr, $rc_type:ty) => {
            let copy: Vec<$rc_type> = $rc_vec.iter().map(|thing| (**thing).clone()).collect();
            assert_eq!(copy, $non_rc_vec);
        };
    }

    #[test]
    fn word_bank_get_possible_words_must_contain_here() -> Result<()> {
        let mut cursor = Cursor::new(String::from("worda\nwordb\nother\nsmore"));

        let word_bank = WordBank::from_reader(&mut cursor)?;

        let still_possible = get_possible_words(
            &WordRestrictions {
                must_contain_here: HashSet::from([
                    LocatedLetter::new('o', 1),
                    LocatedLetter::new('b', 4),
                ]),
                must_contain_but_not_here: HashSet::new(),
                must_not_contain: HashSet::new(),
            },
            &word_bank,
        );

        assert_rc_eq!(still_possible, vec!["wordb"], String);
        Ok(())
    }

    #[test]
    fn word_bank_get_possible_words_must_contain_not_here() -> Result<()> {
        let mut cursor = Cursor::new(String::from("worda\nwordb\nother\nsmore"));

        let word_bank = WordBank::from_reader(&mut cursor)?;

        let still_possible = get_possible_words(
            &WordRestrictions {
                must_contain_here: HashSet::new(),
                must_contain_but_not_here: HashSet::from([LocatedLetter::new('o', 0)]),
                must_not_contain: HashSet::new(),
            },
            &word_bank,
        );

        assert_rc_eq!(still_possible, vec!["worda", "wordb", "smore"], String);
        Ok(())
    }

    #[test]
    fn word_bank_get_possible_words_must_not_contain() -> Result<()> {
        let mut cursor = Cursor::new(String::from("worda\nwordb\nother\nsmore"));

        let word_bank = WordBank::from_reader(&mut cursor)?;

        let still_possible = get_possible_words(
            &WordRestrictions {
                must_contain_here: HashSet::new(),
                must_contain_but_not_here: HashSet::new(),
                must_not_contain: HashSet::from(['w']),
            },
            &word_bank,
        );

        assert_rc_eq!(still_possible, vec!["other", "smore"], String);
        Ok(())
    }

    #[test]
    fn word_bank_get_possible_words_no_match() -> Result<()> {
        let mut cursor = Cursor::new(String::from("worda\nwordb\nother\nsmore"));

        let word_bank = WordBank::from_reader(&mut cursor)?;

        let still_possible = get_possible_words(
            &WordRestrictions {
                must_contain_here: HashSet::from([LocatedLetter::new('o', 1)]),
                must_contain_but_not_here: HashSet::from([LocatedLetter::new('b', 4)]),
                must_not_contain: HashSet::from(['w']),
            },
            &word_bank,
        );

        assert!(still_possible.is_empty());
        Ok(())
    }

    #[test]
    fn word_counter_num_words_with_located_letter() {
        let counter = WordCounter::new(&rc_string_vec(vec!["hello", "hallo", "worda"]));

        assert_eq!(
            counter.num_words_with_located_letter(&LocatedLetter::new('h', 0)),
            2
        );
        assert_eq!(
            counter.num_words_with_located_letter(&LocatedLetter::new('e', 1)),
            1
        );
        assert_eq!(
            counter.num_words_with_located_letter(&LocatedLetter::new('l', 2)),
            2
        );
        assert_eq!(
            counter.num_words_with_located_letter(&LocatedLetter::new('l', 3)),
            2
        );
        assert_eq!(
            counter.num_words_with_located_letter(&LocatedLetter::new('o', 4)),
            2
        );
        assert_eq!(
            counter.num_words_with_located_letter(&LocatedLetter::new('a', 1)),
            1
        );
        assert_eq!(
            counter.num_words_with_located_letter(&LocatedLetter::new('w', 0)),
            1
        );
        assert_eq!(
            counter.num_words_with_located_letter(&LocatedLetter::new('o', 1)),
            1
        );
        assert_eq!(
            counter.num_words_with_located_letter(&LocatedLetter::new('r', 2)),
            1
        );
        assert_eq!(
            counter.num_words_with_located_letter(&LocatedLetter::new('d', 3)),
            1
        );
        assert_eq!(
            counter.num_words_with_located_letter(&LocatedLetter::new('a', 4)),
            1
        );

        // Missing letters:
        assert_eq!(
            counter.num_words_with_located_letter(&LocatedLetter::new('h', 1)),
            0
        );
        assert_eq!(
            counter.num_words_with_located_letter(&LocatedLetter::new('z', 0)),
            0
        );
    }

    #[test]
    fn word_counter_num_words_with_letter() {
        let counter = WordCounter::new(&rc_string_vec(vec!["hello", "hallo", "worda"]));

        assert_eq!(counter.num_words_with_letter('h'), 2);
        assert_eq!(counter.num_words_with_letter('e'), 1);
        assert_eq!(counter.num_words_with_letter('l'), 2);
        assert_eq!(counter.num_words_with_letter('o'), 3);
        assert_eq!(counter.num_words_with_letter('a'), 2);
        assert_eq!(counter.num_words_with_letter('w'), 1);
        assert_eq!(counter.num_words_with_letter('r'), 1);
        assert_eq!(counter.num_words_with_letter('d'), 1);

        // Missing letters:
        assert_eq!(counter.num_words_with_letter('z'), 0);
    }

    #[test]
    fn word_counter_remove() {
        let mut counter = WordCounter::new(&rc_string_vec(vec!["hello", "hallo", "worda"]));

        counter.remove("hallo");

        assert_eq!(
            counter.num_words_with_located_letter(&LocatedLetter::new('h', 0)),
            1
        );
        assert_eq!(
            counter.num_words_with_located_letter(&LocatedLetter::new('a', 1)),
            0
        );
        assert_eq!(
            counter.num_words_with_located_letter(&LocatedLetter::new('l', 2)),
            1
        );
        assert_eq!(
            counter.num_words_with_located_letter(&LocatedLetter::new('l', 3)),
            1
        );
        assert_eq!(
            counter.num_words_with_located_letter(&LocatedLetter::new('o', 4)),
            1
        );
        assert_eq!(counter.num_words_with_letter('h'), 1);
        assert_eq!(counter.num_words_with_letter('l'), 1);
        assert_eq!(counter.num_words_with_letter('o'), 2);
        assert_eq!(counter.num_words_with_letter('a'), 1);
    }

    fn rc_string_vec(vec_str: Vec<&'static str>) -> Vec<Rc<String>> {
        vec_str
            .iter()
            .map(|word| Rc::new(word.to_string()))
            .collect()
    }
}
