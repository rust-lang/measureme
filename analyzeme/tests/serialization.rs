use analyzeme::testing_common::run_end_to_end_serialization_test;
use measureme::{FileSerializationSink, MmapSerializationSink};

#[test]
fn test_file_serialization_sink_1_thread() {
    run_end_to_end_serialization_test::<FileSerializationSink>("file_serialization_sink_test_1_thread", 1);
}

#[test]
fn test_file_serialization_sink_8_threads() {
    run_end_to_end_serialization_test::<FileSerializationSink>("file_serialization_sink_test_8_threads", 8);
}

#[test]
fn test_mmap_serialization_sink_1_thread() {
    run_end_to_end_serialization_test::<MmapSerializationSink>("mmap_serialization_sink_test_1_thread", 1);
}

#[test]
fn test_mmap_serialization_sink_8_threads() {
    run_end_to_end_serialization_test::<MmapSerializationSink>("mmap_serialization_sink_test_8_threads", 8);
}
