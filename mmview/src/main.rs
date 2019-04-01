use std::path::PathBuf;
use measureme::ProfilingData;

use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Opt {
    file_prefix: PathBuf,
}

fn main() {
    let opt = Opt::from_args();

    let data = ProfilingData::new(&opt.file_prefix);

    for event in data.iter() {
        println!("{:?}", event);
    }
}
