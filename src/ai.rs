use std::collections::HashMap;

use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestSystemMessage, ChatCompletionRequestUserMessage,
        CreateChatCompletionRequestArgs, ReasoningEffort, ResponseFormat, ResponseFormatJsonSchema,
    },
    Client,
};
use chrono::NaiveDate;
use dialoguer::{theme::ColorfulTheme, FuzzySelect, Input};
use eyre::{Context, Report, Result};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{preferences::{AiConfig, Preferences}, CalendarDayItems, DishItem};

#[derive(Debug, Serialize)]
pub struct SelectDishQuestion {
    pub user_preferences: String,
    pub last_days_choices: IndexMap<String, Vec<AiMenuDietOption>>,
    pub dish_items: Vec<AiDishItem>,
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
    pub analysis: HashMap<String, String>,
}

async fn fetch_models(client: &Client<OpenAIConfig>) -> Result<Vec<String>> {
    match client.models().list().await {
        Ok(models) => {
            let model_names: Vec<String> = models.data
                .into_iter()
                .map(|model| model.id)
                .collect();
            Ok(model_names)
        },
        Err(err) => {
            eprintln!("Failed to fetch models: {err}");
            Err(Report::new(err))
        }
    }
}

// Helper function to prompt user for model selection or input
fn get_model_input(models: Vec<String>) -> Result<String> {
    if models.is_empty() {
        Input::<String>::new()
            .with_prompt("Enter model name")
            .default("gpt-4o-2024-08-06".to_string())
            .interact()
            .map_err(Into::into)
    } else {
        let filtered_items = FuzzySelect::with_theme(&ColorfulTheme::default())
            .with_prompt("Select a model (type to filter)")
            .default(0)
            .items(&models)
            .interact()?;
        Ok(models[filtered_items].clone())
    }
}

// Main configuration function now async
pub async fn configure_ai() -> Result<AiConfig> {
    println!("AI Configuration Setup");

    let api_base = Input::<String>::new()
        .with_prompt("Enter API base URL (e.g., https://api.openai.com/v1 or your LiteLLM server URL)")
        .default("https://api.openai.com/v1".to_string())
        .interact()?;

    let api_key = Input::<String>::new()
        .with_prompt("Enter API key")
        .interact()?;

    // Create a temporary client to fetch models
    let temp_config = OpenAIConfig::new()
        .with_api_base(&api_base)
        .with_api_key(&api_key);
    let client = Client::with_config(temp_config);

    // Try to fetch models
    let models = match fetch_models(&client).await {
        Ok(models) => models,
        Err(_) => {
            println!("Could not fetch models list, please enter model name manually.");
            Vec::new() // Empty vec will trigger manual input
        }
    };

    // Now get user input for model selection or manual entry
    let model = get_model_input(models)?;

    let config = AiConfig {
        api_base,
        api_key,
        model,
    };

    Preferences::save_ai_config(config.clone());
    println!("AI configuration saved");

    Ok(config)
}

fn get_openai_client(config: &AiConfig) -> Client<OpenAIConfig> {
    let openai_config = OpenAIConfig::new()
        .with_api_base(&config.api_base)
        .with_api_key(&config.api_key);

    Client::with_config(openai_config)
}

pub async fn select_dish(
    date: NaiveDate,
    dish_items: &Vec<DishItem>,
    last_days_choices: &IndexMap<String, CalendarDayItems>,
    user_preferences: &str,
) -> eyre::Result<AiResponse> {
    // Get or configure AI settings
    let ai_config = match Preferences::ai_config() {
        Some(config) => config,
        None => configure_ai().await?,
    };

    let client = get_openai_client(&ai_config);

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
                "analysis": {
                    "type": "object",
                    "description": "Analyze available options and argue how good it is for the user",
                    "properties": dish_item.options().iter().map(|dish| (dish.dish.id.clone(), json!({
                        "type": "string",
                    }))).collect::<serde_json::Map<_,_>>(),
                    "required": dish_item.options().iter().map(|dish| dish.dish.id.clone()).collect::<Vec<String>>(),
                    "additionalProperties": false
                },
                "reason": { "type": "string", "description": "Justification why this meal should fit user preferences" },
                "dish_id": { "type": "string", "enum": dish_item.options().iter().map(|dish| dish.dish.id.clone()).collect::<Vec<String>>() },
            },
            "required": ["analysis", "reason", "dish_id"],
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

    let question = SelectDishQuestion {
        user_preferences: user_preferences.to_string(),
        last_days_choices: last_days_choices.iter().map(|(day, menu)| {
            (day.clone(), menu.diet_elements.members.iter().map(|dish_item| {
                let dish = dish_item.get_selected_option().expect("No selected option");
                AiMenuDietOption {
                    name: dish.name.clone(),
                    ingredients: dish.ingredients.as_ref().map(|i| i.ingredients.clone()).unwrap_or_default(),
                    id: dish.dish.id.clone(),
                }
            }).collect())
        }).collect(),
        dish_items: dish_items.iter().map(|dish_item| AiDishItem {
            id: dish_item.id.clone(),
            meal_type: dish_item.meal_type.name.clone(),
            options: dish_item.options().iter().map(|dish| AiMenuDietOption {
                name: dish.name.clone(),
                ingredients: dish.ingredients.as_ref().map(|i| i.ingredients.clone()).unwrap_or_default(),
                id: dish.dish.id.clone(),
            }).collect(),
        }).collect(),
        menu_date: date,
    };

    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(1024u32 * 40)
        .model(&ai_config.model)
        .reasoning_effort(ReasoningEffort::High)
        .messages([
            ChatCompletionRequestSystemMessage::from(
                "You are personal meal assistant. You have to select meals for the user. Figure out what the user wants to eat from the menu. Use historic data to figure out user preferences. Try not to pick the same meal as the user had in the last days.",
            )
            .into(),
            ChatCompletionRequestUserMessage::from(serde_json::to_string(&question).unwrap()).into(),
        ])
        .response_format(response_format)
        .build()?;

    // Retry logic for handling empty responses
    let mut retries = 0;
    const MAX_RETRIES: u32 = 5;

    loop {
        let response = client.chat().create(request.clone()).await?;

        if let Some(choice) = response.choices.first() {
            if let Some(content) = &choice.message.content {
                if content.trim().is_empty() {
                    retries += 1;
                    if retries <= MAX_RETRIES {
                        tracing::warn!("Received empty response from AI, retrying... ({}/{})", retries, MAX_RETRIES);
                        continue;
                    }
                    eyre::bail!("Received empty response from AI after {} retries", MAX_RETRIES);
                }

                let response: AiResponse = serde_json::from_str(content).wrap_err(format!("in ai response: {content}"))?;
                return Ok(response);
            }
            eyre::bail!("No content in response from AI");
        }
        eyre::bail!("No response from AI");
    }
}
pub async fn ai_generate_preferences(
    adjustments: &[UserAdjustment],
    cfg: &AiConfig,
) -> eyre::Result<String> {
    use async_openai::types::*;
    let client = get_openai_client(cfg);

    let system_prompt = r#"
You are a dietary preference analyzer. Synthesize the provided list of user adjustments into a
coherent, natural language description of dietary preferences. Guidelines:

- Write in first-person perspective ("I prefer...", "I avoid...")
- Group similar preferences together (allergies, preferences, restrictions)
- Infer dietary patterns from recurring adjustments
- Keep it concise (4-8 bullet points or short paragraphs)
- Include specific foods to avoid or prefer
- Mention reasoning when explicitly provided
- Format naturally without technical formatting (JSON, markdown, etc.)

Output ONLY the preference description without additional commentary."#;

    let user = format!(
        "Convert these historical meal adjustments into user-friendly dietary preferences:\n\n{}",
        serde_json::to_string_pretty(adjustments)?
    );

    let mut attempts = 0;
    const MAX_ATTEMPTS: u8 = 3;

    while attempts < MAX_ATTEMPTS {
        let req = CreateChatCompletionRequestArgs::default()
            .model(&cfg.model)
            .messages([
                ChatCompletionRequestSystemMessage::from(system_prompt).into(),
                ChatCompletionRequestUserMessage::from(user.as_str()).into(),
            ])
            .max_tokens(1024u32 * 40)
            .reasoning_effort(ReasoningEffort::High)
            .build()?;

        match client.chat().create(req).await {
            Ok(resp) => {
                return resp.choices
                    .first()
                    .and_then(|c| c.message.content.clone())
                    .filter(|s| !s.trim().is_empty())
                    .ok_or_else(|| eyre::eyre!("Empty AI response"));
            }
            Err(e) => {
                attempts += 1;
                if attempts < MAX_ATTEMPTS {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    continue;
                }
                return Err(e).wrap_err("AI service failed after retries");
            }
        }
    }
    unreachable!()
}
