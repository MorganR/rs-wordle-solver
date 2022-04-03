//! Provides tools to algorithmically select the answer to a game of [Wordle](https://www.nytimes.com/games/wordle/index.html).
//!
//! ## Quick start
//!
//! ### Solving a game in one function
//!
//! ```
//! use rs_wordle_solver::*;
//!
//! // Construct a WordBank to select guesses from.
//! let bank = WordBank::from_iterator(&["abc", "bcd", "def"])?;
//! // Construct a guesser.
//! let guesser = RandomGuesser::new(&bank);
//! // Play the game.
//! let max_number_of_guesses = 3;
//! let objective = "bcd";
//! let result = play_game_with_guesser(objective, max_number_of_guesses, guesser);
//!
//! assert!(matches!(result, GameResult::Success(_guesses_made)));
//! # Ok::<(), WordleError>(())
//! ```
//!
//! ### Solving a game interactively
//!
//! ```
//! use rs_wordle_solver::*;
//!
//! // Construct a WordBank to select guesses from.
//! let bank = WordBank::from_iterator(&["abc", "bcd", "def"])?;
//! // Construct a guesser.
//! let mut guesser = RandomGuesser::new(&bank);
//!
//! // Take a guess
//! let objective = "abc";
//! let guess = guesser.select_next_guess().unwrap();
//!
//! // Get the results
//! let results = get_result_for_guess(objective, &guess)?;
//!
//! // Update the guesser
//! guesser.update(&results)?;
//!
//! // Repeat!
//! let guess = guesser.select_next_guess().unwrap();
//!
//! // ...
//!
//! # Ok::<(), WordleError>(())
//! ```
//!
//! ## Solving algorithms
//!
//! See the implementations of [`Guesser`] for more information on the available guessing
//! algorithms.
//!
//! If you want to implement your own algorithm, the easiest place to start is likely by
//! implementing the [`WordScorer`] trait, and using this with [`MaxScoreGuesser`]. There are
//! additional helpful utilities for implementing your own algorithms in the [`details`] mod.

mod data;
mod engine;
mod restrictions;
mod results;

pub use data::WordBank;
pub use data::WordCounter;
pub use engine::*;
pub use results::*;

/// Internals and other things that may be useful if you want to implement your own Wordle solving
/// algorithms.
pub mod details {
    pub use crate::data::CompressedGuessResult;
    pub use crate::data::LocatedLetter;
    pub use crate::data::WordTracker;
    pub use crate::restrictions::*;
}
