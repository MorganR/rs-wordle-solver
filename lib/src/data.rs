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
                    let word_ref = Rc::new(word);
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
}

fn add_to_words_per_located_letter(
    word: &Rc<String>,
    words_per_located_letter: &mut HashMap<LocatedLetter, HashSet<Rc<String>>>,
) -> () {
    for (index, letter) in word.char_indices() {
        let located_letter = LocatedLetter::new(letter, index as u8);
        let words = words_per_located_letter
            .entry(located_letter)
            .or_insert_with(HashSet::new);
        words.insert(Rc::clone(word));
    }
}
