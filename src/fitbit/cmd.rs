use chrono::NaiveDate;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "fitbit_exporter")]
pub struct Args {
    /// Dump historical metrics to a file instead of running as a server
    #[structopt(short = "d", long = "dump-historical-metrics")]
    pub dump_historical_metrics: bool,

    /// Start date for historical data export (inclusive). Defaults to 1 year ago.
    #[structopt(short = "s", long = "start-date", requires = "dump-historical-metrics")]
    pub start_date: Option<NaiveDate>,

    /// End date for historical data export (inclusive). Defaults to yesterday.
    #[structopt(short = "e", long = "end-date", requires = "dump-historical-metrics")]
    pub end_date: Option<NaiveDate>,

    /// Output file path for historical data export. Defaults to "fitbit_historical_metrics.prom"
    #[structopt(short = "o", long = "output-file", parse(from_os_str), requires = "dump-historical-metrics")]
    pub output_file: Option<PathBuf>,
}
