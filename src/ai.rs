use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestSystemMessage, ChatCompletionRequestUserMessage,
        CreateChatCompletionRequestArgs, ReasoningEffort, ResponseFormat, ResponseFormatJsonSchema,
    },
    Client,
};
use chrono::{NaiveDate, Utc};
use dialoguer::{theme::ColorfulTheme, FuzzySelect, Input};
use eyre::{Context, Report, Result};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::HashMap,
    fs::{create_dir_all, OpenOptions},
    io::Write,
    path::PathBuf,
};

use crate::{
    preferences::{AiConfig, Preferences},
    prompts, CalendarDayItems, DishItem,
};

// Candidate dish with explicit named fields (replaces previous tuple alias)
#[derive(Debug, Clone)]
struct CandidateDish {
    _dish_id: String,
    name: String,
    ingredients: Vec<String>,
}

// Mapping: dish_item_id -> list of candidate dishes
type DishItemCandidates = HashMap<String, Vec<CandidateDish>>;

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

    if dish_items.is_empty() {
        return Err(eyre::eyre!("No dish items provided to select_dish function"));
    }

    let mut dish_item_name = HashMap::new();
    let mut dish_name = HashMap::new();
    // dish_item_id -> list of (dish_id, dish_name, ingredients)
    let mut dish_item_candidates: DishItemCandidates = HashMap::new();

    let mut properties = serde_json::Map::new();
    for dish_item in dish_items {
        let dish_item_id = dish_item.id.clone();
        dish_item_name.insert(dish_item_id.clone(), dish_item.meal_type.name.clone());
        let mut candidates: Vec<CandidateDish> = Vec::new();
        for dish in &dish_item.options() {
            dish_name.insert(dish.dish.id.clone(), dish.name.clone());
            candidates.push(CandidateDish {
                _dish_id: dish.dish.id.clone(),
                name: dish.name.clone(),
                ingredients: dish
                    .ingredients
                    .as_ref()
                    .map(|i| i.ingredients.clone())
                    .unwrap_or_default(),
            });
        }
        dish_item_candidates.insert(dish_item_id.clone(), candidates);
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

    let system_prompt = prompts::build_meal_selection_system_prompt(
        user_preferences,
        !last_days_choices.is_empty(),
    );


    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(1024u32 * 40)
        .model(&ai_config.model)
        // .reasoning_effort(ReasoningEffort::High)
        .messages([
            ChatCompletionRequestSystemMessage::from(system_prompt).into(),
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

                // Log human readable AI response; ignore logging errors
                if let Err(e) = log_ai_response(
                    date,
                    &response,
                    &dish_item_name,
                    &dish_name,
                    &dish_item_candidates,
                ) {
                    tracing::warn!(error = ?e, "Failed to write AI response log");
                }
                return Ok(response);
            }
            eyre::bail!("No content in response from AI");
        }
        eyre::bail!("No response from AI");
    }
}

fn ai_log_path() -> Option<PathBuf> {
    // Determine a sensible per-user log location.
    // Prefer XDG_STATE_HOME, fallback to ~/.local/state
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".local").join("state")))?;
    Some(base.join("powermeal-ai-choice").join("ai_responses.log"))
}

fn log_ai_response(
    date: NaiveDate,
    response: &AiResponse,
    dish_item_name: &HashMap<String, String>,
    dish_name: &HashMap<String, String>,
    dish_item_candidates: &DishItemCandidates,
) -> eyre::Result<()> {
    let path = match ai_log_path() {
        Some(p) => p,
        None => return Ok(()), // silently ignore if we cannot determine path
    };
    if let Some(parent) = path.parent() {
        create_dir_all(parent)?;
    }
    let mut f = OpenOptions::new().create(true).append(true).open(&path)?;
    writeln!(
        f,
        "=== AI Meal Selection {} (generated {}) ===",
        date,
        Utc::now().to_rfc3339()
    )?;
    if !response.reasoning.is_empty() {
        writeln!(f, "Reasoning:")?;
        for line in &response.reasoning {
            writeln!(f, "- {}", line.trim())?;
        }
    }
    writeln!(f, "Selections:")?;
    for (dish_item_id, selection) in &response.selections {
        let meal_type = dish_item_name
            .get(dish_item_id)
            .map(|s| s.as_str())
            .unwrap_or("");
        let human_dish_name = dish_name
            .get(&selection.dish_id)
            .map(|s| s.as_str())
            .unwrap_or(&selection.dish_id);
        writeln!(f, "[{meal_type}] {human_dish_name}",)?;
        // Log candidate dishes (all available options for this dish item)
        if let Some(cands) = dish_item_candidates.get(dish_item_id) {
            if !cands.is_empty() {
                writeln!(f, "  Candidates:")?;
                for cand in cands {
                    writeln!(f, "    - {}", cand.name)?;
                    if !cand.ingredients.is_empty() {
                        writeln!(f, "      Ingredients: {}", cand.ingredients.join(", "))?;
                    }
                }
            }
        }
        writeln!(f, "  Reason: {}", selection.reason.trim())?;
        if !selection.analysis.is_empty() {
            writeln!(f, "  Analysis:")?;
            for (opt_id, text) in &selection.analysis {
                let opt_name = dish_name.get(opt_id).map(|s| s.as_str()).unwrap_or(opt_id);
                writeln!(f, "    - {}: {}", opt_name, text.trim())?;
            }
        }
    }
    writeln!(f)?; // blank line separator
    Ok(())
}

pub async fn ai_generate_preferences(
    adjustments: &[UserAdjustment],
    cfg: &AiConfig,
) -> eyre::Result<String> {
    use async_openai::types::*;
    let client = get_openai_client(cfg);

    let system_prompt = prompts::PREFERENCE_SUMMARY_SYSTEM_PROMPT;

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
