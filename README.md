# rs-wordle-solver

An automated solving library for the popular game: [Wordle](https://www.nytimes.com/games/wordle/index.html)

## How to use it?

See [the docs](https://docs.rs/rs-wordle-solver/).

## Solve efficiency benchmarks

Different guessing algorithms have been benchmarked against a few word lists:

*  `wordle-answers.txt`: this list is the all answer words from
   [Wordle](https://www.nytimes.com/games/wordle/index.html) before it was purchased by the New
   York Times.

*  `improved-words.txt`: this list combines all 5-letter words from the [Corncob list of more than
   58,000 English words](http://www.mieliestronk.com/wordlist.html), the [MIT 10000 English words
   list](https://www.mit.edu/~ecprice/wordlist.10000), and any remaining Wordle answer words. This
   is often used instead of the full wordle word list, as many allowed Wordle words seem to be
   somewhat nonsensical.

Most algorithms have been benchmarked against the whole *improved* words list. Some algorithms run
in two configurations:

1.  guessing from `PossibleWords`, i.e. only guessing words that are still possible. This is
    equivalent to Wordle's "hard" mode. 
2.  guessing from `AllUnguessedWords`, including words that can't be the answer. This is usually
    better as the algorithms are able to eliminate more words faster.

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
each guess, and chooses the word that eliminates the most other guesses. This is relatively
expensive to compute, so it precomputes as much as possible when the scorer is first created. On my
machine, constructing the scorer with the improved-words list takes about 1.4 seconds, but this
enables each subsequent game to be played in about 27ms.

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

When benchmarked against the wordle answers using the improved-words list as a word bank, the
results are:

|Num guesses|Num games|
|-----------|---------|
|2|23|
|3|777|
|4|1364|
|5|148|
|6|3|

**Average number of guesses:** 3.71 +/- 0.60 (time taken: 20.6 seconds)

### MaxComboEliminationsScorer

This extends the `MaxEliminationsScorer` to choose the word that should maximize the number of
eliminated words across the next two guesses, as long as there are more than a specifiable limit
of possible words. We'll call this the `combo limit`. Once there are fewer than the `combo limit`
possible words left, this falls back to the `MaxEliminationsScorer` behavior.

The theory behind this scorer was that, since most words take at least 3 guesses, the scorer should
try to eliminate the most possible words collectively in the first two guesses. This could perform
differently from choosing the best individual word per round, as is done in
`MaxEliminationsScorer`. Empirically, it seems to perform worse (see below).

This is **extremely expensive** to compute, as it scales in approximately *O*(*n*<super>3</super>)
where `n` is the number of words in the word bank. Similar to `MaxEliminationScorer`, it
precomputes the first guess, which takes *hours*. For that reason, this was only benchmarked
against the wordle answers using the improved-words list as a word bank. It was benchmarked in two
configurations: setting the `combo limit` to 100, and setting the `combo limit` to 1000.

**Combo limit: 100**

|Num guesses|Num games|
|-----------|---------|
|2|19|
|3|654|
|4|1414|
|5|222|
|6|6|

**Average number of guesses:** 3.80 +/- 0.62 (time taken: 41 hours 12 minutes)

**Combo limit: 1000**

|Num guesses|Num games|
|-----------|---------|
|2|20|
|3|719|
|4|1410|
|5|163|
|6|3|

**Average number of guesses:** 3.75 +/- 0.60 (time taken: 6 hours 14 minutes)

## Speed benchmarks

### RandomGuesser

```
test bench_guess_random_improved_words ... bench:      97,871 ns/iter (+/- 7,004)
test bench_guess_random_wordle_words   ... bench:     271,783 ns/iter (+/- 26,410)
```

### MaxUniqueLetterFrequencyScorer

```
test bench_unique_letters_improved_words                        ... bench:   1,414,721 ns/iter (+/- 177,899)
```

### LocatedLettersScorer

```
test bench_located_letters_improved_words                       ... bench:   2,035,637 ns/iter (+/- 387,882)
```

### MaxApproximateEliminationsScorer

```
test bench_max_approximate_eliminations_improved_words          ... bench:   2,378,306 ns/iter (+/- 399,961)
```

### MaxEliminationsScorer

```
test bench_max_eliminations_scorer_no_precompute_improved_words ... bench: 1,384,103,132 ns/iter (+/- 39,615,208)
test bench_max_eliminations_scorer_precomputed_improved_words   ... bench:  27,068,036 ns/iter (+/- 32,937,953)
```

### MaxComboEliminationsScorer

Precompute on improved words takes roughly: 5 hours 45 minutes