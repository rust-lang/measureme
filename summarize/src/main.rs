#[macro_use]
extern crate prettytable;

use std::path::PathBuf;
use measureme::ProfilingData;

use prettytable::{Table};
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

    let mut table = Table::new();

    table.add_row(row![
        "Item",
        "Self time",
        "% of total time",
        "Item count",
        "Cache hits",
        "Blocked time",
        "Incremental load time",
    ]);

    let total_time = results.total_time.as_nanos() as f64;

    for query_data in results.query_data {
        table.add_row(row![
            query_data.label,
            format!("{:.2?}", query_data.self_time),
            format!("{:.3}", ((query_data.self_time.as_nanos() as f64) / total_time) * 100.0),
            format!("{}", query_data.number_of_cache_hits + query_data.number_of_cache_misses),
            format!("{}", query_data.number_of_cache_hits),
            format!("{:.2?}", query_data.blocked_time),
            format!("{:.2?}", query_data.incremental_load_time),
        ]);
    }

    table.printstd();

    println!("Total cpu time: {:?}", results.total_time);
}
