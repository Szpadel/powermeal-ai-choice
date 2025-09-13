//! PowerMeal API client module
//!
//! This module provides a comprehensive API client for interacting with the PowerMeal service.
//! All API functions include automatic retry logic for rate limiting (429) and server errors (5xx),
//! ensuring robust communication with the PowerMeal backend.
//!
//! # Features
//!
//! - Automatic token refresh using refresh tokens
//! - Diet management and retrieval
//! - Calendar operations for meal planning
//! - Menu customization and changes
//! - Ingredient information fetching
//!
//! # Retry Logic
//!
//! All API calls automatically retry on:
//! - HTTP 429 (Too Many Requests) - respects Retry-After header
//! - HTTP 5xx (Server Errors) - uses exponential backoff

use crate::{Calendar, CalendarDayItems, ChangeMenuRequest, DietsList, DishIngredients, DishSizeIngredients, RefreshTokenResponse};
use chrono::{DateTime, Local, NaiveDate};
use eyre::{Context, Ok};

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

/// Exchanges a refresh token for a new access token.
///
/// This function is used to obtain a new JWT access token when the current one expires.
/// It does not require authentication as it uses the refresh token itself for validation.
///
/// # Arguments
///
/// * `refresh_token` - A valid refresh token obtained from a previous authentication
///
/// # Returns
///
/// Returns a `RefreshTokenResponse` containing:
/// - New access token (JWT)
/// - Token expiration information
/// - Potentially a new refresh token
///
/// # Errors
///
/// This function will return an error if:
/// - The HTTP request fails
/// - The refresh token is invalid or expired
/// - The response cannot be parsed as valid JSON
/// - The PowerMeal API is unavailable
///
/// # Example
///
/// ```no_run
/// # async fn example() -> eyre::Result<()> {
/// let refresh_token = "your_refresh_token_here";
/// let response = api::refresh_token(refresh_token).await?;
/// println!("New access token: {}", response.access_token);
/// # Ok(())
/// # }
/// ```
pub async fn refresh_token(refresh_token: &str) -> eyre::Result<RefreshTokenResponse> {
    let url = "https://api.powermeal.pl/refresh_token";
    let body = format!("{{\"refreshToken\":\"{refresh_token}\"}}");
    let data = send_request(url, "", reqwest::Method::PUT, Some(body)).await?;
    let refresh_token_response: RefreshTokenResponse = serde_json::from_str(&data)
        .wrap_err_with(|| format!("while getting JWT token\nJSON: {data:?}"))?;
    Ok(refresh_token_response)
}

/// Fetches menu items for a specific diet on a given date.
///
/// Retrieves the complete meal plan for a particular diet subscription on the specified date,
/// including all meals, dishes, and their details.
///
/// # Arguments
///
/// * `date` - The date for which to fetch the menu items
/// * `diet_id` - The unique identifier of the diet subscription
/// * `token` - A valid JWT access token for authentication
///
/// # Returns
///
/// Returns a `CalendarDayItems` structure containing:
/// - All meals for the specified date
/// - Dish information including names, descriptions, and nutritional data
/// - Portion sizes and customization options
/// - Delivery and preparation details
///
/// # Errors
///
/// This function will return an error if:
/// - The authentication token is invalid or expired
/// - The diet_id does not exist or user lacks access
/// - The date is outside the valid subscription period
/// - The response cannot be parsed as valid JSON
/// - Network or server errors occur
///
/// # Example
///
/// ```no_run
/// # use chrono::{DateTime, Local};
/// # async fn example() -> eyre::Result<()> {
/// let date = Local::now();
/// let diet_id = 12345;
/// let token = "your_jwt_token";
/// let menu = api::get_diet(&date, diet_id, token).await?;
/// println!("Found {} items for the day", menu.items.len());
/// # Ok(())
/// # }
/// ```
pub async fn get_diet(
    date: &DateTime<Local>,
    diet_id: i64,
    token: &str,
) -> eyre::Result<CalendarDayItems> {
    let url = format!(
        "https://api.powermeal.pl/v2/frontend/secure/calendar/{diet_id}/days/{}/items",
        date.format("%Y-%m-%d"),
    );
    let data = send_request(&url, token, reqwest::Method::GET, None).await?;
    let calendar_day_items: CalendarDayItems = serde_json::from_str(&data)
        .wrap_err_with(|| format!("while parsing json\nJson: {data:?}"))?;
    Ok(calendar_day_items)
}

/// Retrieves all active diet subscriptions for the authenticated user.
///
/// Fetches a comprehensive list of all diet plans the user has subscribed to,
/// including both active and scheduled subscriptions.
///
/// # Arguments
///
/// * `token` - A valid JWT access token for authentication
///
/// # Returns
///
/// Returns a `DietsList` containing:
/// - List of all user's diet subscriptions
/// - Diet IDs for API operations
/// - Subscription status and validity periods
/// - Diet configuration and preferences
///
/// # Errors
///
/// This function will return an error if:
/// - The authentication token is invalid or expired
/// - The user has no diet subscriptions
/// - The response cannot be parsed as valid JSON
/// - Network or server errors occur
///
/// # Example
///
/// ```no_run
/// # async fn example() -> eyre::Result<()> {
/// let token = "your_jwt_token";
/// let diets = api::fetch_diets(token).await?;
/// for diet in &diets.items {
///     println!("Diet ID: {}, Status: {}", diet.id, diet.status);
/// }
/// # Ok(())
/// # }
/// ```
pub async fn fetch_diets(token: &str) -> eyre::Result<DietsList> {
    let url = "https://api.powermeal.pl/frontend/secure/my-diets?pagination=false";
    let data = send_request(url, token, reqwest::Method::GET, None).await?;
    let diets: DietsList = serde_json::from_str(&data)
        .wrap_err_with(|| format!("while parsing ordered diets\nJson: {data:?}",))?;
    Ok(diets)
}

/// Fetches calendar status for a diet within a specified date range.
///
/// Retrieves calendar information showing which days have meals configured,
/// delivery status, and other scheduling information for the specified period.
///
/// # Arguments
///
/// * `token` - A valid JWT access token for authentication
/// * `diet_id` - The unique identifier of the diet subscription
/// * `from` - The start date of the range (inclusive)
/// * `to` - The end date of the range (inclusive)
///
/// # Returns
///
/// Returns a `Calendar` structure containing:
/// - Day-by-day status for the entire date range
/// - Delivery schedules and cutoff times
/// - Menu availability and lock status
/// - Special events or modifications
///
/// # Errors
///
/// This function will return an error if:
/// - The authentication token is invalid or expired
/// - The diet_id does not exist or user lacks access
/// - The date range is invalid (from > to)
/// - The date range exceeds API limits
/// - The response cannot be parsed as valid JSON
/// - Network or server errors occur
///
/// # Example
///
/// ```no_run
/// # use chrono::NaiveDate;
/// # async fn example() -> eyre::Result<()> {
/// let token = "your_jwt_token";
/// let diet_id = 12345;
/// let from = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
/// let to = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
/// let calendar = api::fetch_calendar(token, diet_id, from, to).await?;
/// println!("Calendar covers {} days", calendar.days.len());
/// # Ok(())
/// # }
/// ```
pub async fn fetch_calendar(
    token: &str,
    diet_id: i64,
    from: NaiveDate,
    to: NaiveDate,
) -> eyre::Result<Calendar> {
    let url = format!("https://api.powermeal.pl/frontend/secure/calendar/{diet_id}/{from}/{to}");
    let data = send_request(&url, token, reqwest::Method::GET, None).await?;
    let calendar_day_items: Calendar = serde_json::from_str(&data)
        .wrap_err_with(|| format!("while parsing json\nJson: {data:?}"))?;
    Ok(calendar_day_items)
}

/// Saves menu selections and modifications for a specific date.
///
/// Updates the meal plan for a given date, allowing users to customize their menu
/// by selecting different dishes, adjusting portions, or making other modifications
/// within the constraints of their diet plan.
///
/// # Arguments
///
/// * `token` - A valid JWT access token for authentication
/// * `date` - The date for which to modify the menu
/// * `diet_id` - The unique identifier of the diet subscription
/// * `change` - A `ChangeMenuRequest` containing the desired menu modifications
///
/// # Returns
///
/// Returns `Ok(())` if the menu change was successfully saved.
///
/// # Errors
///
/// This function will return an error if:
/// - The authentication token is invalid or expired
/// - The diet_id does not exist or user lacks access
/// - The date is locked for modifications (too close to delivery)
/// - The requested changes violate diet constraints
/// - The selected dishes are unavailable
/// - Network or server errors occur
///
/// # Example
///
/// ```no_run
/// # use chrono::NaiveDate;
/// # async fn example() -> eyre::Result<()> {
/// let token = "your_jwt_token";
/// let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
/// let diet_id = 12345;
/// let change_request = ChangeMenuRequest {
///     // ... menu modifications
/// };
/// api::change_menu(token, &date, diet_id, &change_request).await?;
/// println!("Menu successfully updated");
/// # Ok(())
/// # }
/// ```
pub async fn change_menu(
    token: &str,
    date: &NaiveDate,
    diet_id: i64,
    change: &ChangeMenuRequest,
) -> eyre::Result<()> {
    let url = format!(
        "https://api.powermeal.pl/v2/frontend/secure/calendar/{diet_id}/days/{date}/change-menu",
    );
    let body = serde_json::to_string(change).wrap_err("while serializing items")?;
    send_request(&url, token, reqwest::Method::PUT, Some(body)).await?;
    Ok(())
}

/// Fetches detailed ingredient information for a specific dish size.
///
/// Retrieves comprehensive ingredient data including names, quantities, allergens,
/// and nutritional information for a particular portion size of a dish.
///
/// # Arguments
///
/// * `token` - A valid JWT access token for authentication
/// * `dish_size_id` - The unique identifier of the dish size/portion
///
/// # Returns
///
/// Returns a `DishSizeIngredients` structure containing:
/// - Complete ingredient list with quantities
/// - Allergen information and dietary markers
/// - Nutritional breakdown per ingredient
/// - Preparation and sourcing details
///
/// # Errors
///
/// This function will return an error if:
/// - The authentication token is invalid or expired
/// - The dish_size_id does not exist
/// - The API returns multiple results (expects exactly one)
/// - The response cannot be parsed as valid JSON
/// - Network or server errors occur
///
/// # Validation
///
/// This function includes built-in validation to ensure exactly one result is returned.
/// If the API returns zero or multiple dish size ingredients, an error is returned.
///
/// # Example
///
/// ```no_run
/// # async fn example() -> eyre::Result<()> {
/// let token = "your_jwt_token";
/// let dish_size_id = 98765;
/// let ingredients = api::fetch_ingredients(token, dish_size_id).await?;
/// println!("Dish contains {} ingredients", ingredients.ingredients.len());
/// for ingredient in &ingredients.ingredients {
///     println!("- {}: {}g", ingredient.name, ingredient.quantity);
/// }
/// # Ok(())
/// # }
/// ```
pub async fn fetch_ingredients(
    token: &str,
    dish_size_id: i64,
) -> eyre::Result<DishSizeIngredients> {
    let url = format!(
        "https://api.powermeal.pl/v2/frontend/ingredients_by_dish_sizes/list?dishSizeIds[]={dish_size_id}",
    );
    let data = send_request(&url, token, reqwest::Method::GET, None).await?;
    let ingredients: DishIngredients = serde_json::from_str(&data)
        .wrap_err_with(|| format!("while parsing ingredients\nJson: {data:?}",))?;

    if ingredients.members.len() != 1 {
        eyre::bail!(
            "Expected one dish size ingredients, got {}",
            ingredients.members.len()
        );
    }
    Ok(ingredients.members.into_iter().next().unwrap())
}
