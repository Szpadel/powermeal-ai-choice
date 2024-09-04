use std::collections::HashMap;

use chrono::{DateTime, FixedOffset, Local, NaiveDate};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct CalendarDayItems {
    #[serde(rename = "dietElements")]
    pub diet_elements: DietElements,
}

impl CalendarDayItems {
    pub fn get_dish(&self, dish_item_id: &str, dish_id: &str) -> Option<&MenuDietOption> {
        self.get_dish_item(dish_item_id).and_then(|dish_item| {
            dish_item
                .options
                .iter()
                .find(|option| option.dish.id == dish_id)
        })
    }
    pub fn get_dish_item(&self, dish_item_id: &str) -> Option<&DishItem> {
        self.diet_elements
            .members
            .iter()
            .find(|dish_item| dish_item.id == dish_item_id)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DietElements {
    #[serde(rename = "hydra:member")]
    pub members: Vec<DishItem>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DishItem {
    #[serde(rename = "@id")]
    pub id: String,
    pub options: Vec<MenuDietOption>,
    #[serde(rename = "mealType")]
    pub meal_type: MealType,
    #[serde(rename = "dishSize")]
    pub dish_size: DishSize,
}

impl DishItem {
    pub fn options(&self) -> Vec<&MenuDietOption> {
        self.options
            .iter()
            .filter(|option| option.enabled)
            .collect()
    }

    pub fn get_dish(&self, dish_id: &str) -> Option<&MenuDietOption> {
        self.options.iter().find(|option| option.dish.id == dish_id)
    }

    pub fn get_selected_option(&self) -> Option<&MenuDietOption> {
        let id = &self.dish_size.dish.id;
        self.options.iter().find(|option| &option.dish.id == id)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DishSize {
    pub dish: Dish,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MealType {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MenuDietOption {
    pub name: String,
    pub ingredients: Vec<String>,
    pub enabled: bool,
    pub dish: Dish,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Dish {
    #[serde(rename = "@id")]
    pub id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RefreshTokenResponse {
    pub token: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
}

impl CalendarDayItems {
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

#[derive(Debug, Deserialize, Serialize)]
pub struct DietsList {
    #[serde(rename = "hydra:member")]
    pub members: Vec<Diet>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Diet {
    pub id: i64,
    #[serde(rename = "firstDeliveryDate")]
    pub first_delivery_date: DateTime<FixedOffset>,
    #[serde(rename = "lastDeliveryDate")]
    pub last_delivery_date: DateTime<FixedOffset>,
}

impl DietsList {
    pub fn diet_for_date(&self, date: &DateTime<Local>) -> Option<&Diet> {
        self.members
            .iter()
            .find(|diet| diet.first_delivery_date <= *date && *date <= diet.last_delivery_date)
    }

    pub fn diets_in_time_range(&self, from: &DateTime<Local>, to: &DateTime<Local>) -> Vec<&Diet> {
        self.members
            .iter()
            .filter(|diet| diet.first_delivery_date <= *to && *from <= diet.last_delivery_date)
            .collect()
    }
}

#[derive(Debug, Deserialize)]

pub struct Calendar {
    pub days: HashMap<NaiveDate, DietCalendarDay>,
}

#[derive(Debug, Deserialize)]
pub struct DietCalendarDay {
    #[serde(rename = "newState")]
    pub state: DietDayState,
}

#[derive(Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum DietDayState {
    #[serde(rename = "NOT_DIET_CANT_PLACE_ORDER")]
    NoDiet,
    #[serde(rename = "DELIVERED_NOT_RATED_CAN_RATE")]
    Delivered,
    #[serde(rename = "NOT_DELIVERED_BLOCKED")]
    CannotChange,
    #[serde(rename = "NOT_DELIVERED_WITH_CONFIGURABLE_ALL")]
    AvailableToSelect,
    #[serde(rename = "NOT_DELIVERED_WITH_CONFIGURABLE_WITHOUT_MENU")]
    WithoutMenu,
}

#[derive(Debug, Serialize, Default)]
pub struct ChangeMenuRequest {
    pub items: Vec<ChangeMenuItem>,
}

#[derive(Debug, Serialize)]
pub struct ChangeMenuItem {
    pub dish: String,
    #[serde(rename = "dishItem")]
    pub dish_item: String,
}
