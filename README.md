# wordle-solver

An automated solver for the popular game: [wordle](https://www.powerlanguage.co.uk/wordle/)

## Available words

This library has several word lists:

*  `data/wordle-words.txt`: this list is the combination of all possible guesses and all answer words from [Wordle](https://www.powerlanguage.co.uk/wordle/). Many of the allowed Wordle guesses are pretty much nonsense, so there is also:

*  `data/improved-words.txt`: this list combines all 5-letter words from the [Corncob list of more than 58,000 English words](http://www.mieliestronk.com/wordlist.html), the [MIT 10000 English words list](https://www.mit.edu/~ecprice/wordlist.10000), and any remaining Wordle answer words.

*  A random set of 1000 words taken from each list.

## Guesses Benchmark

When benchmarked against the whole *improved* words list:

### MaxUniqueLetterFrequencySelector

This is a fairly naive selector. It selects the word that maximizes the sum of the frequency of
unique letters in the remaining possible words.

|Num guesses|Num games|
|-----------|---------|
|1|1|
|2|137|
|3|1264|
|4|1828|
|5|831|
|6|322|
|7|129|
|8|57|
|9|26|
|10|6|
|11|1|

**Average number of guesses:** 4.16 +/- 1.22

### MaxUnguessedUniqueLetterFrequencySelector

This selects the word that maximizes the sum of the frequency of unique letters that have not yet
been guessed in the remaining possible words. It can select a guess from words that could not
possibly be the answer in order to maximize the information gained per guess. This results in fewer
lucky guesses early on, but with a dramatically improved long tail. 

|Num guesses|Num games|
|-----------|---------|
|1|1|
|2|32|
|3|1030|
|4|2183|
|5|1078|
|6|233|
|7|42|
|8|3|

**Average number of guesses:** 4.13 +/- 0.88

### ScoreLocatedLettersGuesser

This selects the word that maximizes a score, based on both the presence and the the location of
that letter in the possible words. The score is computed for each letter and then summed. Each
letter is scored as follows:

For each letter, score:

* 1 point if the letter must be in this location.
* 1 point for every word with this letter in this place if the letter's location is not yet
  known, and this is a new location for the letter.
* If this letter is completely new:

   * If this letter has not yet been scored in this word:

      * 2 points for every possible word with this letter in the same place.
      * 1 point for every possible word with this letter in another place.
   
   * Else:

      * 1 point for every possible word with this letter in the same place.

|Num guesses|Num games|
|-----------|---------|
|1|1|
|2|180|
|3|1443|
|4|1831|
|5|725|
|6|262|
|7|101|
|8|41|
|9|13|
|10|3|
|11|1|
|12|1|

**Average number of guesses:** 4.00 +/- 1.15

## Speed Benchmark

### MaxUniqueLetterFrequencySelector

```
running 2 tests
test bench_guess_random_improved_words ... bench:   1,839,250 ns/iter (+/- 93,806)
test bench_guess_random_wordle_words   ... bench:   5,134,434 ns/iter (+/- 278,388)

test result: ok. 0 passed; 0 failed; 0 ignored; 2 measured; 0 filtered out; finished in 8.49s
```

### MaxUnguessedUniqueLetterFrequencySelector

Engine benchmark result:

```
running 2 tests
test bench_guess_random_improved_words ... bench:   3,944,031 ns/iter (+/- 863,425)
test bench_guess_random_wordle_words   ... bench:  11,516,727 ns/iter (+/- 3,847,099)

test result: ok. 0 passed; 0 failed; 0 ignored; 2 measured; 0 filtered out; finished in 7.09s
```

### ScoreLocatedLettersGuesser

```
running 2 tests
test bench_guess_random_improved_words ... bench:   1,961,780 ns/iter (+/- 44,518)
test bench_guess_random_wordle_words   ... bench:   5,502,857 ns/iter (+/- 237,024)

test result: ok. 0 passed; 0 failed; 0 ignored; 2 measured; 0 filtered out; finished in 9.11s
```