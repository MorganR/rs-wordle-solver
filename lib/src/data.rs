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
pub struct PossibleWords {
    all_words: Vec<String>,
}

impl PossibleWords {
    /// Constructs a new `PossibleWords` struct by reading words from the given reader.
    /// 
    /// The reader should provide one word per line. Each word will be converted to lower case.
    pub fn new<R: BufRead>(word_reader: &mut R) -> Result<Self> {
        Ok(Self {
            all_words: word_reader
                .lines()
                .map(|maybe_word| maybe_word.map(|word| word.to_lowercase()))
                .collect::<Result<Vec<String>>>()?,
        })
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
