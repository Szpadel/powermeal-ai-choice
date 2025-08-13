pub mod ai;
mod api;
mod cache;
mod preferences;
pub mod prompts;
pub mod serde;

use crate::api::*;
use crate::cache::IngredientsCache;
use crate::serde::*;
use ai::AiResponse;
use chrono::{DateTime, Days, Local, NaiveDate, TimeZone};
use clap::{Parser, Subcommand};
use dialoguer::{theme::ColorfulTheme, Select};
use eyre::{Context, ContextCompat, OptionExt};
use indexmap::IndexMap;
use preferences::Preferences;
use std::{
    collections::HashMap,
    io::{self, Write},
    time::Duration,
};
use tokio::time::sleep;
use tracing_subscriber::{layer::SubscriberExt, prelude::*, util::SubscriberInitExt};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    /// Auto accept AI selections without user confirmation
    #[arg(long)]
    yolo: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Configure AI settings (API URL, key, model)
    ConfigureAi,
    /// Edit your free-text meal preferences
    EditPreferences,
}


const FETCH_HISTORY_DAYS: i64 = 14;

fn status(txt: &str) {
    clear_status();
    print!("{txt}\r");
    io::stdout().flush().unwrap();
}

fn clear_status() {
    print!("\r\x1b[2K");
    io::stdout().flush().unwrap();
}

async fn print_with_delay(message: &str, delay_ms: u64) {
    for c in message.chars() {
        print!("{c}");
        io::stdout().flush().unwrap();
        sleep(Duration::from_millis(delay_ms)).await;
    }
    println!();
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    init_tracing();
    let _cache = IngredientsCache::get_instance();

    let mut preferences = Preferences::load_preferences();

    // Migration check
    if preferences.needs_migration() {
        migrate_preferences(&mut preferences).await?;
    }

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::ConfigureAi) => {
            preferences.ai_config = Some(ai::configure_ai().await?);
            preferences.save_preferences();
            return Ok(());
        }
        Some(Commands::EditPreferences) => {
            edit_preferences_cli(&preferences)?;
            return Ok(());
        }
        None => {}
    }

    if Preferences::token().is_none() {
        print!("Session refresh token is not set.");
        update_token().await?;
    }

    if Preferences::ai_config().is_none() {
        print!("AI configuration is not set.");
        ai::configure_ai().await?;
    }

    status("Authenticating...");
    let token =
        match refresh_token(&Preferences::token().ok_or_eyre("refresh token missing")?).await {
            Ok(token) => token.token,
            Err(e) => {
                clear_status();
                eprintln!("Error: {e}");
                update_token().await?.token
            }
        };

    let diets = fetch_diets(&token).await.wrap_err("fetch diets")?;
    let days = days_available_to_select(&token, &diets).await?;

    if days.is_empty() {
        clear_status();
        println!("No days available to select menu");
        return Ok(());
    }

    for next_day in days {
        select_dishes_for_day(&token, next_day, &diets, cli.yolo, &preferences).await?;
    }

    Ok(())
}

async fn diet_for_date<'a>(
    token: &str,
    diet_list: &'a DietsList,
    date: &DateTime<Local>,
) -> eyre::Result<Option<&'a Diet>> {
    if let Some(diet) = diet_list.diet_for_date(date) {
        return Ok(Some(diet));
    }

    // In case we reschedule some days in the diet, we might get some days that are outside set delivery dates
    for diet in diet_list.members.iter() {
        status(&format!(
            "Searching for diet: Fetching calendar for diet #{}",
            diet.id
        ));
        let calendar = fetch_calendar(
            token,
            diet.id,
            // Extend the range to fetch the calendar
            if date < &diet.first_delivery_date {
                date.date_naive()
            } else {
                diet.first_delivery_date.date_naive()
            },
            if date > &diet.last_delivery_date {
                date.date_naive()
            } else {
                diet.last_delivery_date.date_naive()
            },
        )
        .await
        .wrap_err("fetch calendar")?;
        if let Some(diet_day) = calendar.days.get(&date.date_naive()) {
            if diet_day.state == DietDayState::AvailableToSelect {
                return Ok(Some(diet));
            }
        }
    }

    Ok(None)
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
                clear_status();
                eprintln!("Error: {e}");
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

    #[derive(Debug, PartialEq)]
    enum DietDayStatus {
        AvailableToSelect,
        NotBoughtDiet,
        Other,
    }
    let mut diet_day_status: HashMap<NaiveDate, DietDayStatus> = HashMap::new();

    for diet in diets.diets_in_time_range(&next_day, &end_day) {
        status(&format!("Fetching calendar for diet #{}", diet.id));
        let calendar = fetch_calendar(token, diet.id, next_day.date_naive(), end_day.date_naive())
            .await
            .wrap_err("fetching calendar")?;
        for (date, status) in calendar.days {
            if matches!(status.state, DietDayState::AvailableToSelect | DietDayState::NotDeliveredCanSelectMenu) {
                diet_day_status.insert(date, DietDayStatus::AvailableToSelect);
                days.push(Local.from_local_datetime(&date.into()).unwrap());
            } else if status.state == DietDayState::NotBoughtDiet {
                diet_day_status
                    .entry(date)
                    .or_insert(DietDayStatus::NotBoughtDiet);
            } else if matches!(
                diet_day_status.get(&date),
                None | Some(DietDayStatus::NotBoughtDiet)
            ) {
                diet_day_status.insert(date, DietDayStatus::Other);
            }
        }
    }

    for (date, status) in diet_day_status {
        if let DietDayStatus::NotBoughtDiet = status {
            clear_status();
            println!("{date}: No diet bought");
        }
    }

    days.sort_unstable();
    Ok(days)
}

async fn get_diet_with_ingredients_with_fallback_search(
    date: &DateTime<Local>,
    primary_diet_id: i64,
    all_diets: &DietsList,
    token: &str
) -> eyre::Result<CalendarDayItems> {
    let primary_diet = get_diet_with_ingredients(date, primary_diet_id, token).await?;
    if ! primary_diet.diet_elements.members.is_empty() {
        return Ok(primary_diet);
    }

    for diet_id in all_diets.members.iter().map(|d| d.id) {
        if diet_id == primary_diet_id {
            continue;
        }

        let alternative_diet = get_diet_with_ingredients(date, diet_id, token).await?;
        if ! alternative_diet.diet_elements.members.is_empty() {
            return Ok(alternative_diet);
        }
    }

    Ok(primary_diet)
}

async fn get_diet_with_ingredients(
    date: &DateTime<Local>,
    diet_id: i64,
    token: &str,
) -> eyre::Result<CalendarDayItems> {
    let mut calendar_day_items = get_diet(date, diet_id, token).await?;
    let ingredients_cache = IngredientsCache::get_instance();

    for dish_item in &mut calendar_day_items.diet_elements.members {
        for option in &mut dish_item.options {
            if option.ingredients.is_none() {
                // Try to get ingredients from cache first
                option.ingredients = ingredients_cache.get(&option.dish_size_id);

                // still nothing, fetch from api
                if option.ingredients.is_none() {
                    status(&format!(
                        "Fetching ingredients for {}",
                        option.name.as_str()
                    ));
                    let ingredients = fetch_ingredients(token, option.dish_size_id)
                        .await
                        .wrap_err("fetching ingredients")?;

                    // cache ingredients
                    ingredients_cache.put(option.dish_size_id, ingredients.clone());
                    option.ingredients = Some(ingredients);
                }
            }
        }
    }
    Ok(calendar_day_items)
}

async fn select_dishes_for_day(
    token: &str,
    date: DateTime<Local>,
    diets: &DietsList,
    yolo: bool,
    preferences: &Preferences,
) -> eyre::Result<()> {
    status("Fetching menu...");
    let diet_id = diet_for_date(token, diets, &date)
        .await
        .wrap_err_with(|| format!("find diet day for {date}"))?
        .ok_or_else(|| eyre::eyre!("no diet for date {date}"))?
        .id;
    let calendar_day_items = get_diet_with_ingredients_with_fallback_search(&date, diet_id, diets, token)
        .await
        .wrap_err("getting diet with ingredients")?;
    clear_status();
    println!("{}, {}", date.format("%Y-%m-%d"), date.format("%A"));
    println!("{}", calendar_day_items.debug_options());

    let last_days_choices = fetch_historical_orders(token, diets, &date, FETCH_HISTORY_DAYS)
        .await
        .wrap_err("fetching historical orders")?;
    cache::IngredientsCache::get_instance()
        .save()
        .wrap_err("saving ingredients cache")?;
    if calendar_day_items.diet_elements.members.is_empty() {
        return Err(eyre::eyre!("No diet elements available for date {}", date.format("%Y-%m-%d")));
    }

    status("Ai is thinking...");
    let result = ai::select_dish(
        date.date_naive(),
        &calendar_day_items.diet_elements.members,
        &last_days_choices,
        &preferences.user_preferences,
    )
    .await
    .wrap_err("selecting dish with ai")?;
    clear_status();
    println!();

    for reason in &result.reasoning {
        print_with_delay(&format!(" 𝔞𝔦 {reason}"), 1).await;
    }

    let mut menu_changes = ChangeMenuRequest::default();
    select_dishes(
        &calendar_day_items,
        &date.date_naive(),
        result,
        &mut menu_changes,
        yolo,
    )
    .await
    .wrap_err("while asking user")?;

    if !menu_changes.items.is_empty() {
        confirm_menu_change(
            token,
            &date.date_naive(),
            diet_id,
            &menu_changes,
            &calendar_day_items,
            yolo,
        )
        .await
        .wrap_err("confirm menu change")?;
    }
    Preferences::set_next_day_to_check(date.date_naive().checked_add_days(Days::new(1)).unwrap());
    Ok(())
}


async fn confirm_menu_change(
    token: &str,
    date: &NaiveDate,
    diet_id: i64,
    menu_changes: &ChangeMenuRequest,
    calendar_day_items: &CalendarDayItems,
    yolo: bool,
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
            "  \x1b[31m{current_name}\x1b[0m -> \x1b[32m{new_name}\x1b[0m"
        );
    }
    let should_save = yolo || dialoguer::Confirm::new()
        .with_prompt("Save menu changes?")
        .interact()?;

    if should_save {
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
        if let Some(diet) = diet_for_date(token, diets, &date)
            .await
            .wrap_err_with(|| format!("find diet day for {date}"))?
        {
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
    _date: &NaiveDate,
    ai_result: AiResponse,
    menu_changes: &mut ChangeMenuRequest,
    yolo: bool,
) -> eyre::Result<()> {
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

        let selection = if yolo {
            println!(" [auto-selected by AI]");
            ai_selected
        } else {
            Select::with_theme(&ColorfulTheme::default())
                .with_prompt(dish_item.meal_type.name.to_string())
                .items(
                    &dish_item
                        .options()
                        .iter()
                        .map(|x| x.name.as_str())
                        .collect::<Vec<_>>(),
                )
                .default(ai_selected)
                .interact()?
        };

        if selection != ai_selected {
            println!("\nYour choice differs from the AI's suggestion.");
            println!("To make this change permanent for future selections, run: powermeal edit-preferences");
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
    Ok(())
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
        println!("{name} [id={dish}] : {count}");
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
async fn migrate_preferences(preferences: &mut Preferences) -> eyre::Result<()> {
    println!("\nDetected legacy structured preferences. Migrating to free-text format...");

    // Only configure AI if needed for migration
    if preferences.ai_config.is_none() {
        println!("AI configuration needed for migration. Setting up...");
        preferences.ai_config = Some(ai::configure_ai().await?);
        preferences.save_preferences();
    }

    println!("Analyzing your previous choices to create preference description...");
    let ai_cfg = preferences.ai_config.as_ref().unwrap();
    let draft = ai::ai_generate_preferences(&preferences.adjustments, ai_cfg).await?;

    println!("Opening your editor so you can review and customize the preferences...");
    let header = "# AI-Generated Preference Draft\n\
                 # Please review, edit, and save:\n\
                 # - Add any preferences not captured by the AI\n\
                 # - Remove anything inaccurate\n\
                 # - Save the file to complete migration\n\n";

    let edited = edit_in_editor(&format!("{header}{draft}"))?;

    if edited.trim().is_empty() {
        eyre::bail!("Migration cancelled - file saved empty. Your legacy preferences remain intact.");
    }

    preferences.complete_migration(edited);
    println!("Preferences migrated successfully!");
    println!("You can review or modify them at any time with: powermeal edit-preferences");
    Ok(())
}

fn edit_in_editor(initial: &str) -> eyre::Result<String> {
    let tmp = tempfile::NamedTempFile::new()?;
    std::fs::write(tmp.path(), initial)?;

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".into());
    println!("Launching editor: {editor} (set EDITOR environment variable to change)");

    let status = std::process::Command::new(&editor)
        .arg(tmp.path())
        .status()
        .wrap_err_with(|| format!("Failed to launch editor: {editor}"))?;

    if !status.success() {
        return Err(eyre::eyre!("Editor exited with error status"));
    }

    Ok(std::fs::read_to_string(tmp.path())?)
}

fn edit_preferences_cli(preferences: &Preferences) -> eyre::Result<()> {
    let existing = preferences.user_preferences.clone();
    let is_new = existing.trim().is_empty();

    if is_new {
        println!("You don't have any saved preferences yet.");
        println!("Describe your dietary requirements, portion preferences, allergens, and dietary goals.");
        println!("Example: 'I'm vegetarian, avoid dairy, love spicy food, need ~2000 kcal/day'");

        if !dialoguer::Confirm::new()
            .with_prompt("Open editor to create preferences?")
            .interact()?
        {
            return Ok(());
        }
    }

    let content = if is_new {
        "# Your Meal Preferences\n\
         # Describe your dietary requirements below. The AI will use this to select meals.\n\
         # Examples:\n\
         #   → I avoid pork for religious reasons\n\
         #   → Need high-protein breakfasts\n\
         #   → Allergic to shellfish\n\n"
    } else {
        &existing
    };

    let edited = edit_in_editor(content)?;
    let trimmed = edited.trim().to_string();

    if trimmed.is_empty() {
        println!("Editor saved empty content. No changes made.");
        return Ok(());
    }

    if trimmed != existing {
        if dialoguer::Confirm::new()
            .with_prompt("Save the changes to your preferences?")
            .interact()?
        {
            let mut prefs = Preferences::load_preferences();
            prefs.user_preferences = trimmed;
            prefs.save_preferences();
            println!("Preferences updated successfully.");
        }
    } else {
        println!("No changes detected in the edited preferences.");
    }

    Ok(())
}
