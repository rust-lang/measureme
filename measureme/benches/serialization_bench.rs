#![feature(test)]

extern crate test;

use measureme::{testing_common, FileSerializationSink, MmapSerializationSink};

#[bench]
fn bench_file_serialization_sink(bencher: &mut test::Bencher) {
    bencher.iter(|| {
        testing_common::run_end_to_end_serialization_test::<FileSerializationSink>(
            "file_serialization_sink_test",
        );
    });
}

#[bench]
fn bench_mmap_serialization_sink(bencher: &mut test::Bencher) {
    bencher.iter(|| {
        testing_common::run_end_to_end_serialization_test::<MmapSerializationSink>(
            "mmap_serialization_sink_test",
        );
    });
}
