use crate::serde_lttp::coordinate_deserialize;
use std::collections::HashMap;

use crate::{serde_lttp::hex_16bit_deserialize, SnesMemoryID};

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::snes::NamedAddresses;

pub fn overworld_transition<T, U>(previous_res: T, response: U) -> bool
where
    T: NamedAddresses,
    U: NamedAddresses,
{
    previous_res.overworld_tile() != response.overworld_tile()
}

pub fn entrance_transition<T, U>(previous_res: T, response: U) -> bool
where
    T: NamedAddresses,
    U: NamedAddresses,
{
    previous_res.indoors() != response.indoors()
}

static TRANSITIONS_JSON: &'static str = include_str!("transitions.json");

#[derive(Debug, PartialEq)]
pub enum Transition {
    Overworld(Tile),
    Entrance(Tile),
    Underworld(Tile),
    None,
}

pub fn check_transition(previous: &Tile, current: &Tile) -> Transition {
    if previous.address_value != current.address_value && !previous.indoors && !current.indoors {
        Transition::Overworld(current.clone())
    } else if previous.indoors != current.indoors {
        Transition::Entrance(current.clone())
    } else if previous.indoors && current.indoors && previous.name != current.name {
        Transition::Underworld(current.clone())
    } else {
        Transition::None
    }
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct Tile {
    pub name: String,
    #[serde(deserialize_with = "hex_16bit_deserialize")]
    pub address_value: u16,
    pub timestamp: Option<DateTime<Utc>>,
    pub indoors: bool,
    pub conditions: Option<Conditions>,
}

impl Tile {
    pub fn new(address_value: u16, indoors: bool) -> Self {
        Tile {
            timestamp: Some(Utc::now()),
            indoors,
            address_value,
            ..Default::default()
        }
    }

    pub fn time_transit(&mut self) {
        self.timestamp = Some(Utc::now())
    }
}

impl Default for Tile {
    fn default() -> Self {
        Self {
            timestamp: None,
            indoors: Default::default(),
            address_value: Default::default(),
            conditions: Default::default(),
            name: Default::default(),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq, Clone, Hash, Eq)]
pub struct ConditionTransition {
    pub name: String,
    #[serde(deserialize_with = "hex_16bit_deserialize")]
    pub address_value: u16,
    pub indoors: bool,
}

/// Extra conditions to evaluate to deem that a specific transition has been triggered.
///
/// As an example, some tiles use the same address AND address value.
/// In these cases we can evaluate which specific tile Link enters by checking that the previous address value
/// equals one defined in `previous_tiles`
#[derive(Debug, Default, Deserialize, PartialEq, Clone, Hash, Eq)]
pub struct Conditions {
    previous_tile: Option<ConditionTransition>,
    coordinates: Option<Vec<Coordinate>>,
}

#[derive(Debug, Deserialize, PartialEq, Clone, Hash, Eq)]
pub struct Coordinate {
    #[serde(deserialize_with = "coordinate_deserialize")]
    x: u16,
    #[serde(deserialize_with = "coordinate_deserialize")]
    y: u16,
}

/// Reads src/checks.json and returns deserialized content
pub fn deserialize_transitions() -> Result<Vec<Tile>, serde_json::Error> {
    serde_json::from_str(TRANSITIONS_JSON)
}

pub fn deserialize_transitions_map() -> Result<HashMap<SnesMemoryID, Tile>, serde_json::Error> {
    Ok(deserialize_transitions()?
        .into_iter()
        .map(|transition| {
            (
                SnesMemoryID {
                    address_value: Some(transition.address_value),
                    indoors: Some(transition.indoors),
                    ..Default::default()
                },
                transition,
            )
        })
        .collect())
}

impl From<Vec<u8>> for Tile {
    fn from(_: Vec<u8>) -> Self {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_transitions() {
        let hobo = deserialize_transitions()
            .unwrap()
            .into_iter()
            .find(|transition| transition.name == "Hobo")
            .unwrap();
        assert_eq!(
            hobo,
            Tile {
                name: "Hobo".to_string(),
                indoors: false,
                address_value: 0x80,
                conditions: Some(Conditions {
                    previous_tile: Some(ConditionTransition {
                        name: "Statues".to_string(),
                        indoors: false,
                        address_value: 0x34
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }
        );
    }

    // add this test back when adding locations with conditions
    fn test_deserialize_transitions_map_keys_have_correct_values() {
        let transitions = deserialize_transitions().unwrap();
        let transitions_map = deserialize_transitions_map().unwrap();

        for transition in transitions {
            let should_be_same_transition = transitions_map
                .get(&SnesMemoryID {
                    address_value: Some(transition.address_value),
                    indoors: Some(transition.indoors),
                    ..Default::default()
                })
                .unwrap();
            assert_eq!(&transition, should_be_same_transition);
        }
    }

    #[test]
    fn test_deserialize_transitions_map_keys_have_correct_values_exclude_conditional_locations() {
        let transitions = deserialize_transitions()
            .unwrap()
            .into_iter()
            .filter(|transition| transition.conditions.is_none());
        let transitions_map: HashMap<SnesMemoryID, Tile> = deserialize_transitions_map()
            .unwrap()
            .into_iter()
            .filter(|(_, transition)| transition.conditions.is_none())
            .collect();

        for transition in transitions {
            let should_be_same_transition = transitions_map
                .get(&SnesMemoryID {
                    address_value: Some(transition.address_value),
                    indoors: Some(transition.indoors),
                    ..Default::default()
                })
                .ok_or(format!(
                    "hashmap doesn't contain {:X}, indoors: {}",
                    transition.address_value, transition.indoors
                ))
                .unwrap();
            assert_eq!(&transition, should_be_same_transition);
        }
    }

    macro_rules! test_trigger_transition {
        ($($name:ident: $previous:expr, $current:expr, $expected_trigger:ident,)*) => {
            $(
                #[test]
                fn $name() {
                    assert_eq!(check_transition(&$previous, &$current), Transition::$expected_trigger($current))
                }
            )*
        };
    }

    macro_rules! test_trigger_no_transition {
        ($($name:ident: $previous:expr, $current:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    assert_eq!(check_transition(&$previous, &$current), Transition::None)
                }
            )*
        };
    }

    test_trigger_transition! {
        overworld_transition:
            Tile {address_value: 0x0, ..Default::default()},
            Tile {address_value: 0x2, ..Default::default()},
            Overworld,
        entrance_transition:
            Tile {address_value: 0x69, indoors: false, ..Default::default()},
            Tile {address_value: 0x69, indoors: true, ..Default::default()},
            Entrance,
        underworld_transition: // TODO: If we have already turned snes vram into Transition objects, then we already have a unique id with the name, thus making all this logic checking redundant
            Tile {name: "Eastern Palace - Lobby".to_string(), address_value: 0x420, indoors: true, ..Default::default()},
            Tile {name: "Eastern Palace - Abyss Bridge".to_string(), address_value: 0x420, indoors: true, ..Default::default()},
            Underworld,
    }

    test_trigger_no_transition! {
        same_overworld_tile:
            Tile {address_value: 0x0, ..Default::default()},
            Tile {address_value: 0x0, ..Default::default()},
    }

    macro_rules! test_from_vec {
        ($($name:ident: $values:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (snes_ram, expected) = $values;
                    assert_eq!(Transition::from(snes_ram), expected)
                }
            )*
        };
    }
}
