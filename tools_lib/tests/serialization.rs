use measureme::{FileSerializationSink, MmapSerializationSink};
use tools_lib::testing_common::run_end_to_end_serialization_test;

#[test]
fn test_file_serialization_sink() {
    run_end_to_end_serialization_test::<FileSerializationSink>("file_serialization_sink_test");
}

#[test]
fn test_mmap_serialization_sink() {
    run_end_to_end_serialization_test::<MmapSerializationSink>("mmap_serialization_sink_test");
}
