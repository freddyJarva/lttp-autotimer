use chrono::serde::ts_milliseconds;
use std::borrow::Borrow;

use chrono::{DateTime, TimeZone, Utc};
use serde::Serialize;

use crate::{check::Check, transition::Tile};

pub trait EventLog {
    fn latest_transition(&self) -> Option<Tile>;
    fn latest_location_check(&self) -> Option<Check>;
    fn latest_item_get(&self) -> Option<Check>;
    fn latest_other_event(&self) -> Option<Check>;
}

impl EventLog for EventTracker {
    fn latest_transition(&self) -> Option<Tile> {
        self.log
            .iter()
            .rev()
            .find(|event| {
                if let EventEnum::Transition(_) = event {
                    true
                } else {
                    false
                }
            })
            .map(|event| {
                if let EventEnum::Transition(t) = event {
                    t.clone()
                } else {
                    panic!("This should never happen")
                }
            })
    }

    fn latest_location_check(&self) -> Option<Check> {
        self.log
            .iter()
            .rev()
            .find(|event| {
                if let EventEnum::LocationCheck(_) = event {
                    true
                } else {
                    false
                }
            })
            .map(|event| {
                if let EventEnum::LocationCheck(t) = event {
                    t.clone()
                } else {
                    panic!("This should never happen")
                }
            })
    }

    fn latest_item_get(&self) -> Option<Check> {
        self.log
            .iter()
            .rev()
            .find(|event| {
                if let EventEnum::ItemGet(_) = event {
                    true
                } else {
                    false
                }
            })
            .map(|event| {
                if let EventEnum::ItemGet(t) = event {
                    t.clone()
                } else {
                    panic!("This should never happen")
                }
            })
    }

    fn latest_other_event(&self) -> Option<Check> {
        self.log
            .iter()
            .rev()
            .find(|event| {
                if let EventEnum::Other(_) = event {
                    true
                } else {
                    false
                }
            })
            .map(|event| {
                if let EventEnum::Other(t) = event {
                    t.clone()
                } else {
                    panic!("This should never happen")
                }
            })
    }
}

#[derive(Debug, Clone)]
pub enum EventEnum {
    Transition(Tile),
    LocationCheck(Check),
    ItemGet(Check),
    Other(Check),
}

pub struct EventTracker {
    log: Vec<EventEnum>,
}

impl EventTracker {
    /// Sets an initial 'auto timer start' Transition with values of `0`.
    ///
    /// This is necessary for the absolute first transition check to work,
    /// as it needs a value to compare to to see if a transition triggered.
    pub fn new() -> Self {
        Self {
            log: vec![EventEnum::Transition(Tile {
                id: 9000,
                name: "AUTO_TIMER_START".to_string(),
                address_value: vec![0x0],
                timestamp: Some(Utc::now()),
                indoors: false,
                conditions: None,
                region: "START".to_string(),
            })],
        }
    }

    pub fn push(&mut self, event: EventEnum) {
        self.log.push(event)
    }
}

impl From<Vec<EventEnum>> for EventTracker {
    fn from(log: Vec<EventEnum>) -> Self {
        EventTracker { log }
    }
}

/// Struct used for serializing different types of checks into the same csv format.
/// Events include transitions, checking locations (e.g. chests), and getting items
#[derive(Serialize, Debug, PartialEq)]
pub struct Event {
    #[serde(with = "ts_milliseconds")]
    timestamp: DateTime<Utc>,
    #[serde(skip_serializing)]
    indoors: Option<bool>,
    transition_id: Option<usize>,
    location_id: Option<usize>,
    item_id: Option<usize>,
    event_id: Option<usize>,
}

impl From<&Tile> for Event {
    fn from(transition: &Tile) -> Self {
        Event {
            timestamp: transition
                .timestamp
                .expect("Found transition missing timestamp when serializing"),
            indoors: Some(transition.indoors),
            transition_id: Some(transition.id),
            location_id: None,
            item_id: None,
            ..Default::default()
        }
    }
}

impl From<&mut Tile> for Event {
    fn from(transition: &mut Tile) -> Self {
        Event {
            timestamp: transition
                .timestamp
                .expect("Found transition missing timestamp when serializing"),
            indoors: Some(transition.indoors),
            transition_id: Some(transition.id),
            location_id: None,
            item_id: None,
            ..Default::default()
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
        if check.is_item {
            Event {
                timestamp,
                item_id: Some(check.id),
                ..Default::default()
            }
        } else {
            Event {
                timestamp,
                location_id: Some(check.id),
                ..Default::default()
            }
        }
    }
}

impl From<&EventEnum> for Event {
    fn from(event: &EventEnum) -> Self {
        match event {
            EventEnum::Transition(t) => Event::from(t),
            EventEnum::LocationCheck(check) => Event {
                timestamp: check.time_of_check.unwrap(),
                location_id: Some(check.id),
                ..Default::default()
            },
            EventEnum::ItemGet(check) => Event {
                timestamp: check.time_of_check.unwrap(),
                item_id: Some(check.id),
                ..Default::default()
            },
            EventEnum::Other(check) => Event {
                timestamp: check.time_of_check.unwrap(),
                event_id: Some(check.id),
                ..Default::default()
            },
        }
    }
}

impl Default for Event {
    fn default() -> Self {
        Self {
            timestamp: chrono::Utc.timestamp_millis(0),
            indoors: Default::default(),
            transition_id: Default::default(),
            location_id: Default::default(),
            item_id: Default::default(),
            event_id: Default::default(),
        }
    }
}

#[allow(non_snake_case)]
#[cfg(test)]
mod tests {

    use crate::assert_attrs;

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
                id: 0,
                name: "Mushroom".to_string(),
                sram_offset: 0x411,
                sram_mask: 0x10,
                time_of_check: Some(Utc.timestamp_millis(200)),
                ..Default::default()
            },
            Event {
                location_id: Some(0),
                timestamp: Utc.timestamp_millis(200),
                ..Default::default()
            }
        ),
        from_normal_item_check: (
            Check {
                id: 4,
                name: "Hookshot".to_string(),
                sram_offset: 0x342,
                sram_mask: 0x01,
                time_of_check: Some(Utc.timestamp_millis(200)),
                is_item: true,
                ..Default::default()
            },
            Event {
                item_id: Some(4),
                timestamp: Utc.timestamp_millis(200),
                ..Default::default()
            }
        ),
        from_progressive_item_check: (
            Check {
                id: 27,
                name: "Progressive Sword".to_string(),
                sram_offset: 0x342,
                sram_mask: 0x01,
                time_of_check: Some(Utc.timestamp_millis(200)),
                is_item: true,
                is_progressive: true,
                progressive_level: 3,
                ..Default::default()
            },
            Event {
                item_id: Some(27),
                timestamp: Utc.timestamp_millis(200),
                ..Default::default()
            }
        ),
        from_transition: (
            Tile {
                id: 1337,
                region: "A great region".to_string(),
                timestamp: Some(Utc.timestamp_millis(200)),
                ..Default::default()
            },
            Event {
                transition_id: Some(1337),
                timestamp: Utc.timestamp_millis(200),
                indoors: Some(false),
                ..Default::default()
            }
        ),
    }

    fn event_log() -> Vec<EventEnum> {
        vec![
            EventEnum::ItemGet(Check {
                name: "nope".to_string(),
                ..Default::default()
            }),
            EventEnum::Transition(Tile {
                name: "not latest".to_string(),
                ..Default::default()
            }),
            EventEnum::LocationCheck(Check {
                name: "meh".to_string(),
                ..Default::default()
            }),
            EventEnum::Transition(Tile {
                name: "latest".to_string(),
                ..Default::default()
            }),
            EventEnum::LocationCheck(Check {
                name: "latest location check".to_string(),
                ..Default::default()
            }),
            EventEnum::ItemGet(Check {
                name: "latest item get".to_string(),
                ..Default::default()
            }),
        ]
    }

    macro_rules! test_eventlog {
        ($($name:ident: $function:ident: $values:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (event_log, expected) = $values;
                    let event_tracker = EventTracker::from(event_log);
                    assert_eq!(event_tracker.$function(), expected)
                }
            )*
        };
    }

    test_eventlog! {
        latest_transition: latest_transition: (event_log(), Some(Tile {
            name: "latest".to_string(),
            ..Default::default()
        })),
        latest_location_check: latest_location_check: (event_log(), Some(Check {
            name: "latest location check".to_string(),
            ..Default::default()
        })),
        latest_item_get: latest_item_get: (event_log(), Some(Check {
            name: "latest item get".to_string(),
            ..Default::default()
        })),
        GIVEN_no_transitions_THEN_return_none: latest_transition: (vec![], None),
        GIVEN_no_location_checks_THEN_return_none: latest_location_check: (vec![], None),
        GIVEN_no_item_get_THEN_return_none: latest_item_get: (vec![], None),
    }

    #[test]
    fn new_event_tracker_is_initialized_with_start_transition() {
        let last_transition = EventTracker::new().latest_transition().unwrap();
        assert_attrs! {
            last_transition: address_value == [0x0],
            name == "AUTO_TIMER_START",
            indoors == false,
        };
    }
}
