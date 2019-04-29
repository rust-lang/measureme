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

    /// Filter the output to items whose self-time is greater than this value
    #[structopt(short = "pa", long = "percent-above", default_value = "0.0")]
    percent_above: f64,
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

    let percent_above = opt.percent_above;
    //cannot be greater than 100% or less than 0%
    if percent_above > 100.0 {
        eprintln!("Percentage of total time cannot be more than 100.0");
        std::process::exit(1);
    } else if percent_above < 0.0 {
        eprintln!("Percentage of total time cannot be less than 0.0");
        std::process::exit(1);
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
    let mut percent_total_time: f64 = 0.0;

    for query_data in results.query_data {

        let curr_percent = (query_data.self_time.as_nanos() as f64) / total_time * 100.0;
        if curr_percent < percent_above { break } //no need to run entire loop if filtering by % time

        percent_total_time = percent_total_time + curr_percent;

        table.add_row(row![
            query_data.label,
            format!("{:.2?}", query_data.self_time),
            format!("{:.3}", curr_percent),
            format!("{}", query_data.invocation_count),
            format!("{}", query_data.number_of_cache_hits),
            format!("{:.2?}", query_data.blocked_time),
            format!("{:.2?}", query_data.incremental_load_time),
        ]);
    }

    table.printstd();

    println!("Total cpu time: {:?}", results.total_time);

    if percent_above != 0.0 {
        println!("Filtered results account for {:.3}% of total time.", percent_total_time);
    }

    Ok(())
}
