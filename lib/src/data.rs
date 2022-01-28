use std::collections::HashSet;
use std::io::BufRead;
use std::io::Result;

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

    /// Returns `true` iff the given word satisfies these restrictions.
    fn is_satisfied_by(&self, word: &str) -> bool {
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
    all_words: Vec<String>,
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
                        word.to_lowercase()
                    })
                })
                .collect::<Result<Vec<String>>>()?,
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
                .map(|word| {
                    let word_length = word.len();
                    if max_word_length < word_length {
                        max_word_length = word_length;
                    }
                    word.to_lowercase()
                })
                .collect(),
            max_word_length: max_word_length,
        }
    }

    /// Retrieves the full list of available words.
    pub fn all_words(&self) -> Vec<&str> {
        self.all_words.iter().map(AsRef::as_ref).collect()
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

/// Gets the list of possible words in the word bank that meet the given restrictions.
pub fn get_possible_words<'a, 'b>(
    restrictions: &'a WordRestrictions,
    bank: &'b WordBank,
) -> Vec<&'b str> {
    bank.all_words
        .iter()
        .filter_map(|word| {
            if restrictions.is_satisfied_by(word) {
                return Some(word.as_str());
            }
            None
        })
        .collect()
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::io::Cursor;

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

        assert_eq!(still_possible, vec!["wordb"]);
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

        assert_eq!(still_possible, vec!["worda", "wordb", "smore"]);
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

        assert_eq!(still_possible, vec!["other", "smore"]);
        Ok(())
    }

    #[test]
    fn word_bank_get_possible_words_no_match() -> Result<()> {
        let mut cursor = Cursor::new(String::from("worda\nwordb\nother\nsmore"));

        let word_bank = WordBank::from_reader(&mut cursor)?;

        let still_possible: Vec<&str> = get_possible_words(
            &WordRestrictions {
                must_contain_here: HashSet::from([LocatedLetter::new('o', 1)]),
                must_contain_but_not_here: HashSet::from([LocatedLetter::new('b', 4)]),
                must_not_contain: HashSet::from(['w']),
            },
            &word_bank,
        );

        let empty: Vec<&str> = Vec::new();
        assert_eq!(still_possible, empty);
        Ok(())
    }
}
