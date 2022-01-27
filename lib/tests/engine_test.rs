use wordle_solver::*;

#[test]
fn calculate_best_guess_no_words() {
    let bank = create_word_bank(vec![]);
    let game = Game::new(&bank);

    assert_eq!(game.calculate_best_guess(), None);
}

#[test]
fn calculate_best_guess_chooses_best_word() {
    let bank = create_word_bank(vec!["abcz", "wxyz", "defy", "ghix"]);
    let game = Game::new(&bank);

    assert_eq!(game.calculate_best_guess(), Some("wxyz"));
}

#[test]
fn update_guess_result_modifies_next_guess() {
    let bank = create_word_bank(vec!["abcz", "weyz", "defy", "ghix"]);
    let mut game = Game::new(&bank);

    game.update_guess_result(&GuessResult {
        letters: vec![
            LetterResult::NotPresent('w'),
            LetterResult::Correct('e'),
            LetterResult::PresentNotHere('y'),
            LetterResult::NotPresent('z'),
        ],
    });

    assert_eq!(game.calculate_best_guess(), Some("defy"));
}

#[test]
fn get_result_for_guess_success() {
    let result = get_result_for_guess("piano", "amino");

    assert_eq!(
        result.letters,
        vec![
            LetterResult::PresentNotHere('a'),
            LetterResult::NotPresent('m'),
            LetterResult::PresentNotHere('i'),
            LetterResult::Correct('n'),
            LetterResult::Correct('o'),
        ]
    )
}

fn create_word_bank(words: Vec<&str>) -> WordBank {
    WordBank::from_vec(words.iter().map(|word| word.to_string()).collect())
}
