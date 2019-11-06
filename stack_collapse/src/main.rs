use std::error::Error;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use measureme::ProfilingData;

use structopt::StructOpt;

use tools_lib::stack_collapse::collapse_stacks;

#[derive(StructOpt, Debug)]
struct Opt {
    file_prefix: PathBuf,

    /// The sampling interval in milliseconds
    #[structopt(short = "i", long = "interval", default_value = "1")]
    interval: u64,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();

    let profiling_data = ProfilingData::new(&opt.file_prefix)?;

    let recorded_stacks = collapse_stacks(profiling_data.iter(), opt.interval);

    let mut file = BufWriter::new(File::create("out.stacks_folded")?);

    //now that we've got all of the recorded data, print the results to the output file
    for (unique_stack, count) in recorded_stacks {
        writeln!(file, "{} {}", unique_stack, count)?;
    }

    Ok(())
}
