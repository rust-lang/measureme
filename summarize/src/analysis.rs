use std::borrow::Cow;
use std::collections::HashMap;
use std::time::Duration;
use measureme::{ProfilingData, TimestampKind, Event};
use measureme::rustc::*;

use serde::{Serialize};

#[derive(Serialize)]
pub struct QueryData {
    pub label: String,
    pub self_time: Duration,
    pub number_of_cache_misses: usize,
    pub number_of_cache_hits: usize,
    pub invocation_count: usize,
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
            invocation_count: 0,
            blocked_time: Duration::from_nanos(0),
            incremental_load_time: Duration::from_nanos(0),
        }
    }
}

#[derive(Serialize)]
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

    /*
        The basic idea is to iterate over all of the events in the profile data file, with some
        special handling for Start and Stop events.

        When calculating timing data, the core thing we're interested in is self-time.
        In order to calculate that correctly, we need to track when an event is running and when
        it has been interrupted by another event.

        Let's look at a simple example with two events:

        Event 1:
        - Started at 0ms
        - Ended at 10ms

        Event 2:
        - Started at 4ms
        - Ended at 6ms

          0  1  2  3  4  5  6  7  8  9  10
          ================================
        1 |------------------------------|
        2             |-----|

        When processing this, we see the events like this:

        - Start Event 1
        - Start Event 2
        - End Event 2
        - End Event 1

        Now, I'll add some annotation to these events to show what's happening in the code:

        - Start Event 1
            - Since there is no other event is running, there is no additional bookkeeping to do
            - We push Event 1 onto the thread stack.
        - Start Event 2
            - Since there is another event on the stack running, record the time from that event's
              start time to this event's start time. (In this case, that's the time from 0ms - 4ms)
            - We push Event 2 onto the thread stack.
        - End Event 2
            - We pop Event 2's start event from the thread stack and record the time from its start
              time to the current time (In this case, that's 4ms - 6ms)
            - Since there's another event on the stack, we mutate its start time to be the current
              time. This effectively "restarts" that event's timer.
        - End Event 1
            - We pop Event 1's start event from the thread stack and record the time from its start
              time to the current time (In this case, that's 6ms - 10ms because we mutated the start
              time when we processed End Event 2)
            - Since there's no other events on the stack, there is no additional bookkeeping to do

        As a result:
        Event 1's self-time is `(4-0)ms + (10-6)ms = 8ms`

        Event 2's self-time is `(6-2)ms = 2ms`
    */

    for event in data.iter() {
        match event.timestamp_kind {
            TimestampKind::Start => {
                let thread_stack = threads.entry(event.thread_id).or_default();

                if &event.event_kind[..] == QUERY_EVENT_KIND ||
                    &event.event_kind[..] == GENERIC_ACTIVITY_EVENT_KIND {
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
                } else if &event.event_kind[..] == QUERY_BLOCKED_EVENT_KIND ||
                          &event.event_kind[..] == INCREMENTAL_LOAD_RESULT_EVENT_KIND {
                    thread_stack.push(event);
                }
            },
            TimestampKind::Instant => {
                if &event.event_kind[..] == QUERY_CACHE_HIT_EVENT_KIND {
                    record_event_data(&event.label, &|data| {
                        data.number_of_cache_hits += 1;
                        data.invocation_count += 1;
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

                if &event.event_kind[..] == QUERY_EVENT_KIND ||
                    &event.event_kind[..] == GENERIC_ACTIVITY_EVENT_KIND {
                    record_event_data(&event.label, &|data| {
                        data.self_time += duration;
                        data.number_of_cache_misses += 1;
                        data.invocation_count += 1;
                    });

                    //this is the critical bit to correctly calculating self-time:
                    //adjust the previous event's start time so that it "started" right now
                    if let Some(previous_event) = thread_stack.last_mut() {
                        assert_eq!(TimestampKind::Start, previous_event.timestamp_kind);
                        previous_event.timestamp = event.timestamp;
                    }

                    //record the total time
                    total_time += duration;
                } else if &event.event_kind[..] == QUERY_BLOCKED_EVENT_KIND {
                    record_event_data(&event.label, &|data| {
                        data.blocked_time += duration;
                    });
                } else if &event.event_kind[..] == INCREMENTAL_LOAD_RESULT_EVENT_KIND {
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
