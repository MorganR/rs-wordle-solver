use clap::Parser;
use std::fs::File;
use std::io::BufReader;
use std::io::Result;
use wordle_solver::*;

/// Simple program to run a Wordle game in reverse, where the computer guesses the word.
#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// Path to a file that contains a list of possible words, with one word on each line.
    #[clap(short = 'f', long)]
    words_file: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    println!("File: {}", args.words_file);

    let mut words_reader = BufReader::new(File::open(args.words_file)?);
    let word_bank = WordBank::from_reader(&mut words_reader)?;

    println!("Read {} words.", word_bank.len());
    Ok(())
}
