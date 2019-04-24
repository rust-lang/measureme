use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::Duration;

use measureme::{Event, ProfilingData, TimestampKind};

use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Opt {
    file_prefix: PathBuf,

    /// The sampling interval in milliseconds
    #[structopt(short = "i", long = "interval", default_value = "1")]
    interval: u64,
}

fn main() -> Result<(), Box<std::error::Error>> {
    let opt = Opt::from_args();

    let profiling_data = ProfilingData::new(&opt.file_prefix);

    let mut recorded_stacks = HashMap::<String, usize>::new();

    let mut next_observation_time = {
        let current_time = profiling_data.iter().next().unwrap().timestamp;
        current_time + Duration::from_millis(1)
    };

    let mut thread_stacks: HashMap<u64, Vec<Event>> = HashMap::new();

    for event in profiling_data.iter() {
        //if this event is after the next_observation_time then we need to record the current stacks
        while event.timestamp > next_observation_time {
            for (_tid, stack) in &thread_stacks {
                let mut stack_string = String::new();
                stack_string.push_str("rustc;");

                for event in stack {
                    stack_string.push_str(&event.label);
                    stack_string.push(';');
                }

                //remove the trailing ';'
                stack_string.remove(stack_string.len() - 1);

                *recorded_stacks.entry(stack_string).or_default() += 1;

                next_observation_time += Duration::from_millis(opt.interval);
            }
        }

        let thread_stack = thread_stacks.entry(event.thread_id).or_default();

        match event.timestamp_kind {
            TimestampKind::Start => {
                thread_stack.push(event);
            },
            TimestampKind::End => {
                let previous_event = thread_stack.pop().expect("no start event found");
                assert_eq!(event.label, previous_event.label);
                assert_eq!(previous_event.timestamp_kind, TimestampKind::Start);
            },
            TimestampKind::Instant => { },
        }
    }

    let mut file = BufWriter::new(File::create("out.stacks_folded")?);

    //now that we've got all of the recorded data, print the results to the output file
    for (unique_stack, count) in recorded_stacks {
        writeln!(file, "{} {}", unique_stack, count)?;
    }

    Ok(())
}
