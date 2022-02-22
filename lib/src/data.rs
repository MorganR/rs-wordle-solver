use crate::results::*;
use std::collections::HashMap;
use std::hash::Hash;
use std::io;
use std::ops::Deref;
use std::rc::Rc;
use std::result::Result;

/// A letter along with its location in the word.
///
/// ```
/// use wordle_solver::details::LocatedLetter;
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

/// Contains all the possible words for a Wordle game.
#[derive(Debug)]
pub struct WordBank {
    all_words: Vec<Rc<str>>,
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
    /// use wordle_solver::WordBank;
    /// # use wordle_solver::WordleError;
    ///
    /// let words_reader = io::BufReader::new(File::open("path/to/my/words.txt")?);
    /// let word_bank = WordBank::from_reader(words_reader)?;
    /// # Ok::<(), WordleError>(())
    /// ```
    pub fn from_reader<R: io::BufRead>(word_reader: R) -> Result<Self, WordleError> {
        let mut word_length = 0;
        let all_words = word_reader
            .lines()
            .map(|maybe_word| {
                maybe_word.map_err(WordleError::from).and_then(|word| {
                    let this_word_length = word.len();
                    if word_length == 0 && this_word_length != 0 {
                        word_length = this_word_length;
                    } else if word_length != this_word_length {
                        return Err(WordleError::WordLength(word_length));
                    }
                    Ok(Rc::from(word.to_lowercase().as_str()))
                })
            })
            .filter(|maybe_word| {
                maybe_word
                    .as_ref()
                    .map_or(true, |word: &Rc<str>| word.len() > 0)
            })
            .collect::<Result<Vec<Rc<str>>, WordleError>>()?;
        Ok(WordBank {
            all_words,
            word_length,
        })
    }

    // TODO: remove this.
    pub fn from_vec(words: Vec<String>) -> Result<Self, WordleError> {
        let mut word_length = 0;
        Ok(WordBank {
            all_words: words
                .iter()
                .filter_map(|word| {
                    let normalized: Rc<str> = Rc::from(word.trim().to_lowercase().as_str());
                    let this_word_length = normalized.len();
                    if this_word_length == 0 {
                        return None;
                    }
                    if word_length == 0 {
                        word_length = this_word_length;
                    } else if word_length != this_word_length {
                        return Some(Err(WordleError::WordLength(word_length)));
                    }
                    Some(Ok(normalized))
                })
                .collect::<Result<Vec<Rc<str>>, WordleError>>()?,
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
    /// use std::rc::Rc;
    /// use wordle_solver::WordBank;
    /// # use wordle_solver::WordleError;
    ///
    /// let words = ["abc", "DEF "];
    /// let word_bank = WordBank::from_slice(&words)?;
    ///
    /// assert_eq!(&word_bank as &[Rc<str>], &[Rc::from("abc"), Rc::from("def")]);
    /// # Ok::<(), WordleError>(())
    /// ```
    pub fn from_slice<S>(words: &[S]) -> Result<Self, WordleError>
    where
        S: AsRef<str>,
    {
        WordBank::from_iter_private(words.iter())
    }

    /// Constructs a new `WordBank` struct using the words from the given vector. Each word will be
    /// trimmed and converted to lower case.
    ///
    /// After trimming, all words must be the same length, else this returns an error of type
    /// [`WordleError::WordLength`].
    ///
    /// ```
    /// use std::rc::Rc;
    /// use wordle_solver::WordBank;
    /// # use wordle_solver::WordleError;
    ///
    /// let words = vec!["abc".to_string(), "DEF ".to_string()];
    /// let word_bank = WordBank::from_iter(words.iter())?;
    ///
    /// assert_eq!(&word_bank as &[Rc<str>], &[Rc::from("abc"), Rc::from("def")]);
    /// # Ok::<(), WordleError>(())
    /// ```
    pub fn from_iter<S>(words: impl Iterator<Item = S>) -> Result<Self, WordleError>
    where
        S: AsRef<str>,
    {
        WordBank::from_iter_private(words.into_iter())
    }

    fn from_iter_private<S>(words: impl Iterator<Item = S>) -> Result<Self, WordleError>
    where
        S: AsRef<str>,
    {
        let mut word_length = 0;
        Ok(WordBank {
            all_words: words
                .filter_map(|word| {
                    let normalized: Rc<str> =
                        Rc::from(word.as_ref().trim().to_lowercase().as_str());
                    let this_word_length = normalized.len();
                    if this_word_length == 0 {
                        return None;
                    }
                    if word_length == 0 {
                        word_length = this_word_length;
                    } else if word_length != this_word_length {
                        return Some(Err(WordleError::WordLength(word_length)));
                    }
                    Some(Ok(normalized))
                })
                .collect::<Result<Vec<Rc<str>>, WordleError>>()?,
            word_length,
        })
    }

    /// Returns the number of possible words.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.all_words.len()
    }

    /// Returns true iff this word bank is empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.all_words.is_empty()
    }

    /// Returns the length of each word in the word bank.
    #[inline(always)]
    pub fn word_length(&self) -> usize {
        self.word_length
    }
}

impl Deref for WordBank {
    type Target = [Rc<str>];

    /// Derefs the list of words in the `WordBank` as a slice.
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
/// # use std::rc::Rc;
/// # use wordle_solver::WordCounter;
/// # use wordle_solver::details::LocatedLetter;
/// let all_words = vec![Rc::from("aba"), Rc::from("bbd"), Rc::from("efg")];
/// let counter = WordCounter::new(&all_words);
///  
/// assert_eq!(counter.num_words(), 3);
/// assert_eq!(counter.num_words_with_letter('b'), 2);
/// assert_eq!(counter.num_words_with_located_letter(
///     &LocatedLetter::new('b', 0)), 1);
/// ```
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
    ///
    /// ```
    /// use std::rc::Rc;
    /// use wordle_solver::WordCounter;
    /// use wordle_solver::details::LocatedLetter;
    ///
    /// let all_words = vec![Rc::from("aba"), Rc::from("bbd"), Rc::from("efg")];
    /// let counter = WordCounter::new(&all_words);
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
    /// use std::rc::Rc;
    /// use wordle_solver::WordCounter;
    ///
    /// let all_words = vec![Rc::from("aba"), Rc::from("bbd"), Rc::from("efg")];
    /// let counter = WordCounter::new(&all_words);
    ///  
    /// assert_eq!(counter.num_words_with_letter('a'), 1);
    /// assert_eq!(counter.num_words_with_letter('b'), 2);
    /// assert_eq!(counter.num_words_with_letter('z'), 0);
    /// ```
    pub fn num_words_with_letter(&self, letter: char) -> u32 {
        *self.num_words_by_letter.get(&letter).unwrap_or(&0)
    }

    /// Retrieves the total number of words in this counter.
    pub fn num_words(&self) -> u32 {
        self.num_words
    }
}

/// Computes the unique set of words that contain each letter anywhere, as well as by the location
/// of each letter.
///
/// If you only need to know the number of words instead of the list of words, see [`WordCounter`].
///
/// ```
/// use std::rc::Rc;
/// use wordle_solver::details::WordTracker;
/// use wordle_solver::details::LocatedLetter;
///
/// let all_words = vec![Rc::from("aba"), Rc::from("bcd"), Rc::from("efg")];
/// let tracker = WordTracker::new(&all_words);
///
/// assert_eq!(tracker.all_words(), &all_words);
/// assert_eq!(
///     Vec::from_iter(tracker.words_with_letter('b')),
///     vec![&Rc::from("aba"), &Rc::from("bcd")]);
/// assert_eq!(
///     Vec::from_iter(tracker.words_with_located_letter(LocatedLetter::new('b', 1))),
///     vec![&Rc::from("aba")]);
/// ```
pub struct WordTracker {
    empty_list: Vec<Rc<str>>,
    all_words: Vec<Rc<str>>,
    words_by_letter: HashMap<char, Vec<Rc<str>>>,
    words_by_located_letter: HashMap<LocatedLetter, Vec<Rc<str>>>,
}

impl WordTracker {
    /// Constructs a new `WordTracker` from the given words. Note that the words are not checked
    /// for uniqueness, so if duplicates exist in the given words, then those duplicates will
    /// remain part of this tracker's information.
    pub fn new<'a, I>(words: I) -> WordTracker
    where
        I: IntoIterator<Item = &'a Rc<str>>,
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
        }
    }

    /// Retrieves the full list of words stored in this word tracker.
    ///
    /// ```
    /// use std::rc::Rc;
    /// use wordle_solver::details::WordTracker;
    ///
    /// let all_words = vec![Rc::from("aba"), Rc::from("bcd"), Rc::from("efg")];
    /// let tracker = WordTracker::new(&all_words);
    ///
    /// assert_eq!(tracker.all_words(), &all_words);
    /// ```
    pub fn all_words(&self) -> &[Rc<str>] {
        &self.all_words
    }

    /// Returns true iff any of the words in this tracker contain the given letter.
    ///
    /// ```
    /// use std::rc::Rc;
    /// use wordle_solver::details::WordTracker;
    ///
    /// let all_words = vec![Rc::from("aba"), Rc::from("bcd"), Rc::from("efg")];
    /// let tracker = WordTracker::new(&all_words);
    ///
    /// assert!(tracker.has_letter('a'));
    /// assert!(!tracker.has_letter('z'));
    /// ```
    pub fn has_letter(&self, letter: char) -> bool {
        self.words_by_letter.contains_key(&letter)
    }

    /// Returns an [`Iterator`] over words that have the given letter at the given location.
    ///
    /// ```
    /// use std::rc::Rc;
    /// use wordle_solver::details::WordTracker;
    /// use wordle_solver::details::LocatedLetter;
    ///
    /// let all_words = vec![Rc::from("bba"), Rc::from("bcd"), Rc::from("efg")];
    /// let tracker = WordTracker::new(&all_words);
    ///
    /// assert_eq!(
    ///     Vec::from_iter(tracker.words_with_located_letter(LocatedLetter::new('b', 0))),
    ///     vec![&Rc::from("bba"), &Rc::from("bcd")]);
    /// assert_eq!(
    ///     Vec::from_iter(tracker.words_with_located_letter(LocatedLetter::new('b', 1))),
    ///     vec![&Rc::from("bba")]);
    /// assert_eq!(
    ///     tracker.words_with_located_letter(LocatedLetter::new('z', 1)).count(),
    ///     0);
    /// ```
    pub fn words_with_located_letter(&self, ll: LocatedLetter) -> impl Iterator<Item = &Rc<str>> {
        self.words_by_located_letter
            .get(&ll)
            .map(|words| words.iter())
            .unwrap_or_else(|| self.empty_list.iter())
    }

    /// Returns an [`Iterator`] over words that have the given letter.
    ///
    /// ```
    /// use std::rc::Rc;
    /// use wordle_solver::details::WordTracker;
    ///
    /// let all_words = vec![Rc::from("bba"), Rc::from("bcd"), Rc::from("efg")];
    /// let tracker = WordTracker::new(&all_words);
    ///
    /// assert_eq!(
    ///     Vec::from_iter(tracker.words_with_letter('b')),
    ///     vec![&Rc::from("bba"), &Rc::from("bcd")]);
    /// assert_eq!(
    ///     Vec::from_iter(tracker.words_with_letter('e')),
    ///     vec![&Rc::from("efg")]);
    /// assert_eq!(
    ///     tracker.words_with_letter('z').count(),
    ///     0);
    /// ```
    pub fn words_with_letter(&self, letter: char) -> impl Iterator<Item = &Rc<str>> {
        self.words_by_letter
            .get(&letter)
            .map(|words| words.iter())
            .unwrap_or_else(|| self.empty_list.iter())
    }

    /// Returns an [`Iterator`] over words that have the given letter, but not at the given
    /// location.
    ///
    /// ```
    /// use std::rc::Rc;
    /// use wordle_solver::details::WordTracker;
    /// use wordle_solver::details::LocatedLetter;
    ///
    /// let all_words = vec![Rc::from("bba"), Rc::from("bcd"), Rc::from("efg")];
    /// let tracker = WordTracker::new(&all_words);
    ///
    /// assert_eq!(
    ///     Vec::from_iter(tracker.words_with_letter_not_here(LocatedLetter::new('b', 1))),
    ///     vec![&Rc::from("bcd")]);
    /// assert_eq!(
    ///     tracker.words_with_letter_not_here(LocatedLetter::new('b', 0)).count(),
    ///     0);
    /// assert_eq!(
    ///     tracker.words_with_letter_not_here(LocatedLetter::new('z', 0)).count(),
    ///     0);
    /// ```
    pub fn words_with_letter_not_here(&self, ll: LocatedLetter) -> impl Iterator<Item = &Rc<str>> {
        let words_with_letter: &Vec<Rc<str>> = self
            .words_by_letter
            .get(&ll.letter)
            .unwrap_or(&self.empty_list);
        words_with_letter
            .iter()
            .filter(move |&word| word.chars().nth(ll.location as usize).unwrap() != ll.letter)
    }

    /// Returns an [`Iterator`] over words that don't have the given letter.
    ///
    /// ```
    /// use std::rc::Rc;
    /// use wordle_solver::details::WordTracker;
    /// use wordle_solver::details::LocatedLetter;
    ///
    /// let all_words = vec![Rc::from("bba"), Rc::from("bcd"), Rc::from("efg")];
    /// let tracker = WordTracker::new(&all_words);
    ///
    /// assert_eq!(
    ///     Vec::from_iter(tracker.words_without_letter('a')),
    ///     vec![&Rc::from("bcd"), &Rc::from("efg")]);
    /// assert_eq!(
    ///     Vec::from_iter(tracker.words_without_letter('z')),
    ///     Vec::from_iter(&all_words));
    /// ```
    pub fn words_without_letter(&self, letter: char) -> impl Iterator<Item = &Rc<str>> {
        self.all_words
            .iter()
            .filter(move |word| !word.contains(letter))
    }
}

impl Clone for WordTracker {
    fn clone(&self) -> WordTracker {
        WordTracker {
            empty_list: Vec::new(),
            all_words: self.all_words.clone(),
            words_by_located_letter: self.words_by_located_letter.clone(),
            words_by_letter: self.words_by_letter.clone(),
        }
    }
}

/// A compressed form of LetterResults. Can only store vectors of up to 10 results.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct CompressedGuessResult {
    data: u32,
}

const MAX_LETTERS_IN_COMPRESSED_GUESS_RESULT: usize = std::mem::size_of::<u32>() * 8 / 3;

impl CompressedGuessResult {
    /// Creates a compressed form of the given letter results.
    ///
    /// Returns a [`WordleError::WordLength`] error if `letter_results` has more than 10 values.
    pub fn from_result(
        letter_results: &[LetterResult],
    ) -> std::result::Result<CompressedGuessResult, WordleError> {
        if letter_results.len() > MAX_LETTERS_IN_COMPRESSED_GUESS_RESULT {
            return Err(WordleError::WordLength(
                MAX_LETTERS_IN_COMPRESSED_GUESS_RESULT,
            ));
        }
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
        Ok(Self { data })
    }
}

/// Precomputes and stores all the results for each (objective, guess) pair from a given set of
/// words.
///
/// Results are stored in a small form factor, but memory use scales according to
/// *O*(*n*<sup>2</sup>), where *n* is the number of words.
#[derive(Clone)]
pub struct PrecomputedGuessResults {
    results_by_objective_guess_pair: HashMap<(Rc<str>, Rc<str>), CompressedGuessResult>,
}

impl PrecomputedGuessResults {
    /// Precomputes and stores all the results for each (objective, guess) pair.
    ///
    /// This is an expensive operation, that scales at roughly *O*(*n*<sup>2</sup>), where *n* is
    /// the number of words, in both compute and memory.
    ///
    /// Words must be 10 characters or less.
    pub fn compute(all_words: &[Rc<str>]) -> std::result::Result<Self, WordleError> {
        let mut results_by_objective_guess_pair: HashMap<
            (Rc<str>, Rc<str>),
            CompressedGuessResult,
        > = HashMap::new();
        for objective in all_words {
            for guess in all_words {
                results_by_objective_guess_pair.insert(
                    (objective.clone(), guess.clone()),
                    CompressedGuessResult::from_result(
                        &get_result_for_guess(objective, guess)?.results,
                    )?,
                );
            }
        }
        Ok(PrecomputedGuessResults {
            results_by_objective_guess_pair,
        })
    }

    /// Retrieves the result of applying the given guess to the given objective.
    ///
    /// ```
    /// use std::rc::Rc;
    /// use wordle_solver::LetterResult;
    /// use wordle_solver::details::CompressedGuessResult;
    /// use wordle_solver::details::PrecomputedGuessResults;
    ///
    /// let all_words = vec![Rc::from("abb"), Rc::from("bcd")];
    /// let results = PrecomputedGuessResults::compute(&all_words).unwrap();
    ///
    /// assert_eq!(
    ///     results.get_result(&all_words[0], &all_words[1]),
    ///     Some(CompressedGuessResult::from_result(&[
    ///         LetterResult::PresentNotHere,
    ///         LetterResult::NotPresent,
    ///         LetterResult::NotPresent]).unwrap()));
    /// assert_eq!(
    ///     results.get_result(&all_words[1], &all_words[0]),
    ///     Some(CompressedGuessResult::from_result(&[
    ///         LetterResult::NotPresent,
    ///         LetterResult::PresentNotHere,
    ///         LetterResult::NotPresent]).unwrap()));
    /// ```
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
