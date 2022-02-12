use crate::results::GuessResult;
use crate::results::LetterResult;
use crate::results::WordleError;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::collections::HashSet;
use std::iter::zip;
use std::result::Result;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum LocatedLetterState {
    Unknown,
    Here,
    NotHere,
}

/// Indicates information about a letter that is in the word.
#[derive(Debug, PartialEq, Eq, Clone)]
struct PresentLetter {
    /// The letter must appear exactly this many times in the word.
    maybe_required_count: Option<u8>,
    /// The number of locations we know the letter must appear.
    num_here: u8,
    /// The number of locations we know the letter must not appear.
    num_not_here: u8,
    /// The status of the letter at these locations.
    located_state: Vec<LocatedLetterState>,
}

impl PresentLetter {
    /// Constructs a `PresentLetter` for use with words of the given length.
    pub fn new(word_length: u8) -> PresentLetter {
        PresentLetter {
            maybe_required_count: None,
            num_here: 0,
            num_not_here: 0,
            located_state: vec![LocatedLetterState::Unknown; word_length as usize],
        }
    }

    pub fn state(&self, index: usize) -> LocatedLetterState {
        self.located_state[index]
    }

    pub fn maybe_required_count(&self) -> Option<u8> {
        self.maybe_required_count
    }

    /// Sets that this letter must be at the given index.
    pub fn set_must_be_at(&mut self, index: usize) -> Result<(), WordleError> {
        let previous = self.located_state[index];
        match previous {
            LocatedLetterState::Here => return Ok(()),
            LocatedLetterState::NotHere => return Err(WordleError::InvalidResults),
            _ => {}
        }
        self.located_state[index] = LocatedLetterState::Here;
        self.num_here += 1;
        if let Some(count) = self.maybe_required_count {
            if self.num_here == count {
                // If the count has been met, then this letter doesn't appear anywhere else.
                self.set_unknowns_to(LocatedLetterState::NotHere);
            } else if (self.located_state.len() as u8 - self.num_not_here) == count {
                // If the letter must be in all possible remaining spaces, set them to here.
                self.set_unknowns_to(LocatedLetterState::Here)
            }
        } else {
            // Set the max count if all states are known to prevent errors.
            self.set_required_count_if_full();
        }
        Ok(())
    }

    /// Sets that this letter must not be at the given index.
    pub fn set_must_not_be_at(&mut self, index: usize) -> Result<(), WordleError> {
        let previous = self.located_state[index];
        match previous {
            LocatedLetterState::NotHere => return Ok(()),
            LocatedLetterState::Here => return Err(WordleError::InvalidResults),
            _ => {}
        }
        self.located_state[index] = LocatedLetterState::NotHere;
        self.num_not_here += 1;
        let max_possible_here = self.located_state.len() as u8 - self.num_not_here;
        if let Some(count) = self.maybe_required_count {
            if max_possible_here == count {
                // If the letter must be in all possible remaining spaces, set them to here.
                self.set_unknowns_to(LocatedLetterState::Here);
            }
        } else {
            if max_possible_here == 1 && self.num_here == 0 {
                self.set_unknowns_to(LocatedLetterState::Here);
                self.maybe_required_count = Some(1);
            } else {
                // Set the max count if all states are known to prevent errors.
                self.set_required_count_if_full();
            }
        }
        Ok(())
    }

    /// Sets the maximum number of times this letter can appear in the word.
    pub fn set_required_count(&mut self, count: u8) -> Result<(), WordleError> {
        if let Some(existing_count) = self.maybe_required_count {
            if existing_count != count {
                return Err(WordleError::InvalidResults);
            } else {
                return Ok(());
            }
        }
        if self.num_here > count {
            return Err(WordleError::InvalidResults);
        }
        let max_possible_num_here = self.located_state.len() as u8 - self.num_not_here;
        if max_possible_num_here < count {
            return Err(WordleError::InvalidResults);
        }
        self.maybe_required_count = Some(count);
        if self.num_here == count {
            self.set_unknowns_to(LocatedLetterState::NotHere);
        } else if max_possible_num_here == count {
            self.set_unknowns_to(LocatedLetterState::Here);
        }
        Ok(())
    }

    fn set_unknowns_to(&mut self, new_state: LocatedLetterState) {
        let mut count_to_update = &mut self.num_here;
        if new_state == LocatedLetterState::NotHere {
            count_to_update = &mut self.num_not_here;
        }
        for state in &mut self.located_state {
            match state {
                LocatedLetterState::Unknown => {
                    *state = new_state;
                    *count_to_update += 1;
                }
                _ => {}
            }
        }
    }

    fn set_required_count_if_full(&mut self) {
        if self.num_here + self.num_not_here == self.located_state.len() as u8 {
            self.maybe_required_count = Some(self.num_here);
        }
    }
}

/// Defines letter restrictions that a word must adhere to.
pub struct WordRestrictions {
    word_length: u8,
    present_letters: HashMap<char, PresentLetter>,
    not_present_letters: HashSet<char>,
}

impl WordRestrictions {
    /// Creates a `WordRestrictions` object for the given word length with all letters unknown.
    fn new(word_length: u8) -> WordRestrictions {
        WordRestrictions {
            word_length: word_length,
            present_letters: HashMap::new(),
            not_present_letters: HashSet::new(),
        }
    }

    /// Returns the restrictions imposed by the given result.
    pub fn from_result(result: &GuessResult) -> Result<WordRestrictions, WordleError> {
        let mut restrictions = WordRestrictions::new(result.guess.len() as u8);
        restrictions.update(result)?;
        Ok(restrictions)
    }

    /// Adds restrictions arising from the given guess result.
    pub fn update(&mut self, guess_result: &GuessResult) -> Result<(), WordleError> {
        for ((index, letter), result) in zip(
            guess_result.guess.char_indices(),
            guess_result.results.iter(),
        ) {
            match result {
                LetterResult::Correct => {
                    self.set_letter_here(letter, index, guess_result)?;
                }
                LetterResult::PresentNotHere => {
                    self.set_letter_present_not_here(letter, index, guess_result)?;
                }
                LetterResult::NotPresent => {
                    self.set_letter_not_present(letter, index, guess_result)?;
                }
            }
        }
        Ok(())
    }

    fn set_letter_here(
        &mut self,
        letter: char,
        location: usize,
        result: &GuessResult,
    ) -> Result<(), WordleError> {
        let mut presence = self
            .present_letters
            .entry(letter)
            .or_insert(PresentLetter::new(self.word_length));
        presence.set_must_be_at(location)?;
        // Remove from the not present letters if it was present. This could happen if the guess
        // included the letter in two places, but the correct word only included it in the latter
        // place.
        if self.not_present_letters.remove(&letter) {
            return WordRestrictions::set_required_count(letter, result, presence);
        }
        Ok(())
    }

    fn set_letter_present_not_here(
        &mut self,
        letter: char,
        location: usize,
        result: &GuessResult,
    ) -> Result<(), WordleError> {
        let mut presence = self
            .present_letters
            .entry(letter)
            .or_insert(PresentLetter::new(self.word_length));
        presence.set_must_not_be_at(location)?;
        // Remove from the not present letters if it was present. This could happen if the guess
        // included the letter in two places, but the correct word only included it in the latter
        // place.
        if self.not_present_letters.remove(&letter) {
            return WordRestrictions::set_required_count(letter, result, presence);
        }
        Ok(())
    }

    fn set_letter_not_present(
        &mut self,
        letter: char,
        location: usize,
        result: &GuessResult,
    ) -> Result<(), WordleError> {
        if let Entry::Occupied(mut presence_entry) = self.present_letters.entry(letter) {
            let mut presence = presence_entry.get_mut();
            if presence.state(location) == LocatedLetterState::Here {
                return Err(WordleError::InvalidResults);
            }
            return WordRestrictions::set_required_count(letter, result, presence);
        }
        self.not_present_letters.insert(letter);
        Ok(())
    }

    fn set_required_count(
        letter: char,
        guess_result: &GuessResult,
        presence: &mut PresentLetter,
    ) -> Result<(), WordleError> {
        // If the letter is present, but this result was `NotPresent`, then it means it's
        // only in the word as many times as it was given a `Correct` or  `PresentNotHere`
        // hint.
        let num_times_present = guess_result
            .guess
            .char_indices()
            .filter(|(index, other_letter)| {
                *other_letter == letter
                    && *guess_result.results.get(*index).unwrap() != LetterResult::NotPresent
            })
            .count();
        presence.set_required_count(num_times_present as u8)
    }

    // /// Adds the given restrictions to this restriction.
    // pub fn union(&mut self, other: &WordRestrictions) {
    //     self.must_contain_here
    //         .extend(other.must_contain_here.iter().map(Clone::clone));
    //     self.must_contain_but_not_here
    //         .extend(other.must_contain_but_not_here.iter().map(Clone::clone));
    //     self.must_not_contain
    //         .extend(other.must_not_contain.iter().map(Clone::clone));
    // }

    /// Returns `true` iff the given word satisfies these restrictions.
    pub fn is_satisfied_by(&self, word: &str) -> bool {
        word.len() == self.word_length as usize
            && self.present_letters.iter().all(|(letter, presence)| {
                let mut count_found = 0;
                for (index, word_letter) in word.char_indices() {
                    if word_letter == *letter {
                        count_found += 1;
                        if presence.state(index) == LocatedLetterState::NotHere {
                            return false;
                        }
                    } else if presence.state(index) == LocatedLetterState::Here {
                        return false;
                    }
                }
                if let Some(required_count) = presence.maybe_required_count() {
                    return count_found == required_count;
                }
                count_found != 0
            })
            && word
                .chars()
                .all(|letter| !self.not_present_letters.contains(&letter))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn present_letter_constructor() -> Result<(), WordleError> {
        let letter = PresentLetter::new(3);

        assert_eq!(letter.state(0), LocatedLetterState::Unknown);
        assert_eq!(letter.state(1), LocatedLetterState::Unknown);
        assert_eq!(letter.state(2), LocatedLetterState::Unknown);
        Ok(())
    }

    #[test]
    fn present_letter_set_here() -> Result<(), WordleError> {
        let mut letter = PresentLetter::new(3);

        letter.set_must_be_at(1)?;

        assert_eq!(letter.state(0), LocatedLetterState::Unknown);
        assert_eq!(letter.state(1), LocatedLetterState::Here);
        assert_eq!(letter.state(2), LocatedLetterState::Unknown);
        Ok(())
    }

    #[test]
    fn present_letter_set_here_can_be_repeated() -> Result<(), WordleError> {
        let mut letter = PresentLetter::new(3);

        letter.set_must_be_at(1)?;
        letter.set_must_be_at(1)?;
        letter.set_must_be_at(1)?;
        letter.set_must_be_at(1)?;

        assert_eq!(letter.state(0), LocatedLetterState::Unknown);
        assert_eq!(letter.state(1), LocatedLetterState::Here);
        assert_eq!(letter.state(2), LocatedLetterState::Unknown);
        Ok(())
    }

    #[test]
    fn present_letter_set_not_here() -> Result<(), WordleError> {
        let mut letter = PresentLetter::new(3);

        letter.set_must_not_be_at(1)?;

        assert_eq!(letter.state(0), LocatedLetterState::Unknown);
        assert_eq!(letter.state(1), LocatedLetterState::NotHere);
        assert_eq!(letter.state(2), LocatedLetterState::Unknown);
        Ok(())
    }

    #[test]
    fn present_letter_set_not_here_can_be_repeated() -> Result<(), WordleError> {
        let mut letter = PresentLetter::new(3);

        letter.set_must_not_be_at(1)?;
        letter.set_must_not_be_at(1)?;
        letter.set_must_not_be_at(1)?;
        letter.set_must_not_be_at(1)?;

        assert_eq!(letter.state(0), LocatedLetterState::Unknown);
        assert_eq!(letter.state(1), LocatedLetterState::NotHere);
        assert_eq!(letter.state(2), LocatedLetterState::Unknown);
        Ok(())
    }

    #[test]
    fn present_letter_infer_must_be_here() -> Result<(), WordleError> {
        let mut letter = PresentLetter::new(3);

        letter.set_must_not_be_at(1)?;
        letter.set_must_not_be_at(2)?;

        assert_eq!(letter.state(0), LocatedLetterState::Here);
        assert_eq!(letter.state(1), LocatedLetterState::NotHere);
        assert_eq!(letter.state(2), LocatedLetterState::NotHere);
        Ok(())
    }

    #[test]
    fn present_letter_must_be_here_whole_word() -> Result<(), WordleError> {
        let mut letter = PresentLetter::new(3);

        letter.set_must_be_at(0)?;
        letter.set_must_be_at(1)?;
        letter.set_must_be_at(2)?;

        assert_eq!(letter.state(0), LocatedLetterState::Here);
        assert_eq!(letter.state(1), LocatedLetterState::Here);
        assert_eq!(letter.state(2), LocatedLetterState::Here);
        Ok(())
    }

    #[test]
    fn present_letter_max_count_then_here_fills_remainder_not_here() -> Result<(), WordleError> {
        let mut letter = PresentLetter::new(3);

        letter.set_required_count(2)?;
        letter.set_must_be_at(1)?;

        assert_eq!(letter.state(0), LocatedLetterState::Unknown);
        assert_eq!(letter.state(1), LocatedLetterState::Here);
        assert_eq!(letter.state(2), LocatedLetterState::Unknown);

        // Same location, no change.
        letter.set_must_be_at(1)?;

        assert_eq!(letter.state(0), LocatedLetterState::Unknown);
        assert_eq!(letter.state(1), LocatedLetterState::Here);
        assert_eq!(letter.state(2), LocatedLetterState::Unknown);

        letter.set_must_be_at(0)?;
        assert_eq!(letter.state(0), LocatedLetterState::Here);
        assert_eq!(letter.state(1), LocatedLetterState::Here);
        assert_eq!(letter.state(2), LocatedLetterState::NotHere);
        Ok(())
    }

    #[test]
    fn present_letter_here_then_max_count_fills_remainder_not_here() -> Result<(), WordleError> {
        let mut letter = PresentLetter::new(3);

        letter.set_must_be_at(1)?;
        letter.set_required_count(1)?;

        assert_eq!(letter.state(0), LocatedLetterState::NotHere);
        assert_eq!(letter.state(1), LocatedLetterState::Here);
        assert_eq!(letter.state(2), LocatedLetterState::NotHere);
        Ok(())
    }

    #[test]
    fn present_letter_max_count_then_not_here_fills_remainder_here() -> Result<(), WordleError> {
        let mut letter = PresentLetter::new(3);

        letter.set_must_be_at(1)?;
        letter.set_required_count(2)?;
        letter.set_must_not_be_at(0)?;

        assert_eq!(letter.state(0), LocatedLetterState::NotHere);
        assert_eq!(letter.state(1), LocatedLetterState::Here);
        assert_eq!(letter.state(2), LocatedLetterState::Here);
        Ok(())
    }

    #[test]
    fn present_letter_max_count_less_than_here_errors() -> Result<(), WordleError> {
        let mut letter = PresentLetter::new(3);

        letter.set_must_be_at(0)?;
        letter.set_must_be_at(1)?;
        assert_eq!(
            letter.set_required_count(1),
            Err(WordleError::InvalidResults)
        );
        Ok(())
    }

    #[test]
    fn present_letter_max_count_more_than_possible_errors() -> Result<(), WordleError> {
        let mut letter = PresentLetter::new(3);

        letter.set_must_not_be_at(0)?;
        letter.set_must_not_be_at(1)?;
        assert_eq!(
            letter.set_required_count(2),
            Err(WordleError::InvalidResults)
        );
        Ok(())
    }

    #[test]
    fn present_letter_here_after_not_here_errors() -> Result<(), WordleError> {
        let mut letter = PresentLetter::new(3);

        letter.set_must_not_be_at(0)?;
        assert_eq!(letter.set_must_be_at(0), Err(WordleError::InvalidResults));
        Ok(())
    }

    #[test]
    fn present_letter_not_here_after_here_errors() -> Result<(), WordleError> {
        let mut letter = PresentLetter::new(3);

        letter.set_must_be_at(0)?;
        assert_eq!(
            letter.set_must_not_be_at(0),
            Err(WordleError::InvalidResults)
        );
        Ok(())
    }

    #[test]
    fn word_restrictions_is_satisfied_by_no_restrictions() {
        let restrictions = WordRestrictions::new(4);

        assert!(restrictions.is_satisfied_by("abcd"));
        assert!(restrictions.is_satisfied_by("zzzz"));

        // Wrong length
        assert_eq!(restrictions.is_satisfied_by(""), false);
        assert_eq!(restrictions.is_satisfied_by("abcde"), false);
    }

    #[test]
    fn word_restrictions_is_satisfied_by_with_restrictions() -> Result<(), WordleError> {
        let mut restrictions = WordRestrictions::new(4);

        restrictions.update(&GuessResult {
            guess: "abbc",
            results: vec![
                LetterResult::PresentNotHere,
                LetterResult::PresentNotHere,
                LetterResult::Correct,
                LetterResult::NotPresent,
            ],
        })?;

        assert!(restrictions.is_satisfied_by("bdba"));
        assert!(restrictions.is_satisfied_by("dabb"));

        assert_eq!(restrictions.is_satisfied_by("bbba"), false);
        assert_eq!(restrictions.is_satisfied_by("bcba"), false);
        assert_eq!(restrictions.is_satisfied_by("adbd"), false);
        assert_eq!(restrictions.is_satisfied_by("bdbd"), false);
        Ok(())
    }

    #[test]
    fn word_restrictions_is_satisfied_by_with_known_required_count() -> Result<(), WordleError> {
        let mut restrictions = WordRestrictions::new(4);

        restrictions.update(&GuessResult {
            guess: "abbc",
            results: vec![
                LetterResult::PresentNotHere,
                LetterResult::NotPresent,
                LetterResult::Correct,
                LetterResult::NotPresent,
            ],
        })?;

        assert!(restrictions.is_satisfied_by("edba"));
        assert!(restrictions.is_satisfied_by("dabe"));
        assert!(restrictions.is_satisfied_by("daba"));

        assert_eq!(restrictions.is_satisfied_by("bdba"), false);
        assert_eq!(restrictions.is_satisfied_by("dcba"), false);
        assert_eq!(restrictions.is_satisfied_by("adbd"), false);
        Ok(())
    }
}
