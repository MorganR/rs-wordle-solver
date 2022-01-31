use wordle_solver::*;

use std::io::Cursor;
use std::io::Result;
use std::rc::Rc;

macro_rules! assert_rc_eq {
    ($rc_vec:expr, $non_rc_vec:expr) => {
        assert_eq!(
            $rc_vec,
            $non_rc_vec
                .iter()
                .map(|thing| Rc::from(*thing))
                .collect::<Vec<Rc<_>>>()
        );
    };
}

#[test]
fn word_bank_from_reader_succeeds() -> Result<()> {
    let mut cursor = Cursor::new(String::from("worda\nwordb"));

    let word_bank = WordBank::from_reader(&mut cursor)?;

    assert_eq!(word_bank.len(), 2);
    assert_rc_eq!(word_bank.all_words(), vec!["worda", "wordb"]);
    Ok(())
}

#[test]
fn word_bank_from_vec_succeeds() {
    let words: Vec<String> = vec![String::from("worda"), String::from("wordb")];
    let word_bank = WordBank::from_vec(words);

    assert_eq!(word_bank.len(), 2);
    assert_rc_eq!(word_bank.all_words(), vec!["worda", "wordb"]);
}
