use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use measureme::{Event, TimestampKind};

pub fn collapse_stacks<'a>(
    events: impl Iterator<Item = Event<'a>>,
    interval: u64,
) -> HashMap<String, usize> {
    let mut recorded_stacks = HashMap::<String, usize>::new();
    let mut thread_stacks: HashMap<u64, (SystemTime, Vec<Event>)> = HashMap::new();

    for event in events {
        let (next_observation_time, thread_stack) = thread_stacks
            .entry(event.thread_id)
            .or_insert((event.timestamp, Vec::new()));
        //if this event is after the next_observation_time then we need to record the current stacks
        if event.timestamp > *next_observation_time {
            let mut stack_string = String::new();
            stack_string.push_str("rustc");

            for event in thread_stack.iter() {
                stack_string.push(';');
                stack_string.push_str(&event.label);
            }

            let count = recorded_stacks.entry(stack_string).or_default();

            while event.timestamp > *next_observation_time {
                *count += 1;
                *next_observation_time += Duration::from_millis(interval);
            }
        }

        match event.timestamp_kind {
            TimestampKind::Start => {
                thread_stack.push(event);
            }
            TimestampKind::End => {
                let previous_event = thread_stack.pop().expect("no start event found");
                assert_eq!(event.label, previous_event.label);
                assert_eq!(previous_event.timestamp_kind, TimestampKind::Start);
            }
            TimestampKind::Instant => {}
        }
    }

    recorded_stacks
}

#[cfg(test)]
mod test {
    use measureme::{Event, TimestampKind};
    use std::collections::HashMap;
    use std::time::{Duration, SystemTime};

    #[test]
    fn basic_test() {
        //                                         <--e1-->
        //                 <--e1-->        <----------e2---------->
        //              T2 1       2       3       4       5       6
        // sample interval |   |   |   |   |   |   |   |   |   |   |
        // stacks count:
        // rustc                       1   2
        // rustc;e1            1   2
        // rustc;e2                            1   2           3   4
        // rustc;e2;e1                                 1   2

        let events = [
            Event {
                event_kind: "Query".into(),
                label: "e1".into(),
                additional_data: &[],
                timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(1),
                timestamp_kind: TimestampKind::Start,
                thread_id: 1,
            },
            Event {
                event_kind: "Query".into(),
                label: "e1".into(),
                additional_data: &[],
                timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(2),
                timestamp_kind: TimestampKind::End,
                thread_id: 1,
            },
            Event {
                event_kind: "Query".into(),
                label: "e2".into(),
                additional_data: &[],
                timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(3),
                timestamp_kind: TimestampKind::Start,
                thread_id: 1,
            },
            Event {
                event_kind: "Query".into(),
                label: "e1".into(),
                additional_data: &[],
                timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(4),
                timestamp_kind: TimestampKind::Start,
                thread_id: 1,
            },
            Event {
                event_kind: "Query".into(),
                label: "e1".into(),
                additional_data: &[],
                timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(5),
                timestamp_kind: TimestampKind::End,
                thread_id: 1,
            },
            Event {
                event_kind: "Query".into(),
                label: "e2".into(),
                additional_data: &[],
                timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(6),
                timestamp_kind: TimestampKind::End,
                thread_id: 1,
            },
        ];

        let recorded_stacks = super::collapse_stacks(events.iter().cloned(), 500);

        let mut expected_stacks = HashMap::<String, usize>::new();
        expected_stacks.insert("rustc;e2;e1".into(), 2);
        expected_stacks.insert("rustc;e2".into(), 4);
        expected_stacks.insert("rustc;e1".into(), 2);
        expected_stacks.insert("rustc".into(), 2);

        assert_eq!(expected_stacks, recorded_stacks);
    }

    #[test]
    fn multi_threaded_test() {
        //                 <--e1-->        <--e1-->
        //              T1 1       2       3       4       5
        //                                 <--e3-->
        //                 <--e1--><----------e2---------->
        //              T2 1       2       3       4       5
        // sample interval |       |       |       |       |
        // stacks count:
        // rustc                           1
        // rustc;e1                2               3
        // rustc;e2                        1               2
        // rustc;e2;e3                             1

        let events = [
            Event {
                event_kind: "Query".into(),
                label: "e1".into(),
                additional_data: &[],
                timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(1),
                timestamp_kind: TimestampKind::Start,
                thread_id: 1,
            },
            Event {
                event_kind: "Query".into(),
                label: "e1".into(),
                additional_data: &[],
                timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(1),
                timestamp_kind: TimestampKind::Start,
                thread_id: 2,
            },
            Event {
                event_kind: "Query".into(),
                label: "e1".into(),
                additional_data: &[],
                timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(2),
                timestamp_kind: TimestampKind::End,
                thread_id: 2,
            },
            Event {
                event_kind: "Query".into(),
                label: "e2".into(),
                additional_data: &[],
                timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(2),
                timestamp_kind: TimestampKind::Start,
                thread_id: 2,
            },
            Event {
                event_kind: "Query".into(),
                label: "e1".into(),
                additional_data: &[],
                timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(2),
                timestamp_kind: TimestampKind::End,
                thread_id: 1,
            },
            Event {
                event_kind: "Query".into(),
                label: "e1".into(),
                additional_data: &[],
                timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(3),
                timestamp_kind: TimestampKind::Start,
                thread_id: 1,
            },
            Event {
                event_kind: "Query".into(),
                label: "e1".into(),
                additional_data: &[],
                timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(4),
                timestamp_kind: TimestampKind::End,
                thread_id: 1,
            },
            Event {
                event_kind: "Query".into(),
                label: "e3".into(),
                additional_data: &[],
                timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(3),
                timestamp_kind: TimestampKind::Start,
                thread_id: 2,
            },
            Event {
                event_kind: "Query".into(),
                label: "e3".into(),
                additional_data: &[],
                timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(4),
                timestamp_kind: TimestampKind::End,
                thread_id: 2,
            },
            Event {
                event_kind: "Query".into(),
                label: "e2".into(),
                additional_data: &[],
                timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(5),
                timestamp_kind: TimestampKind::End,
                thread_id: 2,
            },
        ];

        let recorded_stacks = super::collapse_stacks(events.iter().cloned(), 1000);

        let mut expected_stacks = HashMap::<String, usize>::new();
        expected_stacks.insert("rustc;e2;e3".into(), 1);
        expected_stacks.insert("rustc;e2".into(), 2);
        expected_stacks.insert("rustc;e1".into(), 3);
        expected_stacks.insert("rustc".into(), 1);

        assert_eq!(expected_stacks, recorded_stacks);
    }
}
