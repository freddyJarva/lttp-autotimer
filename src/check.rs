use crate::condition::Conditions;
use crate::serde_lttp::hex_byte_deserialize_option;
use crate::serde_lttp::hex_deserialize_option;
use chrono::serde::ts_milliseconds_option;
use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Default, Clone)]
pub struct Check {
    pub id: usize,
    pub name: String,
    #[serde(default)]
    #[serde(deserialize_with = "hex_deserialize_option")]
    pub sram_offset: Option<u32>,
    #[serde(default)]
    #[serde(deserialize_with = "hex_byte_deserialize_option")]
    pub sram_mask: Option<u8>,
    #[serde(default)]
    pub is_checked: bool,
    #[serde(with = "ts_milliseconds_option", default)]
    pub time_of_check: Option<DateTime<Utc>>,
    pub item: Option<String>,
    #[serde(default)]
    pub is_progressive: bool,
    #[serde(default)]
    pub progressive_level: usize,
    #[serde(default)]
    pub snes_value: u8,
    #[serde(default)]
    pub is_item: bool,
    pub conditions: Option<Vec<Conditions>>,
}

static CHECKS_JSON: &'static str = include_str!("checks.json");
static ITEMS_JSON: &'static str = include_str!("items.json");
static EVENTS_JSON: &'static str = include_str!("events.json");

/// Reads src/checks.json and returns deserialized content
pub fn deserialize_location_checks() -> Result<Vec<Check>, serde_json::Error> {
    serde_json::from_str(CHECKS_JSON)
}

/// Reads src/checks.json and returns deserialized content, setting `is_item` to `true` if not already set.
pub fn deserialize_item_checks() -> Result<Vec<Check>, serde_json::Error> {
    serde_json::from_str(ITEMS_JSON).map(|items: Vec<Check>| {
        items
            .into_iter()
            .map(|mut item| {
                item.is_item = true;
                item
            })
            .collect()
    })
}

/// Reads src/events.json and returns deserialized content
pub fn deserialize_event_checks() -> Result<Vec<Check>, serde_json::Error> {
    serde_json::from_str(EVENTS_JSON)
}

impl Check {
    pub fn new(id: usize) -> Self {
        Check {
            id,
            ..Default::default()
        }
    }

    /// Handle updating values of the check
    pub fn update_check(&mut self, time_of_check: &DateTime<Utc>) {
        if self.is_progressive {
            self.progressive_level += 1
        } else {
            self.is_checked = true;
        }
        self.time_of_check = Some(time_of_check.clone())
    }

    pub fn mark_as_checked(&mut self, time_of_check: &DateTime<Utc>) {
        self.is_checked = true;
        self.time_of_check = Some(time_of_check.clone())
    }

    pub fn progress_item(&mut self, snes_value: u8, time_of_check: &DateTime<Utc>) {
        self.progressive_level += 1;
        self.snes_value = snes_value;
        self.time_of_check = Some(time_of_check.clone());
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use crate::assert_attrs;

    use super::*;
    #[test]
    fn test_deserialize_location_checks() {
        assert_eq!(
            deserialize_location_checks().unwrap()[0],
            Check {
                name: "Mushroom".to_string(),
                sram_offset: Some(0xf411),
                sram_mask: Some(0x10),
                ..Default::default()
            }
        )
    }

    #[test]
    fn test_deserialize_item_checks() {
        assert_eq!(
            deserialize_item_checks().unwrap()[0],
            Check {
                name: "Bow".to_string(),
                sram_offset: Some(0xf38e),
                sram_mask: Some(0x80),
                is_item: true,
                ..Default::default()
            }
        )
    }

    #[test]
    fn test_deserialize_event_checks() {
        assert_eq!(
            deserialize_event_checks().unwrap()[3],
            Check {
                id: 2,
                name: "Overworld Mirror".to_string(),
                is_progressive: true,
                sram_offset: Some(0xf43a),
                sram_mask: Some(0xff),
                ..Default::default()
            }
        )
    }

    #[test]
    fn GIVEN_normal_check_WHEN_update_check_THEN_is_checked_is_set_to_true() {
        // Given
        let mut check = Check {
            ..Default::default()
        };
        let time_of_check = &Utc::now();
        // When
        check.update_check(time_of_check);
        // Then
        assert_attrs! {check: is_checked == true, time_of_check == Some(*time_of_check),}
    }

    #[test]
    fn GIVEN_progressive_WHEN_update_check_THEN_increment_progressive_level() {
        // Given
        let mut check = Check {
            is_progressive: true,
            ..Default::default()
        };
        let time_of_check = &Utc::now();
        // When
        check.update_check(time_of_check);
        // Then
        assert_attrs! {check: is_checked == false, progressive_level == 1, time_of_check == Some(*time_of_check),}
    }
}
