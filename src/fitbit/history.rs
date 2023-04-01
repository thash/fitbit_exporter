use chrono::{DateTime, Duration as ChronoDuration, Utc};
use chrono::NaiveDate;
use prometheus_client::registry::Registry;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::sleep;
use std::path::Path;
use std::fs::File;
use std::io::Write;
use prometheus_client::encoding::text::encode;
use std::time::Duration;
use tokio::sync::RwLock;
use log::debug;

use crate::fitbit::FitbitClient;
use crate::fitbit::FitbitMetrics;
use crate::fitbit::cmd;


pub async fn dump_historical_metrics(client: Arc<RwLock<FitbitClient>>, metrics: Arc<FitbitMetrics>, args: cmd::Args) -> Result<(), Box<dyn Error>> {
    let yesterday = Utc::now().date_naive().pred_opt().unwrap();
    let start_date = args.start_date.unwrap_or_else(|| yesterday - ChronoDuration::days(365));
    let end_date = args.end_date.unwrap_or_else(|| yesterday);
    let output_file = args.output_file.unwrap_or_else(|| PathBuf::from("fitbit_historical_metrics.prom"));
    debug!("start_date: {:?}, end_date: {:?}, output_file: {:?}", start_date, end_date, output_file);

    let read_locked_client = client.read().await;

    let steps_range_data = read_locked_client.fetch_steps_range(start_date, end_date).await?;
    for (date, steps) in steps_range_data {
        // Currently, I treat the NativeDate as UTC. Possibly Fitbit user's timezone configuration can be used:
        // https://dev.fitbit.com/build/reference/web-api/user/get-profile/
        let datetime_utc = DateTime::<Utc>::from_utc(date.and_hms_opt(0, 0, 0).unwrap(), Utc);
        let timestamp = datetime_utc.timestamp() as u64;
        debug!("date: {:?}, steps: {}, converted timestamp: {:?}", date, steps, timestamp);

        metrics.steps.push(steps as i64, Some(Duration::from_secs(timestamp)));
    }

    let mut txt = String::new();
    encode(&mut txt, &metrics.registry).unwrap();
    println!("=== [Command Line Mode] in the `dump_historical_metrics` > txt >>> ===\n{}", txt);
    println!("=== <<< txt");

    let mut file = File::create(&output_file)?;
    file.write_all(txt.as_bytes())?;

    Ok(())
}
