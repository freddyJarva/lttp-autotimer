use crate::condition::Conditions;
use crate::serde_lttp::{hex_byte_deserialize, hex_deserialize};
use chrono::serde::ts_milliseconds_option;
use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Default, Clone)]
pub struct Check {
    pub name: String,
    #[serde(deserialize_with = "hex_deserialize")]
    pub sram_offset: u32,
    #[serde(deserialize_with = "hex_byte_deserialize")]
    pub sram_mask: u8,
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

impl Check {
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
                sram_offset: 0xf411,
                sram_mask: 0x10,
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
                sram_offset: 0xf38e,
                sram_mask: 0x80,
                is_item: true,
                ..Default::default()
            }
        )
    }
}
