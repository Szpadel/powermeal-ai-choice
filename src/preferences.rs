use std::path::PathBuf;

use chrono::{DateTime, Local, NaiveDate, TimeZone};
use serde::{Deserialize, Serialize};

use crate::ai::UserAdjustment;

const PREFERENCES_FILE: &str = ".config/powermeal-ai/preferences.json";

#[derive(Debug, Deserialize, Serialize)]
pub struct Preferences {
    adjustments: Vec<UserAdjustment>,
    last_day_selected: Option<NaiveDate>,
    token: Option<String>,
}

impl Preferences {
    pub fn add_new_preferences(adjustment: Vec<UserAdjustment>) {
        let mut preferences = Self::load_preferences();
        preferences.adjustments.extend(adjustment);
        // limit to last 100 adjustments
        if preferences.adjustments.len() > 100 {
            preferences
                .adjustments
                .drain(..preferences.adjustments.len() - 100);
        }
        preferences.save_preferences();
    }

    pub fn get_preferences() -> Vec<UserAdjustment> {
        Self::load_preferences().adjustments
    }

    pub fn next_day_to_check() -> Option<DateTime<Local>> {
        Self::load_preferences()
            .last_day_selected
            .map(|d| Local.from_local_datetime(&d.into()).unwrap())
    }

    pub fn set_next_day_to_check(date: NaiveDate) {
        let mut preferences = Self::load_preferences();
        preferences.last_day_selected = Some(date);
        preferences.save_preferences();
    }

    fn load_preferences() -> Self {
        let path = Self::config_path();
        if path.exists() {
            let file = std::fs::File::open(path).unwrap();
            let reader = std::io::BufReader::new(file);
            let preferences: Preferences = serde_json::from_reader(reader).unwrap();
            preferences
        } else {
            Preferences {
                adjustments: Vec::new(),
                last_day_selected: None,
                token: None,
            }
        }
    }

    pub fn save_token(token: &str) {
        let mut preferences = Self::load_preferences();
        preferences.token = Some(token.to_string());
        preferences.save_preferences();
    }

    pub fn token() -> Option<String> {
        Self::load_preferences().token
    }

    fn save_preferences(self) {
        let path = Self::config_path();
        if !path.exists() {
            std::fs::create_dir_all(path.parent().unwrap()).expect("Failed to create directory");
        }
        let file = std::fs::File::create(path).unwrap();
        let writer = std::io::BufWriter::new(file);
        serde_json::to_writer(writer, &self).unwrap();
    }

    fn config_path() -> PathBuf {
        std::env::var("HOME")
            .expect("HOME not set")
            .parse::<PathBuf>()
            .expect("invalid HOME")
            .join(PREFERENCES_FILE)
    }
}
