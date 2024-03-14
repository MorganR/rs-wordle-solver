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
        play_game_with_guesser(test_word, 128, RandomGuesser::new(bank.clone()))
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
        play_game_with_guesser(test_word, 128, RandomGuesser::new(bank.clone()))
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
        let guesser =
            MaxScoreGuesser::new(GuessFrom::AllUnguessedWords, bank.clone(), scorer.clone());
        play_game_with_guesser(test_word, 128, guesser)
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
        let guesser =
            MaxScoreGuesser::new(GuessFrom::AllUnguessedWords, bank.clone(), scorer.clone());
        play_game_with_guesser(test_word, 128, guesser)
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
        let guesser =
            MaxScoreGuesser::new(GuessFrom::AllUnguessedWords, bank.clone(), scorer.clone());
        play_game_with_guesser(test_word, 128, guesser)
    });

    Ok(())
}

#[bench]
fn bench_max_eliminations_improved_words_with_precompute(
    b: &mut Bencher,
) -> std::result::Result<(), Box<dyn Error>> {
    let test_words = io::BufReader::new(File::open("../data/1000-improved-words-shuffled.txt")?);
    let all_words = io::BufReader::new(File::open("../data/improved-words.txt")?);

    let bank = WordBank::from_reader(all_words)?;
    let scorer = MaxEliminationsScorer::new(bank.clone());
    let mut base_guesser = MaxScoreGuesser::new(GuessFrom::AllUnguessedWords, bank, scorer);
    base_guesser.compute_scores_if_unknown();

    let test_words: Vec<String> = test_words.lines().collect::<io::Result<Vec<String>>>()?;
    let mut test_word_iter = test_words.iter().cycle();

    b.iter(|| {
        let test_word = test_word_iter.next().unwrap();
        let guesser = base_guesser.clone();
        play_game_with_guesser(test_word, 128, guesser)
    });

    Ok(())
}

#[bench]
fn bench_max_eliminations_scorer_precompute_improved_words(
    b: &mut Bencher,
) -> std::result::Result<(), Box<dyn Error>> {
    let mut all_words = io::BufReader::new(File::open("../data/improved-words.txt")?);
    let bank = WordBank::from_reader(&mut all_words)?;
    let scorer = MaxEliminationsScorer::new(bank.clone());

    b.iter(|| {
        let mut guesser =
            MaxScoreGuesser::new(GuessFrom::AllUnguessedWords, bank.clone(), scorer.clone());
        guesser.compute_scores_if_unknown();
        guesser
    });

    Ok(())
}

macro_rules! bench_max_eliminations_with_precompute_parallelisation_limit {
    ($limit:literal, $name:ident) => {
        #[bench]
        fn $name(b: &mut Bencher) -> std::result::Result<(), Box<dyn Error>> {
            let test_words =
                io::BufReader::new(File::open("../data/1000-improved-words-shuffled.txt")?);
            let all_words = io::BufReader::new(File::open("../data/improved-words.txt")?);

            let bank = WordBank::from_reader(all_words)?;
            let scorer = MaxEliminationsScorer::new(bank.clone());
            let mut base_guesser = MaxScoreGuesser::with_parallelisation_limit(
                GuessFrom::AllUnguessedWords,
                bank,
                scorer,
                $limit,
            );
            base_guesser.compute_scores_if_unknown();

            let test_words: Vec<String> =
                test_words.lines().collect::<io::Result<Vec<String>>>()?;
            let mut test_word_iter = test_words.iter().cycle();

            b.iter(|| {
                let test_word = test_word_iter.next().unwrap();
                let guesser = base_guesser.clone();
                play_game_with_guesser(test_word, 128, guesser)
            });

            Ok(())
        }
    };
}

bench_max_eliminations_with_precompute_parallelisation_limit!(
    1,
    bench_max_eliminations_with_precompute_p0001
);
bench_max_eliminations_with_precompute_parallelisation_limit!(
    10,
    bench_max_eliminations_with_precompute_p0010
);
bench_max_eliminations_with_precompute_parallelisation_limit!(
    128,
    bench_max_eliminations_with_precompute_p0128
);
bench_max_eliminations_with_precompute_parallelisation_limit!(
    1024,
    bench_max_eliminations_with_precompute_p1024
);

#[bench]
fn bench_select_top_5_guesses_all(b: &mut Bencher) -> Result<(), WordleError> {
    let all_words = io::BufReader::new(File::open("../data/wordle-words.txt")?);

    let bank = WordBank::from_reader(all_words)?;
    let scorer = MaxEliminationsScorer::new(bank.clone());
    let mut base_guesser = MaxScoreGuesser::new(GuessFrom::AllUnguessedWords, bank, scorer);
    base_guesser.compute_scores_if_unknown();

    b.iter(|| {
        let mut guesser = base_guesser.clone();
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
    let scorer = MaxEliminationsScorer::new(bank.clone());
    let mut base_guesser = MaxScoreGuesser::new(GuessFrom::AllUnguessedWords, bank, scorer);
    base_guesser.compute_scores_if_unknown();

    let guess = "tares";

    b.iter(|| {
        let test_word = test_word_iter.next().unwrap();
        let mut guesser = base_guesser.clone();
        let result = get_result_for_guess(test_word, guess);
        guesser.update(&result.unwrap()).unwrap();
        guesser.select_top_n_guesses(5)
    });

    Ok(())
}

macro_rules! bench_select_top_n_parallelisation_limit {
    ($limit:literal, $name:ident) => {
        #[bench]
        fn $name(b: &mut Bencher) -> std::result::Result<(), Box<dyn Error>> {
            let test_words =
                io::BufReader::new(File::open("../data/1000-wordle-words-shuffled.txt")?);
            let all_words = io::BufReader::new(File::open("../data/wordle-words.txt")?);

            let test_words: Vec<String> =
                test_words.lines().collect::<io::Result<Vec<String>>>()?;
            let mut test_word_iter = test_words.iter().cycle();

            let bank = WordBank::from_reader(all_words)?;
            let scorer = MaxEliminationsScorer::new(bank.clone());
            let mut base_guesser = MaxScoreGuesser::with_parallelisation_limit(
                GuessFrom::PossibleWords,
                bank,
                scorer,
                $limit,
            );
            base_guesser.compute_scores_if_unknown();

            let guess = "tares";

            b.iter(|| {
                let test_word = test_word_iter.next().unwrap();
                let mut guesser = base_guesser.clone();
                let result = get_result_for_guess(&test_word, guess);
                guesser.update(&result.unwrap()).unwrap();
                guesser.select_top_n_guesses(5)
            });

            Ok(())
        }
    };
}

bench_select_top_n_parallelisation_limit!(1, bench_select_top_5_post_guess_possible_only_p0001);
bench_select_top_n_parallelisation_limit!(10, bench_select_top_5_post_guess_possible_only_p0010);
bench_select_top_n_parallelisation_limit!(128, bench_select_top_5_post_guess_possible_only_p0128);
bench_select_top_n_parallelisation_limit!(1024, bench_select_top_5_post_guess_possible_only_p1024);
