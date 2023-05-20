#![feature(test)]

extern crate test;

use rs_wordle_solver::scorers::*;
use rs_wordle_solver::*;

use std::error::Error;
use std::fs::File;
use std::io;
use std::io::BufRead;
use std::result::Result;
use test::Bencher;

#[bench]
fn bench_guess_random_wordle_words(b: &mut Bencher) -> Result<(), WordleError> {
    let test_words = io::BufReader::new(File::open("../data/1000-wordle-words-shuffled.txt")?);
    let all_words = io::BufReader::new(File::open("../data/wordle-words.txt")?);

    let bank = WordBank::from_reader(all_words)?;

    let test_words: Vec<String> = test_words.lines().collect::<io::Result<Vec<String>>>()?;
    let mut test_word_iter = test_words.iter().cycle();

    b.iter(|| {
        let test_word = test_word_iter.next().unwrap();
        return play_game_with_guesser(test_word, 128, RandomGuesser::new(&bank));
    });

    Ok(())
}

#[bench]
fn bench_guess_random_improved_words(b: &mut Bencher) -> Result<(), WordleError> {
    let test_words = io::BufReader::new(File::open("../data/1000-improved-words-shuffled.txt")?);
    let all_words = io::BufReader::new(File::open("../data/improved-words.txt")?);

    let bank = WordBank::from_reader(all_words)?;

    let test_words: Vec<String> = test_words.lines().collect::<io::Result<Vec<String>>>()?;
    let mut test_word_iter = test_words.iter().cycle();

    b.iter(|| {
        let test_word = test_word_iter.next().unwrap();
        return play_game_with_guesser(test_word, 128, RandomGuesser::new(&bank));
    });

    Ok(())
}

#[bench]
fn bench_unique_letters_improved_words(b: &mut Bencher) -> Result<(), WordleError> {
    let test_words = io::BufReader::new(File::open("../data/1000-improved-words-shuffled.txt")?);
    let all_words = io::BufReader::new(File::open("../data/improved-words.txt")?);

    let bank = WordBank::from_reader(all_words)?;
    let scorer = MaxUniqueLetterFrequencyScorer::new(&bank);

    let test_words: Vec<String> = test_words.lines().collect::<io::Result<Vec<String>>>()?;
    let mut test_word_iter = test_words.iter().cycle();

    b.iter(|| {
        let test_word = test_word_iter.next().unwrap();
        let guesser = MaxScoreGuesser::new(GuessFrom::AllUnguessedWords, &bank, scorer.clone());
        return play_game_with_guesser(test_word, 128, guesser);
    });

    Ok(())
}

#[bench]
fn bench_located_letters_improved_words(b: &mut Bencher) -> Result<(), WordleError> {
    let test_words = io::BufReader::new(File::open("../data/1000-improved-words-shuffled.txt")?);
    let all_words = io::BufReader::new(File::open("../data/improved-words.txt")?);

    let bank = WordBank::from_reader(all_words)?;
    let scorer = LocatedLettersScorer::new(&bank);

    let test_words: Vec<String> = test_words.lines().collect::<io::Result<Vec<String>>>()?;
    let mut test_word_iter = test_words.iter().cycle();

    b.iter(|| {
        let test_word = test_word_iter.next().unwrap();
        let guesser = MaxScoreGuesser::new(GuessFrom::AllUnguessedWords, &bank, scorer.clone());
        return play_game_with_guesser(test_word, 128, guesser);
    });

    Ok(())
}

#[bench]
fn bench_max_approximate_eliminations_improved_words(b: &mut Bencher) -> Result<(), WordleError> {
    let test_words = io::BufReader::new(File::open("../data/1000-improved-words-shuffled.txt")?);
    let all_words = io::BufReader::new(File::open("../data/improved-words.txt")?);

    let bank = WordBank::from_reader(all_words)?;
    let scorer = MaxApproximateEliminationsScorer::new(&bank);

    let test_words: Vec<String> = test_words.lines().collect::<io::Result<Vec<String>>>()?;
    let mut test_word_iter = test_words.iter().cycle();

    b.iter(|| {
        let test_word = test_word_iter.next().unwrap();
        let guesser = MaxScoreGuesser::new(GuessFrom::AllUnguessedWords, &bank, scorer.clone());
        return play_game_with_guesser(test_word, 128, guesser);
    });

    Ok(())
}

#[bench]
fn bench_max_eliminations_scorer_with_precomputed_improved_words(
    b: &mut Bencher,
) -> std::result::Result<(), Box<dyn Error>> {
    let test_words = io::BufReader::new(File::open("../data/1000-improved-words-shuffled.txt")?);
    let all_words = io::BufReader::new(File::open("../data/improved-words.txt")?);

    let bank = WordBank::from_reader(all_words)?;
    let scorer = MaxEliminationsScorer::new(&bank)?;

    let test_words: Vec<String> = test_words.lines().collect::<io::Result<Vec<String>>>()?;
    let mut test_word_iter = test_words.iter().cycle();

    b.iter(|| {
        let test_word = test_word_iter.next().unwrap();
        let guesser = MaxScoreGuesser::new(GuessFrom::AllUnguessedWords, &bank, scorer.clone());
        return play_game_with_guesser(test_word, 128, guesser);
    });

    Ok(())
}

#[bench]
fn bench_max_eliminations_scorer_precompute_improved_words(
    b: &mut Bencher,
) -> std::result::Result<(), Box<dyn Error>> {
    let mut all_words = io::BufReader::new(File::open("../data/improved-words.txt")?);
    let bank = WordBank::from_reader(&mut all_words)?;

    b.iter(|| {
        return MaxEliminationsScorer::new(&bank);
    });

    Ok(())
}

#[bench]
fn bench_select_top_5_guesses_all(b: &mut Bencher) -> Result<(), WordleError> {
    let all_words = io::BufReader::new(File::open("../data/wordle-words.txt")?);

    let bank = WordBank::from_reader(all_words)?;
    let scorer = MaxEliminationsScorer::new(&bank)?;

    b.iter(|| {
        let guesser = MaxScoreGuesser::new(GuessFrom::AllUnguessedWords, &bank, scorer.clone());
        guesser.select_top_n_guesses(5)
    });

    Ok(())
}

#[bench]
fn bench_select_top_5_guesses_post_guess_all(b: &mut Bencher) -> Result<(), WordleError> {
    let test_words = io::BufReader::new(File::open("../data/1000-wordle-words-shuffled.txt")?);
    let all_words = io::BufReader::new(File::open("../data/wordle-words.txt")?);

    let test_words: Vec<String> = test_words.lines().collect::<io::Result<Vec<String>>>()?;
    let mut test_word_iter = test_words.iter().cycle();

    let bank = WordBank::from_reader(all_words)?;
    let scorer = MaxEliminationsScorer::new(&bank)?;

    let guess = "tares";

    b.iter(|| {
        let test_word = test_word_iter.next().unwrap();
        let mut guesser = MaxScoreGuesser::new(GuessFrom::AllUnguessedWords, &bank, scorer.clone());
        let result = get_result_for_guess(&test_word, guess);
        guesser.update(&result.unwrap()).unwrap();
        guesser.select_top_n_guesses(5)
    });

    Ok(())
}

#[bench]
fn bench_select_top_5_guesses_post_guess_possible_only(b: &mut Bencher) -> Result<(), WordleError> {
    let test_words = io::BufReader::new(File::open("../data/1000-wordle-words-shuffled.txt")?);
    let all_words = io::BufReader::new(File::open("../data/wordle-words.txt")?);

    let test_words: Vec<String> = test_words.lines().collect::<io::Result<Vec<String>>>()?;
    let mut test_word_iter = test_words.iter().cycle();

    let bank = WordBank::from_reader(all_words)?;
    let scorer = MaxEliminationsScorer::new(&bank)?;

    let guess = "tares";

    b.iter(|| {
        let test_word = test_word_iter.next().unwrap();
        let mut guesser = MaxScoreGuesser::new(GuessFrom::PossibleWords, &bank, scorer.clone());
        let result = get_result_for_guess(&test_word, guess);
        guesser.update(&result.unwrap()).unwrap();
        guesser.select_top_n_guesses(5)
    });

    Ok(())
}
