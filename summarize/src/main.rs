use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use measureme::{ProfilingData, TimestampKind, Event};

use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Opt {
    file_prefix: PathBuf,
}

fn main() {
    let opt = Opt::from_args();

    let data = ProfilingData::new(&opt.file_prefix);

    let mut cumulative_time = HashMap::<_, Duration>::new();
    let mut threads = HashMap::<_, Vec<Event>>::new();

    for event in data.iter() {
        match event.timestamp_kind {
            TimestampKind::Start => {
                let thread_stack = threads.entry(event.thread_id).or_default();

                if let Some(prev_event) = thread_stack.last() {
                    //count the time run so far for this event
                    let duration =
                        event.timestamp.duration_since(prev_event.timestamp)
                            .unwrap_or(Duration::from_nanos(0));
                    *cumulative_time.entry(prev_event.label.clone()).or_default() += duration;
                }

                thread_stack.push(event);
            },
            TimestampKind::Instant => { },
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
                *cumulative_time.entry(start_event.label.clone()).or_default() += duration;

                //now adjust the previous event's start time so that it "started" right now
                if let Some(previous_event) = thread_stack.last_mut() {
                    assert_eq!(TimestampKind::Start, previous_event.timestamp_kind);
                    previous_event.timestamp = event.timestamp;
                }
            }
        }
    }

    let mut total_time = Duration::from_nanos(0);

    let mut times: Vec<_> = cumulative_time.iter().collect();
    times.sort_by_key(|(_, v)| *v);
    times.reverse();
    for (k, v) in times {
        total_time += *v;
        println!("{}- {:?}", k, v);
    }

    println!("Total cpu time: {:?}", total_time);
}
