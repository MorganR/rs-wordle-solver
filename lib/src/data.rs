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
    pub must_contain_here: Vec<LocatedLetter>,
    /// Letters that must be present, but must be somewhere else in the word.
    pub must_contain_but_not_here: Vec<LocatedLetter>,
    /// Letters that must not be in the word.
    pub must_not_contain: Vec<char>,
}

impl WordRestrictions {
    /// Creates a new empty WordRestrictions struct.
    pub fn new() -> WordRestrictions {
        WordRestrictions {
            must_contain_here: Vec::new(),
            must_contain_but_not_here: Vec::new(),
            must_not_contain: Vec::new(),
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
}

impl WordBank {
    /// Constructs a new `WordBank` struct by reading words from the given reader.
    ///
    /// The reader should provide one word per line. Each word will be converted to lower case.
    pub fn from_reader<R: BufRead>(word_reader: &mut R) -> Result<Self> {
        Ok(WordBank {
            all_words: word_reader
                .lines()
                .map(|maybe_word| maybe_word.map(|word| word.to_lowercase()))
                .collect::<Result<Vec<String>>>()?,
        })
    }

    /// Constructs a new `WordBank` struct using the words from the given vector.
    ///
    /// Each word will be converted to lower case.
    pub fn from_vec(words: Vec<String>) -> Self {
        WordBank {
            all_words: words.iter().map(|word| word.to_lowercase()).collect(),
        }
    }

    /// Returns the number of possible words.
    pub fn len(&self) -> usize {
        self.all_words.len()
    }

    /// Gets the list of possible words that meet the given restrictions.
    pub fn get_possible_words(&self, restrictions: &WordRestrictions) -> Vec<&str> {
        self.all_words
            .iter()
            .filter_map(|word| {
                if restrictions.is_satisfied_by(word) {
                    return Some(word.as_str());
                }
                None
            })
            .collect()
    }
}

#[test]
fn word_bank_get_possible_words_must_contain_here() -> Result<()> {
    let mut cursor = std::io::Cursor::new(String::from("worda\nwordb\nother\nsmore"));

    let word_bank = WordBank::from_reader(&mut cursor)?;

    let still_possible = word_bank.get_possible_words(&WordRestrictions {
        must_contain_here: vec![LocatedLetter::new('o', 1), LocatedLetter::new('b', 4)],
        must_contain_but_not_here: vec![],
        must_not_contain: vec![],
    });

    assert_eq!(still_possible, vec!["wordb"]);
    Ok(())
}

#[test]
fn word_bank_get_possible_words_must_contain_not_here() -> Result<()> {
    let mut cursor = std::io::Cursor::new(String::from("worda\nwordb\nother\nsmore"));

    let word_bank = WordBank::from_reader(&mut cursor)?;

    let still_possible = word_bank.get_possible_words(&WordRestrictions {
        must_contain_here: vec![],
        must_contain_but_not_here: vec![LocatedLetter::new('o', 0)],
        must_not_contain: vec![],
    });

    assert_eq!(still_possible, vec!["worda", "wordb", "smore"]);
    Ok(())
}

#[test]
fn word_bank_get_possible_words_must_not_contain() -> Result<()> {
    let mut cursor = std::io::Cursor::new(String::from("worda\nwordb\nother\nsmore"));

    let word_bank = WordBank::from_reader(&mut cursor)?;

    let still_possible = word_bank.get_possible_words(&WordRestrictions {
        must_contain_here: vec![],
        must_contain_but_not_here: vec![],
        must_not_contain: vec!['w'],
    });

    assert_eq!(still_possible, vec!["other", "smore"]);
    Ok(())
}

#[test]
fn word_bank_get_possible_words_no_match() -> Result<()> {
    let mut cursor = std::io::Cursor::new(String::from("worda\nwordb\nother\nsmore"));

    let word_bank = WordBank::from_reader(&mut cursor)?;

    let still_possible: Vec<&str> = word_bank.get_possible_words(&WordRestrictions {
        must_contain_here: vec![LocatedLetter::new('o', 1)],
        must_contain_but_not_here: vec![LocatedLetter::new('b', 4)],
        must_not_contain: vec!['w'],
    });

    let empty: Vec<&str> = Vec::new();
    assert_eq!(still_possible, empty);
    Ok(())
}
