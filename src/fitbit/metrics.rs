use chrono::{NaiveDateTime, DateTime, Utc};
use log::error;
use prometheus_client::metrics::gauge::MultiPointGauge;
use prometheus_client::registry::Registry;
use std::error::Error;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::fitbit::{FitbitClient,FitbitError};

// #[derive(Clone)]
pub struct FitbitMetrics {
    pub registry: Registry,
    pub steps: MultiPointGauge,

/* 
    // sleep metrics
    pub sleep_minutes_deep: Gauge,
    pub sleep_minutes_light: Gauge,
    pub sleep_minutes_rem: Gauge,
    pub sleep_minutes_wake: Gauge,
    pub sleep_duration: Gauge,
    pub sleep_efficiency: Gauge,
    pub sleep_start_time: Gauge,
    pub sleep_end_time: Gauge,
    pub total_time_in_bed: Gauge,
    pub total_minutes_asleep: Gauge,
    pub time_in_bed: Gauge,
    pub minutes_asleep: Gauge,
    pub minutes_awake: Gauge,
    pub minutes_after_wakeup: Gauge,
    pub is_main_sleep: Gauge,
 */
}

impl FitbitMetrics {
    pub fn new() -> Self {
        let mut registry = Registry::default();

        let steps = MultiPointGauge::<i64>::default();
        registry.register("fitbit_steps", "Total number of steps", steps.clone());

/* 
        let sleep_minutes_deep = register_metric!(registry, Gauge::<i64, AtomicI64>::default(), "fitbit_sleep_minutes_deep", "Total minutes of deep sleep");
        let sleep_minutes_light = register_metric!(registry, Gauge::<i64, AtomicI64>::default(), "fitbit_sleep_minutes_light", "Total minutes of light sleep");
        let sleep_minutes_rem = register_metric!(registry, Gauge::<i64, AtomicI64>::default(), "fitbit_sleep_minutes_rem", "Total minutes of REM sleep");
        let sleep_minutes_wake = register_metric!(registry, Gauge::<i64, AtomicI64>::default(), "fitbit_sleep_minutes_wake", "Total minutes of wake time during sleep");
        let sleep_duration = register_metric!(registry, Gauge::<i64, AtomicI64>::default(), "fitbit_sleep_duration", "Total sleep duration in minutes");
        let sleep_efficiency = register_metric!(registry, Gauge::<i64, AtomicI64>::default(), "fitbit_sleep_efficiency", "Sleep efficiency percentage");
        let sleep_start_time = register_metric!(registry, Gauge::<i64, AtomicI64>::default(), "fitbit_sleep_start_time", "Sleep start time as UNIX timestamp");
        let sleep_end_time = register_metric!(registry, Gauge::<i64, AtomicI64>::default(), "fitbit_sleep_end_time", "Sleep end time as UNIX timestamp");
        let total_time_in_bed = register_metric!(registry, Gauge::<i64, AtomicI64>::default(), "fitbit_total_time_in_bed", "Total time in bed in minutes");
        let total_minutes_asleep = register_metric!(registry, Gauge::<i64, AtomicI64>::default(), "fitbit_total_minutes_asleep", "Total minutes asleep");
        let time_in_bed = register_metric!(registry, Gauge::<i64, AtomicI64>::default(), "fitbit_time_in_bed", "Time in bed in minutes");
        let minutes_asleep = register_metric!(registry, Gauge::<i64, AtomicI64>::default(), "fitbit_minutes_asleep", "Minutes asleep");
        let minutes_awake = register_metric!(registry, Gauge::<i64, AtomicI64>::default(), "fitbit_minutes_awake", "Minutes awake");
        let minutes_after_wakeup = register_metric!(registry, Gauge::<i64, AtomicI64>::default(), "fitbit_minutes_after_wakeup", "Minutes after wakeup");
        let is_main_sleep = register_metric!(registry, Gauge::<i64, AtomicI64>::default(), "fitbit_is_main_sleep", "Is main sleep");
 */

        Self {
            registry,
            steps,

/* 
            sleep_minutes_deep,
            sleep_minutes_light,
            sleep_minutes_rem,
            sleep_minutes_wake,
            sleep_duration,
            sleep_efficiency,
            sleep_start_time,
            sleep_end_time,
            total_time_in_bed,
            total_minutes_asleep,
            time_in_bed,
            minutes_asleep,
            minutes_awake,
            minutes_after_wakeup,
            is_main_sleep,
 */
        }
    }
}


/// Fetches data from the Fitbit API and updates the corresponding metric.
///
/// This function is a generic utility for fetching data using a given future
/// and updating a metric by applying a provided update function.
///
/// # Arguments
///
/// * `fitbit_client` - An `Arc<RwLock<FitbitClient>>` containing the shared Fitbit client.
/// * `data_future` - A future that resolves to a `Result<T, FitbitError>`, where `T` is the data to be fetched.
/// * `update_metric` - A function that takes the fetched data `T` and returns a future `G` that resolves to `()`.
///                     This function is responsible for updating the corresponding metric using the fetched data.
///
/// # Type Parameters
///
/// * `T` - The type of the fetched data.
/// * `F` - The type of the function responsible for updating the metric.
/// * `G` - The type of the future returned by the update function, which resolves to `()`.
///
/// # Errors
///
/// Returns a `FitbitError` if there's an error while fetching the data or updating the metric.
async fn process_future<T, F, G>(
    fitbit_client: Arc<RwLock<FitbitClient>>,
    data_future: impl Future<Output = Result<T, FitbitError>>,
    callback: F,
) -> Result<(), FitbitError>
where
    F: FnOnce(T) -> G,
    G: Future<Output = T>,
{
    let read_locked_client = fitbit_client.read().await;
    match data_future.await {
        Ok(data) => {
            drop(read_locked_client); // Release the read lock explicitly before calling update_metric
            callback(data).await;
            Ok(())
        }
        Err(FitbitError::AccessTokenExpired) => {
            error!("Access token expired during a fetch operation");
            Err(FitbitError::AccessTokenExpired)
        }
        Err(e) => Err(e),
    }
}


/// Updates metrics by fetching data from the Fitbit API.
///
/// This function fetches data from the Fitbit API for different data categories
/// and updates the corresponding metrics using the fetched data.
///
/// # Arguments
///
/// * `fitbit_client` - An `Arc<RwLock<FitbitClient>>` containing the shared Fitbit client.
/// * `fitbit_metrics` - An `Arc<FitbitMetrics>` containing the shared Fitbit metrics.
///
/// # Errors
///
/// Returns a boxed error if there's an issue while updating the metrics.
pub async fn update_current_metrics(
    fitbit_client: Arc<RwLock<FitbitClient>>,
    fitbit_metrics: Arc<FitbitMetrics>,
) -> Result<(), Box<dyn Error>> {
    let read_locked_client = fitbit_client.read().await;

    // NOTE: actually no difference in response w.r.t. "only one day" vs "retrieve range"
    // https://dev.fitbit.com/build/reference/web-api/activity-timeseries/get-activity-timeseries-by-date/
    // TODO: commonize this - anyway set timestamp as the converted timestamp from datetime
    // confirm how prometheus treats the timestamp
    // => I can use `max_over_time(fitbit_steps[1d])` to visualize the max steps in days whose steps date were updated regularly and have multiple data points in a day. Also it can visualize historical data that only has one metric point in a day, both in consistent way

    // Update steps metric
    let steps_future = read_locked_client.fetch_steps();
    process_future(fitbit_client.clone(), steps_future, {
        move |steps| async move {
            match fitbit_metrics.steps.metric_points().len() {
                0 => fitbit_metrics.steps.push(steps as i64, None),
                1 => fitbit_metrics.steps.metric_points()[0] = (steps as i64, None),
                _ => error!("Unexpected number of metric points for steps metric: {}",
                            fitbit_metrics.steps.metric_points().len()),
            }
            steps
        }
    })
    .await?;

/* 
    // Update sleep metric
    let sleep_future = read_locked_client.fetch_sleep();

    fetch_and_update_metric(fitbit_client.clone(), sleep_future, {
        let fitbit_metrics = fitbit_metrics.clone();
        move |sleep_json| async move {
            if let Some(sleep) = sleep_json["sleep"].as_array().and_then(|arr| arr.get(0)) {
                let summary = &sleep["levels"]["summary"];
                fitbit_metrics.sleep_minutes_deep.set(summary["deep"]["minutes"].as_i64().unwrap_or(0));
                fitbit_metrics.sleep_minutes_light.set(summary["light"]["minutes"].as_i64().unwrap_or(0));
                fitbit_metrics.sleep_minutes_rem.set(summary["rem"]["minutes"].as_i64().unwrap_or(0));
                fitbit_metrics.sleep_minutes_wake.set(summary["wake"]["minutes"].as_i64().unwrap_or(0));

                fitbit_metrics.sleep_duration.set(sleep_json["summary"]["totalMinutesAsleep"].as_i64().unwrap_or(0));
                fitbit_metrics.sleep_efficiency.set(sleep["efficiency"].as_i64().unwrap_or(0));
                fitbit_metrics.total_time_in_bed.set(sleep_json["summary"]["totalTimeInBed"].as_i64().unwrap_or(0));
                fitbit_metrics.total_minutes_asleep.set(sleep_json["summary"]["totalMinutesAsleep"].as_i64().unwrap_or(0));

                if let (Some(start_time), Some(end_time)) = (
                    sleep["startTime"].as_str(),
                    sleep["endTime"].as_str(),
                ) {
                    fitbit_metrics.sleep_start_time.set(parse_datetime_to_unix_timestamp(start_time));
                    fitbit_metrics.sleep_end_time.set(parse_datetime_to_unix_timestamp(end_time));
                } else {
                    error!("Start or end time not found in sleep data");
                }

                fitbit_metrics.time_in_bed.set(sleep["timeInBed"].as_i64().unwrap_or(0));
                fitbit_metrics.minutes_asleep.set(sleep["minutesAsleep"].as_i64().unwrap_or(0));
                fitbit_metrics.minutes_awake.set(sleep["minutesAwake"].as_i64().unwrap_or(0));
                fitbit_metrics.minutes_after_wakeup.set(sleep["minutesAfterWakeup"].as_i64().unwrap_or(0));
                fitbit_metrics.is_main_sleep.set(sleep["isMainSleep"].as_bool().unwrap_or(false) as i64);
                
            } else {
                error!("Sleep data not found or in unexpected format");
            }
            sleep_json
        }
    })
    .await?;
 */

    Ok(())
}


/// Parses a datetime string in the format "%Y-%m-%dT%H:%M:%S%.f" and returns a UNIX timestamp.
///
/// # Arguments
///
/// * `datetime` - A string representing a datetime in the format "%Y-%m-%dT%H:%M:%S%.f".
///
/// # Example
///
/// ```
/// let timestamp = parse_datetime_to_unix_timestamp("2023-03-04T03:47:00.000");
/// ```
///
/// # Returns
///
/// A `i64` representing the UNIX timestamp of the provided datetime string, or 0 if an error occurs during parsing.
///
/// # Notes
///
/// The provided datetime string is expected to have a timezone-agnostic format. This function assumes the datetime
/// is in UTC when converting to a UNIX timestamp.
fn parse_datetime_to_unix_timestamp(datetime: &str) -> i64 {
    // The expected format for the datetime string
    let format = "%Y-%m-%dT%H:%M:%S%.f";

    // Attempt to parse the datetime string into a NaiveDateTime (which ignores the timezone)
    match NaiveDateTime::parse_from_str(datetime, format) {
        Ok(naive_dt) => {
            // If parsing is successful, convert the NaiveDateTime to a DateTime<Utc>
            let dt: DateTime<Utc> = DateTime::from_utc(naive_dt, Utc);
            dt.timestamp() // Return the UNIX timestamp of the DateTime<Utc>
        }
        Err(_) => {
            error!("Failed to parse date-time string: {}", datetime);
            0
        }
    }
}