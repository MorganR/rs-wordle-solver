#![feature(test)]

extern crate test;

use std::collections::HashMap;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Result;
use test::Bencher;
use wordle_solver::*;

#[bench]
fn bench_guess_1000_words(b: &mut Bencher) -> Result<()> {
    let test_words = BufReader::new(File::open("../data/1000-words-shuffled.txt")?);
    let mut all_words = BufReader::new(File::open("../data/all-words-sorted.txt")?);

    let bank = WordBank::from_reader(&mut all_words)?;

    let mut num_games_per_round: HashMap<u8, u32> = HashMap::new();
    let test_words: Vec<String> = test_words.lines().collect::<Result<Vec<String>>>()?;

    let mut test_word_iter = test_words.iter();

    b.iter(|| {
        let test_word = test_word_iter.next().unwrap_or_else(|| {
            // Reset the iterator.
            test_word_iter = test_words.iter();
            test_word_iter.next().unwrap()
        });
        let num_games = num_games_per_round
            .entry(play_game(test_word, &bank))
            .or_insert(0);
        *num_games += 1;
    });

    Ok(())
}

fn play_game(word: &str, bank: &WordBank) -> u8 {
    let mut game = Game::new(bank);

    for round in 1..=255 {
        let guess = game.calculate_best_guess().unwrap();
        let result = get_result_for_guess(word, guess);

        if result.letters.iter().all(|lr| match lr {
            LetterResult::Correct(_) => true,
            _ => false,
        }) {
            return round;
        }
        game.update_guess_result(&result);
    }
    return 255;
}
