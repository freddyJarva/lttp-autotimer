use crate::serde_lttp::coordinate_deserialize;
use crate::serde_lttp::coordinate_range_deserialize;
use crate::serde_lttp::hex_16bit_option_deserialize;
use crate::serde_lttp::hex_usize_deserialize;
use crate::snes::NamedAddresses;
use crate::transition::Tile;
use crate::SnesRam;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Clone, Hash, Eq, Default)]
pub struct ConditionTransition {
    pub name: String,
    #[serde(default)]
    #[serde(deserialize_with = "hex_16bit_option_deserialize")]
    pub address_value: Option<u16>,
    pub indoors: Option<bool>,
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
    Coordinates {
        coordinates: Vec<Coordinate>,
    },
    Underworld,
    DungeonCounterIncreased {
        #[serde(deserialize_with = "hex_usize_deserialize")]
        sram_offset: usize,
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
                        && y <= &(y_chest + 2)
                }
            },
            Coordinate::Range { x: _, y: _ } => todo!(),
            Coordinate::Chest { x: _, y: _ } => todo!(),
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
    condition.name == previous_tile.name || tile.name == previous_tile.name
}
