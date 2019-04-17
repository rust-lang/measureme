use std::path::PathBuf;
use measureme::ProfilingData;

use structopt::StructOpt;

mod analysis;

#[derive(StructOpt, Debug)]
struct Opt {
    file_prefix: PathBuf,
}

fn main() {
    let opt = Opt::from_args();

    let data = ProfilingData::new(&opt.file_prefix);

    let mut results = analysis::perform_analysis(data);

    //order the results by descending self time
    results.query_data.sort_by(|l, r| r.self_time.cmp(&l.self_time));

    println!("| Item | Self Time | % of total time | Number of invocations \
              | Cache hits | Blocked time |");

    for query_data in results.query_data {
        println!(
            "{} | {:?} | {} | {} | {:?} |",
            query_data.label,
            query_data.self_time,
            query_data.number_of_cache_hits + query_data.number_of_cache_misses,
            query_data.number_of_cache_hits,
            query_data.blocked_time,
        );
    }

    println!("Total cpu time: {:?}", results.total_time);
}
