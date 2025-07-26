use std::path::PathBuf;

use chrono::{DateTime, Local, NaiveDate, TimeZone};
use serde::{Deserialize, Serialize};

use crate::ai::UserAdjustment;

const PREFERENCES_FILE: &str = ".config/powermeal-ai/preferences.json";

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Preferences {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub user_preferences: String,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub adjustments: Vec<UserAdjustment>,

    last_day_selected: Option<NaiveDate>,
    token: Option<String>,
    pub ai_config: Option<AiConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AiConfig {
    pub api_base: String,
    pub api_key: String,
    pub model: String,
}

impl Preferences {
    /// Checks if legacy data needs migration to the new format
    pub fn needs_migration(&self) -> bool {
        !self.adjustments.is_empty() && self.user_preferences.trim().is_empty()
    }

    /// Completes the migration by saving edited text and clearing legacy data
    pub fn complete_migration(&mut self, edited_text: String) {
        if !edited_text.trim().is_empty() {
            self.user_preferences = edited_text.trim().to_string();
            self.adjustments.clear();
            self.save_preferences();
        }
    }

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

    pub fn set_next_day_to_check(date: NaiveDate) {
        let mut preferences = Self::load_preferences();
        preferences.last_day_selected = Some(date);
        preferences.save_preferences();
    }

    /// Loads preferences from disk
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

    /// Saves preferences to disk
    pub fn save_preferences(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("Failed to create config directory");
        }
        let file = std::fs::File::create(&path).expect("Failed to create config file");
        let writer = std::io::BufWriter::new(file);
        serde_json::to_writer(writer, self).expect("Failed to serialize preferences");
    }

    pub fn save_ai_config(config: AiConfig) {
        let mut preferences = Self::load_preferences();
        preferences.ai_config = Some(config);
        preferences.save_preferences();
    }

    pub fn ai_config() -> Option<AiConfig> {
        Self::load_preferences().ai_config
    }

    pub fn save_token(token: &str) {
        let mut preferences = Self::load_preferences();
        preferences.token = Some(token.to_string());
        preferences.save_preferences();
    }

    pub fn token() -> Option<String> {
        Self::load_preferences().token
    }

    fn config_path() -> PathBuf {
        std::env::var("HOME")
            .expect("HOME not set")
            .parse::<PathBuf>()
            .expect("invalid HOME")
            .join(PREFERENCES_FILE)
    }
}
