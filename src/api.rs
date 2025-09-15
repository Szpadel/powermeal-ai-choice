//! PowerMeal API client module - PowerFoods API implementation
//!
//! This module provides API client functionality for the new PowerFoods API.
//! Phase 3 implementation complete with all new API functions.

use eyre::{Context, bail};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD as BASE64};
use serde_json::Value;
use std::result::Result;
use crate::serde::{
    ClientDietsResponse, ClientDietDetails, MenuResponse,
    DishUpdateRequest, DeliveryConfig
};

// Base URL for PowerFoods API
const API_BASE: &str = "https://api.powerfoods.pl/api/v1";

/// API Error types for proper retry logic
#[derive(Debug)]
enum ApiError {
    /// Server errors (5xx) - should retry
    ServerError(reqwest::StatusCode, String),
    /// Client errors (4xx) - should not retry
    ClientError(reqwest::StatusCode, String),
    /// Rate limiting (429) - handled separately with Retry-After
    RateLimited(u64),
    /// Network or other errors
    Other(eyre::Error),
}

impl From<ApiError> for eyre::Error {
    fn from(err: ApiError) -> Self {
        match err {
            ApiError::ServerError(status, body) => eyre::eyre!("Server error {}: {}", status, body),
            ApiError::ClientError(status, body) => eyre::eyre!("Client error {}: {}", status, body),
            ApiError::RateLimited(seconds) => eyre::eyre!("Rate limited, retry after {} seconds", seconds),
            ApiError::Other(e) => e,
        }
    }
}

/// Extract brand_id from JWT token payload
pub fn extract_brand_id(token: &str) -> eyre::Result<i32> {
    // JWT tokens have three parts separated by dots: header.payload.signature
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        bail!("Invalid JWT token format");
    }

    // Decode the payload (second part)
    let payload_bytes = BASE64
        .decode(parts[1])
        .wrap_err("Failed to decode JWT payload")?;

    // Parse JSON payload
    let payload_str = String::from_utf8(payload_bytes)
        .wrap_err("Invalid UTF-8 in JWT payload")?;
    let payload: Value = serde_json::from_str(&payload_str)
        .wrap_err("Failed to parse JWT payload as JSON")?;

    // Extract brand_id
    let brand_id = payload["brand_id"]
        .as_i64()
        .ok_or_else(|| eyre::eyre!("brand_id not found in JWT payload"))?;

    Ok(brand_id as i32)
}

/// Fetch active client diets
pub async fn fetch_client_diets(token: &str, brand_id: i32) -> eyre::Result<ClientDietsResponse> {
    let url = format!("{}/clientDiets?brand_id={}&type=active", API_BASE, brand_id);
    let response = send_request_with_retry(&url, token, reqwest::Method::GET, None).await
        .wrap_err("Failed to fetch client diets")?;

    let diets: ClientDietsResponse = serde_json::from_str(&response)
        .wrap_err("Failed to parse client diets response")?;

    Ok(diets)
}

/// Fetch diet details with days
pub async fn fetch_diet_details(token: &str, client_diet_id: i64) -> eyre::Result<ClientDietDetails> {
    let url = format!("{}/clientDiets/{}", API_BASE, client_diet_id);
    let response = send_request_with_retry(&url, token, reqwest::Method::GET, None).await
        .wrap_err_with(|| format!("Failed to fetch diet details for client_diet_id: {}", client_diet_id))?;

    match serde_json::from_str::<ClientDietDetails>(&response) {
        Ok(details) => Ok(details),
        Err(e) => {
            eprintln!("DEBUG: Parse error: {}", e);
            eprintln!("DEBUG: Raw response (first 500 chars): {}", &response[..response.len().min(500)]);
            Err(e).wrap_err("Failed to parse diet details response")
        }
    }
}

/// Fetch menu (all available or current selections)
pub async fn fetch_menu(
    token: &str,
    diet_id: i64,
    var_id: i64,
    var_cal_id: i64,
    date: &str,
    menu_type: &str, // "all" or "client"
    brand_id: i32,
    client_diet_id: i64,
) -> eyre::Result<MenuResponse> {
    let url = format!(
        "{}/diets/menu?diet_id={}&var_id={}&var_cal_id={}&dmenu={}&type={}&brand_id={}&client_diet_id={}",
        API_BASE, diet_id, var_id, var_cal_id, date, menu_type, brand_id, client_diet_id
    );

    let response = send_request_with_retry(&url, token, reqwest::Method::GET, None).await
        .wrap_err_with(|| format!("Failed to fetch menu for date: {}", date))?;

    match serde_json::from_str::<MenuResponse>(&response) {
        Ok(menu) => Ok(menu),
        Err(e) => {
            eprintln!("DEBUG: Menu parse error: {}", e);
            eprintln!("DEBUG: Menu response (first 500 chars): {}", &response[..response.len().min(500)]);
            Err(e).wrap_err("Failed to parse menu response")
        }
    }
}

/// Update single dish selection
/// If existing_dish_id is provided, uses PATCH to update existing dish.
/// Otherwise uses POST to create new dish selection.
pub async fn update_dish_selection(
    token: &str,
    update: &DishUpdateRequest,
    existing_dish_id: Option<i32>,
) -> eyre::Result<()> {
    let (url, method) = match existing_dish_id {
        Some(id) => {
            // Use PATCH to update existing dish
            (format!("{}/clientDiets/dish/{}", API_BASE, id), reqwest::Method::PATCH)
        }
        None => {
            // Use POST to create new dish selection
            (format!("{}/clientDiets/dish", API_BASE), reqwest::Method::POST)
        }
    };

    let body = serde_json::to_string(update)
        .wrap_err("Failed to serialize dish update request")?;

    let _response = send_request_with_retry(&url, token, method, Some(body)).await
        .wrap_err("Failed to update dish selection")?;

    Ok(())
}

/// Fetch delivery configuration
pub async fn fetch_delivery_config(token: &str, brand_id: i32) -> eyre::Result<DeliveryConfig> {
    let url = format!("{}/diets/delivery?brand_id={}", API_BASE, brand_id);
    let response = send_request_with_retry(&url, token, reqwest::Method::GET, None).await
        .wrap_err("Failed to fetch delivery configuration")?;

    match serde_json::from_str::<DeliveryConfig>(&response) {
        Ok(config) => Ok(config),
        Err(e) => {
            eprintln!("DEBUG: Failed to parse delivery config. Raw response: {}", response);
            Err(e).wrap_err("Failed to parse delivery configuration response")
        }
    }
}

/// Core request function with enhanced retry logic for 500 errors
async fn send_request_with_retry(
    url: &str,
    token: &str,
    method: reqwest::Method,
    body: Option<String>,
) -> eyre::Result<String> {
    let mut retries = 0;
    let max_retries = 10;

    loop {
        match send_request(url, token, method.clone(), body.clone()).await {
            Result::Ok(response) => return Ok(response),
            Result::Err(ApiError::ServerError(status, body)) if retries < max_retries => {
                // Exponential backoff: 2^retries seconds
                let delay = 2_u64.pow(retries);
                tracing::warn!("Server error {}: {}, retry {} of {}, waiting {} seconds",
                             status, body, retries + 1, max_retries, delay);
                tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                retries += 1;
            }
            Result::Err(e) => return Err(e.into()),
        }
    }
}

// Core request function - handles basic retry for rate limiting
async fn send_request(
    url: &str,
    token: &str,
    method: reqwest::Method,
    body: Option<String>,
) -> Result<String, ApiError> {
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

        let response = request_builder
            .send()
            .await
            .map_err(|e| ApiError::Other(e.into()))?;

        let status = response.status();

        // Handle rate limiting with automatic retry
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(10);
            tokio::time::sleep(std::time::Duration::from_secs(retry_after)).await;
            tracing::warn!("Got rate limited, retrying after {} seconds", retry_after);
            continue;
        }

        // Handle server errors (5xx) - return error for retry logic
        if status.is_server_error() {
            let body = response.text().await.unwrap_or_else(|_| "Unable to read response body".to_string());
            return Result::Err(ApiError::ServerError(status, body));
        }

        // Handle client errors (4xx) - do not retry
        if status.is_client_error() {
            let body = response.text().await.unwrap_or_else(|_| "Unable to read response body".to_string());
            return Result::Err(ApiError::ClientError(status, body));
        }

        // Success - return response body
        let data = response.text()
            .await
            .map_err(|e| ApiError::Other(e.into()))?;
        return Result::Ok(data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD as BASE64};

    // Helper to create token from JSON payload
    fn create_token(payload: serde_json::Value) -> String {
        let header = BASE64.encode(r#"{"alg":"HS256","typ":"JWT"}"#);
        let payload = BASE64.encode(payload.to_string());
        format!("{}.{}.signature", header, payload)
    }

    // Helper to create token from raw string payload
    fn create_raw_token(payload: &str) -> String {
        let header = BASE64.encode(r#"{"alg":"HS256","typ":"JWT"}"#);
        let payload = BASE64.encode(payload);
        format!("{}.{}.signature", header, payload)
    }

    #[test]
    fn test_extract_brand_id_data_driven() {
        // Test case structure
        struct TestCase {
            name: &'static str,
            token: String,
            expected: Result<i32, &'static str>,
        }

        // Define all test cases
        let test_cases = vec![
            // Valid cases
            TestCase {
                name: "valid token with brand_id 1 (provided token)",
                token: "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpZCI6MTIzNDUsImJyYW5kX2lkIjoxLCJpYXQiOjEyMzQ1NiwiZXhwIjoxNzY0OTE1MjU4fQ.kh0wz_hgl0j1dfhtRRPO_B7Oy2rnkU_UyOevD3upToQ".to_string(),
                expected: Ok(1),
            },
            TestCase {
                name: "valid token with large brand_id",
                token: create_token(json!({"brand_id": 2147483647})),
                expected: Ok(2147483647),
            },
            TestCase {
                name: "valid token with extra fields",
                token: create_token(json!({"brand_id": 42, "extra": "field", "nested": {"obj": true}})),
                expected: Ok(42),
            },
            TestCase {
                name: "valid token with zero brand_id",
                token: create_token(json!({"brand_id": 0})),
                expected: Ok(0),
            },
            TestCase {
                name: "valid token with negative brand_id",
                token: create_token(json!({"brand_id": -1})),
                expected: Ok(-1),
            },

            // Invalid format
            TestCase {
                name: "missing signature part",
                token: "header.payload".to_string(),
                expected: Err("Invalid JWT token format"),
            },
            TestCase {
                name: "too many parts",
                token: "header.payload.signature.extra".to_string(),
                expected: Err("Invalid JWT token format"),
            },
            TestCase {
                name: "empty token",
                token: "".to_string(),
                expected: Err("Invalid JWT token format"),
            },
            TestCase {
                name: "no dots in token",
                token: "notavalidtoken".to_string(),
                expected: Err("Invalid JWT token format"),
            },

            // Invalid base64
            TestCase {
                name: "invalid base64 in payload",
                token: "header.!!!invalid_base64!!!.signature".to_string(),
                expected: Err("Failed to decode JWT payload"),
            },

            // Invalid JSON
            TestCase {
                name: "payload not valid JSON",
                token: create_raw_token("not json"),
                expected: Err("Failed to parse JWT payload as JSON"),
            },
            TestCase {
                name: "payload is JSON array",
                token: create_raw_token("[1, 2, 3]"),
                expected: Err("brand_id not found in JWT payload"),
            },

            // Missing or invalid brand_id
            TestCase {
                name: "missing brand_id field",
                token: create_token(json!({"id": 12345, "other": "field"})),
                expected: Err("brand_id not found in JWT payload"),
            },
            TestCase {
                name: "brand_id is null",
                token: create_token(json!({"brand_id": null})),
                expected: Err("brand_id not found in JWT payload"),
            },
            TestCase {
                name: "brand_id is string",
                token: create_token(json!({"brand_id": "1"})),
                expected: Err("brand_id not found in JWT payload"),
            },
            TestCase {
                name: "brand_id is float",
                token: create_token(json!({"brand_id": 1.5})),
                expected: Err("brand_id not found in JWT payload"),
            },
            TestCase {
                name: "brand_id is boolean",
                token: create_token(json!({"brand_id": true})),
                expected: Err("brand_id not found in JWT payload"),
            },
            TestCase {
                name: "brand_id is object",
                token: create_token(json!({"brand_id": {"nested": 1}})),
                expected: Err("brand_id not found in JWT payload"),
            },
        ];

        // Run all test cases
        for test_case in test_cases {
            let result = extract_brand_id(&test_case.token);

            match (result, test_case.expected) {
                (Ok(actual), Ok(expected)) => {
                    assert_eq!(actual, expected,
                        "Test '{}' failed: expected {}, got {}",
                        test_case.name, expected, actual);
                }
                (Err(actual_err), Err(expected_msg)) => {
                    let actual_msg = actual_err.to_string();
                    assert!(actual_msg.contains(expected_msg),
                        "Test '{}' failed: expected error containing '{}', got '{}'",
                        test_case.name, expected_msg, actual_msg);
                }
                (Ok(actual), Err(expected_msg)) => {
                    panic!("Test '{}' failed: expected error '{}', got success with value {}",
                        test_case.name, expected_msg, actual);
                }
                (Err(actual_err), Ok(expected)) => {
                    panic!("Test '{}' failed: expected success with value {}, got error: {}",
                        test_case.name, expected, actual_err);
                }
            }
        }
    }
}