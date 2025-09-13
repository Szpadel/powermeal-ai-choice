//! Data models for PowerFoods API serialization and deserialization.
//!
//! This module contains data structures for the new PowerFoods API.
//! Phase 2 implementation: New API data models.

use serde::{Deserialize, Serialize};

// ============================================================================
// Client Diet List Structures
// ============================================================================

/// Response from GET /clientDiets endpoint
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ClientDietsResponse {
    pub status: String,
    pub data: ClientDietsData,
}

/// Wrapper for the list of client diets
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ClientDietsData {
    pub diets: Vec<ClientDiet>,
}

/// Individual client diet information
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ClientDiet {
    pub id: i64,                        // client_diet_id
    pub client_id: i64,
    pub client_address_id: i64,
    pub diet_id: i64,                   // diet type ID
    pub date_from: String,
    pub date_to: String,
    pub created_at: String,
    pub updated_at: String,
    pub var_cal_id: i64,                // calorie variant ID
    pub quantity: i32,
    pub is_active: i32,
    pub diet_name: String,
    pub var_cal_name: String,
    pub var_id: i64,                    // variant ID
    pub variant_name: Option<String>,
    pub street: Option<String>,
    pub building: Option<String>,
    pub flat: Option<String>,
    pub city_name: Option<String>,
    pub postcode: Option<String>,
    pub diet_price: Option<f64>,
    pub total_days: Option<i32>,
    pub has_menu_choice: i32,
    pub order_id: Option<i64>,
    pub client_diet_name: Option<String>,
    pub invoice_id: Option<i64>,
}

// ============================================================================
// Diet Details Structures
// ============================================================================

/// Response from GET /clientDiets/{id} endpoint
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ClientDietDetails {
    pub status: String,
    pub data: DietDetailsData,
}

/// Wrapper for diet details with days information
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DietDetailsData {
    pub items: Vec<ClientDietItem>,
    #[serde(rename = "totalDays")]
    pub total_days: i32,
    #[serde(rename = "pastDays")]
    pub past_days: i32,
}

/// Individual day item in a diet
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ClientDietItem {
    pub id: i64,                        // client_diet_item_id (day identifier)
    pub date_dlv: String,                // delivery date
    pub diet_id: i64,
    pub var_id: i64,
    pub var_cal_id: i64,
    pub has_menu_choice: i32,           // unreliable, requires validation
    pub dishes: Vec<DishInfo>,
}

/// Basic dish information within a diet item
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DishInfo {
    pub dish_id: i64,
    pub dish_name: String,
    pub var_cal_meal_id: i64,
    // Additional fields can be added as needed
}

// ============================================================================
// Menu Structures
// ============================================================================

/// Response from GET /diets/menu endpoint
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MenuResponse {
    pub data: Vec<MenuDish>,
}

/// Detailed dish information with nutritional data
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MenuDish {
    pub brand_id: i32,
    pub dmenu: String,                  // date
    pub diet_id: i64,
    pub diet_name: String,
    pub var_id: i64,
    pub var_name: String,
    pub var_cal_id: i64,
    pub var_cal_name: String,
    pub var_cal_meal_id: i64,           // unique meal variant identifier

    // Dish information
    pub dish_id: i64,
    pub dish_name: String,
    pub dish_photo: Option<String>,
    pub dish_photo_client: Option<String>,
    pub dish_suggest: Option<String>,
    pub dish_tags: Option<String>,

    // Meal information
    pub meal_id: i64,
    pub meal_name: String,
    pub meal_seq: i32,                  // sequence (1=breakfast, etc.)

    // Nutritional information
    pub r#macro: Option<String>,        // formatted macro string
    pub protein: String,
    pub fat: String,
    pub fat_saturated: Option<String>,
    pub carbohydrate: String,
    pub sugar: Option<String>,
    pub salt: Option<String>,
    pub fiber: Option<String>,
    pub calory: String,
    pub weight: Option<String>,

    // Ingredients and allergens
    pub dish_ing_names: String,         // ingredients text
    pub dish_allergens: Option<String>,

    // Rating
    pub rating: Option<f64>,
    pub rating_descr: Option<String>,
    pub rating_id: Option<i64>,
}

// ============================================================================
// Update Request Structures
// ============================================================================

/// Request body for POST /clientDiets/dish endpoint
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DishUpdateRequest {
    pub brand_id: i32,
    pub client_diet_item_id: i64,
    pub dish_id: i64,
    pub diet_id: i64,
    pub var_cal_meal_id: i64,
}

// ============================================================================
// Delivery Configuration Structures
// ============================================================================

/// Response from GET /diets/delivery endpoint
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DeliveryConfig {
    pub data: DeliveryData,
}

/// Wrapper for delivery rules
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DeliveryData {
    pub delivery: Vec<DeliveryRule>,
}

/// Individual delivery rule with cutoff times
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DeliveryRule {
    pub day_id: i32,                    // cutoff day (1=Sunday...7=Saturday)
    pub delv_day_id: i32,               // delivery day
    pub delv_type_id: i32,              // 5 = menu selection
    pub delv_time: String,               // cutoff time
}

// ============================================================================
// Legacy Placeholder Structures (to be removed in Phase 5)
// ============================================================================

/// Placeholder for backward compatibility - will be removed
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CalendarDayItems {
    pub placeholder: String,
}

/// Placeholder for backward compatibility - will be removed
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DishItem {
    pub placeholder: String,
}

/// Placeholder for backward compatibility - will be removed
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DishSizeIngredients {
    pub placeholder: String,
}