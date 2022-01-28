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
fn play_game_with_unknown_word() {
    let bank = create_word_bank(vec!["abcz", "weyz", "defy", "ghix"]);

    assert_eq!(play_game("nope", 10, &bank), GameResult::UnknownWord());
}

#[test]
fn play_game_with_known_word() {
    let bank = create_word_bank(vec!["abcz", "weyz", "defy", "ghix"]);

    if let GameResult::Success(guesses) = play_game("abcz", 10, &bank) {
        assert!(guesses.len() < 10);
        assert_eq!(guesses.iter().last(), Some(&String::from("abcz")));
    } else {
        assert!(false);
    }
}

#[test]
fn play_game_takes_too_many_guesses() {
    let bank = create_word_bank(vec!["abcz", "weyz", "defy", "ghix"]);

    if let GameResult::Failure(guesses) = play_game("abcz", 1, &bank) {
        assert_eq!(guesses.len(), 1);
        assert!(!guesses.contains(&String::from("abcz")));
    } else {
        assert!(false);
    }
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
