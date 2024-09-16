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
            .header("Authorization", format!("Bearer {}", token))
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
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(10);
            tokio::time::sleep(std::time::Duration::from_secs(retry_after)).await;
            tracing::warn!("Rate limited, retrying");
            continue;
        }
        let data = response.text().await.wrap_err("while reading response")?;
        return Ok(data);
    }
}

pub async fn refresh_token(refresh_token: &str) -> eyre::Result<RefreshTokenResponse> {
    let url = "https://api.powermeal.pl/refresh_token";
    let body = format!("{{\"refreshToken\":\"{refresh_token}\"}}");
    let data = send_request(url, "", reqwest::Method::PUT, Some(body)).await?;
    let refresh_token_response: RefreshTokenResponse = serde_json::from_str(&data)
        .wrap_err_with(|| format!("while getting JWT token\nJSON: {data:?}"))?;
    Ok(refresh_token_response)
}

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

pub async fn fetch_diets(token: &str) -> eyre::Result<DietsList> {
    let url = "https://api.powermeal.pl/frontend/secure/my-diets?pagination=false";
    let data = send_request(url, token, reqwest::Method::GET, None).await?;
    let diets: DietsList = serde_json::from_str(&data)
        .wrap_err_with(|| format!("while parsing ordered diets\nJson: {data:?}",))?;
    Ok(diets)
}

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
