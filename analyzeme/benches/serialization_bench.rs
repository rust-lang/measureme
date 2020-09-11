#![feature(test)]

extern crate test;

use analyzeme::testing_common;

#[bench]
fn bench_file_serialization_sink(bencher: &mut test::Bencher) {
    bencher.iter(|| {
        testing_common::run_serialization_bench("file_serialization_sink_test", 500_000, 1);
    });
}

#[bench]
fn bench_file_serialization_sink_8_threads(bencher: &mut test::Bencher) {
    bencher.iter(|| {
        testing_common::run_serialization_bench("file_serialization_sink_test", 50_000, 8);
    });
}
