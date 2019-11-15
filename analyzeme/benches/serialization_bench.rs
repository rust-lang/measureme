#![feature(test)]

extern crate test;

use analyzeme::testing_common;
use measureme::{FileSerializationSink, MmapSerializationSink};

#[bench]
fn bench_file_serialization_sink(bencher: &mut test::Bencher) {
    bencher.iter(|| {
        testing_common::run_serialization_bench::<FileSerializationSink>(
            "file_serialization_sink_test", 500_000, 1
        );
    });
}

#[bench]
fn bench_mmap_serialization_sink(bencher: &mut test::Bencher) {
    bencher.iter(|| {
        testing_common::run_serialization_bench::<MmapSerializationSink>(
            "mmap_serialization_sink_test", 500_000, 1
        );
    });
}

#[bench]
fn bench_file_serialization_sink_8_threads(bencher: &mut test::Bencher) {
    bencher.iter(|| {
        testing_common::run_serialization_bench::<FileSerializationSink>(
            "file_serialization_sink_test", 50_000, 8
        );
    });
}

#[bench]
fn bench_mmap_serialization_sink_8_threads(bencher: &mut test::Bencher) {
    bencher.iter(|| {
        testing_common::run_serialization_bench::<MmapSerializationSink>(
            "mmap_serialization_sink_test", 50_000, 8
        );
    });
}
