use hyper::{header, Body, Request, Response, Server, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use std::time::Duration;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use log::{debug, error, info};
// use prometheus::{Encoder, TextEncoder};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::Path;
use std::fs::read_to_string;
use std::sync::Arc;
use tokio::sync::RwLock;
use prometheus_client::encoding::text::encode;

use crate::fitbit::{FitbitClient, FitbitMetrics, update_current_metrics};

/// Start and run an HTTP server that serves the Fitbit metrics for Prometheus to scrape.
///
/// # Arguments
///
/// * `client` - An `Arc<RwLock<FitbitClient>>` that provides access to the shared Fitbit client.
/// * `shared_fitbit_metrics` - An `Arc<FitbitMetrics>` that provides access to the shared Fitbit metrics.
///
/// # Errors
///
/// Returns an error if the server encounters an issue while running.
pub async fn run_server(client: Arc<RwLock<FitbitClient>>, shared_fitbit_metrics: Arc<FitbitMetrics>) -> Result<(), Box<dyn std::error::Error>> {
    // Use make_service_fn to create a new service function for each connection to the server.
    // The move |_| captures the `shared_*`, making them accessible within the closure.
    let make_svc = make_service_fn(move |_| {
        let cloned_fitbit_client = Arc::clone(&client);
        let cloned_fitbit_metrics = Arc::clone(&shared_fitbit_metrics);

        async move {
            // Return an infallible service function that takes an incoming request and
            // calls the metrics_handler with the cloned Arc pointers.
            Ok::<_, Infallible>(service_fn(move |req| {
                metrics_handler(req, cloned_fitbit_client.clone(), cloned_fitbit_metrics.clone())
            }))
        }
    });

    // Set up the HTTP server for Prometheus to scrape the metrics
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let server = Server::bind(&addr).serve(make_svc);
    info!("Server running on http://{}", addr);

    server.await.unwrap_or_else(|e| error!("An error occurred while running the server: {}", e));

    Ok(())
}


/// Handles HTTP requests for the /metrics endpoint.
///
/// This function serves Prometheus metrics by fetching data from the Fitbit API,
/// updating the FitbitMetrics struct, and encoding the metrics for Prometheus.
///
/// # Arguments
///
/// * `req` - The incoming HTTP request.
/// * `fitbit_client` - An Arc<RwLock<FitbitClient>> to access the Fitbit API.
/// * `fitbit_metrics` - An Arc<FitbitMetrics> to store and update the metrics.
///
/// # Returns
///
/// * A Result containing an HTTP Response, or an Infallible error.
async fn metrics_handler(
    req: Request<Body>,
    fitbit_client: Arc<RwLock<FitbitClient>>,
    fitbit_metrics: Arc<FitbitMetrics>,
) -> Result<Response<Body>, Infallible> {
    match (req.method(), req.uri().path()) {
        (&hyper::Method::GET, "/metrics") => {
            // Update the metrics - fetch the latest data from the Fitbit API (considering changing the function name)
            match update_current_metrics(fitbit_client.clone(), fitbit_metrics.clone()).await {
                Err(err) => build_error_response(format!("Error updating metrics: {:?}", err)),
                Ok(_) => {
                    // Encode the metrics for Prometheus
                    let mut txt = String::new();
                    encode(&mut txt, &fitbit_metrics.registry).unwrap();
                    build_text_response(txt)
                }
            }
        },
        // Retrieves 1y steps per day via Fitbit API (not from a .prom file). Controle by Prometheus scraping frequency.
        (&hyper::Method::GET, "/history") => {

        let yesterday = Utc::now().date_naive().pred_opt().unwrap();
        let start_date = yesterday - ChronoDuration::days(30); // to get 1 month (days(30)) of data during testing. In production, use days(365)

        let read_locked_client = fitbit_client.read().await;

        let steps_range_data = read_locked_client.fetch_steps_range(start_date, yesterday).await;
        for (date, steps) in steps_range_data {
            // Currently, I treat the NativeDate as UTC. Possibly Fitbit user's timezone configuration can be used:
            // https://dev.fitbit.com/build/reference/web-api/user/get-profile/
            let datetime_utc = DateTime::<Utc>::from_utc(date.and_hms_opt(0, 0, 0).unwrap(), Utc);
            let timestamp = datetime_utc.timestamp() as u64;
            debug!("date: {:?}, steps: {}, converted timestamp: {:?}", date, steps, timestamp);

            fitbit_metrics.steps.push(steps as i64, Some(Duration::from_secs(timestamp)));
        }

        let mut txt = String::new();
        encode(&mut txt, &fitbit_metrics.registry).unwrap();
        build_text_response(txt)

/* 
            // Read the contents of the .prom file
            let file_path = Path::new("fitbit_historical_metrics.prom");
            match read_to_string(&file_path) {
                Ok(content) => build_text_response(content),
                Err(err) => build_error_response(format!("Error reading file: {:?}", err))
            }
 */

        },
        // Return a 404 Not Found response for any other request path.
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not found"))
            .unwrap()),
    }
}

fn build_text_response(txt: String) -> Result<Response<Body>, Infallible> {
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/plain; version=0.0.4")
        .body(Body::from(txt))
        .unwrap())
}

fn build_error_response(err_msg: String) -> Result<Response<Body>, Infallible> {
    error!("{}", err_msg);
    Ok(Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Body::from(err_msg))
        .unwrap())
}