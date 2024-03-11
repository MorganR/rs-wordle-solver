use crate::results::*;
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::io;
use std::ops::Deref;
use std::result::Result;
use std::sync::Arc;

/// A letter along with its location in the word.
///
/// ```
/// use rs_wordle_solver::details::LocatedLetter;
///
/// let word = "abc";
///
/// let mut located_letters = Vec::new();
/// for (index, letter) in word.char_indices() {
///    located_letters.push(LocatedLetter::new(letter, index as u8));
/// }
///
/// assert_eq!(&located_letters, &[
///     LocatedLetter::new('a', 0),
///     LocatedLetter::new('b', 1),
///     LocatedLetter::new('c', 2),
/// ]);
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
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

/// Contains all the possible words for a Wordle game.
#[derive(Clone, Debug, PartialEq)]
pub struct WordBank {
    all_words: Vec<Arc<str>>,
    word_length: usize,
}

impl WordBank {
    /// Constructs a new `WordBank` struct by reading words from the given reader.
    ///
    /// The reader should provide one word per line. Each word will be trimmed and converted to
    /// lower case.
    ///
    /// After trimming, all words must be the same length, else this returns an error of type
    /// [`WordleError::WordLength`].
    ///
    /// ```no_run
    /// use std::fs::File;
    /// use std::io;
    /// use rs_wordle_solver::WordBank;
    /// # use rs_wordle_solver::WordleError;
    ///
    /// let words_reader = io::BufReader::new(File::open("path/to/my/words.txt")?);
    /// let word_bank = WordBank::from_reader(words_reader)?;
    /// # Ok::<(), WordleError>(())
    /// ```
    pub fn from_reader<R: io::BufRead>(word_reader: R) -> Result<Self, WordleError> {
        let mut word_length = 0;
        let all_words = word_reader
            .lines()
            .filter_map(|maybe_word| {
                maybe_word.map_or_else(
                    |err| Some(Err(WordleError::from(err))),
                    |word| {
                        let normalized: Option<Result<Arc<str>, WordleError>>;
                        (word_length, normalized) =
                            WordBank::parse_word_to_arc(word_length, word.as_ref());
                        normalized
                    },
                )
            })
            .collect::<Result<Vec<Arc<str>>, WordleError>>()?;
        Ok(WordBank {
            all_words,
            word_length,
        })
    }

    /// Constructs a new `WordBank` struct using the words from the given vector. Each word will be
    /// trimmed and converted to lower case.
    ///
    /// After trimming, all words must be the same length, else this returns an error of type
    /// [`WordleError::WordLength`].
    ///
    /// ```
    /// use std::sync::Arc;
    /// use rs_wordle_solver::WordBank;
    /// # use rs_wordle_solver::WordleError;
    ///
    /// let words = vec!["abc".to_string(), "DEF ".to_string()];
    /// let word_bank = WordBank::from_iterator(words.iter())?;
    ///
    /// assert_eq!(&word_bank as &[Arc<str>], &[Arc::from("abc"), Arc::from("def")]);
    /// # Ok::<(), WordleError>(())
    /// ```
    pub fn from_iterator<S>(words: impl IntoIterator<Item = S>) -> Result<Self, WordleError>
    where
        S: AsRef<str>,
    {
        let mut word_length = 0;
        Ok(WordBank {
            all_words: words
                .into_iter()
                .filter_map(|word| {
                    let normalized: Option<Result<Arc<str>, WordleError>>;
                    (word_length, normalized) =
                        WordBank::parse_word_to_arc(word_length, word.as_ref());
                    normalized
                })
                .collect::<Result<Vec<Arc<str>>, WordleError>>()?,
            word_length,
        })
    }

    /// Cleans and parses the given word to an `Arc<str>`, while filtering out empty lines and
    /// returning an error if the word's length differs from `word_length` (if non-zero).
    ///
    /// Returns the new `word_length` to use (if `word_length` was zero before), and the parsed
    /// word.
    fn parse_word_to_arc(
        word_length: usize,
        word: &str,
    ) -> (usize, Option<Result<Arc<str>, WordleError>>) {
        let normalized: Arc<str> = Arc::from(word.trim().to_lowercase().as_str());
        let this_word_length = normalized.len();
        if this_word_length == 0 {
            return (word_length, None);
        }
        if word_length != 0 && word_length != this_word_length {
            return (word_length, Some(Err(WordleError::WordLength(word_length))));
        }
        (this_word_length, Some(Ok(normalized)))
    }

    /// Returns the number of possible words.
    #[inline]
    pub fn len(&self) -> usize {
        self.all_words.len()
    }

    /// Returns true iff this word bank is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.all_words.is_empty()
    }

    /// Returns the length of each word in the word bank.
    #[inline]
    pub fn word_length(&self) -> usize {
        self.word_length
    }
}

impl Deref for WordBank {
    type Target = [Arc<str>];

    /// Derefs the list of words in the `WordBank` as a slice.
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.all_words
    }
}

/// Counts the number of words that contain each letter anywhere, as well as by the location of
/// each letter.
///
/// If you need to know what those words are, see [`WordTracker`].
///
/// Use:
///
/// ```
/// # use rs_wordle_solver::details::WordCounter;
/// # use rs_wordle_solver::details::LocatedLetter;
/// let all_words = vec!["aba", "bbd", "efg"];
/// let counter = WordCounter::new(&all_words);
///
/// assert_eq!(counter.num_words(), 3);
/// assert_eq!(counter.num_words_with_letter('b'), 2);
/// assert_eq!(counter.num_words_with_located_letter(
///     &LocatedLetter::new('b', 0)), 1);
/// ```
#[derive(Clone, Debug)]
pub struct WordCounter {
    num_words: u32,
    num_words_by_ll: HashMap<LocatedLetter, u32>,
    num_words_by_letter: HashMap<char, u32>,
}

impl WordCounter {
    /// Creates a new word counter based on the given word list.
    #[inline]
    pub fn new<S>(words: &[S]) -> WordCounter
    where
        S: AsRef<str>,
    {
        WordCounter::from_iter(words)
    }

    /// Retrieves the count of words with the given letter at the given location.
    ///
    /// ```
    /// use rs_wordle_solver::details::WordCounter;
    /// use rs_wordle_solver::details::LocatedLetter;
    ///
    /// let all_words = vec!["aba", "bbd", "efg"];
    /// let counter = WordCounter::from_iter(&all_words);
    ///
    /// assert_eq!(counter.num_words_with_located_letter(
    ///     &LocatedLetter::new('b', 0)), 1);
    /// assert_eq!(counter.num_words_with_located_letter(
    ///     &LocatedLetter::new('b', 1)), 2);
    /// assert_eq!(counter.num_words_with_located_letter(
    ///     &LocatedLetter::new('b', 2)), 0);
    /// assert_eq!(counter.num_words_with_located_letter(
    ///     &LocatedLetter::new('b', 3)), 0);
    /// assert_eq!(counter.num_words_with_located_letter(
    ///     &LocatedLetter::new('z', 0)), 0);
    /// ```
    pub fn num_words_with_located_letter(&self, ll: &LocatedLetter) -> u32 {
        *self.num_words_by_ll.get(ll).unwrap_or(&0)
    }

    /// Retrieves the count of words that contain the given letter.
    ///
    /// ```
    /// use rs_wordle_solver::details::WordCounter;
    ///
    /// let all_words = vec!["aba", "bbd", "efg"];
    /// let counter = WordCounter::from_iter(&all_words);
    ///
    /// assert_eq!(counter.num_words_with_letter('a'), 1);
    /// assert_eq!(counter.num_words_with_letter('b'), 2);
    /// assert_eq!(counter.num_words_with_letter('z'), 0);
    /// ```
    pub fn num_words_with_letter(&self, letter: char) -> u32 {
        *self.num_words_by_letter.get(&letter).unwrap_or(&0)
    }

    /// Retrieves the total number of words in this counter.
    #[inline]
    pub fn num_words(&self) -> u32 {
        self.num_words
    }
}

impl<S> FromIterator<S> for WordCounter
where
    S: AsRef<str>,
{
    /// Creates a new word counter based on the given word list.
    ///
    /// ```
    /// use rs_wordle_solver::details::WordCounter;
    ///
    /// let all_words = vec!["bba", "bcd", "efg"];
    /// let counter: WordCounter = all_words.iter().collect();
    ///
    /// assert_eq!(counter.num_words(), 3);
    /// ```
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = S>,
    {
        let mut num_words_by_ll: HashMap<LocatedLetter, u32> = HashMap::new();
        let mut num_words_by_letter: HashMap<char, u32> = HashMap::new();
        let mut num_words = 0;
        for word in iter.into_iter() {
            num_words += 1;
            for (index, letter) in word.as_ref().char_indices() {
                *num_words_by_ll
                    .entry(LocatedLetter::new(letter, index as u8))
                    .or_insert(0) += 1;
                if index == 0
                    || word
                        .as_ref()
                        .chars()
                        .take(index)
                        .all(|other_letter| other_letter != letter)
                {
                    *num_words_by_letter.entry(letter).or_insert(0) += 1;
                }
            }
        }
        WordCounter {
            num_words,
            num_words_by_ll,
            num_words_by_letter,
        }
    }
}

/// Computes the unique set of words that contain each letter anywhere, as well as by the location
/// of each letter.
///
/// If you only need to know the number of words instead of the list of words, see [`WordCounter`].
///
/// ```
/// use std::sync::Arc;
/// use rs_wordle_solver::details::WordTracker;
/// use rs_wordle_solver::details::LocatedLetter;
///
/// let all_words = [Arc::from("aba"), Arc::from("bcd"), Arc::from("efg")];
/// let tracker = WordTracker::new(&all_words);
///
/// assert_eq!(tracker.all_words(), &all_words);
/// assert_eq!(
///     Vec::from_iter(tracker.words_with_letter('b')),
///     vec![&Arc::from("aba"), &Arc::from("bcd")]);
/// assert_eq!(
///     Vec::from_iter(tracker.words_with_located_letter(LocatedLetter::new('b', 1))),
///     vec![&Arc::from("aba")]);
/// ```
#[derive(Clone, Debug)]
pub struct WordTracker<'w> {
    all_words: &'w [Arc<str>],
    words_by_letter: HashMap<char, Vec<Arc<str>>>,
    words_by_located_letter: HashMap<LocatedLetter, Vec<Arc<str>>>,
}

impl<'w> WordTracker<'w> {
    /// Constructs a new `WordTracker` from the given words. Note that the words are not checked
    /// for uniqueness, so if duplicates exist in the given words, then those duplicates will
    /// remain part of this tracker's information.
    ///
    /// ```
    /// use std::sync::Arc;
    /// use rs_wordle_solver::details::WordTracker;
    ///
    /// let all_words = vec![Arc::from("aba"), Arc::from("bcd"), Arc::from("efg")];
    /// let tracker = WordTracker::new(&all_words);
    ///
    /// assert_eq!(tracker.all_words(), &all_words);
    /// ```
    pub fn new<'w_in: 'w>(all_words: &'w_in [Arc<str>]) -> WordTracker<'w> {
        let mut words_by_letter: HashMap<char, Vec<Arc<str>>> = HashMap::new();
        let mut words_by_located_letter: HashMap<LocatedLetter, Vec<Arc<str>>> = HashMap::new();
        for word in all_words.iter() {
            let word_ref = word.as_ref();
            for (index, letter) in word_ref.char_indices() {
                words_by_located_letter
                    .entry(LocatedLetter::new(letter, index as u8))
                    .or_default()
                    .push(Arc::clone(word));
                if index == 0
                    || word_ref
                        .chars()
                        .take(index)
                        .all(|other_letter| letter != other_letter)
                {
                    words_by_letter
                        .entry(letter)
                        .or_default()
                        .push(Arc::clone(word));
                }
            }
        }
        WordTracker {
            all_words,
            words_by_letter,
            words_by_located_letter,
        }
    }

    /// Retrieves the full list of words stored in this word tracker.
    ///
    /// ```
    /// use std::sync::Arc;
    /// use rs_wordle_solver::details::WordTracker;
    ///
    /// let all_words = [Arc::from("aba"), Arc::from("bcd"), Arc::from("efg")];
    /// let tracker = WordTracker::new(&all_words);
    ///
    /// assert_eq!(tracker.all_words(), &all_words);
    /// ```
    #[inline]
    pub fn all_words(&self) -> &[Arc<str>] {
        self.all_words
    }

    /// Returns true iff any of the words in this tracker contain the given letter.
    ///
    /// ```
    /// use std::sync::Arc;
    /// use rs_wordle_solver::details::WordTracker;
    ///
    /// let all_words = [Arc::from("aba"), Arc::from("bcd"), Arc::from("efg")];
    /// let tracker = WordTracker::new(&all_words);
    ///
    /// assert!(tracker.has_letter('a'));
    /// assert!(!tracker.has_letter('z'));
    /// ```
    #[inline]
    pub fn has_letter(&self, letter: char) -> bool {
        self.words_by_letter.contains_key(&letter)
    }

    /// Returns an [`Iterator`] over words that have the given letter at the given location.
    ///
    /// ```
    /// use std::sync::Arc;
    /// use rs_wordle_solver::details::WordTracker;
    /// use rs_wordle_solver::details::LocatedLetter;
    ///
    /// let all_words = [Arc::from("bba"), Arc::from("bcd"), Arc::from("efg")];
    /// let tracker = WordTracker::new(&all_words);
    ///
    /// assert_eq!(
    ///     Vec::from_iter(tracker.words_with_located_letter(LocatedLetter::new('b', 0))),
    ///     vec![&Arc::from("bba"), &Arc::from("bcd")]);
    /// assert_eq!(
    ///     Vec::from_iter(tracker.words_with_located_letter(LocatedLetter::new('b', 1))),
    ///     vec![&Arc::from("bba")]);
    /// assert_eq!(
    ///     tracker.words_with_located_letter(LocatedLetter::new('z', 1)).count(),
    ///     0);
    /// ```
    pub fn words_with_located_letter(&self, ll: LocatedLetter) -> impl Iterator<Item = &Arc<str>> {
        self.words_by_located_letter
            .get(&ll)
            .map(|words| words.iter())
            .unwrap_or_default()
    }

    /// Returns an [`Iterator`] over words that have the given letter.
    ///
    /// ```
    /// use std::sync::Arc;
    /// use rs_wordle_solver::details::WordTracker;
    ///
    /// let all_words = [Arc::from("bba"), Arc::from("bcd"), Arc::from("efg")];
    /// let tracker = WordTracker::new(&all_words);
    ///
    /// assert_eq!(
    ///     Vec::from_iter(tracker.words_with_letter('b')),
    ///     vec![&Arc::from("bba"), &Arc::from("bcd")]);
    /// assert_eq!(
    ///     Vec::from_iter(tracker.words_with_letter('e')),
    ///     vec![&Arc::from("efg")]);
    /// assert_eq!(
    ///     tracker.words_with_letter('z').count(),
    ///     0);
    /// ```
    pub fn words_with_letter(&self, letter: char) -> impl Iterator<Item = &Arc<str>> {
        self.words_by_letter
            .get(&letter)
            .map(|words| words.iter())
            .unwrap_or_default()
    }

    /// Returns an [`Iterator`] over words that have the given letter, but not at the given
    /// location.
    ///
    /// ```
    /// use std::sync::Arc;
    /// use rs_wordle_solver::details::WordTracker;
    /// use rs_wordle_solver::details::LocatedLetter;
    ///
    /// let all_words = [Arc::from("bba"), Arc::from("bcd"), Arc::from("efg")];
    /// let tracker = WordTracker::new(&all_words);
    ///
    /// assert_eq!(
    ///     Vec::from_iter(tracker.words_with_letter_not_here(LocatedLetter::new('b', 1))),
    ///     vec![&Arc::from("bcd")]);
    /// assert_eq!(
    ///     tracker.words_with_letter_not_here(LocatedLetter::new('b', 0)).count(),
    ///     0);
    /// assert_eq!(
    ///     tracker.words_with_letter_not_here(LocatedLetter::new('z', 0)).count(),
    ///     0);
    /// ```
    pub fn words_with_letter_not_here(&self, ll: LocatedLetter) -> impl Iterator<Item = &Arc<str>> {
        let words_with_letter = self
            .words_by_letter
            .get(&ll.letter)
            .map(|words| words.iter())
            .unwrap_or_default();
        words_with_letter
            .filter(move |&word| word.chars().nth(ll.location as usize).unwrap() != ll.letter)
    }

    /// Returns an [`Iterator`] over words that don't have the given letter.
    ///
    /// ```
    /// use std::sync::Arc;
    /// use rs_wordle_solver::details::WordTracker;
    /// use rs_wordle_solver::details::LocatedLetter;
    ///
    /// let all_words = [Arc::from("bba"), Arc::from("bcd"), Arc::from("efg")];
    /// let tracker = WordTracker::new(&all_words);
    ///
    /// assert_eq!(
    ///     Vec::from_iter(tracker.words_without_letter('a')),
    ///     vec![&Arc::from("bcd"), &Arc::from("efg")]);
    /// assert_eq!(
    ///     Vec::from_iter(tracker.words_without_letter('z')),
    ///     Vec::from_iter(&all_words));
    /// ```
    pub fn words_without_letter(&self, letter: char) -> impl Iterator<Item = &'w Arc<str>> {
        self.all_words
            .iter()
            .filter(move |word| !word.contains(letter))
    }
}

/// Efficiently tracks all possible words and all unguessed words as zero-cost slices within a
/// single array of all words. This assumes that only unguessed words are possible.
#[derive(Clone, Debug)]
pub struct GroupedWords {
    pub all_words: Vec<Arc<str>>,
    // The `all_words` vector keeps words grouped according to whether they're still possible, and
    // whether they have been guessed. It's grouped into four sections:
    // Index: 0
    // 1. Words that have been guessed, and are still possible. Then index:
    _first_unguessed_possible_word: usize,
    // 2. Words that have not been guessed, and are still possible. Then index:
    _num_possible_words: usize,
    // 3. Words that have not been guessed, but are not possible. Then index:
    _first_guessed_impossible_word: usize,
    // 4. Words that have been guessed, and are not possible.
}

impl GroupedWords {
    /// Constructs a new GroupedWords instance. Initially all words are considered possible and
    /// unguessed.
    pub fn new(words: &WordBank) -> Self {
        Self {
            all_words: words.to_vec(),
            _first_unguessed_possible_word: 0,
            _num_possible_words: words.len(),
            _first_guessed_impossible_word: words.len(),
        }
    }

    pub fn num_possible_words(&self) -> usize {
        self._num_possible_words
    }

    pub fn num_unguessed_words(&self) -> usize {
        self._first_guessed_impossible_word - self._first_unguessed_possible_word
    }

    /// The slice of all unguessed words. Guaranteed to start with possible words.
    pub fn unguessed_words(&self) -> &[Arc<str>] {
        &self.all_words[self._first_unguessed_possible_word..self._first_guessed_impossible_word]
    }

    /// The slice of all possible words.
    pub fn possible_words(&self) -> &[Arc<str>] {
        &self.all_words[0..self._num_possible_words]
    }

    /// Removes this word from the set of unguessed words, if it's present in the word list.
    /// This also removes the word from the list of possible words.
    pub fn remove_guess_if_present(&mut self, guess: &str) {
        // TODO: Support both by using the start of the array for possible words that have been
        // guessed.
        if let Some(position) = self
            .unguessed_words()
            .iter()
            .position(|word| word.as_ref() == guess)
            .map(|unguessed_position| unguessed_position + self._first_unguessed_possible_word)
        {
            // If it's a possible word, put it in section 1.
            if position < self._num_possible_words {
                self.all_words
                    .swap(position, self._first_unguessed_possible_word);
                self._first_unguessed_possible_word += 1;
            } else {
                // If it's an impossible word, put it in section 4.
                self.all_words
                    .swap(position, self._first_guessed_impossible_word - 1);
                self._first_guessed_impossible_word -= 1;
            }
        }
    }

    /// Filters out possible words for which the filter returns false.
    pub fn filter_possible_words<F>(&mut self, filter: F)
    where
        F: Fn(&str) -> bool,
    {
        if self._num_possible_words == 0 {
            return;
        }

        // Iterate backwards so that, in the common case, we swap the minimum numher of words.
        let mut i = self._num_possible_words - 1;
        loop {
            let word = &self.all_words[i];

            if !filter(word.as_ref()) {
                // Move this word from section 2 (possible unguessed words) to section 3 (impossible
                // unguessed words).
                self._num_possible_words -= 1;
                self.all_words.swap(i, self._num_possible_words);
            }

            if i == self._first_unguessed_possible_word {
                break;
            }
            i -= 1;
        }
        // See if there are any guessed possible words to check. This is rare.
        if i == 0 {
            return;
        }

        i -= 1;
        loop {
            let word = &self.all_words[i];

            if !filter(word.as_ref()) {
                // We're going to bump the word to the end of section 1, then end of section 2, then end of section 3.
                self._first_guessed_impossible_word -= 1;
                self._num_possible_words -= 1;
                self._first_unguessed_possible_word -= 1;
                self.all_words.swap(i, self._first_unguessed_possible_word);
                self.all_words.swap(
                    self._first_unguessed_possible_word,
                    self._num_possible_words,
                );
                self.all_words.swap(
                    self._num_possible_words,
                    self._first_guessed_impossible_word,
                );
            }
            if i == 0 {
                break;
            }
            i -= 1;
        }
    }
}

impl Display for GroupedWords {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("GroupedWords { possible, guessed: ")?;
        f.debug_list()
            .entries(self.all_words[0..self._first_unguessed_possible_word].iter())
            .finish()?;
        f.write_str("; possible, unguessed: ")?;
        f.debug_list()
            .entries(
                self.all_words[self._first_unguessed_possible_word..self._num_possible_words]
                    .iter(),
            )
            .finish()?;
        f.write_str("; impossible, guessed: ")?;
        f.debug_list()
            .entries(
                self.all_words[self._first_guessed_impossible_word..self.all_words.len()].iter(),
            )
            .finish()?;
        f.write_str("; impossible, unguessed: ")?;
        f.debug_list()
            .entries(
                self.all_words[self._num_possible_words..self._first_guessed_impossible_word]
                    .iter(),
            )
            .finish()?;
        f.write_str(" }")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_grouped_words_new() -> Result<(), WordleError> {
        let words =
            WordBank::from_iterator(&[Arc::from("the"), Arc::from("big"), Arc::from("dog")])?;
        let grouped_words = GroupedWords::new(&words);

        assert_eq!(grouped_words.all_words, words.to_vec());
        assert_eq!(grouped_words.num_possible_words(), 3);
        assert_eq!(grouped_words.num_unguessed_words(), 3);

        Ok(())
    }

    #[test]
    fn test_grouped_words_remove_guess_if_present() -> Result<(), WordleError> {
        let words =
            WordBank::from_iterator(&[Arc::from("the"), Arc::from("big"), Arc::from("dog")])?;
        let mut grouped_words = GroupedWords::new(&words);

        // Remove random word, should be no-op.
        grouped_words.remove_guess_if_present("cat");

        assert_eq!(grouped_words.num_possible_words(), 3);
        assert_eq!(grouped_words.num_unguessed_words(), 3);
        assert_eq!(grouped_words.unguessed_words(), words.to_vec());

        // Remove a present word, should remove it from unguessed words.
        grouped_words.remove_guess_if_present("big");

        assert_eq!(grouped_words.num_possible_words(), 3);
        assert_eq!(grouped_words.num_unguessed_words(), 2);
        assert_eq!(
            grouped_words.unguessed_words(),
            &[Arc::from("the"), Arc::from("dog")]
        );
        assert_eq!(
            HashSet::from_iter(grouped_words.possible_words()),
            (&words).iter().collect::<HashSet<_>>()
        );

        // Remove again, should be no-op.
        grouped_words.remove_guess_if_present("big");

        assert_eq!(grouped_words.num_possible_words(), 3);
        assert_eq!(grouped_words.num_unguessed_words(), 2);
        assert_eq!(
            grouped_words.unguessed_words(),
            &[Arc::from("the"), Arc::from("dog")]
        );

        Ok(())
    }

    #[test]
    fn test_grouped_words_filter_possible_words() -> Result<(), WordleError> {
        let words =
            WordBank::from_iterator(&[Arc::from("the"), Arc::from("big"), Arc::from("dog")])?;
        let mut grouped_words = GroupedWords::new(&words);

        grouped_words.filter_possible_words(|word| word == "big");

        assert_eq!(grouped_words.num_possible_words(), 1);
        assert_eq!(grouped_words.num_unguessed_words(), 3);
        assert_eq!(grouped_words.possible_words(), &[Arc::from("big")]);
        assert_eq!(
            HashSet::from_iter(grouped_words.unguessed_words()),
            (&words).iter().collect::<HashSet<_>>()
        );

        Ok(())
    }

    #[test]
    fn test_grouped_words_guessing_the_word() -> Result<(), WordleError> {
        let words =
            WordBank::from_iterator(&[Arc::from("the"), Arc::from("big"), Arc::from("dog")])?;
        let mut grouped_words = GroupedWords::new(&words);

        grouped_words.remove_guess_if_present("big");
        grouped_words.filter_possible_words(|word| word == "big");

        assert_eq!(grouped_words.num_possible_words(), 1);
        assert_eq!(grouped_words.num_unguessed_words(), 2);
        assert_eq!(grouped_words.possible_words(), &[Arc::from("big")]);
        assert_eq!(
            grouped_words
                .unguessed_words()
                .iter()
                .collect::<HashSet<_>>(),
            HashSet::from_iter(&[Arc::from("the"), Arc::from("dog")])
        );

        Ok(())
    }

    #[test]
    fn test_grouped_words_display() -> Result<(), WordleError> {
        let words = WordBank::from_iterator(&[
            Arc::from("the"),
            Arc::from("big"),
            Arc::from("dog"),
            Arc::from("cat"),
            Arc::from("bat"),
        ])?;
        let mut grouped_words = GroupedWords::new(&words);

        grouped_words.remove_guess_if_present("the");
        grouped_words.filter_possible_words(|word| word == "big" || word == "bat");
        grouped_words.remove_guess_if_present("big");

        assert_eq!(
            format!("{}", grouped_words),
            "GroupedWords { possible, guessed: [\"big\"]; possible, unguessed: [\"bat\"]; impossible, guessed: [\"the\"]; impossible, unguessed: [\"cat\", \"dog\"] }"
        );
        Ok(())
    }
}
