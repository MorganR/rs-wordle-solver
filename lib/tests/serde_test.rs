#[macro_use]
extern crate assert_matches;

use std::{collections::HashMap, sync::Arc};

use ron;
use rs_wordle_solver::{scorers::MaxEliminationsScorer, *};

#[test]
fn max_eliminations_scorer_serde() {
    let word_bank = WordBank::from_iterator(vec!["worda", "wordb"]).unwrap();
    let scorer = MaxEliminationsScorer::new(&word_bank).unwrap();
    let expected_eliminations = scorer.first_guess_eliminations();

    let ser = ron::to_string(&expected_eliminations);
    assert_matches!(ser, Ok(_));

    let deser = ron::from_str::<HashMap<Arc<str>, f64>>(&ser.unwrap());
    assert_matches!(deser, Ok(_));
    let deser_scorer = MaxEliminationsScorer::from_first_guess_eliminations(deser.unwrap());
    assert!(deser_scorer.is_ok());
}
