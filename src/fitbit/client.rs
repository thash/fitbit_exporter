use chrono::NaiveDate;
use log::{debug, error};
use oauth2::{AccessToken, AuthUrl, ClientId, ClientSecret, RefreshToken, TokenResponse, TokenUrl};
use oauth2::basic::{BasicClient, BasicErrorResponseType};
use oauth2::reqwest::async_http_client;
use reqwest::Url;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::RwLock;


// Define the FitbitError
#[derive(Debug, Error)]
pub enum FitbitError {
    #[error("HTTP error: {0}")]
    HttpError(reqwest::Error),

    #[error("URL error: {0}")]
    UrlError(url::ParseError),

    #[error("Invalid data format")]
    InvalidData,

    #[error("Access token expired")]
    AccessTokenExpired,

    #[error("Invalid grant - e.g. invalid refresh token")]
    InvalidGrant,

    #[error("Token error: {0}")]
    TokenError(String),
}

/// A client for interacting with the Fitbit API.
///
/// The `FitbitClient` provides methods for refreshing access tokens, fetching data from the Fitbit API,
/// and fetching specific data, such as the number of steps.
#[derive(Clone)]
pub struct FitbitClient {
    client: BasicClient,
    pub refresh_token: Option<RefreshToken>,
    access_token: AccessToken,
}

// Implement methods for the FitbitClient struct
impl FitbitClient {
    /// Creates a new instance of `FitbitClient` using the provided access token and refresh token.
    /// The refresh_token is used to refresh the access token when it expires.
    ///
    /// # Arguments
    ///
    /// * `access_token` - The access token for the Fitbit API.
    /// * `refresh_token` - The refresh token for the Fitbit API.
    pub fn new(client_id: &str, client_secret: &str, refresh_token: &Option<String>, initial_access_token: &str) -> Self {
        let client = BasicClient::new(
            ClientId::new(client_id.to_string()),
            Some(ClientSecret::new(client_secret.to_string())),
            AuthUrl::new("https://www.fitbit.com/oauth2/authorize".to_string()).expect("Invalid authorization endpoint URL"),
            Some(TokenUrl::new("https://api.fitbit.com/oauth2/token".to_string()).expect("Invalid token endpoint URL")),
        );

        Self {
            client,
            refresh_token: refresh_token.as_ref().map(|token| RefreshToken::new(token.to_string())),
            access_token: AccessToken::new(initial_access_token.to_string()),
        }
    }

    /// Refreshes the access token using the refresh token, which is passed via the environment variable FITBIT_REFRESH_TOKEN
    /// When to use: With the Authorization Code Flow, the access token should be updated when it expires. With the Implicit Grant Flow, the access token won't be updated and you need to pass a new access token via the environment variable FITBIT_ACCESS_TOKEN.
    ///
    /// This method is used to update the access token when it expires. The refresh token is updated as well.
    /// If the refresh token is not set, it will print a warning message and return early.
    ///
    /// # Errors
    ///
    /// Returns an error variant of `FitbitError` if the token refresh fails or encounters an issue.
    pub async fn refresh_access_token(&mut self) -> Result<(), FitbitError> {
        debug!("Refreshing access token...");
        // If the refresh token is set, proceed with the token refresh. Otherwise, print a warning message and return early.
        if let Some(refresh_token) = &self.refresh_token {
            let token_result = self.client
                .exchange_refresh_token(&RefreshToken::new(refresh_token.secret().to_string()))
                .request_async(async_http_client)
                .await;

            match token_result {
                Ok(token_result) => {
                    self.access_token = token_result.access_token().clone();
                    debug!("Access token successfully refreshed");

                    // The response should includes a new "refresh" token as well, which we need to store for the next refresh.
                    // FYI: https://dev.fitbit.com/build/reference/web-api/authorization/refresh-token/
                    if let Some(new_refresh_token) = token_result.refresh_token() {
                        self.refresh_token = Some(new_refresh_token.clone());
                        debug!("New refresh token received and updated");
                    }
                }
                Err(oauth2::RequestTokenError::ServerResponse(err_resp)) => {
                    if *err_resp.error() == BasicErrorResponseType::InvalidGrant {
                        return Err(FitbitError::InvalidGrant);
                    } else {
                        return Err(FitbitError::TokenError(format!("Server response error: {:?}", err_resp)));
                    }
                },
                Err(err) => return Err(FitbitError::TokenError(format!("Request token error: {:?}", err))),
            }
    
        } else {
            error!("Warning: Refresh token is not set. Skipping access token refresh.");
        }
    
        Ok(())
    }

    /// Fetches data from the Fitbit API for the given endpoint.
    ///
    /// This is a general-purpose method that takes an API endpoint as a parameter and returns the
    /// JSON response as a `serde_json::Value`.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - The API endpoint to fetch data from.
    ///
    /// # Errors
    ///
    /// Returns an error variant of `FitbitError` if there is a problem with the request, such as
    /// an expired token or invalid data.
    async fn fetch_data(&self, endpoint: &str) -> Result<Value, FitbitError> {
    // async fn fetch_data(&mut self, endpoint: &str) -> Result<Value, FitbitError> {
        debug!("Fetching data from endpoint: {}", endpoint);
        let url = Url::parse(endpoint).map_err(FitbitError::UrlError)?;
        let response = reqwest::Client::new()
            .get(url.clone())
            .bearer_auth(self.access_token.secret())
            .send()
            .await
            .map_err(FitbitError::HttpError)?;

        let json: Value = response.json().await.map_err(FitbitError::HttpError)?;
        if json["errors"][0]["errorType"].as_str() == Some("expired_token") {
            debug!("Access token expired.");
            return Err(FitbitError::AccessTokenExpired);
        }
        debug!("Data fetched successfully");
        Ok(json)
    }

    /// Fetches the number of steps from the Fitbit API, by using:
    /// https://dev.fitbit.com/build/reference/web-api/activity-timeseries/get-activity-timeseries-by-date/
    ///
    /// This method calls the `fetch_data` internally and extracts the number of steps from the JSON response.
    ///
    /// # Errors
    ///
    /// Returns an error variant of `FitbitError` if there is a problem with the request, such as
    /// an expired token or invalid data.
    pub async fn fetch_steps(&self) -> Result<u64, FitbitError> {
    // pub async fn fetch_steps(&mut self) -> Result<u64, FitbitError> {
        debug!("Fetching steps data...");
        let json = self
            .fetch_data("https://api.fitbit.com/1/user/-/activities/steps/date/today/1d.json")
            .await?;
        let steps = json["activities-steps"][0]["value"]
            .as_str()
            .ok_or(FitbitError::InvalidData)?
            .parse::<u64>()
            .map_err(|_| FitbitError::InvalidData)?;
        debug!("Fetched steps: {}", steps);
        Ok(steps)
    }

    // pub async fn fetch_steps_for_past_month(&mut self) -> Result<Vec<u64>, FitbitError> {
    //     debug!("Fetching steps data for past month...");
    //     let json = self
    //         .fetch_data("https://api.fitbit.com/1/user/-/activities/steps/date/today/1m.json")
    //         .await?;
    //     let steps = json["activities-steps"]
    //         .as_array()
    //         .ok_or(FitbitError::InvalidData)?
    //         .iter()
    //         .map(|step| step["value"].as_str().ok_or(FitbitError::InvalidData))
    //         .collect::<Result<Vec<&str>, FitbitError>>()?
    //         .iter()
    //         .map(|step| step.parse::<u64>().map_err(|_| FitbitError::InvalidData))
    //         .collect::<Result<Vec<u64>, FitbitError>>()?;
    //     debug!("Fetched steps: {:?}", steps);
    //     Ok(steps)
    // }

    pub async fn fetch_sleep(&self) -> Result<Value, FitbitError> {
        let json = self
            // .fetch_data("https://api.fitbit.com/1.2/user/-/sleep/date/today.json") // FIXME
            .fetch_data("https://api.fitbit.com/1.2/user/-/sleep/date/2023-03-04.json")
            .await?;
        debug!("Fetched sleep: {:?}", json);
        Ok(json)
    }

    // pub async fn fetch_weight(&self) -> Result<Value, FitbitError> {
    //     let json = self
    //         .fetch_data("https://api.fitbit.com/1/user/-/body/log/weight/date/today.json")
    //         .await?;
    //     Ok(json)
    // }



    pub async fn fetch_steps_range(&self, start_date: NaiveDate, end_date: NaiveDate) -> Result<Vec<(NaiveDate, u64)>, FitbitError> {
        debug!("Fetching historical steps data from {} to {}", start_date, end_date);
    
        let start_date_str = start_date.format("%Y-%m-%d").to_string();
        let end_date_str = end_date.format("%Y-%m-%d").to_string();
        let endpoint = format!("https://api.fitbit.com/1/user/-/activities/steps/date/{}/{}.json", start_date_str, end_date_str);
    
        let json = self.fetch_data(&endpoint).await?;
    
        let steps_data = json["activities-steps"]
            .as_array()
            .ok_or(FitbitError::InvalidData)?;
    
        let mut results: Vec<(NaiveDate, u64)> = Vec::new();
    
        for entry in steps_data {
            let date_str = entry["dateTime"]
                .as_str()
                .ok_or(FitbitError::InvalidData)?;
    
            let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                .map_err(|_| FitbitError::InvalidData)?;
    
            let steps = entry["value"]
                .as_str()
                .ok_or(FitbitError::InvalidData)?
                .parse::<u64>()
                .map_err(|_| FitbitError::InvalidData)?;
    
            results.push((date, steps));
        }
    
        debug!("Fetched historical steps data: {:?}", results);
        Ok(results)
    }
    
}


/// Refresh the access token periodically at the specified interval.
///
/// This function is designed to run in an async loop, refreshing the access token
/// before it expires to ensure continuous access to the Fitbit API.
///
/// # Arguments
///
/// * `fitbit_client` - An `Arc<RwLock<FitbitClient>>` that provides access to the shared Fitbit client.
/// * `interval` - A `Duration` that specifies the interval between token refresh attempts.
pub async fn refresh_token_periodically(fitbit_client: Arc<RwLock<FitbitClient>>, interval: Duration) {
    loop {
        debug!("[refresh_token_periodically] The spawned refreshing task is sleeping for {} seconds before refreshing the access token...", interval.as_secs());
        tokio::time::sleep(interval).await;
        debug!("[refresh_token_periodically] Sleep ended. Trying to aquire write lock on fitbit_client (Arc<RwLock<FitbitClient>>");
        let mut write_locked_client = fitbit_client.write().await;
        debug!("[refresh_token_periodically] Refreshing the access token by calling refresh_access_token()...");
        match write_locked_client.refresh_access_token().await {
            Ok(_) => debug!("[refresh_token_periodically] Access token successfully refreshed."),
            Err(err) => error!("[refresh_token_periodically] Error refreshing access token: {:?}", err),
        }
    }
}