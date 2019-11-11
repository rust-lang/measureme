use rustc_hash::FxHashMap;
use std::fs;
use std::io::BufWriter;
use std::path::PathBuf;
use std::time::Duration;
use std::time::SystemTime;

use analyzeme::{ProfilingData, Timestamp};

use serde::{Serialize, Serializer};
use std::cmp;
use structopt::StructOpt;

fn as_micros<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
    let v = (d.as_secs() * 1_000_000) + (d.subsec_nanos() as u64 / 1_000);
    s.serialize_u64(v)
}

#[derive(Clone, Copy, Eq, PartialEq, Serialize)]
enum EventType {
    #[serde(rename = "X")]
    Complete,
}

#[derive(Serialize)]
struct Event {
    name: String,
    #[serde(rename = "cat")]
    category: String,
    #[serde(rename = "ph")]
    event_type: EventType,
    #[serde(rename = "ts", serialize_with = "as_micros")]
    #[serde()]
    timestamp: Duration,
    #[serde(rename = "dur", serialize_with = "as_micros")]
    duration: Duration,
    #[serde(rename = "pid")]
    process_id: u32,
    #[serde(rename = "tid")]
    thread_id: u64,
    args: Option<FxHashMap<String, String>>,
}

#[derive(StructOpt, Debug)]
struct Opt {
    file_prefix: PathBuf,
    /// collapse threads without overlapping events
    #[structopt(long = "collapse-threads")]
    collapse_threads: bool,
    /// filter out events with shorter duration (in microseconds)
    #[structopt(long = "minimum-duration")]
    minimum_duration: Option<u128>,
}

// generate mapping from thread_id to collapsed thread_id or an empty map
fn generate_thread_to_collapsed_thread_mapping(
    opt: &Opt,
    data: &ProfilingData,
) -> FxHashMap<u64, u64> {
    let mut thread_to_collapsed_thread: FxHashMap<u64, u64> = FxHashMap::default();

    if opt.collapse_threads {
        // collect start and end times for all threads
        let mut thread_start_and_end: FxHashMap<u64, (SystemTime, SystemTime)> =
            FxHashMap::default();
        for event in data.iter() {
            thread_start_and_end
                .entry(event.thread_id)
                .and_modify(|(thread_start, thread_end)| {
                    let (event_min, event_max) = timestamp_to_min_max(event.timestamp);
                    *thread_start = cmp::min(*thread_start, event_min);
                    *thread_end = cmp::max(*thread_end, event_max);
                })
                .or_insert_with(|| timestamp_to_min_max(event.timestamp));
        }
        // collect the the threads in order of the end time
        let mut end_and_thread = thread_start_and_end
            .iter()
            .map(|(&thread_id, &(_start, end))| (end, thread_id))
            .collect::<Vec<_>>();

        end_and_thread.sort_unstable_by_key(|&(end, _thread_id)| end);
        let mut next_end_iter = end_and_thread.iter().peekable();

        // collect the the threads in order of the start time
        let mut start_and_thread = thread_start_and_end
            .iter()
            .map(|(&thread_id, &(start, _end))| (start, thread_id))
            .collect::<Vec<_>>();

        start_and_thread.sort_unstable_by_key(|&(start, _thread_id)| start);

        let mut current_thread_id = 0; // use new thread_ids to avoid strange gaps in the numbers
        for &(start, thread_id) in start_and_thread.iter() {
            // safe to unwrap due to end_and_thread and start_and_thread have the same length
            let (next_end, next_thread_id) = next_end_iter.peek().unwrap();
            if start > *next_end {
                next_end_iter.next();
                // need to lookup the thread_id due to new and collapsed threads
                let mapped_thread_id = *thread_to_collapsed_thread
                    .get(&next_thread_id)
                    .unwrap_or(&next_thread_id);

                thread_to_collapsed_thread.insert(thread_id, mapped_thread_id);
            } else {
                thread_to_collapsed_thread.insert(thread_id, current_thread_id);
                current_thread_id += 1;
            }
        }
    }
    thread_to_collapsed_thread
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();

    let data = ProfilingData::new(&opt.file_prefix)?;

    let chrome_file = BufWriter::new(fs::File::create("chrome_profiler.json")?);

    //find the earlier timestamp (it should be the first event)
    //subtract one tick so that the start of the event shows in Chrome
    let first_event_timestamp = make_start_timestamp(&data);

    let mut serializer = serde_json::Serializer::new(chrome_file);
    let thread_to_collapsed_thread = generate_thread_to_collapsed_thread_mapping(&opt, &data);
    let mut event_iterator = data.iter();

    //create an iterator so we can avoid allocating a Vec with every Event for serialization
    let json_event_iterator = std::iter::from_fn(|| {
        while let Some(event) = event_iterator.next() {
            // Chrome does not seem to like how many QueryCacheHit events we generate
            // only handle startStop events for now
            if let Timestamp::Interval { start, end } = event.timestamp {
                let duration = end.duration_since(start).unwrap();
                if let Some(minimum_duration) = opt.minimum_duration {
                    if duration.as_micros() < minimum_duration {
                        continue;
                    }
                }
                return Some(Event {
                    name: event.label.clone().into_owned(),
                    category: event.event_kind.clone().into_owned(),
                    event_type: EventType::Complete,
                    timestamp: start.duration_since(first_event_timestamp).unwrap(),
                    duration,
                    process_id: 0,
                    thread_id: *thread_to_collapsed_thread
                        .get(&event.thread_id)
                        .unwrap_or(&event.thread_id),
                    args: None,
                });
            }
        }

        None
    });

    serializer.collect_seq(json_event_iterator)?;

    Ok(())
}

fn timestamp_to_min_max(timestamp: Timestamp) -> (SystemTime, SystemTime) {
    match timestamp {
        Timestamp::Instant(t) => (t, t),
        Timestamp::Interval { start, end } => {
            // Usually start should always be greater than end, but let's not
            // choke on invalid data here.
            (cmp::min(start, end), cmp::max(start, end))
        }
    }
}

// FIXME: Move this to `ProfilingData` and base it on the `start_time` field
//        from metadata.
fn make_start_timestamp(data: &ProfilingData) -> SystemTime {
    // We cannot assume the first event in the stream actually is the first
    // event because interval events are emitted at their end. So in theory it
    // is possible that the event with the earliest starting time is the last
    // event in the stream (i.e. if there is an interval event that spans the
    // entire execution of the profile).
    //
    // Let's be on the safe side and iterate the whole stream.
    let min = data
        .iter()
        .map(|e| timestamp_to_min_max(e.timestamp).0)
        .min()
        .unwrap();

    min - Duration::from_micros(1)
}
