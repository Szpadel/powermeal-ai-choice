//! Data models for PowerMeal API serialization and deserialization.
//!
//! This module will contain data structures for the new PowerFoods API.
//! Currently in migration from old PowerMeal API to new PowerFoods API.
//!
//! # Phase 1: Old data structures removed
//! # Phase 2: Will implement new structures:
//!
//! ## Client Diet Structures
//! - ClientDietsResponse: Response containing list of client diets
//! - ClientDietsData: Wrapper for diet list
//! - ClientDiet: Individual client diet information
//!
//! ## Diet Details Structures
//! - ClientDietDetails: Detailed diet information with days
//! - DietDetailsData: Wrapper for diet details
//! - ClientDietItem: Individual day item in a diet
//!
//! ## Menu Structures
//! - MenuResponse: Response containing menu dishes
//! - MenuDish: Individual dish with full nutritional info
//!
//! ## Update Structures
//! - DishUpdateRequest: Request to update a single dish selection
//!
//! ## Delivery Configuration
//! - DeliveryConfig: Delivery rules and cutoff times
//! - DeliveryData: Wrapper for delivery rules
//! - DeliveryRule: Individual delivery rule

use serde::{Deserialize, Serialize};

// Temporary placeholder structures to allow compilation
// These will be replaced with actual structures in Phase 2

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CalendarDayItems {
    pub placeholder: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DishItem {
    pub placeholder: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DishSizeIngredients {
    pub placeholder: String,
}

// TODO: Remove these placeholder structures once Phase 2 is implemented