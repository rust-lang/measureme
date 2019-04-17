use std::borrow::Cow;
use std::collections::HashMap;
use std::time::Duration;
use measureme::{ProfilingData, TimestampKind, Event};

pub struct QueryData {
    pub label: String,
    pub self_time: Duration,
    pub number_of_cache_misses: usize,
    pub number_of_cache_hits: usize,
    pub blocked_time: Duration,
    pub incremental_load_time: Duration,
}

impl QueryData {
    fn new(label: String) -> QueryData {
        QueryData {
            label,
            self_time: Duration::from_nanos(0),
            number_of_cache_misses: 0,
            number_of_cache_hits: 0,
            blocked_time: Duration::from_nanos(0),
            incremental_load_time: Duration::from_nanos(0),
        }
    }
}

pub struct Results {
    pub query_data: Vec<QueryData>,
    pub total_time: Duration,
}

pub fn perform_analysis(data: ProfilingData) -> Results {
    let mut query_data = HashMap::<String, QueryData>::new();
    let mut threads = HashMap::<_, Vec<Event>>::new();
    let mut total_time = Duration::from_nanos(0);

    let mut record_event_data = |label: &Cow<'_, str>, f: &Fn(&mut QueryData)| {
        if let Some(data) = query_data.get_mut(&label[..]) {
            f(data);
        } else {
            let mut data = QueryData::new(label.clone().into_owned());
            f(&mut data);
            query_data.insert(label.clone().into_owned(), data);
        }
    };

    for event in data.iter() {
        match event.timestamp_kind {
            TimestampKind::Start => {
                let thread_stack = threads.entry(event.thread_id).or_default();

                if &event.event_kind[..] == "Query" || &event.event_kind[..] == "GenericActivity" {
                    if let Some(prev_event) = thread_stack.last() {
                        //count the time run so far for this event
                        let duration =
                            event.timestamp.duration_since(prev_event.timestamp)
                                .unwrap_or(Duration::from_nanos(0));

                        record_event_data(&prev_event.label, &|data| {
                            data.self_time += duration;
                        });

                        //record the total time
                        total_time += duration;
                    }

                    thread_stack.push(event);
                } else if &event.event_kind[..] == "QueryBlocked" ||
                          &event.event_kind[..] == "IncrementalLoadResult" {
                    thread_stack.push(event);
                }
            },
            TimestampKind::Instant => {
                if &event.event_kind[..] == "QueryCacheHit" {
                    record_event_data(&event.label, &|data| {
                        data.number_of_cache_hits += 1;
                    });
                }
            },
            TimestampKind::End => {
                let thread_stack = threads.get_mut(&event.thread_id).unwrap();
                let start_event = thread_stack.pop().unwrap();

                assert_eq!(start_event.event_kind, event.event_kind);
                assert_eq!(start_event.label, event.label);
                assert_eq!(start_event.timestamp_kind, TimestampKind::Start);

                //track the time for this event
                let duration =
                    event.timestamp
                        .duration_since(start_event.timestamp)
                        .unwrap_or(Duration::from_nanos(0));

                if &event.event_kind[..] == "Query" || &event.event_kind[..] == "GenericActivity" {
                    record_event_data(&event.label, &|data| {
                        data.self_time += duration;
                        data.number_of_cache_misses += 1;
                    });

                    //now adjust the previous event's start time so that it "started" right now
                    if let Some(previous_event) = thread_stack.last_mut() {
                        assert_eq!(TimestampKind::Start, previous_event.timestamp_kind);
                        previous_event.timestamp = event.timestamp;
                    }

                    //record the total time
                    total_time += duration;
                } else if &event.event_kind[..] == "QueryBlocked" {
                    record_event_data(&event.label, &|data| {
                        data.blocked_time += duration;
                    });
                } else if &event.event_kind[..] == "IncrementalLoadResult" {
                    record_event_data(&event.label, &|data| {
                        data.incremental_load_time += duration;
                    });
                }
            }
        }
    }

    Results {
        query_data: query_data.drain().map(|(_, value)| value).collect(),
        total_time,
    }
}
