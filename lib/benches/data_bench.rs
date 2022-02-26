#![feature(test)]

extern crate test;

use rs_wordle_solver::details::*;
use rs_wordle_solver::*;

use std::fs::File;
use std::io;
use std::result::Result;
use test::Bencher;

#[bench]
fn bench_word_counter_new(b: &mut Bencher) -> Result<(), WordleError> {
    let words_reader = io::BufReader::new(File::open("../data/1000-improved-words-shuffled.txt")?);
    let bank = WordBank::from_reader(words_reader)?;

    b.iter(|| WordCounter::new(&bank));

    Ok(())
}

#[bench]
fn bench_word_counter_clone(b: &mut Bencher) -> Result<(), WordleError> {
    let words_reader = io::BufReader::new(File::open("../data/1000-improved-words-shuffled.txt")?);
    let bank = WordBank::from_reader(words_reader)?;
    let counter = WordCounter::new(&bank);

    b.iter(|| counter.clone());

    Ok(())
}

#[bench]
fn bench_word_tracker_from_slice(b: &mut Bencher) -> Result<(), WordleError> {
    let words_reader = io::BufReader::new(File::open("../data/1000-improved-words-shuffled.txt")?);
    let bank = WordBank::from_reader(words_reader)?;

    b.iter(|| WordTracker::from_slice(&bank));

    Ok(())
}

#[bench]
fn bench_word_tracker_clone(b: &mut Bencher) -> Result<(), WordleError> {
    let words_reader = io::BufReader::new(File::open("../data/1000-improved-words-shuffled.txt")?);
    let bank = WordBank::from_reader(words_reader)?;
    let tracker = WordTracker::from_slice(&bank);

    b.iter(|| tracker.clone());

    Ok(())
}
