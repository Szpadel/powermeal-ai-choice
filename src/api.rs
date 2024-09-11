use crate::{Calendar, CalendarDayItems, ChangeMenuRequest, DietsList, DishIngredients, DishSizeIngredients, RefreshTokenResponse};
use chrono::{DateTime, Local, NaiveDate};
use eyre::{Context, Ok};

pub async fn refresh_token(refresh_token: &str) -> eyre::Result<RefreshTokenResponse> {
    let url = "https://api.powermeal.pl/refresh_token";
    let response = reqwest::Client::new()
        .put(url)
        .header("Accept", "application/json, text/plain, */*")
        .header("Origin", "https://panel.powermeal.pl")
        .header("Content-Type", "application/json")
        .body(format!("{{\"refreshToken\":\"{refresh_token}\"}}"))
        .send()
        .await;

    let data = response?.text().await?;
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
    let response = reqwest::Client::new()
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Origin", "https://panel.powermeal.pl")
        .header("Accept", "application/json, text/plain, */*")
        .send()
        .await
        .wrap_err("in diet http request")?;
    let data = response.text().await.wrap_err("while reading response")?;
    let calendar_day_items: CalendarDayItems = serde_json::from_str(&data)
        .wrap_err_with(|| format!("while parsing json\nJson: {data:?}"))?;
    Ok(calendar_day_items)
}

pub async fn fetch_diets(token: &str) -> eyre::Result<DietsList> {
    let url = "https://api.powermeal.pl/frontend/secure/my-diets?pagination=false";
    let response = reqwest::Client::new()
        .get(url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Origin", "https://panel.powermeal.pl")
        .header("Accept", "application/json, text/plain, */*")
        .send()
        .await
        .wrap_err("in diet http request")?;
    let data = response.text().await.wrap_err("while reading response")?;
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
    let response = reqwest::Client::new()
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Origin", "https://panel.powermeal.pl")
        .header("Accept", "application/json, text/plain, */*")
        .send()
        .await
        .wrap_err("in calendar http request")?;
    let data = response.text().await.wrap_err("while reading response")?;
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
    reqwest::Client::new()
        .put(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Origin", "https://panel.powermeal.pl")
        .header("Accept", "application/json, text/plain, */*")
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(change).wrap_err("while serializing items")?)
        .send()
        .await
        .wrap_err("in menu change http request")?;
    Ok(())
}

pub async fn fetch_ingredients(
    token: &str,
    dish_size_id: i64,
) -> eyre::Result<DishSizeIngredients> {
    let url = format!(
        "https://api.powermeal.pl/v2/frontend/ingredients_by_dish_sizes/list?dishSizeIds[]={dish_size_id}",
    );
    let response = reqwest::Client::new()
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Origin", "https://panel.powermeal.pl")
        .header("Accept", "application/json, text/plain, */*")
        .send()
        .await
        .wrap_err("in ingredients http request")?;
    let data = response.text().await.wrap_err("while reading response")?;
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
