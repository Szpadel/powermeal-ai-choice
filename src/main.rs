pub mod ai;
mod api;
mod preferences;
pub mod serde;

use crate::api::*;
use crate::serde::*;
use ai::{AiResponse, UserAdjustment};
use chrono::{DateTime, Days, Local, NaiveDate, TimeZone};
use dialoguer::{theme::ColorfulTheme, Input, Select};
use eyre::{Context, ContextCompat, OptionExt};
use indexmap::IndexMap;
use preferences::Preferences;
use std::{
    io::{self, Write},
    time::Duration,
};
use tokio::time::sleep;
use tracing_subscriber::{layer::SubscriberExt, prelude::*, util::SubscriberInitExt};

fn status(txt: &str) {
    clear_status();
    print!("{}\r", txt);
    io::stdout().flush().unwrap();
}

fn clear_status() {
    print!("\r\x1b[2K");
    io::stdout().flush().unwrap();
}

async fn print_with_delay(message: &str, delay_ms: u64) {
    for c in message.chars() {
        print!("{}", c);
        io::stdout().flush().unwrap();
        sleep(Duration::from_millis(delay_ms)).await;
    }
    println!();
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    init_tracing();
    // dish_stats().await?;

    if Preferences::token().is_none() {
        print!("Session refresh token is not set.");
        update_token().await?;
    }

    status("Authenticating...");
    let token =
        match refresh_token(&Preferences::token().ok_or_eyre("refresh token missing")?).await {
            Ok(token) => token.token,
            Err(e) => {
                clear_status();
                eprintln!("Error: {}", e);
                update_token().await?.token
            }
        };

    let diets = fetch_diets(&token).await?;
    let days = days_available_to_select(&token, &diets).await?;

    if days.is_empty() {
        clear_status();
        println!("No days available to select menu");
        return Ok(());
    }

    for next_day in days {
        select_dishes_for_day(&token, next_day, &diets).await?;
    }

    Ok(())
}

async fn update_token() -> eyre::Result<RefreshTokenResponse> {
    loop {
        let token = dialoguer::Input::<String>::new()
            .with_prompt("Enter your refresh token")
            .interact()?;
        match refresh_token(&token).await {
            Ok(resp) => {
                Preferences::save_token(&token);
                return Ok(resp);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }
}

async fn days_available_to_select(
    token: &str,
    diets: &DietsList,
) -> eyre::Result<Vec<DateTime<Local>>> {
    let mut days = Vec::new();
    let next_day = preferences::Preferences::next_day_to_check().unwrap_or_else(chrono::Local::now);
    let end_day = next_day + chrono::Duration::days(14);

    for diet in diets.diets_in_time_range(&next_day, &end_day) {
        status(&format!("Fetching calendar for diet #{}", diet.id));
        let calendar =
            fetch_calendar(token, diet.id, next_day.date_naive(), end_day.date_naive()).await?;
        for (date, status) in calendar.days {
            if status.state == DietDayState::AvailableToSelect {
                days.push(Local.from_local_datetime(&date.into()).unwrap());
            }
        }
    }

    days.sort_unstable();
    Ok(days)
}

async fn get_diet_with_ingredients(
    date: &DateTime<Local>,
    diet_id: i64,
    token: &str,
) -> eyre::Result<CalendarDayItems> {
    let mut calendar_day_items = get_diet(date, diet_id, token).await?;
    for dish_item in &mut calendar_day_items.diet_elements.members {
        for option in &mut dish_item.options {
            if option.ingredients.is_none() {
                status(&format!(
                    "Fetching ingredients for {}",
                    option.name.as_str()
                ));
                let ingredients = fetch_ingredients(token, option.dish_size_id)
                    .await
                    .wrap_err("while fetching ingredients")?;
                option.ingredients = Some(ingredients);
            }
        }
    }
    Ok(calendar_day_items)
}

async fn select_dishes_for_day(
    token: &str,
    date: DateTime<Local>,
    diets: &DietsList,
) -> eyre::Result<()> {
    status("Fetching menu...");
    let diet_id = diets.diet_for_date(&date).wrap_err("no diet for date")?.id;
    let calendar_day_items = get_diet_with_ingredients(&date, diet_id, token).await?;
    clear_status();
    println!("{}, {}", date.format("%Y-%m-%d"), date.format("%A"));
    println!("{}", calendar_day_items.debug_options());
    let last_days_choices = fetch_historical_orders(token, diets, &date, 7).await?;
    status("Ai is thinking...");
    let result = ai::select_dish(
        date.date_naive(),
        &calendar_day_items.diet_elements.members,
        &last_days_choices,
    )
    .await?;
    clear_status();
    println!();

    for reason in &result.reasoning {
        print_with_delay(&format!(" 𝔞𝔦 {}", reason), 1).await;
    }

    let mut menu_changes = ChangeMenuRequest::default();
    let new_preferences = select_dishes(
        &calendar_day_items,
        &date.date_naive(),
        result,
        &mut menu_changes,
    )
    .await
    .wrap_err("while asking user")?;

    if !new_preferences.is_empty() {
        confirm_preferences_save(new_preferences).await?;
    }

    if !menu_changes.items.is_empty() {
        confirm_menu_change(
            token,
            &date.date_naive(),
            diet_id,
            &menu_changes,
            &calendar_day_items,
        )
        .await?;
    }
    Preferences::set_next_day_to_check(date.date_naive().checked_add_days(Days::new(1)).unwrap());
    Ok(())
}

async fn confirm_preferences_save(new_preferences: Vec<UserAdjustment>) -> eyre::Result<()> {
    println!("New preferences:");
    for pref in &new_preferences {
        println!(
            "  \x1b[31m{}\x1b[0m -> \x1b[32m{}\x1b[0m{}",
            pref.from,
            pref.to,
            pref.reason
                .as_ref()
                .map(|x| format!("\n  because: {}", x))
                .unwrap_or_default()
        );
    }
    if dialoguer::Confirm::new()
        .with_prompt("Add new preferences?")
        .interact()?
    {
        preferences::Preferences::add_new_preferences(new_preferences);
        println!("Preferences saved");
    }
    println!();
    Ok(())
}

async fn confirm_menu_change(
    token: &str,
    date: &NaiveDate,
    diet_id: i64,
    menu_changes: &ChangeMenuRequest,
    calendar_day_items: &CalendarDayItems,
) -> eyre::Result<()> {
    println!("Menu changes:");
    for item in &menu_changes.items {
        let dish_item = calendar_day_items
            .get_dish_item(&item.dish_item)
            .ok_or_eyre("dish item not found")?;
        let current_name = dish_item.get_selected_option().unwrap().name.clone();
        let new_name = calendar_day_items
            .get_dish(&item.dish_item, &item.dish)
            .map(|dish| dish.name.clone())
            .unwrap_or_else(|| {
                tracing::warn!("Dish not found: {}", item.dish);
                "Unknown dish".to_string()
            });
        println!("\x1b[1m{}\x1b[0m", dish_item.meal_type.name);
        println!(
            "  \x1b[31m{}\x1b[0m -> \x1b[32m{}\x1b[0m",
            current_name, new_name
        );
    }
    if dialoguer::Confirm::new()
        .with_prompt("Save menu changes?")
        .interact()?
    {
        status("Saving menu changes...");
        change_menu(token, date, diet_id, menu_changes).await?;
        clear_status();
    }
    println!();
    Ok(())
}

async fn fetch_historical_orders(
    token: &str,
    diets: &DietsList,
    date: &DateTime<Local>,
    days: i64,
) -> eyre::Result<IndexMap<String, CalendarDayItems>> {
    let mut last_days_choices = IndexMap::new();
    for day in (1..=days).rev() {
        let date = date
            .checked_sub_signed(chrono::Duration::days(day))
            .unwrap();
        status(&format!(
            "Fetching menu for {} (-{} days)",
            date.format("%Y-%m-%d"),
            day
        ));
        if let Some(diet) = diets.diet_for_date(&date) {
            let calendar_day_items = get_diet_with_ingredients(&date, diet.id, token).await?;
            last_days_choices.insert(
                if day == 1 {
                    "yesterday".to_string()
                } else {
                    format!("{day} days ago")
                },
                calendar_day_items,
            );
        } else {
            clear_status();
            println!("No diet active for {}", date.format("%Y-%m-%d"));
        }
    }
    Ok(last_days_choices)
}

async fn select_dishes(
    calendar_day_items: &CalendarDayItems,
    date: &NaiveDate,
    ai_result: AiResponse,
    menu_changes: &mut ChangeMenuRequest,
) -> eyre::Result<Vec<UserAdjustment>> {
    let mut new_preferences = Vec::new();
    println!();
    for dish_item in &calendar_day_items.diet_elements.members {
        let ai = ai_result.selections.get(&dish_item.id).unwrap();
        let ai_selected = dish_item
            .options()
            .iter()
            .position(|x| x.dish.id == ai.dish_id)
            .unwrap();

        for (dish_id, analysis) in ai.analysis.iter() {
            print_with_delay(
                &format!(
                    " 𝔞𝔦 \x1b[1m{}\x1b[0m {}",
                    dish_item
                        .get_dish(dish_id)
                        .map(|d| d.name.as_str())
                        .unwrap_or("unknown"),
                    analysis
                ),
                1,
            )
            .await;
        }
        println!();
        print_with_delay(&format!(" 𝔞𝔦 {}", ai.reason), 1).await;
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt(dish_item.meal_type.name.to_string())
            .items(
                &dish_item
                    .options()
                    .iter()
                    .map(|x| x.name.as_str())
                    .collect::<Vec<_>>(),
            )
            .default(ai_selected)
            .interact()?;

        if selection != ai_selected {
            let explaination: String = Input::new()
                .with_prompt("Why?")
                .allow_empty(true)
                .interact_text()?;
            new_preferences.push(UserAdjustment {
                from: dish_item.options()[ai_selected].name.clone(),
                to: dish_item.options()[selection].name.clone(),
                reason: if explaination.is_empty() {
                    None
                } else {
                    Some(explaination)
                },
                date: *date,
            });
        }

        let selected_option_id = dish_item
            .get_selected_option()
            .map(|x| x.dish.id.clone())
            .unwrap_or_default();

        let current = dish_item
            .options()
            .iter()
            .position(|x| x.dish.id == selected_option_id)
            .unwrap();
        if selection != current {
            menu_changes.items.push(ChangeMenuItem {
                dish: dish_item.options()[selection].dish.id.clone(),
                dish_item: dish_item.id.clone(),
            });
        }
        println!();
    }
    Ok(new_preferences)
}

async fn _dish_stats() -> eyre::Result<()> {
    let token = refresh_token(&Preferences::token().unwrap()).await?.token;
    let diets = fetch_diets(&token).await?;

    // Map to store dish counts
    let mut dish_counts = std::collections::HashMap::new();
    let mut dish_names = std::collections::HashMap::new();
    // Iterate over last 30 days
    for i in 0..30 {
        let date = chrono::Local::now()
            .checked_sub_signed(chrono::Duration::days(i))
            .unwrap();
        let diet_id = diets.diet_for_date(&date).wrap_err("no diet for date")?.id;
        let calendar_day_items = get_diet_with_ingredients(&date, diet_id, &token).await?;

        // Count dishes
        for dish in calendar_day_items
            .diet_elements
            .members
            .iter()
            .flat_map(|x| &x.options)
        {
            if !dish_counts.contains_key(&dish.dish.id) {
                dish_names.insert(dish.dish.id.clone(), dish.name.clone());
            }
            *dish_counts.entry(dish.dish.id.clone()).or_insert(0) += 1;
        }
    }

    // Print dish counts
    for (dish, count) in dish_counts {
        let name = dish_names.get(&dish).unwrap();
        println!("{} [id={}] : {}", name, dish, count);
    }

    Ok(())
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer().with_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "powermeal-ai-choice=info".into()),
            ),
        )
        .init();
}
