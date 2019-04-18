#[macro_use]
extern crate prettytable;

use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use measureme::ProfilingData;

use prettytable::{Table};
use structopt::StructOpt;

mod analysis;

#[derive(StructOpt, Debug)]
struct Opt {
    file_prefix: PathBuf,

    /// Writes the analysis to a json file next to <file_prefix> instead of stdout
    #[structopt(long = "json")]
    json: bool,
}

fn main() -> Result<(), Box<std::error::Error>> {
    let opt = Opt::from_args();

    let data = ProfilingData::new(&opt.file_prefix);

    let mut results = analysis::perform_analysis(data);

    //just output the results into a json file
    if opt.json {
        let file = BufWriter::new(File::create(opt.file_prefix.with_extension("json"))?);
        serde_json::to_writer(file, &results)?;
        return Ok(());
    }

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
            format!("{}", query_data.invocation_count),
            format!("{}", query_data.number_of_cache_hits),
            format!("{:.2?}", query_data.blocked_time),
            format!("{:.2?}", query_data.incremental_load_time),
        ]);
    }

    table.printstd();

    println!("Total cpu time: {:?}", results.total_time);

    Ok(())
}
