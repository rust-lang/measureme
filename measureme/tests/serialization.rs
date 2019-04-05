
#[cfg(unix)] use measureme::AsyncMmapSerializationSink;
use measureme::{FileSerializationSink, MmapSerializationSink};
use measureme::testing_common::run_end_to_end_serialization_test;

#[test]
fn test_file_serialization_sink() {
    run_end_to_end_serialization_test::<FileSerializationSink>("file_serialization_sink_test");
}

#[test]
fn test_mmap_serialization_sink() {
    run_end_to_end_serialization_test::<MmapSerializationSink>("mmap_serialization_sink_test");
}

#[cfg(unix)]
#[test]
fn test_unix_mmap_serialization_sink() {
    run_end_to_end_serialization_test::<AsyncMmapSerializationSink>("async_mmap_serialization_sink_test");
}
