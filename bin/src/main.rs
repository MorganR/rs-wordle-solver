use clap::{Parser, Subcommand};
use rs_wordle_solver::*;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::BufRead;
use std::num::NonZeroUsize;
use std::result::Result;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

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
    LocatedLetters,
    ApproximateEliminations,
    MaxEliminations,
    MaxComboEliminations,
}

impl std::str::FromStr for GuesserImpl {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "random" => Ok(GuesserImpl::Random),
            "unique_letters" => Ok(GuesserImpl::UniqueLetterFrequency),
            "located_letters" => Ok(GuesserImpl::LocatedLetters),
            "approx_eliminations" => Ok(GuesserImpl::ApproximateEliminations),
            "max_eliminations" => Ok(GuesserImpl::MaxEliminations),
            "max_combo_eliminations" => Ok(GuesserImpl::MaxComboEliminations),
            _ => Err(String::from("Valid guesser implementations are: 'approx_eliminations', 'located_letters', 'max_eliminations', 'max_combo_eliminations', 'random', 'unique_letters', and 'unique_unguessed_letters'."))
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

impl Into<rs_wordle_solver::GuessFrom> for GuessFrom {
    fn into(self) -> rs_wordle_solver::GuessFrom {
        match self {
            Self::AllUnguessedWords => rs_wordle_solver::GuessFrom::AllUnguessedWords,
            Self::PossibleWords => rs_wordle_solver::GuessFrom::PossibleWords,
        }
    }
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Benchmark the solver against every word in the given words file.
    Benchmark {
        /// The file of words to benchmark against.
        bench_file: String,
    },
    /// Run a single game with the given word.
    Single { word: String },
    /// Run an interactive game against the solver.
    Interactive,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    let args = Args::parse();
    println!("File: {}", args.words_file);

    let mut words_reader = io::BufReader::new(File::open(args.words_file)?);
    let word_bank = WordBank::from_reader(&mut words_reader)?;
    println!("There are {} possible words.", word_bank.len());

    match args.command {
        Command::Benchmark { bench_file } => {
            run_benchmark(&word_bank, args.guesser_impl, args.guess_from, &bench_file)?
        }
        Command::Single { word } => {
            play_single_game(&word, &word_bank, args.guesser_impl, args.guess_from)?
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

fn run_benchmark(
    word_bank: &WordBank,
    guesser_impl: GuesserImpl,
    guess_from: GuessFrom,
    bench_file: &str,
) -> Result<(), WordleError> {
    let mut num_guesses_per_game: Vec<u32> = Vec::new();
    let mut second_guess_count: HashMap<Box<str>, u32> = HashMap::new();
    let mut third_guess_count: HashMap<Box<str>, u32> = HashMap::new();
    let bench_words_reader = io::BufReader::new(File::open(bench_file)?);
    let bench_words = Arc::new(
        bench_words_reader
            .lines()
            .map(|maybe_word| {
                maybe_word
                    .map_err(WordleError::from)
                    .map(|word| Arc::from(word.to_lowercase().as_str()))
            })
            .filter(|maybe_word| {
                maybe_word
                    .as_ref()
                    .map_or(true, |word: &Arc<str>| word.len() > 0)
            })
            .collect::<Result<Vec<Arc<str>>, WordleError>>()?,
    );
    let num_bench_words = bench_words.len();
    let all_words: Arc<Vec<Arc<str>>> = Arc::new(
        word_bank
            .iter()
            .map(|word| Arc::from(word.as_ref()))
            .collect(),
    );

    let num_workers = thread::available_parallelism()
        .map(NonZeroUsize::get)
        .unwrap_or(1);
    let mut worker_handles: Vec<thread::JoinHandle<Vec<GameResult>>> =
        Vec::with_capacity(num_workers);
    let words_per_worker = num_bench_words / num_workers;
    let remaining_words = num_bench_words % num_workers;
    let mut base_word_index = 0;

    for i in 0..num_workers {
        let mut next_word_index = base_word_index + words_per_worker;
        if i == 0 {
            next_word_index += remaining_words;
        }
        let bench_words_view = Arc::clone(&bench_words);
        let all_words_view = Arc::clone(&all_words);

        worker_handles.push(thread::spawn(move || {
            benchmark_words(
                &bench_words_view[base_word_index..next_word_index],
                all_words_view,
                guesser_impl,
                guess_from,
            )
        }));
        base_word_index = next_word_index;
    }

    let mut first_guess: Box<str> = Box::from("");
    let possible_word_buckets = vec![1, 2, 4, 8, 16, 32, 64, 96, 128, 256, 512, 1024, 2048];
    let mut possible_words_count_0 = vec![0; 13];
    let mut possible_words_count_1 = vec![0; 13];
    let mut possible_words_count_2 = vec![0; 13];
    let mut possible_words_count_3 = vec![0; 13];
    let mut possible_words_count_0_from_end = vec![0; 13];
    let mut possible_words_count_1_from_end = vec![0; 13];
    let mut possible_words_count_2_from_end = vec![0; 13];
    let mut possible_words_count_3_from_end = vec![0; 13];
    for handle in worker_handles {
        let results = handle.join().expect("Error on join.");
        for result in results {
            if let GameResult::Success(data) = result {
                num_guesses_per_game.push(data.turns.len() as u32);
                first_guess = data.turns[0].guess.clone();
                if data.turns.len() > 1 {
                    *second_guess_count
                        .entry(data.turns[1].guess.clone())
                        .or_default() += 1;
                    if data.turns.len() > 2 {
                        *third_guess_count
                            .entry(data.turns[2].guess.clone())
                            .or_default() += 1;
                    }
                }
                for (index, num_possible_words) in data
                    .turns
                    .iter()
                    .map(|turn| turn.num_possible_words_before_guess)
                    .enumerate()
                {
                    let bucket_index = possible_word_buckets
                        .iter()
                        .enumerate()
                        .filter(|(_, bucket_min)| **bucket_min <= num_possible_words)
                        .map(|(bucket_index, _)| bucket_index)
                        .last()
                        .unwrap();
                    match index {
                        0 => possible_words_count_0[bucket_index] += 1,
                        1 => possible_words_count_1[bucket_index] += 1,
                        2 => possible_words_count_2[bucket_index] += 1,
                        3 => possible_words_count_3[bucket_index] += 1,
                        _ => break,
                    }
                }
                for (index, num_possible_words) in data
                    .turns
                    .iter()
                    .rev()
                    .map(|turn| turn.num_possible_words_before_guess)
                    .enumerate()
                {
                    let bucket_index = possible_word_buckets
                        .iter()
                        .enumerate()
                        .filter(|(_, bucket_min)| **bucket_min <= num_possible_words)
                        .map(|(bucket_index, _)| bucket_index)
                        .last()
                        .unwrap();
                    match index {
                        0 => possible_words_count_0_from_end[bucket_index] += 1,
                        1 => possible_words_count_1_from_end[bucket_index] += 1,
                        2 => possible_words_count_2_from_end[bucket_index] += 1,
                        3 => possible_words_count_3_from_end[bucket_index] += 1,
                        _ => break,
                    }
                }
            }
        }
    }

    println!("Solved {} words. Results:", num_bench_words);

    let mut num_games_per_round: HashMap<u32, u32> = HashMap::new();
    for num_guesses in num_guesses_per_game.iter() {
        *(num_games_per_round.entry(*num_guesses).or_insert(0)) += 1;
    }

    println!("|Num guesses|Num games|");
    println!("|-----------|---------|");
    let mut num_rounds = Vec::from_iter(num_games_per_round.keys().copied());
    num_rounds.sort_unstable();
    for num_round in num_rounds.iter() {
        println!(
            "|{}|{}|",
            num_round,
            num_games_per_round.get(num_round).unwrap()
        );
    }

    println!("\nNum possible words remaining:");

    println!("\nFrom round 0:");
    println!("Round 0:");
    println!("|Min word count|Num games|");
    println!("|--------------|---------|");
    for (bucket, count) in possible_word_buckets.iter().zip(&possible_words_count_0) {
        println!("|{}|{}|", bucket, count);
    }
    println!("Round 1:");
    println!("|Min word count|Num games|");
    println!("|--------------|---------|");
    for (bucket, count) in possible_word_buckets.iter().zip(&possible_words_count_1) {
        println!("|{}|{}|", bucket, count);
    }
    println!("Round 2:");
    println!("|Min word count|Num games|");
    println!("|--------------|---------|");
    for (bucket, count) in possible_word_buckets.iter().zip(&possible_words_count_2) {
        println!("|{}|{}|", bucket, count);
    }
    println!("Round 3:");
    println!("|Min word count|Num games|");
    println!("|--------------|---------|");
    for (bucket, count) in possible_word_buckets.iter().zip(&possible_words_count_3) {
        println!("|{}|{}|", bucket, count);
    }

    println!("\nFrom the final guess:");
    println!("Final round:");
    println!("|Min word count|Num games|");
    println!("|--------------|---------|");
    for (bucket, count) in possible_word_buckets
        .iter()
        .zip(&possible_words_count_0_from_end)
    {
        println!("|{}|{}|", bucket, count);
    }
    println!("Final round - 1:");
    println!("|Min word count|Num games|");
    println!("|--------------|---------|");
    for (bucket, count) in possible_word_buckets
        .iter()
        .zip(&possible_words_count_1_from_end)
    {
        println!("|{}|{}|", bucket, count);
    }
    println!("Final round - 2:");
    println!("|Min word count|Num games|");
    println!("|--------------|---------|");
    for (bucket, count) in possible_word_buckets
        .iter()
        .zip(&possible_words_count_2_from_end)
    {
        println!("|{}|{}|", bucket, count);
    }
    println!("Final round - 3:");
    println!("|Min word count|Num games|");
    println!("|--------------|---------|");
    for (bucket, count) in possible_word_buckets
        .iter()
        .zip(&possible_words_count_3_from_end)
    {
        println!("|{}|{}|", bucket, count);
    }

    println!("First guess: {}", first_guess);
    println!("Top second guesses:");
    print_top_n(second_guess_count, 10);
    println!("Top third guesses:");
    print_top_n(third_guess_count, 10);

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

    Ok(())
}

fn benchmark_words(
    words_to_bench: &[Arc<str>],
    all_words: Arc<Vec<Arc<str>>>,
    guesser_impl: GuesserImpl,
    guess_from: GuessFrom,
) -> Vec<GameResult> {
    let word_bank: WordBank = WordBank::from_iterator(all_words.iter()).unwrap();
    let mut results: Vec<GameResult> = Vec::with_capacity(words_to_bench.len());
    let maybe_max_eliminations_scorer = match guesser_impl {
        GuesserImpl::MaxEliminations => Some(MaxEliminationsScorer::new(&word_bank).unwrap()),
        _ => None,
    };
    let maybe_max_combo_eliminations_scorer = match guesser_impl {
        GuesserImpl::MaxComboEliminations => {
            Some(MaxComboEliminationsScorer::new(&word_bank).unwrap())
        }
        _ => None,
    };
    for word in words_to_bench.iter() {
        let max_num_guesses = 128;
        let result = match guesser_impl {
            GuesserImpl::Random => {
                play_game_with_guesser(word, max_num_guesses, RandomGuesser::new(&word_bank))
            }
            GuesserImpl::UniqueLetterFrequency => play_game_with_guesser(
                word,
                max_num_guesses,
                MaxScoreGuesser::new(
                    guess_from.into(),
                    &word_bank,
                    MaxUniqueLetterFrequencyScorer::new(WordCounter::new(&word_bank)),
                ),
            ),
            GuesserImpl::LocatedLetters => play_game_with_guesser(
                word,
                max_num_guesses,
                MaxScoreGuesser::new(
                    guess_from.into(),
                    &word_bank,
                    LocatedLettersScorer::new(&word_bank, WordCounter::new(&word_bank)),
                ),
            ),
            GuesserImpl::ApproximateEliminations => play_game_with_guesser(
                word,
                max_num_guesses,
                MaxScoreGuesser::new(
                    guess_from.into(),
                    &word_bank,
                    MaxApproximateEliminationsScorer::new(WordCounter::new(&word_bank)),
                ),
            ),
            GuesserImpl::MaxEliminations => play_game_with_guesser(
                word,
                max_num_guesses,
                MaxScoreGuesser::new(
                    guess_from.into(),
                    &word_bank,
                    maybe_max_eliminations_scorer.as_ref().unwrap().clone(),
                ),
            ),
            GuesserImpl::MaxComboEliminations => play_game_with_guesser(
                word,
                max_num_guesses,
                MaxScoreGuesser::new(
                    guess_from.into(),
                    &word_bank,
                    maybe_max_combo_eliminations_scorer
                        .as_ref()
                        .unwrap()
                        .clone(),
                ),
            ),
        };
        if let GameResult::Success(data) = &result {
            println!("Solved {} in {} guesses", word, data.turns.len());
        } else {
            panic!("Failed to guess word: {}. Error: {:?}", word, result);
        }
        results.push(result);
    }
    results
}

fn print_top_n(guess_count: HashMap<Box<str>, u32>, n: usize) {
    let mut guesses = Vec::from_iter(guess_count.keys());
    guesses.sort_by(|a, b| {
        guess_count
            .get(*b)
            .unwrap()
            .cmp(guess_count.get(*a).unwrap())
    });
    for (index, guess) in guesses.iter().enumerate() {
        if index == n {
            break;
        }
        println!(
            "\t{}: {} ({})",
            index,
            guess,
            guess_count.get(*guess).unwrap()
        );
    }
}

fn play_single_game(
    word: &str,
    word_bank: &WordBank,
    guesser_impl: GuesserImpl,
    guess_from: GuessFrom,
) -> Result<(), WordleError> {
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
                word_bank,
                MaxUniqueLetterFrequencyScorer::new(WordCounter::new(word_bank)),
            ),
        ),
        GuesserImpl::LocatedLetters => play_game_with_guesser(
            word,
            max_num_guesses,
            MaxScoreGuesser::new(
                guess_from.into(),
                word_bank,
                LocatedLettersScorer::new(word_bank, WordCounter::new(word_bank)),
            ),
        ),
        GuesserImpl::ApproximateEliminations => play_game_with_guesser(
            word,
            max_num_guesses,
            MaxScoreGuesser::new(
                guess_from.into(),
                word_bank,
                MaxApproximateEliminationsScorer::new(WordCounter::new(word_bank)),
            ),
        ),
        GuesserImpl::MaxEliminations => play_game_with_guesser(
            word,
            max_num_guesses,
            MaxScoreGuesser::new(
                guess_from.into(),
                word_bank,
                MaxEliminationsScorer::new(word_bank)?,
            ),
        ),
        GuesserImpl::MaxComboEliminations => play_game_with_guesser(
            word,
            max_num_guesses,
            MaxScoreGuesser::new(
                guess_from.into(),
                word_bank,
                MaxComboEliminationsScorer::new(word_bank)?,
            ),
        ),
    };
    match result {
        GameResult::Success(data) => {
            println!("Solved it! It took me {} guesses.", data.turns.len());
            for guess in data.turns.iter().map(|turn| &turn.guess) {
                println!("\t{}", guess);
            }
        }
        GameResult::Failure(data) => {
            println!(
                "I still couldn't solve it after {} guesses :(",
                data.turns.len()
            );
            for guess in data.turns.iter().map(|turn| &turn.guess) {
                println!("\t{}", guess);
            }
        }
        GameResult::UnknownWord => {
            eprintln!("Error: given word not in the word list.");
            std::process::exit(1);
        }
    }
    Ok(())
}

fn play_interactive_game(
    word_bank: &WordBank,
    guesser_impl: GuesserImpl,
    guess_from: GuessFrom,
) -> Result<(), Box<dyn std::error::Error>> {
    match guesser_impl {
        GuesserImpl::Random => play_interactive_game_with_guesser(RandomGuesser::new(word_bank)),
        GuesserImpl::UniqueLetterFrequency => {
            play_interactive_game_with_guesser(MaxScoreGuesser::new(
                guess_from.into(),
                word_bank,
                MaxUniqueLetterFrequencyScorer::new(WordCounter::new(word_bank)),
            ))
        }
        GuesserImpl::LocatedLetters => play_interactive_game_with_guesser(MaxScoreGuesser::new(
            guess_from.into(),
            word_bank,
            LocatedLettersScorer::new(word_bank, WordCounter::new(word_bank)),
        )),
        GuesserImpl::ApproximateEliminations => {
            play_interactive_game_with_guesser(MaxScoreGuesser::new(
                guess_from.into(),
                word_bank,
                MaxApproximateEliminationsScorer::new(WordCounter::new(word_bank)),
            ))
        }
        GuesserImpl::MaxEliminations => play_interactive_game_with_guesser(MaxScoreGuesser::new(
            guess_from.into(),
            word_bank,
            MaxEliminationsScorer::new(word_bank)?,
        )),
        GuesserImpl::MaxComboEliminations => {
            play_interactive_game_with_guesser(MaxScoreGuesser::new(
                guess_from.into(),
                word_bank,
                MaxComboEliminationsScorer::new(word_bank)?,
            ))
        }
    }
    .map_err(Box::from)
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

        guesser.update(&result).unwrap();
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
        guess,
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
