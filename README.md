# wordle-solver

An automated solver for the popular game: [wordle](https://www.powerlanguage.co.uk/wordle/)

## Available words

This library has several word lists:

*  `data/wordle-words.txt`: this list is the combination of all possible guesses and all answer words from [Wordle](https://www.powerlanguage.co.uk/wordle/). Many of the allowed Wordle guesses are pretty much nonsense, so there is also:

*  `data/improved-words.txt`: this list combines all 5-letter words from the [Corncob list of more than 58,000 English words](http://www.mieliestronk.com/wordlist.html), the [MIT 10000 English words list](https://www.mit.edu/~ecprice/wordlist.10000), and any remaining Wordle answer words.

*  A random set of 1000 words taken from each list.

## Benchmark

Engine benchmark result:

```
running 2 tests
test bench_guess_random_improved_words ... bench:   1,785,671 ns/iter (+/- 85,368)
test bench_guess_random_wordle_words   ... bench:   4,963,037 ns/iter (+/- 306,767)

test result: ok. 0 passed; 0 failed; 0 ignored; 2 measured; 0 filtered out; finished in 8.24s
```
