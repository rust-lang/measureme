use std::collections::BTreeMap;
use std::fs;
use std::io::BufWriter;
use std::path::PathBuf;
use std::time::Duration;
use std::time::SystemTime;

use measureme::{MatchingEvent, ProfilingData};

use serde::{Serialize, Serializer};
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
    args: Option<BTreeMap<String, String>>,
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
) -> BTreeMap<u64, u64> {
    let mut thread_to_collapsed_thread: BTreeMap<u64, u64> = BTreeMap::new();

    if opt.collapse_threads {
        // collect start and end times for all threads
        let mut thread_start_and_end: BTreeMap<u64, (SystemTime, SystemTime)> = BTreeMap::new();
        for event in data.iter() {
            thread_start_and_end
                .entry(event.thread_id)
                .and_modify(|(start, end)| {
                    if *start > event.timestamp {
                        *start = event.timestamp;
                    } else if *end < event.timestamp {
                        *end = event.timestamp;
                    }
                })
                .or_insert_with(|| (event.timestamp, event.timestamp));
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
    let first_event_timestamp = data.iter().next().unwrap().timestamp - Duration::from_micros(1);

    let mut serializer = serde_json::Serializer::new(chrome_file);
    let thread_to_collapsed_thread = generate_thread_to_collapsed_thread_mapping(&opt, &data);
    let mut event_iterator = data.iter_matching_events();

    //create an iterator so we can avoid allocating a Vec with every Event for serialization
    let json_event_iterator = std::iter::from_fn(|| {
        while let Some(event) = event_iterator.next() {
            // Chrome does not seem to like how many QueryCacheHit events we generate
            // only handle startStop events for now
            if let MatchingEvent::StartStop(start, stop) = event {
                let duration = stop.timestamp.duration_since(start.timestamp).unwrap();
                if let Some(minimum_duration) = opt.minimum_duration {
                    if duration.as_micros() < minimum_duration {
                        continue;
                    }
                }
                return Some(Event {
                    name: start.label.clone().into_owned(),
                    category: start.event_kind.clone().into_owned(),
                    event_type: EventType::Complete,
                    timestamp: start
                        .timestamp
                        .duration_since(first_event_timestamp)
                        .unwrap(),
                    duration,
                    process_id: 0,
                    thread_id: *thread_to_collapsed_thread
                        .get(&start.thread_id)
                        .unwrap_or(&start.thread_id),
                    args: None,
                });
            }
        }

        None
    });

    serializer.collect_seq(json_event_iterator)?;

    Ok(())
}
