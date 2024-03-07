use crate::{Event, EventPayload, ProfilingData, Timestamp};
use measureme::{EventId, EventIdBuilder, Profiler, StringId};
use rustc_hash::FxHashMap;
use std::borrow::Cow;
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

#[derive(Clone)]
struct ExpectedEvent {
    kind: Cow<'static, str>,
    label: Cow<'static, str>,
    args: Vec<Cow<'static, str>>,
}

impl ExpectedEvent {
    fn new(kind: &'static str, label: &'static str, args: &[&'static str]) -> ExpectedEvent {
        ExpectedEvent {
            kind: Cow::from(kind),
            label: Cow::from(label),
            args: args.iter().map(|&x| Cow::from(x)).collect(),
        }
    }
}

// Generate some profiling data. This is the part that would run in rustc.
fn generate_profiling_data(
    filestem: &Path,
    num_stacks: usize,
    num_threads: usize,
) -> Vec<Event<'static>> {
    let profiler = Arc::new(Profiler::new(Path::new(filestem)).unwrap());

    let event_id_virtual = EventId::from_label(StringId::new_virtual(42u64));
    let event_id_builder = EventIdBuilder::new(&profiler);

    let event_ids: Vec<(StringId, EventId)> = vec![
        (
            profiler.alloc_string("Generic"),
            EventId::from_label(profiler.alloc_string("SomeGenericActivity")),
        ),
        (profiler.alloc_string("Query"), event_id_virtual),
        (
            profiler.alloc_string("QueryWithArg"),
            event_id_builder.from_label_and_arg(
                profiler.alloc_string("AQueryWithArg"),
                profiler.alloc_string("some_arg"),
            ),
        ),
    ];

    // This and event_ids have to match!
    let expected_events_templates = vec![
        ExpectedEvent::new("Generic", "SomeGenericActivity", &[]),
        ExpectedEvent::new("Query", "SomeQuery", &[]),
        ExpectedEvent::new("QueryWithArg", "AQueryWithArg", &["some_arg"]),
    ];

    let threads: Vec<_> = (0..num_threads)
        .map(|thread_id| {
            let event_ids = event_ids.clone();
            let profiler = profiler.clone();
            let expected_events_templates = expected_events_templates.clone();

            std::thread::spawn(move || {
                let mut expected_events = Vec::new();

                for i in 0..num_stacks {
                    // Allocate some invocation stacks

                    pseudo_invocation(
                        &profiler,
                        i,
                        thread_id as u32,
                        4,
                        &event_ids[..],
                        &expected_events_templates,
                        &mut expected_events,
                    );
                }

                expected_events
            })
        })
        .collect();

    // An example of allocating the string contents of an event id that has
    // already been used
    profiler.map_virtual_to_concrete_string(
        event_id_virtual.to_string_id(),
        profiler.alloc_string("SomeQuery"),
    );

    drop(profiler);

    let expected_events: Vec<_> = threads
        .into_iter()
        .flat_map(|t| t.join().unwrap())
        .collect();

    expected_events
}

// Process some profiling data. This is the part that would run in a
// post processing tool.
fn process_profiling_data(filestem: &Path, expected_events: &[Event<'static>]) {
    let profiling_data = ProfilingData::new(filestem).unwrap();

    // Check iterating forward over the events
    check_profiling_data(
        &mut profiling_data.iter_full(),
        &mut expected_events.iter().cloned(),
        expected_events.len(),
    );
    // Check iterating backwards over the events
    check_profiling_data(
        &mut profiling_data.iter_full().rev(),
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

    // This assertion makes sure that the ExactSizeIterator impl works as expected.
    assert_eq!(
        (num_expected_events, Some(num_expected_events)),
        actual_events.size_hint()
    );

    let actual_events_per_thread = collect_events_per_thread(actual_events);
    let expected_events_per_thread = collect_events_per_thread(expected_events);

    let thread_ids: Vec<_> = actual_events_per_thread.keys().collect();
    assert_eq!(
        thread_ids,
        expected_events_per_thread.keys().collect::<Vec<_>>()
    );

    for thread_id in thread_ids {
        let actual_events = &actual_events_per_thread[thread_id];
        let expected_events = &expected_events_per_thread[thread_id];

        assert_eq!(actual_events.len(), expected_events.len());

        for (actual_event, expected_event) in actual_events.iter().zip(expected_events.iter()) {
            assert_eq!(actual_event.event_kind, expected_event.event_kind);
            assert_eq!(actual_event.label, expected_event.label);
            assert_eq!(actual_event.additional_data, expected_event.additional_data);
            assert_eq!(
                actual_event.payload.is_interval(),
                expected_event.payload.is_interval()
            );
            assert_eq!(
                actual_event.payload.is_instant(),
                expected_event.payload.is_instant()
            );

            if expected_event.payload.is_integer() {
                assert_eq!(actual_event.payload, expected_event.payload);
            }

            count += 1;
        }
    }

    assert_eq!(count, num_expected_events);
}

fn collect_events_per_thread<'a>(
    events: &mut dyn Iterator<Item = Event<'a>>,
) -> FxHashMap<u32, Vec<Event<'a>>> {
    let mut per_thread: FxHashMap<_, _> = Default::default();

    for event in events {
        per_thread
            .entry(event.thread_id)
            .or_insert(Vec::new())
            .push(event);
    }

    per_thread
}

pub fn run_serialization_bench(file_name_stem: &str, num_events: usize, num_threads: usize) {
    let filestem = mk_filestem(file_name_stem);
    generate_profiling_data(&filestem, num_events, num_threads);
}

pub fn run_end_to_end_serialization_test(file_name_stem: &str, num_threads: usize) {
    let filestem = mk_filestem(file_name_stem);
    let expected_events = generate_profiling_data(&filestem, 10_000, num_threads);
    process_profiling_data(&filestem, &expected_events);
}

fn pseudo_invocation(
    profiler: &Profiler,
    random: usize,
    thread_id: u32,
    recursions_left: usize,
    event_ids: &[(StringId, EventId)],
    expected_events_templates: &[ExpectedEvent],
    expected_events: &mut Vec<Event<'static>>,
) {
    if recursions_left == 0 {
        return;
    }

    let random_event_index = random % event_ids.len();

    let (event_kind, event_id) = event_ids[random_event_index];

    let _prof_guard = profiler.start_recording_interval_event(event_kind, event_id, thread_id);

    pseudo_integer_event(
        profiler,
        random * 7,
        thread_id,
        event_ids,
        expected_events_templates,
        expected_events,
    );

    pseudo_invocation(
        profiler,
        random * 17,
        thread_id,
        recursions_left - 1,
        event_ids,
        expected_events_templates,
        expected_events,
    );

    pseudo_instant_event(
        profiler,
        random * 23,
        thread_id,
        event_ids,
        expected_events_templates,
        expected_events,
    );

    expected_events.push(Event {
        event_kind: expected_events_templates[random_event_index].kind.clone(),
        label: expected_events_templates[random_event_index].label.clone(),
        additional_data: expected_events_templates[random_event_index].args.clone(),
        thread_id,
        // We can't test the actual timestamp value, so we just assign
        // SystemTime::UNIX_EPOCH to everything.
        payload: EventPayload::Timestamp(Timestamp::Interval {
            start: SystemTime::UNIX_EPOCH,
            end: SystemTime::UNIX_EPOCH,
        }),
    });
}

fn pseudo_integer_event(
    profiler: &Profiler,
    random: usize,
    thread_id: u32,
    event_ids: &[(StringId, EventId)],
    expected_events_templates: &[ExpectedEvent],
    expected_events: &mut Vec<Event<'static>>,
) {
    let random_event_index = random % event_ids.len();

    let payload_value = random as u64 * 33;

    let (event_kind, event_id) = event_ids[random_event_index];
    profiler.record_integer_event(event_kind, event_id, thread_id, payload_value);

    expected_events.push(Event {
        event_kind: expected_events_templates[random_event_index].kind.clone(),
        label: expected_events_templates[random_event_index].label.clone(),
        additional_data: expected_events_templates[random_event_index].args.clone(),
        thread_id,
        payload: EventPayload::Integer(payload_value),
    });
}

fn pseudo_instant_event(
    profiler: &Profiler,
    random: usize,
    thread_id: u32,
    event_ids: &[(StringId, EventId)],
    expected_events_templates: &[ExpectedEvent],
    expected_events: &mut Vec<Event<'static>>,
) {
    let random_event_index = random % event_ids.len();

    let (event_kind, event_id) = event_ids[random_event_index];
    profiler.record_instant_event(event_kind, event_id, thread_id);

    expected_events.push(Event {
        event_kind: expected_events_templates[random_event_index].kind.clone(),
        label: expected_events_templates[random_event_index].label.clone(),
        additional_data: expected_events_templates[random_event_index].args.clone(),
        thread_id,
        // We can't test the actual timestamp value, so we just assign
        // SystemTime::UNIX_EPOCH to everything.
        payload: EventPayload::Timestamp(Timestamp::Instant(SystemTime::UNIX_EPOCH)),
    });
}
