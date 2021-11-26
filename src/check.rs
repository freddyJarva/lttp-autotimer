use serde::{de::Error, Deserialize, Deserializer};

/// Terrible deserializer of hex values. But hey, it works. I guess.
fn hex_deserialize<'de, D>(d: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(d)?;
    u32::from_str_radix(&s[2..], 16).map_err(D::Error::custom)
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Check {
    pub name: String,
    #[serde(deserialize_with = "hex_deserialize")]
    pub address: u32,
    #[serde(deserialize_with = "hex_deserialize")]
    player_address: u32,
    crystal: String,
    hint_text: String,
    #[serde(deserialize_with = "hex_deserialize")]
    dunka_offset: u32,
    #[serde(deserialize_with = "hex_deserialize")]
    dunka_mask: u32,
}

static CHECKS_JSON: &'static str = include_str!("checks.json");

/// Reads src/checks.json and returns deserialized content
pub fn deserialize_checks() -> Result<Vec<Check>, serde_json::Error> {
    serde_json::from_str(CHECKS_JSON)
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
                player_address: 0x186338,
                crystal: "False".to_string(),
                hint_text: "in the woods".to_string(),
                dunka_offset: 0x0,
                dunka_mask: 0x10,
            }
        )
    }
}
