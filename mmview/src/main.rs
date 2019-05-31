use std::error::Error;
use std::path::PathBuf;
use measureme::ProfilingData;

use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Opt {
    file_prefix: PathBuf,

    /// Filter to events which occured on the specified thread id
    #[structopt(short = "t", long = "thread-id")]
    thread_id: Option<u64>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();

    let data = ProfilingData::new(&opt.file_prefix)?;

    for event in data.iter() {
        if let Some(thread_id) = opt.thread_id {
            if event.thread_id != thread_id {
                continue;
            }
        }

        println!("{:?}", event);
    }

    Ok(())
}
