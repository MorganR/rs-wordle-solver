use crate::results::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::Hash;
use std::hash::Hasher;
use std::io::BufRead;
use std::io::Result;
use std::iter::zip;
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
        for ((index, letter), result) in zip(
            guess_result.guess.char_indices(),
            guess_result.results.iter(),
        ) {
            match result {
                LetterResult::Correct => {
                    self.must_contain_here
                        .insert(LocatedLetter::new(letter, index as u8));
                }
                LetterResult::PresentNotHere => {
                    self.must_contain_but_not_here
                        .insert(LocatedLetter::new(letter, index as u8));
                }
                LetterResult::NotPresent => {
                    self.must_not_contain.insert(letter);
                }
            }
        }
    }

    /// Adds the given restrictions to this restriction.
    pub fn union(&mut self, other: &WordRestrictions) {
        self.must_contain_here
            .extend(other.must_contain_here.iter().map(Clone::clone));
        self.must_contain_but_not_here
            .extend(other.must_contain_but_not_here.iter().map(Clone::clone));
        self.must_not_contain
            .extend(other.must_not_contain.iter().map(Clone::clone));
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
pub struct RefPtrEq<T: ?Sized> {
    rc: Rc<T>,
}

impl<T: ?Sized> RefPtrEq<T> {
    pub fn as_rc(&self) -> &Rc<T> {
        &self.rc
    }
}

impl<T: ?Sized> Clone for RefPtrEq<T> {
    fn clone(&self) -> RefPtrEq<T> {
        RefPtrEq {
            rc: Rc::clone(&self.rc),
        }
    }
}

impl<T: Eq + ?Sized> Eq for RefPtrEq<T> {}

impl<T: ?Sized> PartialEq for RefPtrEq<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.rc, &other.rc)
    }
}

impl<T: ?Sized> PartialEq<&T> for RefPtrEq<T> {
    fn eq(&self, other: &&T) -> bool {
        std::ptr::eq(Rc::as_ptr(&self.rc), *other)
    }
}

impl<T: ?Sized> Hash for RefPtrEq<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let ptr: *const T = Rc::as_ptr(&self.rc);
        ptr.hash(state);
    }
}

impl<T> From<&Rc<T>> for RefPtrEq<T>
where
    T: ?Sized,
{
    fn from(other: &Rc<T>) -> Self {
        RefPtrEq {
            rc: Rc::clone(other),
        }
    }
}

impl<T> AsRef<T> for RefPtrEq<T>
where
    T: ?Sized,
{
    fn as_ref(&self) -> &T {
        self.rc.as_ref()
    }
}

pub struct WordTracker {
    empty_set: HashSet<Rc<str>>,
    all_words: HashSet<Rc<str>>,
    words_by_ll: HashMap<LocatedLetter, HashSet<Rc<str>>>,
    words_by_letter: HashMap<char, HashSet<Rc<str>>>,
}

impl WordTracker {
    pub fn new<'a, I>(words: I) -> WordTracker
    where
        I: IntoIterator<Item = &'a Rc<str>>,
    {
        let mut words_by_ll: HashMap<LocatedLetter, HashSet<Rc<str>>> = HashMap::new();
        let mut words_by_letter: HashMap<char, HashSet<Rc<str>>> = HashMap::new();
        let all_words: HashSet<Rc<str>> = words.into_iter().map(Rc::clone).collect();
        for word in all_words.iter() {
            for (index, letter) in word.as_ref().char_indices() {
                words_by_ll
                    .entry(LocatedLetter::new(letter, index as u8))
                    .or_insert(HashSet::new())
                    .insert(Rc::clone(word));
                if index == 0
                    || word
                        .as_ref()
                        .chars()
                        .take(index)
                        .all(|other_letter| other_letter != letter)
                {
                    words_by_letter
                        .entry(letter)
                        .or_insert(HashSet::new())
                        .insert(Rc::clone(word));
                }
            }
        }
        WordTracker {
            empty_set: HashSet::new(),
            all_words: all_words,
            words_by_ll: words_by_ll,
            words_by_letter: words_by_letter,
        }
    }

    pub fn all_words(&self) -> &HashSet<Rc<str>> {
        &self.all_words
    }

    pub fn has_letter(&self, letter: char) -> bool {
        self.words_by_letter.contains_key(&letter)
    }

    pub fn words_with_located_letter(&self, ll: &LocatedLetter) -> &HashSet<Rc<str>> {
        self.words_by_ll.get(ll).unwrap_or(&self.empty_set)
    }

    pub fn words_with_letter(&self, letter: char) -> &HashSet<Rc<str>> {
        self.words_by_letter.get(&letter).unwrap_or(&self.empty_set)
    }

    // pub fn remove(&mut self, word: &'a str) {
    //     for (index, letter) in word.char_indices() {
    //         self.words_by_ll
    //             .entry(LocatedLetter::new(letter, index as u8))
    //             .and_modify(|set| {
    //                 set.remove(&word.into());
    //             });
    //         if index == 0
    //             || word
    //                 .chars()
    //                 .take(index)
    //                 .all(|other_letter| other_letter != letter)
    //         {
    //             self.words_by_letter.entry(letter).and_modify(|set| {
    //                 set.remove(&word.into());
    //             });
    //         }
    //     }
    // }

    // pub fn remove_all<I: IntoIterator<Item = &'a str>>(&mut self, words: I) {
    //     for word in words {
    //         self.remove(word);
    //     }
    // }
}

impl Clone for WordTracker {
    fn clone(&self) -> WordTracker {
        WordTracker {
            empty_set: HashSet::new(),
            all_words: self.all_words.clone(),
            words_by_ll: self.words_by_ll.clone(),
            words_by_letter: self.words_by_letter.clone(),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

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

        let word1 = Rc::from("abc");
        let word2 = Rc::from("abc");

        let ref1 = RefPtrEq::from(&word1);
        let ref2 = RefPtrEq::from(&word2);
        assert!(ref1 != ref2);

        map.insert(RefPtrEq::from(&word1), 2);
        map.insert(RefPtrEq::from(&word2), 3);

        assert_eq!(*map.get(&RefPtrEq::from(&word1)).unwrap(), 2);
        assert_eq!(*map.get(&RefPtrEq::from(&word2)).unwrap(), 3);
    }

    #[test]
    fn ref_ptr_clone() {
        let word1: Rc<str> = Rc::from("abc");

        let ref1 = RefPtrEq::from(&word1);
        let ref2 = ref1.clone();

        assert_eq!(ref1, ref2);
    }

    #[test]
    fn word_tracker_all_words() {
        let all_words = rc_string_vec(vec!["hello", "hallo", "worda"]);
        let tracker = WordTracker::new(&all_words);

        assert_eq!(
            *tracker.all_words(),
            HashSet::from_iter(all_words[0..3].iter().map(Rc::clone))
        );
    }

    #[test]
    fn word_tracker_by_located_letter() {
        let all_words = rc_string_vec(vec!["hello", "hallo", "worda"]);
        let tracker = WordTracker::new(&all_words);

        assert_eq!(
            *tracker.words_with_located_letter(&LocatedLetter::new('a', 1)),
            HashSet::from_iter(all_words[1..2].iter().map(Rc::clone))
        );
        assert_eq!(
            *tracker.words_with_located_letter(&LocatedLetter::new('h', 0)),
            HashSet::from_iter(all_words[0..2].iter().map(Rc::clone))
        );
        assert_eq!(
            *tracker.words_with_located_letter(&LocatedLetter::new('w', 0)),
            HashSet::from_iter(all_words[2..3].iter().map(Rc::clone))
        );
        assert_eq!(
            tracker.words_with_located_letter(&LocatedLetter::new('w', 1)),
            &HashSet::new(),
        );
        assert_eq!(
            tracker.words_with_located_letter(&LocatedLetter::new('z', 1)),
            &HashSet::new(),
        );
    }

    #[test]
    fn word_tracker_by_letter() {
        let all_words = rc_string_vec(vec!["hello", "hallo", "worda"]);
        let tracker = WordTracker::new(&all_words);

        assert_eq!(
            *tracker.words_with_letter('a'),
            HashSet::from_iter(all_words[1..3].iter().map(Rc::clone))
        );
        assert_eq!(
            *tracker.words_with_letter('h'),
            HashSet::from_iter(all_words[0..2].iter().map(Rc::clone))
        );
        assert_eq!(
            *tracker.words_with_letter('w'),
            HashSet::from_iter(all_words[2..3].iter().map(Rc::clone))
        );
        assert_eq!(tracker.words_with_letter('z'), &HashSet::new());
    }

    // fn word_tracker_remove() {
    //     let all_words = rc_string_vec(vec!["hello", "hallo", "worda"]);
    //     let mut tracker = WordTracker::new(&all_words);

    //     tracker.remove(&all_words[1]);

    //     assert_eq!(
    //         *tracker.words_with_letter('a'),
    //         all_words[2..3]
    //             .iter()
    //             .map(|rc_word| rc_word.as_ref().into())
    //             .collect()
    //     );
    //     assert_eq!(
    //         *tracker.words_with_letter('l'),
    //         all_words[0..1]
    //             .iter()
    //             .map(|rc_word| rc_word.as_ref().into())
    //             .collect()
    //     );
    //     assert_eq!(
    //         *tracker
    //             .words_with_located_letter(&LocatedLetter::new('h', 0)),
    //         all_words[0..1]
    //             .iter()
    //             .map(|rc_word| rc_word.as_ref().into())
    //             .collect()
    //     );
    // }

    // fn word_tracker_clone() {
    //     let all_words = rc_string_vec(vec!["hello", "hallo", "worda"]);
    //     let tracker = WordTracker::new(&all_words);

    //     let mut tracker_clone = tracker.clone();
    //     tracker_clone.remove(&all_words[2]);

    //     assert_eq!(tracker.words_with_letter('a').len(), 2);
    //     assert_eq!(tracker_clone.words_with_letter('a').len(), 1);
    // }
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
            let set: HashSet<RefPtrEq<str>> = words_ref.iter().map(RefPtrEq::from).collect();
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
            let set: HashSet<RefPtrEq<str>> = words_ref.iter().map(RefPtrEq::from).collect();
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
            let set_a: HashSet<RefPtrEq<str>> = words_ref_a.iter().map(RefPtrEq::from).collect();
            let set_b: HashSet<RefPtrEq<str>> = words_ref_b.iter().map(RefPtrEq::from).collect();
            let set_c: HashSet<RefPtrEq<str>> = words_ref_c.iter().map(RefPtrEq::from).collect();
            let mut joined: HashSet<RefPtrEq<str>> =
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
