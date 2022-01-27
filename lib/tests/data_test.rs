use wordle_solver::data::*;

use std::collections::hash_set::HashSet;
use std::io::Cursor;
use std::io::Result;
use std::rc::Rc;

#[test]
fn possible_words_len_succeeds() -> Result<()> {
    let mut cursor = Cursor::new(String::from("worda\nwordb"));

    let possible_words = PossibleWords::new(&mut cursor)?;

    assert_eq!(possible_words.len(), 2);
    Ok(())
}

#[test]
fn possible_words_by_located_letter_succeeds() -> Result<()> {
    let mut cursor = Cursor::new(String::from("worda\nwordb"));

    let possible_words = PossibleWords::new(&mut cursor)?;

    if let Some(words) = possible_words.by_located_letter(&LocatedLetter::new('w', 0)) {
        assert_eq!(
            words,
            &HashSet::from([
                Rc::new(String::from("worda")),
                Rc::new(String::from("wordb"))
            ])
        );
    } else {
        assert!(false);
    }
    if let Some(words) = possible_words.by_located_letter(&LocatedLetter::new('a', 4)) {
        assert_eq!(words, &HashSet::from([Rc::new(String::from("worda"))]));
    } else {
        assert!(false);
    }
    assert!(possible_words
        .by_located_letter(&LocatedLetter::new('z', 0))
        .is_none());
    assert!(possible_words
        .by_located_letter(&LocatedLetter::new('w', 1))
        .is_none());
    Ok(())
}
