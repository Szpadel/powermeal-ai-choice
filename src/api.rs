//! PowerMeal API client module - Migration to PowerFoods API in progress
//!
//! This module will provide API client functionality for the new PowerFoods API.
//! Currently in migration from old PowerMeal API to new PowerFoods API.
//!
//! # Phase 1: Old API code removed
//! # Phase 2: Will implement new API functions:
//! - extract_brand_id: Extract brand_id from JWT token
//! - fetch_client_diets: Get active client diets
//! - fetch_diet_details: Get diet details with days
//! - fetch_menu: Get menu (all available or current selections)
//! - update_dish_selection: Update single dish selection
//! - fetch_delivery_config: Get delivery configuration

use eyre::{Context, Ok};

// Core request function with retry logic - will be reused for new API
#[allow(dead_code)]
async fn send_request(
    url: &str,
    token: &str,
    method: reqwest::Method,
    body: Option<String>,
) -> eyre::Result<String> {
    loop {
        let client = reqwest::Client::new();
        let request_builder = client
            .request(method.clone(), url)
            .header("Authorization", format!("Bearer {token}"))
            .header("Origin", "https://panel.powermeal.pl")
            .header("Accept", "application/json, text/plain, */*");

        let request_builder = if let Some(body) = &body {
            request_builder
                .header("Content-Type", "application/json")
                .body(body.to_string())
        } else {
            request_builder
        };

        let response = request_builder.send().await.wrap_err("in http request")?;
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS || response.status().is_server_error() {
            let retry_after = response
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(10);
            tokio::time::sleep(std::time::Duration::from_secs(retry_after)).await;
            tracing::warn!("Got {}, retrying", response.status());
            continue;
        }
        let data = response.text().await.wrap_err("while reading response")?;
        return Ok(data);
    }
}