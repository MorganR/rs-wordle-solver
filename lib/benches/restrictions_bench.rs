#![feature(test)]

extern crate test;

use rs_wordle_solver::details::*;
use rs_wordle_solver::*;
use test::Bencher;

#[bench]
fn bench_restrictions_from_result_correct(b: &mut Bencher) {
    let result = get_result_for_guess("abcbd", "abcbd").unwrap();

    b.iter(|| WordRestrictions::from_result(&result));
}

#[bench]
fn bench_restrictions_from_result_mixed(b: &mut Bencher) {
    let result = get_result_for_guess("abcbd", "blkda").unwrap();

    b.iter(|| WordRestrictions::from_result(&result));
}

#[bench]
fn bench_restrictions_from_result_not_present(b: &mut Bencher) {
    let result = get_result_for_guess("abcbd", "zywxv").unwrap();

    b.iter(|| WordRestrictions::from_result(&result));
}

#[bench]
fn bench_restrictions_is_satisfied_by_correct(b: &mut Bencher) {
    let result = get_result_for_guess("abcbd", "abcbd").unwrap();
    let restrictions = WordRestrictions::from_result(&result);

    b.iter(|| restrictions.is_satisfied_by("abcbd"));
}

#[bench]
fn bench_restrictions_is_satisfied_by_mixed(b: &mut Bencher) {
    let result = get_result_for_guess("abcbd", "blkba").unwrap();
    let restrictions = WordRestrictions::from_result(&result);

    b.iter(|| {
        restrictions.is_satisfied_by("abcef")
            | restrictions.is_satisfied_by("abcbd")
            | restrictions.is_satisfied_by("accbd")
            | restrictions.is_satisfied_by("blkba")
            | restrictions.is_satisfied_by("zzzzz")
    });
}

#[bench]
fn bench_restrictions_is_satisfied_by_unknown(b: &mut Bencher) {
    let result = get_result_for_guess("abcbd", "zywxv").unwrap();
    let restrictions = WordRestrictions::from_result(&result);

    b.iter(|| restrictions.is_satisfied_by("abcbd"));
}

#[bench]
fn bench_restrictions_is_satisfied_by_not_present(b: &mut Bencher) {
    let result = get_result_for_guess("abcbd", "zywxv").unwrap();
    let restrictions = WordRestrictions::from_result(&result);

    b.iter(|| restrictions.is_satisfied_by("zywxv"));
}
