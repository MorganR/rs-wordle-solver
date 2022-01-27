use wordle_solver::*;

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

#[test]
fn possible_words_get_possible_words_must_contain_here() -> Result<()> {
    let mut cursor = Cursor::new(String::from("worda\nwordb\nother\nsmore"));

    let possible_words = PossibleWords::new(&mut cursor)?;

    let still_possible = possible_words.get_possible_words(&WordRestrictions {
        must_contain_here: vec![LocatedLetter::new('o', 1), LocatedLetter::new('b', 4)],
        must_contain_but_not_here: vec![],
        must_not_contain: vec![],
    });

    assert_eq!(still_possible, vec!["wordb"]);
    Ok(())
}

#[test]
fn possible_words_get_possible_words_must_contain_not_here() -> Result<()> {
    let mut cursor = Cursor::new(String::from("worda\nwordb\nother\nsmore"));

    let possible_words = PossibleWords::new(&mut cursor)?;

    let still_possible = possible_words.get_possible_words(&WordRestrictions {
        must_contain_here: vec![],
        must_contain_but_not_here: vec![LocatedLetter::new('o', 0)],
        must_not_contain: vec![],
    });

    assert_eq!(still_possible, vec!["worda", "wordb", "smore"]);
    Ok(())
}

#[test]
fn possible_words_get_possible_words_must_not_contain() -> Result<()> {
    let mut cursor = Cursor::new(String::from("worda\nwordb\nother\nsmore"));

    let possible_words = PossibleWords::new(&mut cursor)?;

    let still_possible = possible_words.get_possible_words(&WordRestrictions {
        must_contain_here: vec![],
        must_contain_but_not_here: vec![],
        must_not_contain: vec!['w'],
    });

    assert_eq!(still_possible, vec!["other", "smore"]);
    Ok(())
}

#[test]
fn possible_words_get_possible_words_no_match() -> Result<()> {
    let mut cursor = Cursor::new(String::from("worda\nwordb\nother\nsmore"));

    let possible_words = PossibleWords::new(&mut cursor)?;

    let still_possible: Vec<&str> = possible_words.get_possible_words(&WordRestrictions {
        must_contain_here: vec![LocatedLetter::new('o', 1)],
        must_contain_but_not_here: vec![LocatedLetter::new('b', 4)],
        must_not_contain: vec!['w'],
    });

    let empty: Vec<&str> = Vec::new();
    assert_eq!(still_possible, empty);
    Ok(())
}