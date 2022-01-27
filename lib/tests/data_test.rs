use wordle_solver::*;

use std::io::Cursor;
use std::io::Result;

#[test]
fn possible_words_len_succeeds() -> Result<()> {
    let mut cursor = Cursor::new(String::from("worda\nwordb"));

    let possible_words = PossibleWords::new(&mut cursor)?;

    assert_eq!(possible_words.len(), 2);
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