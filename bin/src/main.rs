use clap::Parser;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use wordle_solver::*;

/// Simple program to run a Wordle game in reverse, where the computer guesses the word.
#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// Path to a file that contains a list of possible words, with one word on each line.
    #[clap(short = 'f', long)]
    words_file: String,

    /// If true, runs a benchmark to determine how many rounds are needed to guess every word in
    /// the words file. Otherwise,  
    #[clap(short = 'b', long)]
    benchmark: bool,
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    println!("File: {}", args.words_file);

    let mut words_reader = io::BufReader::new(File::open(args.words_file)?);
    let word_bank = WordBank::from_reader(&mut words_reader)?;
    println!("There are {} possible words.", word_bank.len());

    if args.benchmark {
        run_benchmark(&word_bank);
    } else {
        play_game(&word_bank)?;
    }

    Ok(())
}

fn run_benchmark(word_bank: &WordBank) {
    let mut num_games_per_round: HashMap<u32, u32> = HashMap::new();

    for word in word_bank.all_words().iter() {
        let game = Game::new(&word_bank);

        if let GameResult::Success(rounds) = game.play_game(word, 128) {
            *(num_games_per_round.entry(rounds).or_insert(0)) += 1;
        } else {
            assert!(false);
        }
    }

    println!("Solved {} words. Results:", word_bank.len());
    println!("|Num guesses|Num games|");
    println!("|-----------|---------|");
    let mut num_rounds = num_games_per_round
        .keys()
        .map(|key| *key)
        .collect::<Vec<u32>>();
    num_rounds.sort_unstable();
    for num_round in num_rounds.iter() {
        println!(
            "|{}|{}|",
            num_round,
            num_games_per_round.get(num_round).unwrap()
        );
    }
    println!(
        "\n**Average number of guesses:** {:.2}",
        num_games_per_round
            .iter()
            .fold(0, |acc, (key, value)| acc + (key * value)) as f64
            / word_bank.len() as f64
    );
}

fn play_game(word_bank: &WordBank) -> io::Result<()> {
    let mut game = Game::new(&word_bank);
    println!("Choose a word from the word-list. Press enter once you've chosen.");

    {
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer)?;
    }

    println!(
        "I will now try to guess your word.\n\n\
         For each guess, enter the correctness of each letter as:\n\n\
           * '.' = this letter is not in the word\n\
           * 'y' = this letter is in the word, but not in this location\n\
           * 'g' = this letter is in the word and in the right location.\n\n\
         For example, if your word was \"spade\" and the guess was \"soapy\", you would enter \"g.gy.\"");

    for round in 1..7 {
        let guess = game.calculate_best_guess().unwrap();
        println!("I'm guessing: {}. How did I do?", guess);

        let mut result = get_result_for_guess(guess);
        while result.is_err() {
            println!("{}", result.unwrap_err());
            result = get_result_for_guess(guess);
        }

        let result = result.unwrap();

        if result
            .letters
            .iter()
            .all(|letter_result| match letter_result {
                LetterResult::Correct(_) => true,
                _ => false,
            })
        {
            println!("I did it! It took me {} guesses.", round);
            return Ok(());
        }

        game.update_guess_result(&result);
    }

    println!("I couldn't guess it :(");

    Ok(())
}

fn get_result_for_guess(guess: &str) -> io::Result<GuessResult> {
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer)?;
    let input = buffer.trim();

    if guess.len() != input.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Input {} didn't match the length of my guess. Try again.",
                input
            ),
        ));
    }

    Ok(GuessResult {
        letters: input
            .char_indices()
            .map(|(index, letter)| match letter {
                '.' => Ok(LetterResult::NotPresent(guess.chars().nth(index).unwrap())),
                'y' => Ok(LetterResult::PresentNotHere(
                    guess.chars().nth(index).unwrap(),
                )),
                'g' => Ok(LetterResult::Correct(guess.chars().nth(index).unwrap())),
                _ => Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Must enter only the letters '.', 'y', or 'g'. Try again.",
                )),
            })
            .collect::<io::Result<Vec<LetterResult>>>()?,
    })
}
