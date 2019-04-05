#![feature(test)]

extern crate test;

use measureme::{
    FileSerializationSink, MmapSerializationSink, testing_common
};

#[cfg(unix)] use measureme::AsyncMmapSerializationSink;

#[bench]
fn bench_file_serialization_sink(bencher: &mut test::Bencher) {
    bencher.iter(|| {
        testing_common::run_end_to_end_serialization_test::<FileSerializationSink>("file_serialization_sink_test");
    });
}

#[bench]
fn bench_mmap_serialization_sink(bencher: &mut test::Bencher) {
    bencher.iter(|| {
        testing_common::run_end_to_end_serialization_test::<MmapSerializationSink>("mmap_serialization_sink_test");
    });
}

#[cfg(unix)]
#[bench]
fn bench_async_mmap_serialization_sink(bencher: &mut test::Bencher) {
    bencher.iter(|| {
        testing_common::run_end_to_end_serialization_test::<AsyncMmapSerializationSink>("async_mmap_serialization_sink_test");
    });
}
