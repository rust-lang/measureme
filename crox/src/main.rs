use std::collections::BTreeMap;
use std::fs;
use std::io::BufWriter;
use std::path::PathBuf;
use std::time::Duration;

use measureme::{ProfilingData, TimestampKind};

use serde::{Serialize, Serializer};
use structopt::StructOpt;

fn as_micros<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
    let v = (d.as_secs() * 1_000_000) + (d.subsec_nanos() as u64 / 1_000);
    s.serialize_u64(v)
}

#[derive(Clone, Copy, Eq, PartialEq, Serialize)]
enum EventType {
    #[serde(rename = "B")]
    Begin,
    #[serde(rename = "E")]
    End,
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
    #[serde(rename = "pid")]
    process_id: u32,
    #[serde(rename = "tid")]
    thread_id: u64,
    args: Option<BTreeMap<String, String>>,
}

#[derive(StructOpt, Debug)]
struct Opt {
    file_prefix: PathBuf,
}

fn main() -> Result<(), Box<std::error::Error>> {
    let opt = Opt::from_args();

    let data = ProfilingData::new(&opt.file_prefix);

    let chrome_file = BufWriter::new(fs::File::create("chrome_profiler.json")?);

    //find the earlier timestamp (it should be the first event)
    //subtract one tick so that the start of the event shows in Chrome
    let first_event_timestamp = data.iter().next().unwrap().timestamp - Duration::from_micros(1);

    let mut serializer = serde_json::Serializer::new(chrome_file);

    let mut event_iterator = data.iter();

    //create an iterator so we can avoid allocating a Vec with every Event for serialization
    let json_event_iterator = std::iter::from_fn(|| {
        while let Some(event) = event_iterator.next() {
            let event_type =
                match event.timestamp_kind {
                    TimestampKind::Start => EventType::Begin,
                    TimestampKind::End => EventType::End,
                    //Chrome does not seem to like how many QueryCacheHit events we generate
                    TimestampKind::Instant => continue,
                };

            return Some(Event {
                name: event.label.clone().into_owned(),
                category: event.event_kind.clone().into_owned(),
                event_type,
                timestamp: event.timestamp.duration_since(first_event_timestamp).unwrap(),
                process_id: 0,
                thread_id: event.thread_id,
                args: None,
            });
        }

        None
    });

    serializer.collect_seq(json_event_iterator)?;

    Ok(())
}
