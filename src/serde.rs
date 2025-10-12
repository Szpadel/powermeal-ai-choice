//! Data models for PowerFoods API serialization and deserialization.
//!
//! This module contains data structures for the PowerFoods API.

use serde::{Deserialize, Deserializer, Serialize};

/// Helper function to deserialize both string and integer values to i32
fn deserialize_string_or_int<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    use std::fmt;

    struct StringOrIntVisitor;

    impl<'de> Visitor<'de> for StringOrIntVisitor {
        type Value = i32;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or an integer")
        }

        fn visit_i32<E>(self, value: i32) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value)
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value as i32)
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value as i32)
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            value.parse::<i32>().map_err(|e| {
                de::Error::custom(format!("Failed to parse '{}' as i32: {}", value, e))
            })
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            self.visit_str(&value)
        }
    }

    deserializer.deserialize_any(StringOrIntVisitor)
}

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
    pub id: i64, // client_diet_id
    pub client_id: i64,
    pub client_address_id: i64,
    pub diet_id: i64, // diet type ID
    pub date_from: String,
    pub date_to: String,
    pub created_at: String,
    pub updated_at: String,
    pub var_cal_id: i64, // calorie variant ID
    pub is_active: i32,
    pub diet_name: String,
    pub var_cal_name: String,
    pub var_id: i64, // variant ID
    pub variant_name: Option<String>,
    pub street: Option<String>,
    pub building: Option<String>,
    pub flat: Option<String>,
    pub city_name: Option<String>,
    pub postcode: Option<String>,
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
    #[serde(rename = "totalDays", deserialize_with = "deserialize_string_or_int")]
    pub total_days: i32,
    #[serde(rename = "pastDays", deserialize_with = "deserialize_string_or_int")]
    pub past_days: i32,
}

/// Individual day item in a diet
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ClientDietItem {
    pub id: i64,          // client_diet_item_id (day identifier)
    pub date_dlv: String, // delivery date
    pub diet_id: i64,
    pub var_id: i64,
    pub var_cal_id: i64,
    pub dishes: Option<Vec<ExistingDish>>, // Existing dish selections
}

/// Existing dish selection in ClientDietItem
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ExistingDish {
    #[serde(deserialize_with = "deserialize_string_or_int")]
    pub id: i32, // client_diet_dishes ID (used for PATCH)
    #[serde(deserialize_with = "deserialize_string_or_int")]
    pub dish_id: i32, // The dish ID
    #[serde(deserialize_with = "deserialize_string_or_int")]
    pub var_cal_meal_id: i32, // Meal variant ID (unique per meal slot)
    #[serde(deserialize_with = "deserialize_string_or_int")]
    pub meal_id: i32, // Meal ID
}

/// Response from GET /diets/menu endpoint
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MenuResponse {
    pub data: Vec<MenuDish>,
}

/// Menu dish information - only fields we actually use
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MenuDish {
    // Critical fields for core functionality
    pub dish_id: i64,      // Primary identifier
    pub dish_name: String, // Display name
    pub meal_name: String, // Meal type name
    #[serde(deserialize_with = "deserialize_string_or_int")]
    pub meal_id: i32, // Unique meal slot identifier
    pub meal_seq: i32,     // Meal ordering (1=breakfast, etc.)

    // Critical for API updates (only present in type=all responses)
    pub var_cal_meal_id: Option<i64>, // Required for dish updates

    // Important for AI context
    pub dish_ing_names: Option<String>, // Ingredients for AI analysis
    pub dmenu: String,                  // Date for history context
}

/// Request body for POST /clientDiets/dish endpoint
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DishUpdateRequest {
    pub brand_id: i32,
    pub client_diet_item_id: i64,
    pub dish_id: i64,
    pub diet_id: i64,
    pub var_cal_meal_id: i64,
}

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
    pub day_id: i32,               // cutoff day (1=Sunday...7=Saturday)
    pub delv_day_id: i32,          // delivery day
    pub delv_type_id: i32,         // 5 = menu selection
    pub delv_time: Option<String>, // cutoff time (can be null for some types)
}
