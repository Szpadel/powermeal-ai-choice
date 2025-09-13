//! Data models for PowerMeal API serialization and deserialization.
//!
//! This module contains all the data structures used to communicate with the PowerMeal API,
//! including menu items, diet options, calendar data, and authentication responses.
//! All structures are designed to match the API's JSON format exactly, using serde
//! field renaming where necessary.
//!
//! # Main Components
//!
//! - `CalendarDayItems` - Daily menu container with all meal slots
//! - `DishItem` - Individual meal slot with available options
//! - `MenuDietOption` - Specific dish choice within a meal slot
//! - `DietsList` - User's active diet subscriptions
//! - `Calendar` - Multi-day calendar with diet states
//! - `DietDayState` - Enumeration of possible menu selection states

use std::collections::HashMap;

use chrono::{DateTime, FixedOffset, Local, NaiveDate};
use serde::{Deserialize, Serialize};

/// Container for all meal items available on a specific day.
///
/// This is the main structure returned by the API when fetching daily menu options.
/// It contains all meal slots (breakfast, lunch, dinner, etc.) with their available dishes.
///
/// # API Mapping
///
/// The `diet_elements` field is renamed from `dietElements` in the JSON response.
#[derive(Debug, Deserialize, Serialize)]
pub struct CalendarDayItems {
    #[serde(rename = "dietElements")]
    pub diet_elements: DietElements,
}

impl CalendarDayItems {
    /// Retrieves a specific dish option by its meal slot and dish IDs.
    ///
    /// # Arguments
    ///
    /// * `dish_item_id` - The ID of the meal slot (e.g., breakfast, lunch)
    /// * `dish_id` - The ID of the specific dish within that slot
    ///
    /// # Returns
    ///
    /// * `Some(&MenuDietOption)` - If the dish exists in the specified slot
    /// * `None` - If either the slot or dish cannot be found
    pub fn get_dish(&self, dish_item_id: &str, dish_id: &str) -> Option<&MenuDietOption> {
        self.get_dish_item(dish_item_id).and_then(|dish_item| {
            dish_item
                .options
                .iter()
                .find(|option| option.dish.id == dish_id)
        })
    }
    /// Retrieves a meal slot by its ID.
    ///
    /// # Arguments
    ///
    /// * `dish_item_id` - The ID of the meal slot to retrieve
    ///
    /// # Returns
    ///
    /// * `Some(&DishItem)` - If the meal slot exists
    /// * `None` - If the meal slot cannot be found
    pub fn get_dish_item(&self, dish_item_id: &str) -> Option<&DishItem> {
        self.diet_elements
            .members
            .iter()
            .find(|dish_item| dish_item.id == dish_item_id)
    }
}

/// Wrapper for the list of meal slots in a day's menu.
///
/// # API Mapping
///
/// The `members` field is renamed from `hydra:member` in the JSON response,
/// following Hydra API conventions.
#[derive(Debug, Deserialize, Serialize)]
pub struct DietElements {
    #[serde(rename = "hydra:member")]
    pub members: Vec<DishItem>,
}

/// Represents a meal slot with its available dish options.
///
/// Each DishItem corresponds to a specific meal time (breakfast, lunch, dinner, etc.)
/// and contains all the dish options available for that slot, along with the currently
/// selected dish.
///
/// # API Mapping
///
/// - `id` is renamed from `@id`
/// - `meal_type` is renamed from `mealType`
/// - `dish_size` is renamed from `dishSize`
#[derive(Debug, Deserialize, Serialize)]
pub struct DishItem {
    #[serde(rename = "@id")]
    pub id: String,
    /// All available dish options for this meal slot
    pub options: Vec<MenuDietOption>,
    #[serde(rename = "mealType")]
    pub meal_type: MealType,
    #[serde(rename = "dishSize")]
    pub dish_size: DishSize,
}

impl DishItem {
    /// Returns only the enabled dish options for this meal slot.
    ///
    /// Filters out any options that have been disabled by the API
    /// (e.g., unavailable dishes or restricted options).
    ///
    /// # Returns
    ///
    /// A vector of references to enabled `MenuDietOption` items.
    pub fn options(&self) -> Vec<&MenuDietOption> {
        self.options
            .iter()
            .filter(|option| option.enabled)
            .collect()
    }

    /// Finds a specific dish option within this meal slot.
    ///
    /// # Arguments
    ///
    /// * `dish_id` - The ID of the dish to find
    ///
    /// # Returns
    ///
    /// * `Some(&MenuDietOption)` - If the dish exists in this slot
    /// * `None` - If the dish is not found
    pub fn get_dish(&self, dish_id: &str) -> Option<&MenuDietOption> {
        self.options.iter().find(|option| option.dish.id == dish_id)
    }

    /// Returns the currently selected dish option for this meal slot.
    ///
    /// The selected option is determined by matching the dish ID stored
    /// in the `dish_size` field with the available options.
    ///
    /// # Returns
    ///
    /// * `Some(&MenuDietOption)` - The currently selected dish
    /// * `None` - If no dish is selected or the selection is invalid
    pub fn get_selected_option(&self) -> Option<&MenuDietOption> {
        let id = &self.dish_size.dish.id;
        self.options.iter().find(|option| &option.dish.id == id)
    }
}

/// Container for ingredient lists of different dish sizes.
///
/// # API Mapping
///
/// The `members` field is renamed from `hydra:member` in the JSON response.
#[derive(Debug, Deserialize, Serialize)]
pub struct DishIngredients {
    #[serde(rename = "hydra:member")]
    pub members: Vec<DishSizeIngredients>,
}

/// Ingredients list for a specific dish size variant.
///
/// Different dish sizes may have slightly different ingredients
/// or proportions, so ingredients are stored per size.
///
/// # API Mapping
///
/// The `dish_size_id` field is renamed from `dishSizeId`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DishSizeIngredients {
    #[serde(rename = "dishSizeId")]
    pub dish_size_id: i64,
    /// List of ingredient names for this dish size
    pub ingredients: Vec<String>,
}

/// Wrapper for dish information within a size context.
#[derive(Debug, Deserialize, Serialize)]
pub struct DishSize {
    /// The dish currently selected for this size
    pub dish: Dish,
}

/// Represents the type of meal (breakfast, lunch, dinner, etc.).
#[derive(Debug, Deserialize, Serialize)]
pub struct MealType {
    /// Human-readable name of the meal type
    pub name: String,
}

/// Represents an individual dish choice within a meal slot.
///
/// Each option contains the dish details, availability status,
/// and optionally cached ingredient information.
///
/// # API Mapping
///
/// - `dish_size_id` is renamed from `dishSizeId`
/// - `ingredients` is not serialized (marked with `skip`)
#[derive(Debug, Deserialize, Serialize)]
pub struct MenuDietOption {
    /// Display name of the dish
    pub name: String,
    /// Whether this option is currently available for selection
    pub enabled: bool,
    /// Reference to the dish entity
    pub dish: Dish,
    #[serde(rename = "dishSizeId")]
    pub dish_size_id: i64,
    /// Cached ingredients (not part of API response, populated separately)
    #[serde(skip)]
    pub ingredients: Option<DishSizeIngredients>,
}

/// Reference to a dish entity.
///
/// # API Mapping
///
/// The `id` field is renamed from `@id` in the JSON response.
#[derive(Debug, Deserialize, Serialize)]
pub struct Dish {
    #[serde(rename = "@id")]
    pub id: String,
}

/// Response from the authentication token refresh endpoint.
///
/// Contains both the new access token and refresh token for future use.
///
/// # API Mapping
///
/// The `refresh_token` field is renamed from `refreshToken`.
#[derive(Debug, Deserialize, Serialize)]
pub struct RefreshTokenResponse {
    /// New access token for API requests
    pub token: String,
    #[serde(rename = "refreshToken")]
    /// New refresh token for future token renewal
    pub refresh_token: String,
}

impl CalendarDayItems {
    /// Generates a human-readable summary of all meal options for debugging.
    ///
    /// Creates a formatted string showing all meal slots, their available options,
    /// and marks the currently selected option with an asterisk.
    ///
    /// # Returns
    ///
    /// A formatted string suitable for console output or logging.
    ///
    /// # Example Output
    ///
    /// ```text
    /// Breakfast
    ///   [*] Oatmeal with berries
    ///   [ ] Scrambled eggs
    /// Lunch
    ///   [ ] Grilled chicken salad
    ///   [*] Vegetarian pasta
    /// ```
    pub fn debug_options(&self) -> String {
        let mut summary = String::new();
        for dish in &self.diet_elements.members {
            summary.push_str(&format!("{}\n", dish.meal_type.name));
            let selected_option_id = dish.get_selected_option().map(|o| o.dish.id.clone()).unwrap_or_default();
            for option in &dish.options {
                if !option.enabled {
                    continue;
                }
                summary.push_str(&format!(
                    "  [{}] {}\n",
                    if option.dish.id == selected_option_id { "*" } else { " " },
                    option.name,
                ));
                // summary.push_str(&format!("        {}\n", option.ingredients.join(", ")));
            }
        }
        summary
    }
}

/// Container for user's diet subscriptions.
///
/// Lists all active and past diet periods for the user account.
///
/// # API Mapping
///
/// The `members` field is renamed from `hydra:member`.
#[derive(Debug, Deserialize, Serialize)]
pub struct DietsList {
    #[serde(rename = "hydra:member")]
    pub members: Vec<Diet>,
}

/// Represents a diet subscription period.
///
/// Each diet has a defined start and end date during which
/// meal deliveries are active.
///
/// # API Mapping
///
/// - `first_delivery_date` is renamed from `firstDeliveryDate`
/// - `last_delivery_date` is renamed from `lastDeliveryDate`
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Diet {
    /// Unique identifier for this diet subscription
    pub id: i64,
    #[serde(rename = "firstDeliveryDate")]
    /// Start date of the diet period
    pub first_delivery_date: DateTime<FixedOffset>,
    #[serde(rename = "lastDeliveryDate")]
    /// End date of the diet period
    pub last_delivery_date: DateTime<FixedOffset>,
}

impl DietsList {
    /// Finds the active diet for a specific date.
    ///
    /// # Arguments
    ///
    /// * `date` - The date to check for an active diet
    ///
    /// # Returns
    ///
    /// * `Some(&Diet)` - The diet active on the given date
    /// * `None` - If no diet is active on that date
    pub fn diet_for_date(&self, date: &DateTime<Local>) -> Option<&Diet> {
        self.members
            .iter()
            .find(|diet| diet.first_delivery_date <= *date && *date <= diet.last_delivery_date)
    }

    /// Finds all diets that overlap with a given time range.
    ///
    /// Returns any diet whose period intersects with the specified range,
    /// even if only partially.
    ///
    /// # Arguments
    ///
    /// * `from` - Start of the time range
    /// * `to` - End of the time range
    ///
    /// # Returns
    ///
    /// A vector of references to all overlapping diets.
    pub fn diets_in_time_range(&self, from: &DateTime<Local>, to: &DateTime<Local>) -> Vec<&Diet> {
        self.members
            .iter()
            .filter(|diet| diet.first_delivery_date <= *to && *from <= diet.last_delivery_date)
            .collect()
    }
}

/// Calendar view of diet days with their states.
///
/// Maps dates to their diet day information, primarily used
/// to determine which days allow meal selection.
#[derive(Debug, Deserialize)]
pub struct Calendar {
    /// Map of dates to their diet day information
    pub days: HashMap<NaiveDate, DietCalendarDay>,
}

/// Information about a specific day in the diet calendar.
///
/// # API Mapping
///
/// The `state` field is renamed from `newState`.
#[derive(Debug, Deserialize)]
pub struct DietCalendarDay {
    #[serde(rename = "newState")]
    /// Current state of meal selection for this day
    pub state: DietDayState,
}

/// Enumeration of all possible states for a diet day.
///
/// These states determine what actions are available for meal selection
/// on a given day. States are ordered by their typical progression
/// in the meal delivery lifecycle.
///
/// # State Descriptions
///
/// - `NoDiet` - No active diet subscription for this day
/// - `NotBoughtDiet` - Diet available but not purchased
/// - `Delivered` - Meals delivered and can be rated
/// - `CannotChange` - Selection locked, too close to delivery
/// - `AvailableToSelect` - Full menu available for selection
/// - `WithoutMenu` - Diet active but menu not yet available
/// - `CannotRate` - Delivered but rating period expired
/// - `Disabled` - Day is disabled for selection
/// - `NotDeliveredCanSelectMenu` - Can select or change menu
#[derive(Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum DietDayState {
    #[serde(rename = "NOT_DIET_CANT_PLACE_ORDER")]
    NoDiet,
    #[serde(rename = "NOT_DIET_CAN_PLACE_ORDER")]
    NotBoughtDiet,
    #[serde(rename = "DELIVERED_NOT_RATED_CAN_RATE")]
    Delivered,
    #[serde(rename = "NOT_DELIVERED_BLOCKED")]
    CannotChange,
    #[serde(rename = "NOT_DELIVERED_WITH_CONFIGURABLE_ALL")]
    AvailableToSelect,
    #[serde(rename = "NOT_DELIVERED_WITH_CONFIGURABLE_WITHOUT_MENU")]
    WithoutMenu,
    #[serde(rename = "DELIVERED_NOT_RATED_BLOCKED")]
    CannotRate,
    #[serde(rename = "DISABLED")]
    Disabled,
    #[serde(rename = "NOT_DELIVERED_CAN_SELECT_MENU")]
    #[serde(alias = "NOT_DELIVERED_HAS_SELECTED_MENU")]
    NotDeliveredCanSelectMenu,
}

/// Request structure for updating meal selections.
///
/// Contains the list of dish changes to apply to a day's menu.
#[derive(Debug, Serialize, Default)]
pub struct ChangeMenuRequest {
    /// List of meal slot changes to apply
    pub items: Vec<ChangeMenuItem>,
}

/// Represents a single dish selection change.
///
/// # API Mapping
///
/// The `dish_item` field is renamed to `dishItem` in the JSON request.
#[derive(Debug, Serialize)]
pub struct ChangeMenuItem {
    /// ID of the dish to select
    pub dish: String,
    #[serde(rename = "dishItem")]
    /// ID of the meal slot to update
    pub dish_item: String,
}
