use std::collections::hash_set::HashSet;
use std::collections::HashMap;
use std::io::BufRead;
use std::io::Result;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LocatedLetter {
    pub letter: char,
    pub location: u8,
}

impl LocatedLetter {
    pub fn new(letter: char, location: u8) -> LocatedLetter {
        LocatedLetter { letter, location }
    }
}

pub struct WordRestrictions {
    pub must_contain_here: Vec<LocatedLetter>,
    pub must_contain_but_not_here: Vec<LocatedLetter>,
    pub must_not_contain: Vec<char>,
}

impl WordRestrictions {
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

pub struct PossibleWords {
    all_words: Vec<Rc<String>>,
    words_per_located_letter: HashMap<LocatedLetter, HashSet<Rc<String>>>,
}

impl PossibleWords {
    pub fn new<R: BufRead>(word_reader: &mut R) -> Result<Self> {
        let mut located_letters_map = HashMap::new();
        let all_words = word_reader
            .lines()
            .map(|maybe_word| {
                maybe_word.map(|word| {
                    let word_ref = Rc::new(word.to_lowercase());
                    add_to_words_per_located_letter(&word_ref, &mut located_letters_map);
                    word_ref
                })
            })
            .collect::<Result<Vec<Rc<String>>>>()?;
        Ok(Self {
            all_words: all_words,
            words_per_located_letter: located_letters_map,
        })
    }

    pub fn len(&self) -> usize {
        self.all_words.len()
    }

    pub fn by_located_letter(&self, l: &LocatedLetter) -> Option<&HashSet<Rc<String>>> {
        self.words_per_located_letter.get(l)
    }

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

fn add_to_words_per_located_letter(
    word: &Rc<String>,
    words_per_located_letter: &mut HashMap<LocatedLetter, HashSet<Rc<String>>>,
) {
    for (index, letter) in word.char_indices() {
        let located_letter = LocatedLetter::new(letter, index as u8);
        let words = words_per_located_letter
            .entry(located_letter)
            .or_insert_with(HashSet::new);
        words.insert(Rc::clone(word));
    }
}
