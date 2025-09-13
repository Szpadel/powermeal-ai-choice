//! Client-side validation for menu selection availability
//!
//! This module handles the validation of whether menu selection is available
//! for a given delivery date based on delivery configuration rules.
//! All times are handled in Europe/Warsaw timezone.

use chrono::{Datelike, Duration, NaiveDate, NaiveTime, TimeZone, Weekday};
use chrono_tz::Europe::Warsaw;
use eyre::{bail, eyre, Result};

use crate::serde::{ClientDietItem, DeliveryConfig};

/// Check if menu selection is available for a specific diet day
///
/// Returns true if the cutoff time has not passed for menu selection
pub fn is_menu_selection_available(
    diet_item: &ClientDietItem,
    delivery_config: &DeliveryConfig,
) -> Result<bool> {
    let delivery_date = NaiveDate::parse_from_str(&diet_item.date_dlv, "%Y-%m-%d")
        .map_err(|e| eyre!("Failed to parse delivery date '{}': {}", diet_item.date_dlv, e))?;

    let time_remaining = calculate_time_remaining(&delivery_date, delivery_config)?;
    Ok(time_remaining > Duration::zero())
}

/// Calculate time remaining until cutoff for a delivery date
///
/// Returns positive duration if time remains, negative if cutoff has passed
fn calculate_time_remaining(
    delivery_date: &NaiveDate,
    config: &DeliveryConfig,
) -> Result<Duration> {
    // Get current time in Warsaw timezone
    let now = chrono::Utc::now().with_timezone(&Warsaw);
    calculate_time_remaining_with_time(delivery_date, config, now)
}

/// Internal function for calculating time remaining with a specific time (for testing)
fn calculate_time_remaining_with_time(
    delivery_date: &NaiveDate,
    config: &DeliveryConfig,
    now: chrono::DateTime<chrono_tz::Tz>,
) -> Result<Duration> {
    // Get the day of week for the delivery date
    let delivery_day_id = get_day_id(delivery_date.weekday());

    // Find the menu selection rule for this delivery day
    let rule = find_menu_selection_rule(config, delivery_day_id)
        .ok_or_else(|| eyre!("No menu selection rule found for delivery day {}", delivery_day_id))?;

    // Parse the cutoff time
    let cutoff_time = match &rule.delv_time {
        Some(time) => parse_cutoff_time(time)?,
        None => bail!("Menu selection rule has no cutoff time specified"),
    };

    // Calculate the actual cutoff date
    // The cutoff is on delv_day_id (which may be different from delivery day)
    let cutoff_date = calculate_cutoff_date(delivery_date, delivery_day_id, rule.delv_day_id);

    // Combine date and time with Warsaw timezone
    let cutoff_datetime = Warsaw
        .from_local_datetime(&cutoff_date.and_time(cutoff_time))
        .single()
        .ok_or_else(|| eyre!("Failed to create cutoff datetime in Warsaw timezone"))?;

    // Return the difference
    Ok(cutoff_datetime.signed_duration_since(now))
}

/// Find the menu selection rule for a given delivery day
fn find_menu_selection_rule(config: &DeliveryConfig, delivery_day_id: i32) -> Option<&crate::serde::DeliveryRule> {
    config.data.delivery
        .iter()
        .find(|r| r.delv_type_id == 5 && r.day_id == delivery_day_id)
}

/// Convert chrono Weekday to API day_id format
///
/// API uses: 1=Sunday, 2=Monday, ..., 7=Saturday
fn get_day_id(weekday: Weekday) -> i32 {
    match weekday {
        Weekday::Sun => 1,
        Weekday::Mon => 2,
        Weekday::Tue => 3,
        Weekday::Wed => 4,
        Weekday::Thu => 5,
        Weekday::Fri => 6,
        Weekday::Sat => 7,
    }
}

/// Parse cutoff time string (format: "HH:MM" or "HH:MM:SS")
fn parse_cutoff_time(time_str: &str) -> Result<NaiveTime> {
    // Try parsing with seconds first, then without
    NaiveTime::parse_from_str(time_str, "%H:%M:%S")
        .or_else(|_| NaiveTime::parse_from_str(time_str, "%H:%M"))
        .map_err(|e| eyre!("Failed to parse cutoff time '{}': {}", time_str, e))
}

/// Calculate the actual cutoff date based on delivery date and day IDs
///
/// The cutoff happens on `cutoff_day_id` before the `delivery_date`
fn calculate_cutoff_date(delivery_date: &NaiveDate, delivery_day_id: i32, cutoff_day_id: i32) -> NaiveDate {
    // Calculate how many days before delivery the cutoff is
    let days_diff = if cutoff_day_id <= delivery_day_id {
        delivery_day_id - cutoff_day_id
    } else {
        // Cutoff is in the previous week
        7 - (cutoff_day_id - delivery_day_id)
    };

    // Subtract days from delivery date
    *delivery_date - Duration::days(days_diff as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDateTime, Timelike};
    use crate::serde::{DeliveryData, DeliveryRule};

    #[test]
    fn test_get_day_id() {
        assert_eq!(get_day_id(Weekday::Sun), 1);
        assert_eq!(get_day_id(Weekday::Mon), 2);
        assert_eq!(get_day_id(Weekday::Tue), 3);
        assert_eq!(get_day_id(Weekday::Wed), 4);
        assert_eq!(get_day_id(Weekday::Thu), 5);
        assert_eq!(get_day_id(Weekday::Fri), 6);
        assert_eq!(get_day_id(Weekday::Sat), 7);
    }

    #[test]
    fn test_parse_cutoff_time() {
        // Test with seconds
        let time1 = parse_cutoff_time("14:30:00").unwrap();
        assert_eq!(time1.hour(), 14);
        assert_eq!(time1.minute(), 30);
        assert_eq!(time1.second(), 0);

        // Test without seconds
        let time2 = parse_cutoff_time("09:15").unwrap();
        assert_eq!(time2.hour(), 9);
        assert_eq!(time2.minute(), 15);

        // Test invalid format
        assert!(parse_cutoff_time("invalid").is_err());
    }

    #[test]
    fn test_calculate_cutoff_date() {
        // Test same week cutoff
        let delivery = NaiveDate::from_ymd_opt(2025, 1, 17).unwrap(); // Friday (day_id = 6)
        let cutoff = calculate_cutoff_date(&delivery, 6, 4); // Cutoff on Wednesday
        assert_eq!(cutoff, NaiveDate::from_ymd_opt(2025, 1, 15).unwrap());

        // Test previous week cutoff
        let delivery = NaiveDate::from_ymd_opt(2025, 1, 13).unwrap(); // Monday (day_id = 2)
        let cutoff = calculate_cutoff_date(&delivery, 2, 6); // Cutoff on Friday (previous week)
        assert_eq!(cutoff, NaiveDate::from_ymd_opt(2025, 1, 10).unwrap());
    }

    fn get_real_delivery_config() -> DeliveryConfig {
        DeliveryConfig {
            data: DeliveryData {
                delivery: vec![
                    // Sunday delivery -> Thursday cutoff
                    DeliveryRule {
                        day_id: 1,
                        delv_type_id: 5,
                        delv_day_id: 5,
                        delv_time: Some("05:00:00".to_string()),
                    },
                    // Monday delivery -> Saturday cutoff
                    DeliveryRule {
                        day_id: 2,
                        delv_type_id: 5,
                        delv_day_id: 7,
                        delv_time: Some("05:00:00".to_string()),
                    },
                    // Tuesday delivery -> Sunday cutoff
                    DeliveryRule {
                        day_id: 3,
                        delv_type_id: 5,
                        delv_day_id: 1,
                        delv_time: Some("05:00:00".to_string()),
                    },
                    // Wednesday delivery -> Monday cutoff
                    DeliveryRule {
                        day_id: 4,
                        delv_type_id: 5,
                        delv_day_id: 2,
                        delv_time: Some("05:00:00".to_string()),
                    },
                    // Thursday delivery -> Tuesday cutoff
                    DeliveryRule {
                        day_id: 5,
                        delv_type_id: 5,
                        delv_day_id: 3,
                        delv_time: Some("05:00:00".to_string()),
                    },
                    // Friday delivery -> Wednesday cutoff
                    DeliveryRule {
                        day_id: 6,
                        delv_type_id: 5,
                        delv_day_id: 4,
                        delv_time: Some("05:00:00".to_string()),
                    },
                    // Saturday delivery -> Thursday cutoff
                    DeliveryRule {
                        day_id: 7,
                        delv_type_id: 5,
                        delv_day_id: 5,
                        delv_time: Some("05:00:00".to_string()),
                    },
                ]
            },
        }
    }

    #[test]
    fn test_real_data_with_time() {
        let delivery_date = NaiveDate::from_ymd_opt(2025, 9, 16).unwrap();
        let config = get_real_delivery_config();

        // Monday delivery (day_id=2) has cutoff on Saturday (delv_day_id=7) at 05:00
        // For Sept 16 (Monday), cutoff is Sept 14 (Saturday) at 05:00

        // To have 3-4 hours remaining, we need to be around Sept 14 at 01:00-02:00
        let mock_time_naive = NaiveDateTime::parse_from_str(
            "2025-09-14 01:30:00",
            "%Y-%m-%d %H:%M:%S"
        ).unwrap();

        let mock_time = Warsaw
            .from_local_datetime(&mock_time_naive)
            .single()
            .unwrap();

        // Calculate time remaining with mocked time
        let time_remaining = calculate_time_remaining_with_time(
            &delivery_date,
            &config,
            mock_time
        ).unwrap();

        let hours = time_remaining.num_hours();
        let minutes = time_remaining.num_minutes() % 60;

        println!("Time remaining: {} hours {} minutes", hours, minutes);

        // Should be approximately 3.5 hours (3 hours 30 minutes)
        assert_eq!(hours, 3);
        assert_eq!(minutes, 30);
        assert!(hours >= 3 && hours <= 4,
                "Expected 3-4 hours remaining, got {} hours {} minutes",
                hours, minutes);
    }

    #[test]
    fn test_real_data_menu_selection_available() {
        // Test with actual recording time where menu selection should be available
        let delivery_date = NaiveDate::from_ymd_opt(2025, 9, 16).unwrap(); // Monday
        let config = get_real_delivery_config();

        // Mock the actual recording time
        let recording_time_naive = NaiveDateTime::parse_from_str(
            "2025-09-14 01:18:30",  // Saturday morning, before 05:00 cutoff
            "%Y-%m-%d %H:%M:%S"
        ).unwrap();

        let recording_time = Warsaw
            .from_local_datetime(&recording_time_naive)
            .single()
            .unwrap();

        // Calculate time remaining
        let time_remaining = calculate_time_remaining_with_time(
            &delivery_date,
            &config,
            recording_time
        ).unwrap();

        // Should be positive (about 3h 41min 30s remaining)
        assert!(time_remaining > Duration::zero(),
                "Expected positive time remaining, got {:?}",
                time_remaining);

        let hours = time_remaining.num_hours();
        let minutes = (time_remaining.num_minutes() % 60) as i32;

        // Should be approximately 3 hours 41 minutes
        assert!(hours == 3 && minutes >= 40 && minutes <= 42,
                "Expected ~3h 41min remaining, got {}h {}min",
                hours, minutes);
    }

    #[test]
    fn test_all_weekday_delivery_rules() {
        let config = get_real_delivery_config();

        // Test that all weekdays have menu selection rules
        // Based on actual rules extracted from debug.har
        let expected_mappings = vec![
            (1, Some(5)), // Sunday delivery -> Thursday cutoff
            (2, Some(7)), // Monday delivery -> Saturday cutoff
            (3, Some(1)), // Tuesday delivery -> Sunday cutoff
            (4, Some(2)), // Wednesday delivery -> Monday cutoff
            (5, Some(3)), // Thursday delivery -> Tuesday cutoff
            (6, Some(4)), // Friday delivery -> Wednesday cutoff
            (7, Some(5)), // Saturday delivery -> Thursday cutoff
        ];

        for (delivery_day_id, expected_cutoff_day) in expected_mappings {
            let rule = find_menu_selection_rule(&config, delivery_day_id);

            if let Some(expected) = expected_cutoff_day {
                assert!(rule.is_some(),
                        "Expected rule for delivery day {}", delivery_day_id);
                assert_eq!(rule.unwrap().delv_day_id, expected,
                          "Wrong cutoff day for delivery day {}", delivery_day_id);
            } else {
                assert!(rule.is_none(),
                        "Unexpected rule for delivery day {}", delivery_day_id);
            }
        }
    }

    #[test]
    fn test_cutoff_time_calculation_for_different_days() {
        let config = get_real_delivery_config();

        // Test various scenarios with specific dates
        // Based on actual rules from test.har
        let test_cases = vec![
            // (delivery_date, expected_cutoff_date)
            ("2025-09-14", "2025-09-11"), // Sunday (1) -> Thursday (5) cutoff: 7-(5-1)=3 days before
            ("2025-09-15", "2025-09-13"), // Monday (2) -> Saturday (7) cutoff: 7-(7-2)=2 days before
            ("2025-09-16", "2025-09-14"), // Tuesday (3) -> Sunday (1) cutoff: 3-1=2 days before
            ("2025-09-17", "2025-09-15"), // Wednesday (4) -> Monday (2) cutoff: 4-2=2 days before
            ("2025-09-18", "2025-09-16"), // Thursday (5) -> Tuesday (3) cutoff: 5-3=2 days before
            ("2025-09-19", "2025-09-17"), // Friday (6) -> Wednesday (4) cutoff: 6-4=2 days before
            ("2025-09-20", "2025-09-18"), // Saturday (7) -> Thursday (5) cutoff: 7-5=2 days before
        ];

        for (delivery_str, expected_cutoff_str) in test_cases {
            let delivery_date = NaiveDate::parse_from_str(delivery_str, "%Y-%m-%d").unwrap();
            let expected_cutoff = NaiveDate::parse_from_str(expected_cutoff_str, "%Y-%m-%d").unwrap();

            let delivery_day_id = get_day_id(delivery_date.weekday());

            if let Some(rule) = find_menu_selection_rule(&config, delivery_day_id) {
                let calculated_cutoff = calculate_cutoff_date(
                    &delivery_date,
                    delivery_day_id,
                    rule.delv_day_id
                );

                assert_eq!(calculated_cutoff, expected_cutoff,
                          "Wrong cutoff date for delivery on {}", delivery_str);
            }
        }
    }
}
