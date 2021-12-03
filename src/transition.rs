use crate::serde_lttp::coordinate_range_deserialize;
use crate::serde_lttp::hex_16bit_array_deserialize;
use crate::{serde_lttp::coordinate_deserialize, snes::SnesRam};
use anyhow::{anyhow, Result};

use crate::serde_lttp::hex_16bit_deserialize;

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::snes::NamedAddresses;

static TRANSITIONS_JSON: &'static str = include_str!("transitions.json");

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct Tile {
    pub name: String,
    #[serde(deserialize_with = "hex_16bit_array_deserialize")]
    pub address_value: Vec<u16>,
    pub timestamp: Option<DateTime<Utc>>,
    pub indoors: bool,
    pub conditions: Option<Conditions>,
}

impl Tile {
    pub fn new(address_value: u16, indoors: bool) -> Self {
        Tile {
            timestamp: Some(Utc::now()),
            indoors,
            address_value: vec![address_value],
            ..Default::default()
        }
    }

    pub fn time_transit(&mut self) {
        self.timestamp = Some(Utc::now())
    }

    pub fn try_from_ram(current: &SnesRam, previous_tile: &Tile) -> Result<Tile> {
        lazy_static! {
            static ref TILES: Vec<Tile> =
                deserialize_transitions().expect("Failed to deserialize transitions.json");
        }
        let matches: Vec<&Tile> = TILES
            .iter()
            .filter(|&t| {
                current.indoors() == 1
                    && t.indoors
                    && t.address_value.contains(&(current.entrance_id() as u16))
                    || current.indoors() == 0
                        && !t.indoors
                        && t.address_value.contains(&(current.overworld_tile() as u16))
            })
            .collect();
        match matches.len() {
            1 => Ok(matches[0].clone()),
            // Further filtering needed
            2.. => {
                for &tile in &matches {
                    match &tile.conditions {
                        Some(conditions) => {
                            let conditions = conditions.clone();
                            if previous_tile_condition_met(&conditions, previous_tile, tile)
                                || coordinate_condition_met(&conditions, &current)
                            {
                                return Ok(tile.clone());
                            }
                        }
                        // If conditions haven't been created yet, just return error
                        None => panic!("This is bad: Tile lacking conditions when sharing ram address and value with others: {:?}", tile),
                        // None => return Err(anyhow!("No matches found for current ram value")),
                    };
                }
                Err(anyhow!("No matches found for current ram value"))
            }
            _ => Err(anyhow!("No matches found for current ram value")),
        }
    }
}

fn coordinate_condition_met(conditions: &Conditions, current: &SnesRam) -> bool {
    if let Some(condition_coordinate) = &conditions.coordinates {
        condition_coordinate
            .iter()
            .any(|c| Coordinate::from(current).matches(c))
    } else {
        false
    }
}

fn previous_tile_condition_met(conditions: &Conditions, previous_tile: &Tile, tile: &Tile) -> bool {
    if let Some(previous_tile_condition) = &conditions.previous_tile {
        previous_tile_condition.name == previous_tile.name || tile.name == previous_tile.name
    } else {
        false
    }
}

impl Default for Tile {
    fn default() -> Self {
        Self {
            timestamp: None,
            indoors: Default::default(),
            address_value: vec![0],
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
#[serde(tag = "type")]
pub enum Coordinate {
    Pair {
        #[serde(deserialize_with = "coordinate_deserialize")]
        x: u16,
        #[serde(deserialize_with = "coordinate_deserialize")]
        y: u16,
    },
    Range {
        #[serde(deserialize_with = "coordinate_range_deserialize")]
        x: (u16, u16),
        #[serde(deserialize_with = "coordinate_range_deserialize")]
        y: (u16, u16),
    },
}

impl Coordinate {
    /// Matches `self` against another `Coordinate` enum returns `true` if criteria is fulfilled
    ///
    /// matching `Coordinate::Pair` against `Coordinate::Pair` is a straightforward equality check.
    ///
    /// when `self` is of type `Pair` and `other` of type `Range`, a match will be found
    /// if the coordinates in `self` is bounded by the coordinate ranges in `other`
    pub fn matches(&self, other: &Self) -> bool {
        match self {
            Coordinate::Pair { x, y } => match other {
                Coordinate::Pair { x: _, y: _ } => self == other,
                Coordinate::Range {
                    x: x_range,
                    y: y_range,
                } => x >= &x_range.0 && x <= &x_range.1 && y >= &y_range.0 && y <= &y_range.1,
            },
            Coordinate::Range { x: _, y: _ } => todo!(),
        }
    }
}

impl From<&SnesRam> for Coordinate {
    fn from(ram: &SnesRam) -> Self {
        Self::Pair {
            x: ram.transition_x(),
            y: ram.transition_y(),
        }
    }
}

/// Reads src/checks.json and returns deserialized content
pub fn deserialize_transitions() -> Result<Vec<Tile>, serde_json::Error> {
    serde_json::from_str(TRANSITIONS_JSON)
}

#[allow(non_snake_case)]
#[cfg(test)]
mod tests {
    use crate::snes::SnesRamInitializer;

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
                address_value: vec![0x80],
                conditions: Some(Conditions {
                    previous_tile: Some(ConditionTransition {
                        name: "Stone Bridge".to_string(),
                        indoors: false,
                        address_value: 0x2d
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }
        );
    }

    macro_rules! test_tile_from {
        ($($name:ident: $values:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (current_ram, previous_tile , expected) = $values;
                    assert_eq!(Tile::try_from_ram(&current_ram, &previous_tile).unwrap().name, expected.to_string())
                }
            )*
        };
    }

    test_tile_from! {
        GIVEN_ram_points_to_links_house_ow_THEN_return_links_house_ow_tile: (
            SnesRamInitializer {overworld_tile: Some(0x2c), ..Default::default()}.build(),
            Tile {..Default::default()},
            "Link's House - OW"),
        GIVEN_ram_points_to_links_house_uw_THEN_return_links_house_uw_tile: (
            SnesRamInitializer {entrance_id: Some(0x1), indoors: Some(1), ..Default::default()}.build(),
            Tile {..Default::default()},
            "Link's House - UW"),
        GIVEN_ram_points_to_hobo_meadow_AND_previous_tile_stone_bridge_THEN_return_hobo_tile: (
            SnesRamInitializer {overworld_tile: Some(0x80), ..Default::default()}.build(),
            Tile {name: "Stone Bridge".to_string(), ..Default::default()},
            "Hobo"),
        GIVEN_ram_points_to_hobo_meadow_AND_previous_tile_lost_woods_THEN_return_meadow: (
            SnesRamInitializer {overworld_tile: Some(0x80), ..Default::default()}.build(),
            Tile {name: "Lost Woods".to_string(), ..Default::default()},
            "Mastersword Meadow"),
        GIVEN_ram_points_to_big_fairy_AND_previous_tile_misery_mire_ow_THEN_return_mire_big_fairy: (
            SnesRamInitializer {entrance_id: Some(0x5e), indoors: Some(1), ..Default::default()}.build(),
            Tile {name: "Misery Mire - OW".to_string(), ..Default::default()},
            "Mire big fairy"),
        GIVEN_ram_points_to_hobo_meadow_AND_previous_tile_meadow_THEN_return_meadow: (
            SnesRamInitializer {overworld_tile: Some(0x80), ..Default::default()}.build(),
            Tile {name: "Mastersword Meadow".to_string(), ..Default::default()},
            "Mastersword Meadow"),
        GIVEN_ram_points_to_ep_uw_AND_xy_points_to_abyss_bridge_THEN_return_abyss_bridge: (
            SnesRamInitializer {entrance_id: Some(0x8), indoors: Some(1), transition_x: Some(4856), transition_y: Some(6336), ..Default::default()}.build(),
            Tile {..Default::default()},
            "Eastern Palace - Abyss Bridge"),
        GIVEN_ram_points_to_hc_uw_AND_xy_is_in_bounds_for_basement_1_range_THEN_return_basement_1: (
            SnesRamInitializer {entrance_id: Some(0x4), indoors: Some(1), transition_x: Some(1192), transition_y: Some(4052), ..Default::default()}.build(),
            Tile {..Default::default()},
            "Hyrule Castle - Basement 1"),
    }
}
