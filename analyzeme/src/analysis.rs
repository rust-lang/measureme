use crate::{Event, EventPayload, ProfilingData, Timestamp};
use measureme::rustc::*;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::time::Duration;
use std::time::SystemTime;

impl ProfilingData {
    /// Collects accumulated summary data for the given ProfilingData.
    ///
    /// The main result we are interested in is the query "self-time". This is the
    /// time spent computing the result of a query `q` minus the time spent in
    /// any other queries that `q` might have called. This "self-time" can be
    /// computed by looking at invocation stacks as follows:
    ///
    /// When we encounter a query provider event, we first add its entire duration
    /// to the self-time counter of the query. Then, when we encounter a direct
    /// child of that query provider event, we subtract the duration of the child
    /// from the self-time counter of the query. Thus, after we've encountered all
    /// direct children we'll end up with the self-time.
    ///
    /// For example, take the following query invocation trace:
    /// ```ignore
    ///                                      <== q4 ==>
    ///           <== q2 ==>           <====== q3 ======>
    ///     <===================== q1 =====================>
    /// -------------------------------------------------------> time
    /// ```
    ///
    /// Query `q1` calls `q2` and later `q3`, which in turn calls `q4`. In order
    /// to get the self-time of `q1`, we take it's entire duration and subtract the
    /// durations of `q2` and `q3`. We do not subtract the duration of `q4` because
    /// that is already accounted for by the duration of `q3`.
    ///
    /// The function below uses an algorithm that computes the self-times of all
    /// queries in a single pass over the profiling data. As the algorithm walks the
    /// data, it needs to keep track of invocation stacks. Because interval events
    /// occur in the stream at their "end" time, a parent event comes after all
    /// child events. For this reason we have to walk the events in *reverse order*.
    /// This way we always encounter the parent before its children, which makes it
    /// simple to keep an up-to-date stack of invocations.
    ///
    /// The algorithm goes as follows:
    ///
    /// ```ignore
    /// for event in profiling_data.reversed()
    ///    // Keep the stack up-to-date by popping all events that
    ///    // don't contain the current event. After this loop, the
    ///    // parent of the current event will be the top of the stack.
    ///    while !stack.top.is_ancestor_of(event)
    ///        stack.pop()
    ///
    ///    // Update the parents self-time if needed
    ///    let parent = stack.top()
    ///    if parent.is_some()
    ///        self_time_for(parent) -= event.duration
    ///
    ///    // Update the self-time for the current-event
    ///    self_time_for(event) += event.duration
    ///
    ///    // Push the current event onto the stack
    ///    stack.push(event)
    /// ```
    ///
    /// Here is an example of what updating the stack looks like:
    ///
    /// ```ignore
    ///      <--e2-->   <--e3-->
    ///  <-----------e1----------->
    /// ```
    ///
    /// In the event stream this shows up as something like:
    ///
    /// ```ignore
    /// [
    ///     { label=e2, start= 5, end=10 },
    ///     { label=e3, start=15, end=20 },
    ///     { label=e1, start= 0, end=25 },
    /// ]
    /// ```
    ///
    /// because events are emitted in the order of their end timestamps. So, as we
    /// walk backwards, we
    ///
    /// 1. encounter `e1`, push it onto the stack, then
    /// 2. encounter `e3`, the stack contains `e1`, but that is fine since the
    ///    time-interval of `e1` includes the time interval of `e3`. `e3` goes onto
    ///    the stack and then we
    /// 3. encounter `e2`. The stack is `[e1, e3]`, but now `e3` needs to be popped
    ///    because we are past its range, so we pop `e3` and push `e2`.
    ///
    /// Why is popping done in a `while` loop? consider the following
    ///
    /// ```ignore
    ///                  <-e4->
    ///      <--e2-->   <--e3-->
    ///  <-----------e1----------->
    /// ```
    ///
    /// This looks as follows in the stream:
    ///
    /// ```ignore
    /// [
    ///     { label=e2, start= 5, end=10 },
    ///     { label=e4, start=17, end=19 },
    ///     { label=e3, start=15, end=20 },
    ///     { label=e1, start= 0, end=25 },
    /// ]
    /// ```
    ///
    /// In this case when we encounter `e2`, the stack is `[e1, e3, e4]`, and both
    /// `e4` and `e3` need to be popped in the same step.
    pub fn perform_analysis(self) -> AnalysisResults {
        struct PerThreadState<'a> {
            stack: Vec<Event<'a>>,
            start: SystemTime,
            end: SystemTime,
        }

        let mut query_data = FxHashMap::<String, QueryData>::default();
        let mut artifact_sizes = BTreeMap::<Cow<'_, str>, ArtifactSize>::default();
        let mut threads = FxHashMap::<_, PerThreadState<'_>>::default();

        let mut record_event_data = |label: &Cow<'_, str>, f: &dyn Fn(&mut QueryData)| {
            if let Some(data) = query_data.get_mut(&label[..]) {
                f(data);
            } else {
                let mut data = QueryData::new(label.clone().into_owned());
                f(&mut data);
                query_data.insert(label.clone().into_owned(), data);
            }
        };

        for current_event in self.iter_full().rev() {
            match current_event.payload {
                EventPayload::Timestamp(Timestamp::Instant(_)) => {
                    if &current_event.event_kind[..] == QUERY_CACHE_HIT_EVENT_KIND {
                        record_event_data(&current_event.label, &|data| {
                            data.number_of_cache_hits += 1;
                            data.invocation_count += 1;
                        });
                    }
                }
                EventPayload::Timestamp(Timestamp::Interval { start, end }) => {
                    // This is an interval event
                    let thread =
                        threads
                            .entry(current_event.thread_id)
                            .or_insert_with(|| PerThreadState {
                                stack: Vec::new(),
                                start,
                                end,
                            });

                    // Pop all events from the stack that are not parents of the
                    // current event.
                    while let Some(current_top) = thread.stack.last().cloned() {
                        if current_top.contains(&current_event) {
                            break;
                        }

                        thread.stack.pop();
                    }

                    let current_event_duration = current_event.duration().unwrap();

                    // If there is something on the stack, subtract the current
                    // interval from it.
                    if let Some(current_top) = thread.stack.last() {
                        record_event_data(
                            &current_top.label,
                            &|data| match &current_top.event_kind[..] {
                                QUERY_EVENT_KIND | GENERIC_ACTIVITY_EVENT_KIND => {
                                    data.self_time -= current_event_duration;
                                }
                                INCREMENTAL_RESULT_HASHING_EVENT_KIND => {
                                    // We are within hashing something. If we now encounter something
                                    // within that event (like the nested "intern-the-dep-node" event)
                                    // then we don't want to attribute that to the hashing time.
                                    data.self_time -= current_event_duration;
                                    data.incremental_hashing_time -= current_event_duration;
                                }
                                INCREMENTAL_LOAD_RESULT_EVENT_KIND => {
                                    data.self_time -= current_event_duration;
                                    data.incremental_load_time -= current_event_duration;
                                }
                                _ => {
                                    // Data sources other than rustc will use their own event kinds so
                                    // just treat this like a GENERIC_ACTIVITY except that we don't
                                    // track cache misses since those may not apply to all data sources.
                                    data.self_time -= current_event_duration;
                                }
                            },
                        );
                    }

                    // Update counters for the current event
                    match &current_event.event_kind[..] {
                        QUERY_EVENT_KIND | GENERIC_ACTIVITY_EVENT_KIND => {
                            record_event_data(&current_event.label, &|data| {
                                data.self_time += current_event_duration;
                                data.time += current_event_duration;
                                data.number_of_cache_misses += 1;
                                data.invocation_count += 1;
                            });
                        }

                        QUERY_BLOCKED_EVENT_KIND => {
                            record_event_data(&current_event.label, &|data| {
                                data.self_time += current_event_duration;
                                data.time += current_event_duration;
                                data.blocked_time += current_event_duration;
                                data.invocation_count += 1;
                            });
                        }

                        INCREMENTAL_LOAD_RESULT_EVENT_KIND => {
                            record_event_data(&current_event.label, &|data| {
                                data.self_time += current_event_duration;
                                data.time += current_event_duration;
                                data.incremental_load_time += current_event_duration;
                            });
                        }

                        INCREMENTAL_RESULT_HASHING_EVENT_KIND => {
                            record_event_data(&current_event.label, &|data| {
                                // Don't add to data.time since this event happens
                                // within the query itself which is already contributing
                                // to data.time
                                data.self_time += current_event_duration;
                                data.incremental_hashing_time += current_event_duration;
                            });
                        }

                        _ => {
                            // Data sources other than rustc will use their own event kinds so just
                            // treat this like a GENERIC_ACTIVITY except that we don't track cache
                            // misses since those may not apply to all data sources.
                            record_event_data(&current_event.label, &|data| {
                                data.self_time += current_event_duration;
                                data.time += current_event_duration;
                                data.invocation_count += 1;
                            });
                        }
                    };

                    // Update the start and end times for thread
                    thread.start = std::cmp::min(thread.start, start);
                    thread.end = std::cmp::max(thread.end, end);

                    // Bring the stack up-to-date
                    thread.stack.push(current_event)
                }
                EventPayload::Integer(value) => {
                    if current_event.event_kind == ARTIFACT_SIZE_EVENT_KIND {
                        // Dedup artifact size events according to their label
                        artifact_sizes
                            .entry(current_event.label.clone())
                            .or_insert_with(|| ArtifactSize::new(current_event.label.into_owned()))
                            .add_value(value);
                    }
                }
            }
        }

        let total_time = threads
            .values()
            .map(|t| t.end.duration_since(t.start).unwrap())
            .sum();

        AnalysisResults {
            query_data: query_data.drain().map(|(_, value)| value).collect(),
            artifact_sizes: artifact_sizes.into_values().collect(),
            total_time,
        }
    }
}

/// A collection data for an entire rustc invocation
#[derive(Serialize, Deserialize, Clone)]
pub struct AnalysisResults {
    pub query_data: Vec<QueryData>,
    pub artifact_sizes: Vec<ArtifactSize>,
    pub total_time: Duration,
}

// These are currently only needed for testing
#[cfg(test)]
impl AnalysisResults {
    pub fn query_data_by_label(&self, label: &str) -> &QueryData {
        self.query_data.iter().find(|qd| qd.label == label).unwrap()
    }

    pub fn artifact_size_by_label(&self, label: &str) -> &ArtifactSize {
        self.artifact_sizes
            .iter()
            .find(|qd| qd.label == label)
            .unwrap()
    }
}

/// Data related to profiling a specific rustc query
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct QueryData {
    pub label: String,
    pub time: Duration,
    pub self_time: Duration,
    pub number_of_cache_misses: usize,
    pub number_of_cache_hits: usize,
    pub invocation_count: usize,
    pub blocked_time: Duration,
    pub incremental_load_time: Duration,
    pub incremental_hashing_time: Duration,
}

impl QueryData {
    pub fn new(label: String) -> QueryData {
        QueryData {
            label,
            ..Self::default()
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ArtifactSize {
    pub label: String,
    pub value: u64,
}

impl ArtifactSize {
    pub fn new(label: String) -> Self {
        Self { label, value: 0 }
    }

    pub(crate) fn add_value(&mut self, value: u64) {
        self.value += value;
    }
}

#[rustfmt::skip]
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use crate::ProfilingDataBuilder;

    #[test]
    fn total_time_and_nesting() {
        let mut b = ProfilingDataBuilder::new();

        b.interval(QUERY_EVENT_KIND, "q1", 0, 100, 200, |b| {
            b.interval(QUERY_EVENT_KIND, "q2", 0, 110, 190, |b| {
                b.interval(QUERY_EVENT_KIND, "q3", 0, 120, 180, |_| {});
            });
        });

        let results = b.into_profiling_data().perform_analysis();

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

        let results = b.into_profiling_data().perform_analysis();

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

        let results = b.into_profiling_data().perform_analysis();

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

        let results = b.into_profiling_data().perform_analysis();

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

        let results = b.into_profiling_data().perform_analysis();

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

        let results = b.into_profiling_data().perform_analysis();

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

        let results = b.into_profiling_data().perform_analysis();

        assert_eq!(results.total_time, Duration::from_nanos(100));

        assert_eq!(results.query_data_by_label("e1").self_time, Duration::from_nanos(100));
        assert_eq!(results.query_data_by_label("e1").invocation_count, 3);
        assert_eq!(results.query_data_by_label("e1").time, Duration::from_nanos(180));
    }

    #[test]
    fn query_blocked() {
        // T1: <---------------q1--------------->
        // T2:         <------q1 (blocked)------>
        // T3:             <----q1 (blocked)---->
        //     0       30  40                   100

        let mut b = ProfilingDataBuilder::new();

        b.interval(QUERY_EVENT_KIND, "q1", 1, 0, 100, |_| {});
        b.interval(QUERY_BLOCKED_EVENT_KIND, "q1", 2, 30, 100, |_| {});
        b.interval(QUERY_BLOCKED_EVENT_KIND, "q1", 3, 40, 100, |_| {});

        let results = b.into_profiling_data().perform_analysis();

        assert_eq!(results.total_time, Duration::from_nanos(230));

        assert_eq!(results.query_data_by_label("q1").self_time, Duration::from_nanos(230));
        assert_eq!(results.query_data_by_label("q1").blocked_time, Duration::from_nanos(130));
        assert_eq!(results.query_data_by_label("q1").time, Duration::from_nanos(230));
    }

    #[test]
    fn query_incr_loading_time() {
        // T1: <---------------q1 (loading)----->
        // T2:         <------q1 (loading)------>
        // T3:             <----q1 (loading)---->
        //     0       30  40                   100

        let mut b = ProfilingDataBuilder::new();

        b.interval(INCREMENTAL_LOAD_RESULT_EVENT_KIND, "q1", 1, 0, 100, |_| {});
        b.interval(INCREMENTAL_LOAD_RESULT_EVENT_KIND, "q1", 2, 30, 100, |_| {});
        b.interval(INCREMENTAL_LOAD_RESULT_EVENT_KIND, "q1", 3, 40, 100, |_| {});

        let results = b.into_profiling_data().perform_analysis();

        assert_eq!(results.total_time, Duration::from_nanos(230));

        assert_eq!(results.query_data_by_label("q1").self_time, Duration::from_nanos(230));
        assert_eq!(results.query_data_by_label("q1").incremental_load_time, Duration::from_nanos(230));
        assert_eq!(results.query_data_by_label("q1").time, Duration::from_nanos(230));
    }

    #[test]
    fn artifact_sizes() {
        let mut b = ProfilingDataBuilder::new();

        b.integer(ARTIFACT_SIZE_EVENT_KIND, "artifact1", 1, 100);
        b.integer(ARTIFACT_SIZE_EVENT_KIND, "artifact1", 2, 50);
        b.integer(ARTIFACT_SIZE_EVENT_KIND, "artifact2", 1, 50);
        b.integer("OTHER_EVENT", "other_id", 1, 50);

        let results = b.into_profiling_data().perform_analysis();

        assert_eq!(results.artifact_sizes.len(), 2);
        assert_eq!(results.artifact_size_by_label("artifact1").value, 150);
        assert_eq!(results.artifact_size_by_label("artifact1").label, "artifact1");
        assert_eq!(results.artifact_size_by_label("artifact2").value, 50);
        assert_eq!(results.artifact_size_by_label("artifact2").label, "artifact2");
    }
}
