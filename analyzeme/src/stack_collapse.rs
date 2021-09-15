use rustc_hash::FxHashMap;
use std::cmp;
use std::time::SystemTime;

use crate::{LightweightEvent, ProfilingData};

// This state is kept up-to-date while iteration over events.
struct PerThreadState {
    stack: Vec<LightweightEvent>,
    stack_id: String,
    start: SystemTime,
    end: SystemTime,
    total_event_time_nanos: u64,
}

/// Collect a map of all stacks and how many nanoseconds are spent in each.
/// Uses a variation of the algorithm in `summarize`.
// Original implementation provided by @andjo403 in
// https://github.com/michaelwoerister/measureme/pull/1
pub fn collapse_stacks<'a>(profiling_data: &ProfilingData) -> FxHashMap<String, u64> {
    let mut counters = FxHashMap::default();
    let mut threads = FxHashMap::<_, PerThreadState>::default();

    for current_event in profiling_data
        .iter()
        .rev()
        .filter(|e| e.payload.is_interval())
    {
        let start = current_event.start().unwrap();
        let end = current_event.end().unwrap();
        let thread = threads
            .entry(current_event.thread_id)
            .or_insert(PerThreadState {
                stack: Vec::new(),
                stack_id: "rustc".to_owned(),
                start,
                end,
                total_event_time_nanos: 0,
            });

        thread.start = cmp::min(thread.start, start);

        // Pop all events from the stack that are not parents of the
        // current event.
        while let Some(current_top) = thread.stack.last().cloned() {
            if current_top.contains(&current_event) {
                break;
            }

            let popped = thread.stack.pop().unwrap();
            let popped = profiling_data.to_full_event(&popped);
            let new_stack_id_len = thread.stack_id.len() - (popped.label.len() + 1);
            thread.stack_id.truncate(new_stack_id_len);
        }

        if !thread.stack.is_empty() {
            // If there is something on the stack, subtract the current
            // interval from it.
            counters
                .entry(thread.stack_id.clone())
                .and_modify(|self_time| {
                    *self_time -= current_event.duration().unwrap().as_nanos() as u64;
                });
        } else {
            // Update the total_event_time_nanos counter as the current event
            // is on top level
            thread.total_event_time_nanos += current_event.duration().unwrap().as_nanos() as u64;
        }

        // Add this event to the stack_id
        thread.stack_id.push(';');
        thread
            .stack_id
            .push_str(&profiling_data.to_full_event(&current_event).label[..]);

        // Update current events self time
        let self_time = counters.entry(thread.stack_id.clone()).or_default();
        *self_time += current_event.duration().unwrap().as_nanos() as u64;

        // Bring the stack up-to-date
        thread.stack.push(current_event)
    }

    // Finally add a stack that accounts for the gaps between any recorded
    // events.
    let mut rustc_time = 0;
    for thread in threads.values() {
        // For each thread we take the time between the start of the first and
        // the end of the last event, and subtract the duration of all top-level
        // events of that thread. That leaves us with the duration of all gaps
        // on the threads timeline.
        rustc_time += thread.end.duration_since(thread.start).unwrap().as_nanos() as u64
            - thread.total_event_time_nanos;
    }
    counters.insert("rustc".to_owned(), rustc_time);

    counters
}

#[cfg(test)]
mod test {
    use crate::ProfilingDataBuilder;
    use rustc_hash::FxHashMap;

    #[test]
    fn basic_test() {
        let mut b = ProfilingDataBuilder::new();

        //                                                 <------e3------>
        //                                         <--------------e1-------------->
        //                 <--e1-->        <------------------------e2-------------------->
        //         thread0 1       2       3       4       5       6       7       8       9
        //
        // stacks count:
        // rustc                   1
        // rustc;e1        1
        // rustc;e2                        1                                       2
        // rustc;e2;e1                             1                       2
        // rustc;e2;e1;e3                                  1       2

        b.interval("Query", "e1", 0, 1, 2, |_| {});
        b.interval("Query", "e2", 0, 3, 9, |b| {
            b.interval("Query", "e1", 0, 4, 8, |b| {
                b.interval("Query", "e3", 0, 5, 7, |_| {});
                // Integer events are expected to be ignored
                b.integer("ArtifactSize", "e4", 0, 100);
            });
        });

        let profiling_data = b.into_profiling_data();

        let recorded_stacks = super::collapse_stacks(&profiling_data);

        let mut expected_stacks = FxHashMap::<String, u64>::default();
        expected_stacks.insert("rustc;e2;e1;e3".into(), 2);
        expected_stacks.insert("rustc;e2;e1".into(), 2);
        expected_stacks.insert("rustc;e2".into(), 2);
        expected_stacks.insert("rustc;e1".into(), 1);
        expected_stacks.insert("rustc".into(), 1);

        assert_eq!(expected_stacks, recorded_stacks);
    }

    #[test]
    fn multi_threaded_test() {
        let mut b = ProfilingDataBuilder::new();

        //                 <--e1-->        <--e1-->
        //         thread1 1       2       3       4       5
        //                                 <--e3-->
        //                 <--e1--><----------e2---------->
        //         thread2 1       2       3       4       5
        //
        // stacks count:
        // rustc                   1
        // rustc;e1        2               3
        // rustc;e2                1               2
        // rustc;e2;e3                     1

        b.interval("Query", "e1", 1, 1, 2, |_| {});
        b.interval("Query", "e1", 1, 3, 4, |_| {});
        b.interval("Query", "e1", 2, 1, 2, |b| {
            b.instant("Instant", "e4", 2, 100);
        });
        b.interval("Query", "e2", 2, 2, 5, |b| {
            b.interval("Query", "e3", 2, 3, 4, |_| {});
            b.integer("ArtifactSize", "e4", 2, 1);
        });

        let profiling_data = b.into_profiling_data();

        let recorded_stacks = super::collapse_stacks(&profiling_data);

        let mut expected_stacks = FxHashMap::<String, u64>::default();
        expected_stacks.insert("rustc;e2;e3".into(), 1);
        expected_stacks.insert("rustc;e2".into(), 2);
        expected_stacks.insert("rustc;e1".into(), 3);
        expected_stacks.insert("rustc".into(), 1);

        assert_eq!(expected_stacks, recorded_stacks);
    }
}
