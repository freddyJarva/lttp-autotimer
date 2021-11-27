use chrono::serde::ts_milliseconds;
use std::borrow::Borrow;

use chrono::{DateTime, TimeZone, Utc};
use serde::Serialize;

use crate::{check::Check, transition::Transition};

/// Struct used for serializing different types of checks into the same csv format.
/// Events include transitions, checking locations (e.g. chests), and getting items
#[derive(Serialize, Debug, PartialEq)]
pub struct Event {
    #[serde(with = "ts_milliseconds")]
    timestamp: DateTime<Utc>,
    indoors: Option<bool>,
    to: Option<String>,
    location_id: Option<String>,
    item_id: Option<String>,
}

impl From<&Transition> for Event {
    fn from(transition: &Transition) -> Self {
        Event {
            timestamp: transition
                .timestamp
                .expect("Found transition missing timestamp when serializing"),
            indoors: Some(transition.indoors),
            to: Some(transition.name.to_string()),
            location_id: None,
            item_id: None,
        }
    }
}

impl From<&mut Transition> for Event {
    fn from(transition: &mut Transition) -> Self {
        Event {
            timestamp: transition
                .timestamp
                .expect("Found transition missing timestamp when serializing"),
            indoors: Some(transition.indoors),
            to: Some(transition.name.to_string()),
            location_id: None,
            item_id: None,
        }
    }
}

impl<T> From<T> for Event
where
    T: Borrow<Check>,
{
    fn from(check: T) -> Self {
        let check: &Check = check.borrow();
        let timestamp = check
            .time_of_check
            .expect("Found check missing timestamp when serializing");
        if check.is_item && !check.is_progressive {
            Event {
                timestamp,
                indoors: None,
                to: None,
                location_id: None,
                item_id: Some(check.name.to_string()),
            }
        } else if check.is_item && check.is_progressive {
            Event {
                timestamp,
                indoors: None,
                to: None,
                location_id: None,
                item_id: Some(format!("{} - {}", check.name, check.progressive_level)),
            }
        } else {
            Event {
                timestamp,
                indoors: None,
                to: None,
                location_id: Some(check.name.to_string()),
                item_id: match &check.item {
                    Some(item) => Some(item.to_string()),
                    None => None,
                },
            }
        }
    }
}

impl Default for Event {
    fn default() -> Self {
        Self {
            timestamp: chrono::Utc.timestamp_millis(0),
            indoors: Default::default(),
            to: Default::default(),
            location_id: Default::default(),
            item_id: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    macro_rules! convert_to_event {
        ($($name:ident: $values:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (from_struct, expected) = $values;
                    assert_eq!(Event::from(&from_struct), expected)
                }
            )*
        };
    }

    convert_to_event! {
        from_location_check: (
            Check {
                name: "Mushroom".to_string(),
                address: 0x180013,
                dunka_offset: 0x411,
                dunka_mask: 0x10,
                time_of_check: Some(Utc.timestamp_millis(200)),
                ..Default::default()
            },
            Event {
                location_id: Some("Mushroom".to_string()),
                timestamp: Utc.timestamp_millis(200),
                ..Default::default()
            }
        ),
        from_normal_item_check: (
            Check {
                name: "Hookshot".to_string(),
                address: 0x0,
                dunka_offset: 0x342,
                dunka_mask: 0x01,
                time_of_check: Some(Utc.timestamp_millis(200)),
                is_item: true,
                ..Default::default()
            },
            Event {
                item_id: Some("Hookshot".to_string()),
                timestamp: Utc.timestamp_millis(200),
                ..Default::default()
            }
        ),
        from_progressive_item_check: (
            Check {
                name: "Progressive Sword".to_string(),
                address: 0x0,
                dunka_offset: 0x342,
                dunka_mask: 0x01,
                time_of_check: Some(Utc.timestamp_millis(200)),
                is_item: true,
                is_progressive: true,
                progressive_level: 3,
                ..Default::default()
            },
            Event {
                item_id: Some("Progressive Sword - 3".to_string()),
                timestamp: Utc.timestamp_millis(200),
                ..Default::default()
            }
        ),
        from_transition: (
            Transition {
                name: "Lala".to_string(),
                timestamp: Some(Utc.timestamp_millis(200)),
                ..Default::default()
            },
            Event {
                to: Some("Lala".to_string()),
                timestamp: Utc.timestamp_millis(200),
                indoors: Some(false),
                ..Default::default()
            }
        ),
    }
}
