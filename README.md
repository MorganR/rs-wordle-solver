# wordle-solver

An automated solver for the popular game: [Wordle](https://www.nytimes.com/games/wordle/index.html)

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
unique letters that have not yet been guessed.

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

**GuessFrom::PossibleWords**

|Num guesses|Num games|
|-----------|---------|
|1|1|
|2|180|
|3|1442|
|4|1838|
|5|722|
|6|259|
|7|101|
|8|41|
|9|13|
|10|3|
|11|1|
|12|1|

**Average number of guesses:** 4.00 +/- 1.15

**GuessFrom::AllUnguessedWords**

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

This selects the word that is expected to eliminate approximately the most other words.
For each letter, the expected number of eliminations is computed for each possible state:

* *{expected number of eliminated words if in state}* * *{fraction of possible words matching this state}*

So for example, with the words `["could", "match", "coast"]`, these would be computed as follows
for the letter `c` in `could`:

* if correct: `match` is removed, so: 1 * (2/3)
* if present not here: `could` and `coast` are removed, so: 2 * (1/3)
* if not present: all are removed, so: 3 * (0/3) *(note: this expectation is skipped if this letter*
  *has already been checked at another location)*.

These per-letter expectations are then summed together to get the expectation value for the word.
Approximating the expected eliminations in this way is cheap to compute, but slightly less accurate,
and therefore less effective, than using the precise counts computed by `MaxEliminationsScorer`.

**GuessFrom::PossibleWords**

|Num guesses|Num games|
|-----------|---------|
|1|1|
|2|180|
|3|1415|
|4|1843|
|5|734|
|6|262|
|7|104|
|8|41|
|9|14|
|10|6|
|11|1|
|12|1|

**Average number of guesses:** 4.02 +/- 1.16

**GuessFrom::AllUnguessedWords**

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

### MaxEliminationsScorer

This probabilistically calculates the expectation value for how many words will be eliminated by 
each guess, and chooses the word that eliminates the most other guesses. This is extremely expensive
to compute, so it precomputes as much as possible when the scorer is first created. On my machine, 
constructing the scorer takes about 9 seconds, but this enables each subsequent game to be played 
in about 650ms.

**GuessFrom::PossibleWords**

|Num guesses|Num games|
|-----------|---------|
|1|1|
|2|180|
|3|1452|
|4|1942|
|5|666|
|6|220|
|7|93|
|8|33|
|9|10|
|10|4|
|11|1|

**Average number of guesses:** 3.95 +/- 1.10

**GuessFrom::AllUnguessedWords**

|Num guesses|Num games|
|-----------|---------|
|1|1|
|2|53|
|3|1426|
|4|2635|
|5|468|
|6|19|

**Average number of guesses:** 3.78 +/- 0.65

## Speed Benchmark

### RandomGuesser

```
test bench_guess_random_improved_words ... bench:      97,871 ns/iter (+/- 7,004)
test bench_guess_random_wordle_words   ... bench:     271,783 ns/iter (+/- 26,410)
```

### MaxUniqueLetterFrequencyScorer

```
test bench_unique_letters_improved_words ... bench:   1,124,631 ns/iter (+/- 152,674)
```

### LocatedLettersScorer

```
test bench_located_letters_improved_words ... bench:   2,139,709 ns/iter (+/- 399,150)
```

### MaxApproximateEliminationsScorer

```
test bench_max_approximate_eliminations_random_improved_words ... bench:   2,215,908 ns/iter (+/- 379,109)
```

### MaxEliminationsScorer

```
test bench_max_eliminations_scorer_precomputed_random_improved_words ... bench: 655,816,300 ns/iter (+/- 107,276,732)
```