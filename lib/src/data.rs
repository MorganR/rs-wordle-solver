use crate::results::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::Hash;
use std::hash::Hasher;
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
    all_words: Vec<Rc<str>>,
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
                        Rc::from(word.to_lowercase().as_str())
                    })
                })
                .filter(|maybe_word| {
                    maybe_word
                        .as_ref()
                        .map_or(true, |word: &Rc<str>| word.len() > 0)
                })
                .collect::<Result<Vec<Rc<str>>>>()?,
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
                    Some(Rc::from(word.to_lowercase().as_str()))
                })
                .collect(),
            max_word_length: max_word_length,
        }
    }

    /// Retrieves the full list of available words.
    pub fn all_words(&self) -> Vec<Rc<str>> {
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

/// Gets the list of possible words in the word bank that meet the given restrictions.
pub fn get_possible_words(restrictions: &WordRestrictions, bank: &WordBank) -> Vec<Rc<str>> {
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

/// Counts the number of words that have letters in certain locations.
#[derive(Clone)]
pub struct WordCounter {
    num_words_by_ll: HashMap<LocatedLetter, u32>,
    num_words_by_letter: HashMap<char, u32>,
}

impl WordCounter {
    /// Creates a new word counter based on the given word list.
    pub fn new(words: &Vec<Rc<str>>) -> WordCounter {
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

#[derive(Debug)]
pub struct RefPtrEq<'a, T: ?Sized> {
    as_ref: &'a T,
}

impl<'a, T: ?Sized> Clone for RefPtrEq<'a, T> {
    fn clone(&self) -> RefPtrEq<'a, T> {
        RefPtrEq {
            as_ref: self.as_ref,
        }
    }
}

impl<'a, T: Eq + ?Sized> Eq for RefPtrEq<'a, T> {}

impl<'a, T: ?Sized> PartialEq for RefPtrEq<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.as_ref, other.as_ref)
    }
}

impl<'a, T: ?Sized> PartialEq<&T> for RefPtrEq<'a, T> {
    fn eq(&self, other: &&T) -> bool {
        std::ptr::eq(self.as_ref, *other)
    }
}

impl<'a, T: ?Sized> Hash for RefPtrEq<'a, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let ptr: *const T = self.as_ref;
        ptr.hash(state);
    }
}

impl<'a, 'b, T> From<&'b T> for RefPtrEq<'a, T>
where
    T: ?Sized,
    'b: 'a,
{
    fn from(other: &'b T) -> Self {
        RefPtrEq { as_ref: other }
    }
}

pub struct WordTracker<'a> {
    words_by_ll: HashMap<LocatedLetter, HashSet<RefPtrEq<'a, str>>>,
    words_by_letter: HashMap<char, HashSet<RefPtrEq<'a, str>>>,
}

impl<'a> WordTracker<'a> {
    pub fn new(words: &Vec<Rc<str>>) -> WordTracker {
        let mut words_by_ll: HashMap<LocatedLetter, HashSet<RefPtrEq<str>>> = HashMap::new();
        let mut words_by_letter: HashMap<char, HashSet<RefPtrEq<str>>> = HashMap::new();
        for word in words {
            for (index, letter) in word.char_indices() {
                words_by_ll
                    .entry(LocatedLetter::new(letter, index as u8))
                    .or_insert(HashSet::new())
                    .insert(RefPtrEq::from(word.as_ref()));
                if index == 0
                    || word
                        .chars()
                        .take(index)
                        .all(|other_letter| other_letter != letter)
                {
                    words_by_letter
                        .entry(letter)
                        .or_insert(HashSet::new())
                        .insert(RefPtrEq::from(word.as_ref()));
                }
            }
        }
        WordTracker {
            words_by_ll: words_by_ll,
            words_by_letter: words_by_letter,
        }
    }

    pub fn words_with_located_letter(
        &self,
        ll: &LocatedLetter,
    ) -> Option<&HashSet<RefPtrEq<'a, str>>> {
        self.words_by_ll.get(ll)
    }

    pub fn words_with_letter(&self, letter: char) -> Option<&HashSet<RefPtrEq<'a, str>>> {
        self.words_by_letter.get(&letter)
    }

    pub fn remove(&mut self, word: &'a str) {
        for (index, letter) in word.char_indices() {
            self.words_by_ll
                .entry(LocatedLetter::new(letter, index as u8))
                .and_modify(|set| {
                    set.remove(&word.into());
                });
            if index == 0
                || word
                    .chars()
                    .take(index)
                    .all(|other_letter| other_letter != letter)
            {
                self.words_by_letter.entry(letter).and_modify(|set| {
                    set.remove(&word.into());
                });
            }
        }
    }
}

impl<'a> Clone for WordTracker<'a> {
    fn clone(&self) -> WordTracker<'a> {
        WordTracker {
            words_by_ll: self.words_by_ll.clone(),
            words_by_letter: self.words_by_letter.clone(),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::io::Cursor;

    macro_rules! assert_rc_eq {
        ($rc_vec:expr, $non_rc_vec:expr) => {
            assert_eq!(
                $rc_vec,
                $non_rc_vec
                    .iter()
                    .map(|thing| Rc::from(*thing))
                    .collect::<Vec<Rc<_>>>()
            );
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

        assert_rc_eq!(still_possible, vec!["wordb"]);
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

        assert_rc_eq!(still_possible, vec!["worda", "wordb", "smore"]);
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

        assert_rc_eq!(still_possible, vec!["other", "smore"]);
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

    fn rc_string_vec(vec_str: Vec<&'static str>) -> Vec<Rc<str>> {
        vec_str.iter().map(|word| Rc::from(*word)).collect()
    }

    #[test]
    fn ref_ptr_eq_in_hash_map() {
        let mut map: HashMap<RefPtrEq<str>, u32> = HashMap::new();

        let word1 = "abc";
        let word2 = "abc".to_string();

        map.insert(word1.into(), 2);
        map.insert(word2.as_str().into(), 3);

        assert_eq!(*map.get(&word1.into()).unwrap(), 2);
        assert_eq!(*map.get(&word2.as_str().into()).unwrap(), 3);
    }

    #[test]
    fn ref_ptr_clone() {
        let word1 = "abc".to_string();

        let ref1 = RefPtrEq::from(word1.as_str());
        let ref2 = ref1.clone();

        assert_eq!(ref1, ref2);
    }

    fn word_tracker_by_located_letter() {
        let all_words = rc_string_vec(vec!["hello", "hallo", "worda"]);
        let tracker = WordTracker::new(&all_words);

        assert_eq!(
            *tracker
                .words_with_located_letter(&LocatedLetter::new('a', 1))
                .unwrap(),
            HashSet::from_iter(
                all_words[1..2]
                    .iter()
                    .map(|rc_word| rc_word.as_ref().into())
            )
        );
        assert_eq!(
            *tracker
                .words_with_located_letter(&LocatedLetter::new('h', 0))
                .unwrap(),
            HashSet::from_iter(
                all_words[0..2]
                    .iter()
                    .map(|rc_word| rc_word.as_ref().into())
            )
        );
        assert_eq!(
            *tracker
                .words_with_located_letter(&LocatedLetter::new('w', 0))
                .unwrap(),
            HashSet::from_iter(
                all_words[3..4]
                    .iter()
                    .map(|rc_word| rc_word.as_ref().into())
            )
        );
        assert_eq!(
            tracker.words_with_located_letter(&LocatedLetter::new('w', 1)),
            None
        );
        assert_eq!(
            tracker.words_with_located_letter(&LocatedLetter::new('z', 1)),
            None
        );
    }

    fn word_tracker_by_letter() {
        let all_words = rc_string_vec(vec!["hello", "hallo", "worda"]);
        let tracker = WordTracker::new(&all_words);

        assert_eq!(
            *tracker.words_with_letter('a').unwrap(),
            HashSet::from_iter(
                all_words[1..3]
                    .iter()
                    .map(|rc_word| rc_word.as_ref().into())
            )
        );
        assert_eq!(
            *tracker.words_with_letter('h').unwrap(),
            HashSet::from_iter(
                all_words[0..2]
                    .iter()
                    .map(|rc_word| rc_word.as_ref().into())
            )
        );
        assert_eq!(
            *tracker.words_with_letter('w').unwrap(),
            HashSet::from_iter(
                all_words[3..4]
                    .iter()
                    .map(|rc_word| rc_word.as_ref().into())
            )
        );
        assert_eq!(tracker.words_with_letter('z'), None);
    }

    fn word_tracker_remove() {
        let all_words = rc_string_vec(vec!["hello", "hallo", "worda"]);
        let mut tracker = WordTracker::new(&all_words);

        tracker.remove(&all_words[1]);

        assert_eq!(
            *tracker.words_with_letter('a').unwrap(),
            all_words[2..3]
                .iter()
                .map(|rc_word| rc_word.as_ref().into())
                .collect()
        );
        assert_eq!(
            *tracker.words_with_letter('l').unwrap(),
            all_words[0..1]
                .iter()
                .map(|rc_word| rc_word.as_ref().into())
                .collect()
        );
        assert_eq!(
            *tracker
                .words_with_located_letter(&LocatedLetter::new('h', 0))
                .unwrap(),
            all_words[0..1]
                .iter()
                .map(|rc_word| rc_word.as_ref().into())
                .collect()
        );
    }

    fn word_tracker_clone() {
        let all_words = rc_string_vec(vec!["hello", "hallo", "worda"]);
        let tracker = WordTracker::new(&all_words);

        let mut tracker_clone = tracker.clone();
        tracker_clone.remove(&all_words[2]);

        assert_eq!(tracker.words_with_letter('a').unwrap().len(), 2);
        assert_eq!(tracker_clone.words_with_letter('a').unwrap().len(), 1);
    }
}

#[cfg(all(feature = "unstable", test))]
mod benches {

    extern crate test;

    use super::*;
    use std::collections::HashSet;
    use std::fs::File;
    use std::io::{BufRead, BufReader, Result};
    use std::rc::Rc;
    use test::Bencher;

    #[bench]
    fn bench_hash_set_construction_no_dupes_ref_ptr_eq(b: &mut Bencher) -> Result<()> {
        let mut words_reader = BufReader::new(File::open("../data/wordle-words.txt")?);
        let words: Vec<String> = words_reader.lines().collect::<Result<Vec<String>>>()?;
        let words_ref: Vec<Rc<str>> = words.iter().map(|word| Rc::from(word.as_str())).collect();

        b.iter(|| {
            let set: HashSet<RefPtrEq<'_, str>> =
                words_ref.iter().map(|word| word.as_ref().into()).collect();
            return set.len();
        });

        Ok(())
    }

    #[bench]
    fn bench_hash_set_construction_with_dupes_ref_ptr_eq(b: &mut Bencher) -> Result<()> {
        let mut words_reader =
            BufReader::new(File::open("../data/1000-wordle-words-shuffled.txt")?);
        let words: Vec<String> = words_reader.lines().collect::<Result<Vec<String>>>()?;
        let words_ref: Vec<Rc<str>> = words
            .iter()
            .chain(words.iter().chain(words.iter()))
            .map(|word| Rc::from(word.as_str()))
            .collect();

        b.iter(|| {
            let set: HashSet<RefPtrEq<'_, str>> =
                words_ref.iter().map(|word| word.as_ref().into()).collect();
            return set.len();
        });

        Ok(())
    }

    #[bench]
    fn bench_hash_set_intersection_ref_ptr_eq(b: &mut Bencher) -> Result<()> {
        let mut words_reader =
            BufReader::new(File::open("../data/1000-wordle-words-shuffled.txt")?);
        let words: Vec<String> = words_reader.lines().collect::<Result<Vec<String>>>()?;
        let words_ref_a: Vec<Rc<str>> = words
            .iter()
            .take(600)
            .map(|word| Rc::from(word.as_str()))
            .collect();
        let words_ref_b: Vec<Rc<str>> = words
            .iter()
            .skip(200)
            .take(600)
            .map(|word| Rc::from(word.as_str()))
            .collect();
        let words_ref_c: Vec<Rc<str>> = words
            .iter()
            .skip(400)
            .take(600)
            .map(|word| Rc::from(word.as_str()))
            .collect();

        b.iter(|| {
            let set_a: HashSet<RefPtrEq<'_, str>> = words_ref_a
                .iter()
                .map(|word| word.as_ref().into())
                .collect();
            let set_b: HashSet<RefPtrEq<'_, str>> = words_ref_b
                .iter()
                .map(|word| word.as_ref().into())
                .collect();
            let set_c: HashSet<RefPtrEq<'_, str>> = words_ref_c
                .iter()
                .map(|word| word.as_ref().into())
                .collect();
            let mut joined: HashSet<RefPtrEq<'_, str>> =
                set_a.intersection(&set_b).map(RefPtrEq::clone).collect();
            return joined.intersection(&set_c).count();
        });

        Ok(())
    }

    #[bench]
    fn bench_hash_set_construction_no_dupes_rc_eq(b: &mut Bencher) -> Result<()> {
        let mut words_reader = BufReader::new(File::open("../data/wordle-words.txt")?);
        let words: Vec<String> = words_reader.lines().collect::<Result<Vec<String>>>()?;
        let words_ref: Vec<Rc<str>> = words.iter().map(|word| Rc::from(word.as_str())).collect();

        b.iter(|| {
            let set: HashSet<Rc<str>> = words_ref.iter().map(|word| Rc::clone(word)).collect();
            return set.len();
        });

        Ok(())
    }

    #[bench]
    fn bench_hash_set_construction_with_dupes_rc_eq(b: &mut Bencher) -> Result<()> {
        let mut words_reader =
            BufReader::new(File::open("../data/1000-wordle-words-shuffled.txt")?);
        let words: Vec<String> = words_reader.lines().collect::<Result<Vec<String>>>()?;
        let words_ref: Vec<Rc<str>> = words
            .iter()
            .chain(words.iter().chain(words.iter()))
            .map(|word| Rc::from(word.as_str()))
            .collect();

        b.iter(|| {
            let set: HashSet<Rc<str>> = words_ref.iter().map(|word| Rc::clone(word)).collect();
            return set.len();
        });

        Ok(())
    }

    #[bench]
    fn bench_hash_set_intersection_rc_eq(b: &mut Bencher) -> Result<()> {
        let mut words_reader =
            BufReader::new(File::open("../data/1000-wordle-words-shuffled.txt")?);
        let words: Vec<String> = words_reader.lines().collect::<Result<Vec<String>>>()?;
        let words_ref_a: Vec<Rc<str>> = words
            .iter()
            .take(600)
            .map(|word| Rc::from(word.as_str()))
            .collect();
        let words_ref_b: Vec<Rc<str>> = words
            .iter()
            .skip(200)
            .take(600)
            .map(|word| Rc::from(word.as_str()))
            .collect();
        let words_ref_c: Vec<Rc<str>> = words
            .iter()
            .skip(400)
            .take(600)
            .map(|word| Rc::from(word.as_str()))
            .collect();

        b.iter(|| {
            let set_a: HashSet<Rc<str>> = words_ref_a.iter().map(Rc::clone).collect();
            let set_b: HashSet<Rc<str>> = words_ref_b.iter().map(Rc::clone).collect();
            let set_c: HashSet<Rc<str>> = words_ref_c.iter().map(Rc::clone).collect();
            let mut joined: HashSet<Rc<str>> = set_a.intersection(&set_b).map(Rc::clone).collect();
            return joined.intersection(&set_c).count();
        });

        Ok(())
    }
}
