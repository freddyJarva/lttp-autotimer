use std::collections::VecDeque;

use crate::serde_lttp::coordinate_deserialize;
use crate::serde_lttp::coordinate_range_deserialize;
use crate::serde_lttp::hex_16bit_option_deserialize;
use crate::serde_lttp::hex_byte_deserialize;
use crate::serde_lttp::hex_usize_deserialize;
use crate::snes::NamedAddresses;
use crate::tile::Tile;
use crate::SnesRam;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Clone, Hash, Eq, Default)]
pub struct ConditionTransition {
    pub id: usize,
    #[serde(default)]
    #[serde(deserialize_with = "hex_16bit_option_deserialize")]
    pub address_value: Option<u16>,
    pub indoors: Option<bool>,
}

#[derive(Debug, Deserialize, PartialEq, Clone, Hash, Eq)]
pub enum Value {
    ValueOfAddress(#[serde(deserialize_with = "hex_usize_deserialize")] usize),
    CheckCount(usize),
    ItemCount(usize),
    EventCount(usize),
}

/// Extra conditions to evaluate to deem that a specific transition has been triggered.
///
/// As an example, some tiles use the same address AND address value.
/// In these cases we can evaluate which specific tile Link enters by checking that the previous address value
/// equals one defined in `previous_tiles`
#[derive(Debug, Deserialize, PartialEq, Clone, Hash, Eq)]
#[serde(tag = "type")]
pub enum Conditions {
    PreviousTile(ConditionTransition),
    CurrentTile(ConditionTransition),
    PreviousEvent {
        id: usize,
    },
    Coordinates {
        coordinates: Vec<Coordinate>,
    },
    Underworld,
    DungeonCounterIncreased {
        #[serde(deserialize_with = "hex_usize_deserialize")]
        sram_offset: usize,
    },
    BitWiseTrue {
        #[serde(deserialize_with = "hex_usize_deserialize")]
        sram_offset: usize,
        #[serde(deserialize_with = "hex_byte_deserialize")]
        sram_mask: u8,
    },
    ValueChanged {
        #[serde(deserialize_with = "hex_usize_deserialize")]
        sram_offset: usize,
    },
    ValueEq {
        #[serde(deserialize_with = "hex_usize_deserialize")]
        sram_offset: usize,
        #[serde(deserialize_with = "hex_byte_deserialize")]
        sram_value: u8,
    },
    ValueGreaterThan {
        #[serde(deserialize_with = "hex_usize_deserialize")]
        sram_offset: usize,
        other: Value,
    },
    PreviousValueEq {
        #[serde(deserialize_with = "hex_usize_deserialize")]
        sram_offset: usize,
        #[serde(deserialize_with = "hex_byte_deserialize")]
        sram_value: u8,
    },
    Any {
        subconditions: Vec<Conditions>,
    },
    Not {
        subconditions: Vec<Conditions>,
    },
    All {
        subconditions: Vec<Conditions>,
    },
    CheckMade {
        id: usize,
    },
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
    Chest {
        #[serde(deserialize_with = "coordinate_deserialize")]
        x: u16,
        #[serde(deserialize_with = "coordinate_deserialize")]
        y: u16,
    },
    BigChest {
        #[serde(deserialize_with = "coordinate_deserialize")]
        x: u16,
        #[serde(deserialize_with = "coordinate_deserialize")]
        y: u16,
    },
    Stairs {
        #[serde(deserialize_with = "coordinate_deserialize")]
        x: u16,
        #[serde(deserialize_with = "coordinate_deserialize")]
        y: u16,
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
                // For chest checks we basically make a bounding box with predefined width/height
                Coordinate::Chest {
                    x: x_chest,
                    y: y_chest,
                } => {
                    x >= &(x_chest - 10)
                        && x <= &(x_chest + 10)
                        && y >= &(y_chest - 1)
                        && y <= &(y_chest + 3)
                }
                // Stairs transitions can vary slightly on the y-axis
                Coordinate::Stairs {
                    x: x_stairs,
                    y: y_stairs,
                } => x == x_stairs && y >= &(y_stairs - 3) && y <= &(y_stairs + 3),
                Coordinate::BigChest {
                    x: x_chest,
                    y: y_chest,
                } => {
                    x >= &(x_chest - 24)
                        && x <= &(x_chest + 24)
                        && y >= &(y_chest - 1)
                        && y <= &(y_chest + 3)
                }
            },
            _ => todo!(),
        }
    }

    fn from_continuous_coords(ram: &SnesRam) -> Self {
        Self::Pair {
            x: ram.x(),
            y: ram.y(),
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

pub fn coordinate_condition_met(conditions: &[Coordinate], current: &SnesRam) -> bool {
    conditions.iter().any(|c| match c {
        Coordinate::Chest { x: _, y: _ } => Coordinate::from_continuous_coords(current).matches(c),
        _ => Coordinate::from(current).matches(c),
    })
}

pub fn previous_tile_condition_met(
    condition: &ConditionTransition,
    previous_tile: &Tile,
    tile: &Tile,
) -> bool {
    condition.id == previous_tile.id || tile.id == previous_tile.id
}

pub fn current_tile_condition_met(condition: &ConditionTransition, tile: &Tile) -> bool {
    condition.id == tile.id
}

pub fn dungeon_counter_condition_met(
    ram_history: &VecDeque<SnesRam>,
    ram: &SnesRam,
    sram_offset: &usize,
) -> bool {
    if ram_history.len() > 0 {
        ram.get_byte(*sram_offset) > ram_history[ram_history.len() - 1].get_byte(*sram_offset)
    } else {
        false
    }
}

pub fn ram_value_change_condition_met(
    previous_values: &mut VecDeque<SnesRam>,
    ram: &SnesRam,
    sram_offset: &usize,
) -> bool {
    if previous_values.len() > 0 {
        ram.get_byte(*sram_offset)
            != previous_values[previous_values.len() - 1].get_byte(*sram_offset)
    } else {
        false
    }
}
