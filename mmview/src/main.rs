use analyzeme::{Event, ProfilingData, Timestamp};
use std::error::Error;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Opt {
    file_prefix: PathBuf,

    /// Filter to events which occured on the specified thread id
    #[structopt(short = "t", long = "thread-id")]
    thread_id: Option<u32>,
}

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let opt = Opt::from_args();

    let data = ProfilingData::new(&opt.file_prefix)?;

    if let Some(global_start_time) = data.iter().map(|e| e.timestamp.start()).min() {
        for event in data.iter() {
            if let Some(thread_id) = opt.thread_id {
                if event.thread_id != thread_id {
                    continue;
                }
            }
            print_event(&event.to_event(), global_start_time);
        }
    } else {
        eprintln!("No events.");
    }

    Ok(())
}

fn system_time_to_micros_since(t: SystemTime, since: SystemTime) -> u128 {
    t.duration_since(since)
        .unwrap_or(Duration::from_nanos(0))
        .as_micros()
}

fn print_event(event: &Event<'_>, global_start_time: SystemTime) {
    let additional_data = event.additional_data.join(",");

    let timestamp = match event.timestamp {
        Timestamp::Instant(t) => {
            format!("{} μs", system_time_to_micros_since(t, global_start_time))
        }
        Timestamp::Interval { start, end } => format!(
            "{} μs - {} μs",
            system_time_to_micros_since(start, global_start_time),
            system_time_to_micros_since(end, global_start_time)
        ),
    };

    println!(
        r#"{{
    kind: {},
    label: {},
    additional_data: [{}],
    timestamp: {},
    thread_id: {},
}}"#,
        event.event_kind, event.label, additional_data, timestamp, event.thread_id
    );
}
