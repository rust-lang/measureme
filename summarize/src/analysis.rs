use crate::query_data::{QueryData, Results};
use measureme::rustc::*;
use measureme::{Event, ProfilingData, TimestampKind};
use std::borrow::Cow;
use std::collections::HashMap;
use std::time::Duration;

pub fn perform_analysis(data: ProfilingData) -> Results {
    let mut query_data = HashMap::<String, QueryData>::new();
    let mut threads = HashMap::<_, Vec<Event>>::new();
    let mut total_time = Duration::from_nanos(0);

    let mut record_event_data = |label: &Cow<'_, str>, f: &dyn Fn(&mut QueryData)| {
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

                if &event.event_kind[..] == QUERY_EVENT_KIND
                    || &event.event_kind[..] == GENERIC_ACTIVITY_EVENT_KIND
                {
                    if let Some(prev_event) = thread_stack.last() {
                        //count the time run so far for this event
                        let duration = event
                            .timestamp
                            .duration_since(prev_event.timestamp)
                            .unwrap_or(Duration::from_nanos(0));

                        record_event_data(&prev_event.label, &|data| {
                            data.self_time += duration;
                        });

                        //record the total time
                        total_time += duration;
                    }

                    thread_stack.push(event);
                } else if &event.event_kind[..] == QUERY_BLOCKED_EVENT_KIND
                    || &event.event_kind[..] == INCREMENTAL_LOAD_RESULT_EVENT_KIND
                {
                    thread_stack.push(event);
                }
            }
            TimestampKind::Instant => {
                if &event.event_kind[..] == QUERY_CACHE_HIT_EVENT_KIND {
                    record_event_data(&event.label, &|data| {
                        data.number_of_cache_hits += 1;
                        data.invocation_count += 1;
                    });
                }
            }
            TimestampKind::End => {
                let thread_stack = threads.get_mut(&event.thread_id).unwrap();
                let start_event = thread_stack.pop().unwrap();

                assert_eq!(start_event.label, event.label);
                assert_eq!(start_event.event_kind, event.event_kind);
                assert_eq!(start_event.timestamp_kind, TimestampKind::Start);

                //track the time for this event
                let duration = event
                    .timestamp
                    .duration_since(start_event.timestamp)
                    .unwrap_or(Duration::from_nanos(0));

                if &event.event_kind[..] == QUERY_EVENT_KIND
                    || &event.event_kind[..] == GENERIC_ACTIVITY_EVENT_KIND
                {
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

#[cfg(test)]
mod tests {
    use super::*;
    use measureme::ProfilingDataBuilder;

    #[test]
    fn total_time_and_nesting() {
        let mut b = ProfilingDataBuilder::new();

        b.interval(QUERY_EVENT_KIND, "q1", 0, 100, 200, |b| {
            b.interval(QUERY_EVENT_KIND, "q2", 0, 110, 190, |b| {
                b.interval(QUERY_EVENT_KIND, "q3", 0, 120, 180, |_| {});
            });
        });

        let results = perform_analysis(b.into_profiling_data());

        assert_eq!(results.total_time, Duration::from_nanos(100));

        // 10ns in the beginning and 10ns in the end
        assert_eq!(results.query_data_by_label("q1").self_time, Duration::from_nanos(20));
        // 10ns in the beginning and 10ns in the end, again
        assert_eq!(results.query_data_by_label("q2").self_time, Duration::from_nanos(20));
        // 60ns of uninterupted self-time
        assert_eq!(results.query_data_by_label("q3").self_time, Duration::from_nanos(60));

        assert_eq!(results.query_data_by_label("q1").invocation_count, 1);
        assert_eq!(results.query_data_by_label("q2").invocation_count, 1);
        assert_eq!(results.query_data_by_label("q3").invocation_count, 1);
    }

    #[test]
    fn events_with_same_starting_time() {
        //                      <--e4-->
        //                      <---e3--->
        //  <--------e1--------><--------e2-------->
        //  100                 200                300

        let mut b = ProfilingDataBuilder::new();

        b.interval(QUERY_EVENT_KIND, "e1", 0, 100, 200, |_| {});
        b.interval(QUERY_EVENT_KIND, "e2", 0, 200, 300, |b| {
            b.interval(QUERY_EVENT_KIND, "e3", 0, 200, 250, |b| {
                b.interval(QUERY_EVENT_KIND, "e4", 0, 200, 220, |_| {});
            });
        });

        let results = perform_analysis(b.into_profiling_data());

        assert_eq!(results.total_time, Duration::from_nanos(200));

        assert_eq!(results.query_data_by_label("e1").self_time, Duration::from_nanos(100));
        assert_eq!(results.query_data_by_label("e2").self_time, Duration::from_nanos(50));
        assert_eq!(results.query_data_by_label("e3").self_time, Duration::from_nanos(30));
        assert_eq!(results.query_data_by_label("e4").self_time, Duration::from_nanos(20));

        assert_eq!(results.query_data_by_label("e1").invocation_count, 1);
        assert_eq!(results.query_data_by_label("e2").invocation_count, 1);
        assert_eq!(results.query_data_by_label("e3").invocation_count, 1);
        assert_eq!(results.query_data_by_label("e4").invocation_count, 1);
    }

    #[test]
    fn events_with_same_end_time() {
        //                                  <--e4-->
        //                                <---e3--->
        //  <--------e1--------><--------e2-------->
        //  100                 200                300

        let mut b = ProfilingDataBuilder::new();

        b.interval(QUERY_EVENT_KIND, "e1", 0, 100, 200, |_| {});
        b.interval(QUERY_EVENT_KIND, "e2", 0, 200, 300, |b| {
            b.interval(QUERY_EVENT_KIND, "e3", 0, 250, 300, |b| {
                b.interval(QUERY_EVENT_KIND, "e4", 0, 280, 300, |_| {});
            });
        });

        let results = perform_analysis(b.into_profiling_data());

        assert_eq!(results.total_time, Duration::from_nanos(200));

        assert_eq!(results.query_data_by_label("e1").self_time, Duration::from_nanos(100));
        assert_eq!(results.query_data_by_label("e2").self_time, Duration::from_nanos(50));
        assert_eq!(results.query_data_by_label("e3").self_time, Duration::from_nanos(30));
        assert_eq!(results.query_data_by_label("e4").self_time, Duration::from_nanos(20));

        assert_eq!(results.query_data_by_label("e1").invocation_count, 1);
        assert_eq!(results.query_data_by_label("e2").invocation_count, 1);
        assert_eq!(results.query_data_by_label("e3").invocation_count, 1);
        assert_eq!(results.query_data_by_label("e4").invocation_count, 1);
    }

    #[test]
    fn same_event_multiple_times() {
        //        <--e3-->            <--e3-->
        //       <---e2--->          <---e2--->
        //  <--------e1--------><--------e1-------->
        //  100                 200                300

        let mut b = ProfilingDataBuilder::new();

        b.interval(QUERY_EVENT_KIND, "e1", 0, 100, 200, |b| {
            b.interval(QUERY_EVENT_KIND, "e2", 0, 120, 180, |b| {
                b.interval(QUERY_EVENT_KIND, "e3", 0, 140, 160, |_| {});
            });
        });

        b.interval(QUERY_EVENT_KIND, "e1", 0, 200, 300, |b| {
            b.interval(QUERY_EVENT_KIND, "e2", 0, 220, 280, |b| {
                b.interval(QUERY_EVENT_KIND, "e3", 0, 240, 260, |_| {});
            });
        });

        let results = perform_analysis(b.into_profiling_data());

        assert_eq!(results.total_time, Duration::from_nanos(200));

        assert_eq!(results.query_data_by_label("e1").self_time, Duration::from_nanos(80));
        assert_eq!(results.query_data_by_label("e2").self_time, Duration::from_nanos(80));
        assert_eq!(results.query_data_by_label("e3").self_time, Duration::from_nanos(40));

        assert_eq!(results.query_data_by_label("e1").invocation_count, 2);
        assert_eq!(results.query_data_by_label("e2").invocation_count, 2);
        assert_eq!(results.query_data_by_label("e3").invocation_count, 2);
    }

    #[test]
    fn multiple_threads() {
        //          <--e3-->            <--e3-->
        //         <---e2--->          <---e2--->
        //    <--------e1--------><--------e1-------->
        // T0 100                 200                300
        //
        //           <--e3-->            <--e3-->
        //          <---e2--->          <---e2--->
        //     <--------e1--------><--------e1-------->
        // T1 100                 200                300

        let mut b = ProfilingDataBuilder::new();

        // Thread 0
        b.interval(QUERY_EVENT_KIND, "e1", 0, 100, 200, |b| {
            b.interval(QUERY_EVENT_KIND, "e2", 0, 120, 180, |b| {
                b.interval(QUERY_EVENT_KIND, "e3", 0, 140, 160, |_| {});
            });
        });

        // Thread 1 -- the same as thread 0 with a slight time offset
        b.interval(QUERY_EVENT_KIND, "e1", 1, 110, 210, |b| {
            b.interval(QUERY_EVENT_KIND, "e2", 1, 130, 190, |b| {
                b.interval(QUERY_EVENT_KIND, "e3", 1, 150, 170, |_| {});
            });
        });

        // Thread 0 -- continued
        b.interval(QUERY_EVENT_KIND, "e1", 0, 200, 300, |b| {
            b.interval(QUERY_EVENT_KIND, "e2", 0, 220, 280, |b| {
                b.interval(QUERY_EVENT_KIND, "e3", 0, 240, 260, |_| {});
            });
        });

        // Thread 1 -- continued
        b.interval(QUERY_EVENT_KIND, "e1", 1, 210, 310, |b| {
            b.interval(QUERY_EVENT_KIND, "e2", 1, 230, 290, |b| {
                b.interval(QUERY_EVENT_KIND, "e3", 1, 250, 270, |_| {});
            });
        });

        let results = perform_analysis(b.into_profiling_data());

        assert_eq!(results.total_time, Duration::from_nanos(400));

        assert_eq!(results.query_data_by_label("e1").self_time, Duration::from_nanos(160));
        assert_eq!(results.query_data_by_label("e2").self_time, Duration::from_nanos(160));
        assert_eq!(results.query_data_by_label("e3").self_time, Duration::from_nanos(80));

        assert_eq!(results.query_data_by_label("e1").invocation_count, 4);
        assert_eq!(results.query_data_by_label("e2").invocation_count, 4);
        assert_eq!(results.query_data_by_label("e3").invocation_count, 4);
    }

    #[test]
    fn instant_events() {
        //          xyxy
        //      y <--e3--> x
        //   x <-----e2-----> x
        //  <--------e1-------->
        //  100                200

        let mut b = ProfilingDataBuilder::new();

        b.interval(QUERY_EVENT_KIND, "e1", 0, 200, 300, |b| {
            b.instant(QUERY_CACHE_HIT_EVENT_KIND, "x", 0, 210);

            b.interval(QUERY_EVENT_KIND, "e2", 0, 220, 280, |b| {

                b.instant(QUERY_CACHE_HIT_EVENT_KIND, "y", 0, 230);

                b.interval(QUERY_EVENT_KIND, "e3", 0, 240, 260, |b| {
                    b.instant(QUERY_CACHE_HIT_EVENT_KIND, "x", 0, 241);
                    b.instant(QUERY_CACHE_HIT_EVENT_KIND, "y", 0, 242);
                    b.instant(QUERY_CACHE_HIT_EVENT_KIND, "x", 0, 243);
                    b.instant(QUERY_CACHE_HIT_EVENT_KIND, "y", 0, 244);
                });

                b.instant(QUERY_CACHE_HIT_EVENT_KIND, "x", 0, 270);
            });

            b.instant(QUERY_CACHE_HIT_EVENT_KIND, "x", 0, 290);
        });

        let results = perform_analysis(b.into_profiling_data());

        assert_eq!(results.total_time, Duration::from_nanos(100));

        assert_eq!(results.query_data_by_label("e1").self_time, Duration::from_nanos(40));
        assert_eq!(results.query_data_by_label("e2").self_time, Duration::from_nanos(40));
        assert_eq!(results.query_data_by_label("e3").self_time, Duration::from_nanos(20));

        assert_eq!(results.query_data_by_label("e1").invocation_count, 1);
        assert_eq!(results.query_data_by_label("e2").invocation_count, 1);
        assert_eq!(results.query_data_by_label("e3").invocation_count, 1);

        assert_eq!(results.query_data_by_label("x").number_of_cache_hits, 5);
        assert_eq!(results.query_data_by_label("y").number_of_cache_hits, 3);
    }

    #[test]
    fn stack_of_same_events() {
        //        <--e1-->
        //     <-----e1----->
        //  <--------e1-------->
        //  100                200

        let mut b = ProfilingDataBuilder::new();

        b.interval(QUERY_EVENT_KIND, "e1", 0, 200, 300, |b| {
            b.interval(QUERY_EVENT_KIND, "e1", 0, 220, 280, |b| {
                b.interval(QUERY_EVENT_KIND, "e1", 0, 240, 260, |_| {});
            });
        });

        let results = perform_analysis(b.into_profiling_data());

        assert_eq!(results.total_time, Duration::from_nanos(100));

        assert_eq!(results.query_data_by_label("e1").self_time, Duration::from_nanos(100));
        assert_eq!(results.query_data_by_label("e1").invocation_count, 3);
    }
}
