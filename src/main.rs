pub mod ai;
mod api;
mod availability;
mod preferences;
pub mod prompts;
pub mod serde;

use ai::select_dish;
use api::{
    extract_brand_id, fetch_client_diets, fetch_delivery_config, fetch_diet_details, fetch_menu,
    update_dish_selection, MenuFetchParams,
};
use availability::is_menu_selection_available;
use chrono::{Local, NaiveDate};
use clap::{Parser, Subcommand};
use dialoguer::{theme::ColorfulTheme, Confirm, Select};
use eyre::{eyre, Context};
use preferences::Preferences;
use serde::{ClientDiet, DeliveryConfig, DishUpdateRequest, ExistingDish, MenuDish};
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

    let mut preferences = Preferences::load_preferences();

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

    // Get token or prompt for one
    let token = match Preferences::token() {
        Some(t) => t,
        None => {
            let new_token = prompt_for_token().await?;
            validate_and_save_token(&new_token).await?;
            new_token
        }
    };

    if Preferences::ai_config().is_none() {
        print!("AI configuration is not set.");
        ai::configure_ai().await?;
    }

    status("Extracting brand information...");
    let brand_id = extract_brand_id(&token).wrap_err("Failed to extract brand_id from token")?;

    status("Fetching active diets...");
    let client_diets = fetch_client_diets(&token, brand_id)
        .await
        .wrap_err("Failed to fetch client diets")?;

    if client_diets.data.diets.is_empty() {
        clear_status();
        println!("No active diets found.");
        return Ok(());
    }

    status("Fetching delivery configuration...");
    let delivery_config = fetch_delivery_config(&token, brand_id)
        .await
        .wrap_err("Failed to fetch delivery configuration")?;

    // Get the resume date from preferences (where we left off)
    let resume_from = Preferences::next_day_to_check().map(|dt| dt.date_naive());

    // Find available days starting from resume date
    let available_days = find_available_days(
        &token,
        &client_diets.data.diets,
        &delivery_config,
        resume_from,
    )
    .await?;

    if available_days.is_empty() {
        clear_status();
        println!("No days available for menu selection.");
        return Ok(());
    }

    clear_status();
    println!(
        "Found {} days available for menu selection.",
        available_days.len()
    );

    // Process each available day
    for day in available_days {
        process_day_selection(&token, &day, &preferences, cli.yolo, brand_id).await?;
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

        if !Confirm::new()
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
        if Confirm::new()
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

/// Prompts the user to enter their authentication token with instructions
async fn prompt_for_token() -> eyre::Result<String> {
    println!("\nAuthentication token is required to use PowerMeal API.");
    println!("\nTo get your token:");
    println!("1. Log in to https://zamowienie.powermeal.pl/");
    println!("2. Open browser DevTools (F12)");
    println!("3. Go to Application/Storage tab");
    println!("4. Find Local Storage → https://zamowienie.powermeal.pl");
    println!("5. Copy the value of 'access_token' (without quotes)");
    println!();

    let token = dialoguer::Input::<String>::new()
        .with_prompt("Enter your authentication token")
        .interact()?;

    Ok(token.trim().to_string())
}

/// Validates a token by making a test API call and saves it if valid
async fn validate_and_save_token(token: &str) -> eyre::Result<()> {
    status("Validating token...");

    // Try to extract brand_id and make a test API call
    let brand_id = extract_brand_id(token).wrap_err("Invalid token format")?;

    // Try fetching diets as validation
    fetch_client_diets(token, brand_id)
        .await
        .wrap_err("Token validation failed")?;

    clear_status();
    println!("✓ Token validated successfully");

    // Save the validated token
    Preferences::save_token(token);
    Ok(())
}

// Data structures for available days
#[derive(Debug, Clone)]
struct AvailableDay {
    date: String,
    client_diet_id: i64,
    client_diet_item_id: i64,
    diet_id: i64,
    var_id: i64,
    var_cal_id: i64,
    existing_dishes: Vec<ExistingDish>, // Track existing dish selections
}

// Find all available days across all client diets
async fn find_available_days(
    token: &str,
    diets: &[ClientDiet],
    delivery_config: &DeliveryConfig,
    start_from: Option<NaiveDate>,
) -> eyre::Result<Vec<AvailableDay>> {
    let mut available_days = Vec::new();
    let today = Local::now().date_naive();
    // Use the later of today or the provided start date
    let start_date = start_from.map(|d| d.max(today)).unwrap_or(today);

    for diet in diets {
        // Fetch diet details to get all days
        let diet_details = fetch_diet_details(token, diet.id)
            .await
            .wrap_err(format!("Failed to fetch details for diet {}", diet.id))?;

        // Check each day in the diet
        for item in diet_details.data.items {
            // Parse the delivery date
            let delivery_date = NaiveDate::parse_from_str(&item.date_dlv, "%Y-%m-%d")
                .wrap_err(format!("Failed to parse date: {}", item.date_dlv))?;

            // Skip dates before our start date
            if delivery_date <= start_date {
                continue;
            }

            // Check if menu selection is available for this day
            match is_menu_selection_available(&item, delivery_config) {
                Ok(true) => {
                    // Create an AvailableDay entry
                    available_days.push(AvailableDay {
                        date: item.date_dlv.clone(),
                        client_diet_id: diet.id,
                        client_diet_item_id: item.id,
                        diet_id: diet.diet_id,
                        var_id: diet.var_id,
                        var_cal_id: diet.var_cal_id,
                        existing_dishes: item.dishes.clone().unwrap_or_default(),
                    });
                }
                Ok(false) => {
                    // Cutoff has passed, skip this day
                    tracing::debug!("Cutoff passed for {}", item.date_dlv);
                }
                Err(e) => {
                    // Log error but continue processing other days
                    tracing::warn!("Error checking availability for {}: {}", item.date_dlv, e);
                }
            }
        }
    }

    // Sort by date for consistent ordering
    available_days.sort_by(|a, b| a.date.cmp(&b.date));

    Ok(available_days)
}

// Group dishes by meal id (unique per meal slot regardless of sequence name)
fn group_dishes_by_meal(dishes: &[MenuDish]) -> HashMap<i32, Vec<MenuDish>> {
    let mut grouped: HashMap<i32, Vec<MenuDish>> = HashMap::new();

    for dish in dishes {
        grouped.entry(dish.meal_id).or_default().push(dish.clone());
    }

    // Sort dishes within each meal group by name for consistent ordering
    for dishes in grouped.values_mut() {
        dishes.sort_by(|a, b| a.dish_name.cmp(&b.dish_name));
    }

    grouped
}

// Submit menu updates sequentially for each meal slot (identified by meal_id)
async fn submit_menu_updates(
    token: &str,
    selections: &[(i32, MenuDish)], // (meal_id, selected_dish)
    day: &AvailableDay,
    brand_id: i32,
) -> eyre::Result<()> {
    status("Saving menu changes...");

    for (meal_id, dish) in selections {
        // Create update request
        let var_cal_meal_id = dish.var_cal_meal_id.unwrap_or_else(|| {
            eprintln!(
                "Warning: var_cal_meal_id missing for dish {}",
                dish.dish_name
            );
            0
        });

        let update_request = DishUpdateRequest {
            brand_id,
            client_diet_item_id: day.client_diet_item_id,
            dish_id: dish.dish_id,
            diet_id: day.diet_id,
            var_cal_meal_id,
        };

        // Check if there's an existing dish for this meal slot
        let existing_dish_id = day
            .existing_dishes
            .iter()
            .find(|d| d.meal_id == *meal_id)
            .or_else(|| {
                day.existing_dishes
                    .iter()
                    .find(|d| d.var_cal_meal_id == var_cal_meal_id as i32)
            })
            .map(|d| d.id);

        // Submit the update (PATCH if existing, POST if new)
        update_dish_selection(token, &update_request, existing_dish_id)
            .await
            .wrap_err(format!(
                "Failed to update dish selection for {} on {}",
                dish.meal_name, day.date
            ))?;

        // Small delay between updates to avoid overwhelming the API
        sleep(Duration::from_millis(100)).await;
    }

    clear_status();
    println!("✓ Menu changes saved successfully for {}", day.date);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_dish(meal_id: i32, meal_seq: i32, dish_id: i64, name: &str) -> MenuDish {
        MenuDish {
            dish_id,
            dish_name: name.to_string(),
            meal_name: format!("Meal {meal_id}"),
            meal_id,
            meal_seq,
            var_cal_meal_id: Some(dish_id),
            dish_ing_names: None,
            dmenu: "2025-01-01".to_string(),
        }
    }

    #[test]
    fn groups_dishes_by_meal_id() {
        let dishes = vec![
            make_dish(101, 1, 1, "B option"),
            make_dish(101, 1, 2, "A option"),
            make_dish(202, 1, 3, "Snack"),
        ];

        let grouped = group_dishes_by_meal(&dishes);

        assert_eq!(
            grouped.len(),
            2,
            "Expected two distinct meal groups keyed by meal_id"
        );

        let breakfast = grouped.get(&101).expect("missing breakfast group");
        assert_eq!(breakfast.len(), 2);
        assert_eq!(breakfast[0].dish_name, "A option");
        assert_eq!(breakfast[1].dish_name, "B option");

        let snack = grouped.get(&202).expect("missing snack group");
        assert_eq!(snack.len(), 1);
        assert_eq!(snack[0].dish_id, 3);
    }
}

// Fetch meal history for the past N days
async fn fetch_meal_history(
    token: &str,
    brand_id: i32,
    client_diet_id: i64,
    diet_id: i64,
    var_id: i64,
    var_cal_id: i64,
    days: i32,
) -> eyre::Result<Vec<MenuDish>> {
    use chrono::{Duration, Local};

    let mut history = Vec::new();
    let today = Local::now().date_naive();

    // Fetch history for each past day
    for i in 1..=days {
        let date = today - Duration::days(i.into());
        let date_str = date.format("%Y-%m-%d").to_string();

        // Try to fetch menu for this date
        status(&format!("Fetching history for {}...", date_str));
        match fetch_menu(
            token,
            MenuFetchParams {
                diet_id,
                var_id,
                var_cal_id,
                date: date_str.clone(),
                menu_type: "client".to_string(), // Get what was actually selected
                brand_id,
                client_diet_id,
            },
        )
        .await
        {
            Ok(menu) => {
                // Add all dishes from this day to history
                history.extend(menu.data);
            }
            Err(e) => {
                // Log the error but continue with other days
                tracing::debug!("Could not fetch history for {}: {}", date_str, e);
                continue;
            }
        }
    }

    Ok(history)
}

async fn process_day_selection(
    token: &str,
    day: &AvailableDay,
    preferences: &Preferences,
    yolo: bool,
    brand_id: i32,
) -> eyre::Result<()> {
    let date = NaiveDate::parse_from_str(&day.date, "%Y-%m-%d").wrap_err("Failed to parse date")?;

    // Fetch current menu selections first
    status("Fetching menu...");
    let current_menu = fetch_menu(
        token,
        MenuFetchParams {
            diet_id: day.diet_id,
            var_id: day.var_id,
            var_cal_id: day.var_cal_id,
            date: day.date.clone(),
            menu_type: "client".to_string(),
            brand_id,
            client_diet_id: day.client_diet_id,
        },
    )
    .await
    .wrap_err("Failed to fetch current menu selections")?;

    // Fetch all available menu options
    let all_menu = fetch_menu(
        token,
        MenuFetchParams {
            diet_id: day.diet_id,
            var_id: day.var_id,
            var_cal_id: day.var_cal_id,
            date: day.date.clone(),
            menu_type: "all".to_string(),
            brand_id,
            client_diet_id: day.client_diet_id,
        },
    )
    .await
    .wrap_err("Failed to fetch available menu options")?;

    clear_status();

    // Display date and current menu (matching original debug_options format)
    println!("{}, {}", day.date, date.format("%A"));

    // Group dishes by meal
    let available_by_meal = group_dishes_by_meal(&all_menu.data);
    let current_by_meal = group_dishes_by_meal(&current_menu.data);

    // Determine a stable ordering for meal slots using meal_seq
    let mut ordered_meal_ids: Vec<(i32, i32)> = available_by_meal
        .iter()
        .filter_map(|(meal_id, dishes)| dishes.first().map(|d| (*meal_id, d.meal_seq)))
        .collect();
    ordered_meal_ids.sort_by_key(|(_, seq)| *seq);

    // Display current menu in the original debug_options format
    for (meal_id, _) in &ordered_meal_ids {
        let available_dishes = match available_by_meal.get(meal_id) {
            Some(dishes) if !dishes.is_empty() => dishes,
            _ => continue,
        };

        let meal_name = &available_dishes[0].meal_name;
        println!("{}", meal_name);

        // Get current selection for this meal
        let current_dish_id = current_by_meal
            .get(meal_id)
            .and_then(|dishes| dishes.first())
            .map(|d| d.dish_id)
            .unwrap_or(-1);

        // Display all options with [*] for selected
        for dish in available_dishes {
            let marker = if dish.dish_id == current_dish_id {
                "*"
            } else {
                " "
            };
            println!("  [{}] {}", marker, dish.dish_name);
        }
    }

    // Check if there are any meals to select
    if available_by_meal.is_empty() {
        println!("\nNo meals available for selection on this day.");
        return Ok(());
    }

    // Fetch meal history for AI context
    let meal_history = fetch_meal_history(
        token,
        brand_id,
        day.client_diet_id,
        day.diet_id,
        day.var_id,
        day.var_cal_id,
        FETCH_HISTORY_DAYS as i32,
    )
    .await
    .wrap_err("Failed to fetch meal history")?;

    // Call AI to get recommendations
    status("AI is thinking...");
    let ai_response = select_dish(
        date,
        &available_by_meal,
        &current_by_meal,
        &meal_history,
        &preferences.user_preferences,
    )
    .await
    .wrap_err("Failed to get AI recommendations")?;

    clear_status();
    println!();

    // Display ALL AI reasoning first (matching original)
    for reason in &ai_response.reasoning {
        print_with_delay(&format!(" 𝔞𝔦 {}", reason), 1).await;
    }

    // Interactive meal selection
    let mut selections = Vec::new();
    println!();

    for (meal_id, _) in &ordered_meal_ids {
        let available_dishes = match available_by_meal.get(meal_id) {
            Some(dishes) if !dishes.is_empty() => dishes,
            _ => continue,
        };

        let meal_name = &available_dishes[0].meal_name;

        // Get current selection for this meal
        let current_dish = current_by_meal
            .get(meal_id)
            .and_then(|dishes| dishes.first());

        // Get AI recommendation for this meal
        let ai_selection = ai_response.selections.get(meal_name);

        // Find which dish the AI recommended
        let ai_recommended_idx = if let Some(ai_sel) = ai_selection {
            available_dishes
                .iter()
                .position(|d| d.dish_id.to_string() == ai_sel.dish_id)
                .unwrap_or(0)
        } else {
            // If no AI recommendation, default to current or first
            current_dish
                .and_then(|c| available_dishes.iter().position(|d| d.dish_id == c.dish_id))
                .unwrap_or(0)
        };

        // Display AI analysis for each dish option (matching original format)
        if let Some(ai_sel) = ai_selection {
            for (dish_id, analysis) in &ai_sel.analysis {
                if let Some(dish) = available_dishes
                    .iter()
                    .find(|d| d.dish_id.to_string() == *dish_id)
                {
                    print_with_delay(
                        &format!(" 𝔞𝔦 \x1b[1m{}\x1b[0m {}", dish.dish_name, analysis),
                        1,
                    )
                    .await;
                }
            }
            println!();
            print_with_delay(&format!(" 𝔞𝔦 {}", ai_sel.reason), 1).await;
        }

        // Handle selection (interactive or YOLO mode)
        let selected_idx = if yolo {
            println!(" [auto-selected by AI]");
            ai_recommended_idx
        } else {
            // Use dialoguer for interactive selection
            Select::with_theme(&ColorfulTheme::default())
                .with_prompt(meal_name.to_string())
                .items(
                    &available_dishes
                        .iter()
                        .map(|x| x.dish_name.as_str())
                        .collect::<Vec<_>>(),
                )
                .default(ai_recommended_idx)
                .interact()?
        };

        let selected_dish = &available_dishes[selected_idx];

        // Check if selection differs from AI recommendation
        if selected_idx != ai_recommended_idx {
            println!("\nYour choice differs from the AI's suggestion.");
            println!("To make this change permanent for future selections, run: powermeal edit-preferences");
        }

        // Only add to selections if it's different from current
        let current_dish_id = current_dish.map(|c| c.dish_id).unwrap_or(-1);
        if selected_dish.dish_id != current_dish_id {
            selections.push((*meal_id, selected_dish.clone()));
        }

        println!();
    }

    // Handle user confirmation and submission
    if selections.is_empty() {
        println!("\n✓ No changes needed - current selections are optimal");
        return Ok(());
    }

    // Show proposed changes with color formatting
    println!("Menu changes:");
    for (meal_id, dish) in &selections {
        // Find what this is replacing
        if let Some(current_dishes) = current_by_meal.get(meal_id) {
            if let Some(current) = current_dishes.first() {
                println!("\x1b[1m{}\x1b[0m", dish.meal_name);
                println!(
                    "  \x1b[31m{}\x1b[0m -> \x1b[32m{}\x1b[0m",
                    current.dish_name, dish.dish_name
                );
            }
        }
    }

    // Ask for confirmation unless in YOLO mode
    let should_save = yolo
        || Confirm::new()
            .with_prompt("Save menu changes?")
            .default(true)
            .interact()?;

    if should_save {
        status("Saving menu changes...");
        submit_menu_updates(token, &selections, day, brand_id).await?;
        clear_status();

        // Update last_day_selected to mark this day as processed
        let next_day = date
            .succ_opt()
            .ok_or_else(|| eyre!("Failed to get next day"))?;
        Preferences::set_next_day_to_check(next_day);
    }

    println!();

    Ok(())
}
