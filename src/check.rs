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
    pub fn mark_as_checked(&mut self) {
        self.is_checked = true;
        self.time_of_check = Some(Utc::now())
    }

    pub fn progress_item(&mut self, snes_value: u8) {
        self.progressive_level += 1;
        self.snes_value = snes_value;
        self.time_of_check = Some(Utc::now());
    }
}

#[cfg(test)]
mod tests {
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
            deserialize_event_checks().unwrap()[0],
            Check {
                name: "Save & Quit".to_string(),
                is_progressive: true,
                sram_offset: Some(0xf42d),
                sram_mask: Some(0xff),
                ..Default::default()
            }
        )
    }
}
