use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;

use analyzeme::{collapse_stacks, ProfilingData};
use inferno::flamegraph::{from_lines, Options as FlamegraphOptions};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Opt {
    file_prefix: PathBuf,
}

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let opt = Opt::from_args();

    let profiling_data = ProfilingData::new(&opt.file_prefix)?;

    let recorded_stacks = collapse_stacks(&profiling_data)
        .iter()
        .map(|(unique_stack, count)| format!("{} {}", unique_stack, count))
        .collect::<Vec<_>>();

    let file = BufWriter::new(File::create("rustc.svg")?);
    let mut flamegraph_options = FlamegraphOptions::default();

    from_lines(
        &mut flamegraph_options,
        recorded_stacks.iter().map(|s| s.as_ref()),
        file,
    )
    .expect(
        "unable to generate a flamegraph \
         from the collapsed stack data",
    );

    Ok(())
}
