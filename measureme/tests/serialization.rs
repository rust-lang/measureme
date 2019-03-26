use measureme::{
    Event, FileSerializationSink, Profiler, ProfilingData, SerializationSink, StringId,
    TimestampKind,
};
use rustc_hash::FxHashMap;
use std::borrow::Cow;
use std::default::Default;
use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;

// Generate some profiling data. This is the part that would run in rustc.
fn generate_profiling_data<S: SerializationSink>(filestem: &str) -> Vec<Event> {
    let profiler = Arc::new(Profiler::<S>::new(Path::new(filestem)));

    let event_id_reserved = StringId::reserved(42);

    let event_ids = &[
        (
            profiler.alloc_string("Generic"),
            profiler.alloc_string("SomeGenericActivity"),
        ),
        (profiler.alloc_string("Query"), event_id_reserved),
    ];

    // This and event_ids have to match!
    let mut event_ids_as_str: FxHashMap<_, _> = Default::default();
    event_ids_as_str.insert(event_ids[0].0, "Generic");
    event_ids_as_str.insert(event_ids[0].1, "SomeGenericActivity");
    event_ids_as_str.insert(event_ids[1].0, "Query");
    event_ids_as_str.insert(event_ids[1].1, "SomeQuery");

    let mut expected_events = Vec::new();
    let mut started_events = Vec::new();

    for i in 0..10_000 {
        // Allocate some invocation stacks
        for _ in 0..4 {
            let thread_id = (i % 3) as u64;

            let (event_kind, event_id) = event_ids[i % event_ids.len()];

            profiler.record_event(event_kind, event_id, thread_id, TimestampKind::Start);
            started_events.push((event_kind, event_id, thread_id));

            expected_events.push(Event {
                event_kind: Cow::from(event_ids_as_str[&event_kind]),
                label: Cow::from(event_ids_as_str[&event_id]),
                additional_data: &[],
                timestamp: SystemTime::UNIX_EPOCH, // We can't test this anyway,
                timestamp_kind: TimestampKind::Start,
            });
        }

        while let Some((event_kind, event_id, thread_id)) = started_events.pop() {
            profiler.record_event(event_kind, event_id, thread_id, TimestampKind::End);

            expected_events.push(Event {
                event_kind: Cow::from(event_ids_as_str[&event_kind]),
                label: Cow::from(event_ids_as_str[&event_id]),
                additional_data: &[],
                timestamp: SystemTime::UNIX_EPOCH, // We can't test this anyway,
                timestamp_kind: TimestampKind::End,
            });
        }
    }

    // An example of allocating the string contents of an event id that has
    // already been used
    profiler.alloc_string_with_reserved_id(event_id_reserved, "SomeQuery");

    expected_events
}

// Process some profiling data. This is the part that would run in a
// post processing tool.
fn process_profiling_data(filestem: &str, expected_events: &[Event]) {
    let profiling_data = ProfilingData::new(Path::new(filestem));

    let mut count = 0;

    for (actual_event, expected_event) in profiling_data.iter().zip(expected_events.iter()) {
        eprintln!("{:?}", actual_event);

        assert_eq!(actual_event.event_kind, expected_event.event_kind);
        assert_eq!(actual_event.label, expected_event.label);
        assert_eq!(actual_event.additional_data, expected_event.additional_data);
        assert_eq!(actual_event.timestamp_kind, expected_event.timestamp_kind);

        count += 1;
    }

    assert_eq!(count, expected_events.len());
}

#[test]
fn test_file_serialization_sink() {
    let expected_events =
        generate_profiling_data::<FileSerializationSink>("file_serialization_sink_test");
    process_profiling_data("file_serialization_sink_test", &expected_events);
}

// #[test]
// fn test_mmap_serialization_sink() {
//     let expected_events =
//         generate_profiling_data::<MmapSerializationSink>("file_serialization_sink_test");
//     process_profiling_data("file_serialization_sink_test", &expected_events);
// }
