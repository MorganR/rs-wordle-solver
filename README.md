# wordle-solver

An automated solver for the popular game: [wordle](https://www.powerlanguage.co.uk/wordle/)

## Available words

This library has several word lists:

*  `data/wordle-words.txt`: this list is the combination of all possible guesses and all answer
   words from [Wordle](https://www.powerlanguage.co.uk/wordle/). Many of the allowed Wordle guesses
   are pretty much nonsense, so there is also:

*  `data/improved-words.txt`: this list combines all 5-letter words from the [Corncob list of more
   than 58,000 English words](http://www.mieliestronk.com/wordlist.html), the
   [MIT 10000 English words list](https://www.mit.edu/~ecprice/wordlist.10000), and any remaining
   Wordle answer words.

*  A random set of 1000 words taken from each list.

## Guesses Benchmark

When benchmarked against the whole *improved* words list:

### RandomGuesser

This selects randomly from the words that are still possible. It's a baseline worst-case selection.
Any other algorithm should be better than this.

One sample benchmark:

|Num guesses|Num games|
|-----------|---------|
|1|1|
|2|106|
|3|816|
|4|1628|
|5|1248|
|6|518|
|7|180|
|8|67|
|9|28|
|10|7|
|11|2|
|12|1|

**Average number of guesses:** 4.49 +/- 1.26

### MaxUniqueLetterFrequencyScorer

This is a fairly naive selector. It selects the word that maximizes the sum of the frequency of
unique letters in the possible words.

**GuessFrom::PossibleWords**

|Num guesses|Num games|
|-----------|---------|
|1|1|
|2|137|
|3|1264|
|4|1831|
|5|829|
|6|321|
|7|129|
|8|57|
|9|26|
|10|6|
|11|1|

**Average number of guesses:** 4.16 +/- 1.22

**GuessFrom::AllUnguessedWords**

|Num guesses|Num games|
|-----------|---------|
|1|1|
|2|93|
|3|1056|
|4|1896|
|5|969|
|6|338|
|7|151|
|8|66|
|9|23|
|10|4|
|11|1|
|12|3|
|13|1|

**Average number of guesses:** 4.28 +/- 1.22

### MaxUniqueUnguessedLetterFrequencyScorer

This selects the word that maximizes the sum of the frequency of unique letters that have not yet
been guessed.

**GuessFrom::PossibleWords**

|Num guesses|Num games|
|-----------|---------|
|1|1|
|2|137|
|3|1264|
|4|1831|
|5|829|
|6|321|
|7|129|
|8|57|
|9|26|
|10|6|
|11|1|

**Average number of guesses:** 4.16 +/- 1.22

**GuessFrom::AllUnguessedWords**

|Num guesses|Num games|
|-----------|---------|
|1|1|
|2|32|
|3|1019|
|4|2227|
|5|1054|
|6|228|
|7|36|
|8|5|

**Average number of guesses:** 4.12 +/- 0.87

### LocatedLettersScorer

This selects the word that maximizes a score, based on both the presence and the the location of
that letter in the possible words. The score is computed for each letter and then summed. Each
letter is scored as follows.

For each letter, score:

* 1 point if the letter must be in this location.
* 1 point for every word with this letter in this place if the letter's location is not yet
  known, and this is a new location for the letter.
* If this letter is completely new:

   * If this letter has not yet been scored in this word:

      * 1 point for every possible word with this letter in the same place.
      * 1 point for every possible word with this letter in another place.
   
   * Else:

      * 1 point for every possible word with this letter in the same place.

|Num guesses|Num games|
|-----------|---------|
|1|1|
|2|114|
|3|1558|
|4|2023|
|5|633|
|6|180|
|7|62|
|8|22|
|9|7|
|10|2|

**Average number of guesses:** 3.90 +/- 0.99

### MaxApproximateEliminationsScorer

This selects the word that is expected to eliminate the most other words. The expected number of
eliminations is computed approximately. For each letter, expected number of eliminations is
computed for each possible state:

* *{expected number of eliminated words if in state}* * *{fraction of possible words matching this state}*

So for example, with the words `["could", "match", "coast"]`, these would be computed as follows
for the letter `c` in `could`:

* if correct: `match` is removed, so: 1 * (2/3)
* if present not here: `could` and `coast` are removed, so: 2 * (1/3)
* if not present: all are removed, so: 3 * (0/3) *(note: this expectation is skipped if this letter*
  *has already been checked at another location)*.

These per-letter expectations are then summed together to get the expectation value for the word.

|Num guesses|Num games|
|-----------|---------|
|1|1|
|2|72|
|3|1303|
|4|2507|
|5|664|
|6|52|
|7|3|

**Average number of guesses:** 3.85 +/- 0.72

## Speed Benchmark

### RandomGuesser

```
running 2 tests
test bench_guess_random_improved_words ... bench:     103,927 ns/iter (+/- 9,880)
test bench_guess_random_wordle_words   ... bench:     289,432 ns/iter (+/- 33,253)

test result: ok. 0 passed; 0 failed; 0 ignored; 2 measured; 0 filtered out; finished in 9.15s
```

### MaxUniqueLetterFrequencyScorer - 

**GuessFrom::PossibleWords**

```
running 2 tests
test bench_guess_random_improved_words ... bench:   1,874,060 ns/iter (+/- 102,103)
test bench_guess_random_wordle_words   ... bench:   5,359,493 ns/iter (+/- 256,125)

test result: ok. 0 passed; 0 failed; 0 ignored; 2 measured; 0 filtered out; finished in 8.79s
```

**GuessFrom::AllUnguessedWords**

```
running 2 tests
test bench_guess_random_improved_words ... bench:   3,727,482 ns/iter (+/- 1,167,432)
test bench_guess_random_wordle_words   ... bench:  12,523,837 ns/iter (+/- 6,333,476)

test result: ok. 0 passed; 0 failed; 0 ignored; 2 measured; 0 filtered out; finished in 7.25s
```

### MaxUniqueUnguessedLetterFrequencyScorer

**GuessFrom::PossibleWords**

```
running 2 tests
test bench_guess_random_improved_words ... bench:   1,838,435 ns/iter (+/- 79,831)
test bench_guess_random_wordle_words   ... bench:   5,257,480 ns/iter (+/- 217,477)

test result: ok. 0 passed; 0 failed; 0 ignored; 2 measured; 0 filtered out; finished in 8.59s
```

**GuessFrom::AllUnguessedWords**

```
running 2 tests
test bench_guess_random_improved_words ... bench:   3,598,302 ns/iter (+/- 719,643)
test bench_guess_random_wordle_words   ... bench:  10,532,152 ns/iter (+/- 2,592,960)

test result: ok. 0 passed; 0 failed; 0 ignored; 2 measured; 0 filtered out; finished in 6.48s
```

### ScoreLocatedLettersGuesser

```
running 2 tests
test bench_guess_random_improved_words ... bench:   3,542,067 ns/iter (+/- 803,738)
test bench_guess_random_wordle_words   ... bench:  10,692,953 ns/iter (+/- 3,955,187)

test result: ok. 0 passed; 0 failed; 0 ignored; 2 measured; 0 filtered out; finished in 6.56s
```

### MostExpectedEliminationsGuesser

```
running 2 tests
test bench_guess_random_improved_words ... bench:   3,352,427 ns/iter (+/- 569,059)
test bench_guess_random_wordle_words   ... bench:  10,573,207 ns/iter (+/- 2,574,954)

test result: ok. 0 passed; 0 failed; 0 ignored; 2 measured; 0 filtered out; finished in 6.23s
```

### MaxEliminationsScorer

```
test bench_max_eliminations_scorer_precomputed_random_improved_words ... bench: 661,249,325 ns/iter (+/- 108,450,673)
```