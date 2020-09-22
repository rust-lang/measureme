use std::error::Error;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use analyzeme::{collapse_stacks, ProfilingData};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Opt {
    file_prefix: PathBuf,
}

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let opt = Opt::from_args();

    let profiling_data = ProfilingData::new(&opt.file_prefix)?;

    let recorded_stacks = collapse_stacks(&profiling_data);

    let mut file = BufWriter::new(File::create("out.stacks_folded")?);

    //now that we've got all of the recorded data, print the results to the output file
    for (unique_stack, count) in recorded_stacks {
        writeln!(file, "{} {}", unique_stack, count)?;
    }

    Ok(())
}
