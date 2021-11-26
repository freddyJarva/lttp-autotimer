use crate::serde_lttp::{hex_byte_deserialize, hex_deserialize};
use chrono::serde::ts_milliseconds_option;
use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
pub struct Check {
    pub name: String,
    #[serde(deserialize_with = "hex_deserialize")]
    pub address: u32,
    player_address: Option<String>,
    crystal: Option<String>,
    hint_text: Option<String>,
    #[serde(deserialize_with = "hex_deserialize")]
    pub dunka_offset: u32,
    #[serde(deserialize_with = "hex_byte_deserialize")]
    pub dunka_mask: u8,
    #[serde(default)]
    pub is_checked: bool,
    #[serde(with = "ts_milliseconds_option", default)]
    pub time_of_check: Option<DateTime<Utc>>,
    pub item: Option<String>,
}

static CHECKS_JSON: &'static str = include_str!("checks.json");

/// Reads src/checks.json and returns deserialized content
pub fn deserialize_checks() -> Result<Vec<Check>, serde_json::Error> {
    serde_json::from_str(CHECKS_JSON)
}

impl Check {
    pub fn mark_as_checked(&mut self) {
        self.is_checked = true;
        self.time_of_check = Some(Utc::now())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_deserialize_checks() {
        assert_eq!(
            deserialize_checks().unwrap()[0],
            Check {
                name: "Mushroom".to_string(),
                address: 0x180013,
                player_address: Some("0x186338".to_string()),
                crystal: Some("False".to_string()),
                hint_text: Some("in the woods".to_string()),
                dunka_offset: 0x411,
                dunka_mask: 0x10,
                is_checked: false,
                time_of_check: None,
                item: None,
            }
        )
    }
}
