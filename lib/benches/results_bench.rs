#![feature(test)]

extern crate test;

use rs_wordle_solver::*;
use test::Bencher;

#[bench]
fn bench_get_result_for_guess_correct(b: &mut Bencher) {
    b.iter(|| get_result_for_guess("abcbd", "abcbd"))
}

#[bench]
fn bench_get_result_for_guess_partial(b: &mut Bencher) {
    b.iter(|| get_result_for_guess("mesas", "sassy"))
}
