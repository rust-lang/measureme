use crate::{Event, Profiler, ProfilingData, SerializationSink, StringId, Timestamp};
use rustc_hash::FxHashMap;
use std::borrow::Cow;
use std::default::Default;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

fn mk_filestem(file_name_stem: &str) -> PathBuf {
    let mut path = PathBuf::new();

    path.push("test-tmp");
    path.push("end_to_end_serialization");
    path.push(file_name_stem);

    path
}

// Generate some profiling data. This is the part that would run in rustc.
fn generate_profiling_data<S: SerializationSink>(filestem: &Path) -> Vec<Event<'static>> {
    let profiler = Arc::new(Profiler::<S>::new(Path::new(filestem)).unwrap());

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

    for i in 0..10_000 {
        // Allocate some invocation stacks

        pseudo_invocation(
            &profiler,
            i,
            4,
            event_ids,
            &event_ids_as_str,
            &mut expected_events,
        );
    }

    // An example of allocating the string contents of an event id that has
    // already been used
    profiler.alloc_string_with_reserved_id(event_id_reserved, "SomeQuery");

    expected_events
}

// Process some profiling data. This is the part that would run in a
// post processing tool.
fn process_profiling_data(filestem: &Path, expected_events: &[Event<'static>]) {
    let profiling_data = ProfilingData::new(filestem).unwrap();

    check_profiling_data(
        &mut profiling_data.iter(),
        &mut expected_events.iter().cloned(),
        expected_events.len(),
    );
    check_profiling_data(
        &mut profiling_data.iter().rev(),
        &mut expected_events.iter().rev().cloned(),
        expected_events.len(),
    );
}

fn check_profiling_data(
    actual_events: &mut dyn Iterator<Item = Event<'_>>,
    expected_events: &mut dyn Iterator<Item = Event<'_>>,
    num_expected_events: usize,
) {
    let mut count = 0;

    assert_eq!(
        (num_expected_events, Some(num_expected_events)),
        actual_events.size_hint()
    );

    for (actual_event, expected_event) in actual_events.zip(expected_events) {
        assert_eq!(actual_event.event_kind, expected_event.event_kind);
        assert_eq!(actual_event.label, expected_event.label);
        assert_eq!(actual_event.additional_data, expected_event.additional_data);
        assert_eq!(
            actual_event.timestamp.is_instant(),
            expected_event.timestamp.is_instant()
        );

        count += 1;
    }
    assert_eq!(count, num_expected_events);
}

pub fn run_end_to_end_serialization_test<S: SerializationSink>(file_name_stem: &str) {
    let filestem = mk_filestem(file_name_stem);
    let expected_events = generate_profiling_data::<S>(&filestem);
    process_profiling_data(&filestem, &expected_events);
}

fn pseudo_invocation<S: SerializationSink>(
    profiler: &Profiler<S>,
    random: usize,
    recursions_left: usize,
    event_ids: &[(StringId, StringId)],
    event_ids_as_str: &FxHashMap<StringId, &'static str>,
    expected_events: &mut Vec<Event<'static>>,
) {
    if recursions_left == 0 {
        return;
    }

    let thread_id = (random % 3) as u64;

    let (event_kind, event_id) = event_ids[random % event_ids.len()];

    let _prof_guard = profiler.start_recording_interval_event(event_kind, event_id, thread_id);

    pseudo_invocation(
        profiler,
        random,
        recursions_left - 1,
        event_ids,
        event_ids_as_str,
        expected_events,
    );

    expected_events.push(Event {
        event_kind: Cow::from(event_ids_as_str[&event_kind]),
        label: Cow::from(event_ids_as_str[&event_id]),
        additional_data: &[],
        // We can't test this anyway:
        timestamp: Timestamp::Interval {
            start: SystemTime::UNIX_EPOCH,
            end: SystemTime::UNIX_EPOCH,
        },
        thread_id,
    });
}
