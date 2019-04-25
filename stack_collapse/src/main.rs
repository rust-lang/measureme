use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::Duration;

use measureme::ProfilingData;

use structopt::StructOpt;

mod stack_collapse;

use stack_collapse::collapse_stacks;

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

    let first_event_time = {
        let current_time = profiling_data.iter().next().unwrap().timestamp;
        current_time + Duration::from_millis(opt.interval)
    };

    let recorded_stacks = collapse_stacks(profiling_data.iter(), first_event_time, opt.interval);

    let mut file = BufWriter::new(File::create("out.stacks_folded")?);

    //now that we've got all of the recorded data, print the results to the output file
    for (unique_stack, count) in recorded_stacks {
        writeln!(file, "{} {}", unique_stack, count)?;
    }

    Ok(())
}
