//! User preferences and configuration management.
//!
//! This module handles persistent storage of user preferences, AI configuration,
//! and meal selection progress tracking. It provides migration support from legacy
//! adjustment-based formats to the current free-form preference text format.
//!
//! # Storage Location
//!
//! Preferences are stored at `~/.config/powermeal-ai/preferences.json` following
//! the XDG Base Directory specification.
//!
//! # Migration Support
//!
//! The module automatically detects legacy preference formats (adjustment arrays)
//! and provides methods to migrate them to the current free-form text format.

use std::path::PathBuf;

use chrono::{DateTime, Local, NaiveDate, TimeZone};
use serde::{Deserialize, Serialize};

use crate::ai::UserAdjustment;

/// Path to the preferences file relative to the user's home directory.
const PREFERENCES_FILE: &str = ".config/powermeal-ai/preferences.json";

/// Main preferences structure containing user dietary preferences and configuration.
///
/// This struct serves as the central configuration store for the application,
/// including user dietary preferences, AI service configuration, authentication tokens,
/// and meal selection progress tracking.
///
/// # Serialization
///
/// The structure is designed to be backward-compatible with older versions.
/// Empty fields are skipped during serialization to keep the JSON clean.
///
/// # Examples
///
/// ```no_run
/// let prefs = Preferences::load_preferences();
/// 
/// // Check if migration is needed
/// if prefs.needs_migration() {
///     // Perform migration with edited text
///     prefs.complete_migration(edited_text);
/// }
/// 
/// // Access user preferences
/// println!("User preferences: {}", prefs.user_preferences);
/// ```
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Preferences {
    /// Free-form text describing user's dietary preferences.
    /// This replaces the legacy adjustment-based system.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub user_preferences: String,

    /// Legacy field: structured adjustments from the old preference system.
    /// Kept for backward compatibility and migration purposes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub adjustments: Vec<UserAdjustment>,

    /// Tracks the last day for which meals were selected.
    /// Used to determine the next day requiring meal selection.
    last_day_selected: Option<NaiveDate>,
    
    /// Authentication token for PowerMeal API access.
    token: Option<String>,
    
    /// Configuration for AI service integration.
    pub ai_config: Option<AiConfig>,
}

/// Configuration for AI service integration.
///
/// Contains all necessary parameters to connect to an AI service
/// for meal selection assistance. Supports various OpenAI-compatible APIs.
///
/// # Examples
///
/// ```no_run
/// let config = AiConfig {
///     api_base: "https://api.openai.com/v1".to_string(),
///     api_key: "sk-...".to_string(),
///     model: "gpt-4-turbo-preview".to_string(),
/// };
/// Preferences::save_ai_config(config);
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AiConfig {
    /// Base URL for the AI API endpoint.
    pub api_base: String,
    /// API key for authentication.
    pub api_key: String,
    /// Model identifier to use for requests.
    pub model: String,
}

impl Preferences {
    /// Returns the next day that requires meal selection.
    ///
    /// Determines the next day to check based on the last selected day.
    /// If the last selected day is in the past, returns the current date.
    /// This helps track progress through meal selection over multiple days.
    ///
    /// # Returns
    ///
    /// * `Some(DateTime<Local>)` - The next day to check for meal selection
    /// * `None` - If no previous selection has been made
    pub fn next_day_to_check() -> Option<DateTime<Local>> {
        let now = Local::now();
        Self::load_preferences()
            .last_day_selected
            .map(|d| {
                let date = Local.from_local_datetime(&d.into()).unwrap();
                if date < now {
                    now
                } else {
                    date
                }
            })
    }

    /// Updates the last selected day for progress tracking.
    ///
    /// Records that meal selection has been completed up to the specified date.
    /// This information is persisted and used by `next_day_to_check()` to determine
    /// where to continue meal selection.
    ///
    /// # Arguments
    ///
    /// * `date` - The date up to which meals have been selected
    pub fn set_next_day_to_check(date: NaiveDate) {
        let mut preferences = Self::load_preferences();
        preferences.last_day_selected = Some(date);
        preferences.save_preferences();
    }

    /// Loads preferences from persistent storage.
    ///
    /// Reads the preferences JSON file from disk and deserializes it.
    /// If the file doesn't exist or contains invalid data, returns default preferences.
    ///
    /// # Returns
    ///
    /// The loaded preferences or default values if loading fails.
    ///
    /// # Panics
    ///
    /// Panics if the config file exists but cannot be opened (permissions issue).
    pub fn load_preferences() -> Self {
        let path = Self::config_path();
        if path.exists() {
            let file = std::fs::File::open(&path).expect("Failed to open config file");
            let reader = std::io::BufReader::new(file);
            serde_json::from_reader(reader).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    /// Saves the current preferences to persistent storage.
    ///
    /// Serializes the preferences to JSON and writes them to disk.
    /// Creates the parent directory if it doesn't exist.
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - The config directory cannot be created
    /// - The config file cannot be written
    /// - The preferences cannot be serialized to JSON
    pub fn save_preferences(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("Failed to create config directory");
        }
        let file = std::fs::File::create(&path).expect("Failed to create config file");
        let writer = std::io::BufWriter::new(file);
        serde_json::to_writer(writer, self).expect("Failed to serialize preferences");
    }

    /// Saves AI configuration to preferences.
    ///
    /// This is a convenience method that loads current preferences,
    /// updates the AI config, and saves everything back to disk.
    ///
    /// # Arguments
    ///
    /// * `config` - The AI configuration to save
    ///
    /// # Thread Safety
    ///
    /// This method is not thread-safe. Concurrent calls may result in
    /// lost updates if multiple threads modify preferences simultaneously.
    pub fn save_ai_config(config: AiConfig) {
        let mut preferences = Self::load_preferences();
        preferences.ai_config = Some(config);
        preferences.save_preferences();
    }

    /// Retrieves the current AI configuration.
    ///
    /// Loads preferences from disk and returns the AI config if present.
    ///
    /// # Returns
    ///
    /// * `Some(AiConfig)` - If AI configuration has been set
    /// * `None` - If no AI configuration exists
    pub fn ai_config() -> Option<AiConfig> {
        Self::load_preferences().ai_config
    }

    /// Saves the authentication token to preferences.
    ///
    /// This is a convenience method that loads current preferences,
    /// updates the token, and saves everything back to disk.
    ///
    /// # Arguments
    ///
    /// * `token` - The authentication token to save
    ///
    /// # Security Note
    ///
    /// The token is stored in plain text in the preferences file.
    /// Ensure appropriate file permissions are set on the config directory.
    pub fn save_token(token: &str) {
        let mut preferences = Self::load_preferences();
        preferences.token = Some(token.to_string());
        preferences.save_preferences();
    }

    /// Retrieves the stored authentication token.
    ///
    /// Loads preferences from disk and returns the token if present.
    ///
    /// # Returns
    ///
    /// * `Some(String)` - If a token has been saved
    /// * `None` - If no token exists
    pub fn token() -> Option<String> {
        Self::load_preferences().token
    }

    /// Returns the full path to the preferences file.
    ///
    /// Constructs the path based on the user's HOME environment variable.
    ///
    /// # Returns
    ///
    /// The absolute path to the preferences JSON file.
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - The HOME environment variable is not set
    /// - The HOME value cannot be parsed as a valid path
    fn config_path() -> PathBuf {
        std::env::var("HOME")
            .expect("HOME not set")
            .parse::<PathBuf>()
            .expect("invalid HOME")
            .join(PREFERENCES_FILE)
    }
}
