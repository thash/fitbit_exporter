pub mod cmd;
pub mod client;
pub mod metrics;
pub mod server;
pub mod history; 

// Re-export structs and functions
pub use client::{FitbitClient, FitbitError};
pub use metrics::{FitbitMetrics, update_current_metrics};
pub use server::run_server;
pub use client::refresh_token_periodically;
pub use history::dump_historical_metrics;