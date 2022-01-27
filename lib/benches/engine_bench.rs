#![feature(test)]

extern crate test;

use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Result;
use test::Bencher;
use wordle_solver::*;

#[bench]
fn bench_guess_random_wordle_words(b: &mut Bencher) -> Result<()> {
    let test_words = BufReader::new(File::open("../data/1000-wordle-words-shuffled.txt")?);
    let mut all_words = BufReader::new(File::open("../data/wordle-words.txt")?);

    let bank = WordBank::from_reader(&mut all_words)?;

    let test_words: Vec<String> = test_words.lines().collect::<Result<Vec<String>>>()?;
    let mut test_word_iter = test_words.iter();

    b.iter(|| {
        let test_word = test_word_iter.next().unwrap_or_else(|| {
            // Reset the iterator.
            test_word_iter = test_words.iter();
            test_word_iter.next().unwrap()
        });
        let game = Game::new(&bank);
        return game.play_game(test_word, u32::MAX);
    });

    Ok(())
}

#[bench]
fn bench_guess_random_improved_words(b: &mut Bencher) -> Result<()> {
    let test_words = BufReader::new(File::open("../data/1000-improved-words-shuffled.txt")?);
    let mut all_words = BufReader::new(File::open("../data/improved-words.txt")?);

    let bank = WordBank::from_reader(&mut all_words)?;

    let test_words: Vec<String> = test_words.lines().collect::<Result<Vec<String>>>()?;
    let mut test_word_iter = test_words.iter();

    b.iter(|| {
        let test_word = test_word_iter.next().unwrap_or_else(|| {
            // Reset the iterator.
            test_word_iter = test_words.iter();
            test_word_iter.next().unwrap()
        });
        let game = Game::new(&bank);
        return game.play_game(test_word, u32::MAX);
    });

    Ok(())
}