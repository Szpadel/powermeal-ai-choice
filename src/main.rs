pub mod ai;
mod api;
mod cache;
mod preferences;
pub mod prompts;
pub mod serde;
mod availability;

// use crate::api::*; // TODO: Phase 3 - re-enable with new API
use crate::cache::IngredientsCache;
// use crate::serde::*; // TODO: Phase 2 - re-enable with new structures
// use ai::AiResponse; // TODO: Phase 5 - re-enable for AI integration
// use chrono::{DateTime, Days, Local, NaiveDate, TimeZone}; // TODO: Phase 5 - re-enable
use clap::{Parser, Subcommand};
// use dialoguer::{theme::ColorfulTheme, Select}; // TODO: Phase 5 - re-enable for interactive selection
use eyre::Context;
// use indexmap::IndexMap; // TODO: Phase 5 - re-enable for historical orders
use preferences::Preferences;
use std::{
    // collections::HashMap, // TODO: Phase 5 - re-enable
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

    // TODO: Phase 5 - Re-enable migration when AI functions are reimplemented
    /*
    // Migration check
    if preferences.needs_migration() {
        migrate_preferences(&mut preferences).await?;
    }
    */

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

    // TODO: Phase 3 - Re-enable token update with new API
    /*
    if Preferences::token().is_none() {
        print!("Session refresh token is not set.");
        update_token().await?;
    }
    */

    if Preferences::ai_config().is_none() {
        print!("AI configuration is not set.");
        ai::configure_ai().await?;
    }

    // TODO: Phase 5 - Implement new workflow
    // The main workflow is temporarily disabled during migration to new API
    /*
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
    */

    println!("PowerMeal API migration in progress. Main functionality temporarily disabled.");
    println!("Phase 1: Old API code removal - COMPLETE");
    println!("Phase 2: New API implementation - PENDING");

    Ok(())
}

/// Finds the appropriate diet for a given date, handling both regular and rescheduled delivery days.
///
/// This function implements a two-phase search strategy to locate the correct diet for a specific date:
///
/// ## Primary Lookup
/// First attempts a direct lookup using the diet's configured delivery date range
/// (`first_delivery_date` to `last_delivery_date`). This is the fast path for normal operations.
///
/// ## Fallback Search with Calendar Fetching
/// If no diet is found in the primary lookup, performs an exhaustive search by fetching
/// each diet's calendar from the API. This handles cases where users have rescheduled
/// delivery days, potentially moving them outside the original diet period.
///
/// The fallback search intelligently extends the calendar fetch range when the target date
/// falls outside the diet's normal boundaries, ensuring rescheduled days are captured.
///
/// # Lifetime Parameters
/// * `'a` - Ensures the returned diet reference lives at least as long as the input `diet_list`.
///          This prevents dangling references and guarantees memory safety.
///
/// # Arguments
/// * `token` - Authentication token for API calls when calendar fetching is required
/// * `diet_list` - Reference to the collection of available diets to search through
/// * `date` - The target date for which to find an appropriate diet
///
/// # Returns
/// * `Ok(Some(&Diet))` - A diet was found that covers the specified date, either through
///                       normal scheduling or via a rescheduled delivery day
/// * `Ok(None)` - No diet is available for the specified date (day might be blocked,
///                unavailable, or fall outside all diet periods)
/// * `Err(eyre::Error)` - An error occurred during API communication while fetching calendars
///
/// # Side Effects
/// During fallback search:
/// * Makes API calls to fetch diet calendars (potentially one call per diet)
/// * Displays status messages to inform the user about ongoing calendar fetches
///
/// # Performance Considerations
/// The fallback search can be expensive as it may need to fetch calendars for all diets
/// sequentially. Each calendar fetch involves an API call with associated network latency.
/// In the worst case (date not found), this results in N API calls where N is the number
/// of diets in the list.
///
/// # Example
/// ```rust,no_run
/// # async fn example() -> eyre::Result<()> {
/// let token = "auth_token";
/// let diet_list = fetch_diets_list(token).await?;
/// let target_date = Local::now();
///
/// match diet_for_date(token, &diet_list, &target_date).await? {
///     Some(diet) => println!("Found diet #{} for date", diet.id),
///     None => println!("No diet available for this date"),
/// }
/// # Ok(())
/// # }
/// ```
// TODO: Phase 5 - Reimplement with new API structures
/*
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
*/

// TODO: Phase 5 - Reimplement with new API
/*
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
*/

// TODO: Phase 5 - Reimplement with new API structures
/*
/// Determines which days are available for meal selection across all user's active diets.
///
/// This is the critical gatekeeper function that drives the entire application workflow.
/// It scans the next 14-day window from the last selected day (or current time) and
/// identifies all dates where the user can select meals for at least one of their diets.
/// Without available days returned by this function, the application has no work to perform.
///
/// # Algorithm
///
/// 1. **Time Window Creation**: Establishes a 14-day planning horizon starting from the
///    last processed day (retrieved from preferences) or current time if no preference exists.
///
/// 2. **Diet Filtering**: Identifies which diets from the user's subscription list overlap
///    with the 14-day window using temporal intersection logic.
///
/// 3. **Calendar Fetching**: Makes parallel API calls to retrieve calendar data for each
///    relevant diet within the time window.
///
/// 4. **Status Processing**: For each day in each diet's calendar, applies priority logic:
///    - `AvailableToSelect` or `NotDeliveredCanSelectMenu`: Day is available for selection
///    - `NotBoughtDiet`: Day is unavailable but generates a warning to the user
///    - Other states: Day is blocked from selection
///
/// 5. **Multi-Diet Conflict Resolution**: When multiple diets cover the same day:
///    - Available status always wins (user can select if any diet allows it)
///    - NotBoughtDiet is only reported if no diet provides availability
///    - Other blocking states override NotBoughtDiet
///
/// 6. **Result Preparation**: Collects all available dates, sorts them chronologically,
///    and returns them for downstream processing.
///
/// # Parameters
///
/// * `token` - Authentication token for PowerMeal API calls. Must be valid and have
///   sufficient permissions to access diet and calendar endpoints.
///
/// * `diets` - User's active diet subscriptions list. Contains diet IDs, validity periods,
///   and metadata needed to determine which diets to query.
///
/// # Returns
///
/// Returns `Ok(Vec<DateTime<Local>>)` containing a sorted vector of dates when meal
/// selection is possible. The vector may be empty if no days are available within
/// the 14-day window. Dates are in local timezone for user convenience.
///
/// # Errors
///
/// Returns `Err` if:
/// - API authentication fails (invalid or expired token)
/// - Network errors occur during calendar fetching
/// - API returns malformed or unexpected data
/// - Calendar parsing fails
///
/// Error messages are wrapped with context about which operation failed.
///
/// # Side Effects
///
/// - **API Calls**: Makes one API request per active diet (potentially multiple concurrent requests)
/// - **Console Output**: Displays status messages during calendar fetching
/// - **Warning Messages**: Prints warnings for days marked as "NotBoughtDiet" to inform
///   the user about subscription gaps
///
/// # Example
///
/// ```rust
/// let token = "user_auth_token";
/// let diets = fetch_user_diets(&token).await?;
/// 
/// match days_available_to_select(&token, &diets).await {
///     Ok(available_days) if available_days.is_empty() => {
///         println!("No days available for meal selection in the next 14 days");
///     }
///     Ok(available_days) => {
///         println!("Found {} days available for selection", available_days.len());
///         for day in available_days {
///             // Process each available day
///             process_day_selection(day).await?;
///         }
///     }
///     Err(e) => {
///         eprintln!("Failed to fetch available days: {}", e);
///     }
/// }
/// ```
///
/// # Performance Considerations
///
/// - Calendar fetching is the primary bottleneck (network I/O bound)
/// - Multiple diets result in multiple API calls (consider rate limiting)
/// - 14-day window balances planning horizon with API response size
///
/// # Edge Cases
///
/// - **No Active Diets**: Returns empty vector without making API calls
/// - **All Days Blocked**: Returns empty vector, application will exit gracefully
/// - **Overlapping Diets**: Correctly handles multiple diets covering same dates
/// - **Time Zone Handling**: All dates converted to local timezone for consistency
/// - **Subscription Gaps**: Detected and reported but don't block other days
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
*/

// TODO: Phase 5 - Reimplement with new API structures
/*
/// Fetches diet information with fallback to alternative diets when the primary diet has no meals.
///
/// This function implements a failover mechanism for diet retrieval. It first attempts to get
/// meal options from the user's primary diet for the specified date. If the primary diet has
/// no available meals (which can happen on certain days or during transitions between diet
/// periods), it automatically searches through all other available diets to find one with
/// meal options. This ensures the user always has meals to choose from when possible.
///
/// The function returns both the diet calendar items (with meal options) and the actual diet
/// object that was used, which may differ from the requested primary diet if a fallback was used.
///
/// # Arguments
///
/// * `date` - The specific date and time for which to retrieve diet information.
///           Used to determine the correct meal schedule for that day.
///
/// * `primary_diet_id` - The ID of the user's primary diet subscription to try first.
///                       This is typically the diet the user has selected for the date range.
///
/// * `all_diets` - Complete list of all available diet subscriptions for the user.
///                 Used to iterate through alternative diets if the primary has no meals.
///
/// * `token` - Bearer authentication token for API requests to the meal provider service.
///
/// # Returns
///
/// Returns a tuple containing:
/// - `CalendarDayItems`: The diet information including all available meal options and ingredients
/// - `Diet`: The diet object that was actually used (may differ from primary if fallback occurred)
///
/// # Errors
///
/// This function will return an error if:
/// * The primary diet ID doesn't exist in the provided diet list
/// * Network failures occur during API calls to fetch diet information
/// * Authentication token is invalid or expired
/// * API returns malformed data that cannot be deserialized
///
/// # Side Effects
///
/// * Makes multiple API calls to fetch diet and ingredient information
/// * Updates the global ingredients cache with fetched data
/// * May perform multiple diet lookups if fallback search is triggered
///
/// # Examples
///
/// ```no_run
/// # async fn example() -> eyre::Result<()> {
/// # use chrono::Local;
/// # let token = "auth_token";
/// # let all_diets = todo!();
/// let now = Local::now();
/// let primary_diet_id = 12345;
/// 
/// let (calendar_items, used_diet) = get_diet_with_ingredients_with_fallback_search(
///     &now,
///     primary_diet_id,
///     &all_diets,
///     token
/// ).await?;
/// 
/// if used_diet.id != primary_diet_id {
///     println!("Using fallback diet: {}", used_diet.id);
/// }
/// # Ok(())
/// # }
/// ```
async fn get_diet_with_ingredients_with_fallback_search(
    date: &DateTime<Local>,
    primary_diet_id: i64,
    all_diets: &DietsList,
    token: &str,
) -> eyre::Result<(CalendarDayItems, Diet)> {
    let primary_diet_obj = all_diets.members.iter().find(|d| d.id == primary_diet_id).ok_or_else(|| {
        eyre::eyre!("Primary diet with ID {primary_diet_id} not found in the list of diets")
    })?;
    let primary_diet = get_diet_with_ingredients(date, primary_diet_id, token).await?;
    if ! primary_diet.diet_elements.members.is_empty() {
        return Ok((primary_diet, primary_diet_obj.clone()));
    }

    for diet in &all_diets.members {
        if diet.id == primary_diet_id {
            continue;
        }

        let alternative_diet = get_diet_with_ingredients(date, diet.id, token).await?;
        if ! alternative_diet.diet_elements.members.is_empty() {
            return Ok((alternative_diet, diet.clone()));
        }
    }

    Ok((primary_diet, primary_diet_obj.clone()))
}
*/

// TODO: Phase 5 - Reimplement with new API structures
/*
/// Fetches diet information for a specific date and enriches it with ingredient details.
///
/// This function retrieves the meal options available for a given diet on a specific date,
/// then enriches each meal option with detailed ingredient information. It implements a
/// two-tier caching strategy: first checking an in-memory LRU cache for ingredients, then
/// falling back to API calls for cache misses. This significantly improves performance when
/// viewing the same meals multiple times.
///
/// The ingredient enrichment is essential for the AI to make informed recommendations based
/// on nutritional content, allergens, and dietary restrictions. Without ingredients, the AI
/// would only have meal names to work with.
///
/// # Arguments
///
/// * `date` - The specific date and time for which to retrieve diet information.
///           Determines which meals are available based on the diet schedule.
///
/// * `diet_id` - The unique identifier of the diet subscription to query.
///               Must be a valid diet ID from the user's active subscriptions.
///
/// * `token` - Bearer authentication token for API requests to the meal provider service.
///
/// # Returns
///
/// Returns a `CalendarDayItems` structure containing:
/// - All available meal options for each meal type (breakfast, lunch, dinner, etc.)
/// - Enriched ingredient information for each meal option
/// - Nutritional data and allergen information when available
/// - The currently selected option for each meal type (if any)
///
/// # Errors
///
/// This function will return an error if:
/// * The initial diet fetch fails due to network issues or invalid diet ID
/// * Authentication token is invalid or expired
/// * API returns malformed data that cannot be deserialized
/// * Individual ingredient fetches fail (wrapped with context about which ingredient)
///
/// # Side Effects
///
/// * Makes one API call to fetch the base diet information
/// * Makes additional API calls for any ingredients not found in cache
/// * Updates the in-memory ingredients cache with newly fetched data
/// * Displays status messages to the terminal during ingredient fetching
/// * The cache is persisted by the caller, not by this function
///
/// # Caching Strategy
///
/// The function uses a singleton `IngredientsCache` instance that:
/// 1. First checks for cached ingredients by dish_size_id
/// 2. Returns cached data if available (avoiding API calls)
/// 3. Fetches from API only for cache misses
/// 4. Updates the cache with newly fetched ingredients
///
/// # Examples
///
/// ```no_run
/// # async fn example() -> eyre::Result<()> {
/// # use chrono::Local;
/// # let token = "auth_token";
/// let today = Local::now();
/// let diet_id = 12345;
/// 
/// let calendar_items = get_diet_with_ingredients(&today, diet_id, token).await?;
/// 
/// for meal in &calendar_items.diet_elements.members {
///     println!("Meal type: {}", meal.meal_type.name);
///     for option in &meal.options {
///         if let Some(ingredients) = &option.ingredients {
///             println!("  {} - {} kcal", option.name, ingredients.kcal);
///         }
///     }
/// }
/// # Ok(())
/// # }
/// ```
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
*/

// TODO: Phase 5 - Reimplement with new API structures
/*
/// Orchestrates the complete meal selection workflow for a single day.
///
/// This function is the core business logic that handles the entire process of selecting
/// meals for a specific date. It integrates multiple systems including the meal provider API,
/// AI recommendation service, user preferences, and historical meal data to provide
/// intelligent meal suggestions while maintaining user control over final selections.
///
/// The workflow consists of:
/// 1. Fetching the available menu for the target date with fallback diet search
/// 2. Gathering 14 days of historical meal choices for context
/// 3. Requesting AI recommendations based on preferences and history
/// 4. Displaying AI reasoning with animated text output
/// 5. Handling user selection (interactive or automatic in yolo mode)
/// 6. Confirming and submitting menu changes to the API
/// 7. Updating progress tracking for subsequent days
///
/// # Arguments
///
/// * `token` - Authentication bearer token for API calls to the meal provider service.
///             Must be a valid token obtained from the login process.
///
/// * `date` - Target date for meal selection. The function will find the appropriate
///            diet schedule for this date and fetch available meal options.
///
/// * `diets` - Reference to the list of available diet plans with their schedules.
///             Used to determine which diet applies to the given date and to fetch
///             historical meal data.
///
/// * `yolo` - When `true`, automatically accepts AI recommendations without user
///            confirmation. Useful for batch processing or automated meal planning.
///            When `false`, prompts user for interactive selection and confirmation.
///
/// * `preferences` - User preferences configuration including dietary restrictions,
///                   meal preferences, and AI model settings. These preferences
///                   guide the AI recommendation engine.
///
/// # Errors
///
/// This function will return an error if:
/// * Network or API communication fails during any of the multiple API calls
/// * No diet schedule is available for the specified date
/// * The diet has no meal options available (empty diet elements)
/// * The AI service fails to generate recommendations
/// * User cancels the selection process in interactive mode
/// * Menu change confirmation fails at the API level
/// * Cache operations fail when saving ingredients data
///
/// All errors are wrapped with contextual information using `eyre::Result` for
/// better debugging and error reporting.
///
/// # Side Effects
///
/// This function has several important side effects:
/// * Makes multiple API calls to fetch diet data, historical orders, and submit changes
/// * Calls the AI service for meal recommendations (may incur API costs)
/// * Saves ingredients cache to disk for performance optimization
/// * Updates the preferences file with the next day to check
/// * Produces terminal output including status messages, AI reasoning, and prompts
/// * May require user interaction if not in yolo mode
/// * Modifies the user's meal selection on the remote service
///
/// # Performance Considerations
///
/// This function performs multiple async operations including:
/// * Several sequential API calls that cannot be parallelized due to data dependencies
/// * AI service call which may have variable latency
/// * Disk I/O for cache operations
/// * Terminal I/O with animated text display
///
/// The function uses fallback mechanisms for diet search to improve reliability
/// when the primary diet lookup fails.
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
    let (calendar_day_items, diet) = get_diet_with_ingredients_with_fallback_search(&date, diet_id, diets, token)
        .await
        .wrap_err("getting diet with ingredients")?;
    let diet_id = diet.id;
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
*/

// TODO: Phase 5 - Reimplement with new API structures
/*
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
*/

// TODO: Phase 5 - Reimplement with new API structures
// The function fetch_historical_orders is commented out below
/*
/// Retrieves historical meal orders for a specified number of past days.
///
/// This function fetches the user's meal selection history to provide context for AI-based
/// meal recommendations. It collects actual meal choices (not just available options) from
/// previous days, which helps the AI understand user preferences, eating patterns, and
/// dietary habits. The history is returned as a map with human-readable time references
/// like "yesterday" and "3 days ago" for easier interpretation.
///
/// The function gracefully handles days where no diet was active, logging these occurrences
/// without failing. It enriches all historical data with ingredient information from the
/// cache or API as needed.
///
/// # Arguments
///
/// * `token` - Bearer authentication token for API requests to the meal provider service.
///
/// * `diets` - Complete list of user's diet subscriptions, used to determine which diet
///             was active on each historical date.
///
/// * `date` - Reference date from which to count backwards. Typically today's date when
///            selecting meals for today or tomorrow.
///
/// * `days` - Number of days of history to fetch, counting backwards from the reference date.
///            Typically 14 days to provide sufficient context for AI recommendations.
///
/// # Returns
///
/// Returns an `IndexMap<String, CalendarDayItems>` where:
/// - Keys are human-readable time references ("yesterday", "2 days ago", "3 days ago", etc.)
/// - Values are the complete diet information including selected meals and ingredients
/// - The map maintains insertion order with most recent days first
///
/// # Errors
///
/// This function will return an error if:
/// * Network failures occur during API calls
/// * Authentication token is invalid or expired
/// * Critical API errors prevent fetching any historical data
/// * Date calculations result in invalid dates (unlikely with valid inputs)
///
/// Note: Days without active diets are handled gracefully and don't cause errors.
///
/// # Side Effects
///
/// * Makes multiple API calls (one per historical day with an active diet)
/// * Displays status messages to the terminal showing fetch progress
/// * Updates the global ingredients cache with any newly fetched ingredient data
/// * Prints informational messages for days without active diets
///
/// # Performance Considerations
///
/// This function makes sequential API calls for each day of history, which can be slow
/// for large values of `days`. With the typical value of 14 days, expect 14+ API calls.
/// Consider implementing parallel fetching if performance becomes an issue.
///
/// # Examples
///
/// ```no_run
/// # async fn example() -> eyre::Result<()> {
/// # use chrono::Local;
/// # let token = "auth_token";
/// # let diets = todo!();
/// let today = Local::now();
/// let history = fetch_historical_orders(token, &diets, &today, 14).await?;
/// 
/// if let Some(yesterday) = history.get("yesterday") {
///     println!("Yesterday's meals: {:?}", yesterday.diet_elements.members);
/// }
/// # Ok(())
/// # }
/// ```
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
*/

// TODO: Phase 5 - Reimplement with new API structures
/*
/// Handles the interactive or automatic selection of meals based on AI recommendations.
///
/// This function presents the AI's meal recommendations to the user and handles the selection
/// process. In interactive mode, it displays detailed AI analysis for each meal option and
/// allows the user to confirm or override AI suggestions. In "yolo" mode, it automatically
/// accepts all AI recommendations without user intervention. The function tracks which meals
/// differ from the currently selected options and prepares change requests accordingly.
///
/// The function provides rich visual feedback including:
/// - AI analysis and reasoning for each meal option
/// - Color-coded terminal output for better readability
/// - Animated text display for AI reasoning (typewriter effect)
/// - Suggestions to update preferences when user choices differ from AI recommendations
///
/// # Arguments
///
/// * `calendar_day_items` - Complete diet information including all available meal options,
///                          current selections, and ingredient data for the target date.
///
/// * `_date` - The date for meal selection (currently unused but kept for potential future use
///             or backwards compatibility).
///
/// * `ai_result` - AI-generated recommendations including selected dishes, detailed analysis
///                 of each option, and reasoning for the selections.
///
/// * `menu_changes` - Mutable reference to accumulate all meal changes. This is populated
///                    with items that differ from the current selections.
///
/// * `yolo` - When `true`, automatically accepts all AI recommendations without prompting.
///            When `false`, presents an interactive selection interface for each meal.
///
/// # Returns
///
/// Returns `Ok(())` on successful completion of the selection process.
///
/// # Errors
///
/// This function will return an error if:
/// * AI recommendations reference dish items not present in the calendar (data inconsistency)
/// * AI recommendations reference dish options not available for a meal type
/// * User cancels the interactive selection process (Ctrl+C or selection error)
/// * Terminal I/O operations fail during interactive prompts
///
/// # Side Effects
///
/// * Modifies the provided `menu_changes` structure by adding change items
/// * Produces extensive terminal output including:
///   - AI analysis for each dish option with formatted text
///   - Animated reasoning text with typewriter effect
///   - Interactive selection prompts (if not in yolo mode)
///   - Difference notifications when user overrides AI suggestions
/// * May block waiting for user input in interactive mode
///
/// # User Experience
///
/// In interactive mode:
/// 1. For each meal type, shows AI analysis of all options
/// 2. Displays AI's reasoning for its selection
/// 3. Presents a selection menu with AI's choice as default
/// 4. Notifies user when their choice differs from AI recommendation
/// 5. Suggests running `powermeal edit-preferences` to update preferences
///
/// In yolo mode:
/// 1. Shows the same AI analysis and reasoning
/// 2. Automatically selects AI recommendations
/// 3. Displays "[auto-selected by AI]" for each selection
///
/// # Examples
///
/// ```no_run
/// # async fn example() -> eyre::Result<()> {
/// # let calendar_day_items = todo!();
/// # let date = chrono::Local::now().date_naive();
/// # let ai_result = todo!();
/// let mut menu_changes = ChangeMenuRequest::default();
/// 
/// // Interactive mode - prompts user for each meal
/// select_dishes(&calendar_day_items, &date, ai_result, &mut menu_changes, false).await?;
/// 
/// // Process the accumulated changes
/// if !menu_changes.items.is_empty() {
///     println!("Total changes: {}", menu_changes.items.len());
/// }
/// # Ok(())
/// # }
/// ```
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
*/

// TODO: Phase 5 - Reimplement with new API structures
/*
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
*/

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
// TODO: Phase 5 - Reimplement when AI functions are available
/*
/// Migrates legacy structured preferences to the new free-text format.
///
/// This function handles the one-time migration from the old preference system (which used
/// structured `UserAdjustment` entries with like/dislike/avoid flags) to the new system
/// that uses natural language descriptions. The migration process is interactive and ensures
/// user control over the final preference text.
///
/// The migration workflow:
/// 1. Checks if AI configuration exists, prompts for setup if needed
/// 2. Uses AI to analyze legacy preferences and generate natural language description
/// 3. Opens the user's preferred text editor with the AI-generated draft
/// 4. Allows user to review, edit, and customize the preferences
/// 5. Saves the final edited version and marks migration as complete
///
/// # Arguments
///
/// * `preferences` - Mutable reference to the user's preferences structure. Will be modified
///                   to include the new free-text preferences and remove legacy data.
///
/// # Returns
///
/// Returns `Ok(())` on successful migration.
///
/// # Errors
///
/// This function will return an error if:
/// * AI configuration setup fails or is cancelled by the user
/// * AI service fails to generate preference description from legacy data
/// * Text editor fails to launch (invalid EDITOR environment variable)
/// * Text editor exits with non-zero status (indicating error)
/// * User saves an empty file (interpreted as cancellation)
/// * File I/O operations fail during temporary file handling
/// * Preferences cannot be saved after migration
///
/// # Side Effects
///
/// * Modifies the preferences object in place:
///   - Sets up AI configuration if not present
///   - Replaces legacy adjustments with free-text preferences
///   - Saves preferences to disk after each major step
/// * Displays status messages and instructions to the terminal
/// * Launches external text editor (uses EDITOR env var, defaults to nano)
/// * Creates and cleans up temporary files for editing
/// * Makes API calls to AI service for preference generation
///
/// # User Interaction
///
/// The function requires active user participation:
/// 1. May prompt for AI API configuration (base URL and API key)
/// 2. Opens text editor with AI-generated draft and instructions
/// 3. Waits for user to edit and save the file
/// 4. Validates that content was saved (not empty)
///
/// # Migration Safety
///
/// The migration is designed to be safe:
/// - Original preferences are preserved until migration succeeds
/// - Empty file save cancels migration, keeping legacy preferences
/// - Clear error messages guide the user through any issues
/// - User has full control over the final preference text
///
/// # Examples
///
/// ```no_run
/// # async fn example() -> eyre::Result<()> {
/// # use powermeal::preferences::Preferences;
/// let mut preferences = Preferences::load()?;
/// 
/// // Check if migration is needed
/// if !preferences.adjustments.is_empty() {
///     println!("Migrating to new preference format...");
///     migrate_preferences(&mut preferences).await?;
///     println!("Migration complete!");
/// }
/// # Ok(())
/// # }
/// ```
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
*/

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

        // TODO: Phase 5 - Re-enable interactive confirmation with dialoguer
        println!("Opening editor to create preferences...");
        /*
        if !dialoguer::Confirm::new()
            .with_prompt("Open editor to create preferences?")
            .interact()?
        {
            return Ok(());
        }
        */
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
        // TODO: Phase 5 - Re-enable interactive confirmation with dialoguer
        println!("Saving the changes to your preferences...");
        /*
        if dialoguer::Confirm::new()
            .with_prompt("Save the changes to your preferences?")
            .interact()?
        {
        */
            let mut prefs = Preferences::load_preferences();
            prefs.user_preferences = trimmed;
            prefs.save_preferences();
            println!("Preferences updated successfully.");
        /*
        }
        */
    } else {
        println!("No changes detected in the edited preferences.");
    }

    Ok(())
}
