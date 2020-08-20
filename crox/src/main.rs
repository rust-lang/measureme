use rustc_hash::FxHashMap;
use std::fs;
use std::io::BufWriter;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use analyzeme::{ProfilingData, Timestamp};

use serde::ser::SerializeSeq;
use serde::{Serialize, Serializer};
use serde_json::json;
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
    thread_id: u32,
    args: Option<FxHashMap<String, String>>,
}

#[derive(StructOpt, Debug)]
struct Opt {
    #[structopt(required_unless = "dir")]
    file_prefix: Vec<PathBuf>,
    /// all event trace files in dir will be merged to one chrome_profiler.json file
    #[structopt(long = "dir")]
    dir: Option<PathBuf>,
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
) -> FxHashMap<u32, u32> {
    let mut thread_to_collapsed_thread: FxHashMap<u32, u32> = FxHashMap::default();

    if opt.collapse_threads {
        // collect start and end times for all threads
        let mut thread_start_and_end: FxHashMap<u32, (SystemTime, SystemTime)> =
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

fn get_args(full_event: &analyzeme::Event) -> Option<FxHashMap<String, String>> {
    if !full_event.additional_data.is_empty() {
        Some(
            full_event
                .additional_data
                .iter()
                .enumerate()
                .map(|(i, arg)| (if let Some(name) = &arg.name {
                    name.to_string()
                } else {
                    format!("arg{}", i).to_string()
                }, arg.value.to_string()))
                .collect(),
        )
    } else {
        None
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();

    let chrome_file = BufWriter::new(fs::File::create("chrome_profiler.json")?);
    let mut serializer = serde_json::Serializer::new(chrome_file);

    let mut seq = serializer.serialize_seq(None)?;

    let dir_paths = file_prefixes_in_dir(&opt)?;

    for file_prefix in opt.file_prefix.iter().chain(dir_paths.iter()) {
        let data = ProfilingData::new(&file_prefix)?;

        let thread_to_collapsed_thread = generate_thread_to_collapsed_thread_mapping(&opt, &data);

        // Chrome does not seem to like how many QueryCacheHit events we generate
        // only handle Interval events for now
        for event in data.iter().filter(|e| !e.timestamp.is_instant()) {
            let duration = event.duration().unwrap();
            if let Some(minimum_duration) = opt.minimum_duration {
                if duration.as_micros() < minimum_duration {
                    continue;
                }
            }
            let full_event = event.to_event();
            let crox_event = Event {
                name: full_event.label.clone().into_owned(),
                category: full_event.event_kind.clone().into_owned(),
                event_type: EventType::Complete,
                timestamp: event.timestamp.start().duration_since(UNIX_EPOCH).unwrap(),
                duration,
                process_id: data.metadata.process_id,
                thread_id: *thread_to_collapsed_thread
                    .get(&event.thread_id)
                    .unwrap_or(&event.thread_id),
                args: get_args(&full_event),
            };
            seq.serialize_element(&crox_event)?;
        }
        // add crate name for the process_id
        let index_of_crate_name = data
            .metadata
            .cmd
            .find(" --crate-name ")
            .map(|index| index + 14);
        if let Some(index) = index_of_crate_name {
            let (_, last) = data.metadata.cmd.split_at(index);
            let (crate_name, _) = last.split_at(last.find(" ").unwrap_or(last.len()));

            let process_name = json!({
                "name": "process_name",
                "ph" : "M",
                "ts" : 0,
                "tid" : 0,
                "cat" : "",
                "pid" : data.metadata.process_id,
                "args": {
                    "name" : crate_name
                }
            });
            seq.serialize_element(&process_name)?;
        }
        // sort the processes after start time
        let process_name = json!({
            "name": "process_sort_index",
            "ph" : "M",
            "ts" : 0,
            "tid" : 0,
            "cat" : "",
            "pid" : data.metadata.process_id,
            "args": {
                "sort_index" : data.metadata.start_time.duration_since(UNIX_EPOCH).unwrap().as_micros() as u64
            }
        });
        seq.serialize_element(&process_name)?;
    }

    seq.end()?;

    Ok(())
}

fn file_prefixes_in_dir(opt: &Opt) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut result = Vec::new();
    if let Some(dir_path) = &opt.dir {
        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().filter(|e| *e == "events").is_some() {
                result.push(path)
            }
        }
    }
    Ok(result)
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
