#[macro_use]
extern crate prettytable;

use analyzeme::ProfilingData;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::{path::PathBuf, time::Duration};

use prettytable::{Cell, Row, Table};
use serde::Serialize;
use structopt::StructOpt;

mod analysis;
mod diff;
mod query_data;
mod signed_duration;

use query_data::Results;

#[derive(StructOpt, Debug)]
struct DiffOpt {
    base: PathBuf,
    change: PathBuf,

    #[structopt(short = "e", long = "exclude")]
    exclude: Vec<String>,

    #[structopt(long = "json")]
    json: bool,
}

#[derive(StructOpt, Debug)]
struct SummarizeOpt {
    file_prefix: PathBuf,

    /// Writes the analysis to a json file next to <file_prefix> instead of stdout
    #[structopt(long = "json")]
    json: bool,

    /// Filter the output to items whose self-time is greater than this value
    #[structopt(short = "pa", long = "percent-above", default_value = "0.0")]
    percent_above: f64,
}

#[derive(StructOpt, Debug)]
enum Opt {
    #[structopt(name = "diff")]
    Diff(DiffOpt),

    /// Processes trace files and produces a summary
    #[structopt(name = "summarize")]
    Summarize(SummarizeOpt),
}

fn process_results(file: &PathBuf) -> Result<Results, Box<dyn Error + Send + Sync>> {
    if file.ends_with("json") {
        let reader = BufReader::new(File::open(&file)?);

        let results: Results = serde_json::from_reader(reader)?;
        Ok(results)
    } else {
        let data = ProfilingData::new(&file)?;

        Ok(analysis::perform_analysis(data))
    }
}

fn write_results_json(
    file: &PathBuf,
    results: impl Serialize,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let file = BufWriter::new(File::create(file.with_extension("json"))?);
    serde_json::to_writer(file, &results)?;
    Ok(())
}

fn diff(opt: DiffOpt) -> Result<(), Box<dyn Error + Send + Sync>> {
    let base = process_results(&opt.base)?;
    let change = process_results(&opt.change)?;

    let results = diff::calculate_diff(base, change);

    if opt.json {
        write_results_json(&opt.change, results)?;
        return Ok(());
    }

    let mut table = Table::new();

    table.add_row(row!(
        "Item",
        "Self Time",
        "Self Time Change",
        "Time",
        "Time Change",
        "Item count",
        "Cache hits",
        "Blocked time",
        "Incremental load time"
    ));

    for query_data in results.query_data {
        let exclude = opt.exclude.iter().any(|e| query_data.label.contains(e));
        if exclude {
            continue;
        }

        table.add_row(row![
            query_data.label,
            format!("{:.2?}", query_data.self_time),
            format!("{:+.2}%", query_data.self_time_change),
            format!("{:.2?}", query_data.time),
            format!("{:+.2}%", query_data.time_change),
            format!("{:+}", query_data.invocation_count),
            format!("{:+}", query_data.number_of_cache_hits),
            format!("{:.2?}", query_data.blocked_time),
            format!("{:.2?}", query_data.incremental_load_time),
        ]);
    }

    table.printstd();

    println!("Total cpu time: {:?}", results.total_time);

    Ok(())
}

fn summarize(opt: SummarizeOpt) -> Result<(), Box<dyn Error + Send + Sync>> {
    let data = ProfilingData::new(&opt.file_prefix)?;

    let mut results = analysis::perform_analysis(data);

    //just output the results into a json file
    if opt.json {
        write_results_json(&opt.file_prefix, &results)?;
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
    results
        .query_data
        .sort_by(|l, r| r.self_time.cmp(&l.self_time));

    let mut table = Table::new();

    let mut has_cache_hits = false;
    let mut has_blocked_time = false;
    let mut has_incremental_load_time = false;

    let duration_zero = Duration::from_secs(0);
    for r in &results.query_data {
        if r.number_of_cache_hits > 0 {
            has_cache_hits = true;
        }
        if r.blocked_time > duration_zero {
            has_blocked_time = true;
        }
        if r.incremental_load_time > duration_zero {
            has_incremental_load_time = true;
        }

        if has_cache_hits && has_blocked_time && has_incremental_load_time {
            break;
        }
    }

    // Don't show the cache hits, blocked time or incremental load time unless there are values
    // to display.
    let columns = &[
        ("Item", true),
        ("Self time", true),
        ("% of total time", true),
        ("Time", true),
        ("Item count", true),
        ("Cache hits", has_cache_hits),
        ("Blocked time", has_blocked_time),
        ("Incremental load time", has_incremental_load_time),
    ];

    fn filter_cells(cells: &[(&str, bool)]) -> Vec<Cell> {
        cells
            .iter()
            .filter(|(_, show)| *show)
            .map(|(cell, _)| Cell::new(cell))
            .collect()
    }

    table.add_row(Row::new(filter_cells(columns)));

    let total_time = results.total_time.as_nanos() as f64;
    let mut percent_total_time: f64 = 0.0;

    for query_data in results.query_data {
        let curr_percent = (query_data.self_time.as_nanos() as f64) / total_time * 100.0;
        if curr_percent < percent_above {
            break;
        } //no need to run entire loop if filtering by % time

        percent_total_time = percent_total_time + curr_percent;

        // Don't show the cache hits, blocked time or incremental load time columns unless there is
        // data to show.
        table.add_row(Row::new(filter_cells(&[
            (&query_data.label, true),
            (&format!("{:.2?}", query_data.self_time), true),
            (&format!("{:.3}", curr_percent), true),
            (&format!("{:.2?}", query_data.time), true),
            (&format!("{}", query_data.invocation_count), true),
            (
                &format!("{}", query_data.number_of_cache_hits),
                has_cache_hits,
            ),
            (
                &format!("{:.2?}", query_data.blocked_time),
                has_blocked_time,
            ),
            (
                &format!("{:.2?}", query_data.incremental_load_time),
                has_incremental_load_time,
            ),
        ])));
    }

    table.printstd();

    println!("Total cpu time: {:?}", results.total_time);

    if percent_above != 0.0 {
        println!(
            "Filtered results account for {:.3}% of total time.",
            percent_total_time
        );
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let opt = Opt::from_args();

    match opt {
        Opt::Summarize(opt) => summarize(opt),
        Opt::Diff(opt) => diff(opt),
    }
}
