use std::collections::HashMap;

use async_openai::{
    types::{
        ChatCompletionRequestSystemMessage, ChatCompletionRequestUserMessage,
        CreateChatCompletionRequestArgs, ResponseFormat, ResponseFormatJsonSchema,
    },
    Client,
};
use chrono::NaiveDate;
use eyre::Context;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{preferences::Preferences, CalendarDayItems, DishItem};

#[derive(Debug, Serialize)]
pub struct SelectDishQuestion {
    pub user_changes: Vec<UserAdjustment>,
    pub dish_items: Vec<AiDishItem>,
    pub last_days_choices: HashMap<String, Vec<AiMenuDietOption>>,
    pub menu_date: NaiveDate,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserAdjustment {
    pub from: String,
    pub to: String,
    pub reason: Option<String>,
    pub date: NaiveDate,
}

#[derive(Debug, Deserialize)]
pub struct AiResponse {
    pub reasoning: Vec<String>,
    pub selections: HashMap<String, ResponseItem>,
}

#[derive(Debug, Serialize)]
pub struct AiDishItem {
    pub id: String,
    pub meal_type: String,
    pub options: Vec<AiMenuDietOption>,
}

#[derive(Debug, Serialize)]
pub struct AiMenuDietOption {
    pub name: String,
    pub ingredients: Vec<String>,
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct ResponseItem {
    pub dish_id: String,
    pub reason: String,
}

pub async fn select_dish(
    date: NaiveDate,
    dish_items: &Vec<DishItem>,
    last_days_choices: &HashMap<String, CalendarDayItems>,
) -> eyre::Result<AiResponse> {
    let client = Client::new();

    let mut dish_item_name = HashMap::new();
    let mut dish_name = HashMap::new();

    let mut properties = serde_json::Map::new();
    for dish_item in dish_items {
        let dish_item_id = dish_item.id.clone();
        dish_item_name.insert(dish_item_id.clone(), dish_item.meal_type.name.clone());
        for dish in &dish_item.options() {
            dish_name.insert(dish.dish.id.clone(), dish.name.clone());
        }
        let dish_item_schema = json!({
            "type": "object",
            "properties": {
                "reason": { "type": "string", "description": "Why this dish is selected" },
                "dish_id": { "type": "string", "enum": dish_item.options().iter().map(|dish| dish.dish.id.clone()).collect::<Vec<String>>() },
            },
            "required": ["reason", "dish_id"],
            "additionalProperties": false
        });
        properties.insert(dish_item_id, dish_item_schema);
    }

    let schema = json!({
        "type": "object",
        "properties": {
            "reasoning": {
                "type": "array",
                "description": "Think about what the user might like and why",
                "items": {
                    "type": "string",
                }
            },
            "selections": {
                "type": "object",
                "properties": properties,
                "required": dish_items.iter().map(|dish_item| dish_item.id.clone()).collect::<Vec<String>>(),
                "additionalProperties": false
            }
        },
        "required": ["reasoning", "selections"],
        "additionalProperties": false
    });

    tracing::info!("Schema: {}", serde_json::to_string_pretty(&schema)?);
    let response_format = ResponseFormat::JsonSchema {
        json_schema: ResponseFormatJsonSchema {
            description: None,
            name: "meal_selection".into(),
            schema: Some(schema),
            strict: Some(true),
        },
    };

    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(2048u32)
        .model("gpt-4o-mini")
        .temperature(0.0)
        .messages([
            ChatCompletionRequestSystemMessage::from(
                "You are personal meal assistant. You have to select meals for the user. Try to figure out what the user wants to eat from the menu. Justify why user should prefer one meal over another."
            )
            .into(),
            ChatCompletionRequestUserMessage::from(serde_json::to_string(&SelectDishQuestion{
                menu_date: date,
                dish_items: dish_items.iter().map(|dish_item| AiDishItem {
                    id: dish_item.id.clone(),
                    meal_type: dish_item.meal_type.name.clone(),
                    options: dish_item.options().iter().map(|dish| AiMenuDietOption {
                        name: dish.name.clone(),
                        ingredients: dish.ingredients.clone(),
                        id: dish.dish.id.clone(),
                    }).collect(),
                }).collect(),
                user_changes: Preferences::get_preferences(),
                last_days_choices: last_days_choices.iter().map(|(day, menu)| {
                    (day.clone(), menu.diet_elements.members.iter().map(|dish_item| {
                        let options = dish_item.options();
                        let dish = options.iter().find(|option| option.is_default).expect("one must dish must be selected");
                        AiMenuDietOption {
                            name: dish.name.clone(),
                            ingredients: dish.ingredients.clone(),
                            id: dish.dish.id.clone(),
                        }
                    }).collect())
                }).collect(),
            }).unwrap()).into(),
        ])
        .response_format(response_format)
        .build()?;

    let response = client.chat().create(request).await?;

    if let Some(choice) = response.choices.first() {
        if let Some(content) = &choice.message.content {
            // println!("{}\n\n\n\n", content);
            let response: AiResponse = serde_json::from_str(content).wrap_err("in ai response")?;
            Ok(response)
            // for reason in &response.reasoning {
            //     println!("Ai: {}", reason);
            // }
            // println!("\n");
            // for (dish_item_id, dish) in response.selections {
            //     println!(
            //         "{}: {}\n   reason: {}",
            //         dish_item_name
            //             .get(&dish_item_id)
            //             .unwrap_or(&"invalid".to_string()),
            //         dish_name
            //             .get(&dish.dish_id)
            //             .unwrap_or(&"invalid".to_string()),
            //         dish.reason,
            //     );
            // }
        } else {
            eyre::bail!("No content in response from AI");
        }
    } else {
        eyre::bail!("No response from AI");
    }
}
