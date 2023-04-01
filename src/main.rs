use dotenv::dotenv;
use std::env;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use structopt::StructOpt;
use tokio::sync::RwLock;

mod fitbit;
use fitbit::{cmd, FitbitClient, FitbitMetrics, run_server, refresh_token_periodically, dump_historical_metrics};

// FYI: The default access token expiration time is 8hr (28800). Defining a shorter refresh interval.
// See https://dev.fitbit.com/build/reference/web-api/developer-guide/authorization/
const REFRESH_ACCESS_TOKEN_INTERVAL: Duration = Duration::from_secs(7 * 60 * 60);

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize the logger. to see debug messages, for example, set RUST_LOG=fitbit_exporter=debug when `cargo run` is executed.
    env_logger::init();

    // Load environment variables from .env file
    dotenv().ok();

    // Read the required environment variables
    let client_id = env::var("FITBIT_CLIENT_ID").expect("FITBIT_CLIENT_ID not set");
    let client_secret = env::var("FITBIT_CLIENT_SECRET").expect("FITBIT_CLIENT_SECRET not set");
    let initial_access_token = env::var("FITBIT_ACCESS_TOKEN").expect("FITBIT_ACCESS_TOKEN not set");

    // Set the refresh token if given via FITBIT_REFRESH_TOKEN. Otherwise set None.
    // The refresh token is only needed for the Authorization Code Flow (`response_type=code`) when calling https://www.fitbit.com/oauth2/authorize.
    // If the Inplicit Grant Flow is used (`response_type=token`) the refresh token is not needed.
    let refresh_token: Option<String> = env::var("FITBIT_REFRESH_TOKEN").map_or(None, |refresh_token| Some(refresh_token));

    // Initialize and wrap the FitbitClient and FitbitMetrics instances in Arc (Atomic Reference Counting) to
    // allow safe sharing and handling of the instances across multiple threads.Gkj
    // Especially, FitbitClient is wrapped by RwLock as well to allow safe updating of the access token.
    let fitbit_client = FitbitClient::new(&client_id, &client_secret, &refresh_token, &initial_access_token);
    let shared_fitbit_client = Arc::new(RwLock::new(fitbit_client));
    let shared_fitbit_metrics = Arc::new(FitbitMetrics::new());

    let args = cmd::Args::from_args();
    if args.dump_historical_metrics {
        // Dump historical metrics to a file (.prom) instead of serving them via HTTP
        dump_historical_metrics(shared_fitbit_client, shared_fitbit_metrics, args).await?;
    } else {
        // Spawn a task to refresh the access token periodically
        tokio::spawn(refresh_token_periodically(shared_fitbit_client.clone(), REFRESH_ACCESS_TOKEN_INTERVAL));

        // Start the HTTP server to serve the metrics for Prometheus
        run_server(shared_fitbit_client.clone(), shared_fitbit_metrics).await?;
    }

    Ok(())
}
