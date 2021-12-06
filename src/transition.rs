use crate::condition::{coordinate_condition_met, previous_tile_condition_met, Conditions};
use crate::serde_lttp::hex_16bit_array_deserialize;
use crate::snes::SnesRam;
use anyhow::{anyhow, Result};

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::snes::NamedAddresses;

static TRANSITIONS_JSON: &'static str = include_str!("transitions.json");

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct Tile {
    pub id: usize,
    pub name: String,
    #[serde(deserialize_with = "hex_16bit_array_deserialize")]
    pub address_value: Vec<u16>,
    pub timestamp: Option<DateTime<Utc>>,
    pub indoors: bool,
    pub conditions: Option<Vec<Conditions>>,
    pub region: String,
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
                            if conditions.iter().all(|c| {
                                match c {
                                    Conditions::PreviousTile(condition) => previous_tile_condition_met(condition, previous_tile, tile),
                                    Conditions::Coordinates { coordinates } => coordinate_condition_met(coordinates, current),
                                    Conditions::Underworld => current.indoors() == 1,
                                    Conditions::DungeonCounterIncreased { sram_offset: _ } => todo!(),
                                }
                            })
                            {
                                return Ok(tile.clone());
                            }
                        }
                        None => panic!("This is bad: Tile lacking conditions when sharing ram address and value with others: {:?}", tile),
                    };
                }
                Err(anyhow!("No matches found for current ram value"))
            }
            _ => Err(anyhow!("No matches found for current ram value")),
        }
    }

    pub fn region(&self) -> String {
        match self.name.find("-") {
            Some(idx) => self.name[..idx - 1].to_string(),
            None => {
                if self.indoors {
                    "Underworld".to_string()
                } else {
                    "Overworld".to_string()
                }
            }
        }
    }
}

impl Default for Tile {
    fn default() -> Self {
        Self {
            id: Default::default(),
            timestamp: None,
            indoors: Default::default(),
            address_value: vec![0],
            conditions: Default::default(),
            name: Default::default(),
            region: Default::default(),
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
    use crate::{condition::ConditionTransition, snes::SnesRamInitializer};

    use super::*;

    #[test]
    fn test_deserialize_transitions() {
        let hobo = deserialize_transitions()
            .unwrap()
            .into_iter()
            .find(|transition| transition.id == 89)
            .unwrap();
        assert_eq!(
            hobo,
            Tile {
                id: 89,
                name: "Hobo".to_string(),
                region: "East Hyrule".to_string(),
                indoors: false,
                address_value: vec![0x80],
                conditions: Some(vec![Conditions::PreviousTile(ConditionTransition {
                    name: "Stone Bridge".to_string(),
                    address_value: Some(45),
                    indoors: Some(false),
                })]),
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
            "Link's House OW"),
        GIVEN_ram_points_to_links_house_uw_THEN_return_links_house_uw_tile: (
            SnesRamInitializer {entrance_id: Some(0x1), indoors: Some(1), ..Default::default()}.build(),
            Tile {..Default::default()},
            "Link's House UW"),
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
            Tile {name: "Misery Mire".to_string(), ..Default::default()},
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

    macro_rules! test_region {
        ($($name:ident: $values:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (tile_id, expected) = $values;
                    let tile  = deserialize_transitions()
                        .unwrap()
                        .into_iter()
                        .find(|t| t.id == tile_id)
                        .unwrap();
                    assert_eq!(tile.region(), expected.to_string())
                }
            )*
        };
    }

    test_region! {
        skull_woods_front: (352, "Skull Woods"),
        links_house_ow: (17, "Overworld"),
        links_house_uw: (20, "Underworld"),
    }
}
