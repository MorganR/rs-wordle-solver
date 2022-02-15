use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::time::Instant;
use wordle_solver::*;

/// Simple program to run a Wordle game in reverse, where the computer guesses the word.
#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// Path to a file that contains a list of possible words, with one word on each line.
    #[clap(short = 'f', long)]
    words_file: String,

    /// Which guesser implementation to use.
    #[clap(short = 'g', long)]
    guesser_impl: GuesserImpl,

    /// Which list of words to guess from, "all", or "possible". Defaults to all.
    #[clap(long)]
    guess_from: GuessFrom,

    /// If true, runs a benchmark to determine how many rounds are needed to guess every word in
    /// the words file. The benchmark is run instead of playing an interactive game.
    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Clone, Copy)]
enum GuesserImpl {
    Random,
    UniqueLetterFrequency,
    UniqueUnguessedLetterFrequency,
    LocatedLetters,
    ApproximateEliminations,
}

impl std::str::FromStr for GuesserImpl {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "random" => Ok(GuesserImpl::Random),
            "unique_letter" => Ok(GuesserImpl::UniqueLetterFrequency),
            "unique_unguessed_letter" => Ok(GuesserImpl::UniqueUnguessedLetterFrequency),
            "located_letters" => Ok(GuesserImpl::LocatedLetters),
            "approx_eliminations" => Ok(GuesserImpl::ApproximateEliminations),
            _ => Err(String::from("Valid guesser implementations are: 'approx_eliminations', 'located_letters', 'random', 'unique_letter', and 'unique_unguessed_letter'."))
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum GuessFrom {
    AllUnguessedWords,
    PossibleWords,
}

impl std::str::FromStr for GuessFrom {
    type Err = io::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "all" | "" => Ok(GuessFrom::AllUnguessedWords),
            "possible" => Ok(GuessFrom::PossibleWords),
            _ => Err(io::Error::from(io::ErrorKind::InvalidInput)),
        }
    }
}

impl Into<wordle_solver::GuessFrom> for GuessFrom {
    fn into(self) -> wordle_solver::GuessFrom {
        match self {
            Self::AllUnguessedWords => wordle_solver::GuessFrom::AllUnguessedWords,
            Self::PossibleWords => wordle_solver::GuessFrom::PossibleWords,
        }
    }
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Benchmark the solver against every word in the given words file.
    Benchmark,
    /// Run a single game with the given word.
    Single { word: String },
    /// Run an interactive game against the solver.
    Interactive,
}

fn main() -> io::Result<()> {
    let start_time = Instant::now();
    let args = Args::parse();
    println!("File: {}", args.words_file);

    let mut words_reader = io::BufReader::new(File::open(args.words_file)?);
    let word_bank = WordBank::from_reader(&mut words_reader)?;
    println!("There are {} possible words.", word_bank.len());

    match args.command {
        Command::Benchmark => run_benchmark(&word_bank, args.guesser_impl, args.guess_from),
        Command::Single { word } => {
            play_single_game(&word, &word_bank, args.guesser_impl, args.guess_from)
        }
        Command::Interactive => {
            play_interactive_game(&word_bank, args.guesser_impl, args.guess_from)?
        }
    }

    println!(
        "Command executed in {:.3}s.",
        start_time.elapsed().as_secs_f64()
    );

    Ok(())
}

fn run_benchmark(word_bank: &WordBank, guesser_impl: GuesserImpl, guess_from: GuessFrom) {
    let mut num_guesses_per_game: Vec<u32> = Vec::new();
    let word_counter = WordCounter::new(&word_bank.all_words());
    for word in word_bank.all_words().iter() {
        let max_num_guesses = 128;
        let result = match guesser_impl {
            GuesserImpl::Random => {
                play_game_with_guesser(word, max_num_guesses, RandomGuesser::new(word_bank))
            }
            GuesserImpl::UniqueLetterFrequency => play_game_with_guesser(
                word,
                max_num_guesses,
                MaxScoreGuesser::new(
                    guess_from.into(),
                    &word_bank,
                    MaxUniqueLetterFrequencyScorer::new(&word_bank.all_words()),
                ),
            ),
            GuesserImpl::UniqueUnguessedLetterFrequency => play_game_with_guesser(
                word,
                max_num_guesses,
                MaxScoreGuesser::new(
                    guess_from.into(),
                    &word_bank,
                    MaxUniqueUnguessedLetterFrequencyScorer::new(&word_bank.all_words()),
                ),
            ),
            GuesserImpl::LocatedLetters => play_game_with_guesser(
                word,
                max_num_guesses,
                MaxScoreGuesser::new(
                    guess_from.into(),
                    &word_bank,
                    LocatedLettersScorer::new(word_bank, word_counter.clone()),
                ),
            ),
            GuesserImpl::ApproximateEliminations => play_game_with_guesser(
                word,
                max_num_guesses,
                MaxScoreGuesser::new(
                    guess_from.into(),
                    &word_bank,
                    MaxApproximateEliminationsScorer::new(word_counter.clone()),
                ),
            ),
        };
        if let GameResult::Success(guesses) = result {
            num_guesses_per_game.push(guesses.len() as u32);
        } else {
            println!("Failed to guess word: {}. Error: {:?}", word, result);
            assert!(false);
        }
    }
    println!("Solved {} words. Results:", word_bank.len());

    let mut num_games_per_round: HashMap<u32, u32> = HashMap::new();
    for num_guesses in num_guesses_per_game.iter() {
        *(num_games_per_round.entry(*num_guesses).or_insert(0)) += 1;
    }

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

    let average: f64 = num_games_per_round
        .iter()
        .fold(0, |acc, (num_guesses, num_games)| {
            acc + (num_guesses * num_games)
        }) as f64
        / num_guesses_per_game.len() as f64;
    let std_dev: f64 = (num_guesses_per_game
        .iter()
        .map(|num_guesses| (*num_guesses as f64 - average).powi(2))
        .sum::<f64>()
        / num_guesses_per_game.len() as f64)
        .sqrt();

    println!(
        "\n**Average number of guesses:** {:.2} +/- {:.2}",
        average, std_dev
    );
}

fn play_single_game(
    word: &str,
    word_bank: &WordBank,
    guesser_impl: GuesserImpl,
    guess_from: GuessFrom,
) {
    let max_num_guesses = 128;
    let result = match guesser_impl {
        GuesserImpl::Random => {
            play_game_with_guesser(word, max_num_guesses, RandomGuesser::new(word_bank))
        }
        GuesserImpl::UniqueLetterFrequency => play_game_with_guesser(
            word,
            max_num_guesses,
            MaxScoreGuesser::new(
                guess_from.into(),
                &word_bank,
                MaxUniqueLetterFrequencyScorer::new(&word_bank.all_words()),
            ),
        ),
        GuesserImpl::UniqueUnguessedLetterFrequency => play_game_with_guesser(
            word,
            max_num_guesses,
            MaxScoreGuesser::new(
                guess_from.into(),
                &word_bank,
                MaxUniqueUnguessedLetterFrequencyScorer::new(&word_bank.all_words()),
            ),
        ),
        GuesserImpl::LocatedLetters => play_game_with_guesser(
            word,
            max_num_guesses,
            MaxScoreGuesser::new(
                guess_from.into(),
                &word_bank,
                LocatedLettersScorer::new(word_bank, WordCounter::new(&word_bank.all_words())),
            ),
        ),
        GuesserImpl::ApproximateEliminations => play_game_with_guesser(
            word,
            max_num_guesses,
            MaxScoreGuesser::new(
                guess_from.into(),
                &word_bank,
                MaxApproximateEliminationsScorer::new(WordCounter::new(&word_bank.all_words())),
            ),
        ),
    };
    match result {
        GameResult::Success(guesses) => {
            println!("Solved it! It took me {} guesses.", guesses.len());
            for guess in guesses.iter() {
                println!("\t{}", guess);
            }
        }
        GameResult::Failure(guesses) => {
            println!(
                "I still couldn't solve it after {} guesses :(",
                guesses.len()
            );
            for guess in guesses.iter() {
                println!("\t{}", guess);
            }
        }
        GameResult::UnknownWord => {
            eprintln!("Error: given word not in the word list.");
            std::process::exit(1);
        }
    }
}

fn play_interactive_game(
    word_bank: &WordBank,
    guesser_impl: GuesserImpl,
    guess_from: GuessFrom,
) -> io::Result<()> {
    match guesser_impl {
        GuesserImpl::Random => play_interactive_game_with_guesser(RandomGuesser::new(word_bank)),
        GuesserImpl::UniqueLetterFrequency => {
            play_interactive_game_with_guesser(MaxScoreGuesser::new(
                guess_from.into(),
                &word_bank,
                MaxUniqueLetterFrequencyScorer::new(&word_bank.all_words()),
            ))
        }
        GuesserImpl::UniqueUnguessedLetterFrequency => {
            play_interactive_game_with_guesser(MaxScoreGuesser::new(
                guess_from.into(),
                &word_bank,
                MaxUniqueUnguessedLetterFrequencyScorer::new(&word_bank.all_words()),
            ))
        }
        GuesserImpl::LocatedLetters => play_interactive_game_with_guesser(MaxScoreGuesser::new(
            guess_from.into(),
            &word_bank,
            LocatedLettersScorer::new(word_bank, WordCounter::new(&word_bank.all_words())),
        )),
        GuesserImpl::ApproximateEliminations => {
            play_interactive_game_with_guesser(MaxScoreGuesser::new(
                guess_from.into(),
                &word_bank,
                MaxApproximateEliminationsScorer::new(WordCounter::new(&word_bank.all_words())),
            ))
        }
    }
}

fn play_interactive_game_with_guesser(mut guesser: impl Guesser) -> io::Result<()> {
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
        let guess = guesser.select_next_guess().unwrap();
        println!("I'm guessing: {}. How did I do?", guess);

        let mut result = get_result_for_guess(guess.as_ref());
        while result.is_err() {
            println!("{}", result.unwrap_err());
            result = get_result_for_guess(guess.as_ref());
        }

        let result = result.unwrap();

        if result
            .results
            .iter()
            .all(|letter_result| *letter_result == LetterResult::Correct)
        {
            println!("I did it! It took me {} guesses.", round);
            return Ok(());
        }

        guesser.update(&result);
    }

    println!("I couldn't guess it :(");

    Ok(())
}

fn get_result_for_guess<'a>(guess: &'a str) -> io::Result<GuessResult<'a>> {
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
        guess: guess,
        results: input
            .char_indices()
            .map(|(_, letter)| match letter {
                '.' => Ok(LetterResult::NotPresent),
                'y' => Ok(LetterResult::PresentNotHere),
                'g' => Ok(LetterResult::Correct),
                _ => Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Must enter only the letters '.', 'y', or 'g'. Try again.",
                )),
            })
            .collect::<io::Result<Vec<LetterResult>>>()?,
    })
}
