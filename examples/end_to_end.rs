use measureme::{FileSerializationSink, Profiler, ProfilingData, StringId, TimestampKind};
use std::path::Path;
use std::sync::Arc;

const PROFILE_FILENAME_STEM: &str = "testprofile";

// Generate some profiling data. This is the part that would run in rustc.
fn generate_profiling_data() {
    let profiler = Arc::new(Profiler::<FileSerializationSink>::new(Path::new(
        PROFILE_FILENAME_STEM,
    )));

    let event_kind_query_provider = profiler.alloc_string("Query");
    let event_kind_generic = profiler.alloc_string("Generic");

    let event_id_some_query = StringId::reserved(42);
    let event_id_some_generic_activity = profiler.alloc_string("SomeGenericActivity");

    let event_ids = &[
        (event_kind_generic, event_id_some_generic_activity),
        (event_kind_query_provider, event_id_some_query),
    ];

    let mut started_events = Vec::new();

    for i in 0..10_000 {
        // Allocate some invocation stacks
        for _ in 0..4 {
            let thread_id = (i % 3) as u64;
            let (event_kind, event_id) = event_ids[i % event_ids.len()];

            profiler.record_event(event_kind, event_id, thread_id, TimestampKind::Start);

            started_events.push((event_kind, event_id, thread_id));
        }

        while let Some((event_kind, event_id, thread_id)) = started_events.pop() {
            profiler.record_event(event_kind, event_id, thread_id, TimestampKind::End);
        }
    }

    // An example of allocating the string contents of an event id that has
    // already been used
    profiler.alloc_string_with_reserved_id(event_id_some_query, "SomeQuery");
}

// Process some profiling data. This is the part that would run in a
// post processing tool.
fn process_profiling_data() {
    let profiling_data = ProfilingData::new(Path::new(PROFILE_FILENAME_STEM));

    profiling_data.iter_events(|_event| {});
}

fn main() {
    generate_profiling_data();
    process_profiling_data();
}
