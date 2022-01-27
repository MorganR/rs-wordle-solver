use wordle_solver::*;

use std::io::Cursor;
use std::io::Result;

#[test]
fn word_bank_from_reader_succeeds() -> Result<()> {
    let mut cursor = Cursor::new(String::from("worda\nwordb"));

    let word_bank = WordBank::from_reader(&mut cursor)?;

    assert_eq!(word_bank.len(), 2);
    Ok(())
}

#[test]
fn word_bank_from_vec_succeeds() {
    let words: Vec<String> = vec![String::from("worda"), String::from("wordb")];
    let word_bank = WordBank::from_vec(words);

    assert_eq!(word_bank.len(), 2);
}

#[test]
fn word_bank_get_word_bank_must_contain_here() -> Result<()> {
    let mut cursor = Cursor::new(String::from("worda\nwordb\nother\nsmore"));

    let word_bank = WordBank::from_reader(&mut cursor)?;

    let still_possible = word_bank.get_possible_words(&WordRestrictions {
        must_contain_here: vec![LocatedLetter::new('o', 1), LocatedLetter::new('b', 4)],
        must_contain_but_not_here: vec![],
        must_not_contain: vec![],
    });

    assert_eq!(still_possible, vec!["wordb"]);
    Ok(())
}

#[test]
fn word_bank_get_word_bank_must_contain_not_here() -> Result<()> {
    let mut cursor = Cursor::new(String::from("worda\nwordb\nother\nsmore"));

    let word_bank = WordBank::from_reader(&mut cursor)?;

    let still_possible = word_bank.get_possible_words(&WordRestrictions {
        must_contain_here: vec![],
        must_contain_but_not_here: vec![LocatedLetter::new('o', 0)],
        must_not_contain: vec![],
    });

    assert_eq!(still_possible, vec!["worda", "wordb", "smore"]);
    Ok(())
}

#[test]
fn word_bank_get_word_bank_must_not_contain() -> Result<()> {
    let mut cursor = Cursor::new(String::from("worda\nwordb\nother\nsmore"));

    let word_bank = WordBank::from_reader(&mut cursor)?;

    let still_possible = word_bank.get_possible_words(&WordRestrictions {
        must_contain_here: vec![],
        must_contain_but_not_here: vec![],
        must_not_contain: vec!['w'],
    });

    assert_eq!(still_possible, vec!["other", "smore"]);
    Ok(())
}

#[test]
fn word_bank_get_word_bank_no_match() -> Result<()> {
    let mut cursor = Cursor::new(String::from("worda\nwordb\nother\nsmore"));

    let word_bank = WordBank::from_reader(&mut cursor)?;

    let still_possible: Vec<&str> = word_bank.get_possible_words(&WordRestrictions {
        must_contain_here: vec![LocatedLetter::new('o', 1)],
        must_contain_but_not_here: vec![LocatedLetter::new('b', 4)],
        must_not_contain: vec!['w'],
    });

    let empty: Vec<&str> = Vec::new();
    assert_eq!(still_possible, empty);
    Ok(())
}
