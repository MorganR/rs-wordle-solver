use wordle_solver::*;

use std::io::Cursor;
use std::io::Result;

macro_rules! assert_rc_eq {
    ($rc_vec:expr, $non_rc_vec:expr, $rc_type:ty) => {
        let copy: Vec<$rc_type> = $rc_vec.iter().map(|thing| (**thing).clone()).collect();
        assert_eq!(copy, $non_rc_vec);
    };
}

#[test]
fn word_bank_from_reader_succeeds() -> Result<()> {
    let mut cursor = Cursor::new(String::from("worda\nwordb"));

    let word_bank = WordBank::from_reader(&mut cursor)?;

    assert_eq!(word_bank.len(), 2);
    assert_rc_eq!(word_bank.all_words(), vec!["worda", "wordb"], String);
    Ok(())
}

#[test]
fn word_bank_from_vec_succeeds() {
    let words: Vec<String> = vec![String::from("worda"), String::from("wordb")];
    let word_bank = WordBank::from_vec(words);

    assert_eq!(word_bank.len(), 2);
    assert_rc_eq!(word_bank.all_words(), vec!["worda", "wordb"], String);
}
