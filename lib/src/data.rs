use crate::results::*;
use std::cmp::max;
use std::collections::HashMap;
use std::hash::Hash;
use std::io::BufRead;
use std::io::Result;
use std::marker::PhantomData;
use std::ops::Deref;
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
            max_word_length,
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
                    max_word_length = max(word_length, max_word_length);
                    Some(Rc::from(word.to_lowercase().as_str()))
                })
                .collect(),
            max_word_length,
        }
    }

    /// Returns the number of possible words.
    pub fn len(&self) -> usize {
        self.all_words.len()
    }

    /// Returns true iff this word bank is empty.
    pub fn is_empty(&self) -> bool {
        self.all_words.is_empty()
    }

    /// Returns the length of the longest word in the bank.
    pub fn max_word_len(&self) -> usize {
        self.max_word_length
    }
}

impl Deref for WordBank {
    type Target = [Rc<str>];

    /// Derefs the list of words in the `WordBank` as a slice.
    fn deref(&self) -> &Self::Target {
        &self.all_words
    }
}

/// Counts the number of words that have letters in certain locations.
#[derive(Clone)]
pub struct WordCounter {
    num_words: u32,
    num_words_by_ll: HashMap<LocatedLetter, u32>,
    num_words_by_letter: HashMap<char, u32>,
}

impl WordCounter {
    /// Creates a new word counter based on the given word list.
    pub fn new(words: &[Rc<str>]) -> WordCounter {
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
            num_words: words.len() as u32,
            num_words_by_ll,
            num_words_by_letter,
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

    /// Retrieves the total number of words in this counter.
    pub fn num_words(&self) -> u32 {
        self.num_words
    }
}

/// Computes the unique set of words that have each letter, and that have a letter in a given
/// location.
///
/// This could be useful for creating your own Wordle-solving algorithms.
pub struct WordTracker<'a> {
    empty_list: Vec<Rc<str>>,
    all_words: Vec<Rc<str>>,
    words_by_letter: HashMap<char, Vec<Rc<str>>>,
    words_by_located_letter: HashMap<LocatedLetter, Vec<Rc<str>>>,
    phantom: PhantomData<&'a Rc<str>>,
}

impl<'a> WordTracker<'a> {
    /// Constructs a new `WordTracker` from the given words. Note that the words are not checked
    /// for uniqueness, so if duplicates exist in the given words, then those duplicates will
    /// remain part of this tracker's information.
    pub fn new<'b, I>(words: I) -> WordTracker<'a>
    where
        I: IntoIterator<Item = &'b Rc<str>>,
    {
        let all_words: Vec<Rc<str>> = words.into_iter().map(Rc::clone).collect();
        let mut words_by_letter: HashMap<char, Vec<Rc<str>>> = HashMap::new();
        let mut words_by_located_letter: HashMap<LocatedLetter, Vec<Rc<str>>> = HashMap::new();
        for word in all_words.iter() {
            for (index, letter) in word.as_ref().char_indices() {
                words_by_located_letter
                    .entry(LocatedLetter::new(letter, index as u8))
                    .or_insert(Vec::new())
                    .push(Rc::clone(word));
                if index == 0
                    || word
                        .chars()
                        .take(index)
                        .all(|other_letter| letter != other_letter)
                {
                    words_by_letter
                        .entry(letter)
                        .or_insert(Vec::new())
                        .push(Rc::clone(word));
                }
            }
        }
        WordTracker {
            empty_list: Vec::new(),
            all_words,
            words_by_letter,
            words_by_located_letter,
            phantom: PhantomData,
        }
    }

    /// Retrieves the full list of words stored in this word tracker.
    pub fn all_words(&self) -> &[Rc<str>] {
        &self.all_words
    }

    /// Returns true iff any of the words in this tracker contain the given letter.
    pub fn has_letter(&self, letter: char) -> bool {
        self.words_by_letter.contains_key(&letter)
    }

    /// Returns an [`Iterator`] over words that have the given letter at the given location.
    pub fn words_with_located_letter(&self, ll: &LocatedLetter) -> impl Iterator<Item = &Rc<str>> {
        self.words_by_located_letter
            .get(ll)
            .map(|words| words.iter())
            .unwrap_or_else(|| self.empty_list.iter())
    }

    /// Returns an [`Iterator`] over words that have the given letter.
    pub fn words_with_letter(&self, letter: char) -> impl Iterator<Item = &Rc<str>> {
        self.words_by_letter
            .get(&letter)
            .map(|words| words.iter())
            .unwrap_or_else(|| self.empty_list.iter())
    }

    /// Returns an [`Iterator`] over words that don't have the given letter at the given location.
    pub fn words_with_letter_not_here<'b>(
        &'a self,
        ll: &'b LocatedLetter,
    ) -> impl Iterator<Item = &'b Rc<str>>
    where
        'a: 'b,
    {
        let words_with_letter: &'a Vec<Rc<str>> = self
            .words_by_letter
            .get(&ll.letter)
            .unwrap_or(&self.empty_list);
        words_with_letter
            .iter()
            .filter(|&word| word.chars().nth(ll.location as usize).unwrap() != ll.letter)
    }

    /// Returns an [`Iterator`] over words that don't have the given letter.
    pub fn words_without_letter<'b>(&'a self, letter: &'b char) -> impl Iterator<Item = &'b Rc<str>>
    where
        'a: 'b,
    {
        self.all_words.iter().filter(|word| !word.contains(*letter))
    }
}

impl<'a> Clone for WordTracker<'a> {
    fn clone(&self) -> WordTracker<'a> {
        WordTracker {
            empty_list: Vec::new(),
            all_words: self.all_words.clone(),
            words_by_located_letter: self.words_by_located_letter.clone(),
            words_by_letter: self.words_by_letter.clone(),
            phantom: PhantomData,
        }
    }
}

/// A compressed form of LetterResults. Can only store vectors of up to 10 results, else it panics.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct CompressedGuessResult {
    data: u32,
}

impl CompressedGuessResult {
    /// Creates a compressed form of the given letter results. Panics if `letter_results` is longer
    /// than 10.
    pub fn from_result(letter_results: &[LetterResult]) -> Self {
        let mut data = 0;
        let mut index = 0;
        for letter in letter_results {
            data |= 1
                << (index
                    + match letter {
                        LetterResult::Correct => 0,
                        LetterResult::PresentNotHere => 1,
                        LetterResult::NotPresent => 2,
                    });
            index += 3;
        }
        Self { data }
    }
}

/// Stores all the results for each objective<->guess pair.
#[derive(Clone)]
pub struct GuessResults {
    results_by_objective_guess_pair: HashMap<(Rc<str>, Rc<str>), CompressedGuessResult>,
}

impl GuessResults {
    /// Precomputes and stores all the results for each objective<->guess pair.
    pub fn compute(all_words: &[Rc<str>]) -> Self {
        let mut results_by_objective_guess_pair: HashMap<
            (Rc<str>, Rc<str>),
            CompressedGuessResult,
        > = HashMap::new();
        for objective in all_words {
            for guess in all_words {
                results_by_objective_guess_pair.insert(
                    (objective.clone(), guess.clone()),
                    CompressedGuessResult::from_result(
                        &get_result_for_guess(objective, guess).results,
                    ),
                );
            }
        }
        Self {
            results_by_objective_guess_pair,
        }
    }

    /// Retrieves the result for the given objective<->guess pair.
    pub fn get_result(
        &self,
        objective: &Rc<str>,
        guess: &Rc<str>,
    ) -> Option<CompressedGuessResult> {
        self.results_by_objective_guess_pair
            .get(&(objective.clone(), guess.clone()))
            .copied()
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

    fn rc_string_vec(vec_str: Vec<&'static str>) -> Vec<Rc<str>> {
        vec_str.iter().map(|word| Rc::from(*word)).collect()
    }

    #[test]
    fn word_tracker_all_words() {
        let all_words = rc_string_vec(vec!["hello", "hallo", "worda"]);
        let tracker = WordTracker::new(&all_words);

        assert_eq!(tracker.all_words(), &all_words,);
    }

    #[test]
    fn word_tracker_by_located_letter() {
        let all_words = rc_string_vec(vec!["hello", "hallo", "worda"]);
        let tracker = WordTracker::new(&all_words);

        assert_eq!(
            Vec::from_iter(tracker.words_with_located_letter(&LocatedLetter::new('a', 1))),
            Vec::from_iter(all_words[1..2].iter())
        );
        assert_eq!(
            Vec::from_iter(tracker.words_with_located_letter(&LocatedLetter::new('h', 0))),
            Vec::from_iter(all_words[0..2].iter())
        );
        assert_eq!(
            Vec::from_iter(tracker.words_with_located_letter(&LocatedLetter::new('w', 0))),
            Vec::from_iter(all_words[2..3].iter())
        );
        assert_eq!(
            tracker
                .words_with_located_letter(&LocatedLetter::new('w', 1))
                .count(),
            0
        );
        assert_eq!(
            tracker
                .words_with_located_letter(&LocatedLetter::new('z', 1))
                .count(),
            0,
        );
    }

    #[test]
    fn word_tracker_by_letter() {
        let all_words = rc_string_vec(vec!["hello", "hallo", "worda"]);
        let tracker = WordTracker::new(&all_words);

        assert_eq!(
            Vec::from_iter(tracker.words_with_letter('a')),
            Vec::from_iter(all_words[1..3].iter())
        );
        assert_eq!(
            Vec::from_iter(tracker.words_with_letter('h')),
            Vec::from_iter(all_words[0..2].iter())
        );
        assert_eq!(
            Vec::from_iter(tracker.words_with_letter('w')),
            Vec::from_iter(all_words[2..3].iter())
        );
        assert_eq!(tracker.words_with_letter('z').count(), 0);
    }

    #[test]
    fn compressed_guess_result_equality() {
        let result_correct = CompressedGuessResult::from_result(&[LetterResult::Correct; 4]);
        let result_not_here =
            CompressedGuessResult::from_result(&[LetterResult::PresentNotHere; 4]);
        let result_not_present = CompressedGuessResult::from_result(&[LetterResult::NotPresent; 4]);

        assert_eq!(result_correct, result_correct);
        assert_eq!(result_not_here, result_not_here);
        assert_eq!(result_not_present, result_not_present);
        assert!(result_correct != result_not_here);
        assert!(result_correct != result_not_present);
        assert!(result_not_here != result_not_present);
    }

    #[test]
    fn guess_results_computes_for_all_words() {
        let all_words = rc_string_vec(vec!["hello", "hallo", "worda"]);
        let results = GuessResults::compute(&all_words);

        assert_eq!(
            results.get_result(&all_words[0], &all_words[0]),
            Some(CompressedGuessResult::from_result(
                &[LetterResult::Correct; 5]
            ))
        );
        assert_eq!(
            results.get_result(&all_words[0], &all_words[1]),
            Some(CompressedGuessResult::from_result(&[
                LetterResult::Correct,
                LetterResult::NotPresent,
                LetterResult::Correct,
                LetterResult::Correct,
                LetterResult::Correct,
            ]))
        );
        assert_eq!(
            results.get_result(&all_words[0], &all_words[2]),
            Some(CompressedGuessResult::from_result(&[
                LetterResult::NotPresent,
                LetterResult::PresentNotHere,
                LetterResult::NotPresent,
                LetterResult::NotPresent,
                LetterResult::NotPresent,
            ]))
        );
        assert_eq!(
            results.get_result(&all_words[1], &all_words[1]),
            Some(CompressedGuessResult::from_result(
                &[LetterResult::Correct; 5]
            ))
        );
        assert_eq!(
            results.get_result(&all_words[1], &all_words[0]),
            Some(CompressedGuessResult::from_result(&[
                LetterResult::Correct,
                LetterResult::NotPresent,
                LetterResult::Correct,
                LetterResult::Correct,
                LetterResult::Correct,
            ]))
        );
        assert_eq!(
            results.get_result(&all_words[1], &all_words[2]),
            Some(CompressedGuessResult::from_result(&[
                LetterResult::NotPresent,
                LetterResult::PresentNotHere,
                LetterResult::NotPresent,
                LetterResult::NotPresent,
                LetterResult::PresentNotHere,
            ]))
        );
        assert_eq!(
            results.get_result(&all_words[2], &all_words[2]),
            Some(CompressedGuessResult::from_result(
                &[LetterResult::Correct; 5]
            ))
        );
        assert_eq!(
            results.get_result(&all_words[2], &all_words[0]),
            Some(CompressedGuessResult::from_result(&[
                LetterResult::NotPresent,
                LetterResult::NotPresent,
                LetterResult::NotPresent,
                LetterResult::NotPresent,
                LetterResult::PresentNotHere,
            ]))
        );
        assert_eq!(
            results.get_result(&all_words[2], &all_words[1]),
            Some(CompressedGuessResult::from_result(&[
                LetterResult::NotPresent,
                LetterResult::PresentNotHere,
                LetterResult::NotPresent,
                LetterResult::NotPresent,
                LetterResult::PresentNotHere,
            ]))
        );
    }
}

#[cfg(all(feature = "unstable", test))]
mod benches {

    extern crate test;

    use super::*;
    use std::fs::File;
    use std::io::{BufReader, Result};
    use test::Bencher;

    #[bench]
    fn bench_word_counter_new(b: &mut Bencher) -> Result<()> {
        let mut words_reader =
            BufReader::new(File::open("../data/1000-improved-words-shuffled.txt")?);
        let bank = WordBank::from_reader(&mut words_reader)?;

        b.iter(|| WordCounter::new(&bank));

        Ok(())
    }

    #[bench]
    fn bench_word_counter_clone(b: &mut Bencher) -> Result<()> {
        let mut words_reader =
            BufReader::new(File::open("../data/1000-improved-words-shuffled.txt")?);
        let bank = WordBank::from_reader(&mut words_reader)?;
        let counter = WordCounter::new(&bank);

        b.iter(|| counter.clone());

        Ok(())
    }

    #[bench]
    fn bench_word_tracker_new(b: &mut Bencher) -> Result<()> {
        let mut words_reader =
            BufReader::new(File::open("../data/1000-improved-words-shuffled.txt")?);
        let bank = WordBank::from_reader(&mut words_reader)?;

        b.iter(|| WordTracker::new(&*bank));

        Ok(())
    }

    #[bench]
    fn bench_word_tracker_clone(b: &mut Bencher) -> Result<()> {
        let mut words_reader =
            BufReader::new(File::open("../data/1000-improved-words-shuffled.txt")?);
        let bank = WordBank::from_reader(&mut words_reader)?;
        let tracker = WordTracker::new(&*bank);

        b.iter(|| tracker.clone());

        Ok(())
    }
}
