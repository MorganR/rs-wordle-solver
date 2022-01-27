use clap::Parser;
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
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    println!("File: {}", args.words_file);

    let mut words_reader = io::BufReader::new(File::open(args.words_file)?);
    let word_bank = WordBank::from_reader(&mut words_reader)?;
    println!("There are {} possible words.", word_bank.len());

    play_game(&word_bank)?;

    Ok(())
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
