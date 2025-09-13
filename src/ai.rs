//! AI integration module for intelligent meal selection using OpenAI/LiteLLM.
//!
//! This module provides the core AI functionality for PowerMeal, enabling intelligent
//! meal selection based on user preferences, dietary requirements, and meal history.
//! It integrates with OpenAI-compatible APIs (including LiteLLM servers) to provide
//! personalized meal recommendations.
//!
//! # Features
//!
//! - Interactive AI service configuration with model discovery
//! - Intelligent meal selection using structured JSON schemas
//! - Preference migration from legacy adjustment records
//! - Comprehensive logging of AI decisions for transparency
//! - Retry logic for handling API failures
//!
//! # Architecture
//!
//! The module uses OpenAI's chat completion API with structured output formats
//! to ensure consistent and parseable responses. It generates complex JSON schemas
//! that guide the AI to analyze each meal option and provide reasoned selections.

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

/// Internal representation of a candidate dish for AI selection.
///
/// Used internally to track available meal options during the selection process.
#[derive(Debug, Clone)]
struct CandidateDish {
    _dish_id: String,
    name: String,
    ingredients: Vec<String>,
}

/// Mapping from dish item ID to list of candidate dishes.
///
/// Used internally to organize available options for each meal slot.
type DishItemCandidates = HashMap<String, Vec<CandidateDish>>;

/// Structured request format for AI meal selection.
///
/// This structure is serialized to JSON and sent to the AI model as part of the
/// user message. It provides complete context for meal selection including user
/// preferences, recent meal history, and available options.
///
/// # Fields
///
/// * `user_preferences` - Natural language description of dietary preferences and requirements
/// * `last_days_choices` - Recent meal selections indexed by date for variety considerations
/// * `dish_items` - Available meal slots with their respective options
/// * `menu_date` - The date for which meals are being selected
///
/// # Example
///
/// ```json
/// {
///   "user_preferences": "I prefer vegetarian meals with lots of protein",
///   "last_days_choices": {
///     "2025-01-11": [{"name": "Grilled Tofu", "ingredients": [...]}]
///   },
///   "dish_items": [{"id": "lunch", "meal_type": "Lunch", "options": [...]}],
///   "menu_date": "2025-01-12"
/// }
/// ```
#[derive(Debug, Serialize)]
pub struct SelectDishQuestion {
    pub user_preferences: String,
    pub last_days_choices: IndexMap<String, Vec<AiMenuDietOption>>,
    pub dish_items: Vec<AiDishItem>,
    pub menu_date: NaiveDate,
}

/// Legacy user adjustment record for meal changes.
///
/// Represents historical meal adjustments made by users, used during migration
/// to generate natural language preferences from past behavior.
///
/// # Fields
///
/// * `from` - Original meal that was replaced
/// * `to` - New meal that was selected
/// * `reason` - Optional explanation for the change
/// * `date` - Date when the adjustment was made
#[derive(Debug, Deserialize, Serialize)]
pub struct UserAdjustment {
    pub from: String,
    pub to: String,
    pub reason: Option<String>,
    pub date: NaiveDate,
}

/// AI response containing meal selections with reasoning.
///
/// This structure represents the complete response from the AI model after
/// analyzing meal options. It includes both the reasoning process and the
/// final selections for each meal slot.
///
/// # Fields
///
/// * `reasoning` - Step-by-step thought process explaining the selection logic
/// * `selections` - Map from dish item ID to selected meal with detailed analysis
///
/// # Example
///
/// ```json
/// {
///   "reasoning": [
///     "User prefers vegetarian options",
///     "Need to ensure variety from yesterday's meals"
///   ],
///   "selections": {
///     "lunch_slot": {
///       "dish_id": "tofu_bowl_123",
///       "reason": "High protein vegetarian option",
///       "analysis": {...}
///     }
///   }
/// }
/// ```
#[derive(Debug, Deserialize)]
pub struct AiResponse {
    pub reasoning: Vec<String>,
    pub selections: HashMap<String, ResponseItem>,
}

/// Meal slot with available options for AI selection.
///
/// Represents a single meal slot (e.g., lunch, dinner) with all available
/// dish options that the AI can choose from.
///
/// # Fields
///
/// * `id` - Unique identifier for this meal slot
/// * `meal_type` - Human-readable meal type (e.g., "Lunch", "Dinner")
/// * `options` - List of available dishes for this slot
#[derive(Debug, Serialize)]
pub struct AiDishItem {
    pub id: String,
    pub meal_type: String,
    pub options: Vec<AiMenuDietOption>,
}

/// Individual meal option with ingredients for AI analysis.
///
/// Represents a single dish option that can be selected for a meal slot,
/// including its ingredients for dietary analysis.
///
/// # Fields
///
/// * `name` - Human-readable dish name
/// * `ingredients` - List of ingredients for dietary analysis
/// * `id` - Unique identifier for this dish
#[derive(Debug, Serialize)]
pub struct AiMenuDietOption {
    pub name: String,
    pub ingredients: Vec<String>,
    pub id: String,
}

/// Individual meal selection with detailed analysis.
///
/// Contains the AI's selection for a single meal slot, including the chosen
/// dish and detailed reasoning about why it was selected over alternatives.
///
/// # Fields
///
/// * `dish_id` - ID of the selected dish
/// * `reason` - Concise justification for this selection
/// * `analysis` - Detailed analysis of each available option (dish_id -> analysis text)
///
/// # Example
///
/// ```json
/// {
///   "dish_id": "salad_123",
///   "reason": "Light, nutritious option with varied vegetables",
///   "analysis": {
///     "salad_123": "Excellent choice with fresh vegetables and protein",
///     "pasta_456": "Too heavy after yesterday's carb-rich meal"
///   }
/// }
/// ```
#[derive(Debug, Deserialize)]
pub struct ResponseItem {
    pub dish_id: String,
    pub reason: String,
    pub analysis: HashMap<String, String>,
}

/// Fetches available models from the AI service.
///
/// # Arguments
///
/// * `client` - OpenAI client configured with API credentials
///
/// # Returns
///
/// Returns a vector of available model IDs, or error if the API call fails.
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

/// Prompts user for model selection or manual input.
///
/// # Arguments
///
/// * `models` - List of available models (empty triggers manual input)
///
/// # Returns
///
/// Returns the selected or manually entered model name.
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

/// Interactive AI service configuration with model discovery.
///
/// Guides the user through setting up AI service connection, including API endpoint,
/// authentication, and model selection. Attempts to fetch available models from the
/// service for easy selection, falling back to manual entry if the list cannot be retrieved.
///
/// # Returns
///
/// Returns `AiConfig` containing the configured API connection details and selected model.
///
/// # Errors
///
/// * Returns error if user input fails (e.g., terminal not available)
/// * Returns error if configuration cannot be saved
///
/// # Example
///
/// ```rust
/// // Interactive configuration flow:
/// // 1. User enters API base URL (defaults to OpenAI)
/// // 2. User enters API key
/// // 3. System fetches available models
/// // 4. User selects model from list or enters manually
/// let config = configure_ai().await?;
/// println!("Using model: {}", config.model);
/// ```
///
/// # Notes
///
/// The configuration is automatically saved to user preferences for future use.
/// Supports both OpenAI API and LiteLLM-compatible endpoints.
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

/// Creates an OpenAI client from configuration.
///
/// # Arguments
///
/// * `config` - AI configuration with API credentials and endpoint
///
/// # Returns
///
/// Returns configured OpenAI client ready for API calls.
fn get_openai_client(config: &AiConfig) -> Client<OpenAIConfig> {
    let openai_config = OpenAIConfig::new()
        .with_api_base(&config.api_base)
        .with_api_key(&config.api_key);

    Client::with_config(openai_config)
}

/// Core AI meal selection with preferences and history analysis.
///
/// Performs intelligent meal selection by analyzing user preferences, recent meal history,
/// and available options. Uses structured JSON schemas to ensure the AI provides detailed
/// analysis and reasoning for each selection.
///
/// # Arguments
///
/// * `date` - The date for which meals are being selected
/// * `dish_items` - Available meal slots with their respective dish options
/// * `last_days_choices` - Recent meal selections for variety and pattern analysis
/// * `user_preferences` - Natural language description of dietary preferences
///
/// # Returns
///
/// Returns `AiResponse` containing:
/// - Reasoning steps explaining the selection logic
/// - Selected dish for each meal slot with detailed justification
/// - Comparative analysis of all available options
///
/// # Errors
///
/// * Returns error if no dish items are provided
/// * Returns error if AI configuration is missing and cannot be created
/// * Returns error if API call fails after retries
/// * Returns error if response parsing fails
///
/// # Example
///
/// ```rust
/// let response = select_dish(
///     NaiveDate::from_ymd(2025, 1, 12),
///     &dish_items,
///     &last_7_days,
///     "I prefer vegetarian meals with high protein"
/// ).await?;
///
/// for (slot_id, selection) in &response.selections {
///     println!("{}: Selected {} because {}", 
///              slot_id, selection.dish_id, selection.reason);
/// }
/// ```
///
/// # Implementation Details
///
/// 1. Builds complex JSON schema defining expected response structure
/// 2. Includes retry logic (up to 5 attempts) for handling empty responses
/// 3. Logs human-readable summaries of AI decisions to state directory
/// 4. Uses structured output format to ensure consistent responses
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

/// Determines the path for AI response logging.
///
/// # Returns
///
/// Returns the log file path following XDG standards, or None if home directory cannot be determined.
fn ai_log_path() -> Option<PathBuf> {
    // Determine a sensible per-user log location.
    // Prefer XDG_STATE_HOME, fallback to ~/.local/state
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".local").join("state")))?;
    Some(base.join("powermeal-ai-choice").join("ai_responses.log"))
}

/// Logs AI meal selection response in human-readable format.
///
/// Creates a detailed log of AI decisions including reasoning, selections,
/// and analysis of all available options. Logs are appended to a persistent
/// file for transparency and debugging.
///
/// # Arguments
///
/// * `date` - Date for which meals were selected
/// * `response` - AI response with selections and reasoning
/// * `dish_item_name` - Mapping of dish item IDs to human-readable names
/// * `dish_name` - Mapping of dish IDs to human-readable names
/// * `dish_item_candidates` - All available options for each meal slot
///
/// # Returns
///
/// Returns Ok(()) on success, or error if file operations fail.
///
/// # Notes
///
/// Logs are stored in `~/.local/state/powermeal-ai-choice/ai_responses.log`
/// following XDG base directory standards.
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

/// Converts legacy meal adjustments to natural language preferences.
///
/// Analyzes historical meal adjustment records to generate a natural language
/// description of user preferences. Used during migration from the old adjustment-based
/// system to the new preference-based system.
///
/// # Arguments
///
/// * `adjustments` - Historical meal adjustments showing user's past choices
/// * `cfg` - AI configuration for API connection
///
/// # Returns
///
/// Returns a natural language string describing inferred dietary preferences
/// based on the pattern of historical adjustments.
///
/// # Errors
///
/// * Returns error if API call fails after retries
/// * Returns error if response is empty
/// * Returns error if JSON serialization fails
///
/// # Example
///
/// ```rust
/// let adjustments = vec![
///     UserAdjustment {
///         from: "Beef Stew".to_string(),
///         to: "Vegetable Curry".to_string(),
///         reason: Some("Avoiding red meat".to_string()),
///         date: NaiveDate::from_ymd(2025, 1, 10),
///     },
/// ];
///
/// let preferences = ai_generate_preferences(&adjustments, &config).await?;
/// // Returns: "Based on your history, you prefer vegetarian options and avoid red meat..."
/// ```
///
/// # Notes
///
/// - Uses high reasoning effort for better preference inference
/// - Includes retry logic (3 attempts) with exponential backoff
/// - Typically used once during user preference migration
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
