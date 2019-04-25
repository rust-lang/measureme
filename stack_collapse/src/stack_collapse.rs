use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use measureme::{Event, TimestampKind};

pub fn collapse_stacks<'a>(events: impl Iterator<Item = Event<'a>>, first_event_time: SystemTime, interval: u64) -> HashMap<String, usize> {
    let mut recorded_stacks = HashMap::<String, usize>::new();

    let mut next_observation_time = first_event_time;

    let mut thread_stacks: HashMap<u64, Vec<Event>> = HashMap::new();

    for event in events {
        //if this event is after the next_observation_time then we need to record the current stacks
        while event.timestamp > next_observation_time {
            for (_tid, stack) in &thread_stacks {
                let mut stack_string = String::new();
                stack_string.push_str("rustc;");

                for event in stack {
                    stack_string.push_str(&event.label);
                    stack_string.push(';');
                }

                //remove the trailing ';'
                stack_string.remove(stack_string.len() - 1);

                *recorded_stacks.entry(stack_string).or_default() += 1;

                next_observation_time += Duration::from_millis(interval);
            }
        }

        let thread_stack = thread_stacks.entry(event.thread_id).or_default();

        match event.timestamp_kind {
            TimestampKind::Start => {
                thread_stack.push(event);
            },
            TimestampKind::End => {
                let previous_event = thread_stack.pop().expect("no start event found");
                assert_eq!(event.label, previous_event.label);
                assert_eq!(previous_event.timestamp_kind, TimestampKind::Start);
            },
            TimestampKind::Instant => { },
        }
    }

    recorded_stacks
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use std::time::{Duration, SystemTime};
    use measureme::{Event, TimestampKind};

    #[test]
    fn basic_test() {
        let events = [
            Event {
                event_kind: "Query".into(),
                label: "EventA".into(),
                additional_data: &[],
                timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(1),
                timestamp_kind: TimestampKind::Start,
                thread_id: 1,
            },
            Event {
                event_kind: "Query".into(),
                label: "EventA".into(),
                additional_data: &[],
                timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(2),
                timestamp_kind: TimestampKind::End,
                thread_id: 1,
            },
            Event {
                event_kind: "Query".into(),
                label: "EventB".into(),
                additional_data: &[],
                timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(3),
                timestamp_kind: TimestampKind::Start,
                thread_id: 1,
            },
            Event {
                event_kind: "Query".into(),
                label: "EventA".into(),
                additional_data: &[],
                timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(4),
                timestamp_kind: TimestampKind::Start,
                thread_id: 1,
            },
            Event {
                event_kind: "Query".into(),
                label: "EventA".into(),
                additional_data: &[],
                timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(5),
                timestamp_kind: TimestampKind::End,
                thread_id: 1,
            },
            Event {
                event_kind: "Query".into(),
                label: "EventB".into(),
                additional_data: &[],
                timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(6),
                timestamp_kind: TimestampKind::End,
                thread_id: 1,
            },
        ];

        let first_event_time = events[0].timestamp;

        let recorded_stacks = super::collapse_stacks(events.iter().cloned(), first_event_time, 1);

        let mut expected_stacks = HashMap::<String, usize>::new();
        expected_stacks.insert("rustc;EventB;EventA".into(), 1000);
        expected_stacks.insert("rustc;EventB".into(), 2000);
        expected_stacks.insert("rustc;EventA".into(), 1000);
        expected_stacks.insert("rustc".into(), 1000);

        assert_eq!(
            expected_stacks,
            recorded_stacks
        );
    }
}
